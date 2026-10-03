//! Minimal read-only LDIF (RFC 2849) reader for `ldapsearch -LLL` /
//! `ldifde -f` output.
//!
//! Supported: `#` comments, a leading `version:` line, folded lines (a line
//! starting with one space continues the previous one), `attr: value`,
//! `attr:: base64` (binary or non-ASCII values — this is how `objectGUID`
//! arrives), blank-line entry separation. Rejected: `attr:< URL` and
//! change records (`changetype:`) — this is an observation reader.

use crate::AdEntry;
use ogar_dir_core::base64;

/// LDIF failure with 1-based line number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LdifError {
    /// A line has no `:`.
    NoColon(usize),
    /// Invalid base64 payload.
    BadBase64(usize),
    /// `:<` URL value or `changetype` record.
    Unsupported(usize),
    /// An entry without a `dn:` first line.
    MissingDn(usize),
    /// A base64-encoded DN was not UTF-8.
    DnNotUtf8(usize),
}

/// Parse LDIF text into entries, in file order.
pub fn parse(text: &str) -> Result<Vec<AdEntry>, LdifError> {
    // Unfold first, remembering the starting line number of each logical line.
    let mut logical: Vec<(usize, String)> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(cont) = line.strip_prefix(' ')
            && let Some((_, last)) = logical.last_mut()
        {
            last.push_str(cont);
            continue;
        }
        logical.push((i + 1, line.to_string()));
    }

    let mut out = Vec::new();
    let mut cur: Option<AdEntry> = None;
    for (ln, line) in logical {
        if line.starts_with('#') {
            continue;
        }
        if line.is_empty() {
            if let Some(e) = cur.take() {
                out.push(e);
            }
            continue;
        }
        let (name, rest) = line.split_once(':').ok_or(LdifError::NoColon(ln))?;
        let value: Vec<u8> = if let Some(b) = rest.strip_prefix(':') {
            base64::decode(b.trim()).ok_or(LdifError::BadBase64(ln))?
        } else if rest.starts_with('<') {
            return Err(LdifError::Unsupported(ln));
        } else {
            rest.strip_prefix(' ').unwrap_or(rest).as_bytes().to_vec()
        };
        if name.eq_ignore_ascii_case("version") && cur.is_none() && out.is_empty() {
            continue;
        }
        if name.eq_ignore_ascii_case("changetype") {
            return Err(LdifError::Unsupported(ln));
        }
        match cur.as_mut() {
            None => {
                if !name.eq_ignore_ascii_case("dn") {
                    return Err(LdifError::MissingDn(ln));
                }
                let dn = String::from_utf8(value).map_err(|_| LdifError::DnNotUtf8(ln))?;
                cur = Some(AdEntry {
                    dn,
                    attrs: Vec::new(),
                });
            }
            Some(e) => e.attrs.push((name.to_string(), value)),
        }
    }
    if let Some(e) = cur {
        out.push(e);
    }
    Ok(out)
}
