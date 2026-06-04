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
pub struct Validation {
    /// Column or attribute the validation applies to.
    pub target: String,
    /// Validation rule body verbatim. Per-ORM grammar is producer-side.
    pub rule_source: String,
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
    fn association_kind_belongs_to_default() {
        let a = Association::default();
        assert!(matches!(a.kind, AssociationKind::BelongsTo));
    }

    #[test]
    fn callback_two_forms() {
        let method_form = Callback {
            event: "before_save".into(),
            target_method: Some("touch_parent".into()),
            body_source: None,
        };
        let block_form = Callback {
            event: "after_create".into(),
            target_method: None,
            body_source: Some("notify_subscribers".into()),
        };
        assert_ne!(method_form, block_form);
        assert!(method_form.target_method.is_some());
        assert!(block_form.body_source.is_some());
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::Ruby
    }
}
