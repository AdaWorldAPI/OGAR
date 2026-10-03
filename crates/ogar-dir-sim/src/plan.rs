//! The execution boundary — described, never executed.
//!
//! A plan holds semantic operations only: no shell text, cmdlet names,
//! endpoints or credentials. Every operation carries the [`Precondition`]
//! that held in the **observed basis** it was derived from, so a future
//! actuator can re-read that one fact from reality before acting (still
//! true → execute; changed → re-observe and re-plan).

use crate::change::{Attribute, Change};
use crate::provenance::VersionId;
use ogar_dir_core::Guid128;

/// A technology-neutral directory operation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Precondition {
    /// The membership must still be absent.
    NotMember,
    /// The membership must still be present.
    IsMember,
    /// The attribute must still hold this value.
    AttributeEquals(Option<String>),
}

/// One planned step.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlannedOp {
    /// What to do.
    pub op: Operation,
    /// What must still hold first.
    pub precondition: Precondition,
}

impl From<Change> for PlannedOp {
    fn from(c: Change) -> Self {
        match c {
            Change::AddMembership { user, group } => Self {
                op: Operation::AddGroupMember {
                    group,
                    member: user,
                },
                precondition: Precondition::NotMember,
            },
            Change::RemoveMembership { user, group } => Self {
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
            } => Self {
                op: Operation::SetAttribute {
                    object: node,
                    attribute,
                    value: to,
                },
                precondition: Precondition::AttributeEquals(from),
            },
        }
    }
}

/// Semantic plan from an observed basis to a desired target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionPlan {
    /// The observation the preconditions were read from.
    pub basis: VersionId,
    /// The desired version the plan reaches.
    pub target: VersionId,
    /// Operations, sorted (canonical order).
    pub ops: Vec<PlannedOp>,
}

impl ExecutionPlan {
    /// Lower a semantic diff `basis → target` into a plan.
    pub fn from_diff(basis: VersionId, target: VersionId, diff: Vec<Change>) -> Self {
        let mut ops: Vec<PlannedOp> = diff.into_iter().map(PlannedOp::from).collect();
        ops.sort();
        Self { basis, target, ops }
    }
}

/// Why no plan was derived.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// The target is not the current `"desired"` version.
    NotDesired(VersionId),
    /// The target is unknown.
    UnknownVersion(VersionId),
    /// The latest observation (`basis`) no longer has the node set the
    /// desired version (`target`) was built on: users or groups were
    /// created or deleted since. Simulate again from the new observation.
    NodeSetChanged {
        /// The latest observation.
        basis: VersionId,
        /// The desired version.
        target: VersionId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowering_carries_the_basis_precondition_and_no_transport() {
        let g = |n| Guid128([n; 16]);
        let plan = ExecutionPlan::from_diff(
            VersionId(0),
            VersionId(2),
            vec![
                Change::SetAttribute {
                    node: g(2),
                    attribute: Attribute::PrimarySmtp,
                    from: Some("bob@example.test".into()),
                    to: Some("robert@example.test".into()),
                },
                Change::AddMembership {
                    user: g(1),
                    group: g(9),
                },
            ],
        );
        assert_eq!(
            plan.ops[0].op,
            Operation::AddGroupMember {
                group: g(9),
                member: g(1)
            }
        );
        assert_eq!(plan.ops[0].precondition, Precondition::NotMember);
        assert_eq!(
            plan.ops[1].precondition,
            Precondition::AttributeEquals(Some("bob@example.test".into()))
        );
    }
}
