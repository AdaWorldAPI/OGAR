//! The change algebra. One type serves twice: a rule *proposes* changes, a
//! diff *reports* them. An attribute change carries the value it expects to
//! replace (`from`): applying it where `from` no longer holds is refused, and
//! the same expectation becomes a plan operation's [`Precondition`](crate::Precondition).

use ogar_dir_core::{Guid128, OuHhtl};

/// Kind of a directory node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeKind {
    /// User.
    User,
    /// Group.
    Group,
}

/// The canonical semantic fields of one node — exactly what a snapshot row
/// holds, nothing else. A created node carries it as its content; a deleted
/// node carries it as the compare-and-set expectation (delete what was
/// read, never whatever is there now).
///
/// `active` is the single enabled flag as observed by the one source the
/// node came from (AD: `userAccountControl` ACCOUNTDISABLE clear). How AD
/// and Entra enabled state combine is not settled; see
/// `docs/DIRECTORY-SIMULATION-POC.md` V4.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeState {
    /// Kind.
    pub kind: NodeKind,
    /// Enabled. Users only: groups have no enabled flag and carry `true`.
    pub active: bool,
    /// Raw UPN.
    pub upn: Option<String>,
    /// Raw primary SMTP.
    pub primary_smtp: Option<String>,
    /// OU location (numeric HHTL, never a DN), if known.
    pub ou: Option<OuHhtl>,
}

/// Attribute a change can set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Attribute {
    /// userPrincipalName.
    Upn,
    /// Primary SMTP address.
    PrimarySmtp,
}

/// One semantic change. `Ord` is total, so a sorted change list is a
/// canonical form (used for determinism).
///
/// Variant order is load-bearing: a sorted list is also a safe application
/// order, respecting every dependency between kinds of change:
///
/// 1. `RemoveMembership` — a node's edges go before the node;
/// 2. `DeleteNode` — frees its UPN / SMTP before anything claims them;
/// 3. `SetAttribute` — renames away from (or into) addresses only after
///    deletes freed them and before creates take them;
/// 4. `CreateNode` — after every address it needs is free;
/// 5. `AddMembership` — once both endpoints exist.
///
/// Within step 3, the plan additionally orders a chain of renames (one
/// object releases the value the next claims). A cycle of renames (two
/// nodes swapping an address) needs a temporary value, which no change list
/// carries yet.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Change {
    /// `user` stops being a member of `group`.
    RemoveMembership {
        /// Member.
        user: Guid128,
        /// Group.
        group: Guid128,
    },
    /// A node stops existing. Refused while it still has a membership: the
    /// change list must remove those edges itself, so none is stranded.
    DeleteNode {
        /// Identity.
        node: Guid128,
        /// The state the deletion expects to remove (compare-and-set).
        state: NodeState,
    },
    /// Compare-and-set of one attribute.
    SetAttribute {
        /// Object.
        node: Guid128,
        /// Which attribute.
        attribute: Attribute,
        /// Value the change expects to replace (raw, as observed).
        from: Option<String>,
        /// New value (raw).
        to: Option<String>,
    },
    /// A node that did not exist comes into existence.
    CreateNode {
        /// Identity.
        node: Guid128,
        /// Its content.
        state: NodeState,
    },
    /// `user` becomes a member of `group`.
    AddMembership {
        /// Member.
        user: Guid128,
        /// Group.
        group: Guid128,
    },
}

/// Comparison form of a UPN / SMTP address: trimmed and lowercased with
/// Unicode case mapping (Exchange and Entra compare these
/// case-insensitively, and neither restricts them to ASCII).
///
/// ASCII-only lowercasing would keep `Ä` and `ä` apart, so two addresses the
/// directory treats as equal could both pass the uniqueness invariant.
/// This is lowercase mapping, not full case folding: `ß` and `ss` stay
/// distinct. No Unicode normalization is applied either: a precomposed `ä`
/// and `a` + U+0308 stay distinct.
pub fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn normalize_folds_non_ascii_case() {
        assert_eq!(
            normalize(" Änne@Example.Test "),
            normalize("änne@example.test")
        );
        assert_eq!(normalize("ÉLODIE@x.test"), "élodie@x.test");
        // Still distinguishes genuinely different addresses.
        assert_ne!(normalize("anne@x.test"), normalize("änne@x.test"));
    }
}
