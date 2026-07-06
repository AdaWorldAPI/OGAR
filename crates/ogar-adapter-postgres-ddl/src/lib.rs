//! `ogar-adapter-postgres-ddl` — PostgreSQL DDL adapter for OGAR.
//!
//! Realizes Operator **Ruling (b, refined)**: the persistence role split is
//! **binding** and must not be violated by any future change to this crate.
//!
//! # The binding role split
//!
//! - **PostgreSQL = transactional System-of-Record.** Writes, GoBD/audit,
//!   ACID. The V3 **facet table** ([`emit_facet_table_ddl`]) — `classid` +
//!   the 12 axis-indexable payload columns — lives here; it is the table the
//!   sink-in writes land in.
//! - **lance-graph = read-optimized hot-path** (zero-copy `ClassView ×
//!   FieldMask` projections, Arrow/batch). Lance is **never the sole
//!   booking store** — append+compaction is not OLTP. This crate emits only
//!   the **PG** side (the SoR schema); the lance-graph hot-path read side is
//!   a documented follow-up (bindings probe, not built here).
//! - **Cache placement (BINDING guardrail, not a preference):** the
//!   `moka` (Rust) <-> `moka-py` (Python) cache tier is **PG-side ONLY**.
//!   **Never put a cache in front of lance-graph** — copying Arrow buffers
//!   into owned cache entries breaks zero-copy. The lance hot-path reads
//!   directly off the buffers, uncached.
//!
//! # Position in the data flow
//!
//! ```text
//!   ClassView (schema stratum)  ──▶  this adapter  ──▶  PostgreSQL DDL (SoR)
//!                                                             │
//!   sink-in write (facet bytes) ──▶ facet table (classid + 12 payload cols)
//!                                                             │
//!   [follow-up, NOT built here]  transactional-outbox row ──▶ async worker
//!       ├─▶ projects into lance-graph hot-path (zero-copy read side)
//!       └─▶ runs `parity::check_parity` against the legacy ORM read
//! ```
//!
//! See the [`parity`] module for the legacy-parity drift-fuse skeleton and
//! the full non-built-runtime writeup (transactional outbox, CDC).
//!
//! # Precedent
//!
//! Sibling of [`ogar-adapter-clickhouse-ddl`]; mirrors its shape (emit
//! function, per-attribute column emission, identifier quoting) but targets
//! PostgreSQL and additionally emits the V3 facet table, which has no
//! ClickHouse analog.
//!
//! # Emit-only
//!
//! No parser, no DB driver, no network — this crate renders DDL **text**;
//! tests assert on the text (same discipline as the ClickHouse adapter's
//! emit half).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use ogar_vocab::{Attribute, Class};

pub mod parity;

// ─────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────

/// Render a slice of OGAR `Class`es as PostgreSQL DDL — one `CREATE TABLE`
/// per class, columns derived from the class's attributes (the
/// ClassView-projected relational view: typed + nullable from the schema
/// stratum). Mirrors `emit_clickhouse_ddl` in the sibling crate
/// `ogar-adapter-clickhouse-ddl`.
///
/// # Determinism
///
/// Emit is deterministic — same input always produces the same DDL string.
#[must_use]
pub fn emit_postgres_ddl(classes: &[Class]) -> String {
    let mut out = String::new();
    for class in classes {
        emit_class(class, &mut out);
        out.push('\n');
    }
    out
}

/// Emit the V3 **facet table** DDL — the substrate System-of-Record shape:
/// `classid` + the 12 payload byte-columns (axis-grouped), each column
/// independently B-tree indexable. One flat table is the KV/facet model
/// (OGAR `CLAUDE.md` P0: `classid(4) + 12-byte payload`, `6x(u8:u8)` /
/// `4x(u8:u8:u8)` / `3x(u8:u8:u8:u8)` axis groupings — 12 bytes either way).
/// This is the store the sink-in writes land in.
///
/// The 12 columns are named `p0..p11` — a generic axis-indexed naming
/// scheme independent of which factorization (`6x(8:8)` / `4x(8:8:8)` /
/// `3x(8:8:8:8)`) a given facet uses; all three factorizations pack into
/// the same 12 bytes, so one column layout serves all of them.
///
/// # Determinism
///
/// Emit is deterministic — same `table_name` always produces the same DDL
/// string.
#[must_use]
pub fn emit_facet_table_ddl(table_name: &str) -> String {
    const PAYLOAD_COLUMNS: usize = 12;
    let table = quote_pg_ident(table_name);
    let mut out = String::new();

    out.push_str(&format!("CREATE TABLE {table} (\n"));
    out.push_str("    classid INTEGER NOT NULL,\n");
    let payload_lines: Vec<String> = (0..PAYLOAD_COLUMNS)
        .map(|i| format!("    p{i} SMALLINT"))
        .collect();
    out.push_str(&payload_lines.join(",\n"));
    out.push('\n');
    out.push_str(");\n");

    // Index names must stay bare identifiers even for a dotted/quoted
    // table_name (`sale.order_p0_idx` would be invalid DDL) — derive a
    // sanitized stem instead of interpolating the raw name.
    let stem: String = table_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    out.push_str(&format!(
        "CREATE INDEX {stem}_classid_idx ON {table} (classid);\n"
    ));
    for i in 0..PAYLOAD_COLUMNS {
        out.push_str(&format!(
            "CREATE INDEX {stem}_p{i}_idx ON {table} (p{i});\n"
        ));
    }
    out
}

// ─────────────────────────────────────────────────────────────────────
// Emit — per-class relational DDL
// ─────────────────────────────────────────────────────────────────────

fn emit_class(class: &Class, out: &mut String) {
    let table = quote_pg_ident(&class.name);
    out.push_str(&format!("CREATE TABLE {table} (\n"));

    let mut col_lines = Vec::new();
    for attr in &class.attributes {
        col_lines.push(format!("    {}", emit_column(attr)));
    }
    out.push_str(&col_lines.join(",\n"));
    if !col_lines.is_empty() {
        out.push('\n');
    }
    out.push_str(");\n");
}

fn emit_column(attr: &Attribute) -> String {
    let pg_type = ogar_type_to_postgres(attr.type_name.as_deref());
    // IR-canonical: `options.required == Some(true)` is the NOT NULL
    // marker. `None` ("unset") and `Some(false)` both mean "no NOT NULL
    // constraint emitted" — nullable is the safe default absent an
    // explicit `required=True`.
    if attr.options.required == Some(true) {
        format!("{} {pg_type} NOT NULL", quote_pg_ident(&attr.name))
    } else {
        format!("{} {pg_type}", quote_pg_ident(&attr.name))
    }
}

/// Map an OGAR canonical `type_name` (`"string"`, `"integer"`, …) to a
/// PostgreSQL type. Lengths / precision-scale are not carried by the IR,
/// so `TEXT` / `NUMERIC` are the safe unparameterized fallbacks (mirrors
/// the ClickHouse adapter's untyped-fallback discipline).
fn ogar_type_to_postgres(t: Option<&str>) -> String {
    match t.map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("string") | Some("char") | Some("html") | Some("binary") => "TEXT".to_string(),
        Some("text") => "TEXT".to_string(),
        Some("integer") | Some("int") => "INTEGER".to_string(),
        Some("bigint") => "BIGINT".to_string(),
        Some("boolean") | Some("bool") => "BOOLEAN".to_string(),
        Some("datetime") | Some("timestamp") => "TIMESTAMP".to_string(),
        Some("date") => "DATE".to_string(),
        Some("decimal") | Some("monetary") | Some("numeric") => "NUMERIC".to_string(),
        Some("float") | Some("double") | Some("real") => "DOUBLE PRECISION".to_string(),
        None => "TEXT".to_string(),
        Some(_other) => "TEXT".to_string(),
    }
}

/// Quote a PostgreSQL identifier with double-quotes when it isn't a safe
/// bare identifier. PostgreSQL bare identifiers match
/// `[a-zA-Z_][a-zA-Z0-9_]*` (case-folded); everything else — notably
/// Odoo's dotted names like `sale.order` — needs `"..."` quoting. Same
/// discipline as `ogar-adapter-clickhouse-ddl`'s `quote_ch_ident`, adapted
/// to PostgreSQL's double-quote identifier syntax.
fn quote_pg_ident(name: &str) -> String {
    let bare = !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if bare {
        name.to_string()
    } else {
        format!("\"{}\"", name.replace('"', "\"\""))
    }
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ogar_vocab::{Attribute, Class};

    #[test]
    fn emit_minimal_class_produces_create_table() {
        let c = Class::new("users");
        let ddl = emit_postgres_ddl(&[c]);
        assert!(ddl.contains("CREATE TABLE users (\n"), "got: {ddl}");
    }

    #[test]
    fn emit_class_typed_and_nullable_columns() {
        let mut c = Class::new("notes");
        let mut subject = Attribute::new("subject");
        subject.type_name = Some("string".into());
        subject.options.required = Some(true);
        c.attributes.push(subject);
        let mut note = Attribute::new("note");
        note.type_name = Some("text".into());
        c.attributes.push(note);

        let ddl = emit_postgres_ddl(&[c]);
        assert!(ddl.contains("subject TEXT NOT NULL"), "got: {ddl}");
        // Nullable: no explicit `required=true` -> no NOT NULL suffix, and
        // no incidental match against the NOT NULL variant of the same
        // column name.
        assert!(ddl.contains("note TEXT") && !ddl.contains("note TEXT NOT NULL"), "got: {ddl}");
    }

    #[test]
    fn emit_quotes_dotted_table_name() {
        let c = Class::new("sale.order");
        let ddl = emit_postgres_ddl(&[c]);
        assert!(ddl.contains("CREATE TABLE \"sale.order\" ("), "got: {ddl}");
    }

    #[test]
    fn emit_facet_table_has_classid_and_twelve_payload_columns() {
        let ddl = emit_facet_table_ddl("facet");
        assert!(ddl.contains("classid INTEGER NOT NULL"), "got: {ddl}");
        for i in 0..12 {
            assert!(ddl.contains(&format!("p{i} SMALLINT")), "missing p{i}: {ddl}");
            assert!(
                ddl.contains(&format!("CREATE INDEX facet_p{i}_idx ON facet (p{i});")),
                "missing index for p{i}: {ddl}"
            );
        }
        assert!(
            ddl.contains("CREATE INDEX facet_classid_idx ON facet (classid);"),
            "got: {ddl}"
        );
    }

    /// Ties SPEC-7 to the SPEC-5 parity fixture: `TimesheetActivity`
    /// (`WoA/models.py:1746`, read-only source, transcribed faithfully).
    ///
    /// NOTE (Sonnet grind, SPEC-7): SPEC-5's synthetic `ModelGraph` ->
    /// `CompiledClass` lift for `TimesheetActivity` lives on a sibling
    /// worktree/branch (`claude/v3c-woa-parity`) that had not merged into
    /// this branch (`claude/v3c-postgres-ddl`, forked from `origin/main`
    /// at the PR #156 merge commit) at the time this crate was built. Per
    /// mission rule ("never invent"), this fixture is hand-transcribed
    /// directly from `WoA/models.py:1746` (read in full) rather than
    /// invented or borrowed from an unmerged branch, and is a local
    /// stand-in for SPEC-5's real lift output. It exercises the same
    /// column set: `id` (PK, omitted here — the lift's ID handling is
    /// SPEC-5's concern), `timesheet_id` deduped as an FK upstream
    /// (omitted, matching the lift's FK-dedup contract), `beschreibung`
    /// (`String(500)`, `nullable=False`), `created_at` (`DateTime`, no
    /// explicit `nullable=False` -> nullable).
    #[test]
    fn woa_timesheet_activity_ddl_roundtrips_field_set() {
        let mut c = Class::new("TimesheetActivity");
        let mut beschreibung = Attribute::new("beschreibung");
        beschreibung.type_name = Some("string".into());
        beschreibung.options.required = Some(true);
        c.attributes.push(beschreibung);
        let mut created_at = Attribute::new("created_at");
        created_at.type_name = Some("datetime".into());
        c.attributes.push(created_at);

        let ddl = emit_postgres_ddl(&[c]);
        assert!(ddl.contains("beschreibung TEXT NOT NULL"), "got: {ddl}");
        assert!(ddl.contains("created_at TIMESTAMP"), "got: {ddl}");
        assert!(!ddl.contains("created_at TIMESTAMP NOT NULL"), "got: {ddl}");
        assert!(!ddl.contains("timesheet_id"), "FK must be deduped upstream: {ddl}");
    }
}
