//! `ogar-dismech` — DisMech disease-mechanism concepts, as a classid
//! reservation AND a causal predicate palette.
//!
//! # What this is
//!
//! DisMech (Monarch Initiative disease-mechanism knowledge) is transcoded in
//! the public sibling repo `AdaWorldAPI/dismech-rs`. That crate is a
//! zero-dependency bake crate with no OGAR dependency at all — the same
//! posture `ogar-obo` itself takes, so a consumer never has to link OGAR
//! just to read a baked slab. This tiny crate is the *authority-side* half
//! of that plug-and-play contract: it mints [`DISMECH_CONCEPT_ID`] and a
//! palette of 19 causal predicates, and proves, at compile-test time, that
//! no `ogar-obo` namespace table claims the same id — mirroring `ogar-ro`'s
//! `RELATION_BODY_CONCEPT_ID` / `RelationVocabulary` pattern exactly
//! (`crates/ogar-ro/src/lib.rs`).
//!
//! # The 19 predicates — a measured, closed set
//!
//! The names below are the complete predicate vocabulary emitted by the
//! upstream DisMech causal-graph builder
//! (`monarch-initiative/dismech`, `src/dismech/graph.py`): `causes`,
//! `leads_to`, `triggers`, `exacerbates`, `predisposes_to`,
//! `protects_against`, `modulates`, `influences`, `targets`, `treats`,
//! `models`, `partially_models`, `fails_to_model`, `perturbs`, `measures`,
//! `rescues`, `readout`, `contributes_to`, `variant_of`. Nineteen names,
//! minted contiguously at [`DOMAIN_FLOOR`](ogar_loco::DOMAIN_FLOOR)
//! (`0x90`) through `0xA2`.
//!
//! # Why `dismech:*`, not `RO:*` (the whole decision)
//!
//! `ogar-ro` cross-references real Relation Ontology / Basic Formal
//! Ontology terms. This palette deliberately does **not** — every one of
//! the 19 CURIEs uses the `dismech:` prefix instead, for a reason grounded
//! in what upstream itself does, not a stylistic preference:
//!
//! - Upstream's own SEPIO exporter declines to claim RO terms for its
//!   causal predicates. It defines its own namespace constants —
//!   `CAUSALLY_UPSTREAM_OF = "dismech:causally_upstream_of"` and
//!   `HAS_PATHOPHYSIOLOGY = "dismech:has_pathophysiology"` — rather than
//!   reaching for an RO CURIE that already exists for a similar-sounding
//!   relation.
//! - Across the entire upstream `src/` tree there is exactly **one** `RO:`
//!   literal, and it names a piece of corpus data (a gene-disease
//!   association), never one of the graph-builder's own predicate names.
//!
//! So minting these 19 under `RO:` CURIEs — even where a plausible-looking
//! match exists (`causes` → `RO:0002411` is the tempting one) — would
//! assert a claim upstream deliberately declined to make. `ogar-ro`'s own
//! precedent covers exactly this situation: its 2026-08-10 entry for
//! `confounds_test` uses the `LOCAL:` prefix (and its `has_interpretation`/
//! `interprets` pair uses `SCTID:`) specifically because *"no RO term
//! carries the [needed] semantics… the byte is the normalized predicate
//! either way; the CURIE records provenance, never a second dispatch key."*
//! The same reasoning applies here, uniformly, to all 19 rows rather than a
//! subset — the palette's provenance signal is a property of the whole set,
//! not a per-row judgment call.
//!
//! **Left open, deliberately:** upstream's SEPIO export uses the label
//! `causally_upstream_of` for its causal family, which is not one of the 19
//! graph-builder predicate names minted here. Whether `dismech:causes` and
//! `dismech:causally_upstream_of` denote the same relation under two labels
//! is **not determined** by this crate — it is recorded as open, not
//! resolved either way.
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

use ogar_loco::{FnIndex, RegistryError, ValueCodebook, Vocabulary, VocabularyRegistry};

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

/// The single basin-local codebook every minted predicate's operands resolve
/// against, mirroring `ogar_ro::RELATION_TARGET_CODEBOOK`. One codebook, not
/// one per predicate: a causal-edge body's subject and object bytes are both
/// IDs into the SAME basin-scoped target table, regardless of which
/// predicate names the edge. `id` is basin-local and carries no meaning
/// outside the basin that owns it; only `name` is stable across basins.
pub const DISMECH_TARGET_CODEBOOK: ValueCodebook = ValueCodebook {
    id: 0,
    name: "dismech_target",
};

/// One DisMech causal predicate this palette mints, paired with its
/// `dismech:`-namespaced CURIE and its canonical mnemonic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisMechPredicate {
    /// The minted [`FnIndex`] byte.
    pub index: FnIndex,
    /// The canonical mnemonic (matches [`Vocabulary::name`]'s answer).
    pub name: &'static str,
    /// The `dismech:*` CURIE — see the module doc for why this is
    /// `dismech:`, never `RO:`.
    pub curie: &'static str,
}

macro_rules! dismech_palette {
    ($( $slot:expr => $ident:ident, $name:literal, $curie:literal );+ $(;)?) => {
        $(
            #[doc = concat!("`", $name, "` (`", $curie, "`) — the minted [`FnIndex`] slot.")]
            pub const $ident: FnIndex = FnIndex($slot);
        )+

        /// Every predicate this palette mints, in slot order — the
        /// enumeration hook a consumer uses to inherit the full set instead
        /// of hand-maintaining a parallel list.
        pub const RELATIONS: &[DisMechPredicate] = &[
            $(
                DisMechPredicate { index: $ident, name: $name, curie: $curie },
            )+
        ];
    };
}

dismech_palette! {
    0x90 => CAUSES,             "causes",             "dismech:causes";
    0x91 => LEADS_TO,           "leads_to",           "dismech:leads_to";
    0x92 => TRIGGERS,           "triggers",           "dismech:triggers";
    0x93 => EXACERBATES,        "exacerbates",        "dismech:exacerbates";
    0x94 => PREDISPOSES_TO,     "predisposes_to",     "dismech:predisposes_to";
    0x95 => PROTECTS_AGAINST,   "protects_against",   "dismech:protects_against";
    0x96 => MODULATES,          "modulates",          "dismech:modulates";
    0x97 => INFLUENCES,         "influences",         "dismech:influences";
    0x98 => TARGETS,            "targets",            "dismech:targets";
    0x99 => TREATS,             "treats",             "dismech:treats";
    0x9A => MODELS,             "models",             "dismech:models";
    0x9B => PARTIALLY_MODELS,   "partially_models",   "dismech:partially_models";
    0x9C => FAILS_TO_MODEL,     "fails_to_model",     "dismech:fails_to_model";
    0x9D => PERTURBS,           "perturbs",           "dismech:perturbs";
    0x9E => MEASURES,           "measures",           "dismech:measures";
    0x9F => RESCUES,            "rescues",            "dismech:rescues";
    0xA0 => READOUT,            "readout",            "dismech:readout";
    0xA1 => CONTRIBUTES_TO,     "contributes_to",     "dismech:contributes_to";
    0xA2 => VARIANT_OF,         "variant_of",         "dismech:variant_of";
}

/// Position lookup over the palette's contiguous slot band — resolving an
/// [`FnIndex`] is an index computation, never a scan.
///
/// The palette mints slots contiguously from [`CAUSES`] (`0x90`) upward, so
/// `RELATIONS[f - 0x90]` IS the row, exactly as `ogar_ro::by_index` reads
/// its own band. A byte below the floor or past the mint is refused, not
/// guessed; contiguity itself is pinned by
/// `the_slot_band_is_contiguous_so_position_is_the_lookup`.
#[must_use]
pub const fn by_index(f: FnIndex) -> Option<&'static DisMechPredicate> {
    let i = f.0.wrapping_sub(CAUSES.0) as usize;
    if i < RELATIONS.len() {
        Some(&RELATIONS[i])
    } else {
        None
    }
}

/// The DisMech causal predicate palette as an `ogar-loco` [`Vocabulary`].
///
/// Every minted predicate is a **binary assertion**: it pops two operands
/// (subject, object), branches to nothing (`body_refs = 0` — a causal edge
/// is a leaf, never a nested body), and pushes nothing — it asserts an
/// edge, it does not compute a value for a caller to consume, mirroring
/// `ogar_ro::RelationVocabulary`'s W-RO-2 reasoning exactly. Bytes above the
/// mint (`0xA3..=0xFF`) are reserved, not allocated — refused rather than
/// guessed, until a consumer needs the next predicate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DisMechVocabulary;

impl DisMechVocabulary {
    fn minted(f: FnIndex) -> bool {
        RELATIONS.iter().any(|r| r.index == f)
    }
}

impl Vocabulary for DisMechVocabulary {
    fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
        Self::minted(f).then_some(2)
    }

    fn domain_body_refs(&self, _f: FnIndex) -> u8 {
        0
    }

    fn domain_pushes_result(&self, f: FnIndex) -> Option<bool> {
        // A causal predicate acts — it asserts an edge — it does not push a
        // value for a caller to consume. `Some(false)`, never `Some(true)`,
        // for every minted predicate.
        Self::minted(f).then_some(false)
    }

    fn domain_name(&self, f: FnIndex) -> Option<&'static str> {
        RELATIONS.iter().find(|r| r.index == f).map(|r| r.name)
    }

    fn domain_value_codebook(&self, f: FnIndex) -> Option<ValueCodebook> {
        Self::minted(f).then_some(DISMECH_TARGET_CODEBOOK)
    }
}

// ── Plug-and-play ───────────────────────────────────────────────────────────

/// Validate this palette and plug it into a consumer's
/// [`VocabularyRegistry`] under [`DISMECH_CONCEPT_ID`] — the USB handshake
/// for this device, identical in shape to `ogar_ro::plug_into` and every
/// other sibling palette's own `plug_into`.
///
/// A consumer deps whichever vocabulary crates it needs and calls each
/// one's `plug_into` at boot; a stored causal-edge node then resolves its
/// table through `registry.resolve_classid(classid)` with no consumer-side
/// knowledge that DisMech exists as a special case.
///
/// # Errors
///
/// [`RegistryError::ConceptTaken`] if something already claimed the
/// DisMech concept.
///
/// # Panics
///
/// Never in practice: [`DisMechVocabulary`] conformance is pinned by this
/// crate's own tests, so `validate` cannot fail here.
pub fn plug_into(registry: &mut VocabularyRegistry) -> Result<(), RegistryError> {
    let checked = ogar_loco::vocabulary::conformance::validate(DisMechVocabulary)
        .expect("DisMechVocabulary conforms; pinned by this crate's tests");
    registry.plug(DISMECH_CONCEPT_ID, &checked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ogar_loco::DOMAIN_FLOOR;
    use ogar_loco::vocabulary::conformance;

    #[test]
    fn dismech_concept_id_is_0x0333() {
        assert_eq!(DISMECH_CONCEPT_ID, 0x0333);
        let id = dismech_render_classid(0x1000);
        assert_eq!(id, 0x0333_1000);
        assert_eq!(id >> 16, u32::from(DISMECH_CONCEPT_ID));
        assert_eq!(id & 0xFFFF, 0x1000);
    }

    #[test]
    fn the_dismech_vocabulary_conforms_to_the_sharing_discipline() {
        assert_eq!(conformance::check(&DisMechVocabulary), Ok(()));
    }

    #[test]
    fn the_palette_mints_exactly_the_19_measured_predicates() {
        assert_eq!(RELATIONS.len(), 19, "the DisMech predicate set is closed");
        assert_eq!(RELATIONS.first().unwrap().index, FnIndex(0x90));
        assert_eq!(RELATIONS.last().unwrap().index, FnIndex(0xA2));
    }

    #[test]
    fn every_minted_predicate_is_a_binary_non_pushing_leaf_assertion() {
        let v = DisMechVocabulary;
        for r in RELATIONS {
            assert_eq!(v.stack_arity(r.index), Some(2), "{} arity", r.name);
            assert_eq!(v.body_refs(r.index), 0, "{} body_refs", r.name);
            assert!(!v.branches(r.index), "{} must not branch", r.name);
            assert_eq!(
                v.pushes_result(r.index),
                Some(false),
                "{} must not push",
                r.name
            );
            assert_eq!(v.name(r.index), Some(r.name));
            assert_eq!(
                v.value_codebook(r.index),
                Some(DISMECH_TARGET_CODEBOOK),
                "{} must declare the target codebook",
                r.name
            );
        }
    }

    #[test]
    fn an_unminted_domain_byte_is_refused_not_guessed() {
        let v = DisMechVocabulary;
        // The first byte PAST the mint — recomputed from the palette rather
        // than hardcoded, so extending the palette moves the probe instead
        // of failing it.
        let unminted = FnIndex(RELATIONS.iter().map(|r| r.index.0).max().unwrap() + 1);
        assert_eq!(v.stack_arity(unminted), None);
        assert_eq!(v.pushes_result(unminted), None);
        assert_eq!(v.value_codebook(unminted), None);
        assert_eq!(v.name(unminted), None);
    }

    #[test]
    fn predicate_slots_are_distinct_and_start_at_the_domain_floor() {
        let mut seen = Vec::new();
        for r in RELATIONS {
            assert!(r.index.0 >= DOMAIN_FLOOR, "{} below the floor", r.name);
            assert!(!seen.contains(&r.index.0), "{} duplicates a slot", r.name);
            seen.push(r.index.0);
        }
        assert_eq!(seen.len(), 19, "palette drifted from 19 predicates");
    }

    #[test]
    fn the_slot_band_is_contiguous_so_position_is_the_lookup() {
        // by_index's soundness condition, pinned: slot i sits at CAUSES.0 + i.
        // A future mint that leaves a gap fails HERE, not by misalignment.
        for (i, r) in RELATIONS.iter().enumerate() {
            assert_eq!(
                r.index.0,
                CAUSES.0 + u8::try_from(i).unwrap(),
                "{} breaks the contiguous band",
                r.name
            );
            assert_eq!(
                by_index(r.index),
                Some(r),
                "{} resolves by position",
                r.name
            );
        }
        // refusal at both edges — the lookup can say no
        assert_eq!(by_index(FnIndex(CAUSES.0 - 1)), None, "below the band");
        let past = RELATIONS.last().unwrap().index.0 + 1;
        assert_eq!(by_index(FnIndex(past)), None, "past the mint");
    }

    #[test]
    fn every_curie_uses_the_dismech_namespace_never_ro_or_bfo() {
        // The can-fire half AND the anti-vacuity twin, in one sweep: every
        // one of the 19 CURIEs must start with `dismech:`, and — so a
        // future "helpful" remap onto a lookalike RO/BFO term is caught
        // rather than silently accepted — none may start with `RO:` or
        // `BFO:`.
        for r in RELATIONS {
            assert!(
                r.curie.starts_with("dismech:"),
                "{} ({}) must use the dismech: namespace",
                r.name,
                r.curie
            );
            assert!(
                !r.curie.starts_with("RO:") && !r.curie.starts_with("BFO:"),
                "{} ({}) must NOT be remapped onto an RO/BFO CURIE — see the \
                 module doc for why this palette stays dismech:*",
                r.name,
                r.curie
            );
        }
    }

    #[test]
    fn every_name_is_unique_and_matches_its_curie_suffix() {
        let mut names = Vec::new();
        for r in RELATIONS {
            assert!(!names.contains(&r.name), "{} duplicates a name", r.name);
            names.push(r.name);
            let suffix = r.curie.strip_prefix("dismech:").unwrap_or_else(|| {
                panic!("{} ({}) has no dismech: prefix to strip", r.name, r.curie)
            });
            assert_eq!(
                suffix, r.name,
                "{}'s CURIE suffix must match its mnemonic",
                r.name
            );
        }
    }

    #[test]
    fn the_palette_plugs_into_a_registry_and_refuses_a_second_claim() {
        let mut registry = VocabularyRegistry::new();
        plug_into(&mut registry).expect("first registration must succeed");
        assert_eq!(
            plug_into(&mut registry),
            Err(RegistryError::ConceptTaken {
                concept_id: DISMECH_CONCEPT_ID
            }),
            "a second registration of the same concept must be refused"
        );
    }
}

#[cfg(test)]
mod concept_id_collision_guard {
    use super::DISMECH_CONCEPT_ID;
    use ogar_obo::registry::{META_STUDY_SPINE, OBO_CORE};

    /// The mirror of `ogar_obo::registry`'s band guard, pointing the other way.
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
