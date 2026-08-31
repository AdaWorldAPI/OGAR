//! Counterfactual visibility probe — WIRED to the loco/r2il thinking surface.
//!
//! # What changed against the earlier lance-graph stream probe
//!
//! The earlier D-SK-STREAM fixture generated witness events from a seeded
//! RNG and called a seed-preserving reshuffle an "intervention" — measuring
//! that a mechanical dummy is mechanically invisible. This probe replaces
//! the dummy with the real thinking substrate:
//!
//! - the stream source is an `ogar_loco::FunctionBody` — a real stored
//!   program under the real call ABI, mixing shared-core operand producers
//!   (`NUMBER`, `VAR_GET`) with R2IL consumers (`IntAdd`/`IntSub`/`IntXor`/
//!   `Store`) resolved through `R2ILVocabulary`;
//! - the Markov window IS the loco statement: `statement_bounds` (the
//!   operand-producing post-order run plus its consuming call, the R5
//!   maskable unit) segments the body, and each statement becomes one
//!   witness window — intra-window order is the STACK DISCIPLINE, not noise;
//! - the counterfactual is a SEMANTIC PROGRAM EDIT, localized to one
//!   statement found via `statement_bounds` and confined under the masked
//!   lane projection (`CallMask` + `project`): CF-1 swaps `IntSub`'s
//!   operand-producing calls (minuend/subtrahend exchange — a dataflow
//!   counterfactual), CF-2 substitutes the consuming operator
//!   (`IntAdd` → `IntSub` — a rule counterfactual), CF-3 is the identical
//!   program (silence).
//!
//! # The three-tier visibility ladder under measurement
//!
//! For each counterfactual, three readings of "how far is the edited
//! timeline from the factual one":
//!
//! 1. **byte tier** — differing call slots between the two value slabs,
//!    counted through the r2il masked projection (loco's own diff);
//! 2. **register tier** — normalized signature-kernel distance with the
//!    24-locus register cast as level-2 coefficients (sign = orientation
//!    within the statement, magnitude = i4 net);
//! 3. **increment tier** — the same kernel, increments only.
//!
//! Pre-registered expectations (each an assert):
//!
//! - G0 (STOP on fail): the inline Goursat solver matches the I₀(2√⟨u,v⟩)
//!   closed form on linear paths (rel err < 2e-2).
//! - G1: segmentation is GREEN over the mixed core+R2IL body, and the edit
//!   is CONFINED: outside the edited statement's `CallMask`, the projected
//!   calls of factual and counterfactual slabs are identical; inside, they
//!    differ (the lens proves locality — can-fire + can-stay-silent).
//! - G2 (CF-1 operand swap): byte > 0, increment == 0 (net per-locus counts
//!   unchanged — increments are structurally blind to dataflow order),
//!   register > 0 (the orientation tier sees the swap).
//! - G3 (CF-2 operator substitution): byte > 0, increment == 0, AND
//!   register == 0 — the honest resolution limit: `IntAdd` and `IntSub`
//!   land on the same arithmetic locus, so a rule counterfactual below the
//!   locus granularity is visible ONLY at the byte tier. Named, not hidden.
//! - G4 (CF-3 identical): all three tiers read exactly 0.
//!
//! Run: `cargo run -p ogar-r2il --example probe_counterfactual_witness_kernel`

use ogar_loco::vocabulary::conformance::validate;
use ogar_loco::{
    Call, FnIndex, FunctionBody, LaneShape, StatementBounds, VALUE_SLAB_LEN, statement_bounds,
};
use ogar_r2il::{CallMask, R2ILFn, R2ILVocabulary, project};

const D: usize = 24;
const N_PAIRS: usize = D * (D - 1) / 2;
const SHAPE: LaneShape = LaneShape::Pairs;

fn r2il(name: &str) -> FnIndex {
    let o = R2ILFn::MNEMONICS
        .iter()
        .position(|m| *m == name)
        .unwrap_or_else(|| panic!("unknown mnemonic {name}"));
    R2ILFn::from_ordinal(o).unwrap().0
}

/// Function → witness locus. Deliberately COARSER than the codebook: the
/// register has 24 loci, the codebook 256 slots, so classes share loci —
/// G3 measures exactly what that costs.
fn locus(f: FnIndex, arith: &[FnIndex]) -> usize {
    if f == FnIndex::NUMBER {
        0
    } else if f == FnIndex::VAR_GET {
        1
    } else if arith[..3].contains(&f) {
        8 // IntAdd / IntSub / IntMult — one arithmetic locus
    } else if f == arith[3] {
        9 // IntXor — the bitwise locus
    } else {
        11 // Store — the memory-write locus
    }
}

/// One witness event, derived from one executed call.
#[derive(Clone, Copy)]
struct Event {
    t: f64, // position within the statement, (idx + 0.5) / stmt_len
    locus: usize,
    delta: f64, // 1 + immediate/64 — the immediate matters, order matters
}

/// The factual program: `n` statements of the R2IL store idiom
/// `NUMBER:a  VAR_GET:p  <arith>  VAR_GET:q  Store` — operand run early,
/// consumer late, exactly the stack shape `statement_bounds` segments.
fn factual_program(n: usize, arith: &[FnIndex]) -> FunctionBody {
    let mut calls = Vec::new();
    for s in 0..n {
        calls.push(Call::with_value(
            FnIndex::NUMBER,
            (7 + 13 * s as u32 % 200) as u8,
        ));
        calls.push(Call::with_value(FnIndex::VAR_GET, (s % 8) as u8));
        calls.push(Call::new(arith[s % 4]));
        calls.push(Call::with_value(FnIndex::VAR_GET, (s % 5) as u8));
        calls.push(Call::new(r2il("Store")));
    }
    FunctionBody::from_calls(SHAPE, &calls).expect("factual body fits")
}

/// Rebuild the body with `edit` applied to the call list — the counterfactual
/// constructor. Edits are index-local; the body is re-validated by
/// `from_calls`, so an edit that broke the ABI would refuse loudly.
fn edited(body: &FunctionBody, edit: impl Fn(&mut Vec<Call>)) -> FunctionBody {
    let mut calls: Vec<Call> = body.calls().collect();
    edit(&mut calls);
    FunctionBody::from_calls(SHAPE, &calls).expect("edited body fits")
}

/// Events per statement — the witness stream, one window per statement.
fn windows(body: &FunctionBody, bounds: &[StatementBounds], arith: &[FnIndex]) -> Vec<Vec<Event>> {
    let calls: Vec<Call> = body.calls().collect();
    bounds
        .iter()
        .map(|b| {
            (0..b.call_count)
                .map(|i| {
                    let c = calls[b.first_call + i];
                    Event {
                        t: (i as f64 + 0.5) / b.call_count as f64,
                        locus: locus(c.function, arith),
                        delta: 1.0 + f64::from(c.values[0]) / 64.0,
                    }
                })
                .collect()
        })
        .collect()
}

fn pair_idx(k: usize, l: usize) -> usize {
    k * D - k * (k + 1) / 2 + (l - k - 1)
}

/// Coarse path + exact per-window areas + 24×i4 registers, from the stream.
struct Coarse {
    pts: Vec<Vec<f64>>,
    areas_exact: Vec<Vec<f64>>,
    regs: Vec<[i8; D]>,
    scale: f64,
}

fn coarsen(wins: &[Vec<Event>]) -> Coarse {
    let mut nets_all = Vec::new();
    for w in wins {
        let mut net = [0.0f64; D];
        for e in w {
            net[e.locus] += e.delta;
        }
        nets_all.extend(net.iter().copied().filter(|v| *v > 0.0));
    }
    let scale = nets_all.iter().sum::<f64>() / nets_all.len().max(1) as f64;

    let mut x = vec![0.0f64; D];
    let mut pts = vec![x.clone()];
    let mut areas_exact = Vec::new();
    let mut regs = Vec::new();
    for w in wins {
        let x0 = x.clone();
        let mut a = vec![0.0f64; N_PAIRS];
        let mut net = [0.0f64; D];
        let mut t_sum = [0.0f64; D];
        for e in w {
            // A_kl += ½·rel_k·δ (moving l) / −½·rel_l·δ (moving k), rel to
            // the window start — the D-SK-STREAM accumulation, verbatim.
            for k in 0..D {
                if k != e.locus {
                    let rel_k = x[k] - x0[k];
                    if k < e.locus {
                        a[pair_idx(k, e.locus)] += 0.5 * rel_k * e.delta;
                    } else {
                        a[pair_idx(e.locus, k)] -= 0.5 * rel_k * e.delta;
                    }
                }
            }
            x[e.locus] += e.delta;
            net[e.locus] += e.delta;
            t_sum[e.locus] += e.t * e.delta;
        }
        let mut reg = [0i8; D];
        for l in 0..D {
            if net[l] > 0.0 {
                let o: i8 = if t_sum[l] / net[l] < 0.5 { -1 } else { 1 };
                reg[l] = o * (net[l] / scale).round().clamp(1.0, 7.0) as i8;
            }
        }
        pts.push(x.clone());
        areas_exact.push(a);
        regs.push(reg);
    }
    // Normalize total drift to 1 so the Goursat scheme stays in its stable
    // regime; registers are scale-free (they carry their own codebook scale).
    let s: f64 = pts
        .last()
        .unwrap()
        .iter()
        .map(|v| v * v)
        .sum::<f64>()
        .sqrt();
    for p in &mut pts {
        for v in p.iter_mut() {
            *v /= s;
        }
    }
    for a in &mut areas_exact {
        for v in a.iter_mut() {
            *v /= s * s;
        }
    }
    Coarse {
        pts,
        areas_exact,
        regs,
        scale: scale / s,
    }
}

/// Register-cast area surrogate (the E-MONOTONE-STREAM… construction):
/// Â_kl = ¼·v_k·v_l·(o_l − o_k), a pure function of the stored register.
fn surrogate_areas(regs: &[[i8; D]], scale: f64) -> Vec<Vec<f64>> {
    regs.iter()
        .map(|reg| {
            let mut a = vec![0.0f64; N_PAIRS];
            for k in 0..D {
                if reg[k] == 0 {
                    continue;
                }
                let (ok, vk) = (
                    f64::from(reg[k].signum()),
                    f64::from(reg[k].unsigned_abs()) * scale,
                );
                for l in (k + 1)..D {
                    if reg[l] == 0 {
                        continue;
                    }
                    let (ol, vl) = (
                        f64::from(reg[l].signum()),
                        f64::from(reg[l].unsigned_abs()) * scale,
                    );
                    a[pair_idx(k, l)] = 0.25 * vk * vl * (ol - ok);
                }
            }
            a
        })
        .collect()
}

/// First-order Goursat recursion with level-2-augmented coefficients —
/// the same scheme the lance-graph D-SK arc measured; inlined because this
/// crate deliberately stays sibling-free (see its module docs on deps).
fn kernel(x: &[Vec<f64>], ax: &[Vec<f64>], y: &[Vec<f64>], ay: &[Vec<f64>]) -> f64 {
    let (n, m) = (x.len(), y.len());
    let mut k = vec![vec![1.0f64; m]; n];
    for i in 0..n - 1 {
        let dx: Vec<f64> = (0..D).map(|d| x[i + 1][d] - x[i][d]).collect();
        for j in 0..m - 1 {
            let mut c = 0.0;
            for d in 0..D {
                c += dx[d] * (y[j + 1][d] - y[j][d]);
            }
            let mut area = 0.0;
            for t in 0..N_PAIRS {
                area += ax[i][t] * ay[j][t];
            }
            c += 2.0 * area;
            k[i + 1][j + 1] = k[i + 1][j] + k[i][j + 1] - k[i][j] + c * k[i][j];
        }
    }
    k[n - 1][m - 1]
}

fn zeros_like(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    a.iter().map(|v| vec![0.0; v.len()]).collect()
}

/// Normalized-kernel distance under a chosen area carrier.
fn nk_dist(cx: &Coarse, ax: &[Vec<f64>], cy: &Coarse, ay: &[Vec<f64>]) -> f64 {
    let kxy = kernel(&cx.pts, ax, &cy.pts, ay);
    let kxx = kernel(&cx.pts, ax, &cx.pts, ax);
    let kyy = kernel(&cy.pts, ay, &cy.pts, ay);
    1.0 - kxy / (kxx * kyy).sqrt().max(1e-300)
}

/// `I₀(2√⟨u,v⟩)` — the linear-path closed form, G0's anchor.
fn bessel_i0_of_2sqrt(dot: f64) -> f64 {
    let mut sum = 0.0;
    let mut term = 1.0f64;
    for k in 0..60 {
        if k > 0 {
            term *= dot / ((k * k) as f64);
        }
        sum += term;
        if term.abs() < 1e-18 * sum.abs() {
            break;
        }
    }
    sum
}

/// Byte-tier distance: differing call slots between two slabs, read through
/// the masked lane projection — loco's own diff, no bespoke byte walk.
fn byte_tier_diff(a: &[u8; VALUE_SLAB_LEN], b: &[u8; VALUE_SLAB_LEN]) -> usize {
    let all = CallMask::all(SHAPE);
    project(a, SHAPE, &all)
        .zip(project(b, SHAPE, &all))
        .filter(|((_, ca), (_, cb))| ca != cb)
        .count()
}

fn slab_of(body: &FunctionBody) -> [u8; VALUE_SLAB_LEN] {
    let mut slab = [0u8; VALUE_SLAB_LEN];
    body.write_into_value_slab(&mut slab);
    slab
}

fn main() {
    // ── G0: solver anchor ────────────────────────────────────────────────
    println!("== 0. G0: inline Goursat solver vs I0 closed form, d={D} ==");
    let n0 = 256;
    let u: Vec<f64> = (0..D)
        .map(|i| 0.4 * ((i * 37 + 11) % 100) as f64 / 100.0 - 0.1)
        .collect();
    let v: Vec<f64> = (0..D)
        .map(|i| 0.4 * ((i * 53 + 29) % 100) as f64 / 100.0 - 0.1)
        .collect();
    let lin = |dir: &[f64]| -> Vec<Vec<f64>> {
        (0..=n0)
            .map(|i| dir.iter().map(|d| d * i as f64 / n0 as f64).collect())
            .collect()
    };
    let (lu, lv) = (lin(&u), lin(&v));
    let dot: f64 = u.iter().zip(&v).map(|(a, b)| a * b).sum();
    let zl: Vec<Vec<f64>> = vec![vec![0.0; N_PAIRS]; n0 + 1];
    let k_pde = kernel(&lu, &zl, &lv, &zl);
    let k_cf = bessel_i0_of_2sqrt(dot);
    let g0 = (k_pde - k_cf).abs() / k_cf.abs().max(1e-12);
    println!("   rel err {g0:.2e}");
    assert!(g0 < 2e-2, "G0 FAIL — STOP");

    // ── The factual program, segmented by the real machinery ─────────────
    let arith = [
        r2il("IntAdd"),
        r2il("IntSub"),
        r2il("IntMult"),
        r2il("IntXor"),
    ];
    let vocab = validate(R2ILVocabulary).expect("R2IL vocabulary conforms");
    let n_stmt = 24usize;
    let factual = factual_program(n_stmt, &arith);
    let bounds = statement_bounds(&vocab, &factual).expect("mixed core+R2IL body segments");
    println!("\n== 1. G1: the loco statement IS the Markov window ==");
    println!(
        "   body: {} calls under {:?}; statement_bounds → {} statements (5 calls each)",
        factual.len(),
        SHAPE,
        bounds.len()
    );
    assert_eq!(bounds.len(), n_stmt, "G1 FAIL: segmentation drifted");

    // ── The three counterfactuals, statement-local, mask-confined ────────
    // The edited statement: pick one mid-body whose consumer is IntSub
    // (s % 4 == 1), so CF-1's operand swap is a real minuend/subtrahend
    // exchange, not a commutative no-op.
    let s_star = (0..n_stmt)
        .find(|s| s % 4 == 1 && *s >= n_stmt / 2)
        .unwrap();
    let b = bounds[s_star];

    // CF-1a: swap the two operand-producing calls (NUMBER ↔ first VAR_GET) —
    // both sit BEFORE the statement midpoint, so this dataflow edit is below
    // the register's 1-bit orientation resolution by construction.
    let cf1a = edited(&factual, |calls| {
        calls.swap(b.first_call, b.first_call + 1);
    });
    // CF-1b: swap NUMBER (position 0, early) with the second VAR_GET
    // (position 3, late) — the dataflow edit CROSSES the midpoint, which is
    // exactly what the register's orientation bit encodes. Still a valid
    // stack program (the segmentation below re-proves it).
    let cf1b = edited(&factual, |calls| {
        calls.swap(b.first_call, b.first_call + 3);
    });
    // CF-2: substitute the consuming operator IntSub → IntAdd.
    let cf2 = edited(&factual, |calls| {
        calls[b.first_call + 2] = Call::new(arith[0]);
    });
    // CF-3: the identical program.
    let cf3 = edited(&factual, |_| {});

    // Mask-confinement lens (G1's second half): outside the statement's
    // CallMask, factual and CF-1 project identically; inside, they differ.
    let mut stmt_mask = CallMask::empty(SHAPE);
    for i in 0..b.call_count {
        stmt_mask.set((b.first_call + i) as u32);
    }
    let outside = stmt_mask.not();
    let (slab_f, slab_1) = (slab_of(&factual), slab_of(&cf1a));
    let same_outside = project(&slab_f, SHAPE, &outside)
        .zip(project(&slab_1, SHAPE, &outside))
        .all(|((_, a), (_, b))| a == b);
    let diff_inside = project(&slab_f, SHAPE, &stmt_mask)
        .zip(project(&slab_1, SHAPE, &stmt_mask))
        .filter(|((_, a), (_, b))| a != b)
        .count();
    assert!(
        same_outside,
        "G1 FAIL: the edit leaked outside its statement mask"
    );
    assert!(
        diff_inside > 0,
        "G1 FAIL: the masked statement shows no edit"
    );
    println!(
        "   G1 PASS: edit confined — outside mask identical, {} differing calls inside (statement {s_star})",
        diff_inside
    );

    // ── Streams + three-tier distances ───────────────────────────────────
    let w_f = windows(&factual, &bounds, &arith);
    let c_f = coarsen(&w_f);
    let a_f = surrogate_areas(&c_f.regs, c_f.scale);
    let z_f = zeros_like(&a_f);

    let tier = |cf: &FunctionBody, name: &str| -> (usize, f64, f64, f64) {
        let bounds_cf = statement_bounds(&vocab, cf).expect("counterfactual segments");
        let w = windows(cf, &bounds_cf, &arith);
        let c = coarsen(&w);
        let a = surrogate_areas(&c.regs, c.scale);
        let z = zeros_like(&a);
        let byte = byte_tier_diff(&slab_of(&factual), &slab_of(cf));
        let incr = nk_dist(&c_f, &z_f, &c, &z);
        let exact = nk_dist(&c_f, &c_f.areas_exact, &c, &c.areas_exact);
        let reg = nk_dist(&c_f, &a_f, &c, &a);
        println!(
            "   {name}: byte {byte} | increment {incr:.3e} | exact-area {exact:.3e} | register {reg:.3e}"
        );
        (byte, incr, exact, reg)
    };

    println!("\n== 2. four-tier visibility of semantic counterfactuals ==");
    let (b1a, i1a, e1a, r1a) = tier(&cf1a, "CF-1a intra-run operand swap    ");
    let (b1b, i1b, e1b, r1b) = tier(&cf1b, "CF-1b cross-midpoint swap       ");
    let (b2, i2, e2, r2) = tier(&cf2, "CF-2 IntSub→IntAdd (rule)       ");
    let (b3, i3, e3, r3) = tier(&cf3, "CF-3 identical (silence)        ");

    // G2a — the fine dataflow edit: exact areas see it; the register's
    // midpoint-granular orientation bit is BLIND to it (named limit #1);
    // increments blind by construction. The probe's first formulation
    // expected the register to see this and was falsified — an intra-run
    // swap never crosses the midpoint the orientation bit encodes.
    assert!(b1a > 0, "G2a FAIL: byte tier blind");
    assert!(
        i1a.abs() < 1e-12,
        "G2a FAIL: increments see a pure reordering ({i1a:.3e})"
    );
    assert!(
        e1a > 1e-9,
        "G2a FAIL: exact areas blind to a real order change ({e1a:.3e})"
    );
    assert!(
        r1a.abs() < 1e-12,
        "G2a FAIL: register claims sub-midpoint resolution it does not have ({r1a:.3e})"
    );
    println!(
        "   G2a PASS: intra-run dataflow edit — exact areas see it; the register's midpoint bit is honestly blind"
    );

    // G2b — the midpoint-crossing dataflow edit: NOW the register sees it.
    assert!(b1b > 0, "G2b FAIL: byte tier blind");
    assert!(
        i1b.abs() < 1e-12,
        "G2b FAIL: increments see a pure reordering ({i1b:.3e})"
    );
    assert!(e1b > 1e-9, "G2b FAIL: exact areas blind ({e1b:.3e})");
    assert!(
        r1b > 1e-9,
        "G2b FAIL: register blind to a midpoint-crossing swap ({r1b:.3e})"
    );
    println!(
        "   G2b PASS: midpoint-crossing dataflow edit — register sees it; increments provably blind"
    );

    // G3 — the rule counterfactual below locus resolution: ONLY bytes see
    // (named limit #2 — IntSub and IntAdd share the arithmetic locus).
    assert!(b2 > 0, "G3 FAIL: byte tier blind to the substitution");
    assert!(
        i2.abs() < 1e-12 && e2.abs() < 1e-12 && r2.abs() < 1e-12,
        "G3 FAIL: a stream tier claims to see below locus resolution (i {i2:.3e} e {e2:.3e} r {r2:.3e})"
    );
    println!(
        "   G3 PASS: rule counterfactual below locus granularity — visible ONLY at the byte tier"
    );

    // G4 — silence.
    assert!(
        b3 == 0 && i3.abs() < 1e-12 && e3.abs() < 1e-12 && r3.abs() < 1e-12,
        "G4 FAIL: phantom distance on identity"
    );
    println!("   G4 PASS: identical program reads zero on every tier");

    println!(
        "\nPROBE GREEN — visibility ladder measured on semantic program edits: \
byte ⊇ exact-area ⊇ register ⊇ increment; the register's two blind spots \
(locus granularity, midpoint granularity) are named, not hidden."
    );
}
