//! `SpineLens` — a borrowed view that answers "who are this class's parents?"
//! straight out of baked rows, with no adjacency index built.
//!
//! # Why this exists rather than an adjacency copy
//!
//! The obvious way to traverse a baked graph is to read every edge once into an
//! adjacency structure and walk that. Measured, for this substrate, that is the
//! expensive option: `lance-graph`'s `GrBMatrix` carries a `Vec<BitVec>` of
//! 16 384-bit values — ~2 KiB **per edge** — so materialising the 107 813 `is_a`
//! edges as a matrix costs on the order of 200 MB to hold an adjacency the rows
//! already contain. That is the opposite of the deployment's whole shape (a
//! large hydrated table, a small resident set), and it is also what the
//! workspace's zero-copy law forbids on principle rather than on cost.
//!
//! So the lens does not build anything. It borrows the rows and answers each
//! question by locating a row and decoding its lanes in place.
//!
//! # The two steps, and what each costs
//!
//! 1. **Resolve** — `(classid, num)` → row index, by binary search. Sound
//!    because the bake emits rows in ascending `(classid, num)`: `family` holds
//!    the high half of the numeric, so ordering by `(classid, family, identity)`
//!    *is* ordering by `(classid, num)`. Verified over the shipped bake — all
//!    68 797 rows strictly ascending, zero inversions. ~17 probes at that size.
//! 2. **Walk** — decode that row's `is_a` links from its edge lanes
//!    ([`crate::edges::EdgeLanes`]), a shift per link, nothing allocated.
//!
//! Against an indexed closure the trade is explicit: a `HashMap` gives O(1)
//! hops after paying a full pass over every edge and holding a map proportional
//! to them; the lens pays O(log n) per hop and holds nothing. Which wins depends
//! on how many questions are asked against how much memory is available, and
//! `examples/elk_lens_vs_closure.rs` measures it rather than asserting it.
//!
//! # It borrows, and it does not cache
//!
//! One field, a shared slice, in the house style of the workspace's other
//! lenses. No memo table: a transitive closure recomputes equal, which under
//! the zero-copy law makes it a projection, and a projection is never stored
//! however much work produced it. Asking twice walks twice, deliberately.

use crate::{NODE_ROW_STRIDE, Predicate, Row512, edges::EdgeLanes, unpack_key};

/// A borrowed view over baked rows that resolves addresses and reads parents.
///
/// Holds a shared slice and nothing else. Constructing one is free; it is meant
/// to be made where it is used rather than kept.
#[derive(Clone, Copy)]
pub struct SpineLens<'a> {
    rows: &'a [Row512],
}

impl core::fmt::Debug for SpineLens<'_> {
    /// Prints the row COUNT, never the rows. A lens over a hydrated table can
    /// borrow tens of megabytes; a derived `Debug` would render all of it.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SpineLens")
            .field("rows", &self.rows.len())
            .finish()
    }
}

impl<'a> SpineLens<'a> {
    /// Borrow a slice of baked rows.
    ///
    /// The rows **must** be in the bake's own order — ascending `(classid,
    /// num)` — because [`Self::resolve`] binary-searches them. Handing over an
    /// unsorted slice does not panic; it silently fails to find rows, which is
    /// why [`Self::is_sorted`] exists and why the example asserts it before use.
    #[must_use]
    pub const fn new(rows: &'a [Row512]) -> Self {
        Self { rows }
    }

    /// Number of rows in view.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the view is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The `(classid, num)` address of row `i`, or `None` if out of range.
    #[must_use]
    pub fn address_at(&self, i: usize) -> Option<(u32, u32)> {
        self.rows.get(i).map(|r| unpack_key(&r.0))
    }

    /// Check the precondition [`Self::resolve`] relies on: strictly ascending
    /// `(classid, num)`.
    ///
    /// O(n) and therefore not called per query — it is for a caller to assert
    /// once at the boundary where rows arrive, so an unsorted slice fails
    /// loudly there instead of returning empty answers everywhere else.
    #[must_use]
    pub fn is_sorted(&self) -> bool {
        self.rows
            .windows(2)
            .all(|w| unpack_key(&w[0].0) < unpack_key(&w[1].0))
    }

    /// Locate the row for an address by binary search.
    ///
    /// `None` means no such row is in view — which is a real and expected
    /// answer, not an error: an edge may point at a term outside the loaded
    /// set, and the walk simply stops there rather than inventing a node.
    #[must_use]
    pub fn resolve(&self, classid: u32, num: u32) -> Option<usize> {
        self.rows
            .binary_search_by(|r| unpack_key(&r.0).cmp(&(classid, num)))
            .ok()
    }

    /// This row's out-degree for one predicate — the CSR `row_ptr` entry, read
    /// from the edge block with **no value decode at all**.
    #[must_use]
    pub fn degree_at(&self, i: usize, p: Predicate) -> u8 {
        self.rows
            .get(i)
            .map_or(0, |r| EdgeLanes::new(&r.0).degree(p))
    }

    /// Visit every link of one predicate for the row at `i`, decoded in place.
    pub fn links_at(&self, i: usize, p: Predicate, visit: &mut dyn FnMut(u32, u32)) {
        let Some(row) = self.rows.get(i) else { return };
        for l in EdgeLanes::new(&row.0).links_of(p) {
            visit(l.classid, l.num);
        }
    }

    /// Visit every **typed relation** (everything except `is_a`) of the row at
    /// `i`, decoded in place — one lane sweep, not one sweep per predicate.
    ///
    /// The typed twin of [`Self::links_at`]: `is_a` is excluded because the
    /// subsumption spine has its own reader ([`Self::parents_of`]) with its own
    /// safety note, and a caller asking for "the relations" almost never wants
    /// subsumption mixed in silently.
    pub fn rel_at(&self, i: usize, visit: &mut dyn FnMut(Predicate, u32, u32)) {
        let Some(row) = self.rows.get(i) else { return };
        for (p, l) in EdgeLanes::new(&row.0).links() {
            if p != Predicate::IsA {
                visit(p, l.classid, l.num);
            }
        }
    }

    /// Visit the **asserted superclasses** of an address: resolve, then walk
    /// that row's `is_a` links.
    ///
    /// `is_a` only, and that is the safety rather than a convenience — walking
    /// subsumption together with `part_of` derives false ancestors (`A part_of
    /// B` and `B ⊑ C` does not give `A ⊑ C`), and a reasoner without role
    /// composition must never be handed the mixed set.
    ///
    /// An address with no row in view yields nothing, silently: an edge into an
    /// unloaded ontology is a boundary, not a fault.
    pub fn parents_of(&self, classid: u32, num: u32, visit: &mut dyn FnMut(u32, u32)) {
        if let Some(i) = self.resolve(classid, num) {
            self.links_at(i, Predicate::IsA, visit);
        }
    }

    /// The **grounding spine** of an address: `is_a` ∪ `part_of` parents.
    ///
    /// This is the spine an existential FILLER generalizes up (R∃), and it is
    /// deliberately NOT [`Self::parents_of`]. An anatomical filler's grounding
    /// is inherited through mereology as well as taxonomy — the deductive form
    /// of "inherit grounding up the anatomy spine".
    ///
    /// **Never hand this to a subsumption reasoner.** Generalizing a *subject*
    /// through `part_of` is the false-ancestor error (`A part_of B`, `B ⊑ C`
    /// does not give `A ⊑ C`); generalizing a *filler* through it is sound.
    /// [`Self::parents_of`] stays `is_a`-only for exactly that reason, and the
    /// two are separate methods so a caller must choose on purpose.
    pub fn grounding_parents_of(&self, classid: u32, num: u32, visit: &mut dyn FnMut(u32, u32)) {
        if let Some(i) = self.resolve(classid, num) {
            self.links_at(i, Predicate::IsA, visit);
            self.links_at(i, Predicate::PartOf, visit);
        }
    }

    /// The asserted existential fillers of one role on an address.
    ///
    /// A thin typed read — the R∃ generalization is the reasoner's job, not
    /// this one's. An address with no row in view yields nothing, silently.
    pub fn fillers_of(
        &self,
        classid: u32,
        num: u32,
        role: Predicate,
        visit: &mut dyn FnMut(u32, u32),
    ) {
        if let Some(i) = self.resolve(classid, num) {
            self.links_at(i, role, visit);
        }
    }
}

/// Reinterpret a byte buffer as rows, if it is correctly sized and aligned.
///
/// Returns `None` when the length is not a whole number of rows or the buffer
/// is not 64-aligned — the same gate `rows_from_le_bytes` applies, restated
/// here so a lens caller has one call rather than two.
#[must_use]
pub fn lens_from_le_bytes(bytes: &[u8]) -> Option<SpineLens<'_>> {
    if !bytes.len().is_multiple_of(NODE_ROW_STRIDE) {
        return None;
    }
    crate::rows_from_le_bytes(bytes).map(SpineLens::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Namespace, TermId, edges::pack_edges};

    const MONDO: u32 = 0x0301_0000;

    fn row(num: u32, parents: &[u32]) -> Row512 {
        let mut r = Row512::zeroed();
        r.0[0..16].copy_from_slice(&crate::pack_key(MONDO, num));
        r.0[crate::EDGES_OFFSET + Predicate::IsA as usize] = parents.len() as u8;
        let mut tg: Vec<_> = parents
            .iter()
            .map(|p| {
                (
                    Predicate::IsA,
                    TermId {
                        ns: Namespace::Mondo as u8,
                        num: *p,
                    },
                )
            })
            .collect();
        pack_edges(&mut r.0, &mut tg, 0x0000, &crate::registry::OBO_CORE);
        r
    }

    /// 1 -> 2 -> 3, plus an isolated 9.
    fn fixture() -> Vec<Row512> {
        vec![row(1, &[2]), row(2, &[3]), row(3, &[]), row(9, &[])]
    }

    #[test]
    fn the_grounding_spine_includes_part_of_and_the_subsumption_one_does_not() {
        // one row with BOTH an is_a and a part_of parent
        let mut r = Row512::zeroed();
        r.0[0..16].copy_from_slice(&crate::pack_key(MONDO, 1));
        r.0[crate::EDGES_OFFSET + Predicate::IsA as usize] = 1;
        r.0[crate::EDGES_OFFSET + Predicate::PartOf as usize] = 1;
        let mut tg = vec![
            (
                Predicate::IsA,
                TermId {
                    ns: Namespace::Mondo as u8,
                    num: 2,
                },
            ),
            (
                Predicate::PartOf,
                TermId {
                    ns: Namespace::Mondo as u8,
                    num: 3,
                },
            ),
        ];
        pack_edges(&mut r.0, &mut tg, 0x0000, &crate::registry::OBO_CORE);
        let rows = vec![r];
        let lens = SpineLens::new(&rows);

        let mut subsumption = Vec::new();
        lens.parents_of(MONDO, 1, &mut |_, n| subsumption.push(n));
        assert_eq!(
            subsumption,
            vec![2],
            "part_of must NOT reach a subsumption walk"
        );

        let mut grounding = Vec::new();
        lens.grounding_parents_of(MONDO, 1, &mut |_, n| grounding.push(n));
        assert_eq!(
            grounding,
            vec![2, 3],
            "the grounding spine is is_a UNION part_of"
        );
    }

    #[test]
    fn fillers_are_read_per_role_and_can_be_empty() {
        let mut r = Row512::zeroed();
        r.0[0..16].copy_from_slice(&crate::pack_key(MONDO, 1));
        r.0[crate::EDGES_OFFSET + Predicate::HasPhenotype as usize] = 1;
        let mut tg = vec![(
            Predicate::HasPhenotype,
            TermId {
                ns: Namespace::Hpo as u8,
                num: 42,
            },
        )];
        pack_edges(&mut r.0, &mut tg, 0x0000, &crate::registry::OBO_CORE);
        let rows = vec![r];
        let lens = SpineLens::new(&rows);

        let mut got = Vec::new();
        lens.fillers_of(MONDO, 1, Predicate::HasPhenotype, &mut |c, n| {
            got.push((c, n))
        });
        assert_eq!(got, vec![(0x0302_0000, 42)], "crosses into HPO");

        // can-stay-silent: a role with no asserted filler yields nothing
        got.clear();
        lens.fillers_of(MONDO, 1, Predicate::HasAnatomy, &mut |c, n| {
            got.push((c, n))
        });
        assert!(
            got.is_empty(),
            "an unasserted role must not manufacture a filler"
        );
    }

    #[test]
    fn resolve_finds_every_row_and_misses_absent_ones() {
        let rows = fixture();
        let lens = SpineLens::new(&rows);
        assert!(lens.is_sorted());
        for (i, n) in [1u32, 2, 3, 9].iter().enumerate() {
            assert_eq!(lens.resolve(MONDO, *n), Some(i));
        }
        // anti-vacuity: resolve must be able to say no
        assert_eq!(lens.resolve(MONDO, 4), None, "absent numeric");
        assert_eq!(
            lens.resolve(0x0302_0000, 1),
            None,
            "right numeric, wrong class"
        );
    }

    #[test]
    fn parents_come_from_the_lanes_and_stop_at_the_boundary() {
        let rows = fixture();
        let lens = SpineLens::new(&rows);
        let mut got = Vec::new();
        lens.parents_of(MONDO, 1, &mut |c, n| got.push((c, n)));
        assert_eq!(got, vec![(MONDO, 2)]);

        got.clear();
        lens.parents_of(MONDO, 3, &mut |c, n| got.push((c, n)));
        assert!(got.is_empty(), "a root has no parents");

        got.clear();
        lens.parents_of(MONDO, 404, &mut |c, n| got.push((c, n)));
        assert!(
            got.is_empty(),
            "an unresolvable address yields nothing, silently"
        );
    }

    #[test]
    fn degree_reads_without_decoding_any_value() {
        let rows = fixture();
        let lens = SpineLens::new(&rows);
        assert_eq!(lens.degree_at(0, Predicate::IsA), 1);
        assert_eq!(lens.degree_at(2, Predicate::IsA), 0);
        assert_eq!(lens.degree_at(0, Predicate::PartOf), 0);
    }

    #[test]
    fn is_sorted_actually_rejects_an_unsorted_slice() {
        // the guard must be able to fail, or it is decoration
        let rows = vec![row(3, &[]), row(1, &[2])];
        assert!(!SpineLens::new(&rows).is_sorted());
    }

    /// The lens drives the reasoner with no adapter type in between — the
    /// property that keeps ogar-obo and ogar-elk free of a dependency edge.
    #[test]
    fn the_lens_composes_with_a_visitor_closure() {
        let rows = fixture();
        let lens = SpineLens::new(&rows);
        // the shape `ogar_elk::Parents` is blanket-implemented for
        let f = |addr: (u32, u32), visit: &mut dyn FnMut((u32, u32))| {
            lens.parents_of(addr.0, addr.1, &mut |c, n| visit((c, n)));
        };
        let mut seen = Vec::new();
        f((MONDO, 1), &mut |a| seen.push(a));
        assert_eq!(seen, vec![(MONDO, 2)]);
    }
}
