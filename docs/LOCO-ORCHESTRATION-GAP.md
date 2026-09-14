# What `ogar-loco` needs to carry `rs-graph-llm`'s orchestration

> Design pass, 2026-09-14. Reference read: `AdaWorldAPI/rs-graph-llm`
> `graph-flow` at `59f9315` (3,723 LOC across 11 modules). Nothing here is
> built; this is the gap list and the shape each closure must take.

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
not have (a `GOTO` can build a cycle the structured ops cannot). The iteration
cap is already the answer; it just has to cover jumps as well as loops.

## G2 — an explicit frame stack (the keystone)

**Today** `Interpreter::run_function(index)` RECURSES. The interpreter's state
therefore lives in the Rust call stack, where it cannot be paused, persisted,
or examined.

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
- **The iteration cap becomes a BUDGET spendable across resumptions.** That is
  also how LangGraph-style step limits fall out for free.

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
