# What `ogar-loco` needs to carry `rs-graph-llm`'s orchestration

> Design pass, 2026-09-14. Reference read: `AdaWorldAPI/rs-graph-llm`
> `graph-flow` at `59f9315` (3,723 LOC across 11 modules). Nothing here is
> built; this is the gap list and the shape each closure must take.

## ⊘ CORRECTED — the answer is ONE thing, and this doc buried it (operator, 2026-09-14)

Operator, on the version below: *"All I need from ogar-loco to know functions
as objects. It can't be so hard."*

Right, and this crate already says so. `LocoConcept::Inventory` — **`0x1702`,
already minted** — carries this doc in `lib.rs`:

> the **inventory** row: the function registry entry (which functions exist,
> **addressed by identity**). A registry read never touches a body.

That IS functions-as-objects. It is already the design, already has a concept
id, and **nothing implements it**. `Program { functions: Vec<FunctionBody> }`
is a placeholder standing in for it, and the interpreter resolves a branch as
`self.program.functions.get(idx)` — a Vec index, not an address.

So the whole change is one indirection:

```
branch(target)  →  inventory.get(address)     // not functions[byte]
```

A function AT REST is already an object: `FunctionNode` is 512 bytes with a
16-byte key in slot 0, and `node.rs` keeps that key deliberately opaque so the
substrate mints it. The identity exists; the runtime does not use it.

**Once a branch target is an address rather than a local index, the four
"gaps" below stop being gaps and become consequences.** A function can be a
VALUE (its address fits an immediate byte through the constant pool, or the
facet's 12 bytes directly); a continuation is a function; graph orchestration
is functions referencing functions. No async, no `Context` map, no
`SessionStorage` — those are graph-flow's answers to *not having this*.

**The error in what follows:** it maps graph-flow's architecture onto loco
feature-by-feature, instead of asking what loco lacks. Everything below is
kept as the reading record of `graph-flow` at `59f9315` — the decision "no
async" still stands, and the four consequences are real — but `Inventory` plus
a `branch` that resolves through it is the answer, and the rest is downstream
of it.

## The headline: loco must NOT become async

`graph-flow` gets human-in-the-loop by being async all the way down —
`#[async_trait] Task`, tokio, a `SessionStorage` trait with four backends, and
a `FlowRunner` that loads a session, steps it, and saves it. Porting that
shape into `ogar-loco` would pull `tokio` + `serde_json` + `async-trait` into
a zero-dep crate whose whole value is being a 512-byte call ABI. That is not a
trade worth making, and it is not necessary.

**Suspension replaces async.** A node that needs to await an LLM call returns
`Suspend`; the HOST awaits; the host resumes. That is strictly more general
than an async runtime, for one reason worth stating plainly:

> A suspended loco program is **bytes** — persistable, inspectable,
> diffable, learnable. A pending Rust future is none of those.

Everything below follows from that one decision.

## Feature-by-feature

| `graph-flow` | `ogar-loco` today | verdict |
|---|---|---|
| `Task` — async node, `run(Context) -> TaskResult` | `Call` / `FunctionBody` | **have it**, and finer-grained: a node is 180 calls, not one closure |
| `Edge { from, to, condition: Arc<dyn Fn(&Context)->bool> }` | `IF` / `IF_ELSE` + the condition span | **have it, and better** — see below |
| `NextAction::{Continue, ContinueAndExecute}` | the pc walk | have it |
| `NextAction::End` | body exhaustion | have it |
| `NextAction::GoTo(String)` | — | **G1** |
| `NextAction::WaitForInput` + session save/load | — | **G2 — the keystone** |
| `Context` — `HashMap<String, Value>` behind a lock | dialect-private state + `VAR_GET`/`VAR_SET`'s 256 slots | **G3** |
| `FanOutTask` + tokio | — | **G4** |
| `SessionStorage` × 4 backends | — | **nothing in loco** — see G2 |

### The conditional edge is already the stronger form

`graph-flow` attaches `Arc<dyn Fn(&Context) -> bool>` to an edge. loco attaches
a **program** — the calls preceding the branch, re-run each iteration
(`Interpreter::operand_span_start`). A closure cannot be serialized, cannot be
inspected, cannot be compiled to a different backend, and cannot be learned. A
condition span is all four. This is not a gap to close; it is a reason the
port goes in this direction rather than the other.

---

## G1 — a jump byte

`NextAction::GoTo(String)` is dynamic routing by name. loco's equivalent is a
call whose value byte is a **body reference** — the `body_refs` machinery the
vocabulary already declares per byte, and which `IF`/`REPEAT` already use.

The shared core reserves `STOP` / `RETURN` / `BREAK` / `CONTINUE` and the
engine currently REFUSES all four (`RunError::UnhandledControlFlow`), which is
the right posture: they were never validated by a probe. `GOTO` joins that
list, and lands the same way — with a falsifier, not with a guess.

Cost: one arm in `run_branching`, plus a loop-safety question `IF`/`REPEAT` do
not have (a `GOTO` can build a cycle the structured ops cannot).

**And "the iteration cap already covers it" is wrong — that sentence stood here
and review was right to reject it.** `iteration_cap` is a PER-LOOP ceiling:
`REPEAT`, `WHILE` and `REPEAT_UNTIL` each check their own count against it. A
`GOTO` cycle contains no loop construct, so it checks nothing and runs forever
inside a cap that is never consulted. A per-loop ceiling cannot bound a
control-flow graph; only a shared budget can.

So `GOTO` lands with a second, distinct quantity, and the spec is the part that
has to exist before the arm does:

- **`step_budget`** — one counter for the whole run, decremented **once per
  call executed**, whatever executed it. A structured loop body, a `GOTO`
  target, a branch into another function, and a resumed run after a suspension
  all consume it identically, because they are all "a call ran".
- **`iteration_cap` stays, and stays per-loop.** It is a different guarantee:
  it bounds ONE construct's repetitions so a runaway loop is attributable to
  that loop. The budget bounds the RUN. Neither subsumes the other — a program
  can exhaust the budget with no loop at all, and a single loop can hit its cap
  while the budget is barely touched.
- **Exhaustion is a refusal, not a truncation:** `RunError::StepBudget`,
  carrying the call that spent the last step, so a caller can see where.
- **A resumed run does NOT get a fresh budget.** The remaining count is part of
  what `snapshot` persists (see G3 below) — otherwise suspension is an
  unbounded-execution loophole: suspend, resume, repeat.

Until that exists, `GOTO` stays refused alongside `STOP` / `RETURN` / `BREAK` /
`CONTINUE`, which is the correct posture and not a gap.

## G2 — an explicit frame stack (the keystone)

**Today** `Interpreter::branch` RECURSES into `run_body`. The interpreter's
state therefore lives in the Rust call stack, where it cannot be paused,
persisted, or examined.

(It was `run_function(index)` until #304 split resolution out of execution —
`run_body` takes an already-resolved body so no path through it can fail to
find one and silently succeed. The recursion this gap is about is unchanged;
only where the address is resolved moved.)

**Needed:** `frames: Vec<Frame { func: u16, pc: u16 }>`, walked iteratively.
Then the whole interpreter state is `(frames, dialect_state, stack)`.

The loco-native part, and the reason this is cheap rather than a rewrite tax:

> A `Frame` is `(u16, u16)` = 4 bytes = exactly one `u8:u8:u8:u8` slot in
> `LaneShape::Quads`. **A continuation IS a `FunctionBody`.** 90 quads = 90
> frames of depth, in one 512-byte node, in the format everything else already
> speaks.

So `run()` becomes:

```
enum Run<V> { Done(V), Suspended(Continuation) }
```

and `resume(k, injected_value)` continues. `SessionStorage` needs no analog in
loco at all: a session is a `FunctionBody` plus a dialect blob, and what a
consumer does with bytes is the consumer's business. The zero-dep posture
survives intact.

Two things this changes that must be re-pinned, not absorbed:

- **The `WHILE` condition-span re-run becomes frame state.** Today it is an
  inner `for` loop inside one `run_branching` call; under an explicit stack it
  must be a resumable position. That is the single subtlest part of the
  rewrite, and it is exactly the machinery
  `while_reruns_its_condition_span_and_computes_gcd` already falsifies — so
  the test that guards it exists before the change does.
- **`step_budget` persists across resumptions; `iteration_cap` stays per-loop.**
  A resumed run continues spending the same budget rather than receiving a
  fresh one — which is what makes step limits fall out for free, and what
  stops suspend-resume-repeat from being an unbounded-execution loophole.

  ⊘ This bullet used to read *"the iteration cap becomes a BUDGET spendable
  across resumptions"*, which contradicted G1's spec four sections above — a
  spec whose own text rejects exactly that reading (*"'the iteration cap
  already covers it' is wrong"*). So the correction was written in one section
  and the error left standing in another. **A second section is a second place
  to be wrong**, and the one that summarizes is the one a reader reaches
  first.

## G3 — a dialect snapshot seam

`graph-flow`'s `Context` is a `HashMap<String, serde_json::Value>` behind a
lock. loco must not grow one: the V3 12-byte facet register keyed by classid
is the stack's own state model, and it is zero-copy where a JSON map is not.

What is genuinely missing is narrower: **the dialect's store is opaque to the
engine**, so the engine cannot persist it across a suspension. The fix is a
byte seam, and loco must define the *seam* and never the *format*:

```
trait Dialect {
    fn snapshot(&self, out: &mut Vec<u8>);          // or a &mut [u8] sink
    fn restore(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
}
```

A dialect whose state is already facet rows writes them directly; one holding
`[i64; 256]` writes 2 KiB; one holding masks writes mask words. The engine
never looks inside. `VAR_GET`/`VAR_SET`'s 256 slots stay exactly what they
are — the named half of the state model, already addressable.

### Where those bytes are allowed to live — the boundary, stated

Review asked this and the doc did not answer it, which is a real omission
rather than a nit: a byte seam whose destination is unspecified is one review
away from becoming a serialization channel.

**The snapshot is HOST-LOCAL and OUTSIDE the hot path. It does not cross a
mailbox.** A suspended run's bytes belong to whoever is holding that run — a
scheduler slot, a local arena, a durable store the host owns — and they are
read back by the same host on resume. That is what makes `snapshot` cheap
enough to be worth having.

This is not a preference; it is what the repo's own non-negotiables already
require:

- **ADR-022 / ADR-023 — the Firewall.** No serialization in the hot path; the
  IR is wire truth. An opaque dialect blob is by construction NOT the IR, so it
  is exactly the thing that must not be on a wire. Writing it host-locally is
  not a crossing; handing it to another mailbox would be.
- **Therefore:** if a suspension ever has to move between owners, it does not
  travel as a snapshot blob. It travels as the IR — the program, its address,
  and the facet rows that are already the state model — and the receiving host
  rebuilds. `snapshot` is a resume aid for one host, never a transport.

The falsifier, so this cannot quietly erode: **no snapshot byte may appear in
any type that crosses an owner boundary.** If a future `Baton`, envelope, or
mailbox row grows a field carrying `Dialect::snapshot` output, that is the
violation, and it is greppable rather than a matter of judgement.

## G4 — fan-out is a MASK op, not a task pool

`FanOutTask` runs N child tasks concurrently on tokio and merges their writes
into one shared `Context`. That is the right shape for an async LLM
orchestrator and the wrong one here.

In loco's world, N parallel branches over one population is **one mask program**:
N `MaskOp::Pred` into N slots, then a combine. No threads, no shared mutable
context, no merge policy to get wrong — and it is the same substrate the query
side uses. Fan-out therefore lowers DOWN into `lance-graph-mask-risc`, not out
into a runtime.

This is where the operator's line lands: *the masking ops were necessary to
make the 34 NARS reasoning cheap*. A tactic that costs a task spawn cannot be
orchestration; a tactic that costs a ternlog can.

---

## The ladder this sits in

```
ndarray::simd masking ops        ← primitives
mask-risc Program                ← primitives composed into a method
loco opcode (0x90.. per classid) ← the method becomes an opcode
loco Interpreter                 ← orchestration: branch, loop, suspend, resume
```

`ogar-r2il` already occupies `0x90..=0xE1` **under its own classid** — 82
machine opcodes. A query dialect, a NARS-tactic dialect and a Blockly palette
each get their own classid and their own `0x90..` range, routed by
`VocabularyRegistry` (concept id → validated `VocabularyTable`, hi-u16
canon-high). That is what "loco speaks all dialects" means mechanically:
**anything else just needs a classid.**

The basic vocabulary is **144 + 34 + 36**: the 144 verb atoms (rung 2), the 34
NARS tactic recipes (`lance_graph_contract::recipes::RECIPES: [Recipe; 34]`,
rung 3 — the runbooks), and the 36 `ThinkingStyle`s (rung 4). The 34 exist
today as ~30 bespoke Rust functions in `ndarray/src/hpc/styles/`; as loco
programs they become data, and data is what can be revised, learned and
persisted.

## Open, and deliberately not decided here

- **The core's 144 slots currently hold the Blockly palette.** The core is
  exactly `0x00..=0x8F` = 144, and the verb table is exactly 12×12 = 144.
  Whether the palette moves above `DOMAIN_FLOOR` so the core can hold the 144
  verb atoms is an operator call, not one to make inside a design doc.
- **`256:256` as function-value syntax.** A `u8:u8` lane read as
  `(function, value)` is one call; read as `palette256:palette256` it is a
  centroid pair. Both readings live in the same 12 bytes and the ClassView
  picks. What that buys the ORCHESTRATION layer specifically — a call whose
  "function" is itself a centroid, i.e. a soft dispatch — is unexplored and
  should be probed before it is designed.
