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

use crate::inventory::{FnAddr, Inventory};
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
    /// A branch recursed past [`Interpreter::recursion_depth`].
    ///
    /// Distinct from [`RunError::IterationCap`] on purpose: a loop that runs
    /// too long and a cycle of bodies calling each other are different
    /// defects in the program, and collapsing them would send an author
    /// looking at the wrong one.
    RecursionDepth {
        /// The branching call that would have exceeded the depth.
        call: FnIndex,
        /// The ceiling it hit.
        depth: u32,
    },
    /// A control-flow call outside the five this engine executes. Refused
    /// rather than approximated — see the module doc.
    UnhandledControlFlow {
        /// The call that branches but has no execution rule here.
        call: FnIndex,
    },
}

/// The shared core's control band: `0x01..=0x1F`.
///
/// Every byte in it is the ENGINE's to execute or refuse — never the
/// dialect's. Below it sits `NOP`; above it start the value families
/// (logic `0x20`, compare `0x30`, arithmetic `0x40`, variables `0x80`), and
/// past [`DOMAIN_FLOOR`] a dialect owns its own bytes outright.
const CONTROL_BAND: core::ops::RangeInclusive<u8> = 0x01..=0x1F;

/// Is `f` the engine's to handle?
///
/// # Why not `Vocabulary::branches`
///
/// Because that answers a DIFFERENT QUESTION, and its own doc says so:
/// *"this predicate answers 'does lowering this call require emitting another
/// function?', which is the only question a cast asks"* — a CODEGEN question.
/// It is `body_refs(f) > 0`, and `BREAK` / `CONTINUE` / `STOP` / `RETURN` /
/// `WAIT` reference no body, so they are `false` under it.
///
/// Used as an EXECUTION predicate it silently handed every one of them to
/// `Dialect::call`, where a dialect could invent a meaning for `BREAK` and
/// let the run report success — while this module's contract promised
/// [`RunError::UnhandledControlFlow`]. Two questions, one name, and the
/// vocabulary had already written the warning.
///
/// The band is the right discriminator: it is a property of the shared core's
/// own layout, it needs no per-byte list to drift, and a domain vocabulary
/// cannot forge it (`DOMAIN_FLOOR` is `0x90`).
fn is_engine_control(f: FnIndex) -> bool {
    CONTROL_BAND.contains(&f.0)
}

/// How deep [`Interpreter::branch`] may recurse before refusing.
///
/// Not a style limit — a safety one. `Program::references_are_resolvable`
/// rejects only target `0` and out-of-range targets; it does NOT reject
/// cycles, so two bodies that branch to each other pass validation and then
/// recurse until the native stack is gone. A stack overflow aborts the
/// process, which is the one failure an orchestration engine must never turn
/// a bad program into.
///
/// 64 is chosen against the substrate rather than taste: a continuation stack
/// is 90 quad slots in one node, so a depth past that could not be reified
/// anyway.
pub const DEFAULT_RECURSION_DEPTH: u32 = 64;

/// A running program: the engine's own state, plus the dialect's.
pub struct Interpreter<'a, V: Vocabulary, D: Dialect> {
    vocab: &'a CheckedVocabulary<V>,
    program: &'a Program,
    /// Where bodies come from, when a caller supplies one.
    ///
    /// `None` resolves against `program.functions`, which is what every
    /// pre-`Inventory` caller does and what keeps this change behaviour-neutral
    /// for them: a `VecInventory` built from a program's own `functions` has
    /// registration order as its address, so the two agree row for row.
    inventory: Option<&'a dyn Inventory>,
    dialect: D,
    stack: Vec<D::Value>,
    iteration_cap: u32,
    recursion_depth: u32,
    depth: u32,
}

impl<'a, V: Vocabulary, D: Dialect> Interpreter<'a, V, D> {
    /// Bind a dialect to a program.
    pub fn new(vocab: &'a CheckedVocabulary<V>, program: &'a Program, dialect: D) -> Self {
        Self {
            vocab,
            program,
            inventory: None,
            dialect,
            stack: Vec::new(),
            iteration_cap: DEFAULT_ITERATION_CAP,
            recursion_depth: DEFAULT_RECURSION_DEPTH,
            depth: 0,
        }
    }

    /// Resolve bodies through an [`Inventory`] instead of the program's own
    /// `functions` list.
    ///
    /// This is the consumer half of functions-as-objects: a node store, a Lance
    /// scan or a cache implements [`Inventory`] and the interpreter branches
    /// into bodies it has never seen inside a `Program`.
    ///
    /// ⊘ Landed after codex flagged, correctly, that `inventory.rs` shipped a
    /// trait no execution path could reach — `Interpreter::new` took only a
    /// `Program` and both resolution sites went through `program.functions`, so
    /// the advertised behaviour was unavailable to any caller. Two built ends
    /// that did not meet, which is the exact shape this session kept naming
    /// elsewhere.
    #[must_use]
    pub fn with_inventory(mut self, inventory: &'a dyn Inventory) -> Self {
        self.inventory = Some(inventory);
        self
    }

    /// The body at `index`, from whichever backing this interpreter resolves
    /// against. One place, so the two call sites cannot disagree.
    fn body_at(&self, index: usize) -> Option<&'a FunctionBody> {
        match self.inventory {
            Some(inv) => u16::try_from(index).ok().and_then(|i| inv.body(FnAddr(i))),
            None => self.program.functions.get(index),
        }
    }

    /// How many addresses the backing can answer — the bound `branch` checks.
    fn body_count(&self) -> usize {
        match self.inventory {
            Some(inv) => inv.len(),
            None => self.program.functions.len(),
        }
    }

    /// Replace the recursion ceiling.
    pub fn with_recursion_depth(mut self, depth: u32) -> Self {
        self.recursion_depth = depth;
        self
    }

    /// The depth a branch may not exceed.
    pub fn recursion_depth(&self) -> u32 {
        self.recursion_depth
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
        let Some(body) = self.body_at(index) else {
            return Ok(());
        };
        let mut pc = 0usize;
        while let Some(call) = body.call(pc) {
            let f = call.function;
            if !is_engine_control(f) {
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
        // The arity is resolved INSIDE the loop arms, not here: it is needed
        // only by the condition-span walk, and computing it up front made
        // `RETURN` — whose arity the core deliberately leaves uncovered —
        // report `UncoveredArity`, which points an author at the vocabulary
        // when the true answer is that this ENGINE does not execute the byte.
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
                let count = self.dialect.repeat_count(&n);
                // REFUSE, never clamp. `min` ran the capped prefix and
                // returned `Ok`, so a 200_000-iteration repeat reported
                // success having run 100_000 times — a wrong answer presented
                // as a right one, and inconsistent with WHILE, which reports
                // the cap in the same situation.
                if count > self.iteration_cap {
                    return Err(RunError::IterationCap {
                        call: f,
                        cap: self.iteration_cap,
                    });
                }
                for _ in 0..count {
                    self.branch(f, call.values[0])?;
                }
            }
            FnIndex::WHILE | FnIndex::REPEAT_UNTIL => {
                let until = f == FnIndex::REPEAT_UNTIL;
                let arity = self
                    .vocab
                    .table()
                    .stack_arity(f)
                    .ok_or(RunError::UncoveredArity { call: f })?;
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
                    // The cap is checked when ANOTHER iteration is requested,
                    // never after the last one ran. Checked afterwards it
                    // rejected a loop that terminates in exactly `cap`
                    // iterations — inside the advertised ceiling — and at
                    // `cap = 0` it ran the body once before refusing.
                    if iters >= self.iteration_cap {
                        return Err(RunError::IterationCap {
                            call: f,
                            cap: self.iteration_cap,
                        });
                    }
                    self.branch(f, call.values[0])?;
                    iters += 1;
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
        if idx == 0 || idx >= self.body_count() {
            return Err(RunError::UnresolvedBody { call, target });
        }
        if self.depth >= self.recursion_depth {
            return Err(RunError::RecursionDepth {
                call,
                depth: self.recursion_depth,
            });
        }
        self.depth += 1;
        let r = self.run_function(idx);
        // Restored on the error path too: an interpreter a caller inspects
        // after a failure would otherwise report a depth that never unwound,
        // and one reused after a caught error would refuse legal programs.
        self.depth -= 1;
        r
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
            // A call satisfies one operand only if it PRODUCES one. The
            // non-pushing set is not hypothetical: it is every control-flow
            // byte in the shared core, plus whatever void verbs a domain
            // declares. Crediting one of those with an operand stops the walk
            // early and drops the call that actually produces the condition.
            if table
                .pushes_result(c.function)
                .ok_or(RunError::UncoveredArity { call: c.function })?
            {
                need -= 1;
            }
            need += usize::from(arity);
        }
        Ok(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::VecInventory;
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
                FnIndex::MUL => {
                    let (b, a) = (pop(stack)?, pop(stack)?);
                    stack.push(a * b);
                }
                FnIndex::DIV => {
                    let (b, a) = (pop(stack)?, pop(stack)?);
                    stack.push(if b == 0 { 0 } else { a / b });
                }
                FnIndex::MOD => {
                    let (b, a) = (pop(stack)?, pop(stack)?);
                    stack.push(if b == 0 { 0 } else { a % b });
                }
                FnIndex::EQ => {
                    let (b, a) = (pop(stack)?, pop(stack)?);
                    stack.push(i64::from(a == b));
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

    /// Run a program whose bodies come from an [`Inventory`] instead of from
    /// the program's own `functions` list.
    fn run_with_inventory(
        program: &Program,
        inv: &dyn Inventory,
    ) -> Result<I64Dialect, RunError<I64Error>> {
        let vocab = validate(CoreOnly).expect("core-only vocabulary conforms");
        let mut interp =
            Interpreter::new(&vocab, program, I64Dialect::default()).with_inventory(inv);
        interp.run()?;
        Ok(interp.dialect)
    }

    /// `total = 0; REPEAT 3 -> body 1`, where body 1 adds `step` to `total`.
    /// The entry is identical in both backings; only body 1 differs, so the
    /// result says which backing was actually read.
    fn step_program(step: u8) -> Program {
        let entry = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, 0),
                Call::with_value(FnIndex::VAR_SET, 0),
                Call::with_value(FnIndex::NUMBER, 3),
                Call::with_value(FnIndex::REPEAT, 1),
            ],
        )
        .unwrap();
        let body = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::VAR_GET, 0),
                Call::with_value(FnIndex::NUMBER, step),
                Call::new(FnIndex::ADD),
                Call::with_value(FnIndex::VAR_SET, 0),
            ],
        )
        .unwrap();
        Program {
            functions: vec![entry, body],
        }
    }

    /// FAILS IF: `with_inventory` does not actually redirect body resolution —
    /// if `run_function` or `branch` still reads `program.functions`, the run
    /// returns the PROGRAM's 3 rather than the INVENTORY's 21.
    ///
    /// Two-sided on purpose: the same program run WITHOUT an inventory must
    /// still return 3, so this cannot pass by an implementation that ignores
    /// the program entirely.
    #[test]
    fn with_inventory_resolves_bodies_the_program_does_not_carry() {
        let program = step_program(1);
        let without = run(&program).expect("runs");
        assert_eq!(
            without.vars[0], 3,
            "the program's own body adds 1, three times"
        );

        // Same entry, a DIFFERENT body 1 — reachable only through the trait.
        let other = step_program(7);
        let inv: VecInventory = other.functions.iter().copied().collect();
        let with = run_with_inventory(&program, &inv).expect("runs");
        assert_eq!(with.vars[0], 21, "the inventory's body adds 7, three times");
        assert_ne!(
            without.vars[0], with.vars[0],
            "if these agree the inventory was never consulted"
        );
    }

    /// FAILS IF: `branch`'s bound reads the program's length while bodies come
    /// from the inventory. A one-body program with a two-body inventory must
    /// reach address 1; the old `self.program.functions.len()` refuses it.
    #[test]
    fn the_branch_bound_follows_the_inventory_not_the_program() {
        let full = step_program(7);
        let entry_only = Program {
            functions: vec![full.functions[0]],
        };
        let inv: VecInventory = full.functions.iter().copied().collect();
        let d = run_with_inventory(&entry_only, &inv).expect("runs");
        assert_eq!(
            d.vars[0], 21,
            "address 1 exists in the inventory, not the program"
        );
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

    /// A vocabulary with one VOID domain byte: pops an operand, pushes
    /// nothing. Legal, declared, and exactly what a thinking dialect's
    /// side-effecting verbs look like.
    struct WithVoidOp;
    impl WithVoidOp {
        const VOID: FnIndex = FnIndex(0x90);
    }
    impl Vocabulary for WithVoidOp {
        fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
            (f == Self::VOID).then_some(1)
        }
        fn domain_body_refs(&self, _f: FnIndex) -> u8 {
            0
        }
        fn domain_pushes_result(&self, f: FnIndex) -> Option<bool> {
            (f == Self::VOID).then_some(false)
        }
    }

    /// FAILS IF: the backward operand-span walk assumes every call it steps
    /// over produced a value.
    ///
    /// It does not. The vocabulary declares `pushes_result` per byte, and the
    /// non-pushing set is not hypothetical — it is every control-flow byte in
    /// the shared core, plus whatever void verbs a domain declares. Stepping
    /// over one and crediting it with an operand stops the walk too early, so
    /// the span misses the call that actually produces the condition, and the
    /// re-run leaves the stack one short.
    ///
    /// The program: `counter = 3`, then the condition producer `VAR_GET 0`,
    /// then a void statement (`NUMBER 0; VOID`) sitting between it and the
    /// `WHILE`. The correct span starts at the `VAR_GET`; a walk that credits
    /// `VOID` with a push starts at the `NUMBER` instead, and the re-run then
    /// pushes one value and immediately voids it.
    #[test]
    fn the_span_walk_does_not_credit_a_void_call_with_an_operand() {
        struct VoidDialect {
            counter: i64,
            counter_reads: u32,
        }
        impl Dialect for VoidDialect {
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
                match f {
                    FnIndex::NUMBER => stack.push(i64::from(values[0])),
                    FnIndex::SUB => {
                        let b = stack.pop().ok_or(())?;
                        let a = stack.pop().ok_or(())?;
                        stack.push(a - b);
                    }
                    FnIndex::VAR_GET => {
                        self.counter_reads += 1;
                        stack.push(self.counter);
                    }
                    FnIndex::VAR_SET => self.counter = stack.pop().ok_or(())?,
                    WithVoidOp::VOID => {
                        stack.pop().ok_or(())?;
                    }
                    _ => return Err(()),
                }
                Ok(())
            }
        }

        let entry = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, 3),
                Call::with_value(FnIndex::VAR_SET, 0),
                Call::with_value(FnIndex::VAR_GET, 0),
                Call::with_value(FnIndex::NUMBER, 0),
                Call::new(WithVoidOp::VOID),
                Call::with_value(FnIndex::WHILE, 1),
            ],
        )
        .unwrap();
        let body = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::VAR_GET, 0),
                Call::with_value(FnIndex::NUMBER, 1),
                Call::new(FnIndex::SUB),
                Call::with_value(FnIndex::VAR_SET, 0),
            ],
        )
        .unwrap();
        let p = Program {
            functions: vec![entry, body],
        };
        let vocab = validate(WithVoidOp).expect("conforms");
        let mut interp = Interpreter::new(
            &vocab,
            &p,
            VoidDialect {
                counter: 0,
                counter_reads: 0,
            },
        );
        interp.run().expect("the loop runs down to zero");
        assert_eq!(interp.dialect.counter, 0, "the loop ran to completion");
        // Anti-vacuity: the condition really was re-evaluated, three times in
        // the span plus once per body pass. A span that never re-ran would
        // read the counter far fewer times.
        assert!(
            interp.dialect.counter_reads >= 6,
            "condition re-runs happened: {} reads",
            interp.dialect.counter_reads
        );
    }

    /// Records every call the dialect is handed, and succeeds on all of them.
    ///
    /// The point is the recording: a refusal the engine owes is only proven
    /// if the byte never reached a dialect that would have accepted it.
    #[derive(Default)]
    struct Permissive {
        seen: Vec<u8>,
    }
    impl Dialect for Permissive {
        type Value = i64;
        type Error = ();
        fn truthy(&self, v: &i64) -> bool {
            *v != 0
        }
        fn repeat_count(&self, v: &i64) -> u32 {
            u32::try_from(*v).unwrap_or(0)
        }
        fn call(&mut self, f: FnIndex, values: [u8; 3], stack: &mut Vec<i64>) -> Result<(), ()> {
            self.seen.push(f.0);
            if f == FnIndex::NUMBER {
                stack.push(i64::from(values[0]));
            }
            Ok(())
        }
    }

    /// FAILS IF: the engine decides what to execute with `Vocabulary::branches`.
    ///
    /// That predicate is `body_refs(f) > 0` and answers a CODEGEN question —
    /// its own doc says so. `BREAK`, `CONTINUE`, `STOP`, `RETURN` and `WAIT`
    /// reference no body, so under it they are not control flow, and every one
    /// was handed to `Dialect::call`. A permissive dialect then accepts them
    /// and the run reports success, while this module's contract promises
    /// `UnhandledControlFlow`.
    ///
    /// The existing `an_unproven_control_flow_call_is_refused_not_approximated`
    /// could not see this: it uses `FOR_EACH`, which HAS a body reference, so
    /// it took the branching path and reached the refusal. The fixture's SHAPE
    /// was the coverage gap, not its content.
    #[test]
    fn a_body_less_control_byte_is_refused_by_the_engine_not_offered_to_the_dialect() {
        for byte in [
            FnIndex::BREAK,
            FnIndex::CONTINUE,
            FnIndex::STOP,
            FnIndex::RETURN,
            FnIndex::WAIT,
        ] {
            // Anti-vacuity: each really is body-less, so the old predicate
            // really did call it a non-branch. Without this the test could
            // pass by accidentally picking branching bytes.
            let vocab = validate(CoreOnly).expect("conforms");
            assert_eq!(
                vocab.table().body_refs(byte),
                0,
                "{byte:?} must be body-less or this row proves nothing"
            );

            let entry = FunctionBody::from_calls(
                LaneShape::Pairs,
                &[Call::with_value(FnIndex::NUMBER, 1), Call::new(byte)],
            )
            .unwrap();
            let p = Program {
                functions: vec![entry],
            };
            let mut interp = Interpreter::new(&vocab, &p, Permissive::default());
            assert_eq!(
                interp.run(),
                Err(RunError::UnhandledControlFlow { call: byte }),
                "{byte:?} must be refused by the engine"
            );
            assert!(
                !interp.dialect.seen.contains(&byte.0),
                "{byte:?} was offered to the dialect: {:?}",
                interp.dialect.seen
            );
        }
    }

    /// FAILS IF: `REPEAT` clamps an over-cap count instead of refusing it.
    ///
    /// `min(count, cap)` ran the capped prefix and returned `Ok`, so a
    /// 200_000-iteration repeat reported SUCCESS having run 100_000 times — a
    /// wrong answer presented as a right one, and inconsistent with `WHILE`,
    /// which reports the cap in exactly this situation.
    #[test]
    fn a_repeat_count_above_the_cap_is_refused_rather_than_truncated() {
        let entry = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, 10),
                Call::with_value(FnIndex::REPEAT, 1),
            ],
        )
        .unwrap();
        let body =
            FunctionBody::from_calls(LaneShape::Pairs, &[Call::with_value(FnIndex::NUMBER, 1)])
                .unwrap();
        let p = Program {
            functions: vec![entry, body],
        };
        let vocab = validate(CoreOnly).expect("conforms");
        let mut interp = Interpreter::new(&vocab, &p, Permissive::default()).with_iteration_cap(4);
        assert_eq!(
            interp.run(),
            Err(RunError::IterationCap {
                call: FnIndex::REPEAT,
                cap: 4
            })
        );
        // The second half, and the one the clamp would fail: refusing means
        // running NOTHING. A version that ran the capped prefix and then
        // errored would satisfy the assertion above and still be wrong.
        assert!(
            !interp.dialect.seen.contains(&FnIndex::NUMBER.0) || interp.dialect.seen.len() == 1,
            "the body must not have run: {:?}",
            interp.dialect.seen
        );
    }

    /// FAILS IF: a cycle of bodies recurses until the native stack is gone.
    ///
    /// `Program::references_are_resolvable` rejects only target `0` and
    /// out-of-range targets — it does NOT reject cycles, so two bodies that
    /// branch to each other pass validation. Unbounded, that is a stack
    /// overflow, which ABORTS THE PROCESS: the one failure an orchestration
    /// engine must never turn a bad program into, and one no `RunError` can
    /// report because there is no stack left to return on.
    #[test]
    fn two_bodies_that_branch_to_each_other_are_refused_not_overflowed() {
        // 1 -> 2 -> 1 -> ... , each hop guarded by an always-true IF.
        let hop = |target: u8| {
            FunctionBody::from_calls(
                LaneShape::Pairs,
                &[
                    Call::with_value(FnIndex::NUMBER, 1),
                    Call::with_value(FnIndex::IF, target),
                ],
            )
            .unwrap()
        };
        let p = Program {
            functions: vec![hop(1), hop(2), hop(1)],
        };
        let vocab = validate(CoreOnly).expect("conforms");
        // Anti-vacuity: the cycle really does pass the program's own check,
        // so this is a defect in the ENGINE and not something validation
        // was already catching.
        assert!(
            p.references_are_resolvable(&vocab),
            "the fixture must be a program validation accepts, or it proves nothing"
        );
        let mut interp = Interpreter::new(&vocab, &p, Permissive::default());
        assert_eq!(
            interp.run(),
            Err(RunError::RecursionDepth {
                call: FnIndex::IF,
                depth: DEFAULT_RECURSION_DEPTH
            })
        );
    }

    /// FAILS IF: the iteration cap is charged after the last body instead of
    /// before the next one.
    ///
    /// Checked afterwards, a loop that terminates in EXACTLY `cap` iterations
    /// — inside the advertised ceiling — was rejected, because the cap fired
    /// before the condition could be re-tested one final time. At `cap = 0`
    /// the same ordering ran the body once before refusing, which is a cap of
    /// zero that executes.
    #[test]
    fn a_loop_terminating_in_exactly_cap_iterations_is_accepted() {
        // counter = 3; while counter > 0 { counter -= 1 } — exactly 3 bodies.
        let entry = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, 3),
                Call::with_value(FnIndex::VAR_SET, 0),
                Call::with_value(FnIndex::VAR_GET, 0),
                Call::with_value(FnIndex::NUMBER, 0),
                Call::new(FnIndex::GT),
                Call::with_value(FnIndex::WHILE, 1),
            ],
        )
        .unwrap();
        let body = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::VAR_GET, 0),
                Call::with_value(FnIndex::NUMBER, 1),
                Call::new(FnIndex::SUB),
                Call::with_value(FnIndex::VAR_SET, 0),
            ],
        )
        .unwrap();
        let p = Program {
            functions: vec![entry, body],
        };
        let vocab = validate(CoreOnly).expect("conforms");
        let mut interp = Interpreter::new(&vocab, &p, I64Dialect::default()).with_iteration_cap(3);
        interp
            .run()
            .expect("3 iterations under a cap of 3 must run");
        assert_eq!(interp.dialect.vars[0], 0, "the loop ran to completion");

        // The paired half: a cap of ZERO must refuse before running anything.
        let mut zero = Interpreter::new(&vocab, &p, I64Dialect::default()).with_iteration_cap(0);
        assert_eq!(
            zero.run(),
            Err(RunError::IterationCap {
                call: FnIndex::WHILE,
                cap: 0
            })
        );
        assert_eq!(
            zero.dialect.vars[0], 3,
            "a cap of zero must not execute a body"
        );
    }
}
