//! SQLAlchemy / WoA field projection — SPEC-5 Part B.
//!
//! `lift_model_with_language` (in `lib.rs`) routes `Language::Python` through
//! [`super::project_odoo_fields`], which never wires `not_null -> required`
//! (only the Rails path, `super::project_rails_fields`, does that). WoA's
//! Flask-SQLAlchemy schema is **total** — every column's nullability is a
//! declared fact (`nullable=True/False`), exactly like a Rails migration
//! column — so it needs the Rails-style wiring (FK-dedup +
//! `not_null -> required` + the attribute-dup guard), NOT the Odoo wiring,
//! on a path that still stamps [`Language::Python`] (WoA is Python source).
//!
//! # Why this module does not share code with `project_rails_fields`
//!
//! SPEC-5 Part B (design note 1) prefers ONE shared
//! `project_total_schema_fields(class, model)` helper called by both
//! `project_rails_fields` and this module's [`project_sqlalchemy_fields`],
//! with `lib.rs` threading an internal `FieldProjection` selector through
//! `lift_model_with_language`.
//!
//! **This session's operator constraint overrides that design note:** a
//! sibling grind agent (G-A) is editing `lib.rs` concurrently on unrelated
//! functions, and the merge contract for this workstream is "new module
//! file(s) + at most one `mod` line in `lib.rs`, otherwise `lib.rs`
//! untouched." So the FK-dedup predicate and the total-nullability wiring
//! are **duplicated** here rather than threaded through `lib.rs`'s private
//! `is_fk_shadowed_by_association` / `project_rails_fields`. This is a named,
//! reported trade-off (see the mission report), not a silent divergence —
//! once G-A's `lib.rs` changes land, a follow-up should consolidate onto the
//! spec's preferred `project_total_schema_fields` shared helper.
//!
//! Consequently [`lift_model_sqlalchemy`] / [`lift_model_graph_sqlalchemy`]
//! are deliberately narrower than [`super::lift_model`] /
//! [`super::lift_model_python`]: they populate `name`, `language`,
//! `associations` (via the already-`pub` [`super::lift_association`]),
//! `source_domain` / `source_curator` / `canonical_concept`, and the
//! schema-stratum `attributes` / `associations` / `computed_fields` (this
//! module's job). Rails-DSL-only slots (`mixins`, `scopes`, `callbacks`,
//! `validations`, `enums`, `inheritance`) stay at their `Class::default()`
//! empty state — a Flask-SQLAlchemy model has no such DSL to lift, so this
//! is the honest empty state for this producer, not a dropped fact.

use ogar_vocab::{Association, AssociationKind, Attribute, Class, ComputedField, Language};
use ruff_spo_triplet::{Model, ModelGraph};

use crate::lift_association;

/// Project a WoA/Flask-SQLAlchemy model's schema stratum
/// (`Model::fields` — the `db.Column(...)` declarations, the stand-in for
/// the not-yet-built `ruff_sqlalchemy_spo` frontend's harvest) onto the
/// schema-carrying [`Class`] columns.
///
/// Mirrors `ogar-from-ruff::project_rails_fields` (SPEC-5 Part B, design
/// note 1) field-for-field:
/// - a relational field (`target` set) -> [`Association`]; dormant for the
///   WoA fixture (associations arrive via `Model::associations` — see
///   [`lift_model_sqlalchemy`] — not via a relational `Model::fields`
///   entry), kept for shape-parity with the Odoo/Rails paths.
/// - a scalar field named `<name>_id` whose model ALSO declares an
///   association named `<name>` (SQLAlchemy `db.relationship`, already
///   lifted onto `class.associations` before this function runs) ->
///   **skipped** ([`is_fk_shadowed_by_association`]). The physical FK
///   column and the declared `db.relationship` are the SAME relation seen
///   twice; the ORM spelling (`timesheet_id: OgInt`) is dropped in favour
///   of the AR-ish spelling (`timesheet: ToOne<TimeSheet>`).
/// - any other scalar field -> [`Attribute`], `type_name` from
///   `Field::field_type`, `AttributeOptions::required` wired from
///   `Field::not_null`: `Some(true)` (`nullable=False`) stays `Some(true)`;
///   `None` (`nullable=True` or absent — SQLAlchemy's column default, per
///   SPEC-5 Part A's mapping table) becomes the EXPLICIT `Some(false)` — the
///   schema stratum is total knowledge, so absence of `nullable=False` IS a
///   positive "nullable" fact, not "unknown".
/// - a compute field (`emitted_by` set) -> [`ComputedField`] — dormant for
///   WoA today (no `@property`/compute-linkage pass exists yet), kept for
///   shape-parity.
fn project_sqlalchemy_fields(class: &mut Class, model: &Model) {
    for field in &model.fields {
        if let Some(comodel) = &field.target {
            let kind = if field.relation_kind.as_deref() == Some("one2many") || field.inverse_name.is_some()
            {
                AssociationKind::HasMany
            } else {
                AssociationKind::BelongsTo
            };
            let mut assoc = Association::new(kind, &field.name);
            assoc.class_name = Some(comodel.clone());
            assoc.inverse_of = field.inverse_name.clone();
            class.associations.push(assoc);
        } else if !is_fk_shadowed_by_association(&field.name, class)
            && !class.attributes.iter().any(|a| a.name == field.name)
        {
            let mut attr = Attribute::new(&field.name);
            attr.type_name = field.field_type.clone();
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

/// FK-dedup predicate for [`project_sqlalchemy_fields`] — duplicated from
/// `ogar-from-ruff::is_fk_shadowed_by_association` (kept `lib.rs`-private,
/// see the module doc for why this file cannot call it directly). Same
/// contract: `<name>_id` is shadowed when `class` already carries an
/// [`Association`] named `<name>`; the bare `id` primary key never matches
/// (no `_id`-suffixed prefix of its own).
fn is_fk_shadowed_by_association(field_name: &str, class: &Class) -> bool {
    field_name
        .strip_suffix("_id")
        .filter(|prefix| !prefix.is_empty())
        .is_some_and(|prefix| class.associations.iter().any(|a| a.name == prefix))
}

/// Lift one WoA/SQLAlchemy [`Model`] to an OGAR [`Class`] stamped
/// [`Language::Python`] — the SQLAlchemy producer path (SPEC-5 Part B).
/// Deliberately narrower than [`super::lift_model_python`]; see the module
/// doc for exactly which `Class` slots this populates.
#[must_use]
pub fn lift_model_sqlalchemy(model: &Model) -> Class {
    let mut class = Class::new(&model.name);
    class.language = Language::Python;
    // AR-DSL-shaped associations (`db.relationship(...)`) lift the same way
    // Rails' `belongs_to`/`has_many` do — `lift_association` is producer
    // agnostic and already `pub` in `lib.rs`.
    class.associations = model.associations.iter().filter_map(lift_association).collect();
    project_sqlalchemy_fields(&mut class, model);
    class
}

/// Lift every model in a [`ModelGraph`] via [`lift_model_sqlalchemy`] —
/// the whole-graph SQLAlchemy entry point SPEC-5 Part B names. Declaration
/// order preserved, mirroring `lift_model_graph_python` /
/// `lift_model_graph`.
#[must_use]
pub fn lift_model_graph_sqlalchemy(graph: &ModelGraph) -> Vec<Class> {
    let domain = classify_woa_domain(&graph.namespace);
    let concept_domain = domain.as_deref().and_then(ogar_vocab::source_domain_concept);
    let curator = if graph.namespace.is_empty() {
        None
    } else {
        Some(graph.namespace.clone())
    };
    graph
        .models
        .iter()
        .map(|m| {
            let mut class = lift_model_sqlalchemy(m);
            class.source_domain = domain.clone();
            class.source_curator = curator.clone();
            class.canonical_concept =
                Some(ogar_vocab::canonical_concept_in_domain(&m.name, concept_domain));
            class
        })
        .collect()
}

/// The WoA/SMB slice of `lib.rs`'s private `classify_domain` — duplicated
/// here for the same merge-constraint reason as
/// [`is_fk_shadowed_by_association`]. Only the branch relevant to a
/// SQLAlchemy producer (WoA / SMB, both Flask-SQLAlchemy German-ERP
/// consumers) is reproduced; the OpenProject/Redmine/Odoo branches don't
/// apply to this producer.
fn classify_woa_domain(namespace: &str) -> Option<String> {
    let ns = namespace.to_ascii_lowercase();
    if ns.contains("woa") || ns.contains("smb") {
        Some("german-erp".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ogar_vocab::AssociationKind;
    use ruff_spo_triplet::{AssocDecl, AssocKind, Field};

    /// The synthetic `TimesheetActivity` fixture (WoA `models.py:1746-1753`,
    /// read-only — see SPEC-5's "concrete parity fixture" and
    /// `tests/woa_parity_probe.rs` for the full transcription with source
    /// quote). A smaller, self-contained copy for this module's unit tests.
    fn timesheet_activity_model() -> Model {
        let mut m = Model::new("TimesheetActivity");
        m.associations.push(AssocDecl {
            kind: AssocKind::BelongsTo,
            name: "timesheet".to_string(),
            options: vec![("class_name".to_string(), "TimeSheet".to_string())],
        });
        m.fields.push(Field {
            name: "id".to_string(),
            field_type: Some("integer".to_string()),
            not_null: Some(true),
            ..Default::default()
        });
        // FK column duplicating the `timesheet` association above — must be
        // shadowed, not double-projected.
        m.fields.push(Field {
            name: "timesheet_id".to_string(),
            field_type: Some("integer".to_string()),
            not_null: Some(true),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "beschreibung".to_string(),
            field_type: Some("string".to_string()),
            not_null: Some(true),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "created_at".to_string(),
            field_type: Some("datetime".to_string()),
            not_null: None, // no `nullable=False` in the source -> nullable.
            ..Default::default()
        });
        m
    }

    #[test]
    fn lift_sqlalchemy_wires_nullability_and_fk_dedup() {
        let class = lift_model_sqlalchemy(&timesheet_activity_model());
        let by_name = |n: &str| class.attributes.iter().find(|a| a.name == n).unwrap();

        assert_eq!(by_name("id").options.required, Some(true));
        assert_eq!(by_name("beschreibung").options.required, Some(true));
        assert_eq!(by_name("created_at").options.required, Some(false));

        assert!(
            !class.attributes.iter().any(|a| a.name == "timesheet_id"),
            "timesheet_id must be FK-shadowed by the `timesheet` association: {:?}",
            class.attributes,
        );
        let timesheet = class
            .associations
            .iter()
            .find(|a| a.name == "timesheet")
            .expect("timesheet association present");
        assert_eq!(timesheet.kind, AssociationKind::BelongsTo);
        assert_eq!(timesheet.class_name.as_deref(), Some("TimeSheet"));
    }

    #[test]
    fn sqlalchemy_path_stamps_python_language() {
        let class = lift_model_sqlalchemy(&timesheet_activity_model());
        assert!(matches!(class.language, Language::Python));
    }

    #[test]
    fn lift_model_graph_sqlalchemy_preserves_order_and_stamps_curator() {
        let mut graph = ModelGraph::new("woa");
        graph.models.push(timesheet_activity_model());
        graph.models.push(Model::new("Reminder"));
        let classes = lift_model_graph_sqlalchemy(&graph);
        assert_eq!(classes.len(), 2);
        assert_eq!(classes[0].name, "TimesheetActivity");
        assert_eq!(classes[1].name, "Reminder");
        assert_eq!(classes[0].source_domain.as_deref(), Some("german-erp"));
        assert_eq!(classes[0].source_curator.as_deref(), Some("woa"));
    }
}
