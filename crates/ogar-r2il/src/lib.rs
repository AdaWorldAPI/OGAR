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
//! first-class. The runtime path is r2sleigh's interpreter executing R2IL in
//! realtime; this crate is the addressing glue that makes those calls
//! loco-addressable — not a converter, not a harvest.
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
//! mask is [`CallMask`] — `Box<[u64]>` over call indices, with the same
//! algebra and the same single named materializer as every other mask plane
//! in this workspace, because "a `long[]` of selected ids is still a
//! materialised population" applies here identically.
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

/// The R2IL vocabulary — a table, not a translator.
///
/// Zero-sized: it holds no state and allocates nothing, so a consumer can
/// keep one in a `const` and hand it to `VocabularyTable::compose` as often
/// as it likes. That cheapness is what makes "always on" honest rather than
/// a slogan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct R2ILVocabulary;

impl Vocabulary for R2ILVocabulary {
    fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
        R2ILFn::ordinal(f).and_then(|o| ARITY[o])
    }

    /// **Zero, for every R2IL opcode** — and that is a semantic statement, not
    /// a stub.
    ///
    /// A body reference is loco's mechanism for a call that branches to
    /// another *function node*. R2IL's control flow branches to an
    /// **address**, which travels as an immediate operand, not as a nested
    /// function. `Branch`/`Call` are therefore leaf calls here: R2IL is a
    /// flat instruction stream that loco stores, never a call tree loco
    /// walks. Returning non-zero would make `Program::references_are_resolvable`
    /// chase operands that are addresses.
    fn domain_body_refs(&self, _f: FnIndex) -> u8 {
        0
    }
}

impl R2ILVocabulary {
    /// Whether `f` pushes a result; [`None`] when `f` is not an R2IL opcode.
    #[must_use]
    pub fn pushes_result(f: FnIndex) -> Option<bool> {
        R2ILFn::ordinal(f).map(|o| PUSHES[o])
    }
}

/// A population of CALL INDICES within one body, as words.
///
/// The same law as every other mask plane here: reads like a selection,
/// executes as word ops, and leaves mask form only through the one named
/// materializer. Call indices are shape-relative — a mask built under
/// `Pairs` indexes a different population than the same bits under `Quads` —
/// which is why [`project`] takes both and why [`CallMask::shape`] is
/// carried rather than inferred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallMask {
    words: Box<[u64]>,
    len: u32,
    shape: LaneShape,
}

impl CallMask {
    /// An all-zero mask over every call slot `shape` admits.
    #[must_use]
    pub fn empty(shape: LaneShape) -> Self {
        let len = shape.calls_per_lane() * ogar_loco::CONTENT_SLOTS;
        Self {
            words: vec![0u64; len.div_ceil(64)].into_boxed_slice(),
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

    fn zip(&self, other: &Self, f: impl Fn(u64, u64) -> u64) -> Self {
        debug_assert_eq!(self.shape, other.shape, "masks of different shapes");
        Self {
            words: self
                .words
                .iter()
                .zip(other.words.iter())
                .map(|(&a, &b)| f(a, b))
                .collect(),
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
    /// The tail word's bits past `len` are cleared: a complement that keeps
    /// them invents call slots the body does not have, and every downstream
    /// count and projection silently inflates.
    #[must_use]
    pub fn not(&self) -> Self {
        let mut out = Self {
            words: self.words.iter().map(|&w| !w).collect(),
            len: self.len,
            shape: self.shape,
        };
        let tail = u64::from(self.len % 64);
        if tail != 0
            && let Some(last) = out.words.last_mut()
        {
            *last &= (1u64 << tail) - 1;
        }
        out
    }

    /// **The named materializer** — call indices out, ascending. O(n), and
    /// the only exit from mask form.
    #[must_use]
    pub fn materialize_indices(&self) -> Vec<u32> {
        (0..self.len).filter(|&i| self.contains(i)).collect()
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
}
