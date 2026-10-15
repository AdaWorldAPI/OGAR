//! # stream — sealed, ordered, two-direction byte stream
//!
//! Encrypts the messages of one live session (a screen stream, a device
//! channel) under a single session key, one XChaCha20-Poly1305 seal per
//! message. It is the generic form of the sealed transport a2ui-server
//! introduced for its frame stream; the a2ui frame type stays in a2ui, and
//! this module only ever sees bytes.
//!
//! ## Nonce
//!
//! The 24-byte nonce is `[direction:1][counter:8 LE][zero:15]`. Each direction
//! keeps its own counter, starting at 0. The direction byte keeps the two
//! halves of the session from producing the same `(key, nonce)` pair.
//!
//! ## The one precondition: one key per session
//!
//! Counters restart at 0 in every session, so two sessions that share a key
//! reuse every nonce, which breaks XChaCha20-Poly1305 confidentiality and
//! integrity. Derive the key per session from a fresh random salt
//! ([`crate::kdf`]) or another source that never repeats. Nothing in this
//! module can detect a reused key.
//!
//! ## Wire form
//!
//! ```text
//! [direction:1][counter:8 LE][ciphertext ‖ 16-byte Poly1305 tag]
//! ```
//!
//! ## What [`SealedStream::open`] refuses
//!
//! - a blob shorter than the prefix plus the tag;
//! - a direction byte other than the one this half receives. Both directions
//!   use the same key, so without this check a message reflected back at its
//!   sender would open and be accepted as if the peer had sent it;
//! - a counter not strictly greater than the last accepted one (replay or
//!   reorder);
//! - a blob that fails to authenticate (wrong key, tampered bytes, or
//!   different associated data; these are indistinguishable by design).
//!
//! It is transport encryption only. It does not replace TLS, and it does not
//! protect data at rest; use [`crate::envelope`] for that.

use zeroize::Zeroize;

use crate::aead::{self, NONCE_LEN, TAG_LEN};

/// The sealed prefix: `direction(1) + counter(8)`.
const PREFIX: usize = 1 + 8;

/// Which half of the session a message belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
    /// Server to client.
    ServerToClient = 0,
    /// Client to server.
    ClientToServer = 1,
}

impl Direction {
    fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::ServerToClient),
            1 => Some(Self::ClientToServer),
            _ => None,
        }
    }
}

/// Why a message could not be sealed or opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamError {
    /// The blob is shorter than the prefix plus the tag.
    Truncated,
    /// The direction byte is not a known [`Direction`].
    BadDirection(u8),
    /// The direction byte names the other half of the session.
    WrongDirection {
        /// The direction this half receives.
        expected: Direction,
        /// The direction the blob carries.
        got: Direction,
    },
    /// The counter is not strictly greater than the last accepted one.
    Replay {
        /// The counter the blob carries.
        got: u64,
        /// The highest counter already accepted.
        last: u64,
    },
    /// The blob did not authenticate.
    Decrypt,
    /// The send counter is exhausted; the session must be re-keyed.
    CounterExhausted,
}

impl core::fmt::Display for StreamError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Truncated => f.write_str("sealed message shorter than its prefix and tag"),
            Self::BadDirection(b) => write!(f, "unknown stream direction {b}"),
            Self::WrongDirection { expected, got } => {
                write!(
                    f,
                    "message for {got:?} offered to the {expected:?} receiver"
                )
            }
            Self::Replay { got, last } => {
                write!(
                    f,
                    "replayed or reordered message: counter {got} <= last {last}"
                )
            }
            Self::Decrypt => f.write_str("sealed message failed to open"),
            Self::CounterExhausted => f.write_str("send counter exhausted; re-key the session"),
        }
    }
}

impl std::error::Error for StreamError {}

/// One half of a sealed session: it seals what it sends and opens what it
/// receives.
///
/// `send` is the direction this endpoint writes; it receives the other one.
/// A server builds `SealedStream::new(key, Direction::ServerToClient)` and a
/// client `SealedStream::new(key, Direction::ClientToServer)`, with the same
/// session key.
///
/// The key is wiped when the stream is dropped.
pub struct SealedStream {
    key: [u8; 32],
    send: Direction,
    send_counter: u64,
    last_recv: Option<u64>,
}

impl Drop for SealedStream {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl SealedStream {
    /// One endpoint of a session keyed by `key`, sending on `send`.
    #[must_use]
    pub fn new(key: [u8; 32], send: Direction) -> Self {
        Self {
            key,
            send,
            send_counter: 0,
            last_recv: None,
        }
    }

    /// The direction this endpoint receives.
    #[must_use]
    pub fn receives(&self) -> Direction {
        match self.send {
            Direction::ServerToClient => Direction::ClientToServer,
            Direction::ClientToServer => Direction::ServerToClient,
        }
    }

    fn nonce(direction: Direction, counter: u64) -> [u8; NONCE_LEN] {
        let mut n = [0u8; NONCE_LEN];
        n[0] = direction as u8;
        n[1..9].copy_from_slice(&counter.to_le_bytes());
        n
    }

    /// Seal `plaintext` as the next message on this endpoint's send
    /// direction. `aad` is authenticated but not encrypted; the receiver must
    /// pass the same bytes.
    ///
    /// # Errors
    ///
    /// [`StreamError::CounterExhausted`] after 2^64 messages, and
    /// [`StreamError::Decrypt`] if the AEAD reports an internal failure (it
    /// does not for a 32-byte key and a 24-byte nonce).
    pub fn seal(&mut self, aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, StreamError> {
        let counter = self.send_counter;
        self.send_counter = counter
            .checked_add(1)
            .ok_or(StreamError::CounterExhausted)?;
        let nonce = Self::nonce(self.send, counter);
        let ciphertext = aead::seal_with_key(&self.key, &nonce, aad, plaintext)
            .map_err(|_| StreamError::Decrypt)?;
        let mut out = Vec::with_capacity(PREFIX + ciphertext.len());
        out.push(self.send as u8);
        out.extend_from_slice(&counter.to_le_bytes());
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// Open a message sent by the other endpoint.
    ///
    /// On success the message's counter becomes the replay watermark. A
    /// refused message leaves the watermark unchanged.
    ///
    /// # Errors
    ///
    /// See [`StreamError`]; each variant is a refusal.
    pub fn open(&mut self, aad: &[u8], sealed: &[u8]) -> Result<Vec<u8>, StreamError> {
        if sealed.len() < PREFIX + TAG_LEN {
            return Err(StreamError::Truncated);
        }
        let direction =
            Direction::from_byte(sealed[0]).ok_or(StreamError::BadDirection(sealed[0]))?;
        let expected = self.receives();
        if direction != expected {
            return Err(StreamError::WrongDirection {
                expected,
                got: direction,
            });
        }
        let mut ctr = [0u8; 8];
        ctr.copy_from_slice(&sealed[1..PREFIX]);
        let counter = u64::from_le_bytes(ctr);
        if let Some(last) = self.last_recv
            && counter <= last
        {
            return Err(StreamError::Replay { got: counter, last });
        }
        let nonce = Self::nonce(direction, counter);
        let plaintext = aead::open_with_key(&self.key, &nonce, aad, &sealed[PREFIX..])
            .map_err(|_| StreamError::Decrypt)?;
        self.last_recv = Some(counter);
        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = [0x5A; 32];
    const AAD: &[u8] = b"v1";

    fn pair() -> (SealedStream, SealedStream) {
        (
            SealedStream::new(KEY, Direction::ServerToClient),
            SealedStream::new(KEY, Direction::ClientToServer),
        )
    }

    #[test]
    fn both_directions_round_trip() {
        let (mut server, mut client) = pair();
        let down = server.seal(AAD, b"node delta").unwrap();
        assert!(
            !down.windows(10).any(|w| w == b"node delta"),
            "plaintext leaked"
        );
        assert_eq!(client.open(AAD, &down).unwrap(), b"node delta");
        let up = client.seal(AAD, b"action").unwrap();
        assert_eq!(server.open(AAD, &up).unwrap(), b"action");
    }

    #[test]
    fn a_reflected_message_is_refused() {
        let (mut server, _) = pair();
        let own = server.seal(AAD, b"node delta").unwrap();
        assert_eq!(
            server.open(AAD, &own),
            Err(StreamError::WrongDirection {
                expected: Direction::ClientToServer,
                got: Direction::ServerToClient,
            })
        );
    }

    #[test]
    fn replay_and_reorder_are_refused() {
        let (mut server, mut client) = pair();
        let m0 = server.seal(AAD, b"a").unwrap();
        let m1 = server.seal(AAD, b"b").unwrap();
        assert!(client.open(AAD, &m1).is_ok());
        assert_eq!(
            client.open(AAD, &m0),
            Err(StreamError::Replay { got: 0, last: 1 })
        );
        assert_eq!(
            client.open(AAD, &m1),
            Err(StreamError::Replay { got: 1, last: 1 })
        );
    }

    #[test]
    fn wrong_key_tamper_and_aad_are_refused() {
        let (mut server, mut client) = pair();
        let m = server.seal(AAD, b"x").unwrap();
        let mut other = SealedStream::new([0xFF; 32], Direction::ClientToServer);
        assert_eq!(other.open(AAD, &m), Err(StreamError::Decrypt));
        assert_eq!(client.open(b"v2", &m), Err(StreamError::Decrypt));
        let mut bad = m.clone();
        let n = bad.len();
        bad[n - 1] ^= 1;
        assert_eq!(client.open(AAD, &bad), Err(StreamError::Decrypt));
        // The refusals did not move the watermark: the genuine message opens.
        assert_eq!(client.open(AAD, &m).unwrap(), b"x");
    }

    #[test]
    fn a_forged_counter_does_not_open() {
        let (mut server, mut client) = pair();
        let mut m = server.seal(AAD, b"x").unwrap();
        m[1] = 7; // claim counter 7; the nonce no longer matches the tag
        assert_eq!(client.open(AAD, &m), Err(StreamError::Decrypt));
    }

    #[test]
    fn short_and_unknown_direction_are_refused() {
        let (_, mut client) = pair();
        assert_eq!(client.open(AAD, &[0; PREFIX]), Err(StreamError::Truncated));
        let mut m = vec![9u8];
        m.extend_from_slice(&[0; PREFIX - 1 + TAG_LEN]);
        assert_eq!(client.open(AAD, &m), Err(StreamError::BadDirection(9)));
    }

    #[test]
    fn the_counter_never_wraps() {
        let mut server = SealedStream::new(KEY, Direction::ServerToClient);
        server.send_counter = u64::MAX;
        assert_eq!(server.seal(AAD, b"x"), Err(StreamError::CounterExhausted));
    }

    /// The wire form is the one a2ui-server's sealed transport used, so a
    /// peer built on either opens the other's messages.
    #[test]
    fn the_wire_prefix_is_direction_then_counter() {
        let mut server = SealedStream::new(KEY, Direction::ServerToClient);
        server.seal(AAD, b"a").unwrap();
        let m = server.seal(AAD, b"b").unwrap();
        assert_eq!(m[0], 0);
        assert_eq!(&m[1..9], &1u64.to_le_bytes());
        assert_eq!(m.len(), PREFIX + 1 + TAG_LEN);
    }
}
