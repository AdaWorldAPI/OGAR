//! Two vocabularies, one hub — the plug-and-play claim, tested across crates.
//!
//! This is the falsifier for "a consumer deps the vocabulary crates it wants
//! and routes purely by classid": it plugs `ogar-blockly` and `ogar-ro` into
//! one registry — exactly what blockly-rs / lance-graph do at boot — and
//! proves a stored node's classid alone selects the right semantic table,
//! with no consumer-side branch naming either vocabulary.

use ogar_loco::{FnIndex, VocabularyRegistry};

/// The boot sequence a consumer actually writes: one hub, N `plug_into`
/// calls, nothing vocabulary-specific afterward.
fn boot() -> VocabularyRegistry {
    let mut hub = VocabularyRegistry::new();
    ogar_blockly::plug_into(&mut hub).unwrap();
    ogar_ro::plug_into(&mut hub).unwrap();
    hub
}

#[test]
fn one_hub_routes_two_vocabularies_by_classid_alone() {
    let hub = boot();
    assert_eq!(hub.len(), 2);

    // Two stored nodes under DIFFERENT app prefixes — routing must ignore
    // the lo u16 (render skin) and read only the hi u16 (concept).
    let blockly_node = ogar_blockly::BlockConcept::Content.render_classid(0x1000);
    let relation_node = ogar_ro::relation_body_render_classid(0xBEEF);

    let blocks = hub.resolve_classid(blockly_node).expect("blocks plugged");
    let relations = hub.resolve_classid(relation_node).expect("ro plugged");

    // The RO table covers its predicates; the Blockly table refuses that
    // same byte (no device family minted). Same FnIndex, two answers —
    // which is the whole point of routing by classid.
    let part_of = ogar_ro::PART_OF;
    assert_eq!(relations.stack_arity(part_of), Some(2));
    assert_eq!(relations.name(part_of), Some("part_of"));
    assert_eq!(blocks.stack_arity(part_of), None);
    assert_eq!(blocks.name(part_of), None);
}

#[test]
fn the_shared_core_is_identical_across_every_plugged_device() {
    // The floor discipline must survive registration: two devices on one hub
    // answer the shared computational range the same way, byte for byte.
    // A drift here would mean `ADD` means two things depending on which node
    // you happened to load.
    let hub = boot();
    let blocks = hub
        .resolve_classid(ogar_blockly::BlockConcept::Content.render_classid(0x1000))
        .unwrap();
    let relations = hub
        .resolve_classid(ogar_ro::relation_body_render_classid(0x1000))
        .unwrap();

    for b in 0..ogar_loco::DOMAIN_FLOOR {
        let f = FnIndex(b);
        assert_eq!(blocks.stack_arity(f), relations.stack_arity(f), "{f:?}");
        assert_eq!(blocks.body_refs(f), relations.body_refs(f), "{f:?}");
        assert_eq!(blocks.pushes_result(f), relations.pushes_result(f), "{f:?}");
        assert_eq!(blocks.name(f), relations.name(f), "{f:?}");
    }
    // Anti-vacuity: the shared range must actually be covered somewhere, or
    // "identical" is trivially true of two empty tables.
    assert_eq!(blocks.stack_arity(FnIndex::ADD), Some(2));
    assert_eq!(blocks.body_refs(FnIndex::IF_ELSE), 2);
}

#[test]
fn an_unplugged_concept_resolves_to_nothing_rather_than_a_default() {
    // The fail-closed half: a consumer that forgot to dep a vocabulary gets
    // `None` and can refuse, never a silently-wrong table. A hub that
    // answered *something* for every classid would carry no information.
    let hub = boot();
    assert!(hub.resolve_classid(0x0999_1000).is_none());
    // …and the inventory concept is deliberately NOT plugged: registry rows
    // are not function bodies, so they carry no call vocabulary.
    let inventory = ogar_blockly::BlockConcept::Inventory.render_classid(0x1000);
    assert!(hub.resolve_classid(inventory).is_none());
}

#[test]
fn plugging_the_same_device_twice_is_refused() {
    // Idempotence is NOT the contract — a double-plug means two crates think
    // they own one concept, which must surface at boot, not at read time.
    let mut hub = boot();
    assert!(ogar_ro::plug_into(&mut hub).is_err());
    assert!(ogar_blockly::plug_into(&mut hub).is_err());
    // …and the first device kept its port.
    assert_eq!(hub.len(), 2);
    let relations = hub
        .resolve_classid(ogar_ro::relation_body_render_classid(0x1000))
        .unwrap();
    assert_eq!(relations.name(ogar_ro::PART_OF), Some("part_of"));
}
