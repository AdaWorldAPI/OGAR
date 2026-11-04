//! Federation boundary: issuer-scoped resolution, store uniqueness, no email
//! linking, and the trust check. Integration test (separate crate) so only the
//! public surface is reachable — which is the point for `VerifiedLogin`.
//!
//! A hand-built `AuthContext::federated(..)` / `AuthenticatedUser` is possible
//! (public fields) but cannot become a `VerifiedLogin`; that is pinned by the
//! two `compile_fail` doctests in `ogar_auth::federation`.

use ogar_auth::federation::{
    AssertionVerifier, LoginError, TrustedIssuers, VerifiedClaims, establish,
};
use ogar_auth::identity::{ExternalIdentity, ExternalSubject, Issuer};
use ogar_auth::user::{
    AuthBinding, AuthContext, AuthStrength, AuthenticatedUser, KeyRef, LocalUserStore, ProviderId,
    StoreConflict, User, UserId, UserStore,
};

const ISS_A: &str = "https://idp-a.example";
const ISS_B: &str = "https://idp-b.example";
const PROV_A: ProviderId = ProviderId("idp-a");
const PROV_B: ProviderId = ProviderId("idp-b");

fn ident(iss: &str, sub: &str) -> ExternalIdentity {
    ExternalIdentity::new(
        Issuer::new(iss).expect("issuer"),
        ExternalSubject::new(sub).expect("subject"),
    )
}

fn user(id: u64, subject: &str, bindings: Vec<AuthBinding>) -> User {
    User {
        id: UserId(id),
        subject: subject.to_string(),
        tenant: 7,
        roles: vec!["physician".to_string()],
        memberships: vec![],
        bindings,
        key_refs: vec![KeyRef(format!("kms://user/{id}"))],
    }
}

/// A verifier that returns whatever claims it was configured with.
struct StubVerifier {
    claims: Result<VerifiedClaims, &'static str>,
}

impl StubVerifier {
    fn ok(identity: ExternalIdentity, email: Option<&str>) -> Self {
        Self {
            claims: Ok(VerifiedClaims {
                identity,
                strength: AuthStrength::MultiFactor,
                email: email.map(str::to_string),
            }),
        }
    }
}

impl AssertionVerifier for StubVerifier {
    type Input = str;
    type Error = &'static str;
    const ID: &'static str = "stub";

    fn verify(&self, _t: &TrustedIssuers, _input: &str) -> Result<VerifiedClaims, Self::Error> {
        self.claims.clone()
    }
}

fn trusted() -> TrustedIssuers {
    let mut t = TrustedIssuers::new();
    t.insert(Issuer::new(ISS_A).unwrap(), PROV_A).unwrap();
    t.insert(Issuer::new(ISS_B).unwrap(), PROV_B).unwrap();
    t
}

// ── F1: the pair, not the sub ───────────────────────────────────────────────

#[test]
fn same_sub_different_issuers_resolve_to_distinct_users() {
    let mut s = LocalUserStore::new();
    s.insert(user(1, "alice", vec![]));
    s.insert(user(2, "bob", vec![]));
    s.bind_external(UserId(1), ident(ISS_A, "same-sub"))
        .unwrap();
    s.bind_external(UserId(2), ident(ISS_B, "same-sub"))
        .unwrap();

    let a = s.resolve_external(&ident(ISS_A, "same-sub")).expect("A");
    let b = s.resolve_external(&ident(ISS_B, "same-sub")).expect("B");
    assert_eq!(a.id, UserId(1));
    assert_eq!(b.id, UserId(2));
    assert_ne!(a.id, b.id, "sub alone must not decide the user");
}

#[test]
fn same_issuer_same_sub_resolves_to_its_one_user() {
    let mut s = LocalUserStore::new();
    s.insert(user(1, "alice", vec![]));
    s.bind_external(UserId(1), ident(ISS_A, "x")).unwrap();
    let first = s.resolve_external(&ident(ISS_A, "x")).map(|u| u.id);
    let second = s.resolve_external(&ident(ISS_A, "x")).map(|u| u.id);
    assert_eq!(first, Some(UserId(1)));
    assert_eq!(first, second);
    // a trailing-slash issuer is a different issuer and resolves to nothing
    assert!(
        s.resolve_external(&ident("https://idp-a.example/", "x"))
            .is_none()
    );
}

// ── F2: store uniqueness ────────────────────────────────────────────────────

#[test]
fn binding_held_by_another_user_is_rejected() {
    let mut s = LocalUserStore::new();
    let shared = AuthBinding::new(ProviderId::LOCAL, "dr-house");
    s.try_insert(user(1, "alice", vec![shared.clone()]))
        .unwrap();
    let err = s
        .try_insert(user(2, "bob", vec![shared]))
        .expect_err("binding already held");
    assert_eq!(err, StoreConflict::BindingHeldBy { holder: UserId(1) });
    assert!(s.user(UserId(2)).is_none(), "store unchanged on error");
}

#[test]
fn duplicate_external_within_user_is_rejected() {
    let mut s = LocalUserStore::new();
    s.insert(user(1, "alice", vec![]));
    s.insert(user(2, "bob", vec![]));
    s.bind_external(UserId(1), ident(ISS_A, "x")).unwrap();
    assert_eq!(
        s.bind_external(UserId(1), ident(ISS_A, "x")),
        Err(StoreConflict::DuplicateExternal)
    );
    assert_eq!(
        s.bind_external(UserId(2), ident(ISS_A, "x")),
        Err(StoreConflict::ExternalHeldBy { holder: UserId(1) })
    );
    assert_eq!(
        s.bind_external(UserId(99), ident(ISS_A, "y")),
        Err(StoreConflict::UnknownUser(UserId(99)))
    );
    // never overwritten
    assert_eq!(
        s.resolve_external(&ident(ISS_A, "x")).map(|u| u.id),
        Some(UserId(1))
    );
}

#[test]
fn duplicate_subject_or_userid_is_rejected() {
    let mut s = LocalUserStore::new();
    s.insert(user(1, "alice", vec![]));
    assert_eq!(
        s.try_insert(user(1, "other", vec![])),
        Err(StoreConflict::DuplicateUserId(UserId(1)))
    );
    assert_eq!(
        s.try_insert(user(2, "alice", vec![])),
        Err(StoreConflict::DuplicateSubject("alice".to_string()))
    );
    let b = AuthBinding::new(ProviderId::LOCAL, "dup");
    assert_eq!(
        s.try_insert(user(3, "carol", vec![b.clone(), b])),
        Err(StoreConflict::DuplicateBinding)
    );
}

#[test]
#[should_panic(expected = "conflicting user")]
fn insert_panics_on_conflict() {
    let mut s = LocalUserStore::new();
    s.insert(user(1, "alice", vec![]));
    s.insert(user(1, "alice", vec![]));
}

#[test]
fn distinct_bindings_insert_and_resolve_uniquely() {
    let mut s = LocalUserStore::new();
    s.try_insert(user(
        1,
        "alice",
        vec![AuthBinding::new(ProviderId::LOCAL, "alice")],
    ))
    .unwrap();
    s.try_insert(user(
        2,
        "bob",
        vec![AuthBinding::new(ProviderId::LOCAL, "bob")],
    ))
    .unwrap();
    assert_eq!(
        s.resolve(&AuthBinding::new(ProviderId::LOCAL, "alice"))
            .map(|u| u.id),
        Some(UserId(1))
    );
    assert_eq!(
        s.resolve(&AuthBinding::new(ProviderId::LOCAL, "bob"))
            .map(|u| u.id),
        Some(UserId(2))
    );
}

// ── F3: no email linking ────────────────────────────────────────────────────

#[test]
fn matching_email_with_other_iss_sub_resolves_to_nothing() {
    let mut s = LocalUserStore::new();
    s.insert(user(
        1,
        "house",
        vec![AuthBinding::new(ProviderId::LOCAL, "house@ex.org")],
    ));
    s.bind_external(UserId(1), ident(ISS_A, "x")).unwrap();

    // Different (iss, sub), but the email equals the local username.
    let v = StubVerifier::ok(ident(ISS_B, "y"), Some("house@ex.org"));
    let r = establish(&v, &trusted(), "tok", &s);
    assert!(matches!(r, Err(LoginError::Unbound)), "got {r:?}");
}

#[test]
fn bound_identity_with_any_email_resolves() {
    let mut s = LocalUserStore::new();
    s.insert(user(1, "house", vec![]));
    s.bind_external(UserId(1), ident(ISS_A, "x")).unwrap();
    for email in [None, Some("unrelated@ex.org"), Some("")] {
        let v = StubVerifier::ok(ident(ISS_A, "x"), email);
        let login = establish(&v, &trusted(), "tok", &s).expect("bound identity resolves");
        assert_eq!(login.authenticated_user().user.id, UserId(1));
    }
}

// ── F4: trust + proof ───────────────────────────────────────────────────────

#[test]
fn untrusted_issuer_is_refused() {
    let mut s = LocalUserStore::new();
    s.insert(user(1, "alice", vec![]));
    let rogue = "https://rogue.example";
    s.bind_external(UserId(1), ident(rogue, "x")).unwrap();
    let v = StubVerifier::ok(ident(rogue, "x"), None);
    let r = establish(&v, &trusted(), "tok", &s);
    assert!(matches!(r, Err(LoginError::UntrustedIssuer)), "got {r:?}");
}

#[test]
fn verifier_failure_is_surfaced() {
    let s = LocalUserStore::new();
    let v = StubVerifier {
        claims: Err("bad signature"),
    };
    let r = establish(&v, &trusted(), "tok", &s);
    assert!(
        matches!(r, Err(LoginError::Verify("bad signature"))),
        "got {r:?}"
    );
}

#[test]
fn registered_verifier_and_binding_yield_verified_login() {
    let mut s = LocalUserStore::new();
    s.insert(user(1, "alice", vec![]));
    s.bind_external(UserId(1), ident(ISS_A, "x")).unwrap();
    let v = StubVerifier::ok(ident(ISS_A, "x"), None);
    let login = establish(&v, &trusted(), "tok", &s).expect("login");

    assert_eq!(login.verifier_id(), "stub");
    assert_eq!(login.identity(), &ident(ISS_A, "x"));
    let au = login.authenticated_user();
    assert_eq!(au.user.id, UserId(1));
    assert!(au.auth.authenticated);
    assert_eq!(au.auth.provider, PROV_A, "provider comes from the registry");
    assert_eq!(au.auth.strength, AuthStrength::MultiFactor);
    assert_eq!(login.actor_context(), au.actor_context());
    assert_eq!(login.into_authenticated_user().user.id, UserId(1));
}

#[test]
fn hand_built_context_gives_a_user_but_no_login() {
    // A hand-built AuthenticatedUser is possible (public fields)...
    let u = AuthenticatedUser {
        user: user(1, "alice", vec![]),
        auth: AuthContext::federated(PROV_A, AuthStrength::MultiFactor),
    };
    assert!(u.auth.authenticated);
    // ...but there is no way to turn it into a `VerifiedLogin`: no struct
    // literal (private fields) and no `From`. The two `compile_fail` doctests
    // in `ogar_auth::federation` pin that.
}
