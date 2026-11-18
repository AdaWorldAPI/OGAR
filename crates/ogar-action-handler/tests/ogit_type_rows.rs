//! Per-type row ranges (`TypeRows`), counted with Quack's `count_by`, against a
//! plain loop over the lanes, and the validator that reads them.
#![cfg(feature = "quack")]

#[allow(dead_code)]
#[path = "../../ogar-from-schema/src/ogit_checkout.rs"]
mod ogit_checkout;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use lance_graph_mask_risc::{
    Foreign, ForeignPlane as ForeignMask, LaneRef, Out, Planes, Scratch, execute_compiled,
    execute_into, words_for,
};
use lance_graph_quack::bind::{BindError, Binder, Op, table};
use lance_graph_quack::lower;
use ogar_action_handler::ogit::{
    AttrId, Flag, Lane, OgitTables, Table, TypeId, TypeRows, TypeRowsError, Validator,
    ValidatorError, Violation,
};

fn tables() -> &'static OgitTables {
    static TABLES: OnceLock<OgitTables> = OnceLock::new();
    TABLES
        .get_or_init(|| OgitTables::load(&ogit_checkout::root()).expect("the OGIT checkout loads"))
}

const COMMIT_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const COMMIT_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// A small OGIT tree on disk (the MARS domain of the checkout) whose
/// `.git/HEAD` holds a given commit, or that has no `.git` at all.
struct Subset(PathBuf);

impl Subset {
    fn new(label: &str, head: Option<&str>) -> Self {
        let root =
            std::env::temp_dir().join(format!("ogar-type-rows-{}-{label}", std::process::id()));
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

/// The per-type tables and their key lane.
const PER_TYPE: [(Table, Lane, &str, &str); 2] = [
    (Table::TypeAttr, Lane::TypeAttrType, "type_attr", "type"),
    (Table::Allowed, Lane::AllowedSrc, "allowed", "src"),
];

/// `count_by(key)` run by hand through the typed front end: the numbers
/// `TypeRows` is built from, without `TypeRows`.
fn count_by(t: &OgitTables, name: &str, key: &str) -> Vec<i64> {
    let binder = t.binder();
    let typed = table(name).count_by(key).bind_typed(&binder).unwrap();
    let q = typed.admit(&binder).unwrap();
    let program = lower(q).unwrap();
    let tb = binder.table(name).unwrap();
    let mut lanes = Vec::new();
    while let Some(l) = binder.lane(tb, lance_graph_quack::Col(lanes.len() as u16)) {
        lanes.push(LaneRef::U32(l));
    }
    let rows = lanes
        .first()
        .map(|l| match l {
            LaneRef::U32(s) => s.len(),
            _ => unreachable!(),
        })
        .unwrap();
    let mut live = vec![u64::MAX; words_for(rows)];
    if !rows.is_multiple_of(64) {
        *live.last_mut().unwrap() = (1u64 << (rows % 64)) - 1;
    }
    let masks: [&[u64]; 1] = [&live];
    let planes = Planes {
        n_rows: rows,
        masks: &masks,
        lanes: &lanes,
    };
    let mut counts = vec![0i64; t.rows(Table::Type)];
    let mut scratch = Scratch::for_program(&program, rows).unwrap();
    execute_into(
        &program,
        &planes,
        &Foreign::NONE,
        &mut scratch,
        Out::I64(&mut counts),
    )
    .unwrap();
    counts
}

/// The scalar reading: one pass over the key lane.
fn scalar_counts(t: &OgitTables, lane: Lane) -> Vec<i64> {
    let mut counts = vec![0i64; t.rows(Table::Type)];
    for &k in t.lane(lane) {
        counts[k as usize] += 1;
    }
    counts
}

fn check_counts(t: &OgitTables) {
    for (which, lane, name, key) in PER_TYPE {
        let scalar = scalar_counts(t, lane);
        assert_eq!(count_by(t, name, key), scalar, "{which:?} count_by");
        let rows = TypeRows::new(t, which).unwrap();
        let lens: Vec<i64> = (0..t.rows(Table::Type))
            .map(|i| {
                let r = rows.range_unchecked(TypeId(u16::try_from(i).unwrap()));
                i64::try_from(r.len()).unwrap()
            })
            .collect();
        assert_eq!(lens, scalar, "{which:?} range lengths");
        assert_eq!(scalar.iter().sum::<i64>() as usize, t.rows(which));
    }
}

#[test]
fn count_by_equals_a_scalar_count_on_the_real_tree() {
    let t = tables();
    check_counts(t);
    // Not vacuous: many types, some with no rows, and a type with several.
    let scalar = scalar_counts(t, Lane::TypeAttrType);
    assert!(scalar.len() >= 200);
    assert!(scalar.contains(&0));
    assert!(scalar.iter().any(|&c| c > 5));
}

#[test]
fn count_by_equals_a_scalar_count_on_a_small_fixture() {
    let sub = Subset::new("counts", Some(COMMIT_A));
    let t = sub.load();
    assert!(t.rows(Table::Type) > 3 && t.rows(Table::Type) < 100);
    check_counts(&t);
}

fn check_partition(t: &OgitTables) {
    for (which, lane, _, _) in PER_TYPE {
        let rows = TypeRows::new(t, which).unwrap();
        let keys = t.lane(lane);
        assert_eq!(rows.types(), t.rows(Table::Type));
        let mut next = 0;
        for i in 0..rows.types() {
            let ty = TypeId(u16::try_from(i).unwrap());
            let r = rows.range(t, ty).unwrap();
            assert_eq!(
                r.start, next,
                "{which:?} type {i} starts where the last ended"
            );
            assert!(r.start <= r.end);
            for row in r.clone() {
                assert_eq!(keys[row], u32::try_from(i).unwrap(), "{which:?} row {row}");
            }
            next = r.end;
        }
        assert_eq!(next, t.rows(which), "{which:?}: the ranges cover every row");
        // A type past the table has no rows.
        assert_eq!(rows.range_unchecked(TypeId(u16::MAX)), 0..0);
    }
}

#[test]
fn the_ranges_partition_the_table_exactly() {
    check_partition(tables());
    check_partition(&Subset::new("partition", Some(COMMIT_A)).load());
}

#[test]
fn the_real_tree_is_sorted_by_type_and_src() {
    // The load order the build asserts: it holds on the real tree.
    let t = tables();
    for (_, lane, _, _) in PER_TYPE {
        let keys = t.lane(lane);
        assert!(keys.windows(2).all(|w| w[0] <= w[1]));
    }
    assert!(t.type_rows(Table::TypeAttr).is_ok());
    assert_eq!(
        t.type_rows(Table::Verb).unwrap_err(),
        TypeRowsError::NotPerType(Table::Verb)
    );
}

// ------------------------------------------------- the validator, range vs scan

/// The rows `v`'s program selects for `ty` and `present` over the WHOLE table,
/// through a fresh evaluation that knows nothing of the ranges.
fn full_scan(
    t: &OgitTables,
    v: &Validator,
    scratch: &mut Scratch<'_>,
    ty: TypeId,
    present: &[AttrId],
) -> Vec<u64> {
    let (n_types, n_attrs, n_rows) = (
        t.rows(Table::Type),
        t.rows(Table::Attribute),
        t.rows(Table::TypeAttr),
    );
    let mut type_plane = vec![0u64; words_for(n_types)];
    type_plane[usize::from(ty.0) / 64] |= 1 << (ty.0 % 64);
    let mut present_plane = vec![0u64; words_for(n_attrs)];
    for a in present {
        present_plane[usize::from(a.0) / 64] |= 1 << (a.0 % 64);
    }
    let foreign = [
        ForeignMask {
            words: &type_plane,
            rows: n_types,
        },
        ForeignMask {
            words: &present_plane,
            rows: n_attrs,
        },
    ];
    let [l0, l1, l2] = v.lanes(t);
    let lanes = [LaneRef::U32(l0), LaneRef::U32(l1), LaneRef::U32(l2)];
    let planes = Planes {
        n_rows,
        masks: &[],
        lanes: &lanes,
    };
    let foreign = Foreign {
        planes: &foreign,
        lanes: &[],
    };
    let mut out = vec![0u64; words_for(n_rows)];
    execute_compiled(
        &v.program().compiled(),
        &planes,
        &foreign,
        scratch,
        Out::Mask(&mut out),
        0..n_rows,
    )
    .unwrap();
    out
}

fn mandatory_of(t: &OgitTables, ty: TypeId) -> Vec<AttrId> {
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

fn check_parity(t: &OgitTables) -> usize {
    let mut v = Validator::new(t).unwrap();
    let mut scratch = Scratch::for_program(v.program().program(), t.rows(Table::TypeAttr)).unwrap();
    let n = t.rows(Table::Type);
    // Forward, then backward, so a result left by one type would show in the
    // next one's mask.
    let order: Vec<usize> = (0..n).chain((0..n).rev()).collect();
    let mut checked = 0;
    for i in order {
        let ty = TypeId(u16::try_from(i).unwrap());
        let mandatory = mandatory_of(t, ty);
        let half: Vec<AttrId> = mandatory.iter().copied().step_by(2).collect();
        for present in [&[][..], &half[..], &mandatory[..]] {
            let want = full_scan(t, &v, &mut scratch, ty, present);
            let got = v.missing_mask(t, ty, present).unwrap().to_vec();
            assert_eq!(got, want, "type {i} present {present:?}");
            checked += 1;
        }
    }
    checked
}

#[test]
fn the_range_run_equals_the_full_scan_for_every_type() {
    let t = tables();
    let checked = check_parity(t);
    assert!(checked >= 3 * 2 * 700, "{checked}");
    check_parity(&Subset::new("parity", Some(COMMIT_A)).load());
}

/// The written violations of every type, range-run, against the mask of the
/// full scan read row by row: what `validate` reports is what the whole-table
/// program says.
#[test]
fn validate_reports_what_the_full_scan_selects() {
    let t = tables();
    let mut v = Validator::new(t).unwrap();
    let mut scratch = Scratch::for_program(v.program().program(), t.rows(Table::TypeAttr)).unwrap();
    let attrs = t.lane(Lane::TypeAttrAttr);
    let mut with_violations = 0;
    for i in 0..t.rows(Table::Type) {
        let ty = TypeId(u16::try_from(i).unwrap());
        let name = t.type_name(ty).unwrap();
        // The empty write misses every mandatory attribute.
        let got = v.validate(t, name, &[]).unwrap();
        let mask = full_scan(t, &v, &mut scratch, ty, &[]);
        let want: Vec<Violation<'_>> = (0..attrs.len())
            .filter(|&r| mask[r / 64] >> (r % 64) & 1 == 1)
            .map(|r| Violation::MissingMandatoryAttribute(AttrId(u16::try_from(attrs[r]).unwrap())))
            .collect();
        assert_eq!(got, want, "{name}");
        with_violations += usize::from(!want.is_empty());
    }
    assert!(with_violations > 100);
}

// ------------------------------------------------------------------- refusals

#[test]
fn a_range_over_a_coded_ogit_field_is_unordered() {
    let t = tables();
    let binder = t.binder();
    for op in [Op::Lt, Op::Le, Op::Gt, Op::Ge] {
        for (name, field, literal) in [
            ("type_attr", "type", "ogit:Person"),
            ("type_attr", "flag", "optional"),
            ("allowed", "src", "ogit:Person"),
            ("type", "domain", ""),
        ] {
            assert_eq!(
                table(name).where_cmp(field, op, literal).bind(&binder),
                Err(BindError::Unordered {
                    field: field.into()
                }),
                "{name}.{field} {op:?}"
            );
        }
    }
    // An integer literal for a coded field is a kind mismatch, not a range.
    assert_eq!(
        table("type_attr")
            .where_cmp("type", Op::Ge, 5)
            .bind(&binder),
        Err(BindError::KindMismatch {
            field: "type".into()
        })
    );
    // `=` on the same field binds.
    assert!(
        table("type_attr")
            .where_eq("type", "ogit:Person")
            .bind(&binder)
            .is_ok()
    );
}

/// `GroupKey` needs a nullable or integer key. Every OGIT field is a coded
/// lane with no validity plane, so no OGIT draft can reach it: every field of
/// every table groups, and an unknown one is an unknown field.
#[test]
fn every_ogit_field_is_a_legal_group_key_so_group_key_is_not_reachable() {
    let t = tables();
    let binder = t.binder();
    for (name, fields) in [
        ("type", &["domain"][..]),
        ("attribute", &["domain"]),
        ("type_attr", &["type", "attr", "flag"]),
        ("verb", &["domain"]),
        ("allowed", &["src", "verb", "dst"]),
    ] {
        for f in fields {
            assert!(
                table(name).count_by(f).bind_typed(&binder).is_ok(),
                "{name}.{f}"
            );
        }
    }
    assert_eq!(
        table("type_attr").count_by("nope").bind(&binder),
        Err(BindError::UnknownField {
            table: "type_attr".into(),
            field: "nope".into()
        })
    );
}

// ------------------------------------------------------------- stale witnesses

#[test]
fn ranges_built_under_one_tree_are_not_used_against_another() {
    let a_tree = Subset::new("wa", Some(COMMIT_A));
    let a_again_tree = Subset::new("wa2", Some(COMMIT_A));
    let b_tree = Subset::new("wb", Some(COMMIT_B));
    let (a, a_again, b) = (a_tree.load(), a_again_tree.load(), b_tree.load());
    // Same content, same sizes: only the commit differs.
    assert_eq!(a.codes_digest(), b.codes_digest());
    assert_eq!(a.rows(Table::TypeAttr), b.rows(Table::TypeAttr));
    let rows = TypeRows::new(&a, Table::TypeAttr).unwrap();
    let ty = a.type_id("ogit.MARS:Machine").unwrap();
    let want = rows.range(&a, ty).unwrap();
    assert!(!want.is_empty());
    // The same commit, loaded again, is the same tree.
    assert_eq!(rows.range(&a_again, ty).unwrap(), want);
    assert_eq!(rows.range(&b, ty), Err(TypeRowsError::StaleTables));
    assert_eq!(rows.witness(), a.witness().unwrap());
    // The full checkout is another tree again.
    assert_eq!(rows.range(tables(), ty), Err(TypeRowsError::StaleTables));
    // The other direction.
    let rb = TypeRows::new(&b, Table::TypeAttr).unwrap();
    assert_eq!(rb.range(&a, ty), Err(TypeRowsError::StaleTables));
}

#[test]
fn an_uncommitted_edit_that_renumbers_or_resizes_is_refused() {
    let tree = Subset::new("edit", Some(COMMIT_A));
    let before = tree.load();
    let rows = TypeRows::new(&before, Table::TypeAttr).unwrap();
    let ty = before.type_id("ogit.MARS:Machine").unwrap();
    assert!(rows.range(&tree.load(), ty).is_ok(), "control");

    // One more `type_attr` row, same names: same witness, other row count.
    let path = tree.0.join("NTO/MARS/entities/Machine.ttl");
    let ttl = fs::read_to_string(&path).unwrap();
    let edited = ttl.replacen(
        "ogit:optional-attributes (",
        "ogit:optional-attributes (\n            ogit.MARS.Machine:class",
        1,
    );
    assert_ne!(ttl, edited);
    fs::write(&path, edited).unwrap();
    let resized = tree.load();
    assert_eq!(before.codes_digest(), resized.codes_digest());
    assert_eq!(rows.range(&resized, ty), Err(TypeRowsError::StaleTables));

    // A new type that sorts first renumbers every id under an unchanged commit.
    fs::write(&path, &ttl).unwrap();
    let dir = tree.0.join("NTO/MARS/entities");
    fs::write(
        dir.join("Aaa.ttl"),
        ttl.replace("ogit.MARS:Machine", "ogit.MARS:Aaa"),
    )
    .unwrap();
    let renumbered = tree.load();
    assert_eq!(before.tree(), renumbered.tree());
    assert_ne!(before.codes_digest(), renumbered.codes_digest());
    assert_eq!(rows.range(&renumbered, ty), Err(TypeRowsError::StaleTables));
}

#[test]
fn a_tree_with_no_commit_cannot_be_vouched_for() {
    let anon = Subset::new("anon", None).load();
    assert_eq!(
        TypeRows::new(&anon, Table::TypeAttr).unwrap_err(),
        TypeRowsError::Unwitnessed
    );
    let a = Subset::new("anon-a", Some(COMMIT_A)).load();
    let rows = TypeRows::new(&a, Table::TypeAttr).unwrap();
    let ty = a.type_id("ogit.MARS:Machine").unwrap();
    assert_eq!(rows.range(&anon, ty), Err(TypeRowsError::StaleTables));
    assert_eq!(
        Validator::new(&anon).unwrap_err(),
        ValidatorError::Unwitnessed
    );
}

// ------------------------------------------- moved rows under an unchanged witness

/// A minimal MARS entity file for `curie` with the given mandatory attributes
/// (an absent list when there are none).
fn entity_ttl(curie: &str, mandatory: &[&str]) -> String {
    let list = if mandatory.is_empty() {
        String::new()
    } else {
        format!(
            "\togit:mandatory-attributes (\n{}\t);\n",
            mandatory
                .iter()
                .map(|a| format!("\t    {a}\n"))
                .collect::<String>()
        )
    };
    format!(
        "@prefix ogit.MARS: <http://www.purl.org/ogit/MARS/> .\n\
         @prefix ogit.MARS.Machine: <http://www.purl.org/ogit/MARS/Machine/> .\n\
         @prefix ogit: <http://www.purl.org/ogit/> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\n\
         {curie}\n\ta rdfs:Class;\n\trdfs:subClassOf ogit:Entity;\n\
         \trdfs:label \"x\";\n\togit:scope \"NTO\";\n\togit:parent ogit:Node;\n{list}.\n"
    )
}

/// Removes the first line of `file` (inside `tree`) that is `attr`, as an
/// uncommitted edit would.
fn drop_line(tree: &Subset, file: &str, attr: &str) {
    let path = tree.0.join("NTO/MARS/entities").join(file);
    let ttl = fs::read_to_string(&path).unwrap();
    let mut gone = false;
    let edited: String = ttl
        .lines()
        .filter(|l| {
            let hit = !gone && l.trim() == attr;
            gone |= hit;
            !hit
        })
        .map(|l| format!("{l}\n"))
        .collect();
    assert!(gone, "{attr} is in {file}");
    fs::write(path, edited).unwrap();
}

const CLASS: &str = "ogit.MARS.Machine:class";

/// The tables, witness and sizes as `a` and `b` see them: all equal.
fn assert_same_code_space(a: &OgitTables, b: &OgitTables) {
    assert_eq!(a.tree(), b.tree(), "same commit");
    assert_eq!(a.codes_digest(), b.codes_digest(), "same name-to-id digest");
    for table in [
        Table::Type,
        Table::Attribute,
        Table::TypeAttr,
        Table::Allowed,
    ] {
        assert_eq!(a.rows(table), b.rows(table), "{table:?} row count");
    }
    assert!(a.witness().is_some() && a.witness() == b.witness());
}

/// The review finding: a mandatory attribute moves from `Machine` to the
/// previously empty `Machine2`, and the commit, the name-to-id digest and the
/// row counts all stay as they were while the per-type ranges change.
#[test]
fn a_mandatory_attribute_moved_to_an_empty_type_is_not_answered_from_stale_ranges() {
    let tree = Subset::new("moved-empty", Some(COMMIT_A));
    let dir = tree.0.join("NTO/MARS/entities");
    fs::write(
        dir.join("Machine2.ttl"),
        entity_ttl("ogit.MARS:Machine2", &[]),
    )
    .unwrap();
    let before = tree.load();
    let machine = before.type_id("ogit.MARS:Machine").unwrap();
    let machine2 = before.type_id("ogit.MARS:Machine2").unwrap();
    let class = before.attribute_id(CLASS).unwrap();

    let rows = TypeRows::new(&before, Table::TypeAttr).unwrap();
    let mut old = Validator::new(&before).unwrap();
    assert!(
        rows.range(&before, machine2).unwrap().is_empty(),
        "B is empty"
    );
    assert!(rows.range(&before, machine).unwrap().len() > 2);
    assert!(
        old.validate(&before, "ogit.MARS:Machine2", &[])
            .unwrap()
            .is_empty()
    );

    // The uncommitted edit: `class` is now Machine2's mandatory attribute.
    drop_line(&tree, "Machine.ttl", CLASS);
    fs::write(
        dir.join("Machine2.ttl"),
        entity_ttl("ogit.MARS:Machine2", &[CLASS]),
    )
    .unwrap();
    let after = tree.load();
    assert_same_code_space(&before, &after);
    assert_eq!(mandatory_of(&after, machine2), vec![class]);
    assert!(
        !mandatory_of(&before, machine).is_empty()
            && !mandatory_of(&after, machine).contains(&class)
    );
    assert_ne!(
        before.lane(Lane::TypeAttrType),
        after.lane(Lane::TypeAttrType),
        "the layout did change"
    );

    // (i) the ranges are refused for both types the move touches.
    assert_eq!(
        rows.range(&after, machine2),
        Err(TypeRowsError::StaleTables)
    );
    assert_eq!(rows.range(&after, machine), Err(TypeRowsError::StaleTables));
    // (ii) the old validator does not accept a Machine2 write without `class`
    // against the new tables: it refuses them as stale.
    assert_eq!(
        old.validate(&after, "ogit.MARS:Machine2", &[]),
        Err(ValidatorError::StaleTables)
    );
    assert_eq!(
        old.missing_mask(&after, machine2, &[]).map(<[u64]>::to_vec),
        Err(ValidatorError::StaleTables)
    );
    // The tables it was made from are still served.
    assert!(
        old.validate(&before, "ogit.MARS:Machine2", &[])
            .unwrap()
            .is_empty()
    );
    // (iii) a validator rebuilt from the new tables reports the attribute.
    let mut fresh = Validator::new(&after).unwrap();
    assert_eq!(
        fresh.validate(&after, "ogit.MARS:Machine2", &[]).unwrap(),
        vec![Violation::MissingMandatoryAttribute(class)]
    );
    assert!(
        fresh
            .validate(&after, "ogit.MARS:Machine2", &[CLASS])
            .unwrap()
            .is_empty()
    );
    // And the whole-table scan agrees with the rebuilt validator.
    let mut scratch =
        Scratch::for_program(fresh.program().program(), after.rows(Table::TypeAttr)).unwrap();
    let mask = full_scan(&after, &fresh, &mut scratch, machine2, &[]);
    assert_eq!(mask.iter().map(|w| w.count_ones()).sum::<u32>(), 1);
    check_parity(&after);
}

/// The same move between two types that both have rows: the counts of both
/// change by one, the total does not.
#[test]
fn a_mandatory_attribute_moved_between_two_non_empty_types_is_refused_too() {
    let tree = Subset::new("moved-swap", Some(COMMIT_A));
    let before = tree.load();
    let machine = before.type_id("ogit.MARS:Machine").unwrap();
    let resource = before.type_id("ogit.MARS:Resource").unwrap();
    let class = before.attribute_id(CLASS).unwrap();
    let rows = TypeRows::new(&before, Table::TypeAttr).unwrap();
    let mut old = Validator::new(&before).unwrap();
    assert!(!rows.range(&before, resource).unwrap().is_empty());

    // `class` leaves Machine's mandatory list and joins Resource's.
    drop_line(&tree, "Machine.ttl", CLASS);
    let path = tree.0.join("NTO/MARS/entities/Resource.ttl");
    let ttl = fs::read_to_string(&path).unwrap();
    let edited = ttl.replacen(
        "ogit:mandatory-attributes (",
        &format!("ogit:mandatory-attributes (\n\t    {CLASS}"),
        1,
    );
    assert_ne!(ttl, edited);
    fs::write(&path, edited).unwrap();
    let after = tree.load();
    assert_same_code_space(&before, &after);
    assert!(mandatory_of(&after, resource).contains(&class));
    assert!(!mandatory_of(&after, machine).contains(&class));

    assert_eq!(rows.range(&after, machine), Err(TypeRowsError::StaleTables));
    assert_eq!(
        rows.range(&after, resource),
        Err(TypeRowsError::StaleTables)
    );
    // A type the move did not touch is still answered.
    let software = before.type_id("ogit.MARS:Software").unwrap();
    assert_eq!(
        rows.range(&after, software),
        Ok(rows.range_unchecked(software))
    );
    for ty in ["ogit.MARS:Machine", "ogit.MARS:Resource"] {
        assert_eq!(
            old.validate(&after, ty, &[]),
            Err(ValidatorError::StaleTables)
        );
    }
    let mut fresh = Validator::new(&after).unwrap();
    let want = fresh.validate(&after, "ogit.MARS:Resource", &[]).unwrap();
    assert!(want.contains(&Violation::MissingMandatoryAttribute(class)));
    check_parity(&after);
}

/// Tables loaded again from an unchanged tree are accepted by every range and
/// by the validator made from the first load.
#[test]
fn unchanged_tables_are_still_accepted() {
    let tree = Subset::new("unchanged", Some(COMMIT_A));
    let a = tree.load();
    let again = tree.load();
    let rows = TypeRows::new(&a, Table::TypeAttr).unwrap();
    let mut v = Validator::new(&a).unwrap();
    for i in 0..a.rows(Table::Type) {
        let ty = TypeId(u16::try_from(i).unwrap());
        assert_eq!(rows.range(&again, ty), rows.range(&a, ty));
        assert!(rows.range(&again, ty).is_ok());
        assert_eq!(
            v.validate(&again, a.type_name(ty).unwrap(), &[]),
            v.validate(&a, a.type_name(ty).unwrap(), &[])
        );
    }
    for (which, _, _, _) in PER_TYPE {
        let rows = TypeRows::new(&a, which).unwrap();
        for i in 0..a.rows(Table::Type) {
            assert!(
                rows.range(&again, TypeId(u16::try_from(i).unwrap()))
                    .is_ok()
            );
        }
    }
    check_parity(&again);
}
