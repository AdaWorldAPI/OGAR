//! SQLAlchemy / WoA field projection — SPEC-5 Part B.
//!
//! `lift_model_with_language` (in `lib.rs`) routes `Language::Python` through
//! [`super::project_odoo_fields`], which never wires `not_null -> required`
//! (only the total-schema path does that). WoA's Flask-SQLAlchemy schema is
//! **total** — every column's nullability is a declared fact
//! (`nullable=True/False`), exactly like a Rails migration column — so it
//! needs the total-schema-style wiring (FK-dedup + `not_null -> required` +
//! the attribute-dup guard), NOT the Odoo wiring, on a path that still stamps
//! [`Language::Python`] (WoA is Python source).
//!
//! # Consolidation onto `project_total_schema_fields` (landed)
//!
//! SPEC-5 Part B (design note 1) named ONE shared
//! `project_total_schema_fields(class, model)` helper, called by both the
//! Rails (Ruby) and this module's SQLAlchemy (Python) producers, as the
//! target design. That consolidation has **landed**: [`project_sqlalchemy_fields`]
//! is now a thin wrapper around [`crate::project_total_schema_fields`], and
//! the FK-dedup predicate (`crate::is_fk_shadowed_by_association`) is no
//! longer duplicated here — both producers route through the single
//! `lib.rs` implementation (Batch-1 Item 2, D-PARITY-PROBE-WOA-1 Nachtrag).
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

use ogar_vocab::{Class, Language};
use ruff_spo_triplet::{Model, ModelGraph};

use crate::lift_association;

/// Project a WoA/Flask-SQLAlchemy model's schema stratum
/// (`Model::fields` — the `db.Column(...)` declarations, the stand-in for
/// the not-yet-built `ruff_sqlalchemy_spo` frontend's harvest) onto the
/// schema-carrying [`Class`] columns.
///
/// WoA/Flask-SQLAlchemy schema-stratum projection — the Python producer's
/// named entry point onto the shared total-schema logic (FK-dedup +
/// not_null->required + attribute-dup guard). One implementation, two
/// producers: this and `ogar-from-ruff::project_total_schema_fields`
/// (Rails), which this function delegates to entirely.
fn project_sqlalchemy_fields(class: &mut Class, model: &Model) {
    crate::project_total_schema_fields(class, model);
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
/// here as a separate, named follow-up (the FK-dedup lockstep copy this
/// module used to carry, `is_fk_shadowed_by_association`, is now
/// consolidated onto `crate::is_fk_shadowed_by_association`; this one is
/// deliberately left as-is, out of scope for that consolidation). Only the
/// branch relevant to a SQLAlchemy producer (WoA / SMB, both
/// Flask-SQLAlchemy German-ERP consumers) is reproduced; the
/// OpenProject/Redmine/Odoo branches don't apply to this producer.
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

    /// PR #156 finding (b), cross-producer: the explicit-`foreign_key` FK-dedup
    /// rule must fire identically for the Rails (Ruby) and SQLAlchemy (Python)
    /// producers, since Batch-1 Item 2 routes both through the single
    /// `crate::project_total_schema_fields` / `crate::is_fk_shadowed_by_association`
    /// implementation. `author`'s association name does NOT match the
    /// `<name>_id` convention for `user_id` (that would require an association
    /// named `user`) — only the explicit `foreign_key: "user_id"` shadows it.
    #[test]
    fn both_producers_share_explicit_foreign_key_dedup() {
        let mut m = Model::new("Post");
        m.associations.push(AssocDecl {
            kind: AssocKind::BelongsTo,
            name: "author".to_string(),
            options: vec![
                ("class_name".to_string(), "User".to_string()),
                ("foreign_key".to_string(), "user_id".to_string()),
            ],
        });
        m.fields.push(Field {
            name: "user_id".to_string(),
            field_type: Some("integer".to_string()),
            not_null: Some(true),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "title".to_string(),
            field_type: Some("string".to_string()),
            not_null: Some(true),
            ..Default::default()
        });

        let mut graph = ModelGraph::new("");
        graph.models.push(m);

        let ruby_classes = crate::lift_model_graph(&graph);
        let sqlalchemy_classes = lift_model_graph_sqlalchemy(&graph);

        for class in [&ruby_classes[0], &sqlalchemy_classes[0]] {
            assert!(
                !class.attributes.iter().any(|a| a.name == "user_id"),
                "user_id must be FK-shadowed by author's explicit foreign_key, not the \
                 <name>_id convention (association is named `author`, not `user`): {:?}",
                class.attributes,
            );
            let author = class
                .associations
                .iter()
                .find(|a| a.name == "author")
                .expect("author association present");
            assert_eq!(author.kind, AssociationKind::BelongsTo);
            let title = class
                .attributes
                .iter()
                .find(|a| a.name == "title")
                .expect("title attribute present");
            assert_eq!(title.options.required, Some(true));
        }

        let ruby_names: std::collections::BTreeSet<_> =
            ruby_classes[0].attributes.iter().map(|a| a.name.clone()).collect();
        let sqlalchemy_names: std::collections::BTreeSet<_> =
            sqlalchemy_classes[0].attributes.iter().map(|a| a.name.clone()).collect();
        assert_eq!(
            ruby_names, sqlalchemy_names,
            "both producers must project the identical attribute name set, proving shared \
             FK-dedup semantics from one implementation"
        );
    }
}
