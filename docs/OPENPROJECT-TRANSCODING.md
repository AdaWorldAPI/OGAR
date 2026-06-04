# OpenProject Transcoding — Rails AR + concerns under `ogit-op::`

> **Purpose.** Expand OGAR against OpenProject (Rails 8 / Ruby 3.4 / ~1M LOC),
> the canonical attachment for the `ogit-op::` prefix the contract already
> uses in its examples. OpenProject is the production-grade Rails AR codebase
> we calibrate against — concerns-heavy, polymorphic associations, STI,
> `acts_as_*` patterns, lifecycle callbacks, paper-trail journaling, and
> data-driven FSMs (`Workflow`). If OGAR's `Class` / `ActionDef` /
> `ActionInvocation` carry all of this with zero core changes (modulo the
> already-shipped `Language::Ruby` variant), the substrate is *production-Rails
> sound*, not toy-Rails sound.
>
> **Position vs Odoo / Chess.** Same kind of work as `ODOO-TRANSCODING.md`
> (ERP/Python) and `CHESS-TRANSCODING.md` (closed/formal). OpenProject is the
> third axis: **open / messy / battle-tested**. Together, the three pin
> universality from three independent directions.
>
> **Grounded in real paths:** `/openproject/app/models/work_package.rb`,
> `/openproject/app/models/status.rb`,
> `/openproject/app/models/work_package/status_transitions.rb`,
> `/openproject/app/models/work_package/{validations,scheduling_rules,hooks,journalized}.rb`.
> Cited line ranges below.
>
> **Grounded in contract:** `OGAR-AST-CONTRACT.md` (`State=ActionState`,
> `Event=ActionDef`, `Context=ActionInvocation`), `ADAPTERS-AND-ACTORS.md` (SPO
> + TeKaMoLo), `IDENTITY-MAPPING.md` (prefix).
>
> Status: **CARVED v0** (2026-06-04).

## 0. Trichotomy for Rails AR — entangled-but-decomposable

In open domains the Semantik/Syntax/Pragmatik trichotomy is entangled. Rails
is the canonical example — Ruby's metaprogramming lets a single line
(`belongs_to :project`) carry vocabulary, syntax, and pragmatics
simultaneously. OpenProject's `WorkPackage` model
(`/openproject/app/models/work_package.rb`) is 200+ lines of this pattern.

The good news: the entanglement is **structured** — concerns, `acts_as_*`
macros, and AR's own lifecycle callbacks each carve a recognizable slice. A
producer can decompose them:

| OGAR layer | Sign relation | Rails AR | Carrier |
|---|---|---|---|
| **Semantik** | sign ↔ object | classes (`WorkPackage`), columns (via schema), `belongs_to`/`has_many` associations, enums (via `Status.is_closed?`), STI types | `Class` / `Association` / `Attribute` / `EnumDecl` |
| **Syntax** | sign ↔ sign | Ruby AR DSL (`belongs_to`, `validates`, `before_save`, `scope`), `acts_as_*` macros, concerns (modules in `app/models/{model}/`) | the `Adapter` HHTL — Ruby AST via lib-ruby-parser (the spec's `ogar-from-ruby` producer) |
| **Pragmatik** | sign ↔ interpreter | lifecycle order (validation → before_save → save → after_save → after_commit), transaction scope, scope-as-guard, polymorphic dispatch, current_user / tenant, paper-trail provenance | `ActionInvocation` SPO + TeKaMoLo + lifecycle |

The producer's job is to *unfold* the metaprogramming-collapsed source into
the three layers — that's the whole point of `ADAPTERS-AND-ACTORS.md §5.2`
(behavior producer extension for Sprint 3+).

## 1. The core carries OpenProject via `ogit-op::` — zero core changes

The `ogit-op::` prefix already appears in the contract's examples
(`ogit-op::WorkPackage`, `OpenProject before_save :touch_parent`).
`Language::Ruby` is already a variant in `ogar-vocab`
(`crates/ogar-vocab/src/lib.rs::Language`). So:

**Adding OpenProject = a producer (`ogar-from-ruby`) + a short TTL
ontology** mapping the AR + `acts_as_*` patterns. Not touching: the
state-machine crate, the codegen, the membrane, the IR structs, the
`Language` enum (already has the variant).

## 2. Structural arm — Rails AR → `Class`

The producer walks each `app/models/*.rb` file via lib-ruby-parser and emits
one `Class` per AR class. For `WorkPackage`:

| Rails AR (with real line refs) | OGAR mapping |
|---|---|
| `class WorkPackage < ApplicationRecord` (`work_package.rb:31`) | `Class { identity: "ogit-op::WorkPackage", language: Ruby, parent: Some("ogit-op::ApplicationRecord") }` |
| `belongs_to :project` (`work_package.rb:54`) | `Association { kind: BelongsTo, name: "project", target_class: "ogit-op::Project" }` |
| `belongs_to :assigned_to, class_name: "Principal", optional: true` (`work_package.rb:58`) | `Association { kind: BelongsTo, name: "assigned_to", target_class: "ogit-op::Principal", required: false }` |
| `has_many :time_entries, dependent: :delete_all, as: :entity` (`work_package.rb:65`) | `Association { kind: HasMany, name: "time_entries", target_class: "ogit-op::TimeEntry", polymorphic_as: "entity", on_delete: Delete }` |
| `has_and_belongs_to_many :changesets` (`work_package.rb:69`) | `Association { kind: HasAndBelongsToMany, name: "changesets", target_class: "ogit-op::Changeset" }` |
| `has_many :storages, through: :project` (`work_package.rb:67`) | `Association { kind: HasMany, name: "storages", through: "project" }` |
| `include WorkPackage::Validations` (`work_package.rb:33`) | `mixins: ["ogit-op::WorkPackage::Validations"]` (concern as separate `Class` under the prefix) |
| `include WorkPackage::StatusTransitions` (`work_package.rb:35`) | `mixins: [...]` — the predicates `reopened?` / `closing?` become methods on the included module's `Class` |
| `acts_as_watchable` (`work_package.rb:142`) | `decorator: "acts_as_watchable"` on the `Class` — marker the consumer (codegen) reads to enable watcher emit |
| `acts_as_customizable` / `acts_as_searchable` / `acts_as_attachable` / `has_paper_trail` (`work_package.rb:152-184`) | each → `decorator` entry on the `Class`; `has_paper_trail` is the *key* one — it maps directly to Lance versions (see §4) |
| `has_closure_tree` (`work_package.rb:181`) | `decorator: "has_closure_tree"` — implies a self-referential `Association` for the tree |
| validators (`Status:41-48`) | `Validation { target: "name", rule: "presence + uniqueness + length:256" }` etc. |
| `acts_as_list` (`Status:35`) | `decorator: "acts_as_list"` — implies an integer `position` `Attribute` |

For `Status` (`/openproject/app/models/status.rb`):

```
Class {
  identity: "ogit-op::Status",
  language: Ruby,
  parent: Some("ogit-op::ApplicationRecord"),
  associations: [
    Association { kind: HasMany, name: "workflows", target_class: "ogit-op::Workflow", foreign_key: "old_status_id" },
    Association { kind: BelongsTo, name: "color", target_class: "ogit-op::Color" },
  ],
  validations: [
    Validation { target: "name", rule: "presence + uniqueness:case_insensitive + length:max=256" },
    Validation { target: "default_done_ratio", rule: "inclusion: 0..100" },
    Validation { target: "is_readonly", rule: "default_status_must_not_be_readonly" },
  ],
  scopes: [ Scope { name: "visible", arity: 1 } ],
  decorators: ["acts_as_list"],
}
```

## 3. Behavioral arm — callbacks / scopes → `ActionDef`

Each AR callback / `acts_as_*` callback / business method becomes an
`ActionDef` per the SPO + TeKaMoLo carve-out:

| Rails AR | `ActionDef` projection |
|---|---|
| `before_save :close_duplicates` (`work_package.rb:146`) | `predicate="close_duplicates"`, `default_subject=Cascade`, `default_temporal=Immediate`, `kausal=LifecycleTrigger{event:"save"}`, `default_modal=Atomic` |
| `before_save :update_done_ratio_from_status` (`work_package.rb:146`) | same shape; `on_enter = Some(EnterEffect { field: "done_ratio", to_value: <derived> })` for the structural transition |
| `before_create :default_assign` (`work_package.rb:147`) | `kausal=LifecycleTrigger{event:"create"}`, `default_modal=Atomic` |
| `around_destroy :save_agenda_item_journals, prepend: true, if: -> { … }` (`work_package.rb:150`) | `kausal=LifecycleTrigger{event:"destroy"}` + conditional guard → `KausalSpec::StateGuard` |
| `after_save :unmark_old_default_value, if: :is_default?` (`status.rb:50`) | `default_temporal=Immediate` (after_save runs **inside** the surrounding transaction; raising from it rolls back the save), `kausal=StateGuard{field:"is_default", value:true}` |
| `after_commit :notify_subscribers` (Rails pattern) | `default_temporal=OnCommit` (runs **after** the transaction commits; cannot roll back the save). This is the only AR callback that maps to `OnCommit` — the `after_*` family without `_commit` all run in-transaction. |
| `before_destroy :check_integrity` (`status.rb:33`) | `kausal=LifecycleTrigger{event:"destroy"}`; raises → `guard_failure_policy=Reject` → `Pending→Failed` |
| `scope :visible, ->(user) { allowed_to(user, :view_work_packages) }` (`work_package.rb:85`) | not an `ActionDef` — scopes are query rewrites → `Scope` on the `Class` |
| `validates :name, presence: true, ...` (`status.rb:41`) | `Validation`; if it fails, the producer emits an `ActionDef` for the failing-save path (`Pending→Failed`) |
| `WorkPackage::StatusTransitions#closing?` (`work_package/status_transitions.rb:45`) | predicate derived from `status_id_changed?` + `Status.is_closed?` — exactly the `KausalSpec::StateGuard` shape (`{field: "status.is_closed", value: true after change}`) |
| `Workflow` model (data-driven FSM) | one `ActionDef` per `(old_status, new_status)` row in the `workflows` table; `kausal=StateGuard{field:"status_id", value:old}`, `on_enter=EnterEffect{field:"status_id", to_value:new}` |

The data-driven FSM is interesting: **OpenProject's transitions aren't
hard-coded — they're rows in a `workflows` table.** So an `ogar-from-ruby`
producer that walks the *code* misses them; the producer needs a *database
hydrator* step (read `Workflow.all`) to emit the runtime-shaped ActionDefs.
This is the same pattern as `lance-graph-ontology`'s TTL hydrators
(`LANCE-GRAPH-INTEGRATION.md`), now applied to AR seed data.

## 4. Lifecycle binding — a `WorkPackage` save end-to-end

A single `WorkPackage#save` with a status change as it flows through the
contract:

```text
ActionInvocation {
  identity:    "ogit-op::WorkPackage::PROJ-42::invocation::<ulid>",
  realizes:    "ogit-op::WorkPackage::action::save",
  state:       Pending,
  subject:     ActionSubject::User,            // current_user
  object_instance: "ogit-op::WorkPackage::PROJ-42",
  lokal:       LokalSpec { actor: "user::<id>", tenant: <none>, company: <none> },
  idempotency_key: Some("<request-id>"),       // §14 OLD↔NEW correlation handle
  trace_id:    Some("<request-trace>"),
  emitted_at_millis: Some(<now>),              // Decision-#4 HLC slot
  ...
}
              │
              │  StateMachine::on_event(Pending, ActionDef{save}, ctx) →
              │
              │  pre-save chain (cascade ActionInvocations):
              │  ├── validate (run_validations!)
              │  │     └── if any validation fails:
              │  │         ├── guard_failure_policy=Reject → Goto(Failed)
              │  │         └── failure_reason = errors.full_messages
              │  ├── before_save callbacks (close_duplicates, update_done_ratio_from_status)
              │  │     └── each is its own ActionDef; subject=Cascade
              │  └── if all green: Goto(Committed)
              ▼
StateMachine::is_commit(Committed) == true
              │
              ▼
CommitHook::on_commit(Pending, Committed, ctx) -> Result<(), ActorProcessingErr>:
   1. apply on_enter (typed EnterEffect, PR #13):
        ctx.object_instance.set_field("status_id", new_status_id);
        ctx.object_instance.set_field("updated_at", now);
   2. row = CognitiveEventRow {
          subject: object_instance, predicate: "save",
          object: serialize(work_package_attrs_after),
          metadata: { before_attrs, after_attrs, current_user, journal_entry },
      };
   3. self.membrane.commit_event(row);   // lance-graph PR #467 — returns new Lance version
   Ok(())

In-transaction cascade (after_save callbacks fire AS Cascade ActionInvocations, default_temporal=Immediate;
their failure rolls back the parent save):
   - unmark_old_default_value (Status side, on its own Pending→Committed lifecycle)
   - paper-trail entry (has_paper_trail) — already absorbed into the Lance version log; the
     PaperTrail::Version row becomes a derived projection of the Lance version, not a
     separate write

Post-commit cascade (only after_commit callbacks fire here, default_temporal=OnCommit;
side-effects are observable to the world but no longer rollback-safe):
   - notification side-effects (e.g. Notification model invocations triggered by after_commit)
   - external integrations (webhooks, email enqueue) — anything that must not run during a
     transaction that might still abort
```

Key observation: **`has_paper_trail` + Lance versions are a degenerate
case** — both are "append a version record on every change," so a faithful
binding eliminates the redundant `versions` table and reads paper-trail
queries off the Lance version log directly. That's a real
substrate-consolidation win: 1-of-N tables collapses to 0 tables when the
substrate already does versioning.

## 5. Edge cases — Rails-specific patterns and their carriers

| Rails pattern | Carrier strategy |
|---|---|
| **STI** (`type` column discriminator, e.g. `Principal` → `User`/`Group`/`PlaceholderUser`) | parent `Class` with `attributes: [type: STIDiscriminator]`; subclasses `Class { parent: Some("ogit-op::Principal") }`; routing by `parent` chain |
| **Polymorphic associations** (`has_many :time_entries, as: :entity`) | `Association { polymorphic_as: "entity" }`; the inverse on `TimeEntry` carries `polymorphic_for: ["WorkPackage", "Project", …]` (the set of carriers) |
| **`through:` associations** (`has_many :storages, through: :project`) | `Association { kind: HasMany, through: "project" }` — codegen resolves via two-hop traversal |
| **`acts_as_*` macros** (`acts_as_watchable`, `acts_as_journalized`, `acts_as_customizable`) | `decorators: Vec<String>` on the `Class`; codegen consumers (callcenter) read these markers and emit the corresponding behavior (watcher actor, journal stream, custom-field columns) |
| **`has_paper_trail`** | dropped as a separate `ActionDef` — the Lance version log IS the paper-trail (§4 observation) |
| **`Concern`s** (modules in `app/models/work_package/*.rb`) | one `Class` per concern under the parent's prefix (e.g. `ogit-op::WorkPackage::StatusTransitions`); the parent's `mixins` field references them |
| **`current_user` / `Current.user`** (RequestStore) | not in `Class`/`Attribute` — emitted on `ActionInvocation.subject`/`lokal` per fire (the runtime context) |
| **Transactional scope** (`ActiveRecord::Base.transaction`) | `default_modal=Atomic`; the codegen wraps `on_commit` in `LanceMembrane::commit_event` (already atomic per gate-1) |
| **Multi-tenancy** (none in OP today, but pattern is `ActsAsTenant` if added) | `LokalSpec.tenant` — already carried |
| **Background jobs** (Good Job — `app/workers/`) | each job class → `Class { decorators: ["good_job_worker"] }`; `#perform` → `ActionDef { default_temporal=Deferred, default_subject=System }` |

## 6. §14 ground-truth — AR save-lifecycle replay

The chess oracle was `shakmaty::play()` — a pure function. OP's oracle is
the *AR save lifecycle replay*, which is impure but reproducible:

```
record (Rails OP):
  for each (controller_action, params) tuple:
    snapshot_before = serialize_all_touched_rows
    run_in_transaction { controller_action.call(params) }
    snapshot_after  = serialize_all_touched_rows
    emit (controller_action, params, snapshot_before, snapshot_after)

replay (OGAR):
  for each tuple in tape:
    ctx = ActionInvocation { object_instance: snapshot_before, ... };
    codegen_actor.fire(ActionDef{controller_action});
    on_event → run guards (validations) → Goto(Committed | Failed)
    on_commit → apply EnterEffect chain → membrane.commit_event(row)
    snapshot_after_ogar = read_back_object_instance(new_version);
    ASSERT snapshot_after_ogar ≈ snapshot_after
      (provenance-normalized: trace_id, updated_at, journal_id stripped;
       idempotency_key correlates rows)
```

This is harder than chess (Rails callbacks can mutate via outside-the-model
side-effects — e.g. `Notification.create!` from within an after_save), so
the provenance-normalization set is larger. But the contract's §14 verdict
bins (PASS / DIVERGENT-RECONCILABLE / DIVERGENT-FAULTY / INDETERMINATE)
still apply; "DIVERGENT-RECONCILABLE" is the right bucket for "we expected
the notification side-effect, AR did it, OGAR will need a cascade producer
to do it."

This makes OpenProject the **progress meter** for substrate completeness:
the diff between PASS-rate and total = the unmodeled-AR-pattern backlog.

## 7. Producer shape — `ogar-from-ruby`

Mirrors `ogar-from-ruff` (Python) and the planned `ogar-from-shakmaty` /
`ogar-from-elixir`. Walks Ruby AST via lib-ruby-parser (per
`ARCHITECTURE.md` Universal AST). Two emit-modes:

```rust
// crates/ogar-from-ruby/ (proposed; Sprint N)
pub fn emit_classes(rails_app_root: &Path) -> Result<Vec<Class>>;
//   walks app/models/**/*.rb; recognises belongs_to / has_many / validates /
//   acts_as_* / include; emits one Class per AR class + one per concern.

pub fn emit_action_defs(rails_app_root: &Path) -> Result<Vec<ActionDef>>;
//   walks the same AST + extracts callbacks (before_save / after_commit /
//   around_destroy …) and business methods marked with annotations.

// Database hydrator for data-driven FSMs (e.g. OpenProject Workflow rows)
pub fn emit_workflow_action_defs(db_url: &str) -> Result<Vec<ActionDef>>;
//   reads the `workflows` table → ActionDef per (old_status, new_status) row.
```

The hydrator step is the **OP-specific delta from the other producers**:
Rails apps lean on seed data for FSMs (Workflow, Roles, Permissions); the
producer must read both code AND data to be complete.

## 8. What this proves about universality

| Claim from the contract | OpenProject evidence |
|---|---|
| **Producers are domain-bound; the core is not.** | `ogar-from-ruby` walks Ruby AR + a database hydrator. The state-machine crate, codegen, membrane, IR structs are untouched. Only addition: per-AR-pattern carriers on existing types (`decorators: Vec<String>`, `mixins: Vec<Identity>`, `Association.polymorphic_as`). |
| **`State = ActionState` lifecycle is universal** | A `WorkPackage#save` is `Pending → Committed/Failed` exactly like a chess ply or an Odoo `action_confirm`. The domain state (`status_id`, etc.) rides as `EnterEffect` on the typed `ActionDef.on_enter` (PR #13) — confirmed cross-domain. |
| **The §6 statem terms are sufficient** | `Postpone` (premove ↔ optimistic AR save-with-pending-deps); `StateTimeout` (chess clock ↔ Rails request timeout); `on_enter` (Move ↔ EnterEffect for status_id); `guardFailurePolicy=Reject` (illegal move ↔ validation failure). All naturally exercised. |
| **`has_paper_trail` is a duplicate of Lance versions** — substrate consolidation | The Lance version log subsumes the AR `versions` table. One less table on disk; same query power. |
| **OpenProject is the production-grade calibration** | Closed-formal (chess) + open-orderly (Odoo) + open-messy (OpenProject = concerns + acts_as_* + STI + polymorphism + data-driven FSMs + paper-trail) = three independent axes of universality. Pass on all three and the substrate is *production-Rails sound*. |

## 9. Open / follow-up

- `ogar-from-ruby` crate scaffold — Sprint N (after `ogar-from-elixir`).
- Database-hydrator pattern (Workflow, Roles, Permissions, custom_actions
  table) generalized into `ogar-hydrator-postgres` — reusable across Rails
  apps.
- A small executable demo: `ogar-from-ruby --emit-classes
  /home/user/openproject/app/models > op_classes.ttl` — proves the producer
  walks a real Rails app of this size in reasonable time.
- The chess + OP + Odoo trichotomy as a *substrate maturity scorecard*:
  PASS-rate across the three domains is the substrate's universality
  metric.

## 10. Producer ecosystem — two-arm pattern + nexgen convergence

The OpenProject (and broader Rails-AR) producer space already has substantial
in-flight work across `AdaWorldAPI/ruff`, `AdaWorldAPI/openproject-nexgen-rs`,
and `AdaWorldAPI/surrealdb`. Naming the arms here so future producers don't
collide.

### 10.1 Two arms from one AST parse

There are **two complementary IR shapes** a Rails-AR producer should fill,
each with a distinct downstream consumer:

| Arm | IR | What it captures | Downstream |
|---|---|---|---|
| **Narrow / SPO** | `ruff_spo_triplet::ModelGraph` (`Model` / `Field` / `Function`) | data-dependency edges — `depends_on`, `reads_field`, `traverses_relation`, `raises`, `emitted_by` (7-predicate closed vocab) | lance-graph SPO store via ndjson |
| **Wide / OGAR** | `ogar-vocab::Class` / `ActionDef` / `ActionInvocation` (this doc, §1–§5) | lifecycle contract — `State=ActionState` (Pending → Committed/Failed/Cancelled), typed `EnterEffect`, `KausalSpec` (StateGuard + Depends), TeKaMoLo, paper-trail-as-Lance-version | `lance-graph-ontology` via `MappingProposal` (boundary stub, Sprint-5b `lance-bind`) |

**One AST parse, both arms.** `lib-ruby-parser` walks `app/models/` once. The
same `class WorkPackage < ApplicationRecord` body fills:
- `RubyClass { name, body_source, associations }` for `ruff_ruby_spo`-style narrow extraction
- `ogar_vocab::Class { …, associations, callbacks, computed_fields, validations, methods, attributes }` for OGAR-wide

Producers should be **named pairs** so the duality is explicit, not
accidental:

| Domain | Narrow SPO (scaffold) | Wide OGAR (planned/this doc) |
|---|---|---|
| Ruby AR / OpenProject | [`ruff_ruby_spo`](https://github.com/AdaWorldAPI/ruff/tree/main/crates/ruff_ruby_spo) — scaffold; `todo!()` stubs documenting the Rails constructs to read | `ogar-from-ruby` — planned crate, fills `Class` / `ActionDef` per §1–§5 |
| Python / Odoo | [`ruff_python_dto_check`](https://github.com/AdaWorldAPI/ruff/tree/main/crates/ruff_python_dto_check) — fully wired (extractors / codegen / preflight / matcher) | `ogar-from-ruff` / `ogar-python` (per `ARCHITECTURE.md` Universal AST diagram) |
| Elixir (HIRO/Bardioc) | `ruff_elixir_spo` — future, mirroring `ruff_ruby_spo` | `ogar-from-elixir` — next crate, per `ELIXIR-HIRO-PREFETCH.md §2` |

The shared core is [`ruff_spo_triplet`](https://github.com/AdaWorldAPI/ruff/tree/main/crates/ruff_spo_triplet)
(zero-dep, serde-only). The two arms are **complementary, not competing**:
SPO answers *"what depends on what"* (the data-dependency DAG that feeds
Rubicon's `KausalSpec::Depends` guard variant); OGAR answers *"when does this
fire, what guards, what state, what commits"* (the lifecycle FSM). Both flow
back through `lance-graph-planner::temporal::classify` to deinterlace into a
single causally-coherent SoA — see §10.3.

### 10.2 nexgen `op-surreal-ast` ↔ OGAR `Class` convergence

[`AdaWorldAPI/openproject-nexgen-rs`](https://github.com/AdaWorldAPI/openproject-nexgen-rs)
already ships in-flight OpenProject-specific work that converges with OGAR's
domain-agnostic IR:

| nexgen crate | Role | Convergence with OGAR |
|---|---|---|
| `op-surreal-ast` (C16a sprint) | OP-specific mirror of `surrealdb-core::catalog` layout — typed structs for OpenProject schema elements | **Special case of `Class` → `catalog::TableDefinition`.** C16c sprint plans `From<op_surreal_ast::*> for catalog::*` impls; once those land, `op-surreal-ast` either drops the mirror or keeps it as a fast in-repo path. |
| `op-codegen-projection` (C15 sprint) | renders OP schema elements as DDL via `op-surreal-ast` | When `ogar-adapter-surrealql` lands (queue item #3 below), `op-codegen-projection` is a special case of the general OGAR `Class` → `TableDefinition::new_for_ddl().with_*()` → `ToSql` path. |
| `op-codegen-pipeline` / `op-codegen-bucket` (C9, C15 sprints) | extract-to-projection pipeline + bucketing | future `ogar-from-ruby` plays this role for the OGAR-wide arm; the two pipelines coexist (narrow projection for nexgen's existing consumers, wide OGAR for substrate-b). |

`AdaWorldAPI/surrealdb` Sprint C16b's `TableDefinition::new_for_ddl(…).with_*(…)`
builders were designed specifically for *"external codegen tools that want to
build a typed `TableDefinition` representing a schema element, render it to
SurrealQL via `ToSql::to_sql()`, never touch the actual database"* (per the
op-codegen-bridge README). Both `op-codegen-projection` and the future
`ogar-adapter-surrealql` are exactly such tools — same builder API, different
IR sources.

**No collision.** nexgen owns its OP-specific in-repo path; OGAR owns the
domain-agnostic generalization. They meet at `surrealdb-core::catalog`, not
at the schema-source level.

### 10.3 `knowable_from` — the meet-point with `lance-graph-planner::temporal`

Per the temporal-deinterlacing design in flight on the runtime session, the
substrate has four interlaced field-clocks that need merging into one
causally-coherent SoA. Reading naïvely produces a *combed* frame — fields
from different writers torn against each other:

| Frame | Clock | Source |
|---|---|---|
| **lance** (storage) | per-writer monotonic `lance_version` | every committed write |
| **surrealql** (schema) | `knowable_from: LanceVersion` — when a class/field became defined | **`ogar-adapter-surrealql`** (queue item #3) — stamps at DDL registration time |
| **ractor** (awareness) | per-actor `V_ref` reading-horizon | each `ClassActor` instance |
| **thinking** (cognition) | Markov ±5 `CognitiveEventRow` trajectory | `cognitive-shader-driver` |

HLC `(server_id, lance_version, hlc_tick)` is the deinterlace key;
`classify(row_version, knowable_from, v_ref) → {CONTEMPORARY | ANACHRONISTIC | SPOILER}`
is the per-row deinterlace decision. A second `DependsClosure`-input axis
plugs in for data-causality once SPO producers emit real `depends_on` edges.

**Durable interface pin (cross-session, authoritative):**

> The SurrealQL frame's **`knowable_from`** is sourced by
> **`ogar-adapter-surrealql`** (OGAR session — queue item #3 below) and
> consumed by **`lance-graph-planner::temporal::classify`** (runtime
> session). The producer-side stamps `knowable_from` at DDL registration
> time; the consumer-side reads it as one of three inputs to
> `classify(row_version, knowable_from, v_ref)`. **Nowhere else in the
> substrate owns either side of this seam.**

This pin should mirror to `bardioc/CROSS_SESSION_COORDINATION.md` (the
runtime-session-owned coord doc) for full cross-session durability. Until
that mirror lands, this §10.3 is the authoritative OGAR-side source.

### 10.4 Producer queue ordering

Driven by what unblocks the most downstream work:

1. **(this PR)** — OpenProject companion: name the arms, pin the meet-point.
2. **`ogar-from-elixir` scaffold** — mirrors `ruff_ruby_spo`'s
   scaffold-with-`todo!()`-stubs pattern but OGAR-shaped; depends on
   `ogar-vocab` only; `todo!()` stubs document Ecto / `gen_statem` / Phoenix
   per `ELIXIR-HIRO-PREFETCH.md §2.2`; locked-shape test against hand-built
   fixtures.
3. **`ogar-adapter-surrealql`** — the SurrealQL bridge per the C16b
   builders + `surrealdb-ast`/`surrealdb-parser` (no longer "deferred until
   crates.io"). `emit` thin via `TableDefinition::new_for_ddl` → `ToSql`;
   `unmap` is the substantive work (AST walk → `Class`); roundtrip
   proptest. **Sources `knowable_from` for §10.3.**
4. **`lance-bind` boundary** — once protoc / cross-repo build holds up.

## 11. Cross-references

- `OGAR-AST-CONTRACT.md` — the typed surface OP lowers onto (`State=ActionState`, `Event=ActionDef`, `Context=ActionInvocation`, typed `on_enter: EnterEffect` from PR #13).
- `ADAPTERS-AND-ACTORS.md` §3 — Action / SPO+TeKaMoLo / the actor-as-resolved-sentence.
- `ELIXIR-HIRO-PREFETCH.md` — the prefetch-the-types-now pattern (mirror application for the OLD HIRO stack).
- `CHESS-TRANSCODING.md` — the calibration target on the *closed-formal* axis.
- `ODOO-TRANSCODING.md` — the original transcoding precedent on the *ERP-ordered* axis; OP completes the third axis (open-messy production Rails).
- `vocab/ogar.ttl` — `Language::Ruby` (already present); `EnterEffect` (PR #13).
- Upstream: [openproject/openproject](https://github.com/opf/openproject) — the open-source Rails 8 app modeled here.
- Runtime: `ractor_actors::state_machine` (`feat/state-machine-actor` @ `38a71a4`); `LanceMembrane::commit_event` (lance-graph PR #467, merged).
- Producers (narrow SPO arm — `AdaWorldAPI/ruff`):
  [`ruff_spo_triplet`](https://github.com/AdaWorldAPI/ruff/tree/main/crates/ruff_spo_triplet) (shared zero-dep core),
  [`ruff_ruby_spo`](https://github.com/AdaWorldAPI/ruff/tree/main/crates/ruff_ruby_spo) (OpenProject scaffold),
  [`ruff_python_dto_check`](https://github.com/AdaWorldAPI/ruff/tree/main/crates/ruff_python_dto_check) (Python frontend, fully wired).
- nexgen ([`AdaWorldAPI/openproject-nexgen-rs`](https://github.com/AdaWorldAPI/openproject-nexgen-rs)):
  `op-surreal-ast` (C16a mirror of `catalog::*` layout),
  `op-codegen-projection` (DDL renderer),
  `op-codegen-pipeline` / `op-codegen-bucket` (extract-to-projection pipeline).
- SurrealDB fork ([`AdaWorldAPI/surrealdb`](https://github.com/AdaWorldAPI/surrealdb)):
  Sprint C16b `TableDefinition::new_for_ddl` + chainable `with_*` builders
  in `surrealdb/core/src/catalog/{table,schema/field,schema/index}.rs`;
  `surrealdb-ast` + `surrealdb-parser` as first-class workspace crates.
- Temporal deinterlacing: `lance-graph-planner::temporal::classify`
  (runtime session, in flight) — consumes `knowable_from` per §10.3.
