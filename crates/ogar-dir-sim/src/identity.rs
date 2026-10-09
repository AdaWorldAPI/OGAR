//! Hybrid identity: how an on-premises AD object and its Entra / Exchange
//! Online counterpart name each other.
//!
//! | on-premises (AD)                  | cloud                        |
//! |-----------------------------------|------------------------------|
//! | `mS-DS-ConsistencyGuid` (16 bytes) | `ImmutableId` = its base64  |
//! | `msDS-ExternalDirectoryObjectId`  | the cloud object id, `User_`-prefixed |
//! | `mailNickname`                    | Exchange Online `Alias`      |
//!
//! **One cloud object id, three spellings.** The Entra (formerly MSOL)
//! `ObjectId`, Exchange Online's `ExternalDirectoryObjectId` and AD's
//! `msDS-ExternalDirectoryObjectId` are the same 128-bit id. The two cloud
//! spellings are the bare GUID; only AD's backsync copy carries the `User_`
//! label. So the id is a [`Guid128`] (`ExternalObjectId::object_id`), the
//! cloud spellings parse with [`Guid128::parse`], and the `User_` text is a
//! [`CloudLabel`] rendered on egress to AD — never a stored string. All
//! three compare as ids, not as text. `ogar-ad` already strips the label on
//! ingest and stores the id in an inline guid slot; the label codec is the
//! one in [`ogar_dir_core::label`].
//!
//! The alias needs no type here: `mailNickname` is ingested raw and is the
//! Exchange Online `Alias` as-is.
//!
//! Decoding is strict, like [`crate::exchange`]: a value that does not
//! re-render to exactly what was observed is not decoded.

use ogar_dir_core::{Encoding, Guid128, base64, label};

pub use ogar_dir_core::label::CloudLabel;

/// A decoded `msDS-ExternalDirectoryObjectId`: the cloud object id, plus the
/// label its text form carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExternalObjectId {
    /// The object-type label.
    pub label: CloudLabel,
    /// The Entra object id.
    pub object_id: Guid128,
}

impl ExternalObjectId {
    /// Decode `User_<guid>` / `Group_<guid>`. `None` for anything that does
    /// not render back to exactly `s` (unknown label, upper-case hex, braces,
    /// a nil id); the caller keeps such a value raw.
    pub fn parse(s: &str) -> Option<Self> {
        let (label, object_id) = label::strip(s)?;
        Some(Self { label, object_id })
    }

    /// The text form, as Entra Connect writes it.
    pub fn render(&self) -> String {
        label::render(self.label, self.object_id)
    }
}

/// The source anchor: `mS-DS-ConsistencyGuid`, whose base64 is the cloud
/// `ImmutableId`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceAnchor(pub Guid128);

impl SourceAnchor {
    /// From the raw attribute value (16 bytes, Microsoft mixed-endian, the
    /// same order as `objectGUID`). `None` for any other length or a nil id.
    pub fn from_consistency_guid(raw: &[u8]) -> Option<Self> {
        let g = Guid128::from_ms_bytes(raw).ok()?;
        (!g.is_nil()).then_some(Self(g))
    }

    /// From a cloud `ImmutableId`. Only the padded form this crate renders
    /// is accepted, so the decode is exact.
    pub fn from_immutable_id(s: &str) -> Option<Self> {
        Encoding::Base64MsGuidBytes16
            .decode(s.as_bytes())
            .ok()
            .map(Self)
    }

    /// The raw attribute value.
    pub fn to_consistency_guid(&self) -> [u8; 16] {
        self.0.to_ms_bytes()
    }

    /// The cloud `ImmutableId`.
    pub fn immutable_id(&self) -> String {
        base64::encode(&self.to_consistency_guid())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0b5c3a1e-7d2f-4c88-9e10-3f6a2b4c5d6e";

    #[test]
    fn the_label_is_rendered_not_stored() {
        let user = ExternalObjectId::parse(&format!("User_{ID}")).unwrap();
        assert_eq!(user.label, CloudLabel::User);
        assert_eq!(user.object_id, Guid128::parse(ID).unwrap());
        assert_eq!(user.render(), format!("User_{ID}"));
        let group = ExternalObjectId::parse(&format!("Group_{ID}")).unwrap();
        // One id, two labels: the id compares equal, the objects do not.
        assert_eq!(group.object_id, user.object_id);
        assert_ne!(group, user);
    }

    #[test]
    fn the_three_spellings_are_one_id() {
        // Entra ObjectId and Exchange Online ExternalDirectoryObjectId are the
        // bare GUID; AD's msDS-ExternalDirectoryObjectId adds the label.
        let entra = Guid128::parse(ID).unwrap();
        let exo = Guid128::parse(ID).unwrap();
        let ad = ExternalObjectId::parse(&format!("User_{ID}")).unwrap();
        assert_eq!(ad.object_id, entra);
        assert_eq!(ad.object_id, exo);
        // The bare cloud spelling is not AD's: it carries no label.
        assert_eq!(ExternalObjectId::parse(ID), None);
        assert_eq!(ad.object_id.to_string(), ID);
    }

    #[test]
    fn anything_that_does_not_render_back_stays_raw() {
        for bad in [
            ID.to_string(),                        // no label
            format!("user_{ID}"),                  // label case
            format!("Device_{ID}"),                // unknown label
            format!("User_{}", ID.to_uppercase()), // hex case
            format!("User_{{{ID}}}"),              // braces
            "User_00000000-0000-0000-0000-000000000000".to_string(),
            format!("User_{ID} "),
        ] {
            assert_eq!(ExternalObjectId::parse(&bad), None, "{bad}");
        }
    }

    #[test]
    fn the_immutable_id_is_the_base64_of_the_consistency_guid() {
        // objectGUID 3f2504e0-4f89-11d3-9a0c-0305e82c3301, as LDAP returns it.
        let raw = base64::decode("4AQlP4lP0xGaDAMF6CwzAQ==").unwrap();
        let a = SourceAnchor::from_consistency_guid(&raw).unwrap();
        assert_eq!(
            a.0,
            Guid128::parse("3f2504e0-4f89-11d3-9a0c-0305e82c3301").unwrap()
        );
        assert_eq!(a.immutable_id(), "4AQlP4lP0xGaDAMF6CwzAQ==");
        assert_eq!(a.to_consistency_guid().to_vec(), raw);
        assert_eq!(SourceAnchor::from_immutable_id(&a.immutable_id()), Some(a));
        // Not 16 bytes, nil, or unpadded: not an anchor.
        assert_eq!(SourceAnchor::from_consistency_guid(&raw[..15]), None);
        assert_eq!(SourceAnchor::from_consistency_guid(&[0; 16]), None);
        assert_eq!(
            SourceAnchor::from_immutable_id("4AQlP4lP0xGaDAMF6CwzAQ"),
            None
        );
    }
}
