//! Reverse direction — emit OGIT TTL from a lifted [`EntityDecl`] /
//! [`AttributeDecl`].
//!
//! **Semantic bijection**, not byte-bijection: `parse(emit(parse(src)))`
//! equals `parse(src)`. Whitespace, prefix-declaration order, and
//! comment positions are not preserved — they are not load-bearing for
//! the structural arm. (Byte-bijection would force the producer to carry
//! raw text alongside the parsed structure, defeating the "schema as IR"
//! pattern this crate exists to support.)
//!
//! The emitter only writes predicates that have content. Optional
//! fields (`dcterms:valid`, `ogit:scope`, `ogit:indexed-attributes`)
//! are skipped when empty/`None`, matching upstream OGIT TTL practice.
//!
//! [`EntityDecl`]: super::EntityDecl
//! [`AttributeDecl`]: super::AttributeDecl

use std::collections::BTreeSet;
use std::fmt::Write;

use super::{AttributeDecl, EntityDecl};

/// Emit a full TTL document for an entity declaration (one
/// `@prefix` block + the subject + its predicates).
#[must_use]
pub fn emit_entity(entity: &EntityDecl) -> String {
    let prefixes = collect_prefixes_for_entity(entity);
    let mut out = emit_prefix_block(&prefixes);
    let _ = writeln!(out);
    out.push_str(&entity.curie);
    let _ = writeln!(out);
    let _ = writeln!(out, "\ta rdfs:Class;");
    if let Some(p) = &entity.parent {
        let _ = writeln!(out, "\trdfs:subClassOf {p};");
    }
    let _ = writeln!(out, "\trdfs:label \"{}\";", entity.label);
    if !entity.description.is_empty() {
        let _ = writeln!(
            out,
            "\tdcterms:description {};",
            quote_description(&entity.description)
        );
    }
    if let Some(v) = &entity.dcterms_valid {
        let _ = writeln!(out, "\tdcterms:valid \"{v}\";");
    }
    if let Some(c) = &entity.dcterms_creator {
        let _ = writeln!(out, "\tdcterms:creator \"{c}\";");
    }
    if let Some(s) = &entity.ogit_scope {
        let _ = writeln!(out, "\togit:scope \"{s}\";");
    }
    if let Some(p) = &entity.ogit_parent {
        let _ = writeln!(out, "\togit:parent {p};");
    }
    emit_list(
        &mut out,
        "mandatory-attributes",
        &entity.mandatory_attributes,
    );
    emit_list(&mut out, "optional-attributes", &entity.optional_attributes);
    emit_list(&mut out, "indexed-attributes", &entity.indexed_attributes);
    emit_allowed(&mut out, &entity.allowed);
    out.push_str(".\n");
    out
}

/// Emit a full TTL document for an attribute declaration.
#[must_use]
pub fn emit_attribute(attr: &AttributeDecl) -> String {
    let prefixes = collect_prefixes_for_attribute(attr);
    let mut out = emit_prefix_block(&prefixes);
    let _ = writeln!(out);
    out.push_str(&attr.curie);
    let _ = writeln!(out);
    let _ = writeln!(out, "\ta owl:DatatypeProperty;");
    let _ = writeln!(out, "\trdfs:subPropertyOf ogit:Attribute;");
    let _ = writeln!(out, "\trdfs:label \"{}\";", attr.label);
    if !attr.description.is_empty() {
        let _ = writeln!(
            out,
            "\tdcterms:description {};",
            quote_description(&attr.description)
        );
    }
    if let Some(v) = &attr.dcterms_valid {
        let _ = writeln!(out, "\tdcterms:valid \"{v}\";");
    }
    if let Some(c) = &attr.dcterms_creator {
        let _ = writeln!(out, "\tdcterms:creator \"{c}\";");
    }
    if let Some(t) = &attr.validation_type {
        let _ = writeln!(out, "\togit:validation-type \"{t}\";");
    }
    if let Some(p) = &attr.validation_parameter {
        let _ = writeln!(out, "\togit:validation-parameter \"{p}\";");
    }
    out.push_str(".\n");
    out
}

// ───────────────────────────────────────────────────────────── helpers ──

/// `"""…"""` for multi-line text, `"…"` for single-line — same choice
/// the upstream OGIT TTL files make.
fn quote_description(text: &str) -> String {
    if text.contains('\n') || text.contains('"') {
        format!("\"\"\"{text}\"\"\"")
    } else {
        format!("\"{text}\"")
    }
}

fn emit_list(out: &mut String, predicate: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    let _ = writeln!(out, "\togit:{predicate} (");
    for item in items {
        let _ = writeln!(out, "\t\t{item}");
    }
    let _ = writeln!(out, "\t);");
}

fn emit_allowed(out: &mut String, allowed: &[(String, String)]) {
    if allowed.is_empty() {
        return;
    }
    let _ = writeln!(out, "\togit:allowed (");
    for (verb, target) in allowed {
        let _ = writeln!(out, "\t\t[ {verb}  {target} ]");
    }
    let _ = writeln!(out, "\t);");
}

/// Mandatory base prefixes the OGIT TTLs always declare.
fn base_prefixes() -> Vec<(&'static str, &'static str)> {
    vec![
        ("ogit", "http://www.purl.org/ogit/"),
        ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
        ("owl", "http://www.w3.org/2002/07/owl#"),
        ("dcterms", "http://purl.org/dc/terms/"),
    ]
}

fn collect_prefixes_for_entity(entity: &EntityDecl) -> Vec<(String, String)> {
    let mut tokens = BTreeSet::new();
    tokens.insert(entity.curie.clone());
    if let Some(p) = &entity.parent {
        tokens.insert(p.clone());
    }
    if let Some(p) = &entity.ogit_parent {
        tokens.insert(p.clone());
    }
    for a in entity
        .mandatory_attributes
        .iter()
        .chain(entity.optional_attributes.iter())
        .chain(entity.indexed_attributes.iter())
    {
        tokens.insert(a.clone());
    }
    for (v, t) in &entity.allowed {
        tokens.insert(v.clone());
        tokens.insert(t.clone());
    }
    derive_prefix_decls(&tokens)
}

fn collect_prefixes_for_attribute(attr: &AttributeDecl) -> Vec<(String, String)> {
    let mut tokens = BTreeSet::new();
    tokens.insert(attr.curie.clone());
    derive_prefix_decls(&tokens)
}

fn derive_prefix_decls(tokens: &BTreeSet<String>) -> Vec<(String, String)> {
    let mut have: BTreeSet<String> = base_prefixes()
        .iter()
        .map(|(p, _)| (*p).to_owned())
        .collect();
    let mut emitted: Vec<(String, String)> = base_prefixes()
        .into_iter()
        .map(|(p, u)| (p.to_owned(), u.to_owned()))
        .collect();
    for tok in tokens {
        let Some(colon) = tok.find(':') else { continue };
        let prefix = &tok[..colon];
        if prefix.is_empty() || have.contains(prefix) {
            continue;
        }
        // Derive the IRI from the prefix by dotted-path conversion.
        let path = prefix.replace('.', "/");
        let iri = format!("http://www.purl.org/{path}/");
        emitted.push((prefix.to_owned(), iri));
        have.insert(prefix.to_owned());
    }
    emitted
}

fn emit_prefix_block(prefixes: &[(String, String)]) -> String {
    let mut out = String::new();
    for (p, u) in prefixes {
        let _ = writeln!(out, "@prefix {p}: <{u}> .");
    }
    out
}

// ───────────────────────────────────────────────────────────── tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TtlDeclaration;
    use crate::ttl::parse_file;

    const MACHINE_TTL: &str =
        include_str!("../../../vocab/imports/ogit/NTO/MARS/entities/Machine.ttl");
    const APPLICATION_TTL: &str =
        include_str!("../../../vocab/imports/ogit/NTO/MARS/entities/Application.ttl");
    const APP_CLASS_TTL: &str =
        include_str!("../../../vocab/imports/ogit/NTO/MARS/Application/attributes/class.ttl");
    const APP_SUBCLASS_TTL: &str =
        include_str!("../../../vocab/imports/ogit/NTO/MARS/Application/attributes/subClass.ttl");

    /// The semantic-bijection contract: `parse(emit(parse(src)))` is
    /// equal to `parse(src)`. Whitespace, comment positions, and prefix
    /// ordering are not preserved (and should not be — they are not
    /// load-bearing) but every declared predicate must survive the
    /// round-trip.
    fn assert_entity_roundtrip(src: &str) {
        let TtlDeclaration::Entity(once) = parse_file(src).expect("parse src") else {
            panic!("not an entity TTL");
        };
        let emitted = emit_entity(&once);
        let TtlDeclaration::Entity(twice) = parse_file(&emitted).expect("parse emitted") else {
            panic!("not an entity TTL after emit");
        };
        assert_eq!(once, twice, "round-trip lost or added a predicate");
    }

    fn assert_attribute_roundtrip(src: &str) {
        let TtlDeclaration::DatatypeAttribute(once) = parse_file(src).expect("parse src") else {
            panic!("not a datatype attribute TTL");
        };
        let emitted = emit_attribute(&once);
        let TtlDeclaration::DatatypeAttribute(twice) = parse_file(&emitted).expect("parse emitted")
        else {
            panic!("not a datatype attribute TTL after emit");
        };
        assert_eq!(once, twice, "round-trip lost or added a predicate");
    }

    #[test]
    fn machine_entity_roundtrip() {
        assert_entity_roundtrip(MACHINE_TTL);
    }

    #[test]
    fn application_entity_roundtrip() {
        assert_entity_roundtrip(APPLICATION_TTL);
    }

    #[test]
    fn application_class_attribute_roundtrip() {
        assert_attribute_roundtrip(APP_CLASS_TTL);
    }

    #[test]
    fn application_subclass_attribute_roundtrip() {
        assert_attribute_roundtrip(APP_SUBCLASS_TTL);
    }

    /// Stress: round-trip every MARS TTL file in `vocab/imports/`.
    /// If a future PR drops a predicate from `EntityDecl` /
    /// `AttributeDecl`, this fails on the first file that uses that
    /// predicate.
    #[test]
    fn all_mars_ttl_files_roundtrip() {
        let stats = assert_domain_roundtrip("MARS");
        // 29 .ttl files in NTO/MARS at the SHA pinned by PROVENANCE.md.
        assert!(
            stats.total >= 29,
            "expected ≥ 29 TTL files in MARS, got {}",
            stats.total
        );
    }

    /// Generic helper that walks `vocab/imports/ogit/NTO/<domain>/`,
    /// dispatches each TTL to the right parser (`parse_file` for entities
    /// and datatype attributes, `crate::sgo::parse_verb` for in-domain
    /// `owl:ObjectProperty` verbs), and asserts semantic round-trip.
    /// Returns per-shape counts so callers can sanity-check the lift
    /// surface they're claiming.
    fn assert_domain_roundtrip(domain: &str) -> DomainStats {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vocab/imports/ogit/NTO")
            .join(domain);
        let mut stats = DomainStats::default();
        for entry in walk_ttl(&dir) {
            let src = std::fs::read_to_string(&entry).expect("read");
            stats.total += 1;
            match parse_file(&src) {
                Some(TtlDeclaration::Entity(_)) => {
                    assert_entity_roundtrip(&src);
                    stats.entities += 1;
                }
                Some(TtlDeclaration::DatatypeAttribute(_)) => {
                    assert_attribute_roundtrip(&src);
                    stats.attributes += 1;
                }
                None => {
                    // Try the verb path — some NTO domains carry their own
                    // in-domain `owl:ObjectProperty` verbs (Transport,
                    // Accounting, Credit, Compliance) alongside SGO's
                    // upstream-shared vocabulary.
                    let Some(once) = crate::sgo::parse_verb(&src) else {
                        panic!(
                            "TTL has no recognised subject type in {domain}: {}",
                            entry.display()
                        );
                    };
                    let emitted = crate::sgo::emit_verb(&once);
                    let twice = crate::sgo::parse_verb(&emitted).expect("re-parse verb");
                    assert_eq!(
                        once,
                        twice,
                        "verb round-trip lost a predicate in {domain}: {}",
                        entry.display()
                    );
                    stats.verbs += 1;
                }
            }
        }
        stats
    }

    #[derive(Debug, Default)]
    struct DomainStats {
        total: usize,
        entities: usize,
        attributes: usize,
        verbs: usize,
    }

    /// Cross-domain bijection coverage. Each row is one of the nine
    /// domains the operator asked OGAR to verify before promoting
    /// the lift surface from MARS-only to multi-domain. If any of
    /// these fails, the producer can't land on that domain without
    /// extending `EntityDecl` / `AttributeDecl` / `VerbDecl` first.
    ///
    /// Counts are also a sanity check on the inventory — they prove
    /// the catalogue's per-domain numbers match what's actually in
    /// `vocab/imports/`.
    #[test]
    fn nine_domains_lift_surface_round_trip() {
        for (domain, expected_total) in [
            ("Transport", 27),
            ("Accounting", 36),
            ("SalesDistribution", 23),
            ("Credit", 21),
            ("Cost", 5),
            ("ServiceManagement", 59),
            ("WorkOrder", 27),
            ("Compliance", 9),
            ("Audit", 3),
        ] {
            let stats = assert_domain_roundtrip(domain);
            assert_eq!(
                stats.total, expected_total,
                "{domain}: TTL count drifted from inventory \
                 (expected {expected_total}, got {})",
                stats.total
            );
        }
    }

    fn walk_ttl(root: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(p) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&p) else {
                continue;
            };
            for entry in rd.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().and_then(|e| e.to_str()) == Some("ttl") {
                    out.push(path);
                }
            }
        }
        out
    }
}
