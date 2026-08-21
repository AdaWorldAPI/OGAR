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

//! # The second band: the search, as a thinking pattern
//!
//! Slots `0xA3..=0xA9` mint the **constraint-propagation verbs** — kept in
//! their own band so the 19 above stay the closed *measured* set that
//! mirrors upstream, and carrying `LOCAL:` CURIEs because they are ours: a
//! `dismech:` prefix would assert a provenance upstream never gave.
//!
//! The pattern is Sudoku, and the word is mechanical rather than decorative:
//! resolution is **elimination**, not similarity. A cell with one remaining
//! candidate is forced ([`NAKED_SINGLE`]); a referent that can sit in only
//! one cell of its unit forces that cell **even while it still holds other
//! candidates** ([`HIDDEN_SINGLE`]) — which is the case where the free text
//! does not know what it names. The eindeutigkeit is a property of the unit,
//! never of the cell read alone. [`ELIMINATE`] is what makes cardinality a
//! moving state rather than a static count: a term with two candidates today
//! has one after a neighbour resolves.
//!
//! ## The residue is an overlay, and that is a cost argument
//!
//! A search leaves a 3-bit band per cell it touched ([`residue_band`]).
//! Those samples are collected **at the graph's own addresses, in separate
//! tables** — never merged down into the graph.
//!
//! The shape is eye tracking. You do not record what the eye *saw* — that is
//! the whole scene, once per viewer. You record *where it looked*: a sparse
//! trace in scene coordinates. So "1:1" is the **addressing**, not the
//! occupancy: one shared, unmodified scene; one sparse trace per search.
//! Writing the trace into each subject instead would give every subject a
//! partial copy of the ontology's structure, which does not stay affordable.
//! It is also why three bits suffice — a fixation sample is tiny by
//! construction.
//!
//! **One-way, structurally.** The overlay reads the graph; the graph never
//! reads the overlay. That is why no search op declares
//! [`DISMECH_TARGET_CODEBOOK`]: their operands are candidate sets and
//! computed bands, never basin-scoped ids, so a residue value has no path to
//! be mistaken for a graph address.
//!
//! ## It is not evidence — it is where to look
//!
//! Propagation *forces* a cell given the constraints; it does not prove it.
//! Put the wrong constraints in and it will force wrong cells with full
//! confidence. The value is not epistemic but search-economic: against tens
//! of thousands of edges the question is rarely "is this true" and almost
//! always "which of these is even worth opening". Confirmation needs a
//! channel the trace did not travel — a second, independently curated source
//! is one; the trace itself never is.
//!
//! ## Pothole → rung degradation → revision
//!
//! Two of [`residue_band`]'s four outcomes are **potholes**, and the mapping
//! IS the degradation:
//!
//! - **0 candidates** — nothing addressable. The gap names its own cell,
//!   which is the reach-out hook: a failure that says what is missing points
//!   at what to fetch, instead of silently deriving nothing.
//! - **`>= diffuse_floor`** — the term does not discriminate. This is
//!   hub-shaped in the exact sense the consumer's own throttle already
//!   means: a term with too many attachments is barred rather than guessed
//!   at. `diffuse_floor` is therefore the same *kind* of quantity as that
//!   throttle's hub threshold — calibrated per corpus, never a shipped
//!   constant.
//!
//! Revision is downstream and elsewhere: this band produces the degraded
//! rung; NARS revision consumes it.
//!
//! ## The rung is the layer, not the payload
//!
//! Projecting rungs as a stack of alpha layers costs zero bits here: each
//! rung is its own table over the same address space, and the 3-bit sample
//! is what sits *inside* one layer. Do not try to carry a rung ordinal in
//! the band — the consumer's band is 3 bits (8 values) and its owner pins,
//! explicitly, that its ordinals must never be mapped onto the rung ladder's.
//! They are unrelated enums that share four variant names.

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

// ── The search band: the Sudoku thinking pattern ────────────────────────────

/// One search operation this palette mints — the constraint-propagation
/// verbs, kept in their own band so the 19 predicates above stay the closed
/// *measured* set that mirrors upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisMechSearchOp {
    /// The minted [`FnIndex`] byte.
    pub index: FnIndex,
    /// The canonical mnemonic (matches [`Vocabulary::name`]'s answer).
    pub name: &'static str,
    /// The `LOCAL:*` CURIE. **Not** `dismech:` — these verbs are ours, not
    /// upstream's; claiming the upstream namespace for them would assert a
    /// provenance upstream never gave. This crate's own module doc records
    /// the `ogar-ro` precedent for exactly this situation (`LOCAL:` where no
    /// external term carries the semantics; the CURIE records provenance,
    /// never a second dispatch key).
    pub curie: &'static str,
    /// Operands popped before the call.
    pub arity: u8,
    /// Function indices carried in the call's value bytes.
    pub bodies: u8,
}

/// `candidates` — the candidate referent set for one cell, drawn from a
/// **named** union of ontologies. The union must be named because
/// cardinality is only meaningful relative to it: unique in one namespace
/// and plural across the union is the same term, two answers.
pub const CANDIDATES: FnIndex = FnIndex(0xA3);
/// `fold_xref` — collapse candidates that are one referent wearing two
/// addresses. **Ordering is load-bearing:** counting before folding reports
/// a bookkeeping artifact as a fork.
pub const FOLD_XREF: FnIndex = FnIndex(0xA4);
/// `naked_single` — exactly one candidate remains in the cell, so the cell
/// is forced. The plain resolution case.
pub const NAKED_SINGLE: FnIndex = FnIndex(0xA5);
/// `hidden_single` — the referent can sit in only one cell of the unit, so
/// that cell is forced *even while it still holds other candidates*. This is
/// the case where the text does not know what it names: the eindeutigkeit is
/// a property of the unit, never of the cell read alone.
pub const HIDDEN_SINGLE: FnIndex = FnIndex(0xA6);
/// `eliminate` — propagate one resolution by striking it from its peers.
/// The step that makes cardinality a moving state rather than a static count.
pub const ELIMINATE: FnIndex = FnIndex(0xA7);
/// `fork` — branch a bounded candidate set into arms that are all kept.
/// **Not `IF_ELSE`:** a conditional discards the untaken arm, a fork does
/// not — that retention is what makes the result a counterfactual instead
/// of a choice.
pub const FORK: FnIndex = FnIndex(0xA8);
/// `residue_band` — push the 3-bit reasoning-band ordinal for the cell's
/// current resolution state. See [`residue_band`] for the mapping and
/// [`SECOND_ORDER_BAND`] for the overlay-about-the-overlay.
pub const RESIDUE_BAND: FnIndex = FnIndex(0xA9);

/// Every search operation this palette mints, in slot order.
pub const SEARCH_OPS: &[DisMechSearchOp] = &[
    DisMechSearchOp {
        index: CANDIDATES,
        name: "candidates",
        curie: "LOCAL:candidates",
        arity: 0,
        bodies: 0,
    },
    DisMechSearchOp {
        index: FOLD_XREF,
        name: "fold_xref",
        curie: "LOCAL:fold_xref",
        arity: 1,
        bodies: 0,
    },
    DisMechSearchOp {
        index: NAKED_SINGLE,
        name: "naked_single",
        curie: "LOCAL:naked_single",
        arity: 1,
        bodies: 0,
    },
    DisMechSearchOp {
        index: HIDDEN_SINGLE,
        name: "hidden_single",
        curie: "LOCAL:hidden_single",
        arity: 2,
        bodies: 0,
    },
    DisMechSearchOp {
        index: ELIMINATE,
        name: "eliminate",
        curie: "LOCAL:eliminate",
        arity: 2,
        bodies: 0,
    },
    DisMechSearchOp {
        index: FORK,
        name: "fork",
        curie: "LOCAL:fork",
        arity: 1,
        bodies: 2,
    },
    DisMechSearchOp {
        index: RESIDUE_BAND,
        name: "residue_band",
        curie: "LOCAL:residue_band",
        arity: 1,
        bodies: 0,
    },
];

/// Position lookup over the search band — the same index computation
/// [`by_index`] performs over the predicate band.
#[must_use]
pub const fn search_op_by_index(f: FnIndex) -> Option<&'static DisMechSearchOp> {
    let i = f.0.wrapping_sub(CANDIDATES.0) as usize;
    if i < SEARCH_OPS.len() {
        Some(&SEARCH_OPS[i])
    } else {
        None
    }
}

// ── The residue: what a search leaves behind ────────────────────────────────

/// The band ordinal for **thinking about thinking** — an overlay whose
/// subject is another overlay, not the graph.
///
/// `6` is `Meta` in the consumer's 3-bit reasoning band ("reasoning ABOUT
/// reasoning/evidence/revision"). Named here so a second-order collector
/// does not re-derive it from a magic number.
pub const SECOND_ORDER_BAND: u8 = 6;

/// Map a cell's candidate cardinality to a 3-bit reasoning-band ordinal.
///
/// **Count only AFTER [`FOLD_XREF`].** Two addresses of one referent are not
/// two referents; counting first reports a bookkeeping artifact as a fork.
///
/// | candidates | ordinal | reading |
/// |---|---:|---|
/// | 0 | 0 (`Surface`) | nothing addressable — an absence, not a claim |
/// | 1 | 2 (`Relation`) | resolved: a named relation between addressed nodes |
/// | `2..diffuse_floor` | 4 (`Counterfactual`) | bounded arms, each interventionable |
/// | `>= diffuse_floor` | 1 (`Association`) | the term does not discriminate |
///
/// Two is a *question*; fifty is noise. Where between them the fork stops
/// being a fork is `diffuse_floor` — **a required parameter with no default
/// on purpose.** No measured value exists, and shipping a constant would
/// launder a guess into an apparent finding; a caller reads it off its own
/// data. A floor of `<= 2` collapses the counterfactual band to empty, a
/// legitimate strict policy, pinned as such.
///
/// # Binding, without a dependency edge
///
/// The ordinal is written by the consumer via its own
/// `with_reasoning_band(from_bits_3(n))`. This crate does not depend on the
/// edge type and must not: that field's owner states it is *"set ONLY by an
/// explicit `with_reasoning_band()` call"* with nothing deriving it
/// internally. Producing the ordinal here and writing it there is what
/// honours that — a derivation inside the edge crate would break it.
///
/// **Provenance gate:** on the consumer's v1 layout those three bits were
/// temporal bits 9-11, so a v1 row can read a non-zero band it never meant.
/// Version-gate rows of unknown provenance.
#[must_use]
pub const fn residue_band(candidates: u32, diffuse_floor: u32) -> u8 {
    if candidates == 0 {
        0 // Surface
    } else if candidates == 1 {
        2 // Relation
    } else if candidates < diffuse_floor {
        4 // Counterfactual
    } else {
        1 // Association
    }
}

/// The DisMech causal predicate palette as an `ogar-loco` [`Vocabulary`].
///
/// Every minted predicate is a **binary assertion**: it pops two operands
/// (subject, object), branches to nothing (`body_refs = 0` — a causal edge
/// is a leaf, never a nested body), and pushes nothing — it asserts an
/// edge, it does not compute a value for a caller to consume, mirroring
/// `ogar_ro::RelationVocabulary`'s W-RO-2 reasoning exactly.
///
/// The search band (`0xA3..=0xA9`) answers differently — see [`SEARCH_OPS`].
/// Bytes past BOTH bands (`0xAA..=0xFF`) are reserved, not allocated —
/// refused rather than guessed, until a consumer needs the next slot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DisMechVocabulary;

impl DisMechVocabulary {
    fn minted(f: FnIndex) -> bool {
        RELATIONS.iter().any(|r| r.index == f)
    }
}

impl Vocabulary for DisMechVocabulary {
    fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
        if Self::minted(f) {
            Some(2)
        } else {
            search_op_by_index(f).map(|o| o.arity)
        }
    }

    fn domain_body_refs(&self, f: FnIndex) -> u8 {
        search_op_by_index(f).map_or(0, |o| o.bodies)
    }

    fn domain_pushes_result(&self, f: FnIndex) -> Option<bool> {
        // A causal predicate acts — it asserts an edge — it does not push a
        // value for a caller to consume. `Some(false)`, never `Some(true)`,
        // for every minted predicate.
        if Self::minted(f) {
            return Some(false);
        }
        // A search op COMPUTES, so it pushes — except `fork`, which branches.
        search_op_by_index(f).map(|o| o.bodies == 0)
    }

    fn domain_name(&self, f: FnIndex) -> Option<&'static str> {
        RELATIONS
            .iter()
            .find(|r| r.index == f)
            .map(|r| r.name)
            .or_else(|| search_op_by_index(f).map(|o| o.name))
    }

    fn domain_value_codebook(&self, f: FnIndex) -> Option<ValueCodebook> {
        // Predicates resolve operands against the basin's target table.
        // Search ops do NOT: their operands are candidate sets and computed
        // bands, never basin-scoped ids. Declaring a codebook for them would
        // invite a residue value being read as a graph address — the
        // contamination this band exists to avoid.
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
    fn the_search_band_is_contiguous_and_never_overlaps_the_predicate_band() {
        assert_eq!(SEARCH_OPS.len(), 7);
        for (i, o) in SEARCH_OPS.iter().enumerate() {
            assert_eq!(o.index.0, CANDIDATES.0 + u8::try_from(i).unwrap());
            assert_eq!(search_op_by_index(o.index), Some(o));
            assert!(
                by_index(o.index).is_none(),
                "{} collides with the predicate band",
                o.name
            );
        }
        // The bands abut with no gap: the search band starts one past the
        // last predicate. A gap would leave an unowned byte between them.
        let last_pred = RELATIONS.iter().map(|r| r.index.0).max().unwrap();
        assert_eq!(CANDIDATES.0, last_pred + 1);
    }

    #[test]
    fn a_search_op_computes_where_a_predicate_asserts() {
        let v = DisMechVocabulary;
        // Anti-vacuity: the two groups must actually DIFFER, so assert both
        // halves rather than only the new one.
        for r in RELATIONS {
            assert_eq!(v.pushes_result(r.index), Some(false), "{}", r.name);
        }
        for o in SEARCH_OPS.iter().filter(|o| o.bodies == 0) {
            assert_eq!(v.pushes_result(o.index), Some(true), "{}", o.name);
        }
        // FORK is the ONE exception and the filter above excludes it, so it
        // must be pinned here or nothing pins it: a branching call is
        // non-pushing, like the shared core's own control calls. Left
        // unpinned, a change making `fork` push would pass every test while
        // silently re-segmenting statements (`pushes_result` is what
        // `statement_bounds` computes stack depth from).
        assert_eq!(
            v.pushes_result(FORK),
            Some(false),
            "a branching call must not also push"
        );
    }

    #[test]
    fn no_search_op_declares_the_target_codebook() {
        let v = DisMechVocabulary;
        // Anti-vacuity: predicates DO declare it, so this cannot pass by
        // everything answering None.
        assert_eq!(
            v.value_codebook(CAUSES),
            Some(DISMECH_TARGET_CODEBOOK),
            "the predicate band must still declare it"
        );
        for o in SEARCH_OPS {
            assert_eq!(
                v.value_codebook(o.index),
                None,
                "{} must not expose a basin address to residue",
                o.name
            );
        }
    }

    #[test]
    fn fork_branches_where_every_predicate_is_a_leaf() {
        let v = DisMechVocabulary;
        assert_eq!(v.body_refs(FORK), 2, "a fork keeps BOTH arms");
        assert!(v.branches(FORK));
        for o in SEARCH_OPS.iter().filter(|o| o.index != FORK) {
            assert!(!v.branches(o.index), "{} must not branch", o.name);
        }
        for r in RELATIONS {
            assert!(!v.branches(r.index), "{} must not branch", r.name);
        }
    }

    #[test]
    fn residue_band_maps_the_cardinality_ladder() {
        // floor = 8 keeps all four outcomes reachable, so no arm is vacuous.
        assert_eq!(residue_band(0, 8), 0, "no candidate -> Surface");
        assert_eq!(residue_band(1, 8), 2, "unique -> Relation");
        assert_eq!(residue_band(2, 8), 4, "a fork -> Counterfactual");
        assert_eq!(residue_band(7, 8), 4, "still bounded -> Counterfactual");
        assert_eq!(residue_band(8, 8), 1, "at the floor -> Association");
        assert_eq!(residue_band(50, 8), 1, "diffuse -> Association");
    }

    #[test]
    fn a_strict_floor_collapses_the_counterfactual_band_shut() {
        // The documented strict policy: floor <= 2 admits no fork at all.
        for n in 2..64 {
            assert_ne!(
                residue_band(n, 2),
                4,
                "floor 2 must admit no counterfactual, but {n} did"
            );
        }
        // ...and it is a POLICY, not a broken function: raise the floor and
        // the same input forks.
        assert_eq!(residue_band(2, 3), 4);
    }

    #[test]
    fn thinking_about_thinking_is_never_a_cardinality_outcome() {
        // SECOND_ORDER_BAND is a LAYER, not something counting can produce.
        // If a future mapping could emit it, meta would collapse into the
        // first order and the overlay-about-the-overlay would lose its own
        // address.
        for floor in 0..16 {
            for n in 0..256 {
                assert_ne!(residue_band(n, floor), SECOND_ORDER_BAND);
            }
        }
    }

    #[test]
    fn an_unminted_domain_byte_is_refused_not_guessed() {
        let v = DisMechVocabulary;
        // The first byte past BOTH bands — recomputed rather than hardcoded,
        // so extending a palette moves the probe instead of failing it.
        // Re-pinned when the search band landed: 0xA3 used to be the probe
        // and is now `candidates`, so reading only RELATIONS would have
        // asserted "unminted" about a minted byte.
        let unminted = FnIndex(SEARCH_OPS.iter().map(|o| o.index.0).max().unwrap() + 1);
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
