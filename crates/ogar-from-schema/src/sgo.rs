//! SGO (System Graph Ontology) upper-vocabulary front-end.
//!
//! SGO is OGIT's upper ontology — the layer above NTO that declares
//! the canonical entities (`ogit:Entity`, `ogit:Node`) and **the
//! canonical verb vocabulary** (`ogit:dependsOn`, `ogit:contains`,
//! `ogit:generates`, `ogit:relates`, `ogit:causes`, …). 176 verb TTLs
//! at `vocab/imports/ogit/SGO/sgo/verbs/`.
//!
//! **Why this matters for OGAR's AST.** Every NTO entity's
//! `ogit:allowed ([ verb target ])` block references SGO verbs by
//! name. Today (`crate::ttl::parse_entity`), those references are
//! captured as raw strings. With SGO lifted, the references can be
//! resolved against a typed [`VerbDecl`] registry — turning the
//! `Association` predicate vocabulary from string-compared into
//! statically-validated. This is the AST predicate vocabulary the
//! `ActionDef` layer reads from.
//!
//! # v0 scope
//!
//! - Parse one SGO verb TTL into a [`VerbDecl`].
//! - The registry index (`build_registry(&[path]) -> HashMap<curie, VerbDecl>`)
//!   is queued — landed once the consumer call sites need it.
//!
//! # Out of v0
//!
//! - SGO entity TTLs (`SGO/core/entities/Node.ttl`,
//!   `SGO/sgo/entities/Entity.ttl`) — these are upper-ontology and
//!   land in a sibling `sgo_entity` module when wired.
//! - `rdfs:domain` / `rdfs:range` constraints on verbs (when OGIT
//!   declares them). Today's SGO verb TTLs are description-only;
//!   domain/range live elsewhere in OGIT.

use std::fmt::Write;

/// Lifted shape of a single SGO verb TTL file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VerbDecl {
    /// The CURIE-form name (`ogit:dependsOn`).
    pub curie: String,
    /// The local name (`dependsOn`).
    pub name: String,
    /// `rdfs:label` text.
    pub label: String,
    /// `dcterms:description` text — multi-line preserved verbatim.
    pub description: String,
    /// `dcterms:valid` validity-range string.
    pub dcterms_valid: Option<String>,
    /// `dcterms:creator` text.
    pub dcterms_creator: Option<String>,
}

/// Parse one SGO verb TTL file's bytes. Returns `None` when the file
/// doesn't declare an `owl:ObjectProperty` subject.
#[must_use]
pub fn parse_verb(src: &str) -> Option<VerbDecl> {
    use crate::ttl::*;
    // Reuse the line-extractors from the ttl module via a narrow API.
    if !src.contains("a owl:ObjectProperty") {
        return None;
    }
    let curie = find_subject_curie(src)?;
    let name = curie.rsplit(':').next().unwrap_or(&curie).trim().to_owned();
    let label = extract_quoted_value(src, "rdfs:label").unwrap_or_default();
    let description =
        extract_triple_quoted_or_quoted(src, "dcterms:description").unwrap_or_default();
    let dcterms_valid = extract_quoted_value(src, "dcterms:valid");
    let dcterms_creator = extract_quoted_value(src, "dcterms:creator");
    Some(VerbDecl {
        curie,
        name,
        label,
        description,
        dcterms_valid,
        dcterms_creator,
    })
}

/// Emit a TTL document for the verb declaration — the reverse direction
/// for round-trip checking, same semantic-bijection contract as
/// [`crate::ttl_emit::emit_entity`].
#[must_use]
pub fn emit_verb(verb: &VerbDecl) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "@prefix ogit: <http://www.purl.org/ogit/> .");
    let _ = writeln!(
        out,
        "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> ."
    );
    let _ = writeln!(out, "@prefix owl: <http://www.w3.org/2002/07/owl#> .");
    let _ = writeln!(out, "@prefix dcterms: <http://purl.org/dc/terms/> .");
    let _ = writeln!(out);
    out.push_str(&verb.curie);
    let _ = writeln!(out);
    let _ = writeln!(out, "\ta owl:ObjectProperty;");
    let _ = writeln!(out, "\trdfs:subPropertyOf ogit:Verb;");
    let _ = writeln!(out, "\trdfs:label \"{}\";", verb.label);
    if !verb.description.is_empty() {
        let body = if verb.description.contains('\n') || verb.description.contains('"') {
            format!("\"\"\"{}\"\"\"", verb.description)
        } else {
            format!("\"{}\"", verb.description)
        };
        let _ = writeln!(out, "\tdcterms:description {body};");
    }
    if let Some(v) = &verb.dcterms_valid {
        let _ = writeln!(out, "\tdcterms:valid \"{v}\";");
    }
    if let Some(c) = &verb.dcterms_creator {
        let _ = writeln!(out, "\tdcterms:creator \"{c}\";");
    }
    out.push_str(".\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEPENDSON_TTL: &str =
        include_str!("../../../vocab/imports/ogit/SGO/sgo/verbs/dependsOn.ttl");
    const CONTAINS_TTL: &str =
        include_str!("../../../vocab/imports/ogit/SGO/sgo/verbs/contains.ttl");

    #[test]
    fn parses_dependson() {
        let v = parse_verb(DEPENDSON_TTL).expect("parses");
        assert_eq!(v.curie, "ogit:dependsOn");
        assert_eq!(v.name, "dependsOn");
        assert_eq!(v.label, "dependsOn");
        assert!(v.description.contains("depending"));
        assert!(v.dcterms_valid.is_some());
        assert!(v.dcterms_creator.is_some());
    }

    #[test]
    fn parses_contains() {
        let v = parse_verb(CONTAINS_TTL).expect("parses");
        assert_eq!(v.name, "contains");
        // Description uses the triple-quoted form because it contains
        // an embedded literal quote (`see also "includes"`).
        assert!(v.description.contains("part of something"));
    }

    #[test]
    fn dependson_roundtrip() {
        let once = parse_verb(DEPENDSON_TTL).expect("parses");
        let emitted = emit_verb(&once);
        let twice = parse_verb(&emitted).expect("re-parses");
        assert_eq!(once, twice, "verb round-trip lost a predicate");
    }

    /// Walk every verb TTL under `SGO/sgo/verbs/` and assert round-trip.
    /// One of OGIT's strengths is that the verb vocabulary is small and
    /// stable; this test catches drift on any of the 176 verbs the
    /// moment a predicate slips out of `VerbDecl`.
    #[test]
    fn all_sgo_verbs_roundtrip() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vocab/imports/ogit/SGO/sgo/verbs");
        let mut checked = 0usize;
        for entry in std::fs::read_dir(&dir).expect("read sgo/verbs/") {
            let entry = entry.expect("entry");
            if entry.path().extension().and_then(|e| e.to_str()) != Some("ttl") {
                continue;
            }
            let src = std::fs::read_to_string(entry.path()).expect("read");
            let Some(once) = parse_verb(&src) else {
                continue;
            };
            let emitted = emit_verb(&once);
            let twice = parse_verb(&emitted).expect("re-parses");
            assert_eq!(
                once, twice,
                "verb {} round-trip lost a predicate",
                once.name
            );
            checked += 1;
        }
        assert!(
            checked >= 170,
            "expected ≥ 170 SGO verbs at this SHA, got {checked}"
        );
    }
}
