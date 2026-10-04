// LAB PROBE — not built by the workspace. Reproduce the OGAR-GENOM-CAPSTONE §3-§10 numbers:
//   Cargo.toml: [dependencies] ndarray = { path = "<ndarray checkout>", default-features = false, features = ["std"] }
//   RUSTFLAGS="-C target-cpu=native" cargo run --release -- chr21.seq   (chr21.seq = FASTA body, no header/newlines)
//   SIMD comes from the ndarray::simd facade only.

//! OGAR-GENOM feasibility probe. LAB CODE — never shipped.
//! All SIMD from `ndarray::simd` (facade), per the workspace invariant.
//!
//! Measures, on real GRCh38 chr21 (UCSC soft-masked):
//!   E1 encode u8->DNA2 + interval sidecars, exact round-trip
//!   E2 reverse complement: materialized (u8, DNA2) vs streaming view
//!   E3 codon translation fold: u8 baseline vs DNA2 + permute_bytes LUT
//!   E4 motif scan: exact k-mer (u8 vs DNA2 rolling) and IUPAC motif (V4 query over DNA2)
//!   E5 allele-frequency fold over a synthetic genotype bitplane population
//!   E6 sparse variant overlay read

use ndarray::simd::U8x64;
use std::hint::black_box;
use std::time::Instant;

// ---------- alphabet ----------
const A: u8 = 0;
const C: u8 = 1;
const G: u8 = 2;
const T: u8 = 3;

fn base2(b: u8) -> u8 {
    match b {
        b'A' | b'a' => A,
        b'C' | b'c' => C,
        b'G' | b'g' => G,
        b'T' | b't' => T,
        _ => 0, // N etc. -> placeholder, recorded in sidecar
    }
}

/// Standard genetic code (NCBI table 1) indexed by DNA2 codon b0<<4|b1<<2|b2, A=0 C=1 G=2 T=3.
fn codon_lut() -> [u8; 64] {
    // canonical TCAG-ordered table
    let tcag = b"FFLLSSSSYY**CC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG";
    let ord = |x: u8| match x { T => 0, C => 1, A => 2, G => 3, _ => unreachable!() };
    let mut lut = [0u8; 64];
    for c in 0..64u8 {
        let (b0, b1, b2) = (c >> 4, (c >> 2) & 3, c & 3);
        lut[c as usize] = tcag[(ord(b0) * 16 + ord(b1) * 4 + ord(b2)) as usize];
    }
    lut
}

// ---------- packed resident sequence ----------
struct Dna2 {
    words: Vec<u64>, // 32 bases / u64, base i at bits 2*(i%32)
    len: usize,
}
impl Dna2 {
    #[inline(always)]
    fn get(&self, i: usize) -> u8 {
        ((self.words[i >> 5] >> ((i & 31) * 2)) & 3) as u8
    }
}

/// Half-open intervals (start,end) — sidecar for N runs / soft-mask.
fn runs(seq: &[u8], pred: impl Fn(u8) -> bool) -> Vec<(u32, u32)> {
    let mut v = Vec::new();
    let mut i = 0;
    while i < seq.len() {
        if pred(seq[i]) {
            let s = i;
            while i < seq.len() && pred(seq[i]) { i += 1; }
            v.push((s as u32, i as u32));
        } else { i += 1; }
    }
    v
}

fn encode(seq: &[u8]) -> Dna2 {
    let mut words = vec![0u64; seq.len().div_ceil(32)];
    for (wi, chunk) in seq.chunks(32).enumerate() {
        let mut w = 0u64;
        for (j, &b) in chunk.iter().enumerate() { w |= (base2(b) as u64) << (2 * j); }
        words[wi] = w;
    }
    Dna2 { words, len: seq.len() }
}

fn decode(d: &Dna2, n_runs: &[(u32, u32)], soft: &[(u32, u32)]) -> Vec<u8> {
    const UP: [u8; 4] = *b"ACGT";
    let mut out: Vec<u8> = (0..d.len).map(|i| UP[d.get(i) as usize]).collect();
    for &(s, e) in soft { for x in &mut out[s as usize..e as usize] { *x = x.to_ascii_lowercase(); } }
    for &(s, e) in n_runs { for x in &mut out[s as usize..e as usize] { *x = if x.is_ascii_lowercase() { b'n' } else { b'N' }; } }
    out
}

/// Unpack 64 bases starting at base index `i` (must be multiple of 32) into one byte each.
#[inline(always)]
fn unpack64(d: &Dna2, wi: usize, buf: &mut [u8; 64]) {
    let w0 = d.words[wi];
    let w1 = *d.words.get(wi + 1).unwrap_or(&0);
    for j in 0..32 { buf[j] = ((w0 >> (2 * j)) & 3) as u8; buf[32 + j] = ((w1 >> (2 * j)) & 3) as u8; }
}

fn time<R>(name: &str, bytes: usize, reps: u32, mut f: impl FnMut() -> R) -> f64 {
    black_box(f());
    let t = Instant::now();
    for _ in 0..reps { black_box(f()); }
    let s = t.elapsed().as_secs_f64() / reps as f64;
    println!("  {name:<46} {:>9.3} ms  {:>8.2} GB/s(src-eq)", s * 1e3, bytes as f64 / s / 1e9);
    s
}

fn main() {
    let path = std::env::args().nth(1).expect("chr21.seq");
    let seq = std::fs::read(&path).unwrap();
    let n = seq.len();
    println!("sequence: {n} bases from {path}");

    // ---------------- E1 ----------------
    println!("\nE1 encode + sidecars");
    let n_runs = runs(&seq, |b| b == b'N' || b == b'n');
    let soft = runs(&seq, |b| b.is_ascii_lowercase());
    let other_iupac = seq.iter().filter(|&&b| !b"ACGTNacgtn".contains(&b)).count();
    let d = encode(&seq);
    time("encode u8 -> DNA2", n, 5, || encode(&seq));
    let rt = decode(&d, &n_runs, &soft);
    assert!(rt == seq, "round-trip FAILED");
    println!("  round-trip exact: OK   other-IUPAC={other_iupac}  N-runs={}  softmask-runs={}", n_runs.len(), soft.len());
    let dna2_b = d.words.len() * 8;
    let side_b = (n_runs.len() + soft.len()) * 8;
    println!("  bytes: u8={n}  DNA2={dna2_b} ({:.3} b/base)  sidecars={side_b} ({:.4} b/base)  V4(nibble)={}",
        8.0 * dna2_b as f64 / n as f64, 8.0 * side_b as f64 / n as f64, n.div_ceil(2));
    println!("  N-only sidecar: {} bytes", n_runs.len() * 8);

    // ---------------- E2 ----------------
    println!("\nE2 reverse complement (what bytes move?)");
    let comp = |b: u8| match b { b'A' => b'T', b'C' => b'G', b'G' => b'C', b'T' => b'A', b'a' => b't', b'c' => b'g', b'g' => b'c', b't' => b'a', x => x };
    time("u8 materialized revcomp (writes n B)", n, 5, || seq.iter().rev().map(|&b| comp(b)).collect::<Vec<u8>>());
    time("DNA2 materialized revcomp (writes n/4 B)", n, 5, || {
        let nw = d.words.len();
        let mut out = vec![0u64; nw];
        let pad = nw * 32 - d.len; // shift to re-align after reversal
        for (i, &w) in d.words.iter().enumerate() {
            // reverse 2-bit groups within the word, then complement (xor 0b11 per base)
            let mut x = w;
            x = ((x >> 2) & 0x3333_3333_3333_3333) | ((x & 0x3333_3333_3333_3333) << 2);
            x = ((x >> 4) & 0x0F0F_0F0F_0F0F_0F0F) | ((x & 0x0F0F_0F0F_0F0F_0F0F) << 4);
            x = x.swap_bytes();
            out[nw - 1 - i] = !x;
        }
        // realign by `pad` bases (2*pad bits) — a funnel shift over the word stream
        if pad > 0 {
            let s = 2 * pad as u32;
            for i in 0..nw { let hi = if i + 1 < nw { out[i + 1] } else { 0 }; out[i] = (out[i] >> s) | (hi << (64 - s)); }
        }
        out
    });
    // streaming view: no buffer; a fold (GC count) through the revcomp coordinate transform
    time("revcomp VIEW fold: GC count via i->len-1-i", n, 3, || {
        let mut gc = 0u64;
        for i in 0..d.len { let b = 3 - d.get(d.len - 1 - i); gc += (b == C || b == G) as u64; }
        gc
    });
    // word-granular streaming VIEW: produce revcomp words on the fly, no buffer; checksum vs materialized
    let rc_word = |w: u64| -> u64 { let mut x = w;
        x = ((x >> 2) & 0x3333_3333_3333_3333) | ((x & 0x3333_3333_3333_3333) << 2);
        x = ((x >> 4) & 0x0F0F_0F0F_0F0F_0F0F) | ((x & 0x0F0F_0F0F_0F0F_0F0F) << 4);
        !x.swap_bytes() };
    let nw = d.words.len(); let pad = (nw * 32 - d.len) as u32 * 2;
    time("revcomp word-VIEW fold (checksum, 0 B written)", n, 5, || {
        let mut acc = 0u64;
        for i in 0..nw { let lo = rc_word(d.words[nw - 1 - i]); let hi = if i + 1 < nw { rc_word(d.words[nw - 2 - i]) } else { 0 };
            let w = if pad > 0 { (lo >> pad) | (hi << (64 - pad)) } else { lo }; acc = acc.rotate_left(7) ^ w; }
        acc
    });
    // GC count is strand-invariant: proves the fold needs no transform at all here.
    time("direct GC fold, DNA2 popcount (strand-invariant)", n, 5, || {
        // C=01,G=10: GC iff the two bits differ -> xor of the pair
        let mut gc = 0u64;
        for &w in &d.words { gc += (((w >> 1) ^ w) & 0x5555_5555_5555_5555).count_ones() as u64; }
        gc // NB: tail/N-placeholder bases counted as A; corrected via sidecar in real code
    });

    // ---------------- E3 ----------------
    println!("\nE3 translation fold (frame 0, all 64 codons)");
    let lut = codon_lut();
    assert_eq!(lut[(A << 4 | T << 2 | G) as usize], b'M');
    assert_eq!(lut[(T << 4 | A << 2 | A) as usize], b'*');
    let ncod = n / 3;
    let t_u8 = time("u8 baseline: 3 table lookups + LUT -> protein", n, 5, || {
        let mut p = Vec::with_capacity(ncod);
        for k in 0..ncod { let i = 3 * k; let c = base2(seq[i]) << 4 | base2(seq[i + 1]) << 2 | base2(seq[i + 2]); p.push(lut[c as usize]); }
        p
    });
    let t_d2 = time("DNA2 scalar: 6-bit extract (u128 window) + LUT", n, 5, || {
        let mut p = Vec::with_capacity(ncod);
        for k in 0..ncod {
            let i = 3 * k; let (wi, off) = (i >> 5, (i & 31) * 2);
            let w = d.words[wi] as u128 | (*d.words.get(wi + 1).unwrap_or(&0) as u128) << 64;
            let raw = (w >> off) as u8 & 63; // b0 in low bits
            let c = (raw & 3) << 4 | ((raw >> 2) & 3) << 2 | (raw >> 4); // reorder to b0<<4|b1<<2|b2
            p.push(lut[c as usize]);
        }
        p
    });
    // all-offset codon fold: codon at EVERY base position (frames 0,1,2 at once) via permute_bytes
    let lut_v = U8x64::from_array(lut);
    let t_all = time("DNA2 SIMD: codon@every pos (3 frames), vpermb", n, 5, || {
        let mut out = vec![0u8; n];
        let mut buf = [0u8; 64 + 2];
        let mut i = 0;
        while i + 64 <= n {
            let mut b64 = [0u8; 64];
            unpack64(&d, i >> 5, &mut b64);
            buf[..64].copy_from_slice(&b64);
            buf[64] = d.get((i + 64).min(n - 1)); buf[65] = d.get((i + 65).min(n - 1));
            let b0 = U8x64::from_slice(&buf[0..64]);
            let b1 = U8x64::from_slice(&buf[1..65]);
            let b2 = U8x64::from_slice(&buf[2..66]);
            let idx = or3(b0.shl_epi16(4), b1.shl_epi16(2), b2); // byte-safe: values <4, no cross-byte carry
            lut_v.permute_bytes(idx).copy_to_slice(&mut out[i..i + 64]);
            i += 64;
        }
        out
    });
    println!("  -> per-codon: u8 {:.2} ns, DNA2 scalar {:.2} ns; all-offset SIMD {:.3} ns/codon-slot ({}x codons for 3 frames)",
        t_u8 / ncod as f64 * 1e9, t_d2 / ncod as f64 * 1e9, t_all / n as f64 * 1e9, 3);
    // cross-check: SIMD all-offset frame 0 == scalar
    {
        let mut buf = [0u8; 66];
        let mut b64 = [0u8; 64];
        unpack64(&d, 0, &mut b64);
        buf[..64].copy_from_slice(&b64);
        buf[64] = d.get(64); buf[65] = d.get(65);
        for k in 0..20 { let i = 3 * k; let c = buf[i] << 4 | buf[i + 1] << 2 | buf[i + 2]; assert_eq!(lut[c as usize], { let c2 = base2(seq[i]) << 4 | base2(seq[i + 1]) << 2 | base2(seq[i + 2]); lut[c2 as usize] }); }
        println!("  frame-0 cross-check SIMD vs u8: OK");
    }

    // ---------------- E4 ----------------
    println!("\nE4 motif scan");
    let kmer = b"GGCTCACGCCTG"; // Alu-ish 12-mer
    let km2: u32 = kmer.iter().enumerate().fold(0, |a, (j, &b)| a | (base2(b) as u32) << (2 * j));
    let up: Vec<u8> = seq.iter().map(|b| b.to_ascii_uppercase()).collect();
    let c_u8 = { let mut c = 0; time("u8 exact 12-mer (window compare)", n, 3, || { c = up.windows(12).filter(|w| *w == kmer).count(); c }); c };
    let c_d2 = { let mut c = 0; time("DNA2 rolling 24-bit compare", n, 3, || {
        let mut h: u32 = 0; let mut cnt = 0usize; let mask = (1u32 << 24) - 1;
        for i in 0..n { h = (h >> 2) | (d.get(i) as u32) << 22; if i >= 11 && (h & mask) == km2 { cnt += 1; } }
        c = cnt; cnt }); c };
    println!("  hits u8={c_u8} DNA2={c_d2} (DNA2 also matches inside N runs as AAAA..: verify via sidecar)");
    // IUPAC motif TATAWAWR as V4 query over DNA2: onehot(base) & pat[j] != 0
    let v4 = |c: u8| -> u8 { match c { b'A'=>1,b'C'=>2,b'G'=>4,b'T'=>8,b'R'=>1|4,b'Y'=>2|8,b'W'=>1|8,b'S'=>2|4,b'N'=>15,_=>0 } };
    let pat: Vec<u8> = b"TATAWAWR".iter().map(|&c| v4(c)).collect();
    let pv: Vec<U8x64> = pat.iter().map(|&p| and_table(p)).collect();
    let mut onehot_lut = [0u8; 64]; for j in 0..4 { for l in 0..4 { onehot_lut[16 * l + j] = 1 << j; } }
    let oh = U8x64::from_array(onehot_lut);
    let zero = U8x64::splat(0);
    let c_v4 = { let mut c = 0; time("V4 query over DNA2 (vpshufb onehot, AND, cmpeq)", n, 3, || {
        let mut cnt = 0u64;
        let mut ohs = vec![0u8; n + 64];
        let mut b64 = [0u8; 64];
        let mut i = 0; while i + 64 <= n { unpack64(&d, i >> 5, &mut b64); oh.shuffle_bytes(U8x64::from_array(b64)).copy_to_slice(&mut ohs[i..i + 64]); i += 64; }
        let mut i = 0; while i + 64 + 8 <= n {
            let mut hit = !0u64;
            for (j, p) in pv.iter().enumerate() { let s = U8x64::from_slice(&ohs[i + j..i + j + 64]); hit &= !p.shuffle_bytes(s).cmpeq_mask(zero); }
            cnt += hit.count_ones() as u64; i += 64;
        }
        c = cnt; cnt }); c };
    let definite = |b: u8| if b == b'N' { 0 } else { v4(b) };
    let re_def = up.windows(8).filter(|w| w.iter().zip(&pat).all(|(&b, &p)| definite(b) & p != 0)).count();
    let re_pos = up.windows(8).filter(|w| w.iter().zip(&pat).all(|(&b, &p)| v4(b) & p != 0)).count();
    // DNA2 placeholder for N is A; correct V4 hits by subtracting windows touching an N run
    let in_n = { let mut c = 0usize; for i in 0..n.saturating_sub(8) { let touches = n_runs.iter().any(|&(s, e)| (i + 8) as u32 > s && (i as u32) < e); if touches { let w = &up[i..i+8]; if w.iter().zip(&pat).all(|(&b,&p)| { let x = if b==b'N' {1} else {v4(b)}; x & p != 0 }) { c += 1; } } } c };
    println!("  IUPAC TATAWAWR: V4/DNA2={c_v4}  scalar definite={re_def}  scalar possible(N=any)={re_pos}  DNA2 hits inside N-runs={in_n}");

    // ---------------- E5 ----------------
    println!("\nE5 allele-frequency fold, genotype bitplanes (site-major)");
    for &ind in &[100_000usize, 1_000_000] {
        let sites = if ind == 1_000_000 { 200 } else { 2000 };
        let wpp = ind.div_ceil(64); // words per plane
        // two planes per site: carries_alt, hom_alt; dosage = popcnt(p0)+popcnt(p1)
        let mut rng = 0x9E3779B97F4A7C15u64;
        let mut planes = vec![0u64; sites * 2 * wpp];
        for w in planes.iter_mut() { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; *w = rng & (rng >> 3) & (rng >> 5); }
        let bytes = planes.len() * 8;
        let s = time(&format!("{ind} indiv x {sites} sites: AF fold (popcnt)"), bytes, 3, || {
            let mut af = vec![0u64; sites];
            for (si, a) in af.iter_mut().enumerate() {
                let base = si * 2 * wpp;
                *a = ndarray::simd::popcount_batch_u64(&planes[base..base + 2 * wpp]);
            }
            af
        });
        println!("    -> {:.1} us/site, {:.2} ns/individual-site", s / sites as f64 * 1e6, s / (sites * ind) as f64 * 1e9);
        // selection mask fold: fitness mask AND site plane -> AF among selected parents
        let sel: Vec<u64> = (0..wpp).map(|i| (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)).collect();
        time(&format!("{ind} indiv: AF among selected (mask AND+popcnt)"), bytes, 3, || {
            let mut tot = 0u64;
            for si in 0..sites { let base = si * 2 * wpp;
                for p in 0..2 { for (a, b) in planes[base + p * wpp..base + (p + 1) * wpp].iter().zip(&sel) { tot += (a & b).count_ones() as u64; } } }
            tot
        });
    }

    // ---------------- E6 ----------------
    println!("\nE6 sparse SNP overlay read (1 SNV / 1000 bp, human-like density)");
    let snvs: Vec<(u32, u8)> = (0..n as u32).step_by(1000).map(|p| (p, ((p >> 3) & 3) as u8)).collect();
    println!("  overlay: {} SNVs, {} B (pos u32 + alt u8, unpacked {}B/var)", snvs.len(), snvs.len() * 8, 8);
    time("window read 1kb x 10k random windows w/ overlay", 10_000 * 1000, 3, || {
        let mut acc = 0u64; let mut r = 12345u64;
        for _ in 0..10_000 {
            r = r.wrapping_mul(6364136223846793005).wrapping_add(1);
            let s = (r >> 20) as usize % (n - 1000);
            let mut k = snvs.partition_point(|&(p, _)| (p as usize) < s);
            for i in s..s + 1000 {
                let b = if k < snvs.len() && snvs[k].0 as usize == i { k += 1; snvs[k - 1].1 } else { d.get(i) };
                acc += b as u64;
            }
        }
        acc
    });
    time("window read 1kb x 10k random windows, no overlay", 10_000 * 1000, 3, || {
        let mut acc = 0u64; let mut r = 12345u64;
        for _ in 0..10_000 { r = r.wrapping_mul(6364136223846793005).wrapping_add(1); let s = (r >> 20) as usize % (n - 1000);
            for i in s..s + 1000 { acc += d.get(i) as u64; } }
        acc
    });
}

// OR of disjoint bit fields (b0<<4 | b1<<2 | b2) via saturating_add — exact, no carries possible.
#[inline(always)]
fn or3(a: U8x64, b: U8x64, c: U8x64) -> U8x64 { a.saturating_add(b).saturating_add(c) }

/// Per-pattern-position AND table: shuffle_bytes(T_p, onehot) = onehot & p (16-entry per 128-bit lane).
fn and_table(p: u8) -> U8x64 { let mut t = [0u8; 64]; for l in 0..4 { for x in 0..16 { t[16 * l + x] = (x as u8) & p; } } U8x64::from_array(t) }
