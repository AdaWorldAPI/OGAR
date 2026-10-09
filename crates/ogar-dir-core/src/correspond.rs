//! Hybrid identity correspondence as a numeric fold.
//!
//! AD, Entra and Exchange Online are observed separately and stay separate:
//! the fold relates rows, it never merges them, and it never edits a lane.
//! Three witnesses are read, each from its own [`CompiledRule`]:
//!
//! * **forward** — the AD source anchor equals the Entra `onPremisesImmutableId`
//!   (the anchor is `mS-DS-ConsistencyGuid` or `objectGUID`, whichever rule
//!   the [`Profile`] carries; there is no fallback between them);
//! * **backsync** — AD `msDS-ExternalDirectoryObjectId` equals the Entra id;
//! * **cloud** — the Exchange Online `ExternalDirectoryObjectId` equals the
//!   Entra id.
//!
//! Each witness that holds becomes one [`DirEdge`] with its own evidence
//! code. Whether the witnesses agree, conflict, or are ambiguous is reported
//! per row as flag bits, and [`AdStatus`] names the outcome. Nothing picks a
//! "first" match: a key matched by several rows is ambiguous on both sides.
//!
//! ## Stages
//!
//! ```text
//! DirRecord + ValuePool ──Lanes::build──► Lanes   (boundary: decodes once)
//! Lanes ──Index::build──► Index                   (cold: sorts, counts edges)
//! Lanes + Index ──fold──► Output                  (hot: numeric only)
//! ```
//!
//! The fold reads `u32` scope ordinals and 128-bit ids, binary-searches the
//! sorted indexes and writes into an [`Output`] sized by `Index::build`. It
//! performs no string work, no decoding and no allocation once the output is
//! sized.
//!
//! ## Scope
//!
//! An AD domain and an Entra tenant are different id spaces, so a key is
//! only compared within a declared [`Binding`] (AD domain → tenant). A row
//! whose scope is not bound joins nothing, whatever its bytes.

use crate::edge::{DirEdge, EdgeEvidence, EdgeKind};
use crate::guid::Guid128;
use crate::identity::{CompiledRule, IdFault, IdValue, Slot};
use crate::pool::ValuePool;
use crate::record::DirRecord;
use crate::schema::SchemaFamily;

/// A row's scope ordinal when its scope is not bound.
pub const UNBOUND: u32 = u32::MAX;
/// "No row".
pub const NO_ROW: u32 = u32::MAX;

/// One AD domain synchronized into one tenant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    /// The AD domain GUID (an AD record's `scope_guid`).
    pub ad_domain: Guid128,
    /// The tenant id (an Entra / Exchange Online record's `scope_guid`).
    pub tenant: Guid128,
}

/// The rules a deployment uses, plus how far apart two observations may be
/// taken and still count as one moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    /// AD source anchor → Entra `onPremisesImmutableId`.
    pub anchor: CompiledRule,
    /// AD `msDS-ExternalDirectoryObjectId` → Entra id.
    pub backsync: CompiledRule,
    /// Entra id → Exchange Online `ExternalDirectoryObjectId`.
    pub cloud: CompiledRule,
    /// Maximum `|observed_at_ms(AD) - observed_at_ms(Entra)|`. An unknown
    /// time (0) is never aligned.
    pub max_skew_ms: i64,
}

/// What a lane cell holds.
pub mod state {
    #![allow(missing_docs)]
    pub const ABSENT: u8 = 0;
    pub const PRESENT: u8 = 1;
    pub const MALFORMED: u8 = 2;
    pub const NIL: u8 = 3;
}

/// One decoded id column with its state column.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IdColumn {
    /// The id; [`Guid128::NIL`] unless the state is `PRESENT`.
    pub id: Vec<Guid128>,
    /// A [`state`] code.
    pub state: Vec<u8>,
}

impl IdColumn {
    fn push(&mut self, v: IdValue) {
        let (g, s) = match v {
            IdValue::Absent => (Guid128::NIL, state::ABSENT),
            IdValue::Fault(IdFault::Malformed) => (Guid128::NIL, state::MALFORMED),
            IdValue::Fault(IdFault::Nil) => (Guid128::NIL, state::NIL),
            IdValue::Id(g) => (g, state::PRESENT),
        };
        self.id.push(g);
        self.state.push(s);
    }
}

/// One family's rows as columns (structure of arrays).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rows {
    /// The row's own id (`node_guid`).
    pub owner: Vec<Guid128>,
    /// Scope ordinal (tenant), or [`UNBOUND`].
    pub scope: Vec<u32>,
    /// `observed_at_ms`.
    pub at: Vec<i64>,
}

impl Rows {
    /// Row count.
    pub fn len(&self) -> usize {
        self.owner.len()
    }
    /// No rows.
    pub fn is_empty(&self) -> bool {
        self.owner.is_empty()
    }
}

/// Every column the fold reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lanes {
    /// AD rows.
    pub ad: Rows,
    /// AD source anchor (per the profile's anchor rule).
    pub ad_anchor: IdColumn,
    /// AD backsync id.
    pub ad_backsync: IdColumn,
    /// Entra rows (owner = Entra id).
    pub entra: Rows,
    /// Entra `onPremisesImmutableId`.
    pub entra_anchor: IdColumn,
    /// Exchange Online rows.
    pub exo: Rows,
    /// Exchange Online `ExternalDirectoryObjectId`.
    pub exo_external: IdColumn,
    /// The anchor rule reads `objectGUID` (legacy evidence code).
    pub anchor_is_object_guid: bool,
    /// [`Profile::max_skew_ms`].
    pub max_skew_ms: i64,
}

/// One family's records and the pool their strings live in.
#[derive(Clone, Copy, Debug)]
pub struct Source<'a> {
    /// Records.
    pub records: &'a [DirRecord],
    /// Their pool.
    pub pool: &'a ValuePool,
}

impl Lanes {
    /// Boundary: read and decode every id once. Records of another family
    /// are skipped; every row keeps its own `node_guid`.
    pub fn build(
        p: &Profile,
        bindings: &[Binding],
        ad: Source<'_>,
        entra: Source<'_>,
        exo: Source<'_>,
    ) -> Self {
        let mut tenants: Vec<Guid128> = Vec::new();
        for b in bindings {
            if !tenants.contains(&b.tenant) {
                tenants.push(b.tenant);
            }
        }
        let tenant_ord = |t: Guid128| {
            tenants
                .iter()
                .position(|x| *x == t)
                .map_or(UNBOUND, |i| i as u32)
        };
        let domain_ord = |d: Guid128| {
            bindings
                .iter()
                .find(|b| b.ad_domain == d)
                .map_or(UNBOUND, |b| tenant_ord(b.tenant))
        };
        let mut l = Lanes {
            anchor_is_object_guid: p.anchor.source.slot == Slot::Node,
            max_skew_ms: p.max_skew_ms,
            ..Self::default()
        };
        let rows = |rows: &mut Rows, r: &DirRecord, scope: u32| {
            rows.owner.push(r.node_guid());
            rows.scope.push(scope);
            rows.at.push(r.observed_at_ms());
        };
        for r in ad
            .records
            .iter()
            .filter(|r| r.schema().family == SchemaFamily::AdDs)
        {
            rows(&mut l.ad, r, domain_ord(r.scope_guid()));
            l.ad_anchor.push(p.anchor.source.read(r, ad.pool));
            l.ad_backsync.push(p.backsync.source.read(r, ad.pool));
        }
        for r in entra
            .records
            .iter()
            .filter(|r| r.schema().family == SchemaFamily::MsGraph)
        {
            rows(&mut l.entra, r, tenant_ord(r.scope_guid()));
            l.entra_anchor.push(p.anchor.target.read(r, entra.pool));
        }
        for r in exo
            .records
            .iter()
            .filter(|r| r.schema().family == SchemaFamily::ExchangeOnline)
        {
            rows(&mut l.exo, r, tenant_ord(r.scope_guid()));
            l.exo_external.push(p.cloud.target.read(r, exo.pool));
        }
        l
    }
}

/// One index entry: a key in a scope, and the row carrying it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Entry {
    scope: u32,
    key: u128,
    row: u32,
}

/// The 16 bytes as one big-endian integer: the same bytes, the same order
/// and equality as [`Guid128`], compared in two machine words.
#[inline]
fn k(g: &Guid128) -> u128 {
    u128::from_be_bytes(g.0)
}

/// Rows of a column sorted by `(scope, key)`; unbound and non-present rows
/// are left out.
fn sorted(scope: &[u32], ids: &[Guid128], present: impl Fn(usize) -> bool) -> Vec<Entry> {
    let mut t: Vec<Entry> = (0..ids.len())
        .filter(|&i| scope[i] != UNBOUND && present(i))
        .map(|i| Entry {
            scope: scope[i],
            key: k(&ids[i]),
            row: i as u32,
        })
        .collect();
    t.sort_unstable();
    t
}

/// Walk two sorted indexes key group by key group, calling `on` with the
/// rows of each side that carry the key (one side may be empty). Linear in
/// the two lengths; reads both sequentially.
#[inline]
fn merge(l: &[Entry], r: &[Entry], mut on: impl FnMut(&[Entry], &[Entry])) {
    let run = |s: &[Entry], i: usize| {
        let (sc, key) = (s[i].scope, s[i].key);
        i + s[i..]
            .iter()
            .take_while(|e| e.scope == sc && e.key == key)
            .count()
    };
    let (mut i, mut j) = (0, 0);
    while i < l.len() || j < r.len() {
        let ord = match (l.get(i), r.get(j)) {
            (Some(a), Some(b)) => (a.scope, a.key).cmp(&(b.scope, b.key)),
            (Some(_), None) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Greater,
        };
        match ord {
            std::cmp::Ordering::Less => {
                let ie = run(l, i);
                on(&l[i..ie], &[]);
                i = ie;
            }
            std::cmp::Ordering::Greater => {
                let je = run(r, j);
                on(&[], &r[j..je]);
                j = je;
            }
            std::cmp::Ordering::Equal => {
                let (ie, je) = (run(l, i), run(r, j));
                on(&l[i..ie], &r[j..je]);
                (i, j) = (ie, je);
            }
        }
    }
}

/// The sorted indexes and the exact edge count the fold will produce.
#[derive(Clone, Debug, Default)]
pub struct Index {
    entra_by_anchor: Vec<Entry>,
    entra_by_id: Vec<Entry>,
    ad_by_anchor: Vec<Entry>,
    ad_by_backsync: Vec<Entry>,
    exo_by_external: Vec<Entry>,
    /// Exact number of edges [`fold`] emits.
    pub edges: usize,
}

fn present(c: &IdColumn) -> impl Fn(usize) -> bool + '_ {
    move |i| c.state[i] == state::PRESENT
}

impl Index {
    /// Cold: sort every key column and count the edges.
    pub fn build(l: &Lanes) -> Self {
        let mut ix = Self {
            entra_by_anchor: sorted(&l.entra.scope, &l.entra_anchor.id, present(&l.entra_anchor)),
            entra_by_id: sorted(&l.entra.scope, &l.entra.owner, |_| true),
            ad_by_anchor: sorted(&l.ad.scope, &l.ad_anchor.id, present(&l.ad_anchor)),
            ad_by_backsync: sorted(&l.ad.scope, &l.ad_backsync.id, present(&l.ad_backsync)),
            exo_by_external: sorted(&l.exo.scope, &l.exo_external.id, present(&l.exo_external)),
            edges: 0,
        };
        let mut n = 0;
        let mut count = |a: &[Entry], b: &[Entry]| n += a.len() * b.len();
        merge(&ix.ad_by_anchor, &ix.entra_by_anchor, &mut count);
        merge(&ix.ad_by_backsync, &ix.entra_by_id, &mut count);
        merge(&ix.exo_by_external, &ix.entra_by_id, &mut count);
        ix.edges = n;
        ix
    }
}

/// Per-row flag bits. A row's flags are the whole evidence; [`AdStatus`]
/// is a reading of them.
pub mod flag {
    #![allow(missing_docs)]
    // AD rows.
    pub const ANCHOR_ABSENT: u32 = 1 << 0;
    pub const ANCHOR_MALFORMED: u32 = 1 << 1;
    pub const ANCHOR_NIL: u32 = 1 << 2;
    pub const UNBOUND_SCOPE: u32 = 1 << 3;
    /// No Entra row carries this anchor.
    pub const FWD_NONE: u32 = 1 << 4;
    pub const FWD_ONE: u32 = 1 << 5;
    pub const FWD_MANY: u32 = 1 << 6;
    /// Another AD row in the scope carries the same anchor.
    pub const ANCHOR_SHARED: u32 = 1 << 7;
    pub const BACK_ABSENT: u32 = 1 << 8;
    pub const BACK_MALFORMED: u32 = 1 << 9;
    /// The backsync id names no observed Entra row.
    pub const BACK_DANGLING: u32 = 1 << 10;
    pub const BACK_ONE: u32 = 1 << 11;
    pub const BACK_MANY: u32 = 1 << 12;
    /// Forward and backsync name the same Entra row.
    pub const AGREE: u32 = 1 << 13;
    /// Forward and backsync name different Entra rows, or one names a row
    /// the other cannot reach.
    pub const CONTRADICT: u32 = 1 << 14;
    /// Both observation times known and within the profile's skew.
    pub const TIME_ALIGNED: u32 = 1 << 15;
    /// The Entra row this row reaches (by either witness) is contested
    /// ([`E_SPLIT`]).
    pub const TARGET_SPLIT: u32 = 1 << 16;
    // Entra rows.
    pub const E_ANCHOR_ABSENT: u32 = 1 << 0;
    pub const E_ANCHOR_FAULT: u32 = 1 << 1;
    /// The immutable id names no observed AD anchor.
    pub const E_ORPHAN: u32 = 1 << 2;
    pub const E_CLAIMED_ONE: u32 = 1 << 3;
    pub const E_CLAIMED_MANY: u32 = 1 << 4;
    pub const E_BACK_ONE: u32 = 1 << 5;
    pub const E_BACK_MANY: u32 = 1 << 6;
    /// Contested: several AD rows claim it by anchor or name it by backsync,
    /// or the one claimer and the one namer are different AD rows.
    pub const E_SPLIT: u32 = 1 << 7;
    /// Another Entra row in the tenant carries the same immutable id.
    pub const E_ANCHOR_SHARED: u32 = 1 << 8;
    pub const E_EXO_ONE: u32 = 1 << 9;
    pub const E_EXO_MANY: u32 = 1 << 10;
    pub const E_UNBOUND_SCOPE: u32 = 1 << 11;
    // Exchange Online rows.
    pub const X_ABSENT: u32 = 1 << 0;
    pub const X_FAULT: u32 = 1 << 1;
    pub const X_DANGLING: u32 = 1 << 2;
    pub const X_ONE: u32 = 1 << 3;
    pub const X_MANY: u32 = 1 << 4;
    pub const X_UNBOUND_SCOPE: u32 = 1 << 5;
}

/// The fold's result. Sized by [`Output::for_index`]; [`fold`] only writes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    /// Per AD row.
    pub ad: Vec<u32>,
    /// Per AD row: the forward target (Entra row) when exactly one, else
    /// [`NO_ROW`].
    pub ad_target: Vec<u32>,
    /// Per AD row: the backsync target when exactly one, else [`NO_ROW`].
    pub ad_back: Vec<u32>,
    /// Per Entra row.
    pub entra: Vec<u32>,
    /// Per Entra row: the AD row whose anchor alone claims it, else [`NO_ROW`].
    pub entra_claimer: Vec<u32>,
    /// Per Entra row: the AD row whose backsync alone names it, else [`NO_ROW`].
    pub entra_namer: Vec<u32>,
    /// Per Exchange Online row.
    pub exo: Vec<u32>,
    /// Witness edges.
    pub edges: Vec<DirEdge>,
}

impl Output {
    /// Allocate exactly what [`fold`] writes.
    pub fn for_index(l: &Lanes, ix: &Index) -> Self {
        Self {
            ad: vec![0; l.ad.len()],
            ad_target: vec![NO_ROW; l.ad.len()],
            ad_back: vec![NO_ROW; l.ad.len()],
            entra: vec![0; l.entra.len()],
            entra_claimer: vec![NO_ROW; l.entra.len()],
            entra_namer: vec![NO_ROW; l.entra.len()],
            exo: vec![0; l.exo.len()],
            edges: Vec::with_capacity(ix.edges),
        }
    }
}

#[inline]
fn count_flag(n: usize, zero: u32, one: u32, many: u32) -> u32 {
    match n {
        0 => zero,
        1 => one,
        _ => many,
    }
}

#[inline]
fn sole(s: &[Entry]) -> u32 {
    if s.len() == 1 { s[0].row } else { NO_ROW }
}

/// Hot: relate the rows. `out` must come from [`Output::for_index`] on the
/// same lanes and index; it is overwritten.
///
/// Three sort-merge joins over the prebuilt indexes (forward, backsync,
/// cloud), then one pass per family that reads the joins' results. Every
/// index is read sequentially; nothing is allocated.
pub fn fold(l: &Lanes, ix: &Index, out: &mut Output) {
    out.edges.clear();
    out.ad_target.fill(NO_ROW);
    out.ad_back.fill(NO_ROW);
    out.entra_claimer.fill(NO_ROW);
    out.entra_namer.fill(NO_ROW);

    // Row-local facts.
    for i in 0..l.ad.len() {
        let mut f = 0;
        if l.ad.scope[i] == UNBOUND {
            f |= flag::UNBOUND_SCOPE;
        }
        f |= match l.ad_anchor.state[i] {
            state::ABSENT => flag::ANCHOR_ABSENT,
            state::MALFORMED => flag::ANCHOR_MALFORMED,
            state::NIL => flag::ANCHOR_NIL,
            _ => 0,
        };
        f |= match l.ad_backsync.state[i] {
            state::ABSENT => flag::BACK_ABSENT,
            state::PRESENT => 0,
            _ => flag::BACK_MALFORMED,
        };
        out.ad[i] = f;
    }
    for j in 0..l.entra.len() {
        let mut f = 0;
        if l.entra.scope[j] == UNBOUND {
            f |= flag::E_UNBOUND_SCOPE;
        }
        f |= match l.entra_anchor.state[j] {
            state::ABSENT => flag::E_ANCHOR_ABSENT,
            state::PRESENT => 0,
            _ => flag::E_ANCHOR_FAULT,
        };
        out.entra[j] = f;
    }
    for x in 0..l.exo.len() {
        let mut f = 0;
        if l.exo.scope[x] == UNBOUND {
            f |= flag::X_UNBOUND_SCOPE;
        }
        f |= match l.exo_external.state[x] {
            state::ABSENT => flag::X_ABSENT,
            state::PRESENT => 0,
            _ => flag::X_FAULT,
        };
        out.exo[x] = f;
    }

    let edge = |ev, src: (SchemaFamily, Guid128), dst: (SchemaFamily, Guid128)| DirEdge {
        kind: EdgeKind::SynchronizesTo,
        evidence: ev,
        src,
        dst,
    };
    let fwd_ev = if l.anchor_is_object_guid {
        EdgeEvidence::ImmutableIdIsObjectGuid
    } else {
        EdgeEvidence::SourceAnchorMatchesImmutableId
    };

    // Forward: AD anchor = Entra immutable id.
    merge(&ix.ad_by_anchor, &ix.entra_by_anchor, |a, e| {
        let shared = if a.len() > 1 { flag::ANCHOR_SHARED } else { 0 };
        let fwd = count_flag(e.len(), flag::FWD_NONE, flag::FWD_ONE, flag::FWD_MANY);
        for x in a {
            let i = x.row as usize;
            out.ad[i] |= fwd | shared;
            out.ad_target[i] = sole(e);
            for y in e {
                out.edges.push(edge(
                    fwd_ev,
                    (SchemaFamily::AdDs, l.ad.owner[i]),
                    (SchemaFamily::MsGraph, l.entra.owner[y.row as usize]),
                ));
            }
        }
        let claimed = count_flag(
            a.len(),
            flag::E_ORPHAN,
            flag::E_CLAIMED_ONE,
            flag::E_CLAIMED_MANY,
        );
        let dup = if e.len() > 1 {
            flag::E_ANCHOR_SHARED
        } else {
            0
        };
        for y in e {
            let j = y.row as usize;
            out.entra[j] |= claimed | dup;
            out.entra_claimer[j] = sole(a);
        }
    });

    // Backsync: AD msDS-ExternalDirectoryObjectId = Entra id.
    merge(&ix.ad_by_backsync, &ix.entra_by_id, |a, e| {
        let back = count_flag(
            e.len(),
            flag::BACK_DANGLING,
            flag::BACK_ONE,
            flag::BACK_MANY,
        );
        for x in a {
            let i = x.row as usize;
            out.ad[i] |= back;
            out.ad_back[i] = sole(e);
            for y in e {
                out.edges.push(edge(
                    EdgeEvidence::AdBacksyncMatchesEntraObjectId,
                    (SchemaFamily::AdDs, l.ad.owner[i]),
                    (SchemaFamily::MsGraph, l.entra.owner[y.row as usize]),
                ));
            }
        }
        let named = count_flag(a.len(), 0, flag::E_BACK_ONE, flag::E_BACK_MANY);
        for y in e {
            let j = y.row as usize;
            out.entra[j] |= named;
            out.entra_namer[j] = sole(a);
        }
    });

    // Cloud: Exchange Online ExternalDirectoryObjectId = Entra id.
    merge(&ix.exo_by_external, &ix.entra_by_id, |x, e| {
        let hit = count_flag(e.len(), flag::X_DANGLING, flag::X_ONE, flag::X_MANY);
        for m in x {
            out.exo[m.row as usize] |= hit;
            for y in e {
                out.edges.push(edge(
                    EdgeEvidence::EntraObjectIdMatchesExchangeExternalId,
                    (SchemaFamily::MsGraph, l.entra.owner[y.row as usize]),
                    (SchemaFamily::ExchangeOnline, l.exo.owner[m.row as usize]),
                ));
            }
        }
        let mbx = count_flag(x.len(), 0, flag::E_EXO_ONE, flag::E_EXO_MANY);
        for y in e {
            out.entra[y.row as usize] |= mbx;
        }
    });

    // A contested Entra row: several AD rows claim or name it, or the one
    // claimer and the one namer differ.
    for j in 0..l.entra.len() {
        let (c, n) = (out.entra_claimer[j], out.entra_namer[j]);
        let many = out.entra[j] & (flag::E_CLAIMED_MANY | flag::E_BACK_MANY) != 0;
        if many || (c != NO_ROW && n != NO_ROW && c != n) {
            out.entra[j] |= flag::E_SPLIT;
        }
    }

    // What the witnesses say together.
    for i in 0..l.ad.len() {
        let (target, back) = (out.ad_target[i], out.ad_back[i]);
        let mut f = out.ad[i];
        if target != NO_ROW {
            if back == target {
                f |= flag::AGREE;
            } else if f & (flag::BACK_ONE | flag::BACK_MANY | flag::BACK_DANGLING) != 0 {
                f |= flag::CONTRADICT;
            }
            if out.entra[target as usize] & flag::E_SPLIT != 0 {
                f |= flag::TARGET_SPLIT;
            }
            let (t0, t1) = (l.ad.at[i], l.entra.at[target as usize]);
            if t0 != 0 && t1 != 0 && (t0 - t1).abs() <= l.max_skew_ms {
                f |= flag::TIME_ALIGNED;
            }
        } else if back != NO_ROW && f & flag::FWD_NONE != 0 {
            // The anchor reaches nothing while backsync reaches a row: the
            // two witnesses disagree about whether a counterpart exists.
            f |= flag::CONTRADICT;
        }
        if back != NO_ROW && out.entra[back as usize] & flag::E_SPLIT != 0 {
            f |= flag::TARGET_SPLIT;
        }
        out.ad[i] = f;
    }
}

/// What an AD row's flags say, most severe first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdStatus {
    /// The scope is not bound to a tenant: nothing is compared.
    OutOfScope,
    /// The configured anchor is present but malformed or nil.
    InvalidSourceAnchor,
    /// Another AD row carries the same anchor.
    AmbiguousSource,
    /// Several Entra rows carry the anchor.
    AmbiguousTarget,
    /// The witnesses disagree.
    Contradictory,
    /// Forward and backsync agree, observed at aligned times.
    Confirmed,
    /// Forward and backsync agree, but the observations are not aligned in
    /// time (or a time is unknown): no claim that both held at once.
    AgreesAcrossTime,
    /// Only the forward anchor witness; no backsync to confirm it.
    ForwardOnly,
    /// Only the backsync witness; the configured anchor is absent.
    BacksyncOnly,
    /// The configured anchor is absent and nothing else relates the row.
    MissingSourceAnchor,
    /// The anchor is present and matches nothing.
    Unresolved,
}

impl AdStatus {
    /// Read an AD row's flags.
    pub fn of(f: u32) -> Self {
        use flag::*;
        if f & UNBOUND_SCOPE != 0 {
            Self::OutOfScope
        } else if f & (ANCHOR_MALFORMED | ANCHOR_NIL) != 0 {
            Self::InvalidSourceAnchor
        } else if f & ANCHOR_SHARED != 0 {
            Self::AmbiguousSource
        } else if f & FWD_MANY != 0 {
            Self::AmbiguousTarget
        } else if f & (CONTRADICT | TARGET_SPLIT) != 0 {
            Self::Contradictory
        } else if f & AGREE != 0 {
            if f & TIME_ALIGNED != 0 {
                Self::Confirmed
            } else {
                Self::AgreesAcrossTime
            }
        } else if f & FWD_ONE != 0 {
            Self::ForwardOnly
        } else if f & ANCHOR_ABSENT != 0 {
            if f & BACK_ONE != 0 {
                Self::BacksyncOnly
            } else {
                Self::MissingSourceAnchor
            }
        } else {
            Self::Unresolved
        }
    }
}

const CONTESTED_AD: u32 = flag::FWD_MANY | flag::ANCHOR_SHARED | flag::BACK_MANY;
const CONTESTED_ENTRA: u32 = flag::E_ANCHOR_SHARED | flag::E_SPLIT;

/// One population on one ruler: slot `i` of every column is the same user
/// slot (the directory's shared `u16` ordinal space, at most 65,536 slots).
/// [`Guid128::NIL`] marks an empty cell — no value, or no counterpart.
///
/// With the rulers held together, correspondence needs no join: each
/// witness is a position-wise compare of two columns ([`fold_aligned`]).
/// A join ([`fold`]) is only how unaligned observations are put onto the
/// ruler, once, cold ([`Ruler::from_fold`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ruler {
    /// AD source anchor.
    pub ad_anchor: Vec<Guid128>,
    /// AD backsync id.
    pub ad_backsync: Vec<Guid128>,
    /// AD observation time.
    pub ad_at: Vec<i64>,
    /// Entra `onPremisesImmutableId`.
    pub entra_anchor: Vec<Guid128>,
    /// Entra id.
    pub entra_id: Vec<Guid128>,
    /// Entra observation time.
    pub entra_at: Vec<i64>,
    /// Exchange Online `ExternalDirectoryObjectId`.
    pub exo_external: Vec<Guid128>,
    /// [`Profile::max_skew_ms`].
    pub max_skew_ms: i64,
}

impl Ruler {
    /// Slot count.
    pub fn len(&self) -> usize {
        self.ad_anchor.len()
    }
    /// No slots.
    pub fn is_empty(&self) -> bool {
        self.ad_anchor.is_empty()
    }

    /// Cold: lay a joined population on the ruler, slot = AD row. Only an
    /// unambiguous counterpart is placed (exactly one forward target, or
    /// failing that exactly one backsync target; exactly one mailbox);
    /// anything else leaves the cell empty, so ambiguity can never be read
    /// back as agreement.
    pub fn from_fold(l: &Lanes, ix: &Index, out: &Output) -> Self {
        let n = l.ad.len();
        let mut r = Self {
            ad_anchor: vec![Guid128::NIL; n],
            ad_backsync: vec![Guid128::NIL; n],
            ad_at: l.ad.at.clone(),
            entra_anchor: vec![Guid128::NIL; n],
            entra_id: vec![Guid128::NIL; n],
            entra_at: vec![0; n],
            exo_external: vec![Guid128::NIL; n],
            max_skew_ms: l.max_skew_ms,
        };
        // The one mailbox of each Entra row, if exactly one.
        let mut mailbox = vec![NO_ROW; l.entra.len()];
        merge(&ix.exo_by_external, &ix.entra_by_id, |x, e| {
            for y in e {
                mailbox[y.row as usize] = sole(x);
            }
        });
        for i in 0..n {
            if l.ad_anchor.state[i] == state::PRESENT && out.ad[i] & flag::ANCHOR_SHARED == 0 {
                r.ad_anchor[i] = l.ad_anchor.id[i];
            }
            if l.ad_backsync.state[i] == state::PRESENT {
                r.ad_backsync[i] = l.ad_backsync.id[i];
            }
            let j = match out.ad_target[i] {
                NO_ROW => out.ad_back[i],
                t => t,
            };
            // A contested counterpart is not placed: a slot on the ruler
            // stands for one row on each side, never for a choice among
            // several.
            if j == NO_ROW || out.ad[i] & CONTESTED_AD != 0 {
                continue;
            }
            let j = j as usize;
            if out.entra[j] & CONTESTED_ENTRA != 0 {
                continue;
            }
            if l.entra_anchor.state[j] == state::PRESENT {
                r.entra_anchor[i] = l.entra_anchor.id[j];
            }
            r.entra_id[i] = l.entra.owner[j];
            r.entra_at[i] = l.entra.at[j];
            if mailbox[j] != NO_ROW {
                r.exo_external[i] = l.exo_external.id[mailbox[j] as usize];
            }
        }
        r
    }
}

/// Bit planes over the ruler's slots (bit `i % 64` of word `i / 64`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Planes {
    /// AD anchor = Entra immutable id, both non-empty.
    pub forward: Vec<u64>,
    /// AD backsync = Entra id, both non-empty.
    pub backsync: Vec<u64>,
    /// Entra id = Exchange Online external id, both non-empty.
    pub cloud: Vec<u64>,
    /// Both observation times known and within the skew.
    pub aligned: Vec<u64>,
    /// Both sides of the forward witness non-empty, but different.
    pub forward_conflict: Vec<u64>,
    /// Both sides of the backsync witness non-empty, but different.
    pub backsync_conflict: Vec<u64>,
}

impl Planes {
    /// Allocate for a ruler.
    pub fn for_ruler(r: &Ruler) -> Self {
        let w = r.len().div_ceil(64);
        Self {
            forward: vec![0; w],
            backsync: vec![0; w],
            cloud: vec![0; w],
            aligned: vec![0; w],
            forward_conflict: vec![0; w],
            backsync_conflict: vec![0; w],
        }
    }

    /// Confirmed: forward and backsync agree at aligned times.
    pub fn confirmed(&self, word: usize) -> u64 {
        self.forward[word] & self.backsync[word] & self.aligned[word]
    }
}

/// Hot: every witness as a position-wise compare. No join, no index, no
/// allocation; `out` must come from [`Planes::for_ruler`].
pub fn fold_aligned(r: &Ruler, out: &mut Planes) {
    let n = r.len();
    for w in 0..n.div_ceil(64) {
        let (lo, hi) = (w * 64, ((w + 1) * 64).min(n));
        let (mut fw, mut bk, mut cl, mut al, mut fc, mut bc) = (0u64, 0, 0, 0, 0, 0);
        for i in lo..hi {
            let sh = i - lo;
            let (a, e) = (k(&r.ad_anchor[i]), k(&r.entra_anchor[i]));
            let both = (a != 0) & (e != 0);
            fw |= ((both & (a == e)) as u64) << sh;
            fc |= ((both & (a != e)) as u64) << sh;
            let (b, id) = (k(&r.ad_backsync[i]), k(&r.entra_id[i]));
            let both = (b != 0) & (id != 0);
            bk |= ((both & (b == id)) as u64) << sh;
            bc |= ((both & (b != id)) as u64) << sh;
            let x = k(&r.exo_external[i]);
            cl |= (((x != 0) & (id != 0) & (x == id)) as u64) << sh;
            let (t0, t1) = (r.ad_at[i], r.entra_at[i]);
            al |= (((t0 != 0) & (t1 != 0) & ((t0 - t1).abs() <= r.max_skew_ms)) as u64) << sh;
        }
        out.forward[w] = fw;
        out.backsync[w] = bk;
        out.cloud[w] = cl;
        out.aligned[w] = al;
        out.forward_conflict[w] = fc;
        out.backsync_conflict[w] = bc;
    }
}
