//! Invariants 1–8, 10–12 of the directory-observation PoC (core side).

use ogar_dir_core::record::{FLAG_OU_PRESENT, RECORD_BYTES, off};
use ogar_dir_core::*;

fn g(s: &str) -> Guid128 {
    Guid128::parse(s).unwrap()
}

const SCHEMA_AD1: SchemaId = SchemaId {
    family: SchemaFamily::AdDs,
    version: 1,
};

// 1. GUIDs survive exact 128-bit round trips — through the record bytes, not
//    just through the string form.
#[test]
fn inv01_guid_round_trips_through_the_record() {
    let node = g("0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0");
    let scope = g("ffffffff-0000-ffff-0000-ffffffffffff");
    let r = DirRecord::new(SCHEMA_AD1, 1, node, scope, 42);
    let back = DirRecord::from_bytes(r.as_bytes()).unwrap();
    assert_eq!(back.node_guid(), node);
    assert_eq!(back.scope_guid(), scope);
    assert_eq!(back.node_guid().0, node.0);
    assert_eq!(&r.as_bytes()[off::NODE_GUID..off::NODE_GUID + 16], &node.0);
}

// 2. Two GUIDs that differ only in their upper 64 bits stay distinct
//    everywhere (no u64 truncation anywhere in the path).
#[test]
fn inv02_upper_64_bits_are_not_truncated() {
    let a = g("00000000-0000-0001-0123-456789abcdef");
    let b = g("00000000-0000-0002-0123-456789abcdef");
    assert_eq!(a.0[8..], b.0[8..], "fixture: lower 64 bits identical");
    assert_ne!(a, b);
    let ra = DirRecord::new(SCHEMA_AD1, 1, a, Guid128::NIL, 0);
    let rb = DirRecord::new(SCHEMA_AD1, 1, b, Guid128::NIL, 0);
    assert_ne!(ra.node_guid(), rb.node_guid());
    assert_ne!(ra.as_bytes(), rb.as_bytes());
    let set: std::collections::HashSet<_> = [ra.node_guid(), rb.node_guid()].into();
    assert_eq!(set.len(), 2);
}

fn ou_dn(ous_leaf_first: &[&str]) -> String {
    let mut s = String::from("CN=Erika Mustermann");
    for o in ous_leaf_first {
        s.push_str(",OU=");
        s.push_str(o);
    }
    s + ",DC=example,DC=de"
}

// 3. A DN with 1–8 OU levels round-trips through HHTL + dictionary.
#[test]
fn inv03_one_to_eight_levels_round_trip() {
    let names = [
        "Stuttgart",
        "Infrastructure",
        "Exchange",
        "L4",
        "L5",
        "L6",
        "L7",
        "L8",
    ];
    let mut dict = OuDictionary::new();
    for depth in 1..=8 {
        let root_first: Vec<&str> = names[..depth].to_vec();
        let leaf_first: Vec<&str> = root_first.iter().rev().copied().collect();
        let dn = Dn::parse(&ou_dn(&leaf_first)).unwrap();
        assert_eq!(dn.ou_path_root_first(), root_first);
        let h = dict.intern(&dn.ou_path_root_first()).unwrap();
        assert_eq!(h.depth(), depth);
        assert_eq!(OuHhtl::from_le_bytes(&h.to_le_bytes()).unwrap(), h);
        assert_eq!(dict.explain(&h).unwrap(), root_first);
        assert_eq!(dict.resolve(&root_first), Some(h));
    }
    // persisted dictionary reconstructs identically
    let reloaded = OuDictionary::from_entries(&dict.entries()).unwrap();
    let h = dict.resolve(&names).unwrap();
    assert_eq!(reloaded.explain(&h).unwrap(), names);
    // and keeps allocating after the reload without reusing an id
    let mut reloaded = reloaded;
    let sib = reloaded.intern(&["Stuttgart", "Finance"]).unwrap();
    assert_ne!(sib, dict.resolve(&["Stuttgart", "Infrastructure"]).unwrap());
    // nine is refused, never truncated
    let nine: Vec<&str> = names.iter().copied().chain(["L9"]).collect();
    assert_eq!(dict.intern(&nine), Err(HhtlError::TooDeep(9)));
}

// 4. DC= components never consume levels. 5. The leaf CN never does.
#[test]
fn inv04_05_dc_and_leaf_are_not_levels() {
    let mut dict = OuDictionary::new();
    let a = Dn::parse("CN=X,OU=Sales,DC=a,DC=b,DC=c,DC=d").unwrap();
    let b = Dn::parse("CN=Y,OU=Sales,DC=z").unwrap();
    let ha = dict.intern(&a.ou_path_root_first()).unwrap();
    let hb = dict.intern(&b.ou_path_root_first()).unwrap();
    assert_eq!(ha.depth(), 1);
    assert_eq!(ha, hb, "neither the DC count nor the leaf changes the HHTL");
    let none = Dn::parse("CN=Z,DC=example,DC=de").unwrap();
    assert_eq!(
        dict.intern(&none.ou_path_root_first()).unwrap(),
        OuHhtl::ROOT
    );
    // An OU object's own name is its leaf, not a level of its location.
    let ou_obj = Dn::parse("OU=Exchange,OU=Infrastructure,DC=x").unwrap();
    assert_eq!(ou_obj.ou_path_root_first(), ["Infrastructure"]);
}

// 8. Segment allocation cannot collide.
#[test]
fn inv08_allocation_is_collision_free() {
    let mut dict = OuDictionary::new();
    // 2000 siblings under one parent: a 16-bit hash would collide here
    // (birthday bound ~300); the explicit dictionary never does.
    let mut seen = std::collections::HashSet::new();
    for i in 0..2000 {
        let h = dict.intern(&["Root", &format!("ou-{i}")]).unwrap();
        assert!(seen.insert(h), "collision at {i}");
    }
    // case-insensitive: same OU under different spelling is the same segment
    assert_eq!(
        dict.intern(&["root", "OU-7"]).unwrap(),
        dict.resolve(&["Root", "ou-7"]).unwrap()
    );
    // same name under different parents: different paths
    let x = dict.intern(&["A", "Exchange"]).unwrap();
    let y = dict.intern(&["B", "Exchange"]).unwrap();
    assert_ne!(x, y);
    // corrupted persisted state (two names on one id) is refused
    let mut e = dict.entries();
    let dup = (e[0].0, e[0].1, "Intruder".to_string());
    e.push(dup);
    assert_eq!(
        OuDictionary::from_entries(&e).unwrap_err(),
        HhtlError::Collision
    );
    // exhaustion is an error, never a wrap-around onto id 0 / id 1
    let mut d = OuDictionary::new();
    for i in 0..u16::MAX as u32 {
        d.intern(&[format!("n{i}")]).unwrap();
    }
    assert_eq!(d.intern(&["one-too-many"]), Err(HhtlError::ParentExhausted));
}

// 10. The fixed DTO is exactly 512 bytes, layout locked.
#[test]
fn inv10_record_is_512_bytes() {
    assert_eq!(std::mem::size_of::<DirRecord>(), 512);
    assert_eq!(RECORD_BYTES, 0x200);
    let r = DirRecord::new(SCHEMA_AD1, 1, Guid128::NIL, Guid128::NIL, 0);
    assert_eq!(r.as_bytes().len(), 512);
    assert_eq!(&r.as_bytes()[0x20..0x24], b"OGDR");
    assert!(r.reserved_is_zero());
    // canonical NodeRow split: key 16 | edges 16 | value 480
    assert_eq!(
        (off::CANONICAL_EDGES, off::VALUE, 512 - off::VALUE),
        (16, 32, 480)
    );
}

// 11. A schema version change is not an ABI version change.
#[test]
fn inv11_schema_version_is_independent_of_abi() {
    let v1 = DirRecord::new(SCHEMA_AD1, 1, Guid128::NIL, Guid128::NIL, 0);
    let v2 = DirRecord::new(
        SchemaId {
            version: 2,
            ..SCHEMA_AD1
        },
        1,
        Guid128::NIL,
        Guid128::NIL,
        0,
    );
    assert_eq!(v1.abi(), v2.abi());
    assert_ne!(v1.schema(), v2.schema());
    // only the schema_version bytes differ
    let diff: Vec<usize> = (0..512)
        .filter(|&i| v1.as_bytes()[i] != v2.as_bytes()[i])
        .collect();
    assert_eq!(diff, vec![off::SCHEMA_VERSION]);
    // a foreign ABI major is rejected, a newer minor is accepted
    let mut b = *v1.as_bytes();
    b[off::ABI_MAJOR] = 2;
    assert_eq!(DirRecord::from_bytes(&b), Err(RecordError::AbiMajor(2)));
    let mut b = *v1.as_bytes();
    b[off::ABI_MINOR] = 9;
    assert!(DirRecord::from_bytes(&b).is_ok());
}

// The decoder refuses malformed HHTL bytes rather than misreading them.
#[test]
fn hhtl_gap_is_rejected_on_decode() {
    let mut r = DirRecord::new(SCHEMA_AD1, 1, Guid128::NIL, Guid128::NIL, 0);
    r.set_ou_hhtl(OuHhtl([1, 0, 3, 0, 0, 0, 0, 0]));
    assert_eq!(r.flags() & FLAG_OU_PRESENT, FLAG_OU_PRESENT);
    assert!(matches!(
        DirRecord::from_bytes(r.as_bytes()),
        Err(RecordError::Hhtl(HhtlError::Gap(2)))
    ));
}

#[test]
fn edge_record_round_trips_and_keeps_endpoints_distinct() {
    let e = DirEdge {
        kind: EdgeKind::SynchronizesTo,
        evidence: EdgeEvidence::ImmutableIdIsObjectGuid,
        src: (
            SchemaFamily::AdDs,
            g("11111111-1111-1111-1111-111111111111"),
        ),
        dst: (
            SchemaFamily::MsGraph,
            g("22222222-2222-2222-2222-222222222222"),
        ),
    };
    let b = e.to_bytes();
    assert_eq!(b.len(), 64);
    assert_eq!(DirEdge::from_bytes(&b), Some(e));
    assert_ne!(e.src.1, e.dst.1);
}

// ABI minor 1: four inline 128-bit id slots. A value round-trips through the
// record bytes, each slot is independent of the others and of the pooled and
// numeric slots, and a minor-0 record (zero bytes, presence clear) reads as
// "no id".
#[test]
fn guid_slots_are_inline_and_additive() {
    let id = g("0b5c3a1e-7d2f-4c88-9e10-3f6a2b4c5d6e");
    let mut r = DirRecord::new(SCHEMA_AD1, 1, Guid128::NIL, Guid128::NIL, 0);
    assert_eq!(r.abi(), (ABI_MAJOR, 1));
    assert!((0..GUID_SLOTS).all(|s| r.guid(s).is_none()));
    r.set_guid(1, id).unwrap();
    assert_eq!(
        r.set_guid(GUID_SLOTS, id),
        Err(RecordError::Slot(GUID_SLOTS))
    );
    let back = DirRecord::from_bytes(r.as_bytes()).unwrap();
    assert_eq!(back.guid(1), Some(id));
    assert_eq!(back.guid(0), None);
    assert!((0..32).all(|s| back.str_ref(s).is_none()));
    assert!((0..4).all(|s| back.num(s).is_none()));
    assert!(back.reserved_is_zero());
    assert_eq!(&back.as_bytes()[off::GUIDS + 16..off::GUIDS + 32], &id.0);
    // A minor-0 writer left these bytes zero and the presence bits clear.
    let mut old = *DirRecord::new(SCHEMA_AD1, 1, Guid128::NIL, Guid128::NIL, 0).as_bytes();
    old[off::ABI_MINOR] = 0;
    let old = DirRecord::from_bytes(&old).unwrap();
    assert!((0..GUID_SLOTS).all(|s| old.guid(s).is_none()));
}

#[test]
fn schema_validation_knows_the_guid_space() {
    let def = |name, slot, kind| AttrDef {
        name,
        slot,
        kind,
        since: 1,
    };
    // A guid slot may share an index with a pooled and a numeric slot.
    assert!(
        schema::validate(&[
            def("a", 0, AttrKind::Str),
            def("b", 0, AttrKind::U32),
            def("c", 0, AttrKind::Guid),
        ])
        .is_ok()
    );
    assert!(schema::validate(&[def("a", 1, AttrKind::Guid), def("b", 1, AttrKind::Guid)]).is_err());
    assert!(schema::validate(&[def("a", GUID_SLOTS as u8, AttrKind::Guid)]).is_err());
}

// A bag member is named by its exact canonical spelling only: `01` is not
// member 1, and a name with a multi-byte character at the prefix boundary
// is simply not a member (it must not panic).
#[test]
fn bag_members_are_named_exactly() {
    use ogar_dir_core::schema::bag_member_of;
    let bag = "extensionAttribute";
    assert_eq!(bag_member_of(bag, "extensionAttribute7"), Some(7));
    assert_eq!(bag_member_of(bag, "ExtensionAttribute15"), Some(15));
    assert_eq!(bag_member_of(bag, "extensionattribute1"), Some(1));
    for not in [
        "extensionAttribute",
        "extensionAttribute0",
        "extensionAttribute01",
        "extensionAttribute16",
        "extensionAttribute+1",
        "extensionAttribute 1",
        "abcdefghijklmnopqé",
        "extensionAttributé1",
        "mail",
    ] {
        assert_eq!(bag_member_of(bag, not), None, "{not}");
    }
}

// objectSid is binary; Graph's onPremisesSecurityIdentifier is its SDDL
// string form, S-{revision}-{authority}-{subauthority}...
#[test]
fn a_binary_sid_renders_as_its_string_form() {
    use ogar_dir_core::sid::sid_to_string;
    let sid = [
        0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xdc, 0xf4, 0xdc,
        0x3b, 0x83, 0x3d, 0x2b, 0x46, 0x82, 0x8b, 0xa6, 0x28, 0x00, 0x02, 0x00, 0x00,
    ];
    assert_eq!(
        sid_to_string(&sid).as_deref(),
        Some("S-1-5-21-1004336348-1177238915-682003330-512")
    );
    // Well-known SID with no sub-authorities beyond one: Everyone, S-1-1-0.
    assert_eq!(
        sid_to_string(&[1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0]).as_deref(),
        Some("S-1-1-0")
    );
    // The length must match the sub-authority count exactly.
    assert_eq!(sid_to_string(&sid[..27]), None);
    let mut long = sid.to_vec();
    long.push(0);
    assert_eq!(sid_to_string(&long), None);
    assert_eq!(sid_to_string(&[]), None);
    assert_eq!(sid_to_string(&[1]), None);
}
