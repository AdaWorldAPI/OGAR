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
//! standards IAM). The *boundary* is built: [`identity`] (issuer-scoped
//! `(iss, sub)`), and [`federation`] ([`federation::AssertionVerifier`],
//! [`federation::establish`], the [`federation::VerifiedLogin`] proof). The
//! token-validating adapter itself (JWKS, claim parsing) is NOT built here; it
//! lands as a sibling crate implementing `AssertionVerifier`, so a consumer's
//! authorization code stays IdP-agnostic. See [`federation`] for the invariant
//! and for what the types cannot guarantee.

pub mod federation;
pub mod identity;
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
