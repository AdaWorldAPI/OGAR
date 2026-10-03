//! 128-bit source identity.
//!
//! **Byte order is fixed, not inferred.** [`Guid128`] stores the 16 bytes in
//! *textual order* — the order the 32 hex digits appear in
//! `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` (RFC 9562 network order). Two inputs
//! differ:
//!
//! * Microsoft Graph returns ids as that string → [`Guid128::parse`].
//! * LDAP returns `objectGUID` as 16 raw bytes in Microsoft *mixed-endian*
//!   (`Data1` u32 LE, `Data2` u16 LE, `Data3` u16 LE, `Data4` 8 bytes) — the
//!   same bytes .NET `Guid.ToByteArray()` and `onPremisesImmutableId` (base64)
//!   carry → [`Guid128::from_ms_bytes`].
//!
//! Getting this wrong would silently produce a different, equally valid-looking
//! GUID; both conversions are tested against a known pair.
//!
//! There is no `u128` in the ABI: the record carries `[u8; 16]` so no Rust
//! integer layout or host endianness is involved. No truncation API exists.

use core::fmt;

/// A 128-bit identifier in textual byte order. All-zero means "absent".
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct Guid128(pub [u8; 16]);

/// Why a GUID string or byte slice was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuidParseError {
    /// Not the 36-char hyphenated (or 38-char braced) form.
    BadLength(usize),
    /// A hyphen is missing or a digit is not hex.
    BadDigit(usize),
    /// Raw input was not exactly 16 bytes.
    BadRawLength(usize),
}

impl fmt::Display for GuidParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadLength(n) => write!(f, "guid string has length {n}, expected 36"),
            Self::BadDigit(i) => write!(f, "guid string invalid at position {i}"),
            Self::BadRawLength(n) => write!(f, "raw guid has {n} bytes, expected 16"),
        }
    }
}
impl std::error::Error for GuidParseError {}

/// Positions of the four hyphens in the 36-char form.
const HYPHENS: [usize; 4] = [8, 13, 18, 23];

impl Guid128 {
    /// The all-zero GUID (never a valid directory object id).
    pub const NIL: Self = Self([0; 16]);

    /// True for [`Self::NIL`].
    pub fn is_nil(&self) -> bool {
        self.0 == [0; 16]
    }

    /// Parse `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`, optionally `{…}`-braced,
    /// case-insensitive.
    pub fn parse(s: &str) -> Result<Self, GuidParseError> {
        let s = s
            .strip_prefix('{')
            .and_then(|t| t.strip_suffix('}'))
            .unwrap_or(s);
        let b = s.as_bytes();
        if b.len() != 36 {
            return Err(GuidParseError::BadLength(b.len()));
        }
        let mut out = [0u8; 16];
        let mut nib = 0usize;
        for (i, &c) in b.iter().enumerate() {
            if HYPHENS.contains(&i) {
                if c != b'-' {
                    return Err(GuidParseError::BadDigit(i));
                }
                continue;
            }
            let v = match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                _ => return Err(GuidParseError::BadDigit(i)),
            };
            out[nib / 2] |= if nib.is_multiple_of(2) { v << 4 } else { v };
            nib += 1;
        }
        Ok(Self(out))
    }

    /// From Microsoft mixed-endian raw bytes (LDAP `objectGUID`,
    /// `Guid.ToByteArray()`, decoded `onPremisesImmutableId`).
    pub fn from_ms_bytes(raw: &[u8]) -> Result<Self, GuidParseError> {
        let r: [u8; 16] = raw
            .try_into()
            .map_err(|_| GuidParseError::BadRawLength(raw.len()))?;
        Ok(Self([
            r[3], r[2], r[1], r[0], r[5], r[4], r[7], r[6], r[8], r[9], r[10], r[11], r[12], r[13],
            r[14], r[15],
        ]))
    }

    /// Back to Microsoft mixed-endian raw bytes (inverse of [`Self::from_ms_bytes`]).
    pub fn to_ms_bytes(&self) -> [u8; 16] {
        let g = self.0;
        [
            g[3], g[2], g[1], g[0], g[5], g[4], g[7], g[6], g[8], g[9], g[10], g[11], g[12], g[13],
            g[14], g[15],
        ]
    }
}

impl fmt::Display for Guid128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, b) in self.0.iter().enumerate() {
            if matches!(i, 4 | 6 | 8 | 10) {
                f.write_str("-")?;
            }
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Guid128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Guid128({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_round_trip_is_exact() {
        let s = "3f2504e0-4f89-11d3-9a0c-0305e82c3301";
        let g = Guid128::parse(s).unwrap();
        assert_eq!(g.0[0], 0x3f);
        assert_eq!(g.0[15], 0x01);
        assert_eq!(g.to_string(), s);
        assert_eq!(Guid128::parse(&s.to_uppercase()).unwrap(), g);
        assert_eq!(Guid128::parse(&format!("{{{s}}}")).unwrap(), g);
    }

    /// Known pair: .NET `new Guid("3f2504e0-4f89-11d3-9a0c-0305e82c3301").ToByteArray()`
    /// = e0 04 25 3f 89 4f d3 11 9a 0c 03 05 e8 2c 33 01.
    #[test]
    fn ms_mixed_endian_matches_dotnet_byte_order() {
        let raw = [
            0xe0, 0x04, 0x25, 0x3f, 0x89, 0x4f, 0xd3, 0x11, 0x9a, 0x0c, 0x03, 0x05, 0xe8, 0x2c,
            0x33, 0x01,
        ];
        let g = Guid128::from_ms_bytes(&raw).unwrap();
        assert_eq!(g.to_string(), "3f2504e0-4f89-11d3-9a0c-0305e82c3301");
        assert_eq!(g.to_ms_bytes(), raw);
        // The two orders really differ — a byte-order mistake is not a no-op.
        assert_ne!(g.0, raw);
    }

    #[test]
    fn rejects_malformed_input() {
        assert!(Guid128::parse("3f2504e0-4f89-11d3-9a0c-0305e82c330").is_err());
        assert!(Guid128::parse("3f2504e0x4f89-11d3-9a0c-0305e82c3301").is_err());
        assert!(Guid128::parse("3f2504e0-4f89-11d3-9a0c-0305e82c33zz").is_err());
        assert!(Guid128::from_ms_bytes(&[0u8; 15]).is_err());
    }
}
