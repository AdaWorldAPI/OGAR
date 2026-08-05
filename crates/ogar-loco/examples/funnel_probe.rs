//! PROBE-ORACLE-FUNNEL Stage 0 — does the funnel discriminate?
//!
//! Pre-registration: lance-graph `.claude/plans/oracle-funnel-probe-v1.md`
//! (expectations E1–E4 written before this ran). Three deterministic
//! generator arms bracket what an LLM could emit; the shipped gates
//! (`validate` → `statement_bounds`) refuse or pass; `FunnelTally` (W-4)
//! reports which gate fired. No LLM, no API, no new mints — the empty-domain
//! blockly shared core only.
//!
//! Honest boundary: bodies are ≤16 calls under `LaneShape::Pairs` with
//! single-byte immediates, so the body-ENTRY gate (`BodyError`) is
//! structurally excluded here — Stage 0 exercises the segmentation gate.
//!
//! Run: `cargo run -p ogar-loco --example funnel_probe`

use ogar_loco::statements::{StatementError, statement_bounds};
use ogar_loco::telemetry::{FunnelTally, RefusalGate};
use ogar_loco::vocabulary::conformance::validate;
use ogar_loco::{Call, FnIndex, FunctionBody, LaneShape, Vocabulary};

const N: usize = 1000;
const MAX_LEN: u64 = 16;
const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// SplitMix64 — the workspace's deterministic sampler (certification-officer
/// convention). Zero-dep, bit-exact across runs.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..bound` (modulo bias negligible at these bounds vs u64).
    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

struct EmptyVocab;
impl Vocabulary for EmptyVocab {
    fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
        None
    }
    fn domain_body_refs(&self, _f: FnIndex) -> u8 {
        0
    }
}

fn main() {
    let checked = validate(EmptyVocab).expect("blockly shared core validates");
    let table = checked.table();

    // The segmentable set: slots declaring BOTH arity and pushes_result.
    let segmentable: Vec<FnIndex> = (0..=u8::MAX)
        .map(FnIndex)
        .filter(|f| table.stack_arity(*f).is_some() && table.pushes_result(*f).is_some())
        .collect();
    println!("segmentable slots: {}", segmentable.len());

    // ── Legend (the Stage-1 prompt anchor): named+covered slots only ──
    let mut legend = String::new();
    for f in (0..=u8::MAX).map(FnIndex) {
        if let (Some(name), Some(arity), Some(pushes)) = (
            table.spec(f).name,
            table.stack_arity(f),
            table.pushes_result(f),
        ) {
            legend.push_str(&format!(
                "0x{:02X} {name} arity={arity} pushes={pushes}\n",
                f.0
            ));
        }
    }
    let lines = legend.lines().count();
    println!(
        "legend: {} lines, {} bytes, ~{} tokens (bytes/4 estimate)\n",
        lines,
        legend.len(),
        legend.len() / 4
    );

    type Gen = fn(&mut SplitMix64, &[FnIndex], &ogar_loco::VocabularyTable) -> Vec<Call>;
    let arms: [(&str, Gen); 3] = [
        ("R (floor: uniform bytes)", gen_uniform),
        ("G (mid: legend-constrained)", gen_legend),
        ("W (ceiling: stack-aware walk)", gen_walk),
    ];

    for (label, generate) in arms {
        let mut rng = SplitMix64(SEED);
        let mut tally = FunnelTally::default();
        let mut stmt_counts: Vec<usize> = Vec::new();
        for _ in 0..N {
            let calls = generate(&mut rng, &segmentable, table);
            let body = FunctionBody::from_calls(LaneShape::Pairs, &calls)
                .expect("constructions stay inside the Pairs budget");
            match statement_bounds(&checked, &body) {
                Ok(stmts) => {
                    tally.record(Ok(()));
                    stmt_counts.push(stmts.len());
                }
                Err(e) => tally.record(Err(RefusalGate::from(&e))),
            }
        }
        println!("arm {label}: {}/{} survived", tally.survived, tally.total());
        for (gate, n) in tally.nonzero_gates() {
            println!("  {gate:?}: {n}");
        }
        if !stmt_counts.is_empty() {
            let sum: usize = stmt_counts.iter().sum();
            let max = stmt_counts.iter().max().copied().unwrap_or(0);
            println!(
                "  survivor statements: mean {:.2}, max {max}",
                sum as f64 / stmt_counts.len() as f64
            );
        }
        println!();
    }

    // Two-sided check on the mapping itself: one known-refused body per gate
    // must land on the RIGHT gate (the funnel's own can-it-fire test).
    let checked2 = validate(EmptyVocab).unwrap();
    let underflow = FunctionBody::from_calls(LaneShape::Pairs, &[Call::new(FnIndex::ADD)]).unwrap();
    assert!(matches!(
        statement_bounds(&checked2, &underflow),
        Err(StatementError::StackUnderflow { .. })
    ));
    println!("gate-identity spot check: StackUnderflow lands on StackUnderflow — ok");
}

/// Arm R: every byte uniform — models an LLM that knows nothing.
fn gen_uniform(
    rng: &mut SplitMix64,
    _seg: &[FnIndex],
    _table: &ogar_loco::VocabularyTable,
) -> Vec<Call> {
    let len = 1 + rng.below(MAX_LEN) as usize;
    (0..len)
        .map(|_| Call::with_value(FnIndex(rng.below(256) as u8), rng.below(256) as u8))
        .collect()
}

/// Arm G: calls uniform over the segmentable set — read the legend, not the
/// grammar.
fn gen_legend(
    rng: &mut SplitMix64,
    seg: &[FnIndex],
    _table: &ogar_loco::VocabularyTable,
) -> Vec<Call> {
    let len = 1 + rng.below(MAX_LEN) as usize;
    (0..len)
        .map(|_| {
            let f = seg[rng.below(seg.len() as u64) as usize];
            Call::with_value(f, rng.below(256) as u8)
        })
        .collect()
}

/// Arm W: stack-aware — only calls whose arity fits the current depth; stops
/// at a statement boundary (depth 0) or a single trailing value (depth 1).
fn gen_walk(
    rng: &mut SplitMix64,
    seg: &[FnIndex],
    table: &ogar_loco::VocabularyTable,
) -> Vec<Call> {
    let target = 1 + rng.below(MAX_LEN) as usize;
    let mut calls = Vec::new();
    let mut depth = 0usize;
    while calls.len() < target || depth > 1 {
        let eligible: Vec<FnIndex> = seg
            .iter()
            .copied()
            .filter(|f| usize::from(table.stack_arity(*f).unwrap()) <= depth)
            .collect();
        let f = eligible[rng.below(eligible.len() as u64) as usize];
        depth -= usize::from(table.stack_arity(f).unwrap());
        if table.pushes_result(f).unwrap() {
            depth += 1;
        }
        calls.push(Call::with_value(f, rng.below(256) as u8));
        // Safety valve so a run of pushes can always be consumed within a
        // bounded body: past the target, prefer any depth-reducing step.
        if calls.len() > target + 32 {
            break;
        }
    }
    calls
}
