//! KI-1 falsifier: HIRO's three-valued WHEN. Run with `--features ki1`.
//!
//! HIRO, KI-1: ON a direction, WHEN `A AND B` with B UNKNOWN, DO a dry run, and
//! it must never fire ("A typed 3-valued guard must not collapse UNKNOWN to
//! FALSE"; D3: "`A AND B` with A true and B unknown does not fire"). Kleene
//! AND, and a method fires only when the guard is TRUE:
//!
//! | A | B | fires |
//! |---|---|-------|
//! | T | U | no    |
//! | T | F | no    |
//! | T | T | yes   |
//! | U | F | no    |
//! | F | U | no    |
//!
//! Decision B (EXO-294, as codec9 / DR512): a missing value is presence bit 0
//! with value 0. A row is absent if and only if its presence bit is 0; the
//! stored value of an absent row carries no meaning, and a present 0 is a
//! value.
//!
//! The file has two halves. The oracle is green today: a scalar Kleene
//! evaluator over presence words and `i32` lanes. The seam test is red-pinned:
//! no MethodBinding seam evaluates a `Logic::Kleene` guard yet.
#![cfg(feature = "ki1")]

use lance_graph_contract::class_view::{ClassId, ClassView, WideFieldMask};
use lance_graph_contract::ontology::{DisplayTemplate, FieldRef};
use lance_graph_contract::set_algebra::{Cmp, Logic, SetExpr};
use lance_graph_contract::set_rules::{Population, Truth};
use lance_graph_dt64::dr::{Focus, FrameRevisions, FrameStore, Guards, Scope};
use lance_graph_dt64::{
    AlgebraRef, Arg, BindStore, ClassSchema, Effect, Extent, Guard, MethodBinding, MethodId,
    SchemaRev, SigRev, ValueDemand,
};
use lance_graph_r2il::binding::{ExecRefusal, R2ilLens, Resolution, prepare, prepare_gated};
use ogar_r2il::CONCEPT_R2IL_FOLD;

// ---------------------------------------------------------------------------
// Oracle: Kleene AND over presence planes, plain scalar code (decision B).
// ---------------------------------------------------------------------------

/// The three truth values of one predicate on one row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tv {
    T,
    F,
    U,
}

const COMBOS: [(Tv, Tv); 9] = [
    (Tv::T, Tv::T),
    (Tv::T, Tv::F),
    (Tv::T, Tv::U),
    (Tv::F, Tv::T),
    (Tv::F, Tv::F),
    (Tv::F, Tv::U),
    (Tv::U, Tv::T),
    (Tv::U, Tv::F),
    (Tv::U, Tv::U),
];

/// The predicate both operands use: `lane < 5`.
const THRESHOLD: i32 = 5;

/// A presence-tagged `i32` lane: `presence` has bit `i` set when row `i` is
/// present. An absent row's `values[i]` is not read as a value.
struct Lane {
    values: Vec<i32>,
    presence: Vec<u64>,
}

/// The truth planes of one predicate: `known` is presence; `true_` is
/// `present AND value < THRESHOLD`. A row is FALSE when known and not true,
/// and UNKNOWN when not known.
struct Planes {
    known: Vec<u64>,
    true_: Vec<u64>,
}

fn words(n: usize) -> usize {
    n.div_ceil(64)
}

fn bit(plane: &[u64], i: usize) -> bool {
    plane[i / 64] >> (i % 64) & 1 == 1
}

fn compare_lt(lane: &Lane) -> Planes {
    let n = lane.values.len();
    let mut true_ = vec![0u64; words(n)];
    for (i, v) in lane.values.iter().enumerate() {
        // Decision B: absent iff presence bit 0. The value is consulted only
        // when presence says there is one.
        if bit(&lane.presence, i) && *v < THRESHOLD {
            true_[i / 64] |= 1 << (i % 64);
        }
    }
    Planes {
        known: lane.presence.clone(),
        true_,
    }
}

/// Kleene AND: the planes `(true, false)` of `a AND b`. Unknown is neither.
fn kleene_and(a: &Planes, b: &Planes) -> (Vec<u64>, Vec<u64>) {
    let t = a.true_.iter().zip(&b.true_).map(|(x, y)| x & y).collect();
    let f = (0..a.known.len())
        .map(|w| (a.known[w] & !a.true_[w]) | (b.known[w] & !b.true_[w]))
        .collect();
    (t, f)
}

/// The rows a method guarded by `a AND b` fires on: the TRUE plane only.
fn fire(a: &Lane, b: &Lane) -> Vec<u64> {
    let (t, _f) = kleene_and(&compare_lt(a), &compare_lt(b));
    t
}

/// One row's lane value for a truth value: T present below the threshold, F
/// present at or above it, U absent. Absent rows alternate between value 0
/// (the stored form of a missing value) and a nonzero value that would be
/// TRUE if it were read: presence alone decides.
fn cell(tv: Tv, row: usize) -> (i32, bool) {
    match tv {
        Tv::T => (if row.is_multiple_of(2) { 0 } else { 1 }, true),
        Tv::F => (9, true),
        Tv::U => (if row.is_multiple_of(2) { 0 } else { 3 }, false),
    }
}

fn lanes_for(n: usize) -> (Lane, Lane) {
    let mut a = Lane {
        values: vec![0; n],
        presence: vec![0; words(n)],
    };
    let mut b = Lane {
        values: vec![0; n],
        presence: vec![0; words(n)],
    };
    for i in 0..n {
        let (ta, tb) = COMBOS[i % COMBOS.len()];
        for (lane, tv) in [(&mut a, ta), (&mut b, tb)] {
            let (v, present) = cell(tv, i);
            lane.values[i] = v;
            if present {
                lane.presence[i / 64] |= 1 << (i % 64);
            }
        }
    }
    (a, b)
}

/// The acceptance table, at widths that cross the word boundary.
#[test]
fn kleene_and_over_presence_planes_fires_only_when_true() {
    for n in [1usize, 63, 64, 65, 1000] {
        let (a, b) = lanes_for(n);
        let fired = fire(&a, &b);
        let mut count = 0usize;
        for i in 0..n {
            let (ta, tb) = COMBOS[i % COMBOS.len()];
            let expect = ta == Tv::T && tb == Tv::T;
            assert_eq!(
                bit(&fired, i),
                expect,
                "width {n} row {i}: A={ta:?} B={tb:?} must fire={expect}"
            );
            count += usize::from(expect);
        }
        // Tail bits past the last row are never set.
        let set: u32 = fired.iter().map(|w| w.count_ones()).sum();
        assert_eq!(set as usize, count, "width {n}: stray bits");
    }
}

/// The five rows of the KI-1 acceptance table, one row each, by name.
#[test]
fn ki1_acceptance_rows() {
    let rows = [
        (Tv::T, Tv::U, false),
        (Tv::T, Tv::F, false),
        (Tv::T, Tv::T, true),
        (Tv::U, Tv::F, false),
        (Tv::F, Tv::U, false),
    ];
    for (ta, tb, expect) in rows {
        let one = |tv| {
            let (v, p) = cell(tv, 0);
            Lane {
                values: vec![v],
                presence: vec![u64::from(p)],
            }
        };
        assert_eq!(
            bit(&fire(&one(ta), &one(tb)), 0),
            expect,
            "A={ta:?} B={tb:?}"
        );
    }
}

/// Decision B exactly: a present 0 is a value (and `0 < 5` is TRUE); an absent
/// row with a nonzero stored value is not read (it would be TRUE if it were);
/// an absent row is UNKNOWN, not FALSE, so it neither fires nor counts as a
/// definite false.
#[test]
fn absent_iff_presence_bit_is_zero() {
    let lane = Lane {
        // row 0: present 0, row 1: absent 0, row 2: absent 3, row 3: present 9
        values: vec![0, 0, 3, 9],
        presence: vec![0b1001],
    };
    let p = compare_lt(&lane);
    assert_eq!(p.true_, vec![0b0001], "only the present 0 is TRUE");
    assert_eq!(p.known, vec![0b1001]);
    let all_true = Planes {
        known: vec![0b1111],
        true_: vec![0b1111],
    };
    let (t, f) = kleene_and(&all_true, &p);
    assert_eq!(t, vec![0b0001]);
    // FALSE only where a known operand is false: row 3. Absent rows 1 and 2
    // are UNKNOWN (neither plane).
    assert_eq!(f, vec![0b1000]);
}

// ---------------------------------------------------------------------------
// The seam: r2il's MethodBinding preparation, red-pinned.
// ---------------------------------------------------------------------------

const CLASS: u32 = 0x7F0B_0001;
const CLASS_VIEW: ClassId = 0x7F0B;
const LENS: u32 = (CONCEPT_R2IL_FOLD as u32) << 16;
const REV: SchemaRev = SchemaRev(1);
const R2IL: R2ilLens = R2ilLens {
    lens: LENS,
    schema_rev: REV,
};
const FOCUS: Focus = Focus { revision: 1 };
const SCOPE: Scope = Scope { id: 1 };
const POSITION: u16 = 3;
const FIELDS: [&str; 3] = ["a", "b", "m"];
const WORDS: [&str; 1] = ["IntSLess"];

struct Views {
    class: Vec<FieldRef>,
    lens: Vec<FieldRef>,
}

fn refs(labels: &[&str], ns: &str) -> Vec<FieldRef> {
    labels
        .iter()
        .map(|l| FieldRef::new(format!("lab:{ns}/{l}"), (*l).to_string()))
        .collect()
}

impl ClassView for Views {
    fn fields(&self, class: ClassId) -> &[FieldRef] {
        match class {
            CLASS_VIEW => &self.class,
            c if c == CONCEPT_R2IL_FOLD => &self.lens,
            _ => &[],
        }
    }
    fn template(&self, _class: ClassId) -> DisplayTemplate {
        DisplayTemplate::Card
    }
    fn dolce_category_id(&self, _class: ClassId) -> u8 {
        0
    }
}

/// ON a, b; WHEN `a < 5 AND b < 5` under `logic`; DO `m = a < 5`.
fn binding(logic: Logic, truth: Truth) -> MethodBinding {
    MethodBinding {
        id: MethodId(1),
        sig_rev: SigRev(1),
        name: "ki1_dry_run".into(),
        on: WideFieldMask::from_positions(&[0, 1]),
        when: Some(Guard::new(
            SetExpr::And(vec![
                SetExpr::i32(0, Cmp::Lt, THRESHOLD),
                SetExpr::i32(1, Cmp::Lt, THRESHOLD),
            ]),
            logic,
        )),
        reads: WideFieldMask::from_positions(&[0, 1]),
        effects: vec![Effect {
            target: 2,
            algebra: AlgebraRef {
                lens: LENS,
                word: 0,
            },
            args: vec![Arg::Field(0), Arg::Const(i64::from(THRESHOLD))],
        }],
        population: Population::Distinct,
        truth,
        demand: ValueDemand::Values,
    }
}

/// `prepare` / `prepare_gated` over the registered binding, as the refusal each
/// returns.
fn prepared(logic: Logic, truth: Truth) -> (Result<(), ExecRefusal>, Result<(), ExecRefusal>) {
    let views = Views {
        class: refs(&FIELDS, "class"),
        lens: refs(&WORDS, "r2il"),
    };
    let mut store = BindStore::new();
    store
        .declare_algebra(LENS, REV, Extent::DR64_CUBE, vec![2])
        .unwrap();
    store
        .register(
            &views,
            CLASS,
            ClassSchema {
                schema_rev: REV,
                field_count: FIELDS.len(),
                extent: Extent::DR64_CUBE,
            },
            vec![binding(logic, truth)],
        )
        .expect("DT64 registers a Kleene-guarded binding");
    let version = store.route(POSITION, CLASS).unwrap();
    let r = store.method_ref(CLASS, MethodId(1)).unwrap();
    let frames = FrameStore::new(FrameRevisions {
        source: 1,
        relation_map: 1,
        coordinates: 1,
    });
    let g = Guards {
        frame: frames.seal(),
        focus: FOCUS,
        scope: SCOPE,
    };
    let res = Resolution {
        method: &r,
        schema_rev: REV,
        route: (version, POSITION),
        guards: &g,
    };
    let now = (FOCUS, SCOPE);
    (
        prepare(&store, &views, R2IL, &res, &frames, now).map(|_| ()),
        prepare_gated(&store, &views, R2IL, &res, &frames, now).map(|_| ()),
    )
}

/// RED-PINNED BLOCKER. No MethodBinding seam evaluates a `Logic::Kleene` WHEN
/// yet, so the KI-1 case cannot be run through DT64 and compared with the
/// oracle above.
///
/// Refusal site: `lance-graph-r2il/src/binding.rs`, `prepare_with`: a guard
/// whose logic is not `TwoValued` returns
/// `ExecRefusal::ThreeValuedNotExecutable` from `prepare_gated`, and a binding
/// with `truth == Truth::ThreeValued` is refused the same way by both
/// `prepare` and `prepare_gated`. The ungated `prepare` refuses any guard as
/// `GuardNotExecutable`.
///
/// Decision B (Linear EXO-294) already gives the data side: dt64 has
/// `PresenceSource`, `LaneBinding::borrow_all_present` and `Arg::Present`, and
/// r2il has `FieldPresence` / `prepare_native_present`. What is missing is the
/// evaluation of the guard over presence planes.
///
/// HIRO source: "A typed 3-valued guard must not collapse UNKNOWN to FALSE";
/// D3: "`A AND B` with A true and B unknown does not fire".
///
/// WHEN lance-graph adds Kleene evaluation over presence planes, this test
/// turns RED. Replace it with one that prepares and runs the binding over
/// presence-tagged lanes (`lanes_for`) and compares the fired rows with
/// `fire` in the oracle half of this file at the same widths.
#[test]
fn ki1_methodbinding_seam_refuses_kleene_until_lance_graph_evaluates_it() {
    // A Kleene guard: gated prepare refuses it as three-valued.
    let (ungated, gated) = prepared(Logic::Kleene, Truth::TwoValued);
    assert_eq!(ungated, Err(ExecRefusal::GuardNotExecutable));
    assert_eq!(gated, Err(ExecRefusal::ThreeValuedNotExecutable));

    // A three-valued binding is refused by both, whatever the guard's logic.
    let (ungated, gated) = prepared(Logic::Kleene, Truth::ThreeValued);
    assert_eq!(ungated, Err(ExecRefusal::GuardNotExecutable));
    assert_eq!(gated, Err(ExecRefusal::ThreeValuedNotExecutable));

    // Control: the same guard under two-valued logic IS accepted by the gated
    // seam, so the refusal above is the logic and nothing else.
    let (_, gated) = prepared(Logic::TwoValued, Truth::TwoValued);
    assert_eq!(gated, Ok(()));
}
