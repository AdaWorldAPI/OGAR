//! Which roles a directory group carries: the authority's versioned policy.
//!
//! A group's membership is directory state, observed and versioned by the
//! directory. Which roles the group confers is policy, decided here: a grant
//! is made and revoked by someone, for a reason, at a time, independent of
//! what the directory looks like. Keeping the two apart means re-observing
//! the directory never changes a grant, and changing a grant never needs a
//! directory version.
//!
//! The policy is an append-only log. Each change is a [`PolicyVersion`] of
//! its own, numbered by this log and by nothing else, and every change keeps
//! the [`GrantProvenance`] it was made with. [`GroupRolePolicy::as_of`] reads
//! the state at any version.
//!
//! The state keeps "no grant data" and "no roles" apart. A group the policy
//! never named answers `None`: nothing was decided for it, which is not the
//! same as a decision that it carries no role. A group whose every grant was
//! revoked answers `Some([])`. A consumer such as dir-sim's `GroupRoles`
//! reads the two differently, and must, so roles are never inferred from
//! missing evidence.

use lance_graph_contract::rbac::RoleId;
use ogar_dir_core::Guid128;
use std::collections::BTreeMap;

/// A version of the group-role policy: the number of changes applied.
/// `PolicyVersion(0)` is the empty policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PolicyVersion(pub u64);

/// Who made a change, on what evidence, and when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantProvenance {
    /// The principal that made the change.
    pub by: String,
    /// The request, ticket or record it was made for.
    pub evidence: String,
    /// When it was made (unix ms).
    pub at_ms: i64,
}

/// One change to the policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyChange {
    /// `group` now confers `role`.
    Grant {
        /// The group, by objectGUID.
        group: Guid128,
        /// The role.
        role: RoleId,
    },
    /// `group` no longer confers `role`.
    Revoke {
        /// The group, by objectGUID.
        group: Guid128,
        /// The role.
        role: RoleId,
    },
}

/// A change as recorded: its version, the change and its provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyEntry {
    /// The version this change produced.
    pub version: PolicyVersion,
    /// The change.
    pub change: PolicyChange,
    /// Who made it, why and when.
    pub provenance: GrantProvenance,
}

/// Why a change was refused. Nothing is recorded for a refused change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyError {
    /// The group already confers the role.
    AlreadyGranted(Guid128, RoleId),
    /// The group does not confer the role.
    NotGranted(Guid128, RoleId),
    /// The policy has no such version.
    UnknownVersion(PolicyVersion),
}

/// The group-role policy: an append-only log of grants and revocations.
#[derive(Clone, Debug, Default)]
pub struct GroupRolePolicy {
    entries: Vec<PolicyEntry>,
    /// The state at [`Self::head`], kept current by [`Self::apply`] so a
    /// change is checked without replaying the log.
    head: GroupRoleState,
}

impl GroupRolePolicy {
    /// The empty policy.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The latest version.
    #[must_use]
    pub fn head(&self) -> PolicyVersion {
        PolicyVersion(self.entries.len() as u64)
    }

    /// Every change, in version order.
    #[must_use]
    pub fn history(&self) -> &[PolicyEntry] {
        &self.entries
    }

    /// Record `change` as the next version.
    ///
    /// # Errors
    ///
    /// [`PolicyError::AlreadyGranted`] for a grant the group already
    /// confers, [`PolicyError::NotGranted`] for a revocation of a role it
    /// does not: a change that would not change the state is not a change.
    pub fn apply(
        &mut self,
        change: PolicyChange,
        provenance: GrantProvenance,
    ) -> Result<PolicyVersion, PolicyError> {
        match change {
            PolicyChange::Grant { group, role } if self.head.confers(group, role) => {
                return Err(PolicyError::AlreadyGranted(group, role));
            }
            PolicyChange::Revoke { group, role } if !self.head.confers(group, role) => {
                return Err(PolicyError::NotGranted(group, role));
            }
            _ => {}
        }
        let version = PolicyVersion(self.entries.len() as u64 + 1);
        let entry = PolicyEntry {
            version,
            change,
            provenance,
        };
        self.head.step(&entry);
        self.entries.push(entry);
        Ok(version)
    }

    /// The policy as of `version`.
    ///
    /// # Errors
    ///
    /// [`PolicyError::UnknownVersion`] past [`Self::head`].
    pub fn as_of(&self, version: PolicyVersion) -> Result<GroupRoleState, PolicyError> {
        let n = usize::try_from(version.0)
            .ok()
            .filter(|&n| n <= self.entries.len())
            .ok_or(PolicyError::UnknownVersion(version))?;
        if n == self.entries.len() {
            return Ok(self.head.clone());
        }
        let mut state = GroupRoleState::default();
        for e in &self.entries[..n] {
            state.step(e);
        }
        Ok(state)
    }
}

/// The policy at one version.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroupRoleState {
    /// Every group the policy has named, with the roles it confers and the
    /// provenance of each standing grant. A group whose grants were all
    /// revoked stays, with no roles.
    groups: BTreeMap<Guid128, BTreeMap<RoleId, GrantProvenance>>,
}

impl GroupRoleState {
    /// Apply one recorded change. The only place a change alters a state,
    /// so the cached head and a replay in [`GroupRolePolicy::as_of`] agree.
    fn step(&mut self, e: &PolicyEntry) {
        match e.change {
            PolicyChange::Grant { group, role } => {
                self.groups
                    .entry(group)
                    .or_default()
                    .insert(role, e.provenance.clone());
            }
            PolicyChange::Revoke { group, role } => {
                self.groups.entry(group).or_default().remove(role);
            }
        }
    }

    /// The roles `group` confers: `None` when the policy never named it,
    /// `Some` (possibly empty) when it did.
    #[must_use]
    pub fn roles_of(&self, group: Guid128) -> Option<Vec<RoleId>> {
        self.groups.get(&group).map(|r| r.keys().copied().collect())
    }

    /// Whether `group` confers `role`.
    #[must_use]
    pub fn confers(&self, group: Guid128, role: RoleId) -> bool {
        self.groups
            .get(&group)
            .is_some_and(|r| r.contains_key(role))
    }

    /// The provenance of the standing grant of `role` to `group`.
    #[must_use]
    pub fn provenance(&self, group: Guid128, role: RoleId) -> Option<&GrantProvenance> {
        self.groups.get(&group).and_then(|r| r.get(role))
    }

    /// Every group the policy has named, with its roles, in GUID order.
    pub fn groups(&self) -> impl Iterator<Item = (Guid128, Vec<RoleId>)> + '_ {
        self.groups
            .iter()
            .map(|(g, r)| (*g, r.keys().copied().collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(n: u8) -> Guid128 {
        Guid128([n; 16])
    }

    fn by(who: &str, evidence: &str, at_ms: i64) -> GrantProvenance {
        GrantProvenance {
            by: who.into(),
            evidence: evidence.into(),
            at_ms,
        }
    }

    const FINANCE: u8 = 0x62;

    // A group never named answers None; a group whose every grant was
    // revoked answers Some(empty). The two are not the same answer.
    #[test]
    fn a_revoked_group_is_empty_not_unread() {
        let mut p = GroupRolePolicy::new();
        let granted = p
            .apply(
                PolicyChange::Grant {
                    group: g(FINANCE),
                    role: "accountant",
                },
                by("admin", "REQ-1", 1),
            )
            .unwrap();
        let revoked = p
            .apply(
                PolicyChange::Revoke {
                    group: g(FINANCE),
                    role: "accountant",
                },
                by("admin", "REQ-2", 2),
            )
            .unwrap();
        let now = p.as_of(revoked).unwrap();
        assert_eq!(now.roles_of(g(FINANCE)), Some(vec![]));
        assert_eq!(now.roles_of(g(0x99)), None);
        assert_eq!(
            p.as_of(granted).unwrap().roles_of(g(FINANCE)),
            Some(vec!["accountant"])
        );
        assert_eq!(
            p.as_of(PolicyVersion(0)).unwrap().roles_of(g(FINANCE)),
            None
        );
    }

    // A standing grant keeps the provenance it was made with; a re-grant
    // after a revocation carries its own, and the log keeps all three.
    #[test]
    fn each_grant_keeps_its_own_provenance() {
        let mut p = GroupRolePolicy::new();
        let grant = PolicyChange::Grant {
            group: g(FINANCE),
            role: "accountant",
        };
        let v1 = p.apply(grant, by("alice", "REQ-1", 10)).unwrap();
        p.apply(
            PolicyChange::Revoke {
                group: g(FINANCE),
                role: "accountant",
            },
            by("bob", "REQ-2", 20),
        )
        .unwrap();
        let v3 = p.apply(grant, by("carol", "REQ-3", 30)).unwrap();
        assert_eq!(
            p.as_of(v1).unwrap().provenance(g(FINANCE), "accountant"),
            Some(&by("alice", "REQ-1", 10))
        );
        assert_eq!(
            p.as_of(v3).unwrap().provenance(g(FINANCE), "accountant"),
            Some(&by("carol", "REQ-3", 30))
        );
        assert_eq!(p.history().len(), 3);
        // The cached head is the replayed state, not a second answer.
        let mut replay = GroupRoleState::default();
        for e in p.history() {
            replay.step(e);
        }
        assert_eq!(p.as_of(p.head()).unwrap(), replay);
        assert_eq!(p.history()[1].provenance.by, "bob");
    }

    // A change that would not change the state is refused and not recorded.
    #[test]
    fn a_no_op_change_is_refused() {
        let mut p = GroupRolePolicy::new();
        let grant = PolicyChange::Grant {
            group: g(FINANCE),
            role: "accountant",
        };
        let revoke = PolicyChange::Revoke {
            group: g(FINANCE),
            role: "accountant",
        };
        assert_eq!(
            p.apply(revoke, by("a", "x", 1)),
            Err(PolicyError::NotGranted(g(FINANCE), "accountant"))
        );
        p.apply(grant, by("a", "x", 1)).unwrap();
        assert_eq!(
            p.apply(grant, by("a", "x", 2)),
            Err(PolicyError::AlreadyGranted(g(FINANCE), "accountant"))
        );
        assert_eq!(p.head(), PolicyVersion(1));
        assert_eq!(
            p.as_of(PolicyVersion(2)),
            Err(PolicyError::UnknownVersion(PolicyVersion(2)))
        );
    }
}
