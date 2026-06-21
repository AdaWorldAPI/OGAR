//! Port specifications — `(namespace, bridge_id, public_name → class_id)`
//! triples consumed by `lance_graph_ontology::UnifiedBridge` to project
//! per-port public name vocabularies onto the shared OGAR codebook.
//!
//! # The goal — one bridge harness, port-specific data
//!
//! Before this module landed, each port shipped a clone of the same
//! NamespaceBridge boilerplate (WoaBridge, MedcareBridge,
//! OpenProjectBridge, RedmineBridge) — same struct, same impl shape,
//! same codebook-aware `entity()` override, with a per-bridge constants
//! table baked into each file. Adding a port meant copy-pasting a
//! NamespaceBridge impl AND duplicating its alias table.
//!
//! [`PortSpec`] flips that: the bridge becomes one generic
//! [`lance_graph_ontology::UnifiedBridge<P: PortSpec>`] harness, and
//! the per-port differences (namespace, bridge_id, alias table) live
//! here as data attached to the canonical class schema. Adding a port
//! is now one `impl PortSpec for FooPort {...}` block with three
//! constants and the alias slice — no bridge boilerplate, no risk of
//! two ports' codebook tables drifting on a shared concept.
//!
//! # Apple meets apple — cross-fork convergence by data
//!
//! Both [`OpenProjectPort`] and [`RedminePort`] map their port-public
//! names to the **same** `class_ids::*` constants. So
//! `OpenProjectPort::class_id("WorkPackage") == RedminePort::class_id("Issue")`,
//! and any consumer reading
//! `bridge.entity(name).schema_ptr.entity_type_id()` gets identical
//! ids across the two ports — the cross-fork convergence the codebook
//! was calcified for, now sourced from the OGAR class schema rather
//! than re-declared per bridge.
//!
//! See [`tests`] below for the convergence pins.

use crate::class_ids;

/// Per-port specification consumed by the unified bridge.
///
/// Implementations carry zero state — they're zero-sized types that
/// parameterize the unified bridge at compile time. Three pieces of
/// data per port:
///
/// - [`Self::NAMESPACE`]: the canonical TTL namespace (matches
///   `ogit.<NS>:` prefix in the per-entity TTL files).
/// - [`Self::BRIDGE_ID`]: lowercase bridge_id for
///   `NamespaceBridge::bridge_id()` and registry dispatch.
/// - [`Self::aliases`]: slice of `(public_name, canonical_class_id)`
///   pairs. The default [`Self::class_id`] does a linear scan over
///   the slice; bypass it only when a port has so many aliases that
///   the O(n) lookup matters (none today; 32 concepts max per port).
pub trait PortSpec: 'static + Send + Sync {
    /// Canonical namespace name (e.g. `"OpenProject"`, `"Redmine"`).
    /// Matches the `ogit.<NS>:` TTL prefix and the
    /// `NamespaceRegistry::seed_defaults()` key.
    const NAMESPACE: &'static str;
    /// Lowercase bridge_id for `NamespaceBridge::bridge_id()`.
    const BRIDGE_ID: &'static str;

    /// All `(port-public-name, canonical-class-id)` aliases for this
    /// port. Order is not significant for resolution but kept stable
    /// for human readability.
    fn aliases() -> &'static [(&'static str, u16)];

    /// Map a port-public name to the canonical OGAR class_id.
    /// Returns `None` for names outside the alias table.
    fn class_id(public_name: &str) -> Option<u16> {
        Self::aliases()
            .iter()
            .find(|(name, _)| *name == public_name)
            .map(|(_, id)| *id)
    }
}

// ── OpenProject port ────────────────────────────────────────────────

/// OpenProject's `PortSpec` — maps OpenProject's Rails model names
/// (`WorkPackage`, `TimeEntry`, …) onto the shared OGAR codebook.
///
/// Sister of [`RedminePort`]. Concept-pair convergence (e.g.
/// `WorkPackage` ↔ `Issue` both → `class_ids::PROJECT_WORK_ITEM`)
/// is pinned by [`tests::openproject_and_redmine_converge_on_shared_concepts`].
pub struct OpenProjectPort;

impl PortSpec for OpenProjectPort {
    const NAMESPACE: &'static str = "OpenProject";
    const BRIDGE_ID: &'static str = "openproject";
    fn aliases() -> &'static [(&'static str, u16)] {
        OPENPROJECT_ALIASES
    }
}

const OPENPROJECT_ALIASES: &[(&str, u16)] = &[
    ("Project", class_ids::PROJECT),
    ("WorkPackage", class_ids::PROJECT_WORK_ITEM),
    ("TimeEntry", class_ids::BILLABLE_WORK_ENTRY),
    ("User", class_ids::PROJECT_ACTOR),
    ("Status", class_ids::PROJECT_STATUS),
    ("Type", class_ids::PROJECT_TYPE),
    ("Priority", class_ids::PRIORITY),
    ("Membership", class_ids::PROJECT_MEMBERSHIP),
    ("Journal", class_ids::PROJECT_JOURNAL),
    ("Repository", class_ids::PROJECT_REPOSITORY),
    ("Version", class_ids::PROJECT_VERSION),
    ("WikiPage", class_ids::PROJECT_WIKI_PAGE),
    ("Query", class_ids::PROJECT_QUERY),
    ("Attachment", class_ids::PROJECT_ATTACHMENT),
    ("CustomField", class_ids::PROJECT_CUSTOM_FIELD),
    ("Relation", class_ids::PROJECT_RELATION),
    ("Changeset", class_ids::PROJECT_CHANGESET),
    ("Watcher", class_ids::PROJECT_WATCHER),
    ("News", class_ids::PROJECT_NEWS),
    ("Message", class_ids::PROJECT_MESSAGE),
    ("Forum", class_ids::PROJECT_FORUM),
    ("Role", class_ids::PROJECT_ROLE),
    ("MemberRole", class_ids::PROJECT_MEMBER_ROLE),
    ("CustomValue", class_ids::PROJECT_CUSTOM_VALUE),
    ("EnabledModule", class_ids::PROJECT_ENABLED_MODULE),
];

// ── Redmine port ────────────────────────────────────────────────────

/// Redmine's `PortSpec` — maps Redmine's Rails model names (`Issue`,
/// `Tracker`, `IssueStatus`, …) onto the shared OGAR codebook.
///
/// Sister of [`OpenProjectPort`]. Both reference the same
/// `class_ids::*` constants for converging concepts, so
/// `OpenProjectPort::class_id("WorkPackage")` and
/// `RedminePort::class_id("Issue")` both resolve to `0x0102
/// project_work_item`.
pub struct RedminePort;

impl PortSpec for RedminePort {
    const NAMESPACE: &'static str = "Redmine";
    const BRIDGE_ID: &'static str = "redmine";
    fn aliases() -> &'static [(&'static str, u16)] {
        REDMINE_ALIASES
    }
}

const REDMINE_ALIASES: &[(&str, u16)] = &[
    ("Project", class_ids::PROJECT),
    ("Issue", class_ids::PROJECT_WORK_ITEM),
    ("TimeEntry", class_ids::BILLABLE_WORK_ENTRY),
    ("User", class_ids::PROJECT_ACTOR),
    ("IssueStatus", class_ids::PROJECT_STATUS),
    ("Tracker", class_ids::PROJECT_TYPE),
    ("Member", class_ids::PROJECT_MEMBERSHIP),
    ("Journal", class_ids::PROJECT_JOURNAL),
    ("Repository", class_ids::PROJECT_REPOSITORY),
    ("Version", class_ids::PROJECT_VERSION),
    ("WikiPage", class_ids::PROJECT_WIKI_PAGE),
    ("Query", class_ids::PROJECT_QUERY),
    ("Attachment", class_ids::PROJECT_ATTACHMENT),
    ("Comment", class_ids::PROJECT_COMMENT),
    ("CustomField", class_ids::PROJECT_CUSTOM_FIELD),
    ("IssueRelation", class_ids::PROJECT_RELATION),
    ("Changeset", class_ids::PROJECT_CHANGESET),
    ("Watcher", class_ids::PROJECT_WATCHER),
    ("News", class_ids::PROJECT_NEWS),
    ("Message", class_ids::PROJECT_MESSAGE),
    ("Board", class_ids::PROJECT_FORUM),
    ("Role", class_ids::PROJECT_ROLE),
    ("MemberRole", class_ids::PROJECT_MEMBER_ROLE),
    ("CustomValue", class_ids::PROJECT_CUSTOM_VALUE),
    ("EnabledModule", class_ids::PROJECT_ENABLED_MODULE),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openproject_namespace_and_bridge_id_match_canonical_strings() {
        assert_eq!(OpenProjectPort::NAMESPACE, "OpenProject");
        assert_eq!(OpenProjectPort::BRIDGE_ID, "openproject");
    }

    #[test]
    fn redmine_namespace_and_bridge_id_match_canonical_strings() {
        assert_eq!(RedminePort::NAMESPACE, "Redmine");
        assert_eq!(RedminePort::BRIDGE_ID, "redmine");
    }

    #[test]
    fn openproject_workpackage_maps_to_project_work_item() {
        assert_eq!(
            OpenProjectPort::class_id("WorkPackage"),
            Some(class_ids::PROJECT_WORK_ITEM)
        );
        assert_eq!(OpenProjectPort::class_id("WorkPackage"), Some(0x0102));
    }

    #[test]
    fn redmine_issue_maps_to_project_work_item() {
        assert_eq!(
            RedminePort::class_id("Issue"),
            Some(class_ids::PROJECT_WORK_ITEM)
        );
        assert_eq!(RedminePort::class_id("Issue"), Some(0x0102));
    }

    /// Headline cross-fork convergence pin: every concept pair below
    /// resolves to the SAME `class_ids::*` constant via both ports'
    /// `class_id()` resolvers. Drift here would re-introduce the codex
    /// P1 bug on PR #559 (distinct entity_type_ids for converging
    /// canonical concepts).
    #[test]
    fn openproject_and_redmine_converge_on_shared_concepts() {
        let pairs: &[(&str, &str, u16)] = &[
            ("Project", "Project", class_ids::PROJECT),
            ("WorkPackage", "Issue", class_ids::PROJECT_WORK_ITEM),
            ("TimeEntry", "TimeEntry", class_ids::BILLABLE_WORK_ENTRY),
            ("User", "User", class_ids::PROJECT_ACTOR),
            ("Status", "IssueStatus", class_ids::PROJECT_STATUS),
            ("Type", "Tracker", class_ids::PROJECT_TYPE),
            ("Membership", "Member", class_ids::PROJECT_MEMBERSHIP),
            ("Journal", "Journal", class_ids::PROJECT_JOURNAL),
            ("Repository", "Repository", class_ids::PROJECT_REPOSITORY),
            ("Version", "Version", class_ids::PROJECT_VERSION),
            ("WikiPage", "WikiPage", class_ids::PROJECT_WIKI_PAGE),
            ("Query", "Query", class_ids::PROJECT_QUERY),
            ("Attachment", "Attachment", class_ids::PROJECT_ATTACHMENT),
            ("CustomField", "CustomField", class_ids::PROJECT_CUSTOM_FIELD),
            ("Relation", "IssueRelation", class_ids::PROJECT_RELATION),
            ("Changeset", "Changeset", class_ids::PROJECT_CHANGESET),
            ("Watcher", "Watcher", class_ids::PROJECT_WATCHER),
            ("News", "News", class_ids::PROJECT_NEWS),
            ("Message", "Message", class_ids::PROJECT_MESSAGE),
            ("Forum", "Board", class_ids::PROJECT_FORUM),
            ("Role", "Role", class_ids::PROJECT_ROLE),
            ("MemberRole", "MemberRole", class_ids::PROJECT_MEMBER_ROLE),
            ("CustomValue", "CustomValue", class_ids::PROJECT_CUSTOM_VALUE),
            ("EnabledModule", "EnabledModule", class_ids::PROJECT_ENABLED_MODULE),
        ];
        for &(op_name, rm_name, expected) in pairs {
            let op = OpenProjectPort::class_id(op_name);
            let rm = RedminePort::class_id(rm_name);
            assert_eq!(
                op,
                Some(expected),
                "OpenProjectPort `{op_name}` should map to 0x{expected:04X}",
            );
            assert_eq!(
                rm,
                Some(expected),
                "RedminePort `{rm_name}` should map to 0x{expected:04X}",
            );
            assert_eq!(
                op, rm,
                "convergence broken: OpenProject `{op_name}` ↔ Redmine `{rm_name}`",
            );
        }
    }

    #[test]
    fn unknown_public_names_resolve_to_none() {
        assert_eq!(OpenProjectPort::class_id("NotAConcept"), None);
        assert_eq!(RedminePort::class_id("NotAConcept"), None);
        assert_eq!(OpenProjectPort::class_id(""), None);
        assert_eq!(RedminePort::class_id(""), None);
    }

    #[test]
    fn each_alias_class_id_is_in_the_codebook() {
        // Every class_id in the alias tables must be a real codebook
        // entry — drift between the OpenProject/Redmine port aliases
        // and `class_ids::ALL` is a P1.
        let codebook_ids: Vec<u16> = class_ids::ALL.iter().map(|(_, id)| *id).collect();
        for &(name, id) in OpenProjectPort::aliases() {
            assert!(
                codebook_ids.contains(&id),
                "OpenProjectPort alias `{name}` -> 0x{id:04X} not in class_ids::ALL"
            );
        }
        for &(name, id) in RedminePort::aliases() {
            assert!(
                codebook_ids.contains(&id),
                "RedminePort alias `{name}` -> 0x{id:04X} not in class_ids::ALL"
            );
        }
    }

    #[test]
    fn each_port_has_25_aliases() {
        // 25 of the 26 project-mgmt concepts are common across both
        // ports. OpenProject doesn't have Comment as a top-level model
        // (Journal carries comments); Redmine doesn't have Priority as
        // a top-level model (Enumeration::IssuePriority is the shape).
        // Both end up with 25 alias rows.
        assert_eq!(OpenProjectPort::aliases().len(), 25);
        assert_eq!(RedminePort::aliases().len(), 25);
    }
}
