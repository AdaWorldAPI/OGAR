//! The structural validator over the OGIT tables, against a plain row loop over
//! the same lanes and against the TTL files.
#![cfg(feature = "quack")]

#[allow(dead_code)]
#[path = "../../ogar-from-schema/src/ogit_checkout.rs"]
mod ogit_checkout;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use lance_graph_mask_risc::{
    Foreign, ForeignPlane as ForeignMask, LaneRef, Out, Planes, Scratch, execute_compiled,
    reference_execute_into, words_for,
};
use ogar_action_handler::ogit::{
    AttrId, Flag, Lane, OgitTables, SYSTEM_KEYS, TypeId, Validator, ValidatorError, Violation,
    is_system_key,
};
use ogar_from_schema::TtlDeclaration;
use ogar_from_schema::ttl::parse_file;

// Counts the allocations of the thread that reads it, so tests that run side
// by side do not see each other's.
struct Counting;

thread_local! {
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

// SAFETY: forwards every call to `System`; the counter is a plain `Cell`.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.with(|c| c.set(c.get() + 1));
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.with(|c| c.set(c.get() + 1));
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn allocs() -> usize {
    ALLOCS.with(Cell::get)
}

fn tables() -> &'static OgitTables {
    static TABLES: OnceLock<OgitTables> = OnceLock::new();
    TABLES
        .get_or_init(|| OgitTables::load(&ogit_checkout::root()).expect("the OGIT checkout loads"))
}

fn validator() -> Validator {
    Validator::new(tables()).expect("the checkout has a readable commit")
}

fn attr(t: &OgitTables, curie: &str) -> AttrId {
    t.attribute_id(curie).expect("a declared attribute")
}

// ---------------------------------------------------------------- the oracle

/// The JanusGraph key of every attribute, derived from the TTL files' own
/// `@prefix` lines with a port of `OntologyLoader.uriToJgKey`
/// (`OntologyLoader.java:318-333`), not through the validator.
fn oracle_keys(t: &OgitTables) -> Vec<Option<String>> {
    let prefixes = prefixes();
    (0..t.rows(Table::Attribute))
        .map(|i| {
            let curie = t.attribute_name(AttrId(u16::try_from(i).unwrap())).unwrap();
            let (p, local) = curie.split_once(':').unwrap();
            let uris = prefixes.get(p)?;
            assert_eq!(uris.len(), 1, "prefix {p} names more than one URI");
            Some(uri_to_jg_key(&format!(
                "{}{local}",
                uris.iter().next().unwrap()
            )))
        })
        .collect()
}

use ogar_action_handler::ogit::Table;

/// `OntologyLoader.uriToJgKey`: drop scheme and host, `#` becomes `/`.
fn uri_to_jg_key(uri: &str) -> String {
    let mut s = uri;
    if let Some(i) = s.find("://") {
        s = &s[i + 3..];
    }
    if let Some(i) = s.find('/') {
        s = &s[i + 1..];
    }
    s.replace('#', "/")
}

fn ttl_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n != ".git") {
                ttl_files(&p, out);
            }
        } else if p.extension().is_some_and(|x| x == "ttl") {
            out.push(p);
        }
    }
}

/// Every `@prefix name: <uri> .` of the checkout, by name.
fn prefixes() -> BTreeMap<String, BTreeSet<String>> {
    let root = ogit_checkout::root();
    let mut files = Vec::new();
    for d in ["NTO", "SGO"] {
        ttl_files(&root.join(d), &mut files);
    }
    let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for f in files {
        let Ok(text) = fs::read_to_string(&f) else {
            continue;
        };
        for line in text.lines() {
            let Some(rest) = line.trim_start().strip_prefix("@prefix") else {
                continue;
            };
            let mut parts = rest.split_whitespace();
            let (Some(name), Some(uri)) = (parts.next(), parts.next()) else {
                continue;
            };
            let name = name.trim_end_matches(':');
            let uri = uri.trim_start_matches('<').trim_end_matches('>');
            map.entry(name.to_owned())
                .or_default()
                .insert(uri.to_owned());
        }
    }
    map
}

/// The mandatory attributes of `ty`, by a plain loop over the lanes, in row
/// (TTL list) order.
fn oracle_mandatory(t: &OgitTables, ty: TypeId) -> Vec<AttrId> {
    let (tys, attrs, flags) = (
        t.lane(Lane::TypeAttrType),
        t.lane(Lane::TypeAttrAttr),
        t.lane(Lane::TypeAttrFlag),
    );
    (0..tys.len())
        .filter(|&r| tys[r] == u32::from(ty.0) && flags[r] == Flag::Mandatory as u32)
        .map(|r| AttrId(u16::try_from(attrs[r]).unwrap()))
        .collect()
}

/// What the Java validator and the plan's Q1-Q3 say, by plain loops: the type
/// by its CURIE, the keys by the TTL-derived JanusGraph keys.
fn oracle_validate<'k>(
    t: &OgitTables,
    jg: &[Option<String>],
    ty: &'k str,
    entries: &[(&'k str, bool)],
) -> Vec<Violation<'k>> {
    let Some(ty_id) = t.type_id(ty) else {
        return vec![Violation::UnknownType(ty)];
    };
    let id_of = |key: &str| jg.iter().position(|k| k.as_deref() == Some(key));
    let mut out = Vec::new();
    for a in oracle_mandatory(t, ty_id) {
        let key = jg[usize::from(a.0)].as_deref().unwrap();
        let present = entries.iter().any(|&(k, non_blank)| k == key && non_blank);
        if !present {
            out.push(Violation::MissingMandatoryAttribute(a));
        }
    }
    for &(key, _) in entries {
        if !is_system_key(key) && id_of(key).is_none() {
            out.push(Violation::UnknownProperty(key));
        }
    }
    out
}

fn machine() -> (&'static str, Vec<String>) {
    let Some(TtlDeclaration::Entity(e)) =
        parse_file(&ogit_checkout::read("NTO/MARS/entities/Machine.ttl"))
    else {
        panic!("Machine.ttl does not read as an entity");
    };
    ("ogit.MARS:Machine", e.mandatory_attributes)
}

/// A JanusGraph key of a declared attribute, by the validator.
fn jg(v: &Validator, t: &OgitTables, curie: &str) -> String {
    v.key(attr(t, curie)).unwrap().to_owned()
}

// ------------------------------------------------------------------- the checks

#[test]
fn a_valid_machine_has_no_violations() {
    let (t, mut v) = (tables(), validator());
    let (ty, mandatory) = machine();
    assert_eq!(mandatory, ["ogit:name", "ogit.MARS.Machine:class"]);
    let mut keys: Vec<String> = mandatory.iter().map(|m| jg(&v, t, m)).collect();
    assert_eq!(keys, ["ogit/name", "ogit/MARS/Machine/class"]);
    keys.push(jg(&v, t, "ogit:vendor"));
    keys.extend(SYSTEM_KEYS.iter().map(|k| (*k).to_owned()));
    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    assert_eq!(v.validate(t, ty, &refs).unwrap(), []);
    // The same write spelled with CURIEs and the full-URI type.
    let curies = ["ogit:name", "ogit.MARS.Machine:class", "ogit/_id"];
    assert_eq!(v.validate(t, ty, &curies).unwrap(), []);
    let uri = "http://www.purl.org/ogit/MARS/Machine";
    assert_eq!(v.validate(t, uri, &refs).unwrap(), []);
    assert_eq!(v.validate(t, "ogit/MARS/Machine", &refs).unwrap(), []);
}

#[test]
fn a_missing_mandatory_attribute_is_the_only_violation() {
    let (t, mut v) = (tables(), validator());
    let (ty, _) = machine();
    let name = attr(t, "ogit:name");
    let class = attr(t, "ogit.MARS.Machine:class");
    let r = v.validate(t, ty, &["ogit/name", "ogit/_id"]).unwrap();
    assert_eq!(r, [Violation::MissingMandatoryAttribute(class)]);
    assert_eq!(r[0].code(), "MISSING_MANDATORY_ATTRIBUTE");
    assert_eq!(v.key(class), Some("ogit/MARS/Machine/class"));
    let r = v.validate(t, ty, &["ogit/MARS/Machine/class"]).unwrap();
    assert_eq!(r, [Violation::MissingMandatoryAttribute(name)]);
    // Both missing: in the TTL list order, the type's first, name before class.
    let r = v.validate(t, ty, &[]).unwrap();
    assert_eq!(
        r,
        [
            Violation::MissingMandatoryAttribute(name),
            Violation::MissingMandatoryAttribute(class)
        ]
    );
    // A blank value is absent, as in Java (`EntityValidator.java:263-265`).
    let r = v
        .validate_entries(
            t,
            ty,
            &[("ogit/name", true), ("ogit/MARS/Machine/class", false)],
        )
        .unwrap();
    assert_eq!(r, [Violation::MissingMandatoryAttribute(class)]);
}

#[test]
fn an_undeclared_key_is_an_unknown_property_and_system_keys_are_not() {
    let (t, mut v) = (tables(), validator());
    let (ty, _) = machine();
    let ok = ["ogit/name", "ogit/MARS/Machine/class"];
    let mut keys = ok.to_vec();
    keys.push("ogit/notDeclaredAnywhere");
    let r = v.validate(t, ty, &keys).unwrap();
    assert_eq!(r, [Violation::UnknownProperty("ogit/notDeclaredAnywhere")]);
    assert_eq!(r[0].code(), "UNKNOWN_PROPERTY");
    // Several, in body order.
    keys.insert(0, "zzz/first");
    keys.push("ogit/_idd");
    let r = v.validate(t, ty, &keys).unwrap();
    assert_eq!(
        r,
        [
            Violation::UnknownProperty("zzz/first"),
            Violation::UnknownProperty("ogit/notDeclaredAnywhere"),
            Violation::UnknownProperty("ogit/_idd"),
        ]
    );
    // The ten system keys are ignored (`OntologyLoader.java:117-128`).
    assert_eq!(SYSTEM_KEYS.len(), 10);
    let mut keys = ok.to_vec();
    keys.extend(SYSTEM_KEYS);
    assert_eq!(v.validate(t, ty, &keys).unwrap(), []);
    // Even in the CURIE-looking spelling of a key that is no attribute.
    assert!(is_system_key("ogit/_type") && !is_system_key("ogit/type"));

    // Java's known set is every declared attribute, not the type's own: an
    // attribute some other type declares is accepted here too.
    let (tys, attrs) = (t.lane(Lane::TypeAttrType), t.lane(Lane::TypeAttrAttr));
    let machine_id = u32::from(t.type_id(ty).unwrap().0);
    let own: BTreeSet<u32> = (0..tys.len())
        .filter(|&r| tys[r] == machine_id)
        .map(|r| attrs[r])
        .collect();
    let foreign = (0..t.rows(Table::Attribute))
        .map(|a| u32::try_from(a).unwrap())
        .find(|a| !own.contains(a))
        .unwrap();
    let foreign_key = v
        .key(AttrId(u16::try_from(foreign).unwrap()))
        .unwrap()
        .to_owned();
    let mut keys = ok.to_vec();
    keys.push(&foreign_key);
    assert_eq!(v.validate(t, ty, &keys).unwrap(), []);
}

#[test]
fn an_unknown_type_is_reported_alone() {
    let (t, mut v) = (tables(), validator());
    let r = v
        .validate(t, "ogit/Nope/Nothing", &["ogit/name", "zzz/unknown"])
        .unwrap();
    assert_eq!(r, [Violation::UnknownType("ogit/Nope/Nothing")]);
    assert_eq!(r[0].code(), "UNKNOWN_TYPE");
    for ty in [
        "",
        "ogit.MARS:",
        "ogit.MARS:machine",
        "MARS:Machine",
        "ogit/",
    ] {
        assert_eq!(
            v.validate(t, ty, &[]).unwrap(),
            [Violation::UnknownType(ty)]
        );
    }
}

#[test]
fn every_spelling_of_a_type_and_an_attribute_resolves_to_the_same_row() {
    let (t, v) = (tables(), validator());
    // The SGO core, in the CURIE, the fixture form and the URI forms.
    let person = t.type_id("ogit:Person").unwrap();
    for s in [
        "ogit:Person",
        "ogit.Person",
        "ogit/Person",
        "http://www.purl.org/ogit/Person",
    ] {
        assert_eq!(v.type_id(s), Some(person), "{s}");
    }
    let auth = t.type_id("ogit.Auth:Account").unwrap();
    for s in [
        "ogit.Auth:Account",
        "ogit/Auth/Account",
        "http://www.purl.org/ogit/Auth/Account",
    ] {
        assert_eq!(v.type_id(s), Some(auth), "{s}");
    }
    let serial = attr(t, "ogit.Auth:fingerprint");
    for s in ["ogit.Auth:fingerprint", "ogit/Auth/fingerprint"] {
        assert_eq!(v.attribute_id(s), Some(serial), "{s}");
    }
    assert_eq!(v.attribute_id("ogit/name"), t.attribute_id("ogit:name"));
}

#[test]
fn the_json_keys_equal_uri_to_jg_key_of_the_ttl_prefixes() {
    let (t, v) = (tables(), validator());
    let oracle = oracle_keys(t);
    let mut covered = 0;
    let mut seen = BTreeSet::new();
    for (i, expected) in oracle.iter().enumerate() {
        let id = AttrId(u16::try_from(i).unwrap());
        let Some(expected) = expected else {
            continue;
        };
        covered += 1;
        assert_eq!(
            v.key(id),
            Some(expected.as_str()),
            "{:?}",
            t.attribute_name(id)
        );
        assert_eq!(v.attribute_id(expected), Some(id));
        assert!(
            seen.insert(expected.clone()),
            "{expected} spells two attributes"
        );
    }
    // Every attribute has an OGIT prefix the checkout declares.
    assert_eq!(covered, t.rows(Table::Attribute));
}

#[test]
fn every_type_accepts_its_mandatory_list_and_names_each_dropped_key() {
    let (t, mut v) = (tables(), validator());
    let oracle = oracle_keys(t);
    let jg_of = |a: AttrId| oracle[usize::from(a.0)].clone().unwrap();
    let n = t.rows(Table::Type);
    assert!(n >= 200);
    let mut dropped = 0;
    for i in 0..n {
        let ty_id = TypeId(u16::try_from(i).unwrap());
        let ty = t.type_name(ty_id).unwrap();
        let mandatory = oracle_mandatory(t, ty_id);
        let keys: Vec<String> = mandatory.iter().map(|&a| jg_of(a)).collect();
        let with_system = |skip: Option<usize>| -> Vec<&str> {
            let mut ks: Vec<&str> = keys
                .iter()
                .enumerate()
                .filter(|&(j, _)| Some(j) != skip)
                .map(|(_, k)| k.as_str())
                .collect();
            // A mandatory key can itself be a system key (`ogit/_id` is
            // `ogit:Node`'s): it is dropped from both lists.
            let dropped = skip.map(|j| keys[j].as_str());
            ks.extend(SYSTEM_KEYS.iter().filter(|&&k| Some(k) != dropped));
            ks
        };
        let all = with_system(None);
        assert_eq!(v.validate(t, ty, &all).unwrap(), [], "{ty}");
        let entries: Vec<(&str, bool)> = all.iter().map(|&k| (k, true)).collect();
        assert_eq!(oracle_validate(t, &oracle, ty, &entries), [], "{ty}");
        for j in 0..keys.len() {
            let ks = with_system(Some(j));
            let got = v.validate(t, ty, &ks).unwrap();
            let entries: Vec<(&str, bool)> = ks.iter().map(|&k| (k, true)).collect();
            assert_eq!(
                got,
                oracle_validate(t, &oracle, ty, &entries),
                "{ty} minus {j}"
            );
            // Exactly that one violation, unless the list names it twice.
            let twice = mandatory.iter().filter(|&&a| a == mandatory[j]).count();
            assert_eq!(got.len(), twice, "{ty} minus {}", keys[j]);
            assert_eq!(got[0], Violation::MissingMandatoryAttribute(mandatory[j]));
            dropped += 1;
        }
        // An unknown key on top of a complete write.
        let mut ks = all.clone();
        ks.push("zzz/unknown");
        assert_eq!(
            v.validate(t, ty, &ks).unwrap(),
            [Violation::UnknownProperty("zzz/unknown")],
            "{ty}"
        );
    }
    assert!(dropped > 1000, "{dropped} drops over {n} types");
}

#[test]
fn the_mandatory_lists_are_the_ttl_lists() {
    let (t, mut v) = (tables(), validator());
    // The TTL file of a few types, read by OGAR's own reader, against the
    // rows the program selects for an empty write.
    for (path, ty) in [
        ("NTO/MARS/entities/Machine.ttl", "ogit.MARS:Machine"),
        ("NTO/MARS/entities/Application.ttl", "ogit.MARS:Application"),
        ("NTO/Auth/entities/Account.ttl", "ogit.Auth:Account"),
    ] {
        let Some(TtlDeclaration::Entity(e)) = parse_file(&ogit_checkout::read(path)) else {
            panic!("{path} does not read as an entity");
        };
        let missing: Vec<AttrId> = v
            .validate(t, ty, &[])
            .unwrap()
            .into_iter()
            .map(|x| match x {
                Violation::MissingMandatoryAttribute(a) => a,
                other => panic!("{other:?}"),
            })
            .collect();
        let names: Vec<&str> = missing
            .iter()
            .map(|&a| t.attribute_name(a).unwrap())
            .collect();
        let expected: Vec<&str> = e
            .mandatory_attributes
            .iter()
            .map(String::as_str)
            .filter(|a| t.attribute_id(a).is_some())
            .collect();
        assert_eq!(names, expected, "{ty}");
    }
}

#[test]
fn the_program_agrees_with_mask_risc_reference_oracle() {
    let (t, mut v) = (tables(), validator());
    let sizes = (
        t.rows(Table::Type),
        t.rows(Table::Attribute),
        t.rows(Table::TypeAttr),
    );
    let mut checked = 0;
    for i in (0..sizes.0).step_by(5) {
        let ty = TypeId(u16::try_from(i).unwrap());
        let mandatory = oracle_mandatory(t, ty);
        // Present: every second mandatory attribute, so some rows are missing.
        let present: Vec<AttrId> = mandatory.iter().copied().step_by(2).collect();
        let got = v.missing_mask(t, ty, &present).unwrap().to_vec();

        // The reference evaluator, over planes this test builds itself.
        let mut type_plane = vec![0u64; words_for(sizes.0)];
        type_plane[i / 64] |= 1 << (i % 64);
        let mut present_plane = vec![0u64; words_for(sizes.1)];
        for a in &present {
            present_plane[usize::from(a.0) / 64] |= 1 << (a.0 % 64);
        }
        let foreign = [
            ForeignMask {
                words: &type_plane,
                rows: sizes.0,
            },
            ForeignMask {
                words: &present_plane,
                rows: sizes.1,
            },
        ];
        let [l0, l1, l2] = v.lanes(t);
        let lanes = [LaneRef::U32(l0), LaneRef::U32(l1), LaneRef::U32(l2)];
        let planes = Planes {
            n_rows: sizes.2,
            masks: &[],
            lanes: &lanes,
        };
        let foreign = Foreign {
            planes: &foreign,
            lanes: &[],
        };
        let mut reference = vec![0u64; words_for(sizes.2)];
        reference_execute_into(
            v.program().program(),
            &planes,
            &foreign,
            Out::Mask(&mut reference),
        )
        .unwrap();
        assert_eq!(got, reference, "type {i}");

        // And the executor's fused path, through a fresh scratch.
        let mut scratch = Scratch::for_program(v.program().program(), sizes.2).unwrap();
        let mut again = vec![0u64; words_for(sizes.2)];
        execute_compiled(
            &v.program().compiled(),
            &planes,
            &foreign,
            &mut scratch,
            Out::Mask(&mut again),
            0..sizes.2,
        )
        .unwrap();
        assert_eq!(again, reference, "type {i}");

        // The row loop.
        let expected: BTreeSet<usize> = (0..sizes.2)
            .filter(|&r| {
                t.lane(Lane::TypeAttrType)[r] == u32::try_from(i).unwrap()
                    && t.lane(Lane::TypeAttrFlag)[r] == Flag::Mandatory as u32
                    && !present
                        .iter()
                        .any(|a| u32::from(a.0) == t.lane(Lane::TypeAttrAttr)[r])
            })
            .collect();
        let rows: BTreeSet<usize> = (0..sizes.2)
            .filter(|&r| got[r / 64] >> (r % 64) & 1 == 1)
            .collect();
        assert_eq!(rows, expected, "type {i}");
        checked += 1;
    }
    assert!(checked > 100);
}

// ------------------------------------------------------------------- the trees

/// A small OGIT tree on disk (the MARS domain of the checkout) whose
/// `.git/HEAD` holds a given commit, or that has no `.git` at all.
struct Subset(PathBuf);

impl Subset {
    fn new(label: &str, head: Option<&str>) -> Self {
        let root =
            std::env::temp_dir().join(format!("ogar-validator-{}-{label}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        copy_dir(
            &ogit_checkout::root().join("NTO/MARS"),
            &root.join("NTO/MARS"),
        );
        fs::create_dir_all(root.join("SGO")).unwrap();
        if let Some(head) = head {
            fs::create_dir_all(root.join(".git")).unwrap();
            fs::write(root.join(".git/HEAD"), format!("{head}\n")).unwrap();
        }
        Self(root)
    }

    fn load(&self) -> OgitTables {
        OgitTables::load(&self.0).expect("the subset loads")
    }
}

impl Drop for Subset {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).unwrap();
        }
    }
}

const COMMIT_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const COMMIT_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn a_validator_refuses_the_tables_of_another_tree() {
    let a_tree = Subset::new("a", Some(COMMIT_A));
    let a_again = Subset::new("a2", Some(COMMIT_A));
    let b_tree = Subset::new("b", Some(COMMIT_B));
    let (a, a_again, b) = (a_tree.load(), a_again.load(), b_tree.load());
    assert_ne!(a.tree(), b.tree());
    let mut v = Validator::new(&a).unwrap();
    // The subset has no SGO core, so `ogit:name` is not declared in it.
    let keys = ["ogit/MARS/Machine/class"];
    let ty = "ogit.MARS:Machine";

    assert_eq!(v.validate(&a, ty, &keys).unwrap(), []);
    // The same commit, loaded again, is the same tree.
    assert_eq!(v.validate(&a_again, ty, &keys).unwrap(), []);
    // Same content, another commit: refused, whatever the write.
    assert_eq!(
        v.validate(&b, ty, &keys).unwrap_err(),
        ValidatorError::StaleTables
    );
    assert_eq!(
        v.validate(&b, "nonsense", &[]).unwrap_err(),
        ValidatorError::StaleTables
    );
    assert_eq!(
        v.missing_mask(&b, TypeId(0), &[]).unwrap_err(),
        ValidatorError::StaleTables
    );
    // The refusal leaves the validator usable on its own tree.
    assert_eq!(v.validate(&a, ty, &keys).unwrap(), []);
    // The other direction too.
    let mut vb = Validator::new(&b).unwrap();
    assert_eq!(
        vb.validate(&a, ty, &keys).unwrap_err(),
        ValidatorError::StaleTables
    );
    // The full checkout's tables are another tree again.
    assert!(v.validate(tables(), ty, &keys).is_err());
}

#[test]
fn an_uncommitted_edit_that_renumbers_ids_is_refused() {
    let tree = Subset::new("renumber", Some(COMMIT_A));
    let before = tree.load();
    let mut v = Validator::new(&before).unwrap();
    let ty = "ogit.MARS:Machine";
    assert!(v.validate(&tree.load(), ty, &[]).is_ok(), "control");

    // An uncommitted entity that sorts before ogit.MARS:Machine shifts its id.
    let dir = tree.0.join("NTO/MARS/entities");
    let machine = fs::read_to_string(dir.join("Machine.ttl")).unwrap();
    fs::write(
        dir.join("Aaa.ttl"),
        machine.replace("ogit.MARS:Machine", "ogit.MARS:Aaa"),
    )
    .unwrap();
    let after = tree.load();
    assert_eq!(before.tree(), after.tree(), "HEAD did not change");
    assert_ne!(before.codes_digest(), after.codes_digest());
    assert_eq!(
        v.validate(&after, ty, &[]).unwrap_err(),
        ValidatorError::StaleTables
    );
}

#[test]
fn a_tree_with_no_commit_cannot_be_vouched_for() {
    let anon = Subset::new("anon", None).load();
    let a = Subset::new("anon-a", Some(COMMIT_A)).load();
    assert_eq!(anon.tree().commit(), None);
    assert_eq!(
        Validator::new(&anon).unwrap_err(),
        ValidatorError::Unwitnessed
    );
    let mut v = Validator::new(&a).unwrap();
    assert_eq!(
        v.validate(&anon, "ogit.MARS:Machine", &[]).unwrap_err(),
        ValidatorError::StaleTables
    );
}

// ------------------------------------------------------------------ zero copy

#[test]
fn the_lanes_are_borrowed_from_the_tables_and_the_planes_are_built_once() {
    let (t, mut v) = (tables(), validator());
    let [ty, attrs, flags] = v.lanes(t);
    assert_eq!(ty.as_ptr(), t.lane(Lane::TypeAttrType).as_ptr());
    assert_eq!(attrs.as_ptr(), t.lane(Lane::TypeAttrAttr).as_ptr());
    assert_eq!(flags.as_ptr(), t.lane(Lane::TypeAttrFlag).as_ptr());
    assert_eq!(ty.len(), t.rows(Table::TypeAttr));

    // Tables loaded again are other memory: the validator lends whichever
    // tables it is given, and keeps no lane of its own.
    let other = OgitTables::load(&ogit_checkout::root()).unwrap();
    assert_eq!(
        v.lanes(&other)[0].as_ptr(),
        other.lane(Lane::TypeAttrType).as_ptr()
    );
    assert_ne!(v.lanes(&other)[0].as_ptr(), ty.as_ptr());

    // The planes and the result mask are allocated once, at construction.
    let before = v.resident_addresses();
    assert!(before.iter().all(|&p| p != 0));
    assert_eq!(
        before.iter().collect::<BTreeSet<_>>().len(),
        3,
        "three buffers"
    );
    let (name, _) = machine();
    for i in 0..1000 {
        let keys: &[&str] = if i % 2 == 0 { &["ogit/name"] } else { &[] };
        v.validate(t, name, keys).unwrap();
    }
    assert_eq!(v.resident_addresses(), before);
}

#[test]
fn a_call_allocates_nothing_when_it_accepts_and_only_its_result_when_it_rejects() {
    let (t, mut v) = (tables(), validator());
    let (ty, _) = machine();
    let ok = [
        "ogit/name",
        "ogit/MARS/Machine/class",
        "ogit/_id",
        "ogit/_type",
    ];
    v.validate(t, ty, &ok).unwrap();
    let before = allocs();
    for _ in 0..1000 {
        assert!(v.validate(t, ty, &ok).unwrap().is_empty());
    }
    assert_eq!(allocs() - before, 0, "accept");

    // The program alone, on the same tables, allocates nothing either way.
    let name = attr(t, "ogit:name");
    let machine_id = t.type_id(ty).unwrap();
    let before = allocs();
    for _ in 0..1000 {
        assert!(
            v.missing_mask(t, machine_id, &[name])
                .unwrap()
                .iter()
                .any(|&w| w != 0)
        );
    }
    assert_eq!(allocs() - before, 0, "program, reject");

    // A reject allocates its result vector: one growth for up to four entries.
    let bad = ["ogit/name", "ogit/MARS/Machine/class", "zzz/a", "zzz/b"];
    let before = allocs();
    for _ in 0..1000 {
        assert_eq!(v.validate(t, ty, &bad).unwrap().len(), 2);
    }
    assert_eq!(allocs() - before, 1000, "one vector per reject");
    let before = allocs();
    for _ in 0..1000 {
        assert_eq!(v.validate(t, "nope", &bad).unwrap().len(), 1);
    }
    assert_eq!(allocs() - before, 1000, "one vector per unknown type");
}
