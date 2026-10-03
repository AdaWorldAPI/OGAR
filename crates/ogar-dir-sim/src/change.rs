//! The change algebra. One type serves twice: a rule *proposes* changes, a
//! diff *reports* them. An attribute change carries the value it expects to
//! replace (`from`): applying it where `from` no longer holds is refused, and
//! the same expectation becomes a plan operation's [`Precondition`](crate::Precondition).

use ogar_dir_core::Guid128;

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
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        /// Value the change expects to replace (raw, as observed).
        from: Option<String>,
        /// New value (raw).
        to: Option<String>,
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
