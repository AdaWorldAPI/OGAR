//! Schema identity, kept apart from ABI identity.
//!
//! * **ABI version** ([`crate::record::ABI_MAJOR`]/`ABI_MINOR`): how the 512
//!   bytes are laid out. Changes only when a byte offset changes meaning.
//! * **Schema family** ([`SchemaFamily`]): which source system's semantics
//!   (`AD_DS` vs `MS_GRAPH`). Never flattened into one generic identity schema.
//! * **Schema version**: which attribute set the *encoder* understood.
//!
//! Schema evolution is append-only within a family: a new attribute takes the
//! next free slot and a higher `since` version; existing slots never change
//! meaning. A vN record therefore reads correctly under vN+1 (the new slots are
//! simply absent), and adding an attribute never touches the ABI. A slot whose
//! meaning must change is retired, not reused.
//!
//! A `SchemaGuid` was considered. OGAR has no schema-GUID convention today and
//! the (family, version) pair is enough to discriminate, so a GUID would be an
//! unanchored new identifier; it can be added later in reserved bytes.

/// Source schema family. `u16` LE on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum SchemaFamily {
    /// Active Directory Domain Services (LDAP).
    AdDs = 1,
    /// Microsoft Graph / Entra ID.
    MsGraph = 2,
    /// Exchange Online (a recipient as `Get-Recipient` reports it).
    ExchangeOnline = 3,
}

impl SchemaFamily {
    /// Decode, `None` for unknown families.
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            1 => Some(Self::AdDs),
            2 => Some(Self::MsGraph),
            3 => Some(Self::ExchangeOnline),
            _ => None,
        }
    }
}

/// Family + encoder schema version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SchemaId {
    /// Which source semantics.
    pub family: SchemaFamily,
    /// Which attribute set the encoder understood.
    pub version: u16,
}

/// How an attribute's value is stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttrKind {
    /// UTF-8 string, pool slot.
    Str,
    /// Raw binary, pool slot.
    Bytes,
    /// Multi-valued UTF-8, pool slot.
    MultiStr,
    /// Unsigned 32-bit, numeric slot.
    U32,
    /// Boolean (0/1), numeric slot. Absence (presence bit clear) = unknown/null.
    Bool,
    /// A positional bag of [`BAG_LEN`] string members, one pooled
    /// multi-value slot: entry `n` is member `n + 1`, an empty entry is an
    /// absent member (LDAP has no empty values; Graph reports `null`). The
    /// encoder gathers the members (AD `extensionAttribute1`..`15`, Graph
    /// `onPremisesExtensionAttributes`), so no member name is stored.
    Bag,
    /// A 128-bit id, inline guid slot (textual byte order). The encoder
    /// converts the source spelling (raw mixed-endian bytes, a labeled
    /// `User_<guid>`) on the way in; nothing about it is stored as text.
    Guid,
}

/// Members of an [`AttrKind::Bag`]: the 15 extension attributes.
pub const BAG_LEN: usize = 15;

/// The member number (1..=[`BAG_LEN`]) `name` names in the bag `bag`
/// (`extensionAttribute7` in `extensionAttribute` is 7), or `None`.
///
/// Only the canonical spelling counts: the bag name in any ASCII case,
/// then the number without sign, space or leading zero. Any other spelling
/// (`extensionAttribute01`) is not a member, so an encoder reports it as
/// ignored instead of dropping it. A name whose bag-length prefix ends
/// inside a multi-byte character is not a member either.
pub fn bag_member_of(bag: &str, name: &str) -> Option<usize> {
    let (head, n) = name.split_at_checked(bag.len())?;
    if !head.eq_ignore_ascii_case(bag)
        || n.starts_with('0')
        || n.is_empty()
        || !n.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let n: usize = n.parse().ok()?;
    (1..=BAG_LEN).contains(&n).then_some(n)
}

/// Which slot space an attribute occupies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotSpace {
    /// The 32 pooled slots.
    Pooled,
    /// The 4 numeric slots.
    Numeric,
    /// The 4 inline 128-bit id slots.
    Guid,
}

impl AttrKind {
    /// True for pool-backed kinds (string slot space), false for numeric.
    pub fn is_pooled(self) -> bool {
        self.space() == SlotSpace::Pooled
    }
    /// The slot space this kind occupies.
    pub fn space(self) -> SlotSpace {
        match self {
            Self::Str | Self::Bytes | Self::MultiStr | Self::Bag => SlotSpace::Pooled,
            Self::U32 | Self::Bool => SlotSpace::Numeric,
            Self::Guid => SlotSpace::Guid,
        }
    }
}

/// One attribute the encoder understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttrDef {
    /// Source-native attribute name, exactly as the source spells it
    /// (`sAMAccountName`, `onPremisesDistinguishedName`).
    pub name: &'static str,
    /// Slot index within its slot space (pooled 0..32, numeric 0..4, guid 0..4).
    pub slot: u8,
    /// Storage kind.
    pub kind: AttrKind,
    /// First schema version that defines this attribute.
    pub since: u16,
}

/// Structural check every schema table must pass: slots in range, no slot or
/// name used twice, `since` ≥ 1. Returns the first problem found.
pub fn validate(attrs: &[AttrDef]) -> Result<(), String> {
    for (i, a) in attrs.iter().enumerate() {
        let limit = match a.kind.space() {
            SlotSpace::Pooled => crate::record::STR_SLOTS,
            SlotSpace::Numeric => crate::record::NUM_SLOTS,
            SlotSpace::Guid => crate::record::GUID_SLOTS,
        };
        if a.slot as usize >= limit {
            return Err(format!("{}: slot {} out of range", a.name, a.slot));
        }
        if a.since == 0 {
            return Err(format!("{}: since must be >= 1", a.name));
        }
        for b in &attrs[..i] {
            if b.name == a.name {
                return Err(format!("{}: duplicate name", a.name));
            }
            if b.slot == a.slot && b.kind.space() == a.kind.space() {
                return Err(format!("{} and {}: same slot {}", b.name, a.name, a.slot));
            }
        }
    }
    Ok(())
}
