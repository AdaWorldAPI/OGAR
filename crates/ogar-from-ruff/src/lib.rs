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
//! | `inherits`             | `mixins` (appended)           | Odoo `_inherit` multi-parent mixin composition — the vocab's `mixins` doc names `_inherit`; the `inheritance` axis excludes mixins. Frontend-agnostic field, populated only by the Odoo frontend |
//! | `functions`            | `Vec<ActionDef>` (DO-arm)     | [`lift_actions`] — one `ActionDef` per method; standalone, not on `Class` |
//! | `fields`               | `attributes` / `associations` / `computed_fields` | the D-AR-3.5 physical schema stratum (migration-DSL columns for Rails, and `db.Column(...)` for Flask-SQLAlchemy, both via [`project_total_schema_fields`]; `fields.X(...)` declarations for Odoo via [`project_odoo_fields`]); `not_null` wires `AttributeOptions::required` (Rails/SQLAlchemy only); a `<name>_id` FK column is skipped when the model also declares an association named `<name>` (double-strata dedup) |
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

pub mod emit;
pub mod mint;
pub mod sqlalchemy; // WS-G-D

use ogar_vocab::{
    canonical_concept, ActionDef, Association, AssociationKind, Attribute, Callback, Class,
    ComputedField, EnumDecl, EnumSource, Inheritance, Language, Scope, Validation,
};
use ruff_spo_triplet::{
    AssocDecl, AssocKind, AttrDecl, AttrKind, Callback as RuffCallback, ConcernKind, Model,
    ModelGraph, ScopeDecl, ScopeKind, StiInfo, Validation as RuffValidation, ValidationKind,
};

/// Lift every model in a [`ModelGraph`] to an OGAR [`Class`] (Rails /
/// `ruff_ruby_spo` producer — [`Language::Ruby`]). Output preserves
/// declaration order so downstream consumers can rely on deterministic
/// ordering for snapshot tests.
#[must_use]
pub fn lift_model_graph(graph: &ModelGraph) -> Vec<Class> {
    lift_model_graph_with_language(graph, Language::Ruby)
}

/// Lift a whole [`ModelGraph`] for a Python / Odoo producer
/// (`ruff_python_spo`) — like [`lift_model_graph`] but stamps each class as
/// [`Language::Python`]. The `odoo` namespace already routes to the `erp`
/// source domain via the same `classify_domain` path.
#[must_use]
pub fn lift_model_graph_python(graph: &ModelGraph) -> Vec<Class> {
    lift_model_graph_with_language(graph, Language::Python)
}

fn lift_model_graph_with_language(graph: &ModelGraph, language: Language) -> Vec<Class> {
    let domain = classify_domain(&graph.namespace);
    let concept_domain = domain.as_deref().and_then(ogar_vocab::source_domain_concept);
    // The harvest namespace IS the curator id (`"openproject"`,
    // `"redmine"`, `"odoo"`, …). `source_domain` is the coarse bucket it
    // maps to; `source_curator` keeps the specific product so two curators
    // in the same domain (Redmine + OpenProject are both `project`) stay
    // distinguishable downstream.
    let curator = if graph.namespace.is_empty() {
        None
    } else {
        Some(graph.namespace.clone())
    };
    graph
        .models
        .iter()
        .map(|m| {
            let mut class = lift_model_with_language(m, language);
            class.source_domain = domain.clone();
            class.source_curator = curator.clone();
            // Domain-gate the canonical concept. `lift_model` resolves
            // domain-blind (an all-domains best guess); here we know the
            // curator's domain, so re-resolve through the gate to withhold a
            // promotion whose codebook domain doesn't match — a generic
            // `Role` in a non-project curator must stay `role`, not become
            // `project_role` (codex P2 on #72). Cross-domain bridges like
            // `billable_work_entry` are exempt and still converge.
            class.canonical_concept =
                Some(ogar_vocab::canonical_concept_in_domain(&m.name, concept_domain));
            class
        })
        .collect()
}

/// Name the curator **domain** from the harvest namespace — the "one tiny
/// regex" that tags OpenProject as a `project` domain and Odoo as an `erp`
/// domain. A coarse, curator-agnostic label (a `ClassFingerprint`
/// component), not the namespace itself. Returns `None` for unrecognized
/// namespaces — the domain stays unset rather than guessed.
fn classify_domain(namespace: &str) -> Option<String> {
    let ns = namespace.to_ascii_lowercase();
    if ns.contains("openproject") || ns.contains("redmine") {
        Some("project".to_string())
    } else if ns.contains("odoo") {
        Some("erp".to_string())
    } else if ns.contains("woa") || ns.contains("smb") {
        // WoA-rs / SMB — the German-ERP sanity witness adapter.
        Some("german-erp".to_string())
    } else {
        None
    }
}

/// Lift one [`Model`] to an OGAR [`Class`] stamped as [`Language::Ruby`]
/// (the Rails / `ruff_ruby_spo` producer). Pure projection — no I/O.
///
/// For the Python / Odoo producer (`ruff_python_spo`) use
/// [`lift_model_python`]. Both delegate to the same projection and differ
/// only in the language discriminant. Language is set explicitly rather
/// than guessed from `ModelGraph::namespace`, because the namespace
/// (`openproject`, `odoo`, …) doesn't bind to the producer language
/// one-to-one.
#[must_use]
pub fn lift_model(model: &Model) -> Class {
    lift_model_with_language(model, Language::Ruby)
}

/// Lift one [`Model`] to an OGAR [`Class`] stamped as [`Language::Python`]
/// — the Odoo / Django producer path (`ruff_python_spo`). Identical
/// projection to [`lift_model`]; only the language discriminant differs.
#[must_use]
pub fn lift_model_python(model: &Model) -> Class {
    lift_model_with_language(model, Language::Python)
}

fn lift_model_with_language(model: &Model, language: Language) -> Class {
    let mut class = Class::new(&model.name);
    class.language = language;
    class.parent = model.sti.as_ref().and_then(sti_parent);
    class.inheritance = lift_inheritance(model);
    class.canonical_concept = Some(canonical_concept(&model.name));
    class.associations = model.associations.iter().filter_map(lift_association).collect();
    class.mixins = lift_mixins(model);
    // Odoo `_inherit` (multi-parent mixin composition) lands on the same
    // mixins shelf the vocab designates for it — `Class::mixins` doc names
    // `_inherit = 'mixin.thread'`, and `Class::inheritance` explicitly
    // excludes mixins ("Mixins / concerns are a SEPARATE axis"). The ruff
    // frontend already normalised the names (dot→underscore→verbatim),
    // deduped, and excluded the bare-`_inherit` reopen self-edge. Only the
    // Odoo frontend populates `Model::inherits`, so this is a no-op for the
    // Rails (`sti`) and C++ (`bases`) producers — hence unconditional.
    class.mixins.extend(model.inherits.iter().cloned());
    class.attributes = model.attributes.iter().filter_map(lift_attribute).collect();
    class.enums = model.attributes.iter().filter_map(lift_enum).collect();
    class.scopes = model.scopes.iter().filter_map(lift_scope).collect();
    class.scope_predeclarations = lift_scope_predeclarations(model);
    class.callbacks = model.callbacks.iter().map(lift_callback).collect();
    class.validations = model.validations.iter().filter_map(lift_validation).collect();
    class.default_scope = lift_default_scope(model);
    // Rails carries its DECLARED schema in the AR-DSL vectors lifted above
    // (`attribute :x, :type`, `belongs_to :y`); an Odoo model instead
    // declares everything as `fields.X(...)`, which lands in the core-7
    // `Model::fields` vector. Both frontends ALSO populate `Model::fields`
    // with the D-AR-3.5 *physical* schema stratum (Rails: the migration DSL
    // columns via `ruff_ruby_spo::extract_app_with_schema`; Odoo: the same
    // vector doubles as the declared schema since Odoo has no separate
    // AR-DSL). So each language gets its own field-projection pass —
    // `project_odoo_fields` for Python/Odoo (Codex P1, PR #131), and
    // `project_total_schema_fields` for Ruby (falsifier #1 gap-close,
    // D-PARITY-PROBE-WP-1; also shared by the SQLAlchemy producer, see
    // `sqlalchemy.rs`) — so neither producer silently drops the schema
    // stratum lifted by its frontend.
    match language {
        Language::Python => project_odoo_fields(&mut class, model),
        Language::Ruby => project_total_schema_fields(&mut class, model),
        _ => {}
    }
    class
}

/// Project an Odoo model's core-7 [`Model::fields`] onto the
/// schema-carrying [`Class`] columns. Python-only: Rails models keep their
/// schema in `model.attributes` / `model.associations` (lifted separately),
/// and ALSO populate `model.fields` (DB columns), so projecting fields for
/// Rails would double-count. Odoo leaves the AR vectors empty and puts
/// everything in `fields`.
///
/// Per-field mapping:
/// - relational field (`target` set) → [`Association`]; the kind comes from
///   the field's cardinality (`relation_kind`), `class_name` is the raw
///   comodel, `inverse_of` the One2many inverse.
/// - non-relational field → [`Attribute`] with `type_name` set from the SPO
///   `Field`'s `field_type` (the lowercased Odoo constructor — `char` /
///   `integer` / `monetary` / …), so the emitters pick a concrete wrapper type
///   instead of the untyped `OgScalar` fallback.
/// - compute field (`emitted_by` set) → [`ComputedField`] (method +
///   `@api.depends`), in addition to its Attribute / Association above.
fn project_odoo_fields(class: &mut Class, model: &Model) {
    for field in &model.fields {
        if let Some(comodel) = &field.target {
            let kind = odoo_relation_kind(field.relation_kind.as_deref(), field.inverse_name.is_some());
            let mut assoc = Association::new(kind, &field.name);
            assoc.class_name = Some(comodel.clone());
            assoc.inverse_of = field.inverse_name.clone();
            class.associations.push(assoc);
        } else {
            let mut attr = Attribute::new(&field.name);
            // Carry the Odoo constructor (field_type) so the emitters can pick a
            // concrete wrapper type (OgStr/OgInt/OgMoney/…) instead of the
            // untyped OgScalar fallback. None for a field whose type ruff did
            // not capture → OgScalar (the safe default).
            attr.type_name = field.field_type.clone();
            class.attributes.push(attr);
        }
        if let Some(compute_method) = &field.emitted_by {
            let mut computed = ComputedField::new(&field.name, compute_method);
            computed.depends = field.depends_on.clone();
            class.computed_fields.push(computed);
        }
    }
}

/// Project a total-nullability model's D-AR-3.5 schema stratum — the
/// PHYSICAL DB columns in `Model::fields` — onto the schema-carrying
/// [`Class`] columns. Shared by two producers:
/// - **Rails** (Ruby): populated by `ruff_ruby_spo::extract_app_with_schema`
///   from the `db/migrate/tables/*.rb` migration DSL. Counterpart of
///   [`project_odoo_fields`]: the AR-DSL vectors (`attributes` /
///   `associations`, from `attribute :x, :type` / `belongs_to :y`) carry the
///   *declared* schema and are lifted separately, earlier in
///   [`lift_model_with_language`]; `fields` carries the *physical* schema
///   (what the migration actually created). Both strata are real and both
///   must reach the [`Class`], or the Rails lift silently drops the physical
///   column set the way the pre-fix Python-only gate did (falsifier #1 /
///   D-PARITY-PROBE-WP-1).
/// - **Flask-SQLAlchemy** (Python/WoA): populated via
///   [`crate::sqlalchemy::lift_model_sqlalchemy`]'s `db.Column(...)`
///   declarations, which are likewise total (every column's nullability is a
///   declared fact) — see that module's doc for why it routes `Language::Python`
///   through this helper instead of [`project_odoo_fields`].
///
/// Per-field mapping (mirrors [`project_odoo_fields`]):
/// - relational field (`target` set) → [`Association`]; same shape as the
///   Odoo path. `ruff_ruby_spo::schema` never sets `target` today (the
///   migration DSL has no relation-aware column form), so this arm is
///   dormant for Rails until a future relation-aware schema pass — kept
///   for shape-parity with Odoo rather than speculative dead code.
/// - scalar field named `<name>_id` where the model ALSO declares an AR-DSL
///   association named `<name>` (already lifted onto `class.associations`
///   earlier in [`lift_model_with_language`]) → **skipped**. The FK column
///   (physical stratum) and the declared `belongs_to`/`has_many` (declared
///   stratum) are the SAME relation seen twice; projecting both would
///   double-report it as `<name>_id: OgInt` (the ORM spelling) AND
///   `<name>: ToOne<X>` (the AR spelling) on the same [`Class`]. The canon
///   keeps the AR spelling — see [`is_fk_shadowed_by_association`]. The
///   literal primary key `id` never matches this pattern (it carries no
///   `_id`-suffixed prefix of its own) and is always kept.
/// - other scalar field → [`Attribute`] with `type_name` from
///   `Field::field_type` (the migration DSL type token verbatim: `string`,
///   `bigint`, `integer`, …), and `AttributeOptions::required` wired from
///   `Field::not_null`: `Some(true)` (`null: false` in the migration) maps
///   to `Some(true)`; `None` (nullable — Rails' column default, and the only
///   other value `not_null` carries per its own doc) maps to `Some(false)`.
///   The schema stratum is total knowledge here — every column has a real
///   nullability, so absence of `null: false` IS a positive "nullable" fact,
///   not "unknown" the way an Odoo `required=` kwarg's absence would be.
/// - compute field (`emitted_by` set — the D-AR-3.5 compute-linkage pass,
///   `ruff_ruby_spo::schema::link_computed_fields`) → [`ComputedField`],
///   same as the Odoo path.
pub(crate) fn project_total_schema_fields(class: &mut Class, model: &Model) {
    for field in &model.fields {
        if let Some(comodel) = &field.target {
            let kind = odoo_relation_kind(field.relation_kind.as_deref(), field.inverse_name.is_some());
            let mut assoc = Association::new(kind, &field.name);
            assoc.class_name = Some(comodel.clone());
            assoc.inverse_of = field.inverse_name.clone();
            class.associations.push(assoc);
        } else if !is_fk_shadowed_by_association(&field.name, class)
            && !class.attributes.iter().any(|a| a.name == field.name)
        {
            // The second guard (no already-lifted attribute of the same
            // name) closes PR #156 finding (c): a model that both declares
            // `attribute :foo, :string` (AR-DSL, lifted earlier in
            // `lift_model_with_language`) AND has a physical `foo` column
            // must not end up with two `foo` struct fields. On collision,
            // the AR-DSL declaration wins (it is the declared truth; the
            // physical column is the same field seen from the migration) —
            // we skip the physical duplicate rather than merge/overwrite,
            // since overwriting would change `type_name` provenance and
            // risk drift. Type reconciliation on collision is a follow-up,
            // not done here.
            let mut attr = Attribute::new(&field.name);
            attr.type_name = field.field_type.clone();
            // Schema stratum is total: null:false -> required; the Rails
            // default (nullable, `not_null == None`) -> explicitly optional.
            attr.options.required = Some(field.not_null.unwrap_or(false));
            class.attributes.push(attr);
        }
        if let Some(compute_method) = &field.emitted_by {
            let mut computed = ComputedField::new(&field.name, compute_method);
            computed.depends = field.depends_on.clone();
            class.computed_fields.push(computed);
        }
    }
}

/// FK-dedup predicate for [`project_total_schema_fields`]. Two independent
/// shadow rules, checked in order:
///
/// 1. **Explicit `foreign_key:`** — `class` carries an [`Association`] whose
///    declared `foreign_key` equals `field_name` verbatim (e.g.
///    `belongs_to :author, foreign_key: "user_id"` shadows the physical
///    `user_id` column even though the association's own name is `author`,
///    not `user`). This is the authoritative signal (PR #156 finding (b)) —
///    checked first.
/// 2. **`<name>_id` naming convention** — `field_name` has the shape
///    `<name>_id` for a non-empty `<name>`, AND `class` already carries an
///    [`Association`] named `<name>` (from the AR-DSL vector, lifted before
///    this function runs in [`lift_model_with_language`]). The bare `id`
///    primary key column never matches either rule — `"id"` has no
///    `"_id"`-suffixed prefix of its own, and no real corpus association
///    declares `foreign_key: "id"` — and is always kept as a scalar
///    attribute.
pub(crate) fn is_fk_shadowed_by_association(field_name: &str, class: &Class) -> bool {
    if class
        .associations
        .iter()
        .any(|a| a.foreign_key.as_deref() == Some(field_name))
    {
        return true;
    }
    field_name
        .strip_suffix("_id")
        .filter(|prefix| !prefix.is_empty())
        .is_some_and(|prefix| class.associations.iter().any(|a| a.name == prefix))
}

/// Map an Odoo relation cardinality to the canonical [`AssociationKind`].
///
/// `relation_kind` (`many2one` / `one2many` / `many2many`, from ruff's
/// `relation_kind` predicate) is the authoritative signal. The
/// `has_inverse` fallback covers the theoretical case of a relational field
/// with no recorded cardinality: an inverse implies a One2many, otherwise
/// the to-one default `BelongsTo`. `target` + `inverse_name` alone cannot
/// separate a Many2one from a Many2many — both are comodel-only with no
/// inverse — which is exactly why `relation_kind` exists.
fn odoo_relation_kind(relation_kind: Option<&str>, has_inverse: bool) -> AssociationKind {
    match relation_kind {
        Some("many2one") => AssociationKind::BelongsTo,
        Some("one2many") => AssociationKind::HasMany,
        Some("many2many") => AssociationKind::HasAndBelongsToMany,
        _ if has_inverse => AssociationKind::HasMany,
        _ => AssociationKind::BelongsTo,
    }
}

// ───────────────────────── ProjectWorkItem role projection ─────────────
//
// Curator-side mapping: project_work_item's canonical roles vs the
// Rails-AR-dialect names that surface them. Used by the lineage-transcode
// smoke (Redmine Issue <-> OpenProject WorkPackage through the canonical
// bridge) — both curators must project losslessly onto the same role set.
//
// Synonyms are deliberate and minimal: only well-known Rails-AR variants
// of the canonical role appear here. Adding a new synonym is the right
// move when a real corpus surfaces it; inventing synonyms isn't.

/// Map a Rails-curator association name to the canonical
/// [`ogar_vocab::project_work_item`] role it realises. Returns `None`
/// when the association is not part of the project-work-item canonical
/// surface (e.g. Redmine `fixed_version`, OP `file_links`).
#[must_use]
pub fn project_work_item_role(curator_name: &str) -> Option<&'static str> {
    match curator_name {
        "project" => Some("project"),
        "status" => Some("status"),
        "tracker" | "type" => Some("type"),
        "priority" => Some("priority"),
        "author" => Some("author"),
        "assigned_to" | "assignee" | "responsible" => Some("assignee"),
        "journals" => Some("journals"),
        "relations" | "relations_from" | "relations_to" => Some("relations"),
        "time_entries" => Some("time_entries"),
        _ => None,
    }
}

/// Map a mixin / `acts_as_*` name to a canonical
/// [`ogar_vocab::project_work_item`] role it provides indirectly. OP's
/// WorkPackage carries `journals` via `acts_as_journalized` and
/// `relations` via the `WorkPackages::Relations` concern instead of
/// direct `has_many` calls; this resolver recovers them so the canonical
/// projection is total.
#[must_use]
pub fn project_work_item_role_from_mixin(mixin: &str) -> Option<&'static str> {
    if mixin == "acts_as_journalized" || mixin.ends_with("::Journalized") {
        return Some("journals");
    }
    if mixin == "WorkPackages::Relations" || mixin.ends_with("::Relations") {
        return Some("relations");
    }
    None
}

/// The set of canonical [`ogar_vocab::project_work_item`] roles a curator
/// class projects onto. Unions association-derived roles
/// ([`project_work_item_role`]) with mixin-derived roles
/// ([`project_work_item_role_from_mixin`]). Returns the empty set when
/// the class has no project-work-item shape.
#[must_use]
pub fn project_work_item_canonical_roles(
    class: &Class,
) -> std::collections::HashSet<&'static str> {
    let mut set = std::collections::HashSet::new();
    for a in &class.associations {
        if let Some(role) = project_work_item_role(&a.name) {
            set.insert(role);
        }
    }
    for m in &class.mixins {
        if let Some(role) = project_work_item_role_from_mixin(m) {
            set.insert(role);
        }
    }
    set
}

// ───────────────────────── Project role projection ─────────────────────
//
// The Project canonical class (ogar_vocab::project) has 4 family edges:
// parent, work_items, time_entries, members. Curators surface them under
// varying Rails-AR names — Redmine `issues` vs OP `work_packages` for the
// work-item set; both spell `members`/`users`/`memberships` for the actor
// set. This resolver normalises curator names onto the canonical roles
// so the lineage-transcode claim holds for `Project` too.

/// Map a Rails-curator association name to the canonical
/// [`ogar_vocab::project`] role it realises. Returns `None` when the
/// association is not part of the project canonical surface (e.g. Redmine
/// `news`, OP `forums` — real but not yet promoted into the canonical
/// shape; future PRs may extend if cross-curator evidence accumulates).
#[must_use]
pub fn project_role(curator_name: &str) -> Option<&'static str> {
    match curator_name {
        "parent" => Some("parent"),
        "issues" | "work_packages" => Some("work_items"),
        "time_entries" => Some("time_entries"),
        // The actor set is reached via multiple AR through-associations
        // in both curators: members / memberships / users (Redmine + OP) /
        // member_principals / principals (OP only — OP adds the
        // through-Principal hop).
        "members" | "memberships" | "users" | "member_principals" | "principals" => {
            Some("members")
        }
        _ => None,
    }
}

/// The set of canonical [`ogar_vocab::project`] roles a curator class
/// projects onto. Pure association-derived for v1 — `project` carries no
/// mixin-borne roles in either Redmine or OP today (the mixin layer
/// only contributes to `project_work_item`'s journals / relations).
#[must_use]
pub fn project_canonical_roles(class: &Class) -> std::collections::HashSet<&'static str> {
    let mut set = std::collections::HashSet::new();
    for a in &class.associations {
        if let Some(role) = project_role(&a.name) {
            set.insert(role);
        }
    }
    set
}

// ───────────────────────────── actions (DO-arm) ─────────────────────────

/// Lift a [`Model`]'s methods to OGAR [`ActionDef`] declarations — the
/// DO-arm of the harvest. One `ActionDef` per [`ruff_spo_triplet::Function`].
///
/// This is a **facts-only** lift: it sets the action's `identity`,
/// `predicate` (the method name — already snake_case on the Rails side),
/// `object_class` (the model name), and the `reads` / `writes` / `calls`
/// effect annotations verbatim from [`ruff_spo_triplet::Function`] (OGAR-AS-IR
/// §3 test 2 — "each `ActionDef` declares what it reads, what it writes, what
/// side effects it has… pure-vs-effectful is a type, not a comment"). It
/// deliberately does **not**:
///
/// - set any execution policy (the vocab [`ActionDef`] has no `exec` slot;
///   backend routing is consumer-private — see the OP-arm plan §5.2),
/// - construct an `ActionInvocation` (a live-instance carrier: no target
///   instance / cycle / trace exists at harvest time),
/// - populate `kausal` from [`ruff_spo_triplet::Function`]`::reads` — a
///   plain Rails method reading a field is **not** a reactive
///   `@api.depends`-style trigger, so claiming one would leak method-body
///   description into causal semantics. `reads` (and `writes` / `calls`) now
///   ride `ActionDef` as **effect annotations** (see above) — that is a
///   distinct, weaker claim than a `KausalSpec::Depends` reactive trigger,
///   which stays `None` here. `raises` / `traverses` have no `ActionDef`
///   slot yet and still ride the narrow / SPO arm as triples only.
///
/// The result is registry-ready: guard / RBAC / `exec` enrichment happens
/// downstream at registration, not in the producer.
#[must_use]
pub fn lift_actions(model: &Model) -> Vec<ActionDef> {
    // Public actions AND non-public helpers (AT-CARRY-1b, review on #164):
    // since ruff #45 the Ruby walker splits private/protected defs into
    // `Model::helpers` — and Rails lifecycle hook targets are conventionally
    // private (Redmine measurement: 67/84 hook targets live there). The W3.3
    // delete-blocking rows ARE those hook bodies, so the DO-arm must carry
    // both. Consistent with this fn's contract: the producer emits effect
    // annotations; routability / guard / RBAC enrichment happens downstream
    // at registration (a registrar can re-join `Model::callbacks` by name to
    // tell hook targets from routable actions).
    model
        .functions
        .iter()
        .chain(model.helpers.iter())
        .map(|f| {
            let mut a = ActionDef::new(
                format!("{}::action_def::{}", model.name, f.name),
                &f.name,
                &model.name,
            );
            a.reads = f.reads.clone();
            a.writes = f.writes.clone();
            a.calls = f.calls.clone();
            a
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
        ActsAs, AssocDecl, AttrDecl, Callback as RuffCallback, ConcernRef, Field, Function,
        ScopeDecl, Validation as RuffValidation,
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
    fn lift_model_python_stamps_python_language() {
        // The Python/Odoo producer path: same projection, Python discriminant.
        let class = lift_model_python(&mk_model());
        assert!(matches!(class.language, Language::Python));
        assert_eq!(class.name, "WorkPackage");
        assert_eq!(class.parent.as_deref(), Some("Issue"));
    }

    #[test]
    fn lift_model_graph_python_stamps_python_and_keeps_erp_domain() {
        // An Odoo ModelGraph (namespace "odoo") lifts as Python and routes to
        // the `erp` source domain / `odoo` curator via classify_domain.
        let mut graph = ModelGraph::new("odoo");
        graph.models.push(Model::new("account_move"));
        let classes = lift_model_graph_python(&graph);
        assert_eq!(classes.len(), 1);
        assert!(matches!(classes[0].language, Language::Python));
        assert_eq!(classes[0].source_domain.as_deref(), Some("erp"));
        assert_eq!(classes[0].source_curator.as_deref(), Some("odoo"));
    }

    /// An Odoo-shape model: schema lives entirely in `Model::fields`
    /// (the AR-DSL vectors are empty, as `ruff_python_spo` produces).
    fn mk_odoo_model() -> Model {
        let mut m = Model::new("account_move");
        m.fields.push(Field {
            name: "name".to_string(),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "partner_id".to_string(),
            target: Some("res.partner".to_string()),
            relation_kind: Some("many2one".to_string()),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "line_ids".to_string(),
            target: Some("account.move.line".to_string()),
            inverse_name: Some("move_id".to_string()),
            relation_kind: Some("one2many".to_string()),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "tag_ids".to_string(),
            target: Some("account.analytic.tag".to_string()),
            relation_kind: Some("many2many".to_string()),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "amount_total".to_string(),
            emitted_by: Some("_compute_amount".to_string()),
            depends_on: vec!["line_ids.balance".to_string()],
            ..Default::default()
        });
        m
    }

    #[test]
    fn lift_model_python_projects_odoo_fields() {
        // Codex P1 (#131): the Python lift must project `Model::fields` or the
        // class loses its whole schema. Scalar → attribute, relational →
        // association (kind from relation_kind), compute → computed_field.
        let class = lift_model_python(&mk_odoo_model());

        // Scalar + computed fields surface as attributes (named columns).
        let attr_names: Vec<&str> = class.attributes.iter().map(|a| a.name.as_str()).collect();
        assert!(attr_names.contains(&"name"));
        assert!(attr_names.contains(&"amount_total"));
        // Relational fields are associations, not attributes.
        assert!(!attr_names.contains(&"partner_id"));
        assert!(!attr_names.contains(&"line_ids"));

        // relation_kind drives the AssociationKind; comodel → class_name.
        let partner = class
            .associations
            .iter()
            .find(|a| a.name == "partner_id")
            .expect("partner_id association");
        assert_eq!(partner.kind, AssociationKind::BelongsTo);
        assert_eq!(partner.class_name.as_deref(), Some("res.partner"));

        let lines = class
            .associations
            .iter()
            .find(|a| a.name == "line_ids")
            .expect("line_ids association");
        assert_eq!(lines.kind, AssociationKind::HasMany);
        assert_eq!(lines.class_name.as_deref(), Some("account.move.line"));
        assert_eq!(lines.inverse_of.as_deref(), Some("move_id"));

        // The case relation_kind exists to disambiguate: a comodel-only,
        // inverse-less field is a Many2many, NOT a Many2one.
        let tags = class
            .associations
            .iter()
            .find(|a| a.name == "tag_ids")
            .expect("tag_ids association");
        assert_eq!(tags.kind, AssociationKind::HasAndBelongsToMany);
        assert_eq!(tags.class_name.as_deref(), Some("account.analytic.tag"));

        // Compute field → computed_field carrying method + @api.depends.
        assert_eq!(class.computed_fields.len(), 1);
        let computed = &class.computed_fields[0];
        assert_eq!(computed.field, "amount_total");
        assert_eq!(computed.compute_method, "_compute_amount");
        assert_eq!(computed.depends, vec!["line_ids.balance".to_string()]);
    }

    /// A Rails-shape model carrying BOTH the AR-DSL `associations` vector
    /// (declared schema — `belongs_to :project`, `has_many :time_entries`)
    /// AND the D-AR-3.5 physical schema stratum (`Model::fields`, the shape
    /// `ruff_ruby_spo::extract_app_with_schema` produces from the migration
    /// DSL). This is exactly the shape the pre-fix Rails lift silently
    /// dropped (falsifier #1 / D-PARITY-PROBE-WP-1): `project_odoo_fields`
    /// was Python-only, so `Model::fields` never reached the `Class` for a
    /// Ruby-language lift.
    fn mk_rails_schema_model() -> Model {
        let mut m = Model::new("WorkPackage");
        m.associations.push(AssocDecl {
            kind: AssocKind::BelongsTo,
            name: "project".to_string(),
            options: vec![],
        });
        m.associations.push(AssocDecl {
            kind: AssocKind::HasMany,
            name: "time_entries".to_string(),
            options: vec![],
        });
        m.fields.push(Field {
            name: "id".to_string(),
            field_type: Some("bigint".to_string()),
            not_null: Some(true),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "subject".to_string(),
            field_type: Some("string".to_string()),
            not_null: Some(true),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "description".to_string(),
            field_type: Some("text".to_string()),
            not_null: None,
            ..Default::default()
        });
        // The FK column that duplicates the `project` association above —
        // must be shadowed, not double-projected as a scalar attribute.
        m.fields.push(Field {
            name: "project_id".to_string(),
            field_type: Some("bigint".to_string()),
            not_null: Some(true),
            ..Default::default()
        });
        m
    }

    /// **(a)** GAP-1: the Ruby lift must project the D-AR-3.5 schema stratum
    /// (`Model::fields`), same as the Python lift does for Odoo's `fields`.
    /// Before this fix, `lift_model_ruby_does_not_project_fields` pinned the
    /// OLD (bug) behaviour — a Rails-shape model's physical columns never
    /// reached `Class.attributes` at all.
    #[test]
    fn lift_model_ruby_projects_schema_stratum_fields_with_types() {
        let class = lift_model(&mk_rails_schema_model());
        let by_name = |n: &str| {
            class
                .attributes
                .iter()
                .find(|a| a.name == n)
                .unwrap_or_else(|| panic!("{n} must be projected as an attribute"))
        };
        assert_eq!(by_name("id").type_name.as_deref(), Some("bigint"));
        assert_eq!(by_name("subject").type_name.as_deref(), Some("string"));
        assert_eq!(by_name("description").type_name.as_deref(), Some("text"));
    }

    /// **(b)** GAP-1 FK-dedup: a scalar `<name>_id` column must NOT also
    /// surface as an `Attribute` when the model declares an association
    /// named `<name>` — the physical FK column and the declared
    /// `belongs_to` are the SAME relation seen from two schema strata, and
    /// double-projecting it would emit both `project_id: OgInt` (ORM
    /// spelling) and `project: ToOne<Project>` (AR spelling) for one
    /// relation. The bare `id` primary key is never shadowed (it carries no
    /// `_id`-suffixed prefix of its own).
    #[test]
    fn lift_model_ruby_fk_scalar_deduped_against_declared_association_id_kept() {
        let class = mk_rails_schema_model();
        let lifted = lift_model(&class);
        assert!(
            !lifted.attributes.iter().any(|a| a.name == "project_id"),
            "project_id must be shadowed by the `project` association: {:?}",
            lifted.attributes,
        );
        assert!(
            lifted.attributes.iter().any(|a| a.name == "id"),
            "the literal id column is never FK-deduped"
        );
        assert!(lifted.associations.iter().any(|a| a.name == "project"));
    }

    /// PR #156 finding (b): `is_fk_shadowed_by_association` must also honour
    /// an association's *explicit* `foreign_key:` option, not just the
    /// `<name>_id` naming convention. `belongs_to :author, foreign_key:
    /// "user_id"` names the association `author` (not `user`), so the
    /// naming-convention rule alone would miss it and double-project
    /// `user_id: OgInt` alongside `author: ToOne<...>`.
    ///
    /// **Red-before-fix:** on unpatched `main`, `user_id` survives as an
    /// attribute (the naming-convention check only strips `_id` and compares
    /// the stem `"user"` to association names, never finding `"author"`) —
    /// this assertion fails until `is_fk_shadowed_by_association` also
    /// checks `Association::foreign_key`.
    #[test]
    fn lift_model_ruby_fk_deduped_by_explicit_foreign_key() {
        let mut m = Model::new("Comment");
        m.associations.push(AssocDecl {
            kind: AssocKind::BelongsTo,
            name: "author".to_string(),
            options: vec![("foreign_key".to_string(), "\"user_id\"".to_string())],
        });
        m.fields.push(Field {
            name: "user_id".to_string(),
            field_type: Some("bigint".to_string()),
            not_null: Some(true),
            ..Default::default()
        });
        let lifted = lift_model(&m);
        assert!(
            !lifted.attributes.iter().any(|a| a.name == "user_id"),
            "user_id must be shadowed by author's explicit foreign_key: {:?}",
            lifted.attributes,
        );
        assert!(lifted.associations.iter().any(|a| a.name == "author"));
    }

    /// PR #156 finding (c): a physical column whose name already matches an
    /// AR-DSL-declared attribute (`attribute :foo, :string`) must not be
    /// projected a second time — `project_total_schema_fields` previously
    /// pushed unconditionally, producing two `foo` entries in
    /// `class.attributes` (an invalid duplicate struct field in the
    /// generated code).
    ///
    /// **Red-before-fix:** on unpatched `main`, the count is 2 (the AR-DSL
    /// lift populates one `foo` attribute earlier in
    /// `lift_model_with_language`, and `project_total_schema_fields` pushes a
    /// second, unguarded, for the physical `foo` column).
    #[test]
    fn lift_model_ruby_physical_column_does_not_duplicate_declared_attribute() {
        let mut m = Model::new("Widget");
        m.attributes.push(AttrDecl {
            kind: AttrKind::Attribute,
            name: "foo".to_string(),
            options: vec![("type".to_string(), "string".to_string())],
        });
        m.fields.push(Field {
            name: "foo".to_string(),
            field_type: Some("string".to_string()),
            ..Default::default()
        });
        let lifted = lift_model(&m);
        assert_eq!(
            lifted.attributes.iter().filter(|a| a.name == "foo").count(),
            1,
            "foo must appear exactly once, not once per schema stratum: {:?}",
            lifted.attributes,
        );
    }

    /// **(c)** GAP-1 required wiring: `Field::not_null` maps onto
    /// `AttributeOptions::required` — `Some(true)` (`null: false`) stays
    /// `Some(true)`; `None` (Rails' nullable default) becomes the EXPLICIT
    /// `Some(false)`, since the schema stratum is total knowledge (every
    /// column has a real nullability). See `emit.rs`'s
    /// `emit_rust_doc_line_prints_concept_high_and_app_low` /
    /// `emits_rust_struct_with_typed_and_optional_schema_fields_for_rails`
    /// for the paired emit-side proof (bare type vs `Option<...>`).
    #[test]
    fn lift_model_ruby_wires_not_null_to_required() {
        let class = lift_model(&mk_rails_schema_model());
        let by_name = |n: &str| class.attributes.iter().find(|a| a.name == n).unwrap();
        assert_eq!(by_name("subject").options.required, Some(true));
        assert_eq!(by_name("description").options.required, Some(false));
        assert_eq!(by_name("id").options.required, Some(true));
    }

    /// The Odoo path is completely unaffected by the Rails schema-field
    /// projection (GAP-1 is additive per-language, not a shared code path
    /// change): `project_odoo_fields` never sets `required` at all, so
    /// `AttributeOptions::required` stays `None` for every Odoo-lifted
    /// attribute — zero drift on the existing Odoo lift/emit tests.
    #[test]
    fn lift_model_python_never_sets_required_zero_drift() {
        let class = lift_model_python(&mk_odoo_model());
        for attr in &class.attributes {
            assert_eq!(
                attr.options.required, None,
                "Odoo path must not set required on {}",
                attr.name
            );
        }
    }

    #[test]
    fn odoo_inherit_lands_on_mixins_not_parent() {
        // The is_a input end of the transpile chain: `ruff_python_spo`
        // populates the frontend-agnostic `Model::inherits` from Odoo
        // `_inherit` (self-reopen already excluded upstream). The lift routes
        // it to `Class::mixins` — the vocab's designated multi-parent shelf —
        // NOT to the single `parent` / `inheritance` is_a spine (those stay
        // Rails-STI-shaped; the vocab excludes mixins from `inheritance`).
        let mut m = mk_odoo_model();
        m.inherits = vec!["mail_thread".to_string(), "mail_activity_mixin".to_string()];
        let class = lift_model_python(&m);

        // Both parents preserved on the mixins shelf, order kept.
        assert!(class.mixins.contains(&"mail_thread".to_string()));
        assert!(class.mixins.contains(&"mail_activity_mixin".to_string()));
        // The is_a spine is untouched — Odoo `_inherit` is NOT STI.
        assert_eq!(class.parent, None);
        assert_eq!(class.inheritance, Inheritance::Root);
    }

    #[test]
    fn empty_inherits_adds_no_mixins() {
        // Frontend-agnostic no-op: the Rails / C++ producers never populate
        // `Model::inherits`, so the lift must not fabricate mixins. A bare
        // model (no concerns, no acts_as, no `_inherit`) lifts with an empty
        // mixins shelf — the `inherits` extension contributes nothing.
        let class = lift_model(&Model::new("Bare"));
        assert!(class.mixins.is_empty());
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

    #[test]
    fn classify_domain_names_op_project_and_odoo_erp() {
        // OpenProject → "project"
        let mut op = ModelGraph::new("openproject");
        op.models.push(Model::new("WorkPackage"));
        assert_eq!(lift_model_graph(&op)[0].source_domain.as_deref(), Some("project"));
        // Odoo → "erp"
        let mut odoo = ModelGraph::new("odoo");
        odoo.models.push(Model::new("AccountMove"));
        assert_eq!(lift_model_graph(&odoo)[0].source_domain.as_deref(), Some("erp"));
        // Unrecognized → None (not guessed).
        let mut other = ModelGraph::new("mystery");
        other.models.push(Model::new("X"));
        assert_eq!(lift_model_graph(&other)[0].source_domain, None);
    }

    #[test]
    fn lift_model_graph_domain_gates_the_canonical_concept() {
        // codex P2 on #72: `lift_model` is domain-blind, but
        // `lift_model_graph` knows the curator's domain and must gate
        // promotions through it. A bare `Role` only becomes `project_role`
        // for a project-mgmt curator.
        let concept = |ns: &str, model: &str| {
            let mut g = ModelGraph::new(ns);
            g.models.push(Model::new(model));
            lift_model_graph(&g)[0].canonical_concept.clone().unwrap()
        };
        // Project curator (OpenProject / Redmine) — `Role` promotes.
        assert_eq!(concept("openproject", "Role"), "project_role");
        assert_eq!(concept("redmine", "Role"), "project_role");
        // Unrelated curator (domain None) — `Role` stays lexical, NOT
        // project_role, so it can't route as ConceptDomain::ProjectMgmt.
        assert_eq!(concept("mystery", "Role"), "role");
        // Foreign-but-known domain (erp) — also withheld.
        assert_eq!(concept("odoo", "Role"), "role");
        // Cross-domain bridge survives the gate from any domain.
        assert_eq!(concept("openproject", "TimeEntry"), "billable_work_entry");
        assert_eq!(concept("odoo", "account_analytic_line"), "billable_work_entry");
        // `lift_model` itself stays domain-blind (all-domains best guess).
        assert_eq!(
            lift_model(&Model::new("Role")).canonical_concept.as_deref(),
            Some("project_role"),
        );
    }

    #[test]
    fn source_curator_carries_namespace_distinct_from_domain() {
        // Two curators in the SAME domain (both `project`) are kept
        // distinguishable by source_curator (the harvest namespace).
        let mut redmine = ModelGraph::new("redmine");
        redmine.models.push(Model::new("Issue"));
        let r = &lift_model_graph(&redmine)[0];
        assert_eq!(r.source_domain.as_deref(), Some("project"));
        assert_eq!(r.source_curator.as_deref(), Some("redmine"));

        let mut op = ModelGraph::new("openproject");
        op.models.push(Model::new("WorkPackage"));
        let o = &lift_model_graph(&op)[0];
        assert_eq!(o.source_domain.as_deref(), Some("project"));
        assert_eq!(o.source_curator.as_deref(), Some("openproject"));

        // Same domain, distinct curators — AND same canonical concept/id:
        // the convergence holds while provenance stays separable.
        assert_eq!(r.source_domain, o.source_domain);
        assert_ne!(r.source_curator, o.source_curator);
        assert_eq!(r.canonical_id(), o.canonical_id());

        // Empty namespace → no curator tag (not an empty string).
        let mut bare = ModelGraph::new("");
        bare.models.push(Model::new("X"));
        assert_eq!(lift_model_graph(&bare)[0].source_curator, None);
    }

    #[test]
    fn project_work_item_role_maps_rails_dialect_synonyms() {
        // Universal names common to Redmine + OP.
        for (src, want) in [
            ("project", "project"),
            ("status", "status"),
            ("priority", "priority"),
            ("author", "author"),
            ("time_entries", "time_entries"),
        ] {
            assert_eq!(project_work_item_role(src), Some(want));
        }
        // Redmine-side dialect: tracker -> type; relations_from/to -> relations.
        assert_eq!(project_work_item_role("tracker"), Some("type"));
        assert_eq!(project_work_item_role("relations_from"), Some("relations"));
        assert_eq!(project_work_item_role("relations_to"), Some("relations"));
        // OP-side dialect: type -> type; responsible -> assignee.
        assert_eq!(project_work_item_role("type"), Some("type"));
        assert_eq!(project_work_item_role("responsible"), Some("assignee"));
        // Off-shape associations return None — `fixed_version` (Redmine),
        // `file_links` (OP) are real-but-not-canonical.
        assert!(project_work_item_role("fixed_version").is_none());
        assert!(project_work_item_role("file_links").is_none());
    }

    #[test]
    fn project_work_item_canonical_roles_unions_associations_and_mixins() {
        // Redmine-shaped class: journals + relations come via direct
        // associations; the union still covers them.
        let mut redmine = Class::new("Issue");
        redmine.associations = vec![
            Association::new(AssociationKind::BelongsTo, "project"),
            Association::new(AssociationKind::BelongsTo, "tracker"),
            Association::new(AssociationKind::BelongsTo, "status"),
            Association::new(AssociationKind::BelongsTo, "author"),
            Association::new(AssociationKind::BelongsTo, "assigned_to"),
            Association::new(AssociationKind::BelongsTo, "priority"),
            Association::new(AssociationKind::HasMany, "journals"),
            Association::new(AssociationKind::HasMany, "time_entries"),
            Association::new(AssociationKind::HasMany, "relations_from"),
            Association::new(AssociationKind::HasMany, "relations_to"),
            // Off-shape: must NOT inflate the role set.
            Association::new(AssociationKind::BelongsTo, "fixed_version"),
        ];
        let roles = project_work_item_canonical_roles(&redmine);
        assert_eq!(roles.len(), 9);
        for r in [
            "project", "status", "type", "priority", "author", "assignee",
            "journals", "relations", "time_entries",
        ] {
            assert!(roles.contains(r), "Redmine projection missing role {r}");
        }

        // OP-shaped class: journals + relations come via MIXINS
        // (`acts_as_journalized`, `WorkPackages::Relations`). Same total
        // role set under the lineage-transcode bridge.
        let mut op = Class::new("WorkPackage");
        op.associations = vec![
            Association::new(AssociationKind::BelongsTo, "project"),
            Association::new(AssociationKind::BelongsTo, "type"),
            Association::new(AssociationKind::BelongsTo, "status"),
            Association::new(AssociationKind::BelongsTo, "author"),
            Association::new(AssociationKind::BelongsTo, "assigned_to"),
            Association::new(AssociationKind::BelongsTo, "responsible"),
            Association::new(AssociationKind::BelongsTo, "priority"),
            Association::new(AssociationKind::HasMany, "time_entries"),
        ];
        op.mixins = vec![
            "acts_as_journalized".to_string(),
            "WorkPackages::Relations".to_string(),
        ];
        let op_roles = project_work_item_canonical_roles(&op);
        assert_eq!(op_roles, roles, "OP must project to the same canonical role set as Redmine");
    }

    #[test]
    fn project_role_maps_rails_dialect_synonyms() {
        // Universal across both Redmine and OP.
        assert_eq!(project_role("parent"), Some("parent"));
        assert_eq!(project_role("time_entries"), Some("time_entries"));
        // Divergent work-item names converge on the canonical role.
        assert_eq!(project_role("issues"), Some("work_items"));
        assert_eq!(project_role("work_packages"), Some("work_items"));
        // The through-association actor chain — both spellings + the OP
        // extra hops (member_principals, principals) land at `members`.
        for src in ["members", "memberships", "users", "member_principals", "principals"] {
            assert_eq!(project_role(src), Some("members"), "{src} -> members");
        }
        // Off-shape names return None (real but not yet promoted into the
        // canonical surface).
        assert!(project_role("news").is_none());
        assert!(project_role("forums").is_none());
        assert!(project_role("repositories").is_none());
    }

    #[test]
    fn project_canonical_roles_covers_both_curators() {
        // Redmine Project shape (subset — only the on-canonical assocs).
        // No `belongs_to :parent` in the fixture: Redmine threads project
        // hierarchy through the `awesome_nested_set` gem mixin, not a
        // direct AR association.
        let mut redmine = Class::new("Project");
        redmine.associations = vec![
            Association::new(AssociationKind::HasMany, "memberships"),
            Association::new(AssociationKind::HasMany, "members"),
            Association::new(AssociationKind::HasMany, "users"),
            Association::new(AssociationKind::HasMany, "issues"),
            Association::new(AssociationKind::HasMany, "time_entries"),
            // Off-shape: must not inflate the role set.
            Association::new(AssociationKind::HasMany, "news"),
        ];
        let r_roles = project_canonical_roles(&redmine);
        // OP Project shape (with work_packages instead of issues, plus
        // OP's extra through-association hops). OP threads parent via
        // the `Projects::Hierarchy` concern, not `belongs_to :parent`.
        let mut op = Class::new("Project");
        op.associations = vec![
            Association::new(AssociationKind::HasMany, "members"),
            Association::new(AssociationKind::HasMany, "memberships"),
            Association::new(AssociationKind::HasMany, "member_principals"),
            Association::new(AssociationKind::HasMany, "users"),
            Association::new(AssociationKind::HasMany, "principals"),
            Association::new(AssociationKind::HasMany, "work_packages"),
            Association::new(AssociationKind::HasMany, "time_entries"),
        ];
        let o_roles = project_canonical_roles(&op);
        // v1 canonical surface: 3 direct-association roles. `parent` is
        // a real cross-curator concept but lives behind mixins and is
        // not yet decoded (see ogar_vocab::project doc).
        let expected: std::collections::HashSet<&'static str> =
            ["work_items", "time_entries", "members"].into_iter().collect();
        assert_eq!(r_roles, expected, "Redmine projection must cover the 3-role canonical surface");
        assert_eq!(o_roles, expected, "OP projection must cover the same surface");
        assert_eq!(r_roles, o_roles, "lineage-transcode parity for Project");
    }

    #[test]
    fn lift_model_sets_canonical_concept_including_promoted_invariant() {
        // Plain class with no promoted invariant → lexical concept.
        assert_eq!(
            lift_model(&Model::new("Account")).canonical_concept.as_deref(),
            Some("account"),
        );
        // Promoted ERP-bridge concept (BillableWorkEntry) — OpenProject
        // `TimeEntry` deterministically wired into the cross-domain bridge.
        assert_eq!(
            lift_model(&Model::new("TimeEntry")).canonical_concept.as_deref(),
            Some("billable_work_entry"),
        );
        // Promoted project-domain concept (ProjectWorkItem) — Redmine
        // `Issue` and OpenProject `WorkPackage` both wire into the
        // same-domain work-item invariant.
        assert_eq!(
            lift_model(&Model::new("Issue")).canonical_concept.as_deref(),
            Some("project_work_item"),
        );
        assert_eq!(
            lift_model(&Model::new("WorkPackage")).canonical_concept.as_deref(),
            Some("project_work_item"),
        );
    }

    fn mk_model_with_functions() -> Model {
        let mut m = Model::new("WorkPackage");
        m.functions.push(Function {
            name: "set_status".to_string(),
            reads: vec!["status".to_string()],
            raises: Vec::new(),
            traverses: Vec::new(),
            ..Default::default()
        });
        m.functions.push(Function {
            name: "close!".to_string(),
            reads: Vec::new(),
            raises: vec!["ArgumentError".to_string()],
            traverses: Vec::new(),
            ..Default::default()
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

    /// SPEC-1(i): `reads` / `writes` / `calls` now ride `ActionDef` as
    /// first-class effect annotations (OGAR-AS-IR §3 test 2), sourced
    /// verbatim from `ruff_spo_triplet::Function`. Crucially, this must NOT
    /// fabricate a reactive `kausal` trigger from a plain method read — a
    /// Rails method reading a field is not an `@api.depends`-style
    /// recomputation trigger, so `kausal` stays `None`.
    #[test]
    fn lift_actions_carries_read_write_call_effect_facts() {
        let mut m = Model::new("SaleOrder");
        m.functions.push(Function {
            name: "recompute_total".to_string(),
            reads: vec!["quantity".to_string(), "price".to_string()],
            writes: vec!["total".to_string()],
            calls: vec!["self.touch".to_string()],
            raises: Vec::new(),
            traverses: Vec::new(),
            ..Default::default()
        });
        let acts = lift_actions(&m);
        assert_eq!(acts.len(), 1);
        let a = &acts[0];
        assert_eq!(a.reads, vec!["quantity".to_string(), "price".to_string()]);
        assert_eq!(a.writes, vec!["total".to_string()]);
        assert_eq!(a.calls, vec!["self.touch".to_string()]);
        assert!(
            a.kausal.is_none(),
            "reads/writes/calls are effect annotations, not a fabricated reactive trigger"
        );
    }

    /// A `Function` with all-empty fact vectors must lift to empty `Vec`s on
    /// `ActionDef` — not `None`-like phantom entries. `reads` / `writes` /
    /// `calls` are `Vec<String>`, so "empty" and "absent" are the same zero
    /// value; this pins that the lift never invents entries when ruff saw
    /// none.
    #[test]
    fn lift_actions_empty_facts_stay_empty_not_none() {
        let mut m = Model::new("Bare");
        m.functions.push(Function {
            name: "noop".to_string(),
            reads: Vec::new(),
            writes: Vec::new(),
            calls: Vec::new(),
            raises: Vec::new(),
            traverses: Vec::new(),
            ..Default::default()
        });
        let acts = lift_actions(&m);
        assert_eq!(acts.len(), 1);
        assert!(acts[0].reads.is_empty());
        assert!(acts[0].writes.is_empty());
        assert!(acts[0].calls.is_empty());
    }

    /// Regression: adding the effect-annotation fields must not disturb the
    /// pre-existing identity / predicate / object_class shape — same
    /// assertions as `lift_actions_predicate_object_class_and_identity`,
    /// kept as an explicit named regression per SPEC-1(i).
    #[test]
    fn lift_actions_identity_predicate_object_unchanged() {
        let acts = lift_actions(&mk_model_with_functions());
        assert_eq!(acts[0].predicate, "set_status");
        assert_eq!(acts[0].object_class, "WorkPackage");
        assert_eq!(acts[0].identity, "WorkPackage::action_def::set_status");
        assert_eq!(acts[1].predicate, "close!");
        assert_eq!(acts[1].object_class, "WorkPackage");
        assert_eq!(acts[1].identity, "WorkPackage::action_def::close!");
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

    #[test]
    fn lift_actions_carries_private_helper_hook_bodies() {
        // AT-CARRY-1b (review on #164): Rails lifecycle hook targets are
        // conventionally private -> `Model::helpers` (ruff #45). The DO-arm
        // must carry their body facts, or the W3.3 delete-blocking rows
        // never reach a consumer.
        let mut m = Model::new("Issue");
        m.functions.push(ruff_spo_triplet::Function {
            name: "public_action".to_string(),
            ..Default::default()
        });
        m.helpers.push(ruff_spo_triplet::Function {
            name: "update_closed_on".to_string(),
            writes: vec!["closed_on".to_string()],
            reads: vec!["updated_on".to_string()],
            ..Default::default()
        });
        let actions = lift_actions(&m);
        assert_eq!(actions.len(), 2, "public action + private hook body");
        let hook = actions
            .iter()
            .find(|a| a.predicate == "update_closed_on")
            .expect("private hook body must arrive in the DO-arm");
        assert_eq!(hook.identity, "Issue::action_def::update_closed_on");
        assert_eq!(hook.writes, vec!["closed_on".to_string()]);
        assert_eq!(hook.reads, vec!["updated_on".to_string()]);
    }
}
