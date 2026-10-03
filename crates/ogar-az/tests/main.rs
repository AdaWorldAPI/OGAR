use ogar_az::{SCHEMA, SCHEMA_V1, attr_str, ingest_page, select_query, sync_edges, users_url};
use ogar_dir_core::edge::{EdgeEvidence, EdgeKind};
use ogar_dir_core::{Dn, Guid128, OuDictionary, SchemaFamily, ValuePool, schema};
use std::collections::HashSet;

const TENANT: &str = "c0ffee00-1234-4abc-8def-000000000042";
const PAGE: &str = include_str!("fixtures/users_page.json");
/// The AD objectGUID that `onPremisesImmutableId` in the fixture encodes.
const AD_GUID: &str = "3f2504e0-4f89-11d3-9a0c-0305e82c3301";

fn tenant() -> Guid128 {
    Guid128::parse(TENANT).unwrap()
}

#[test]
fn schema_table_is_well_formed() {
    schema::validate(SCHEMA_V1).unwrap();
}

#[test]
fn select_is_derived_from_the_schema() {
    let q = select_query(1);
    let names: Vec<&str> = q.split(',').collect();
    assert_eq!(names[0], "id");
    assert_eq!(names.len(), 1 + SCHEMA_V1.len());
    for d in SCHEMA_V1 {
        assert!(names.contains(&d.name), "{} missing from $select", d.name);
    }
    // a version before any attribute existed selects only the identity
    assert_eq!(select_query(0), "id");
    assert!(users_url(1, 5000).ends_with("&$top=999"));
    assert!(!users_url(1, 100).contains(' '));
}

#[test]
fn graph_page_ingests() {
    let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
    let page = ingest_page(PAGE, tenant(), &mut dict, &mut pool, 99).unwrap();
    assert_eq!(page.records.len(), 2);
    assert!(page.next_link.as_deref().unwrap().contains("$skiptoken"));
    assert_eq!(page.ignored, ["someNewGraphProperty"]);

    let synced = &page.records[0];
    assert_eq!(synced.schema(), SCHEMA);
    assert_eq!(
        synced.node_guid().to_string(),
        "7a3c9e11-2b44-4c6d-9e8f-0123456789ab"
    );
    assert_eq!(synced.scope_guid(), tenant());
    assert_eq!(attr_str(synced, &pool, "displayName"), Some("Erika Müller"));
    assert_eq!(
        attr_str(synced, &pool, "employeeId"),
        None,
        "null is absence"
    );
    assert_eq!(synced.num(0), Some(1)); // accountEnabled
    assert_eq!(synced.num(1), Some(1)); // onPremisesSyncEnabled
    let ou = synced.ou_hhtl().unwrap();
    assert_eq!(
        dict.explain(&ou).unwrap(),
        ["Stuttgart", "Infrastructure", "Exchange"]
    );

    let cloud = &page.records[1];
    assert_eq!(cloud.num(0), Some(0)); // accountEnabled = false is present
    assert_eq!(cloud.num(1), None); // syncEnabled null = unknown, not false
    assert_eq!(cloud.ou_hhtl(), None);
    assert_eq!(attr_str(cloud, &pool, "onPremisesImmutableId"), None);
}

// 9. AD and AZ nodes with different GUIDs relate without identity collapse.
#[test]
fn inv09_sync_edge_relates_two_distinct_nodes() {
    let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
    let page = ingest_page(PAGE, tenant(), &mut dict, &mut pool, 99).unwrap();
    let ad = Guid128::parse(AD_GUID).unwrap();

    // not observed in AD -> no dangling edge
    assert!(sync_edges(&page.records, &pool, &HashSet::new()).is_empty());

    let edges = sync_edges(&page.records, &pool, &HashSet::from([ad]));
    assert_eq!(edges.len(), 1);
    let e = edges[0];
    assert_eq!(e.kind, EdgeKind::SynchronizesTo);
    assert_eq!(e.evidence, EdgeEvidence::ImmutableIdIsObjectGuid);
    assert_eq!(e.src, (SchemaFamily::AdDs, ad));
    assert_eq!(e.dst, (SchemaFamily::MsGraph, page.records[0].node_guid()));
    assert_ne!(e.src.1, e.dst.1, "two nodes, never merged");
    // The AZ record still carries its own id, not the AD one.
    assert_ne!(page.records[0].node_guid(), ad);
}

// With the on-prem domain's dictionary shared, the AZ evidence and the AD
// observation of one OU produce the identical HHTL.
#[test]
fn shared_onprem_dictionary_aligns_ad_and_az_hhtl() {
    let mut dict = OuDictionary::new();
    let ad_dn =
        Dn::parse("CN=Someone Else,OU=Exchange,OU=Infrastructure,OU=Stuttgart,DC=example,DC=de")
            .unwrap();
    let ad_h = dict.intern(&ad_dn.ou_path_root_first()).unwrap();
    let mut pool = ValuePool::new();
    let page = ingest_page(PAGE, tenant(), &mut dict, &mut pool, 99).unwrap();
    assert_eq!(page.records[0].ou_hhtl(), Some(ad_h));
}

#[test]
fn malformed_pages_are_refused() {
    let (mut d, mut p) = (OuDictionary::new(), ValuePool::new());
    assert!(ingest_page("{}", tenant(), &mut d, &mut p, 0).is_err());
    assert!(ingest_page(r#"{"value":[{"id":"nope"}]}"#, tenant(), &mut d, &mut p, 0).is_err());
    assert!(
        ingest_page(
            r#"{"value":[{"id":"7a3c9e11-2b44-4c6d-9e8f-0123456789ab","accountEnabled":"yes"}]}"#,
            tenant(),
            &mut d,
            &mut p,
            0
        )
        .is_err()
    );
}

// Bugbot #313: `attr_str` must not hand back the length-prefixed blob of a
// multi-valued slot as if it were a string.
#[test]
fn attr_str_refuses_multi_values_and_attr_multi_reads_them() {
    let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
    let page = ingest_page(PAGE, tenant(), &mut dict, &mut pool, 99).unwrap();
    assert_eq!(attr_str(&page.records[0], &pool, "proxyAddresses"), None);
    assert_eq!(
        ogar_az::attr_multi(&page.records[0], &pool, "proxyAddresses").unwrap(),
        [
            "SMTP:erika.mueller@example.de",
            "smtp:emueller@example.mail.onmicrosoft.com"
        ]
    );
    assert_eq!(
        ogar_az::attr_multi(&page.records[1], &pool, "proxyAddresses").unwrap(),
        Vec::<&str>::new()
    );
    assert_eq!(ogar_az::attr_multi(&page.records[0], &pool, "mail"), None);
}
