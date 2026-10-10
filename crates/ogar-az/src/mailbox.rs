//! Exchange Online mailboxes as Microsoft Graph shows them.
//!
//! Graph does not report a mailbox as a property of the `user`: it takes
//! separate reads, each with its own permission. This module names those
//! reads ([`Pull`]), the least-privileged permission each one needs
//! ([`Permission`], granted as [`Grant`]), and encodes what they return into one
//! [`DirRecord`] per user (object kind [`AzKind::Mailbox`]):
//!
//! * WHO   = the Graph user `id` (the Entra object id, which Exchange Online
//!   reports as the mailbox's `ExternalDirectoryObjectId`);
//! * WHERE = the tenant id;
//! * WHAT  = [`MAILBOX_SCHEMA_V1`]: `mailboxSettings.userPurpose` and
//!   `settings/exchange.primaryMailboxId`, raw, plus the mailbox GUID
//!   decoded from `primaryMailboxId`.
//!
//! Every pull is a `GET`. Nothing here writes to Graph, and nothing here
//! performs HTTP: the caller fetches with its own token and hands the
//! response bodies in.
//!
//! One more read rides along: [`Pull::UserDrive`] resolves a user's
//! OneDrive ([`user_drive`]), so a consumer that uploads documents (Spear's
//! drive scope) can address the drive from the directory's user id. The
//! upload itself is the consumer's write, not this module's.
//!
//! ## `primaryMailboxId`
//!
//! Graph documents `primaryMailboxId` only as "the unique identifier for the
//! user's primary mailbox". The spelling observed in tenants is
//! `MBX:{mailbox GUID}@{tenant id}`, and for a primary mailbox that GUID is
//! its `ExchangeGuid`. That reading is not a documented contract, so it is
//! decoded strictly ([`mailbox_guid_of`]): two complete GUIDs, and the
//! tenant part must equal the record's tenant. Any other spelling keeps the
//! raw value and decodes no GUID; it is never guessed.

use crate::{AzError, AzKind};
use ogar_dir_core::{AttrDef, AttrKind, DirRecord, Guid128, SchemaFamily, SchemaId, ValuePool};
use serde_json::Value;

/// Encoder schema version of [`MAILBOX_SCHEMA_V1`].
pub const MAILBOX_SCHEMA_VERSION: u16 = 1;
/// The mailbox record's schema id. It shares the Graph family with the
/// `user` records; [`AzKind::Mailbox`] tells the two tables apart.
pub const MAILBOX_SCHEMA: SchemaId = SchemaId {
    family: SchemaFamily::MsGraph,
    version: MAILBOX_SCHEMA_VERSION,
};

/// The mailbox attribute table, v1.
pub const MAILBOX_SCHEMA_V1: &[AttrDef] = &[
    AttrDef {
        name: "userPurpose",
        slot: 0,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "primaryMailboxId",
        slot: 1,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "mailboxGuid",
        slot: 0,
        kind: AttrKind::Guid,
        since: 1,
    },
];

const PURPOSE: usize = 0;
const MAILBOX_ID: usize = 1;
const MAILBOX_GUID: usize = 0;

/// A Graph application permission a pull needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Permission {
    /// `User.Read.All`.
    UserReadAll,
    /// `MailboxSettings.Read`.
    MailboxSettingsRead,
    /// `MailboxFolder.Read.All`.
    MailboxFolderReadAll,
    /// `MailboxItem.Read.All`.
    MailboxItemReadAll,
    /// `Files.Read.All`.
    FilesReadAll,
}

/// How a permission is granted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Grant {
    /// To the application itself (no signed-in user).
    Application,
    /// On behalf of a signed-in user.
    Delegated,
}

impl Permission {
    /// The permission's name as Entra lists it.
    pub fn name(self) -> &'static str {
        match self {
            Self::UserReadAll => "User.Read.All",
            Self::MailboxSettingsRead => "MailboxSettings.Read",
            Self::MailboxFolderReadAll => "MailboxFolder.Read.All",
            Self::MailboxItemReadAll => "MailboxItem.Read.All",
            Self::FilesReadAll => "Files.Read.All",
        }
    }
}

/// One read against Graph v1.0. All are `GET`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pull<'a> {
    /// `GET /users/{id}/mailboxSettings`: carries `userPurpose`.
    MailboxSettings {
        /// The user.
        user: Guid128,
    },
    /// `GET /users/{id}/settings/exchange`: carries `primaryMailboxId`.
    ExchangeSettings {
        /// The user.
        user: Guid128,
    },
    /// `GET /admin/exchange/mailboxes/{mailboxId}/folders`.
    MailboxFolders {
        /// `primaryMailboxId` as read.
        mailbox: &'a str,
    },
    /// `GET /admin/exchange/mailboxes/{mailboxId}/folders/{folderId}/items`.
    MailboxItems {
        /// `primaryMailboxId` as read.
        mailbox: &'a str,
        /// The folder id as read.
        folder: &'a str,
    },
    /// `GET /users/{id}/drive`: the user's OneDrive, whose id addresses
    /// uploads to it.
    UserDrive {
        /// The user.
        user: Guid128,
    },
}

impl Pull<'_> {
    /// The request URL. A mailbox or folder id is percent-encoded.
    pub fn url(&self) -> String {
        const BASE: &str = "https://graph.microsoft.com/v1.0";
        match *self {
            Self::MailboxSettings { user } => format!("{BASE}/users/{user}/mailboxSettings"),
            Self::ExchangeSettings { user } => format!("{BASE}/users/{user}/settings/exchange"),
            Self::MailboxFolders { mailbox } => {
                format!(
                    "{BASE}/admin/exchange/mailboxes/{}/folders",
                    encode(mailbox)
                )
            }
            Self::UserDrive { user } => format!("{BASE}/users/{user}/drive"),
            Self::MailboxItems { mailbox, folder } => format!(
                "{BASE}/admin/exchange/mailboxes/{}/folders/{}/items",
                encode(mailbox),
                encode(folder)
            ),
        }
    }

    /// The least-privileged permission the pull needs, under
    /// [`Pull::grant`].
    pub fn permission(&self) -> Permission {
        match self {
            Self::UserDrive { .. } => Permission::FilesReadAll,
            Self::MailboxSettings { .. } => Permission::MailboxSettingsRead,
            Self::ExchangeSettings { .. } => Permission::UserReadAll,
            Self::MailboxFolders { .. } => Permission::MailboxFolderReadAll,
            Self::MailboxItems { .. } => Permission::MailboxItemReadAll,
        }
    }

    /// How [`Pull::permission`] is granted. Microsoft lists reading a
    /// user's drive as delegated only; the mailbox reads take an
    /// application permission.
    pub fn grant(&self) -> Grant {
        match self {
            Self::UserDrive { .. } => Grant::Delegated,
            _ => Grant::Application,
        }
    }
}

/// A user's OneDrive, as `GET /users/{id}/drive` returns it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserDrive {
    /// The drive id, which addresses the drive (`/drives/{id}`).
    pub id: String,
    /// `driveType`: `business` for OneDrive for Business.
    pub drive_type: String,
}

/// Read a `GET /users/{id}/drive` body. The drive is `user`'s only when its
/// `owner.user.id` is `user`; a drive owned by anyone else, or a body
/// without an id, is `None`.
pub fn user_drive(body: &str, user: Guid128) -> Result<Option<UserDrive>, AzError> {
    let v: Value = serde_json::from_str(body).map_err(|_| AzError::NotAPage)?;
    let o = v.as_object().ok_or(AzError::NotAPage)?;
    let owner = o
        .get("owner")
        .and_then(|w| w.get("user"))
        .and_then(|u| u.get("id"))
        .and_then(Value::as_str)
        .and_then(|s| Guid128::parse(s).ok());
    if owner != Some(user) {
        return Ok(None);
    }
    let field = |name| o.get(name).and_then(Value::as_str).map(str::to_string);
    Ok(field("id").map(|id| UserDrive {
        id,
        drive_type: field("driveType").unwrap_or_default(),
    }))
}

/// Percent-encode everything outside RFC 3986's unreserved set.
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The mailbox GUID in a `primaryMailboxId`, when it is spelled
/// `MBX:{guid}@{tenant}` with two complete, unbraced GUIDs and the tenant
/// part equals `tenant`. Anything else is `None`.
pub fn mailbox_guid_of(primary_mailbox_id: &str, tenant: Guid128) -> Option<Guid128> {
    let rest = primary_mailbox_id.strip_prefix("MBX:")?;
    let (guid, at) = rest.split_once('@')?;
    if guid.len() != 36 || at.len() != 36 {
        return None;
    }
    (Guid128::parse(at).ok()? == tenant)
        .then(|| Guid128::parse(guid).ok())
        .flatten()
}

/// What one user's mailbox reads returned. A body that was not fetched is
/// `None`; the record then leaves those attributes absent.
#[derive(Clone, Copy, Debug)]
pub struct MailboxBodies<'a> {
    /// The user (Graph `id`).
    pub user: Guid128,
    /// The `GET /users/{id}/mailboxSettings` body.
    pub mailbox_settings: Option<&'a str>,
    /// The `GET /users/{id}/settings/exchange` body.
    pub exchange_settings: Option<&'a str>,
}

/// Encode one user's mailbox reads into a [`DirRecord`] of kind
/// [`AzKind::Mailbox`].
pub fn encode_mailbox(
    bodies: &MailboxBodies<'_>,
    tenant: Guid128,
    pool: &mut ValuePool,
    observed_at_ms: i64,
) -> Result<DirRecord, AzError> {
    let mut rec = DirRecord::new(
        MAILBOX_SCHEMA,
        AzKind::Mailbox as u16,
        bodies.user,
        tenant,
        observed_at_ms,
    );
    let st = |e: &dyn std::fmt::Debug| AzError::Storage(format!("{e:?}"));
    if let Some(body) = bodies.mailbox_settings
        && let Some(p) = string_field(body, "userPurpose")?
    {
        let r = pool.push(p.as_bytes()).map_err(|e| st(&e))?;
        rec.set_str(PURPOSE, r).map_err(|e| st(&e))?;
    }
    if let Some(body) = bodies.exchange_settings
        && let Some(id) = string_field(body, "primaryMailboxId")?
    {
        let r = pool.push(id.as_bytes()).map_err(|e| st(&e))?;
        rec.set_str(MAILBOX_ID, r).map_err(|e| st(&e))?;
        if let Some(g) = mailbox_guid_of(&id, tenant) {
            rec.set_guid(MAILBOX_GUID, g).map_err(|e| st(&e))?;
        }
    }
    Ok(rec)
}

/// A top-level string property of a response body: `None` when absent or
/// `null`, an error when the body is not an object or the value is not a
/// string.
fn string_field(body: &str, name: &'static str) -> Result<Option<String>, AzError> {
    let v: Value = serde_json::from_str(body).map_err(|_| AzError::NotAPage)?;
    let o = v.as_object().ok_or(AzError::NotAPage)?;
    match o.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(AzError::BadType(0, name)),
    }
}

/// `userPurpose` as read.
pub fn user_purpose<'p>(rec: &DirRecord, pool: &'p ValuePool) -> Option<&'p str> {
    read_str(rec, pool, PURPOSE)
}

/// `primaryMailboxId` as read.
pub fn primary_mailbox_id<'p>(rec: &DirRecord, pool: &'p ValuePool) -> Option<&'p str> {
    read_str(rec, pool, MAILBOX_ID)
}

/// The mailbox GUID decoded from `primaryMailboxId`.
pub fn mailbox_guid(rec: &DirRecord) -> Option<Guid128> {
    is_mailbox(rec).then(|| rec.guid(MAILBOX_GUID)).flatten()
}

fn is_mailbox(rec: &DirRecord) -> bool {
    rec.schema().family == SchemaFamily::MsGraph && rec.object_kind() == AzKind::Mailbox as u16
}

fn read_str<'p>(rec: &DirRecord, pool: &'p ValuePool, slot: usize) -> Option<&'p str> {
    if !is_mailbox(rec) {
        return None;
    }
    std::str::from_utf8(pool.get(rec.str_ref(slot)?)?).ok()
}
