//! The Quack binder over the OGIT tables: bound queries against a row-loop
//! oracle over the same lanes.
#![cfg(feature = "quack")]

#[allow(dead_code)]
#[path = "../../ogar-from-schema/src/ogit_checkout.rs"]
mod ogit_checkout;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use lance_graph_mask_risc::{
    Foreign, LaneRef, Out, Planes, Scratch, Value, execute_into, materialize_rows, words_for,
};
use lance_graph_quack::bind::{BindError, Binder, CodeWitness, Draft, TableId, table};
use lance_graph_quack::{Agg, Col, Query, lower};
use ogar_action_handler::ogit::{AttrId, Flag, Lane, OgitBinder, OgitTables, Table};
use ogar_from_schema::TtlDeclaration;
use ogar_from_schema::ttl::parse_file;

fn tables() -> &'static OgitTables {
    static TABLES: OnceLock<OgitTables> = OnceLock::new();
    TABLES
        .get_or_init(|| OgitTables::load(&ogit_checkout::root()).expect("the OGIT checkout loads"))
}

/// Executes `q` over the table `name` on the lanes the binder lends, with an
/// all-ones live plane at plane 0. Returns the aggregate and, for a rows
/// query, the matching row numbers.
fn run(t: &OgitTables, name: &str, q: &Query) -> (Value, Vec<usize>) {
    let b = t.binder();
    let id = b.table(name).expect("a table");
    let n = t.rows(match name {
        "type" => Table::Type,
        "attribute" => Table::Attribute,
        "type_attr" => Table::TypeAttr,
        "verb" => Table::Verb,
        _ => Table::Allowed,
    });
    let lanes: Vec<LaneRef<'_>> = (0..)
        .map_while(|c| b.lane(id, Col(c)))
        .map(LaneRef::U32)
        .collect();
    let mut live = vec![u64::MAX; words_for(n)];
    if !n.is_multiple_of(64) {
        *live.last_mut().unwrap() = (1u64 << (n % 64)) - 1;
    }
    let masks: [&[u64]; 1] = [&live];
    let planes = Planes {
        n_rows: n,
        masks: &masks,
        lanes: &lanes,
    };
    let program = lower(q).expect("the bound query lowers");
    let mut scratch = Scratch::for_program(&program, n).unwrap();
    let mut bits = vec![0u64; words_for(n)];
    let out = if q.agg == Agg::Rows {
        Out::Mask(&mut bits)
    } else {
        Out::None
    };
    let v = execute_into(&program, &planes, &Foreign::NONE, &mut scratch, out).unwrap();
    (v, materialize_rows(&bits, n))
}

/// The rows of `type_attr` the plain loop keeps for `(type, flag)`.
fn oracle_type_attr(t: &OgitTables, ty: &str, flag: Flag) -> Vec<usize> {
    let want = u32::from(t.type_id(ty).unwrap().0);
    let (tys, flags) = (t.lane(Lane::TypeAttrType), t.lane(Lane::TypeAttrFlag));
    (0..tys.len())
        .filter(|&i| tys[i] == want && flags[i] == flag as u32)
        .collect()
}

fn machine_mandatory() -> Draft {
    table("type_attr")
        .where_eq("type", "ogit.MARS:Machine")
        .where_eq("flag", "mandatory")
}

#[test]
fn mandatory_attributes_of_a_type_equal_the_oracle_and_the_ttl() {
    let t = tables();
    let b = t.binder();
    let count = machine_mandatory().count().bind(&b).unwrap();
    let rows = machine_mandatory().rows().bind(&b).unwrap();
    let oracle = oracle_type_attr(t, "ogit.MARS:Machine", Flag::Mandatory);
    assert!(!oracle.is_empty(), "Machine declares mandatory attributes");
    assert_eq!(run(t, "type_attr", &count).0, Value::Count(oracle.len()));
    assert_eq!(run(t, "type_attr", &rows).1, oracle);

    // The same rows, read as names, are the TTL's list in its order.
    let Some(TtlDeclaration::Entity(e)) =
        parse_file(&ogit_checkout::read("NTO/MARS/entities/Machine.ttl"))
    else {
        panic!("Machine.ttl does not read as an entity");
    };
    let expected: Vec<&str> = e
        .mandatory_attributes
        .iter()
        .map(String::as_str)
        .filter(|a| t.attribute_id(a).is_some())
        .collect();
    let attrs = t.lane(Lane::TypeAttrAttr);
    let names: Vec<&str> = run(t, "type_attr", &rows)
        .1
        .into_iter()
        .map(|i| {
            t.attribute_name(AttrId(u16::try_from(attrs[i]).unwrap()))
                .unwrap()
        })
        .collect();
    assert_eq!(names, expected);

    // No string reaches the bound query.
    let text = format!("{rows:?}");
    assert!(
        !text.contains("Machine") && !text.contains("mandatory"),
        "{text}"
    );
}

#[test]
fn allowed_edges_from_application_over_depends_on_equal_the_oracle() {
    let t = tables();
    let b = t.binder();
    let draft = table("allowed")
        .where_eq("src", "ogit.MARS:Application")
        .where_eq("verb", "ogit:dependsOn");
    let rows = draft.clone().rows().bind(&b).unwrap();
    let count = draft.count().bind(&b).unwrap();

    let src = u32::from(t.type_id("ogit.MARS:Application").unwrap().0);
    let verb = u32::from(t.verb_id("ogit:dependsOn").unwrap().0);
    let (srcs, verbs, dsts) = (
        t.lane(Lane::AllowedSrc),
        t.lane(Lane::AllowedVerb),
        t.lane(Lane::AllowedDst),
    );
    let oracle: Vec<usize> = (0..srcs.len())
        .filter(|&i| srcs[i] == src && verbs[i] == verb)
        .collect();
    assert_eq!(run(t, "allowed", &rows).1, oracle);
    assert_eq!(run(t, "allowed", &count).0, Value::Count(oracle.len()));

    let resource = u32::from(t.type_id("ogit.MARS:Resource").unwrap().0);
    assert!(
        oracle.iter().any(|&i| dsts[i] == resource),
        "(Application, dependsOn, Resource) is an allowed edge"
    );
}

#[test]
fn flag_and_domain_codes_bind_over_the_other_tables() {
    let t = tables();
    let b = t.binder();
    let q = table("type")
        .where_eq("domain", "MARS")
        .rows()
        .bind(&b)
        .unwrap();
    let mars = t.domain_id("MARS").unwrap();
    let oracle: Vec<usize> = (0..t.rows(Table::Type))
        .filter(|&i| t.lane(Lane::TypeDomain)[i] == mars)
        .collect();
    assert!(!oracle.is_empty());
    assert_eq!(run(t, "type", &q).1, oracle);
    for (name, flag) in [
        ("mandatory", Flag::Mandatory),
        ("optional", Flag::Optional),
        ("indexed", Flag::Indexed),
    ] {
        let q = table("type_attr")
            .where_eq("flag", name)
            .count()
            .bind(&b)
            .unwrap();
        let n = t
            .lane(Lane::TypeAttrFlag)
            .iter()
            .filter(|&&f| f == flag as u32)
            .count();
        assert_eq!(run(t, "type_attr", &q).0, Value::Count(n), "{name}");
    }
}

#[test]
fn an_unknown_table_field_or_name_fails_closed_with_its_error() {
    let t = tables();
    let b = t.binder();
    let err = |d: Draft| d.bind(&b).unwrap_err();
    assert_eq!(err(table("types")), BindError::UnknownTable("types".into()));
    assert_eq!(
        err(table("type_attr").where_eq("domain", "MARS")),
        BindError::UnknownField {
            table: "type_attr".into(),
            field: "domain".into()
        }
    );
    for (tbl, field, value) in [
        ("type_attr", "type", "ogit.MARS:NoSuchType"),
        ("type_attr", "attr", "ogit:noSuchAttribute"),
        ("type_attr", "flag", "Mandatory"),
        ("allowed", "verb", "ogit:noSuchVerb"),
        ("allowed", "dst", "ogit.Person"),
        ("type", "domain", "NoSuchDomain"),
    ] {
        assert_eq!(
            err(table(tbl).where_eq(field, value)),
            BindError::UnknownValue {
                field: field.into(),
                value: value.into()
            },
            "{tbl}.{field} = {value}"
        );
    }
    // A number is not a name.
    assert_eq!(
        err(table("allowed").where_eq("src", 3)),
        BindError::KindMismatch {
            field: "src".into()
        }
    );
}

#[test]
fn the_binder_lends_the_tables_own_lanes() {
    let t = tables();
    let b = t.binder();
    for (name, lanes) in [
        ("type", &[Lane::TypeDomain][..]),
        ("attribute", &[Lane::AttributeDomain]),
        (
            "type_attr",
            &[Lane::TypeAttrType, Lane::TypeAttrAttr, Lane::TypeAttrFlag],
        ),
        ("verb", &[Lane::VerbDomain]),
        (
            "allowed",
            &[Lane::AllowedSrc, Lane::AllowedVerb, Lane::AllowedDst],
        ),
    ] {
        let id = b.table(name).unwrap();
        for (c, &lane) in lanes.iter().enumerate() {
            let lent = b.lane(id, Col(u16::try_from(c).unwrap())).unwrap();
            assert!(
                std::ptr::eq(lent, t.lane(lane)),
                "{name} column {c} is a copy"
            );
        }
        assert!(
            b.lane(id, Col(u16::try_from(lanes.len()).unwrap()))
                .is_none()
        );
    }
    assert!(b.lane(TableId(5), Col(0)).is_none());
}

/// A small OGIT tree on disk (the MARS domain of the checkout) whose
/// `.git/HEAD` holds a given commit, or that has no `.git` at all.
struct Subset(PathBuf);

impl Subset {
    fn new(label: &str, head: Option<&str>) -> Self {
        let root = std::env::temp_dir().join(format!("ogar-binder-{}-{label}", std::process::id()));
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
fn a_typed_query_is_admitted_on_its_own_tree_and_refused_on_another() {
    let a = Subset::new("a", Some(COMMIT_A));
    let a_again = Subset::new("a2", Some(COMMIT_A));
    let b = Subset::new("b", Some(COMMIT_B));
    let (a, a_again, b) = (a.load(), a_again.load(), b.load());
    assert_ne!(a.tree(), b.tree());

    let typed = machine_mandatory().bind_typed(&a.binder()).unwrap();
    let admitted = typed.admit(&a.binder()).expect("its own tree");
    assert_eq!(admitted, &machine_mandatory().bind(&a.binder()).unwrap());
    // Another load of the same commit is the same tree.
    assert!(typed.admit(&a_again.binder()).is_ok());
    // Same content, another commit: the ids mean nothing there.
    assert_eq!(
        typed.admit(&b.binder()).err(),
        Some(BindError::StaleCodeSpace)
    );
}

#[test]
fn a_tree_with_no_commit_cannot_be_vouched_for() {
    let anon = Subset::new("anon", None).load();
    let a = Subset::new("anon-a", Some(COMMIT_A)).load();
    assert_eq!(anon.tree().commit(), None);

    // Plain bind does not look at the tree; a typed bind refuses.
    assert!(machine_mandatory().bind(&anon.binder()).is_ok());
    assert!(matches!(
        machine_mandatory().bind_typed(&anon.binder()),
        Err(BindError::Unwitnessed { .. })
    ));
    // A witness is never admitted against a tree with no commit.
    let typed = machine_mandatory().bind_typed(&a.binder()).unwrap();
    assert_eq!(
        typed.admit(&anon.binder()).err(),
        Some(BindError::StaleCodeSpace)
    );
    // And a held witness without a commit is admitted nowhere.
    let b = anon.binder();
    let id = b.table("type_attr").unwrap();
    let held = anon.tree().clone();
    assert_eq!(
        b.admit_code(id, Col(0), &held),
        Err(BindError::StaleCodeSpace)
    );
    assert_eq!(
        a.binder().admit_code(id, Col(0), &held),
        Err(BindError::StaleCodeSpace)
    );
}

#[test]
fn the_checkout_binds_typed_when_its_commit_is_known() {
    let t = tables();
    let b: OgitBinder<'_> = t.binder();
    let typed = machine_mandatory().bind_typed(&b);
    if t.tree().commit().is_some() {
        let typed = typed.unwrap();
        assert!(typed.admit(&b).is_ok());
        // Against another tree's tables: refused.
        let other = Subset::new("other", Some(COMMIT_B)).load();
        assert_ne!(other.tree(), t.tree());
        assert_eq!(
            typed.admit(&other.binder()).err(),
            Some(BindError::StaleCodeSpace)
        );
    } else {
        assert!(matches!(typed, Err(BindError::Unwitnessed { .. })));
    }
}
