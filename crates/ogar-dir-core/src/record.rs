//! The fixed 512-byte directory observation record.
//!
//! ## Exact layout (all multi-byte integers little-endian; no padding exists
//! because the record is one `[u8; 512]` addressed by the offsets below)
//!
//! ```text
//! 0x000..0x010  canonical_key    [16]  NodeRow key slot. ZERO in this PoC
//!                                       (= dormant default class). NOT the source GUID.
//! 0x010..0x020  canonical_edges  [16]  NodeRow edge-facet slot. ZERO; relations are DirEdge.
//! ---- 0x020..0x200 = the 480-byte NodeRow value slab -------------------------
//! 0x020..0x024  magic            [4]   b"OGDR"
//! 0x024..0x026  abi_major        u16   ABI_MAJOR (exact match required)
//! 0x026..0x028  abi_minor        u16   ABI_MINOR (additive; readers ignore newer)
//! 0x028..0x02A  schema_family    u16   SchemaFamily (1 = AD_DS, 2 = MS_GRAPH)
//! 0x02A..0x02C  schema_version   u16   encoder's schema version
//! 0x02C..0x02E  object_kind      u16   family-scoped kind code
//! 0x02E..0x030  flags            u16   FLAG_* below
//! 0x030..0x040  node_guid        [16]  WHO   source object id, textual byte order
//! 0x040..0x050  scope_guid       [16]  WHERE AD domain GUID / Entra tenant id
//! 0x050..0x060  ou_hhtl          [16]  WHERE 8 × u16 LE, root first
//! 0x060..0x068  observed_at_ms   i64   WHEN  unix ms the observation was taken (0 = unknown)
//! 0x068..0x070  presence         u64   bit s = pooled slot s present (0..32);
//!                                       bit 32+n = numeric slot n present (n < 4)
//! 0x070..0x080  num              4×u32 numeric slots (meaning per schema)
//! 0x080..0x180  str_refs         32×(u32 off, u32 len) into the batch ValuePool
//! 0x180..0x200  reserved         [128] zero; writers MUST zero, readers MUST ignore
//! ```
//!
//! GUIDs are stored in textual byte order (see [`crate::guid`]). Strings never
//! live in the record; an absent slot has its presence bit clear (a present
//! empty string is distinguishable from absence).
//!
//! The canonical `key|edges|value` split mirrors lance-graph-contract
//! `NodeRow` (16|16|480, stride 512). That mirror is **unguarded** in this
//! crate (no dependency on the contract); see the PoC conflict notes.

use crate::guid::Guid128;
use crate::hhtl::{HhtlError, OuHhtl};
use crate::pool::StrRef;
use crate::schema::{SchemaFamily, SchemaId};

/// Record size in bytes.
pub const RECORD_BYTES: usize = 512;
/// Layout major: bump only when an existing offset changes meaning.
pub const ABI_MAJOR: u16 = 1;
/// Layout minor: bump when reserved bytes gain meaning (additive).
pub const ABI_MINOR: u16 = 0;
/// Magic.
pub const MAGIC: [u8; 4] = *b"OGDR";
/// Pooled slot count.
pub const STR_SLOTS: usize = 32;
/// Numeric slot count.
pub const NUM_SLOTS: usize = 4;

/// Offsets (public so a non-Rust reader can be generated from them).
pub mod off {
    #![allow(missing_docs)]
    pub const CANONICAL_KEY: usize = 0x000;
    pub const CANONICAL_EDGES: usize = 0x010;
    pub const VALUE: usize = 0x020;
    pub const MAGIC: usize = 0x020;
    pub const ABI_MAJOR: usize = 0x024;
    pub const ABI_MINOR: usize = 0x026;
    pub const SCHEMA_FAMILY: usize = 0x028;
    pub const SCHEMA_VERSION: usize = 0x02A;
    pub const OBJECT_KIND: usize = 0x02C;
    pub const FLAGS: usize = 0x02E;
    pub const NODE_GUID: usize = 0x030;
    pub const SCOPE_GUID: usize = 0x040;
    pub const OU_HHTL: usize = 0x050;
    pub const OBSERVED_AT: usize = 0x060;
    pub const PRESENCE: usize = 0x068;
    pub const NUM: usize = 0x070;
    pub const STR_REFS: usize = 0x080;
    pub const RESERVED: usize = 0x180;
    pub const END: usize = 0x200;
}

/// `ou_hhtl` was derived from a parsed DN (else it is zero and meaningless).
pub const FLAG_OU_PRESENT: u16 = 1 << 0;
/// The location contains a non-OU, non-DC container (e.g. `CN=Users`).
pub const FLAG_NON_OU_CONTAINER: u16 = 1 << 1;
/// A DN was observed but could not be encoded (unparseable or > 8 OUs); the
/// raw DN is still in its slot.
pub const FLAG_DN_UNENCODED: u16 = 1 << 2;

// Layout lock.
const _: () = assert!(off::END == RECORD_BYTES);
const _: () = assert!(off::STR_REFS + STR_SLOTS * 8 == off::RESERVED);
const _: () = assert!(off::NUM + NUM_SLOTS * 4 == off::STR_REFS);
const _: () = assert!(off::VALUE + 480 == RECORD_BYTES);
const _: () = assert!(core::mem::size_of::<DirRecord>() == RECORD_BYTES);
const _: () = assert!(core::mem::align_of::<DirRecord>() == 64);

/// One observation. A thin typed view over exactly 512 bytes.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C, align(64))]
pub struct DirRecord {
    bytes: [u8; RECORD_BYTES],
}

/// Why bytes were not accepted as a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    /// Magic mismatch.
    BadMagic,
    /// Layout major differs.
    AbiMajor(u16),
    /// Unknown schema family.
    UnknownFamily(u16),
    /// Malformed HHTL.
    Hhtl(HhtlError),
    /// Slot index out of range.
    Slot(usize),
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dir record: {self:?}")
    }
}
impl std::error::Error for RecordError {}

impl std::fmt::Debug for DirRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirRecord")
            .field("schema", &self.schema())
            .field("kind", &self.object_kind())
            .field("node", &self.node_guid())
            .field("scope", &self.scope_guid())
            .field("ou_hhtl", &self.ou_hhtl())
            .field("flags", &self.flags())
            .finish()
    }
}

impl DirRecord {
    fn u16_at(&self, o: usize) -> u16 {
        u16::from_le_bytes([self.bytes[o], self.bytes[o + 1]])
    }
    fn put_u16(&mut self, o: usize, v: u16) {
        self.bytes[o..o + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn guid_at(&self, o: usize) -> Guid128 {
        Guid128(self.bytes[o..o + 16].try_into().expect("16"))
    }

    /// A fresh record: header, identity and scope set; no attributes.
    pub fn new(
        schema: SchemaId,
        kind: u16,
        node: Guid128,
        scope: Guid128,
        observed_at_ms: i64,
    ) -> Self {
        let mut r = Self {
            bytes: [0; RECORD_BYTES],
        };
        r.bytes[off::MAGIC..off::MAGIC + 4].copy_from_slice(&MAGIC);
        r.put_u16(off::ABI_MAJOR, ABI_MAJOR);
        r.put_u16(off::ABI_MINOR, ABI_MINOR);
        r.put_u16(off::SCHEMA_FAMILY, schema.family as u16);
        r.put_u16(off::SCHEMA_VERSION, schema.version);
        r.put_u16(off::OBJECT_KIND, kind);
        r.bytes[off::NODE_GUID..off::NODE_GUID + 16].copy_from_slice(&node.0);
        r.bytes[off::SCOPE_GUID..off::SCOPE_GUID + 16].copy_from_slice(&scope.0);
        r.bytes[off::OBSERVED_AT..off::OBSERVED_AT + 8]
            .copy_from_slice(&observed_at_ms.to_le_bytes());
        r
    }

    /// Validate and wrap raw bytes.
    pub fn from_bytes(b: &[u8; RECORD_BYTES]) -> Result<Self, RecordError> {
        let r = Self { bytes: *b };
        if r.bytes[off::MAGIC..off::MAGIC + 4] != MAGIC {
            return Err(RecordError::BadMagic);
        }
        let major = r.u16_at(off::ABI_MAJOR);
        if major != ABI_MAJOR {
            return Err(RecordError::AbiMajor(major));
        }
        let fam = r.u16_at(off::SCHEMA_FAMILY);
        SchemaFamily::from_u16(fam).ok_or(RecordError::UnknownFamily(fam))?;
        let h: [u8; 16] = r.bytes[off::OU_HHTL..off::OU_HHTL + 16]
            .try_into()
            .expect("16");
        OuHhtl::from_le_bytes(&h).map_err(RecordError::Hhtl)?;
        Ok(r)
    }

    /// The 512 wire bytes.
    pub fn as_bytes(&self) -> &[u8; RECORD_BYTES] {
        &self.bytes
    }

    /// `(major, minor)`.
    pub fn abi(&self) -> (u16, u16) {
        (self.u16_at(off::ABI_MAJOR), self.u16_at(off::ABI_MINOR))
    }

    /// Schema family + version.
    pub fn schema(&self) -> SchemaId {
        SchemaId {
            family: SchemaFamily::from_u16(self.u16_at(off::SCHEMA_FAMILY)).expect("validated"),
            version: self.u16_at(off::SCHEMA_VERSION),
        }
    }

    /// Family-scoped kind code.
    pub fn object_kind(&self) -> u16 {
        self.u16_at(off::OBJECT_KIND)
    }
    /// Flags.
    pub fn flags(&self) -> u16 {
        self.u16_at(off::FLAGS)
    }
    /// Source object id.
    pub fn node_guid(&self) -> Guid128 {
        self.guid_at(off::NODE_GUID)
    }
    /// Domain / tenant id.
    pub fn scope_guid(&self) -> Guid128 {
        self.guid_at(off::SCOPE_GUID)
    }
    /// Observation time.
    pub fn observed_at_ms(&self) -> i64 {
        i64::from_le_bytes(
            self.bytes[off::OBSERVED_AT..off::OBSERVED_AT + 8]
                .try_into()
                .expect("8"),
        )
    }

    /// OU path; `None` unless [`FLAG_OU_PRESENT`].
    pub fn ou_hhtl(&self) -> Option<OuHhtl> {
        if self.flags() & FLAG_OU_PRESENT == 0 {
            return None;
        }
        let h: [u8; 16] = self.bytes[off::OU_HHTL..off::OU_HHTL + 16]
            .try_into()
            .expect("16");
        OuHhtl::from_le_bytes(&h).ok()
    }

    /// Set the OU path (sets [`FLAG_OU_PRESENT`]).
    pub fn set_ou_hhtl(&mut self, h: OuHhtl) {
        self.bytes[off::OU_HHTL..off::OU_HHTL + 16].copy_from_slice(&h.to_le_bytes());
        self.add_flags(FLAG_OU_PRESENT);
    }

    /// OR flags in.
    pub fn add_flags(&mut self, f: u16) {
        let v = self.flags() | f;
        self.put_u16(off::FLAGS, v);
    }

    fn presence(&self) -> u64 {
        u64::from_le_bytes(
            self.bytes[off::PRESENCE..off::PRESENCE + 8]
                .try_into()
                .expect("8"),
        )
    }
    fn set_presence_bit(&mut self, bit: usize) {
        let v = self.presence() | (1u64 << bit);
        self.bytes[off::PRESENCE..off::PRESENCE + 8].copy_from_slice(&v.to_le_bytes());
    }

    /// Set a pooled slot.
    pub fn set_str(&mut self, slot: usize, r: StrRef) -> Result<(), RecordError> {
        if slot >= STR_SLOTS {
            return Err(RecordError::Slot(slot));
        }
        let o = off::STR_REFS + slot * 8;
        self.bytes[o..o + 8].copy_from_slice(&r.to_le_bytes());
        self.set_presence_bit(slot);
        Ok(())
    }

    /// Read a pooled slot.
    pub fn str_ref(&self, slot: usize) -> Option<StrRef> {
        if slot >= STR_SLOTS || self.presence() & (1 << slot) == 0 {
            return None;
        }
        let o = off::STR_REFS + slot * 8;
        Some(StrRef::from_le_bytes(
            self.bytes[o..o + 8].try_into().expect("8"),
        ))
    }

    /// Set a numeric slot.
    pub fn set_num(&mut self, slot: usize, v: u32) -> Result<(), RecordError> {
        if slot >= NUM_SLOTS {
            return Err(RecordError::Slot(slot));
        }
        let o = off::NUM + slot * 4;
        self.bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
        self.set_presence_bit(STR_SLOTS + slot);
        Ok(())
    }

    /// Read a numeric slot.
    pub fn num(&self, slot: usize) -> Option<u32> {
        if slot >= NUM_SLOTS || self.presence() & (1 << (STR_SLOTS + slot)) == 0 {
            return None;
        }
        let o = off::NUM + slot * 4;
        Some(u32::from_le_bytes(
            self.bytes[o..o + 4].try_into().expect("4"),
        ))
    }

    /// Reserved bytes are zero (writer obligation).
    pub fn reserved_is_zero(&self) -> bool {
        self.bytes[off::CANONICAL_KEY..off::VALUE]
            .iter()
            .all(|&b| b == 0)
            && self.bytes[off::RESERVED..off::END].iter().all(|&b| b == 0)
    }
}
