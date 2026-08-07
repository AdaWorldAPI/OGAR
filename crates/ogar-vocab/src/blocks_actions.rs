//! Blocks capability surface — the **visual block-programming authoritative
//! action table**.
//!
//! Declares the capabilities a block frontend exposes over the `0x17XX` Blocks
//! concepts, as real [`ActionDef`]s, so
//! [`resolve_hotplug`](crate::capability_registry::resolve_hotplug) can
//! activate `blockly-rs` (and any sibling frontend) when it plugs a Blocks
//! classid in.
//!
//! # Why this lives HERE and not in `ogar-blockly`
//!
//! Same reason `geo_actions` lives here and not in `ogar-osm`, and the same
//! defect avoided: [`domain_tables`](crate::capability_registry::domain_tables)
//! resolves its entries at compile time from modules inside THIS crate, so a
//! table declared in the producer crate is a **parallel registry that verifies
//! itself against itself and passes** while the real port answers
//! `NoCapabilitiesFor`. `ogar-blockly` keeps what it is actually for — the
//! opcode palette, the [`Vocabulary`] implementation, the SoA split — none of
//! which is a capability declaration.
//!
//! # The subject split
//!
//! Only `block_function` binds capabilities. `block_inventory` is minted and
//! addressable but declares none, deliberately: a registry read never touches
//! a body, so no executor has an inventory arm, and a capability with no arm
//! fails `resolve_hotplug`'s both-directions coverage check for every
//! consumer. Declaring surface nobody implements is the fiction that check
//! exists to catch.
//!
//! | subject concept | capabilities |
//! |---|---|
//! | `block_function` (`0x1701`) | `lower_script` / `raise_calls` / `render_text` / `parse_text` / `klickweg_address` |
//! | `block_inventory` (`0x1702`) | *(none — registry rows are not bodies)* |
//!
//! # Every entry exists in the consumer today
//!
//! `lower_script` / `raise_calls` / `render_text` / `parse_text` are
//! `blockly_abi`'s own public functions; `klickweg_address` is
//! `blockly_abi::klickweg::address_of`. `resolve_hotplug` checks coverage in
//! BOTH directions, so an aspirational entry here fails blockly-rs's own
//! activation test rather than quietly describing work nobody did.
//!
//! [`Vocabulary`]: https://docs.rs/ogar-loco

use crate::{ActionDef, ActionSubject, KausalSpec};

/// Every Blocks capability name, in table order — the `const`-evaluable
/// fingerprint of [`blocks_actions`], for a cheap exhaustiveness fuse without
/// paying for the table's allocations.
pub const BLOCKS_ACTION_NAMES: &[&str] = &[
    "lower_script",
    "raise_calls",
    "render_text",
    "parse_text",
    "klickweg_address",
];

/// One Blocks [`ActionDef`]. `object_class` is `ogit-blocks/{concept}` so
/// `derive_action_rows` recovers the concept from the last `/` segment and
/// resolves it against the codebook — the same fuse shape as
/// `geo_actions::geo_action_def` and `ocr_actions::ocr_action_def`.
fn blocks_action_def(capability: &'static str, subject_concept: &'static str) -> ActionDef {
    let object_class = format!("ogit-blocks/{subject_concept}");
    let identity = format!("{object_class}::action_def::{capability}");
    ActionDef {
        identity,
        predicate: capability.to_owned(),
        object_class,
        // Lowering a workspace or raising a body is a pure transform the
        // editor invokes on its own content — the caller is the substrate
        // (an editor cast, a render pass), not an authenticated User.
        default_subject: ActionSubject::System,
        // Invoked directly by a same-process caller with no OGAR-side
        // precondition to guard on — `KausalSpec::External`'s documented case.
        kausal: Some(KausalSpec::External),
        ..ActionDef::default()
    }
}

/// The Blocks capability surface — one [`ActionDef`] per capability, in
/// [`BLOCKS_ACTION_NAMES`] order.
#[must_use]
pub fn blocks_actions() -> Vec<ActionDef> {
    const SUBJECT_OF: &[(&str, &str)] = &[
        ("lower_script", "block_function"),
        ("raise_calls", "block_function"),
        ("render_text", "block_function"),
        ("parse_text", "block_function"),
        ("klickweg_address", "block_function"),
    ];
    SUBJECT_OF
        .iter()
        .map(|&(capability, subject)| blocks_action_def(capability, subject))
        .collect()
}

/// The executors the authority EXPECTS to register against this table.
///
/// `blockly-abi` is the crate that owns every capability above. A sibling
/// frontend (scratch-rs) that grows the same arms is added here in the PR
/// that ships them, never in advance.
pub const BLOCKS_EXPECTED_EXECUTORS: &[&str] = &["blockly-abi"];

/// The distinct subject classids this table binds (canon-high concept ids).
/// A registering consumer must activate exactly this set — `block_inventory`
/// is deliberately absent (see the module's subject split).
pub const BLOCKS_SUBJECT_CLASSIDS: &[u16] = &[crate::class_ids::BLOCK_FUNCTION];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_registry::{HotplugDrift, entries_from_actions, resolve_hotplug};

    #[test]
    fn every_subject_concept_resolves_through_the_codebook() {
        // The fuse: `derive_action_rows` splits `object_class` on '/' and
        // resolves the tail. A typo'd concept name would land in the slag
        // ledger as id 0 rather than failing loudly here, so assert the ids.
        let rows = entries_from_actions(&blocks_actions());
        assert_eq!(rows.len(), BLOCKS_ACTION_NAMES.len());
        for (capability, id) in &rows {
            assert_ne!(*id, 0, "{capability} did not resolve to a minted concept");
            assert_eq!(
                id >> 8,
                0x17,
                "{capability} resolved outside the Blocks domain"
            );
        }
    }

    #[test]
    fn the_fingerprint_matches_the_table_in_order() {
        let defs = blocks_actions();
        assert_eq!(defs.len(), BLOCKS_ACTION_NAMES.len());
        for (def, name) in defs.iter().zip(BLOCKS_ACTION_NAMES) {
            assert_eq!(&def.predicate, name, "fingerprint drifted from the table");
        }
    }

    #[test]
    fn the_declared_subjects_are_exactly_the_subjects_the_table_uses() {
        let mut used: Vec<u16> = entries_from_actions(&blocks_actions())
            .into_iter()
            .map(|(_, id)| id)
            .collect();
        used.sort_unstable();
        used.dedup();
        assert_eq!(used, BLOCKS_SUBJECT_CLASSIDS.to_vec());
    }

    #[test]
    fn plugging_the_blocks_classid_into_the_port_activates() {
        // The activation blockly-rs's own test mirrors. Before this module
        // existed the same call returned `Err(UnknownClassid(0x1701))` — the
        // Blocks concepts were not in `class_ids::ALL` at all.
        let (concepts, capabilities) =
            resolve_hotplug("blockly-abi", BLOCKS_SUBJECT_CLASSIDS, BLOCKS_ACTION_NAMES)
                .expect("the blocks domain must activate");
        assert_eq!(capabilities.len(), BLOCKS_ACTION_NAMES.len());
        let names: Vec<&str> = concepts.iter().map(|&(n, _)| n).collect();
        assert_eq!(names, vec!["block_function"]);
    }

    #[test]
    fn the_port_rejects_a_wrong_consumer_and_coverage_gaps_both_ways() {
        // Can-fire halves, so the activation above is not "the port says yes
        // to everything".
        assert!(matches!(
            resolve_hotplug(
                "some-other-crate",
                BLOCKS_SUBJECT_CLASSIDS,
                BLOCKS_ACTION_NAMES
            ),
            Err(HotplugDrift::UnexpectedConsumer(_))
        ));
        // Declared-but-uncovered: the executor is missing an arm.
        assert!(matches!(
            resolve_hotplug("blockly-abi", BLOCKS_SUBJECT_CLASSIDS, &["lower_script"]),
            Err(HotplugDrift::Uncovered(_))
        ));
        // Covered-but-undeclared: the executor claims surface the authority
        // does not declare — the other direction, which a one-way check misses.
        let mut over = BLOCKS_ACTION_NAMES.to_vec();
        over.push("compile_to_wasm");
        assert!(matches!(
            resolve_hotplug("blockly-abi", BLOCKS_SUBJECT_CLASSIDS, &over),
            Err(HotplugDrift::Undeclared(_))
        ));
    }

    #[test]
    fn the_inventory_concept_is_minted_but_binds_nothing() {
        // The deliberate half of the subject split: `block_inventory` is a
        // real, addressable classid, and plugging it reports the honest
        // "no capability" rather than silently activating an empty set.
        assert!(
            crate::class_ids::ALL
                .iter()
                .any(|&(_, id)| id == crate::class_ids::BLOCK_INVENTORY)
        );
        assert!(matches!(
            resolve_hotplug(
                "blockly-abi",
                &[crate::class_ids::BLOCK_INVENTORY],
                BLOCKS_ACTION_NAMES
            ),
            Err(HotplugDrift::NoCapabilitiesFor(0x1702))
        ));
    }
}
