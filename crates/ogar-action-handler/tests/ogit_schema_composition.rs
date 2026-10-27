//! Lab probe: OGIT's `ogit:allowed` edge whitelist composed as Boolean
//! relations through lance-graph's DR64 relcomp kernel
//! (`dr-math-binding-probe`), checked against naive enumeration.
//!
//! The input is [`OgitTables`]' `Allowed` lanes (`src`, `verb`, `dst`, one row
//! per allowed edge). Two compositions:
//!
//! 1. **Chainable verb pairs.** `targets` (verb × type: verb `v` may end at
//!    type `t`) composed with `sources` (type × verb: an edge with verb `w` may
//!    start at `t`) gives verb × verb: `v ; w` is chainable when some type is a
//!    target of `v` and a source of `w`. This is the schema-level composition
//!    table a DT64 slot registry over OGIT verbs would need.
//! 2. **Typed composition of named verbs.** For selected verbs, the type ×
//!    type relation `R_v`, and `R_v ∘ R_w` and a three-verb chain in both
//!    associations.
//!
//! Every relcomp result is compared pair by pair with an enumeration that
//! shares no code with the kernel. The schema mirror of the inverse-pair
//! candidates (`consistsOf` / `isPartOf`) is measured and printed; nothing is
//! asserted about it, because schema mirroring is not evidence that the
//! instance relations are inverse.
//!
//! The OGIT checkout is found like the other OGIT tests: `OGIT_FORK_PATH`,
//! else `OGIT` next to this repository. A missing checkout fails the test.

#[allow(dead_code)]
#[path = "../../ogar-from-schema/src/ogit_checkout.rs"]
mod ogit_checkout;

use std::collections::BTreeSet;
use std::sync::OnceLock;

use dr_math_binding_probe::{
    Axis, AxisKind, BoundDr64, Focus, FrameRevisions, FrameStore, Guards, MethodSignature, Operand,
    Scope, bind_dr64,
    relcomp::{Chain, RelExpr, Rows},
};
use ogar_action_handler::ogit::{Lane, OgitTables, Table};

const W: usize = 64;
const FOCUS: Focus = Focus { revision: 1 };
const SCOPE: Scope = Scope { id: 1 };
const VERB_AXIS: Axis = Axis {
    class: 1,
    kind: AxisKind::Capability,
};
const TYPE_AXIS: Axis = Axis {
    class: 2,
    kind: AxisKind::Identity,
};

fn tables() -> &'static OgitTables {
    static TABLES: OnceLock<OgitTables> = OnceLock::new();
    TABLES
        .get_or_init(|| OgitTables::load(&ogit_checkout::root()).expect("the OGIT checkout loads"))
}

/// The allowed edges as `(src type, verb, dst type)` row ids.
fn edges(t: &OgitTables) -> Vec<(usize, usize, usize)> {
    let (s, v, d) = (
        t.lane(Lane::AllowedSrc),
        t.lane(Lane::AllowedVerb),
        t.lane(Lane::AllowedDst),
    );
    (0..s.len())
        .map(|i| (s[i] as usize, v[i] as usize, d[i] as usize))
        .collect()
}

/// A Boolean relation stored as 64 × 64 blocks, row-block major.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Blocked {
    rows: usize,
    cols: usize,
    blocks: Vec<Rows>,
}

impl Blocked {
    fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            blocks: vec![[0u64; W]; rows.div_ceil(W) * cols.div_ceil(W)],
        }
    }
    fn col_blocks(&self) -> usize {
        self.cols.div_ceil(W)
    }
    fn set(&mut self, i: usize, j: usize) {
        let b = (i / W) * self.col_blocks() + j / W;
        self.blocks[b][i % W] |= 1u64 << (j % W);
    }
    fn get(&self, i: usize, j: usize) -> bool {
        let b = (i / W) * self.col_blocks() + j / W;
        (self.blocks[b][i % W] >> (j % W)) & 1 == 1
    }
    fn block(&self, bi: usize, bj: usize) -> &Rows {
        &self.blocks[bi * self.col_blocks() + bj]
    }
    fn count(&self) -> usize {
        self.blocks
            .iter()
            .flatten()
            .map(|w| w.count_ones() as usize)
            .sum()
    }
}

/// A relation together with the axes its rows and columns live on.
struct Rel<'a> {
    m: &'a Blocked,
    row: Axis,
    col: Axis,
}

fn store() -> FrameStore {
    FrameStore::new(FrameRevisions {
        source: 1,
        relation_map: 1,
        coordinates: 1,
    })
}

fn bind<'a>(st: &FrameStore, r: &Rel<'_>, ordinal: u16, rows: &'a Rows) -> BoundDr64<'a> {
    let sig = MethodSignature::presence(7, ordinal, r.row, r.col);
    let at = Guards {
        frame: st.seal(),
        focus: FOCUS,
        scope: SCOPE,
    };
    bind_dr64(
        st,
        sig,
        sig,
        &at,
        (FOCUS, SCOPE),
        Operand {
            rows,
            validity: None,
        },
    )
    .expect("a two-valued Distinct operand binds")
}

/// `a ∘ b`; every block product goes through `Chain::eval`.
fn compose(st: &FrameStore, a: &Rel<'_>, b: &Rel<'_>) -> Blocked {
    assert_eq!(a.m.cols, b.m.rows, "inner dimensions");
    let mut out = Blocked::new(a.m.rows, b.m.cols);
    let order = RelExpr::comp(RelExpr::Leaf(0), RelExpr::Leaf(1));
    let n = out.col_blocks();
    for bi in 0..a.m.rows.div_ceil(W) {
        for bk in 0..b.m.cols.div_ceil(W) {
            let mut acc = [0u64; W];
            for bj in 0..a.m.cols.div_ceil(W) {
                let chain = Chain::new(
                    st,
                    vec![
                        bind(st, a, 0, a.m.block(bi, bj)),
                        bind(st, b, 1, b.m.block(bj, bk)),
                    ],
                )
                .expect("domains line up");
                let (rows, _) = chain.eval(&order).expect("order covers the chain");
                for (dst, src) in acc.iter_mut().zip(rows.iter()) {
                    *dst |= src;
                }
            }
            out.blocks[bi * n + bk] = acc;
        }
    }
    out
}

/// `R_v`: type × type edges of one verb.
fn verb_relation(t: &OgitTables, curie: &str) -> Blocked {
    let v = usize::from(
        t.verb_id(curie)
            .unwrap_or_else(|| panic!("{curie} is a verb"))
            .0,
    );
    let n = t.rows(Table::Type);
    let mut r = Blocked::new(n, n);
    for (s, verb, d) in edges(t) {
        if verb == v {
            r.set(s, d);
        }
    }
    r
}

/// Naive composition of two type × type relations, sharing no code with the
/// kernel.
fn naive_compose(a: &Blocked, b: &Blocked) -> BTreeSet<(usize, usize)> {
    let mut out = BTreeSet::new();
    for i in 0..a.rows {
        for j in 0..a.cols {
            if !a.get(i, j) {
                continue;
            }
            for k in 0..b.cols {
                if b.get(j, k) {
                    out.insert((i, k));
                }
            }
        }
    }
    out
}

/// A pair set back as a square matrix.
fn naive_matrix(p: &BTreeSet<(usize, usize)>, n: usize) -> Blocked {
    let mut m = Blocked::new(n, n);
    for &(i, j) in p {
        m.set(i, j);
    }
    m
}

fn pairs(m: &Blocked) -> BTreeSet<(usize, usize)> {
    let mut out = BTreeSet::new();
    for i in 0..m.rows {
        for j in 0..m.cols {
            if m.get(i, j) {
                out.insert((i, j));
            }
        }
    }
    out
}

#[test]
fn chainable_verb_pairs_match_enumeration() {
    let t = tables();
    let (n_types, n_verbs) = (t.rows(Table::Type), t.rows(Table::Verb));
    let e = edges(t);
    assert!(
        n_types > W && n_verbs > W,
        "both axes need block composition"
    );

    let mut targets = Blocked::new(n_verbs, n_types);
    let mut sources = Blocked::new(n_types, n_verbs);
    for &(s, v, d) in &e {
        targets.set(v, d);
        sources.set(s, v);
    }
    let st = store();
    let chainable = compose(
        &st,
        &Rel {
            m: &targets,
            row: VERB_AXIS,
            col: TYPE_AXIS,
        },
        &Rel {
            m: &sources,
            row: TYPE_AXIS,
            col: VERB_AXIS,
        },
    );

    // Enumeration: for each type, every verb that may end there times every
    // verb that may start there.
    let mut into: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n_types];
    let mut out_of: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n_types];
    for &(s, v, d) in &e {
        into[d].insert(v);
        out_of[s].insert(v);
    }
    let mut expected = BTreeSet::new();
    for ty in 0..n_types {
        for &v in &into[ty] {
            for &w in &out_of[ty] {
                expected.insert((v, w));
            }
        }
    }
    assert_eq!(pairs(&chainable), expected);
    assert!(!expected.is_empty());
    let used: BTreeSet<usize> = e.iter().map(|&(_, v, _)| v).collect();
    eprintln!(
        "reader: {} files rejected, {} names declared twice, {} unresolved references",
        t.rejected().len(),
        t.duplicates().len(),
        t.unresolved().len()
    );
    let mut by_table = std::collections::BTreeMap::new();
    for u in t.unresolved() {
        *by_table.entry(format!("{:?}", u.table)).or_insert(0usize) += 1;
    }
    eprintln!("unresolved by table: {by_table:?}");
    let mut rejected_by_kind = std::collections::BTreeMap::new();
    for r in t.rejected() {
        let kind = ["/entities/", "/verbs/", "/attributes/"]
            .into_iter()
            .find(|k| r.contains(k))
            .unwrap_or("other");
        *rejected_by_kind.entry(kind).or_insert(0usize) += 1;
    }
    eprintln!("rejected files by kind: {rejected_by_kind:?}");
    for u in t
        .unresolved()
        .iter()
        .filter(|u| u.table == Table::Verb)
        .take(5)
    {
        eprintln!("unresolved verb: {} in {}", u.name, u.from);
    }
    eprintln!(
        "OGIT tables: {n_types} types, {n_verbs} verbs ({} used in allowed edges), {} allowed edges; \
         {} chainable verb pairs of {} possible among used verbs",
        used.len(),
        e.len(),
        chainable.count(),
        used.len() * used.len()
    );
}

#[test]
fn typed_composition_of_named_verbs_matches_enumeration() {
    let t = tables();
    let st = store();
    let rel = |m| Rel {
        m,
        row: TYPE_AXIS,
        col: TYPE_AXIS,
    };
    let consists = verb_relation(t, "ogit:consistsOf");
    let part_of = verb_relation(t, "ogit:isPartOf");
    let contains = verb_relation(t, "ogit:contains");

    for (name, a, b) in [
        ("consistsOf ; isPartOf", &consists, &part_of),
        ("isPartOf ; contains", &part_of, &contains),
        ("contains ; contains", &contains, &contains),
    ] {
        let comp = compose(&st, &rel(a), &rel(b));
        let expected = naive_compose(a, b);
        assert_eq!(pairs(&comp), expected, "{name}");
        eprintln!("{name}: {} type pairs", expected.len());
    }

    // Three verbs, both associations, through the kernel.
    // consistsOf ; isPartOf ; consistsOf. (consistsOf ; isPartOf ; contains is
    // empty on this schema, which would make the check vacuous.)
    let consists_part = compose(&st, &rel(&consists), &rel(&part_of));
    let part_consists = compose(&st, &rel(&part_of), &rel(&consists));
    let left = compose(&st, &rel(&consists_part), &rel(&consists));
    let right = compose(&st, &rel(&consists), &rel(&part_consists));
    assert_eq!(
        pairs(&left),
        naive_compose(
            &naive_matrix(&naive_compose(&consists, &part_of), consists.rows),
            &consists
        ),
        "the three-verb chain matches enumeration"
    );
    assert_eq!(
        left, right,
        "composition is associative on the schema relations"
    );
    assert_ne!(left.count(), 0, "the three-verb chain is non-empty");
}

#[test]
fn inverse_candidate_mirror_is_measured() {
    let t = tables();
    let consists = pairs(&verb_relation(t, "ogit:consistsOf"));
    let part_of = pairs(&verb_relation(t, "ogit:isPartOf"));
    let mirrored: BTreeSet<(usize, usize)> = consists.iter().map(|&(a, b)| (b, a)).collect();
    let both = mirrored.intersection(&part_of).count();
    assert!(both > 0, "the candidate has some schema support");
    eprintln!(
        "consistsOf: {} type pairs; isPartOf: {}; consistsOf mirrored into isPartOf: {both} \
         ({} consistsOf pairs have no isPartOf mirror, {} isPartOf pairs have no consistsOf mirror)",
        consists.len(),
        part_of.len(),
        mirrored.difference(&part_of).count(),
        part_of.difference(&mirrored).count()
    );
}
