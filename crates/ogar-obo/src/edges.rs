//! The `col_idx` half of the CSR — edge targets packed into value lanes.
//!
//! # Why this module exists
//!
//! The bake's edge block at `[16,32)` is a per-predicate **degree histogram**:
//! it says *how many* targets a node has per predicate, but not *which*. In CSR
//! terms the bake persisted `row_ptr` and dropped `col_idx`, so the on-disk
//! artifact could describe a node's local shape but could not be traversed.
//! This module completes it: the targets ride the free value lanes, and the
//! degree histogram becomes load-bearing instead of decorative.
//!
//! # The lane form
//!
//! A lane is the one form the substrate has — `classid(4) + 12 content bytes`,
//! and the classid says how to read its own 12 bytes. An edge lane reads its
//! content as the **G2 `4×u24`** carving (`legacy_outliers::LegacyOutlier::
//! WideTriple`), i.e. four literal links, and its `classid` is **the target
//! ontology's own row classid**. So a link is exactly the `(classid, identity)`
//! pair a *key* carries — the same binary search that resolves a key resolves a
//! link. Nothing new is invented to point at a node.
//!
//! ```text
//! lane:  | classid: u32 | u24 | u24 | u24 | u24 |
//!          ^ target ontology     ^ four CURIE numerics
//! ```
//!
//! # Layout
//!
//! The 480-byte value slab is 30 lanes. Lane 6 is the `EntityType` tenant
//! ([`crate::ENTITY_TYPE_SLAB_OFFSET`]); lanes 0..6 stay reserved for further
//! value tenants (RESERVE, DON'T RECLAIM). Edges occupy lanes 7..30 —
//! [`EDGE_LANE_COUNT`] lanes = [`EDGE_SLOT_COUNT`] link slots.
//!
//! Measured demand on the OBO core (`probe_edge_capacity`): 186 029 triples
//! over 68 789 subjects, max out-degree **62**, max lane demand for a single
//! row **16**. No row overflows the 23-lane budget, and the packer never
//! truncates silently — an overflow is reported ([`PackStats::dropped`]) so a
//! driver can fail rather than quietly lose edges.
//!
//! # Emit order (the reader's contract)
//!
//! Targets are emitted ascending by `(predicate, target namespace, numeric)`,
//! and **a lane never mixes target ontologies** — its classid names exactly
//! one, so a namespace change starts a new lane even mid-predicate. Unused
//! slots in a lane are all-zero bytes, and a reader treats a zero slot as
//! "this lane is spent, advance".
//!
//! # The `+1` numeric bias (gate A1, applied 2026-08-08)
//!
//! An all-zero slot is the pad, so the slot encoding must have no *other*
//! meaning for zero. The original contract achieved that by assertion —
//! *"no OBO CURIE numeric is 0"* — which held for the five baked namespaces
//! and **is false in general**. `probe_spine` measured a real term at numeric
//! `0` in three further namespaces, one of them that namespace's own root. An
//! edge into such a term was indistinguishable from a pad, so a reader's walk
//! stopped early and *silently* — the worst failure shape available, because
//! the artifact still validates and the traversal just returns less.
//!
//! The fix removes the assumption instead of documenting around it: the
//! numeric is stored **biased by one** ([`encode_numeric`] / [`decode_numeric`]).
//! Stored `0` now means pad and nothing else, by construction rather than by
//! a property of the input.
//!
//! Cost: one value off the top of the range. [`MAX_LINK_NUMERIC`] is
//! `2²⁴ − 2 = 16_777_214`, against a measured maximum of `9_999_994` across
//! every namespace probed. A numeric above that is **reported as dropped**,
//! never truncated (see [`PackStats::dropped`]).
//!
//! This changed every byte of every edge lane, so it re-baked all five core
//! slabs and moved their digests. That was the price of the gate and it was
//! paid deliberately, not absorbed.
//!
//! A reader walks lanes in order and consumes `degree(p)` targets for each
//! predicate `p` in ascending order. That is [`EdgeLanes`], which borrows the
//! row and decodes a u24 with a shift — no allocation, no gathered window.

use crate::registry::NsRegistry;
use crate::{EDGES_OFFSET, NODE_ROW_STRIDE, Namespace, Predicate, TermId, VALUE_OFFSET};

/// Bytes per lane — the one form (`classid(4) + content(12)`).
pub const LANE_BYTES: usize = 16;
/// Links per lane under the G2 `4×u24` carving.
pub const LINKS_PER_LANE: usize = 4;
/// First edge lane's offset **within the 480-byte value slab** (lane 7).
/// Derived from the per-class LE contract ([`crate::layout::OBO_CORE_ROW`]),
/// never an independent literal.
pub const EDGE_LANE_SLAB_OFFSET: usize = crate::layout::OBO_CORE_ROW.edge_lanes.slab_off;
/// Number of lanes reserved for edge targets (lanes 7..30). Derived from the
/// per-class LE contract, never an independent literal.
pub const EDGE_LANE_COUNT: usize = crate::layout::OBO_CORE_ROW.lane_count;
/// Total link slots per row.
pub const EDGE_SLOT_COUNT: usize = EDGE_LANE_COUNT * LINKS_PER_LANE;

/// The largest CURIE numeric an edge lane can carry.
///
/// One below the u24 ceiling, because the stored form is biased by one so that
/// stored `0` can mean *pad* unconditionally (see the module docs, gate A1).
/// Measured maximum across every namespace probed: `9_999_994`.
pub const MAX_LINK_NUMERIC: u32 = 0x00FF_FFFF - 1;

/// CURIE numeric → its stored form. Total on `0..=MAX_LINK_NUMERIC`.
#[inline]
#[must_use]
pub const fn encode_numeric(num: u32) -> u32 {
    num + 1
}

/// Stored form → CURIE numeric. `stored == 0` is the pad and has no numeric,
/// so callers must test for it *before* decoding; [`EdgeLanes::links`] does.
#[inline]
#[must_use]
pub const fn decode_numeric(stored: u32) -> u32 {
    stored - 1
}

const _: () = assert!(EDGE_LANE_SLAB_OFFSET > crate::ENTITY_TYPE_SLAB_OFFSET);
const _: () = assert!(
    VALUE_OFFSET + EDGE_LANE_SLAB_OFFSET + EDGE_LANE_COUNT * LANE_BYTES == NODE_ROW_STRIDE,
    "edge lanes must run to the end of the row without overlapping a value tenant"
);

/// Read a little-endian `u24` (as a `u32`, high byte zero).
#[inline]
const fn u24_le(a: u8, b: u8, c: u8) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16)
}

/// Write a little-endian `u24`. Debug-asserts the 24-bit fit — no silent
/// truncation, the same guard the key's u24 groups use.
#[inline]
fn put_u24_le(dst: &mut [u8], v: u32) {
    debug_assert!(v <= 0x00FF_FFFF, "edge target numeric must fit 24 bits");
    dst[0] = (v & 0xFF) as u8;
    dst[1] = ((v >> 8) & 0xFF) as u8;
    dst[2] = ((v >> 16) & 0xFF) as u8;
}

/// One resolved link: the `(classid, identity)` pair, exactly as a key carries
/// it. `classid` names the target ontology; `num` is the CURIE numeric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Link {
    /// the target row's classid (`Namespace::render_classid`)
    pub classid: u32,
    /// the target CURIE numeric (24-bit)
    pub num: u32,
}

/// What a pack pass did — reported so a driver can refuse to ship a truncation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PackStats {
    /// links written into lanes
    pub written: usize,
    /// lanes consumed
    pub lanes: usize,
    /// links that did NOT fit the lane budget (must be 0 to ship)
    pub dropped: usize,
    /// rows that dropped at least one link
    pub rows_overflowed: usize,
}

/// Pack one node's targets into its row's edge lanes.
///
/// `targets` must already be the node's full out-edge set; this function sorts
/// it into the canonical `(predicate, namespace, numeric)` emit order itself,
/// so callers cannot get the reader's contract wrong by accident.
///
/// Returns the per-row stats. A link is counted in [`PackStats::dropped`] and
/// **not** written — never silently truncated — when either the lane budget is
/// exhausted or the numeric exceeds [`MAX_LINK_NUMERIC`]. Both mean the same
/// thing to a driver: the artifact is missing an edge, so it must not ship.
pub fn pack_edges(
    row: &mut [u8; NODE_ROW_STRIDE],
    targets: &mut [(Predicate, TermId)],
    app_prefix: u16,
    reg: &NsRegistry,
) -> PackStats {
    let mut st = PackStats::default();
    targets.sort_unstable_by_key(|(p, t)| (*p as u8, t.ns, t.num));

    let base = VALUE_OFFSET + EDGE_LANE_SLAB_OFFSET;
    let mut lane = 0usize;
    let mut slot = LINKS_PER_LANE; // force a fresh lane on the first target
    let mut group: Option<(u8, u8)> = None;

    for (i, (p, t)) in targets.iter().enumerate() {
        if t.num > MAX_LINK_NUMERIC {
            // Unrepresentable under the +1 bias — the one numeric the biased
            // u24 cannot hold. Reported, not truncated, and not allowed to
            // consume a slot: a driver refuses to ship on `dropped != 0`.
            st.dropped += 1;
            st.rows_overflowed = 1;
            continue;
        }
        let g = (*p as u8, t.ns);
        if group != Some(g) || slot == LINKS_PER_LANE {
            group = Some(g);
            lane += 1;
            slot = 0;
            if lane > EDGE_LANE_COUNT {
                // budget exhausted — count this one and every one after it,
                // write nothing more.
                st.dropped += targets.len() - i;
                st.rows_overflowed = 1;
                st.lanes = EDGE_LANE_COUNT;
                return st;
            }
            let off = base + (lane - 1) * LANE_BYTES;
            let Some(classid) = reg.render_classid(t.ns, app_prefix) else {
                // A target whose namespace is not in this registry has no
                // classid, so the lane could not name it. Report, never guess.
                st.dropped += 1;
                st.rows_overflowed = 1;
                lane -= 1;
                slot = LINKS_PER_LANE;
                group = None;
                continue;
            };
            row[off..off + 4].copy_from_slice(&classid.to_le_bytes());
        }
        let off = base + (lane - 1) * LANE_BYTES + 4 + slot * 3;
        put_u24_le(&mut row[off..off + 3], encode_numeric(t.num));
        slot += 1;
        st.written += 1;
    }
    st.lanes = lane;
    st
}

/// A borrowed view over one row's edge lanes — the traversal peek.
///
/// Holds no copy of anything: `degree` reads the edge block in place and
/// [`links`] decodes a `u24` with a shift as it walks. This is the `col_idx`
/// side of a CSR row; combined with [`EdgeLanes::degree`] (`row_ptr`) it is a
/// complete adjacency row, which is what makes a semiring traversal — rather
/// than pointer chasing — the natural way to consume it.
#[derive(Debug, Clone, Copy)]
pub struct EdgeLanes<'a> {
    row: &'a [u8; NODE_ROW_STRIDE],
}

impl<'a> EdgeLanes<'a> {
    /// Borrow a row's edge lanes.
    #[must_use]
    pub const fn new(row: &'a [u8; NODE_ROW_STRIDE]) -> Self {
        EdgeLanes { row }
    }

    /// This node's out-degree for one predicate — the `row_ptr` entry, read
    /// straight from the edge block with no value decode at all.
    #[must_use]
    pub const fn degree(&self, p: Predicate) -> u8 {
        self.row[EDGES_OFFSET + p as usize]
    }

    /// Total out-degree across every predicate.
    #[must_use]
    pub fn total_degree(&self) -> usize {
        (1..8).map(|i| self.row[EDGES_OFFSET + i] as usize).sum()
    }

    /// Every link in emit order, paired with its predicate.
    ///
    /// Walks lanes in order, consuming `degree(p)` links per predicate in
    /// ascending predicate order; a `0` slot means the lane is spent (no OBO
    /// CURIE numeric is 0) and the walk advances to the next lane.
    pub fn links(self) -> impl Iterator<Item = (Predicate, Link)> + 'a {
        LinkIter {
            row: self.row,
            lane: 0,
            slot: 0,
            pred: 0,
            remaining: 0,
        }
    }

    /// Only the links carried by one predicate — the traversal hot path
    /// (`is_a` for the subsumption spine, `part_of` for the anatomy backbone).
    pub fn links_of(self, p: Predicate) -> impl Iterator<Item = Link> + 'a {
        self.links().filter(move |(q, _)| *q == p).map(|(_, l)| l)
    }
}

struct LinkIter<'a> {
    row: &'a [u8; NODE_ROW_STRIDE],
    lane: usize,
    slot: usize,
    pred: u8,
    remaining: u8,
}

impl Iterator for LinkIter<'_> {
    type Item = (Predicate, Link);

    fn next(&mut self) -> Option<(Predicate, Link)> {
        let base = VALUE_OFFSET + EDGE_LANE_SLAB_OFFSET;
        loop {
            // advance to the next predicate that has any degree left
            while self.remaining == 0 {
                self.pred += 1;
                if self.pred > 7 {
                    return None;
                }
                self.remaining = self.row[EDGES_OFFSET + self.pred as usize];
            }
            if self.lane >= EDGE_LANE_COUNT {
                return None;
            }
            if self.slot >= LINKS_PER_LANE {
                self.lane += 1;
                self.slot = 0;
                continue;
            }
            let off = base + self.lane * LANE_BYTES;
            let n = u24_le(
                self.row[off + 4 + self.slot * 3],
                self.row[off + 5 + self.slot * 3],
                self.row[off + 6 + self.slot * 3],
            );
            if n == 0 {
                // Pad — the lane is spent, next lane. Unconditional under the
                // +1 bias: no real target encodes to 0, whatever its numeric.
                self.lane += 1;
                self.slot = 0;
                continue;
            }
            let classid = u32::from_le_bytes([
                self.row[off],
                self.row[off + 1],
                self.row[off + 2],
                self.row[off + 3],
            ]);
            self.slot += 1;
            self.remaining -= 1;
            let p = PREDICATES[self.pred as usize];
            return Some((
                p,
                Link {
                    classid,
                    num: decode_numeric(n),
                },
            ));
        }
    }
}

/// Predicate by discriminant; index 0 is reserved and never yielded.
const PREDICATES: [Predicate; 8] = [
    Predicate::IsA, // slot 0 reserved — never reached (degree byte 0 unused)
    Predicate::IsA,
    Predicate::PartOf,
    Predicate::HasPhenotype,
    Predicate::HasLocation,
    Predicate::HasAnatomy,
    Predicate::HasQuality,
    Predicate::Other,
];

/// The classid a link into `ns` carries, for callers building expectations.
#[must_use]
pub const fn target_classid(ns: Namespace, app_prefix: u16) -> u32 {
    ns.render_classid(app_prefix)
}

/// This row's own address as a [`Link`] — the same `(classid, identity)` shape
/// a target carries, because a subject and a target are the same kind of thing.
#[must_use]
pub const fn subject(row: &[u8; NODE_ROW_STRIDE]) -> Link {
    let (classid, num) = crate::unpack_key(row);
    Link { classid, num }
}

/// The **subsumption projection** over baked rows: `(subclass, superclass)`
/// pairs, `is_a` only.
///
/// This is the adapter a post-bake EL reasoner consumes. It reads the baked
/// artifact and nothing else — no file, no CURIE, no label — and the `is_a`
/// filter is the point rather than a convenience: walking subsumption together
/// with `part_of` derives FALSE ancestors (`A part_of B` and `B ⊑ C` does not
/// give `A ⊑ C`), so a reasoner without role composition must never be handed
/// the mixed set. Filtering here means a caller cannot make that mistake by
/// forgetting to.
///
/// Borrows throughout: the pairs are decoded from the rows as the iterator
/// walks, nothing is gathered.
pub fn subsumptions(rows: &[crate::Row512]) -> impl Iterator<Item = (Link, Link)> + '_ {
    rows.iter().flat_map(|row| {
        let s = subject(&row.0);
        EdgeLanes::new(&row.0)
            .links_of(Predicate::IsA)
            .map(move |p| (s, p))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(ns: Namespace, num: u32) -> TermId {
        TermId { ns: ns as u8, num }
    }

    fn row_with_degrees(degs: &[(Predicate, u8)]) -> [u8; NODE_ROW_STRIDE] {
        let mut row = [0u8; NODE_ROW_STRIDE];
        for (p, d) in degs {
            row[EDGES_OFFSET + *p as usize] = *d;
        }
        row
    }

    #[test]
    fn the_budget_covers_the_measured_worst_row() {
        // probe_edge_capacity on the real OBO core: max out-degree 62, max lane
        // demand 16. Both must fit, with the lane demand the binding one.
        const { assert!(16 <= EDGE_LANE_COUNT) };
        const { assert!(62 <= EDGE_SLOT_COUNT) };
    }

    #[test]
    fn a_single_link_round_trips_with_its_target_classid() {
        let mut row = row_with_degrees(&[(Predicate::IsA, 1)]);
        let mut tg = vec![(Predicate::IsA, t(Namespace::Mondo, 5015))];
        let st = pack_edges(&mut row, &mut tg, 0x0000, &crate::registry::OBO_CORE);
        assert_eq!(st.written, 1);
        assert_eq!(st.dropped, 0);

        let lanes = EdgeLanes::new(&row);
        assert_eq!(lanes.degree(Predicate::IsA), 1);
        let got: Vec<_> = lanes.links().collect();
        assert_eq!(
            got,
            vec![(
                Predicate::IsA,
                Link {
                    classid: target_classid(Namespace::Mondo, 0x0000),
                    num: 5015
                }
            )]
        );
    }

    #[test]
    fn a_namespace_change_starts_a_new_lane_so_a_classid_never_lies() {
        // one predicate, two target ontologies: the lane classid names exactly
        // one ontology, so the second target cannot share the first's lane.
        let mut row = row_with_degrees(&[(Predicate::Other, 2)]);
        let mut tg = vec![
            (Predicate::Other, t(Namespace::Mondo, 1)),
            (Predicate::Other, t(Namespace::Hpo, 2)),
        ];
        let st = pack_edges(&mut row, &mut tg, 0x0000, &crate::registry::OBO_CORE);
        assert_eq!(st.written, 2);
        assert_eq!(st.lanes, 2, "a namespace change must start a new lane");

        let got: Vec<_> = EdgeLanes::new(&row).links().map(|(_, l)| l).collect();
        assert_eq!(
            got,
            vec![
                Link {
                    classid: target_classid(Namespace::Mondo, 0x0000),
                    num: 1
                },
                Link {
                    classid: target_classid(Namespace::Hpo, 0x0000),
                    num: 2
                }
            ]
        );
    }

    #[test]
    fn a_full_lane_spills_into_the_next_one() {
        let mut row = row_with_degrees(&[(Predicate::IsA, 5)]);
        let mut tg: Vec<_> = (1..=5)
            .map(|n| (Predicate::IsA, t(Namespace::Uberon, n)))
            .collect();
        let st = pack_edges(&mut row, &mut tg, 0x0000, &crate::registry::OBO_CORE);
        assert_eq!(st.written, 5);
        assert_eq!(st.lanes, 2);
        let got: Vec<_> = EdgeLanes::new(&row)
            .links_of(Predicate::IsA)
            .map(|l| l.num)
            .collect();
        assert_eq!(got, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn predicates_are_recovered_from_the_degree_histogram() {
        // the lane carries no predicate byte — the reader recovers it from
        // row_ptr. Two predicates, one shared target ontology.
        let mut row = row_with_degrees(&[(Predicate::IsA, 2), (Predicate::PartOf, 1)]);
        let mut tg = vec![
            (Predicate::PartOf, t(Namespace::Uberon, 300)),
            (Predicate::IsA, t(Namespace::Uberon, 100)),
            (Predicate::IsA, t(Namespace::Uberon, 200)),
        ];
        pack_edges(&mut row, &mut tg, 0x0000, &crate::registry::OBO_CORE);
        let got: Vec<_> = EdgeLanes::new(&row)
            .links()
            .map(|(p, l)| (p, l.num))
            .collect();
        assert_eq!(
            got,
            vec![
                (Predicate::IsA, 100),
                (Predicate::IsA, 200),
                (Predicate::PartOf, 300),
            ]
        );
    }

    #[test]
    fn an_overflow_is_reported_and_never_silently_truncated() {
        let mut row = row_with_degrees(&[(Predicate::IsA, 255)]);
        // one target ontology, more links than the budget holds
        let over = EDGE_SLOT_COUNT + 7;
        let mut tg: Vec<_> = (1..=over as u32)
            .map(|n| (Predicate::IsA, t(Namespace::Hpo, n)))
            .collect();
        let st = pack_edges(&mut row, &mut tg, 0x0000, &crate::registry::OBO_CORE);
        assert_eq!(st.rows_overflowed, 1);
        assert_eq!(st.dropped, over - EDGE_SLOT_COUNT);
        assert_eq!(st.written, EDGE_SLOT_COUNT);
    }

    #[test]
    fn the_reader_can_fail() {
        // anti-vacuity: the walk is sensitive to the histogram, not decorative.
        // Same lane bytes, a degree of 1 instead of 3 → exactly one link read.
        let mut row = row_with_degrees(&[(Predicate::IsA, 3)]);
        let mut tg: Vec<_> = (1..=3)
            .map(|n| (Predicate::IsA, t(Namespace::Pato, n)))
            .collect();
        pack_edges(&mut row, &mut tg, 0x0000, &crate::registry::OBO_CORE);
        assert_eq!(EdgeLanes::new(&row).links().count(), 3);

        row[EDGES_OFFSET + Predicate::IsA as usize] = 1;
        assert_eq!(
            EdgeLanes::new(&row).links().count(),
            1,
            "row_ptr is load-bearing: a wrong degree must change the walk"
        );
    }

    #[test]
    fn the_subsumption_projection_drops_every_non_is_a_edge() {
        // A row with both an is_a and a part_of parent. Handing the mixed set
        // to a reasoner without role composition derives false ancestors, so
        // the projection must yield ONLY the is_a edge.
        let mut row = row_with_degrees(&[(Predicate::IsA, 1), (Predicate::PartOf, 1)]);
        row[0..16].copy_from_slice(&crate::pack_key(
            target_classid(Namespace::Uberon, 0x0000),
            2244,
        ));
        let mut tg = vec![
            (Predicate::IsA, t(Namespace::Uberon, 10)),
            (Predicate::PartOf, t(Namespace::Uberon, 20)),
        ];
        pack_edges(&mut row, &mut tg, 0x0000, &crate::registry::OBO_CORE);

        // both edges are readable...
        assert_eq!(EdgeLanes::new(&row).links().count(), 2);
        // ...but only the subsumption is projected, and the subject is the row.
        let rows = [crate::Row512(row)];
        let got: Vec<_> = subsumptions(&rows).collect();
        assert_eq!(
            got,
            vec![(
                Link {
                    classid: target_classid(Namespace::Uberon, 0x0000),
                    num: 2244
                },
                Link {
                    classid: target_classid(Namespace::Uberon, 0x0000),
                    num: 10
                }
            )],
            "part_of must not reach a reasoner that lacks role composition"
        );
    }

    /// Gate A1, stated as the bug it fixes.
    ///
    /// A link to numeric `0` — a real term in three measured namespaces, one
    /// of them a namespace root — must round-trip AND must not end the walk.
    /// Under the pre-bias encoding this test fails on both counts: the slot
    /// bytes are all-zero, so the reader reads a pad, drops the link, and
    /// **silently truncates the rest of the lane** — the second target is lost
    /// too. That is why the assertion below checks the *following* link, not
    /// just the zero one.
    #[test]
    fn a_link_to_numeric_zero_survives_and_does_not_end_the_walk() {
        let mut row = row_with_degrees(&[(Predicate::IsA, 2)]);
        let mut tg = vec![
            (Predicate::IsA, t(Namespace::Hpo, 0)),
            (Predicate::IsA, t(Namespace::Hpo, 118)),
        ];
        let st = pack_edges(&mut row, &mut tg, 0x0000, &crate::registry::OBO_CORE);
        assert_eq!(st.written, 2);
        assert_eq!(st.dropped, 0);

        let got: Vec<_> = EdgeLanes::new(&row)
            .links_of(Predicate::IsA)
            .map(|l| l.num)
            .collect();
        assert_eq!(
            got,
            vec![0, 118],
            "numeric 0 is a real target, and the link after it must survive too"
        );

        // The stored bytes must NOT be zero — that is the whole mechanism.
        let base = VALUE_OFFSET + EDGE_LANE_SLAB_OFFSET;
        assert_ne!(
            u24_le(row[base + 4], row[base + 5], row[base + 6]),
            0,
            "a real target must never encode to the pad value"
        );
    }

    /// The pad still works — the can-it-stay-silent half. An unused slot must
    /// still terminate the lane, or the bias would have traded one silent
    /// failure for another.
    #[test]
    fn an_unused_slot_still_reads_as_pad_and_advances_the_lane() {
        // Two target ontologies force two lanes; the first lane keeps three
        // empty slots after its single link.
        let mut row = row_with_degrees(&[(Predicate::IsA, 2)]);
        let mut tg = vec![
            (Predicate::IsA, t(Namespace::Mondo, 5015)),
            (Predicate::IsA, t(Namespace::Uberon, 955)),
        ];
        pack_edges(&mut row, &mut tg, 0x0000, &crate::registry::OBO_CORE);
        let got: Vec<_> = EdgeLanes::new(&row)
            .links()
            .map(|(_, l)| (l.classid, l.num))
            .collect();
        assert_eq!(
            got,
            vec![
                (target_classid(Namespace::Mondo, 0x0000), 5015),
                (target_classid(Namespace::Uberon, 0x0000), 955),
            ],
            "the three empty slots after the first link must advance the lane, \
             not yield phantom links"
        );
    }

    #[test]
    fn the_bias_round_trips_across_the_whole_representable_range() {
        for n in [
            0u32,
            1,
            2,
            118,
            65_535,
            65_536,
            700_092,
            9_999_994,
            MAX_LINK_NUMERIC,
        ] {
            assert_eq!(decode_numeric(encode_numeric(n)), n, "{n} must round-trip");
            assert_ne!(encode_numeric(n), 0, "{n} must not collide with the pad");
            assert!(
                encode_numeric(n) <= 0x00FF_FFFF,
                "{n} must still fit the u24 slot after biasing"
            );
        }
    }

    /// The one numeric the biased u24 cannot hold is reported, not truncated.
    ///
    /// `TermId::parse` already rejects `> 0x00FF_FFFF`, so this is reachable
    /// only by constructing a `TermId` directly — but a guard that cannot be
    /// shown to fire is the defect one level up.
    #[test]
    fn an_unrepresentable_numeric_is_dropped_rather_than_wrapped() {
        let mut row = row_with_degrees(&[(Predicate::IsA, 2)]);
        let mut tg = vec![
            (
                Predicate::IsA,
                TermId {
                    ns: Namespace::Hpo as u8,
                    num: 0x00FF_FFFF,
                },
            ),
            (Predicate::IsA, t(Namespace::Hpo, 7)),
        ];
        let st = pack_edges(&mut row, &mut tg, 0x0000, &crate::registry::OBO_CORE);
        assert_eq!(st.dropped, 1, "the unrepresentable link must be reported");
        assert_eq!(st.rows_overflowed, 1, "and must block the ship");
        assert_eq!(st.written, 1, "the representable link still lands");
        // It must not have wrapped to the pad, nor to some other term.
        let got: Vec<_> = EdgeLanes::new(&row)
            .links_of(Predicate::IsA)
            .map(|l| l.num)
            .collect();
        assert_eq!(got, vec![7], "no phantom target from a wrapped numeric");
    }

    #[test]
    fn a_row_key_round_trips_through_the_family_identity_rail() {
        // an oversize CURIE (> u16) must survive: the numeric rides the rail,
        // high half in `family`. A truncating decode would return 4556 here.
        let mut row = [0u8; NODE_ROW_STRIDE];
        let mondo = target_classid(Namespace::Mondo, 0x0000);
        row[0..16].copy_from_slice(&crate::pack_key(mondo, 700_092));
        assert_eq!(
            subject(&row),
            Link {
                classid: mondo,
                num: 700_092
            }
        );
    }

    #[test]
    fn an_edge_lane_never_overwrites_the_entity_type_tenant() {
        let mut row = [0u8; NODE_ROW_STRIDE];
        let et = VALUE_OFFSET + crate::ENTITY_TYPE_SLAB_OFFSET;
        row[et..et + 2].copy_from_slice(&7u16.to_le_bytes());
        row[EDGES_OFFSET + Predicate::IsA as usize] = EDGE_SLOT_COUNT as u8;
        let mut tg: Vec<_> = (1..=EDGE_SLOT_COUNT as u32)
            .map(|n| (Predicate::IsA, t(Namespace::Ro, n)))
            .collect();
        let st = pack_edges(&mut row, &mut tg, 0x0000, &crate::registry::OBO_CORE);
        assert_eq!(st.dropped, 0);
        assert_eq!(
            u16::from_le_bytes([row[et], row[et + 1]]),
            7,
            "a full edge pack must leave the EntityType tenant intact"
        );
    }
}
