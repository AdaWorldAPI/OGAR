//! `guid.rs` — the **brutal, uniform GUID**: every tier is a `[256:256]`
//! `[container : member]` pair.
//!
//! # The one relation, at every scale (operator, 2026-06-23)
//!
//! ```text
//!   [Galaxy : planet]   [country : city]   [school : student]
//!   [bodypart : bone]   [cm² : mm²]         [residue : atom]
//! ```
//!
//! Every `[256:256]` tier is the *same* relation — **`[container : member]`**:
//! the high byte names the coarse container (a 256-codebook), the low byte the
//! member attached within it (another 256-codebook). It is scale-free: the
//! identical algebra addresses a galaxy's planet, a country's city, a
//! bodypart's bone, or a residue's atom. The GUID is nothing but these tiers
//! stacked:
//!
//! ```text
//!   [classid] [HEEL] [HIP] [TWIG] [LEAF]   tiers, each [container:member]
//!    0xAA:CC   x:y    x:y   x:y    family:identity
//!    bytes:    0,1    2,3   4,5    6,7      8,9   (10..16 reserved, deeper refine)
//! ```
//!
//! Containment manifests **two ways that agree** (D-BOTHCASC):
//! - **HHTL = spatial containment** — the `cm²:mm²` axis (coarse body cell ⊃
//!   fine cell), a 2D-Morton refinement of the rest-pose position.
//! - **LEAF = categorical containment** — the `bodypart:bone` axis (group ⊃
//!   instance).
//!
//! Each tier byte is simultaneously a directly-addressable 256-codebook index
//! **and** a 4×4 Morton pyramid (two nibble-tiles) that can carry
//! perturbation-shader refinement. `tier = level >> 2` is uniform end to end —
//! the leaf is just the last HHTL tier, not a special tail.

use crate::morton::{KEY_BYTES, tile};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Number of `[256:256]` tiers in the 16-byte key (`KEY_BYTES / 2` = 8).
pub const TIERS: usize = KEY_BYTES / 2;

/// Tier 0 — the classid (`app:concept`), the routing prefix.
pub const TIER_CLASSID: usize = 0;
/// Tier 1 — HEEL: coarse spatial container:member.
pub const TIER_HEEL: usize = 1;
/// Tier 2 — HIP: finer spatial.
pub const TIER_HIP: usize = 2;
/// Tier 3 — TWIG: finer still.
pub const TIER_TWIG: usize = 3;
/// Tier 4 — LEAF: the categorical `familyNode:identity` (`bodypart:bone`).
pub const TIER_LEAF: usize = 4;

/// One `[256:256]` tier — a `[container : member]` pair. `container` (the high
/// byte) names the coarse cell; `member` (the low byte) the instance attached
/// within it. Each byte is a 256-codebook index and a 4×4 Morton pyramid.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Tier {
    /// The coarse container (high byte) — galaxy / country / bodypart / cm².
    pub container: u8,
    /// The member attached within the container (low byte) — planet / city /
    /// bone / mm².
    pub member: u8,
}

impl Tier {
    /// The tier as its 16-bit value (`container << 8 | member`).
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        ((self.container as u16) << 8) | self.member as u16
    }
}

/// The LEAF tier named in its categorical role: `familyNode:identity` — the
/// `[bodypart : bone]` of the skeleton. `family_node` is the containing group
/// (routing prefix byte); `identity` is the instance attached within it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LeafTile {
    /// The containing family / bodypart / group (a 256-codebook).
    pub family_node: u8,
    /// The instance attached within the family (a 256-codebook).
    pub identity: u8,
}

/// The 16-byte canonical GUID, read as [`TIERS`] `[container:member]` tiers.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Guid {
    bytes: [u8; KEY_BYTES],
}

impl Default for Guid {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Guid {
    /// The all-zero GUID (the bootstrap / default-everything address — the
    /// zero-fallback ladder's base).
    pub const ZERO: Self = Self {
        bytes: [0u8; KEY_BYTES],
    };

    /// Wrap raw key bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; KEY_BYTES]) -> Self {
        Self { bytes }
    }

    /// The raw 16-byte key.
    #[must_use]
    pub const fn bytes(&self) -> [u8; KEY_BYTES] {
        self.bytes
    }

    /// Read tier `i` (`0..TIERS`) as a `[container:member]` pair.
    #[must_use]
    pub const fn tier(&self, i: usize) -> Tier {
        Tier {
            container: self.bytes[2 * i],
            member: self.bytes[2 * i + 1],
        }
    }

    /// Write tier `i`.
    pub const fn set_tier(&mut self, i: usize, t: Tier) {
        self.bytes[2 * i] = t.container;
        self.bytes[2 * i + 1] = t.member;
    }

    /// The classid (tier 0) as a `u16` (`app << 8 | concept` byte order — here
    /// the canonical concept, e.g. `0x0A03` for `bone`).
    #[must_use]
    pub const fn classid(&self) -> u16 {
        self.tier(TIER_CLASSID).as_u16()
    }

    /// The HEEL spatial tier.
    #[must_use]
    pub const fn heel(&self) -> Tier {
        self.tier(TIER_HEEL)
    }

    /// The LEAF tier as `familyNode:identity`.
    #[must_use]
    pub const fn leaf(&self) -> LeafTile {
        let t = self.tier(TIER_LEAF);
        LeafTile {
            family_node: t.container,
            identity: t.member,
        }
    }

    /// The family-node (the leaf container — `bodypart`).
    #[must_use]
    pub const fn family_node(&self) -> u8 {
        self.bytes[2 * TIER_LEAF]
    }

    /// The identity (the leaf member — the specific `bone`), attached within
    /// the family node.
    #[must_use]
    pub const fn identity(&self) -> u8 {
        self.bytes[2 * TIER_LEAF + 1]
    }

    /// `true` if `self` and `other` are in the **same family**: every tier up
    /// to and including the leaf's `family_node` agrees; only the leaf
    /// `identity` differs. This is "identity attached to family node" — the
    /// family node is the routing prefix, the identity discriminates within.
    #[must_use]
    pub fn same_family(&self, other: &Self) -> bool {
        self.bytes[..2 * TIER_LEAF + 1] == other.bytes[..2 * TIER_LEAF + 1]
    }
}

impl core::fmt::Debug for Guid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Guid[")?;
        for i in 0..TIERS {
            let t = self.tier(i);
            let sep = if i == 0 { "" } else { " " };
            write!(f, "{sep}{:02x}:{:02x}", t.container, t.member)?;
        }
        write!(f, "]")
    }
}

/// Fold an 8-bit-per-axis coronal position `(x, y)` into a spatial
/// `[container : member]` tier: the **coarse** byte (`cm²`) is the top 4 bits
/// of each axis, 2D-Morton-interleaved; the **fine** byte (`mm²`) is the
/// bottom 4 bits. Laterality (the X axis) therefore lives in the even bits of
/// both bytes — recoverable from the address prefix before any value decode.
#[must_use]
pub fn spatial_tier(x: u8, y: u8) -> Tier {
    let coarse = (tile((x >> 6) & 3, (y >> 6) & 3) << 4) | tile((x >> 4) & 3, (y >> 4) & 3);
    let fine = (tile((x >> 2) & 3, (y >> 2) & 3) << 4) | tile(x & 3, y & 3);
    Tier {
        container: coarse,
        member: fine,
    }
}

/// The **mode** of an HHTL tier — the operator's "heel" distinction
/// (2026-06-23).
///
/// The *same* `[container:member]` tier algebra reads two ways:
///
/// - [`Located`](Self::Located) — HEEL is the **literal heel**: a real
///   position (`heel : muscle`). Preserves location, Cesium-style (the tile
///   anchors to an actual coordinate). HHTL = spatial Morton ([`spatial_tier`]).
///   This is the **bone** case — bones ARE the position anchor.
/// - [`Cascade`](Self::Cascade) — HEEL is a classification rung in a scale-free
///   containment cascade (`universe … atom`, `Anatomy … PapillaryMuscle`).
///   **No spatial address** — pure ontology. HHTL = named codebook entries
///   (the self-speaking GUID, `ANAT0001-CARD-HERT-LVNT-PAPMUS-…`). This is the
///   **soft-tissue / muscle** case — classified, then projected onto.
///
/// Both modes share the LEAF `familyNode:identity` and the same 16-byte key;
/// the mode is a reading of the HHTL block, not a different structure.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HhtlMode {
    /// Spatial / position-preserving (Cesium). The bone case.
    Located,
    /// Ontological cascade, no spatial address. The muscle case.
    Cascade,
}

/// An ontology tier from two named codebook bytes — `[parentClass : childClass]`
/// (e.g. `[Heart : LeftVentricle]`). The Cascade-mode counterpart of
/// [`spatial_tier`]; it is just a [`Tier`] with the names made explicit.
#[must_use]
pub const fn ontology_tier(parent_class: u8, child_class: u8) -> Tier {
    Tier {
        container: parent_class,
        member: child_class,
    }
}

// ── Relations live in the family-node addressing, NOT a fixed edge block ──
//
// The pre-family-node taxonomy carved relations into a fixed `12 in-family +
// 4 out-of-family` edge block (the papillary-muscle worked example shows it).
// That is **superseded by family nodes** (operator, 2026-06-23). With the
// `[container:member]` tier model, relations ARE the addressing:
//
//   - **local relation** ⇔ shared family prefix — `Guid::same_family` (same
//     tiers up to the leaf `family_node`); no slots needed, the address says it.
//   - **cross relation** ⇔ a reference to another family node's `Guid` (the
//     target's address), unbounded — not a fixed-4 inherited-interface carve.
//
// So there is no `EdgeBlock` here. A node's neighbourhood is read from the key
// space (prefix containment + family-node references), keeping the node a pure
// `key + value` block with no special edge taxonomy.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_roundtrips_through_bytes() {
        let mut g = Guid::ZERO;
        g.set_tier(
            TIER_CLASSID,
            Tier {
                container: 0x0A,
                member: 0x03,
            },
        );
        g.set_tier(
            TIER_LEAF,
            Tier {
                container: 0x42,
                member: 0x07,
            },
        );
        assert_eq!(g.classid(), 0x0A03);
        assert_eq!(g.family_node(), 0x42);
        assert_eq!(g.identity(), 0x07);
        assert_eq!(
            g.leaf(),
            LeafTile {
                family_node: 0x42,
                identity: 0x07
            }
        );
    }

    #[test]
    fn same_family_means_identity_attached() {
        let mut a = Guid::ZERO;
        a.set_tier(
            TIER_CLASSID,
            Tier {
                container: 0x0A,
                member: 0x03,
            },
        );
        a.set_tier(
            TIER_LEAF,
            Tier {
                container: 0x42,
                member: 0x01,
            },
        );
        let mut b = a; // same family, different identity
        b.set_tier(
            TIER_LEAF,
            Tier {
                container: 0x42,
                member: 0x02,
            },
        );
        let mut c = a; // different family
        c.set_tier(
            TIER_LEAF,
            Tier {
                container: 0x43,
                member: 0x01,
            },
        );
        assert!(a.same_family(&b), "same family_node, identity attached");
        assert!(!a.same_family(&c), "different family_node");
        assert_ne!(a.bytes(), b.bytes());
    }

    #[test]
    fn spatial_tier_encodes_laterality_in_x_bits() {
        // Two points differing only in X (laterality) must differ in the tier;
        // the difference lives in the even (X) bit-positions.
        let left = spatial_tier(0x20, 0x80);
        let right = spatial_tier(0xE0, 0x80);
        assert_ne!(left, right);
        // Same X, differing Y differ too (cranio-caudal).
        assert_ne!(spatial_tier(0x80, 0x20), spatial_tier(0x80, 0xE0));
    }

    #[test]
    fn uniform_eight_tiers_over_sixteen_bytes() {
        assert_eq!(TIERS, 8);
        let g = Guid::from_bytes([0xAB; KEY_BYTES]);
        for i in 0..TIERS {
            assert_eq!(
                g.tier(i),
                Tier {
                    container: 0xAB,
                    member: 0xAB
                }
            );
        }
    }

    #[test]
    fn located_and_cascade_share_leaf_and_key_width() {
        // Same classid + leaf; HHTL filled two ways. The mode is a reading of
        // the HHTL block, not a different structure — both are 16-byte keys
        // with the same family:identity leaf.
        let classid = Tier {
            container: 0x0A,
            member: 0x03,
        };
        let leaf = Tier {
            container: 0x42,
            member: 0x07,
        };

        let mut located = Guid::ZERO; // Cesium: HHTL = spatial Morton
        located.set_tier(TIER_CLASSID, classid);
        located.set_tier(TIER_HEEL, spatial_tier(0xC0, 0x40));
        located.set_tier(TIER_LEAF, leaf);

        let mut cascade = Guid::ZERO; // ontology: HHTL = named codebook path
        cascade.set_tier(TIER_CLASSID, classid);
        cascade.set_tier(TIER_HEEL, ontology_tier(0x01, 0x02)); // e.g. Heart:LeftVentricle
        cascade.set_tier(TIER_LEAF, leaf);

        assert_eq!(located.leaf(), cascade.leaf(), "same family:identity");
        assert_eq!(located.classid(), cascade.classid());
        assert_eq!(located.bytes().len(), cascade.bytes().len());
        assert_ne!(located.heel(), cascade.heel(), "HHTL differs by mode");
        let _ = HhtlMode::Located;
        let _ = HhtlMode::Cascade;
    }

    #[test]
    fn relations_are_family_prefix_not_an_edge_block() {
        // Superseding the 12+4 edge taxonomy: a local relation is a shared
        // family prefix; a cross relation is a reference to another node's Guid.
        let classid = Tier {
            container: 0x0A,
            member: 0x03,
        };
        let mut a = Guid::ZERO;
        a.set_tier(TIER_CLASSID, classid);
        a.set_tier(
            TIER_LEAF,
            Tier {
                container: 0x42,
                member: 0x01,
            },
        );
        let mut sibling = a; // same family node, identity attached
        sibling.set_tier(
            TIER_LEAF,
            Tier {
                container: 0x42,
                member: 0x02,
            },
        );
        let mut other_family = a;
        other_family.set_tier(
            TIER_LEAF,
            Tier {
                container: 0x99,
                member: 0x01,
            },
        );

        assert!(
            a.same_family(&sibling),
            "local relation = shared family prefix"
        );
        assert!(!a.same_family(&other_family));
        // A cross relation is just the other node's address (a Guid), unbounded.
        let _cross_ref: Guid = other_family;
    }
}
