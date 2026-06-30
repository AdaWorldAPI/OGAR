//! Per-class minting — the OGAR transpile substrate's addressing step.
//!
//! [`lift_model_graph_python`](crate::lift_model_graph_python) turns a
//! [`ModelGraph`] into the schema-carrying [`Class`]es. This module adds the
//! other half: the 16-byte **rail facet** for each class, with its render
//! classid resolved through a language/app [`PortSpec`]
//! (`OdooPort` / `OpenProjectPort` / …). The pair is the unit a consumer
//! pulls back — language-agnostic compiled business logic, addressed by
//! classid:
//!
//! ```text
//!   ModelGraph                         (any ogar-from-<lang> frontend)
//!     │  lift_model_graph_*            -> Vec<Class>     (the schema)
//!     │  expand + mint_with_classid    -> Mint           (the rail facets)
//!     ▼
//!   Vec<CompiledClass { class, facet }>
//!            ▲
//!     a consumer (odoo-rs, medcare-rs, …) pulls these via its thin wrapper
//!     contract (lance-graph-contract is Rust's) — the "impossible" 15 %
//!     (intrusive / stateful logic) stays a per-language adapter + ClassView.
//! ```
//!
//! The classid is the cross-app join key: the **low u16** is the shared
//! concept ([`PortSpec::class_id`] → the OGAR codebook), the **high u16** is
//! the app render prefix ([`PortSpec::APP_PREFIX`]). So `account.move` lands
//! as `0x0002_0202` under [`OdooPort`](ogar_vocab::ports::OdooPort) and an
//! OpenProject `WorkPackage` lands as `0x0001_0102` under
//! [`OpenProjectPort`](ogar_vocab::ports::OpenProjectPort) — different render
//! skins, the same conceptual identity where they converge.

use ogar_vocab::app::render_classid_for;
use ogar_vocab::ports::PortSpec;
use ogar_vocab::Class;
use ruff_spo_address::{mint_with_classid, Facet, Mint};
use ruff_spo_triplet::{expand, ModelGraph};

use crate::{lift_model_graph, lift_model_graph_python};

/// A class compiled to its rail-shaped, language-agnostic form: the lifted
/// schema ([`Class`]) plus its 16-byte address ([`Facet`]). This is what a
/// consumer pulls from the OGAR substrate and renders through its own
/// `ClassView` — the schema says *what the class is*, the facet says *where
/// it lives* (classid + the `part_of` / `is_a` cascade).
#[derive(Debug, Clone)]
pub struct CompiledClass {
    /// The lifted schema — attributes / associations / computed fields.
    pub class: Class,
    /// The 16-byte rail facet: render classid + the `part_of` / `is_a`
    /// tier chains.
    pub facet: Facet,
}

/// Mint the rail facet for every node in `graph`, resolving each node's
/// render classid through port `P`. Frontend-agnostic: any
/// `ogar-from-<lang>` produces the same [`ModelGraph`], so the mint is
/// shared. The returned [`Mint`] addresses every node (model *and* member)
/// — look one up with [`Mint::facet`].
#[must_use]
pub fn mint_graph<P: PortSpec>(graph: &ModelGraph) -> Mint {
    let triples = expand(graph);
    mint_with_classid(&triples, |node| classid_for_node::<P>(node))
}

/// Compile a Python/Odoo [`ModelGraph`] into rail-shaped [`CompiledClass`]es:
/// lift each model's schema and pair it with its minted facet, classid via
/// port `P`. Declaration order is preserved (mirrors
/// [`lift_model_graph_python`]).
#[must_use]
pub fn compile_graph_python<P: PortSpec>(graph: &ModelGraph) -> Vec<CompiledClass> {
    let mint = mint_graph::<P>(graph);
    lift_model_graph_python(graph)
        .into_iter()
        .map(|class| {
            let node = format!("{}:{}", graph.namespace, class.name);
            // A model always appears as a subject node (`rdf:type ObjectType`),
            // so it mints; the fallback covers a degenerate graph by stamping
            // the classid with empty tier chains (still a valid rail address).
            let facet = mint
                .facet(&node)
                .unwrap_or_else(|| Facet::from_parts(classid_for_node::<P>(&node), [0; 6], [0; 6]));
            CompiledClass { class, facet }
        })
        .collect()
}

/// Compile a Ruby/Rails [`ModelGraph`] into rail-shaped [`CompiledClass`]es:
/// lift each model's schema ([`Language::Ruby`](ogar_vocab::Language), via
/// [`lift_model_graph`] — **not** [`lift_model_graph_python`], which would
/// mis-stamp the producer language) and pair it with its minted facet,
/// classid via port `P`. Declaration order is preserved (mirrors
/// [`lift_model_graph`]). The Rails counterpart of
/// [`compile_graph_python`] — same mint, same assembly shape, the only
/// difference is which lift stamps the [`Class::language`](ogar_vocab::Class)
/// field. E.g. `openproject:WorkPackage` → `0x0001_0102`,
/// `redmine:Issue` → `0x0007_0102` (same shared concept, different render
/// prefix — the two-render-skins-one-concept convergence).
#[must_use]
pub fn compile_graph_ruby<P: PortSpec>(graph: &ModelGraph) -> Vec<CompiledClass> {
    let mint = mint_graph::<P>(graph);
    lift_model_graph(graph)
        .into_iter()
        .map(|class| {
            let node = format!("{}:{}", graph.namespace, class.name);
            let facet = mint
                .facet(&node)
                .unwrap_or_else(|| Facet::from_parts(classid_for_node::<P>(&node), [0; 6], [0; 6]));
            CompiledClass { class, facet }
        })
        .collect()
}

/// Resolve a node IRI's full render classid via port `P`.
///
/// The IRI is `<ns>:<model>` or `<ns>:<model>.<member>`; members inherit
/// their class's id, so we resolve the *model*'s concept and compose the
/// render classid `(APP_PREFIX << 16) | concept`. Odoo models arrive
/// underscore-normalised in the IRI (`account_move`) while ports key on the
/// dotted name (`account.move`), so we try the dotted form first, then the
/// raw form (mirrors `od-ontology`'s `concept_classid`). An unmapped model
/// resolves to `0` — the bootstrap address, left for the registry /
/// ref-escape path, never silently mis-stamped.
fn classid_for_node<P: PortSpec>(node: &str) -> u32 {
    let model = model_of(node);
    P::class_id(&model.replace('_', "."))
        .or_else(|| P::class_id(model))
        .map_or(0, render_classid_for::<P>)
}

/// The model segment of a node IRI: strip the `<ns>:` prefix, take up to the
/// first `.` (the model↔member separator).
fn model_of(node: &str) -> &str {
    let body = node.split_once(':').map_or(node, |(_, rest)| rest);
    body.split_once('.').map_or(body, |(model, _)| model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ogar_vocab::ports::{OdooPort, OpenProjectPort, RedminePort};
    use ogar_vocab::Language;
    use ruff_spo_triplet::{AssocDecl, AssocKind, Field, Function, Model};

    // A representative `account.move` `ModelGraph`, constructed directly (the
    // source→ModelGraph parse is `ruff_python_spo`'s job, tested there). Carries
    // the relation-aware shape this session's ruff#34/#35 work added.
    fn account_move_graph() -> ModelGraph {
        let mut m = Model::new("account_move");
        m.fields.push(Field {
            name: "name".to_string(),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "partner_id".to_string(),
            target: Some("res.partner".to_string()),
            relation_kind: Some("many2one".to_string()),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "line_ids".to_string(),
            target: Some("account.move.line".to_string()),
            inverse_name: Some("move_id".to_string()),
            relation_kind: Some("one2many".to_string()),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "amount_total".to_string(),
            emitted_by: Some("_compute_amount".to_string()),
            depends_on: vec!["line_ids.balance".to_string()],
            ..Default::default()
        });
        m.functions.push(Function {
            name: "_compute_amount".to_string(),
            reads: Vec::new(),
            raises: Vec::new(),
            traverses: Vec::new(),
            ..Default::default()
        });
        let mut g = ModelGraph::new("odoo");
        g.models.push(m);
        g
    }

    #[test]
    fn account_move_compiles_to_commercial_document_rail_class() {
        let graph = account_move_graph();
        let compiled = compile_graph_python::<OdooPort>(&graph);
        assert_eq!(compiled.len(), 1);
        let cc = &compiled[0];

        // The schema is present (the #132 field projection): partner_id /
        // line_ids land as associations, amount_total as a computed field.
        assert_eq!(cc.class.name, "account_move");
        assert!(
            !cc.class.associations.is_empty(),
            "relational fields project into associations"
        );
        assert!(
            !cc.class.computed_fields.is_empty(),
            "amount_total projects into computed_fields"
        );

        // The rail facet carries the canonical render classid — Odoo prefix
        // 0x0002 | commercial_document concept 0x0202.
        assert_eq!(cc.facet.facet_classid(), 0x0002_0202);
        assert_eq!(cc.facet.facet_classid() & 0xFFFF, 0x0202, "shared concept");
        assert_eq!(
            (cc.facet.facet_classid() >> 16) as u16,
            OdooPort::APP_PREFIX,
            "Odoo render prefix",
        );
    }

    #[test]
    fn member_facets_inherit_the_class_render_classid() {
        let graph = account_move_graph();
        let mint = mint_graph::<OdooPort>(&graph);
        // Members are addressed within their class: same render classid.
        for node in ["odoo:account_move", "odoo:account_move.partner_id"] {
            let f = mint.facet(node).unwrap_or_else(|| panic!("{node} mints"));
            assert_eq!(f.facet_classid(), 0x0002_0202, "{node} classid");
        }
    }

    #[test]
    fn mint_is_port_agnostic_same_concept_different_render_prefix() {
        // The mint doesn't care about the source language — only the port
        // lens. An OpenProject WorkPackage graph minted through OpenProjectPort
        // lands as 0x0001_0102 (OpenProject prefix | project_work_item), the
        // SAME low-u16 concept Odoo/Redmine converge on, a different prefix.
        let mut graph = ModelGraph::new("openproject");
        graph.models.push(Model::new("WorkPackage"));
        let mint = mint_graph::<OpenProjectPort>(&graph);
        let facet = mint
            .facet("openproject:WorkPackage")
            .expect("WorkPackage mints");
        assert_eq!(facet.facet_classid(), 0x0001_0102);
        assert_eq!(facet.facet_classid() & 0xFFFF, 0x0102, "project_work_item");
        assert_eq!(
            (facet.facet_classid() >> 16) as u16,
            OpenProjectPort::APP_PREFIX,
        );
    }

    #[test]
    fn unmapped_model_mints_the_bootstrap_address_not_a_wrong_classid() {
        // A model with no codebook entry resolves to classid 0 (bootstrap),
        // never a mis-stamped foreign id.
        let mut graph = ModelGraph::new("odoo");
        graph.models.push(Model::new("ir_cron"));
        let mint = mint_graph::<OdooPort>(&graph);
        let facet = mint.facet("odoo:ir_cron").expect("node mints");
        assert_eq!(facet.facet_classid(), 0, "unmapped -> bootstrap address");
    }

    // ───── compile_graph_ruby (Rails: the OP/Redmine convergence path) ─────

    /// A representative `WorkPackage` `ModelGraph`, constructed directly (the
    /// source→ModelGraph parse is `ruff_ruby_spo`'s job, tested there).
    /// Carries an association so the schema-arm projects something besides
    /// a bare name.
    fn work_package_graph(namespace: &str, model_name: &str) -> ModelGraph {
        let mut m = Model::new(model_name);
        m.associations.push(AssocDecl {
            kind: AssocKind::BelongsTo,
            name: "project".to_string(),
            options: Vec::new(),
        });
        m.functions.push(Function {
            name: "update_status".to_string(),
            ..Default::default()
        });
        let mut g = ModelGraph::new(namespace);
        g.models.push(m);
        g
    }

    #[test]
    fn compile_graph_ruby_stamps_language_ruby_not_python() {
        // The bug compile_graph_python's own doc-comment warns against:
        // calling the Python lift on a Rails graph mis-stamps the producer
        // language. compile_graph_ruby must route through `lift_model_graph`
        // (Language::Ruby), never `lift_model_graph_python`.
        let graph = work_package_graph("openproject", "WorkPackage");
        let compiled = compile_graph_ruby::<OpenProjectPort>(&graph);
        assert_eq!(compiled.len(), 1);
        assert_eq!(compiled[0].class.language, Language::Ruby);
    }

    #[test]
    fn openproject_work_package_compiles_to_project_work_item_rail_class() {
        let graph = work_package_graph("openproject", "WorkPackage");
        let compiled = compile_graph_ruby::<OpenProjectPort>(&graph);
        assert_eq!(compiled.len(), 1);
        let cc = &compiled[0];

        assert_eq!(cc.class.name, "WorkPackage");
        assert!(
            !cc.class.associations.is_empty(),
            "belongs_to :project projects into an association"
        );

        // OpenProject prefix 0x0001 | project_work_item concept 0x0102.
        assert_eq!(cc.facet.facet_classid(), 0x0001_0102);
        assert_eq!(
            cc.facet.facet_classid() & 0xFFFF,
            0x0102,
            "shared concept"
        );
        assert_eq!(
            (cc.facet.facet_classid() >> 16) as u16,
            OpenProjectPort::APP_PREFIX,
            "OpenProject render prefix",
        );
    }

    #[test]
    fn openproject_and_redmine_compile_to_the_same_concept_different_render_skin() {
        // The literal "one canonical concept, two render skins" proof: the
        // SAME Rails-shaped ModelGraph (WorkPackage / Issue) minted through
        // each fork's port converges on the identical low-u16 concept and
        // diverges only on the high-u16 app-render prefix.
        let op_graph = work_package_graph("openproject", "WorkPackage");
        let rm_graph = work_package_graph("redmine", "Issue");

        let op_compiled = compile_graph_ruby::<OpenProjectPort>(&op_graph);
        let rm_compiled = compile_graph_ruby::<RedminePort>(&rm_graph);

        let op_id = op_compiled[0].facet.facet_classid();
        let rm_id = rm_compiled[0].facet.facet_classid();

        assert_eq!(op_id & 0xFFFF, rm_id & 0xFFFF, "shared concept converges");
        assert_eq!(op_id & 0xFFFF, 0x0102, "project_work_item");
        assert_ne!(
            (op_id >> 16) as u16,
            (rm_id >> 16) as u16,
            "render prefixes diverge (OpenProject vs Redmine skin)"
        );
        assert_eq!((op_id >> 16) as u16, OpenProjectPort::APP_PREFIX);
        assert_eq!((rm_id >> 16) as u16, RedminePort::APP_PREFIX);
    }
}
