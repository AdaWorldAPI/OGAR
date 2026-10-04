//! The execution boundary — described, never executed.
//!
//! A plan holds semantic operations only: no shell text, cmdlet names,
//! endpoints or credentials. Every operation carries the [`Precondition`]
//! that held in the **observed basis** it was derived from, so a future
//! actuator can re-read that one fact from reality before acting (still
//! true → execute; changed → re-observe and re-plan).

use crate::change::{Attribute, Change, NodeState};
use crate::provenance::VersionId;
use ogar_dir_core::Guid128;

/// A technology-neutral directory operation.
///
/// Variant order is load-bearing and mirrors [`Change`]: removals, deletes,
/// attribute sets, creates, adds. A sorted plan is therefore a safe
/// execution order (an address is freed before it is claimed; a node's
/// edges are gone before it is, and it exists before it gains one).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Operation {
    /// Remove `member` from `group`.
    RemoveGroupMember {
        /// Group.
        group: Guid128,
        /// Member.
        member: Guid128,
    },
    /// Delete `object`.
    DeleteObject {
        /// Object.
        object: Guid128,
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
    /// Create `object` with `state`.
    CreateObject {
        /// Object.
        object: Guid128,
        /// Its content.
        state: NodeState,
    },
    /// Add `member` to `group`.
    AddGroupMember {
        /// Group.
        group: Guid128,
        /// Member.
        member: Guid128,
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
    /// No object with this identity may exist yet.
    ObjectAbsent,
    /// The object must still exist in exactly this state and have no
    /// membership on either side (none was added since the observation, and
    /// the plan's own removals have run), so a delete never strands an edge.
    ObjectRemovable(NodeState),
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
            Change::CreateNode { node, state } => Self {
                op: Operation::CreateObject {
                    object: node,
                    state,
                },
                precondition: Precondition::ObjectAbsent,
            },
            Change::DeleteNode { node, state } => Self {
                op: Operation::DeleteObject { object: node },
                precondition: Precondition::ObjectRemovable(state),
            },
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
        ops.dedup();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change::NodeKind;

    #[test]
    fn a_repeated_change_is_planned_once() {
        let c = Change::RemoveMembership {
            user: Guid128([1; 16]),
            group: Guid128([2; 16]),
        };
        let p = ExecutionPlan::from_diff(VersionId(0), VersionId(1), vec![c.clone(), c]);
        assert_eq!(p.ops.len(), 1);
    }

    fn state(kind: NodeKind) -> NodeState {
        NodeState {
            kind,
            active: true,
            upn: None,
            primary_smtp: None,
            ou: None,
        }
    }

    #[test]
    fn a_sorted_plan_frees_before_it_claims() {
        let g = |n| Guid128([n; 16]);
        let with_smtp = |smtp: &str| NodeState {
            primary_smtp: Some(smtp.into()),
            ..state(NodeKind::User)
        };
        // Given in the worst order: user 5 is deleted and user 1 created
        // with the address 5 had, user 3 is renamed, user 1 joins a group.
        let plan = ExecutionPlan::from_diff(
            VersionId(0),
            VersionId(1),
            vec![
                Change::AddMembership {
                    user: g(1),
                    group: g(9),
                },
                Change::CreateNode {
                    node: g(1),
                    state: with_smtp("shared@example.test"),
                },
                Change::SetAttribute {
                    node: g(3),
                    attribute: Attribute::PrimarySmtp,
                    from: Some("c@example.test".into()),
                    to: Some("c2@example.test".into()),
                },
                Change::DeleteNode {
                    node: g(5),
                    state: with_smtp("shared@example.test"),
                },
                Change::RemoveMembership {
                    user: g(5),
                    group: g(9),
                },
            ],
        );
        let ops: Vec<_> = plan.ops.iter().map(|p| &p.op).collect();
        assert!(matches!(ops[0], Operation::RemoveGroupMember { member, .. } if *member == g(5)));
        // The delete frees "shared@" before the create claims it.
        assert_eq!(ops[1], &Operation::DeleteObject { object: g(5) });
        assert!(matches!(ops[2], Operation::SetAttribute { object, .. } if *object == g(3)));
        assert!(matches!(ops[3], Operation::CreateObject { object, .. } if *object == g(1)));
        assert!(matches!(ops[4], Operation::AddGroupMember { member, .. } if *member == g(1)));
        assert_eq!(
            plan.ops[1].precondition,
            Precondition::ObjectRemovable(with_smtp("shared@example.test"))
        );
        assert_eq!(plan.ops[3].precondition, Precondition::ObjectAbsent);
    }

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
            plan.ops[1].op,
            Operation::AddGroupMember {
                group: g(9),
                member: g(1)
            }
        );
        assert_eq!(plan.ops[1].precondition, Precondition::NotMember);
        assert_eq!(
            plan.ops[0].precondition,
            Precondition::AttributeEquals(Some("bob@example.test".into()))
        );
    }
}
