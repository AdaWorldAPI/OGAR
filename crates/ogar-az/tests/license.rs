//! The Graph licence reads: assignment paths, effective SKUs, tenant capacity.

use ogar_az::AzError;
use ogar_az::license::{
    LicensePath, PathState, ingest_licenses_page, ingest_skus_page, licenses_url, skus_url,
};
use ogar_dir_core::Guid128;

const U1: &str = "7a3c9e11-2b44-4c6d-9e8f-0123456789ab";
const U2: &str = "7a3c9e11-2b44-4c6d-9e8f-0123456789ac";
const U3: &str = "7a3c9e11-2b44-4c6d-9e8f-0123456789ad";
const SKU: &str = "c7df2760-2c81-4ef7-b578-5b5392b571df";
const SKU2: &str = "4b9405b0-7788-4568-add1-99614e613b69";
const GROUP: &str = "3f2504e0-4f89-11d3-9a0c-0305e82c3301";
const PLAN: &str = "57ff2da0-773e-42df-b2af-ffb7a2317929";

fn g(s: &str) -> Guid128 {
    Guid128::parse(s).unwrap()
}

/// U1: SKU directly and through GROUP; U2: read, none; U3: not read.
fn page() -> String {
    format!(
        r#"{{"value":[
        {{"id":"{U1}","licenseAssignmentStates":[
            {{"skuId":"{SKU}","state":"Active","assignedByGroup":null,"error":"None","disabledPlans":[]}},
            {{"skuId":"{SKU}","state":"Active","assignedByGroup":"{GROUP}","error":"None","disabledPlans":["{PLAN}"]}}],
         "assignedLicenses":[{{"skuId":"{SKU}","disabledPlans":[]}}]}},
        {{"id":"{U2}","licenseAssignmentStates":[],"assignedLicenses":[]}},
        {{"id":"{U3}"}}],
        "@odata.nextLink":"https://graph.microsoft.com/v1.0/users?$skiptoken=x"}}"#
    )
}

#[test]
fn urls_are_read_only_and_bounded() {
    let u = licenses_url(5000);
    assert!(u.contains("$select=id,licenseAssignmentStates,assignedLicenses"));
    assert!(u.ends_with("&$top=999"));
    assert!(!u.contains(' '));
    assert!(skus_url().ends_with("/v1.0/subscribedSkus"));
}

#[test]
fn not_read_is_not_none() {
    let p = ingest_licenses_page(&page()).unwrap();
    assert_eq!(p.users.len(), 3);
    assert!(p.next_link.as_deref().unwrap().contains("$skiptoken"));
    // read, none
    assert_eq!(p.users[1].paths, Some(vec![]));
    assert_eq!(p.users[1].effective, Some(vec![]));
    assert_eq!(p.users[1].skus(), Some(vec![]));
    // not read
    assert_eq!(p.users[2].paths, None);
    assert_eq!(p.users[2].effective, None);
    assert_eq!(p.users[2].skus(), None);
    // null reads as not read too
    let null = format!(r#"{{"value":[{{"id":"{U1}","licenseAssignmentStates":null}}]}}"#);
    assert_eq!(ingest_licenses_page(&null).unwrap().users[0].paths, None);
}

#[test]
fn two_paths_are_one_sku() {
    let p = ingest_licenses_page(&page()).unwrap();
    let u1 = &p.users[0];
    assert_eq!(u1.user, g(U1));
    let paths = u1.paths.as_ref().unwrap();
    assert_eq!(paths.len(), 2);
    assert_eq!(u1.skus(), Some(vec![g(SKU)]));
    assert_eq!(u1.effective.as_ref().unwrap().len(), 1);
    assert_eq!(
        paths[0],
        LicensePath {
            sku: g(SKU),
            assigned_by_group: None,
            state: PathState::Active,
            error: Some("None".into()),
            disabled_plans: vec![],
        }
    );
    assert_eq!(paths[1].assigned_by_group, Some(g(GROUP)));
    assert_eq!(paths[1].disabled_plans, vec![g(PLAN)]);
}

#[test]
fn unknown_state_text_is_kept_not_coerced() {
    let body = format!(
        r#"{{"value":[{{"id":"{U1}","licenseAssignmentStates":[
        {{"skuId":"{SKU}","state":"ActiveWithError"}},
        {{"skuId":"{SKU2}","state":"Disabled"}},
        {{"skuId":"{SKU2}","state":"Error","error":"CountViolation"}}]}}]}}"#
    );
    let p = ingest_licenses_page(&body).unwrap();
    let paths = p.users[0].paths.as_ref().unwrap();
    assert_eq!(paths[0].state, PathState::Other("ActiveWithError".into()));
    assert_eq!(paths[1].state, PathState::Disabled);
    assert_eq!(paths[2].state, PathState::Error);
    assert_eq!(paths[2].error.as_deref(), Some("CountViolation"));
    // Silence half: plain Active stays Active.
    let active = format!(
        r#"{{"value":[{{"id":"{U1}","licenseAssignmentStates":[{{"skuId":"{SKU}","state":"Active"}}]}}]}}"#
    );
    let p = ingest_licenses_page(&active).unwrap();
    assert_eq!(
        p.users[0].paths.as_ref().unwrap()[0].state,
        PathState::Active
    );
}

#[test]
fn malformed_licence_pages_are_refused_not_dropped() {
    assert_eq!(ingest_licenses_page("{}"), Err(AzError::NotAPage));
    let bad_user = r#"{"value":[{"id":"nope"}]}"#;
    assert_eq!(ingest_licenses_page(bad_user), Err(AzError::BadId(0)));
    let bad_sku = format!(
        r#"{{"value":[{{"id":"{U1}","licenseAssignmentStates":[{{"skuId":"x","state":"Active"}}]}}]}}"#
    );
    assert_eq!(
        ingest_licenses_page(&bad_sku),
        Err(AzError::BadType(0, "skuId"))
    );
    let bad_group = format!(
        r#"{{"value":[{{"id":"{U1}","licenseAssignmentStates":[{{"skuId":"{SKU}","state":"Active","assignedByGroup":"x"}}]}}]}}"#
    );
    assert_eq!(
        ingest_licenses_page(&bad_group),
        Err(AzError::BadType(0, "assignedByGroup"))
    );
    let no_state =
        format!(r#"{{"value":[{{"id":"{U1}","licenseAssignmentStates":[{{"skuId":"{SKU}"}}]}}]}}"#);
    assert_eq!(
        ingest_licenses_page(&no_state),
        Err(AzError::BadType(0, "state"))
    );
    let not_array = format!(r#"{{"value":[{{"id":"{U1}","licenseAssignmentStates":{{}}}}]}}"#);
    assert_eq!(
        ingest_licenses_page(&not_array),
        Err(AzError::BadType(0, "licenseAssignmentStates"))
    );
}

fn skus() -> String {
    format!(
        r#"{{"value":[
        {{"skuId":"{SKU}","skuPartNumber":"SPE_E3","consumedUnits":1200,
          "prepaidUnits":{{"enabled":1000,"suspended":5,"warning":2,"lockedOut":0}},
          "servicePlans":[{{"servicePlanId":"{PLAN}","servicePlanName":"TEAMS1"}}]}},
        {{"skuId":"{SKU2}","consumedUnits":3,"prepaidUnits":{{"enabled":10,"suspended":0,"warning":0}}}}]}}"#
    )
}

#[test]
fn capacity_is_exact_and_free_may_be_negative() {
    let p = ingest_skus_page(&skus()).unwrap();
    assert_eq!(p.skus.len(), 2);
    let e3 = &p.skus[0];
    assert_eq!(e3.sku, g(SKU));
    assert_eq!(e3.part_number.as_deref(), Some("SPE_E3"));
    assert_eq!(
        (e3.enabled, e3.suspended, e3.warning, e3.consumed),
        (1000, 5, 2, 1200)
    );
    assert_eq!(e3.free(), -200);
    assert_eq!(e3.plans, vec![g(PLAN)]);
    // Silence half: an ordinary SKU has positive free units and no plans.
    let small = &p.skus[1];
    assert_eq!(small.free(), 7);
    assert_eq!(small.part_number, None);
    assert!(small.plans.is_empty());
}

#[test]
fn a_missing_count_is_an_error_not_zero() {
    let no_consumed = format!(
        r#"{{"value":[{{"skuId":"{SKU}","prepaidUnits":{{"enabled":1,"suspended":0,"warning":0}}}}]}}"#
    );
    assert_eq!(
        ingest_skus_page(&no_consumed),
        Err(AzError::BadType(0, "consumedUnits"))
    );
    let no_prepaid = format!(r#"{{"value":[{{"skuId":"{SKU}","consumedUnits":1}}]}}"#);
    assert_eq!(
        ingest_skus_page(&no_prepaid),
        Err(AzError::BadType(0, "prepaidUnits"))
    );
    let fractional = format!(
        r#"{{"value":[{{"skuId":"{SKU}","consumedUnits":1.5,"prepaidUnits":{{"enabled":1,"suspended":0,"warning":0}}}}]}}"#
    );
    assert_eq!(
        ingest_skus_page(&fractional),
        Err(AzError::BadType(0, "consumedUnits"))
    );
}
