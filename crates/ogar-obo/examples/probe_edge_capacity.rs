//! Probe: does the `col_idx` half of the CSR fit in the free value lanes?
//!
//! The bake persists `row_ptr` (the per-predicate degree histogram in the edge
//! block) but not `col_idx` (the targets). Packing the targets as G2 `4×u24`
//! literal links needs a fixed layout constant — how many lanes to reserve.
//! This probe measures the demand on the real sources before that constant is
//! chosen, and checks the two assumptions the packing rests on:
//!
//!   A1  CURIE numeric `0` never occurs (so `0` is a safe in-lane pad).
//!   A2  every numeric fits 24 bits (`TermId::parse` already rejects >24-bit,
//!       so this re-checks the *baked* ids, not the parser).
//!
//! Usage: `cargo run -p ogar-obo --example probe_edge_capacity -- [src_dir]`

use ogar_obo::{Namespace, Predicate, bake, merge_logical_defs, parse_hp_logical_defs, parse_obo};
use std::collections::HashMap;

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/obo".to_string());

    let mut nodes = HashMap::new();
    for f in ["mondo", "hp", "uberon", "pato", "ro"] {
        let path = format!("{dir}/{f}.obo");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        for (id, n) in parse_obo(&text) {
            let e = nodes.entry(id).or_insert_with(ogar_obo::OboNode::default);
            e.is_a.extend(n.is_a);
            e.rel.extend(n.rel);
            e.xref.extend(n.xref);
            e.obsolete |= n.obsolete;
        }
    }
    let owl = std::fs::read_to_string(format!("{dir}/hp-base.owl")).unwrap_or_default();
    merge_logical_defs(&mut nodes, &parse_hp_logical_defs(&owl));

    let baked = bake(&nodes, 0x0000);

    // ── group the triples by subject, in the canonical emit order ────────────
    // ascending predicate, then ascending target namespace: a lane carries ONE
    // target ontology (its classid names it), so a namespace switch inside a
    // predicate starts a new lane. That fragmentation is what makes the lane
    // demand exceed `ceil(degree / 4)` — which is exactly what we are measuring.
    let mut by_subject: HashMap<ogar_obo::TermId, Vec<(u8, u8, u32)>> = HashMap::new();
    for t in &baked.triples {
        by_subject
            .entry(t.s)
            .or_default()
            .push((t.p as u8, t.o.ns, t.o.num));
    }

    let mut max_deg = 0usize;
    let mut max_lanes = 0usize;
    let mut worst_deg: Option<ogar_obo::TermId> = None;
    let mut worst_lanes: Option<ogar_obo::TermId> = None;
    let mut lane_hist: HashMap<usize, usize> = HashMap::new();
    let mut total_lanes = 0usize;
    let mut zero_numeric = 0usize;
    let mut over_24bit = 0usize;

    for (s, edges) in &mut by_subject {
        edges.sort_unstable();
        // lane demand: a new lane per 4 targets, restarting on (pred, ns) change
        let mut lanes = 0usize;
        let mut slot = 4usize; // force a fresh lane on the first target
        let mut group: Option<(u8, u8)> = None;
        for &(p, ns, num) in edges.iter() {
            if num == 0 {
                zero_numeric += 1;
            }
            if num > 0x00FF_FFFF {
                over_24bit += 1;
            }
            if group != Some((p, ns)) || slot == 4 {
                if group != Some((p, ns)) {
                    group = Some((p, ns));
                }
                lanes += 1;
                slot = 0;
            }
            slot += 1;
        }
        if edges.len() > max_deg {
            max_deg = edges.len();
            worst_deg = Some(*s);
        }
        if lanes > max_lanes {
            max_lanes = lanes;
            worst_lanes = Some(*s);
        }
        *lane_hist.entry(lanes).or_default() += 1;
        total_lanes += lanes;
    }

    println!("=== EDGE CAPACITY PROBE ===");
    println!("  subjects with >=1 edge : {}", by_subject.len());
    println!("  total triples          : {}", baked.triples.len());
    println!("  total lanes demanded   : {total_lanes}");
    println!("  max out-degree         : {max_deg}  ({worst_deg:?})");
    println!("  max lanes for one row  : {max_lanes}  ({worst_lanes:?})");

    println!("\n  A1 CURIE numeric == 0  : {zero_numeric} occurrences (must be 0)");
    println!("  A2 numeric > 24 bit    : {over_24bit} occurrences (must be 0)");

    // How many rows would overflow a given lane budget?
    println!("\n  rows exceeding a lane budget:");
    for budget in [8usize, 12, 16, 20, 23, 29] {
        let over: usize = lane_hist
            .iter()
            .filter(|(l, _)| **l > budget)
            .map(|(_, c)| *c)
            .sum();
        println!(
            "    budget {budget:2} lanes ({:3} slots): {over} rows over",
            budget * 4
        );
    }

    // Per-namespace is_a degree totals — the falsifier against the baked slabs.
    let mut is_a_by_ns: HashMap<u8, usize> = HashMap::new();
    for t in &baked.triples {
        if t.p == Predicate::IsA {
            *is_a_by_ns.entry(t.s.ns).or_default() += 1;
        }
    }
    println!("\n  is_a degree totals by subject namespace (falsifier vs baked slabs):");
    for ns in [
        Namespace::Mondo,
        Namespace::Hpo,
        Namespace::Uberon,
        Namespace::Pato,
        Namespace::Ro,
    ] {
        println!(
            "    {:8}: {}",
            ns.prefix(),
            is_a_by_ns.get(&(ns as u8)).copied().unwrap_or(0)
        );
    }
}
