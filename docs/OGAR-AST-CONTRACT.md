# OGAR AST Contract — the typed surface the ractor codegen lands on

> **Purpose.** Hand the actor/runtime session the *exact* type surface to
> generate against, so `lance-graph-callcenter`'s ractor codegen **lands on
> the OGAR Types** instead of declaring a parallel set. Without this contract
> the codegen and the IR drift and it's a mess. With it, "the class IS the
> actor spec" (ARCHITECTURE.md) becomes literal: the actor is *generated from*
> the `Class`, never hand-written.
>
> Companion / grounded in: `ARCHITECTURE.md` (Universal AST), `SOA-IMPLEMENTATION.md`
> (IR RecordBatch schemas + SurrealQL adapter + §5 runtime), `ADAPTERS-AND-ACTORS.md`
> (Action / SPO+TeKaMoLo), `IDENTITY-MAPPING.md` (Identity), `LANCE-GRAPH-INTEGRATION.md`
> (contract types). The companion `RACTOR-STATEM` design defines the `StateMachine`
> shim referenced in §3.
>
> Status: **CONTRACT v0** (2026-06-04). Cross-session handover.

## 0. The contract — two layers

Per the runtime session, the design splits cleanly and the split matters:

- **Generic `state_machine` crate** (lives in `ractor_actors`): **OGAR-agnostic.**
  `Context` is an opaque associated type the crate never inspects; `on_enter`
  delegates to a `CommitHook`. **Zero OGAR types** — so the OGAR IR can evolve
  without ever touching `ractor_actors`. *(Owned by the runtime session.)*
- **The OGAR/Rubicon binding** (the *only* layer that references OGAR types):
  fills `Context`/`Event`/`State`, supplies the `CommitHook` + a `Context`
  constructor, and is where the callcenter codegen lands.

**This document is the OGAR type surface + projection rules the binding maps
onto** — `Class`, `Action`, `Identity` (canonical), with SurrealQL DDL as a
bidirectional bridge (§2). It does **not** re-implement the binding (that's the
runtime session's, grounded in these types) and adds **no** parallel
actor/message/state types.

## 1. OGAR IR core types — the landing surface

These mirror the `ogar-vocab-soa` RecordBatch schemas 1:1 (SOA-IMPLEMENTATION §3).
The codegen consumes the struct form; storage is the columnar form; **same types**,
SoA round-trip is identity.

```rust
/// Layer-1 routing key. Prefix-radix NiblePath; 27-bit segments;
/// ALWAYS dictionary-encoded on storage. The domain prefix (§4) lives here.
pub struct Identity { /* segments: SmallVec<NibleSeg>; canonical per IDENTITY-MAPPING */ }

/// Structural arm. One `Class` == one actor (§3). Field set == class_record_batch_schema().
pub struct Class {
    pub identity: Identity,
    pub name: String,
    pub parent: Option<Identity>,          // subClassOf == supervision edge (ARCHITECTURE)
    pub language: Lang,                     // Ruby | Python | SurrealQL | Sql | ...
    pub declared_in_module: Option<String>,
    pub source_version: Option<String>,
    pub abstract_model: bool,
    pub transient: bool,
    // nested ListArrays of structs — SoA all the way down:
    pub associations:    Vec<Association>,  // BelongsTo / HasMany / MemberOf ...
    pub enums:           Vec<EnumDecl>,
    pub scopes:          Vec<Scope>,
    pub callbacks:       Vec<Callback>,
    pub computed_fields: Vec<ComputedField>,
    pub methods:         Vec<MethodDecl>,
    pub validations:     Vec<Validation>,
    pub attributes:      Vec<Attribute>,
}

/// Behavior arm. SPO + TeKaMoLo (ADAPTERS-AND-ACTORS §3). == action_record_batch_schema().
/// This IS the actor message (§3): `type Msg = ActionMsg` over this row.
pub struct Action {
    pub identity:  Identity,
    pub subject:   ActionSubject,   // User | System | Cron | Trigger | Cascade
    pub predicate: String,          // the event name == the StateMachine Event
    pub object:    Identity,        // the acted-upon Class
    pub temporal:  TemporalSpec,    // Immediate | Deferred | Scheduled | OnCommit | (StateTimeout*)
    pub kausal:    KausalSpec,      // StateGuard{field,value} | LifecycleTrigger{event} | DependsPath | None
    pub modal:     ModalSpec,       // Sync | Async | Idempotent | Atomic | Requires | (Postponable*)
    pub lokal:     Identity,        // the executing actor
    pub method_body: Option<String>,
    pub results_in:  Option<StateTransition>,  // ::S-to-T  → Transition::Next(T)
}
// (*) = the three sub-properties this contract asks the vocab to add — see §6.
```

`KausalSpec::StateGuard { field, value }` + `Action.results_in` are the **only**
places a state machine survives the IR flattening — see §3.

## 2. SurrealQL DDL AST bridge (Layer 3)

We **adapt** the upstream `surrealdb_core::sql::statements` AST; we do not
reimplement it (pin exact version; migrate to `surrealdb-parser`/`surrealdb-ast`
when on crates.io). The bridge is bidirectional and the IR is the two-way meeting
point — SurrealDB becomes the dev-facing DSL over lance-graph with no storage
duplication.

| SurrealQL AST node | → OGAR IR type |
|---|---|
| `DefineTable { name }` | `Class { identity: name }` |
| `DefineField TYPE record<x>` | `Association(BelongsTo, class = x)` |
| `DefineField TYPE string + ASSERT $value IN [...]` | `EnumDecl` |
| `DefineField TYPE option<X>` | `Attribute { required: false }` |
| `DefineField TYPE <scalar>` | `Attribute { type_name }` |

```rust
pub fn parse_surrealql_ddl(input: &str) -> Result<Vec<Class>>;   // producer
pub fn emit_surrealql_ddl(classes: &[Class]) -> String;          // consumer
// invariant: parse(emit(parse(x))) == parse(x)   (proptest)
```

## 3. The lowering contract — generic crate + OGAR binding

**This supersedes the hand-written `ClassActor::handle` in SOA-IMPLEMENTATION
§5.1.** The actor body is *generated*, not authored — and split so the generic
crate stays OGAR-free.

**Generic layer (runtime session, zero OGAR types).** `Context` opaque; `on_enter`
delegates to a `CommitHook`:

```rust
pub trait StateMachine {
    type State: Clone + PartialEq;
    type Event;
    type Context;                                  // OPAQUE — crate never inspects it
    fn handle(&mut self, st: &Self::State, ev: Self::Event,
              cx: &mut Self::Context, hook: &mut dyn CommitHook<Self>) -> Transition<Self>;
    fn on_enter(&mut self, _st: &Self::State, _cx: &mut Self::Context,
                _hook: &mut dyn CommitHook<Self>) {}
}
pub trait CommitHook<M: StateMachine + ?Sized> {   // the on_enter side-effect seam
    fn commit(&mut self, st: &M::State, cx: &M::Context);
}
```

**Binding layer (this contract's OGAR inputs).** The codegen fills the three
associated types from OGAR and supplies a `LanceCommitHook`:

```rust
// GENERATED per Action-bearing Class:
impl StateMachine for <Class>Invocation {
    type State   = ActionState;       // Pending -> Committed / Failed / Cancelled — lifecycle / Rubicon
    type Event   = ActionDef;         // transition decl: predicate, object_class, kausal, default temporal/modal
    type Context = ActionInvocation;  // state, object_instance, idempotency_key, trace_id, parent_invocation,
                                      // emitted_at_millis: Option<_> (decision #4 HLC), failure_reason, lokal
}
struct LanceCommitHook { membrane: LanceMembrane }   // LanceMembrane = callcenter's sole writer
impl CommitHook<_> for LanceCommitHook { /* on_enter(Committed): atomically apply ActionDef.results_in to
                                            object_instance + append the Lance version */ }
```

**Two-level state (resolved binding).** `State = ActionState` — the invocation
**lifecycle** the callcenter drives/audits; the Rubicon crossing is
`on_enter(Committed)` = the Lance commit. The **domain** workflow (`draft→sale`)
is *not* the machine state — it's a guarded **effect** on `object_instance`,
applied at the `Pending→Committed` crossing, gated by `KausalSpec::StateGuard`,
atomic under `ModalSpec::Atomic`. Lifecycle formalized; workflow as data.

| Machine construct | Sourced from |
|---|---|
| `State` = `ActionState` | the lifecycle enum (Pending/Committed/Failed/Cancelled — exists in vocab) |
| `Pending → Committed` | `KausalSpec::StateGuard` satisfied on `object_instance` |
| `Pending → Failed` | guard fails non-transiently / `ModalSpec` violation |
| `on_enter(Committed)` | `CommitHook`: apply `ActionDef.results_in` to `object_instance` + append Lance version (Atomic) — "state history IS the version log" |
| `Transition::Postpone` (stay `Pending`) | `StateGuard` fails *transiently* + `guardFailurePolicy = Postponable` |
| `state_timeout` on `Pending` | `TemporalSpec` (Scheduled/Deferred/`StateTimeout`) → SLA deadline; auto-cancels at the crossing |
| `Event` | `ActionDef` (predicate / object_class / kausal / defaults) |

Full authoritative binding record → `CROSS_SESSION_COORDINATION.md` (runtime
session); this section is the OGAR-side type surface it binds to.

**Hot-path constraint (I-2 invariant).** The shim's dispatch + postpone queue are
`std::sync` (`Mutex<VecDeque> + Condvar`), **never** `tokio`, on the hot loop.
`tokio` is reserved for Layer-3 cold sinks (SLA coord). The `ractor-statem` crate
must honor this — no `tokio::sync` in the generated dispatch/postpone path.

## 4. Universality — the same core carries every domain

The "flexible enough to be everything later" requirement is satisfied
structurally, not by special-casing:

- **Domain == `Identity` prefix.** Now: `ogit-op::` (OpenProject/Rails),
  `ogit-erp::` (Odoo). Later: `bardioc::`, `foundry::`, `wikidata-med::`.
- **Each domain's source AST maps onto the same `Class`/`Action`** via an
  `Adapter` (Ruby AR → lib-ruby-parser; Python Odoo → libcst; SQL DDL →
  sqlparser-rs; SurrealQL → surrealdb-core; new domains → a producer/TTL hydrator).
- **The codegen is domain-agnostic** — it lowers *any* `Class` → actor regardless
  of prefix. **Adding a domain = adding a producer/adapter, never touching the
  codegen or the core types.** That is the flexibility guarantee.

### 4.1 Attached now — Odoo + OpenProject on the same types

```
Odoo        ogit-erp::sale.order   action_confirm (draft→sale)
            → State {Draft, Sale, ...}  Event=confirm  Next(Sale)
            → on_enter(Sale): _send_order_confirmation_mail + Lance commit

OpenProject ogit-op::WorkPackage    before_save :touch_parent
            → Action(subject=Cascade, temporal=OnCommit)
            → cascade-emit to ogit-op::Project actor (kanban-bounded)

Cross-system: SaleOrder.workPackage  → one lance-graph traversal
            (both extend OGAR under the same prefix-radix; both are
             statem actors over the one append-only log)
```

### 4.2 Extends later — same types, new prefixes

| Target | object → `Class` | link → `Association` | action → `Action` | how it attaches |
|---|---|---|---|---|
| Palantir Foundry | object-type | link-type | action-type | adapter (Foundry's ontology *is* this shape) |
| Wikidata-medical | item | property | — | TTL hydrator (joins SKOS/PROV-O/schema.org/FIBO/Odoo/ZUGFeRD/SKR03-04 already in `lance-graph-ontology`); planet-scale fits single-node (compression-to-the-floor — "Wikidata fits") |
| bardioc (NEW-stack) | capability shape | — | — | `bardioc::` prefix; **consumer-internal specs stay out of lance-graph** (guardrail) |

**AGI aspiration (the substrate wiring):** "a thought is a Raft commit" —
distributed cognition replicates the frozen *generator* (semantics + syntax) and
re-runs the *wave* (pragmatics/actors) locally. The Rubicon statem over the
append-only log is the wiring; this contract is the type surface it runs on. Same
core, no new substrate.

## 5. Anti-mess carve-outs (non-negotiable — these keep ractor ON the OGAR types)

1. **The generic `state_machine` crate references zero OGAR types** —
   `State`/`Event`/`Context` are associated types; *all* OGAR coupling lives in
   the binding (so the IR evolves without touching `ractor_actors`). The
   **binding** emits `impl StateMachine` over OGAR `Class`/`Action`/`Identity` —
   no parallel hand-rolled actor/message/state types. (Supersedes the
   hand-authored `ClassActor::handle`.)
2. `type Event = ActionMsg` — the message **is** the `Action` (SPO+TeKaMoLo).
   Actors never invent message enums.
3. `type State` is **derived** from the Class's `StateGuard` values, never
   hand-declared.
4. `type Data` is the Class-instance SoA row (Arrow-scalar). No row-form structs.
5. Routing is by `Identity` NiblePath only.
6. Hot path `std::sync` (Condvar). The statem shim never pulls `tokio` into
   dispatch/postpone. `tokio` = Layer-3 cold only.
7. Inter-actor wire form is RecordBatch IPC — N actions = 1 batch.

## 6. Vocab extensions needed from the ontology session

Three semantics don't survive the Action-flattening; each is a sub-property of an
existing TeKaMoLo slot (carve-out: "seven slots, no eighth"):

| New term | Slot | Lowers to |
|---|---|---|
| `ogar:onEnter` | (entry effect) | `StateMachine::on_enter` → the Lance commit |
| `ogar:guardFailurePolicy = Postponable` | Modal | `Transition::Postpone` (vs reject/`raise`) |
| `ogar:StateTimeout` | Temporal | gen-stamped `state_timeout` (auto-cancel on transition) |

Without these, the codegen can re-assemble states/events/transitions from the
existing triples, but **cannot** express enter-effects, postpone, or per-state
SLA timeouts — i.e. the Rubicon machine can't be generated faithfully.

## 7. Open decisions / what this session needs back

1. **Scope:** is the statem lowering for *every* `Class` with a state field (all
   `Workflow.Transition` classes), or **Rubicon-only**? Sets projection breadth.
2. **`LanceMembrane` sole-writer signature** — to type `CommitHook::commit` (the
   `on_enter` Lance commit). Callcenter owns it; the binding calls it. This is the
   one binding seam I cannot type without the runtime/ontology session.
3. **Fork access:** land `ractor-statem` in `AdaWorldAPI/ractor_actors` (over the
   `ractor` core fork). Needs scope/PyGithub.
4. **Timing:** pin this contract into the **Sprint-7** callcenter design *before*
   the hand-loop is built, or it gets re-done.
