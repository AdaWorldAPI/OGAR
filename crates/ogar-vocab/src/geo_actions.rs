//! Geo capability surface — the **OSM substrate authoritative action table**.
//!
//! Declares the capabilities the geo bake exposes over the `0x0FXX` Geo
//! concepts, as real [`ActionDef`]s, so
//! [`resolve_hotplug`](crate::capability_registry::resolve_hotplug) can
//! activate a consumer that plugs a geo classid in.
//!
//! # Why this lives HERE and not in `ogar-osm`
//!
//! It did live there, briefly, and that was the defect this module corrects.
//! `ogar-osm` shipped its own `OSM_CAPABILITIES` / `OSM_SUBJECT_CLASSIDS` /
//! `verify_osm_registration` — a **parallel registry beside
//! [`domain_tables`](crate::capability_registry::domain_tables)**, never
//! registered in it. It verified itself against itself and passed, while the
//! real port answered `NoCapabilitiesFor(0x0F01)` — *"the table was
//! forgotten"*, which is precisely the case the port exists to report.
//!
//! [`domain_tables`](crate::capability_registry::domain_tables) resolves its
//! entries at compile time from modules inside this crate, so a domain table
//! must be here to be reachable. `ogar-osm` keeps what it is actually for:
//! the **reusable patterns a consumer needs to plug OSM in** — the facet field
//! table, the identity-slot addressing, the classid composition — none of
//! which is a capability declaration.
//!
//! # The subject split
//!
//! Only `osm_node` and `osm_way` bind capabilities. The other eight Geo
//! concepts are minted and addressable but declare none, deliberately: a
//! capability with no executor arm would fail
//! [`resolve_hotplug`](crate::capability_registry::resolve_hotplug)'s coverage
//! check for every consumer, and declaring surface nobody implements is the
//! fiction the both-directions check exists to catch.
//!
//! | subject concept | capabilities |
//! |---|---|
//! | `osm_node` (`0x0F01`) | `locate_point` / `locate_tile` / `locate_fragment` / `project_fields` |
//! | `osm_way` (`0x0F02`) | `street_edges` / `polyline_length` |

use crate::{ActionDef, ActionSubject, KausalSpec};

/// Every geo capability name, in table order — the `const`-evaluable
/// fingerprint of [`geo_actions`], for a cheap exhaustiveness fuse without
/// paying for the table's allocations.
pub const GEO_ACTION_NAMES: &[&str] = &[
    "locate_point",
    "locate_tile",
    "locate_fragment",
    "project_fields",
    "street_edges",
    "polyline_length",
];

/// One geo [`ActionDef`]. `object_class` is `ogit-geo/{concept}` so
/// `derive_action_rows` recovers the concept from the last `/` segment and
/// resolves it against the codebook — the same fuse shape as
/// `ocr_actions::ocr_action_def` and `healthcare_actions::healthcare_action_def`.
fn geo_action_def(capability: &'static str, subject_concept: &'static str) -> ActionDef {
    let object_class = format!("ogit-geo/{subject_concept}");
    let identity = format!("{object_class}::action_def::{capability}");
    ActionDef {
        identity,
        predicate: capability.to_owned(),
        object_class,
        // A geo lookup is a read against a baked corpus, not a user action:
        // the caller is the substrate itself (a tile request, a route solve),
        // so the subject is the System rather than an authenticated User.
        default_subject: ActionSubject::System,
        // Invoked directly by a caller (a tile/route edge, or a same-process
        // executor call) with no OGAR-side precondition to guard on —
        // `KausalSpec::External`'s documented case.
        kausal: Some(KausalSpec::External),
        ..ActionDef::default()
    }
}

/// The geo capability surface — one [`ActionDef`] per capability, in
/// [`GEO_ACTION_NAMES`] order.
///
/// Every entry corresponds to a primitive that **exists in the consumer
/// today**. `resolve_hotplug` checks coverage in both directions, so an
/// aspirational entry here fails the consumer's own activation rather than
/// quietly describing work nobody did.
#[must_use]
pub fn geo_actions() -> Vec<ActionDef> {
    const SUBJECT_OF: &[(&str, &str)] = &[
        ("locate_point", "osm_node"),
        ("locate_tile", "osm_node"),
        ("locate_fragment", "osm_node"),
        ("project_fields", "osm_node"),
        ("street_edges", "osm_way"),
        ("polyline_length", "osm_way"),
    ];
    SUBJECT_OF
        .iter()
        .map(|&(capability, subject)| geo_action_def(capability, subject))
        .collect()
}

/// The executors the authority EXPECTS to register against this table.
pub const GEO_EXPECTED_EXECUTORS: &[&str] = &["osm-soa-bake"];

/// The distinct subject classids this table binds (canon-high concept ids).
/// A registering consumer must activate exactly this set.
pub const GEO_SUBJECT_CLASSIDS: &[u16] = &[crate::class_ids::OSM_NODE, crate::class_ids::OSM_WAY];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_registry::{HotplugDrift, entries_from_actions, resolve_hotplug};
    use crate::class_ids;

    #[test]
    fn every_subject_concept_resolves_through_the_codebook() {
        // The fuse: `derive_action_rows` splits `object_class` on '/' and
        // resolves the tail. A typo'd concept name would land in the slag
        // ledger as id 0 rather than failing loudly here, so assert the ids.
        let rows = entries_from_actions(&geo_actions());
        assert_eq!(rows.len(), GEO_ACTION_NAMES.len());
        for (capability, id) in &rows {
            assert_ne!(*id, 0, "{capability} did not resolve to a minted concept");
            assert_eq!(
                id >> 8,
                0x0F,
                "{capability} resolved outside the Geo domain"
            );
        }
    }

    #[test]
    fn the_fingerprint_matches_the_table_in_order() {
        let defs = geo_actions();
        assert_eq!(defs.len(), GEO_ACTION_NAMES.len());
        for (def, name) in defs.iter().zip(GEO_ACTION_NAMES) {
            assert_eq!(&def.predicate, name, "fingerprint drifted from the table");
        }
    }

    #[test]
    fn the_declared_subjects_are_exactly_the_subjects_the_table_uses() {
        let mut used: Vec<u16> = entries_from_actions(&geo_actions())
            .into_iter()
            .map(|(_, id)| id)
            .collect();
        used.sort_unstable();
        used.dedup();
        assert_eq!(used, GEO_SUBJECT_CLASSIDS.to_vec());
    }

    #[test]
    fn plugging_a_geo_classid_into_the_port_now_activates() {
        // The regression this module exists for. Before it, this returned
        // `Err(NoCapabilitiesFor(0x0F01))` — measured — because the geo table
        // was declared in `ogar-osm`, outside `domain_tables()`.
        let (concepts, capabilities) =
            resolve_hotplug("osm-soa-bake", GEO_SUBJECT_CLASSIDS, GEO_ACTION_NAMES)
                .expect("the geo domain must activate");
        assert_eq!(capabilities.len(), GEO_ACTION_NAMES.len());
        let names: Vec<&str> = concepts.iter().map(|&(n, _)| n).collect();
        assert!(names.contains(&"osm_node") && names.contains(&"osm_way"));
    }

    #[test]
    fn the_port_rejects_a_wrong_consumer_and_incomplete_coverage() {
        // Can-fire halves, so the activation above is not "the port says yes
        // to everything".
        assert!(matches!(
            resolve_hotplug("some-other-crate", GEO_SUBJECT_CLASSIDS, GEO_ACTION_NAMES),
            Err(HotplugDrift::UnexpectedConsumer(_))
        ));
        assert!(matches!(
            resolve_hotplug("osm-soa-bake", GEO_SUBJECT_CLASSIDS, &["locate_point"]),
            Err(HotplugDrift::Uncovered(_))
        ));
        assert!(matches!(
            resolve_hotplug(
                "osm-soa-bake",
                GEO_SUBJECT_CLASSIDS,
                &[
                    "locate_point",
                    "locate_tile",
                    "locate_fragment",
                    "project_fields",
                    "street_edges",
                    "polyline_length",
                    "teleport",
                ]
            ),
            Err(HotplugDrift::Undeclared(_))
        ));
    }

    #[test]
    fn a_geo_concept_with_no_capabilities_does_not_silently_activate() {
        // Eight Geo concepts bind nothing. Plugging one in must report the
        // forgotten table rather than activating an empty set — the exact
        // error the whole geo domain used to return.
        assert_eq!(
            resolve_hotplug("osm-soa-bake", &[class_ids::OSM_CHANGESET], &[]),
            Err(HotplugDrift::NoCapabilitiesFor(class_ids::OSM_CHANGESET))
        );
    }
}
