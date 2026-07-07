# ActionDef value-dispatch — body lowering + a value interpreter

> **Status:** PROPOSAL (2026-07-07). Producer/executor capability gap; no
> consumer-specific detail. Sibling of `OGAR-AS-IR.md` (the compiler framing)
> and `OGAR-TRANSPILE-SUBSTRATE.md` (the bidirectional per-class transpiler).

## Problem

`compile_graph_*` (the `ruby` / `python` / `sqlalchemy` / `csharp` frontends)
emit `CompiledClass { class, facet, actions }`. Each `ActionDef` carries
`identity` / `predicate` / `object_class` plus the effect sets `writes` /
`reads` / `calls` / `raises` — but **not what the method computes**.

Measured over a real compiled corpus (2748 `ActionDef`s), every emitted action has:

- `kausal = None`, `body_source = None`, `on_enter = None`, `decorators = []`
- `default_modal` / `default_temporal` / `default_subject` = the constants
  `Sync` / `Immediate` / `System` (no per-method information)

Only `identity` / `predicate` / `object_class` + the name-level effect sets are
populated. `crates/ogar-vocab/src/lib.rs` documents this directly:

- `writes` — *"the value written is not captured by today's frontend, so this is
  name-level only and does NOT auto-build an `on_enter` `EnterEffect`"*
  (lib.rs:402-403).
- `calls` — *"Effect annotation for call-graph analysis; no body is captured."*
  (lib.rs:406).

Crucially, the **typed carriers already exist** — `on_enter: Option<EnterEffect>`
(lib.rs:423), `EnterEffect` (lib.rs:447), and `ActionDef::kausal` /
`ActionDef::body_source` (consumed by `ogar-emitter` at lib.rs:667-674). The gap
is not the schema — it is that **the frontend never populates them**.

## Consequence

An `ActionDef` says *which fields a method touches*, not *what it computes*. A
consumer can therefore verify **structural effect-parity** — does the running
code's observed write-set match the declared `writes`? — but it **cannot execute
an `ActionDef` to reproduce a recoverable method's return value**. There is no
expression, formula, or lowered body in the artifact.

The two executors that exist today are not value interpreters:

- `ogar-action-handler::NativeCommandExecutor` / `CapabilityExecutor::execute` —
  runs a capability by shelling a bound `command` parameter to an external
  handler (SSH/REST/shell). The logic lives in the external handler plus the
  invocation params, not in the `ActionDef`.
- `lance-graph-ogar::OgarActionProvider` + `contract::action::ActionInvocation::commit`
  — the authorization/gate path (RBAC → StateGuard → MUL `GateDecision`). A
  capability dispatcher / gate, not a value interpreter.

So a consumer that wants a recoverable `Compute` / `Normalize` method to run
*through* its `ActionDef` (instead of a hand-written reimplementation) is blocked:
reproducing the value would force the consumer to re-derive the formula itself —
exactly the "logic in the consumer" that the consumer-side boundary forbids
(`OGAR-CONSUMER-BEST-PRACTICES.md`: the classid is address; the magic resolves at
the target — behaviour is a property of the Core node, never re-implemented
downstream).

## Proposal

Two coordinated pieces, so recoverable methods can dispatch value-for-value.

### 1. Frontend lowering — populate the carriers that already exist

During `compile_graph_*`, capture method bodies into an executable form and
populate the **existing** typed fields rather than name-level effect sets only:

- `ActionDef::on_enter: EnterEffect` — the value-producing transition for a
  `Compute` / `Normalize` recipe (the recipe-codebook's recoverable band).
- `ActionDef::kausal: KausalSpec` — the guard/lifecycle/dependency structure for
  `Guard` / `Cascade`.

This is the frontend half of the recipe-codebook's promise: a recoverable body is
supposed to lower to a *declarative recipe*, but today the recipe classification
lands while the recipe *content* does not. The `fuzzy-recipe-codebook` method
(ruff `.claude/knowledge/`) already correlates a body to a `(verb, criteria)`
recipe; the missing step is emitting that recipe's computable form into
`EnterEffect` / `KausalSpec`.

### 2. A value-reproducing interpreter

An interpreter that takes an `ActionDef` (+ typed inputs) and yields its effect
values. Per the "thinking lives in lance-graph" boundary, this belongs on the
lance-graph side, consistent with `lance-graph-ogar` already owning the
authorization/commit path. Consumers consume it through a thin trait seam:

```rust
trait ActionDefExecutor {
    fn apply(&self, def: &ActionDef, inputs: &EffectInputs)
        -> Result<EffectValues, Unsupported>;
}
```

A consumer wires this seam with a default `Unsupported` implementation today and
flips it to the real impl the moment the interpreter lands — never implementing
the formula itself.

## Scope / non-goal

This is the gap between *"the offline transpile emits effect descriptors"*
(works today) and *"a runtime dispatches recoverable methods value-for-value"*
(blocked). It is purely a producer/executor capability gap with no
consumer-specific detail.

**Staging.** Until both pieces land, a consumer's honest ceiling for any
recoverable route is **structural-effect-parity green, value-dispatch pending**:
it can diff a running handler's observed write-set against the declared `writes`
(a mechanical, non-thinking check) and shadow it as observability, but it cannot
promote a route to `ActionDef`-primary. The `ActionDefExecutor`-defaults-to-
`Unsupported` seam is exactly the forward-compatible shape for that staging: no
behaviour changes downstream until the interpreter exists, and no formula is ever
re-implemented in a consumer.

## Falsifier

The proposal is validated when, for one recoverable `Compute` method, an
`EnterEffect`-lowered `ActionDef` fed through the interpreter reproduces the
hand-written oracle's return value **bit-for-bit** on a golden fixture set — the
same shape as the existing byte-parity discipline elsewhere in the stack.
