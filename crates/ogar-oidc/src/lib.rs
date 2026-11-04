//! Provider-agnostic OIDC ID-token verifier for `ogar-auth` (PR-01).
//!
//! [`OidcVerifier`] implements [`ogar_auth::federation::AssertionVerifier`]: it
//! takes a compact JWS ID token and returns [`VerifiedClaims`] that
//! [`ogar_auth::federation::establish`] turns into a `VerifiedLogin`. Nothing in
//! here is specific to one identity provider; ZITADEL, Keycloak or Entra ID are
//! all a [`Client`] registration away.
//!
//! # What this crate does NOT do
//!
//! - **No network I/O.** The caller fetches the discovery document and the JWKS
//!   (and refreshes them on key rotation) and hands the bytes in. That keeps the
//!   verifier deterministic, testable and wasm-clean.
//! - **No authorization.** It establishes *who* signed in, never what they may
//!   do. Roles stay with `ogar-rbac`.
//! - **No account linking.** `email` is returned as a display hint only, and
//!   only when the provider marks it verified.
//!
//! # Checks, in order
//!
//! 1. The token is three base64url segments; the header `alg` is `ES256` or
//!    `EdDSA` (everything else, `none` and `HS*` included, is refused); a header
//!    with `crit` is refused.
//! 2. The payload `iss` is read **unverified** only to pick the [`Client`]; it
//!    must byte-equal a registered issuer and be in [`TrustedIssuers`].
//! 3. The signing key is looked up **only in that issuer's key set**, by `kid`,
//!    and its type must match `alg`. A key registered for another issuer can
//!    never verify a token naming this one.
//! 4. The signature is checked over `header.payload`.
//! 5. Claims: `sub` non-empty; `aud` contains the client id; with several
//!    audiences `azp` is required, and `azp` when present must equal the client
//!    id; `exp` required and in the future; `iat` required and not in the
//!    future; `nbf` if present has passed (all with [`OidcVerifier::leeway_secs`]);
//!    `nonce` equals the expected nonce when one is given.
//!
//! RS256 is not implemented yet: no AdaWorldAPI fork of an RSA crate is wired.
//! A provider must sign ID tokens with a P-256 or Ed25519 key.

use std::collections::BTreeMap;
use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ogar_auth::federation::{AssertionVerifier, TrustedIssuers, VerifiedClaims};
use ogar_auth::identity::{ExternalIdentity, ExternalSubject, Issuer};
use ogar_auth::user::AuthStrength;
use p256::ecdsa::signature::Verifier as _;
use serde_json::Value;

/// One verification key from a JWKS.
#[derive(Clone)]
enum Key {
    Es256(p256::ecdsa::VerifyingKey),
    Ed25519([u8; ogar_encryption::sign::PUBLIC_KEY_LEN]),
}

impl Key {
    fn alg(&self) -> Alg {
        match self {
            Self::Es256(_) => Alg::Es256,
            Self::Ed25519(_) => Alg::EdDsa,
        }
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Es256(_) => "Key::Es256",
            Self::Ed25519(_) => "Key::Ed25519",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Alg {
    Es256,
    EdDsa,
}

/// Why a JWKS document could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySetError {
    /// The document is not JSON or has no `keys` array.
    Malformed,
    /// A supported key (EC P-256 or OKP Ed25519) has an invalid encoding.
    InvalidKey,
    /// Two signing keys share a `kid`.
    DuplicateKid,
}

impl fmt::Display for KeySetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Malformed => "JWKS document is malformed",
            Self::InvalidKey => "JWKS contains an invalid key",
            Self::DuplicateKid => "JWKS contains two signing keys with the same kid",
        })
    }
}

impl std::error::Error for KeySetError {}

/// The signing keys of one issuer, read from its JWKS document.
#[derive(Debug, Clone, Default)]
pub struct KeySet {
    keys: BTreeMap<String, Key>,
    unkeyed: Vec<Key>,
    skipped: usize,
}

impl KeySet {
    /// Parse a JWKS document (`{"keys":[...]}`).
    ///
    /// EC P-256 and OKP Ed25519 signing keys are kept. Keys of other types
    /// (RSA included) and keys whose `use` is not `sig` are skipped and counted
    /// in [`KeySet::skipped`], so a mixed JWKS still loads.
    ///
    /// # Errors
    ///
    /// [`KeySetError`] for a malformed document, an invalid supported key, or a
    /// duplicate `kid`.
    pub fn from_jwks_json(json: &[u8]) -> Result<Self, KeySetError> {
        let doc: Value = serde_json::from_slice(json).map_err(|_| KeySetError::Malformed)?;
        let keys = doc
            .get("keys")
            .and_then(Value::as_array)
            .ok_or(KeySetError::Malformed)?;
        let mut set = Self::default();
        for jwk in keys {
            if jwk
                .get("use")
                .and_then(Value::as_str)
                .is_some_and(|u| u != "sig")
            {
                set.skipped += 1;
                continue;
            }
            let Some(key) = parse_jwk(jwk)? else {
                set.skipped += 1;
                continue;
            };
            match jwk.get("kid").and_then(Value::as_str) {
                Some(kid) => {
                    if set.keys.insert(kid.to_owned(), key).is_some() {
                        return Err(KeySetError::DuplicateKid);
                    }
                }
                None => set.unkeyed.push(key),
            }
        }
        Ok(set)
    }

    /// Number of usable signing keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len() + self.unkeyed.len()
    }

    /// Whether no usable signing key was found.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Keys present in the document but not usable here (unsupported type, or
    /// not a signing key).
    #[must_use]
    pub fn skipped(&self) -> usize {
        self.skipped
    }

    /// The key a token header selects. With a `kid` only that key; without one
    /// only when the set holds exactly one key.
    fn select(&self, kid: Option<&str>) -> Option<&Key> {
        match kid {
            Some(kid) => self.keys.get(kid),
            None if self.len() == 1 => self.keys.values().chain(&self.unkeyed).next(),
            None => None,
        }
    }
}

fn parse_jwk(jwk: &Value) -> Result<Option<Key>, KeySetError> {
    let field = |name: &str| jwk.get(name).and_then(Value::as_str);
    let alg = field("alg");
    match (field("kty"), field("crv")) {
        (Some("EC"), Some("P-256")) => {
            if alg.is_some_and(|a| a != "ES256") {
                return Ok(None);
            }
            let x = b64_fixed::<32>(field("x")).ok_or(KeySetError::InvalidKey)?;
            let y = b64_fixed::<32>(field("y")).ok_or(KeySetError::InvalidKey)?;
            let mut sec1 = [0u8; 65];
            sec1[0] = 0x04;
            sec1[1..33].copy_from_slice(&x);
            sec1[33..].copy_from_slice(&y);
            let key = p256::ecdsa::VerifyingKey::from_sec1_bytes(&sec1)
                .map_err(|_| KeySetError::InvalidKey)?;
            Ok(Some(Key::Es256(key)))
        }
        (Some("OKP"), Some("Ed25519")) => {
            if alg.is_some_and(|a| a != "EdDSA") {
                return Ok(None);
            }
            let x = b64_fixed::<32>(field("x")).ok_or(KeySetError::InvalidKey)?;
            Ok(Some(Key::Ed25519(x)))
        }
        _ => Ok(None),
    }
}

fn b64_fixed<const N: usize>(s: Option<&str>) -> Option<[u8; N]> {
    URL_SAFE_NO_PAD.decode(s?).ok()?.try_into().ok()
}

/// An OIDC client registration: the issuer, this application's client id at
/// that issuer, and the issuer's current signing keys.
#[derive(Debug, Clone)]
pub struct Client {
    issuer: Issuer,
    client_id: String,
    keys: KeySet,
}

impl Client {
    /// Register `client_id` at `issuer` with its JWKS.
    #[must_use]
    pub fn new(issuer: Issuer, client_id: impl Into<String>, keys: KeySet) -> Self {
        Self {
            issuer,
            client_id: client_id.into(),
            keys,
        }
    }

    /// The issuer this registration belongs to.
    #[must_use]
    pub fn issuer(&self) -> &Issuer {
        &self.issuer
    }

    /// Replace the key set (after the caller re-fetched the JWKS on rotation).
    pub fn set_keys(&mut self, keys: KeySet) {
        self.keys = keys;
    }
}

/// The issuer and JWKS location read from an OIDC discovery document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    /// The `issuer` the document states; checked byte-equal to the expected one.
    pub issuer: Issuer,
    /// Where the caller fetches the JWKS from.
    pub jwks_uri: String,
}

/// Why a discovery document was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    /// Not JSON, or `issuer` / `jwks_uri` missing.
    Malformed,
    /// The document names a different issuer than the one it was fetched for.
    IssuerMismatch,
    /// `jwks_uri` is not https.
    InsecureJwksUri,
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Malformed => "discovery document is malformed",
            Self::IssuerMismatch => "discovery document names a different issuer",
            Self::InsecureJwksUri => "discovery jwks_uri is not https",
        })
    }
}

impl std::error::Error for DiscoveryError {}

impl Discovery {
    /// Read `/.well-known/openid-configuration` fetched for `expected`.
    ///
    /// # Errors
    ///
    /// [`DiscoveryError`] when the document is malformed, its `issuer` is not
    /// byte-equal to `expected` (OIDC Discovery §4.3), or `jwks_uri` is not https.
    pub fn from_json(expected: &Issuer, json: &[u8]) -> Result<Self, DiscoveryError> {
        let doc: Value = serde_json::from_slice(json).map_err(|_| DiscoveryError::Malformed)?;
        let issuer = doc
            .get("issuer")
            .and_then(Value::as_str)
            .ok_or(DiscoveryError::Malformed)?;
        if issuer != expected.as_str() {
            return Err(DiscoveryError::IssuerMismatch);
        }
        let jwks_uri = doc
            .get("jwks_uri")
            .and_then(Value::as_str)
            .ok_or(DiscoveryError::Malformed)?;
        if !jwks_uri.starts_with("https://") {
            return Err(DiscoveryError::InsecureJwksUri);
        }
        Ok(Self {
            issuer: expected.clone(),
            jwks_uri: jwks_uri.to_owned(),
        })
    }
}

/// The input to one verification: the raw ID token, the current time and the
/// nonce the login request sent (if any).
#[derive(Debug, Clone)]
pub struct IdToken {
    /// The compact JWS (`header.payload.signature`).
    pub token: String,
    /// Seconds since the Unix epoch, supplied by the caller.
    pub now_unix: u64,
    /// The nonce this login sent; when set, the token must carry the same one.
    pub nonce: Option<String>,
}

/// Why an ID token was refused. `Display` never includes token contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OidcError {
    /// Not three base64url segments, or a segment is not JSON.
    Malformed,
    /// The header `alg` is not ES256 / EdDSA, or the header carries `crit`.
    UnsupportedAlgorithm,
    /// No [`Client`] is registered for the token's `iss`.
    UnknownIssuer,
    /// The issuer has a client but is not in [`TrustedIssuers`].
    UntrustedIssuer,
    /// No key of this issuer matches the header `kid` and `alg`.
    UnknownKey,
    /// The signature does not verify.
    BadSignature,
    /// A required claim is missing or has the wrong type.
    MissingClaim,
    /// `aud` / `azp` do not name this client.
    AudienceMismatch,
    /// `exp` has passed.
    Expired,
    /// `nbf` or `iat` lies in the future.
    NotYetValid,
    /// The nonce does not match the one the login sent.
    NonceMismatch,
}

impl fmt::Display for OidcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Malformed => "ID token is malformed",
            Self::UnsupportedAlgorithm => "ID token uses an unsupported algorithm",
            Self::UnknownIssuer => "ID token issuer has no client registration",
            Self::UntrustedIssuer => "ID token issuer is not trusted",
            Self::UnknownKey => "no signing key of this issuer matches the ID token",
            Self::BadSignature => "ID token signature does not verify",
            Self::MissingClaim => "ID token lacks a required claim",
            Self::AudienceMismatch => "ID token is not addressed to this client",
            Self::Expired => "ID token has expired",
            Self::NotYetValid => "ID token is not valid yet",
            Self::NonceMismatch => "ID token nonce does not match",
        })
    }
}

impl std::error::Error for OidcError {}

/// The OIDC [`AssertionVerifier`]: one [`Client`] per issuer.
#[derive(Debug, Clone)]
pub struct OidcVerifier {
    clients: BTreeMap<Issuer, Client>,
    /// Clock skew tolerated on `exp`, `nbf` and `iat`, in seconds.
    pub leeway_secs: u64,
}

impl Default for OidcVerifier {
    fn default() -> Self {
        Self {
            clients: BTreeMap::new(),
            leeway_secs: 60,
        }
    }
}

impl OidcVerifier {
    /// An empty verifier with 60 s leeway.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a client, replacing an earlier registration for the same issuer.
    pub fn add_client(&mut self, client: Client) {
        self.clients.insert(client.issuer.clone(), client);
    }

    /// The registration for `issuer`, for key rotation.
    pub fn client_mut(&mut self, issuer: &Issuer) -> Option<&mut Client> {
        self.clients.get_mut(issuer)
    }
}

fn decode_json(segment: &str) -> Result<Value, OidcError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|_| OidcError::Malformed)?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| OidcError::Malformed)?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(OidcError::Malformed)
    }
}

fn verify_signature(key: &Key, message: &[u8], signature: &[u8]) -> bool {
    match key {
        Key::Es256(vk) => p256::ecdsa::Signature::from_slice(signature)
            .is_ok_and(|sig| vk.verify(message, &sig).is_ok()),
        Key::Ed25519(pk) => <&[u8; ogar_encryption::sign::SIGNATURE_LEN]>::try_from(signature)
            .is_ok_and(|sig| ogar_encryption::sign::verify(pk, message, sig)),
    }
}

/// `MultiFactor` when the provider says so (`amr` contains `mfa`) or when the
/// `amr` values (RFC 8176) cover at least two independent factor categories:
/// knowledge, possession, inherence. Several methods of one category (e.g.
/// `otp` + `sms`) are one factor, and `user` (presence) or other method labels
/// that are not a factor count for nothing. Otherwise `SingleFactor`.
fn strength(claims: &Value) -> AuthStrength {
    const KNOWLEDGE: [&str; 3] = ["pwd", "pin", "kba"];
    const POSSESSION: [&str; 7] = ["hwk", "swk", "otp", "sms", "tel", "pop", "sc"];
    const INHERENCE: [&str; 5] = ["fpt", "face", "iris", "retina", "vbm"];
    let Some(amr) = claims.get("amr").and_then(Value::as_array) else {
        return AuthStrength::SingleFactor;
    };
    let values: Vec<&str> = amr.iter().filter_map(Value::as_str).collect();
    if values.contains(&"mfa") {
        return AuthStrength::MultiFactor;
    }
    let covers = |category: &[&str]| category.iter().any(|m| values.contains(m));
    let categories = [&KNOWLEDGE[..], &POSSESSION[..], &INHERENCE[..]]
        .into_iter()
        .filter(|c| covers(c))
        .count();
    if categories >= 2 {
        AuthStrength::MultiFactor
    } else {
        AuthStrength::SingleFactor
    }
}

fn check_audience(claims: &Value, client_id: &str) -> Result<(), OidcError> {
    let aud: Vec<&str> = match claims.get("aud") {
        Some(Value::String(s)) => vec![s.as_str()],
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect(),
        _ => return Err(OidcError::MissingClaim),
    };
    if !aud.contains(&client_id) {
        return Err(OidcError::AudienceMismatch);
    }
    match claims.get("azp") {
        Some(Value::String(azp)) if azp == client_id => Ok(()),
        Some(_) => Err(OidcError::AudienceMismatch),
        None if aud.len() > 1 => Err(OidcError::AudienceMismatch),
        None => Ok(()),
    }
}

fn check_time(claims: &Value, now: u64, leeway: u64) -> Result<(), OidcError> {
    let num = |name: &str| {
        claims
            .get(name)
            .map(|v| v.as_u64().ok_or(OidcError::MissingClaim))
    };
    let exp = num("exp").ok_or(OidcError::MissingClaim)??;
    if now >= exp.saturating_add(leeway) {
        return Err(OidcError::Expired);
    }
    let iat = num("iat").ok_or(OidcError::MissingClaim)??;
    if iat > now.saturating_add(leeway) {
        return Err(OidcError::NotYetValid);
    }
    if let Some(nbf) = num("nbf").transpose()?
        && nbf > now.saturating_add(leeway)
    {
        return Err(OidcError::NotYetValid);
    }
    Ok(())
}

impl AssertionVerifier for OidcVerifier {
    type Input = IdToken;
    type Error = OidcError;
    const ID: &'static str = "ogar-oidc/id-token";

    fn verify(
        &self,
        trusted: &TrustedIssuers,
        input: &IdToken,
    ) -> Result<VerifiedClaims, OidcError> {
        let mut parts = input.token.split('.');
        let (Some(h), Some(p), Some(s), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(OidcError::Malformed);
        };
        let header = decode_json(h)?;
        if header.get("crit").is_some() {
            return Err(OidcError::UnsupportedAlgorithm);
        }
        let alg = match header.get("alg").and_then(Value::as_str) {
            Some("ES256") => Alg::Es256,
            Some("EdDSA") => Alg::EdDsa,
            _ => return Err(OidcError::UnsupportedAlgorithm),
        };

        // Unverified read, used only to choose whose keys to try.
        let claims = decode_json(p)?;
        let iss = claims
            .get("iss")
            .and_then(Value::as_str)
            .ok_or(OidcError::MissingClaim)?;
        let issuer = Issuer::new(iss).map_err(|_| OidcError::UnknownIssuer)?;
        let client = self.clients.get(&issuer).ok_or(OidcError::UnknownIssuer)?;
        if trusted.provider_of(&issuer).is_none() {
            return Err(OidcError::UntrustedIssuer);
        }

        let kid = header.get("kid").and_then(Value::as_str);
        let key = client
            .keys
            .select(kid)
            .filter(|k| k.alg() == alg)
            .ok_or(OidcError::UnknownKey)?;
        let signature = URL_SAFE_NO_PAD
            .decode(s)
            .map_err(|_| OidcError::Malformed)?;
        let signed = &input.token[..h.len() + 1 + p.len()];
        if !verify_signature(key, signed.as_bytes(), &signature) {
            return Err(OidcError::BadSignature);
        }

        // From here the claims are authenticated.
        let sub = claims
            .get("sub")
            .and_then(Value::as_str)
            .ok_or(OidcError::MissingClaim)?;
        let subject = ExternalSubject::new(sub).map_err(|_| OidcError::MissingClaim)?;
        check_audience(&claims, &client.client_id)?;
        check_time(&claims, input.now_unix, self.leeway_secs)?;
        if let Some(expected) = &input.nonce
            && claims.get("nonce").and_then(Value::as_str) != Some(expected.as_str())
        {
            return Err(OidcError::NonceMismatch);
        }
        let email = (claims.get("email_verified").and_then(Value::as_bool) == Some(true))
            .then(|| {
                claims
                    .get("email")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .flatten();

        Ok(VerifiedClaims {
            identity: ExternalIdentity::new(issuer, subject),
            strength: strength(&claims),
            email,
        })
    }
}
