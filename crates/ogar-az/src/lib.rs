//! # ogar-az — Entra ID as observed through Microsoft Graph
//!
//! Encodes one Graph `user` object into one [`DirRecord`]:
//!
//! * WHO   = Graph `id` (the Entra object id). **Never** the AD `objectGUID`:
//!   a synchronized identity is two nodes, related by a
//!   [`DirEdge`] ([`sync_edges`]), not one.
//! * WHERE = tenant id + (when present) the OU chain of
//!   `onPremisesDistinguishedName` as an [`OuHhtl`](ogar_dir_core::OuHhtl).
//!   That HHTL is *evidence about the on-premises location*, not a Graph
//!   location. Pass the on-premises domain's [`OuDictionary`] (the same one
//!   `ogar-ad` uses) and the AD and AZ HHTLs of one OU coincide.
//! * WHAT  = the attributes in [`SCHEMA_V1`], raw.
//!
//! The `$select` projection is derived from the schema ([`select_query`]),
//! never maintained as a separate list. Read-only: nothing here writes to
//! Graph; this crate does not even perform HTTP (see `examples/az_ingest.rs`).

use ogar_dir_core::dn::Dn;
use ogar_dir_core::edge::{DirEdge, EdgeEvidence, EdgeKind};
use ogar_dir_core::record::{FLAG_DN_UNENCODED, FLAG_NON_OU_CONTAINER};
use ogar_dir_core::{
    AttrDef, AttrKind, DirRecord, Guid128, OuDictionary, SchemaFamily, SchemaId, ValuePool, base64,
};
use serde_json::Value;
use std::collections::HashSet;

/// Encoder schema version understood by this crate.
pub const SCHEMA_VERSION: u16 = 1;
/// This crate's schema id.
pub const SCHEMA: SchemaId = SchemaId {
    family: SchemaFamily::MsGraph,
    version: SCHEMA_VERSION,
};

/// The embedded Graph `user` attribute table, v1.
///
/// Selection: naming (`userPrincipalName`, `mail`, `mailNickname`,
/// `proxyAddresses`, display triplet), state (`accountEnabled`, `userType`),
/// HR-ish context (`employeeId`, `department`, `companyName`,
/// `officeLocation`), and every `onPremises*` property that is *evidence about
/// the AD counterpart* (DN, sAMAccountName, domain, immutable id, SID, sync
/// flag, last sync). Licences, sign-in activity and group membership are out
/// of scope (the last becomes edges).
pub const SCHEMA_V1: &[AttrDef] = &[
    AttrDef {
        name: "userPrincipalName",
        slot: 0,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "displayName",
        slot: 1,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "givenName",
        slot: 2,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "surname",
        slot: 3,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "mail",
        slot: 4,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "mailNickname",
        slot: 5,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "proxyAddresses",
        slot: 6,
        kind: AttrKind::MultiStr,
        since: 1,
    },
    AttrDef {
        name: "employeeId",
        slot: 7,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "department",
        slot: 8,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "companyName",
        slot: 9,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "officeLocation",
        slot: 10,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "userType",
        slot: 11,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "onPremisesDistinguishedName",
        slot: 12,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "onPremisesSamAccountName",
        slot: 13,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "onPremisesDomainName",
        slot: 14,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "onPremisesImmutableId",
        slot: 15,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "onPremisesSecurityIdentifier",
        slot: 16,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "onPremisesLastSyncDateTime",
        slot: 17,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "accountEnabled",
        slot: 0,
        kind: AttrKind::Bool,
        since: 1,
    },
    AttrDef {
        name: "onPremisesSyncEnabled",
        slot: 1,
        kind: AttrKind::Bool,
        since: 1,
    },
];

/// AZ object kinds (family-scoped codes). Only users are ingested in v1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum AzKind {
    /// A Graph `user`.
    User = 1,
}

/// `$select` value for schema `version`: `id` plus every attribute defined at
/// or before that version, in table order.
pub fn select_query(version: u16) -> String {
    std::iter::once("id")
        .chain(
            SCHEMA_V1
                .iter()
                .filter(|a| a.since <= version)
                .map(|a| a.name),
        )
        .collect::<Vec<_>>()
        .join(",")
}

/// The read-only list URL for users (v1.0 endpoint).
pub fn users_url(version: u16, page_size: u16) -> String {
    format!(
        "https://graph.microsoft.com/v1.0/users?$select={}&$top={}",
        select_query(version),
        page_size.clamp(1, 999)
    )
}

/// Encoding failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AzError {
    /// The page is not `{ "value": [...] }`.
    NotAPage,
    /// Object `index` has no parseable `id`.
    BadId(usize),
    /// Object `index`, attribute `name` has the wrong JSON type.
    BadType(usize, &'static str),
    /// Pool or slot failure.
    Storage(String),
}

impl std::fmt::Display for AzError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "az encode: {self:?}")
    }
}
impl std::error::Error for AzError {}

/// One ingested page.
#[derive(Debug, Clone)]
pub struct Page {
    /// Records, in page order.
    pub records: Vec<DirRecord>,
    /// `@odata.nextLink`, if Graph returned one (follow it to continue).
    pub next_link: Option<String>,
    /// Distinct property names seen but not in the schema (not stored).
    pub ignored: Vec<String>,
}

/// Encode one Graph user object.
pub fn encode_user(
    obj: &serde_json::Map<String, Value>,
    index: usize,
    tenant: Guid128,
    onprem_dict: &mut OuDictionary,
    pool: &mut ValuePool,
    observed_at_ms: i64,
) -> Result<DirRecord, AzError> {
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .and_then(|s| Guid128::parse(s).ok())
        .ok_or(AzError::BadId(index))?;
    let mut rec = DirRecord::new(SCHEMA, AzKind::User as u16, id, tenant, observed_at_ms);
    let st = |e: &dyn std::fmt::Debug| AzError::Storage(format!("{e:?}"));

    for def in SCHEMA_V1 {
        // Graph returns `null` for unset properties: that is absence.
        let Some(v) = obj.get(def.name).filter(|v| !v.is_null()) else {
            continue;
        };
        let bad = || AzError::BadType(index, def.name);
        match def.kind {
            AttrKind::Str | AttrKind::Bytes => {
                let s = v.as_str().ok_or_else(bad)?;
                let r = pool.push(s.as_bytes()).map_err(|e| st(&e))?;
                rec.set_str(def.slot as usize, r).map_err(|e| st(&e))?;
            }
            AttrKind::MultiStr => {
                let arr = v.as_array().ok_or_else(bad)?;
                let vals: Vec<&str> = arr
                    .iter()
                    .map(|x| x.as_str().ok_or_else(bad))
                    .collect::<Result<_, _>>()?;
                let r = pool.push_multi(&vals).map_err(|e| st(&e))?;
                rec.set_str(def.slot as usize, r).map_err(|e| st(&e))?;
            }
            AttrKind::Bool => {
                let b = v.as_bool().ok_or_else(bad)?;
                rec.set_num(def.slot as usize, b as u32)
                    .map_err(|e| st(&e))?;
            }
            AttrKind::U32 => {
                let n = v
                    .as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or_else(bad)?;
                rec.set_num(def.slot as usize, n).map_err(|e| st(&e))?;
            }
        }
    }

    if let Some(dn) = obj
        .get("onPremisesDistinguishedName")
        .and_then(Value::as_str)
    {
        match Dn::parse(dn) {
            Ok(dn) => {
                if dn.has_non_ou_container() {
                    rec.add_flags(FLAG_NON_OU_CONTAINER);
                }
                match onprem_dict.intern(&dn.ou_path_root_first()) {
                    Ok(h) => rec.set_ou_hhtl(h),
                    Err(_) => rec.add_flags(FLAG_DN_UNENCODED),
                }
            }
            Err(_) => rec.add_flags(FLAG_DN_UNENCODED),
        }
    }
    Ok(rec)
}

/// Ingest one Graph list-page body (`GET /users?$select=…`).
pub fn ingest_page(
    body: &str,
    tenant: Guid128,
    onprem_dict: &mut OuDictionary,
    pool: &mut ValuePool,
    observed_at_ms: i64,
) -> Result<Page, AzError> {
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
            if k != "id" && !SCHEMA_V1.iter().any(|d| d.name == k) && !ignored.contains(k) {
                ignored.push(k.clone());
            }
        }
        records.push(encode_user(
            obj,
            i,
            tenant,
            onprem_dict,
            pool,
            observed_at_ms,
        )?);
    }
    ignored.sort();
    let next_link = v
        .get("@odata.nextLink")
        .and_then(Value::as_str)
        .map(str::to_string);
    Ok(Page {
        records,
        next_link,
        ignored,
    })
}

/// Read a single-valued string slot by attribute name. `None` for absent
/// values and for attributes that are not [`AttrKind::Str`] (a multi-valued
/// slot is a length-prefixed blob, never a string — use [`attr_multi`]).
pub fn attr_str<'p>(rec: &DirRecord, pool: &'p ValuePool, name: &str) -> Option<&'p str> {
    let def = SCHEMA_V1
        .iter()
        .find(|d| d.name == name && d.kind == AttrKind::Str)?;
    std::str::from_utf8(pool.get(rec.str_ref(def.slot as usize)?)?).ok()
}

/// Read a multi-valued string slot by attribute name. `None` for absent
/// values and for attributes that are not [`AttrKind::MultiStr`].
pub fn attr_multi<'p>(rec: &DirRecord, pool: &'p ValuePool, name: &str) -> Option<Vec<&'p str>> {
    let def = SCHEMA_V1
        .iter()
        .find(|d| d.name == name && d.kind == AttrKind::MultiStr)?;
    pool.get_multi(rec.str_ref(def.slot as usize)?)?
        .into_iter()
        .map(|v| std::str::from_utf8(v).ok())
        .collect()
}

/// `SynchronizesTo` edges for AZ records whose `onPremisesImmutableId`
/// base64-decodes to the `objectGUID` of an **observed** AD object.
///
/// This is evidence, not a decision: anchors that are not a 16-byte GUID
/// (e.g. custom source anchors) yield no edge, and an immutable id matching no
/// observed AD object yields no dangling edge. The two nodes stay distinct.
pub fn sync_edges(
    az: &[DirRecord],
    pool: &ValuePool,
    observed_ad: &HashSet<Guid128>,
) -> Vec<DirEdge> {
    az.iter()
        .filter_map(|r| {
            let raw = base64::decode(attr_str(r, pool, "onPremisesImmutableId")?)?;
            let ad = Guid128::from_ms_bytes(&raw).ok()?;
            observed_ad.contains(&ad).then_some(DirEdge {
                kind: EdgeKind::SynchronizesTo,
                evidence: EdgeEvidence::ImmutableIdIsObjectGuid,
                src: (SchemaFamily::AdDs, ad),
                dst: (SchemaFamily::MsGraph, r.node_guid()),
            })
        })
        .collect()
}
