//! End-to-end integration: the DO-arm lift reaches the SPO emitter.
//!
//! Proves the full behavioral-arm path closes —
//!
//! ```text
//!   OGIT Automation TTL  →  ttl::parse_file  →  EntityDecl
//!                        →  do_arm::into_action_def  →  ogar_vocab::ActionDef
//!                        →  ogar_emitter::TripleEmitter::emit_action_def  →  Vec<Triple>
//! ```
//!
//! i.e. the `KnowledgeItem` → `ActionDef` contract template (D‑HIRO‑DO)
//! emits as canonical `ogar:ActionDef` SPO triples carrying the §4 field
//! mapping, with the lossless‑DO invariant preserved across the whole path
//! (no body inlined, so no `ogar:actionBody` triple is produced at the schema
//! level). This is the lift→triples seam the RBAC / query layers consume.

use ogar_emitter::TripleEmitter;
use ogar_from_schema::do_arm::into_action_def;
use ogar_from_schema::ttl::parse_file;
use ogar_from_schema::TtlDeclaration;

const KNOWLEDGE_ITEM_TTL: &str =
    include_str!("../../../vocab/imports/ogit/NTO/Automation/entities/KnowledgeItem.ttl");

fn obj(triples: &[ogar_emitter::Triple], subject: &str, predicate: &str) -> Option<String> {
    triples
        .iter()
        .find(|t| t.subject == subject && t.predicate == predicate)
        .map(|t| t.object.clone())
}

#[test]
fn knowledge_item_lifts_and_emits_as_action_def_triples() {
    // Lift.
    let TtlDeclaration::Entity(ki) = parse_file(KNOWLEDGE_ITEM_TTL).expect("parses") else {
        panic!("expected entity");
    };
    let def = into_action_def(&ki).expect("KnowledgeItem → ActionDef");
    let id = def.identity.clone();
    assert_eq!(id, "ogit-automation/knowledge_item::action_def::execute");

    // Emit.
    let triples = TripleEmitter::emit_action_def(&def);

    // Type triple — the def IS an ogar:ActionDef on the SPO surface.
    assert_eq!(obj(&triples, &id, "rdf:type").as_deref(), Some("ogar:ActionDef"));

    // The §4 field mapping survives the lift→emit path verbatim.
    assert_eq!(
        obj(&triples, &id, "ogar:actionPredicate").as_deref(),
        Some("execute")
    );
    assert_eq!(
        obj(&triples, &id, "ogar:actionObjectClass").as_deref(),
        Some("ogit-automation/mars_node_template")
    );

    // The `contains Trigger` → LifecycleTrigger kausal emits its triple(s).
    assert!(
        triples
            .iter()
            .any(|t| t.subject == id && t.predicate.starts_with("ogar:") && t.object == "Trigger"),
        "LifecycleTrigger event `Trigger` missing from emitted triples: {triples:?}"
    );

    // The §4 actuator verbs ride as ogar:actionDecorator provenance.
    for verb in ["relates", "contains", "uses"] {
        assert!(
            triples
                .iter()
                .any(|t| t.subject == id
                    && t.predicate == "ogar:actionDecorator"
                    && t.object == verb),
            "decorator `{verb}` missing from emitted triples"
        );
    }
}

#[test]
fn lossless_do_survives_the_emit_path() {
    // The lossless-DO invariant must hold END-TO-END, not just at the lift:
    // since `body_source` is None (the body is pointed-to, never inlined),
    // the emitter must NOT produce an `ogar:actionBody` triple. A body triple
    // here would mean some bytes leaked into the IR.
    let TtlDeclaration::Entity(ki) = parse_file(KNOWLEDGE_ITEM_TTL).expect("parses") else {
        panic!("expected entity");
    };
    let def = into_action_def(&ki).expect("ActionDef");
    let triples = TripleEmitter::emit_action_def(&def);

    assert!(
        !triples.iter().any(|t| t.predicate == "ogar:actionBody"),
        "lossless-DO violated across emit: a body triple was produced ({triples:?})"
    );
}
