//! The **lazy lens** seam — reasoning against borrowed rows, with no index built.
//!
//! [`Closure`] indexes its edges into a `HashMap` up front. That is the right
//! shape when the asserted set is already in memory and will be queried many
//! times, and the wrong shape when the substrate is a hydrated columnar table
//! you would rather not copy: building the index costs a pass over every edge
//! and holds a map proportional to them, before the first question is asked.
//!
//! This module is the other end. A [`Parents`] implementation answers *one*
//! question — who are this class's asserted superclasses — and [`LensClosure`]
//! computes entailment on top of it, touching only the classes a query actually
//! reaches.
//!
//! # No hashing, anywhere on this path
//!
//! Dedup is an ordered insert and membership is a binary search. Nothing here
//! hashes. That is not a micro-optimisation: the addresses arrive already
//! ordered from the bake, hashing throws that ordering away, and it has to be
//! rebuilt before two ancestries can be intersected. Keeping the order means
//! [`LensClosure::meet`] is one merge pass, and it means two runs produce
//! byte-identical output — the property a layer that emits *facts* has to have.
//!
//! # The lens never caches, and that is the law rather than an oversight
//!
//! A transitive closure is a deterministic function of the asserted edges: it
//! recomputes equal, every time. Under the workspace's zero-copy law that makes
//! it a **projection** — `output_rung == max(input_rungs)` — and a projection is
//! never stored, however much arithmetic produced it. So [`LensClosure`] holds
//! no memo table, no visited cache between calls, and no adjacency copy. It
//! holds a borrowed reference and a depth cap.
//!
//! The cost of that is real and is not hidden: repeated queries redo work. What
//! it buys is that nothing is resident between queries, which is the point when
//! the rows live in a hydrated table an order of magnitude larger than RAM.
//!
//! # What it does NOT change
//!
//! The answers. [`LensClosure::supers_of`] and [`LensClosure::entails`] return
//! exactly what [`Closure`]'s equivalents return over the same asserted set —
//! asserted in the test module, and measured against the real bake in
//! `examples/elk_lens_vs_closure.rs`. A lens that were *cheaper* and *different*
//! would be a different reasoner wearing this one's name.
//!
//! # Where the lens genuinely wins
//!
//! [`LensClosure::entails`] **early-exits** on the first hit. `Closure::entails`
//! is `sub == sup || self.supers_of(sub).contains(&sup)`, which materialises the
//! whole ancestor set before testing membership. For a yes-answer near the query
//! class, the lens stops there; for a no-answer both walk the same ground.

use std::collections::VecDeque;

use crate::{ClassAddr, DEFAULT_DEPTH_CAP};

/// A source of **asserted** superclasses for a class.
///
/// One method, taking a visitor rather than returning a collection: an
/// implementation over borrowed rows can then decode links in place and hand
/// them out one at a time, with nothing allocated per call. Returning
/// `Vec<ClassAddr>` would put an allocation on the hot path of every hop and
/// quietly undo the reason this trait exists.
///
/// **Asserted, not entailed.** An implementation reports the direct parents it
/// holds and performs no transitive reasoning of its own — that is
/// [`LensClosure`]'s job, and splitting it this way is what lets the same lens
/// serve a closure, a one-hop query, or a cycle check without any of them
/// re-deriving what the others did.
pub trait Parents {
    /// Visit each asserted superclass of `c`, in any order.
    ///
    /// Called once per class per walk, so an implementation should avoid
    /// allocating. Duplicates are permitted; [`LensClosure`] de-duplicates.
    fn parents_of(&self, c: ClassAddr, visit: &mut dyn FnMut(ClassAddr));
}

/// Any closure of the right shape is a [`Parents`] source.
///
/// This is what keeps the seam dependency-free in both directions: the crate
/// that owns the byte layout can hand over a closure capturing its own borrowed
/// view, and neither crate has to name the other's types. There is no adapter
/// struct to write and no `ogar-obo`/`ogar-elk` dependency edge in either
/// direction.
impl<F> Parents for F
where
    F: Fn(ClassAddr, &mut dyn FnMut(ClassAddr)),
{
    #[inline]
    fn parents_of(&self, c: ClassAddr, visit: &mut dyn FnMut(ClassAddr)) {
        self(c, visit);
    }
}

/// Entailment computed against a [`Parents`] source, materialising nothing.
///
/// Borrows its source; holds no state beyond the depth cap. Constructing one is
/// free, so it is meant to be created per query rather than kept.
#[derive(Debug, Clone, Copy)]
pub struct LensClosure<'p, P: Parents + ?Sized> {
    parents: &'p P,
    depth_cap: usize,
}

impl<'p, P: Parents + ?Sized> LensClosure<'p, P> {
    /// Borrow a parent source at the default depth cap.
    #[must_use]
    pub const fn new(parents: &'p P) -> Self {
        Self {
            parents,
            depth_cap: DEFAULT_DEPTH_CAP,
        }
    }

    /// Override the transitivity depth guard.
    #[must_use]
    pub const fn with_depth_cap(mut self, cap: usize) -> Self {
        self.depth_cap = cap;
        self
    }

    /// The depth guard in force — so a caller can tell "not entailed" from
    /// "the walk stopped", which are different answers.
    #[must_use]
    pub const fn depth_cap(&self) -> usize {
        self.depth_cap
    }

    /// **R1 + R2** — every superclass of `c`, transitively.
    ///
    /// Breadth-first, so the walk terminates at `depth_cap` hops rather than at
    /// `depth_cap` nodes. Excludes `c` itself unless a cycle genuinely returns
    /// to it — the signal [`Self::reaches_itself`] reads.
    ///
    /// The returned set is the caller's answer, not a cache: it is dropped when
    /// the caller drops it, and asking twice walks twice.
    #[must_use]
    pub fn supers_of(&self, c: ClassAddr) -> Vec<ClassAddr> {
        let mut seen: Vec<ClassAddr> = Vec::new();
        let mut queue = VecDeque::from([(c, 0usize)]);
        while let Some((node, depth)) = queue.pop_front() {
            if depth >= self.depth_cap {
                continue;
            }
            self.parents.parents_of(node, &mut |p| {
                // Ordered insert, not a hash probe. The result stays sorted by
                // construction, which is what makes `meet` a merge pass — and
                // it is deterministic run to run, which a hash set is not.
                if let Err(at) = seen.binary_search(&p) {
                    seen.insert(at, p);
                    queue.push_back((p, depth + 1));
                }
            });
        }
        seen
    }

    /// **Does `sub ⊑ sup` follow?** Reflexive per R1.
    ///
    /// Early-exits on the first hit instead of building the full ancestor set,
    /// which is the one place the lens is strictly cheaper than the indexed
    /// [`Closure`](crate::Closure) rather than merely leaner.
    #[must_use]
    pub fn entails(&self, sub: ClassAddr, sup: ClassAddr) -> bool {
        if sub == sup {
            return true;
        }
        let mut seen: Vec<ClassAddr> = Vec::new();
        let mut queue = VecDeque::from([(sub, 0usize)]);
        while let Some((node, depth)) = queue.pop_front() {
            if depth >= self.depth_cap {
                continue;
            }
            let mut hit = false;
            self.parents.parents_of(node, &mut |p| {
                if p == sup {
                    hit = true;
                }
                if let Err(at) = seen.binary_search(&p) {
                    seen.insert(at, p);
                    queue.push_back((p, depth + 1));
                }
            });
            if hit {
                return true;
            }
        }
        false
    }

    /// Ancestors as a **sorted, de-duplicated** address list.
    ///
    /// The shape [`Self::supers_of`] should have had from the start. A hash set
    /// discards the ordering the bake already provides — rows are emitted
    /// ascending by `(classid, identity)` — and that ordering is exactly what
    /// makes [`meet`](Self::meet) a single merge pass instead of a lookup
    /// storm. Sorted output is also deterministic run to run, which a hash set
    /// is not, and determinism is the property a fact-producing layer is
    /// supposed to have.
    #[must_use]
    pub fn ancestors_sorted(&self, c: ClassAddr) -> Vec<ClassAddr> {
        // already sorted by construction — supers_of does ordered inserts
        self.supers_of(c)
    }

    /// **Schnittpunkte** — the addresses reachable from *every* seed.
    ///
    /// This is the multi-hop cross-ontology lookup, and it is deliberately not
    /// a matrix product: an x-hop question across ontologies asks where the
    /// reachable sets MEET, and meeting is an intersection over sorted
    /// addresses — one merge pass per seed, no vectors, no per-edge payload,
    /// nothing serialized.
    ///
    /// Returns sorted addresses. An empty seed list yields nothing (there is no
    /// meaningful "intersection of no sets" here — the identity would be every
    /// address in existence, which is not an answer a caller can use).
    ///
    /// The seeds may sit in different ontologies; the result names wherever
    /// their ancestries coincide, which is the fact the caller is after.
    #[must_use]
    pub fn meet(&self, seeds: &[ClassAddr]) -> Vec<ClassAddr> {
        let Some((first, rest)) = seeds.split_first() else {
            return Vec::new();
        };
        let mut acc = self.ancestors_sorted(*first);
        for s in rest {
            if acc.is_empty() {
                break;
            }
            acc = intersect_sorted(&acc, &self.ancestors_sorted(*s));
        }
        acc
    }

    /// Whether `c` reaches itself — an equivalence cycle through `c`.
    ///
    /// Per-class rather than whole-graph: a lens has no membership roll to
    /// enumerate, and inventing one would mean materialising the very index
    /// this module exists to avoid. A caller that wants every cycle iterates
    /// the classes it cares about; a caller that has the full roll can use
    /// [`Closure::equivalence_cycles`](crate::Closure::equivalence_cycles).
    #[must_use]
    pub fn reaches_itself(&self, c: ClassAddr) -> bool {
        self.supers_of(c).binary_search(&c).is_ok()
    }
}

/// Intersect two sorted, de-duplicated address lists in one pass.
///
/// Linear in the inputs and allocation-free apart from the result, which is why
/// the sorted representation is worth keeping: the same job over hash sets is a
/// probe per element and returns in arbitrary order.
#[must_use]
pub fn intersect_sorted(a: &[ClassAddr], b: &[ClassAddr]) -> Vec<ClassAddr> {
    let (mut i, mut j) = (0usize, 0usize);
    let mut out = Vec::new();
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            core::cmp::Ordering::Less => i += 1,
            core::cmp::Ordering::Greater => j += 1,
            core::cmp::Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
        }
    }
    out
}

// ── S4b: the existential arm (R∃) ────────────────────────────────────────────

/// An existential role — the `r` in `A ⊑ ∃r.B`.
///
/// A plain tag rather than a rich type, so this crate stays zero-dep and does
/// not need the producer's predicate enum. It is deliberately NOT a
/// [`Subsumption`](crate::Subsumption): a role is an existential restriction,
/// and the two compose by different rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Role(pub u8);

/// A source of **asserted existential fillers** for a class under one role.
///
/// Same visitor shape as [`Parents`], and for the same reason: an
/// implementation over borrowed rows hands out fillers one at a time with
/// nothing allocated per call.
pub trait Fillers {
    /// Visit each asserted filler of `role` on `c`, in any order.
    fn fillers_of(&self, c: ClassAddr, role: Role, visit: &mut dyn FnMut(ClassAddr));
}

impl<F> Fillers for F
where
    F: Fn(ClassAddr, Role, &mut dyn FnMut(ClassAddr)),
{
    #[inline]
    fn fillers_of(&self, c: ClassAddr, role: Role, visit: &mut dyn FnMut(ClassAddr)) {
        self(c, role, visit);
    }
}

/// **R∃** — `A ⊑ ∃r.B` and `B ⊑ C` give `A ⊑ ∃r.C`.
///
/// Returns every filler of `role` on `subject`, each generalized upward through
/// `grounding_spine`. Sorted and de-duplicated; no hashing.
///
/// # The two spines are NOT the same, and swapping them is the classic error
///
/// `grounding_spine` is the spine a **filler** generalizes up. For the OBO core
/// that is `is_a ∪ part_of`: an anatomical filler's grounding is inherited
/// through mereology as well as taxonomy, which is the deductive form of
/// "inherit grounding". It is deliberately a DIFFERENT source from the one
/// [`LensClosure::entails`] uses, which must be `is_a` alone.
///
/// The distinction is exactly the false-ancestor hazard, and it is easy to get
/// backwards: generalizing a **subject** through `part_of` is unsound
/// (`A part_of B`, `B ⊑ C` does NOT give `A ⊑ C`); generalizing a **filler**
/// through it is sound and is what this function does. Both sources are
/// therefore explicit parameters — there is no default that could silently be
/// the wrong one.
///
/// Note this generalizes the filler only. A caller wanting "every phenotype of
/// this disease INCLUDING those asserted on its superclasses" walks the
/// subject's own subsumption ancestors first and unions the results — that is a
/// larger question, and keeping it out of here is what lets this match the
/// producer's own saturation count exactly.
#[must_use]
pub fn fillers_closed<S, F>(
    subject: ClassAddr,
    role: Role,
    fillers: &F,
    grounding_spine: &S,
    depth_cap: usize,
) -> Vec<ClassAddr>
where
    S: Parents + ?Sized,
    F: Fillers + ?Sized,
{
    let mut out: Vec<ClassAddr> = Vec::new();
    let spine = LensClosure::new(grounding_spine).with_depth_cap(depth_cap);
    fillers.fillers_of(subject, role, &mut |o| {
        if let Err(at) = out.binary_search(&o) {
            out.insert(at, o);
        }
        for a in spine.supers_of(o) {
            if let Err(at) = out.binary_search(&a) {
                out.insert(at, a);
            }
        }
    });
    out
}

/// The **cross-ontology Schnittpunkt** — where several subjects' role-fillers
/// meet, after R∃ generalization.
///
/// This is what `meet` over `is_a` alone cannot answer: subsumption never
/// leaves its own ontology, so two diseases only coincide once you cross by a
/// role (`has_phenotype` into HPO, `has_anatomy` into Uberon) and then climb
/// the filler's own spine. Intersection over sorted addresses, one merge pass
/// per seed.
///
/// Empty seeds yield nothing — the identity of an intersection over no sets is
/// not an answer a caller can use.
#[must_use]
pub fn meet_via<S, F>(
    seeds: &[ClassAddr],
    role: Role,
    fillers: &F,
    grounding_spine: &S,
    depth_cap: usize,
) -> Vec<ClassAddr>
where
    S: Parents + ?Sized,
    F: Fillers + ?Sized,
{
    let Some((first, rest)) = seeds.split_first() else {
        return Vec::new();
    };
    let mut acc = fillers_closed(*first, role, fillers, grounding_spine, depth_cap);
    for s in rest {
        if acc.is_empty() {
            break;
        }
        acc = intersect_sorted(
            &acc,
            &fillers_closed(*s, role, fillers, grounding_spine, depth_cap),
        );
    }
    acc
}

/// Keep only the **most specific** members of a sorted address set — those that
/// are not an ancestor of another member.
///
/// # Why a raw meet is sound and useless
///
/// Every filler generalizes up to its ontology's root, so ANY two subjects
/// share that root and [`meet_via`] returns it. Measured on the OBO core: 120
/// of 120 sampled disease pairs "shared a phenotype", and in every case the
/// shared address was `HP:0000001` — "All". A result that fires on every input
/// carries exactly as much information as one that never fires.
///
/// The informative answer to *what do these two subjects share* is the most
/// specific common filler, which is what this returns: drop `x` when some other
/// member `y` has `x` among its ancestors, because `y` already says everything
/// `x` does and says it more precisely.
///
/// Cost is |set|² spine walks in the worst case, which is fine because a meet
/// over seeded facts is small. It is deliberately NOT folded into [`meet_via`]:
/// the raw meet is the sound object, and narrowing is a presentation choice a
/// caller makes.
#[must_use]
pub fn most_specific<S>(set: &[ClassAddr], spine: &S, depth_cap: usize) -> Vec<ClassAddr>
where
    S: Parents + ?Sized,
{
    let lens = LensClosure::new(spine).with_depth_cap(depth_cap);
    let mut out = Vec::new();
    for (i, x) in set.iter().enumerate() {
        let redundant = set
            .iter()
            .enumerate()
            .any(|(j, y)| i != j && lens.supers_of(*y).binary_search(x).is_ok());
        if !redundant {
            out.push(*x);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Closure, Subsumption};

    const NS: u32 = 0x0301_0000;

    fn c(id: u32) -> ClassAddr {
        ClassAddr::new(NS, id)
    }

    /// A deliberately dumb source: a linear scan of asserted pairs. Slow on
    /// purpose — it shares no code with `Closure`'s HashMap index, so agreement
    /// between the two is evidence rather than tautology.
    struct Pairs(Vec<(u32, u32)>);

    impl Parents for Pairs {
        fn parents_of(&self, x: ClassAddr, visit: &mut dyn FnMut(ClassAddr)) {
            for (a, b) in &self.0 {
                if c(*a) == x {
                    visit(c(*b));
                }
            }
        }
    }

    fn both(edges: &[(u32, u32)]) -> (Closure, Pairs) {
        let cl = Closure::from_asserted(edges.iter().map(|(a, b)| Subsumption::new(c(*a), c(*b))));
        (cl, Pairs(edges.to_vec()))
    }

    #[test]
    fn the_lens_agrees_with_the_indexed_closure() {
        let edges = [(1, 2), (2, 3), (3, 4), (5, 3), (6, 7)];
        let (cl, src) = both(&edges);
        let lens = LensClosure::new(&src);
        for i in 1..=8u32 {
            let mut want: Vec<_> = cl.supers_of(c(i)).into_iter().collect();
            want.sort_unstable();
            assert_eq!(lens.supers_of(c(i)), want, "ancestor sets diverge at {i}");
            for j in 1..=8u32 {
                assert_eq!(
                    lens.entails(c(i), c(j)),
                    cl.entails(c(i), c(j)),
                    "entailment diverges at {i} -> {j}"
                );
            }
        }
    }

    /// Anti-vacuity for the test above: the fixture must contain both
    /// entailed and NOT-entailed pairs, or agreement proves nothing.
    #[test]
    fn the_agreement_fixture_discriminates() {
        let edges = [(1, 2), (2, 3), (3, 4), (5, 3), (6, 7)];
        let (_, src) = both(&edges);
        let lens = LensClosure::new(&src);
        assert!(lens.entails(c(1), c(4)), "derived through three hops");
        assert!(
            !lens.entails(c(1), c(7)),
            "must NOT entail a separate component"
        );
        assert!(!lens.entails(c(4), c(1)), "and must not run the wrong way");
        let reached: usize = (1..=8u32).map(|i| lens.supers_of(c(i)).len()).sum();
        assert!(
            reached > 0 && reached < 8 * 8,
            "fixture is neither empty nor total"
        );
    }

    #[test]
    fn reflexivity_holds_with_no_edges_at_all() {
        let src = Pairs(vec![]);
        let lens = LensClosure::new(&src);
        assert!(lens.entails(c(7), c(7)));
        assert!(lens.supers_of(c(7)).is_empty());
    }

    #[test]
    fn a_cycle_is_seen_and_a_clean_spine_is_not() {
        let cyc = Pairs(vec![(1, 2), (2, 1)]);
        let lens = LensClosure::new(&cyc);
        assert!(lens.reaches_itself(c(1)));
        assert!(lens.reaches_itself(c(2)));

        // can-stay-silent, over a NON-trivial acyclic graph
        let clean = Pairs(vec![(1, 2), (2, 3), (1, 3), (4, 3)]);
        let lens = LensClosure::new(&clean);
        for i in 1..=4u32 {
            assert!(!lens.reaches_itself(c(i)), "{i} is not on a cycle");
        }
    }

    #[test]
    fn the_depth_cap_binds_and_is_visible() {
        let chain: Vec<(u32, u32)> = (1..10).map(|i| (i, i + 1)).collect();
        let src = Pairs(chain);
        let shallow = LensClosure::new(&src).with_depth_cap(2);
        assert_eq!(shallow.depth_cap(), 2);
        assert!(!shallow.entails(c(1), c(10)), "beyond the cap");
        assert!(shallow.entails(c(1), c(3)), "within the cap");
        assert!(
            LensClosure::new(&src).entails(c(1), c(10)),
            "reachable at the default cap"
        );
    }

    #[test]
    fn intersect_is_a_single_merge_pass_and_can_return_nothing() {
        let a: Vec<ClassAddr> = [1u32, 3, 5, 7].iter().map(|i| c(*i)).collect();
        let b: Vec<ClassAddr> = [3u32, 4, 5, 9].iter().map(|i| c(*i)).collect();
        assert_eq!(intersect_sorted(&a, &b), vec![c(3), c(5)]);
        // anti-vacuity: disjoint inputs must yield empty, not everything
        let d: Vec<ClassAddr> = [2u32, 6].iter().map(|i| c(*i)).collect();
        assert!(intersect_sorted(&a, &d).is_empty());
        assert!(intersect_sorted(&a, &[]).is_empty());
    }

    /// The Schnittpunkt: two classes in DIFFERENT namespaces whose ancestries
    /// coincide at a shared address. This is the cross-ontology x-hop lookup,
    /// and it is an intersection — no vectors, no matrix product.
    #[test]
    fn meet_finds_where_two_ontologies_coincide() {
        let other = 0x0302_0000;
        let shared = ClassAddr::new(NS, 100);
        let f = move |x: ClassAddr, visit: &mut dyn FnMut(ClassAddr)| {
            // MONDO:1 -> MONDO:100 ; HP:1 -> MONDO:100 ; MONDO:2 -> MONDO:200
            if x == ClassAddr::new(NS, 1) || x == ClassAddr::new(other, 1) {
                visit(shared);
            }
            if x == ClassAddr::new(NS, 2) {
                visit(ClassAddr::new(NS, 200));
            }
        };
        let lens = LensClosure::new(&f);
        assert_eq!(
            lens.meet(&[ClassAddr::new(NS, 1), ClassAddr::new(other, 1)]),
            vec![shared],
            "the two ancestries meet at exactly one address"
        );
        // can-stay-silent: seeds that share nothing produce no Schnittpunkt
        assert!(
            lens.meet(&[ClassAddr::new(NS, 1), ClassAddr::new(NS, 2)])
                .is_empty(),
            "disjoint ancestries must not manufacture a meeting point"
        );
        assert!(lens.meet(&[]).is_empty(), "no seeds is not 'everything'");
    }

    #[test]
    fn ancestors_sorted_is_sorted_deduped_and_agrees_with_the_set() {
        let edges = [(1, 2), (2, 3), (1, 3), (3, 4)];
        let (cl, src) = both(&edges);
        let lens = LensClosure::new(&src);
        let v = lens.ancestors_sorted(c(1));
        assert!(v.windows(2).all(|w| w[0] < w[1]), "sorted and deduped");
        let mut want = cl.supers_of(c(1)).into_iter().collect::<Vec<_>>();
        want.sort_unstable();
        assert_eq!(v, want, "same content as the indexed closure");
    }

    /// A closure is a `Parents` source with no adapter — the property that
    /// keeps the seam dependency-free in both directions.
    #[test]
    fn a_bare_closure_is_a_parent_source() {
        let f = |x: ClassAddr, visit: &mut dyn FnMut(ClassAddr)| {
            if x == c(1) {
                visit(c(2));
            }
            if x == c(2) {
                visit(c(3));
            }
        };
        let lens = LensClosure::new(&f);
        assert!(lens.entails(c(1), c(3)));
        assert!(!lens.entails(c(3), c(1)));
    }

    /// `entails` must not walk further than it has to. The source counts its
    /// own calls, so "early exit" is measured rather than asserted.
    #[test]
    fn entails_early_exits_instead_of_building_the_whole_set() {
        use std::cell::Cell;
        let calls = Cell::new(0usize);
        let chain: Vec<(u32, u32)> = (1..40).map(|i| (i, i + 1)).collect();
        let counting = |x: ClassAddr, visit: &mut dyn FnMut(ClassAddr)| {
            calls.set(calls.get() + 1);
            for (a, b) in &chain {
                if c(*a) == x {
                    visit(c(*b));
                }
            }
        };
        let lens = LensClosure::new(&counting);

        assert!(lens.entails(c(1), c(3)));
        let near = calls.get();
        calls.set(0);
        let _ = lens.supers_of(c(1));
        let full = calls.get();
        assert!(
            near < full,
            "early exit must touch fewer classes than the full walk ({near} vs {full})"
        );
    }
}
