// SPDX-License-Identifier: Apache-2.0

//! `ogar-r2il` — **proxy glue**: r2sleigh's R2IL opcode set as an
//! [`ogar_loco::Vocabulary`], plus the **masked lane projection** that
//! re-reads one already-written body under any [`LaneShape`] without
//! rebuilding it.
//!
//! # Why a proxy, and why it is ALWAYS ON
//!
//! `ogar-loco` says it in its own module doc: it is *"everything that is the
//! same no matter what the bytes mean"*, and *"a vocabulary per domain,
//! declared by whoever owns it"* plugs into the [`Vocabulary`] seam. R2IL is
//! such a domain. This crate is therefore **not a new surface** — it is the
//! sibling codebook that seam was cut for, and it mints nothing: no node
//! layout, no lane carving, no call encoding, no second addressing system.
//!
//! It carries **no `r2sleigh` dependency**, and that is the load-bearing
//! design choice rather than an omission. `r2il`'s opcode set is an
//! *enumeration*; what loco needs from it is an *arity table*. Depending on
//! the crate would drag `r2il`'s `Varnode`/`SpaceId`/`ArchSpec` object graph
//! into every consumer that only wanted to know that `IntAdd` pops two
//! operands. A table of 82 arities is
//! not worth an object graph, so the table is declared here and pinned to the
//! source enum by [`R2ILFn::MNEMONICS`], which a drift test compares against
//! `r2il`'s own opcode list when that crate is present.
//!
//! **Operator ruling (2026-08-26): R2IL is EXECUTED, never pre-converted.**
//! `ruff_r2il` (the ruff-side R2IL→SPO harvest) is NOT on this path and never
//! will be: converting live V4 R2IL down to a V3 SPO projection before running
//! it is a lossy static shadow of semantics the interpreter already has
//! first-class. The MACHINE reading's runtime path is r2sleigh's interpreter
//! executing R2IL in realtime; this crate is the addressing glue that makes
//! those calls loco-addressable — not a converter, not a harvest.
//!
//! **Scope note (2026-09-22), NOT a reversal.** The ruling rules out the
//! `ruff_r2il` SPO pre-pass, and that stands unchanged. What the sentence
//! above no longer implies is that r2sleigh's interpreter is the ONLY lawful
//! runtime path for these bytes: the fold extension band below introduces a
//! second READING of the same table under its own concept id, whose dialect
//! lives in a consumer (lance-graph), not here. Both readings execute R2IL;
//! neither pre-converts it. This crate remains an arity table and addressing
//! glue under either, which is why the ruling needs no storno — only this
//! narrowing of "the runtime path" to "the machine reading's runtime path".
//!
//! **Always on** means exactly this: no feature gate, no `optional = true`,
//! no `cfg`. A consumer that has `ogar-loco` has R2IL semantics available,
//! because a vocabulary that must be switched on is one a caller can forget
//! to switch on.
//!
//! # The domain floor is not negotiable
//!
//! `ogar-loco` reserves `FnIndex < DOMAIN_FLOOR` (0x90) for its shared
//! computational core and reads those bytes *from the core, not from the
//! vocabulary* — `VocabularyTable::compose` is written so a vocabulary
//! **cannot** forge them. R2IL ops therefore occupy `0x90..=0xE1` (82 slots),
//! and the arithmetic ops that look like they "are" the core's `ADD` are
//! deliberately NOT aliased onto it: the core's `ADD` is loco's own semantics,
//! and an R2IL `IntAdd` carries a machine's flag/width behaviour the core
//! never promised. Same word, two contracts — aliasing them would be the
//! "same function name, different semantics" trap `I-LEGACY-API-FEATURE-GATED`
//! catalogues five instances of.
//!
//! # The masked lane projection ("reshuffling")
//!
//! One body's 360 content bytes hold 180 / 120 / 90 calls depending on the
//! [`LaneShape`] they are read under. `ogar_loco::call_in_slab` already reads
//! call `i` under a given shape — the reshuffle primitive exists. What this
//! crate adds is the **masked, lazy** form:
//!
//! ```text
//! project(&slab, shape, &mask)   →  Iterator<(index, Call)>
//! ```
//!
//! Reads like a filtered query, executes as word-tests over a bitmask; no
//! `Vec<Call>` is built and no call outside the mask is ever decoded. The
//! mask is [`CallMask`] — an inline `[u64; 3]` over call indices (three
//! words, no allocation, cover every [`LaneShape`]'s call population), with
//! the same algebra and the same single named materializer as every other
//! mask plane in this workspace, because "a `long[]` of selected ids is
//! still a materialised population" applies here identically.
//!
//! **Reshuffling is a re-READ, never a re-WRITE.** The same slab under two
//! shapes yields two different call streams from the same unchanged bytes —
//! that is the point, and it is why [`project`] takes `&[u8]` and returns an
//! iterator rather than a new body. A shape is a lens, not a migration.

use ogar_loco::{Call, FnIndex, LaneShape, VALUE_SLAB_LEN, Vocabulary, call_in_slab};

/// First `FnIndex` this vocabulary owns — `ogar_loco::DOMAIN_FLOOR`.
pub const R2IL_BASE: u8 = ogar_loco::DOMAIN_FLOOR;

/// How many R2IL opcodes this table covers.
pub const R2IL_OPS: usize = 82;

/// One R2IL opcode, as its `ogar-loco` function index.
///
/// A newtype over the index rather than a mirrored enum: mirroring `r2il`'s
/// 77 variants here would create the second vocabulary this crate exists to
/// avoid, and every mirror is a drift surface. The ordinal IS the identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct R2ILFn(pub FnIndex);

impl R2ILFn {
    /// The opcode mnemonics, in `r2il::R2ILOp` declaration order.
    ///
    /// This is the ONE place the two crates are coupled, and it is coupled by
    /// **name**, not by type. `r2il` owns the enum; this owns the arities; the
    /// drift test owns the claim that they still describe the same set.
    pub const MNEMONICS: [&'static str; R2IL_OPS] = [
        "Copy",
        "Load",
        "Store",
        "Fence",
        "LoadLinked",
        "StoreConditional",
        "AtomicCAS",
        "LoadGuarded",
        "StoreGuarded",
        "IntAdd",
        "IntSub",
        "IntMult",
        "IntDiv",
        "IntSDiv",
        "IntRem",
        "IntSRem",
        "IntNegate",
        "IntCarry",
        "IntSCarry",
        "IntSBorrow",
        "IntAnd",
        "IntOr",
        "IntXor",
        "IntNot",
        "IntLeft",
        "IntRight",
        "IntSRight",
        "IntEqual",
        "IntNotEqual",
        "IntLess",
        "IntSLess",
        "IntLessEqual",
        "IntSLessEqual",
        "IntZExt",
        "IntSExt",
        "BoolNot",
        "BoolAnd",
        "BoolOr",
        "BoolXor",
        "Piece",
        "Subpiece",
        "PopCount",
        "Lzcount",
        "Branch",
        "CBranch",
        "BranchInd",
        "Call",
        "CallInd",
        "Return",
        "FloatAdd",
        "FloatSub",
        "FloatMult",
        "FloatDiv",
        "FloatNeg",
        "FloatAbs",
        "FloatSqrt",
        "FloatCeil",
        "FloatFloor",
        "FloatRound",
        "FloatNaN",
        "FloatEqual",
        "FloatNotEqual",
        "FloatLess",
        "FloatLessEqual",
        "Int2Float",
        "Float2Int",
        "FloatFloat",
        "Trunc",
        "CallOther",
        "Nop",
        "Unimplemented",
        "CpuId",
        "Breakpoint",
        "Multiequal",
        "Indirect",
        "PtrAdd",
        "PtrSub",
        "SegmentOp",
        "New",
        "Cast",
        "Extract",
        "Insert",
    ];

    /// The function index for opcode `ordinal`, or [`None`] past the table.
    #[must_use]
    pub const fn from_ordinal(ordinal: usize) -> Option<Self> {
        if ordinal < R2IL_OPS {
            Some(Self(FnIndex(R2IL_BASE + ordinal as u8)))
        } else {
            None
        }
    }

    /// The opcode ordinal, or [`None`] when `f` is not ours (core bytes and
    /// other domains both answer `None` — a vocabulary that claimed either
    /// would be forging the shared core).
    #[must_use]
    pub const fn ordinal(f: FnIndex) -> Option<usize> {
        if f.0 < R2IL_BASE {
            return None;
        }
        let o = (f.0 - R2IL_BASE) as usize;
        if o < R2IL_OPS { Some(o) } else { None }
    }

    /// The canonical mnemonic.
    #[must_use]
    pub fn mnemonic(self) -> Option<&'static str> {
        Self::ordinal(self.0).map(|o| Self::MNEMONICS[o])
    }
}

/// How many operands each opcode pops, in [`R2ILFn::MNEMONICS`] order.
///
/// **Derived from `r2il::R2ILOp`'s own field structure**, not written by
/// hand: an opcode's arity is the count of its `Varnode`-typed INPUT fields
/// (`dst`/`output`/`result` excluded, being the push side). A first draft of
/// this table WAS written from memory and was wrong in fourteen places
/// against an enum of 82 — five variants short, nine invented
/// (`Switch`, `Halt`, `AtomicRmw`, …). The table is data read from a source,
/// per this workspace's data-as-config rule, and
/// `the_table_matches_r2il_s_own_enum` is the guard that keeps it so.
///
/// **`None` means variadic, not zero.** `CallOther` and `Multiequal` carry
/// `inputs: Vec<Varnode>` — their operand count is a per-site property no
/// table can hold. `None` is `Vocabulary`'s own "not covered (refused)"
/// answer, so a body that uses them is refused rather than silently treated
/// as nullary, which is what a `0` here would have meant.
const ARITY: [Option<u8>; R2IL_OPS] = [
    Some(1),
    Some(1),
    Some(2),
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(2),
    Some(3),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(1),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(1),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(1),
    Some(1),
    Some(1),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(1),
    Some(1),
    Some(1),
    Some(1),
    Some(2),
    Some(1),
    Some(1),
    Some(1),
    Some(1),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(1),
    Some(1),
    Some(1),
    Some(1),
    Some(1),
    Some(1),
    Some(1),
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(1),
    Some(1),
    Some(1),
    Some(1),
    None,
    Some(0),
    Some(0),
    Some(0),
    Some(0),
    None,
    Some(2),
    Some(2),
    Some(2),
    Some(2),
    Some(1),
    Some(1),
    Some(2),
    Some(3),
];

/// Which opcodes push a result.
const PUSHES: [bool; R2IL_OPS] = [
    true, true, false, false, true, true, true, true, false, true, true, true, true, true, true,
    true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true,
    true, true, true, true, true, true, true, true, true, true, true, true, false, false, false,
    false, false, false, true, true, true, true, true, true, true, true, true, true, true, true,
    true, true, true, true, true, true, true, true, false, false, true, false, true, true, true,
    true, true, true, true, true, true,
];

// ── The fold extension band (0xE2..=0xED) ───────────────────────────────────
//
// **One table, two classids.** `R2ILVocabulary` answers for TWO readings of
// the SAME `VocabularyTable`, plugged into a [`ogar_loco::VocabularyRegistry`]
// under two distinct concept ids ([`CONCEPT_R2IL_MACHINE`] and
// [`CONCEPT_R2IL_FOLD`]). Composition (`VocabularyTable::compose`) samples
// `domain_stack_arity`/`domain_pushes_result`/`domain_body_refs` exactly
// once, so arity, pushes and body_refs are IDENTICAL under both classids by
// construction — only the *dialect* interpreting a resolved call differs.
// Plugging one table under two ids is not a workaround for missing per-
// classid state; it is the whole point: the table describes SHAPE, and shape
// does not change when the reader's intent does.
//
// **The fold band's rows are declarations of arity, not implementations.**
// This crate never executes anything — it is proxy glue (see the module
// docs) — so `FOLD_ARITY`/`FOLD_PUSHES` are the same kind of fact as
// `ARITY`/`PUSHES` above: a machine dialect refuses every `0xE2..` byte by
// name (scalar R2IL has no lane population to fold over), and a fold
// dialect refuses `CBranch` (branching on a population is not a single
// decision) and the unsigned compares `IntLess`/`IntLessEqual` (mask-risc,
// the fold dialect's target machine, has no unsigned lane compare — only
// the signed pair `IntSLess`/`IntSLessEqual`). Declaring those refusals is
// each dialect's job, not this table's; the table only says what SHAPE a
// byte has when it is not refused.
//
// **Why each of the twelve exists.** Every row is backed by a mask-risc
// `Pred`/`Terminal` that already earned its existence in a prior wave with
// its own parity case — minting a byte here NAMES what already passed,
// rather than speculating ahead of one:
//   - `VIA`        — indexed gather, the lane-population read.
//   - `RANGE`       — the iota/arange population constructor (arity 0: it
//                     needs no input population, only the shape it fills).
//   - `SUM`/`MIN`/`MAX` — the three reduction terminals.
//   - `GROUP_SUM`   — grouped reduction (population, key, accumulator).
//   - `KEY_RUNS`    — run-length grouping over a sorted key population.
//   - `ANY`/`ALL`   — the two Boolean reduction terminals.
//   - `KEEP`        — the compaction/select-by-mask terminal (no push: it
//                     narrows a population in place rather than producing a
//                     new scalar).
//   - `SCATTER_OR`  — scatter-write with OR-merge (a write-shaped terminal:
//                     no push).
//   - `BLEND`       — three-input predicated merge over a population.
//
// **Why `VIA` does not reuse `PtrAdd` (R2IL ordinal 71, arity 2).** Scalar
// pointer arithmetic (`PtrAdd`: base + offset, one machine word) and a
// lane-indexed gather (`VIA`: an index population selecting FROM a lane
// population) are different operations over different domains — one scalar,
// one population-shaped. Reusing `PtrAdd`'s mnemonic for the fold reading
// would make the same byte lie about its own arity's *meaning* even though
// the arity numeral (2) happens to coincide.
//
// **Why `BLEND` does not reuse `Multiequal` (R2IL ordinal 72).**
// `Multiequal`'s shared-table arity is `None` — REFUSED, being one of the
// two genuinely variadic R2IL opcodes (`variadic_opcodes_are_refused_rather_
// than_called_nullary` pins this). Changing that refusal to `Some(3)` so
// `BLEND` could ride the same byte would change the MACHINE reading, which
// the "answers exactly as before" invariant (below) forbids. `BLEND` earns
// its own byte instead.
//
// **What is deliberately NOT minted here, and why.** A row minted before its
// falsifier passed is enum explosion — the exact failure mode `ARITY`'s own
// doc comment above records (a first draft invented nine variants from
// memory). Held back:
//   - `FIRST`         — the witness terminal; earns a byte when a
//                        `receive`-shaped probe demonstrably fails without
//                        one.
//   - `MATCH`         — `MatchU32`/`MatchU64` have no P-Code spelling and no
//                        frontend emits them yet.
//   - `SCATTER_COUNT` — the mask-risc terminal exists but is HELD pending
//                        its own parity case.
//
// **`Load` (R2IL ordinal 1, arity 1, pushes true) needs no new byte.**
// P-Code's own shape already carries the fold reading: the single stack
// operand IS the lane index (the offset), and the lane KIND rides the
// call's immediate as the address SPACE — 0 = u32 lane space, 1 = i32,
// 2 = u64, matching `LaneRef::{U32,I32,U64}`. `Load` is therefore already
// arity-correct under both dialects without joining this band.

/// First `FnIndex` this crate's extension band owns — immediately after the
/// R2IL band, never a hand-written literal.
pub const FOLD_BASE: u8 = R2IL_BASE + R2IL_OPS as u8;

/// How many fold-band opcodes this table covers.
pub const FOLD_OPS: usize = 12;

/// One extension-band opcode, as its `ogar-loco` function index.
///
/// Mirrors [`R2ILFn`]'s newtype-over-index shape for the same reason: the
/// ordinal IS the identity, and a mirrored enum would be the second
/// vocabulary this crate exists to avoid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FoldFn(pub FnIndex);

impl FoldFn {
    /// The function index for fold-band opcode `ordinal`, or [`None`] past
    /// the table.
    #[must_use]
    pub const fn from_ordinal(ordinal: usize) -> Option<Self> {
        if ordinal < FOLD_OPS {
            Some(Self(FnIndex(FOLD_BASE + ordinal as u8)))
        } else {
            None
        }
    }

    /// The fold-band ordinal for `f`, or [`None`] when `f` is not ours — the
    /// R2IL band and every other domain both answer `None` here.
    #[must_use]
    pub const fn ordinal(f: FnIndex) -> Option<usize> {
        if f.0 < FOLD_BASE {
            return None;
        }
        let o = (f.0 - FOLD_BASE) as usize;
        if o < FOLD_OPS { Some(o) } else { None }
    }

    /// The canonical mnemonic.
    #[must_use]
    pub fn mnemonic(self) -> Option<&'static str> {
        Self::ordinal(self.0).map(|o| FOLD_MNEMONICS[o])
    }
}

/// The fold-band mnemonics, in [`FoldFn::from_ordinal`] order.
const FOLD_MNEMONICS: [&str; FOLD_OPS] = [
    "VIA",
    "RANGE",
    "SUM",
    "MIN",
    "MAX",
    "GROUP_SUM",
    "KEY_RUNS",
    "ANY",
    "ALL",
    "KEEP",
    "SCATTER_OR",
    "BLEND",
];

/// How many operands each fold-band opcode pops, in [`FOLD_MNEMONICS`]
/// order — the same "data read from a source, not written from memory" rule
/// as [`ARITY`]: each row is the arity of an already-existing, already
/// parity-cased mask-risc `Pred`/`Terminal` (see the band doc above), never
/// a guess ahead of one.
const FOLD_ARITY: [Option<u8>; FOLD_OPS] = [
    Some(2), // VIA
    Some(0), // RANGE
    Some(2), // SUM
    Some(2), // MIN
    Some(2), // MAX
    Some(3), // GROUP_SUM
    Some(2), // KEY_RUNS
    Some(1), // ANY
    Some(1), // ALL
    Some(1), // KEEP
    Some(2), // SCATTER_OR
    Some(3), // BLEND
];

/// Which fold-band opcodes push a result, in [`FOLD_MNEMONICS`] order.
const FOLD_PUSHES: [bool; FOLD_OPS] = [
    true, true, true, true, true, false, true, true, true, false, false, false,
];

/// `VIA` — indexed gather (ordinal 0).
pub const VIA: FnIndex = FnIndex(FOLD_BASE);
/// `RANGE` — population constructor (ordinal 1).
pub const RANGE: FnIndex = FnIndex(FOLD_BASE + 1);
/// `SUM` — reduction terminal (ordinal 2).
pub const SUM: FnIndex = FnIndex(FOLD_BASE + 2);
/// `MIN` — reduction terminal (ordinal 3).
pub const MIN: FnIndex = FnIndex(FOLD_BASE + 3);
/// `MAX` — reduction terminal (ordinal 4).
pub const MAX: FnIndex = FnIndex(FOLD_BASE + 4);
/// `GROUP_SUM` — grouped reduction (ordinal 5).
pub const GROUP_SUM: FnIndex = FnIndex(FOLD_BASE + 5);
/// `KEY_RUNS` — run-length grouping (ordinal 6).
pub const KEY_RUNS: FnIndex = FnIndex(FOLD_BASE + 6);
/// `ANY` — Boolean reduction terminal (ordinal 7).
pub const ANY: FnIndex = FnIndex(FOLD_BASE + 7);
/// `ALL` — Boolean reduction terminal (ordinal 8).
pub const ALL: FnIndex = FnIndex(FOLD_BASE + 8);
/// `KEEP` — the demanded mask itself, as a terminal (ordinal 9).
///
/// NOT compaction: it hands back the population as a mask, never a dense
/// array of survivors. An earlier doc said "compaction", which would invite
/// exactly the materialisation the zero-copy law forbids.
pub const KEEP: FnIndex = FnIndex(FOLD_BASE + 9);
/// `SCATTER_OR` — scatter-write with OR-merge (ordinal 10).
pub const SCATTER_OR: FnIndex = FnIndex(FOLD_BASE + 10);
/// `BLEND` — three-input predicated merge (ordinal 11).
pub const BLEND: FnIndex = FnIndex(FOLD_BASE + 11);

/// The machine reading's concept id — R2IL as scalar machine semantics.
///
/// **PROVISIONAL.** Not yet minted in `ogar-vocab`'s canonical codebook; no
/// persisted GUID may use this value until it is. Chosen from
/// `ogar_vocab::ConceptDomain::BinaryLifting`'s `0xC4XX` range (the domain
/// this crate's own subject matter — normalized machine-code semantics —
/// already names and reserves), at `0xC400`: the first slot of that domain,
/// unclaimed as of this writing (grepped clean across the workspace before
/// picking it).
pub const CONCEPT_R2IL_MACHINE: u16 = 0xC400;

/// The folded reading's concept id — the same table over populations.
///
/// **PROVISIONAL**, on the same footing as [`CONCEPT_R2IL_MACHINE`]: not yet
/// minted, no persisted GUID may use it until it is. Chosen as the adjacent
/// slot in `ConceptDomain::BinaryLifting`'s `0xC4XX` range, `0xC401`, so the
/// two readings of one table sit next to each other in the codebook the way
/// they sit next to each other in this file.
pub const CONCEPT_R2IL_FOLD: u16 = 0xC401;

/// The R2IL vocabulary — a table, not a translator.
///
/// Zero-sized: it holds no state and allocates nothing, so a consumer can
/// keep one in a `const` and hand it to `VocabularyTable::compose` as often
/// as it likes. That cheapness is what makes "always on" honest rather than
/// a slogan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct R2ILVocabulary;

impl Vocabulary for R2ILVocabulary {
    /// R2IL band first ([`ARITY`]), then the fold band ([`FOLD_ARITY`]);
    /// `None` outside both. The R2IL half is byte-for-byte the original
    /// expression — `the_r2il_band_answers_exactly_as_before` pins that the
    /// extension changed nothing about it.
    fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
        R2ILFn::ordinal(f)
            .and_then(|o| ARITY[o])
            .or_else(|| FoldFn::ordinal(f).and_then(|o| FOLD_ARITY[o]))
    }

    /// Wires the [`PUSHES`] column into the segmentation seam, then the fold
    /// band's [`FOLD_PUSHES`] column.
    ///
    /// Without this override, `ogar_loco::statement_bounds` refuses every
    /// R2IL call as `Uncovered` — the crate shipped the pushes data (the
    /// inherent [`R2ILVocabulary::pushes_result`]) but never answered the
    /// trait hook that the statement walk actually reads, leaving R2IL
    /// bodies lowerable but not segmentable. Found by the counterfactual
    /// witness probe on its first run against a mixed core+R2IL body. The
    /// R2IL half (`Self::pushes_result(f)`) is unchanged by the fold-band
    /// addition — it already answers `None` for a non-R2IL byte, which is
    /// exactly the case the `or_else` fold-band lookup fills in.
    fn domain_pushes_result(&self, f: FnIndex) -> Option<bool> {
        Self::pushes_result(f).or_else(|| FoldFn::ordinal(f).map(|o| FOLD_PUSHES[o]))
    }

    /// **Zero, for every R2IL opcode and every fold-band opcode alike** —
    /// and that is a semantic statement, not a stub.
    ///
    /// A body reference is loco's mechanism for a call that branches to
    /// another *function node*. R2IL's control flow branches to an
    /// **address**, which travels as an immediate operand, not as a nested
    /// function. `Branch`/`Call` are therefore leaf calls here: R2IL is a
    /// flat instruction stream that loco stores, never a call tree loco
    /// walks. Returning non-zero would make `Program::references_are_resolvable`
    /// chase operands that are addresses. The fold band's twelve opcodes
    /// reference no bodies either — they are terminals over an already-
    /// resolved population, never a call to another function node — so the
    /// same `0` covers both bands without a branch.
    fn domain_body_refs(&self, _f: FnIndex) -> u8 {
        0
    }

    /// The canonical mnemonic — [`R2ILFn::MNEMONICS`] for an R2IL byte,
    /// [`FOLD_MNEMONICS`] for a fold-band byte, `None` outside both.
    ///
    /// Additive fix, not new surface: before this override,
    /// `VocabularyTable::name()` was `None` for every one of the 82 R2IL
    /// opcodes even though [`R2ILFn::MNEMONICS`] already existed — the trait
    /// hook was simply never wired, the same shape of gap
    /// `domain_pushes_result` closed above.
    /// `domain_name_now_answers_for_the_r2il_band_too` pins the closed gap.
    fn domain_name(&self, f: FnIndex) -> Option<&'static str> {
        if let Some(o) = R2ILFn::ordinal(f) {
            Some(R2ILFn::MNEMONICS[o])
        } else {
            FoldFn::ordinal(f).map(|o| FOLD_MNEMONICS[o])
        }
    }
}

impl R2ILVocabulary {
    /// Whether `f` pushes a result; [`None`] when `f` is not an R2IL opcode.
    #[must_use]
    pub fn pushes_result(f: FnIndex) -> Option<bool> {
        R2ILFn::ordinal(f).map(|o| PUSHES[o])
    }
}

/// Inline word count for [`CallMask`]'s bitset.
///
/// Derived, not chosen: the widest [`LaneShape`] is `Pairs`, at
/// `180 = LaneShape::Pairs.calls_per_lane() * ogar_loco::CONTENT_SLOTS` call
/// slots (pinned by the
/// `masks_of_different_shapes_range_over_different_populations` test), and
/// `180.div_ceil(64) == 3`. `Triples` (120) and `Quads` (90) both need fewer
/// words and simply leave the high word(s) unused — always zero, since
/// [`CallMask::set`] only ever touches a word index `< len.div_ceil(64)`.
/// Three inline `u64` words therefore cover every shape with no allocation.
const MASK_WORDS: usize = 3;

/// A population of CALL INDICES within one body, as words.
///
/// The same law as every other mask plane here: reads like a selection,
/// executes as word ops, and leaves mask form only through the one named
/// materializer ([`CallMask::materialize_indices`]) — the lazy
/// [`CallMask::set_indices`] iterator is not a second exit, it stays inside
/// the same word-scan discipline and only turns into data when a caller
/// chooses to collect it. Call indices are shape-relative — a mask built
/// under `Pairs` indexes a different population than the same bits under
/// `Quads` — which is why [`project`] takes both and why [`CallMask::shape`]
/// is carried rather than inferred.
///
/// Backed by [`MASK_WORDS`] inline `u64` words rather than a `Box<[u64]>`:
/// every [`LaneShape`]'s call population fits in three words with room to
/// spare, so a boxed, heap-allocated slice would only be paying for an
/// allocation the largest shape never needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallMask {
    words: [u64; MASK_WORDS],
    len: u32,
    shape: LaneShape,
}

impl CallMask {
    /// An all-zero mask over every call slot `shape` admits.
    #[must_use]
    pub fn empty(shape: LaneShape) -> Self {
        let len = shape.calls_per_lane() * ogar_loco::CONTENT_SLOTS;
        debug_assert!(
            len.div_ceil(64) <= MASK_WORDS,
            "shape needs more than MASK_WORDS={MASK_WORDS} inline words"
        );
        Self {
            words: [0u64; MASK_WORDS],
            len: u32::try_from(len).unwrap_or(u32::MAX),
            shape,
        }
    }

    /// Every call slot set — the "no filter" reading.
    #[must_use]
    pub fn all(shape: LaneShape) -> Self {
        let mut m = Self::empty(shape);
        for i in 0..m.len {
            m.set(i);
        }
        m
    }

    /// The shape this mask's indices are relative to.
    #[must_use]
    pub fn shape(&self) -> LaneShape {
        self.shape
    }

    /// How many call slots the mask ranges over.
    #[must_use]
    pub fn len(&self) -> u32 {
        self.len
    }

    /// Whether the mask ranges over nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Set call index `i`. Out-of-range indices are ignored, never wrapped.
    pub fn set(&mut self, i: u32) {
        if i < self.len {
            self.words[(i / 64) as usize] |= 1u64 << (i % 64);
        }
    }

    /// Whether call index `i` is selected.
    #[must_use]
    pub fn contains(&self, i: u32) -> bool {
        i < self.len && (self.words[(i / 64) as usize] >> (i % 64)) & 1 == 1
    }

    /// How many calls are selected — one popcount sweep.
    #[must_use]
    pub fn count(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }

    /// The mask's **active** words, borrowed — the zero-copy way out of this
    /// carrier and into a plane-shaped consumer.
    ///
    /// Sliced to `len.div_ceil(64)`, NOT the full [`MASK_WORDS`] inline
    /// array: only `Pairs` (180 calls) fills three words; `Triples` (120) and
    /// `Quads` (90) fill two, and their third word is a phantom the
    /// population does not have. Handing it out would give a consumer a word
    /// count that disagrees with [`len`](Self::len) — and a consumer whose
    /// own complement clears the tail against a DIFFERENT word count than
    /// this mask's [`not`](Self::not) does would then disagree on the
    /// phantom's bits while agreeing on every real one. That is a
    /// wrong-answer shape, not a wasted-word shape.
    ///
    /// **Population identity travels with the words, never inside them.**
    /// The bits alone do not say what they index: a consumer needs
    /// [`len`](Self::len) (the bit count) and [`shape`](Self::shape) (the
    /// index space) to read them at all. In particular these indices are
    /// CALL SLOTS WITHIN ONE BODY — at most 180 — and are not row ordinals
    /// of any table; a row-population consumer sharing this carrier's
    /// Boolean algebra shares the algebra, not the index space.
    #[must_use]
    pub fn words(&self) -> &[u64] {
        &self.words[..(self.len.div_ceil(64) as usize)]
    }

    fn zip(&self, other: &Self, f: impl Fn(u64, u64) -> u64) -> Self {
        debug_assert_eq!(self.shape, other.shape, "masks of different shapes");
        Self {
            words: std::array::from_fn(|i| f(self.words[i], other.words[i])),
            len: self.len,
            shape: self.shape,
        }
    }

    /// Intersection.
    #[must_use]
    pub fn and(&self, other: &Self) -> Self {
        self.zip(other, |a, b| a & b)
    }
    /// Union.
    #[must_use]
    pub fn or(&self, other: &Self) -> Self {
        self.zip(other, |a, b| a | b)
    }
    /// Symmetric difference.
    #[must_use]
    pub fn xor(&self, other: &Self) -> Self {
        self.zip(other, |a, b| a ^ b)
    }
    /// `self` minus `other`.
    #[must_use]
    pub fn and_not(&self, other: &Self) -> Self {
        self.zip(other, |a, b| a & !b)
    }

    /// Complement WITHIN the shape's call count.
    ///
    /// Every bit at or past `len` is cleared, word by word — not just in the
    /// physically-last of [`MASK_WORDS`] inline words. With fixed inline
    /// storage, a shape narrower than `Pairs` (e.g. `Triples`, `Quads`) never
    /// touches the high word(s) at all, so a plain `!w` over every word would
    /// flip those always-zero, never-meant-to-be-addressed words to all-ones
    /// — phantom call slots the body does not have, inflating every
    /// downstream count and projection. Clearing per-word by each word's own
    /// bit range (fully out of range → zeroed; straddling `len` → masked to
    /// its in-range bits; fully in range → left untouched) is what keeps this
    /// correct for every shape, not only the widest one.
    #[must_use]
    pub fn not(&self) -> Self {
        let mut words: [u64; MASK_WORDS] = std::array::from_fn(|i| !self.words[i]);
        for (word_idx, w) in words.iter_mut().enumerate() {
            let word_start = (word_idx as u32) * 64;
            if word_start >= self.len {
                *w = 0;
            } else if word_start + 64 > self.len {
                let bits_in_word = self.len - word_start;
                *w &= (1u64 << bits_in_word) - 1;
            }
        }
        Self {
            words,
            len: self.len,
            shape: self.shape,
        }
    }

    /// Set bit indices, ascending, as a lazy iterator.
    ///
    /// One `trailing_zeros` + `w &= w - 1` per set bit, word by word — O(3 +
    /// popcount) rather than [`materialize_indices`](Self::materialize_indices)'s
    /// O(len) per-index `contains` scan. This is the lazy sibling, not a
    /// second exit from mask form: nothing is collected until a caller
    /// chooses to. `materialize_indices` is built on top of it.
    ///
    /// The trailing `take_while` is a defensive bound, not a hot path: by
    /// construction no bit is ever set at an index `>= len` (`set` guards it,
    /// and `not` now clears per-word instead of assuming the tail lives in
    /// the last of [`MASK_WORDS`] words), so this only ever matters if that
    /// invariant is ever broken elsewhere.
    //
    // No `#[must_use]`: `impl Iterator` already carries it, and stacking a
    // second bare one is `clippy::double_must_use` (CI runs `-D warnings`).
    pub fn set_indices(&self) -> impl Iterator<Item = u32> + '_ {
        let len = self.len;
        self.words
            .iter()
            .enumerate()
            .flat_map(|(word_idx, &word)| {
                let base = (word_idx as u32) * 64;
                let mut w = word;
                std::iter::from_fn(move || {
                    if w == 0 {
                        None
                    } else {
                        let bit = w.trailing_zeros();
                        w &= w - 1;
                        Some(base + bit)
                    }
                })
            })
            .take_while(move |&i| i < len)
    }

    /// **The named materializer** — call indices out, ascending. O(n), and
    /// the only *eager* exit from mask form (see [`Self::set_indices`] for
    /// the lazy view this is built on).
    #[must_use]
    pub fn materialize_indices(&self) -> Vec<u32> {
        self.set_indices().collect()
    }
}

/// The **masked lane projection**: re-read `slab` under `shape`, yielding
/// only the calls `mask` selects.
///
/// Lazy by construction — nothing outside the mask is decoded, and no
/// intermediate collection is built. The projection is a LENS: `slab` is
/// borrowed unchanged, so projecting the same bytes under two shapes is two
/// reads, never a rewrite.
///
/// `mask.shape()` must equal `shape`; a mismatch is a programming error
/// (`debug_assert`) because the indices would silently address a different
/// population — the failure would look like wrong data, not like a bug.
pub fn project<'a>(
    slab: &'a [u8; VALUE_SLAB_LEN],
    shape: LaneShape,
    mask: &'a CallMask,
) -> impl Iterator<Item = (u32, Call)> + 'a {
    debug_assert_eq!(mask.shape(), shape, "mask indices are shape-relative");
    (0..mask.len())
        .filter(move |&i| mask.contains(i))
        .map(move |i| (i, call_in_slab(slab, shape, i as usize)))
}

/// Project only the calls whose function index this vocabulary owns.
///
/// The composition `project` + "is it R2IL" that a caller would otherwise
/// write by hand, offered once so the `R2IL_BASE`/`R2IL_OPS` arithmetic has
/// exactly one site.
pub fn project_r2il<'a>(
    slab: &'a [u8; VALUE_SLAB_LEN],
    shape: LaneShape,
    mask: &'a CallMask,
) -> impl Iterator<Item = (u32, Call)> + 'a {
    project(slab, shape, mask).filter(|(_, c)| R2ILFn::ordinal(c.function).is_some())
}

/// A mask of every call slot in `slab` whose function is an R2IL opcode,
/// under `shape` — the selection half, so a caller can intersect it with
/// another mask before paying for any projection.
#[must_use]
pub fn r2il_mask(slab: &[u8; VALUE_SLAB_LEN], shape: LaneShape) -> CallMask {
    let mut m = CallMask::empty(shape);
    for i in 0..m.len() {
        if R2ILFn::ordinal(call_in_slab(slab, shape, i as usize).function).is_some() {
            m.set(i);
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The guard that would have caught the first draft.**
    ///
    /// The arity table was originally written from memory: 77 entries against
    /// an enum of 82, five variants missing and nine invented. This test reads
    /// `r2il/src/opcode.rs` and compares the variant list — by name, in
    /// declaration order — against [`R2ILFn::MNEMONICS`].
    ///
    /// It parses the source rather than depending on the crate deliberately:
    /// the coupling this crate accepts is one of NAMES, and a `use r2il::…`
    /// would drag the `Varnode`/`SpaceId` object graph into every consumer
    /// (see the module docs). Skips when the sibling checkout is absent, so a
    /// standalone clone still builds green — and asserts the parse found a
    /// plausible enum before comparing, so a moved file degrades to a loud
    /// skip instead of a silent pass.
    #[test]
    fn the_table_matches_r2il_s_own_enum() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../r2sleigh/crates/r2il/src/opcode.rs"
        );
        let Ok(src) = std::fs::read_to_string(path) else {
            eprintln!("skip: r2sleigh sibling checkout absent at {path}");
            return;
        };
        let body = src
            .split_once("pub enum R2ILOp {")
            .expect(
                "r2il no longer declares `pub enum R2ILOp` — the parse, not the table, is stale",
            )
            .1;
        let body = body.split("\n}\n").next().unwrap();

        // Variant heads: four-space indent, capitalised, followed by `{`, `(`
        // or `,`. Doc comments and nested field blocks are deeper-indented.
        let mut found: Vec<&str> = Vec::new();
        for line in body.lines() {
            let Some(rest) = line.strip_prefix("    ") else {
                continue;
            };
            if rest.starts_with(char::is_whitespace)
                || rest.starts_with("//")
                || rest.starts_with('#')
            {
                continue;
            }
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            if name.is_empty() || !name.starts_with(|c: char| c.is_ascii_uppercase()) {
                continue;
            }
            let tail = rest[name.len()..].trim_start();
            if tail.starts_with('{') || tail.starts_with('(') || tail.starts_with(',') {
                if let Some(pos) = body[..].find(&name) {
                    let _ = pos;
                }
                if !found.contains(&&rest[..name.len()]) {
                    found.push(&rest[..name.len()]);
                }
            }
        }
        assert!(
            found.len() > 40,
            "parsed only {} variants — the source shape changed and this test \
             would otherwise pass vacuously",
            found.len()
        );
        assert_eq!(
            found.len(),
            R2IL_OPS,
            "r2il declares {} opcodes, the table holds {R2IL_OPS}",
            found.len()
        );
        assert_eq!(
            found,
            R2ILFn::MNEMONICS,
            "the table drifted from r2il's declaration order"
        );
    }

    #[test]
    fn every_opcode_sits_above_the_domain_floor_and_below_the_next_domain() {
        assert_eq!(R2IL_BASE, ogar_loco::DOMAIN_FLOOR);
        let last = R2ILFn::from_ordinal(R2IL_OPS - 1).unwrap();
        assert!(
            last.0.0 as usize <= 0xFF,
            "82 opcodes from 0x90 must fit under 0xFF"
        );
        // …and one past the end is not ours.
        assert!(R2ILFn::from_ordinal(R2IL_OPS).is_none());
        assert!(R2ILFn::ordinal(FnIndex(last.0.0 + 1)).is_none());
    }

    /// A vocabulary must not be able to answer for the shared core. Loco
    /// composes core bytes from the core itself, but a vocabulary that
    /// *claimed* them would still be a bug at every direct call site.
    #[test]
    fn the_shared_core_is_never_claimed() {
        let v = R2ILVocabulary;
        for b in [0x00u8, 0x40, ogar_loco::DOMAIN_FLOOR - 1] {
            assert!(R2ILFn::ordinal(FnIndex(b)).is_none(), "byte {b:#04x}");
            assert!(v.domain_stack_arity(FnIndex(b)).is_none(), "byte {b:#04x}");
        }
    }

    /// The pushes column is WIRED into the segmentation seam — an R2IL body
    /// segments into statements. The probe that found the gap: a mixed
    /// core+R2IL body refused with `Uncovered(IntAdd)` because the crate
    /// shipped [`PUSHES`] but never answered `domain_pushes_result`.
    #[test]
    fn r2il_bodies_are_segmentable() {
        use ogar_loco::{Call, FunctionBody, LaneShape, statement_bounds};
        let v = ogar_loco::vocabulary::conformance::validate(R2ILVocabulary).unwrap();
        let int_add = R2ILFn::from_ordinal(
            R2ILFn::MNEMONICS
                .iter()
                .position(|m| *m == "IntAdd")
                .unwrap(),
        )
        .unwrap()
        .0;
        let store = R2ILFn::from_ordinal(
            R2ILFn::MNEMONICS
                .iter()
                .position(|m| *m == "Store")
                .unwrap(),
        )
        .unwrap()
        .0;
        // NUMBER, NUMBER, IntAdd (pushes), NUMBER, Store (arity 2, no push).
        let body = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(ogar_loco::FnIndex::NUMBER, 5),
                Call::with_value(ogar_loco::FnIndex::NUMBER, 3),
                Call::new(int_add),
                Call::with_value(ogar_loco::FnIndex::NUMBER, 9),
                Call::new(store),
            ],
        )
        .unwrap();
        let bounds = statement_bounds(&v, &body).expect("R2IL body must segment");
        assert_eq!(
            bounds.len(),
            1,
            "one Store statement spanning all five calls"
        );
        assert_eq!(bounds[0].call_count, 5);
        // …and the silence twin: a variadic op still refuses (arity None
        // short-circuits before pushes is ever consulted).
        let call_other = R2ILFn::from_ordinal(
            R2ILFn::MNEMONICS
                .iter()
                .position(|m| *m == "CallOther")
                .unwrap(),
        )
        .unwrap()
        .0;
        let bad = FunctionBody::from_calls(LaneShape::Pairs, &[Call::new(call_other)]).unwrap();
        assert!(
            statement_bounds(&v, &bad).is_err(),
            "variadic stays refused"
        );
    }

    /// Variadic opcodes are REFUSED, not reported as nullary.
    #[test]
    fn variadic_opcodes_are_refused_rather_than_called_nullary() {
        let v = R2ILVocabulary;
        for name in ["CallOther", "Multiequal"] {
            let o = R2ILFn::MNEMONICS.iter().position(|m| *m == name).unwrap();
            let f = R2ILFn::from_ordinal(o).unwrap().0;
            assert_eq!(
                v.domain_stack_arity(f),
                None,
                "{name} carries Vec<Varnode>; a 0 here would read as nullary"
            );
        }
        // …and a fixed-arity neighbour still answers, so `None` is not blanket.
        let add = R2ILFn::MNEMONICS
            .iter()
            .position(|m| *m == "IntAdd")
            .unwrap();
        assert_eq!(
            v.domain_stack_arity(R2ILFn::from_ordinal(add).unwrap().0),
            Some(2)
        );
    }

    /// R2IL control flow branches to an ADDRESS, never to another function
    /// node — so no opcode declares a body reference, and `Program`'s
    /// reference walk never chases an operand that is an address.
    #[test]
    fn no_opcode_declares_a_body_reference() {
        let v = R2ILVocabulary;
        for o in 0..R2IL_OPS {
            let f = R2ILFn::from_ordinal(o).unwrap().0;
            assert_eq!(v.domain_body_refs(f), 0, "{}", R2ILFn::MNEMONICS[o]);
            assert!(!v.branches(f), "{}", R2ILFn::MNEMONICS[o]);
        }
    }

    fn slab_with(shape: LaneShape, calls: &[(usize, u8)]) -> [u8; VALUE_SLAB_LEN] {
        let mut slab = [0u8; VALUE_SLAB_LEN];
        let per = shape.bytes_per_call();
        for &(i, f) in calls {
            let lane = i / shape.calls_per_lane();
            let within = i % shape.calls_per_lane();
            let base = lane * ogar_loco::SLOT_STRIDE + ogar_loco::CLASSID_BYTES + within * per;
            slab[base] = f;
        }
        slab
    }

    /// The reshuffle: ONE slab, TWO shapes, two different call streams from
    /// the same unchanged bytes. If the projection ever normalised or rewrote,
    /// these would agree.
    #[test]
    fn the_same_bytes_project_differently_under_two_shapes() {
        let f = R2ILFn::from_ordinal(0).unwrap().0.0; // Copy
        let slab = slab_with(LaneShape::Pairs, &[(0, f), (3, f)]);

        let pairs: Vec<u32> = r2il_mask(&slab, LaneShape::Pairs).materialize_indices();
        let quads: Vec<u32> = r2il_mask(&slab, LaneShape::Quads).materialize_indices();
        assert_eq!(pairs, vec![0, 3], "written under Pairs, read under Pairs");
        assert_ne!(
            pairs, quads,
            "a re-read under another carving must land on different call \
             indices — equal streams would mean the shape is not a lens"
        );
        // The bytes themselves never moved.
        let again = r2il_mask(&slab, LaneShape::Pairs).materialize_indices();
        assert_eq!(pairs, again, "projection mutated the slab");
    }

    /// The mask actually filters: nothing outside it is yielded, and the
    /// paired half — a full mask yields everything the shape admits.
    #[test]
    fn the_projection_yields_exactly_the_masked_calls() {
        let f = R2ILFn::from_ordinal(0).unwrap().0.0;
        let slab = slab_with(LaneShape::Pairs, &[(0, f), (1, f), (2, f)]);

        let mut only_one = CallMask::empty(LaneShape::Pairs);
        only_one.set(1);
        let got: Vec<u32> = project(&slab, LaneShape::Pairs, &only_one)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(got, vec![1], "the mask did not filter");

        let all = r2il_mask(&slab, LaneShape::Pairs);
        let got_all: Vec<u32> = project_r2il(&slab, LaneShape::Pairs, &all)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(got_all, vec![0, 1, 2], "a full mask must yield every match");
    }

    /// The complement stays inside the shape's call count — a `not()` that
    /// keeps the tail word's phantom bits invents call slots the body has not
    /// got, and every count and projection downstream inflates silently.
    #[test]
    fn a_complement_never_invents_call_slots() {
        for shape in LaneShape::ALL {
            let mut m = CallMask::empty(shape);
            m.set(0);
            let un = m.not();
            assert_eq!(
                un.count(),
                m.len() - 1,
                "{shape:?}: complement must be len-1, more means phantom bits"
            );
            assert!(!un.contains(m.len()), "{shape:?}: bit past len is set");
            assert_eq!(m.and(&un).count(), 0, "{shape:?}: x AND !x must be empty");
        }
    }

    /// Mask indices are shape-RELATIVE, which is why the shape is carried.
    #[test]
    fn masks_of_different_shapes_range_over_different_populations() {
        assert_eq!(CallMask::all(LaneShape::Pairs).count(), 180);
        assert_eq!(CallMask::all(LaneShape::Triples).count(), 120);
        assert_eq!(CallMask::all(LaneShape::Quads).count(), 90);
        assert_ne!(
            CallMask::empty(LaneShape::Pairs).len(),
            CallMask::empty(LaneShape::Quads).len()
        );
    }

    /// A body of pure shared-core bytes contains no R2IL, and the mask says
    /// so — the can-stay-silent half of `r2il_mask`.
    #[test]
    fn a_body_with_no_r2il_calls_masks_to_nothing() {
        let core = ogar_loco::DOMAIN_FLOOR - 1;
        let slab = slab_with(LaneShape::Pairs, &[(0, core), (5, core)]);
        assert_eq!(r2il_mask(&slab, LaneShape::Pairs).count(), 0);
        // …and the same slab with one R2IL byte is NOT silent.
        let mixed = slab_with(
            LaneShape::Pairs,
            &[(0, core), (5, R2ILFn::from_ordinal(0).unwrap().0.0)],
        );
        assert_eq!(r2il_mask(&mixed, LaneShape::Pairs).count(), 1);
    }

    /// `CallMask` is exactly its three inline fields, padded to its own
    /// alignment — no `Box<[u64]>` (pointer + length) hiding inside it. The
    /// expected size is computed from the field layout itself rather than a
    /// hardcoded byte count, so this stays correct however `LaneShape` is
    /// represented.
    #[test]
    fn call_mask_stores_its_words_inline_not_boxed() {
        let words_bytes = std::mem::size_of::<[u64; MASK_WORDS]>();
        let len_bytes = std::mem::size_of::<u32>();
        let shape_bytes = std::mem::size_of::<LaneShape>();
        let raw_sum = words_bytes + len_bytes + shape_bytes;
        let align = std::mem::align_of::<CallMask>();
        let expected = raw_sum.div_ceil(align) * align;
        assert_eq!(
            std::mem::size_of::<CallMask>(),
            expected,
            "CallMask must be exactly its inline fields (padded to its own \
             alignment) — a Box<[u64]> backing would additionally cost a \
             pointer plus a length"
        );
        // A CallMask is trivially Copy-able data now — no field left behind
        // that would make Copy unsound.
        let m = CallMask::empty(LaneShape::Pairs);
        let copy = m;
        assert_eq!(m, copy, "CallMask must remain usable after being Copy'd");
    }

    /// [`CallMask::set_indices`] yields exactly the set-bit indices,
    /// ascending — for an empty mask (silence), a single-bit mask, and a
    /// full mask (every index the shape admits), across every `LaneShape`.
    #[test]
    fn set_indices_yields_exactly_the_set_bit_indices() {
        for shape in LaneShape::ALL {
            // Empty: yields nothing.
            let empty = CallMask::empty(shape);
            assert_eq!(
                empty.set_indices().collect::<Vec<u32>>(),
                Vec::<u32>::new(),
                "{shape:?}: empty mask must iterate to nothing"
            );

            // A handful of scattered bits, including one that straddles the
            // word boundary at index 64 and (for Pairs) one near the tail.
            let mut m = CallMask::empty(shape);
            let candidates = [0u32, 1, 63, 64, 65, m.len().saturating_sub(1)];
            let mut expected: Vec<u32> = candidates
                .into_iter()
                .filter(|&i| i < m.len())
                .collect::<std::collections::BTreeSet<u32>>()
                .into_iter()
                .collect();
            expected.sort_unstable();
            for &i in &expected {
                m.set(i);
            }
            assert_eq!(
                m.set_indices().collect::<Vec<u32>>(),
                expected,
                "{shape:?}: scattered bits must iterate ascending, exactly the set ones"
            );
            assert_eq!(
                m.materialize_indices(),
                expected,
                "{shape:?}: materialize_indices must agree with set_indices"
            );

            // Full: every index the shape admits, ascending, none skipped.
            let full = CallMask::all(shape);
            let got_full: Vec<u32> = full.set_indices().collect();
            let want_full: Vec<u32> = (0..full.len()).collect();
            assert_eq!(
                got_full, want_full,
                "{shape:?}: full mask must yield 0..len"
            );
        }
    }

    /// **The accessor is sliced to the POPULATION, not to the carrier.**
    ///
    /// `MASK_WORDS` is 3 because `Pairs` (180 calls) needs three words. The
    /// other two shapes do not: `Triples` is 120 calls and `Quads` is 90,
    /// both two words, and their third inline word is a phantom the body
    /// never had. Two-sided on purpose — the assertion that a narrow shape
    /// yields FEWER words than the carrier holds is what fails if
    /// `words()` is ever changed to hand out `&self.words` whole.
    #[test]
    fn words_is_sliced_to_the_population_not_the_carrier() {
        assert_eq!(MASK_WORDS, 3, "carrier width changed; re-derive the table");

        for (shape, want_words, want_len) in [
            (LaneShape::Pairs, 3usize, 180u32),
            (LaneShape::Triples, 2, 120),
            (LaneShape::Quads, 2, 90),
        ] {
            let m = CallMask::all(shape);
            assert_eq!(m.len(), want_len, "{shape:?}: population");
            assert_eq!(
                m.words().len(),
                want_words,
                "{shape:?}: words() must span len.div_ceil(64), not MASK_WORDS"
            );
            assert_eq!(
                m.words().len(),
                (m.len().div_ceil(64)) as usize,
                "{shape:?}: slice width must be derived from len, never hardcoded"
            );
        }

        // Anti-vacuity: at least one shape must be NARROWER than the carrier,
        // or the two readings agree by accident and this test proves nothing.
        assert!(
            CallMask::all(LaneShape::Quads).words().len() < MASK_WORDS,
            "no shape is narrower than the carrier: the slice cannot be falsified"
        );
    }

    /// `words()` is the same mask `contains` reads — bit for bit, including
    /// the straddling word and the bits just under `len`.
    ///
    /// An AGREEMENT test, not a slicing one: it reads only indices below
    /// `len`, which live in the same words under either slice width, so it
    /// is green whether or not the carrier is over-exposed. It falsifies a
    /// wrong word index or a wrong bit order — nothing about the width.
    #[test]
    fn words_agrees_with_contains_bit_for_bit() {
        for shape in [LaneShape::Pairs, LaneShape::Triples, LaneShape::Quads] {
            let mut m = CallMask::empty(shape);
            // Seeded scatter plus every boundary this shape can express: the
            // word edges (63/64, 127/128) and its own last two slots.
            let mut seeded = 0u32;
            for i in 0..m.len() {
                if i % 7 == 0 || i % 13 == 3 {
                    m.set(i);
                    seeded += 1;
                }
            }
            for edge in [
                0,
                1,
                62,
                63,
                64,
                65,
                126,
                127,
                128,
                m.len() - 2,
                m.len() - 1,
            ] {
                if edge < m.len() {
                    m.set(edge);
                }
            }
            assert!(seeded > 0, "{shape:?}: fixture set no bits");

            let w = m.words();
            for i in 0..m.len() {
                let from_words = (w[(i / 64) as usize] >> (i % 64)) & 1 == 1;
                assert_eq!(
                    m.contains(i),
                    from_words,
                    "{shape:?}: bit {i} disagrees between contains() and words()"
                );
            }
            assert_eq!(
                w.iter().map(|x| x.count_ones()).sum::<u32>(),
                m.count(),
                "{shape:?}: popcount over the slice must equal count()"
            );
        }
    }

    // ── The fold extension band ─────────────────────────────────────────

    /// FAILS IF: `FOLD_BASE` drifts from `R2IL_BASE + R2IL_OPS` — either the
    /// arithmetic identity or the literal byte it must equal today.
    #[test]
    fn the_fold_band_starts_exactly_where_r2il_ends() {
        assert_eq!(FOLD_BASE, R2IL_BASE + R2IL_OPS as u8);
        assert_eq!(FOLD_BASE, 0xE2);
    }

    /// FAILS IF: the fold band's last byte overflows `u8`, or drifts from
    /// its known last opcode `BLEND` at `0xED`.
    #[test]
    fn the_fold_band_fits_under_the_byte_ceiling() {
        let last = FoldFn::from_ordinal(FOLD_OPS - 1).unwrap();
        assert!(last.0.0 as usize <= 0xFF, "fold band must fit under 0xFF");
        assert_eq!(last.0.0, 0xED, "BLEND must sit at the last fold byte");
    }

    /// FAILS IF: any of the three fold-band arrays drifts in length from the
    /// other two, or from the independently pinned `FOLD_OPS == 12`.
    #[test]
    fn the_three_fold_arrays_agree_on_length() {
        assert_eq!(FOLD_OPS, 12);
        assert_eq!(FOLD_MNEMONICS.len(), FOLD_OPS);
        assert_eq!(FOLD_ARITY.len(), FOLD_OPS);
        assert_eq!(FOLD_PUSHES.len(), FOLD_OPS);
    }

    /// FAILS IF: any fold-band byte is missing an arity, a name, or has an
    /// empty name — a gap in the band that a caller could silently fall
    /// through.
    #[test]
    fn every_fold_byte_has_an_arity_a_pushes_and_a_name() {
        let v = R2ILVocabulary;
        let last = FOLD_BASE + (FOLD_OPS as u8 - 1);
        for b in FOLD_BASE..=last {
            let f = FnIndex(b);
            assert!(
                v.domain_stack_arity(f).is_some(),
                "byte {b:#04x} has no arity"
            );
            let name = v.domain_name(f);
            assert!(name.is_some(), "byte {b:#04x} has no name");
            assert!(!name.unwrap().is_empty(), "byte {b:#04x} has an empty name");
        }
    }

    /// FAILS IF: a fold-band byte is also claimed by the R2IL band, or vice
    /// versa — both directions, since a one-sided check could not catch a
    /// band that silently grew into its neighbour.
    #[test]
    fn no_fold_byte_collides_with_the_r2il_band() {
        for o in 0..FOLD_OPS {
            let f = FoldFn::from_ordinal(o).unwrap().0;
            assert!(
                R2ILFn::ordinal(f).is_none(),
                "fold byte {:#04x} is also claimed by the R2IL band",
                f.0
            );
        }
        for o in 0..R2IL_OPS {
            let f = R2ILFn::from_ordinal(o).unwrap().0;
            assert!(
                FoldFn::ordinal(f).is_none(),
                "R2IL byte {:#04x} is also claimed by the fold band",
                f.0
            );
        }
    }

    /// FAILS IF: adding the fold band changed a single R2IL byte's answer —
    /// the no-regression pin for the whole extension.
    #[test]
    fn the_r2il_band_answers_exactly_as_before() {
        let v = R2ILVocabulary;
        for (o, (&arity, &name)) in ARITY.iter().zip(R2ILFn::MNEMONICS.iter()).enumerate() {
            let f = R2ILFn::from_ordinal(o).unwrap().0;
            assert_eq!(v.domain_stack_arity(f), arity, "{name}");
            assert_eq!(
                v.domain_pushes_result(f),
                R2ILVocabulary::pushes_result(f),
                "{name}"
            );
        }
    }

    /// FAILS IF: `R2ILVocabulary` fails `ogar_loco`'s own conformance check
    /// once the fold band is present — the fold band must not break the
    /// shared-core-is-unforgeable / shape-consistency guarantees every
    /// vocabulary must pass.
    #[test]
    fn the_vocabulary_still_conforms_with_the_fold_band_present() {
        ogar_loco::vocabulary::conformance::validate(R2ILVocabulary)
            .expect("R2ILVocabulary must still conform with the fold band present");
    }

    /// FAILS IF: the same composed table cannot plug into a
    /// [`ogar_loco::VocabularyRegistry`] under two distinct concept ids with
    /// equal results, or a repeated concept id is not refused — the "one
    /// table, two classids" claim, proven both ways.
    #[test]
    fn one_table_plugs_under_both_concept_ids() {
        let checked = ogar_loco::vocabulary::conformance::validate(R2ILVocabulary).unwrap();
        let mut registry = ogar_loco::VocabularyRegistry::new();

        registry
            .plug(CONCEPT_R2IL_MACHINE, &checked)
            .expect("machine-reading plug must succeed");
        registry
            .plug(CONCEPT_R2IL_FOLD, &checked)
            .expect("fold-reading plug must succeed");

        let machine = registry.resolve_concept(CONCEPT_R2IL_MACHINE);
        let fold = registry.resolve_concept(CONCEPT_R2IL_FOLD);
        assert!(machine.is_some(), "machine concept id must resolve");
        assert!(fold.is_some(), "fold concept id must resolve");
        assert_eq!(
            machine, fold,
            "one table plugged under two ids must resolve identically"
        );

        // …and the anti-vacuity half: a repeat plug under an already-used id
        // is refused loudly, never silently accepted.
        assert_eq!(
            registry.plug(CONCEPT_R2IL_MACHINE, &checked),
            Err(ogar_loco::RegistryError::ConceptTaken {
                concept_id: CONCEPT_R2IL_MACHINE
            }),
            "a second plug under a taken concept id must be refused"
        );
    }

    /// FAILS IF: `domain_name` still answers `None` for the R2IL band — the
    /// additive fix must be live, not merely present for the fold band.
    #[test]
    fn domain_name_now_answers_for_the_r2il_band_too() {
        let v = R2ILVocabulary;
        let load = R2ILFn::from_ordinal(1).unwrap().0; // "Load"
        let int_equal = R2ILFn::from_ordinal(27).unwrap().0; // "IntEqual"
        assert_eq!(v.domain_name(load), Some("Load"));
        assert_eq!(v.domain_name(int_equal), Some("IntEqual"));
    }

    /// FAILS IF: the fold-band arrays are reordered — this is the readable
    /// pin, spelling out every (name, arity, pushes) triple by hand so a
    /// silent reorder of the arrays fails HERE, not only in a byte-count
    /// check that cannot see order.
    #[test]
    fn the_fold_arities_are_the_ones_the_fold_dialect_needs() {
        let expected: [(&str, Option<u8>, bool); FOLD_OPS] = [
            ("VIA", Some(2), true),
            ("RANGE", Some(0), true),
            ("SUM", Some(2), true),
            ("MIN", Some(2), true),
            ("MAX", Some(2), true),
            ("GROUP_SUM", Some(3), false),
            ("KEY_RUNS", Some(2), true),
            ("ANY", Some(1), true),
            ("ALL", Some(1), true),
            ("KEEP", Some(1), false),
            ("SCATTER_OR", Some(2), false),
            ("BLEND", Some(3), false),
        ];
        for (o, (name, arity, pushes)) in expected.iter().enumerate() {
            assert_eq!(FOLD_MNEMONICS[o], *name, "ordinal {o} name");
            assert_eq!(FOLD_ARITY[o], *arity, "ordinal {o} ({name}) arity");
            assert_eq!(FOLD_PUSHES[o], *pushes, "ordinal {o} ({name}) pushes");
        }
    }
}
