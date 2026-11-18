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
//! The file has two halves. The oracle is a scalar Kleene evaluator over
//! presence words and `i32` lanes. The seam half registers the real
//! `MethodBinding`, computes the guard's TRUE rows THROUGH THE CLASS BINDING
//! (the guard read from the bound method, its fields requested as a
//! `WideFieldMask`, lanes and presence planes lent by
//! `LaneBinding::borrow_all_present` from a source that maps the resolved
//! `FieldRef`, then `where_true` over what was lent), runs the gated r2il plan,
//! and compares with the oracle. It requires lance-graph #2132 (EXO-326), which
//! lets `prepare_gated` admit a Kleene guard. What is still refused is pinned
//! beside it, each with its blocker.
#![cfg(feature = "ki1")]

use lance_graph_contract::class_view::{ClassId, ClassView, WideFieldMask};
use lance_graph_contract::ontology::{DisplayTemplate, FieldRef};
use lance_graph_contract::set_algebra::{Cmp, Logic, SetExpr};
use lance_graph_contract::set_rules::{Population, Truth};
use lance_graph_dt64::dr::{Focus, Frame, FrameRevisions, FrameStore, Guards, Scope};
use lance_graph_dt64::{
    AlgebraRef, Arg, BindStore, ClassSchema, CoordSpace, Effect, Extent, Guard, LaneAddr,
    LaneRefusal, LaneRequest, LaneSource, MethodBinding, MethodId, PresenceSource, Refusal,
    RouteAt, RouteVersion, SchemaRev, SigRev, ValueDemand,
};
use lance_graph_mask_risc::{
    Foreign, LaneRef, Out, Planes as ResidentPlanes, Scratch, execute_into,
};
use lance_graph_quack::set::{SetLowered, lower_set};
use lance_graph_quack::{Agg, Mask, Query, lower};
use lance_graph_r2il::FoldDialect;
use lance_graph_r2il::binding::{
    ExecPermit, ExecRefusal, FieldLanes, Gate, Prepared, R2ilLens, Receipt, Resolution, prepare,
    prepare_gated, run_gated,
};
use lance_graph_r2il::binding_native::prepare_native;
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
// The seam: r2il's MethodBinding preparation, run under a Kleene gate.
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
const NOW: (Focus, Scope) = (FOCUS, SCOPE);
const POSITION: u16 = 3;
/// The class's own order of fields: two `i32` lanes, the predicate mask `m`,
/// and the count `s`. A layout may reorder these and add a field `c`.
const FIELDS: [&str; 4] = ["a", "b", "m", "s"];
/// The lens words, by label: `IntSLess` (arity 2) and `PopCount` (arity 1).
const WORDS: [&str; 2] = ["IntSLess", "PopCount"];
const W_LESS: u8 = 0;
const W_COUNT: u8 = 1;
/// The rows of the source: one population in one row order.
const ROWS: CoordSpace = CoordSpace {
    population: 1,
    coordinates: 1,
};

/// The Kleene guard, gated: WHEN `a < 5 AND b < 5`.
const KLEENE: u32 = 1;
/// A two-valued guard of another shape (`a < 5`): the source of a foreign gate.
const OTHER_GUARD: u32 = 2;
/// The Kleene guard again, with a three-valued RESULT.
const THREE_VALUED: u32 = 3;

/// The class view's order of fields. The guard and the effects are written
/// against the positions this layout resolves, never against fixed numbers.
#[derive(Clone)]
struct Layout {
    fields: Vec<&'static str>,
}

impl Layout {
    fn pos(&self, label: &str) -> u8 {
        let i = self.fields.iter().position(|f| *f == label).unwrap();
        u8::try_from(i).unwrap()
    }

    /// Field to resident lane of the plan's planes: `a` is lane 0, `b` is
    /// lane 1; every other field has none.
    fn plan_lanes(&self) -> Vec<Option<u16>> {
        self.fields
            .iter()
            .map(|f| match *f {
                "a" => Some(0),
                "b" => Some(1),
                _ => None,
            })
            .collect()
    }
}

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

/// `a < 5 AND b < 5`, over the positions of `a` and `b` in `layout`.
fn kleene_guard(layout: &Layout) -> SetExpr {
    SetExpr::And(vec![
        SetExpr::i32(u16::from(layout.pos("a")), Cmp::Lt, THRESHOLD),
        SetExpr::i32(u16::from(layout.pos("b")), Cmp::Lt, THRESHOLD),
    ])
}

/// ON a, b; WHEN `when`; DO `m = a < 5; s = PopCount(m)`.
fn binding(layout: &Layout, id: u32, when: (SetExpr, Logic), truth: Truth) -> MethodBinding {
    let (a, b, m, s) = (
        layout.pos("a"),
        layout.pos("b"),
        layout.pos("m"),
        layout.pos("s"),
    );
    MethodBinding {
        id: MethodId(id),
        sig_rev: SigRev(1),
        name: format!("ki1_dry_run_{id}"),
        on: WideFieldMask::from_positions(&[a, b]),
        when: Some(Guard::new(when.0, when.1)),
        reads: WideFieldMask::from_positions(&[a, b]),
        effects: vec![
            Effect {
                target: m,
                algebra: AlgebraRef {
                    lens: LENS,
                    word: W_LESS,
                },
                args: vec![Arg::Field(a), Arg::Const(i64::from(THRESHOLD))],
            },
            Effect {
                target: s,
                algebra: AlgebraRef {
                    lens: LENS,
                    word: W_COUNT,
                },
                args: vec![Arg::Field(m)],
            },
        ],
        population: Population::Distinct,
        truth,
        demand: ValueDemand::Values,
    }
}

/// The three methods of the class, bound together.
fn bindings(layout: &Layout) -> Vec<MethodBinding> {
    vec![
        binding(
            layout,
            KLEENE,
            (kleene_guard(layout), Logic::Kleene),
            Truth::TwoValued,
        ),
        binding(
            layout,
            OTHER_GUARD,
            (
                SetExpr::i32(u16::from(layout.pos("a")), Cmp::Lt, THRESHOLD),
                Logic::TwoValued,
            ),
            Truth::TwoValued,
        ),
        binding(
            layout,
            THREE_VALUED,
            (kleene_guard(layout), Logic::Kleene),
            Truth::ThreeValued,
        ),
    ]
}

fn schema(layout: &Layout) -> ClassSchema {
    ClassSchema {
        schema_rev: REV,
        field_count: layout.fields.len(),
        extent: Extent::DR64_CUBE,
    }
}

struct Fixture {
    layout: Layout,
    views: Views,
    store: BindStore,
    version: RouteVersion,
    frames: FrameStore,
}

impl Fixture {
    fn new() -> Self {
        Self::with_order(&FIELDS)
    }

    /// The class bound over a view that orders its fields as `order`.
    fn with_order(order: &[&'static str]) -> Self {
        let layout = Layout {
            fields: order.to_vec(),
        };
        let views = Views {
            class: refs(order, "class"),
            lens: refs(&WORDS, "r2il"),
        };
        let mut store = BindStore::new();
        store
            .declare_algebra(LENS, REV, Extent::DR64_CUBE, vec![2, 1])
            .unwrap();
        store
            .register(&views, CLASS, schema(&layout), bindings(&layout))
            .expect("DT64 registers a Kleene-guarded binding");
        let version = store.route(POSITION, CLASS).unwrap();
        Self {
            layout,
            views,
            store,
            version,
            frames: FrameStore::new(FrameRevisions {
                source: 1,
                relation_map: 1,
                coordinates: 1,
            }),
        }
    }

    /// `prepare` (`gated == false`) or `prepare_gated` (`true`) for method `id`.
    fn prepare(&self, id: u32, gated: bool) -> Result<Prepared, ExecRefusal> {
        let r = self.store.method_ref(CLASS, MethodId(id)).unwrap();
        let g = Guards {
            frame: self.frames.seal(),
            focus: FOCUS,
            scope: SCOPE,
        };
        let res = Resolution {
            method: &r,
            schema_rev: REV,
            route: (self.version, POSITION),
            guards: &g,
        };
        let f = if gated { prepare_gated } else { prepare };
        f(&self.store, &self.views, R2IL, &res, &self.frames, NOW)
    }

    /// The gate a caller hands over for method `id`: plane 0, with the witness
    /// of the guard that method's reference carries.
    fn gate(&self, id: u32) -> Gate {
        Gate {
            plane: 0,
            guard: self
                .store
                .method_ref(CLASS, MethodId(id))
                .unwrap()
                .guard
                .expect("the method is guarded"),
        }
    }

    fn permit(&self, ids: &[u32]) -> ExecPermit {
        ExecPermit {
            methods: ids
                .iter()
                .map(|&i| self.store.method_ref(CLASS, MethodId(i)).unwrap())
                .collect(),
        }
    }

    /// `run_gated` over the lanes of `a` and `b`, with `gate_plane` as plane 0.
    fn run(
        &self,
        plan: &Prepared,
        a: &Lane,
        b: &Lane,
        gate_plane: &[u64],
        gate: &Gate,
    ) -> Result<Receipt, ExecRefusal> {
        let lanes = [LaneRef::I32(&a.values), LaneRef::I32(&b.values)];
        let masks = [gate_plane];
        let planes = ResidentPlanes {
            n_rows: a.values.len(),
            masks: &masks,
            lanes: &lanes,
        };
        let mut dialect = FoldDialect::new(&planes, &Foreign::NONE);
        run_gated(
            plan,
            &self.store,
            &self.frames,
            NOW,
            &self.permit(&[KLEENE, OTHER_GUARD, THREE_VALUED]),
            FieldLanes {
                lanes: &self.layout.plan_lanes(),
            },
            &mut dialect,
            gate,
        )
    }
}

// ---------------------------------------------------------------------------
// The source: the owner of the stored lanes and their presence planes.
// ---------------------------------------------------------------------------

/// A lane whose every row is present, with value 0: a field of the source
/// that no guard here reads.
fn full_lane(n: usize) -> Lane {
    let mut presence = vec![0u64; words(n)];
    for i in 0..n {
        presence[i / 64] |= 1 << (i % 64);
    }
    Lane {
        values: vec![0; n],
        presence,
    }
}

/// The source's own lane numbers, by the field's label.
const SRC_A: u16 = 0;
const SRC_B: u16 = 1;
const SRC_C: u16 = 2;

/// Three stored lanes (`a`, `b`, `c`) and the presence plane of each that the
/// source states. It maps the field the class view resolved, by label: a
/// layout may order the class's fields as it likes, and it still holds.
struct Source<'a> {
    frames: &'a FrameStore,
    values: [&'a [i32]; 3],
    present: [Option<&'a [u64]>; 3],
}

impl<'a> Source<'a> {
    fn new(frames: &'a FrameStore, a: &'a Lane, b: &'a Lane, c: &'a Lane) -> Self {
        Source {
            frames,
            values: [&a.values, &b.values, &c.values],
            present: [
                Some(&a.presence[..]),
                Some(&b.presence[..]),
                Some(&c.presence[..]),
            ],
        }
    }

    /// The same source stating no presence plane for source lane `lane`.
    fn without_presence(mut self, lane: u16) -> Self {
        self.present[usize::from(lane)] = None;
        self
    }
}

impl LaneSource for Source<'_> {
    type Lane<'s>
        = LaneRef<'s>
    where
        Self: 's;

    fn lane_of(
        &self,
        class: u32,
        rev: SchemaRev,
        _field: u8,
        basis: &FieldRef,
        frame: &Frame,
    ) -> Option<LaneAddr> {
        self.frames.check(frame).ok()?;
        // By the field the class view resolved, never by its position.
        let lane = match basis.label.as_str() {
            "a" => SRC_A,
            "b" => SRC_B,
            "c" => SRC_C,
            _ => return None,
        };
        (class == CLASS && rev == REV).then_some(LaneAddr {
            space: ROWS,
            lane,
            scope: SCOPE,
        })
    }

    fn borrow(&self, addr: &LaneAddr) -> Option<LaneRef<'_>> {
        Some(LaneRef::I32(self.values.get(usize::from(addr.lane))?))
    }

    fn frames(&self) -> &FrameStore {
        self.frames
    }
}

impl PresenceSource for Source<'_> {
    fn borrow_presence(&self, addr: &LaneAddr) -> Option<&[u64]> {
        *self.present.get(usize::from(addr.lane))?
    }
}

/// The rows where the guard of method `id` is TRUE, computed through the class
/// binding. The caller names the method and the route and nothing else:
///
/// 1. the guard and its logic are read from the bound method, not re-stated;
/// 2. its fields are requested as a `WideFieldMask`;
/// 3. their lanes and presence planes are lent by
///    `LaneBinding::borrow_all_present`, which asks the source for each
///    RESOLVED `FieldRef`;
/// 4. the TRUE rows are `where_true` over what was lent.
///
/// With `validity == false` the lent planes are not named nullable, so every
/// row is read as present: the two-valued reading, which is wrong here.
fn gate_through_binding(
    fx: &Fixture,
    source: &Source<'_>,
    id: u32,
    validity: bool,
) -> Result<Vec<u64>, LaneRefusal> {
    let bound = fx
        .store
        .admit(&fx.store.method_ref(CLASS, MethodId(id)).unwrap())
        .unwrap();
    let guard = bound.when.as_ref().expect("the method is guarded");
    assert_eq!(guard.logic, Logic::Kleene);
    let SetLowered::Filter(set) = lower_set(&guard.expr, guard.logic).unwrap() else {
        panic!("a filter")
    };
    let fields: Vec<u8> = set
        .lanes()
        .iter()
        .map(|&l| u8::try_from(l).expect("a field position"))
        .collect();
    let frame = fx.frames.seal();
    let lane_binding = fx.store.bind_lanes(
        &fx.views,
        &fx.frames,
        source,
        LaneRequest {
            route: RouteAt {
                position: POSITION,
                version: fx.version,
            },
            rev: REV,
            fields: &WideFieldMask::from_positions(&fields),
            frame: &frame,
            scope: SCOPE,
            selection: ROWS,
        },
    )?;
    let (mut lanes, mut masks, mut nullable) = (Vec::new(), Vec::new(), Vec::new());
    for lent in lane_binding.borrow_all_present(&fx.store, &fx.views, &fx.frames, source)? {
        let (field, lane, plane) = lent?;
        nullable.push((u16::from(field), Mask(masks.len() as u16)));
        lanes.push(lane);
        masks.push(plane);
    }
    // Column `i` reads `set.lanes()[i]`: both are in ascending field order.
    let filter = set
        .where_true(set.lanes(), if validity { &nullable } else { &[] })
        .unwrap();
    let rows = source.values[0].len();
    let planes = ResidentPlanes {
        n_rows: rows,
        masks: &masks,
        lanes: &lanes,
    };
    let program = lower(&Query {
        filter,
        agg: Agg::Rows,
    })
    .unwrap();
    let mut scratch = Scratch::for_program(&program, rows).unwrap();
    let mut out = vec![0u64; words(rows)];
    execute_into(
        &program,
        &planes,
        &Foreign::NONE,
        &mut scratch,
        Out::Mask(&mut out),
    )
    .unwrap();
    Ok(out)
}

/// [`gate_through_binding`] over a source that states every presence plane.
fn gate_of(
    fx: &Fixture,
    a: &Lane,
    b: &Lane,
    id: u32,
    validity: bool,
) -> Result<Vec<u64>, LaneRefusal> {
    let c = full_lane(a.values.len());
    gate_through_binding(fx, &Source::new(&fx.frames, a, b, &c), id, validity)
}

fn count(plane: &[u64]) -> i64 {
    plane.iter().map(|w| i64::from(w.count_ones())).sum()
}

/// The registered binding, run through DT64 and r2il, fires on exactly the rows
/// the scalar Kleene oracle fires on.
///
/// The gate is computed through the class binding (`gate_through_binding`):
/// the guard is read from the bound method, its fields are requested as a
/// `WideFieldMask`, `LaneBinding::borrow_all_present` lends the lanes and
/// presence planes the source maps by the resolved `FieldRef`, and `where_true`
/// runs over what was lent. The guard `a < 5 AND b < 5` needs no equality form.
/// The plan is `prepare_gated`, the run is `run_gated`, and its `PopCount`
/// folds the mask `m = a < 5` restricted to the gate. Because the guard implies
/// `a < 5`, that count is the number of fired rows. The fired rows themselves
/// are the gate plane, which the r2il receipt does not return, so they are
/// compared directly.
#[test]
fn ki1_methodbinding_fires_only_on_true_rows_through_the_kleene_gate() {
    let fx = Fixture::new();
    let plan = fx
        .prepare(KLEENE, true)
        .expect("lance-graph #2132 (EXO-326): a gated plan admits a Kleene guard");
    let count_field = fx.layout.pos("s");
    let m_field = fx.layout.pos("m");
    for n in [1usize, 63, 64, 65, 1000] {
        let (a, b) = lanes_for(n);
        let oracle = fire(&a, &b);

        // The gate holds exactly the oracle's rows, row by row.
        let gate = gate_of(&fx, &a, &b, KLEENE, true).expect("the class binding lends the guard");
        for i in 0..n {
            let (ta, tb) = COMBOS[i % COMBOS.len()];
            assert_eq!(
                bit(&gate, i),
                bit(&oracle, i),
                "width {n} row {i}: A={ta:?} B={tb:?} gate must keep the row iff the guard is TRUE"
            );
        }
        assert_eq!(gate, oracle, "width {n}: gate plane differs (stray bits)");

        // The plan, run under that gate, counts the fired rows.
        let want = count(&oracle);
        let receipt = fx
            .run(&plan, &a, &b, &gate, &fx.gate(KLEENE))
            .expect("the gated plan runs");
        assert_eq!(
            receipt.output.values,
            vec![(count_field, want)],
            "width {n}: fired count"
        );
        assert_eq!(
            receipt.output.intermediates,
            vec![m_field],
            "m is no answer"
        );

        // The two-valued reading of the same lanes: an absent row's stored value
        // is read as a value. It fires on UNKNOWN rows, so it fires more, as soon
        // as the width holds one (width 1 is a lone (T, T) row).
        let two_valued = (0..n)
            .filter(|&i| a.values[i] < THRESHOLD && b.values[i] < THRESHOLD)
            .count() as i64;
        if n >= COMBOS.len() {
            assert!(
                want < two_valued,
                "width {n}: {want} must be < {two_valued}"
            );
        } else {
            assert_eq!(want, two_valued, "width {n}");
        }
        // And that reading is what Quack gives with no validity planes named.
        assert_eq!(
            count(&gate_of(&fx, &a, &b, KLEENE, false).unwrap()),
            two_valued,
            "width {n}"
        );
    }
}

/// The source maps the field the class view resolved, so the gate does not
/// depend on where the view puts the field: appended, prepended (every
/// position shifts) or reordered, it is the oracle's gate, and the plan counts
/// the same fired rows.
///
/// FAILS IF: the source answers by position. In the prepended layout `a` sits
/// at position 1 and `b` at 2, where a positional source holds `b` and `c`.
#[test]
fn ki1_gate_does_not_depend_on_where_the_view_puts_the_fields() {
    let layouts: [&[&'static str]; 4] = [
        &["a", "b", "m", "s"],
        &["a", "b", "m", "s", "c"],
        &["c", "a", "b", "m", "s"],
        &["s", "b", "m", "a", "c"],
    ];
    for order in layouts {
        let fx = Fixture::with_order(order);
        let plan = fx.prepare(KLEENE, true).unwrap();
        for n in [64usize, 65, 1000] {
            let (a, b) = lanes_for(n);
            let oracle = fire(&a, &b);
            let gate = gate_of(&fx, &a, &b, KLEENE, true).unwrap();
            assert_eq!(gate, oracle, "{order:?} width {n}: gate");
            let receipt = fx.run(&plan, &a, &b, &gate, &fx.gate(KLEENE)).unwrap();
            assert_eq!(
                receipt.output.values,
                vec![(fx.layout.pos("s"), count(&oracle))],
                "{order:?} width {n}: fired count"
            );
        }
    }
}

/// A guard field whose source states no presence plane is refused by name,
/// never read as all present. The refusal names the class view's field
/// position, and a field the guard does not read may stay unstated.
#[test]
fn ki1_a_guard_field_without_a_presence_plane_is_refused_by_name() {
    let fx = Fixture::new();
    let (a, b) = lanes_for(64);
    let c = full_lane(64);
    let pos = |l: &str| fx.layout.pos(l);

    // Silence twin: every plane stated, and one the guard does not read left
    // unstated, is fine.
    let ok = Source::new(&fx.frames, &a, &b, &c).without_presence(SRC_C);
    assert_eq!(
        gate_through_binding(&fx, &ok, KLEENE, true),
        Ok(fire(&a, &b))
    );

    let no_a = Source::new(&fx.frames, &a, &b, &c).without_presence(SRC_A);
    assert_eq!(
        gate_through_binding(&fx, &no_a, KLEENE, true),
        Err(LaneRefusal::PresenceUnbound { field: pos("a") })
    );
    assert_eq!(pos("a"), 0);

    let no_b = Source::new(&fx.frames, &a, &b, &c).without_presence(SRC_B);
    assert_eq!(
        gate_through_binding(&fx, &no_b, KLEENE, true),
        Err(LaneRefusal::PresenceUnbound { field: pos("b") })
    );
    assert_eq!(pos("b"), 1);

    // By the view's field, not the source's lane: in a view that puts `c` and
    // `a` in front, `b` sits at position 2 while the source holds it at lane 1.
    let moved = Fixture::with_order(&["c", "a", "b", "m", "s"]);
    let no_b = Source::new(&moved.frames, &a, &b, &c).without_presence(SRC_B);
    assert_eq!(
        gate_through_binding(&moved, &no_b, KLEENE, true),
        Err(LaneRefusal::PresenceUnbound { field: 2 })
    );
}

/// The guard's logic is fixed when the class is bound. Binding the same class
/// again with the same guard under `Logic::TwoValued` is
/// `Refusal::BehaviourImmutable(class)`; binding it again unchanged is idempotent;
/// and the registered guard is still Kleene afterwards.
#[test]
fn ki1_the_guard_logic_is_fixed_at_binding() {
    let mut fx = Fixture::new();
    let layout = fx.layout.clone();
    let mut methods = bindings(&layout);
    methods[0] = binding(
        &layout,
        KLEENE,
        (kleene_guard(&layout), Logic::TwoValued),
        Truth::TwoValued,
    );
    assert_eq!(
        fx.store
            .register(&fx.views, CLASS, schema(&layout), methods),
        Err(Refusal::BehaviourImmutable(CLASS))
    );
    // Silence twin: the original binding is accepted again.
    assert_eq!(
        fx.store
            .register(&fx.views, CLASS, schema(&layout), bindings(&layout)),
        Ok(())
    );
    // And the bound method still carries its Kleene guard.
    let bound = fx
        .store
        .admit(&fx.store.method_ref(CLASS, MethodId(KLEENE)).unwrap())
        .unwrap();
    assert_eq!(bound.when.as_ref().unwrap().logic, Logic::Kleene);
}

/// Still refused, blocker: the ungated r2il seam never evaluates a WHEN. A
/// guarded binding runs only under a gate (`prepare_gated`); `prepare` gives
/// `GuardNotExecutable`. This is the design of the seam, not a gap #2132 closes.
#[test]
fn ki1_ungated_prepare_still_refuses_a_guard() {
    let fx = Fixture::new();
    assert_eq!(
        fx.prepare(KLEENE, false).map(|_| ()),
        Err(ExecRefusal::GuardNotExecutable)
    );
}

/// Still refused, blocker: the seam checks the gate's witness, not its bits. A
/// gate evaluated for another guard than the plan's is `GateMismatch`.
#[test]
fn ki1_a_foreign_gate_is_refused() {
    let fx = Fixture::new();
    let plan = fx.prepare(KLEENE, true).unwrap();
    let (a, b) = lanes_for(64);
    let bits = gate_of(&fx, &a, &b, KLEENE, true).unwrap();
    assert_eq!(
        fx.run(&plan, &a, &b, &bits, &fx.gate(OTHER_GUARD))
            .map(|_| ()),
        Err(ExecRefusal::GateMismatch)
    );
    // Silence twin: the plan's own gate runs.
    assert!(fx.run(&plan, &a, &b, &bits, &fx.gate(KLEENE)).is_ok());
}

/// Still refused, blocker: the dialect has no UNKNOWN rows, so a method whose
/// RESULT is three-valued (`Truth::ThreeValued`) cannot be executed: gated it is
/// `ThreeValuedNotExecutable` (ungated, the guard refuses first). #2132 admits
/// a three-valued GUARD, not a three-valued result.
#[test]
fn ki1_a_three_valued_result_is_refused() {
    let fx = Fixture::new();
    assert_eq!(
        fx.prepare(THREE_VALUED, true).map(|_| ()),
        Err(ExecRefusal::ThreeValuedNotExecutable)
    );
    assert_eq!(
        fx.prepare(THREE_VALUED, false).map(|_| ()),
        Err(ExecRefusal::GuardNotExecutable)
    );
}

/// Still refused, blocker: native gated plans (the literal ask of EXO-326).
/// `prepare_native` freezes its programs and has no gate operand, so it refuses
/// every gated plan with `GuardNotExecutable`, two-valued guards included.
///
/// TURNS RED WHEN NATIVE GATED PLANS LAND in lance-graph. Then replace it with
/// a test that prepares the native plan and runs it against the oracle.
#[test]
fn ki1_native_gated_plan_is_refused_until_native_gates_land() {
    let fx = Fixture::new();
    let (a, b) = lanes_for(64);
    let gate = gate_of(&fx, &a, &b, KLEENE, true).unwrap();
    let lanes = [LaneRef::I32(&a.values), LaneRef::I32(&b.values)];
    let masks = [&gate[..]];
    let planes = ResidentPlanes {
        n_rows: 64,
        masks: &masks,
        lanes: &lanes,
    };
    let plan_lanes = fx.layout.plan_lanes();
    for id in [KLEENE, OTHER_GUARD] {
        let plan = fx.prepare(id, true).unwrap();
        assert_eq!(
            prepare_native(
                &plan,
                &fx.store,
                &fx.frames,
                NOW,
                FieldLanes { lanes: &plan_lanes },
                &planes,
            )
            .map(|_| ()),
            Err(ExecRefusal::GuardNotExecutable),
            "method {id}"
        );
    }
}
