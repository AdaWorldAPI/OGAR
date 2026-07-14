//! `ogar-a2ui-frame` — the **addressed-surface wire frames** (W1 of
//! `docs/A2UI-SCREEN-ADDRESSING-PROPOSAL.md`).
//!
//! The remote desktop stops pushing pixels and starts addressing the screen:
//!
//! - **Down** (server → client): [`NodeDelta`] — `(GUID key 16 B, wide mask
//!   delta, changed LE values)`. The client resolves `key → ClassView →
//!   template` locally (the ClassView registry is the font of the desktop)
//!   and repaints only the masked positions.
//! - **Up** (client → server): [`ActionInvoke`] — the address of an
//!   [`ActionDef`] on the Core node, never an inline handler (charter trap
//!   T2: behavior lives on the node, the surface carries only its address).
//!
//! # Wire-truth (the Firewall, ADR-022/023)
//!
//! Frames are **LE bytes end-to-end** — `to_le_bytes` / `from_le_bytes` ARE
//! the wire format; there is no serde in the hot path (the `serde` feature is
//! a membrane-only debug/interchange convenience, charter trap T3). A wasm
//! client ingests these bytes zero-copy.
//!
//! # Closed vocabulary + version gate (`ogar-doc-ir` discipline)
//!
//! [`FrameKind`] is CLOSED (a u8 on the wire; unknown = hard fail) and every
//! frame leads with [`FRAME_VERSION`] (mismatch = hard fail, never silent
//! reinterpretation — I-LEGACY-API-FEATURE-GATED applied prospectively).
//!
//! # Masks and the wide-RBAC correction (#205)
//!
//! The delta mask is carried as **explicit u64 words** (`mask_words`), so a
//! surface past 64 fields is native — no narrow-mask ceiling on the wire.
//! What the server puts INTO the mask is the projection `surface ∩ role`
//! (charter C1.4: `ClassRbac::field_mask` retyped to `WideFieldMask`;
//! zero-extension fail-closed; sentinel ban). The frame is dumb transport —
//! RBAC happens before framing, exactly once.

#![forbid(unsafe_code)]

/// Wire version — the FIRST byte of every frame. A reader that sees any
/// other value MUST refuse the frame (no silent reinterpretation).
pub const FRAME_VERSION: u8 = 1;

/// Fixed header size: version(1) + kind(1) + reserved(2) + payload_len(4).
pub const HEADER_LEN: usize = 8;

/// The CLOSED frame vocabulary — a u8 on the wire. Unknown values are
/// refused at [`Frame::from_le_bytes`], never skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    /// Server → client: one addressed node's masked field update.
    NodeDelta = 0,
    /// Client → server: invoke an `ActionDef` by address.
    ActionInvoke = 1,
}

impl FrameKind {
    /// Decode the wire byte; `None` for anything outside the closed set.
    #[must_use]
    pub const fn from_u8(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::NodeDelta),
            1 => Some(Self::ActionInvoke),
            _ => None,
        }
    }
}

/// Why a frame could not be decoded. Every arm is a refusal — there is no
/// lossy/partial acceptance path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    /// Fewer bytes than the fixed header.
    Truncated { need: usize, have: usize },
    /// First byte was not [`FRAME_VERSION`].
    WrongVersion { found: u8 },
    /// Kind byte outside the closed [`FrameKind`] vocabulary.
    UnknownKind(u8),
    /// Header's `payload_len` disagrees with the bytes provided.
    LengthMismatch { declared: usize, have: usize },
    /// Payload too short for its kind's fixed prefix.
    ShortPayload {
        kind: FrameKind,
        need: usize,
        have: usize,
    },
}

impl core::fmt::Display for FrameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Truncated { need, have } => {
                write!(f, "frame truncated: need {need} header bytes, have {have}")
            }
            Self::WrongVersion { found } => {
                write!(f, "frame version {found} != {FRAME_VERSION} — refused")
            }
            Self::UnknownKind(b) => write!(f, "frame kind {b} outside the closed vocabulary"),
            Self::LengthMismatch { declared, have } => {
                write!(
                    f,
                    "payload_len {declared} disagrees with {have} bytes present"
                )
            }
            Self::ShortPayload { kind, need, have } => {
                write!(f, "{kind:?} payload needs >= {need} bytes, have {have}")
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// Server → client: one addressed node's update.
///
/// `key` is the canonical 16-byte GUID — the client prerenders layout from it
/// with zero value decode (P0 canon). `mask_words` is the CHANGED-fields
/// delta as explicit u64 words (word `w`, bit `b` ⇒ field position
/// `w*64 + b`); wide surfaces are native. `values` are the changed fields'
/// bytes concatenated in ascending field-position order — their carving is
/// resolved by the classid's ClassView on the client, never described on the
/// wire (content-blind, like the 4+12 facet register).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeDelta {
    /// The canonical GUID of the addressed node.
    pub key: [u8; 16],
    /// Changed-field positions as u64 words (LE on the wire).
    pub mask_words: Vec<u64>,
    /// Changed fields' bytes, ascending field-position order, ClassView-carved.
    pub values: Vec<u8>,
}

/// Client → server: invoke an `ActionDef` **by address** (charter trap T2 —
/// the surface never carries behavior, only the address of behavior).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActionInvoke {
    /// The node the action targets.
    pub key: [u8; 16],
    /// The action's identity within the node's class — index into the
    /// class's ActionDef set (registry-resolved; NOT a method pointer).
    pub action_ordinal: u32,
    /// Invocation arguments, ClassView/ActionDef-schema carved (opaque here).
    pub args: Vec<u8>,
}

/// One decoded frame.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Frame {
    /// Server → client.
    NodeDelta(NodeDelta),
    /// Client → server.
    ActionInvoke(ActionInvoke),
}

impl Frame {
    /// The frame's wire kind.
    #[must_use]
    pub const fn kind(&self) -> FrameKind {
        match self {
            Self::NodeDelta(_) => FrameKind::NodeDelta,
            Self::ActionInvoke(_) => FrameKind::ActionInvoke,
        }
    }

    /// Encode to the LE wire form (header + payload). This IS the wire
    /// format — there is no other canonical serialization.
    #[must_use]
    pub fn to_le_bytes(&self) -> Vec<u8> {
        let payload = match self {
            Self::NodeDelta(d) => {
                let mut p = Vec::with_capacity(16 + 2 + d.mask_words.len() * 8 + d.values.len());
                p.extend_from_slice(&d.key);
                let n = u16::try_from(d.mask_words.len()).expect("mask words fit u16");
                p.extend_from_slice(&n.to_le_bytes());
                for w in &d.mask_words {
                    p.extend_from_slice(&w.to_le_bytes());
                }
                p.extend_from_slice(&d.values);
                p
            }
            Self::ActionInvoke(a) => {
                let mut p = Vec::with_capacity(16 + 4 + a.args.len());
                p.extend_from_slice(&a.key);
                p.extend_from_slice(&a.action_ordinal.to_le_bytes());
                p.extend_from_slice(&a.args);
                p
            }
        };
        let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
        out.push(FRAME_VERSION);
        out.push(self.kind() as u8);
        out.extend_from_slice(&[0u8; 2]); // reserved
        let len = u32::try_from(payload.len()).expect("payload fits u32");
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&payload);
        out
    }

    /// Decode one frame from LE bytes — the load gate. Refuses (never
    /// skips) wrong version, unknown kind, and length lies.
    ///
    /// # Errors
    ///
    /// See [`FrameError`] — every arm is a refusal.
    pub fn from_le_bytes(bytes: &[u8]) -> Result<Self, FrameError> {
        if bytes.len() < HEADER_LEN {
            return Err(FrameError::Truncated {
                need: HEADER_LEN,
                have: bytes.len(),
            });
        }
        if bytes[0] != FRAME_VERSION {
            return Err(FrameError::WrongVersion { found: bytes[0] });
        }
        let kind = FrameKind::from_u8(bytes[1]).ok_or(FrameError::UnknownKind(bytes[1]))?;
        let declared = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        let payload = &bytes[HEADER_LEN..];
        if payload.len() != declared {
            return Err(FrameError::LengthMismatch {
                declared,
                have: payload.len(),
            });
        }
        match kind {
            FrameKind::NodeDelta => {
                const FIXED: usize = 16 + 2;
                if payload.len() < FIXED {
                    return Err(FrameError::ShortPayload {
                        kind,
                        need: FIXED,
                        have: payload.len(),
                    });
                }
                let mut key = [0u8; 16];
                key.copy_from_slice(&payload[..16]);
                let n = u16::from_le_bytes([payload[16], payload[17]]) as usize;
                let mask_end = FIXED + n * 8;
                if payload.len() < mask_end {
                    return Err(FrameError::ShortPayload {
                        kind,
                        need: mask_end,
                        have: payload.len(),
                    });
                }
                let mask_words = payload[FIXED..mask_end]
                    .chunks_exact(8)
                    .map(|c| u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
                    .collect();
                Ok(Self::NodeDelta(NodeDelta {
                    key,
                    mask_words,
                    values: payload[mask_end..].to_vec(),
                }))
            }
            FrameKind::ActionInvoke => {
                const FIXED: usize = 16 + 4;
                if payload.len() < FIXED {
                    return Err(FrameError::ShortPayload {
                        kind,
                        need: FIXED,
                        have: payload.len(),
                    });
                }
                let mut key = [0u8; 16];
                key.copy_from_slice(&payload[..16]);
                let action_ordinal =
                    u32::from_le_bytes([payload[16], payload[17], payload[18], payload[19]]);
                Ok(Self::ActionInvoke(ActionInvoke {
                    key,
                    action_ordinal,
                    args: payload[FIXED..].to_vec(),
                }))
            }
        }
    }
}

/// Which field positions a [`NodeDelta`] mask carries — the iterator a
/// client walks to apply `values` in ascending position order.
pub fn mask_positions(mask_words: &[u64]) -> impl Iterator<Item = u32> + '_ {
    mask_words.iter().enumerate().flat_map(|(w, word)| {
        let w = w as u32;
        (0..64u32).filter_map(move |b| ((word >> b) & 1 == 1).then_some(w * 64 + b))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> [u8; 16] {
        // classid 0x0801_0003 LE + tiers — shape only, not a real mint.
        [3, 0, 1, 8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
    }

    #[test]
    fn node_delta_roundtrips_wide() {
        // A WIDE surface: 3 mask words = fields up to position 191 — the
        // #205 correction made concrete: positions >= 64 are native.
        let f = Frame::NodeDelta(NodeDelta {
            key: key(),
            mask_words: vec![0b1010, 0, 1 << 5], // positions 1, 3, 133
            values: vec![0xDE, 0xAD, 0xBE, 0xEF],
        });
        let wire = f.to_le_bytes();
        let back = Frame::from_le_bytes(&wire).expect("valid frame");
        assert_eq!(f, back);
        let pos: Vec<u32> = match &back {
            Frame::NodeDelta(d) => mask_positions(&d.mask_words).collect(),
            _ => unreachable!(),
        };
        assert_eq!(pos, vec![1, 3, 133], "wide positions decode past 64");
    }

    #[test]
    fn action_invoke_roundtrips() {
        let f = Frame::ActionInvoke(ActionInvoke {
            key: key(),
            action_ordinal: 7,
            args: vec![1, 2, 3],
        });
        assert_eq!(Frame::from_le_bytes(&f.to_le_bytes()).unwrap(), f);
    }

    #[test]
    fn wrong_version_is_refused() {
        let mut wire = Frame::ActionInvoke(ActionInvoke {
            key: key(),
            action_ordinal: 0,
            args: vec![],
        })
        .to_le_bytes();
        wire[0] = 2;
        assert!(matches!(
            Frame::from_le_bytes(&wire),
            Err(FrameError::WrongVersion { found: 2 })
        ));
    }

    #[test]
    fn unknown_kind_is_refused() {
        let mut wire = Frame::ActionInvoke(ActionInvoke {
            key: key(),
            action_ordinal: 0,
            args: vec![],
        })
        .to_le_bytes();
        wire[1] = 9;
        assert!(matches!(
            Frame::from_le_bytes(&wire),
            Err(FrameError::UnknownKind(9))
        ));
    }

    #[test]
    fn length_lie_is_refused() {
        let mut wire = Frame::ActionInvoke(ActionInvoke {
            key: key(),
            action_ordinal: 0,
            args: vec![],
        })
        .to_le_bytes();
        wire[4] = wire[4].wrapping_add(1); // declared len now lies
        assert!(matches!(
            Frame::from_le_bytes(&wire),
            Err(FrameError::LengthMismatch { .. })
        ));
    }

    #[test]
    fn short_mask_is_refused_not_partially_read() {
        // Declare 2 mask words but truncate the payload mid-mask; the
        // header length is made consistent so ONLY the inner gate fires.
        let d = Frame::NodeDelta(NodeDelta {
            key: key(),
            mask_words: vec![u64::MAX, 1],
            values: vec![],
        });
        let mut wire = d.to_le_bytes();
        wire.truncate(HEADER_LEN + 16 + 2 + 8); // one word instead of two
        let new_len = (wire.len() - HEADER_LEN) as u32;
        wire[4..8].copy_from_slice(&new_len.to_le_bytes());
        assert!(matches!(
            Frame::from_le_bytes(&wire),
            Err(FrameError::ShortPayload { .. })
        ));
    }
}
