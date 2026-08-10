//! # `rdf` — retain the assertions, key them by the CURIE numeric.
//!
//! The workspace already parses RDF: `lance-graph-ontology`'s `OwlHydrator`
//! reads Turtle and RDF/XML through `oxttl` / `oxrdfxml`. But it interns every
//! named IRI to a **sequential counter** and then *drops the triple* — its
//! `OntologySlot` is `{ entity_count, iri_to_id }` and nothing else. Two
//! consequences make it unusable as a bake front-end:
//!
//! 1. **No assertions survive.** There is no `subClassOf` anywhere after
//!    hydration; `ContextBundle::edge_types` is a whitelist of predicate IRIs
//!    a traversal *may* follow, not stored edges.
//! 2. **The identity is order-dependent.** An intern counter depends on file
//!    order, so the same ontology hydrated from a different file list yields
//!    different ids. A bake keyed on it would not be reproducible.
//!
//! This module is the opposite of both: it **retains** the edges and takes the
//! identity from the term itself — `…/obo/HP_0000118` → `HP:0000118` → the
//! 24-bit numeric `118`, which is exactly the identity [`crate::pack_key`]
//! writes on the `family:identity` rail. Bake the same ontology from any file
//! order and the rows are byte-identical.
//!
//! It is NOT a second parser: `oxttl` / `oxrdfxml` do the reading, pinned to
//! the same versions `lance-graph-ontology` pins.
//!
//! ## The extraction map IS the [`NsRegistry`]
//!
//! Every OBO PURL is `<base><PREFIX>_<NNNNNNN>`. One rule decodes all of them,
//! so a new ontology needs a **table row**, never a code path. That is the
//! whole per-ontology config: prefix + concept id.
//!
//! ## Predicates come from `owl:onProperty`, not from a namespace-pair guess
//!
//! [`crate::parse_obo`] classifies an edge by *which two ontologies it joins*
//! ([`crate::classify`]) — a deliberate choice, because the `.obo`
//! `relationship:` label string is unreliable. RDF does not have that problem:
//! a restriction names its property explicitly, so this path reads
//! [`OBO_RESTRICTION_PREDICATES`] and falls through to
//! [`Predicate::Other`](crate::Predicate::Other) rather than guessing.
//!
//! **This means an RDF bake and a `.obo` bake of the same ontology may assign
//! different predicates to the same edge.** That is a real difference between
//! two sources, not a bug in either; the shipped five-namespace bake was
//! produced from `.obo` and is not re-baked by this module.
//!
//! ## Documented limits (not silent gaps)
//!
//! - Only **direct** restrictions are harvested: `X subClassOf [ onProperty P ;
//!   someValuesFrom Y ]` and the same shape under `owl:equivalentClass`.
//!   Restrictions nested inside an `owl:intersectionOf` RDF **collection** are
//!   NOT walked, so logical definitions expressed that way yield no edge here.
//! - `owl:equivalentClass` to a **named** class is skipped. Equivalence is not
//!   subsumption, and recording it as `is_a` would invent a parent.
//! - A term becomes a node only if it is asserted `a owl:Class`. Being merely
//!   *mentioned* as some other term's parent does not create a row — the same
//!   rule `.obo` follows with its `id:` stanza line.

use std::collections::HashMap;

use oxrdf::{NamedOrBlankNode, Term};
use oxrdfxml::RdfXmlParser;
use oxttl::TurtleParser;

use crate::registry::NsRegistry;
use crate::{OboNode, Predicate, TermId, Xref, XrefSource};

/// The OBO PURL base every OBO Foundry term IRI is built on.
pub const OBO_PURL_BASE: &str = "http://purl.obolibrary.org/obo/";

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const OWL_ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
const OWL_SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
const OWL_EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
const OWL_DEPRECATED: &str = "http://www.w3.org/2002/07/owl#deprecated";
const OBO_IN_OWL_HAS_DB_XREF: &str = "http://www.geneontology.org/formats/oboInOwl#hasDbXref";

/// The RDF serializations this module reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RdfFormat {
    /// Turtle / N-Triples family (`oxttl`).
    Turtle,
    /// RDF/XML (`oxrdfxml`).
    RdfXml,
}

/// Choose a serialization from the file extension, falling back to sniffing
/// the head of the document.
///
/// `.owl` is genuinely ambiguous — it ships in both forms in the wild — so an
/// unknown or `.owl` extension sniffs for an XML preamble or an `<rdf:RDF>`
/// opening tag and defaults to Turtle otherwise. Same rule
/// `lance-graph-ontology` uses, so the two agree on any given file.
#[must_use]
pub fn detect_format(extension: Option<&str>, head: &[u8]) -> RdfFormat {
    match extension.map(str::to_ascii_lowercase).as_deref() {
        Some("rdf" | "rdfxml" | "owx") => RdfFormat::RdfXml,
        Some("ttl" | "nt" | "n3" | "trig") => RdfFormat::Turtle,
        _ => {
            let head = &head[..head.len().min(256)];
            let start = head
                .iter()
                .position(|b| !b.is_ascii_whitespace() && *b != 0xEF && *b != 0xBB && *b != 0xBF)
                .unwrap_or(0);
            let trimmed = &head[start..];
            if trimmed.starts_with(b"<?xml") || trimmed.starts_with(b"<rdf:RDF") {
                RdfFormat::RdfXml
            } else {
                RdfFormat::Turtle
            }
        }
    }
}

/// One `object-property CURIE → RO predicate` row. Config, like [`NsSpec`].
///
/// [`NsSpec`]: crate::registry::NsSpec
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PredSpec {
    /// The property's OBO CURIE, e.g. `"BFO:0000050"`.
    pub curie: &'static str,
    /// The RO palette byte it maps to.
    pub predicate: Predicate,
}

/// The restriction properties this crate already names in its
/// [`Predicate`](crate::Predicate) documentation — read from there, not
/// invented here.
///
/// [`Predicate::HasAnatomy`](crate::Predicate::HasAnatomy) and
/// [`Predicate::HasQuality`](crate::Predicate::HasQuality) are deliberately
/// **absent**: the crate documents them as *harvested from a logical
/// definition*, never as a named property, so there is no CURIE to put here
/// that would not be a guess. Edges carrying those properties land on
/// [`Predicate::Other`](crate::Predicate::Other).
pub const OBO_RESTRICTION_PREDICATES: &[PredSpec] = &[
    PredSpec {
        curie: "BFO:0000050",
        predicate: Predicate::PartOf,
    },
    PredSpec {
        curie: "RO:0002200",
        predicate: Predicate::HasPhenotype,
    },
    PredSpec {
        curie: "RO:0004026",
        predicate: Predicate::HasLocation,
    },
];

/// Failure reading an RDF document.
#[derive(Debug)]
pub struct RdfErr {
    /// The underlying parser's message.
    pub message: String,
}

impl std::fmt::Display for RdfErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RDF parse error: {}", self.message)
    }
}

impl std::error::Error for RdfErr {}

/// Split an OBO PURL into its `(prefix, 24-bit numeric)` parts.
///
/// `None` when the IRI is not an OBO PURL, has no `_` separator, or the
/// numeric is malformed or exceeds 24 bits — the same width contract
/// [`TermId::parse`] enforces.
#[must_use]
pub fn purl_to_curie(iri: &str) -> Option<(&str, u32)> {
    let local = iri.strip_prefix(OBO_PURL_BASE)?;
    let (prefix, digits) = local.rsplit_once('_')?;
    if prefix.is_empty() {
        return None;
    }
    let num: u32 = digits.parse().ok()?;
    if num > 0x00FF_FFFF {
        return None;
    }
    Some((prefix, num))
}

/// Resolve an OBO PURL to a [`TermId`] under a registry, or `None` if the
/// prefix is outside the table (imports from ontologies this bake does not
/// tile are skipped, exactly as [`crate::parse_obo`] skips them).
#[must_use]
pub fn term_of(iri: &str, reg: &NsRegistry) -> Option<TermId> {
    let (prefix, num) = purl_to_curie(iri)?;
    Some(TermId {
        ns: reg.ordinal_of(prefix)?,
        num,
    })
}

/// Map a restriction's property IRI to an RO predicate through a table.
#[must_use]
pub fn predicate_of(property_iri: &str, table: &[PredSpec]) -> Predicate {
    let Some((prefix, num)) = purl_to_curie(property_iri) else {
        return Predicate::Other;
    };
    table
        .iter()
        .find(|s| {
            s.curie
                .split_once(':')
                .is_some_and(|(p, n)| p == prefix && n.parse::<u32>().ok() == Some(num))
        })
        .map_or(Predicate::Other, |s| s.predicate)
}

/// A partially-seen `owl:Restriction` blank node.
#[derive(Default)]
struct Restriction {
    property: Option<String>,
    filler: Option<String>,
}

/// Everything collected in the single streaming pass, before blank nodes are
/// resolved against the restrictions they refer to.
#[derive(Default)]
struct Harvest {
    nodes: HashMap<TermId, OboNode>,
    restrictions: HashMap<String, Restriction>,
    blank_edges: Vec<(TermId, String)>,
}

impl Harvest {
    fn absorb(
        &mut self,
        subject: &NamedOrBlankNode,
        predicate: &str,
        object: &Term,
        reg: &NsRegistry,
    ) {
        match subject {
            NamedOrBlankNode::NamedNode(s) => {
                let Some(sid) = term_of(s.as_str(), reg) else {
                    return;
                };
                match (predicate, object) {
                    // `a owl:Class` is what creates a row — mere mention does not.
                    (RDF_TYPE, Term::NamedNode(o)) if o.as_str() == OWL_CLASS => {
                        self.nodes.entry(sid).or_default();
                    }
                    (RDFS_SUBCLASS_OF, Term::NamedNode(o)) => {
                        if let Some(parent) = term_of(o.as_str(), reg) {
                            self.nodes.entry(sid).or_default().is_a.push(parent);
                        }
                    }
                    // A restriction hangs off a blank node; resolve it later.
                    (RDFS_SUBCLASS_OF | OWL_EQUIVALENT_CLASS, Term::BlankNode(b)) => {
                        self.blank_edges.push((sid, b.as_str().to_string()));
                    }
                    (OWL_DEPRECATED, Term::Literal(l)) if l.value() == "true" => {
                        self.nodes.entry(sid).or_default().obsolete = true;
                    }
                    (OBO_IN_OWL_HAS_DB_XREF, Term::Literal(l)) => {
                        if let Some((src, id)) = l.value().split_once(':')
                            && !src.is_empty()
                            && !id.is_empty()
                        {
                            self.nodes.entry(sid).or_default().xref.push(Xref {
                                source: XrefSource::from_prefix(src),
                                id: id.to_string(),
                            });
                        }
                    }
                    _ => {}
                }
            }
            NamedOrBlankNode::BlankNode(b) => {
                let slot = self.restrictions.entry(b.as_str().to_string()).or_default();
                match (predicate, object) {
                    (OWL_ON_PROPERTY, Term::NamedNode(o)) => {
                        slot.property = Some(o.as_str().to_string());
                    }
                    (OWL_SOME_VALUES_FROM, Term::NamedNode(o)) => {
                        slot.filler = Some(o.as_str().to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    /// Join the blank-node edges to their restrictions. A blank node with no
    /// `onProperty`/`someValuesFrom` pair — an intersection list, a cardinality
    /// restriction, an anonymous union — yields nothing, per the documented
    /// limits.
    fn resolve(mut self, reg: &NsRegistry, table: &[PredSpec]) -> HashMap<TermId, OboNode> {
        for (sid, blank) in std::mem::take(&mut self.blank_edges) {
            let Some(r) = self.restrictions.get(&blank) else {
                continue;
            };
            let (Some(prop), Some(filler)) = (r.property.as_deref(), r.filler.as_deref()) else {
                continue;
            };
            let Some(object) = term_of(filler, reg) else {
                continue;
            };
            let p = predicate_of(prop, table);
            self.nodes.entry(sid).or_default().rel.push((p, object));
        }
        // Deterministic edge order regardless of triple arrival order — the
        // bake packs `is_a` then `rel` into value lanes positionally, so an
        // unstable order here would produce a different (still correct) row.
        for node in self.nodes.values_mut() {
            node.is_a.sort_unstable();
            node.is_a.dedup();
            node.rel.sort_unstable();
            node.rel.dedup();
        }
        self.nodes
    }
}

/// Parse an RDF document into the `(id → node)` map [`crate::bake`] consumes.
///
/// Streams the triples once, then joins the blank-node restrictions. Terms
/// whose prefix is outside `reg` are skipped, so an ontology's imports do not
/// leak rows into a bake that was not asked to tile them.
///
/// # Errors
///
/// Returns [`RdfErr`] if the underlying `oxttl` / `oxrdfxml` parser rejects
/// the document.
pub fn parse_rdf(
    bytes: &[u8],
    format: RdfFormat,
    reg: &NsRegistry,
    table: &[PredSpec],
) -> Result<HashMap<TermId, OboNode>, RdfErr> {
    let mut h = Harvest::default();
    match format {
        RdfFormat::Turtle => {
            for t in TurtleParser::new().for_slice(bytes) {
                let t = t.map_err(|e| RdfErr {
                    message: e.to_string(),
                })?;
                h.absorb(&t.subject, t.predicate.as_str(), &t.object, reg);
            }
        }
        RdfFormat::RdfXml => {
            for t in RdfXmlParser::new().for_slice(bytes) {
                let t = t.map_err(|e| RdfErr {
                    message: e.to_string(),
                })?;
                h.absorb(&t.subject, t.predicate.as_str(), &t.object, reg);
            }
        }
    }
    Ok(h.resolve(reg, table))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{NsRegistry, NsSpec};

    const SPINE: NsRegistry = NsRegistry::new(&[
        NsSpec {
            prefix: "BFO",
            concept_id: 0x0310,
        },
        NsSpec {
            prefix: "UBERON",
            concept_id: 0x0303,
        },
    ]);

    /// Turtle with: two classes, a named subClassOf, a blank-node part_of
    /// restriction, a deprecation, an xref, an out-of-registry parent, and a
    /// blank node that is NOT a well-formed restriction.
    const TTL: &str = r#"
@prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl:  <http://www.w3.org/2002/07/owl#> .
@prefix obo:  <http://purl.obolibrary.org/obo/> .
@prefix oio:  <http://www.geneontology.org/formats/oboInOwl#> .

obo:UBERON_0000955 a owl:Class ;
    rdfs:subClassOf obo:UBERON_0000062 ;
    rdfs:subClassOf obo:CHEBI_0000001 ;
    rdfs:subClassOf [ a owl:Restriction ;
        owl:onProperty obo:BFO_0000050 ;
        owl:someValuesFrom obo:UBERON_0000033 ] ;
    oio:hasDbXref "MeSH:D001921" .

obo:UBERON_0000062 a owl:Class .
obo:UBERON_0000033 a owl:Class .

obo:UBERON_0000099 a owl:Class ;
    owl:deprecated "true" .

obo:UBERON_0000077 a owl:Class ;
    rdfs:subClassOf [ a owl:Class ;
        owl:unionOf ( obo:UBERON_0000062 obo:UBERON_0000033 ) ] .
"#;

    fn u(num: u32) -> TermId {
        TermId { ns: 1, num }
    }

    #[test]
    fn purl_decoding_accepts_obo_iris_and_rejects_everything_else() {
        assert_eq!(
            purl_to_curie("http://purl.obolibrary.org/obo/HP_0000118"),
            Some(("HP", 118))
        );
        // The LAST underscore separates — some OBO locals carry one.
        assert_eq!(
            purl_to_curie("http://purl.obolibrary.org/obo/NCBITaxon_Union_0000001"),
            Some(("NCBITaxon_Union", 1))
        );
        // Rejects: wrong base, no separator, non-numeric, over 24 bits.
        assert_eq!(purl_to_curie("http://example.org/HP_0000118"), None);
        assert_eq!(
            purl_to_curie("http://purl.obolibrary.org/obo/HP0000118"),
            None
        );
        assert_eq!(purl_to_curie("http://purl.obolibrary.org/obo/HP_abc"), None);
        assert_eq!(
            purl_to_curie("http://purl.obolibrary.org/obo/HP_99999999"),
            None,
            "the 24-bit width contract must hold here as it does in TermId::parse"
        );
        assert_eq!(purl_to_curie("http://purl.obolibrary.org/obo/_1"), None);
    }

    #[test]
    fn the_registry_is_the_extraction_map() {
        // UBERON is in SPINE, HP is not. Same IRI shape, different outcome —
        // so the table is genuinely consulted rather than a prefix hardcoded.
        assert_eq!(
            term_of("http://purl.obolibrary.org/obo/UBERON_0000955", &SPINE),
            Some(u(955))
        );
        assert_eq!(
            term_of("http://purl.obolibrary.org/obo/HP_0000118", &SPINE),
            None
        );
        // …and the ordinal is table-relative: UBERON is row 1 here, row 2 in
        // the shipped core.
        assert_eq!(
            term_of("http://purl.obolibrary.org/obo/UBERON_0000955", &SPINE)
                .unwrap()
                .ns,
            1
        );
        assert_eq!(
            term_of(
                "http://purl.obolibrary.org/obo/UBERON_0000955",
                &crate::registry::OBO_CORE
            )
            .unwrap()
            .ns,
            2
        );
    }

    #[test]
    fn restriction_properties_map_through_the_table_and_fall_through_otherwise() {
        assert_eq!(
            predicate_of(
                "http://purl.obolibrary.org/obo/BFO_0000050",
                OBO_RESTRICTION_PREDICATES
            ),
            Predicate::PartOf
        );
        assert_eq!(
            predicate_of(
                "http://purl.obolibrary.org/obo/RO_0002200",
                OBO_RESTRICTION_PREDICATES
            ),
            Predicate::HasPhenotype
        );
        // An OBO property with no row → Other, never a guess.
        assert_eq!(
            predicate_of(
                "http://purl.obolibrary.org/obo/RO_0002131",
                OBO_RESTRICTION_PREDICATES
            ),
            Predicate::Other
        );
        // A non-OBO property IRI → Other.
        assert_eq!(
            predicate_of(
                "http://www.w3.org/2000/01/rdf-schema#seeAlso",
                OBO_RESTRICTION_PREDICATES
            ),
            Predicate::Other
        );
        // An EMPTY table must classify everything as Other — proof the table
        // is read rather than the mapping being baked into the function.
        assert_eq!(
            predicate_of("http://purl.obolibrary.org/obo/BFO_0000050", &[]),
            Predicate::Other
        );
    }

    #[test]
    fn turtle_harvest_keeps_edges_and_drops_what_it_documents_dropping() {
        let nodes = parse_rdf(
            TTL.as_bytes(),
            RdfFormat::Turtle,
            &SPINE,
            OBO_RESTRICTION_PREDICATES,
        )
        .expect("turtle parses");

        // Four `a owl:Class` assertions → four rows. UBERON_0000033 and
        // _0000062 are declared classes AND referenced; CHEBI is referenced
        // but outside the registry, so it must NOT create a row.
        assert_eq!(nodes.len(), 5, "one row per `a owl:Class`, none for CHEBI");

        let brain = &nodes[&u(955)];
        // The named parent is kept; the out-of-registry one is skipped.
        assert_eq!(brain.is_a, vec![u(62)], "CHEBI parent must be skipped");
        // The blank-node restriction resolved to a typed edge.
        assert_eq!(brain.rel, vec![(Predicate::PartOf, u(33))]);
        // The xref survived.
        assert_eq!(brain.xref.len(), 1);
        assert_eq!(brain.xref[0].source, XrefSource::Mesh);
        assert_eq!(brain.xref[0].id, "D001921");

        assert!(nodes[&u(99)].obsolete, "owl:deprecated true is read");
        assert!(!brain.obsolete);

        // The documented limit, asserted rather than hoped: a blank node that
        // is a unionOf — not an onProperty/someValuesFrom restriction —
        // contributes NO edge.
        assert!(
            nodes[&u(77)].rel.is_empty() && nodes[&u(77)].is_a.is_empty(),
            "a non-restriction blank node must yield nothing, not a wrong edge"
        );
    }

    /// The property this module exists for: identity comes from the term, so
    /// the harvest cannot depend on document order.
    ///
    /// **What would make it fail:** an intern-counter identity, which is what
    /// `lance-graph-ontology`'s `OwlHydrator` assigns. Reversing the stanzas
    /// would then renumber every term.
    #[test]
    fn identity_is_the_curie_numeric_so_document_order_cannot_change_it() {
        let forward = parse_rdf(
            TTL.as_bytes(),
            RdfFormat::Turtle,
            &SPINE,
            OBO_RESTRICTION_PREDICATES,
        )
        .unwrap();

        // Re-emit the same graph with the class stanzas in reverse order.
        let mut stanzas: Vec<&str> = TTL.split("\n\n").collect();
        let header = stanzas.remove(0);
        stanzas.reverse();
        let reordered = format!("{header}\n\n{}", stanzas.join("\n\n"));
        let backward = parse_rdf(
            reordered.as_bytes(),
            RdfFormat::Turtle,
            &SPINE,
            OBO_RESTRICTION_PREDICATES,
        )
        .unwrap();

        assert_eq!(forward.len(), backward.len());
        for (id, node) in &forward {
            let other = backward.get(id).expect("same ids under any order");
            assert_eq!(node.is_a, other.is_a, "{id:?} parents must match");
            assert_eq!(node.rel, other.rel, "{id:?} typed edges must match");
            assert_eq!(node.obsolete, other.obsolete);
        }
        // The reorder must have actually changed the bytes, or this proves
        // nothing about order-independence.
        assert_ne!(reordered.as_str(), TTL);
    }

    #[test]
    fn rdfxml_reads_the_same_graph_as_turtle() {
        const XML: &str = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
         xmlns:owl="http://www.w3.org/2002/07/owl#">
  <owl:Class rdf:about="http://purl.obolibrary.org/obo/UBERON_0000955">
    <rdfs:subClassOf rdf:resource="http://purl.obolibrary.org/obo/UBERON_0000062"/>
    <rdfs:subClassOf>
      <owl:Restriction>
        <owl:onProperty rdf:resource="http://purl.obolibrary.org/obo/BFO_0000050"/>
        <owl:someValuesFrom rdf:resource="http://purl.obolibrary.org/obo/UBERON_0000033"/>
      </owl:Restriction>
    </rdfs:subClassOf>
  </owl:Class>
</rdf:RDF>
"#;
        let nodes = parse_rdf(
            XML.as_bytes(),
            RdfFormat::RdfXml,
            &SPINE,
            OBO_RESTRICTION_PREDICATES,
        )
        .expect("rdf/xml parses");
        let brain = &nodes[&u(955)];
        assert_eq!(brain.is_a, vec![u(62)]);
        assert_eq!(brain.rel, vec![(Predicate::PartOf, u(33))]);
    }

    #[test]
    fn a_malformed_document_errors_rather_than_returning_an_empty_graph() {
        let e = parse_rdf(
            b"this is not turtle @@@",
            RdfFormat::Turtle,
            &SPINE,
            OBO_RESTRICTION_PREDICATES,
        );
        assert!(
            e.is_err(),
            "a parse failure must not look like an empty ontology"
        );
    }

    #[test]
    fn format_detection_prefers_the_extension_then_sniffs() {
        assert_eq!(detect_format(Some("ttl"), b"<?xml"), RdfFormat::Turtle);
        assert_eq!(detect_format(Some("RDF"), b"@prefix"), RdfFormat::RdfXml);
        // `.owl` is ambiguous in the wild → sniff decides, both ways.
        assert_eq!(
            detect_format(Some("owl"), b"<?xml version"),
            RdfFormat::RdfXml
        );
        assert_eq!(
            detect_format(Some("owl"), b"@prefix owl:"),
            RdfFormat::Turtle
        );
        assert_eq!(detect_format(None, b"  \n<rdf:RDF "), RdfFormat::RdfXml);
        assert_eq!(detect_format(None, b""), RdfFormat::Turtle);
    }
}
