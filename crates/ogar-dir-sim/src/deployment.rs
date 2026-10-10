//! Which Exchange the simulated directory has, and what each source decides.
//!
//! The same object reads differently depending on where its mailbox can
//! live. [`ExchangeDeployment`] names the three shapes and says which
//! observation is authoritative for what; [`MailboxPurpose`] is the mailbox
//! type as Microsoft Graph reports it (`mailboxSettings.userPurpose`), which
//! is the cloud side's counterpart of the on-premises recipient type.

use crate::exchange::RemoteKind;

/// Where mailboxes live.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExchangeDeployment {
    /// Exchange Server only. AD carries every recipient type; there is no
    /// cloud mailbox, so a remote mailbox has nowhere to deliver.
    OnPremises,
    /// Exchange Online only. A user is mail-enabled when Exchange Online has
    /// a mailbox for it; the mailbox type is its [`MailboxPurpose`], and its
    /// `ExchangeGuid` exists only in the cloud. The `msExch*` attributes are
    /// not read, even when users are synchronized from AD.
    Online,
    /// Both, joined by Entra Connect. AD carries the recipient type (a remote
    /// mailbox for a cloud-hosted one); Exchange Online confirms the mailbox
    /// exists and carries its cloud `ExchangeGuid`, which must match the
    /// on-premises `msExchMailboxGuid` for the mailbox to be migratable.
    Hybrid,
}

impl ExchangeDeployment {
    /// Whether the AD recipient attributes (`msExchRemoteRecipientType`,
    /// `msExchRecipientDisplayType`, `msExchRecipientTypeDetails`,
    /// `targetAddress`, `msExchMailboxGuid`) are authoritative.
    pub fn reads_on_premises_recipient(self) -> bool {
        matches!(self, Self::OnPremises | Self::Hybrid)
    }

    /// Whether the Exchange Online mailbox is observed and decides delivery
    /// to a cloud-hosted recipient.
    pub fn reads_cloud_mailbox(self) -> bool {
        matches!(self, Self::Online | Self::Hybrid)
    }

    /// Whether both sides carry an `ExchangeGuid` to compare.
    pub fn compares_exchange_guid(self) -> bool {
        self == Self::Hybrid
    }
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

    #[test]
    fn each_deployment_reads_its_own_sources() {
        use ExchangeDeployment::*;
        let table = [
            (OnPremises, true, false, false),
            (Online, false, true, false),
            (Hybrid, true, true, true),
        ];
        for (d, onprem, cloud, compare) in table {
            assert_eq!(d.reads_on_premises_recipient(), onprem, "{d:?}");
            assert_eq!(d.reads_cloud_mailbox(), cloud, "{d:?}");
            assert_eq!(d.compares_exchange_guid(), compare, "{d:?}");
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
