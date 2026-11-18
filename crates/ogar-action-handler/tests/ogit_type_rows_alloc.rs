//! Allocation of the per-write path with the per-type row ranges, in a test
//! binary of its own (it installs a counting global allocator).
#![cfg(feature = "quack")]

#[allow(dead_code)]
#[path = "../../ogar-from-schema/src/ogit_checkout.rs"]
mod ogit_checkout;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use lance_graph_mask_risc::{
    Foreign, ForeignPlane as ForeignMask, LaneRef, Out, Planes, Scratch, execute_compiled,
    words_for,
};
use ogar_action_handler::ogit::{OgitTables, Table, TypeId, TypeRows, Validator};

// Counts the allocations of the thread that reads it.
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

const WRITES: usize = 1000;

/// What the validator allocated per write before the ranges: nothing on an
/// accept, one result vector on a reject (an unknown type included). The same
/// numbers `tests/ogit_validator.rs` pins on the whole-table scan.
#[test]
fn the_per_write_path_allocates_exactly_what_it_did_before_the_ranges() {
    let t = OgitTables::load(&ogit_checkout::root()).unwrap();
    let mut v = Validator::new(&t).unwrap();
    let ty = "ogit.MARS:Machine";
    let ok = [
        "ogit/name",
        "ogit/MARS/Machine/class",
        "ogit/_id",
        "ogit/_type",
    ];
    let bad = ["ogit/name", "ogit/MARS/Machine/class", "zzz/a", "zzz/b"];
    let missing = ["ogit/name"];
    v.validate(&t, ty, &ok).unwrap();

    let before = allocs();
    for _ in 0..WRITES {
        assert!(v.validate(&t, ty, &ok).unwrap().is_empty());
    }
    let accept = allocs() - before;

    let before = allocs();
    for _ in 0..WRITES {
        assert_eq!(v.validate(&t, ty, &bad).unwrap().len(), 2);
    }
    let reject_unknown_key = allocs() - before;

    let before = allocs();
    for _ in 0..WRITES {
        assert_eq!(v.validate(&t, ty, &missing).unwrap().len(), 1);
    }
    let reject_missing = allocs() - before;

    let before = allocs();
    for _ in 0..WRITES {
        assert_eq!(v.validate(&t, "nope", &bad).unwrap().len(), 1);
    }
    let unknown_type = allocs() - before;

    // Switching types between calls (the range moves, the buffer is cleared)
    // allocates nothing either.
    let other = "ogit.Auth:Account";
    let before = allocs();
    for i in 0..WRITES {
        let ty = if i % 2 == 0 { ty } else { other };
        v.validate(&t, ty, &[]).unwrap();
    }
    let alternating = allocs() - before;

    // The range lookup itself.
    let rows = TypeRows::new(&t, Table::TypeAttr).unwrap();
    let id = t.type_id(ty).unwrap();
    let before = allocs();
    for _ in 0..WRITES {
        assert!(!rows.range(&t, id).unwrap().is_empty());
    }
    let lookup = allocs() - before;

    // An empty type and a type past the table are bound too, and allocate
    // nothing either.
    let before = allocs();
    for _ in 0..WRITES {
        assert!(rows.range(&t, TypeId(u16::MAX)).unwrap().is_empty());
    }
    let lookup_past = allocs() - before;

    println!(
        "allocations per {WRITES} writes: accept {accept}, reject (unknown key) \
         {reject_unknown_key}, reject (missing) {reject_missing}, unknown type \
         {unknown_type}, alternating types {alternating}, range lookup {lookup}"
    );
    assert_eq!(accept, 0, "accept");
    assert_eq!(reject_unknown_key, WRITES, "one vector per reject");
    assert_eq!(reject_missing, WRITES, "one vector per reject");
    assert_eq!(unknown_type, WRITES, "one vector per unknown type");
    // `Machine` rejects with two missing mandatory attributes, `Account` too:
    // each alternating write returns its vector, so a vector per write.
    assert_eq!(alternating, WRITES, "one vector per write that rejects");
    assert_eq!(lookup, 0, "the range lookup allocates nothing");
    assert_eq!(lookup_past, 0, "a lookup past the types allocates nothing");
}

/// The whole-table program the validator ran before, on the same tables: it
/// allocates nothing either, so the ranges add no allocation and remove none.
#[test]
fn the_whole_table_scan_allocated_nothing_either() {
    let t = OgitTables::load(&ogit_checkout::root()).unwrap();
    let v = Validator::new(&t).unwrap();
    let (n_types, n_attrs, n_rows) = (
        t.rows(Table::Type),
        t.rows(Table::Attribute),
        t.rows(Table::TypeAttr),
    );
    let mut scratch = Scratch::for_program(v.program().program(), n_rows).unwrap();
    let mut type_plane = vec![0u64; words_for(n_types)];
    type_plane[0] = 1;
    let present_plane = vec![0u64; words_for(n_attrs)];
    let mut out = vec![0u64; words_for(n_rows)];
    let [l0, l1, l2] = v.lanes(&t);
    let lanes = [LaneRef::U32(l0), LaneRef::U32(l1), LaneRef::U32(l2)];
    let planes = Planes {
        n_rows,
        masks: &[],
        lanes: &lanes,
    };
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
    let foreign = Foreign {
        planes: &foreign,
        lanes: &[],
    };
    let compiled = v.program().compiled();
    let before = allocs();
    for _ in 0..WRITES {
        execute_compiled(
            &compiled,
            &planes,
            &foreign,
            &mut scratch,
            Out::Mask(&mut out),
            0..n_rows,
        )
        .unwrap();
    }
    assert_eq!(allocs() - before, 0);
}
