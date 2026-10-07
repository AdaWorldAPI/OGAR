//! Exchange hybrid recipients, as the on-premises directory records them.
//!
//! Three attributes say what an object is to Exchange:
//!
//! * `msExchRemoteRecipientType` — flags for an Exchange Online (remote)
//!   mailbox seen from on-premises: provision / migrated / deprovision of the
//!   mailbox, provision / deprovision of the archive, room / equipment
//!   (both = shared);
//! * `msExchRecipientDisplayType` — a signed display class;
//! * `msExchRecipientTypeDetails` — a 64-bit type code.
//!
//! plus `targetAddress`, which for a remote mailbox is its routing address
//! (`alias@tenant.mail.onmicrosoft.com`).
//!
//! The decode accepts exactly the combinations the hybrid lifecycle
//! produces (the 26 `msExchRemoteRecipientType` values of
//! `jahube/PShell RemoteMailbox-Archive/Enable-Remotemailbox.PS1`, with the
//! type and display codes that go with them). Anything else is kept raw as
//! [`Recipient::Other`] and never guessed into a nearby state.
//!
//! **The enabled flag is not part of this.** A shared, room or equipment
//! mailbox is a disabled account by design and is still a recipient.

use crate::change::ValueId;

/// `msExchRemoteRecipientType` flag bits.
pub mod flag {
    /// ProvisionMailbox — a remote mailbox created in Exchange Online.
    pub const PROVISION_MAILBOX: u32 = 1;
    /// ProvisionArchive.
    pub const PROVISION_ARCHIVE: u32 = 2;
    /// Migrated — the mailbox was moved to Exchange Online.
    pub const MIGRATED: u32 = 4;
    /// DeprovisionMailbox.
    pub const DEPROVISION_MAILBOX: u32 = 8;
    /// DeprovisionArchive.
    pub const DEPROVISION_ARCHIVE: u32 = 16;
    /// RoomMailbox.
    pub const ROOM: u32 = 32;
    /// EquipmentMailbox.
    pub const EQUIPMENT: u32 = 64;
    /// SharedMailbox (room and equipment bits together).
    pub const SHARED: u32 = ROOM | EQUIPMENT;
}

/// `msExchRecipientTypeDetails` values this vocabulary decodes.
pub mod type_details {
    /// On-premises user mailbox.
    pub const USER_MAILBOX: u64 = 1;
    /// RemoteUserMailbox.
    pub const REMOTE_USER_MAILBOX: u64 = 2_147_483_648;
    /// RemoteRoomMailbox.
    pub const REMOTE_ROOM_MAILBOX: u64 = 8_589_934_592;
    /// RemoteEquipmentMailbox.
    pub const REMOTE_EQUIPMENT_MAILBOX: u64 = 17_179_869_184;
    /// RemoteSharedMailbox.
    pub const REMOTE_SHARED_MAILBOX: u64 = 34_359_738_368;
}

/// `msExchRecipientDisplayType` values this vocabulary decodes.
pub mod display_type {
    /// On-premises (ACL-able) mailbox user.
    pub const MAILBOX_USER: i32 = 1_073_741_824;
    /// Synced remote user (and shared) mailbox.
    pub const REMOTE_USER_MAILBOX: i32 = -2_147_483_642;
    /// Synced remote room mailbox.
    pub const REMOTE_ROOM_MAILBOX: i32 = -2_147_481_850;
    /// Synced remote equipment mailbox.
    pub const REMOTE_EQUIPMENT_MAILBOX: i32 = -2_147_481_594;
}

/// The four recipient attributes as stored. `None` = attribute absent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecipientAttributes {
    /// `msExchRemoteRecipientType`.
    pub remote_recipient_type: Option<u32>,
    /// `msExchRecipientDisplayType`.
    pub display_type: Option<i32>,
    /// `msExchRecipientTypeDetails`.
    pub type_details: Option<u64>,
    /// `targetAddress` (the address, without its `SMTP:` prefix).
    pub target_address: Option<ValueId>,
}

/// What a remote mailbox is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RemoteKind {
    /// A user's mailbox.
    User,
    /// A room.
    Room,
    /// Equipment.
    Equipment,
    /// A shared mailbox.
    Shared,
}

/// Where the remote mailbox is in its lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MailboxState {
    /// Created directly in Exchange Online (`Enable-RemoteMailbox`).
    Provisioned,
    /// Moved from on-premises to Exchange Online.
    Migrated,
    /// Removed (`Disable-RemoteMailbox`); the object is no longer a mail
    /// recipient, the flags tell the sync to remove the cloud mailbox.
    Deprovisioned,
}

/// The archive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArchiveState {
    /// No archive was ever provisioned.
    None,
    /// An Exchange Online archive.
    Provisioned,
    /// The archive was removed.
    Deprovisioned,
}

/// A remote mailbox state that the hybrid lifecycle can produce. Built only
/// through [`RemoteMailbox::new`], so an unreachable combination cannot be
/// represented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RemoteMailbox {
    kind: RemoteKind,
    mailbox: MailboxState,
    archive: ArchiveState,
    routing: Option<ValueId>,
}

/// One object's role in Exchange.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Recipient {
    /// No recipient attribute at all.
    NotMailEnabled,
    /// A mailbox hosted on-premises, with its Exchange Online archive state
    /// (`msExchRemoteRecipientType` 2 / 16, or absent).
    OnPremisesMailbox {
        /// Archive.
        archive: ArchiveState,
    },
    /// A remote (Exchange Online) mailbox.
    RemoteMailbox(RemoteMailbox),
    /// Any other combination — a mail user, contact, group, or a value this
    /// vocabulary does not decode. Kept raw.
    Other(RecipientAttributes),
}

/// The `msExchRemoteRecipientType` values the lifecycle produces. A remote
/// mailbox state is valid iff its code is listed; codes 2 and 16 belong to
/// an on-premises mailbox with a remote archive.
pub const REMOTE_RECIPIENT_TYPES: [u32; 26] = [
    1, 2, 3, 4, 6, 8, 10, 16, 17, 20, 24, // user
    33, 35, 36, 38, 49, 52, // room
    65, 67, 68, 70, 81, 84, // equipment
    100, 102, 116, // shared
];

fn archive_bits(a: ArchiveState) -> u32 {
    match a {
        ArchiveState::None => 0,
        ArchiveState::Provisioned => flag::PROVISION_ARCHIVE,
        ArchiveState::Deprovisioned => flag::DEPROVISION_ARCHIVE,
    }
}

impl RemoteMailbox {
    /// A remote mailbox, if the lifecycle can produce it. A provisioned or
    /// migrated mailbox carries its routing address; a deprovisioned one
    /// has none (the attributes are removed with it).
    pub fn new(
        kind: RemoteKind,
        mailbox: MailboxState,
        archive: ArchiveState,
        routing: Option<ValueId>,
    ) -> Option<Self> {
        let m = Self {
            kind,
            mailbox,
            archive,
            routing,
        };
        let routed = mailbox != MailboxState::Deprovisioned;
        (REMOTE_RECIPIENT_TYPES.contains(&m.code()) && routing.is_some() == routed).then_some(m)
    }
    /// Kind.
    pub fn kind(&self) -> RemoteKind {
        self.kind
    }
    /// Mailbox state.
    pub fn mailbox(&self) -> MailboxState {
        self.mailbox
    }
    /// Archive state.
    pub fn archive(&self) -> ArchiveState {
        self.archive
    }
    /// Routing address (`targetAddress`).
    pub fn routing(&self) -> Option<ValueId> {
        self.routing
    }
    /// `msExchRemoteRecipientType`.
    pub fn code(&self) -> u32 {
        let mailbox = match self.mailbox {
            MailboxState::Provisioned => flag::PROVISION_MAILBOX,
            MailboxState::Migrated => flag::MIGRATED,
            MailboxState::Deprovisioned => flag::DEPROVISION_MAILBOX,
        };
        let kind = match self.kind {
            RemoteKind::User => 0,
            RemoteKind::Room => flag::ROOM,
            RemoteKind::Equipment => flag::EQUIPMENT,
            RemoteKind::Shared => flag::SHARED,
        };
        mailbox | archive_bits(self.archive) | kind
    }
    /// `(display type, type details)` while the mailbox exists; none once it
    /// is deprovisioned.
    fn types(&self) -> Option<(i32, u64)> {
        if self.mailbox == MailboxState::Deprovisioned {
            return None;
        }
        Some(match self.kind {
            RemoteKind::User => (
                display_type::REMOTE_USER_MAILBOX,
                type_details::REMOTE_USER_MAILBOX,
            ),
            RemoteKind::Room => (
                display_type::REMOTE_ROOM_MAILBOX,
                type_details::REMOTE_ROOM_MAILBOX,
            ),
            RemoteKind::Equipment => (
                display_type::REMOTE_EQUIPMENT_MAILBOX,
                type_details::REMOTE_EQUIPMENT_MAILBOX,
            ),
            RemoteKind::Shared => (
                display_type::REMOTE_USER_MAILBOX,
                type_details::REMOTE_SHARED_MAILBOX,
            ),
        })
    }
    fn decode_code(code: u32) -> Option<(RemoteKind, MailboxState, ArchiveState)> {
        if !REMOTE_RECIPIENT_TYPES.contains(&code) {
            return None;
        }
        let kind = match code & flag::SHARED {
            0 => RemoteKind::User,
            flag::ROOM => RemoteKind::Room,
            flag::EQUIPMENT => RemoteKind::Equipment,
            _ => RemoteKind::Shared,
        };
        let mailbox = if code & flag::PROVISION_MAILBOX != 0 {
            MailboxState::Provisioned
        } else if code & flag::MIGRATED != 0 {
            MailboxState::Migrated
        } else if code & flag::DEPROVISION_MAILBOX != 0 {
            MailboxState::Deprovisioned
        } else {
            return None; // 2 / 16: an on-premises mailbox, not a remote one
        };
        let archive = if code & flag::PROVISION_ARCHIVE != 0 {
            ArchiveState::Provisioned
        } else if code & flag::DEPROVISION_ARCHIVE != 0 {
            ArchiveState::Deprovisioned
        } else {
            ArchiveState::None
        };
        Some((kind, mailbox, archive))
    }
}

impl Recipient {
    /// Decode stored attributes. Strict: a decoded state re-encodes to
    /// exactly `a`; everything else is [`Recipient::Other`].
    pub fn from_attributes(a: RecipientAttributes) -> Self {
        let decoded = Self::decode(a);
        match decoded {
            Some(r) if r.attributes() == a => r,
            _ => Self::Other(a),
        }
    }
    fn decode(a: RecipientAttributes) -> Option<Self> {
        if a == RecipientAttributes::default() {
            return Some(Self::NotMailEnabled);
        }
        if a.type_details == Some(type_details::USER_MAILBOX) {
            let archive = match a.remote_recipient_type {
                None => ArchiveState::None,
                Some(flag::PROVISION_ARCHIVE) => ArchiveState::Provisioned,
                Some(flag::DEPROVISION_ARCHIVE) => ArchiveState::Deprovisioned,
                Some(_) => return None,
            };
            return Some(Self::OnPremisesMailbox { archive });
        }
        let (kind, mailbox, archive) = RemoteMailbox::decode_code(a.remote_recipient_type?)?;
        RemoteMailbox::new(kind, mailbox, archive, a.target_address).map(Self::RemoteMailbox)
    }
    /// The stored attributes of this recipient.
    pub fn attributes(&self) -> RecipientAttributes {
        match self {
            Self::NotMailEnabled => RecipientAttributes::default(),
            Self::OnPremisesMailbox { archive } => RecipientAttributes {
                remote_recipient_type: (*archive != ArchiveState::None)
                    .then(|| archive_bits(*archive)),
                display_type: Some(display_type::MAILBOX_USER),
                type_details: Some(type_details::USER_MAILBOX),
                target_address: None,
            },
            Self::RemoteMailbox(m) => {
                let types = m.types();
                RecipientAttributes {
                    remote_recipient_type: Some(m.code()),
                    display_type: types.map(|t| t.0),
                    type_details: types.map(|t| t.1),
                    target_address: m.routing,
                }
            }
            Self::Other(a) => *a,
        }
    }
    /// Whether Exchange treats the object as a recipient that owns its
    /// addresses. A deprovisioned remote mailbox and a non-mail-enabled
    /// object do not; everything else (including [`Recipient::Other`]) does.
    pub fn is_recipient(&self) -> bool {
        match self {
            Self::NotMailEnabled => false,
            Self::RemoteMailbox(m) => m.mailbox != MailboxState::Deprovisioned,
            Self::OnPremisesMailbox { .. } | Self::Other(_) => true,
        }
    }
}

/// One step of the hybrid remote-mailbox lifecycle (semantic; the actuator
/// maps it to its own commands).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RemoteMailboxOp {
    /// `Enable-RemoteMailbox [-Room | -Equipment] -RemoteRoutingAddress`:
    /// a non-mail-enabled user gets a provisioned remote mailbox.
    Enable {
        /// User, room or equipment (there is no shared variant of enable).
        kind: RemoteKind,
        /// Routing address.
        routing: ValueId,
    },
    /// `Enable-RemoteMailbox -Archive` (remote mailbox) or
    /// `Enable-Mailbox -RemoteArchive` (on-premises mailbox).
    EnableArchive,
    /// `Disable-RemoteMailbox -Archive` / `Disable-Mailbox -RemoteArchive`.
    DisableArchive,
    /// A completed move of an on-premises mailbox to Exchange Online.
    CompleteMove {
        /// Routing address of the moved mailbox.
        routing: ValueId,
    },
    /// `Set-RemoteMailbox -Type`: change what the remote mailbox is.
    SetType(RemoteKind),
    /// `Disable-RemoteMailbox`.
    Disable,
}

/// Why a lifecycle step does not apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleRefusal {
    /// The step does not start from this state.
    NotApplicable,
    /// The step would reach a state the lifecycle does not produce.
    NotInTable,
}

impl RemoteMailboxOp {
    /// The state after this step.
    ///
    /// # Errors
    ///
    /// [`LifecycleRefusal`].
    pub fn apply(self, from: &Recipient) -> Result<Recipient, LifecycleRefusal> {
        use LifecycleRefusal::{NotApplicable, NotInTable};
        let remote = |kind, mailbox, archive, routing| {
            RemoteMailbox::new(kind, mailbox, archive, routing)
                .map(Recipient::RemoteMailbox)
                .ok_or(NotInTable)
        };
        let live = |m: &RemoteMailbox| m.mailbox != MailboxState::Deprovisioned;
        match (self, from) {
            // A shared mailbox has no "provisioned" code (97 is not listed),
            // so the table refuses it; no separate check.
            (Self::Enable { kind, routing }, Recipient::NotMailEnabled) => remote(
                kind,
                MailboxState::Provisioned,
                ArchiveState::None,
                Some(routing),
            ),
            (Self::EnableArchive, Recipient::RemoteMailbox(m))
                if live(m) && m.archive != ArchiveState::Provisioned =>
            {
                remote(m.kind, m.mailbox, ArchiveState::Provisioned, m.routing)
            }
            (Self::EnableArchive, Recipient::OnPremisesMailbox { archive })
                if *archive != ArchiveState::Provisioned =>
            {
                Ok(Recipient::OnPremisesMailbox {
                    archive: ArchiveState::Provisioned,
                })
            }
            (Self::DisableArchive, Recipient::RemoteMailbox(m))
                if live(m) && m.archive == ArchiveState::Provisioned =>
            {
                remote(m.kind, m.mailbox, ArchiveState::Deprovisioned, m.routing)
            }
            (
                Self::DisableArchive,
                Recipient::OnPremisesMailbox {
                    archive: ArchiveState::Provisioned,
                },
            ) => Ok(Recipient::OnPremisesMailbox {
                archive: ArchiveState::Deprovisioned,
            }),
            (Self::CompleteMove { routing }, Recipient::OnPremisesMailbox { archive }) => remote(
                RemoteKind::User,
                MailboxState::Migrated,
                *archive,
                Some(routing),
            ),
            (Self::SetType(kind), Recipient::RemoteMailbox(m)) if live(m) && m.kind != kind => {
                remote(kind, m.mailbox, m.archive, m.routing)
            }
            (Self::Disable, Recipient::RemoteMailbox(m)) if live(m) => {
                remote(m.kind, MailboxState::Deprovisioned, m.archive, None)
            }
            _ => Err(NotApplicable),
        }
    }

    /// The one step that takes `from` to `to`, if exactly one does.
    pub fn between(from: &Recipient, to: &Recipient) -> Option<Self> {
        let routing = match to {
            Recipient::RemoteMailbox(m) => m.routing,
            _ => None,
        };
        let kinds = [
            RemoteKind::User,
            RemoteKind::Room,
            RemoteKind::Equipment,
            RemoteKind::Shared,
        ];
        let mut candidates = vec![Self::EnableArchive, Self::DisableArchive, Self::Disable];
        candidates.extend(kinds.map(Self::SetType));
        if let Some(routing) = routing {
            candidates.extend(kinds.map(|kind| Self::Enable { kind, routing }));
            candidates.push(Self::CompleteMove { routing });
        }
        let mut hits = candidates
            .into_iter()
            .filter(|op| op.apply(from).as_ref() == Ok(to));
        let first = hits.next()?;
        hits.next().is_none().then_some(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: ValueId = ValueId(42);

    fn attrs(rrt: Option<u32>, display: Option<i32>, details: Option<u64>) -> RecipientAttributes {
        RecipientAttributes {
            remote_recipient_type: rrt,
            display_type: display,
            type_details: details,
            target_address: None,
        }
    }

    // Every code 0..=255: exactly the 26 listed codes decode to a lifecycle
    // state (with the type codes and routing address that belong to them);
    // every other code stays raw. Each decoded state re-encodes to its code.
    #[test]
    fn exactly_the_26_listed_remote_recipient_types_decode() {
        let mut decoded = Vec::new();
        for code in 0..=255u32 {
            let mut hit = None;
            for kind in [
                RemoteKind::User,
                RemoteKind::Room,
                RemoteKind::Equipment,
                RemoteKind::Shared,
            ] {
                for mailbox in [
                    MailboxState::Provisioned,
                    MailboxState::Migrated,
                    MailboxState::Deprovisioned,
                ] {
                    for archive in [
                        ArchiveState::None,
                        ArchiveState::Provisioned,
                        ArchiveState::Deprovisioned,
                    ] {
                        let routing = (mailbox != MailboxState::Deprovisioned).then_some(R);
                        if let Some(m) = RemoteMailbox::new(kind, mailbox, archive, routing)
                            && m.code() == code
                        {
                            let r = Recipient::RemoteMailbox(m);
                            assert_eq!(Recipient::from_attributes(r.attributes()), r);
                            hit = Some(code);
                        }
                    }
                }
            }
            for archive in [ArchiveState::Provisioned, ArchiveState::Deprovisioned] {
                let r = Recipient::OnPremisesMailbox { archive };
                if r.attributes().remote_recipient_type == Some(code) {
                    assert_eq!(Recipient::from_attributes(r.attributes()), r);
                    hit = Some(code);
                }
            }
            decoded.extend(hit);
        }
        assert_eq!(decoded, REMOTE_RECIPIENT_TYPES.to_vec());
    }

    // The script's own example: Enable-RemoteMailbox then -Archive gives
    // msExchRemoteRecipientType 3, DisplayType -2147483642, TypeDetails
    // 2147483648.
    #[test]
    fn enable_then_archive_matches_the_script() {
        let r = RemoteMailboxOp::Enable {
            kind: RemoteKind::User,
            routing: R,
        }
        .apply(&Recipient::NotMailEnabled)
        .unwrap();
        let r = RemoteMailboxOp::EnableArchive.apply(&r).unwrap();
        assert_eq!(
            r.attributes(),
            RecipientAttributes {
                remote_recipient_type: Some(3),
                display_type: Some(-2_147_483_642),
                type_details: Some(2_147_483_648),
                target_address: Some(R),
            }
        );
    }

    // A walk through the table's transitions, each landing on the listed
    // code.
    #[test]
    fn lifecycle_transitions_land_on_listed_codes() {
        let code = |r: &Recipient| r.attributes().remote_recipient_type;
        let room = RemoteMailboxOp::Enable {
            kind: RemoteKind::Room,
            routing: R,
        }
        .apply(&Recipient::NotMailEnabled)
        .unwrap();
        assert_eq!(code(&room), Some(33));
        let room = RemoteMailboxOp::EnableArchive.apply(&room).unwrap();
        assert_eq!(code(&room), Some(35));
        assert_eq!(
            code(&RemoteMailboxOp::DisableArchive.apply(&room).unwrap()),
            Some(49)
        );

        let onprem = Recipient::OnPremisesMailbox {
            archive: ArchiveState::None,
        };
        let archived = RemoteMailboxOp::EnableArchive.apply(&onprem).unwrap();
        assert_eq!(code(&archived), Some(2));
        let moved = RemoteMailboxOp::CompleteMove { routing: R }
            .apply(&archived)
            .unwrap();
        assert_eq!(code(&moved), Some(6));
        let shared = RemoteMailboxOp::SetType(RemoteKind::Shared)
            .apply(&moved)
            .unwrap();
        assert_eq!(code(&shared), Some(102));
        assert_eq!(
            shared.attributes().type_details,
            Some(type_details::REMOTE_SHARED_MAILBOX)
        );

        let user = RemoteMailboxOp::Enable {
            kind: RemoteKind::User,
            routing: R,
        }
        .apply(&Recipient::NotMailEnabled)
        .unwrap();
        let gone = RemoteMailboxOp::Disable.apply(&user).unwrap();
        assert_eq!(code(&gone), Some(8));
        assert_eq!(gone.attributes().type_details, None);
        assert_eq!(gone.attributes().target_address, None);
        assert!(!gone.is_recipient());
    }

    // Steps outside the table are refused, never approximated.
    #[test]
    fn steps_outside_the_table_are_refused() {
        use LifecycleRefusal::{NotApplicable, NotInTable};
        let enable = |kind| RemoteMailboxOp::Enable { kind, routing: R };
        // No shared variant of enable; a provisioned shared mailbox is not listed.
        assert_eq!(
            enable(RemoteKind::Shared).apply(&Recipient::NotMailEnabled),
            Err(NotInTable)
        );
        let room = enable(RemoteKind::Room)
            .apply(&Recipient::NotMailEnabled)
            .unwrap();
        assert_eq!(
            RemoteMailboxOp::SetType(RemoteKind::Shared).apply(&room),
            Err(NotInTable)
        );
        // Disable is listed for user mailboxes only.
        assert_eq!(RemoteMailboxOp::Disable.apply(&room), Err(NotInTable));
        // Enable twice; enable on an on-premises mailbox; archive twice.
        assert_eq!(enable(RemoteKind::User).apply(&room), Err(NotApplicable));
        assert_eq!(
            enable(RemoteKind::User).apply(&Recipient::OnPremisesMailbox {
                archive: ArchiveState::None
            }),
            Err(NotApplicable)
        );
        let archived = RemoteMailboxOp::EnableArchive.apply(&room).unwrap();
        assert_eq!(
            RemoteMailboxOp::EnableArchive.apply(&archived),
            Err(NotApplicable)
        );
        // Nothing applies to a raw recipient.
        let raw = Recipient::Other(attrs(None, Some(6), Some(128)));
        assert_eq!(RemoteMailboxOp::Disable.apply(&raw), Err(NotApplicable));
    }

    // Decode is strict: a listed code with the wrong type codes, a remote
    // mailbox without its routing address, or an unlisted code stays raw.
    #[test]
    fn inconsistent_attributes_stay_raw() {
        let mut a = attrs(Some(1), Some(-2_147_483_642), Some(2_147_483_648));
        assert!(matches!(Recipient::from_attributes(a), Recipient::Other(_)));
        a.target_address = Some(R);
        assert!(matches!(
            Recipient::from_attributes(a),
            Recipient::RemoteMailbox(_)
        ));
        a.type_details = Some(type_details::REMOTE_SHARED_MAILBOX);
        assert_eq!(Recipient::from_attributes(a), Recipient::Other(a));
        let unlisted = attrs(Some(5), None, None);
        assert_eq!(
            Recipient::from_attributes(unlisted),
            Recipient::Other(unlisted)
        );
        // A mail user (128) is a recipient this vocabulary keeps raw.
        let mail_user = attrs(None, Some(6), Some(128));
        assert!(Recipient::from_attributes(mail_user).is_recipient());
        assert_eq!(
            Recipient::from_attributes(RecipientAttributes::default()),
            Recipient::NotMailEnabled
        );
    }

    // `between` names the one step joining two states, and nothing when no
    // single step does.
    #[test]
    fn between_finds_the_single_step() {
        let user = RemoteMailboxOp::Enable {
            kind: RemoteKind::User,
            routing: R,
        }
        .apply(&Recipient::NotMailEnabled)
        .unwrap();
        assert_eq!(
            RemoteMailboxOp::between(&Recipient::NotMailEnabled, &user),
            Some(RemoteMailboxOp::Enable {
                kind: RemoteKind::User,
                routing: R
            })
        );
        let archived = RemoteMailboxOp::EnableArchive.apply(&user).unwrap();
        assert_eq!(
            RemoteMailboxOp::between(&user, &archived),
            Some(RemoteMailboxOp::EnableArchive)
        );
        // Two steps apart: enable and archive at once.
        assert_eq!(
            RemoteMailboxOp::between(&Recipient::NotMailEnabled, &archived),
            None
        );
    }
}
