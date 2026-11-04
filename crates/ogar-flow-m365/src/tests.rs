//! Falsifiers F1–F10 for the command adjacency slice. All data is synthetic.

use std::cell::RefCell;
use std::collections::BTreeMap;

use lance_graph_contract::class_view::FieldMask;
use lance_graph_contract::sla::TenantId;
use ogar_loco::vocabulary::conformance;
use ogar_loco::{Call, DOMAIN_FLOOR, FnIndex, FunctionBody, Program, RunError};

use crate::adjacency::{self, ConceptBinding, Effect, LABELS, OPERATIONS, ResolveError};
use crate::fixture::{
    self, FixtureAuthority, FixtureDirectory, OTHER_TENANT, OUTSIDER, READER, TEST_TENANT,
};
use crate::host::{DirectoryObservations, FieldValue, SourceStamp, UserObservation};
use crate::report::{Report, ReportRow};
use crate::vocab::{self, FLOW_SHAPE, FlowVocabulary, ReportField};
use crate::{CompileError, FlowError, compile, run_flow};

const JOHN_DOE_101: &str = r#"Get-MgUser -UserId "john.doe@example.test" |
    Get-IamLicenseEvidence |
    Select-Object DisplayName, Department, Licenses |
    Export-Report -Format JSON"#;

fn reader() -> ogar_auth::federation::VerifiedLogin {
    fixture::login("fixture-signed:reader").expect("fixture reader logs in")
}

fn run(src: &str) -> Result<Report, RunError<FlowError>> {
    let flow = compile(src).expect("pipeline compiles");
    run_flow(
        &reader(),
        &flow,
        &FixtureDirectory,
        &FixtureAuthority::standard(),
    )
}

// ── The executable slice ────────────────────────────────────────────────────

#[test]
fn john_doe_101_runs_through_loco_and_reports_the_three_selected_fields() {
    let report = run(JOHN_DOE_101).expect("reader is scoped");
    assert_eq!(report.requested, "john.doe@example.test");
    assert_eq!(report.tenant, TEST_TENANT);
    assert_eq!(report.policy_version, 1);
    assert_eq!(report.source.source_id, "fixture:john-doe-101");
    assert_eq!(report.source.observed_version, 7);
    assert_eq!(report.rows.len(), 1);
    let row = &report.rows[0].fields;
    assert_eq!(row.len(), 3);
    assert_eq!(
        row.get(&ReportField::DisplayName),
        Some(&FieldValue::Text("John Doe".into()))
    );
    assert_eq!(
        row.get(&ReportField::Department),
        Some(&FieldValue::Text("Operations".into()))
    );
    assert_eq!(
        row.get(&ReportField::Licenses),
        Some(&FieldValue::List(vec![
            "M365 E3".into(),
            "Teams Phone".into()
        ]))
    );
    let ops: Vec<_> = report.trace.iter().map(|t| t.operation).collect();
    assert_eq!(
        ops,
        [
            "query_user",
            "read_license_evidence",
            "select_fields",
            "export_report"
        ]
    );
    assert_eq!(
        report.to_json(),
        concat!(
            r#"{"format":"ogar-flow-m365/report.v1","requested":"john.doe@example.test","#,
            r#""scope":{"tenant":32343,"policy_version":1,"fields":["ObjectId","UserPrincipalName","DisplayName","Department","AccountEnabled","Licenses","Memberships","MailboxAvailable"]},"#,
            r#""source":{"id":"fixture:john-doe-101","observed_version":7},"#,
            r#""rows":[{"DisplayName":"John Doe","Department":"Operations","Licenses":["M365 E3","Teams Phone"]}],"#,
            r#""trace":[{"label":"Get-MgUser","operation":"query_user","fn":"0x90"},"#,
            r#"{"label":"Get-IamLicenseEvidence","operation":"read_license_evidence","fn":"0x91"},"#,
            r#"{"label":"Select-Object","operation":"select_fields","fn":"0x94"},"#,
            r#"{"label":"Export-Report","operation":"export_report","fn":"0x95"}]}"#
        )
    );
}

#[test]
fn the_lowered_program_is_one_quads_body_of_four_domain_calls() {
    let flow = compile(JOHN_DOE_101).unwrap();
    assert_eq!(flow.program.len(), 1);
    let body = flow.program.entry();
    assert_eq!(body.shape(), FLOW_SHAPE);
    let fns: Vec<_> = body.calls().map(|c| c.function).collect();
    assert_eq!(
        fns,
        [
            vocab::GET_USER,
            vocab::GET_LICENSE_EVIDENCE,
            vocab::SELECT,
            vocab::EXPORT
        ]
    );
    assert!(fns.iter().all(|f| f.is_domain_specific()));
    assert_eq!(flow.args, ["john.doe@example.test"]);
}

#[test]
fn memberships_and_filters_compose_in_one_pipeline() {
    let report = run(r#"Get-MgUser john.doe@example.test | Get-IamMembership |
           Where-Object Memberships -contains "servicedesk" |
           Select-Object DisplayName, Memberships | Export-Report"#)
    .unwrap();
    assert_eq!(report.rows.len(), 1);
    assert_eq!(
        report.rows[0].fields.get(&ReportField::Memberships),
        Some(&FieldValue::List(vec![
            "Reporting-Readers".into(),
            "ServiceDesk".into()
        ]))
    );
    // The same filter on a value John does not hold keeps nothing.
    let none = run(r#"Get-MgUser john.doe@example.test | Get-IamMembership |
           Where-Object Memberships -contains "Board" |
           Select-Object DisplayName | Export-Report"#)
    .unwrap();
    assert!(none.rows.is_empty());
    let dept = run(
        r#"Get-MgUser john.doe@example.test | Where-Object Department -ne Operations |
           Select-Object DisplayName | Export-Report"#,
    )
    .unwrap();
    assert!(dept.rows.is_empty());
}

// ── F1: command resolution ──────────────────────────────────────────────────

#[test]
fn f1_known_labels_resolve_to_their_typed_operation() {
    let cases = [
        ("Get-MgUser", "query_user", vocab::GET_USER),
        ("get-mguser", "query_user", vocab::GET_USER),
        (
            "Get-IamLicenseEvidence",
            "read_license_evidence",
            vocab::GET_LICENSE_EVIDENCE,
        ),
        (
            "Get-IamMembership",
            "read_membership",
            vocab::GET_MEMBERSHIP,
        ),
        ("Where-Object", "filter_rows", vocab::WHERE),
        ("Select-Object", "select_fields", vocab::SELECT),
        ("Export-Report", "export_report", vocab::EXPORT),
    ];
    for (label, name, f) in cases {
        let (_, op) = adjacency::resolve(label).unwrap();
        assert_eq!((op.name, op.fn_index), (name, Some(f)), "{label}");
    }
}

#[test]
fn f1_unknown_labels_are_refused() {
    for label in ["Get-MgUsr", "ForEach-Object", "Get-Process", ""] {
        assert_eq!(
            adjacency::resolve(label),
            Err(ResolveError::Unknown(label.to_owned()))
        );
    }
    assert!(matches!(
        compile("Get-MgUserX -UserId a | Export-Report"),
        Err(CompileError::Resolve(ResolveError::Unknown(_)))
    ));
}

// ── F2: alias equivalence ───────────────────────────────────────────────────

#[test]
fn f2_aliases_lower_to_identical_programs_and_identical_reports() {
    let a = compile(
        "Get-MgUser john.doe@example.test | Where-Object Department -eq Operations | Select-Object DisplayName | Export-Report",
    )
    .unwrap();
    let b = compile(
        "Get-AzureADUser john.doe@example.test | ? Department -eq Operations | select DisplayName | Export-Report",
    )
    .unwrap();
    assert_eq!(a.program, b.program);
    assert_eq!(a.args, b.args);
    let ra = run_flow(
        &reader(),
        &a,
        &FixtureDirectory,
        &FixtureAuthority::standard(),
    )
    .unwrap();
    let rb = run_flow(
        &reader(),
        &b,
        &FixtureDirectory,
        &FixtureAuthority::standard(),
    )
    .unwrap();
    assert_eq!(ra.rows, rb.rows);
    // Only the trace's as-written labels differ.
    let ops = |r: &Report| {
        r.trace
            .iter()
            .map(|t| (t.operation, t.fn_index))
            .collect::<Vec<_>>()
    };
    assert_eq!(ops(&ra), ops(&rb));
    assert_ne!(ra.trace[0].label, rb.trace[0].label);
}

#[test]
fn f2_every_alias_group_shares_one_operation_row() {
    let mut by_op: BTreeMap<u8, Vec<&str>> = BTreeMap::new();
    for l in LABELS {
        by_op.entry(l.operation.0).or_default().push(l.text);
    }
    assert_eq!(by_op[&0], ["Get-MgUser", "Get-AzureADUser"]);
    assert_eq!(by_op[&3], ["Where-Object", "where", "?"]);
    // No two labels collide case-insensitively.
    for (i, a) in LABELS.iter().enumerate() {
        for b in &LABELS[i + 1..] {
            assert!(
                !a.text.eq_ignore_ascii_case(b.text),
                "{} / {}",
                a.text,
                b.text
            );
        }
    }
}

// ── F3: canonical vocabulary reuse ──────────────────────────────────────────

#[test]
fn f3_no_classid_is_minted_and_no_label_is_a_concept() {
    for op in OPERATIONS {
        // An operation name that matched a minted concept would be a second
        // spelling of it.
        assert_eq!(
            ogar_vocab::canonical_concept_id(op.name),
            None,
            "{}",
            op.name
        );
        // Bindings name an existing schema, the generic operator, or the
        // missing concept, never a classid of their own.
        assert!(matches!(
            op.binding,
            ConceptBinding::Generic | ConceptBinding::DirSchema(_) | ConceptBinding::Unminted(_)
        ));
    }
    for l in LABELS {
        let normal = l.text.to_ascii_lowercase().replace('-', "_");
        assert_eq!(
            ogar_vocab::canonical_concept_id(&normal),
            None,
            "{}",
            l.text
        );
    }
}

#[test]
fn f3_call_bytes_are_unique_and_comparisons_reuse_the_shared_core_codebook() {
    let bytes: Vec<_> = OPERATIONS.iter().filter_map(|o| o.fn_index).collect();
    let mut sorted = bytes.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), bytes.len());
    assert!(bytes.iter().all(|f| f.0 >= DOMAIN_FLOOR));
    for c in [
        vocab::Comparison::Eq,
        vocab::Comparison::Ne,
        vocab::Comparison::Contains,
    ] {
        assert!(FnIndex(c.byte()).is_shared_core());
        assert!(ogar_loco::vocabulary::shared_core::name(FnIndex(c.byte())).is_some());
    }
}

// ── F4: read-only enforcement ───────────────────────────────────────────────

#[test]
fn f4_mutating_and_executing_labels_are_known_and_refused() {
    for label in [
        "Set-MgUser",
        "Update-MgUser",
        "Remove-MgUser",
        "New-MgGroupMember",
        "Set-MgUserLicense",
        "Invoke-Expression",
        "iex",
    ] {
        let err = adjacency::resolve(label).unwrap_err();
        let ResolveError::Unavailable { effect, .. } = err else {
            panic!("{label}: {err:?}")
        };
        assert!(
            matches!(effect, Effect::Mutation | Effect::Execution),
            "{label}"
        );
    }
    assert!(matches!(
        compile("Get-MgUser john.doe@example.test | Set-MgUser | Export-Report"),
        Err(CompileError::Resolve(ResolveError::Unavailable { .. }))
    ));
}

#[test]
fn f4_no_operation_with_a_write_effect_has_a_call_byte() {
    for op in OPERATIONS {
        if matches!(op.effect, Effect::Mutation | Effect::Execution) {
            assert_eq!(op.fn_index, None, "{}", op.name);
            assert!(!op.is_available());
        }
    }
}

#[test]
fn f4_a_hand_built_body_with_an_uncovered_byte_is_refused_by_the_dialect() {
    let mut flow = compile(JOHN_DOE_101).unwrap();
    for f in [
        FnIndex(0xA0),
        FnIndex::ADD,
        FnIndex::PRINT,
        FnIndex::VAR_SET,
    ] {
        flow.program = Program {
            functions: vec![
                FunctionBody::from_calls(
                    FLOW_SHAPE,
                    &[Call::with_value(vocab::GET_USER, 0), Call::new(f)],
                )
                .unwrap(),
            ],
        };
        let err = run_flow(
            &reader(),
            &flow,
            &FixtureDirectory,
            &FixtureAuthority::standard(),
        );
        assert_eq!(
            err,
            Err(RunError::Dialect(FlowError::UnsupportedCall { call: f })),
            "{f:?}"
        );
    }
}

// ── F5: actor isolation ─────────────────────────────────────────────────────

/// A host that records every query, to prove a denied run never reads.
#[derive(Default)]
struct CountingHost {
    queries: RefCell<Vec<String>>,
}

impl DirectoryObservations for CountingHost {
    fn source(&self) -> SourceStamp {
        FixtureDirectory.source()
    }
    fn find_user(&self, t: TenantId, r: &str, f: FieldMask) -> Option<UserObservation> {
        self.queries
            .borrow_mut()
            .push(format!("user {t} {r} {:#x}", f.0));
        FixtureDirectory.find_user(t, r, f)
    }
    fn licenses(&self, t: TenantId, k: &str) -> Vec<String> {
        self.queries.borrow_mut().push(format!("licenses {t} {k}"));
        FixtureDirectory.licenses(t, k)
    }
    fn memberships(&self, t: TenantId, k: &str) -> Vec<String> {
        self.queries
            .borrow_mut()
            .push(format!("memberships {t} {k}"));
        FixtureDirectory.memberships(t, k)
    }
}

#[test]
fn f5_an_actor_without_scope_gets_no_data_and_the_host_is_never_queried() {
    let outsider = fixture::login("fixture-signed:outsider").unwrap();
    assert_eq!(outsider.authenticated_user().user.id, OUTSIDER);
    let flow = compile(JOHN_DOE_101).unwrap();
    let host = CountingHost::default();
    let out = run_flow(&outsider, &flow, &host, &FixtureAuthority::standard());
    assert_eq!(out, Err(RunError::Dialect(FlowError::NoScope)));
    assert!(host.queries.borrow().is_empty());
}

#[test]
fn f5_a_missing_capability_fails_before_that_operation_reads_anything() {
    let mut scope = fixture::reader_scope();
    scope.capabilities =
        crate::CapabilitySet::of(&[crate::Capability::ReadIdentity, crate::Capability::Export]);
    let authority = FixtureAuthority::default().grant(READER, TEST_TENANT, scope);
    let host = CountingHost::default();
    let flow = compile(JOHN_DOE_101).unwrap();
    let out = run_flow(&reader(), &flow, &host, &authority);
    assert_eq!(
        out,
        Err(RunError::Dialect(FlowError::CapabilityDenied {
            operation: "read_license_evidence",
            capability: crate::Capability::ReadLicenses,
        }))
    );
    let q = host.queries.borrow();
    assert_eq!(q.len(), 1, "{q:?}");
    assert!(q[0].starts_with("user "));
    // No export capability: no report, even though every read was allowed.
    let mut scope = fixture::reader_scope();
    scope.capabilities = crate::CapabilitySet::of(&[
        crate::Capability::ReadIdentity,
        crate::Capability::ReadLicenses,
    ]);
    let authority = FixtureAuthority::default().grant(READER, TEST_TENANT, scope);
    assert!(matches!(
        run_flow(&reader(), &flow, &FixtureDirectory, &authority),
        Err(RunError::Dialect(FlowError::CapabilityDenied {
            operation: "export_report",
            ..
        }))
    ));
}

#[test]
fn f5_another_tenants_user_is_indistinguishable_from_no_user() {
    // The pipeline names a user in OTHER_TENANT; the reader is scoped to
    // TEST_TENANT, so the query runs there and finds nothing.
    let foreign = run(
        "Get-MgUser john.doe@other.test | Get-IamLicenseEvidence | Select-Object DisplayName, Licenses | Export-Report",
    )
    .unwrap();
    let missing = run(
        "Get-MgUser nobody@example.test | Get-IamLicenseEvidence | Select-Object DisplayName, Licenses | Export-Report",
    )
    .unwrap();
    assert!(foreign.rows.is_empty());
    assert_eq!(foreign.tenant, TEST_TENANT);
    let strip = |r: Report| Report {
        requested: String::new(),
        trace: Vec::new(),
        ..r
    };
    assert_eq!(strip(foreign), strip(missing));
    assert!(
        FixtureDirectory
            .find_user(OTHER_TENANT, "john.doe@other.test", FieldMask(!0))
            .is_some()
    );
}

#[test]
fn f5_pipeline_arguments_cannot_choose_the_actor() {
    // Naming another actor as a parameter is refused outright.
    for src in [
        "Get-MgUser john.doe@example.test -RunAs reader | Export-Report",
        "Get-MgUser -Actor reader | Export-Report",
        "Get-MgUser -UserId john.doe@example.test | Get-IamLicenseEvidence -Credential reader | Export-Report",
    ] {
        assert!(
            matches!(
                compile(src),
                Err(CompileError::UnknownParameter { .. } | CompileError::BadArguments { .. })
            ),
            "{src}"
        );
    }
    // Asking about the reader's own subject does not change whose scope runs.
    let flow = compile("Get-MgUser reader | Export-Report").unwrap();
    let outsider = fixture::login("fixture-signed:outsider").unwrap();
    assert_eq!(
        run_flow(
            &outsider,
            &flow,
            &FixtureDirectory,
            &FixtureAuthority::standard()
        ),
        Err(RunError::Dialect(FlowError::NoScope))
    );
}

#[test]
fn f5_an_unverified_assertion_never_becomes_an_actor() {
    assert!(fixture::login("reader").is_err());
    assert!(fixture::login("fixture-signed:nobody").is_err());
}

// ── F6: field projection ────────────────────────────────────────────────────

/// A host that ignores the requested projection and returns every field.
struct OversharingHost;

impl DirectoryObservations for OversharingHost {
    fn source(&self) -> SourceStamp {
        FixtureDirectory.source()
    }
    fn find_user(&self, t: TenantId, r: &str, _f: FieldMask) -> Option<UserObservation> {
        FixtureDirectory.find_user(t, r, FieldMask(!0))
    }
    fn licenses(&self, t: TenantId, k: &str) -> Vec<String> {
        FixtureDirectory.licenses(t, k)
    }
    fn memberships(&self, t: TenantId, k: &str) -> Vec<String> {
        FixtureDirectory.memberships(t, k)
    }
}

#[test]
fn f6_the_unpermitted_field_is_never_requested_from_the_host() {
    let host = CountingHost::default();
    let flow = compile(JOHN_DOE_101).unwrap();
    run_flow(&reader(), &flow, &host, &FixtureAuthority::standard()).unwrap();
    let q = host.queries.borrow();
    let mask = u64::from_str_radix(
        q[0].rsplit(' ').next().unwrap().trim_start_matches("0x"),
        16,
    )
    .unwrap();
    assert_eq!(
        mask & (1 << ReportField::Mail.position()),
        0,
        "Mail was requested"
    );
    assert_ne!(mask & (1 << ReportField::DisplayName.position()), 0);
}

#[test]
fn f6_an_oversharing_host_cannot_leak_the_fourth_field() {
    let flow = compile(
        "Get-MgUser john.doe@example.test | Get-IamLicenseEvidence | Select-Object DisplayName, Department, Licenses | Export-Report",
    )
    .unwrap();
    let r = run_flow(
        &reader(),
        &flow,
        &OversharingHost,
        &FixtureAuthority::standard(),
    )
    .unwrap();
    assert_eq!(r.rows[0].fields.len(), 3);
    let keys: Vec<_> = r.rows[0].fields.keys().copied().collect();
    assert_eq!(
        keys,
        [
            ReportField::DisplayName,
            ReportField::Department,
            ReportField::Licenses
        ]
    );
    let json = r.to_json();
    let rows = &json[json.find("\"rows\"").unwrap()..json.find("\"trace\"").unwrap()];
    assert!(!rows.contains("Mail") && !rows.contains('@'), "{rows}");
    // Selecting the unpermitted field is refused, not silently dropped.
    let flow = compile(
        "Get-MgUser john.doe@example.test | Select-Object DisplayName, Mail | Export-Report",
    )
    .unwrap();
    assert_eq!(
        run_flow(
            &reader(),
            &flow,
            &OversharingHost,
            &FixtureAuthority::standard()
        ),
        Err(RunError::Dialect(FlowError::FieldDenied {
            field: ReportField::Mail
        }))
    );
    // Filtering on it is refused too: a filter would leak it through the row count.
    let flow = compile(
        "Get-MgUser john.doe@example.test | Where-Object Mail -eq john.doe@example.test | Select-Object DisplayName | Export-Report",
    )
    .unwrap();
    assert_eq!(
        run_flow(
            &reader(),
            &flow,
            &OversharingHost,
            &FixtureAuthority::standard()
        ),
        Err(RunError::Dialect(FlowError::FieldDenied {
            field: ReportField::Mail
        }))
    );
}

#[test]
fn f6_selecting_an_unloaded_field_is_refused() {
    assert_eq!(
        run("Get-MgUser john.doe@example.test | Select-Object Licenses | Export-Report"),
        Err(RunError::Dialect(FlowError::FieldNotLoaded {
            field: ReportField::Licenses
        }))
    );
}

// ── F7: determinism ─────────────────────────────────────────────────────────

#[test]
fn f7_identical_inputs_produce_identical_bytes() {
    let a = run(JOHN_DOE_101).unwrap().to_json();
    for _ in 0..8 {
        assert_eq!(run(JOHN_DOE_101).unwrap().to_json(), a);
    }
    let mut changed = fixture::reader_scope();
    changed.policy_version = 2;
    let auth = FixtureAuthority::default().grant(READER, TEST_TENANT, changed);
    let b = run_flow(
        &reader(),
        &compile(JOHN_DOE_101).unwrap(),
        &FixtureDirectory,
        &auth,
    )
    .unwrap()
    .to_json();
    assert_ne!(a, b, "the policy version is part of the result");
}

// ── F8: LOCO parity ─────────────────────────────────────────────────────────

/// The John Doe report computed directly in Rust, with no LOCO program.
fn direct_john_doe_report() -> Report {
    let scope = fixture::reader_scope();
    let host = FixtureDirectory;
    let obs = host
        .find_user(
            scope.tenant,
            "john.doe@example.test",
            ReportField::USER_FIELDS.intersect(scope.fields),
        )
        .unwrap();
    let mut fields = BTreeMap::new();
    for (k, v) in obs.fields {
        if matches!(k, ReportField::DisplayName | ReportField::Department) {
            fields.insert(k, v);
        }
    }
    let mut lic = host.licenses(scope.tenant, &obs.object_id);
    lic.sort();
    fields.insert(ReportField::Licenses, FieldValue::List(lic));
    let flow = compile(JOHN_DOE_101).unwrap();
    Report {
        requested: "john.doe@example.test".into(),
        tenant: scope.tenant,
        policy_version: scope.policy_version,
        permitted_fields: ReportField::ALL
            .into_iter()
            .filter(|f| scope.fields.has(f.position()))
            .collect(),
        source: host.source(),
        rows: vec![ReportRow { fields }],
        trace: flow.trace,
    }
}

#[test]
fn f8_the_loco_run_equals_the_direct_typed_reference() {
    assert_eq!(run(JOHN_DOE_101).unwrap(), direct_john_doe_report());
}

// ── F9: vocabulary conformance ──────────────────────────────────────────────

#[test]
fn f9_the_flow_vocabulary_passes_loco_conformance() {
    assert_eq!(conformance::check(&FlowVocabulary), Ok(()));
    let checked = conformance::validate(FlowVocabulary).unwrap();
    let t = checked.table();
    for op in OPERATIONS.iter().filter(|o| o.is_available()) {
        let f = op.fn_index.unwrap();
        assert!(t.stack_arity(f).is_some(), "{}", op.name);
        assert_eq!(t.body_refs(f), 0);
        assert!(t.name(f).is_some());
    }
    assert!(
        compile(JOHN_DOE_101)
            .unwrap()
            .program
            .references_are_resolvable(&checked)
    );
}

#[test]
fn f9_every_domain_byte_outside_the_six_is_refused_by_the_table() {
    let t = *vocab::checked().table();
    let covered: Vec<u8> = (DOMAIN_FLOOR..=255)
        .filter(|&b| t.stack_arity(FnIndex(b)).is_some())
        .collect();
    assert_eq!(covered, [0x90, 0x91, 0x92, 0x93, 0x94, 0x95]);
}

// ── F10: unsupported syntax ─────────────────────────────────────────────────

#[test]
fn f10_script_blocks_variables_and_separators_are_refused() {
    for (src, found) in [
        (
            "Get-MgUser a | Where-Object { $_.Department -eq 'Ops' } | Export-Report",
            '{',
        ),
        ("Get-MgUser $user | Export-Report", '$'),
        ("Get-MgUser a; Remove-MgUser a", ';'),
        ("Get-MgUser (Get-Content users.txt) | Export-Report", '('),
        ("Get-MgUser a | Export-Report > out.json", '>'),
        ("Get-MgUser \"$(whoami)\" | Export-Report", '$'),
        ("Get-MgUser a `\n | Export-Report", '`'),
        ("& Get-MgUser a | Export-Report", '&'),
        ("Get-MgUser @params | Export-Report", '@'),
    ] {
        assert_eq!(
            compile(src),
            Err(CompileError::UnsupportedSyntax { found }),
            "{src}"
        );
    }
}

#[test]
fn f10_unsupported_stages_and_shapes_are_refused() {
    assert_eq!(compile("Get-MgUser a"), Err(CompileError::NoExport));
    assert!(matches!(
        compile("Get-IamLicenseEvidence | Export-Report"),
        Err(CompileError::ShapeMismatch { .. })
    ));
    assert!(matches!(
        compile("Get-MgUser a | Get-MgUser b | Export-Report"),
        Err(CompileError::ShapeMismatch { .. })
    ));
    assert!(matches!(
        compile("Get-MgUser a | Export-Report | Select-Object DisplayName"),
        Err(CompileError::ShapeMismatch { .. })
    ));
    assert!(matches!(
        compile("Get-MgUser a | Where-Object Licenses -eq x | Export-Report"),
        Err(CompileError::ComparisonMismatch { .. })
    ));
    assert!(matches!(
        compile("Get-MgUser a | Where-Object Department -like x* | Export-Report"),
        Err(CompileError::UnknownParameter { .. })
    ));
    assert!(matches!(
        compile("Get-MgUser a | Select-Object Salary | Export-Report"),
        Err(CompileError::UnknownField(_))
    ));
    assert!(matches!(
        compile("Get-MgUser a | Export-Report -Format CSV"),
        Err(CompileError::UnknownFormat(_))
    ));
    assert_eq!(
        compile("Get-MgUser 'a | Export-Report"),
        Err(CompileError::UnterminatedString)
    );
    assert_eq!(
        compile("Get-MgUser a || Export-Report"),
        Err(CompileError::EmptyStage)
    );
}
