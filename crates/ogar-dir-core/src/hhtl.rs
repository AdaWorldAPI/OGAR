//! The OU tree as a 128-bit HHTL: 8 levels × 16-bit segment ids, root first.
//!
//! ```text
//!   CN=Jan Hübener,OU=Exchange,OU=Infrastructure,OU=Stuttgart,DC=example,DC=de
//!                                                 ─── DC: scope, not a level
//!   level:   0           1                2         3  4  5  6  7
//!   segment: Stuttgart   Infrastructure   Exchange  0  0  0  0  0
//! ```
//!
//! * Only `OU=` components are levels. The leaf and every `DC=` are not.
//! * Segment id `0` means "no level here". Ids `1..=65535` are allocated.
//! * Depth is the number of leading non-zero levels; a non-zero level after a
//!   zero one is malformed and rejected on decode.
//! * More than 8 OU levels is an error, never a silent truncation.
//!
//! ## Segment assignment: explicit per-parent dictionary, not a hash
//!
//! Hashing an OU name into 16 bits collides by the birthday bound after ~300
//! siblings and is not reversible. Instead an [`OuDictionary`] (one per
//! directory scope — one per AD domain / Entra tenant) allocates ids
//! **sequentially per parent path** on first sight, starting at 1:
//!
//! * collisions are impossible by construction (a `(parent, name)` key maps to
//!   exactly one id; an id under a parent maps back to exactly one name);
//! * an id is only meaningful under its parent, so the same id under two
//!   parents is not a collision (65535 children per OU, not per directory);
//! * reconstruction is exact: [`OuDictionary::explain`] returns the original
//!   (first-seen) spelling;
//! * determinism: ids depend on first-seen order, so the dictionary is
//!   *state* and must be persisted alongside the records
//!   ([`OuDictionary::entries`] / [`OuDictionary::from_entries`]). Re-deriving
//!   it from a differently ordered crawl would assign different ids — this is
//!   the documented price of reversibility.
//!
//! Name matching is case-insensitive (AD compares RDN values that way) using
//! Unicode lowercase as the fold. This is an approximation of AD's own
//! collation and is flagged as such; the original spelling is what `explain`
//! returns.

use std::collections::HashMap;

/// Number of HHTL levels.
pub const OU_LEVELS: usize = 8;

/// 8 × u16 OU path, root first. Bytes on the wire: each level little-endian.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug, PartialOrd, Ord)]
pub struct OuHhtl(pub [u16; OU_LEVELS]);

/// HHTL / dictionary failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HhtlError {
    /// More than [`OU_LEVELS`] OU components.
    TooDeep(usize),
    /// A parent already has 65535 children.
    ParentExhausted,
    /// A non-zero level follows a zero level.
    Gap(usize),
    /// An OU name was empty.
    EmptyName,
    /// Persisted entries map two names to one id, or one name to two ids.
    Collision,
}

impl std::fmt::Display for HhtlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ou hhtl: {self:?}")
    }
}
impl std::error::Error for HhtlError {}

impl OuHhtl {
    /// The empty path (object directly under the naming context).
    pub const ROOT: Self = Self([0; OU_LEVELS]);

    /// Number of populated levels.
    pub fn depth(&self) -> usize {
        self.0.iter().take_while(|&&s| s != 0).count()
    }

    /// The first `depth` levels, rest zero.
    pub fn prefix(&self, depth: usize) -> Self {
        let mut p = Self::ROOT;
        p.0[..depth].copy_from_slice(&self.0[..depth]);
        p
    }

    /// Child path with `seg` appended at the next free level.
    fn child(&self, seg: u16) -> Result<Self, HhtlError> {
        let d = self.depth();
        if d >= OU_LEVELS {
            return Err(HhtlError::TooDeep(d + 1));
        }
        let mut c = *self;
        c.0[d] = seg;
        Ok(c)
    }

    /// True if `self` is a (non-strict) ancestor of `other`.
    pub fn is_ancestor_of(&self, other: &Self) -> bool {
        let d = self.depth();
        d <= other.depth() && self.0[..d] == other.0[..d]
    }

    /// 16 wire bytes, each level little-endian.
    pub fn to_le_bytes(&self) -> [u8; 16] {
        let mut b = [0u8; 16];
        for (i, s) in self.0.iter().enumerate() {
            b[i * 2..i * 2 + 2].copy_from_slice(&s.to_le_bytes());
        }
        b
    }

    /// Decode 16 wire bytes, rejecting gaps.
    pub fn from_le_bytes(b: &[u8; 16]) -> Result<Self, HhtlError> {
        let mut h = Self::ROOT;
        for i in 0..OU_LEVELS {
            h.0[i] = u16::from_le_bytes([b[i * 2], b[i * 2 + 1]]);
        }
        let d = h.depth();
        if let Some(i) = (d..OU_LEVELS).find(|&i| h.0[i] != 0) {
            return Err(HhtlError::Gap(i));
        }
        Ok(h)
    }
}

/// Per-scope OU segment registry. See the module docs.
#[derive(Debug, Default, Clone)]
pub struct OuDictionary {
    forward: HashMap<(OuHhtl, String), u16>,
    reverse: HashMap<(OuHhtl, u16), String>,
    next: HashMap<OuHhtl, u16>,
}

fn fold(name: &str) -> String {
    name.to_lowercase()
}

impl OuDictionary {
    /// Empty dictionary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve a root-first OU path, allocating ids for unseen segments.
    pub fn intern<S: AsRef<str>>(&mut self, root_first: &[S]) -> Result<OuHhtl, HhtlError> {
        if root_first.len() > OU_LEVELS {
            return Err(HhtlError::TooDeep(root_first.len()));
        }
        let mut path = OuHhtl::ROOT;
        for name in root_first {
            let name = name.as_ref();
            if name.is_empty() {
                return Err(HhtlError::EmptyName);
            }
            let key = (path, fold(name));
            let seg = match self.forward.get(&key) {
                Some(&s) => s,
                None => {
                    let n = self.next.entry(path).or_insert(1);
                    if *n == 0 {
                        return Err(HhtlError::ParentExhausted);
                    }
                    let s = *n;
                    *n = n.wrapping_add(1); // 65535 -> 0 marks exhaustion
                    self.forward.insert(key, s);
                    self.reverse.insert((path, s), name.to_string());
                    s
                }
            };
            path = path.child(seg)?;
        }
        Ok(path)
    }

    /// Resolve without allocating; `None` if any segment is unknown.
    pub fn resolve<S: AsRef<str>>(&self, root_first: &[S]) -> Option<OuHhtl> {
        if root_first.len() > OU_LEVELS {
            return None;
        }
        let mut path = OuHhtl::ROOT;
        for name in root_first {
            let s = *self.forward.get(&(path, fold(name.as_ref())))?;
            path = path.child(s).ok()?;
        }
        Some(path)
    }

    /// Reconstruct the root-first OU names (original spelling).
    pub fn explain(&self, h: &OuHhtl) -> Option<Vec<String>> {
        (0..h.depth())
            .map(|d| self.reverse.get(&(h.prefix(d), h.0[d])).cloned())
            .collect()
    }

    /// Persistable entries `(parent, segment, original name)`, sorted.
    pub fn entries(&self) -> Vec<(OuHhtl, u16, String)> {
        let mut v: Vec<_> = self
            .reverse
            .iter()
            .map(|((p, s), n)| (*p, *s, n.clone()))
            .collect();
        v.sort();
        v
    }

    /// Rebuild from [`Self::entries`]. Rejects a duplicate `(parent, segment)`
    /// or a duplicate `(parent, folded name)` — either would be a collision.
    pub fn from_entries(entries: &[(OuHhtl, u16, String)]) -> Result<Self, HhtlError> {
        let mut d = Self::new();
        for (p, s, n) in entries {
            if *s == 0 || n.is_empty() {
                return Err(HhtlError::EmptyName);
            }
            if d.reverse.insert((*p, *s), n.clone()).is_some()
                || d.forward.insert((*p, fold(n)), *s).is_some()
            {
                return Err(HhtlError::Collision);
            }
            let next = d.next.entry(*p).or_insert(1);
            if *s == u16::MAX {
                *next = 0; // parent exhausted
            } else if *next != 0 && s + 1 > *next {
                *next = s + 1;
            }
        }
        Ok(d)
    }

    /// Number of allocated segments.
    pub fn len(&self) -> usize {
        self.reverse.len()
    }

    /// True if nothing is allocated.
    pub fn is_empty(&self) -> bool {
        self.reverse.is_empty()
    }
}
