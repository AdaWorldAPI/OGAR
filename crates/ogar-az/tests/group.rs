//! Graph groups and their direct members.

use ogar_az::group::{
    GROUP_SCHEMA_V1, MemberKind, group_flag, group_multi, group_select_query, group_str,
    groups_url, ingest_groups_page, ingest_members_page, members_url,
};
use ogar_az::{AzError, AzKind, attr_str};
use ogar_dir_core::{Guid128, SchemaFamily, ValuePool, schema};

const TENANT: &str = "c0ffee00-1234-4abc-8def-000000000042";
const SEC: &str = "a1a1a1a1-0000-4000-8000-000000000001";
const DL: &str = "d1d1d1d1-0000-4000-8000-000000000002";
const USER: &str = "7a3c9e11-2b44-4c6d-9e8f-0123456789ab";

fn g(s: &str) -> Guid128 {
    Guid128::parse(s).unwrap()
}

const GROUPS: &str = r#"{"@odata.context":"x","@odata.nextLink":"https://graph.microsoft.com/v1.0/groups?$skiptoken=abc",
 "value":[
  {"id":"a1a1a1a1-0000-4000-8000-000000000001","displayName":"Finance","mail":"finance@example.de",
   "mailNickname":"finance","mailEnabled":true,"securityEnabled":true,"groupTypes":[],
   "proxyAddresses":["SMTP:finance@example.de","smtp:fin@example.de"],
   "onPremisesSyncEnabled":true,"onPremisesSamAccountName":"Finance",
   "onPremisesSecurityIdentifier":"S-1-5-21-1-2-3-1104","securityIdentifier":"S-1-12-1-1","visibility":null},
  {"id":"d1d1d1d1-0000-4000-8000-000000000002","displayName":"All staff","mail":"all@example.de",
   "mailEnabled":true,"securityEnabled":false,"groupTypes":[],"onPremisesSyncEnabled":null,
   "classification":"internal"}
 ]}"#;

#[test]
fn group_schema_is_well_formed_and_selected() {
    schema::validate(GROUP_SCHEMA_V1).unwrap();
    let q = group_select_query();
    assert!(q.starts_with("id,"));
    assert_eq!(q.split(',').count(), 1 + GROUP_SCHEMA_V1.len());
    assert!(groups_url(5000).ends_with("&$top=999"));
    assert!(members_url(&g(SEC), 100).contains(&format!("/groups/{SEC}/members?$select=id")));
}

#[test]
fn a_groups_page_ingests() {
    let mut pool = ValuePool::new();
    let page = ingest_groups_page(GROUPS, g(TENANT), &mut pool, 7).unwrap();
    assert_eq!(page.records.len(), 2);
    assert!(page.next_link.unwrap().contains("$skiptoken"));
    assert_eq!(page.ignored, ["classification", "visibility"]);

    let sec = &page.records[0];
    assert_eq!(sec.schema().family, SchemaFamily::MsGraph);
    assert_eq!(sec.object_kind(), AzKind::Group as u16);
    assert_eq!(sec.node_guid(), g(SEC));
    assert_eq!(sec.scope_guid(), g(TENANT));
    assert_eq!(sec.ou_hhtl(), None, "Graph reports no OU for a group");
    assert_eq!(group_str(sec, &pool, "displayName"), Some("Finance"));
    assert_eq!(group_flag(sec, "mailEnabled"), Some(true));
    assert_eq!(group_flag(sec, "securityEnabled"), Some(true));
    assert_eq!(
        group_multi(sec, &pool, "proxyAddresses").unwrap(),
        ["SMTP:finance@example.de", "smtp:fin@example.de"]
    );
    assert_eq!(
        group_multi(sec, &pool, "groupTypes").unwrap(),
        Vec::<&str>::new()
    );
    assert_eq!(
        group_str(sec, &pool, "onPremisesSecurityIdentifier"),
        Some("S-1-5-21-1-2-3-1104")
    );

    let dl = &page.records[1];
    assert_eq!(dl.node_guid(), g(DL));
    assert_eq!(group_flag(dl, "securityEnabled"), Some(false));
    // null is unknown, never false.
    assert_eq!(group_flag(dl, "onPremisesSyncEnabled"), None);
    assert_eq!(group_multi(dl, &pool, "proxyAddresses"), None);
}

// A group record lays out its slots differently from a user record, so each
// side's readers refuse the other's records rather than misread them.
#[test]
fn user_and_group_readers_do_not_cross() {
    let mut pool = ValuePool::new();
    let page = ingest_groups_page(GROUPS, g(TENANT), &mut pool, 7).unwrap();
    let sec = &page.records[0];
    // Slot 2 of a group holds `mail`; the user table puts `givenName` there.
    assert_eq!(attr_str(sec, &pool, "givenName"), None);
    assert_eq!(attr_str(sec, &pool, "displayName"), None);
    assert_eq!(group_str(sec, &pool, "mail"), Some("finance@example.de"));

    let users = include_str!("fixtures/users_page.json");
    let (mut d, mut p) = (ogar_dir_core::OuDictionary::new(), ValuePool::new());
    let user = &ogar_az::ingest_page(users, g(TENANT), &mut d, &mut p, 0)
        .unwrap()
        .records[0];
    assert!(attr_str(user, &p, "displayName").is_some());
    assert_eq!(group_str(user, &p, "displayName"), None);
    assert_eq!(group_flag(user, "mailEnabled"), None);
}

#[test]
fn members_keep_users_and_groups_and_count_the_rest() {
    let body = format!(
        r##"{{"@odata.nextLink":"next","value":[
        {{"@odata.type":"#microsoft.graph.user","id":"{USER}"}},
        {{"@odata.type":"#microsoft.graph.group","id":"{DL}"}},
        {{"@odata.type":"#microsoft.graph.device","id":"0e0e0e0e-0000-4000-8000-000000000003"}},
        {{"@odata.type":"#microsoft.graph.orgContact","id":"0c0c0c0c-0000-4000-8000-000000000004"}}]}}"##
    );
    let page = ingest_members_page(&body).unwrap();
    assert_eq!(
        page.members,
        [(g(USER), MemberKind::User), (g(DL), MemberKind::Group)]
    );
    assert_eq!(page.skipped, 2);
    assert_eq!(page.next_link.as_deref(), Some("next"));
}

#[test]
fn a_member_without_a_type_or_id_is_refused() {
    let untyped = format!(r#"{{"value":[{{"id":"{USER}"}}]}}"#);
    assert_eq!(
        ingest_members_page(&untyped),
        Err(AzError::BadType(0, "@odata.type"))
    );
    let bad = r##"{"value":[{"@odata.type":"#microsoft.graph.user","id":"nope"}]}"##;
    assert_eq!(ingest_members_page(bad), Err(AzError::BadId(0)));
    assert_eq!(ingest_members_page("{}"), Err(AzError::NotAPage));
}

#[test]
fn a_wrongly_typed_group_property_is_refused() {
    let mut pool = ValuePool::new();
    let bad = format!(r#"{{"value":[{{"id":"{SEC}","securityEnabled":"yes"}}]}}"#);
    assert!(matches!(
        ingest_groups_page(&bad, g(TENANT), &mut pool, 0),
        Err(AzError::BadType(0, "securityEnabled"))
    ));
}
