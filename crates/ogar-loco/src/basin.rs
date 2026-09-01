//! The **shared basin codebook** — where an operand byte spends its SHARED
//! meaning, as opposed to the per-function meaning [`crate::pool`] gives it.
//!
//! # The half that was missing
//!
//! Two mechanisms in this crate each had one of the two properties a shared
//! operand table needs, and neither had both:
//!
//! | | index | shape | ownership |
//! |---|---|---|---|
//! | [`ConstantPool`](crate::pool::ConstantPool) | `u8` `1..=255` | `classid(4) + 12` V3 facet | **owned, per-function** |
//! | [`ValueCodebook`](crate::vocabulary::ValueCodebook) | `u8` id | **none** — advisory `{id, name}` | **shared, basin-scoped** |
//!
//! `ValueCodebook`'s own contract is that `part_of(subject, object)` in one
//! basin and in a sibling basin "can legally resolve their object byte against
//! two different codebooks; the call itself is agnostic to which" — but it
//! carries no table, so nothing could actually resolve. `ConstantPool` carries
//! exactly the right table and is owned by ONE function, so a vocabulary-wide
//! set of targets held there would be copied into every function that names it.
//!
//! This module is the join: the pool's facet arithmetic under the codebook's
//! ownership.
//!
//! # Why this is not the shared-mutable sink `pool` refused
//!
//! [`crate::pool`] rejects holding constants in the Inventory SoA because
//! *"Inventory is shared by every function, so a per-function pool living there
//! is a shared-mutable sink with N writers."* That objection is about
//! **mutability**, not sharing — and it is answered structurally here rather
//! than by convention:
//!
//! [`BasinCodebookBuilder`] is the only thing that can intern, and
//! [`seal`](BasinCodebookBuilder::seal) consumes it, yielding a
//! [`BasinCodebook`] with **no `&mut self` method at all**. One writer at
//! mint time; zero writers afterwards. A body resolving an operand byte
//! borrows `&BasinCodebook` and cannot reach a mutation it does not have.
//! Sharing an immutable table is not a sink.
//!
//! The pool's other two disciplines carry over unchanged, for the same
//! reasons: interning is **content-addressed** (one value referenced twice is
//! one entry), and an entry is never rewritten in place — a changed meaning is
//! a new entry the referent repoints to, so a local edit cannot change a
//! reading elsewhere. Because a sealed codebook cannot be edited at all, the
//! repointing happens one level up, at the mint that builds the next version.
//!
//! # Why this is a value table and not more opcodes
//!
//! The domain range is `0x90..=0xFF` — **112** function slots per vocabulary.
//! A palette of a few dozen predicates fits there (`ogar-ro` mints 22 from
//! `0x90`); a 144-entry relation vocabulary does not, and no amount of
//! carving makes 144 fit 112. So a large palette is not a set of functions: it
//! is ONE function whose operand byte indexes a codebook — one `FnIndex` for
//! the whole table instead of one per entry.
//!
//! # What the entry classid buys
//!
//! An entry is `classid(4) + 12` — the V3 content-blind register. The classid
//! names the READING of those 12 bytes, exactly as it does everywhere else:
//! `6×(u8:u8)`, `4×(u8:u8:u8)`, `3×(u8:u8:u8:u8)` or `12×u8` are the same
//! bytes under different classids. That is what makes an index into this table
//! different in kind from an enum ordinal: a flat ordinal has no metric —
//! nothing about the number 71 says it is near 72 — whereas a codebook whose
//! payload is a quantizer code can answer distance by table lookup. Compiling
//! meaning into a codebook is precisely minting the entry classid that says
//! how the 12 bytes read.
//!
//! This crate does not mint any such classid. As in [`crate::pool`], **the
//! classids are parameters**: the caller supplies them, because minting a
//! concept is an operator decision with a ledger entry.
//!
//! # Orchestration is agnostic; the thinking IR is a caller
//!
//! `ogar-loco` stays vocabulary-agnostic — it owns the call ABI, the shapes,
//! the pool and now this table, and knows nothing about what any byte means.
//! A thinking IR (`ogar-r2il`) is a **caller**: it plugs its vocabulary and
//! its codebooks in and is resolved through the same seams as any other
//! consumer. Nothing in this module knows it exists.

use crate::pool::{
    CONSTANT_BYTES, CONSTANTS_PER_NODE, Constant, ConstantPool, MAX_CONSTANTS, PoolError,
};
use crate::vocabulary::{ValueCodebook, Vocabulary};
use crate::{CLASSID_BYTES, CONTENT_SLOTS, Call, FnIndex, SLOT_STRIDE};

/// Why a codebook could not join a [`BasinCodebooks`] set.
///
/// Interning failures reuse [`PoolError`] rather than restating it: a value
/// too wide for a facet and a table at 255 entries are the same two failures
/// for the same two reasons, and a parallel enum would be a second name for
/// one contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BasinError {
    /// Two codebooks claimed the same [`ValueCodebook::id`] in one basin.
    /// Refused rather than shadowed: an operand byte must have exactly one
    /// table to resolve against.
    DuplicateCodebook {
        /// The contested id.
        id: u8,
    },
}

impl core::fmt::Display for BasinError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BasinError::DuplicateCodebook { id } => {
                write!(f, "codebook id {id} is already plugged into this basin")
            }
        }
    }
}

impl core::error::Error for BasinError {}

/// The one writer. Interns entries at mint time, then
/// [`seal`](Self::seal)s into a read-only [`BasinCodebook`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasinCodebookBuilder {
    declares: ValueCodebook,
    entries: Vec<Constant>,
}

impl BasinCodebookBuilder {
    /// A builder for the table `declares` names.
    #[must_use]
    pub fn new(declares: ValueCodebook) -> Self {
        Self {
            declares,
            entries: Vec::new(),
        }
    }

    /// How many entries are interned so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is interned yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Intern an entry, returning the index an operand byte carries.
    ///
    /// Content-addressed: the same `(classid, bytes)` twice is one entry and
    /// one index. Payload is zero-padded to the facet width.
    ///
    /// # Errors
    ///
    /// [`PoolError::TooWide`] past a facet; [`PoolError::Full`] at 255.
    pub fn intern(&mut self, classid: u32, value: &[u8]) -> Result<u8, PoolError> {
        if value.len() > CONSTANT_BYTES {
            return Err(PoolError::TooWide {
                needed: value.len(),
            });
        }
        let mut bytes = [0u8; CONSTANT_BYTES];
        bytes[..value.len()].copy_from_slice(value);
        let candidate = Constant { classid, bytes };

        if let Some(pos) = self.entries.iter().position(|c| *c == candidate) {
            // pos < len <= MAX_CONSTANTS, so +1 cannot leave u8.
            return Ok(u8::try_from(pos + 1).expect("interned index within u8"));
        }
        if self.entries.len() >= MAX_CONSTANTS {
            return Err(PoolError::Full);
        }
        self.entries.push(candidate);
        u8::try_from(self.entries.len()).map_err(|_| PoolError::Full)
    }

    /// Freeze into the shared, read-only table. Consumes the builder — after
    /// this there is no writer, which is what makes the sharing sound.
    #[must_use]
    pub fn seal(self) -> BasinCodebook {
        BasinCodebook {
            declares: self.declares,
            entries: self.entries,
        }
    }
}

/// A sealed, shared operand table for one basin.
///
/// Read-only by construction: it has no `&mut self` method, so every function
/// in the basin can hold `&BasinCodebook` and none can drift it. Build one
/// with [`BasinCodebookBuilder`].
///
/// Storage is the same V3 node shape [`ConstantPool`] uses — 30 facet slots of
/// `classid(4) + 12` per node — and the index arithmetic is literally the
/// pool's, called rather than copied ([`ConstantPool::locate`],
/// [`ConstantPool::slot_payload_offset`]). The pool's own docs say the defects
/// live in that arithmetic; a second implementation of it would be a second
/// place for them to live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasinCodebook {
    declares: ValueCodebook,
    entries: Vec<Constant>,
}

impl BasinCodebook {
    /// The [`ValueCodebook`] this table answers for — the id a call's
    /// [`Vocabulary::value_codebook`] names, and the name legends print.
    #[must_use]
    pub fn declares(&self) -> ValueCodebook {
        self.declares
    }

    /// The codebook id, `declares().id`.
    #[must_use]
    pub fn id(&self) -> u8 {
        self.declares.id
    }

    /// How many entries the table holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many V3 nodes this table occupies.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.entries.len().div_ceil(CONSTANTS_PER_NODE)
    }

    /// Resolve an operand byte to its entry.
    ///
    /// `0` is the zero-fallback and yields `None` — a zeroed operand means
    /// "no entry", never "entry zero". The whole monotonic zero ladder rests
    /// on this: an unwritten byte is *not consulted*, never a wrong meaning.
    #[must_use]
    pub fn resolve(&self, idx: u8) -> Option<&Constant> {
        if idx == 0 {
            return None;
        }
        self.entries.get(usize::from(idx) - 1)
    }

    /// Which node an index lives in, and which slot within it. Delegates to
    /// [`ConstantPool::locate`] — same layout, one implementation.
    #[must_use]
    pub fn locate(idx: u8) -> Option<(usize, usize)> {
        ConstantPool::locate(idx)
    }

    /// Write one node's value slab: classid + payload per occupied slot,
    /// zeroes elsewhere. Unoccupied slots are zeroed, not skipped — reserve,
    /// don't reclaim.
    #[must_use]
    pub fn write_node(&self, node_ordinal: usize) -> [u8; CONTENT_SLOTS * SLOT_STRIDE] {
        let mut slab = [0u8; CONTENT_SLOTS * SLOT_STRIDE];
        let base = node_ordinal * CONSTANTS_PER_NODE;
        for slot_j in 0..CONSTANTS_PER_NODE {
            let Some(c) = self.entries.get(base + slot_j) else {
                break;
            };
            let at = slot_j * SLOT_STRIDE;
            slab[at..at + CLASSID_BYTES].copy_from_slice(&c.classid.to_le_bytes());
            let p = ConstantPool::slot_payload_offset(slot_j);
            slab[p..p + CONSTANT_BYTES].copy_from_slice(&c.bytes);
        }
        slab
    }
}

/// The codebooks in scope for one basin — the set an operand byte is resolved
/// against, keyed by [`ValueCodebook::id`].
///
/// A basin is the classid prefix a body lives under. Two basins holding
/// different tables under the same id is the intended case, not a collision:
/// that IS the "same call, different codebook" property
/// [`ValueCodebook`] describes. A collision within ONE basin is refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BasinCodebooks {
    books: Vec<BasinCodebook>,
}

impl BasinCodebooks {
    /// An empty set.
    #[must_use]
    pub fn new() -> Self {
        Self { books: Vec::new() }
    }

    /// How many codebooks are in scope.
    #[must_use]
    pub fn len(&self) -> usize {
        self.books.len()
    }

    /// Whether no codebook is in scope.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.books.is_empty()
    }

    /// Add a sealed codebook to this basin.
    ///
    /// # Errors
    ///
    /// [`BasinError::DuplicateCodebook`] if the id is already claimed. Refused
    /// rather than shadowed — an operand byte must have exactly one table.
    pub fn plug(&mut self, book: BasinCodebook) -> Result<(), BasinError> {
        let id = book.id();
        if self.books.iter().any(|b| b.id() == id) {
            return Err(BasinError::DuplicateCodebook { id });
        }
        self.books.push(book);
        Ok(())
    }

    /// The codebook with this id, if any.
    #[must_use]
    pub fn get(&self, id: u8) -> Option<&BasinCodebook> {
        self.books.iter().find(|b| b.id() == id)
    }

    /// Resolve `idx` against the table `declared` names.
    #[must_use]
    pub fn resolve(&self, declared: ValueCodebook, idx: u8) -> Option<&Constant> {
        self.get(declared.id)?.resolve(idx)
    }

    /// Resolve one of a call's operand bytes end to end: ask the vocabulary
    /// which codebook the call's function declares, then resolve that byte
    /// against it.
    ///
    /// `None` when the function declares no codebook (its operands are
    /// literals or body references, not codebook indices), when no such table
    /// is in scope, or when the byte is the zero-fallback. This is the seam
    /// that makes [`Vocabulary::value_codebook`] load-bearing rather than
    /// advisory: before it, the declaration named a table nothing could reach.
    #[must_use]
    pub fn resolve_operand<V: Vocabulary>(
        &self,
        vocab: &V,
        call: &Call,
        which: usize,
    ) -> Option<&Constant> {
        let declared = vocab.value_codebook(call.function)?;
        let byte = *call.values.get(which)?;
        self.resolve(declared, byte)
    }

    /// Whether `f` names a codebook that is actually in scope here — the
    /// check a validator runs before trusting a body's operand bytes.
    #[must_use]
    pub fn covers<V: Vocabulary>(&self, vocab: &V, f: FnIndex) -> bool {
        vocab
            .value_codebook(f)
            .is_some_and(|d| self.get(d.id).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DOMAIN_FLOOR;

    /// Deliberately invalid classids, same posture as [`crate::pool::placeholder`]:
    /// a placeholder escaping into stored data must be loud, not plausible.
    const TEST_CLASSID: u32 = 0xDEAD_1001;
    const OTHER_CLASSID: u32 = 0xDEAD_1002;

    const VERBS: ValueCodebook = ValueCodebook {
        id: 3,
        name: "verb",
    };
    const TARGETS: ValueCodebook = ValueCodebook {
        id: 4,
        name: "relation_target",
    };

    /// A vocabulary whose one minted op declares the VERBS codebook, and a
    /// second that declares nothing — so the "declares no codebook" arm is
    /// exercised by a real function, not by an absent one.
    struct TestVocab;
    const ASSERT_EDGE: FnIndex = FnIndex(DOMAIN_FLOOR);
    const PLAIN_OP: FnIndex = FnIndex(DOMAIN_FLOOR + 1);

    impl Vocabulary for TestVocab {
        fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
            (f == ASSERT_EDGE || f == PLAIN_OP).then_some(2)
        }
        fn domain_body_refs(&self, _f: FnIndex) -> u8 {
            0
        }
        fn domain_value_codebook(&self, f: FnIndex) -> Option<ValueCodebook> {
            (f == ASSERT_EDGE).then_some(VERBS)
        }
    }

    fn book(declares: ValueCodebook, n: u8) -> BasinCodebook {
        let mut b = BasinCodebookBuilder::new(declares);
        for i in 0..n {
            b.intern(TEST_CLASSID, &[i]).expect("fits");
        }
        b.seal()
    }

    #[test]
    fn interning_is_content_addressed_and_distinct_values_are_distinct() {
        let mut b = BasinCodebookBuilder::new(VERBS);
        let first = b.intern(TEST_CLASSID, b"causes").unwrap();
        let again = b.intern(TEST_CLASSID, b"causes").unwrap();
        // The dedup half.
        assert_eq!(first, again, "same value must reuse its index");
        assert_eq!(b.len(), 1);
        // The anti-vacuity half: dedup must not be "everything is index 1".
        let other = b.intern(TEST_CLASSID, b"prevents").unwrap();
        assert_ne!(first, other, "distinct values must get distinct indices");
        // Same bytes under a DIFFERENT classid read differently, so they are a
        // different entry -- the classid is part of the identity, not a label.
        let reclassed = b.intern(OTHER_CLASSID, b"causes").unwrap();
        assert_ne!(first, reclassed, "classid participates in identity");
        assert_eq!(b.len(), 3);
    }

    #[test]
    fn zero_is_the_fallback_and_is_never_minted() {
        let sealed = book(VERBS, 4);
        assert!(sealed.resolve(0).is_none(), "0 means no entry");
        assert!(BasinCodebook::locate(0).is_none());
        // ...and the first real entry is 1, so index 0 is not merely unused
        // but unreachable by minting.
        let mut b = BasinCodebookBuilder::new(VERBS);
        assert_eq!(b.intern(TEST_CLASSID, b"x").unwrap(), 1);
    }

    #[test]
    fn a_full_table_refuses_and_a_too_wide_value_refuses() {
        let mut b = BasinCodebookBuilder::new(VERBS);
        for i in 0..MAX_CONSTANTS {
            // Distinct payloads, so dedup cannot mask the capacity limit.
            b.intern(TEST_CLASSID, &(i as u32).to_le_bytes())
                .expect("within capacity");
        }
        assert_eq!(b.len(), MAX_CONSTANTS);
        assert_eq!(
            b.intern(TEST_CLASSID, &9_999_u32.to_le_bytes()),
            Err(PoolError::Full)
        );
        // Refusal must leave the table UNCHANGED. Without the explicit
        // capacity check the entry is pushed first and only then rejected by
        // the u8 conversion -- same error, corrupted state. The error value
        // alone cannot tell those apart, so assert the length too.
        assert_eq!(
            b.len(),
            MAX_CONSTANTS,
            "a refused intern must not grow the table"
        );
        // A value already present still resolves at capacity -- Full is about
        // new entries, not about lookups.
        assert!(b.intern(TEST_CLASSID, &0_u32.to_le_bytes()).is_ok());

        let mut w = BasinCodebookBuilder::new(VERBS);
        assert_eq!(
            w.intern(TEST_CLASSID, &[0u8; CONSTANT_BYTES + 1]),
            Err(PoolError::TooWide {
                needed: CONSTANT_BYTES + 1
            })
        );
    }

    #[test]
    fn the_node_slab_puts_the_classid_before_the_payload() {
        let mut b = BasinCodebookBuilder::new(VERBS);
        b.intern(TEST_CLASSID, b"AB").unwrap(); // index 1 -> slot 0
        b.intern(OTHER_CLASSID, b"CD").unwrap(); // index 2 -> slot 1
        let slab = b.seal().write_node(0);

        assert_eq!(&slab[0..4], &TEST_CLASSID.to_le_bytes());
        assert_eq!(&slab[4..6], b"AB");
        assert_eq!(
            &slab[SLOT_STRIDE..SLOT_STRIDE + 4],
            &OTHER_CLASSID.to_le_bytes()
        );
        assert_eq!(&slab[SLOT_STRIDE + 4..SLOT_STRIDE + 6], b"CD");
        // Unoccupied slots are zeroed, not skipped.
        assert!(slab[2 * SLOT_STRIDE..].iter().all(|&x| x == 0));
    }

    #[test]
    fn entries_span_nodes_at_the_thirty_slot_boundary() {
        let sealed = book(VERBS, 31);
        assert_eq!(sealed.node_count(), 2);
        // Index 30 is the last slot of node 0; 31 is the first of node 1.
        assert_eq!(BasinCodebook::locate(30), Some((0, 29)));
        assert_eq!(BasinCodebook::locate(31), Some((1, 0)));
        // And node 1's slab really carries entry 31 at slot 0.
        assert_eq!(&sealed.write_node(1)[4..5], &[30u8]);
    }

    #[test]
    fn one_basin_refuses_a_duplicate_id_but_two_basins_may_disagree() {
        // Two tables in ONE basin, holding DIFFERENT bytes at the same index,
        // so resolution must key on the declared id. Building them with the
        // same payloads would let "return the first book" pass.
        let mut basin = BasinCodebooks::new();
        let mut v = BasinCodebookBuilder::new(VERBS);
        v.intern(TEST_CLASSID, b"causes").unwrap();
        basin.plug(v.seal()).unwrap();
        let mut t = BasinCodebookBuilder::new(TARGETS);
        t.intern(TEST_CLASSID, b"heart").unwrap();
        basin.plug(t.seal()).unwrap();
        assert_eq!(basin.len(), 2);

        // Index 1 in both tables, two different answers, same basin.
        assert_eq!(&basin.resolve(VERBS, 1).unwrap().bytes[..6], b"causes");
        assert_eq!(&basin.resolve(TARGETS, 1).unwrap().bytes[..5], b"heart");
        assert_ne!(
            basin.resolve(VERBS, 1).unwrap().bytes,
            basin.resolve(TARGETS, 1).unwrap().bytes,
            "resolution must key on the declared id, not on table order"
        );

        assert_eq!(
            basin.plug(book(VERBS, 1)),
            Err(BasinError::DuplicateCodebook { id: VERBS.id })
        );

        // The property ValueCodebook's contract promises: the SAME id in a
        // sibling basin is a different table, and the same operand byte
        // therefore resolves to different bytes. This is the point of the
        // whole module -- if it failed, the codebook would be global.
        let mut sibling = BasinCodebooks::new();
        let mut b = BasinCodebookBuilder::new(VERBS);
        b.intern(TEST_CLASSID, b"elsewhere").unwrap();
        sibling.plug(b.seal()).unwrap();

        let here = basin.resolve(VERBS, 1).unwrap();
        let there = sibling.resolve(VERBS, 1).unwrap();
        assert_ne!(
            here.bytes, there.bytes,
            "one byte, two basins, two readings"
        );
    }

    #[test]
    fn an_operand_resolves_end_to_end_through_the_vocabulary_declaration() {
        let mut basin = BasinCodebooks::new();
        let mut b = BasinCodebookBuilder::new(VERBS);
        let causes = b.intern(TEST_CLASSID, b"causes").unwrap();
        basin.plug(b.seal()).unwrap();

        let call = Call::with_values(ASSERT_EDGE, [causes, 0, 0]);
        let got = basin
            .resolve_operand(&TestVocab, &call, 0)
            .expect("declared codebook is in scope");
        assert_eq!(&got.bytes[..6], b"causes");
        assert!(basin.covers(&TestVocab, ASSERT_EDGE));

        // A function that declares NO codebook: its operands are literals, so
        // there is nothing to resolve even though the byte is a valid index.
        let plain = Call::with_values(PLAIN_OP, [causes, 0, 0]);
        assert!(basin.resolve_operand(&TestVocab, &plain, 0).is_none());
        assert!(!basin.covers(&TestVocab, PLAIN_OP));

        // A declared codebook that is NOT plugged into this basin: covers()
        // is false and resolution refuses rather than guessing a literal.
        let empty = BasinCodebooks::new();
        assert!(!empty.covers(&TestVocab, ASSERT_EDGE));
        assert!(empty.resolve_operand(&TestVocab, &call, 0).is_none());

        // The zero-fallback operand, through the same path.
        let unwritten = Call::with_values(ASSERT_EDGE, [0, 0, 0]);
        assert!(basin.resolve_operand(&TestVocab, &unwritten, 0).is_none());
        // An operand slot past the call's width.
        assert!(basin.resolve_operand(&TestVocab, &call, 9).is_none());
    }

    #[test]
    fn a_sealed_codebook_keeps_what_the_builder_interned() {
        let mut b = BasinCodebookBuilder::new(VERBS);
        assert!(b.is_empty());
        let i = b.intern(TEST_CLASSID, b"enables").unwrap();
        let sealed = b.seal();
        assert_eq!(sealed.len(), 1);
        assert!(!sealed.is_empty());
        assert_eq!(sealed.declares(), VERBS);
        assert_eq!(sealed.id(), VERBS.id);
        assert_eq!(&sealed.resolve(i).unwrap().bytes[..7], b"enables");
    }
}
