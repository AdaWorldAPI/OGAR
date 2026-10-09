//! Measures the identity correspondence fold, stage by stage.
//!
//! ```text
//! cargo run --release -p ogar-dir-core --example identity_fold_bench
//! ```
//!
//! Every stage is timed separately (boundary decode, index build, fold), with
//! warm-up and repetitions; the allocator counts allocations and peak bytes,
//! so "zero allocations in the fold" is a measurement, not a claim.

use ogar_dir_core::correspond::{
    AdStatus, IdColumn, Index, Lanes, NO_ROW, Output, Planes, Rows, Ruler, UNBOUND, fold,
    fold_aligned, state,
};
use ogar_dir_core::{Encoding, Guid128, LabelPattern};
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        let now = LIVE.fetch_add(l.size(), Relaxed) + l.size();
        PEAK.fetch_max(now, Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Relaxed);
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static A: Counting = Counting;

const TENANTS: u32 = 16;
const PER_TENANT: u32 = 65_536; // the directory simulator's population cap
const N: usize = (TENANTS * PER_TENANT) as usize; // 1,048,576
const WARM: usize = 3;
const REPS: usize = 15;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn id(rng: &mut Rng) -> Guid128 {
    let mut b = [0u8; 16];
    b[..8].copy_from_slice(&rng.next().to_le_bytes());
    b[8..].copy_from_slice(&rng.next().to_le_bytes());
    Guid128(b)
}

/// Median and maximum of `REPS` timed runs (after `WARM` untimed ones), in
/// nanoseconds, plus allocations made inside the timed runs.
fn time(mut f: impl FnMut()) -> (u128, u128, usize) {
    for _ in 0..WARM {
        f();
    }
    let mut t: Vec<u128> = Vec::with_capacity(REPS);
    let a0 = ALLOCS.load(Relaxed);
    for _ in 0..REPS {
        let s = Instant::now();
        f();
        t.push(s.elapsed().as_nanos());
    }
    let allocs = ALLOCS.load(Relaxed) - a0;
    t.sort_unstable();
    (t[REPS / 2], t[REPS - 1], allocs)
}

fn row(name: &str, ops: usize, (med, max, allocs): (u128, u128, usize)) {
    println!(
        "| {name:<44} | {ops:>9} | {:>9.3} | {:>9.3} | {:>7.2} | {:>8.1} | {allocs:>6} |",
        med as f64 / 1e6,
        max as f64 / 1e6,
        med as f64 / ops as f64,
        ops as f64 / (med as f64 / 1e9) / 1e6,
    );
}

/// The workload knobs.
#[derive(Clone, Copy)]
struct Shape {
    /// Percent of AD rows with an anchor that some Entra row carries.
    match_pct: u64,
    /// Percent of AD rows whose anchor is absent.
    absent_pct: u64,
    /// Percent of AD rows that share their anchor with the next row.
    dup_pct: u64,
    /// Percent of AD rows in an unbound scope.
    unbound_pct: u64,
}

fn lanes(shape: Shape, seed: u64) -> Lanes {
    let mut rng = Rng(seed);
    let mut l = Lanes {
        max_skew_ms: 60_000,
        ..Lanes::default()
    };
    let rows = |n: usize| Rows {
        owner: Vec::with_capacity(n),
        scope: Vec::with_capacity(n),
        at: Vec::with_capacity(n),
    };
    let col = |n: usize| IdColumn {
        id: Vec::with_capacity(n),
        state: Vec::with_capacity(n),
    };
    (l.ad, l.entra, l.exo) = (rows(N), rows(N), rows(N));
    (l.ad_anchor, l.ad_backsync, l.entra_anchor, l.exo_external) = (col(N), col(N), col(N), col(N));
    let mut prev_anchor = Guid128::NIL;
    for i in 0..N {
        let tenant = i as u32 / PER_TENANT;
        let (entra_id, anchor) = (id(&mut rng), id(&mut rng));
        let roll = rng.next() % 100;
        // Entra: every row a cloud object; the immutable id is the anchor
        // for `match_pct` of them, an unrelated id otherwise.
        l.entra.owner.push(entra_id);
        l.entra.scope.push(tenant);
        l.entra.at.push(1_000);
        let carried = roll < shape.match_pct;
        l.entra_anchor
            .id
            .push(if carried { anchor } else { id(&mut rng) });
        l.entra_anchor.state.push(state::PRESENT);
        // AD.
        l.ad.owner.push(id(&mut rng));
        l.ad.scope.push(if rng.next() % 100 < shape.unbound_pct {
            UNBOUND
        } else {
            tenant
        });
        l.ad.at.push(1_000);
        let a_roll = rng.next() % 100;
        if a_roll < shape.absent_pct {
            l.ad_anchor.id.push(Guid128::NIL);
            l.ad_anchor.state.push(state::ABSENT);
        } else {
            let a = if a_roll < shape.absent_pct + shape.dup_pct && !prev_anchor.is_nil() {
                prev_anchor
            } else {
                anchor
            };
            prev_anchor = a;
            l.ad_anchor.id.push(a);
            l.ad_anchor.state.push(state::PRESENT);
        }
        l.ad_backsync.id.push(entra_id);
        l.ad_backsync.state.push(state::PRESENT);
        // Exchange Online: one mailbox per cloud object.
        l.exo.owner.push(id(&mut rng));
        l.exo.scope.push(tenant);
        l.exo.at.push(1_000);
        l.exo_external.id.push(entra_id);
        l.exo_external.state.push(state::PRESENT);
    }
    l
}

fn fold_case(name: &str, shape: Shape) {
    let l = lanes(shape, 0xC0FF_EE00_0000_0001);
    let a0 = ALLOCS.load(Relaxed);
    let t = Instant::now();
    let ix = Index::build(&l);
    let build_ns = t.elapsed().as_nanos();
    let build_allocs = ALLOCS.load(Relaxed) - a0;
    let mut out = Output::for_index(&l, &ix);
    let r = time(|| {
        fold(black_box(&l), black_box(&ix), &mut out);
        black_box(&out);
    });
    row(&format!("C fold, {name}"), N, r);
    println!(
        "|   index build (cold, once)                   | {N:>9} | {:>9.3} |           |         |          | {build_allocs:>6} |",
        build_ns as f64 / 1e6
    );
    let mut hist: HashMap<AdStatus, usize> = HashMap::new();
    for &f in &out.ad {
        *hist.entry(AdStatus::of(f)).or_default() += 1;
    }
    let mut h: Vec<_> = hist.into_iter().collect();
    h.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    println!("|   edges {} · outcomes {h:?}", out.edges.len());
    let targeted = out.ad_target.iter().filter(|&&t| t != NO_ROW).count();
    println!("|   AD rows with exactly one forward target: {targeted}");

    // The same population on one ruler: slot i is the same user in every
    // column, so each witness is a position-wise compare.
    let t = Instant::now();
    let ruler = Ruler::from_fold(&l, &ix, &out);
    let lay_ns = t.elapsed().as_nanos();
    let mut planes = Planes::for_ruler(&ruler);
    let r = time(|| {
        fold_aligned(black_box(&ruler), &mut planes);
        black_box(&planes);
    });
    row(&format!("R aligned fold, {name}"), N, r);
    println!(
        "|   lay onto ruler (cold, once)                | {N:>9} | {:>9.3} |           |         |          |        |",
        lay_ns as f64 / 1e6
    );
    let confirmed: u32 = (0..planes.forward.len())
        .map(|w| planes.confirmed(w).count_ones())
        .sum();
    let joined = out
        .ad
        .iter()
        .filter(|&&f| AdStatus::of(f) == AdStatus::Confirmed)
        .count();
    println!("|   confirmed slots {confirmed} (join fold: {joined})");
}

fn main() {
    println!(
        "rustc: {}",
        option_env!("RUSTC_VERSION").unwrap_or("see `rustc -V`")
    );
    if let Ok(c) = std::fs::read_to_string("/proc/cpuinfo") {
        if let Some(m) = c.lines().find(|l| l.starts_with("model name")) {
            println!("cpu: {}", m.split(':').nth(1).unwrap_or("").trim());
        }
        println!(
            "logical cpus: {} (single-threaded run)",
            c.lines().filter(|l| l.starts_with("processor")).count()
        );
    }
    println!(
        "profile: {}",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    println!("warm-up {WARM}, repetitions {REPS}, rows per side {N}\n");
    println!(
        "| stage                                        |       ops | median ms |    max ms | ns / op | Mops / s | allocs |"
    );
    println!(
        "|----------------------------------------------|-----------|-----------|-----------|---------|----------|--------|"
    );

    // A: pure equality, 50 % equal.
    let mut rng = Rng(7);
    let xs: Vec<Guid128> = (0..N).map(|_| id(&mut rng)).collect();
    let ys: Vec<Guid128> = xs
        .iter()
        .map(|x| {
            if rng.next().is_multiple_of(2) {
                *x
            } else {
                id(&mut rng)
            }
        })
        .collect();
    let r = time(|| {
        let eq = xs.iter().zip(&ys).filter(|(a, b)| a == b).count();
        black_box(eq);
    });
    row("A Guid128 equality (50 % equal)", N, r);

    // B: indexed lookups, two baselines, prebuilt (build time excluded).
    let keys: Vec<Guid128> = (0..N).map(|_| id(&mut rng)).collect();
    let probes: Vec<Guid128> = keys
        .iter()
        .map(|k| {
            if rng.next().is_multiple_of(2) {
                *k
            } else {
                id(&mut rng)
            }
        })
        .collect();
    let map: HashMap<Guid128, u32> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| (*k, i as u32))
        .collect();
    let r = time(|| {
        let hit = probes.iter().filter(|p| map.contains_key(p)).count();
        black_box(hit);
    });
    row("B HashMap<Guid128> lookup (50 % hit)", N, r);
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    let r = time(|| {
        let hit = probes
            .iter()
            .filter(|p| sorted.binary_search(p).is_ok())
            .count();
        black_box(hit);
    });
    row("B sorted Guid128 binary search (50 % hit)", N, r);

    // C: the full three-witness fold.
    let typical = Shape {
        match_pct: 95,
        absent_pct: 3,
        dup_pct: 0,
        unbound_pct: 0,
    };
    fold_case("typical (95 % matched)", typical);

    // E: adversarial shapes.
    fold_case(
        "sparse (1 % matched)",
        Shape {
            match_pct: 1,
            ..typical
        },
    );
    fold_case(
        "duplicate anchors (20 %)",
        Shape {
            dup_pct: 20,
            ..typical
        },
    );
    fold_case(
        "half absent anchors",
        Shape {
            absent_pct: 50,
            ..typical
        },
    );
    fold_case(
        "half unbound scope",
        Shape {
            unbound_pct: 50,
            ..typical
        },
    );

    // D: boundary conversions, which the fold never performs.
    let ms: Vec<[u8; 16]> = keys.iter().map(|k| k.to_ms_bytes()).collect();
    let b64: Vec<Vec<u8>> = keys
        .iter()
        .map(|k| Encoding::Base64MsGuidBytes16.encode(*k))
        .collect();
    let r = time(|| {
        for b in &b64 {
            black_box(Encoding::Base64MsGuidBytes16.decode(b).ok());
        }
    });
    row("D decode onPremisesImmutableId (base64)", N, r);
    let r = time(|| {
        for b in &ms {
            black_box(Encoding::MsGuidBytes16.decode(b).ok());
        }
    });
    row("D decode mixed-endian 16 bytes", N, r);
    let user = Encoding::Labeled(LabelPattern("User_{0}"));
    let r = time(|| {
        for k in &keys {
            black_box(user.encode(*k));
        }
    });
    row("D render User_{0} (egress)", N, r);
    let r = time(|| {
        for k in &keys {
            black_box(Encoding::Base64MsGuidBytes16.encode(*k));
        }
    });
    row("D render base64 (egress)", N, r);

    println!(
        "\npeak heap: {:.1} MiB",
        PEAK.load(Relaxed) as f64 / (1 << 20) as f64
    );
}
