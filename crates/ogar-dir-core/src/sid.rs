//! Windows security identifiers. AD stores `objectSid` as the binary form
//! ([MS-DTYP] 2.4.2.2); Graph's `onPremisesSecurityIdentifier` is the
//! string form ([MS-DTYP] 2.4.2.1), `S-{revision}-{authority}-{sub}...`.

use std::fmt::Write;

/// The string form of a binary SID, or `None` if it is malformed (shorter
/// than the 8-byte header, or a length that disagrees with its
/// sub-authority count).
///
/// The 48-bit identifier authority is big-endian and printed in decimal
/// when it fits 32 bits, otherwise as `0x` and 12 hex digits; each
/// sub-authority is a little-endian `u32` in decimal.
pub fn sid_to_string(b: &[u8]) -> Option<String> {
    let [revision, count, ..] = *b else {
        return None;
    };
    let authority = b
        .get(2..8)?
        .iter()
        .fold(0u64, |a, &x| a << 8 | u64::from(x));
    let subs = &b[8..];
    if subs.len() != usize::from(count) * 4 {
        return None;
    }
    let mut s = if authority < 1 << 32 {
        format!("S-{revision}-{authority}")
    } else {
        format!("S-{revision}-0x{authority:012X}")
    };
    for c in subs.as_chunks::<4>().0 {
        let _ = write!(s, "-{}", u32::from_le_bytes(*c));
    }
    Some(s)
}
