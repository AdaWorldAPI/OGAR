//! `ogar-from-ruff` — lift `ruff_spo_triplet::Model` into `ogar_vocab::Class`.
//!
//! The ruff IR (`ruff_spo_triplet::Model` populated by `ruff_ruby_spo` and
//! future `ruff_python_spo` / `ruff_elixir_spo` frontends) was
//! deliberately shaped to mirror `ogar_vocab::Class` (see
//! `ogar-vocab/src/lib.rs`:14: "The types deliberately mirror the
//! C17a–c stable shape in `ruff_ruby_spo` so the existing producer can
//! be lifted in-place"). This crate is the mechanical projection that
//! does that lift in-place.
//!
//! # Layer position
//!
//! ```text
//!   source tree           → ruff_ruby_spo::extract → ruff_spo_triplet::ModelGraph
//!                                                              │
//!                                                  this crate ─┤  lift_model_graph / lift_model
//!                                                              ▼
//!                                                     ogar_vocab::Class
//!                                                              │
//!                                                  ogar-from-rails (or callers)
//!                                                              ▼
//!                                            lance-graph-ontology::OntologyRegistry
//! ```
//!
//! # Field map (the part of the contract that matters)
//!
//! | ruff (Model)           | ogar (Class)                  | notes                                       |
//! |------------------------|-------------------------------|---------------------------------------------|
//! | `name`                 | `name`                        | verbatim                                    |
//! | `associations`         | `associations`                | rich option parsing per [`lift_association`]; `AcceptsNestedAttributesFor` skipped (UI form helper, not a relation) |
//! | `validations`          | `validations`                 | flatten kind + target + options into `rule_source` |
//! | `callbacks`            | `callbacks`                   | `phase` → `event`, `target` → `target_method` |
//! | `concerns`             | `mixins`                      | `IncludesModule` / `ExtendsModule` / `PrependsModule` lift their target; block markers (`ConcernClassMethods` / `ConcernIncludedBlock`) skipped |
//! | `attributes`           | `attributes`                  | the option `"type"` lifts to `Attribute.type_name` |
//! | `scopes`               | `scopes` / `default_scope`    | `Scope` / `Scopes` → `Class.scopes`; `DefaultScope` → `Class.default_scope` |
//! | `acts_as`              | `mixins`                      | rendered as `acts_as_<variant>` so they survive on the same shelf as concerns (`ogar-vocab` has no separate `acts_as` slot) |
//! | `sti.inherits_from`    | `parent`                      | STI parent — matches `Class.parent` slot    |
//! | `functions`            | `Vec<ActionDef>` (DO-arm)     | [`lift_actions`] — one `ActionDef` per method; standalone, not on `Class` |
//!
//! Fields NOT lifted today (no equivalent on the ruff side OR no clean
//! semantic mapping):
//!
//! - `Model::functions` IS now lifted — by [`lift_actions`] (the DO-arm)
//!   to a standalone `Vec<ActionDef>`, not onto `Class` (the OGAR `Class`
//!   is the THING/THINK shape; actions register on the DO-arm
//!   separately). Each method's `reads` / `raises` / `traverses` edges
//!   stay on the narrow / SPO arm as triples — `lift_actions` does not
//!   duplicate them, nor claim a reactive dependency plain Rails methods
//!   don't declare.
//! - `Model::delegations`, `Model::dsl_calls`, `Model::gem_dsl`,
//!   `Model::dynamic_methods`, `Model::refinements` — ruff's
//!   long-tail. OGAR doesn't model them yet; can land as
//!   extension prefixes (`ogar-extensions/rails/...`) later.
//!
//! # What this crate does NOT do
//!
//! - **Source I/O**: it operates on already-extracted `Model` /
//!   `ModelGraph` values. `ogar-from-rails` adds the file-walk +
//!   `ruff_ruby_spo::extract` invocation.
//! - **Ontology registration**: handing the resulting `Class` off to
//!   `lance-graph-ontology::OntologyRegistry` is a separate step.
//!   This crate's output is registry-ready, not registry-bound.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use ogar_vocab::{
    ActionDef, Association, AssociationKind, Attribute, Callback, Class, EnumDecl, EnumSource,
    Inheritance, Language, Scope, Validation,
};
use ruff_spo_triplet::{
    AssocDecl, AssocKind, AttrDecl, AttrKind, Callback as RuffCallback, ConcernKind, Model,
    ModelGraph, ScopeDecl, ScopeKind, StiInfo, Validation as RuffValidation, ValidationKind,
};

/// Lift every model in a [`ModelGraph`] to an OGAR [`Class`]. Output
/// preserves declaration order so downstream consumers can rely on
/// deterministic ordering for snapshot tests.
#[must_use]
pub fn lift_model_graph(graph: &ModelGraph) -> Vec<Class> {
    graph.models.iter().map(lift_model).collect()
}

/// Lift one [`Model`] to an OGAR [`Class`]. Pure projection — no I/O.
///
/// **Language is always [`Language::Ruby`]** at this layer. Other
/// frontends (Python/Elixir/etc.) using `ruff_spo_triplet::Model` will
/// likely want their own wrapper that sets the discriminant. We don't
/// guess it from `ModelGraph::namespace` because the namespace
/// (`openproject`, `odoo`, …) doesn't bind to the producer language
/// one-to-one.
#[must_use]
pub fn lift_model(model: &Model) -> Class {
    let mut class = Class::new(&model.name);
    class.language = Language::Ruby;
    class.parent = model.sti.as_ref().and_then(sti_parent);
    class.inheritance = lift_inheritance(model);
    class.associations = model.associations.iter().filter_map(lift_association).collect();
    class.mixins = lift_mixins(model);
    class.attributes = model.attributes.iter().filter_map(lift_attribute).collect();
    class.enums = model.attributes.iter().filter_map(lift_enum).collect();
    class.scopes = model.scopes.iter().filter_map(lift_scope).collect();
    class.scope_predeclarations = lift_scope_predeclarations(model);
    class.callbacks = model.callbacks.iter().map(lift_callback).collect();
    class.validations = model.validations.iter().filter_map(lift_validation).collect();
    class.default_scope = lift_default_scope(model);
    class
}

// ───────────────────────────── actions (DO-arm) ─────────────────────────

/// Lift a [`Model`]'s methods to OGAR [`ActionDef`] declarations — the
/// DO-arm of the harvest. One `ActionDef` per [`ruff_spo_triplet::Function`].
///
/// This is a **facts-only** lift: it sets the action's `identity`,
/// `predicate` (the method name — already snake_case on the Rails side),
/// and `object_class` (the model name). It deliberately does **not**:
///
/// - set any execution policy (the vocab [`ActionDef`] has no `exec` slot;
///   backend routing is consumer-private — see the OP-arm plan §5.2),
/// - construct an `ActionInvocation` (a live-instance carrier: no target
///   instance / cycle / trace exists at harvest time),
/// - populate `kausal` from [`ruff_spo_triplet::Function`]`::reads` — a
///   plain Rails method reading a field is **not** a reactive
///   `@api.depends`-style trigger, so claiming one would leak method-body
///   description into causal semantics. Those `reads` / `raises` /
///   `traverses` edges already ride the narrow / SPO arm as triples.
///
/// The result is registry-ready: guard / RBAC / `exec` enrichment happens
/// downstream at registration, not in the producer.
#[must_use]
pub fn lift_actions(model: &Model) -> Vec<ActionDef> {
    model
        .functions
        .iter()
        .map(|f| {
            ActionDef::new(
                format!("{}::action_def::{}", model.name, f.name),
                &f.name,
                &model.name,
            )
        })
        .collect()
}

// ───────────────────────────── associations ──────────────────────────────

/// Lift one [`AssocDecl`] to an OGAR [`Association`].
///
/// Returns `None` for [`AssocKind::AcceptsNestedAttributesFor`] —
/// Rails' `accepts_nested_attributes_for :foo` is a UI form helper,
/// not a relation. The ogar [`AssociationKind`] enum has no variant
/// for it; skipping is correct.
#[must_use]
pub fn lift_association(a: &AssocDecl) -> Option<Association> {
    let kind = match a.kind {
        AssocKind::BelongsTo => AssociationKind::BelongsTo,
        AssocKind::HasOne => AssociationKind::HasOne,
        AssocKind::HasMany => AssociationKind::HasMany,
        AssocKind::HasAndBelongsToMany => AssociationKind::HasAndBelongsToMany,
        AssocKind::AcceptsNestedAttributesFor => return None,
    };
    let mut out = Association::new(kind, &a.name);
    for (k, v) in &a.options {
        let trimmed = strip_ruby_literal_markers(v);
        match k.as_str() {
            "class_name" => out.class_name = Some(trimmed.to_string()),
            "foreign_key" => out.foreign_key = Some(trimmed.to_string()),
            "polymorphic" => out.polymorphic = parse_bool(trimmed),
            "through" => out.through = Some(trimmed.to_string()),
            "source" => out.source = Some(trimmed.to_string()),
            "as" => out.as_target = Some(trimmed.to_string()),
            "dependent" => out.dependent = Some(trimmed.to_string()),
            "optional" => out.optional = parse_bool(trimmed),
            "inverse_of" => out.inverse_of = Some(trimmed.to_string()),
            "before_add" => out.before_add = Some(trimmed.to_string()),
            "after_add" => out.after_add = Some(trimmed.to_string()),
            "before_remove" => out.before_remove = Some(trimmed.to_string()),
            "after_remove" => out.after_remove = Some(trimmed.to_string()),
            // Unknown options pass through silently — forward-compat
            // for ORM extensions Rails ships later.
            _ => {}
        }
    }
    Some(out)
}

// ───────────────────────────── mixins ───────────────────────────────────

fn lift_mixins(model: &Model) -> Vec<String> {
    let mut out = Vec::new();
    for c in &model.concerns {
        // Block markers (`class_methods do` / `included do`) carry no
        // target; skip them. Real `include` / `extend` / `prepend`
        // declarations lift to the mixin name verbatim.
        match c.kind {
            ConcernKind::Include | ConcernKind::Extend | ConcernKind::Prepend => {
                out.push(c.module.clone());
            }
            ConcernKind::ClassMethodsBlock | ConcernKind::IncludedBlock => {}
        }
    }
    for aa in &model.acts_as {
        // Render `acts_as_<variant>` so consumers can pattern-match
        // on the prefix. Inline options drop to keep the mixin string
        // bounded (full options survive on the ruff side).
        out.push(format!("acts_as_{}", aa.variant));
    }
    out
}

// ───────────────────────────── attributes ───────────────────────────────

fn lift_attribute(a: &AttrDecl) -> Option<Attribute> {
    // Only "real" attribute-bearing kinds project; alias / undef /
    // store_accessor are different shelves (the consumer doesn't
    // benefit from squashing them all into Attribute). `Enum` is
    // intentionally excluded — it has its own `Class.enums` slot
    // populated by [`lift_enum`].
    let kept = matches!(
        a.kind,
        AttrKind::Attribute
            | AttrKind::AttrAccessor
            | AttrKind::AttrReader
            | AttrKind::AttrReadonly
            | AttrKind::StoreAttribute
            | AttrKind::Serialize
            | AttrKind::DefineAttributeMethod
    );
    if !kept {
        return None;
    }
    let mut out = Attribute::new(&a.name);
    out.type_name = a
        .options
        .iter()
        .find_map(|(k, v)| (k == "type" && !v.is_empty()).then(|| v.clone()));
    Some(out)
}

/// Lift a Rails `enum :status, { open: 0, closed: 1 }` declaration
/// to an OGAR [`EnumDecl`]. Returns `None` for non-Enum [`AttrDecl`]
/// kinds.
///
/// `ruff_ruby_spo::walk` drops the variant-list hash on the floor
/// (see the walker comment: "`enum :status, { active: 0 }` — 1 attr
/// (skip Hash)"), so the lifted `EnumDecl::source` is always
/// `EnumSource::Static(empty)` for Rails today. The column name is
/// still useful — downstream consumers know "this column is
/// enum-backed", even without the variant list. A future
/// `ruff_ruby_spo` enrichment could pass the variants through
/// `AttrDecl::options` and this lift would extend.
fn lift_enum(a: &AttrDecl) -> Option<EnumDecl> {
    if !matches!(a.kind, AttrKind::Enum) {
        return None;
    }
    let mut out = EnumDecl::new(&a.name);
    out.source = EnumSource::Static(Vec::new());
    Some(out)
}

// ───────────────────────────── scopes / default_scope ───────────────────

fn lift_scope(s: &ScopeDecl) -> Option<Scope> {
    // `Scope` only — the singular form has a body. `DefaultScope`
    // lives on a separate `Class.default_scope` slot. `Scopes`
    // (OpenProject's plural-list DSL `scopes :a, :b, :c`) is a
    // name-only predeclaration with no body; codex P2 on #52 flagged
    // that lifting it as a body-less `Scope` produces bogus
    // body-empty scope records — it should land in
    // `Class.scope_predeclarations` instead (handled by
    // [`lift_scope_predeclarations`]).
    if !matches!(s.kind, ScopeKind::Scope) {
        return None;
    }
    Some(Scope::new(&s.name, &s.body_ref))
}

/// Lift OpenProject's plural-list `scopes :a, :b, :c` DSL to OGAR's
/// `Class.scope_predeclarations` slot — a name-only predeclaration of
/// scope methods defined elsewhere (typically a mixin's `included`
/// block). Distinct from `Class.scopes` (which carries the lambda
/// body) per codex P2 on #52.
fn lift_scope_predeclarations(model: &Model) -> Vec<String> {
    model
        .scopes
        .iter()
        .filter(|s| matches!(s.kind, ScopeKind::Scopes))
        .map(|s| s.name.clone())
        .collect()
}

fn lift_default_scope(model: &Model) -> Option<String> {
    model
        .scopes
        .iter()
        .find(|s| matches!(s.kind, ScopeKind::DefaultScope))
        .map(|s| s.body_ref.clone())
}

// ───────────────────────────── callbacks ────────────────────────────────

fn lift_callback(cb: &RuffCallback) -> Callback {
    // Block-form callbacks (`before_save { ... }` / `do ... end`)
    // arrive with `target` empty (ruff drops the block body). Codex
    // P2 on #52: routing those through `Callback::method` sets
    // `target_method = Some("")`, which downstream emitters then
    // turn into an `ogar:targetMethod` triple with an empty object.
    // The correct OGAR shape is a block-form Callback with
    // `target_method = None` (body source `None` too, since ruff
    // doesn't extract block bodies for callbacks today). Construct
    // explicitly so neither side carries an empty placeholder.
    if cb.target.is_empty() {
        // `Callback` is `#[non_exhaustive]`, so struct-literal
        // construction isn't permitted. Build via `Default` +
        // mutation so target_method / body_source both stay `None`.
        let mut out = Callback::default();
        out.event = cb.phase.clone();
        return out;
    }
    Callback::method(&cb.phase, &cb.target)
}

// ───────────────────────────── validations ──────────────────────────────

fn lift_validation(v: &RuffValidation) -> Option<Validation> {
    // `normalizes :attr, with: ...` is semantically distinct from
    // `validates :attr, ...` — it's a write-time transformation, not
    // a constraint. OGAR `Validation` is the constraint shelf;
    // normalizes belongs elsewhere (no current OGAR slot). Skip.
    if matches!(v.kind, ValidationKind::Normalizes) {
        return None;
    }
    // Flatten kind + target + option keys into the `rule_source`
    // verbatim chunk — consumers can re-parse per ORM. The ruff side
    // already carries structured kinds via the `validation_kind` and
    // `validation_param` predicate triples for downstream schema
    // consumers; OGAR's Class shape stays opaque-rule-source.
    let mut rule = String::new();
    rule.push_str(match v.kind {
        ValidationKind::Validate => "validate",
        ValidationKind::Validates => "validates",
        ValidationKind::ValidatesAssociated => "validates_associated",
        ValidationKind::ValidatesEach => "validates_each",
        ValidationKind::Normalizes => unreachable!("filtered above"),
    });
    if !v.target.is_empty() {
        rule.push(' ');
        rule.push_str(&v.target);
    }
    for (k, val) in &v.options {
        rule.push_str(", ");
        rule.push_str(k);
        rule.push_str(": ");
        rule.push_str(val);
    }
    Some(Validation::new(&v.target, rule))
}

// ───────────────────────────── STI ──────────────────────────────────────

fn sti_parent(sti: &StiInfo) -> Option<String> {
    sti.inherits_from.clone()
}

/// Metabolize Rails STI facts into the agnostic [`Inheritance`] slot.
///
/// Priority: a declared parent makes it an [`Inheritance::Concrete`] child
/// regardless of other flags; `abstract_class` makes it
/// [`Inheritance::Abstract`]; an `inheritance_column` with no parent makes
/// it the [`Inheritance::RootedAt`] root of a hierarchy; otherwise
/// [`Inheritance::Root`]. Mixins / concerns are NOT consulted — they are a
/// separate axis (`Class.mixins`).
fn lift_inheritance(model: &Model) -> Inheritance {
    match model.sti.as_ref() {
        Some(sti) if sti.inherits_from.is_some() => Inheritance::Concrete {
            parent: sti.inherits_from.clone().expect("checked is_some"),
        },
        Some(sti) if sti.abstract_class => Inheritance::Abstract,
        Some(sti) if sti.inheritance_column.is_some() => Inheritance::RootedAt {
            root: model.name.clone(),
        },
        _ => Inheritance::Root,
    }
}

// ───────────────────────────── helpers ──────────────────────────────────

/// Strip ruby-source markers (quote pairs, leading symbol colon) from
/// an option value. `walk::format_hash_inline` in `ruff_ruby_spo`
/// renders these verbatim; consumers want the bare token.
fn strip_ruby_literal_markers(s: &str) -> &str {
    for q in ['"', '\''] {
        if let Some(inner) = s.strip_prefix(q).and_then(|t| t.strip_suffix(q)) {
            return inner;
        }
    }
    s.strip_prefix(':').unwrap_or(s)
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruff_spo_triplet::{
        ActsAs, AssocDecl, AttrDecl, Callback as RuffCallback, ConcernRef, Function, ScopeDecl,
        Validation as RuffValidation,
    };

    fn mk_model() -> Model {
        let mut m = Model::new("WorkPackage");
        m.associations.push(AssocDecl {
            kind: AssocKind::BelongsTo,
            name: "project".to_string(),
            options: vec![
                ("class_name".to_string(), "\"Project\"".to_string()),
                ("optional".to_string(), "true".to_string()),
                ("dependent".to_string(), ":destroy".to_string()),
            ],
        });
        m.associations.push(AssocDecl {
            kind: AssocKind::HasMany,
            name: "time_entries".to_string(),
            options: vec![("dependent".to_string(), ":delete_all".to_string())],
        });
        m.associations.push(AssocDecl {
            kind: AssocKind::AcceptsNestedAttributesFor,
            name: "comments".to_string(),
            options: vec![],
        });
        m.validations.push(RuffValidation {
            kind: ValidationKind::Validates,
            target: "subject".to_string(),
            options: vec![("presence".to_string(), "true".to_string())],
        });
        m.validations.push(RuffValidation {
            kind: ValidationKind::Normalizes,
            target: "email".to_string(),
            options: vec![],
        });
        m.callbacks.push(RuffCallback {
            phase: "before_save".to_string(),
            target: "set_status".to_string(),
            options: Vec::new(),
        });
        m.concerns.push(ConcernRef {
            kind: ConcernKind::Include,
            module: "Acts::Customizable".to_string(),
            body_ref: None,
        });
        m.concerns.push(ConcernRef {
            kind: ConcernKind::ClassMethodsBlock,
            module: String::new(),
            body_ref: None,
        });
        m.attributes.push(AttrDecl {
            kind: AttrKind::Attribute,
            name: "age".to_string(),
            options: vec![("type".to_string(), "integer".to_string())],
        });
        m.attributes.push(AttrDecl {
            kind: AttrKind::AliasAttribute,
            name: "new=orig".to_string(),
            options: vec![],
        });
        m.scopes.push(ScopeDecl {
            kind: ScopeKind::Scope,
            name: "active".to_string(),
            body_ref: "where(active: true)".to_string(),
        });
        m.scopes.push(ScopeDecl {
            kind: ScopeKind::DefaultScope,
            name: String::new(),
            body_ref: "order(:id)".to_string(),
        });
        m.acts_as.push(ActsAs {
            variant: "list".to_string(),
            options: vec![],
        });
        m.sti = Some(StiInfo {
            inherits_from: Some("Issue".to_string()),
            abstract_class: false,
            inheritance_column: Some("type".to_string()),
        });
        m
    }

    #[test]
    fn lift_model_carries_name_and_parent() {
        let class = lift_model(&mk_model());
        assert_eq!(class.name, "WorkPackage");
        assert_eq!(class.parent.as_deref(), Some("Issue"));
        assert!(matches!(class.language, Language::Ruby));
    }

    #[test]
    fn lift_inheritance_concrete_from_sti_parent() {
        // mk_model's StiInfo has inherits_from = Some("Issue").
        let class = lift_model(&mk_model());
        assert_eq!(
            class.inheritance,
            Inheritance::Concrete { parent: "Issue".to_string() },
        );
    }

    #[test]
    fn lift_inheritance_abstract_rooted_and_root() {
        // abstract_class → Abstract
        let mut m = Model::new("ApplicationRecord");
        m.sti = Some(StiInfo {
            inherits_from: None,
            abstract_class: true,
            inheritance_column: None,
        });
        assert_eq!(lift_model(&m).inheritance, Inheritance::Abstract);

        // inheritance_column, no parent → RootedAt(self): the STI root.
        let mut r = Model::new("Principal");
        r.sti = Some(StiInfo {
            inherits_from: None,
            abstract_class: false,
            inheritance_column: Some("type".to_string()),
        });
        assert_eq!(
            lift_model(&r).inheritance,
            Inheritance::RootedAt { root: "Principal".to_string() },
        );

        // no STI info at all → Root.
        assert_eq!(lift_model(&Model::new("Plain")).inheritance, Inheritance::Root);
    }

    #[test]
    fn lift_associations_drops_accepts_nested_and_parses_options() {
        let class = lift_model(&mk_model());
        assert_eq!(class.associations.len(), 2);
        let project = &class.associations[0];
        assert_eq!(project.kind, AssociationKind::BelongsTo);
        assert_eq!(project.name, "project");
        // Quote-stripped class_name.
        assert_eq!(project.class_name.as_deref(), Some("Project"));
        // Boolean parsed.
        assert_eq!(project.optional, Some(true));
        // Symbol-prefix-stripped dependent.
        assert_eq!(project.dependent.as_deref(), Some("destroy"));
        let time_entries = &class.associations[1];
        assert_eq!(time_entries.kind, AssociationKind::HasMany);
        assert_eq!(time_entries.dependent.as_deref(), Some("delete_all"));
    }

    #[test]
    fn lift_validations_keeps_validates_drops_normalizes() {
        let class = lift_model(&mk_model());
        assert_eq!(class.validations.len(), 1);
        let v = &class.validations[0];
        assert_eq!(v.target, "subject");
        assert!(v.rule_source.starts_with("validates subject"));
        assert!(v.rule_source.contains("presence: true"));
    }

    #[test]
    fn lift_callbacks_carries_phase_and_target() {
        let class = lift_model(&mk_model());
        assert_eq!(class.callbacks.len(), 1);
        assert_eq!(class.callbacks[0].event, "before_save");
        assert_eq!(class.callbacks[0].target_method.as_deref(), Some("set_status"));
    }

    #[test]
    fn lift_mixins_carries_include_and_acts_as_prefix() {
        let class = lift_model(&mk_model());
        assert!(class.mixins.contains(&"Acts::Customizable".to_string()));
        assert!(class.mixins.contains(&"acts_as_list".to_string()));
        // Block markers don't leak.
        assert!(!class.mixins.iter().any(String::is_empty));
    }

    #[test]
    fn lift_attributes_keeps_real_kinds_with_type_option() {
        let class = lift_model(&mk_model());
        assert_eq!(class.attributes.len(), 1);
        assert_eq!(class.attributes[0].name, "age");
        assert_eq!(class.attributes[0].type_name.as_deref(), Some("integer"));
    }

    #[test]
    fn lift_scopes_separates_named_from_default() {
        let class = lift_model(&mk_model());
        assert_eq!(class.scopes.len(), 1);
        assert_eq!(class.scopes[0].name, "active");
        assert_eq!(class.scopes[0].body_source, "where(active: true)");
        assert_eq!(class.default_scope.as_deref(), Some("order(:id)"));
    }

    /// **Codex P2 on #52** — OpenProject's plural-list `scopes :a, :b`
    /// DSL emits `ScopeKind::Scopes` (no body). These are name-only
    /// pre-declarations and must land in `Class.scope_predeclarations`,
    /// NOT in `Class.scopes` as body-less Scope records.
    #[test]
    fn lift_scope_predeclarations_routes_plural_form() {
        let mut m = Model::new("WorkPackage");
        m.scopes.push(ScopeDecl {
            kind: ScopeKind::Scope,
            name: "active".to_string(),
            body_ref: "where(active: true)".to_string(),
        });
        m.scopes.push(ScopeDecl {
            kind: ScopeKind::Scopes,
            name: "visible".to_string(),
            body_ref: String::new(),
        });
        m.scopes.push(ScopeDecl {
            kind: ScopeKind::Scopes,
            name: "pending".to_string(),
            body_ref: String::new(),
        });
        let class = lift_model(&m);
        // Only the singular `Scope` ends up in scopes.
        assert_eq!(class.scopes.len(), 1);
        assert_eq!(class.scopes[0].name, "active");
        // The plural list lands in scope_predeclarations.
        assert_eq!(
            class.scope_predeclarations,
            vec!["visible".to_string(), "pending".to_string()],
        );
    }

    /// **Codex P2 on #52** — Rails `enum :status, { open: 0,
    /// closed: 1 }` declarations must lift to `Class.enums` as
    /// [`EnumDecl`], NOT to `Class.attributes`. Downstream emitters
    /// read enum constraints from `Class.enums`; misrouting drops
    /// the enum semantics on the floor.
    #[test]
    fn lift_enums_routes_attr_kind_enum_to_enums_slot() {
        let mut m = Model::new("WorkPackage");
        m.attributes.push(AttrDecl {
            kind: AttrKind::Enum,
            name: "status".to_string(),
            options: vec![],
        });
        m.attributes.push(AttrDecl {
            kind: AttrKind::Attribute,
            name: "age".to_string(),
            options: vec![("type".to_string(), "integer".to_string())],
        });
        let class = lift_model(&m);
        // Enum lifts to enums slot only.
        assert_eq!(class.enums.len(), 1);
        assert_eq!(class.enums[0].column, "status");
        assert!(matches!(class.enums[0].source, EnumSource::Static(ref v) if v.is_empty()));
        // Attribute lifts to attributes slot only — enum does NOT
        // double-emit.
        assert_eq!(class.attributes.len(), 1);
        assert_eq!(class.attributes[0].name, "age");
        assert!(!class.attributes.iter().any(|a| a.name == "status"));
    }

    /// **Codex P2 on #52** — block-form callbacks (`before_save { ... }`
    /// or `before_save do ... end`) arrive with `target` empty.
    /// Routing them through `Callback::method` produces
    /// `target_method = Some("")`, which downstream emitters turn into
    /// `ogar:targetMethod ""` — bogus. The lift must yield a Callback
    /// with `target_method = None` for the block form.
    #[test]
    fn lift_callback_block_form_yields_no_target_method() {
        let mut m = Model::new("WorkPackage");
        m.callbacks.push(RuffCallback {
            phase: "before_save".to_string(),
            target: "set_status".to_string(),
            options: Vec::new(),
        });
        m.callbacks.push(RuffCallback {
            phase: "after_create".to_string(),
            target: String::new(),
            options: Vec::new(),
        });
        let class = lift_model(&m);
        assert_eq!(class.callbacks.len(), 2);
        // Method-form callback: target_method populated.
        assert_eq!(class.callbacks[0].event, "before_save");
        assert_eq!(
            class.callbacks[0].target_method.as_deref(),
            Some("set_status"),
        );
        // Block-form callback: target_method is None (not
        // Some("")) — no bogus empty-method-name triple downstream.
        assert_eq!(class.callbacks[1].event, "after_create");
        assert_eq!(class.callbacks[1].target_method, None);
        assert_eq!(class.callbacks[1].body_source, None);
    }

    #[test]
    fn lift_model_graph_preserves_order() {
        let mut graph = ModelGraph::new("openproject");
        graph.models.push(mk_model());
        graph.models.push(Model::new("Project"));
        let classes = lift_model_graph(&graph);
        assert_eq!(classes.len(), 2);
        assert_eq!(classes[0].name, "WorkPackage");
        assert_eq!(classes[1].name, "Project");
    }

    fn mk_model_with_functions() -> Model {
        let mut m = Model::new("WorkPackage");
        m.functions.push(Function {
            name: "set_status".to_string(),
            reads: vec!["status".to_string()],
            raises: Vec::new(),
            traverses: Vec::new(),
        });
        m.functions.push(Function {
            name: "close!".to_string(),
            reads: Vec::new(),
            raises: vec!["ArgumentError".to_string()],
            traverses: Vec::new(),
        });
        m
    }

    #[test]
    fn lift_actions_emits_one_def_per_function() {
        let acts = lift_actions(&mk_model_with_functions());
        assert_eq!(acts.len(), 2);
    }

    #[test]
    fn lift_actions_predicate_object_class_and_identity() {
        let acts = lift_actions(&mk_model_with_functions());
        assert_eq!(acts[0].predicate, "set_status");
        assert_eq!(acts[0].object_class, "WorkPackage");
        assert_eq!(acts[0].identity, "WorkPackage::action_def::set_status");
        assert_eq!(acts[1].predicate, "close!");
        assert_eq!(acts[1].identity, "WorkPackage::action_def::close!");
    }

    /// Facts-only: no exec policy (no such slot on the vocab `ActionDef`),
    /// no causal claim derived from `reads` (a plain Rails read is not a
    /// reactive dependency), no body (ruff captures no method-body text),
    /// no decorators (Rails functions carry none in the ruff IR).
    #[test]
    fn lift_actions_is_facts_only() {
        let acts = lift_actions(&mk_model_with_functions());
        let a = &acts[0];
        assert!(a.kausal.is_none(), "reads must NOT become a causal dependency");
        assert!(a.body_source.is_none());
        assert!(a.decorators.is_empty());
    }

    #[test]
    fn lift_actions_empty_functions_yields_empty() {
        assert!(lift_actions(&Model::new("Empty")).is_empty());
    }

    #[test]
    fn strip_ruby_literal_markers_handles_all_source_forms() {
        assert_eq!(strip_ruby_literal_markers("\"User\""), "User");
        assert_eq!(strip_ruby_literal_markers("'User'"), "User");
        assert_eq!(strip_ruby_literal_markers(":User"), "User");
        assert_eq!(strip_ruby_literal_markers("User"), "User");
        // Mismatched / partial — pass-through.
        assert_eq!(strip_ruby_literal_markers("\"User"), "\"User");
    }
}
