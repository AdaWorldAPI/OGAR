//! **Functions as objects** — a call resolves an ADDRESS, not a `Vec` index.
//!
//! # What was missing, and where it already said so
//!
//! [`LocoConcept::Inventory`](crate::LocoConcept::Inventory) — `0x1702`, minted
//! — already carries the design in its own doc:
//!
//! > the **inventory** row: the function registry entry (which functions exist,
//! > **addressed by identity**). A registry read never touches a body.
//!
//! That is functions-as-objects, and nothing implemented it.
//! [`Program`](crate::Program)'s `Vec<FunctionBody>` stood in for it, so a
//! branch target was a byte index into one program's private list. A function
//! could not be passed, stored, shared between programs, or named from
//! anywhere but the body that happened to contain it.
//!
//! A function AT REST was already an object:
//! [`FunctionNode`](crate::FunctionNode) is 512 bytes with a 16-byte key in
//! slot 0, and `node.rs` keeps that key deliberately opaque so the substrate
//! mints it. The identity existed; the runtime did not use it.
//!
//! # What an address is
//!
//! [`FnAddr`] is a `u16` — 65,536 functions per inventory, the same ceiling
//! the substrate's other `u16` index spaces use. It is an **index into an
//! inventory**, NOT a GUID: the 16-byte key is the canon's, minting is the
//! substrate's, and an interpreter that embedded a key in every branch would
//! be carrying 16 bytes where a call has one or two.
//!
//! The inventory is what maps between them ([`Inventory::key_of`]), so a
//! consumer that stores by key and executes by address needs no second table.
//!
//! # Why a trait
//!
//! Because where the bodies live is not this crate's business. A test holds
//! them in a `Vec`; a consumer resolves them out of a node store, a Lance
//! scan, or a cache. The interpreter only ever asks "give me the body at this
//! address", which is the one question every such backing can answer.

use crate::FunctionBody;

/// A function's address within one [`Inventory`].
///
/// Deliberately NOT the 16-byte key: an address is what a CALL carries, and a
/// call's whole payload is 1-3 bytes. The key is what a STORE carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct FnAddr(pub u16);

impl FnAddr {
    /// The entry address. Reserved: a body-reference byte of `0` means
    /// "unset", so no call may branch to it — the same rule
    /// `Program::references_are_resolvable` already enforces.
    pub const ENTRY: FnAddr = FnAddr(0);
}

/// Where the interpreter gets a body from.
///
/// One question, because that is all the interpreter asks. `key_of` is the
/// second half of functions-as-objects — without it an address is a private
/// index again, and a consumer could not tell two inventories apart.
pub trait Inventory {
    /// The body at `addr`, or `None` if nothing is registered there.
    fn body(&self, addr: FnAddr) -> Option<&FunctionBody>;

    /// The canonical 16-byte key for `addr`, when the backing knows one.
    ///
    /// `None` is honest rather than a zero key: a test inventory built from
    /// bare bodies has no minted identity, and answering `[0u8; 16]` would be
    /// a GUID that collides with every other unminted function.
    fn key_of(&self, _addr: FnAddr) -> Option<[u8; 16]> {
        None
    }

    /// How many addresses this inventory can answer. Used only for
    /// diagnostics; `body` returning `None` is the real bound.
    fn len(&self) -> usize;

    /// Whether the inventory holds nothing.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The obvious in-memory inventory: bodies in registration order.
///
/// Registration order IS the address, which makes a `VecInventory` built from
/// a [`Program`](crate::Program)'s own `functions` behave exactly as the old
/// index did — that equivalence is what lets the change land without
/// re-pinning a single existing program.
#[derive(Debug, Clone, Default)]
pub struct VecInventory {
    bodies: Vec<FunctionBody>,
    keys: Vec<Option<[u8; 16]>>,
}

impl VecInventory {
    /// An empty inventory.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a body, returning the address it landed at.
    pub fn push(&mut self, body: FunctionBody) -> FnAddr {
        let a = self.bodies.len();
        self.bodies.push(body);
        self.keys.push(None);
        // A `u16` address space means the 65,537th body has nowhere to live.
        // Saturating would silently alias it onto the last legal address, so
        // the cast is checked and the overflow is a panic at BUILD time, not
        // a wrong branch at run time.
        FnAddr(u16::try_from(a).expect("inventory exceeds the u16 address space"))
    }

    /// Append a body with its minted key.
    pub fn push_keyed(&mut self, key: [u8; 16], body: FunctionBody) -> FnAddr {
        let a = self.push(body);
        self.keys[a.0 as usize] = Some(key);
        a
    }

    /// The address holding `key`, if any. Linear — a consumer with many
    /// functions brings its own index; this one is for tests and small sets.
    #[must_use]
    pub fn addr_of(&self, key: &[u8; 16]) -> Option<FnAddr> {
        self.keys
            .iter()
            .position(|k| k.as_ref() == Some(key))
            .and_then(|i| u16::try_from(i).ok())
            .map(FnAddr)
    }
}

impl FromIterator<FunctionBody> for VecInventory {
    fn from_iter<I: IntoIterator<Item = FunctionBody>>(iter: I) -> Self {
        let bodies: Vec<FunctionBody> = iter.into_iter().collect();
        let keys = vec![None; bodies.len()];
        Self { bodies, keys }
    }
}

impl Inventory for VecInventory {
    fn body(&self, addr: FnAddr) -> Option<&FunctionBody> {
        self.bodies.get(addr.0 as usize)
    }

    fn key_of(&self, addr: FnAddr) -> Option<[u8; 16]> {
        self.keys.get(addr.0 as usize).copied().flatten()
    }

    fn len(&self) -> usize {
        self.bodies.len()
    }
}
