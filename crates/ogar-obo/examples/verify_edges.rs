//! Verify the packed edge lanes against the sources, ontology by ontology.
//!
//! The falsifier is independence: the links are read back **out of the row
//! bytes** (lane decode via [`ogar_obo::edges::EdgeLanes`]) and compared as a
//! multiset against the SPO triple table the bake built from the parsed `.obo`.
//! Those are two different code paths over the same input, so agreement is
//! evidence and disagreement names the row.
//!
//! Order is RO → UBERON → HPO → (PATO, MONDO), smallest first: RO is two edges
//! and can be checked by eye before anything larger is trusted.
//!
//! Usage: `cargo run -p ogar-obo --example verify_edges -- [src_dir]`

use ogar_obo::{
    Namespace, Predicate, TermId, bake, edges::EdgeLanes, merge_logical_defs,
    parse_hp_logical_defs, parse_obo,
};
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
    let s = &baked.stats;
    println!("=== BAKE ===");
    println!("  rows          : {}", s.nodes);
    println!("  triples       : {}", s.triples);
    println!("  links packed  : {}", s.links_packed);
    println!("  links dropped : {} (must be 0)", s.links_dropped);
    println!("  rows overflow : {} (must be 0)", s.rows_overflowed);
    assert_eq!(
        s.links_dropped, 0,
        "edges were dropped — artifact incomplete"
    );
    assert_eq!(s.rows_overflowed, 0);
    assert_eq!(
        s.links_packed, s.triples,
        "every out-of-line triple must also be in-row"
    );

    // expected: (subject, predicate, target-classid, target-num) from the triples
    let mut expected: HashMap<TermId, Vec<(u8, u32, u32)>> = HashMap::new();
    for t in &baked.triples {
        expected.entry(t.s).or_default().push((
            t.p as u8,
            t.o.namespace().render_classid(0x0000),
            t.o.num,
        ));
    }

    let order = [
        Namespace::Ro,
        Namespace::Uberon,
        Namespace::Hpo,
        Namespace::Pato,
        Namespace::Mondo,
    ];
    let mut all_ok = true;
    for ns in order {
        let mut rows = 0usize;
        let mut links = 0usize;
        let mut is_a = 0usize;
        let mut bad: Vec<TermId> = Vec::new();
        for (i, id) in baked.ids.iter().enumerate() {
            if id.ns != ns as u8 {
                continue;
            }
            rows += 1;
            let lanes = EdgeLanes::new(&baked.rows[i].0);
            let mut got: Vec<(u8, u32, u32)> = lanes
                .links()
                .map(|(p, l)| (p as u8, l.classid, l.num))
                .collect();
            links += got.len();
            is_a += lanes.degree(Predicate::IsA) as usize;
            let mut want = expected.get(id).cloned().unwrap_or_default();
            got.sort_unstable();
            want.sort_unstable();
            if got != want {
                bad.push(*id);
            }
        }
        let ok = bad.is_empty();
        all_ok &= ok;
        println!(
            "\n=== {:8} rows={rows} links_read={links} is_a_degree_total={is_a} — {}",
            ns.prefix(),
            if ok { "MATCH" } else { "MISMATCH" }
        );
        if !ok {
            for id in bad.iter().take(5) {
                let i = baked.ids.iter().position(|x| x == id).unwrap();
                println!(
                    "    {}:{:07} got={:?} want={:?}",
                    ns.prefix(),
                    id.num,
                    EdgeLanes::new(&baked.rows[i].0)
                        .links()
                        .map(|(p, l)| (p, l.num))
                        .collect::<Vec<_>>(),
                    expected.get(id)
                );
            }
            println!("    ... {} mismatching rows total", bad.len());
        }
        // RO is small enough to print in full — the by-eye check.
        if ns == Namespace::Ro {
            for (i, id) in baked.ids.iter().enumerate() {
                if id.ns != ns as u8 {
                    continue;
                }
                let l: Vec<_> = EdgeLanes::new(&baked.rows[i].0)
                    .links()
                    .map(|(p, l)| format!("{p:?}->0x{:08x}:{}", l.classid, l.num))
                    .collect();
                println!("    RO:{:07} -> [{}]", id.num, l.join(", "));
            }
        }
    }

    assert!(all_ok, "lane decode disagrees with the triple table");
    println!("\nALL LANES VERIFIED against the SPO triple table.");
}
