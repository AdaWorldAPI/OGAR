//! The execution boundary — derived, never executed here.
//!
//! ```text
//!   desired version ──diff from its observed basis──► ExecutionPlan ──X── actuator (later)
//! ```
//!
//! A plan holds semantic operations only: no shell text, no cmdlet names,
//! no endpoints, no credentials. Each operation carries the
//! [`Precondition`] that held in the **observed basis** the plan was derived
//! from. A future actuator re-reads that one fact from reality before acting:
//! still true → execute; changed → re-observe and re-plan (optimistic
//! concurrency). Nothing in this module performs that check or any I/O.

use crate::graph::{Attribute, Change};
use crate::store::{Origin, SimError, VersionId, VersionStore};
use ogar_dir_core::Guid128;

/// A technology-neutral directory operation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Operation {
    /// Add `member` to `group`.
    AddGroupMember {
        /// Group.
        group: Guid128,
        /// Member.
        member: Guid128,
    },
    /// Remove `member` from `group`.
    RemoveGroupMember {
        /// Group.
        group: Guid128,
        /// Member.
        member: Guid128,
    },
    /// Set an attribute (`None` clears it).
    SetAttribute {
        /// Object.
        object: Guid128,
        /// Attribute.
        attribute: Attribute,
        /// New value.
        value: Option<String>,
    },
}

/// What reality must still look like for the operation to be safe.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precondition {
    /// The membership must still be absent.
    NotMember,
    /// The membership must still be present.
    IsMember,
    /// The attribute must still hold this value.
    AttributeEquals(Option<String>),
}

/// One planned step.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlannedOp {
    /// What to do.
    pub op: Operation,
    /// What must still hold first.
    pub precondition: Precondition,
}

/// Semantic plan from an observed basis to a desired target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionPlan {
    /// The observation the preconditions were read from.
    pub basis: VersionId,
    /// The desired version the plan reaches.
    pub target: VersionId,
    /// Operations, deterministic order.
    pub ops: Vec<PlannedOp>,
}

/// Why no plan was derived.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// The target is not the current `"desired"` version.
    NotDesired(VersionId),
    /// Version lookup failed.
    Sim(SimError),
    /// The target creates or deletes nodes (not plannable in this slice).
    NodeSetChanged,
}

fn lower(c: Change) -> PlannedOp {
    match c {
        Change::AddMembership { user, group } => PlannedOp {
            op: Operation::AddGroupMember {
                group,
                member: user,
            },
            precondition: Precondition::NotMember,
        },
        Change::RemoveMembership { user, group } => PlannedOp {
            op: Operation::RemoveGroupMember {
                group,
                member: user,
            },
            precondition: Precondition::IsMember,
        },
        Change::SetAttribute {
            node,
            attribute,
            from,
            to,
        } => PlannedOp {
            op: Operation::SetAttribute {
                object: node,
                attribute,
                value: to,
            },
            precondition: Precondition::AttributeEquals(from),
        },
    }
}

impl ExecutionPlan {
    /// Derive the plan for the current `"desired"` version `target`, from
    /// the observed root of its lineage.
    pub fn derive(store: &VersionStore, target: VersionId) -> Result<Self, PlanError> {
        if store.tag(crate::store::TAG_DESIRED) != Some(target) {
            return Err(PlanError::NotDesired(target));
        }
        let basis = store.lineage(target).map_err(PlanError::Sim)?[0];
        debug_assert!(matches!(
            store.version(basis).map(|v| &v.origin),
            Some(Origin::Observed { .. })
        ));
        let (b, t) = (
            store.state(basis).map_err(PlanError::Sim)?,
            store.state(target).map_err(PlanError::Sim)?,
        );
        if b.node_set_differs(&t) {
            return Err(PlanError::NodeSetChanged);
        }
        let mut ops: Vec<PlannedOp> = b.diff(&t).into_iter().map(lower).collect();
        ops.sort();
        Ok(Self { basis, target, ops })
    }
}
