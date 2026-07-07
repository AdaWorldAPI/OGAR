//! `ogar-proposal` — the producer mapping from OGAR IR to
//! `lance-graph-ontology::MappingProposal`.
//!
//! Per `docs/LANCE-GRAPH-INTEGRATION.md`: OGAR is a `SchemaSource`
//! producer into the existing upstream `OntologyRegistry`. This crate
//! holds the mapping logic.
//!
//! # The `&'static str` impedance
//!
//! The upstream `lance-graph-contract` types are **const-leaning**:
//! `Schema.name`, `PropertySpec.predicate`, and `LinkSpec.{subject,
//! predicate, object}_type` are all `&'static str` (designed for
//! compile-time `const` schemas, like `odoo_blueprint`'s
//! `const ENTITIES: &[OdooEntity]`). OGAR produces schemas at **runtime**
//! from parsed source.
//!
//! This crate resolves the impedance with an **owned mirror**: the
//! `ProposalDraft` / `SchemaDraft` / `PropertyDraft` types carry `String`
//! where the contract carries `&'static str`. The mapping
//! [`class_to_drafts`] is fully testable in-repo, with **zero dependency
//! on the upstream crate** (which has a heavy build graph: protoc, oxttl,
//! the BLOCKED kv-lance surreal deps).
//!
//! The actual `impl SchemaSource` is a thin boundary that interns the
//! owned strings into `&'static str` via [`intern`] (a `Box::leak` of a
//! deduplicated string set — justified because ontology terms are
//! bounded and live for the process lifetime anyway). That boundary
//! lands in Sprint 5b behind a `lance-bind` feature once the cross-repo
//! dependency + protoc are wired. See [`boundary`] for the sketch.
//!
//! # Example
//!
//! ```
//! use ogar_proposal::{class_to_drafts, DraftKind};
//! use ogar_vocab::{Class, Association, AssociationKind};
//!
//! let mut wp = Class::new("WorkPackage");
//! wp.declared_in_module = Some("open_project".into());
//! let mut assoc = Association::new(AssociationKind::BelongsTo, "project");
//! assoc.class_name = Some("Project".into());
//! wp.associations.push(assoc);
//!
//! let drafts = class_to_drafts(&wp, "ogit-op");
//! // one Entity draft for the class + one Edge draft for the association
//! assert!(drafts.iter().any(|d| matches!(d.kind, DraftKind::Entity { .. })));
//! assert!(drafts.iter().any(|d| matches!(d.kind, DraftKind::Edge { .. })));
//! ```

#![warn(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use ogar_vocab::{Association, AssociationKind, Attribute, Class};

// ─────────────────────────────────────────────────────────────────────
// Owned mirror types (String where the contract uses &'static str)
// ─────────────────────────────────────────────────────────────────────

/// Owned mirror of `lance-graph-ontology::MappingProposal`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ProposalDraft {
    /// Producer-facing public name (the OGAR class / relation name).
    pub public_name: String,
    /// Bridge id — the application prefix (`ogit-op`, `ogit-erp`).
    pub bridge_id: String,
    /// Namespace segment for the OGIT URI.
    pub namespace: String,
    /// What this proposal carries.
    pub kind: DraftKind,
    /// GDPR / governance marking.
    pub marking: MarkingDraft,
    /// Confidence: 1.0 canonical, <1.0 for heuristic-derived semantics.
    pub confidence: f32,
    /// Audit: the source file / module path.
    pub source_uri: String,
    /// Who/what produced this. `"ogar_ruby_producer_v1"` etc.
    pub created_by: String,
}

/// Owned mirror of `MappingProposalKind`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DraftKind {
    /// An entity (OGAR `Class`) → contract `Schema`.
    Entity {
        /// The schema draft.
        schema: SchemaDraft,
    },
    /// An edge (OGAR `Association`) → contract `LinkSpec`.
    Edge {
        /// The link draft.
        link: LinkDraft,
    },
    /// A single attribute → contract `Attribute` proposal.
    Attribute {
        /// Predicate name.
        predicate: String,
        /// Semantic type.
        semantic: SemanticDraft,
    },
}

/// Owned mirror of contract `Schema`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SchemaDraft {
    /// Schema name (the class identity).
    pub name: String,
    /// Property specs for the class's attributes.
    pub properties: Vec<PropertyDraft>,
}

/// Owned mirror of contract `PropertySpec`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PropertyDraft {
    /// Predicate name (the attribute name).
    pub predicate: String,
    /// Required / Optional / Free.
    pub kind: PropertyKindDraft,
    /// Codec route hint.
    pub codec_route: CodecRouteDraft,
    /// Semantic type.
    pub semantic: SemanticDraft,
    /// Governance marking.
    pub marking: MarkingDraft,
}

/// Owned mirror of contract `LinkSpec`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LinkDraft {
    /// Subject type — the owning class identity.
    pub subject_type: String,
    /// Predicate — the relation name.
    pub predicate: String,
    /// Object type — the target class identity (or `"<polymorphic>"`).
    pub object_type: String,
    /// Cardinality.
    pub cardinality: CardinalityDraft,
}

/// Owned mirror of contract `PropertyKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PropertyKindDraft {
    /// MUST exist (Rails `presence: true`, Odoo `required=True`).
    Required,
    /// MAY exist.
    Optional,
    /// Schema-free (store_accessor pseudo-fields, custom fields).
    Free,
}

/// Owned mirror of contract `CodecRoute`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CodecRouteDraft {
    /// Lossless / exact-match (Required props, identity fields).
    Passthrough,
    /// Similarity / argmax (free text, descriptions).
    CamPq,
}

/// Owned mirror of contract `Cardinality`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CardinalityDraft {
    /// 1:1 (`has_one`, `belongs_to` from the subject's single-object view).
    OneToOne,
    /// 1:N (`has_many`).
    OneToMany,
    /// M:N (`has_and_belongs_to_many`, `many_to_many`, `has_many :through`).
    ManyToMany,
}

/// Owned mirror of contract `Marking`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MarkingDraft {
    /// No restriction.
    Public,
    /// Internal use only (the GDPR-safe default).
    Internal,
    /// Personally Identifiable Information.
    Pii,
    /// Financial / bookkeeping data.
    Financial,
    /// Highest restriction.
    Restricted,
}

/// Owned mirror of contract `SemanticType` (the variants OGAR can
/// currently infer; the contract has more that producers may set
/// explicitly).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SemanticDraft {
    /// Opaque text (the default).
    PlainText,
    /// IBAN.
    Iban,
    /// Currency with ISO 4217 code.
    Currency(String),
    /// Email address.
    Email,
    /// Phone number.
    Phone,
    /// Date (day precision by default).
    Date,
    /// Datetime.
    DateTime,
    /// Postal address.
    Address,
    /// URL.
    Url,
    /// Tax identification number.
    TaxId,
}

// ─────────────────────────────────────────────────────────────────────
// The mapping: ogar::Class → Vec<ProposalDraft>
// ─────────────────────────────────────────────────────────────────────

/// Map an OGAR `Class` to its proposal drafts: one `Entity` draft for
/// the class (with all attributes as `PropertyDraft`s) plus one `Edge`
/// draft per association.
///
/// `bridge_id` is the application prefix (`ogit-op`, `ogit-erp`). The
/// namespace is derived from `declared_in_module` when present, else the
/// class name.
#[must_use]
pub fn class_to_drafts(class: &Class, bridge_id: &str) -> Vec<ProposalDraft> {
    let namespace = class
        .declared_in_module
        .clone()
        .unwrap_or_else(|| class.name.clone());
    let class_identity = format!("{bridge_id}/{}", class.name);
    let source_uri = class
        .declared_in_module
        .as_deref()
        .map(|m| format!("module:{m}"))
        .unwrap_or_else(|| "unknown".into());
    let created_by = format!("ogar_producer_{}", language_tag(class.language));

    // Confidence: any heuristic-derived semantic on a property pulls the
    // entity proposal's confidence below 1.0 so reviewers can spot
    // guessed annotations. Pure-structural mappings stay at 1.0.
    let properties: Vec<PropertyDraft> =
        class.attributes.iter().map(attribute_to_property).collect();
    let any_heuristic = properties
        .iter()
        .any(|p| !matches!(p.semantic, SemanticDraft::PlainText));
    let entity_confidence = if any_heuristic { 0.9 } else { 1.0 };

    let mut drafts = Vec::with_capacity(1 + class.associations.len());

    drafts.push(ProposalDraft {
        public_name: class.name.clone(),
        bridge_id: bridge_id.to_string(),
        namespace: namespace.clone(),
        kind: DraftKind::Entity {
            schema: SchemaDraft {
                name: class_identity.clone(),
                properties,
            },
        },
        marking: MarkingDraft::Internal,
        confidence: entity_confidence,
        source_uri: source_uri.clone(),
        created_by: created_by.clone(),
    });

    for assoc in &class.associations {
        drafts.push(ProposalDraft {
            public_name: assoc.name.clone(),
            bridge_id: bridge_id.to_string(),
            namespace: namespace.clone(),
            kind: DraftKind::Edge {
                link: association_to_link(assoc, &class_identity, bridge_id),
            },
            marking: MarkingDraft::Internal,
            confidence: 1.0,
            source_uri: source_uri.clone(),
            created_by: created_by.clone(),
        });
    }

    drafts
}

/// Map an OGAR `Attribute` to a contract-shaped `PropertyDraft`.
fn attribute_to_property(attr: &Attribute) -> PropertyDraft {
    let required = attr.options.required.unwrap_or(false);
    let kind = if required {
        PropertyKindDraft::Required
    } else {
        PropertyKindDraft::Optional
    };
    let semantic = infer_semantic(attr);
    let marking = infer_marking(attr, &semantic);
    // Required + identity-ish fields are lossless (Passthrough); free
    // text leans CamPq for similarity. Heuristic: Required → Passthrough.
    let codec_route = if required {
        CodecRouteDraft::Passthrough
    } else {
        CodecRouteDraft::CamPq
    };
    PropertyDraft {
        predicate: attr.name.clone(),
        kind,
        codec_route,
        semantic,
        marking,
    }
}

/// Map an OGAR `Association` to a contract-shaped `LinkDraft`.
fn association_to_link(assoc: &Association, subject_identity: &str, bridge_id: &str) -> LinkDraft {
    let object_type = if assoc.polymorphic == Some(true) {
        "<polymorphic>".to_string()
    } else if let Some(ref cn) = assoc.class_name {
        format!("{bridge_id}/{cn}")
    } else {
        // Infer the target from the relation name (Rails convention:
        // singularize + classify). We keep it simple: capitalize.
        format!("{bridge_id}/{}", classify_relation(&assoc.name))
    };
    LinkDraft {
        subject_type: subject_identity.to_string(),
        predicate: assoc.name.clone(),
        object_type,
        cardinality: cardinality_of(assoc.kind),
    }
}

/// Cardinality mapping. NOTE: `BelongsTo` is N:1, which the contract's
/// `Cardinality` (OneToOne / OneToMany / ManyToMany) cannot express
/// directly. We map it to `OneToOne` — each subject relates to a single
/// object; the "many subjects" side is captured by the inverse
/// `has_many` association the ORM declares separately on the target.
fn cardinality_of(kind: AssociationKind) -> CardinalityDraft {
    match kind {
        AssociationKind::BelongsTo => CardinalityDraft::OneToOne,
        AssociationKind::HasOne => CardinalityDraft::OneToOne,
        AssociationKind::HasMany => CardinalityDraft::OneToMany,
        AssociationKind::HasAndBelongsToMany => CardinalityDraft::ManyToMany,
        _ => CardinalityDraft::OneToOne,
    }
}

/// Infer a `SemanticDraft` for an attribute. Conservative: defaults to
/// `PlainText`, upgrading only on clear ORM-type or field-name signals.
/// Heuristic upgrades carry the entity confidence below 1.0 (see
/// [`class_to_drafts`]) so reviewers can audit guessed semantics.
fn infer_semantic(attr: &Attribute) -> SemanticDraft {
    let name = attr.name.to_lowercase();
    let type_name = attr.type_name.as_deref().unwrap_or("").to_lowercase();

    // ORM-type-driven (high confidence — the type IS the semantic).
    match type_name.as_str() {
        "monetary" => {
            let code = attr
                .options
                .currency_field
                .as_ref()
                .map(|_| "EUR".to_string()) // currency code resolved downstream from the linked field
                .unwrap_or_else(|| "EUR".to_string());
            return SemanticDraft::Currency(code);
        }
        "date" => return SemanticDraft::Date,
        "datetime" => return SemanticDraft::DateTime,
        _ => {}
    }

    // Field-name heuristics (lower confidence — a guess).
    if name == "iban" || name.ends_with("_iban") {
        SemanticDraft::Iban
    } else if name == "email" || name.ends_with("_email") || name.ends_with("mail") {
        SemanticDraft::Email
    } else if name == "phone" || name.ends_with("_phone") || name == "tel" || name == "telephone" {
        SemanticDraft::Phone
    } else if name == "url" || name.ends_with("_url") || name == "website" {
        SemanticDraft::Url
    } else if (name.contains("tax") && name.contains("id")) || name == "ustid" || name == "vat" {
        SemanticDraft::TaxId
    } else if name == "address" || name.ends_with("_address") {
        SemanticDraft::Address
    } else {
        SemanticDraft::PlainText
    }
}

/// Infer a governance marking from the attribute + its semantic type.
/// PII-ish semantics → `Pii`; money → `Financial`; otherwise default
/// `Internal`. `company_dependent` does not itself imply a marking.
fn infer_marking(attr: &Attribute, semantic: &SemanticDraft) -> MarkingDraft {
    match semantic {
        SemanticDraft::Iban
        | SemanticDraft::Email
        | SemanticDraft::Phone
        | SemanticDraft::Address => {
            return MarkingDraft::Pii;
        }
        SemanticDraft::TaxId => return MarkingDraft::Restricted,
        SemanticDraft::Currency(_) => return MarkingDraft::Financial,
        _ => {}
    }
    let name = attr.name.to_lowercase();
    if name.contains("amount")
        || name.contains("price")
        || name.contains("balance")
        || name.contains("total")
        || name.contains("salary")
    {
        MarkingDraft::Financial
    } else if name.contains("birth") || name.contains("ssn") || name.contains("passport") {
        MarkingDraft::Pii
    } else {
        MarkingDraft::Internal
    }
}

/// Rails-style classify: singularize-ish + capitalize the first letter.
/// Intentionally minimal — `class_name` override is the precise path.
fn classify_relation(relation: &str) -> String {
    let singular = relation.strip_suffix('s').unwrap_or(relation);
    let mut chars = singular.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn language_tag(lang: ogar_vocab::Language) -> &'static str {
    use ogar_vocab::Language;
    match lang {
        Language::Ruby => "ruby_v1",
        Language::Python => "python_v1",
        Language::Sql => "sql_v1",
        Language::TypeScript => "typescript_v1",
        Language::SurrealQl => "surrealql_v1",
        Language::Elixir => "elixir_v1",
        _ => "unknown_v1",
    }
}

/// Compute a stable content checksum for idempotent re-hydration.
/// Uses the standard-library hasher (no `sha2` dependency in this
/// crate); the real `impl SchemaSource` boundary may recompute a
/// SHA256 if the registry requires that exact algorithm.
#[must_use]
pub fn draft_checksum(draft: &ProposalDraft) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    draft.public_name.hash(&mut h);
    draft.bridge_id.hash(&mut h);
    draft.namespace.hash(&mut h);
    draft.source_uri.hash(&mut h);
    // The kind discriminant + its key fields.
    match &draft.kind {
        DraftKind::Entity { schema } => {
            "entity".hash(&mut h);
            schema.name.hash(&mut h);
            for p in &schema.properties {
                p.predicate.hash(&mut h);
            }
        }
        DraftKind::Edge { link } => {
            "edge".hash(&mut h);
            link.subject_type.hash(&mut h);
            link.predicate.hash(&mut h);
            link.object_type.hash(&mut h);
        }
        DraftKind::Attribute { predicate, .. } => {
            "attribute".hash(&mut h);
            predicate.hash(&mut h);
        }
    }
    format!("{:016x}", h.finish())
}

/// The cross-repo boundary sketch (Sprint 5b, behind a `lance-bind`
/// feature). Documents how an owned `ProposalDraft` interns into a real
/// `lance_graph_ontology::MappingProposal`. Not compiled here — the
/// upstream crate has a heavy build graph (protoc, oxttl, BLOCKED
/// kv-lance). When wired, the interning is a single `Box::leak` of a
/// deduplicated string set, justified because ontology terms are bounded
/// and live for the process lifetime.
///
/// ```ignore
/// // cfg(feature = "lance-bind")
/// use lance_graph_ontology::{MappingProposal, MappingProposalKind};
/// use lance_graph_contract::property::{Schema, PropertySpec, SemanticType, Marking};
///
/// fn intern(s: &str) -> &'static str {
///     // dedup via a process-global HashSet<&'static str>, Box::leak on miss
///     INTERN.intern(s)
/// }
///
/// pub fn draft_to_proposal(d: &ProposalDraft) -> MappingProposal {
///     let kind = match &d.kind {
///         DraftKind::Entity { schema } => {
///             let mut b = Schema::builder(intern(&schema.name));
///             for p in &schema.properties {
///                 b = b.property(PropertySpec { predicate: intern(&p.predicate), .. });
///             }
///             MappingProposalKind::Entity { schema: b.build() }
///         }
///         // Edge / Attribute analogously
///         _ => unimplemented!(),
///     };
///     MappingProposal { public_name: d.public_name.clone(), kind, .. }
/// }
/// ```
pub mod boundary {}

#[cfg(test)]
mod tests {
    use super::*;
    use ogar_vocab::{Association, AssociationKind, Attribute, Class};

    fn work_package() -> Class {
        let mut wp = Class::new("WorkPackage");
        wp.declared_in_module = Some("open_project".into());
        let mut subject = Attribute::new("subject");
        subject.type_name = Some("Char".into());
        subject.options.required = Some(true);
        wp.attributes.push(subject);
        let mut email = Attribute::new("author_email");
        email.type_name = Some("Char".into());
        wp.attributes.push(email);
        let mut total = Attribute::new("amount_total");
        total.type_name = Some("Monetary".into());
        total.options.currency_field = Some("currency_id".into());
        wp.attributes.push(total);
        let mut proj = Association::new(AssociationKind::BelongsTo, "project");
        proj.class_name = Some("Project".into());
        wp.associations.push(proj);
        let items = Association::new(AssociationKind::HasMany, "time_entries");
        wp.associations.push(items);
        wp
    }

    #[test]
    fn class_yields_one_entity_plus_one_edge_per_assoc() {
        let drafts = class_to_drafts(&work_package(), "ogit-op");
        let entities = drafts
            .iter()
            .filter(|d| matches!(d.kind, DraftKind::Entity { .. }))
            .count();
        let edges = drafts
            .iter()
            .filter(|d| matches!(d.kind, DraftKind::Edge { .. }))
            .count();
        assert_eq!(entities, 1);
        assert_eq!(edges, 2);
    }

    #[test]
    fn entity_schema_name_is_class_identity() {
        let drafts = class_to_drafts(&work_package(), "ogit-op");
        let entity = drafts
            .iter()
            .find_map(|d| match &d.kind {
                DraftKind::Entity { schema } => Some(schema),
                _ => None,
            })
            .unwrap();
        assert_eq!(entity.name, "ogit-op/WorkPackage");
        assert_eq!(entity.properties.len(), 3);
    }

    #[test]
    fn required_attribute_maps_to_required_passthrough() {
        let drafts = class_to_drafts(&work_package(), "ogit-op");
        let entity = drafts
            .iter()
            .find_map(|d| match &d.kind {
                DraftKind::Entity { schema } => Some(schema),
                _ => None,
            })
            .unwrap();
        let subject = entity
            .properties
            .iter()
            .find(|p| p.predicate == "subject")
            .unwrap();
        assert_eq!(subject.kind, PropertyKindDraft::Required);
        assert_eq!(subject.codec_route, CodecRouteDraft::Passthrough);
    }

    #[test]
    fn email_field_infers_pii_marking() {
        let drafts = class_to_drafts(&work_package(), "ogit-op");
        let entity = drafts
            .iter()
            .find_map(|d| match &d.kind {
                DraftKind::Entity { schema } => Some(schema),
                _ => None,
            })
            .unwrap();
        let email = entity
            .properties
            .iter()
            .find(|p| p.predicate == "author_email")
            .unwrap();
        assert_eq!(email.semantic, SemanticDraft::Email);
        assert_eq!(email.marking, MarkingDraft::Pii);
    }

    #[test]
    fn monetary_field_infers_currency_and_financial() {
        let drafts = class_to_drafts(&work_package(), "ogit-op");
        let entity = drafts
            .iter()
            .find_map(|d| match &d.kind {
                DraftKind::Entity { schema } => Some(schema),
                _ => None,
            })
            .unwrap();
        let total = entity
            .properties
            .iter()
            .find(|p| p.predicate == "amount_total")
            .unwrap();
        assert_eq!(total.semantic, SemanticDraft::Currency("EUR".into()));
        assert_eq!(total.marking, MarkingDraft::Financial);
    }

    #[test]
    fn heuristic_semantic_lowers_entity_confidence() {
        // work_package has email + monetary → heuristic semantics present.
        let drafts = class_to_drafts(&work_package(), "ogit-op");
        let entity = drafts
            .iter()
            .find(|d| matches!(d.kind, DraftKind::Entity { .. }))
            .unwrap();
        assert!(entity.confidence < 1.0);

        // A plain class with only PlainText attributes stays at 1.0.
        let mut plain = Class::new("Tag");
        let mut n = Attribute::new("label");
        n.type_name = Some("Char".into());
        plain.attributes.push(n);
        let plain_drafts = class_to_drafts(&plain, "ogit-op");
        let plain_entity = plain_drafts
            .iter()
            .find(|d| matches!(d.kind, DraftKind::Entity { .. }))
            .unwrap();
        assert_eq!(plain_entity.confidence, 1.0);
    }

    #[test]
    fn belongs_to_maps_one_to_one_with_explicit_target() {
        let drafts = class_to_drafts(&work_package(), "ogit-op");
        let link = drafts
            .iter()
            .find_map(|d| match &d.kind {
                DraftKind::Edge { link } if link.predicate == "project" => Some(link),
                _ => None,
            })
            .unwrap();
        assert_eq!(link.cardinality, CardinalityDraft::OneToOne);
        assert_eq!(link.subject_type, "ogit-op/WorkPackage");
        assert_eq!(link.object_type, "ogit-op/Project");
    }

    #[test]
    fn has_many_maps_one_to_many_with_classified_target() {
        let drafts = class_to_drafts(&work_package(), "ogit-op");
        let link = drafts
            .iter()
            .find_map(|d| match &d.kind {
                DraftKind::Edge { link } if link.predicate == "time_entries" => Some(link),
                _ => None,
            })
            .unwrap();
        assert_eq!(link.cardinality, CardinalityDraft::OneToMany);
        // No class_name override → naive classify (singularize 's' +
        // capitalize). "time_entries" → "Time_entrie" — deliberately
        // crude; the precise target comes from `class_name:` (Rails) or
        // `comodel_name` (Odoo). This asserts the documented fallback so
        // a future producer that improves classify updates it knowingly.
        assert_eq!(link.object_type, "ogit-op/Time_entrie");
    }

    #[test]
    fn polymorphic_belongs_to_has_no_concrete_target() {
        let mut c = Class::new("Comment");
        let mut a = Association::new(AssociationKind::BelongsTo, "commentable");
        a.polymorphic = Some(true);
        c.associations.push(a);
        let drafts = class_to_drafts(&c, "ogit-op");
        let link = drafts
            .iter()
            .find_map(|d| match &d.kind {
                DraftKind::Edge { link } => Some(link),
                _ => None,
            })
            .unwrap();
        assert_eq!(link.object_type, "<polymorphic>");
    }

    #[test]
    fn namespace_comes_from_declared_module() {
        let drafts = class_to_drafts(&work_package(), "ogit-op");
        assert!(drafts.iter().all(|d| d.namespace == "open_project"));
    }

    #[test]
    fn checksum_is_stable_and_distinguishes_drafts() {
        let drafts = class_to_drafts(&work_package(), "ogit-op");
        let c0 = draft_checksum(&drafts[0]);
        let c0_again = draft_checksum(&drafts[0]);
        assert_eq!(c0, c0_again, "checksum must be deterministic");
        let c1 = draft_checksum(&drafts[1]);
        assert_ne!(c0, c1, "different drafts must differ");
    }

    #[test]
    fn created_by_carries_language_tag() {
        let drafts = class_to_drafts(&work_package(), "ogit-op");
        assert!(
            drafts
                .iter()
                .all(|d| d.created_by == "ogar_producer_ruby_v1")
        );
    }
}
