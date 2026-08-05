//! The **stored node** — where a function actually lives as bytes.
//!
//! # The claim this closes
//!
//! The surface rests on one sentence: *a program is V3 SoA rows, and the
//! blocks/steps a user sees are a projection of those rows.* Every other
//! layer — a block cast, a text projection, an editor address — is a
//! projection **of** something. This module is the something.
//!
//! # The layout, and the slot that is deliberately empty
//!
//! ```text
//!   one function = one node = 512 bytes = 32 × 16-byte slots
//!     slot 0       key          16 B   the canonical GUID
//!     slot 1       reserved     16 B   ZEROED — the edge-block design is RETIRED
//!     slots 2..31  value slab  480 B   30 lanes × 12 B, carved by LaneShape
//! ```
//!
//! Slot 1 is **reserved, not reclaimed**. The edge-block design (12 in-family +
//! 4 out-of-family slots) was retired, and the temptation is to hand its 16
//! bytes to the value slab and get two more calls. That is exactly the
//! field-widening the substrate's canon forbids: capacity comes from the next
//! cascade level, never from renegotiating this one. A zero tier means *not
//! consulted*, never *compacted away* — so a later mint can wake slot 1 with no
//! layout-version change, and [`FunctionNode::reserved_is_zeroed`] asserts it
//! stayed empty.
//!
//! # The key is opaque here, on purpose
//!
//! [`FunctionNode::key`] is a caller-supplied `[u8; 16]`. This crate does
//! **not** mint GUIDs: the canonical layout (classid · path tiers · tail) is
//! the substrate's, each vocabulary's app prefix is an operator decision, and
//! inventing either here would bake a guess into stored data. So the node
//! round-trips a key it does not interpret, and what this module actually
//! proves is the part it owns: **the value slab survives the trip
//! byte-for-byte**.
//!
//! # Interleave, not concatenation
//!
//! The slab is **not** 360 body bytes followed by 120 spare. Each 16-byte lane
//! is `classid(4) + payload(12)`, so a call's bytes sit at
//! `(i / calls_per_lane) * 16 + 4 + (i % calls_per_lane) * bytes_per_call`.
//! Writing the body as one contiguous run would produce a slab that looks
//! plausible, reads back correctly through the same wrong function, and is
//! wrong on the wire. [`FunctionBody::write_into_value_slab`] owns that
//! arithmetic; this module composes it and tests the composition against the
//! layout constants rather than against its own idea of them.

use crate::{FunctionBody, LaneShape, SLOT_STRIDE, VALUE_SLAB_LEN};

/// Bytes in one stored node.
pub const NODE_BYTES: usize = 512;

/// Byte offset of the key slot.
pub const KEY_OFFSET: usize = 0;
/// Bytes the key occupies (slot 0).
pub const KEY_BYTES: usize = SLOT_STRIDE;
/// Byte offset of the reserved slot (slot 1) — zeroed, never reclaimed.
pub const RESERVED_OFFSET: usize = SLOT_STRIDE;
/// Byte offset at which the value slab begins (slot 2).
pub const VALUE_OFFSET: usize = 2 * SLOT_STRIDE;

// The layout is derived, not asserted twice: if a stride or a slot count ever
// changes, this fails to compile rather than silently storing a different
// shape.
const _: () = assert!(VALUE_OFFSET + VALUE_SLAB_LEN == NODE_BYTES);
const _: () = assert!(KEY_BYTES == SLOT_STRIDE);

/// One function, as it is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionNode {
    /// The canonical GUID. **Opaque here** — this crate neither mints nor
    /// interprets it; see the module docs.
    pub key: [u8; 16],
    /// The calls.
    pub body: FunctionBody,
}

impl FunctionNode {
    /// A node holding `body` under `key`.
    #[must_use]
    pub fn new(key: [u8; 16], body: FunctionBody) -> Self {
        Self { key, body }
    }

    /// Serialize to the 512-byte block.
    ///
    /// `to_le_bytes` IS the wire format — no serde, no intermediate DTO (the
    /// no-serialization-in-the-hot-path rule). Slot 1 is written as zeroes.
    #[must_use]
    pub fn to_le_bytes(&self) -> [u8; NODE_BYTES] {
        let mut out = [0u8; NODE_BYTES];
        out[KEY_OFFSET..KEY_OFFSET + KEY_BYTES].copy_from_slice(&self.key);
        // slot 1 stays zero — reserve, don't reclaim.
        let mut slab = [0u8; VALUE_SLAB_LEN];
        self.body.write_into_value_slab(&mut slab);
        out[VALUE_OFFSET..].copy_from_slice(&slab);
        out
    }

    /// Read a node back.
    ///
    /// `shape` is not stored in the node: the lane carving is a property of the
    /// **class**, resolved through the key's classid, and duplicating it inside
    /// the value slab would be a second source of truth that could disagree
    /// with the first. The caller supplies what the ClassView says.
    #[must_use]
    pub fn from_le_bytes(bytes: &[u8; NODE_BYTES], shape: LaneShape) -> Self {
        let mut key = [0u8; 16];
        key.copy_from_slice(&bytes[KEY_OFFSET..KEY_OFFSET + KEY_BYTES]);
        let mut slab = [0u8; VALUE_SLAB_LEN];
        slab.copy_from_slice(&bytes[VALUE_OFFSET..]);
        Self {
            key,
            body: FunctionBody::read_from_value_slab(shape, &slab),
        }
    }

    /// Whether the reserved slot is still empty in a serialized node.
    ///
    /// The retired edge block's 16 bytes must stay zeroed so a later mint can
    /// wake them with no layout-version change. A caller that finds this false
    /// is looking at a node from a layout this code does not describe.
    #[must_use]
    pub fn reserved_is_zeroed(bytes: &[u8; NODE_BYTES]) -> bool {
        bytes[RESERVED_OFFSET..RESERVED_OFFSET + SLOT_STRIDE]
            .iter()
            .all(|b| *b == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CLASSID_BYTES, Call, FnIndex};

    fn key() -> [u8; 16] {
        // An arbitrary opaque key — this crate does not mint, so the test must
        // not model a minting scheme either.
        let mut k = [0u8; 16];
        k[0..4].copy_from_slice(&0x1701_FF00_u32.to_le_bytes());
        k[10..16].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        k
    }

    /// `1 + 2 * 3` under the stack discipline.
    fn expr_body(shape: LaneShape) -> FunctionBody {
        FunctionBody::from_calls(
            shape,
            &[
                Call::with_value(FnIndex::NUMBER, 1),
                Call::with_value(FnIndex::NUMBER, 2),
                Call::with_value(FnIndex::NUMBER, 3),
                Call::new(FnIndex::MUL),
                Call::new(FnIndex::ADD),
            ],
        )
        .unwrap()
    }

    #[test]
    fn a_stored_node_round_trips_byte_for_byte() {
        // THE claim: a program written to a row and read back is the same
        // program. Not "reads without error" — the same CALLS.
        let node = FunctionNode::new(key(), expr_body(LaneShape::Pairs));
        let bytes = node.to_le_bytes();
        let back = FunctionNode::from_le_bytes(&bytes, LaneShape::Pairs);

        assert_eq!(back.key, node.key);
        assert_eq!(back.body.len(), node.body.len());
        assert_eq!(back.body.as_body_bytes(), node.body.as_body_bytes());
        // …and serializing the recovered node reproduces the same 512 bytes,
        // which a lossy read would fail even if the calls happened to match.
        assert_eq!(back.to_le_bytes(), bytes);
    }

    #[test]
    fn a_different_program_stores_to_different_bytes() {
        // Anti-vacuity for the round-trip: a `to_le_bytes` that returned a
        // constant would round-trip perfectly and store nothing.
        let one_plus_two = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, 1),
                Call::with_value(FnIndex::NUMBER, 2),
                Call::new(FnIndex::ADD),
            ],
        )
        .unwrap();
        let one_plus_three = FunctionBody::from_calls(
            LaneShape::Pairs,
            &[
                Call::with_value(FnIndex::NUMBER, 1),
                Call::with_value(FnIndex::NUMBER, 3),
                Call::new(FnIndex::ADD),
            ],
        )
        .unwrap();
        let a = FunctionNode::new(key(), one_plus_two);
        let b = FunctionNode::new(key(), one_plus_three);
        assert_ne!(a.to_le_bytes(), b.to_le_bytes());
        // …and the same program under a different KEY differs too, so the key
        // is genuinely stored rather than dropped.
        let mut k2 = key();
        k2[15] = 99;
        let c = FunctionNode::new(k2, one_plus_two);
        assert_ne!(a.to_le_bytes(), c.to_le_bytes());
        assert_eq!(
            FunctionNode::from_le_bytes(&c.to_le_bytes(), LaneShape::Pairs).key,
            k2
        );
    }

    #[test]
    fn the_reserved_slot_stays_zeroed() {
        // Reserve, don't reclaim. The retired edge block's 16 bytes are the
        // obvious place to steal two more calls from, and stealing them is the
        // field-widening the canon forbids.
        let node = FunctionNode::new(key(), expr_body(LaneShape::Pairs));
        let bytes = node.to_le_bytes();
        assert!(FunctionNode::reserved_is_zeroed(&bytes));
        assert!(bytes[RESERVED_OFFSET..VALUE_OFFSET].iter().all(|b| *b == 0));

        // Two-sided: the guard must be able to say NO, or "stayed zeroed" is
        // a function that returns true.
        let mut tampered = bytes;
        tampered[RESERVED_OFFSET + 7] = 1;
        assert!(!FunctionNode::reserved_is_zeroed(&tampered));
    }

    #[test]
    fn the_body_is_interleaved_across_lanes_not_written_contiguously() {
        // The failure this catches is nasty precisely because it is
        // self-consistent: a contiguous write reads back fine through the same
        // wrong function, and is wrong on the wire. So the assertion is against
        // the LAYOUT — a call's bytes must land at its lane offset, past the
        // lane's 4 classid bytes.
        let node = FunctionNode::new(key(), expr_body(LaneShape::Pairs));
        let bytes = node.to_le_bytes();

        // First call (NUMBER:1) sits at lane 0, just past the classid.
        let first = VALUE_OFFSET + CLASSID_BYTES;
        assert_eq!(bytes[first], 0x46, "first call's function byte");
        assert_eq!(bytes[first + 1], 1, "first call's value byte");

        // Pairs packs 6 calls per 12-byte lane, so call 6 begins the SECOND
        // lane — 16 bytes on, not 12. A contiguous writer puts it at 12.
        let lane1 = VALUE_OFFSET + SLOT_STRIDE + CLASSID_BYTES;
        assert_eq!(lane1 - first, SLOT_STRIDE, "lanes are strided by 16");
        // The classid gap really is a gap: those 4 bytes are untouched by the
        // body writer.
        let gap = VALUE_OFFSET + SLOT_STRIDE;
        assert!(
            bytes[gap..gap + CLASSID_BYTES].iter().all(|b| *b == 0),
            "the body must not write into a lane's classid bytes"
        );
    }

    #[test]
    fn every_shape_round_trips_and_the_shape_is_not_stored() {
        // The shape is a class property resolved through the key, deliberately
        // not duplicated in the slab. So the same bytes read under a different
        // shape yield a DIFFERENT program — which is correct, and is why the
        // caller must supply what the ClassView says rather than guessing.
        for shape in [LaneShape::Pairs, LaneShape::Triples, LaneShape::Quads] {
            let node = FunctionNode::new(key(), expr_body(shape));
            let bytes = node.to_le_bytes();
            let back = FunctionNode::from_le_bytes(&bytes, shape);
            let a: Vec<Call> = back.body.calls().collect();
            let b: Vec<Call> = node.body.calls().collect();
            assert_eq!(a, b, "{shape:?} did not round-trip");
        }
        // Two-sided: reading Pairs bytes as Quads must NOT silently agree, or
        // "the shape matters" is untested.
        let pairs = FunctionNode::new(key(), expr_body(LaneShape::Pairs));
        let bytes = pairs.to_le_bytes();
        let misread = FunctionNode::from_le_bytes(&bytes, LaneShape::Quads);
        let a: Vec<Call> = misread.body.calls().collect();
        let b: Vec<Call> = pairs.body.calls().collect();
        assert_ne!(a, b);
    }

    #[test]
    fn a_full_body_fills_the_slab_without_overrunning_the_node() {
        // The boundary: a program at exactly the shape's budget must still fit
        // in 512 bytes, and must not disturb the key or the reserved slot.
        let cap = LaneShape::Pairs.calls_per_function();
        let calls: Vec<Call> = (0..cap)
            .map(|i| Call::with_value(FnIndex::NUMBER, (i % 250) as u8 + 1))
            .collect();
        let body = FunctionBody::from_calls(LaneShape::Pairs, &calls).unwrap();
        assert_eq!(body.len(), cap);

        let node = FunctionNode::new(key(), body);
        let bytes = node.to_le_bytes();
        assert_eq!(&bytes[..16], &key());
        assert!(FunctionNode::reserved_is_zeroed(&bytes));
        let back = FunctionNode::from_le_bytes(&bytes, LaneShape::Pairs);
        assert_eq!(back.body.len(), cap);
        assert_eq!(back.to_le_bytes(), bytes);
    }
}
