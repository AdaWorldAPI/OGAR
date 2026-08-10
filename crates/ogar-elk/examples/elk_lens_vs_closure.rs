//! The falsifier for the lazy lens, plus an honest cost comparison.
//!
//! The lens exists to reason over a hydrated table without copying it. That is
//! only worth anything if it gives the **same answers** as the indexed
//! [`Closure`], so this runs both over the real bake and compares them class by
//! class — not on a sample, on every class that has a parent.
//!
//! It also reports what the lens costs. The lens is not free and is not
//! uniformly faster; the numbers below are printed whether or not they flatter
//! it, because a lens that quietly lost on every axis would still pass a
//! correctness check.
//!
//! Usage: `cargo run --release -p ogar-elk --example elk_lens_vs_closure -- [slab.soa]`

use ogar_elk::{ClassAddr, Closure, LensClosure, Subsumption};
use ogar_obo::{NODE_ROW_STRIDE, Row512, edges, spine::SpineLens};

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/obo/obo-core.soa".to_string());
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    assert!(bytes.len().is_multiple_of(NODE_ROW_STRIDE), "not a slab");

    // Harness copy into an aligned buffer. On the real path Lance hands over an
    // arrow-aligned FixedSizeBinary(512) column and this step does not exist.
    let n = bytes.len() / NODE_ROW_STRIDE;
    let mut rows = vec![Row512::zeroed(); n];
    for (i, r) in rows.iter_mut().enumerate() {
        r.0.copy_from_slice(&bytes[i * NODE_ROW_STRIDE..(i + 1) * NODE_ROW_STRIDE]);
    }
    println!("slab {path}\n  rows {n}");

    // ── the lens: borrow, assert the precondition, wrap in a Parents closure ──
    let lens = SpineLens::new(&rows);
    assert!(
        lens.is_sorted(),
        "binary-search resolve requires ascending (classid, num)"
    );
    let parents = |a: ClassAddr, visit: &mut dyn FnMut(ClassAddr)| {
        lens.parents_of(a.classid, a.identity, &mut |c, num| {
            visit(ClassAddr::new(c, num));
        });
    };
    let lc = LensClosure::new(&parents);

    // ── the indexed closure, for comparison ──
    let t = std::time::Instant::now();
    let cl = Closure::from_asserted(edges::subsumptions(&rows).map(|(s, p)| {
        Subsumption::new(
            ClassAddr::new(s.classid, s.num),
            ClassAddr::new(p.classid, p.num),
        )
    }));
    let build = t.elapsed();
    let subs: Vec<ClassAddr> = cl.subclasses().collect();
    println!(
        "  indexed closure: {} asserted edges over {} subclasses, built in {build:?}",
        cl.asserted_len(),
        subs.len()
    );
    println!("  lens: 0 edges indexed, 0 build time — it borrows the rows\n");

    // ── THE FALSIFIER: identical ancestor sets, every class, no sampling ──
    let t = std::time::Instant::now();
    let mut total = 0usize;
    let mut diverged = 0usize;
    let mut first_bad: Option<ClassAddr> = None;
    for &c in &subs {
        let a = lc.supers_of(c);
        let mut b: Vec<ClassAddr> = cl.supers_of(c).into_iter().collect();
        b.sort_unstable();
        if a != b {
            diverged += 1;
            if first_bad.is_none() {
                first_bad = Some(c);
            }
        }
        total += a.len();
    }
    let lens_walk = t.elapsed();

    let t = std::time::Instant::now();
    let control: usize = subs.iter().map(|&c| cl.supers_of(c).len()).sum();
    let idx_walk = t.elapsed();

    println!("=== FALSIFIER ===");
    println!("  classes compared      : {}", subs.len());
    println!(
        "  ancestor sets differing: {diverged}  (must be 0){}",
        first_bad.map_or(String::new(), |c| format!("  first: {c:?}"))
    );
    println!("  transitive ancestors  : lens {total} | indexed {control}");
    assert_eq!(diverged, 0, "the lens disagrees with the indexed closure");
    assert_eq!(total, control, "ancestor totals differ");

    // Cycles: the lens is per-class by design (it has no membership roll).
    let lens_cycles = subs.iter().filter(|&&c| lc.reaches_itself(c)).count();
    let idx_cycles = cl.equivalence_cycles().len();
    println!("  equivalence cycles    : lens {lens_cycles} | indexed {idx_cycles}");
    assert_eq!(lens_cycles, idx_cycles);

    println!("\n=== COST (honest) ===");
    println!("  build           : lens 0s        | indexed {build:?}");
    println!("  full sweep      : lens {lens_walk:?} | indexed {idx_walk:?}");
    let ratio = lens_walk.as_secs_f64() / idx_walk.as_secs_f64().max(1e-9);
    println!("  sweep ratio     : lens is {ratio:.1}x the indexed walk");
    println!(
        "  residency       : lens holds a &[Row512] + a depth cap; the index holds a \n\
         \x20                  HashMap over {} edges for the life of the query set",
        cl.asserted_len()
    );

    // ── Schnittpunkte: the cross-ontology x-hop lookup ──
    println!("\n=== SCHNITTPUNKTE ===");
    let mut seeds: Vec<ClassAddr> = Vec::new();
    let mut by_class: std::collections::BTreeMap<u32, ClassAddr> = Default::default();
    for &c in &subs {
        by_class.entry(c.classid).or_insert(c);
    }
    for (_, c) in by_class.iter().take(2) {
        seeds.push(*c);
    }
    if seeds.len() == 2 {
        let m = lc.meet(&seeds);
        println!(
            "  meet({:?}) -> {} shared ancestor(s)",
            seeds
                .iter()
                .map(|s| format!("0x{:08x}:{}", s.classid, s.identity))
                .collect::<Vec<_>>(),
            m.len()
        );
        for a in m.iter().take(5) {
            println!("    0x{:08x}:{}", a.classid, a.identity);
        }
        // a class always meets itself in its own ancestry — a control that the
        // primitive is not simply returning empty for everything
        let self_meet = lc.meet(&[seeds[0], seeds[0]]);
        println!(
            "  control meet(x,x) -> {} (equals |ancestors(x)| = {})",
            self_meet.len(),
            lc.supers_of(seeds[0]).len()
        );
        assert_eq!(self_meet.len(), lc.supers_of(seeds[0]).len());
    }

    println!("\nLENS VERIFIED: identical answers, zero index built.");
}
