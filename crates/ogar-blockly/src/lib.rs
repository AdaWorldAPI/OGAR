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
//! `operator_lt` (Scratch) are ONE slot — [`PaletteOp::LT`]. `operator_mathop`
//! (one Scratch block with a dropdown) fans out to the same slots that
//! `math_single` + `math_trig` (two Blockly blocks) fan out to. The palette is
//! where the two vocabularies actually meet.
//!
//! # The shape: one classid for content, 360 ops per function
//!
//! A V3 node is 512 bytes = 32 × 16-byte slots: `key(16) | edges(16) |
//! value(480)`. The value slab is 30 slots, each a `classid(4) + 12-byte
//! payload` facet — so a node carries **30 × 12 = 360 payload bytes**.
//!
//! One palette byte is one operation, so:
//!
//! ```text
//!   one function  =  one node  =  512 bytes
//!     key   slot 0        classid = CONTENT (one) · identity = which function
//!     edges slot 1        wiring — callers / callees
//!     value slots 2..31   30 × 12 = 360 operation bytes
//! ```
//!
//! **A function body is capped at [`OPS_PER_FUNCTION`] = 360 operations, and
//! the cap is enforced** ([`FunctionBody::push`] / [`FunctionBody::from_ops`]).
//! Over-length is not a bigger row — it is a **split into two functions**. This
//! is the substrate's own rule applied to program structure: *scale is the next
//! cascade level, never field-widening.* The cap is a forcing function for
//! decomposition, and it makes "does this function fit?" a fact you can check
//! before you write, not a surprise at runtime.
//!
//! Only ONE concept id is spent on content ([`BlockConcept::Content`]) — the
//! operations live in the payload, not in the classid space. An earlier design
//! pass sized a per-operation concept space and hit an imagined 255-slot
//! ceiling; that space does not need to exist.
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
//! # Classid routing — the reserved Blocks domain (`0x17XX`)
//!
//! `ogar_vocab` reserves `ConceptDomain::Blocks` (`0x17XX`) and ships ZERO
//! concept rows there. This crate is the authoritative home for the ids inside
//! that domain — the same plug-and-play posture as `ogar-obo` over
//! `ConceptDomain::Ontology`: only consumers that dep `ogar-blockly` compile
//! them, so ERP / clinical / project consumers never pull a block vocabulary
//! they have no use for.
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

/// The reserved Blocks [`ConceptDomain`] every block node routes on. Live in
/// `ogar_vocab` with zero shared codebook rows, so a consumer can branch on it
/// today.
pub const BLOCKS_DOMAIN: ConceptDomain = ConceptDomain::Blocks;

/// The high byte of the Blocks domain (`0x17`) — the `id >> 8` a consumer
/// matches when routing a block node from a bare classid.
pub const BLOCKS_DOMAIN_HI: u8 = 0x17;

// ── The function-body budget ────────────────────────────────────────────────

/// Value-slab facet slots in a 512-byte node: `value(480) / 16` = **30**.
pub const CONTENT_SLOTS: usize = 30;

/// Bytes of one facet slot: the V3 16-byte facet stride.
pub const SLOT_STRIDE: usize = 16;

/// Bytes of a facet's classid prefix.
pub const CLASSID_BYTES: usize = 4;

/// Payload bytes in one 16-byte facet: `16 - classid(4)` = **12**.
pub const PAYLOAD_BYTES_PER_SLOT: usize = SLOT_STRIDE - CLASSID_BYTES;

/// Bytes of a node's value slab: `30 × 16` = **480**.
///
/// Note the asymmetry that catches people: the slab is **480** bytes but only
/// [`OPS_PER_FUNCTION`] = 360 of them are operations. The other 120 are the 30
/// facets' 4-byte classids, interleaved — never a contiguous run.
pub const VALUE_SLAB_LEN: usize = CONTENT_SLOTS * SLOT_STRIDE;

/// Operations one function body carries: `30 × 12` = **360**.
///
/// This is a derived budget, not a chosen constant — it is exactly the payload
/// capacity of a node's value slab once the key and edge slots are taken. A
/// function that needs more is **split**, never widened.
pub const OPS_PER_FUNCTION: usize = CONTENT_SLOTS * PAYLOAD_BYTES_PER_SLOT;

const _: () = assert!(
    OPS_PER_FUNCTION == 360,
    "360 = 30 value-slab facet slots × 12 payload bytes each"
);

// ── Concept ids (authoritative here, NOT in the shared codebook) ────────────

/// The concepts this crate owns inside `0x17XX`.
///
/// Deliberately small. The operations are palette **bytes**, not concepts — the
/// classid names the schema, and one content schema is all a function body
/// needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum BlockConcept {
    /// `0x1701` — a **function body**: identity names the function, the value
    /// slab carries up to [`OPS_PER_FUNCTION`] palette bytes. The one content
    /// classid.
    Content,
    /// `0x1702` — the **inventory** row: the function registry entry (which
    /// functions exist, addressed by identity). Reads never touch a body.
    Inventory,
}

impl BlockConcept {
    /// Every concept, in id order — the enumeration hook a consumer uses to
    /// inherit the full set instead of hand-maintaining a parallel list.
    pub const ALL: [BlockConcept; 2] = [BlockConcept::Content, BlockConcept::Inventory];

    /// This concept's canonical id inside the `0x17XX` Blocks domain.
    ///
    /// Authoritative HERE; `ogar_vocab`'s shared CODEBOOK deliberately carries
    /// zero `0x17XX` rows (plug-and-play, mirroring `ogar_obo::Namespace`).
    #[must_use]
    pub const fn concept_id(self) -> u16 {
        match self {
            BlockConcept::Content => 0x1701,
            BlockConcept::Inventory => 0x1702,
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

// ── The 256-slot palette ────────────────────────────────────────────────────

/// First palette slot reserved for **device-specific** families — the
/// sprite/stage vocabulary (motion, looks, sound, events, sensing) that exists
/// in a Scratch-style frontend and has no counterpart in a general block
/// editor.
///
/// Slots below this floor are the **shared computational core**: every one of
/// them means the same thing in every frontend. `slot >= DEVICE_FAMILY_FLOOR`
/// is therefore a one-compare test for "this op is frontend-specific", which a
/// renderer or a compiler can branch on without a table lookup.
///
/// The range above the floor is **reserved, not allocated** — 108 device
/// opcodes were measured in the Apache-2.0 `scratch-blocks` definitions, and
/// they mint when a consumer needs them. Reserve, don't reclaim.
pub const DEVICE_FAMILY_FLOOR: u8 = 0x90;

/// One operation — a single byte of a function body, and the unit the palette
/// indexes.
///
/// `0x00` is reserved as the zero-fallback: an unwritten payload byte reads as
/// [`PaletteOp::NOP`], so a partially-filled body is well-defined without a
/// length field. This mirrors the substrate's monotonic zero ladder (a zero
/// tier means *not consulted*, never *compacted away*).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(transparent)]
pub struct PaletteOp(pub u8);

impl PaletteOp {
    /// The zero slot — an unwritten body byte. Never a real operation.
    pub const NOP: PaletteOp = PaletteOp(0x00);

    // ── control (0x01..0x1F) ────────────────────────────────────────────
    /// Conditional with no else arm. `controls_if` · `control_if`.
    pub const IF: PaletteOp = PaletteOp(0x01);
    /// Conditional with an else arm. `controls_ifelse` · `control_if_else`.
    pub const IF_ELSE: PaletteOp = PaletteOp(0x02);
    /// Bounded repeat. `controls_repeat`/`_ext` · `control_repeat`.
    pub const REPEAT: PaletteOp = PaletteOp(0x03);
    /// Repeat until a condition holds. `controls_whileUntil[UNTIL]` ·
    /// `control_repeat_until`.
    pub const REPEAT_UNTIL: PaletteOp = PaletteOp(0x04);
    /// Repeat while a condition holds. `controls_whileUntil[WHILE]` ·
    /// `control_while`.
    pub const WHILE: PaletteOp = PaletteOp(0x05);
    /// Unbounded repeat. `control_forever` (no Blockly counterpart).
    pub const FOREVER: PaletteOp = PaletteOp(0x06);
    /// Iterate a list. `controls_forEach` · `control_for_each`.
    pub const FOR_EACH: PaletteOp = PaletteOp(0x07);
    /// Iterate a numeric range. `controls_for` (no Scratch counterpart).
    pub const FOR_RANGE: PaletteOp = PaletteOp(0x08);
    /// Suspend for a duration. `control_wait`.
    pub const WAIT: PaletteOp = PaletteOp(0x09);
    /// Suspend until a condition holds. `control_wait_until`.
    pub const WAIT_UNTIL: PaletteOp = PaletteOp(0x0A);
    /// Stop this script / all / others. `control_stop`.
    pub const STOP: PaletteOp = PaletteOp(0x0B);
    /// Leave the enclosing loop. `controls_flow_statements[BREAK]`.
    pub const BREAK: PaletteOp = PaletteOp(0x0C);
    /// Skip to the enclosing loop's next iteration.
    /// `controls_flow_statements[CONTINUE]`.
    pub const CONTINUE: PaletteOp = PaletteOp(0x0D);
    /// Return from the enclosing function. `procedures_ifreturn`.
    pub const RETURN: PaletteOp = PaletteOp(0x0E);

    // ── logic (0x20..0x2F) ──────────────────────────────────────────────
    /// Boolean conjunction. `logic_operation[AND]` · `operator_and`.
    pub const AND: PaletteOp = PaletteOp(0x20);
    /// Boolean disjunction. `logic_operation[OR]` · `operator_or`.
    pub const OR: PaletteOp = PaletteOp(0x21);
    /// Boolean negation. `logic_negate` · `operator_not`.
    pub const NOT: PaletteOp = PaletteOp(0x22);
    /// Literal true. `logic_boolean[TRUE]`.
    pub const TRUE: PaletteOp = PaletteOp(0x23);
    /// Literal false. `logic_boolean[FALSE]`.
    pub const FALSE: PaletteOp = PaletteOp(0x24);
    /// Literal null. `logic_null` (no Scratch counterpart).
    pub const NULL: PaletteOp = PaletteOp(0x25);
    /// Conditional expression. `logic_ternary` (no Scratch counterpart).
    pub const TERNARY: PaletteOp = PaletteOp(0x26);

    // ── comparison (0x30..0x3F) ─────────────────────────────────────────
    /// Equality. `logic_compare[EQ]` · `operator_equals`.
    pub const EQ: PaletteOp = PaletteOp(0x30);
    /// Inequality. `logic_compare[NEQ]` (no Scratch counterpart).
    pub const NEQ: PaletteOp = PaletteOp(0x31);
    /// Less than. `logic_compare[LT]` · `operator_lt`.
    pub const LT: PaletteOp = PaletteOp(0x32);
    /// Less than or equal. `logic_compare[LTE]` (no Scratch counterpart).
    pub const LTE: PaletteOp = PaletteOp(0x33);
    /// Greater than. `logic_compare[GT]` · `operator_gt`.
    pub const GT: PaletteOp = PaletteOp(0x34);
    /// Greater than or equal. `logic_compare[GTE]` (no Scratch counterpart).
    pub const GTE: PaletteOp = PaletteOp(0x35);

    // ── math (0x40..0x5F) ───────────────────────────────────────────────
    /// Addition. `math_arithmetic[ADD]` · `operator_add`.
    pub const ADD: PaletteOp = PaletteOp(0x40);
    /// Subtraction. `math_arithmetic[MINUS]` · `operator_subtract`.
    pub const SUB: PaletteOp = PaletteOp(0x41);
    /// Multiplication. `math_arithmetic[MULTIPLY]` · `operator_multiply`.
    pub const MUL: PaletteOp = PaletteOp(0x42);
    /// Division. `math_arithmetic[DIVIDE]` · `operator_divide`.
    pub const DIV: PaletteOp = PaletteOp(0x43);
    /// Exponentiation. `math_arithmetic[POWER]` (no Scratch counterpart).
    pub const POW: PaletteOp = PaletteOp(0x44);
    /// Modulo. `math_modulo` · `operator_mod`.
    pub const MOD: PaletteOp = PaletteOp(0x45);
    /// Numeric literal. `math_number` (Scratch uses a field, not a block).
    pub const NUMBER: PaletteOp = PaletteOp(0x46);
    /// Absolute value. `math_single[ABS]` · `operator_mathop[abs]`.
    pub const ABS: PaletteOp = PaletteOp(0x47);
    /// Negation. `math_single[NEG]`.
    pub const NEG: PaletteOp = PaletteOp(0x48);
    /// Round to nearest. `math_round[ROUND]` · `operator_round`.
    pub const ROUND: PaletteOp = PaletteOp(0x49);
    /// Round toward -inf. `math_round[ROUNDDOWN]` · `operator_mathop[floor]`.
    pub const FLOOR: PaletteOp = PaletteOp(0x4A);
    /// Round toward +inf. `math_round[ROUNDUP]` · `operator_mathop[ceiling]`.
    pub const CEIL: PaletteOp = PaletteOp(0x4B);
    /// Square root. `math_single[ROOT]` · `operator_mathop[sqrt]`.
    pub const SQRT: PaletteOp = PaletteOp(0x4C);
    /// Natural logarithm. `math_single[LN]` · `operator_mathop[ln]`.
    pub const LN: PaletteOp = PaletteOp(0x4D);
    /// Base-10 logarithm. `math_single[LOG10]` · `operator_mathop[log]`.
    pub const LOG10: PaletteOp = PaletteOp(0x4E);
    /// `e^x`. `math_single[EXP]` · `operator_mathop[e ^]`.
    pub const EXP_E: PaletteOp = PaletteOp(0x4F);
    /// `10^x`. `math_single[POW10]` · `operator_mathop[10 ^]`.
    pub const EXP_10: PaletteOp = PaletteOp(0x50);
    /// Sine. `math_trig[SIN]` · `operator_mathop[sin]`.
    pub const SIN: PaletteOp = PaletteOp(0x51);
    /// Cosine. `math_trig[COS]` · `operator_mathop[cos]`.
    pub const COS: PaletteOp = PaletteOp(0x52);
    /// Tangent. `math_trig[TAN]` · `operator_mathop[tan]`.
    pub const TAN: PaletteOp = PaletteOp(0x53);
    /// Arcsine. `math_trig[ASIN]` · `operator_mathop[asin]`.
    pub const ASIN: PaletteOp = PaletteOp(0x54);
    /// Arccosine. `math_trig[ACOS]` · `operator_mathop[acos]`.
    pub const ACOS: PaletteOp = PaletteOp(0x55);
    /// Arctangent. `math_trig[ATAN]` · `operator_mathop[atan]`.
    pub const ATAN: PaletteOp = PaletteOp(0x56);
    /// Two-argument arctangent. `math_atan2` (no Scratch counterpart).
    pub const ATAN2: PaletteOp = PaletteOp(0x57);
    /// Random integer in a range. `math_random_int` · `operator_random`.
    pub const RANDOM_INT: PaletteOp = PaletteOp(0x58);
    /// Random fraction. `math_random_float` (no Scratch counterpart).
    pub const RANDOM_FLOAT: PaletteOp = PaletteOp(0x59);
    /// Clamp to a range. `math_constrain` (no Scratch counterpart).
    pub const CONSTRAIN: PaletteOp = PaletteOp(0x5A);
    /// Numeric predicate (even/odd/prime/whole/positive/negative/divisible).
    /// `math_number_property` (no Scratch counterpart).
    pub const NUMBER_PROPERTY: PaletteOp = PaletteOp(0x5B);
    /// Named constant (pi/e/phi/sqrt2/sqrt1_2/infinity). `math_constant`.
    pub const CONSTANT: PaletteOp = PaletteOp(0x5C);
    /// Aggregate over a list (sum/min/max/average/median/mode/std_dev).
    /// `math_on_list` (no Scratch counterpart).
    pub const ON_LIST: PaletteOp = PaletteOp(0x5D);

    // ── text (0x60..0x6F) ───────────────────────────────────────────────
    /// String literal. `text`.
    pub const TEXT: PaletteOp = PaletteOp(0x60);
    /// Concatenate. `text_join` · `operator_join`.
    pub const JOIN: PaletteOp = PaletteOp(0x61);
    /// Character count. `text_length` · `operator_length`.
    pub const LENGTH: PaletteOp = PaletteOp(0x62);
    /// Character at a position. `text_charAt` · `operator_letter_of`.
    pub const CHAR_AT: PaletteOp = PaletteOp(0x63);
    /// Substring search. `text_indexOf` (no Scratch counterpart).
    pub const INDEX_OF: PaletteOp = PaletteOp(0x64);
    /// Emptiness test. `text_isEmpty` (no Scratch counterpart).
    pub const IS_EMPTY: PaletteOp = PaletteOp(0x65);
    /// Substring extraction. `text_getSubstring` (no Scratch counterpart).
    pub const SUBSTRING: PaletteOp = PaletteOp(0x66);
    /// Case conversion. `text_changeCase` (no Scratch counterpart).
    pub const CHANGE_CASE: PaletteOp = PaletteOp(0x67);
    /// Whitespace trim. `text_trim` (no Scratch counterpart).
    pub const TRIM: PaletteOp = PaletteOp(0x68);
    /// Containment test. `text_contains`-shaped · `operator_contains`.
    pub const CONTAINS: PaletteOp = PaletteOp(0x69);
    /// Append to a variable. `text_append`.
    pub const APPEND: PaletteOp = PaletteOp(0x6A);
    /// Emit to output. `text_print`.
    pub const PRINT: PaletteOp = PaletteOp(0x6B);
    /// Prompt for input. `text_prompt`/`_ext`.
    pub const PROMPT: PaletteOp = PaletteOp(0x6C);
    /// Occurrence count. `text_count` (no Scratch counterpart).
    pub const COUNT: PaletteOp = PaletteOp(0x6D);
    /// Substring replacement. `text_replace` (no Scratch counterpart).
    pub const REPLACE: PaletteOp = PaletteOp(0x6E);
    /// Reversal. `text_reverse` (no Scratch counterpart).
    pub const REVERSE: PaletteOp = PaletteOp(0x6F);

    // ── list (0x70..0x7F) ───────────────────────────────────────────────
    /// Empty list literal. `lists_create_empty`.
    pub const LIST_EMPTY: PaletteOp = PaletteOp(0x70);
    /// List literal with items. `lists_create_with`.
    pub const LIST_WITH: PaletteOp = PaletteOp(0x71);
    /// Repeat an item into a list. `lists_repeat`.
    pub const LIST_REPEAT: PaletteOp = PaletteOp(0x72);
    /// Item count. `lists_length` · `data_lengthoflist`.
    pub const LIST_LENGTH: PaletteOp = PaletteOp(0x73);
    /// Emptiness test. `lists_isEmpty`.
    pub const LIST_IS_EMPTY: PaletteOp = PaletteOp(0x74);
    /// Position of an item. `lists_indexOf` · `data_itemnumoflist`.
    pub const LIST_INDEX_OF: PaletteOp = PaletteOp(0x75);
    /// Read an item. `lists_getIndex` · `data_itemoflist`.
    pub const LIST_GET: PaletteOp = PaletteOp(0x76);
    /// Write an item. `lists_setIndex[SET]` · `data_replaceitemoflist`.
    pub const LIST_SET: PaletteOp = PaletteOp(0x77);
    /// Insert an item. `lists_setIndex[INSERT]` · `data_insertatlist`.
    pub const LIST_INSERT: PaletteOp = PaletteOp(0x78);
    /// Append an item. `data_addtolist`.
    pub const LIST_ADD: PaletteOp = PaletteOp(0x79);
    /// Remove an item. `lists_getIndex[REMOVE]` · `data_deleteoflist`.
    pub const LIST_DELETE: PaletteOp = PaletteOp(0x7A);
    /// Remove every item. `data_deletealloflist`.
    pub const LIST_DELETE_ALL: PaletteOp = PaletteOp(0x7B);
    /// Sublist extraction. `lists_getSublist` (no Scratch counterpart).
    pub const LIST_SUBLIST: PaletteOp = PaletteOp(0x7C);
    /// Split / join against a delimiter. `lists_split`.
    pub const LIST_SPLIT: PaletteOp = PaletteOp(0x7D);
    /// Ordering. `lists_sort` (no Scratch counterpart).
    pub const LIST_SORT: PaletteOp = PaletteOp(0x7E);
    /// Containment test. `lists_indexOf`-shaped · `data_listcontainsitem`.
    pub const LIST_CONTAINS: PaletteOp = PaletteOp(0x7F);

    // ── variable + procedure (0x80..0x8F) ───────────────────────────────
    /// Read a variable. `variables_get` · `data_variable`.
    pub const VAR_GET: PaletteOp = PaletteOp(0x80);
    /// Write a variable. `variables_set` · `data_setvariableto`.
    pub const VAR_SET: PaletteOp = PaletteOp(0x81);
    /// Increment a variable. `math_change` · `data_changevariableby`.
    pub const VAR_CHANGE: PaletteOp = PaletteOp(0x82);
    /// Define a function. `procedures_defnoreturn`/`_defreturn` ·
    /// `procedures_definition`.
    pub const PROC_DEF: PaletteOp = PaletteOp(0x83);
    /// Invoke a function. `procedures_callnoreturn`/`_callreturn` ·
    /// `procedures_call`.
    pub const PROC_CALL: PaletteOp = PaletteOp(0x84);
    /// Read a call argument. `procedures_defreturn` argument access.
    pub const PROC_ARG: PaletteOp = PaletteOp(0x85);

    /// Is this a **shared computational** operation — one that means the same
    /// thing in every frontend?
    ///
    /// A one-compare test, no table lookup: everything below
    /// [`DEVICE_FAMILY_FLOOR`] is shared. [`PaletteOp::NOP`] is not an
    /// operation and answers `false`.
    #[must_use]
    pub const fn is_shared_core(self) -> bool {
        self.0 != 0 && self.0 < DEVICE_FAMILY_FLOOR
    }

    /// Is this a **device-specific** operation — sprite/stage vocabulary with
    /// no counterpart in a general block editor?
    #[must_use]
    pub const fn is_device_family(self) -> bool {
        self.0 >= DEVICE_FAMILY_FLOOR
    }
}

// ── Function bodies ─────────────────────────────────────────────────────────

/// A function body exceeded [`OPS_PER_FUNCTION`].
///
/// The remedy is a **split into two functions**, never a wider row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyOverflow {
    /// How many operations were offered.
    pub offered: usize,
}

impl core::fmt::Display for BodyOverflow {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "function body of {} operations exceeds the {}-op budget \
             (30 value-slab slots × 12 payload bytes); split the function",
            self.offered, OPS_PER_FUNCTION
        )
    }
}

impl core::error::Error for BodyOverflow {}

/// One function's operations — exactly the payload capacity of a node's value
/// slab, and never more.
///
/// The cap is enforced at every entry point, so a `FunctionBody` that exists is
/// a function that fits in one 512-byte node. There is no partially-valid
/// state and no runtime surprise at write time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionBody {
    /// Stored as raw bytes, not `[PaletteOp; N]`, so
    /// [`as_payload_bytes`](Self::as_payload_bytes) is a plain borrow of the
    /// wire form — no transmute, no copy, no `unsafe`.
    ops: [u8; OPS_PER_FUNCTION],
    len: u16,
}

impl Default for FunctionBody {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionBody {
    /// An empty body — every byte [`PaletteOp::NOP`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ops: [0u8; OPS_PER_FUNCTION],
            len: 0,
        }
    }

    /// Build from a slice, rejecting anything past the budget.
    ///
    /// # Errors
    ///
    /// [`BodyOverflow`] when `ops.len() > OPS_PER_FUNCTION` — split the
    /// function rather than widening the row.
    pub fn from_ops(ops: &[PaletteOp]) -> Result<Self, BodyOverflow> {
        if ops.len() > OPS_PER_FUNCTION {
            return Err(BodyOverflow { offered: ops.len() });
        }
        let mut body = Self::new();
        let mut written = 0usize;
        for (slot, op) in body.ops.iter_mut().zip(ops) {
            *slot = op.0;
            written += 1;
        }
        // Count what was COPIED, never what was offered: `len` indexes a
        // fixed 360-byte array, so deriving it from the caller's length would
        // make the guard above solely responsible for keeping `ops()` in
        // bounds. Belt and braces — a future edit to the guard cannot produce
        // an out-of-range `len`.
        body.len = written as u16;
        Ok(body)
    }

    /// Append one operation.
    ///
    /// # Errors
    ///
    /// [`BodyOverflow`] when the body is already full.
    pub fn push(&mut self, op: PaletteOp) -> Result<(), BodyOverflow> {
        let n = self.len as usize;
        if n >= OPS_PER_FUNCTION {
            return Err(BodyOverflow { offered: n + 1 });
        }
        self.ops[n] = op.0;
        self.len = (n + 1) as u16;
        Ok(())
    }

    /// The operations written so far. (`impl Iterator` is already `#[must_use]`
    /// — a second attribute here is `clippy::double_must_use`.)
    pub fn ops(&self) -> impl Iterator<Item = PaletteOp> + '_ {
        self.ops[..self.len as usize].iter().copied().map(PaletteOp)
    }

    /// The operation at `index`, or `None` past [`len`](Self::len).
    #[must_use]
    pub fn op(&self, index: usize) -> Option<PaletteOp> {
        (index < self.len as usize).then(|| PaletteOp(self.ops[index]))
    }

    /// How many operations are written.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Is the body empty?
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Remaining budget before a split is required.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        OPS_PER_FUNCTION - self.len as usize
    }

    /// The body's 360 operation bytes in **execution order**, `NOP`-padded past
    /// [`len`](Self::len).
    ///
    /// # This is the GATHERED form, NOT the slab layout
    ///
    /// These bytes are **not** contiguous in a node's value slab. The slab is
    /// `CONTENT_SLOTS × 16` = 480 bytes of `classid(4) + payload(12)` facets, so
    /// operation `i` lives at slab offset
    /// [`slab_offset(i)`](Self::slab_offset) — stride 16, `+4` into each facet,
    /// never at offset `i`.
    ///
    /// Copying this array over the front of a slab would overwrite the first
    /// 22½ facets' classids *and* payloads. Use
    /// [`write_into_value_slab`](Self::write_into_value_slab) to place it, or
    /// [`op_in_slab`] to read one operation in place without gathering at all.
    #[must_use]
    pub const fn as_ops_bytes(&self) -> &[u8; OPS_PER_FUNCTION] {
        &self.ops
    }

    /// Byte offset of operation `index` within a node's **480-byte value slab**.
    ///
    /// `(index / 12) * 16 + 4 + (index % 12)` — pick the facet, skip its
    /// 4-byte classid, then index within its 12-byte payload lane.
    ///
    /// # Panics
    ///
    /// When `index >= OPS_PER_FUNCTION`.
    #[must_use]
    pub const fn slab_offset(index: usize) -> usize {
        assert!(index < OPS_PER_FUNCTION, "operation index out of range");
        let facet = index / PAYLOAD_BYTES_PER_SLOT;
        let within = index % PAYLOAD_BYTES_PER_SLOT;
        facet * SLOT_STRIDE + CLASSID_BYTES + within
    }

    /// Scatter the body into a node's value slab, writing **only** the 12-byte
    /// payload lane of each facet.
    ///
    /// The 4-byte classid of every facet is left untouched — this writes the
    /// operations, never the addressing.
    pub fn write_into_value_slab(&self, slab: &mut [u8; VALUE_SLAB_LEN]) {
        for facet in 0..CONTENT_SLOTS {
            let src = facet * PAYLOAD_BYTES_PER_SLOT;
            let dst = facet * SLOT_STRIDE + CLASSID_BYTES;
            slab[dst..dst + PAYLOAD_BYTES_PER_SLOT]
                .copy_from_slice(&self.ops[src..src + PAYLOAD_BYTES_PER_SLOT]);
        }
    }

    /// Gather a body back out of a node's value slab — the inverse of
    /// [`write_into_value_slab`](Self::write_into_value_slab).
    ///
    /// `len` is recovered as the position after the last non-`NOP` byte, since
    /// the wire form carries no length field (that is the whole point of the
    /// zero-fallback padding — the 362-byte in-memory `FunctionBody` has a
    /// `u16 len`, the 360-byte wire form deliberately does not).
    #[must_use]
    pub fn read_from_value_slab(slab: &[u8; VALUE_SLAB_LEN]) -> Self {
        let mut body = Self::new();
        for facet in 0..CONTENT_SLOTS {
            let src = facet * SLOT_STRIDE + CLASSID_BYTES;
            let dst = facet * PAYLOAD_BYTES_PER_SLOT;
            body.ops[dst..dst + PAYLOAD_BYTES_PER_SLOT]
                .copy_from_slice(&slab[src..src + PAYLOAD_BYTES_PER_SLOT]);
        }
        body.len = body
            .ops
            .iter()
            .rposition(|&b| b != PaletteOp::NOP.0)
            .map_or(0, |last| last as u16 + 1);
        body
    }
}

/// Read one operation **in place** from a node's value slab — no gather, no
/// copy, one indexed byte read.
///
/// This is the zero-copy read the substrate wants: a consumer that needs
/// operation `i` never materialises the other 359.
///
/// # Panics
///
/// When `index >= OPS_PER_FUNCTION`.
#[must_use]
pub const fn op_in_slab(slab: &[u8; VALUE_SLAB_LEN], index: usize) -> PaletteOp {
    PaletteOp(slab[FunctionBody::slab_offset(index)])
}

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
    /// One function's body — up to [`OPS_PER_FUNCTION`] palette bytes.
    Content,
}

impl SoaSplit {
    /// The concept whose classid this partition's rows carry.
    #[must_use]
    pub const fn concept(self) -> BlockConcept {
        match self {
            SoaSplit::Inventory => BlockConcept::Inventory,
            SoaSplit::Content => BlockConcept::Content,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let id = BlockConcept::Content.render_classid(0x1000);
        assert_eq!(id, 0x1701_1000);
        assert_eq!(id >> 16, u32::from(BlockConcept::Content.concept_id()));
        assert_eq!(id & 0xFFFF, 0x1000);
    }

    #[test]
    fn the_360_budget_is_derived_from_the_node_layout() {
        // 512-byte node = key(16) | edges(16) | value(480); value = 30 facet
        // slots of classid(4)+12. If any of those change, this must fail rather
        // than silently re-budget.
        assert_eq!(CONTENT_SLOTS, 480 / 16);
        assert_eq!(PAYLOAD_BYTES_PER_SLOT, 16 - 4);
        assert_eq!(OPS_PER_FUNCTION, 360);
    }

    #[test]
    fn a_body_at_the_budget_is_accepted_and_one_past_it_is_rejected() {
        // Two-sided: the cap must admit exactly 360 and refuse 361 — a cap that
        // only ever rejects, or only ever accepts, carries no information.
        let exact = vec![PaletteOp::ADD; OPS_PER_FUNCTION];
        let body = FunctionBody::from_ops(&exact).expect("360 ops must fit");
        assert_eq!(body.len(), OPS_PER_FUNCTION);
        assert_eq!(body.remaining(), 0);

        let over = vec![PaletteOp::ADD; OPS_PER_FUNCTION + 1];
        let err = FunctionBody::from_ops(&over).expect_err("361 ops must not fit");
        assert_eq!(err.offered, OPS_PER_FUNCTION + 1);
    }

    #[test]
    fn push_enforces_the_same_budget_as_from_ops() {
        // The cap must not be reachable by a back door: filling one op at a
        // time has to stop at exactly the same place.
        let mut body = FunctionBody::new();
        for _ in 0..OPS_PER_FUNCTION {
            body.push(PaletteOp::MUL).expect("within budget");
        }
        assert_eq!(body.len(), OPS_PER_FUNCTION);
        let err = body.push(PaletteOp::MUL).expect_err("the 361st must fail");
        assert_eq!(err.offered, OPS_PER_FUNCTION + 1);
        // and the failed push must not have mutated the body
        assert_eq!(body.len(), OPS_PER_FUNCTION);
    }

    #[test]
    fn ops_bytes_are_execution_order_nop_padded() {
        let body = FunctionBody::from_ops(&[PaletteOp::IF, PaletteOp::LT]).unwrap();
        let bytes = body.as_ops_bytes();
        assert_eq!(bytes.len(), OPS_PER_FUNCTION);
        assert_eq!(bytes[0], PaletteOp::IF.0);
        assert_eq!(bytes[1], PaletteOp::LT.0);
        // Everything past len is the zero-fallback, so a partially-filled body
        // needs no length field on the wire.
        assert!(bytes[2..].iter().all(|&b| b == PaletteOp::NOP.0));
    }

    #[test]
    fn the_slab_interleaves_classids_so_ops_are_not_contiguous() {
        // The defect this guards: the gathered array is NOT the slab layout.
        // Op i lives at stride 16, +4 into each facet — never at offset i.
        assert_eq!(FunctionBody::slab_offset(0), 4);
        assert_eq!(FunctionBody::slab_offset(11), 15); // last byte of facet 0
        assert_eq!(FunctionBody::slab_offset(12), 20); // facet 1 skips its classid
        assert_eq!(FunctionBody::slab_offset(OPS_PER_FUNCTION - 1), 479);

        // Anti-vacuity: the mapping must be a genuine permutation, not identity.
        let identity_matches = (0..OPS_PER_FUNCTION)
            .filter(|&i| FunctionBody::slab_offset(i) == i)
            .count();
        assert_eq!(
            identity_matches, 0,
            "no operation may sit at its own index; the slab interleaves"
        );

        // Every offset lands inside a payload lane, never on a classid byte.
        for i in 0..OPS_PER_FUNCTION {
            let off = FunctionBody::slab_offset(i);
            assert!(
                off % SLOT_STRIDE >= CLASSID_BYTES,
                "op {i} at slab offset {off} lands on a classid byte"
            );
            assert!(off < VALUE_SLAB_LEN);
        }
    }

    #[test]
    fn scatter_gather_round_trips_and_never_touches_a_classid() {
        let ops: Vec<PaletteOp> = (0..40u8).map(|i| PaletteOp(i.max(1))).collect();
        let body = FunctionBody::from_ops(&ops).unwrap();

        // Pre-stamp every facet's classid with a sentinel; the write must
        // leave all 120 of those bytes untouched — it writes operations, not
        // addressing.
        let mut slab = [0u8; VALUE_SLAB_LEN];
        for facet in 0..CONTENT_SLOTS {
            for b in 0..CLASSID_BYTES {
                slab[facet * SLOT_STRIDE + b] = 0xC1;
            }
        }
        body.write_into_value_slab(&mut slab);

        for facet in 0..CONTENT_SLOTS {
            for b in 0..CLASSID_BYTES {
                assert_eq!(
                    slab[facet * SLOT_STRIDE + b],
                    0xC1,
                    "facet {facet} classid byte {b} was overwritten"
                );
            }
        }

        // In-place read agrees with the gathered order, op by op.
        for (i, op) in ops.iter().enumerate() {
            assert_eq!(op_in_slab(&slab, i), *op, "op {i} misplaced in the slab");
        }

        // And the gather is the exact inverse.
        let back = FunctionBody::read_from_value_slab(&slab);
        assert_eq!(back.len(), ops.len());
        assert_eq!(back.as_ops_bytes(), body.as_ops_bytes());
    }

    #[test]
    fn a_naive_contiguous_copy_is_detectably_wrong() {
        // This is the bug an earlier doc comment would have caused: treating
        // the gathered 360 bytes as the front of the slab. It must be
        // observably different from the correct scatter, or the distinction
        // this API draws carries no information.
        let ops: Vec<PaletteOp> = (1..=30u8).map(PaletteOp).collect();
        let body = FunctionBody::from_ops(&ops).unwrap();

        let mut correct = [0u8; VALUE_SLAB_LEN];
        body.write_into_value_slab(&mut correct);

        let mut naive = [0u8; VALUE_SLAB_LEN];
        naive[..OPS_PER_FUNCTION].copy_from_slice(body.as_ops_bytes());

        assert_ne!(
            correct, naive,
            "scatter and contiguous copy must not coincide"
        );
        // Concretely: the naive copy puts op 0 on facet 0's FIRST classid byte.
        assert_eq!(naive[0], ops[0].0);
        assert_eq!(correct[0], 0, "facet 0's classid must stay untouched");
        assert_eq!(correct[FunctionBody::slab_offset(0)], ops[0].0);
    }

    #[test]
    fn in_memory_body_is_larger_than_the_wire_form() {
        // 362 = [u8; 360] + u16 len. The len is deliberately NOT written to the
        // slab — zero padding is the length signal — so the wire form is
        // exactly 360 and this gap must stay visible rather than surprising a
        // consumer that assumed size_of == payload size.
        assert_eq!(core::mem::size_of::<FunctionBody>(), 362);
        assert_eq!(body_wire_len(), OPS_PER_FUNCTION);
        assert_eq!(OPS_PER_FUNCTION, 360);
        assert_eq!(VALUE_SLAB_LEN, 480);
        assert_eq!(
            VALUE_SLAB_LEN - OPS_PER_FUNCTION,
            CONTENT_SLOTS * CLASSID_BYTES
        );
    }

    const fn body_wire_len() -> usize {
        core::mem::size_of::<[u8; OPS_PER_FUNCTION]>()
    }

    #[test]
    fn shared_core_and_device_family_partition_the_palette() {
        // Can-fire AND can-stay-silent on the same predicate: a classifier that
        // answers the same way for everything is worthless.
        assert!(PaletteOp::LT.is_shared_core());
        assert!(!PaletteOp::LT.is_device_family());

        let device = PaletteOp(DEVICE_FAMILY_FLOOR);
        assert!(device.is_device_family());
        assert!(!device.is_shared_core());

        // NOP is not an operation at all — neither bucket claims it.
        assert!(!PaletteOp::NOP.is_shared_core());
        assert!(!PaletteOp::NOP.is_device_family());
    }

    #[test]
    fn every_named_op_is_a_distinct_slot_in_the_shared_core() {
        // The whole value of the palette is that two frontends land on ONE
        // slot. A duplicate here would silently merge two operations; a slot at
        // or above the device floor would misclassify a shared op.
        let named: &[(&str, PaletteOp)] = &[
            ("IF", PaletteOp::IF),
            ("IF_ELSE", PaletteOp::IF_ELSE),
            ("REPEAT", PaletteOp::REPEAT),
            ("REPEAT_UNTIL", PaletteOp::REPEAT_UNTIL),
            ("WHILE", PaletteOp::WHILE),
            ("FOREVER", PaletteOp::FOREVER),
            ("FOR_EACH", PaletteOp::FOR_EACH),
            ("FOR_RANGE", PaletteOp::FOR_RANGE),
            ("WAIT", PaletteOp::WAIT),
            ("WAIT_UNTIL", PaletteOp::WAIT_UNTIL),
            ("STOP", PaletteOp::STOP),
            ("BREAK", PaletteOp::BREAK),
            ("CONTINUE", PaletteOp::CONTINUE),
            ("RETURN", PaletteOp::RETURN),
            ("AND", PaletteOp::AND),
            ("OR", PaletteOp::OR),
            ("NOT", PaletteOp::NOT),
            ("TRUE", PaletteOp::TRUE),
            ("FALSE", PaletteOp::FALSE),
            ("NULL", PaletteOp::NULL),
            ("TERNARY", PaletteOp::TERNARY),
            ("EQ", PaletteOp::EQ),
            ("NEQ", PaletteOp::NEQ),
            ("LT", PaletteOp::LT),
            ("LTE", PaletteOp::LTE),
            ("GT", PaletteOp::GT),
            ("GTE", PaletteOp::GTE),
            ("ADD", PaletteOp::ADD),
            ("SUB", PaletteOp::SUB),
            ("MUL", PaletteOp::MUL),
            ("DIV", PaletteOp::DIV),
            ("POW", PaletteOp::POW),
            ("MOD", PaletteOp::MOD),
            ("NUMBER", PaletteOp::NUMBER),
            ("ABS", PaletteOp::ABS),
            ("NEG", PaletteOp::NEG),
            ("ROUND", PaletteOp::ROUND),
            ("FLOOR", PaletteOp::FLOOR),
            ("CEIL", PaletteOp::CEIL),
            ("SQRT", PaletteOp::SQRT),
            ("LN", PaletteOp::LN),
            ("LOG10", PaletteOp::LOG10),
            ("EXP_E", PaletteOp::EXP_E),
            ("EXP_10", PaletteOp::EXP_10),
            ("SIN", PaletteOp::SIN),
            ("COS", PaletteOp::COS),
            ("TAN", PaletteOp::TAN),
            ("ASIN", PaletteOp::ASIN),
            ("ACOS", PaletteOp::ACOS),
            ("ATAN", PaletteOp::ATAN),
            ("ATAN2", PaletteOp::ATAN2),
            ("RANDOM_INT", PaletteOp::RANDOM_INT),
            ("RANDOM_FLOAT", PaletteOp::RANDOM_FLOAT),
            ("CONSTRAIN", PaletteOp::CONSTRAIN),
            ("NUMBER_PROPERTY", PaletteOp::NUMBER_PROPERTY),
            ("CONSTANT", PaletteOp::CONSTANT),
            ("ON_LIST", PaletteOp::ON_LIST),
            ("TEXT", PaletteOp::TEXT),
            ("JOIN", PaletteOp::JOIN),
            ("LENGTH", PaletteOp::LENGTH),
            ("CHAR_AT", PaletteOp::CHAR_AT),
            ("INDEX_OF", PaletteOp::INDEX_OF),
            ("IS_EMPTY", PaletteOp::IS_EMPTY),
            ("SUBSTRING", PaletteOp::SUBSTRING),
            ("CHANGE_CASE", PaletteOp::CHANGE_CASE),
            ("TRIM", PaletteOp::TRIM),
            ("CONTAINS", PaletteOp::CONTAINS),
            ("APPEND", PaletteOp::APPEND),
            ("PRINT", PaletteOp::PRINT),
            ("PROMPT", PaletteOp::PROMPT),
            ("COUNT", PaletteOp::COUNT),
            ("REPLACE", PaletteOp::REPLACE),
            ("REVERSE", PaletteOp::REVERSE),
            ("LIST_EMPTY", PaletteOp::LIST_EMPTY),
            ("LIST_WITH", PaletteOp::LIST_WITH),
            ("LIST_REPEAT", PaletteOp::LIST_REPEAT),
            ("LIST_LENGTH", PaletteOp::LIST_LENGTH),
            ("LIST_IS_EMPTY", PaletteOp::LIST_IS_EMPTY),
            ("LIST_INDEX_OF", PaletteOp::LIST_INDEX_OF),
            ("LIST_GET", PaletteOp::LIST_GET),
            ("LIST_SET", PaletteOp::LIST_SET),
            ("LIST_INSERT", PaletteOp::LIST_INSERT),
            ("LIST_ADD", PaletteOp::LIST_ADD),
            ("LIST_DELETE", PaletteOp::LIST_DELETE),
            ("LIST_DELETE_ALL", PaletteOp::LIST_DELETE_ALL),
            ("LIST_SUBLIST", PaletteOp::LIST_SUBLIST),
            ("LIST_SPLIT", PaletteOp::LIST_SPLIT),
            ("LIST_SORT", PaletteOp::LIST_SORT),
            ("LIST_CONTAINS", PaletteOp::LIST_CONTAINS),
            ("VAR_GET", PaletteOp::VAR_GET),
            ("VAR_SET", PaletteOp::VAR_SET),
            ("VAR_CHANGE", PaletteOp::VAR_CHANGE),
            ("PROC_DEF", PaletteOp::PROC_DEF),
            ("PROC_CALL", PaletteOp::PROC_CALL),
            ("PROC_ARG", PaletteOp::PROC_ARG),
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
    }

    #[test]
    fn soa_split_maps_each_partition_to_its_own_concept() {
        // Inventory and Content must NOT share a classid — the whole point of
        // the split is that a registry read never touches a body.
        assert_eq!(SoaSplit::Inventory.concept(), BlockConcept::Inventory);
        assert_eq!(SoaSplit::Content.concept(), BlockConcept::Content);
        assert_ne!(
            SoaSplit::Inventory.concept().concept_id(),
            SoaSplit::Content.concept().concept_id()
        );
    }
}
