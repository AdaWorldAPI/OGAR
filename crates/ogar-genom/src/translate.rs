//! The codon fold: DNA2 codon (6 bits) → amino acid, standard genetic code.
//!
//! The 64-entry table is the whole "ISA". It is biology's table, not ours, and
//! it is kept apart from every other 0..63 vocabulary in the workspace.

use crate::alphabet::{Base, BaseSet};
use crate::coord::Frame;
use crate::view::SeqView;

/// Codon index `b0<<4 | b1<<2 | b2` in DNA2 codes (A=0 C=1 G=2 T=3).
#[inline]
pub const fn codon_index(b0: Base, b1: Base, b2: Base) -> u8 {
    (b0.code() << 4) | (b1.code() << 2) | b2.code()
}

/// NCBI translation table 1, indexed by [`codon_index`]; `*` is stop.
pub const CODON_TABLE: [u8; 64] = build_table();

const fn build_table() -> [u8; 64] {
    // The classic TCAG-ordered table, re-indexed into DNA2 (ACGT) order.
    const TCAG: &[u8; 64] = b"FFLLSSSSYY**CC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG";
    const ORD: [usize; 4] = [2, 1, 3, 0]; // DNA2 code → TCAG rank (A→2, C→1, G→3, T→0)
    let mut out = [0u8; 64];
    let mut c = 0;
    while c < 64 {
        out[c] = TCAG[ORD[c >> 4] * 16 + ORD[(c >> 2) & 3] * 4 + ORD[c & 3]];
        c += 1;
    }
    out
}

/// Translate one codon of possible-base sets. If every expansion yields the
/// same residue (e.g. `GCN` → `A`), that residue; otherwise `X`. An empty set
/// anywhere yields `X`.
pub fn translate_codon_sets(s0: BaseSet, s1: BaseSet, s2: BaseSet) -> u8 {
    if let (Some(a), Some(b), Some(c)) = (s0.as_single(), s1.as_single(), s2.as_single()) {
        return CODON_TABLE[codon_index(a, b, c) as usize];
    }
    let mut seen: Option<u8> = None;
    for a in s0.iter() {
        for b in s1.iter() {
            for c in s2.iter() {
                let aa = CODON_TABLE[codon_index(a, b, c) as usize];
                match seen {
                    None => seen = Some(aa),
                    Some(x) if x == aa => {}
                    Some(_) => return b'X',
                }
            }
        }
    }
    seen.unwrap_or(b'X')
}

/// Streaming translation of a view in a reading frame. Yields one residue per
/// complete codon; a trailing partial codon is dropped. Allocates nothing.
pub fn translate<'a>(view: SeqView<'a>, frame: Frame) -> AminoAcids<'a> {
    AminoAcids {
        view,
        pos: frame.offset() as u64,
    }
}

pub struct AminoAcids<'a> {
    view: SeqView<'a>,
    pos: u64,
}

impl Iterator for AminoAcids<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if self.pos + 3 > self.view.len() {
            return None;
        }
        let p = self.pos;
        self.pos += 3;
        let v = &self.view;
        Some(translate_codon_sets(
            v.base_set(p),
            v.base_set(p + 1),
            v.base_set(p + 2),
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.view.len().saturating_sub(self.pos) / 3) as usize;
        (n, Some(n))
    }
}

impl ExactSizeIterator for AminoAcids<'_> {}
