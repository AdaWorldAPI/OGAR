//! Where mailboxes live, and what each source decides.
//!
//! The same directory object reads differently depending on which mail
//! system hosts its mailbox. Three hosts are modelled ([`MailboxHost`]):
//! Exchange Server and Stalwart on-premises, Exchange Online in the cloud.
//! A [`Deployment`] names the on-premises host (if any) and whether
//! Exchange Online is present, and says which observation is authoritative
//! for what. [`Deployment::host_of`] answers, per recipient, where its
//! mailbox is — the question a mail server must answer before it accepts a
//! message as local or relays it.
//!
//! A mailbox's location is its [`MailboxLocation`]: the host, the location
//! type Exchange reports (`Get-MailboxLocation`, [`MailboxLocationType`]),
//! and the mailbox GUID. Exchange identifies a location as
//! `TenantGUID\MailboxGUID`; Microsoft Graph spells the same pair
//! `MBX:{MailboxGUID}@{TenantGUID}` (`primaryMailboxId`).

use crate::exchange::{MailboxState, Recipient, RemoteKind};
use ogar_dir_core::Guid128;

/// The mail system that hosts a mailbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MailboxHost {
    /// Exchange Server, on-premises.
    ExchangeServer,
    /// Stalwart, on-premises, serving recipients from the directory.
    Stalwart,
    /// Exchange Online.
    ExchangeOnline,
}

/// Which mail systems are present.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Deployment {
    /// The on-premises host, if any: [`MailboxHost::ExchangeServer`] or
    /// [`MailboxHost::Stalwart`].
    pub on_premises: Option<MailboxHost>,
    /// Exchange Online is present.
    pub online: bool,
}

impl Deployment {
    /// Exchange Server only.
    pub const EXCHANGE_SERVER: Self = Self {
        on_premises: Some(MailboxHost::ExchangeServer),
        online: false,
    };
    /// Exchange Online only.
    pub const ONLINE: Self = Self {
        on_premises: None,
        online: true,
    };
    /// Exchange Server and Exchange Online, joined by Entra Connect.
    pub const EXCHANGE_HYBRID: Self = Self {
        on_premises: Some(MailboxHost::ExchangeServer),
        online: true,
    };
    /// Stalwart only.
    pub const STALWART: Self = Self {
        on_premises: Some(MailboxHost::Stalwart),
        online: false,
    };
    /// Stalwart on-premises next to Exchange Online.
    pub const STALWART_WITH_ONLINE: Self = Self {
        on_premises: Some(MailboxHost::Stalwart),
        online: true,
    };

    /// Whether the on-premises recipient attributes
    /// (`msExchRemoteRecipientType`, `msExchRecipientDisplayType`,
    /// `msExchRecipientTypeDetails`, `targetAddress`, `msExchMailboxGuid`)
    /// are authoritative. Without an on-premises host they are not read,
    /// even for users synchronized from AD.
    pub fn reads_on_premises_recipient(self) -> bool {
        self.on_premises.is_some()
    }

    /// Whether the Exchange Online mailbox is observed and decides delivery
    /// to a cloud-hosted recipient.
    pub fn reads_cloud_mailbox(self) -> bool {
        self.online
    }

    /// Whether both sides carry an `ExchangeGuid` to compare: Exchange
    /// Server writes `msExchMailboxGuid`; Stalwart has no `ExchangeGuid`.
    pub fn compares_exchange_guid(self) -> bool {
        self.on_premises == Some(MailboxHost::ExchangeServer) && self.online
    }

    /// Where `recipient`'s mailbox is hosted, by its on-premises recipient
    /// type. An on-premises mailbox is on the on-premises host; a remote
    /// mailbox that is not deprovisioned is in Exchange Online. Anything
    /// else (not mail-enabled, deprovisioned, a type this vocabulary keeps
    /// raw) has no hosted mailbox, and neither has a mailbox whose host the
    /// deployment does not include.
    ///
    /// This reads the recipient type only; whether Exchange Online actually
    /// holds the mailbox is the cloud observation's question.
    pub fn host_of(self, recipient: &Recipient) -> Option<MailboxHost> {
        match recipient {
            Recipient::OnPremisesMailbox { .. } => self.on_premises,
            Recipient::RemoteMailbox(m) if m.mailbox() != MailboxState::Deprovisioned => {
                self.online.then_some(MailboxHost::ExchangeOnline)
            }
            _ => None,
        }
    }
}

/// `MailboxLocationType`, as `Get-MailboxLocation` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MailboxLocationType {
    /// The primary mailbox.
    Primary,
    /// The main archive.
    MainArchive,
    /// An auxiliary archive.
    AuxArchive,
    /// An auxiliary primary mailbox.
    AuxPrimary,
    /// A component-shared mailbox.
    ComponentShared,
    /// An aggregated mailbox.
    Aggregated,
    /// The previous primary mailbox (Exchange Online only).
    PreviousPrimary,
}

impl MailboxLocationType {
    /// Decode Exchange's spelling, exactly. Anything else is `None`.
    pub fn from_exchange(s: &str) -> Option<Self> {
        Some(match s {
            "Primary" => Self::Primary,
            "MainArchive" => Self::MainArchive,
            "AuxArchive" => Self::AuxArchive,
            "AuxPrimary" => Self::AuxPrimary,
            "ComponentShared" => Self::ComponentShared,
            "Aggregated" => Self::Aggregated,
            "PreviousPrimary" => Self::PreviousPrimary,
            _ => return None,
        })
    }
}

/// Where one mailbox is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MailboxLocation {
    /// The host.
    pub host: MailboxHost,
    /// Which of the owner's mailboxes this is.
    pub kind: MailboxLocationType,
    /// The mailbox GUID, when the host has one (an Exchange mailbox's
    /// `ExchangeGuid` for its primary location). `None` = not observed, or
    /// a host without one.
    pub mailbox_guid: Option<Guid128>,
}

/// `mailboxSettings.userPurpose`: what a cloud mailbox is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MailboxPurpose {
    /// A user mailbox.
    User,
    /// A linked mailbox (its owner is an account in another forest).
    Linked,
    /// A shared mailbox.
    Shared,
    /// A room.
    Room,
    /// Equipment.
    Equipment,
    /// Graph's `others`: a mailbox of a type Graph does not name.
    Others,
}

impl MailboxPurpose {
    /// Decode Graph's spelling. A value this vocabulary does not know,
    /// including Graph's `unknownFutureValue`, is `None`: the raw value
    /// stays with the observation and is never guessed.
    pub fn from_graph(s: &str) -> Option<Self> {
        Some(match s {
            "user" => Self::User,
            "linked" => Self::Linked,
            "shared" => Self::Shared,
            "room" => Self::Room,
            "equipment" => Self::Equipment,
            "others" => Self::Others,
            _ => return None,
        })
    }

    /// The remote mailbox kind a hybrid AD object carries for a cloud
    /// mailbox of this purpose. A linked mailbox and `others` have no remote
    /// mailbox kind.
    pub fn remote_kind(self) -> Option<RemoteKind> {
        match self {
            Self::User => Some(RemoteKind::User),
            Self::Shared => Some(RemoteKind::Shared),
            Self::Room => Some(RemoteKind::Room),
            Self::Equipment => Some(RemoteKind::Equipment),
            Self::Linked | Self::Others => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValueId;
    use crate::exchange::{ArchiveState, RemoteMailbox};

    #[test]
    fn each_deployment_reads_its_own_sources() {
        let table = [
            (Deployment::EXCHANGE_SERVER, true, false, false),
            (Deployment::ONLINE, false, true, false),
            (Deployment::EXCHANGE_HYBRID, true, true, true),
            (Deployment::STALWART, true, false, false),
            // Stalwart has no ExchangeGuid to compare.
            (Deployment::STALWART_WITH_ONLINE, true, true, false),
        ];
        for (d, onprem, cloud, compare) in table {
            assert_eq!(d.reads_on_premises_recipient(), onprem, "{d:?}");
            assert_eq!(d.reads_cloud_mailbox(), cloud, "{d:?}");
            assert_eq!(d.compares_exchange_guid(), compare, "{d:?}");
        }
    }

    // The same recipient lands on different hosts per deployment: an
    // on-premises mailbox on Exchange Server or Stalwart, a remote mailbox in
    // Exchange Online only where Exchange Online is present.
    #[test]
    fn host_of_follows_the_recipient_type_and_the_deployment() {
        use MailboxHost::*;
        let onprem = Recipient::OnPremisesMailbox {
            archive: ArchiveState::None,
        };
        let remote = Recipient::RemoteMailbox(
            RemoteMailbox::new(
                RemoteKind::User,
                MailboxState::Provisioned,
                ArchiveState::None,
                Some(ValueId(1)),
            )
            .unwrap(),
        );
        let gone = Recipient::RemoteMailbox(
            RemoteMailbox::new(
                RemoteKind::User,
                MailboxState::Deprovisioned,
                ArchiveState::None,
                None,
            )
            .unwrap(),
        );
        let table = [
            (Deployment::EXCHANGE_SERVER, Some(ExchangeServer), None),
            (Deployment::STALWART, Some(Stalwart), None),
            (Deployment::ONLINE, None, Some(ExchangeOnline)),
            (
                Deployment::EXCHANGE_HYBRID,
                Some(ExchangeServer),
                Some(ExchangeOnline),
            ),
            (
                Deployment::STALWART_WITH_ONLINE,
                Some(Stalwart),
                Some(ExchangeOnline),
            ),
        ];
        for (d, on, rem) in table {
            assert_eq!(d.host_of(&onprem), on, "{d:?}");
            assert_eq!(d.host_of(&remote), rem, "{d:?}");
            assert_eq!(d.host_of(&gone), None, "{d:?}");
            assert_eq!(d.host_of(&Recipient::NotMailEnabled), None, "{d:?}");
        }
    }

    #[test]
    fn location_types_decode_exactly() {
        for (s, t) in [
            ("Primary", MailboxLocationType::Primary),
            ("MainArchive", MailboxLocationType::MainArchive),
            ("AuxArchive", MailboxLocationType::AuxArchive),
            ("AuxPrimary", MailboxLocationType::AuxPrimary),
            ("ComponentShared", MailboxLocationType::ComponentShared),
            ("Aggregated", MailboxLocationType::Aggregated),
            ("PreviousPrimary", MailboxLocationType::PreviousPrimary),
        ] {
            assert_eq!(MailboxLocationType::from_exchange(s), Some(t), "{s}");
        }
        for s in ["primary", "Archive", ""] {
            assert_eq!(MailboxLocationType::from_exchange(s), None, "{s}");
        }
    }

    #[test]
    fn graph_purposes_decode_exactly_and_map_to_remote_kinds() {
        let table = [
            ("user", MailboxPurpose::User, Some(RemoteKind::User)),
            ("linked", MailboxPurpose::Linked, None),
            ("shared", MailboxPurpose::Shared, Some(RemoteKind::Shared)),
            ("room", MailboxPurpose::Room, Some(RemoteKind::Room)),
            (
                "equipment",
                MailboxPurpose::Equipment,
                Some(RemoteKind::Equipment),
            ),
            ("others", MailboxPurpose::Others, None),
        ];
        for (s, p, k) in table {
            assert_eq!(MailboxPurpose::from_graph(s), Some(p), "{s}");
            assert_eq!(p.remote_kind(), k, "{s}");
        }
        for s in ["unknownFutureValue", "User", "", "mailUser"] {
            assert_eq!(MailboxPurpose::from_graph(s), None, "{s}");
        }
    }
}
