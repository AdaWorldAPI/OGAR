//! Out-of-line value storage. Strings, binary values (`objectSid`) and
//! multi-valued attributes (`proxyAddresses`) live here; the fixed record
//! carries only a [`StrRef`] per attribute slot.
//!
//! A pool belongs to a batch of records (one column chunk). A [`StrRef`] is
//! meaningless without the pool it was issued by. Values are stored byte-exact
//! as observed — no case folding, trimming or normalisation.
//!
//! Multi-valued layout inside the pool: `(u32 LE length, bytes)*`, in source
//! order. Whether a slot is single- or multi-valued is a property of the
//! schema ([`AttrKind`](crate::schema::AttrKind)), not of the bytes.

/// `(offset, length)` into a [`ValuePool`], both u32 LE on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct StrRef {
    /// Byte offset into the pool.
    pub off: u32,
    /// Byte length.
    pub len: u32,
}

impl StrRef {
    /// 8 wire bytes.
    pub fn to_le_bytes(self) -> [u8; 8] {
        let mut b = [0u8; 8];
        b[..4].copy_from_slice(&self.off.to_le_bytes());
        b[4..].copy_from_slice(&self.len.to_le_bytes());
        b
    }
    /// From 8 wire bytes.
    pub fn from_le_bytes(b: [u8; 8]) -> Self {
        Self {
            off: u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            len: u32::from_le_bytes([b[4], b[5], b[6], b[7]]),
        }
    }
}

/// Pool failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    /// The pool would exceed 4 GiB (u32 offsets).
    Overflow,
}

/// Append-only byte arena.
#[derive(Debug, Default, Clone)]
pub struct ValuePool {
    bytes: Vec<u8>,
}

impl ValuePool {
    /// Empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    fn reserve_ref(&self, len: usize) -> Result<StrRef, PoolError> {
        let off = u32::try_from(self.bytes.len()).map_err(|_| PoolError::Overflow)?;
        let len32 = u32::try_from(len).map_err(|_| PoolError::Overflow)?;
        off.checked_add(len32).ok_or(PoolError::Overflow)?;
        Ok(StrRef { off, len: len32 })
    }

    /// Store one value.
    pub fn push(&mut self, v: &[u8]) -> Result<StrRef, PoolError> {
        let r = self.reserve_ref(v.len())?;
        self.bytes.extend_from_slice(v);
        Ok(r)
    }

    /// Store a multi-valued attribute.
    pub fn push_multi<V: AsRef<[u8]>>(&mut self, vs: &[V]) -> Result<StrRef, PoolError> {
        let total: usize = vs.iter().map(|v| 4 + v.as_ref().len()).sum();
        let r = self.reserve_ref(total)?;
        for v in vs {
            let v = v.as_ref();
            self.bytes
                .extend_from_slice(&(v.len() as u32).to_le_bytes());
            self.bytes.extend_from_slice(v);
        }
        Ok(r)
    }

    /// The bytes behind a ref, `None` if out of range.
    pub fn get(&self, r: StrRef) -> Option<&[u8]> {
        let s = r.off as usize;
        self.bytes.get(s..s.checked_add(r.len as usize)?)
    }

    /// Decode a multi-valued ref, `None` if malformed or out of range.
    /// Member `n` (1-based) of a bag slot ([`crate::AttrKind::Bag`]);
    /// `None` if out of range or absent (empty).
    pub fn bag_member(&self, r: StrRef, n: usize) -> Option<&[u8]> {
        let m = *self.get_multi(r)?.get(n.checked_sub(1)?)?;
        (!m.is_empty()).then_some(m)
    }

    pub fn get_multi(&self, r: StrRef) -> Option<Vec<&[u8]>> {
        let mut b = self.get(r)?;
        let mut out = Vec::new();
        while !b.is_empty() {
            let n = u32::from_le_bytes(b.get(..4)?.try_into().ok()?) as usize;
            out.push(b.get(4..4 + n)?);
            b = &b[4 + n..];
        }
        Some(out)
    }

    /// Raw pool bytes (the column payload).
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_and_multi_round_trip() {
        let mut p = ValuePool::new();
        let a = p.push("SMTP:jan@example.de".as_bytes()).unwrap();
        let m = p.push_multi(&["SMTP:a@x", "smtp:b@x", ""]).unwrap();
        assert_eq!(p.get(a).unwrap(), b"SMTP:jan@example.de");
        assert_eq!(
            p.get_multi(m).unwrap(),
            vec![&b"SMTP:a@x"[..], b"smtp:b@x", b""]
        );
        assert_eq!(StrRef::from_le_bytes(m.to_le_bytes()), m);
        assert!(p.get(StrRef { off: 1000, len: 1 }).is_none());
    }
}
