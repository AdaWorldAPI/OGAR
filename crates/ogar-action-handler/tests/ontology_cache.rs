//! The Ontology cache over the floating AdaWorldAPI/OGIT checkout, and how
//! its names compare with the OGIT tables'.
#![cfg(feature = "ontology-cache")]

#[allow(dead_code)]
#[path = "../../ogar-from-schema/src/ogit_checkout.rs"]
mod ogit_checkout;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use lance_graph_ontology::namespace::SchemaKind;
use ogar_action_handler::ogit::{OgitTables, Table, TypeId, entity_curies, load_registry};
use ogar_from_schema::TtlDeclaration;
use ogar_from_schema::ttl::parse_all;

/// Entities OGAR's reader returned and the cache missed when this was last
/// measured, all subclasses of `ogit.Healthcare:Assessment`. A record of that
/// measurement, not a contract: the test below holds on any OGIT tree and only
/// reports how the tree compares with this list.
const ONLY_IN_THE_TABLES: [&str; 3] = [
    "ogit.Healthcare:HistologicAssessment",
    "ogit.Healthcare:MriAssessment",
    "ogit.Healthcare:SonographicAssessment",
];

/// What the cache's `ttl_parse` needs of an entity file, and OGAR's reader
/// does not: a direct `rdfs:subClassOf ogit:Entity`, and the entity's own
/// prefix declared in the file. An entity only the tables hold lacks one.
struct Entity {
    parent: Option<String>,
    prefix_declared: bool,
}

/// Every entity under an `entities/` directory of the tree, by CURIE.
fn entities(root: &Path) -> BTreeMap<String, Entity> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for path in std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()) {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let in_entities = dir
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|d| d.eq_ignore_ascii_case("entities"));
            if !in_entities || path.extension().is_none_or(|e| e != "ttl") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            for decl in parse_all(&src) {
                if let TtlDeclaration::Entity(e) = decl {
                    let prefix = e.curie.split(':').next().unwrap_or("");
                    out.insert(
                        e.curie.clone(),
                        Entity {
                            parent: e.parent,
                            prefix_declared: src.contains(&format!("@prefix {prefix}:")),
                        },
                    );
                }
            }
        }
    }
    out
}

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

/// The tables and the cache read one tree, and may name different types. The
/// difference is measured and printed (`--nocapture`); what must hold on any
/// tree is that the tables miss nothing the cache has, and that every type
/// only the tables have is one the cache's parser cannot take, for a reason
/// the cache documents. OGIT adding or moving entities does not change that.
#[test]
fn the_tables_and_the_cache_name_the_same_types_but_for_documented_reasons() {
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
    let only_cache: Vec<&String> = cache.difference(&ours).collect();
    let only_ours: Vec<&String> = ours.difference(&cache).collect();
    println!(
        "tree {:?}: {} types in the tables, {} in the cache",
        t.tree().commit(),
        ours.len(),
        cache.len()
    );
    println!("only in the cache: {only_cache:?}");
    println!("only in the tables: {only_ours:?}");
    println!("recorded as only in the tables: {ONLY_IN_THE_TABLES:?}");

    // A real reader regression (a file the reader no longer takes) shows up
    // here, and in `ogit_tables.rs` as a new rejected file.
    assert!(
        only_cache.is_empty(),
        "the cache has types OGAR's reader does not: {only_cache:?}"
    );

    // Each type only the tables have is outside what the cache's parser takes.
    let found = entities(&root);
    for curie in &only_ours {
        let e = found
            .get(curie.as_str())
            .unwrap_or_else(|| panic!("{curie} is in the tables but in no entities/ file"));
        let direct = e.parent.as_deref() == Some("ogit:Entity");
        assert!(
            !direct || !e.prefix_declared,
            "{curie} is a direct `ogit:Entity` subclass with its prefix declared, yet              the cache lacks it"
        );
    }

    // What was recorded as only in the tables, and is still in the tree, is
    // still not in the cache.
    for curie in ONLY_IN_THE_TABLES {
        if ours.contains(curie) {
            assert!(!cache.contains(curie), "{curie} is now in the cache");
        }
    }
}
