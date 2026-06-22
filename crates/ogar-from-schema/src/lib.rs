//! `ogar-from-schema` — schema-as-input producer family.
//!
//! Sibling to source-AST producers (`ogar-from-rails`, `ogar-from-elixir`,
//! `ogar-from-ruff`). The pair carves the producer surface along the same
//! split [`SURREAL-AST-AS-ADAPTER.md`] already carved on the codegen side:
//!
//! ```text
//!   STRUCTURAL ARM  ──►  ogar-from-schema    (this crate)
//!   (XSD, TTL,             schemas are declarative & bijective
//!    JSON-Schema,          → round-trip provable
//!    OpenAPI, Prisma)      → 80 OGIT domains for free
//!
//!   BEHAVIORAL ARM  ──►  ogar-from-{rails,elixir,ruff,…}
//!   (callbacks, FSMs,     source code only — schemas can't carry it
//!    @api.depends,        → best-effort, language-specific
//!    gen_statem,…)        → adds what schemas can't see
//! ```
//!
//! **Why both — the funny insight** (`docs/HIRO-IN-CLASSES.md` §2):
//!
//! A schema gets you LESS, more RELIABLY. A source AST gets you MORE,
//! less reliably. The two are not redundant — they cover disjoint surfaces
//! that meet only at the structural arm, and at that meeting point they
//! become each other's oracle:
//!
//! - Schema lift produces a `Class` set that is **byte-exact** (the schema
//!   IS the contract).
//! - Source lift produces a `Class` set that is **best-effort** (Ruby is
//!   dynamic; `method_missing` defeats static extraction).
//! - Where both cover the same domain, emitting a schema from the
//!   source-lifted `Class` and diffing against the committed schema is a
//!   **drift detector** for every PR. That is what Palantir Foundry charges
//!   for as "ontology change management" — we get it from this crate
//!   plus `extract_classes.py`.
//!
//! # v0 scope
//!
//! - `ttl` front-end: reads the line-oriented OGIT TTL dialect into
//!   [`Class`]. Demo target: `vocab/imports/ogit/MARS/`.
//! - Cross-check: the **fixed-enum agreement test** asserts the TTL
//!   `ogit:validation-parameter` set matches the XSD oracle's extracted
//!   classifications (the chess-grade bijection, applied at the schema
//!   level — see `docs/calibration/mars/README.md`).
//!
//! # Out of v0 (queued)
//!
//! - `xsd` front-end (lift `MARSSchema2015.xsd` directly; cross-check vs TTL).
//! - `json_schema`, `openapi`, `prisma` front-ends.
//! - Full Turtle / RDF-XML / OWL import via `oxttl`/`oxrdf`.
//! - Behavioral-arm fields on `Class` are intentionally left empty by this
//!   crate — schemas can't carry them; source-AST producers fill them in.
//!
//! [`SURREAL-AST-AS-ADAPTER.md`]: ../docs/SURREAL-AST-AS-ADAPTER.md

#![warn(missing_docs)]
#![forbid(unsafe_code)]

use ogar_vocab::{Attribute, Class, EnumDecl, EnumSource, Language};

pub mod sgo;
pub mod ttl;
pub mod ttl_emit;

/// What a single TTL file describes — exactly one of: an entity (`Class`),
/// a datatype attribute (`Attribute`), or a verb (`Association` shape).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TtlDeclaration {
    /// An `rdfs:Class` declaration (`entities/<Name>.ttl`).
    Entity(EntityDecl),
    /// An `owl:DatatypeProperty` declaration (`<Entity>/attributes/<name>.ttl`).
    DatatypeAttribute(AttributeDecl),
}

/// Lifted shape of a single OGIT entity TTL file (e.g.
/// `entities/Machine.ttl`). The shape captures every predicate the OGIT
/// TTL dialect uses on `rdfs:Class` subjects, so that round-tripping
/// (`parse → emit → parse`) preserves the semantic content. Mapping into
/// the full [`ogar_vocab::Class`] surface happens in [`into_class`];
/// emitting back to TTL happens in [`crate::ttl_emit::emit_entity`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntityDecl {
    /// The CURIE-form name (`ogit.MARS:Machine`).
    pub curie: String,
    /// The local name (`Machine`).
    pub name: String,
    /// `rdfs:label` text (often equal to `name`).
    pub label: String,
    /// `dcterms:description` text — multi-line preserved verbatim.
    pub description: String,
    /// `rdfs:subClassOf` target as written (`ogit:Entity`).
    pub parent: Option<String>,
    /// `dcterms:valid` validity-range string (`"start=2018-06-01;"`).
    pub dcterms_valid: Option<String>,
    /// `dcterms:creator` text (`"fotto@arago.de"` / `"FCO"`).
    pub dcterms_creator: Option<String>,
    /// `ogit:scope` text (`"NTO"` / `"SGO"`).
    pub ogit_scope: Option<String>,
    /// `ogit:parent` token (`ogit:Node`) — separate from
    /// [`Self::parent`] (`rdfs:subClassOf`) — both can be present and
    /// carry different information.
    pub ogit_parent: Option<String>,
    /// `ogit:mandatory-attributes` list, as written.
    pub mandatory_attributes: Vec<String>,
    /// `ogit:optional-attributes` list, as written.
    pub optional_attributes: Vec<String>,
    /// `ogit:indexed-attributes` list, as written.
    pub indexed_attributes: Vec<String>,
    /// `ogit:allowed (...)` block — each entry is `(verb, target)` as
    /// written (`(ogit:dependsOn, ogit.MARS:Resource)`).
    pub allowed: Vec<(String, String)>,
}

/// Lifted shape of a single OGIT attribute TTL file (e.g.
/// `Application/attributes/class.ttl`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AttributeDecl {
    /// The CURIE-form name (`ogit.MARS.Application:class`).
    pub curie: String,
    /// The local name (`class`).
    pub name: String,
    /// `rdfs:label` text.
    pub label: String,
    /// `dcterms:description` text.
    pub description: String,
    /// `dcterms:valid` validity-range string.
    pub dcterms_valid: Option<String>,
    /// `dcterms:creator` text.
    pub dcterms_creator: Option<String>,
    /// `ogit:validation-type` value (e.g. `"fixed"`) when present.
    pub validation_type: Option<String>,
    /// `ogit:validation-parameter` value — when `validation_type` is
    /// `"fixed"`, this is the comma-separated set of allowed values.
    pub validation_parameter: Option<String>,
}

impl AttributeDecl {
    /// When this attribute is `validation-type="fixed"`, split the
    /// comma-separated parameter into a sorted, de-duplicated value set —
    /// the shape that **must agree with the XSD-extracted classification
    /// set** for the same `(entity, attribute)` pair.
    ///
    /// Returns `None` when the attribute is not fixed-enum.
    #[must_use]
    pub fn fixed_enum_values(&self) -> Option<Vec<String>> {
        if self.validation_type.as_deref() != Some("fixed") {
            return None;
        }
        let raw = self.validation_parameter.as_deref()?;
        let mut values: Vec<String> = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        values.sort();
        values.dedup();
        Some(values)
    }
}

/// Lower a lifted [`EntityDecl`] into the canonical [`ogar_vocab::Class`].
/// Behavior-arm fields ([`Class::callbacks`], etc.) stay empty — this is
/// the structural arm only, per the §`# v0 scope` invariant.
#[must_use]
pub fn into_class(entity: &EntityDecl, attributes: &[(&str, &AttributeDecl)]) -> Class {
    let enums: Vec<EnumDecl> = attributes
        .iter()
        .filter_map(|(col, attr)| {
            let values = attr.fixed_enum_values()?;
            // OGIT TTL carries the value list flat (`ogit:validation-parameter
            // "Data,Development,..."`) with no separate label vocabulary, so the
            // lowering renders each value as a self-labeled `(value, value)`
            // pair. A future XSD lift will source labels from
            // `<xs:documentation>` per `<xs:enumeration>`.
            let pairs: Vec<(String, String)> = values.into_iter().map(|v| (v.clone(), v)).collect();
            let mut decl = EnumDecl::default();
            decl.column = (*col).to_owned();
            decl.source = EnumSource::Static(pairs);
            Some(decl)
        })
        .collect();

    let attrs: Vec<Attribute> = attributes
        .iter()
        .map(|(col, _attr)| {
            let mut a = Attribute::default();
            a.name = (*col).to_owned();
            a
        })
        .collect();

    let mut cls = Class::default();
    cls.name = entity.name.clone();
    cls.parent = entity.parent.clone();
    cls.language = Language::Unknown;
    cls.attributes = attrs;
    cls.enums = enums;
    cls
}
