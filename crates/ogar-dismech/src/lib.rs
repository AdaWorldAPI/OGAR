//! `ogar-dismech` — classid reservation for DisMech disease-mechanism
//! concepts.
//!
//! # What this is
//!
//! DisMech (Monarch Initiative disease-mechanism knowledge) is transcoded in
//! the public sibling repo `AdaWorldAPI/dismech-rs`. That crate is a
//! zero-dependency bake crate with no OGAR dependency at all — the same
//! posture `ogar-obo` itself takes, so a consumer never has to link OGAR
//! just to read a baked slab. This tiny crate is the *authority-side* half
//! of that plug-and-play contract: it mints [`DISMECH_CONCEPT_ID`] and
//! proves, at compile-test time, that no `ogar-obo` namespace table claims
//! the same id — mirroring `ogar-ro`'s `RELATION_BODY_CONCEPT_ID` pattern
//! exactly (`crates/ogar-ro/src/lib.rs`).
//!
//! # Provenance
//!
//! `dismech-rs` mints ZERO rows in the shared `ogar_vocab` codebook: the
//! DisMech concept classid lives inside the already-reserved
//! `ogar_vocab::ConceptDomain::Ontology` (`0x03XX`), clear of
//! `ogar_obo::registry::OBO_CORE` (`0x0301..=0x0305`), `ogar-ro`'s
//! `RELATION_BODY_CONCEPT_ID` (`0x0306`), the documented private-consumer
//! odd-stride run (`0x0307..=0x031D` live, `0x031F`/`0x0321` retired-not-
//! reused — see `ogar-obo/src/registry.rs`'s `META_STUDY_SPINE` doc comment
//! for the three-attempt history that produced this band), and
//! `ogar_obo::registry::META_STUDY_SPINE` (`0x0340..=0x0347`).

#![warn(missing_docs)]
#![forbid(unsafe_code)]

/// DisMech disease-mechanism concept id (`dismech-rs::pack::DISMECH_CONCEPT`).
///
/// Reserved in the `0x03` Ontology domain, clear of
/// `ogar_obo::registry::OBO_CORE` (0x0301-0x0305),
/// `ogar_ro::RELATION_BODY_CONCEPT_ID` (0x0306), the documented private
/// odd-stride run (0x0307..=0x031D live, 0x031F/0x0321 retired), and
/// `ogar_obo::registry::META_STUDY_SPINE` (0x0340-0x0347).
pub const DISMECH_CONCEPT_ID: u16 = 0x0333;

/// The full V3 render classid under a consumer's app prefix — canon-high
/// `(concept << 16) | app_prefix`, the same idiom every sibling vocabulary
/// uses (`ogar_vocab::render_classid`, `ogar_ro::relation_body_render_classid`).
#[must_use]
pub const fn dismech_render_classid(app_prefix: u16) -> u32 {
    ((DISMECH_CONCEPT_ID as u32) << 16) | (app_prefix as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dismech_concept_id_is_0x0333() {
        assert_eq!(DISMECH_CONCEPT_ID, 0x0333);
        let id = dismech_render_classid(0x1000);
        assert_eq!(id, 0x0333_1000);
        assert_eq!(id >> 16, u32::from(DISMECH_CONCEPT_ID));
        assert_eq!(id & 0xFFFF, 0x1000);
    }
}

#[cfg(test)]
mod concept_id_collision_guard {
    use super::DISMECH_CONCEPT_ID;
    use ogar_obo::registry::{META_STUDY_SPINE, OBO_CORE};

    /// The same guard `ogar-ro` carries for `RELATION_BODY_CONCEPT_ID`,
    /// pointing at `DISMECH_CONCEPT_ID` instead.
    ///
    /// `ogar-obo` cannot see this crate (the dependency runs `ogar-dismech`
    /// → `ogar-obo`), so it can only assert that its own tables stay out of
    /// the reserved band (`ogar-obo/src/registry.rs`'s
    /// `spine_does_not_collide_with_the_core_or_the_reserved_band`). This
    /// side asserts the thing that actually matters: our real constant is
    /// claimed by no `ogar-obo` table.
    ///
    /// **What would make it fail:** any `ogar-obo` table growing a row at
    /// `DISMECH_CONCEPT_ID`.
    #[test]
    fn no_ogar_obo_namespace_claims_our_concept_id() {
        for s in OBO_CORE.specs().iter().chain(META_STUDY_SPINE.specs()) {
            assert_ne!(
                s.concept_id, DISMECH_CONCEPT_ID,
                "ogar-obo namespace {} claims {:#06X}, which is \
                 DISMECH_CONCEPT_ID — a baked {} row and a dismech-rs \
                 concept row would be the same classid",
                s.prefix, DISMECH_CONCEPT_ID, s.prefix
            );
        }
    }

    /// Anti-vacuity: the guard above compares against tables that are
    /// actually populated. A pair of empty tables would pass it while
    /// proving nothing.
    #[test]
    fn the_tables_the_guard_checks_are_non_empty() {
        assert!(OBO_CORE.len() >= 5, "core table must be populated");
        assert!(META_STUDY_SPINE.len() >= 8, "spine table must be populated");
    }

    /// `DISMECH_CONCEPT_ID` must stay inside the `0x03` Ontology domain, and
    /// clear of both the documented private-consumer odd-stride run
    /// (live through 0x031D, retired through 0x0321) and `META_STUDY_SPINE`
    /// (0x0340..=0x0347) — the two bands `ogar-obo/src/registry.rs`
    /// documents as the reason `META_STUDY_SPINE` itself took three
    /// attempts to place.
    #[test]
    fn stays_in_the_0x03_ontology_domain_clear_of_documented_bands() {
        assert_eq!(DISMECH_CONCEPT_ID >> 8, 0x03);
        assert!(
            DISMECH_CONCEPT_ID > 0x0321,
            "must clear the documented private-consumer odd-stride run \
             (live through 0x031D, retired through 0x0321)"
        );
        assert!(
            DISMECH_CONCEPT_ID < 0x0340,
            "must clear META_STUDY_SPINE (0x0340..=0x0347)"
        );
    }
}
