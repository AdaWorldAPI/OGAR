# Elixir / HIRO prefetch — every migration debt has an OGAR home

> **Purpose.** The OLD HIRO/Bardioc stack is **Elixir** (BEAM/OTP + Phoenix,
> over a JVM graph core). Migration debt is the terminal metric of the whole
> substrate-b effort, and the only way to discharge it with confidence is
> **wire roundtrip** against the OLD stack. The runtime bidirectional adapter
> (`unmap`, the SurrealQL/wire parse direction) is OGAR **Sprint 4.5** — deferred.
> But the **types cannot be deferred**: every OLD-stack concept needs its
> canonical OGAR home *now*, so that when `unmap` lands it has a complete target
> and no debt is discovered late. This doc is that prefetch — the OLD-Elixir-stack
> → OGAR mapping, grounded in the existing vocab (`Class` / `Action` / `Identity`),
> not a parallel set.
>
> **Elixir gets the headstart** (`Language::Elixir`, this PR) deliberately: it is
> the source of every byte of migration debt and the bridge to the old adapters.
>
> Companion to: `ADAPTERS-AND-ACTORS.md` (Action / SPO+TeKaMoLo), `OGAR-AST-CONTRACT.md`
> (the lowering the ractor codegen lands on), `IDENTITY-MAPPING.md` (Identity).
> Status: **PREFETCH v0** (2026-06-04).

## 1. The prefetch principle

```
OLD Elixir/HIRO concept  ──(this doc: type home NOW)──▶  OGAR canonical type
                                                               │
                          ──(Sprint 4.5: unmap, deferred)──▶  runtime wire bridge
                                                               │
                          ──(§14 gate)──▶  wire roundtrip: same wire in ≈ wire out
```

Model the type home now; wire the runtime later. Debt that has no OGAR type is
debt discovered *during* cutover (the expensive time). Debt that has a type is
debt measured *before* cutover (the cheap time). So: **prefetch the types.**

## 2. The OLD stack, in two arms (same AST→IR→triples pipeline)

The OLD stack decomposes onto OGAR's existing two arms exactly as Rails/Odoo do:

- **Structural arm → `Class`** (`Language::Elixir`): Ecto schemas, Phoenix
  contexts, and the OGIT entity model.
- **Behavioral arm → `Action` (SPO+TeKaMoLo)**: GenServer/`gen_statem` callbacks,
  Phoenix channel/controller handlers, Ecto `Multi`, lifecycle hooks.

### 2.1 Structural arm — Elixir → `Class`

| OLD Elixir shape | OGAR `Class` mapping |
|---|---|
| `Ecto.Schema` (`schema "accounts" do … end`) | `Class { language: Elixir, name, table_name }` |
| `field :name, :string` | `Attribute { name, type_name }` + `AttributeOptions` |
| `belongs_to` / `has_one` / `has_many` / `many_to_many` | `Association{ BelongsTo / HasOne / HasMany / HasAndBelongsToMany }` |
| `embeds_one` / `embeds_many` | `Association` (nested) + `Attribute` (embedded shape) |
| `Ecto.Changeset` (`validate_required`, `validate_format`) | `Validation { target, rule_source }` |
| `@primary_key` / `@foreign_key_type` | `AttributeOptions` (indexed / type override) |
| Phoenix **context** module (bounded-context grouping) | `Identity` prefix segment (HHTL namespace) |
| OGIT entity types (BGFS File/Folder, Auth Person/Account, Ticket, …) | `Class` under the `ogit:` / `ogit-<app>:` prefix (their native ontology) |

### 2.2 Behavioral arm — Elixir → `Action` + the lifecycle state machine

This is the load-bearing thread. The OLD stack runs OTP state machines
(`gen_statem`, `:gen_state_machine`, and `rafted_value`'s Raft on top of it).
Every one lowers onto the **same** `Action` lifecycle the contract fixed:
`State = ActionState` (Pending → Committed/Failed/Cancelled), Rubicon crossing =
`on_enter(Committed)` = the Lance append.

| OLD Elixir shape | OGAR `Action` / lifecycle mapping |
|---|---|
| `gen_statem` / `:gen_state_machine` state callbacks | `Action` per transition; the **domain** state (the `gen_statem` state name) is the `KausalSpec::StateGuard` field — rides as data, not as the machine `State` |
| `gen_statem` `state_enter` callback | `ogar:onEnter` → `StateMachine::on_enter` → the commit (this PR's term) |
| `gen_statem` `postpone` | `ogar:guardFailurePolicy = Postponable` → `Transition::Postpone` (this PR's term) |
| `gen_statem` state timeout / `{:timeout, …}` | `ogar:StateTimeout` → per-state SLA on `Pending` (this PR's term) |
| `rafted_value` (Raft on gen_statem) | consensus actions; leader/follower/candidate are domain states; log entries are `ActionInvocation`s. (Consensus runtime stays `openraft` on the NEW side — separate Phase H+I.) |
| `GenServer.handle_call` (sync) | `Action { modal: Sync }` |
| `GenServer.handle_cast` (async) | `Action { modal: Async }` |
| Phoenix channel `handle_in` / `handle_out` | `ActionDef` on the **wire** surface (directly relevant to roundtrip) |
| Phoenix controller action (REST) | `ActionDef { subject: User }` |
| `Ecto.Multi` (transaction) | `Action { modal: Atomic }` |
| `after_commit` / `Oban` after-transaction | `Action { temporal: OnCommit }` |
| Quantum / cron job | `Action { subject: Cron, temporal: Scheduled }` |
| `@api`-style reactive (PubSub-triggered) | `Action { subject: Trigger }` |

### 2.3 Coordination / runtime — Elixir OTP → `LokalSpec` + control specs

| OLD Elixir shape | OGAR / NEW-stack mapping |
|---|---|
| `swarm` (distributed actor registry) | `LokalSpec.actor` (NiblePath routing) + actor materialization |
| `libcluster` / `libring` (clustering + hash ring) | actor placement / `LokalSpec.tenant` partitioning |
| `sbroker` / `pobox` (sojourn broker / mailbox backpressure) | bounded mailbox + `MessagingErr::Saturated` (ractor) — the `Async`/SLA-coord path, never the hot loop |
| `locker` (distributed locks) | `ModalSpec::Atomic` / the commit-time lock |
| `con_cache` / `lru_cache` (ETS cache) | the cache tier (storage concern; not an `Action`) |
| `expr` (expression evaluator) | the guard evaluator — `KausalSpec` dispatch |
| `exometer_influxdb` (metrics → TSDB) | observability via the Lance-version log (version-as-TSDB) + `trace_id` |

### 2.4 Wire / adapter — connecting to the adapters of the old (Sprint 4.5 target)

The OLD wire forms (the OGIT REST/WS graph API, Phoenix channels, Gremlin) are
what an `ElixirAdapter` / `HiroAdapter` maps to OGAR canonical. The bidirectional
`unmap` (Sprint 4.5) is the parse direction that lets a recorded OLD-stack wire
exchange become canonical `Action`s — the precondition for **wire roundtrip**:

```
record:  OLD Elixir stack          → (W_in, W_out_old)
replay:  W_in --ElixirAdapter.unmap--> canonical Action --[substrate-b]--> canonical --map--> W_out_new
assert:  W_out_new ≈ W_out_old   (provenance-normalized: strip trace_id / emitted_at_millis / ULID identity)
```

The provenance fields already on `ActionInvocation` (`trace_id`,
`idempotency_key`, `emitted_at_millis`, `parent_invocation`) **are** the
roundtrip normalization handles; `idempotency_key` is the OLD↔NEW correlation
key. The shapes already allow roundtrip — the missing piece is `unmap`, which is
why Sprint 4.5 is migration **critical-path**, not a deferral.

## 3. The prefetched-debt ledger (summary)

| Debt class | OLD Elixir source | OGAR home | Status |
|---|---|---|---|
| Schema / records | Ecto schemas | `Class` (`Language::Elixir`) | **typed** (this PR) |
| Relations | Ecto associations | `Association` | typed |
| Validations | Ecto changesets | `Validation` | typed |
| Lifecycle state machines | `gen_statem` / `rafted_value` | `ActionState` + `ogar:onEnter` / `Postponable` / `StateTimeout` | **typed** (this PR) |
| Sync/async ops | `handle_call` / `handle_cast` | `Action.modal` | typed |
| Wire endpoints | Phoenix channels / controllers | `ActionDef` (wire surface) | typed; runtime via Sprint 4.5 |
| Scheduling | Quantum / cron | `Action{Cron, Scheduled}` | typed |
| Transactions | `Ecto.Multi` | `Action{Atomic}` | typed |
| Actor topology | `swarm` / `libcluster` / `libring` | `LokalSpec` (actor/tenant) | typed |
| Backpressure | `sbroker` / `pobox` | bounded mailbox + `Saturated` | typed (NEW-side) |
| Consensus | `rafted_value` | `openraft` (Phase H+I) | typed; runtime deferred |
| Metrics/audit | `exometer_influxdb` | Lance version log (version-as-TSDB) | typed (NEW-side) |
| Entity model | OGIT (BGFS/Auth/Tickets/OSINT/…) | `Class` under `ogit:` / `ogit-<app>:` | typed |

No row is "discovered at cutover": every OLD-stack concept lands on an existing
OGAR type or a term added here.

## 4. Identity / prefix

OLD HIRO entities map under the OGIT prefix (`ogit:` / `ogit-<app>:`) — OGIT *is*
the old stack's ontology, so no new top-level prefix is needed. Elixir *source*
shapes (Ecto/GenServer/`gen_statem`) are captured under `ogar-extensions/elixir/`
when per-language extension structs are needed (follow-up; the base `Class` /
`Action` carry the common shape today). Identity grammar is unchanged
(`prefix/Name`, `@vN`, `tenant.` — the stable string the `NiblePath` consumes).

## 5. Open / follow-up (not blocking the prefetch)

- `ogar-extensions/elixir` crate for the few Elixir-only structs (supervision
  tree shape, `gen_statem` data-term) — only if a producer needs more than the
  base `Class`/`Action` carries.
- `ElixirAdapter` / `HiroAdapter` HHTL leaves (the OLD-wire ↔ canonical table) —
  pairs with Sprint 4.5 `unmap`.
- The producer that walks real Elixir AST (`ogar-from-elixir`) — Sprint 4+.

## 6. Cross-references

- `OGAR-AST-CONTRACT.md` §3 (lowering: `State=ActionState`), §6 (the three terms
  this PR adds: `ogar:onEnter`, `ogar:guardFailurePolicy=Postponable`, `ogar:StateTimeout`)
- `ADAPTERS-AND-ACTORS.md` §3 (Action / SPO+TeKaMoLo / the actor-as-resolved-sentence)
- `vocab/ogar.ttl` (the three terms + `ogar:Elixir`, added in this PR)
- Runtime: `ractor_actors::state_machine` (the StateMachine shim the lifecycle lowers onto);
  `lance-graph-callcenter` `ExternalMembrane` (the sole-writer commit seam)
- `CROSS_SESSION_COORDINATION.md` (the authoritative binding record, runtime session)
