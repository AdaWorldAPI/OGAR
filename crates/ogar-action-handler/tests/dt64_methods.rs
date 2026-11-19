//! The `dt64` converter against the five authoritative domain tables and the
//! refusals it names. Run with `--features dt64`.
#![cfg(feature = "dt64")]

use lance_graph_contract::class_view::{ClassId, ClassView};
use lance_graph_contract::ontology::{DisplayTemplate, FieldRef};
use lance_graph_contract::set_algebra::{Logic, SetExpr};
use lance_graph_dt64::{BindStore, MethodId, Refusal, SchemaRev};
use ogar_action_handler::dt64::{
    BehaviourArmRefusal, BehaviourView, CanonicalClasses, ClassMap, ConvertRefusal, FieldRole,
    LedgerRefusal, TableOutcome, TableRefusal, method_of, register_domain_table,
    register_domain_table_activated, register_domain_table_activated_with_arms,
    register_domain_table_for, register_domain_table_with_arms,
};
use ogar_class_view::OgarClassView;
use ogar_vocab::arm_ledger::{CANONICAL, ConsumerArms, canonical_arm};
use ogar_vocab::capability_registry::{DomainTable, HotplugDrift, domain_tables, resolve_hotplug};
use ogar_vocab::effect_class::{EffectClass, classify};
use ogar_vocab::{ActionDef, CallParam, KausalSpec, class_ids};

/// Ignored by the converter (the right half of a classid is the behaviour arm);
/// kept because the entry points still take it.
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

/// A consumer ledger for a test table: each def's identity on a consumer arm,
/// `0xF800` upward in table order (a repeated identity keeps its first arm).
fn probe_arms(table: &DomainTable) -> ConsumerArms {
    let mut arms = ConsumerArms::new();
    let mut seen: Vec<String> = Vec::new();
    for d in (table.actions)() {
        if !seen.contains(&d.identity) {
            arms = arms.with(&d.identity, 0xF800 + seen.len() as u16);
            seen.push(d.identity);
        }
    }
    arms
}

/// Register a test table of behaviours OGAR does not know, under OGAR's canon
/// classes, with consumer arms from [`probe_arms`].
fn register_probe<V: ClassView + ?Sized>(
    store: &mut BindStore,
    view: &V,
    table: &DomainTable,
) -> Result<TableOutcome, TableRefusal> {
    register_domain_table_with_arms(
        store,
        view,
        &CanonicalClasses,
        &probe_arms(table),
        table,
        REV,
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
/// 14 / 6 / 20 / 3 / 7 per domain. All 50 register: the ocr and document defs
/// (17) were refused while their call parameters sat in `reads`, and are
/// registered now that call parameters are declared on the def (`params`).
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
            ("ocr", 14, 14, 0),
            ("geo", 6, 6, 0),
            ("healthcare", 20, 20, 0),
            ("document", 3, 3, 0),
            ("erp", 7, 7, 0),
        ]
    );
    let registered: usize = outcomes.iter().map(|o| o.registered().count()).sum();
    let refused: usize = outcomes.iter().map(|o| o.refused().count()).sum();
    assert_eq!((registered, refused), (50, 0));
    assert_eq!(registered + refused, 50);
}

/// `reads` and `writes` of every def name stored fields of the def's class
/// only: the call's inputs and outputs are on the def as `params` and
/// `results`, and the class has no field for any of them.
#[test]
fn reads_and_writes_are_stored_fields_and_call_parameters_are_not_class_fields() {
    let data = OgarClassView::new();
    let mut with_params = 0;
    for def in all_defs() {
        let concept = def.object_class.rsplit('/').next().unwrap();
        let class = ogar_vocab::canonical_concept_id(concept).expect("minted");
        let labels: Vec<&str> = data
            .fields(class)
            .iter()
            .map(|f| f.label.as_ref())
            .collect();
        for name in def.reads.iter().chain(&def.writes) {
            assert!(
                labels.contains(&name.as_str()),
                "{}: `{name}` is not a stored field of {concept}",
                def.identity
            );
        }
        if !def.params.is_empty() || !def.results.is_empty() {
            with_params += 1;
            // The 17 ocr/document defs: no stored field is read or written.
            assert!(
                def.reads.is_empty() && def.writes.is_empty(),
                "{}",
                def.identity
            );
            // Declaring a parameter did not add a field to the class: none of
            // the call's names is a field of it.
            for name in def
                .params
                .iter()
                .map(|p| &*p.name)
                .chain(def.results.iter().map(String::as_str))
            {
                assert!(
                    !labels.contains(&name),
                    "{}: `{name}` appeared as a field of {concept}",
                    def.identity
                );
            }
        }
    }
    assert_eq!(
        with_params, 17,
        "ocr 14 + document 3 declare a call signature"
    );
}

/// The registration exposes the def's parameter list beside the method, and the
/// method itself stays class-fields-only: the class's field basis is unchanged
/// and the read mask is empty.
#[test]
fn the_registration_carries_the_call_parameters_beside_a_class_fields_only_method() {
    let (store, outcomes) = census();
    let data = OgarClassView::new();
    let view = BehaviourView::from_outcomes(&data, &outcomes);
    let ocr = &outcomes[0];
    let rl = ocr
        .registered()
        .find(|r| r.capability == "recognize_line")
        .expect("recognize_line registers");
    assert_eq!(
        rl.params,
        vec![
            CallParam::required("grey_line"),
            CallParam::required("width"),
            CallParam::required("height"),
            CallParam::optional("with_dict"),
        ]
    );
    assert_eq!(
        rl.results,
        vec!["text".to_string(), "unichar_ids".to_string()]
    );
    let resolved = store.resolve(&view, rl.behaviour, REV).unwrap();
    assert_eq!(resolved.fields, data.fields(class_ids::TEXTLINE));
    assert!(resolved.fields.iter().all(|f| f.label != "grey_line"));
    let m = &resolved.methods[0];
    assert!(m.reads.is_empty() && m.on.is_empty() && m.when.is_none());
    // Every registered row carries exactly its def's lists.
    for o in &outcomes {
        for reg in o.registered() {
            let def = all_defs()
                .into_iter()
                .find(|d| d.identity == reg.identity)
                .unwrap();
            assert_eq!(reg.params, def.params, "{}", reg.capability);
            assert_eq!(reg.results, def.results, "{}", reg.capability);
        }
    }
    // The optional/required split survives (reconstruct_document).
    let rec = outcomes[3]
        .registered()
        .find(|r| r.capability == "reconstruct_document")
        .unwrap();
    assert_eq!(
        rec.params,
        vec![
            CallParam::required("document_guid"),
            CallParam::optional("template")
        ]
    );
}

/// A def is never refused because it HAS call parameters, whatever they are
/// called; and a call parameter in `reads` is refused by name, as an unknown
/// field, even on a real OCR def.
#[test]
fn call_parameters_never_refuse_a_def_but_a_call_parameter_in_reads_does() {
    let view = textline_view();
    let mut d = def("recognize_line");
    d.kausal = Some(KausalSpec::External);
    d.params = vec![
        CallParam::required("grey_line"),
        CallParam::optional("with_dict"),
    ];
    d.results = vec!["text".into()];
    let m = method_of(&d, MethodId(0), &view).expect("params are not fields, and not refused");
    assert!(m.reads.is_empty());

    // The same def with a parameter name moved back into `reads`.
    d.reads = vec!["grey_line".into()];
    assert_eq!(
        method_of(&d, MethodId(0), &view),
        Err(ConvertRefusal::UnknownField {
            role: FieldRole::Reads,
            name: "grey_line".into()
        })
    );
    // On the real table row (through OGAR's class view).
    let data = OgarClassView::new();
    let mut real = all_defs()
        .into_iter()
        .find(|d| d.predicate == "recognize_line")
        .unwrap();
    assert!(method_of(&real, MethodId(0), &data).is_ok());
    real.reads.push("grey_line".into());
    assert_eq!(
        method_of(&real, MethodId(0), &data),
        Err(ConvertRefusal::UnknownField {
            role: FieldRole::Reads,
            name: "grey_line".into()
        })
    );
}

/// A guard is over stored fields only: a guard field or `Depends` path that is
/// a call parameter (and not a field) is refused by name. A name that is both a
/// call parameter and a stored field is the field, and a name that is neither
/// stays an unknown field.
#[test]
fn a_guard_over_a_call_parameter_is_refused_by_name() {
    let view = textline_view(); // fields: state, text, width
    let mut d = def("confirm");
    d.params = vec![
        CallParam::required("grey_line"),
        CallParam::required("width"),
    ];
    let guard = |field: &str| {
        let mut g = d.clone();
        g.kausal = Some(KausalSpec::StateGuard {
            guard_field: field.into(),
            guard_values: vec!["1".into()],
        });
        method_of(&g, MethodId(0), &view)
    };
    assert_eq!(
        guard("grey_line"),
        Err(ConvertRefusal::CallParameterInGuard {
            role: FieldRole::GuardField,
            name: "grey_line".into()
        })
    );
    // `width` is a call parameter AND a stored field: the guard is over the field.
    assert!(guard("width").is_ok());
    // Neither: unknown field, as before.
    assert_eq!(
        guard("nothing"),
        Err(ConvertRefusal::UnknownField {
            role: FieldRole::GuardField,
            name: "nothing".into()
        })
    );
    let mut dep = d.clone();
    dep.kausal = Some(KausalSpec::Depends {
        paths: vec!["text".into(), "grey_line".into()],
    });
    assert_eq!(
        method_of(&dep, MethodId(0), &view),
        Err(ConvertRefusal::CallParameterInGuard {
            role: FieldRole::DependsPath,
            name: "grey_line".into()
        })
    );
}

/// The behaviour arm (right half of `class :: F###`) of every one of the 50
/// defs, by identity, in table order: the canonical ledger as it was first
/// handed out (`0xF001` upward). Pinned here and in `ogar_vocab::arm_ledger`'s
/// own tests; independent of the consumer.
const GOLDEN_ARMS: [(&str, u16); 50] = [
    ("ogit-ocr/textline::action_def::recognize_line", 0xF001),
    ("ogit-ocr/page_image::action_def::recognize_page", 0xF002),
    (
        "ogit-ocr/page_image::action_def::extract_text_layer",
        0xF003,
    ),
    (
        "ogit-ocr/page_image::action_def::extract_page_image",
        0xF004,
    ),
    ("ogit-ocr/ocr_renderer::action_def::render_text", 0xF005),
    ("ogit-ocr/ocr_renderer::action_def::render_tsv", 0xF006),
    ("ogit-ocr/ocr_renderer::action_def::render_hocr", 0xF007),
    (
        "ogit-ocr/ocr_renderer::action_def::render_searchable_pdf",
        0xF008,
    ),
    (
        "ogit-ocr/page_image::action_def::recognize_page_words",
        0xF009,
    ),
    (
        "ogit-ocr/page_image::action_def::recognize_document",
        0xF00A,
    ),
    ("ogit-ocr/page_layout::action_def::harvest_fields", 0xF00B),
    ("ogit-ocr/page_image::action_def::segment_page", 0xF00C),
    (
        "ogit-ocr/page_image::action_def::detect_halftone_regions",
        0xF00D,
    ),
    (
        "ogit-ocr/page_layout::action_def::detect_page_furniture",
        0xF00E,
    ),
    ("ogit-geo/osm_node::action_def::locate_point", 0xF00F),
    ("ogit-geo/osm_node::action_def::locate_tile", 0xF010),
    ("ogit-geo/osm_node::action_def::locate_fragment", 0xF011),
    ("ogit-geo/osm_node::action_def::project_fields", 0xF012),
    ("ogit-geo/osm_way::action_def::street_edges", 0xF013),
    ("ogit-geo/osm_way::action_def::polyline_length", 0xF014),
    ("ogit-health/patient::action_def::register_patient", 0xF015),
    (
        "ogit-health/patient::action_def::get_patient_record",
        0xF016,
    ),
    ("ogit-health/patient::action_def::list_patients", 0xF017),
    (
        "ogit-health/patient::action_def::update_patient_access",
        0xF018,
    ),
    ("ogit-health/diagnosis::action_def::add_diagnosis", 0xF019),
    ("ogit-health/diagnosis::action_def::get_diagnosis", 0xF01A),
    ("ogit-health/diagnosis::action_def::list_diagnoses", 0xF01B),
    (
        "ogit-health/diagnosis::action_def::delete_diagnosis",
        0xF01C,
    ),
    ("ogit-health/lab_value::action_def::add_lab_result", 0xF01D),
    ("ogit-health/lab_value::action_def::get_lab_result", 0xF01E),
    (
        "ogit-health/lab_value::action_def::list_lab_results",
        0xF01F,
    ),
    ("ogit-health/medication::action_def::add_medication", 0xF020),
    ("ogit-health/medication::action_def::get_medication", 0xF021),
    (
        "ogit-health/medication::action_def::list_medications",
        0xF022,
    ),
    ("ogit-health/treatment::action_def::add_treatment", 0xF023),
    ("ogit-health/treatment::action_def::get_treatment", 0xF024),
    ("ogit-health/treatment::action_def::list_treatments", 0xF025),
    ("ogit-health/vital_sign::action_def::add_vital_sign", 0xF026),
    (
        "ogit-health/vital_sign::action_def::list_vital_signs",
        0xF027,
    ),
    (
        "ogit-health/vital_sign::action_def::delete_vital_signs",
        0xF028,
    ),
    ("ogit-ocr/document::action_def::persist_document", 0xF029),
    ("ogit-ocr/document::action_def::read_document", 0xF02A),
    (
        "ogit-ocr/document::action_def::reconstruct_document",
        0xF02B,
    ),
    ("ogit-erp/erp_polyfill::action_def::create_entry", 0xF02C),
    ("ogit-erp/erp_polyfill::action_def::post_entry", 0xF02D),
    ("ogit-erp/erp_polyfill::action_def::read_entry", 0xF02E),
    ("ogit-erp/erp_polyfill::action_def::set_const", 0xF02F),
    ("ogit-erp/erp_polyfill::action_def::mark_set", 0xF030),
    (
        "ogit-erp/erp_polyfill::action_def::record_time_entry",
        0xF031,
    ),
    ("ogit-erp/erp_polyfill::action_def::emit_row", 0xF032),
];

fn all_defs() -> Vec<ActionDef> {
    domain_tables().iter().flat_map(|t| (t.actions)()).collect()
}

/// The arm is the def's own number: the 50 arms are pinned, every one is in
/// `0xF001..=0xFFFF`, and a refused row keeps its arm as its id.
#[test]
fn the_behaviour_arms_of_the_fifty_defs_are_pinned() {
    let defs = all_defs();
    assert_eq!(defs.len(), 50);
    for (def, (identity, arm)) in defs.iter().zip(GOLDEN_ARMS) {
        assert_eq!(def.identity, identity, "the golden list is in table order");
        assert_eq!(canonical_arm(&def.identity), Some(arm), "{identity}");
        assert!((0xF001..=0xFFFF).contains(&arm), "{identity}");
    }
    let (_, outcomes) = census();
    let rows: Vec<_> = outcomes.iter().flat_map(|o| o.rows.iter()).collect();
    assert_eq!(rows.len(), 50);
    assert_eq!(CANONICAL.len(), 50);
    for (row, (_, arm)) in rows.iter().zip(GOLDEN_ARMS) {
        assert_eq!(row.id, MethodId(u32::from(arm)), "{}", row.predicate);
        if let Ok(reg) = &row.result {
            assert_eq!(reg.arm, arm);
        }
    }
}

/// Every registered method's id has its right half in `0xF000..=0xFFFF` and
/// its left half the class it applies on (the subject the def declares, where
/// the consumer is OGAR's canon); ids are unique.
#[test]
fn every_registered_id_is_class_over_an_f_arm_and_unique() {
    let (_, outcomes) = census();
    let mut seen = std::collections::BTreeSet::new();
    for o in &outcomes {
        for reg in o.registered() {
            assert!((0xF000..=0xFFFF).contains(&(reg.behaviour & 0xFFFF)));
            assert_eq!((reg.behaviour & 0xFFFF) as u16, reg.arm);
            assert_eq!((reg.behaviour >> 16) as u16, reg.on);
            assert_eq!(reg.class, u32::from(reg.on) << 16);
            let def = all_defs()
                .into_iter()
                .find(|d| d.identity == reg.identity)
                .unwrap();
            let subject = def.object_class.rsplit('/').next().unwrap().to_string();
            assert_eq!(
                Some(reg.on),
                ogar_vocab::canonical_concept_id(&subject),
                "{}",
                reg.capability
            );
            assert!(seen.insert(reg.behaviour), "{:#x} twice", reg.behaviour);
        }
    }
    assert_eq!(seen.len(), 50);
}

/// One consumer's full ids, pinned: OGAR's canon as the consumer (erp seat,
/// patient).
#[test]
fn the_canonical_consumers_full_ids_are_pinned() {
    let (_, outcomes) = census();
    let full = |domain: &str, predicate: &str| {
        outcomes
            .iter()
            .find(|o| o.domain == domain)
            .and_then(|o| o.registered().find(|r| r.capability == predicate))
            .map(|r| r.behaviour)
    };
    assert_eq!(class_ids::ERP_POLYFILL, 0x020C);
    assert_eq!(full("erp", "create_entry"), Some(0x020C_F02C));
    assert_eq!(full("erp", "emit_row"), Some(0x020C_F032));
    assert_eq!(
        full("healthcare", "register_patient"),
        Some(u32::from(class_ids::PATIENT) << 16 | 0xF015)
    );
    assert_eq!(
        full("geo", "street_edges"),
        Some(u32::from(class_ids::OSM_WAY) << 16 | 0xF013)
    );
}

/// The arm does not depend on the table: a table holding only one more def, or
/// the defs reversed or cut down, leaves every earlier def's id unchanged.
#[test]
fn registering_one_more_def_leaves_every_earlier_id_unchanged() {
    let view = OgarClassView::new();
    let erp = domain_tables()
        .into_iter()
        .find(|t| t.domain == "erp")
        .unwrap();
    let mut a = BindStore::new();
    let before = register_domain_table(&mut a, &view, &erp, APP, REV).unwrap();

    fn erp_plus_one() -> Vec<ActionDef> {
        let mut defs = ogar_vocab::erp_actions::erp_actions();
        defs.insert(
            2,
            ActionDef::new(
                "ogit-erp/erp_polyfill::action_def::cancel_entry",
                "cancel_entry",
                "ogit-erp/erp_polyfill",
            ),
        );
        defs.reverse();
        defs
    }
    let bigger = DomainTable {
        domain: "erp",
        expected_executors: erp.expected_executors,
        entries: erp.entries,
        actions: erp_plus_one,
    };
    // Into the SAME store: the earlier ids are accepted as they were (D5), the
    // new def lands on its own id.
    // `cancel_entry` is not in OGAR's ledger: the consumer supplies its arm.
    let arms = ConsumerArms::new().with("ogit-erp/erp_polyfill::action_def::cancel_entry", 0xF800);
    let after =
        register_domain_table_with_arms(&mut a, &view, &CanonicalClasses, &arms, &bigger, REV)
            .unwrap();
    for r in before.registered() {
        let again = after
            .registered()
            .find(|x| x.identity == r.identity)
            .expect("still registered");
        assert_eq!(again.behaviour, r.behaviour, "{}", r.capability);
    }
    assert_eq!(after.registered().count(), 8);
    let cancel = after
        .registered()
        .find(|r| r.capability == "cancel_entry")
        .unwrap();
    assert_eq!(
        cancel.arm, 0xF800,
        "the consumer's own arm, apart from OGAR's"
    );
    assert_eq!(a.class_count(), 8);
}

/// Every registered method is found under its behaviour classid by the one
/// lookup, with its name, and a reference the store issues for it is admitted.
/// The behaviour's basis is the class's fields, read by the full classid.
#[test]
fn every_registered_method_resolves_under_its_class() {
    let (store, outcomes) = census();
    let data = OgarClassView::new();
    let view = BehaviourView::from_outcomes(&data, &outcomes);
    let mut found = 0;
    for o in &outcomes {
        for reg in o.registered() {
            let resolved = store
                .resolve(&view, reg.behaviour, REV)
                .unwrap_or_else(|e| panic!("{}: {e}", reg.capability));
            assert_eq!(resolved.fields.len(), data.fields(reg.on).len());
            assert_eq!(
                resolved.methods.len(),
                1,
                "one method per behaviour classid"
            );
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
            let r = store.method_ref(reg.behaviour, reg.id).expect("issued");
            assert_eq!(store.admit(&r).expect("admitted").id, reg.id);
            found += 1;
        }
    }
    assert_eq!(found, 50);
    // One classid per behaviour: fourteen classes carry them (textline,
    // page_image, page_layout, ocr_renderer, document, patient, diagnosis,
    // lab_value, medication, treatment, vital_sign, osm_node, osm_way,
    // erp_polyfill), 50 behaviours in all.
    assert_eq!(store.class_count(), 50);
}

/// Every registered method carries an effect class, the 50 are all
/// `ExternalHost` (`External` defs naming no stored-field effect), and no method whose class is not
/// F000-admissible carries F000 effects: a write, lifecycle step, transaction
/// or external call is dispatched through the gate, never run as a fold.
#[test]
fn every_registered_method_carries_an_effect_class_and_only_folds_may_carry_f000() {
    let (store, outcomes) = census();
    let data = OgarClassView::new();
    let view = BehaviourView::from_outcomes(&data, &outcomes);
    let mut by_class: std::collections::BTreeMap<&str, usize> = Default::default();
    for o in &outcomes {
        for reg in o.registered() {
            *by_class.entry(reg.effect.name()).or_default() += 1;
            // The class is exactly what `classify` says of the def.
            let def = (domain_tables()
                .iter()
                .find(|t| t.domain == o.domain)
                .unwrap()
                .actions)()
            .into_iter()
            .find(|d| d.identity == reg.identity)
            .expect("identity finds the def");
            assert_eq!(reg.effect, classify(&def), "{}", reg.capability);

            let resolved = store.resolve(&view, reg.behaviour, REV).unwrap();
            let m = resolved.methods.iter().find(|m| m.id == reg.id).unwrap();
            assert!(
                reg.effect.is_f000_admissible() || m.effects.is_empty(),
                "{} is {:?} and must not carry F000 effects",
                reg.capability,
                reg.effect
            );
            assert!(reg.effect.requires_gate(), "{}", reg.capability);
        }
    }
    assert_eq!(
        by_class,
        std::collections::BTreeMap::from([("external_host", 50)])
    );
    // The 17 ocr and document defs were LifecycleWrite while their output
    // names sat in `writes`; those are call results now, so they are
    // ExternalHost like the rest.
    let moved: Vec<_> = domain_tables()
        .iter()
        .filter(|t| t.domain == "ocr" || t.domain == "document")
        .flat_map(|t| (t.actions)())
        .map(|d| classify(&d))
        .collect();
    assert_eq!(moved.len(), 17);
    assert!(moved.iter().all(|c| *c == EffectClass::ExternalHost));
}

/// A def that names a write registers with its class and empty F000 effects:
/// the class, not the converter, is what keeps a write off the lens.
#[test]
fn a_write_def_registers_as_a_lifecycle_write_with_no_f000_effects() {
    fn writer() -> Vec<ActionDef> {
        let mut d = ActionDef::new(
            "ogit-ocr/textline::action_def::rewrite",
            "rewrite",
            "ogit-ocr/textline",
        );
        d.writes = vec!["text".into()];
        let mut c = ActionDef::new(
            "ogit-ocr/textline::action_def::after_commit_hook",
            "after_commit_hook",
            "ogit-ocr/textline",
        );
        // `OnCommit` alone is not commit-stage evidence (it also covers Odoo
        // `@api.depends`), so this def stays Unresolved and needs the gate.
        c.default_temporal = ogar_vocab::TemporalSpec::OnCommit;
        let bare = ActionDef::new(
            "ogit-ocr/textline::action_def::bare",
            "bare",
            "ogit-ocr/textline",
        );
        vec![d, c, bare]
    }
    fn none() -> Vec<(String, u16)> {
        Vec::new()
    }
    let view = textline_view();
    let table = DomainTable {
        domain: "probe",
        expected_executors: &[],
        entries: none,
        actions: writer,
    };
    let mut store = BindStore::new();
    let out = register_probe(&mut store, &view, &table).unwrap();
    let classes: Vec<_> = out.registered().map(|r| r.effect).collect();
    assert_eq!(
        classes,
        vec![
            EffectClass::LifecycleWrite,
            EffectClass::Unresolved,
            EffectClass::Unresolved
        ]
    );
    let bview = BehaviourView::from_outcomes(&view, [&out]);
    assert_eq!(out.registered().count(), 3);
    for reg in out.registered() {
        let r = store.resolve(&bview, reg.behaviour, REV).unwrap();
        assert!(reg.effect.requires_gate());
        assert!(
            r.methods.iter().all(|m| m.effects.is_empty()),
            "{}",
            reg.capability
        );
    }
}

/// A behaviour is found at its own classid only: not under another
/// behaviour, and not at the class's data classid. An unregistered classid is
/// UNKNOWN, never "no methods".
#[test]
fn a_method_is_found_only_under_its_own_class() {
    let (store, outcomes) = census();
    let data = OgarClassView::new();
    let view = BehaviourView::from_outcomes(&data, &outcomes);
    let healthcare = &outcomes[2];
    let by_name = |n: &str| healthcare.registered().find(|r| r.capability == n).unwrap();
    let register = by_name("register_patient");
    let add_dx = by_name("add_diagnosis");
    assert_eq!(register.on, class_ids::PATIENT);
    assert_eq!(add_dx.on, class_ids::DIAGNOSIS);
    let on_patient = store.resolve(&view, register.behaviour, REV).unwrap();
    assert_eq!(on_patient.methods.len(), 1);
    assert_eq!(on_patient.methods[0].name, "register_patient");
    // add_diagnosis's classid is a different one, holding a different method.
    let on_dx = store.resolve(&view, add_dx.behaviour, REV).unwrap();
    assert_eq!(on_dx.methods[0].name, "add_diagnosis");
    // The method is not at the other behaviour's classid.
    assert_eq!(
        store.method_ref(add_dx.behaviour, register.id).unwrap_err(),
        Refusal::UnknownMethod {
            class: add_dx.behaviour,
            method: register.id
        }
    );
    // Nothing is registered at the data classid.
    assert_eq!(
        store.resolve(&view, register.class, REV).unwrap_err(),
        Refusal::UnknownClass(register.class)
    );
}

/// O9, adapted red-first: a behaviour classid does not read its data class's
/// basis by accident. Through the plain data view it is `UnknownLensBasis`;
/// through a behaviour view that did not declare it, the same; declared, it
/// reads exactly its class's fields.
#[test]
fn a_behaviour_id_never_reads_the_data_classs_basis_by_accident() {
    let (store, outcomes) = census();
    let data = OgarClassView::new();
    let reg = outcomes[2].registered().next().unwrap();
    assert!(
        !data.fields(reg.on).is_empty(),
        "the data class has a basis"
    );
    assert_eq!(
        store.resolve(&data, reg.behaviour, REV).unwrap_err(),
        Refusal::UnknownLensBasis(reg.behaviour)
    );
    let undeclared = BehaviourView::new(&data);
    assert_eq!(
        store.resolve(&undeclared, reg.behaviour, REV).unwrap_err(),
        Refusal::UnknownLensBasis(reg.behaviour)
    );
    let declared = BehaviourView::from_outcomes(&data, &outcomes);
    assert_eq!(
        store.resolve(&declared, reg.behaviour, REV).unwrap().fields,
        data.fields(reg.on)
    );
    // Declaring something that is not a behaviour is refused.
    let mut v = BehaviourView::new(&data);
    assert!(!v.declare(reg.class));
    assert!(v.declare(reg.behaviour));
}

/// The registered table is idempotent: the same table again is accepted
/// (DT64's D5); the same def with another guard is a changed behaviour and is
/// refused as `BehaviourImmutable` at its own classid, surfaced as
/// `TableRefusal::Dt64`.
#[test]
fn re_registering_is_idempotent_and_a_changed_guard_is_refused() {
    let view = OgarClassView::new();
    let tables = domain_tables();
    let erp = tables.iter().find(|t| t.domain == "erp").unwrap();
    let mut store = BindStore::new();
    let first = register_domain_table(&mut store, &view, erp, APP, REV).unwrap();
    let again = register_domain_table(&mut store, &view, erp, APP, REV).unwrap();
    assert_eq!(first, again);
    assert_eq!(store.class_count(), 7);

    // A changed guard needs a field to guard on: the textline probe class.
    let view = textline_view();
    fn plain() -> Vec<ActionDef> {
        vec![ActionDef::new(
            "ogit-ocr/textline::action_def::confirm",
            "confirm",
            "ogit-ocr/textline",
        )]
    }
    fn guarded() -> Vec<ActionDef> {
        let mut d = plain().remove(0);
        d.kausal = Some(KausalSpec::StateGuard {
            guard_field: "state".into(),
            guard_values: vec!["1".into()],
        });
        vec![d]
    }
    fn none() -> Vec<(String, u16)> {
        Vec::new()
    }
    let t = |actions: fn() -> Vec<ActionDef>| DomainTable {
        domain: "probe",
        expected_executors: &[],
        entries: none,
        actions,
    };
    let mut store = BindStore::new();
    let out = register_probe(&mut store, &view, &t(plain)).unwrap();
    let behaviour = out.registered().next().unwrap().behaviour;
    assert_eq!(
        register_probe(&mut store, &view, &t(guarded)),
        Err(TableRefusal::Dt64 {
            class: behaviour,
            refusal: Refusal::BehaviourImmutable(behaviour)
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
    assert_eq!(store.class_count(), 20);
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
    let out = register_probe(&mut store, &view, &table).unwrap();
    let reg = out.registered().next().unwrap();
    assert_eq!(out.method_ids(), vec![MethodId(u32::from(reg.arm))]);
    assert_eq!(reg.on, class_ids::TEXTLINE);
    let bview = BehaviourView::from_outcomes(&view, [&out]);
    let r = store.resolve(&bview, reg.behaviour, REV).unwrap();
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

// ── the consumer supplies the left side ─────────────────────────────────────

/// A consumer's view: its own classes, each with the same two-field basis.
struct Consumer {
    classes: std::collections::BTreeMap<u16, Vec<FieldRef>>,
    none: Vec<FieldRef>,
}

impl Consumer {
    fn of(classes: &[u16]) -> Consumer {
        let basis = vec![
            FieldRef::new("id".to_string(), "id".to_string()),
            FieldRef::new("state".to_string(), "state".to_string()),
        ];
        Consumer {
            classes: classes.iter().map(|c| (*c, basis.clone())).collect(),
            none: Vec::new(),
        }
    }
}

impl ClassView for Consumer {
    fn fields(&self, class: ClassId) -> &[FieldRef] {
        self.classes.get(&class).unwrap_or(&self.none)
    }
    fn template(&self, _class: ClassId) -> DisplayTemplate {
        DisplayTemplate::Detail
    }
    fn dolce_category_id(&self, _class: ClassId) -> u8 {
        0
    }
}

const HEALTH_CONCEPTS: [&str; 6] = [
    "patient",
    "diagnosis",
    "lab_value",
    "medication",
    "treatment",
    "vital_sign",
];

fn consumer_map(base: u16) -> ClassMap {
    HEALTH_CONCEPTS
        .iter()
        .enumerate()
        .fold(ClassMap::new(), |m, (i, c)| m.with(c, base + i as u16))
}

fn healthcare() -> DomainTable {
    domain_tables()
        .into_iter()
        .find(|t| t.domain == "healthcare")
        .unwrap()
}

/// Two consumers register the same domain table under their own classes: the
/// class ids (left halves) differ, the F### arms (right halves) are identical.
#[test]
fn two_consumers_get_different_class_ids_and_identical_arms() {
    let table = healthcare();
    let register = |base: u16| {
        let view = Consumer::of(&(0..6).map(|i| base + i).collect::<Vec<_>>());
        let mut store = BindStore::new();
        let out = register_domain_table_for(&mut store, &view, &consumer_map(base), &table, REV)
            .expect("registers");
        (store, view, out)
    };
    let (store_a, view_a, a) = register(0x7F01);
    let (_, _, b) = register(0x7E01);
    assert_eq!(a.registered().count(), 20);
    assert_eq!(b.registered().count(), 20);
    for (ra, rb) in a.registered().zip(b.registered()) {
        assert_eq!(ra.identity, rb.identity);
        assert_eq!(
            ra.arm, rb.arm,
            "{}: same arm for both consumers",
            ra.capability
        );
        assert_ne!(ra.on, rb.on);
        assert_ne!(ra.behaviour, rb.behaviour);
        assert_eq!(ra.on >> 8, 0x7F);
        assert_eq!(rb.on >> 8, 0x7E);
    }
    // Consumer A's behaviours resolve through A's own view, with A's basis.
    let bview = BehaviourView::from_outcomes(&view_a, [&a]);
    let reg = a.registered().next().unwrap();
    assert_eq!(reg.behaviour, 0x7F01_0000 | u32::from(reg.arm));
    let r = store_a.resolve(&bview, reg.behaviour, REV).unwrap();
    assert_eq!(r.fields.len(), 2);
}

/// A def whose class the consumer's mapping lacks is refused by name; it is
/// never read as the canonical concept, and the rest still register.
#[test]
fn a_def_whose_class_the_consumer_lacks_is_refused_by_name() {
    let table = healthcare();
    let view = Consumer::of(&[0x7F01]);
    let only_patient = ClassMap::new().with("patient", 0x7F01);
    let mut store = BindStore::new();
    let out = register_domain_table_for(&mut store, &view, &only_patient, &table, REV).unwrap();
    assert_eq!(out.registered().count(), 4);
    let refused: Vec<_> = out.refused().collect();
    assert_eq!(refused.len(), 16);
    for (row, why) in &refused {
        assert!(
            matches!(why, ConvertRefusal::ConsumerClassMissing { concept } if concept != "patient"),
            "{}: {why:?}",
            row.predicate
        );
    }
    assert!(refused.iter().any(|(_, w)| **w
        == ConvertRefusal::ConsumerClassMissing {
            concept: "diagnosis".into()
        }));
    assert_eq!(store.class_count(), 4);
    // The canonical consumer, by contrast, does have `diagnosis`.
    let mut canon = BindStore::new();
    let ok = register_domain_table_for(
        &mut canon,
        &OgarClassView::new(),
        &CanonicalClasses,
        &table,
        REV,
    )
    .unwrap();
    assert_eq!(ok.registered().count(), 20);
}

/// The migration leaves the reading-only concepts without capabilities: no
/// method lands on `commercial_document` (0x0202) or `billable_work_entry`
/// (0x0103) (the ERP defs sit on the `erp_polyfill` seat, 0x020C), and the
/// hot-plug join still answers `NoCapabilitiesFor` for them.
#[test]
fn the_reading_only_concepts_still_answer_no_capabilities() {
    let (_, outcomes) = census();
    let lefts: std::collections::BTreeSet<u16> = outcomes
        .iter()
        .flat_map(|o| o.registered())
        .map(|r| r.on)
        .collect();
    assert!(lefts.contains(&class_ids::ERP_POLYFILL));
    assert!(!lefts.contains(&class_ids::COMMERCIAL_DOCUMENT));
    assert!(!lefts.contains(&class_ids::BILLABLE_WORK_ENTRY));
    assert_eq!(
        resolve_hotplug("erp-ledger", &[class_ids::COMMERCIAL_DOCUMENT], &[]),
        Err(HotplugDrift::NoCapabilitiesFor(
            class_ids::COMMERCIAL_DOCUMENT
        ))
    );
    assert_eq!(
        resolve_hotplug("erp-ledger", &[class_ids::BILLABLE_WORK_ENTRY], &[]),
        Err(HotplugDrift::NoCapabilitiesFor(
            class_ids::BILLABLE_WORK_ENTRY
        ))
    );
    // Neither is a registered classid: no behaviour sits on either.
    let erp = outcomes.iter().find(|o| o.domain == "erp").unwrap();
    assert!(erp.registered().all(|r| r.on == class_ids::ERP_POLYFILL));
}

fn textline_def(name: &str) -> ActionDef {
    ActionDef::new(
        format!("ogit-ocr/textline::action_def::{name}"),
        name,
        "ogit-ocr/textline",
    )
}

fn textline_table(actions: fn() -> Vec<ActionDef>) -> DomainTable {
    fn none() -> Vec<(String, u16)> {
        Vec::new()
    }
    DomainTable {
        domain: "probe",
        expected_executors: &[],
        entries: none,
        actions,
    }
}

/// A def whose identity is in neither ledger is refused by name and the table
/// is not registered: there is no fallback number. The canonical entry points
/// refuse it too.
#[test]
fn an_identity_with_no_ledger_entry_is_refused_by_name_and_never_gets_a_number() {
    fn unknown() -> Vec<ActionDef> {
        // `x` is first and OGAR knows it; the unknown one is behind it.
        vec![
            ActionDef::new(
                "ogit-ocr/textline::action_def::recognize_line",
                "recognize_line",
                "ogit-ocr/textline",
            ),
            textline_def("not_in_any_ledger"),
        ]
    }
    let view = textline_view();
    let table = textline_table(unknown);
    let want = Err(TableRefusal::Arm(BehaviourArmRefusal::NoLedgerEntry {
        identity: "ogit-ocr/textline::action_def::not_in_any_ledger".into(),
    }));
    let mut store = BindStore::new();
    assert_eq!(
        register_domain_table(&mut store, &view, &table, APP, REV),
        want
    );
    assert_eq!(
        register_domain_table_for(&mut store, &view, &CanonicalClasses, &table, REV),
        want
    );
    // A consumer ledger that does not cover it does not help either.
    let other = ConsumerArms::new().with("some::other::identity", 0xF800);
    assert_eq!(
        register_domain_table_with_arms(&mut store, &view, &CanonicalClasses, &other, &table, REV),
        want
    );
    assert_eq!(store.class_count(), 0, "nothing was registered");
    // The consumer's entry for it registers it, at that arm and no other.
    let own = ConsumerArms::new().with("ogit-ocr/textline::action_def::not_in_any_ledger", 0xF900);
    let out =
        register_domain_table_with_arms(&mut store, &view, &CanonicalClasses, &own, &table, REV)
            .unwrap();
    let arms: Vec<u16> = out.registered().map(|r| r.arm).collect();
    assert_eq!(arms, vec![0xF001, 0xF900]);
}

/// A table that lists one identity twice is refused; nothing is registered.
#[test]
fn a_def_listed_twice_is_refused_by_name() {
    fn same_twice() -> Vec<ActionDef> {
        vec![textline_def("x"), textline_def("x")]
    }
    let view = textline_view();
    let mut store = BindStore::new();
    assert_eq!(
        register_probe(&mut store, &view, &textline_table(same_twice)),
        Err(TableRefusal::Arm(BehaviourArmRefusal::DuplicateDef {
            on: class_ids::TEXTLINE,
            identity: "ogit-ocr/textline::action_def::x".into()
        }))
    );
    assert_eq!(store.class_count(), 0);
}

/// The consumer's ledger entries are checked before DT64 is called: reusing a
/// canonical arm, rebinding a canonical identity, taking an arm in OGAR's range
/// and an arm outside the behaviour range are each refused by name, and nothing
/// is registered. Consumer arms live in `0xF800..=0xFFFF`.
#[test]
fn a_consumer_ledger_that_crosses_ogars_arms_is_refused_by_name() {
    fn one() -> Vec<ActionDef> {
        vec![textline_def("probe_one")]
    }
    let id = "ogit-ocr/textline::action_def::probe_one";
    let (canon_id, canon_arm) = CANONICAL[0];
    let view = textline_view();
    let table = textline_table(one);
    let try_arms = |arms: ConsumerArms| {
        let mut store = BindStore::new();
        let r = register_domain_table_with_arms(
            &mut store,
            &view,
            &CanonicalClasses,
            &arms,
            &table,
            REV,
        );
        assert_eq!(store.class_count(), 0);
        r
    };
    let refused = |l: LedgerRefusal| Err(TableRefusal::Arm(BehaviourArmRefusal::Ledger(l)));

    assert_eq!(
        try_arms(ConsumerArms::new().with(id, canon_arm)),
        refused(LedgerRefusal::ReusesCanonicalArm {
            identity: id.into(),
            arm: canon_arm,
            canonical_identity: canon_id.into()
        })
    );
    assert_eq!(
        try_arms(ConsumerArms::new().with(id, 0xF800).with(canon_id, 0xF801)),
        refused(LedgerRefusal::RebindsCanonicalIdentity {
            identity: canon_id.into(),
            canonical: canon_arm,
            asked: 0xF801
        })
    );
    assert_eq!(
        try_arms(ConsumerArms::new().with(id, 0xF7FF)),
        refused(LedgerRefusal::ReservedForOgar {
            identity: id.into(),
            arm: 0xF7FF
        })
    );
    assert_eq!(
        try_arms(ConsumerArms::new().with(id, 0xF000)),
        refused(LedgerRefusal::ArmOutOfRange {
            identity: id.into(),
            arm: 0xF000
        })
    );
    assert_eq!(
        try_arms(ConsumerArms::new().with(id, 0xF800).with("other", 0xF800)),
        refused(LedgerRefusal::DuplicateArm {
            arm: 0xF800,
            first: id.into(),
            second: "other".into()
        })
    );
    // The edges of the consumer range are accepted.
    for arm in [0xF800, 0xFFFF] {
        let r = try_arms_ok(&view, &table, ConsumerArms::new().with(id, arm));
        assert_eq!(r.registered().next().unwrap().arm, arm);
    }
}

fn try_arms_ok<V: ClassView + ?Sized>(
    view: &V,
    table: &DomainTable,
    arms: ConsumerArms,
) -> TableOutcome {
    let mut store = BindStore::new();
    register_domain_table_with_arms(&mut store, view, &CanonicalClasses, &arms, table, REV)
        .expect("registers")
}

/// The activated entry point takes the consumer's ledger too: the activation
/// runs first (a wrong consumer never reaches the ledger), then the ledger is
/// checked, then DT64 is called.
#[test]
fn the_activated_entry_point_takes_the_consumer_ledger() {
    let view = OgarClassView::new();
    let table = healthcare();
    let covered = ogar_vocab::healthcare_actions::HEALTHCARE_ACTION_NAMES;
    let bad = ConsumerArms::new().with("x", CANONICAL[0].1);
    let good = ConsumerArms::new().with("x", 0xF800);
    let run = |store: &mut BindStore, arms: &ConsumerArms, consumer: &str| {
        register_domain_table_activated_with_arms(
            store,
            &view,
            &CanonicalClasses,
            arms,
            &table,
            consumer,
            covered,
            REV,
        )
    };
    let mut store = BindStore::new();
    assert!(matches!(
        run(&mut store, &bad, "tesseract-ogar"),
        Err(TableRefusal::NotActivated(_))
    ));
    assert!(matches!(
        run(&mut store, &bad, "medcare-rs"),
        Err(TableRefusal::Arm(BehaviourArmRefusal::Ledger(
            LedgerRefusal::ReusesCanonicalArm { .. }
        )))
    ));
    assert_eq!(store.class_count(), 0);
    let ok = run(&mut store, &good, "medcare-rs").expect("green");
    assert_eq!(ok.registered().count(), 20);
}

// ── a generic concept shared by two consumers ───────────────────────────────

/// A name-only def on the shared ticket concept that reads a real field.
fn ticket_def(name: &str) -> ActionDef {
    let mut d = ActionDef::new(
        format!("ogit-crm/crm_ticket::action_def::{name}"),
        name,
        "ogit-crm/crm_ticket",
    );
    d.kausal = Some(KausalSpec::External);
    d.reads = vec!["state".into()];
    d
}

fn ticket_close() -> Vec<ActionDef> {
    vec![ticket_def("close_ticket")]
}

/// The same identity as `ticket_close`, other behaviour: a state guard.
fn ticket_close_guarded() -> Vec<ActionDef> {
    let mut d = ticket_def("close_ticket");
    d.kausal = Some(KausalSpec::StateGuard {
        guard_field: "state".into(),
        guard_values: vec!["1".into()],
    });
    vec![d]
}

fn ticket_close_and_reopen() -> Vec<ActionDef> {
    vec![ticket_def("close_ticket"), ticket_def("reopen_ticket")]
}

/// A shared behaviour plus one that lives on the consumer's own seat.
fn ticket_close_and_seat() -> Vec<ActionDef> {
    let mut seat = ActionDef::new(
        "ogit-zammad/zammad_seat::action_def::escalate_to_group",
        "escalate_to_group",
        "ogit-zammad/zammad_seat",
    );
    seat.kausal = Some(KausalSpec::External);
    vec![ticket_def("close_ticket"), seat]
}

fn ticket_table(actions: fn() -> Vec<ActionDef>) -> DomainTable {
    fn none() -> Vec<(String, u16)> {
        Vec::new()
    }
    DomainTable {
        domain: "ticket-probe",
        expected_executors: &[],
        entries: none,
        actions,
    }
}

const SEAT: u16 = 0x7F01;

/// The consumer's own ledger for the ticket behaviours (OGAR does not know
/// them): its range, `0xF800` upward.
fn ticket_arms() -> ConsumerArms {
    ConsumerArms::new()
        .with("ogit-crm/crm_ticket::action_def::close_ticket", 0xF800)
        .with("ogit-crm/crm_ticket::action_def::reopen_ticket", 0xF801)
        .with(
            "ogit-zammad/zammad_seat::action_def::escalate_to_group",
            0xF802,
        )
}

fn register_ticket<V: ClassView + ?Sized>(
    store: &mut BindStore,
    view: &V,
    classes: &ClassMap,
    table: &DomainTable,
    rev: SchemaRev,
) -> Result<TableOutcome, TableRefusal> {
    register_domain_table_with_arms(store, view, classes, &ticket_arms(), table, rev)
}

/// Both consumers read the ticket concept as the shared canonical class.
fn shared_ticket() -> (Consumer, ClassMap) {
    (
        Consumer::of(&[class_ids::CRM_TICKET, SEAT]),
        ClassMap::new()
            .with("crm_ticket", class_ids::CRM_TICKET)
            .with("zammad_seat", SEAT),
    )
}

/// Case 1: Two consumers sharing the ticket concept register the same def into one
/// store: one behaviour class id for both, the second registration is
/// idempotent.
#[test]
fn two_consumers_sharing_a_concept_register_one_behaviour() {
    let (view_a, map_a) = shared_ticket();
    let (view_b, map_b) = shared_ticket();
    let mut store = BindStore::new();
    let a = register_ticket(
        &mut store,
        &view_a,
        &map_a,
        &ticket_table(ticket_close),
        REV,
    )
    .unwrap();
    let b = register_ticket(
        &mut store,
        &view_b,
        &map_b,
        &ticket_table(ticket_close),
        REV,
    )
    .unwrap();
    let (ra, rb) = (
        a.registered().next().unwrap(),
        b.registered().next().unwrap(),
    );
    assert_eq!(ra.on, class_ids::CRM_TICKET);
    assert_eq!(
        ra.behaviour, rb.behaviour,
        "one class id for both consumers"
    );
    assert_eq!(
        ra.behaviour,
        u32::from(class_ids::CRM_TICKET) << 16 | u32::from(ra.arm)
    );
    assert_eq!(store.class_count(), 1);
    let bview = BehaviourView::from_outcomes(&view_a, [&a]);
    let r = store.resolve(&bview, ra.behaviour, REV).unwrap();
    assert_eq!(r.methods.len(), 1, "one method, not two");
    // The canonical entry point is the same choice and agrees.
    let mut via_canon = BindStore::new();
    let c = register_domain_table_with_arms(
        &mut via_canon,
        &view_a,
        &CanonicalClasses,
        &ticket_arms(),
        &ticket_table(ticket_close),
        REV,
    )
    .unwrap();
    assert_eq!(c.registered().next().unwrap().behaviour, ra.behaviour);
}

/// Case 2: The same identity with another guard from the second consumer is a
/// changed behaviour: refused `BehaviourImmutable` at that class id, the first
/// registration untouched.
#[test]
fn a_second_consumer_cannot_change_a_shared_behaviour() {
    let (view_a, map_a) = shared_ticket();
    let (view_b, map_b) = shared_ticket();
    let mut store = BindStore::new();
    let a = register_ticket(
        &mut store,
        &view_a,
        &map_a,
        &ticket_table(ticket_close),
        REV,
    )
    .unwrap();
    let ra = a.registered().next().unwrap();
    let refused = register_ticket(
        &mut store,
        &view_b,
        &map_b,
        &ticket_table(ticket_close_guarded),
        REV,
    );
    assert_eq!(
        refused,
        Err(TableRefusal::Dt64 {
            class: ra.behaviour,
            refusal: Refusal::BehaviourImmutable(ra.behaviour)
        })
    );
    let bview = BehaviourView::from_outcomes(&view_a, [&a]);
    let r = store.resolve(&bview, ra.behaviour, REV).unwrap();
    assert_eq!(r.methods.len(), 1);
    assert!(
        r.methods[0].when.is_none(),
        "the first binding is untouched"
    );
    let mref = store.method_ref(ra.behaviour, ra.id).unwrap();
    assert!(store.admit(&mref).is_ok());
}

/// Case 3: The second consumer adds a def with a new identity on the shared class:
/// it gets its own arm, the first consumer's id is unchanged, both resolve.
#[test]
fn a_second_consumer_adds_a_behaviour_beside_the_shared_one() {
    let (view_a, map_a) = shared_ticket();
    let (view_b, map_b) = shared_ticket();
    let mut store = BindStore::new();
    let a = register_ticket(
        &mut store,
        &view_a,
        &map_a,
        &ticket_table(ticket_close),
        REV,
    )
    .unwrap();
    let b = register_ticket(
        &mut store,
        &view_b,
        &map_b,
        &ticket_table(ticket_close_and_reopen),
        REV,
    )
    .unwrap();
    let close_a = a.registered().next().unwrap();
    let close_b = b
        .registered()
        .find(|r| r.capability == "close_ticket")
        .unwrap();
    let reopen = b
        .registered()
        .find(|r| r.capability == "reopen_ticket")
        .unwrap();
    assert_eq!(
        close_a.behaviour, close_b.behaviour,
        "the first id did not move"
    );
    assert_ne!(reopen.arm, close_a.arm, "the new def has its own arm");
    assert_eq!(reopen.on, class_ids::CRM_TICKET);
    assert_eq!(store.class_count(), 2);
    let bview = BehaviourView::from_outcomes(&view_b, [&a, &b]);
    for reg in [close_a, reopen] {
        let r = store.resolve(&bview, reg.behaviour, REV).unwrap();
        assert_eq!(r.methods[0].name, reg.capability);
    }
}

/// Case 4: A consumer-specific behaviour on its own seat does not appear under the
/// shared class, while the common one does.
#[test]
fn a_seat_behaviour_does_not_appear_under_the_shared_class() {
    let (view, map) = shared_ticket();
    let mut store = BindStore::new();
    let out = register_ticket(
        &mut store,
        &view,
        &map,
        &ticket_table(ticket_close_and_seat),
        REV,
    )
    .unwrap();
    let shared = out
        .registered()
        .find(|r| r.capability == "close_ticket")
        .unwrap();
    let seat = out
        .registered()
        .find(|r| r.capability == "escalate_to_group")
        .unwrap();
    assert_eq!(shared.on, class_ids::CRM_TICKET);
    assert_eq!(seat.on, SEAT);
    assert_eq!(seat.arm, 0xF802);
    // Nothing of the seat is registered under the shared concept.
    let bview = BehaviourView::from_outcomes(&view, [&out]);
    let under_shared = (u32::from(class_ids::CRM_TICKET) << 16) | u32::from(seat.arm);
    assert_eq!(
        store.resolve(&bview, under_shared, REV).unwrap_err(),
        Refusal::UnknownClass(under_shared)
    );
    assert!(
        out.registered()
            .filter(|r| r.on == class_ids::CRM_TICKET)
            .all(|r| r.capability != "escalate_to_group")
    );
    // The seat behaviour resolves under the seat; the shared one under the
    // shared class.
    assert_eq!(
        store.resolve(&bview, seat.behaviour, REV).unwrap().methods[0].name,
        "escalate_to_group"
    );
    assert_eq!(
        store
            .resolve(&bview, shared.behaviour, REV)
            .unwrap()
            .methods[0]
            .name,
        "close_ticket"
    );
}
