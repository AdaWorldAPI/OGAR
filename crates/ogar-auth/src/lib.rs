//! # ogar-auth — the OGAR auth arm
//!
//! One authentication SDK every OGAR consumer pulls, so the auth primitives
//! never diverge into per-consumer copies. Before this crate, each consumer
//! was on its way to hand-rolling its own TOTP, its own Argon2 wrapper, its
//! own legacy-cipher transition — the dilution this crate exists to prevent.
//!
//! ## What lives here
//!
//! | Module | Primitive | Role |
//! |---|---|---|
//! | [`password`] | Argon2id (PHC string) | forward login-credential hash + verify |
//! | [`totp`] | RFC 6238 (HMAC-SHA1) | second factor at the token-mint point |
//! | [`legacy`] | 3DES-EDE2 / PBKDF1-MD5 | transition-verify legacy ciphertext during migration |
//! | [`kdf`] (re-export) | Argon2id → raw key | envelope key derivation |
//! | [`aead`] (re-export) | XChaCha20-Poly1305 | authenticated encryption |
//! | [`hash`] (re-export) | SHA-384 | merkle / fingerprint hashing |
//! | [`sign`] (re-export) | Ed25519 | licence / audit signatures |
//! | [`envelope`] (re-export) | seal / open | wasm-capable zero-knowledge envelope |
//!
//! The forward suite (`kdf` / `aead` / `hash` / `sign` / `envelope`) is
//! **re-exported via [`ogar-encryption`](https://github.com/AdaWorldAPI/OGAR),
//! never re-implemented**. `ogar-encryption` is itself a thin, classid-agnostic
//! re-export of the ndarray `encryption` crate — the audited, wasm-capable
//! home of XChaCha20-Poly1305 + Argon2id + Ed25519 + SHA-384; duplicating its
//! byte-exact envelope layout here would be exactly the dilution ogar-auth
//! prevents. The three modules this crate *adds* (`password`, `totp`,
//! `legacy`) are the auth-specific primitives neither `ogar-encryption` nor
//! the encryption crate it wraps deliberately carries.
//!
//! ## Agnostic by construction — no consumer secrets
//!
//! Every symbol here is data-free. [`legacy::decrypt_prefixed`] takes the
//! password table as a `&[&str]` argument; the caller (e.g. a private consumer
//! crate) owns the table. [`totp::provisioning_uri`] takes the issuer and
//! account as arguments. Nothing in this public crate names a consumer, a
//! tenant, a table, or a key.
//!
//! ## Endgame seam — OGIT[auth] IAM federation
//!
//! Today ogar-auth is the *local* authentication substrate: passwords, second
//! factor, legacy migration, envelopes. The next arm is **federated identity**
//! — OGAR ↔ `OGIT[auth]` bound to an external IdP (OIDC / Zitadel / any
//! standards IAM). That surface is deliberately NOT built yet; it lands as a
//! sibling adapter (cf. the `ogar-adapter-*` family) that maps an OIDC
//! `IdToken` / introspection response onto the same identity envelope the
//! local path produces, so a consumer's authorization code is IdP-agnostic.
//! See [`federation`] for the reserved shape and the one invariant that seam
//! must honour.

pub mod legacy;
pub mod password;
pub mod totp;
pub mod user;

// ── Forward suite: re-exported via `ogar-encryption` ────────────────────────
// Reused wholesale, never re-implemented (see crate docs). A consumer that
// pulls `ogar-auth` gets the entire forward crypto surface under one import.
pub use ogar_encryption::{aead, envelope, hash, kdf, sign};

/// Unified error for the auth-specific primitives this crate adds
/// (`password`, `totp`, `legacy`). Field-free on the secret-bearing paths so
/// no key or plaintext ever lands in an error string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// A base32 / base64 secret or ciphertext was not valid for its encoding.
    Encoding(&'static str),
    /// Legacy ciphertext structural failure (block alignment, padding, prefix).
    Legacy(&'static str),
    /// Argon2id hashing or verification failed (parameters, format, or the
    /// stored hash was malformed). Carries no password material.
    Password(&'static str),
}

impl core::fmt::Display for AuthError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Encoding(m) => write!(f, "encoding error: {m}"),
            Self::Legacy(m) => write!(f, "legacy cipher error: {m}"),
            Self::Password(m) => write!(f, "password hash error: {m}"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Result alias for the auth-specific primitives.
pub type AuthResult<T> = Result<T, AuthError>;

/// The reserved federation seam — OGAR ↔ `OGIT[auth]` external-IdP binding.
///
/// This module is intentionally a **contract stub**, not an implementation.
/// It records the one invariant a future OIDC / Zitadel adapter MUST honour so
/// the endgame is legible from the SDK today, and so nobody re-derives it:
///
/// > **A federated login and a local login converge on the SAME identity
/// > envelope before any authorization decision is made.** The IdP adapter's
/// > job ends at producing `(subject, tenant, roles)` — the exact shape the
/// > local password + TOTP path already produces (mirrors
/// > `lance_graph_contract::auth::ActorContext`). Authorization code downstream
/// > is therefore identical whether the actor authenticated locally or through
/// > Zitadel; the IdP is a *source of the envelope*, never a fork in the
/// > authorization logic.
///
/// The adapter itself (token validation, JWKS fetch, claim mapping) lands as a
/// sibling crate under the `ogar-adapter-*` convention when the endgame is
/// scheduled — it is out of scope for the local auth substrate this crate is.
///
/// **The convergence point is now shipped**, as [`crate::user`]: an adapter's
/// job ends at producing an [`AuthBinding`](crate::user::AuthBinding), which
/// [`UserStore::resolve`](crate::user::UserStore::resolve) maps to the canonical
/// [`User`](crate::user::User); the envelope every path converges on is
/// [`AuthenticatedUser::actor_context`](crate::user::AuthenticatedUser::actor_context).
/// This module stays an empty marker: what is still out of scope here is
/// unchanged — token validation, JWKS fetch and claim parsing.
pub mod federation {}
