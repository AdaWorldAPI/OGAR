//! `ogar-ro` — Relation Ontology predicates as a callable [`Vocabulary`].
//!
//! # What this is
//!
//! A curated palette of RO/BFO binary relation predicates (`part_of`,
//! `has_part`, `regulates`, `capable_of`, …), each minted as an
//! [`ogar_loco::FnIndex`] byte in the domain range. An elixir-shaped
//! template or a graph-reasoning body can therefore ASSERT a typed edge
//! `subject R object` the exact same way a Blockly body asserts arithmetic —
//! `Call = (predicate : subject, object)` — riding the identical stack-ABI,
//! [`LaneShape`] carving, and stored-node round-trip every sibling
//! vocabulary shares. One content classid, one 512-byte node, up to
//! [`LaneShape::calls_per_function`] relation calls per body.
//!
//! # Why relations are statements, not expressions (W-RO-2)
//!
//! A relation call ACTS — it asserts an edge — it does not compute a value
//! for something else to consume. So [`RelationVocabulary::domain_pushes_result`]
//! answers `Some(false)` for every minted predicate, exactly the shape
//! [`ogar_loco::statement_bounds`] already expects of a non-pushing call: a
//! statement of relation calls closes at each one, and a body of N
//! assertions segments into N statements with zero new machinery. This
//! confirms the wishlist finding directly, in running code, rather than by
//! assertion: `pushes_result` was never RO-specific plumbing to add — the
//! shared-core column already covers exactly this shape.
//!
//! # Basin-local targets (W-RO-3)
//!
//! A relation's operands are not literal numbers and not nested function
//! indices: they are IDs into a target codebook scoped to the basin the
//! call's own body lives under (medcare-gotham's `EdgeSlots`, or any sibling
//! consumer's equivalent). [`RelationVocabulary::domain_value_codebook`]
//! declares [`RELATION_TARGET_CODEBOOK`] for every minted predicate — the
//! new [`ogar_loco::vocabulary::Vocabulary::value_codebook`] seam this crate
//! is the first user of, so a renderer or oracle-schema generator knows to
//! resolve a relation call's operand bytes against the basin's own target
//! table instead of guessing "probably a literal."
//!
//! # Callability was never required (W-RO-4)
//!
//! [`CheckedVocabulary`](ogar_loco::CheckedVocabulary) proves the sharing
//! discipline and the shape invariant — nothing more. It carries no
//! "is this vocabulary actually invoked at runtime" marker, and none is
//! needed here: `ogar-ro` is validated and read exactly like any other
//! palette, whether or not any consumer ever executes a relation call. A
//! callability flag would be a second, redundant proof for a property the
//! type never claimed in the first place.
//!
//! # `LaneShape`/`CascadeShape` unification (W-RO-5) — NOT decided here
//!
//! `ogar_loco::LaneShape` (`Pairs`/`Triples`/`Quads`, 6×(u8:u8) /
//! 4×(u8:u8:u8) / 3×(u8:u8:u8:u8)) is bit-for-bit the same carving as
//! lance-graph-contract's `CascadeShape` (`G6D2`/`G4D3`/`G3D4`). The
//! duplication is deliberate, not an oversight — `ogar-loco` is zero-dep and
//! must stay that way (see its crate docs), while `lance-graph-contract`
//! sits in a different repo with its own release cadence. Collapsing the two
//! into one shared type would mean either OGAR depends on a lance-graph
//! crate (inverting the "OGAR is the producer, lance-graph consumes" seam) or
//! lance-graph depends on `ogar-loco` for a type it already has under its own
//! name (a needless coupling for a type that never changes shape). Neither
//! direction is this crate's call to make — it is a cross-repo dependency
//! decision, filed here as a named, unresolved wishlist item (W-RO-5) rather
//! than quietly picked. Until an operator ruling lands, the two types stay
//! independently defined and mechanically identical, exactly as they are
//! today.
//!
//! # Provenance
//!
//! Predicate CURIEs cross-reference the Relation Ontology (RO) and Basic
//! Formal Ontology (BFO) — both public, CC-BY-licensed reference
//! ontologies — via [`ogar_obo::Namespace::Ro`]. This crate mints ZERO rows
//! in the shared `ogar_vocab` codebook: the relation-body content classid
//! lives inside the already-reserved `ogar_vocab::ConceptDomain::Ontology`
//! (`0x03XX`), one slot past `ogar_obo`'s own RO term-node concept
//! (`0x0305`), the same plug-and-play posture every palette uses — the
//! Blockly palette (`0x1717`) is declared in `blockly-rs`, its own consumer.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use ogar_loco::{
    BODY_BYTES, CLASSID_BYTES, CONTENT_SLOTS, Call, FnIndex, FunctionBody, LaneShape,
    MAX_VALUES_PER_CALL, PAYLOAD_BYTES_PER_SLOT, SLOT_STRIDE, VALUE_SLAB_LEN, ValueCodebook,
    call_in_slab,
};

use ogar_loco::{RegistryError, Vocabulary, VocabularyRegistry};

/// The relation-body content classid's concept id — one slot past
/// `ogar_obo::Namespace::Ro`'s term-node concept (`0x0305`) inside the
/// shared `Ontology` domain (`0x03XX`). Authoritative HERE, never minted in
/// `ogar_vocab`'s codebook — the codebook carries shared concepts, never a
/// palette's private reading of a body.
pub const RELATION_BODY_CONCEPT_ID: u16 = 0x0306;

/// The full V3 render classid under a consumer's app prefix — canon-high
/// `(concept << 16) | app_prefix`, the same idiom every sibling vocabulary
/// uses (`ogar_vocab::render_classid`).
#[must_use]
pub const fn relation_body_render_classid(app_prefix: u16) -> u32 {
    ((RELATION_BODY_CONCEPT_ID as u32) << 16) | (app_prefix as u32)
}

/// The single basin-local codebook every minted predicate's operands resolve
/// against (W-RO-3). One codebook, not one per predicate: a relation body's
/// subject and object bytes are both IDs into the SAME basin-scoped target
/// table, regardless of which predicate names the edge — mirroring
/// medcare-gotham's `EdgeSlots` shape (one target space, many predicate
/// types over it). `id` is basin-local and carries no meaning outside the
/// basin that owns it; only `name` is stable across basins.
pub const RELATION_TARGET_CODEBOOK: ValueCodebook = ValueCodebook {
    id: 0,
    name: "relation_target",
};

/// One RO/BFO predicate this palette mints, paired with its real ontology
/// CURIE for cross-reference and its canonical mnemonic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RelationPredicate {
    /// The minted [`FnIndex`] byte.
    pub index: FnIndex,
    /// The canonical mnemonic (matches [`Vocabulary::name`]'s answer).
    pub name: &'static str,
    /// The real ontology CURIE this predicate names — `RO:*` for Relation
    /// Ontology terms, `BFO:*` for Basic Formal Ontology terms (mereology
    /// predates RO and still lives in BFO upstream).
    pub curie: &'static str,
}

macro_rules! relation_palette {
    ($( $slot:expr => $ident:ident, $name:literal, $curie:literal );+ $(;)?) => {
        $(
            #[doc = concat!("`", $name, "` (`", $curie, "`) — the minted [`FnIndex`] slot.")]
            pub const $ident: FnIndex = FnIndex($slot);
        )+

        /// Every predicate this palette mints, in slot order — the
        /// enumeration hook a consumer uses to inherit the full set instead
        /// of hand-maintaining a parallel list.
        pub const RELATIONS: &[RelationPredicate] = &[
            $(
                RelationPredicate { index: $ident, name: $name, curie: $curie },
            )+
        ];
    };
}

relation_palette! {
    0x90 => IS_A,                     "is_a",                     "RO:0002331";
    0x91 => PART_OF,                  "part_of",                  "BFO:0000050";
    0x92 => HAS_PART,                 "has_part",                 "BFO:0000051";
    0x93 => OVERLAPS,                 "overlaps",                 "RO:0002131";
    0x94 => DEVELOPS_FROM,            "develops_from",            "RO:0002202";
    0x95 => REGULATES,                "regulates",                "RO:0002211";
    0x96 => NEGATIVELY_REGULATES,     "negatively_regulates",     "RO:0002212";
    0x97 => POSITIVELY_REGULATES,     "positively_regulates",     "RO:0002213";
    0x98 => CAPABLE_OF,               "capable_of",               "RO:0002215";
    0x99 => CAPABLE_OF_PART_OF,       "capable_of_part_of",       "RO:0002216";
    0x9A => IMMEDIATELY_PRECEDED_BY,  "immediately_preceded_by",  "RO:0002090";
    0x9B => LOCATED_IN,               "located_in",               "RO:0001025";
    0x9C => HAS_PARTICIPANT,          "has_participant",          "RO:0000057";
    0x9D => DERIVES_FROM,             "derives_from",             "RO:0001000";
    0x9E => HAS_PHENOTYPE,            "has_phenotype",            "RO:0002200";
    0x9F => DISEASE_HAS_LOCATION,     "disease_has_location",     "RO:0004026";
}

/// The RO/BFO relation palette as an `ogar-loco` [`Vocabulary`].
///
/// Every minted predicate is a **binary assertion**: it pops two operands
/// (subject, object), branches to nothing (`body_refs = 0` — a relation is a
/// leaf, never a nested body), and pushes nothing (W-RO-2). Bytes above the
/// mint (`0xA0..=0xFF`) are reserved, not allocated — the same posture the
/// Blockly palette takes for its unminted device families: refused rather
/// than guessed, until a consumer needs the next predicate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RelationVocabulary;

impl RelationVocabulary {
    fn minted(f: FnIndex) -> bool {
        RELATIONS.iter().any(|r| r.index == f)
    }
}

impl Vocabulary for RelationVocabulary {
    fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
        Self::minted(f).then_some(2)
    }

    fn domain_body_refs(&self, _f: FnIndex) -> u8 {
        0
    }

    fn domain_pushes_result(&self, f: FnIndex) -> Option<bool> {
        // W-RO-2: a relation acts, it does not compute a value for a caller
        // to consume — `Some(false)`, never `Some(true)`, for every minted
        // predicate.
        Self::minted(f).then_some(false)
    }

    fn domain_name(&self, f: FnIndex) -> Option<&'static str> {
        RELATIONS.iter().find(|r| r.index == f).map(|r| r.name)
    }

    fn domain_value_codebook(&self, f: FnIndex) -> Option<ValueCodebook> {
        Self::minted(f).then_some(RELATION_TARGET_CODEBOOK)
    }
}

// ── Plug-and-play ───────────────────────────────────────────────────────────

/// Validate this palette and plug it into a consumer's
/// [`VocabularyRegistry`] under [`RELATION_BODY_CONCEPT_ID`] — the USB
/// handshake for this device, identical in shape to every other palette's
/// own `plug_into`.
///
/// A consumer deps whichever vocabulary crates it needs and calls each
/// one's `plug_into` at boot; a stored relation node then resolves its
/// table through `registry.resolve_classid(classid)` with no consumer-side
/// knowledge that RO exists as a special case.
///
/// # Errors
///
/// [`RegistryError::ConceptTaken`] if something already claimed the
/// relation-body concept.
///
/// # Panics
///
/// Never in practice: [`RelationVocabulary`] conformance is pinned by this
/// crate's own tests, so `validate` cannot fail here.
pub fn plug_into(registry: &mut VocabularyRegistry) -> Result<(), RegistryError> {
    let checked = ogar_loco::vocabulary::conformance::validate(RelationVocabulary)
        .expect("RelationVocabulary conforms; pinned by this crate's tests");
    registry.plug(RELATION_BODY_CONCEPT_ID, &checked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ogar_loco::DOMAIN_FLOOR;
    use ogar_loco::vocabulary::conformance;

    #[test]
    fn the_relation_vocabulary_conforms_to_the_sharing_discipline() {
        assert_eq!(conformance::check(&RelationVocabulary), Ok(()));
    }

    #[test]
    fn every_minted_predicate_is_a_binary_non_pushing_leaf_assertion() {
        let v = RelationVocabulary;
        for r in RELATIONS {
            assert_eq!(v.stack_arity(r.index), Some(2), "{} arity", r.name);
            assert_eq!(v.body_refs(r.index), 0, "{} body_refs", r.name);
            assert!(!v.branches(r.index), "{} must not branch", r.name);
            assert_eq!(
                v.pushes_result(r.index),
                Some(false),
                "{} must not push (W-RO-2)",
                r.name
            );
            assert_eq!(v.name(r.index), Some(r.name));
            assert_eq!(
                v.value_codebook(r.index),
                Some(RELATION_TARGET_CODEBOOK),
                "{} must declare the target codebook (W-RO-3)",
                r.name
            );
        }
    }

    #[test]
    fn an_unminted_domain_byte_is_refused_not_guessed() {
        let v = RelationVocabulary;
        let unminted = FnIndex(0xA0);
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
        assert!(seen.len() >= 12, "palette shrank to {}", seen.len());
    }

    #[test]
    fn ro_curies_parse_as_real_ontology_terms() {
        // Cross-reference: every RO: (not BFO:, which predates RO and isn't
        // in ogar_obo::Namespace) predicate CURIE here must parse as a real
        // ogar_obo::TermId under Namespace::Ro — proving this palette names
        // actual RO terms, not invented mnemonics.
        for r in RELATIONS.iter().filter(|r| r.curie.starts_with("RO:")) {
            let term = ogar_obo::TermId::parse(r.curie)
                .unwrap_or_else(|| panic!("{} ({}) does not parse", r.name, r.curie));
            assert_eq!(term.namespace(), ogar_obo::Namespace::Ro, "{}", r.name);
        }
    }

    #[test]
    fn relation_body_classid_lands_one_slot_past_the_ro_term_node_concept() {
        assert_eq!(RELATION_BODY_CONCEPT_ID, 0x0306);
        let id = relation_body_render_classid(0x1000);
        assert_eq!(id, 0x0306_1000);
        assert_eq!(id >> 16, u32::from(RELATION_BODY_CONCEPT_ID));
        assert_eq!(id & 0xFFFF, 0x1000);
    }
}
