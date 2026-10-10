use ogar_ad::{AdEntry, AdKind, SCHEMA, SCHEMA_V1, encode, ldif};
use ogar_dir_core::record::{FLAG_NON_OU_CONTAINER, RECORD_BYTES};
use ogar_dir_core::{DirRecord, Guid128, OuDictionary, ValuePool, schema};

const DOMAIN: &str = "d0d0d0d0-0000-4000-8000-000000000001";

fn domain() -> Guid128 {
    Guid128::parse(DOMAIN).unwrap()
}

fn slot(name: &str) -> usize {
    SCHEMA_V1.iter().find(|d| d.name == name).unwrap().slot as usize
}

#[test]
fn schema_table_is_well_formed() {
    schema::validate(SCHEMA_V1).unwrap();
}

#[test]
fn ldif_lab_export_encodes() {
    let text = include_str!("fixtures/lab.ldif");
    let entries = ldif::parse(text).unwrap();
    assert_eq!(entries.len(), 2);
    let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());

    let user = encode(&entries[0], domain(), &mut dict, &mut pool, 1).unwrap();
    let r = user.record;
    assert_eq!(r.schema(), SCHEMA);
    assert_eq!(r.object_kind(), AdKind::User as u16);
    // objectGUID arrived as raw MS bytes and lands in textual order.
    assert_eq!(
        r.node_guid().to_string(),
        "3f2504e0-4f89-11d3-9a0c-0305e82c3301"
    );
    assert_eq!(r.scope_guid(), domain());
    let ou = r.ou_hhtl().unwrap();
    assert_eq!(
        dict.explain(&ou).unwrap(),
        ["Stuttgart", "Infrastructure", "Exchange"]
    );
    assert_eq!(
        pool.get(r.str_ref(slot("displayName")).unwrap()).unwrap(),
        "Erika Müller".as_bytes()
    );
    assert_eq!(
        pool.get_multi(r.str_ref(slot("proxyAddresses")).unwrap())
            .unwrap(),
        vec![
            &b"SMTP:erika.mueller@example.de"[..],
            b"smtp:emueller@example.mail.onmicrosoft.com"
        ]
    );
    assert_eq!(r.num(0), Some(512));
    // the DN line lands raw in the distinguishedName slot
    assert!(
        std::str::from_utf8(
            pool.get(r.str_ref(slot("distinguishedName")).unwrap())
                .unwrap()
        )
        .unwrap()
        .starts_with("CN=Erika Müller,OU=Exchange")
    );

    let comp = encode(&entries[1], domain(), &mut dict, &mut pool, 1).unwrap();
    assert_eq!(comp.record.object_kind(), AdKind::Computer as u16);
    // CN=Users is a container, not an OU: depth 0, but flagged as such
    assert_eq!(comp.record.ou_hhtl().unwrap().depth(), 0);
    assert_ne!(comp.record.flags() & FLAG_NON_OU_CONTAINER, 0);
    assert_eq!(comp.ignored, ["description"]);
}

fn entry(guid_ms_b64: &str, dn: &str) -> AdEntry {
    AdEntry {
        dn: dn.into(),
        attrs: vec![
            (
                "objectGUID".into(),
                ogar_dir_core::base64::decode(guid_ms_b64).unwrap(),
            ),
            ("objectClass".into(), b"user".to_vec()),
            ("sAMAccountName".into(), b"emueller".to_vec()),
        ],
    }
}

// 6. Moving the object between OUs changes HHTL, preserves NodeGuid.
#[test]
fn inv06_ou_move_changes_hhtl_not_identity() {
    let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
    let g = "4AQlP4lP0xGaDAMF6CwzAQ==";
    let before = encode(
        &entry(g, "CN=E M,OU=Exchange,OU=Stuttgart,DC=example,DC=de"),
        domain(),
        &mut dict,
        &mut pool,
        1,
    )
    .unwrap()
    .record;
    let after = encode(
        &entry(g, "CN=E M,OU=Finance,OU=Berlin,DC=example,DC=de"),
        domain(),
        &mut dict,
        &mut pool,
        2,
    )
    .unwrap()
    .record;
    assert_eq!(before.node_guid(), after.node_guid());
    assert_ne!(before.ou_hhtl(), after.ou_hhtl());
}

// 7. Renaming the leaf CN preserves HHTL and NodeGuid.
#[test]
fn inv07_leaf_rename_preserves_hhtl_and_identity() {
    let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
    let g = "4AQlP4lP0xGaDAMF6CwzAQ==";
    let a = encode(
        &entry(g, "CN=Erika Mustermann,OU=Exchange,DC=example,DC=de"),
        domain(),
        &mut dict,
        &mut pool,
        1,
    )
    .unwrap()
    .record;
    let b = encode(
        &entry(g, "CN=Erika Musterfrau,OU=Exchange,DC=example,DC=de"),
        domain(),
        &mut dict,
        &mut pool,
        2,
    )
    .unwrap()
    .record;
    assert_eq!(a.node_guid(), b.node_guid());
    assert_eq!(a.ou_hhtl(), b.ou_hhtl());
    assert_eq!(dict.len(), 1, "no new segment for a leaf rename");
}

// 12. Unknown/new source attributes do not corrupt the fixed ABI.
#[test]
fn inv12_unknown_attributes_do_not_touch_the_record() {
    let (mut d1, mut p1) = (OuDictionary::new(), ValuePool::new());
    let (mut d2, mut p2) = (OuDictionary::new(), ValuePool::new());
    let g = "4AQlP4lP0xGaDAMF6CwzAQ==";
    let plain = entry(g, "CN=E,OU=Exchange,DC=x");
    let mut noisy = plain.clone();
    for i in 0..200 {
        noisy
            .attrs
            .push((format!("msDS-Future{i}"), vec![0xFF; 300]));
    }
    noisy
        .attrs
        .push(("member;range=0-1499".into(), b"CN=a,DC=x".to_vec()));
    let a = encode(&plain, domain(), &mut d1, &mut p1, 7).unwrap();
    let b = encode(&noisy, domain(), &mut d2, &mut p2, 7).unwrap();
    assert_eq!(a.record.as_bytes(), b.record.as_bytes());
    assert_eq!(
        p1.as_bytes(),
        p2.as_bytes(),
        "unknown values never reach the pool"
    );
    assert_eq!(b.ignored.len(), 201);
    assert!(b.record.reserved_is_zero());
    assert_eq!(b.record.as_bytes().len(), RECORD_BYTES);
    assert!(DirRecord::from_bytes(b.record.as_bytes()).is_ok());
}

#[test]
fn missing_or_malformed_object_guid_is_refused() {
    let (mut d, mut p) = (OuDictionary::new(), ValuePool::new());
    let mut e = entry("4AQlP4lP0xGaDAMF6CwzAQ==", "CN=E,DC=x");
    e.attrs.retain(|(n, _)| n != "objectGUID");
    assert!(encode(&e, domain(), &mut d, &mut p, 0).is_err());
    e.attrs.push(("objectGUID".into(), vec![1, 2, 3]));
    assert!(encode(&e, domain(), &mut d, &mut p, 0).is_err());
}

#[test]
fn too_deep_dn_is_flagged_not_truncated() {
    let (mut d, mut p) = (OuDictionary::new(), ValuePool::new());
    let dn = "CN=E,OU=1,OU=2,OU=3,OU=4,OU=5,OU=6,OU=7,OU=8,OU=9,DC=x";
    let r = encode(
        &entry("4AQlP4lP0xGaDAMF6CwzAQ==", dn),
        domain(),
        &mut d,
        &mut p,
        0,
    )
    .unwrap()
    .record;
    assert_eq!(r.ou_hhtl(), None);
    assert_ne!(r.flags() & ogar_dir_core::record::FLAG_DN_UNENCODED, 0);
    assert!(d.is_empty(), "nothing interned for a refused path");
}

// Bugbot #313: `ldifde -f` emits `changetype: add` after every dn. That is the
// export marker, not a change record — it must be accepted. Real change
// records (modify/delete/modrdn) are still refused.
#[test]
fn ldifde_changetype_add_is_accepted_other_changetypes_refused() {
    let ldifde = "dn: CN=E,OU=Exchange,DC=x\nchangetype: add\nobjectGUID:: 4AQlP4lP0xGaDAMF6CwzAQ==\nobjectClass: user\n";
    let e = ldif::parse(ldifde).unwrap();
    assert_eq!(e.len(), 1);
    assert!(
        e[0].attrs
            .iter()
            .all(|(n, _)| !n.eq_ignore_ascii_case("changetype"))
    );
    let (mut d, mut p) = (OuDictionary::new(), ValuePool::new());
    assert!(
        encode(&e[0], domain(), &mut d, &mut p, 0)
            .unwrap()
            .ignored
            .is_empty()
    );
    for ct in ["modify", "delete", "modrdn", "moddn"] {
        let text = format!("dn: CN=E,DC=x\nchangetype: {ct}\n");
        assert!(
            matches!(ldif::parse(&text), Err(ldif::LdifError::Unsupported(2))),
            "{ct}"
        );
    }
}

// v2: the Exchange recipient triplet of a remote shared mailbox, as the
// on-premises directory holds it. The display type is signed (bit-cast into
// its u32 slot); the 64-bit type details stay their LDAP decimal text.
#[test]
fn exchange_recipient_attributes_are_kept_raw() {
    let text = "dn: CN=Team,OU=Shared,DC=example,DC=test\nobjectGUID:: 4AQlP4lP0xGaDAMF6CwzAQ==\nobjectClass: user\nuserAccountControl: 514\nmsExchRemoteRecipientType: 100\nmsExchRecipientDisplayType: -2147483642\nmsExchRecipientTypeDetails: 34359738368\ntargetAddress: SMTP:team@tenant.mail.onmicrosoft.com\n";
    let e = &ldif::parse(text).unwrap()[0];
    let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
    let enc = encode(e, domain(), &mut dict, &mut pool, 1).unwrap();
    assert!(enc.ignored.is_empty(), "{:?}", enc.ignored);
    let r = enc.record;
    let num = |n: &str| SCHEMA_V1.iter().find(|d| d.name == n).unwrap().slot as usize;
    assert_eq!(r.num(num("msExchRemoteRecipientType")), Some(100));
    assert_eq!(
        r.num(num("msExchRecipientDisplayType")).map(|n| n as i32),
        Some(-2_147_483_642)
    );
    assert_eq!(
        pool.get(r.str_ref(slot("msExchRecipientTypeDetails")).unwrap())
            .unwrap(),
        b"34359738368"
    );
    // The account is disabled: a shared mailbox is a disabled account.
    assert_eq!(r.num(0), Some(514));
}

const CLOUD_ID: &str = "0b5c3a1e-7d2f-4c88-9e10-3f6a2b4c5d6e";

fn hybrid_pool(
    class: &str,
    external: &str,
) -> (Result<ogar_ad::Encoded, ogar_ad::AdError>, ValuePool) {
    let text = format!(
        "dn: CN=X,OU=Staff,DC=example,DC=test\nobjectGUID:: 4AQlP4lP0xGaDAMF6CwzAQ==\nobjectClass: {class}\nmS-DS-ConsistencyGuid:: 4AQlP4lP0xGaDAMF6CwzAQ==\nmsDS-ExternalDirectoryObjectId: {external}\n"
    );
    let e = &ldif::parse(&text).unwrap()[0];
    let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
    (encode(e, domain(), &mut dict, &mut pool, 1), pool)
}
fn hybrid(class: &str, external: &str) -> Result<ogar_ad::Encoded, ogar_ad::AdError> {
    hybrid_pool(class, external).0
}

// In: the label is stripped and both anchors are 128-bit ids in guid slots —
// no string slot holds them. Out: AD's spelling comes back exactly.
#[test]
fn hybrid_identity_anchors_are_ids_not_strings() {
    let (enc, pool) = hybrid_pool("user", &format!("User_{CLOUD_ID}"));
    let enc = enc.unwrap();
    assert!(enc.ignored.is_empty(), "{:?}", enc.ignored);
    let r = enc.record;
    // Entra Connect seeds the anchor from objectGUID by default.
    assert_eq!(r.guid(slot("mS-DS-ConsistencyGuid")), Some(r.node_guid()));
    assert_eq!(
        r.guid(slot("msDS-ExternalDirectoryObjectId")),
        Some(Guid128::parse(CLOUD_ID).unwrap())
    );
    // Neither id is pooled: no string slot holds the label, the id text or
    // the anchor's bytes (only the DN and objectClass are pooled here).
    let anchor = ogar_dir_core::base64::decode("4AQlP4lP0xGaDAMF6CwzAQ==").unwrap();
    let pooled: Vec<&[u8]> = (0..32)
        .filter_map(|s| r.str_ref(s))
        .map(|sr| pool.get(sr).unwrap())
        .collect();
    assert!(!pooled.is_empty());
    for v in pooled {
        let has = |n: &[u8]| v.windows(n.len()).any(|w| w == n);
        assert!(!has(b"User_") && !has(CLOUD_ID.as_bytes()) && !has(&anchor));
    }
    // Out.
    assert_eq!(
        ogar_ad::external_directory_object_id(&r),
        Some(format!("User_{CLOUD_ID}"))
    );
    assert_eq!(
        ogar_ad::consistency_guid(&r).map(|b| b.to_vec()),
        ogar_dir_core::base64::decode("4AQlP4lP0xGaDAMF6CwzAQ==")
    );
    // A group carries Group_, and gets it back.
    let g = hybrid("group", &format!("Group_{CLOUD_ID}"))
        .unwrap()
        .record;
    assert_eq!(
        ogar_ad::external_directory_object_id(&g),
        Some(format!("Group_{CLOUD_ID}"))
    );
}

// The label must match the object kind, and the id must be one: otherwise
// the entry is refused, never half-written and never stored as text.
#[test]
fn a_wrong_or_missing_label_is_refused() {
    use ogar_ad::AdError::BadId;
    let bad = BadId("msDS-ExternalDirectoryObjectId");
    for (class, value) in [
        ("user", format!("Group_{CLOUD_ID}")),
        ("group", format!("User_{CLOUD_ID}")),
        ("user", CLOUD_ID.to_string()),
        ("user", format!("user_{CLOUD_ID}")),
        ("contact", format!("User_{CLOUD_ID}")),
        (
            "user",
            "User_00000000-0000-0000-0000-000000000000".to_string(),
        ),
    ] {
        assert_eq!(hybrid(class, &value).unwrap_err(), bad, "{class} {value}");
    }
}

// v4: the office is kept raw; it becomes the cloud's officeLocation.
#[test]
fn the_office_is_kept_raw() {
    let text = "dn: CN=A,OU=Staff,DC=example,DC=test\nobjectGUID:: 4AQlP4lP0xGaDAMF6CwzAQ==\nobjectClass: user\nphysicalDeliveryOfficeName: Berlin 4.12\n";
    let e = &ldif::parse(text).unwrap()[0];
    let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
    let enc = encode(e, domain(), &mut dict, &mut pool, 1).unwrap();
    assert!(enc.ignored.is_empty(), "{:?}", enc.ignored);
    let r = enc.record;
    assert_eq!(
        pool.get(r.str_ref(slot("physicalDeliveryOfficeName")).unwrap())
            .unwrap(),
        b"Berlin 4.12"
    );
}

// v5: extensionAttribute1..15 are one bag; numbered members outside 1..15
// stay unknown attributes; a member with two values is refused.
#[test]
fn extension_attributes_are_one_bag() {
    let base = "dn: CN=A,OU=Staff,DC=example,DC=test\nobjectGUID:: 4AQlP4lP0xGaDAMF6CwzAQ==\nobjectClass: user\n";
    let enc = |extra: &str| {
        let e = ldif::parse(&format!("{base}{extra}")).unwrap().remove(0);
        let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
        encode(&e, domain(), &mut dict, &mut pool, 1).map(|x| (x, pool))
    };
    let (e, pool) = enc("extensionAttribute15: last\nextensionAttribute3: three\nextensionAttribute16: no\nextensionAttribute0: no\n").unwrap();
    let r = e.record.str_ref(slot("extensionAttribute")).unwrap();
    assert_eq!(pool.bag_member(r, 15), Some(&b"last"[..]));
    assert_eq!(pool.bag_member(r, 3), Some(&b"three"[..]));
    assert_eq!(pool.bag_member(r, 1), None);
    assert_eq!(
        e.ignored,
        vec!["extensionAttribute16", "extensionAttribute0"]
    );
    assert_eq!(
        enc("extensionAttribute2: a\nextensionAttribute2: b\n").unwrap_err(),
        ogar_ad::AdError::MultipleValues("extensionAttribute")
    );
    let (none, _) = enc("").unwrap();
    assert_eq!(none.record.str_ref(slot("extensionAttribute")), None);
    // Codex P2: a non-canonical spelling is not member 1. It is neither
    // stored nor silently dropped: it is reported as ignored.
    let (odd, _) = enc("extensionAttribute01: x\n").unwrap();
    assert_eq!(odd.record.str_ref(slot("extensionAttribute")), None);
    assert_eq!(odd.ignored, vec!["extensionAttribute01"]);
}

// msExchMailboxGuid is the mailbox's own GUID: stored as a 128-bit id in a
// guid slot (never pooled text) and given back as AD's 16 mixed-endian
// bytes. A value that is not 16 bytes is refused, never stored as text.
#[test]
fn the_exchange_guid_is_an_id_not_a_string() {
    const BOX: &str = "Ab5xLqKmTkiZ8gH0cDeFqw==";
    let text = format!(
        "dn: CN=X,OU=Staff,DC=example,DC=test\nobjectGUID:: 4AQlP4lP0xGaDAMF6CwzAQ==\nobjectClass: user\nmsExchMailboxGuid:: {BOX}\n"
    );
    let e = &ldif::parse(&text).unwrap()[0];
    let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
    let enc = encode(e, domain(), &mut dict, &mut pool, 1).unwrap();
    assert!(enc.ignored.is_empty(), "{:?}", enc.ignored);
    let r = enc.record;
    let raw = ogar_dir_core::base64::decode(BOX).unwrap();
    assert_eq!(
        r.guid(slot("msExchMailboxGuid")),
        Some(Guid128::from_ms_bytes(&raw).unwrap())
    );
    assert_ne!(r.guid(slot("msExchMailboxGuid")), Some(r.node_guid()));
    for s in 0..32 {
        if let Some(sr) = r.str_ref(s) {
            let v = pool.get(sr).unwrap();
            assert!(!v.windows(raw.len()).any(|w| w == raw.as_slice()));
        }
    }
    assert_eq!(ogar_ad::exchange_guid(&r).map(|b| b.to_vec()), Some(raw));

    let bad = "dn: CN=X,OU=Staff,DC=example,DC=test\nobjectGUID:: 4AQlP4lP0xGaDAMF6CwzAQ==\nobjectClass: user\nmsExchMailboxGuid:: AAEC\n";
    let e = &ldif::parse(bad).unwrap()[0];
    let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
    assert!(encode(e, domain(), &mut dict, &mut pool, 1).is_err());
}

// groupType is read raw and bit-cast; its high bit, not the mail attributes,
// says whether a group is security-enabled. Unread stays unread.
#[test]
fn group_type_is_read_and_says_security() {
    let group = |gt: Option<&str>| {
        let mut text = String::from(
            "dn: CN=G,OU=Groups,DC=example,DC=test\nobjectGUID:: 4AQlP4lP0xGaDAMF6CwzAQ==\nobjectClass: group\n",
        );
        if let Some(gt) = gt {
            text.push_str(&format!("groupType: {gt}\n"));
        }
        let e = ldif::parse(&text).unwrap().remove(0);
        let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
        let enc = encode(&e, domain(), &mut dict, &mut pool, 1).unwrap();
        assert!(enc.ignored.is_empty(), "{:?}", enc.ignored);
        assert_eq!(enc.record.object_kind(), AdKind::Group as u16);
        enc.record.num(slot("groupType"))
    };
    // Global security group: ADS_GROUP_TYPE_GLOBAL | SECURITY_ENABLED.
    let security = group(Some("-2147483646")).unwrap();
    assert_eq!(security, 0x8000_0002);
    assert!(ogar_ad::is_security_enabled(security));
    // Universal distribution group.
    let distribution = group(Some("8")).unwrap();
    assert!(!ogar_ad::is_security_enabled(distribution));
    // Universal security group.
    assert!(ogar_ad::is_security_enabled(
        group(Some("-2147483640")).unwrap()
    ));
    assert_eq!(group(None), None);
    // Out of 32-bit range is refused, not wrapped.
    let text = "dn: CN=G,OU=Groups,DC=example,DC=test\nobjectGUID:: 4AQlP4lP0xGaDAMF6CwzAQ==\nobjectClass: group\ngroupType: 4294967296\n";
    let e = &ldif::parse(text).unwrap()[0];
    let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
    assert!(encode(e, domain(), &mut dict, &mut pool, 1).is_err());
}
