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
