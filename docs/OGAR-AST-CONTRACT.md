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
onto** — `Class`, `ActionDef` / `ActionInvocation`, `Identity` (canonical), with
SurrealQL DDL as a bidirectional bridge (§2). It does **not** re-implement the
binding (that's the runtime session's, grounded in these types) and adds **no**
parallel actor/message/state types.

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
    pub parent: Option<Identity>,            // subClassOf == supervision edge (ARCHITECTURE)
    pub language: Language,                  // Ruby | Python | SurrealQl | Sql | TypeScript | Elixir | Unknown
    pub declared_in_module: Option<String>,
    pub source_version: Option<String>,
    pub description: Option<String>,
    pub record_order: Option<String>,
    pub rec_name: Option<String>,
    pub abstract_model: bool,
    pub transient: bool,
    pub auto_create_table: Option<bool>,
    pub log_access: Option<bool>,
    pub inheritance_column_disabled: bool,
    pub mixins: Vec<Identity>,               // structural mixins (Elixir use_, Ruby include)
    pub store_accessors: Vec<String>,        // store-backed accessors (Rails, Ecto embeds)
    // nested ListArrays of structs — SoA all the way down:
    pub associations:    Vec<Association>,   // BelongsTo / HasMany / MemberOf ...
    pub enums:           Vec<EnumDecl>,
    pub scopes:          Vec<Scope>,
    pub callbacks:       Vec<Callback>,
    pub computed_fields: Vec<ComputedField>,
    pub methods:         Vec<MethodDecl>,
    pub validations:     Vec<Validation>,
    pub attributes:      Vec<Attribute>,
}

/// Behavior arm — STATIC declaration (one per transition / handler). SPO + TeKaMoLo
/// (ADAPTERS-AND-ACTORS §3). Carries the four Sprint-7 statem terms landed in #10:
/// `ogar:onEnter`, `ogar:guardFailurePolicy`, `ogar:StateTimeout` (+ `stateTimeoutMillis`).
pub struct ActionDef {
    pub identity:    Identity,
    pub predicate:   String,              // event name
    pub object_class: Identity,           // acted-upon Class
    pub default_subject:  ActionSubject,  // User | System | Cron | Trigger | Cascade
    pub default_temporal: TemporalSpec,   // Immediate | Deferred | Scheduled | OnCommit | StateTimeout
    pub default_modal:    ModalSpec,      // Sync | Async | Idempotent | Atomic | Requires
    pub kausal:           KausalSpec,     // StateGuard{field,value} | LifecycleTrigger | DependsPath | None
    pub method_body:      Option<String>,
    pub results_in:       Option<StateTransition>,  // ::S-to-T → applied at Pending→Committed
    // statem terms (vocab/ogar.ttl, PR #10):
    pub on_enter:               Option<EnterEffect>,             // ogar:onEnter (typed; range tightened from xsd:string)
    pub guard_failure_policy:   GuardFailurePolicy,             // Reject (default) | Postponable
    pub state_timeout_millis:   Option<i64>,                    // ogar:stateTimeoutMillis
}

/// Behavior arm — DYNAMIC instance (one per fire). Carries lifecycle + provenance;
/// `state` is the StateMachine state per §3 (lifecycle = Pending → Committed/Failed/Cancelled).
pub struct ActionInvocation {
    pub identity:          Identity,
    pub realizes:          Identity,                  // → ActionDef.identity
    pub state:             ActionState,               // StateMachine::State (lifecycle)
    pub subject:           ActionSubject,             // provenance
    pub object_instance:   Identity,                  // the specific instance acted on
    pub lokal:             LokalSpec,                 // {actor, tenant, company}
    pub idempotency_key:   Option<String>,            // OLD↔NEW correlation handle for §14 roundtrip
    pub trace_id:          Option<String>,
    pub parent_invocation: Option<Identity>,
    pub emitted_at_millis: Option<i64>,               // decision #4 — keep Option<_> until HLC lands
    pub failure_reason:    Option<String>,
}
// (statem terms above are the three §6 vocab terms LANDED in PR #10's vocab/ogar.ttl.)
```

`KausalSpec::StateGuard { field, value }` (on `ActionDef`) + `ActionDef.results_in`
are the **only** places a domain workflow survives the IR flattening; the
**lifecycle** state machine is carried by `ActionInvocation.state: ActionState` —
see §3.

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

**Generic layer (runtime session, zero OGAR types).** `Context` opaque; the hook
is wired at spawn (no I/O surface threaded through `on_event`). Canonical
signatures from `AdaWorldAPI/ractor_actors` `feat/state-machine-actor` @ `38a71a4`:

```rust
pub trait StateMachine: Send + Sync + 'static {
    type State:   Clone + PartialEq + std::fmt::Debug + Send + Sync + 'static;
    type Event:   Send + 'static;
    type Context: Send + 'static;          // OPAQUE — crate never inspects

    fn initial(&self) -> Self::State;

    /// Pure dispatch — guards live here; no hook param, no I/O surface.
    fn on_event(&self,
                state: &Self::State,
                event: &Self::Event,
                ctx:   &mut Self::Context) -> Transition<Self::State>;

    /// Per-state SLA deadline. Lowering target for `ogar:StateTimeout`.
    fn timeout(&self, _state: &Self::State)
        -> Option<ractor::concurrency::Duration> { None }

    /// Timer fired. May NOT return `Postpone` (timeouts don't replay).
    fn on_timeout(&self, _state: &Self::State, _ctx: &mut Self::Context)
        -> Transition<Self::State> { Transition::Stay }

    /// Commit predicate. Entering a state where this is `true` fires `CommitHook`.
    fn is_commit(&self, _state: &Self::State) -> bool { false }
}

pub enum Transition<S> {
    Goto(S),    // if is_commit(new): CommitHook fires; then postponed replay (FIFO)
    Stay,       // no-op / guard rejects this event
    Postpone,   // defer until next state change; replay FIFO ahead of newer events
    Stop,       // stop the actor
}

/// Side-effect seam. SYNC + fallible (honours I-2 — no tokio at the membrane).
pub trait CommitHook<SM: StateMachine>: Send + Sync + 'static {
    fn on_commit(&self,
                 from: &SM::State,
                 to:   &SM::State,
                 ctx:  &SM::Context) -> Result<(), ractor::ActorProcessingErr>;
}

// spawn / fire / current_state — see crate for the actor wiring.
```

**Binding layer (this contract's OGAR inputs).** The codegen fills the three
associated types from OGAR and supplies a `LanceCommitHook`:

```rust
// GENERATED per Action-bearing Class:
impl StateMachine for <Class>Invocation {
    type State   = ActionState;       // Pending → Committed / Failed / Cancelled — lifecycle / Rubicon
    type Event   = ActionDef;         // realized as ActionInvocation via `realizes`
    type Context = ActionInvocation;  // state, object_instance, idempotency_key, trace_id,
                                      // parent_invocation, emitted_at_millis: Option<_> (decision #4 HLC),
                                      // failure_reason, lokal

    fn is_commit(&self, s: &ActionState) -> bool { matches!(s, ActionState::Committed) }
    fn timeout(&self, _s: &ActionState) -> Option<Duration> {
        self.def.state_timeout_millis.map(Duration::from_millis)   // ogar:StateTimeout carrier
    }
    // on_event: KausalSpec guard → Goto(Committed) | Stay | Postpone(if guardFailurePolicy=Postponable) | Goto(Failed)
}

struct LanceCommitHook { membrane: LanceMembrane }   // LanceMembrane = callcenter's sole writer
impl CommitHook<<Class>Invocation> for LanceCommitHook {
    fn on_commit(&self, _from: &ActionState, _to: &ActionState, ctx: &ActionInvocation)
        -> Result<(), ActorProcessingErr>
    {
        // Atomic: apply ActionDef.on_enter to ctx.object_instance + append the Lance version.
        let row = CognitiveEventRow::from(ctx);
        self.membrane.commit_event(row);   // sibling on LanceMembrane (runtime session, gate-1 follow-up)
        Ok(())
    }
}
```

**Two-level state (resolved binding).** `State = ActionState` — the invocation
**lifecycle** the callcenter drives/audits; the Rubicon crossing is
`Goto(Committed)` where `is_commit(Committed) = true` → `CommitHook::on_commit`
= the Lance commit. The **domain** workflow (`draft→sale`) is *not* the machine
state — it's a guarded **effect** on `object_instance`, applied at the
`Pending→Committed` crossing, gated by `KausalSpec::StateGuard`, atomic under
`ModalSpec::Atomic`. Lifecycle formalized; workflow as data.

| Machine construct | Sourced from |
|---|---|
| `State` = `ActionState` | the lifecycle enum (Pending/Committed/Failed/Cancelled — `vocab/ogar.ttl::ActionStateKind`) |
| `Event` | `ActionDef` (predicate / object_class / kausal / defaults / on_enter / guard_failure_policy / state_timeout_millis) |
| `on_event` (guard) | `KausalSpec::StateGuard` satisfied on `object_instance` → `Goto(Committed)`; transient fail + `guardFailurePolicy=Postponable` → `Postpone`; hard fail → `Goto(Failed)` |
| `is_commit(Committed)` | true → fires `CommitHook::on_commit` (the only commit gate) |
| `CommitHook::on_commit` | apply `ActionDef.on_enter` to `object_instance` + `LanceMembrane::commit_event` (Atomic) — "state history IS the version log" |
| `timeout(Pending)` | `ActionDef.state_timeout_millis` (`ogar:StateTimeout`) → per-state SLA deadline |
| `on_timeout(Pending)` | `Goto(Failed)` (or domain-specific `Stay`); **never** `Postpone` |

Full authoritative binding record → `CROSS_SESSION_COORDINATION.md` (runtime
session); this section is the OGAR-side type surface it binds to.

**Hot-path constraint (I-2 invariant).** `CommitHook::on_commit` is **sync +
fallible** (returns `Result<(), ActorProcessingErr>`) — no tokio at the
membrane. The actor's dispatch + postpone replay are `std::sync`; `tokio` is
reserved for Layer-3 cold sinks. The `state_machine` crate honors this; the
binding's `CommitHook` impl must not introduce an `await`.

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
   **binding** emits `impl StateMachine` over OGAR `Class`/`ActionDef`/
   `ActionInvocation`/`Identity` — no parallel hand-rolled actor/message/state
   types. (Supersedes the hand-authored `ClassActor::handle`.)
2. **`type Event = ActionDef`** — the event is the *static* transition
   declaration, not a bespoke enum. Actors never invent message enums.
3. **`type State = ActionState`** — the *lifecycle* enum (Pending → Committed /
   Failed / Cancelled), drawn from `vocab/ogar.ttl` `ActionStateKind`. Never
   derived from per-class `StateGuard` values; never hand-declared. Domain
   workflow rides as guarded effect on `ActionInvocation.object_instance`, not
   as machine state.
4. **`type Context = ActionInvocation`** — the *dynamic* invocation row
   (state, object_instance, idempotency_key, trace_id, parent_invocation,
   emitted_at_millis, failure_reason, lokal). No row-form structs.
5. **`ActionDef` and `ActionInvocation` stay split** — never collapsed into a
   single `Action` type. Static decl ≠ dynamic instance; `realizes` is the link.
6. Routing is by `Identity` NiblePath only.
7. Hot path `std::sync` (Condvar). The statem shim never pulls `tokio` into
   dispatch/postpone. `tokio` = Layer-3 cold only.
8. Inter-actor wire form is RecordBatch IPC — N invocations = 1 batch.

## 6. Vocab extensions — LANDED in PR #10

Three semantics didn't survive the Action-flattening; each is now a sub-property
of an existing TeKaMoLo slot in `vocab/ogar.ttl` (carve-out: "seven slots, no
eighth"), shipped in `AdaWorldAPI/OGAR#10` and carried on `ActionDef` (§1):

| Term | Slot | Carrier field | Lowers to |
|---|---|---|---|
| `ogar:onEnter` | (entry effect) | `ActionDef.on_enter: Option<EnterEffect>` (typed since PR #13) | `StateMachine::on_enter(Committed)` → `CommitHook` → Lance append (the Rubicon crossing) |
| `ogar:guardFailurePolicy` ∈ `{Postponable, Reject}` | Modal | `ActionDef.guard_failure_policy: GuardFailurePolicy` | `Postponable` → `Transition::Postpone` (stay `Pending`); `Reject` (default) → `Pending → Failed` |
| `ogar:StateTimeout` (+ `stateTimeoutMillis`) | Temporal | `ActionDef.state_timeout_millis: Option<i64>` | per-state SLA on `Pending`; gen-stamped timer auto-cancels at the crossing |

Without these the codegen could re-assemble states/transitions from existing
triples but couldn't express enter-effects, postpone, or per-state SLA timeouts.
**Now expressible end-to-end**, modulo the §7 runtime-side pieces.

Companion: **`docs/ELIXIR-HIRO-PREFETCH.md`** (also #10) — the OLD-Elixir-stack
→ OGAR debt ledger; `gen_statem` `state_enter`/`postpone`/state-timeout lower
onto exactly these three terms, so every migration debt has a type home before
Sprint 4.5 `unmap` lands. `Language::Elixir` is first-class in `ogar-vocab`.

Typed `EnterEffect` promotion (`xsd:string → ogar:EnterEffect`) — **CLOSED in
PR #13**. `ActionDef.on_enter` is now `Option<EnterEffect>` with `field` +
`to_value` fields; codegen applies the lifecycle-visible transition
structurally instead of string-parsing. Further tightening of `to_value` to
typed values beyond strings remains a tracked follow-up.

## 7. Status of the open seams

1. **Scope — RESOLVED.** The lowering covers **every Action-bearing
   invocation**, not Rubicon-only. The lifecycle (`Pending → Committed/Failed/
   Cancelled`) is universal to every `ActionInvocation`; "Rubicon" is just the
   name for the `Pending → Committed` crossing, not a special subset. Codegen
   is uniform — same lowering for Odoo `action_confirm`, OpenProject
   `before_save`, and any future domain.
2. **`LanceMembrane` sole-writer signature — ANSWERED, runtime-side to land.**
   `LanceMembrane` adds a sibling to the zero-dep `ExternalMembrane::project`:
   ```rust
   fn commit_event(&self, row: Self::Commit /* = CognitiveEventRow */) -> u64
   ```
   returning the new Lance version. Action commits skip the cognitive-cycle
   `ShaderBus` shape. **This is the precise `CommitHook::on_commit` call
   target** (see §3's binding example), to be added in `lance-graph-callcenter`
   by the runtime session. Until then the binding's `LanceCommitHook` impl
   compiles against a trait-object/stub for `LanceMembrane`.
3. **Fork access — RESOLVED.** `GH_TOKEN` PAT authenticates as `AdaWorldAPI`
   with push on `AdaWorldAPI/{OGAR, ractor, ractor_actors, lance-graph,
   bardioc}`. The generic `state_machine` crate lives in `ractor_actors/`
   (cargo-workspace member, pushed at `feat/state-machine-actor` @ `38a71a4` —
   7/7 tests green, clippy clean, includes the load-bearing postpone-replay
   test); the binding lives in `lance-graph-callcenter`.
4. **§3 signatures — RECONCILED (PR #11).** §3 now carries the canonical
   `on_event(state, event, ctx) -> Transition`, `is_commit(state) -> bool`,
   `timeout(state)` + `on_timeout(state, ctx)`, `Transition::{Goto, Stay,
   Postpone, Stop}`, and `CommitHook::on_commit(from, to, ctx) -> Result<(), ActorProcessingErr>`
   (sync + fallible — honours I-2) from `ractor_actors` `feat/state-machine-actor`
   @ `38a71a4`. The OGAR-side surface (§1, §5, §6) is unchanged by the
   reconciliation.
5. **Sprint-7 timing — STILL APPLIES.** Pin this contract into the
   `lance-graph-callcenter` design *before* any hand-loop dispatch is written.
