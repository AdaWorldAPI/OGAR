//! `TsInterface` emitter — T2 from the Northstar plan §3.
//!
//! Lifts an [`ogar_vocab::Class`] into a TypeScript `interface` declaration
//! with a `CLASS_ID` const (the OGAR codebook id, `as const` so consumers
//! get the literal type). Mirrors the shape of
//! [`crate::artifact_kinds::rust_struct`] — different template, different
//! binding-struct field names, same canonical input.
//!
//! Per Northstar §1.6 (mass-mail templates): the template names variables,
//! the binding struct supplies them. The Rust ↔ TS difference lives in the
//! type mapping (`rails_to_ts_type`) and the identifier escape rules
//! (TypeScript allows almost anything as a property name, but a small set
//! of JS keywords is safest quoted) — the rest is the same shape.

use askama::Template;

use super::ArtifactEmitter;
use crate::spec::ArtifactSpec;
use ogar_vocab::{canonical_concept_id, AssociationKind};

#[derive(Template)]
#[template(path = "dispatch/ts_interface.askama", escape = "none")]
struct TsInterfaceCtx {
    name: String,
    concept_fn: String,
    canonical_concept: String,
    class_id_hex: String,
    attributes: Vec<TsAttr>,
    associations: Vec<TsEdge>,
}

struct TsAttr {
    name: String,
    /// The property name as it appears in the emitted interface. Either
    /// a bare identifier or a quoted string for safety on JS reserved
    /// words. See [`escape_ts_property`].
    ts_name: String,
    ts_type: String,
    type_name: String,
}

struct TsEdge {
    name: String,
    ts_name: String,
    ts_type: String,
    kind_label: String,
    target: String,
}

/// The concrete emitter for
/// [`ArtifactKind::TsInterface`](crate::ArtifactKind::TsInterface).
pub struct TsInterfaceEmitter;

impl ArtifactEmitter for TsInterfaceEmitter {
    fn emit(&self, spec: &ArtifactSpec<'_>) -> Result<String, askama::Error> {
        let class = spec.class;
        let concept = class.canonical_concept.as_deref().unwrap_or("");
        let class_id_hex = canonical_concept_id(concept)
            .map(|id| format!("0x{id:04X}"))
            .unwrap_or_default();

        let attributes = class
            .attributes
            .iter()
            .map(|a| TsAttr {
                name: a.name.clone(),
                ts_name: escape_ts_property(&a.name),
                ts_type: rails_to_ts_type(a.type_name.as_deref()),
                type_name: a.type_name.clone().unwrap_or_default(),
            })
            .collect();

        let associations = class
            .associations
            .iter()
            .map(|a| TsEdge {
                name: a.name.clone(),
                ts_name: escape_ts_property(&a.name),
                ts_type: edge_ts_type(a),
                kind_label: assoc_label(a.kind),
                target: a.class_name.clone().unwrap_or_default(),
            })
            .collect();

        TsInterfaceCtx {
            name: class.name.clone(),
            concept_fn: concept.to_string(),
            canonical_concept: concept.to_string(),
            class_id_hex,
            attributes,
            associations,
        }
        .render()
    }
}

/// Quote JS reserved words / non-identifier names so the emitted TS
/// compiles. TypeScript accepts any string-literal property name in an
/// interface body (`"type": string;`), so escaping by quoting is the
/// cleanest safe move. Identifiers that are unambiguously bare (snake_case
/// without reserved-word collisions, no embedded dots / spaces) pass
/// through unchanged.
///
/// Conservative list: the union of strict JS reserved words plus a few
/// contextual ones that consumers sometimes trip over (`type` in TS
/// declaration contexts; safe here because we're in an interface body,
/// but quoting costs nothing).
fn escape_ts_property(name: &str) -> String {
    // Property names containing characters that can't appear in a bare
    // identifier must be quoted (e.g. Odoo-style `"account.move.line"`).
    let ident_safe = !name.is_empty()
        && name
            .chars()
            .enumerate()
            .all(|(i, c)| c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()));
    const QUOTED: &[&str] = &[
        // Strict JS reserved words (subset that field names sometimes hit).
        "break", "case", "catch", "class", "const", "continue", "debugger",
        "default", "delete", "do", "else", "enum", "export", "extends",
        "false", "finally", "for", "function", "if", "import", "in",
        "instanceof", "new", "null", "return", "super", "switch", "this",
        "throw", "true", "try", "typeof", "var", "void", "while", "with",
        "yield",
        // ES strict-mode reserved.
        "let", "static", "implements", "interface", "package", "private",
        "protected", "public",
        // TypeScript contextual keywords — bare in property position is
        // technically accepted, but consumers' linters / downstream
        // codegen often choke. Conservative quote: zero downside, removes
        // a class of "compiles here, breaks at consumer" footguns. This
        // is the TS-side companion of Rust's `r#type` escape (codex P1
        // on #78).
        "type", "async", "await", "as", "from", "of", "is", "infer",
        "keyof", "namespace", "satisfies", "readonly",
    ];
    if !ident_safe || QUOTED.contains(&name) {
        format!("\"{name}\"")
    } else {
        name.to_string()
    }
}

/// Map a producer-side Rails type name onto a TypeScript type. Coarse —
/// downstream consumers can specialise (e.g. `Decimal` strings vs
/// `number`); the canonical contract round-trips on the structural shape.
fn rails_to_ts_type(t: Option<&str>) -> String {
    match t {
        Some("string") | Some("text") => "string".into(),
        Some("integer") | Some("big_integer") | Some("bigint") => "number".into(),
        Some("float") | Some("double") => "number".into(),
        Some("decimal") | Some("monetary") => "number".into(),
        Some("boolean") | Some("bool") => "boolean".into(),
        // ISO-8601 string at the canonical layer; consumers may parse to
        // Date downstream.
        Some("date") | Some("datetime") | Some("timestamp") => "string".into(),
        Some("json") | Some("jsonb") => "unknown".into(),
        Some(_) | None => "string".into(),
    }
}

fn edge_ts_type(a: &ogar_vocab::Association) -> String {
    // Same coarse shape as the Rust emitter: belongs_to / has_one → fk id
    // (nullable); has_many / habtm → array of fk ids. Concrete consumers
    // can swap for typed object references downstream.
    match a.kind {
        AssociationKind::HasMany | AssociationKind::HasAndBelongsToMany => {
            "ReadonlyArray<number>".into()
        }
        _ => "number | null".into(),
    }
}

fn assoc_label(k: AssociationKind) -> String {
    match k {
        AssociationKind::BelongsTo => "belongs_to".into(),
        AssociationKind::HasOne => "has_one".into(),
        AssociationKind::HasMany => "has_many".into(),
        AssociationKind::HasAndBelongsToMany => "has_and_belongs_to_many".into(),
        _ => format!("{k:?}").to_ascii_lowercase(),
    }
}
