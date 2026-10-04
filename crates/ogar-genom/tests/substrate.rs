//! Falsifiers for the first ogar-genom proof. Every check compares against an
//! independent derivation (a naive u8 implementation or a hand-written table),
//! never against the code under test.

use ogar_genom::*;

/// Independent naive reference: u8 reverse complement over IUPAC letters.
fn naive_revcomp(s: &[u8]) -> Vec<u8> {
    s.iter()
        .rev()
        .map(|&c| {
            BaseSet::from_iupac(c)
                .expect("fixture is IUPAC")
                .complement()
                .to_iupac()
        })
        .collect()
}

/// Independent naive translation over unambiguous upper-case ACGT.
fn naive_translate(s: &[u8], frame: usize) -> Vec<u8> {
    // hand-written ACGT-ordered standard code (not derived from CODON_TABLE)
    const T: &[u8; 64] = b"KNKNTTTTRSRSIIMIQHQHPPPPRRRRLLLLEDEDAAAAGGGGVVVV*Y*YSSSS*CWCLFLF";
    let code = |c: u8| b"ACGT".iter().position(|&x| x == c).unwrap();
    s[frame..]
        .as_chunks::<3>()
        .0
        .iter()
        .map(|w| T[code(w[0]) * 16 + code(w[1]) * 4 + code(w[2])])
        .collect()
}

fn lcg_seq(n: usize, seed: u64) -> Vec<u8> {
    let mut x = seed;
    (0..n)
        .map(|_| {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            b"ACGT"[(x >> 62) as usize]
        })
        .collect()
}

#[test]
fn round_trip_is_exact_including_case_and_ambiguity() {
    let fixtures: &[&[u8]] = &[
        b"",
        b"A",
        b"ACGTacgt",
        b"NNNNNNNNNNacgtNNNNnnnnACGTRYKMSWBDHVrykmswbdhvN",
        b"NNNRRRNNN",                         // adjacent runs of different symbols
        b"ACGTACGTACGTACGTACGTACGTACGTACGTA", // 33: crosses the first word boundary
    ];
    for &f in fixtures {
        let p = encode(f).unwrap();
        assert_eq!(decode(&p), f, "fixture {:?}", String::from_utf8_lossy(f));
        assert_eq!(p.len(), f.len() as u64);
    }
    let long = lcg_seq(10_007, 7);
    assert_eq!(decode(&encode(&long).unwrap()), long);
}

#[test]
fn sidecars_are_maximal_runs() {
    let p = encode(b"ACNNNNRRgtNN").unwrap();
    let runs: Vec<_> = p
        .ambiguity_runs()
        .iter()
        .map(|r| (r.interval.start(), r.interval.end(), r.set.to_iupac()))
        .collect();
    assert_eq!(runs, vec![(2, 6, b'N'), (6, 8, b'R'), (10, 12, b'N')]);
    let soft: Vec<_> = p.soft_mask().iter().map(|i| (i.start(), i.end())).collect();
    assert_eq!(soft, vec![(8, 10)]);
}

#[test]
fn invalid_symbols_are_refused_with_position() {
    for (s, pos, byte) in [
        (&b"ACGU"[..], 3u64, b'U'),
        (b"AC-G", 2, b'-'),
        (b"AC\nG", 2, b'\n'),
        (b"AXG", 1, b'X'),
    ] {
        assert_eq!(encode(s), Err(EncodeError::InvalidSymbol { pos, byte }));
    }
}

#[test]
fn v4_algebra_over_all_iupac_letters() {
    let letters = b"ACGTRYSWKMBDHVN";
    for &c in letters {
        let s = BaseSet::from_iupac(c).unwrap();
        assert_eq!(s.to_iupac(), c, "letter round trip");
        assert_eq!(
            s.complement().complement(),
            s,
            "complement is an involution"
        );
        // complement maps each member to its complement base
        let expect = s.iter().fold(BaseSet::EMPTY, |acc, b| {
            acc.union(BaseSet::single(b.complement()))
        });
        assert_eq!(s.complement(), expect);
    }
    let r = BaseSet::from_iupac(b'R').unwrap(); // A|G
    let y = BaseSet::from_iupac(b'Y').unwrap(); // C|T
    assert!(r.intersection(y).is_empty());
    assert_eq!(r.union(y), BaseSet::ANY);
    assert!(BaseSet::single(Base::A).is_subset_of(r));
    assert!(!BaseSet::single(Base::C).is_subset_of(r));
    assert_eq!(r.complement(), y);
    assert_eq!(BaseSet::EMPTY.to_iupac(), b'-');
}

#[test]
fn codon_table_matches_an_independent_hand_written_table() {
    const HAND: &[u8; 64] = b"KNKNTTTTRSRSIIMIQHQHPPPPRRRRLLLLEDEDAAAAGGGGVVVV*Y*YSSSS*CWCLFLF";
    assert_eq!(&CODON_TABLE, HAND);
    for a in Base::ALL {
        for b in Base::ALL {
            for c in Base::ALL {
                let s = [a.to_ascii(), b.to_ascii(), c.to_ascii()];
                assert_eq!(
                    CODON_TABLE[codon_index(a, b, c) as usize],
                    naive_translate(&s, 0)[0]
                );
            }
        }
    }
    assert_eq!(CODON_TABLE.iter().filter(|&&x| x == b'*').count(), 3);
    assert_eq!(
        CODON_TABLE[codon_index(Base::A, Base::T, Base::G) as usize],
        b'M'
    );
}

#[test]
fn reverse_complement_is_a_view() {
    let src = b"AACGTTTGCAnRYacg";
    let p = encode(src).unwrap();
    let v = SeqView::new(&p);
    let rc = v.reverse_complement();
    assert_eq!(rc.materialize_iupac(), naive_revcomp(src));
    assert_eq!(
        rc.reverse_complement().materialize_iupac(),
        v.materialize_iupac()
    );
    assert!(
        core::ptr::eq(rc.sequence(), &p),
        "same resident sequence, nothing copied"
    );
    // soft mask follows the reference position, not the view position
    assert!(rc.is_soft_masked(0) && !rc.is_soft_masked(3));
}

#[test]
fn sub_views_compose_on_both_strands() {
    let src = lcg_seq(200, 3);
    let p = encode(&src).unwrap();
    let v = SeqView::new(&p);
    let iv = Interval::new(37, 151).unwrap();
    let fwd = v.sub(iv).unwrap();
    assert_eq!(fwd.materialize_iupac(), &src[37..151]);
    let rc_sub = v.reverse_complement().sub(iv).unwrap();
    let full_rc = naive_revcomp(&src);
    assert_eq!(rc_sub.materialize_iupac(), &full_rc[37..151]);
    // sub of a reverse view == reverse of the matching forward sub
    let inner = Interval::new(10, 40).unwrap();
    assert_eq!(
        rc_sub.sub(inner).unwrap().materialize_iupac(),
        &full_rc[47..77]
    );
    assert!(v.sub(Interval::new(0, 201).unwrap()).is_none());
}

#[test]
fn all_frames_both_strands_across_word_boundaries_match_naive() {
    for (n, seed) in [
        (5usize, 1u64),
        (31, 2),
        (32, 3),
        (33, 4),
        (64, 5),
        (97, 6),
        (1000, 7),
        (4099, 8),
    ] {
        let src = lcg_seq(n, seed);
        let p = encode(&src).unwrap();
        let v = SeqView::new(&p);
        let rc = naive_revcomp(&src);
        for f in 0..3u8 {
            let frame = Frame::new(f).unwrap();
            if n >= f as usize {
                assert_eq!(
                    translate(v, frame).collect::<Vec<_>>(),
                    naive_translate(&src, f as usize),
                    "fwd n={n} f={f}"
                );
                assert_eq!(
                    translate(v.reverse_complement(), frame).collect::<Vec<_>>(),
                    naive_translate(&rc, f as usize),
                    "rev n={n} f={f}"
                );
            }
        }
    }
}

#[test]
fn known_fixtures_translate() {
    // Human preproinsulin (INS) CDS, first 10 codons: MALWMRLLPL
    let ins = encode(b"ATGGCCCTGTGGATGCGCCTCCTGCCCCTG").unwrap();
    assert_eq!(
        translate(SeqView::new(&ins), Frame::ZERO).collect::<Vec<_>>(),
        b"MALWMRLLPL"
    );
    let t = encode(b"ATGGCCATTGTAATGGGCCGCTGAAAGGGTGCCCGATAG").unwrap();
    assert_eq!(
        translate(SeqView::new(&t), Frame::ZERO).collect::<Vec<_>>(),
        b"MAIVMGR*KGAR*"
    );
}

#[test]
fn ambiguity_translates_only_when_determinate() {
    let cases: &[(&[u8], &[u8])] = &[
        (b"GCN", b"A"), // fourfold degenerate site
        (b"GAR", b"E"), // GAA, GAG
        (b"GAN", b"X"), // D or E
        (b"NNN", b"X"),
        (b"TRA", b"X"), // TAA stop or TGA stop? both '*' -> determinate stop
    ];
    for &(s, want) in &cases[..4] {
        let p = encode(s).unwrap();
        assert_eq!(
            translate(SeqView::new(&p), Frame::ZERO).collect::<Vec<_>>(),
            want,
            "{}",
            String::from_utf8_lossy(s)
        );
    }
    // TRA = TAA | TGA: both stop -> '*', not 'X'
    let p = encode(cases[4].0).unwrap();
    assert_eq!(
        translate(SeqView::new(&p), Frame::ZERO).collect::<Vec<_>>(),
        b"*"
    );
}

#[test]
fn inverted_intervals_cannot_be_constructed() {
    // Fields are private: `Interval::new` is the only public constructor.
    assert!(Interval::new(5, 3).is_none());
    let empty = Interval::new(4, 4).unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    let p = encode(b"ACGTACGT").unwrap();
    let v = SeqView::of(&p, Interval::new(2, 6).unwrap(), Strand::Reverse).unwrap();
    assert_eq!(v.len(), 4);
    assert_eq!(v.sub(Interval::new(0, 4).unwrap()).unwrap().len(), 4);
}
