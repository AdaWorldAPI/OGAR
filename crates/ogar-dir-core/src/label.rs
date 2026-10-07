//! The object-type label Entra Connect writes in front of a cloud object id
//! in AD's `msDS-ExternalDirectoryObjectId` (`User_<guid>`, `Group_<guid>`).
//!
//! The Entra (formerly MSOL) `ObjectId`, Exchange Online's
//! `ExternalDirectoryObjectId` and AD's `msDS-ExternalDirectoryObjectId` are
//! one 128-bit id; only AD's copy carries the label. So the id is stored as a
//! [`Guid128`] and the label is stripped on the way in ([`strip`]) and added
//! on the way out ([`render`]) — never kept as text. This is the one
//! definition of the label; the encoders and the simulation vocabulary all
//! use it.

use crate::guid::Guid128;

/// The label in front of the id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CloudLabel {
    /// `User_`.
    User,
    /// `Group_` (group writeback).
    Group,
}

impl CloudLabel {
    /// Every label.
    pub const ALL: [Self; 2] = [Self::User, Self::Group];

    /// The text written in front of the id.
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::User => "User_",
            Self::Group => "Group_",
        }
    }
}

/// Strip the label: `User_<guid>` → `(User, guid)`. `None` for anything that
/// does not render back to exactly `s` (no or unknown label, label case,
/// upper-case hex, braces, the nil id).
pub fn strip(s: &str) -> Option<(CloudLabel, Guid128)> {
    let (label, rest) = CloudLabel::ALL
        .into_iter()
        .find_map(|l| s.strip_prefix(l.prefix()).map(|r| (l, r)))?;
    let id = Guid128::parse(rest).ok()?;
    (!id.is_nil() && render(label, id) == s).then_some((label, id))
}

/// Add the label: `(User, guid)` → `User_<guid>`.
pub fn render(label: CloudLabel, id: Guid128) -> String {
    format!("{}{}", label.prefix(), id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0b5c3a1e-7d2f-4c88-9e10-3f6a2b4c5d6e";

    #[test]
    fn strip_and_render_are_inverse() {
        let g = Guid128::parse(ID).unwrap();
        for label in CloudLabel::ALL {
            let text = render(label, g);
            assert_eq!(strip(&text), Some((label, g)));
        }
        assert_eq!(render(CloudLabel::User, g), format!("User_{ID}"));
    }

    #[test]
    fn anything_that_does_not_render_back_is_refused() {
        for bad in [
            ID.to_string(),
            format!("user_{ID}"),
            format!("Device_{ID}"),
            format!("User_{}", ID.to_uppercase()),
            format!("User_{{{ID}}}"),
            "User_00000000-0000-0000-0000-000000000000".to_string(),
            format!("User_{ID} "),
        ] {
            assert_eq!(strip(&bad), None, "{bad}");
        }
    }
}
