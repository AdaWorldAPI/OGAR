//! A **program** — the several functions a script with control flow becomes.
//!
//! # Why one script is not one function
//!
//! Nesting is **by reference**: a loop body is another function's node, named
//! by index. No `END` marker, no jump offset. So `repeat 10 [ … ]` is **two**
//! functions — the caller and the body — and the caller spends one value byte
//! naming the second.
//!
//! That is what keeps a node fixed-size: a body of any length costs its parent
//! exactly one byte. Splicing the body inline with a terminator would make a
//! call's width depend on its contents, so editing inside a loop would shift
//! every later call in the enclosing function — the same defect that ruled out
//! literal-as-call-run for the constant pool.
//!
//! # Function 0 is the entry, and indices are stable
//!
//! [`Program::functions`] is indexed by the byte a caller stores. Function `0`
//! is the script's own body. An index, once assigned, does not move — which is
//! what makes it safe to store.
//!
//! Index `0` is therefore a **real** function rather than a zero-fallback, and
//! that is deliberate: nothing references function 0 (the entry is entered,
//! not branched to), so `0` in a body-reference byte would be a bug rather
//! than a sentinel — [`Program::references_are_resolvable`] is what says so.
//!
//! # The forward-reference invariant (for lowerers)
//!
//! A lowerer should reserve a body's index BEFORE lowering its contents, so a
//! parent's index is always LOWER than its children's and every stored
//! reference points **forward**. (Both orders are bijective; the reservation
//! buys the readable invariant, not collision-freedom — a lesson recorded in
//! the first lowerer's history.) Lowering itself lives in the vocabulary
//! crates: casting a frontend's records into calls needs the frontend's
//! shapes, and this crate deliberately has none.
//!
//! # What this does not do
//!
//! It does not mint keys. A stored program is N
//! [`FunctionNode`](crate::FunctionNode)s and each needs a GUID; minting is
//! the substrate's, per vocabulary. A `Program` carries bodies and the caller
//! supplies keys — the same boundary [`crate::node`] draws.

use crate::{Call, FunctionBody, Vocabulary};

/// One script's functions. Index `0` is the entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// The bodies, indexed by the byte a body-reference stores.
    pub functions: Vec<FunctionBody>,
}

impl Program {
    /// The entry body.
    #[must_use]
    pub fn entry(&self) -> &FunctionBody {
        &self.functions[0]
    }

    /// How many functions the script became.
    #[must_use]
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Whether the program holds no functions. Never true for a lowered
    /// script — the entry always exists.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    /// Every body-reference names a function that exists, and none names the
    /// entry.
    ///
    /// Both halves matter. An out-of-range index is a dangling branch; a
    /// reference to function `0` is a loop back into the entry, which a
    /// lowerer never emits and which would be an unbounded recursion if
    /// honoured.
    ///
    /// The vocabulary is a parameter because *which value bytes are
    /// references* is a per-function fact ([`Vocabulary::body_refs`]) — the
    /// same bytes under a different vocabulary could be plain immediates.
    #[must_use]
    pub fn references_are_resolvable<V: Vocabulary>(&self, v: &V) -> bool {
        for body in &self.functions {
            for call in body.calls() {
                let n = v.body_refs(call.function);
                for slot in 0..usize::from(n) {
                    let idx = usize::from(call.values[slot]);
                    if idx == 0 || idx >= self.functions.len() {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// Read a body's calls back as `(index, call, targets)` triples, resolving
/// which value bytes are branches.
///
/// Useful to a consumer walking a program: it says *which* bytes are function
/// indices without re-deriving [`Vocabulary::body_refs`] call by call.
#[must_use]
pub fn branches_of<V: Vocabulary>(v: &V, body: &FunctionBody) -> Vec<(usize, Call, Vec<u8>)> {
    body.calls()
        .enumerate()
        .filter_map(|(i, c)| {
            let n = usize::from(v.body_refs(c.function));
            (n > 0).then(|| (i, c, c.values[..n].to_vec()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FnIndex, LaneShape};

    const S: LaneShape = LaneShape::Pairs;

    /// The smallest conforming vocabulary — everything below the floor is the
    /// shared core, nothing above it.
    struct EmptyVocab;
    impl Vocabulary for EmptyVocab {
        fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
            None
        }
        fn domain_body_refs(&self, _f: FnIndex) -> u8 {
            0
        }
    }

    /// `repeat 10 [ print 1 ]` — hand-lowered: entry = (NUMBER:10)(REPEAT:1),
    /// function 1 = (NUMBER:1)(PRINT).
    fn repeat_ten() -> Program {
        Program {
            functions: vec![
                FunctionBody::from_calls(
                    S,
                    &[
                        Call::with_value(FnIndex::NUMBER, 10),
                        Call::with_value(FnIndex::REPEAT, 1),
                    ],
                )
                .unwrap(),
                FunctionBody::from_calls(
                    S,
                    &[
                        Call::with_value(FnIndex::NUMBER, 1),
                        Call::new(FnIndex::PRINT),
                    ],
                )
                .unwrap(),
            ],
        }
    }

    #[test]
    fn every_reference_resolves_and_none_points_at_the_entry() {
        let v = EmptyVocab;
        let prog = repeat_ten();
        assert!(prog.references_are_resolvable(&v));
        assert_eq!(prog.len(), 2);
        assert_eq!(prog.entry().len(), 2);

        // Two-sided, and both halves are real failure modes. A dangling index:
        let mut dangling = prog.clone();
        dangling.functions[0] = FunctionBody::from_calls(
            S,
            &[
                Call::with_value(FnIndex::NUMBER, 10),
                Call::with_value(FnIndex::REPEAT, 9),
            ],
        )
        .unwrap();
        assert!(
            !dangling.references_are_resolvable(&v),
            "a dangling branch must be caught"
        );

        // …and a branch back into the entry, which would be unbounded.
        dangling.functions[0] = FunctionBody::from_calls(
            S,
            &[
                Call::with_value(FnIndex::NUMBER, 10),
                Call::with_value(FnIndex::REPEAT, 0),
            ],
        )
        .unwrap();
        assert!(
            !dangling.references_are_resolvable(&v),
            "a branch to the entry must be caught"
        );
    }

    #[test]
    fn resolvability_consults_the_vocabulary_not_a_baked_table() {
        // The SAME bytes must flip from resolvable to dangling when the
        // vocabulary declares the function branching — which is the whole
        // reason `references_are_resolvable` takes a vocabulary parameter.
        struct BranchyDomain;
        impl Vocabulary for BranchyDomain {
            fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
                Some(0)
            }
            fn domain_body_refs(&self, f: FnIndex) -> u8 {
                u8::from(f.0 == 0x90)
            }
        }

        let prog = Program {
            functions: vec![
                FunctionBody::from_calls(S, &[Call::with_value(FnIndex(0x90), 7)]).unwrap(),
            ],
        };
        // Under the empty vocabulary 0x90's value byte is a plain immediate.
        assert!(prog.references_are_resolvable(&EmptyVocab));
        // Under a vocabulary where 0x90 branches, 7 is a dangling reference.
        assert!(!prog.references_are_resolvable(&BranchyDomain));
    }

    #[test]
    fn branches_of_reports_which_bytes_are_function_indices() {
        let v = EmptyVocab;
        let prog = repeat_ten();
        let b = branches_of(&v, prog.entry());
        assert_eq!(b.len(), 1, "one branching call in the entry");
        let (idx, call, targets) = &b[0];
        assert_eq!(*idx, 1, "it is the second call");
        assert_eq!(call.function, FnIndex::REPEAT);
        assert_eq!(targets, &vec![1u8]);
        // Silence twin: a straight-line body reports NO branches, so this is
        // not a function that always finds something.
        assert!(branches_of(&v, &prog.functions[1]).is_empty());
    }
}
