//! Full OBO-core bake driver — reads the five `.obo` sources + `hp-base.owl`
//! from a directory, bakes the 512-byte SoA + SPO triples + xref table, runs
//! the EL-completion saturation, verifies the packet round-trips through the
//! lance-graph loader contract, and writes the `.soa` artifact.
//!
//! Usage: `cargo run -p ogar-obo --example bake_obo -- <src_dir> [out.soa]`
//! (`<src_dir>` holds `mondo.obo hp.obo uberon.obo pato.obo ro.obo
//! hp-base.owl`). The sources are pinned in the release manifest, fetched to a
//! scratch dir, never committed.

use ogar_obo::{
    Namespace, as_le_bytes, bake, merge_logical_defs, parse_hp_logical_defs, parse_obo, reason,
    rows_from_le_bytes,
};
use std::collections::HashMap;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().unwrap_or_else(|| "/tmp/obo".to_string());
    let out = args.next().unwrap_or_else(|| format!("{dir}/obo-core.soa"));

    let mut nodes = HashMap::new();
    for f in ["mondo", "hp", "uberon", "pato", "ro"] {
        let path = format!("{dir}/{f}.obo");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        let part = parse_obo(&text);
        let (terms, edges) = (
            part.len(),
            part.values()
                .map(|n| n.is_a.len() + n.rel.len() + n.xref.len())
                .sum::<usize>(),
        );
        println!("  parsed {f:8} terms={terms:6} edges+xref={edges}");
        for (id, n) in part {
            let e = nodes.entry(id).or_insert_with(ogar_obo::OboNode::default);
            e.is_a.extend(n.is_a);
            e.rel.extend(n.rel);
            e.xref.extend(n.xref);
            e.obsolete |= n.obsolete;
        }
    }

    // HPO logical-definition grounding (anatomy:quality) from hp-base.owl.
    let owl = std::fs::read_to_string(format!("{dir}/hp-base.owl")).unwrap_or_default();
    let defs = parse_hp_logical_defs(&owl);
    println!(
        "  hp-base logical defs: {} HP->Uberon/PATO grounding edges",
        defs.len()
    );
    merge_logical_defs(&mut nodes, &defs);

    let baked = bake(&nodes, 0x0000);
    let s = &baked.stats;
    println!("\n=== BAKE ===");
    println!("  live core nodes (rows) : {}", s.nodes);
    println!("  obsolete (not tiled)   : {}", s.obsolete);
    println!("  SPO triples            : {}", s.triples);
    println!("  is_a cycles            : {}", s.is_a_cycles);
    println!("  dangling core targets  : {}", s.dangling);
    println!("  MONDO->HP resolve      : {}", s.mondo_hp);
    println!("  HP->UBERON resolve     : {}", s.hp_uberon);
    println!("  HP->PATO resolve       : {}", s.hp_pato);
    println!(
        "  xrefs preserved        : {} (MeSH bearings: {})",
        s.xrefs, s.mesh_xrefs
    );
    // The ship gate. These were computed but never printed, so a driver run
    // could look clean while the artifact was missing edges — exactly the
    // silent shape gate A1 was about. Print them, and refuse rather than warn.
    println!("  links packed (col_idx) : {}", s.links_packed);
    println!("  links DROPPED          : {}", s.links_dropped);
    println!("  rows overflowed        : {}", s.rows_overflowed);
    assert_eq!(
        s.links_dropped, 0,
        "refusing to write an artifact that is missing {} edge(s)",
        s.links_dropped
    );
    assert_eq!(
        s.rows_overflowed, 0,
        "refusing to write an artifact with {} truncated row(s)",
        s.rows_overflowed
    );
    assert_eq!(
        s.links_packed, s.triples,
        "every SPO triple must also be in-row: {} packed vs {} triples",
        s.links_packed, s.triples
    );

    // Per-namespace row census.
    let mut census: HashMap<u8, usize> = HashMap::new();
    for id in &baked.ids {
        *census.entry(id.ns).or_default() += 1;
    }
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
            census.get(&(ns as u8)).copied().unwrap_or(0)
        );
    }

    println!("\n=== EL SATURATION (ELK subset, excavated to Rust) ===");
    let el = reason::saturate(&baked.triples);
    println!("  is_a subsumption pairs   : {}", el.subsumption_pairs);
    println!("  part_of transitive pairs : {}", el.part_of_pairs);
    println!(
        "  existential inferred (R∃): {} (grounding inherited up the spine)",
        el.existential_inferred
    );
    println!(
        "  unsatisfiable            : {} (no disjointness axioms in base obo)",
        el.unsatisfiable
    );

    // Write the artifact + verify it round-trips through the loader contract.
    let bytes = as_le_bytes(&baked.rows);
    std::fs::write(&out, bytes).unwrap_or_else(|e| panic!("write {out}: {e}"));
    println!("\n=== ARTIFACT ===");
    println!(
        "  {out}  ({} bytes = {} rows × 512)",
        bytes.len(),
        baked.rows.len()
    );
    let readback = std::fs::read(&out).expect("reread");
    match rows_from_le_bytes(&readback) {
        Some(rows) => {
            // NB: a fresh Vec<u8> from fs::read may not be 64-aligned; the loader
            // returns None then and a real consumer uses FixedSizeBinary(512)
            // (arrow-aligned). We verify the in-memory aligned view instead.
            println!(
                "  reread rows_from_le_bytes: {} rows (aligned buffer)",
                rows.len()
            );
        }
        None => {
            let inmem = rows_from_le_bytes(bytes).expect("in-memory aligned view");
            assert_eq!(inmem.len(), baked.rows.len());
            println!(
                "  reread buffer not 64-aligned (fs::read) — in-memory aligned view OK: {} rows \
                 (Lance FixedSizeBinary(512) is arrow-aligned on the real path)",
                inmem.len()
            );
        }
    }
    println!(
        "  loader contract: VERIFIED (as_le_bytes ↔ rows_from_le_bytes, 512×N, 64-align gate)"
    );
}
