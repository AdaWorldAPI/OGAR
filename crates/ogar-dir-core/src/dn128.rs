//! The hot hierarchy coordinate: up to 16 levels, one byte per level.
//!
//! ```text
//!   DirectoryScope  ×  Dn128
//!   (domain/tenant)    codes[0..depth]  depth (side metadata)
//! ```
//!
//! * A level's code is a byte, `0..=255`: up to **256 distinct children per
//!   parent**. Every byte value is a real code — `0` included — so a byte is
//!   never stolen as a "no level" marker. How many levels are populated is the
//!   explicit [`Dn128::depth`], stored beside the codes.
//! * Bytes past `depth` are zero. That is a canonical form (so equality and
//!   ordering are byte comparisons), not a meaning: presence comes from
//!   `depth` alone.
//! * The domain or tenant is not a level. It is the [`DirectoryScope`] the
//!   codes are interpreted in; a code is only meaningful under its scope and
//!   its parent path.
//!
//! Subtree selection is a prefix match on the first `d` bytes plus
//! `depth >= d`: one ternary compare on the 16 bytes and one on the depth.
//!
//! [`OuHhtl`] (`[u16; 8]`) remains the ingress and wire format of the
//! directory records. [`Dn128::from_ou_hhtl`] converts it and fails closed
//! when a parent has more than 256 children; it never hashes or truncates.

use crate::guid::Guid128;
use crate::hhtl::OuHhtl;

/// Maximum hierarchy depth.
pub const DN_LEVELS: usize = 16;

/// Maximum distinct child codes under one parent.
pub const DN_CHILDREN: usize = 256;

/// The directory a [`Dn128`] is interpreted in: an AD domain or an Entra
/// tenant, by its identifier. Codes from two scopes are never comparable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DirectoryScope(pub Guid128);

/// 16 one-byte hierarchy levels, root first, plus an explicit depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Dn128 {
    codes: [u8; DN_LEVELS],
    depth: u8,
}

/// Why a [`Dn128`] could not be formed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dn128Error {
    /// More than [`DN_LEVELS`] levels.
    TooDeep(usize),
    /// A parent has more than [`DN_CHILDREN`] children: the source segment
    /// id (1-based, as `OuDictionary` allocates it) does not fit a byte.
    ChildCodeOverflow {
        /// Level (0 = root).
        level: usize,
        /// The source segment id.
        segment: u16,
    },
    /// The source path has a zero segment before its last level (malformed).
    Gap(usize),
}

impl std::fmt::Display for Dn128Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dn128: {self:?}")
    }
}
impl std::error::Error for Dn128Error {}

impl Dn128 {
    /// The root (depth 0): an object directly under the naming context.
    pub const ROOT: Self = Self {
        codes: [0; DN_LEVELS],
        depth: 0,
    };

    /// From root-first level codes.
    pub fn new(codes: &[u8]) -> Result<Self, Dn128Error> {
        if codes.len() > DN_LEVELS {
            return Err(Dn128Error::TooDeep(codes.len()));
        }
        let mut d = Self::ROOT;
        d.codes[..codes.len()].copy_from_slice(codes);
        d.depth = codes.len() as u8;
        Ok(d)
    }

    /// Convert an ingress [`OuHhtl`]. Segment `s` (1..=65535, allocated per
    /// parent from 1) becomes code `s - 1`; `s > 256` means the parent has
    /// more than 256 children and is refused.
    pub fn from_ou_hhtl(h: &OuHhtl) -> Result<Self, Dn128Error> {
        let depth = h.depth();
        if let Some(i) = (depth..h.0.len()).find(|&i| h.0[i] != 0) {
            return Err(Dn128Error::Gap(i));
        }
        let mut codes = [0u8; DN_LEVELS];
        for (level, &segment) in h.0[..depth].iter().enumerate() {
            codes[level] = u8::try_from(segment - 1)
                .map_err(|_| Dn128Error::ChildCodeOverflow { level, segment })?;
        }
        Self::new(&codes[..depth])
    }

    /// Number of populated levels.
    pub fn depth(&self) -> usize {
        usize::from(self.depth)
    }

    /// The populated codes, root first.
    pub fn codes(&self) -> &[u8] {
        &self.codes[..self.depth()]
    }

    /// All 16 code bytes (zero past [`Self::depth`]) — the lane field.
    pub fn bytes(&self) -> [u8; DN_LEVELS] {
        self.codes
    }

    /// The first `depth` levels.
    pub fn prefix(&self, depth: usize) -> Self {
        let d = depth.min(self.depth());
        let mut p = Self::ROOT;
        p.codes[..d].copy_from_slice(&self.codes[..d]);
        p.depth = d as u8;
        p
    }

    /// True if `self` is a (non-strict) ancestor of `other`.
    pub fn is_ancestor_of(&self, other: &Self) -> bool {
        let d = self.depth();
        d <= other.depth() && self.codes[..d] == other.codes[..d]
    }

    /// The ternary match selecting the subtree of `self`: compare the first
    /// `depth` bytes exactly, ignore the rest. Combined with
    /// `node.depth >= self.depth` it selects exactly ancestor-or-self.
    pub fn subtree_mask(&self) -> ([u8; DN_LEVELS], [u8; DN_LEVELS]) {
        let mut care = [0u8; DN_LEVELS];
        care[..self.depth()].fill(0xFF);
        (self.codes, care)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ou(levels: &[u16]) -> OuHhtl {
        let mut h = OuHhtl::ROOT;
        h.0[..levels.len()].copy_from_slice(levels);
        h
    }

    #[test]
    fn code_zero_is_a_level_and_depth_is_explicit() {
        let a = Dn128::new(&[0]).unwrap();
        assert_eq!(a.depth(), 1);
        assert_ne!(a, Dn128::ROOT, "code 0 is a real level, not absence");
        let deep = Dn128::new(&[0; 16]).unwrap();
        assert_eq!(deep.depth(), 16);
        assert!(a.is_ancestor_of(&deep));
        assert!(!deep.is_ancestor_of(&a));
    }

    #[test]
    fn deeper_than_sixteen_is_refused() {
        assert_eq!(Dn128::new(&[1; 17]), Err(Dn128Error::TooDeep(17)));
    }

    #[test]
    fn ou_hhtl_converts_and_fails_closed_past_256_children() {
        let d = Dn128::from_ou_hhtl(&ou(&[1, 256, 7])).unwrap();
        assert_eq!(d.codes(), &[0, 255, 6]);
        assert_eq!(
            Dn128::from_ou_hhtl(&ou(&[1, 257])),
            Err(Dn128Error::ChildCodeOverflow {
                level: 1,
                segment: 257
            })
        );
        assert_eq!(Dn128::from_ou_hhtl(&OuHhtl::ROOT), Ok(Dn128::ROOT));
    }

    #[test]
    fn subtree_mask_compares_only_the_prefix() {
        let p = Dn128::new(&[3, 9]).unwrap();
        let (pattern, care) = p.subtree_mask();
        let matches =
            |n: &Dn128| (0..DN_LEVELS).all(|k| (n.bytes()[k] ^ pattern[k]) & care[k] == 0);
        assert!(matches(&Dn128::new(&[3, 9, 200, 1]).unwrap()));
        assert!(!matches(&Dn128::new(&[3, 8]).unwrap()));
        // The byte compare alone also accepts a shallower node whose
        // zero-filled bytes happen to equal the prefix; the depth gate is
        // what excludes it.
        let shallow = Dn128::new(&[3]).unwrap();
        let p0 = Dn128::new(&[3, 0]).unwrap();
        let (pat0, care0) = p0.subtree_mask();
        assert!((0..DN_LEVELS).all(|k| (shallow.bytes()[k] ^ pat0[k]) & care0[k] == 0));
        assert!(!p0.is_ancestor_of(&shallow));
    }
}
