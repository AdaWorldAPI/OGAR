//! The vocabulary seam — how a sibling codebook plugs into the shared surface.
//!
//! # The sharing discipline
//!
//! One call-ABI, sibling vocabularies selected by classid, *"not much
//! different than blockly, just different vocabulary"* (the operator frame).
//! The discipline that keeps that from decaying into N dialects:
//!
//! - Bytes **below [`DOMAIN_FLOOR`]** are the **shared computational core**.
//!   Their stack arities and body-reference counts live ONCE, here, in
//!   [`shared_core`] — so `IF` cannot quietly mean two things in two domains,
//!   and no sibling can drift on `ADD`'s arity.
//! - Bytes **at/above the floor** belong to the vocabulary. A [`Vocabulary`]
//!   implementation answers for exactly that range via its `domain_*` hooks;
//!   the composed methods route each byte to the right table.
//! - A shared-core byte the core does not cover (today: the five genuinely
//!   variadic entries listed on [`shared_core::stack_arity`]) is **refused
//!   everywhere** — a vocabulary does not get to guess for it. Coverage
//!   grows in the core, once, for everyone.
//!
//! [`conformance::check`] is the mechanical enforcement: run it in every
//! vocabulary crate's tests. It catches a composed-method override that
//! drifts from the core (the JVM-verifier / Wasm-validator posture: validate
//! before trusting, refuse loudly).
//!
//! # The two-quantity split (why there are TWO tables)
//!
//! A call has two independent numbers, and conflating them is the bug this
//! module exists to prevent:
//!
//! | | what it is | where it lives |
//! |---|---|---|
//! | [`stack_arity`](Vocabulary::stack_arity) | operands evaluated before the call | the stack |
//! | [`body_refs`](Vocabulary::body_refs) | function indices this call branches to | the call's **value bytes** |
//!
//! `repeat 10 [ … ]` consumes **one** stack operand (the count) and
//! **references** a body. The body is not an operand: it is not on the stack,
//! it was not evaluated before the call, and popping it would silently
//! reattribute whatever *was* on the stack. `forever` proves the quantities
//! are independent: zero operands, one body. A single conflated number cannot
//! express it.

use crate::{DOMAIN_FLOOR, FnIndex, LaneShape};

/// The shared computational core's tables — authoritative for every byte
/// below [`DOMAIN_FLOOR`], in every vocabulary.
pub mod shared_core {
    use super::{FnIndex, LaneShape};

    /// How many operands a shared-core call pops from the stack.
    ///
    /// `None` means the core does not cover this function — refused rather
    /// than guessed, because a wrong arity does not produce a slightly-wrong
    /// result: it desynchronizes the stack and reattributes every later
    /// operand.
    ///
    /// # What is still uncovered, and why it is not an oversight
    ///
    /// Exactly five entries answer `None`, and every one is **genuinely
    /// variadic** — it has no single correct arity to record, so covering it
    /// would mean choosing a number that is wrong for some legitimate call:
    ///
    /// | function | why it has no fixed arity |
    /// |---|---|
    /// | `RETURN` | a void return evaluates nothing; a value return evaluates one |
    /// | `NUMBER_PROPERTY` | `even`/`odd`/`prime`/… take the number alone, `divisible by` takes a divisor too |
    /// | `PROMPT` | `text_prompt` carries its message as a field, `text_prompt_ext` as an operand |
    /// | `LIST_WITH` | a list literal holds as many items as its mutator says |
    /// | `PROC_CALL` | a call passes as many arguments as its procedure declares |
    ///
    /// These want an arity that is a property of the CALL rather than of the
    /// function — a per-call count carried in the call's own bytes. That is a
    /// deliberate design question, not a table row, so they stay refused
    /// until it is answered. Everything else the palette names is covered.
    ///
    /// For control flow this counts **only** the evaluated operands. A loop
    /// body is not among them; see [`body_refs`].
    #[must_use]
    pub fn stack_arity(f: FnIndex) -> Option<u8> {
        Some(match f {
            // ── control: bodies only — nothing evaluated first.
            FnIndex::FOREVER => 0,
            // One condition or count, then a body.
            FnIndex::IF
            | FnIndex::IF_ELSE
            | FnIndex::REPEAT
            | FnIndex::WHILE
            | FnIndex::REPEAT_UNTIL
            | FnIndex::FOR_EACH => 1,
            // from, to, by — then a body.
            FnIndex::FOR_RANGE => 3,
            // ── epistemic mask algebra (0x86..0x8B). TERNLOG pops three
            // masks; its truth table rides as the call's value byte, never
            // as an operand. The two list-consuming ops take the LIST as
            // one operand (the arity of the call, not of the children —
            // which is how the variadic-refusal rule stays satisfied).
            FnIndex::TERNLOG => 3,
            FnIndex::BELNAP_JOIN
            | FnIndex::INFO_GAIN
            | FnIndex::SIGMA_TENSION
            | FnIndex::STANCE_ENTROPY => 2,
            FnIndex::ACCUMULATE => 1,
            // Leave the enclosing loop / iteration. No operand, no body.
            FnIndex::BREAK | FnIndex::CONTINUE => 0,
            // ── leaves — they push, they do not consume.
            FnIndex::NUMBER
            | FnIndex::TEXT
            | FnIndex::TRUE
            | FnIndex::FALSE
            | FnIndex::NULL
            | FnIndex::CONSTANT
            | FnIndex::VAR_GET => 0,
            // ── unary.
            FnIndex::NOT
            | FnIndex::NEG
            | FnIndex::ABS
            | FnIndex::SQRT
            | FnIndex::LN
            | FnIndex::LOG10
            | FnIndex::EXP_E
            | FnIndex::EXP_10
            | FnIndex::SIN
            | FnIndex::COS
            | FnIndex::TAN
            | FnIndex::ASIN
            | FnIndex::ACOS
            | FnIndex::ATAN
            | FnIndex::ROUND
            | FnIndex::FLOOR
            | FnIndex::CEIL
            | FnIndex::LENGTH => 1,
            // ── binary.
            FnIndex::ADD
            | FnIndex::SUB
            | FnIndex::MUL
            | FnIndex::DIV
            | FnIndex::POW
            | FnIndex::MOD
            | FnIndex::EQ
            | FnIndex::NEQ
            | FnIndex::LT
            | FnIndex::LTE
            | FnIndex::GT
            | FnIndex::GTE
            | FnIndex::AND
            | FnIndex::OR
            | FnIndex::JOIN => 2,

            // ── control: the suspend/stop family.
            // A duration or a condition, evaluated first; `STOP`'s target
            // (this script / all / others) is a selector immediate, not an
            // operand — both frontends agree (`control_wait` declares one
            // input, `control_stop` declares none).
            FnIndex::WAIT | FnIndex::WAIT_UNTIL => 1,
            FnIndex::STOP => 0,

            // ── expressions with a fixed shape.
            // Corroborated by BOTH frontends where both have the block, and
            // by the surviving one where only one does: `logic_ternary` and
            // `math_constrain` declare three inputs, `math_atan2` and
            // `math_random_int`/`operator_random` two, `math_random_float`
            // none, `text_isEmpty` one.
            FnIndex::TERNARY | FnIndex::CONSTRAIN => 3,
            FnIndex::ATAN2 | FnIndex::RANDOM_INT => 2,
            FnIndex::RANDOM_FLOAT => 0,

            // ── text.
            // The selector-style ops take the subject only; the dropdown
            // (which case, which end to trim) is a value byte, never an
            // operand — the same split `math_single[ROOT]` already uses.
            FnIndex::IS_EMPTY
            | FnIndex::CHANGE_CASE
            | FnIndex::TRIM
            | FnIndex::REVERSE
            | FnIndex::PRINT => 1,
            // `APPEND` follows the `VAR_SET` shape below: the variable is an
            // immediate, so only the appended value is evaluated
            // (`text_append` declares one input).
            FnIndex::APPEND => 1,
            FnIndex::CHAR_AT | FnIndex::INDEX_OF | FnIndex::CONTAINS | FnIndex::COUNT => 2,
            FnIndex::SUBSTRING | FnIndex::REPLACE => 3,

            // ── variables.
            // `VAR_GET` is a leaf (above): the variable id rides as an
            // immediate value byte and nothing is popped. Its writing twins
            // therefore evaluate only the incoming value — which is exactly
            // what both frontends declare (`data_setvariableto` and
            // `variables_set` one input each, the variable itself a field).
            FnIndex::VAR_SET | FnIndex::VAR_CHANGE => 1,

            // ── lists.
            //
            // The list is an ORDINARY OPERAND here, not an immediate, and
            // that is what lets one table serve two frontends that model
            // lists differently. Blockly passes a list *expression*
            // (`lists_length` takes a LIST input); Scratch names a global
            // list in a *field* (`data_lengthoflist` declares zero inputs).
            // On a stack machine the two converge: Scratch's field lowers to
            // an explicit push of the list reference — the same shape
            // `VAR_GET` already has — and the op then pops it. So the arity
            // below counts the list, and a Scratch lowering supplies it
            // rather than needing a second opcode family.
            FnIndex::LIST_EMPTY => 0,
            FnIndex::LIST_LENGTH
            | FnIndex::LIST_IS_EMPTY
            | FnIndex::LIST_DELETE_ALL
            | FnIndex::LIST_SORT
            | FnIndex::ON_LIST => 1,
            FnIndex::LIST_REPEAT
            | FnIndex::LIST_GET
            | FnIndex::LIST_ADD
            | FnIndex::LIST_DELETE
            | FnIndex::LIST_INDEX_OF
            | FnIndex::LIST_CONTAINS
            | FnIndex::LIST_SPLIT => 2,
            FnIndex::LIST_SET | FnIndex::LIST_INSERT | FnIndex::LIST_SUBLIST => 3,

            // ── procedures.
            // `PROC_DEF` evaluates nothing — its body is a REFERENCE, counted
            // by `body_refs`, which is the two-quantity split this module
            // exists to keep apart. `PROC_ARG` is a leaf like `VAR_GET`.
            FnIndex::PROC_DEF | FnIndex::PROC_ARG => 0,

            // Deliberately still uncovered — see the module note on the five
            // variadic entries. Refused rather than guessed.
            _ => return None,
        })
    }

    /// How many of a shared-core call's value bytes are **function indices**
    /// it branches to.
    ///
    /// Zero for every expression call. `IF_ELSE` is the only two, which is
    /// why it needs a shape wider than [`LaneShape::Pairs`] — see
    /// [`min_shape`].
    #[must_use]
    pub fn body_refs(f: FnIndex) -> u8 {
        match f {
            FnIndex::IF
            | FnIndex::REPEAT
            | FnIndex::WHILE
            | FnIndex::REPEAT_UNTIL
            | FnIndex::FOREVER
            | FnIndex::FOR_EACH
            | FnIndex::FOR_RANGE
            // A function definition references the body it defines. It
            // evaluates nothing (`stack_arity` 0), which is the two-quantity
            // split in its purest form — and both frontends declare exactly
            // this shape: `procedures_definition` has one statement input and
            // zero value inputs.
            | FnIndex::PROC_DEF => 1,
            FnIndex::IF_ELSE => 2,
            _ => 0,
        }
    }

    /// Whether this function is control flow at all — i.e. it references a
    /// body.
    ///
    /// `BREAK` and `CONTINUE` are control flow in the language sense but
    /// reference nothing, so they are deliberately **not** included: this
    /// predicate answers "does lowering this call require emitting another
    /// function?", which is the only question a cast asks.
    #[must_use]
    pub fn branches(f: FnIndex) -> bool {
        body_refs(f) > 0
    }

    /// The narrowest [`LaneShape`] that can hold this call's value bytes.
    ///
    /// `IF_ELSE` carries two body references, so it cannot be stored under
    /// `Pairs` — a one-byte immediate would truncate the else arm into
    /// nothing, and the program would run its then-branch and silently skip
    /// the else. A cast refuses rather than narrowing; this is what it
    /// consults. (No shared-core function carries three references today;
    /// the `Quads` arm exists so the mapping is total by construction
    /// rather than by the current table's accident.)
    #[must_use]
    pub fn min_shape(f: FnIndex) -> LaneShape {
        match body_refs(f) {
            0 | 1 => LaneShape::Pairs,
            2 => LaneShape::Triples,
            _ => LaneShape::Quads,
        }
    }

    /// The canonical name of a shared-core slot — the prompt-legend /
    /// oracle-schema column (wishlist W-2).
    ///
    /// Names are NOT coverage: refused-but-real palette entries (`WAIT`,
    /// `STOP`, …) are named so a legend can say "exists, refused" instead of
    /// omitting them into apparent nonexistence. [`FnIndex::NOP`] is not an
    /// operation and has no name. Defined once here so no consumer
    /// hand-rolls a name map that can drift — a legend becomes a
    /// serialization of the validated table itself, at the membrane where
    /// serialization is legal.
    #[must_use]
    pub fn name(f: FnIndex) -> Option<&'static str> {
        Some(match f {
            FnIndex::IF => "IF",
            FnIndex::IF_ELSE => "IF_ELSE",
            FnIndex::REPEAT => "REPEAT",
            FnIndex::REPEAT_UNTIL => "REPEAT_UNTIL",
            FnIndex::WHILE => "WHILE",
            FnIndex::FOREVER => "FOREVER",
            FnIndex::FOR_EACH => "FOR_EACH",
            FnIndex::FOR_RANGE => "FOR_RANGE",
            FnIndex::WAIT => "WAIT",
            FnIndex::WAIT_UNTIL => "WAIT_UNTIL",
            FnIndex::STOP => "STOP",
            FnIndex::BREAK => "BREAK",
            FnIndex::CONTINUE => "CONTINUE",
            FnIndex::RETURN => "RETURN",
            FnIndex::AND => "AND",
            FnIndex::OR => "OR",
            FnIndex::NOT => "NOT",
            FnIndex::TRUE => "TRUE",
            FnIndex::FALSE => "FALSE",
            FnIndex::NULL => "NULL",
            FnIndex::TERNARY => "TERNARY",
            FnIndex::EQ => "EQ",
            FnIndex::NEQ => "NEQ",
            FnIndex::LT => "LT",
            FnIndex::LTE => "LTE",
            FnIndex::GT => "GT",
            FnIndex::GTE => "GTE",
            FnIndex::ADD => "ADD",
            FnIndex::SUB => "SUB",
            FnIndex::MUL => "MUL",
            FnIndex::DIV => "DIV",
            FnIndex::POW => "POW",
            FnIndex::MOD => "MOD",
            FnIndex::NUMBER => "NUMBER",
            FnIndex::ABS => "ABS",
            FnIndex::NEG => "NEG",
            FnIndex::ROUND => "ROUND",
            FnIndex::FLOOR => "FLOOR",
            FnIndex::CEIL => "CEIL",
            FnIndex::SQRT => "SQRT",
            FnIndex::LN => "LN",
            FnIndex::LOG10 => "LOG10",
            FnIndex::EXP_E => "EXP_E",
            FnIndex::EXP_10 => "EXP_10",
            FnIndex::SIN => "SIN",
            FnIndex::COS => "COS",
            FnIndex::TAN => "TAN",
            FnIndex::ASIN => "ASIN",
            FnIndex::ACOS => "ACOS",
            FnIndex::ATAN => "ATAN",
            FnIndex::ATAN2 => "ATAN2",
            FnIndex::TERNLOG => "TERNLOG",
            FnIndex::BELNAP_JOIN => "BELNAP_JOIN",
            FnIndex::INFO_GAIN => "INFO_GAIN",
            FnIndex::SIGMA_TENSION => "SIGMA_TENSION",
            FnIndex::ACCUMULATE => "ACCUMULATE",
            FnIndex::STANCE_ENTROPY => "STANCE_ENTROPY",
            FnIndex::RANDOM_INT => "RANDOM_INT",
            FnIndex::RANDOM_FLOAT => "RANDOM_FLOAT",
            FnIndex::CONSTRAIN => "CONSTRAIN",
            FnIndex::NUMBER_PROPERTY => "NUMBER_PROPERTY",
            FnIndex::CONSTANT => "CONSTANT",
            FnIndex::ON_LIST => "ON_LIST",
            FnIndex::TEXT => "TEXT",
            FnIndex::JOIN => "JOIN",
            FnIndex::LENGTH => "LENGTH",
            FnIndex::CHAR_AT => "CHAR_AT",
            FnIndex::INDEX_OF => "INDEX_OF",
            FnIndex::IS_EMPTY => "IS_EMPTY",
            FnIndex::SUBSTRING => "SUBSTRING",
            FnIndex::CHANGE_CASE => "CHANGE_CASE",
            FnIndex::TRIM => "TRIM",
            FnIndex::CONTAINS => "CONTAINS",
            FnIndex::APPEND => "APPEND",
            FnIndex::PRINT => "PRINT",
            FnIndex::PROMPT => "PROMPT",
            FnIndex::COUNT => "COUNT",
            FnIndex::REPLACE => "REPLACE",
            FnIndex::REVERSE => "REVERSE",
            FnIndex::LIST_EMPTY => "LIST_EMPTY",
            FnIndex::LIST_WITH => "LIST_WITH",
            FnIndex::LIST_REPEAT => "LIST_REPEAT",
            FnIndex::LIST_LENGTH => "LIST_LENGTH",
            FnIndex::LIST_IS_EMPTY => "LIST_IS_EMPTY",
            FnIndex::LIST_INDEX_OF => "LIST_INDEX_OF",
            FnIndex::LIST_GET => "LIST_GET",
            FnIndex::LIST_SET => "LIST_SET",
            FnIndex::LIST_INSERT => "LIST_INSERT",
            FnIndex::LIST_ADD => "LIST_ADD",
            FnIndex::LIST_DELETE => "LIST_DELETE",
            FnIndex::LIST_DELETE_ALL => "LIST_DELETE_ALL",
            FnIndex::LIST_SUBLIST => "LIST_SUBLIST",
            FnIndex::LIST_SPLIT => "LIST_SPLIT",
            FnIndex::LIST_SORT => "LIST_SORT",
            FnIndex::LIST_CONTAINS => "LIST_CONTAINS",
            FnIndex::VAR_GET => "VAR_GET",
            FnIndex::VAR_SET => "VAR_SET",
            FnIndex::VAR_CHANGE => "VAR_CHANGE",
            FnIndex::PROC_DEF => "PROC_DEF",
            FnIndex::PROC_CALL => "PROC_CALL",
            FnIndex::PROC_ARG => "PROC_ARG",
            _ => return None,
        })
    }

    /// Whether a covered shared-core call **pushes a result** onto the stack.
    ///
    /// `Some(true)` for every covered expression (leaves, unary, binary);
    /// `Some(false)` for the covered control calls and `BREAK`/`CONTINUE`,
    /// which act and push nothing; `None` for anything the core does not
    /// cover — the same refuse-don't-guess rule as [`stack_arity`].
    ///
    /// This is the column statement segmentation
    /// ([`crate::statements::statement_bounds`]) runs on: a statement ends
    /// where the stack returns to empty after a non-pushing call. A wrong
    /// guess here would mis-group statements silently, which is why the
    /// uncovered answer is `None` and never a default.
    #[must_use]
    pub fn pushes_result(f: FnIndex) -> Option<bool> {
        stack_arity(f)?;
        Some(!matches!(
            f,
            FnIndex::IF
                | FnIndex::IF_ELSE
                | FnIndex::REPEAT
                | FnIndex::WHILE
                | FnIndex::REPEAT_UNTIL
                | FnIndex::FOREVER
                | FnIndex::FOR_EACH
                | FnIndex::FOR_RANGE
                | FnIndex::BREAK
                | FnIndex::CONTINUE
        ))
    }
}

/// A basin-local codebook a call's value bytes are drawn from.
///
/// Most calls' value bytes are literal numbers ([`FnIndex::NUMBER`]) or
/// function indices ([`Vocabulary::body_refs`]). A relation call is neither:
/// its operands are IDs into a **basin-local target codebook** — a codebook
/// scoped to the classid prefix the call's own body lives under, not a
/// vocabulary-wide table. `part_of(subject, object)` in one basin and
/// `part_of(subject, object)` in a sibling basin can legally resolve their
/// object byte against two different codebooks; the call itself is agnostic
/// to which.
///
/// This is advisory metadata, not a shape constraint: declaring a codebook
/// does not change `stack_arity`/`body_refs`/`min_shape`, it tells a
/// renderer, oracle-schema generator, or fuzzer WHICH table to resolve a
/// call's operand bytes against instead of guessing "probably a literal."
/// Default `None` everywhere (shared core AND undeclared domain calls) —
/// declaring it is opt-in, paid only by a vocabulary that has basin-scoped
/// operands (W-RO-3, `ogar-ro`'s relation predicates being the first user).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueCodebook {
    /// The codebook's id, meaningful only within the basin the call's own
    /// body lives under — never a global registry key. Interpreted by
    /// whatever consumer owns that basin.
    pub id: u8,
    /// The codebook's canonical name, for legends and oracle schemas —
    /// mirrors [`FnSpec::name`]'s role for the call itself.
    pub name: &'static str,
}

/// A sibling codebook over the shared surface.
///
/// Implementations answer for the **domain range** (bytes at/above
/// [`DOMAIN_FLOOR`]) through the `domain_*` hooks; the composed methods
/// route shared-core bytes to [`shared_core`]'s tables unconditionally.
///
/// **Do not override the composed methods.** Rust cannot seal a default
/// method, so the guarantee is enforced socially AND mechanically: every
/// vocabulary crate runs [`conformance::check`] in its tests, and an
/// override that drifts a shared-core answer fails it.
pub trait Vocabulary {
    /// Stack arity for a domain-range function. `None` = not covered
    /// (refused). Never consulted for shared-core bytes.
    fn domain_stack_arity(&self, f: FnIndex) -> Option<u8>;

    /// Body-reference count for a domain-range function. Never consulted for
    /// shared-core bytes.
    fn domain_body_refs(&self, f: FnIndex) -> u8;

    /// How many operands `f` pops — shared core first, domain hooks above
    /// the floor.
    fn stack_arity(&self, f: FnIndex) -> Option<u8> {
        if f.0 < DOMAIN_FLOOR {
            shared_core::stack_arity(f)
        } else {
            self.domain_stack_arity(f)
        }
    }

    /// How many of `f`'s value bytes are function indices — shared core
    /// first, domain hooks above the floor.
    fn body_refs(&self, f: FnIndex) -> u8 {
        if f.0 < DOMAIN_FLOOR {
            shared_core::body_refs(f)
        } else {
            self.domain_body_refs(f)
        }
    }

    /// Does lowering `f` require emitting another function?
    fn branches(&self, f: FnIndex) -> bool {
        self.body_refs(f) > 0
    }

    /// The narrowest shape that can hold `f`'s body references.
    ///
    /// Three references are LEGAL (they fit `Quads` exactly), so the default
    /// maps them there rather than to `Triples` — an earlier draft's
    /// `_ => Triples` would have made [`conformance::check`] wrongly reject a
    /// conforming three-reference domain function. Four or more references
    /// fit no shape; the default still answers `Quads` and the conformance
    /// shape check is what refuses them.
    fn min_shape(&self, f: FnIndex) -> LaneShape {
        match self.body_refs(f) {
            0 | 1 => LaneShape::Pairs,
            2 => LaneShape::Triples,
            _ => LaneShape::Quads,
        }
    }

    /// Whether a covered domain-range function pushes a result — the column
    /// statement segmentation runs on.
    ///
    /// The default is `None` — "not declared" — which makes segmentation
    /// REFUSE bodies using such functions rather than mis-group them. A
    /// vocabulary that wants its bodies statement-segmentable (templates
    /// masking by `StepMask` position, most importantly) declares the column
    /// for its verbs; a vocabulary that never segments pays nothing.
    fn domain_pushes_result(&self, _f: FnIndex) -> Option<bool> {
        None
    }

    /// Whether `f` pushes a result — shared core first, domain hook above
    /// the floor.
    fn pushes_result(&self, f: FnIndex) -> Option<bool> {
        if f.0 < DOMAIN_FLOOR {
            shared_core::pushes_result(f)
        } else {
            self.domain_pushes_result(f)
        }
    }

    /// The canonical name of a domain-range function, for legends and
    /// oracle schemas. Default `None` — an unnamed slot appears in no
    /// legend, which is honest for reserved space.
    fn domain_name(&self, _f: FnIndex) -> Option<&'static str> {
        None
    }

    /// The name of `f` — shared core first, domain hook above the floor.
    fn name(&self, f: FnIndex) -> Option<&'static str> {
        if f.0 < DOMAIN_FLOOR {
            shared_core::name(f)
        } else {
            self.domain_name(f)
        }
    }

    /// The basin-local codebook `f`'s value bytes are drawn from, if any
    /// (W-RO-3). Default `None` — most calls carry literals or body
    /// references, not codebook ids, so the default costs nothing.
    fn domain_value_codebook(&self, _f: FnIndex) -> Option<ValueCodebook> {
        None
    }

    /// The codebook `f`'s operands resolve against — shared core is always
    /// `None` (the computational core has no basin-scoped operands), domain
    /// hook above the floor.
    fn value_codebook(&self, f: FnIndex) -> Option<ValueCodebook> {
        if f.0 < DOMAIN_FLOOR {
            None
        } else {
            self.domain_value_codebook(f)
        }
    }
}

// ── The canonical data form ─────────────────────────────────────────────────

/// One codebook slot's semantics, as data.
///
/// The data-first ruling (R4 of the ratified design rulings): the
/// authoritative truth about a vocabulary is a TABLE — inspectable by
/// compilers, renderers, fuzzers, and oracle schemas, validated once as a
/// whole, incapable of the mutual inconsistency separate methods can drift
/// into. The trait's methods are the authoring/access surface; the composed
/// [`VocabularyTable`] inside a
/// [`CheckedVocabulary`](conformance::CheckedVocabulary) is what everything
/// downstream actually reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FnSpec {
    /// Operands popped from the stack; `None` = not covered (refused).
    pub stack_arity: Option<u8>,
    /// Value bytes that are function indices this call branches to.
    pub body_refs: u8,
    /// The narrowest lane carving that can hold this call.
    pub min_shape: LaneShape,
    /// Whether the call pushes a result; `None` = not declared, so
    /// statement segmentation refuses rather than guesses.
    pub pushes_result: Option<bool>,
    /// The canonical mnemonic, for legends and oracle schemas; `None` = an
    /// unnamed (reserved) slot, absent from any legend. Lives IN the spec —
    /// OQ-1 answered toward "the table stays the single artifact": a legend
    /// is then a serialization of the validated table, nothing beside it.
    pub name: Option<&'static str>,
    /// The basin-local codebook this call's value bytes resolve against;
    /// `None` = literal/body-reference operands, the common case (W-RO-3).
    pub value_codebook: Option<ValueCodebook>,
}

impl FnSpec {
    /// The uncovered slot — refused everywhere.
    pub const REFUSED: FnSpec = FnSpec {
        stack_arity: None,
        body_refs: 0,
        min_shape: LaneShape::Pairs,
        pushes_result: None,
        name: None,
        value_codebook: None,
    };
}

/// The composed 256-slot semantic table of one vocabulary.
///
/// [`compose`](Self::compose) stamps the shared-core half from
/// [`shared_core`]'s own tables — **never** from the vocabulary — so a
/// sibling cannot even EXPRESS a divergent opinion about a shared byte in
/// the table consumers read. The domain half is sampled from the
/// vocabulary's hooks once, at composition time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VocabularyTable {
    specs: [FnSpec; 256],
}

impl VocabularyTable {
    /// Compose a vocabulary's full table: shared core below the floor
    /// (unforgeable — read from the core, not the vocabulary), the
    /// vocabulary's hooks above it.
    #[must_use]
    pub fn compose<V: Vocabulary>(v: &V) -> Self {
        let mut specs = [FnSpec::REFUSED; 256];
        for (b, spec) in specs.iter_mut().enumerate() {
            let f = FnIndex(b as u8);
            *spec = if f.0 < DOMAIN_FLOOR {
                FnSpec {
                    stack_arity: shared_core::stack_arity(f),
                    body_refs: shared_core::body_refs(f),
                    min_shape: shared_core::min_shape(f),
                    pushes_result: shared_core::pushes_result(f),
                    name: shared_core::name(f),
                    value_codebook: None,
                }
            } else {
                FnSpec {
                    stack_arity: v.domain_stack_arity(f),
                    body_refs: v.domain_body_refs(f),
                    min_shape: v.min_shape(f),
                    pushes_result: v.domain_pushes_result(f),
                    name: v.domain_name(f),
                    value_codebook: v.domain_value_codebook(f),
                }
            };
        }
        Self { specs }
    }

    /// The spec for one slot.
    #[must_use]
    pub fn spec(&self, f: FnIndex) -> &FnSpec {
        &self.specs[usize::from(f.0)]
    }

    /// Operands `f` pops; `None` = refused.
    #[must_use]
    pub fn stack_arity(&self, f: FnIndex) -> Option<u8> {
        self.spec(f).stack_arity
    }

    /// Function-index value bytes `f` carries.
    #[must_use]
    pub fn body_refs(&self, f: FnIndex) -> u8 {
        self.spec(f).body_refs
    }

    /// Whether `f` references a body at all.
    #[must_use]
    pub fn branches(&self, f: FnIndex) -> bool {
        self.spec(f).body_refs > 0
    }

    /// The narrowest shape holding `f`.
    #[must_use]
    pub fn min_shape(&self, f: FnIndex) -> LaneShape {
        self.spec(f).min_shape
    }

    /// Whether `f` pushes a result; `None` = not declared.
    #[must_use]
    pub fn pushes_result(&self, f: FnIndex) -> Option<bool> {
        self.spec(f).pushes_result
    }

    /// The canonical mnemonic of `f`; `None` = unnamed (reserved) slot.
    #[must_use]
    pub fn name(&self, f: FnIndex) -> Option<&'static str> {
        self.spec(f).name
    }

    /// The basin-local codebook `f`'s operands resolve against; `None` = no
    /// codebook (literal or body-reference operands) — the common case.
    #[must_use]
    pub fn value_codebook(&self, f: FnIndex) -> Option<ValueCodebook> {
        self.spec(f).value_codebook
    }
}

/// Mechanical conformance: what every vocabulary crate's tests must run.
pub mod conformance {
    use super::{DOMAIN_FLOOR, FnIndex, Vocabulary, VocabularyTable, shared_core};

    /// A way a vocabulary violates the sharing discipline.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ConformanceError {
        /// A shared-core byte answers differently through the vocabulary than
        /// through [`shared_core`] — a composed-method override drifted.
        SharedCoreDrift {
            /// The byte that drifted.
            f: FnIndex,
            /// Which table drifted: `"stack_arity"`, `"body_refs"`, or
            /// `"pushes_result"`.
            what: &'static str,
        },
        /// `min_shape` reports a shape too narrow to hold the function's own
        /// body references — a truncation waiting to happen.
        ShapeTooNarrowForRefs {
            /// The offending byte.
            f: FnIndex,
        },
    }

    impl core::fmt::Display for ConformanceError {
        fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                ConformanceError::SharedCoreDrift { f, what } => write!(
                    fmt,
                    "vocabulary drifts from the shared core on {what} for {f:?} — \
                     shared-core bytes are answered by the core, never the vocabulary"
                ),
                ConformanceError::ShapeTooNarrowForRefs { f } => write!(
                    fmt,
                    "{f:?}: min_shape cannot hold the call's own body references"
                ),
            }
        }
    }

    impl core::error::Error for ConformanceError {}

    /// A vocabulary that has PASSED [`check`] — the proof-carrying form.
    ///
    /// Program traversal ([`Program::references_are_resolvable`],
    /// [`branches_of`]) requires this wrapper rather than a bare
    /// [`Vocabulary`], because those walks index a call's fixed three value
    /// bytes by `body_refs` — under an unvalidated vocabulary claiming more
    /// than three references that indexing would panic. The wrapper turns
    /// "every consumer remembers to run the conformance test" into a type:
    /// it cannot be constructed except through [`validate`], so holding one
    /// IS the proof that `body_refs ≤ 3` everywhere (the shape check bounds
    /// it: no [`LaneShape`](crate::LaneShape) holds more than three value
    /// bytes).
    ///
    /// It implements [`Vocabulary`] by delegation, so it composes anywhere a
    /// vocabulary is accepted.
    ///
    /// [`Program::references_are_resolvable`]: crate::Program::references_are_resolvable
    /// [`branches_of`]: crate::branches_of
    ///
    /// # Table-backed since the data-first ruling (R4)
    ///
    /// The wrapper stores the [`VocabularyTable`] composed at validation and
    /// answers every semantic query FROM that table — not by delegating to
    /// the inner vocabulary. Delegation left one pathological gap (a
    /// vocabulary whose methods answer differently across calls); the stored
    /// table closes it: what was validated is, byte for byte, what is read.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CheckedVocabulary<V: Vocabulary> {
        vocab: V,
        table: VocabularyTable,
    }

    impl<V: Vocabulary> CheckedVocabulary<V> {
        /// Borrow the validated vocabulary.
        pub fn inner(&self) -> &V {
            &self.vocab
        }

        /// Unwrap, discarding the proof.
        pub fn into_inner(self) -> V {
            self.vocab
        }

        /// The composed, validated semantic table — the authoritative data
        /// form everything downstream reads.
        pub fn table(&self) -> &VocabularyTable {
            &self.table
        }
    }

    impl<V: Vocabulary> Vocabulary for CheckedVocabulary<V> {
        fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
            self.vocab.domain_stack_arity(f)
        }
        fn domain_body_refs(&self, f: FnIndex) -> u8 {
            self.vocab.domain_body_refs(f)
        }
        fn domain_pushes_result(&self, f: FnIndex) -> Option<bool> {
            self.vocab.domain_pushes_result(f)
        }
        // The composed methods read the VALIDATED TABLE, never the inner
        // vocabulary: what was checked is what answers.
        fn stack_arity(&self, f: FnIndex) -> Option<u8> {
            self.table.stack_arity(f)
        }
        fn body_refs(&self, f: FnIndex) -> u8 {
            self.table.body_refs(f)
        }
        fn branches(&self, f: FnIndex) -> bool {
            self.table.branches(f)
        }
        fn min_shape(&self, f: FnIndex) -> crate::LaneShape {
            self.table.min_shape(f)
        }
        fn pushes_result(&self, f: FnIndex) -> Option<bool> {
            self.table.pushes_result(f)
        }
        fn domain_name(&self, f: FnIndex) -> Option<&'static str> {
            self.vocab.domain_name(f)
        }
        fn name(&self, f: FnIndex) -> Option<&'static str> {
            self.table.name(f)
        }
        fn domain_value_codebook(&self, f: FnIndex) -> Option<super::ValueCodebook> {
            self.vocab.domain_value_codebook(f)
        }
        fn value_codebook(&self, f: FnIndex) -> Option<super::ValueCodebook> {
            self.table.value_codebook(f)
        }
    }

    /// Validate a vocabulary and, on success, return the proof-carrying
    /// wrapper program traversal requires — carrying the composed
    /// [`VocabularyTable`] it was validated as.
    ///
    /// # Compose-then-check (W-1, closing finding F-1)
    ///
    /// The table is composed FIRST, and the shape invariant is then checked
    /// on the **stored table itself** — not only through the method sweep.
    /// The earlier order (`check` then `compose`) sampled the domain hooks
    /// twice, so a phase-unstable vocabulary could pass the check on one set
    /// of answers while the table froze another — including a `body_refs`
    /// beyond a call's capacity, re-opening the traversal edge the wrapper's
    /// proof exists to close. Now the artifact everything reads is the
    /// proven object, to zero resamplings: [`check`]'s method sweep still
    /// runs (drift detection inherently reads the methods), and whichever
    /// sampling a hostile vocabulary poisons, one of the two gates fires.
    ///
    /// # Errors
    ///
    /// The first [`ConformanceError`] found, naming the byte and the defect.
    /// [`check`] remains available for test-time assertion without taking
    /// ownership.
    pub fn validate<V: Vocabulary>(v: V) -> Result<CheckedVocabulary<V>, ConformanceError> {
        let table = VocabularyTable::compose(&v);
        check(&v)?;
        // The stored table is the proven object: re-assert the shape
        // invariant on the exact bytes the wrapper will carry.
        for b in 0..=255u8 {
            let f = FnIndex(b);
            let spec = table.spec(f);
            if spec.min_shape.values_per_call() < usize::from(spec.body_refs) {
                return Err(ConformanceError::ShapeTooNarrowForRefs { f });
            }
        }
        Ok(CheckedVocabulary { vocab: v, table })
    }

    /// Check a vocabulary against the sharing discipline, over the full
    /// 256-byte codebook.
    ///
    /// # Errors
    ///
    /// The first [`ConformanceError`] found, naming the byte and the defect.
    pub fn check<V: Vocabulary>(v: &V) -> Result<(), ConformanceError> {
        for b in 0..=255u8 {
            let f = FnIndex(b);
            if b < DOMAIN_FLOOR {
                // Below the floor the vocabulary must be transparent: its
                // composed answers ARE the core's answers. This includes NOP
                // (0x00), which the core refuses.
                if v.stack_arity(f) != shared_core::stack_arity(f) {
                    return Err(ConformanceError::SharedCoreDrift {
                        f,
                        what: "stack_arity",
                    });
                }
                if v.body_refs(f) != shared_core::body_refs(f) {
                    return Err(ConformanceError::SharedCoreDrift {
                        f,
                        what: "body_refs",
                    });
                }
                if v.pushes_result(f) != shared_core::pushes_result(f) {
                    return Err(ConformanceError::SharedCoreDrift {
                        f,
                        what: "pushes_result",
                    });
                }
                if v.name(f) != shared_core::name(f) {
                    return Err(ConformanceError::SharedCoreDrift { f, what: "name" });
                }
                if v.value_codebook(f).is_some() {
                    return Err(ConformanceError::SharedCoreDrift {
                        f,
                        what: "value_codebook",
                    });
                }
            }
            // Everywhere: the reported minimum shape must actually hold the
            // call's own body references.
            if v.min_shape(f).values_per_call() < usize::from(v.body_refs(f)) {
                return Err(ConformanceError::ShapeTooNarrowForRefs { f });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::conformance::{ConformanceError, check};
    use super::*;

    /// A vocabulary with an empty domain range — the smallest conforming
    /// implementation (and exactly what a palette that is all shared-core
    /// looks like).
    struct EmptyVocab;
    impl Vocabulary for EmptyVocab {
        fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
            None
        }
        fn domain_body_refs(&self, _f: FnIndex) -> u8 {
            0
        }
    }

    /// A well-behaved domain vocabulary: two functions above the floor, one
    /// of which branches.
    struct DomainVocab;
    impl Vocabulary for DomainVocab {
        fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
            match f.0 {
                0x90 => Some(1),
                0x91 => Some(0),
                _ => None,
            }
        }
        fn domain_body_refs(&self, f: FnIndex) -> u8 {
            u8::from(f.0 == 0x91)
        }
    }

    /// The drift the conformance check exists to catch: an override of the
    /// COMPOSED method that changes a shared-core answer.
    struct DriftingVocab;
    impl Vocabulary for DriftingVocab {
        fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
            None
        }
        fn domain_body_refs(&self, _f: FnIndex) -> u8 {
            0
        }
        fn stack_arity(&self, f: FnIndex) -> Option<u8> {
            // ADD as a unary operator — the classic silent stack
            // desynchronization.
            if f == FnIndex::ADD {
                Some(1)
            } else if f.0 < DOMAIN_FLOOR {
                shared_core::stack_arity(f)
            } else {
                self.domain_stack_arity(f)
            }
        }
    }

    /// A min_shape override that would truncate IF_ELSE's else arm.
    struct NarrowShapeVocab;
    impl Vocabulary for NarrowShapeVocab {
        fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
            None
        }
        fn domain_body_refs(&self, _f: FnIndex) -> u8 {
            0
        }
        fn min_shape(&self, _f: FnIndex) -> LaneShape {
            LaneShape::Pairs
        }
    }

    #[test]
    fn a_body_reference_is_not_a_stack_operand() {
        // THE distinction. `repeat` pops the COUNT (one operand) and
        // references a body. If the body were counted as an operand, nesting
        // would pop something that was never pushed and every earlier operand
        // would shift by one.
        assert_eq!(shared_core::stack_arity(FnIndex::REPEAT), Some(1));
        assert_eq!(shared_core::body_refs(FnIndex::REPEAT), 1);
        // `forever` proves the two are genuinely independent: zero operands,
        // one body. A single conflated number cannot express it.
        assert_eq!(shared_core::stack_arity(FnIndex::FOREVER), Some(0));
        assert_eq!(shared_core::body_refs(FnIndex::FOREVER), 1);
        // …and the mirror: an expression pops operands and references nothing.
        assert_eq!(shared_core::stack_arity(FnIndex::ADD), Some(2));
        assert_eq!(shared_core::body_refs(FnIndex::ADD), 0);
        assert_eq!(shared_core::body_refs(FnIndex::SQRT), 0);
    }

    #[test]
    fn if_else_carries_two_bodies_and_therefore_needs_a_wider_shape() {
        assert_eq!(shared_core::body_refs(FnIndex::IF_ELSE), 2);
        assert_eq!(shared_core::stack_arity(FnIndex::IF_ELSE), Some(1));
        // Under Pairs the else arm would be truncated away and the program
        // would run the then-branch and silently skip the else — the exact
        // "looks complete and is not" failure the ABI refuses elsewhere.
        assert_eq!(shared_core::min_shape(FnIndex::IF_ELSE), LaneShape::Triples);
        // Two-sided: one-body forms fit Pairs, so the requirement is specific
        // to IF_ELSE rather than a blanket widening of all control flow.
        assert_eq!(shared_core::min_shape(FnIndex::IF), LaneShape::Pairs);
        assert_eq!(shared_core::min_shape(FnIndex::REPEAT), LaneShape::Pairs);
        assert_eq!(shared_core::min_shape(FnIndex::ADD), LaneShape::Pairs);
    }

    #[test]
    fn the_two_tables_agree_on_what_is_covered() {
        // A function with a stack arity but no body-ref entry (or vice versa)
        // would lower half-correctly. Every control-flow opcode the shared
        // core names must appear consistently in both.
        for f in [
            FnIndex::IF,
            FnIndex::IF_ELSE,
            FnIndex::REPEAT,
            FnIndex::WHILE,
            FnIndex::REPEAT_UNTIL,
            FnIndex::FOREVER,
            FnIndex::FOR_EACH,
            FnIndex::FOR_RANGE,
        ] {
            assert!(
                shared_core::stack_arity(f).is_some(),
                "{f:?} has no stack arity"
            );
            assert!(shared_core::branches(f), "{f:?} should reference a body");
        }
        // Silence twin: what the core does NOT model stays refused rather
        // than being quietly assigned a plausible shape.
        //
        // Re-pinned when the coverage sweep landed: WAIT / WAIT_UNTIL / STOP
        // are now covered (both frontends declare their shape, so there was
        // nothing left to guess). What remains refused is the genuinely
        // VARIADIC set — an arity that belongs to the call, not the function
        // — which is a design question and not a table row. See
        // `shared_core::stack_arity`'s doc table.
        for f in [
            FnIndex::RETURN,
            FnIndex::NUMBER_PROPERTY,
            FnIndex::PROMPT,
            FnIndex::LIST_WITH,
            FnIndex::PROC_CALL,
        ] {
            assert_eq!(shared_core::stack_arity(f), None, "{f:?} must stay refused");
        }
        // …and the newly covered half, so this test cannot pass by refusing
        // everything: a silence twin with no can-fire half proves nothing.
        assert_eq!(shared_core::stack_arity(FnIndex::WAIT), Some(1));
        assert_eq!(shared_core::stack_arity(FnIndex::STOP), Some(0));
        assert_eq!(shared_core::stack_arity(FnIndex::LIST_SET), Some(3));
    }

    #[test]
    fn the_composed_methods_route_by_the_floor() {
        let v = DomainVocab;
        // Below the floor: the core answers, and the domain hook is never the
        // source (its answer for ADD would be None — the composed method must
        // NOT return that).
        assert_eq!(v.stack_arity(FnIndex::ADD), Some(2));
        assert_eq!(v.domain_stack_arity(FnIndex::ADD), None);
        // Above the floor: the domain answers.
        assert_eq!(v.stack_arity(FnIndex(0x90)), Some(1));
        assert_eq!(v.body_refs(FnIndex(0x91)), 1);
        assert!(v.branches(FnIndex(0x91)));
        // …and an unclaimed domain byte is refused, not guessed.
        assert_eq!(v.stack_arity(FnIndex(0xF0)), None);
    }

    #[test]
    fn conformance_stays_silent_for_conforming_vocabularies() {
        // The silence half — over NON-trivial inputs: DomainVocab genuinely
        // claims bytes above the floor and still passes.
        assert_eq!(check(&EmptyVocab), Ok(()));
        assert_eq!(check(&DomainVocab), Ok(()));
    }

    #[test]
    fn conformance_fires_on_a_shared_core_drift() {
        // The can-fire half: a vocabulary that overrides the composed method
        // and re-answers ADD is caught, and the error names the byte.
        assert_eq!(
            check(&DriftingVocab),
            Err(ConformanceError::SharedCoreDrift {
                f: FnIndex::ADD,
                what: "stack_arity",
            })
        );
    }

    #[test]
    fn three_body_references_are_legal_and_default_to_quads() {
        // The default min_shape must map 3 refs to Quads (they fit exactly).
        // An earlier draft mapped everything >=2 to Triples, which would have
        // wrongly rejected this conforming vocabulary — surfaced while
        // hardening the >3 panic edge, pinned here so it cannot regress.
        struct TripleRefVocab;
        impl Vocabulary for TripleRefVocab {
            fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
                (f.0 == 0x90).then_some(0)
            }
            fn domain_body_refs(&self, f: FnIndex) -> u8 {
                if f.0 == 0x90 { 3 } else { 0 }
            }
        }
        assert_eq!(
            TripleRefVocab.min_shape(FnIndex(0x90)),
            LaneShape::Quads,
            "three refs fit Quads exactly"
        );
        assert_eq!(check(&TripleRefVocab), Ok(()));
    }

    #[test]
    fn validate_refuses_a_vocabulary_whose_refs_exceed_a_calls_capacity() {
        // The panic edge the proof-carrying wrapper closes: body_refs beyond
        // MAX_VALUES_PER_CALL would drive `call.values[slot]` out of bounds
        // in program traversal. No LaneShape holds more than three value
        // bytes, so even a min_shape override answering Quads cannot make
        // 200 references conform — validate must refuse, and traversal is
        // unreachable without the wrapper it refused to construct.
        struct HostileVocab;
        impl Vocabulary for HostileVocab {
            fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
                Some(0)
            }
            fn domain_body_refs(&self, f: FnIndex) -> u8 {
                if f.0 == 0x90 { 200 } else { 0 }
            }
            fn min_shape(&self, _f: FnIndex) -> LaneShape {
                LaneShape::Quads
            }
        }
        assert_eq!(
            conformance::validate(HostileVocab).err(),
            Some(ConformanceError::ShapeTooNarrowForRefs { f: FnIndex(0x90) })
        );
        // Silence twin: validate hands back the proof for conforming
        // vocabularies, and the wrapper answers exactly as the inner one.
        let checked = conformance::validate(EmptyVocab).expect("EmptyVocab conforms");
        assert_eq!(checked.stack_arity(FnIndex::ADD), Some(2));
        assert_eq!(checked.body_refs(FnIndex::IF_ELSE), 2);
        assert_eq!(checked.inner().body_refs(FnIndex::IF_ELSE), 2);
    }

    #[test]
    fn conformance_fires_on_a_shape_too_narrow_for_the_refs() {
        // IF_ELSE under a Pairs-only min_shape would truncate its else arm.
        assert_eq!(
            check(&NarrowShapeVocab),
            Err(ConformanceError::ShapeTooNarrowForRefs {
                f: FnIndex::IF_ELSE
            })
        );
    }

    #[test]
    fn the_composed_tables_shared_half_is_unforgeable() {
        // The data-first strengthening: compose() reads the shared half from
        // the core, never from the vocabulary — so even the DRIFTING
        // vocabulary's table carries the correct ADD. The drift exists only
        // in its methods, where check() still rejects it. Both facts
        // together: the table cannot express the defect, and the vocabulary
        // that tries never gets a table.
        let table = VocabularyTable::compose(&DriftingVocab);
        assert_eq!(table.stack_arity(FnIndex::ADD), Some(2));
        assert_eq!(DriftingVocab.stack_arity(FnIndex::ADD), Some(1));
        assert!(conformance::validate(DriftingVocab).is_err());
    }

    #[test]
    fn the_checked_wrapper_answers_from_its_validated_table() {
        let checked = conformance::validate(DomainVocab).expect("DomainVocab conforms");
        // Shared core through the table: expressions push, control does not,
        // uncovered stays None.
        assert_eq!(checked.pushes_result(FnIndex::ADD), Some(true));
        assert_eq!(checked.pushes_result(FnIndex::REPEAT), Some(false));
        assert_eq!(checked.pushes_result(FnIndex::RETURN), None);
        // Domain half sampled from the hooks; the undeclared pushes column
        // defaults to None (refused by segmentation, never guessed).
        assert_eq!(checked.table().stack_arity(FnIndex(0x90)), Some(1));
        assert_eq!(checked.table().pushes_result(FnIndex(0x90)), None);
        // Table and composed methods agree byte-for-byte — the methods READ
        // the table, so disagreement is unrepresentable.
        for b in 0..=255u8 {
            let f = FnIndex(b);
            assert_eq!(checked.stack_arity(f), checked.table().stack_arity(f));
            assert_eq!(checked.body_refs(f), checked.table().body_refs(f));
        }
    }

    #[test]
    fn validate_catches_a_phase_unstable_vocabulary_in_either_phase() {
        // THE F-1 regression (wishlist W-1). A vocabulary whose hooks answer
        // differently between validate's two samplings must be caught no
        // matter WHICH sampling it poisons. Under the old check-then-compose
        // order, poisoning the SECOND sampling slipped through: check passed
        // on clean answers, then compose froze body_refs=200 into the table
        // the wrapper carried — the exact traversal edge the proof fences.
        struct UnstableVocab {
            poison_on_poll: u32,
            polls: core::cell::Cell<u32>,
        }
        impl Vocabulary for UnstableVocab {
            fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
                Some(0)
            }
            fn domain_body_refs(&self, f: FnIndex) -> u8 {
                if f.0 != 0x90 {
                    return 0;
                }
                let n = self.polls.get();
                self.polls.set(n + 1);
                if n == self.poison_on_poll { 200 } else { 0 }
            }
            fn min_shape(&self, _f: FnIndex) -> LaneShape {
                // Quads, so ONLY a poisoned 200 (never a clean 0) can fail
                // the shape invariant — the test isolates instability.
                LaneShape::Quads
            }
        }
        let poison_first = UnstableVocab {
            poison_on_poll: 0, // compose's sampling gets 200 → stored-table gate
            polls: core::cell::Cell::new(0),
        };
        assert_eq!(
            conformance::validate(poison_first).err(),
            Some(ConformanceError::ShapeTooNarrowForRefs { f: FnIndex(0x90) }),
            "a poisoned TABLE sampling must be caught by the stored-table gate"
        );
        let poison_second = UnstableVocab {
            poison_on_poll: 1, // check's sampling gets 200 → method gate
            polls: core::cell::Cell::new(0),
        };
        assert_eq!(
            conformance::validate(poison_second).err(),
            Some(ConformanceError::ShapeTooNarrowForRefs { f: FnIndex(0x90) }),
            "a poisoned METHOD sampling must be caught by the method gate"
        );
        // Silence twin: the same shape with NO poison validates fine.
        let stable = UnstableVocab {
            poison_on_poll: u32::MAX,
            polls: core::cell::Cell::new(0),
        };
        assert!(conformance::validate(stable).is_ok());
    }

    #[test]
    fn the_name_column_serves_the_legend_and_cannot_drift() {
        // W-2: names are not coverage — refused-but-real entries are named
        // so a legend can say "exists, refused" rather than omitting them.
        assert_eq!(shared_core::name(FnIndex::ADD), Some("ADD"));
        assert_eq!(shared_core::name(FnIndex::RETURN), Some("RETURN"));
        assert_eq!(shared_core::stack_arity(FnIndex::RETURN), None);
        assert_eq!(shared_core::name(FnIndex::NOP), None, "NOP is not an op");
        assert_eq!(shared_core::name(FnIndex(0x1F)), None, "reserved = unnamed");

        // Composed: shared names ride the table; an undeclared domain slot
        // stays unnamed; a declared one is served through the table.
        struct NamedDomain;
        impl Vocabulary for NamedDomain {
            fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
                (f.0 == 0x90).then_some(0)
            }
            fn domain_body_refs(&self, _f: FnIndex) -> u8 {
                0
            }
            fn domain_name(&self, f: FnIndex) -> Option<&'static str> {
                (f.0 == 0x90).then_some("OBSERVE")
            }
        }
        let checked = conformance::validate(NamedDomain).unwrap();
        assert_eq!(checked.table().name(FnIndex::IF_ELSE), Some("IF_ELSE"));
        assert_eq!(checked.table().name(FnIndex(0x90)), Some("OBSERVE"));
        assert_eq!(checked.table().name(FnIndex(0x91)), None);

        // The name is a drift channel like the other three: a vocabulary
        // renaming a shared-core op is caught by name.
        struct RenamingVocab;
        impl Vocabulary for RenamingVocab {
            fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
                None
            }
            fn domain_body_refs(&self, _f: FnIndex) -> u8 {
                0
            }
            fn name(&self, f: FnIndex) -> Option<&'static str> {
                if f == FnIndex::ADD {
                    Some("PLUS")
                } else if f.0 < DOMAIN_FLOOR {
                    shared_core::name(f)
                } else {
                    None
                }
            }
        }
        assert_eq!(
            check(&RenamingVocab),
            Err(ConformanceError::SharedCoreDrift {
                f: FnIndex::ADD,
                what: "name",
            })
        );
    }

    #[test]
    fn conformance_fires_on_a_pushes_drift_too() {
        // The new column is a new drift channel; it must be guarded like the
        // other two. ADD claiming to push nothing would silently break
        // statement segmentation for every consumer.
        struct PushDriftVocab;
        impl Vocabulary for PushDriftVocab {
            fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
                None
            }
            fn domain_body_refs(&self, _f: FnIndex) -> u8 {
                0
            }
            fn pushes_result(&self, f: FnIndex) -> Option<bool> {
                if f == FnIndex::ADD {
                    Some(false)
                } else if f.0 < DOMAIN_FLOOR {
                    shared_core::pushes_result(f)
                } else {
                    None
                }
            }
        }
        assert_eq!(
            check(&PushDriftVocab),
            Err(ConformanceError::SharedCoreDrift {
                f: FnIndex::ADD,
                what: "pushes_result",
            })
        );
    }
}

#[cfg(test)]
mod coverage {
    use super::*;

    /// Every function the palette NAMES has an arity, except the five
    /// documented variadic entries.
    ///
    /// The core used to name 96 operations and give arities for 50 — so a
    /// consumer could resolve an opcode, get a legal `FnIndex`, and then find
    /// the substrate unable to say what it consumed. That gap was invisible
    /// from inside `ogar-loco` (nothing here enumerated it) and surfaced only
    /// when a consumer mapped a second frontend onto the same core.
    ///
    /// This test is the enumeration that was missing. It walks the whole byte
    /// range rather than a hand-listed set, so a NEWLY MINTED constant with no
    /// arity row fails here immediately instead of years later in a consumer.
    #[test]
    fn every_named_function_has_an_arity_except_the_documented_variadics() {
        const VARIADIC: [FnIndex; 5] = [
            FnIndex::RETURN,
            FnIndex::NUMBER_PROPERTY,
            FnIndex::PROMPT,
            FnIndex::LIST_WITH,
            FnIndex::PROC_CALL,
        ];

        let mut named = 0usize;
        let mut missing = Vec::new();
        for b in 0..DOMAIN_FLOOR {
            let f = FnIndex(b);
            if shared_core::name(f).is_none() {
                // Not a minted operation — nothing is owed for it. But it
                // must not answer either, or the range would be lying.
                assert_eq!(
                    shared_core::stack_arity(f),
                    None,
                    "{f:?} is unnamed yet reports an arity"
                );
                continue;
            }
            named += 1;
            if shared_core::stack_arity(f).is_none() && !VARIADIC.contains(&f) {
                missing.push((b, shared_core::name(f)));
            }
        }
        assert!(
            missing.is_empty(),
            "named operations with no arity and no documented reason: {missing:?}"
        );

        // Can-stay-silent: the variadics really are still refused, so the
        // assertion above cannot be satisfied by covering everything.
        for f in VARIADIC {
            assert_eq!(shared_core::stack_arity(f), None, "{f:?} must stay refused");
            assert!(
                shared_core::name(f).is_some(),
                "{f:?} must still be NAMED — refused is not the same as absent"
            );
        }

        // And the census itself, pinned: 101 named operations (102 constants
        // minus NOP, which is not an operation), 5 refused, 96 covered.
        // Re-pinned 2026-09-01: +6 for the epistemic mask-algebra band
        // (0x86..0x8B — TERNLOG, BELNAP_JOIN, INFO_GAIN, SIGMA_TENSION,
        // ACCUMULATE, STANCE_ENTROPY), minted into the reserved core slots.
        assert_eq!(named, 101, "the named census moved — re-pin deliberately");
    }
}
