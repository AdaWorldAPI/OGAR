//! Bake the meta-study spine — the leg gate A1 was blocking.
//!
//! ```text
//! cargo run -p ogar-obo --features rdf --example bake_spine -- <dir> [out.soa]
//! ```
//!
//! Reads one RDF document per namespace (`<PREFIX>.<ext>`), extracts through
//! `ogar_obo::rdf` (assertions retained, identity from the CURIE numeric), and
//! bakes through `bake_with` against
//! [`ogar_obo::registry::META_STUDY_SPINE`] — no code path per ontology, only
//! table rows.
//!
//! Three of these namespaces carry a real term at numeric `0`. Before the A1
//! `+1` bias those links encoded to the lane pad and truncated their whole
//! lane silently, so this bake could not be trusted. The run asserts the ship
//! gate (`links_dropped == 0`, `rows_overflowed == 0`,
//! `links_packed == triples`) and refuses to write otherwise.

use std::collections::HashMap;
use std::path::Path;

use ogar_obo::rdf::{self, OBO_RESTRICTION_PREDICATES};
use ogar_obo::registry::META_STUDY_SPINE;
use ogar_obo::{OboNode, Predicate, TermId, as_le_bytes, bake_with, rows_from_le_bytes};

fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().unwrap_or_else(|| "/tmp/onto".to_string());
    let out = a.next().unwrap_or_else(|| format!("{dir}/spine.soa"));
    let reg = &META_STUDY_SPINE;

    let mut nodes: HashMap<TermId, OboNode> = HashMap::new();
    let mut missing: Vec<&str> = Vec::new();
    for spec in reg.specs() {
        let Some((path, bytes)) = read_named(Path::new(&dir), spec.prefix) else {
            missing.push(spec.prefix);
            continue;
        };
        let fmt = rdf::detect_format(
            path.extension().and_then(|e| e.to_str()),
            &bytes[..bytes.len().min(256)],
        );
        let part = rdf::parse_rdf(&bytes, fmt, reg, OBO_RESTRICTION_PREDICATES)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        println!("  parsed {:6} classes={}", spec.prefix, part.len());
        for (id, n) in part {
            let e = nodes.entry(id).or_default();
            e.is_a.extend(n.is_a);
            e.rel.extend(n.rel);
            e.xref.extend(n.xref);
            e.obsolete |= n.obsolete;
        }
    }
    assert!(
        missing.is_empty(),
        "refusing to bake a partial spine — not read: {missing:?}"
    );

    // The A1 evidence, measured on the real input rather than assumed.
    let zeros: Vec<&str> = reg
        .specs()
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            nodes.contains_key(&TermId {
                ns: *i as u8,
                num: 0,
            })
        })
        .map(|(_, s)| s.prefix)
        .collect();
    println!("\n  namespaces with a real term at numeric 0: {zeros:?}");
    let zero_in_edges: usize = nodes
        .values()
        .map(|n| {
            n.is_a.iter().filter(|t| t.num == 0).count()
                + n.rel.iter().filter(|(_, t)| t.num == 0).count()
        })
        .sum();
    println!(
        "  edges POINTING AT a numeric-0 term      : {zero_in_edges} \
         (each one truncated its lane before A1)"
    );

    let baked = bake_with(&nodes, 0x0000, reg);
    let s = &baked.stats;
    println!("\n=== BAKE ===");
    println!("  rows            : {}", s.nodes);
    println!("  obsolete        : {}", s.obsolete);
    println!("  triples         : {}", s.triples);
    println!("  links packed    : {}", s.links_packed);
    println!("  links DROPPED   : {}", s.links_dropped);
    println!("  rows overflowed : {}", s.rows_overflowed);
    println!("  is_a cycles     : {}", s.is_a_cycles);
    println!("  dangling        : {}", s.dangling);
    assert_eq!(s.links_dropped, 0, "artifact is missing edges");
    assert_eq!(s.rows_overflowed, 0, "artifact has truncated rows");
    assert_eq!(s.links_packed, s.triples, "not every triple is in-row");

    // Per-namespace census in table (= row) order.
    let mut census: HashMap<u8, usize> = HashMap::new();
    for id in &baked.ids {
        *census.entry(id.ns).or_default() += 1;
    }
    for (i, spec) in reg.specs().iter().enumerate() {
        println!(
            "    {:6}: {:5} rows  classid 0x{:08X}",
            spec.prefix,
            census.get(&(i as u8)).copied().unwrap_or(0),
            reg.render_classid(i as u8, 0x0000).unwrap()
        );
    }

    // Read every numeric-0 link back out of the baked lanes — the direct
    // proof that the bake is now traversable through those terms.
    let mut recovered = 0usize;
    for row in &baked.rows {
        for (_, l) in ogar_obo::edges::EdgeLanes::new(&row.0).links() {
            if l.num == 0 {
                recovered += 1;
            }
        }
    }
    println!("\n  numeric-0 links READ BACK from lanes    : {recovered} of {zero_in_edges}");
    assert_eq!(
        recovered, zero_in_edges,
        "every numeric-0 edge must survive the round trip — this is gate A1"
    );

    let bytes = as_le_bytes(&baked.rows);
    std::fs::write(&out, bytes).unwrap_or_else(|e| panic!("write {out}: {e}"));
    let inmem = rows_from_le_bytes(bytes).expect("in-memory aligned view");
    assert_eq!(inmem.len(), baked.rows.len());
    println!(
        "\n=== ARTIFACT ===\n  {out}  ({} bytes = {} rows x 512)\n  loader contract: VERIFIED",
        bytes.len(),
        baked.rows.len()
    );

    let other = baked
        .triples
        .iter()
        .filter(|t| t.p == Predicate::Other)
        .count();
    println!(
        "  typed edges on Predicate::Other: {other} of {} (no CURIE row matched)",
        s.triples
    );
}

fn read_named(dir: &Path, prefix: &str) -> Option<(std::path::PathBuf, Vec<u8>)> {
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.file_stem().and_then(|s| s.to_str()) == Some(prefix) {
            let b = std::fs::read(&p).ok()?;
            return Some((p, b));
        }
    }
    None
}
