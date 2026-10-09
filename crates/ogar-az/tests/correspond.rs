//! Hybrid identity correspondence: falsifiers F1-F14 and a randomized
//! comparison with an independent reference.
//!
//! Every fixture goes through the real encoders (`ogar_ad::encode`,
//! `ogar_az::encode_user`), so the fold sees the records production builds.
//! The reference ([`oracle`]) reads the fixture specification, not the
//! records, and uses plain loops over the intended ids; it never calls the
//! fold, the index or the rule codecs.

use ogar_ad::AdEntry;
use ogar_dir_core::correspond::{
    AdStatus, Binding, Index, Lanes, Output, Planes, Profile, Ruler, Source, flag, fold,
    fold_aligned,
};
use ogar_dir_core::identity::{
    BACKSYNC_TO_ENTRA_ID, CONSISTENCY_GUID_TO_IMMUTABLE_ID, ENTRA_ID_TO_EXO_EXTERNAL_ID,
    OBJECT_GUID_TO_IMMUTABLE_ID,
};
use ogar_dir_core::{
    AttrDef, AttrKind, DirRecord, EdgeEvidence, Guid128, IdentityRule, OuDictionary, SchemaFamily,
    SchemaId, ValuePool, base64,
};
use serde_json::{Map, Value, json};

const EXO_SCHEMA: &[AttrDef] = &[AttrDef {
    name: "ExternalDirectoryObjectId",
    slot: 0,
    kind: AttrKind::Guid,
    since: 1,
}];

fn g(n: u64) -> Guid128 {
    // A spread-out, non-nil id per number; the high bits make the textual
    // and mixed-endian orders differ.
    let mut b = [0u8; 16];
    b[..8].copy_from_slice(&(n.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1).to_be_bytes());
    b[8..].copy_from_slice(&n.to_be_bytes());
    Guid128(b)
}

const DOMAIN: u64 = 9_001;
const TENANT: u64 = 9_002;
const OTHER_DOMAIN: u64 = 9_003;
const OTHER_TENANT: u64 = 9_004;

/// A value as the fixture means it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wire {
    Absent,
    Id(Guid128),
    /// Present but not a value of the encoding.
    Malformed,
    /// The all-zero id, spelled correctly.
    Nil,
}

#[derive(Clone, Debug)]
struct Ad {
    guid: Guid128,
    domain: Guid128,
    cg: Wire,
    backsync: Wire,
    group: bool,
    at: i64,
}

#[derive(Clone, Debug)]
struct Entra {
    id: Guid128,
    tenant: Guid128,
    immutable: Wire,
    upn: Option<&'static str>,
    at: i64,
}

#[derive(Clone, Debug)]
struct Exo {
    guid: Guid128,
    tenant: Guid128,
    external: Wire,
}

#[derive(Clone, Debug, Default)]
struct World {
    ad: Vec<Ad>,
    entra: Vec<Entra>,
    exo: Vec<Exo>,
}

fn ad(guid: Guid128, cg: Wire, backsync: Wire) -> Ad {
    Ad {
        guid,
        domain: g(DOMAIN),
        cg,
        backsync,
        group: false,
        at: 1_000,
    }
}

fn entra(id: Guid128, immutable: Wire) -> Entra {
    Entra {
        id,
        tenant: g(TENANT),
        immutable,
        upn: None,
        at: 1_000,
    }
}

// ---- the fixture as the encoders see it -----------------------------------

fn ad_entry(a: &Ad) -> AdEntry {
    let mut attrs = vec![
        ("objectGUID".to_string(), a.guid.to_ms_bytes().to_vec()),
        (
            "objectClass".to_string(),
            if a.group {
                b"group".to_vec()
            } else {
                b"user".to_vec()
            },
        ),
    ];
    match a.cg {
        Wire::Absent => {}
        Wire::Id(id) => attrs.push(("mS-DS-ConsistencyGuid".into(), id.to_ms_bytes().to_vec())),
        Wire::Malformed => attrs.push(("mS-DS-ConsistencyGuid".into(), vec![7u8; 15])),
        Wire::Nil => attrs.push(("mS-DS-ConsistencyGuid".into(), vec![0u8; 16])),
    }
    let label = if a.group { "Group_" } else { "User_" };
    match a.backsync {
        Wire::Absent => {}
        Wire::Id(id) => attrs.push((
            "msDS-ExternalDirectoryObjectId".into(),
            format!("{label}{id}").into_bytes(),
        )),
        Wire::Malformed => attrs.push((
            "msDS-ExternalDirectoryObjectId".into(),
            b"User_not-a-guid".to_vec(),
        )),
        Wire::Nil => attrs.push((
            "msDS-ExternalDirectoryObjectId".into(),
            format!("{label}{}", Guid128::NIL).into_bytes(),
        )),
    }
    AdEntry {
        dn: String::new(),
        attrs,
    }
}

fn entra_obj(e: &Entra) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("id".into(), json!(e.id.to_string()));
    let imm = match e.immutable {
        Wire::Absent => None,
        Wire::Id(id) => Some(base64::encode(&id.to_ms_bytes())),
        Wire::Malformed => Some("not base64!".to_string()),
        Wire::Nil => Some(base64::encode(&[0u8; 16])),
    };
    if let Some(s) = imm {
        m.insert("onPremisesImmutableId".into(), json!(s));
    }
    if let Some(u) = e.upn {
        m.insert("userPrincipalName".into(), json!(u));
        m.insert("mail".into(), json!(u));
    }
    m
}

struct Encoded {
    ad: Vec<DirRecord>,
    ad_pool: ValuePool,
    entra: Vec<DirRecord>,
    entra_pool: ValuePool,
    exo: Vec<DirRecord>,
    exo_pool: ValuePool,
    /// AD entries the encoder refused, by fixture index.
    ad_refused: Vec<usize>,
}

fn encode(w: &World) -> Encoded {
    let (mut dict, mut ad_pool) = (OuDictionary::new(), ValuePool::new());
    let mut ad_out = Vec::new();
    let mut ad_refused = Vec::new();
    for (i, a) in w.ad.iter().enumerate() {
        match ogar_ad::encode(&ad_entry(a), a.domain, &mut dict, &mut ad_pool, a.at) {
            Ok(e) => ad_out.push(e.record),
            Err(_) => ad_refused.push(i),
        }
    }
    let mut entra_pool = ValuePool::new();
    let entra_out = w
        .entra
        .iter()
        .enumerate()
        .map(|(i, e)| {
            ogar_az::encode_user(&entra_obj(e), i, e.tenant, &mut dict, &mut entra_pool, e.at)
                .unwrap()
        })
        .collect();
    let exo_schema = SchemaId {
        family: SchemaFamily::ExchangeOnline,
        version: 1,
    };
    let exo_out = w
        .exo
        .iter()
        .map(|x| {
            let mut r = DirRecord::new(exo_schema, 1, x.guid, x.tenant, 1_000);
            if let Wire::Id(id) = x.external {
                r.set_guid(0, id).unwrap();
            }
            r
        })
        .collect();
    Encoded {
        ad: ad_out,
        ad_pool,
        entra: entra_out,
        entra_pool,
        exo: exo_out,
        exo_pool: ValuePool::new(),
        ad_refused,
    }
}

fn profile(anchor: IdentityRule) -> Profile {
    Profile {
        anchor: anchor
            .compile(ogar_ad::SCHEMA_V1, ogar_az::SCHEMA_V1)
            .unwrap(),
        backsync: BACKSYNC_TO_ENTRA_ID
            .compile(ogar_ad::SCHEMA_V1, ogar_az::SCHEMA_V1)
            .unwrap(),
        cloud: ENTRA_ID_TO_EXO_EXTERNAL_ID
            .compile(ogar_az::SCHEMA_V1, EXO_SCHEMA)
            .unwrap(),
        max_skew_ms: 60_000,
    }
}

fn bindings() -> Vec<Binding> {
    vec![Binding {
        ad_domain: g(DOMAIN),
        tenant: g(TENANT),
    }]
}

struct Run {
    lanes: Lanes,
    out: Output,
}

fn run(w: &World, anchor: IdentityRule) -> Run {
    let e = encode(w);
    assert!(e.ad_refused.is_empty(), "refused: {:?}", e.ad_refused);
    run_encoded(&e, anchor, &bindings())
}

fn run_encoded(e: &Encoded, anchor: IdentityRule, b: &[Binding]) -> Run {
    let lanes = Lanes::build(
        &profile(anchor),
        b,
        Source {
            records: &e.ad,
            pool: &e.ad_pool,
        },
        Source {
            records: &e.entra,
            pool: &e.entra_pool,
        },
        Source {
            records: &e.exo,
            pool: &e.exo_pool,
        },
    );
    let ix = Index::build(&lanes);
    let mut out = Output::for_index(&lanes, &ix);
    fold(&lanes, &ix, &mut out);
    assert_eq!(out.edges.len(), ix.edges, "the index counts every edge");
    // The same rows on one ruler: the position-wise fold confirms exactly
    // the slots the join confirms.
    let ruler = Ruler::from_fold(&lanes, &ix, &out);
    let mut planes = Planes::for_ruler(&ruler);
    fold_aligned(&ruler, &mut planes);
    for (i, &f) in out.ad.iter().enumerate() {
        let bit = planes.confirmed(i / 64) >> (i % 64) & 1 == 1;
        assert_eq!(
            bit,
            AdStatus::of(f) == AdStatus::Confirmed,
            "slot {i}: ruler vs join, ad {f:x} entra {:x?}",
            out.entra
        );
    }
    Run { lanes, out }
}

fn statuses(r: &Run) -> Vec<AdStatus> {
    r.out.ad.iter().map(|&f| AdStatus::of(f)).collect()
}

fn edges(r: &Run, ev: EdgeEvidence) -> Vec<(Guid128, Guid128)> {
    r.out
        .edges
        .iter()
        .filter(|e| e.evidence == ev)
        .map(|e| (e.src.1, e.dst.1))
        .collect()
}

// ---- the independent reference --------------------------------------------

/// Expected status per AD row, from the specification only.
fn oracle(w: &World, anchor_is_object_guid: bool, b: &[Binding], skew: i64) -> Vec<AdStatus> {
    let tenant_of = |d: Guid128| b.iter().find(|x| x.ad_domain == d).map(|x| x.tenant);
    let anchor = |a: &Ad| {
        if anchor_is_object_guid {
            Wire::Id(a.guid)
        } else {
            a.cg
        }
    };
    let mut res = Vec::new();
    for a in &w.ad {
        let Some(t) = tenant_of(a.domain) else {
            res.push(AdStatus::OutOfScope);
            continue;
        };
        let back_target: Vec<&Entra> = match a.backsync {
            Wire::Id(id) => w
                .entra
                .iter()
                .filter(|e| e.tenant == t && e.id == id)
                .collect(),
            _ => vec![],
        };
        let back_present = matches!(a.backsync, Wire::Id(_));
        let id = match anchor(a) {
            Wire::Malformed | Wire::Nil => {
                res.push(AdStatus::InvalidSourceAnchor);
                continue;
            }
            Wire::Absent => {
                let split = back_target.len() == 1
                    && split_at(w, b, t, back_target[0], anchor_is_object_guid);
                res.push(if split {
                    AdStatus::Contradictory
                } else if back_target.len() == 1 {
                    AdStatus::BacksyncOnly
                } else {
                    AdStatus::MissingSourceAnchor
                });
                continue;
            }
            Wire::Id(id) => id,
        };
        let sharers =
            w.ad.iter()
                .filter(|x| tenant_of(x.domain) == Some(t) && anchor(x) == Wire::Id(id))
                .count();
        if sharers > 1 {
            res.push(AdStatus::AmbiguousSource);
            continue;
        }
        let fwd: Vec<&Entra> = w
            .entra
            .iter()
            .filter(|e| e.tenant == t && e.immutable == Wire::Id(id))
            .collect();
        if fwd.len() > 1 {
            res.push(AdStatus::AmbiguousTarget);
            continue;
        }
        if fwd.is_empty() {
            res.push(if back_target.len() == 1 {
                AdStatus::Contradictory
            } else {
                AdStatus::Unresolved
            });
            continue;
        }
        let x = fwd[0];
        let split = split_at(w, b, t, x, anchor_is_object_guid)
            || (back_target.len() == 1 && split_at(w, b, t, back_target[0], anchor_is_object_guid));
        let agree = back_target.len() == 1 && back_target[0].id == x.id;
        res.push(if split || (back_present && !agree) {
            AdStatus::Contradictory
        } else if agree {
            if a.at != 0 && x.at != 0 && (a.at - x.at).abs() <= skew {
                AdStatus::Confirmed
            } else {
                AdStatus::AgreesAcrossTime
            }
        } else {
            AdStatus::ForwardOnly
        });
    }
    res
}

/// Entra row `x` is contested: claimed or named by several AD rows, or
/// claimed by one and named by another.
fn split_at(w: &World, b: &[Binding], t: Guid128, x: &Entra, by_object_guid: bool) -> bool {
    let in_t = |a: &&Ad| b.iter().any(|y| y.ad_domain == a.domain && y.tenant == t);
    let claimers: Vec<&Ad> = match x.immutable {
        Wire::Id(id) => {
            w.ad.iter()
                .filter(in_t)
                .filter(|a| {
                    (if by_object_guid {
                        Wire::Id(a.guid)
                    } else {
                        a.cg
                    }) == Wire::Id(id)
                })
                .collect()
        }
        _ => vec![],
    };
    let namers: Vec<&Ad> =
        w.ad.iter()
            .filter(in_t)
            .filter(|a| a.backsync == Wire::Id(x.id))
            .collect();
    // Rows are compared as rows: two observations can carry one objectGUID.
    claimers.len() > 1
        || namers.len() > 1
        || (claimers.len() == 1 && namers.len() == 1 && !std::ptr::eq(claimers[0], namers[0]))
}

fn check(w: &World, anchor: IdentityRule) -> Run {
    let r = run(w, anchor);
    let want = oracle(
        w,
        anchor == OBJECT_GUID_TO_IMMUTABLE_ID,
        &bindings(),
        60_000,
    );
    assert_eq!(
        statuses(&r),
        want,
        "fold disagrees with the reference: {w:#?}\nflags {:x?} entra {:x?}",
        r.out.ad,
        r.out.entra
    );
    r
}

// ---- F1-F14 ----------------------------------------------------------------

/// F1: the anchor is `mS-DS-ConsistencyGuid` and differs from `objectGUID`.
#[test]
fn f01_consistency_guid_differs_from_object_guid() {
    let (a, s, x) = (g(1), g(2), g(3));
    let w = World {
        ad: vec![ad(a, Wire::Id(s), Wire::Id(x))],
        entra: vec![entra(x, Wire::Id(s))],
        ..World::default()
    };
    assert_ne!(a, s);
    let r = check(&w, CONSISTENCY_GUID_TO_IMMUTABLE_ID);
    assert_eq!(statuses(&r), [AdStatus::Confirmed]);
    assert_eq!(
        edges(&r, EdgeEvidence::SourceAnchorMatchesImmutableId),
        [(a, x)]
    );
    assert!(edges(&r, EdgeEvidence::ImmutableIdIsObjectGuid).is_empty());
    // Two nodes, related; neither carries the other's id.
    assert_eq!(r.lanes.ad.owner[0], a);
    assert_eq!(r.lanes.entra.owner[0], x);

    // The objectGUID profile must not claim this correspondence.
    let r = check(&w, OBJECT_GUID_TO_IMMUTABLE_ID);
    assert_eq!(statuses(&r), [AdStatus::Contradictory]);
    assert!(edges(&r, EdgeEvidence::ImmutableIdIsObjectGuid).is_empty());
    // The existing objectGUID-only API finds nothing either.
    let e = encode(&w);
    let observed = std::collections::HashSet::from([a]);
    assert!(ogar_az::sync_edges(&e.entra, &e.entra_pool, &observed).is_empty());
}

/// F2: a deployment that anchors on `objectGUID`.
#[test]
fn f02_explicit_object_guid_profile() {
    let (a, x) = (g(1), g(3));
    let w = World {
        ad: vec![ad(a, Wire::Absent, Wire::Id(x))],
        entra: vec![entra(x, Wire::Id(a))],
        ..World::default()
    };
    let r = check(&w, OBJECT_GUID_TO_IMMUTABLE_ID);
    assert_eq!(statuses(&r), [AdStatus::Confirmed]);
    // Legacy evidence code, legacy meaning.
    assert_eq!(edges(&r, EdgeEvidence::ImmutableIdIsObjectGuid), [(a, x)]);
}

/// F3: no implicit fallback. The configured anchor is missing; `objectGUID`
/// happens to match the immutable id.
#[test]
fn f03_no_fallback_to_object_guid() {
    let (a, x) = (g(1), g(3));
    let w = World {
        ad: vec![ad(a, Wire::Absent, Wire::Absent)],
        entra: vec![entra(x, Wire::Id(a))],
        ..World::default()
    };
    let r = check(&w, CONSISTENCY_GUID_TO_IMMUTABLE_ID);
    assert_eq!(statuses(&r), [AdStatus::MissingSourceAnchor]);
    assert!(r.out.edges.is_empty());
    assert_eq!(r.out.entra[0] & flag::E_ORPHAN, flag::E_ORPHAN);
}

/// F4: backsync agrees with the anchor.
#[test]
fn f04_backsync_agrees() {
    let (a, s, x) = (g(1), g(2), g(3));
    let w = World {
        ad: vec![ad(a, Wire::Id(s), Wire::Id(x))],
        entra: vec![entra(x, Wire::Id(s))],
        ..World::default()
    };
    let r = check(&w, CONSISTENCY_GUID_TO_IMMUTABLE_ID);
    assert_ne!(r.out.ad[0] & flag::AGREE, 0);
    assert_eq!(
        edges(&r, EdgeEvidence::AdBacksyncMatchesEntraObjectId),
        [(a, x)]
    );
}

/// F5: the anchor reaches Entra X, the backsync names Entra Y.
#[test]
fn f05_backsync_contradicts() {
    let (a, s, x, y) = (g(1), g(2), g(3), g(4));
    let w = World {
        ad: vec![ad(a, Wire::Id(s), Wire::Id(y))],
        entra: vec![entra(x, Wire::Id(s)), entra(y, Wire::Absent)],
        ..World::default()
    };
    let r = check(&w, CONSISTENCY_GUID_TO_IMMUTABLE_ID);
    assert_eq!(statuses(&r), [AdStatus::Contradictory]);
    // Both witnesses stay: no repair, no choice between them.
    assert_eq!(
        edges(&r, EdgeEvidence::SourceAnchorMatchesImmutableId),
        [(a, x)]
    );
    assert_eq!(
        edges(&r, EdgeEvidence::AdBacksyncMatchesEntraObjectId),
        [(a, y)]
    );
}

/// F5b: one cloud object, claimed by one AD row's anchor and named by
/// another AD row's backsync.
#[test]
fn f05b_witnesses_split_across_ad_rows() {
    let (a1, a2, s, x) = (g(1), g(5), g(2), g(3));
    let w = World {
        ad: vec![
            ad(a1, Wire::Id(s), Wire::Absent),
            ad(a2, Wire::Absent, Wire::Id(x)),
        ],
        entra: vec![entra(x, Wire::Id(s))],
        ..World::default()
    };
    let r = check(&w, CONSISTENCY_GUID_TO_IMMUTABLE_ID);
    assert_eq!(
        statuses(&r),
        [AdStatus::Contradictory, AdStatus::Contradictory]
    );
    assert_ne!(r.out.entra[0] & flag::E_SPLIT, 0);
}

/// F6: two AD objects carry one anchor.
#[test]
fn f06_duplicate_source_anchor() {
    let (a1, a2, s, x) = (g(1), g(5), g(2), g(3));
    let w = World {
        ad: vec![
            ad(a1, Wire::Id(s), Wire::Absent),
            ad(a2, Wire::Id(s), Wire::Absent),
        ],
        entra: vec![entra(x, Wire::Id(s))],
        ..World::default()
    };
    let r = check(&w, CONSISTENCY_GUID_TO_IMMUTABLE_ID);
    assert_eq!(
        statuses(&r),
        [AdStatus::AmbiguousSource, AdStatus::AmbiguousSource]
    );
    // Both witnesses kept; the order of the input does not pick a winner.
    let mut got = edges(&r, EdgeEvidence::SourceAnchorMatchesImmutableId);
    got.sort();
    let mut want = vec![(a1, x), (a2, x)];
    want.sort();
    assert_eq!(got, want);
    assert_ne!(r.out.entra[0] & flag::E_CLAIMED_MANY, 0);
}

/// F7: two cloud objects carry one anchor.
#[test]
fn f07_cloud_duplication() {
    let (a, s, x, y) = (g(1), g(2), g(3), g(4));
    let w = World {
        ad: vec![ad(a, Wire::Id(s), Wire::Absent)],
        entra: vec![entra(x, Wire::Id(s)), entra(y, Wire::Id(s))],
        ..World::default()
    };
    let r = check(&w, CONSISTENCY_GUID_TO_IMMUTABLE_ID);
    assert_eq!(statuses(&r), [AdStatus::AmbiguousTarget]);
    assert_eq!(r.out.ad_target[0], ogar_dir_core::correspond::NO_ROW);
    assert!(r.out.entra.iter().all(|f| f & flag::E_ANCHOR_SHARED != 0));
}

/// F8: the label must match the object's kind.
#[test]
fn f08_wrong_label_is_refused() {
    let x = g(3);
    let mut grp = ad(g(1), Wire::Absent, Wire::Absent);
    grp.group = true;
    let mut entry = ad_entry(&grp);
    entry.attrs.push((
        "msDS-ExternalDirectoryObjectId".into(),
        format!("User_{x}").into_bytes(),
    ));
    let (mut d, mut p) = (OuDictionary::new(), ValuePool::new());
    assert_eq!(
        ogar_ad::encode(&entry, g(DOMAIN), &mut d, &mut p, 1).unwrap_err(),
        ogar_ad::AdError::BadId("msDS-ExternalDirectoryObjectId")
    );
    let mut user = ad_entry(&ad(g(1), Wire::Absent, Wire::Absent));
    user.attrs.push((
        "msDS-ExternalDirectoryObjectId".into(),
        format!("Group_{x}").into_bytes(),
    ));
    assert!(ogar_ad::encode(&user, g(DOMAIN), &mut d, &mut p, 1).is_err());
    // And the codec agrees on its own.
    let rule = BACKSYNC_TO_ENTRA_ID.source.encoding;
    assert!(rule.decode(format!("Group_{x}").as_bytes()).is_err());
    assert_eq!(rule.decode(format!("User_{x}").as_bytes()), Ok(x));
}

/// F9: Microsoft's mixed-endian vector and its base64.
#[test]
fn f09_byte_order() {
    let id = Guid128::parse("3f2504e0-4f89-11d3-9a0c-0305e82c3301").unwrap();
    let b64 = "4AQlP4lP0xGaDAMF6CwzAQ==";
    let rule = CONSISTENCY_GUID_TO_IMMUTABLE_ID;
    assert_eq!(rule.target.encoding.decode(b64.as_bytes()), Ok(id));
    // The naive reading, bytes in wire order, is a different id.
    let wire = base64::decode(b64).unwrap();
    assert_ne!(Guid128(wire.clone().try_into().unwrap()), id);
    // End to end: the AD side stores the decoded anchor, the Entra side the
    // base64; they meet only through the declared conversion.
    let w = World {
        ad: vec![ad(g(1), Wire::Id(id), Wire::Absent)],
        entra: vec![Entra {
            immutable: Wire::Id(id),
            ..entra(g(3), Wire::Absent)
        }],
        ..World::default()
    };
    let e = encode(&w);
    assert_eq!(
        ogar_az::attr_str(&e.entra[0], &e.entra_pool, "onPremisesImmutableId"),
        Some(b64)
    );
    assert_eq!(statuses(&check(&w, rule)), [AdStatus::ForwardOnly]);
}

/// F10: observations taken at different times.
#[test]
fn f10_stale_observation_is_not_simultaneous() {
    let (a, s, x) = (g(1), g(2), g(3));
    let mut w = World {
        ad: vec![ad(a, Wire::Id(s), Wire::Id(x))],
        entra: vec![entra(x, Wire::Id(s))],
        ..World::default()
    };
    w.entra[0].at = 1_000 + 3_600_000;
    assert_eq!(
        statuses(&check(&w, CONSISTENCY_GUID_TO_IMMUTABLE_ID)),
        [AdStatus::AgreesAcrossTime]
    );
    w.entra[0].at = 0; // unknown time
    assert_eq!(
        statuses(&check(&w, CONSISTENCY_GUID_TO_IMMUTABLE_ID)),
        [AdStatus::AgreesAcrossTime]
    );
}

/// F11: identical bytes in another tenant do not join.
#[test]
fn f11_same_guid_other_scope() {
    let (a, s, x, y) = (g(1), g(2), g(3), g(4));
    let mut stranger = entra(y, Wire::Id(s));
    stranger.tenant = g(OTHER_TENANT);
    let mut unbound = ad(g(6), Wire::Id(s), Wire::Absent);
    unbound.domain = g(OTHER_DOMAIN);
    let w = World {
        ad: vec![ad(a, Wire::Id(s), Wire::Id(x)), unbound],
        entra: vec![entra(x, Wire::Id(s)), stranger],
        ..World::default()
    };
    let r = check(&w, CONSISTENCY_GUID_TO_IMMUTABLE_ID);
    assert_eq!(statuses(&r), [AdStatus::Confirmed, AdStatus::OutOfScope]);
    assert!(
        r.out.edges.iter().all(|e| e.dst.1 != y),
        "no edge into the other tenant"
    );
    assert_ne!(r.out.entra[1] & flag::E_UNBOUND_SCOPE, 0);
}

/// F12: absent, malformed, nil and unmatched are four different answers.
#[test]
fn f12_missing_malformed_nil_unknown() {
    let s = g(2);
    // AD refuses a malformed or nil consistency guid at ingest: the entry is
    // not observed at all, it does not become a row with no anchor.
    for bad in [Wire::Malformed, Wire::Nil] {
        let w = World {
            ad: vec![ad(g(1), bad, Wire::Absent)],
            ..World::default()
        };
        assert_eq!(encode(&w).ad_refused, [0], "{bad:?}");
    }
    // The cloud side keeps its raw value, so every state reaches the lane.
    let w = World {
        ad: vec![
            ad(g(1), Wire::Absent, Wire::Absent),
            ad(g(5), Wire::Id(s), Wire::Absent),
        ],
        entra: vec![
            entra(g(3), Wire::Absent),
            entra(g(4), Wire::Malformed),
            entra(g(7), Wire::Nil),
            entra(g(8), Wire::Id(g(99))),
        ],
        ..World::default()
    };
    let r = check(&w, CONSISTENCY_GUID_TO_IMMUTABLE_ID);
    assert_eq!(
        statuses(&r),
        [AdStatus::MissingSourceAnchor, AdStatus::Unresolved]
    );
    let e = &r.out.entra;
    assert_ne!(e[0] & flag::E_ANCHOR_ABSENT, 0);
    assert_ne!(e[1] & flag::E_ANCHOR_FAULT, 0);
    assert_ne!(e[2] & flag::E_ANCHOR_FAULT, 0);
    assert_ne!(e[3] & flag::E_ORPHAN, 0);
    assert_eq!(e[1] & flag::E_ANCHOR_ABSENT, 0, "malformed is not absent");
    use ogar_dir_core::correspond::state;
    assert_eq!(
        r.lanes.entra_anchor.state,
        [state::ABSENT, state::MALFORMED, state::NIL, state::PRESENT]
    );
}

/// F13: matching UPN and mail without an id witness relates nothing.
#[test]
fn f13_soft_match_is_not_a_hard_match() {
    let w = World {
        ad: vec![ad(g(1), Wire::Absent, Wire::Absent)],
        entra: vec![Entra {
            upn: Some("anna@example.org"),
            ..entra(g(3), Wire::Absent)
        }],
        ..World::default()
    };
    let r = check(&w, CONSISTENCY_GUID_TO_IMMUTABLE_ID);
    assert_eq!(statuses(&r), [AdStatus::MissingSourceAnchor]);
    assert!(r.out.edges.is_empty());
}

/// F14: the fold leaves every observation byte-identical.
#[test]
fn f14_observations_are_not_touched() {
    let (a, s, x) = (g(1), g(2), g(3));
    let w = World {
        ad: vec![ad(a, Wire::Id(s), Wire::Id(x))],
        entra: vec![entra(x, Wire::Id(s))],
        exo: vec![Exo {
            guid: g(40),
            tenant: g(TENANT),
            external: Wire::Id(x),
        }],
    };
    let e = encode(&w);
    let before: Vec<[u8; 512]> =
        e.ad.iter()
            .chain(&e.entra)
            .chain(&e.exo)
            .map(|r| *r.as_bytes())
            .collect();
    let (ap, ep) = (
        e.ad_pool.as_bytes().to_vec(),
        e.entra_pool.as_bytes().to_vec(),
    );
    let r = run_encoded(&e, CONSISTENCY_GUID_TO_IMMUTABLE_ID, &bindings());
    let after: Vec<[u8; 512]> =
        e.ad.iter()
            .chain(&e.entra)
            .chain(&e.exo)
            .map(|r| *r.as_bytes())
            .collect();
    assert_eq!(before, after);
    assert_eq!(ap, e.ad_pool.as_bytes());
    assert_eq!(ep, e.entra_pool.as_bytes());
    // The cloud witness, Entra → Exchange Online.
    assert_eq!(
        edges(&r, EdgeEvidence::EntraObjectIdMatchesExchangeExternalId),
        [(x, g(40))]
    );
    assert_ne!(r.out.entra[0] & flag::E_EXO_ONE, 0);
    assert_ne!(r.out.exo[0] & flag::X_ONE, 0);
}

#[test]
fn exchange_online_dangling_and_duplicate() {
    let x = g(3);
    let exo = |n, ext| Exo {
        guid: g(n),
        tenant: g(TENANT),
        external: ext,
    };
    let w = World {
        entra: vec![entra(x, Wire::Absent)],
        exo: vec![
            exo(40, Wire::Id(x)),
            exo(41, Wire::Id(x)),
            exo(42, Wire::Id(g(77))),
            exo(43, Wire::Absent),
        ],
        ..World::default()
    };
    let r = run(&w, CONSISTENCY_GUID_TO_IMMUTABLE_ID);
    assert_ne!(r.out.entra[0] & flag::E_EXO_MANY, 0);
    assert_eq!(
        r.out.exo,
        [flag::X_ONE, flag::X_ONE, flag::X_DANGLING, flag::X_ABSENT]
    );
}

// ---- randomized comparison --------------------------------------------------

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// Small id spaces so collisions, duplicates, splits and contradictions all
/// occur; two domains, one bound.
fn random_world(rng: &mut Rng) -> World {
    let pick = |rng: &mut Rng, space: u64| match rng.below(10) {
        0..=1 => Wire::Absent,
        _ => Wire::Id(g(100 + rng.below(space))),
    };
    let entra_ids: Vec<Guid128> = (0..rng.below(8)).map(|i| g(500 + i)).collect();
    let n_ad = rng.below(8);
    let ad_rows = (0..n_ad)
        .map(|i| {
            let back = match rng.below(4) {
                0 => Wire::Absent,
                1 => Wire::Id(g(500 + rng.below(10))),
                _ if !entra_ids.is_empty() => {
                    Wire::Id(entra_ids[rng.below(entra_ids.len() as u64) as usize])
                }
                _ => Wire::Absent,
            };
            Ad {
                guid: g(if rng.below(5) == 0 {
                    100 + rng.below(6)
                } else {
                    1_000 + i
                }),
                domain: g(if rng.below(6) == 0 {
                    OTHER_DOMAIN
                } else {
                    DOMAIN
                }),
                cg: pick(rng, 6),
                backsync: back,
                group: false,
                at: [0, 1_000, 2_000, 900_000][rng.below(4) as usize],
            }
        })
        .collect();
    let entra_rows = entra_ids
        .iter()
        .map(|&id| Entra {
            id,
            tenant: g(if rng.below(6) == 0 {
                OTHER_TENANT
            } else {
                TENANT
            }),
            immutable: match rng.below(12) {
                0 => Wire::Malformed,
                _ => pick(rng, 6),
            },
            upn: None,
            at: [0, 1_000, 30_000][rng.below(3) as usize],
        })
        .collect();
    World {
        ad: ad_rows,
        entra: entra_rows,
        exo: vec![],
    }
}

#[test]
fn randomized_fold_matches_the_reference() {
    let mut rng = Rng(0x5EED_1D3A_7172_0001);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..3_000 {
        let w = random_world(&mut rng);
        for rule in [
            CONSISTENCY_GUID_TO_IMMUTABLE_ID,
            OBJECT_GUID_TO_IMMUTABLE_ID,
        ] {
            let r = check(&w, rule);
            seen.extend(statuses(&r));
        }
    }
    // Anti-vacuity: the generator reached every outcome the fold can name.
    for s in [
        AdStatus::OutOfScope,
        AdStatus::AmbiguousSource,
        AdStatus::AmbiguousTarget,
        AdStatus::Contradictory,
        AdStatus::Confirmed,
        AdStatus::AgreesAcrossTime,
        AdStatus::ForwardOnly,
        AdStatus::BacksyncOnly,
        AdStatus::MissingSourceAnchor,
        AdStatus::Unresolved,
    ] {
        assert!(seen.contains(&s), "never generated {s:?}");
    }
}
