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

/// The OpenProject port's `(public_name, class_id)` alias slice,
/// exposed for downstream `pub const` re-exports
/// (e.g. `lance_graph_ontology::bridges::OPENPROJECT_CODEBOOK` keeps
/// its pre-migration shape by aliasing this slice). Prefer
/// [`OpenProjectPort::aliases`] in new code — going through the
/// `PortSpec` impl works generically across ports.
pub const OPENPROJECT_ALIASES: &[(&str, u16)] = &[
    ("Project", class_ids::PROJECT),
    ("WorkPackage", class_ids::PROJECT_WORK_ITEM),
    ("TimeEntry", class_ids::BILLABLE_WORK_ENTRY),
    // STI chain → project_actor (codex P2 on PR #87). Both OpenProject
    // and Redmine ship `User`, `Principal`, and `Group` as three classes
    // backed by one table; the codebook intentionally collapses them
    // ("Principal + User + Group STI chain collapsed" — see
    // `class_ids::PROJECT_ACTOR` doc). All three aliases route to the
    // SAME canonical id so dispatching on `entity_type_id()` reaches the
    // same arm regardless of which name the consumer hands in.
    ("User", class_ids::PROJECT_ACTOR),
    ("Principal", class_ids::PROJECT_ACTOR),
    ("Group", class_ids::PROJECT_ACTOR),
    ("Status", class_ids::PROJECT_STATUS),
    ("Type", class_ids::PROJECT_TYPE),
    // Both ports expose the priority class as `IssuePriority` (Rails
    // STI on Enumeration); codex P2 on PR #87 caught the original
    // `Priority` entry was the canonical-concept *name*, not the Rails
    // class name. Match the actual class so `class_id("IssuePriority")`
    // resolves.
    ("IssuePriority", class_ids::PRIORITY),
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

/// The Redmine port's `(public_name, class_id)` alias slice — the
/// `pub` counterpart of [`OPENPROJECT_ALIASES`] for symmetry. See its
/// doc for rationale.
pub const REDMINE_ALIASES: &[(&str, u16)] = &[
    ("Project", class_ids::PROJECT),
    ("Issue", class_ids::PROJECT_WORK_ITEM),
    ("TimeEntry", class_ids::BILLABLE_WORK_ENTRY),
    // STI chain → project_actor (codex P2 on PR #87). Same fold as
    // OpenProject's aliases — see OPENPROJECT_ALIASES doc.
    ("User", class_ids::PROJECT_ACTOR),
    ("Principal", class_ids::PROJECT_ACTOR),
    ("Group", class_ids::PROJECT_ACTOR),
    ("IssueStatus", class_ids::PROJECT_STATUS),
    ("Tracker", class_ids::PROJECT_TYPE),
    // IssuePriority maps to the shared `priority` codebook arm
    // (codex P2 on PR #87 — Redmine's port previously had no
    // priority entry at all).
    ("IssuePriority", class_ids::PRIORITY),
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

// ── Healthcare (medcare-rs) port ────────────────────────────────────

/// MedCare-rs's `PortSpec` — maps the `Healthcare` namespace's OGIT
/// entity names (`Patient`, `Diagnosis`, `LabValue`, `Medication`,
/// `Treatment`, `Visit`, `VitalSign`) onto the canonical OGAR Health
/// codebook (`0x09XX`).
///
/// Unlike [`OpenProjectPort`] / [`RedminePort`] — which converge two
/// project-management forks on a shared codebook — Healthcare is a
/// single-tenant namespace today, so there is no cross-port convergence
/// pin yet. The port exists so `lance_graph_ontology`'s `MedcareBridge`
/// collapses to `UnifiedBridge<HealthcarePort>`: the namespace,
/// bridge_id, and alias table are now **inherited from this canonical
/// class schema** instead of being re-declared per bridge in lance-graph.
/// Northstar T9 (Healthcare codebook promotion).
///
/// When a second clinical curator lands (FMA / SNOMED / RadLex import,
/// `lance-graph-rdf-fma-snomed-v1`), its port maps onto these same
/// `class_ids::*` constants — at which point Healthcare gains the same
/// apple-meets-apple convergence the project-management ports have.
pub struct HealthcarePort;

impl PortSpec for HealthcarePort {
    const NAMESPACE: &'static str = "Healthcare";
    const BRIDGE_ID: &'static str = "medcare";
    fn aliases() -> &'static [(&'static str, u16)] {
        HEALTHCARE_ALIASES
    }
}

/// The Healthcare port's `(public_name, class_id)` alias slice — the OGIT
/// `NTO/Healthcare/entities/` entity names projected onto `class_ids::*`.
/// `pub` for symmetry with [`OPENPROJECT_ALIASES`] / [`REDMINE_ALIASES`].
pub const HEALTHCARE_ALIASES: &[(&str, u16)] = &[
    ("Patient", class_ids::PATIENT),
    ("Diagnosis", class_ids::DIAGNOSIS),
    ("LabValue", class_ids::LAB_VALUE),
    ("Medication", class_ids::MEDICATION),
    ("Treatment", class_ids::TREATMENT),
    ("Visit", class_ids::VISIT),
    ("VitalSign", class_ids::VITAL_SIGN),
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
    fn healthcare_namespace_and_bridge_id_match_canonical_strings() {
        assert_eq!(HealthcarePort::NAMESPACE, "Healthcare");
        assert_eq!(HealthcarePort::BRIDGE_ID, "medcare");
    }

    #[test]
    fn healthcare_entities_resolve_into_the_health_domain() {
        use crate::{canonical_concept_domain, ConceptDomain};
        for &(name, _) in HealthcarePort::aliases() {
            let id = HealthcarePort::class_id(name)
                .unwrap_or_else(|| panic!("`{name}` must resolve"));
            assert_eq!(
                canonical_concept_domain(id),
                ConceptDomain::Health,
                "`{name}` -> 0x{id:04X} must live in the Health (0x09XX) domain",
            );
        }
        assert_eq!(HealthcarePort::class_id("Patient"), Some(class_ids::PATIENT));
        assert_eq!(HealthcarePort::class_id("Patient"), Some(0x0901));
    }

    #[test]
    fn healthcare_alias_count_matches_ogit_entities() {
        // Patient / Diagnosis / LabValue / Medication / Treatment / Visit
        // / VitalSign — the 7 OGIT `NTO/Healthcare/entities/` classes.
        assert_eq!(
            HealthcarePort::aliases().len(),
            7,
            "Healthcare alias count drift — re-count against OGIT entities"
        );
    }

    #[test]
    fn healthcare_unknown_public_names_resolve_to_none() {
        assert_eq!(HealthcarePort::class_id("WorkPackage"), None);
        assert_eq!(HealthcarePort::class_id(""), None);
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
            // STI fold (PROJECT_ACTOR): User / Principal / Group all
            // share the same id in both ports.
            ("User", "User", class_ids::PROJECT_ACTOR),
            ("Principal", "Principal", class_ids::PROJECT_ACTOR),
            ("Group", "Group", class_ids::PROJECT_ACTOR),
            ("Status", "IssueStatus", class_ids::PROJECT_STATUS),
            ("Type", "Tracker", class_ids::PROJECT_TYPE),
            ("IssuePriority", "IssuePriority", class_ids::PRIORITY),
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
        for &(name, id) in HealthcarePort::aliases() {
            assert!(
                codebook_ids.contains(&id),
                "HealthcarePort alias `{name}` -> 0x{id:04X} not in class_ids::ALL"
            );
        }
    }

    #[test]
    fn each_port_has_expected_alias_count() {
        // Asymmetric by design — each port carries exactly the Rails
        // classes its corpus ships, no phantom aliases for concepts
        // the port doesn't expose as a top-level model.
        //
        // OpenProject (27): 25 distinct concept entries + 2 STI-fold
        //   rows (Principal, Group fold into PROJECT_ACTOR alongside
        //   User). No `Comment` entry — OpenProject's Journal carries
        //   the comment-equivalent state, no standalone Comment model.
        // Redmine (28): 26 distinct concept entries + 2 STI-fold rows.
        //   Has a standalone `Comment` model on top of `Journal` (the
        //   one extra row vs OpenProject).
        //
        // Both gained the same +2 STI-fold rows and +0/+1 IssuePriority
        // entry under codex P2 on PR #87 (Redmine previously had no
        // priority entry; OpenProject's was misnamed `Priority`).
        assert_eq!(
            OpenProjectPort::aliases().len(),
            27,
            "OpenProject alias count drift — re-count the table"
        );
        assert_eq!(
            RedminePort::aliases().len(),
            28,
            "Redmine alias count drift — re-count the table"
        );
    }

    #[test]
    fn sti_actor_fold_routes_user_principal_group_to_project_actor() {
        // Codex P2 on PR #87: User/Principal/Group all map to the same
        // codebook id in both ports.
        for name in ["User", "Principal", "Group"] {
            assert_eq!(
                OpenProjectPort::class_id(name),
                Some(class_ids::PROJECT_ACTOR),
                "OpenProjectPort `{name}` should fold into PROJECT_ACTOR",
            );
            assert_eq!(
                RedminePort::class_id(name),
                Some(class_ids::PROJECT_ACTOR),
                "RedminePort `{name}` should fold into PROJECT_ACTOR",
            );
        }
    }

    #[test]
    fn issue_priority_resolves_via_actual_rails_class_name() {
        // Codex P2 on PR #87: both ports expose IssuePriority (not
        // the canonical-concept name `Priority`) as the Rails class.
        assert_eq!(
            OpenProjectPort::class_id("IssuePriority"),
            Some(class_ids::PRIORITY)
        );
        assert_eq!(
            RedminePort::class_id("IssuePriority"),
            Some(class_ids::PRIORITY)
        );
    }
}
