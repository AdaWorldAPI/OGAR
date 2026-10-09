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
//! they are not defined until a slice actually emits them. The evidence code
//! says which witness asserted an edge; an edge is a witness, never a
//! decision, so two witnesses that disagree both stay.
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
    ///
    /// Meaning unchanged since code 1 was assigned: only the `objectGUID`
    /// anchor profile asserts it. A `mS-DS-ConsistencyGuid` match is
    /// [`Self::SourceAnchorMatchesImmutableId`].
    ImmutableIdIsObjectGuid = 1,
    /// Graph `onPremisesImmutableId` decodes to the AD object's
    /// `mS-DS-ConsistencyGuid` (the configured source anchor), which may
    /// differ from its `objectGUID`.
    SourceAnchorMatchesImmutableId = 2,
    /// AD `msDS-ExternalDirectoryObjectId` (label stripped) equals the Entra
    /// object id: the backsync witness.
    AdBacksyncMatchesEntraObjectId = 3,
    /// Exchange Online `ExternalDirectoryObjectId` equals the Entra object
    /// id. Direction: Entra → Exchange Online.
    EntraObjectIdMatchesExchangeExternalId = 4,
}

impl EdgeEvidence {
    /// Decode, `None` for unknown codes.
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            1 => Some(Self::ImmutableIdIsObjectGuid),
            2 => Some(Self::SourceAnchorMatchesImmutableId),
            3 => Some(Self::AdBacksyncMatchesEntraObjectId),
            4 => Some(Self::EntraObjectIdMatchesExchangeExternalId),
            _ => None,
        }
    }
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
        let evidence = EdgeEvidence::from_u16(u(2))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_evidence_code_round_trips_and_code_1_keeps_its_value() {
        assert_eq!(EdgeEvidence::ImmutableIdIsObjectGuid as u16, 1);
        for ev in [
            EdgeEvidence::ImmutableIdIsObjectGuid,
            EdgeEvidence::SourceAnchorMatchesImmutableId,
            EdgeEvidence::AdBacksyncMatchesEntraObjectId,
            EdgeEvidence::EntraObjectIdMatchesExchangeExternalId,
        ] {
            let e = DirEdge {
                kind: EdgeKind::SynchronizesTo,
                evidence: ev,
                src: (SchemaFamily::MsGraph, Guid128([1; 16])),
                dst: (SchemaFamily::ExchangeOnline, Guid128([2; 16])),
            };
            assert_eq!(DirEdge::from_bytes(&e.to_bytes()), Some(e));
        }
        let mut b = [0u8; EDGE_BYTES];
        b[0] = 1;
        b[2] = 5; // unknown evidence
        b[4] = 1;
        b[6] = 2;
        assert_eq!(DirEdge::from_bytes(&b), None);
    }
}
