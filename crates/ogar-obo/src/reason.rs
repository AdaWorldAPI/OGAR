//! OWL-EL completion — the ELK subset the OBO EL profile actually exercises,
//! excavated onto plain Rust (no Java, no jar). OBO is deliberately OWL 2 EL,
//! so classification is **consequence-based saturation** over a small rule set;
//! the three rules that carry the OBO core:
//!
//! * **R⊑ (subsumption transitivity)** `A ⊑ B, B ⊑ C ⟹ A ⊑ C` — `is_a`
//!   classification.
//! * **R∘ (transitive role)** `A ⊑ ∃r.B, B ⊑ ∃r.C ⟹ A ⊑ ∃r.C` for a
//!   transitive `r` — `part_of` mereology on the anatomy backbone.
//! * **R∃ (existential + filler subsumption)** `A ⊑ ∃r.B, B ⊑ C ⟹ A ⊑ ∃r.C`
//!   — an HP grounded to a specific Uberon site is grounded to that site's
//!   `is_a`/`part_of` ancestors too. This is the deductive form of "inherit
//!   the anatomy grounding up" (the Tier-1 completion, now a proof not a prior).
//!
//! Unsatisfiability (`A ⊑ ⊥`) needs disjointness/`owl:Nothing` axioms that the
//! base `.obo` does not carry, so [`ElStats::unsatisfiable`] is reported `0`
//! with that caveat — closing it is the `hp-base.owl` / disjointness pass.

use crate::{Namespace, Predicate, TermId, Triple};
use std::collections::{HashMap, HashSet};

/// Count `is_a` cycles (strongly-connected components of size > 1) — must be 0
/// on a clean OBO core. Iterative Tarjan so a deep chain can't blow the stack.
#[must_use]
pub fn count_is_a_cycles(triples: &[Triple]) -> usize {
    // child -> parents adjacency over is_a only
    let mut adj: HashMap<TermId, Vec<TermId>> = HashMap::new();
    let mut nodes: Vec<TermId> = Vec::new();
    for t in triples {
        if t.p == Predicate::IsA {
            adj.entry(t.s).or_default().push(t.o);
            nodes.push(t.s);
            nodes.push(t.o);
        }
    }
    nodes.sort_unstable();
    nodes.dedup();

    let mut index: HashMap<TermId, u32> = HashMap::new();
    let mut low: HashMap<TermId, u32> = HashMap::new();
    let mut on_stack: HashMap<TermId, bool> = HashMap::new();
    let mut stack: Vec<TermId> = Vec::new();
    let mut idx: u32 = 0;
    let mut cycles = 0usize;

    // explicit DFS frames: (node, next-child-cursor)
    let empty: Vec<TermId> = Vec::new();
    for &root in &nodes {
        if index.contains_key(&root) {
            continue;
        }
        let mut call: Vec<(TermId, usize)> = vec![(root, 0)];
        while let Some(&(v, ci)) = call.last() {
            if ci == 0 {
                index.insert(v, idx);
                low.insert(v, idx);
                idx += 1;
                stack.push(v);
                on_stack.insert(v, true);
            }
            let children = adj.get(&v).unwrap_or(&empty);
            if ci < children.len() {
                let w = children[ci];
                call.last_mut().unwrap().1 += 1;
                if !index.contains_key(&w) {
                    call.push((w, 0));
                } else if *on_stack.get(&w).unwrap_or(&false) {
                    let lw = index[&w];
                    let lv = low[&v];
                    low.insert(v, lv.min(lw));
                }
            } else {
                // done with v — pop SCC if root
                if low[&v] == index[&v] {
                    let mut sz = 0;
                    while let Some(w) = stack.pop() {
                        on_stack.insert(w, false);
                        sz += 1;
                        if w == v {
                            break;
                        }
                    }
                    if sz > 1 {
                        cycles += 1;
                    }
                }
                call.pop();
                if let Some(&(parent, _)) = call.last() {
                    let lp = low[&parent];
                    let lv = low[&v];
                    low.insert(parent, lp.min(lv));
                }
            }
        }
    }
    cycles
}

/// The aggregate EL-saturation counts over the OBO core.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ElStats {
    /// transitive `is_a` pairs (the classification closure size)
    pub subsumption_pairs: usize,
    /// transitive `part_of` pairs (mereology closure size)
    pub part_of_pairs: usize,
    /// existential edges inferred by R∃ *beyond* the asserted ones — the
    /// "grounding inherited up the anatomy `is_a`/`part_of` spine" count
    pub existential_inferred: usize,
    /// classes proven unsatisfiable (`A ⊑ ⊥`) — 0 without disjointness axioms
    /// (base `.obo` carries none; see the module note)
    pub unsatisfiable: usize,
}

/// Topological ancestor DP over a single relation's `child -> parents` graph.
/// Returns, per node, the sorted deduped set of all transitive ancestors.
/// Assumes acyclic (run [`count_is_a_cycles`] first); a residual cycle is
/// simply not expanded past a re-visit, never loops.
fn ancestor_closure(adj: &HashMap<TermId, Vec<TermId>>) -> HashMap<TermId, Vec<TermId>> {
    // Kahn order over the parent graph, then DP bottom-up.
    // Build reverse (parent -> children) for indegree over child->parent edges.
    let mut all: Vec<TermId> = Vec::new();
    for (c, ps) in adj {
        all.push(*c);
        all.extend(ps.iter().copied());
    }
    all.sort_unstable();
    all.dedup();

    // process nodes in an order where all parents precede a child:
    // repeatedly resolve nodes whose parents are all resolved (memoized).
    let mut memo: HashMap<TermId, Vec<TermId>> = HashMap::new();
    let empty: Vec<TermId> = Vec::new();
    // iterative post-order DFS to fill memo
    for &start in &all {
        if memo.contains_key(&start) {
            continue;
        }
        let mut stack: Vec<(TermId, usize)> = vec![(start, 0)];
        while let Some(&(v, ci)) = stack.last() {
            let parents = adj.get(&v).unwrap_or(&empty);
            if ci < parents.len() {
                let p = parents[ci];
                stack.last_mut().unwrap().1 += 1;
                if !memo.contains_key(&p) && !stack.iter().any(|(x, _)| *x == p) {
                    stack.push((p, 0));
                }
            } else {
                if !memo.contains_key(&v) {
                    let mut anc: Vec<TermId> = Vec::new();
                    for &p in adj.get(&v).unwrap_or(&empty) {
                        anc.push(p);
                        if let Some(pa) = memo.get(&p) {
                            anc.extend(pa.iter().copied());
                        }
                    }
                    anc.sort_unstable();
                    anc.dedup();
                    memo.insert(v, anc);
                }
                stack.pop();
            }
        }
    }
    memo
}

/// Run the EL completion subset over the triples and return the aggregate
/// counts. `is_a` and `part_of` closures via [`ancestor_closure`]; R∃
/// propagates every existential filler up the combined `is_a`+`part_of` spine.
#[must_use]
pub fn saturate(triples: &[Triple]) -> ElStats {
    let mut isa: HashMap<TermId, Vec<TermId>> = HashMap::new();
    let mut partof: HashMap<TermId, Vec<TermId>> = HashMap::new();
    // spine = is_a ∪ part_of (both propagate anatomy grounding upward)
    let mut spine: HashMap<TermId, Vec<TermId>> = HashMap::new();
    let mut existential: Vec<(TermId, Predicate, TermId)> = Vec::new();

    for t in triples {
        match t.p {
            Predicate::IsA => {
                isa.entry(t.s).or_default().push(t.o);
                spine.entry(t.s).or_default().push(t.o);
            }
            Predicate::PartOf => {
                partof.entry(t.s).or_default().push(t.o);
                spine.entry(t.s).or_default().push(t.o);
                existential.push((t.s, t.p, t.o));
            }
            Predicate::HasAnatomy
            | Predicate::HasQuality
            | Predicate::HasLocation
            | Predicate::HasPhenotype => {
                existential.push((t.s, t.p, t.o));
            }
            Predicate::Other => {}
        }
    }

    let isa_c = ancestor_closure(&isa);
    let partof_c = ancestor_closure(&partof);
    let spine_c = ancestor_closure(&spine);

    let subsumption_pairs: usize = isa_c.values().map(Vec::len).sum();
    let part_of_pairs: usize = partof_c.values().map(Vec::len).sum();
    // R∃: each existential (s,r,o) also holds for every ancestor of o on the
    // is_a∪part_of spine — the inferred-beyond-asserted count.
    let empty: Vec<TermId> = Vec::new();
    let existential_inferred: usize = existential
        .iter()
        .map(|(_, _, o)| spine_c.get(o).unwrap_or(&empty).len())
        .sum();

    ElStats {
        subsumption_pairs,
        part_of_pairs,
        existential_inferred,
        unsatisfiable: 0,
    }
}

// ── per-term reasoning walk, generic over its edge source ──────────────────
//
// `saturate` gives the aggregate EL closure; a consumer resolving ONE entity
// (a disease → its anatomy site + phenotypes) needs the per-term walk. The
// walk applies the same is_a-saturation + existential-propagation rules to a
// single subject — and it is generic over WHERE the asserted edges come from,
// so the same rules run over the pre-bake parse map at join time and over the
// baked lanes at read time, without a consumer ever rebuilding the map from
// rows it already holds (the 2026-08-13 crosswalk audit's headline finding).

/// Where the per-term walk reads a subject's **asserted** edges from.
///
/// **SoA lenses only — never a map.** The operator ruling (2026-08-13): a
/// JOIN exists only **cross-domain** (a crosswalk resolving a foreign code
/// into this domain's address space, upstream of the bake); **inner-domain
/// edges live in the live SoA substrate** and are read there, in place. The
/// pre-bake parse map ([`crate::parse_obo`]'s output) is bake input and does
/// NOT implement this trait — a reasoning path holding a parsed map beside
/// baked rows is the second-projection the 2026-08-13 crosswalk audit
/// measured, and it is structurally excluded here rather than discouraged.
///
/// Shipped sources:
///
/// * [`SpineSource`] — a borrowed lens over one baked slab, decoding edge
///   lanes in place;
/// * [`Stacked`] — **SoA lens stacking**: several borrowed sources layered
///   into one, for the deployment shape where a domain's edge families live
///   in more than one slab (crystal spine + a sidecar SoA of cross-angle
///   lanes). When the primary slab cannot carry an edge family, the answer
///   is a **sidecar SoA under the same LE contract, stacked** — never a
///   hand-rolled exception container.
///
/// Both visitors must yield a deterministic order for a given source (the
/// walk's output order is defined by it).
pub trait EdgeSource {
    /// Visit the asserted `is_a` parents of `id`. Unknown ids yield nothing,
    /// silently — an edge into an unloaded ontology is a boundary, not a fault.
    fn is_a(&self, id: TermId, visit: &mut dyn FnMut(TermId));
    /// Visit the asserted typed relations `(predicate, object)` of `id`
    /// (everything except `is_a`).
    fn rel(&self, id: TermId, visit: &mut dyn FnMut(Predicate, TermId));
}

/// **SoA lens stacking** — several borrowed [`EdgeSource`] layers read as one.
///
/// Layers are visited in the given order, so the stack's determinism is the
/// layers' determinism plus their order. Duplicate suppression is the walk's
/// job (its `seen` set), not the stack's — stacking stays a pure read.
///
/// This is the sanctioned shape for "the spine slab has `is_a`, the sidecar
/// slab has the cross-angle relations": two lenses, one stack, zero rebuild.
#[derive(Clone, Copy)]
pub struct Stacked<'a> {
    layers: &'a [&'a dyn EdgeSource],
}

impl<'a> Stacked<'a> {
    /// Stack borrowed layers, visited in order.
    #[must_use]
    pub const fn new(layers: &'a [&'a dyn EdgeSource]) -> Self {
        Self { layers }
    }
}

impl core::fmt::Debug for Stacked<'_> {
    /// Prints the layer COUNT, never the layers (a layer can lens megabytes).
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Stacked")
            .field("layers", &self.layers.len())
            .finish()
    }
}

impl EdgeSource for Stacked<'_> {
    fn is_a(&self, id: TermId, visit: &mut dyn FnMut(TermId)) {
        for l in self.layers {
            l.is_a(id, visit);
        }
    }

    fn rel(&self, id: TermId, visit: &mut dyn FnMut(Predicate, TermId)) {
        for l in self.layers {
            l.rel(id, visit);
        }
    }
}

/// [`crate::spine::SpineLens`] lifted into [`TermId`] space — the baked-lane
/// [`EdgeSource`].
///
/// Address translation is derived, never re-arithmetic'd: `TermId → classid`
/// through [`Namespace::render_classid`] under the bake's `app_prefix`, and
/// `classid → TermId` through [`Namespace::from_concept_id`] on the hi-u16.
/// A link into a concept outside the core five (or into a row not in view) is
/// dropped silently — the same boundary rule the lens itself follows.
#[derive(Clone, Copy, Debug)]
pub struct SpineSource<'a> {
    lens: crate::spine::SpineLens<'a>,
    app_prefix: u16,
}

impl<'a> SpineSource<'a> {
    /// Borrow a lens over rows baked under `app_prefix`.
    #[must_use]
    pub const fn new(lens: crate::spine::SpineLens<'a>, app_prefix: u16) -> Self {
        Self { lens, app_prefix }
    }

    /// `(classid, num)` → [`TermId`], via the namespace's own concept-id
    /// inverse. `None` outside the OBO core.
    fn term_of(classid: u32, num: u32) -> Option<TermId> {
        let ns = Namespace::from_concept_id((classid >> 16) as u16)?;
        Some(TermId { ns: ns as u8, num })
    }
}

impl EdgeSource for SpineSource<'_> {
    fn is_a(&self, id: TermId, visit: &mut dyn FnMut(TermId)) {
        let classid = id.namespace().render_classid(self.app_prefix);
        self.lens.parents_of(classid, id.num, &mut |c, n| {
            if let Some(t) = Self::term_of(c, n) {
                visit(t);
            }
        });
    }

    fn rel(&self, id: TermId, visit: &mut dyn FnMut(Predicate, TermId)) {
        let classid = id.namespace().render_classid(self.app_prefix);
        if let Some(i) = self.lens.resolve(classid, id.num) {
            self.lens.rel_at(i, &mut |p, c, n| {
                if let Some(t) = Self::term_of(c, n) {
                    visit(p, t);
                }
            });
        }
    }
}

/// Transitive `is_a` ancestry of `id` over any [`EdgeSource`] (the term itself
/// excluded), deduped, in deterministic order, depth-capped. The start id is
/// pre-marked seen, so a cyclic `is_a` (incl. a self-parent) can never
/// re-emit the query term.
///
/// The returned `Vec` is the caller's transient answer, never a cache — an
/// ancestor is the same KIND as a parent (a member, not a fact about the
/// set), so the closure is a projection under the zero-copy law: asking twice
/// walks twice, deliberately.
#[must_use]
pub fn ancestors<S: EdgeSource + ?Sized>(src: &S, id: TermId) -> Vec<TermId> {
    let mut seen = HashSet::new();
    seen.insert(id);
    let mut out = Vec::new();
    let mut frontier = vec![id];
    for _ in 0..64 {
        frontier.sort_unstable();
        let mut next = Vec::new();
        for f in frontier.drain(..) {
            src.is_a(f, &mut |p| {
                if seen.insert(p) {
                    out.push(p);
                    next.push(p);
                }
            });
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    out
}

/// Every `pred` target reachable from `id` OR any of its `is_a` ancestors — the
/// existential-role propagation up the subsumption spine, for a single subject.
/// Deduped, deterministic (term-local edges first, then up the ancestry).
#[must_use]
pub fn related_via_ancestry<S: EdgeSource + ?Sized>(
    src: &S,
    id: TermId,
    pred: Predicate,
) -> Vec<TermId> {
    let mut chain = vec![id];
    chain.extend(ancestors(src, id));
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for c in chain {
        src.rel(c, &mut |p, obj| {
            if p == pred && seen.insert(obj) {
                out.push(obj);
            }
        });
    }
    out
}

/// The term's anatomy sites (`disease_has_location` → Uberon, classified by the
/// Mondo→Uberon namespace pair), propagated up the `is_a` ancestry.
#[must_use]
pub fn anatomy_of<S: EdgeSource + ?Sized>(src: &S, id: TermId) -> Vec<TermId> {
    related_via_ancestry(src, id, Predicate::HasLocation)
        .into_iter()
        .filter(|t| t.namespace() == Namespace::Uberon)
        .collect()
}

/// The term's phenotypes (`has_phenotype` / `disease_has_feature` → HPO,
/// classified by the Mondo→Hpo namespace pair), propagated up the ancestry.
#[must_use]
pub fn phenotypes_of<S: EdgeSource + ?Sized>(src: &S, id: TermId) -> Vec<TermId> {
    related_via_ancestry(src, id, Predicate::HasPhenotype)
        .into_iter()
        .filter(|t| t.namespace() == Namespace::Hpo)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(sns: Namespace, s: u32, p: Predicate, ons: Namespace, o: u32) -> Triple {
        Triple {
            s: TermId {
                ns: sns as u8,
                num: s,
            },
            p,
            o: TermId {
                ns: ons as u8,
                num: o,
            },
        }
    }

    #[test]
    fn no_cycle_on_a_chain() {
        // A is_a B is_a C
        let tr = vec![
            t(Namespace::Mondo, 1, Predicate::IsA, Namespace::Mondo, 2),
            t(Namespace::Mondo, 2, Predicate::IsA, Namespace::Mondo, 3),
        ];
        assert_eq!(count_is_a_cycles(&tr), 0);
    }

    #[test]
    fn detects_a_cycle() {
        // A is_a B is_a A  (a real ontology bug)
        let tr = vec![
            t(Namespace::Mondo, 1, Predicate::IsA, Namespace::Mondo, 2),
            t(Namespace::Mondo, 2, Predicate::IsA, Namespace::Mondo, 1),
        ];
        assert_eq!(count_is_a_cycles(&tr), 1);
    }

    #[test]
    fn r_subsumption_transitive_closure() {
        // A⊑B⊑C  ⟹ ancestors: A={B,C}, B={C} = 3 pairs
        let tr = vec![
            t(Namespace::Mondo, 1, Predicate::IsA, Namespace::Mondo, 2),
            t(Namespace::Mondo, 2, Predicate::IsA, Namespace::Mondo, 3),
        ];
        assert_eq!(saturate(&tr).subsumption_pairs, 3);
    }

    #[test]
    fn r_existential_grounding_inherited_up() {
        // The load-bearing EL inference:
        //   HP:9 has_anatomy UBERON:100         (a specific site)
        //   UBERON:100 part_of UBERON:200       (site is part of a bigger site)
        //   UBERON:200 is_a    UBERON:300       (which is a kind of ...)
        // ⟹ HP:9 is grounded to UBERON:{200,300} too (2 inferred-beyond-asserted).
        let tr = vec![
            t(
                Namespace::Hpo,
                9,
                Predicate::HasAnatomy,
                Namespace::Uberon,
                100,
            ),
            t(
                Namespace::Uberon,
                100,
                Predicate::PartOf,
                Namespace::Uberon,
                200,
            ),
            t(
                Namespace::Uberon,
                200,
                Predicate::IsA,
                Namespace::Uberon,
                300,
            ),
        ];
        let s = saturate(&tr);
        // R∃ fires on BOTH existentials: HP:9→{200,300} (2) AND the part_of
        // edge 100→200 chains through 200 is_a 300 to give 100 part_of 300 (1).
        // 3 inferred-beyond-asserted total — the part_of∘is_a chaining is a real
        // OBO inference, not an artifact.
        assert_eq!(
            s.existential_inferred, 3,
            "HP:9 grounds to UBERON:{{200,300}} and 100 part_of 300 chains"
        );
    }

    /// The per-term walk over the BAKED LANES — parse feeds the bake, the walk
    /// reads the rows. The parse map itself is not a source (inner-domain is
    /// the live SoA substrate; the map is join-stage input only).
    #[test]
    fn per_term_walk_resolves_ancestry_anatomy_phenotype_over_the_baked_lanes() {
        // A disease with its anatomy edge on an is_a PARENT — the walk must
        // inherit it up the subsumption spine.
        let obo = "[Term]\n\
id: MONDO:0005249\n\
name: pneumonia\n\
is_a: MONDO:0000270 ! lower respiratory tract disorder\n\
relationship: disease_has_feature HP:0012735 ! Cough\n\
\n\
[Term]\n\
id: MONDO:0000270\n\
name: lower respiratory tract disorder\n\
relationship: disease_has_location UBERON:0001558 ! lower respiratory tract\n\
\n\
[Term]\n\
id: HP:0012735\n\
name: Cough\n\
\n\
[Term]\n\
id: UBERON:0001558\n\
name: lower respiratory tract\n";
        let baked = crate::bake(&crate::parse_obo(obo), 0x0000);
        let lens = crate::spine::SpineLens::new(&baked.rows);
        assert!(lens.is_sorted(), "the bake's own order carries the lens");
        let src = SpineSource::new(lens, 0x0000);
        let pneu = TermId::parse("MONDO:0005249").unwrap();
        assert!(
            ancestors(&src, pneu).contains(&TermId::parse("MONDO:0000270").unwrap()),
            "is_a ancestry"
        );
        assert!(
            anatomy_of(&src, pneu).contains(&TermId::parse("UBERON:0001558").unwrap()),
            "disease_has_location → Uberon, inherited from the is_a parent"
        );
        assert!(
            phenotypes_of(&src, pneu).contains(&TermId::parse("HP:0012735").unwrap()),
            "disease_has_feature → HPO (own edge)"
        );
        // self excluded from its own ancestry (cycle-safe)
        assert!(!ancestors(&src, pneu).contains(&pneu));
    }

    /// SoA lens STACKING: the subsumption spine in one slab, the cross-angle
    /// relations in a SIDECAR slab — the stack answers what neither slab can
    /// alone (the anatomy edge sits on the parent, and the parent link sits in
    /// the OTHER slab). This is the deployment shape the trait exists for.
    #[test]
    fn a_sidecar_slab_stacks_onto_the_spine_without_any_rebuild() {
        // slab 1: the is_a spine only
        let spine_obo = "[Term]\n\
id: MONDO:0005249\n\
name: pneumonia\n\
is_a: MONDO:0000270 ! lower respiratory tract disorder\n\
\n\
[Term]\n\
id: MONDO:0000270\n\
name: lower respiratory tract disorder\n";
        // slab 2 (sidecar): the cross-angle relation only
        let sidecar_obo = "[Term]\n\
id: MONDO:0000270\n\
name: lower respiratory tract disorder\n\
relationship: disease_has_location UBERON:0001558 ! lower respiratory tract\n\
\n\
[Term]\n\
id: UBERON:0001558\n\
name: lower respiratory tract\n";
        let spine_bake = crate::bake(&crate::parse_obo(spine_obo), 0x0000);
        let sidecar_bake = crate::bake(&crate::parse_obo(sidecar_obo), 0x0000);
        let spine = SpineSource::new(crate::spine::SpineLens::new(&spine_bake.rows), 0x0000);
        let sidecar = SpineSource::new(crate::spine::SpineLens::new(&sidecar_bake.rows), 0x0000);
        let layers: [&dyn EdgeSource; 2] = [&spine, &sidecar];
        let stack = Stacked::new(&layers);

        let pneu = TermId::parse("MONDO:0005249").unwrap();
        let site = TermId::parse("UBERON:0001558").unwrap();
        // Neither slab alone can resolve the anatomy…
        assert!(
            anatomy_of(&spine, pneu).is_empty(),
            "the spine slab carries no relation"
        );
        assert!(
            anatomy_of(&sidecar, pneu).is_empty(),
            "the sidecar slab carries no ancestry into the relation"
        );
        // …the STACK can: ancestry from slab 1, relation from slab 2.
        assert!(
            anatomy_of(&stack, pneu).contains(&site),
            "stacked lenses compose ancestry × cross-angle"
        );
    }
}
