//! SPEC-5 Part C — WoA parity probe: synthetic `ModelGraph` -> lift ->
//! `emit_python` -> structural 1:1 diff against the real WoA
//! `TimesheetActivity` class.
//!
//! ruff has **no SQLAlchemy frontend** yet (RECON R4 / SPEC-5 Part A is the
//! not-yet-built follow-up), so this probe hand-builds the `ModelGraph` a
//! future `ruff_sqlalchemy_spo` would produce, transcribed faithfully from
//! the real, read-only WoA source (never written to). The transcription
//! itself stands in for the ruff frontend and is the auditable artifact —
//! hence the verbatim source quote below.
//!
//! # Source (READ-ONLY, quoted verbatim for audit — `/home/user/WoA/models.py:1746-1753`)
//! ```python
//! class TimesheetActivity(db.Model):
//!     __tablename__ = 'timesheet_activities'
//!     id            = db.Column(db.Integer, primary_key=True)
//!     timesheet_id  = db.Column(db.Integer, db.ForeignKey('timesheets.id', ondelete='CASCADE'), nullable=False)
//!     beschreibung  = db.Column(db.String(500), nullable=False)
//!     created_at    = db.Column(db.DateTime, default=datetime.now)
//!
//!     timesheet = db.relationship('TimeSheet', backref=db.backref('activities', lazy='select', order_by='TimesheetActivity.created_at'))
//! ```
//!
//! Verified in-repo against `/home/user/WoA/models.py` on 2026-07-06 (WoA
//! stayed read-only; only read for this transcription check).
//!
//! # Named SPEC-5 deviation — classid (see `src/mint.rs`'s
//! `compile_graph_sqlalchemy` doc for the full writeup)
//!
//! SPEC-5 expected `TimesheetActivity` to be *unaliased* in `WoaPort` and
//! resolve to a bootstrap classid `0x0000_0003`. That premise is false:
//! `ogar-vocab/src/ports.rs`'s `WOA_ALIASES` (and `RECON.md` R8, written
//! *before* this spec) already pins `"TimesheetActivity"` to
//! `class_ids::BILLABLE_WORK_ENTRY` (`0x0103`) as the deliberate
//! planner/ERP convergence concept. The REAL emitted classid is therefore
//! `0x0103_0003`, not `0x0000_0003` — reflected in the `expected_py` fixture
//! below and called out explicitly in the printed DRIFT list. This is a
//! verified, reported deviation, not an invented workaround.

use ogar_from_ruff::emit::{emit_python, emit_python_prelude};
use ogar_from_ruff::mint::compile_graph_sqlalchemy;
use ogar_vocab::ports::WoaPort;
use ruff_spo_triplet::{AssocDecl, AssocKind, Field, Model, ModelGraph};

/// Build the synthetic `ModelGraph` transcribed from the source quoted
/// above. Namespace `"woa"` (routes `WoaPort` / the `german-erp` domain).
fn timesheet_activity_graph() -> ModelGraph {
    let mut m = Model::new("TimesheetActivity");

    // `timesheet = db.relationship('TimeSheet', backref=db.backref('activities', ...))`
    // — backref `'activities'` is plural, so per SPEC-5 Part A's heuristic
    // this side (`TimesheetActivity.timesheet`) is `BelongsTo` `TimeSheet`.
    m.associations.push(AssocDecl {
        kind: AssocKind::BelongsTo,
        name: "timesheet".to_string(),
        options: vec![("class_name".to_string(), "TimeSheet".to_string())],
    });

    // `id = db.Column(db.Integer, primary_key=True)` — PKs are not-null.
    m.fields.push(Field {
        name: "id".to_string(),
        field_type: Some("integer".to_string()),
        not_null: Some(true),
        ..Default::default()
    });
    // `timesheet_id = db.Column(db.Integer, db.ForeignKey(...), nullable=False)`
    // — physical FK column, shadowed by the `timesheet` association above.
    m.fields.push(Field {
        name: "timesheet_id".to_string(),
        field_type: Some("integer".to_string()),
        not_null: Some(true),
        ..Default::default()
    });
    // `beschreibung = db.Column(db.String(500), nullable=False)`.
    m.fields.push(Field {
        name: "beschreibung".to_string(),
        field_type: Some("string".to_string()),
        not_null: Some(true),
        ..Default::default()
    });
    // `created_at = db.Column(db.DateTime, default=datetime.now)` — no
    // `nullable=False`, so per the SQLAlchemy-DSL mapping table this is
    // `not_null: None` (nullable, schema-total).
    m.fields.push(Field {
        name: "created_at".to_string(),
        field_type: Some("datetime".to_string()),
        not_null: None,
        ..Default::default()
    });

    let mut g = ModelGraph::new("woa");
    g.models.push(m);
    g
}

/// The hand-derived expected `@dataclass` body, derived from the source
/// above with SPEC-0's FK-dedup canon applied (`timesheet_id` deduped
/// against the `timesheet` association). The classid reflects the REAL
/// `WoaPort` alias resolution (`0x0103_0003`, see the module doc), not
/// SPEC-5's assumed bootstrap value.
const EXPECTED_CLASSID_LINE: &str = "    CLASSID: ClassVar[int] = 0x01030003";
const EXPECTED_LINES: &[&str] = &[
    "class TimesheetActivity:",
    EXPECTED_CLASSID_LINE,
    "    id: OgInt",
    "    beschreibung: OgStr",
    "    created_at: Optional[OgDateTime]",
    "    timesheet: ToOne[\"TimeSheet\"]",
];

#[test]
fn woa_timesheet_activity_lifts_and_emits_structurally_1_to_1() {
    let graph = timesheet_activity_graph();
    let compiled = compile_graph_sqlalchemy::<WoaPort>(&graph);
    assert_eq!(compiled.len(), 1, "exactly one class compiled");
    let py = emit_python(&compiled[0]);

    for line in EXPECTED_LINES {
        assert!(
            py.contains(line),
            "expected line {line:?} missing from emitted Python:\n{py}"
        );
    }
    assert!(
        !py.contains("timesheet_id"),
        "FK column timesheet_id must be deduped (shadowed by the `timesheet` association):\n{py}"
    );
}

#[test]
fn woa_parity_probe_prints_the_d_parity_probe_wp_1_metric() {
    let graph = timesheet_activity_graph();
    let compiled = compile_graph_sqlalchemy::<WoaPort>(&graph);
    assert_eq!(compiled.len(), 1);
    let cc = &compiled[0];
    let py = emit_python(cc);

    // ── classes axis ──
    let classes_axis = (compiled.len(), 1usize);

    // ── columns typed / nullability axes (computed from the live Class,
    // not hardcoded, so the metric can't silently drift from the lift) ──
    let expected_scalars = ["id", "beschreibung", "created_at"];
    let typed = expected_scalars
        .iter()
        .filter(|n| {
            cc.class
                .attributes
                .iter()
                .any(|a| &a.name == *n && a.type_name.is_some())
        })
        .count();
    let nullability_wired = expected_scalars
        .iter()
        .filter(|n| {
            cc.class
                .attributes
                .iter()
                .any(|a| &a.name == *n && a.options.required.is_some())
        })
        .count();
    assert_eq!(
        cc.class.attributes.len(),
        3,
        "no extra/missing attributes (FK deduped, no dup)"
    );

    // ── associations axis ──
    let timesheet_assoc = cc.class.associations.iter().find(|a| a.name == "timesheet");
    let fk_deduped = !cc.class.attributes.iter().any(|a| a.name == "timesheet_id");
    let associations_axis = (usize::from(timesheet_assoc.is_some() && fk_deduped), 1usize);

    // ── action names axis (schema-only v0: WoA methods aren't harvested
    // yet — `model.functions` is empty in the synthetic graph, honestly
    // reflecting Part A not being built) ──
    let actions_axis = (0usize, 0usize);

    // ── drift: the classid deviation (named above), otherwise clean type mapping ──
    let classid_drift = format!(
        "classid 0x{:08X} is the WOA_ALIASES convergence pin (BILLABLE_WORK_ENTRY) — \
         SPEC-5 assumed an unaliased bootstrap 0x00000003; TimesheetActivity IS aliased \
         in ogar-vocab/src/ports.rs (see src/mint.rs doc)",
        cc.facet.facet_classid()
    );
    let emit_nullability_drift =
        "emit_python Optional[]-Nullability: GESCHLOSSEN (Batch-1 Item 1) — \
created_at wird als Optional[OgDateTime] emittiert; options.required==Some(false) → Optional[…], \
sonst bare (Spiegel emit_rust)"
            .to_string();
    let drift_list = [classid_drift, emit_nullability_drift];

    let metric = format!(
        "WoA parity probe — TimesheetActivity\n  \
         classes:       {}/{}\n  \
         columns typed: {}/3   (id:integer, beschreibung:string, created_at:datetime)\n  \
         nullability:   {}/3   (id:req, beschreibung:req, created_at:opt)\n  \
         associations:  {}/{}   (timesheet -> ToOne<TimeSheet>, FK timesheet_id deduped)\n  \
         action names:  {}/{}   (methods harvested — 0 in schema-only v0)\n  \
         DRIFT:         {}",
        classes_axis.0,
        classes_axis.1,
        typed,
        nullability_wired,
        associations_axis.0,
        associations_axis.1,
        actions_axis.0,
        actions_axis.1,
        if drift_list.is_empty() {
            "none".to_string()
        } else {
            drift_list.join("; ")
        },
    );
    println!("{metric}");
    println!("--- emitted Python ---\n{py}");

    // Structural axes must be N/N — a parity failure here is the finding
    // the mission cares about (per SPEC-5's guardrail: don't massage the
    // fixture, report it). All hold at N/N for this fixture.
    assert_eq!(classes_axis.0, classes_axis.1, "classes axis must be N/N");
    assert_eq!(typed, 3, "columns typed axis must be 3/3");
    assert_eq!(nullability_wired, 3, "nullability axis must be 3/3");
    assert_eq!(
        associations_axis.0, associations_axis.1,
        "associations axis must be N/N"
    );
    assert_eq!(
        actions_axis.0, actions_axis.1,
        "action names axis must be N/N"
    );

    assert!(metric.contains("classes:       1/1"));
    assert!(metric.contains("columns typed: 3/3"));
    assert!(metric.contains("nullability:   3/3"));
    assert!(metric.contains("associations:  1/1"));
    assert!(metric.contains("action names:  0/0"));
    assert!(metric.contains("DRIFT:"));
}

// ───────────────────────── Part D — py_compile + instantiate gate ─────────────────────────
//
// The emitted `@dataclass` text is the Python mirror of `CompiledClass`
// (Operator §6 / E-PYTHON-SUBSTRATE-MIRROR). Prove it is real, importable
// Python: py_compile it, then instantiate it with dummy values.

#[test]
fn woa_dataclass_py_compiles_and_instantiates() {
    let graph = timesheet_activity_graph();
    let compiled = compile_graph_sqlalchemy::<WoaPort>(&graph);
    // The emitted module pulls its wrapper-contract types via a single
    // `from ogar_runtime import (...)` block (design option (a) — see
    // `emit.rs`'s module doc); it is NOT inlined into every emitted file.
    let module_src = format!("{}\n{}", emit_python_prelude(), emit_python(&compiled[0]));

    let tmp_dir = std::env::var("CARGO_TARGET_TMPDIR")
        .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().to_string());
    std::fs::create_dir_all(&tmp_dir).expect("tmp dir");
    let py_path = std::path::Path::new(&tmp_dir).join("woa_timesheet_activity.py");
    std::fs::write(&py_path, &module_src).expect("write emitted module");

    // Copy the shipped reference `ogar_runtime.py` next to the emitted
    // module (same dir on `sys.path`) so `from ogar_runtime import …`
    // resolves — the consumer pulls the contract, it is not re-minted.
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("python/ogar_runtime.py"),
        std::path::Path::new(&tmp_dir).join("ogar_runtime.py"),
    )
    .expect("copy ogar_runtime.py next to the emitted module");

    // Gate 1: valid Python (py_compile, exit 0).
    let compile_status = std::process::Command::new("python3")
        .args(["-m", "py_compile", py_path.to_str().unwrap()])
        .status()
        .expect("python3 available");
    assert!(
        compile_status.success(),
        "py_compile must exit 0 on the emitted module"
    );

    // Gate 2: usable dataclass (instantiate_check.py imports + instantiates it).
    let checker =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/py/instantiate_check.py");
    let check_status = std::process::Command::new("python3")
        .arg(&checker)
        .arg(&py_path)
        .status()
        .expect("python3 available");
    assert!(check_status.success(), "instantiate_check.py must exit 0");
}
