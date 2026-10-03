//! # ogar-dir-sim — the vocabulary of a simulated directory future
//!
//! ```text
//! observed G0 ──rule──► G1 ──rule──► G2 ──validate──► "desired" ──diff──► ExecutionPlan ──X
//! ```
//!
//! This crate holds only the **meaning** of that pipeline: what a change is,
//! where a version came from, what a violation says, and what a plan asks an
//! actuator to do. It executes nothing. Snapshots, versions, rules,
//! invariant evaluation and diffs run in lance-graph
//! (`crates/lance-graph-dir-sim`) over the SoA store and Quack's masking
//! operators — OGAR is the IR, lance-graph the execution (OGAR-AS-IR).
//!
//! Identity is always [`Guid128`](ogar_dir_core::Guid128). Dense ordinals,
//! dictionary ids and mask bits are execution detail and never appear here.
//! Design: `docs/DIRECTORY-SIMULATION-POC.md`.

pub mod change;
pub mod plan;
pub mod provenance;
pub mod violation;

pub use change::{Attribute, Change, normalize};
pub use plan::{ExecutionPlan, Operation, PlanError, PlannedOp, Precondition};
pub use provenance::{EvidenceRef, Origin, RuleId, TAG_DESIRED, TAG_OBSERVED, Version, VersionId};
pub use violation::{Endpoint, Violation};
