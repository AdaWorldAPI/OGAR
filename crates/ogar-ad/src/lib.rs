//! # ogar-ad — Active Directory as observed
//!
//! Encodes one LDAP entry into one [`DirRecord`]:
//!
//! * WHO   = `objectGUID` (raw LDAP bytes, Microsoft mixed-endian → textual order)
//! * WHERE = the domain GUID (caller-supplied) + the OU chain of the DN as an
//!   [`OuHhtl`](ogar_dir_core::OuHhtl), interned in a per-domain
//!   [`OuDictionary`]
//! * WHAT  = the attributes in [`SCHEMA_V1`], stored raw in a [`ValuePool`]
//!
//! It does not interpret `userAccountControl`, does not compute "enabled",
//! does not normalise addresses. Derived views are functions over the raw
//! record, never stored in it.
//!
//! Ingest is read-only: [`ldif::parse`] reads `ldapsearch`/`ldifde` output.

pub mod ldif;

use ogar_dir_core::dn::Dn;
use ogar_dir_core::label::{self, CloudLabel};
use ogar_dir_core::record::{FLAG_DN_UNENCODED, FLAG_NON_OU_CONTAINER};
use ogar_dir_core::{
    AttrDef, AttrKind, BAG_LEN, DirRecord, Guid128, OuDictionary, SchemaFamily, SchemaId, ValuePool,
};

/// Encoder schema version understood by this crate.
pub const SCHEMA_VERSION: u16 = 5;
/// This crate's schema id.
pub const SCHEMA: SchemaId = SchemaId {
    family: SchemaFamily::AdDs,
    version: SCHEMA_VERSION,
};

/// The embedded AD attribute table. Append-only: each entry carries the
/// schema version that introduced it (`since`); the name is kept from v1.
///
/// Selection: the attributes needed to (a) name an account in every form AD
/// and Exchange use (`sAMAccountName`, `userPrincipalName`, `mail`,
/// `mailNickname`, `proxyAddresses`, `targetAddress`), (b) say what it is
/// (`objectClass`, `objectSid`, `userAccountControl`), (c) say where it is
/// (`distinguishedName`) and (d) date it (`whenCreated`, `whenChanged`), plus
/// the display triplet. Group membership (`memberOf`/`member`) is a relation
/// and becomes edges later, never an inline list.
///
/// v2 adds the Exchange recipient triplet, raw, for the hybrid recipient
/// model (`ogar-dir-sim::exchange`): `msExchRemoteRecipientType` (flags,
/// numeric), `msExchRecipientDisplayType` (signed, numeric, bit-cast like
/// every signed LDAP integer here) and `msExchRecipientTypeDetails` (64-bit,
/// so kept as its LDAP decimal text in a pooled slot). Other `msExch*`
/// attributes stay out until a consumer needs them.
///
/// v3 adds the hybrid identity anchors as 128-bit ids in inline guid slots,
/// never as text: `mS-DS-ConsistencyGuid` (16 mixed-endian bytes, the source
/// anchor whose base64 is the cloud `ImmutableId`) and
/// `msDS-ExternalDirectoryObjectId` (written back by Entra Connect as
/// `User_<cloud object id>`). The `User_` / `Group_` label is stripped on the
/// way in — it must match the object's kind — and added back on the way out
/// from the kind ([`external_directory_object_id`]); it is never stored.
///
/// v4 adds `physicalDeliveryOfficeName`, the attribute Entra Connect syncs
/// to the cloud's `officeLocation` (see `ogar_az::SYNCED`).
///
/// v5 adds `extensionAttribute1`..`15` as one positional bag
/// ([`AttrKind::Bag`]), the shape Graph returns them in
/// (`onPremisesExtensionAttributes`).
pub const SCHEMA_V1: &[AttrDef] = &[
    AttrDef {
        name: "distinguishedName",
        slot: 0,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "objectSid",
        slot: 1,
        kind: AttrKind::Bytes,
        since: 1,
    },
    AttrDef {
        name: "sAMAccountName",
        slot: 2,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "userPrincipalName",
        slot: 3,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "objectClass",
        slot: 4,
        kind: AttrKind::MultiStr,
        since: 1,
    },
    AttrDef {
        name: "displayName",
        slot: 5,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "givenName",
        slot: 6,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "sn",
        slot: 7,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "mail",
        slot: 8,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "mailNickname",
        slot: 9,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "proxyAddresses",
        slot: 10,
        kind: AttrKind::MultiStr,
        since: 1,
    },
    AttrDef {
        name: "targetAddress",
        slot: 11,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "whenCreated",
        slot: 12,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "whenChanged",
        slot: 13,
        kind: AttrKind::Str,
        since: 1,
    },
    AttrDef {
        name: "userAccountControl",
        slot: 0,
        kind: AttrKind::U32,
        since: 1,
    },
    AttrDef {
        name: "msExchRemoteRecipientType",
        slot: 1,
        kind: AttrKind::U32,
        since: 2,
    },
    AttrDef {
        name: "msExchRecipientDisplayType",
        slot: 2,
        kind: AttrKind::U32,
        since: 2,
    },
    AttrDef {
        name: "msExchRecipientTypeDetails",
        slot: 14,
        kind: AttrKind::Str,
        since: 2,
    },
    AttrDef {
        name: "mS-DS-ConsistencyGuid",
        slot: 0,
        kind: AttrKind::Guid,
        since: 3,
    },
    AttrDef {
        name: "msDS-ExternalDirectoryObjectId",
        slot: 1,
        kind: AttrKind::Guid,
        since: 3,
    },
    AttrDef {
        name: "physicalDeliveryOfficeName",
        slot: 15,
        kind: AttrKind::Str,
        since: 4,
    },
    AttrDef {
        name: "extensionAttribute",
        slot: 16,
        kind: AttrKind::Bag,
        since: 5,
    },
];

/// AD object kinds (family-scoped codes in `object_kind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum AdKind {
    /// None of the below.
    Other = 0,
    /// `user` (and not `computer`).
    User = 1,
    /// `group`.
    Group = 2,
    /// `computer` (which is also a `user` subclass in AD).
    Computer = 3,
    /// `contact`.
    Contact = 4,
    /// `organizationalUnit`.
    OrganizationalUnit = 5,
}

impl AdKind {
    /// From the record's `object_kind` code.
    pub fn from_code(code: u16) -> Option<Self> {
        [
            Self::Other,
            Self::User,
            Self::Group,
            Self::Computer,
            Self::Contact,
            Self::OrganizationalUnit,
        ]
        .into_iter()
        .find(|k| *k as u16 == code)
    }

    /// From `objectClass` values; most specific class wins.
    pub fn from_object_class<S: AsRef<str>>(classes: &[S]) -> Self {
        let has = |c: &str| classes.iter().any(|x| x.as_ref().eq_ignore_ascii_case(c));
        if has("computer") {
            Self::Computer
        } else if has("user") {
            Self::User
        } else if has("group") {
            Self::Group
        } else if has("contact") {
            Self::Contact
        } else if has("organizationalUnit") {
            Self::OrganizationalUnit
        } else {
            Self::Other
        }
    }
}

/// One LDAP entry as read: DN plus attribute values in source order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdEntry {
    /// The entry DN.
    pub dn: String,
    /// `(attribute name as written, raw value bytes)`, repeated for multi-values.
    pub attrs: Vec<(String, Vec<u8>)>,
}

impl AdEntry {
    /// All values of an attribute (case-insensitive name; `;options` ignored).
    pub fn values(&self, name: &str) -> Vec<&[u8]> {
        self.attrs
            .iter()
            .filter(|(n, _)| n.split(';').next().unwrap_or(n).eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_slice())
            .collect()
    }
}

/// Encoding failure (the entry is skipped, never half-written).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdError {
    /// No `objectGUID`, or not 16 bytes.
    MissingObjectGuid,
    /// A string attribute was not UTF-8.
    NotUtf8(&'static str),
    /// A single-valued attribute had several values.
    MultipleValues(&'static str),
    /// A numeric attribute did not parse.
    BadNumber(&'static str),
    /// A 128-bit id attribute was not a non-nil id in its source spelling
    /// (16 raw bytes; `<Label>_<guid>` whose label matches the object kind).
    BadId(&'static str),
    /// Pool or slot failure.
    Storage(String),
}

impl std::fmt::Display for AdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ad encode: {self:?}")
    }
}
impl std::error::Error for AdError {}

/// Result of encoding one entry.
#[derive(Debug, Clone)]
pub struct Encoded {
    /// The fixed record.
    pub record: DirRecord,
    /// Attribute names present in the entry but not in the schema. They are
    /// not stored and cannot affect the fixed ABI.
    pub ignored: Vec<String>,
}

/// The label an object of this kind carries in
/// `msDS-ExternalDirectoryObjectId`, if any.
pub fn cloud_label(kind: AdKind) -> Option<CloudLabel> {
    match kind {
        AdKind::User => Some(CloudLabel::User),
        AdKind::Group => Some(CloudLabel::Group),
        _ => None,
    }
}

/// Inbound: a 128-bit id attribute's source spelling → the id.
fn decode_id(name: &str, raw: &[u8], kind: u16) -> Option<Guid128> {
    let g = match name {
        "mS-DS-ConsistencyGuid" => Guid128::from_ms_bytes(raw).ok()?,
        "msDS-ExternalDirectoryObjectId" => {
            // The kind's own template: a User_ id on a group does not match.
            let label = cloud_label(AdKind::from_code(kind)?)?;
            label.pattern().strip(std::str::from_utf8(raw).ok()?)?
        }
        _ => return None,
    };
    (!g.is_nil()).then_some(g)
}

fn guid_slot(name: &str) -> usize {
    SCHEMA_V1
        .iter()
        .find(|d| d.name == name && d.kind == AttrKind::Guid)
        .expect("a guid attribute of SCHEMA_V1")
        .slot as usize
}

/// Outbound: `msDS-ExternalDirectoryObjectId` as AD spells it — the stored
/// id with the label of the record's kind added back.
pub fn external_directory_object_id(rec: &DirRecord) -> Option<String> {
    let g = rec.guid(guid_slot("msDS-ExternalDirectoryObjectId"))?;
    let label = cloud_label(AdKind::from_code(rec.object_kind())?)?;
    Some(label::render(label, g))
}

/// Outbound: `mS-DS-ConsistencyGuid` as AD stores it (16 mixed-endian bytes).
pub fn consistency_guid(rec: &DirRecord) -> Option<[u8; 16]> {
    Some(rec.guid(guid_slot("mS-DS-ConsistencyGuid"))?.to_ms_bytes())
}

/// `base` is a member of a bag attribute (`extensionAttribute7`).
fn is_bag_member(base: &str) -> bool {
    SCHEMA_V1
        .iter()
        .filter(|d| d.kind == AttrKind::Bag)
        .any(|d| {
            base.len() > d.name.len()
                && base[..d.name.len()].eq_ignore_ascii_case(d.name)
                && base[d.name.len()..]
                    .parse::<usize>()
                    .is_ok_and(|n| (1..=BAG_LEN).contains(&n))
        })
}

fn identity_or_dn(name: &str) -> bool {
    name.eq_ignore_ascii_case("objectGUID") || name.eq_ignore_ascii_case("dn")
}

/// Encode one entry. `domain` is the AD domain's GUID (the naming-context
/// head's `objectGUID`); `dict` must be that domain's OU dictionary.
pub fn encode(
    entry: &AdEntry,
    domain: Guid128,
    dict: &mut OuDictionary,
    pool: &mut ValuePool,
    observed_at_ms: i64,
) -> Result<Encoded, AdError> {
    let guid_vals = entry.values("objectGUID");
    let node = match guid_vals.as_slice() {
        [g] => Guid128::from_ms_bytes(g).map_err(|_| AdError::MissingObjectGuid)?,
        _ => return Err(AdError::MissingObjectGuid),
    };

    let classes: Vec<String> = entry
        .values("objectClass")
        .iter()
        .map(|v| String::from_utf8_lossy(v).into_owned())
        .collect();
    let kind = AdKind::from_object_class(&classes) as u16;
    let mut rec = DirRecord::new(SCHEMA, kind, node, domain, observed_at_ms);
    let st = |e: &dyn std::fmt::Debug| AdError::Storage(format!("{e:?}"));

    for def in SCHEMA_V1 {
        if def.kind == AttrKind::Bag {
            // Gather the numbered members into one positional bag.
            let mut members: Vec<&[u8]> = Vec::with_capacity(BAG_LEN);
            for n in 1..=BAG_LEN {
                match entry.values(&format!("{}{n}", def.name)).as_slice() {
                    [] => members.push(b""),
                    [v] if std::str::from_utf8(v).is_ok() => members.push(v),
                    [_] => return Err(AdError::NotUtf8(def.name)),
                    _ => return Err(AdError::MultipleValues(def.name)),
                }
            }
            if members.iter().any(|m| !m.is_empty()) {
                let r = pool.push_multi(&members).map_err(|e| st(&e))?;
                rec.set_str(def.slot as usize, r).map_err(|e| st(&e))?;
            }
            continue;
        }
        let mut vals = entry.values(def.name);
        // The DN line is authoritative for distinguishedName when the
        // attribute itself was not requested.
        if def.name == "distinguishedName" && vals.is_empty() && !entry.dn.is_empty() {
            vals = vec![entry.dn.as_bytes()];
        }
        if vals.is_empty() {
            continue;
        }
        match def.kind {
            AttrKind::Str | AttrKind::Bytes => {
                let [v] = vals.as_slice() else {
                    return Err(AdError::MultipleValues(def.name));
                };
                if def.kind == AttrKind::Str && std::str::from_utf8(v).is_err() {
                    return Err(AdError::NotUtf8(def.name));
                }
                let r = pool.push(v).map_err(|e| st(&e))?;
                rec.set_str(def.slot as usize, r).map_err(|e| st(&e))?;
            }
            // Gathered before the match.
            AttrKind::Bag => {}
            AttrKind::MultiStr => {
                if vals.iter().any(|v| std::str::from_utf8(v).is_err()) {
                    return Err(AdError::NotUtf8(def.name));
                }
                let r = pool.push_multi(&vals).map_err(|e| st(&e))?;
                rec.set_str(def.slot as usize, r).map_err(|e| st(&e))?;
            }
            AttrKind::Guid => {
                let [v] = vals.as_slice() else {
                    return Err(AdError::MultipleValues(def.name));
                };
                let g = decode_id(def.name, v, kind).ok_or(AdError::BadId(def.name))?;
                rec.set_guid(def.slot as usize, g).map_err(|e| st(&e))?;
            }
            AttrKind::U32 | AttrKind::Bool => {
                let [v] = vals.as_slice() else {
                    return Err(AdError::MultipleValues(def.name));
                };
                let s = std::str::from_utf8(v).map_err(|_| AdError::BadNumber(def.name))?;
                // LDAP integers are signed decimal; UAC fits in i32/u32.
                let n = s
                    .trim()
                    .parse::<i64>()
                    .ok()
                    .and_then(|n| {
                        u32::try_from(n)
                            .ok()
                            .or_else(|| i32::try_from(n).ok().map(|i| i as u32))
                    })
                    .ok_or(AdError::BadNumber(def.name))?;
                rec.set_num(def.slot as usize, n).map_err(|e| st(&e))?;
            }
        }
    }

    // WHERE: the OU chain of the DN. Never the leaf, never DC=.
    if !entry.dn.is_empty() {
        match Dn::parse(&entry.dn) {
            Ok(dn) => {
                if dn.has_non_ou_container() {
                    rec.add_flags(FLAG_NON_OU_CONTAINER);
                }
                match dict.intern(&dn.ou_path_root_first()) {
                    Ok(h) => rec.set_ou_hhtl(h),
                    Err(_) => rec.add_flags(FLAG_DN_UNENCODED),
                }
            }
            Err(_) => rec.add_flags(FLAG_DN_UNENCODED),
        }
    }

    let mut ignored: Vec<String> = entry
        .attrs
        .iter()
        .map(|(n, _)| n.clone())
        .filter(|n| {
            let base = n.split(';').next().unwrap_or(n);
            !identity_or_dn(base)
                && !SCHEMA_V1.iter().any(|d| d.name.eq_ignore_ascii_case(base))
                && !is_bag_member(base)
        })
        .collect();
    ignored.dedup();
    Ok(Encoded {
        record: rec,
        ignored,
    })
}
