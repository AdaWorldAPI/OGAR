//! `ogar-elk` — the EL subsumption closure as an **observation over the bake**.
//!
//! # What this is, and the two questions it answers
//!
//! Third member of the factfinder family beside [`ogar-obo`] (harvest) and
//! [`ogar-ro`] (the predicate palette). Those two say what is *asserted*; this
//! one says what *follows*. It answers exactly two questions:
//!
//! 1. **Does `A ⊑ B` follow** from the baked edges? ([`Spine::entails`])
//! 2. **Is adding a set of axioms sound** — does the merged closure introduce
//!    an equivalence cycle that was not already there? ([`Spine::merge`])
//!
//! Nothing here is graded, ranked, scored or weighted. An EL entailment is a
//! **fact**: it follows necessarily from the asserted set, or it does not. The
//! thinking that consumes these facts lives elsewhere; this crate is unfazed by
//! it.
//!
//! [`ogar-obo`]: https://docs.rs/ogar-obo
//! [`ogar-ro`]: https://docs.rs/ogar-ro
//!
//! # Where this runs: AFTER the bake, over what the bake already holds
//!
//! **The joins are pre-bake.** Reconciling which source says what, resolving a
//! CURIE, deciding that two labels name one concept — all of that finishes
//! upstream, and the bake freezes the result into positions. This crate runs on
//! the far side of that line: it is the **observation** of what the baked spine
//! entails, not a stage that produces one.
//!
//! Which is why it **borrows and never owns**. A [`Spine`] is a lens over a
//! sorted slice the bake already emitted — `ogar_obo::Bake::triples` projected
//! to its `is_a` edges. Parents are found by [`slice::binary_search`] over that
//! slice and returned **as a subslice of it**; the crate allocates nothing that
//! mirrors the substrate. Building a `HashMap<_, Vec<_>>` here would be a
//! second copy of an adjacency the bake already laid out in order, and a second
//! copy is a second thing that can be wrong.
//!
//! The one allocation a walk does make is its **frontier** — the set of nodes
//! already visited. That is the algorithm's working set, bounded by the size of
//! the answer, and it is not a duplicate of the data.
//!
//! # The one precondition
//!
//! Edges must be **sorted by subclass**. The bake emits them that way (its ids
//! are sorted before rows are packed), so [`Spine::over`] takes that on trust
//! and checks it in debug builds. A caller holding a slice of uncertain
//! provenance uses [`Spine::try_over`], which verifies in release too — an
//! unsorted slice makes the binary search silently miss parents, which would
//! read as "not entailed" and is the one failure this crate must not have.
//!
//! # The fragment, stated precisely
//!
//! Three completion rules, which is the entire calculus for a pure subsumption
//! spine:
//!
//! ```text
//! R1  reflexivity     A ⊑ A
//! R2  transitivity    A ⊑ B , B ⊑ C  ⟹  A ⊑ C
//! R3  merge-soundness closing C ∪ S introduces no A ≡ B (A ≠ B) absent from C
//! ```
//!
//! R3 is the validation rule and the reason this crate exists rather than a
//! transitive-closure helper. Two independently authored sources can each be
//! internally consistent and still disagree about the DIRECTION of a relation;
//! merging them then derives `A ⊑ B` and `B ⊑ A`, i.e. `A ≡ B`, for classes
//! neither source calls equivalent. That cycle is the disagreement, made
//! mechanical — and it is found at **any distance**, including cycles that close
//! through a chain no pairwise comparison would think to check.
//!
//! Note what R3 is NOT a duplicate of: `ogar_obo::BakeStats::is_a_cycles`
//! reports cycles **inside one** baked core. R3 asks what a *further* set of
//! axioms would do to a spine already baked — the question you can only ask
//! once you have the bake to ask it about.
//!
//! # What is deliberately NOT here
//!
//! - **Existential restrictions** (`∃r.C`)
//! - **Role composition** (`part_of ∘ part_of ⊑ part_of`)
//! - **Bottom propagation** (unsatisfiability)
//! - **Conjunction, disjunction, complement, self-restriction**
//!
//! Each is a real part of EL++ and each is absent on purpose. They become
//! necessary the moment typed cross-angle edges (an `ogar-ro` predicate other
//! than subsumption) enter the closure — and at that point the correct move is
//! to wrap a full reasoner, not to grow this file. The boundary is named here so
//! a future session sees it as a decision rather than an oversight.
//!
//! **The concrete hazard that boundary guards:** without role composition,
//! walking subsumption and part-of edges together derives FALSE ancestors.
//! `A part_of B` and `B ⊑ C` does not give `A ⊑ C`. This crate cannot make that
//! mistake because it only ever reads subsumptions — but a caller that projects
//! a mixed edge set can, so [`Spine::over`] takes [`Subsumption`] by type rather
//! than raw pairs: the projection from the baked triple table has to name the
//! predicate it kept.

#![forbid(unsafe_code)]

use std::collections::{HashSet, VecDeque};

/// The address a baked row carries: `(classid, identity)`.
///
/// Read off the row's key by position — `classid` at `[0,4)`, the identity rail
/// in the V3 tail — never parsed and never resolved to a label. Ordered so a
/// slice of edges can be sorted and binary-searched; that ordering is the whole
/// access method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClassAddr {
    /// The row's classid.
    pub classid: u32,
    /// The row's identity within that class.
    pub identity: u32,
}

impl ClassAddr {
    /// Construct an address.
    #[must_use]
    pub const fn new(classid: u32, identity: u32) -> Self {
        Self { classid, identity }
    }
}

/// A baked subsumption `sub ⊑ sup`.
///
/// A distinct type rather than a `(ClassAddr, ClassAddr)` tuple, so a caller
/// projecting the baked triple table cannot hand this crate a `part_of` edge by
/// accident — see the crate doc's false-ancestor hazard. `Ord` sorts by `sub`
/// first, which is the order [`Spine`] binary-searches in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Subsumption {
    /// The subclass.
    pub sub: ClassAddr,
    /// The superclass.
    pub sup: ClassAddr,
}

impl Subsumption {
    /// Construct a subsumption.
    #[must_use]
    pub const fn new(sub: ClassAddr, sup: ClassAddr) -> Self {
        Self { sub, sup }
    }
}

/// How deep a transitivity walk may go before it stops.
///
/// A guard, not a tuning knob. A well-formed subsumption spine is acyclic and
/// shallow, but this crate runs on real baked data where a cycle would otherwise
/// spin forever — and [`Spine::equivalence_cycles`] exists precisely because
/// cycles DO occur. The cap is reported by [`Spine::depth_cap`] so a caller can
/// tell "not entailed" from "we stopped looking", which are different answers
/// and must not be conflated.
pub const DEFAULT_DEPTH_CAP: usize = 64;

/// A **lens** over baked subsumption edges — borrowed, never owned.
///
/// Holds two sorted slices: the `base` spine (what the bake emitted) and an
/// optional `overlay` (candidate axioms, for [`Self::merge`]). Both are borrowed
/// from the caller; adding an overlay allocates nothing, which is what makes
/// "what WOULD this merge do" cheap enough to ask before deciding.
///
/// The closure is walked per query rather than materialised: a spine of N
/// classes has an O(N²) transitive closure but only O(N) baked edges, and almost
/// every caller asks about a handful of classes. Materialising would trade a
/// cheap walk over data that already exists for an expensive table nobody reads
/// in full.
#[derive(Debug, Clone, Copy)]
pub struct Spine<'a> {
    base: &'a [Subsumption],
    overlay: &'a [Subsumption],
    depth_cap: usize,
}

impl<'a> Spine<'a> {
    /// Lens over a slice of baked edges, **sorted by `sub`**.
    ///
    /// The bake emits them sorted, so this takes it on trust and only checks in
    /// debug builds. Use [`Self::try_over`] for a slice whose provenance you do
    /// not control.
    #[must_use]
    pub fn over(edges: &'a [Subsumption]) -> Self {
        debug_assert!(is_sorted(edges), "Spine::over: edges must be sorted by sub");
        Self {
            base: edges,
            overlay: &[],
            depth_cap: DEFAULT_DEPTH_CAP,
        }
    }

    /// Lens over a slice whose sortedness is **verified**, in release too.
    ///
    /// `None` means the slice is unsorted, which would make the binary search
    /// miss parents and report "not entailed" for edges that are right there.
    /// Returning `None` rather than sorting a copy is deliberate: sorting would
    /// mean owning, and the caller who holds the real order should fix it.
    #[must_use]
    pub fn try_over(edges: &'a [Subsumption]) -> Option<Self> {
        is_sorted(edges).then_some(Self {
            base: edges,
            overlay: &[],
            depth_cap: DEFAULT_DEPTH_CAP,
        })
    }

    /// A lens over `base ∪ overlay`, borrowing both. The overlay must be sorted
    /// by `sub` on the same terms as the base.
    #[must_use]
    pub fn with_overlay(self, overlay: &'a [Subsumption]) -> Self {
        debug_assert!(
            is_sorted(overlay),
            "Spine::with_overlay: overlay must be sorted by sub"
        );
        Self { overlay, ..self }
    }

    /// Override the transitivity depth guard.
    #[must_use]
    pub const fn with_depth_cap(mut self, cap: usize) -> Self {
        self.depth_cap = cap;
        self
    }

    /// The depth guard in force — so a caller can distinguish "not entailed"
    /// from "the walk stopped".
    #[must_use]
    pub const fn depth_cap(&self) -> usize {
        self.depth_cap
    }

    /// Number of edges in view (base + overlay).
    #[must_use]
    pub const fn len(&self) -> usize {
        self.base.len() + self.overlay.len()
    }

    /// Whether any edge is in view.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.base.is_empty() && self.overlay.is_empty()
    }

    /// The asserted parents of `c`, **as two subslices of the borrowed data**.
    ///
    /// This is the lookup the whole crate is built on: an equal-range binary
    /// search by `sub`, returning a window into the caller's slice. Nothing is
    /// copied, so a caller can hold the result as long as it holds the edges.
    #[must_use]
    pub fn parents_of(&self, c: ClassAddr) -> (&'a [Subsumption], &'a [Subsumption]) {
        (equal_range(self.base, c), equal_range(self.overlay, c))
    }

    /// **R1 + R2** — every superclass of `c`, transitively.
    ///
    /// Excludes `c` itself unless a cycle genuinely returns to it, which is the
    /// signal [`Self::equivalence_cycles`] reads. Breadth-first, so the walk
    /// terminates at `depth_cap` hops rather than at `depth_cap` nodes.
    ///
    /// The `HashSet` is the walk's frontier, not a copy of the spine: it holds
    /// the answer, which is what the caller asked for.
    #[must_use]
    pub fn supers_of(&self, c: ClassAddr) -> HashSet<ClassAddr> {
        let mut seen = HashSet::new();
        let mut queue = VecDeque::from([(c, 0usize)]);
        while let Some((node, depth)) = queue.pop_front() {
            if depth >= self.depth_cap {
                continue;
            }
            let (a, b) = self.parents_of(node);
            for e in a.iter().chain(b) {
                if seen.insert(e.sup) {
                    queue.push_back((e.sup, depth + 1));
                }
            }
        }
        seen
    }

    /// **The first question: does `sub ⊑ sup` follow?**
    ///
    /// Reflexive per R1: every class subsumes itself, which is a fact even when
    /// no edge is baked for it.
    #[must_use]
    pub fn entails(&self, sub: ClassAddr, sup: ClassAddr) -> bool {
        sub == sup || self.supers_of(sub).contains(&sup)
    }

    /// Every class with at least one baked parent, in sorted order and without
    /// duplicates — the walk domain for [`Self::equivalence_cycles`].
    fn subclasses(&self) -> Vec<ClassAddr> {
        let mut v: Vec<ClassAddr> = self
            .base
            .iter()
            .chain(self.overlay)
            .map(|e| e.sub)
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Classes that reach themselves — an equivalence cycle.
    ///
    /// Empty for a sound spine. A non-empty result is not automatically a bug:
    /// two classes an ontology models separately may be genuine synonyms. It IS
    /// always a finding, because a cycle makes the two mutually substitutable,
    /// and a consumer treating one as more specific than the other is then
    /// relying on something the baked set does not support.
    #[must_use]
    pub fn equivalence_cycles(&self) -> Vec<ClassAddr> {
        self.subclasses()
            .into_iter()
            .filter(|&c| self.supers_of(c).contains(&c))
            .collect()
    }
}

/// Sorted-by-`sub` check. Cheap enough to run in debug on every construction.
fn is_sorted(edges: &[Subsumption]) -> bool {
    edges.windows(2).all(|w| w[0].sub <= w[1].sub)
}

/// The window of `edges` whose `sub == c` — an equal-range binary search.
///
/// Returns a subslice of the input, which is the point: a parent lookup costs a
/// `log n` probe and yields a borrowed view, never an allocation.
fn equal_range(edges: &[Subsumption], c: ClassAddr) -> &[Subsumption] {
    let lo = edges.partition_point(|e| e.sub < c);
    let hi = edges.partition_point(|e| e.sub <= c);
    &edges[lo..hi]
}

/// What merging a set of axioms into a baked spine would do.
///
/// Returned by [`Spine::merge`] **without applying anything** — a caller decides
/// whether to accept the merge after seeing the verdict, which is the only order
/// that makes the verdict useful. Nothing is mutated because nothing is owned.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MergeVerdict {
    /// Axioms the baked spine already entailed. These corroborate it and add
    /// nothing — the strongest possible outcome for an axiom, and the one that
    /// carries no risk.
    pub corroborating: Vec<Subsumption>,
    /// Axioms the spine did NOT entail, and which introduce no cycle. New
    /// structure the bake was silent about — **enrichment**, not conflict.
    /// Silence is not disagreement.
    pub enriching: Vec<Subsumption>,
    /// Classes drawn into an equivalence cycle BY this merge. Empty means the
    /// merge is sound over this fragment.
    pub introduced_cycles: Vec<ClassAddr>,
    /// Cycles already present in the bake — reported so the merge cannot be
    /// blamed for them.
    pub pre_existing_cycles: Vec<ClassAddr>,
}

impl MergeVerdict {
    /// Whether the merge introduces no new equivalence cycle.
    ///
    /// Note what this does NOT claim: soundness **over this fragment**. A merge
    /// that is sound here can still be wrong under role composition or bottom
    /// propagation, neither of which this crate implements. A caller needing
    /// that guarantee needs a full reasoner, and the crate doc says where the
    /// line is.
    #[must_use]
    pub fn is_sound(&self) -> bool {
        self.introduced_cycles.is_empty()
    }

    /// Total axioms considered.
    #[must_use]
    pub fn considered(&self) -> usize {
        self.corroborating.len() + self.enriching.len()
    }
}

impl Spine<'_> {
    /// **The second question: is adding `axioms` sound?**
    ///
    /// Splits them into corroborating and enriching, then observes the spine
    /// **with the enriching ones overlaid** and reports any equivalence cycle
    /// that appears. The overlay is borrowed, so the "what would happen"
    /// question costs a second lens and no copy of the spine.
    ///
    /// `axioms` must be sorted by `sub` — same precondition as the base, for
    /// the same reason.
    #[must_use]
    pub fn merge(&self, axioms: &[Subsumption]) -> MergeVerdict {
        let pre: HashSet<ClassAddr> = self.equivalence_cycles().into_iter().collect();

        let mut corroborating = Vec::new();
        let mut enriching = Vec::new();
        for &ax in axioms {
            if self.entails(ax.sub, ax.sup) {
                corroborating.push(ax);
            } else {
                enriching.push(ax);
            }
        }

        // Only the enriching axioms can change what follows, so only they are
        // overlaid for the cycle check. Including the corroborating ones would
        // cost a wider walk to reach an identical answer.
        //
        // The overlay must be sorted on its own terms — `axioms` being sorted
        // does not survive the filter's re-collection into a new Vec in general,
        // so it is re-sorted here rather than assumed.
        let mut overlay = enriching.clone();
        overlay.sort_unstable();
        let merged = Spine {
            base: self.base,
            overlay: &overlay,
            depth_cap: self.depth_cap,
        };

        let mut introduced: Vec<ClassAddr> = merged
            .equivalence_cycles()
            .into_iter()
            .filter(|c| !pre.contains(c))
            .collect();
        introduced.sort_unstable();

        let mut pre_existing: Vec<ClassAddr> = pre.into_iter().collect();
        pre_existing.sort_unstable();

        MergeVerdict {
            corroborating,
            enriching,
            introduced_cycles: introduced,
            pre_existing_cycles: pre_existing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NS: u32 = 0x0301_0000;

    fn c(id: u32) -> ClassAddr {
        ClassAddr::new(NS, id)
    }

    fn sub(a: u32, b: u32) -> Subsumption {
        Subsumption::new(c(a), c(b))
    }

    /// Baked edges arrive sorted; the tests mirror that rather than pretending
    /// the lens sorts for them.
    fn baked(mut e: Vec<Subsumption>) -> Vec<Subsumption> {
        e.sort_unstable();
        e
    }

    /// R1 + R2: transitivity derives what is not baked, and reflexivity holds
    /// for a class with no edges at all.
    ///
    /// The anti-vacuity half is `!entails(1, 9)`: without it, an `entails` that
    /// returned `true` unconditionally would pass every positive assertion here.
    #[test]
    fn transitivity_derives_and_reflexivity_holds() {
        let e = baked(vec![sub(1, 2), sub(2, 3), sub(3, 4)]);
        let s = Spine::over(&e);
        assert!(s.entails(c(1), c(2)), "baked");
        assert!(s.entails(c(1), c(4)), "derived through two hops");
        assert!(s.entails(c(7), c(7)), "reflexive even with no edges");
        assert!(!s.entails(c(1), c(9)), "must NOT entail an unrelated class");
        assert!(!s.entails(c(4), c(1)), "and must not run the wrong way");
    }

    /// The lens borrows: a parent lookup hands back a window into the caller's
    /// own slice, and multiple parents land in one contiguous run.
    #[test]
    fn parents_are_a_borrowed_window_not_a_copy() {
        let e = baked(vec![sub(1, 2), sub(1, 3), sub(4, 5)]);
        let s = Spine::over(&e);
        let (base, overlay) = s.parents_of(c(1));
        assert_eq!(base.len(), 2, "both parents in one equal-range window");
        assert!(overlay.is_empty(), "no overlay in view");
        assert!(
            std::ptr::eq(base.as_ptr(), e.as_ptr()),
            "the window points INTO the caller's slice — nothing was copied"
        );
        assert!(s.parents_of(c(9)).0.is_empty(), "an absent class has none");
    }

    /// An unsorted slice is refused rather than silently mis-searched, and a
    /// sorted one is accepted — the discriminating pair.
    #[test]
    fn try_over_refuses_unsorted_and_accepts_sorted() {
        let bad = vec![sub(5, 6), sub(1, 2)];
        assert!(Spine::try_over(&bad).is_none(), "unsorted must be refused");
        let good = baked(bad);
        assert!(Spine::try_over(&good).is_some(), "sorted must be accepted");
    }

    /// The depth cap is a real limit and is REPORTED, so "not entailed" stays
    /// distinguishable from "we stopped looking".
    #[test]
    fn the_depth_cap_binds_and_is_visible() {
        let e = baked((1..10).map(|i| sub(i, i + 1)).collect());
        let shallow = Spine::over(&e).with_depth_cap(2);
        assert_eq!(shallow.depth_cap(), 2);
        assert!(!shallow.entails(c(1), c(10)), "beyond the cap");
        assert!(shallow.entails(c(1), c(3)), "within the cap");
        assert!(
            Spine::over(&e).entails(c(1), c(10)),
            "reachable at the default cap"
        );
    }

    /// A sound spine has no cycles; a two-way edge produces one.
    ///
    /// The can-stay-silent half uses a NON-trivial acyclic graph — an empty
    /// spine would prove only that emptiness has no cycles.
    #[test]
    fn cycles_are_detected_and_absent_when_they_should_be() {
        let ok = baked(vec![sub(1, 2), sub(2, 3), sub(1, 3), sub(4, 3)]);
        assert!(
            Spine::over(&ok).equivalence_cycles().is_empty(),
            "a real acyclic spine reports no cycle"
        );
        let bad = baked(vec![sub(1, 2), sub(2, 1)]);
        assert_eq!(
            Spine::over(&bad).equivalence_cycles(),
            vec![c(1), c(2)],
            "both ends of the equivalence are named"
        );
    }

    /// A cycle closing through a LONG chain is still found — the property that
    /// makes this a reasoner rather than a pairwise check.
    #[test]
    fn a_cycle_through_a_long_chain_is_found() {
        let mut v: Vec<Subsumption> = (1..8).map(|i| sub(i, i + 1)).collect();
        v.push(sub(8, 1)); // closes 1 → 8 → 1
        let e = baked(v);
        assert_eq!(
            Spine::over(&e).equivalence_cycles().len(),
            8,
            "every class on the ring"
        );
    }

    /// **The merge verdict discriminates all three outcomes.** A verdict that
    /// answered one class for everything would carry no information.
    #[test]
    fn merge_splits_corroborating_enriching_and_conflicting() {
        let e = baked(vec![sub(1, 2), sub(2, 3)]);
        let s = Spine::over(&e);

        // Corroborating: already derivable through 1 ⊑ 2 ⊑ 3.
        let v = s.merge(&[sub(1, 3)]);
        assert_eq!(v.corroborating.len(), 1);
        assert!(v.enriching.is_empty());
        assert!(v.is_sound());

        // Enriching: the bake is SILENT about 4, not opposed to it.
        let v = s.merge(&[sub(4, 3)]);
        assert!(v.corroborating.is_empty());
        assert_eq!(v.enriching.len(), 1);
        assert!(v.is_sound(), "silence is not disagreement");

        // Conflicting: the bake says 1 ⊑ 3; the reverse closes a cycle.
        let v = s.merge(&[sub(3, 1)]);
        assert!(!v.is_sound(), "a direction conflict must NOT read as sound");
        assert_eq!(v.introduced_cycles, vec![c(1), c(2), c(3)]);
        assert!(v.pre_existing_cycles.is_empty(), "the bake was clean");
    }

    /// Pre-existing cycles are never blamed on the merge — the control that
    /// makes `introduced_cycles` mean what it says.
    #[test]
    fn a_pre_existing_cycle_is_not_attributed_to_the_merge() {
        let e = baked(vec![sub(1, 2), sub(2, 1)]);
        let v = Spine::over(&e).merge(&[sub(5, 6)]);
        assert_eq!(v.pre_existing_cycles, vec![c(1), c(2)]);
        assert!(
            v.introduced_cycles.is_empty(),
            "an innocent merge stays innocent against a dirty base"
        );
        assert!(v.is_sound());
    }

    /// `merge` observes; the overlay applies. The base slice is untouched by
    /// either, because the lens never had write access to begin with.
    #[test]
    fn merge_observes_and_the_overlay_applies() {
        let e = baked(vec![sub(1, 2)]);
        let s = Spine::over(&e);
        let v = s.merge(&[sub(2, 3)]);
        assert_eq!(e.len(), 1, "the baked slice was not written to");
        assert!(!s.entails(c(1), c(3)), "and the lens still sees only the bake");
        let extra = baked(v.enriching.clone());
        assert!(
            s.with_overlay(&extra).entails(c(1), c(3)),
            "overlaying really applies it"
        );
    }

    /// Addresses are class-scoped: the same identity under a different classid
    /// is a different class, so no relation leaks across namespaces.
    #[test]
    fn identity_alone_does_not_make_two_classes_the_same() {
        let other = 0x0302_0000;
        let e = baked(vec![Subsumption::new(
            ClassAddr::new(NS, 1),
            ClassAddr::new(NS, 2),
        )]);
        let s = Spine::over(&e);
        assert!(!s.entails(ClassAddr::new(other, 1), ClassAddr::new(NS, 2)));
        assert!(s.entails(ClassAddr::new(NS, 1), ClassAddr::new(NS, 2)));
    }
}
