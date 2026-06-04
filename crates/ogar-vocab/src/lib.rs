//! `ogar-vocab` — the canonical Rust types for the OGAR vocabulary.
//!
//! OGAR is the language-independent Active Record pattern as a graph
//! ontology. These types are the **IR** that producers (Ruby AR via
//! `ruff_ruby_spo`, Python Odoo via `ogar-python`, SQL DDL via
//! `ogar-sql-ddl`, …) emit and consumers (lance-graph triple loader,
//! `ogar-to-postgres`, `ogar-to-surrealql`, …) read.
//!
//! See [`Class`] for the entry-point shape. The types deliberately mirror
//! the C17a–c stable shape in `ruff_ruby_spo` so the existing producer can
//! be lifted in-place; the only change is stripping the Ruby-specific
//! framing (`body_source` becomes opaque `source` with a `language`
//! discriminant on the parent class).
//!
//! # Layer position
//!
//! ```text
//!   source AST  ──▶  ogar-vocab::Class  ──▶  ogar-ontology  ──▶  lance-graph triples
//!   (Ruby/Py/  )      (this crate)            (prefix         (Arrow/Lance SoA)
//!    SQL/TS    )                              routing)
//! ```

#![warn(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Source language hint — discriminates the producer for traceability
/// and for emitter dispatch on Ruby/Python-specific extension shapes
/// (e.g. Odoo `ComputedField`). Not a hard schema discriminator: a class
/// is fully described by the canonical fields below regardless of
/// `language`.
///
/// **Vocabulary versioning:** `#[non_exhaustive]` so adding a new
/// language (e.g. `Elixir`) is non-breaking. Match expressions in
/// consumer code must include a `_ =>` arm. This applies to every
/// `pub enum` / `pub struct` in this module: the OGAR vocabulary is
/// expected to evolve over time, and every base type is forward-
/// compatible-by-construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum Language {
    /// Ruby ActiveRecord (`class Foo < ApplicationRecord`).
    Ruby,
    /// Python — covers Django ORM and Odoo `models.Model`.
    Python,
    /// SQL DDL (`CREATE TABLE …`).
    Sql,
    /// TypeScript — covers Prisma, TypeORM, Drizzle.
    TypeScript,
    /// SurrealQL DDL (`DEFINE TABLE …`).
    SurrealQl,
    /// Elixir — Ecto schemas (`schema "t" do …`), Phoenix contexts, and
    /// OTP behaviours (`GenServer` / `gen_statem`). **First-class for
    /// migration**: the OLD HIRO/Bardioc stack is Elixir, so it is the
    /// source of every byte of migration debt and the bridge to the old
    /// adapters. `gen_statem` lifecycles lower onto the same `Action`
    /// state machine as every other producer (see `docs/ELIXIR-HIRO-PREFETCH.md`).
    Elixir,
    /// Unknown or hand-authored.
    Unknown,
}

/// The canonical OGAR class — a single AR-shaped record-class declaration
/// lifted from its source language into the language-independent vocabulary.
///
/// Fields are grouped by C17 sprint of origin in the `ruff_ruby_spo` lift:
/// - **C17a** core: [`name`](Self::name), [`parent`](Self::parent),
///   [`associations`](Self::associations).
/// - **C17b** schema-extensions: [`enums`](Self::enums),
///   [`store_accessors`](Self::store_accessors),
///   [`attributes`](Self::attributes),
///   [`mixins`](Self::mixins), [`table_name`](Self::table_name),
///   [`inheritance_column_disabled`](Self::inheritance_column_disabled).
/// - **C17c** runtime-shape: [`ignored_columns`](Self::ignored_columns),
///   [`scopes`](Self::scopes),
///   [`scope_predeclarations`](Self::scope_predeclarations),
///   [`default_scope`](Self::default_scope), [`callbacks`](Self::callbacks).
///
/// Per-language extensions (Odoo `compute`, `_inherits` delegation,
/// workflow state machines) are not on this base type — they live in
/// `ogar-extensions/*` crates so the core IR stays canonical.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Class {
    /// Class name as written in the source. For dotted-name ORMs
    /// (Odoo `account.move`) the dots are preserved; the prefix-radix
    /// routing in `ogar-ontology` handles the dotted segments.
    pub name: String,
    /// Superclass name as written, when one is declared. Used by
    /// consumers to assemble single-table-inheritance hierarchies.
    pub parent: Option<String>,
    /// Source language of the producer that emitted this class.
    pub language: Language,
    /// `belongs_to` / `has_one` / `has_many` / `has_and_belongs_to_many`
    /// declarations in source order.
    pub associations: Vec<Association>,
    /// `include Mixin` / `_inherit = 'mixin.thread'` mixin paths in
    /// declaration order. Dotted names preserved verbatim.
    pub mixins: Vec<String>,
    /// `enum status: { ... }` / `fields.Selection([...])` enum-backed
    /// columns in declaration order.
    pub enums: Vec<EnumDecl>,
    /// `store_accessor :col, %i[a b c]` JSONB pseudo-field bundles in
    /// declaration order. Rails-only today; Python equivalents (Odoo
    /// `fields.Json` with derived properties) lift here too.
    pub store_accessors: Vec<StoreAccessor>,
    /// `attribute :name, :type` typed-attribute overrides in
    /// declaration order.
    pub attributes: Vec<Attribute>,
    /// `self.table_name = "..."` literal-string override. `None` when
    /// the consumer should infer the table name (the common case).
    pub table_name: Option<String>,
    /// `self.inheritance_column = :_type_disabled` was set. Signals
    /// the class deliberately opts out of STI dispatch even with
    /// subclasses present.
    pub inheritance_column_disabled: bool,
    /// `self.ignored_columns += [...]` runtime blacklist columns in
    /// source order across however many `+=` statements appear.
    pub ignored_columns: Vec<String>,
    /// `scope :name, -> { body }` definitions in source order.
    pub scopes: Vec<Scope>,
    /// `scopes :a, :b, :c` declarative-list scope-name predeclarations
    /// — a DSL form that pre-declares scope class-methods defined in
    /// mixins elsewhere.
    pub scope_predeclarations: Vec<String>,
    /// `default_scope -> { body }` global filter body, when present.
    pub default_scope: Option<String>,
    /// Lifecycle callback declarations in source order.
    pub callbacks: Vec<Callback>,
    /// Validation declarations in source order (`validates :col, ...`,
    /// `@api.constrains('col')`).
    pub validations: Vec<Validation>,

    // ─────────────────────────────────────────────────────────────
    // Odoo-shaped fields (also populated by Rails/Django where
    // sensible). See `docs/ODOO-TRANSCODING.md` §7.
    // ─────────────────────────────────────────────────────────────
    /// `_description = 'Sale Order'` (Odoo) — human-readable name.
    /// Rails has no direct equivalent (class comment usually).
    pub description: Option<String>,
    /// `_order = 'date desc, id'` (Odoo) — default record ordering.
    /// Distinct from `default_scope` (Rails) which is a full where
    /// clause; `record_order` is just the ORDER BY tail.
    pub record_order: Option<String>,
    /// `_rec_name = 'name'` (Odoo) — UI display field. Defaults to
    /// `'name'` if unset (Odoo convention).
    pub rec_name: Option<String>,
    /// `_check_company_auto = True` (Odoo) — auto multi-company
    /// check on FK targets.
    pub check_company_auto: Option<bool>,
    /// `_log_access = False` (Odoo) — skip create_uid / write_uid
    /// audit columns.
    pub log_access: Option<bool>,
    /// `_auto = False` (Odoo) — no auto CREATE TABLE (SQL view
    /// models like `account.invoice.report`).
    pub auto_create_table: Option<bool>,
    /// `_abstract = True` (Odoo) — base class, no table. Methods
    /// inherited but data not stored.
    pub abstract_model: bool,
    /// `_transient = True` (Odoo) — wizard/scratchpad model with
    /// vacuumed rows.
    pub transient: bool,
    /// `_register = False` (Odoo) — skip from registry (rare;
    /// usually only base classes).
    pub register: Option<bool>,
    /// Module name from `__manifest__.py` (`'sale'`, `'account'`).
    /// Required for Odoo classes (every class lives in one module);
    /// optional for Rails (engines / gems are the closest concept).
    /// Emitted as `ogar:declaredIn <module>` triple — see BO2 #3.
    pub declared_in_module: Option<String>,
    /// Source language major version (`"17.0"`, `"7.2"`, ...) for
    /// multi-version source compatibility. Reserved; v1 leaves `None`.
    pub source_version: Option<String>,
    /// Computed-field declarations (Odoo `compute=...` fields, also
    /// Rails / Django where producers can detect them). Lives in
    /// base vocab — see `docs/ODOO-TRANSCODING.md` §8.
    pub computed_fields: Vec<ComputedField>,
    /// CRUD overrides and other method declarations (Odoo
    /// `def create / write / unlink / copy` overrides, Rails
    /// `def self.method`, etc.). Distinct from `callbacks` which
    /// are declarative hooks.
    pub methods: Vec<MethodDecl>,
}

/// A computed-field declaration. Carries the field name, the compute
/// method's symbol, and the dependency list from `@api.depends`.
/// Universal across ORMs: Odoo `compute='_compute_x'` + `@api.depends`,
/// Django `cached_property`, Rails instance-method-derived attributes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct ComputedField {
    /// The field being computed.
    pub field: String,
    /// Compute method name (`"_compute_total"`).
    pub compute_method: String,
    /// Dependency paths from `@api.depends('partner_id',
    /// 'order_line.price_total')`. Empty if no `@api.depends`.
    pub depends: Vec<String>,
    /// `@api.depends_context('uid', 'company')` — env-context
    /// dependencies (Odoo only).
    pub depends_context: Vec<String>,
    /// `store=True` — store result in DB column. `False` recomputes
    /// on every read.
    pub stored: bool,
    /// `inverse='_inverse_total'` — write-direction helper
    /// (turning back from computed value to raw field assignments).
    pub inverse_method: Option<String>,
    /// `search='_search_total'` — search helper for filtering by
    /// computed value.
    pub search_method: Option<String>,
}

/// A method declaration: CRUD override (`def create`/`def write`/
/// `def unlink`/`def copy`), `@api.model` helper, or plain instance
/// method. Distinct from `Callback` which is a declarative hook.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct MethodDecl {
    /// Method name as written.
    pub name: String,
    /// The kind of method — distinguishes total overrides from
    /// declared hooks.
    pub kind: MethodKind,
    /// Method body verbatim. Consumers re-parse or emit opaque.
    pub body_source: String,
    /// Decorator names as written: `["api.depends", "api.constrains"]`.
    pub decorators: Vec<String>,
    /// Recordset binding semantics — does the method bind to a
    /// single record, a recordset, or class-level?
    pub semantics: RecordSemantics,
}

/// Method kind — distinguishes overrides from helpers from plain
/// methods. The producer determines kind from decorator + name
/// inspection; see `docs/ODOO-TRANSCODING.md` §13.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum MethodKind {
    /// `def create(self, vals_list)` — total override of an ORM
    /// CRUD method. Semantically distinct from Rails callbacks.
    CrudOverride,
    /// `@api.model def helper(self, ...)` — class-method-like.
    ApiModel,
    /// `@api.model_create_multi def create(self, vals_list)` —
    /// Odoo's bulk-create override.
    ApiModelCreateMulti,
    /// Plain instance method, no special semantics.
    Instance,
}

impl Default for MethodKind {
    fn default() -> Self {
        Self::Instance
    }
}

/// Recordset semantics — Odoo methods can bind to a record (single),
/// a recordset (the default for most methods), or be class-level
/// (`@api.model`). Captured for cross-language consumers that
/// project to per-record vs per-collection APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum RecordSemantics {
    /// Single-record context.
    Record,
    /// Recordset (Odoo default for most methods).
    Recordset,
    /// Class-level (`@api.model` or no `self`).
    ClassLevel,
}

impl Default for RecordSemantics {
    fn default() -> Self {
        Self::Recordset
    }
}

// ─────────────────────────────────────────────────────────────────────
// Sprint 3 — Action vocabulary with SPO + TeKaMoLo grammar
// (per docs/ADAPTERS-AND-ACTORS.md + brutal-review cycle 3 fixes)
//
// B1 fix: Action split into ActionDef (declaration) + ActionInvocation
// (per-context invocation). One ActionDef may have N invocations.
// B1 fix: KausalSpec is a proper sum type, not free-form opaque.
// B2 fix: Provenance fields (trace_id, parent_action_id, idempotency_key,
// emitted_at) carved into ActionInvocation.
// B2 fix: ActionState lifecycle (Pending / Committed / Failed) carved.
// ─────────────────────────────────────────────────────────────────────

/// An action declaration — the AST-extracted shape of a business
/// operation (a method-decorator combo, a callback declaration, a
/// workflow transition). One per source-level method/callback decl.
/// Invocations of this declaration become `ActionInvocation` triples.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct ActionDef {
    /// Stable identity for the action declaration (e.g.
    /// `ogit-erp/sale.order::action_def::action_confirm`).
    pub identity: String,
    /// Predicate name as written in source — `action_confirm`,
    /// `before_save`, etc.
    pub predicate: String,
    /// Object class — the OGAR-canonical class identity this action
    /// applies to (`ogit-erp/sale.order`).
    pub object_class: String,
    /// Default subject when not specified by an invocation.
    pub default_subject: ActionSubject,
    /// Default temporal annotation.
    pub default_temporal: TemporalSpec,
    /// Default modal annotation.
    pub default_modal: ModalSpec,
    /// Causal precondition — when None, action fires unconditionally
    /// at the right Te point. Sum type: real producers populate one
    /// of the typed variants below.
    pub kausal: Option<KausalSpec>,
    /// Method body verbatim (for projection emission).
    pub body_source: Option<String>,
    /// Decorator names that drove the extraction (Odoo `@api.depends`,
    /// Rails callback macro name).
    pub decorators: Vec<String>,

    // ── Rubicon statem carriers (OGAR-AST-CONTRACT §6) ──
    // The three semantics that don't survive Action-flattening; each lowers
    // onto `ractor_actors::state_machine` with `State = ActionState`.
    /// Entry effect fired on entering `Committed` (the Rubicon crossing) —
    /// typically the domain transition + side effects. Emitted as
    /// `ogar:onEnter`; lowers to `StateMachine::on_enter` / the `CommitHook`.
    /// Free-form today; a typed `EnterEffect` is a tracked follow-up.
    pub on_enter: Option<String>,
    /// Disposition when the Kausal `StateGuard` fails: `Postponable` (stay
    /// `Pending`, replay) vs `Reject` (`Pending → Failed`, the default).
    /// Emitted as `ogar:guardFailurePolicy`.
    pub guard_failure_policy: Option<GuardFailurePolicy>,
    /// Per-state SLA deadline on `Pending`, in milliseconds. Emitted as
    /// `ogar:stateTimeoutMillis`; the gen-stamped timer auto-cancels at the
    /// `Pending → Committed` crossing.
    pub state_timeout_millis: Option<i64>,
}

/// Disposition when a `KausalSpec::StateGuard` is not satisfied — the Modal
/// sub-property for the Rubicon statem lowering (OGAR-AST-CONTRACT §6).
/// `#[non_exhaustive]` per the vocabulary forward-compat convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum GuardFailurePolicy {
    /// Transient failure — stay `Pending` and replay after the next
    /// transition. Lowers to `Transition::Postpone`.
    Postponable,
    /// Hard failure — `Pending → Failed` (the default).
    Reject,
}

impl Default for GuardFailurePolicy {
    fn default() -> Self {
        Self::Reject
    }
}

/// A runtime invocation of an `ActionDef` — one per (S, P, O, context)
/// tuple. Captures the actual subject (which user / cron / cascade
/// fired this), provenance for tracing, and lifecycle state.
///
/// B2 production-blocker fixes: every invocation carries trace_id,
/// parent_action_id, idempotency_key (for at-least-once dedup), and
/// ActionState (Pending / Committed / Failed).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct ActionInvocation {
    /// Unique per-invocation identity (ULID/UUID at runtime; OGAR
    /// canonical form `ogit-erp/sale.order::invocation::<ulid>`).
    pub identity: String,
    /// Reference to the ActionDef this invocation realizes.
    pub action_def: String,
    /// Subject of this specific invocation.
    pub subject: ActionSubject,
    /// Object instance identity (e.g. `ogit-erp/sale.order/42`).
    pub object_instance: String,
    /// Actual temporal context at invocation time (may differ from
    /// ActionDef.default_temporal — e.g. cron-deferred vs immediate).
    pub temporal: TemporalSpec,
    /// Actual modal context.
    pub modal: ModalSpec,
    /// Resolved Lokal (which actor instance / tenant / company).
    pub lokal: LokalSpec,
    /// Lifecycle state. Sprint 3 — start Pending; the callcenter
    /// (Sprint 7) transitions to Committed or Failed.
    pub state: ActionState,
    // ── B2 provenance fields ────────────────────────────────────
    /// OpenTelemetry trace ID for cross-actor correlation.
    pub trace_id: Option<String>,
    /// Parent action invocation that cascaded this one (None for
    /// top-level user-initiated actions).
    pub parent_invocation: Option<String>,
    /// Idempotency key — for Modal=Idempotent actions, the dedup
    /// store keys on this string.
    pub idempotency_key: Option<String>,
    /// UTC timestamp of invocation emit (millis since epoch).
    pub emitted_at_millis: Option<i64>,
    /// Failure detail when state == Failed.
    pub failure_reason: Option<String>,
}

/// Subject of a business action — who/what initiated it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum ActionSubject {
    /// A human user (UI button click, RPC call from authenticated user).
    User,
    /// Internal system trigger (no specific user).
    System,
    /// Scheduled (`ir.cron`, Rails `Whenever`).
    Cron,
    /// Reactive (DB event, `@api.depends` triggered).
    Trigger,
    /// Cascaded from a parent action invocation.
    Cascade,
}

impl Default for ActionSubject {
    fn default() -> Self {
        Self::System
    }
}

/// Temporal context — when does the action happen relative to its
/// trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum TemporalSpec {
    /// Synchronous, on-call.
    Immediate,
    /// Queued, async background.
    Deferred,
    /// Run at scheduled time / interval (cron-like).
    Scheduled,
    /// After DB transaction commits (Rails `after_commit`,
    /// Odoo `@api.depends`).
    OnCommit,
}

impl Default for TemporalSpec {
    fn default() -> Self {
        Self::Immediate
    }
}

/// Modal context — how is the action performed.
/// Per B3 YAGNI: dropped `Requires` (no v1 consumer); kept Idempotent
/// because it gates the dedup mechanism in `ActionInvocation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum ModalSpec {
    /// Synchronous, blocking.
    Sync,
    /// Fire-and-forget.
    Async,
    /// Safe to retry (uses `idempotency_key`).
    Idempotent,
    /// All-or-nothing transaction.
    Atomic,
}

impl Default for ModalSpec {
    fn default() -> Self {
        Self::Sync
    }
}

/// Causal precondition — what triggered this action. **Sum type** per
/// B1 review fix. Producers populate one variant; the runtime guard
/// evaluator dispatches on the variant.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum KausalSpec {
    /// State-field precondition (`self.state in {'draft', 'sent'}`).
    StateGuard {
        /// Field name on the object class.
        guard_field: String,
        /// Allowed values for the field.
        guard_values: Vec<String>,
    },
    /// Lifecycle event trigger (`before_save`, `after_create`).
    LifecycleTrigger {
        /// Event name as written.
        event: String,
    },
    /// `@api.depends` dependency paths (1..N).
    /// Per R3 research: avg 3 paths, p95 8, max 14.
    Depends {
        /// Field paths that trigger this action's recomputation.
        paths: Vec<String>,
    },
    /// `@api.depends_context` env-context keys.
    ContextDepends {
        /// Context keys that trigger recomputation.
        keys: Vec<String>,
    },
    /// External cause (RPC call, HTTP request) — no precondition
    /// to check inside the system.
    External,
}

/// Lokal context — where does the action execute (which actor /
/// tenant / company / db partition).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct LokalSpec {
    /// Actor identity that should handle this invocation (e.g.
    /// `ogit-erp/sale.order::actor`). Routes via NiblePath.
    pub actor: Option<String>,
    /// Tenant scope from the `Identity` tenant prefix.
    pub tenant: Option<String>,
    /// Multi-company id when applicable.
    pub company: Option<String>,
}

/// Lifecycle state of an `ActionInvocation`.
/// Per B2 production-blocker #3: explicit state machine prevents
/// the silent-gap problem (action started, didn't complete, no record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum ActionState {
    /// Emitted but not yet processed by the callcenter.
    Pending,
    /// Successfully processed; effects committed.
    Committed,
    /// Processing failed; rollback complete (if Atomic) or
    /// partial effects recorded in `failure_reason`.
    Failed,
    /// Cancelled before execution.
    Cancelled,
}

impl Default for ActionState {
    fn default() -> Self {
        Self::Pending
    }
}

// ─────────────────────────────────────────────────────────────────────
// Sprint 3 constructors (per #[non_exhaustive] convention)
// ─────────────────────────────────────────────────────────────────────

impl ActionDef {
    /// Build an ActionDef with identity, predicate, and object class.
    /// All other fields default.
    #[must_use]
    pub fn new(
        identity: impl Into<String>,
        predicate: impl Into<String>,
        object_class: impl Into<String>,
    ) -> Self {
        Self {
            identity: identity.into(),
            predicate: predicate.into(),
            object_class: object_class.into(),
            ..Default::default()
        }
    }
}

impl ActionInvocation {
    /// Build an ActionInvocation pointing at a defined ActionDef.
    #[must_use]
    pub fn new(
        identity: impl Into<String>,
        action_def: impl Into<String>,
        object_instance: impl Into<String>,
    ) -> Self {
        Self {
            identity: identity.into(),
            action_def: action_def.into(),
            object_instance: object_instance.into(),
            ..Default::default()
        }
    }
}

impl KausalSpec {
    /// Convenience: build a StateGuard.
    #[must_use]
    pub fn state_guard(field: impl Into<String>, values: Vec<String>) -> Self {
        Self::StateGuard {
            guard_field: field.into(),
            guard_values: values,
        }
    }

    /// Convenience: build a LifecycleTrigger.
    #[must_use]
    pub fn lifecycle(event: impl Into<String>) -> Self {
        Self::LifecycleTrigger { event: event.into() }
    }

    /// Convenience: build a Depends spec.
    #[must_use]
    pub fn depends(paths: Vec<String>) -> Self {
        Self::Depends { paths }
    }
}

/// The four canonical Active Record relation kinds. Cross-ORM mapping:
/// Rails `belongs_to`/`has_one`/`has_many`/`has_and_belongs_to_many`,
/// Odoo `Many2one`/`One2many`/`Many2many` (Odoo collapses `has_one` into
/// `One2many` constrained to 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum AssociationKind {
    /// Owning side of a 1:N — the FK lives on this class's table.
    BelongsTo,
    /// Non-owning side of a 1:1.
    HasOne,
    /// Non-owning side of a 1:N.
    HasMany,
    /// Both sides of an M:N via join table.
    HasAndBelongsToMany,
}

/// An association declaration with the full Rails / Odoo option set.
/// Options unset by the source class are `None`; the consumer should
/// treat `None` as "infer per ORM defaults".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Association {
    /// The relation kind.
    pub kind: AssociationKind,
    /// Relation name — the leading symbol on the macro call
    /// (`:project`, `:line_items`, …).
    pub name: String,
    /// `class_name: "Foo::Bar"` — explicit target type when it can't be
    /// inferred from the relation name. `::` namespaces preserved.
    pub class_name: Option<String>,
    /// `foreign_key: "user_id"` — the FK column on the owning table.
    pub foreign_key: Option<String>,
    /// `polymorphic: true` — on `BelongsTo`, target is determined at
    /// runtime by a `<name>_type` column.
    pub polymorphic: Option<bool>,
    /// `through: :memberships` — names the intermediate association
    /// for `HasMany`/`HasOne`.
    pub through: Option<String>,
    /// `source: :principal` — aliasing on a through-association.
    pub source: Option<String>,
    /// `as: :container` — reverse-side polymorphism marker.
    pub as_target: Option<String>,
    /// `dependent: :destroy` / `:delete_all` / `:nullify` / `:restrict_*`.
    pub dependent: Option<String>,
    /// `optional: true` — on `BelongsTo`, allows the FK to be null.
    pub optional: Option<bool>,
    /// `inverse_of: :user` — the reciprocal relation on the target.
    pub inverse_of: Option<String>,
    /// `before_add: :method` collection callback.
    pub before_add: Option<String>,
    /// `after_add: :method` collection callback.
    pub after_add: Option<String>,
    /// `before_remove: :method` collection callback.
    pub before_remove: Option<String>,
    /// `after_remove: :method` collection callback.
    pub after_remove: Option<String>,
    /// Scoping lambda body — for Rails `has_many :line_items, -> { where(active: true) }`,
    /// Django `limit_choices_to={'active': True}`, Odoo `domain=[('active','=',True)]`.
    ///
    /// Captured verbatim as source text. Consumers treat as opaque
    /// (emit into the target form directly) or re-parse for their
    /// needs. `None` means the association has no scoping constraint
    /// — the default and most common case.
    pub scope_source: Option<String>,
    /// `ondelete='cascade'/'restrict'/'set null'/'set default'` —
    /// **DB-level FK action**, distinct from Rails `dependent:`
    /// (app-level). Stored separately to prevent cascade-semantics
    /// confusion. See `docs/ODOO-TRANSCODING.md` §5.
    pub ondelete: Option<String>,
    /// `auto_join=True` (Odoo) — auto SQL-join instead of lazy
    /// load on Many2one.
    pub auto_join: Option<bool>,
    /// `context={...}` (Odoo) — UI default context for navigation
    /// through this association. Captured verbatim as source text.
    pub context_source: Option<String>,
    /// `check_company=True` (Odoo) — multi-company tenancy check
    /// on the FK target.
    pub check_company: Option<bool>,
    /// `delegate=True` — legacy Odoo Many2one delegation (rare;
    /// modern Odoo uses `_inherits` on the class instead).
    pub delegate: Option<bool>,
}

impl Default for AssociationKind {
    fn default() -> Self {
        Self::BelongsTo
    }
}

/// An enum-backed column declaration.
///
/// The `source` field captures three Odoo cases (static / computed /
/// additive); for Rails / Django / Ecto only `Static` applies.
/// See `docs/ODOO-TRANSCODING.md` §6.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct EnumDecl {
    /// Column the enum is backed by.
    pub column: String,
    /// Where the enum's variant list comes from.
    pub source: EnumSource,
    /// `scopes: false` (Rails) — disables ORM-generated scope class
    /// methods. `None` when unset or non-bool.
    pub scopes_disabled: Option<bool>,
}

impl Default for EnumDecl {
    fn default() -> Self {
        Self {
            column: String::new(),
            source: EnumSource::Static(Vec::new()),
            scopes_disabled: None,
        }
    }
}

/// Source of an enum's variant list. Three cases capture Odoo's
/// `selection=`, `selection=lambda`, and `selection_add=`
/// surface; Rails / Django / Ecto producers always emit `Static`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum EnumSource {
    /// `selection=[('draft', 'Draft'), ('done', 'Done')]` — fixed
    /// list of `(key, label)` pairs.
    Static(Vec<(String, String)>),
    /// `selection=lambda self: self.env['res.country']...` — computed
    /// at runtime. The lambda body is captured verbatim.
    Computed(String),
    /// `selection_add=[('paid', 'Paid')]` — extends a parent
    /// `_inherit` model's enum without redeclaring it. `parent_selection`
    /// names the parent class.
    Add {
        /// Additional variants to add to the parent's selection.
        items: Vec<(String, String)>,
        /// The parent class whose selection is being extended
        /// (e.g. `"account.move.line"`).
        parent_selection: String,
    },
}

/// A `store_accessor :col, %i[a b c], prefix: true` declaration — N
/// JSONB pseudo-fields backed by one column.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct StoreAccessor {
    /// JSONB column backing the pseudo-fields.
    pub column: String,
    /// Pseudo-field names in source order.
    pub fields: Vec<String>,
    /// `prefix:` option as written.
    pub prefix: Option<bool>,
}

/// An `attribute :name, :type` schemaless / typed-attribute override.
///
/// Carries an `options` struct for all the cross-cutting kwargs Odoo,
/// Django, and Rails attach to field declarations (`required`,
/// `default`, `translate`, `tracking`, `index`, etc.). Producers
/// populate the subset they support; consumers branch on what's
/// `Some`. See `docs/ODOO-TRANSCODING.md` §4 for the full Odoo
/// kwarg → option mapping.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Attribute {
    /// Attribute name as written.
    pub name: String,
    /// Type name as written (`"string"`, `"integer"`, `"big_integer"`,
    /// `"Char"`, `"Many2one"`, `"Monetary"`, `"Html"`, `"Image"`, …).
    /// Producer-specific — consumers interpret per source language.
    pub type_name: Option<String>,
    /// Cross-cutting per-attribute options. Populated by Odoo /
    /// Django / Rails producers as applicable.
    pub options: AttributeOptions,
}

/// The structured option-set on `Attribute`. Every Odoo kwarg has a
/// home here; no kwarg-dump bucket. Forward-compat via
/// `#[non_exhaustive]` — new producers add new fields, no breaking
/// change.
///
/// See `docs/ODOO-TRANSCODING.md` §4 for the full mapping.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct AttributeOptions {
    /// `default=value` — literal as source text, or callable name.
    /// `None` means no default — column null / Rails default.
    pub default_source: Option<String>,
    /// `required=True` — NOT NULL constraint at the ORM level.
    pub required: Option<bool>,
    /// `readonly=True` — UI / ORM-write blocked.
    pub readonly: Option<bool>,
    /// `index=True` — DB index on the column.
    pub indexed: Option<bool>,
    /// `store=True` — relevant for computed fields; `False` means
    /// the value is recomputed on every read.
    pub stored: Option<bool>,
    /// `translate=True` — i18n column (jsonb in Odoo 17.0).
    pub translate: Option<bool>,
    /// `tracking=True` / `tracking=10` — Odoo audit log priority.
    /// `None` means no tracking; `Some(0)` means tracking with
    /// default priority; higher values are explicit priorities.
    pub tracking: Option<u8>,
    /// `groups='group.xml.id,another.group'` — visibility ACL.
    /// Comma-split into a list.
    pub groups: Vec<String>,
    /// `company_dependent=True` — value varies by `res.company`
    /// (Odoo multi-tenancy).
    pub company_dependent: Option<bool>,
    /// `copy=False` — excluded from `model.copy()`.
    pub copy_on_duplicate: Option<bool>,
    /// `help='...'` — UI tooltip text.
    pub help_text: Option<String>,
    /// `string='Label'` — UI label override (independent of `name`).
    pub label: Option<String>,
    /// `digits=(precision, scale)` — Float/Monetary precision.
    pub digits: Option<(u8, u8)>,
    /// `size=N` — Char/Binary size limit.
    pub size: Option<usize>,
    /// `currency_field='currency_id'` — Monetary field's currency
    /// linkage. Required for `Monetary` type, ignored otherwise.
    pub currency_field: Option<String>,
}

/// A `scope :name, -> { body }` definition. `body_source` is opaque
/// (verbatim source between the lambda brackets) — consumers either
/// accept it as an opaque SQL/DSL snippet or re-parse it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Scope {
    /// Scope name.
    pub name: String,
    /// Body source verbatim between the lambda brackets.
    pub body_source: String,
}

/// A lifecycle callback declaration. Two source forms collapse here:
///
/// - `event :method_name` → `target_method = Some`, `body_source = None`.
/// - `event do ... end` → `target_method = None`, `body_source = Some(text)`.
///
/// The event distinction (`before_*`/`after_*`/`around_*`) is preserved
/// in [`event`](Self::event) so consumers can reason about cascade vs.
/// wrap semantics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Callback {
    /// Event name as written: `before_save`, `after_create`,
    /// `around_destroy`, `after_commit`, …
    pub event: String,
    /// Method name target when the callback names a method.
    pub target_method: Option<String>,
    /// Block body source when the callback is `event do ... end` /
    /// `event { ... }`.
    pub body_source: Option<String>,
}

/// A validation declaration — `validates :col, presence: true` /
/// `@api.constrains('col')`. Placeholder shape; the validation-rule
/// grammar is the next sprint to lift cleanly across ORMs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Validation {
    /// Column or attribute the validation applies to.
    pub target: String,
    /// Validation rule body verbatim. Per-ORM grammar is producer-side.
    pub rule_source: String,
}

// ─────────────────────────────────────────────────────────────────────
// Constructors
//
// Because the public types in this module are `#[non_exhaustive]` (for
// forward compatibility — see the `Language` enum docs), external crates
// cannot construct them with struct-literal syntax. The constructors
// below take the minimal required fields and default the rest, then
// the caller mutates whatever it needs:
//
//     let mut class = Class::new("WorkPackage");
//     class.parent = Some("ApplicationRecord".into());
//
// This is the canonical Rust pattern for `#[non_exhaustive]` types.
// ─────────────────────────────────────────────────────────────────────

impl Class {
    /// Build a new class with only the name set. All other fields are
    /// `Default::default()`. Mutate after construction.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Default::default() }
    }
}

impl Association {
    /// Build a new association with kind and name set.
    #[must_use]
    pub fn new(kind: AssociationKind, name: impl Into<String>) -> Self {
        Self { kind, name: name.into(), ..Default::default() }
    }
}

impl EnumDecl {
    /// Build a new enum declaration with the column set.
    #[must_use]
    pub fn new(column: impl Into<String>) -> Self {
        Self { column: column.into(), ..Default::default() }
    }
}

impl StoreAccessor {
    /// Build a new store-accessor bundle with the JSONB column set.
    #[must_use]
    pub fn new(column: impl Into<String>) -> Self {
        Self { column: column.into(), ..Default::default() }
    }
}

impl Attribute {
    /// Build a new attribute override with the name set.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Default::default() }
    }
}

impl Scope {
    /// Build a new scope with name and body source.
    #[must_use]
    pub fn new(name: impl Into<String>, body_source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            body_source: body_source.into(),
        }
    }
}

impl Callback {
    /// Build a new method-form callback: `before_save :method_name`.
    #[must_use]
    pub fn method(event: impl Into<String>, target_method: impl Into<String>) -> Self {
        Self {
            event: event.into(),
            target_method: Some(target_method.into()),
            body_source: None,
        }
    }

    /// Build a new block-form callback: `after_create do ... end`.
    #[must_use]
    pub fn block(event: impl Into<String>, body_source: impl Into<String>) -> Self {
        Self {
            event: event.into(),
            target_method: None,
            body_source: Some(body_source.into()),
        }
    }
}

impl Validation {
    /// Build a new validation rule with target column and rule body.
    #[must_use]
    pub fn new(target: impl Into<String>, rule_source: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            rule_source: rule_source.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_default_is_empty() {
        let c = Class::default();
        assert!(c.name.is_empty());
        assert!(c.associations.is_empty());
        assert!(matches!(c.language, Language::Ruby));
    }

    #[test]
    fn class_new_sets_only_name() {
        let c = Class::new("WorkPackage");
        assert_eq!(c.name, "WorkPackage");
        assert!(c.parent.is_none());
        assert!(c.associations.is_empty());
    }

    #[test]
    fn elixir_language_is_a_distinct_first_class_variant() {
        // The OLD HIRO/Bardioc stack is Elixir; it is a first-class source
        // language (the migration-roundtrip bridge), not Unknown.
        let mut c = Class::new("Account");
        c.language = Language::Elixir;
        assert_eq!(c.language, Language::Elixir);
        assert_ne!(c.language, Language::Unknown);
        assert_ne!(c.language, Language::Ruby);
    }

    #[test]
    fn association_kind_belongs_to_default() {
        let a = Association::default();
        assert!(matches!(a.kind, AssociationKind::BelongsTo));
    }

    #[test]
    fn association_new_sets_kind_and_name() {
        let a = Association::new(AssociationKind::HasMany, "line_items");
        assert!(matches!(a.kind, AssociationKind::HasMany));
        assert_eq!(a.name, "line_items");
        assert!(a.scope_source.is_none());
    }

    #[test]
    fn association_scope_source_field_present() {
        let mut a = Association::new(AssociationKind::HasMany, "line_items");
        a.scope_source = Some("where(active: true)".into());
        assert_eq!(a.scope_source.as_deref(), Some("where(active: true)"));
    }

    #[test]
    fn callback_two_forms() {
        let method_form = Callback::method("before_save", "touch_parent");
        let block_form = Callback::block("after_create", "notify_subscribers");
        assert_ne!(method_form, block_form);
        assert!(method_form.target_method.is_some());
        assert!(method_form.body_source.is_none());
        assert!(block_form.body_source.is_some());
        assert!(block_form.target_method.is_none());
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::Ruby
    }
}
