//! The semantic directory graph of ONE version, and the change algebra.
//!
//! This is directory meaning (users, groups, UPN, primary SMTP, membership),
//! not storage geometry. The 512-byte `DirRecord` is how an observation is
//! stored; this is what a rule reasons about and what a diff reports.
//!
//! [`Change`] is used twice on purpose: a rule *proposes* changes, and
//! [`GraphState::diff`] *reports* changes. Every attribute change carries the
//! value it expects to replace (`from`), so a change is a compare-and-set:
//! applying it to a state where `from` no longer holds is refused. That same
//! expectation is what a future actuator checks against reality before acting.

use ogar_dir_core::Guid128;
use std::collections::{BTreeMap, BTreeSet};

/// Kind of directory object this graph models.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeKind {
    /// A user (a recipient when it has a primary SMTP address).
    User,
    /// A group.
    Group,
}

/// One directory object. Values are stored as observed (raw); comparisons
/// for uniqueness use [`normalize`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// User or group.
    pub kind: NodeKind,
    /// Display name (explanatory only; never identity).
    pub name: String,
    /// False for disabled accounts; inactive nodes do not own addresses.
    pub active: bool,
    /// userPrincipalName.
    pub upn: Option<String>,
    /// Primary SMTP address (the `SMTP:` proxy).
    pub primary_smtp: Option<String>,
}

impl Node {
    /// Active user with UPN and primary SMTP.
    pub fn user(name: &str, upn: &str, smtp: &str) -> Self {
        Self {
            kind: NodeKind::User,
            name: name.into(),
            active: true,
            upn: Some(upn.into()),
            primary_smtp: Some(smtp.into()),
        }
    }
    /// Group without addresses.
    pub fn group(name: &str) -> Self {
        Self {
            kind: NodeKind::Group,
            name: name.into(),
            active: true,
            upn: None,
            primary_smtp: None,
        }
    }
}

/// Comparison form of a UPN / SMTP address: ASCII-lowercased and trimmed.
/// (Exchange and Entra compare these case-insensitively.)
pub fn normalize(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

/// Attribute a change can set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Attribute {
    /// userPrincipalName.
    Upn,
    /// Primary SMTP address.
    PrimarySmtp,
}

/// One semantic change. Ordering is total so diffs are deterministic.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Change {
    /// `user` becomes a member of `group`.
    AddMembership {
        /// Member.
        user: Guid128,
        /// Group.
        group: Guid128,
    },
    /// `user` stops being a member of `group`.
    RemoveMembership {
        /// Member.
        user: Guid128,
        /// Group.
        group: Guid128,
    },
    /// Compare-and-set of one attribute.
    SetAttribute {
        /// Object.
        node: Guid128,
        /// Which attribute.
        attribute: Attribute,
        /// Value the change expects to replace.
        from: Option<String>,
        /// New value.
        to: Option<String>,
    },
}

/// Why a change could not be applied to a state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyError {
    /// A `SetAttribute` named a node that does not exist.
    UnknownNode(Guid128),
    /// A `SetAttribute`'s `from` does not match the current value (stale).
    Stale {
        /// Node.
        node: Guid128,
        /// Attribute.
        attribute: Attribute,
        /// What the change expected.
        expected: Option<String>,
        /// What the state holds.
        actual: Option<String>,
    },
    /// The membership already holds (add) / does not hold (remove).
    NoOp(Change),
}

/// Full semantic state of one version.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphState {
    nodes: BTreeMap<Guid128, Node>,
    /// `(user, group)` pairs. Endpoints are NOT required to exist here — a
    /// dangling edge is representable so validation can report it.
    members: BTreeSet<(Guid128, Guid128)>,
}

impl GraphState {
    /// Empty graph.
    pub fn new() -> Self {
        Self::default()
    }
    /// Insert or replace a node (observation building only).
    pub fn put_node(&mut self, id: Guid128, node: Node) {
        self.nodes.insert(id, node);
    }
    /// Insert a membership (observation building only).
    pub fn put_membership(&mut self, user: Guid128, group: Guid128) {
        self.members.insert((user, group));
    }
    /// Node by id.
    pub fn node(&self, id: &Guid128) -> Option<&Node> {
        self.nodes.get(id)
    }
    /// All nodes, ordered by id.
    pub fn nodes(&self) -> impl Iterator<Item = (&Guid128, &Node)> {
        self.nodes.iter()
    }
    /// All `(user, group)` memberships, ordered.
    pub fn memberships(&self) -> impl Iterator<Item = &(Guid128, Guid128)> {
        self.members.iter()
    }
    /// Membership test.
    pub fn is_member(&self, user: Guid128, group: Guid128) -> bool {
        self.members.contains(&(user, group))
    }

    fn attr(&self, id: &Guid128, a: Attribute) -> Result<Option<String>, ApplyError> {
        let n = self.nodes.get(id).ok_or(ApplyError::UnknownNode(*id))?;
        Ok(match a {
            Attribute::Upn => n.upn.clone(),
            Attribute::PrimarySmtp => n.primary_smtp.clone(),
        })
    }

    /// Apply one change. Pure: returns a new state, `self` untouched.
    pub fn apply(&self, c: &Change) -> Result<Self, ApplyError> {
        let mut next = self.clone();
        match c {
            Change::AddMembership { user, group } => {
                if !next.members.insert((*user, *group)) {
                    return Err(ApplyError::NoOp(c.clone()));
                }
            }
            Change::RemoveMembership { user, group } => {
                if !next.members.remove(&(*user, *group)) {
                    return Err(ApplyError::NoOp(c.clone()));
                }
            }
            Change::SetAttribute {
                node,
                attribute,
                from,
                to,
            } => {
                let actual = self.attr(node, *attribute)?;
                if &actual != from {
                    return Err(ApplyError::Stale {
                        node: *node,
                        attribute: *attribute,
                        expected: from.clone(),
                        actual,
                    });
                }
                let n = next.nodes.get_mut(node).expect("checked by attr");
                match attribute {
                    Attribute::Upn => n.upn = to.clone(),
                    Attribute::PrimarySmtp => n.primary_smtp = to.clone(),
                }
            }
        }
        Ok(next)
    }

    /// Apply a change set in order (all or nothing).
    pub fn apply_all(&self, cs: &[Change]) -> Result<Self, ApplyError> {
        cs.iter().try_fold(self.clone(), |s, c| s.apply(c))
    }

    /// Net semantic difference `self → other`, deterministic order.
    /// Applying the result to `self` yields `other` (for the modelled
    /// attributes and memberships; node creation/deletion is out of scope
    /// for this slice and reported by [`GraphState::node_set_differs`]).
    pub fn diff(&self, other: &Self) -> Vec<Change> {
        let mut out = Vec::new();
        for (user, group) in other.members.difference(&self.members) {
            out.push(Change::AddMembership {
                user: *user,
                group: *group,
            });
        }
        for (user, group) in self.members.difference(&other.members) {
            out.push(Change::RemoveMembership {
                user: *user,
                group: *group,
            });
        }
        for (id, a) in &self.nodes {
            let Some(b) = other.nodes.get(id) else {
                continue;
            };
            for (attr, x, y) in [
                (Attribute::Upn, &a.upn, &b.upn),
                (Attribute::PrimarySmtp, &a.primary_smtp, &b.primary_smtp),
            ] {
                if x != y {
                    out.push(Change::SetAttribute {
                        node: *id,
                        attribute: attr,
                        from: x.clone(),
                        to: y.clone(),
                    });
                }
            }
        }
        out.sort();
        out
    }

    /// True if the two states disagree on which nodes exist (not expressible
    /// as a [`Change`] in this slice).
    pub fn node_set_differs(&self, other: &Self) -> bool {
        !self.nodes.keys().eq(other.nodes.keys())
    }
}
