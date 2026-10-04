//! # ogar-genom — packed genomic sequence substrate (first proof)
//!
//! The law this crate tests: **the sequence stays resident; another biological
//! interpretation is a new coordinate, a mask, or a fold — not a copy.**
//!
//! | operation | what bytes move |
//! |---|---|
//! | [`encode`](sequence::encode) | ASCII in → 2 bits/base + interval sidecars (a durable write, once) |
//! | [`SeqView::sub`](view::SeqView::sub) / [`SeqView::reverse_complement`](view::SeqView::reverse_complement) | none — a view is a borrow plus an interval and a strand |
//! | [`SeqView::base`](view::SeqView::base), [`SeqView::base_set`](view::SeqView::base_set) | one word read, plus a binary search of the ambiguity sidecar |
//! | [`translate`](translate::translate) | streaming fold: yields amino acids, allocates nothing |
//! | [`decode`](sequence::decode) | full materialization, by name, O(n) |
//!
//! ## Representation (measured, not chosen by elegance)
//!
//! Resident sequence is **DNA2** (`A=00 C=01 G=10 T=11`, 32 bases per `u64`).
//! Ambiguity (IUPAC `N`, `R`, `Y`, …) lives in a **sparse run sidecar**, not in
//! the resident bits. On GRCh38 chr21 (46.7 Mbp) the reference has **52** `N`
//! runs, **zero** non-`N` IUPAC symbols, and **48,677** soft-mask runs: the
//! sidecars cost 0.067 bits/base against V4's 4 bits/base everywhere. **V4**
//! ([`BaseSet`](alphabet::BaseSet)) is kept as the *query/answer* algebra —
//! union, intersection and subset over possible bases — not as storage.
//! See `docs/OGAR-GENOM-CAPSTONE.md` for the measurements and the falsifiers.
//!
//! ## What this crate does NOT do
//!
//! No variants, transcripts, populations, lineage or ontology addresses (those
//! are later layers), no file-format parsers (adapters at the boundary), no
//! classid mints, and no SIMD: kernels here are word-level bit operations. SIMD
//! fast paths, when they land, come from `ndarray::simd` only.

pub mod alphabet;
pub mod coord;
pub mod sequence;
pub mod translate;
pub mod view;

pub use alphabet::{Base, BaseSet};
pub use coord::{Frame, Interval, Strand};
pub use sequence::{AmbiguityRun, EncodeError, PackedSeq, decode, encode};
pub use translate::{AminoAcids, CODON_TABLE, codon_index, translate};
pub use view::SeqView;
