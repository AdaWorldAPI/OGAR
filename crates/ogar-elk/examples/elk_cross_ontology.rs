//! S4b — the typed cross-predicate walk, verified on **seeded, local** queries.
//!
//! `meet` over `is_a` alone always returns nothing across ontologies:
//! subsumption never leaves the ontology it belongs to. Crossing needs a
//! **role** — `has_phenotype` into HPO, `has_anatomy` into Uberon — and then
//! R∃ generalization of the filler up the grounding spine.
//!
//! # Scale: seeded from facts, never a global saturation
//!
//! A query starts from facts that already exist — an observed lab code, a
//! dispensed drug, an imaging study, a coded diagnosis. That is a handful of
//! addresses. Saturating the whole ontology to answer something local is a
//! theory-of-everything computation nobody asked for, and this example
//! deliberately does not do it.
//!
//! # Why the check is NOT against `reason::saturate`
//!
//! Measured this session: the combined `is_a ∪ part_of` grounding spine carries
//! **2 277 back edges** (is_a alone has none), and `ancestor_closure` SKIPS a
//! back edge when it meets one (`!stack.iter().any(|(x, _)| *x == p)`). Its
//! memo is therefore an incomplete closure whose value depends on DFS start
//! order — the same code over the same triples in two orders produced
//! 9 522 084 and 9 437 429. A number that moves cannot falsify anything.
//!
//! So this verifies the R∃ *semantics* on bounded queries against an
//! independent computation sharing no code with [`fillers_closed`].
//!
//! Usage: `cargo run --release -p ogar-elk --example elk_cross_ontology -- [slab.soa]`

use ogar_elk::{ClassAddr, LensClosure, Role, fillers_closed, meet_via, most_specific};
use ogar_obo::{NODE_ROW_STRIDE, Predicate, Row512, edges::EdgeLanes, spine::SpineLens};

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/obo/obo-core.soa".to_string());
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let n = bytes.len() / NODE_ROW_STRIDE;
    let mut rows = vec![Row512::zeroed(); n];
    for (i, r) in rows.iter_mut().enumerate() {
        r.0.copy_from_slice(&bytes[i * NODE_ROW_STRIDE..(i + 1) * NODE_ROW_STRIDE]);
    }
    let lens = SpineLens::new(&rows);
    assert!(lens.is_sorted());
    println!("slab {path}\n  rows {n}");

    // The GROUNDING spine — is_a ∪ part_of. Deliberately NOT the subsumption
    // source: a filler generalizes through mereology, a subject must not.
    let grounding = |a: ClassAddr, visit: &mut dyn FnMut(ClassAddr)| {
        lens.grounding_parents_of(a.classid, a.identity, &mut |c, num| {
            visit(ClassAddr::new(c, num));
        });
    };
    let fillers = |a: ClassAddr, r: Role, visit: &mut dyn FnMut(ClassAddr)| {
        let p = match r.0 {
            3 => Predicate::HasPhenotype,
            4 => Predicate::HasLocation,
            5 => Predicate::HasAnatomy,
            6 => Predicate::HasQuality,
            _ => Predicate::PartOf,
        };
        lens.fillers_of(a.classid, a.identity, p, &mut |c, num| {
            visit(ClassAddr::new(c, num));
        });
    };
    let has_phenotype = Role(Predicate::HasPhenotype as u8);

    // ── seeds: the facts a query would actually start from ──
    let mut seeds: Vec<ClassAddr> = Vec::new();
    for row in &rows {
        if EdgeLanes::new(&row.0).degree(Predicate::HasPhenotype) > 0 {
            let s = ogar_obo::edges::subject(&row.0);
            seeds.push(ClassAddr::new(s.classid, s.num));
            if seeds.len() == 64 {
                break;
            }
        }
    }
    println!("  seeds (diseases asserting a phenotype): {}", seeds.len());
    assert!(!seeds.is_empty(), "no seed carries a phenotype");

    // ── FALSIFIER: R∃ against an independent local computation ──
    let t = std::time::Instant::now();
    let mut checked = 0usize;
    let mut widened = 0usize;
    for &d in &seeds {
        let got = fillers_closed(d, has_phenotype, &fillers, &grounding, 64);

        // Independent path: asserted fillers off the row, then a hand-rolled
        // BFS up the grounding spine. No shared code with `fillers_closed`.
        let mut asserted: Vec<ClassAddr> = Vec::new();
        lens.fillers_of(
            d.classid,
            d.identity,
            Predicate::HasPhenotype,
            &mut |c, num| asserted.push(ClassAddr::new(c, num)),
        );
        let mut want: Vec<ClassAddr> = Vec::new();
        for &f in &asserted {
            let mut queue = std::collections::VecDeque::from([(f, 0usize)]);
            let mut seen: Vec<ClassAddr> = vec![f];
            if let Err(at) = want.binary_search(&f) {
                want.insert(at, f);
            }
            while let Some((x, depth)) = queue.pop_front() {
                if depth >= 64 {
                    continue;
                }
                lens.grounding_parents_of(x.classid, x.identity, &mut |c, num| {
                    let p = ClassAddr::new(c, num);
                    if let Err(at) = seen.binary_search(&p) {
                        seen.insert(at, p);
                        queue.push_back((p, depth + 1));
                        if let Err(w) = want.binary_search(&p) {
                            want.insert(w, p);
                        }
                    }
                });
            }
        }
        assert_eq!(
            got, want,
            "R∃ diverges for 0x{:08x}:{}",
            d.classid, d.identity
        );
        checked += 1;
        if got.len() > asserted.len() {
            widened += 1;
        }
    }
    let el = t.elapsed();

    println!("\n=== FALSIFIER (R∃, local) ===");
    println!("  seeds checked          : {checked}  (0 divergences)  in {el:?}");
    println!("  seeds where R∃ widened : {widened}  (must be > 0, else generalization is inert)");
    assert!(
        widened > 0,
        "R∃ never added an ancestor — the rule is doing nothing"
    );

    // ── the Schnittpunkt is_a alone cannot find ──
    println!("\n=== CROSS-ONTOLOGY SCHNITTPUNKTE ===");

    // control: is_a alone must never leave the seed's own ontology
    let isa = |a: ClassAddr, visit: &mut dyn FnMut(ClassAddr)| {
        lens.parents_of(a.classid, a.identity, &mut |c, num| {
            visit(ClassAddr::new(c, num));
        });
    };
    let isa_lens = LensClosure::new(&isa);
    let mut isa_cross = 0usize;
    let k = seeds.len().min(16);
    for i in 0..k {
        for j in (i + 1)..k {
            if isa_lens
                .meet(&[seeds[i], seeds[j]])
                .iter()
                .any(|a| a.classid != seeds[i].classid)
            {
                isa_cross += 1;
            }
        }
    }
    println!("  via is_a : {isa_cross} pairs meeting OUTSIDE their own ontology (expected 0)");
    assert_eq!(isa_cross, 0, "subsumption must not cross an ontology");

    let mut found = 0usize;
    let mut empty = 0usize;
    let mut root_only = 0usize;
    let mut specific = 0usize;
    for i in 0..k {
        for j in (i + 1)..k {
            let m = meet_via(
                &[seeds[i], seeds[j]],
                has_phenotype,
                &fillers,
                &grounding,
                64,
            );
            if m.is_empty() {
                empty += 1;
                continue;
            }
            found += 1;
            // The raw meet is sound but vacuous: every phenotype generalizes to
            // the HPO root, so every pair "shares" it. Narrow to the most
            // specific common filler — the answer that carries information.
            let narrowed = most_specific(&m, &grounding, 64);
            let is_root_only = narrowed.len() == 1
                && lens
                    .resolve(narrowed[0].classid, narrowed[0].identity)
                    .is_some_and(|r| EdgeLanes::new(&rows[r].0).degree(Predicate::IsA) == 0);
            if is_root_only {
                root_only += 1;
            } else {
                specific += 1;
                if specific <= 3 {
                    println!(
                        "    0x{:08x}:{} ∩ 0x{:08x}:{} -> {} shared, {} most-specific, e.g. 0x{:08x}:{}",
                        seeds[i].classid,
                        seeds[i].identity,
                        seeds[j].classid,
                        seeds[j].identity,
                        m.len(),
                        narrowed.len(),
                        narrowed[0].classid,
                        narrowed[0].identity
                    );
                }
            }
        }
    }
    println!("  via role : {found} pairs share something | {empty} disjoint");
    println!("  narrowed : {specific} pairs share something SPECIFIC | {root_only} only the root");
    assert!(
        found > 0,
        "the role walk found no Schnittpunkt — is_a-only behaviour"
    );
    assert!(
        specific > 0,
        "no pair shared anything below the root — narrowing found nothing informative"
    );
    assert!(
        root_only > 0 || empty > 0,
        "every pair sharing something specific would mean the walk discriminates nothing"
    );

    println!("\nS4b VERIFIED: R∃ sound on seeded queries, roles cross, meets discriminate.");
}
