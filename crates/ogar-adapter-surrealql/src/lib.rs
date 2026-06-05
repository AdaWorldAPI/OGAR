//! `ogar-adapter-surrealql` — bidirectional SurrealQL DDL bridge for OGAR.
//!
//! # The two directions
//!
//! Per `docs/OGAR-AST-CONTRACT.md §2`:
//!
//! | direction | function | status |
//! |---|---|---|
//! | `Class` -> SurrealQL DDL string | [`emit_surrealql_ddl`] | **wired** — hand-written formatter |
//! | SurrealQL DDL string -> `Vec<Class>` | [`parse_surrealql_ddl`] | **scaffold** — `todo!()` until rust-version bump |
//!
//! # The §10.3 meet-point: `knowable_from`
//!
//! Per `docs/OPENPROJECT-TRANSCODING.md §10.3`, the SurrealQL frame's
//! `knowable_from` is sourced here ([`register_class_knowable_from`])
//! and consumed by `lance-graph-planner::temporal::classify` (runtime
//! session). This crate is the *only* OGAR-side owner of that side of
//! the seam.
//!
//! # The rust-version blocker on `parse_surrealql_ddl`
//!
//! `AdaWorldAPI/surrealdb` pins `rust-version = "1.95"`; OGAR's workspace
//! is on `1.85`. A direct git dep on `surrealdb-ast` + `surrealdb-parser`
//! won't build in OGAR's CI today. The `surrealdb-parser` feature flag in
//! `Cargo.toml` documents the wiring intent; the actual `dependencies`
//! entries land alongside an OGAR `rust-version` bump.
//!
//! Until then:
//! - [`emit_surrealql_ddl`] is fully implemented (hand-written formatter).
//! - [`parse_surrealql_ddl`] is `todo!()` with documented parser wiring.
//! - [`register_class_knowable_from`] is `todo!()` (needs a Lance writer
//!   too — that's the `lance-bind` Sprint-5b boundary).
//!
//! # Alignment with `surrealdb-core::catalog::TableDefinition::new_for_ddl`
//!
//! Per the Sprint C16b op-codegen-bridge initiative
//! (`AdaWorldAPI/surrealdb/.claude/op-codegen-bridge/README.md`),
//! `TableDefinition::new_for_ddl().with_*(...)` followed by
//! `ToSql::to_sql()` is the canonical external-codegen DDL path. The
//! hand-written formatter below produces DDL of the same shape; when the
//! `surrealdb-parser` feature lands, a follow-up can swap the formatter
//! body to call the catalog builders directly. The public function
//! signature stays the same — this is the durable interface.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::path::Path;

use ogar_vocab::{Association, AssociationKind, Attribute, Class, EnumDecl};

// ─────────────────────────────────────────────────────────────────────
// Public API surface — locked, durable across the rust-version bump
// ─────────────────────────────────────────────────────────────────────

/// Render a slice of OGAR `Class`es as a SurrealQL DDL string.
///
/// Produces `DEFINE TABLE` + `DEFINE FIELD` statements per class,
/// translating OGAR `Attribute`/`Association`/`EnumDecl` shapes to
/// SurrealQL field declarations. Aligned with the shape produced by
/// `surrealdb-core::catalog::TableDefinition::new_for_ddl(...).with_*(...)`
/// + `ToSql::to_sql()` (Sprint C16b op-codegen-bridge).
///
/// Hand-written today (no heavy `surrealdb-core` dep); the function
/// signature is the durable interface — a follow-up can swap the body
/// to call catalog builders directly once the rust-version blocker
/// clears.
#[must_use]
pub fn emit_surrealql_ddl(classes: &[Class]) -> String {
    let mut out = String::new();
    for class in classes {
        emit_class(class, &mut out);
        out.push('\n');
    }
    out
}

/// Parse SurrealQL DDL into a `Vec<Class>` — the `unmap` direction
/// (per `docs/OGAR-AST-CONTRACT.md §2`).
///
/// # Panics
///
/// Currently `todo!()` — requires `surrealdb-ast` + `surrealdb-parser`
/// (`AdaWorldAPI/surrealdb`), gated behind OGAR's rust-version bump
/// from `1.85` to `1.95+`.
///
/// # What to wire
///
/// Per `docs/OGAR-AST-CONTRACT.md §2` mapping:
///
/// | SurrealQL AST node | -> OGAR IR type |
/// |---|---|
/// | `DefineTable { name, .. }` | `Class { identity: name, .. }` |
/// | `DefineField TYPE record<x>` | `Association { kind: BelongsTo, class_name: Some(x) }` |
/// | `DefineField TYPE string + ASSERT $value IN [...]` | `EnumDecl { variants: [...] }` |
/// | `DefineField TYPE option<X>` | `Attribute { type_name: Some(X), optional: true }` |
/// | `DefineField TYPE <scalar>` | `Attribute { type_name: Some(scalar) }` |
///
/// Use `surrealdb_parser::parse_module(input)?` -> `surrealdb_ast::Library`,
/// walk `.define_table_stmt[*]` + `.define_field_stmt[*]` (the
/// `library!` macro node lists), build one `Class` per `DefineTable`,
/// dispatch each field to attribute/association/enum per the table
/// above.
///
/// # Roundtrip invariant
///
/// When wired, the property
/// `parse_surrealql_ddl(emit_surrealql_ddl(parse_surrealql_ddl(x)?)?) == parse_surrealql_ddl(x)?`
/// should hold for any well-formed SurrealQL DDL `x`. Proptest fixture
/// lands alongside the parser wiring.
pub fn parse_surrealql_ddl(_input: &str) -> Result<Vec<Class>, ParseError> {
    todo!(
        "wire surrealdb-parser; pending OGAR rust-version bump to 1.95+; see crate-level docs"
    )
}

/// Register a `Class` and return its `knowable_from` Lance version —
/// the timestamp at which this class became defined in the substrate.
///
/// This is the **producer side of the §10.3 meet-point** with
/// `lance-graph-planner::temporal::classify`. Consumed by `classify` as
/// one of three inputs:
/// `classify(row_version, knowable_from, v_ref) -> {CONTEMPORARY | ANACHRONISTIC | SPOILER}`.
///
/// # Panics
///
/// Currently `todo!()` — requires a Lance dataset writer
/// (`LanceMembrane::commit_event` or equivalent kv-lance backend).
/// The `lance-bind` Sprint-5b boundary is the integration path.
///
/// # What to wire
///
/// Emit the class's `DEFINE TABLE` + field DDL via [`emit_surrealql_ddl`]
/// (or build the equivalent typed `TableDefinition::new_for_ddl(...).with_*(...)`),
/// commit it to the Lance dataset, and return the new monotonic
/// `lance_version` as the `knowable_from` stamp. Persist the
/// `(class_identity, knowable_from)` mapping in a dedicated Lance row
/// so `temporal::classify` can join.
pub fn register_class_knowable_from(
    _class: &Class,
    _lance_dataset: &Path,
) -> Result<u64, RegisterError> {
    todo!(
        "needs Lance writer; gated by lance-bind Sprint-5b; the (class_identity, \
         knowable_from) mapping is the §10.3 seam"
    )
}

/// Errors from [`parse_surrealql_ddl`].
#[derive(Debug, Clone)]
pub enum ParseError {
    /// The input couldn't be tokenized / parsed by `surrealdb-parser`.
    Parse(String),
    /// A `DEFINE FIELD` couldn't be mapped to an `Attribute`/`Association`/`EnumDecl`.
    UnmappableField {
        /// Table the field is on.
        table: String,
        /// Field name.
        field: String,
        /// Why mapping failed.
        reason: String,
    },
}

/// Errors from [`register_class_knowable_from`].
#[derive(Debug, Clone)]
pub enum RegisterError {
    /// Lance dataset write failed.
    LanceWrite(String),
    /// Class is missing required identity / name.
    MalformedClass(String),
}

// ─────────────────────────────────────────────────────────────────────
// emit — hand-written formatter (see crate-level docs on alignment)
// ─────────────────────────────────────────────────────────────────────

fn emit_class(class: &Class, out: &mut String) {
    // DEFINE TABLE <name> [SCHEMAFULL];
    // SCHEMAFULL is the safe default for OGAR-produced classes — they have
    // a typed shape. If a producer needs SCHEMALESS it can wire it through
    // a Class decorator and we extend here.
    out.push_str(&format!("DEFINE TABLE {} SCHEMAFULL", class.name));
    if let Some(desc) = &class.description {
        out.push_str(&format!(" COMMENT {}", surreal_string_literal(desc)));
    }
    out.push_str(";\n");

    // DEFINE FIELD per Attribute
    for attr in &class.attributes {
        emit_field_attr(&class.name, attr, out);
    }
    // DEFINE FIELD per Association (renders as record<target>)
    for assoc in &class.associations {
        emit_field_assoc(&class.name, assoc, out);
    }
    // DEFINE FIELD per EnumDecl (renders as string ASSERT $value IN [...])
    for enum_decl in &class.enums {
        emit_field_enum(&class.name, enum_decl, out);
    }
}

fn emit_field_attr(table: &str, attr: &Attribute, out: &mut String) {
    let surreal_type = attr
        .type_name
        .as_deref()
        .map(map_type_to_surrealql)
        .unwrap_or_else(|| "string".to_string());
    out.push_str(&format!(
        "DEFINE FIELD {} ON {} TYPE {};\n",
        attr.name, table, surreal_type
    ));
}

fn emit_field_assoc(table: &str, assoc: &Association, out: &mut String) {
    // Only owning side gets a field on this table (BelongsTo).
    // HasMany/HasOne are the non-owning side — FK lives on the other table;
    // we emit a comment marker so a roundtrip via unmap can reconstruct
    // the inverse, but no DEFINE FIELD here.
    match assoc.kind {
        AssociationKind::BelongsTo => {
            let target = assoc
                .class_name
                .as_deref()
                .unwrap_or(&assoc.name); // fallback: relation name as target
            let ty = if assoc.optional.unwrap_or(false) {
                format!("option<record<{target}>>")
            } else {
                format!("record<{target}>")
            };
            out.push_str(&format!(
                "DEFINE FIELD {} ON {} TYPE {};\n",
                assoc.name, table, ty
            ));
        }
        AssociationKind::HasOne | AssociationKind::HasMany | AssociationKind::HasAndBelongsToMany => {
            // Non-owning / join-table sides: no field on this table.
            // Roundtrip note for unmap: the inverse side reconstructs from
            // the owning side's `record<X>` field on the target table.
            out.push_str(&format!(
                "-- {} {:?} {} (no DEFINE FIELD — non-owning / join side)\n",
                table, assoc.kind, assoc.name
            ));
        }
    }
}

fn emit_field_enum(table: &str, enum_decl: &EnumDecl, out: &mut String) {
    // Per `ogar-vocab::EnumDecl`: `column` names the field; `source`
    // carries the variant list (Static / Computed / Add).
    match &enum_decl.source {
        ogar_vocab::EnumSource::Static(items) => {
            let variants = items
                .iter()
                .map(|(key, _label)| surreal_string_literal(key))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "DEFINE FIELD {} ON {} TYPE string ASSERT $value IN [{}];\n",
                enum_decl.column, table, variants
            ));
        }
        ogar_vocab::EnumSource::Add { items, parent_selection } => {
            // Inherited enum: emit the added variants only; downstream
            // consumers reconcile against the parent via `parent_selection`.
            let variants = items
                .iter()
                .map(|(key, _label)| surreal_string_literal(key))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "DEFINE FIELD {} ON {} TYPE string /* selection_add from {} */ ASSERT $value IN [{}];\n",
                enum_decl.column, table, parent_selection, variants
            ));
        }
        ogar_vocab::EnumSource::Computed(body) => {
            // Runtime-computed: can't enumerate at DDL time. Emit a string
            // field with a comment preserving the lambda body for the unmap
            // side to reconstitute.
            let escaped = body.replace("*/", "* /");
            out.push_str(&format!(
                "DEFINE FIELD {} ON {} TYPE string /* computed: {} */;\n",
                enum_decl.column, table, escaped
            ));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Type mapping — Ruby/Python/Elixir scalar names -> SurrealQL types
// ─────────────────────────────────────────────────────────────────────

fn map_type_to_surrealql(producer_type: &str) -> String {
    match producer_type.to_lowercase().as_str() {
        // string family
        "string" | "char" | "text" | "html" | "binary" => "string".to_string(),
        // integer family
        "integer" | "int" | "bigint" | "big_integer" | "smallint" | "tinyint" => "int".to_string(),
        // float / decimal
        "float" | "double" | "real" => "float".to_string(),
        "decimal" | "monetary" | "numeric" => "decimal".to_string(),
        // bool / date
        "boolean" | "bool" => "bool".to_string(),
        "datetime" | "timestamp" | "datetimewithtimezone" => "datetime".to_string(),
        "date" => "datetime".to_string(),
        // identity
        "uuid" => "uuid".to_string(),
        // unknown → string with a comment (added at call site via `-- ...`)
        other => format!("string /* unmapped producer type: {other} */"),
    }
}

fn surreal_string_literal(s: &str) -> String {
    // SurrealQL string literal: single-quoted, single-quote escaped.
    let escaped = s.replace('\'', "''");
    format!("'{escaped}'")
}

// ─────────────────────────────────────────────────────────────────────
// Tests — lock the emit shape; parse_surrealql_ddl tests land with the
// parser wiring.
// ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use ogar_vocab::{EnumVariant, Language};

    #[test]
    fn emit_minimal_class_produces_define_table() {
        let c = Class::new("widget");
        let ddl = emit_surrealql_ddl(&[c]);
        assert!(ddl.contains("DEFINE TABLE widget SCHEMAFULL;"), "got: {ddl}");
    }

    #[test]
    fn emit_class_with_string_attribute() {
        let mut c = Class::new("account");
        let mut email = Attribute::new("email");
        email.type_name = Some("string".into());
        c.attributes.push(email);
        let ddl = emit_surrealql_ddl(&[c]);
        assert!(ddl.contains("DEFINE TABLE account SCHEMAFULL;"));
        assert!(ddl.contains("DEFINE FIELD email ON account TYPE string;"), "got: {ddl}");
    }

    #[test]
    fn emit_class_with_belongs_to_association_renders_record_type() {
        let mut c = Class::new("work_package");
        let mut owner = Association::new(AssociationKind::BelongsTo, "owner");
        owner.class_name = Some("user".into());
        c.associations.push(owner);
        let ddl = emit_surrealql_ddl(&[c]);
        assert!(
            ddl.contains("DEFINE FIELD owner ON work_package TYPE record<user>;"),
            "got: {ddl}"
        );
    }

    #[test]
    fn emit_class_with_optional_belongs_to_renders_option_record_type() {
        let mut c = Class::new("work_package");
        let mut assignee = Association::new(AssociationKind::BelongsTo, "assignee");
        assignee.class_name = Some("user".into());
        assignee.optional = Some(true);
        c.associations.push(assignee);
        let ddl = emit_surrealql_ddl(&[c]);
        assert!(
            ddl.contains("DEFINE FIELD assignee ON work_package TYPE option<record<user>>;"),
            "got: {ddl}"
        );
    }

    #[test]
    fn emit_class_with_has_many_does_not_define_field_on_this_table() {
        let mut c = Class::new("project");
        let assoc = Association::new(AssociationKind::HasMany, "work_packages");
        c.associations.push(assoc);
        let ddl = emit_surrealql_ddl(&[c]);
        // No DEFINE FIELD; only a comment marker (FK is on the other table)
        assert!(!ddl.contains("DEFINE FIELD work_packages"), "got: {ddl}");
        assert!(ddl.contains("(no DEFINE FIELD"), "expected non-owning-side comment, got: {ddl}");
    }

    #[test]
    fn emit_class_with_static_enum_renders_assert_in_list() {
        let mut c = Class::new("ticket");
        let status = EnumDecl {
            column: "status".into(),
            source: ogar_vocab::EnumSource::Static(vec![
                ("open".into(), "Open".into()),
                ("closed".into(), "Closed".into()),
            ]),
            scopes_disabled: None,
        };
        c.enums.push(status);
        let ddl = emit_surrealql_ddl(&[c]);
        assert!(
            ddl.contains("DEFINE FIELD status ON ticket TYPE string ASSERT $value IN ['open', 'closed'];"),
            "got: {ddl}"
        );
    }

    #[test]
    fn emit_class_with_add_enum_emits_parent_selection_marker() {
        let mut c = Class::new("account_move_line");
        let parent_enum = EnumDecl {
            column: "state".into(),
            source: ogar_vocab::EnumSource::Add {
                items: vec![("paid".into(), "Paid".into())],
                parent_selection: "account.move.line".into(),
            },
            scopes_disabled: None,
        };
        c.enums.push(parent_enum);
        let ddl = emit_surrealql_ddl(&[c]);
        assert!(
            ddl.contains("selection_add from account.move.line"),
            "expected parent-selection marker, got: {ddl}"
        );
        assert!(ddl.contains("ASSERT $value IN ['paid']"), "got: {ddl}");
    }

    #[test]
    fn emit_class_with_computed_enum_emits_lambda_marker() {
        let mut c = Class::new("address");
        let country_enum = EnumDecl {
            column: "country".into(),
            source: ogar_vocab::EnumSource::Computed(
                "lambda self: self.env[\'res.country\']...".into(),
            ),
            scopes_disabled: None,
        };
        c.enums.push(country_enum);
        let ddl = emit_surrealql_ddl(&[c]);
        assert!(
            ddl.contains("DEFINE FIELD country ON address TYPE string /* computed:"),
            "got: {ddl}"
        );
        // No ASSERT $value IN — runtime-computed
        assert!(
            !ddl.contains("ASSERT $value IN ["),
            "computed enums shouldn't ASSERT on a static list, got: {ddl}"
        );
    }

    #[test]
    fn emit_multiple_classes_in_order() {
        let cs = vec![Class::new("a"), Class::new("b"), Class::new("c")];
        let ddl = emit_surrealql_ddl(&cs);
        let a = ddl.find("DEFINE TABLE a").expect("a present");
        let b = ddl.find("DEFINE TABLE b").expect("b present");
        let c = ddl.find("DEFINE TABLE c").expect("c present");
        assert!(a < b && b < c, "classes should emit in input order");
    }

    #[test]
    fn type_mapping_covers_common_scalars() {
        assert_eq!(map_type_to_surrealql("string"), "string");
        assert_eq!(map_type_to_surrealql("integer"), "int");
        assert_eq!(map_type_to_surrealql("bigint"), "int");
        assert_eq!(map_type_to_surrealql("float"), "float");
        assert_eq!(map_type_to_surrealql("decimal"), "decimal");
        assert_eq!(map_type_to_surrealql("monetary"), "decimal");
        assert_eq!(map_type_to_surrealql("boolean"), "bool");
        assert_eq!(map_type_to_surrealql("datetime"), "datetime");
        assert_eq!(map_type_to_surrealql("uuid"), "uuid");
    }

    #[test]
    fn type_mapping_marks_unmapped_with_comment() {
        let mapped = map_type_to_surrealql("some_exotic_type");
        assert!(mapped.starts_with("string"));
        assert!(mapped.contains("unmapped producer type"));
        assert!(mapped.contains("some_exotic_type"));
    }

    #[test]
    fn elixir_class_emits_correctly_no_language_specific_paths() {
        // Smoke: Language::Elixir should not change emit output — the
        // bridge is producer-agnostic. The Language field is metadata for
        // downstream consumers, not for the DDL emitter.
        let mut c = Class::new("account");
        c.language = Language::Elixir;
        let mut email = Attribute::new("email");
        email.type_name = Some("string".into());
        c.attributes.push(email);
        let ddl = emit_surrealql_ddl(&[c]);
        assert!(ddl.contains("DEFINE TABLE account SCHEMAFULL;"));
        assert!(ddl.contains("DEFINE FIELD email ON account TYPE string;"));
    }

    /// The roundtrip property (`parse(emit(parse(x))) == parse(x)`)
    /// can't be tested until parse_surrealql_ddl is wired (pending the
    /// rust-version bump). This test is a placeholder that documents
    /// the intent and asserts the emit-only direction is stable.
    #[test]
    fn roundtrip_intent_documented() {
        // Once parse_surrealql_ddl is wired, replace this with:
        //   let parsed = parse_surrealql_ddl(emit_surrealql_ddl(&[c]))?;
        //   assert_eq!(parsed, vec![c]);  // modulo identity prefix
        let mut c = Class::new("widget");
        let mut size = Attribute::new("size");
        size.type_name = Some("int".into());
        c.attributes.push(size);
        let ddl = emit_surrealql_ddl(&[c.clone()]);
        // Emit is deterministic — same input twice gives same output:
        let ddl2 = emit_surrealql_ddl(&[c]);
        assert_eq!(ddl, ddl2);
    }
}
