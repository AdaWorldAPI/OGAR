//! # ogar-obo — the OBO-core reference bake
//!
//! Parses the OBO Foundry biological-ontology core — **MONDO** (disease),
//! **HPO** (phenotype/symptom), **Uberon** (anatomy), **PATO** (quality),
//! **RO** (relations) — into a canonical **512-byte SoA `NodeRow` buffer**
//! that the lance-graph loader reads **zero-copy**, plus the OWL-EL
//! completion subset the OBO EL profile actually exercises (see [`reason`]).
//!
//! ## Why here (and why zero PHI)
//!
//! OBO is public CC-BY / CC0 reference vocabulary — the biological *backbone*
//! every clinical consumer attaches to. It lives in OGAR (the public
//! producer / codebook) next to [`ogar_vocab`], `ogar-fma`, `ogar-cpic`, and
//! carries **no patient data and no consumer-private codebook** — a consumer
//! wires it exactly as it wires `ogar-fma`. The Anatomy-vs-Health firewall
//! ([`ogar_vocab`] `canonical_concept` docs) holds: the OBO reference lives in
//! the public `0x03` Ontology domain, never the `0x09` Health PHI domain.
//!
//! ## The loader connection is a BYTE-LAYOUT contract, not a code dep
//!
//! This crate stays lean (deps: `ogar-vocab` only). It emits the exact
//! little-endian 512-byte `NodeRow` geometry
//! (`lance_graph_contract::canonical_node`): `key(16) | edges(16) |
//! value(480)`. lance-graph's `node_rows_from_le_bytes` reads those bytes
//! back as `&[NodeRow]` with no deserialize — so byte compatibility, verified
//! by fixture ([`tests`] + a lance-graph-side load test), IS the connection.
//! No cross-repo compile coupling.
//!
//! ```text
//!   key (16, LE)                            edges (16)         value (480)
//!   ┌────────┬────┬────┬────┬──────┬──────┐ ┌──────────────┐  ┌─────────────┐
//!   classid  HEEL HIP  TWIG family identity 12 in + 4 out     tenant slab
//!   u32(4)   u16  u16  u16  u24(3) u24(3)   1 byte per slot   (EntityType…)
//!   └────────┴────┴────┴────┴──────┴──────┘ └──────────────┘  └─────────────┘
//! ```
//!
//! The CURIE numeric id (`MONDO:0007739` → `7739`) is the node's 24-bit
//! **identity**; the namespace is the **classid** (canon-high `concept<<16 |
//! app`). All OBO numeric ids fit in 24 bits (< 16.7 M).

#![deny(missing_docs)]

pub mod crosswalk;
pub mod edges;
pub mod reason;
pub mod spine;

/// Row stride of the canonical SoA node — `key(16) + edges(16) + value(480)`.
/// Mirrors `lance_graph_contract::canonical_node::NODE_ROW_STRIDE`.
pub const NODE_ROW_STRIDE: usize = 512;
/// Byte offset of the edge block within a row.
pub const EDGES_OFFSET: usize = 16;
/// Byte offset of the value slab within a row.
pub const VALUE_OFFSET: usize = 32;
/// `EntityType` value-tenant offset **within the 480-byte value slab**
/// (tenant ordinal 8; see the contract `VALUE_TENANTS` carve). A `u16`
/// that records this node's OBO namespace so a reader can group by ontology
/// without decoding the classid.
pub const ENTITY_TYPE_SLAB_OFFSET: usize = 96;

/// The five OBO-core namespaces this bake carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Namespace {
    /// MONDO — disease (`0x0301`, the `0x03` OBO Ontology domain).
    Mondo,
    /// HPO — human phenotype / clinical symptom (`0x0302`).
    Hpo,
    /// Uberon — anatomy spine (`0x0303`). Cross-references FMA (`ogar-fma`,
    /// `0x0A` Anatomy) by edge, not by shared domain byte.
    Uberon,
    /// PATO — phenotypic quality (`0x0304`).
    Pato,
    /// RO — relations ontology (predicate classes; `0x0305`). Predicates ride
    /// the [`Predicate`] byte palette on edges, not as node classids, but RO
    /// term nodes are still baked for completeness.
    Ro,
}

impl Namespace {
    /// The OBO CURIE prefix (`"MONDO"`, `"HP"`, `"UBERON"`, `"PATO"`, `"RO"`).
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Namespace::Mondo => "MONDO",
            Namespace::Hpo => "HP",
            Namespace::Uberon => "UBERON",
            Namespace::Pato => "PATO",
            Namespace::Ro => "RO",
        }
    }

    /// Parse an OBO CURIE prefix to a namespace.
    #[must_use]
    pub fn from_prefix(p: &str) -> Option<Self> {
        Some(match p {
            "MONDO" => Namespace::Mondo,
            "HP" => Namespace::Hpo,
            "UBERON" => Namespace::Uberon,
            "PATO" => Namespace::Pato,
            "RO" => Namespace::Ro,
            _ => return None,
        })
    }

    /// The canonical hi-u16 **concept id** (`0xDDCC`, domain `DD` · slot `CC`)
    /// this namespace routes on — the public-reference assignment. All five
    /// OBO namespaces live in the `0x03` **Ontology** domain (public reference,
    /// firewall-separated from `0x09` Health PHI). The domain is reserved in
    /// [`ogar_vocab`] as `ConceptDomain::Ontology` with **zero shared CODEBOOK
    /// rows** — these concept ids are authoritative here, kept plug-and-play so
    /// only consumers that dep `ogar-obo` compile them (ERP / project consumers
    /// never pull them in).
    #[must_use]
    pub const fn concept_id(self) -> u16 {
        match self {
            Namespace::Mondo => 0x0301,
            Namespace::Hpo => 0x0302,
            Namespace::Uberon => 0x0303,
            Namespace::Pato => 0x0304,
            Namespace::Ro => 0x0305,
        }
    }

    /// The full V3 render classid under a consumer's app prefix — canon-high
    /// `(concept as u32) << 16 | app_prefix`. Identical idiom to
    /// `ogar_fma::FmaStructure::render_classid` and `ogar_vocab::render_classid`.
    #[must_use]
    pub const fn render_classid(self, app_prefix: u16) -> u32 {
        ((self.concept_id() as u32) << 16) | (app_prefix as u32)
    }
}

/// The RO-predicate byte palette carried on edges (the `P` of an SPO triple).
/// One byte indexes the relationship type; the small core set the OBO EL
/// profile uses. `0` is reserved (unset / fall-through).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Predicate {
    /// `is_a` / `rdfs:subClassOf` — the subsumption spine (transitive).
    IsA = 1,
    /// `BFO:0000050` part_of — mereology (transitive; the anatomy backbone).
    PartOf = 2,
    /// `RO:0002200` has_phenotype — disease → HPO manifestation.
    HasPhenotype = 3,
    /// `RO:0004026` disease_has_location — disease → Uberon site.
    HasLocation = 4,
    /// existential toward an anatomy genus (`has_part`/`inheres_in` → Uberon)
    /// harvested from an HPO logical definition (`intersection_of`).
    HasAnatomy = 5,
    /// existential toward a PATO quality harvested from a logical definition.
    HasQuality = 6,
    /// any other `relationship:` predicate, preserved generically.
    Other = 7,
}

/// A parsed OBO term id: `(namespace, 24-bit numeric)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TermId {
    /// Ordinal namespace tag (see [`Namespace`]).
    pub ns: u8,
    /// The CURIE numeric id (fits 24 bits for every OBO id).
    pub num: u32,
}

impl TermId {
    /// Parse a CURIE like `"MONDO:0007739"` into a [`TermId`], or `None` if the
    /// prefix is outside the OBO core or the numeric part is malformed / >24-bit.
    #[must_use]
    pub fn parse(curie: &str) -> Option<TermId> {
        let (p, n) = curie.split_once(':')?;
        let ns = Namespace::from_prefix(p)?;
        let num: u32 = n.parse().ok()?;
        if num > 0x00FF_FFFF {
            return None;
        }
        Some(TermId { ns: ns as u8, num })
    }

    /// This term's namespace.
    #[must_use]
    pub fn namespace(self) -> Namespace {
        // ns is always constructed from a Namespace discriminant.
        [
            Namespace::Mondo,
            Namespace::Hpo,
            Namespace::Uberon,
            Namespace::Pato,
            Namespace::Ro,
        ][self.ns as usize]
    }
}

/// An external cross-reference — the `xref:` lines OBO carries to the
/// clinical/coding world (MeSH, UMLS, OMIM, Orphanet, SNOMED, ICD, …).
///
/// **These are load-bearing, never truncated.** They are (a) the horizontal
/// **projection-join / multilateration bearings** — the "who am I" evidence a
/// broadMatch is triangulated against — and (b) the **guideline-spider path**:
/// a MeSH reference resolves a disease/phenotype to its clinical guideline,
/// pulled online→local by a consumer, even when the end surface is "just" a
/// basic PDF viewer. Dropping xrefs during CURIE parsing would silently kill
/// both downstreams.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Xref {
    /// which external coding system this reference points into
    pub source: XrefSource,
    /// the external id verbatim (e.g. `"D003922"` for `MeSH:D003922`) — kept as
    /// a string because external coding systems are not the OBO 24-bit id scheme
    pub id: String,
}

/// External coding systems OBO xrefs point into — the projection lanes. MeSH is
/// first-class (the guideline-spider source); everything unrecognised is
/// preserved verbatim in [`XrefSource::Other`], never dropped.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum XrefSource {
    /// MeSH (`MeSH` / `MSH` / `MESH`) — the guideline-spider bearing.
    Mesh,
    /// UMLS Metathesaurus CUI.
    Umls,
    /// OMIM (Mendelian).
    Omim,
    /// Orphanet (`Orphanet` / `ORPHA`).
    Orphanet,
    /// SNOMED CT (`SCTID` / `SNOMEDCT`).
    Snomed,
    /// ICD-10 / ICD-10-CM / ICD-9.
    Icd,
    /// any other coding system — the prefix preserved verbatim.
    Other(String),
}

impl XrefSource {
    /// Map an xref CURIE prefix to a source (case-insensitive on the known set).
    #[must_use]
    pub fn from_prefix(p: &str) -> XrefSource {
        match p.to_ascii_uppercase().as_str() {
            "MESH" | "MSH" => XrefSource::Mesh,
            "UMLS" => XrefSource::Umls,
            "OMIM" | "OMIMPS" | "MIM" => XrefSource::Omim,
            "ORPHANET" | "ORPHA" => XrefSource::Orphanet,
            "SCTID" | "SNOMEDCT" | "SNOMEDCT_US" | "SNOMED" => XrefSource::Snomed,
            "ICD10" | "ICD10CM" | "ICD-10" | "ICD9" | "ICD9CM" => XrefSource::Icd,
            _ => XrefSource::Other(p.to_string()),
        }
    }
}

/// One OBO node with its outgoing typed edges + preserved xrefs (the
/// intermediate the bake packs).
#[derive(Debug, Clone, Default)]
pub struct OboNode {
    /// `is_a` parents.
    pub is_a: Vec<TermId>,
    /// typed relationships `(predicate, object)`.
    pub rel: Vec<(Predicate, TermId)>,
    /// external cross-references — the projection-join / guideline-spider
    /// bearings, preserved verbatim (never truncated).
    pub xref: Vec<Xref>,
    /// whether the term is obsolete (kept for the read-only tail; not tiled).
    pub obsolete: bool,
}

/// A 64-byte-aligned 512-byte row cell — the alignment `node_rows_from_le_bytes`
/// requires for a genuine zero-copy `&[NodeRow]` view. Emitting into a
/// `Vec<Row512>` (not a bare `Vec<u8>`) guarantees the buffer pointer is
/// 64-aligned, so lance-graph reads it in place.
#[derive(Clone, Copy)]
#[repr(C, align(64))]
pub struct Row512(pub [u8; NODE_ROW_STRIDE]);

impl Row512 {
    /// Zero row.
    #[must_use]
    pub const fn zeroed() -> Self {
        Row512([0u8; NODE_ROW_STRIDE])
    }
}

/// Build the 16-byte canon key: `classid(4) | HEEL(2) | HIP(2) | TWIG(2) |
/// family(3) | identity(3)`, all little-endian. HEEL/HIP/TWIG (HHTL cascade)
/// and family stay `0` in this first bake — the zero-fallback ladder means
/// `identity` alone discriminates until an HHTL basin mint wakes routing
/// (RESERVE, DON'T RECLAIM: fixed offsets, zero layout change later).
pub(crate) fn pack_key(classid: u32, identity: u32) -> [u8; 16] {
    let mut k = [0u8; 16];
    k[0..4].copy_from_slice(&classid.to_le_bytes());
    // 4..10 HEEL/HIP/TWIG = 0 (dormant cascade)
    // 10..12 leaf = 0 (dormant 4th HHTL tier — RESERVE, DON'T RECLAIM)
    //
    // V3 tail (`canonical_node::NodeGuid::new_v2`): leaf:u16 @10..12,
    // family:u16 @12..14, identity:u16 @14..16. This was the V1 u24 tail
    // (identity's low 3 bytes at 13..16, family zeroed) until 2026-08-05;
    // migrated because the contract defaults to `guid-v3-tail` and there are
    // no pre-flip rows of this bake to preserve.
    //
    // The CURIE numeric does NOT fit `identity` alone — OBO ids run past
    // 65_535 (MONDO:0700092 = 700_092), and `mint_for`'s V2/V3 arm asserts
    // `identity <= 0xFFFF` rather than truncating. So the 24-bit numeric is
    // carried by the family:identity RAIL, high half in `family`:
    //   family   = num >> 16      (0..=255 for a 24-bit CURIE)
    //   identity = num & 0xFFFF
    // That is the point of V3 over the flat u24 — a u24 has no axis and
    // cannot carry an `X:Y` rail; this pair can, and it is lossless for the
    // full 24 bits with 8 bits of headroom left in `family`.
    //
    // Sort order is UNCHANGED: `family` holds the high bits, so ordering by
    // (classid, family, identity) is ordering by (classid, num) — the
    // binary-search-by-key invariant over the baked artifact survives.
    let f = ((identity >> 16) as u16).to_le_bytes();
    let i = ((identity & 0xFFFF) as u16).to_le_bytes();
    k[12] = f[0];
    k[13] = f[1];
    k[14] = i[0];
    k[15] = i[1];
    k
}

/// Read a row's key back as `(classid, CURIE numeric)` — the inverse of
/// `pack_key`, recombining the `family:identity` rail into the 24-bit numeric.
///
/// This is the *whole* prerender property in one call: a reader learns which
/// class a row is and which instance, with **zero value decode**.
#[must_use]
pub const fn unpack_key(row: &[u8; NODE_ROW_STRIDE]) -> (u32, u32) {
    let classid = u32::from_le_bytes([row[0], row[1], row[2], row[3]]);
    let family = u16::from_le_bytes([row[12], row[13]]) as u32;
    let identity = u16::from_le_bytes([row[14], row[15]]) as u32;
    (classid, (family << 16) | identity)
}

/// Pack one node into its 512-byte row.
///
/// The in-row `edges` block records the *degree* per predicate (a one-byte
/// histogram, saturating at 255) — the CSR `row_ptr`. The **targets** (`col_idx`)
/// ride the free value lanes as G2 `4×u24` literal links; see [`edges`] for the
/// lane form and the reader's contract. The `EntityType` value tenant records
/// the namespace. The same edges are also emitted out-of-line as SPO triples by
/// [`bake`] — the in-row copy is what makes a row traversable on its own.
///
/// Returns the row plus the pack stats, so a driver can refuse to ship a
/// truncation rather than lose edges silently.
fn pack_row(
    classid: u32,
    id: &TermId,
    node: &OboNode,
    app_prefix: u16,
) -> (Row512, edges::PackStats) {
    let mut row = Row512::zeroed();
    row.0[0..16].copy_from_slice(&pack_key(classid, id.num));
    // edges block [16,32): a per-predicate degree histogram (index by Predicate
    // discriminant), saturating. Slot 0 reserved.
    let mut deg = [0u16; 8];
    deg[Predicate::IsA as usize] = node.is_a.len() as u16;
    for (p, _) in &node.rel {
        deg[*p as usize] = deg[*p as usize].saturating_add(1);
    }
    for (i, d) in deg.iter().enumerate() {
        row.0[EDGES_OFFSET + i] = (*d).min(255) as u8;
    }
    // value slab: EntityType tenant (u16 namespace) at its carve offset.
    let et = VALUE_OFFSET + ENTITY_TYPE_SLAB_OFFSET;
    row.0[et..et + 2].copy_from_slice(&(id.ns as u16).to_le_bytes());
    // value slab: the edge targets — the col_idx half of the CSR.
    let mut targets: Vec<(Predicate, TermId)> =
        Vec::with_capacity(node.is_a.len() + node.rel.len());
    targets.extend(node.is_a.iter().map(|t| (Predicate::IsA, *t)));
    targets.extend(node.rel.iter().copied());
    let st = edges::pack_edges(&mut row.0, &mut targets, app_prefix);
    (row, st)
}

// ── OBO `.obo` parser ─────────────────────────────────────────────────────

/// An SPO edge: `subject --predicate--> object`, the out-of-line triple form
/// lance-graph's SPO store consumes natively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Triple {
    /// subject term
    pub s: TermId,
    /// predicate (RO palette)
    pub p: Predicate,
    /// object term
    pub o: TermId,
}

/// Classify a raw `(subject_ns, object)` edge into an RO [`Predicate`] by the
/// namespace pair — the robust, measurement-matched mapping (§backbone
/// convergence): the predicate is implied by which two ontologies an edge
/// joins, so we never depend on the exact `relationship:` label string.
fn classify(subj: Namespace, obj: Namespace) -> Predicate {
    use Namespace::{Hpo, Mondo, Pato, Uberon};
    match (subj, obj) {
        (Mondo, Hpo) => Predicate::HasPhenotype,
        (Mondo, Uberon) => Predicate::HasLocation,
        (Hpo, Uberon) => Predicate::HasAnatomy,
        (Hpo, Pato) => Predicate::HasQuality,
        (Uberon, Uberon) => Predicate::PartOf,
        _ => Predicate::Other,
    }
}

/// Parse one OBO `.obo` document's `[Term]` stanzas into `(id -> node)`.
/// Recognises `id:`, `is_obsolete:`, `is_a:`, `relationship:` and
/// `intersection_of:` (the logical-definition genus/differentia); every
/// CURIE outside the OBO core (imported CL / GO / CHEBI / BFO relations …)
/// is silently skipped — only the five core namespaces are tiled.
#[must_use]
pub fn parse_obo(text: &str) -> std::collections::HashMap<TermId, OboNode> {
    use std::collections::HashMap;
    let mut nodes: HashMap<TermId, OboNode> = HashMap::new();
    let mut cur: Option<TermId> = None;
    let mut in_term = false;
    for line in text.lines() {
        if line == "[Term]" {
            in_term = true;
            cur = None;
            continue;
        }
        if line.starts_with('[') {
            in_term = false;
            cur = None;
            continue;
        }
        if !in_term {
            continue;
        }
        if let Some(rest) = line.strip_prefix("id: ") {
            cur = TermId::parse(rest.trim());
            if let Some(id) = cur {
                nodes.entry(id).or_default();
            }
        } else if line.starts_with("is_obsolete: true") {
            if let Some(id) = cur {
                nodes.entry(id).or_default().obsolete = true;
            }
        } else if let Some(rest) = line.strip_prefix("is_a: ") {
            if let (Some(sid), Some(t)) = (cur, TermId::parse(first_curie(rest))) {
                nodes.entry(sid).or_default().is_a.push(t);
            }
        } else if let Some(rest) = line.strip_prefix("relationship: ") {
            // `relationship: <REL> <TARGET> ! label` — the target is the LAST
            // whitespace token before any `!`.
            if let Some(sid) = cur
                && let Some(t) = last_curie(rest)
            {
                let p = classify(sid.namespace(), t.namespace());
                nodes.entry(sid).or_default().rel.push((p, t));
            }
        } else if let Some(rest) = line.strip_prefix("intersection_of: ") {
            if let Some(sid) = cur
                && let Some(t) = last_curie(rest)
            {
                let p = classify(sid.namespace(), t.namespace());
                nodes.entry(sid).or_default().rel.push((p, t));
            }
        } else if let Some(rest) = line.strip_prefix("xref: ") {
            // `xref: <SOURCE>:<ID> ! label` — the projection-join / guideline
            // bearing. Kept verbatim; NEVER truncated (MeSH → Leitlinie spider).
            if let Some(sid) = cur {
                let tok = rest.split('!').next().unwrap_or(rest).trim();
                let tok = tok.split_whitespace().next().unwrap_or(tok);
                if let Some((src, id)) = tok.split_once(':')
                    && !src.is_empty()
                    && !id.is_empty()
                {
                    nodes.entry(sid).or_default().xref.push(Xref {
                        source: XrefSource::from_prefix(src),
                        id: id.to_string(),
                    });
                }
            }
        }
    }
    nodes
}

/// First whitespace token of a line (before any `!` comment) — the target of
/// an `is_a:` line.
fn first_curie(s: &str) -> &str {
    s.split('!')
        .next()
        .unwrap_or(s)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim()
}

/// Last CURIE-shaped token before any `!` — the object of a `relationship:` /
/// `intersection_of:` line (the predicate is the earlier token).
fn last_curie(s: &str) -> Option<TermId> {
    let head = s.split('!').next().unwrap_or(s);
    head.split_whitespace()
        .rfind(|t| t.contains(':'))
        .and_then(TermId::parse)
}

// ── bake: nodes+edges → 512-byte rows + SPO triples + stats ────────────────

/// Validation invariants recomputed from the baked graph — the fidelity /
/// structural / convergence numbers the OBO-core validation pass asserts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BakeStats {
    /// live (non-obsolete) core nodes tiled into rows
    pub nodes: usize,
    /// obsolete terms (kept in the map, not tiled)
    pub obsolete: usize,
    /// total SPO edges emitted
    pub triples: usize,
    /// `is_a` cycles found (SCC size > 1) — must be 0 on a clean OBO core
    pub is_a_cycles: usize,
    /// edges whose object namespace is core but the object node is absent
    pub dangling: usize,
    /// MONDO→HP has_phenotype edges resolving to a loaded HP node
    pub mondo_hp: usize,
    /// HP→Uberon has_anatomy edges resolving (the logical-def grounding)
    pub hp_uberon: usize,
    /// HP→PATO has_quality edges resolving
    pub hp_pato: usize,
    /// total preserved xrefs (projection-join / guideline bearings)
    pub xrefs: usize,
    /// MeSH xrefs specifically — the guideline-spider bearings
    pub mesh_xrefs: usize,
    /// edge targets written into value lanes — the CSR `col_idx`. Equals
    /// [`BakeStats::triples`] on a clean bake (every edge is also in-row).
    pub links_packed: usize,
    /// edge targets that did NOT fit the lane budget. **Must be 0 to ship** —
    /// a non-zero value means the artifact is missing edges.
    pub links_dropped: usize,
    /// rows that overflowed the lane budget (must be 0 to ship)
    pub rows_overflowed: usize,
}

/// The bake result: the 512-byte SoA rows (loader-facing) + the SPO triple
/// table (lance-graph SPO-store-facing) + the validation stats.
pub struct Bake {
    /// One 64-aligned 512-byte row per live core node, sorted by `(ns, num)`
    /// so the buffer is a deterministic, HHTL-ready ordering.
    pub rows: Vec<Row512>,
    /// The node ids in row order (row `i` ↔ `ids[i]`).
    pub ids: Vec<TermId>,
    /// SPO edges (out-of-line; the EdgeBlock byte model is the later HHTL pass).
    pub triples: Vec<Triple>,
    /// preserved external cross-references — the projection-join /
    /// guideline-spider table: `(subject term, xref)`. NEVER truncated; MeSH
    /// entries are what a consumer resolves to a Leitlinie (online→local PDF).
    pub xrefs: Vec<(TermId, Xref)>,
    /// recomputed invariants
    pub stats: BakeStats,
}

/// Bake a merged `(id → node)` map (all five `.obo` unioned, plus any HPO
/// logical-def edges folded into the nodes' `rel`) into [`Bake`]. `app_prefix`
/// is the lo-u16 render skin (`0x0000` = the canonical reference skin).
#[must_use]
pub fn bake(nodes: &std::collections::HashMap<TermId, OboNode>, app_prefix: u16) -> Bake {
    let mut ids: Vec<TermId> = nodes
        .iter()
        .filter(|(_, n)| !n.obsolete)
        .map(|(id, _)| *id)
        .collect();
    ids.sort_unstable();

    let mut rows = Vec::with_capacity(ids.len());
    let mut triples = Vec::new();
    let mut xrefs = Vec::new();
    let mut stats = BakeStats {
        obsolete: nodes.values().filter(|n| n.obsolete).count(),
        ..Default::default()
    };

    for id in &ids {
        let node = &nodes[id];
        let classid = id.namespace().render_classid(app_prefix);
        let (row, pk) = pack_row(classid, id, node, app_prefix);
        rows.push(row);
        stats.links_packed += pk.written;
        stats.links_dropped += pk.dropped;
        stats.rows_overflowed += pk.rows_overflowed;
        // Preserve every external cross-reference — the projection-join /
        // guideline-spider bearings. NEVER truncated.
        for x in &node.xref {
            if x.source == XrefSource::Mesh {
                stats.mesh_xrefs += 1;
            }
            xrefs.push((*id, x.clone()));
        }
        for parent in &node.is_a {
            triples.push(Triple {
                s: *id,
                p: Predicate::IsA,
                o: *parent,
            });
            if !nodes.contains_key(parent) {
                stats.dangling += 1;
            }
        }
        for (p, o) in &node.rel {
            triples.push(Triple {
                s: *id,
                p: *p,
                o: *o,
            });
            if !nodes.contains_key(o) {
                stats.dangling += 1;
            } else {
                match p {
                    Predicate::HasPhenotype => stats.mondo_hp += 1,
                    Predicate::HasAnatomy => stats.hp_uberon += 1,
                    Predicate::HasQuality => stats.hp_pato += 1,
                    _ => {}
                }
            }
        }
    }
    stats.nodes = ids.len();
    stats.triples = triples.len();
    stats.xrefs = xrefs.len();
    stats.is_a_cycles = reason::count_is_a_cycles(&triples);
    Bake {
        rows,
        ids,
        triples,
        xrefs,
        stats,
    }
}

// ── the loader connection: 64-aligned LE bytes ↔ &[Row512] ─────────────────

/// Zero-copy view of the row buffer as contiguous LE bytes — the packet a
/// Lance `FixedSizeBinary(512)` column stores, and exactly what
/// `lance_graph_contract::canonical_node::node_rows_from_le_bytes` reads back
/// as `&[NodeRow]`. Length is `rows.len() * 512`.
#[must_use]
pub fn as_le_bytes(rows: &[Row512]) -> &[u8] {
    // SAFETY: Row512 is #[repr(C, align(64))] wrapping [u8; 512] — plain bytes,
    // no padding, no niche. Viewing a &[Row512] as &[u8] of len*512 is the same
    // column-store packing lance-graph's own NodeRowPacket::as_le_bytes does;
    // every byte is init and valid for read, align(64) ⊇ align(1).
    unsafe { core::slice::from_raw_parts(rows.as_ptr().cast::<u8>(), rows.len() * NODE_ROW_STRIDE) }
}

/// The inverse — a zero-copy `&[Row512]` over an external LE buffer, mirroring
/// `node_rows_from_le_bytes`: `Some` iff the length is a whole number of
/// 512-byte rows AND the pointer is 64-aligned (so lance-graph's identical
/// check would also accept it — this is the cross-repo compatibility proof).
#[must_use]
pub fn rows_from_le_bytes(bytes: &[u8]) -> Option<&[Row512]> {
    if bytes.is_empty() {
        return Some(&[]);
    }
    if !bytes.len().is_multiple_of(NODE_ROW_STRIDE) {
        return None;
    }
    if !(bytes.as_ptr() as usize).is_multiple_of(core::mem::align_of::<Row512>()) {
        return None;
    }
    let n = bytes.len() / NODE_ROW_STRIDE;
    // SAFETY: length is an exact multiple of the stride and the pointer is
    // align_of::<Row512>()-aligned (both checked above); every 512-byte window
    // is a valid Row512 (all-bytes type, no invalid bit pattern).
    Some(unsafe { core::slice::from_raw_parts(bytes.as_ptr().cast::<Row512>(), n) })
}

/// Decode a row's key back to its `(classid, identity)` — the reader side of
/// [`pack_key`], used by the round-trip proof.
#[must_use]
pub fn decode_key(row: &Row512) -> (u32, u32) {
    let classid = u32::from_le_bytes([row.0[0], row.0[1], row.0[2], row.0[3]]);
    // V3 tail: reassemble the CURIE numeric from the family:identity rail
    // (family @12..14 = high half, identity @14..16 = low half). See
    // `pack_key` for why the numeric spans the pair rather than one field.
    let family = u16::from_le_bytes([row.0[12], row.0[13]]);
    let ident = u16::from_le_bytes([row.0[14], row.0[15]]);
    (classid, (u32::from(family) << 16) | u32::from(ident))
}

const _: () = assert!(core::mem::size_of::<Row512>() == 512);
const _: () = assert!(core::mem::align_of::<Row512>() == 64);

// ── HPO logical definitions (hp-base.owl) — the anatomy:quality grounding ──

/// Parse HPO's OWL logical definitions out of `hp-base.owl` (RDF/XML): the
/// `owl:equivalentClass` blocks that decompose a phenotype as
/// `has_part some (UBERON … and has_quality some PATO …)`. The plain `hp.obo`
/// release strips these, so HP→Uberon/PATO grounding needs this second source.
///
/// Returns `(HP term, predicate, UBERON|PATO term)` edges — `HasAnatomy` for a
/// UBERON filler, `HasQuality` for a PATO filler. Stateful line scan (balanced
/// `equivalentClass` depth); robust to the interleaved anonymous-class nesting.
#[must_use]
pub fn parse_hp_logical_defs(owl: &str) -> Vec<(TermId, Predicate, TermId)> {
    let mut out = Vec::new();
    let mut cur: Option<u32> = None;
    let mut in_eq: i32 = 0;
    for line in owl.lines() {
        // a named HP class opens a new subject
        if let Some(hp) = find_hp_about(line) {
            cur = Some(hp);
        }
        if line.contains("<owl:equivalentClass>") {
            in_eq += 1;
        }
        if in_eq > 0
            && let Some(sid) = cur
        {
            for uid in find_obo_ids(line, "UBERON_") {
                out.push((
                    TermId {
                        ns: Namespace::Hpo as u8,
                        num: sid,
                    },
                    Predicate::HasAnatomy,
                    TermId {
                        ns: Namespace::Uberon as u8,
                        num: uid,
                    },
                ));
            }
            for pid in find_obo_ids(line, "PATO_") {
                out.push((
                    TermId {
                        ns: Namespace::Hpo as u8,
                        num: sid,
                    },
                    Predicate::HasQuality,
                    TermId {
                        ns: Namespace::Pato as u8,
                        num: pid,
                    },
                ));
            }
        }
        if line.contains("</owl:equivalentClass>") {
            in_eq = (in_eq - 1).max(0);
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Extract the numeric id from a `owl:Class rdf:about="…/HP_NNNNNNN"` line.
fn find_hp_about(line: &str) -> Option<u32> {
    let i = line.find("owl:Class rdf:about=")?;
    let rest = &line[i..];
    let j = rest.find("HP_")?;
    let digits: String = rest[j + 3..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// Find every `PREFIX_NNNNNNN` numeric id on a line (e.g. `UBERON_0000955`).
fn find_obo_ids(line: &str, prefix: &str) -> Vec<u32> {
    let mut ids = Vec::new();
    let mut hay = line;
    while let Some(i) = hay.find(prefix) {
        let after = &hay[i + prefix.len()..];
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(n) = digits.parse::<u32>()
            && n <= 0x00FF_FFFF
        {
            ids.push(n);
        }
        hay = &after[digits.len()..];
    }
    ids
}

/// Fold logical-definition edges (from [`parse_hp_logical_defs`]) into a parsed
/// node map — the merge that completes the HP→Uberon:PATO half of the backbone.
pub fn merge_logical_defs(
    nodes: &mut std::collections::HashMap<TermId, OboNode>,
    defs: &[(TermId, Predicate, TermId)],
) {
    for (s, p, o) in defs {
        nodes.entry(*s).or_default().rel.push((*p, *o));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn n(ns: Namespace, num: u32) -> TermId {
        TermId { ns: ns as u8, num }
    }

    /// The V3 tail carries the CURIE numeric on the `family:identity` RAIL,
    /// and this test is the reason the rail exists rather than a bare
    /// `identity`.
    ///
    /// **What would make it fail:** putting the numeric in `identity` alone.
    /// Real OBO ids run past `u16` (MONDO:0700092 = 700_092), so a single-field
    /// tail either truncates to 41_500 or trips `mint_for`'s
    /// `identity <= 0xFFFF` assert. A small id like the `9` the existing
    /// round-trip test uses cannot distinguish those layouts — it fits either.
    ///
    /// Also pins the byte positions against `NodeGuid::new_v2`
    /// (leaf @10..12, family @12..14, identity @14..16) so a silent
    /// re-carve of the tail is caught here and not at a consumer.
    #[test]
    fn v3_tail_carries_oversize_curie_numerics_on_the_family_identity_rail() {
        // Above u16 — the case a bare `identity` cannot hold.
        let big = 700_092u32;
        assert!(
            big > u32::from(u16::MAX),
            "fixture must exceed the u16 the rail exists for"
        );
        let k = pack_key(Namespace::Mondo.render_classid(0x0000), big);

        // Byte positions, per new_v2.
        assert_eq!(
            &k[10..12],
            &[0, 0],
            "leaf stays dormant (RESERVE, DON'T RECLAIM)"
        );
        let family = u16::from_le_bytes([k[12], k[13]]);
        let identity = u16::from_le_bytes([k[14], k[15]]);
        assert_eq!(u32::from(family), big >> 16, "family holds the high half");
        assert_eq!(
            u32::from(identity),
            big & 0xFFFF,
            "identity holds the low half"
        );

        // Lossless through the public reader.
        let mut row = Row512::zeroed();
        row.0[0..16].copy_from_slice(&k);
        let (cid, num) = decode_key(&row);
        assert_eq!(num, big, "the rail round-trips the full 24-bit numeric");
        assert_eq!(cid, Namespace::Mondo.render_classid(0x0000));

        // The V1 read of the SAME bytes must NOT agree — proof the tail really
        // moved, not that both layouts happen to coincide on this fixture.
        let v1_read = u32::from_le_bytes([row.0[13], row.0[14], row.0[15], 0]);
        assert_ne!(
            v1_read, big,
            "a V1 u24 read of a V3 row must be observably wrong"
        );

        // Ordering is preserved: family is the HIGH half, so (family, identity)
        // sorts as the numeric does. This is what keeps binary-search-by-key
        // valid over the sorted bake.
        let lo = pack_key(Namespace::Mondo.render_classid(0x0000), 5_148);
        assert!(
            lo[12..16] < k[12..16],
            "tail bytes order as the numeric orders"
        );
    }

    #[test]
    fn curie_parse_and_classid_canon() {
        let t = TermId::parse("MONDO:0007739").unwrap();
        assert_eq!(t.namespace(), Namespace::Mondo);
        assert_eq!(t.num, 7739);
        // canon-high: (0x0301 << 16) | app_prefix ; all five in the 0x03 domain
        assert_eq!(Namespace::Mondo.render_classid(0x0000), 0x0301_0000);
        assert_eq!(Namespace::Uberon.render_classid(0x00AB), 0x0303_00AB);
        // out-of-scope / malformed
        assert!(TermId::parse("CHEBI:12345").is_none());
        assert!(TermId::parse("MONDO:99999999").is_none()); // > 24-bit
    }

    #[test]
    fn xrefs_are_preserved_never_truncated() {
        let obo = "\
[Term]\n\
id: MONDO:0005148\n\
name: type 2 diabetes mellitus\n\
xref: MeSH:D003924\n\
xref: ICD10:E11\n\
xref: SNOMEDCT:44054006\n\
is_a: MONDO:0005015 ! diabetes mellitus\n";
        let nodes = parse_obo(obo);
        let t = n(Namespace::Mondo, 5148);
        let node = &nodes[&t];
        assert_eq!(node.xref.len(), 3, "all three xrefs kept");
        assert!(
            node.xref
                .iter()
                .any(|x| x.source == XrefSource::Mesh && x.id == "D003924")
        );
        let bake = bake(&nodes, 0x0000);
        assert_eq!(bake.stats.mesh_xrefs, 1, "MeSH bearing counted");
        assert_eq!(bake.stats.xrefs, 3);
    }

    #[test]
    fn bake_round_trips_through_the_lance_graph_loader_contract() {
        // Build a tiny 3-node core and bake it.
        let mut nodes: HashMap<TermId, OboNode> = HashMap::new();
        nodes.insert(n(Namespace::Uberon, 300), OboNode::default());
        nodes.insert(
            n(Namespace::Uberon, 100),
            OboNode {
                is_a: vec![n(Namespace::Uberon, 300)],
                ..Default::default()
            },
        );
        nodes.insert(
            n(Namespace::Hpo, 9),
            OboNode {
                rel: vec![(Predicate::HasAnatomy, n(Namespace::Uberon, 100))],
                ..Default::default()
            },
        );
        let baked = bake(&nodes, 0x0000);
        assert_eq!(baked.rows.len(), 3);

        // The loader contract: as_le_bytes is 512×N and 64-aligned, and
        // rows_from_le_bytes (the mirror of node_rows_from_le_bytes) accepts it
        // and returns the SAME rows byte-for-byte — this is the cross-repo
        // zero-copy proof.
        let bytes = as_le_bytes(&baked.rows);
        assert_eq!(bytes.len(), 3 * NODE_ROW_STRIDE);
        assert_eq!(bytes.as_ptr() as usize % 64, 0, "64-aligned for zero-copy");
        let back = rows_from_le_bytes(bytes).expect("loader accepts the packet");
        assert_eq!(back.len(), 3);
        for (a, b) in baked.rows.iter().zip(back) {
            assert_eq!(a.0, b.0, "row bytes identical through the loader");
        }
        // key decode: row 0 is the lowest (ns,num) = HP:9 (Hpo ordinal 1) vs
        // Uberon ordinal 2 — ids sorted by (ns,num), so HP (ns=1) sorts first.
        let (classid0, id0) = decode_key(&back[0]);
        assert_eq!(id0, 9);
        assert_eq!(classid0, Namespace::Hpo.render_classid(0x0000));

        // A non-aligned / non-multiple buffer is rejected (loader would fall
        // back to a copy) — the can-it-refuse half.
        assert!(rows_from_le_bytes(&bytes[1..]).is_none());
    }

    #[test]
    fn hp_base_logical_def_scan_extracts_grounding() {
        // A minimal RDF/XML equivalentClass block in hp-base.owl shape.
        let owl = r#"
<owl:Class rdf:about="http://purl.obolibrary.org/obo/HP_0001250">
  <owl:equivalentClass>
    <owl:Class><owl:intersectionOf rdf:parseType="Collection">
      <owl:Restriction>
        <owl:someValuesFrom rdf:resource="http://purl.obolibrary.org/obo/UBERON_0000955"/>
      </owl:Restriction>
      <owl:Restriction>
        <owl:someValuesFrom rdf:resource="http://purl.obolibrary.org/obo/PATO_0000001"/>
      </owl:Restriction>
    </owl:intersectionOf></owl:Class>
  </owl:equivalentClass>
</owl:Class>
"#;
        let defs = parse_hp_logical_defs(owl);
        assert!(defs.contains(&(
            n(Namespace::Hpo, 1250),
            Predicate::HasAnatomy,
            n(Namespace::Uberon, 955)
        )));
        assert!(defs.contains(&(
            n(Namespace::Hpo, 1250),
            Predicate::HasQuality,
            n(Namespace::Pato, 1)
        )));
    }
}
