//! Entra groups and their direct memberships as Microsoft Graph shows them.
//!
//! A Graph `group` becomes one [`DirRecord`] of kind [`AzKind::Group`]:
//!
//! * WHO   = the Graph `id` (the Entra object id);
//! * WHERE = the tenant id. Graph reports no on-premises distinguished name
//!   for a group, so a group carries no OU, synchronized or not;
//! * WHAT  = [`GROUP_SCHEMA_V1`], raw.
//!
//! Membership is a separate read per group (`GET /groups/{id}/members`),
//! which lists the **direct** members only; nesting is the consumer's walk
//! (`/transitiveMembers` would hand over a flattened answer and lose the
//! chain a right is inherited through). [`ingest_members_page`] keeps the
//! members that are users or groups and counts the rest (devices, service
//! principals, organizational contacts), which a directory of people does
//! not hold.
//!
//! `mailEnabled` and `securityEnabled` are kept as read; an absent flag is
//! unknown, never `false`. Read-only: every call is a `GET`, and nothing here
//! performs HTTP.

use crate::{AzError, AzKind};
use ogar_dir_core::{AttrDef, AttrKind, DirRecord, Guid128, SchemaFamily, SchemaId, ValuePool};
use serde_json::Value;

/// Encoder schema version of [`GROUP_SCHEMA_V1`].
pub const GROUP_SCHEMA_VERSION: u16 = 1;
/// The group record's schema id. It shares the Graph family with the
/// `user` records; [`AzKind::Group`] tells the tables apart.
pub const GROUP_SCHEMA: SchemaId = SchemaId {
    family: SchemaFamily::MsGraph,
    version: GROUP_SCHEMA_VERSION,
};

/// The Graph `group` attribute table, v1: naming (`displayName`, `mail`,
/// `mailNickname`, `proxyAddresses`), kind (`groupTypes`, `mailEnabled`,
/// `securityEnabled`), the cloud SID, and the `onPremises*` evidence about an
/// AD counterpart.
pub const GROUP_SCHEMA_V1: &[AttrDef] = &[
    AttrDef {
        name: "displayName",
        slot: 0,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "description",
        slot: 1,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "mail",
        slot: 2,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "mailNickname",
        slot: 3,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "proxyAddresses",
        slot: 4,
        kind: AttrKind::MultiStr,
        since: 1,
    },
    AttrDef {
        name: "groupTypes",
        slot: 5,
        kind: AttrKind::MultiStr,
        since: 1,
    },
    AttrDef {
        name: "securityIdentifier",
        slot: 6,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "onPremisesSamAccountName",
        slot: 7,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "onPremisesSecurityIdentifier",
        slot: 8,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "onPremisesDomainName",
        slot: 9,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "onPremisesLastSyncDateTime",
        slot: 10,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "mailEnabled",
        slot: 0,
        kind: AttrKind::Bool,
        since: 1,
    },
    AttrDef {
        name: "securityEnabled",
        slot: 1,
        kind: AttrKind::Bool,
        since: 1,
    },
    AttrDef {
        name: "onPremisesSyncEnabled",
        slot: 2,
        kind: AttrKind::Bool,
        since: 1,
    },
];

/// `$select` value for groups: `id` plus every attribute in table order.
pub fn group_select_query() -> String {
    std::iter::once("id")
        .chain(GROUP_SCHEMA_V1.iter().map(|a| a.name))
        .collect::<Vec<_>>()
        .join(",")
}

/// The read-only list URL for groups (v1.0 endpoint).
pub fn groups_url(page_size: u16) -> String {
    format!(
        "https://graph.microsoft.com/v1.0/groups?$select={}&$top={}",
        group_select_query(),
        page_size.clamp(1, 999)
    )
}

/// The read-only URL for one group's direct members. Only `id` is selected;
/// `@odata.type` is always returned and tells users from groups.
pub fn members_url(group: &Guid128, page_size: u16) -> String {
    format!(
        "https://graph.microsoft.com/v1.0/groups/{group}/members?$select=id&$top={}",
        page_size.clamp(1, 999)
    )
}

/// Encode one Graph group object.
pub fn encode_group(
    obj: &serde_json::Map<String, Value>,
    index: usize,
    tenant: Guid128,
    pool: &mut ValuePool,
    observed_at_ms: i64,
) -> Result<DirRecord, AzError> {
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .and_then(|s| Guid128::parse(s).ok())
        .ok_or(AzError::BadId(index))?;
    let mut rec = DirRecord::new(
        GROUP_SCHEMA,
        AzKind::Group as u16,
        id,
        tenant,
        observed_at_ms,
    );
    let st = |e: &dyn std::fmt::Debug| AzError::Storage(format!("{e:?}"));
    for def in GROUP_SCHEMA_V1 {
        let Some(v) = obj.get(def.name).filter(|v| !v.is_null()) else {
            continue;
        };
        let bad = || AzError::BadType(index, def.name);
        match def.kind {
            AttrKind::Str => {
                let r = pool
                    .push(v.as_str().ok_or_else(bad)?.as_bytes())
                    .map_err(|e| st(&e))?;
                rec.set_str(def.slot as usize, r).map_err(|e| st(&e))?;
            }
            AttrKind::MultiStr => {
                let vals: Vec<&str> = v
                    .as_array()
                    .ok_or_else(bad)?
                    .iter()
                    .map(|x| x.as_str().ok_or_else(bad))
                    .collect::<Result<_, _>>()?;
                let r = pool.push_multi(&vals).map_err(|e| st(&e))?;
                rec.set_str(def.slot as usize, r).map_err(|e| st(&e))?;
            }
            AttrKind::Bool => {
                let b = v.as_bool().ok_or_else(bad)?;
                rec.set_num(def.slot as usize, u32::from(b))
                    .map_err(|e| st(&e))?;
            }
            // The table holds no other kind.
            AttrKind::Bytes | AttrKind::Bag | AttrKind::Guid | AttrKind::U32 => {
                return Err(bad());
            }
        }
    }
    Ok(rec)
}

/// Ingest one Graph group list-page body (`GET /groups?$select=…`).
pub fn ingest_groups_page(
    body: &str,
    tenant: Guid128,
    pool: &mut ValuePool,
    observed_at_ms: i64,
) -> Result<crate::Page, AzError> {
    let v: Value = serde_json::from_str(body).map_err(|_| AzError::NotAPage)?;
    let items = v
        .get("value")
        .and_then(Value::as_array)
        .ok_or(AzError::NotAPage)?;
    let mut records = Vec::with_capacity(items.len());
    let mut ignored: Vec<String> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let obj = item.as_object().ok_or(AzError::BadId(i))?;
        for k in obj.keys() {
            if k != "id"
                && !k.starts_with('@')
                && !GROUP_SCHEMA_V1.iter().any(|d| d.name == k)
                && !ignored.contains(k)
            {
                ignored.push(k.clone());
            }
        }
        records.push(encode_group(obj, i, tenant, pool, observed_at_ms)?);
    }
    ignored.sort();
    Ok(crate::Page {
        records,
        next_link: next_link(&v),
        ignored,
    })
}

/// What a member is, from its `@odata.type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MemberKind {
    /// `#microsoft.graph.user`.
    User,
    /// `#microsoft.graph.group`: a nested group.
    Group,
}

/// One page of a group's direct members.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MembersPage {
    /// The users and groups, in page order.
    pub members: Vec<(Guid128, MemberKind)>,
    /// Members of any other type (devices, service principals,
    /// organizational contacts), not kept.
    pub skipped: usize,
    /// `@odata.nextLink`, if Graph returned one.
    pub next_link: Option<String>,
}

/// Ingest one `GET /groups/{id}/members` page body.
///
/// A member without a parseable `id` or without an `@odata.type` is an
/// error, never a guess: the type is what decides user from group.
pub fn ingest_members_page(body: &str) -> Result<MembersPage, AzError> {
    let v: Value = serde_json::from_str(body).map_err(|_| AzError::NotAPage)?;
    let items = v
        .get("value")
        .and_then(Value::as_array)
        .ok_or(AzError::NotAPage)?;
    let mut page = MembersPage {
        next_link: next_link(&v),
        ..MembersPage::default()
    };
    for (i, item) in items.iter().enumerate() {
        let obj = item.as_object().ok_or(AzError::BadId(i))?;
        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .and_then(|s| Guid128::parse(s).ok())
            .ok_or(AzError::BadId(i))?;
        let kind = match obj.get("@odata.type").and_then(Value::as_str) {
            Some("#microsoft.graph.user") => MemberKind::User,
            Some("#microsoft.graph.group") => MemberKind::Group,
            Some(_) => {
                page.skipped += 1;
                continue;
            }
            None => return Err(AzError::BadType(i, "@odata.type")),
        };
        page.members.push((id, kind));
    }
    Ok(page)
}

fn next_link(v: &Value) -> Option<String> {
    v.get("@odata.nextLink")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn is_group(rec: &DirRecord) -> bool {
    rec.schema().family == SchemaFamily::MsGraph && rec.object_kind() == AzKind::Group as u16
}

fn def(name: &str, kind: AttrKind) -> Option<&'static AttrDef> {
    GROUP_SCHEMA_V1
        .iter()
        .find(|d| d.name == name && d.kind == kind)
}

/// A single-valued string attribute of a group record. `None` for absent
/// values, unknown names and records that are not groups.
pub fn group_str<'p>(rec: &DirRecord, pool: &'p ValuePool, name: &str) -> Option<&'p str> {
    let d = def(name, AttrKind::Str).filter(|_| is_group(rec))?;
    std::str::from_utf8(pool.get(rec.str_ref(d.slot as usize)?)?).ok()
}

/// A multi-valued string attribute of a group record.
pub fn group_multi<'p>(rec: &DirRecord, pool: &'p ValuePool, name: &str) -> Option<Vec<&'p str>> {
    let d = def(name, AttrKind::MultiStr).filter(|_| is_group(rec))?;
    pool.get_multi(rec.str_ref(d.slot as usize)?)?
        .into_iter()
        .map(|v| std::str::from_utf8(v).ok())
        .collect()
}

/// A boolean attribute of a group record, `None` when it was not read.
pub fn group_flag(rec: &DirRecord, name: &str) -> Option<bool> {
    let d = def(name, AttrKind::Bool).filter(|_| is_group(rec))?;
    rec.num(d.slot as usize).map(|n| n != 0)
}
