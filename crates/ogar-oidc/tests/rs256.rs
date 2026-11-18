//! RS256 tests (feature `rs256`): the public surface only. RSA test keys are
//! generated once per test binary; every failure asserts the exact variant.
#![cfg(feature = "rs256")]

use std::sync::OnceLock;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ogar_auth::federation::{AssertionVerifier, TrustedIssuers, VerifiedClaims, establish};
use ogar_auth::identity::{ExternalIdentity, ExternalSubject, Issuer};
use ogar_auth::user::{KeyRef, LocalUserStore, ProviderId, User, UserId};
use ogar_encryption::sign::Keypair;
use ogar_oidc::{Client, IdToken, KeySet, KeySetError, OidcError, OidcVerifier};
use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::Signer;
use rsa::sha2::Sha256;
use rsa::traits::PublicKeyParts;
use serde_json::{Value, json};

const NOW: u64 = 1_800_000_000;
const ISS_A: &str = "https://idp-a.example";
const CLIENT: &str = "client-a";
const PROV_A: ProviderId = ProviderId("idp-a");

// ── RSA keys, generated once per test binary ────────────────────────────────

fn rsa_2048_a() -> &'static rsa::PrivateKey {
    static K: OnceLock<rsa::PrivateKey> = OnceLock::new();
    K.get_or_init(|| rsa::PrivateKey::new(&mut rand::rng(), 2048).expect("2048-bit key"))
}

fn rsa_2048_b() -> &'static rsa::PrivateKey {
    static K: OnceLock<rsa::PrivateKey> = OnceLock::new();
    K.get_or_init(|| rsa::PrivateKey::new(&mut rand::rng(), 2048).expect("2048-bit key"))
}

fn rsa_1024() -> &'static rsa::PrivateKey {
    static K: OnceLock<rsa::PrivateKey> = OnceLock::new();
    K.get_or_init(|| rsa::PrivateKey::new(&mut rand::rng(), 1024).expect("1024-bit key"))
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn minimal_be(bytes: &[u8]) -> &[u8] {
    let first = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
    &bytes[first..]
}

fn jwk_rsa(key: &rsa::PrivateKey, kid: &str) -> Value {
    json!({
        "kty": "RSA",
        "kid": kid,
        "use": "sig",
        "n": b64(minimal_be(&key.n().to_be_bytes())),
        "e": b64(minimal_be(&key.e().to_be_bytes())),
    })
}

fn jwk_es256(sk: &SigningKey, kid: &str) -> Value {
    let sec1 = sk.verifying_key().to_sec1_bytes();
    json!({
        "kty": "EC", "crv": "P-256", "kid": kid, "use": "sig",
        "x": b64(&sec1[1..33]), "y": b64(&sec1[33..]),
    })
}

fn jwk_ed25519(kp: &Keypair, kid: &str) -> Value {
    json!({ "kty": "OKP", "crv": "Ed25519", "kid": kid, "use": "sig", "x": b64(&kp.public_key()) })
}

fn parse(keys: Vec<Value>) -> Result<KeySet, KeySetError> {
    KeySet::from_jwks_json(json!({ "keys": keys }).to_string().as_bytes())
}

fn keyset(keys: Vec<Value>) -> KeySet {
    parse(keys).expect("keyset")
}

fn signing_input(header: &Value, claims: &Value) -> String {
    format!(
        "{}.{}",
        b64(header.to_string().as_bytes()),
        b64(claims.to_string().as_bytes())
    )
}

fn sign_rs256_with(key: &rsa::PrivateKey, header: Value, claims: Value) -> String {
    let input = signing_input(&header, &claims);
    let sig = key.pkcs1v15_sign::<Sha256>(input.as_bytes()).expect("sign");
    format!("{input}.{}", b64(&sig))
}

fn rs_header(kid: &str) -> Value {
    json!({ "alg": "RS256", "typ": "JWT", "kid": kid })
}

fn base_claims() -> Value {
    json!({
        "iss": ISS_A, "sub": "user-123", "aud": CLIENT,
        "iat": NOW, "exp": NOW + 300,
    })
}

fn issuer(s: &str) -> Issuer {
    Issuer::new(s).expect("issuer")
}

fn trusted() -> TrustedIssuers {
    let mut t = TrustedIssuers::new();
    t.insert(issuer(ISS_A), PROV_A).unwrap();
    t
}

fn verifier(keys: KeySet, opted_in: bool) -> OidcVerifier {
    let client = Client::new(issuer(ISS_A), CLIENT, keys);
    let client = if opted_in {
        client.allow_rs256()
    } else {
        client
    };
    let mut v = OidcVerifier::new();
    v.add_client(client);
    v
}

fn input(token: String) -> IdToken {
    IdToken {
        token,
        now_unix: NOW,
        nonce: None,
    }
}

fn run(v: &OidcVerifier, token: String) -> Result<VerifiedClaims, OidcError> {
    v.verify(&trusted(), &input(token))
}

/// Opted-in verifier holding the 2048-bit key A as "r1".
fn rs_verifier() -> OidcVerifier {
    verifier(keyset(vec![jwk_rsa(rsa_2048_a(), "r1")]), true)
}

fn rs_token(claims: Value) -> String {
    sign_rs256_with(rsa_2048_a(), rs_header("r1"), claims)
}

// ── 1: valid token, through establish ───────────────────────────────────────

#[test]
fn valid_rs256_token_verifies_through_establish() {
    let v = rs_verifier();
    let inp = input(rs_token(base_claims()));
    let claims = v.verify(&trusted(), &inp).expect("valid RS256 token");
    let expected = ExternalIdentity::new(issuer(ISS_A), ExternalSubject::new("user-123").unwrap());
    assert_eq!(claims.identity, expected);

    let mut store = LocalUserStore::new();
    store.insert(User {
        id: UserId(1),
        subject: "alice".to_string(),
        tenant: 7,
        roles: vec!["physician".to_string()],
        memberships: vec![],
        bindings: vec![],
        key_refs: vec![KeyRef("kms://user/1".to_string())],
    });
    store.bind_external(UserId(1), expected.clone()).unwrap();
    let login = establish(&v, &trusted(), &inp, &store).expect("login");
    assert_eq!(login.identity(), &expected);
    assert_eq!(login.authenticated_user().user.id, UserId(1));
    assert_eq!(login.authenticated_user().auth.provider, PROV_A);
}

// ── 2: opt-in is required ───────────────────────────────────────────────────

#[test]
fn rs256_without_opt_in_is_unsupported() {
    let v = verifier(keyset(vec![jwk_rsa(rsa_2048_a(), "r1")]), false);
    assert_eq!(
        run(&v, rs_token(base_claims())),
        Err(OidcError::UnsupportedAlgorithm)
    );
}

// ── 3: wrong signer ────────────────────────────────────────────────────────

#[test]
fn token_signed_by_another_rsa_key_is_bad_signature() {
    let v = rs_verifier();
    let token = sign_rs256_with(rsa_2048_b(), rs_header("r1"), base_claims());
    assert_eq!(run(&v, token), Err(OidcError::BadSignature));
}

// ── 4: weak key is skipped ──────────────────────────────────────────────────

#[test]
fn rsa_1024_is_skipped_and_never_verifies() {
    let set = keyset(vec![jwk_rsa(rsa_1024(), "weak")]);
    assert_eq!(set.len(), 0);
    assert_eq!(set.skipped(), 1);

    let mixed = keyset(vec![
        jwk_rsa(rsa_2048_a(), "r1"),
        jwk_rsa(rsa_1024(), "weak"),
    ]);
    assert_eq!(mixed.len(), 1);
    assert_eq!(mixed.skipped(), 1);

    let v = verifier(mixed, true);
    let token = sign_rs256_with(rsa_1024(), rs_header("weak"), base_claims());
    assert_eq!(run(&v, token), Err(OidcError::UnknownKey));
}

#[test]
fn modulus_bounds_are_exact() {
    let e = b64(&[1, 0, 1]);
    let with_n = |n: Vec<u8>| {
        parse(vec![
            json!({ "kty": "RSA", "kid": "k", "n": b64(&n), "e": e }),
        ])
    };

    // 2048 bits: accepted. 2047 bits: skipped.
    let mut n2048 = vec![0xFFu8; 256];
    let set = with_n(n2048.clone()).expect("parse");
    assert_eq!((set.len(), set.skipped()), (1, 0));
    n2048[0] = 0x7F;
    let set = with_n(n2048).expect("parse");
    assert_eq!((set.len(), set.skipped()), (0, 1));

    // 8192 bits: accepted. 8193 bits: skipped. Far oversized: skipped.
    let set = with_n(vec![0xFFu8; 1024]).expect("parse");
    assert_eq!((set.len(), set.skipped()), (1, 0));
    let mut n8193 = vec![0u8; 1025];
    n8193[0] = 0x01;
    n8193[1024] = 0x01;
    let set = with_n(n8193).expect("parse");
    assert_eq!((set.len(), set.skipped()), (0, 1));
    let set = with_n(vec![0xFFu8; 100_000]).expect("parse");
    assert_eq!((set.len(), set.skipped()), (0, 1));

    // Even modulus: skipped.
    let mut even = vec![0xFFu8; 256];
    even[255] = 0xFE;
    let set = with_n(even).expect("parse");
    assert_eq!((set.len(), set.skipped()), (0, 1));
}

// ── 5 and 6: algorithm confusion ────────────────────────────────────────────

#[test]
fn es256_header_with_rsa_kid_is_unknown_key() {
    let v = rs_verifier();
    let sk = SigningKey::from_slice(&[7u8; 32]).unwrap();
    let header = json!({ "alg": "ES256", "kid": "r1" });
    let input = signing_input(&header, &base_claims());
    let sig: p256::ecdsa::Signature = sk.sign(input.as_bytes());
    let token = format!("{input}.{}", b64(&sig.to_bytes()));
    assert_eq!(run(&v, token), Err(OidcError::UnknownKey));

    // Same for EdDSA naming the RSA key.
    let kp = Keypair::from_seed(&[9u8; 32]);
    let input = signing_input(&json!({ "alg": "EdDSA", "kid": "r1" }), &base_claims());
    let token = format!("{input}.{}", b64(&kp.sign(input.as_bytes())));
    assert_eq!(run(&v, token), Err(OidcError::UnknownKey));
}

#[test]
fn rs256_header_with_non_rsa_kid_is_unknown_key() {
    let sk = SigningKey::from_slice(&[7u8; 32]).unwrap();
    let kp = Keypair::from_seed(&[9u8; 32]);
    let v = verifier(
        keyset(vec![jwk_es256(&sk, "ec1"), jwk_ed25519(&kp, "ed1")]),
        true,
    );
    for kid in ["ec1", "ed1"] {
        let token = sign_rs256_with(rsa_2048_a(), rs_header(kid), base_claims());
        assert_eq!(run(&v, token), Err(OidcError::UnknownKey), "kid {kid}");
    }
}

// ── 7: other RSA algorithms ─────────────────────────────────────────────────

#[test]
fn other_rsa_algorithms_are_skipped_and_refused() {
    for alg in ["PS256", "RS512", "RS384"] {
        let mut jwk = jwk_rsa(rsa_2048_a(), "r1");
        jwk["alg"] = json!(alg);
        let set = keyset(vec![jwk]);
        assert_eq!((set.len(), set.skipped()), (0, 1), "jwk alg {alg}");

        let v = rs_verifier();
        let header = json!({ "alg": alg, "kid": "r1" });
        let token = sign_rs256_with(rsa_2048_a(), header, base_claims());
        assert_eq!(
            run(&v, token),
            Err(OidcError::UnsupportedAlgorithm),
            "header alg {alg}"
        );
    }
    // A JWK that states alg RS256 is kept.
    let mut jwk = jwk_rsa(rsa_2048_a(), "r1");
    jwk["alg"] = json!("RS256");
    assert_eq!(keyset(vec![jwk]).len(), 1);
}

// ── 8: exponent and encoding rules ──────────────────────────────────────────

#[test]
fn bad_exponents_are_skipped_and_bad_base64_is_invalid() {
    let n = jwk_rsa(rsa_2048_a(), "k")["n"].clone();
    let with_e = |e: Vec<u8>| {
        parse(vec![
            json!({ "kty": "RSA", "kid": "k", "n": n, "e": b64(&e) }),
        ])
    };

    for e in [vec![1u8], vec![4], vec![0], vec![1, 0, 0], vec![0xFF; 9]] {
        let set = with_e(e.clone()).expect("parse");
        assert_eq!((set.len(), set.skipped()), (0, 1), "e = {e:?}");
    }
    // e = 3 and e = 65537 are fine.
    for e in [vec![3u8], vec![1, 0, 1]] {
        let set = with_e(e.clone()).expect("parse");
        assert_eq!((set.len(), set.skipped()), (1, 0), "e = {e:?}");
    }

    let bad_n = json!({ "kty": "RSA", "kid": "k", "n": "***", "e": "AQAB" });
    assert_eq!(parse(vec![bad_n]).unwrap_err(), KeySetError::InvalidKey);
    let bad_e = json!({ "kty": "RSA", "kid": "k", "n": n, "e": "***" });
    assert_eq!(parse(vec![bad_e]).unwrap_err(), KeySetError::InvalidKey);
    let no_e = json!({ "kty": "RSA", "kid": "k", "n": n });
    assert_eq!(parse(vec![no_e]).unwrap_err(), KeySetError::InvalidKey);
}

// ── 9: signature length ─────────────────────────────────────────────────────

#[test]
fn truncated_and_extended_signatures_are_bad_signature() {
    let v = rs_verifier();
    let token = rs_token(base_claims());
    let (head, sig_b64) = token.rsplit_once('.').unwrap();
    let sig = URL_SAFE_NO_PAD.decode(sig_b64).unwrap();
    assert_eq!(sig.len(), 256);

    let mut short = sig.clone();
    short.pop();
    assert_eq!(
        run(&v, format!("{head}.{}", b64(&short))),
        Err(OidcError::BadSignature)
    );
    let mut long = sig.clone();
    long.push(0);
    assert_eq!(
        run(&v, format!("{head}.{}", b64(&long))),
        Err(OidcError::BadSignature)
    );
    assert_eq!(
        run(&v, format!("{head}.")),
        Err(OidcError::BadSignature),
        "empty signature"
    );
    // Control: the untouched token verifies.
    assert!(run(&v, token).is_ok());
}

// ── 10: mixed JWKS ──────────────────────────────────────────────────────────

#[test]
fn mixed_jwks_keeps_es256_and_eddsa_working() {
    let sk = SigningKey::from_slice(&[7u8; 32]).unwrap();
    let kp = Keypair::from_seed(&[9u8; 32]);
    let set = keyset(vec![
        jwk_es256(&sk, "ec1"),
        jwk_ed25519(&kp, "ed1"),
        jwk_rsa(rsa_2048_a(), "r1"),
    ]);
    assert_eq!((set.len(), set.skipped()), (3, 0));
    let v = verifier(set, true);

    let input_es = signing_input(&json!({ "alg": "ES256", "kid": "ec1" }), &base_claims());
    let sig: p256::ecdsa::Signature = sk.sign(input_es.as_bytes());
    assert!(run(&v, format!("{input_es}.{}", b64(&sig.to_bytes()))).is_ok());

    let input_ed = signing_input(&json!({ "alg": "EdDSA", "kid": "ed1" }), &base_claims());
    assert!(
        run(
            &v,
            format!("{input_ed}.{}", b64(&kp.sign(input_ed.as_bytes())))
        )
        .is_ok()
    );

    assert!(run(&v, rs_token(base_claims())).is_ok());
}

// ── 11: claim checks still apply ────────────────────────────────────────────

#[test]
fn claim_checks_apply_to_rs256_tokens() {
    let v = rs_verifier();

    let mut wrong_aud = base_claims();
    wrong_aud["aud"] = json!("client-b");
    assert_eq!(
        run(&v, rs_token(wrong_aud)),
        Err(OidcError::AudienceMismatch)
    );

    let mut expired = base_claims();
    expired["exp"] = json!(NOW - 100);
    assert_eq!(run(&v, rs_token(expired)), Err(OidcError::Expired));

    let mut future = base_claims();
    future["iat"] = json!(NOW + 100);
    assert_eq!(run(&v, rs_token(future)), Err(OidcError::NotYetValid));

    let mut with_nonce = base_claims();
    with_nonce["nonce"] = json!("n2");
    let inp = IdToken {
        token: rs_token(with_nonce),
        now_unix: NOW,
        nonce: Some("n1".to_string()),
    };
    assert_eq!(v.verify(&trusted(), &inp), Err(OidcError::NonceMismatch));

    // A tampered payload under an RS256 signature fails the signature check.
    let token = rs_token(base_claims());
    let mut parts: Vec<&str> = token.split('.').collect();
    let mut forged = base_claims();
    forged["sub"] = json!("attacker");
    let forged_payload = b64(forged.to_string().as_bytes());
    parts[1] = &forged_payload;
    assert_eq!(run(&v, parts.join(".")), Err(OidcError::BadSignature));
}
