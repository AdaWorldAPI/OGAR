//! Argon2id password hashing — the forward login-credential path.
//!
//! Produces and verifies a PHC-format string (`$argon2id$v=19$m=…,t=…,p=…$salt$hash`),
//! the self-describing credential a consumer stores in its user table. This is
//! the *verification* path (does this password match the stored hash?), which
//! is distinct from [`crate::kdf`] — the envelope *key-derivation* path that
//! turns a password into raw AEAD key bytes. Both are Argon2id; they answer
//! different questions, so the SDK carries both.
//!
//! Argon2's own defaults are used (currently m=19456 KiB, t=2, p=1 — the OWASP
//! first-recommended Argon2id configuration). Because the cost parameters are
//! embedded in the PHC string, a later cost bump stays backward-compatible:
//! old hashes verify against their own embedded parameters.

use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};

use crate::{AuthError, AuthResult};

/// Hash `password` with Argon2id and a fresh random salt, returning the
/// PHC-format string to store. Salt entropy comes from the platform CSPRNG via
/// `getrandom` (the crate's single entropy source), not a userspace PRNG.
///
/// # Errors
/// [`AuthError::Password`] if the CSPRNG is unavailable or the underlying
/// hashing fails. No password material is included in the error.
pub fn hash_password(password: &str) -> AuthResult<String> {
    let mut salt_bytes = [0u8; 16];
    getrandom::getrandom(&mut salt_bytes).map_err(|_| AuthError::Password("CSPRNG unavailable"))?;
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|_| AuthError::Password("salt encode failed"))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AuthError::Password("argon2 hashing failed"))
}

/// Verify `password` against a stored PHC `hash`. Returns `Ok(false)` for a
/// well-formed hash that simply does not match; returns `Err` only if the
/// stored `hash` string is itself malformed (not a valid PHC encoding).
///
/// # Errors
/// [`AuthError::Password`] if `hash` is not a parseable PHC string.
pub fn verify_password(password: &str, hash: &str) -> AuthResult<bool> {
    let parsed =
        PasswordHash::new(hash).map_err(|_| AuthError::Password("malformed stored hash"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrip() {
        let h = hash_password("correct horse battery staple").unwrap();
        assert!(h.starts_with("$argon2id$"));
        assert!(verify_password("correct horse battery staple", &h).unwrap());
        assert!(!verify_password("wrong", &h).unwrap());
    }

    #[test]
    fn distinct_salts_give_distinct_hashes() {
        let a = hash_password("same-pw").unwrap();
        let b = hash_password("same-pw").unwrap();
        assert_ne!(a, b, "random salt must make the PHC strings differ");
        // …yet both verify against the original password.
        assert!(verify_password("same-pw", &a).unwrap());
        assert!(verify_password("same-pw", &b).unwrap());
    }

    #[test]
    fn malformed_stored_hash_errors() {
        assert!(matches!(
            verify_password("pw", "not-a-phc-string"),
            Err(AuthError::Password(_))
        ));
    }
}
