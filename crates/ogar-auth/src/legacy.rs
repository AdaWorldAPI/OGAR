//! Legacy 3DES-EDE2 transition — verify-during-migration, not a forward path.
//!
//! Some legacy corpora (classic .NET line-of-business apps) store secrets that
//! `TripleDESCryptoServiceProvider` produced with `PasswordDeriveBytes` +
//! `CryptDeriveKey`. To migrate such a corpus onto the forward suite
//! ([`crate::password`] / [`crate::kdf`]) you must first *read* the old value
//! once — decrypt it, then immediately re-protect it forward and discard the
//! legacy path. This module is that one-way door.
//!
//! ## The algorithm (agnostic — carries no secrets)
//!
//! The .NET construction all such corpora share:
//!
//! - **Key derivation** — `PasswordDeriveBytes(password, emptySalt)` then
//!   `CryptDeriveKey("RC2","MD5",128,IV)`: `state = MD5(password)`, folded
//!   `iterations-1` times, one final `MD5(state)`; the first 16 bytes are the
//!   RC2-128 key, which `des.Key = <16 bytes>` interprets as TripleDES-EDE2
//!   (K3 = K1). See [`derive_tdes_key_pbkdf1_md5`].
//! - **Cipher** — 3DES-EDE2, CBC, zero IV (`des.IV = new byte[8]`), PKCS#7.
//! - **Storage** — `prefix ‖ base64(ciphertext)`, one prefix char naming a
//!   password-table index (base62 order: `0-9`, `a-z`, `A-Z`).
//!
//! **This crate bakes NO password table.** The 62-key (or however-many)
//! table belongs to the consumer that owns the legacy corpus and is passed as
//! a `&[&str]` argument. That is the whole reason this primitive is public and
//! agnostic while the table stays private: the *algorithm* is a documented
//! .NET convention; the *table* is a live secret.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyInit};
use md5::{Digest, Md5};

use crate::{AuthError, AuthResult};

/// .NET `PasswordDeriveBytes` default iteration count.
pub const DEFAULT_PBKDF1_ITERATIONS: usize = 100;

const MD5_LEN: usize = 16;
const TDES_BLOCK: usize = 8;

/// Map a single ciphertext prefix char to its password-table index using the
/// base62 order .NET's helper emits: `'0'..='9'` → 0..9, `'a'..='z'` → 10..35,
/// `'A'..='Z'` → 36..61. Returns `None` for any other byte.
#[must_use]
pub fn prefix_to_index(c: char) -> Option<usize> {
    match c {
        '0'..='9' => Some(c as usize - '0' as usize),
        'a'..='z' => Some(10 + (c as usize - 'a' as usize)),
        'A'..='Z' => Some(36 + (c as usize - 'A' as usize)),
        _ => None,
    }
}

/// Inverse of [`prefix_to_index`] (0..=61 → the prefix char).
#[must_use]
pub fn index_to_prefix(idx: usize) -> Option<char> {
    match idx {
        0..=9 => char::from_u32('0' as u32 + idx as u32),
        10..=35 => char::from_u32('a' as u32 + (idx - 10) as u32),
        36..=61 => char::from_u32('A' as u32 + (idx - 36) as u32),
        _ => None,
    }
}

/// `.NET PasswordDeriveBytes` over MD5 with empty salt and `iterations`
/// rounds, then the `CryptDeriveKey("RC2","MD5",128,…)` step, expanded to the
/// 24-byte TripleDES-EDE2 key (K1‖K2‖K1).
#[must_use]
pub fn derive_tdes_key_pbkdf1_md5(password: &[u8], iterations: usize) -> [u8; 24] {
    // Step 1 — initial hash of password (empty salt appended unchanged).
    let mut hasher = Md5::new();
    hasher.update(password);
    let mut state: [u8; MD5_LEN] = hasher.finalize().into();

    // Steps 2..iterations-1 — fold through MD5.
    for _ in 1..iterations.saturating_sub(1) {
        let mut h = Md5::new();
        h.update(state);
        state = h.finalize().into();
    }

    // Output stream — first (only needed) chunk: MD5(state) → 16-byte RC2 key.
    let mut h = Md5::new();
    h.update(state);
    let derived: [u8; MD5_LEN] = h.finalize().into();

    let mut k = [0u8; 24];
    k[..16].copy_from_slice(&derived);
    k[16..].copy_from_slice(&derived[..8]); // K3 = K1 (two-key EDE)
    k
}

/// Encrypt `plaintext` with a 24-byte EDE2 key: 3DES CBC, zero IV, PKCS#7.
/// The inverse of [`decrypt_cbc_zero_iv`]; deterministic (zero IV), which is
/// what makes round-trip self-consistency testable.
#[must_use]
pub fn encrypt_cbc_zero_iv(key: &[u8; 24], plaintext: &[u8]) -> Vec<u8> {
    let mut tdes = des::TdesEde3::new_from_slice(key).expect("TripleDES key length is 24");
    let mut buf = plaintext.to_vec();
    let pad = TDES_BLOCK - (buf.len() % TDES_BLOCK);
    buf.extend(std::iter::repeat_n(pad as u8, pad));

    let mut prev_iv = [0u8; TDES_BLOCK];
    for block in buf.chunks_mut(TDES_BLOCK) {
        for (b, iv) in block.iter_mut().zip(prev_iv.iter()) {
            *b ^= *iv;
        }
        let mut arr = [0u8; TDES_BLOCK];
        arr.copy_from_slice(block);
        tdes.encrypt_block_mut((&mut arr).into());
        block.copy_from_slice(&arr);
        prev_iv.copy_from_slice(block);
    }
    buf
}

/// Decrypt `ciphertext` (a multiple of the 8-byte block size) with a 24-byte
/// EDE2 key: 3DES CBC, zero IV, PKCS#7 strip. Returns the raw plaintext bytes.
///
/// # Errors
/// [`AuthError::Legacy`] on block misalignment or PKCS#7 padding failure.
pub fn decrypt_cbc_zero_iv(key: &[u8; 24], ciphertext: &[u8]) -> AuthResult<Vec<u8>> {
    if ciphertext.is_empty() || !ciphertext.len().is_multiple_of(TDES_BLOCK) {
        return Err(AuthError::Legacy(
            "ciphertext is not a whole number of 3DES blocks",
        ));
    }
    let mut tdes = des::TdesEde3::new_from_slice(key).expect("TripleDES key length is 24");
    let mut plain = Vec::with_capacity(ciphertext.len());
    let mut prev_iv = [0u8; TDES_BLOCK];
    for chunk in ciphertext.chunks(TDES_BLOCK) {
        let mut block = [0u8; TDES_BLOCK];
        block.copy_from_slice(chunk);
        let mut decoded = block;
        tdes.decrypt_block_mut((&mut decoded).into());
        for (b, iv) in decoded.iter_mut().zip(prev_iv.iter()) {
            *b ^= *iv;
        }
        plain.extend_from_slice(&decoded);
        prev_iv = block;
    }

    let pad = *plain
        .last()
        .ok_or(AuthError::Legacy("empty plaintext after decrypt"))? as usize;
    if pad == 0 || pad > TDES_BLOCK || pad > plain.len() {
        return Err(AuthError::Legacy("PKCS#7 pad byte out of range"));
    }
    if plain[plain.len() - pad..]
        .iter()
        .any(|&b| b as usize != pad)
    {
        return Err(AuthError::Legacy("PKCS#7 padding bytes are not uniform"));
    }
    plain.truncate(plain.len() - pad);
    Ok(plain)
}

/// Encrypt `plaintext` under `table[index]` and emit the stored form
/// (`prefix ‖ base64(ciphertext)`). The inverse of [`decrypt_prefixed`];
/// exists for round-trip self-consistency and for generating vectors to diff
/// against a private oracle. NOT a forward path.
///
/// # Errors
/// [`AuthError::Legacy`] if `index` is outside the prefix range or the table.
pub fn encrypt_prefixed(
    plaintext: &str,
    index: usize,
    table: &[&str],
    iterations: usize,
) -> AuthResult<String> {
    let prefix = index_to_prefix(index).ok_or(AuthError::Legacy("index has no prefix char"))?;
    let password = table
        .get(index)
        .ok_or(AuthError::Legacy("index outside password table"))?;
    let key = derive_tdes_key_pbkdf1_md5(password.as_bytes(), iterations);
    let ct = encrypt_cbc_zero_iv(&key, plaintext.as_bytes());
    Ok(format!("{prefix}{}", BASE64.encode(&ct)))
}

/// Decrypt a stored legacy value (`prefix ‖ base64(ciphertext)`) using the
/// caller-supplied password `table`. Returns `Ok(None)` for empty input.
///
/// The `table` is the consumer's private secret; this crate never carries one.
///
/// # Errors
/// [`AuthError::Legacy`] on an unknown/out-of-range prefix, a truncated value,
/// bad base64, block misalignment, or padding failure. [`AuthError::Encoding`]
/// if the decrypted bytes are not valid UTF-8.
pub fn decrypt_prefixed(
    input: &str,
    table: &[&str],
    iterations: usize,
) -> AuthResult<Option<String>> {
    if input.is_empty() {
        return Ok(None);
    }
    let mut chars = input.chars();
    let prefix = chars
        .next()
        .ok_or(AuthError::Legacy("empty legacy value"))?;
    let index = prefix_to_index(prefix).ok_or(AuthError::Legacy("unrecognised prefix char"))?;
    let password = table
        .get(index)
        .ok_or(AuthError::Legacy("prefix index outside password table"))?;
    let body: String = chars.collect();
    if body.is_empty() {
        return Err(AuthError::Legacy("legacy value has a prefix but no body"));
    }

    let ciphertext = BASE64
        .decode(body.as_bytes())
        .map_err(|_| AuthError::Encoding("legacy body is not base64"))?;
    let key = derive_tdes_key_pbkdf1_md5(password.as_bytes(), iterations);
    let plain = decrypt_cbc_zero_iv(&key, &ciphertext)?;
    let s = String::from_utf8(plain)
        .map_err(|_| AuthError::Encoding("legacy plaintext is not UTF-8"))?;
    Ok(Some(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    // A SYNTHETIC table — invented passwords, never a real corpus's secret.
    // It exists only to prove the machinery is internally coherent; byte-parity
    // against a real .NET oracle is a private consumer test, not a public one.
    const SYNTHETIC_TABLE: [&str; 62] = [
        "pw00", "pw01", "pw02", "pw03", "pw04", "pw05", "pw06", "pw07", "pw08", "pw09", "pw10",
        "pw11", "pw12", "pw13", "pw14", "pw15", "pw16", "pw17", "pw18", "pw19", "pw20", "pw21",
        "pw22", "pw23", "pw24", "pw25", "pw26", "pw27", "pw28", "pw29", "pw30", "pw31", "pw32",
        "pw33", "pw34", "pw35", "pw36", "pw37", "pw38", "pw39", "pw40", "pw41", "pw42", "pw43",
        "pw44", "pw45", "pw46", "pw47", "pw48", "pw49", "pw50", "pw51", "pw52", "pw53", "pw54",
        "pw55", "pw56", "pw57", "pw58", "pw59", "pw60", "pw61",
    ];

    #[test]
    fn prefix_index_round_trips() {
        for (c, i) in [
            ('0', 0),
            ('9', 9),
            ('a', 10),
            ('z', 35),
            ('A', 36),
            ('Z', 61),
        ] {
            assert_eq!(prefix_to_index(c), Some(i));
            assert_eq!(index_to_prefix(i), Some(c));
        }
        assert_eq!(prefix_to_index('!'), None);
        assert_eq!(index_to_prefix(62), None);
    }

    #[test]
    fn encrypt_decrypt_round_trips_for_every_index() {
        // Proves the plumbing (PBKDF1/MD5 → 3DES-EDE2 → CBC zero-IV → PKCS#7 →
        // prefix) is internally coherent for a caller-supplied table. It does
        // NOT prove parity with any real .NET oracle — that is a private test.
        let samples = ["", "a", "hunter2", "café — münchen — 12345", "block-align8"];
        for idx in [0usize, 1, 9, 10, 35, 36, 61] {
            for pt in samples {
                let ct =
                    encrypt_prefixed(pt, idx, &SYNTHETIC_TABLE, DEFAULT_PBKDF1_ITERATIONS).unwrap();
                assert_eq!(prefix_to_index(ct.chars().next().unwrap()), Some(idx));
                let back =
                    decrypt_prefixed(&ct, &SYNTHETIC_TABLE, DEFAULT_PBKDF1_ITERATIONS).unwrap();
                assert_eq!(back.as_deref(), Some(pt), "round-trip idx={idx} pt={pt:?}");
            }
        }
    }

    #[test]
    fn decrypt_empty_is_none() {
        assert!(matches!(
            decrypt_prefixed("", &SYNTHETIC_TABLE, DEFAULT_PBKDF1_ITERATIONS),
            Ok(None)
        ));
    }

    #[test]
    fn decrypt_unknown_prefix_errors() {
        assert!(matches!(
            decrypt_prefixed("!body", &SYNTHETIC_TABLE, DEFAULT_PBKDF1_ITERATIONS),
            Err(AuthError::Legacy(_))
        ));
    }

    #[test]
    fn decrypt_prefix_only_errors() {
        assert!(matches!(
            decrypt_prefixed("0", &SYNTHETIC_TABLE, DEFAULT_PBKDF1_ITERATIONS),
            Err(AuthError::Legacy(_))
        ));
    }

    #[test]
    fn encrypt_out_of_range_index_errors() {
        assert!(matches!(
            encrypt_prefixed("x", 62, &SYNTHETIC_TABLE, DEFAULT_PBKDF1_ITERATIONS),
            Err(AuthError::Legacy(_))
        ));
    }
}
