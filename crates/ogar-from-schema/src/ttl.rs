//! OGIT TTL front-end.
//!
//! Reads the line-oriented subset of Turtle the OGIT NTO files actually
//! use. Not a full Turtle parser — the OGIT dialect is machine-stable
//! and narrow: prefix declarations, one `rdfs:Class` or
//! `owl:DatatypeProperty` subject per file, predicate-object lines
//! ending in `;`, the file terminator `.`. Whitespace between tokens is
//! free (OGIT aligns columns), `#` comments and string contents are skipped
//! when locating predicates and list ends (see [`mask`]).
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

/// Parse every declaration in one OGIT TTL file's bytes, in file order.
///
/// A file normally holds one subject, but may hold several (an entity file
/// that also declares its attributes). Each statement, up to its closing `.`,
/// is read as [`parse_file`] reads a whole file; statements that declare
/// nothing recognised are left out. [`parse_file`] keeps returning only the
/// first kind it finds in the whole file.
#[must_use]
pub fn parse_all(src: &str) -> Vec<TtlDeclaration> {
    statements(src).into_iter().filter_map(parse_file).collect()
}

#[derive(Debug, Clone, Copy)]
enum SubjectKind {
    Class,
    DatatypeProperty,
}

fn detect_kind(src: &str) -> Option<SubjectKind> {
    declared_types(&mask(src)).find_map(|ty| match ty {
        "rdfs:Class" => Some(SubjectKind::Class),
        "owl:DatatypeProperty" => Some(SubjectKind::DatatypeProperty),
        _ => None,
    })
}

/// The types a file declares: the token after each standalone `a`, whatever
/// whitespace separates them. Takes [`mask`]ed text, so prose that happens to
/// say `a rdfs:Class` is not a declaration.
pub(crate) fn declared_types(masked: &str) -> impl Iterator<Item = &str> {
    let mut toks = masked.split_whitespace();
    std::iter::from_fn(move || {
        loop {
            if toks.next()? == "a" {
                let ty = toks.next()?;
                return Some(ty.trim_end_matches([';', ',', '.']));
            }
        }
    })
}

/// `src` with every `#` comment and the inside of every string literal
/// blanked to spaces. Same byte length, same line breaks, so offsets and line
/// numbers carry over. `<...>` IRIs are kept: a `#` there is a fragment.
pub(crate) fn mask(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let blank = |out: &mut String, c: char| {
        if c == '\n' || c == '\r' {
            out.push(c);
        } else {
            out.extend(std::iter::repeat_n(' ', c.len_utf8()));
        }
    };
    let mut i = 0;
    while i < src.len() {
        let c = src[i..].chars().next().expect("i is on a char boundary");
        match c {
            '#' => {
                while let Some(c) = src[i..].chars().next().filter(|&c| c != '\n') {
                    blank(&mut out, c);
                    i += c.len_utf8();
                }
            }
            '"' | '\'' => {
                let closer = [bytes[i]; 3];
                let delim = if bytes[i..].starts_with(&closer) {
                    3
                } else {
                    1
                };
                out.push_str(&src[i..i + delim]);
                i += delim;
                while i < src.len() {
                    if bytes[i..].starts_with(&closer[..delim]) {
                        out.push_str(&src[i..i + delim]);
                        i += delim;
                        break;
                    }
                    let c = src[i..].chars().next().expect("i is on a char boundary");
                    blank(&mut out, c);
                    i += c.len_utf8();
                    if c == '\\'
                        && let Some(escaped) = src[i..].chars().next()
                    {
                        blank(&mut out, escaped);
                        i += escaped.len_utf8();
                    }
                }
            }
            '<' => {
                // An IRI has no whitespace; a lone `<` is just a character.
                let end = src[i..]
                    .find(|c: char| c == '>' || c.is_whitespace())
                    .filter(|&n| bytes[i + n] == b'>');
                let len = end.map_or(1, |n| n + 1);
                out.push_str(&src[i..i + len]);
                i += len;
            }
            _ => {
                out.push(c);
                i += c.len_utf8();
            }
        }
    }
    out
}

/// The statements of `src`, each up to and including the `.` that closes it
/// (a `.` outside brackets and followed by whitespace). Prefix declarations
/// are left out; an unterminated last statement is kept.
fn statements(src: &str) -> Vec<&str> {
    let masked = mask(src);
    let mut out = Vec::new();
    let (mut start, mut depth) = (0, 0usize);
    let mut chars = masked.char_indices().peekable();
    let mut push = |from: usize, to: usize| {
        let Some(first) = masked[from..to].split_whitespace().next() else {
            return;
        };
        let directive = first.starts_with('@')
            || first.eq_ignore_ascii_case("prefix")
            || first.eq_ignore_ascii_case("base");
        if !directive {
            out.push(&src[from..to]);
        }
    };
    while let Some((i, c)) = chars.next() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            '.' if depth == 0 && chars.peek().is_none_or(|(_, n)| n.is_whitespace()) => {
                push(start, i + 1);
                start = i + 1;
            }
            _ => {}
        }
    }
    push(start, src.len());
    out
}

pub(crate) fn find_subject_curie(src: &str) -> Option<String> {
    // The subject CURIE is the first non-prefix, non-blank, non-comment
    // line whose content does not start with `@` or whitespace. OGIT
    // declares it at column zero followed by an indented `a rdfs:Class;`
    // on the next line.
    // A trailing `#` comment (`ogit.X:old #deprecated`) is not part of it.
    for line in mask(src).lines() {
        if line.starts_with('@')
            || line.is_empty()
            || line.starts_with('#')
            || line.starts_with(char::is_whitespace)
        {
            continue;
        }
        let candidate = line.trim().trim_end_matches(';').trim();
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
    let masked = mask(src);
    let mandatory_attributes = extract_paren_list(&masked, "ogit:mandatory-attributes");
    let optional_attributes = extract_paren_list(&masked, "ogit:optional-attributes");
    let indexed_attributes = extract_paren_list(&masked, "ogit:indexed-attributes");
    let allowed = extract_allowed_block(&masked);
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
pub(crate) fn extract_token_value(src: &str, predicate: &str) -> Option<String> {
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

/// The text between the parentheses of `predicate (...)`, from [`mask`]ed
/// text, so a predicate named in a string or comment is not the one found, and
/// a `)` in a comment does not close the list. `None` when no occurrence of
/// the predicate is followed by a list.
fn list_body<'a>(masked: &'a str, predicate: &str) -> Option<&'a str> {
    masked.match_indices(predicate).find_map(|(idx, _)| {
        let before = masked[..idx].chars().next_back();
        let after = &masked[idx + predicate.len()..];
        let whole = before.is_none_or(char::is_whitespace)
            && after.starts_with(|c: char| c.is_whitespace() || c == '(');
        let body = after.trim_start().strip_prefix('(').filter(|_| whole)?;
        body.find(')').map(|close| &body[..close])
    })
}

/// Extract the entries inside an `ogit:mandatory-attributes (...)` block.
fn extract_paren_list(masked: &str, predicate: &str) -> Vec<String> {
    list_body(masked, predicate)
        .unwrap_or_default()
        .split_whitespace()
        .filter(|tok| *tok != ";")
        .map(str::to_owned)
        .collect()
}

/// Extract the `ogit:allowed (...)` block — each entry is
/// `[ <verb> <target> ]` on its own line.
fn extract_allowed_block(masked: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in list_body(masked, "ogit:allowed")
        .unwrap_or_default()
        .lines()
    {
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

    fn mars(path: &str) -> String {
        crate::ogit_checkout::read(&format!("NTO/MARS/{path}"))
    }

    const PREFIXES: &str = "@prefix ogit: <http://www.purl.org/ogit/> .\n\
        @prefix ogit.X: <http://www.purl.org/ogit/X/> .\n";

    fn entity(src: &str) -> EntityDecl {
        match parse_file(src) {
            Some(TtlDeclaration::Entity(e)) => e,
            other => panic!("expected entity, got {other:?}"),
        }
    }

    #[test]
    fn any_whitespace_may_follow_a() {
        // OGIT aligns columns: several spaces, or a tab, after `a`.
        let e = entity(&format!(
            "{PREFIXES}\nogit.X:Foo\n    a                   rdfs:Class ;\n    \
             rdfs:subClassOf     ogit:Entity .\n"
        ));
        assert_eq!(e.curie, "ogit.X:Foo");
        let e = entity(&format!(
            "{PREFIXES}\nogit.X:Foo\n\ta\t\trdfs:Class;\n\trdfs:subClassOf ogit:Entity;\n.\n"
        ));
        assert_eq!(e.parent.as_deref(), Some("ogit:Entity"));
        let src = format!(
            "{PREFIXES}\nogit.X:bar\n    a     owl:DatatypeProperty ;\n    rdfs:label \"bar\" .\n"
        );
        assert!(matches!(
            parse_file(&src),
            Some(TtlDeclaration::DatatypeAttribute(a)) if a.name == "bar"
        ));
    }

    #[test]
    fn a_comment_after_the_subject_is_not_part_of_its_name() {
        let src = format!(
            "{PREFIXES}\nogit.X:old #deprecated\n    a owl:DatatypeProperty;\n    rdfs:label \"old\" .\n"
        );
        assert!(matches!(
            parse_file(&src),
            Some(TtlDeclaration::DatatypeAttribute(a)) if a.curie == "ogit.X:old" && a.name == "old"
        ));
    }

    #[test]
    fn a_type_named_in_prose_is_not_a_declaration() {
        let src = format!(
            "{PREFIXES}\nogit.X:Foo\n    dcterms:description \"this is a rdfs:Class of things\" ;\n    \
             rdfs:label \"Foo\" .\n"
        );
        assert!(parse_file(&src).is_none());
    }

    #[test]
    fn comments_in_an_attribute_list_are_not_attributes() {
        let e = entity(&format!(
            "{PREFIXES}\nogit.X:Foo\n    a rdfs:Class ;\n    rdfs:subClassOf ogit:Entity ;\n    \
             ogit:mandatory-attributes (\n        ogit:name # the name (display)\n        \
             # ogit.X:old\n        ogit.X:id\n    ) ;\n    \
             ogit:optional-attributes ( ogit.X:b ) .\n"
        ));
        assert_eq!(e.mandatory_attributes, ["ogit:name", "ogit.X:id"]);
        assert_eq!(e.optional_attributes, ["ogit.X:b"]);
    }

    #[test]
    fn the_allowed_block_ignores_comments_and_strings() {
        let e = entity(&format!(
            "{PREFIXES}\nogit.X:Foo\n    a rdfs:Class ;\n    rdfs:subClassOf ogit:Entity ;\n    \
             dcterms:description \"the ogit:allowed (list) names edges\" ;\n    \
             ogit:allowed (\n        [ ogit:has ogit.X:A ] # first (of two)\n        \
             [ ogit:uses ogit.X:B ]\n        # [ ogit:old ogit.X:C ]\n    ) .\n"
        ));
        assert_eq!(
            e.allowed,
            [
                ("ogit:has".to_owned(), "ogit.X:A".to_owned()),
                ("ogit:uses".to_owned(), "ogit.X:B".to_owned()),
            ]
        );
    }

    #[test]
    fn parse_all_returns_every_declaration_of_a_file() {
        let src = format!(
            "{PREFIXES}\n# an entity and its attribute\nogit.X:Foo\n    a rdfs:Class ;\n    \
             rdfs:subClassOf ogit:Entity ;\n    ogit:optional-attributes ( ogit.X:bar ) .\n\n\
             ogit.X:bar\n    a   owl:DatatypeProperty ;\n    rdfs:label \"bar\" ;\n    \
             dcterms:description \"ends with a dot. \" .\n\nogit.X:note\n    rdfs:label \"n\" .\n"
        );
        let all = parse_all(&src);
        assert_eq!(all.len(), 2, "{all:?}");
        assert!(matches!(&all[0], TtlDeclaration::Entity(e) if e.name == "Foo"));
        assert!(matches!(&all[1], TtlDeclaration::DatatypeAttribute(a) if a.name == "bar"));
        // `parse_file` still returns the first kind in the file.
        assert!(matches!(parse_file(&src), Some(TtlDeclaration::Entity(_))));
        // A file with one subject and no closing `.` still yields it.
        let open = format!("{PREFIXES}\nogit.X:Foo\n    a rdfs:Class ;\n    rdfs:label \"F\" ;\n");
        assert_eq!(parse_all(&open).len(), 1);
        assert!(parse_all(PREFIXES).is_empty());
    }

    #[test]
    fn mask_blanks_comments_and_strings_but_not_iris() {
        let src = "<http://a/#b> \"QQ # RR\" # CC\n'''PP\nSS''' ogit:z";
        let m = mask(src);
        assert_eq!(m.len(), src.len());
        assert_eq!(m.lines().count(), src.lines().count());
        assert!(m.starts_with("<http://a/#b> \""));
        assert!(
            ["QQ", "RR", "CC", "PP", "SS"]
                .iter()
                .all(|w| !m.contains(w))
        );
        assert!(m.ends_with("ogit:z"));
    }

    #[test]
    fn parses_machine_entity() {
        let TtlDeclaration::Entity(e) = parse_file(&mars("entities/Machine.ttl")).expect("parses")
        else {
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
        let TtlDeclaration::Entity(e) =
            parse_file(&mars("entities/Application.ttl")).expect("parses")
        else {
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
        let TtlDeclaration::DatatypeAttribute(a) =
            parse_file(&mars("Application/attributes/class.ttl")).expect("parses")
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
        // Matches the XSD oracle's ApplicationClass count (7).
        assert_eq!(values.len(), 7);
    }

    #[test]
    fn parses_fixed_enum_subclass_attribute() {
        let TtlDeclaration::DatatypeAttribute(a) =
            parse_file(&mars("Application/attributes/subClass.ttl")).expect("parses")
        else {
            panic!("expected datatype attribute");
        };
        let values = a.fixed_enum_values().expect("fixed enum");
        // From the XSD oracle ApplicationSubClass extraction: 50
        // distinct (Class, SubClass) PAIRS, but the SubClass *set*
        // is 50 values too (verified by the run cached in
        // `vocab/oracles/mars/classifications.adoc`).
        assert_eq!(
            values.len(),
            50,
            "TTL subclass-set size must match XSD oracle (50)",
        );
    }

    /// The bijective oracle test: every Application classification
    /// in the OGIT TTL is also present in the XSD-extracted set
    /// (cached at `vocab/oracles/mars/classifications.adoc`). This is the
    /// chess-grade calibration applied to a frozen schema — same
    /// shape as `shakmaty::Position::play` round-tripping for chess.
    ///
    /// We assert membership here; the full bijection (TTL set ==
    /// XSD set, no missing, no extra) lives in the calibration
    /// doc under `docs/calibration/mars/README.md` so consumers
    /// can re-run it manually with
    /// `python3 vocab/oracles/mars/extract_classes_py3.py`.
    #[test]
    fn application_class_values_appear_in_xsd_oracle() {
        const ORACLE: &str = include_str!("../../../vocab/oracles/mars/classifications.adoc");
        let TtlDeclaration::DatatypeAttribute(a) =
            parse_file(&mars("Application/attributes/class.ttl")).expect("parses")
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
        let TtlDeclaration::Entity(machine) =
            parse_file(&mars("entities/Machine.ttl")).expect("parses")
        else {
            panic!("expected entity");
        };
        let TtlDeclaration::DatatypeAttribute(class_attr) =
            parse_file(&mars("Machine/attributes/class.ttl")).expect("parses")
        else {
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
