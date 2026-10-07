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

/// Dictionary-backed attribute a [`Change::SetAttribute`] can set. Its
/// values are [`ValueId`]s; the enabled flag and the location are not
/// dictionary values and have their own typed compare-and-set changes
/// ([`Change::SetActive`], [`Change::SetLocation`]).
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
/// 4. `SetActive`, `SetLocation` — properties of a node that exists before
///    and after; neither frees nor claims a value, so no other change
///    depends on them;
/// 5. `CreateNode` — after every address it needs is free;
/// 6. `AddMembership` — once both endpoints exist.
///
/// ## Every property of an existing node, and `kind`
///
/// A node present in both versions differs only through compare-and-set
/// changes: UPN and primary SMTP ([`Change::SetAttribute`]), the enabled
/// flag ([`Change::SetActive`]) and the location ([`Change::SetLocation`]).
/// [`NodeState::apply`] is their one reference semantics.
///
/// **`kind` is identity, not a property, and no change alters it.** Users
/// and groups are separate populations with separate ordinal spaces, a
/// membership is typed (`user`, `group`) by its endpoints, and a directory
/// object does not turn from a user into a group. A different kind under the
/// same identity is a different object, so it is not representable as a
/// mutation — and re-creating an identity under another kind is refused by
/// the executor, never planned.
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
    /// Compare-and-set of the enabled flag, all three values: `Some(true)`
    /// enabled, `Some(false)` disabled, `None` unknown. Users only (a group
    /// has no flag). A diff reports `→ None` when a later observation stops
    /// reporting the flag; no plan can actuate it (an actuator cannot write
    /// "unknown").
    SetActive {
        /// Object.
        node: Guid128,
        /// Flag the change expects to replace (as observed).
        from: Option<bool>,
        /// New flag.
        to: Option<bool>,
    },
    /// Compare-and-set of the hierarchy location (numeric; never a DN).
    /// `None` = unknown location. A move to `None` is reportable by a diff
    /// but not actuatable.
    SetLocation {
        /// Object.
        node: Guid128,
        /// Location the change expects to replace (as observed).
        from: Option<Dn128>,
        /// New location.
        to: Option<Dn128>,
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

/// Why [`NodeState::apply`] refused a change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The change is not a property change of one existing node
    /// (memberships, creates and deletes act on the directory, not on a
    /// node's state).
    NotAPropertyChange,
    /// The node no longer holds the value the change expects to replace.
    Stale,
    /// [`Change::SetActive`] on a group: groups carry no enabled flag.
    NoEnabledFlag,
}

impl NodeState {
    /// The reference semantics of a compare-and-set property change on one
    /// existing node: the state after it, or why it is refused. Every
    /// executor must agree with it.
    ///
    /// The change applies only if the node still holds its `from`;
    /// otherwise it is [`Refusal::Stale`], whatever `to` is. `kind` is never
    /// changed (no change can express it).
    ///
    /// # Errors
    ///
    /// [`Refusal`].
    pub fn apply(&self, c: &Change) -> Result<Self, Refusal> {
        let mut next = self.clone();
        match c {
            Change::SetAttribute {
                attribute,
                from,
                to,
                ..
            } => {
                let slot = match attribute {
                    Attribute::Upn => &mut next.upn,
                    Attribute::PrimarySmtp => &mut next.primary_smtp,
                };
                if slot != from {
                    return Err(Refusal::Stale);
                }
                *slot = *to;
            }
            Change::SetActive { from, to, .. } => {
                if self.kind == NodeKind::Group {
                    return Err(Refusal::NoEnabledFlag);
                }
                if self.active != *from {
                    return Err(Refusal::Stale);
                }
                next.active = *to;
            }
            Change::SetLocation { from, to, .. } => {
                if self.dn != *from {
                    return Err(Refusal::Stale);
                }
                next.dn = *to;
            }
            Change::RemoveMembership { .. }
            | Change::DeleteNode { .. }
            | Change::CreateNode { .. }
            | Change::AddMembership { .. } => return Err(Refusal::NotAPropertyChange),
        }
        Ok(next)
    }
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

    use super::{Attribute, Change, NodeKind, NodeState, Refusal, ValueId};
    use ogar_dir_core::{Dn128, Guid128};

    const N: Guid128 = Guid128([7; 16]);
    const STATES: [Option<bool>; 3] = [Some(true), Some(false), None];

    fn user(active: Option<bool>, dn: Option<Dn128>) -> NodeState {
        NodeState {
            kind: NodeKind::User,
            active,
            upn: Some(ValueId(1)),
            primary_smtp: Some(ValueId(2)),
            dn,
        }
    }
    fn dn(l: &[u8]) -> Option<Dn128> {
        Some(Dn128::new(l).unwrap())
    }
    fn set_active(from: Option<bool>, to: Option<bool>) -> Change {
        Change::SetActive { node: N, from, to }
    }
    fn set_location(from: Option<Dn128>, to: Option<Dn128>) -> Change {
        Change::SetLocation { node: N, from, to }
    }

    // Every one of the nine (from, to) pairs over the three flag values:
    // applies when `from` is current, and lands exactly on `to` — `None`
    // stays `None`, never collapsing to either known value. A change whose
    // `from` is any OTHER value is stale, whatever its `to`.
    #[test]
    fn set_active_is_compare_and_set_over_all_three_values() {
        for current in STATES {
            for from in STATES {
                for to in STATES {
                    let got = user(current, None).apply(&set_active(from, to));
                    if from == current {
                        assert_eq!(got, Ok(user(to, None)), "{current:?}: {from:?}->{to:?}");
                    } else {
                        assert_eq!(got, Err(Refusal::Stale), "{current:?}: {from:?}->{to:?}");
                    }
                }
            }
        }
    }

    // A group has no flag: even a "no-op" `Some(true) -> Some(true)` is
    // refused, so no executor can grow a group flag lane.
    #[test]
    fn a_group_has_no_enabled_flag_to_set() {
        let group = NodeState {
            kind: NodeKind::Group,
            active: Some(true),
            upn: None,
            primary_smtp: None,
            dn: None,
        };
        for (from, to) in [(Some(true), Some(false)), (Some(true), Some(true))] {
            assert_eq!(
                group.apply(&set_active(from, to)),
                Err(Refusal::NoEnabledFlag)
            );
        }
        // Its location is a property like any other.
        assert_eq!(
            group.apply(&set_location(None, dn(&[3]))).map(|s| s.dn),
            Ok(dn(&[3]))
        );
    }

    #[test]
    fn set_location_moves_between_known_and_unknown() {
        let (a, b) = (dn(&[0, 1]), dn(&[2]));
        for (from, to) in [(None, a), (a, None), (a, b)] {
            assert_eq!(
                user(None, from).apply(&set_location(from, to)),
                Ok(user(None, to))
            );
        }
        // Stale: expected A while the node is at B, or expected a known
        // location while it is unknown (and the other way round).
        for (current, from) in [(b, a), (None, a), (a, None)] {
            assert_eq!(
                user(None, current).apply(&set_location(from, b)),
                Err(Refusal::Stale)
            );
        }
    }

    #[test]
    fn set_attribute_is_compare_and_set() {
        let ok = Change::SetAttribute {
            node: N,
            attribute: Attribute::Upn,
            from: Some(ValueId(1)),
            to: Some(ValueId(9)),
        };
        let mut want = user(None, None);
        want.upn = Some(ValueId(9));
        assert_eq!(user(None, None).apply(&ok), Ok(want));
        let stale = Change::SetAttribute {
            node: N,
            attribute: Attribute::PrimarySmtp,
            from: Some(ValueId(1)),
            to: None,
        };
        assert_eq!(user(None, None).apply(&stale), Err(Refusal::Stale));
    }

    // `kind` is identity: no property change alters it, from either kind,
    // and the changes that are not property changes are refused here.
    #[test]
    fn no_change_alters_kind() {
        for kind in [NodeKind::User, NodeKind::Group] {
            let s = NodeState {
                kind,
                ..user(Some(true), dn(&[1]))
            };
            for c in [
                set_active(Some(true), Some(false)),
                set_location(dn(&[1]), dn(&[2])),
                Change::SetAttribute {
                    node: N,
                    attribute: Attribute::Upn,
                    from: Some(ValueId(1)),
                    to: None,
                },
            ] {
                if let Ok(next) = s.apply(&c) {
                    assert_eq!(next.kind, kind);
                }
            }
            for c in [
                Change::CreateNode {
                    node: N,
                    state: s.clone(),
                },
                Change::DeleteNode {
                    node: N,
                    state: s.clone(),
                },
                Change::AddMembership { user: N, group: N },
                Change::RemoveMembership { user: N, group: N },
            ] {
                assert_eq!(s.apply(&c), Err(Refusal::NotAPropertyChange));
            }
        }
    }

    // The sorted order places the two new property changes after renames
    // and before creates.
    #[test]
    fn property_changes_sort_between_renames_and_creates() {
        let mut cs = [
            Change::CreateNode {
                node: N,
                state: user(None, None),
            },
            set_location(None, dn(&[1])),
            set_active(None, Some(true)),
            Change::SetAttribute {
                node: N,
                attribute: Attribute::Upn,
                from: None,
                to: None,
            },
        ];
        cs.sort();
        assert!(matches!(cs[0], Change::SetAttribute { .. }));
        assert!(matches!(cs[1], Change::SetActive { .. }));
        assert!(matches!(cs[2], Change::SetLocation { .. }));
        assert!(matches!(cs[3], Change::CreateNode { .. }));
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
