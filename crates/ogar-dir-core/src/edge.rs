//! Relationships are records of their own, never inline lists in the 512-byte
//! node. One fixed 64-byte edge:
//!
//! ```text
//! 0x00 kind        u16  EdgeKind
//! 0x02 evidence    u16  EdgeEvidence — WHY the edge was asserted
//! 0x04 src_family  u16  SchemaFamily of src
//! 0x06 dst_family  u16  SchemaFamily of dst
//! 0x08 reserved    [8]  zero
//! 0x10 src_guid    [16] textual byte order
//! 0x20 dst_guid    [16]
//! 0x30 reserved    [16] zero
//! ```
//!
//! Endpoints are `(family, guid)` pairs. An edge between an AD object and an
//! Entra object relates two **distinct nodes**; it never merges them.
//!
//! The vocabulary is deliberately minimal: only [`EdgeKind::SynchronizesTo`]
//! exists. `MEMBER_OF` / `MANAGER` are expected later and get the next codes;
//! they are not defined until a slice actually emits them.
//!
//! lance-graph's `CausalEdge64` was checked and does not fit: it packs a
//! reasoning edge into 8 bytes and cannot carry two 128-bit endpoints.

use crate::guid::Guid128;
use crate::schema::SchemaFamily;

/// Edge record size.
pub const EDGE_BYTES: usize = 64;

/// What the edge asserts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum EdgeKind {
    /// On-premises source object is synchronized to a cloud object (the
    /// SAME_AS candidate). Direction: AD → Entra.
    SynchronizesTo = 1,
}

/// Which observation supports the edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum EdgeEvidence {
    /// Graph `onPremisesImmutableId` base64-decodes to 16 bytes equal (as a
    /// Microsoft mixed-endian GUID) to an observed AD `objectGUID`.
    ImmutableIdIsObjectGuid = 1,
}

/// One edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirEdge {
    /// Kind.
    pub kind: EdgeKind,
    /// Evidence.
    pub evidence: EdgeEvidence,
    /// Source endpoint.
    pub src: (SchemaFamily, Guid128),
    /// Destination endpoint.
    pub dst: (SchemaFamily, Guid128),
}

impl DirEdge {
    /// 64 wire bytes.
    pub fn to_bytes(&self) -> [u8; EDGE_BYTES] {
        let mut b = [0u8; EDGE_BYTES];
        b[0..2].copy_from_slice(&(self.kind as u16).to_le_bytes());
        b[2..4].copy_from_slice(&(self.evidence as u16).to_le_bytes());
        b[4..6].copy_from_slice(&(self.src.0 as u16).to_le_bytes());
        b[6..8].copy_from_slice(&(self.dst.0 as u16).to_le_bytes());
        b[0x10..0x20].copy_from_slice(&self.src.1.0);
        b[0x20..0x30].copy_from_slice(&self.dst.1.0);
        b
    }

    /// Decode; `None` on unknown codes.
    pub fn from_bytes(b: &[u8; EDGE_BYTES]) -> Option<Self> {
        let u = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        let kind = match u(0) {
            1 => EdgeKind::SynchronizesTo,
            _ => return None,
        };
        let evidence = match u(2) {
            1 => EdgeEvidence::ImmutableIdIsObjectGuid,
            _ => return None,
        };
        Some(Self {
            kind,
            evidence,
            src: (
                SchemaFamily::from_u16(u(4))?,
                Guid128(b[0x10..0x20].try_into().ok()?),
            ),
            dst: (
                SchemaFamily::from_u16(u(6))?,
                Guid128(b[0x20..0x30].try_into().ok()?),
            ),
        })
    }
}
