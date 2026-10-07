//! The label Entra Connect writes around a cloud object id in AD's
//! `msDS-ExternalDirectoryObjectId` (`User_<guid>`, `Group_<guid>`).
//!
//! The Entra (formerly MSOL) `ObjectId`, Exchange Online's
//! `ExternalDirectoryObjectId` and AD's `msDS-ExternalDirectoryObjectId` are
//! one 128-bit id; only AD's copy carries the label. So the id is stored as a
//! [`Guid128`] and the label is stripped on the way in and added on the way
//! out — never kept as text.
//!
//! A label is a template, [`LabelPattern`]: `"User_{0}"`, where `{0}` is the
//! id. Stripping matches the text around `{0}`; rendering substitutes the id
//! into it. A new kind of object is a new template, not new code. This is the
//! one definition; the encoders and the simulation vocabulary all use it.

use crate::guid::Guid128;

/// A label template with exactly one `{0}`, the id's place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LabelPattern(pub &'static str);

impl LabelPattern {
    /// The text before and after `{0}`. `None` unless the template has
    /// exactly one `{0}`.
    fn parts(self) -> Option<(&'static str, &'static str)> {
        let (pre, post) = self.0.split_once("{0}")?;
        (!post.contains("{0}")).then_some((pre, post))
    }

    /// Strip the label: the id inside, if `s` is exactly this template
    /// around a non-nil id in its canonical spelling (lower-case hex, no
    /// braces), so that rendering it gives `s` back.
    pub fn strip(self, s: &str) -> Option<Guid128> {
        let (pre, post) = self.parts()?;
        let id = Guid128::parse(s.strip_prefix(pre)?.strip_suffix(post)?).ok()?;
        (!id.is_nil() && self.render(id) == s).then_some(id)
    }

    /// Add the label: the template with `{0}` replaced by the id.
    pub fn render(self, id: Guid128) -> String {
        self.0.replacen("{0}", &id.to_string(), 1)
    }
}

/// The labels Entra Connect writes, by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CloudLabel {
    /// `User_{0}`.
    User,
    /// `Group_{0}` (group writeback).
    Group,
}

impl CloudLabel {
    /// Every label.
    pub const ALL: [Self; 2] = [Self::User, Self::Group];

    /// Its template.
    pub const fn pattern(self) -> LabelPattern {
        match self {
            Self::User => LabelPattern("User_{0}"),
            Self::Group => LabelPattern("Group_{0}"),
        }
    }
}

/// Strip whichever label `s` carries: `User_<guid>` → `(User, guid)`.
pub fn strip(s: &str) -> Option<(CloudLabel, Guid128)> {
    CloudLabel::ALL
        .into_iter()
        .find_map(|l| l.pattern().strip(s).map(|g| (l, g)))
}

/// Add a label: `(User, guid)` → `User_<guid>`.
pub fn render(label: CloudLabel, id: Guid128) -> String {
    label.pattern().render(id)
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
    fn a_template_places_the_id_anywhere() {
        // Not tied to a prefix: any text around {0} works the same way.
        let g = Guid128::parse(ID).unwrap();
        for t in ["Device_{0}", "{0}@cloud", "x{0}y", "{0}"] {
            let p = LabelPattern(t);
            assert_eq!(p.strip(&p.render(g)), Some(g), "{t}");
        }
        assert_eq!(LabelPattern("{0}@cloud").render(g), format!("{ID}@cloud"));
        assert_eq!(
            LabelPattern("{0}@cloud").strip(&format!("{ID}@clouds")),
            None
        );
        // A template without exactly one {0} matches nothing.
        for t in ["User_", "{0}{0}"] {
            assert_eq!(LabelPattern(t).strip(&format!("User_{ID}")), None, "{t}");
        }
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
