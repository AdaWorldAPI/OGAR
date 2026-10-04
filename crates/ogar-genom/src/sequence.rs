//! The resident sequence: DNA2 words plus sparse interval sidecars.
//!
//! Exact round trip is part of the contract: [`decode`]`(`[`encode`]`(x)) == x`
//! for every accepted input, case included.

use crate::alphabet::{Base, BaseSet};
use crate::coord::Interval;

/// A maximal run of one ambiguous IUPAC symbol, e.g. 50,000 × `N`.
///
/// Resident DNA2 bits inside the run are **placeholder zeros** and carry no
/// meaning; every read that may land in a run must go through
/// [`PackedSeq::base_set`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AmbiguityRun {
    pub interval: Interval,
    pub set: BaseSet,
}

/// DNA2-packed sequence: 32 bases per `u64`, base `i` at bits `2*(i%32)..+2`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackedSeq {
    words: Box<[u64]>,
    len: u64,
    /// Sorted, non-overlapping, maximal (adjacent runs differ in `set`).
    ambiguity: Box<[AmbiguityRun]>,
    /// Sorted, non-overlapping, maximal lower-case runs (soft-masked repeats).
    soft_mask: Box<[Interval]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    /// A byte that is not an IUPAC nucleotide letter. `U`/`u` is refused on
    /// purpose: RNA is a different layer, and accepting it as `T` would make
    /// the round trip lossy.
    InvalidSymbol { pos: u64, byte: u8 },
}

impl core::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EncodeError::InvalidSymbol { pos, byte } => {
                write!(
                    f,
                    "invalid nucleotide symbol 0x{byte:02x} at position {pos}"
                )
            }
        }
    }
}

impl std::error::Error for EncodeError {}

/// Encode an ASCII nucleotide sequence (no FASTA headers or newlines).
pub fn encode(ascii: &[u8]) -> Result<PackedSeq, EncodeError> {
    let len = ascii.len() as u64;
    let mut words = vec![0u64; ascii.len().div_ceil(32)].into_boxed_slice();
    let mut ambiguity: Vec<AmbiguityRun> = Vec::new();
    let mut soft_mask: Vec<Interval> = Vec::new();

    for (i, &c) in ascii.iter().enumerate() {
        let pos = i as u64;
        if c.is_ascii_lowercase() {
            match soft_mask.last_mut() {
                Some(iv) if iv.end == pos => iv.end += 1,
                _ => soft_mask.push(Interval {
                    start: pos,
                    end: pos + 1,
                }),
            }
        }
        if let Some(b) = Base::from_ascii(c) {
            words[i >> 5] |= (b.code() as u64) << ((i & 31) * 2);
            continue;
        }
        let set = match BaseSet::from_iupac(c) {
            Some(s) if !matches!(c, b'U' | b'u') => s,
            _ => return Err(EncodeError::InvalidSymbol { pos, byte: c }),
        };
        match ambiguity.last_mut() {
            Some(r) if r.interval.end == pos && r.set == set => r.interval.end += 1,
            _ => ambiguity.push(AmbiguityRun {
                interval: Interval {
                    start: pos,
                    end: pos + 1,
                },
                set,
            }),
        }
    }
    Ok(PackedSeq {
        words,
        len,
        ambiguity: ambiguity.into_boxed_slice(),
        soft_mask: soft_mask.into_boxed_slice(),
    })
}

/// Full materialization back to ASCII. O(n) bytes written — the one named
/// materializer of this layer.
pub fn decode(seq: &PackedSeq) -> Vec<u8> {
    let mut out: Vec<u8> = (0..seq.len)
        .map(|p| seq.resident_base(p).to_ascii())
        .collect();
    for r in seq.ambiguity.iter() {
        let c = r.set.to_iupac();
        out[r.interval.start as usize..r.interval.end as usize].fill(c);
    }
    for iv in seq.soft_mask.iter() {
        for x in &mut out[iv.start as usize..iv.end as usize] {
            *x = x.to_ascii_lowercase();
        }
    }
    out
}

impl PackedSeq {
    #[inline]
    pub fn len(&self) -> u64 {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The packed words. Bits past `len` in the last word are zero.
    #[inline]
    pub fn words(&self) -> &[u64] {
        &self.words
    }

    pub fn ambiguity_runs(&self) -> &[AmbiguityRun] {
        &self.ambiguity
    }

    pub fn soft_mask(&self) -> &[Interval] {
        &self.soft_mask
    }

    /// Resident bytes: words + sidecars (excluding the struct header).
    pub fn resident_bytes(&self) -> usize {
        self.words.len() * 8
            + self.ambiguity.len() * core::mem::size_of::<AmbiguityRun>()
            + self.soft_mask.len() * core::mem::size_of::<Interval>()
    }

    /// The raw resident base — **ignores the ambiguity overlay**. Correct only
    /// where the caller already knows `pos` lies outside every ambiguity run.
    #[inline]
    pub fn resident_base(&self, pos: u64) -> Base {
        debug_assert!(pos < self.len);
        let i = pos as usize;
        Base::from_code((self.words[i >> 5] >> ((i & 31) * 2)) as u8)
    }

    /// The ambiguity run covering `pos`, if any. O(log runs).
    pub fn ambiguity_at(&self, pos: u64) -> Option<AmbiguityRun> {
        let k = self.ambiguity.partition_point(|r| r.interval.end <= pos);
        self.ambiguity
            .get(k)
            .copied()
            .filter(|r| r.interval.contains(pos))
    }

    /// The set of possible bases at `pos` on the forward strand. Panics if out of range.
    pub fn base_set(&self, pos: u64) -> BaseSet {
        assert!(
            pos < self.len,
            "position {pos} out of range (len {})",
            self.len
        );
        match self.ambiguity_at(pos) {
            Some(r) => r.set,
            None => BaseSet::single(self.resident_base(pos)),
        }
    }

    /// Whether `pos` is soft-masked (lower-case in the source). O(log runs).
    pub fn is_soft_masked(&self, pos: u64) -> bool {
        let k = self.soft_mask.partition_point(|iv| iv.end <= pos);
        self.soft_mask.get(k).is_some_and(|iv| iv.contains(pos))
    }

    /// Whole-sequence interval `[0, len)`.
    pub fn full(&self) -> Interval {
        Interval {
            start: 0,
            end: self.len,
        }
    }
}
