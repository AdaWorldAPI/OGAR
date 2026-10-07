//! The change algebra. One type serves twice: a rule *proposes* changes, a
//! diff *reports* them. An attribute change carries the value it expects to
//! replace (`from`): applying it where `from` no longer holds is refused, and
//! the same expectation becomes a plan operation's [`Precondition`](crate::Precondition).
//!
//! ## No strings past ingress
//!
//! ```text
//!   external String ──► ingress (normalize) ──► cold label/value store
//!                                                 │ stable ValueId / KeyId
//!   ════════════════════ numeric substrate ════════╧═══════════════════════
//!     Change · NodeState · Violation · ExecutionPlan carry ids only
//!   ═══════════════════════════════════════════════════════════════════════
//!   actuator / UI / evidence formatting ──► resolve id ──► String
//! ```
//!
//! A [`ValueId`] names one exact raw value; a [`KeyId`] names its comparison
//! form ([`normalize`]). Both are issued by the store that owns the
//! observations, append-only, so an id keeps its meaning from observation
//! through simulation, the desired version, reconciliation and the plan. They
//! are not snapshot ordinals.

use ogar_dir_core::{Dn128, Guid128};

/// Stable identity of one exact raw attribute value (case and spacing kept).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueId(pub u32);

/// Stable identity of a value's comparison form ([`normalize`]): two values
/// the directory treats as equal share one key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyId(pub u32);

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
/// `active` is three-valued: `Some(true)` enabled, `Some(false)` disabled,
/// `None` unknown (no source reported a flag). An unknown flag is never read
/// as enabled. Several sources combine through [`effective_active`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeState {
    /// Kind.
    pub kind: NodeKind,
    /// Enabled; `None` = unknown. Users only: groups have no enabled flag
    /// and carry `Some(true)`.
    pub active: Option<bool>,
    /// UPN.
    pub upn: Option<ValueId>,
    /// Primary SMTP.
    pub primary_smtp: Option<ValueId>,
    /// Hierarchy location (numeric, within the store's directory scope;
    /// never a DN), if known.
    pub dn: Option<Dn128>,
}

/// The one definition of "active" across sources (V4, decided 2026-10-07):
/// **active iff at least one source is known and no known source says
/// disabled. An unknown source abstains.**
///
/// ```text
/// known_any ∧ ¬known_disabled
/// = (ad_known ∨ entra_known) ∧ (¬ad_known ∨ ad_enabled) ∧ (¬entra_known ∨ entra_enabled)
/// ```
///
/// | AD | Entra | effective |
/// |---|---|---|
/// | enabled | enabled | active |
/// | enabled | unknown | active |
/// | unknown | enabled | active |
/// | disabled | enabled | inactive |
/// | enabled | disabled | inactive |
/// | disabled | unknown | inactive |
/// | unknown | disabled | inactive |
/// | unknown | unknown | unknown |
///
/// Absence of evidence never disables; negative evidence always does; two
/// unknowns stay unknown rather than collapsing to either value. Every
/// executor that derives "active" from more than one source must agree with
/// this function row for row.
#[must_use]
pub const fn effective_active(ad: Option<bool>, entra: Option<bool>) -> Option<bool> {
    match (ad, entra) {
        (None, None) => None,
        (Some(false), _) | (_, Some(false)) => Some(false),
        _ => Some(true),
    }
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
        /// Value the change expects to replace (as observed).
        from: Option<ValueId>,
        /// New value.
        to: Option<ValueId>,
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

/// Comparison form of a UPN / SMTP address, computed once at ingress (the
/// store maps each [`ValueId`] to its [`KeyId`]); never during execution: trimmed and lowercased with
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
    use super::effective_active;

    /// The full 3×3 table, every row. A combinator that read unknown as
    /// enabled (the old `from_ad` default) fails the `(None, None)` row; one
    /// that read it as disabled fails the four rows with a single known
    /// `true`.
    #[test]
    fn effective_active_is_the_v4_table() {
        let (e, d, u) = (Some(true), Some(false), None);
        let rows = [
            (e, e, e),
            (e, u, e),
            (u, e, e),
            (d, e, d),
            (e, d, d),
            (d, u, d),
            (u, d, d),
            (d, d, d),
            (u, u, u),
        ];
        for (ad, entra, want) in rows {
            assert_eq!(
                effective_active(ad, entra),
                want,
                "ad={ad:?} entra={entra:?}"
            );
            // Source order does not matter.
            assert_eq!(effective_active(entra, ad), want, "swapped");
        }
    }

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
