//! The OGIT tables over the floating AdaWorldAPI/OGIT checkout.

#[allow(dead_code)]
#[path = "../../ogar-from-schema/src/ogit_checkout.rs"]
mod ogit_checkout;

use std::fmt::Debug;
use std::sync::OnceLock;

use ogar_action_handler::ogit::{AttrId, Flag, Lane, OgitTables, Table, TypeId, VerbId};
use ogar_from_schema::TtlDeclaration;
use ogar_from_schema::ttl::parse_file;

fn tables() -> &'static OgitTables {
    static TABLES: OnceLock<OgitTables> = OnceLock::new();
    TABLES
        .get_or_init(|| OgitTables::load(&ogit_checkout::root()).expect("the OGIT checkout loads"))
}

fn id(i: usize) -> u16 {
    u16::try_from(i).expect("a u16 row")
}

/// Files OGAR's reader is known to reject; Bardioc-rs's codebook tracks the
/// same set.
const KNOWN_REJECTED: [&str; 6] = [
    "NTO/CustomerSupport/",
    "NTO/Mobile/",
    "NTO/WorkOrder/verbs/",
    "SGO/sgo/attributes/public.ttl",
    "SGO/sgo/attributes/source.ttl",
    "SGO/sgo/attributes/timeZone.ttl",
];

fn strictly_increasing<T: PartialOrd + Debug>(what: &str, rows: &[T]) {
    if let Some(w) = rows.windows(2).find(|w| w[0] >= w[1]) {
        panic!("{what} out of order: {:?} then {:?}", w[0], w[1]);
    }
}

#[test]
fn the_tables_hold_the_hiro_types_and_the_sgo_core() {
    let t = tables();
    for curie in [
        "ogit:Person",
        "ogit.Automation:ActionHandler",
        "ogit.Automation:KnowledgeItem",
        "ogit.Automation:MARSNodeTemplate",
        "ogit.MARS:Application",
        "ogit.MARS:Machine",
    ] {
        let ty = t
            .type_id(curie)
            .unwrap_or_else(|| panic!("{curie} is not a type"));
        assert_eq!(t.type_name(ty), Some(curie));
    }
    // The SGO core that lance-graph-ontology skips: a verb and an attribute
    // without a domain.
    assert!(t.verb_id("ogit:dependsOn").is_some());
    assert!(t.attribute_id("ogit:name").is_some());
    assert_eq!(t.domain_id(""), Some(0), "the SGO core sorts first");
}

#[test]
fn credit_is_read_although_its_directories_are_capitalized() {
    assert!(tables().type_id("ogit.Credit:Contract").is_some());
}

#[test]
fn the_reader_rejects_nothing_beyond_the_known_files() {
    let t = tables();
    let fresh: Vec<&String> = t
        .rejected()
        .iter()
        .filter(|f| !KNOWN_REJECTED.iter().any(|known| f.starts_with(known)))
        .collect();
    assert!(
        fresh.is_empty(),
        "OGAR's reader rejects new files: {fresh:?}"
    );
    assert!(t.duplicates().is_empty(), "{:?}", t.duplicates());
}

#[test]
fn rows_follow_the_documented_order() {
    let t = tables();
    let types: Vec<(u32, &str)> = (0..t.rows(Table::Type))
        .map(|i| {
            (
                t.lane(Lane::TypeDomain)[i],
                t.type_name(TypeId(id(i))).unwrap(),
            )
        })
        .collect();
    strictly_increasing("types", &types);
    let attributes: Vec<(u32, &str)> = (0..t.rows(Table::Attribute))
        .map(|i| {
            let name = t.attribute_name(AttrId(id(i))).unwrap();
            (t.lane(Lane::AttributeDomain)[i], name)
        })
        .collect();
    strictly_increasing("attributes", &attributes);
    let verbs: Vec<(u32, &str)> = (0..t.rows(Table::Verb))
        .map(|i| {
            (
                t.lane(Lane::VerbDomain)[i],
                t.verb_name(VerbId(id(i))).unwrap(),
            )
        })
        .collect();
    strictly_increasing("verbs", &verbs);

    let (ty, flag) = (t.lane(Lane::TypeAttrType), t.lane(Lane::TypeAttrFlag));
    assert!(
        (1..ty.len()).all(|i| (ty[i - 1], flag[i - 1]) <= (ty[i], flag[i])),
        "type_attr is not ordered by type, then flag"
    );
    let (src, verb, dst) = (
        t.lane(Lane::AllowedSrc),
        t.lane(Lane::AllowedVerb),
        t.lane(Lane::AllowedDst),
    );
    let allowed: Vec<(u32, u32, u32)> = (0..src.len()).map(|i| (src[i], verb[i], dst[i])).collect();
    strictly_increasing("allowed", &allowed);

    // Every id points at a row of its table, and every domain at a domain.
    let rows = |table| u32::try_from(t.rows(table)).unwrap();
    for (lane, table) in [
        (Lane::TypeAttrType, Table::Type),
        (Lane::TypeAttrAttr, Table::Attribute),
        (Lane::AllowedSrc, Table::Type),
        (Lane::AllowedVerb, Table::Verb),
        (Lane::AllowedDst, Table::Type),
    ] {
        assert!(t.lane(lane).iter().all(|&v| v < rows(table)), "{lane:?}");
    }
    let domains = u32::try_from(t.domains().len()).unwrap();
    for lane in [Lane::TypeDomain, Lane::AttributeDomain, Lane::VerbDomain] {
        assert!(t.lane(lane).iter().all(|&d| d < domains), "{lane:?}");
    }
}

#[test]
fn type_attr_keeps_each_types_lists_in_order() {
    let t = tables();
    let (ty, attr, flag) = (
        t.lane(Lane::TypeAttrType),
        t.lane(Lane::TypeAttrAttr),
        t.lane(Lane::TypeAttrFlag),
    );
    for (curie, file) in [
        ("ogit:Person", "SGO/sgo/entities/Person.ttl"),
        ("ogit.MARS:Machine", "NTO/MARS/entities/Machine.ttl"),
        (
            "ogit.Automation:KnowledgeItem",
            "NTO/Automation/entities/KnowledgeItem.ttl",
        ),
    ] {
        let Some(TtlDeclaration::Entity(e)) = parse_file(&ogit_checkout::read(file)) else {
            panic!("{file} does not read as an entity");
        };
        let expected: Vec<(u32, &str)> = [
            (Flag::Mandatory, &e.mandatory_attributes),
            (Flag::Optional, &e.optional_attributes),
            (Flag::Indexed, &e.indexed_attributes),
        ]
        .into_iter()
        .flat_map(|(f, list)| list.iter().map(move |a| (f as u32, a.as_str())))
        .filter(|(_, a)| t.attribute_id(a).is_some())
        .collect();
        let row = u32::from(t.type_id(curie).unwrap().0);
        let actual: Vec<(u32, &str)> = (0..ty.len())
            .filter(|&i| ty[i] == row)
            .map(|i| {
                let a = AttrId(u16::try_from(attr[i]).unwrap());
                (flag[i], t.attribute_name(a).unwrap())
            })
            .collect();
        assert_eq!(actual, expected, "{curie}");
    }
}

#[test]
fn the_mars_backbone_is_an_allowed_edge() {
    let t = tables();
    let want = (
        u32::from(t.type_id("ogit.MARS:Application").unwrap().0),
        u32::from(t.verb_id("ogit:dependsOn").unwrap().0),
        u32::from(t.type_id("ogit.MARS:Resource").unwrap().0),
    );
    let (src, verb, dst) = (
        t.lane(Lane::AllowedSrc),
        t.lane(Lane::AllowedVerb),
        t.lane(Lane::AllowedDst),
    );
    assert!((0..src.len()).any(|i| (src[i], verb[i], dst[i]) == want));
}

#[test]
fn two_loads_of_one_tree_are_identical() {
    let again = OgitTables::load(&ogit_checkout::root()).unwrap();
    assert!(&again == tables(), "two loads of one tree differ");
}
