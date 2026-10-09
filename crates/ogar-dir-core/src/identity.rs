//! Identity rules: how one system's spelling of an id relates to another's.
//!
//! An [`IdentityRule`] is a declaration, not code: *this field on this
//! family, in this external encoding, names the same 128-bit id as that
//! field on that family, in that encoding*. The encoding is applied once at
//! the boundary ([`Encoding::decode`]); afterwards both sides are
//! [`Guid128`] and compare as 16 bytes ([`Compare::Canonical128`]).
//!
//! A rule has two forms:
//!
//! * the declaration ([`IdentityRule`]) — attribute names, readable;
//! * the compiled descriptor ([`CompiledRule`]) — the same rule with every
//!   name resolved to a numeric slot of a schema table. Only this form is
//!   used to build lanes; no name is looked up per row.
//!
//! The rules here are the hybrid-identity correspondences. The source
//! anchor is a deployment choice: [`CONSISTENCY_GUID_TO_IMMUTABLE_ID`] when
//! `mS-DS-ConsistencyGuid` is the anchor, [`OBJECT_GUID_TO_IMMUTABLE_ID`]
//! when `objectGUID` is. They are different rules; one never falls back to
//! the other.

use crate::base64;
use crate::guid::Guid128;
use crate::label::LabelPattern;
use crate::record::{DirRecord, STR_SLOTS};
use crate::schema::{AttrDef, AttrKind, SchemaFamily};

/// How a system spells a 128-bit id on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// 16 raw bytes, Microsoft mixed-endian (`objectGUID`,
    /// `mS-DS-ConsistencyGuid`).
    MsGuidBytes16,
    /// Padded base64 of those 16 bytes (`onPremisesImmutableId`).
    Base64MsGuidBytes16,
    /// A label around the textual GUID (`User_{0}`).
    Labeled(LabelPattern),
    /// The textual GUID (`id`, `ExternalDirectoryObjectId`).
    GuidText,
}

/// Why a value is not an id in its declared encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IdFault {
    /// Present, but not the encoding (wrong length, bad character, wrong
    /// label, a spelling that does not render back).
    Malformed,
    /// Well-formed, but the all-zero id, which names no object.
    Nil,
}

impl Encoding {
    /// Decode a value to its id. Strict: only a spelling that renders back
    /// to exactly `raw` is accepted, so two different spellings can never
    /// decode to one id.
    pub fn decode(self, raw: &[u8]) -> Result<Guid128, IdFault> {
        let g = match self {
            Self::MsGuidBytes16 => Guid128::from_ms_bytes(raw).map_err(|_| IdFault::Malformed)?,
            Self::Base64MsGuidBytes16 => {
                let s = std::str::from_utf8(raw).map_err(|_| IdFault::Malformed)?;
                let bytes = base64::decode(s).ok_or(IdFault::Malformed)?;
                let g = Guid128::from_ms_bytes(&bytes).map_err(|_| IdFault::Malformed)?;
                if base64::encode(&bytes) != s {
                    return Err(IdFault::Malformed);
                }
                g
            }
            Self::Labeled(p) => {
                let s = std::str::from_utf8(raw).map_err(|_| IdFault::Malformed)?;
                // `strip` already refuses nil and non-canonical spellings;
                // tell the two apart.
                match p.strip(s) {
                    Some(g) => g,
                    None if p.strip_any(s) == Some(Guid128::NIL) => return Err(IdFault::Nil),
                    None => return Err(IdFault::Malformed),
                }
            }
            Self::GuidText => {
                let s = std::str::from_utf8(raw).map_err(|_| IdFault::Malformed)?;
                let g = Guid128::parse(s).map_err(|_| IdFault::Malformed)?;
                if g.to_string() != s {
                    return Err(IdFault::Malformed);
                }
                g
            }
        };
        if g.is_nil() { Err(IdFault::Nil) } else { Ok(g) }
    }

    /// Spell an id in this encoding (egress only).
    pub fn encode(self, id: Guid128) -> Vec<u8> {
        match self {
            Self::MsGuidBytes16 => id.to_ms_bytes().to_vec(),
            Self::Base64MsGuidBytes16 => base64::encode(&id.to_ms_bytes()).into_bytes(),
            Self::Labeled(p) => p.render(id).into_bytes(),
            Self::GuidText => id.to_string().into_bytes(),
        }
    }
}

/// How the two decoded sides compare. There is one way: the 16 bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Compare {
    /// Byte equality of the canonical [`Guid128`]; no text, no hash.
    Canonical128,
}

/// Which value of an object a rule reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    /// The object's own id (AD `objectGUID`, Graph `id`).
    Node,
    /// A schema attribute, by its source-native name.
    Attr(&'static str),
}

/// One side of a rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Endpoint {
    /// The source system.
    pub family: SchemaFamily,
    /// The value read.
    pub field: Field,
    /// How the source spells it.
    pub encoding: Encoding,
}

/// A declared identity correspondence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IdentityRule {
    /// Reported name (diagnostics only).
    pub name: &'static str,
    /// Where the id comes from.
    pub source: Endpoint,
    /// Where it is expected again.
    pub target: Endpoint,
    /// How the two compare.
    pub compare_as: Compare,
}

const fn ep(family: SchemaFamily, field: Field, encoding: Encoding) -> Endpoint {
    Endpoint {
        family,
        field,
        encoding,
    }
}

/// Source anchor `mS-DS-ConsistencyGuid` → cloud `onPremisesImmutableId`.
pub const CONSISTENCY_GUID_TO_IMMUTABLE_ID: IdentityRule = IdentityRule {
    name: "mS-DS-ConsistencyGuid -> onPremisesImmutableId",
    source: ep(
        SchemaFamily::AdDs,
        Field::Attr("mS-DS-ConsistencyGuid"),
        Encoding::MsGuidBytes16,
    ),
    target: ep(
        SchemaFamily::MsGraph,
        Field::Attr("onPremisesImmutableId"),
        Encoding::Base64MsGuidBytes16,
    ),
    compare_as: Compare::Canonical128,
};

/// Source anchor `objectGUID` → cloud `onPremisesImmutableId`, for
/// deployments that anchor on `objectGUID`. Not a fallback of
/// [`CONSISTENCY_GUID_TO_IMMUTABLE_ID`].
pub const OBJECT_GUID_TO_IMMUTABLE_ID: IdentityRule = IdentityRule {
    name: "objectGUID -> onPremisesImmutableId",
    source: ep(SchemaFamily::AdDs, Field::Node, Encoding::MsGuidBytes16),
    target: ep(
        SchemaFamily::MsGraph,
        Field::Attr("onPremisesImmutableId"),
        Encoding::Base64MsGuidBytes16,
    ),
    compare_as: Compare::Canonical128,
};

/// Backsync: AD `msDS-ExternalDirectoryObjectId` (`User_<id>`) → Entra `id`.
pub const BACKSYNC_TO_ENTRA_ID: IdentityRule = IdentityRule {
    name: "msDS-ExternalDirectoryObjectId -> id",
    source: ep(
        SchemaFamily::AdDs,
        Field::Attr("msDS-ExternalDirectoryObjectId"),
        Encoding::Labeled(LabelPattern("User_{0}")),
    ),
    target: ep(SchemaFamily::MsGraph, Field::Node, Encoding::GuidText),
    compare_as: Compare::Canonical128,
};

/// Entra `id` → Exchange Online `ExternalDirectoryObjectId`.
pub const ENTRA_ID_TO_EXO_EXTERNAL_ID: IdentityRule = IdentityRule {
    name: "id -> ExternalDirectoryObjectId",
    source: ep(SchemaFamily::MsGraph, Field::Node, Encoding::GuidText),
    target: ep(
        SchemaFamily::ExchangeOnline,
        Field::Attr("ExternalDirectoryObjectId"),
        Encoding::GuidText,
    ),
    compare_as: Compare::Canonical128,
};

/// Where a compiled side reads its value in a [`DirRecord`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    /// `node_guid` (already canonical).
    Node,
    /// An inline guid slot (decoded at ingest, already canonical).
    Guid(u8),
    /// A pooled string slot holding the external spelling; decoded with the
    /// side's [`Encoding`] when the lane is built.
    Pooled(u8),
}

/// One compiled side: numeric only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Selector {
    /// Family the record must carry.
    pub family: SchemaFamily,
    /// Where the value is.
    pub slot: Slot,
    /// The external encoding (applied only for [`Slot::Pooled`]).
    pub encoding: Encoding,
}

/// A rule with every name resolved against the schema tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CompiledRule {
    /// Source side.
    pub source: Selector,
    /// Target side.
    pub target: Selector,
}

/// Why a rule does not compile against the given tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompileError {
    /// The attribute is not in the table.
    Unknown(&'static str),
    /// The attribute is stored in a way the encoding cannot be read from
    /// (a multi-valued slot, a numeric slot, raw bytes for a text encoding).
    Incompatible(&'static str),
}

fn select(e: Endpoint, table: &[AttrDef]) -> Result<Selector, CompileError> {
    let slot = match e.field {
        Field::Node => Slot::Node,
        Field::Attr(name) => {
            let d = table
                .iter()
                .find(|d| d.name == name)
                .ok_or(CompileError::Unknown(name))?;
            match (d.kind, e.encoding) {
                (AttrKind::Guid, _) => Slot::Guid(d.slot),
                (AttrKind::Str, Encoding::MsGuidBytes16) => {
                    return Err(CompileError::Incompatible(name));
                }
                (AttrKind::Str, _) | (AttrKind::Bytes, Encoding::MsGuidBytes16) => {
                    Slot::Pooled(d.slot)
                }
                _ => return Err(CompileError::Incompatible(name)),
            }
        }
    };
    Ok(Selector {
        family: e.family,
        slot,
        encoding: e.encoding,
    })
}

impl IdentityRule {
    /// Resolve both sides against their families' schema tables.
    pub fn compile(
        &self,
        source_table: &[AttrDef],
        target_table: &[AttrDef],
    ) -> Result<CompiledRule, CompileError> {
        Ok(CompiledRule {
            source: select(self.source, source_table)?,
            target: select(self.target, target_table)?,
        })
    }
}

/// What a record holds for one selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IdValue {
    /// No value (presence bit clear).
    Absent,
    /// A value that is not an id in the declared encoding.
    Fault(IdFault),
    /// The id.
    Id(Guid128),
}

impl Selector {
    /// Read the value from a record (boundary: may decode a pooled string).
    /// A record of another family is [`IdValue::Absent`].
    pub fn read(&self, rec: &DirRecord, pool: &crate::ValuePool) -> IdValue {
        if rec.schema().family != self.family {
            return IdValue::Absent;
        }
        let g = match self.slot {
            Slot::Node => Some(rec.node_guid()),
            Slot::Guid(s) => rec.guid(s as usize),
            Slot::Pooled(s) if (s as usize) < STR_SLOTS => {
                let Some(r) = rec.str_ref(s as usize) else {
                    return IdValue::Absent;
                };
                return match pool.get(r) {
                    None => IdValue::Fault(IdFault::Malformed),
                    Some(b) => match self.encoding.decode(b) {
                        Ok(g) => IdValue::Id(g),
                        Err(f) => IdValue::Fault(f),
                    },
                };
            }
            Slot::Pooled(_) => None,
        };
        match g {
            None => IdValue::Absent,
            Some(g) if g.is_nil() => IdValue::Fault(IdFault::Nil),
            Some(g) => IdValue::Id(g),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "3f2504e0-4f89-11d3-9a0c-0305e82c3301";
    const MS: [u8; 16] = [
        0xe0, 0x04, 0x25, 0x3f, 0x89, 0x4f, 0xd3, 0x11, 0x9a, 0x0c, 0x03, 0x05, 0xe8, 0x2c, 0x33,
        0x01,
    ];
    const B64: &str = "4AQlP4lP0xGaDAMF6CwzAQ==";

    #[test]
    fn every_encoding_round_trips_the_known_vector() {
        let g = Guid128::parse(ID).unwrap();
        for (enc, wire) in [
            (Encoding::MsGuidBytes16, MS.to_vec()),
            (Encoding::Base64MsGuidBytes16, B64.as_bytes().to_vec()),
            (
                Encoding::Labeled(LabelPattern("User_{0}")),
                format!("User_{ID}").into_bytes(),
            ),
            (Encoding::GuidText, ID.as_bytes().to_vec()),
        ] {
            assert_eq!(enc.decode(&wire), Ok(g), "{enc:?}");
            assert_eq!(enc.encode(g), wire, "{enc:?}");
        }
    }

    /// F9: reading the mixed-endian bytes as if they were textual order is
    /// a different, equally valid-looking id.
    #[test]
    fn byte_order_is_not_a_no_op() {
        let right = Encoding::MsGuidBytes16.decode(&MS).unwrap();
        assert_ne!(right, Guid128(MS));
        assert_eq!(right.to_string(), ID);
    }

    #[test]
    fn malformed_and_nil_are_different_faults() {
        let labeled = Encoding::Labeled(LabelPattern("User_{0}"));
        for (enc, wire, fault) in [
            (Encoding::MsGuidBytes16, vec![0u8; 15], IdFault::Malformed),
            (Encoding::MsGuidBytes16, vec![0u8; 16], IdFault::Nil),
            (
                Encoding::Base64MsGuidBytes16,
                b"4AQlP4lP0xGaDAMF6CwzAQ".to_vec(), // unpadded
                IdFault::Malformed,
            ),
            (
                Encoding::Base64MsGuidBytes16,
                b"AAAAAAAAAAAAAAAAAAAAAA==".to_vec(),
                IdFault::Nil,
            ),
            (
                Encoding::Base64MsGuidBytes16,
                b"4AQlP4lP0xGaDAMF6Cwz".to_vec(), // 15 bytes
                IdFault::Malformed,
            ),
            (
                labeled,
                format!("Group_{ID}").into_bytes(),
                IdFault::Malformed,
            ),
            (
                labeled,
                format!("User_{}", ID.to_uppercase()).into_bytes(),
                IdFault::Malformed,
            ),
            (
                labeled,
                b"User_00000000-0000-0000-0000-000000000000".to_vec(),
                IdFault::Nil,
            ),
            (
                Encoding::GuidText,
                ID.to_uppercase().into_bytes(),
                IdFault::Malformed,
            ),
            (
                Encoding::GuidText,
                b"00000000-0000-0000-0000-000000000000".to_vec(),
                IdFault::Nil,
            ),
        ] {
            assert_eq!(enc.decode(&wire), Err(fault), "{enc:?} {wire:?}");
        }
    }

    #[test]
    fn the_two_anchor_rules_read_different_fields() {
        assert_ne!(
            CONSISTENCY_GUID_TO_IMMUTABLE_ID.source.field,
            OBJECT_GUID_TO_IMMUTABLE_ID.source.field
        );
        assert_eq!(
            CONSISTENCY_GUID_TO_IMMUTABLE_ID.target,
            OBJECT_GUID_TO_IMMUTABLE_ID.target
        );
    }
}
