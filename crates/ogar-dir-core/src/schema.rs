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
}

impl SchemaFamily {
    /// Decode, `None` for unknown families.
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            1 => Some(Self::AdDs),
            2 => Some(Self::MsGraph),
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
}

impl AttrKind {
    /// True for pool-backed kinds (string slot space), false for numeric.
    pub fn is_pooled(self) -> bool {
        matches!(self, Self::Str | Self::Bytes | Self::MultiStr)
    }
}

/// One attribute the encoder understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttrDef {
    /// Source-native attribute name, exactly as the source spells it
    /// (`sAMAccountName`, `onPremisesDistinguishedName`).
    pub name: &'static str,
    /// Slot index within its slot space (pooled 0..32, numeric 0..4).
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
        let limit = if a.kind.is_pooled() {
            crate::record::STR_SLOTS
        } else {
            crate::record::NUM_SLOTS
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
            if b.slot == a.slot && b.kind.is_pooled() == a.kind.is_pooled() {
                return Err(format!("{} and {}: same slot {}", b.name, a.name, a.slot));
            }
        }
    }
    Ok(())
}
