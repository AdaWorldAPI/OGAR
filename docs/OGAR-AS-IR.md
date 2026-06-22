# OGAR as IR — the compiler framing

> **For designers and reviewers.** lance-graph is a multi-phase compiler
> whose IR is OGAR. Every existing architectural decision in this repo
> restates a standard compiler discipline. New decisions should pass the
> IR-shape tests in §3 before landing.
>
> Status: **FRAMING v1** (2026-06-22).

This doc is a *lens*, not a rule. It does not add a new abstraction
layer; it labels the layer that is already here, in vocabulary that has
been field-tested for fifty years of compiler engineering. Reach for the
labels when designing; reach for the tests when reviewing.

---

## §1. The mapping

| Compiler phase | OGAR / lance-graph piece |
|---|---|
| Source readers / front-end parsers | `ogar-from-ruby` (`ruff_ruby_spo`) · `ogar-from-python` · `ogar-from-elixir` · `ogar-sql-ddl` · future readers |
| AST / IR | `ogar-vocab::{Class, ActionDef, ActionInvocation, KausalSpec, Identity}` |
| Symbol table | `ogar-vocab::class_ids` + `ports::*Port` alias tables |
| Linker / name resolution | cross-fork convergence — `Stundenzettel` / `TimeEntry` / `HrAttendance` resolve to the same `0x0103` symbol |
| Public ABI / wire-compat surface | `lance_graph_contract::ogar_codebook` — the BBB membrane, zero-`ogar-vocab` mirror, parity-tested against the canonical table |
| Semantic analysis | `lance-graph-ontology` (registry validity); the pending `lance-graph-rbac::authorize` keystone |
| Optimization passes | `lance-graph-planner` — 16 strategies, ontology-aware traversal, thinking-style selection |
| Codegen back-ends | `ogar-adapter-surrealql` (DDL) · `ogar-to-postgres` · `ogar-to-openapi` · `ogar-to-typescript` · the SoA columnar back-end (Arrow IPC / Lance) |
| Native codegen | `jitson` (Cranelift) compiling the 36 thinking-style scan kernels into native machine code |
| Runtime / dynamic linking | `lance-graph-callcenter` dispatching messages to `ogar/Class` actors |

---

## §2. Existing decisions, restated in compiler vocabulary

Each principle this repo already pins, re-named so future contributors reach for the right primitive:

| Rule as the repo states it | Standard compiler primitive |
|---|---|
| "The classid is pure address; the magic is what it resolves to." | **SSA value-id with a symbol table.** The id names the value; the table behind it carries the type, the methods, the effects. |
| "Behaviour flows producer → OGAR `Class`+`ActionDef` → adapter; never producer → DDL." | **No `eval` at codegen time.** Lowering passes emit; they do not interpret. The IR is the source of truth. |
| "Spine vs membrane — the BBB-barrier." | **Internal IR types vs public ABI.** Membrane consumers link against the wire-compat mirror; spine-internal can use the full IR. |
| "One concept, many renders." | **Same canonical symbol, multiple lowering targets.** Compile once, emit N. |
| "SurrealQL DDL is an adapter, not the spine." | **A codegen back-end is not the IR.** Putting lifecycle in DDL = encoding the source program inside the assembly output. |
| "The adapter is lossy on the behavioural arm." | **Codegen back-ends discard IR information the target cannot represent.** DDL has no FSM vocabulary, so the behavioural arm cannot survive lowering. |
| "Pull the classid; never re-mint the Core." | **Link against the symbol table; do not fork it.** Local copies drift on the next symbol-table version. |
| "Append-only canon; regrade in place." | **Symbol ids are stable across versions.** A symbol once minted is never re-numbered. |
| "No serialization in the hot path." | **IR is the runtime wire-truth; there is no parse step at runtime.** Compiled output runs, not source code. |
| "RBAC is a relation, not a stamp — classid + role + membership." | **Authorization is a semantic-analysis pass over the IR.** It verifies a property; it does not invent one. |

---

## §3. Six tests for new IR design

Before adding a field to `Class`, a variant to `ActionDef`, a slot to `KausalSpec`, or any other surface that future producers and back-ends will need to round-trip — a proposal must pass these six tests.

1. **SSA / dataflow-explicit.** Each operand and result is a named value. No implicit hidden state. Lifecycle transitions in `KausalSpec` traverse as a control-flow graph: basic blocks, edges, joins at phi-shaped merge points.
2. **Effect annotations are first-class.** Each `ActionDef` declares what it reads, what it writes, what side effects it has (audit, network, persistence, mutation). Pure-vs-effectful is a *type*, not a comment. Without this, no optimization is sound.
3. **Typed signature, not field bag.** `Class.attributes` is a typed signature: every attribute carries a stable type symbol from the type system, not an opaque string blob. The structural arm IS the type system.
4. **Lowering passes are named, not implicit.** Each back-end is an explicit pass with a stated signature `IR → target`. Adapters are lowering passes; they do not transform the IR. If a back-end needs IR changes, the IR changes come first, then the lowering.
5. **Optimization passes declare a semantic-preservation guarantee.** Each transformation states what it preserves: "observable behaviour modulo X." A pass without a guarantee is a rewriter, not an optimizer.
6. **The IR is the canonical artifact.** Source ASTs and target outputs are interchangeable over time; only the IR is preserved. If a design lets a target dialect creep into the IR, it is conflating layers and will rot at the next back-end.

A proposal that fails any test is not rejected — it is **rerouted**. The right place for the proposed structure may be the source reader (a parsing concern), the back-end (a lowering concern), or the optimizer (an analysis result) — but it is *not* the IR. Keep the IR small, typed, dataflow-explicit, effect-annotated.

---

## §4. Cross-references — already compiler-shaped, now labeled

The docs below were already compiler-shaped in design. This framing makes the labels explicit so future contributors can reach for the right vocabulary.

| Doc | What it is, in compiler vocabulary |
|---|---|
| [THE-FIREWALL.md](THE-FIREWALL.md) (ADR-022/023) | "No serialization in the hot path" = **IR is the runtime wire-truth; there is no parse step.** Compiled output runs, not source. |
| [OGAR-AST-CONTRACT.md](OGAR-AST-CONTRACT.md) | The typed IR header — the OGAR equivalent of an LLVM IR include. `Class`/`ActionDef`/`Identity`/`KausalSpec` are the IR node kinds. |
| [SURREAL-AST-AS-ADAPTER.md](SURREAL-AST-AS-ADAPTER.md) | The carved decision that DDL is a **codegen back-end, not the IR**. The structural arm lowers; the behavioural arm cannot survive lowering and stays in the IR. |
| [APP-CLASS-CODEBOOK-LAYOUT.md](APP-CLASS-CODEBOOK-LAYOUT.md) | The symbol-table layout. `hi u16 ‖ lo u16` = **object-file identifier ‖ symbol id.** The APP prefix selects a lowering target's view; the concept is the linker-canonical name. |
| [CLASSID-RBAC-KEYSTONE-SPEC.md](CLASSID-RBAC-KEYSTONE-SPEC.md) | The pending **semantic-analysis pass.** `authorize(actor, classid, op) → AccessDecision` is the verification predicate over the IR. |
| [OGAR-CONSUMER-BEST-PRACTICES.md](OGAR-CONSUMER-BEST-PRACTICES.md) | The consumer rule "pull the classid via `PortSpec`, never re-mint" = **link against the public symbol table; do not fork it.** §2's spine-vs-membrane split is **ABI tiering**. |
| [CONSUMER-MIGRATION-HOWTO.md](CONSUMER-MIGRATION-HOWTO.md) | The bridge-removal recipe = **switch from a private linker (a bridge wrapping the registry) to the public linker (`PortSpec` / `contract::ogar_codebook`).** |
| [SURREAL-AST-TRAP-PREFLIGHT.md](SURREAL-AST-TRAP-PREFLIGHT.md) | The pre-flight against treating a back-end's vocabulary as the IR's. The five questions are *"are you trying to encode IR-only concepts in a target-language form?"* |

---

## §5. Why this framing

Two reasons.

First, the **vocabulary**. Every contributor reaching for a new abstraction now has fifty years of compiler engineering to draw from. "Should this be in the IR or the back-end?" has a known shape; "Should this be a pass or a structural change?" has a known shape; "Is this semantic-preserving?" has a known shape. Saying "OGAR is a graph ontology" loses those shapes; saying "OGAR is the IR of a compiler" recovers them.

Second, the **discipline**. Compiler IRs are designed to be optimized, to be lowered to many targets, to be analyzed for correctness. Database schemas are designed to be queried. The work already shipped in this repo — multi-target lowering, semantic analysis, optimization passes, native codegen via jitson — is compiler work. Naming it as compiler work makes the next design choices automatic.

The framing changes no existing decision. It changes every future one.
