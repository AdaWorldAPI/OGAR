# Architectural Decisions — Session 2026-06-04

> **Purpose.** ADR-style capture of every architectural decision made
> during the 2026-06-04 OGAR session. The companion forward-looking
> doc (`docs/SUBSTRATE-ENDGAME.md`) maps *where the substrate is
> going*; this doc records *why we got to the design we did*, in the
> ADR convention (Context / Decision / Alternatives Considered /
> Consequences / Status / References).
>
> **Why both docs.** Future sessions need both: the endgame doc
> answers "what are we building toward"; this doc answers "why was X
> decided this way / what was considered and ruled out / what does
> this constrain." The two compose; read in either order.
>
> **Cross-session memory.** Several decisions span repos
> (OGAR / lance-graph / ractor_actors / openproject-nexgen-rs /
> surrealdb fork / ruff fork). Each ADR records the cross-repo
> references so future sessions in any of those repos can trace back
> to the reasoning. Bardioc's `CROSS_SESSION_COORDINATION.md` is the
> runtime-session-owned mirror for cross-repo coordination; the
> ADRs here are the OGAR-side authoritative record.
>
> **Status:** **CARVED v0** (2026-06-04, doc committed 2026-06-05).
> Append-only — new decisions land as new ADRs; existing ones get
> a "Supersededby" link if revisited.

## Index of decisions

| # | Decision | Status | PRs / refs |
|---|---|---|---|
| ADR-001 | `State = ActionState` (lifecycle), not domain state, for Rubicon binding | **Pinned** | OGAR PR #9 (contract §3); confirmed via Rubicon Phase 1 |
| ADR-002 | `ActionDef` and `ActionInvocation` stay split — never collapse | **Pinned** | OGAR PR #13 (carve-out 5) |
| ADR-003 | `Class` field set 1:1 with `class_record_batch_schema` (full structural fidelity) | **Pinned** | OGAR PR #13 (Codex P2 #2) |
| ADR-004 | Three §6 Rubicon-statem vocab terms as TeKaMoLo sub-properties (`onEnter` / `guardFailurePolicy` / `StateTimeout`) | **Pinned** | OGAR PR #10 |
| ADR-005 | `EnterEffect { field, to_value }` is typed (not free-form `xsd:string`) | **Pinned** | OGAR PR #13 |
| ADR-006 | `EnterEffect` is `#[non_exhaustive]` per vocab forward-compat convention | **Pinned** | OGAR PR #15 (Codex P2 follow-up) |
| ADR-007 | §3 signatures = canonical `ractor_actors::state_machine` (`on_event` / `is_commit` / `timeout` / `on_timeout` / `Transition::{Goto,Stay,Postpone,Stop}` / sync-fallible `on_commit`) | **Pinned** | OGAR PR #11; ractor_actors `feat/state-machine-actor` @ `38a71a4` |
| ADR-008 | `LanceMembrane::commit_event(row) -> u64` as the `CommitHook::on_commit` target (sibling to `ExternalMembrane::project`) | **Pinned** | lance-graph PR #467 (merged); OGAR PR #11 §7 item 2 |
| ADR-009 | `lance-graph-planner::temporal` = the deinterlace engine with two causal axes (TIME via HLC + DATA via `DependsClosure`) | **Pinned** | lance-graph PR #468; OGAR PR #16 §10.3 |
| ADR-010 | `knowable_from` meet-point single ownership: sourced by `ogar-adapter-surrealql`, consumed by `temporal::classify`, nowhere else | **Pinned** | OGAR PR #16 §10.3; lance-graph PR #468 |
| ADR-011 | Two-arm naming pattern for producers (`ruff_<lang>_spo` narrow + `ogar-from-<lang>` wide) | **Pinned** | OGAR PR #16 §10.1 |
| ADR-012 | nexgen `op-surreal-ast` is a special case of `Class → catalog::TableDefinition`, not a collision | **Pinned** | OGAR PR #16 §10.2 |
| ADR-013 | `has_paper_trail` is a duplicate of Lance versions — substrate consolidation | **Pinned** | OGAR PR #14 §4 observation |
| ADR-014 | Data-driven FSMs (OP `Workflow`, Roles, Permissions) need database-hydrator pattern (generalize ontology TTL hydrators to AR seed data) | **Pinned** | OGAR PR #14 §7 |
| ADR-015 | `Language` enum is the extension point for source-AST tags (Elixir landed via PR #10, Rust deferred as one-line follow-up) | **Pinned** | OGAR PR #10 + PR #12 §1 |
| ADR-016 | SurrealQL DDL AST is **not** the universal IR (structural arm canonicalizes via projection; behavioral arm OGAR-owned) | **Pinned** | OGAR PR #19 |
| ADR-017 | `surrealdb-parser` cross-repo dep deferred behind `surrealdb-parser` feature flag pending OGAR rust-version bump (1.85 → 1.95+) | **Pinned** | OGAR PR #18 |
| ADR-018 | Kanban-as-polyglot-dispatcher during migration (six work-item form variants; per-actor §14-oracle-gated graduation) | **Pinned** | OGAR PR #19 §6 + PR #20 §2 |
| ADR-019 | OP-as-operator-pane is the substrate's self-hosting destination (Room 3 of endgame) | **Pinned** | OGAR PR #20 §3 |
| ADR-020 | SDK endgame is deeper than Foundry going OSS via three structural differentiators (migration scaffold, self-hosting reference, substrate-layer OSS) | **Pinned** | OGAR PR #20 §5.3 |
| ADR-021 | **Meta-hygiene**: always grep peer crates before copying manifest patterns (the `[lints] workspace = true` cascade lesson) | **Pinned** | OGAR PR #15 + PR #17/#18 follow-ups |

## ADR-001: `State = ActionState` (lifecycle), not domain state, for Rubicon binding

**Status:** Pinned (2026-06-04). Confirmed via Rubicon Phase 1 (`one_machine_type_drives_every_domain` test green).

**Context.** When building the OGAR/Rubicon binding for the Rubicon
state machine, there were two candidate state notions in the vocab:

- (a) **invocation lifecycle** — `ActionState {Pending, Committed, Failed, Cancelled}`, universal to every invocation;
- (b) **domain workflow** — the `KausalSpec::StateGuard` field (e.g. `draft → sale → done`), per-class.

The contract §3 sketch had to commit one of them to `State` in the
generic `state_machine` crate's `impl StateMachine for <Class>Invocation`.

**Decision.** `State = ActionState` (the lifecycle).

**Alternatives considered.**
- (b) Domain workflow — would have made the machine domain-specific
  (one state enum per Class), forcing the codegen to emit a new
  `State` type per Class. Also conflates the substrate-universal
  lifecycle (Pending → Committed) with per-class behaviour. Rejected:
  per-class state types make the machine non-uniform and the codegen
  per-class custom.
- A two-level state (lifecycle outer + domain inner) — possible but
  the generic crate would need to know about both levels. Rejected
  for the same uniformity reason; the cleaner separation is
  "lifecycle = machine state; domain = guarded effect on
  `object_instance`."

**Consequences.**
- **Universal codegen path**: `Rubicon-from-OGAR` produces the same
  shape of `impl StateMachine` for every Action-bearing class —
  Odoo `action_confirm`, chess ply, OP `WorkPackage#save`, HIRO
  `gen_statem` callback. Confirmed by Rubicon Phase 1's
  `one_machine_type_drives_every_domain` test.
- **Domain workflow rides as data**: `KausalSpec::StateGuard` gates
  `Pending → Committed` transitions; `EnterEffect` applies the
  domain mutation (e.g. `state := "sale"`) on entering Committed;
  `object_instance` field is the carrier.
- **Lifecycle-formalized observability**: every actor's
  state-transition count, Failed rate, Postpone retry rate,
  StateTimeout hit rate are all uniform metrics — no per-class
  custom instrumentation.
- **"Rubicon" naming clarified**: the Rubicon crossing is the
  `Pending → Committed` transition specifically (the only
  `is_commit` state). Not a special subset of actors; every
  Action-bearing invocation has one.

**References.**
- OGAR `docs/OGAR-AST-CONTRACT.md` §3 (the binding).
- OGAR PR #9 (contract merged).
- Rubicon Phase 1 (other session's verified crate, awaiting durable home).

## ADR-002: `ActionDef` and `ActionInvocation` stay split — never collapse into "Action"

**Status:** Pinned (2026-06-04). Codified as `OGAR-AST-CONTRACT.md §5` carve-out 5.

**Context.** Early drafts of OGAR's contract had a single `Action`
type carrying both static declaration and dynamic invocation fields.
Codex P2 review on PR #13 flagged that the canonical vocabulary
(in `vocab/ogar.ttl`) already split these into `ActionDef` (static
declaration) and `ActionInvocation` (dynamic instance) with
`realizes` as the link — and the proposed unified `Action` type
would either fail to compile against the existing `ogar-vocab`
structs or drop invocation-only data (lifecycle state, provenance).

**Decision.** `ActionDef` and `ActionInvocation` are separate types,
linked by `ActionInvocation.realizes: Identity → ActionDef.identity`.

**Alternatives considered.**
- Single `Action` type with optional fields covering both static
  and dynamic concerns. Rejected: makes invariants harder to enforce
  (e.g. "static decl must have predicate; dynamic instance must have
  state"); diverges from the existing `vocab/ogar.ttl`; opens
  ambiguity about which fields are populated when.
- `ActionDef` only, with `ActionInvocation` as a typed view over
  Lance rows (no in-memory struct). Rejected: Rubicon's
  `Context = ActionInvocation` requires it as a first-class struct
  the binding can mutate.

**Consequences.**
- **Carve-out**: `OGAR-AST-CONTRACT.md §5` item 5 codifies "never
  collapse into a single `Action` type." Reviewable rule.
- **Codegen surface**: `Rubicon-from-OGAR` distinguishes `Event =
  ActionDef` (the static decl driving dispatch) from `Context =
  ActionInvocation` (the dynamic row carrying state + provenance).
- **Vocab alignment**: `vocab/ogar.ttl` has the parallel split
  with `ogar:realizes` as the predicate; OGAR's Rust types mirror it
  1:1.
- **Round-trip integrity**: emit / parse cycles preserve both types
  without information loss; the link via `realizes` is the only
  required cross-reference.

**References.**
- OGAR PR #13 (Codex P2 #1 reply).
- OGAR `docs/OGAR-AST-CONTRACT.md` §1 (the typed definitions), §5 (carve-out 5).
- `ogar-vocab/src/lib.rs` (`pub struct ActionDef`, `pub struct ActionInvocation`).

## ADR-003: `Class` field set 1:1 with `class_record_batch_schema` (full structural fidelity)

**Status:** Pinned (2026-06-04).

**Context.** Early drafts of OGAR's `Class` struct omitted several
fields present in the canonical RecordBatch schema and the existing
Rust struct: `description`, `record_order`, `rec_name`,
`auto_create_table`, `log_access`, `inheritance_column_disabled`,
`mixins`, `store_accessors`. Codex P2 on PR #13 caught that the
contract claimed "1:1 with `class_record_batch_schema()`" while the
shown definition omitted these.

**Decision.** OGAR-AST-CONTRACT.md §1's `Class` definition lists
**every** field present in the `ogar-vocab` Rust struct, in the same
order, with the same types.

**Alternatives considered.**
- "Essential fields only" minimal `Class` — rejected because it
  introduces the question "what's essential?" and producers extracted
  metadata that doesn't fit gets silently dropped.
- Multiple `Class` shapes for different purposes (`MinimalClass`,
  `FullClass`) — rejected as type-proliferation; the structural arm
  has one canonical IR.

**Consequences.**
- **No silent metadata loss**: when `ogar-from-ruby` extracts a
  Rails model with `acts_as_*` decorators, mixins (concerns), and
  store accessors, all of it lands on the canonical `Class`.
- **Roundtrip fidelity**: emit / parse cycles preserve the full
  `Class` shape without information loss.
- **Codegen completeness**: downstream consumers (the `From<Class>
  for catalog::TableDefinition` projection per ADR-016, the
  `ogar-emitter` triple emission) can rely on the full field set
  being available.
- **Carve-out**: §5 item 5 (this PR's amendment).

**References.**
- OGAR PR #13 (Codex P2 #2 reply).
- `crates/ogar-vocab/src/lib.rs` (`pub struct Class`).
- OGAR `docs/OGAR-AST-CONTRACT.md` §1.

## ADR-004: Three §6 Rubicon-statem vocab terms as TeKaMoLo sub-properties

**Status:** Pinned (2026-06-04).

**Context.** OGAR's Action vocabulary (`ADAPTERS-AND-ACTORS.md §3`)
defines seven SPO+TeKaMoLo slots and explicitly disallows an
eighth ("seven slots, no eighth" carve-out). But the Rubicon
state-machine lowering needs three behavioral concepts the existing
slots don't capture: entry effects (on-enter), guard-failure
disposition (postpone vs reject), and per-state SLA timeouts.

**Decision.** Three new terms in `vocab/ogar.ttl`, each as a
**sub-property of an existing TeKaMoLo slot**:

- `ogar:onEnter` — entry effect (sub-property of "entry effect"
  carrier on `ActionDef`); lowers to `StateMachine::on_enter`
  (later refined to `is_commit` per ADR-007).
- `ogar:guardFailurePolicy` ∈ `{Postponable, Reject}` — Modal sub-
  property; lowers to `Transition::Postpone` vs `Pending → Failed`.
- `ogar:StateTimeout` (with `ogar:stateTimeoutMillis`) — Temporal
  sub-property; lowers to per-state `state_machine::timeout()`.

Carrier fields on `ogar-vocab::ActionDef`: `on_enter`,
`guard_failure_policy`, `state_timeout_millis`.

**Alternatives considered.**
- Adding new top-level TeKaMoLo slots — rejected by the "seven
  slots, no eighth" carve-out.
- Hijacking existing slots (e.g. encoding state-timeout in
  `TemporalSpec::Scheduled.cronInterval`) — rejected as semantic
  collision (Scheduled has a different meaning).
- Out-of-band side-cars (separate structs not on `ActionDef`) —
  rejected because the Rubicon binding needs them per-ActionDef;
  side-cars complicate codegen.

**Consequences.**
- **Vocab carve-out preserved**: the seven-slot constraint holds;
  sub-properties extend without violating the slot count.
- **Codegen target**: `Rubicon-from-OGAR` reads these three fields
  off `ActionDef` and emits `is_commit` / `timeout` /
  `Transition::Postpone` paths.
- **Producer responsibility**: Elixir / Ruby / Python frontends
  populate these fields when the source language has equivalent
  constructs (gen_statem `state_enter` → onEnter; `[:postpone]` →
  Postponable; `[{:state_timeout, ms, _}]` → StateTimeout).
- **Round-trip integrity**: triples (per `ogar-emitter`) carry the
  three fields as the typed shape (structural emission via link +
  two leaf triples per ADR-005's `EnterEffect`).

**References.**
- OGAR PR #10 (terms shipped in `vocab/ogar.ttl`).
- `crates/ogar-vocab/src/lib.rs` (carrier fields).
- OGAR `docs/OGAR-AST-CONTRACT.md` §6 (after PR #13 marked CLOSED).

## ADR-005: `EnterEffect { field, to_value }` is typed (not free-form `xsd:string`)

**Status:** Pinned (2026-06-04). Supersedes the v0 free-form `Option<String>`.

**Context.** Initial PR #10 had `ogar:onEnter` ranging `xsd:string` and
`ActionDef.on_enter: Option<String>`. Codegen would have to string-
parse the effect at the load-bearing `on_commit` site, which is
where "state history IS the version log" happens. The runtime
session flagged this as the single highest-leverage thing OGAR could
ship for the codegen's clean path.

**Decision.** Promote `ActionDef.on_enter` to `Option<EnterEffect>`
where:

```rust
pub struct EnterEffect {
    pub field: String,        // field on object_instance being set
    pub to_value: String,     // string-encoded; future typing of
                              // to_value remains a tracked follow-up
}
```

`vocab/ogar.ttl` `ogar:onEnter` range tightens from `xsd:string` to
`ogar:EnterEffect` (new owl:Class with `ogar:enterField` and
`ogar:enterToValue` properties).

**Alternatives considered.**
- Keep `Option<String>` and write a parser at each consumer —
  rejected: every codegen site re-implements parsing; semantics
  drift; the type system stops helping.
- Enum-based `EnterEffect` (e.g. `EnterEffect::Transition { … }`,
  `EnterEffect::Custom { kind, payload }`) — considered for
  extensibility but rejected for v1: the dominant lifecycle-FSM case
  is `field := to_value`; complex domain operations (chess moves)
  carry their payload on `ActionInvocation` instead. Future
  extensions can be variants on a wider sum type later.
- Wider `EnterEffect` with arbitrary expression — rejected as
  premature; the typed minimal shape is more useful than an
  open-ended expression language.

**Consequences.**
- **Codegen applies transitions structurally**: `on_commit` body
  becomes `ctx.object_instance.set_field(&effect.field,
  &effect.to_value)` — no parsing.
- **#[non_exhaustive]** (per ADR-006) lets future `to_value` typing
  evolve without breaking SemVer.
- **Emitter restructured**: `ogar-emitter::emit_action_def` emits
  one `ogar:onEnter` link triple plus two leaf triples
  (`ogar:enterField`, `ogar:enterToValue`) — matches the TTL
  ontology, structural rather than free-form.
- **Cascade fix needed**: PR #13's first push missed the
  `ogar-emitter` callers using the old `Option<String>` shape;
  fixed in PR #15.

**References.**
- OGAR PR #13 (typed `EnterEffect` lands).
- OGAR PR #15 (placement fix + emitter cascade fix).
- `crates/ogar-vocab/src/lib.rs` (`pub struct EnterEffect`).
- `vocab/ogar.ttl` (`ogar:EnterEffect` owl:Class).

## ADR-006: `EnterEffect` carries `#[non_exhaustive]` per vocab forward-compat convention

**Status:** Pinned (2026-06-04).

**Context.** PR #15 (the EnterEffect placement fix) initially shipped
`EnterEffect` as an exhaustive public struct. Codex P2 caught that
`ogar-vocab`'s module convention is `#[non_exhaustive]` on every
public vocabulary struct (`ActionDef` and others follow this) for
forward compatibility — without it, downstream crates can construct
with struct literals and adding a future field is a SemVer break.

**Decision.** `EnterEffect` carries `#[non_exhaustive]` immediately
before `pub struct EnterEffect { … }`, after the derives, matching
`ActionDef`'s attribute ordering convention.

**Alternatives considered.** Leave `EnterEffect` exhaustive and bump
SemVer on future field additions. Rejected: every other public vocab
struct follows the `#[non_exhaustive]` convention; consistency
matters and SemVer-stability is the whole point.

**Consequences.**
- In-crate tests using struct literals still compile
  (`#[non_exhaustive]` only restricts external crate construction).
- Future tightening (e.g. `to_value: String → Value` enum) becomes
  non-breaking.
- External crates use the `EnterEffect::transition(field, to_value)`
  constructor instead of struct literals — the right API.

**References.** OGAR PR #15; vocab convention in `crates/ogar-vocab/src/lib.rs`.

## ADR-007: §3 signatures = canonical `ractor_actors::state_machine`

**Status:** Pinned (2026-06-04).

**Context.** OGAR-AST-CONTRACT.md §3's first draft used a sketchy
trait shape (`handle(state, event, ctx, hook) -> Transition`,
infallible `commit`). The runtime session built the real
`ractor_actors::state_machine` crate at
`feat/state-machine-actor @ 38a71a4` with different signatures:
`on_event` (pure, no hook param), `is_commit` predicate,
`timeout`/`on_timeout`, `Transition::{Goto, Stay, Postpone, Stop}`,
sync-fallible `CommitHook::on_commit`. Two sources of truth for the
binding's signature shape.

**Decision.** §3 of the contract aligns to the **actual crate at
`38a71a4`**. The crate is authoritative; the doc reflects reality,
not aspiration.

```rust
pub trait StateMachine: Send + Sync + 'static {
    type State:   Clone + PartialEq + Debug + Send + Sync + 'static;
    type Event:   Send + 'static;
    type Context: Send + 'static;    // opaque
    fn initial(&self) -> Self::State;
    fn on_event(&self, state: &Self::State, event: &Self::Event,
                ctx: &mut Self::Context) -> Transition<Self::State>;
    fn timeout(&self, _state: &Self::State) -> Option<Duration> { None }
    fn on_timeout(&self, _state: &Self::State, _ctx: &mut Self::Context)
        -> Transition<Self::State> { Transition::Stay }
    fn is_commit(&self, _state: &Self::State) -> bool { false }
}
pub enum Transition<S> { Goto(S), Stay, Postpone, Stop }
pub trait CommitHook<SM: StateMachine>: Send + Sync + 'static {
    fn on_commit(&self, from: &SM::State, to: &SM::State, ctx: &SM::Context)
        -> Result<(), ractor::ActorProcessingErr>;
}
```

**Alternatives considered.**
- Keep the sketchy contract draft and force the crate to align —
  rejected: the crate has 7/7 tests green including the load-bearing
  `postponed_event_is_replayed_after_transition`; reality wins.
- Maintain two compatible-but-different surfaces — rejected as
  inviting dual-source drift.

**Consequences.**
- **`on_commit` is sync + fallible** — honours the I-2 invariant
  (no tokio at the membrane) and lets `commit_event` errors fail
  the actor cleanly.
- **Hook wired at spawn**, not threaded per call — keeps `on_event`
  pure (no I/O surface in the guard).
- **`is_commit` predicate** — actor calls hook synchronously on
  entering any `is_commit` state; no separate `on_enter` method.
- **`Postpone` semantics formalized** — FIFO replay ahead of newer
  events, after the next state change. Single-actor; per-actor
  postpone queue is single-writer.

**References.** OGAR PR #11; ractor_actors `feat/state-machine-actor @ 38a71a4`; Rubicon Phase 1.

## ADR-008: `LanceMembrane::commit_event(row: CognitiveEventRow) -> u64`

**Status:** Pinned (2026-06-04). Code-complete in lance-graph PR #467.

**Context.** OGAR's `CommitHook::on_commit` (per ADR-007) needs a
sole-writer membrane to append the Lance version. The zero-dep
`ExternalMembrane::project(&self, bus: &ShaderBus, meta: MetaWord)
-> Self::Commit` is the existing surface, but it forces actions
through the cognitive-cycle `ShaderBus` shape — wrong abstraction
for the action-commit path.

**Decision.** Add a sibling on `LanceMembrane` (callcenter-side, not
zero-dep `ExternalMembrane`):

```rust
fn commit_event(&self, row: Self::Commit /* = CognitiveEventRow */) -> u64
```

Returns the new monotonic Lance version. Action commits skip the
`ShaderBus` cognitive-cycle shape; binding's `LanceCommitHook` builds
a `CognitiveEventRow` from the `ActionInvocation` context and appends.

**Alternatives considered.**
- Force the action commit through `ExternalMembrane::project` with
  a dummy `ShaderBus` — rejected: wrong shape; conflates two paths.
- Add the action-commit method to the zero-dep `ExternalMembrane`
  trait — rejected: zero-dep stays zero-dep; action concerns
  shouldn't leak into the cognitive trait.
- Open a separate trait for action-commit-only — possible but
  overengineering; sibling method on the callcenter-side
  `LanceMembrane` is the cleanest scope.

**Consequences.**
- **Two commit paths cleanly separated**: cognitive cycle via
  `project()`; action lifecycle via `commit_event()`. Same underlying
  Lance dataset; different semantic interfaces.
- **Gate 1 of the §7 status table closed**: `CommitHook::on_commit`
  has a typed call target.
- **Test confirmation**: `commit_event_ticks_version_and_returns_new`
  test green in lance-graph PR #467; verifies monotonic version
  increment.
- **Convergence with kv-lance future**: the runtime session's
  Phase-2 `RubiconWriter` has two backends (`LanceMembraneWriter` +
  `KvLanceWriter`) sharing the same commit contract — `commit_event`
  is the membrane surface; kv-lance gets its own equivalent.

**References.**
- lance-graph PR #467 (merged); OGAR `OGAR-AST-CONTRACT.md` §7 item 2.

## ADR-009: `lance-graph-planner::temporal` = deinterlace engine with two causal axes

**Status:** Pinned (2026-06-04). Shipped in lance-graph PR #468 (open).

**Context.** The substrate writes from multiple producers at
independent clocks: storage (Lance versions), schema (SurrealQL DDL
registration time), actor awareness (per-actor V_ref), cognition
(Markov ±5 CognitiveEventRow trajectory). Naïve "read current
state" produces *combed frames* — fields from different writers
torn against each other. Rubicon's guards must evaluate against a
*deinterlaced* state or multi-writer decisions fire on combed views.

**Decision.** `lance-graph-planner::temporal` is the deinterlace
engine with **two causal axes**, both type-visible from day one:

- **TIME-causal**: `QueryReference { server_id, ref_version,
  hlc_tick: Option<u64>, mode, rung }` carries HLC tick from day
  one; `classify(row_version, knowable_from, v_ref) ->
  Classification {Contemporary, Anachronistic, Spoiler, Unknowable}`.
- **DATA-causal**: `DependsClosure` trait — opaque seam the
  SPO `depends_on` / `reads_field` source plugs into. Rubicon's
  `KausalSpec::Depends` guard implements it.

`EpistemicMode {Strict, Aware, Retro}` controls what `classify`
admits per-rung.

**Alternatives considered.**
- TIME-only `classify(row_version, knowable_from, v_ref)` without
  HLC — rejected: forces a breaking change when cross-server lands,
  same trap as the original `emitted_at_millis: u64` (decision #4).
- DATA-axis built into `classify` as a hard dep on a specific SPO
  source — rejected: couples temporal to a specific producer;
  trait-based `DependsClosure` is the right abstraction (symmetric
  to `CommitHook` being opaque to the membrane).
- Both axes deferred and built later — rejected: type-level shape is
  the load-bearing decision; bodies can be trivial single-server
  initially.

**Consequences.**
- **HLC cluster-bus-ready signature**: cross-server policy lands as
  a body change, not a breaking interface change.
- **DependsClosure trivial impl today**: `NoDeps` no-op until SPO
  frontends emit real `depends_on` edges. Rubicon's `Depends` guard
  implements the trait when SPO data is available.
- **Per-row deinterlace decision**: every row read against a
  `QueryReference` gets a `Classification`; downstream consumers
  decide based on epistemic mode + rung.
- **§10.3 meet-point with OGAR**: `knowable_from` sourced by
  `ogar-adapter-surrealql` (per ADR-010), consumed here.

**References.**
- lance-graph PR #468 (open); OGAR `OPENPROJECT-TRANSCODING.md` §10.3;
  `OGAR-AST-CONTRACT.md` §6 amendment.

## ADR-010: `knowable_from` meet-point — single ownership, durable interface

**Status:** Pinned (2026-06-04). Cross-session authoritative pin.

**Context.** The SurrealQL frame in the four-frame deinterlace model
(per ADR-009) needs a stamp for "when did this class/field become
defined." Two plausible owners: OGAR (the schema producer) or the
runtime side (the storage layer). Without a pin, two consumers
could plausibly stamp it, producing dual-source drift.

**Decision.** **Single ownership**:
- `ogar-adapter-surrealql::register_class_knowable_from(class,
  lance_dataset) -> u64` is the producer side (stamps at DDL
  registration time).
- `lance-graph-planner::temporal::classify(row_version,
  knowable_from, v_ref)` is the consumer side.
- **Nowhere else** in the substrate owns either side of this seam.

Pinned in `OGAR/docs/OPENPROJECT-TRANSCODING.md §10.3` as the
authoritative OGAR-side source. To be mirrored to bardioc's
`CROSS_SESSION_COORDINATION.md` (runtime-session-owned) for full
cross-session durability.

**Alternatives considered.**
- Two owners coordinating via convention — rejected as dual-source.
- A separate ownership record (e.g. a `kv-lance` row per class) —
  possible but adds a third party; this seam works without it.

**Consequences.**
- **Cross-session clarity**: any future session in OGAR or runtime
  or any other repo finds the single owner via §10.3.
- **`register_class_knowable_from` signature reserved**: today a
  `todo!()` stub in `ogar-adapter-surrealql` (#18); when wired
  (gated by `lance-bind` Sprint-5b), the seam is type-locked.
- **Roundtrip**: `parse_surrealql_ddl → register_class_knowable_from
  → temporal::classify` is the closed loop; producer + consumer
  versions of `knowable_from` are bit-identical.

**References.**
- OGAR PR #16 §10.3; OGAR PR #18 (`register_class_knowable_from` stub);
  lance-graph PR #468 (`classify` shipped); bardioc
  `CROSS_SESSION_COORDINATION.md` (mirror pending).

## ADR-011: Two-arm naming pattern (narrow SPO + wide OGAR) for producers

**Status:** Pinned (2026-06-04).

**Context.** `AdaWorldAPI/ruff` has `ruff_ruby_spo` (Ruby/Rails
scaffold) and `ruff_python_dto_check` (Python, fully wired) emitting
narrow SPO triples via `ruff_spo_triplet::ModelGraph`. OGAR's wide
arm needs `ogar-from-ruby` and `ogar-from-elixir` to emit `Class` +
`ActionDef`. Without naming hygiene, these look like duplication
when they're actually complementary.

**Decision.** Producers come as **named pairs per domain**:

| Domain | Narrow SPO (scaffold/wired) | Wide OGAR (planned) |
|---|---|---|
| Ruby AR | `ruff_ruby_spo` | `ogar-from-ruby` |
| Python | `ruff_python_dto_check` | `ogar-from-ruff` / `ogar-python` |
| Elixir | `ruff_elixir_spo` (future) | `ogar-from-elixir` |

One AST parse fills both; narrow SPO answers "what depends on what"
(data-dependency DAG, feeds Rubicon's `KausalSpec::Depends`); wide
OGAR answers "when does this fire, what guards, what commits"
(lifecycle FSM).

**Alternatives considered.** Merge both arms into a single producer
emitting both shapes. Rejected: SPO consumers and OGAR consumers
have different downstream paths; tight coupling at the producer is
premature; the AST parse is the shared piece, not the emitter.

**Consequences.** Producer collisions prevented; clear ownership; both
arms can evolve independently.

**References.** OGAR PR #16 §10.1.

## ADR-012: nexgen `op-surreal-ast` is a special case of OGAR `Class → catalog::TableDefinition`

**Status:** Pinned (2026-06-04). No collision; convergence path defined.

**Context.** `AdaWorldAPI/openproject-nexgen-rs` ships
`op-surreal-ast` (Sprint C16a — mirror of `surrealdb-core::catalog`
layout) + `op-codegen-projection` (Sprint C15 — DDL renderer via
`op-surreal-ast`). Could be read as "we've already done SurrealQL
emission for OP" colliding with OGAR's planned `ogar-adapter-
surrealql`.

**Decision.** nexgen's path is a **special case of the general OGAR
projection**: `Class → catalog::TableDefinition` via the Sprint C16b
`new_for_ddl().with_*()` builders. C16c plans `From<op_surreal_ast::*>
for catalog::*` impls; once landed, `op-surreal-ast` either drops the
mirror or keeps as a fast in-repo path. Generalization:
`From<ogar_vocab::Class> for catalog::TableDefinition` is the same
shape for OGAR's wide IR.

**Alternatives considered.** Have nexgen and OGAR each ship parallel
SurrealQL emitters — rejected as duplication. Have OGAR depend on
nexgen — rejected, scoping inverted (nexgen is OP-specific; OGAR is
domain-agnostic). Have nexgen depend on OGAR — possible eventually
but adds heavy dep today.

**Consequences.** They meet at `surrealdb-core::catalog`, not at
schema-source level. `op-codegen-projection` (nexgen) +
`ogar-adapter-surrealql::emit_surrealql_ddl` (OGAR) coexist as fast
in-repo path + general path; both call the same C16b builders.

**References.** OGAR PR #16 §10.2; nexgen sprints C15/C16a/C16c;
surrealdb Sprint C16b `.claude/op-codegen-bridge/README.md`.

## ADR-013: `has_paper_trail` is a duplicate of Lance versions — substrate consolidation

**Status:** Pinned (2026-06-04). Observation, not enforcement.

**Context.** OpenProject's `WorkPackage` uses `has_paper_trail` (per
the `paper_trail` gem) to record every change as a row in a separate
`versions` table. The substrate's Lance dataset is append-only and
versioned by construction — same property, different table.

**Decision.** The `versions` table is *subsumed* by the Lance version
log. Faithful binding eliminates the redundant AR `versions` table;
`PaperTrail::Version` row becomes a derived projection of the Lance
version log, not a separate write.

**Alternatives considered.** Keep both — rejected as duplicate
storage + double-write hazard. Drop `has_paper_trail` from OP
silently — rejected: OP code expects it; the binding must preserve
the query interface even when removing the underlying table.

**Consequences.**
- **One less table on disk** per migration; same query power.
- **Audit equivalence**: every Lance commit row carries the same
  info (subject, predicate, before/after attrs, current_user via
  `lokal`, trace_id, idempotency_key) that PaperTrail captures.
- **§14 oracle implication**: when graduating an OP action that
  triggers `has_paper_trail`, the §14 verdict must compare
  "PaperTrail::Version row on Rails side" against "Lance version row
  on substrate side, projected to PaperTrail shape" — same data,
  different storage path.

**References.** OGAR `OPENPROJECT-TRANSCODING.md` §4 + §8.

## ADR-014: Data-driven FSMs need database-hydrator pattern

**Status:** Pinned (2026-06-04). Generalizes the TTL hydrator pattern from `lance-graph-ontology`.

**Context.** OpenProject's transitions aren't hard-coded in Ruby —
they're rows in the `workflows` table. A producer that walks only
the *code* misses them. Same pattern for `Roles`, `Permissions`,
`custom_actions`, etc. The data IS the FSM.

**Decision.** OGAR producers consuming Rails apps with data-driven
FSMs need a **database-hydrator step** alongside the AST walk:

```rust
pub fn emit_workflow_action_defs(db_url: &str) -> Result<Vec<ActionDef>>;
```

Reads the `workflows` table → emits one `ActionDef` per
`(old_status, new_status)` row with `kausal=StateGuard{status_id,
[old]}`, `on_enter=EnterEffect{status_id, new}`, role-gate decorator.

Generalizes the TTL hydrator pattern (`lance-graph-ontology`'s
SKOS/PROV-O/FIBO hydrators) to AR seed data — same pattern, different
source format. A reusable `ogar-hydrator-postgres` crate is the
natural extraction.

**Alternatives considered.** Mandate that Rails apps move
data-driven FSMs into code — rejected as not-our-call; apps make
their own design choices. Skip data-driven FSMs in OGAR's coverage
— rejected: misses critical OP behavior.

**Consequences.** OP-graduation depends on this; HIRO-graduation
likely too (config tables exist there); applies to any Rails AR app
with admin-configurable workflows.

**References.** OGAR `OPENPROJECT-TRANSCODING.md` §7.

## ADR-015: `Language` enum is the extension point for source-AST tags

**Status:** Pinned (2026-06-04). Established by PR #10's `Language::Elixir`.

**Context.** New source languages (Rust source via shakmaty, future
Go/Swift/etc.) need a way to be tagged on `Class.language`. The
`Language` enum lives in `ogar-vocab` (a core crate); external
producers can't add variants without modifying core.

**Decision.** `Language` is the **established extension point** —
adding a variant is a one-line core PR (precedent: `Language::Elixir`
in #10). External producers ship with `Language::Unknown` initially
and earn a typed variant via follow-up PR. **Not a structural IR
change** in the "zero core changes" sense — `Language` is open by
design.

**Alternatives considered.** Make `Language` an open trait — rejected
as type-system overkill; the variant set is small and well-defined.
Allow `Language::Custom(String)` — rejected: dilutes the type system;
tags would proliferate without coordination.

**Consequences.**
- **Chess producer** ships with `Language::Unknown` (per PR #12 §1
  amendment); a `Language::Rust` variant is a one-line follow-up
  when needed.
- **New producers** follow the pattern: ship working with `Unknown`,
  earn the typed variant later.
- **Convention preserved**: structural IR (`Class`, `Association`,
  etc.) stays closed to producer modification; `Language` is the open
  seam.

**References.** OGAR PR #10 (`Language::Elixir`); PR #12 §1; PR #17 (Elixir scaffold uses it).

## ADR-016: SurrealQL DDL AST is **not** the universal IR

**Status:** Pinned (2026-06-04). Full analysis in `docs/SURREAL-AST-AS-ADAPTER.md`.

**Context.** Open architectural question: could `surrealdb-ast::Library`
/ `DefineTable` / `DefineField` replace OGAR's `Class` / `ActionDef`
as the universal IR for both DDL and non-DDL sources? Beauty: one
fewer IR layer, mature parsers/emitters, automatic convergence with
nexgen.

**Decision.** **No** — split answer per arm:
- **Structural arm**: canonicalize via `From<Class> for
  catalog::TableDefinition` (Sprint C16b alignment). `Class` stays
  as wide source-side IR; `catalog::TableDefinition` is the
  schema-projection target.
- **Behavioral arm**: stays OGAR-owned (`ActionDef`,
  `ActionInvocation`, the four §6 Rubicon-statem carriers). SurrealQL
  DDL has no vocabulary for SPO+TeKaMoLo, lifecycle FSM, `KausalSpec`,
  `EnterEffect`, `GuardFailurePolicy`, `state_timeout_millis`.

**Alternatives considered.**
- Unify on surreal-AST entirely — rejected: behavioral arm has no DDL
  vocabulary; encoding tricks (sentinel comments in `DEFINE EVENT
  WHEN … THEN …`) are negative-beauty workarounds that don't survive
  maintenance.
- Reject surreal-AST entirely — rejected: structural projection
  alignment is genuinely useful; nexgen's `op-surreal-ast` is the
  precedent.

**Consequences.**
- **`Class` stays wide IR** — keeps producer metadata (Rails
  `dependent:`, `inverse_of:`, `polymorphic:`, `mixins:`,
  `decorators:`, `acts_as_*`, `callbacks:`, `scopes:`) that the
  schema projection loses.
- **`From<Class> for catalog::TableDefinition`** is the durable
  bridge; both `op-codegen-projection` and `ogar-adapter-surrealql`
  consume it.
- **§3 verdict steady-state**; migration-period polyglotism (ADR-018)
  is orthogonal.

**References.** OGAR PR #19 (full doc); OGAR PR #18 (`ogar-adapter-surrealql`).

## ADR-017: `surrealdb-parser` cross-repo dep deferred behind feature flag pending rust-version bump

**Status:** Pinned (2026-06-04). Unblocks when OGAR rust-version bumps.

**Context.** `ogar-adapter-surrealql`'s `parse_surrealql_ddl(input)
-> Result<Vec<Class>>` needs `surrealdb-ast` + `surrealdb-parser`.
The AdaWorldAPI/surrealdb fork pins `rust-version = "1.95"`; OGAR
workspace pins `"1.85"`. A direct git dep won't build in OGAR's CI.

**Decision.** `parse_surrealql_ddl` ships as `todo!()` stub; the
`surrealdb-parser` feature flag in `Cargo.toml` documents the wiring
intent. Git dep activates when OGAR's `rust-version` bumps to
`1.95+`. `emit_surrealql_ddl` is fully implemented in the interim
(hand-written formatter aligned with the C16b builders).

**Alternatives considered.**
- Bump OGAR's rust-version immediately — rejected as scope creep
  beyond this adapter PR's intent.
- Implement a minimal SurrealQL parser in OGAR — rejected as
  reinventing battle-tested upstream work.
- Ship without the `parse` direction at all — rejected: half of the
  bidirectional bridge missing dilutes the surface.

**Consequences.**
- **`emit` works today**: hand-written formatter; 12 tests lock the
  shape; can be refactored to call `catalog::TableDefinition::new_for_ddl`
  later (signature stays).
- **Roundtrip property deferred**: `parse(emit(parse(x))) == parse(x)`
  not provable until the dep is wired.
- **OGAR rust-version bump becomes a tracked work item**: required
  before §10.3 meet-point can be fully wired (which needs
  `register_class_knowable_from` to actually persist).

**References.** OGAR PR #18.

## ADR-018: Kanban-as-polyglot-dispatcher during migration

**Status:** Pinned (2026-06-04). Full architecture in `docs/SUBSTRATE-ENDGAME.md` Room 2.

**Context.** Steady-state target architecture is every actor as a
native Rust ractor handler over OGAR's IR. But during migration from
existing Rails-AR / Elixir-OTP stacks, requiring all actors to be
translated to native Rust before the substrate goes live is
unacceptable lead time. The Kanban dispatcher's contract (per
`SOA-IMPLEMENTATION.md §5.2`) is shaped narrowly enough that
multiple executable forms can satisfy it.

**Decision.** During migration, the Kanban admits **six work-item
form variants** as alternatives to native Rust handlers:
- BEAM-compiled Elixir function call (via Erlang Port / NIF).
- Tiny Elixir-AST interpreter (limited commandlets subset).
- HTTP RPC to Rails sidecar (lowest friction; OP's natural form).
- Embedded CRuby via FFI (in-process Ruby).
- Static reflection dump as producer-input (cheapest beauty win — no
  embedding, sharpens `ogar-from-ruby` extraction).
- (Plus native Rust ractor handler — the steady-state target.)

Per-actor graduation via the §14 oracle: when a tape of (input_state,
input_event, input_ctx) replays against the native candidate and the
output matches (provenance-normalized) for N consecutive runs, swap
the work-item form. Old form parked for rollback.

**Alternatives considered.**
- Flag-day migration (rewrite all actors before substrate goes live)
  — rejected as unacceptable lead time.
- Lift-only-storage migration (substrate hosts only the Lance commit
  layer; behavior stays in Rails/Elixir entirely) — rejected: misses
  the lifecycle/Rubicon/temporal benefits the substrate provides.
- Stable-API-on-Rails-side bridge (Rails app exposes a "substrate
  driver" interface that the Kanban talks to) — converges with the
  HTTP sidecar variant.

**Consequences.**
- **Substrate goes live early**: Lance commits, deinterlace, temporal
  classify, RubiconHook all work with migration-form actors before
  any native translation lands.
- **Adoption friction lowest at the per-actor commitment ceiling**:
  bring up substrate alongside existing app, route a single actor's
  work-items through HTTP sidecar, prove §14 oracle, graduate, repeat.
- **Two-runtime maintenance cost during the migration window**:
  honest list in `SUBSTRATE-ENDGAME.md` §7.2; mitigated by
  prioritizing per-class graduation by usage frequency.
- **Kanban contract is the stable interface**, work-item form
  migrates underneath — substrate doesn't churn during migration.

**References.** OGAR PR #19 §6; OGAR PR #20 (`SUBSTRATE-ENDGAME.md` Room 2).

## ADR-019: OP-as-operator-pane is the substrate's self-hosting destination

**Status:** Pinned (2026-06-04). Full architecture in `docs/SUBSTRATE-ENDGAME.md` Room 3.

**Context.** Once OP graduates onto substrate-b per ADR-018, what's
the natural endpoint? OP's existing feature set is *literally* the
substrate's operator vocabulary (WorkPackage = queued action, Status
+ Workflow = lifecycle FSM, kanban board view = the live operator
UI, Members/Roles = RBAC). Building a separate operator pane would
re-implement what OP already provides.

**Decision.** OpenProject **is** the substrate's operator pane.
Self-hosting recursion: the substrate's first production user is
itself. OP's UI surface (Hotwire / Turbo / ViewComponent kanban
view, admin UI for Workflow editing, notifications, journals) is the
operator's window onto the substrate that hosts OP.

**Alternatives considered.**
- Build a dedicated operator pane from scratch — rejected: 20+ years
  of OP UX polish is the wrong thing to re-invent.
- Use Grafana alone as the operator pane — rejected: too thin for
  operator workflows (operators want to trigger transitions, edit
  workflows, see journals — not just metrics).
- Adopt a different OSS PM tool (Taiga, Wekan, Plane) — possible
  but OP has the most aligned feature set + the strongest community
  + the right license.

**Consequences.**
- **20 years of UX polish for free**: kanban drag-and-drop, custom
  fields, multi-tenant projects, notifications, RBAC.
- **Operator-edited Workflow table → live Rubicon machines** (per
  ADR-014's hydrator pattern): substrate behavior is operator-
  configurable via OP's existing admin UI.
- **Existence proof at scale**: substrate validates by surviving its
  own bootstrap; the universality claim has a self-hosting receipt.
- **Community engagement**: OP maintainers as the first audience for
  the substrate (per Room 5.4 + 5.5 in endgame doc).

**References.** OGAR PR #20 (`SUBSTRATE-ENDGAME.md` Room 3).

## ADR-020: SDK endgame is deeper than Foundry going OSS

**Status:** Pinned (2026-06-04). Three structural differentiators.

**Context.** Comparing substrate-b's SDK shape against the
hypothetical scenario of Palantir Foundry going open-source.
Foundry would still be Palantir-shaped: a vertical enterprise
platform someone else built and you adapt to.

**Decision.** Substrate-b is structurally different in **three
ways**:

1. **Migration scaffold as bootstrap** (ADR-018): per-actor
   commitment ceiling vs platform-wide commitment; subscribe-and-
   graduate vs lift-onto-platform engagement.
2. **OP-as-production-operator-pane** (ADR-019): reference UI is a
   real OSS project with thousands of deployments + 20-year UX
   pedigree; the substrate's first user is itself; contributions to
   OP benefit every substrate adopter (network-effect-shaped).
3. **SPO + OGAR + temporal + Rubicon are substrate primitives, not
   application features**: substrate-b is the *layer Foundry would
   be built on if Foundry restarted today*; OSS at the substrate
   layer is one level lower than OSS at the application layer.

**Alternatives considered.** Frame substrate-b as "Foundry-OSS-
shaped" — rejected: undersells the architectural depth.
Frame as a research project — rejected: production-grade primitives
are shipping (PR #467, #468, ractor_actors); not research.

**Consequences.**
- **Positioning**: substrate, not application. Capabilities first,
  comparison earned by being technically defensible.
- **Adoption model**: per-actor graduation via §14 oracle = lower
  commitment ceiling = wider potential adoption.
- **OP as the unicorn vision**: makes OP's existing maintainer
  community the first engagement audience.
- **Long path**: 12-24 months from Room 1 to demonstrable Room 5
  per `SUBSTRATE-ENDGAME.md §6.7`.

**References.** OGAR PR #20 (`SUBSTRATE-ENDGAME.md` §5).

## ADR-021: Meta-hygiene — always grep peer crates before copying manifest patterns

**Status:** Pinned (2026-06-04). Lesson from the `[lints] workspace = true` cascade.

**Context.** PR #17 (`ogar-from-elixir` scaffold) imported `[lints]
workspace = true` from `ruff_ruby_spo`'s Cargo.toml. OGAR's root
Cargo.toml doesn't define `[workspace.lints]`, so the entire
workspace failed manifest parsing. PR #18 (`ogar-adapter-surrealql`)
inherited the same copy-from-ruff pattern and the same bug. Codex
flagged both; fixed in #17 (direct fix) + #18 (pre-emptive fix).

**Decision.** Process rule: **before copying any manifest pattern
from a sibling repo, grep existing peer crates in the target
workspace to verify the pattern is supported.** Concretely for OGAR:
no Cargo.toml uses `[lints]`; therefore new crates don't either
unless the root workspace adds `[workspace.lints]` first.

**Alternatives considered.** Add `[workspace.lints]` to OGAR's root
to make the imported pattern work — would have been fine but adds
scope beyond the scaffold PR's intent; deferred as a separate
hygiene PR if/when desired.

**Consequences.**
- **Hygiene rule for future scaffolds** (and any cross-repo pattern
  borrowing).
- **CI gap also identified**: workspace `cargo check --workspace`
  should be a required check on `crates/**` PRs; #13's
  `EnterEffect` placement bug + #17/#18's `[lints]` bug would both
  have been caught at CI.
- **Architectural decision records (this doc) explicitly capture
  meta-lessons**, not just feature decisions — process learnings
  are decisions too.

**References.** OGAR PR #15 (placement fix + emitter cascade); PR
#17 (Codex P1 fix); PR #18 (pre-emptive fix).

## Cross-references

### Companion docs (this repo)

- `docs/SUBSTRATE-ENDGAME.md` — the forward-looking five-rooms architecture (companion to this doc; read together).
- `docs/OGAR-AST-CONTRACT.md` — the typed contract surface (ADR-001, ADR-002, ADR-003 codified here).
- `docs/SURREAL-AST-AS-ADAPTER.md` — full analysis of ADR-016; §6 covers ADR-018's migration scaffold counterpoint.
- `docs/OPENPROJECT-TRANSCODING.md` — Rails AR transcoding (ADR-011 §10.1 two-arm pattern; ADR-012 §10.2 nexgen convergence; ADR-010 §10.3 knowable_from meet-point; ADR-013 §4 paper_trail consolidation; ADR-014 §7 hydrator pattern).
- `docs/ELIXIR-HIRO-PREFETCH.md` — OLD HIRO/Bardioc debt ledger (ADR-018 migration scaffold's primary Elixir-side target).
- `docs/CHESS-TRANSCODING.md` — closed-formal calibration; ADR-015 `Language::Unknown` precedent.
- `vocab/ogar.ttl` — vocab terms shipped per ADR-004 (three Rubicon-statem terms) + ADR-005 (`EnterEffect` typed).

### Cross-repo references

| Repo | PRs / files | Decisions touched |
|---|---|---|
| `AdaWorldAPI/OGAR` | PRs #9, #10, #11, #12, #13, #14, #15, #16, #17, #18, #19, #20 | ADR-001 to ADR-021 (this doc's full set) |
| `AdaWorldAPI/lance-graph` | PR #467 (`commit_event` sibling) | ADR-008 |
| `AdaWorldAPI/lance-graph` | PR #468 (`temporal::classify` + `DependsClosure`) | ADR-009, ADR-010 |
| `AdaWorldAPI/ractor_actors` | `feat/state-machine-actor` @ `38a71a4` | ADR-007 |
| `AdaWorldAPI/openproject-nexgen-rs` | C9, C15, C16a, C16c sprints | ADR-011, ADR-012 |
| `AdaWorldAPI/surrealdb` | Sprint C16b op-codegen-bridge | ADR-012, ADR-016, ADR-017 |
| `AdaWorldAPI/ruff` | `ruff_spo_triplet`, `ruff_python_dto_check`, `ruff_ruby_spo` | ADR-011 |
| `AdaWorldAPI/bardioc` | `CROSS_SESSION_COORDINATION.md` (runtime-session-owned mirror) | ADR-010 (cross-session pin authority) |

### Decisions still in flight (not pinned here)

These were considered but explicitly deferred — flagging so future
sessions don't re-litigate or assume closure:

- **Cross-server HLC merge policy** (ADR-009 follow-up). Type-level
  HLC awareness shipped; cross-server merge policy is body work,
  deferred until peer-Raft / cluster bus lands.
- **`lance-bind` boundary impl** (ADR-010 follow-up). Sprint-5b
  blocker; needs protoc + cross-repo build sorted.
- **OGAR rust-version bump 1.85 → 1.95+** (ADR-017 unblock).
  Scope-creep concern from #18; revisit when other crates need it
  too.
- **`Rubicon` durable home** (Rubicon Phase 1 ↔ Phase 2 transition).
  Currently in runtime session's scratch; bardioc-excluded-standalone-
  crate is the likely destination but not pinned.
- **`KvLanceWriter` impl** (Rubicon Phase 2). Per runtime session's
  earlier message; pairs with `LanceMembraneWriter` over the same
  Lance 7.0.0 commit contract.
- **`Workflow`-as-live-Rubicon-machine dynamic regen** (Room 3 dep
  per `SUBSTRATE-ENDGAME.md §6.3` row 3.3). Requires `Rubicon-from-
  OGAR` codegen to support dynamic regeneration; not yet specified.

## Doc lifecycle

- **Format**: ADR convention (Context / Decision / Alternatives /
  Consequences / References). Append-only.
- **Adding a new ADR**: assign next sequence number; add row to the
  Index at the top; full entry in chronological order in the body;
  PRs / commits that touch the decision cite the ADR-NNN.
- **Superseding an ADR**: don't delete or modify the original; add
  a new ADR-MMM with "Supersedes ADR-NNN" + the original gets a
  "**Status: Superseded by ADR-MMM**" line. Reasoning chain stays
  visible.
- **Cross-repo cadence**: when a decision spans repos, the
  primary-owner repo carries the ADR; other repos reference it.
  OGAR is the primary owner for substrate-architecture decisions;
  bardioc's `CROSS_SESSION_COORDINATION.md` is the runtime-session-
  owned mirror.
- **Authority**: this doc is the authoritative OGAR-side record.
  Future sessions in OGAR cite ADR-NNN when implementing or
  revisiting any decision listed here.

