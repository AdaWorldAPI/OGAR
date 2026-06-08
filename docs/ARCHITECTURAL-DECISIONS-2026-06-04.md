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
| ADR-001 | `State = ActionState` (lifecycle), not domain state, for Rubicon binding | **Pinned + Implemented** | OGAR PR #9 (contract §3); confirmed via Rubicon Phase 1 |
| ADR-002 | `ActionDef` and `ActionInvocation` stay split — never collapse | **Pinned + Implemented** | OGAR PR #13 (carve-out 5) |
| ADR-003 | `Class` field set 1:1 with `class_record_batch_schema` (full structural fidelity) | **Pinned** | OGAR PR #13 (Codex P2 #2) |
| ADR-004 | Three §6 Rubicon-statem vocab terms as TeKaMoLo sub-properties (`onEnter` / `guardFailurePolicy` / `StateTimeout`) | **Pinned** | OGAR PR #10 |
| ADR-005 | `EnterEffect { field, to_value }` is typed (not free-form `xsd:string`) | **Pinned + Implemented** | OGAR PR #13 |
| ADR-006 | `EnterEffect` is `#[non_exhaustive]` per vocab forward-compat convention | **Pinned** | OGAR PR #15 (Codex P2 follow-up) |
| ADR-007 | §3 signatures = canonical `ractor_actors::state_machine` (`on_event` / `is_commit` / `timeout` / `on_timeout` / `Transition::{Goto,Stay,Postpone,Stop}` / sync-fallible `on_commit`) | **Pinned + Implemented** | OGAR PR #11; ractor_actors `feat/state-machine-actor` @ `38a71a4` |
| ADR-008 | `LanceMembrane::commit_event(row) -> u64` as the `CommitHook::on_commit` target (sibling to `ExternalMembrane::project`) | **Pinned + Implemented** | lance-graph PR #467 (merged); OGAR PR #11 §7 item 2 |
| ADR-009 | `lance-graph-planner::temporal` = the deinterlace engine with two causal axes (TIME via HLC + DATA via `DependsClosure`) | **Pinned + Implemented** | lance-graph PR #468; OGAR PR #16 §10.3 |
| ADR-010 | `knowable_from` meet-point single ownership: sourced by `ogar-adapter-surrealql`, consumed by `temporal::classify`, nowhere else | **Pinned (half-implemented)** | OGAR PR #16 §10.3; lance-graph PR #468 |
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
| ADR-022 | **The Firewall** — absolute inner/outer boundary; no serialization in hot path; inner = compile-time HHTL; outer = contract-trait pluggable | **Pinned** | OGAR (this PR); `docs/THE-FIREWALL.md` |
| ADR-023 | **IR-as-wire-truth** — the source-language AST is *input dialect*; the canonical `Class`/`Attribute`/`Association`/`EnumDecl`/`ActionDef` IR is *wire truth*. Adapters lift dialects into IR; the IR routes everything (registry key, actor mailbox, Lance version, audit-log dimension) | **Pinned** | OGAR (this PR); `crates/ogar-vocab/`; `bardioc/substrate-b-shadow::EdgeDecoder<E>` (PR #19) |
| ADR-024 | **Palette256 + HHTL codec** — the substrate's universal compression primitive. HHTL prefix establishes a frame; within the frame, values cluster; clustered values quantize to 256-index palette + const-table lookup. Names an existing primitive (Binary16K perms + bgz-tensor attention + arm-discovery aerial codebook, ρ=0.9973 vs cosine) rather than proposing one | **Pinned** | OGAR (this PR); `MedCare-rs/crates/medcare-analytics/src/{graph_contract.rs,column_mask_bridge.rs}`; `bgz-tensor/examples/compare_stacked_vs_i16.rs`; `lance-graph-arm-discovery` |
| ADR-025 | **Probe-free hot path** — *address arithmetic (helix + HHTL) gives geometry; Jirak certificate (jc) gives error; together they give level — and the level is the only thing the hot path picks.* Closed-form LOD/level selection replaces empirical probing; the hot path takes zero data-dependent branches. Completes the substrate-doctrine canon (022 boundary / 023 IR / 024 codec / 025 selection) | **Pinned** | OGAR (this PR); runtime: `crates/helix`, `crates/jc` (`jc::weyl` + `jc::jirak`), `crates/cesium/src/{sse,implicit_tiling}.rs` |

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

## ADR-022: The Firewall — absolute inner/outer boundary, no serialization in hot path

**Status:** Pinned (2026-06-05). **Absolute invariant** — full treatment in `docs/THE-FIREWALL.md`.

**Context.** The substrate has a hot path (Rubicon dispatch, identity
resolution, intra-process actor messaging) and an outer boundary
(external storage, external schema, cross-process/server, wire formats).
Without an explicit, named, absolute boundary, the hot path erodes one
"small" serialization / runtime-lookup / heavy-dep at a time. The
operator declared the boundary **absolute** and named it **The
Firewall**, with the enforcement rule **"no serialization in the hot
path."**

**Decision.** The Firewall is the absolute boundary between:
- **Inner** = compile-time HHTL. Identity/ontology resolution is
  resolved at compile time (const/typestate generated at build time via
  jinja/xml/whatever templating), exactly as OGIT is compile-time-
  checked. The hot path does **no serialization, no serde, no
  trait-object dispatch for known classes, no tokio (I-2), Arrow-scalar
  values (BBB), `Arc` zero-copy**.
- **Outer** = contract-trait pluggable backends. `ExternalMembrane`
  (`lance-graph-contract`) + `KnowableFromStore` (`ogar-knowable-from`)
  are the firewall interfaces; Redis / SeaORM / Postgres /
  schema-from-whatever implement them at `lance-graph-callcenter` +
  `lance-graph-contract`. Boundary tax (serialization, crypto, network,
  external reads) is acceptable here — **paid once per crossing**, then
  cached (`LazyLock` fallback for unknown schema).

The litmus ("crypto on post stamps"): a cost paid once per firewall
crossing is fine; the same cost per inner operation is forbidden.

**Alternatives considered.**
- Runtime-trait inner architecture (pluggable all the way down) —
  rejected: puts trait-object dispatch + potential serialization on the
  hot path. Pluggability is a boundary feature, not an inner one.
- "Serialization is fine if it's fast" — rejected: the operator
  declared the firewall absolute; "fast enough" is how hot paths erode.
- No named principle (rely on convention) — rejected: the boundary is
  too load-bearing to leave implicit.

**Consequences.**
- **Compile-time HHTL codegen is a required build-time step** for OGAR
  (net-new — OGAR has no `build.rs` today). Lowers `Class`/`ActionDef`
  IR into compile-time-resolvable structures.
- **`serde` stays feature-gated** across OGAR crates — boundary use
  only, never a hot-path dependency.
- **`KnowableFromStore` (ADR-010) confirmed outer-boundary** — its
  Lance-row serialization is correct *because it's the firewall*.
  Same for `LanceMembrane::commit_event` (ADR-008).
- **`SOA-IMPLEMENTATION §5.3` clarified**: intra-process actor
  messaging is zero-copy `Arc<RecordBatch>` (no serialization); the
  "RecordBatch IPC" framing applies only cross-instance (a firewall
  crossing).
- **The Firewall is the umbrella** that I-2 (no tokio on hot loop) +
  BBB (Arrow-scalar on hot loop) are facets of; "no serialization" is
  the new facet.
- **Enforcement aspiration**: a future CI lint denying `serde` reachable
  from hot-path entry points would make it mechanical. Noted, not built.
- **Change policy**: weakening the Firewall requires an explicit
  superseding ADR with measured justification, not incremental erosion.

**References.**
- `docs/THE-FIREWALL.md` (full treatment).
- `docs/SUBSTRATE-ENDGAME.md` §5 (the SDK seam the outer boundary enables).
- `docs/SOA-IMPLEMENTATION.md` §5.3 (the RecordBatch-IPC clarification).
- ADR-008 (`commit_event` — outer firewall write), ADR-010
  (`knowable_from` — outer firewall seam), `lance-graph-contract::ExternalMembrane`.
- Precedent: MedCare-rs (`Membrane` + `LazyLock`), Woa-rs (SeaORM backend).

**Reception receipts — cross-session convergence (2026-06-05).** The
runtime session — working independently on the BindSpace dissolution
and the qualia codebook — arrived at the same inner/outer split this
ADR pins, without coordination on the surface itself. Three independent
landings now triangulate The Firewall:

- **OGAR-side pin** (this ADR + `docs/THE-FIREWALL.md`): the boundary is
  absolute; inner = compile-time HHTL, outer = contract-trait pluggable;
  litmus = "crypto on post stamps".
- **Runtime-side implementation** (bardioc PR #17, Rubicon Phases 1-5,
  33 tests): `LanceMembraneWriter` carries the outer write surface, with
  the hot path (Rubicon state-machine dispatch) sitting inside the
  firewall.
- **Runtime-side architectural handover** (bardioc PR #18 +
  lance-graph PR #470, `BINDSPACE_DISSOLUTION_HANDOVER.md`): the
  runtime session independently named the same split — *"Rubicon's
  `LanceMembrane::commit_event` keyed on `inv.object_instance` becomes
  a trie append"* (inner) over the outer `commit_event` write — and
  cited `HEALTHCARE-TRANSCODING.md §3` (the row-level `_effectiveReaders`
  mask) as the model for task-scoped qualia activation via `QualiaScope`.

Triangulation matters because architectural axioms are only load-
bearing if multiple sessions can re-derive them. Three sessions
(OGAR docs, Rubicon implementation, runtime handover) now share the
inner = trie-append / outer = `commit_event` boundary as common ground.
Weakening The Firewall now requires re-coordinating across all three
surfaces — exactly the friction this ADR's change-policy clause was
designed to produce.

**Reception references.**
- bardioc PR #17 (Rubicon Phases 1-5; the Implementation Receipts
  section below already cites its commits).
- bardioc PR #18 (`BINDSPACE_DISSOLUTION_HANDOVER.md` — the canonical
  architectural-delta doc).
- lance-graph PR #470 (`.claude/handovers/2026-06-05-0445-bardioc-to-
  lance-graph-bindspace-arch-delta.md` — the lance-graph-side pointer).

## ADR-023: IR-as-wire-truth — Class is the wire format, not the source AST

**Status:** Pinned (2026-06-05). Companion to ADR-022 (The Firewall);
captures the framing principle the firewall's inner side has been
operating under.

**Context.** Cross-session conversation surfaced the question
*"what's the wire format between source-language frontends and the
substrate?"* — raised in the context of Elixir ASTs, ClickHouse DDL,
SurrealQL DDL, FIBO/FMA TTL, and the planned `ch`/`ecto_ch` shadow
extraction. The naive answer ("forward the source AST as-is") is
wrong; the firewall's inner discipline already implies the right
answer, but it hadn't been named explicitly.

**Decision.** The canonical wire format is the OGAR IR — `Class`,
`Attribute`, `Association`, `EnumDecl`, `ActionDef`, `KausalSpec`,
`Identity` (the `NiblePath` prefix-radix). Source-language ASTs
(Elixir quoted form, SurrealQL DDL AST, Ruby AR macro tree, Odoo
Python `models.Model` shape, ClickHouse CREATE TABLE, OWL TTL
triples) are *input dialects* — each lifted into the canonical IR
by a dedicated **adapter crate**. Once lifted, everything downstream
(registry key, actor mailbox routing, Lance version stamp,
audit-log dimension, HHTL compile-time codegen) routes through the
*same* IR.

The aphorism: **"Elixir AST is input dialect; the canonical IR is
wire truth."** Generalizes to any source dialect; the IR is the
shared substrate.

**Alternatives considered.**

- *Forward the source AST as the wire format.* Rejected: leaks
  source-language syntax + semantics into every downstream consumer;
  breaks the firewall's "no serialization in hot path" invariant
  (source ASTs are too rich + heterogeneous to be compile-time-
  HHTL-resolvable); makes cross-source comparison (e.g. §14 oracle
  equivalence-running between OLD-stack Elixir + NEW-stack Rust)
  arbitrarily hard because the comparison surface differs per source.
- *Use a "least common denominator" subset of OWL DL.* Rejected: OWL
  doesn't model state machines or lifecycle behaviour (the
  `ActionDef` + `KausalSpec` axis OGAR adds past OWL DL); the LCD
  surface would be insufficient. OGAR sits *above* OWL DL (OWL is
  one of OGAR's supported source dialects, not a constraint on the
  IR).

**Consequences.**

- **Same IR → same hash → same actor routing → same Lance row →
  same audit dimension.** Content-addressing primitive. Already
  realized in code: `ogar-ontology::class_identity(prefix, name)`
  produces the canonical identity string; PR #31 closed the
  collision hazard; bardioc PR #19 (`substrate-b-shadow::EdgeDecoder<E>`)
  consumes the IR as `ActionInvocation` at the OLD-stack-shadow seam.

- **Adapters are pluggable, the IR is fixed.** New source dialects
  ship as new adapter crates (`ogar-adapter-surrealql`,
  `ogar-adapter-ttl` planned, `ogar-from-elixir`, `ogar-from-ecto`
  proposed). The `Class` IR doesn't change; only the lift code does.

- **Round-trip is the adapter contract.** `parse_<dialect>` →
  `Vec<Class>` → `emit_<dialect>` should reproduce the source. OGAR
  PR #32 demonstrated this for SurrealQL DDL; round-trip tests
  are now part of the adapter contract.

- **The `schema_ddl_hint` loop closes here.** PR #25 introduced
  `KnowableFromStore::register(class_identity, schema_ddl_hint:
  Option<&str>)` with the docstring claim *"so the registry is
  self-describing"*. PR #32 landed `emit_surrealql_ddl`. This PR
  wires the two together (feature-gated `surrealql-hint`): the
  registry now carries the producer's `Class` IR projected into
  SurrealQL DDL alongside the `knowable_from` stamp. The IR-as-wire-
  truth claim is no longer aspirational.

- **Cross-session triangulation receipt.** bardioc PR #19
  (`substrate-b-shadow`) consumes `ogar-vocab` as a direct
  dependency — its `EdgeDecoder<E>` trait IS the IR-as-wire-truth
  pattern in code. The HIRO-Graph + ClickHouse decoders return
  `ActionInvocation` regardless of source; the rest of the substrate
  consumes one shape.

**Change policy.** Adding a new source dialect (new adapter crate)
is routine. Changing the IR — adding a field to `Class`,
`AssociationKind`, `EnumSource`, `KausalSpec` — is a substrate-wide
contract change requiring (a) backward-compatible default (typically
`Option<…>` field), (b) round-trip preservation in all adapter
crates, (c) consultation with the runtime session (bardioc /
lance-graph) before merge.

**References.**

- `crates/ogar-vocab/` — the canonical IR.
- `crates/ogar-ontology/` — identity routing + canonical-form helpers.
- `crates/ogar-knowable-from/` — the registry seam; this PR wires
  the `schema_ddl_hint` loop via the `surrealql-hint` feature.
- `crates/ogar-adapter-surrealql/` — first round-trip adapter (PR
  #24 wired the parser; PR #32 closed the walk + round-trip).
- `crates/ogar-from-elixir/` — Elixir SchemaSource scaffold.
- ADR-022 (The Firewall) — the invariant ADR-023 makes explicit.
- ADR-016 (SurrealQL DDL AST is not the universal IR) — the
  predecessor; ADR-023 generalizes ADR-016's claim from SurrealQL
  to *all* source dialects.
- bardioc PR #17 (Rubicon Phases 1-5) — consumer of `ogar-vocab`
  for actor dispatch.
- bardioc PR #19 (`substrate-b-shadow::EdgeDecoder<E>`) — the
  pattern materialized in runtime-side code.
- `docs/RDF-OWL-ALIGNMENT.md` §3 (OGAR's position in L1-L5) — the
  IR sits at the AR-pattern lift seam.

## ADR-024: Palette256 + HHTL codec — the substrate's universal compression primitive

**Status:** Pinned (2026-06-05). Names an existing primitive (three
independent deployments + one empirical anchor) rather than proposing
one. Companion to ADR-022 (The Firewall — this ADR specifies one of
its inner-side primitives) and ADR-023 (IR-as-wire-truth — palette256
is the codec on the IR's wire form).

**Context.** The substrate has accumulated three independent
palette256 deployments developed for their own domains:

- **Security mesh** — `Binary16K = [u64; 256]` in
  `MedCare-rs/crates/medcare-analytics/src/graph_contract.rs:36`
  (canonical home). The per-row `_effectiveReaders` bitmap; auth is
  Hamming-popcount bit-intersection at the inner / hot path
  (`HEALTHCARE-TRANSCODING.md §3.1`). Wired into production at
  `MedCare-rs/crates/medcare-analytics/src/column_mask_bridge.rs` →
  `medcare-server/state.rs:167, 265, 439`.
- **Attention** — `bgz-tensor` `WeightPalette::build(…, 256)` +
  `AttentionTable::build` (`crates/bgz-tensor/examples/
  compare_stacked_vs_i16.rs:90-92`). Replaces dense FP weights with
  256-index palette + precomputed distance table on the model's hot
  path.
- **Distance** — `lance-graph-arm-discovery` aerial codebook —
  measured **ρ = 0.9973 vs cosine**. The empirical anchor: palette256
  reproduces cosine distance with correlation 0.9973 (i.e. on a
  scale where 1.0 = identical, palette256 is ~0.003 from cosine).

Cross-domain analysis revealed all three are instances of the *same
codec*: HHTL prefix establishes a frame; within frame, values
cluster; clustered values quantize to a 256-index palette; decode is
a const-table lookup. The runtime side's BindSpace dissolution work
(bardioc PR #18 / lance-graph PR #470) hinted at this with the
Quintenzirkel qualia codebook ("frozen set + circle-of-fifths
progression → 8 B → 1-2 B per row") — same compression strategy,
different domain.

The proposal in the cross-session conversation (2026-06-05) was to
name the primitive explicitly so:
1. Future adopters don't reinvent it per domain.
2. New adopters report a falsifiable measurement (ρ-vs-reference)
   at adoption time rather than after the fact.
3. The 256-ceiling escape hatches are documented before reviewers
   ask.

**Decision.** **The codec is:**

```text
HHTL prefix         (NiblePath / quadkey / class identity)
  ↓  establishes spatial / semantic frame
within-frame values cluster
  ↓  quantize to 256-index palette
  ↓  const-table lookup (compile-time HHTL where possible)
1-byte index per element, sub-microsecond decode, zero heap allocation
```

**Adoption checklist** for a new domain:
1. **Identify the prefix.** The NiblePath / quadkey / class identity
   that establishes the frame the values live in.
2. **Identify the palette domain.** Which values cluster within the
   frame? (Closed-keyspace tags, quantized continuous values,
   enumerated state, etc.)
3. **Build the palette + measure ρ-vs-reference.** The reference is
   the domain's full-precision metric (cosine for embeddings, L2 for
   coordinates, exact-match for tags). Report ρ at adoption time as
   the falsifiable property. Target: **ρ ≥ 0.99** to match the
   arm-discovery anchor.
4. **Decode = const-table lookup.** Compile-time HHTL if the palette
   is static; runtime const-table if the palette is per-frame /
   per-tile. Either way the decode path is zero-allocation.

**The 256-ceiling escape hatches** (documented to avoid the
predictable reviewer question):

- **Per-tile / per-frame palettes** — the cheapest answer. Different
  spatial-frame, different 256 entries. Used by Cesium tile codecs;
  matches the quadkey-prefix discipline. Long-tail OSM tags inside a
  zoom-21 tile rarely exceed 256.
- **Hierarchical palettes** — coarser palette at higher quadkey
  levels, finer per leaf. Mirrors the standard tile pyramid; the SH
  L0/L1 vs L2/L3 split in `splat-fit` is the same pattern.
- **Palette-64K upgrade** — 2-byte index instead of 1, for hot
  palettes that genuinely exceed 256 distinct values (rare; reserve
  for measured cases, not speculation).

The escape hatches are part of the primitive, not exceptions to it.

**Alternatives considered.**

- *Continuous distributions that don't cluster* (e.g. timestamps in
  microseconds, free-form text). Rejected as a counterargument to
  the codec — these are out-of-domain. For them, use delta encoding
  or VarInt or a different codec entirely. The codec applies to
  *clustered* domains; the adoption checklist's step 2 is the filter.
- *Domain-specific codecs per domain.* Rejected. Three independent
  re-derivations of the same primitive (security / attention /
  distance) is the receipt that the abstraction is real, not the
  receipt that each domain should have its own. ADR-024 reduces
  per-domain re-derivation.
- *Skip the ρ-vs-reference measurement.* Rejected. The arm-discovery
  ρ = 0.9973 is the existing FINDING-grade stake; new domains
  reporting at adoption time keeps the empirical floor honest as the
  primitive spreads.

**Consequences.**

- **The primitive is named.** Cross-domain reuse is now load-bearing,
  not coincidental. New domains adopt the codec instead of inventing
  their own quantization.
- **ρ-vs-reference becomes the adoption contract.** Reported once at
  adoption per domain. The arm-discovery 0.9973 is the existing
  anchor; new adopters target ≥ 0.99 and document if they fall short.
- **Two next-domain adopters are queued** (planned, not yet wired):
  - **D-OSM-2** (OSM tag palette + tile-local coordinate
    quantization) — per `lance-graph` PR #473 (`cesium-osm-substrate
    -v1.md`). Reports ρ-vs-reference on first per-country PBF run per
    the runtime session's §11 follow-up commitment.
  - **D-SPLAT-4** (SH-aware palette extension on the
    `Gaussian3D` carrier) — per the splat-native arc. Same codec; SH
    coefficients are the long-tail-budget challenger.
- **The 256-ceiling has three explicit escapes** in the ADR body
  (per-tile / hierarchical / palette-64K). Reviewers don't need to
  re-derive the answer.
- **Cross-arc reuse argument is sharpened.** The substrate-reuse
  framing in `docs/RDF-OWL-ALIGNMENT.md §10` (geographic litmus
  complements anatomical) cashes out as: FMA-bones and OSM-vectors
  use *the same codec* (palette256 + HHTL prefix), not just the same
  IR. The §6 callout in `DOMAIN-INSTANCES.md` (queued, awaiting
  lance-graph PR #473 land) will reference ADR-024 as the falsifiable
  property.
- **The falsifiable property** that ties the substrate-reuse claim
  down: *"the same compile-time HHTL prefix + palette256 codec
  decodes (a) `_effectiveReaders` for row auth, (b) OSM way
  attributes at zoom-21 tile, and (c) FMA-bone SH coefficients at
  sub-microsecond per element with zero heap allocation."* If that
  property holds across all three, the substrate is doing its job.
  If it fails on one, the substrate is leaking dialect into the codec.

**Change policy.** Adding a new palette256 adopter (new domain) is
routine — follow the adoption checklist + report ρ-vs-reference.
Changing the codec itself (e.g. palette-64K becoming default, or a
new escape-hatch added) is a substrate-wide concern and requires
consultation with the runtime session.

**References.**

- `lance-graph/.claude/board/EPIPHANIES.md:28` — FINDING-grade
  anchor for palette256 + Hamming popcount on `_effectiveReaders`.
- `lance-graph/.claude/knowledge/old-stack-capability-parity.md §3.39`
  — knowledge-doc record of the same primitive.
- `MedCare-rs/crates/medcare-analytics/src/graph_contract.rs:36` —
  `Binary16K = [u64; 256]` canonical home.
- `MedCare-rs/crates/medcare-analytics/src/column_mask_bridge.rs` —
  production wire-up; `redaction_mode_for` (line 128),
  `column_mask_policy_for_table` (line 165),
  `build_medcare_column_mask_registry` (line 192).
- `MedCare-rs/crates/medcare-server/src/state.rs:167, 265, 439` —
  F2-E install sites consuming the column-mask registry.
- `bgz-tensor/examples/compare_stacked_vs_i16.rs:90-92` —
  `WeightPalette::build(…, 256)` + `AttentionTable::build`.
- `lance-graph-arm-discovery` — aerial codebook with ρ = 0.9973 vs
  cosine measurement.
- ADR-022 (The Firewall) — the inner-side discipline this ADR
  specifies a primitive for.
- ADR-023 (IR-as-wire-truth) — palette256 is the codec on the IR's
  wire form.
- `docs/THE-FIREWALL.md` §3 (the inner/hot side) — palette256 + HHTL
  is one of its load-bearing primitives.
- `docs/HEALTHCARE-TRANSCODING.md §3.1` — palette256 + Hamming
  popcount on Binary16K named as the inner-side security mesh.
- `docs/RDF-OWL-ALIGNMENT.md §10` — the brutal-upgrade sequencing
  context (Phase 2c geospatial adopts the codec).
- `bardioc` PR #18 + `lance-graph` PR #470 — Quintenzirkel qualia
  codebook (8 B → 1-2 B per row) as the same compression strategy in
  a different domain.
- `lance-graph` PR #473 (forthcoming) `cesium-osm-substrate-v1.md`
  §11 — runtime-side commitment to a follow-up callout on this ADR
  once D-OSM-2 / D-SPLAT-4 wire.

## ADR-025: Probe-free hot path — address arithmetic + Jirak certificate give the level

**Status:** Pinned (2026-06-05). Completes the substrate-doctrine
canon (ADR-022 boundary / ADR-023 IR / ADR-024 codec / ADR-025
selection). Names an existing capability surface — three independent
address-as-geometry deployments + one independent bound machinery,
all already in production — rather than proposing a new mechanism.

**The one-sentence statement:** *Address arithmetic (helix + HHTL)
gives geometry; Jirak certificate (jc) gives error; together they
give level — and the level is the only thing the hot path picks.*

**Context.** The substrate has accumulated three independent
**address-as-geometry** surfaces:

- **Cesium 3D Tiles 1.1 implicit-tiling** (`crates/cesium/src/
  implicit_tiling.rs`) — child bounds are derived from parent + child
  index by closed-form arithmetic, not stored per-tile. The OGC
  spec §7.3.1 already specifies this.
- **Helix half-orb placement** (`crates/helix`) — golden-stride
  hemisphere; centroid + Σ at every cell are derived from the
  prefix-bit-pattern's projection through the helix template; no
  per-scene fit.
- **HHTL radix-prefix routing** (`NiblePath`) — identity-IS-address;
  the same primitive routes actor mailbox, registry key, Lance
  version, audit dimension (ADR-023 / ADR-024).

And one independent **bound machinery**:

- **`jc::weyl`** — proves 1-D `{k·φ⁻¹ mod 1}` star-discrepancy at
  representative N (e.g. N=144, N=1000), supplying the constructive
  certificate that golden-stride sampling achieves Jirak-grade
  concentration. *[citation per runtime session — not personally
  verified from the OGAR side]*
- **`jc::jirak`** — supplies the rate `n^{-(p/2-1)}` for p ∈ (2,3]
  and `n^{-1/2}` in L^q for p ≥ 4 under weak dependence. Pinned as
  the **`I-NOISE-FLOOR-JIRAK` iron rule** in the workspace's root
  CLAUDE.md. *[citation per runtime session]*

Cross-domain analysis (runtime session, 2026-06-05) revealed: when
address arithmetic gives geometry (helix + HHTL) AND a Jirak-grade
error bound gives the level (jc), the hot path becomes **probe-free**
by construction — no residual-check branch, no metadata-fetch round-
trip, no fallback path. The empirical-probe pattern collapses into
closed-form level selection.

**Decision.** Three composable primitives form the address-and-error
substrate; their composition is **the** mechanism the hot path uses
to pick its work, with zero data-dependent branches:

```text
  Address arithmetic    →  geometry  (closed-form, branch-free)
       │
       │  (helix + HHTL)
       ▼
  Jirak certificate     →  error    (predicted per-level, deployment-
       │                            static; not per-query)
       │  (jc::weyl + jc::jirak)
       ▼
  Together              →  level    (smallest L where predicted_error_L
                                     < tolerance)
       ↓
  The level is the only thing the hot path picks.
       ↓
  Single address-arithmetic pass → single Lance column read → render.
```

**The three primitives in detail.**

1. **Helix + HHTL = bounds are addresses, not negotiations.**
   At any HHTL depth L, cells are partitioned by NiblePath prefix at
   depth L with identical templated geometry inside each cell.
   Boundary = address-prefix change. No fuzzy edges; no overlap-and-
   blend at tile borders; no per-tile metadata about where it ends.
   **Tile `bounding_volume` becomes a derived field, not a stored
   field** — both producer and renderer compute it from the same
   NiblePath arithmetic; no float-rounding daylight between them.

2. **Jirak says which level L the cheap path will succeed at.**
   For the LOD pyramid, sample size grows by a fixed branching factor
   `b` per hop-radius increment: `n = b^r`. Then:
     `predicted_error_L = C · n^{-1/2} = C · b^{-r/2}`
   (L^q regime, which the Σ-sandwich Mahalanobis distance lives in).
   Solving `predicted_error_L < τ` for the smallest accepting level:
     `C · b^{-r/2} < τ  ⟹  b^{r/2} > C/τ  ⟹  r > 2·log_b(C/τ)`
     **`r* = ⌈2·log_b(C/τ)⌉`** for clinical tolerance τ — closed-form,
     evaluated once per task class.
   The helix pyramid ships **`b = 16`** (16-way per-hop sample growth
   — the pyramid increases resolution ×4 in **both** x and y per hop,
   so `4 × 4 = 16` samples per level; per the runtime session's
   helix-pyramid spec), so `n = 16^r`, `n^{-1/2} = 4^{-r}`, and the
   general formula reduces to the canonical anchor:
     `r* = ⌈2·log_16(C/τ)⌉ = ⌈log₄(C/τ)⌉`.
   **Implementer note for `D-JC-PREDICT-LOD`:** build the level
   formula on the pyramid's true per-level *sample* growth, not its
   per-axis fan-out. Here the ×4 per-axis fan-out gives `n = 16^r`
   (`= 4 × 4` per level) → `r* = ⌈log₄(C/τ)⌉`. The trap Codex flagged:
   pairing that `⌈log₄⌉` pick with a pyramid that actually grows only
   `n = 4^r` selects a level **half as deep as that pyramid needs** —
   a 4-ary pyramid needs `r* = ⌈log₂(C/τ)⌉` (since `log₂ = 2·log₄`) —
   and so leaves the predicted error above tolerance. Verify the real
   `b` before wiring the formula.
   No per-frame probing; the LOD pick is `r*` directly.

3. **HHTL routes the prefix at the picked level.**
   Standard radix-trie traversal. The hot path does the address
   arithmetic to get `(prefix_at_L, cell_centroid, cell_Σ,
   palette_window, predicted_error)` in one closed-form pass, then
   one read of the Lance column at that prefix.

**The falsifiable property.** The hot path takes **zero data-
dependent branches**. Verified by *absence*:

- No `if residual < threshold` patterns in `crates/cesium/src/sse.rs`
  after `D-CESIUM-IMPLICIT-HELIX` lands.
- No `try_then_fallback` patterns in registration solvers after
  `D-JC-PREDICT-LOD` lands (the ICP-Σ-sandwich path in particular).
- No bounds-check branch / metadata-fetch round-trip in the Cesium
  tileset consumer path after `D-HELIX-HHTL-BOUNDS` lands.

If the absence holds across the three sites, the substrate is doing
its job. If `if`-branches reappear on data, the substrate is leaking
probing into the codec.

**Three queued deliverables** (runtime-side; mirror ADR-024's
D-OSM-2 / D-SPLAT-4 adoption pattern):

| D-id | What | Repo |
|---|---|---|
| **D-HELIX-HHTL-BOUNDS** | `helix::bounds(nibblepath) → BoundingVolume` and `helix::centroid(nibblepath) → Vec3` as closed-form bounds/centroid derivation | `crates/helix` (ndarray-hpc feature) |
| **D-CESIUM-IMPLICIT-HELIX** | Extend `crates/cesium/src/implicit_tiling.rs` to consume the helix-bounds derivation as one of its tiling backends, alongside the OGC §7.3.1 uniform-subdivision backend already there | ndarray cesium crate |
| **D-JC-PREDICT-LOD** | `jc::predict_lod(scene_cert, tolerance) → level` — consumed by SSE, registration, and tile-pyramid LOD selectors | `crates/jc` |

Each is small (50-200 LOC); each gates only its own concern; each is
consumed by D-SPLAT-12 + D-OSM-* + the existing 3DGS-ArcGIS-Cesium
plan as additive deliverables, not redesigns. The runtime session
committed in their preceding analysis to file a §11 ADR-025 callout
on both `cesium-osm-substrate-v1.md` and `splat-native-ultrasound
-v1.md` once this ADR lands (same cross-arc symmetry pattern as
ADR-024 → §11 callouts in PR #475 + PR #476).

**Alternatives considered.**

- *Empirical calibration per scene class.* Rejected: collapses the
  predictive-bound win and reverses the architectural payoff. The
  whole point is the wasted-cheap-try + the failure-detection branch
  going away; empirical calibration keeps both.
- *Stored bounds in tileset JSON.* Rejected: bounds drift across LOD
  levels (parent must contain children; floating-point error in the
  union accumulates), producer/renderer disagreement (server says
  `[-1.2, 1.3]`; renderer rounds to `[-1.19, 1.29]` and the missing
  strip is a visible seam), mandatory bounds-composition pass at
  producer time. Derived bounds via helix+HHTL eliminate all three.
- *Per-frame LOD probing.* Rejected: the branch becomes data-
  dependent on residual; branch predictor mispredicts; pipeline
  stalls. Every miss is a cache-line read on the expensive path.
  The Jirak-bound variant pays one constant-time check (data-
  INdependent: function of L and n at that level, not of the
  specific query).

**Honest scope — what this ADR does NOT do.**

- *Doesn't make the bad-case work go away.* When `predicted_error
  > tolerance` the expensive path is taken directly. What's saved
  is the wasted cheap-try + the fallback branch. If 95% of queries
  are bad-case, the savings are noise.
- *Needs the jc certificate per scene class.* Medical splat volumes,
  OSM tile aggregates, and ArcGIS scene layers each need their `p`
  parameter (the moment-bound order) measured once at substrate-
  build time. The certificate is deployment-static (per scene class,
  not per query); but it must exist. This is what `crates/jc` is
  *for* — and it's why the substrate has the certificate path
  explicitly named (CLAUDE.md `I-NOISE-FLOOR-JIRAK`).
- *Doesn't help if the dependence structure changes at scale.*
  Locality probe (e.g. 98.6% intra-basin at ~10³-class ontologies,
  unproven at 10⁸ per the relevant tracking PR) is still the slab.
  If Wikidata-scale locality degrades, the `p` parameter shifts and
  the bound loosens. Separate concern; doesn't invalidate ADR-025
  but does bound where it applies cleanly.

**Consequences.**

- **The substrate is probe-free in the hot path** is no longer a
  slogan — it's a deliverable name (the three D-* above) and a
  falsifiable property (the three absences above).
- **Cesium tileset `bounding_volume` becomes a derived field** for
  helix+HHTL-shaped tilesets. Producer/renderer round-trip lossless
  by construction.
- **5-12 hop range = `r* = ⌈log₄(C/τ)⌉`** (the `b = 16` reduction of
  `r* = ⌈2·log_b(C/τ)⌉`) evaluated against clinical (FMA
  registration), pixel (Cesium SSE), and tile (OSM pyramid)
  tolerances. Same formula across the three domains; the
  per-domain Jirak certificate supplies `C`.
- **theta ∈ [1.45, 1.6] window** is the regime where the Jirak
  bound applies cleanly to palette256 distances (ADR-024 codec).
  Outside it, the bound loosens and the predictive guarantee
  degrades. The window is the empirical regime the substrate's
  codec is calibrated in.
- **The four ADRs form the substrate-doctrine canon**:
    - **022** — boundary (no serialization in hot path)
    - **023** — IR (Class is wire truth; dialects absorbed by
      adapters)
    - **024** — codec (palette256 + HHTL = 1-byte index, sub-µs
      decode, ρ ≥ 0.99)
    - **025** — selection (address gives geometry, certificate
      gives error, together give level; hot path picks level only,
      branch-free)
  Each closes a different floor; together they describe a substrate
  whose hot path is deterministic by construction.

**Change policy.** Adding a new adopter of the codec (palette256)
or of the selection mechanism (helix+HHTL+jc) is routine. Changing
the mechanism itself — adopting a different bound machinery (e.g.
moving from Jirak to a different concentration inequality), or
changing the closed-form address arithmetic at the helix or HHTL
layer — is a substrate-wide concern and requires consultation with
the runtime session.

**References.**

- ADR-022 (The Firewall) — "no serialization in hot path" sister
  claim; ADR-025 closes the loop on "no probing in hot path."
- ADR-023 (IR-as-wire-truth) — semantic substrate the level picks
  navigate over.
- ADR-024 (Palette256 + HHTL codec) — the codec the level decode
  consumes; theta ∈ [1.45, 1.6] window is the regime where the bound
  applies cleanly to palette256 distances.
- `crates/helix` (runtime) — golden-stride hemisphere placement.
  *[per runtime session]*
- `crates/jc` — `jc::weyl` + `jc::jirak` certificate machinery.
  *[per runtime session]*
- `crates/cesium/src/sse.rs` — SSE LOD math (will lose probing
  branches per `D-CESIUM-IMPLICIT-HELIX`). *[per runtime session]*
- `crates/cesium/src/implicit_tiling.rs` — 3D Tiles 1.1 implicit
  tiling (`D-CESIUM-IMPLICIT-HELIX` entry point). *[per runtime
  session]*
- CLAUDE.md (workspace root) `I-NOISE-FLOOR-JIRAK` — the iron rule
  that gates the bound machinery. *[per runtime session]*
- `bardioc` PR #18 + `lance-graph` PR #470 (BindSpace dissolution
  handover) — runtime-side trie-append pattern that helix+HHTL
  composes with.
- `lance-graph` PR #475 + PR #476 — the §11 ADR-024 callout
  symmetry pattern (D-OSM-2 + D-SPLAT-4 both routed through ADR-024
  contract); ADR-025 follows the same shape — `lance-graph` PR
  #475 + #476 forthcoming amendments for §11 ADR-025 callouts on
  `cesium-osm-substrate-v1.md` + `splat-native-ultrasound-v1.md`
  per the runtime session's explicit commitment.

**Honest discipline note.** I have not personally verified the
runtime-side artifacts cited (`crates/jc`, `crates/helix`,
`crates/cesium/src/{sse,implicit_tiling}.rs`, CLAUDE.md
`I-NOISE-FLOOR-JIRAK`). Citations are taken as authoritative from
the runtime session's 2026-06-05 cross-session analysis (the same
verification pattern that ADR-024's three deployments + ρ = 0.9973
anchor were carried under). The §11 amendments the runtime session
will file once this ADR lands are the receipts that close the
verification loop from their side.

## Implementation receipts — ADR ↔ commit cross-reference

> **Added in follow-up addendum (2026-06-05).** Records the implementation
> receipts for ADRs that moved from **Pinned** to **Pinned + Implemented**
> via the runtime/bardioc session's Phase 1 → 4 work. Future sessions
> citing an ADR can now resolve to both the architectural decision *and*
> the executable proof.
>
> **Closed-loop end-to-end** (proven 2026-06-04):
>
> ```
> ActionDef → RubiconMachine::on_event
>   → evaluate_guard (StateGuard + Depends-via-DependsClosure)
>   → Transition::Goto(Committed)
>   → RubiconCommitHook::on_commit  (sync, fallible, I-2 preserved)
>   → LanceMembraneWriter::commit
>   → LanceMembrane::commit_event(row) -> u64
>   → version bump + LanceVersionWatcher fan-out
>   → next actor's Pending evaluates against post-commit deinterlace
> ```
>
> Cumulative Rubicon test count progressed Phase 1 → 4: **6/6 → 13/13 → 20/20 → 25/25** (cumulative at each phase). The final substrate has 25 tests covering every link in the closed loop.

### Receipt table

| ADR | Decision | Implementation | Receipt | Tests |
|---|---|---|---|---|
| **ADR-001** | `State = ActionState` (lifecycle binding) | Rubicon Phase 1 — `RubiconMachine` over `State = ActionState` + the `one_machine_type_drives_every_domain` test (same machine drives Odoo + chess + OP) | bardioc `9412c68` (Phase 1 repoint into bardioc as excluded crate; consumes merged `lance_graph_planner::temporal`) | 6/6 |
| **ADR-002** | `ActionDef` and `ActionInvocation` stay split | Rubicon Phase 1 — `Event = ActionDef`, `Context = ActionInvocation`, `realizes` link | bardioc `9412c68` | 6/6 |
| **ADR-005** | `EnterEffect { field, to_value }` typed | Rubicon Phase 1 uses `EnterEffect::transition(field, to_value)` for the Rubicon crossing effect | bardioc `9412c68`; structural emission shipped in OGAR PR #15 (`ogar-emitter`) | 6/6 |
| **ADR-007** | §3 signatures = canonical `ractor_actors::state_machine` | `ractor_actors` crate — `on_event` / `is_commit` / `timeout` / `on_timeout` / `Transition::{Goto,Stay,Postpone,Stop}` / sync-fallible `on_commit`; Rubicon Phase 1 binds against this surface | `AdaWorldAPI/ractor_actors` `feat/state-machine-actor @ 38a71a4` (incl. load-bearing `postponed_event_is_replayed_after_transition`); pinned via submodule | 7/7 (`ractor_actors`) + 6/6 (Phase 1 consuming) |
| **ADR-008** | `LanceMembrane::commit_event(row) -> u64` sibling as the `CommitHook::on_commit` target | (a) lance-graph PR #467 adds the sibling on `LanceMembrane`; (b) Rubicon Phase 2 wires `RubiconWriter::commit` → `LanceMembrane::commit_event` via `LanceMembraneWriter` adapter (`gate_commit=true` marker, deterministic 128-bit content-address fingerprint) | lance-graph PR #467 (merged) + bardioc Phase 2 `8c74c18` | lance-graph #467: `commit_event_ticks_version_and_returns_new` green; bardioc Phase 2: 13/13 |
| **ADR-009** | `lance-graph-planner::temporal` two-axis engine (TIME via HLC + DATA via `DependsClosure`) | (a) lance-graph PR #468 ships the engine (`classify` / `deinterlace` / `EpistemicMode` / `QueryReference` with HLC-aware signature + `DependsClosure` trait); (b) Rubicon Phase 3 implements `OgarDependsGuard: DependsClosure` for `KausalSpec::Depends { paths }`; `RubiconMachine<D>` generic with `NoDeps` default; full `KausalSpec` variant coverage — **the Room-2 unblock for Odoo `@api.depends` / Rails reactive callbacks** | lance-graph PR #468 (merged) + bardioc Phase 3 `b055bfc` | lance-graph #468: 13/13; bardioc Phase 3: 20/20 |
| **ADR-010** | `knowable_from` meet-point single ownership | **Half-implemented**: consumer side is `temporal::classify(row_version, knowable_from, v_ref)` live on lance-graph `main` (PR #468); producer side `register_class_knowable_from` is a stub in OGAR PR #18's `ogar-adapter-surrealql` (gated by `lance-bind` Sprint-5b + OGAR rust-version bump per ADR-017). Meet-point closes end-to-end when producer side wires. | Consumer: lance-graph PR #468 `bardioc/MIGRATION_SPINE.md §2` lists this seam as one of five tracked cross-session meet-points | Consumer side: covered by lance-graph #468 tests; producer side: stub awaiting wiring |
| **§14 acceptance gate** (referenced in ADR-018 migration scaffold + `SUBSTRATE-ENDGAME §2.4 / §6.2` + `CHESS-TRANSCODING §6` + `ELIXIR-HIRO-PREFETCH §2.4` + `OPENPROJECT-TRANSCODING §6`) | The oracle infrastructure for per-actor graduation verification (record migration form, replay native candidate, compare provenance-normalized, emit verdict bucket {PASS / DIVERGENT-RECONCILABLE / DIVERGENT-FAULTY / INDETERMINATE}) | Rubicon Phase 4 — `OracleSubstrate` trait + four `Verdict` outcomes + `compare_normalised` provenance-strip + `MinimalChessOracle` (drop-in for shakmaty) | bardioc Phase 4 `43b272a` | 25/25 |

### What this enables

Each "Pinned + Implemented" ADR now has bidirectional resolution:
- **Forward** (decision → implementation): cite ADR-NNN, follow the
  receipt to the commit + test count.
- **Backward** (implementation → decision): from the bardioc commit
  or the lance-graph PR, the relevant ADR explains *why* this shape.

The bardioc-side `MIGRATION_SPINE.md §2` carries the parallel "five
meet-points" table referencing OGAR's ADR numbers; both docs are
mutually-anchored. `CROSS_SESSION_COORDINATION.md` (bardioc-owned)
carries the cumulative Phase 1→4 + PR cascade record.

### What remains "Pinned" (decision shipped, implementation pending)

| ADR | Why still Pinned (not yet Implemented) |
|---|---|
| ADR-003 (Class field set 1:1) | Producer-arm completeness — the *consumers* of full `Class` fidelity (`ogar-from-ruby`, `ogar-from-elixir`, future `op-codegen-pipeline` integration) are scaffolds; producer extraction wiring is the gap |
| ADR-004 (three §6 vocab terms as TeKaMoLo sub-properties) | Vocab shipped (OGAR #10); consumed by Rubicon via `ActionDef` carriers (covered by ADR-008/009 receipts); the term-vocabulary itself is the decision — no further "implementation receipt" beyond the vocab landing |
| ADR-006 (`#[non_exhaustive]` on `EnterEffect`) | Convention-rule decision; "implementation" is the absence of a SemVer break + the constructor pattern in use; receipt is OGAR #15's correctness fix |
| ADR-011 (two-arm naming pattern) | Naming convention decision; "implementation" is consistent producer crate naming across `ogar-from-elixir` (shipped scaffold, PR #17) and future `ogar-from-ruby` / `ruff_elixir_spo` |
| ADR-012 (nexgen convergence) | Awaits nexgen's C16c sprint (`From<op_surreal_ast::*> for catalog::*` impls); architectural convergence is documented (`OPENPROJECT-TRANSCODING.md §10.2` + `SUBSTRATE-ENDGAME §1.5`); implementation is nexgen-side work |
| ADR-013 (paper_trail consolidation) | Observation, not enforcement — applies during OP-graduation when the §14 oracle compares PaperTrail rows on Rails-side vs Lance versions on substrate-side |
| ADR-014 (database hydrator pattern) | Generalizes existing TTL-hydrator pattern; first concrete implementation lands when `ogar-from-ruby` consumes OP's `workflows` table |
| ADR-015 (`Language` extension point) | Convention; implementations land case-by-case (e.g. `Language::Elixir` shipped in PR #10) |
| ADR-016 (SurrealQL DDL AST is not the universal IR) | Pure decision; "implementation" is the absence of the unification, which is the steady-state architecture |
| ADR-017 (`surrealdb-parser` cross-repo dep deferred behind feature flag) | Awaits OGAR rust-version bump 1.85 → 1.95; `ogar-adapter-surrealql` (#18 merged) ships with the `parse_surrealql_ddl` `todo!()` stub + the `surrealdb-parser` feature flag reserved |
| ADR-018 (Kanban-as-polyglot-dispatcher) | §14 oracle infrastructure shipped (Phase 4 receipt above); work-item-form trait + per-actor registration table + HTTP-sidecar bridge are the runtime-side next pieces |
| ADR-019 (OP-as-operator-pane) | Gated by Room 2 (work-item-form trait + first OP class graduation); the destination is documented (`SUBSTRATE-ENDGAME §3`); implementation is months of per-class graduation work |
| ADR-020 (SDK deeper than Foundry-OSS) | Vision; implementation is the cumulative outcome of all prior decisions reaching steady-state |
| ADR-021 (meta-hygiene: always grep peer crates) | Process rule; "implementation" is the consistent practice (validated by the absence of further `[lints]`-style cascade bugs after PR #15/#17/#18 fixes) |

### Cumulative ADR status as of 2026-06-05

- **Pinned + Implemented**: 6 of 22 ADRs have executable receipts (ADR-001, ADR-002, ADR-005, ADR-007, ADR-008, ADR-009; plus the §14 protocol referenced by ADR-018).
- **Pinned, half-implemented**: 1 (ADR-010 — consumer side live, producer side stubbed).
- **Pinned, awaiting implementation**: 14 (the remaining ADRs, each with a clear unlock condition per the table above).

This is the steady-state distribution: most ADRs are *architectural*
decisions whose "implementation" lands incrementally as the
substrate's wider ecosystem matures. The closed-loop core (ADR-001
through ADR-009) is now fully wired; the migration scaffold, OP-
graduation, and SDK-shape ADRs (ADR-018 through ADR-020) are
forward-looking by design.

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

