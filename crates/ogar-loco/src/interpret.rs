//! The **orchestration engine** — control flow, and deliberately nothing else.
//!
//! # Why this owns so little
//!
//! This crate's promise is that it *"can speak all dialects"*. An engine that
//! knew what `ADD` meant could only ever speak one. So the split here is
//! total: [`Interpreter`] owns the program counter, the branch decisions, the
//! loop bounds and the recursion into referenced bodies, and a [`Dialect`]
//! owns **every semantic** — what a value is, when one is true, what a
//! non-branching call does.
//!
//! Blockly plugs in with `i64` arithmetic. A thinking dialect plugs in with
//! masks and truth values, whose primitives are cheap for a reason — the
//! masking algebra exists to make the reasoning tactics cheap, which is what
//! lets a tactic be a *program here* rather than a bespoke function
//! somewhere below. The engine never learns which dialect it is running.
//!
//! # What it owns, exactly
//!
//! Five control-flow functions: [`FnIndex::IF`], [`FnIndex::IF_ELSE`],
//! [`FnIndex::REPEAT`], [`FnIndex::WHILE`], [`FnIndex::REPEAT_UNTIL`].
//!
//! That is not the whole control-flow block, and the gap is deliberate.
//! `FOREVER`, `BREAK`, `CONTINUE`, `STOP`, `RETURN`, `FOR_EACH`, `FOR_RANGE`
//! and `PROC_DEF` are **refused**, not guessed
//! ([`RunError::UnhandledControlFlow`]). These five are the set an earlier
//! probe ran against real algorithms — iterative GCD, summation, nested
//! classification, Collatz step-counting — each checked against an
//! independent reference implementation. Shipping the rest would be shipping
//! behaviour nothing has executed. They land when a falsifier lands with
//! them.
//!
//! # The loop subtlety, which is easy to lose
//!
//! A `WHILE`'s condition is not a separate body. It is the calls immediately
//! *preceding* the loop call in the same body, already evaluated by the
//! forward walk that reached it. So re-testing means re-running that local
//! span — [`Interpreter::operand_span_start`] walks backwards over the
//! arities to find where it begins. An implementation that re-tested the
//! stack top without re-running the span would loop forever on the first
//! truthy condition, and would pass any test whose loop runs zero or one
//! times.

use crate::vocabulary::conformance::CheckedVocabulary;
use crate::{Call, FnIndex, FunctionBody, Program, Vocabulary};

/// Default ceiling on iterations of one loop call, before
/// [`RunError::IterationCap`].
///
/// A cap rather than trust: a `WHILE` whose dialect never falsifies its
/// condition is a hang, and a hang in an orchestration layer is
/// indistinguishable from work.
pub const DEFAULT_ITERATION_CAP: u32 = 100_000;

/// Everything the engine refuses to know.
///
/// The three reads are separated on purpose. `truthy` is a branch decision,
/// `repeat_count` is a loop bound, and they are different questions a dialect
/// may answer differently — a mask is "true" when it is non-empty, and its
/// repeat count is its population.
pub trait Dialect {
    /// What this dialect's stack holds.
    type Value;
    /// What its calls can fail with.
    type Error;

    /// Does this value branch a conditional?
    fn truthy(&self, value: &Self::Value) -> bool;

    /// How many times should a [`FnIndex::REPEAT`] carrying this value run?
    fn repeat_count(&self, value: &Self::Value) -> u32;

    /// Execute one NON-branching call: pop its operands, push its result.
    ///
    /// The engine has already established that `f` does not branch. Whether
    /// `f` is a shared-core byte or one of this dialect's own is the
    /// dialect's business — the engine does not look.
    fn call(
        &mut self,
        f: FnIndex,
        values: [u8; 3],
        stack: &mut Vec<Self::Value>,
    ) -> Result<(), Self::Error>;
}

/// Why a run stopped short.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RunError<E> {
    /// The dialect refused a call.
    Dialect(E),
    /// A control-flow call wanted an operand and the stack was empty.
    StackUnderflow {
        /// The call that asked.
        call: FnIndex,
    },
    /// The vocabulary does not cover this call's arity, so the engine cannot
    /// know how many operands it consumes — and a wrong arity does not give a
    /// slightly wrong answer, it reattributes every later operand.
    UncoveredArity {
        /// The call whose arity is unknown.
        call: FnIndex,
    },
    /// A branch named a function index the program does not contain.
    UnresolvedBody {
        /// The branching call.
        call: FnIndex,
        /// The index it named.
        target: u8,
    },
    /// A loop ran past [`Interpreter::iteration_cap`].
    IterationCap {
        /// The loop call.
        call: FnIndex,
        /// The cap it exceeded.
        cap: u32,
    },
    /// A control-flow call outside the five this engine executes. Refused
    /// rather than approximated — see the module doc.
    UnhandledControlFlow {
        /// The call that branches but has no execution rule here.
        call: FnIndex,
    },
}

/// A running program: the engine's own state, plus the dialect's.
pub struct Interpreter<'a, V: Vocabulary, D: Dialect> {
    vocab: &'a CheckedVocabulary<V>,
    program: &'a Program,
    dialect: D,
    stack: Vec<D::Value>,
    iteration_cap: u32,
}

impl<'a, V: Vocabulary, D: Dialect> Interpreter<'a, V, D> {
    /// Bind a dialect to a program.
    pub fn new(vocab: &'a CheckedVocabulary<V>, program: &'a Program, dialect: D) -> Self {
        Self {
            vocab,
            program,
            dialect,
            stack: Vec::new(),
            iteration_cap: DEFAULT_ITERATION_CAP,
        }
    }

    /// Replace the per-loop iteration ceiling.
    pub fn with_iteration_cap(mut self, cap: u32) -> Self {
        self.iteration_cap = cap;
        self
    }

    /// The ceiling a loop may not exceed.
    pub fn iteration_cap(&self) -> u32 {
        self.iteration_cap
    }

    /// Borrow the dialect (its own state is where results usually land).
    pub fn dialect(&self) -> &D {
        &self.dialect
    }

    /// Borrow the operand stack.
    pub fn stack(&self) -> &[D::Value] {
        &self.stack
    }

    /// Run the program's entry function.
    pub fn run(&mut self) -> Result<(), RunError<D::Error>> {
        self.run_function(0)
    }

    /// Run one function body to completion.
    fn run_function(&mut self, index: usize) -> Result<(), RunError<D::Error>> {
        let Some(body) = self.program.functions.get(index) else {
            return Ok(());
        };
        let mut pc = 0usize;
        while let Some(call) = body.call(pc) {
            let f = call.function;
            if !self.vocab.table().branches(f) {
                self.dialect
                    .call(f, call.values, &mut self.stack)
                    .map_err(RunError::Dialect)?;
                pc += 1;
                continue;
            }
            self.run_branching(body, pc, call)?;
            pc += 1;
        }
        Ok(())
    }

    /// Execute one branching call at `pc`.
    fn run_branching(
        &mut self,
        body: &FunctionBody,
        pc: usize,
        call: Call,
    ) -> Result<(), RunError<D::Error>> {
        let f = call.function;
        let arity = self
            .vocab
            .table()
            .stack_arity(f)
            .ok_or(RunError::UncoveredArity { call: f })?;

        match f {
            FnIndex::IF => {
                let cond = self.pop(f)?;
                if self.dialect.truthy(&cond) {
                    self.branch(f, call.values[0])?;
                }
            }
            FnIndex::IF_ELSE => {
                let cond = self.pop(f)?;
                let target = if self.dialect.truthy(&cond) {
                    call.values[0]
                } else {
                    call.values[1]
                };
                self.branch(f, target)?;
            }
            FnIndex::REPEAT => {
                let n = self.pop(f)?;
                let count = self.dialect.repeat_count(&n).min(self.iteration_cap);
                for _ in 0..count {
                    self.branch(f, call.values[0])?;
                }
            }
            FnIndex::WHILE | FnIndex::REPEAT_UNTIL => {
                let until = f == FnIndex::REPEAT_UNTIL;
                // Where the condition's own calls begin — re-running THEM is
                // what makes the next test a new test.
                let cond_start = self.operand_span_start(body, pc, arity)?;
                let mut iters = 0u32;
                loop {
                    let cond = self.pop(f)?;
                    let truthy = self.dialect.truthy(&cond);
                    if truthy == until {
                        break;
                    }
                    self.branch(f, call.values[0])?;
                    iters += 1;
                    if iters >= self.iteration_cap {
                        return Err(RunError::IterationCap {
                            call: f,
                            cap: self.iteration_cap,
                        });
                    }
                    for i in cond_start..pc {
                        let c = body.call(i).expect("in bounds: span already walked");
                        if self.vocab.table().branches(c.function) {
                            return Err(RunError::UnhandledControlFlow { call: c.function });
                        }
                        self.dialect
                            .call(c.function, c.values, &mut self.stack)
                            .map_err(RunError::Dialect)?;
                    }
                }
            }
            other => return Err(RunError::UnhandledControlFlow { call: other }),
        }
        Ok(())
    }

    /// Recurse into the body a branch names.
    fn branch(&mut self, call: FnIndex, target: u8) -> Result<(), RunError<D::Error>> {
        let idx = usize::from(target);
        if idx == 0 || idx >= self.program.functions.len() {
            return Err(RunError::UnresolvedBody { call, target });
        }
        self.run_function(idx)
    }

    fn pop(&mut self, call: FnIndex) -> Result<D::Value, RunError<D::Error>> {
        self.stack.pop().ok_or(RunError::StackUnderflow { call })
    }

    /// Walk backwards from `end` over `want` operands' worth of calls.
    ///
    /// Each step consumes one needed operand and adds back that call's own
    /// arity, so a nested expression is counted whole rather than by depth.
    fn operand_span_start(
        &self,
        body: &FunctionBody,
        end: usize,
        want: u8,
    ) -> Result<usize, RunError<D::Error>> {
        let table = self.vocab.table();
        let mut need = usize::from(want);
        let mut i = end;
        while need > 0 {
            if i == 0 {
                // The body does not contain the operands this call claims —
                // a malformed program, refused rather than clamped to 0,
                // which would silently re-run the whole body as a condition.
                return Err(RunError::StackUnderflow {
                    call: body.call(end).map(|c| c.function).unwrap_or(FnIndex::NOP),
                });
            }
            i -= 1;
            let c = body.call(i).expect("in bounds: walked forward over these");
            let arity = table
                .stack_arity(c.function)
                .ok_or(RunError::UncoveredArity { call: c.function })?;
            need -= 1;
            need += usize::from(arity);
        }
        Ok(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::conformance::validate;
    use crate::{Call, LaneShape};

    /// No domain bytes — the shared core alone, which is all these programs use.
    struct CoreOnly;
    impl Vocabulary for CoreOnly {
        fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
            None
        }
        fn domain_body_refs(&self, _f: FnIndex) -> u8 {
            0
        }
    }

    /// The smallest dialect that can run an algorithm: `i64`, 256 variables.
    ///
    /// Deliberately a TEST dialect, not a shipped one. The engine's claim is
    /// that it carries no semantics; proving that needs *a* dialect, not *the*
    /// dialect, and shipping one here would be the engine learning a language.
    struct I64Dialect {
        vars: [i64; 256],
        calls: u32,
    }

    // `[i64; 256]` has no `Default` (arrays implement it only up to 32), so
    // the derive cannot be used here.
    impl Default for I64Dialect {
        fn default() -> Self {
            Self {
                vars: [0; 256],
                calls: 0,
            }
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum I64Error {
        Underflow(u8),
        Unsupported(u8),
    }

    impl Dialect for I64Dialect {
        type Value = i64;
        type Error = I64Error;

        fn truthy(&self, v: &i64) -> bool {
            *v != 0
        }

        fn repeat_count(&self, v: &i64) -> u32 {
            u32::try_from(*v).unwrap_or(0)
        }

        fn call(
            &mut self,
            f: FnIndex,
            values: [u8; 3],
            stack: &mut Vec<i64>,
        ) -> Result<(), I64Error> {
            self.calls += 1;
            let pop = |s: &mut Vec<i64>| s.pop().ok_or(I64Error::Underflow(f.0));
            match f {
                FnIndex::NUMBER => stack.push(i64::from(values[0])),
                FnIndex::ADD => {
                    let (b, a) = (pop(stack)?, pop(stack)?);
                    stack.push(a + b);
                }
                FnIndex::SUB => {
                    let (b, a) = (pop(stack)?, pop(stack)?);
                    stack.push(a - b);
                }
                FnIndex::MOD => {
                    let (b, a) = (pop(stack)?, pop(stack)?);
                    stack.push(if b == 0 { 0 } else { a % b });
                }
                FnIndex::GT => {
                    let (b, a) = (pop(stack)?, pop(stack)?);
                    stack.push(i64::from(a > b));
                }
                FnIndex::VAR_GET => stack.push(self.vars[usize::from(values[0])]),
                FnIndex::VAR_SET => {
                    let v = pop(stack)?;
                    self.vars[usize::from(values[0])] = v;
                }
                other => return Err(I64Error::Unsupported(other.0)),
            }
            Ok(())
        }
    }

    fn run(program: &Program) -> Result<I64Dialect, RunError<I64Error>> {
        let vocab = validate(CoreOnly).expect("core-only vocabulary conforms");
        let mut interp = Interpreter::new(&vocab, program, I64Dialect::default());
        interp.run()?;
        Ok(interp.dialect)
    }

    /// `sum 1..=n` with `REPEAT`: var0 = total, var1 = counter.
    fn sum_program(n: u8) -> Program {
        let entry = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, 0),
                Call::with_value(FnIndex::VAR_SET, 0),
                Call::with_value(FnIndex::NUMBER, 0),
                Call::with_value(FnIndex::VAR_SET, 1),
                Call::with_value(FnIndex::NUMBER, n),
                Call::with_value(FnIndex::REPEAT, 1),
            ],
        )
        .unwrap();
        let body = FunctionBody::from_calls(
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
            functions: vec![entry, body],
        }
    }

    /// Euclid's GCD with `WHILE` — var0 = a, var1 = b, loop while b > 0.
    ///
    /// The condition is `VAR_GET 1, NUMBER 0, GT` — three calls sitting
    /// immediately before the `WHILE`, in the SAME body. Re-testing means
    /// re-running them, which is the whole point of this fixture.
    fn gcd_program(a: u8, b: u8) -> Program {
        let entry = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, a),
                Call::with_value(FnIndex::VAR_SET, 0),
                Call::with_value(FnIndex::NUMBER, b),
                Call::with_value(FnIndex::VAR_SET, 1),
                Call::with_value(FnIndex::VAR_GET, 1),
                Call::with_value(FnIndex::NUMBER, 0),
                Call::new(FnIndex::GT),
                Call::with_value(FnIndex::WHILE, 1),
            ],
        )
        .unwrap();
        let body = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::VAR_GET, 1),
                Call::with_value(FnIndex::VAR_GET, 0),
                Call::with_value(FnIndex::VAR_GET, 1),
                Call::new(FnIndex::MOD),
                Call::with_value(FnIndex::VAR_SET, 1),
                Call::with_value(FnIndex::VAR_SET, 0),
            ],
        )
        .unwrap();
        Program {
            functions: vec![entry, body],
        }
    }

    fn gcd_ref(mut a: i64, mut b: i64) -> i64 {
        while b > 0 {
            let t = a % b;
            a = b;
            b = t;
        }
        a
    }

    /// FAILS IF: `REPEAT` runs the wrong number of times, or the engine loses
    /// the dialect's state across a branch. Checked against closed form, not
    /// against a second run of the same engine.
    #[test]
    fn repeat_sums_to_the_closed_form() {
        for n in [0u8, 1, 2, 7, 50] {
            let d = run(&sum_program(n)).expect("runs");
            let want = i64::from(n) * (i64::from(n) + 1) / 2;
            assert_eq!(d.vars[0], want, "sum 1..={n}");
        }
    }

    /// FAILS IF: the condition's operand span is not re-run between
    /// iterations — the loop then re-tests a stale stack top and either spins
    /// to the iteration cap or exits after one pass. Both are visible here
    /// because GCD needs several iterations and has an independent reference.
    ///
    /// The `(8, 0)` case is the silent half: a condition false on the FIRST
    /// test, so the body must never run.
    #[test]
    fn while_reruns_its_condition_span_and_computes_gcd() {
        for (a, b) in [(48u8, 18u8), (17, 5), (100, 75), (8, 0), (1, 1)] {
            let d = run(&gcd_program(a, b)).expect("runs");
            assert_eq!(
                d.vars[0],
                gcd_ref(i64::from(a), i64::from(b)),
                "gcd({a}, {b})"
            );
        }
    }

    /// FAILS IF: a runaway loop hangs instead of being refused. The dialect
    /// here never falsifies the condition, which is exactly the shape a real
    /// mis-written program has.
    #[test]
    fn a_loop_that_never_falsifies_hits_the_cap_rather_than_hanging() {
        let entry = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, 1),
                Call::with_value(FnIndex::WHILE, 1),
            ],
        )
        .unwrap();
        // The body pushes the same truthy constant every pass.
        let body =
            FunctionBody::from_calls(LaneShape::Pairs, &[Call::with_value(FnIndex::NUMBER, 1)])
                .unwrap();
        let p = Program {
            functions: vec![entry, body],
        };
        let vocab = validate(CoreOnly).expect("conforms");
        let mut interp = Interpreter::new(&vocab, &p, I64Dialect::default()).with_iteration_cap(64);
        assert_eq!(
            interp.run(),
            Err(RunError::IterationCap {
                call: FnIndex::WHILE,
                cap: 64
            })
        );
    }

    /// FAILS IF: a branch to a function the program does not contain is
    /// followed rather than refused — silently running nothing, which reads
    /// as a program that simply did not do much.
    #[test]
    fn a_branch_to_a_missing_body_is_refused() {
        let entry = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, 1),
                Call::with_value(FnIndex::IF, 9),
            ],
        )
        .unwrap();
        let p = Program {
            functions: vec![entry],
        };
        assert_eq!(
            run(&p).err(),
            Some(RunError::UnresolvedBody {
                call: FnIndex::IF,
                target: 9
            })
        );
        // silent half: index 0 is the entry and is never a legal target
        let entry0 = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, 1),
                Call::with_value(FnIndex::IF, 0),
            ],
        )
        .unwrap();
        assert!(matches!(
            run(&Program {
                functions: vec![entry0]
            })
            .err(),
            Some(RunError::UnresolvedBody { target: 0, .. })
        ));
    }

    /// FAILS IF: a control-flow function outside the proven five is executed
    /// on a guess. `FOR_EACH` branches, so the engine reaches it — and must
    /// say so rather than approximate it.
    #[test]
    fn an_unproven_control_flow_call_is_refused_not_approximated() {
        let entry = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, 1),
                Call::with_value(FnIndex::FOR_EACH, 1),
            ],
        )
        .unwrap();
        let body =
            FunctionBody::from_calls(LaneShape::Pairs, &[Call::with_value(FnIndex::NUMBER, 1)])
                .unwrap();
        assert_eq!(
            run(&Program {
                functions: vec![entry, body]
            })
            .err(),
            Some(RunError::UnhandledControlFlow {
                call: FnIndex::FOR_EACH
            })
        );
    }

    /// FAILS IF: the engine executes a shared-core byte itself instead of
    /// asking the dialect. A dialect that refuses EVERYTHING must make even
    /// `NUMBER` fail — if it does not, the engine is carrying semantics.
    #[test]
    fn the_engine_carries_no_semantics_of_its_own() {
        struct RefuseAll;
        impl Dialect for RefuseAll {
            type Value = i64;
            type Error = ();
            fn truthy(&self, _: &i64) -> bool {
                true
            }
            fn repeat_count(&self, _: &i64) -> u32 {
                0
            }
            fn call(&mut self, _: FnIndex, _: [u8; 3], _: &mut Vec<i64>) -> Result<(), ()> {
                Err(())
            }
        }
        let entry =
            FunctionBody::from_calls(LaneShape::Pairs, &[Call::with_value(FnIndex::NUMBER, 5)])
                .unwrap();
        let p = Program {
            functions: vec![entry],
        };
        let vocab = validate(CoreOnly).expect("conforms");
        let mut interp = Interpreter::new(&vocab, &p, RefuseAll);
        assert_eq!(interp.run(), Err(RunError::Dialect(())));
    }

    /// The silent twin of the test above, and the half that was missing.
    ///
    /// FAILS IF: the engine routes a control-flow call to the dialect. The
    /// split is a claim in BOTH directions — the dialect sees every
    /// non-branching call (above) and never sees a branching one (here) — and
    /// a suite that only pins the first would pass an engine that handed
    /// `IF` to the dialect and let it improvise a branch.
    #[test]
    fn control_flow_never_reaches_the_dialect() {
        /// Records every call the engine delegates.
        #[derive(Default)]
        struct Recorder {
            seen: Vec<u8>,
        }
        impl Dialect for Recorder {
            type Value = i64;
            type Error = ();
            fn truthy(&self, v: &i64) -> bool {
                *v != 0
            }
            fn repeat_count(&self, v: &i64) -> u32 {
                u32::try_from(*v).unwrap_or(0)
            }
            fn call(
                &mut self,
                f: FnIndex,
                values: [u8; 3],
                stack: &mut Vec<i64>,
            ) -> Result<(), ()> {
                self.seen.push(f.0);
                if f == FnIndex::NUMBER {
                    stack.push(i64::from(values[0]));
                }
                Ok(())
            }
        }

        // NUMBER 1, IF -> body(1); NUMBER 2, REPEAT -> body(1).
        let entry = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, 1),
                Call::with_value(FnIndex::IF, 1),
                Call::with_value(FnIndex::NUMBER, 2),
                Call::with_value(FnIndex::REPEAT, 1),
            ],
        )
        .unwrap();
        let body =
            FunctionBody::from_calls(LaneShape::Pairs, &[Call::with_value(FnIndex::NUMBER, 9)])
                .unwrap();
        let p = Program {
            functions: vec![entry, body],
        };
        let vocab = validate(CoreOnly).expect("conforms");
        let mut interp = Interpreter::new(&vocab, &p, Recorder::default());
        interp.run().expect("runs");

        let seen = &interp.dialect.seen;
        // Anti-vacuity: the program really did execute control flow — the IF
        // branched once and the REPEAT ran twice, so the body's NUMBER 9 ran
        // three times in total. Without this the silence below would hold for
        // a program that never branched at all.
        assert_eq!(
            seen.iter().filter(|b| **b == FnIndex::NUMBER.0).count(),
            5,
            "2 in the entry + 3 from the branched body: {seen:?}"
        );
        for cf in [FnIndex::IF, FnIndex::REPEAT] {
            assert!(
                !seen.contains(&cf.0),
                "control flow {cf:?} was handed to the dialect: {seen:?}"
            );
        }
    }
}
