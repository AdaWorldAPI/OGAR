//! Q4 over the OGIT tables: `OgitEdges::allowed` against a linear-scan oracle
//! over the same lanes, on the OGIT checkout. No feature is needed.

#[allow(dead_code)]
#[path = "../../ogar-from-schema/src/ogit_checkout.rs"]
mod ogit_checkout;

use std::alloc::{GlobalAlloc, Layout, System};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use ogar_action_handler::ogit::{EdgeError, Lane, OgitEdges, OgitTables, Table, TypeId, VerbId};

// Counts every allocation call and byte of this binary. The accepting-path
// test reads the counters around its queries; every test holds `GUARD`, so no
// other thread allocates in between.
struct Counting;

static CALLS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

// SAFETY: forwards every call to `System`; the counters are atomics.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        CALLS.fetch_add(1, Ordering::SeqCst);
        BYTES.fetch_add(l.size(), Ordering::SeqCst);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        CALLS.fetch_add(1, Ordering::SeqCst);
        BYTES.fetch_add(n, Ordering::SeqCst);
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

static GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn tables() -> &'static OgitTables {
    static TABLES: OnceLock<OgitTables> = OnceLock::new();
    TABLES
        .get_or_init(|| OgitTables::load(&ogit_checkout::root()).expect("the OGIT checkout loads"))
}

fn tid(i: u32) -> TypeId {
    TypeId(u16::try_from(i).unwrap())
}

fn vid(i: u32) -> VerbId {
    VerbId(u16::try_from(i).unwrap())
}

fn lanes(t: &OgitTables) -> (&[u32], &[u32], &[u32]) {
    (
        t.lane(Lane::AllowedSrc),
        t.lane(Lane::AllowedVerb),
        t.lane(Lane::AllowedDst),
    )
}

/// The oracle: a linear scan over the lanes, with no order assumed.
fn naive(t: &OgitTables, s: u32, v: u32, d: u32) -> bool {
    let (a, b, c) = lanes(t);
    (0..a.len()).any(|i| a[i] == s && b[i] == v && c[i] == d)
}

/// A fixed-seed xorshift, so the sample is the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> u32 {
        u32::try_from(self.next() % u64::try_from(n).unwrap()).unwrap()
    }
}

#[test]
fn the_allowed_rows_are_strictly_sorted_by_src_verb_dst() {
    let _g = serial();
    let (a, b, c) = lanes(tables());
    assert!(!a.is_empty(), "the checkout has allowed edges");
    assert!(a.len() == b.len() && a.len() == c.len());
    let key = |i: usize| (a[i], b[i], c[i]);
    if let Some(i) = (1..a.len()).find(|&i| key(i - 1) >= key(i)) {
        panic!(
            "the sortedness OgitEdges relies on broke at row {i}: {:?} then {:?}",
            key(i - 1),
            key(i)
        );
    }
    assert!(tables().edges().is_ok());
}

#[test]
fn every_allowed_row_is_allowed() {
    let _g = serial();
    let t = tables();
    let e = t.edges().unwrap();
    let (a, b, c) = lanes(t);
    assert_eq!(e.len(), a.len());
    for i in 0..a.len() {
        assert!(e.allowed(tid(a[i]), vid(b[i]), tid(c[i])), "row {i}");
    }
}

#[test]
fn non_rows_equal_the_linear_scan() {
    let _g = serial();
    let t = tables();
    let e = t.edges().unwrap();
    let (a, b, c) = lanes(t);
    let (types, verbs) = (t.rows(Table::Type), t.rows(Table::Verb));
    let mut checked = 0usize;
    let mut yes = 0usize;
    let mut check = |s: u32, v: u32, d: u32| {
        let got = e.allowed(tid(s), vid(v), tid(d));
        assert_eq!(got, naive(t, s, v, d), "({s}, {v}, {d})");
        checked += 1;
        yes += usize::from(got);
    };
    // Every row with one lane moved, and reversed.
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for i in 0..a.len() {
        check(a[i], b[i], rng.below(types));
        check(rng.below(types), b[i], c[i]);
        check(a[i], rng.below(verbs), c[i]);
        check(c[i], b[i], a[i]);
    }
    // Random triples.
    for _ in 0..20_000 {
        check(rng.below(types), rng.below(verbs), rng.below(types));
    }
    assert!(checked >= 20_000 + 4 * a.len());
    assert!(yes > 0 && yes < checked, "the sample has both answers");
}

#[test]
fn ids_outside_every_table_are_not_edges() {
    let _g = serial();
    let e = tables().edges().unwrap();
    assert!(!e.allowed(TypeId(u16::MAX), VerbId(u16::MAX), TypeId(u16::MAX)));
    assert!(!e.allowed(TypeId(0), VerbId(u16::MAX), TypeId(0)));
}

#[test]
fn names_resolve_and_unknown_names_are_refused() {
    let _g = serial();
    let t = tables();
    let e = t.edges().unwrap();
    assert_eq!(
        e.allowed_names(
            "ogit.MARS:Application",
            "ogit:dependsOn",
            "ogit.MARS:Resource"
        ),
        Ok(true),
        "(Application, dependsOn, Resource) is an allowed edge"
    );
    // A known triple that is not a row is Ok(false), not an error.
    let (s, v) = (
        t.type_id("ogit.MARS:Application").unwrap(),
        t.verb_id("ogit:dependsOn").unwrap(),
    );
    let not_row = (0..t.rows(Table::Type))
        .map(|d| tid(u32::try_from(d).unwrap()))
        .find(|&d| !e.allowed(s, v, d))
        .unwrap();
    let d_name = t.type_name(not_row).unwrap();
    assert_eq!(
        e.allowed_names("ogit.MARS:Application", "ogit:dependsOn", d_name),
        Ok(false)
    );
    for (args, role, name) in [
        (
            ("ogit.MARS:NoSuch", "ogit:dependsOn", "ogit:Person"),
            "src",
            "ogit.MARS:NoSuch",
        ),
        (
            ("ogit:Person", "ogit:noSuchVerb", "ogit:Person"),
            "verb",
            "ogit:noSuchVerb",
        ),
        (
            ("ogit:Person", "ogit:dependsOn", "ogit.Person"),
            "dst",
            "ogit.Person",
        ),
    ] {
        assert_eq!(
            e.allowed_names(args.0, args.1, args.2),
            Err(EdgeError::UnknownName {
                role,
                name: name.into()
            })
        );
    }
}

#[test]
fn the_lanes_are_lent_not_copied() {
    let _g = serial();
    let t = tables();
    let e = t.edges().unwrap();
    let (a, b, c) = e.lanes();
    assert!(std::ptr::eq(a, t.lane(Lane::AllowedSrc)));
    assert!(std::ptr::eq(b, t.lane(Lane::AllowedVerb)));
    assert!(std::ptr::eq(c, t.lane(Lane::AllowedDst)));
}

#[test]
fn the_accepting_path_allocates_nothing() {
    let _g = serial();
    let t = tables();
    let e = t.edges().unwrap();
    let held = e.witness();
    let (a, b, c) = lanes(t);
    // Everything a query needs is made before the counters are read.
    let rows: Vec<(TypeId, VerbId, TypeId)> = [0, a.len() / 2, a.len() - 1]
        .into_iter()
        .map(|i| (tid(a[i]), vid(b[i]), tid(c[i])))
        .collect();
    let miss = (tid(c[0]), vid(b[0]), tid(a[0]));
    let want_miss = naive(t, c[0], b[0], a[0]);
    let names = (
        "ogit.MARS:Application",
        "ogit:dependsOn",
        "ogit.MARS:Resource",
    );

    let (calls0, bytes0) = (CALLS.load(Ordering::SeqCst), BYTES.load(Ordering::SeqCst));
    let mut accepted = 0;
    for &(s, v, d) in &rows {
        accepted += usize::from(e.allowed(s, v, d));
        accepted += usize::from(e.allowed_with(&held, s, v, d) == Ok(true));
    }
    accepted += usize::from(e.allowed_names(names.0, names.1, names.2) == Ok(true));
    let missed = e.allowed(miss.0, miss.1, miss.2);
    let (calls, bytes) = (
        CALLS.load(Ordering::SeqCst) - calls0,
        BYTES.load(Ordering::SeqCst) - bytes0,
    );

    assert_eq!(accepted, 7, "every query was accepted");
    assert_eq!(missed, want_miss);
    assert_eq!((calls, bytes), (0, 0), "allocation calls and bytes");
}

// ---------------------------------------------------------------- one tree

struct Subset(PathBuf);

impl Subset {
    fn new(label: &str, head: Option<&str>) -> Self {
        let root = std::env::temp_dir().join(format!("ogar-edges-{}-{label}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        copy_dir(
            &ogit_checkout::root().join("NTO/MARS"),
            &root.join("NTO/MARS"),
        );
        copy_dir(&ogit_checkout::root().join("SGO"), &root.join("SGO"));
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
fn a_witness_of_another_tree_is_refused() {
    let _g = serial();
    let a = Subset::new("a", Some(COMMIT_A));
    let a2 = Subset::new("a2", Some(COMMIT_A));
    let b = Subset::new("b", Some(COMMIT_B));
    let (a, a2, b) = (a.load(), a2.load(), b.load());
    let (ea, ea2, eb) = (a.edges().unwrap(), a2.edges().unwrap(), b.edges().unwrap());
    let (s, v, d) = {
        let (x, y, z) = lanes(&a);
        (tid(x[0]), vid(y[0]), tid(z[0]))
    };
    let held = ea.witness();
    assert_eq!(ea.allowed_with(&held, s, v, d), Ok(true));
    // Another load of the same commit is the same code space.
    assert_eq!(ea2.allowed_with(&held, s, v, d), Ok(true));
    // Same content, another commit: the ids mean nothing there.
    assert_eq!(
        eb.allowed_with(&held, s, v, d),
        Err(EdgeError::StaleCodeSpace)
    );
}

#[test]
fn an_uncommitted_edit_that_renumbers_ids_is_refused() {
    let _g = serial();
    let tree = Subset::new("renumber", Some(COMMIT_A));
    let before = tree.load();
    let held = before.edges().unwrap().witness();
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
        after
            .edges()
            .unwrap()
            .allowed_with(&held, tid(0), vid(0), tid(0)),
        Err(EdgeError::StaleCodeSpace)
    );
}

#[test]
fn a_tree_with_no_commit_cannot_be_vouched_for() {
    let _g = serial();
    let anon = Subset::new("anon", None).load();
    assert_eq!(anon.tree().commit(), None);
    assert_eq!(anon.edges().err(), Some(EdgeError::Unwitnessed));
    assert_eq!(OgitEdges::new(&anon).err(), Some(EdgeError::Unwitnessed));
    // A held witness is admitted nowhere without a commit.
    let a = Subset::new("anon-a", Some(COMMIT_A)).load();
    let held = a.edges().unwrap().witness();
    assert!(!held.admits(&anon));
}
