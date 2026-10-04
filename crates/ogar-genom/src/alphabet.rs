//! The two alphabets: DNA2 [`Base`] (resident) and V4 [`BaseSet`] (ambiguity algebra).

/// One unambiguous nucleotide, in DNA2 order. Complement is `3 - b` (= `b ^ 0b11`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Base {
    A = 0,
    C = 1,
    G = 2,
    T = 3,
}

impl Base {
    pub const ALL: [Base; 4] = [Base::A, Base::C, Base::G, Base::T];

    /// From the 2-bit code; only the low two bits are read.
    #[inline]
    pub const fn from_code(code: u8) -> Base {
        match code & 3 {
            0 => Base::A,
            1 => Base::C,
            2 => Base::G,
            _ => Base::T,
        }
    }

    #[inline]
    pub const fn code(self) -> u8 {
        self as u8
    }

    #[inline]
    pub const fn complement(self) -> Base {
        Base::from_code(3 - self as u8)
    }

    /// Upper-case ASCII letter.
    #[inline]
    pub const fn to_ascii(self) -> u8 {
        b"ACGT"[self as usize]
    }

    /// From an unambiguous ASCII base (either case); `None` for anything else.
    #[inline]
    pub const fn from_ascii(c: u8) -> Option<Base> {
        match c {
            b'A' | b'a' => Some(Base::A),
            b'C' | b'c' => Some(Base::C),
            b'G' | b'g' => Some(Base::G),
            b'T' | b't' => Some(Base::T),
            _ => None,
        }
    }
}

/// V4: a nibble naming the set of possible bases. Bit `b` set ⇔ `Base` with code `b` is possible.
///
/// `A=0001 C=0010 G=0100 T=1000`; `R=A|G=0101`, `Y=C|T=1010`, `N=1111`.
/// The empty set (`0000`) is representable and means "contradiction"; it is
/// never produced by IUPAC decoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BaseSet(u8);

impl BaseSet {
    pub const EMPTY: BaseSet = BaseSet(0);
    pub const ANY: BaseSet = BaseSet(0b1111);

    /// From the raw nibble; bits above 3 are discarded.
    #[inline]
    pub const fn from_bits(bits: u8) -> BaseSet {
        BaseSet(bits & 0b1111)
    }

    #[inline]
    pub const fn bits(self) -> u8 {
        self.0
    }

    #[inline]
    pub const fn single(b: Base) -> BaseSet {
        BaseSet(1 << b as u8)
    }

    #[inline]
    pub const fn union(self, o: BaseSet) -> BaseSet {
        BaseSet(self.0 | o.0)
    }

    #[inline]
    pub const fn intersection(self, o: BaseSet) -> BaseSet {
        BaseSet(self.0 & o.0)
    }

    #[inline]
    pub const fn is_subset_of(self, o: BaseSet) -> bool {
        self.0 & !o.0 == 0
    }

    #[inline]
    pub const fn contains(self, b: Base) -> bool {
        self.0 & (1 << b as u8) != 0
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn len(self) -> u32 {
        self.0.count_ones()
    }

    /// The single base, if the set has exactly one member.
    #[inline]
    pub const fn as_single(self) -> Option<Base> {
        match self.0 {
            0b0001 => Some(Base::A),
            0b0010 => Some(Base::C),
            0b0100 => Some(Base::G),
            0b1000 => Some(Base::T),
            _ => None,
        }
    }

    /// Complement of every member (A↔T, C↔G) — the set on the other strand.
    /// Reverses the nibble's bit order, because complement is `3 - code`.
    #[inline]
    pub const fn complement(self) -> BaseSet {
        let b = self.0;
        BaseSet(((b & 1) << 3) | ((b & 2) << 1) | ((b & 4) >> 1) | ((b & 8) >> 3))
    }

    /// Iterate the members in code order.
    pub fn iter(self) -> impl Iterator<Item = Base> {
        Base::ALL.into_iter().filter(move |&b| self.contains(b))
    }

    /// From an IUPAC nucleotide letter (either case; `U` reads as `T`).
    pub const fn from_iupac(c: u8) -> Option<BaseSet> {
        let bits = match c.to_ascii_uppercase() {
            b'A' => 0b0001,
            b'C' => 0b0010,
            b'G' => 0b0100,
            b'T' | b'U' => 0b1000,
            b'R' => 0b0101,
            b'Y' => 0b1010,
            b'S' => 0b0110,
            b'W' => 0b1001,
            b'K' => 0b1100,
            b'M' => 0b0011,
            b'B' => 0b1110,
            b'D' => 0b1101,
            b'H' => 0b1011,
            b'V' => 0b0111,
            b'N' => 0b1111,
            _ => return None,
        };
        Some(BaseSet(bits))
    }

    /// Upper-case IUPAC letter. The empty set has no letter and returns `b'-'`
    /// (IUPAC's gap symbol); callers that can produce it should check first.
    pub const fn to_iupac(self) -> u8 {
        b"-ACMGRSVTWYHKDBN"[self.0 as usize]
    }
}
