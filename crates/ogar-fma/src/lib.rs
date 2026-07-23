//! `ogar-fma` — the **FMA anatomy structure resolver**: the public
//! name→structure reference surface for organs, joints, systems, and tissues,
//! the soft-tissue complement to the rigid [`ogar-fma-skeleton`] bone atlas.
//!
//! # What this is
//!
//! A consumer (a clinical reasoning layer, an imaging viewer) knows an
//! anatomical site by *name* — "synovial joint", "kidney", "lung". It needs
//! two things that are **public reference**, not PHI:
//!
//! 1. the **canonical Anatomy classid** (`0x0AXX`) the name routes on, so the
//!    node addresses into the same partonomy every imaging modality registers
//!    against (`ConceptDomain::Anatomy` — deliberately NOT `Health`, per the
//!    firewall split in `ogar_vocab`: reference structure is public, a clinical
//!    *finding about* it on a named patient is PHI);
//! 2. the **FMA cross-reference** (`FMA:xxxxx`) where confidently known — the
//!    key q2 `/helix` uses to zoom a surfel render to the affected structure.
//!
//! This crate is the single public place that mapping lives. A consumer pulls
//! it (`ogar_fma::resolve("synovial joint")`); it never hand-maintains a
//! parallel FMA table nor fabricates ids (the honesty fence below).
//!
//! # Relationship to `ogar-fma-skeleton`
//!
//! [`ogar-fma-skeleton`] is the **bone** atlas — the ~206 rigid clamped
//! convergence anchors, each a `bone` (`0x0A03`) cascade-path node with a
//! Located 3-axis address. This crate is the **non-bone** structures (organs,
//! joints, systems, tissues) that a disease localizes to, keyed by name and
//! routed on the broader Anatomy concepts (`anatomical_structure` `0x0A01` /
//! `joint` `0x0A04`). A [`FmaPartition::Bone`] result defers to the skeleton
//! crate for the spatial address; here it carries only the concept routing.
//!
//! # Honesty fence (inherited from `ogar-fma-skeleton`)
//!
//! FMA numeric ids are filled **only where a canonical, stable FMA id is
//! confidently known** (the same practice the skeleton atlas uses — femur
//! `FMA:9611`, tibia `FMA:12856`). Where not, [`FmaStructure::fma_id`] is
//! `None` — a captured loose end resolved against the q2 `/helix` structure
//! table, never fabricated here. What is rigorous is the *structure*: the
//! name→partition→classid routing and the laterality, which carry regardless
//! of whether the numeric cross-reference is yet pinned.
//!
//! [`ogar-fma-skeleton`]: https://docs.rs/ogar-fma-skeleton

#![warn(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The partonomic kind of an anatomical structure — the axis that selects
/// which canonical Anatomy concept (`0x0AXX`) the structure routes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum FmaPartition {
    /// A whole organ (kidney, lung, pancreas, thyroid gland). Routes on
    /// `anatomical_structure` (`0x0A01`) — FMA's universal root kind.
    Organ,
    /// An articulation between bones (synovial joint, first MTP joint). Routes
    /// on `joint` (`0x0A04`).
    Joint,
    /// An anatomical *system* / branching tree (systemic arterial system,
    /// bronchial tree). Routes on `anatomical_structure` (`0x0A01`).
    System,
    /// A tissue (bone marrow). Routes on `anatomical_structure` (`0x0A01`).
    Tissue,
    /// A skeletal element. Routes on `bone` (`0x0A03`); the *spatial* address
    /// lives in [`ogar-fma-skeleton`](crate), not here.
    Bone,
}

impl FmaPartition {
    /// The canonical Anatomy concept id (`0x0AXX`) this partition routes on —
    /// the hi-u16 (`domain:concept`) half of a rendered classid.
    #[must_use]
    pub const fn concept_id(self) -> u16 {
        match self {
            FmaPartition::Joint => ogar_vocab::class_ids::JOINT,
            FmaPartition::Bone => ogar_vocab::class_ids::BONE,
            FmaPartition::Organ | FmaPartition::System | FmaPartition::Tissue => {
                ogar_vocab::class_ids::ANATOMICAL_STRUCTURE
            }
        }
    }
}

/// Laterality of a structure — carried so a consumer's per-side finding
/// (a left vs right kidney) addresses the correct instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum Laterality {
    /// A single midline / systemic structure (pancreas, systemic arterial system).
    Unpaired,
    /// A paired structure that a finding may localize to one side (kidney, lung).
    Paired,
    /// A structure characteristically involved on both sides at once
    /// (symmetric small-joint synovitis).
    Bilateral,
}

/// A resolved public FMA anatomical structure — the reference row a consumer
/// pulls to route an anatomical site into the Anatomy partonomy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FmaStructure {
    /// The canonical structure name (the `/helix` structure-index key).
    pub name: &'static str,
    /// The FMA numeric cross-reference (`FMA:xxxxx`) where confidently known;
    /// `None` = a loose end resolved against `/helix`, never fabricated.
    pub fma_id: Option<u32>,
    /// The partonomic kind — selects the routing concept.
    pub partition: FmaPartition,
    /// Characteristic laterality.
    pub laterality: Laterality,
}

impl FmaStructure {
    /// The canonical Anatomy concept id (`0x0AXX`) this structure routes on.
    #[must_use]
    pub const fn concept_id(&self) -> u16 {
        self.partition.concept_id()
    }

    /// The full V3 render classid for this structure under a consumer's app
    /// prefix (the lo-u16 render skin): `(concept as u32) << 16 | app_prefix`.
    /// The hi-u16 is the shared `domain:concept` canon; the lo-u16 is the
    /// per-consumer classview. Mirrors `ogar_vocab::render_classid`.
    #[must_use]
    pub const fn render_classid(&self, app_prefix: u16) -> u32 {
        ((self.concept_id() as u32) << 16) | (app_prefix as u32)
    }
}

/// The curated public structure atlas — the non-bone anatomical sites a
/// clinical reasoning layer localizes to. Extended as `/helix` pins more
/// canonical FMA ids; the name→partition→concept routing is stable regardless.
const ATLAS: &[FmaStructure] = &[
    // Organs — route on `anatomical_structure` (0x0A01). FMA ids filled where
    // canonical + stable (same practice as the skeleton atlas's bone ids).
    FmaStructure {
        name: "kidney",
        fma_id: Some(7203),
        partition: FmaPartition::Organ,
        laterality: Laterality::Paired,
    },
    FmaStructure {
        name: "lung",
        fma_id: Some(7195),
        partition: FmaPartition::Organ,
        laterality: Laterality::Paired,
    },
    FmaStructure {
        name: "pancreas",
        fma_id: Some(7198),
        partition: FmaPartition::Organ,
        laterality: Laterality::Unpaired,
    },
    FmaStructure {
        name: "thyroid gland",
        fma_id: Some(9603),
        partition: FmaPartition::Organ,
        laterality: Laterality::Unpaired,
    },
    // Joints — route on `joint` (0x0A04).
    FmaStructure {
        name: "synovial joint",
        fma_id: Some(7501),
        partition: FmaPartition::Joint,
        laterality: Laterality::Bilateral,
    },
    FmaStructure {
        // The classic podagra site. Numeric FMA id is a loose end (many
        // per-toe MTP-joint ids exist; not fabricating the aggregate here).
        name: "first metatarsophalangeal joint",
        fma_id: None,
        partition: FmaPartition::Joint,
        laterality: Laterality::Unpaired,
    },
    // Systems / trees — route on `anatomical_structure` (0x0A01).
    FmaStructure {
        name: "systemic arterial system",
        fma_id: None,
        partition: FmaPartition::System,
        laterality: Laterality::Unpaired,
    },
    FmaStructure {
        name: "bronchial tree",
        fma_id: None,
        partition: FmaPartition::System,
        laterality: Laterality::Bilateral,
    },
    // Tissue — routes on `anatomical_structure` (0x0A01).
    FmaStructure {
        name: "bone marrow",
        fma_id: None,
        partition: FmaPartition::Tissue,
        laterality: Laterality::Unpaired,
    },
];

/// Resolve an anatomical-structure name to its public FMA reference row.
/// Case-insensitive on ASCII. `None` for a name not (yet) in the atlas — a
/// consumer treats that as a captured loose end, exactly as it would an
/// unfilled `fma_id`.
///
/// ```
/// let knee = ogar_fma::resolve("synovial joint").unwrap();
/// assert_eq!(knee.concept_id(), ogar_vocab::class_ids::JOINT);
/// assert_eq!(knee.fma_id, Some(7501));
/// // routes into the Anatomy domain, never Health/PHI:
/// assert_eq!(
///     ogar_vocab::canonical_concept_domain(knee.concept_id()),
///     ogar_vocab::ConceptDomain::Anatomy,
/// );
/// ```
#[must_use]
pub fn resolve(name: &str) -> Option<&'static FmaStructure> {
    ATLAS
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(name.trim()))
}

/// Every structure in the public atlas — for a consumer building a complete
/// name→classid index once at startup.
#[must_use]
pub fn atlas() -> &'static [FmaStructure] {
    ATLAS
}

#[cfg(test)]
mod tests {
    use super::*;
    use ogar_vocab::{ConceptDomain, canonical_concept_domain, class_ids};

    #[test]
    fn every_structure_routes_into_the_anatomy_domain() {
        // The firewall split: an FMA structure is public Anatomy, never Health.
        for s in atlas() {
            assert_eq!(
                canonical_concept_domain(s.concept_id()),
                ConceptDomain::Anatomy,
                "{} escaped the Anatomy domain",
                s.name,
            );
        }
    }

    #[test]
    fn joints_route_on_joint_organs_on_structure() {
        assert_eq!(
            resolve("synovial joint").unwrap().concept_id(),
            class_ids::JOINT,
        );
        assert_eq!(
            resolve("first metatarsophalangeal joint")
                .unwrap()
                .concept_id(),
            class_ids::JOINT,
        );
        assert_eq!(
            resolve("kidney").unwrap().concept_id(),
            class_ids::ANATOMICAL_STRUCTURE,
        );
        assert_eq!(
            resolve("bronchial tree").unwrap().concept_id(),
            class_ids::ANATOMICAL_STRUCTURE,
        );
    }

    #[test]
    fn resolve_is_case_and_whitespace_insensitive() {
        assert_eq!(resolve("  KIDNEY ").unwrap().name, "kidney");
        assert!(resolve("Synovial Joint").is_some());
        assert!(resolve("no such structure").is_none());
    }

    #[test]
    fn confident_fma_ids_are_filled_uncertain_are_loose_ends() {
        // Filled where canonical + stable; None (honest loose end) otherwise.
        assert_eq!(resolve("kidney").unwrap().fma_id, Some(7203));
        assert_eq!(resolve("synovial joint").unwrap().fma_id, Some(7501));
        assert_eq!(resolve("bone marrow").unwrap().fma_id, None);
        assert_eq!(resolve("systemic arterial system").unwrap().fma_id, None,);
    }

    #[test]
    fn render_classid_packs_concept_high_appid_low() {
        let s = resolve("synovial joint").unwrap();
        // q2 render skin 0x1000: hi-u16 = 0x0A04 (joint), lo-u16 = 0x1000.
        assert_eq!(s.render_classid(0x1000), 0x0A04_1000);
        assert_eq!((s.render_classid(0x1000) >> 16) as u16, class_ids::JOINT);
    }

    #[test]
    fn structure_names_are_unique() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for s in atlas() {
            assert!(seen.insert(s.name), "duplicate structure {}", s.name);
        }
    }
}
