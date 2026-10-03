//! The simulation slice: observe → simulate → validate → diff → plan.

use ogar_dir_core::Guid128;
use ogar_dir_sim::rule::{GrantGroup, ImplyGroup, SetPrimarySmtp};
use ogar_dir_sim::*;

fn g(n: u8) -> Guid128 {
    Guid128([n; 16])
}
const ALICE: u8 = 0xA1;
const BOB: u8 = 0xB0;
const EMPLOYEES: u8 = 0xE0;
const EXCHANGE: u8 = 0xEC;

const EXCHANGE_ACCESS: RuleId = RuleId {
    name: "ExchangeAccess",
    version: 1,
};
const EMPLOYEES_GET_EXCHANGE: RuleId = RuleId {
    name: "EmployeesGetExchange",
    version: 1,
};
const RENAME_MAIL: RuleId = RuleId {
    name: "RenameMail",
    version: 1,
};

/// Observed G0: Alice and Bob in Employees; ExchangeUsers exists, empty.
fn observed() -> GraphState {
    let mut s = GraphState::new();
    s.put_node(
        g(ALICE),
        Node::user("Alice", "alice@example.test", "alice@example.test"),
    );
    s.put_node(
        g(BOB),
        Node::user("Bob", "bob@example.test", "bob@example.test"),
    );
    s.put_node(g(EMPLOYEES), Node::group("Employees"));
    s.put_node(g(EXCHANGE), Node::group("ExchangeUsers"));
    s.put_membership(g(ALICE), g(EMPLOYEES));
    s.put_membership(g(BOB), g(EMPLOYEES));
    s
}

fn grant_alice() -> GrantGroup {
    GrantGroup {
        rule: EXCHANGE_ACCESS,
        group: g(EXCHANGE),
        to: vec![g(ALICE)],
    }
}
fn imply() -> ImplyGroup {
    ImplyGroup {
        rule: EMPLOYEES_GET_EXCHANGE,
        source: g(EMPLOYEES),
        target: g(EXCHANGE),
    }
}
fn ev(s: &str) -> Vec<EvidenceRef> {
    vec![EvidenceRef(s.into())]
}

/// G0 → G1 (ExchangeAccess for Alice) → G2 (all employees get Exchange).
fn chain() -> (VersionStore, VersionId, VersionId, VersionId) {
    let mut st = VersionStore::new();
    let g0 = st.observe("lab", 1_000, observed());
    let g1 = st.simulate(g0, &grant_alice(), &ev("REQ-1")).unwrap();
    let g2 = st.simulate(g1, &imply(), &ev("POLICY-7")).unwrap();
    (st, g0, g1, g2)
}

// 1. G0 remains immutable after simulation.
#[test]
fn t01_observed_version_is_immutable() {
    let (st, g0, ..) = chain();
    assert_eq!(st.state(g0).unwrap(), observed());
    assert!(st.version(g0).unwrap().delta.is_empty());
    assert!(!st.state(g0).unwrap().is_member(g(ALICE), g(EXCHANGE)));
}

// 2. A rule produces G1 with G0 as parent; 10. provenance names the rule.
#[test]
fn t02_t10_rule_produces_child_with_provenance() {
    let (st, g0, g1, _) = chain();
    let v1 = st.version(g1).unwrap();
    assert_eq!(v1.parent, Some(g0));
    assert_eq!(
        v1.origin,
        Origin::Simulated {
            rule: EXCHANGE_ACCESS,
            evidence: ev("REQ-1")
        }
    );
    assert_eq!(
        v1.delta,
        vec![Change::AddMembership {
            user: g(ALICE),
            group: g(EXCHANGE)
        }]
    );
    assert!(st.state(g1).unwrap().is_member(g(ALICE), g(EXCHANGE)));
}

// 3. A second rule produces G2 from G1 — and it is population-shaped:
//    it adds only Bob, because Alice already holds the target.
#[test]
fn t03_chained_rule_from_g1() {
    let (st, g0, g1, g2) = chain();
    let v2 = st.version(g2).unwrap();
    assert_eq!(v2.parent, Some(g1));
    assert_eq!(
        v2.delta,
        vec![Change::AddMembership {
            user: g(BOB),
            group: g(EXCHANGE)
        }]
    );
    assert_eq!(st.lineage(g2).unwrap(), vec![g0, g1, g2]);
    // running the population rule on G2 again has nothing left to do
    let mut st = st;
    assert_eq!(
        st.simulate(g2, &imply(), &[]),
        Err(SimError::EmptyProposal(EMPLOYEES_GET_EXCHANGE))
    );
}

// 4. diff(G0, G1) is exactly one semantic change.
#[test]
fn t04_diff_is_one_semantic_change() {
    let (st, g0, g1, g2) = chain();
    assert_eq!(
        st.diff(g0, g1).unwrap(),
        vec![Change::AddMembership {
            user: g(ALICE),
            group: g(EXCHANGE)
        }]
    );
    assert_eq!(st.diff(g0, g2).unwrap().len(), 2);
    assert!(st.diff(g1, g1).unwrap().is_empty());
}

// 5. Valid group membership passes invariants.
#[test]
fn t05_valid_version_passes() {
    let (mut st, g0, _, g2) = chain();
    assert!(st.validate(g0).unwrap().is_empty());
    assert!(st.validate(g2).unwrap().is_empty());
    st.promote_desired(g2).unwrap();
    assert_eq!(st.tag(TAG_DESIRED), Some(g2));
}

// 6. A dangling membership fails, with structured evidence.
#[test]
fn t06_dangling_membership_fails() {
    let ghost = g(0x66);
    let mut st = VersionStore::new();
    let g0 = st.observe("lab", 0, observed());
    let bad = st
        .simulate(
            g0,
            &GrantGroup {
                rule: EXCHANGE_ACCESS,
                group: ghost,
                to: vec![g(ALICE)],
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        st.validate(bad).unwrap(),
        vec![Violation::DanglingMembership {
            user: g(ALICE),
            group: ghost,
            missing: Endpoint::Group
        }]
    );
    // a user-side dangling edge, and a group used as a member, both fail too
    let mut s = observed();
    s.put_membership(ghost, g(EXCHANGE));
    s.put_membership(g(EMPLOYEES), g(EXCHANGE));
    let v = validate(&s);
    assert!(v.contains(&Violation::DanglingMembership {
        user: ghost,
        group: g(EXCHANGE),
        missing: Endpoint::User
    }));
    assert!(v.contains(&Violation::DanglingMembership {
        user: g(EMPLOYEES),
        group: g(EXCHANGE),
        missing: Endpoint::User
    }));
}

// 7. Duplicate UPN fails (normalized), but an inactive owner does not count.
#[test]
fn t07_duplicate_upn_fails() {
    let mut s = observed();
    s.put_node(
        g(0x77),
        Node::user("Imposter", "  ALICE@example.test", "other@example.test"),
    );
    assert_eq!(
        validate(&s),
        vec![Violation::DuplicateUpn {
            upn: "alice@example.test".into(),
            owners: vec![g(0x77), g(ALICE)]
        }]
    );
    let mut disabled = Node::user("Imposter", "alice@example.test", "other@example.test");
    disabled.active = false;
    s.put_node(g(0x77), disabled);
    assert!(validate(&s).is_empty());
}

// 8 + 9 + rejected future. Bob.primarySMTP = alice@… is constructible,
// detected, refused for promotion, kept as evidence — and nothing else moved.
#[test]
fn t08_t09_smtp_collision_is_rejected_without_touching_ancestors() {
    let (mut st, g0, g1, g2) = chain();
    st.promote_desired(g2).unwrap();
    let before: Vec<_> = [g0, g1, g2].iter().map(|v| st.state(*v).unwrap()).collect();

    let rule = SetPrimarySmtp {
        rule: RENAME_MAIL,
        user: g(BOB),
        to: "Alice@Example.test".into(),
    };
    let g3 = st
        .simulate(g2, &rule, &ev("REQ-2"))
        .expect("the hypothetical future is constructible");
    let rej = st.promote_desired(g3).unwrap_err();
    assert_eq!(rej.version, g3);
    assert_eq!(
        rej.violations,
        vec![Violation::DuplicateSmtp {
            address: "alice@example.test".into(),
            owners: vec![g(ALICE), g(BOB)]
        }]
    );
    // 9: desired tag unchanged; every ancestor identical; G3 kept as evidence
    assert_eq!(st.tag(TAG_DESIRED), Some(g2));
    let after: Vec<_> = [g0, g1, g2].iter().map(|v| st.state(*v).unwrap()).collect();
    assert_eq!(before, after);
    assert_eq!(st.verdict(g3), Some(rej.violations.as_slice()));
    assert_eq!(
        st.state(g3)
            .unwrap()
            .node(&g(BOB))
            .unwrap()
            .primary_smtp
            .as_deref(),
        Some("Alice@Example.test")
    );
}

// A stale compare-and-set is refused and creates no version.
#[test]
fn stale_change_creates_no_version() {
    struct Stale;
    impl Rule for Stale {
        fn id(&self) -> RuleId {
            RENAME_MAIL
        }
        fn propose(&self, _: &GraphState, _: &[EvidenceRef]) -> Vec<Change> {
            vec![Change::SetAttribute {
                node: g(BOB),
                attribute: Attribute::PrimarySmtp,
                from: Some("not-what-it-is@example.test".into()),
                to: Some("x@example.test".into()),
            }]
        }
    }
    let mut st = VersionStore::new();
    let g0 = st.observe("lab", 0, observed());
    assert!(matches!(
        st.simulate(g0, &Stale, &[]),
        Err(SimError::Apply(_))
    ));
    assert!(st.version(VersionId(1)).is_none());
}

// 11. The semantic diff becomes an ExecutionPlan with no shell information,
//     and every op carries the precondition from the observed basis.
#[test]
fn t11_plan_is_semantic_with_preconditions() {
    let (mut st, g0, g1, g2) = chain();
    assert_eq!(
        ExecutionPlan::derive(&st, g2),
        Err(PlanError::NotDesired(g2)),
        "only desired versions plan"
    );
    st.promote_desired(g2).unwrap();
    let plan = ExecutionPlan::derive(&st, g2).unwrap();
    assert_eq!((plan.basis, plan.target), (g0, g2));
    assert_eq!(
        plan.ops,
        vec![
            PlannedOp {
                op: Operation::AddGroupMember {
                    group: g(EXCHANGE),
                    member: g(ALICE)
                },
                precondition: Precondition::NotMember
            },
            PlannedOp {
                op: Operation::AddGroupMember {
                    group: g(EXCHANGE),
                    member: g(BOB)
                },
                precondition: Precondition::NotMember
            },
        ]
    );
    let _ = g1;
    // An attribute op carries the value it expects reality to still hold.
    let mut st2 = VersionStore::new();
    let b0 = st2.observe("lab", 0, observed());
    let rule = SetPrimarySmtp {
        rule: RENAME_MAIL,
        user: g(BOB),
        to: "robert@example.test".into(),
    };
    let b1 = st2.simulate(b0, &rule, &[]).unwrap();
    st2.promote_desired(b1).unwrap();
    assert_eq!(
        ExecutionPlan::derive(&st2, b1).unwrap().ops,
        vec![PlannedOp {
            op: Operation::SetAttribute {
                object: g(BOB),
                attribute: Attribute::PrimarySmtp,
                value: Some("robert@example.test".into())
            },
            precondition: Precondition::AttributeEquals(Some("bob@example.test".into())),
        }]
    );
}

// 12. Same observed input + same rules ⇒ same desired state and same plan.
#[test]
fn t12_deterministic() {
    let run = || {
        let (mut st, _, _, g2) = chain();
        st.promote_desired(g2).unwrap();
        (
            st.state(g2).unwrap(),
            ExecutionPlan::derive(&st, g2).unwrap(),
        )
    };
    assert_eq!(run(), run());
}

// Audit: why is Alice in ExchangeUsers?  observed G0 → ExchangeAccess/v1 → G1.
#[test]
fn audit_chain_reconstructs_the_cause() {
    let (st, g0, g1, g2) = chain();
    let why = st.explain_membership(g2, g(ALICE), g(EXCHANGE)).unwrap();
    assert_eq!(why.iter().map(|v| v.id).collect::<Vec<_>>(), vec![g0, g1]);
    assert!(matches!(why[0].origin, Origin::Observed { .. }));
    match &why[1].origin {
        Origin::Simulated { rule, evidence } => {
            assert_eq!(rule.to_string(), "ExchangeAccess/v1");
            assert_eq!(evidence, &ev("REQ-1"));
        }
        o => panic!("unexpected origin {o:?}"),
    }
    // Bob's membership has a different cause; Employees was observed.
    let bob = st.explain_membership(g2, g(BOB), g(EXCHANGE)).unwrap();
    assert_eq!(bob.last().unwrap().id, g2);
    assert_eq!(
        st.explain_membership(g2, g(ALICE), g(EMPLOYEES))
            .unwrap()
            .len(),
        1
    );
    assert!(st.explain_membership(g0, g(ALICE), g(EXCHANGE)).is_none());
}

// Convergence check shape: a later observation equal to desired has an empty diff.
#[test]
fn observing_the_desired_state_converges() {
    let (mut st, _, _, g2) = chain();
    st.promote_desired(g2).unwrap();
    let after = st.state(g2).unwrap();
    let o = st.observe("lab", 2_000, after);
    assert!(st.diff(o, st.tag(TAG_DESIRED).unwrap()).unwrap().is_empty());
    assert_eq!(st.tag(TAG_OBSERVED), Some(o));
}

// Observed state from PR #313 records.
#[test]
fn observe_from_ogar_ad_records() {
    use ogar_dir_core::{OuDictionary, ValuePool};
    let ldif = "dn: CN=Alice,OU=Staff,DC=example,DC=test\nobjectGUID:: 4AQlP4lP0xGaDAMF6CwzAQ==\nobjectClass: user\nuserPrincipalName: alice@example.test\nproxyAddresses: smtp:a@legacy.test\nproxyAddresses: SMTP:alice@example.test\nuserAccountControl: 512\n\ndn: CN=Employees,OU=Groups,DC=example,DC=test\nobjectGUID:: 1MOyoQAAAECAAAAAAAC+7w==\nobjectClass: group\n";
    let (mut d, mut p) = (OuDictionary::new(), ValuePool::new());
    let recs: Vec<_> = ogar_ad::ldif::parse(ldif)
        .unwrap()
        .iter()
        .map(|e| {
            ogar_ad::encode(e, Guid128::NIL, &mut d, &mut p, 0)
                .unwrap()
                .record
        })
        .collect();
    let s = observe::from_ad(&recs, &p);
    let a = s.node(&recs[0].node_guid()).unwrap();
    assert_eq!((a.kind, a.active), (NodeKind::User, true));
    assert_eq!(a.primary_smtp.as_deref(), Some("alice@example.test"));
    assert_eq!(s.node(&recs[1].node_guid()).unwrap().kind, NodeKind::Group);
}
