//! `ogar-elk` — the EL subsumption closure as a **factfinder**.
//!
//! # What this is, and the two questions it answers
//!
//! Third member of the factfinder family beside [`ogar-obo`] (harvest and bake)
//! and [`ogar-ro`] (the predicate palette). Those two say what is *asserted*;
//! this one observes the baked result and says what *follows*. It answers
//! exactly two questions:
//!
//! 1. **Does `A ⊑ B` follow** from the asserted edges? ([`Closure::entails`])
//! 2. **Is adding a set of axioms sound** — does the merged closure introduce
//!    an equivalence cycle that was not already there? ([`Closure::merge`])
//!
//! Nothing here is graded, ranked, scored or weighted. An EL entailment is a
//! **fact**: it follows necessarily from the asserted set, or it does not. The
//! thinking that consumes these facts lives elsewhere; this crate is unfazed by
//! it.
//!
//! [`ogar-obo`]: https://docs.rs/ogar-obo
//! [`ogar-ro`]: https://docs.rs/ogar-ro
//!
//! # Where this runs: AFTER the bake, as observation
//!
//! **The joins are pre-bake; this is not a join.** Reconciling independently
//! authored sources is upstream work that must finish before anything is baked,
//! because the bake is what freezes the answer into positions. What this crate
//! does is the opposite end: it **observes the baked substrate** and reports
//! what follows from it. The bake is its input, not its output.
//!
//! Concretely, its input is the `is_a` links carried in a baked row's own edge
//! lanes — `ogar_obo::edges::subsumptions` is the adapter, and it exists so a
//! reasoner reads addresses out of positions rather than re-deriving them.
//!
//! Three consequences follow, and they are the whole design:
//!
//! 1. **These are readings, not schema.** [`ClassAddr`] is the `(classid,
//!    identity)` pair a baked key already carries, decoded from its position —
//!    the same shape `ogar_obo::edges::Link` reads out of a lane. It declares
//!    nothing about a class and outlives nothing; a class is still resolved by
//!    position. [`Subsumption`] is a direction, and it is a distinct type only
//!    so a caller cannot hand this crate a `part_of` edge by accident.
//! 2. **Nothing here is in the hot path**, so nothing here may pretend to be.
//!    There is no serialization surface — not behind a feature, not optionally.
//!    An observer that could serialize its verdict would invite someone to ship
//!    the verdict as if it were substrate.
//! 3. **No file, no CURIE, no label.** Input is addressed edges read from baked
//!    rows — which carry no CURIE and no label to reach for in the first place.
//!    Reaching back for the source document would put parsing inside the
//!    observer and re-introduce the coupling the bake exists to remove.
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
//! mistake because it only ever ingests subsumptions — but a caller that feeds
//! it a mixed edge set can, so [`Closure::from_asserted`] takes subsumptions by
//! type rather than raw pairs.

#![forbid(unsafe_code)]

pub mod lens;

pub use lens::{Fillers, LensClosure, Parents, Role, fillers_closed, meet_via, most_specific};

use std::collections::{HashMap, HashSet, VecDeque};

/// The **join key** the pre-bake reconciliation uses: `(classid, identity)`.
///
/// Not an ABI address, and deliberately not documented as one — it is the pair
/// the joiner has already agreed on for the two sides it is joining, which is
/// all a closure needs to index by. What a *baked* row carries is a matter of
/// position, resolved by the class, and this crate is finished long before that
/// question is asked.
///
/// Ordered and hashable so a closure can index by it without a side table. It
/// carries no namespace, no CURIE and no label — resolving those is the
/// caller's business and none of this crate's.
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

/// An asserted subsumption `sub ⊑ sup`.
///
/// A distinct type rather than a `(ClassAddr, ClassAddr)` tuple, so a caller
/// cannot hand this crate a `part_of` edge by accident — see the crate doc's
/// false-ancestor hazard.
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
/// shallow, but this crate runs on real harvested data where a cycle would
/// otherwise spin forever — and [`Closure::equivalence_cycles`] exists precisely
/// because cycles DO occur. The cap is reported by [`Closure::depth_cap`] so a
/// caller can tell "not entailed" from "we stopped looking", which are different
/// answers and must not be conflated.
pub const DEFAULT_DEPTH_CAP: usize = 64;

/// The asserted subsumption graph, closed on demand.
///
/// The closure is computed per query rather than materialised: a spine of N
/// classes has an O(N²) transitive closure but only O(N) asserted edges, and
/// almost every caller asks about a handful of classes. Materialising would
/// trade a cheap walk for an expensive table nobody reads in full.
#[derive(Debug, Clone, Default)]
pub struct Closure {
    parents: HashMap<ClassAddr, Vec<ClassAddr>>,
    depth_cap: usize,
}

impl Closure {
    /// Build from asserted subsumptions.
    ///
    /// Duplicate assertions are kept as asserted — deduplicating here would
    /// silently change what [`Self::asserted_len`] reports, and a caller
    /// counting its own input has a right to get its own number back.
    #[must_use]
    pub fn from_asserted(edges: impl IntoIterator<Item = Subsumption>) -> Self {
        let mut parents: HashMap<ClassAddr, Vec<ClassAddr>> = HashMap::new();
        for e in edges {
            parents.entry(e.sub).or_default().push(e.sup);
        }
        Self {
            parents,
            depth_cap: DEFAULT_DEPTH_CAP,
        }
    }

    /// Override the transitivity depth guard.
    #[must_use]
    pub fn with_depth_cap(mut self, cap: usize) -> Self {
        self.depth_cap = cap;
        self
    }

    /// The depth guard in force — so a caller can distinguish "not entailed"
    /// from "the walk stopped".
    #[must_use]
    pub const fn depth_cap(&self) -> usize {
        self.depth_cap
    }

    /// Number of asserted edges.
    #[must_use]
    pub fn asserted_len(&self) -> usize {
        self.parents.values().map(Vec::len).sum()
    }

    /// Whether any edge is asserted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parents.is_empty()
    }

    /// Classes with at least one asserted superclass.
    pub fn subclasses(&self) -> impl Iterator<Item = ClassAddr> + '_ {
        self.parents.keys().copied()
    }

    /// **R1 + R2** — every superclass of `c`, transitively.
    ///
    /// Excludes `c` itself unless a cycle genuinely returns to it, which is the
    /// signal [`Self::equivalence_cycles`] reads. Breadth-first, so the walk
    /// terminates at `depth_cap` hops rather than at `depth_cap` nodes.
    #[must_use]
    pub fn supers_of(&self, c: ClassAddr) -> HashSet<ClassAddr> {
        let mut seen = HashSet::new();
        let mut queue = VecDeque::from([(c, 0usize)]);
        while let Some((node, depth)) = queue.pop_front() {
            if depth >= self.depth_cap {
                continue;
            }
            for &p in self.parents.get(&node).into_iter().flatten() {
                if seen.insert(p) {
                    queue.push_back((p, depth + 1));
                }
            }
        }
        seen
    }

    /// **The first question: does `sub ⊑ sup` follow?**
    ///
    /// Reflexive per R1: every class subsumes itself, which is a fact even when
    /// no edge is asserted for it.
    #[must_use]
    pub fn entails(&self, sub: ClassAddr, sup: ClassAddr) -> bool {
        sub == sup || self.supers_of(sub).contains(&sup)
    }

    /// Classes that reach themselves — an equivalence cycle.
    ///
    /// Empty for a sound spine. A non-empty result is not automatically a bug:
    /// two classes an ontology models separately may be genuine synonyms. It IS
    /// always a finding, because a cycle makes the two mutually substitutable,
    /// and a consumer treating one as more specific than the other is then
    /// relying on something the asserted set does not support.
    #[must_use]
    pub fn equivalence_cycles(&self) -> Vec<ClassAddr> {
        let mut out: Vec<ClassAddr> = self
            .parents
            .keys()
            .copied()
            .filter(|&c| self.supers_of(c).contains(&c))
            .collect();
        out.sort_unstable();
        out
    }
}

/// What merging a set of axioms into an existing closure would do.
///
/// Returned by [`Closure::merge`] **without mutating** the closure: a caller
/// decides whether to accept the merge after seeing the verdict, which is the
/// only order that makes the verdict useful.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MergeVerdict {
    /// Axioms the closure already entailed. These corroborate the existing set
    /// and add nothing — the strongest possible outcome for an axiom, and the
    /// one that carries no risk.
    pub corroborating: Vec<Subsumption>,
    /// Axioms the closure did NOT entail, and which introduce no cycle. New
    /// structure the existing set was silent about — **enrichment**, not
    /// conflict. Silence is not disagreement.
    pub enriching: Vec<Subsumption>,
    /// Classes drawn into an equivalence cycle BY this merge. Empty means the
    /// merge is sound over this fragment.
    pub introduced_cycles: Vec<ClassAddr>,
    /// Cycles that were already present before the merge — reported so the
    /// merge cannot be blamed for them.
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

impl Closure {
    /// **The second question: is adding `axioms` sound?**
    ///
    /// Splits them into corroborating and enriching, then closes the union and
    /// reports any equivalence cycle the merge introduced. The receiver is
    /// untouched; apply the merge yourself with [`Self::extended`] once the
    /// verdict is acceptable.
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

        // Only the enriching axioms can change the closure, so only they are
        // merged for the cycle check. Including the corroborating ones would
        // cost a larger walk to reach an identical answer.
        let merged = self.extended(&enriching);
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

    /// A copy with `axioms` added. Pairs with [`Self::merge`]: check first,
    /// then extend.
    #[must_use]
    pub fn extended(&self, axioms: &[Subsumption]) -> Closure {
        let mut parents = self.parents.clone();
        for ax in axioms {
            parents.entry(ax.sub).or_default().push(ax.sup);
        }
        Closure {
            parents,
            depth_cap: self.depth_cap,
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

    /// R1 + R2: transitivity derives what is not asserted, and reflexivity
    /// holds for a class with no edges at all.
    ///
    /// The anti-vacuity half is `!entails(1, 9)`: without it, an `entails` that
    /// returned `true` unconditionally would pass every positive assertion here.
    #[test]
    fn transitivity_derives_and_reflexivity_holds() {
        let cl = Closure::from_asserted([sub(1, 2), sub(2, 3), sub(3, 4)]);
        assert!(cl.entails(c(1), c(2)), "asserted");
        assert!(cl.entails(c(1), c(4)), "derived through two hops");
        assert!(cl.entails(c(7), c(7)), "reflexive even with no edges");
        assert!(
            !cl.entails(c(1), c(9)),
            "must NOT entail an unrelated class"
        );
        assert!(!cl.entails(c(4), c(1)), "and must not run the wrong way");
    }

    /// The depth cap is a real limit and is REPORTED, so "not entailed" stays
    /// distinguishable from "we stopped looking".
    ///
    /// Both directions are asserted: at cap 2 the far end is unreachable, at
    /// the default it is reachable. A cap that did nothing would fail the first
    /// assertion; one that clamped everything would fail the second.
    #[test]
    fn the_depth_cap_binds_and_is_visible() {
        let chain: Vec<Subsumption> = (1..10).map(|i| sub(i, i + 1)).collect();
        let shallow = Closure::from_asserted(chain.clone()).with_depth_cap(2);
        assert_eq!(shallow.depth_cap(), 2);
        assert!(!shallow.entails(c(1), c(10)), "beyond the cap");
        assert!(shallow.entails(c(1), c(3)), "within the cap");
        let deep = Closure::from_asserted(chain);
        assert!(deep.entails(c(1), c(10)), "reachable at the default cap");
    }

    /// A sound spine has no cycles; a two-way assertion produces one.
    ///
    /// The can-stay-silent half uses a NON-trivial acyclic graph — an empty
    /// closure would prove only that emptiness has no cycles.
    #[test]
    fn cycles_are_detected_and_absent_when_they_should_be() {
        let acyclic = Closure::from_asserted([sub(1, 2), sub(2, 3), sub(1, 3), sub(4, 3)]);
        assert!(
            acyclic.equivalence_cycles().is_empty(),
            "a real acyclic spine reports no cycle"
        );
        let cyclic = Closure::from_asserted([sub(1, 2), sub(2, 1)]);
        assert_eq!(
            cyclic.equivalence_cycles(),
            vec![c(1), c(2)],
            "both ends of the equivalence are named"
        );
    }

    /// A cycle closing through a LONG chain is still found — the property that
    /// makes this a reasoner rather than a pairwise check.
    #[test]
    fn a_cycle_through_a_long_chain_is_found() {
        let mut axioms: Vec<Subsumption> = (1..8).map(|i| sub(i, i + 1)).collect();
        axioms.push(sub(8, 1)); // closes 1 → 8 → 1
        let cl = Closure::from_asserted(axioms);
        assert_eq!(cl.equivalence_cycles().len(), 8, "every class on the ring");
    }

    /// **The merge verdict discriminates all three outcomes.** A verdict that
    /// answered one class for everything would carry no information.
    #[test]
    fn merge_splits_corroborating_enriching_and_conflicting() {
        let base = Closure::from_asserted([sub(1, 2), sub(2, 3)]);

        // Corroborating: already derivable through 1 ⊑ 2 ⊑ 3.
        let v = base.merge(&[sub(1, 3)]);
        assert_eq!(v.corroborating.len(), 1);
        assert!(v.enriching.is_empty());
        assert!(v.is_sound());

        // Enriching: the base is SILENT about 4, not opposed to it.
        let v = base.merge(&[sub(4, 3)]);
        assert!(v.corroborating.is_empty());
        assert_eq!(v.enriching.len(), 1);
        assert!(v.is_sound(), "silence is not disagreement");

        // Conflicting: the base says 1 ⊑ 3; the reverse closes a cycle.
        let v = base.merge(&[sub(3, 1)]);
        assert!(!v.is_sound(), "a direction conflict must NOT read as sound");
        assert_eq!(v.introduced_cycles, vec![c(1), c(2), c(3)]);
        assert!(v.pre_existing_cycles.is_empty(), "the base was clean");
    }

    /// Pre-existing cycles are never blamed on the merge — the control that
    /// makes `introduced_cycles` mean what it says.
    #[test]
    fn a_pre_existing_cycle_is_not_attributed_to_the_merge() {
        let dirty = Closure::from_asserted([sub(1, 2), sub(2, 1)]);
        let v = dirty.merge(&[sub(5, 6)]);
        assert_eq!(v.pre_existing_cycles, vec![c(1), c(2)]);
        assert!(
            v.introduced_cycles.is_empty(),
            "an innocent merge stays innocent against a dirty base"
        );
        assert!(v.is_sound());
    }

    /// `merge` does not mutate; `extended` does. Checking first and applying
    /// after is the only order in which a verdict is useful.
    #[test]
    fn merge_inspects_and_extended_applies() {
        let base = Closure::from_asserted([sub(1, 2)]);
        let before = base.asserted_len();
        let v = base.merge(&[sub(2, 3)]);
        assert_eq!(base.asserted_len(), before, "merge left the closure alone");
        assert!(!base.entails(c(1), c(3)));
        let applied = base.extended(&v.enriching);
        assert!(
            applied.entails(c(1), c(3)),
            "and extended really applies it"
        );
    }

    /// Addresses are class-scoped: the same identity under a different classid
    /// is a different class, so no relation leaks across namespaces.
    #[test]
    fn identity_alone_does_not_make_two_classes_the_same() {
        let other = 0x0302_0000;
        let cl = Closure::from_asserted([Subsumption::new(
            ClassAddr::new(NS, 1),
            ClassAddr::new(NS, 2),
        )]);
        assert!(!cl.entails(ClassAddr::new(other, 1), ClassAddr::new(NS, 2)));
        assert!(cl.entails(ClassAddr::new(NS, 1), ClassAddr::new(NS, 2)));
    }
}
