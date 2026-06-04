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

## 10. Cross-references

- `OGAR-AST-CONTRACT.md` — the typed surface OP lowers onto (`State=ActionState`, `Event=ActionDef`, `Context=ActionInvocation`, typed `on_enter: EnterEffect` from PR #13).
- `ADAPTERS-AND-ACTORS.md` §3 — Action / SPO+TeKaMoLo / the actor-as-resolved-sentence.
- `ELIXIR-HIRO-PREFETCH.md` — the prefetch-the-types-now pattern (mirror application for the OLD HIRO stack).
- `CHESS-TRANSCODING.md` — the calibration target on the *closed-formal* axis.
- `ODOO-TRANSCODING.md` — the original transcoding precedent on the *ERP-ordered* axis; OP completes the third axis (open-messy production Rails).
- `vocab/ogar.ttl` — `Language::Ruby` (already present); `EnterEffect` (PR #13).
- Upstream: [openproject/openproject](https://github.com/opf/openproject) — the open-source Rails 8 app modeled here.
- Runtime: `ractor_actors::state_machine` (`feat/state-machine-actor` @ `38a71a4`); `LanceMembrane::commit_event` (lance-graph PR #467, merged).
