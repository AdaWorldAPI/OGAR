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
//! - A shared-core byte the core does not cover (e.g. `WAIT` today) is
//!   **refused everywhere** — a vocabulary does not get to guess for it.
//!   Coverage grows in the core, once, for everyone.
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
    /// operand. (`WAIT`, `STOP`, `RETURN`, `TERNARY` and others are real
    /// palette entries deliberately not yet covered.)
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
            | FnIndex::FOR_RANGE => 1,
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
    /// consults.
    #[must_use]
    pub fn min_shape(f: FnIndex) -> LaneShape {
        match body_refs(f) {
            0 | 1 => LaneShape::Pairs,
            _ => LaneShape::Triples,
        }
    }
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
    fn min_shape(&self, f: FnIndex) -> LaneShape {
        match self.body_refs(f) {
            0 | 1 => LaneShape::Pairs,
            _ => LaneShape::Triples,
        }
    }
}

/// Mechanical conformance: what every vocabulary crate's tests must run.
pub mod conformance {
    use super::{DOMAIN_FLOOR, FnIndex, Vocabulary, shared_core};

    /// A way a vocabulary violates the sharing discipline.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ConformanceError {
        /// A shared-core byte answers differently through the vocabulary than
        /// through [`shared_core`] — a composed-method override drifted.
        SharedCoreDrift {
            /// The byte that drifted.
            f: FnIndex,
            /// Which table drifted: `"stack_arity"` or `"body_refs"`.
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
        // Silence twin: uncovered control flow stays uncovered rather than
        // being quietly assigned a plausible shape. WAIT/STOP/RETURN are real
        // palette entries the core does not yet model.
        for f in [
            FnIndex::WAIT,
            FnIndex::WAIT_UNTIL,
            FnIndex::STOP,
            FnIndex::RETURN,
        ] {
            assert_eq!(shared_core::stack_arity(f), None, "{f:?} must stay refused");
        }
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
    fn conformance_fires_on_a_shape_too_narrow_for_the_refs() {
        // IF_ELSE under a Pairs-only min_shape would truncate its else arm.
        assert_eq!(
            check(&NarrowShapeVocab),
            Err(ConformanceError::ShapeTooNarrowForRefs {
                f: FnIndex::IF_ELSE
            })
        );
    }
}
