//! # ogar-encryption — OGAR's generic encryption surface
//!
//! One small, classid-agnostic crate re-exporting the Ada stack's forward
//! crypto suite, so every OGAR consumer that needs raw encryption imports
//! **exactly one crate** instead of depping the ndarray fork directly or
//! (worse) hand-rolling its own Argon2/XChaCha20/Ed25519 wrapper.
//!
//! ## What lives here
//!
//! Nothing is implemented in this crate. It is a thin, documented re-export
//! of [`encryption`] (the ndarray fork's audited, wasm-capable crypto
//! module):
//!
//! | Module / item | Primitive | Role |
//! |---|---|---|
//! | [`kdf`] | Argon2id | password/secret → raw key derivation |
//! | [`aead`] | XChaCha20-Poly1305 | authenticated encryption |
//! | [`hash`] | SHA-384 | merkle / fingerprint hashing |
//! | [`sign`] | Ed25519 | licence / audit signatures |
//! | [`envelope`] | seal / open | wasm-capable zero-knowledge envelope |
//! | [`seal`], [`open`] | — | root-level aliases for `envelope::seal` / `envelope::open` |
//! | [`EnvelopeError`], [`KdfParams`] | — | root-level aliases for the envelope's error + parameter types |
//! | [`RngError`] | — | the platform-CSPRNG-unavailable error |
//! | [`wasm`] (feature `wasm`) | — | wasm-bindgen bindings for browser consumers |
//!
//! ## Generic, classid-agnostic, no secrets — by construction
//!
//! This crate carries **no consumer specifics**: no classid, no tenant, no
//! key material, no wire DTO. It is pure re-export surface — every symbol
//! here is exactly what [`encryption`] exports, unmodified. That is the
//! point: the forward suite must never diverge into per-consumer copies, and
//! a crate that re-exports and adds nothing cannot dilute it.
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

// ── The forward suite: re-exported wholesale from the ndarray `encryption`
// crate. Reused, never re-implemented (see crate docs). A consumer that pulls
// `ogar-encryption` gets the entire forward crypto surface under one import.
pub use encryption::{aead, envelope, hash, kdf, sign};

// ── Root-level convenience aliases, mirrored from `encryption`'s own root
// re-exports (`envelope::{seal, open}` plus the envelope's error/parameter
// types), so callers that used the upstream crate's short paths keep them.
pub use encryption::{EnvelopeError, KdfParams, open, seal};

// ── The platform-CSPRNG-unavailable error, mirrored from `encryption`'s
// crate root.
pub use encryption::RngError;

/// wasm-bindgen bindings for browser consumers (forwarded from
/// [`encryption`]'s `wasm-bindings` feature via this crate's `wasm` feature).
#[cfg(feature = "wasm")]
pub use encryption::wasm;
