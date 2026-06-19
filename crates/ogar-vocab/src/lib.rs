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
    /// Agnostic inheritance slot — metabolizes the three things Rails
    /// conflates (STI parent / abstract base / STI root) into one typed
    /// value. Mixins / concerns are a SEPARATE axis ([`Self::mixins`]) and
    /// are never folded in here. `parent` / `abstract_model` /
    /// `inheritance_column_disabled` are retained for one migration cycle;
    /// new consumers should read `inheritance`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub inheritance: Inheritance,
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
    /// Curator **domain** — the kind of system this class was harvested
    /// from: `"project"` (OpenProject / Redmine), `"erp"` (Odoo / SAP), …
    /// A coarse, curator-agnostic tag (NOT the namespace or module) and a
    /// component of the `ClassFingerprint` used to mint a stable `ClassId`.
    /// Set by the frontend from the harvest namespace; `None` when
    /// unrecognized.
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_domain: Option<String>,
    /// The class's canonical **concept** — its normalized identity
    /// ([`canonical_concept`]); the key cross-domain convergence bridges
    /// on. Most names normalize lexically (`User` → `user`); proven
    /// cross-domain invariants resolve to a promoted concept (OpenProject
    /// `TimeEntry` and Odoo `account.analytic.line` both →
    /// `billable_work_entry`, the [`billable_work_entry`] canonical class).
    /// Set by the frontend at lift time.
    #[cfg_attr(feature = "serde", serde(default))]
    pub canonical_concept: Option<String>,
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

/// How a class sits in its inheritance lattice — the agnostic
/// metabolization of the three things Rails conflates: STI parent,
/// abstract base, and STI root. Mixins / concerns are a SEPARATE axis
/// ([`Class::mixins`]) and are never folded in here.
///
/// Producer IR carries parent/root as **names** (`String`); the registry
/// mints the `ClassId` later. Cross-curator mapping: Rails `< Parent` /
/// `self.abstract_class` / `self.inheritance_column`; Odoo `_inherit` /
/// `_abstract`; Django abstract base classes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum Inheritance {
    /// No superclass beyond the ORM root (Rails `< ApplicationRecord`).
    #[default]
    Root,
    /// Concrete STI child of `parent` (shares the parent's table).
    Concrete {
        /// Parent class name as written.
        parent: String,
    },
    /// Abstract base — methods / fields inherited, but no table of its own
    /// (Rails `self.abstract_class = true`; Odoo `_abstract = True`).
    Abstract,
    /// Root of an STI hierarchy — defines the discriminator column but is
    /// not itself a child. `root` is this class's own name.
    RootedAt {
        /// The hierarchy root class name (this class).
        root: String,
    },
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
    /// typed via [`EnterEffect`] so codegen can apply the transition
    /// structurally (no string-parsing). Emitted as `ogar:onEnter`; lowers
    /// to `StateMachine::on_enter` / the `CommitHook`.
    pub on_enter: Option<EnterEffect>,
    /// Disposition when the Kausal `StateGuard` fails: `Postponable` (stay
    /// `Pending`, replay) vs `Reject` (`Pending → Failed`, the default).
    /// Emitted as `ogar:guardFailurePolicy`.
    pub guard_failure_policy: Option<GuardFailurePolicy>,
    /// Per-state SLA deadline on `Pending`, in milliseconds. Emitted as
    /// `ogar:stateTimeoutMillis`; the gen-stamped timer auto-cancels at the
    /// `Pending → Committed` crossing.
    pub state_timeout_millis: Option<i64>,
}

/// Typed entry effect — the structured representation of the state mutation
/// that fires on entering `Committed` (the Rubicon crossing). Replaces
/// free-form strings on [`ActionDef::on_enter`] so the codegen can apply the
/// transition structurally instead of string-parsing.
///
/// v1 carries the dominant lifecycle-FSM case (`field := to_value`). Complex
/// domain operations (e.g. chess `Move::Castle`) carry their payload on the
/// `ActionInvocation` and use `on_enter` only for the lifecycle-visible
/// transition (e.g. `side_to_move := Black`). Future tightening to typed
/// values (beyond string-encoded `to_value`) is a tracked follow-up.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct EnterEffect {
    /// Field on `object_instance` being set.
    pub field: String,
    /// Value to set the field to (string-encoded; typed values noted as a follow-up).
    pub to_value: String,
}

impl EnterEffect {
    /// Convenience constructor for the common `field := value` case.
    pub fn transition(field: impl Into<String>, to_value: impl Into<String>) -> Self {
        Self { field: field.into(), to_value: to_value.into() }
    }
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

    /// Resolve this class's binary [OGAR codebook] identity — the `u16`
    /// canonical id derived from its stored `canonical_concept`. Returns
    /// `None` when the class has no canonical concept set (a producer
    /// that never set it; rare in practice).
    ///
    /// This is the load-bearing convergence claim: two curator-shaped
    /// classes (Redmine `Issue`, OpenProject `WorkPackage`) lifting to
    /// the same `canonical_concept` produce the same `canonical_id`.
    /// **String labels are decorative; the codebook value is the identity.**
    #[must_use]
    pub fn canonical_id(&self) -> Option<u16> {
        self.canonical_concept.as_deref().and_then(canonical_concept_id)
    }

    /// `canonical_id` rendered as **2 little-endian bytes** — the wire
    /// contract for downstream consumers (SurrealAST, lance-graph-planner,
    /// kanban, …). `None` when no `canonical_concept` is set.
    #[must_use]
    pub fn canonical_id_le(&self) -> Option<[u8; 2]> {
        self.canonical_id().map(u16::to_le_bytes)
    }
}

/// **OGAR codebook registry** — the curated `(canonical_concept, id)`
/// table. Per the integration contract (`docs/INTEGRATION-MAP.md`:92-93,
/// "ClassId / entity_type is minted uniquely by the registry and is
/// never a content hash"), codebook ids are **assigned**, not derived
/// from a hash: a 16-bit hash has real collisions (codex P1 on PR #60
/// confirmed `outcome` and `handle_out` both fold to 33032 under FNV-1a
/// XOR) and would silently merge unrelated concepts downstream.
///
/// Each new promoted canonical concept is added here with the next free
/// id. Ids are stable forever — once shipped, never re-assigned. Id `0`
/// is reserved (`NodeGuid::CLASSID_DEFAULT`); promoted concepts start at
/// `0x0001`. Ids are dense-low so the 2-byte LE wire stays compact.
///
/// Verified collision-free + non-zero by `codebook_has_no_duplicate_ids_or_zero`.
const CODEBOOK: &[(&str, u16)] = &[
    ("project", 0x0001),
    ("project_work_item", 0x0002),
    ("billable_work_entry", 0x0003),
];

/// **OGAR codebook lookup** — resolve a canonical-concept string to its
/// stable `u16` codebook id via the curated [`CODEBOOK`] registry.
/// Returns `None` for unpromoted concepts — they are not in the codebook.
///
/// `u16` width per `OD-CLASSID-WIDTH` (lance-graph-contract `ClassId`).
///
/// # Wire contract — 2 little-endian bytes
///
/// Downstream consumers (SurrealDB AST, lance-graph-planner, kanban, …)
/// serialise the id as 2 little-endian bytes via `u16::to_le_bytes`. Byte
/// order matches the `NodeGuid` layout (`lance-graph-contract`:
/// `canonical_node.rs` — LE throughout) so codebook ids and the
/// `NodeGuid.classid` u16 low half are wire-compatible.
///
/// The contract type ([`LabelDTO`]) lives in `ogar-vocab` today; long-term
/// it belongs in `lance-graph-contract` alongside `ClassId` and the
/// `NodeGuid` LE layout. Wire is the source of truth: any encoder/decoder
/// agreeing on `u16` LE is compatible regardless of which crate exports
/// the DTO.
#[must_use]
pub fn canonical_concept_id(concept: &str) -> Option<u16> {
    CODEBOOK
        .iter()
        .find_map(|(name, id)| if *name == concept { Some(*id) } else { None })
}

/// **Consumer-facing label DTO** — `(label, id, canonical)` triple. The
/// three fields cover the three roles a class identity plays:
///
/// - `label` — **consumer-local** name (curator surface like `"Issue"` /
///   `"account.analytic.line"`, or a domain-specific tag). Not normalised
///   by OGAR.
/// - `id` — **binary codebook identity** ([`ogar_codebook`] of `label`).
///   The actual identity used for set-equality, lookup, dispatch. Two
///   consumers with different labels for the same concept produce DTOs
///   with different `label`s and equal `id`s.
/// - `canonical` — **canonical-AST label** ([`canonical_concept`] of
///   `label`). The portable symbol used by AST consumers (SurrealDB AST,
///   lance-graph-planner, kanban, …) when they need a stable
///   curator-agnostic name. AST emission picks this; identity comparison
///   picks `id`; presentation picks `label`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct LabelDTO {
    /// Consumer-local label. Not normalised by OGAR.
    pub label: String,
    /// OGAR codebook binary identity.
    pub id: u16,
    /// Canonical-AST label — the portable symbol AST / planner / kanban
    /// consumers emit when they need a stable curator-agnostic name.
    pub canonical: String,
}

impl LabelDTO {
    /// Build a `LabelDTO` from a consumer-shaped alias. The OGAR codebook
    /// resolves the alias to its canonical `u16` id without normalising
    /// the `label` itself — `"account.analytic.line"` stays
    /// `"account.analytic.line"`, but its `id` is the same as the id for
    /// `"TimeEntry"` and for `"billable_work_entry"`, and its `canonical`
    /// is `"billable_work_entry"` ready for AST emission.
    ///
    /// Returns `None` when `label` does not resolve to a promoted
    /// canonical concept in the [`CODEBOOK`] — unknown labels have no
    /// codebook identity (they are not in the registry).
    #[must_use]
    pub fn from_alias(label: impl Into<String>) -> Option<Self> {
        let label = label.into();
        let canonical = canonical_concept(&label);
        let id = canonical_concept_id(&canonical)?;
        Some(Self { label, id, canonical })
    }

    /// `id` rendered as **2 little-endian bytes** — the wire contract for
    /// downstream consumers. Roundtrip via `u16::from_le_bytes`.
    #[must_use]
    pub fn id_le(&self) -> [u8; 2] {
        self.id.to_le_bytes()
    }
}

/// **OGAR codebook lookup** — map any alias (curator-shaped *or*
/// canonical-shaped) to its canonical binary id. The curator name does
/// not need to be normalised by the producer; passing the raw Rails or
/// Odoo class name yields the same `u16` as the canonical-concept string.
///
/// ```text
///   ogar_codebook("Issue")                     == codebook("project_work_item")
///   ogar_codebook("WorkPackage")               == codebook("project_work_item")
///   ogar_codebook("TimeEntry")                 == codebook("billable_work_entry")
///   ogar_codebook("account.analytic.line")     == codebook("billable_work_entry")
///   ogar_codebook("Project")                   == codebook("project")
/// ```
///
/// Implementation: resolves the alias through [`canonical_concept`]
/// (which carries the promoted-invariant table) and looks the result up
/// in the [`CODEBOOK`] registry via [`canonical_concept_id`]. Returns
/// `None` when the resolved concept is not in the codebook — unknown
/// aliases have no codebook identity.
#[must_use]
pub fn ogar_codebook(alias: &str) -> Option<u16> {
    canonical_concept_id(&canonical_concept(alias))
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

// ─────────────────────────────────────────────────────────────────────
// Cross-domain synergies
// ─────────────────────────────────────────────────────────────────────

/// A cross-domain **synergy**: one canonical concept that surfaces in two
/// or more curator [domains](Class::source_domain) — e.g. `user` in both
/// the `project` domain (OpenProject `User`) and the `erp` domain (Odoo
/// `res.users`). Wiring synergies is what makes the agnostic vocab more
/// than the sum of its curators: shared concepts unify across domains.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Synergy {
    /// Canonical concept (normalized class name) the members share.
    pub concept: String,
    /// The classes that realize this concept — one entry per domain that
    /// has it, ordered by domain.
    pub members: Vec<SynergyMember>,
}

/// One domain's realization of a [`Synergy`] concept.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct SynergyMember {
    /// The curator domain (`"project"`, `"erp"`, …) — see
    /// [`Class::source_domain`].
    pub domain: String,
    /// The class name as written in that domain.
    pub class_name: String,
}

/// Wire cross-domain synergies across a set of lifted [`Class`]es.
///
/// Groups classes by [`canonical_concept`] and keeps only concepts that
/// appear in **2+ distinct** [`source_domain`](Class::source_domain)s —
/// those bridges are the synergies. Classes with no `source_domain` are
/// skipped (a synergy needs domains to bridge); the first class seen per
/// (concept, domain) wins. Output is deterministic (ordered by concept,
/// then domain).
#[must_use]
pub fn wire_synergies(classes: &[Class]) -> Vec<Synergy> {
    use std::collections::BTreeMap;
    let mut by_concept: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for c in classes {
        let Some(domain) = c.source_domain.as_ref() else {
            continue;
        };
        // Prefer the concept the producer stored; else compute it
        // deterministically from the name — so any consumer session
        // rediscovers the same bridge from ontology surfaces alone.
        let concept = c
            .canonical_concept
            .clone()
            .unwrap_or_else(|| canonical_concept(&c.name));
        by_concept
            .entry(concept)
            .or_default()
            .entry(domain.clone())
            .or_insert_with(|| c.name.clone());
    }
    by_concept
        .into_iter()
        .filter(|(_, domains)| domains.len() >= 2)
        .map(|(concept, domains)| Synergy {
            concept,
            members: domains
                .into_iter()
                .map(|(domain, class_name)| SynergyMember { domain, class_name })
                .collect(),
        })
        .collect()
}

/// Resolve a class name to its canonical OGAR **concept**.
///
/// Two layers, in order:
/// 1. **Promoted cross-domain invariants** — concepts a Claude Code
///    convergence pass has proven across 2+ domains and promoted into
///    OGAR. OGAR stores only the stable result; the proof is the test, the
///    "finding" was the PR that added the arm. (No `SynergyKind` /
///    `SynergyFinding` taxonomy — convergence is an operation, not a
///    stored object.)
/// 2. **Lexical fallback** — lowercase, last dotted segment (Odoo
///    `res.users` → `users`), drop a single trailing plural `s` except
///    after `ss`. Coarse by design, not a thesaurus.
#[must_use]
pub fn canonical_concept(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    // ── Layer 1: promoted invariants ──
    // BillableWorkEntry — booked work / time / cost against a project or
    // order (see [`billable_work_entry`]). OpenProject `TimeEntry` ↔ Odoo
    // `account.analytic.line` ↔ WoA `Arbeitszeit`. ruff normalizes Odoo
    // dots to underscores, so match both forms.
    if matches!(
        lower.as_str(),
        "timeentry"
            | "time_entry"
            | "account.analytic.line"
            | "account_analytic_line"
            | "leistungsposition"
            | "arbeitszeit"
            // canonical class-name spellings (codex P2 on PR #60):
            // `billable_work_entry().name` is "BillableWorkEntry" so the
            // canonical class must round-trip to its own codebook id.
            | "billable_work_entry"
            | "billableworkentry"
    ) {
        return "billable_work_entry".to_string();
    }
    // ProjectWorkItem — project-scoped work items with status, assignment,
    // author, type/tracker, journals, relations, time tracking. The
    // Redmine `Issue` and OpenProject `WorkPackage` overlap (the fork
    // lineage Redmine → ChiliProject → OpenProject preserves the
    // invariant) — both lift here regardless of OpenProject's later
    // modular enrichment. See [`project_work_item`].
    if matches!(
        lower.as_str(),
        "issue"
            | "workpackage"
            | "work_package"
            // canonical class-name spellings (codex P2 on PR #60).
            | "project_work_item"
            | "projectworkitem"
    ) {
        return "project_work_item".to_string();
    }
    // Project — the root container of project-domain work. Both Redmine
    // and OpenProject use `Project`; explicit promotion (rather than
    // relying on lexical fallback) so the canonical class name round-trips
    // (codex P2 on PR #60).
    if matches!(lower.as_str(), "project" | "projects") {
        return "project".to_string();
    }
    // ── Layer 2: lexical fallback ──
    let last = lower.rsplit('.').next().unwrap_or(lower.as_str());
    if last.len() > 3 && last.ends_with('s') && !last.ends_with("ss") {
        last[..last.len() - 1].to_string()
    } else {
        last.to_string()
    }
}

/// The promoted canonical class for the **first convergence invariant**:
/// booked work / time / cost against a project or order. The shared shape
/// under OpenProject `TimeEntry` (project domain), Odoo
/// `account.analytic.line` (erp domain), and WoA `Leistungsposition` /
/// `Arbeitszeit` (german-erp witness). Curators map in via
/// [`canonical_concept`] (`"billable_work_entry"`).
///
/// This is OGAR storing the *stable result* of a convergence pass — not a
/// synergy taxonomy.
///
/// # The 12 family edges (internal) + the adapter edge (external)
///
/// BillableWorkEntry carries **12 family edges** — relations to other
/// canonical concepts, internal to the ontology. The link from a curator
/// surface (OpenProject `TimeEntry`, Odoo `account.analytic.line`) is the
/// **adapter edge**, living *out of family* on the curator class (its
/// `source_domain` + `canonical_concept`), never among these edges.
///
/// **Tax is a boundary policy.** Three family edges — `classified_by →
/// TaxPolicy`, `materializes_as → InvoiceLineCandidate`, `posted_by →
/// PostingAction` — are populated only past the **ERP / posting
/// boundary**. The project domain records work evidence (`duration`,
/// `about → WorkPackage`, `performed_by → Worker`) and never applies tax.
///
/// `materializes_as` is `BelongsTo`, so many BillableWorkEntries aggregate
/// into one `InvoiceLineCandidate` (one invoice line, many work entries).
#[must_use]
pub fn billable_work_entry() -> Class {
    let mut c = Class::new("BillableWorkEntry");
    // Synthetic canonical class — NOT a Ruby-harvested model. Mark the
    // language neutral so the triple-emitter writes `ogar:sourceLanguage`
    // = `Unknown` and consumers do not route this through Ruby-specific
    // handling (codex P2 on OGAR#57).
    c.language = Language::Unknown;
    c.canonical_concept = Some("billable_work_entry".to_string());
    // The 12 family edges — internal ontology meaning. Every target is a
    // canonical concept (PascalCase), never a curator/adapter surface.
    c.associations = vec![
        family_edge("project", "Project"),
        // `about` targets the canonical project-work-item concept — NOT
        // `WorkPackage` (OP curator surface), so Redmine `Issue` and OP
        // `WorkPackage` converge here through their shared
        // `ProjectWorkItem` projection (codex P2 on OGAR#58).
        family_edge("about", "ProjectWorkItem"),
        family_edge("performed_by", "Worker"),
        family_edge("duration", "Duration"),
        family_edge("priced_by", "RatePolicy"),
        family_edge("cost_center", "CostCenter"),
        family_edge("classified_by", "TaxPolicy"), // ERP boundary
        family_edge("materializes_as", "InvoiceLineCandidate"), // ERP boundary
        family_edge("approval_state", "ApprovalState"),
        family_edge("tenant", "Tenant"),
        family_edge("audit_trail", "AuditTrail"),
        family_edge("posted_by", "PostingAction"), // ERP boundary
    ];
    // The defining flag — typed as boolean so DDL adapters that default
    // untyped fields to string-like columns generate the right schema
    // shape (codex P2 on OGAR#57).
    let mut billable = Attribute::new("billable");
    billable.type_name = Some("boolean".to_string());
    c.attributes = vec![billable];
    c
}

/// Build one BillableWorkEntry **family edge** — a `BelongsTo` relation to
/// a canonical ontology concept (the edge's `class_name`). Family edges
/// are internal; curator / adapter links never appear here.
fn family_edge(role: &str, target_concept: &str) -> Association {
    let mut a = Association::new(AssociationKind::BelongsTo, role);
    a.class_name = Some(target_concept.to_string());
    a
}

/// Build one **has-many** family edge — for canonical concepts that
/// aggregate (a [`project_work_item`] has-many `ProjectJournal`s, etc.).
fn family_has_many(role: &str, target_concept: &str) -> Association {
    let mut a = Association::new(AssociationKind::HasMany, role);
    a.class_name = Some(target_concept.to_string());
    a
}

/// The promoted canonical class for the **project-domain work-item
/// invariant**: project-scoped work with status, assignment, type/tracker,
/// priority, author, journals, relations, and time tracking.
///
/// The Redmine → ChiliProject → OpenProject lineage preserves this
/// invariant: Redmine `Issue` and OpenProject `WorkPackage` both map here
/// via [`canonical_concept`] (`"project_work_item"`). Curator labels
/// (`Tracker`, `Type`, `assigned_to`, `responsible`) are leaf details on
/// the curator class; only the canonical roles survive here.
///
/// The 9 family edges sit fully **inside the project domain** — no
/// ERP-boundary slots. The cross-domain bridge to billable work lives on
/// the `time_entries → BillableWorkEntry` has-many edge (project work
/// produces billable work; tax/posting happens past
/// [`billable_work_entry`]'s ERP-boundary edges).
#[must_use]
pub fn project_work_item() -> Class {
    let mut c = Class::new("ProjectWorkItem");
    // Synthetic canonical class — neutral language so the triple-emitter
    // does not route this through Ruby-specific handling. Same fix as
    // [`billable_work_entry`] (codex P2 on OGAR#57).
    c.language = Language::Unknown;
    c.canonical_concept = Some("project_work_item".to_string());
    c.associations = vec![
        family_edge("project", "Project"),
        family_edge("status", "ProjectStatus"),
        family_edge("type", "ProjectType"), // Redmine Tracker / OP Type
        family_edge("priority", "Priority"),
        family_edge("author", "ProjectActor"),
        family_edge("assignee", "ProjectActor"), // Redmine assigned_to / OP assignee
        family_has_many("journals", "ProjectJournal"),
        family_has_many("relations", "ProjectRelation"),
        family_has_many("time_entries", "BillableWorkEntry"),
    ];
    c
}

/// The promoted canonical class for **project** — the root container of
/// project-domain work. Referenced by [`project_work_item`]'s `project`
/// family edge and [`billable_work_entry`]'s `project` edge; this is the
/// canonical class those edges resolve to.
///
/// Redmine `Project` and OpenProject `Project` are universal and share
/// the AR shape: nested-set `parent` (a project may belong to a parent
/// project), `members` (people on the project), the work items
/// themselves, and the time entries booked against the project. Both
/// curators carry `name` + `identifier` as the identity attributes.
///
/// The `work_items` family edge targets the canonical
/// [`project_work_item`] (not the curator surfaces Redmine `Issue` or OP
/// `WorkPackage`); `time_entries` targets [`billable_work_entry`]; the
/// `members` edge points forward at the still-to-come canonical
/// `ProjectActor`.
#[must_use]
pub fn project() -> Class {
    let mut c = Class::new("Project");
    // Synthetic canonical class — neutral language (codex P2 doctrine).
    c.language = Language::Unknown;
    c.canonical_concept = Some("project".to_string());
    c.associations = vec![
        family_has_many("work_items", "ProjectWorkItem"),
        family_has_many("time_entries", "BillableWorkEntry"),
        family_has_many("members", "ProjectActor"),
        // Nested-project parent is a real cross-curator concept but is
        // surfaced via MIXINS in both: Redmine threads it through the
        // `awesome_nested_set` gem (no direct `belongs_to`), OP through
        // the `Projects::Hierarchy` concern. The producer
        // (`ogar_ruby_spo`) does not yet decode either mixin into a
        // canonical parent edge — when it does, a follow-up PR adds
        // `family_edge("parent", "Project")` here and the matching
        // mixin-derived arm to `ogar_from_ruff::project_role_from_mixin`.
    ];
    // Identity attributes — both curators carry these as the canonical
    // human + URL identity.
    let mut name = Attribute::new("name");
    name.type_name = Some("string".to_string());
    let mut identifier = Attribute::new("identifier");
    identifier.type_name = Some("string".to_string());
    c.attributes = vec![name, identifier];
    c
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
    fn wire_synergies_links_a_concept_across_domains() {
        let mut op_user = Class::new("User");
        op_user.source_domain = Some("project".to_string());
        let mut odoo_user = Class::new("res.users");
        odoo_user.source_domain = Some("erp".to_string());
        let mut op_wp = Class::new("WorkPackage");
        op_wp.source_domain = Some("project".to_string());

        let syn = wire_synergies(&[op_user, odoo_user, op_wp]);
        assert_eq!(syn.len(), 1, "only `user` bridges both domains");
        assert_eq!(syn[0].concept, "user");
        assert_eq!(syn[0].members.len(), 2);
        // ordered by domain: erp before project
        assert_eq!(syn[0].members[0].domain, "erp");
        assert_eq!(syn[0].members[0].class_name, "res.users");
        assert_eq!(syn[0].members[1].domain, "project");
        assert_eq!(syn[0].members[1].class_name, "User");
    }

    #[test]
    fn wire_synergies_needs_two_distinct_domains() {
        // same concept, same domain → not a synergy
        let mut a = Class::new("User");
        a.source_domain = Some("project".to_string());
        let mut b = Class::new("Users");
        b.source_domain = Some("project".to_string());
        // an undomained class is ignored entirely
        let c = Class::new("res.users");
        assert!(wire_synergies(&[a, b, c]).is_empty());
    }

    #[test]
    fn canonical_concept_promotes_billable_work_entry_deterministically() {
        // Promoted cross-domain invariant — OpenProject `TimeEntry` and
        // Odoo `account.analytic.line` converge to one canonical concept
        // (both the dotted and ruff's underscored form). Pure +
        // deterministic: same input → same output, every session.
        for name in [
            "TimeEntry",
            "time_entry",
            "account.analytic.line",
            "account_analytic_line",
            "Leistungsposition",
            "Arbeitszeit",
        ] {
            assert_eq!(canonical_concept(name), "billable_work_entry");
            assert_eq!(canonical_concept(name), canonical_concept(name));
        }
        // Un-promoted names still normalize lexically.
        assert_eq!(canonical_concept("User"), "user");
        assert_eq!(canonical_concept("res.users"), "user");
    }

    #[test]
    fn billable_work_entry_has_twelve_family_edges() {
        let c = billable_work_entry();
        assert_eq!(c.name, "BillableWorkEntry");
        assert_eq!(c.canonical_concept.as_deref(), Some("billable_work_entry"));
        // Synthetic canonical class — neutral language (codex P2 on #57).
        assert_eq!(c.language, Language::Unknown);
        // Defining `billable` flag is typed as a boolean so DDL adapters
        // do not default it to string (codex P2 on #57).
        let billable = c
            .attributes
            .iter()
            .find(|a| a.name == "billable")
            .expect("billable attribute");
        assert_eq!(billable.type_name.as_deref(), Some("boolean"));
        // Exactly the 12 internal family edges, to canonical concepts —
        // `about` points at `ProjectWorkItem` (not the OP curator surface
        // `WorkPackage`) so Redmine `Issue` and OP `WorkPackage` converge
        // here through their shared canonical concept (codex P2 on #58).
        assert_eq!(c.associations.len(), 12);
        for target in [
            "Project",
            "ProjectWorkItem",
            "Worker",
            "Duration",
            "RatePolicy",
            "CostCenter",
            "TaxPolicy",
            "InvoiceLineCandidate",
            "ApprovalState",
            "Tenant",
            "AuditTrail",
            "PostingAction",
        ] {
            assert!(
                c.associations.iter().any(|e| e.class_name.as_deref() == Some(target)),
                "missing family edge → {target}",
            );
        }
    }

    #[test]
    fn convergence_project_and_erp_materialize_to_billable_work_entry() {
        let canonical = billable_work_entry();
        // Two curators, only domain tag + name — a consumer session
        // rediscovers the bridge deterministically from these surfaces.
        let mut op = Class::new("TimeEntry");
        op.source_domain = Some("project".to_string());
        op.canonical_concept = Some(canonical_concept("TimeEntry"));
        let mut odoo = Class::new("account_analytic_line");
        odoo.source_domain = Some("erp".to_string());
        odoo.canonical_concept = Some(canonical_concept("account_analytic_line"));

        // Both materialize to the SAME canonical concept as the class.
        assert_eq!(op.canonical_concept, canonical.canonical_concept);
        assert_eq!(odoo.canonical_concept, canonical.canonical_concept);

        // wire_synergies rediscovers exactly one cross-domain bridge,
        // and is idempotent (deterministic).
        let syn = wire_synergies(&[op.clone(), odoo.clone()]);
        assert_eq!(syn, wire_synergies(&[op, odoo]));
        assert_eq!(syn.len(), 1);
        assert_eq!(syn[0].concept, "billable_work_entry");
        assert_eq!(syn[0].members.len(), 2);
    }

    #[test]
    fn tax_policy_is_an_erp_boundary_edge_not_in_project_evidence() {
        // TaxPolicy is a family edge on the canonical shape ...
        let bwe = billable_work_entry();
        assert!(bwe
            .associations
            .iter()
            .any(|e| e.class_name.as_deref() == Some("TaxPolicy")));
        // ... but the project curator records work evidence with no tax.
        let mut op = Class::new("TimeEntry");
        op.source_domain = Some("project".to_string());
        op.canonical_concept = Some(canonical_concept("TimeEntry"));
        assert!(op.associations.is_empty());
        assert!(!op.attributes.iter().any(|a| a.name.contains("tax")));
    }

    #[test]
    fn one_invoice_line_aggregates_many_billable_work_entries() {
        let bwe = billable_work_entry();
        let mat = bwe
            .associations
            .iter()
            .find(|e| e.name == "materializes_as")
            .expect("materializes_as edge");
        // BelongsTo: many BillableWorkEntries → one InvoiceLineCandidate
        // (one invoice line aggregates many work entries).
        assert_eq!(mat.kind, AssociationKind::BelongsTo);
        assert_eq!(mat.class_name.as_deref(), Some("InvoiceLineCandidate"));
    }

    #[test]
    fn canonical_concept_promotes_project_work_item_deterministically() {
        // Promoted project-domain invariant — Redmine `Issue` and
        // OpenProject `WorkPackage` (both spellings) resolve to one
        // canonical concept. Pure + deterministic.
        for name in ["Issue", "issue", "WorkPackage", "work_package", "workpackage"] {
            assert_eq!(canonical_concept(name), "project_work_item");
            assert_eq!(canonical_concept(name), canonical_concept(name));
        }
    }

    #[test]
    fn project_work_item_has_required_family_edges() {
        let c = project_work_item();
        assert_eq!(c.name, "ProjectWorkItem");
        assert_eq!(c.canonical_concept.as_deref(), Some("project_work_item"));
        // Synthetic canonical class — neutral language (codex P2 on #57).
        assert_eq!(c.language, Language::Unknown);
        // The 9 family edges named in the smoke spec.
        for (role, target) in [
            ("project", "Project"),
            ("status", "ProjectStatus"),
            ("type", "ProjectType"),
            ("priority", "Priority"),
            ("author", "ProjectActor"),
            ("assignee", "ProjectActor"),
            ("journals", "ProjectJournal"),
            ("relations", "ProjectRelation"),
            ("time_entries", "BillableWorkEntry"),
        ] {
            let e = c
                .associations
                .iter()
                .find(|a| a.name == role)
                .unwrap_or_else(|| panic!("missing family edge: {role}"));
            assert_eq!(e.class_name.as_deref(), Some(target));
        }
        // has-many vs belongs-to cardinality is correct: journals /
        // relations / time_entries aggregate; the rest are single refs.
        for role in ["journals", "relations", "time_entries"] {
            let e = c.associations.iter().find(|a| a.name == role).unwrap();
            assert_eq!(e.kind, AssociationKind::HasMany);
        }
    }

    #[test]
    fn same_project_domain_curators_do_not_create_duplicate_canonical_concepts() {
        // Redmine `Issue` and OpenProject `WorkPackage` are project-domain
        // work-item curators; they MUST converge to one canonical concept,
        // never two — that's exactly what makes the agnostic vocab worth
        // more than its curators.
        assert_eq!(canonical_concept("Issue"), canonical_concept("WorkPackage"));
        assert_eq!(canonical_concept("Issue"), "project_work_item");
        // The lexical layer remains deterministic for unpromoted names.
        assert_eq!(canonical_concept("User"), canonical_concept("Users"));
    }

    #[test]
    fn codebook_has_no_duplicate_ids_or_zero() {
        // Per `NodeGuid::CLASSID_DEFAULT`, id 0 is canon-reserved; the
        // codebook entries must all be non-zero and unique. This
        // collision-check pins the registry contract (codex P1 on PR #60:
        // unique mint, never a content hash).
        let mut ids = std::collections::HashSet::new();
        let mut names = std::collections::HashSet::new();
        for (name, id) in CODEBOOK {
            assert_ne!(*id, 0, "id 0 is reserved (CLASSID_DEFAULT); offender: {name}");
            assert!(ids.insert(*id), "duplicate codebook id at `{name}`");
            assert!(names.insert(*name), "duplicate canonical name `{name}`");
        }
    }

    #[test]
    fn canonical_concept_id_returns_some_for_promoted_none_for_unknown() {
        // Promoted concepts are in the curated registry — assigned ids.
        for s in ["project", "project_work_item", "billable_work_entry"] {
            assert!(canonical_concept_id(s).is_some(), "promoted `{s}` must be in codebook");
        }
        // Unknown concepts have NO codebook identity — they are not in
        // the registry. Returning None instead of a synthesised hash is
        // the no-silent-collision contract.
        assert_eq!(canonical_concept_id("outcome"), None);
        assert_eq!(canonical_concept_id("handle_out"), None);
        assert_eq!(canonical_concept_id(""), None);
        assert_eq!(canonical_concept_id("user"), None);
    }

    #[test]
    fn ogar_codebook_maps_curator_labels_to_canonical_id() {
        // The load-bearing insight: leave the curator name shape intact;
        // the codebook is what maps to the canonical target.
        let pwi = canonical_concept_id("project_work_item");
        assert!(pwi.is_some());
        assert_eq!(ogar_codebook("Issue"), pwi);
        assert_eq!(ogar_codebook("WorkPackage"), pwi);
        assert_eq!(ogar_codebook("work_package"), pwi);
        // PascalCase canonical class-name spelling resolves to the same
        // id as snake_case canonical (codex P2 fix).
        assert_eq!(ogar_codebook("ProjectWorkItem"), pwi);

        let bwe = canonical_concept_id("billable_work_entry");
        assert!(bwe.is_some());
        assert_eq!(ogar_codebook("TimeEntry"), bwe);
        assert_eq!(ogar_codebook("BillableWorkEntry"), bwe);
        // Odoo-shaped name maps to the same binary id without producer-
        // side normalisation. (Lift implementation lives in the
        // python-side producer the other session owns; the codebook
        // mapping itself stands here.)
        assert_eq!(ogar_codebook("account.analytic.line"), bwe);
        assert_eq!(ogar_codebook("account_analytic_line"), bwe);

        assert_eq!(ogar_codebook("Project"), canonical_concept_id("project"));

        // Unknown alias -> None (no silent hash collision).
        assert_eq!(ogar_codebook("outcome"), None);
        assert_eq!(ogar_codebook("handle_out"), None);
    }

    #[test]
    fn label_dto_carries_local_label_and_shared_codebook_id() {
        // Two consumers with totally different labels for the same
        // concept produce LabelDTOs with different labels and EQUAL ids,
        // and the SAME canonical-AST label (for SurrealAST / planner /
        // kanban consumers that emit a portable symbol).
        let a = LabelDTO::from_alias("Issue").unwrap();
        let b = LabelDTO::from_alias("WorkPackage").unwrap();
        let canonical = LabelDTO::from_alias("project_work_item").unwrap();
        let odoo_shaped = LabelDTO::from_alias("account.analytic.line").unwrap();
        let bwe = LabelDTO::from_alias("billable_work_entry").unwrap();
        // PascalCase canonical class name also resolves (codex P2 fix).
        let pwi_pascal = LabelDTO::from_alias("ProjectWorkItem").unwrap();
        // Labels stay local — not normalised.
        assert_ne!(a.label, b.label, "labels are local");
        assert_eq!(a.label, "Issue");
        assert_eq!(odoo_shaped.label, "account.analytic.line");
        assert_eq!(pwi_pascal.label, "ProjectWorkItem");
        // Ids converge — the address is the identity.
        assert_eq!(a.id, b.id, "address is the identity");
        assert_eq!(a.id, canonical.id, "curator and OGAR labels share the id");
        assert_eq!(a.id, pwi_pascal.id, "PascalCase canonical name shares the id");
        assert_eq!(odoo_shaped.id, bwe.id, "cross-domain label converges on the id");
        assert_ne!(a.id, bwe.id, "distinct concepts have distinct ids");
        // Canonical-AST labels converge — what AST consumers emit.
        assert_eq!(a.canonical, "project_work_item");
        assert_eq!(b.canonical, "project_work_item");
        assert_eq!(canonical.canonical, "project_work_item");
        assert_eq!(pwi_pascal.canonical, "project_work_item");
        assert_eq!(odoo_shaped.canonical, "billable_work_entry");
        assert_eq!(bwe.canonical, "billable_work_entry");

        // Unknown labels: None — they are not in the codebook.
        assert!(LabelDTO::from_alias("outcome").is_none());
        assert!(LabelDTO::from_alias("user").is_none());
    }

    #[test]
    fn le_wire_contract_round_trips() {
        // The wire contract: u16 little-endian, roundtrip-stable across
        // Class.canonical_id_le() and LabelDTO.id_le(). What downstream
        // consumers (SurrealAST, planner, kanban) read off the wire.
        let issue = LabelDTO::from_alias("Issue").unwrap();
        let wp = LabelDTO::from_alias("WorkPackage").unwrap();
        // Same wire bytes for the same concept.
        assert_eq!(issue.id_le(), wp.id_le());
        // Roundtrip via u16::from_le_bytes recovers the id.
        assert_eq!(u16::from_le_bytes(issue.id_le()), issue.id);
        // Class.canonical_id_le agrees with LabelDTO.id_le for the same
        // canonical concept.
        let pwi = project_work_item();
        assert_eq!(
            pwi.canonical_id_le().unwrap(),
            LabelDTO::from_alias("project_work_item").unwrap().id_le(),
        );
        // No canonical -> None on the wire.
        assert_eq!(Class::new("Bare").canonical_id_le(), None);
    }

    #[test]
    fn class_canonical_id_round_trips_through_codebook() {
        // A Class with a canonical_concept set produces the matching
        // codebook id; without one, returns None.
        let c = project_work_item();
        assert_eq!(c.canonical_id(), canonical_concept_id("project_work_item"));
        // Curator-shaped class with canonical_concept populated by the
        // lift: same binary id as a hand-built canonical class.
        let mut redmine_issue = Class::new("Issue");
        redmine_issue.canonical_concept = Some(canonical_concept("Issue"));
        assert_eq!(redmine_issue.canonical_id(), project_work_item().canonical_id());
        // Without a canonical_concept, no id.
        assert_eq!(Class::new("Whatever").canonical_id(), None);
        // Also: canonical_concept that's not promoted -> no codebook id
        // (no silent hash). Set a non-promoted concept directly and
        // confirm None.
        let mut bare = Class::new("Bare");
        bare.canonical_concept = Some("totally_unknown".to_string());
        assert_eq!(bare.canonical_id(), None);
    }

    #[test]
    fn project_is_the_promoted_canonical_class() {
        let c = project();
        assert_eq!(c.name, "Project");
        assert_eq!(c.canonical_concept.as_deref(), Some("project"));
        // Synthetic canonical class — neutral language (codex P2 doctrine).
        assert_eq!(c.language, Language::Unknown);
        // The three direct family edges — all to canonical concepts.
        // (The `parent` edge waits on a producer-side mixin decode for
        // `awesome_nested_set` / `Projects::Hierarchy` — see project()
        // doc.)
        assert_eq!(c.associations.len(), 3);
        for (role, target, kind) in [
            ("work_items", "ProjectWorkItem", AssociationKind::HasMany),
            ("time_entries", "BillableWorkEntry", AssociationKind::HasMany),
            ("members", "ProjectActor", AssociationKind::HasMany),
        ] {
            let e = c
                .associations
                .iter()
                .find(|a| a.name == role)
                .unwrap_or_else(|| panic!("missing family edge: {role}"));
            assert_eq!(e.class_name.as_deref(), Some(target));
            assert_eq!(e.kind, kind);
        }
        // Identity attributes carry types so DDL adapters generate the
        // right column shape (codex P2 doctrine on typed scalars).
        for attr in ["name", "identifier"] {
            let a = c.attributes.iter().find(|x| x.name == attr).unwrap();
            assert_eq!(a.type_name.as_deref(), Some("string"));
        }
    }

    #[test]
    fn openproject_enrichment_does_not_break_redmine_ar_overlap() {
        // OpenProject's WorkPackage is the richer organism (extra includes
        // like `WorkPackages::SpentTime`, `WorkPackages::Costs`,
        // `WorkPackages::Relations`); Redmine's Issue is the cleaner AR
        // fossil. The agnostic vocab survives the evolution: both lift to
        // the same canonical concept.
        let mut redmine_issue = Class::new("Issue");
        redmine_issue.source_domain = Some("project".to_string());
        redmine_issue.canonical_concept = Some(canonical_concept("Issue"));
        redmine_issue.mixins = vec!["Redmine::Acts::Mentionable".to_string()];

        let mut op_wp = Class::new("WorkPackage");
        op_wp.source_domain = Some("project".to_string());
        op_wp.canonical_concept = Some(canonical_concept("WorkPackage"));
        op_wp.mixins = vec![
            "WorkPackages::SpentTime".to_string(),
            "WorkPackages::Costs".to_string(),
            "WorkPackages::Relations".to_string(),
            "WorkPackages::Scheduling".to_string(),
            "OpenProject::Journal::AttachmentHelper".to_string(),
        ];

        // OP is strictly richer than Redmine at the mixin axis ...
        assert!(op_wp.mixins.len() > redmine_issue.mixins.len());
        // ... yet the canonical concept is identical: enrichment did not
        // break the overlap.
        assert_eq!(redmine_issue.canonical_concept, op_wp.canonical_concept);
        assert_eq!(
            redmine_issue.canonical_concept.as_deref(),
            Some("project_work_item"),
        );
    }

    #[test]
    fn family_edges_internal_adapter_edges_external() {
        let bwe = billable_work_entry();
        // All 12 family-edge targets are ONTOLOGY concepts (PascalCase),
        // never curator / adapter surfaces — internal by construction.
        assert_eq!(bwe.associations.len(), 12);
        for e in &bwe.associations {
            let target = e.class_name.as_deref().unwrap_or_default();
            assert!(
                target.starts_with(|ch: char| ch.is_ascii_uppercase()),
                "family edge target must be an ontology concept: {target:?}",
            );
            for curator in ["TimeEntry", "account.", "account_", "OpenProject", "Odoo", "res."] {
                assert!(
                    !target.contains(curator),
                    "curator surface leaked into a family edge: {target:?}",
                );
            }
        }
        // The adapter edge lives OUT of family — on the curator class
        // (source_domain + canonical_concept), not among these edges.
        let mut op = Class::new("TimeEntry");
        op.source_domain = Some("project".to_string());
        op.canonical_concept = Some(canonical_concept("TimeEntry"));
        assert_eq!(op.canonical_concept.as_deref(), Some("billable_work_entry"));
        assert!(bwe.associations.iter().all(|e| e.name != "TimeEntry"));
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

    #[test]
    fn enter_effect_is_typed_and_constructible() {
        // EnterEffect replaces the free-form string carrier on ActionDef.on_enter
        // (per OGAR-AST-CONTRACT §6 follow-up); codegen applies the transition
        // structurally instead of string-parsing.
        let e = EnterEffect::transition("state", "sale");
        assert_eq!(e.field, "state");
        assert_eq!(e.to_value, "sale");
        assert_eq!(e, EnterEffect { field: "state".into(), to_value: "sale".into() });
        assert_ne!(e, EnterEffect::default());
    }

    #[test]
    fn action_def_on_enter_is_typed_enter_effect() {
        // ActionDef.on_enter is now Option<EnterEffect>, not Option<String>.
        let mut a = ActionDef::default();
        assert!(a.on_enter.is_none());
        a.on_enter = Some(EnterEffect::transition("state", "sale"));
        assert_eq!(a.on_enter.as_ref().unwrap().field, "state");
        assert_eq!(a.on_enter.as_ref().unwrap().to_value, "sale");
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::Ruby
    }
}
