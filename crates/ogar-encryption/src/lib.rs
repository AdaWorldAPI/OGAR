//! # ogar-encryption — OGAR's generic encryption surface
//!
//! One small, classid-agnostic crate re-exporting the Ada stack's forward
//! crypto suite, so every OGAR consumer that needs raw encryption imports
//! **exactly one crate** instead of depping the ndarray fork directly or
//! (worse) hand-rolling its own Argon2/XChaCha20/Ed25519 wrapper.
//!
//! ## What lives here
//!
//! [`kdf`] and [`envelope`] are implemented here on `argon2` 0.6 from the
//! `AdaWorldAPI/password-hashes` fork (feature `ndarray-simd`: the
//! compression function runs on `ndarray::simd::U64x8`). The rest of the
//! forward suite is a documented re-export of [`encryption`] (the ndarray
//! fork's wasm-capable crypto module):
//!
//! | Module / item | Primitive | Role |
//! |---|---|---|
//! | [`kdf`] | Argon2id (0.6, local) | password/secret → raw key derivation |
//! | [`aead`] | XChaCha20-Poly1305 | authenticated encryption |
//! | [`hash`] | SHA-384 | merkle / fingerprint hashing |
//! | [`sign`] | Ed25519 | licence / audit signatures |
//! | [`envelope`] | seal / open (local) | zero-knowledge envelope, `ADAC` v1 byte layout |
//! | [`seal`], [`open`] | — | root-level aliases for `envelope::seal` / `envelope::open` |
//! | [`EnvelopeError`], [`KdfParams`] | — | root-level aliases for the envelope's error + parameter types |
//! | [`RngError`] | — | the platform-CSPRNG-unavailable error |
//! | [`wasm`] (feature `wasm`, local) | — | wasm-bindgen bindings for browser consumers |
//!
//! ## Generic, classid-agnostic, no secrets — by construction
//!
//! This crate carries **no consumer specifics**: no classid, no tenant, no
//! key material, no wire DTO. [`kdf`] and [`envelope`] keep the API of
//! [`encryption`]'s modules of the same name; everything else is re-exported
//! unmodified.
//!
//! ## Who builds on this
//!
//! - [`ogar-auth`](https://github.com/AdaWorldAPI/OGAR) depends on
//!   `ogar-encryption` for the forward suite and adds the auth-specific
//!   primitives the encryption crate deliberately does not carry (Argon2id
//!   PHC password hash/verify, RFC 6238 TOTP, the legacy 3DES-EDE2/PBKDF1-MD5
//!   transition cipher).
//! - Every other Ada consumer (medcare-rs, woa-rs, smb-office-rs, and
//!   siblings) that needs raw forward crypto — sealing a secret client-side,
//!   verifying a licence signature, hashing for a merkle chain — pulls
//!   `ogar-encryption` directly rather than the ndarray fork or a hand-rolled
//!   equivalent.
//!
//! ## wasm build
//!
//! ```text
//! cargo check -p ogar-encryption --target wasm32-unknown-unknown
//! cargo build -p ogar-encryption --target wasm32-unknown-unknown \
//!     --features wasm --release
//! ```

#![forbid(unsafe_code)]

// ── KDF + envelope: implemented here on argon2 0.6 (see crate docs).
pub mod envelope;
pub mod kdf;

// ── The rest of the forward suite: re-exported from the ndarray `encryption`
// crate, unmodified.
pub use encryption::{aead, hash, sign};

// ── Root-level convenience aliases, so callers that used the upstream
// crate's short paths keep them.
pub use envelope::{EnvelopeError, KdfParams, open, seal};

// ── The platform-CSPRNG-unavailable error, mirrored from `encryption`'s
// crate root.
pub use encryption::RngError;

/// Fill `buf` from the platform CSPRNG (`getrandom`; on wasm32 this is
/// `crypto.getRandomValues`). The single entropy chokepoint of this crate.
pub(crate) fn fill_random(buf: &mut [u8]) -> Result<(), RngError> {
    getrandom::getrandom(buf).map_err(|_| RngError)
}

/// wasm-bindgen bindings for browser consumers (feature `wasm`): seal/open
/// on this crate's argon2-0.6 envelope, plus Ed25519 and SHA-384.
#[cfg(feature = "wasm")]
pub mod wasm;
