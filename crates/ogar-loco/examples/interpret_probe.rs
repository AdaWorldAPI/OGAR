//! PROBE-LOCO-INTERPRETER-1 — does the shared computational core actually
//! execute, and does execution produce non-trivial, input-dependent traces?
//!
//! Pre-registration (lance-graph `.claude/brainstorms/
//! 2026-08-22-behavioral-ir-fathoming.md` §F): before this ran, the workspace
//! had a byte-exact, validated, branch-capable instruction format
//! (`ogar_loco::Call` / `FunctionBody` / `Program`) and **no interpreter
//! anywhere** — a crate-wide search for `fn execute`/`eval`/`interpret`/
//! `step`/`run` returned nothing, and this crate's own `telemetry.rs` states
//! it "only knows whether a candidate parses, casts, and segments." This
//! probe is the smallest experiment that can kill or advance the hypothesis
//! that the substrate is a learnable behavioural micro-IR: build the missing
//! interpreter, run it over a **bounded, real, hand-authored corpus** (four
//! classic small algorithms, several real inputs each), and check the four
//! pre-registered kill conditions honestly.
//!
//! # Scope — what this probe does NOT attempt
//!
//! This interpreter executes only the **shared computational core**
//! (`FnIndex` values below [`ogar_loco::DOMAIN_FLOOR`]) — arithmetic, logic,
//! comparisons, variables, and the four control-flow shapes used below
//! (`IF`, `IF_ELSE`, `REPEAT`, `WHILE`). It does **not** interpret the 34
//! NARS recipes lance-graph-ogar mints above the floor: those op bytes have
//! no generic semantics an `ogar-loco`-only interpreter can execute — their
//! meaning lives in `lance-graph-ogar`'s `ThoughtCtx`/`recipe_dispatch`
//! wiring, which this probe does not pull in. **Kill condition 1 (the 34
//! recipes' effects are separable) is therefore explicitly UNTESTED here,
//! not passed** — see the printed report's final line.
//!
//! No BPE, no macro learner, no policy. This probe only asks: does the IR
//! execute, deterministically, with real branching?
//!
//! # Finding surfaced while building this (not pre-registered, discovered)
//!
//! The shared core's own declared table (`vocabulary::shared_core::
//! pushes_result`) marks `VAR_SET`/`VAR_CHANGE` as **pushing** a result —
//! true chainable-assignment semantics. That makes `ogar_loco::statements::
//! statement_bounds` (the crate's whole-body segmentation used for
//! step-mask dispatch) correctly REFUSE (`DanglingOperands`) any body
//! containing an ordinary imperative "set a; set b; …" sequence, because
//! nothing in the shared core consumes the leftover pushed values — there is
//! no `DROP`/`POP` primitive. This is not a probe bug; it is a genuine,
//! previously-untested property of the ABI (nobody had built a real
//! multi-statement program against it before). This interpreter therefore
//! does NOT use `statement_bounds` to drive execution — it treats `VAR_SET`/
//! `VAR_CHANGE` as void statements (see `exec_and_trace`) and walks each
//! function body as one linear program-counter pass, using its own local
//! backward operand-span scan (`operand_span_start`) only where a loop
//! genuinely needs to re-run a condition. `statement_bounds`'s refusal on
//! these bodies is real and orthogonal — it is answering a masking
//! question this probe never asks.
//!
//! # No new deps, no fabricated corpus
//!
//! Four classic algorithms with independently-known-correct answers (GCD,
//! a summation, FizzBuzz-style classification, Collatz step counts) —
//! chosen because their control flow is real and their answers are checkable
//! against arithmetic, not against a label this probe invents. Every input
//! is a small literal chosen by hand, not sampled.
//!
//! Run: `cargo run -p ogar-loco --example interpret_probe`

use ogar_loco::program::{Program, branches_of};
use ogar_loco::vocabulary::conformance::{CheckedVocabulary, validate};
use ogar_loco::{Call, FnIndex, FunctionBody, LaneShape, Vocabulary};

/// The smallest conforming vocabulary — shared core only, nothing above the
/// floor. This probe never touches vocabulary-specific bytes.
struct EmptyVocab;
impl Vocabulary for EmptyVocab {
    fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
        None
    }
    fn domain_body_refs(&self, _f: FnIndex) -> u8 {
        0
    }
}

/// Safety cap on WHILE/REPEAT_UNTIL iterations — a defensive guard against an
/// interpreter bug hanging the harness. Never intended to fire: every program
/// below is hand-verified to terminate at these inputs. If it fires, that is
/// an interpreter defect, reported as such, never silently swallowed.
const ITER_CAP: u32 = 100_000;

/// One traced call — the format PROBE-LOCO-INTERPRETER-1 was pre-registered
/// to record: `(mailbox_owner, seq, FnIndex, values, pre_version,
/// post_version, outcome)`.
///
/// `mailbox_owner` and the version fields are explicit STAND-INS: this probe
/// runs outside any real `MailboxSoA`/Lance-version wiring, so `owner` is a
/// fixed synthetic label and `pre_version`/`post_version` are the trace's own
/// monotonic call ordinal (post = pre + 1, always) — never claimed to be a
/// real Lance version horizon. Recorded honestly as what they are.
#[derive(Debug, Clone)]
#[allow(dead_code)] // recorded for honesty per the pre-registered trace shape; not
// all fields are read back by this probe's own report — see the doc comment.
struct TraceEvent {
    mailbox_owner: &'static str,
    seq: u32,
    function: FnIndex,
    name: &'static str,
    values: [u8; 3],
    pre_version: u32,
    post_version: u32,
}

#[derive(Debug)]
#[allow(dead_code)] // `Debug`-formatted only, on panic — fields are not read
// programmatically, but naming the offending call in a panic message is why
// each variant carries one.
enum ExecError {
    StackUnderflow { call: FnIndex },
    DivByZero { call: FnIndex },
    IterationCapExceeded,
    Unhandled { call: FnIndex },
}

struct Interpreter<'a> {
    v: &'a CheckedVocabulary<EmptyVocab>,
    prog: &'a Program,
    stack: Vec<i64>,
    vars: [i64; 256],
    trace: Vec<TraceEvent>,
    seq: u32,
}

impl<'a> Interpreter<'a> {
    fn new(v: &'a CheckedVocabulary<EmptyVocab>, prog: &'a Program) -> Self {
        Self {
            v,
            prog,
            stack: Vec::new(),
            vars: [0i64; 256],
            trace: Vec::new(),
            seq: 0,
        }
    }

    fn pop(&mut self, call: FnIndex) -> Result<i64, ExecError> {
        self.stack.pop().ok_or(ExecError::StackUnderflow { call })
    }

    /// Execute one non-control-flow call's real effect on the stack/vars, and
    /// record its trace event. Control-flow calls (branches() == true) are
    /// handled by `run_function`'s dispatch, never here. `VAR_SET`/
    /// `VAR_CHANGE` are treated as VOID here (pop, mutate, push nothing) —
    /// see the module doc's "Finding" section for why that is a deliberate
    /// deviation from the shared core's declared `pushes_result` table.
    fn exec_and_trace(&mut self, call: Call) -> Result<(), ExecError> {
        let pre = self.seq;
        use FnIndex as F;
        match call.function {
            F::NUMBER | F::TRUE => self.stack.push(i64::from(call.values[0])),
            F::FALSE => self.stack.push(0),
            F::VAR_GET => self.stack.push(self.vars[usize::from(call.values[0])]),
            F::VAR_SET => {
                let val = self.pop(call.function)?;
                self.vars[usize::from(call.values[0])] = val;
            }
            F::VAR_CHANGE => {
                let delta = self.pop(call.function)?;
                self.vars[usize::from(call.values[0])] += delta;
            }
            F::ADD => {
                let b = self.pop(call.function)?;
                let a = self.pop(call.function)?;
                self.stack.push(a + b);
            }
            F::SUB => {
                let b = self.pop(call.function)?;
                let a = self.pop(call.function)?;
                self.stack.push(a - b);
            }
            F::MUL => {
                let b = self.pop(call.function)?;
                let a = self.pop(call.function)?;
                self.stack.push(a * b);
            }
            F::DIV => {
                let b = self.pop(call.function)?;
                let a = self.pop(call.function)?;
                if b == 0 {
                    return Err(ExecError::DivByZero {
                        call: call.function,
                    });
                }
                self.stack.push(a / b);
            }
            F::MOD => {
                let b = self.pop(call.function)?;
                let a = self.pop(call.function)?;
                if b == 0 {
                    return Err(ExecError::DivByZero {
                        call: call.function,
                    });
                }
                self.stack.push(a % b);
            }
            F::EQ => {
                let b = self.pop(call.function)?;
                let a = self.pop(call.function)?;
                self.stack.push(i64::from(a == b));
            }
            F::NEQ => {
                let b = self.pop(call.function)?;
                let a = self.pop(call.function)?;
                self.stack.push(i64::from(a != b));
            }
            F::LT => {
                let b = self.pop(call.function)?;
                let a = self.pop(call.function)?;
                self.stack.push(i64::from(a < b));
            }
            F::LTE => {
                let b = self.pop(call.function)?;
                let a = self.pop(call.function)?;
                self.stack.push(i64::from(a <= b));
            }
            F::GT => {
                let b = self.pop(call.function)?;
                let a = self.pop(call.function)?;
                self.stack.push(i64::from(a > b));
            }
            F::GTE => {
                let b = self.pop(call.function)?;
                let a = self.pop(call.function)?;
                self.stack.push(i64::from(a >= b));
            }
            F::AND => {
                let b = self.pop(call.function)?;
                let a = self.pop(call.function)?;
                self.stack.push(i64::from(a != 0 && b != 0));
            }
            F::OR => {
                let b = self.pop(call.function)?;
                let a = self.pop(call.function)?;
                self.stack.push(i64::from(a != 0 || b != 0));
            }
            F::NOT => {
                let a = self.pop(call.function)?;
                self.stack.push(i64::from(a == 0));
            }
            other => return Err(ExecError::Unhandled { call: other }),
        }
        self.record(call, pre);
        Ok(())
    }

    /// Record a decision call (IF/IF_ELSE/REPEAT/WHILE) that popped its
    /// operand(s) in the dispatch match arm rather than in `exec_and_trace`.
    fn record(&mut self, call: Call, pre: u32) {
        self.seq += 1;
        self.trace.push(TraceEvent {
            mailbox_owner: "probe-owner-0",
            seq: pre,
            function: call.function,
            name: ogar_loco::vocabulary::shared_core::name(call.function).unwrap_or("?"),
            values: call.values,
            pre_version: pre,
            post_version: self.seq,
        });
    }

    /// Find the start index of the LOCAL operand-producing span that
    /// supplies exactly `want` values to the call ending at `end` (exclusive)
    /// — a backward postfix-expression-tree reconstruction: `need` starts at
    /// `want`; walking backward, each call visited must be one that pushes
    /// exactly one result (true of every shared-core expression op other
    /// than `VAR_SET`/`VAR_CHANGE`, which this interpreter treats as void —
    /// see `exec_and_trace`), so it satisfies one unit of `need` and in turn
    /// itself demands `arity` more.
    ///
    /// This is deliberately NOT `ogar_loco::statements::statement_bounds`.
    /// That function segments the WHOLE body under the crate's declared
    /// `pushes_result` table (where `VAR_SET` DOES push, for chainable-
    /// assignment use) and rightly REFUSES (`DanglingOperands`) the moment an
    /// earlier, unrelated void-assignment statement leaves a value nobody
    /// reads — a real, load-bearing property for the masking use case it
    /// exists for. This probe surfaced that refusal empirically (see the
    /// module doc's "Finding" note) and needed a narrower question: not "can
    /// this whole body be segmented for masking" but "which calls does THIS
    /// ONE control call's condition need re-run, ignoring whatever unrelated
    /// garbage sits deeper in the stack." The backward scan below answers
    /// exactly that, locally, and does not require the whole body to be
    /// maskable.
    fn operand_span_start(&self, body: &FunctionBody, end: usize, want: u8) -> usize {
        let table = self.v.table();
        let mut need = usize::from(want);
        let mut i = end;
        while need > 0 {
            i -= 1;
            let call = body
                .call(i)
                .expect("in bounds: end was reached by walking forward over these same calls");
            let arity = table
                .stack_arity(call.function)
                .expect("already executed once by the forward walk that reached `end`");
            need -= 1;
            need += usize::from(arity);
        }
        i
    }

    /// Run function `func_idx` to completion: a single linear walk over the
    /// body's calls with a program counter, executing each call once in
    /// order (a plain stack machine — exactly what the shared-core doc
    /// describes: "each call pops its operands and, for expressions, pushes
    /// a result"). Control-flow calls branch into sibling function bodies;
    /// `WHILE`/`REPEAT_UNTIL` additionally re-run their condition's local
    /// operand span (via `operand_span_start`) before each re-test. Returns
    /// the final stack top as the function's "return value" if the body ends
    /// on a trailing expression (nothing consumed it), `None` if it ends on
    /// a control-flow call.
    fn run_function(&mut self, func_idx: usize) -> Result<Option<i64>, ExecError> {
        let body = &self.prog.functions[func_idx];
        let mut pc = 0usize;
        let mut last_top: Option<i64> = None;
        while pc < body.len() {
            let call = body.call(pc).expect("pc kept in range by the loop guard");
            let f = call.function;

            if !self.v.branches(f) {
                self.exec_and_trace(call)?;
                last_top = self.stack.last().copied();
                pc += 1;
                continue;
            }

            let arity = self
                .v
                .table()
                .stack_arity(f)
                .ok_or(ExecError::Unhandled { call: f })?;

            use FnIndex as F;
            match f {
                F::IF => {
                    let pre = self.seq;
                    let cond = self.pop(f)?;
                    self.record(call, pre);
                    if cond != 0 {
                        self.run_function(usize::from(call.values[0]))?;
                    }
                }
                F::IF_ELSE => {
                    let pre = self.seq;
                    let cond = self.pop(f)?;
                    self.record(call, pre);
                    let target = if cond != 0 {
                        call.values[0]
                    } else {
                        call.values[1]
                    };
                    self.run_function(usize::from(target))?;
                }
                F::REPEAT => {
                    let pre = self.seq;
                    let count = self.pop(f)?;
                    self.record(call, pre);
                    for _ in 0..count.max(0) {
                        self.run_function(usize::from(call.values[0]))?;
                    }
                }
                F::WHILE | F::REPEAT_UNTIL => {
                    // The condition was already computed by the forward walk
                    // that just reached `pc` — pop it for the first test.
                    // `cond_start` is only needed to RE-run the condition's
                    // own local span on later iterations.
                    let cond_start = self.operand_span_start(body, pc, arity);
                    let mut iters = 0u32;
                    loop {
                        let pre = self.seq;
                        let cond = self.pop(f)?;
                        self.record(call, pre);
                        let keep_going = if f == F::WHILE { cond != 0 } else { cond == 0 };
                        if !keep_going {
                            break;
                        }
                        self.run_function(usize::from(call.values[0]))?;
                        iters += 1;
                        if iters > ITER_CAP {
                            return Err(ExecError::IterationCapExceeded);
                        }
                        for i in cond_start..pc {
                            let c = body
                                .call(i)
                                .expect("in bounds: within the span just walked");
                            self.exec_and_trace(c)?;
                        }
                    }
                }
                other => return Err(ExecError::Unhandled { call: other }),
            }
            last_top = None;
            pc += 1;
        }
        Ok(last_top)
    }
}

// ── Program builders (hand-authored, real algorithms) ──────────────────────

/// `entry(a0, b0)`: iterative Euclidean GCD. vars: a=0, b=1, t=2.
fn build_gcd(a0: u8, b0: u8) -> Program {
    let entry = FunctionBody::from_calls(
        LaneShape::Pairs,
        &[
            Call::with_value(FnIndex::NUMBER, a0),
            Call::with_value(FnIndex::VAR_SET, 0),
            Call::with_value(FnIndex::NUMBER, b0),
            Call::with_value(FnIndex::VAR_SET, 1),
            Call::with_value(FnIndex::VAR_GET, 1),
            Call::with_value(FnIndex::NUMBER, 0),
            Call::new(FnIndex::NEQ),
            Call::with_value(FnIndex::WHILE, 1),
            Call::with_value(FnIndex::VAR_GET, 0),
        ],
    )
    .unwrap();
    let loop_body = FunctionBody::from_calls(
        LaneShape::Pairs,
        &[
            Call::with_value(FnIndex::VAR_GET, 1),
            Call::with_value(FnIndex::VAR_SET, 2),
            Call::with_value(FnIndex::VAR_GET, 0),
            Call::with_value(FnIndex::VAR_GET, 1),
            Call::new(FnIndex::MOD),
            Call::with_value(FnIndex::VAR_SET, 1),
            Call::with_value(FnIndex::VAR_GET, 2),
            Call::with_value(FnIndex::VAR_SET, 0),
        ],
    )
    .unwrap();
    Program {
        functions: vec![entry, loop_body],
    }
}

/// `entry(n)`: sum 1..=n via REPEAT. vars: sum=0, i=1.
fn build_sum(n: u8) -> Program {
    let entry = FunctionBody::from_calls(
        LaneShape::Pairs,
        &[
            Call::with_value(FnIndex::NUMBER, 0),
            Call::with_value(FnIndex::VAR_SET, 0),
            Call::with_value(FnIndex::NUMBER, 0),
            Call::with_value(FnIndex::VAR_SET, 1),
            Call::with_value(FnIndex::NUMBER, n),
            Call::with_value(FnIndex::REPEAT, 1),
            Call::with_value(FnIndex::VAR_GET, 0),
        ],
    )
    .unwrap();
    let loop_body = FunctionBody::from_calls(
        LaneShape::Pairs,
        &[
            Call::with_value(FnIndex::VAR_GET, 1),
            Call::with_value(FnIndex::NUMBER, 1),
            Call::new(FnIndex::ADD),
            Call::with_value(FnIndex::VAR_SET, 1),
            Call::with_value(FnIndex::VAR_GET, 0),
            Call::with_value(FnIndex::VAR_GET, 1),
            Call::new(FnIndex::ADD),
            Call::with_value(FnIndex::VAR_SET, 0),
        ],
    )
    .unwrap();
    Program {
        functions: vec![entry, loop_body],
    }
}

/// `entry(n)`: FizzBuzz-shaped classification via nested IF_ELSE.
/// 3 => n%15==0, 1 => n%3==0, 2 => n%5==0, 0 => none. var: n=0, result=1.
fn build_fizz(n0: u8) -> Program {
    // functions: 0=entry, 1=then(%15), 2=else(%15) -> nested, 3=then(%3),
    // 4=else(%3) -> nested, 5=then(%5), 6=else(%5)
    let entry = FunctionBody::from_calls(
        LaneShape::Triples,
        &[
            Call::with_values(FnIndex::NUMBER, [n0, 0, 0]),
            Call::with_values(FnIndex::VAR_SET, [0, 0, 0]),
            Call::with_values(FnIndex::VAR_GET, [0, 0, 0]),
            Call::with_values(FnIndex::NUMBER, [15, 0, 0]),
            Call::new(FnIndex::MOD),
            Call::with_values(FnIndex::NUMBER, [0, 0, 0]),
            Call::new(FnIndex::EQ),
            Call::with_values(FnIndex::IF_ELSE, [1, 2, 0]),
            Call::with_values(FnIndex::VAR_GET, [1, 0, 0]),
        ],
    )
    .unwrap();
    let then15 = FunctionBody::from_calls(
        LaneShape::Triples,
        &[
            Call::with_values(FnIndex::NUMBER, [3, 0, 0]),
            Call::with_values(FnIndex::VAR_SET, [1, 0, 0]),
        ],
    )
    .unwrap();
    let else15 = FunctionBody::from_calls(
        LaneShape::Triples,
        &[
            Call::with_values(FnIndex::VAR_GET, [0, 0, 0]),
            Call::with_values(FnIndex::NUMBER, [3, 0, 0]),
            Call::new(FnIndex::MOD),
            Call::with_values(FnIndex::NUMBER, [0, 0, 0]),
            Call::new(FnIndex::EQ),
            Call::with_values(FnIndex::IF_ELSE, [3, 4, 0]),
        ],
    )
    .unwrap();
    let then3 = FunctionBody::from_calls(
        LaneShape::Triples,
        &[
            Call::with_values(FnIndex::NUMBER, [1, 0, 0]),
            Call::with_values(FnIndex::VAR_SET, [1, 0, 0]),
        ],
    )
    .unwrap();
    let else3 = FunctionBody::from_calls(
        LaneShape::Triples,
        &[
            Call::with_values(FnIndex::VAR_GET, [0, 0, 0]),
            Call::with_values(FnIndex::NUMBER, [5, 0, 0]),
            Call::new(FnIndex::MOD),
            Call::with_values(FnIndex::NUMBER, [0, 0, 0]),
            Call::new(FnIndex::EQ),
            Call::with_values(FnIndex::IF_ELSE, [5, 6, 0]),
        ],
    )
    .unwrap();
    let then5 = FunctionBody::from_calls(
        LaneShape::Triples,
        &[
            Call::with_values(FnIndex::NUMBER, [2, 0, 0]),
            Call::with_values(FnIndex::VAR_SET, [1, 0, 0]),
        ],
    )
    .unwrap();
    let else5 = FunctionBody::from_calls(
        LaneShape::Triples,
        &[
            Call::with_values(FnIndex::NUMBER, [0, 0, 0]),
            Call::with_values(FnIndex::VAR_SET, [1, 0, 0]),
        ],
    )
    .unwrap();
    Program {
        functions: vec![entry, then15, else15, then3, else3, then5, else5],
    }
}

/// `entry(n0)`: Collatz step count. vars: n=0, steps=1.
fn build_collatz(n0: u8) -> Program {
    let entry = FunctionBody::from_calls(
        LaneShape::Pairs,
        &[
            Call::with_value(FnIndex::NUMBER, n0),
            Call::with_value(FnIndex::VAR_SET, 0),
            Call::with_value(FnIndex::NUMBER, 0),
            Call::with_value(FnIndex::VAR_SET, 1),
            Call::with_value(FnIndex::VAR_GET, 0),
            Call::with_value(FnIndex::NUMBER, 1),
            Call::new(FnIndex::NEQ),
            Call::with_value(FnIndex::WHILE, 1),
            Call::with_value(FnIndex::VAR_GET, 1),
        ],
    )
    .unwrap();
    let loop_body = FunctionBody::from_calls(
        LaneShape::Triples,
        &[
            Call::with_values(FnIndex::VAR_GET, [0, 0, 0]),
            Call::with_values(FnIndex::NUMBER, [2, 0, 0]),
            Call::new(FnIndex::MOD),
            Call::with_values(FnIndex::NUMBER, [0, 0, 0]),
            Call::new(FnIndex::EQ),
            Call::with_values(FnIndex::IF_ELSE, [2, 3, 0]),
            Call::with_values(FnIndex::VAR_GET, [1, 0, 0]),
            Call::with_values(FnIndex::NUMBER, [1, 0, 0]),
            Call::new(FnIndex::ADD),
            Call::with_values(FnIndex::VAR_SET, [1, 0, 0]),
        ],
    )
    .unwrap();
    let even = FunctionBody::from_calls(
        LaneShape::Pairs,
        &[
            Call::with_value(FnIndex::VAR_GET, 0),
            Call::with_value(FnIndex::NUMBER, 2),
            Call::new(FnIndex::DIV),
            Call::with_value(FnIndex::VAR_SET, 0),
        ],
    )
    .unwrap();
    let odd = FunctionBody::from_calls(
        LaneShape::Pairs,
        &[
            Call::with_value(FnIndex::VAR_GET, 0),
            Call::with_value(FnIndex::NUMBER, 3),
            Call::new(FnIndex::MUL),
            Call::with_value(FnIndex::NUMBER, 1),
            Call::new(FnIndex::ADD),
            Call::with_value(FnIndex::VAR_SET, 0),
        ],
    )
    .unwrap();
    Program {
        functions: vec![entry, loop_body, even, odd],
    }
}

/// One executed episode: builds the program fresh, validates every branch
/// resolves, runs it, returns (trace, final_value).
fn run_episode(
    v: &CheckedVocabulary<EmptyVocab>,
    prog: &Program,
) -> Result<(Vec<TraceEvent>, Option<i64>), ExecError> {
    assert!(
        prog.references_are_resolvable(v),
        "hand-authored program must resolve every branch — a bug in this probe, not a finding"
    );
    let mut interp = Interpreter::new(v, prog);
    let result = interp.run_function(0)?;
    Ok((interp.trace, result))
}

fn trace_signature(trace: &[TraceEvent]) -> Vec<(u8, [u8; 3])> {
    trace.iter().map(|e| (e.function.0, e.values)).collect()
}

fn main() {
    let v = validate(EmptyVocab).expect("shared core validates");

    println!("═══ PROBE-LOCO-INTERPRETER-1 ═══\n");
    println!(
        "Scope: shared computational core only (below DOMAIN_FLOOR=0x{:02X}).",
        ogar_loco::DOMAIN_FLOOR
    );
    println!("The 34 lance-graph-ogar recipes are NOT interpreted here — see the");
    println!("final report line for why, and what that means for kill condition 1.\n");

    let mut all_episode_lens: Vec<usize> = Vec::new();
    let mut all_signatures: Vec<Vec<(u8, [u8; 3])>> = Vec::new();
    let mut determinism_ok = true;
    let mut correctness_ok = true;

    // ── GCD ──────────────────────────────────────────────────────────────
    println!("── GCD (iterative Euclidean algorithm) ──");
    for &(a0, b0) in &[(48u8, 18u8), (200, 192), (17, 5), (100, 10), (7, 7)] {
        let expected = gcd_ref(a0, b0);
        let prog = build_gcd(a0, b0);
        let (trace1, r1) = run_episode(&v, &prog).unwrap();
        let (trace2, r2) = run_episode(&v, &prog).unwrap();
        let det = trace_signature(&trace1) == trace_signature(&trace2) && r1 == r2;
        determinism_ok &= det;
        let correct = r1 == Some(i64::from(expected));
        correctness_ok &= correct;
        println!(
            "  gcd({a0},{b0}) = {:?} (expected {expected}), calls={}, deterministic={det}, correct={correct}",
            r1,
            trace1.len()
        );
        all_episode_lens.push(trace1.len());
        all_signatures.push(trace_signature(&trace1));
    }

    // ── Sum 1..N ─────────────────────────────────────────────────────────
    println!("\n── sum 1..=N (bounded REPEAT) ──");
    for &n in &[5u8, 10, 20, 50] {
        let expected = u32::from(n) * (u32::from(n) + 1) / 2;
        let prog = build_sum(n);
        let (trace1, r1) = run_episode(&v, &prog).unwrap();
        let (trace2, r2) = run_episode(&v, &prog).unwrap();
        let det = trace_signature(&trace1) == trace_signature(&trace2) && r1 == r2;
        determinism_ok &= det;
        let correct = r1 == Some(i64::from(expected));
        correctness_ok &= correct;
        println!(
            "  sum(1..={n}) = {:?} (expected {expected}), calls={}, deterministic={det}, correct={correct}",
            r1,
            trace1.len()
        );
        all_episode_lens.push(trace1.len());
        all_signatures.push(trace_signature(&trace1));
    }

    // ── FizzBuzz classification ──────────────────────────────────────────
    println!("\n── FizzBuzz-shaped classification (nested IF_ELSE) ──");
    let mut fizz_class_counts = [0u32; 4];
    for n in 1u8..=30 {
        let expected = fizz_ref(n);
        let prog = build_fizz(n);
        let (trace1, r1) = run_episode(&v, &prog).unwrap();
        let (trace2, r2) = run_episode(&v, &prog).unwrap();
        let det = trace_signature(&trace1) == trace_signature(&trace2) && r1 == r2;
        determinism_ok &= det;
        let correct = r1 == Some(i64::from(expected));
        correctness_ok &= correct;
        fizz_class_counts[expected as usize] += 1;
        if !(1..=5).contains(&n) {
            // Keep console output bounded; still checked above for all 30.
        } else {
            println!(
                "  fizz({n}) = {:?} (expected {expected}), calls={}, deterministic={det}, correct={correct}",
                r1,
                trace1.len()
            );
        }
        all_episode_lens.push(trace1.len());
        all_signatures.push(trace_signature(&trace1));
    }
    println!(
        "  ... 30 inputs total. class counts [none,%3,%5,%15] = {:?} (n=1..=30 real math: 16/8/4/2)",
        fizz_class_counts
    );
    correctness_ok &= fizz_class_counts == [16, 8, 4, 2];

    // ── Collatz step count ───────────────────────────────────────────────
    println!("\n── Collatz step count (WHILE, input-dependent length) ──");
    for &n0 in &[1u8, 6, 7, 15, 27] {
        let expected = collatz_ref(u64::from(n0));
        let prog = build_collatz(n0);
        let (trace1, r1) = run_episode(&v, &prog).unwrap();
        let (trace2, r2) = run_episode(&v, &prog).unwrap();
        let det = trace_signature(&trace1) == trace_signature(&trace2) && r1 == r2;
        determinism_ok &= det;
        let correct = r1 == Some(i64::try_from(expected).expect("collatz step count fits i64"));
        correctness_ok &= correct;
        println!(
            "  collatz({n0}) steps = {:?} (expected {expected}), calls={}, deterministic={det}, correct={correct}",
            r1,
            trace1.len()
        );
        all_episode_lens.push(trace1.len());
        all_signatures.push(trace_signature(&trace1));
    }

    // ── branches_of sanity: confirm the IR's own branch-reporting agrees ──
    let gcd_prog = build_gcd(48, 18);
    let b = branches_of(&v, gcd_prog.entry());
    assert_eq!(b.len(), 1, "entry has exactly one branching call (WHILE)");
    assert_eq!(b[0].1.function, FnIndex::WHILE);

    // ── Kill-condition report ────────────────────────────────────────────
    println!("\n═══ Kill-condition report ═══");

    // KC2 — determinism under replay.
    println!(
        "KC2 (determinism under fixed inputs): {}",
        if determinism_ok {
            "PASS — every episode replayed byte-identical"
        } else {
            "FAIL — see per-episode output above"
        }
    );

    // KC3 — trivial trace length (median >= 5).
    let mut sorted_lens = all_episode_lens.clone();
    sorted_lens.sort_unstable();
    let median = sorted_lens[sorted_lens.len() / 2];
    println!(
        "KC3 (median episode length >= 5 calls): median={median} across {} episodes — {}",
        all_episode_lens.len(),
        if median >= 5 { "PASS" } else { "FAIL" }
    );

    // KC4 — traces are not all the same sequence (input-dependence, not a
    // static ladder_program()-shaped ordering).
    let mut distinct: Vec<&Vec<(u8, [u8; 3])>> = Vec::new();
    for sig in &all_signatures {
        if !distinct.contains(&sig) {
            distinct.push(sig);
        }
    }
    println!(
        "KC4 (traces are not all identical): {} distinct call sequences across {} episodes — {}",
        distinct.len(),
        all_signatures.len(),
        if distinct.len() > 1 { "PASS" } else { "FAIL" }
    );

    println!(
        "\nInterpreter correctness (arithmetic ground truth, independent of this probe): {}",
        if correctness_ok {
            "ALL EPISODES CORRECT"
        } else {
            "MISMATCH — see per-episode output"
        }
    );

    println!(
        "\nKC1 (the 34 lance-graph-ogar recipes have separable executable effects): \
         NOT TESTED. This probe interprets only the shared computational core; \
         the recipe vocabulary's semantics live in lance-graph-ogar's ThoughtCtx/ \
         recipe_dispatch wiring, out of scope for an ogar-loco-only interpreter. \
         Wiring that is the next required step, not a result this run can report."
    );
}

// ── Independent arithmetic ground truth (not derived from the interpreter) ──

fn gcd_ref(mut a: u8, mut b: u8) -> u8 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn fizz_ref(n: u8) -> u8 {
    if n.is_multiple_of(15) {
        3
    } else if n.is_multiple_of(3) {
        1
    } else if n.is_multiple_of(5) {
        2
    } else {
        0
    }
}

fn collatz_ref(mut n: u64) -> u64 {
    let mut steps = 0u64;
    while n != 1 {
        if n.is_multiple_of(2) {
            n /= 2;
        } else {
            n = n * 3 + 1;
        }
        steps += 1;
    }
    steps
}
