//! `ogar-blockly` — the **visual block-programming vocabulary**: one 256-slot
//! palette of commands and concepts, shared by every block frontend.
//!
//! # What this is
//!
//! A block editor (Blockly, Scratch, or any successor) renders *tiles*. This
//! crate is the single public place the tile vocabulary lives: **one byte per
//! command**, 256 slots, deduplicated across frontends so that two editors
//! rendering the same operation land on the **same palette slot** rather than
//! on two ids that merely sit in the same domain.
//!
//! That convergence is the entire point. `logic_compare[LT]` (Blockly) and
//! `operator_lt` (Scratch) are ONE slot — [`FnIndex::LT`]. `operator_mathop`
//! (one Scratch block with a dropdown) fans out to the same slots that
//! `math_single` + `math_trig` (two Blockly blocks) fan out to. The palette is
//! where the two vocabularies actually meet.
//!
//! # The ABI lives one level down, in `ogar-loco`
//!
//! The call encoding this palette rides on — `Call = (function : value)`
//! rails, [`LaneShape`] carvings, [`FunctionBody`] budgets, the stored-node
//! round-trip, the constant pool, the [`Program`](ogar_loco::Program)
//! reference rules, and the shared computational core's arity tables — is the
//! **vocabulary-agnostic surface** every sibling codebook shares
//! (elixir-shaped templates and flow frontends are next in line). It was
//! hoisted from this crate into [`ogar_loco`]; this crate re-exports that
//! surface unchanged, so existing consumers keep compiling, and adds what is
//! genuinely Blockly/Scratch:
//!
//! - the palette constants' *meanings* (documented on [`FnIndex`]'s
//!   associated constants, re-exported from the core where the shared
//!   computational range is defined once for every vocabulary),
//! - the Blocks concept domain (`0x17XX`) and its ONE concept id (`0x1717`),
//!   which names the PALETTE; the node shapes it stores into are
//!   [`ogar_loco::LocoConcept`]'s (`0x1701` / `0x1702`), globally owned,
//! - the [`SoaSplit`] storage partitioning,
//! - [`BlocklyVocabulary`], this palette's [`Vocabulary`] implementation.
//!
//! # Classid routing — the reserved Blocks domain (`0x17XX`)
//!
//! `ogar_vocab` reserves `ConceptDomain::Blocks` (`0x17XX`) and ships ZERO
//! concept rows there. This crate is the authoritative home for the ids inside
//! that domain — the same plug-and-play posture as `ogar-obo` over
//! `ConceptDomain::Ontology`: only consumers that dep `ogar-blockly` compile
//! them, so ERP / clinical / project consumers never pull a block vocabulary
//! they have no use for.
//!
//! # Storage shape — inventory SoA + N content SoAs, split by function
//!
//! Functions are not pooled into one table. There is an **inventory** SoA (the
//! registry: which functions exist, addressed by identity) and **N content**
//! SoAs, **partitioned by function** — see [`SoaSplit`]. That partitioning is
//! the V3 mailbox doctrine, not a storage preference: one function = one owner
//! = its own SoA, so every write is owned and no singleton table accumulates
//! writers.
//!
//! The split belongs to the SUBSTRATE, not to this palette: both partitions
//! resolve to [`ogar_loco::LocoConcept`], because a registry row and a
//! function body are the same two shapes for every vocabulary.
//!
//! # Provenance fence (load-bearing, not decorative)
//!
//! Every palette entry here is derived from **permissively-licensed or
//! specification** sources — the Apache-2.0 Blockly block definitions and the
//! Apache-2.0 `scratch-blocks` block definitions — and **never** by
//! transcribing a GPL/AGPL implementation (`scratch-vm` is AGPL; it is not a
//! source for this table). That is what keeps this public codebook
//! unencumbered while a GPL consumer links it freely, and it is why the GPL
//! boundary can sit entirely inside the consumer repo instead of propagating
//! here. Cross-ref: `ogar_vocab::ConceptDomain::Blocks`, `docs/DISCOVERY-MAP.md`
//! `D-BLOCKS-DOMAIN`.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use ogar_vocab::ConceptDomain;

// ── The shared surface, re-exported unchanged ───────────────────────────────
// The ABI hoist (ogar-loco) must be invisible to existing consumers: every
// name this crate exported before the hoist is re-exported here, same paths,
// same semantics. New surface (Vocabulary, node/pool/program modules) is NOT
// re-exported — a consumer that wants the vocabulary-agnostic machinery deps
// `ogar-loco` directly.
pub use ogar_loco::{
    BODY_BYTES, BodyError, CLASSID_BYTES, CONTENT_SLOTS, Call, FnIndex, FunctionBody, LaneShape,
    MAX_VALUES_PER_CALL, PAYLOAD_BYTES_PER_SLOT, SLOT_STRIDE, VALUE_SLAB_LEN, call_in_slab,
};

use ogar_loco::{DOMAIN_FLOOR, RegistryError, Vocabulary, VocabularyRegistry};

/// The reserved Blocks [`ConceptDomain`] every block node routes on. Live in
/// `ogar_vocab` with zero shared codebook rows, so a consumer can branch on it
/// today.
pub const BLOCKS_DOMAIN: ConceptDomain = ConceptDomain::Blocks;

/// The high byte of the Blocks domain (`0x17`) — the `id >> 8` a consumer
/// matches when routing a block node from a bare classid.
pub const BLOCKS_DOMAIN_HI: u8 = 0x17;

/// First palette slot reserved for **device-specific** families — the
/// sprite/stage vocabulary (motion, looks, sound, events, sensing) that exists
/// in a Scratch-style frontend and has no counterpart in a general block
/// editor.
///
/// This is this palette's reading of the core's
/// [`DOMAIN_FLOOR`] (re-exported under the
/// historical name): below the floor is the shared computational core, whose
/// tables live once in `ogar_loco::vocabulary::shared_core`; at/above it is
/// this vocabulary's own range. The range above the floor is **reserved, not
/// allocated** — 108 device opcodes were measured in the Apache-2.0
/// `scratch-blocks` definitions, and they mint when a consumer needs them.
/// Reserve, don't reclaim.
pub const DEVICE_FAMILY_FLOOR: u8 = DOMAIN_FLOOR;

// ── Concept ids (authoritative here, NOT in the shared codebook) ────────────

/// The **one** concept this crate owns: `0x1717`.
///
/// # Why exactly one, and why not `0x1701`/`0x1702`
///
/// The node shapes a block program stores into — the function body and the
/// inventory row — are NOT Blockly's. They are described entirely in
/// `ogar-loco`'s own vocabulary ([`FunctionBody`], [`LaneShape`], the value
/// slab), and an elixir-shaped thinking template or an RO relation body is
/// the same shape with a different palette. So they belong to the substrate
/// and live at [`ogar_loco::LocoConcept`] (`0x1701` / `0x1702`) — **global
/// interest**, because thinking orchestration rides the same call ABI.
///
/// This crate previously owned those two ids, which read as though a
/// frontend owned the universal shape. It doesn't. What is genuinely
/// Blockly's is ONE thing: *which palette resolves the bytes* — and that
/// needs exactly one classid.
///
/// # This crate is a CONSUMER inside loco's domain
///
/// `0x17` belongs to the substrate, not to block programming (operator,
/// 2026-08-07). This palette is **seated at `0x1717`** — deliberately high,
/// so `0x1703`–`0x1716` stays contiguous headroom for `ogar-loco`'s own
/// growth (uplifting, Klickwege, whatever the ABI needs next). One slot is
/// the whole allocation: if a block frontend ever outgrows it, it gets its
/// **own domain** rather than expanding into the substrate's headroom.
///
/// The operations themselves are palette **bytes**, never concepts: 256
/// `FnIndex` slots resolved through this crate's [`BlocklyVocabulary`], not
/// 256 codebook rows. That is why one id suffices, and why the shared
/// codebook stays at zero `0x17XX` rows — this palette is **plug-and-play**,
/// activated only in a build that actually contains a block frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum BlockConcept {
    /// `0x1717` — **the Blockly/Scratch palette**: the classid that says
    /// "resolve this node's call bytes through [`BlocklyVocabulary`]". The
    /// node's SHAPE comes from [`ogar_loco::LocoConcept`]; this names the
    /// vocabulary, not the shape.
    Palette,
}

impl BlockConcept {
    /// Every concept, in id order — the enumeration hook a consumer uses to
    /// inherit the full set instead of hand-maintaining a parallel list.
    pub const ALL: [BlockConcept; 1] = [BlockConcept::Palette];

    /// This concept's canonical id inside the `0x17XX` Blocks domain.
    ///
    /// Authoritative HERE; `ogar_vocab`'s shared CODEBOOK deliberately carries
    /// zero `0x17XX` rows (plug-and-play, mirroring `ogar_obo::Namespace`).
    #[must_use]
    pub const fn concept_id(self) -> u16 {
        match self {
            BlockConcept::Palette => 0x1717,
        }
    }

    /// The full V3 render classid under a consumer's app prefix — canon-high
    /// `(concept as u32) << 16 | app_prefix`. Identical idiom to
    /// `ogar_obo::Namespace::render_classid` and `ogar_vocab::render_classid`.
    #[must_use]
    pub const fn render_classid(self, app_prefix: u16) -> u32 {
        ((self.concept_id() as u32) << 16) | (app_prefix as u32)
    }
}

// ── Storage partitioning ────────────────────────────────────────────────────

/// How block content is partitioned across SoA tables.
///
/// Functions are **not** pooled into one table: an [`Inventory`](SoaSplit::Inventory)
/// SoA registers which functions exist, and each function's body lives in its
/// own [`Content`](SoaSplit::Content) SoA.
///
/// That split is the V3 mailbox doctrine rather than a storage preference —
/// one function = one owner = its own SoA, so every write is owned and no
/// shared table accumulates writers. A registry read (what exists, where) never
/// touches a body, and a body write never contends with another function's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SoaSplit {
    /// The function registry — one row per function, addressed by identity.
    Inventory,
    /// One function's body — up to [`LaneShape::calls_per_function`] calls.
    Content,
}

impl SoaSplit {
    /// The **`ogar-loco`** concept whose classid this partition's rows carry.
    ///
    /// The split is the substrate's, not this palette's: a registry row and a
    /// function body are the same two shapes for every vocabulary, so they
    /// resolve to [`ogar_loco::LocoConcept`], never to [`BlockConcept`]. This
    /// crate's own id ([`BlockConcept::Palette`], `0x1717`) says which
    /// vocabulary resolves the call bytes — a different question from which
    /// shape the row is.
    #[must_use]
    pub const fn concept(self) -> ogar_loco::LocoConcept {
        match self {
            SoaSplit::Inventory => ogar_loco::LocoConcept::Inventory,
            SoaSplit::Content => ogar_loco::LocoConcept::FunctionBody,
        }
    }
}

// ── This palette's Vocabulary implementation ────────────────────────────────

/// The Blockly/Scratch palette as an `ogar-loco` [`Vocabulary`].
///
/// Every operation this palette has *allocated* sits below the floor — i.e.
/// in the shared computational core, whose tables live in the core crate —
/// so the domain hooks answer nothing yet. That is honest, not lazy: the
/// device families above [`DEVICE_FAMILY_FLOOR`] are **reserved, not
/// allocated**, and when they mint, their arity/body-reference tables land
/// here (and only here — the core never learns device vocabulary).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BlocklyVocabulary;

impl Vocabulary for BlocklyVocabulary {
    fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
        // No device family is minted yet; an above-floor byte is refused,
        // never guessed.
        None
    }

    fn domain_body_refs(&self, _f: FnIndex) -> u8 {
        0
    }
}

// ── Plug-and-play ───────────────────────────────────────────────────────────

/// Validate this palette and plug it into a consumer's
/// [`VocabularyRegistry`] under the Blocks **content** concept
/// ([`BlockConcept::Palette`]) — the USB handshake for this device.
///
/// A consumer (blockly-rs, lance-graph) builds ONE registry at boot and
/// calls each vocabulary crate's `plug_into`; every stored function node
/// then resolves through `registry.resolve_classid(node_classid)`, with no
/// consumer-side "this node must be Blockly" branch. Only the CONTENT
/// concept is plugged: it names WHICH vocabulary resolves a node's call
/// bytes. The node SHAPES ([`ogar_loco::LocoConcept`]) are the substrate's
/// and are not this palette's to register.
///
/// # Errors
///
/// [`RegistryError::ConceptTaken`] if something already claimed the Blocks
/// content concept — refused loudly rather than silently overwritten.
///
/// # Panics
///
/// Never in practice: [`BlocklyVocabulary`] conformance is pinned by this
/// crate's own tests, so `validate` cannot fail here.
pub fn plug_into(registry: &mut VocabularyRegistry) -> Result<(), RegistryError> {
    let checked = ogar_loco::vocabulary::conformance::validate(BlocklyVocabulary)
        .expect("BlocklyVocabulary conforms; pinned by this crate's tests");
    registry.plug(BlockConcept::Palette.concept_id(), &checked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ogar_loco::vocabulary::{conformance, shared_core};
    use ogar_vocab::canonical_concept_domain;

    #[test]
    fn every_concept_routes_to_the_blocks_domain() {
        for c in BlockConcept::ALL {
            assert_eq!(canonical_concept_domain(c.concept_id()), BLOCKS_DOMAIN);
            assert_eq!((c.concept_id() >> 8) as u8, BLOCKS_DOMAIN_HI);
        }
    }

    #[test]
    fn concept_ids_are_distinct_and_nonzero() {
        // A zero id would collide with NodeGuid::CLASSID_DEFAULT; a duplicate
        // would silently alias two schemas onto one classid.
        let mut seen = Vec::new();
        for c in BlockConcept::ALL {
            let id = c.concept_id();
            assert_ne!(id, 0, "{c:?} has a zero concept id");
            assert!(!seen.contains(&id), "{c:?} duplicates id {id:#06x}");
            seen.push(id);
        }
    }

    #[test]
    fn render_classid_is_canon_high() {
        // canon concept HIGH, app render prefix LOW (D-CLASSID-CANON-HIGH-FLIP).
        let id = BlockConcept::Palette.render_classid(0x1000);
        assert_eq!(id, 0x1717_1000);
        assert_eq!(id >> 16, u32::from(BlockConcept::Palette.concept_id()));
        assert_eq!(id & 0xFFFF, 0x1000);
    }

    #[test]
    fn soa_split_maps_each_partition_to_its_own_concept() {
        // Inventory and Content must NOT share a classid — the whole point of
        // the split is that a registry read never touches a body.
        assert_eq!(
            SoaSplit::Inventory.concept(),
            ogar_loco::LocoConcept::Inventory
        );
        assert_eq!(
            SoaSplit::Content.concept(),
            ogar_loco::LocoConcept::FunctionBody
        );
        assert_ne!(
            SoaSplit::Inventory.concept().concept_id(),
            SoaSplit::Content.concept().concept_id()
        );
    }

    #[test]
    fn the_blockly_vocabulary_conforms_to_the_sharing_discipline() {
        // The mechanical gate every vocabulary crate must run: shared-core
        // bytes answer from the core, the domain range refuses what is not
        // minted, and no reported shape can truncate a call's own body
        // references.
        assert_eq!(conformance::check(&BlocklyVocabulary), Ok(()));
        // Spot-check the routing this palette relies on: control flow and
        // expressions answer from the shared core THROUGH the vocabulary.
        let v = BlocklyVocabulary;
        assert_eq!(v.stack_arity(FnIndex::REPEAT), Some(1));
        assert_eq!(v.body_refs(FnIndex::IF_ELSE), 2);
        assert_eq!(v.stack_arity(FnIndex::ADD), Some(2));
        // …and an unminted device byte is refused, not guessed.
        assert_eq!(v.stack_arity(FnIndex(DEVICE_FAMILY_FLOOR)), None);
    }

    #[test]
    fn shared_core_and_device_family_partition_the_palette() {
        // Can-fire AND can-stay-silent on the same predicate: a classifier that
        // answers the same way for everything is worthless. (`is_domain_specific`
        // is the core's name; this palette reads it as "device family".)
        assert!(FnIndex::LT.is_shared_core());
        assert!(!FnIndex::LT.is_domain_specific());

        let device = FnIndex(DEVICE_FAMILY_FLOOR);
        assert!(device.is_domain_specific());
        assert!(!device.is_shared_core());

        // NOP is not an operation at all — neither bucket claims it.
        assert!(!FnIndex::NOP.is_shared_core());
        assert!(!FnIndex::NOP.is_domain_specific());
    }

    #[test]
    fn every_named_op_is_a_distinct_slot_in_the_shared_core() {
        // The whole value of the palette is that two frontends land on ONE
        // slot. A duplicate here would silently merge two operations; a slot at
        // or above the device floor would misclassify a shared op. This ALSO
        // proves the re-export surface is complete for every named constant —
        // the census compiles against `ogar_blockly::FnIndex`, exactly as the
        // downstream consumers do.
        let named: &[(&str, FnIndex)] = &[
            ("IF", FnIndex::IF),
            ("IF_ELSE", FnIndex::IF_ELSE),
            ("REPEAT", FnIndex::REPEAT),
            ("REPEAT_UNTIL", FnIndex::REPEAT_UNTIL),
            ("WHILE", FnIndex::WHILE),
            ("FOREVER", FnIndex::FOREVER),
            ("FOR_EACH", FnIndex::FOR_EACH),
            ("FOR_RANGE", FnIndex::FOR_RANGE),
            ("WAIT", FnIndex::WAIT),
            ("WAIT_UNTIL", FnIndex::WAIT_UNTIL),
            ("STOP", FnIndex::STOP),
            ("BREAK", FnIndex::BREAK),
            ("CONTINUE", FnIndex::CONTINUE),
            ("RETURN", FnIndex::RETURN),
            ("AND", FnIndex::AND),
            ("OR", FnIndex::OR),
            ("NOT", FnIndex::NOT),
            ("TRUE", FnIndex::TRUE),
            ("FALSE", FnIndex::FALSE),
            ("NULL", FnIndex::NULL),
            ("TERNARY", FnIndex::TERNARY),
            ("EQ", FnIndex::EQ),
            ("NEQ", FnIndex::NEQ),
            ("LT", FnIndex::LT),
            ("LTE", FnIndex::LTE),
            ("GT", FnIndex::GT),
            ("GTE", FnIndex::GTE),
            ("ADD", FnIndex::ADD),
            ("SUB", FnIndex::SUB),
            ("MUL", FnIndex::MUL),
            ("DIV", FnIndex::DIV),
            ("POW", FnIndex::POW),
            ("MOD", FnIndex::MOD),
            ("NUMBER", FnIndex::NUMBER),
            ("ABS", FnIndex::ABS),
            ("NEG", FnIndex::NEG),
            ("ROUND", FnIndex::ROUND),
            ("FLOOR", FnIndex::FLOOR),
            ("CEIL", FnIndex::CEIL),
            ("SQRT", FnIndex::SQRT),
            ("LN", FnIndex::LN),
            ("LOG10", FnIndex::LOG10),
            ("EXP_E", FnIndex::EXP_E),
            ("EXP_10", FnIndex::EXP_10),
            ("SIN", FnIndex::SIN),
            ("COS", FnIndex::COS),
            ("TAN", FnIndex::TAN),
            ("ASIN", FnIndex::ASIN),
            ("ACOS", FnIndex::ACOS),
            ("ATAN", FnIndex::ATAN),
            ("ATAN2", FnIndex::ATAN2),
            ("RANDOM_INT", FnIndex::RANDOM_INT),
            ("RANDOM_FLOAT", FnIndex::RANDOM_FLOAT),
            ("CONSTRAIN", FnIndex::CONSTRAIN),
            ("NUMBER_PROPERTY", FnIndex::NUMBER_PROPERTY),
            ("CONSTANT", FnIndex::CONSTANT),
            ("ON_LIST", FnIndex::ON_LIST),
            ("TEXT", FnIndex::TEXT),
            ("JOIN", FnIndex::JOIN),
            ("LENGTH", FnIndex::LENGTH),
            ("CHAR_AT", FnIndex::CHAR_AT),
            ("INDEX_OF", FnIndex::INDEX_OF),
            ("IS_EMPTY", FnIndex::IS_EMPTY),
            ("SUBSTRING", FnIndex::SUBSTRING),
            ("CHANGE_CASE", FnIndex::CHANGE_CASE),
            ("TRIM", FnIndex::TRIM),
            ("CONTAINS", FnIndex::CONTAINS),
            ("APPEND", FnIndex::APPEND),
            ("PRINT", FnIndex::PRINT),
            ("PROMPT", FnIndex::PROMPT),
            ("COUNT", FnIndex::COUNT),
            ("REPLACE", FnIndex::REPLACE),
            ("REVERSE", FnIndex::REVERSE),
            ("LIST_EMPTY", FnIndex::LIST_EMPTY),
            ("LIST_WITH", FnIndex::LIST_WITH),
            ("LIST_REPEAT", FnIndex::LIST_REPEAT),
            ("LIST_LENGTH", FnIndex::LIST_LENGTH),
            ("LIST_IS_EMPTY", FnIndex::LIST_IS_EMPTY),
            ("LIST_INDEX_OF", FnIndex::LIST_INDEX_OF),
            ("LIST_GET", FnIndex::LIST_GET),
            ("LIST_SET", FnIndex::LIST_SET),
            ("LIST_INSERT", FnIndex::LIST_INSERT),
            ("LIST_ADD", FnIndex::LIST_ADD),
            ("LIST_DELETE", FnIndex::LIST_DELETE),
            ("LIST_DELETE_ALL", FnIndex::LIST_DELETE_ALL),
            ("LIST_SUBLIST", FnIndex::LIST_SUBLIST),
            ("LIST_SPLIT", FnIndex::LIST_SPLIT),
            ("LIST_SORT", FnIndex::LIST_SORT),
            ("LIST_CONTAINS", FnIndex::LIST_CONTAINS),
            ("VAR_GET", FnIndex::VAR_GET),
            ("VAR_SET", FnIndex::VAR_SET),
            ("VAR_CHANGE", FnIndex::VAR_CHANGE),
            ("PROC_DEF", FnIndex::PROC_DEF),
            ("PROC_CALL", FnIndex::PROC_CALL),
            ("PROC_ARG", FnIndex::PROC_ARG),
        ];

        let mut seen: Vec<(u8, &str)> = Vec::new();
        for (name, op) in named {
            assert!(
                op.is_shared_core(),
                "{name} at {:#04x} is not in the shared core",
                op.0
            );
            if let Some((_, prior)) = seen.iter().find(|(slot, _)| *slot == op.0) {
                panic!("{name} collides with {prior} at slot {:#04x}", op.0);
            }
            seen.push((op.0, name));
        }

        // Anti-vacuity: the table must actually be substantial, or "all
        // distinct" is trivially true of a near-empty list.
        assert!(seen.len() >= 90, "palette census shrank to {}", seen.len());

        // And the shared core's tables must cover the control range this
        // palette's frontends lower through — spot-anchored so a core-side
        // regression is caught from the vocabulary side too.
        assert_eq!(shared_core::stack_arity(FnIndex::REPEAT), Some(1));
        assert_eq!(shared_core::body_refs(FnIndex::REPEAT), 1);
    }
}
