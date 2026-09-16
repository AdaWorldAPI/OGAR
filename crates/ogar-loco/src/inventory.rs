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

/// How many addresses a [`VecInventory`] can name: 65 536, the `u16` space.
///
/// ONE spelling, read by both [`VecInventory::push`] and the `FromIterator`
/// impl, so the two cannot enforce different bounds.
pub const MAX_ADDRESSES: usize = u16::MAX as usize + 1;

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
    /// Panics past [`MAX_ADDRESSES`] — and does so BEFORE mutating.
    ///
    /// A `u16` address space means the 65 537th body has nowhere to live.
    /// Saturating would silently alias it onto the last legal address, so the
    /// conversion is checked and the overflow is a panic at BUILD time rather
    /// than a wrong branch at run time.
    ///
    /// ⊘ The order here is the fix, not the check. The first version pushed to
    /// both vectors and converted afterwards, which review caught on
    /// OGAR #304: a caller that catches the panic is then holding an inventory
    /// with 65 537 entries whose last one no `FnAddr` can name, and `len()`
    /// counts it. Converting FIRST makes the failure leave nothing behind —
    /// the conversion IS the check, so there is no second rule to keep in
    /// step with it.
    pub fn push(&mut self, body: FunctionBody) -> FnAddr {
        let Ok(addr) = u16::try_from(self.bodies.len()) else {
            panic!(
                "inventory exceeds the u16 address space: {} bodies, max {MAX_ADDRESSES}",
                self.bodies.len()
            );
        };
        self.bodies.push(body);
        self.keys.push(None);
        FnAddr(addr)
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
    /// Panics on more than [`MAX_ADDRESSES`] bodies, exactly as [`push`] does.
    ///
    /// ⊘ The first version collected straight into the `Vec` with no check,
    /// which review flagged: it bypassed the bound `push` enforces, so a
    /// 65 537-body iterator produced an inventory whose tail no `FnAddr` can
    /// name while `len()` still counted it. A silently unaddressable entry is
    /// worse than a panic — the bound is the address space, not a policy.
    ///
    /// [`push`]: VecInventory::push
    fn from_iter<I: IntoIterator<Item = FunctionBody>>(iter: I) -> Self {
        let bodies: Vec<FunctionBody> = iter.into_iter().collect();
        assert!(
            bodies.len() <= MAX_ADDRESSES,
            "inventory exceeds the u16 address space: {} bodies, max {MAX_ADDRESSES}",
            bodies.len()
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Call, FnIndex, LaneShape};

    fn body(v: u8) -> FunctionBody {
        FunctionBody::from_calls(LaneShape::Pairs, &[Call::with_value(FnIndex::NUMBER, v)])
            .expect("a one-call body is well formed")
    }

    /// FAILS IF: an address stops being registration order, or `key_of`
    /// invents a key for an unkeyed entry. The `None` is the load-bearing
    /// half — answering `[0u8; 16]` would be a GUID colliding with every
    /// other unminted function.
    #[test]
    fn an_address_is_registration_order_and_an_unkeyed_entry_has_no_key() {
        let mut inv = VecInventory::new();
        let a = inv.push(body(1));
        let b = inv.push(body(2));
        assert_eq!((a, b), (FnAddr(0), FnAddr(1)));
        assert_eq!(inv.len(), 2);
        assert_eq!(inv.body(a), Some(&body(1)));
        assert_eq!(inv.body(b), Some(&body(2)));
        assert_eq!(inv.key_of(a), None, "an unkeyed entry must not invent one");
        assert_eq!(
            inv.body(FnAddr(2)),
            None,
            "past the end is None, not a wrap"
        );
    }

    /// FAILS IF: `push_keyed` does not bind the key to the address it
    /// returned, or `addr_of` matches a key it was never given. Two entries,
    /// so a "return the only key" implementation cannot pass.
    #[test]
    fn a_minted_key_round_trips_to_its_own_address() {
        let mut inv = VecInventory::new();
        let k1 = [1u8; 16];
        let k2 = [2u8; 16];
        let a = inv.push_keyed(k1, body(1));
        let b = inv.push_keyed(k2, body(2));
        assert_ne!(a, b);
        assert_eq!(inv.key_of(a), Some(k1));
        assert_eq!(inv.key_of(b), Some(k2));
        assert_eq!(inv.addr_of(&k1), Some(a));
        assert_eq!(inv.addr_of(&k2), Some(b));
        assert_eq!(
            inv.addr_of(&[9u8; 16]),
            None,
            "an absent key has no address"
        );
    }

    /// FAILS IF: `FromIterator` collects without the bound `push` enforces.
    ///
    /// Review flagged exactly this on OGAR #304: the 65 537th body has no
    /// `FnAddr` that can name it, so a silent collect produces an inventory
    /// whose tail is unreachable while `len()` still counts it. The bound is
    /// the address space, not a policy, so it panics rather than truncates.
    #[test]
    #[should_panic(expected = "exceeds the u16 address space")]
    fn from_iter_refuses_more_bodies_than_the_address_space_can_name() {
        let _: VecInventory = std::iter::repeat_n(body(1), MAX_ADDRESSES + 1).collect();
    }

    /// FAILS IF: `push` mutates before it validates. A caught panic must leave
    /// the inventory EXACTLY as it was — `len()` unchanged and every address
    /// still resolvable — because a caller that recovers is otherwise holding
    /// an entry no `FnAddr` can name.
    ///
    /// The `len()` check is the load-bearing assertion: the old order pushed
    /// to both vectors first, so this would read `MAX_ADDRESSES + 1`.
    #[test]
    fn a_refused_push_leaves_the_inventory_untouched() {
        let mut inv: VecInventory = std::iter::repeat_n(body(1), MAX_ADDRESSES).collect();
        assert_eq!(inv.len(), MAX_ADDRESSES, "full to the last address");

        let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            inv.push(body(2));
        }));
        assert!(refused.is_err(), "the 65 537th push must be refused");
        assert_eq!(
            inv.len(),
            MAX_ADDRESSES,
            "a refused push must not have grown the inventory"
        );
        assert!(
            inv.body(FnAddr(u16::MAX)).is_some(),
            "the last legal address still resolves"
        );
    }

    /// The paired silent half: EXACTLY the address space is legal, and every
    /// one of its addresses resolves. Without this the test above would pass
    /// for an implementation that rejects any non-trivial iterator.
    #[test]
    fn from_iter_accepts_exactly_the_address_space() {
        let inv: VecInventory = std::iter::repeat_n(body(1), MAX_ADDRESSES).collect();
        assert_eq!(inv.len(), MAX_ADDRESSES);
        assert!(inv.body(FnAddr(0)).is_some(), "the first address resolves");
        assert!(
            inv.body(FnAddr(u16::MAX)).is_some(),
            "the LAST address resolves — this is the one an off-by-one loses"
        );
    }
}
