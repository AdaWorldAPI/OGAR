//! Funnel telemetry — refusal statistics as data (wishlist W-4).
//!
//! # The oracle contract's safe half
//!
//! A generate-and-filter loop (an LLM emitting N candidate bodies, this
//! crate's parse/validate/cast refusing the garbage) needs to know WHICH
//! gate killed each surviving-or-not candidate, so the loop can react —
//! re-prompt, narrow the vocabulary shown, adjust the shape. That is
//! **validity feedback**: "candidate 7 underflowed the stack at call 3." It
//! is safe to hand back to a generator raw; it names a structural defect in
//! the emitted bytes, not a judgment about the candidate's quality.
//!
//! **Fitness feedback — how WELL a surviving candidate performed — is
//! deliberately out of scope here.** That is a downstream instrument's
//! concern (lance-graph's observer-effect payload law: distribution shape ×
//! rank, never a raw scalar looped back into the generator), and this crate
//! has no fitness signal to report in the first place — it only knows
//! whether a candidate parses, casts, and segments.
//!
//! This module is therefore a **pure tally**: fold a batch of
//! [`Result`]s from the crate's own error types into counts per variant.
//! No scoring, no ranking, no scalar feedback — the safe slice, and
//! nothing past it.

use crate::pool::PoolError;
use crate::statements::StatementError;
use crate::vocabulary::conformance::ConformanceError;

/// Which named gate refused a candidate — a flat key so counts can be
/// reported without matching on three different error enums downstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum RefusalGate {
    /// [`ConformanceError::SharedCoreDrift`].
    ConformanceDrift,
    /// [`ConformanceError::ShapeTooNarrowForRefs`].
    ConformanceShapeTooNarrow,
    /// [`StatementError::Uncovered`].
    StatementUncovered,
    /// [`StatementError::StackUnderflow`].
    StatementUnderflow,
    /// [`StatementError::DanglingOperands`].
    StatementDangling,
    /// [`PoolError::Full`].
    PoolFull,
    /// [`PoolError::TooWide`].
    PoolTooWide,
}

impl RefusalGate {
    /// Every gate, for a stable iteration/report order.
    pub const ALL: [RefusalGate; 7] = [
        RefusalGate::ConformanceDrift,
        RefusalGate::ConformanceShapeTooNarrow,
        RefusalGate::StatementUncovered,
        RefusalGate::StatementUnderflow,
        RefusalGate::StatementDangling,
        RefusalGate::PoolFull,
        RefusalGate::PoolTooWide,
    ];
}

impl From<&ConformanceError> for RefusalGate {
    fn from(e: &ConformanceError) -> Self {
        match e {
            ConformanceError::SharedCoreDrift { .. } => RefusalGate::ConformanceDrift,
            ConformanceError::ShapeTooNarrowForRefs { .. } => {
                RefusalGate::ConformanceShapeTooNarrow
            }
        }
    }
}

impl From<&StatementError> for RefusalGate {
    fn from(e: &StatementError) -> Self {
        match e {
            StatementError::Uncovered { .. } => RefusalGate::StatementUncovered,
            StatementError::StackUnderflow { .. } => RefusalGate::StatementUnderflow,
            StatementError::DanglingOperands { .. } => RefusalGate::StatementDangling,
        }
    }
}

impl From<&PoolError> for RefusalGate {
    fn from(e: &PoolError) -> Self {
        match e {
            PoolError::Full => RefusalGate::PoolFull,
            PoolError::TooWide { .. } => RefusalGate::PoolTooWide,
        }
    }
}

/// A batch's refusal tally — how many candidates survived, and how many the
/// funnel refused, broken down by [`RefusalGate`].
///
/// Deliberately data-only: no ranking, no scoring, no fitness signal. Build
/// with [`FunnelTally::default`] and [`record`](Self::record) each
/// candidate's outcome as the batch runs, or fold a slice of results with
/// [`FunnelTally::from_results`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FunnelTally {
    /// How many candidates survived every gate.
    pub survived: u32,
    /// Refusal counts, one entry per [`RefusalGate::ALL`] member in order.
    counts: [u32; RefusalGate::ALL.len()],
}

impl FunnelTally {
    /// A tally over an already-collected batch of outcomes.
    #[must_use]
    pub fn from_results<'a, E, I>(results: I) -> Self
    where
        E: 'a,
        RefusalGate: for<'b> From<&'b E>,
        I: IntoIterator<Item = &'a Result<(), E>>,
    {
        let mut t = Self::default();
        for r in results {
            t.record(r.as_ref().map(|_| ()).map_err(RefusalGate::from));
        }
        t
    }

    /// Record one candidate's outcome: `Ok(())` for a survivor, `Err(gate)`
    /// for a refusal at the named gate.
    pub fn record(&mut self, outcome: Result<(), RefusalGate>) {
        match outcome {
            Ok(()) => self.survived += 1,
            Err(gate) => {
                let i = RefusalGate::ALL
                    .iter()
                    .position(|g| *g == gate)
                    .expect("RefusalGate::ALL is exhaustive over the enum");
                self.counts[i] += 1;
            }
        }
    }

    /// How many candidates this tally has seen in total.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.survived + self.counts.iter().sum::<u32>()
    }

    /// Refusals at one gate.
    #[must_use]
    pub fn at(&self, gate: RefusalGate) -> u32 {
        let i = RefusalGate::ALL
            .iter()
            .position(|g| *g == gate)
            .expect("RefusalGate::ALL is exhaustive over the enum");
        self.counts[i]
    }

    /// `(gate, count)` for every gate that refused at least one candidate,
    /// in [`RefusalGate::ALL`] order — the report a caller actually wants
    /// (a batch that never hit `PoolFull` should not print a zero row).
    pub fn nonzero_gates(&self) -> impl Iterator<Item = (RefusalGate, u32)> + '_ {
        RefusalGate::ALL
            .into_iter()
            .zip(self.counts)
            .filter(|(_, n)| *n > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mixed_batch_tallies_survivors_and_gates_separately() {
        let mut t = FunnelTally::default();
        t.record(Ok(()));
        t.record(Ok(()));
        t.record(Err(RefusalGate::StatementUnderflow));
        t.record(Err(RefusalGate::StatementUnderflow));
        t.record(Err(RefusalGate::PoolFull));

        assert_eq!(t.survived, 2);
        assert_eq!(t.at(RefusalGate::StatementUnderflow), 2);
        assert_eq!(t.at(RefusalGate::PoolFull), 1);
        // Silence twin: a gate that never fired reports zero, not absence
        // masquerading as failure to look it up.
        assert_eq!(t.at(RefusalGate::PoolTooWide), 0);
        assert_eq!(t.total(), 5);
    }

    #[test]
    fn nonzero_gates_omits_gates_that_never_fired() {
        let mut t = FunnelTally::default();
        t.record(Ok(()));
        t.record(Err(RefusalGate::ConformanceDrift));
        let report: Vec<_> = t.nonzero_gates().collect();
        assert_eq!(report, vec![(RefusalGate::ConformanceDrift, 1)]);
        // Anti-vacuity: a fully-clean batch reports an EMPTY nonzero list,
        // not a list of every gate at zero.
        let clean = FunnelTally {
            survived: 3,
            ..Default::default()
        };
        assert_eq!(clean.nonzero_gates().count(), 0);
    }

    #[test]
    fn real_error_types_map_to_the_gate_that_actually_fired() {
        // This is the point: a caller running the real funnel does not
        // hand-translate three enums into RefusalGate — `.into()` does it,
        // and it must land on the RIGHT gate, not just *a* gate.
        let underflow = StatementError::StackUnderflow {
            index: 0,
            f: crate::FnIndex::ADD,
        };
        assert_eq!(
            RefusalGate::from(&underflow),
            RefusalGate::StatementUnderflow
        );
        let dangling = StatementError::DanglingOperands { depth: 2 };
        assert_eq!(RefusalGate::from(&dangling), RefusalGate::StatementDangling);
        let full = PoolError::Full;
        assert_eq!(RefusalGate::from(&full), RefusalGate::PoolFull);
        let drift = ConformanceError::SharedCoreDrift {
            f: crate::FnIndex::ADD,
            what: "stack_arity",
        };
        assert_eq!(RefusalGate::from(&drift), RefusalGate::ConformanceDrift);
    }

    #[test]
    fn from_results_folds_a_batch_without_hand_rolled_matching() {
        let batch: Vec<Result<(), StatementError>> = vec![
            Ok(()),
            Err(StatementError::StackUnderflow {
                index: 0,
                f: crate::FnIndex::ADD,
            }),
            Ok(()),
        ];
        let t = FunnelTally::from_results(batch.iter());
        assert_eq!(t.survived, 2);
        assert_eq!(t.at(RefusalGate::StatementUnderflow), 1);
    }
}
