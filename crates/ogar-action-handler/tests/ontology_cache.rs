//! The Ontology cache over the floating AdaWorldAPI/OGIT checkout, and how
//! its names compare with the OGIT tables'.
#![cfg(feature = "ontology-cache")]

#[allow(dead_code)]
#[path = "../../ogar-from-schema/src/ogit_checkout.rs"]
mod ogit_checkout;

use std::collections::BTreeSet;
use std::path::Path;

use lance_graph_ontology::namespace::SchemaKind;
use ogar_action_handler::ogit::{OgitTables, Table, TypeId, entity_curies, load_registry};

/// Entities OGAR's reader returns and the cache misses, measured on
/// AdaWorldAPI/OGIT `master` at `2315167`.
const ONLY_IN_THE_TABLES: [&str; 4] = [
    "ogit.Accounting:FiscalJurisdiction",
    "ogit.PTF:Dummy",
    "ogit.PTF:HiroTopology",
    "ogit.PTF:Test",
];

#[test]
fn the_cache_holds_the_hiro_classes_and_skips_the_sgo_core() {
    let root = ogit_checkout::root();
    let cache = load_registry(&root).unwrap();
    for curie in [
        "ogit.Automation:ActionHandler",
        "ogit.Automation:KnowledgeItem",
        "ogit.Automation:MAID",
        "ogit.Automation:MARSNodeTemplate",
        "ogit.MARS:Machine",
    ] {
        let row = cache
            .row_for_uri(curie)
            .unwrap_or_else(|| panic!("{curie} is not in the ontology cache"));
        assert_eq!(row.kind, SchemaKind::Entity, "{curie}");
    }
    // lance-graph-ontology keeps `ogit.<Domain>:<Name>` only. The files are in
    // the checkout, so their absence is the loader's. If this fails, the cache
    // now holds the SGO core: update the `ogit` module docs.
    for (curie, file) in [
        ("ogit:Person", "SGO/sgo/entities/Person.ttl"),
        ("ogit:dependsOn", "SGO/sgo/verbs/dependsOn.ttl"),
        ("ogit:name", "SGO/sgo/attributes/name.ttl"),
    ] {
        assert!(root.join(file).is_file(), "{file}");
        assert!(cache.row_for_uri(curie).is_none(), "{curie}");
    }
    assert!(load_registry(Path::new("/nonexistent/OGIT")).is_err());
}

#[test]
fn the_tables_and_the_cache_name_the_same_types_but_for_known_differences() {
    let root = ogit_checkout::root();
    let cache: BTreeSet<String> = entity_curies(&load_registry(&root).unwrap())
        .into_iter()
        .collect();
    let t = OgitTables::load(&root).unwrap();
    let ours: BTreeSet<String> = (0..t.rows(Table::Type))
        .filter_map(|i| t.type_name(TypeId(u16::try_from(i).unwrap())))
        .filter(|n| n.starts_with("ogit."))
        .map(str::to_owned)
        .collect();
    // The cache also reads the files OGAR's reader rejects...
    let only_cache: Vec<&String> = cache
        .difference(&ours)
        .filter(|c| !(c.starts_with("ogit.CustomerSupport:") || c.starts_with("ogit.Mobile:")))
        .collect();
    assert!(only_cache.is_empty(), "{only_cache:?}");
    // ...and misses a few entities OGAR's reader has.
    let only_ours: Vec<&String> = ours
        .difference(&cache)
        .filter(|c| !ONLY_IN_THE_TABLES.contains(&c.as_str()))
        .collect();
    assert!(only_ours.is_empty(), "{only_ours:?}");
}
