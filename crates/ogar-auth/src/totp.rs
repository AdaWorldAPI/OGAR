//! TOTP (RFC 6238) — the second factor at the token-mint point.
//!
//! HMAC-SHA1 / 30-second step / 6 digits: the authenticator-app ecosystem
//! default (Google Authenticator, Aegis, 1Password, FreeOTP all assume it).
//! SHA1 here is interop-mandated and MAC-keyed (HMAC), not a collision
//! surface; passwords stay on Argon2id (see [`crate::password`]), integrity
//! stays on SHA-384 (see [`crate::hash`]).
//!
//! Consumer pattern: enforce at the ONE place a session token is minted (both
//! login paths funnel through it). An account with a confirmed secret cannot
//! obtain a token without a valid code; unenrolled and enrolled-but-unconfirmed
//! accounts mint exactly as before — behaviour-preserving until the user proves
//! possession once. The secret is stored base32-encoded (RFC 4648, no padding)
//! — the same representation the `otpauth://` provisioning URI carries, so one
//! string serves storage, QR, and verification.

use hmac::{Hmac, Mac};
use sha1::Sha1;

use crate::{AuthError, AuthResult};

/// RFC 6238 defaults: 30-second time step, 6-digit codes.
pub const STEP_SECONDS: u64 = 30;
pub const DIGITS: u32 = 6;
/// Accept the previous/next step as well (±30 s) — clock-skew tolerance,
/// the common authenticator-server compromise (RFC 6238 §5.2).
pub const SKEW_STEPS: u64 = 1;

const B32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// RFC 4648 base32, no padding — the otpauth:// secret representation.
#[must_use]
pub fn base32_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(5) * 8);
    for chunk in data.chunks(5) {
        let mut buf = [0u8; 5];
        buf[..chunk.len()].copy_from_slice(chunk);
        let bits = u64::from(buf[0]) << 32
            | u64::from(buf[1]) << 24
            | u64::from(buf[2]) << 16
            | u64::from(buf[3]) << 8
            | u64::from(buf[4]);
        let n_chars = match chunk.len() {
            1 => 2,
            2 => 4,
            3 => 5,
            4 => 7,
            _ => 8,
        };
        for i in 0..n_chars {
            let idx = ((bits >> (35 - 5 * i)) & 0x1f) as usize;
            out.push(B32_ALPHABET[idx] as char);
        }
    }
    out
}

/// Decode RFC 4648 base32 (padding-free; case-insensitive; rejects
/// non-alphabet bytes). Inverse of [`base32_encode`].
pub fn base32_decode(s: &str) -> AuthResult<Vec<u8>> {
    let mut bits: u64 = 0;
    let mut n_bits: u32 = 0;
    let mut out = Vec::with_capacity(s.len() * 5 / 8);
    for ch in s.bytes() {
        if ch == b'=' {
            continue; // tolerate padded input
        }
        let v = match ch.to_ascii_uppercase() {
            c @ b'A'..=b'Z' => c - b'A',
            c @ b'2'..=b'7' => c - b'2' + 26,
            _ => return Err(AuthError::Encoding("totp secret is not valid base32")),
        };
        bits = (bits << 5) | u64::from(v);
        n_bits += 5;
        if n_bits >= 8 {
            n_bits -= 8;
            out.push(((bits >> n_bits) & 0xff) as u8);
        }
    }
    // RFC 4648: a valid unpadded base32 group leaves 0..=4 stray bits, ALL of
    // which MUST be zero (the encoder zero-pads). 5..=7 stray bits means an
    // invalid length — a lone/partial symbol carrying real entropy that cannot
    // complete a byte (e.g. "A" would otherwise decode to 0 bytes = an empty
    // HMAC key). Fail CLOSED so a corrupted / imported enrollment secret is
    // rejected, not silently truncated to a shorter (or zero-entropy) key.
    if n_bits >= 5 || (bits & ((1u64 << n_bits) - 1)) != 0 {
        return Err(AuthError::Encoding(
            "totp secret has invalid trailing base32 bits",
        ));
    }
    Ok(out)
}

/// Generate a fresh 20-byte secret (RFC 4226 §4's HMAC-SHA1 block advantage),
/// returned base32-encoded (the storage + provisioning representation).
/// Entropy is the platform CSPRNG via `getrandom` (crypto.getRandomValues on
/// wasm32) — never a userspace PRNG.
///
/// # Errors
/// Returns [`AuthError::Encoding`] only if the platform CSPRNG is unavailable.
pub fn generate_secret_base32() -> AuthResult<String> {
    let mut secret = [0u8; 20];
    getrandom::getrandom(&mut secret)
        .map_err(|_| AuthError::Encoding("platform CSPRNG unavailable"))?;
    Ok(base32_encode(&secret))
}

/// The otpauth:// provisioning URI an authenticator app enrolls from (rendered
/// as a QR client-side; the URI itself is enough for manual entry). `issuer`
/// and `account` are caller-supplied — this crate names no consumer.
#[must_use]
pub fn provisioning_uri(secret_base32: &str, account: &str, issuer: &str) -> String {
    // Minimal percent-encoding for the label/issuer (space + reserved chars).
    fn enc(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    out.push(b as char);
                }
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }
    format!(
        "otpauth://totp/{issuer}:{account}?secret={secret}&issuer={issuer}&algorithm=SHA1&digits={DIGITS}&period={STEP_SECONDS}",
        issuer = enc(issuer),
        account = enc(account),
        secret = secret_base32,
    )
}

/// HOTP (RFC 4226): HMAC-SHA1 over the big-endian counter, dynamic
/// truncation, mod 10^digits.
fn hotp(secret: &[u8], counter: u64) -> u32 {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret)
        .expect("HMAC-SHA1 accepts any key length by construction");
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let bin = (u32::from(digest[offset]) & 0x7f) << 24
        | u32::from(digest[offset + 1]) << 16
        | u32::from(digest[offset + 2]) << 8
        | u32::from(digest[offset + 3]);
    bin % 10u32.pow(DIGITS)
}

/// The current 6-digit code for a base32 secret at `unix_time` (test seam;
/// production callers use [`verify_code`]).
pub fn code_at(secret_base32: &str, unix_time: u64) -> AuthResult<String> {
    let secret = base32_decode(secret_base32)?;
    let code = hotp(&secret, unix_time / STEP_SECONDS);
    Ok(format!("{code:0width$}", width = DIGITS as usize))
}

/// Verify a submitted code against the secret at `unix_time`, accepting
/// ±[`SKEW_STEPS`] steps of clock skew. Comparison is uniform-work over the
/// fixed-width code strings (no early exit), the RFC 6238 §5.2
/// resynchronization guidance bounded to one step in each direction.
pub fn verify_code(secret_base32: &str, submitted: &str, unix_time: u64) -> AuthResult<bool> {
    let submitted = submitted.trim();
    if submitted.len() != DIGITS as usize || !submitted.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(false);
    }
    let secret = base32_decode(secret_base32)?;
    let step_now = unix_time / STEP_SECONDS;
    let mut ok = 0u8;
    for step in step_now.saturating_sub(SKEW_STEPS)..=step_now + SKEW_STEPS {
        let expected = format!("{:0width$}", hotp(&secret, step), width = DIGITS as usize);
        // Accumulate matches without early exit — uniform work per attempt.
        let mut diff = 0u8;
        for (a, b) in expected.bytes().zip(submitted.bytes()) {
            diff |= a ^ b;
        }
        ok |= u8::from(diff == 0);
    }
    Ok(ok == 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238 Appendix B reference vectors (SHA1 mode), truncated from the
    /// published 8-digit codes to our 6-digit width (same dynamic-truncation
    /// output, mod 10^6). Secret = ASCII "12345678901234567890".
    #[test]
    fn rfc6238_appendix_b_vectors_sha1() {
        let secret_b32 = base32_encode(b"12345678901234567890");
        for (t, expected8) in [
            (59u64, "94287082"),
            (1_111_111_109, "07081804"),
            (1_111_111_111, "14050471"),
            (1_234_567_890, "89005924"),
            (2_000_000_000, "69279037"),
            (20_000_000_000, "65353130"),
        ] {
            let want = &expected8[2..]; // 6-digit truncation of the 8-digit vector
            assert_eq!(code_at(&secret_b32, t).unwrap(), want, "T={t}");
        }
    }

    #[test]
    fn base32_round_trips_all_lengths() {
        for len in 0..=20 {
            let data: Vec<u8> = (0..len as u8).map(|i| i.wrapping_mul(37) ^ 0x5c).collect();
            let enc = base32_encode(&data);
            assert_eq!(base32_decode(&enc).unwrap(), data, "len={len}");
            // decoder is case-insensitive (authenticator apps lowercase on export)
            assert_eq!(base32_decode(&enc.to_lowercase()).unwrap(), data);
        }
        assert!(base32_decode("not!base32").is_err());
    }

    #[test]
    fn base32_rejects_invalid_trailing_bits() {
        // Lengths that leave 5..=7 stray bits cannot complete a byte and would
        // otherwise silently truncate — "A" -> empty HMAC key (zero entropy).
        for bad in ["A", "ABC", "ABCDEF", "AAAAAA"] {
            assert!(
                base32_decode(bad).is_err(),
                "{bad:?} has an invalid unpadded length and must be rejected"
            );
        }
        // Valid length but non-zero trailing bits (encoder always zero-pads, so
        // this is a corrupted secret) must also fail closed.
        assert!(base32_decode("AB").is_err(), "non-zero trailing bits");
        // Canonical zero-padded values still decode.
        assert_eq!(base32_decode("AA").unwrap(), vec![0u8]);
        assert!(base32_decode(&base32_encode(b"hi")).is_ok());
    }

    #[test]
    fn verify_accepts_skew_window_and_rejects_outside() {
        let secret = generate_secret_base32().unwrap();
        let now = 1_700_000_000u64;
        let code = code_at(&secret, now).unwrap();
        assert!(verify_code(&secret, &code, now).unwrap());
        // ±1 step (30 s) accepted — clock-skew tolerance
        assert!(verify_code(&secret, &code, now + STEP_SECONDS).unwrap());
        assert!(verify_code(&secret, &code, now - STEP_SECONDS).unwrap());
        // ±2 steps rejected — the window is bounded
        assert!(!verify_code(&secret, &code, now + 2 * STEP_SECONDS).unwrap());
        assert!(!verify_code(&secret, &code, now - 2 * STEP_SECONDS).unwrap());
    }

    #[test]
    fn verify_rejects_malformed_codes_without_erroring() {
        let secret = generate_secret_base32().unwrap();
        for bad in ["", "12345", "1234567", "12a456", " 123456 7"] {
            assert!(
                !verify_code(&secret, bad, 1_700_000_000).unwrap(),
                "{bad:?}"
            );
        }
        // whitespace-trimmed valid-width code still verifies
        let code = code_at(&secret, 1_700_000_000).unwrap();
        assert!(verify_code(&secret, &format!(" {code} "), 1_700_000_000).unwrap());
    }

    #[test]
    fn provisioning_uri_shape() {
        let uri = provisioning_uri("JBSWY3DPEHPK3PXP", "demo", "Example Org");
        assert!(uri.starts_with("otpauth://totp/Example%20Org:demo?secret=JBSWY3DPEHPK3PXP"));
        assert!(
            uri.contains("algorithm=SHA1") && uri.contains("digits=6") && uri.contains("period=30")
        );
    }
}
