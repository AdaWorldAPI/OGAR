//! The Graph mailbox reads: which `GET`, which permission, and what is kept.

use ogar_az::mailbox::{
    MAILBOX_SCHEMA_V1, MailboxBodies, Permission, Pull, encode_mailbox, mailbox_guid,
    mailbox_guid_of, primary_mailbox_id, user_purpose,
};
use ogar_az::{AzError, AzKind, select_query};
use ogar_dir_core::{Guid128, ValuePool, schema};

const TENANT: &str = "c0ffee00-1234-4abc-8def-000000000042";
const USER: &str = "3f2504e0-4f89-11d3-9a0c-0305e82c3302";
const MAILBOX: &str = "8a6e2f11-0b3c-4d5e-9f70-112233445566";
const SETTINGS: &str = include_str!("fixtures/mailbox_settings.json");

fn g(s: &str) -> Guid128 {
    Guid128::parse(s).unwrap()
}

fn exchange_settings(id: &str) -> String {
    format!(r#"{{"@odata.context":"x","primaryMailboxId":"{id}"}}"#)
}

#[test]
fn mailbox_schema_is_well_formed_and_not_selected_with_users() {
    schema::validate(MAILBOX_SCHEMA_V1).unwrap();
    // Graph returns these from their own endpoints, never in a users page.
    let q = select_query(ogar_az::SCHEMA_VERSION);
    for d in MAILBOX_SCHEMA_V1 {
        assert!(!q.split(',').any(|n| n == d.name), "{}", d.name);
    }
}

// Every pull is a read, each with the least-privileged application
// permission Microsoft documents for it.
#[test]
fn each_pull_names_its_get_and_its_permission() {
    let user = g(USER);
    let table = [
        (
            Pull::MailboxSettings { user },
            format!("https://graph.microsoft.com/v1.0/users/{USER}/mailboxSettings"),
            "MailboxSettings.Read",
        ),
        (
            Pull::ExchangeSettings { user },
            format!("https://graph.microsoft.com/v1.0/users/{USER}/settings/exchange"),
            "User.Read.All",
        ),
        (
            Pull::MailboxFolders {
                mailbox: "MBX:a@b",
            },
            "https://graph.microsoft.com/v1.0/admin/exchange/mailboxes/MBX%3Aa%40b/folders"
                .to_string(),
            "MailboxFolder.Read.All",
        ),
        (
            Pull::MailboxItems {
                mailbox: "MBX:a@b",
                folder: "AAMk/AG=",
            },
            "https://graph.microsoft.com/v1.0/admin/exchange/mailboxes/MBX%3Aa%40b/folders/AAMk%2FAG%3D/items"
                .to_string(),
            "MailboxItem.Read.All",
        ),
    ];
    for (pull, url, perm) in table {
        assert_eq!(pull.url(), url);
        assert_eq!(pull.permission().name(), perm);
    }
    assert_ne!(
        Permission::MailboxFolderReadAll.name(),
        Permission::MailboxItemReadAll.name()
    );
}

// The mailbox GUID is decoded only from the full `MBX:{guid}@{tenant}`
// spelling, and only for the record's own tenant.
#[test]
fn the_mailbox_guid_is_decoded_strictly() {
    let tenant = g(TENANT);
    let good = format!("MBX:{MAILBOX}@{TENANT}");
    assert_eq!(mailbox_guid_of(&good, tenant), Some(g(MAILBOX)));
    assert_eq!(
        mailbox_guid_of(
            &format!("MBX:{}@{}", MAILBOX.to_uppercase(), TENANT.to_uppercase()),
            tenant
        ),
        Some(g(MAILBOX))
    );
    let other_tenant = "c0ffee00-1234-4abc-8def-000000000043";
    for bad in [
        // Microsoft's shortened documentation example.
        "MBX:e0643f21@a7809c93".to_string(),
        format!("MBX:{MAILBOX}@{other_tenant}"),
        format!("MBX:{{{MAILBOX}}}@{TENANT}"),
        format!("{MAILBOX}@{TENANT}"),
        format!("mbx:{MAILBOX}@{TENANT}"),
        format!("MBX:{MAILBOX}"),
        String::new(),
    ] {
        assert_eq!(mailbox_guid_of(&bad, tenant), None, "{bad}");
    }
}

#[test]
fn both_reads_land_in_one_mailbox_record() {
    let mut pool = ValuePool::new();
    let ex = exchange_settings(&format!("MBX:{MAILBOX}@{TENANT}"));
    let rec = encode_mailbox(
        &MailboxBodies {
            user: g(USER),
            mailbox_settings: Some(SETTINGS),
            exchange_settings: Some(&ex),
        },
        g(TENANT),
        &mut pool,
        7,
    )
    .unwrap();
    assert_eq!(rec.node_guid(), g(USER));
    assert_eq!(rec.scope_guid(), g(TENANT));
    assert_eq!(rec.object_kind(), AzKind::Mailbox as u16);
    assert_eq!(user_purpose(&rec, &pool), Some("shared"));
    let raw = format!("MBX:{MAILBOX}@{TENANT}");
    assert_eq!(primary_mailbox_id(&rec, &pool), Some(raw.as_str()));
    assert_eq!(mailbox_guid(&rec), Some(g(MAILBOX)));
}

// A read not made leaves its attributes absent; a primaryMailboxId whose
// spelling does not decode is kept raw, without a GUID.
#[test]
fn missing_or_undecodable_reads_stay_absent() {
    let mut pool = ValuePool::new();
    let none = encode_mailbox(
        &MailboxBodies {
            user: g(USER),
            mailbox_settings: None,
            exchange_settings: None,
        },
        g(TENANT),
        &mut pool,
        0,
    )
    .unwrap();
    assert_eq!(user_purpose(&none, &pool), None);
    assert_eq!(primary_mailbox_id(&none, &pool), None);
    assert_eq!(mailbox_guid(&none), None);

    let foreign = exchange_settings(&format!(
        "MBX:{MAILBOX}@c0ffee00-1234-4abc-8def-000000000043"
    ));
    let rec = encode_mailbox(
        &MailboxBodies {
            user: g(USER),
            mailbox_settings: Some(r#"{"userPurpose":null}"#),
            exchange_settings: Some(&foreign),
        },
        g(TENANT),
        &mut pool,
        0,
    )
    .unwrap();
    assert_eq!(user_purpose(&rec, &pool), None);
    assert!(primary_mailbox_id(&rec, &pool).is_some());
    assert_eq!(mailbox_guid(&rec), None);
}

#[test]
fn malformed_bodies_are_refused() {
    let mut pool = ValuePool::new();
    let enc = |settings: &str, pool: &mut ValuePool| {
        encode_mailbox(
            &MailboxBodies {
                user: g(USER),
                mailbox_settings: Some(settings),
                exchange_settings: None,
            },
            g(TENANT),
            pool,
            0,
        )
    };
    assert_eq!(enc("not json", &mut pool), Err(AzError::NotAPage));
    assert_eq!(enc("[]", &mut pool), Err(AzError::NotAPage));
    assert_eq!(
        enc(r#"{"userPurpose":3}"#, &mut pool),
        Err(AzError::BadType(0, "userPurpose"))
    );
}

// The accessors read mailbox records only: a user record's slots mean
// something else.
#[test]
fn accessors_ignore_user_records() {
    let mut pool = ValuePool::new();
    let page = include_str!("fixtures/users_page.json");
    let mut dict = ogar_dir_core::OuDictionary::new();
    let users = ogar_az::ingest_page(page, g(TENANT), &mut dict, &mut pool, 0).unwrap();
    let user = &users.records[0];
    assert!(ogar_az::attr_str(user, &pool, "userPrincipalName").is_some());
    assert_eq!(user_purpose(user, &pool), None);
    assert_eq!(primary_mailbox_id(user, &pool), None);
    assert_eq!(mailbox_guid(user), None);
}

// A user's OneDrive: a delegated read, and the drive counts only when the
// user owns it.
#[test]
fn the_user_drive_is_a_delegated_read_owned_by_the_user() {
    use ogar_az::mailbox::{Grant, UserDrive, user_drive};
    let user = g(USER);
    let pull = Pull::UserDrive { user };
    assert_eq!(
        pull.url(),
        format!("https://graph.microsoft.com/v1.0/users/{USER}/drive")
    );
    assert_eq!(pull.permission().name(), "Files.Read.All");
    assert_eq!(pull.grant(), Grant::Delegated);
    assert_eq!(Pull::MailboxSettings { user }.grant(), Grant::Application);

    let body = |owner: &str| {
        format!(
            r#"{{"id":"b!t18F8ybsHUq1","driveType":"business","owner":{{"user":{{"id":"{owner}","displayName":"x"}}}}}}"#
        )
    };
    assert_eq!(
        user_drive(&body(USER), user).unwrap(),
        Some(UserDrive {
            id: "b!t18F8ybsHUq1".into(),
            drive_type: "business".into()
        })
    );
    assert_eq!(user_drive(&body(TENANT), user).unwrap(), None);
    assert_eq!(user_drive(r#"{"id":"b!x"}"#, user).unwrap(), None);
    assert_eq!(user_drive("[]", user), Err(AzError::NotAPage));
}
