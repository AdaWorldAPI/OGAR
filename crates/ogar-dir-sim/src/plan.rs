//! The execution boundary — described, never executed.
//!
//! A plan holds semantic operations only: no shell text, cmdlet names,
//! endpoints or credentials. Every operation carries the [`Precondition`]
//! that held in the **observed basis** it was derived from, so a future
//! actuator can re-read that one fact from reality before acting (still
//! true → execute; changed → re-observe and re-plan).

use crate::change::{Attribute, Change, KeyId, NodeState, ValueId};
use crate::provenance::VersionId;
use ogar_dir_core::{Dn128, Guid128};
use std::collections::{BTreeMap, BTreeSet};

/// A technology-neutral directory operation.
///
/// Variant order is load-bearing and mirrors [`Change`]: removals, deletes,
/// attribute sets, enable/disable, moves, creates, adds. A sorted plan is therefore a safe
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
        value: Option<ValueId>,
    },
    /// Enable or disable `object`. A `bool`, not an `Option`: an actuator
    /// cannot write "unknown", so a change towards an unknown flag has no
    /// operation.
    SetEnabled {
        /// Object.
        object: Guid128,
        /// Enabled.
        enabled: bool,
    },
    /// Move `object` to a known location (numeric; the actuator resolves
    /// it to its own addressing at the boundary).
    MoveObject {
        /// Object.
        object: Guid128,
        /// Destination.
        to: Dn128,
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
    AttributeEquals(Option<ValueId>),
    /// The enabled flag must still read this (`None`: still unreported).
    EnabledEquals(Option<bool>),
    /// The object must still be at this location (`None`: still unknown).
    LocationEquals(Option<Dn128>),
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

impl PlannedOp {
    /// Lower one change. `Err(node)` when the change has no operation: its
    /// target is an unknown flag or location, which no actuator can write.
    ///
    /// # Errors
    ///
    /// The node of a non-actuatable change.
    pub fn lower(c: Change) -> Result<Self, Guid128> {
        Ok(match c {
            Change::SetActive { node, from, to } => Self {
                op: Operation::SetEnabled {
                    object: node,
                    enabled: to.ok_or(node)?,
                },
                precondition: Precondition::EnabledEquals(from),
            },
            Change::SetLocation { node, from, to } => Self {
                op: Operation::MoveObject {
                    object: node,
                    to: to.ok_or(node)?,
                },
                precondition: Precondition::LocationEquals(from),
            },
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
        })
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
    /// Lower a semantic diff `basis → target` into a plan. `key` maps a
    /// value to its comparison key (the store's ingress mapping), so rename
    /// dependencies follow the directory's notion of equal addresses.
    ///
    /// # Errors
    ///
    /// [`PlanError::NotActuatable`] if a change targets an unknown flag or
    /// location; no partial plan is returned.
    pub fn from_diff(
        basis: VersionId,
        target: VersionId,
        diff: Vec<Change>,
        key: impl Fn(ValueId) -> KeyId,
    ) -> Result<Self, PlanError> {
        let mut ops = diff
            .into_iter()
            .map(PlannedOp::lower)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|node| PlanError::NotActuatable {
                basis,
                target,
                node,
            })?;
        ops.sort();
        ops.dedup();
        order_value_transfers(&mut ops, &key);
        Ok(Self { basis, target, ops })
    }
}

/// A claimed or released value, by comparison key (`None` = no value).
type Transfer = (Attribute, Option<KeyId>);

/// The value an attribute op claims, and the value it releases.
fn transfer(p: &PlannedOp, key: &impl Fn(ValueId) -> KeyId) -> Option<(Transfer, Transfer)> {
    match (&p.op, &p.precondition) {
        (
            Operation::SetAttribute {
                attribute, value, ..
            },
            Precondition::AttributeEquals(from),
        ) => Some(((*attribute, value.map(key)), (*attribute, from.map(key)))),
        _ => None,
    }
}

/// Within the (contiguous) attribute-set block of a sorted plan, run every
/// set that releases a value before any set on another object that claims
/// it: object 2 `b → c` before object 1 `a → b`. Kahn's algorithm, ties
/// broken by the canonical order, so the result is deterministic. Sets in a
/// rename cycle (two objects swapping a value) cannot be ordered without a
/// temporary value; they keep their canonical order at the end of the block.
fn order_value_transfers(ops: &mut [PlannedOp], key: &impl Fn(ValueId) -> KeyId) {
    let Some(start) = ops.iter().position(|p| transfer(p, key).is_some()) else {
        return;
    };
    let len = ops[start..]
        .iter()
        .take_while(|p| transfer(p, key).is_some())
        .count();
    let block = &mut ops[start..start + len];
    let keys: Vec<_> = block.iter().filter_map(|p| transfer(p, key)).collect();
    let object = |p: &PlannedOp| match &p.op {
        Operation::SetAttribute { object, .. } => Some(*object),
        _ => None,
    };
    // releaser[v] = the ops releasing value v; no value releases nothing.
    let mut releasers: BTreeMap<&Transfer, Vec<usize>> = BTreeMap::new();
    for (i, (_, released)) in keys.iter().enumerate() {
        if released.1.is_some() {
            releasers.entry(released).or_default().push(i);
        }
    }
    // Edge r → c when r releases what c claims, on a different object.
    let mut blockers = vec![0usize; len];
    let mut unblocks: Vec<Vec<usize>> = vec![Vec::new(); len];
    for (c, (claimed, _)) in keys.iter().enumerate() {
        for &r in releasers.get(claimed).into_iter().flatten() {
            if r != c && object(&block[r]) != object(&block[c]) {
                blockers[c] += 1;
                unblocks[r].push(c);
            }
        }
    }
    let mut ready: BTreeSet<usize> = (0..len).filter(|&i| blockers[i] == 0).collect();
    let mut order = Vec::with_capacity(len);
    while let Some(i) = ready.pop_first() {
        order.push(i);
        for &c in &unblocks[i] {
            blockers[c] -= 1;
            if blockers[c] == 0 {
                ready.insert(c);
            }
        }
    }
    // A cycle leaves its members blocked: keep them, in canonical order.
    let placed: BTreeSet<usize> = order.iter().copied().collect();
    order.extend((0..len).filter(|i| !placed.contains(i)));
    let sorted: Vec<PlannedOp> = order.into_iter().map(|i| block[i].clone()).collect();
    block.clone_from_slice(&sorted);
}

/// Why no plan was derived.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// The target is not the current `"desired"` version.
    NotDesired(VersionId),
    /// The target is unknown.
    UnknownVersion(VersionId),
    /// The latest observation (`basis`) already holds `node`, which the
    /// desired version (`target`) creates, but with another kind. Kind is
    /// identity and no change alters it, so the create is neither done nor
    /// doable; simulate again from the new observation. (Every other field
    /// of an existing node converges through a compare-and-set change.)
    Unconvergeable {
        /// The latest observation.
        basis: VersionId,
        /// The desired version.
        target: VersionId,
        /// The node whose observed state cannot reach the desired one.
        node: Guid128,
    },
    /// The desired version changes `node` towards an unknown enabled flag or
    /// location. "Unknown" is an observation, not an intention: no actuator
    /// can write it, so no plan is derived.
    NotActuatable {
        /// The observation.
        basis: VersionId,
        /// The desired version.
        target: VersionId,
        /// The node.
        node: Guid128,
    },
    /// The latest observation and the desired version describe different
    /// directories (`DirectoryScope`). Hierarchy codes are only meaningful
    /// within one scope, so the two are not compared and no plan is derived.
    ScopeMismatch {
        /// The latest observation.
        basis: VersionId,
        /// The desired version.
        target: VersionId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change::NodeKind;

    /// Test values: `ValueId(k * 10 + variant)`, all variants of one `k`
    /// sharing comparison key `k` (like `b@x` and `B@x`).
    fn key(v: ValueId) -> KeyId {
        KeyId(v.0 / 10)
    }
    fn v(n: u32) -> Option<ValueId> {
        Some(ValueId(n))
    }

    #[test]
    fn a_repeated_change_is_planned_once() {
        let c = Change::RemoveMembership {
            user: Guid128([1; 16]),
            group: Guid128([2; 16]),
        };
        let p =
            ExecutionPlan::from_diff(VersionId(0), VersionId(1), vec![c.clone(), c], key).unwrap();
        assert_eq!(p.ops.len(), 1);
    }

    fn state(kind: NodeKind) -> NodeState {
        NodeState {
            kind,
            active: Some(true),
            upn: None,
            primary_smtp: None,
            dn: None,
        }
    }

    #[test]
    fn a_sorted_plan_frees_before_it_claims() {
        let g = |n| Guid128([n; 16]);
        let with_smtp = |smtp: u32| NodeState {
            primary_smtp: v(smtp),
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
                    state: with_smtp(70),
                },
                Change::SetAttribute {
                    node: g(3),
                    attribute: Attribute::PrimarySmtp,
                    from: v(30),
                    to: v(40),
                },
                Change::DeleteNode {
                    node: g(5),
                    state: with_smtp(70),
                },
                Change::RemoveMembership {
                    user: g(5),
                    group: g(9),
                },
            ],
            key,
        )
        .unwrap();
        let ops: Vec<_> = plan.ops.iter().map(|p| &p.op).collect();
        assert!(matches!(ops[0], Operation::RemoveGroupMember { member, .. } if *member == g(5)));
        // The delete frees value 70 before the create claims it.
        assert_eq!(ops[1], &Operation::DeleteObject { object: g(5) });
        assert!(matches!(ops[2], Operation::SetAttribute { object, .. } if *object == g(3)));
        assert!(matches!(ops[3], Operation::CreateObject { object, .. } if *object == g(1)));
        assert!(matches!(ops[4], Operation::AddGroupMember { member, .. } if *member == g(1)));
        assert_eq!(
            plan.ops[1].precondition,
            Precondition::ObjectRemovable(with_smtp(70))
        );
        assert_eq!(plan.ops[3].precondition, Precondition::ObjectAbsent);
    }

    fn rename(n: u8, from: u32, to: u32) -> Change {
        Change::SetAttribute {
            node: Guid128([n; 16]),
            attribute: Attribute::PrimarySmtp,
            from: v(from),
            to: v(to),
        }
    }
    fn renamed(plan: &ExecutionPlan) -> Vec<u8> {
        plan.ops
            .iter()
            .filter_map(|p| match &p.op {
                Operation::SetAttribute { object, .. } => Some(object.0[0]),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_value_is_released_before_another_object_claims_it() {
        // 1: a → B, 2: b → c, 3: C → d, where b/B and c/C are case variants
        // (distinct values, one key). Canonical order is 1, 2, 3; the safe
        // order is 3, 2, 1. The dependency is found through the key, so
        // case differences do not hide it.
        let plan = ExecutionPlan::from_diff(
            VersionId(0),
            VersionId(1),
            vec![rename(1, 10, 21), rename(2, 20, 30), rename(3, 31, 40)],
            key,
        )
        .unwrap();
        assert_eq!(renamed(&plan), vec![3, 2, 1]);
    }

    #[test]
    fn independent_renames_keep_canonical_order_and_a_cycle_is_kept() {
        let plan = ExecutionPlan::from_diff(
            VersionId(0),
            VersionId(1),
            vec![rename(2, 50, 60), rename(1, 70, 80)],
            key,
        )
        .unwrap();
        assert_eq!(renamed(&plan), vec![1, 2]);
        // A swap cannot be ordered: both stay, in canonical order.
        let swap = ExecutionPlan::from_diff(
            VersionId(0),
            VersionId(1),
            vec![rename(1, 10, 20), rename(2, 20, 10)],
            key,
        )
        .unwrap();
        assert_eq!(renamed(&swap), vec![1, 2]);
    }

    fn dn(l: &[u8]) -> Dn128 {
        Dn128::new(l).unwrap()
    }

    // SetActive / SetLocation lower to typed operations carrying the
    // observed value as their precondition (including an observed
    // `None`), and the result is the same whatever the input order.
    #[test]
    fn property_changes_lower_with_their_observed_precondition() {
        let g = |n| Guid128([n; 16]);
        let cs = vec![
            Change::SetLocation {
                node: g(2),
                from: None,
                to: Some(dn(&[4, 1])),
            },
            Change::SetActive {
                node: g(1),
                from: Some(true),
                to: Some(false),
            },
            Change::SetActive {
                node: g(3),
                from: None,
                to: Some(true),
            },
        ];
        let plan = ExecutionPlan::from_diff(VersionId(0), VersionId(1), cs.clone(), key).unwrap();
        let mut rev = cs;
        rev.reverse();
        assert_eq!(
            ExecutionPlan::from_diff(VersionId(0), VersionId(1), rev, key).unwrap(),
            plan,
            "deterministic"
        );
        assert_eq!(
            plan.ops,
            vec![
                PlannedOp {
                    op: Operation::SetEnabled {
                        object: g(1),
                        enabled: false
                    },
                    precondition: Precondition::EnabledEquals(Some(true)),
                },
                PlannedOp {
                    op: Operation::SetEnabled {
                        object: g(3),
                        enabled: true
                    },
                    precondition: Precondition::EnabledEquals(None),
                },
                PlannedOp {
                    op: Operation::MoveObject {
                        object: g(2),
                        to: dn(&[4, 1])
                    },
                    precondition: Precondition::LocationEquals(None),
                },
            ]
        );
    }

    // "Unknown" is never an operation: a change towards an unknown flag or
    // location refuses the whole plan instead of being dropped or guessed.
    #[test]
    fn a_change_towards_unknown_is_not_actuatable() {
        let g = Guid128([5; 16]);
        for c in [
            Change::SetActive {
                node: g,
                from: Some(true),
                to: None,
            },
            Change::SetLocation {
                node: g,
                from: Some(dn(&[1])),
                to: None,
            },
        ] {
            assert_eq!(
                ExecutionPlan::from_diff(
                    VersionId(0),
                    VersionId(1),
                    vec![rename(1, 10, 20), c],
                    key
                ),
                Err(PlanError::NotActuatable {
                    basis: VersionId(0),
                    target: VersionId(1),
                    node: g
                })
            );
        }
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
                    from: v(20),
                    to: v(30),
                },
                Change::AddMembership {
                    user: g(1),
                    group: g(9),
                },
            ],
            key,
        )
        .unwrap();
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
            Precondition::AttributeEquals(v(20))
        );
    }
}
