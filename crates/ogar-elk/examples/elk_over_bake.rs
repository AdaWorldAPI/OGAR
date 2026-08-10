//! ELK over the **baked** substrate — the post-bake observation path.
//!
//! Reads a `.soa` slab, peeks the `is_a` links out of each row's own edge lanes
//! (`ogar_obo::edges::subsumptions`), closes over them, and reports what
//! follows. No `.obo` file is opened; no CURIE is parsed. The only input is
//! positions in baked rows, which is what makes this observation rather than a
//! second harvest.
//!
//! Usage: `cargo run -p ogar-elk --example elk_over_bake -- <slab.soa>`

use ogar_elk::{ClassAddr, Closure, Subsumption};
use ogar_obo::{NODE_ROW_STRIDE, Row512, edges};

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/obo/obo-core.soa".to_string());
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    assert!(
        bytes.len().is_multiple_of(NODE_ROW_STRIDE),
        "not a slab: {} bytes is not a multiple of {NODE_ROW_STRIDE}",
        bytes.len()
    );

    // A harness copy into an aligned buffer. On the real path Lance hands over
    // an arrow-aligned FixedSizeBinary(512) column and this step does not exist.
    let n = bytes.len() / NODE_ROW_STRIDE;
    let mut rows = vec![Row512::zeroed(); n];
    for (i, r) in rows.iter_mut().enumerate() {
        r.0.copy_from_slice(&bytes[i * NODE_ROW_STRIDE..(i + 1) * NODE_ROW_STRIDE]);
    }
    println!("slab: {path}\n  rows: {n}");

    // ── the peek: is_a links straight out of the lanes ───────────────────────
    let t0 = std::time::Instant::now();
    let closure = Closure::from_asserted(edges::subsumptions(&rows).map(|(sub, sup)| {
        Subsumption::new(
            ClassAddr::new(sub.classid, sub.num),
            ClassAddr::new(sup.classid, sup.num),
        )
    }));
    let build = t0.elapsed();
    println!(
        "  asserted subsumptions read from lanes: {} in {build:?}",
        closure.asserted_len()
    );

    // Per-class census of what the closure indexed, so the numbers are
    // attributable to an ontology rather than a single total.
    let mut by_class: std::collections::BTreeMap<u32, usize> = Default::default();
    for c in closure.subclasses() {
        *by_class.entry(c.classid).or_default() += 1;
    }
    println!("  classes with at least one parent:");
    for (cid, count) in &by_class {
        println!("    0x{cid:08x}: {count}");
    }

    // ── question 1: what follows ─────────────────────────────────────────────
    // Depth and breadth of the spine, sampled over every indexed subclass.
    let t1 = std::time::Instant::now();
    let mut max_ancestors = 0usize;
    let mut deepest: Option<ClassAddr> = None;
    let mut total = 0usize;
    for c in closure.subclasses() {
        let a = closure.supers_of(c).len();
        total += a;
        if a > max_ancestors {
            max_ancestors = a;
            deepest = Some(c);
        }
    }
    let walk = t1.elapsed();
    let indexed = by_class.values().sum::<usize>();
    println!(
        "\n  transitive ancestors: {total} over {indexed} subclasses \
         (mean {:.1}, max {max_ancestors} at {deepest:?}) in {walk:?}",
        total as f64 / indexed.max(1) as f64
    );

    // ── question 2: is the baked spine sound ─────────────────────────────────
    let cycles = closure.equivalence_cycles();
    println!(
        "\n  equivalence cycles in the baked spine: {} (depth cap {})",
        cycles.len(),
        closure.depth_cap()
    );
    for c in cycles.iter().take(10) {
        println!("    0x{:08x}:{}", c.classid, c.identity);
    }
    if cycles.len() > 10 {
        println!("    ... {} total", cycles.len());
    }

    // A worked entailment, printed so the run is not merely a count: the
    // deepest class and one of its derived ancestors, plus a control that must
    // NOT be entailed.
    if let Some(d) = deepest {
        let supers = closure.supers_of(d);
        if let Some(top) = supers.iter().max() {
            assert!(closure.entails(d, *top));
            println!(
                "\n  entails: 0x{:08x}:{} ⊑ 0x{:08x}:{}  (derived)",
                d.classid, d.identity, top.classid, top.identity
            );
        }
        // control: an identity no ontology mints (numeric 0 never occurs)
        let ghost = ClassAddr::new(d.classid, 0);
        assert!(
            !closure.entails(d, ghost),
            "entailment must be able to say no"
        );
        println!("  and does NOT entail a class that was never minted — the walk discriminates");
    }
}
