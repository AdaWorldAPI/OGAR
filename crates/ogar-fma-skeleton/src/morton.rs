//! The **4×4 Morton-tile pyramid** — the uniform family-node address cascade.
//!
//! # Why this shape (operator correction, 2026-06-23)
//!
//! The canonical node key is **16 bytes = 16×8-bit family nodes**, a *uniform*
//! array — never a heterogeneous `12+4` carve. Uniformity is load-bearing:
//! it is exactly why the operator reversed the `4/3/3` v8-native carve on
//! 2026-06-10 ("broke the uniform Morton stride"). Each **nibble** (4 bits) is
//! one **4×4 Morton tile**: 2 bits X interleaved with 2 bits Y. 16 bytes ⇒ 32
//! nibble-levels ⇒ a 32-level pyramid of 4×4 spatial refinements, uniform
//! stride throughout.
//!
//! ```text
//!  byte:   0    1    2    3   ...                            15
//!        ┌────┬────┬────┬────┬─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┬────┐
//!        │ hi lo │ …  16 family-node bytes = 16×8 bit = 128-bit key  │
//!        └────┴────┴────┴────┴─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┴────┘
//!  nibble:  0  1   2  3 …                                       31
//!  each nibble = one 4×4 Morton tile:  bit0=x0 bit1=y0 bit2=x1 bit3=y1
//! ```
//!
//! # The perturbation-shader cascade (why we can't drop the Morton structure)
//!
//! Per the OGAR canon (`CLAUDE.md` §"Perturbation encoding"), the stacked
//! perturbation decomposes as `(exponent, location, phase, magnitude)`. Three
//! of the four live in the address itself: **exponent = the pyramid level**
//! (`tier = level >> 2`), **location = the Morton tile at that level**, and
//! **phase = a deterministic recurrence from the address** (generated, never
//! stored). Only **magnitude** is stored — and it lives in the splat layer
//! (ndarray), not here. If the address stored opaque tree-branch indices
//! instead of Morton tiles, `exponent` + `location` would be lost and the
//! shader pyramid would have nothing to refine — "the data we save we lose."
//! OGAR owns `(exponent, location)`; ndarray owns `(phase, magnitude)`.
//!
//! # The one-address collapse (D-BOTHCASC)
//!
//! Because each nibble is a 4×4 Morton tile of a child's position within its
//! parent's bounding box, **nibble-prefix containment is simultaneously
//! spatial containment and partonomy containment**. A bone's address is built
//! by descending the Morton quadtree of its siblings ([`assign_morton_suffixes`]),
//! so `parent.is_ancestor_of(child)` holds by construction *and* spatially
//! near siblings share a longer Morton prefix.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Bits per nibble-level (one 4×4 Morton tile).
pub const TILE_BITS: u32 = 4;
/// Bits per axis per level (2 ⇒ 4 positions per axis ⇒ a 4×4 tile).
pub const AXIS_BITS: u32 = 2;
/// Positions per axis per level (`1 << AXIS_BITS` = 4).
pub const AXIS_CELLS: u8 = 1 << AXIS_BITS;
/// Bytes in a canonical family-node key (16 × 8 bit = 128 bit).
pub const KEY_BYTES: usize = 16;
/// Nibble-levels in the full pyramid (`KEY_BYTES * 2` = 32).
pub const LEVELS: usize = KEY_BYTES * 2;

/// Encode a single 4×4 Morton tile from 2-bit axis coordinates.
///
/// `x` and `y` are each in `0..4`; the result is the interleaved nibble
/// `0..16` with `bit0 = x0, bit1 = y0, bit2 = x1, bit3 = y1` (standard
/// Z-order, X in the even bit-positions).
///
/// ```
/// use ogar_fma_skeleton::morton::tile;
/// assert_eq!(tile(0, 0), 0b0000);
/// assert_eq!(tile(3, 0), 0b0101); // x=11, y=00 -> bits 0 and 2 set
/// assert_eq!(tile(0, 3), 0b1010); // x=00, y=11 -> bits 1 and 3 set
/// assert_eq!(tile(3, 3), 0b1111);
/// ```
#[must_use]
pub const fn tile(x: u8, y: u8) -> u8 {
    let x = x & 0b11;
    let y = y & 0b11;
    (x & 1) | ((y & 1) << 1) | ((x & 2) << 1) | ((y & 2) << 2)
}

/// Decode a 4×4 Morton tile nibble back into `(x, y)` 2-bit axis coordinates.
///
/// Inverse of [`tile`]. The result components are each in `0..4`.
///
/// ```
/// use ogar_fma_skeleton::morton::{tile, untile};
/// for x in 0u8..4 {
///     for y in 0u8..4 {
///         assert_eq!(untile(tile(x, y)), (x, y));
///     }
/// }
/// ```
#[must_use]
pub const fn untile(nibble: u8) -> (u8, u8) {
    let n = nibble & 0b1111;
    let x = (n & 0b0001) | ((n >> 1) & 0b0010);
    let y = ((n >> 1) & 0b0001) | ((n >> 2) & 0b0010);
    (x, y)
}

/// The cascade **tier** a pyramid level belongs to — `level >> 2`, a shift,
/// never a branch (the canon's uniform-stride property). Four nibble-levels
/// per tier ⇒ a 256×256 centroid tile per tier (`CLAUDE.md` §"Tier
/// interpretation").
#[must_use]
pub const fn tier(level: usize) -> usize {
    level >> 2
}

/// A canonical **family-node address**: 16 bytes (16×8 bit) of Morton-tile
/// pyramid, plus the number of assigned nibble-levels (`depth`). Unassigned
/// tail levels are zero (the canon's RESERVE-DON'T-RECLAIM / zero-fallback
/// ladder: zero = "not yet consulted", never compacted away).
///
/// `depth` is in **nibble-levels** (`0..=32`); each level is one 4×4 tile.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FamilyAddress {
    tiles: [u8; KEY_BYTES],
    depth: u8,
}

impl Default for FamilyAddress {
    fn default() -> Self {
        Self::ROOT
    }
}

impl FamilyAddress {
    /// The empty (depth-0) root address — the whole-key zero prefix.
    pub const ROOT: Self = Self {
        tiles: [0u8; KEY_BYTES],
        depth: 0,
    };

    /// Build an address from an explicit nibble-level path (each entry a
    /// 4×4 Morton tile `0..16`). Levels beyond [`LEVELS`] are ignored; tile
    /// values are masked to 4 bits.
    #[must_use]
    pub fn from_nibbles(nibbles: &[u8]) -> Self {
        let mut addr = Self::ROOT;
        for &n in nibbles {
            addr.push(n);
        }
        addr
    }

    /// Number of assigned nibble-levels (`0..=32`).
    #[must_use]
    pub const fn depth(&self) -> u8 {
        self.depth
    }

    /// The raw 16-byte key (Morton tiles packed two-per-byte, hi nibble first).
    #[must_use]
    pub const fn bytes(&self) -> [u8; KEY_BYTES] {
        self.tiles
    }

    /// The Morton tile at nibble-level `level` (`0` = coarsest). Returns `0`
    /// for levels at or beyond [`LEVELS`].
    #[must_use]
    pub const fn nibble(&self, level: usize) -> u8 {
        if level >= LEVELS {
            return 0;
        }
        let byte = self.tiles[level >> 1];
        if level & 1 == 0 {
            byte >> 4
        } else {
            byte & 0x0F
        }
    }

    /// Append one 4×4 Morton tile at the current depth. No-op once the
    /// pyramid is full ([`LEVELS`] levels). The tile is masked to 4 bits.
    pub fn push(&mut self, t: u8) {
        let level = self.depth as usize;
        if level >= LEVELS {
            return;
        }
        let t = t & 0x0F;
        let byte = level >> 1;
        if level & 1 == 0 {
            self.tiles[byte] = (self.tiles[byte] & 0x0F) | (t << 4);
        } else {
            self.tiles[byte] = (self.tiles[byte] & 0xF0) | t;
        }
        self.depth += 1;
    }

    /// Builder form of [`push`](Self::push).
    #[must_use]
    pub fn with(mut self, t: u8) -> Self {
        self.push(t);
        self
    }

    /// Extend this address by a whole Morton-tile suffix.
    #[must_use]
    pub fn extend(mut self, suffix: &[u8]) -> Self {
        for &t in suffix {
            self.push(t);
        }
        self
    }

    /// Number of leading nibble-levels shared with `other`, capped at the
    /// shallower of the two depths.
    #[must_use]
    pub fn common_prefix_len(&self, other: &Self) -> u8 {
        let limit = self.depth.min(other.depth) as usize;
        let mut n = 0u8;
        while (n as usize) < limit && self.nibble(n as usize) == other.nibble(n as usize) {
            n += 1;
        }
        n
    }

    /// `true` if `self` is a partonomy/spatial **ancestor** of `other`: every
    /// assigned nibble of `self` is a prefix of `other`. A node is its own
    /// ancestor (reflexive). This is the canon's `is_ancestor_of` =
    /// prefix-containment, which — because each nibble is a Morton tile — is
    /// simultaneously spatial containment.
    #[must_use]
    pub fn is_ancestor_of(&self, other: &Self) -> bool {
        self.depth <= other.depth && self.common_prefix_len(other) == self.depth
    }
}

impl core::fmt::Debug for FamilyAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "FamilyAddress(")?;
        for level in 0..self.depth as usize {
            write!(f, "{:x}", self.nibble(level))?;
        }
        write!(f, "/d{})", self.depth)
    }
}

/// The axis-aligned bounding box a Morton subdivision refines within.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    /// Minimum X (e.g. anatomical left→right) and Y (superior→inferior).
    pub min: (f32, f32),
    /// Maximum X and Y.
    pub max: (f32, f32),
}

impl Bounds {
    /// The tightest box covering all `points` (each `(x, y)`). Degenerate
    /// (zero-width) axes are tolerated — [`tile_of`](Self::tile_of) clamps.
    #[must_use]
    pub fn of(points: &[(f32, f32)]) -> Self {
        let mut min = (f32::INFINITY, f32::INFINITY);
        let mut max = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for &(x, y) in points {
            min.0 = min.0.min(x);
            min.1 = min.1.min(y);
            max.0 = max.0.max(x);
            max.1 = max.1.max(y);
        }
        if !min.0.is_finite() {
            min = (0.0, 0.0);
            max = (0.0, 0.0);
        }
        Self { min, max }
    }

    /// The 4×4 Morton tile a point falls into within this box.
    #[must_use]
    pub fn tile_of(&self, p: (f32, f32)) -> u8 {
        tile(
            self.axis_cell(p.0, self.min.0, self.max.0),
            self.axis_cell(p.1, self.min.1, self.max.1),
        )
    }

    /// The sub-box of one 4×4 cell (`tx, ty` each `0..4`).
    #[must_use]
    pub fn subdivide(&self, t: u8) -> Self {
        let (tx, ty) = untile(t);
        let wx = (self.max.0 - self.min.0) / f32::from(AXIS_CELLS);
        let wy = (self.max.1 - self.min.1) / f32::from(AXIS_CELLS);
        let minx = self.min.0 + f32::from(tx) * wx;
        let miny = self.min.1 + f32::from(ty) * wy;
        Self {
            min: (minx, miny),
            max: (minx + wx, miny + wy),
        }
    }

    fn axis_cell(&self, v: f32, lo: f32, hi: f32) -> u8 {
        let span = hi - lo;
        if span <= f32::EPSILON {
            return 0;
        }
        let frac = ((v - lo) / span) * f32::from(AXIS_CELLS);
        let idx = frac.floor() as i32;
        idx.clamp(0, i32::from(AXIS_CELLS) - 1) as u8
    }
}

/// Maximum Morton-tile suffix length assigned to separate one group of
/// siblings. A cap (not a limit reached in practice for the curated atlas);
/// if siblings still collide at this depth the caller's uniqueness test
/// fails, signalling two bones with coincident rest-pose centroids to fix.
pub const MAX_SUFFIX_LEVELS: usize = 8;

/// Assign each sibling a Morton-tile suffix by descending the 4×4 quadtree
/// of their `centroids` until every sibling occupies a distinct cell.
///
/// Returned `suffixes[i]` is the nibble path for `centroids[i]`, refining its
/// parent's address. Because all suffixes descend the *same* quadtree, the
/// shared prefix of two siblings is their spatial proximity — the Morton
/// locality property. Siblings in the same coarse cell recurse deeper; lone
/// siblings stop early (variable-length suffixes = the pyramid adapting to
/// where children cluster).
///
/// A single child (or none) gets an empty suffix — there is nothing to
/// disambiguate.
#[must_use]
pub fn assign_morton_suffixes(centroids: &[(f32, f32)]) -> Vec<Vec<u8>> {
    let mut out = vec![Vec::new(); centroids.len()];
    if centroids.len() > 1 {
        let all: Vec<usize> = (0..centroids.len()).collect();
        refine(centroids, &all, Bounds::of(centroids), 0, &mut out);
    }
    out
}

fn refine(
    centroids: &[(f32, f32)],
    idxs: &[usize],
    bounds: Bounds,
    level: usize,
    out: &mut [Vec<u8>],
) {
    if idxs.len() <= 1 || level >= MAX_SUFFIX_LEVELS {
        return;
    }
    // Bucket the siblings by their 4×4 tile in this box.
    let mut buckets: [Vec<usize>; 16] = Default::default();
    for &i in idxs {
        buckets[bounds.tile_of(centroids[i]) as usize].push(i);
    }
    for (t, members) in buckets.into_iter().enumerate() {
        if members.is_empty() {
            continue;
        }
        let t = t as u8;
        for &i in &members {
            out[i].push(t);
        }
        if members.len() > 1 {
            refine(centroids, &members, bounds.subdivide(t), level + 1, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_roundtrips_all_16() {
        for n in 0u8..16 {
            let (x, y) = untile(n);
            assert!(x < 4 && y < 4);
            assert_eq!(tile(x, y), n);
        }
    }

    #[test]
    fn tier_is_level_shift_two() {
        assert_eq!(tier(0), 0);
        assert_eq!(tier(3), 0);
        assert_eq!(tier(4), 1);
        assert_eq!(tier(11), 2); // HEEL/HIP/TWIG boundary: level 11 -> tier 2 (TWIG)
    }

    #[test]
    fn push_and_nibble_are_inverse() {
        let path = [0x3u8, 0xA, 0x0, 0xF, 0x5, 0x9];
        let a = FamilyAddress::from_nibbles(&path);
        assert_eq!(a.depth(), path.len() as u8);
        for (level, &n) in path.iter().enumerate() {
            assert_eq!(a.nibble(level), n);
        }
    }

    #[test]
    fn ancestor_is_prefix_containment() {
        let parent = FamilyAddress::from_nibbles(&[0x1, 0x2]);
        let child = FamilyAddress::from_nibbles(&[0x1, 0x2, 0x9, 0x4]);
        let cousin = FamilyAddress::from_nibbles(&[0x1, 0x3]);
        assert!(parent.is_ancestor_of(&child));
        assert!(parent.is_ancestor_of(&parent)); // reflexive
        assert!(!child.is_ancestor_of(&parent)); // deeper is not ancestor
        assert!(!cousin.is_ancestor_of(&child));
    }

    #[test]
    fn uniform_16_byte_key_width() {
        assert_eq!(LEVELS, 32);
        assert_eq!(KEY_BYTES, 16);
        let full = FamilyAddress::from_nibbles(&[0xF; LEVELS]);
        assert_eq!(full.depth(), 32);
        assert_eq!(full.bytes(), [0xFF; KEY_BYTES]);
    }

    #[test]
    fn morton_suffixes_are_distinct_and_local() {
        // Four points in four quadrants -> one-nibble distinct suffixes.
        let pts = [(0.0f32, 0.0f32), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        let s = assign_morton_suffixes(&pts);
        assert_eq!(s.len(), 4);
        let set: std::collections::HashSet<_> = s.iter().collect();
        assert_eq!(set.len(), 4, "all suffixes distinct");
        for suf in &s {
            assert_eq!(suf.len(), 1, "one level separates four quadrants");
        }
    }

    #[test]
    fn colinear_siblings_descend_deeper() {
        // 8 colinear points along Y must still separate (vertebral-column case).
        let pts: Vec<(f32, f32)> = (0..8).map(|i| (0.0, i as f32)).collect();
        let s = assign_morton_suffixes(&pts);
        let set: std::collections::HashSet<_> = s.iter().collect();
        assert_eq!(
            set.len(),
            8,
            "colinear siblings all separate via deeper Morton levels"
        );
    }
}
