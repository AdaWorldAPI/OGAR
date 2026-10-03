//! RFC 4514 Distinguished Name parsing — just enough to split a DN into its
//! leaf, its location (OU chain) and its naming context (DC chain).
//!
//! A DN is never identity here. It is *evidence about location*: the OU
//! components feed the [`OuHhtl`](crate::hhtl::OuHhtl); the leaf `CN=` and the
//! `DC=` components never do.
//!
//! Supported: `\,` style escapes, `\HH` hex escapes (multi-byte UTF-8 via
//! consecutive pairs, e.g. `H\C3\BCbener`), optional spaces around `,` and `=`.
//! Rejected (returned as [`DnError`], never guessed): multi-valued RDNs (`+`),
//! empty components, dangling escapes, non-UTF-8 hex payloads.

use core::fmt;

/// One `type=value` component, value unescaped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rdn {
    /// Attribute type as written (`OU`, `ou`, `CN`, `DC`, …).
    pub attr: String,
    /// Unescaped value.
    pub value: String,
}

impl Rdn {
    /// Case-insensitive attribute-type test.
    pub fn is(&self, attr: &str) -> bool {
        self.attr.eq_ignore_ascii_case(attr)
    }
}

/// A parsed DN, components in written order (leaf first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dn {
    /// Components, leaf first — exactly as written.
    pub rdns: Vec<Rdn>,
}

/// Why a DN could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnError {
    /// Empty input.
    Empty,
    /// A component had no `=`.
    MissingEquals(usize),
    /// A component had an empty type or value.
    EmptyComponent(usize),
    /// `+` multi-valued RDNs are out of PoC scope.
    MultiValuedRdn(usize),
    /// Backslash at end, or an invalid hex escape.
    BadEscape(usize),
    /// Hex escapes did not form valid UTF-8.
    NotUtf8(usize),
}

impl fmt::Display for DnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid distinguished name: {self:?}")
    }
}
impl std::error::Error for DnError {}

fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

impl Dn {
    /// Parse an RFC 4514 DN string.
    pub fn parse(s: &str) -> Result<Self, DnError> {
        if s.trim().is_empty() {
            return Err(DnError::Empty);
        }
        let b = s.as_bytes();
        let mut rdns = Vec::new();
        let mut i = 0usize;
        while i <= b.len() {
            let comp = rdns.len();
            // attribute type: up to '='
            let start = i;
            while i < b.len() && b[i] != b'=' && b[i] != b',' {
                i += 1;
            }
            if i >= b.len() || b[i] != b'=' {
                return Err(DnError::MissingEquals(comp));
            }
            let attr = s[start..i].trim().to_string();
            i += 1; // '='
            while i < b.len() && b[i] == b' ' {
                i += 1;
            }
            // value: until unescaped ','
            let mut val: Vec<u8> = Vec::new();
            let mut trailing_unescaped_spaces = 0usize;
            while i < b.len() && b[i] != b',' {
                match b[i] {
                    b'\\' => {
                        let n = *b.get(i + 1).ok_or(DnError::BadEscape(comp))?;
                        if let Some(h) = hex(n) {
                            let l = b
                                .get(i + 2)
                                .copied()
                                .and_then(hex)
                                .ok_or(DnError::BadEscape(comp))?;
                            val.push(h << 4 | l);
                            i += 3;
                        } else {
                            val.push(n);
                            i += 2;
                        }
                        trailing_unescaped_spaces = 0;
                    }
                    b'+' => return Err(DnError::MultiValuedRdn(comp)),
                    c => {
                        val.push(c);
                        trailing_unescaped_spaces = if c == b' ' {
                            trailing_unescaped_spaces + 1
                        } else {
                            0
                        };
                        i += 1;
                    }
                }
            }
            val.truncate(val.len() - trailing_unescaped_spaces);
            let value = String::from_utf8(val).map_err(|_| DnError::NotUtf8(comp))?;
            if attr.is_empty() || value.is_empty() {
                return Err(DnError::EmptyComponent(comp));
            }
            rdns.push(Rdn { attr, value });
            if i >= b.len() {
                break;
            }
            i += 1; // ','
        }
        Ok(Self { rdns })
    }

    /// The leaf component (first written). Never part of the HHTL.
    pub fn leaf(&self) -> &Rdn {
        &self.rdns[0]
    }

    /// OU values, **root first** (the reverse of written order), excluding the
    /// leaf even if the leaf itself is an `OU=` (an OU object's own name is its
    /// leaf, not its location).
    pub fn ou_path_root_first(&self) -> Vec<&str> {
        self.rdns[1..]
            .iter()
            .rev()
            .filter(|r| r.is("OU"))
            .map(|r| r.value.as_str())
            .collect()
    }

    /// `DC=` values in written order (`["example", "de"]`).
    pub fn dc_components(&self) -> Vec<&str> {
        self.rdns
            .iter()
            .filter(|r| r.is("DC"))
            .map(|r| r.value.as_str())
            .collect()
    }

    /// True when the location (all non-leaf components) contains something that
    /// is neither `OU=` nor `DC=` — e.g. the `CN=Users` container. Such objects
    /// have a location the OU-HHTL does not express; the record flags it rather
    /// than conflating it with "directly under the domain root".
    pub fn has_non_ou_container(&self) -> bool {
        self.rdns[1..].iter().any(|r| !r.is("OU") && !r.is("DC"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_leaf_ou_and_dc() {
        let dn =
            Dn::parse("CN=Jan Hübener,OU=Exchange,OU=Infrastructure,OU=Stuttgart,DC=example,DC=de")
                .unwrap();
        assert_eq!(dn.leaf().value, "Jan Hübener");
        assert_eq!(
            dn.ou_path_root_first(),
            ["Stuttgart", "Infrastructure", "Exchange"]
        );
        assert_eq!(dn.dc_components(), ["example", "de"]);
        assert!(!dn.has_non_ou_container());
    }

    #[test]
    fn handles_escapes_and_spaces() {
        let dn =
            Dn::parse(r"CN=Doe\, John , OU = Sales\2C EMEA, OU=H\C3\BCbener\20, DC=x").unwrap();
        assert_eq!(dn.leaf().value, "Doe, John");
        assert_eq!(dn.ou_path_root_first(), ["Hübener ", "Sales, EMEA"]);
    }

    #[test]
    fn flags_cn_containers() {
        let dn = Dn::parse("CN=Jan,CN=Users,DC=example,DC=de").unwrap();
        assert!(dn.ou_path_root_first().is_empty());
        assert!(dn.has_non_ou_container());
    }

    #[test]
    fn rejects_out_of_scope_or_broken_input() {
        assert_eq!(Dn::parse(""), Err(DnError::Empty));
        assert!(matches!(
            Dn::parse("CN=a+UID=b,DC=x"),
            Err(DnError::MultiValuedRdn(0))
        ));
        assert!(matches!(
            Dn::parse("CN=a,OU,DC=x"),
            Err(DnError::MissingEquals(1))
        ));
        assert!(matches!(Dn::parse(r"CN=a\"), Err(DnError::BadEscape(0))));
        assert!(matches!(
            Dn::parse("CN=,DC=x"),
            Err(DnError::EmptyComponent(0))
        ));
        assert!(matches!(
            Dn::parse(r"CN=\FF\FE,DC=x"),
            Err(DnError::NotUtf8(0))
        ));
    }
}
