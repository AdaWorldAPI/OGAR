//! Integration tests for `ogar-oidc`: the public surface only. Tokens are
//! signed in-test with fixed keys; every failure asserts the exact variant.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ogar_auth::federation::{AssertionVerifier, LoginError, TrustedIssuers, establish};
use ogar_auth::identity::{ExternalIdentity, ExternalSubject, Issuer};
use ogar_auth::user::{AuthStrength, KeyRef, LocalUserStore, ProviderId, User, UserId};
use ogar_encryption::sign::Keypair;
use ogar_oidc::{
    Client, Discovery, DiscoveryError, IdToken, KeySet, KeySetError, OidcError, OidcVerifier,
};
use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::Signer;
use serde_json::{Value, json};

const NOW: u64 = 1_800_000_000;
const ISS_A: &str = "https://idp-a.example";
const ISS_B: &str = "https://idp-b.example";
const CLIENT: &str = "client-a";
const PROV_A: ProviderId = ProviderId("idp-a");
const PROV_B: ProviderId = ProviderId("idp-b");

// ── helpers ─────────────────────────────────────────────────────────────────

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn es_key(seed_byte: u8) -> SigningKey {
    SigningKey::from_slice(&[seed_byte; 32]).expect("valid scalar")
}

fn jwk_es256(sk: &SigningKey, kid: &str) -> Value {
    // Uncompressed SEC1: 0x04 || x(32) || y(32).
    let sec1 = sk.verifying_key().to_sec1_bytes();
    assert_eq!(sec1.len(), 65);
    json!({
        "kty": "EC",
        "crv": "P-256",
        "kid": kid,
        "use": "sig",
        "x": b64(&sec1[1..33]),
        "y": b64(&sec1[33..]),
    })
}

fn jwk_ed25519(kp: &Keypair, kid: &str) -> Value {
    json!({
        "kty": "OKP",
        "crv": "Ed25519",
        "kid": kid,
        "use": "sig",
        "x": b64(&kp.public_key()),
    })
}

fn without(mut jwk: Value, field: &str) -> Value {
    jwk.as_object_mut().expect("object").remove(field);
    jwk
}

fn keyset(jwks: Vec<Value>) -> KeySet {
    let doc = json!({ "keys": jwks });
    KeySet::from_jwks_json(doc.to_string().as_bytes()).expect("keyset")
}

fn signing_input(header: &Value, claims: &Value) -> String {
    format!(
        "{}.{}",
        b64(header.to_string().as_bytes()),
        b64(claims.to_string().as_bytes())
    )
}

fn sign_es256(sk: &SigningKey, header: Value, claims: Value) -> String {
    let input = signing_input(&header, &claims);
    let sig: p256::ecdsa::Signature = sk.sign(input.as_bytes());
    format!("{input}.{}", b64(&sig.to_bytes()))
}

fn sign_ed25519(kp: &Keypair, header: Value, claims: Value) -> String {
    let input = signing_input(&header, &claims);
    let sig = kp.sign(input.as_bytes());
    format!("{input}.{}", b64(&sig))
}

fn es_header(kid: &str) -> Value {
    json!({ "alg": "ES256", "typ": "JWT", "kid": kid })
}

fn base_claims(iss: &str) -> Value {
    json!({
        "iss": iss,
        "sub": "user-123",
        "aud": CLIENT,
        "iat": NOW,
        "exp": NOW + 300,
    })
}

fn drop_claim(mut claims: Value, name: &str) -> Value {
    claims.as_object_mut().expect("object").remove(name);
    claims
}

fn issuer(s: &str) -> Issuer {
    Issuer::new(s).expect("issuer")
}

fn trusted_both() -> TrustedIssuers {
    let mut t = TrustedIssuers::new();
    t.insert(issuer(ISS_A), PROV_A).unwrap();
    t.insert(issuer(ISS_B), PROV_B).unwrap();
    t
}

fn verifier_a(keys: KeySet) -> OidcVerifier {
    let mut v = OidcVerifier::new();
    v.add_client(Client::new(issuer(ISS_A), CLIENT, keys));
    v
}

fn input(token: String) -> IdToken {
    IdToken {
        token,
        now_unix: NOW,
        nonce: None,
    }
}

fn run(
    v: &OidcVerifier,
    token: String,
) -> Result<ogar_auth::federation::VerifiedClaims, OidcError> {
    v.verify(&trusted_both(), &input(token))
}

/// ES256 happy-path scaffold: key "a1", verifier for idp-a, token over `claims`.
fn es_run(claims: Value) -> Result<ogar_auth::federation::VerifiedClaims, OidcError> {
    let sk = es_key(7);
    let v = verifier_a(keyset(vec![jwk_es256(&sk, "a1")]));
    run(&v, sign_es256(&sk, es_header("a1"), claims))
}

fn user(id: u64, subject: &str) -> User {
    User {
        id: UserId(id),
        subject: subject.to_string(),
        tenant: 7,
        roles: vec!["physician".to_string()],
        memberships: vec![],
        bindings: vec![],
        key_refs: vec![KeyRef(format!("kms://user/{id}"))],
    }
}

fn ident(iss: &str, sub: &str) -> ExternalIdentity {
    ExternalIdentity::new(issuer(iss), ExternalSubject::new(sub).unwrap())
}

// ── 1-2: valid tokens ───────────────────────────────────────────────────────

#[test]
fn valid_es256_token_is_accepted() {
    let claims = es_run(base_claims(ISS_A)).expect("valid token");
    assert_eq!(claims.identity, ident(ISS_A, "user-123"));
    assert_eq!(claims.strength, AuthStrength::SingleFactor);
    assert_eq!(claims.email, None);
}

#[test]
fn valid_eddsa_token_is_accepted() {
    let kp = Keypair::from_seed(&[9u8; 32]);
    let v = verifier_a(keyset(vec![jwk_ed25519(&kp, "e1")]));
    let header = json!({ "alg": "EdDSA", "kid": "e1" });
    let claims = run(&v, sign_ed25519(&kp, header, base_claims(ISS_A))).expect("valid token");
    assert_eq!(claims.identity, ident(ISS_A, "user-123"));
}

// ── 3: end-to-end through establish ─────────────────────────────────────────

#[test]
fn establish_end_to_end_bound_and_unbound() {
    let sk = es_key(7);
    let v = verifier_a(keyset(vec![jwk_es256(&sk, "a1")]));
    let token = sign_es256(&sk, es_header("a1"), base_claims(ISS_A));
    let inp = input(token);

    let mut store = LocalUserStore::new();
    store.insert(user(1, "alice"));
    store
        .bind_external(UserId(1), ident(ISS_A, "user-123"))
        .unwrap();
    let login = establish(&v, &trusted_both(), &inp, &store).expect("login");
    assert_eq!(login.identity(), &ident(ISS_A, "user-123"));
    assert_eq!(login.authenticated_user().user.id, UserId(1));
    assert_eq!(login.authenticated_user().auth.provider, PROV_A);

    let mut empty = LocalUserStore::new();
    empty.insert(user(1, "alice"));
    let r = establish(&v, &trusted_both(), &inp, &empty);
    assert!(matches!(r, Err(LoginError::Unbound)), "got {r:?}");
}

// ── 4: algorithm confusion ──────────────────────────────────────────────────

#[test]
fn alg_none_hs256_and_crit_are_unsupported() {
    let sk = es_key(7);
    let v = verifier_a(keyset(vec![jwk_es256(&sk, "a1")]));
    let claims = base_claims(ISS_A);

    let none_header = json!({ "alg": "none", "kid": "a1" });
    let unsigned = format!("{}.", signing_input(&none_header, &claims));
    assert_eq!(run(&v, unsigned), Err(OidcError::UnsupportedAlgorithm));

    let hs_header = json!({ "alg": "HS256", "kid": "a1" });
    let hs = format!("{}.{}", signing_input(&hs_header, &claims), b64(&[1u8; 32]));
    assert_eq!(run(&v, hs), Err(OidcError::UnsupportedAlgorithm));

    let crit_header = json!({ "alg": "ES256", "kid": "a1", "crit": ["exp"] });
    let crit = sign_es256(&sk, crit_header, claims);
    assert_eq!(run(&v, crit), Err(OidcError::UnsupportedAlgorithm));
}

// ── 5: cross-issuer key confusion ───────────────────────────────────────────

#[test]
fn key_of_one_issuer_never_verifies_another() {
    let key_a = es_key(7);
    let key_b = es_key(8);
    let mut v = OidcVerifier::new();
    v.add_client(Client::new(
        issuer(ISS_A),
        CLIENT,
        keyset(vec![jwk_es256(&key_a, "a1")]),
    ));
    v.add_client(Client::new(
        issuer(ISS_B),
        CLIENT,
        keyset(vec![jwk_es256(&key_b, "b1")]),
    ));

    // Signed by A, kid of A, but naming issuer B: B has no kid "a1".
    let token = sign_es256(&key_a, es_header("a1"), base_claims(ISS_B));
    assert_eq!(run(&v, token), Err(OidcError::UnknownKey));

    // Same signer, kid of B: B's key is found but the signature is A's.
    let token = sign_es256(&key_a, es_header("b1"), base_claims(ISS_B));
    assert_eq!(run(&v, token), Err(OidcError::BadSignature));

    // Control: B signing for B verifies.
    let token = sign_es256(&key_b, es_header("b1"), base_claims(ISS_B));
    assert!(run(&v, token).is_ok());
}

// ── 6: tampering ────────────────────────────────────────────────────────────

#[test]
fn tampered_payload_fails_signature() {
    let sk = es_key(7);
    let v = verifier_a(keyset(vec![jwk_es256(&sk, "a1")]));
    let token = sign_es256(&sk, es_header("a1"), base_claims(ISS_A));
    let mut parts: Vec<&str> = token.split('.').collect();
    let mut forged = base_claims(ISS_A);
    forged["sub"] = json!("attacker");
    let forged_payload = b64(forged.to_string().as_bytes());
    parts[1] = &forged_payload;
    assert_eq!(run(&v, parts.join(".")), Err(OidcError::BadSignature));
}

// ── 7: issuer matching and trust ────────────────────────────────────────────

#[test]
fn issuer_is_byte_exact_and_must_be_trusted() {
    let sk = es_key(7);
    let v = verifier_a(keyset(vec![jwk_es256(&sk, "a1")]));

    let slash = sign_es256(&sk, es_header("a1"), base_claims("https://idp-a.example/"));
    assert_eq!(run(&v, slash), Err(OidcError::UnknownIssuer));

    let mut only_b = TrustedIssuers::new();
    only_b.insert(issuer(ISS_B), PROV_B).unwrap();
    let token = sign_es256(&sk, es_header("a1"), base_claims(ISS_A));
    assert_eq!(
        v.verify(&only_b, &input(token)),
        Err(OidcError::UntrustedIssuer)
    );
}

// ── 8: time ─────────────────────────────────────────────────────────────────

#[test]
fn time_claims_and_leeway_boundaries() {
    let with = |name: &str, value: u64| {
        let mut c = base_claims(ISS_A);
        c[name] = json!(value);
        c
    };
    // leeway is 60: exp + 60 <= now is expired.
    assert_eq!(es_run(with("exp", NOW - 61)), Err(OidcError::Expired));
    assert!(es_run(with("exp", NOW - 59)).is_ok());
    assert_eq!(es_run(with("iat", NOW + 61)), Err(OidcError::NotYetValid));
    assert!(es_run(with("iat", NOW + 59)).is_ok());
    assert_eq!(es_run(with("nbf", NOW + 61)), Err(OidcError::NotYetValid));
    assert!(es_run(with("nbf", NOW + 59)).is_ok());
    assert_eq!(
        es_run(drop_claim(base_claims(ISS_A), "exp")),
        Err(OidcError::MissingClaim)
    );
}

// ── 9: audience ─────────────────────────────────────────────────────────────

#[test]
fn audience_and_authorized_party_rules() {
    let mut other = base_claims(ISS_A);
    other["aud"] = json!("client-b");
    assert_eq!(es_run(other), Err(OidcError::AudienceMismatch));

    let mut multi = base_claims(ISS_A);
    multi["aud"] = json!(["client-a", "client-b"]);
    assert_eq!(es_run(multi.clone()), Err(OidcError::AudienceMismatch));
    multi["azp"] = json!("client-a");
    assert!(es_run(multi).is_ok());

    let mut wrong_azp = base_claims(ISS_A);
    wrong_azp["azp"] = json!("client-b");
    assert_eq!(es_run(wrong_azp), Err(OidcError::AudienceMismatch));
}

// ── 10: nonce ───────────────────────────────────────────────────────────────

#[test]
fn nonce_must_match_when_expected() {
    let sk = es_key(7);
    let v = verifier_a(keyset(vec![jwk_es256(&sk, "a1")]));
    let with_nonce = |claim: Option<&str>| {
        let mut c = base_claims(ISS_A);
        if let Some(n) = claim {
            c["nonce"] = json!(n);
        }
        let token = sign_es256(&sk, es_header("a1"), c);
        IdToken {
            token,
            now_unix: NOW,
            nonce: Some("n1".to_string()),
        }
    };
    assert!(v.verify(&trusted_both(), &with_nonce(Some("n1"))).is_ok());
    assert_eq!(
        v.verify(&trusted_both(), &with_nonce(Some("n2"))),
        Err(OidcError::NonceMismatch)
    );
    assert_eq!(
        v.verify(&trusted_both(), &with_nonce(None)),
        Err(OidcError::NonceMismatch)
    );
}

// ── 11: strength ────────────────────────────────────────────────────────────

#[test]
fn amr_decides_strength() {
    let with_amr = |amr: Value| {
        let mut c = base_claims(ISS_A);
        c["amr"] = amr;
        es_run(c).expect("valid").strength
    };
    assert_eq!(with_amr(json!(["pwd", "otp"])), AuthStrength::MultiFactor);
    assert_eq!(with_amr(json!(["mfa"])), AuthStrength::MultiFactor);
    assert_eq!(with_amr(json!(["pwd"])), AuthStrength::SingleFactor);
    // Hardware key with user presence: one factor (RFC 8176: `user` is presence).
    assert_eq!(with_amr(json!(["hwk", "user"])), AuthStrength::SingleFactor);
    // Two methods of one category (possession) are still one factor.
    assert_eq!(with_amr(json!(["otp", "sms"])), AuthStrength::SingleFactor);
    // Possession + inherence (e.g. a passkey unlocked by fingerprint).
    assert_eq!(with_amr(json!(["hwk", "fpt"])), AuthStrength::MultiFactor);
}

// ── 12: email ───────────────────────────────────────────────────────────────

#[test]
fn email_is_returned_only_when_verified() {
    let email = |verified: Option<bool>| {
        let mut c = base_claims(ISS_A);
        c["email"] = json!("a@b.example");
        if let Some(v) = verified {
            c["email_verified"] = json!(v);
        }
        es_run(c).expect("valid").email
    };
    assert_eq!(email(Some(true)), Some("a@b.example".to_string()));
    assert_eq!(email(Some(false)), None);
    assert_eq!(email(None), None);
}

// ── 13: kid handling ────────────────────────────────────────────────────────

#[test]
fn kid_selection_rules() {
    let k1 = es_key(7);
    let k2 = es_key(8);
    let no_kid_header = json!({ "alg": "ES256" });

    // Two keys, token without kid: ambiguous.
    let v = verifier_a(keyset(vec![jwk_es256(&k1, "a1"), jwk_es256(&k2, "a2")]));
    let token = sign_es256(&k1, no_kid_header.clone(), base_claims(ISS_A));
    assert_eq!(run(&v, token), Err(OidcError::UnknownKey));

    // One unkeyed key, token without kid: accepted.
    let v = verifier_a(keyset(vec![without(jwk_es256(&k1, "x"), "kid")]));
    let token = sign_es256(&k1, no_kid_header, base_claims(ISS_A));
    assert!(run(&v, token).is_ok());

    // kid points at an Ed25519 key but the header says ES256.
    let kp = Keypair::from_seed(&[9u8; 32]);
    let v = verifier_a(keyset(vec![jwk_ed25519(&kp, "e1")]));
    let token = sign_es256(&k1, es_header("e1"), base_claims(ISS_A));
    assert_eq!(run(&v, token), Err(OidcError::UnknownKey));
}

// ── 14: JWKS parsing ────────────────────────────────────────────────────────

#[test]
fn jwks_parsing_skips_and_rejects() {
    let sk = es_key(7);
    let ec = jwk_es256(&sk, "k1");
    let rsa = json!({ "kty": "RSA", "n": "AQAB", "e": "AQAB", "kid": "r1" });
    let parse =
        |keys: Vec<Value>| KeySet::from_jwks_json(json!({ "keys": keys }).to_string().as_bytes());

    let set = parse(vec![rsa, ec.clone()]).expect("mixed");
    assert_eq!(set.len(), 1);
    assert_eq!(set.skipped(), 1);

    let mut enc = jwk_es256(&es_key(8), "k2");
    enc["use"] = json!("enc");
    let set = parse(vec![ec.clone(), enc]).expect("enc skipped");
    assert_eq!(set.len(), 1);
    assert_eq!(set.skipped(), 1);

    // key_ops restricts usage (RFC 7517 §4.3): without "verify" it is skipped.
    let mut derive = jwk_es256(&es_key(8), "k3");
    derive.as_object_mut().expect("object").remove("use");
    derive["key_ops"] = json!(["deriveKey"]);
    let set = parse(vec![ec.clone(), derive]).expect("deriveKey skipped");
    assert_eq!(set.len(), 1);
    assert_eq!(set.skipped(), 1);
    let mut verify = jwk_es256(&es_key(8), "k4");
    verify["key_ops"] = json!(["verify"]);
    let set = parse(vec![ec.clone(), verify]).expect("verify kept");
    assert_eq!(set.len(), 2);
    assert_eq!(set.skipped(), 0);

    let dup = jwk_es256(&es_key(8), "k1");
    assert!(matches!(
        parse(vec![ec, dup]),
        Err(KeySetError::DuplicateKid)
    ));

    let zero = b64(&[0u8; 32]);
    let bad_point = json!({ "kty": "EC", "crv": "P-256", "kid": "z", "x": zero, "y": zero });
    assert!(matches!(
        parse(vec![bad_point]),
        Err(KeySetError::InvalidKey)
    ));

    assert!(matches!(
        KeySet::from_jwks_json(b"{}"),
        Err(KeySetError::Malformed)
    ));
    assert!(matches!(
        KeySet::from_jwks_json(b"not json"),
        Err(KeySetError::Malformed)
    ));
}

// ── 15: discovery ───────────────────────────────────────────────────────────

#[test]
fn discovery_checks_issuer_and_jwks_scheme() {
    let expected = issuer(ISS_A);
    let doc = |iss: &str, uri: &str| {
        json!({ "issuer": iss, "jwks_uri": uri })
            .to_string()
            .into_bytes()
    };

    let d = Discovery::from_json(&expected, &doc(ISS_A, "https://idp-a.example/jwks"))
        .expect("valid discovery");
    assert_eq!(d.issuer, expected);
    assert_eq!(d.jwks_uri, "https://idp-a.example/jwks");

    assert_eq!(
        Discovery::from_json(
            &expected,
            &doc("https://idp-a.example/", "https://idp-a.example/jwks")
        ),
        Err(DiscoveryError::IssuerMismatch)
    );
    assert_eq!(
        Discovery::from_json(&expected, &doc(ISS_A, "http://idp-a.example/jwks")),
        Err(DiscoveryError::InsecureJwksUri)
    );
    assert_eq!(
        Discovery::from_json(&expected, b"{}"),
        Err(DiscoveryError::Malformed)
    );
}

// ── 16: errors never leak the subject or the token ──────────────────────────

#[test]
fn error_display_never_leaks_subject_or_token() {
    let sk = es_key(7);
    let v = verifier_a(keyset(vec![jwk_es256(&sk, "a1")]));
    let mut bad_aud = base_claims(ISS_A);
    bad_aud["aud"] = json!("client-b");
    let mut expired = base_claims(ISS_A);
    expired["exp"] = json!(NOW - 100);

    let tokens = vec![
        sign_es256(&sk, es_header("a1"), bad_aud),
        sign_es256(&sk, es_header("a1"), expired),
        sign_es256(&sk, es_header("zz"), base_claims(ISS_A)),
        sign_es256(&es_key(8), es_header("a1"), base_claims(ISS_A)),
        sign_es256(&sk, es_header("a1"), base_claims("https://other.example")),
        format!(
            "{}.",
            signing_input(&json!({ "alg": "none" }), &base_claims(ISS_A))
        ),
        "garbage".to_string(),
    ];
    for token in tokens {
        let err = run(&v, token.clone()).expect_err("must fail");
        let shown = format!("{err} / {err:?}");
        assert!(!shown.contains("user-123"), "leaked subject: {shown}");
        assert!(!shown.contains(&token), "leaked token: {shown}");
    }
}
