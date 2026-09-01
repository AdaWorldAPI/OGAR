//! `ogar-loco` — the **low-code program surface**: the vocabulary-agnostic
//! call ABI that every block/template/flow frontend shares.
//!
//! # What this is, and why it exists
//!
//! The block-editor arc (the `blockly-rs` consumers) proved a
//! storage shape for programs on the V3 substrate. The operator direction that
//! created this crate generalizes it: *elixir-shaped templates are "just a
//! rails-shaped semantic over classid index, 256:256 — not much different than
//! blockly, just different vocabulary — a reusable surface for any other
//! purposes."* Power-Automate-style flows are the third consumer in line.
//!
//! So the surface splits in two:
//!
//! - **This crate** — everything that is the same no matter what the bytes
//!   *mean*: the node layout, the lane carvings, the call encoding, the
//!   budgets, the refuse-don't-truncate guards, the shared computational
//!   core's tables, the constant pool, the program/reference rules, and the
//!   [`Vocabulary`] seam a sibling codebook plugs into.
//! - **A vocabulary per domain, declared by whoever owns it** — the
//!   palette *meanings* above the shared core, value-parameter codebooks,
//!   frontend membranes (Blockly JSON, a template DSL, flow JSON), and
//!   lowering from that frontend's records.
//!
//! # The shape: everything is a call — `(function : value)`
//!
//! The V3 substrate reads every 12-byte payload as rails; program content
//! takes the indexed reading, and the pair is **[`Call`] = `(function :
//! value)`** — each byte an index into a 256-entry codebook, which is what
//! makes every call an address into a 256×256 table:
//!
//! ```text
//!   one function  =  one node  =  512 bytes
//!     key    slot 0        classid = the content concept · identity = which function
//!     slot 1               reserved (16 B, zeroed; the retired edge-block
//!                          design is NOT revived — relations ride the
//!                          payload rails as indexed calls)
//!     value  slots 2..31   30 lanes, each carved by the body's LaneShape:
//!                          6×(fn:val) · 4×(fn:val:val) · 3×(fn:val:val:val)
//!                          → 180 / 120 / 90 calls, always 360 bytes
//! ```
//!
//! **There is no "opcode" distinct from a "function call".** `ADD` is function
//! `0x40`; a user-defined function is another index in the same `<256`
//! codebook; invoking either is the same two bytes — see [`FnIndex`].
//!
//! ## Arity — two mechanisms, and they compose
//!
//! **1. The classid widens the lane.** A 12-byte lane carves three sanctioned
//! ways, and the classid selects which ([`LaneShape`], mirroring the LE
//! contract's `CascadeShape`):
//!
//! | shape | carving | per call | calls / node |
//! |---|---|---|---|
//! | [`LaneShape::Pairs`] | `6 × (u8:u8)` | `function : value` | **180** |
//! | [`LaneShape::Triples`] | `4 × (u8:u8:u8)` | `function : value : value` | **120** |
//! | [`LaneShape::Quads`] | `3 × (u8:u8:u8:u8)` | `function : value ×3` | **90** |
//!
//! A function needing more than one immediate does not get a wider *field* —
//! its class picks a wider *carving* of the same 12 bytes. Byte budget is
//! constant at 360; only the call count moves.
//!
//! **2. The stack carries nested expressions.** Immediates are what the value
//! bytes hold; *computed* arguments come from a stack discipline — each call
//! consumes its operands and pushes its result, so `5 + 3` is
//! `(NUMBER:5) (NUMBER:3) (ADD:0)` in any shape.
//!
//! Either way every call stays **independently readable**: call `i` is at a
//! computed offset ([`FunctionBody::call_slab_offset`]) with no scan from the
//! start — a property a variable-arity or immediate-following encoding would
//! destroy. (This is the one deliberate divergence from otherwise-kindred
//! public designs like Wasm's LEB128 stream: fixed lanes buy addressability.)
//!
//! ## Nesting is by reference, not by delimiter
//!
//! A function index can name *another function*, so `IF` calls a body living
//! in its own node. No `END` marker, no jump offset — the same model as SB3
//! block references and Forth threaded code. A loop body of any length costs
//! its parent exactly one byte.
//!
//! ## Budgets
//!
//! A body is capped at [`LaneShape::calls_per_function`] and the cap is
//! enforced ([`FunctionBody::push`] / [`FunctionBody::from_calls`]).
//! Over-length is a **split into two functions**, never a bigger row — the
//! substrate's rule (*scale is the next cascade level, never field-widening*)
//! applied to program structure. The codebook is capped at **`<256` functions
//! per scope** by the same logic: one byte names any function in scope, and
//! scopes cascade rather than widen.
//!
//! ## Wide literals
//!
//! A value byte holds `0..=255`. A call needing more (`WAIT:1.5`, a string)
//! spends its value byte as a **constant-pool index** ([`pool`]) instead of
//! the value — same pair shape, different codebook.
//!
//! # The sharing discipline (load-bearing)
//!
//! Bytes **below [`DOMAIN_FLOOR`]** are the **shared computational core**:
//! they mean the same thing in every vocabulary, their arity/body-reference
//! tables live ONCE in [`vocabulary::shared_core`], and a sibling vocabulary
//! cannot answer for them — that is what keeps `IF` from quietly meaning two
//! things in two domains. Bytes at/above the floor belong to the vocabulary
//! the classid selects. [`vocabulary::conformance`] is the mechanical check.
//!
//! # What this crate deliberately does NOT do
//!
//! - **No concept mints.** Content/Inventory concept ids per domain are
//!   operator decisions with ledger entries; vocabulary crates carry them
//!   (this crate's own `0x1701`/`0x1702` are the precedent).
//! - **No GUID minting.** [`node::FunctionNode`] round-trips an opaque key.
//! - **No frontend lowering.** Casting a Blockly record / template DSL / flow
//!   JSON into calls is the vocabulary crate's job; this crate holds the
//!   [`Program`] shape and the reference rules those lowerings must satisfy.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod node;
pub mod pool;
pub mod program;
pub mod registry;
pub mod statements;
pub mod telemetry;
pub mod vocabulary;

pub use node::FunctionNode;
pub use pool::{Constant, ConstantPool, PoolError};
pub use program::{Program, branches_of};
pub use registry::{RegistryError, VocabularyRegistry};
pub use statements::{StatementBounds, StatementError, statement_bounds};
pub use telemetry::{FunnelTally, RefusalGate};
pub use vocabulary::conformance::CheckedVocabulary;
pub use vocabulary::{FnSpec, ValueCodebook, Vocabulary, VocabularyTable};

// ── The universal node concepts ─────────────────────────────────────────────

/// The **global** concepts this crate owns: the node shapes EVERY sibling
/// vocabulary stores into, whatever palette it speaks.
///
/// These are not a frontend's property. A Blockly script body, an
/// elixir-shaped thinking template, and an RO relation body are all the same
/// [`FunctionBody`] in the same 512-byte [`FunctionNode`] — they differ only
/// in which [`Vocabulary`] resolves their bytes. So the classid that names
/// *"a function body"* belongs here, at the substrate, and a frontend
/// references it rather than minting its own.
///
/// This is the global half of the split: `ogar-loco` is global interest
/// (thinking orchestration rides the same call ABI), whereas a particular
/// palette — Blockly/Scratch opcodes, `blockly-rs`'s `0x1717` — is
/// activated only in a build that actually contains that frontend.
///
/// # `0x17` is LOCO's domain, and consumers are seated above
///
/// The whole `0x17XX` block belongs to this substrate (operator, 2026-08-07).
/// The layout inside it:
///
/// | range | who | for what |
/// |---|---|---|
/// | `0x1701` / `0x1702` | **loco** | the node shapes — body + inventory |
/// | `0x1703`–`0x1716` | **loco, reserved** | headroom for the substrate's own growth — uplifting, Klickwege, whatever the ABI needs next |
/// | `0x1717`+ | **consumers** | one slot per frontend palette (`blockly-rs` = `0x1717`) |
///
/// Consumers were deliberately seated *high* so the substrate keeps
/// contiguous room beneath them. A frontend that outgrows a single slot gets
/// its **own domain** rather than eating into that headroom — the same
/// "scale = the next cascade level, never field-widening" rule the GUID canon
/// uses.
///
/// The `ogar_vocab::ConceptDomain::Blocks` label on `0x17` predates this and
/// now reads narrowly (the domain is the substrate's, not one frontend's).
/// Renaming a reserved domain is a canon change and is left to the operator;
/// nothing here depends on the label, only on the `id >> 8` route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum LocoConcept {
    /// `0x1701` — a **function body**: identity names the function, the value
    /// slab carries up to [`LaneShape::calls_per_function`] calls. The one
    /// content classid, shared by every vocabulary.
    FunctionBody,
    /// `0x1702` — the **inventory** row: the function registry entry (which
    /// functions exist, addressed by identity). A registry read never touches
    /// a body — the V3 mailbox split, and it is the same split regardless of
    /// palette.
    Inventory,
}

impl LocoConcept {
    /// Every concept, in id order — the enumeration hook a consumer uses
    /// instead of hand-maintaining a parallel list.
    pub const ALL: [LocoConcept; 2] = [LocoConcept::FunctionBody, LocoConcept::Inventory];

    /// This concept's canonical id.
    ///
    /// Authoritative HERE. Whether these are additionally promoted into
    /// `ogar_vocab`'s shared codebook is a separate, operator-ruled canon
    /// decision — this crate does not assume it, and nothing here breaks if
    /// they are not.
    #[must_use]
    pub const fn concept_id(self) -> u16 {
        match self {
            LocoConcept::FunctionBody => 0x1701,
            LocoConcept::Inventory => 0x1702,
        }
    }

    /// The full V3 render classid under a consumer's app prefix — canon-high
    /// `(concept as u32) << 16 | app_prefix`.
    #[must_use]
    pub const fn render_classid(self, app_prefix: u16) -> u32 {
        ((self.concept_id() as u32) << 16) | (app_prefix as u32)
    }
}

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
/// [`BODY_BYTES`] = 360 of them are call payload (180 / 120 / 90 calls,
/// depending on the [`LaneShape`]). The other 120 are the 30 facets' 4-byte
/// classids, interleaved — never a contiguous run.
pub const VALUE_SLAB_LEN: usize = CONTENT_SLOTS * SLOT_STRIDE;

/// Payload bytes one function body carries: `30 × 12` = **360**.
///
/// A derived budget, not a chosen constant — exactly the payload capacity of a
/// node's value slab. Constant across every [`LaneShape`]; what changes with
/// the shape is how many CALLS those bytes hold, never how many bytes there
/// are.
pub const BODY_BYTES: usize = CONTENT_SLOTS * PAYLOAD_BYTES_PER_SLOT;

const _: () = assert!(
    BODY_BYTES == 360,
    "360 = 30 value-slab facet slots × 12 payload bytes each"
);

/// How a 12-byte lane is carved into calls — selected by the body's **classid**,
/// uniform within one body.
///
/// Mirrors the LE contract's `CascadeShape` (`G6D2` / `G4D3` / `G3D4`); defined
/// locally so this crate keeps its plug-and-play posture and takes no
/// substrate dependency.
///
/// Every shape spends the same 12 bytes per lane and the same [`BODY_BYTES`]
/// per node. A function needing more immediates picks a wider **carving**, not
/// a wider field — the canon's *scale is the next cascade level, never
/// field-widening*, applied one level down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LaneShape {
    /// `6 × (u8:u8)` — `function : value`. One immediate per call, 180 calls
    /// per node. The default: most calls take zero or one immediate.
    #[default]
    Pairs,
    /// `4 × (u8:u8:u8)` — `function : value : value`. Two immediates, 120 calls.
    Triples,
    /// `3 × (u8:u8:u8:u8)` — `function : value × 3`. Three immediates, 90 calls.
    Quads,
}

impl LaneShape {
    /// Every shape, widest call first.
    pub const ALL: [LaneShape; 3] = [LaneShape::Quads, LaneShape::Triples, LaneShape::Pairs];

    /// Bytes one call occupies: 2, 3, or 4.
    #[must_use]
    pub const fn bytes_per_call(self) -> usize {
        match self {
            LaneShape::Pairs => 2,
            LaneShape::Triples => 3,
            LaneShape::Quads => 4,
        }
    }

    /// Immediate value bytes one call carries: 1, 2, or 3 (the call's byte
    /// width minus its one function-index byte).
    #[must_use]
    pub const fn values_per_call(self) -> usize {
        self.bytes_per_call() - 1
    }

    /// Calls in one 12-byte lane: 6, 4, or 3.
    #[must_use]
    pub const fn calls_per_lane(self) -> usize {
        PAYLOAD_BYTES_PER_SLOT / self.bytes_per_call()
    }

    /// Calls one function body carries: `30 × calls_per_lane` = 180 / 120 / 90.
    #[must_use]
    pub const fn calls_per_function(self) -> usize {
        CONTENT_SLOTS * self.calls_per_lane()
    }
}

// Every shape divides the 12-byte lane exactly — no remainder, no dead bytes.
const _: () = assert!(LaneShape::Pairs.calls_per_lane() * 2 == PAYLOAD_BYTES_PER_SLOT);
const _: () = assert!(LaneShape::Triples.calls_per_lane() * 3 == PAYLOAD_BYTES_PER_SLOT);
const _: () = assert!(LaneShape::Quads.calls_per_lane() * 4 == PAYLOAD_BYTES_PER_SLOT);
const _: () = assert!(LaneShape::Pairs.calls_per_function() == 180);
const _: () = assert!(LaneShape::Triples.calls_per_function() == 120);
const _: () = assert!(LaneShape::Quads.calls_per_function() == 90);

// ── The codebook floor ──────────────────────────────────────────────────────

/// First codebook slot that belongs to the **vocabulary**, not the shared
/// core.
///
/// Slots below this floor are the **shared computational core**: every one of
/// them means the same thing in every vocabulary, and their tables live once
/// in [`vocabulary::shared_core`]. `slot >= DOMAIN_FLOOR` is therefore a
/// one-compare test for "this op is vocabulary-specific", which a renderer,
/// validator, or compiler can branch on without a table lookup.
///
/// # This boundary is PERMANENT, not today's allocation
///
/// `0x00..=0x8F` is the universal ABI **forever**; `0x90..=0xFF` is
/// vocabulary-local **forever**. Stored programs encode the boundary
/// implicitly in every function byte they carry: moving the floor would
/// silently reinterpret persisted bytes — a shared-core opcode becoming
/// vocabulary-local (or the reverse) under a reader that never changed the
/// data. That is the same layout-reclaim hazard the substrate's
/// reserve-don't-reclaim rule exists to forbid. Unallocated shared-core
/// slots (`0x0F..0x1F` gaps, `0x86..0x8F`, …) stay reserved for the CORE to
/// mint; unallocated domain slots stay reserved for each vocabulary. Neither
/// side ever annexes the other's range.
///
/// In the first vocabulary (the `blockly-rs` palette) the domain range hosts the
/// Scratch-style *device families* (motion, looks, sound, …) and the constant
/// is re-exported there under its historical name `DEVICE_FAMILY_FLOOR`.
pub const DOMAIN_FLOOR: u8 = 0x90;

// The permanence pin: changing the floor MUST fail compilation here, and the
// failure message must say why the "fix" is wrong.
const _: () = assert!(
    DOMAIN_FLOOR == 0x90,
    "DOMAIN_FLOOR is stored-byte ABI: moving it reinterprets every persisted \
     program; mint inside the existing ranges instead"
);

/// An index into the **function codebook** — one byte that names any callable
/// thing in scope.
///
/// There is no opcode/function distinction: the named constants below are the
/// primitive low range (the shared computational core) of the same `<256`
/// codebook that user-defined functions mint into, resolved through the
/// vocabulary's inventory registry (see the `blockly-rs` `SoaSplit` for the
/// first concrete registry split). A [`Call`]'s first byte is a `FnIndex`; an
/// editor's pick-from palette is a *rendering* of this codebook.
///
/// `0x00` is reserved as the zero-fallback: an unwritten payload byte reads as
/// [`FnIndex::NOP`], so a partially-filled body is well-defined without a
/// length field. This mirrors the substrate's monotonic zero ladder (a zero
/// tier means *not consulted*, never *compacted away*).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(transparent)]
pub struct FnIndex(pub u8);

impl FnIndex {
    /// The zero slot — an unwritten body byte. Never a real operation.
    pub const NOP: FnIndex = FnIndex(0x00);

    // ── control (0x01..0x1F) ────────────────────────────────────────────
    /// Conditional with no else arm. `controls_if` · `control_if`.
    pub const IF: FnIndex = FnIndex(0x01);
    /// Conditional with an else arm. `controls_ifelse` · `control_if_else`.
    pub const IF_ELSE: FnIndex = FnIndex(0x02);
    /// Bounded repeat. `controls_repeat`/`_ext` · `control_repeat`.
    pub const REPEAT: FnIndex = FnIndex(0x03);
    /// Repeat until a condition holds. `controls_whileUntil[UNTIL]` ·
    /// `control_repeat_until`.
    pub const REPEAT_UNTIL: FnIndex = FnIndex(0x04);
    /// Repeat while a condition holds. `controls_whileUntil[WHILE]` ·
    /// `control_while`.
    pub const WHILE: FnIndex = FnIndex(0x05);
    /// Unbounded repeat. `control_forever` (no Blockly counterpart).
    pub const FOREVER: FnIndex = FnIndex(0x06);
    /// Iterate a list. `controls_forEach` · `control_for_each`.
    pub const FOR_EACH: FnIndex = FnIndex(0x07);
    /// Iterate a numeric range. `controls_for` (no Scratch counterpart).
    pub const FOR_RANGE: FnIndex = FnIndex(0x08);
    /// Suspend for a duration. `control_wait`.
    pub const WAIT: FnIndex = FnIndex(0x09);
    /// Suspend until a condition holds. `control_wait_until`.
    pub const WAIT_UNTIL: FnIndex = FnIndex(0x0A);
    /// Stop this script / all / others. `control_stop`.
    pub const STOP: FnIndex = FnIndex(0x0B);
    /// Leave the enclosing loop. `controls_flow_statements[BREAK]`.
    pub const BREAK: FnIndex = FnIndex(0x0C);
    /// Skip to the enclosing loop's next iteration.
    /// `controls_flow_statements[CONTINUE]`.
    pub const CONTINUE: FnIndex = FnIndex(0x0D);
    /// Return from the enclosing function. `procedures_ifreturn`.
    pub const RETURN: FnIndex = FnIndex(0x0E);

    // ── logic (0x20..0x2F) ──────────────────────────────────────────────
    /// Boolean conjunction. `logic_operation[AND]` · `operator_and`.
    pub const AND: FnIndex = FnIndex(0x20);
    /// Boolean disjunction. `logic_operation[OR]` · `operator_or`.
    pub const OR: FnIndex = FnIndex(0x21);
    /// Boolean negation. `logic_negate` · `operator_not`.
    pub const NOT: FnIndex = FnIndex(0x22);
    /// Literal true. `logic_boolean[TRUE]`.
    pub const TRUE: FnIndex = FnIndex(0x23);
    /// Literal false. `logic_boolean[FALSE]`.
    pub const FALSE: FnIndex = FnIndex(0x24);
    /// Literal null. `logic_null` (no Scratch counterpart).
    pub const NULL: FnIndex = FnIndex(0x25);
    /// Conditional expression. `logic_ternary` (no Scratch counterpart).
    pub const TERNARY: FnIndex = FnIndex(0x26);

    // ── comparison (0x30..0x3F) ─────────────────────────────────────────
    /// Equality. `logic_compare[EQ]` · `operator_equals`.
    pub const EQ: FnIndex = FnIndex(0x30);
    /// Inequality. `logic_compare[NEQ]` (no Scratch counterpart).
    pub const NEQ: FnIndex = FnIndex(0x31);
    /// Less than. `logic_compare[LT]` · `operator_lt`.
    pub const LT: FnIndex = FnIndex(0x32);
    /// Less than or equal. `logic_compare[LTE]` (no Scratch counterpart).
    pub const LTE: FnIndex = FnIndex(0x33);
    /// Greater than. `logic_compare[GT]` · `operator_gt`.
    pub const GT: FnIndex = FnIndex(0x34);
    /// Greater than or equal. `logic_compare[GTE]` (no Scratch counterpart).
    pub const GTE: FnIndex = FnIndex(0x35);

    // ── math (0x40..0x5F) ───────────────────────────────────────────────
    /// Addition. `math_arithmetic[ADD]` · `operator_add`.
    pub const ADD: FnIndex = FnIndex(0x40);
    /// Subtraction. `math_arithmetic[MINUS]` · `operator_subtract`.
    pub const SUB: FnIndex = FnIndex(0x41);
    /// Multiplication. `math_arithmetic[MULTIPLY]` · `operator_multiply`.
    pub const MUL: FnIndex = FnIndex(0x42);
    /// Division. `math_arithmetic[DIVIDE]` · `operator_divide`.
    pub const DIV: FnIndex = FnIndex(0x43);
    /// Exponentiation. `math_arithmetic[POWER]` (no Scratch counterpart).
    pub const POW: FnIndex = FnIndex(0x44);
    /// Modulo. `math_modulo` · `operator_mod`.
    pub const MOD: FnIndex = FnIndex(0x45);
    /// Numeric literal. `math_number` (Scratch uses a field, not a block).
    pub const NUMBER: FnIndex = FnIndex(0x46);
    /// Absolute value. `math_single[ABS]` · `operator_mathop[abs]`.
    pub const ABS: FnIndex = FnIndex(0x47);
    /// Negation. `math_single[NEG]`.
    pub const NEG: FnIndex = FnIndex(0x48);
    /// Round to nearest. `math_round[ROUND]` · `operator_round`.
    pub const ROUND: FnIndex = FnIndex(0x49);
    /// Round toward -inf. `math_round[ROUNDDOWN]` · `operator_mathop[floor]`.
    pub const FLOOR: FnIndex = FnIndex(0x4A);
    /// Round toward +inf. `math_round[ROUNDUP]` · `operator_mathop[ceiling]`.
    pub const CEIL: FnIndex = FnIndex(0x4B);
    /// Square root. `math_single[ROOT]` · `operator_mathop[sqrt]`.
    pub const SQRT: FnIndex = FnIndex(0x4C);
    /// Natural logarithm. `math_single[LN]` · `operator_mathop[ln]`.
    pub const LN: FnIndex = FnIndex(0x4D);
    /// Base-10 logarithm. `math_single[LOG10]` · `operator_mathop[log]`.
    pub const LOG10: FnIndex = FnIndex(0x4E);
    /// `e^x`. `math_single[EXP]` · `operator_mathop[e ^]`.
    pub const EXP_E: FnIndex = FnIndex(0x4F);
    /// `10^x`. `math_single[POW10]` · `operator_mathop[10 ^]`.
    pub const EXP_10: FnIndex = FnIndex(0x50);
    /// Sine. `math_trig[SIN]` · `operator_mathop[sin]`.
    pub const SIN: FnIndex = FnIndex(0x51);
    /// Cosine. `math_trig[COS]` · `operator_mathop[cos]`.
    pub const COS: FnIndex = FnIndex(0x52);
    /// Tangent. `math_trig[TAN]` · `operator_mathop[tan]`.
    pub const TAN: FnIndex = FnIndex(0x53);
    /// Arcsine. `math_trig[ASIN]` · `operator_mathop[asin]`.
    pub const ASIN: FnIndex = FnIndex(0x54);
    /// Arccosine. `math_trig[ACOS]` · `operator_mathop[acos]`.
    pub const ACOS: FnIndex = FnIndex(0x55);
    /// Arctangent. `math_trig[ATAN]` · `operator_mathop[atan]`.
    pub const ATAN: FnIndex = FnIndex(0x56);
    /// Two-argument arctangent. `math_atan2` (no Scratch counterpart).
    pub const ATAN2: FnIndex = FnIndex(0x57);
    /// Random integer in a range. `math_random_int` · `operator_random`.
    pub const RANDOM_INT: FnIndex = FnIndex(0x58);
    /// Random fraction. `math_random_float` (no Scratch counterpart).
    pub const RANDOM_FLOAT: FnIndex = FnIndex(0x59);
    /// Clamp to a range. `math_constrain` (no Scratch counterpart).
    pub const CONSTRAIN: FnIndex = FnIndex(0x5A);
    /// Numeric predicate (even/odd/prime/whole/positive/negative/divisible).
    /// `math_number_property` (no Scratch counterpart).
    pub const NUMBER_PROPERTY: FnIndex = FnIndex(0x5B);
    /// Named constant (pi/e/phi/sqrt2/sqrt1_2/infinity). `math_constant`.
    pub const CONSTANT: FnIndex = FnIndex(0x5C);
    /// Aggregate over a list (sum/min/max/average/median/mode/std_dev).
    /// `math_on_list` (no Scratch counterpart).
    pub const ON_LIST: FnIndex = FnIndex(0x5D);

    // ── text (0x60..0x6F) ───────────────────────────────────────────────
    /// String literal. `text`.
    pub const TEXT: FnIndex = FnIndex(0x60);
    /// Concatenate. `text_join` · `operator_join`.
    pub const JOIN: FnIndex = FnIndex(0x61);
    /// Character count. `text_length` · `operator_length`.
    pub const LENGTH: FnIndex = FnIndex(0x62);
    /// Character at a position. `text_charAt` · `operator_letter_of`.
    pub const CHAR_AT: FnIndex = FnIndex(0x63);
    /// Substring search. `text_indexOf` (no Scratch counterpart).
    pub const INDEX_OF: FnIndex = FnIndex(0x64);
    /// Emptiness test. `text_isEmpty` (no Scratch counterpart).
    pub const IS_EMPTY: FnIndex = FnIndex(0x65);
    /// Substring extraction. `text_getSubstring` (no Scratch counterpart).
    pub const SUBSTRING: FnIndex = FnIndex(0x66);
    /// Case conversion. `text_changeCase` (no Scratch counterpart).
    pub const CHANGE_CASE: FnIndex = FnIndex(0x67);
    /// Whitespace trim. `text_trim` (no Scratch counterpart).
    pub const TRIM: FnIndex = FnIndex(0x68);
    /// Containment test. `text_contains`-shaped · `operator_contains`.
    pub const CONTAINS: FnIndex = FnIndex(0x69);
    /// Append to a variable. `text_append`.
    pub const APPEND: FnIndex = FnIndex(0x6A);
    /// Emit to output. `text_print`.
    pub const PRINT: FnIndex = FnIndex(0x6B);
    /// Prompt for input. `text_prompt`/`_ext`.
    pub const PROMPT: FnIndex = FnIndex(0x6C);
    /// Occurrence count. `text_count` (no Scratch counterpart).
    pub const COUNT: FnIndex = FnIndex(0x6D);
    /// Substring replacement. `text_replace` (no Scratch counterpart).
    pub const REPLACE: FnIndex = FnIndex(0x6E);
    /// Reversal. `text_reverse` (no Scratch counterpart).
    pub const REVERSE: FnIndex = FnIndex(0x6F);

    // ── list (0x70..0x7F) ───────────────────────────────────────────────
    /// Empty list literal. `lists_create_empty`.
    pub const LIST_EMPTY: FnIndex = FnIndex(0x70);
    /// List literal with items. `lists_create_with`.
    pub const LIST_WITH: FnIndex = FnIndex(0x71);
    /// Repeat an item into a list. `lists_repeat`.
    pub const LIST_REPEAT: FnIndex = FnIndex(0x72);
    /// Item count. `lists_length` · `data_lengthoflist`.
    pub const LIST_LENGTH: FnIndex = FnIndex(0x73);
    /// Emptiness test. `lists_isEmpty`.
    pub const LIST_IS_EMPTY: FnIndex = FnIndex(0x74);
    /// Position of an item. `lists_indexOf` · `data_itemnumoflist`.
    pub const LIST_INDEX_OF: FnIndex = FnIndex(0x75);
    /// Read an item. `lists_getIndex` · `data_itemoflist`.
    pub const LIST_GET: FnIndex = FnIndex(0x76);
    /// Write an item. `lists_setIndex[SET]` · `data_replaceitemoflist`.
    pub const LIST_SET: FnIndex = FnIndex(0x77);
    /// Insert an item. `lists_setIndex[INSERT]` · `data_insertatlist`.
    pub const LIST_INSERT: FnIndex = FnIndex(0x78);
    /// Append an item. `data_addtolist`.
    pub const LIST_ADD: FnIndex = FnIndex(0x79);
    /// Remove an item. `lists_getIndex[REMOVE]` · `data_deleteoflist`.
    pub const LIST_DELETE: FnIndex = FnIndex(0x7A);
    /// Remove every item. `data_deletealloflist`.
    pub const LIST_DELETE_ALL: FnIndex = FnIndex(0x7B);
    /// Sublist extraction. `lists_getSublist` (no Scratch counterpart).
    pub const LIST_SUBLIST: FnIndex = FnIndex(0x7C);
    /// Split / join against a delimiter. `lists_split`.
    pub const LIST_SPLIT: FnIndex = FnIndex(0x7D);
    /// Ordering. `lists_sort` (no Scratch counterpart).
    pub const LIST_SORT: FnIndex = FnIndex(0x7E);
    /// Containment test. `lists_indexOf`-shaped · `data_listcontainsitem`.
    pub const LIST_CONTAINS: FnIndex = FnIndex(0x7F);

    // ── variable + procedure (0x80..0x8F) ───────────────────────────────
    /// Read a variable. `variables_get` · `data_variable`.
    pub const VAR_GET: FnIndex = FnIndex(0x80);
    /// Write a variable. `variables_set` · `data_setvariableto`.
    pub const VAR_SET: FnIndex = FnIndex(0x81);
    /// Increment a variable. `math_change` · `data_changevariableby`.
    pub const VAR_CHANGE: FnIndex = FnIndex(0x82);
    /// Define a function. `procedures_defnoreturn`/`_defreturn` ·
    /// `procedures_definition`.
    pub const PROC_DEF: FnIndex = FnIndex(0x83);
    /// Invoke a function. `procedures_callnoreturn`/`_callreturn` ·
    /// `procedures_call`.
    pub const PROC_CALL: FnIndex = FnIndex(0x84);
    /// Read a call argument. `procedures_defreturn` argument access.
    pub const PROC_ARG: FnIndex = FnIndex(0x85);

    // ── epistemic mask algebra (0x86..0x8B, minted 2026-09-01) ──────────
    //
    // The operator's essence-directive, verbatim in intent: primitives like
    // Belnap become reusable bytecode macros here, for literally everything.
    // Semantics are owned by `lance-graph-contract::epistemic_bassin` (the
    // scalar oracles these calls are parity-tested against); batch SIMD
    // execution is ndarray's shipped W1a-#9 `ternlog` (native VPTERNLOGQ).
    // Green-certified atoms only — Hambly-Lyons is deliberately ABSENT
    // while jc Pillar 11 is red (the red-pillar mint rule).

    /// Any 3-input boolean over stacked masks: pops three masks, and the
    /// call's ONE VALUE BYTE **is the 8-bit truth table** (IMM bit index =
    /// `(a << 2) | (b << 1) | c`) — one FnIndex covers all 256 stacked-mask
    /// combinators, which is the purest `(function : value)` in the ABI.
    /// Canonical tables (asked-contested 0x80, asked-silent 0x02 = the
    /// missing link, …) live in `epistemic_bassin::sweep_ternlog`.
    pub const TERNLOG: FnIndex = FnIndex(0x86);
    /// Belnap/FDE knowledge-order join of two mask PAIRS — bitwise OR per
    /// side; provably the state layer of one-hop accumulation.
    pub const BELNAP_JOIN: FnIndex = FnIndex(0x87);
    /// Shannon expected-information-gain readout: pops (before, after)
    /// candidate counts, pushes whole bits of narrowing (u4-saturated).
    /// Proprioception, never evidence.
    pub const INFO_GAIN: FnIndex = FnIndex(0x88);
    /// EWA tension readout: pops (log-norm growth, certificate bound),
    /// pushes quarters-of-the-bound (u4; 7 = the 1.75× PASS slack).
    pub const SIGMA_TENSION: FnIndex = FnIndex(0x89);
    /// One-hop accumulation of a LIST of bassin pairs into the parent pair —
    /// exact sums per side, one clamp; a parent expresses its DIRECT
    /// children only.
    pub const ACCUMULATE: FnIndex = FnIndex(0x8A);
    /// One-hop contested-ness: pops (list of pairs, axis), pushes the
    /// four-state Shannon entropy of the children's stances on that axis.
    pub const STANCE_ENTROPY: FnIndex = FnIndex(0x8B);

    /// Is this a **shared computational** operation — one that means the same
    /// thing in every vocabulary?
    ///
    /// A one-compare test, no table lookup: everything below [`DOMAIN_FLOOR`]
    /// is shared. [`FnIndex::NOP`] is not an operation and answers `false`.
    #[must_use]
    pub const fn is_shared_core(self) -> bool {
        self.0 != 0 && self.0 < DOMAIN_FLOOR
    }

    /// Is this a **vocabulary-specific** operation — one whose meaning comes
    /// from the classid-selected vocabulary rather than the shared core?
    ///
    /// (In the first vocabulary, the `blockly-rs` palette, this range hosts the
    /// Scratch-style device families.)
    #[must_use]
    pub const fn is_domain_specific(self) -> bool {
        self.0 >= DOMAIN_FLOOR
    }
}

// ── Calls ───────────────────────────────────────────────────────────────────

/// The widest immediate a call can carry — [`LaneShape::Quads`]' three value
/// bytes. Narrower shapes use a prefix of [`Call::values`]; the unused tail
/// MUST be zero (enforced by [`FunctionBody::push`] via [`Call::fits`]).
pub const MAX_VALUES_PER_CALL: usize = 3;

/// One call — a function index plus up to three immediate value bytes.
///
/// This is the unit of a function body. How many of [`values`](Self::values)
/// are actually stored is decided by the body's [`LaneShape`] (1, 2, or 3);
/// a `Call` itself always carries the widest form so the same value moves
/// freely between shapes when it fits.
///
/// Computed (non-immediate) arguments do not live here — they come from the
/// stack discipline (see the crate docs): each call consumes its operands from
/// the stack and pushes its result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Call {
    /// Which function — an index into the scope's `<256` codebook.
    pub function: FnIndex,
    /// Immediate value bytes, execution-order. Under [`LaneShape::Pairs`] only
    /// `values[0]` is stored; [`Triples`](LaneShape::Triples) store two;
    /// [`Quads`](LaneShape::Quads) all three.
    pub values: [u8; MAX_VALUES_PER_CALL],
}

impl Call {
    /// The all-zero call — an unwritten slot. Never a real call.
    pub const NOP: Call = Call {
        function: FnIndex::NOP,
        values: [0; MAX_VALUES_PER_CALL],
    };

    /// A call with no immediates (arguments, if any, come from the stack).
    #[must_use]
    pub const fn new(function: FnIndex) -> Self {
        Self {
            function,
            values: [0; MAX_VALUES_PER_CALL],
        }
    }

    /// A call with one immediate — the [`LaneShape::Pairs`] shape:
    /// `WAIT:10`, `REPEAT:4`, `VAR_GET:slot`.
    #[must_use]
    pub const fn with_value(function: FnIndex, v0: u8) -> Self {
        Self {
            function,
            values: [v0, 0, 0],
        }
    }

    /// A call with up to three immediates.
    #[must_use]
    pub const fn with_values(function: FnIndex, values: [u8; MAX_VALUES_PER_CALL]) -> Self {
        Self { function, values }
    }

    /// Is this the unwritten slot? (All bytes zero — the zero-fallback.)
    #[must_use]
    pub const fn is_nop(&self) -> bool {
        self.function.0 == 0 && self.values[0] == 0 && self.values[1] == 0 && self.values[2] == 0
    }

    /// Can `shape` store this call without dropping a value byte?
    ///
    /// True iff every value byte beyond the shape's
    /// [`values_per_call`](LaneShape::values_per_call) is zero. This is the
    /// guard that makes narrowing LOUD: a two-immediate call refuses to enter
    /// a [`Pairs`](LaneShape::Pairs) body instead of silently truncating.
    #[must_use]
    pub const fn fits(&self, shape: LaneShape) -> bool {
        let keep = shape.values_per_call();
        let mut i = keep;
        while i < MAX_VALUES_PER_CALL {
            if self.values[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }
}

// ── Function bodies ─────────────────────────────────────────────────────────

/// Why a call could not enter a [`FunctionBody`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyError {
    /// The body is at its shape's call budget. The remedy is a **split into
    /// two functions**, never a wider row.
    Overflow {
        /// How many calls were offered.
        offered: usize,
        /// The shape's budget ([`LaneShape::calls_per_function`]).
        capacity: usize,
    },
    /// The call carries a nonzero value byte the body's shape cannot store.
    /// The remedy is a wider [`LaneShape`] (a different class), never silent
    /// truncation.
    ValueBeyondShape {
        /// Position of the offending call in the offered sequence.
        index: usize,
        /// The shape that cannot hold it.
        shape: LaneShape,
    },
}

impl core::fmt::Display for BodyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BodyError::Overflow { offered, capacity } => write!(
                f,
                "function body of {offered} calls exceeds the {capacity}-call \
                 budget of its lane shape; split the function"
            ),
            BodyError::ValueBeyondShape { index, shape } => write!(
                f,
                "call {index} carries an immediate beyond what {shape:?} can \
                 store; use a wider lane shape, never truncate"
            ),
        }
    }
}

impl core::error::Error for BodyError {}

/// One function's calls — exactly the payload capacity of a node's value
/// slab, and never more.
///
/// The cap is enforced at every entry point, so a `FunctionBody` that exists
/// is a function that fits in one 512-byte node. The [`LaneShape`] is fixed at
/// construction (it comes from the body's classid) and uniform across the
/// body; there is no partially-valid state and no runtime surprise at write
/// time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionBody {
    /// The gathered 360 payload bytes, execution order. Stored as raw bytes so
    /// [`as_body_bytes`](Self::as_body_bytes) is a plain borrow — no
    /// transmute, no copy, no `unsafe`.
    bytes: [u8; BODY_BYTES],
    /// How many CALLS are written (not bytes).
    len: u16,
    /// How the lanes are carved — from the body's classid, uniform.
    shape: LaneShape,
}

impl Default for FunctionBody {
    fn default() -> Self {
        Self::new(LaneShape::Pairs)
    }
}

impl FunctionBody {
    /// An empty body of the given shape — every byte zero ([`Call::NOP`]).
    #[must_use]
    pub const fn new(shape: LaneShape) -> Self {
        Self {
            bytes: [0u8; BODY_BYTES],
            len: 0,
            shape,
        }
    }

    /// This body's lane carving.
    #[must_use]
    pub const fn shape(&self) -> LaneShape {
        self.shape
    }

    /// The shape's call budget — 180 / 120 / 90.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.shape.calls_per_function()
    }

    /// Build from a slice, rejecting anything past the budget and any call the
    /// shape cannot store losslessly.
    ///
    /// # Errors
    ///
    /// [`BodyError::Overflow`] when `calls.len()` exceeds the shape's budget;
    /// [`BodyError::ValueBeyondShape`] when a call carries an immediate the
    /// shape would truncate.
    pub fn from_calls(shape: LaneShape, calls: &[Call]) -> Result<Self, BodyError> {
        let mut body = Self::new(shape);
        for (i, call) in calls.iter().enumerate() {
            body.push(*call).map_err(|e| match e {
                // Re-index the per-call error to the offered sequence.
                BodyError::ValueBeyondShape { shape, .. } => {
                    BodyError::ValueBeyondShape { index: i, shape }
                }
                BodyError::Overflow { capacity, .. } => BodyError::Overflow {
                    offered: calls.len(),
                    capacity,
                },
            })?;
        }
        Ok(body)
    }

    /// Append one call.
    ///
    /// # Errors
    ///
    /// [`BodyError::Overflow`] when the body is at its shape's budget;
    /// [`BodyError::ValueBeyondShape`] when the call carries an immediate the
    /// shape would truncate. A failed push does not mutate the body.
    pub fn push(&mut self, call: Call) -> Result<(), BodyError> {
        let n = self.len as usize;
        let capacity = self.shape.calls_per_function();
        if n >= capacity {
            return Err(BodyError::Overflow {
                offered: n + 1,
                capacity,
            });
        }
        if !call.fits(self.shape) {
            return Err(BodyError::ValueBeyondShape {
                index: n,
                shape: self.shape,
            });
        }
        let bpc = self.shape.bytes_per_call();
        let at = n * bpc;
        self.bytes[at] = call.function.0;
        let vals = self.shape.values_per_call();
        let mut v = 0usize;
        while v < vals {
            self.bytes[at + 1 + v] = call.values[v];
            v += 1;
        }
        self.len = (n + 1) as u16;
        Ok(())
    }

    /// The call at `index`, or `None` past [`len`](Self::len).
    ///
    /// Value bytes beyond the shape's width read as zero — the call comes back
    /// exactly as [`push`](Self::push) accepted it.
    #[must_use]
    pub fn call(&self, index: usize) -> Option<Call> {
        (index < self.len as usize).then(|| self.call_unchecked(index))
    }

    fn call_unchecked(&self, index: usize) -> Call {
        let bpc = self.shape.bytes_per_call();
        let at = index * bpc;
        let mut values = [0u8; MAX_VALUES_PER_CALL];
        let vals = self.shape.values_per_call();
        values[..vals].copy_from_slice(&self.bytes[at + 1..at + 1 + vals]);
        Call {
            function: FnIndex(self.bytes[at]),
            values,
        }
    }

    /// The calls written so far, in execution order.
    pub fn calls(&self) -> impl Iterator<Item = Call> + '_ {
        (0..self.len as usize).map(|i| self.call_unchecked(i))
    }

    /// How many calls are written.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Is the body empty?
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Remaining call budget before a split is required.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.shape.calls_per_function() - self.len as usize
    }

    /// The body's 360 payload bytes in **execution order**, zero-padded past
    /// the written calls.
    ///
    /// # This is the GATHERED form, NOT the slab layout
    ///
    /// These bytes are **not** contiguous in a node's value slab. The slab is
    /// `CONTENT_SLOTS × 16` = 480 bytes of `classid(4) + payload(12)` facets,
    /// so call `i` starts at slab offset
    /// [`call_slab_offset(shape, i)`](Self::call_slab_offset) — stride 16,
    /// `+4` into each facet, never at `i × bytes_per_call`.
    ///
    /// Copying this array over the front of a slab would overwrite the first
    /// 22½ facets' classids *and* payloads. Use
    /// [`write_into_value_slab`](Self::write_into_value_slab) to place it, or
    /// [`call_in_slab`] to read one call in place without gathering at all.
    #[must_use]
    pub const fn as_body_bytes(&self) -> &[u8; BODY_BYTES] {
        &self.bytes
    }

    /// Byte offset where call `index` STARTS within a node's **480-byte value
    /// slab**, under `shape`.
    ///
    /// `(index / calls_per_lane) × 16 + 4 + (index % calls_per_lane) ×
    /// bytes_per_call` — pick the lane, skip its 4-byte classid, then step by
    /// whole calls within its 12-byte payload. No call straddles a lane
    /// boundary: every shape divides 12 exactly.
    ///
    /// # Panics
    ///
    /// When `index >= shape.calls_per_function()`.
    #[must_use]
    pub const fn call_slab_offset(shape: LaneShape, index: usize) -> usize {
        assert!(
            index < shape.calls_per_function(),
            "call index out of range for this lane shape"
        );
        let cpl = shape.calls_per_lane();
        let lane = index / cpl;
        let within = (index % cpl) * shape.bytes_per_call();
        lane * SLOT_STRIDE + CLASSID_BYTES + within
    }

    /// Scatter the body into a node's value slab, writing **only** the 12-byte
    /// payload lane of each facet.
    ///
    /// The 4-byte classid of every facet is left untouched — this writes the
    /// calls, never the addressing. (The scatter is shape-independent: gathered
    /// bytes map lane-linearly, `bytes[l×12..][..12] → slab[l×16+4..][..12]`.)
    pub fn write_into_value_slab(&self, slab: &mut [u8; VALUE_SLAB_LEN]) {
        for lane in 0..CONTENT_SLOTS {
            let src = lane * PAYLOAD_BYTES_PER_SLOT;
            let dst = lane * SLOT_STRIDE + CLASSID_BYTES;
            slab[dst..dst + PAYLOAD_BYTES_PER_SLOT]
                .copy_from_slice(&self.bytes[src..src + PAYLOAD_BYTES_PER_SLOT]);
        }
    }

    /// Gather a body back out of a node's value slab — the inverse of
    /// [`write_into_value_slab`](Self::write_into_value_slab).
    ///
    /// The shape is a parameter because it is NOT in the slab — it comes from
    /// the body's classid (slot purity: the payload is dumb bytes; the class
    /// selects the reading). `len` is recovered as the position after the last
    /// non-[`NOP`](Call::NOP) call, since the wire form carries no length
    /// field — zero padding IS the length signal, which also means an interior
    /// all-zero call is indistinguishable from padding and is not a valid
    /// program element.
    #[must_use]
    pub fn read_from_value_slab(shape: LaneShape, slab: &[u8; VALUE_SLAB_LEN]) -> Self {
        let mut body = Self::new(shape);
        for lane in 0..CONTENT_SLOTS {
            let src = lane * SLOT_STRIDE + CLASSID_BYTES;
            let dst = lane * PAYLOAD_BYTES_PER_SLOT;
            body.bytes[dst..dst + PAYLOAD_BYTES_PER_SLOT]
                .copy_from_slice(&slab[src..src + PAYLOAD_BYTES_PER_SLOT]);
        }
        let bpc = shape.bytes_per_call();
        let last_live = (0..shape.calls_per_function())
            .rev()
            .find(|&i| body.bytes[i * bpc..(i + 1) * bpc].iter().any(|&b| b != 0));
        body.len = last_live.map_or(0, |i| i as u16 + 1);
        body
    }
}

/// Read one call **in place** from a node's value slab — no gather, no copy,
/// `bytes_per_call` indexed reads.
///
/// This is the zero-copy read the substrate wants: a consumer that needs call
/// `i` never materialises the rest of the body.
///
/// # Panics
///
/// When `index >= shape.calls_per_function()`.
#[must_use]
pub fn call_in_slab(slab: &[u8; VALUE_SLAB_LEN], shape: LaneShape, index: usize) -> Call {
    let at = FunctionBody::call_slab_offset(shape, index);
    let mut values = [0u8; MAX_VALUES_PER_CALL];
    let vals = shape.values_per_call();
    values[..vals].copy_from_slice(&slab[at + 1..at + 1 + vals]);
    Call {
        function: FnIndex(slab[at]),
        values,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_byte_budget_is_derived_from_the_node_layout() {
        // 512-byte node = key(16) | reserved(16) | value(480); value = 30 lanes
        // of classid(4)+12. If any of those change, this must fail rather than
        // silently re-budget.
        assert_eq!(CONTENT_SLOTS, 480 / 16);
        assert_eq!(PAYLOAD_BYTES_PER_SLOT, 16 - 4);
        assert_eq!(BODY_BYTES, 360);
        // Per-shape call budgets: same 360 bytes, different carving.
        assert_eq!(LaneShape::Pairs.calls_per_function(), 180);
        assert_eq!(LaneShape::Triples.calls_per_function(), 120);
        assert_eq!(LaneShape::Quads.calls_per_function(), 90);
        for shape in LaneShape::ALL {
            // Every shape divides a lane exactly and spends exactly 360 bytes.
            assert_eq!(shape.calls_per_lane() * shape.bytes_per_call(), 12);
            assert_eq!(
                shape.calls_per_function() * shape.bytes_per_call(),
                BODY_BYTES
            );
            assert_eq!(shape.values_per_call(), shape.bytes_per_call() - 1);
        }
    }

    #[test]
    fn a_body_at_capacity_is_accepted_and_one_past_is_rejected_in_every_shape() {
        // Two-sided per shape: the cap must admit exactly the budget and refuse
        // one more — a cap that only rejects, or only accepts, carries no
        // information.
        for shape in LaneShape::ALL {
            let cap = shape.calls_per_function();
            let exact = vec![Call::new(FnIndex::ADD); cap];
            let body = FunctionBody::from_calls(shape, &exact)
                .unwrap_or_else(|e| panic!("{cap} calls must fit {shape:?}: {e}"));
            assert_eq!(body.len(), cap);
            assert_eq!(body.remaining(), 0);
            assert_eq!(body.capacity(), cap);

            let over = vec![Call::new(FnIndex::ADD); cap + 1];
            match FunctionBody::from_calls(shape, &over) {
                Err(BodyError::Overflow { offered, capacity }) => {
                    assert_eq!(offered, cap + 1);
                    assert_eq!(capacity, cap);
                }
                other => panic!(
                    "{} calls in {shape:?} must overflow, got {other:?}",
                    cap + 1
                ),
            }
        }
    }

    #[test]
    fn push_enforces_the_same_budget_as_from_calls() {
        // The cap must not be reachable by a back door: filling one call at a
        // time has to stop at exactly the same place.
        let mut body = FunctionBody::new(LaneShape::Pairs);
        for _ in 0..LaneShape::Pairs.calls_per_function() {
            body.push(Call::new(FnIndex::MUL)).expect("within budget");
        }
        assert_eq!(body.len(), 180);
        let err = body
            .push(Call::new(FnIndex::MUL))
            .expect_err("the 181st must fail");
        assert!(matches!(
            err,
            BodyError::Overflow {
                offered: 181,
                capacity: 180
            }
        ));
        // and the failed push must not have mutated the body
        assert_eq!(body.len(), 180);
    }

    #[test]
    fn a_call_too_wide_for_the_shape_is_rejected_not_truncated() {
        // The narrowing guard, two-sided: the SAME call must be refused by the
        // shape that would drop a value byte and accepted by the shape that
        // holds it. Silent truncation is the defect this exists to prevent.
        let two_immediates = Call::with_values(FnIndex::CONSTRAIN, [10, 20, 0]);
        let three_immediates = Call::with_values(FnIndex::CONSTRAIN, [10, 20, 30]);

        let mut pairs = FunctionBody::new(LaneShape::Pairs);
        match pairs.push(two_immediates) {
            Err(BodyError::ValueBeyondShape { index: 0, shape }) => {
                assert_eq!(shape, LaneShape::Pairs);
            }
            other => panic!("two immediates must not fit Pairs, got {other:?}"),
        }
        assert!(pairs.is_empty(), "a rejected push must not mutate the body");

        let mut triples = FunctionBody::new(LaneShape::Triples);
        triples
            .push(two_immediates)
            .expect("two immediates fit Triples");
        assert_eq!(triples.call(0), Some(two_immediates));
        match triples.push(three_immediates) {
            Err(BodyError::ValueBeyondShape { index: 1, shape }) => {
                assert_eq!(shape, LaneShape::Triples);
            }
            other => panic!("three immediates must not fit Triples, got {other:?}"),
        }

        let mut quads = FunctionBody::new(LaneShape::Quads);
        quads
            .push(three_immediates)
            .expect("three immediates fit Quads");
        assert_eq!(quads.call(0), Some(three_immediates));

        // from_calls applies the same guard and reports the offending index.
        let seq = [Call::new(FnIndex::ADD), two_immediates];
        match FunctionBody::from_calls(LaneShape::Pairs, &seq) {
            Err(BodyError::ValueBeyondShape { index: 1, .. }) => {}
            other => panic!("from_calls must index the offending call, got {other:?}"),
        }
    }

    #[test]
    fn body_bytes_are_execution_order_zero_padded() {
        // `5 + 3` under the stack discipline: (NUMBER:5) (NUMBER:3) (ADD:_).
        let body = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, 5),
                Call::with_value(FnIndex::NUMBER, 3),
                Call::new(FnIndex::ADD),
            ],
        )
        .unwrap();
        let bytes = body.as_body_bytes();
        assert_eq!(bytes.len(), BODY_BYTES);
        assert_eq!(&bytes[..6], &[0x46, 5, 0x46, 3, 0x40, 0]);
        // Everything past the written calls is the zero-fallback, so a
        // partially-filled body needs no length field on the wire.
        assert!(bytes[6..].iter().all(|&b| b == 0));
        // And the calls read back exactly as pushed.
        let calls: Vec<Call> = body.calls().collect();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0], Call::with_value(FnIndex::NUMBER, 5));
        assert_eq!(calls[2], Call::new(FnIndex::ADD));
    }

    #[test]
    fn the_slab_interleaves_classids_so_calls_are_not_contiguous() {
        // The defect this guards: the gathered array is NOT the slab layout.
        // Call i starts at (i/cpl)*16 + 4 + (i%cpl)*bpc — never at i*bpc.
        assert_eq!(FunctionBody::call_slab_offset(LaneShape::Pairs, 0), 4);
        assert_eq!(FunctionBody::call_slab_offset(LaneShape::Pairs, 5), 14); // last pair, lane 0
        assert_eq!(FunctionBody::call_slab_offset(LaneShape::Pairs, 6), 20); // lane 1 skips classid
        assert_eq!(FunctionBody::call_slab_offset(LaneShape::Pairs, 179), 478);
        assert_eq!(FunctionBody::call_slab_offset(LaneShape::Quads, 89), 476);

        for shape in LaneShape::ALL {
            let bpc = shape.bytes_per_call();
            for i in 0..shape.calls_per_function() {
                let off = FunctionBody::call_slab_offset(shape, i);
                // Anti-vacuity: a genuine permutation, never identity — the
                // gathered offset i*bpc must never equal the slab offset.
                assert_ne!(
                    off,
                    i * bpc,
                    "{shape:?} call {i} sits at its own gathered offset"
                );
                // The whole call lands inside one payload lane: past the
                // classid, and not running off the lane's end (no straddle).
                assert!(off % SLOT_STRIDE >= CLASSID_BYTES);
                assert!(off % SLOT_STRIDE + bpc <= SLOT_STRIDE);
                assert!(off + bpc <= VALUE_SLAB_LEN);
            }
        }
    }

    #[test]
    fn scatter_gather_round_trips_and_never_touches_a_classid() {
        for shape in LaneShape::ALL {
            // Distinct function bytes, shape-widest immediates.
            let mut vals = [0u8; MAX_VALUES_PER_CALL];
            vals[..shape.values_per_call()].copy_from_slice(&[7, 9, 11][..shape.values_per_call()]);
            let calls: Vec<Call> = (1..=40u8)
                .map(|i| Call::with_values(FnIndex(i), vals))
                .collect();
            let body = FunctionBody::from_calls(shape, &calls).unwrap();

            // Pre-stamp every lane's classid with a sentinel; the write must
            // leave all 120 of those bytes untouched — it writes calls, not
            // addressing.
            let mut slab = [0u8; VALUE_SLAB_LEN];
            for lane in 0..CONTENT_SLOTS {
                for b in 0..CLASSID_BYTES {
                    slab[lane * SLOT_STRIDE + b] = 0xC1;
                }
            }
            body.write_into_value_slab(&mut slab);

            for lane in 0..CONTENT_SLOTS {
                for b in 0..CLASSID_BYTES {
                    assert_eq!(
                        slab[lane * SLOT_STRIDE + b],
                        0xC1,
                        "{shape:?}: lane {lane} classid byte {b} was overwritten"
                    );
                }
            }

            // In-place read agrees with the pushed calls, call by call.
            for (i, call) in calls.iter().enumerate() {
                assert_eq!(
                    call_in_slab(&slab, shape, i),
                    *call,
                    "{shape:?}: call {i} misplaced in the slab"
                );
            }

            // And the gather is the exact inverse — shape supplied by the
            // caller (it lives in the classid, never in the slab).
            let back = FunctionBody::read_from_value_slab(shape, &slab);
            assert_eq!(back.len(), calls.len(), "{shape:?}: len not recovered");
            assert_eq!(back.as_body_bytes(), body.as_body_bytes());
            assert_eq!(back.shape(), shape);
        }
    }

    #[test]
    fn a_naive_contiguous_copy_is_detectably_wrong() {
        // This is the bug an earlier doc comment would have caused: treating
        // the gathered 360 bytes as the front of the slab. It must be
        // observably different from the correct scatter, or the distinction
        // this API draws carries no information.
        let calls: Vec<Call> = (1..=30u8)
            .map(|i| Call::with_value(FnIndex(i), i))
            .collect();
        let body = FunctionBody::from_calls(LaneShape::Pairs, &calls).unwrap();

        let mut correct = [0u8; VALUE_SLAB_LEN];
        body.write_into_value_slab(&mut correct);

        let mut naive = [0u8; VALUE_SLAB_LEN];
        naive[..BODY_BYTES].copy_from_slice(body.as_body_bytes());

        assert_ne!(
            correct, naive,
            "scatter and contiguous copy must not coincide"
        );
        // Concretely: the naive copy puts call 0's function byte on lane 0's
        // FIRST classid byte.
        assert_eq!(naive[0], calls[0].function.0);
        assert_eq!(correct[0], 0, "lane 0's classid must stay untouched");
        assert_eq!(
            correct[FunctionBody::call_slab_offset(LaneShape::Pairs, 0)],
            calls[0].function.0
        );
    }

    #[test]
    fn in_memory_body_is_larger_than_the_wire_form() {
        // [u8; 360] + u16 len + LaneShape(1 B) → 363, padded to 364 by the u16's
        // alignment. Neither len nor shape is written to the slab — zero padding
        // is the length signal, the classid is the shape signal — so the wire
        // form is exactly 360 and this gap must stay visible rather than
        // surprising a consumer that assumed size_of == payload size.
        assert_eq!(core::mem::size_of::<FunctionBody>(), 364);
        assert_eq!(BODY_BYTES, 360);
        assert_eq!(VALUE_SLAB_LEN, 480);
        assert_eq!(VALUE_SLAB_LEN - BODY_BYTES, CONTENT_SLOTS * CLASSID_BYTES);
    }

    #[test]
    fn len_recovery_finds_the_last_live_call_per_shape() {
        // A body whose LAST call is function-only (all value bytes zero) must
        // still recover its full length — the liveness test is "any nonzero
        // byte in the call", not "nonzero value".
        for shape in LaneShape::ALL {
            let calls = [
                Call::with_value(FnIndex::NUMBER, 9),
                Call::new(FnIndex::ADD), // function byte only
            ];
            let body = FunctionBody::from_calls(shape, &calls).unwrap();
            let mut slab = [0u8; VALUE_SLAB_LEN];
            body.write_into_value_slab(&mut slab);
            let back = FunctionBody::read_from_value_slab(shape, &slab);
            assert_eq!(back.len(), 2, "{shape:?}: trailing bare call lost");
            assert_eq!(back.call(1), Some(Call::new(FnIndex::ADD)));
        }
        // And an empty body recovers as empty (the silence half).
        let empty = [0u8; VALUE_SLAB_LEN];
        for shape in LaneShape::ALL {
            assert_eq!(
                FunctionBody::read_from_value_slab(shape, &empty).len(),
                0,
                "{shape:?}: empty slab must read as empty"
            );
        }
    }

    #[test]
    fn shared_core_and_domain_range_partition_the_codebook() {
        // Can-fire AND can-stay-silent on the same predicate: a classifier that
        // answers the same way for everything is worthless.
        assert!(FnIndex::LT.is_shared_core());
        assert!(!FnIndex::LT.is_domain_specific());

        let domain = FnIndex(DOMAIN_FLOOR);
        assert!(domain.is_domain_specific());
        assert!(!domain.is_shared_core());

        // NOP is not an operation at all — neither bucket claims it.
        assert!(!FnIndex::NOP.is_shared_core());
        assert!(!FnIndex::NOP.is_domain_specific());
    }
}
