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

use crate::{Predicate, TermId, Triple};
use std::collections::HashMap;

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
fn ancestor_closure(
    adj: &HashMap<TermId, Vec<TermId>>,
) -> HashMap<TermId, Vec<TermId>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Namespace;

    fn t(sns: Namespace, s: u32, p: Predicate, ons: Namespace, o: u32) -> Triple {
        Triple {
            s: TermId { ns: sns as u8, num: s },
            p,
            o: TermId { ns: ons as u8, num: o },
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
            t(Namespace::Hpo, 9, Predicate::HasAnatomy, Namespace::Uberon, 100),
            t(Namespace::Uberon, 100, Predicate::PartOf, Namespace::Uberon, 200),
            t(Namespace::Uberon, 200, Predicate::IsA, Namespace::Uberon, 300),
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
}
