// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The Lance Authors

//! `ogar-epistemic` — the **named 24-axis epistemic basis, v3**: the classid
//! reservation and axis catalogue for the `EpistemicBassin24` magnitude
//! register (lance-graph `epistemic_bassin`, the `agree_u4[24] +
//! disagree_u4[24]` pair).
//!
//! # The facet classid names the basis — and versions it
//!
//! Per the operator co-architect ruling (2026-09-01): each nibble of the
//! register is a quantized value on a **named epistemic axis**, never an
//! address, and the facet's 4-byte classid names WHICH 24-axis basis (and
//! which version) the register's lanes are read against. "Epistemic witness
//! v3" is a classid, not a Rust name — so a superseded catalogue is retired
//! by minting the NEXT concept id (v4), never by re-reading old rows under
//! new labels. Append-only, the same discipline as every codebook here.
//!
//! # Where the axes come from
//!
//! Derived from the ruled projection (episodic references / named basis /
//! premise trace / revision) and the ruled lane semantics (lanes carry
//! proprioception and pressure, never empirical truth — exact quantities
//! stay in their exact carriers). Every axis names a pressure GROUNDED in a
//! shipped surface, listed per row in [`AXES`]; none is invented. The
//! catalogue is the session's derivation from those rulings — supersedable
//! by a v4 mint, which is deliberately cheap.
//!
//! Four of the axes are **magnitude twins** of A9 ContextLoci pointer axes
//! (`Quorum`, `Contradiction`, and the TEKAMOLO four collapse to their own
//! group): the locus points at WHO (the agreeing peer, the disagreeing
//! peer); the basis axis carries HOW MUCH. Two readings, one register
//! family, zero overlap in what they store.
//!
//! # What this crate deliberately does NOT do
//!
//! - No shared-codebook rows: [`EPISTEMIC_BASIS_V3_CONCEPT_ID`] lives inside
//!   the already-reserved `Ontology` domain (`0x03XX`), clear of every
//!   neighbour (collision-guarded in tests against `ogar-dismech` and
//!   `ogar-obo`'s bands).
//! - No domain `FnIndex` mints: the six epistemic calls are ogar-loco CORE
//!   (`0x86..0x8B`) and therefore readable in every vocabulary already; a
//!   domain palette (per-axis macros, revision verbs — and per the
//!   rung-per-classid ruling, per-rung palettes) mints later, as content
//!   earns it.
//! - No Hambly-Lyons axis: sigker's path-signature classification is gated
//!   on jc Pillar 11 (DEFERRED) — the red-pillar rule holds for axes as it
//!   does for lanes and mints.

/// The v3 basis concept id, inside the reserved `Ontology` domain (`0x03XX`).
///
/// Clear of: OBO core (`0x0301..=0x0305`), `ogar-ro` (`0x0306`), the
/// private-consumer run (`0x0307..=0x031D`, `0x031F`/`0x0321` retired),
/// DisMech (`0x0333`), and the meta-study spine (`0x0340..=0x0347`).
/// A v4 catalogue mints the NEXT free id; this one is then retired in
/// place, never re-labelled.
pub const EPISTEMIC_BASIS_V3_CONCEPT_ID: u16 = 0x0334;

/// The full V3 render classid under a consumer's app prefix — canon-high
/// (`concept << 16 | app_prefix`), the same compose every codebook uses.
#[must_use]
pub const fn epistemic_basis_render_classid(app_prefix: u16) -> u32 {
    ((EPISTEMIC_BASIS_V3_CONCEPT_ID as u32) << 16) | app_prefix as u32
}

/// Axis count — must equal the register's lane count (`BASIS_AXES` on the
/// lance-graph side; the armed parity check holds the two together).
pub const AXES_LEN: usize = 24;

/// The six axis groups, four axes each — the ruled projection made
/// positional: `group = axis >> 2` is a shift, never a lookup.
pub const GROUPS: [&str; 6] = [
    "set",          // Mengenlehre kind 1 — the inheritance field itself
    "evidence",     // dismech stances
    "derivation",   // Tarski / premise trace
    "field",        // Shannon / EWA proprioception
    "circumstance", // TEKAMOLO frame agreement
    "witness",      // provenance / revision / peers
];

/// The named basis: `(axis, name, group, grounding)` — the grounding names
/// the SHIPPED surface each pressure is measured against, so no axis is a
/// free-floating adjective.
pub const AXES: [(u8, &str, &str, &str); AXES_LEN] = [
    // ── set (0..4): the whale lives here ─────────────────────────────────
    (
        0,
        "IS_A",
        "set",
        "the is_a rail (hhtl path / episodic_basin L1-L3)",
    ),
    (
        1,
        "PART_OF",
        "set",
        "the part_of rail (same rails, mereology axis)",
    ),
    (
        2,
        "TYPICALITY",
        "set",
        "conformance to the basin's Cam96 self_code neighbourhood",
    ),
    (
        3,
        "MISSING_LINK",
        "set",
        "rail-expected relation with silent evidence (asked-but-silent, sweep_ternlog 0x02)",
    ),
    // ── evidence (4..8): dismech stances ─────────────────────────────────
    (
        4,
        "SUPPORT",
        "evidence",
        "dismech_evidence::Supports::Support mass",
    ),
    (
        5,
        "REFUTE",
        "evidence",
        "dismech_evidence::Supports::Refute mass",
    ),
    (
        6,
        "PARTIAL",
        "evidence",
        "dismech_evidence::Supports::Partial mass",
    ),
    (
        7,
        "REPLICATION",
        "evidence",
        "replication pressure across independent evidence rows",
    ),
    // ── derivation (8..12): Tarski / premise trace ───────────────────────
    (
        8,
        "PREMISE",
        "derivation",
        "premise-ancestry soundness (exact depth stays in the ancestry)",
    ),
    (9, "DEDUCTION", "derivation", "NARS deduction-step validity"),
    (
        10,
        "FALSIFIER",
        "derivation",
        "known-falsifier pressure (Tarski's falsifier side)",
    ),
    (
        11,
        "COUNTERFACTUAL",
        "derivation",
        "rung-3 counterfactual pressure (CE64 mantissa -6; never observed truth)",
    ),
    // ── field (12..16): Shannon / EWA ────────────────────────────────────
    (
        12,
        "INFO_GAIN",
        "field",
        "epistemic_bassin::info_gain_u4 over candidate counts",
    ),
    (
        13,
        "TENSION",
        "field",
        "epistemic_bassin::sigma_tension_u4 vs sigma_propagation::pillar_5plus_bound",
    ),
    (
        14,
        "COHERENCE",
        "field",
        "EWA residual coherence with the neighbourhood Sigma",
    ),
    (
        15,
        "AMBIGUITY",
        "field",
        "open candidate-set width (dismech_candidates::Evaluation)",
    ),
    // ── circumstance (16..20): TEKAMOLO agreement ────────────────────────
    (
        16,
        "TEMPORAL",
        "circumstance",
        "children agree on the WHEN frame (Tekamolo tenant / Locus::Temporal twin)",
    ),
    (
        17,
        "KAUSAL",
        "circumstance",
        "children agree on the WHY frame (Locus::Kausal twin)",
    ),
    (
        18,
        "MODAL",
        "circumstance",
        "children agree on the HOW frame (Locus::Modal twin)",
    ),
    (
        19,
        "LOKAL",
        "circumstance",
        "children agree on the WHERE frame (Locus::Lokal twin)",
    ),
    // ── witness (20..24): provenance / revision / peers ──────────────────
    (
        20,
        "PROVENANCE",
        "witness",
        "episodic grounding strength (EpisodicBasin references exist)",
    ),
    (
        21,
        "REVISION",
        "witness",
        "admitted-then-revised pressure (NARS revision; what was admitted and when)",
    ),
    (
        22,
        "QUORUM",
        "witness",
        "peer-agreement mass (magnitude twin of Locus::Quorum)",
    ),
    (
        23,
        "CONTRADICTION",
        "witness",
        "preserved-contradiction depth (magnitude twin of Locus::Contradiction)",
    ),
];

/// Group of an axis — a shift, never a branch.
#[must_use]
pub const fn group_of(axis: u8) -> Option<&'static str> {
    if (axis as usize) < AXES_LEN {
        Some(GROUPS[(axis >> 2) as usize])
    } else {
        None
    }
}

/// Name of an axis, `None` past the basis.
#[must_use]
pub const fn axis_name(axis: u8) -> Option<&'static str> {
    if (axis as usize) < AXES_LEN {
        Some(AXES[axis as usize].1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_is_dense_unique_and_grouped_by_shift() {
        for (i, &(axis, name, group, grounding)) in AXES.iter().enumerate() {
            assert_eq!(axis as usize, i, "axes are dense 0..24, no gaps");
            assert_eq!(group, GROUPS[i >> 2], "group is axis >> 2");
            assert!(
                !grounding.is_empty(),
                "{name}: every axis names its grounding"
            );
        }
        for (i, a) in AXES.iter().enumerate() {
            for b in AXES.iter().skip(i + 1) {
                assert_ne!(a.1, b.1, "axis names are unique");
            }
        }
        assert_eq!(AXES_LEN, 24);
        assert_eq!(GROUPS.len() * 4, AXES_LEN);
    }

    #[test]
    fn the_concept_id_collides_with_no_reserved_neighbour() {
        assert_ne!(
            EPISTEMIC_BASIS_V3_CONCEPT_ID,
            ogar_dismech::DISMECH_CONCEPT_ID
        );
        // OBO core + ogar-ro + the private-consumer run + retired ids.
        assert!(
            !(0x0301..=0x0321).contains(&EPISTEMIC_BASIS_V3_CONCEPT_ID),
            "clear of the OBO/ro/private run"
        );
        // the meta-study spine
        assert!(
            !(0x0340..=0x0347).contains(&EPISTEMIC_BASIS_V3_CONCEPT_ID),
            "clear of META_STUDY_SPINE"
        );
        // inside the reserved Ontology domain at all
        assert_eq!(EPISTEMIC_BASIS_V3_CONCEPT_ID >> 8, 0x03);
    }

    #[test]
    fn render_classid_is_canon_high() {
        let c = epistemic_basis_render_classid(0x0002);
        assert_eq!(c >> 16, EPISTEMIC_BASIS_V3_CONCEPT_ID as u32);
        assert_eq!(c & 0xFFFF, 0x0002);
    }

    #[test]
    fn lookups_answer_inside_the_basis_and_refuse_past_it() {
        assert_eq!(axis_name(0), Some("IS_A"));
        assert_eq!(axis_name(23), Some("CONTRADICTION"));
        assert_eq!(
            axis_name(24),
            None,
            "past the basis is refused, not wrapped"
        );
        assert_eq!(group_of(3), Some("set"));
        assert_eq!(group_of(23), Some("witness"));
        assert_eq!(group_of(24), None);
    }

    /// The red-pillar rule as an assertable absence: no axis claims a
    /// Hambly-Lyons / path-signature quantity while jc Pillar 11 is red.
    #[test]
    fn no_axis_claims_a_red_pillar_quantity() {
        for &(_, name, _, grounding) in AXES.iter() {
            for banned in ["HAMBLY", "LYONS", "SIGNATURE"] {
                assert!(
                    !name.to_uppercase().contains(banned)
                        && !grounding.to_uppercase().contains(banned),
                    "{name}: red-pillar quantity in the catalogue"
                );
            }
        }
    }
}
