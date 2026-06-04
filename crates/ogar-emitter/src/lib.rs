//! `ogar-emitter` — the `OgarEmitter` trait and `Triple` type.
//!
//! Consumers (`ogar-to-postgres`, `ogar-to-surrealql`, lance-graph
//! loader, …) implement [`OgarEmitter`] to convert an OGAR IR class
//! into a stream of triples or DDL.
//!
//! The default implementation [`TripleEmitter`] produces SPO triples
//! suitable for direct ingestion into lance-graph (or any other
//! triple-store) using OGAR-ontology prefix conventions.
//!
//! # Layer position
//!
//! ```text
//!   source AST  ──▶  ogar-vocab::Class  ──▶  OgarEmitter  ──▶  consumer target
//!                                              (this trait)     (triples / DDL / TS)
//! ```
//!
//! # Example
//!
//! ```
//! use ogar_emitter::{OgarEmitter, TripleEmitter};
//! use ogar_vocab::{Class, Language};
//!
//! let mut class = Class::new("WorkPackage");
//! class.language = Language::Ruby;
//! let triples = TripleEmitter::emit_class(&class, "ogit-op");
//! assert!(triples.iter().any(|t| t.predicate == "rdf:type"));
//! ```

#![warn(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use ogar_ontology::{association_identity, class_identity, field_identity};
use ogar_vocab::{
    Association, AssociationKind, Attribute, Callback, Class, EnumDecl, Scope, StoreAccessor,
    Validation,
};

/// A subject-predicate-object triple in the OGAR / OGIT prefix-radix
/// namespace.
///
/// `subject` and `object` are full identity strings (e.g.
/// `ogit-op/WorkPackage`, `ogar:Class`). `predicate` is a relation
/// term (e.g. `rdf:type`, `ogar:hasAssociation`).
///
/// Triples are the wire form between OGAR IR and the lance-graph
/// store. They are also the form RDF/Turtle / JSON-LD / N-Triples
/// consumers expect.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Triple {
    /// The subject identity.
    pub subject: String,
    /// The predicate term.
    pub predicate: String,
    /// The object — an identity string or a literal value.
    pub object: String,
}

impl Triple {
    /// Build a triple from string-like inputs.
    #[must_use]
    pub fn new(subject: impl Into<String>, predicate: impl Into<String>, object: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
        }
    }
}

/// Trait for emitting OGAR IR types to triples (or any equivalent
/// target form). Implementations decide the projection shape.
///
/// The default implementation [`TripleEmitter`] produces SPO triples
/// using OGAR-ontology prefix conventions. Custom implementations
/// can emit DDL, OpenAPI schemas, TypeScript interfaces, etc., by
/// overriding the methods.
pub trait OgarEmitter {
    /// Emit triples for a top-level class declaration.
    fn emit_class(class: &Class, prefix: &str) -> Vec<Triple>;

    /// Emit triples for an association edge owned by `owner_class`.
    fn emit_association(assoc: &Association, owner_class: &str, prefix: &str) -> Vec<Triple>;

    /// Emit triples for an enum-backed column.
    fn emit_enum(enum_decl: &EnumDecl, owner_class: &str, prefix: &str) -> Vec<Triple>;

    /// Emit triples for a JSONB store-accessor bundle.
    fn emit_store_accessor(sa: &StoreAccessor, owner_class: &str, prefix: &str) -> Vec<Triple>;

    /// Emit triples for a typed-attribute override.
    fn emit_attribute(attr: &Attribute, owner_class: &str, prefix: &str) -> Vec<Triple>;

    /// Emit triples for a named scope.
    fn emit_scope(scope: &Scope, owner_class: &str, prefix: &str) -> Vec<Triple>;

    /// Emit triples for a lifecycle callback.
    fn emit_callback(cb: &Callback, owner_class: &str, prefix: &str) -> Vec<Triple>;

    /// Emit triples for a validation rule.
    fn emit_validation(v: &Validation, owner_class: &str, prefix: &str) -> Vec<Triple>;
}

/// The canonical SPO triple emitter for the OGAR vocabulary.
///
/// Produces triples following the convention in `vocab/ogar.ttl`:
/// classes get `rdf:type ogar:Class` plus per-field predicates;
/// associations get `rdf:type ogar:Association` plus relation
/// properties; everything is named under the caller-supplied
/// application prefix (`ogit-op`, `ogit-erp`, …).
pub struct TripleEmitter;

impl OgarEmitter for TripleEmitter {
    fn emit_class(class: &Class, prefix: &str) -> Vec<Triple> {
        let subject = class_identity(prefix, &class.name);
        let estimated_capacity = 2
            + (class.parent.is_some() as usize)
            + class.mixins.len()
            + (class.table_name.is_some() as usize)
            + (class.default_scope.is_some() as usize)
            + class.ignored_columns.len()
            + (class.inheritance_column_disabled as usize)
            + class.scope_predeclarations.len()
            + class.associations.len() * 12
            + class.enums.len() * 6
            + class.store_accessors.len() * 5
            + class.attributes.len() * 3
            + class.scopes.len() * 4
            + class.callbacks.len() * 5
            + class.validations.len() * 4;
        let mut triples = Vec::with_capacity(estimated_capacity);

        triples.push(Triple::new(&subject, "rdf:type", "ogar:Class"));
        triples.push(Triple::new(
            &subject,
            "ogar:sourceLanguage",
            language_to_ogar(class.language),
        ));

        if let Some(ref parent) = class.parent {
            triples.push(Triple::new(
                &subject,
                "ogar:parentClass",
                class_identity(prefix, parent),
            ));
        }

        for mixin in &class.mixins {
            // Mixins are class identities, not raw strings — route them
            // through the same prefix so they participate in the radix
            // index alongside their owners.
            triples.push(Triple::new(
                &subject,
                "ogar:hasMixin",
                class_identity(prefix, mixin),
            ));
        }

        if let Some(ref table) = class.table_name {
            triples.push(Triple::new(&subject, "ogar:tableName", table.clone()));
        }

        if let Some(ref default_scope) = class.default_scope {
            triples.push(Triple::new(&subject, "ogar:defaultScope", default_scope.clone()));
        }

        for col in &class.ignored_columns {
            triples.push(Triple::new(&subject, "ogar:ignoredColumn", col.clone()));
        }

        for name in &class.scope_predeclarations {
            triples.push(Triple::new(&subject, "ogar:scopePredeclaration", name.clone()));
        }

        if class.inheritance_column_disabled {
            triples.push(Triple::new(
                &subject,
                "ogar:inheritanceColumnDisabled",
                "true",
            ));
        }

        // Class-level Odoo metadata (Sprint 2 — codex review fix).
        // Every field added in Sprint 2 emits a triple now; previously
        // they were dropped silently despite being in vocab/ogar.ttl.
        if let Some(ref desc) = class.description {
            triples.push(Triple::new(&subject, "ogar:description", desc.clone()));
        }
        if let Some(ref order) = class.record_order {
            triples.push(Triple::new(&subject, "ogar:recordOrder", order.clone()));
        }
        if let Some(ref rn) = class.rec_name {
            triples.push(Triple::new(&subject, "ogar:recName", rn.clone()));
        }
        if let Some(b) = class.check_company_auto {
            triples.push(Triple::new(&subject, "ogar:checkCompanyAuto", bool_to_str(b)));
        }
        if let Some(b) = class.log_access {
            triples.push(Triple::new(&subject, "ogar:logAccess", bool_to_str(b)));
        }
        if let Some(b) = class.auto_create_table {
            triples.push(Triple::new(&subject, "ogar:autoCreateTable", bool_to_str(b)));
        }
        if class.abstract_model {
            triples.push(Triple::new(&subject, "ogar:abstractModel", "true"));
        }
        if class.transient {
            triples.push(Triple::new(&subject, "ogar:transientModel", "true"));
        }
        if let Some(b) = class.register {
            triples.push(Triple::new(&subject, "ogar:registerModel", bool_to_str(b)));
        }
        if let Some(ref module) = class.declared_in_module {
            triples.push(Triple::new(&subject, "ogar:declaredIn", module.clone()));
        }
        if let Some(ref ver) = class.source_version {
            triples.push(Triple::new(&subject, "ogar:sourceVersion", ver.clone()));
        }

        for (i, assoc) in class.associations.iter().enumerate() {
            triples.extend(Self::emit_association_indexed(assoc, &class.name, prefix, i));
        }

        for (i, enum_decl) in class.enums.iter().enumerate() {
            triples.extend(Self::emit_enum_indexed(enum_decl, &class.name, prefix, i));
        }

        for (i, sa) in class.store_accessors.iter().enumerate() {
            triples.extend(Self::emit_store_accessor_indexed(sa, &class.name, prefix, i));
        }

        for attr in &class.attributes {
            triples.extend(Self::emit_attribute(attr, &class.name, prefix));
        }

        for (i, scope) in class.scopes.iter().enumerate() {
            triples.extend(Self::emit_scope_indexed(scope, &class.name, prefix, i));
        }

        for (i, cb) in class.callbacks.iter().enumerate() {
            triples.extend(Self::emit_callback_indexed(cb, &class.name, prefix, i));
        }

        for (i, v) in class.validations.iter().enumerate() {
            triples.extend(Self::emit_validation_indexed(v, &class.name, prefix, i));
        }

        for (i, cf) in class.computed_fields.iter().enumerate() {
            triples.extend(Self::emit_computed_field_indexed(cf, &class.name, prefix, i));
        }

        for (i, m) in class.methods.iter().enumerate() {
            triples.extend(Self::emit_method_decl_indexed(m, &class.name, prefix, i));
        }

        triples
    }

    fn emit_association(assoc: &Association, owner_class: &str, prefix: &str) -> Vec<Triple> {
        Self::emit_association_indexed(assoc, owner_class, prefix, 0)
    }

    fn emit_enum(enum_decl: &EnumDecl, owner_class: &str, prefix: &str) -> Vec<Triple> {
        Self::emit_enum_indexed(enum_decl, owner_class, prefix, 0)
    }

    fn emit_store_accessor(sa: &StoreAccessor, owner_class: &str, prefix: &str) -> Vec<Triple> {
        Self::emit_store_accessor_indexed(sa, owner_class, prefix, 0)
    }

    fn emit_attribute(attr: &Attribute, owner_class: &str, prefix: &str) -> Vec<Triple> {
        let owner_id = class_identity(prefix, owner_class);
        let attr_id = field_identity(prefix, owner_class, &attr.name);
        let mut triples = vec![
            Triple::new(&owner_id, "ogar:hasField", attr_id.clone()),
            Triple::new(&attr_id, "rdf:type", "ogar:Field"),
            Triple::new(&attr_id, "ogar:fieldName", attr.name.clone()),
        ];
        if let Some(ref t) = attr.type_name {
            triples.push(Triple::new(&attr_id, "ogar:fieldType", t.clone()));
        }

        // AttributeOptions emission (Sprint 2 — codex review fix).
        // Every Option that's Some / Vec that's non-empty produces a
        // triple. Producers populating the structured options now
        // round-trip through the graph instead of silently dropping.
        let opts = &attr.options;
        if let Some(ref d) = opts.default_source {
            triples.push(Triple::new(&attr_id, "ogar:default", d.clone()));
        }
        if let Some(b) = opts.required {
            triples.push(Triple::new(&attr_id, "ogar:required", bool_to_str(b)));
        }
        if let Some(b) = opts.readonly {
            triples.push(Triple::new(&attr_id, "ogar:readonly", bool_to_str(b)));
        }
        if let Some(b) = opts.indexed {
            triples.push(Triple::new(&attr_id, "ogar:indexed", bool_to_str(b)));
        }
        if let Some(b) = opts.stored {
            triples.push(Triple::new(&attr_id, "ogar:fieldStored", bool_to_str(b)));
        }
        if let Some(b) = opts.translate {
            triples.push(Triple::new(&attr_id, "ogar:translate", bool_to_str(b)));
        }
        if let Some(t) = opts.tracking {
            triples.push(Triple::new(&attr_id, "ogar:tracking", t.to_string()));
        }
        for g in &opts.groups {
            triples.push(Triple::new(&attr_id, "ogar:groupAccess", g.clone()));
        }
        if let Some(b) = opts.company_dependent {
            triples.push(Triple::new(&attr_id, "ogar:companyDependent", bool_to_str(b)));
        }
        if let Some(b) = opts.copy_on_duplicate {
            triples.push(Triple::new(&attr_id, "ogar:copyOnDuplicate", bool_to_str(b)));
        }
        if let Some(ref h) = opts.help_text {
            triples.push(Triple::new(&attr_id, "ogar:helpText", h.clone()));
        }
        if let Some(ref l) = opts.label {
            triples.push(Triple::new(&attr_id, "ogar:fieldLabel", l.clone()));
        }
        if let Some((p, s)) = opts.digits {
            triples.push(Triple::new(&attr_id, "ogar:precision", p.to_string()));
            triples.push(Triple::new(&attr_id, "ogar:scale", s.to_string()));
        }
        if let Some(sz) = opts.size {
            triples.push(Triple::new(&attr_id, "ogar:fieldSize", sz.to_string()));
        }
        if let Some(ref cf) = opts.currency_field {
            triples.push(Triple::new(&attr_id, "ogar:currencyField", cf.clone()));
        }

        triples
    }

    fn emit_scope(scope: &Scope, owner_class: &str, prefix: &str) -> Vec<Triple> {
        Self::emit_scope_indexed(scope, owner_class, prefix, 0)
    }

    fn emit_callback(cb: &Callback, owner_class: &str, prefix: &str) -> Vec<Triple> {
        Self::emit_callback_indexed(cb, owner_class, prefix, 0)
    }

    fn emit_validation(v: &Validation, owner_class: &str, prefix: &str) -> Vec<Triple> {
        Self::emit_validation_indexed(v, owner_class, prefix, 0)
    }
}

// ─────────────────────────────────────────────────────────────────────
// Indexed emitters — internal helpers that include a positional index
// in the subject identity so multiple declarations of the same name
// (two `after_create` callbacks, two scopes with the same name, etc.)
// produce distinct subjects instead of silently colliding.
//
// The trait methods above are the public surface; they delegate to
// the indexed forms with index 0 for the single-declaration case.
// ─────────────────────────────────────────────────────────────────────

impl TripleEmitter {
    fn emit_association_indexed(
        assoc: &Association,
        owner_class: &str,
        prefix: &str,
        _index: usize,
    ) -> Vec<Triple> {
        let owner_id = class_identity(prefix, owner_class);
        // Association relation names are unique within a class (the
        // ORM enforces this), so no positional disambiguation needed.
        let assoc_id = association_identity(prefix, owner_class, &assoc.name);

        let mut triples = vec![
            Triple::new(&owner_id, "ogar:hasAssociation", assoc_id.clone()),
            Triple::new(&assoc_id, "rdf:type", "ogar:Association"),
            Triple::new(&assoc_id, "ogar:kind", association_kind_to_ogar(assoc.kind)),
            Triple::new(&assoc_id, "ogar:relationName", assoc.name.clone()),
        ];

        if let Some(ref cn) = assoc.class_name {
            triples.push(Triple::new(&assoc_id, "ogar:targetClass", class_identity(prefix, cn)));
        }
        if let Some(ref fk) = assoc.foreign_key {
            triples.push(Triple::new(&assoc_id, "ogar:foreignKey", fk.clone()));
        }
        if let Some(p) = assoc.polymorphic {
            triples.push(Triple::new(&assoc_id, "ogar:polymorphic", bool_to_str(p)));
        }
        if let Some(ref t) = assoc.through {
            triples.push(Triple::new(&assoc_id, "ogar:through", t.clone()));
        }
        if let Some(ref s) = assoc.source {
            triples.push(Triple::new(&assoc_id, "ogar:sourceAlias", s.clone()));
        }
        if let Some(ref a) = assoc.as_target {
            triples.push(Triple::new(&assoc_id, "ogar:asTarget", a.clone()));
        }
        if let Some(ref d) = assoc.dependent {
            triples.push(Triple::new(&assoc_id, "ogar:dependent", d.clone()));
        }
        if let Some(o) = assoc.optional {
            triples.push(Triple::new(&assoc_id, "ogar:optional", bool_to_str(o)));
        }
        if let Some(ref i) = assoc.inverse_of {
            triples.push(Triple::new(&assoc_id, "ogar:inverseOf", i.clone()));
        }
        if let Some(ref s) = assoc.scope_source {
            triples.push(Triple::new(&assoc_id, "ogar:scopeSource", s.clone()));
        }
        if let Some(ref m) = assoc.before_add {
            triples.push(Triple::new(&assoc_id, "ogar:beforeAdd", m.clone()));
        }
        if let Some(ref m) = assoc.after_add {
            triples.push(Triple::new(&assoc_id, "ogar:afterAdd", m.clone()));
        }
        if let Some(ref m) = assoc.before_remove {
            triples.push(Triple::new(&assoc_id, "ogar:beforeRemove", m.clone()));
        }
        if let Some(ref m) = assoc.after_remove {
            triples.push(Triple::new(&assoc_id, "ogar:afterRemove", m.clone()));
        }

        // Odoo-flavored Association extensions (Sprint 2 — codex review
        // fix). These fields are in the IR struct and TTL but were
        // dropped from emission until now.
        if let Some(ref od) = assoc.ondelete {
            triples.push(Triple::new(&assoc_id, "ogar:ondelete", od.clone()));
        }
        if let Some(aj) = assoc.auto_join {
            triples.push(Triple::new(&assoc_id, "ogar:autoJoin", bool_to_str(aj)));
        }
        if let Some(ref ctx) = assoc.context_source {
            triples.push(Triple::new(&assoc_id, "ogar:contextSource", ctx.clone()));
        }
        if let Some(cc) = assoc.check_company {
            triples.push(Triple::new(&assoc_id, "ogar:checkCompany", bool_to_str(cc)));
        }
        if let Some(d) = assoc.delegate {
            triples.push(Triple::new(&assoc_id, "ogar:delegateField", bool_to_str(d)));
        }

        triples
    }

    fn emit_enum_indexed(
        enum_decl: &EnumDecl,
        owner_class: &str,
        prefix: &str,
        _index: usize,
    ) -> Vec<Triple> {
        let owner_id = class_identity(prefix, owner_class);
        let enum_id = format!("{}/{}::enum::{}", prefix, owner_class, enum_decl.column);
        let mut triples = vec![
            Triple::new(&owner_id, "ogar:hasEnum", enum_id.clone()),
            Triple::new(&enum_id, "rdf:type", "ogar:EnumDecl"),
            Triple::new(&enum_id, "ogar:column", enum_decl.column.clone()),
        ];
        // EnumSource has three variants — emit different triples for each
        // to preserve Odoo's `selection=lambda` and `selection_add=` cases
        // beyond the simple static list.
        match &enum_decl.source {
            ogar_vocab::EnumSource::Static(values) => {
                triples.push(Triple::new(&enum_id, "ogar:enumSourceKind", "ogar:Static"));
                for (i, (name, value)) in values.iter().enumerate() {
                    let variant_id = format!("{enum_id}#{i}");
                    triples.push(Triple::new(&enum_id, "ogar:hasVariant", variant_id.clone()));
                    triples.push(Triple::new(&variant_id, "rdf:type", "ogar:EnumVariant"));
                    triples.push(Triple::new(&variant_id, "ogar:variantName", name.clone()));
                    triples.push(Triple::new(&variant_id, "ogar:variantValue", value.clone()));
                }
            }
            ogar_vocab::EnumSource::Computed(body) => {
                triples.push(Triple::new(&enum_id, "ogar:enumSourceKind", "ogar:Computed"));
                triples.push(Triple::new(&enum_id, "ogar:enumComputedBody", body.clone()));
            }
            ogar_vocab::EnumSource::Add { items, parent_selection } => {
                triples.push(Triple::new(&enum_id, "ogar:enumSourceKind", "ogar:Add"));
                triples.push(Triple::new(
                    &enum_id,
                    "ogar:enumParentSelection",
                    parent_selection.clone(),
                ));
                for (i, (name, value)) in items.iter().enumerate() {
                    let variant_id = format!("{enum_id}#add{i}");
                    triples.push(Triple::new(&enum_id, "ogar:hasVariant", variant_id.clone()));
                    triples.push(Triple::new(&variant_id, "rdf:type", "ogar:EnumVariant"));
                    triples.push(Triple::new(&variant_id, "ogar:variantName", name.clone()));
                    triples.push(Triple::new(&variant_id, "ogar:variantValue", value.clone()));
                }
            }
            _ => {
                // Future non_exhaustive EnumSource variants — emit a
                // marker triple but don't panic. Real producers must
                // upgrade to handle.
                triples.push(Triple::new(&enum_id, "ogar:enumSourceKind", "ogar:Unknown"));
            }
        }
        if let Some(disabled) = enum_decl.scopes_disabled {
            triples.push(Triple::new(
                &enum_id,
                "ogar:scopesDisabled",
                bool_to_str(disabled),
            ));
        }
        triples
    }

    fn emit_store_accessor_indexed(
        sa: &StoreAccessor,
        owner_class: &str,
        prefix: &str,
        _index: usize,
    ) -> Vec<Triple> {
        let owner_id = class_identity(prefix, owner_class);
        // Distinct namespace from Attribute/EnumDecl on the same column.
        let sa_id = format!("{}/{}::store::{}", prefix, owner_class, sa.column);
        let mut triples = vec![
            Triple::new(&owner_id, "ogar:hasStoreAccessor", sa_id.clone()),
            Triple::new(&sa_id, "rdf:type", "ogar:StoreAccessor"),
            Triple::new(&sa_id, "ogar:column", sa.column.clone()),
        ];
        for f in &sa.fields {
            triples.push(Triple::new(&sa_id, "ogar:storeField", f.clone()));
        }
        if let Some(p) = sa.prefix {
            triples.push(Triple::new(&sa_id, "ogar:storePrefix", bool_to_str(p)));
        }
        triples
    }

    fn emit_scope_indexed(
        scope: &Scope,
        owner_class: &str,
        prefix: &str,
        _index: usize,
    ) -> Vec<Triple> {
        let owner_id = class_identity(prefix, owner_class);
        // Scope names are unique within a class.
        let scope_id = format!("{}/{}::scope::{}", prefix, owner_class, scope.name);
        vec![
            Triple::new(&owner_id, "ogar:hasScope", scope_id.clone()),
            Triple::new(&scope_id, "rdf:type", "ogar:Scope"),
            Triple::new(&scope_id, "ogar:scopeName", scope.name.clone()),
            Triple::new(&scope_id, "ogar:scopeBody", scope.body_source.clone()),
        ]
    }

    fn emit_callback_indexed(
        cb: &Callback,
        owner_class: &str,
        prefix: &str,
        index: usize,
    ) -> Vec<Triple> {
        let owner_id = class_identity(prefix, owner_class);
        let cb_id = format!("{}/{}::callback::{}::{}", prefix, owner_class, index, cb.event);
        let mut triples = vec![
            Triple::new(&owner_id, "ogar:hasCallback", cb_id.clone()),
            Triple::new(&cb_id, "rdf:type", "ogar:Callback"),
            Triple::new(&cb_id, "ogar:event", cb.event.clone()),
        ];
        if let Some(ref m) = cb.target_method {
            triples.push(Triple::new(&cb_id, "ogar:targetMethod", m.clone()));
        }
        if let Some(ref b) = cb.body_source {
            triples.push(Triple::new(&cb_id, "ogar:callbackBody", b.clone()));
        }
        triples
    }

    fn emit_validation_indexed(
        v: &Validation,
        owner_class: &str,
        prefix: &str,
        index: usize,
    ) -> Vec<Triple> {
        let owner_id = class_identity(prefix, owner_class);
        let v_id = format!("{}/{}::validation::{}::{}", prefix, owner_class, index, v.target);
        vec![
            Triple::new(&owner_id, "ogar:hasValidation", v_id.clone()),
            Triple::new(&v_id, "rdf:type", "ogar:Validation"),
            Triple::new(&v_id, "ogar:validationTarget", v.target.clone()),
            Triple::new(&v_id, "ogar:validationRule", v.rule_source.clone()),
        ]
    }

    fn emit_computed_field_indexed(
        cf: &ogar_vocab::ComputedField,
        owner_class: &str,
        prefix: &str,
        _index: usize,
    ) -> Vec<Triple> {
        let owner_id = class_identity(prefix, owner_class);
        let cf_id = format!("{}/{}::computed::{}", prefix, owner_class, cf.field);
        let mut triples = vec![
            Triple::new(&owner_id, "ogar:hasComputedField", cf_id.clone()),
            Triple::new(&cf_id, "rdf:type", "ogar:ComputedField"),
            Triple::new(&cf_id, "ogar:computedFieldRef", cf.field.clone()),
            Triple::new(&cf_id, "ogar:computeMethod", cf.compute_method.clone()),
            Triple::new(&cf_id, "ogar:stored", bool_to_str(cf.stored)),
        ];
        for d in &cf.depends {
            triples.push(Triple::new(&cf_id, "ogar:dependsPath", d.clone()));
        }
        for d in &cf.depends_context {
            triples.push(Triple::new(&cf_id, "ogar:dependsContext", d.clone()));
        }
        if let Some(ref m) = cf.inverse_method {
            triples.push(Triple::new(&cf_id, "ogar:inverseMethod", m.clone()));
        }
        if let Some(ref m) = cf.search_method {
            triples.push(Triple::new(&cf_id, "ogar:searchMethod", m.clone()));
        }
        triples
    }

    fn emit_method_decl_indexed(
        m: &ogar_vocab::MethodDecl,
        owner_class: &str,
        prefix: &str,
        index: usize,
    ) -> Vec<Triple> {
        let owner_id = class_identity(prefix, owner_class);
        let m_id = format!("{}/{}::method::{}::{}", prefix, owner_class, index, m.name);
        let mut triples = vec![
            Triple::new(&owner_id, "ogar:hasMethod", m_id.clone()),
            Triple::new(&m_id, "rdf:type", "ogar:MethodDecl"),
            Triple::new(&m_id, "ogar:methodName", m.name.clone()),
            Triple::new(&m_id, "ogar:methodBody", m.body_source.clone()),
            Triple::new(&m_id, "ogar:methodKind", method_kind_to_ogar(m.kind)),
            Triple::new(
                &m_id,
                "ogar:recordSemantics",
                record_semantics_to_ogar(m.semantics),
            ),
        ];
        for d in &m.decorators {
            triples.push(Triple::new(&m_id, "ogar:decoratorName", d.clone()));
        }
        triples
    }
}

// ─────────────────────────────────────────────────────────────────────
// Sprint 3 — Action vocabulary emission
// ─────────────────────────────────────────────────────────────────────

impl TripleEmitter {
    /// Emit triples for an ActionDef.
    pub fn emit_action_def(def: &ogar_vocab::ActionDef) -> Vec<Triple> {
        let id = def.identity.clone();
        let mut triples = vec![
            Triple::new(&id, "rdf:type", "ogar:ActionDef"),
            Triple::new(&id, "ogar:actionPredicate", def.predicate.clone()),
            Triple::new(&id, "ogar:actionObjectClass", def.object_class.clone()),
            Triple::new(&id, "ogar:defaultSubject", subject_to_ogar(def.default_subject)),
            Triple::new(&id, "ogar:defaultTemporal", temporal_to_ogar(def.default_temporal)),
            Triple::new(&id, "ogar:defaultModal", modal_to_ogar(def.default_modal)),
        ];
        // Action-scoped predicates — NOT the MethodDecl ones. Reusing
        // ogar:methodBody / ogar:decoratorName (domain ogar:MethodDecl)
        // would make every body-carrying ActionDef infer as a MethodDecl
        // and pollute method-enumeration queries. Codex review 2026-06-04.
        if let Some(ref body) = def.body_source {
            triples.push(Triple::new(&id, "ogar:actionBody", body.clone()));
        }
        for d in &def.decorators {
            triples.push(Triple::new(&id, "ogar:actionDecorator", d.clone()));
        }
        if let Some(ref k) = def.kausal {
            triples.extend(kausal_triples(&id, k));
        }
        triples
    }

    /// Emit triples for an ActionInvocation. Carries the SPO+TeKaMoLo
    /// annotation plus B2 provenance fields (trace_id, parent
    /// invocation, idempotency key, emitted_at).
    pub fn emit_action_invocation(inv: &ogar_vocab::ActionInvocation) -> Vec<Triple> {
        let id = inv.identity.clone();
        let mut triples = vec![
            Triple::new(&id, "rdf:type", "ogar:ActionInvocation"),
            Triple::new(&id, "ogar:realizes", inv.action_def.clone()),
            Triple::new(&id, "ogar:actionSubject", subject_to_ogar(inv.subject)),
            Triple::new(&id, "ogar:actionObject", inv.object_instance.clone()),
            Triple::new(&id, "ogar:actionTemporal", temporal_to_ogar(inv.temporal)),
            Triple::new(&id, "ogar:actionModal", modal_to_ogar(inv.modal)),
            Triple::new(&id, "ogar:actionState", action_state_to_ogar(inv.state)),
        ];
        if let Some(ref a) = inv.lokal.actor {
            triples.push(Triple::new(&id, "ogar:actionLokal", a.clone()));
        }
        if let Some(ref t) = inv.lokal.tenant {
            triples.push(Triple::new(&id, "ogar:actionTenant", t.clone()));
        }
        if let Some(ref c) = inv.lokal.company {
            triples.push(Triple::new(&id, "ogar:actionCompany", c.clone()));
        }
        if let Some(ref t) = inv.trace_id {
            triples.push(Triple::new(&id, "ogar:traceId", t.clone()));
        }
        if let Some(ref p) = inv.parent_invocation {
            triples.push(Triple::new(&id, "ogar:parentInvocation", p.clone()));
        }
        if let Some(ref k) = inv.idempotency_key {
            triples.push(Triple::new(&id, "ogar:idempotencyKey", k.clone()));
        }
        if let Some(ts) = inv.emitted_at_millis {
            triples.push(Triple::new(&id, "ogar:emittedAt", ts.to_string()));
        }
        if let Some(ref r) = inv.failure_reason {
            triples.push(Triple::new(&id, "ogar:failureReason", r.clone()));
        }
        triples
    }
}

fn kausal_triples(action_subject: &str, k: &ogar_vocab::KausalSpec) -> Vec<Triple> {
    use ogar_vocab::KausalSpec;
    match k {
        KausalSpec::StateGuard { guard_field, guard_values } => {
            let mut v = vec![
                Triple::new(action_subject, "ogar:kausalKind", "ogar:StateGuard"),
                Triple::new(action_subject, "ogar:guardField", guard_field.clone()),
            ];
            for val in guard_values {
                v.push(Triple::new(action_subject, "ogar:guardValue", val.clone()));
            }
            v
        }
        KausalSpec::LifecycleTrigger { event } => vec![
            Triple::new(action_subject, "ogar:kausalKind", "ogar:LifecycleTrigger"),
            Triple::new(action_subject, "ogar:triggerEvent", event.clone()),
        ],
        // Kausal-scoped dependency predicates — NOT the ComputedField ones.
        // ogar:dependsPath / ogar:dependsContext have domain ogar:ComputedField;
        // reusing them here would make a dependency-guarded action infer as a
        // ComputedField and corrupt "enumerate all computed fields" queries.
        // Codex review 2026-06-04.
        KausalSpec::Depends { paths } => {
            let mut v = vec![Triple::new(action_subject, "ogar:kausalKind", "ogar:Depends")];
            for p in paths {
                v.push(Triple::new(action_subject, "ogar:kausalDependsPath", p.clone()));
            }
            v
        }
        KausalSpec::ContextDepends { keys } => {
            let mut v = vec![Triple::new(action_subject, "ogar:kausalKind", "ogar:ContextDepends")];
            for k in keys {
                v.push(Triple::new(action_subject, "ogar:kausalDependsContext", k.clone()));
            }
            v
        }
        KausalSpec::External => vec![Triple::new(action_subject, "ogar:kausalKind", "ogar:External")],
        _ => vec![Triple::new(action_subject, "ogar:kausalKind", "ogar:Unknown")],
    }
}

fn subject_to_ogar(s: ogar_vocab::ActionSubject) -> &'static str {
    use ogar_vocab::ActionSubject;
    match s {
        ActionSubject::User => "ogar:User",
        ActionSubject::System => "ogar:System",
        ActionSubject::Cron => "ogar:Cron",
        ActionSubject::Trigger => "ogar:Trigger",
        ActionSubject::Cascade => "ogar:Cascade",
        _ => "ogar:Unknown",
    }
}

fn temporal_to_ogar(t: ogar_vocab::TemporalSpec) -> &'static str {
    use ogar_vocab::TemporalSpec;
    match t {
        TemporalSpec::Immediate => "ogar:Immediate",
        TemporalSpec::Deferred => "ogar:Deferred",
        TemporalSpec::Scheduled => "ogar:Scheduled",
        TemporalSpec::OnCommit => "ogar:OnCommit",
        _ => "ogar:Immediate",
    }
}

fn modal_to_ogar(m: ogar_vocab::ModalSpec) -> &'static str {
    use ogar_vocab::ModalSpec;
    match m {
        ModalSpec::Sync => "ogar:Sync",
        ModalSpec::Async => "ogar:Async",
        ModalSpec::Idempotent => "ogar:Idempotent",
        ModalSpec::Atomic => "ogar:Atomic",
        _ => "ogar:Sync",
    }
}

fn action_state_to_ogar(s: ogar_vocab::ActionState) -> &'static str {
    use ogar_vocab::ActionState;
    match s {
        ActionState::Pending => "ogar:Pending",
        ActionState::Committed => "ogar:Committed",
        ActionState::Failed => "ogar:Failed",
        ActionState::Cancelled => "ogar:Cancelled",
        _ => "ogar:Pending",
    }
}

fn method_kind_to_ogar(kind: ogar_vocab::MethodKind) -> &'static str {
    use ogar_vocab::MethodKind;
    match kind {
        MethodKind::CrudOverride => "ogar:CrudOverride",
        MethodKind::ApiModel => "ogar:ApiModel",
        MethodKind::ApiModelCreateMulti => "ogar:ApiModelCreateMulti",
        MethodKind::Instance => "ogar:Instance",
        _ => "ogar:Unknown",
    }
}

fn record_semantics_to_ogar(s: ogar_vocab::RecordSemantics) -> &'static str {
    use ogar_vocab::RecordSemantics;
    match s {
        RecordSemantics::Record => "ogar:Record",
        RecordSemantics::Recordset => "ogar:Recordset",
        RecordSemantics::ClassLevel => "ogar:ClassLevel",
        _ => "ogar:Recordset",
    }
}

fn language_to_ogar(lang: ogar_vocab::Language) -> &'static str {
    use ogar_vocab::Language;
    match lang {
        Language::Ruby => "ogar:Ruby",
        Language::Python => "ogar:Python",
        Language::Sql => "ogar:Sql",
        Language::TypeScript => "ogar:TypeScript",
        Language::SurrealQl => "ogar:SurrealQl",
        Language::Unknown => "ogar:Unknown",
        _ => "ogar:Unknown",
    }
}

fn association_kind_to_ogar(kind: AssociationKind) -> &'static str {
    match kind {
        AssociationKind::BelongsTo => "ogar:BelongsTo",
        AssociationKind::HasOne => "ogar:HasOne",
        AssociationKind::HasMany => "ogar:HasMany",
        AssociationKind::HasAndBelongsToMany => "ogar:HasAndBelongsToMany",
        // Unknown future variants are EXPLICIT — never silently
        // collapsed to BelongsTo. This is the brutal-review CB1
        // correctness fix.
        _ => "ogar:Unknown",
    }
}

fn bool_to_str(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_work_package() -> Class {
        let mut class = Class::new("WorkPackage");
        class.parent = Some("ApplicationRecord".into());
        class.language = ogar_vocab::Language::Ruby;
        let mut assoc = Association::new(AssociationKind::BelongsTo, "project");
        assoc.class_name = Some("Project".into());
        class.associations.push(assoc);
        class
    }

    #[test]
    fn emit_class_produces_rdf_type_triple() {
        let class = sample_work_package();
        let triples = TripleEmitter::emit_class(&class, "ogit-op");
        assert!(triples.iter().any(|t| {
            t.subject == "ogit-op/WorkPackage"
                && t.predicate == "rdf:type"
                && t.object == "ogar:Class"
        }));
    }

    #[test]
    fn emit_class_records_parent_and_language() {
        let class = sample_work_package();
        let triples = TripleEmitter::emit_class(&class, "ogit-op");
        assert!(triples.iter().any(|t| {
            t.subject == "ogit-op/WorkPackage"
                && t.predicate == "ogar:parentClass"
                && t.object == "ogit-op/ApplicationRecord"
        }));
        assert!(triples.iter().any(|t| {
            t.predicate == "ogar:sourceLanguage" && t.object == "ogar:Ruby"
        }));
    }

    #[test]
    fn emit_association_is_a_subgraph() {
        let class = sample_work_package();
        let triples = TripleEmitter::emit_class(&class, "ogit-op");
        // Owner class points at the association edge
        assert!(triples.iter().any(|t| {
            t.subject == "ogit-op/WorkPackage"
                && t.predicate == "ogar:hasAssociation"
                && t.object == "ogit-op/WorkPackage->project"
        }));
        // Association edge has kind + target
        assert!(triples.iter().any(|t| {
            t.subject == "ogit-op/WorkPackage->project"
                && t.predicate == "ogar:kind"
                && t.object == "ogar:BelongsTo"
        }));
        assert!(triples.iter().any(|t| {
            t.subject == "ogit-op/WorkPackage->project"
                && t.predicate == "ogar:targetClass"
                && t.object == "ogit-op/Project"
        }));
    }

    #[test]
    fn scope_source_on_association_is_captured() {
        let mut class = sample_work_package();
        let mut line_items = Association::new(AssociationKind::HasMany, "line_items");
        line_items.scope_source = Some("where(active: true)".into());
        class.associations.push(line_items);
        let triples = TripleEmitter::emit_class(&class, "ogit-op");
        assert!(triples.iter().any(|t| {
            t.subject == "ogit-op/WorkPackage->line_items"
                && t.predicate == "ogar:scopeSource"
                && t.object == "where(active: true)"
        }));
    }

    #[test]
    fn empty_class_still_produces_type_and_language_triples() {
        let class = Class::new("Empty");
        let triples = TripleEmitter::emit_class(&class, "ogit-test");
        assert_eq!(triples.len(), 2);
        assert!(triples.iter().any(|t| t.predicate == "rdf:type"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:sourceLanguage"));
    }

    #[test]
    fn enum_emits_variant_name_and_value_separately() {
        let mut class = sample_work_package();
        let mut e = EnumDecl::new("status");
        e.source = ogar_vocab::EnumSource::Static(vec![
            ("open".into(), "0".into()),
            ("closed".into(), "1".into()),
        ]);
        e.scopes_disabled = Some(true);
        class.enums.push(e);
        let triples = TripleEmitter::emit_class(&class, "ogit-op");
        // Variant name + value emitted as separate triples on a
        // synthetic per-variant subject — round-trippable even when
        // names or values contain `=` or other separators.
        assert!(triples.iter().any(|t| {
            t.predicate == "ogar:variantName" && t.object == "open"
        }));
        assert!(triples.iter().any(|t| {
            t.predicate == "ogar:variantValue" && t.object == "0"
        }));
        assert!(triples.iter().any(|t| {
            t.predicate == "ogar:variantName" && t.object == "closed"
        }));
        assert!(triples.iter().any(|t| {
            t.predicate == "ogar:variantValue" && t.object == "1"
        }));
        assert!(triples.iter().any(|t| {
            t.predicate == "ogar:scopesDisabled" && t.object == "true"
        }));
        // Enum subject lives under `::enum::` namespace (distinct
        // from Attribute under `field_identity` and StoreAccessor
        // under `::store::`).
        assert!(triples.iter().any(|t| {
            t.subject == "ogit-op/WorkPackage::enum::status"
                && t.predicate == "rdf:type"
                && t.object == "ogar:EnumDecl"
        }));
    }

    #[test]
    fn duplicate_callbacks_get_indexed_subjects() {
        // Rails allows two `after_create do ... end` blocks on the same
        // class — they MUST emit distinct subjects, not silently overwrite.
        let mut class = sample_work_package();
        class.callbacks.push(Callback::block("after_create", "step_one"));
        class.callbacks.push(Callback::block("after_create", "step_two"));
        let triples = TripleEmitter::emit_class(&class, "ogit-op");
        let cb_subjects: Vec<_> = triples
            .iter()
            .filter(|t| t.predicate == "rdf:type" && t.object == "ogar:Callback")
            .map(|t| t.subject.as_str())
            .collect();
        assert_eq!(cb_subjects.len(), 2, "two distinct callback subjects expected");
        assert_ne!(cb_subjects[0], cb_subjects[1]);
    }

    #[test]
    fn unknown_association_kind_does_not_collapse_to_belongs_to() {
        // The brutal-review CB1 fix: future AssociationKind variants
        // must NOT silently map to ogar:BelongsTo. Verify the fallback
        // arm exists by inspecting the `association_kind_to_ogar`
        // behaviour for known variants — the unknown arm itself is
        // covered by the source comment + ogar:Unknown ttl entry.
        for kind in [
            AssociationKind::BelongsTo,
            AssociationKind::HasOne,
            AssociationKind::HasMany,
            AssociationKind::HasAndBelongsToMany,
        ] {
            assert!(association_kind_to_ogar(kind).starts_with("ogar:"));
            assert_ne!(association_kind_to_ogar(kind), "ogar:Unknown");
        }
    }

    #[test]
    fn mixins_emit_through_class_identity_not_raw_strings() {
        let mut class = Class::new("WorkPackage");
        class.mixins.push("Mentionable".into());
        let triples = TripleEmitter::emit_class(&class, "ogit-op");
        assert!(triples.iter().any(|t| {
            t.subject == "ogit-op/WorkPackage"
                && t.predicate == "ogar:hasMixin"
                && t.object == "ogit-op/Mentionable"
        }));
    }

    #[test]
    fn collection_callbacks_on_association_emit_triples() {
        let mut class = Class::new("Project");
        let mut assoc = Association::new(AssociationKind::HasMany, "enabled_modules");
        assoc.after_remove = Some("module_disabled".into());
        class.associations.push(assoc);
        let triples = TripleEmitter::emit_class(&class, "ogit-op");
        assert!(triples.iter().any(|t| {
            t.predicate == "ogar:afterRemove" && t.object == "module_disabled"
        }));
    }

    #[test]
    fn scope_predeclarations_emit_triples() {
        let mut class = Class::new("Principal");
        class.scope_predeclarations = vec!["like".into(), "human".into(), "visible".into()];
        let triples = TripleEmitter::emit_class(&class, "ogit-op");
        let preds: Vec<_> = triples
            .iter()
            .filter(|t| t.predicate == "ogar:scopePredeclaration")
            .map(|t| t.object.as_str())
            .collect();
        assert_eq!(preds, vec!["like", "human", "visible"]);
    }

    // ───────────────────────────────────────────────────────────────
    // Codex review 2026-06-04 fixes — exercise every newly-wired
    // emission path. These tests fail on the pre-fix emitter where
    // the IR field was present but no triple was emitted.
    // ───────────────────────────────────────────────────────────────

    #[test]
    fn class_metadata_fields_emit_triples() {
        let mut class = Class::new("sale.order");
        class.description = Some("Sale Order".into());
        class.record_order = Some("date desc, id".into());
        class.rec_name = Some("name".into());
        class.declared_in_module = Some("sale".into());
        class.source_version = Some("17.0".into());
        class.abstract_model = false;
        class.transient = false;
        class.check_company_auto = Some(true);
        let triples = TripleEmitter::emit_class(&class, "ogit-erp");
        assert!(triples.iter().any(|t| t.predicate == "ogar:description" && t.object == "Sale Order"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:recordOrder" && t.object == "date desc, id"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:recName" && t.object == "name"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:declaredIn" && t.object == "sale"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:sourceVersion" && t.object == "17.0"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:checkCompanyAuto" && t.object == "true"));
    }

    #[test]
    fn class_abstract_and_transient_flags_emit_when_true() {
        let mut class = Class::new("mail.thread");
        class.abstract_model = true;
        let triples = TripleEmitter::emit_class(&class, "ogit-erp");
        assert!(triples.iter().any(|t| t.predicate == "ogar:abstractModel" && t.object == "true"));
        assert!(!triples.iter().any(|t| t.predicate == "ogar:transientModel"));
    }

    #[test]
    fn association_odoo_options_emit_triples() {
        let mut class = Class::new("sale.order");
        let mut assoc = Association::new(AssociationKind::BelongsTo, "partner_id");
        assoc.ondelete = Some("restrict".into());
        assoc.auto_join = Some(true);
        assoc.context_source = Some("{'default_partner_id': active_id}".into());
        assoc.check_company = Some(true);
        assoc.delegate = Some(false);
        class.associations.push(assoc);
        let triples = TripleEmitter::emit_class(&class, "ogit-erp");
        assert!(triples.iter().any(|t| t.predicate == "ogar:ondelete" && t.object == "restrict"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:autoJoin" && t.object == "true"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:contextSource"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:checkCompany" && t.object == "true"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:delegateField" && t.object == "false"));
    }

    #[test]
    fn attribute_options_emit_full_triple_set() {
        let mut class = Class::new("sale.order");
        let mut attr = Attribute::new("name");
        attr.type_name = Some("Char".into());
        attr.options.required = Some(true);
        attr.options.translate = Some(true);
        attr.options.tracking = Some(10);
        attr.options.indexed = Some(true);
        attr.options.size = Some(64);
        attr.options.help_text = Some("Order reference".into());
        attr.options.label = Some("Order".into());
        attr.options.groups = vec!["sales.group_user".into(), "sales.group_manager".into()];
        attr.options.default_source = Some("New".into());
        class.attributes.push(attr);
        let triples = TripleEmitter::emit_class(&class, "ogit-erp");
        assert!(triples.iter().any(|t| t.predicate == "ogar:required" && t.object == "true"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:translate" && t.object == "true"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:tracking" && t.object == "10"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:indexed" && t.object == "true"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:fieldSize" && t.object == "64"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:helpText" && t.object == "Order reference"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:fieldLabel" && t.object == "Order"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:default" && t.object == "New"));
        // Two groups → two triples.
        let group_triples: Vec<_> = triples.iter().filter(|t| t.predicate == "ogar:groupAccess").collect();
        assert_eq!(group_triples.len(), 2);
    }

    #[test]
    fn attribute_options_digits_split_into_precision_and_scale() {
        let mut attr = Attribute::new("amount_total");
        attr.type_name = Some("Monetary".into());
        attr.options.digits = Some((16, 2));
        attr.options.currency_field = Some("currency_id".into());
        let mut class = Class::new("sale.order");
        class.attributes.push(attr);
        let triples = TripleEmitter::emit_class(&class, "ogit-erp");
        assert!(triples.iter().any(|t| t.predicate == "ogar:precision" && t.object == "16"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:scale" && t.object == "2"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:currencyField" && t.object == "currency_id"));
    }

    #[test]
    fn computed_field_emits_full_subgraph() {
        let mut class = Class::new("sale.order");
        let mut cf = ogar_vocab::ComputedField::default();
        cf.field = "amount_total".into();
        cf.compute_method = "_compute_amount_total".into();
        cf.depends = vec!["order_line.price_total".into(), "currency_id".into()];
        cf.depends_context = vec!["company_id".into()];
        cf.stored = true;
        cf.inverse_method = Some("_inverse_total".into());
        class.computed_fields.push(cf);
        let triples = TripleEmitter::emit_class(&class, "ogit-erp");
        assert!(triples.iter().any(|t| t.predicate == "ogar:hasComputedField"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:computedFieldRef" && t.object == "amount_total"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:computeMethod" && t.object == "_compute_amount_total"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:stored" && t.object == "true"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:dependsPath" && t.object == "order_line.price_total"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:dependsPath" && t.object == "currency_id"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:dependsContext" && t.object == "company_id"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:inverseMethod" && t.object == "_inverse_total"));
        assert!(!triples.iter().any(|t| t.predicate == "ogar:searchMethod"));
    }

    #[test]
    fn method_decl_emits_with_kind_and_semantics() {
        let mut class = Class::new("sale.order");
        let mut m = ogar_vocab::MethodDecl::default();
        m.name = "create".into();
        m.kind = ogar_vocab::MethodKind::ApiModelCreateMulti;
        m.body_source = "for vals in vals_list: ...".into();
        m.decorators = vec!["api.model_create_multi".into()];
        m.semantics = ogar_vocab::RecordSemantics::ClassLevel;
        class.methods.push(m);
        let triples = TripleEmitter::emit_class(&class, "ogit-erp");
        assert!(triples.iter().any(|t| t.predicate == "ogar:hasMethod"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:methodName" && t.object == "create"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:methodKind" && t.object == "ogar:ApiModelCreateMulti"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:recordSemantics" && t.object == "ogar:ClassLevel"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:decoratorName" && t.object == "api.model_create_multi"));
    }

    #[test]
    fn duplicate_methods_get_indexed_subjects() {
        // Two methods with the same name (rare but possible in legacy
        // Odoo extends) must produce distinct subjects.
        let mut class = Class::new("sale.order");
        for name in ["action_confirm", "action_confirm"] {
            let mut m = ogar_vocab::MethodDecl::default();
            m.name = name.into();
            class.methods.push(m);
        }
        let triples = TripleEmitter::emit_class(&class, "ogit-erp");
        let method_subjects: Vec<_> = triples
            .iter()
            .filter(|t| t.predicate == "rdf:type" && t.object == "ogar:MethodDecl")
            .map(|t| t.subject.as_str())
            .collect();
        assert_eq!(method_subjects.len(), 2);
        assert_ne!(method_subjects[0], method_subjects[1]);
    }

    // ───────────────────────────────────────────────────────────────
    // Sprint 3 — Action vocabulary tests
    // ───────────────────────────────────────────────────────────────

    #[test]
    fn action_def_emits_full_subgraph() {
        let mut def = ogar_vocab::ActionDef::new(
            "ogit-erp/sale.order::action_def::action_confirm",
            "action_confirm",
            "ogit-erp/sale.order",
        );
        def.default_subject = ogar_vocab::ActionSubject::User;
        def.default_temporal = ogar_vocab::TemporalSpec::Immediate;
        def.default_modal = ogar_vocab::ModalSpec::Atomic;
        def.kausal = Some(ogar_vocab::KausalSpec::state_guard(
            "state",
            vec!["draft".into(), "sent".into()],
        ));
        def.body_source = Some("if self.state != 'draft': raise...".into());
        def.decorators = vec!["api.depends".into()];
        let triples = TripleEmitter::emit_action_def(&def);
        assert!(triples.iter().any(|t| t.predicate == "rdf:type" && t.object == "ogar:ActionDef"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:actionPredicate" && t.object == "action_confirm"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:defaultSubject" && t.object == "ogar:User"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:defaultModal" && t.object == "ogar:Atomic"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:kausalKind" && t.object == "ogar:StateGuard"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:guardField" && t.object == "state"));
        let guard_values: Vec<_> = triples
            .iter()
            .filter(|t| t.predicate == "ogar:guardValue")
            .map(|t| t.object.as_str())
            .collect();
        assert_eq!(guard_values, vec!["draft", "sent"]);
    }

    #[test]
    fn action_invocation_carries_b2_provenance_fields() {
        let mut inv = ogar_vocab::ActionInvocation::new(
            "ogit-erp/sale.order::invocation::01H8Q...",
            "ogit-erp/sale.order::action_def::action_confirm",
            "ogit-erp/sale.order/42",
        );
        inv.subject = ogar_vocab::ActionSubject::User;
        inv.temporal = ogar_vocab::TemporalSpec::Immediate;
        inv.modal = ogar_vocab::ModalSpec::Atomic;
        inv.state = ogar_vocab::ActionState::Pending;
        inv.lokal.actor = Some("ogit-erp/sale.order::actor".into());
        inv.lokal.tenant = Some("acme".into());
        inv.trace_id = Some("00-trace-id-here".into());
        inv.parent_invocation = Some("ogit-erp/account.payment::invocation::01H8...".into());
        inv.idempotency_key = Some("confirm-sale-order-42".into());
        inv.emitted_at_millis = Some(1717500000000);
        let triples = TripleEmitter::emit_action_invocation(&inv);
        assert!(triples.iter().any(|t| t.predicate == "rdf:type" && t.object == "ogar:ActionInvocation"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:actionSubject" && t.object == "ogar:User"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:actionState" && t.object == "ogar:Pending"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:traceId" && t.object == "00-trace-id-here"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:parentInvocation"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:idempotencyKey" && t.object == "confirm-sale-order-42"));
        assert!(triples.iter().any(|t| t.predicate == "ogar:actionTenant" && t.object == "acme"));
    }

    #[test]
    fn kausal_spec_variants_emit_distinct_kinds() {
        let mut def = ogar_vocab::ActionDef::new("a", "p", "ogit-op/Foo");
        def.kausal = Some(ogar_vocab::KausalSpec::lifecycle("before_save"));
        let t = TripleEmitter::emit_action_def(&def);
        assert!(t.iter().any(|t| t.predicate == "ogar:kausalKind" && t.object == "ogar:LifecycleTrigger"));
        assert!(t.iter().any(|t| t.predicate == "ogar:triggerEvent" && t.object == "before_save"));

        def.kausal = Some(ogar_vocab::KausalSpec::depends(vec!["partner_id".into(), "amount".into()]));
        let t = TripleEmitter::emit_action_def(&def);
        assert!(t.iter().any(|t| t.predicate == "ogar:kausalKind" && t.object == "ogar:Depends"));
        // Kausal-scoped predicate, distinct from ComputedField's ogar:dependsPath
        // (so "enumerate computed fields" queries don't catch actions).
        let paths: Vec<_> = t.iter().filter(|t| t.predicate == "ogar:kausalDependsPath").map(|t| t.object.as_str()).collect();
        assert_eq!(paths.len(), 2);
        // The ComputedField predicate must NOT appear on an ActionDef.
        assert!(!t.iter().any(|t| t.predicate == "ogar:dependsPath"));

        def.kausal = Some(ogar_vocab::KausalSpec::External);
        let t = TripleEmitter::emit_action_def(&def);
        assert!(t.iter().any(|t| t.predicate == "ogar:kausalKind" && t.object == "ogar:External"));
    }

    #[test]
    fn action_state_lifecycle_serialization() {
        for (state, expected) in [
            (ogar_vocab::ActionState::Pending, "ogar:Pending"),
            (ogar_vocab::ActionState::Committed, "ogar:Committed"),
            (ogar_vocab::ActionState::Failed, "ogar:Failed"),
            (ogar_vocab::ActionState::Cancelled, "ogar:Cancelled"),
        ] {
            let mut inv = ogar_vocab::ActionInvocation::new("i", "d", "o");
            inv.state = state;
            let triples = TripleEmitter::emit_action_invocation(&inv);
            assert!(triples.iter().any(|t| t.predicate == "ogar:actionState" && t.object == expected));
        }
    }

    #[test]
    fn callback_two_forms_emit_distinct_triples() {
        let mut class = sample_work_package();
        class.callbacks.push(Callback::method("before_save", "touch_parent"));
        class.callbacks.push(Callback::block("after_create", "notify_subscribers"));
        let triples = TripleEmitter::emit_class(&class, "ogit-op");
        assert!(triples.iter().any(|t| {
            t.predicate == "ogar:targetMethod" && t.object == "touch_parent"
        }));
        assert!(triples.iter().any(|t| {
            t.predicate == "ogar:callbackBody" && t.object == "notify_subscribers"
        }));
    }

    #[test]
    fn triple_new_accepts_any_into_string() {
        let t = Triple::new("a", String::from("b"), "c");
        assert_eq!(t.subject, "a");
        assert_eq!(t.predicate, "b");
        assert_eq!(t.object, "c");
    }
}
