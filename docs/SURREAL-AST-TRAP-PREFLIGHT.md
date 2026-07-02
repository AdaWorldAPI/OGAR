# SurrealQL AST trap — pre-flight introspection (the spellbook)

> **Operational extract** of `SURREAL-AST-AS-ADAPTER.md` (the design).
> Read this BEFORE the keyboard fires — five questions, 90 seconds, one
> mirror. If you finish all five, you've cleared the trap; if you stop
> at any one, you've caught it pre-materialization.
>
> The pattern this protects against is **pre-cognitive**, not reasoned:
> sessions don't argue their way in, they *reflex* their way in. This
> doc exists because a mirror that fires before the reflex is cheaper
> than a refactor that fires after.
>
> Status: **SPELLBOOK v0** (2026-06-22). Append-only.
>
> Grounded in: `SURREAL-AST-AS-ADAPTER.md` §0 (the carved decision),
> `OGAR-AST-CONTRACT.md` §1 (the canonical IR — `Class` + `ActionDef`
> + `Identity` + `KausalSpec`), `THE-FIREWALL.md` (ADR-022/023 — no
> serialization in the hot path), `ARCHITECTURE.md` (the AR-as-spine
> doctrine).
>
> READ BEFORE: any producer→IR / transcode / codegen / `ogar-from-<lang>`
> reader / `.surql` authoring session. The agent ensemble
> (`core-first-architect`, `adapter-shaper`, `core-gap-auditor`,
> the `Plan` subagent) loads this Tier-1 before output.

## §0. Why this trap is structurally impossible to fix from inside DDL

> **The classid is pure address. The magic is what the address
> resolves to.** (Drilled with worked examples in
> `OGAR-CONSUMER-BEST-PRACTICES.md` §0.)

The classid (`0xDDCC_AAAA`) is **pure address**. Both halves —
hi u16 (canonical concept) and lo u16 (APP / render lens) — are
*address dimensions*. Neither carries behavior. (Order flipped
2026-07-02 — canon HIGH / custom LOW; pre-flip docs and baked data use
the legacy order — see `docs/DISCOVERY-MAP.md` D-CLASSID-CANON-HIGH-FLIP.)

Where does the behavior live? At the **resolution target**:

```
classid ──┬──► ClassView                  the SKIN     (render — per-app)
          ├──► Class (structural)         the SHAPE    (canonical — shared)
          └──► ActionDef + KausalSpec     the MAGIC    (behavioral — lifecycle/
                                                        callbacks/validations,
                                                        ALWAYS in OGAR Core)
```

This is the Core-First identity = classid rule: **the address is dumb,
the magic is what it points at**. Class-magic (callbacks, lifecycle,
validations — the rails ActiveRecord blob) is a property of the Core
node the address resolves to, **never** a property of the address itself.

This is *why* the SurrealQL trap is structurally impossible from inside
the DDL: SurrealQL has the address but **not** the Core types it
resolves to. So encoding lifecycle in `DEFINE EVENT … WHEN … THEN …`
or sentinel comments is trying to **put the value into the key** — to
forge behavior at the wrong layer entirely. It looks like it works in
the narrow case and rots immediately at every other layer.

The five questions below are the operational mirror that catches this
before keyboard. If you find yourself reaching for DDL to express
something the address can't carry, the question isn't *"how do I
encode this in DDL?"* — it's *"which Core type does this address
resolve to, and have I lifted the behavior into THAT?"*

## The trap, in one paragraph

You're reading an AR-shaped producer — Ruby callbacks, Odoo
`@api.constrains`, Ecto `Multi`, Elixir `GenStateMachine`, TypeScript
class with decorators. The first reflex is *"I'll walk the structural
DDL form and parse it from there."* That reflex is **System-1** — it
looks like a clean parse step. **System-2** is the harder move: route
through OGAR's `Class` + `ActionDef` first, lift the behavior into
typed fields, *then* emit to SurrealQL (or any other surface) as a
lossy adapter step. The trap is the absence of OGAR — substituting
SurrealQL AST for it. Once you commit to SurrealQL as your primary IR,
the lifecycle has nowhere to go, and the pressure to encode it in
`DEFINE EVENT … WHEN … THEN …` becomes *structural*, not avoidable.
What follows is the negative-beauty hijack the design doc explicitly
rejects: comments, sentinels, DDL pretending to be a state machine.

The trap is silent at the time of writing and loud at the time of
re-derivation. Avoiding it costs five questions. Recovering from it
costs a fork.

## The five questions — run BEFORE the keyboard fires

```
Q1.  WHAT am I reading FROM?
     ├─ Ruby AR / Python @api / Ecto / Elixir GenStateMachine /
     │  TypeScript class with decorators / Smalltalk methods / …
     │      →  HAS BEHAVIOR.  Route through a producer-side reader.
     │
     └─ SurrealQL DDL / .surql / RDF triples / pure schema /
        Avro / JSON Schema / protobuf
            →  HAS NO BEHAVIOR to recover.  Structural-only is honest.

Q2.  Does the source have LIFECYCLE VOCABULARY?
     Look for, in order of frequency:
       • callbacks            before_save / after_commit / after_update
       • decorators           @api.constrains / @api.depends / @validates
       • state machines       Statesman / Ecto.Multi / GenStateMachine
       • validations          validates_presence_of / Zod refine
       • lifecycle hooks      on_enter / on_exit / state_timeout
       • timeout / retry      state_timeout_millis / retry-with-backoff
       • guards               can?(:transition) / GuardFailurePolicy
     ├─ ANY of the above present  →  MUST land as ActionDef / KausalSpec.
     │                              NOT in DDL.  Stop reaching for .surql.
     └─ NONE present             →  structural-only is the honest scope.

Q3.  What is the TARGET IR I'm about to commit to?
     ├─ ogar_vocab::Class + ActionDef + Identity + KausalSpec
     │      →  canonical; behavior has a home.  Proceed.
     │
     └─ surreal_ast::Library / DefineTable / DefineEvent / .surql files
            →  if Q2 was YES, STOP.  You're about to hijack DDL.
               The target should be OGAR Class FIRST.
               Then emit via ogar-adapter-surrealql as a SEPARATE step.

Q4.  Which way does my arrow point?
     ├─ producer  →  OGAR  →  adapter(SurrealQL/REST/codegen)
     │      →  correct.  Canonical retains behavior; adapter is lossy
     │         egress; the loss is paid once at the boundary, by design.
     │
     └─ producer  →  SurrealQL  →  ???
            →  trap.  Behavior loses its home.  Pressure to encode
               lifecycle in DEFINE EVENT is now structural, not avoidable.

Q5.  Would the INVERSE recover the behavior?
     Imagine: emit this DDL, hand it to a fresh session, ask them to
     distill back to AR.  Do they recover ActionDef / lifecycle / guards?
     ├─ Yes, recoverable             →  you're smuggling.  Behavior is
     │  via sentinel comments,         hiding in conventions you'll forget
     │  hijacked DEFINE EVENT,         in 6 months and a reviewer in 2.
     │  string-typed state names       That IS the trap.  Stop.
     │
     └─ No, irrecoverable            →  the behavior was never canonical
        from the DDL alone              here.  Find its real home (Q1's
                                        producer) and read it there.
```

## Diagnostic signatures — what the trap looks like in code review

If you weren't in the room for the decisions, these signatures let you
spot the trap post-hoc:

- **`.surql` files with `DEFINE EVENT … WHEN … THEN …` doing more than
  a single row trigger** — chains, conditional cascades, state-name
  string literals in the WHEN clause, multi-step transitions
- **`DEFINE FUNCTION` used as a state-machine transition** rather than
  a stored procedure (no return value, side-effects on other rows,
  reads `state` field and writes a new one)
- **Sentinel comments in DDL**: `-- @action:`, `-- @state:`,
  `-- @lifecycle:`, `-- @before_save:`, `-- @kausal:`, `-- @rubicon:`
  — any `-- @<vocab-word>` in a DDL file is a smell
- **An IR file named `surreal_ast.rs` / `*_ast.rs` that is the
  project's primary producer-side IR** — not an adapter, not a
  projection target, *the IR itself*. The name is the tell. Adapters
  are named adapters; spines are named IRs.
- **Test files asserting `parse(emit(parse(x))) == parse(x)` for
  *behavioral* roundtrips** — structural roundtrips through DDL are
  fine (the adapter doc says so); behavioral roundtrips cannot survive
  DDL, so the assertion only passes because of hijacked encoding
- **`From<DdlAst> for ActionDef`** / `to_action_def()` /
  `extract_lifecycle_from_ddl()` — any function with this shape is
  admitting that behavior is being smuggled through DDL
- **String-typed state machines**: `state: "pending" | "committed"`
  fields in DDL, with transitions implied by `DEFINE EVENT` triggers
  reading those strings
- **A `triple.rs` or `recompute_dag.rs` next to `surreal_ast.rs`** —
  the colocation pattern of the odoo-rs `od-ontology` fork. If you
  see the cluster, the IR has metastasized

A single signature is suspicious; two or more on the same crate is
the trap with high confidence.

## Remediation — if you're already inside

The trap is recoverable, but not by patching. The remediation is
three structural moves; trying to patch in-place re-creates the trap
at a different layer.

1. **Find the original producer.** Whose lifecycle was it really?
   Ruby `before_validation`? Odoo `@api.constrains`? Elixir
   `GenStateMachine.handle_event`? That source IS the truth — the
   DDL is just where its echo materialized. Read the original.

2. **Write an `ogar-from-<lang>` reader** that parses the original
   producer (Ruby AST / Python AST / Elixir AST / TypeScript AST)
   directly into `Class` + `ActionDef`. Behavior lifts cleanly here;
   no sentinels, no DDL hijack. This reader is the new canonical
   path — name it for what it does.

3. **Demote the SurrealQL form to egress only.** Keep emitting via
   `ogar-adapter-surrealql` (structural arm, lossy on behavior — fine;
   the adapter is *meant* to be lossy on the behavioral arm). Delete
   the DDL hijacks; they were never canonical, and the canonical reader
   from step 2 supersedes them. The fork (if any) collapses to the
   adapter call.

odoo-rs is the worked example: W3 of `APP-CODEBOOK-MIGRATION-PLAN.md`
already says this — lower `od-ontology::{surreal_ast,triple,emit}` onto
`ogar_vocab::Class`, emit via the canonical adapter, delete the fork.
The remediation is the migration plan.

## When this doc fires

For **author** sessions (before keyboard):
- Any producer → IR transcode work touching a language with lifecycle
  vocabulary (Ruby AR, Python `@api`, Ecto, Elixir, TypeScript classes)
- Any codegen session producing `.surql`, `DEFINE EVENT/FIELD/TABLE`,
  `surreal_ast.rs`, or anything named `*_ast.rs` as a *spine*
- Any "distill from SurrealQL" / "lower onto `ogar_vocab::Class`" task
- Any new `ogar-from-<lang>` reader authoring
- Any "behavior-preserving roundtrip" claim involving DDL

For **review** sessions (PR landing):
- Any PR introducing or modifying `.surql` files beyond row-trigger scope
- Any PR adding `From<DdlAst>` / `to_action_def()` conversions
- Any PR claiming `parse(emit(parse(x))) == parse(x)` for behavioral
  roundtrips
- Any PR that touches `surreal_ast.rs` / `triple.rs` / `recompute_dag.rs`
  in the same crate

For the **agent ensemble**:
- `core-first-architect` (loads at Knowledge Activation Tier-1)
- `adapter-shaper` (loads at Knowledge Activation Tier-1)
- `core-gap-auditor` (loads at Knowledge Activation Tier-1)
- The `Plan` subagent on any task mentioning the trigger phrases above

## Trigger phrases (for Knowledge Activation Protocol)

`SurrealQL AST` · `DDL spine` · `DEFINE EVENT` · `DEFINE FUNCTION` ·
`.surql` · `surreal_ast.rs` · `*_ast.rs` (as IR, not adapter) ·
`distill from SurrealQL` · `lower onto ogar_vocab::Class` ·
`lifecycle in schema` · `from_triples` · `recompute_dag` ·
`producer-side reader` · `behavioral roundtrip`

If the prompt mentions ANY of these, this doc is mandatory pre-read.

## What this doc is NOT

- **Not a ban on `.surql`.** SurrealDB is a real backing store; DDL is
  a real interop format; `ogar-adapter-surrealql` is the right emitter.
  The trap is using DDL as a *spine*, not as an *adapter*.
- **Not a ban on `DEFINE EVENT`.** Genuine row triggers are fine. The
  trap is `DEFINE EVENT` carrying lifecycle / state-machine semantics.
- **Not retroactive.** Code that already lives in the trap (the named
  W3 odoo-rs case) doesn't get a citation; it gets the three-move
  remediation. Citations are for future sessions.

## Cross-refs

- `docs/OGAR-CONSUMER-BEST-PRACTICES.md` — the muscle-memory guide:
  classid-address-vs-class-magic with worked examples across every
  consumer (medcare, woa, smb, odoo, openproject, redmine); the four
  canonical patterns; anti-pattern catalogue. Read this if you're
  working on a consumer-side path.
- `docs/SURREAL-AST-AS-ADAPTER.md` — the design decision (the *why*)
- `docs/OGAR-AST-CONTRACT.md` — the canonical IR (the *what*)
- `docs/THE-FIREWALL.md` — the structural reason DDL can't be the spine
  (ADR-022/023)
- `docs/ARCHITECTURE.md` — the AR-as-spine doctrine
- `docs/ADAPTERS-AND-ACTORS.md` — the Action / SPO+TeKaMoLo vocabulary
  that has no DDL home
- `docs/APP-CODEBOOK-MIGRATION-PLAN.md` W3 — the worked remediation case
  (odoo-rs)
- **Parallel-session sibling**: `lance-graph/.claude/knowledge/ogar-consumer-preflight.md`
  (lance-graph #591) — same spellbook pattern cast in lance-graph's
  consumer domain; ships the `ISS-CONTRACT-APP-PREFIX-MIRROR` Core-gap
  entry.
