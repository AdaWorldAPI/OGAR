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
//! slots in a lane are `0`, which is unambiguous: no OBO CURIE numeric is 0
//! (asserted by `probe_edge_capacity` A1 on the real sources).
//!
//! A reader walks lanes in order and consumes `degree(p)` targets for each
//! predicate `p` in ascending order, treating a `0` slot as "this lane is
//! spent, advance". That is [`EdgeLanes`], which borrows the row and decodes a
//! u24 with a shift — no allocation, no gathered window.

use crate::{EDGES_OFFSET, NODE_ROW_STRIDE, Namespace, Predicate, TermId, VALUE_OFFSET};

/// Bytes per lane — the one form (`classid(4) + content(12)`).
pub const LANE_BYTES: usize = 16;
/// Links per lane under the G2 `4×u24` carving.
pub const LINKS_PER_LANE: usize = 4;
/// First edge lane's offset **within the 480-byte value slab** (lane 7).
pub const EDGE_LANE_SLAB_OFFSET: usize = 112;
/// Number of lanes reserved for edge targets (lanes 7..30).
pub const EDGE_LANE_COUNT: usize = 23;
/// Total link slots per row.
pub const EDGE_SLOT_COUNT: usize = EDGE_LANE_COUNT * LINKS_PER_LANE;

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
/// Returns the per-row stats. Links beyond [`EDGE_SLOT_COUNT`] are counted in
/// [`PackStats::dropped`] and **not** written — never silently truncated.
pub fn pack_edges(
    row: &mut [u8; NODE_ROW_STRIDE],
    targets: &mut [(Predicate, TermId)],
    app_prefix: u16,
) -> PackStats {
    let mut st = PackStats::default();
    targets.sort_unstable_by_key(|(p, t)| (*p as u8, t.ns, t.num));

    let base = VALUE_OFFSET + EDGE_LANE_SLAB_OFFSET;
    let mut lane = 0usize;
    let mut slot = LINKS_PER_LANE; // force a fresh lane on the first target
    let mut group: Option<(u8, u8)> = None;

    for (p, t) in targets.iter() {
        let g = (*p as u8, t.ns);
        if group != Some(g) || slot == LINKS_PER_LANE {
            group = Some(g);
            lane += 1;
            slot = 0;
            if lane > EDGE_LANE_COUNT {
                // budget exhausted — count the remainder, write nothing more.
                st.dropped = targets.len() - st.written;
                st.rows_overflowed = 1;
                st.lanes = EDGE_LANE_COUNT;
                return st;
            }
            let off = base + (lane - 1) * LANE_BYTES;
            let classid = t.namespace().render_classid(app_prefix);
            row[off..off + 4].copy_from_slice(&classid.to_le_bytes());
        }
        let off = base + (lane - 1) * LANE_BYTES + 4 + slot * 3;
        put_u24_le(&mut row[off..off + 3], t.num);
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
    pub fn links(&self) -> impl Iterator<Item = (Predicate, Link)> + '_ {
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
    pub fn links_of(&self, p: Predicate) -> impl Iterator<Item = Link> + '_ {
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
                // pad — the lane is spent, next lane
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
            return Some((p, Link { classid, num: n }));
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
        let st = pack_edges(&mut row, &mut tg, 0x0000);
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
                    classid: 0x0301_0000,
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
        let st = pack_edges(&mut row, &mut tg, 0x0000);
        assert_eq!(st.written, 2);
        assert_eq!(st.lanes, 2, "a namespace change must start a new lane");

        let got: Vec<_> = EdgeLanes::new(&row).links().map(|(_, l)| l).collect();
        assert_eq!(
            got,
            vec![
                Link {
                    classid: 0x0301_0000,
                    num: 1
                },
                Link {
                    classid: 0x0302_0000,
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
        let st = pack_edges(&mut row, &mut tg, 0x0000);
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
        pack_edges(&mut row, &mut tg, 0x0000);
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
        let st = pack_edges(&mut row, &mut tg, 0x0000);
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
        pack_edges(&mut row, &mut tg, 0x0000);
        assert_eq!(EdgeLanes::new(&row).links().count(), 3);

        row[EDGES_OFFSET + Predicate::IsA as usize] = 1;
        assert_eq!(
            EdgeLanes::new(&row).links().count(),
            1,
            "row_ptr is load-bearing: a wrong degree must change the walk"
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
        let st = pack_edges(&mut row, &mut tg, 0x0000);
        assert_eq!(st.dropped, 0);
        assert_eq!(
            u16::from_le_bytes([row[et], row[et + 1]]),
            7,
            "a full edge pack must leave the EntityType tenant intact"
        );
    }
}
