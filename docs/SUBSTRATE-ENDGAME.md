# Substrate Endgame — Five-Rooms Architecture: Migration Scaffold → Operator Pane → SDK

> **Purpose.** Capture the full architectural endgame for OGAR + substrate-b
> as a concrete five-rooms map, so future sessions can pick up the path
> without re-deriving every step from first principles. The architecture
> assembled in this session — migration scaffold (Kanban-as-polyglot-
> dispatcher), OpenProject-as-operator-pane (self-hosting recursion),
> visualization tier-stack (boring → sexy), and SDK shape comparable to
> Palantir Foundry going open source — is captured here in single-doc
> form because every step depends on the prior.
>
> **Reading mode: "multiple rooms ahead".** Each room is a discernible
> substrate state with concrete dependencies on the prior. We're in Room 1
> (typed contract on `main` + Rubicon Phase 1 verified + temporal engine
> shipping); this doc maps Rooms 2 → 5, with the dependency tree (§6)
> making the path explicit.
>
> **Brutal honesty.** Each room has known shape and known dependencies, but
> the total path is months of engineering, not a sprint. The unique angle
> vs Foundry-OSS isn't ambition — it's that the migration scaffold and
> self-hosting recursion are *structurally different patterns* that
> Foundry can't replicate by open-sourcing. Detail in §5.
>
> **Cross-session muscle memory.** This is the durable record. Future
> sessions, future maintainers, future contributors: this is the single
> source of truth for *where the substrate is going*. Architectural
> decisions made during the 2026-06-04 session that produced this doc are
> recorded in `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` (ADR-style).
> The two docs compose: this one is forward-looking, that one is the
> reasoning chain.
>
> Status: **CARVED v0** (2026-06-04). Roadmap; revisit per quarter as
> rooms graduate to "complete" status.

## 0. Multi-room executive summary

```
ROOM 1: TODAY                                ROOM 5: SDK ENDGAME
  - OGAR contract on main                       - SDK ships: stable public API
  - Rubicon Phase 1 verified                    - OP as production operator pane
  - lance-graph temporal::classify              - Migration scaffold proven
    (PR #468)                                   - Foundry-class capability, OSS
  - commit_event sibling                        - 3+ vertical deployments
    (lance-graph #467)                          - Community in awe
  - 3 transcoding specs                         - "deeper than Foundry-OSS"
    (chess / OP / Elixir-HIRO)                          ▲
  - ogar-from-elixir scaffold                           │
  - ogar-adapter-surrealql                              │
                                                Months of engineering, with
            ▼                                   each room unblocking the next.
ROOM 2: MIGRATION SCAFFOLD
  Kanban as runtime-polyglot dispatcher
    work-item forms: native ractor / BEAM call / Elixir interpreter /
                     HTTP-sidecar Rails / embedded CRuby /
                     reflection-dump-as-producer-input
  §14 oracle gates per-actor graduation: migration form → native Rust
  Substrate hosts existing Rails/Elixir apps INCREMENTALLY (no flag day)
            │
            ▼
ROOM 3: OP-AS-OPERATOR-PANE
  OpenProject migrates onto substrate-b (per Room 2)
  OP's UI (kanban board, Workflow table, RBAC, journals) becomes the
    operator pane for the substrate that hosts it (self-hosting recursion)
  Substrate's first user is itself; universality is demonstrated by survival
            │
            ▼
ROOM 4: VISUALIZATION TIER-STACK
  Boring:    Grafana panels (Lance commits, Rubicon transitions,
                              mailbox depth, deinterlace ratios, HLC drift)
  OP-native: kanban UI live-updates from substrate state
  Sexy:      3D actor topology, cognitive trajectory animation,
             four-frame deinterlace visualizer (Markov ±5 window)
            │
            ▼
ROOM 5: SDK ENDGAME
  Stable public API across ogar-vocab / ogar-proposal / ogar-adapter-* /
    lance-graph-planner / lance-graph-callcenter
  Reference implementation: OP-on-substrate as the visible production user
  Migration scaffold packaged: any Rails/Elixir shop can graduate
  Foundry-parity: ontology / actions / time-versioned / row-level perms /
    pipeline orchestration / multi-language frontends — all OSS
  Deeper than Foundry-OSS: see §5.3 (three structural differentiators)
```

The path is the bridge. Each step is a sprint or two; the total is months,
not weeks. Each room is operable in its own right (Room 2 alone unlocks
HIRO/Bardioc migration; Room 3 alone unlocks OP-as-product; Room 4 alone
unlocks observability for any deployment; Room 5 is the sum of the prior).
This doc is the bridge, not the build.

## 1. Room 1 — where we are now (the floor)

Concrete snapshot of substrate-b primitives, captured 2026-06-04 so future
sessions don't have to reconstruct the floor before reasoning about
rooms 2-5.

### 1.1 OGAR side (this repo, `AdaWorldAPI/OGAR`)

**Contract on `main`:**
- `docs/OGAR-AST-CONTRACT.md` — the typed surface the ractor codegen lands
  on (`State=ActionState`, `Event=ActionDef`, `Context=ActionInvocation`,
  with `EnterEffect` typed since #13). The runtime session's Rubicon
  Phase-1 binds against exactly this surface and proves
  `one_machine_type_drives_every_domain` (Odoo + chess + OP, same
  `RubiconMachine`, only IR data differs).
- `docs/ADAPTERS-AND-ACTORS.md` — the Action / SPO + TeKaMoLo vocabulary
  (carve-out v0 from 2026-06-04). Behavior arm's grammar.
- `docs/IDENTITY-MAPPING.md` — `Identity` (NiblePath) routing.
- `docs/ARCHITECTURE.md` — the Semantik / Syntax / Pragmatik trichotomy
  that grounds the whole design.
- `docs/LANCE-GRAPH-INTEGRATION.md` — OGAR's role as a `SchemaSource`
  producer into upstream `OntologyRegistry`; today the boundary is
  `pub mod boundary {}` (Sprint 5b `lance-bind` feature, deferred).

**Producers / vocabulary / scaffolds:**
- `crates/ogar-vocab` — `Class` / `ActionDef` / `ActionInvocation` /
  `EnterEffect` / `GuardFailurePolicy` / etc. The IR types.
- `crates/ogar-emitter` — emits OGAR triples; after PR #13 + the
  cascade-fix PR #15, emits typed `EnterEffect` as one link triple +
  two leaf triples (matches the §6 TTL shape).
- `crates/ogar-proposal` — owned-mirror of `MappingProposal` for the
  upstream boundary; `class_to_drafts` is fully implemented; the actual
  `impl SchemaSource` boundary is the deferred Sprint-5b piece.
- `crates/ogar-adapter` — adapter trait + HHTL lookup tables.
- `crates/ogar-ontology` — prefix conventions + NiblePath identity
  routing.
- `crates/ogar-from-elixir` (PR #17, **merged**) — wide OGAR scaffold
  for the OLD HIRO/Bardioc stack; 5 locked-shape tests cover the
  `gen_statem` Rubicon-statem case (vote_request with all four §6
  carriers), Ecto.Schema, Phoenix controller, Oban worker.
- `crates/ogar-adapter-surrealql` (PR #18, **open**) — `emit` wired
  with hand-written formatter aligned with C16b builders; `parse` is
  a `todo!()` stub pending the rust-version 1.95 bump; 12 tests lock
  the emit shape; `register_class_knowable_from` reserves the §10.3
  meet-point seam.

**Transcoding specs on `main` (the three calibration axes):**
- `docs/CHESS-TRANSCODING.md` (PR #12, merged) — closed-formal axis;
  trichotomy provably separated by construction; shakmaty's
  `Position::play(Move) -> Result<_, PlayError>` is the free §14 oracle.
- `docs/OPENPROJECT-TRANSCODING.md` (PR #14, merged; §10 added in
  PR #16, merged) — open-messy production-Rails axis; grounded in
  real model paths from `/home/user/openproject`; §10 names the
  two-arm pattern + nexgen convergence + `knowable_from` meet-point.
- `docs/ELIXIR-HIRO-PREFETCH.md` (PR #10, merged) — debt ledger for
  the OLD HIRO/Bardioc stack; every Elixir/OTP construct has its OGAR
  type home **before** `ogar-from-elixir` wires.
- `docs/SURREAL-AST-AS-ADAPTER.md` (PR #19, in review) — feasibility +
  brutal-honesty analysis of using SurrealQL AST as universal IR;
  conclusion: structural canonicalization via `From<Class> for
  catalog::TableDefinition`, behavioral arm stays OGAR-owned.

### 1.2 Runtime side (`AdaWorldAPI/lance-graph`, `AdaWorldAPI/ractor_actors`)

**On main (lance-graph):**
- `lance-graph-contract` — zero-dep canonical types (`Schema`,
  `LinkSpec`, `SemanticType`, `Marking`, etc.).
- `lance-graph-ontology` — `OntologyRegistry`, `MappingProposal`,
  `SchemaSource`, TTL hydrators (SKOS, PROV-O, schema.org, FIBO, Odoo,
  ZUGFeRD, SKR03/04, Wikidata-HHTL), 47KB Lance dictionary cache.
- `lance-graph-callcenter` — `ExternalMembrane` impl;
  `LanceMembrane::commit_event(row: CognitiveEventRow) -> u64`
  shipped in lance-graph PR #467 — **Gate 1 code-complete**.
- `lance-graph-planner::temporal` (lance-graph PR #468) — the
  deinterlace engine: `EpistemicMode {Strict, Aware, Retro}` +
  `QueryReference` (HLC-aware signature, single-server body) +
  `classify(row_version, knowable_from, v_ref) -> Classification` +
  `deinterlace(rows, v_ref, deps)`. `DependsClosure` trait is the
  DATA-causal seam (TIME-causal via HLC). 13 tests green.

**On `AdaWorldAPI/ractor_actors`:**
- `feat/state-machine-actor` @ `38a71a4` — the generic `StateMachine`
  crate with canonical signatures (`on_event -> Transition`,
  `is_commit`, `timeout`/`on_timeout`, sync-fallible `on_commit`,
  `Transition::{Goto, Stay, Postpone, Stop}` with FIFO postpone-replay).
  7/7 tests green including the load-bearing
  `postponed_event_is_replayed_after_transition`.

**On `AdaWorldAPI/surrealdb`:**
- Sprint C16b `TableDefinition::new_for_ddl(...).with_*(...)` +
  chainable setters in `surrealdb/core/src/catalog/{table, schema/field,
  schema/index}.rs`. Designed *for external codegen tools*. nexgen's
  `op-codegen-projection` is the first downstream consumer; the future
  `ogar-adapter-surrealql` body (post rust-version bump) is the second.

**Cross-session coordination:**
- bardioc's `CROSS_SESSION_COORDINATION.md` is the authoritative
  cross-session coord doc (runtime-session owned). The OGAR-side
  meet-point pins (§10.3 `knowable_from`) are in
  `OPENPROJECT-TRANSCODING.md §10.3` as the authoritative OGAR-side
  source until bardioc mirrors.

### 1.3 Rubicon (other session's verified Phase 1)

`RubiconMachine` exists in the other session's scratch (uncommitted to
a durable home as of the session capture). Implements `StateMachine`
over OGAR's IR with `State = ActionState`, `Event = ActionDef`,
`Context = ActionInvocation`. Test
`one_machine_type_drives_every_domain` proves universality across
Odoo `action_confirm`, chess ply, OP status transition — same machine
type, only IR data differs. Clippy clean, 6/6 tests, real `ogar-vocab`
+ `ractor_actors` deps.

Phase 2 work pending: `RubiconWriter` two backends (`LanceMembraneWriter`
+ `KvLanceWriter`) over the same Lance 7.0.0 commit contract.

### 1.4 Producers ecosystem (`AdaWorldAPI/ruff`)

- `crates/ruff_spo_triplet` — language-agnostic shared core; `ModelGraph`
  IR with `Model` / `Field` / `Function`; `expand()` → NARS-weighted SPO
  triples as ndjson loading directly into lance-graph SPO store. 7-
  predicate closed vocab (`rdf:type`, `has_function`, `emitted_by`,
  `depends_on`, `reads_field`, `raises`, `traverses_relation`).
- `crates/ruff_python_dto_check` — fully wired Python frontend
  (extractors / matcher / preflight / calibrate / 19KB observations).
  Detects Flask routes, Pydantic DTOs, decorators. CLI: `ruff-py-dto`.
- `crates/ruff_ruby_spo` — scaffold (`todo!()` stubs with exact Rails
  constructs documented). Locks the target SPO shape via a passing
  test against a hand-built `ModelGraph` fixture.

### 1.5 nexgen (`AdaWorldAPI/openproject-nexgen-rs`)

In-flight OpenProject-specific work that converges with OGAR via
`surrealdb-core::catalog`:
- `op-surreal-ast` (Sprint C16a) — OP-specific mirror of catalog layout.
- `op-codegen-projection` (Sprint C15) — DDL renderer via op-surreal-ast.
- `op-codegen-pipeline` / `op-codegen-bucket` (Sprints C9, C15) —
  extract-to-projection.
- Plus per-domain crates: `op-api`, `op-work-packages`, `op-projects`,
  `op-models`, `op-users`, `op-services`, `op-contracts`, `op-db`,
  `op-journals`, `op-notifications`, `op-attachments`, `op-auth`,
  `op-queries`, `op-server`, `op-cli`, `op-core`.

C16c sprint plans `From<op_surreal_ast::*> for catalog::*` impls;
generalization to `From<ogar_vocab::Class> for catalog::TableDefinition`
is the convergence path (per `OGAR-AST-CONTRACT.md §2` +
`SURREAL-AST-AS-ADAPTER.md §5`).

### 1.6 Gaps blocking Rooms 2-5

Known and listed honestly:
1. **`lance-bind` boundary** — `crates/ogar-proposal::boundary {}` is
   empty placeholder. The `From<ProposalDraft>` → `impl SchemaSource`
   wire-up needs cross-repo dep + protoc availability + `Box::leak`-
   based string interning. Blocks runtime OGAR → `OntologyRegistry`
   flow. Sprint 5b.
2. **`surrealdb-parser` cross-repo dep** — OGAR `rust-version = "1.85"`,
   surrealdb fork `rust-version = "1.95"`. Blocks `parse_surrealql_ddl`
   in `ogar-adapter-surrealql` (#18). Workaround: bump OGAR's
   rust-version when feasible.
3. **`ogar-from-elixir` extraction wiring** — scaffold landed (#17);
   tree-sitter-elixir integration is the meaningful next step.
4. **`ogar-from-ruby` crate** — not yet started; the narrow-arm scaffold
   `ruff_ruby_spo` exists, the wide OGAR-arm crate is queued (per
   `OPENPROJECT-TRANSCODING.md §9` and `§10.4`).
5. **`RubiconWriter::KvLanceWriter` impl** — Rubicon Phase 2; runtime
   session owned.
6. **Cross-server HLC policy** — `temporal::QueryReference` carries
   `hlc_tick: Option<u64>` from day one; single-server body works;
   cross-server merge policy is deferred until peer-Raft / cluster
   bus lands.

These are the rocks in the path. Each room below addresses what comes
after they clear.

## 2. Room 2 — migration scaffold (the Kanban-as-polyglot-dispatcher)

### 2.1 The Kanban contract — the stable interface

The whole migration scaffold rests on one architectural observation:
**the substrate's actor scheduler (the Kanban dispatcher per
`SOA-IMPLEMENTATION.md §5.2`) has a narrow contract** — a work-item is
"given `(state, event, ctx)`, produce `Transition<State>`, then on
`Goto(C)` where `is_commit(C)`, fire `CommitHook::on_commit`."

This contract doesn't care how the work-item's body is implemented. It
could be:
- A native Rust closure (the steady-state target).
- A method call on a ractor `Actor` impl (the conventional native form).
- A foreign function call into BEAM (Erlang Port / NIF dispatch to
  a `:gen_statem` callback).
- An interpreted Elixir AST evaluation (in-process, no BEAM).
- An HTTP RPC to a Rails Puma worker.
- An embedded CRuby `rb_funcall` invocation.
- A precomputed lookup from a static metadata dump (Rails AR
  reflection cached at boot).

What unifies them is the **shape of the contract**, not the runtime.
The Kanban is the stable adapter; the work-item's executable form is
free to vary.

```
┌──────────────────────────────────────────────────────────────┐
│  Kanban Dispatcher  (per SOA-IMPLEMENTATION §5.2)            │
│  WIP-bounded, pull-based, backpressure-aware                 │
└────────┬─────────────────────────────────────────────────────┘
         │ pulls work-item                                       
         ▼                                                       
┌──────────────────────────────────────────────────────────────┐
│  Work-item contract:                                         │
│     fn execute(state: &State, event: &Event, ctx: &mut Ctx) │
│         -> Transition<State>                                 │
│  +  fn is_commit(&State) -> bool                             │
│  +  CommitHook::on_commit(&from, &to, &ctx)                  │
└────────┬─────────────────────────────────────────────────────┘
         │ implemented by one of:                                
         ▼                                                       
┌──────────────────────────────┬──────────────────────────────┐
│  Native Rust ractor handler  │  BEAM-compiled Elixir call   │
│  (Rubicon-from-OGAR target)  │  (Erlang Port / NIF dispatch)│
├──────────────────────────────┼──────────────────────────────┤
│  Tiny Elixir-AST interpreter │  HTTP RPC to Rails sidecar   │
│  (limited commandlets)       │  (POST /api/v3/...)          │
├──────────────────────────────┼──────────────────────────────┤
│  Embedded CRuby via FFI      │  Static reflection dump       │
│  (in-process Ruby dispatch)  │  (AR Model.reflect_* at boot)│
└──────────────────────────────┴──────────────────────────────┘
```

The dispatcher above the dashed line stays unchanged across the
migration. The form below the dashed line graduates per-actor (or per-
controller-action, per-callback) from migration-form to native-Rust as
the §14 oracle proves equivalence.

### 2.2 Elixir-runtime work-item variants

Three feasible variants, costs ranked:

#### 2.2.1 BEAM embed (full Elixir runtime in-process)

Embed BEAM as a runtime sibling to the Rust process. Work-items become
Erlang Port or NIF dispatches into running `:gen_statem` /
`GenServer` / Phoenix callbacks.

- **Pros:** full Elixir / OTP fidelity; HIRO's existing actors run
  *unchanged*; behaviour-equivalence with the OLD stack is by
  construction (it IS the OLD stack, hosted differently).
- **Cons:** tens of MB binary bloat; separate GC; separate scheduler;
  the substrate-b parity story ("no JVM, no BEAM, just Rust + Lance")
  becomes a *target state*, not a starting state; cross-runtime
  marshalling overhead per call.
- **When justified:** HIRO/Bardioc has dozens of `:gen_statem`-shaped
  actors with complex transitions; rewriting each in Rust before the
  substrate is online is unacceptable lead time. Embed BEAM during the
  migration window; deprecate the embedding once graduation completes.

#### 2.2.2 Tiny Elixir-AST interpreter (limited commandlets)

Ship a small Rust crate that interprets a narrowed Elixir subset
(`gen_statem` state callbacks, `GenServer.handle_*`, Ecto changesets) —
without embedding BEAM. Pattern-matcher + tuple/list/map ops + closures
+ the OTP subset (`{:reply, _, _}`, `{:next_state, _, _}`,
`{:keep_state_and_data, [:postpone]}`, `[{:state_timeout, _, _}]`).

- **Pros:** smaller binary; no separate GC / scheduler; the "limited
  commandlets" framing is honest — Elixir's OTP behaviour vocab IS
  finite; the interpreter is a real engineering project but not
  open-ended (the subset is bounded).
- **Cons:** real engineering (writing an Elixir interpreter is not
  small even when narrowed); maintenance against Elixir's evolution;
  loses access to the full standard library (only the OTP subset
  ships).
- **When justified:** when BEAM embed is rejected for binary-size or
  no-runtime-mixing reasons, but per-actor migration still needs to
  graduate behaviour-equivalence-proven before native-Rust translation.

#### 2.2.3 HIRO bridge (peer node via distributed Erlang)

Run substrate-b alongside HIRO/Bardioc as a peer; the Kanban dispatches
work via distributed Erlang messaging or a custom protocol over
TCP/HTTP/gRPC. HIRO continues to run its Elixir actors; substrate-b
hosts the Lance commits and the new actors.

- **Pros:** no embedding; both runtimes stand alone; existing HIRO
  cluster topology survives; migration is "per actor swap from
  HIRO to substrate-b" coordinated at the cluster level.
- **Cons:** distributed-systems complexity (peer health, partitions,
  retries); two operational planes; the migration story is more about
  cluster topology than per-actor graduation.
- **When justified:** HIRO is already deployed at scale and needs to
  keep running while substrate-b is brought up. Probably the most
  realistic production migration pattern.

### 2.3 Rails-runtime work-item variants

Three feasible variants, costs ranked:

#### 2.3.1 Static reflection dump (the cheapest beauty win)

Boot the Rails app once in `rails runner` mode; dump every
`Model.reflect_on_all_associations` / `_validators` / `_save_callbacks`
/ `_create_callbacks` / `_destroy_callbacks` / `columns_hash` /
`store_accessors` / etc. to a structured file (JSON / TOML / Arrow);
feed that file to `ogar-from-ruby` as a producer input alongside (or
instead of) static Ruby AST extraction.

- **Pros:** no embedding, no sidecar, no runtime mixing — the cheapest
  beauty win; sees `acts_as_*` macro expansions and other runtime-
  materialized metadata that static AST extraction can't see; same
  general pattern as the `lance-graph-ontology` TTL hydrators
  generalized to AR seed data; **probably the right place to start
  for OP specifically**.
- **Cons:** static snapshot — doesn't capture runtime-conditional
  behaviour (e.g. STI subclasses loaded lazily, dynamic `define_method`
  results); requires one-time `rails runner` execution as a build step.
- **When justified:** **always, as a producer-quality improvement**;
  even if other Rails variants are used for runtime dispatch, the
  reflection-dump strengthens the OGAR extraction.

#### 2.3.2 HTTP sidecar (lowest-friction runtime variant)

Run the Rails app as a normal Puma deployment. Kanban work-items
become HTTP RPCs: `POST /api/v3/work_packages/:id/<action>`, with the
Rails controller doing the dispatch. Commit on 2xx; failure on 4xx/5xx;
graduate per controller-action.

- **Pros:** Rails app deploys *unchanged*; production-ready today;
  telemetry/auth via standard middleware; lowest commitment to a
  specific embedding strategy; per-call observability comes for free
  (HTTP request logs, traces, etc.).
- **Cons:** per-call HTTP overhead (typically 1-5ms); requires the
  substrate to network-talk to the Rails app (deploy topology
  consideration).
- **When justified:** **the default for OP migration**. Lowest friction,
  highest compatibility, easiest to reason about. CRuby/mruby/JRuby
  embedding is overkill unless per-call latency becomes load-bearing.

#### 2.3.3 Embedded CRuby via FFI

Embed `libruby` in the Rust binary; dispatch via `rb_funcall`;
reflection via `ActiveRecord::Reflection#reflect_on_*`.

- **Pros:** in-process performance (no HTTP overhead); GIL serializes
  Ruby per-process which matches the Kanban's per-actor mailbox model
  (one Ruby work-item at a time per actor is fine).
- **Cons:** binary bloat (libruby is multi-MB); C ABI is mature but
  complex; Ruby GC + Rust ownership boundaries need careful glue;
  thread-safety constraints differ between the two runtimes.
- **When justified:** if HTTP sidecar's overhead measurably limits
  throughput AND the deployment can't tolerate the two-process model.
  Rare; HTTP sidecar usually wins.

### 2.4 Work-item graduation protocol (the §14 swap mechanics)

The Kanban contract being stable means *graduation* is a per-actor swap
without disrupting the rest of the substrate. Concrete protocol:

```
PHASE 1 — record (migration form active):
   For each (actor, action) pair under migration:
     Run the action via the migration form (e.g. HTTP sidecar).
     Tape: (input_state, input_event, input_ctx, output_transition,
            output_commit_row, output_committed_state).
     Persist tape to a Lance dataset row.

PHASE 2 — translate (parallel-implement native form):
   `ogar-from-{ruby,elixir}` extracts the Class + ActionDef from the
   source code (with reflection-dump augmentation for AR).
   Rubicon-from-OGAR codegen produces `impl StateMachine for X` over the
   ActionDef. This is the native-Rust candidate work-item.

PHASE 3 — verify (§14 oracle):
   For each tape row, run the native candidate against the same
   (input_state, input_event, input_ctx) and compare:
     - Transition output: equal? (modulo postpone-replay reordering
       caveats — see lance-graph #468 temporal::deinterlace)
     - Commit row: provenance-normalized equality (strip trace_id,
       emitted_at_millis, ULID identity per ELIXIR-HIRO-PREFETCH §2.4)
     - Final state: same? (this is the load-bearing check)
   Verdict per row: PASS / DIVERGENT-RECONCILABLE / DIVERGENT-FAULTY /
   INDETERMINATE.

PHASE 4 — swap (atomic per-actor):
   When the §14 verdict shows N consecutive PASS / DIVERGENT-
   RECONCILABLE rows across operational load, swap the actor's
   work-item form from migration to native. Done at the Kanban
   dispatcher's actor-registration table: same actor identity, new
   work-item.
   Old migration form stays parked for rollback (a single config
   flag flips back if a regression surfaces post-swap).

PHASE 5 — deprecate (per migration runtime):
   When ALL actors that depended on a migration runtime have graduated,
   that migration runtime can be removed from the substrate binary.
   The graduation is observable: count of "migration-form work-items
   active" → 0.
```

This is a strict subset of the contract's §14 acceptance gate (PASS /
DIVERGENT-RECONCILABLE / DIVERGENT-FAULTY / INDETERMINATE), applied
per-actor not per-deployment.

### 2.5 What changes per actor as it graduates

Concretely, for a single OP `WorkPackage#save` action:

**Before graduation (HTTP sidecar form):**
```
Kanban work-item:
  identity:    "ogit-op::WorkPackage::PROJ-42::invocation::<ulid>"
  realizes:    "ogit-op::WorkPackage::action::save"
  state:       Pending
  execute_fn:  http_post("/api/v3/work_packages/42/save", params)
  is_commit:   |new_state| matches!(new_state, ActionState::Committed)
  on_commit:   |_, _, ctx| {
                 let row = CognitiveEventRow::from(ctx);
                 self.membrane.commit_event(row);
                 Ok(())
               }
```

The Rails app handles validation, callbacks (before_save / after_save /
after_commit), persistence to PostgreSQL, etc. The substrate only sees:
"work-item came in, HTTP returned 200, here's the resulting state,
commit the Lance row."

**After graduation (native ractor handler):**
```
Kanban work-item:
  identity:    "ogit-op::WorkPackage::PROJ-42::invocation::<ulid>"
  realizes:    "ogit-op::WorkPackage::action::save"
  state:       Pending
  execute_fn:  WorkPackageActor::handle_save  // generated by
                                              // Rubicon-from-OGAR
  is_commit:   |new_state| matches!(new_state, ActionState::Committed)
  on_commit:   |_, _, ctx| {
                 // apply ActionDef.on_enter (typed EnterEffect, PR #13)
                 ctx.object_instance.set_field(&effect.field, &effect.to_value);
                 let row = CognitiveEventRow::from(ctx);
                 self.membrane.commit_event(row);
                 Ok(())
               }
```

The actor is native Rust; the dispatch is in-process; commit semantics
are identical (the `on_commit` body is the same). The Kanban dispatcher
swaps the actor's `execute_fn`; everything else stays.

This is the per-actor migration unit. Multiply across all `WorkPackage`
actions, then all `Project` actions, then all other model classes, and
OP fully graduates from Rails sidecar to native substrate.

### 2.6 Why this is "feasible beauty" not "sandcastle"

The Kanban contract was already there (per SOA-IMPLEMENTATION §5.2);
this design just *names* the work-item form as a variable. The
implementation cost is mostly bridges (HTTP client, Erlang Port, FFI
glue) — well-understood engineering, not research. The §14 protocol is
the same one we'd apply to any cross-stack migration.

The key insight: **the substrate's commit/storage layer (Lance +
`CommitHook` + temporal deinterlace) is ALREADY built to be runtime-
agnostic**. Forcing every actor to be native-Rust before the substrate
goes live conflates the storage-layer correctness (proven) with the
actor-runtime translation (in flight). Decoupling them via the Kanban
polyglot pattern lets the substrate go live with proven-correct actors
(running in their original runtime), and grows native-Rust coverage
behind the §14 oracle.

That's what makes Room 3 (OP-as-operator-pane) reachable in months, not
years.

## 3. Room 3 — OP-as-operator-pane (the self-hosting recursion)

### 3.1 OpenProject's existing feature set as substrate operator vocabulary

OpenProject is — by accident of having been built as a project management
tool — *literally* the substrate's operator vocabulary. Each OP feature
maps to a substrate concept with eerie precision:

| OP feature | Substrate concept |
|---|---|
| **WorkPackage** (work item with status, assignee, priority, type, project) | A queued action / Kanban work-item (Pending → Committed/Failed/Cancelled lifecycle) |
| **Status** with `is_closed?` + `Workflow` table (`old_status_id`, `new_status_id`, `role_id`) | Lifecycle FSM with role-gated transitions = OGAR `KausalSpec::StateGuard` + `ActionDef` + `EnterEffect` |
| **Kanban board view** (drag-and-drop work packages between status columns) | The live operator UI for the Kanban dispatcher — drag = trigger state transition; column = `ActionState` lifecycle column or domain status |
| **Project hierarchy** (nested projects with custom_fields) | Substrate actor hierarchy / supervision tree, with per-project schema extensibility |
| **Members + Roles + Permissions** | RBAC for the operator pane (which operators can see / dispatch / configure which actors); maps to OGAR's `LokalSpec.actor / tenant / company` |
| **Notifications + Reminders** | Substrate observability for the human in the loop; every Rubicon commit → notification; every `StateTimeout` hit → reminder |
| **Journals** (paper-trail history of every change) | **Duplicate of Lance versions** — per `OPENPROJECT-TRANSCODING.md §4`, `has_paper_trail` is subsumed by the Lance version log. One less table; same query power. |
| **Watchers** | Subscriptions to substrate event streams (which actors / actions an operator wants notified about) |
| **Comments + Attachments** | Operator annotations on substrate state (which work-item is being investigated, which decision is contested, etc.) |
| **Time entries** | Latency / throughput annotations per work-item (the operator's "this took 4 hours" maps to the substrate's "this action took 4ms p99") |
| **Custom fields** | Per-class schema extensibility (operator-defined; same affordance as OGAR's `Class.attributes` extensions) |
| **Custom actions** | Operator-defined substrate actions (already exists in OP as the `CustomAction` model — exactly OGAR's `ActionDef` with operator-defined predicate / kausal / on_enter) |
| **Workflow** model itself | Operator-configurable Rubicon machines — change a workflow row, the substrate's Rubicon binding for that class updates (per `OPENPROJECT-TRANSCODING.md §3` data-driven FSM observation) |

The parallel isn't aspiration; it's how OP was *already built*. The
substrate's operator pane is a UI it doesn't have to build — it's
already shipping at <https://www.openproject.org>.

### 3.2 The self-hosting recursion

Once OP graduates onto substrate-b per Room 2, the substrate's first
production user is itself:

```
                                          OPERATORS (humans)
                                                  │
                                                  ▼
                                       ┌──────────────────────┐
                                       │  OpenProject UI       │
                                       │  (Hotwire / Turbo /   │
                                       │   Stimulus / ViewCmp) │
                                       └──────────┬───────────┘
                                                  │ HTTP / WebSocket
                                                  ▼
                                       ┌──────────────────────┐
                                       │  OpenProject Rails   │
                                       │  controllers / views │
                                       │  (Rails AR per Room 2│
                                       │   migration scaffold)│
                                       └──────────┬───────────┘
                                                  │ Kanban work-items
                                                  ▼
┌──────────────────────────────────────────────────────────────────────┐
│                  Substrate-b (the runtime OP is hosted on)           │
│                                                                      │
│  Kanban dispatcher (polyglot work-items)                             │
│      │                                                               │
│      ▼                                                               │
│  ractor actors per OGAR Class (incl. WorkPackage, Project, Status,   │
│      User, Role, Workflow — OP's own data model is substrate state)  │
│      │                                                               │
│      ▼                                                               │
│  Rubicon (StateMachine binding; lifecycle Pending→Committed)         │
│      │                                                               │
│      ▼                                                               │
│  CommitHook → LanceMembrane::commit_event (Lance version log)        │
│      │                                                               │
│      ▼                                                               │
│  lance-graph-planner::temporal (deinterlace, classify, four-frame)   │
│      │                                                               │
│      ▼                                                               │
│  Lance dataset (the one append-only log — schema + instance + audit  │
│      + temporal queries all on the same versioned bytes)             │
└──────────────────────────────────────────────────────────────────────┘
```

OP doesn't know it's hosted on the substrate. The Rails app stays a
Rails app from the operator's perspective. The substrate is the
runtime, the OP UI is the surface. Everything else flows from this:
operator drags a work-package from `In Progress` to `Done` → Rails
controller receives the request → dispatches to the substrate's Kanban
→ substrate's WorkPackageActor handles the save → Rubicon transitions
`Pending → Committed` → `CommitHook` writes the Lance version → OP's
view re-renders.

### 3.3 What each side gains

**OP gains (over current Rails-only deployment):**
- **Time-travel queries** — `WorkPackage.find(42).as_of(yesterday)` for free; the Lance version log is the substrate's primitive.
- **Built-in audit** — every change is a Lance commit with `trace_id`, `idempotency_key`, `emitted_at_millis`; no `paper_trail` table needed (already a Lance-substrate consolidation per §4 of OP transcoding).
- **Cross-stack observability** — temporal deinterlace handles concurrent writers without combed views; no more "stale association cache" bugs.
- **Substrate speed** — `lance-graph-callcenter` actors are sub-millisecond dispatch; OP's per-request latency benefits.
- **Schema mobility** — OGAR-emitted SurrealQL DDL means OP's model graph is portable across SurrealDB, PostgreSQL (via OGAR's projection), and any future backend.
- **Rubicon-bound workflow** — operator-edited `Workflow` table rows become live Rubicon machines (per §3.4 below); no Rails-app restart for FSM changes.

**Substrate gains (vs building an operator UI from scratch):**
- **20 years of UX polish** — drag-and-drop kanban, role-based views, custom fields, notifications. None of this needs to be designed.
- **Production-grade auth + RBAC** — OP's Role / Member / Permission model is the operator pane's auth surface; substrate doesn't reinvent it.
- **Multi-tenancy via Project hierarchy** — `LokalSpec.tenant` maps to OP's project; substrate gets tenant isolation as a side-effect of OP's existing model.
- **Self-hosting validation** — the substrate's first user is itself, at scale, with real operators. The architecture validates by surviving its own bootstrap (the universality demonstration this enables).
- **Community traction** — OP has a real user base; substrate adoption is gated by something operators recognize and want, not by greenfield "trust us, it's better."

### 3.4 The Workflow table as live Rubicon machines

This is the load-bearing piece for "operator-editable substrate
behavior." OpenProject's `Workflow` model (`db/schema.rb`:
`workflows` table) carries rows like:

| id | tracker_id | role_id | old_status_id | new_status_id |
|---|---|---|---|---|
| 1 | Task | Manager | New | In Progress |
| 2 | Task | Manager | In Progress | Done |
| 3 | Task | Developer | New | In Progress |
| 4 | Bug | QA | Done | Reopened |

Per the database-hydrator pattern (`OPENPROJECT-TRANSCODING.md §7`),
OGAR's `ogar-from-ruby` producer (or its reflection-dump variant per
Room 2 §2.3.1) reads this table at boot and emits one `ActionDef` per
row:

```rust
ActionDef {
    identity: "ogit-op::Task::action::transition_new_to_in_progress",
    predicate: "transition",
    object_class: "ogit-op::Task",
    default_subject: ActionSubject::User,  // OP transitions are user-driven
    default_modal: ModalSpec::Atomic,
    kausal: Some(KausalSpec::StateGuard {
        guard_field: "status_id".into(),
        guard_values: vec!["New".into()],
    }),
    on_enter: Some(EnterEffect::transition("status_id", "In Progress")),
    guard_failure_policy: Some(GuardFailurePolicy::Reject),
    state_timeout_millis: None,
    decorators: vec!["role_gate:Manager".into()],
    // ...
}
```

Each Workflow row → one ActionDef → Rubicon-from-OGAR generates one
match-arm in the `WorkPackageActor`'s state machine. When operators
edit the Workflow table (via OP's admin UI — already exists), the
substrate re-reads, regenerates the ActionDef set, and the Rubicon
machine for that class updates *without a deploy*.

**This is operator-configurable substrate behavior.** Not via a YAML
file or a custom DSL — via OP's existing admin UI that operators
already know how to use. The Rubicon binding is the bridge; the
Workflow table is the operator-facing knob.

### 3.5 What's required for Room 3 specifically

Concrete dependencies on Room 2:
- HTTP-sidecar form of OP work-items must work end-to-end (per Room 2.3.2).
- Reflection-dump producer-input must extend `ogar-from-ruby` (per Room 2.3.1).
- §14 oracle protocol must be running for OP-specifically (per Room 2.4).
- Per-class graduation must be tractable (`WorkPackage`, `Project`,
  `User`, `Role`, `Workflow` first; secondary models next).

When `WorkPackage` alone graduates from HTTP-sidecar to native ractor:
the operator pane is functional, latency drops, time-travel queries
work. That's a demoable milestone — OP graduates incrementally; not
all-or-nothing.

## 4. Room 4 — visualization tier-stack

Once OP-as-operator-pane is up (Room 3), substrate visibility falls
into three tiers, each enabling the next:

### 4.1 Boring tier — Grafana panels (off-the-shelf)

The substrate is observable by design: Lance writes are append-only
versioned; ractor mailboxes expose depth; Rubicon transitions are
discrete events; `temporal::classify` outputs CONTEMPORARY /
ANACHRONISTIC / SPOILER per row. Standard OpenTelemetry / Prometheus
exposition gives:

| Panel | Source | Insight |
|---|---|---|
| Substrate health | Process metrics + `LanceMembrane::commit_event` rate | "is the substrate alive" |
| Per-actor WIP | `KanbanMailbox::current_wip` | "which actors are saturated" |
| Per-actor p50 / p99 dispatch latency | `state_machine::on_event` instrumentation | "is per-class latency degrading" |
| Lifecycle rates | Rubicon `Pending → {Committed, Failed, Cancelled}` counters | "is anything failing more than baseline" |
| Postpone retry rate | `Transition::Postpone` counter / `Pending` events ratio | "how much premove-like backpressure" |
| StateTimeout hit rate | `on_timeout` counter / `state_timeout_millis` armings | "how often are SLA deadlines violated" |
| Lance commits/sec | `commit_event` invocation rate | "substrate throughput" |
| Lance version log size | dataset metadata | "storage growth" |
| Deinterlace ratios | `classify` output histogram (CONTEMPORARY / ANACHRONISTIC / SPOILER) | "how often do operators see combed views" — should be CONTEMPORARY >>> others |
| HLC tick drift | `QueryReference.hlc_tick` variance across server_ids | "is the cross-server clock holding" |
| §14 verdict ratios | per-class PASS / DIVERGENT-RECONCILABLE / DIVERGENT-FAULTY / INDETERMINATE counts | "graduation readiness per actor" |

Boring, useful, off-the-shelf. The substrate's exposition is the
non-fancy version of the visibility story; ops engineers can use this
without knowing what Rubicon is.

### 4.2 OP-native tier — the kanban UI augmented with substrate state

OP's kanban board is *already* visualizing work-items moving through
state columns. The augmentation: those work-items aren't UI mocks;
they're the substrate's actual Kanban dispatcher work-items. The
columns aren't custom display states; they're the `ActionState`
lifecycle (or per-class domain states) live-updating from substrate.

Concretely:
- **Drag a work-package** → triggers a substrate Kanban work-item → if
  `KausalSpec::StateGuard` permits, Rubicon transitions
  `Pending → Committed` → `CommitHook` writes Lance version → OP's
  WebSocket pushes the new state to the operator's browser → the
  kanban card visibly moves to the new column.
- **A `StateTimeout` fires** in the substrate → an actor's work-item
  transitions to `Failed` → OP's Notification model generates an
  operator notification → kanban card shows a red border + the
  failure reason; clicking opens the journal entry from the Lance
  version row.
- **A `Postpone` happens** → the work-item stays in the kanban
  column it was in (still `Pending`) but shows a small "queued for
  replay" indicator → when the next state change re-fires it, the
  card animates the eventual transition.
- **The `Workflow` table is edited** (operator admin UI) → the
  substrate's Rubicon machine for that class regenerates → the
  kanban board's available transition arrows update without a
  reload.

This is OP's existing UI showing real substrate semantics. The
implementation: a small adapter layer between OP's view layer
(ViewComponent / Hotwire stream) and the substrate's event stream
(`lance-graph-callcenter::version_watcher`). No new UI design — just
plumbing.

### 4.3 Sexy tier — bespoke visualizations

Where the substrate becomes visibly *interesting* rather than just
operable. Each of these is a custom build but small (typically a
single-page web app or Grafana datasource plugin); the substrate's
exposition is rich enough that the data sources exist:

#### 4.3.1 Live 3D actor topology

The supervision tree as a force-directed graph (CytoscapeJS / three.js /
react-force-graph). Nodes are actors; edges are message-passing /
parent-child supervision; node size = current WIP; node color =
recent failure rate; flowing particles along edges = real-time
message dispatch (sampled from the cognitive event log).

Operators see the substrate's actor structure as a living thing.
Anomalies (one node ballooning in WIP, a hot-path edge with
sustained particle flow) are visible at a glance.

#### 4.3.2 Cognitive trajectory animation

The Markov ±5 `CognitiveEventRow` trajectory animated in real time.
Each cognitive event is a point in a state space; the trajectory is
a line tracing recent decisions. Patterns (recurring loops, divergent
paths, dead-ends) become visible. This is what the `cognitive-shader-
driver` (lance-graph) was designed for; the visualization is the
human-facing readout.

#### 4.3.3 Four-frame deinterlace visualizer

The most architecturally novel piece. Shows the four interlaced
field-clocks (lance / surrealql / ractor / thinking — per
`OPENPROJECT-TRANSCODING.md §10.3`) as actual interlaced video-style
fields, with the deinterlaced output (`temporal::deinterlace`)
overlaid. When the four frames are in-phase (CONTEMPORARY dominates),
the output is clean. When one frame drifts (e.g. schema definition
arrives before instance data, ANACHRONISTIC spikes), the visualizer
shows the combed pattern + the `classify` decisions.

**This is the AGI-aspiration "thought is a Raft commit, replicate the
generator, re-run the wave locally" made visibly tangible.** You can
*watch* the deinterlacing happen. The substrate's correctness
guarantees become a visualization tool — the academic / research /
press-coverage angle.

### 4.4 The progression — each tier enables the next

Boring tier is automatic (just expose OpenTelemetry from existing
instrumentation). OP-native tier is mostly side-effect of Room 3 (OP
already does this for project state; the augmentation makes it real
substrate state). Sexy tier is bespoke but each piece is small (a few
weeks of frontend work per visualizer); the substrate's exposition
makes the data sources free.

The progression matters for adoption:
- **Operators** care about Boring (uptime, throughput, error rates).
- **Operators + Developers** care about OP-native (does my dragged
  work-package actually do what I think? is the workflow what I
  configured?).
- **Decision-makers + Researchers + Press** care about Sexy (is this
  architecture actually different? what does "substrate" mean visibly?).

All three tiers ship from the same substrate exposition.

## 5. Room 5 — SDK endgame (deeper than Foundry going OSS)

### 5.1 What "SDK" means here

Not "a CLI tool you `npm install`." The substrate already isn't shipped
as a single binary; it's a *set of crates* + *a reference operator
pane* (OP) + *a migration scaffold pattern* + *the temporal /
deinterlace runtime*. The SDK shape is making all of this:

- **Versioned and semver-stable.** Public API surfaces across
  `ogar-vocab`, `ogar-proposal`, `ogar-adapter-surrealql`,
  `lance-graph-{contract, ontology, planner, callcenter}`,
  `ractor_actors::state_machine` are pinned; breaking changes are
  major-version bumps; deprecation paths are documented.
- **Discoverable.** A `getting-started.md` walks from zero to a working
  per-class actor in <30 minutes. A `cargo new --template ogar-actor-
  class` (or equivalent) scaffolds the boilerplate. Examples cover
  each of the three calibration axes (chess / OP / Elixir-HIRO).
- **Extensible.** Adding a new producer (e.g. for a new source language
  like Go or Swift) follows a documented pattern (mirror
  `ogar-from-elixir` scaffold structure; pair with a narrow
  `ruff_<lang>_spo` for the SPO arm). Adding a new transcoding spec
  (e.g. for medical Wikidata) is a TTL hydrator + a docs file.
- **Hosted-reference-implementation.** OP-on-substrate-b is the
  reference operator pane. The substrate's first production user is
  visible, queriable, demoable. Other deployments graduate from
  "we've heard of it" to "we've seen it run."
- **Migration-friendly.** The migration scaffold (Room 2) means any
  Rails-AR / Elixir-OTP shop can graduate incrementally — there's no
  flag day. This is the SDK's "you can adopt this without betting the
  farm" feature.

### 5.2 Foundry parity comparison — the receipt

| Foundry capability | Substrate-b equivalent | Status (2026-06-04) |
|---|---|---|
| **Ontology curation** (object-type, link-type, action-type) | `ogar-vocab::Class` / `Association` / `ActionDef` | **shipped on OGAR `main`** (PR #9–#17) |
| **Action types** as first-class lifecycle objects with guards | `ActionInvocation` + `ActionState {Pending, Committed, Failed, Cancelled}` + `KausalSpec` (5 guard kinds) + `EnterEffect` (typed) + `GuardFailurePolicy` + `state_timeout_millis` | **shipped** (PR #10 — three statem terms, PR #13 — typed EnterEffect) |
| **Time-versioned datasets** + time-travel queries | Lance versions (single append-only log = schema + instance + audit) + `temporal::classify` per-row deinterlace | **shipped** (lance-graph PR #468) |
| **Branch-on-data** for "what if" scenarios | Lance branches (native Lance feature) + `EpistemicMode::Retro` for cross-version reads | **available** (Lance native) + **shipped** (PR #468) |
| **Row-level permissions** | palette256 + Hamming popcount on Binary16K `_effectiveReaders` per-vertex; auth check = bit-op intersection | **primitive shipped** (bardioc parity-plan §3.6); **wiring partial** (per-class `_effectiveReaders` materialization is per-domain producer work) |
| **Pipeline orchestration** | Kanban-as-polyglot-dispatcher (Room 2) + work-item graduation protocol (§14 oracle gates per-actor swap) | **architecture pinned in this doc** + Kanban primitive in `SOA-IMPLEMENTATION.md §5.2`; **implementation pending Room 2 sprints** |
| **Code-workbook / multi-language integration** | ruff producers (`ruff_python_dto_check` wired, `ruff_ruby_spo` scaffold, future `ruff_elixir_spo`) + SPO triplet expansion via `ruff_spo_triplet` | **Python wired; Ruby scaffold; Elixir queued** |
| **Operator pane / UI** | **OpenProject** as reference operator pane (Room 3) | **OP exists as Rails app**; substrate-b hosting via Room 2 migration scaffold; **architecture pinned, implementation pending** |
| **Multi-tenant deployments** | OP's `Project` hierarchy + `LokalSpec.tenant / company` in OGAR's IR | **affordance present** (OP's project model); **substrate-side tenant routing pending Room 3** |
| **Enterprise integrations** (SSO, audit export, etc.) | OP's existing Enterprise edition (SSO, LDAP, SCIM); Lance version log IS the audit export; Hotwire / Turbo APIs for UI extension | **OP has it**; substrate-side wiring is mostly Room 2 + Room 3 work |
| **Visualization tools** | Boring tier (Grafana — automatic), OP-native tier (kanban UI augmented — Room 3 side-effect), Sexy tier (3D actor topology, cognitive trajectory animation, four-frame deinterlace visualizer — Room 4) | **Boring tier automatic when OpenTelemetry exposition lands**; OP-native + sexy tiers are dedicated work but small per piece |
| **Closed-source proprietary platform** | **MIT-licensed Apache-2.0 OSS substrate** | the structural differentiator vs Foundry |

Architecturally most of it is on `main` or one-sprint-away. The
endgame is making it *legible as a product* — not researching what to
build. That gap (legibility, packaging, getting-started experience,
reference-operator-pane visible) is what Room 5 closes.

### 5.3 What makes this deeper than Foundry going OSS

"Foundry going open source" would be a meaningful event, but it
would still be Palantir-shaped — a vertical enterprise platform
someone else built and you adapt to. **Substrate-b is structurally
different in three ways that matter**:

#### 5.3.1 The migration scaffold IS the bootstrap

Foundry adoption is a project: data engineers map your existing
schema to Foundry's ontology, write data pipelines, set up auth,
build user-facing apps. That's months-to-years per engagement, with
six-figure-plus contract sizes to justify the migration cost.

The substrate's adoption mechanism (Room 2's Kanban-as-polyglot-
dispatcher) is a *subscribe-and-graduate* pattern: bring up the
substrate alongside your existing Rails / Elixir stack, route a
single actor's work-items through the substrate via the HTTP-sidecar
or BEAM-bridge mechanism, prove the §14 oracle passes for that
actor, graduate it to native Rust, repeat. **Per-actor commitment
ceiling, not platform-wide commitment ceiling.** Lower friction =
wider adoption.

If Foundry went OSS, you'd still need the engagement to lift your app
onto Foundry's worldview. Substrate-b's worldview is *additive* —
your app keeps running while specific behaviours graduate.

#### 5.3.2 OpenProject is the production reference operator pane, not a sales demo

Foundry has reference dashboards for sales demos. Substrate-b's
reference operator pane is **OpenProject itself** — a production
Rails app with thousands of real deployments, 20 years of UX work,
existing Enterprise tier with SSO/LDAP/SCIM, multi-language
localization, MIT license.

When you adopt the substrate, the operator pane you get is the
operator pane *every other adopter gets* — the same UI, the same
auth model, the same RBAC. That's network-effect-shaped: contributions
to OP's UI benefit every substrate adopter. Foundry's UI is
single-vendor; substrate-b's operator pane is OSS at all layers.

This isn't a UI Foundry could provide by going OSS — they'd open-
source their *application*; substrate-b open-sources the *operator-
pane pattern* with a production reference implementation.

#### 5.3.3 SPO + OGAR + temporal + Rubicon are substrate primitives, not application features

Foundry the *application* exposes: ontology editing, action types,
time-travel queries, row-level security, pipeline orchestration.
These are features of Foundry's worldview.

Substrate-b the *substrate* exposes the *primitives those features
are built from*: `ogar-vocab::Class` / `ActionDef`, `Lance` versions,
`palette256` per-element auth, Kanban dispatcher, `Rubicon::StateMachine`,
`temporal::classify`. **You don't have to want a Foundry-shaped
application to use these primitives** — you can build:
- Another OP (general-purpose project management).
- Another HIRO/Bardioc (knowledge-graph + automation).
- A medical Wikidata browser with row-level audit.
- A chess server with full-history replay and cross-game analysis.
- An AGI substrate where actor states represent cognitive states and
  Rubicon transitions are reasoning steps (the "thought is a Raft
  commit" line from `ARCHITECTURE.md`).

The substrate is the **layer Foundry would be built on if Foundry
restarted today**. That's the architectural depth: substrate-b is one
layer below Foundry in the abstraction stack. Going OSS at the
*substrate* layer is structurally different from going OSS at the
*application* layer.

### 5.4 The OP-as-unicorn vision

The user's framing: *"making OpenProject so lean and so perfect
surface that it becomes the unicorn it deserves that makes and
developers of OpenProject in awe to see the potential breathing"*.

Concretely this means OP's existing developer community sees their
own app *running on the substrate*, with capabilities OP couldn't
have on Rails alone:
- Real-time kanban that reflects sub-millisecond substrate state.
- Time-travel queries on any WorkPackage (`as_of` parameter).
- Audit log that IS the storage layer (no separate paper-trail table).
- Operator-edited Workflow rows become live Rubicon machines without
  Rails restart.
- Per-tenant performance isolation via the actor model.
- Visualizations (Room 4) that no Rails app could afford to build.

**OP's maintainer community is the first audience.** A working demo
of "your WorkPackage now has these capabilities, and your existing
code didn't change" is the conversation-starter. The OpenProject GmbH
team has built and maintained one of the most successful OSS PM
tools; showing them a substrate that makes OP *more itself* (faster,
more queryable, more observable) without rewriting OP is the "in
awe" trigger.

Concretely: a demo, a video, a blog post, a forum post in
opf/openproject's discussions. Engaging at the community level, not
the corporate level. The "unicorn it deserves" framing is a
projection of where this goes when OP-maintainers + substrate-b adopt
each other.

### 5.5 Community engagement path

Concretely, in dependency order:

1. **Working end-to-end demo** — `WorkPackage#save` running through
   substrate-b's Kanban + Rubicon + Lance commit, with Grafana panels
   showing live state. This is the screenshot.
2. **Blog post / talk** — "OpenProject as a substrate operator pane:
   what self-hosting recursion looks like." Aimed at OP's maintainer
   community + the broader Rails/substrate communities.
3. **GitHub discussion in `opf/openproject`** — formal introduction
   of the substrate; offer of collaboration; not a fork (`opf/openproject`
   stays the canonical Rails app; substrate-b is a hosting target).
4. **Joint sprint with OP maintainers** — graduate one OP model class
   (probably `WorkPackage` first) end-to-end with their input. They
   review the Rubicon-from-OGAR output for their model; they
   validate the §14 oracle results; they sign off on the swap.
5. **Production deployment** — OpenProject Edge (their staging) or a
   willing community instance. Real users, real load, real Grafana
   dashboards.
6. **Wider community** — once OP-on-substrate-b is production-proven,
   the SDK packaging is the wider-community moment: "if it works for
   OP, here's how to graduate your Rails or Elixir app."

Each step is months apart but each unlocks the next. The total
timeline from Room 1 to a credible Room 5 is somewhere between 12
and 24 months of focused engineering + community engagement,
depending on velocity.

## 6. Dependency tree — concrete path from Room 1 to Room 5

The architecture is real; the path needs to be navigated. Each leaf
unlocks specific parts of upstream rooms.

### 6.1 Critical path

```
[Room 1 — TODAY] --(Room 2 deps below)--> [Room 2 — MIGRATION SCAFFOLD]
                                                  │
                                          [Room 2 graduates OP class-by-class]
                                                  ▼
                                          [Room 3 — OP-AS-OPERATOR-PANE]
                                                  │
                                          [Room 3 + OpenTelemetry exposition]
                                                  ▼
                                          [Room 4 — VISUALIZATION TIER-STACK]
                                                  │
                                          [Room 4 sexy tier + community traction]
                                                  ▼
                                          [Room 5 — SDK ENDGAME]
```

### 6.2 Room 2 dependencies (migration scaffold)

In approximate dependency order:

| # | Dependency | Owner | Status |
|---|---|---|---|
| 2.1 | `ogar-from-ruby` crate scaffold (mirror of `ogar-from-elixir` scaffold landed in #17) | OGAR session | **next** queue item per `OPENPROJECT-TRANSCODING.md §9` + §10.4 |
| 2.2 | Rails-AR reflection-dump producer-input — `rails runner` script that emits `Model.reflect_*` to JSON/Arrow | OGAR session + nexgen session coordination | **near-term** — cheapest beauty win per Room 2.3.1 |
| 2.3 | `ogar-from-elixir` extraction wiring (tree-sitter-elixir integration) | OGAR session | scaffold landed #17; wiring is medium-effort sprint |
| 2.4 | Kanban dispatcher work-item polyglot interface — formalize the trait that admits multiple executable forms | runtime session | architecture pinned in this doc; implementation is small |
| 2.5 | HTTP-sidecar bridge — Rust client that issues `POST /api/v3/work_packages/:id/<action>` and decodes responses to `Transition<State>` | runtime session | engineering: HTTP client + JSON marshalling + retry/auth (standard) |
| 2.6 | `§14` oracle harness — record-replay protocol per Room 2.4 | runtime session | architecture pinned in this doc + per `ELIXIR-HIRO-PREFETCH §2.4`; engineering is fixture replay + provenance-normalized diff |
| 2.7 | Per-actor work-item registration (lookup table in Kanban: `actor_identity → work_item_form`) | runtime session | small; affects Kanban implementation details |
| 2.8 | Rubicon-from-OGAR codegen (other session's Phase-2 work) | runtime session | Phase-1 verified (this session); Phase-2 = `RubiconWriter` two backends + codegen wiring |
| 2.9 | BEAM bridge (Erlang Port or NIF) — IF the Elixir-side path is BEAM-embed instead of HTTP-sidecar | runtime session, post Rails sidecar | optional; HIRO/Bardioc-specific |
| 2.10 | Tiny Elixir-AST interpreter (limited commandlets) — IF the Elixir-side path goes interpreted | runtime session | optional; alternative to 2.9; substantial engineering |

**Critical path through Room 2**: 2.1 → 2.2 → 2.4 → 2.5 → 2.6 → first
per-actor §14 oracle pass on an OP `WorkPackage` controller action.
Then 2.7 + 2.8 land the swap protocol. The Elixir-side pieces (2.3, 2.9,
2.10) parallel for HIRO/Bardioc migration if that target lights up.

### 6.3 Room 3 dependencies (OP-as-operator-pane)

Mostly downstream of Room 2:

| # | Dependency | Owner | Status |
|---|---|---|---|
| 3.1 | First OP class graduation end-to-end (probably `WorkPackage`) | runtime + nexgen | gated by Room 2.1–2.8 |
| 3.2 | OP's Hotwire stream / WebSocket integration with the substrate's `version_watcher` | OGAR + nexgen | small adapter layer |
| 3.3 | Operator-edited `Workflow` table → live Rubicon machine update | runtime + nexgen | per Room 3.4; requires reflection-dump producer (2.2) + Rubicon dynamic regeneration capability |
| 3.4 | Per-tenant routing via `LokalSpec.tenant` ↔ OP `Project.id` | OGAR + nexgen | downstream design work; tenant model is well-defined |
| 3.5 | Per-class graduation for `Project`, `User`, `Role`, `Status`, `Workflow` (the substrate's own model) | runtime + nexgen | each is a Room 2 iteration; volume is the cost |

### 6.4 Room 4 dependencies (visualization tier-stack)

The Boring tier mostly comes for free; OP-native is Room 3 side-effect;
Sexy tier is dedicated work but small per piece.

| # | Dependency | Owner | Status |
|---|---|---|---|
| 4.1 | OpenTelemetry / Prometheus exposition from substrate-b crates | runtime session | small (existing instrumentation crates) |
| 4.2 | Standard Grafana datasource + dashboard JSON | optional contributor | small; community-friendly |
| 4.3 | OP kanban-view-augmented-with-substrate-state | OGAR + nexgen | Room 3 side-effect; small additional view-layer work |
| 4.4 | Live 3D actor topology web app | bespoke | small SPA (CytoscapeJS or three.js); needs substrate's actor-tree API exposed |
| 4.5 | Cognitive trajectory animation | bespoke | small SPA over the `CognitiveEventRow` stream |
| 4.6 | Four-frame deinterlace visualizer (the academic / press piece) | bespoke | small SPA over `temporal::classify` output stream |

### 6.5 Room 5 dependencies (SDK endgame)

| # | Dependency | Owner | Status |
|---|---|---|---|
| 5.1 | Public API surfaces stabilize (semver pinning across `ogar-*` + `lance-graph-*` + `ractor_actors::state_machine`) | OGAR + runtime sessions | iterative; aligned with each crate's `1.0` milestone |
| 5.2 | `lance-bind` boundary fully wired — `ProposalDraft` → `MappingProposal` via the Sprint-5b `impl SchemaSource` boundary | OGAR session | currently `pub mod boundary {}`; needs protoc + cross-repo build sorted |
| 5.3 | `getting-started.md` + `cargo new --template ogar-actor-class` scaffold | OGAR session | when 5.1 + 5.2 are settled |
| 5.4 | Examples covering each of the three calibration axes (chess, OP, Elixir-HIRO) as runnable starter projects | OGAR session | small per example; chess is most demo-friendly |
| 5.5 | Reference operator pane production deployment (OP-on-substrate-b at OpenProject Edge or community instance) | OGAR + nexgen + OP community | gated by Room 3 completion |
| 5.6 | Community engagement (talks, blog posts, OP community thread, joint sprints) | human-only — coordination work | gated by 5.5 being demo-able |
| 5.7 | First non-OP production adoption (a Rails / Elixir shop graduates an app onto substrate-b) | external adopter | the validation that the SDK is genuinely shippable |

### 6.6 Parallelizable vs serial

Parallelizable work that doesn't block the critical path:
- Room 4.4–4.6 (sexy visualizations) — bespoke, can land anytime after
  Boring tier (4.1).
- Reflection-dump producer extension (2.2) — small, doesn't block other
  Room 2 work.
- Per-class graduation iterations beyond the first — can be parallel
  across model classes once the swap protocol (2.7) is proven.
- `lance-bind` boundary (5.2) — important for SDK packaging but doesn't
  gate any single demo.
- Sexy visualizations — academic / press / talk material; lights up
  Room 5's "deeper than Foundry-OSS" argument without gating it.

Serial dependencies that **do** gate downstream work:
- Room 2.4 (Kanban polyglot interface) gates 2.5–2.10.
- Room 2.5+2.6 gate first per-actor §14 pass.
- First per-actor §14 pass gates 2.7+2.8 (swap protocol).
- 2.7+2.8 gate Room 3.1 (first OP class graduation).
- Room 3.1 gates Room 3.2–3.5 (the rest of OP).
- Room 3 substantially gates Room 5.5 (production deployment) and 5.6
  (community).

### 6.7 Critical-path time estimate (rough)

Honest unknowns, but bounded:
- Room 2 to first §14 pass on one OP action: **2-3 months** (Room 2.1–2.6
  sprints).
- Full OP `WorkPackage` graduation (all controller actions): **1-2
  months after first §14 pass** (Room 2.7–2.8 + iteration).
- Room 3 substantially complete (OP's core models all graduated):
  **3-6 months after first graduation** (Room 3.1–3.5; volume work).
- Room 4 visible (Boring + OP-native operational): **immediate** once
  Room 3 lands.
- Room 4 sexy tier: **3-6 months of dedicated bespoke work**, parallel
  to Room 5 prep.
- Room 5 demonstrable: **6-12 months from now** if focused; OP-as-
  unicorn community vision is post-deployment.

Total: **12-24 months** to a credible Room 5 from Room 1. Each room
is operable in its own right at the threshold; the substrate doesn't
need to be "complete" to be useful.

## 7. Open questions and risks

Honest list — what could derail any of the rooms.

### 7.1 Substrate-correctness unknowns

- **Cross-server HLC merge policy.** `temporal::QueryReference` carries
  `hlc_tick: Option<u64>` from day one (type-level), but the actual
  merge policy is deferred. When cross-server workloads land (peer-Raft
  / cluster bus), the policy becomes load-bearing. Risk: getting it
  wrong means combed-frame reads cluster-wide. Mitigation: the policy
  decision is deferred *deliberately*; cross-server can be tested in
  isolation before any production deployment.
- **`Postpone` replay ordering** under high concurrency. The Rubicon
  `state_machine` test
  `postponed_event_is_replayed_after_transition` proves FIFO replay
  for one actor. Whether the same property holds when multiple
  concurrent transitions interact with a single actor's postpone-queue
  is the harder property. The architecture handles it (per-actor queue
  is single-writer); the test coverage at scale is the gap.
- **`§14` oracle false-negatives**: provenance-normalization is hard.
  Stripping `trace_id`, `emitted_at_millis`, ULID identity is
  straightforward. Side-effects to external systems (Notification
  rows, email sends, webhooks) where Rails fires them in-transaction
  vs the substrate fires them post-commit — those are real semantic
  differences the §14 oracle has to detect *and* know how to bucket
  (DIVERGENT-RECONCILABLE vs DIVERGENT-FAULTY). Risk: graduating an
  actor on a false-PASS verdict.

### 7.2 Migration-scaffold risk

- **HTTP-sidecar latency in production**. Per-call HTTP overhead is
  1-5ms; if OP has hot-path actions where this dominates user-visible
  latency, the sidecar form becomes too slow before graduation
  completes. Mitigation: in-process embedding fallback (2.10 / CRuby
  FFI) for hot paths; or graduate hot paths first via prioritized §14
  oracle work.
- **Operator confidence in graduation**. The §14 oracle gives
  per-action PASS verdicts, but does that translate to operator-
  confidence in flipping production traffic from "Rails handles this"
  to "substrate handles this"? Probably needs human-in-the-loop
  review for the first N graduations; can become automated after a
  pattern is established.
- **Two-runtime maintenance cost during the migration window**. While
  the substrate hosts both migration-form actors AND native-Rust
  actors, the support burden is double. The window has to be short
  enough that maintenance doesn't dominate progress. Mitigation:
  prioritize per-class graduation by usage frequency; cold actors
  can stay in migration form longer.

### 7.3 OP-as-operator-pane risk

- **OP community engagement timing**. Pushing a substrate to OP
  maintainers *before* a working demo is premature; *after* a working
  demo, momentum compounds quickly. The risk is mis-timing the
  outreach — too early loses credibility; too late wastes community
  cycles. Mitigation: keep cross-session coordination tight; the
  community engagement step (5.6) is the human-only part of the
  critical path.
- **OP's existing roadmap.** OpenProject GmbH has their own product
  roadmap; substrate-b adoption asks them to entertain a parallel
  hosting story. Risk: even with a working demo, alignment with OP's
  business priorities is uncertain. Mitigation: position substrate-b
  as an additive runtime (the existing Rails app keeps shipping),
  not a fork.
- **Multi-tenant scope creep.** OP's multi-tenant story (per-project
  isolation) is well-defined but tying it to `LokalSpec.tenant`
  across the substrate is design work that can expand. Risk: scope
  creep makes Room 3 take twice as long as estimated.

### 7.4 SDK endgame risk

- **"Just one more refactor" syndrome.** Stable public API across 12+
  crates is hard; the temptation to "refactor one more thing" delays
  semver-1.0 indefinitely. Mitigation: pick a hard cutoff date for
  API freeze; document deprecation paths for known-imperfect surfaces;
  ship rather than perfect.
- **Documentation half-life.** Architecture docs (like this one) need
  to track reality. As crates evolve, the §6 dependency tree shifts;
  as decisions are made, the §7 open-questions list shrinks; as
  rooms graduate, the §1 floor moves up. Risk: stale docs are worse
  than no docs. Mitigation: per-quarter review of this doc + ADR
  appendix appended as decisions land.
- **Foundry-comparison reception.** The Foundry comparison in §5
  is sharp and accurate but politically loaded. Risk: framing matters
  more than substance for community reception. Mitigation: lead with
  capabilities, not comparison; the comparison earns its place by
  being technically defensible, not by being the headline.

### 7.5 Cross-session coordination risk

- **Bardioc's `CROSS_SESSION_COORDINATION.md` drift.** The OGAR-side
  pins (e.g. `OPENPROJECT-TRANSCODING.md §10.3` for `knowable_from`)
  are authoritative until bardioc mirrors. Risk: bardioc doesn't
  mirror in time and a third session re-litigates the seam. Mitigation:
  the runtime session that wrote temporal (lance-graph PR #468)
  already cites the §10.3 pin; mirror lands when they next update.
- **Nexgen / OGAR convergence.** `op-surreal-ast` + `op-codegen-
  projection` (nexgen) and the planned `ogar-from-ruby` + reflection-
  dump (OGAR) need to converge cleanly. The convergence is documented
  (`OPENPROJECT-TRANSCODING.md §10.2`); the implementation coordination
  is per-PR. Risk: implementations drift if cross-session check-ins
  lapse. Mitigation: the nexgen session has the C16c sprint scheduled
  for `From<op_surreal_ast::*> for catalog::*` — that's the natural
  convergence point.
- **This doc itself going stale.** The whole point is durable
  capture; failing at that is the meta-risk. Mitigation: each major
  PR in OGAR / lance-graph / nexgen that touches the rooms cited here
  should add a one-line update to this doc (e.g. "Room 2.5 landed in
  runtime PR #N").

## 8. Cross-references

### 8.1 OGAR (this repo)

- `docs/OGAR-AST-CONTRACT.md` — the typed surface (`Class` / `ActionDef` / `ActionInvocation` / `Identity`).
- `docs/ADAPTERS-AND-ACTORS.md` — Action / SPO + TeKaMoLo vocabulary.
- `docs/ARCHITECTURE.md` — Semantik / Syntax / Pragmatik trichotomy.
- `docs/LANCE-GRAPH-INTEGRATION.md` — OGAR as `SchemaSource` producer.
- `docs/CHESS-TRANSCODING.md` — closed-formal calibration axis.
- `docs/OPENPROJECT-TRANSCODING.md` — open-messy calibration; §10 names the two-arm pattern + nexgen convergence + `knowable_from` meet-point.
- `docs/ELIXIR-HIRO-PREFETCH.md` — OLD HIRO/Bardioc debt ledger.
- `docs/SURREAL-AST-AS-ADAPTER.md` — structural-vs-behavioral decision; §6 covers the migration scaffold counterpoint to §3.
- `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` — ADR-style capture of every architectural decision made this session (the backward-looking companion to this forward-looking doc).
- `vocab/ogar.ttl` — vocab terms including the three §6 Rubicon-statem carriers (`onEnter`, `guardFailurePolicy`, `StateTimeout`).

### 8.2 lance-graph

- `lance-graph-contract` — zero-dep canonical types (`Schema` / `LinkSpec` / etc.).
- `lance-graph-ontology` — `OntologyRegistry` / `MappingProposal` / TTL hydrators.
- `lance-graph-callcenter` — `LanceMembrane::commit_event(row) -> u64` (PR #467).
- `lance-graph-planner::temporal` — `classify` / `deinterlace` / `EpistemicMode` / `QueryReference` / `DependsClosure` (PR #468).
- Per-repo tailored view: `lance-graph/docs/SUBSTRATE-ENDGAME-RUNTIME-VIEW.md` (placed alongside this PR).

### 8.3 ractor_actors

- `feat/state-machine-actor` @ `38a71a4` — canonical `StateMachine` signatures (`on_event` / `is_commit` / `timeout` / `on_timeout` / `Transition` / `CommitHook::on_commit`).

### 8.4 openproject-nexgen-rs

- `op-surreal-ast` (C16a) — mirror of `surrealdb-core::catalog`.
- `op-codegen-projection` (C15) — DDL renderer.
- `op-codegen-pipeline` / `op-codegen-bucket` (C9, C15) — extract-to-projection.
- C16c sprint (planned) — `From<op_surreal_ast::*> for catalog::*` impls; convergence point.
- Per-repo tailored view: `openproject-nexgen-rs/docs/SUBSTRATE-ENDGAME-NEXGEN-VIEW.md` (placed alongside this PR).

### 8.5 surrealdb fork

- Sprint C16b — `TableDefinition::new_for_ddl(...).with_*(...)` builders in `surrealdb/core/src/catalog/{table,schema/field,schema/index}.rs`.
- `surrealdb-ast` + `surrealdb-parser` — first-class workspace crates.
- `.claude/op-codegen-bridge/README.md` — Sprint C16b initiative; the "external codegen tools" target this doc's `ogar-adapter-surrealql` aligns with.

### 8.6 ruff fork

- `crates/ruff_spo_triplet` — narrow SPO core; `ModelGraph` IR; `expand()` to NARS-weighted triples.
- `crates/ruff_python_dto_check` — Python frontend (fully wired).
- `crates/ruff_ruby_spo` — OpenProject Ruby scaffold (`todo!()` stubs).

### 8.7 bardioc

- `CROSS_SESSION_COORDINATION.md` — authoritative cross-session coord doc (runtime-session owned). OGAR-side meet-point pins (especially `knowable_from`) are mirrored from `OPENPROJECT-TRANSCODING.md §10.3`.

### 8.8 OpenProject upstream

- [opf/openproject](https://github.com/opf/openproject) — the production Rails app modeled in §1.5 and graduated in Room 3.

## 9. Doc lifecycle

- **Author:** OGAR session, 2026-06-04.
- **Status:** CARVED v0; forward-looking roadmap.
- **Update cadence:** per-quarter review minimum. Each major PR in
  OGAR / lance-graph / nexgen that crosses a room threshold should
  add a one-line update.
- **Companion doc:** `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` —
  the backward-looking session ADR record. Read together.
- **Repo placements:** master copy in this repo (OGAR); tailored
  views in `lance-graph/docs/SUBSTRATE-ENDGAME-RUNTIME-VIEW.md` and
  `openproject-nexgen-rs/docs/SUBSTRATE-ENDGAME-NEXGEN-VIEW.md`,
  each focusing on the slice that repo owns and pointing back at
  this master.
