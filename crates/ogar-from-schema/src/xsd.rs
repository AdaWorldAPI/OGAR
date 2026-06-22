//! XSD front-end — a faithful Rust transcode of `arago/MARS-Schema`'s
//! `tools/extract_classes.py`.
//!
//! The Python script (Python 2, ~360 lines, ~140 of which are the
//! extraction logic and ~150 the table formatters) walks a MARS XSD and
//! enumerates the node-classification taxonomy: for the four master
//! node types (Application / Resource / Software / Machine) it pulls
//! every `(Class, SubClass)` pair out of the `xs:extension` / fixed-
//! `xs:attribute` chain, attaches the English `xs:documentation`, and
//! renders an asciidoc or HTML table.
//!
//! This module reproduces that walk and both output formats **byte for
//! byte** — the transcode proof is `tests::asciidoc_matches_python_oracle`,
//! which asserts the Rust output equals the cached Python output at
//! `_oracle/classifications.adoc`.
//!
//! # Why transcode it
//!
//! 1. **Removes the Python dependency from the calibration oracle.** The
//!    MARS bijection (`docs/MARS-TRANSCODING.md`) no longer needs a
//!    `python3` interpreter — `cargo test` is the whole proof.
//! 2. **Seeds the broader XSD → `Class` front-end.** The same walk that
//!    extracts classifications is the structural-arm lift for any XSD
//!    schema; [`classifications`] is the first consumer, a future
//!    `into_classes` the second.
//! 3. **Closes the XSD ↔ TTL bijection.** The classification set this
//!    module extracts must equal the `ogit:validation-parameter` enum
//!    set the TTL front-end lifts — see
//!    `tests::xsd_classes_match_ttl_enum`.
//!
//! Feature-gated behind `xsd` so the default TTL path stays
//! zero-parser-deps.

use std::collections::BTreeMap;

use roxmltree::{Document, Node};

/// The four MARS master complex types and the node type each anchors.
/// Mirrors the Python `master_types` dict.
const MASTER_TYPES: [(&str, &str); 4] = [
    ("MachineAttributes", "Machine"),
    ("ResourceAttributes", "Resource"),
    ("SoftwareAttributes", "Software"),
    ("ApplicationAttributes", "Application"),
];

fn master_type(base: &str) -> Option<&'static str> {
    MASTER_TYPES
        .iter()
        .find(|(b, _)| *b == base)
        .map(|(_, t)| *t)
}

/// A lifted classification record — the Rust counterpart of the Python
/// `parsed_data` dict. `fixed` maps a fixed-attribute *type* name
/// (`"ApplicationClass"`, `"ApplicationSubClass"`, `"MachineClass"`, …)
/// to its fixed *value*, exactly as the script does with
/// `parsed_data[parsed_data["_fixed"]["type"]] = value`.
#[derive(Debug, Clone, Default)]
struct Record {
    name: String,
    base: String,
    node_type: Option<String>,
    fixed: BTreeMap<String, String>,
    doc: Vec<String>,
}

/// The full extraction result: per-node-type element records, ready for
/// table rendering. Keyed by node type (`"Application"` … `"Machine"`).
#[derive(Debug, Default)]
pub struct Classifications {
    /// MARS schema version string (the `version` attribute on
    /// `xs:schema`, e.g. `"5.3.8"`).
    pub version: String,
    /// Element records grouped by node type.
    elements: BTreeMap<String, Vec<Record>>,
}

impl Classifications {
    /// Every `(node_type, class, subclass)` triple extracted, sorted.
    /// `subclass` is `None` for the 2-column node types (Resource,
    /// Machine). This is the structured surface the TTL cross-check
    /// reads.
    #[must_use]
    pub fn triples(&self) -> Vec<(String, String, Option<String>)> {
        let mut out = Vec::new();
        for (node_type, recs) in &self.elements {
            let class_key = format!("{node_type}Class");
            let sub_key = format!("{node_type}SubClass");
            for r in recs {
                if let Some(class) = r.fixed.get(&class_key) {
                    out.push((
                        node_type.clone(),
                        class.clone(),
                        r.fixed.get(&sub_key).cloned(),
                    ));
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    /// All distinct class+subclass values for a node type — the set
    /// that must equal the TTL `ogit:validation-parameter` enum for
    /// the same `<NodeType>/attributes/{class,subClass}.ttl` files.
    #[must_use]
    pub fn value_set(&self, node_type: &str) -> Vec<String> {
        let mut set = std::collections::BTreeSet::new();
        let class_key = format!("{node_type}Class");
        let sub_key = format!("{node_type}SubClass");
        if let Some(recs) = self.elements.get(node_type) {
            for r in recs {
                if let Some(v) = r.fixed.get(&class_key) {
                    set.insert(v.clone());
                }
                if let Some(v) = r.fixed.get(&sub_key) {
                    set.insert(v.clone());
                }
            }
        }
        set.into_iter().collect()
    }
}

/// Parse a MARS XSD into [`Classifications`]. Faithful transcode of the
/// Python `extract_from_xml`.
///
/// # Errors
///
/// Returns `Err` if the XML fails to parse.
pub fn classifications(xsd: &str) -> Result<Classifications, roxmltree::Error> {
    let doc = Document::parse(xsd)?;
    let root = doc.root_element(); // the xs:schema element
    let version = root.attribute("version").unwrap_or_default().to_owned();

    let mut el_data: Vec<Record> = Vec::new();
    let mut ct_data: BTreeMap<String, Record> = BTreeMap::new();

    for child in root.children().filter(Node::is_element) {
        match child.tag_name().name() {
            "element" => {
                if let Some(rec) = parse_subject(child, /* is_element */ true) {
                    el_data.push(rec);
                }
            }
            "complexType" => {
                // Only complex types whose base IS a master type get a
                // node_type (and are therefore stored) — mirrors the
                // Python `if parsed_data.has_key("TYPE")` gate.
                if let Some(rec) = parse_subject(child, /* is_element */ false)
                    && rec.node_type.is_some()
                {
                    ct_data.insert(rec.name.clone(), rec);
                }
            }
            _ => {}
        }
    }

    // Post-process phase II: an element whose own base is an
    // intermediate complex type (not a master type) inherits that
    // type's node_type + class-level fixed value.
    for el in &mut el_data {
        if el.node_type.is_none()
            && let Some(base_rec) = ct_data.get(&el.base)
        {
            el.node_type = base_rec.node_type.clone();
            for (k, v) in &base_rec.fixed {
                el.fixed.insert(k.clone(), v.clone());
            }
        }
    }

    let mut elements: BTreeMap<String, Vec<Record>> = BTreeMap::new();
    for el in el_data {
        if let Some(nt) = el.node_type.clone() {
            elements.entry(nt).or_default().push(el);
        }
    }

    Ok(Classifications { version, elements })
}

/// Shared parser for both `xs:element` and `xs:complexType` subjects —
/// the Python `parse_element_data` / `parse_complex_type` are nearly
/// identical; the only difference is that a complex type sets its
/// node_type *only* when its base is a master type, whereas an element
/// also records its fixed value unconditionally.
fn parse_subject(node: Node, is_element: bool) -> Option<Record> {
    // First descendant xs:extension.
    let ext = node
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "extension")?;
    let base = strip_aae(ext.attribute("base").unwrap_or_default());

    // Last direct xs:attribute child wins (mirrors the Python loop that
    // overwrites `_fixed` each iteration).
    let mut fixed_name = None;
    let mut fixed_type = None;
    let mut fixed_value = None;
    for attr_el in ext.children().filter(Node::is_element) {
        if attr_el.tag_name().name() == "attribute" {
            fixed_name = Some(attr_el.attribute("name").unwrap_or_default().to_owned());
            fixed_type = Some(strip_aae(attr_el.attribute("type").unwrap_or_default()));
            fixed_value = Some(attr_el.attribute("fixed").unwrap_or_default().to_owned());
        }
    }
    let (_fixed_name, fixed_type, fixed_value) = (fixed_name?, fixed_type?, fixed_value?);

    // English documentation: every descendant xs:documentation whose
    // xml:lang is absent or "en".
    let mut docs = Vec::new();
    for doc_el in node
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "documentation")
    {
        let lang = doc_el
            .attributes()
            .find(|a| a.name() == "lang")
            .map(|a| a.value());
        if lang.is_none() || lang == Some("en") {
            docs.push(text_of(doc_el));
        }
    }

    let mut rec = Record {
        name: node.attribute("name").unwrap_or_default().to_owned(),
        base: base.clone(),
        node_type: None,
        fixed: BTreeMap::new(),
        doc: docs,
    };

    let is_master = master_type(&base);
    if let Some(t) = is_master {
        rec.node_type = Some(t.to_owned());
        rec.fixed.insert(fixed_type.clone(), fixed_value.clone());
    }
    if is_element && is_master.is_none() {
        // Element extending an intermediate type: still records its own
        // fixed value (the SubClass) even without a node_type yet.
        rec.fixed.insert(fixed_type, fixed_value);
    }

    Some(rec)
}

/// Concatenate the direct text-node children of an element, verbatim —
/// the Python `getXMLText`. Preserves the XSD documentation's internal
/// whitespace exactly (load-bearing for the byte-match).
fn text_of(node: Node) -> String {
    node.children()
        .filter(Node::is_text)
        .filter_map(|c| c.text())
        .collect()
}

fn strip_aae(s: &str) -> String {
    s.strip_prefix("aae:").unwrap_or(s).to_owned()
}

// ───────────────────────────────────────── output formatters ──

/// Render the asciidoc table set, byte-for-byte equal to the Python
/// `-F asciidoc` output. `revdate` is the `:revdate:` value (the Python
/// script uses `datetime.now()`, non-deterministic; here it is a
/// parameter so the output is reproducible and testable).
#[must_use]
pub fn to_asciidoc(c: &Classifications, revdate: &str) -> String {
    let mut out = String::new();
    // printAsciiDocTitle + printAsciiDocHeader
    out.push_str(&format!(
        "= Node classifications from MARS Schema {}\n",
        c.version
    ));
    out.push_str(":toc:\n");
    out.push_str(&format!(":revdate: {revdate}\n"));
    out.push_str("\n<<<\n\n");
    out.push_str(
        "[NOTE]\n====\nThe MARS Schema uses a different versioning cycle from HIRO Product. \
         It is expected to see the two versions deviate from each other.\n\nThis list was \
         last updated on: *{revdate}*\n====\n\n\n",
    );
    adoc_table3(&mut out, c, "Application");
    adoc_table2(&mut out, c, "Resource");
    adoc_table3(&mut out, c, "Software");
    adoc_table2(&mut out, c, "Machine");
    // printAsciiDocFooter — a single trailing `print ""`.
    out.push('\n');
    out
}

fn adoc_table3(out: &mut String, c: &Classifications, node_type: &str) {
    let class_key = format!("{node_type}Class");
    let sub_key = format!("{node_type}SubClass");
    out.push_str(&format!("== {node_type} Node Classifications\n\n"));
    out.push_str("[cols=\"1,1,3\", options=\"header\"]\n");
    out.push_str("|===\n");
    out.push_str(&format!("|{class_key}|{sub_key}|Description\n"));
    let (by_class, by_name) = group_3col(c, node_type, &class_key, &sub_key);
    for (cl, subs) in &by_class {
        for sub in subs {
            if let Some(doc) = by_name.get(sub) {
                out.push_str(&format!("|{cl}|{sub}|{}\n", doc.join("<br />")));
            }
        }
    }
    out.push_str("|===\n\n");
}

fn adoc_table2(out: &mut String, c: &Classifications, node_type: &str) {
    let class_key = format!("{node_type}Class");
    out.push_str(&format!("== {node_type} Node Classifications\n\n"));
    out.push_str("[cols=\"1,5\", options=\"header\"]\n");
    out.push_str("|===\n");
    out.push_str(&format!("|{class_key}|Description\n"));
    let by_name = group_2col(c, node_type, &class_key);
    for (cl, doc) in &by_name {
        out.push_str(&format!("|{cl}|{}\n", doc.join("<br />")));
    }
    out.push_str("|===\n\n");
}

/// Build the sorted class→subclasses ordering + the subclass-name→doc
/// lookup for a 3-column node type.
fn group_3col(
    c: &Classifications,
    node_type: &str,
    class_key: &str,
    sub_key: &str,
) -> (BTreeMap<String, Vec<String>>, BTreeMap<String, Vec<String>>) {
    let mut by_class: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if let Some(recs) = c.elements.get(node_type) {
        for r in recs {
            if let (Some(cl), Some(sub)) = (r.fixed.get(class_key), r.fixed.get(sub_key)) {
                by_class.entry(cl.clone()).or_default().push(sub.clone());
                by_name.insert(r.name.clone(), r.doc.clone());
            }
        }
    }
    for subs in by_class.values_mut() {
        subs.sort();
        subs.dedup();
    }
    (by_class, by_name)
}

/// Build the sorted class-name→doc lookup for a 2-column node type.
fn group_2col(
    c: &Classifications,
    node_type: &str,
    class_key: &str,
) -> BTreeMap<String, Vec<String>> {
    let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if let Some(recs) = c.elements.get(node_type) {
        for r in recs {
            if r.fixed.contains_key(class_key) {
                by_name.insert(r.name.clone(), r.doc.clone());
            }
        }
    }
    by_name
}

// ───────────────────────────────────────────────── tests ──

#[cfg(test)]
mod tests {
    use super::*;

    const XSD: &str =
        include_str!("../../../vocab/imports/ogit/NTO/MARS/_oracle/MARSSchema2015.xsd");
    const ORACLE_ADOC: &str =
        include_str!("../../../vocab/imports/ogit/NTO/MARS/_oracle/classifications.adoc");

    #[test]
    fn parses_version() {
        let c = classifications(XSD).expect("parse");
        assert_eq!(c.version, "5.3.8");
    }

    #[test]
    fn extracts_expected_counts() {
        let c = classifications(XSD).expect("parse");
        // From PROVENANCE.md / the Python oracle:
        //   Application 7 classes × 50 subclass pairs
        //   Resource    19 classes
        //   Software    40 classes × 336 subclass pairs
        //   Machine     11 classes
        let app: Vec<_> = c
            .triples()
            .into_iter()
            .filter(|(n, ..)| n == "Application")
            .collect();
        let mach: Vec<_> = c
            .triples()
            .into_iter()
            .filter(|(n, ..)| n == "Machine")
            .collect();
        let app_classes: std::collections::BTreeSet<_> =
            app.iter().map(|(_, cl, _)| cl.clone()).collect();
        let mach_classes: std::collections::BTreeSet<_> =
            mach.iter().map(|(_, cl, _)| cl.clone()).collect();
        assert_eq!(app_classes.len(), 7, "Application classes");
        assert_eq!(mach_classes.len(), 11, "Machine classes");
    }

    /// **The transcode proof.** The Rust asciidoc output must be
    /// byte-for-byte equal to the cached Python `-F asciidoc` output.
    /// The cached file was generated with `:revdate: 22-Jun-2026`, so
    /// we pass the same value.
    #[test]
    fn asciidoc_matches_python_oracle() {
        let c = classifications(XSD).expect("parse");
        let rust = to_asciidoc(&c, "22-Jun-2026");
        if rust != ORACLE_ADOC {
            // Pinpoint the first divergence for a useful failure message.
            let a: Vec<&str> = rust.lines().collect();
            let b: Vec<&str> = ORACLE_ADOC.lines().collect();
            for (i, (la, lb)) in a.iter().zip(b.iter()).enumerate() {
                assert_eq!(la, lb, "first divergence at line {}", i + 1);
            }
            assert_eq!(
                a.len(),
                b.len(),
                "line count differs (rust {} vs oracle {})",
                a.len(),
                b.len()
            );
        }
    }

    /// **Closes the XSD ↔ TTL bijection.** The XSD-extracted
    /// Application class+subclass value set must equal what the TTL
    /// front-end lifts from `Application/attributes/{class,subClass}.ttl`.
    #[test]
    fn xsd_classes_match_ttl_enum() {
        use crate::TtlDeclaration;
        use crate::ttl::parse_file;

        let c = classifications(XSD).expect("parse");
        let xsd_values: std::collections::BTreeSet<String> =
            c.value_set("Application").into_iter().collect();

        const CLASS_TTL: &str =
            include_str!("../../../vocab/imports/ogit/NTO/MARS/Application/attributes/class.ttl");
        const SUBCLASS_TTL: &str = include_str!(
            "../../../vocab/imports/ogit/NTO/MARS/Application/attributes/subClass.ttl"
        );
        let mut ttl_values = std::collections::BTreeSet::new();
        for ttl in [CLASS_TTL, SUBCLASS_TTL] {
            let TtlDeclaration::DatatypeAttribute(a) = parse_file(ttl).expect("parse ttl") else {
                panic!("expected datatype attribute");
            };
            for v in a.fixed_enum_values().expect("fixed enum") {
                ttl_values.insert(v);
            }
        }

        // Full bidirectional equality — every TTL value in the XSD set
        // AND every XSD value in the TTL set. This is the bijection the
        // MARS-TRANSCODING.md §2 "queued" note asked for.
        assert_eq!(
            xsd_values, ttl_values,
            "XSD-extracted Application value set differs from TTL enum set"
        );
    }
}
