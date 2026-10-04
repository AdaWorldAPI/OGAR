//! Coordinates: half-open intervals, strand, reading frame.
//!
//! These are **reference coordinates** — positions in one packed sequence. A
//! coordinate on a variant-bearing haplotype is a different space (indels
//! shift it), and is deliberately not representable here: the liftover between
//! the two belongs to the variant layer, not to this type.

/// Half-open `[start, end)` interval in base positions.
///
/// The fields are private so `start <= end` holds for every value: the only
/// public constructor is [`Interval::new`], which refuses inverted bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Interval {
    start: u64,
    end: u64,
}

impl Interval {
    /// `None` when `start > end`.
    pub const fn new(start: u64, end: u64) -> Option<Interval> {
        if start <= end {
            Some(Interval { start, end })
        } else {
            None
        }
    }

    /// Crate-internal constructor for bounds the caller has already ordered.
    #[inline]
    pub(crate) const fn ordered(start: u64, end: u64) -> Interval {
        debug_assert!(start <= end);
        Interval { start, end }
    }

    /// Crate-internal: grow a run by one base at its end.
    #[inline]
    pub(crate) fn extend_end(&mut self) {
        self.end += 1;
    }

    #[inline]
    pub const fn start(self) -> u64 {
        self.start
    }

    #[inline]
    pub const fn end(self) -> u64 {
        self.end
    }

    #[inline]
    pub const fn len(self) -> u64 {
        self.end - self.start
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    #[inline]
    pub const fn contains(self, pos: u64) -> bool {
        self.start <= pos && pos < self.end
    }

    #[inline]
    pub const fn contains_interval(self, o: Interval) -> bool {
        self.start <= o.start && o.end <= self.end
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Strand {
    Forward,
    Reverse,
}

impl Strand {
    #[inline]
    pub const fn flip(self) -> Strand {
        match self {
            Strand::Forward => Strand::Reverse,
            Strand::Reverse => Strand::Forward,
        }
    }
}

/// Reading frame offset `0..3` within a view (after strand is applied).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Frame(u8);

impl Frame {
    pub const ZERO: Frame = Frame(0);

    /// `None` for offsets ≥ 3.
    pub const fn new(offset: u8) -> Option<Frame> {
        if offset < 3 {
            Some(Frame(offset))
        } else {
            None
        }
    }

    #[inline]
    pub const fn offset(self) -> u8 {
        self.0
    }
}
