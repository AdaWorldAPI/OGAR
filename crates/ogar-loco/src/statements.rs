//! Statement boundaries — the unit a step mask may address.
//!
//! # Why calls are not maskable
//!
//! A body is a stack program: each call pops its operands and (for
//! expressions) pushes a result. Masking one CALL out of that stream
//! desynchronizes every later consumer — the exact defect the two-quantity
//! split exists to prevent, reintroduced at dispatch time. So the maskable
//! unit is the **statement**: the operand-producing post-order run PLUS the
//! consuming statement call, skipped or kept as one piece.
//!
//! This is the R5 ruling made mechanical, and it dissolves the apparent
//! capacity mismatch between a 64-bit step mask and a 180-call body: the
//! mask addresses up to 64 STATEMENTS; the body budget stays 180 CALLS.
//! A template wanting more than 64 statements is a split signal, not a
//! mask-widening use case.
//!
//! # The segmentation rule
//!
//! Walk the calls simulating stack depth (`depth -= arity; depth += 1` if
//! the call pushes). A statement CLOSES where depth returns to zero after a
//! **non-pushing** call. A body ending at depth one closes a final
//! *expression statement* (the script's value). Anything else refuses:
//!
//! - an **uncovered** call (no arity, or no `pushes_result` declaration) —
//!   segmentation does not guess; a vocabulary that wants segmentable
//!   bodies declares the column;
//! - a stack **underflow** — the body is malformed under this vocabulary;
//! - **dangling operands** (final depth ≥ 2) — no honest grouping exists.
//!
//! The vocabulary arrives as a [`CheckedVocabulary`] and the walk reads its
//! validated table.

use crate::vocabulary::conformance::CheckedVocabulary;
use crate::{FnIndex, FunctionBody, Vocabulary};

/// One statement's extent inside a body, in call indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatementBounds {
    /// Index of the statement's first call.
    pub first_call: usize,
    /// How many calls the statement spans (operands + the statement call).
    pub call_count: usize,
}

/// Why a body could not be segmented into statements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementError {
    /// A call's function has no arity or no `pushes_result` declaration in
    /// this vocabulary — refused rather than guessed, because a wrong guess
    /// mis-groups statements silently.
    Uncovered {
        /// Position of the call in the body.
        index: usize,
        /// The undeclared function.
        f: FnIndex,
    },
    /// A call pops more operands than the stack holds — the body is
    /// malformed under this vocabulary.
    StackUnderflow {
        /// Position of the underflowing call.
        index: usize,
        /// The function that underflowed.
        f: FnIndex,
    },
    /// The body ends with two or more values on the stack — there is no
    /// honest statement grouping for dangling operands.
    DanglingOperands {
        /// The final stack depth.
        depth: usize,
    },
}

impl core::fmt::Display for StatementError {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StatementError::Uncovered { index, f } => write!(
                fmt,
                "call {index} ({f:?}) is not declared for segmentation in this vocabulary"
            ),
            StatementError::StackUnderflow { index, f } => {
                write!(fmt, "call {index} ({f:?}) pops more operands than exist")
            }
            StatementError::DanglingOperands { depth } => {
                write!(fmt, "body ends with {depth} dangling operands")
            }
        }
    }
}

impl core::error::Error for StatementError {}

/// Segment a body into statements — the derived metadata a step-mask
/// dispatcher consumes (`statement ordinal → [first_call, call_count]`).
///
/// # Errors
///
/// See [`StatementError`]; every arm refuses rather than guesses.
pub fn statement_bounds<V: Vocabulary>(
    v: &CheckedVocabulary<V>,
    body: &FunctionBody,
) -> Result<Vec<StatementBounds>, StatementError> {
    let table = v.table();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, call) in body.calls().enumerate() {
        let f = call.function;
        let (Some(arity), Some(pushes)) = (table.stack_arity(f), table.pushes_result(f)) else {
            return Err(StatementError::Uncovered { index, f });
        };
        depth = depth
            .checked_sub(usize::from(arity))
            .ok_or(StatementError::StackUnderflow { index, f })?;
        if pushes {
            depth += 1;
        } else if depth == 0 {
            out.push(StatementBounds {
                first_call: start,
                call_count: index + 1 - start,
            });
            start = index + 1;
        }
    }
    match depth {
        0 => Ok(out),
        1 => {
            // A trailing value: the final expression statement (the script's
            // own result). `start < len` holds — a boundary resets depth to
            // zero, so depth one implies calls after the last boundary.
            out.push(StatementBounds {
                first_call: start,
                call_count: body.len() - start,
            });
            Ok(out)
        }
        depth => Err(StatementError::DanglingOperands { depth }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::conformance::validate;
    use crate::{Call, LaneShape};

    struct EmptyVocab;
    impl Vocabulary for EmptyVocab {
        fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
            None
        }
        fn domain_body_refs(&self, _f: FnIndex) -> u8 {
            0
        }
    }

    const S: LaneShape = LaneShape::Pairs;

    fn body(calls: &[Call]) -> FunctionBody {
        FunctionBody::from_calls(S, calls).unwrap()
    }

    #[test]
    fn an_expression_is_one_trailing_statement() {
        // `5 + 3` leaves its value: one expression statement spanning all
        // three calls — the operand run is atomic with its consumer.
        let v = validate(EmptyVocab).unwrap();
        let b = body(&[
            Call::with_value(FnIndex::NUMBER, 5),
            Call::with_value(FnIndex::NUMBER, 3),
            Call::new(FnIndex::ADD),
        ]);
        assert_eq!(
            statement_bounds(&v, &b).unwrap(),
            vec![StatementBounds {
                first_call: 0,
                call_count: 3
            }]
        );
    }

    #[test]
    fn control_calls_close_statements_and_two_statements_split_correctly() {
        // `if 5 [→1]; repeat 10 [→1]` — each control call consumes its
        // operand and closes at depth zero. THE capacity dissolution: the
        // mask addresses these two STATEMENTS, not the four calls.
        let v = validate(EmptyVocab).unwrap();
        let b = body(&[
            Call::with_value(FnIndex::NUMBER, 5),
            Call::with_value(FnIndex::IF, 1),
            Call::with_value(FnIndex::NUMBER, 10),
            Call::with_value(FnIndex::REPEAT, 1),
        ]);
        assert_eq!(
            statement_bounds(&v, &b).unwrap(),
            vec![
                StatementBounds {
                    first_call: 0,
                    call_count: 2
                },
                StatementBounds {
                    first_call: 2,
                    call_count: 2
                },
            ]
        );
        // …and a closed statement followed by a trailing expression mixes.
        let mixed = body(&[
            Call::with_value(FnIndex::NUMBER, 5),
            Call::with_value(FnIndex::IF, 1),
            Call::with_value(FnIndex::NUMBER, 9),
        ]);
        assert_eq!(
            statement_bounds(&v, &mixed).unwrap(),
            vec![
                StatementBounds {
                    first_call: 0,
                    call_count: 2
                },
                StatementBounds {
                    first_call: 2,
                    call_count: 1
                },
            ]
        );
    }

    #[test]
    fn masking_hazards_are_refused_not_guessed() {
        let v = validate(EmptyVocab).unwrap();
        // Underflow: ADD with an empty stack is malformed, not "arity 0".
        assert_eq!(
            statement_bounds(&v, &body(&[Call::new(FnIndex::ADD)])),
            Err(StatementError::StackUnderflow {
                index: 0,
                f: FnIndex::ADD
            })
        );
        // Dangling operands: two values, no consumer — no honest grouping.
        assert_eq!(
            statement_bounds(
                &v,
                &body(&[
                    Call::with_value(FnIndex::NUMBER, 1),
                    Call::with_value(FnIndex::NUMBER, 2),
                ])
            ),
            Err(StatementError::DanglingOperands { depth: 2 })
        );
        // Uncovered: WAIT has no shared-core tables — refuse, never guess.
        assert_eq!(
            statement_bounds(&v, &body(&[Call::with_value(FnIndex::WAIT, 3)])),
            Err(StatementError::Uncovered {
                index: 0,
                f: FnIndex::WAIT
            })
        );
    }

    #[test]
    fn a_domain_verb_without_the_pushes_column_refuses_segmentation() {
        // Arity-covered but pushes-undeclared = lowerable but NOT
        // segmentable — the honest partial-coverage state the None default
        // produces. The silence twin: declaring the column makes the same
        // body segment.
        struct ArityOnly;
        impl Vocabulary for ArityOnly {
            fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
                (f.0 == 0x90).then_some(0)
            }
            fn domain_body_refs(&self, _f: FnIndex) -> u8 {
                0
            }
        }
        struct Declared;
        impl Vocabulary for Declared {
            fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
                (f.0 == 0x90).then_some(0)
            }
            fn domain_body_refs(&self, _f: FnIndex) -> u8 {
                0
            }
            fn domain_pushes_result(&self, f: FnIndex) -> Option<bool> {
                (f.0 == 0x90).then_some(false)
            }
        }
        let b = body(&[Call::new(FnIndex(0x90))]);
        let arity_only = validate(ArityOnly).unwrap();
        assert_eq!(
            statement_bounds(&arity_only, &b),
            Err(StatementError::Uncovered {
                index: 0,
                f: FnIndex(0x90)
            })
        );
        let declared = validate(Declared).unwrap();
        assert_eq!(
            statement_bounds(&declared, &b).unwrap(),
            vec![StatementBounds {
                first_call: 0,
                call_count: 1
            }]
        );
    }
}
