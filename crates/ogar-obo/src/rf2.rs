//! **The RF2 snapshot reader** — one parser for every SNOMED release-format
//! file, so no consumer hand-rolls its own.
//!
//! RF2 snapshot files are TSV with a header row; the reader is deliberately
//! COLUMN-BLIND past the five columns every `der2_*` refset shares
//! (`id effectiveTime active moduleId refsetId`), because the refset family
//! is encoded in the FILENAME's pattern (`sRefset` = one string column,
//! `iissscc` = the extended-map shape, …) and a consumer knows which file it
//! opened. The reader's job is the part every consumer would otherwise
//! duplicate wrongly:
//!
//! - **the `active` filter** — an inactive row is history, not content, and
//!   a consumer that forgets the filter bakes retired mappings;
//! - **snapshot uniqueness** — a snapshot carries at most one row per `id`;
//!   a duplicate means the file is not a snapshot (a Full file, or a corrupt
//!   concatenation), and the reader REFUSES it rather than keeping either row;
//! - **column integrity** — every row must have exactly the header's arity
//!   (a permissive reader silently mis-aligns every column after a stray tab).
//!
//! Nothing here interprets meaning: no concept model, no map semantics, no
//! vocabulary. Types exist only before the bake; this is the join stage's
//! input surface.

use std::collections::HashSet;

/// One parsed snapshot: the header's column names and the ACTIVE rows, each
/// exactly `columns.len()` wide, borrowed from the input text.
#[derive(Debug, PartialEq, Eq)]
pub struct Rf2Snapshot<'a> {
    /// Column names, verbatim from the header row.
    pub columns: Vec<&'a str>,
    /// Active rows only (`active == "1"`), in file order.
    pub rows: Vec<Vec<&'a str>>,
    /// How many rows the file carried that were NOT kept because they are
    /// inactive — reported so a bake's census can state what it excluded
    /// rather than looking mysteriously short.
    pub inactive: usize,
}

/// Why a file was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rf2Error {
    /// No header row.
    Empty,
    /// The header lacks one of the five columns every refset shares.
    MissingColumn(&'static str),
    /// A row's arity differs from the header's (line number, got, want).
    Ragged(usize, usize, usize),
    /// Two rows share an `id` — not a snapshot. Carries the duplicated id.
    DuplicateId(String),
}

/// Parse one RF2 snapshot file. Borrows from `text`; allocates only the
/// row/column vectors.
pub fn parse_snapshot(text: &str) -> Result<Rf2Snapshot<'_>, Rf2Error> {
    let mut lines = text.lines().filter(|l| !l.is_empty());
    let header = lines.next().ok_or(Rf2Error::Empty)?;
    let columns: Vec<&str> = header.split('\t').collect();
    for want in ["id", "effectiveTime", "active", "moduleId", "refsetId"] {
        if !columns.contains(&want) {
            // sct2_* core files carry the first four but not refsetId; they
            // are still readable — only der2_* demand all five.
            if want == "refsetId" && columns.contains(&"definitionStatusId")
                || want == "refsetId" && columns.contains(&"typeId")
                || want == "refsetId" && columns.contains(&"conceptId")
            {
                continue;
            }
            return Err(Rf2Error::MissingColumn(match want {
                "id" => "id",
                "effectiveTime" => "effectiveTime",
                "active" => "active",
                "moduleId" => "moduleId",
                _ => "refsetId",
            }));
        }
    }
    let active_ix = columns.iter().position(|c| *c == "active").unwrap();
    let id_ix = columns.iter().position(|c| *c == "id").unwrap();

    let mut rows = Vec::new();
    let mut inactive = 0usize;
    let mut seen: HashSet<&str> = HashSet::new();
    for (n, line) in lines.enumerate() {
        let cells: Vec<&str> = line.split('\t').collect();
        if cells.len() != columns.len() {
            return Err(Rf2Error::Ragged(n + 2, cells.len(), columns.len()));
        }
        if !seen.insert(cells[id_ix]) {
            return Err(Rf2Error::DuplicateId(cells[id_ix].to_string()));
        }
        if cells[active_ix] == "1" {
            rows.push(cells);
        } else {
            inactive += 1;
        }
    }
    Ok(Rf2Snapshot {
        columns,
        rows,
        inactive,
    })
}

impl<'a> Rf2Snapshot<'a> {
    /// Index of a named column, or `None` — the caller decides whether a
    /// missing map column is an error for ITS file family.
    #[must_use]
    pub fn col(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| *c == name)
    }

    /// A row's cell in a named column, parsed as an SCTID (`u64` — SNOMED ids
    /// run to 60 bits and never fit smaller). `None` for an absent column or
    /// a non-numeric cell — refused, never truncated.
    #[must_use]
    pub fn sctid(&self, row: usize, name: &str) -> Option<u64> {
        self.rows.get(row)?.get(self.col(name)?)?.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = "id\teffectiveTime\tactive\tmoduleId\trefsetId\treferencedComponentId\tmapTarget\n\
        a1\t20250101\t1\t900\t447562003\t73211009\tE11\n\
        a2\t20250101\t0\t900\t447562003\t44054006\tE10\n\
        a3\t20250101\t1\t900\t447562003\t16898231000119107\tD66\n";

    /// **The active filter fires and is COUNTED.** An inactive row is
    /// excluded from `rows` and reported in `inactive`, so a bake census can
    /// say what it left out instead of looking short.
    #[test]
    fn inactive_rows_are_excluded_and_counted() {
        let s = parse_snapshot(OK).expect("parses");
        assert_eq!(s.rows.len(), 2);
        assert_eq!(s.inactive, 1);
        assert!(s.rows.iter().all(|r| r[2] == "1"));
    }

    /// **SCTIDs read as u64, including past 2^53.** The 54-bit id in the
    /// fixture is a real measured shape (`orphanet.rs` carries the same one);
    /// a reader that parsed into f64 or u32 corrupts it silently.
    #[test]
    fn sctids_are_u64_and_refuse_rather_than_truncate() {
        let s = parse_snapshot(OK).expect("parses");
        assert_eq!(s.sctid(1, "referencedComponentId"), Some(16898231000119107));
        assert_eq!(
            s.sctid(0, "mapTarget"),
            None,
            "E11 is not numeric — refused"
        );
        assert_eq!(s.sctid(0, "nope"), None, "absent column — refused");
    }

    /// **A duplicate id is REFUSED, not resolved.** A snapshot carries one
    /// row per id; two rows means the file is a Full export or a corrupt
    /// concatenation, and keeping either row silently would bake whichever
    /// the file order favored.
    #[test]
    fn a_duplicate_id_refuses_the_whole_file() {
        let dup = OK.replace("a3\t", "a1\t");
        assert_eq!(
            parse_snapshot(&dup),
            Err(Rf2Error::DuplicateId("a1".into()))
        );
    }

    /// **A ragged row refuses the file with its line number.** A permissive
    /// reader mis-aligns every column after a stray tab — the worst kind of
    /// silent, because every cell still LOOKS like a value.
    #[test]
    fn a_ragged_row_refuses_with_the_line_number() {
        let ragged = OK.replace("\tD66", "");
        assert_eq!(parse_snapshot(&ragged), Err(Rf2Error::Ragged(4, 6, 7)));
        assert_eq!(parse_snapshot(""), Err(Rf2Error::Empty));
        assert_eq!(
            parse_snapshot("id\tactive\tmoduleId\trefsetId\n"),
            Err(Rf2Error::MissingColumn("effectiveTime"))
        );
    }
}
