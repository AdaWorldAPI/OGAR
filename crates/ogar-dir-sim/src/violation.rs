//! Invariant violations as structured evidence — identities and the
//! offending comparison key, never a message string. Resolution to names
//! is an output projection done by whoever renders the violation.

use crate::change::KeyId;
use ogar_dir_core::Guid128;

/// Which end of a membership edge is missing or of the wrong kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Endpoint {
    /// The member side.
    User,
    /// The group side.
    Group,
}

/// Under which attribute a node holds an address. One address space spans
/// all of them: Exchange refuses an address another object already holds,
/// whichever attribute holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AddressRole {
    /// `userPrincipalName`.
    Upn,
    /// The primary SMTP address (`SMTP:` / `PrimarySmtpAddress`).
    PrimarySmtp,
    /// A secondary SMTP proxy (`smtp:`).
    SecondarySmtp,
    /// The `mail` attribute.
    Mail,
    /// A remote mailbox's routing address (`targetAddress`).
    Routing,
}

/// One invariant violation. `Ord` is total; a validator returns a sorted list.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Violation {
    /// Two or more active users own the same normalized primary SMTP.
    DuplicateSmtp {
        /// Comparison key of the address.
        key: KeyId,
        /// Every owner, sorted.
        owners: Vec<Guid128>,
    },
    /// Two or more active users own the same normalized UPN.
    DuplicateUpn {
        /// Comparison key of the UPN.
        key: KeyId,
        /// Every owner, sorted.
        owners: Vec<Guid128>,
    },
    /// Two or more objects hold the same normalized address under different
    /// attributes: a UPN or SMTP address that is another object's SMTP,
    /// `mail` or routing address, or a `mail` held by another object (an
    /// admin account whose `mail` points at someone's mailbox as a
    /// password-reset target, for example). Same-attribute collisions are
    /// [`Violation::DuplicateSmtp`] / [`Violation::DuplicateUpn`]; one object
    /// holding an address under several attributes is not a conflict.
    AddressConflict {
        /// Comparison key of the address.
        key: KeyId,
        /// Every holder with the attribute it holds the address under,
        /// sorted.
        holders: Vec<(Guid128, AddressRole)>,
    },
    /// A remote mailbox whose routing address is not one of its own SMTP
    /// proxies: Exchange routes to `targetAddress`, and the object must also
    /// own it (`smtp:{alias}@{tenant}.mail.onmicrosoft.com`).
    RoutingNotInProxies {
        /// The mailbox.
        node: Guid128,
    },
    /// A remote mailbox whose routing address is not
    /// `{alias}@{tenant}.mail.onmicrosoft.com` for its own `mailNickname`
    /// (see [`crate::exchange::ROUTING`]).
    RoutingMismatch {
        /// The mailbox.
        node: Guid128,
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
