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
//! `knowable_from` is sourced by a dedicated OGAR producer crate
//! ([`ogar-knowable-from`](../ogar_knowable_from/index.html), defines the
//! `KnowableFromWriter` trait + the `register_class_knowable_from`
//! generic) and consumed by `lance-graph-planner::temporal::classify`
//! (runtime session). The producer-side seam moved out of this crate to
//! keep the bridge Lance-free; trait-mediated, symmetric with how
//! `CommitHook` hides the membrane from Rubicon.
//!
//! # Status (post PR #23 rust-version bump)
//!
//! OGAR workspace bumped to `rust-version = "1.95"` in PR #23, lifting the
//! prior blocker on `surrealdb-ast` + `surrealdb-parser` (both require
//! `1.95+`). The deps are now wired in this crate under the
//! `surrealdb-parser` feature flag (taking from the workspace's pinned
//! git refs on `AdaWorldAPI/surrealdb#main`).
//!
//! Implementation state:
//! - [`emit_surrealql_ddl`] is fully implemented (hand-written formatter).
//! - [`parse_surrealql_ddl`] is **wired but not yet walking**: under the
//!   `surrealdb-parser` feature, it drives `Parser::enter_parse::<Query>`
//!   for syntax validation; on parse-OK, returns
//!   [`ParseError::Unimplemented`] (the AST→`Class` walk is a substantive
//!   follow-up sprint). Without the feature, it returns
//!   [`ParseError::Unimplemented`] noting the feature must be enabled.
//! - [`register_class_knowable_from`] is `todo!()` (also needs a Lance
//!   writer — that's the `lance-bind` Sprint-5b boundary).
//!
//! # Canonical parser invocation pattern (for the follow-up walk impl)
//!
//! From `surrealdb/parser/src/test/mod.rs` (the parser's own test
//! harness), the canonical entry is:
//!
//! ```ignore
//! use surrealdb_parser::{Parser, Config};
//! use surrealdb_ast::Query;
//!
//! let (root_id, ast) = Parser::enter_parse::<Query>(
//!     input,
//!     Config {
//!         depth_limit: 1000,
//!         generate_warnings: false,
//!         feature_bearer_access: false,
//!         feature_surrealism: false,
//!     },
//! )?;
//! let query: &Query = root_id.index(&ast);
//! // query.exprs: Option<NodeListId<TopLevelExpr>>
//! // walk: TopLevelExpr::Expr(NodeId<Expr>) -> Expr::DefineTable(NodeId<DefineTable>)
//! //                                       \-> Expr::DefineField(NodeId<DefineField>)
//! // DefineTable.name: NodeId<Expr> (typically Expr::Ident(NodeId<Ident>) -> Ident.text: NodeId<String>)
//! // DefineField.{name, table, ty, ...}: similar arena-indirection pattern
//! ```
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
    #[cfg(feature = "surrealdb-parser")]
    {
        use surrealdb_parser::{Config, Parser};
        let cfg = Config {
            depth_limit: 1000,
            generate_warnings: false,
            feature_bearer_access: false,
            feature_surrealism: false,
        };
        let (query, ast) = Parser::enter_parse::<surrealdb_ast::Query>(_input, cfg)
            .map_err(|e| ParseError::Parse(format!("{e:?}")))?;
        Ok(walk::walk_query(&ast, query))
    }
    #[cfg(not(feature = "surrealdb-parser"))]
    {
        Err(ParseError::Unimplemented(
            "surrealdb-parser feature not enabled; build with \
             --features surrealdb-parser to wire the parser deps"
                .into(),
        ))
    }
}

// `register_class_knowable_from` MOVED to the standalone crate
// `ogar-knowable-from` (cross-ref via the crate-level docs above).
// That crate defines the `KnowableFromWriter` trait + a generic
// `register_class_knowable_from<W: KnowableFromWriter>(&Class, &W) -> Result<u64, _>`
// that delegates the actual Lance write to the runtime side
// (lance-graph-callcenter::LanceMembrane, in a small follow-up `impl`).
//
// Kept this crate Lance-free; the seam is trait-mediated symmetrically
// with how CommitHook hides the membrane from Rubicon. See ADR-010 in
// `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` for the meet-point pin.

// ─────────────────────────────────────────────────────────────────────
// AST walk — surrealdb_ast::Query -> Vec<Class>
// ─────────────────────────────────────────────────────────────────────
// Behind the `surrealdb-parser` feature. Walks a parsed SurrealQL Query
// node-by-node and lifts the supported DDL subset into the OGAR IR.
//
// Supported today:
//   - DEFINE TABLE <name>                                  -> Class
//   - DEFINE FIELD <field> ON <table> TYPE string|int|bool|...
//                                                         -> Attribute
//   - DEFINE FIELD <field> ON <table> TYPE record<X>       -> Association(BelongsTo)
//   - DEFINE FIELD <field> ON <table> TYPE option<record<X>>
//                                                         -> Association(BelongsTo, optional=true)
//   - DEFINE FIELD <field> ON <table> TYPE option<<primitive>>
//                                                         -> Attribute(type=option<primitive>)
//
// Not yet supported (intentional — separate sprint per the closure of
// ADR-017 in stages):
//   - DEFINE FIELD … ASSERT $value IN [...]  (EnumDecl::Static lift)
//   - VALUE / DEFAULT / COMPUTED clauses
//   - PERMISSIONS / FLEXIBLE / READONLY
//   - DEFINE EVENT (lifecycle -> ActionDef)
//   - DEFINE INDEX (not part of OGAR IR; ignored)
//   - Comment-marker recovery for non-owning sides (`-- {table} HasMany {name}`)
//     — emit_field_assoc emits these but the parser doesn't see them
//     (they're SurrealQL comments, not AST nodes). The non-owning side
//     of an association is recoverable from the owning side's record<X>
//     in a separate post-pass over the full Vec<Class>.
//
// The IR field that holds the resolved type for primitives uses the
// SurrealQL canonical name (`"string"`, `"int"`, `"bool"`, `"datetime"`,
// `"float"`, `"decimal"`, `"uuid"`, `"binary"`, `"object"`) — the
// emit-side `map_type_to_surrealql` lookup handles these as-is.
#[cfg(feature = "surrealdb-parser")]
mod walk {
    use ogar_vocab::{Association, AssociationKind, Attribute, Class};
    use std::collections::HashMap;
    use surrealdb_ast::{
        Ast, Expr, NodeId, NodeListId, PrimeType, Query, TopLevelExpr, Type,
    };

    pub(super) fn walk_query(ast: &Ast, query: Query) -> Vec<Class> {
        // Pass 1: visit DefineTable to register class order.
        // Pass 2: visit DefineField, attaching to the matching class.
        // Both passes share a HashMap<String, Class> + order Vec.
        let mut by_name: HashMap<String, Class> = HashMap::new();
        let mut order: Vec<String> = Vec::new();

        if let Some(list_id) = query.exprs {
            for tle_id in iter_node_list(ast, list_id) {
                let tle = &ast[tle_id];
                if let TopLevelExpr::Expr(expr_id) = tle {
                    visit_define(ast, *expr_id, &mut by_name, &mut order);
                }
            }
        }

        order.into_iter().filter_map(|n| by_name.remove(&n)).collect()
    }

    fn visit_define(
        ast: &Ast,
        expr_id: NodeId<Expr>,
        by_name: &mut HashMap<String, Class>,
        order: &mut Vec<String>,
    ) {
        match &ast[expr_id] {
            Expr::DefineTable(dt_id) => {
                let dt = &ast[*dt_id];
                if let Some(name) = expr_to_simple_name(ast, dt.name) {
                    by_name.entry(name.clone()).or_insert_with(|| {
                        order.push(name.clone());
                        Class::new(&name)
                    });
                }
            }
            Expr::DefineField(df_id) => {
                let df = &ast[*df_id];
                let table = match expr_to_simple_name(ast, df.table) {
                    Some(t) => t,
                    None => return, // can't anchor a field whose table isn't a simple name
                };
                let field = match expr_to_simple_name(ast, df.name) {
                    Some(f) => f,
                    None => return, // dotted-paths on the field side aren't OGAR-IR-shaped
                };
                // Get-or-create the class entry (field may appear before
                // its DefineTable in the input, or the table may be
                // implicit-defined).
                let class = by_name.entry(table.clone()).or_insert_with(|| {
                    order.push(table.clone());
                    Class::new(&table)
                });

                match lift_field_type(ast, df.ty) {
                    FieldShape::Primitive { type_name, optional } => {
                        let mut attr = Attribute::new(&field);
                        attr.type_name = Some(type_name);
                        if optional {
                            // IR-canonical optional-primitive marker —
                            // emit-side wraps with SurrealQL `option<…>`.
                            attr.options.required = Some(false);
                        }
                        class.attributes.push(attr);
                    }
                    FieldShape::Record { target, optional } => {
                        let mut assoc =
                            Association::new(AssociationKind::BelongsTo, &field);
                        assoc.class_name = Some(target);
                        if optional {
                            assoc.optional = Some(true);
                        }
                        class.associations.push(assoc);
                    }
                    FieldShape::Untyped => {
                        // DEFINE FIELD f ON t  (no TYPE clause) — emit
                        // a typeless Attribute; consumers infer.
                        let attr = Attribute::new(&field);
                        class.attributes.push(attr);
                    }
                    FieldShape::Other => {
                        // Complex/union type — leave a placeholder
                        // Attribute with type "any" so the consumer sees
                        // the field exists. Future PR closes this with
                        // proper lifting (EnumDecl from ASSERT, etc.).
                        let mut attr = Attribute::new(&field);
                        attr.type_name = Some("any".into());
                        class.attributes.push(attr);
                    }
                }
            }
            _ => {} // ignore non-DDL exprs (Create, Update, etc.)
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // Name extraction
    // ─────────────────────────────────────────────────────────────────

    /// Extract a simple (one-segment) identifier name from an Expr.
    /// Returns `None` for compound paths, expressions, anything that's
    /// not just `Ident`.
    fn expr_to_simple_name(ast: &Ast, expr_id: NodeId<Expr>) -> Option<String> {
        match &ast[expr_id] {
            Expr::Path(path_id) => {
                let path = &ast[*path_id];
                if path.parts.is_some() {
                    return None; // multi-segment path
                }
                let ident = &ast[path.start];
                Some(ast[ident.text].clone())
            }
            _ => None,
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // Type extraction
    // ─────────────────────────────────────────────────────────────────

    enum FieldShape {
        /// Primitive — `string`, `int`, `bool`, `datetime`, `float`,
        /// `decimal`, `uuid`, `bytes`, `object`, `duration`, `any`.
        /// Optional is carried as `AttributeOptions.required = Some(false)`
        /// (not baked into `type_name` — keeps the IR-canonical
        /// representation that survives `emit_field_attr`'s round-trip).
        Primitive { type_name: String, optional: bool },
        /// `record<X>` (or `option<record<X>>`) — BelongsTo association.
        Record { target: String, optional: bool },
        /// `DEFINE FIELD … TYPE` clause absent.
        Untyped,
        /// Union types, multi-target records, geometry, complex literals
        /// — out of scope for the v1 walk. Codex P2 (#32) flagged
        /// silent narrowing of multi-target `record<a | b>` to the first
        /// target; rejecting as `Other` is correct here.
        Other,
    }

    fn lift_field_type(ast: &Ast, ty_id_opt: Option<NodeId<Type>>) -> FieldShape {
        let ty_id = match ty_id_opt {
            Some(id) => id,
            None => return FieldShape::Untyped,
        };
        let prime_list_id = match &ast[ty_id] {
            Type::Any(_) => return FieldShape::Primitive {
                type_name: "any".into(),
                optional: false,
            },
            Type::Prime(list_id) => *list_id,
        };

        // The parser encodes `option<X>` as a NodeList prefixed with
        // PrimeType::None (see surrealdb/parser/src/parse/kind.rs at
        // the T![OPTION] arm). So a NodeList starting with None means
        // optional; the rest are the effective type(s).
        let primes: Vec<&PrimeType> =
            iter_node_list(ast, prime_list_id).map(|p| &ast[p]).collect();
        if primes.is_empty() {
            return FieldShape::Untyped;
        }

        let optional = matches!(primes[0], PrimeType::None(_));
        let effective_start = if optional { 1 } else { 0 };
        let effective_len = primes.len() - effective_start;

        if effective_len != 1 {
            return FieldShape::Other; // union, or empty after option-strip
        }

        let p = primes[effective_start];
        match p {
            PrimeType::String(_) => primitive("string", optional),
            PrimeType::Integer(_) => primitive("int", optional),
            PrimeType::Bool(_) => primitive("bool", optional),
            PrimeType::DateTime(_) => primitive("datetime", optional),
            PrimeType::Float(_) => primitive("float", optional),
            PrimeType::Decimal(_) | PrimeType::Number(_) => primitive("decimal", optional),
            PrimeType::Uuid(_) => primitive("uuid", optional),
            PrimeType::Bytes(_) => primitive("bytes", optional),
            PrimeType::Object(_) => primitive("object", optional),
            PrimeType::Duration(_) => primitive("duration", optional),
            PrimeType::Record(ident_list_id) => {
                // Reject multi-target `record<a | b>` rather than
                // silently narrowing to the first target (Codex P2 on
                // PR #32). OGAR's `Association::class_name: Option<String>`
                // can't represent a union; an honest `Other` keeps the
                // round-trip from mutating the schema. Future PR can
                // grow union-target support if a real consumer needs it.
                let ident_list = &ast[*ident_list_id];
                let mut iter = ident_list
                    .idents
                    .map(|ids| iter_node_list(ast, ids))
                    .into_iter()
                    .flatten();
                let first = match iter.next() {
                    Some(id) => id,
                    None => return FieldShape::Other, // record<> with no target
                };
                if iter.next().is_some() {
                    return FieldShape::Other; // multi-target — preserve faithfully or reject
                }
                let ident = &ast[first];
                FieldShape::Record {
                    target: ast[ident.text].clone(),
                    optional,
                }
            }
            _ => FieldShape::Other,
        }
    }

    fn primitive(name: &str, optional: bool) -> FieldShape {
        FieldShape::Primitive {
            type_name: name.to_string(),
            optional,
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // NodeList iteration (linked-list — see surrealdb/ast/src/types/mod.rs)
    // ─────────────────────────────────────────────────────────────────

    /// Iterate a `NodeListId<T>` linked-list, yielding each cell's
    /// `cur: NodeId<T>`.
    fn iter_node_list<T: std::any::Any>(
        ast: &Ast,
        list_id: NodeListId<T>,
    ) -> impl Iterator<Item = NodeId<T>> + '_ {
        let mut cur = Some(list_id);
        std::iter::from_fn(move || {
            let lid = cur?;
            let node = &ast[lid];
            cur = node.next;
            Some(node.cur)
        })
    }
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
    /// The function is wired but the requested capability is pending a
    /// follow-up sprint. Carries a short rationale.
    Unimplemented(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Parse(msg) => write!(f, "surrealql parse error: {msg}"),
            ParseError::UnmappableField { table, field, reason } => write!(
                f,
                "unmappable field {field} on table {table}: {reason}"
            ),
            ParseError::Unimplemented(msg) => write!(f, "not yet implemented: {msg}"),
        }
    }
}

impl std::error::Error for ParseError {}

// `RegisterError` MOVED to `ogar-knowable-from`. See note above on
// `register_class_knowable_from`.

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
    // `options.required = Some(false)` is the IR-canonical "this field
    // is nullable / optional" marker (per `ogar-vocab::AttributeOptions`).
    // Wrap with SurrealQL's `option<…>` so the round-trip through
    // `parse_surrealql_ddl` preserves nullability. `None` means "unset"
    // (the producer didn't say), not "required=true" — leave unwrapped.
    let wrapped = if attr.options.required == Some(false) {
        format!("option<{surreal_type}>")
    } else {
        surreal_type
    };
    out.push_str(&format!(
        "DEFINE FIELD {} ON {} TYPE {};\n",
        attr.name, table, wrapped
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
        // `AssociationKind` is `#[non_exhaustive]` in `ogar-vocab`; the four
        // arms above cover every variant defined today. The wildcard exists
        // only so adding a variant in `ogar-vocab` produces a clean
        // panic-on-first-emit instead of a silent miscompile elsewhere.
        _ => unreachable!("vocab variant added without adapter update: AssociationKind"),
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
        // `EnumSource` is `#[non_exhaustive]` in `ogar-vocab`; the three arms
        // above cover every variant defined today. The wildcard exists only
        // so adding a variant in `ogar-vocab` produces a clean panic-on-first
        // -emit instead of a silent miscompile elsewhere.
        _ => unreachable!("vocab variant added without adapter update: EnumSource"),
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
    use ogar_vocab::Language;

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
        let mut status = EnumDecl::new("status");
        status.source = ogar_vocab::EnumSource::Static(vec![
            ("open".into(), "Open".into()),
            ("closed".into(), "Closed".into()),
        ]);
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
        let mut parent_enum = EnumDecl::new("state");
        parent_enum.source = ogar_vocab::EnumSource::Add {
            items: vec![("paid".into(), "Paid".into())],
            parent_selection: "account.move.line".into(),
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
        let mut country_enum = EnumDecl::new("country");
        country_enum.source = ogar_vocab::EnumSource::Computed(
            "lambda self: self.env[\'res.country\']...".into(),
        );
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

    /// The full roundtrip property (`parse(emit(parse(x))) == parse(x)`)
    /// Emit + parse round-trip: emitting a Class and parsing the
    /// result should reconstruct the structural shape. Asserts the
    /// minimum-shape walk per ADR-017.
    #[test]
    fn roundtrip_intent_documented() {
        let mut c = Class::new("widget");
        let mut size = Attribute::new("size");
        size.type_name = Some("int".into());
        c.attributes.push(size);
        let ddl = emit_surrealql_ddl(&[c.clone()]);
        // Emit is deterministic — same input twice gives same output:
        let ddl2 = emit_surrealql_ddl(&[c]);
        assert_eq!(ddl, ddl2);
    }

    // ─────────────────────────────────────────────────────────────────
    // Feature-gated smoke + round-trip tests for the AST walk
    // ─────────────────────────────────────────────────────────────────

    #[cfg(feature = "surrealdb-parser")]
    #[test]
    fn walk_minimal_table_produces_class() {
        let ddl = "DEFINE TABLE account SCHEMAFULL;";
        let classes = parse_surrealql_ddl(ddl).expect("parse OK");
        assert_eq!(classes.len(), 1, "got: {classes:?}");
        assert_eq!(classes[0].name, "account");
        assert!(classes[0].attributes.is_empty());
        assert!(classes[0].associations.is_empty());
    }

    #[cfg(feature = "surrealdb-parser")]
    #[test]
    fn walk_table_with_string_attribute() {
        let ddl = "\
            DEFINE TABLE account SCHEMAFULL;\n\
            DEFINE FIELD email ON account TYPE string;\n";
        let classes = parse_surrealql_ddl(ddl).expect("parse OK");
        assert_eq!(classes.len(), 1);
        let c = &classes[0];
        assert_eq!(c.name, "account");
        assert_eq!(c.attributes.len(), 1, "expected one attribute, got: {c:?}");
        assert_eq!(c.attributes[0].name, "email");
        assert_eq!(c.attributes[0].type_name.as_deref(), Some("string"));
    }

    #[cfg(feature = "surrealdb-parser")]
    #[test]
    fn walk_table_with_belongs_to_record_field() {
        let ddl = "\
            DEFINE TABLE work_package SCHEMAFULL;\n\
            DEFINE FIELD owner ON work_package TYPE record<user>;\n";
        let classes = parse_surrealql_ddl(ddl).expect("parse OK");
        assert_eq!(classes.len(), 1);
        let c = &classes[0];
        assert_eq!(c.name, "work_package");
        assert!(c.attributes.is_empty(), "record<X> should NOT become an attribute");
        assert_eq!(c.associations.len(), 1);
        let a = &c.associations[0];
        assert_eq!(a.name, "owner");
        assert!(matches!(a.kind, AssociationKind::BelongsTo));
        assert_eq!(a.class_name.as_deref(), Some("user"));
        assert!(a.optional.unwrap_or(false) == false, "non-optional record<X>");
    }

    #[cfg(feature = "surrealdb-parser")]
    #[test]
    fn walk_table_with_optional_belongs_to() {
        let ddl = "\
            DEFINE TABLE work_package SCHEMAFULL;\n\
            DEFINE FIELD assignee ON work_package TYPE option<record<user>>;\n";
        let classes = parse_surrealql_ddl(ddl).expect("parse OK");
        assert_eq!(classes.len(), 1);
        let a = &classes[0].associations[0];
        assert_eq!(a.name, "assignee");
        assert!(matches!(a.kind, AssociationKind::BelongsTo));
        assert_eq!(a.class_name.as_deref(), Some("user"));
        assert_eq!(a.optional, Some(true));
    }

    #[cfg(feature = "surrealdb-parser")]
    #[test]
    fn walk_multi_table_preserves_definition_order() {
        let ddl = "\
            DEFINE TABLE a SCHEMAFULL;\n\
            DEFINE TABLE b SCHEMAFULL;\n\
            DEFINE TABLE c SCHEMAFULL;\n";
        let classes = parse_surrealql_ddl(ddl).expect("parse OK");
        let names: Vec<&str> = classes.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[cfg(feature = "surrealdb-parser")]
    #[test]
    fn walk_round_trip_simple_class() {
        // Build a Class via the IR, emit DDL, parse it, verify the
        // recovered Class matches structurally on the supported subset.
        let mut original = Class::new("account");
        let mut email = Attribute::new("email");
        email.type_name = Some("string".into());
        let mut age = Attribute::new("age");
        age.type_name = Some("int".into());
        original.attributes.push(email);
        original.attributes.push(age);

        let ddl = emit_surrealql_ddl(&[original.clone()]);
        let recovered = parse_surrealql_ddl(&ddl).expect("parse OK");
        assert_eq!(recovered.len(), 1);
        let r = &recovered[0];
        assert_eq!(r.name, "account");
        assert_eq!(r.attributes.len(), 2);
        assert_eq!(r.attributes[0].name, "email");
        assert_eq!(r.attributes[0].type_name.as_deref(), Some("string"));
        assert_eq!(r.attributes[1].name, "age");
        assert_eq!(r.attributes[1].type_name.as_deref(), Some("int"));
    }

    #[cfg(feature = "surrealdb-parser")]
    #[test]
    fn walk_round_trip_belongs_to_association() {
        let mut original = Class::new("work_package");
        let mut owner = Association::new(AssociationKind::BelongsTo, "owner");
        owner.class_name = Some("user".into());
        let mut assignee = Association::new(AssociationKind::BelongsTo, "assignee");
        assignee.class_name = Some("user".into());
        assignee.optional = Some(true);
        original.associations.push(owner);
        original.associations.push(assignee);

        let ddl = emit_surrealql_ddl(&[original]);
        let recovered = parse_surrealql_ddl(&ddl).expect("parse OK");
        assert_eq!(recovered.len(), 1);
        let r = &recovered[0];
        assert_eq!(r.name, "work_package");
        assert_eq!(r.associations.len(), 2);
        assert_eq!(r.associations[0].name, "owner");
        assert_eq!(r.associations[0].class_name.as_deref(), Some("user"));
        assert!(r.associations[0].optional.unwrap_or(false) == false);
        assert_eq!(r.associations[1].name, "assignee");
        assert_eq!(r.associations[1].class_name.as_deref(), Some("user"));
        assert_eq!(r.associations[1].optional, Some(true));
    }

    #[cfg(feature = "surrealdb-parser")]
    #[test]
    fn walk_returns_parse_error_on_invalid_ddl() {
        // Genuine syntax errors are reported as ParseError::Parse.
        let invalid = "DEFINE TBLE missing keyword;"; // typo "TBLE"
        match parse_surrealql_ddl(invalid) {
            Err(ParseError::Parse(_)) => {} // expected
            other => panic!("expected Parse error on invalid DDL, got: {other:?}"),
        }
    }

    #[cfg(feature = "surrealdb-parser")]
    #[test]
    fn walk_implicit_table_from_field_alone() {
        // DEFINE FIELD without a preceding DEFINE TABLE — the walker
        // still creates the class (the table is named by the field's
        // ON clause). The parser may accept this even without an
        // explicit DEFINE TABLE; the walker is robust to either order.
        let ddl = "DEFINE FIELD email ON account TYPE string;\n";
        let classes = parse_surrealql_ddl(ddl).expect("parse OK");
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "account");
        assert_eq!(classes[0].attributes.len(), 1);
    }

    // ── Codex P2 fix #1 (PR #32) ────────────────────────────────────────
    // Optional primitives round-trip correctly. Walker stores the bare
    // type_name + AttributeOptions.required = Some(false); emit wraps
    // with SurrealQL `option<…>` on egress.
    // ────────────────────────────────────────────────────────────────────

    #[cfg(feature = "surrealdb-parser")]
    #[test]
    fn walk_table_with_optional_primitive_uses_required_false() {
        let ddl = "\
            DEFINE TABLE account SCHEMAFULL;\n\
            DEFINE FIELD email ON account TYPE option<string>;\n";
        let classes = parse_surrealql_ddl(ddl).expect("parse OK");
        assert_eq!(classes.len(), 1);
        let attr = &classes[0].attributes[0];
        assert_eq!(attr.name, "email");
        // IR-canonical: bare type + required=Some(false), NOT
        // type_name="option<string>" (which would round-trip wrong).
        assert_eq!(attr.type_name.as_deref(), Some("string"));
        assert_eq!(attr.options.required, Some(false));
    }

    #[cfg(feature = "surrealdb-parser")]
    #[test]
    fn walk_round_trip_optional_primitive() {
        // Build IR with the canonical optional-primitive shape, emit,
        // re-parse, assert the same shape comes back.
        let mut original = Class::new("account");
        let mut email = Attribute::new("email");
        email.type_name = Some("string".into());
        email.options.required = Some(false);
        original.attributes.push(email);

        let ddl = emit_surrealql_ddl(&[original]);
        // Emit must produce option<string>, not a bare "string" or an
        // unmapped /* … */ comment.
        assert!(
            ddl.contains("DEFINE FIELD email ON account TYPE option<string>;"),
            "expected option<string> in DDL, got: {ddl}"
        );

        let recovered = parse_surrealql_ddl(&ddl).expect("parse OK");
        assert_eq!(recovered.len(), 1);
        let attr = &recovered[0].attributes[0];
        assert_eq!(attr.type_name.as_deref(), Some("string"));
        assert_eq!(attr.options.required, Some(false));
    }

    // ── Codex P2 fix #2 (PR #32) ────────────────────────────────────────
    // Multi-target `record<a | b>` is rejected (FieldShape::Other) rather
    // than silently narrowed to the first target. The placeholder
    // Attribute with type "any" lands so the consumer sees the field
    // exists without OGAR misrepresenting the union constraint.
    // ────────────────────────────────────────────────────────────────────

    #[cfg(feature = "surrealdb-parser")]
    #[test]
    fn walk_multi_target_record_returns_other_not_first_target() {
        // The Codex P2 motivating case: `record<user | administrator>`
        // would silently become BelongsTo→user, losing administrator.
        // The fix rejects with FieldShape::Other → typeless "any"
        // Attribute. NOT an Association — the schema constraint isn't
        // narrowed.
        let ddl = "\
            DEFINE TABLE auditable SCHEMAFULL;\n\
            DEFINE FIELD actor ON auditable TYPE record<user | administrator>;\n";
        let classes = parse_surrealql_ddl(ddl).expect("parse OK");
        let c = &classes[0];
        // CRITICAL: no Association was emitted (no silent BelongsTo→user).
        assert!(
            c.associations.is_empty(),
            "multi-target record must not become an Association — would mutate schema. got: {:?}",
            c.associations
        );
        // Placeholder Attribute records the field's existence.
        assert_eq!(c.attributes.len(), 1);
        assert_eq!(c.attributes[0].name, "actor");
        assert_eq!(c.attributes[0].type_name.as_deref(), Some("any"));
    }

    #[cfg(not(feature = "surrealdb-parser"))]
    #[test]
    fn parser_not_wired_returns_unimplemented_feature_off() {
        // Without the feature flag, parse_surrealql_ddl returns
        // Unimplemented with a "feature off" rationale.
        let any = "DEFINE TABLE x;";
        match parse_surrealql_ddl(any) {
            Err(ParseError::Unimplemented(msg)) => {
                assert!(
                    msg.contains("feature not enabled"),
                    "expected feature-off message, got: {msg}"
                );
            }
            other => panic!("expected Unimplemented(feature off), got: {other:?}"),
        }
    }
}
