//! OSM domain types — transcoded from `openstreetmap-website` (Rails) via the
//! ruff → OGAR pipeline. See the repo `README.md` and `harvest/osm_ir.txt`.
//!
//! [`generated`] holds the 50 rendered domain structs — associations become
//! typed edge fields (`belongs_to → Option<u64>`, `has_many → Vec<u64>`), each
//! with a `new(..)` constructor. Rendered by `ogar-render-askama`; do not edit
//! by hand. [`HARVESTED_CLASSES`] is the flat inventory.

pub mod generated;

/// The 50 classes harvested from `openstreetmap-website@173885c1` `app/models`,
/// in the order OGAR lifted them. Each maps to a rendered domain module.
pub const HARVESTED_CLASSES: [&str; 50] = [
    "Acl",
    "ApplicationRecord",
    "Changeset",
    "ChangesetComment",
    "ChangesetSubscription",
    "ChangesetTag",
    "Community",
    "DiaryComment",
    "DiaryEntry",
    "DiaryEntrySubscription",
    "Follow",
    "GeoRecord::Coord",
    "Issue",
    "IssueComment",
    "Language",
    "Message",
    "ModerationZone",
    "Node",
    "NodeTag",
    "Note",
    "NoteComment",
    "NoteSubscription",
    "Oauth2Application",
    "OldNode",
    "OldNodeTag",
    "OldRelation",
    "OldRelationMember",
    "OldRelationTag",
    "OldWay",
    "OldWayNode",
    "OldWayTag",
    "Redaction",
    "Relation",
    "RelationMember",
    "RelationTag",
    "Report",
    "SocialLink",
    "SpammyPhrase",
    "Trace",
    "Tracepoint",
    "Tracetag",
    "User",
    "UserBlock",
    "UserMute",
    "UserNotificationPreferences",
    "UserPreference",
    "UserRole",
    "Way",
    "WayNode",
    "WayTag",
];

#[cfg(test)]
mod tests {
    use super::HARVESTED_CLASSES;

    #[test]
    fn harvest_inventory_is_the_full_50() {
        assert_eq!(HARVESTED_CLASSES.len(), 50);
        // core geodata graph present
        for core in ["Node", "Way", "Relation", "Changeset", "User"] {
            assert!(HARVESTED_CLASSES.contains(&core), "missing {core}");
        }
    }
}
