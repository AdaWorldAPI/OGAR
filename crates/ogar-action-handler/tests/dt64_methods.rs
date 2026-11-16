//! The `dt64` converter against the five authoritative domain tables and the
//! refusals it names. Run with `--features dt64`.
#![cfg(feature = "dt64")]

use lance_graph_contract::class_view::{ClassId, ClassView};
use lance_graph_contract::ontology::{DisplayTemplate, FieldRef};
use lance_graph_contract::set_algebra::{Logic, SetExpr};
use lance_graph_dt64::{BindStore, MethodId, Refusal, SchemaRev};
use ogar_action_handler::dt64::{
    ConvertRefusal, FieldRole, TableOutcome, TableRefusal, method_of, register_domain_table,
    register_domain_table_activated,
};
use ogar_class_view::OgarClassView;
use ogar_vocab::app::render_classid;
use ogar_vocab::capability_registry::{DomainTable, HotplugDrift, domain_tables};
use ogar_vocab::{ActionDef, KausalSpec, class_ids};

const APP: u16 = 0x0001;
const REV: SchemaRev = SchemaRev(1);

/// A one-class view with chosen field labels, for the refusal tests.
struct Fields {
    class: ClassId,
    fields: Vec<FieldRef>,
}

impl Fields {
    fn of(class: ClassId, labels: &[&str]) -> Fields {
        Fields {
            class,
            fields: labels
                .iter()
                .map(|l| FieldRef::new((*l).to_string(), (*l).to_string()))
                .collect(),
        }
    }
}

impl ClassView for Fields {
    fn fields(&self, class: ClassId) -> &[FieldRef] {
        if class == self.class {
            &self.fields
        } else {
            &[]
        }
    }
    fn template(&self, _class: ClassId) -> DisplayTemplate {
        DisplayTemplate::Detail
    }
    fn dolce_category_id(&self, _class: ClassId) -> u8 {
        0
    }
}

fn def(predicate: &str) -> ActionDef {
    ActionDef::new(
        format!("ogit-ocr/textline::action_def::{predicate}"),
        predicate,
        "ogit-ocr/textline",
    )
}

fn textline_view() -> Fields {
    Fields::of(class_ids::TEXTLINE, &["state", "text", "width"])
}

/// Register every table into one store and return the outcomes, in
/// `domain_tables()` order.
fn census() -> (BindStore, Vec<TableOutcome>) {
    let view = OgarClassView::new();
    let mut store = BindStore::new();
    let outcomes = domain_tables()
        .iter()
        .map(|t| register_domain_table(&mut store, &view, t, APP, REV).expect("DT64 accepts"))
        .collect();
    (store, outcomes)
}

/// The 50-row table: domain, predicate, MethodId or refusal reason.
fn render(outcomes: &[TableOutcome]) -> String {
    let mut out = String::new();
    for o in outcomes {
        for row in &o.rows {
            let verdict = match &row.result {
                Ok(r) => format!("registered id={}", r.id.0),
                Err(ConvertRefusal::UnknownField { role, name }) => {
                    format!("refused UnknownField({role:?} {name})")
                }
                Err(other) => format!("refused {other:?}"),
            };
            out.push_str(&format!(
                "{:<10} {:<24} {verdict}\n",
                o.domain, row.predicate
            ));
        }
    }
    out
}

/// Every def of every table is registered or refused by name: 50 rows, and
/// 14 / 6 / 20 / 3 / 7 per domain. Today geo, healthcare and erp (33) register
/// and ocr and document (17) are refused, each for the first parameter that is
/// not a field of the class.
#[test]
fn every_one_of_the_fifty_defs_is_registered_or_refused_by_name() {
    let (_, outcomes) = census();
    println!("{}", render(&outcomes));

    let per_domain: Vec<(&str, usize, usize, usize)> = outcomes
        .iter()
        .map(|o| {
            (
                o.domain,
                o.rows.len(),
                o.registered().count(),
                o.refused().count(),
            )
        })
        .collect();
    // (domain, defs, registered, refused)
    assert_eq!(
        per_domain,
        vec![
            ("ocr", 14, 0, 14),
            ("geo", 6, 6, 0),
            ("healthcare", 20, 20, 0),
            ("document", 3, 0, 3),
            ("erp", 7, 7, 0),
        ]
    );
    let registered: usize = outcomes.iter().map(|o| o.registered().count()).sum();
    let refused: usize = outcomes.iter().map(|o| o.refused().count()).sum();
    assert_eq!((registered, refused), (33, 17));
    assert_eq!(registered + refused, 50);

    // Every refusal is the named UnknownField over `reads`, and the name is
    // the def's first call parameter that the class does not have.
    for o in &outcomes {
        for (row, refusal) in o.refused() {
            assert!(
                matches!(
                    refusal,
                    ConvertRefusal::UnknownField {
                        role: FieldRole::Reads,
                        ..
                    }
                ),
                "{} {}: {refusal:?}",
                o.domain,
                row.predicate
            );
        }
    }
    let ocr_first = |p: &str| {
        outcomes[0]
            .rows
            .iter()
            .find(|r| r.predicate == p)
            .and_then(|r| r.result.as_ref().err().cloned())
    };
    assert_eq!(
        ocr_first("recognize_line"),
        Some(ConvertRefusal::UnknownField {
            role: FieldRole::Reads,
            name: "grey_line".into()
        })
    );
}

/// `MethodId` is the def's position in its table. This pins today's ids: a
/// reorder, insertion or removal in a table renumbers every later def, and
/// fails here.
#[test]
fn method_ids_are_pinned_to_the_tables_declared_order() {
    let (_, outcomes) = census();
    let id_of = |domain: &str, predicate: &str| {
        outcomes
            .iter()
            .find(|o| o.domain == domain)
            .and_then(|o| o.rows.iter().find(|r| r.predicate == predicate))
            .map(|r| r.id)
    };
    // Literal anchors: first, middle and last of each registered domain, and a
    // refused row (which keeps its number).
    assert_eq!(id_of("geo", "locate_point"), Some(MethodId(0)));
    assert_eq!(id_of("geo", "street_edges"), Some(MethodId(4)));
    assert_eq!(id_of("geo", "polyline_length"), Some(MethodId(5)));
    assert_eq!(id_of("healthcare", "register_patient"), Some(MethodId(0)));
    assert_eq!(id_of("healthcare", "add_diagnosis"), Some(MethodId(4)));
    assert_eq!(id_of("healthcare", "add_vital_sign"), Some(MethodId(17)));
    assert_eq!(
        id_of("healthcare", "delete_vital_signs"),
        Some(MethodId(19))
    );
    assert_eq!(id_of("erp", "create_entry"), Some(MethodId(0)));
    assert_eq!(id_of("erp", "record_time_entry"), Some(MethodId(5)));
    assert_eq!(id_of("erp", "emit_row"), Some(MethodId(6)));
    assert_eq!(id_of("ocr", "harvest_fields"), Some(MethodId(10)));
    assert_eq!(id_of("document", "reconstruct_document"), Some(MethodId(2)));

    // And in general: row i of every table has id i, and the registered ids are
    // exactly the rows that converted, in order.
    for o in &outcomes {
        for (i, row) in o.rows.iter().enumerate() {
            assert_eq!(row.id, MethodId(i as u32), "{} {}", o.domain, row.predicate);
        }
    }
    assert_eq!(
        outcomes[4].method_ids(),
        (0..7).map(MethodId).collect::<Vec<_>>()
    );
    assert!(outcomes[0].method_ids().is_empty());
}

/// Every registered method is found under its class by the one lookup, with
/// its name, and a reference the store issues for it is admitted.
#[test]
fn every_registered_method_resolves_under_its_class() {
    let (store, outcomes) = census();
    let view = OgarClassView::new();
    let mut found = 0;
    for o in &outcomes {
        for reg in o.registered() {
            let resolved = store
                .resolve(&view, reg.class, REV)
                .unwrap_or_else(|e| panic!("{}: {e}", reg.capability));
            let m = resolved
                .methods
                .iter()
                .find(|m| m.id == reg.id)
                .unwrap_or_else(|| panic!("{} not under {:#x}", reg.capability, reg.class));
            assert_eq!(m.name, reg.capability, "name is carried");
            assert!(
                resolved.action_ordinals().any(|id| id == reg.id),
                "{} is an action ordinal of its class",
                reg.capability
            );
            assert!(m.effects.is_empty(), "no Frozen word for the hand-off");
            let r = store.method_ref(reg.class, reg.id).expect("issued");
            assert_eq!(store.admit(&r).expect("admitted").id, reg.id);
            found += 1;
        }
    }
    assert_eq!(found, 33);
    // Six classes carry behaviour: patient, diagnosis, lab_value, medication,
    // treatment, vital_sign (6) + osm_node, osm_way (2) + erp_polyfill (1).
    assert_eq!(store.class_count(), 9);
}

/// A method registered under one class is not found under another.
#[test]
fn a_method_is_found_only_under_its_own_class() {
    let (store, _) = census();
    let view = OgarClassView::new();
    let patient = render_classid(APP, class_ids::PATIENT);
    let diagnosis = render_classid(APP, class_ids::DIAGNOSIS);
    let on_patient = store.resolve(&view, patient, REV).unwrap();
    let on_diagnosis = store.resolve(&view, diagnosis, REV).unwrap();
    assert!(
        on_patient
            .methods
            .iter()
            .any(|m| m.name == "register_patient")
    );
    assert!(
        !on_diagnosis
            .methods
            .iter()
            .any(|m| m.name == "register_patient")
    );
    assert_eq!(on_patient.methods.len(), 4);
    assert_eq!(on_diagnosis.methods.len(), 4);
    // An unregistered classid is UNKNOWN, never "no methods".
    let ocr = render_classid(APP, class_ids::TEXTLINE);
    assert_eq!(
        store.resolve(&view, ocr, REV).unwrap_err(),
        Refusal::UnknownClass(ocr)
    );
}

/// The registered table is idempotent: the same table over the same basis
/// again is accepted (DT64's D5), and a different binding of a bound class is
/// refused as `BehaviourImmutable`, surfaced as `TableRefusal::Dt64`.
#[test]
fn re_registering_is_idempotent_and_a_changed_binding_is_refused() {
    let view = OgarClassView::new();
    let tables = domain_tables();
    let erp = tables.iter().find(|t| t.domain == "erp").unwrap();
    let mut store = BindStore::new();
    let first = register_domain_table(&mut store, &view, erp, APP, REV).unwrap();
    let again = register_domain_table(&mut store, &view, erp, APP, REV).unwrap();
    assert_eq!(first, again);
    assert_eq!(store.class_count(), 1);

    fn erp_without_the_last() -> Vec<ActionDef> {
        let mut defs = ogar_vocab::erp_actions::erp_actions();
        defs.pop();
        defs
    }
    let shorter = DomainTable {
        domain: "erp",
        expected_executors: erp.expected_executors,
        entries: erp.entries,
        actions: erp_without_the_last,
    };
    let class = render_classid(APP, class_ids::ERP_POLYFILL);
    assert_eq!(
        register_domain_table(&mut store, &view, &shorter, APP, REV),
        Err(TableRefusal::Dt64 {
            class,
            refusal: Refusal::BehaviourImmutable(class)
        })
    );
}

/// The HotPlug-gated registration: OGAR's own `resolve_hotplug` runs first.
/// Green registers; the wrong consumer, an incomplete cover and an extra cover
/// are each named drift and leave the store unchanged.
#[test]
fn registration_is_gated_on_the_hot_plug_activation() {
    let view = OgarClassView::new();
    let tables = domain_tables();
    let table = tables.iter().find(|t| t.domain == "healthcare").unwrap();
    let covered = ogar_vocab::healthcare_actions::HEALTHCARE_ACTION_NAMES;

    let mut store = BindStore::new();
    let wrong = register_domain_table_activated(
        &mut store,
        &view,
        table,
        "tesseract-ogar",
        covered,
        APP,
        REV,
    );
    assert!(matches!(
        wrong,
        Err(TableRefusal::NotActivated(
            HotplugDrift::UnexpectedConsumer(_)
        ))
    ));
    let mut missing = covered.to_vec();
    missing.pop();
    let short =
        register_domain_table_activated(&mut store, &view, table, "medcare-rs", &missing, APP, REV);
    assert!(matches!(
        short,
        Err(TableRefusal::NotActivated(HotplugDrift::Uncovered(_)))
    ));
    let mut extra = covered.to_vec();
    extra.push("does_not_exist");
    let over =
        register_domain_table_activated(&mut store, &view, table, "medcare-rs", &extra, APP, REV);
    assert!(matches!(
        over,
        Err(TableRefusal::NotActivated(HotplugDrift::Undeclared(_)))
    ));
    assert_eq!(store.class_count(), 0, "DT64 was never called");

    let green =
        register_domain_table_activated(&mut store, &view, table, "medcare-rs", covered, APP, REV)
            .expect("green");
    assert_eq!(green.registered().count(), 20);
    assert_eq!(store.class_count(), 6);
}

// ── method_of: the mapping and the named refusals ───────────────────────────

/// A name-only external def is an unguarded, effect-free method carrying its
/// name; its parameters that ARE fields become the read mask.
#[test]
fn an_external_def_maps_name_reads_and_an_empty_effect_list() {
    let view = textline_view();
    let mut d = def("recognize_line");
    d.kausal = Some(KausalSpec::External);
    d.reads = vec!["text".into(), "width".into()];
    d.writes = vec!["text".into()];
    let m = method_of(&d, MethodId(7), &view).expect("converts");
    assert_eq!(m.id, MethodId(7));
    assert_eq!(m.name, "recognize_line");
    assert!(m.when.is_none());
    assert!(m.on.is_empty());
    assert!(m.reads.has(1) && m.reads.has(2) && !m.reads.has(0));
    assert!(m.effects.is_empty());
}

/// A `reads` name that is not a field of the class is refused by name, not
/// dropped and not mapped to a neighbour.
#[test]
fn an_unknown_read_is_refused_by_name() {
    let view = textline_view();
    let mut d = def("recognize_line");
    d.reads = vec!["text".into(), "grey_line".into()];
    assert_eq!(
        method_of(&d, MethodId(0), &view),
        Err(ConvertRefusal::UnknownField {
            role: FieldRole::Reads,
            name: "grey_line".into()
        })
    );
}

/// A state guard over a field of the class, with decimal-literal values, is a
/// Kleene `SetExpr` equality over that field's lane.
#[test]
fn a_numeric_state_guard_becomes_a_set_expr_over_the_fields_lane() {
    let view = textline_view();
    let mut d = def("confirm");
    d.kausal = Some(KausalSpec::StateGuard {
        guard_field: "state".into(),
        guard_values: vec!["1".into()],
    });
    let m = method_of(&d, MethodId(0), &view).unwrap();
    let g = m.when.expect("guarded");
    assert_eq!(g.expr, SetExpr::eq_u32(0, 1));
    assert_eq!(g.logic, Logic::Kleene);

    d.kausal = Some(KausalSpec::StateGuard {
        guard_field: "text".into(),
        guard_values: vec!["1".into(), "3".into()],
    });
    let g = method_of(&d, MethodId(0), &view).unwrap().when.unwrap();
    assert_eq!(
        g.expr,
        SetExpr::Or(vec![SetExpr::eq_u32(1, 1), SetExpr::eq_u32(1, 3)])
    );
}

/// What a `SetExpr` equality cannot state is refused as `UnportedGuard`, never
/// approximated: a string state, an empty set, and every non-guard kausal.
#[test]
fn a_guard_that_set_expr_cannot_state_is_refused_not_approximated() {
    let view = textline_view();
    let kausal = |k| {
        let mut d = def("x");
        d.kausal = Some(k);
        method_of(&d, MethodId(0), &view)
    };
    let unported =
        |r: Result<_, ConvertRefusal>| matches!(r, Err(ConvertRefusal::UnportedGuard { .. }));

    assert!(unported(kausal(KausalSpec::StateGuard {
        guard_field: "state".into(),
        guard_values: vec!["draft".into(), "sent".into()],
    })));
    assert!(unported(kausal(KausalSpec::StateGuard {
        guard_field: "state".into(),
        guard_values: vec![],
    })));
    assert!(unported(kausal(KausalSpec::LifecycleTrigger {
        event: "before_save".into()
    })));
    assert!(unported(kausal(KausalSpec::Constrains {
        paths: vec!["text".into()]
    })));
    assert!(unported(kausal(KausalSpec::Onchange {
        paths: vec!["text".into()]
    })));
    assert!(unported(kausal(KausalSpec::ContextDepends {
        keys: vec!["lang".into()]
    })));
    // The reason names the variant and what failed.
    match kausal(KausalSpec::StateGuard {
        guard_field: "state".into(),
        guard_values: vec!["draft".into()],
    }) {
        Err(ConvertRefusal::UnportedGuard { kausal }) => {
            assert!(
                kausal.starts_with("StateGuard:") && kausal.contains("draft"),
                "{kausal}"
            )
        }
        other => panic!("{other:?}"),
    }
    // A guard over a field the class lacks is UnknownField, not UnportedGuard.
    assert_eq!(
        kausal(KausalSpec::StateGuard {
            guard_field: "missing".into(),
            guard_values: vec!["1".into()],
        }),
        Err(ConvertRefusal::UnknownField {
            role: FieldRole::GuardField,
            name: "missing".into()
        })
    );
}

/// `Depends` over fields of the class is ON; `methods_on` finds the method
/// under exactly those fields.
#[test]
fn depends_paths_become_on_and_methods_on_finds_them() {
    let view = textline_view();
    let mut d = def("recompute");
    d.kausal = Some(KausalSpec::Depends {
        paths: vec!["text".into(), "width".into()],
    });
    let m = method_of(&d, MethodId(0), &view).unwrap();
    assert!(m.on.has(1) && m.on.has(2) && !m.on.has(0));
    assert!(m.when.is_none());

    d.kausal = Some(KausalSpec::Depends {
        paths: vec!["text.nested".into()],
    });
    assert_eq!(
        method_of(&d, MethodId(0), &view),
        Err(ConvertRefusal::UnknownField {
            role: FieldRole::DependsPath,
            name: "text.nested".into()
        })
    );

    // Through the store: register one class with a Depends method and look it
    // up by the field that triggers it.
    fn recompute() -> Vec<ActionDef> {
        let mut d = ActionDef::new(
            "ogit-ocr/textline::action_def::recompute",
            "recompute",
            "ogit-ocr/textline",
        );
        d.kausal = Some(KausalSpec::Depends {
            paths: vec!["text".into()],
        });
        vec![d]
    }
    fn none() -> Vec<(String, u16)> {
        Vec::new()
    }
    let table = DomainTable {
        domain: "probe",
        expected_executors: &[],
        entries: none,
        actions: recompute,
    };
    let mut store = BindStore::new();
    let out = register_domain_table(&mut store, &view, &table, APP, REV).unwrap();
    let class = render_classid(APP, class_ids::TEXTLINE);
    assert_eq!(out.method_ids(), vec![MethodId(0)]);
    let r = store.resolve(&view, class, REV).unwrap();
    assert_eq!(r.methods_on(1).count(), 1);
    assert_eq!(r.methods_on(0).count(), 0);
}

/// An `on_enter` transition has no Frozen word: refused, not dropped. An
/// unminted object class has no classid: refused.
#[test]
fn an_on_enter_effect_and_an_unminted_concept_are_refused() {
    let view = textline_view();
    let mut d = def("confirm");
    d.on_enter = Some(ogar_vocab::EnterEffect::transition("state", "1"));
    assert_eq!(
        method_of(&d, MethodId(0), &view),
        Err(ConvertRefusal::UnportedEffect { what: "on_enter" })
    );
    let stray = ActionDef::new("x", "y", "any:ns/totally_not_a_concept");
    assert_eq!(
        method_of(&stray, MethodId(0), &view),
        Err(ConvertRefusal::UnmintedConcept {
            concept: "totally_not_a_concept".into()
        })
    );
}
