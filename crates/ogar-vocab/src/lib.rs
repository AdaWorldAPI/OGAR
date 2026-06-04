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
    /// `@api.constrains('col')`). Future C17 sprint — placeholder.
    pub validations: Vec<Validation>,
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
}

impl Default for AssociationKind {
    fn default() -> Self {
        Self::BelongsTo
    }
}

/// An `enum :col, { variant: value, ... }, scopes: false` declaration.
/// Values are stringified so int-backed (`{ active: 1 }`) and
/// string-backed (`{ active: "active" }`) enums fit one shape.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct EnumDecl {
    /// Column the enum is backed by.
    pub column: String,
    /// Variant name → stringified literal value, in declaration order.
    pub values: Vec<(String, String)>,
    /// `scopes: false` was passed (disables ORM-generated scope class
    /// methods). `None` when unset or non-bool.
    pub scopes_disabled: Option<bool>,
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Attribute {
    /// Attribute name as written.
    pub name: String,
    /// Type name as written (`"string"`, `"integer"`, `"big_integer"`,
    /// `"Char"`, …). Producer-specific — consumers interpret per
    /// language.
    pub type_name: Option<String>,
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
