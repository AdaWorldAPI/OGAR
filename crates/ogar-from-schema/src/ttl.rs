//! OGIT TTL front-end.
//!
//! Reads the line-oriented subset of Turtle the OGIT NTO files actually
//! use. Not a full Turtle parser — the OGIT dialect is machine-stable
//! and narrow: prefix declarations, one `rdfs:Class` or
//! `owl:DatatypeProperty` subject per file, predicate-object lines
//! ending in `;`, the file terminator `.`.
//!
//! When the surface grows beyond this dialect (Wikidata-shaped TTL,
//! multi-subject files, OWL imports), swap the walker for `oxttl` /
//! `oxrdf` without touching the [`EntityDecl`] / [`AttributeDecl`]
//! target shape.
//!
//! [`EntityDecl`]: super::EntityDecl
//! [`AttributeDecl`]: super::AttributeDecl

use super::{AttributeDecl, EntityDecl, TtlDeclaration};

/// Parse one OGIT TTL file's bytes into the lifted declaration it
/// describes. Returns `None` when the file doesn't declare a recognised
/// subject (e.g. a comment-only file).
///
/// # Errors / `None` cases
///
/// - File contains no subject line that starts at column zero
/// - Subject is neither `a rdfs:Class;` nor `a owl:DatatypeProperty;`
#[must_use]
pub fn parse_file(src: &str) -> Option<TtlDeclaration> {
    let kind = detect_kind(src)?;
    match kind {
        SubjectKind::Class => parse_entity(src).map(TtlDeclaration::Entity),
        SubjectKind::DatatypeProperty => {
            parse_attribute(src).map(TtlDeclaration::DatatypeAttribute)
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SubjectKind {
    Class,
    DatatypeProperty,
}

fn detect_kind(src: &str) -> Option<SubjectKind> {
    for line in src.lines() {
        let trimmed = line.trim();
        if trimmed.contains("a rdfs:Class") {
            return Some(SubjectKind::Class);
        }
        if trimmed.contains("a owl:DatatypeProperty") {
            return Some(SubjectKind::DatatypeProperty);
        }
    }
    None
}

pub(crate) fn find_subject_curie(src: &str) -> Option<String> {
    // The subject CURIE is the first non-prefix, non-blank, non-comment
    // line whose content does not start with `@` or whitespace. OGIT
    // declares it at column zero followed by an indented `a rdfs:Class;`
    // on the next line.
    for line in src.lines() {
        if line.starts_with('@')
            || line.is_empty()
            || line.starts_with('#')
            || line.starts_with(char::is_whitespace)
        {
            continue;
        }
        let candidate = line.trim_end_matches(';').trim();
        if candidate.is_empty() || candidate.starts_with("a ") {
            continue;
        }
        return Some(candidate.to_owned());
    }
    None
}

pub(crate) fn local_name_from_curie(curie: &str) -> String {
    // `ogit.MARS:Machine` → `Machine`
    // `ogit.MARS.Application:class` → `class`
    curie.rsplit(':').next().unwrap_or(curie).trim().to_owned()
}

fn parse_entity(src: &str) -> Option<EntityDecl> {
    let curie = find_subject_curie(src)?;
    let name = local_name_from_curie(&curie);
    let label = extract_quoted_value(src, "rdfs:label").unwrap_or_default();
    let description =
        extract_triple_quoted_or_quoted(src, "dcterms:description").unwrap_or_default();
    let parent = extract_token_value(src, "rdfs:subClassOf");
    let dcterms_valid = extract_quoted_value(src, "dcterms:valid");
    let dcterms_creator = extract_quoted_value(src, "dcterms:creator");
    let ogit_scope = extract_quoted_value(src, "ogit:scope");
    let ogit_parent = extract_token_value(src, "ogit:parent");
    let mandatory_attributes = extract_paren_list(src, "ogit:mandatory-attributes");
    let optional_attributes = extract_paren_list(src, "ogit:optional-attributes");
    let indexed_attributes = extract_paren_list(src, "ogit:indexed-attributes");
    let allowed = extract_allowed_block(src);
    Some(EntityDecl {
        curie,
        name,
        label,
        description,
        parent,
        dcterms_valid,
        dcterms_creator,
        ogit_scope,
        ogit_parent,
        mandatory_attributes,
        optional_attributes,
        indexed_attributes,
        allowed,
    })
}

fn parse_attribute(src: &str) -> Option<AttributeDecl> {
    let curie = find_subject_curie(src)?;
    let name = local_name_from_curie(&curie);
    let label = extract_quoted_value(src, "rdfs:label").unwrap_or_default();
    let description =
        extract_triple_quoted_or_quoted(src, "dcterms:description").unwrap_or_default();
    let dcterms_valid = extract_quoted_value(src, "dcterms:valid");
    let dcterms_creator = extract_quoted_value(src, "dcterms:creator");
    let validation_type = extract_quoted_value(src, "ogit:validation-type");
    let validation_parameter = extract_quoted_value(src, "ogit:validation-parameter");
    Some(AttributeDecl {
        curie,
        name,
        label,
        description,
        dcterms_valid,
        dcterms_creator,
        validation_type,
        validation_parameter,
    })
}

/// Extract a single-line predicate value of the form
/// `<indent>predicate "value";`. Returns the inner string.
pub(crate) fn extract_quoted_value(src: &str, predicate: &str) -> Option<String> {
    for line in src.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(predicate) {
            let rest = rest.trim_start();
            // Skip non-quoted values (e.g. predicate has a list/CURIE
            // form that another extractor handles).
            if let Some(stripped) = rest.strip_prefix('"')
                && let Some(end) = stripped.find('"')
            {
                return Some(stripped[..end].to_owned());
            }
        }
    }
    None
}

/// `dcterms:description` is sometimes triple-quoted (Python-style
/// `"""..."""`) to allow embedded newlines. Try triple-quoted first, fall
/// back to single-line.
pub(crate) fn extract_triple_quoted_or_quoted(src: &str, predicate: &str) -> Option<String> {
    if let Some(triple) = extract_triple_quoted_value(src, predicate) {
        return Some(triple);
    }
    extract_quoted_value(src, predicate)
}

fn extract_triple_quoted_value(src: &str, predicate: &str) -> Option<String> {
    let idx = src.find(predicate)?;
    let after = &src[idx + predicate.len()..];
    let after = after.trim_start();
    let body = after.strip_prefix("\"\"\"")?;
    let end = body.find("\"\"\"")?;
    Some(body[..end].to_owned())
}

/// Extract a CURIE-shaped predicate value, e.g.
/// `<indent>rdfs:subClassOf ogit:Entity;`.
fn extract_token_value(src: &str, predicate: &str) -> Option<String> {
    for line in src.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(predicate) {
            let rest = rest.trim_start().trim_end_matches(';').trim();
            if !rest.is_empty() && !rest.starts_with('"') && !rest.starts_with('(') {
                return Some(rest.to_owned());
            }
        }
    }
    None
}

/// Extract the entries inside an `ogit:mandatory-attributes (...)` block.
fn extract_paren_list(src: &str, predicate: &str) -> Vec<String> {
    let Some(idx) = src.find(predicate) else {
        return Vec::new();
    };
    let after = &src[idx + predicate.len()..];
    let Some(open) = after.find('(') else {
        return Vec::new();
    };
    let body = &after[open + 1..];
    let Some(close) = body.find(')') else {
        return Vec::new();
    };
    body[..close]
        .split_whitespace()
        .filter(|tok| !tok.is_empty() && *tok != ";")
        .map(str::to_owned)
        .collect()
}

/// Extract the `ogit:allowed (...)` block — each entry is
/// `[ <verb> <target> ]` on its own line.
fn extract_allowed_block(src: &str) -> Vec<(String, String)> {
    let Some(idx) = src.find("ogit:allowed") else {
        return Vec::new();
    };
    let after = &src[idx + "ogit:allowed".len()..];
    let Some(open) = after.find('(') else {
        return Vec::new();
    };
    let body = &after[open + 1..];
    let Some(close) = body.find(')') else {
        return Vec::new();
    };
    let inner = &body[..close];
    let mut out = Vec::new();
    for line in inner.lines() {
        let trimmed = line
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut toks = trimmed.split_whitespace();
        if let (Some(verb), Some(target)) = (toks.next(), toks.next()) {
            out.push((verb.to_owned(), target.to_owned()));
        }
    }
    out
}

// ───────────────────────────────────────────────────────────── tests ──

#[cfg(test)]
mod tests {
    use super::*;

    const MACHINE_TTL: &str =
        include_str!("../../../vocab/imports/ogit/NTO/MARS/entities/Machine.ttl");
    const APPLICATION_TTL: &str =
        include_str!("../../../vocab/imports/ogit/NTO/MARS/entities/Application.ttl");
    const APP_CLASS_TTL: &str =
        include_str!("../../../vocab/imports/ogit/NTO/MARS/Application/attributes/class.ttl");
    const APP_SUBCLASS_TTL: &str =
        include_str!("../../../vocab/imports/ogit/NTO/MARS/Application/attributes/subClass.ttl");

    #[test]
    fn parses_machine_entity() {
        let TtlDeclaration::Entity(e) = parse_file(MACHINE_TTL).expect("parses") else {
            panic!("expected entity");
        };
        assert_eq!(e.curie, "ogit.MARS:Machine");
        assert_eq!(e.name, "Machine");
        assert_eq!(e.label, "Machine");
        assert_eq!(e.parent.as_deref(), Some("ogit:Entity"));
        assert!(e.mandatory_attributes.contains(&"ogit:name".to_owned()));
        assert!(
            e.mandatory_attributes
                .contains(&"ogit.MARS.Machine:class".to_owned())
        );
        assert!(
            e.optional_attributes
                .contains(&"ogit.MARS.Network:fqdn".to_owned())
        );
        // The `ogit:allowed` block carries verb → target pairs.
        assert!(
            e.allowed
                .iter()
                .any(|(v, t)| v == "ogit:locatedAt" && t == "ogit:Location")
        );
    }

    #[test]
    fn parses_application_entity_with_dependson_chain() {
        let TtlDeclaration::Entity(e) = parse_file(APPLICATION_TTL).expect("parses") else {
            panic!("expected entity");
        };
        assert_eq!(e.name, "Application");
        // The A→R→S→M backbone: Application dependsOn Resource.
        assert!(
            e.allowed
                .iter()
                .any(|(v, t)| v == "ogit:dependsOn" && t == "ogit.MARS:Resource"),
            "expected `dependsOn Resource` allowed-edge, got {:?}",
            e.allowed
        );
    }

    #[test]
    fn parses_fixed_enum_attribute() {
        let TtlDeclaration::DatatypeAttribute(a) = parse_file(APP_CLASS_TTL).expect("parses")
        else {
            panic!("expected datatype attribute");
        };
        assert_eq!(a.name, "class");
        assert_eq!(a.validation_type.as_deref(), Some("fixed"));
        let values = a.fixed_enum_values().expect("fixed enum");
        // From `Application/attributes/class.ttl`:
        //   "Data,Development,Education,Enterprise,Financial,Media,Others"
        assert_eq!(
            values,
            vec![
                "Data",
                "Development",
                "Education",
                "Enterprise",
                "Financial",
                "Media",
                "Others",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
        );
        // Matches the XSD oracle's ApplicationClass count (7) per
        // PROVENANCE.md.
        assert_eq!(values.len(), 7);
    }

    #[test]
    fn parses_fixed_enum_subclass_attribute() {
        let TtlDeclaration::DatatypeAttribute(a) = parse_file(APP_SUBCLASS_TTL).expect("parses")
        else {
            panic!("expected datatype attribute");
        };
        let values = a.fixed_enum_values().expect("fixed enum");
        // From the XSD oracle ApplicationSubClass extraction: 50
        // distinct (Class, SubClass) PAIRS, but the SubClass *set*
        // is 50 values too (verified by the run cached in
        // `vocab/imports/ogit/MARS/_oracle/classifications.adoc`).
        assert_eq!(
            values.len(),
            50,
            "TTL subclass-set size must match XSD oracle (50)",
        );
    }

    /// The bijective oracle test: every Application classification
    /// in the OGIT TTL is also present in the XSD-extracted set
    /// (cached at `_oracle/classifications.adoc`). This is the
    /// chess-grade calibration applied to a frozen schema — same
    /// shape as `shakmaty::Position::play` round-tripping for chess.
    ///
    /// We assert membership here; the full bijection (TTL set ==
    /// XSD set, no missing, no extra) lives in the calibration
    /// doc under `docs/calibration/mars/README.md` so consumers
    /// can re-run it manually with `python3 _oracle/extract_classes_py3.py`.
    #[test]
    fn application_class_values_appear_in_xsd_oracle() {
        const ORACLE: &str =
            include_str!("../../../vocab/imports/ogit/NTO/MARS/_oracle/classifications.adoc");
        let TtlDeclaration::DatatypeAttribute(a) = parse_file(APP_CLASS_TTL).expect("parses")
        else {
            panic!("expected datatype attribute");
        };
        for value in a.fixed_enum_values().expect("fixed enum") {
            assert!(
                ORACLE.contains(&format!("|{value}|")),
                "TTL ApplicationClass value `{value}` missing from XSD oracle output",
            );
        }
    }

    #[test]
    fn into_class_carries_attribute_set_and_enums() {
        use super::super::into_class;
        let TtlDeclaration::Entity(machine) = parse_file(MACHINE_TTL).expect("parses") else {
            panic!("expected entity");
        };
        let TtlDeclaration::DatatypeAttribute(class_attr) = parse_file(include_str!(
            "../../../vocab/imports/ogit/NTO/MARS/Machine/attributes/class.ttl"
        ))
        .expect("parses") else {
            panic!("expected datatype attribute");
        };
        let attrs = vec![("class", &class_attr)];
        let cls = into_class(&machine, &attrs);
        assert_eq!(cls.name, "Machine");
        assert_eq!(cls.parent.as_deref(), Some("ogit:Entity"));
        // Behavior arm intentionally empty — schemas can't carry it.
        assert!(cls.callbacks.is_empty());
        // Enum was carried.
        assert_eq!(cls.enums.len(), 1);
        assert_eq!(cls.enums[0].column, "class");
    }
}
