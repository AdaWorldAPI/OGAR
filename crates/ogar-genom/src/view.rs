//! Zero-copy views: a borrow of the resident sequence plus an interval and a strand.
//!
//! A view owns nothing. `sub` and `reverse_complement` return new views over the
//! same words; no base is copied until a caller asks for one.

use crate::alphabet::{Base, BaseSet};
use crate::coord::{Interval, Strand};
use crate::sequence::PackedSeq;

#[derive(Clone, Copy, Debug)]
pub struct SeqView<'a> {
    seq: &'a PackedSeq,
    /// Reference interval, always forward-strand coordinates.
    interval: Interval,
    strand: Strand,
}

impl<'a> SeqView<'a> {
    /// Forward-strand view of the whole sequence.
    pub fn new(seq: &'a PackedSeq) -> SeqView<'a> {
        SeqView {
            seq,
            interval: seq.full(),
            strand: Strand::Forward,
        }
    }

    /// View of a reference interval on a strand; `None` if out of range.
    pub fn of(seq: &'a PackedSeq, interval: Interval, strand: Strand) -> Option<SeqView<'a>> {
        (interval.end() <= seq.len()).then_some(SeqView {
            seq,
            interval,
            strand,
        })
    }

    #[inline]
    pub fn sequence(&self) -> &'a PackedSeq {
        self.seq
    }

    #[inline]
    pub fn reference_interval(&self) -> Interval {
        self.interval
    }

    #[inline]
    pub fn strand(&self) -> Strand {
        self.strand
    }

    #[inline]
    pub fn len(&self) -> u64 {
        self.interval.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.interval.is_empty()
    }

    /// View position `i` → forward reference position. Panics when out of range.
    #[inline]
    pub fn reference_pos(&self, i: u64) -> u64 {
        assert!(
            i < self.len(),
            "view index {i} out of range (len {})",
            self.len()
        );
        match self.strand {
            Strand::Forward => self.interval.start() + i,
            Strand::Reverse => self.interval.end() - 1 - i,
        }
    }

    /// Possible bases at view position `i`, read on this view's strand.
    pub fn base_set(&self, i: u64) -> BaseSet {
        let s = self.seq.base_set(self.reference_pos(i));
        match self.strand {
            Strand::Forward => s,
            Strand::Reverse => s.complement(),
        }
    }

    /// The base at `i` if unambiguous.
    #[inline]
    pub fn base(&self, i: u64) -> Option<Base> {
        self.base_set(i).as_single()
    }

    pub fn is_soft_masked(&self, i: u64) -> bool {
        self.seq.is_soft_masked(self.reference_pos(i))
    }

    /// Sub-view by **view** coordinates (so on the reverse strand, `[0, k)` is
    /// the first `k` bases read 5'→3' on that strand). `None` if out of range.
    pub fn sub(&self, view_interval: Interval) -> Option<SeqView<'a>> {
        if view_interval.end() > self.len() {
            return None;
        }
        let interval = match self.strand {
            Strand::Forward => Interval::ordered(
                self.interval.start() + view_interval.start(),
                self.interval.start() + view_interval.end(),
            ),
            Strand::Reverse => Interval::ordered(
                self.interval.end() - view_interval.end(),
                self.interval.end() - view_interval.start(),
            ),
        };
        Some(SeqView { interval, ..*self })
    }

    /// Same reference interval, read on the other strand. No bytes move.
    #[inline]
    pub fn reverse_complement(&self) -> SeqView<'a> {
        SeqView {
            strand: self.strand.flip(),
            ..*self
        }
    }

    /// Iterate possible-base sets along the view. Allocates nothing.
    pub fn base_sets(&self) -> impl Iterator<Item = BaseSet> + 'a {
        let v = *self;
        (0..v.len()).map(move |i| v.base_set(i))
    }

    /// Materialize as IUPAC ASCII (upper-case). O(len) — by name.
    pub fn materialize_iupac(&self) -> Vec<u8> {
        self.base_sets().map(BaseSet::to_iupac).collect()
    }
}
