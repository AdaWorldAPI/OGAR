//! `ogar-adapter-clickhouse-ddl` — bidirectional ClickHouse DDL bridge
//! for OGAR.
//!
//! Companion to [`ogar-adapter-surrealql`] and [`ogar-adapter-ttl`].
//! Phase 2b of `docs/RDF-OWL-ALIGNMENT.md §10` — the brutal-upgrade
//! ingestion substrate's second source-language adapter that targets
//! the ClickHouse-ecosystem shadow path opened by bardioc PR #19.
//!
//! # The two directions
//!
//! | direction | function | feature |
//! |---|---|---|
//! | `Class` -> ClickHouse DDL | [`emit_clickhouse_ddl`] | always available |
//! | ClickHouse DDL -> `Vec<Class>` | [`parse_clickhouse_ddl`] | `clickhouse-parser` |
//!
//! # Position in the data flow
//!
//! ```text
//!   ClickHouse CREATE TABLE  ──▶  this adapter  ──▶  Class IR (schema)
//!                                                          │
//!   ClickHouse rows (RowBinary) ──▶ substrate-b-shadow      │  schema
//!                                   EdgeDecoder  ──▶  ActionInvocation
//!                                                          │
//!                                            interprets ───┘
//! ```
//!
//! The OGAR-side concern is the **schema** (DDL → `Class`). The
//! bardioc/runtime-side concern is the runtime **rows** (`RowBinary` →
//! `ActionInvocation`). They share the same OGAR `Class` IR as the
//! cross-substrate contract — per ADR-023 (IR-as-wire-truth).
//!
//! # Type mapping (ClickHouse ⇄ OGAR canonical)
//!
//! On parse, ClickHouse types are normalized to OGAR's canonical
//! type-name lexicon (the same set used by `ogar-adapter-surrealql`):
//!
//! | ClickHouse                                  | OGAR `type_name` |
//! |---------------------------------------------|------------------|
//! | `String`, `FixedString(N)`                  | `string`         |
//! | `UInt8/16/32/64`, `Int8/16/32/64`           | `int`            |
//! | `Float32`, `Float64`                        | `float`          |
//! | `Decimal(P,S)`                              | `decimal`        |
//! | `DateTime`, `DateTime64`, `Date`            | `datetime`       |
//! | `Bool`                                      | `bool`           |
//! | `UUID`                                      | `uuid`           |
//! | `Nullable(<inner>)`                         | inner type + `options.required = Some(false)` |
//! | anything else                               | `any`            |
//!
//! On emit, the inverse map produces canonical ClickHouse types
//! (default-width — `Int64`, `Float64`, `Decimal(18, 4)`,
//! `DateTime` — sufficient for AR-pattern fidelity).
//!
//! # Roundtrip property
//!
//! `parse_clickhouse_ddl(emit_clickhouse_ddl(&classes))` reconstructs
//! structurally-equivalent classes for the v1 supported subset.
//! Verified by the `round_trip_*` tests below.
//!
//! # Supported in this v1
//!
//! - `CREATE TABLE <name> (<col1> <type1>, …) ENGINE = MergeTree ORDER BY …`
//! - Column types in the table above (Nullable handled).
//! - `ORDER BY <expr>` lifted to `Class.record_order` when scalar.
//!
//! # Not yet supported (next sprint)
//!
//! - Foreign-key-like patterns (ClickHouse uses `Array(UInt64)` or
//!   joined materialized views — semantic lift, not a v1 concern).
//! - Materialized views, dictionaries, distributed tables.
//! - `CREATE DATABASE`, `ATTACH`, `RENAME`.
//! - Column-level codec / compression hints (`CODEC(...)`, `TTL`).
//! - Engine-specific clauses beyond `ENGINE = MergeTree`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use ogar_vocab::{Attribute, Class};

// ─────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────

/// Errors from [`parse_clickhouse_ddl`].
#[derive(Debug, Clone)]
pub enum ParseError {
    /// The input couldn't be tokenized / parsed by `sqlparser`.
    Parse(String),
    /// A `CREATE TABLE` statement couldn't be mapped to a `Class`.
    Unmappable {
        /// Table the failing column is on.
        table: String,
        /// Why mapping failed.
        reason: String,
    },
    /// The function is wired but the requested capability is pending a
    /// follow-up sprint.
    Unimplemented(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Parse(msg) => write!(f, "ClickHouse DDL parse error: {msg}"),
            ParseError::Unmappable { table, reason } => {
                write!(f, "couldn't lift table `{table}`: {reason}")
            }
            ParseError::Unimplemented(msg) => write!(f, "unimplemented: {msg}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Render a slice of OGAR `Class`es as ClickHouse DDL.
///
/// Each class produces a `CREATE TABLE` statement with column
/// declarations derived from the class's attributes. The `ENGINE =
/// MergeTree()` clause is appended unconditionally (the most common
/// AR-pattern fit; future PRs can lift other engines if a real
/// consumer needs them). `ORDER BY` is `tuple()` when no
/// `record_order` is set, otherwise the literal `record_order` value.
///
/// # Determinism
///
/// Emit is deterministic — same input always produces the same DDL
/// string.
#[must_use]
pub fn emit_clickhouse_ddl(classes: &[Class]) -> String {
    let mut out = String::new();
    for class in classes {
        emit_class(class, &mut out);
        out.push('\n');
    }
    out
}

/// Parse ClickHouse DDL into a `Vec<Class>` — the `unmap` direction.
///
/// # Feature gate
///
/// Requires the `clickhouse-parser` feature (which pulls in
/// `sqlparser`). Without the feature, returns
/// [`ParseError::Unimplemented`].
pub fn parse_clickhouse_ddl(_input: &str) -> Result<Vec<Class>, ParseError> {
    #[cfg(feature = "clickhouse-parser")]
    {
        walk::parse_inner(_input)
    }
    #[cfg(not(feature = "clickhouse-parser"))]
    {
        Err(ParseError::Unimplemented(
            "clickhouse-parser feature not enabled; rebuild with \
             --features clickhouse-parser to enable parsing".into(),
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────
// Emit
// ─────────────────────────────────────────────────────────────────────

fn emit_class(class: &Class, out: &mut String) {
    let table = quote_ch_ident(&class.name);
    out.push_str(&format!("CREATE TABLE {table} (\n"));

    let mut col_lines = Vec::new();
    for attr in &class.attributes {
        col_lines.push(format!("    {}", emit_column(attr)));
    }
    out.push_str(&col_lines.join(",\n"));
    if !col_lines.is_empty() {
        out.push('\n');
    }
    out.push_str(") ENGINE = MergeTree() ORDER BY ");
    match &class.record_order {
        Some(expr) => out.push_str(expr),
        None => out.push_str("tuple()"),
    }
    out.push_str(";\n");
}

fn emit_column(attr: &Attribute) -> String {
    let ch_type_inner = ogar_type_to_clickhouse(attr.type_name.as_deref());
    // `options.required = Some(false)` is the IR-canonical optional
    // marker (per `ogar-vocab::AttributeOptions`). Wrap with
    // `Nullable(…)` to round-trip; `None` means "unset" (not
    // "required=true"), so leave unwrapped.
    let ch_type = if attr.options.required == Some(false) {
        format!("Nullable({ch_type_inner})")
    } else {
        ch_type_inner
    };
    format!("{} {ch_type}", quote_ch_ident(&attr.name))
}

/// Map an OGAR canonical `type_name` (`"string"`, `"int"`, …) to a
/// ClickHouse type. Default widths (`Int64`/`Float64`/`DateTime`/
/// `Decimal(18, 4)`) are AR-pattern-sufficient; future PRs can lift
/// width hints from `Attribute.options.digits` / `Attribute.options.size`.
fn ogar_type_to_clickhouse(t: Option<&str>) -> String {
    match t.map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("string") | Some("text") | Some("char") | Some("html") | Some("binary") => {
            "String".to_string()
        }
        Some("int") | Some("integer") | Some("bigint") | Some("smallint") | Some("tinyint") => {
            "Int64".to_string()
        }
        Some("float") | Some("double") | Some("real") => "Float64".to_string(),
        Some("decimal") | Some("monetary") | Some("numeric") => {
            "Decimal(18, 4)".to_string()
        }
        Some("bool") | Some("boolean") => "Bool".to_string(),
        Some("datetime") | Some("timestamp") | Some("date") => "DateTime".to_string(),
        Some("uuid") => "UUID".to_string(),
        Some("any") | None => "String".to_string(),
        Some(other) => format!("String /* unmapped OGAR type: {other} */"),
    }
}

/// Quote a ClickHouse identifier with backticks when it isn't a safe
/// bare identifier. ClickHouse identifiers match
/// `[a-zA-Z_][a-zA-Z0-9_]*` for bare form; everything else needs
/// backticking. Same discipline as `ogar-adapter-surrealql`'s
/// `surrealql_ident` (closes the same Codex P2 class — Odoo dotted
/// names like `sale.order` need quoting).
fn quote_ch_ident(name: &str) -> String {
    let bare = !name.is_empty()
        && name.chars().next().map_or(false, |c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if bare {
        name.to_string()
    } else {
        format!("`{}`", name.replace('`', "``"))
    }
}

// ─────────────────────────────────────────────────────────────────────
// Parse (behind `clickhouse-parser`)
// ─────────────────────────────────────────────────────────────────────

#[cfg(feature = "clickhouse-parser")]
mod walk {
    use super::*;
    use ogar_vocab::Attribute;
    use sqlparser::ast::{ColumnDef, DataType, ObjectName, Statement};
    use sqlparser::dialect::ClickHouseDialect;
    use sqlparser::parser::Parser;

    pub(super) fn parse_inner(input: &str) -> Result<Vec<Class>, ParseError> {
        let stmts = Parser::parse_sql(&ClickHouseDialect {}, input)
            .map_err(|e| ParseError::Parse(format!("{e}")))?;

        let mut classes = Vec::new();
        for stmt in stmts {
            if let Statement::CreateTable(ct) = stmt {
                let name = last_ident_from_object_name(&ct.name);
                let mut class = Class::new(name);
                for col in &ct.columns {
                    class.attributes.push(lift_column(col));
                }
                // ORDER BY → record_order when scalar / simple.
                if let Some(order) = ct.order_by.as_ref() {
                    let rendered = order.to_string();
                    // sqlparser renders the clause as `ORDER BY <expr>`
                    // OR just `<expr>` depending on the AST node — strip
                    // the leading "ORDER BY" if present to keep
                    // record_order as the bare expression.
                    let stripped = rendered
                        .strip_prefix("ORDER BY ")
                        .unwrap_or(&rendered)
                        .trim();
                    if stripped != "tuple()" {
                        class.record_order = Some(stripped.to_string());
                    }
                }
                classes.push(class);
            }
        }
        Ok(classes)
    }

    fn lift_column(col: &ColumnDef) -> Attribute {
        let mut attr = Attribute::new(col.name.value.clone());
        let (canonical_type, optional) = clickhouse_type_to_ogar(&col.data_type);
        attr.type_name = Some(canonical_type);
        if optional {
            attr.options.required = Some(false);
        }
        attr
    }

    /// Map a `sqlparser::ast::DataType` (under ClickHouseDialect) back
    /// to OGAR's canonical type names. Returns `(canonical, optional)`.
    fn clickhouse_type_to_ogar(dt: &DataType) -> (String, bool) {
        // Custom-typed columns (e.g. `Nullable(...)`, `UInt64`, …)
        // arrive as `DataType::Custom(ObjectName, ...)`. The first
        // ident segment is the type name; the trailing arguments live
        // on the modifiers / type-args. We render the whole thing as
        // a string and match on a normalized lowercase form — simpler
        // than walking sqlparser's whole DataType variant tree, and
        // gives correct results for the common ClickHouse families.
        let rendered = dt.to_string();
        let lower = rendered.to_ascii_lowercase();

        // Unwrap Nullable(...) recursively. ClickHouse forbids
        // nested Nullable (`Nullable(Nullable(X))` is rejected by the
        // server), so single-strip is sufficient.
        if let Some(inner) = strip_wrapper(&rendered, "nullable") {
            let (canonical, _already_optional) =
                clickhouse_type_to_ogar(&dummy_data_type(&inner));
            return (canonical, true);
        }

        // Family matching by lowercase prefix.
        let canonical = if lower.starts_with("string") || lower.starts_with("fixedstring") {
            "string"
        } else if lower.starts_with("uint") || lower.starts_with("int") {
            "int"
        } else if lower.starts_with("float") {
            "float"
        } else if lower.starts_with("decimal") {
            "decimal"
        } else if lower.starts_with("datetime") || lower.starts_with("date") {
            "datetime"
        } else if lower.starts_with("bool") {
            "bool"
        } else if lower.starts_with("uuid") {
            "uuid"
        } else {
            "any"
        };
        (canonical.to_string(), false)
    }

    /// Strip `wrapper(<inner>)` and return `<inner>` (case-insensitive
    /// match on the wrapper name). Returns `None` if the input isn't
    /// wrapped in `wrapper(...)`.
    fn strip_wrapper(rendered: &str, wrapper: &str) -> Option<String> {
        let trimmed = rendered.trim();
        let lower = trimmed.to_ascii_lowercase();
        let prefix = format!("{wrapper}(");
        if !lower.starts_with(&prefix) || !trimmed.ends_with(')') {
            return None;
        }
        let inner = &trimmed[prefix.len()..trimmed.len() - 1];
        Some(inner.trim().to_string())
    }

    /// Build a placeholder DataType from a string. Used for the
    /// recursive Nullable-unwrap path. We don't need the full sqlparser
    /// AST round-trip — only the rendered form for the family-prefix
    /// match. `DataType::Custom` carries an ObjectName which renders
    /// back to the input via Display.
    fn dummy_data_type(s: &str) -> DataType {
        use sqlparser::ast::ObjectName;
        DataType::Custom(
            ObjectName::from(vec![sqlparser::ast::Ident::new(s.to_string())]),
            vec![],
        )
    }

    /// Get the table name from an `ObjectName` — handles both:
    /// 1. **Multi-part qualified names** like `database.table` —
    ///    sqlparser surfaces this as `ObjectName(vec![db_part,
    ///    table_part])`; the table name is the last segment.
    /// 2. **Backtick-quoted dotted single names** like
    ///    `` `sale.order` `` (Odoo convention) — sqlparser surfaces
    ///    this as `ObjectName(vec![one_part_with_value="sale.order"])`;
    ///    the dot is INSIDE the identifier value, not a separator.
    ///
    /// Closes Codex P2 on PR #38: the previous `last_ident(&str)`
    /// converted the `ObjectName` to a string and then split on `.`
    /// before stripping backticks, which silently turned
    /// `` `sale.order` `` into `order` and broke the round-trip the
    /// PR claimed to support for Odoo dotted class names.
    ///
    /// Using `Ident.value` directly bypasses the string round-trip
    /// — `value` is the *unquoted* text, so backtick-quoting and the
    /// `.` ambiguity are resolved structurally by the parser before
    /// this helper sees it.
    fn last_ident_from_object_name(name: &ObjectName) -> String {
        name.0
            .last()
            .and_then(|part| part.as_ident())
            .map(|ident| ident.value.clone())
            .unwrap_or_default()
    }
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ogar_vocab::{Attribute, Class};

    // ── Emit tests (always available) ─────────────────────────────────

    #[test]
    fn emit_minimal_class_produces_create_table_mergetree() {
        let c = Class::new("users");
        let ddl = emit_clickhouse_ddl(&[c]);
        assert!(ddl.contains("CREATE TABLE users (\n"), "got: {ddl}");
        assert!(ddl.contains("ENGINE = MergeTree() ORDER BY tuple();"), "got: {ddl}");
    }

    #[test]
    fn emit_class_with_string_column() {
        let mut c = Class::new("account");
        let mut email = Attribute::new("email");
        email.type_name = Some("string".into());
        c.attributes.push(email);
        let ddl = emit_clickhouse_ddl(&[c]);
        assert!(ddl.contains("email String"), "got: {ddl}");
    }

    #[test]
    fn emit_class_with_optional_column_wraps_with_nullable() {
        let mut c = Class::new("account");
        let mut deleted = Attribute::new("deleted_at");
        deleted.type_name = Some("datetime".into());
        deleted.options.required = Some(false);
        c.attributes.push(deleted);
        let ddl = emit_clickhouse_ddl(&[c]);
        assert!(
            ddl.contains("deleted_at Nullable(DateTime)"),
            "got: {ddl}"
        );
    }

    #[test]
    fn emit_class_with_record_order_lifts_to_order_by() {
        let mut c = Class::new("events");
        c.record_order = Some("id, created_at".into());
        let ddl = emit_clickhouse_ddl(&[c]);
        assert!(ddl.contains("ORDER BY id, created_at"), "got: {ddl}");
    }

    #[test]
    fn emit_quotes_non_bare_table_name() {
        // Odoo dotted names need backticking — same pattern as
        // ogar-adapter-surrealql's identifier quoting (closes the
        // same class of footgun).
        let c = Class::new("sale.order");
        let ddl = emit_clickhouse_ddl(&[c]);
        assert!(ddl.contains("CREATE TABLE `sale.order` ("), "got: {ddl}");
    }

    #[test]
    fn emit_full_class_with_multiple_columns() {
        let mut c = Class::new("orders");
        let mut id = Attribute::new("id");
        id.type_name = Some("int".into());
        c.attributes.push(id);
        let mut amount = Attribute::new("amount");
        amount.type_name = Some("decimal".into());
        c.attributes.push(amount);
        let mut paid = Attribute::new("paid_at");
        paid.type_name = Some("datetime".into());
        paid.options.required = Some(false);
        c.attributes.push(paid);
        c.record_order = Some("id".into());
        let ddl = emit_clickhouse_ddl(&[c]);
        assert!(ddl.contains("CREATE TABLE orders ("), "got: {ddl}");
        assert!(ddl.contains("id Int64"), "got: {ddl}");
        assert!(ddl.contains("amount Decimal(18, 4)"), "got: {ddl}");
        assert!(ddl.contains("paid_at Nullable(DateTime)"), "got: {ddl}");
        assert!(ddl.contains("ORDER BY id"), "got: {ddl}");
    }

    // ── Parse tests (gated behind `clickhouse-parser`) ────────────────

    #[cfg(feature = "clickhouse-parser")]
    #[test]
    fn parse_minimal_create_table_lifts_one_class() {
        let ddl = "CREATE TABLE users (id UInt64, name String) ENGINE = MergeTree ORDER BY id;";
        let classes = parse_clickhouse_ddl(ddl).expect("parse OK");
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "users");
        assert_eq!(classes[0].attributes.len(), 2);
        assert_eq!(classes[0].attributes[0].name, "id");
        assert_eq!(classes[0].attributes[0].type_name.as_deref(), Some("int"));
        assert_eq!(classes[0].attributes[1].name, "name");
        assert_eq!(classes[0].attributes[1].type_name.as_deref(), Some("string"));
    }

    #[cfg(feature = "clickhouse-parser")]
    #[test]
    fn parse_nullable_lifts_to_required_false() {
        let ddl = "CREATE TABLE t (deleted_at Nullable(DateTime)) ENGINE = MergeTree ORDER BY tuple();";
        let classes = parse_clickhouse_ddl(ddl).expect("parse OK");
        let attr = &classes[0].attributes[0];
        // IR-canonical: bare type + required=Some(false). Same shape
        // ogar-adapter-surrealql uses for option<X> primitives.
        assert_eq!(attr.type_name.as_deref(), Some("datetime"));
        assert_eq!(attr.options.required, Some(false));
    }

    #[cfg(feature = "clickhouse-parser")]
    #[test]
    fn parse_recognizes_type_families() {
        let ddl = "CREATE TABLE t (\
                       a String, b UInt8, c Int64, d Float32, \
                       e Decimal(10, 2), f DateTime, g Bool, h UUID, \
                       i FixedString(16) \
                   ) ENGINE = MergeTree ORDER BY tuple();";
        let classes = parse_clickhouse_ddl(ddl).expect("parse OK");
        let names: Vec<&str> = classes[0].attributes.iter()
            .map(|a| a.type_name.as_deref().unwrap_or(""))
            .collect();
        assert_eq!(
            names,
            vec!["string", "int", "int", "float", "decimal", "datetime", "bool", "uuid", "string"],
        );
    }

    #[cfg(feature = "clickhouse-parser")]
    #[test]
    fn parse_rejects_invalid_ddl() {
        let bad = "this is not DDL";
        match parse_clickhouse_ddl(bad) {
            Err(ParseError::Parse(_)) => {}
            other => panic!("expected Parse error, got: {other:?}"),
        }
    }

    // ── Codex P2 fix (PR #38) ───────────────────────────────────────────
    // Backtick-quoted dotted table names (Odoo `sale.order` convention)
    // round-trip correctly. Previously `last_ident` split on `.` BEFORE
    // stripping backticks, silently truncating `` `sale.order` `` to
    // `order` and breaking the round-trip the PR claimed to support.
    // ────────────────────────────────────────────────────────────────────

    #[cfg(feature = "clickhouse-parser")]
    #[test]
    fn parse_backtick_quoted_dotted_table_keeps_full_name() {
        let ddl = "CREATE TABLE `sale.order` (id UInt64) ENGINE = MergeTree ORDER BY id;";
        let classes = parse_clickhouse_ddl(ddl).expect("parse OK");
        assert_eq!(classes.len(), 1);
        // CRITICAL: must be "sale.order", not "order" (the Codex P2
        // motivating case).
        assert_eq!(classes[0].name, "sale.order");
    }

    #[cfg(feature = "clickhouse-parser")]
    #[test]
    fn parse_qualified_db_table_takes_last_segment() {
        // Multi-part qualified name `db.table` (unquoted) — sqlparser
        // surfaces as ObjectName with two parts; the table name is the
        // last segment.
        let ddl = "CREATE TABLE my_db.users (id UInt64) ENGINE = MergeTree ORDER BY id;";
        let classes = parse_clickhouse_ddl(ddl).expect("parse OK");
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "users");
    }

    #[cfg(feature = "clickhouse-parser")]
    #[test]
    fn round_trip_odoo_dotted_table_name() {
        // The load-bearing round-trip: build IR with an Odoo dotted
        // name, emit, parse, assert the name survives byte-identical.
        let mut original = Class::new("sale.order");
        let mut amount = Attribute::new("amount_total");
        amount.type_name = Some("decimal".into());
        original.attributes.push(amount);

        let ddl = emit_clickhouse_ddl(&[original]);
        // Emit must produce backtick-quoted form (per the existing
        // emit_quotes_non_bare_table_name test).
        assert!(
            ddl.contains("CREATE TABLE `sale.order` ("),
            "expected backtick-quoted CREATE TABLE, got: {ddl}"
        );

        let recovered = parse_clickhouse_ddl(&ddl).expect("parse OK");
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].name, "sale.order");
        assert_eq!(recovered[0].attributes.len(), 1);
        assert_eq!(recovered[0].attributes[0].name, "amount_total");
        assert_eq!(recovered[0].attributes[0].type_name.as_deref(), Some("decimal"));
    }

    #[cfg(feature = "clickhouse-parser")]
    #[test]
    fn round_trip_three_segment_odoo_name() {
        // `account.move.line` — three-segment dotted name backtick-
        // quoted as a single identifier (no qualified-db semantics).
        let c = Class::new("account.move.line");
        let ddl = emit_clickhouse_ddl(&[c]);
        assert!(
            ddl.contains("`account.move.line`"),
            "three-segment name must be quoted, got: {ddl}"
        );
        let recovered = parse_clickhouse_ddl(&ddl).expect("parse OK");
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].name, "account.move.line");
    }

    // ── Round-trip tests (the load-bearing ones) ──────────────────────

    #[cfg(feature = "clickhouse-parser")]
    #[test]
    fn round_trip_simple_class_preserves_attributes() {
        let mut original = Class::new("account");
        let mut email = Attribute::new("email");
        email.type_name = Some("string".into());
        original.attributes.push(email);
        let mut age = Attribute::new("age");
        age.type_name = Some("int".into());
        original.attributes.push(age);

        let ddl = emit_clickhouse_ddl(&[original]);
        let recovered = parse_clickhouse_ddl(&ddl).expect("parse OK");
        assert_eq!(recovered.len(), 1);
        let r = &recovered[0];
        assert_eq!(r.name, "account");
        assert_eq!(r.attributes.len(), 2);
        assert_eq!(r.attributes[0].name, "email");
        assert_eq!(r.attributes[0].type_name.as_deref(), Some("string"));
        assert_eq!(r.attributes[1].name, "age");
        assert_eq!(r.attributes[1].type_name.as_deref(), Some("int"));
    }

    #[cfg(feature = "clickhouse-parser")]
    #[test]
    fn round_trip_optional_primitive() {
        let mut original = Class::new("account");
        let mut deleted = Attribute::new("deleted_at");
        deleted.type_name = Some("datetime".into());
        deleted.options.required = Some(false);
        original.attributes.push(deleted);

        let ddl = emit_clickhouse_ddl(&[original]);
        let recovered = parse_clickhouse_ddl(&ddl).expect("parse OK");
        let attr = &recovered[0].attributes[0];
        assert_eq!(attr.name, "deleted_at");
        assert_eq!(attr.type_name.as_deref(), Some("datetime"));
        assert_eq!(attr.options.required, Some(false));
    }

    #[cfg(not(feature = "clickhouse-parser"))]
    #[test]
    fn parse_returns_unimplemented_when_feature_off() {
        let r = parse_clickhouse_ddl("anything");
        assert!(matches!(r, Err(ParseError::Unimplemented(_))));
    }
}
