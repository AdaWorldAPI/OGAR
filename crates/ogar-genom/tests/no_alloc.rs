//! "A view does not secretly allocate a population": count heap allocations
//! while building views and running folds over a resident sequence.

use ogar_genom::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static A: Counting = Counting;

#[test]
fn views_and_folds_allocate_nothing() {
    let src: Vec<u8> = (0..100_000u32)
        .map(|i| b"ACGTNacgt"[(i.wrapping_mul(2654435761) >> 29) as usize % 9])
        .collect();
    let p = encode(&src).unwrap();
    let before = ALLOCS.load(Ordering::SeqCst);

    let v = SeqView::new(&p);
    let rc = v.reverse_complement();
    let sub = rc.sub(Interval::new(1_000, 90_000).unwrap()).unwrap();
    let mut residues = 0usize;
    for f in 0..3 {
        residues += translate(sub, Frame::new(f).unwrap())
            .filter(|&a| a != b'X')
            .count();
    }
    let gc = sub
        .base_sets()
        .filter(|s| s.is_subset_of(BaseSet::from_iupac(b'S').unwrap()))
        .count();

    let after = ALLOCS.load(Ordering::SeqCst);
    assert_eq!(
        after - before,
        0,
        "views/folds allocated {} times",
        after - before
    );
    // anti-vacuity: the folds really ran over real content
    assert!(
        residues > 1_000 && gc > 1_000,
        "residues={residues} gc={gc}"
    );
    // ...and the counter can fire: a materialization is observed
    let _m = sub.materialize_iupac();
    assert!(
        ALLOCS.load(Ordering::SeqCst) > after,
        "counting allocator is live"
    );
}
