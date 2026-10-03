//! Invariant violations as structured evidence — identities and the
//! offending normalized value, never a message string. Resolution to names
//! is an output projection done by whoever renders the violation.

use ogar_dir_core::Guid128;

/// Which end of a membership edge is missing or of the wrong kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Endpoint {
    /// The member side.
    User,
    /// The group side.
    Group,
}

/// One invariant violation. `Ord` is total; a validator returns a sorted list.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Violation {
    /// Two or more active users own the same normalized primary SMTP.
    DuplicateSmtp {
        /// Normalized address.
        address: String,
        /// Every owner, sorted.
        owners: Vec<Guid128>,
    },
    /// Two or more active users own the same normalized UPN.
    DuplicateUpn {
        /// Normalized UPN.
        upn: String,
        /// Every owner, sorted.
        owners: Vec<Guid128>,
    },
    /// A membership edge references a missing or wrong-kind endpoint.
    DanglingMembership {
        /// Member side as recorded.
        user: Guid128,
        /// Group side as recorded.
        group: Guid128,
        /// Which side is broken (the user side is reported first).
        missing: Endpoint,
    },
}
