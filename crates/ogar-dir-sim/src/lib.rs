//! # ogar-dir-sim — explore a directory future without touching reality
//!
//! ```text
//! observed G0 ──Rule──► G1 ──Rule──► G2 ──validate──► desired? ──diff(G0,G2)──► ExecutionPlan ──X
//! ```
//!
//! * [`graph`] — the semantic state of one version and the [`Change`] algebra.
//! * [`population`] — node sets as bitsets; rules select populations, not loops.
//! * [`rule`] — pure rules: `&GraphState -> Vec<Change>`, no I/O handle.
//! * [`store`] — append-only versions; provenance and tags ARE the audit trail.
//! * [`validate`] — invariants returning structured [`Violation`]s.
//! * [`plan`] — semantic [`ExecutionPlan`] with per-op preconditions. Never executed here.
//! * [`observe`] — `ogar-ad` records → observed state.
//!
//! Nothing in this crate writes to AD, Entra, Exchange, LDAP or PowerShell,
//! and nothing in it can: there is no network, process or file I/O.
//! Design notes: `docs/DIRECTORY-SIMULATION-POC.md`.

pub mod graph;
pub mod observe;
pub mod plan;
pub mod population;
pub mod rule;
pub mod store;
pub mod validate;

pub use graph::{Attribute, Change, GraphState, Node, NodeKind};
pub use plan::{ExecutionPlan, Operation, PlanError, PlannedOp, Precondition};
pub use population::Population;
pub use rule::{EvidenceRef, Rule, RuleId};
pub use store::{
    Origin, Rejection, SimError, TAG_DESIRED, TAG_OBSERVED, Version, VersionId, VersionStore,
};
pub use validate::{Endpoint, Violation, validate};
