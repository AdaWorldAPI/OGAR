//! Membrane probe: OGIT entity TTL (+ its attribute TTLs) → `ogar-from-schema`
//! → canonical `Class` (the controller-DTO wire shape).
//!
//! What this probe proves: TTL entity + attributes → canonical `Class` (DTO
//! wire shape), wire-names pinned, via the named `lift_ogit_entity` pipeline
//! helper. What it does NOT yet do (named, not implemented here):
//! - **Label→wire-name late resolution** (woa-rs practice: DTO field labels
//!   from `rdfs:label`) — the `Class` carries local names; a controller
//!   DTO's human labels resolve from the OGIT hashmap at render.
//! - **owl:Class support** so `vocab/ogar.ttl` (the OGAR meta-vocab) itself
//!   parses (walker enhancement) — out of scope per SPEC-3 Non-Goals.
//! - **DTO emission**: turning the `Class` into a concrete controller DTO
//!   type (Rust/Python) needs a Facet (classid) — for OGIT entities that is
//!   the registry-resolve path, not a port mint. `emit_python` on a
//!   bootstrap `CompiledClass { class, facet: classid 0 }` would show the
//!   `@dataclass` DTO text; a follow-up can add a dedicated
//!   `ogar-from-schema` DTO lowering pass.
//!
//! Fixture note: the mission named `vocab/ogar.ttl`, but that file is
//! `owl:Class` meta-vocab (26 occurrences of `a owl:Class`, 0 `rdfs:Class`)
//! which the walker's `detect_kind` does not recognise (only `a rdfs:Class`
//! / `a owl:DatatypeProperty`). This probe uses the real OGIT NTO entity
//! fixture family (`rdfs:Class` dialect) instead:
//! `vocab/imports/ogit/NTO/Documents/entities/DocumentInfoRecord.ttl`.

use ogar_from_schema::lift_ogit_entity;

const ENTITY: &str =
    include_str!("../../../vocab/imports/ogit/NTO/Documents/entities/DocumentInfoRecord.ttl");
const A_NUMBER: &str =
    include_str!("../../../vocab/imports/ogit/NTO/Documents/attributes/documentNumber.ttl");
const A_TYPE: &str =
    include_str!("../../../vocab/imports/ogit/NTO/Documents/attributes/documentType.ttl");
const A_PARTID: &str =
    include_str!("../../../vocab/imports/ogit/NTO/Documents/attributes/documentPartId.ttl");
const A_VERSION: &str =
    include_str!("../../../vocab/imports/ogit/NTO/Documents/attributes/documentVersion.ttl");

#[test]
fn document_info_record_lowers_to_controller_dto_with_expected_wire_names() {
    let class = lift_ogit_entity(ENTITY, &[A_NUMBER, A_TYPE, A_PARTID, A_VERSION])
        .expect("DocumentInfoRecord entity must lower");
    // (a) entity → class name.
    assert_eq!(class.name, "DocumentInfoRecord");
    // (b) parent lowered from rdfs:subClassOf (verified against
    // DocumentInfoRecord.ttl: `rdfs:subClassOf ogit:Entity;`).
    assert_eq!(class.parent.as_deref(), Some("ogit:Entity"));
    // (c) the DTO wire-name set — the controller-DTO field names — includes
    //     every declared optional attribute local name. The entity's
    //     `optional-attributes` list carries curies
    //     (`ogit.Documents:documentNumber`); `lift_ogit_entity` localizes
    //     them to match `into_class`'s column names.
    let names: std::collections::HashSet<&str> =
        class.attributes.iter().map(|a| a.name.as_str()).collect();
    for wire in [
        "documentNumber",
        "documentType",
        "documentPartId",
        "documentVersion",
    ] {
        assert!(names.contains(wire), "missing wire name `{wire}` in DTO: {names:?}");
    }
}
