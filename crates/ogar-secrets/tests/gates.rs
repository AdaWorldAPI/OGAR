//! Gate tests for the tenant credential broker. Synthetic data only.

use std::cell::RefCell;

use lance_graph_contract::rbac::{ActorId, ClassGrant, OpMask, RoleId};
use lance_graph_contract::rbac_plug::{RbacAuthority, RbacBinding, RbacPlug};
use ogar_auth::federation::{
    AssertionVerifier, TrustedIssuers, VerifiedClaims, VerifiedLogin, establish,
};
use ogar_auth::identity::{ExternalIdentity, ExternalSubject, Issuer};
use ogar_auth::user::{AuthStrength, LocalUserStore, ProviderId, User, UserId};
use ogar_rbac::{GrantSource, OgarRbac};
use ogar_secrets::{
    AccessRecord, AuditSink, Broker, BrokerOutcome, CredentialGrant, CredentialKind,
    CredentialRequest, GrantRefusal, InMemoryProvider, OutcomeClass, Redacted, RefusalReason,
    TenantSecretBroker, UnavailableReason,
};
use ogar_vocab::class_ids::{CRM_COMPANY, TENANT_CREDENTIAL};

const ISS: &str = "https://zitadel.lab.example";
const PROV: ProviderId = ProviderId("zitadel");

const SENTINEL: RbacPlug = RbacPlug {
    consumer: "sentinel-rs",
    classids: &[TENANT_CREDENTIAL, CRM_COMPANY],
    roles: &["credential-reader", "analyst"],
};
const PORTAL: RbacPlug = RbacPlug {
    consumer: "report-rs",
    ..SENTINEL
};
const NO_CRED: RbacPlug = RbacPlug {
    classids: &[CRM_COMPANY],
    ..SENTINEL
};

struct Grants {
    reader: Vec<ClassGrant>,
    analyst: Vec<ClassGrant>,
}
impl GrantSource for Grants {
    fn roles_of(&self, _actor: ActorId<'_>) -> &[RoleId] {
        &[]
    }
    fn grants_of(&self, role: RoleId) -> &[ClassGrant] {
        match role {
            "credential-reader" => &self.reader,
            "analyst" => &self.analyst,
            _ => &[],
        }
    }
    fn defines_role(&self, role: RoleId) -> bool {
        matches!(role, "credential-reader" | "analyst")
    }
}

fn authority() -> OgarRbac<Grants> {
    OgarRbac::new(Grants {
        reader: vec![ClassGrant::new(TENANT_CREDENTIAL, OpMask::ACT)],
        analyst: vec![ClassGrant::new(CRM_COMPANY, OpMask::READ)],
    })
}

fn bind(plug: &RbacPlug) -> RbacBinding {
    authority().bind(plug).expect("green bind")
}

fn ident(sub: &str) -> ExternalIdentity {
    ExternalIdentity::new(
        Issuer::new(ISS).expect("issuer"),
        ExternalSubject::new(sub).expect("subject"),
    )
}

/// Stub verifier: the input string is the subject.
struct Stub;
impl AssertionVerifier for Stub {
    type Input = str;
    type Error = ();
    const ID: &'static str = "stub";
    fn verify(&self, _t: &TrustedIssuers, input: &str) -> Result<VerifiedClaims, ()> {
        Ok(VerifiedClaims {
            identity: ident(input),
            strength: AuthStrength::MultiFactor,
            email: None,
        })
    }
}

fn user(id: u64, subject: &str, tenant: u64, role: &str) -> User {
    User {
        id: UserId(id),
        subject: subject.to_string(),
        tenant,
        roles: vec![role.to_string()],
        memberships: vec![],
        bindings: vec![],
        key_refs: vec![],
    }
}

fn login(subject: &str) -> VerifiedLogin {
    let mut store = LocalUserStore::new();
    for (id, sub, tenant, role) in [
        (1, "wl-a", 1001, "credential-reader"),
        (2, "wl-b", 1002, "credential-reader"),
        (3, "analyst-a", 1001, "analyst"),
    ] {
        store.insert(user(id, sub, tenant, role));
        store.bind_external(UserId(id), ident(sub)).unwrap();
    }
    let mut trusted = TrustedIssuers::new();
    trusted.insert(Issuer::new(ISS).unwrap(), PROV).unwrap();
    establish(&Stub, &trusted, subject, &store).expect("login")
}

fn grant(subject: &str, plug: &RbacPlug) -> CredentialGrant {
    CredentialGrant::authorize(&bind(plug), &login(subject)).expect("grant")
}

#[derive(Default)]
struct Sink(RefCell<Vec<AccessRecord>>);
impl AuditSink for Sink {
    fn record(&self, r: AccessRecord) {
        self.0.borrow_mut().push(r);
    }
}

fn seeded() -> InMemoryProvider {
    InMemoryProvider::new()
        .with_secret(
            "kv/data/lab/tenants/1001/sentinel-rs/graph/app",
            1,
            "synthetic-secret-A-v1",
        )
        .with_secret(
            "kv/data/lab/tenants/1001/sentinel-rs/graph/app",
            2,
            "synthetic-secret-A-v2",
        )
        .with_secret(
            "kv/data/lab/tenants/1002/sentinel-rs/graph/app",
            1,
            "synthetic-secret-B-v1",
        )
        .with_secret(
            "kv/data/lab/tenants/1001/report-rs/graph/app",
            1,
            "synthetic-secret-portal-v1",
        )
}

fn broker(p: InMemoryProvider) -> Broker<InMemoryProvider, Sink> {
    Broker::new("lab", p, Sink::default()).expect("environment")
}

fn graph(version: Option<u32>) -> CredentialRequest {
    CredentialRequest {
        kind: CredentialKind::GraphApp,
        version,
    }
}

fn released(o: BrokerOutcome) -> String {
    match o {
        BrokerOutcome::Released(m) => m.secret.expose().clone(),
        other => panic!("expected Released, got {other:?}"),
    }
}

/// Disable control: build the path from a request-supplied tenant instead of the grant.
#[test]
fn v0_tenant_a_releases_only_its_own_credential() {
    let b = broker(seeded());
    let a = released(b.release(&grant("wl-a", &SENTINEL), graph(None)));
    assert_eq!(a, "synthetic-secret-A-v2");
    assert!(
        b.provider()
            .calls()
            .iter()
            .all(|p| p.contains("/tenants/1001/")),
        "calls: {:?}",
        b.provider().calls()
    );
    let bv = released(b.release(&grant("wl-b", &SENTINEL), graph(None)));
    assert_eq!(bv, "synthetic-secret-B-v1");
    assert_ne!(a, bv);
}

/// Disable control: remove the portal-only path from the fixture (the broker must not reach it).
#[test]
fn v2_sentinel_workload_cannot_reach_portal_secret() {
    let only_portal = InMemoryProvider::new().with_secret(
        "kv/data/lab/tenants/1001/report-rs/graph/app",
        1,
        "synthetic-secret-portal-v1",
    );
    let b = broker(only_portal);
    let o = b.release(&grant("wl-a", &SENTINEL), graph(None));
    assert!(matches!(
        o,
        BrokerOutcome::Unavailable(UnavailableReason::NotFound)
    ));
    assert!(
        b.provider()
            .calls()
            .iter()
            .all(|p| !p.contains("report-rs"))
    );
    // The same fixture does release for the portal consumer: the test can fail.
    let ok = b.release(&grant("wl-a", &PORTAL), graph(None));
    assert_eq!(released(ok), "synthetic-secret-portal-v1");
}

/// Disable control: map a missing path or version to an empty Released.
#[test]
fn v3_missing_credential_is_unavailable_not_empty() {
    let b = broker(seeded());
    let g = grant("wl-a", &SENTINEL);
    let o = b.release(
        &g,
        CredentialRequest {
            kind: CredentialKind::Certificate,
            version: None,
        },
    );
    assert!(matches!(
        o,
        BrokerOutcome::Unavailable(UnavailableReason::NotFound)
    ));
    let o = b.release(&g, graph(Some(9)));
    assert!(matches!(
        o,
        BrokerOutcome::Unavailable(UnavailableReason::VersionNotFound)
    ));
}

/// Disable control: derive Debug on the secret wrapper without redaction.
#[test]
fn v4_secret_never_in_debug_display_audit() {
    let b = broker(seeded());
    let g = grant("wl-a", &SENTINEL);
    let o = b.release(&g, graph(None));
    let shown = format!("{o:?}");
    assert!(shown.contains("[redacted]"), "{shown}");
    assert!(!shown.contains("synthetic-secret"));
    let r = Redacted::new("synthetic-secret-x".to_string());
    assert_eq!(format!("{r}"), "[redacted]");
    assert_eq!(format!("{r:?}"), "[redacted]");
    let _ = b.release(&g, graph(Some(9)));
    for rec in b.audit().0.borrow().iter() {
        assert!(!format!("{rec:?}").contains("synthetic-secret"));
    }
    assert!(!format!("{g:?}").contains("synthetic-secret"));
}

/// Disable control: drop the label rule on the environment name.
#[test]
fn v4_dot_segments_are_refused() {
    let bad = Broker::new("..", InMemoryProvider::new(), Sink::default()).err();
    assert_eq!(bad, Some(RefusalReason::InvalidEnvironment));
    assert!(Broker::new("lab", InMemoryProvider::new(), Sink::default()).is_ok());
}

/// Disable control: remove ACT on tenant_credential from the analyst check (give analyst the grant).
#[test]
fn v6_human_session_yields_no_grant() {
    let r = CredentialGrant::authorize(&bind(&SENTINEL), &login("analyst-a"));
    assert_eq!(r.err(), Some(GrantRefusal::Denied));
}

/// Disable control: remove the `plugged` check (the kernel would still deny, as Denied).
#[test]
fn v6_binding_without_credential_concept_yields_no_grant() {
    let r = CredentialGrant::authorize(&bind(&NO_CRED), &login("wl-a"));
    assert_eq!(r.err(), Some(GrantRefusal::NotPlugged));
}

/// Disable control: drop the label rule on the consumer name.
#[test]
fn v4_consumer_name_outside_the_label_rule_yields_no_grant() {
    const BAD: RbacPlug = RbacPlug {
        consumer: "../report-rs",
        ..SENTINEL
    };
    let r = CredentialGrant::authorize(&bind(&BAD), &login("wl-a"));
    assert_eq!(r.err(), Some(GrantRefusal::InvalidConsumer));
}

/// Disable control: map ProviderDown/Sealed to a Released value.
#[test]
fn v7_provider_down_fails_closed() {
    let g = grant("wl-a", &SENTINEL);
    let down = broker(seeded().with_down(true));
    assert!(matches!(
        down.release(&g, graph(None)),
        BrokerOutcome::Unavailable(UnavailableReason::ProviderDown)
    ));
    let sealed = broker(seeded().with_sealed(true));
    assert!(matches!(
        sealed.release(&g, graph(None)),
        BrokerOutcome::Unavailable(UnavailableReason::Sealed)
    ));
}

/// Disable control: add an env fallback to the crate source.
#[test]
fn v7_crate_has_no_env_access() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut seen = 0;
    for entry in std::fs::read_dir(dir).expect("src") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).expect("read");
            // Built at run time so this file's own text never matches itself.
            let a = ["std", "::", "env"].concat();
            let b = ["env", "::", "var"].concat();
            assert!(!text.contains(&a), "{path:?}");
            assert!(!text.contains(&b), "{path:?}");
            seen += 1;
        }
    }
    assert!(seen >= 2, "scanned {seen} files");
}

/// Disable control: skip `audit.record` on a path (counts would drop).
#[test]
fn audit_records_every_outcome() {
    let b = broker(seeded());
    let g = grant("wl-a", &SENTINEL);
    let _ = b.release(&g, graph(None));
    let _ = b.release(&g, graph(Some(9)));
    let recs = b.audit().0.borrow();
    assert_eq!(recs.len(), 2);
    assert_eq!(recs[0].outcome, OutcomeClass::Released);
    assert_eq!(recs[0].version, Some(2));
    assert_eq!(
        recs[1].outcome,
        OutcomeClass::Unavailable(UnavailableReason::VersionNotFound)
    );
    assert!(recs.iter().all(|r| r.tenant == 1001 && r.subject == "wl-a"));
    assert!(recs.iter().all(|r| r.consumer == "sentinel-rs"));
}

/// Disable control: make the provider ignore the requested version.
#[test]
fn released_values_differ_by_version() {
    let b = broker(seeded());
    let g = grant("wl-a", &SENTINEL);
    let v1 = released(b.release(&g, graph(Some(1))));
    let v2 = released(b.release(&g, graph(Some(2))));
    let latest = released(b.release(&g, graph(None)));
    assert_ne!(v1, v2);
    assert_eq!(latest, v2);
}
