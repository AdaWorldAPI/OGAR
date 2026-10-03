//! Invariants over one version. Each failure is structured evidence (which
//! objects, which value), never a message string.

use crate::graph::{GraphState, NodeKind, normalize};
use ogar_dir_core::Guid128;
use std::collections::BTreeMap;

/// Which end of a membership edge is missing or of the wrong kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Endpoint {
    /// The member side.
    User,
    /// The group side.
    Group,
}

/// One invariant violation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Violation {
    /// Two or more active recipients own the same normalized primary SMTP.
    DuplicateSmtp {
        /// Normalized address.
        address: String,
        /// Every owner, ordered.
        owners: Vec<Guid128>,
    },
    /// Two or more active users own the same normalized UPN.
    DuplicateUpn {
        /// Normalized UPN.
        upn: String,
        /// Every owner, ordered.
        owners: Vec<Guid128>,
    },
    /// A membership edge references a missing or wrong-kind endpoint.
    DanglingMembership {
        /// Member side as recorded.
        user: Guid128,
        /// Group side as recorded.
        group: Guid128,
        /// Which side is broken.
        missing: Endpoint,
    },
}

fn duplicates(entries: impl Iterator<Item = (String, Guid128)>) -> Vec<(String, Vec<Guid128>)> {
    let mut by: BTreeMap<String, Vec<Guid128>> = BTreeMap::new();
    for (k, id) in entries {
        by.entry(k).or_default().push(id);
    }
    by.into_iter().filter(|(_, v)| v.len() > 1).collect()
}

/// All violations of `g`, deterministic order. Empty = valid.
pub fn validate(g: &GraphState) -> Vec<Violation> {
    let active_users = || {
        g.nodes()
            .filter(|(_, n)| n.kind == NodeKind::User && n.active)
    };
    let mut out = Vec::new();
    for (address, owners) in duplicates(
        active_users().filter_map(|(id, n)| Some((normalize(n.primary_smtp.as_ref()?), *id))),
    ) {
        out.push(Violation::DuplicateSmtp { address, owners });
    }
    for (upn, owners) in
        duplicates(active_users().filter_map(|(id, n)| Some((normalize(n.upn.as_ref()?), *id))))
    {
        out.push(Violation::DuplicateUpn { upn, owners });
    }
    for &(user, group) in g.memberships() {
        let is = |id: &Guid128, k| g.node(id).is_some_and(|n| n.kind == k);
        if !is(&user, NodeKind::User) {
            out.push(Violation::DanglingMembership {
                user,
                group,
                missing: Endpoint::User,
            });
        } else if !is(&group, NodeKind::Group) {
            out.push(Violation::DanglingMembership {
                user,
                group,
                missing: Endpoint::Group,
            });
        }
    }
    out.sort();
    out
}
