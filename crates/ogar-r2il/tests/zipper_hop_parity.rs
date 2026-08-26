// SPDX-License-Identifier: Apache-2.0

//! **PROBE-ZIPPER-HOP-PARITY** — the falsifier for the §7.8 zipper
//! isomorphism (lance-graph `r2il-machine-semantic-contract-v1.md`):
//!
//! > An address is a pure straight-line program. One zipper descent step =
//! > one hop-law application (`Mask × ClassView → Mask`). R2IL's pure
//! > fragment and masked rail descent are ONE algebra.
//!
//! The claim is testable as bit-parity between two *independently shaped*
//! implementations over the same population:
//!
//! - **Native hop** — the "compiled" form: one fused prefix comparison per
//!   row (`(facet ⊕ path) ∧ prefix_mask == 0` on the u128 register view).
//!   This is the shape a baked closure has: the composed mask of the whole
//!   descent, one operation.
//! - **Replayed program** — the "interpreted" form: the SAME descent as an
//!   explicit straight-line program of real [`R2ILFn`] ops
//!   (`Load` / `IntEqual` / `BoolAnd`, one triple per zipper level),
//!   executed op-at-a-time by a tiny masked interpreter of the PURE
//!   fragment.
//!
//! Green ⇒ navigation and execution are one algebra (the interpreter needs
//! no special case for addresses). Red ⇒ the zipper reading is demoted to
//! poetry before anything is built on it.
//!
//! # Zero-copy discipline (operator nudge, 2026-08-26)
//!
//! V3 and V4 are both zero-copy; serialization exists only in the intake
//! arm. This probe honours that end to end: the population is ONE byte
//! slab, rows are [`FacetCascade::ref_from_bytes`] reinterpret views (a
//! no-op, `repr(C, align(16))`), `Load` reads the facet register in place,
//! and intermediate values live in `unique` space (never-persisted
//! scratch, the one legal materialization R2IL's space model already
//! names). Both sides answer with MASKS. Nothing is serialized, gathered,
//! or turned into a row-id list at any point.
//!
//! # Space mapping exercised (§7.8)
//!
//! `register` = the row's own facet register (read in place) · `unique` =
//! the interpreter's scratch stack · no `Store` — the pure fragment has
//! none, which is exactly THE FENCE: an address with side effects would be
//! behavior riding the address (T2 / SURREAL-AST trap).
//!
//! # Disable-run log (each verified red-then-green before commit)
//!
//! | falsifier | disable | observed red |
//! |---|---|---|
//! | parity (out-of-order rows) | `IntEqual` compares `lo` byte only | half-pair row admitted by replay, parity fails |
//! | parity (partial-prefix rows) | `BoolAnd` executed as OR | level-0-only row admitted by replay, parity fails |
//! | parity (depth) | replay skips the last level | deeper-mismatch row admitted, parity fails |
//! | table is load-bearing | interpreter dispatch by mnemonic string without `R2ILFn::ordinal` | core-byte refusal test fails |
//! | tail word | mask words sized `n/64` (floor) | rows past the last full word never match |

use lance_graph_contract::facet::{FacetCascade, FacetTier};
use ogar_loco::FnIndex;
use ogar_r2il::R2ILFn;

// ── the probe-local mask harness ─────────────────────────────────────────
//
// Deliberately NOT `CallMask` (that universe is calls-in-a-slab, lengths
// 180/120/90 — the wrong population) and deliberately not a new public
// surface: the RowFocusMask-vs-AlphaMask reconciliation is an open operator
// question, and a probe must not pre-empt it. A probe builds its own
// instrument; this one is ~30 lines of word algebra.

#[derive(Clone, PartialEq, Eq, Debug)]
struct RowMask {
    words: Vec<u64>,
    len: u32,
}

impl RowMask {
    fn empty(len: u32) -> Self {
        RowMask {
            words: vec![0; len.div_ceil(64) as usize],
            len,
        }
    }
    fn all(len: u32) -> Self {
        let mut m = Self::empty(len);
        let n = m.words.len();
        for (i, w) in m.words.iter_mut().enumerate() {
            *w = !0u64;
            if i + 1 == n && !len.is_multiple_of(64) {
                *w = (1u64 << (len % 64)) - 1;
            }
        }
        m
    }
    fn set(&mut self, i: u32) {
        assert!(i < self.len);
        self.words[(i / 64) as usize] |= 1 << (i % 64);
    }
    fn contains(&self, i: u32) -> bool {
        i < self.len && self.words[(i / 64) as usize] >> (i % 64) & 1 == 1
    }
    fn and(&self, o: &Self) -> Self {
        assert_eq!(self.len, o.len);
        RowMask {
            words: self
                .words
                .iter()
                .zip(&o.words)
                .map(|(a, b)| a & b)
                .collect(),
            len: self.len,
        }
    }
    fn count(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }
}

// ── the population: one slab, reinterpret views ──────────────────────────

/// `n` deliberately `% 64 != 0` so the tail word is live in every test.
const N: u32 = 197;
const PATH: [FacetTier; 3] = [
    FacetTier { lo: 0x21, hi: 0x07 },
    FacetTier { lo: 0x33, hi: 0x0A },
    FacetTier { lo: 0x5C, hi: 0x01 },
];

/// Build the slab so that per-level, per-byte, in-order matching is
/// LOAD-BEARING — a replay that ORs levels, compares only `lo`, or skips
/// the last level admits rows the native hop refuses (each shape below is
/// the anti-vacuity guard for one disable in the table above).
fn population() -> Vec<u8> {
    let mut slab = Vec::with_capacity(N as usize * 16);
    for i in 0..N {
        let tiers: [FacetTier; 6] = match i % 8 {
            // full-prefix match (the rows both sides must ADMIT)
            0 => [PATH[0], PATH[1], PATH[2], t(i), t(i), t(i)],
            // level-0 only — catches AND→OR
            1 => [PATH[0], t(i), t(i), t(i), t(i), t(i)],
            // levels 0..1 only, wrong at 2 — catches a skipped last level
            2 => [PATH[0], PATH[1], t(i.wrapping_add(9)), t(i), t(i), t(i)],
            // out-of-order: the path's pairs present, wrong levels — catches
            // any unordered-set reading of the zipper
            3 => [PATH[1], PATH[2], PATH[0], t(i), t(i), t(i)],
            // half-pair: lo matches at every level, hi never — catches an
            // IntEqual that compares only the low byte
            4 => [
                FacetTier {
                    lo: PATH[0].lo,
                    hi: 0xEE,
                },
                FacetTier {
                    lo: PATH[1].lo,
                    hi: 0xEE,
                },
                FacetTier {
                    lo: PATH[2].lo,
                    hi: 0xEE,
                },
                t(i),
                t(i),
                t(i),
            ],
            // unrelated
            _ => [
                t(i),
                t(i.wrapping_add(1)),
                t(i.wrapping_add(2)),
                t(i),
                t(i),
                t(i),
            ],
        };
        let f = FacetCascade {
            facet_classid: 0xC604_0000 | i, // machine_memory_map-prefixed, per row
            tiers,
        };
        slab.extend_from_slice(f.as_bytes());
    }
    slab
}

fn t(i: u32) -> FacetTier {
    FacetTier {
        lo: (i as u8) ^ 0x5A,
        hi: (i >> 3) as u8 | 0x80,
    }
}

/// Zero-copy row view: reinterpret, never decode.
fn row(slab: &[u8], i: u32) -> &FacetCascade {
    let b: &[u8; 16] = slab[i as usize * 16..][..16].try_into().unwrap();
    FacetCascade::ref_from_bytes(b).expect("16B aligned slab row")
}

// ── side A: the native hop (the "compiled" composed form) ────────────────

/// One fused prefix comparison per row on the u128 register view — the
/// shape a baked closure has. Bytes `[4 .. 4 + 2·depth)` are the rail
/// prefix; classid excluded (the codebook selector, not the path).
fn native_hop_mask(slab: &[u8], depth: usize) -> RowMask {
    let mut path_bytes = [0u8; 16];
    for (l, p) in PATH.iter().enumerate().take(depth) {
        path_bytes[4 + 2 * l] = p.lo;
        path_bytes[4 + 2 * l + 1] = p.hi;
    }
    let path = u128::from_le_bytes(path_bytes);
    let mut prefix = 0u128;
    for byte in 4..4 + 2 * depth {
        prefix |= 0xFFu128 << (8 * byte);
    }
    let mut m = RowMask::empty(N);
    for i in 0..N {
        if (row(slab, i).as_u128() ^ path) & prefix == 0 {
            m.set(i);
        }
    }
    m
}

// ── side B: the replayed program (the "interpreted" form) ────────────────

/// The pure-fragment straight-line program: one (Load, IntEqual, BoolAnd)
/// triple per zipper level. Operands: `Load` carries the level (register-
/// space address); `IntEqual` carries the immediate 8:8 pair.
fn descent_program(depth: usize) -> Vec<(FnIndex, u16)> {
    let op = |name: &str| {
        let ord = R2ILFn::MNEMONICS.iter().position(|m| *m == name).unwrap();
        R2ILFn::from_ordinal(ord).unwrap().0
    };
    let mut prog = Vec::new();
    for (l, p) in PATH.iter().enumerate().take(depth) {
        prog.push((op("Load"), l as u16));
        prog.push((op("IntEqual"), p.as_u16()));
        prog.push((op("BoolAnd"), 0));
    }
    prog
}

/// A value in `unique` space: a per-row u16 column (Load result) or a mask
/// (predicate / fold state). Scratch only — dropped at program end, never
/// persisted; the pure fragment has no `Store` by construction.
enum Unique {
    Column(Vec<u16>),
    Mask(RowMask),
}

/// Op-at-a-time masked interpretation of the pure fragment. Every op is
/// dispatched THROUGH the real table (`R2ILFn::ordinal` + mnemonic) — an
/// index outside the R2IL range is refused, so the vocabulary is
/// load-bearing, not decoration.
fn interpret(slab: &[u8], prog: &[(FnIndex, u16)]) -> Result<RowMask, String> {
    let mut stack: Vec<Unique> = vec![Unique::Mask(RowMask::all(N))];
    let mut executed = 0usize;
    for &(f, imm) in prog {
        let ord = R2ILFn::ordinal(f).ok_or_else(|| format!("not an R2IL op: {f:?}"))?;
        match R2ILFn::MNEMONICS[ord] {
            "Load" => {
                // register space, read in place: tier `imm` of every row.
                let col = (0..N)
                    .map(|i| row(slab, i).tiers[imm as usize].as_u16())
                    .collect();
                stack.push(Unique::Column(col));
            }
            "IntEqual" => {
                let Some(Unique::Column(col)) = stack.pop() else {
                    return Err("IntEqual expects a column".into());
                };
                let mut m = RowMask::empty(N);
                for (i, v) in col.iter().enumerate() {
                    if *v == imm {
                        m.set(i as u32);
                    }
                }
                stack.push(Unique::Mask(m));
            }
            "BoolAnd" => {
                let (Some(Unique::Mask(a)), Some(Unique::Mask(b))) = (stack.pop(), stack.pop())
                else {
                    return Err("BoolAnd expects two masks".into());
                };
                stack.push(Unique::Mask(a.and(&b)));
            }
            other => return Err(format!("op outside the pure descent fragment: {other}")),
        }
        executed += 1;
    }
    assert_eq!(
        executed,
        prog.len(),
        "the interpreter must execute every op — a fused shortcut would make parity vacuous"
    );
    match (stack.pop(), stack.is_empty()) {
        (Some(Unique::Mask(m)), true) => Ok(m),
        _ => Err("program did not reduce to one mask".into()),
    }
}

// ── the falsifiers ───────────────────────────────────────────────────────

#[test]
fn replayed_program_mask_is_bit_identical_to_the_native_hop() {
    let slab = population();
    let native = native_hop_mask(&slab, 3);
    let replay = interpret(&slab, &descent_program(3)).unwrap();
    // Anti-vacuity: the admitted set is non-trivial in BOTH directions —
    // some rows match (the probe can fire) and most do not (it can stay
    // silent). 25 full-prefix rows out of 197.
    assert!(native.count() > 0, "no row matched — the fixture is broken");
    assert!(
        native.count() * 3 < N,
        "almost everything matched — the traps are not trapping"
    );
    assert_eq!(
        native, replay,
        "the zipper reading FAILED: one descent, two answers"
    );
}

#[test]
fn every_trap_row_shape_is_actually_refused() {
    // The population's rows 1..5 (mod 8) each exist to catch one disable.
    // Prove they are genuinely refused by BOTH sides — otherwise the
    // parity test could go green while a trap sits inert.
    let slab = population();
    let native = native_hop_mask(&slab, 3);
    let replay = interpret(&slab, &descent_program(3)).unwrap();
    for i in 0..N {
        let should_match = i % 8 == 0;
        assert_eq!(native.contains(i), should_match, "native row {i}");
        assert_eq!(replay.contains(i), should_match, "replay row {i}");
    }
}

#[test]
fn a_divergent_path_yields_a_different_mask() {
    // Can-fail half: descend only 2 of 3 levels — the partial-prefix rows
    // (i % 8 == 2) are now legitimately admitted, so the shallow mask must
    // be a strict superset of the deep one.
    let slab = population();
    let deep = native_hop_mask(&slab, 3);
    let shallow = interpret(&slab, &descent_program(2)).unwrap();
    assert!(
        shallow.count() > deep.count(),
        "depth changed nothing — vacuous"
    );
    // and the shallow replay still agrees with the shallow native hop:
    assert_eq!(shallow, native_hop_mask(&slab, 2));
}

#[test]
fn the_interpreter_refuses_an_op_outside_the_r2il_table() {
    // A core byte (below the domain floor) must be refused BY THE TABLE —
    // dispatching on mnemonic strings alone could not fail here, which is
    // what makes the vocabulary load-bearing.
    let slab = population();
    let err = interpret(&slab, &[(FnIndex(0x10), 0)]).unwrap_err();
    assert!(err.contains("not an R2IL op"), "{err}");
    // …and an R2IL op outside the pure fragment is refused as such
    // (Store IS in the table but not in the descent algebra — THE FENCE).
    let store = R2ILFn::MNEMONICS
        .iter()
        .position(|m| *m == "Store")
        .unwrap();
    let err = interpret(&slab, &[(R2ILFn::from_ordinal(store).unwrap().0, 0)]).unwrap_err();
    assert!(err.contains("outside the pure descent fragment"), "{err}");
}

#[test]
fn the_tail_word_carries_no_phantom_rows() {
    // N = 197 = 3×64 + 5: the last word is partial. No bit at or past N may
    // ever be set, on either side (a floor-sized word vec, or an
    // unmasked `all()`, both die here).
    let slab = population();
    for m in [
        native_hop_mask(&slab, 3),
        interpret(&slab, &descent_program(3)).unwrap(),
        RowMask::all(N),
    ] {
        let tail = *m.words.last().unwrap();
        assert_eq!(tail >> (N % 64), 0, "phantom rows past N in the tail word");
        assert_eq!(m.words.len() as u32, N.div_ceil(64));
    }
}
