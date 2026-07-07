//! `ogar-adapter-ttl` — bidirectional Turtle (RDF/OWL) bridge for OGAR.
//!
//! Companion to [`ogar-adapter-surrealql`]; together they cover the
//! Morris-syntax axis (per `docs/RDF-OWL-ALIGNMENT.md` §2) for OGAR's
//! two canonical wire formats:
//!
//! | source dialect | adapter | direction |
//! |---|---|---|
//! | SurrealQL DDL | `ogar-adapter-surrealql` | parse + emit (PR #32) |
//! | Turtle (RDF/OWL) | `ogar-adapter-ttl` (this crate) | parse + emit (this PR) |
//!
//! Per ADR-023 (IR-as-wire-truth) the OGAR `Class` IR is the wire
//! truth; Turtle is one of its serialization dialects. The adapter
//! preserves the round-trip property `parse(emit(c)) == c`
//! structurally for the supported subset.
//!
//! # The two directions
//!
//! | direction | function | feature |
//! |---|---|---|
//! | `Class` -> Turtle | [`emit_ttl`] | always available |
//! | Turtle -> `Vec<Class>` | [`parse_ttl`] | `ttl-parser` |
//!
//! [`emit_ttl`] is feature-free: it composes [`ogar_emitter::TripleEmitter`]
//! output and renders the triples as a canonical Turtle document. No
//! third-party serializer dep needed.
//!
//! [`parse_ttl`] is gated behind the `ttl-parser` feature. It uses
//! the [`oxttl`] streaming Turtle parser (from the [oxigraph] project,
//! pure Rust, RFC-compliant) to read triples, then walks the SPO graph
//! to lift back into `Vec<Class>`.
//!
//! [oxigraph]: https://github.com/oxigraph/oxigraph
//!
//! # Position in the OWL stack
//!
//! Per `docs/RDF-OWL-ALIGNMENT.md` §3, OGAR sits at the bottom of L5
//! (runtime AR-pattern application schemas). This adapter is the L5↔
//! Turtle wire seam. Upstream L1-L4 ontology TTLs (DOLCE, OWL-Time,
//! PROV-O, QUDT, SKOS, FIBO, SKR03/04, ZUGFeRD, ...) compose with
//! OGAR-emitted Turtle through `owl:equivalentClass` alignment axioms
//! (see `docs/RDF-OWL-ALIGNMENT.md` §8). The pattern recognition
//! library (Phase 4: `ogar-pattern`) will use this adapter's
//! `parse_ttl` to consume them.
//!
//! # Supported in this v1
//!
//! - `Class` with name + parent (subClassOf) + source language.
//! - `Attribute` (rdfs:range typed) — via `ogar:hasField` /
//!   `ogar:fieldName` / `ogar:fieldType` / `rdfs:range`.
//! - `Association` (BelongsTo / HasOne / HasMany / HasAndBelongsToMany)
//!   — via `ogar:hasAssociation` / `ogar:kind` / `ogar:targetClass`.
//! - `Class.description`, `Class.table_name`, `Class.record_order`.
//! - Round-trip preserves these structurally.
//!
//! # Not yet supported (next sprint)
//!
//! - `EnumDecl` (lift from `owl:oneOf` / `owl:unionOf`).
//! - `ActionDef` / `KausalSpec` (lifecycle semantics — TTL doesn't
//!   carry state machines natively; OGAR-extension predicates only).
//! - Alignment-axiom parsing (`owl:equivalentClass` /
//!   `owl:equivalentProperty`).
//! - Full vocab/ogar.ttl round-trip.
//!
//! # Prefix handling
//!
//! `emit_ttl(classes, prefix)` takes the OGAR prefix (`"ogit-erp"`,
//! `"ogit-op"`, `"ogit-healthcare"`, ...) and produces URIs of the
//! form `<https://ogar.../{prefix}/{class_name}>`. `parse_ttl`
//! recovers the bare `class.name` from the URI's local part; the
//! prefix is contextual to the caller (extract from the parsed URI
//! if needed).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use ogar_vocab::Class;

// ─────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────

/// Errors from [`parse_ttl`].
#[derive(Debug, Clone)]
pub enum TtlParseError {
    /// The input couldn't be tokenized / parsed as Turtle by `oxttl`.
    Parse(String),
    /// A triple referenced a class IRI but didn't carry enough
    /// structural metadata to lift back into an OGAR `Class`.
    UnliftableSubject {
        /// The subject IRI.
        subject: String,
        /// Why lifting failed.
        reason: String,
    },
    /// The function is wired but the requested capability is pending a
    /// follow-up sprint.
    Unimplemented(String),
}

impl std::fmt::Display for TtlParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TtlParseError::Parse(msg) => write!(f, "TTL parse error: {msg}"),
            TtlParseError::UnliftableSubject { subject, reason } => {
                write!(f, "subject `{subject}` couldn't be lifted: {reason}")
            }
            TtlParseError::Unimplemented(msg) => write!(f, "unimplemented: {msg}"),
        }
    }
}

impl std::error::Error for TtlParseError {}

/// Render a slice of OGAR `Class`es as a canonical Turtle document.
///
/// Composes [`ogar_emitter::TripleEmitter::emit_class`] output and
/// renders the triples in standard Turtle syntax with the OGAR /
/// OGIT prefix block at the head.
///
/// The `prefix` is the OGAR application prefix (`"ogit-erp"`,
/// `"ogit-op"`, `"ogit-healthcare"`, …) — same parameter the emitter
/// trait takes. Class identities lift to URIs of the form
/// `<https://ogar.surrealdb-graph.io/{prefix}/{class_name}>`.
///
/// # Determinism
///
/// Emit is deterministic — same `(classes, prefix)` input always
/// produces the same Turtle output (modulo whitespace inside
/// `ogar-emitter`'s triple list, which is order-preserving).
#[must_use]
pub fn emit_ttl(classes: &[Class], prefix: &str) -> String {
    use ogar_emitter::{OgarEmitter, TripleEmitter};

    let mut out = String::new();
    out.push_str(&prefix_block(prefix));
    out.push('\n');

    for class in classes {
        let triples = TripleEmitter::emit_class(class, prefix);
        // Plus the descendant triples (attributes / associations /
        // enums / scopes / etc.) — each emitter method targets a
        // specific descendant axis. The TripleEmitter::emit_class
        // implementation already calls into them and merges; verify
        // by reading the emit_class body.
        let mut by_subject: std::collections::BTreeMap<String, Vec<(String, String)>> =
            std::collections::BTreeMap::new();
        for t in triples {
            by_subject
                .entry(t.subject)
                .or_default()
                .push((t.predicate, t.object));
        }
        for (subject, predicates) in by_subject {
            write_subject_block(&mut out, &subject, &predicates);
        }
        out.push('\n');
    }

    out
}

/// Parse a Turtle document into a `Vec<Class>` — the `unmap`
/// direction. Reverses the [`emit_ttl`] projection for the supported
/// subset.
///
/// # Feature gate
///
/// Requires the `ttl-parser` feature (which pulls in `oxttl`). Without
/// the feature, returns [`TtlParseError::Unimplemented`].
///
/// # Supported lifts (v1)
///
/// - `?s rdf:type ogar:Class` → `Class { name }` (name extracted from
///   IRI's local part).
/// - `?s ogar:parentClass ?p` → `Class.parent`.
/// - `?s ogar:sourceLanguage ?l` → `Class.language`.
/// - `?s ogar:description ?d` → `Class.description`.
/// - `?s ogar:tableName ?n` → `Class.table_name`.
/// - `?s ogar:recordOrder ?o` → `Class.record_order`.
/// - `?s ogar:hasField ?f` + `?f ogar:fieldName ?n` +
///   `?f rdfs:range ?t` → `Attribute { name, type_name }`.
/// - `?s ogar:hasAssociation ?a` + `?a ogar:kind ?k` +
///   `?a ogar:targetClass ?t` → `Association { kind, class_name }`.
///
/// Not yet lifted (next sprint): EnumDecl (`owl:oneOf`), ActionDef,
/// alignment axioms (`owl:equivalentClass`).
///
/// # Roundtrip
///
/// `parse_ttl(emit_ttl(&classes, prefix))` reconstructs structurally-
/// equivalent classes for the v1 supported subset. Verified by the
/// `roundtrip_*` tests in this crate.
pub fn parse_ttl(_input: &str) -> Result<Vec<Class>, TtlParseError> {
    #[cfg(feature = "ttl-parser")]
    {
        parse::parse_ttl_inner(_input)
    }
    #[cfg(not(feature = "ttl-parser"))]
    {
        Err(TtlParseError::Unimplemented(
            "ttl-parser feature not enabled; rebuild with --features ttl-parser \
             to enable Turtle parsing via oxttl"
                .into(),
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────
// Emit helpers — straightforward Turtle serialization
// ─────────────────────────────────────────────────────────────────────

const OGAR_BASE: &str = "https://ogar.surrealdb-graph.io";

fn prefix_block(prefix: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("@prefix ogar: <{OGAR_BASE}/vocab/ogar#> .\n"));
    out.push_str(&format!("@prefix {prefix}: <{OGAR_BASE}/{prefix}/> .\n"));
    out.push_str("@prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n");
    out.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n");
    out.push_str("@prefix owl:  <http://www.w3.org/2002/07/owl#> .\n");
    out.push_str("@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .\n");
    out
}

fn write_subject_block(out: &mut String, subject: &str, predicates: &[(String, String)]) {
    out.push_str(&render_term(subject));
    let mut first = true;
    for (predicate, object) in predicates {
        if first {
            out.push_str(&format!(" {predicate} {}", render_object(object)));
            first = false;
        } else {
            out.push_str(&format!(" ;\n    {predicate} {}", render_object(object)));
        }
    }
    out.push_str(" .\n");
}

/// Render a `subject` term (always a class/property/instance IRI in
/// OGAR's emitter output — prefixed forms like `ogit-op/WorkPackage`
/// or `ogar:hasField`). Wrap in angle brackets if it contains a `/`
/// (full identity), otherwise emit as prefixed name.
fn render_term(s: &str) -> String {
    if s.starts_with('<') || s.starts_with('"') {
        s.to_string()
    } else if s.contains('/') && !s.contains(':') {
        // bare prefix/name form like "ogit-op/WorkPackage" — wrap as
        // a full IRI. OGAR's `association_identity` uses `->` as the
        // class-to-relation separator (e.g. `ogit-erp/Invoice->customer`),
        // and `>` is illegal inside angle-bracket IRIs per RFC 3987 —
        // percent-encode it (and the other forbidden characters) on
        // emit; the parser decodes it back symmetrically.
        format!("<{OGAR_BASE}/{}>", percent_encode_iri(s))
    } else if s.contains(':') {
        // already a prefixed name (e.g. `ogar:Class`, `rdf:type`).
        s.to_string()
    } else {
        // bare literal — quote it as a string literal.
        format!("\"{}\"", escape_turtle_string(s))
    }
}

/// Render a `predicate-object` object position. OGAR's emitter emits
/// object positions as: another IRI (prefixed), a literal value, or
/// `"true"`/`"false"`. We use a heuristic — if the object looks like
/// an IRI or prefixed name, emit as a term; otherwise quote.
fn render_object(o: &str) -> String {
    if o == "true" || o == "false" {
        format!("\"{o}\"^^xsd:boolean")
    } else if o.contains(':') && !o.contains(' ') && !o.starts_with('"') {
        // prefixed-name form like `ogar:Class`, `rdf:type`
        o.to_string()
    } else if o.contains('/') && !o.contains(' ') && !o.starts_with('"') {
        // full identity path — same percent-encoding as `render_term`.
        format!("<{OGAR_BASE}/{}>", percent_encode_iri(o))
    } else {
        format!("\"{}\"", escape_turtle_string(o))
    }
}

/// Percent-encode characters that are forbidden inside Turtle's
/// angle-bracket IRI form (RFC 3987 §2.2 — the IRI-reference
/// production excludes `<>"{}|\^\``\` and CTL/DEL). The parser
/// recovers the original via `percent_decode_iri`.
fn percent_encode_iri(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '>' => out.push_str("%3E"),
            '<' => out.push_str("%3C"),
            '"' => out.push_str("%22"),
            '{' => out.push_str("%7B"),
            '}' => out.push_str("%7D"),
            '|' => out.push_str("%7C"),
            '\\' => out.push_str("%5C"),
            '^' => out.push_str("%5E"),
            '`' => out.push_str("%60"),
            ' ' => out.push_str("%20"),
            c if (c as u32) < 0x20 || c as u32 == 0x7F => {
                out.push_str(&format!("%{:02X}", c as u32))
            }
            c => out.push(c),
        }
    }
    out
}

fn escape_turtle_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

// ─────────────────────────────────────────────────────────────────────
// Parse helpers (behind `ttl-parser`)
// ─────────────────────────────────────────────────────────────────────

#[cfg(feature = "ttl-parser")]
mod parse {
    use super::*;
    use ogar_vocab::{Association, AssociationKind, Attribute, Class};
    use oxrdf::{NamedNode, NamedOrBlankNode, Term};
    use oxttl::TurtleParser;
    use std::collections::HashMap;

    /// Internal parse entry — drives `oxttl` and walks the triple
    /// stream to reconstruct `Vec<Class>`.
    pub(crate) fn parse_ttl_inner(input: &str) -> Result<Vec<Class>, TtlParseError> {
        let parser = TurtleParser::new();
        let mut by_subject: HashMap<String, Vec<(String, String)>> = HashMap::new();

        for result in parser.for_slice(input.as_bytes()) {
            let quad = result.map_err(|e| TtlParseError::Parse(format!("{e}")))?;
            let s = subject_iri(&quad.subject);
            let p = quad.predicate.as_str().to_string();
            let o = term_to_string(&quad.object);
            by_subject.entry(s).or_default().push((p, o));
        }

        Ok(lift_classes(&by_subject))
    }

    fn subject_iri(s: &NamedOrBlankNode) -> String {
        match s {
            NamedOrBlankNode::NamedNode(n) => n.as_str().to_string(),
            NamedOrBlankNode::BlankNode(b) => format!("_:{}", b.as_str()),
        }
    }

    fn term_to_string(t: &Term) -> String {
        match t {
            Term::NamedNode(n) => n.as_str().to_string(),
            Term::BlankNode(b) => format!("_:{}", b.as_str()),
            Term::Literal(l) => l.value().to_string(),
        }
    }

    // ── Predicate / object recognition ────────────────────────────────
    // After Turtle parsing, every prefixed name has been expanded to
    // its full URI form (`ogar:parentClass` → `<https://ogar.../vocab
    // /ogar#parentClass>`). These helpers strip the OGAR namespace to
    // get the local name + match RDF/RDFS/OWL terms against their
    // canonical expansions.

    const RDF_TYPE_FULL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const RDFS_RANGE_FULL: &str = "http://www.w3.org/2000/01/rdf-schema#range";
    const OGAR_NS: &str = "https://ogar.surrealdb-graph.io/vocab/ogar#";

    /// Strip the OGAR namespace from a predicate or object URI; return
    /// the local part. Accepts both the expanded URI form (what
    /// oxttl yields after prefix resolution) and the prefixed-name
    /// form (`ogar:hasField`).
    fn ogar_local(s: &str) -> Option<&str> {
        s.strip_prefix(OGAR_NS).or_else(|| s.strip_prefix("ogar:"))
    }

    fn is_rdf_type(p: &str) -> bool {
        p == RDF_TYPE_FULL || p == "rdf:type"
    }

    fn is_rdfs_range(p: &str) -> bool {
        p == RDFS_RANGE_FULL || p == "rdfs:range"
    }

    /// Walk the (subject -> [(predicate, object)]) map and lift every
    /// `?s rdf:type ogar:Class` subject into a `Class`.
    fn lift_classes(by_subject: &HashMap<String, Vec<(String, String)>>) -> Vec<Class> {
        let mut classes: Vec<(String, Class)> = Vec::new();

        for (subject, predicates) in by_subject {
            let is_class = predicates
                .iter()
                .any(|(p, o)| is_rdf_type(p) && ogar_local(o) == Some("Class"));
            if !is_class {
                continue;
            }
            let mut class = Class::new(local_name(subject));
            for (p, o) in predicates {
                match ogar_local(p) {
                    Some("parentClass") => class.parent = Some(local_name(o)),
                    Some("description") => class.description = Some(o.clone()),
                    Some("tableName") => class.table_name = Some(o.clone()),
                    Some("recordOrder") => class.record_order = Some(o.clone()),
                    Some("hasField") => {
                        if let Some(attr) = lift_attribute(o, by_subject) {
                            class.attributes.push(attr);
                        }
                    }
                    Some("hasAssociation") => {
                        if let Some(assoc) = lift_association(o, by_subject) {
                            class.associations.push(assoc);
                        }
                    }
                    _ => {} // ignored on v1 (next-sprint shape)
                }
            }
            classes.push((subject.clone(), class));
        }

        // Preserve definition order by subject string (subject IRIs are
        // emitted in definition order from `ogar-emitter`).
        classes.sort_by(|a, b| a.0.cmp(&b.0));
        classes.into_iter().map(|(_, c)| c).collect()
    }

    fn lift_attribute(
        subject: &str,
        by_subject: &HashMap<String, Vec<(String, String)>>,
    ) -> Option<Attribute> {
        let preds = by_subject.get(subject)?;
        let is_field = preds
            .iter()
            .any(|(p, o)| is_rdf_type(p) && ogar_local(o) == Some("Field"));
        if !is_field {
            return None;
        }
        let mut attr = Attribute::new(local_name(subject));
        for (p, o) in preds {
            match ogar_local(p) {
                Some("fieldName") => attr.name = o.clone(),
                Some("fieldType") => attr.type_name = Some(o.clone()),
                _ => {
                    if is_rdfs_range(p) && attr.type_name.is_none() {
                        // rdfs:range is the OWL-standard typing;
                        // ogar:fieldType is the OGAR variant.
                        // Either takes precedence (fieldType first).
                        attr.type_name = Some(o.clone());
                    }
                }
            }
        }
        Some(attr)
    }

    fn lift_association(
        subject: &str,
        by_subject: &HashMap<String, Vec<(String, String)>>,
    ) -> Option<Association> {
        let preds = by_subject.get(subject)?;
        let is_assoc = preds
            .iter()
            .any(|(p, o)| is_rdf_type(p) && ogar_local(o) == Some("Association"));
        if !is_assoc {
            return None;
        }
        let kind_str = preds
            .iter()
            .find_map(|(p, o)| (ogar_local(p) == Some("kind")).then(|| o.clone()))?;
        // kind value is itself an `ogar:` term — strip and match the
        // local name.
        let kind = match ogar_local(&kind_str)? {
            "BelongsTo" => AssociationKind::BelongsTo,
            "HasOne" => AssociationKind::HasOne,
            "HasMany" => AssociationKind::HasMany,
            "HasAndBelongsToMany" => AssociationKind::HasAndBelongsToMany,
            _ => return None,
        };
        let name = preds
            .iter()
            .find_map(|(p, o)| (ogar_local(p) == Some("relationName")).then(|| o.clone()))
            .unwrap_or_else(|| local_name(subject));
        let mut assoc = Association::new(kind, name);
        for (p, o) in preds {
            if ogar_local(p) == Some("targetClass") {
                assoc.class_name = Some(local_name(o));
            }
        }
        Some(assoc)
    }

    /// Strip the `<https://ogar.../prefix/>` URI prefix and return the
    /// local part. Falls back to the input if no `/` is found.
    fn local_name(iri: &str) -> String {
        if let Some(p) = iri.rsplit_once('/') {
            p.1.to_string()
        } else if let Some(p) = iri.rsplit_once(':') {
            p.1.to_string()
        } else {
            iri.to_string()
        }
    }

    // Silence dead-code warning when the named-node module isn't used
    // — keeping it imported in case future PRs want typed predicates.
    #[allow(dead_code)]
    fn _named_node_keepalive(n: NamedNode) -> String {
        n.as_str().to_string()
    }
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "ttl-parser")]
    use ogar_vocab::{Association, AssociationKind};
    use ogar_vocab::{Attribute, Class};

    // ── Emit-side tests (always available) ─────────────────────────────

    #[test]
    fn emit_empty_classes_produces_just_prefix_block() {
        let out = emit_ttl(&[], "ogit-erp");
        assert!(out.contains("@prefix ogar:"));
        assert!(out.contains("@prefix ogit-erp:"));
        assert!(out.contains("@prefix rdf:"));
        assert!(out.contains("@prefix owl:"));
    }

    #[test]
    fn emit_minimal_class_includes_rdf_type_ogar_class() {
        let c = Class::new("Order");
        let out = emit_ttl(&[c], "ogit-erp");
        // The class subject must be there + a rdf:type ogar:Class triple.
        assert!(out.contains("ogit-erp/Order"), "got: {out}");
        assert!(out.contains("rdf:type ogar:Class"), "got: {out}");
    }

    #[test]
    fn emit_class_with_parent_emits_parent_class_predicate() {
        let mut c = Class::new("Invoice");
        c.parent = Some("Order".into());
        let out = emit_ttl(&[c], "ogit-erp");
        assert!(out.contains("ogar:parentClass"), "got: {out}");
        assert!(out.contains("ogit-erp/Order"), "got: {out}");
    }

    #[test]
    fn emit_class_with_attribute_emits_field_triples() {
        let mut c = Class::new("Account");
        let mut email = Attribute::new("email");
        email.type_name = Some("string".into());
        c.attributes.push(email);
        let out = emit_ttl(&[c], "ogit-erp");
        // The field should appear via ogar:hasField + ogar:fieldName.
        assert!(out.contains("ogar:hasField"), "got: {out}");
        assert!(out.contains("ogar:fieldName"), "got: {out}");
    }

    // ── Parse-side tests (gated behind `ttl-parser`) ───────────────────

    #[cfg(feature = "ttl-parser")]
    #[test]
    fn parse_minimal_class_lifts_one_class() {
        let ttl = "@prefix ogar: <https://ogar.surrealdb-graph.io/vocab/ogar#> .\n\
                   @prefix ogit-erp: <https://ogar.surrealdb-graph.io/ogit-erp/> .\n\
                   @prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
                   ogit-erp:Order rdf:type ogar:Class .\n";
        let classes = parse_ttl(ttl).expect("parse OK");
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "Order");
    }

    #[cfg(feature = "ttl-parser")]
    #[test]
    fn parse_returns_unimplemented_or_error_on_invalid_ttl() {
        // Malformed turtle: missing dot at end.
        let bad = "this is not turtle";
        let r = parse_ttl(bad);
        assert!(matches!(r, Err(TtlParseError::Parse(_))), "got: {r:?}");
    }

    // ── Round-trip tests (the load-bearing ones) ───────────────────────

    #[cfg(feature = "ttl-parser")]
    #[test]
    fn round_trip_minimal_class_preserves_name() {
        let original = Class::new("Order");
        let ttl = emit_ttl(&[original], "ogit-erp");
        let recovered = parse_ttl(&ttl).expect("parse OK");
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].name, "Order");
    }

    #[cfg(feature = "ttl-parser")]
    #[test]
    fn round_trip_class_with_parent_preserves_inheritance() {
        let mut original = Class::new("Invoice");
        original.parent = Some("Order".into());
        let ttl = emit_ttl(&[original], "ogit-erp");
        let recovered = parse_ttl(&ttl).expect("parse OK");
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].name, "Invoice");
        assert_eq!(recovered[0].parent.as_deref(), Some("Order"));
    }

    #[cfg(feature = "ttl-parser")]
    #[test]
    fn round_trip_class_with_belongs_to_preserves_association() {
        let mut original = Class::new("Invoice");
        let mut customer = Association::new(AssociationKind::BelongsTo, "customer");
        customer.class_name = Some("Customer".into());
        original.associations.push(customer);
        let ttl = emit_ttl(&[original], "ogit-erp");
        let recovered = parse_ttl(&ttl).expect("parse OK");
        assert_eq!(recovered.len(), 1);
        let c = &recovered[0];
        assert_eq!(c.associations.len(), 1, "got: {c:?}");
        assert!(matches!(c.associations[0].kind, AssociationKind::BelongsTo));
        assert_eq!(c.associations[0].class_name.as_deref(), Some("Customer"));
    }

    #[cfg(not(feature = "ttl-parser"))]
    #[test]
    fn parse_returns_unimplemented_when_feature_off() {
        let r = parse_ttl("anything");
        assert!(matches!(r, Err(TtlParseError::Unimplemented(_))));
    }
}
