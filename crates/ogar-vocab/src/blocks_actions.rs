//! Blocks capability surface — **feature-activated, never shared canon**.
//!
//! Compiled ONLY under `--features blocks`, which `ogar-blockly` turns on for
//! its own dependents. A consumer that never touches a block editor never
//! compiles this module, never carries its rows, and sees `0x17XX` exactly as
//! it did before: a reserved domain with zero concepts.
//!
//! # Why this is not in `class_ids::ALL`
//!
//! `class_ids::ALL` is mirrored into `lance_graph_contract::ogar_codebook`
//! under a **compile-time** count fuse (`lance_graph_ogar::parity::COUNT_FUSE`).
//! Anything minted there is, by construction, a lance-graph change — and a
//! block editor's palette is not lance-graph's concern. Minting these rows
//! there once already turned lance-graph red against `main` and dragged in
//! `ogar-class-view`, `all_promoted_classes` and both fuse halves.
//!
//! So the activated rows live HERE, in [`ACTIVATED_CONCEPTS`], which
//! [`resolve_hotplug`](crate::capability_registry::resolve_hotplug) consults
//! **in addition to** `class_ids::ALL`. The global codebook keeps its exact
//! contents; a particular frontend's codebook rides its own feature.
//!
//! # Why the table is here and not in `ogar-blockly`
//!
//! [`domain_tables`](crate::capability_registry::domain_tables) resolves its
//! entries at compile time from modules inside THIS crate, so a table declared
//! in the producer crate is a **parallel registry that verifies itself against
//! itself and passes** while the real port answers `NoCapabilitiesFor` — the
//! `ogar-osm` defect `geo_actions` was written to correct. `ogar-blockly`
//! cannot host it for a second reason: `ogar-blockly` depends on
//! `ogar-vocab`, so `ogar-vocab` could never read the ids back.
//!
//! That dependency direction is also why the id is declared here and *read* by
//! `ogar-blockly`, not the reverse — one source of truth for `0x1717`.
//!
//! # The subject
//!
//! Only the PALETTE concept binds capabilities. `ogar-loco`'s node shapes
//! (`0x1701` / `0x1702`) are deliberately absent: they are the substrate's,
//! shared by every vocabulary, and a consumer plugging them would claim
//! ownership of the shape every sibling rides.

use crate::{ActionDef, ActionSubject, KausalSpec};

/// The Blocks **palette** concept — `0x1717`, canon-high.
///
/// Seated at `0x1717` rather than low in the domain because `0x1701`–`0x1716`
/// is `ogar-loco`'s: `0x1701`/`0x1702` are the node shapes and `0x1703`–
/// `0x1716` is the substrate's reserved headroom. Consumers are seated high so
/// the substrate keeps contiguous room beneath them (OGAR #255).
///
/// Read by `ogar_blockly::BlockConcept::Palette` — declared once, here, since
/// `ogar-blockly` deps this crate and the reverse is impossible.
pub const BLOCK_PALETTE: u16 = 0x1717;

/// The concept rows this feature ACTIVATES — consulted by
/// [`resolve_hotplug`](crate::capability_registry::resolve_hotplug) alongside
/// `class_ids::ALL`, and deliberately never merged into it.
///
/// This is the whole "codebook triggered by plug-and-play" mechanism: with the
/// feature off the slice does not exist, `canonical_concept_domain(0x1717)`
/// still routes to [`Blocks`](crate::ConceptDomain::Blocks) on the reserved
/// domain byte alone, and a plug of `0x1717` correctly reports
/// `UnknownClassid`. With the feature on — i.e. when a block editor is
/// actually in the build graph — the same plug resolves.
pub const ACTIVATED_CONCEPTS: &[(&str, u16)] = &[("block_palette", BLOCK_PALETTE)];

/// Every Blocks capability name, in table order — the `const`-evaluable
/// fingerprint of [`blocks_actions`].
pub const BLOCKS_ACTION_NAMES: &[&str] = &[
    "lower_script",
    "raise_calls",
    "render_text",
    "parse_text",
    "klickweg_address",
];

/// One Blocks [`ActionDef`], keyed by the palette concept.
///
/// `object_class` carries the concept name so `derive_action_rows` recovers it
/// from the last `/` segment — the same fuse shape as `geo_actions` and
/// `ocr_actions`. Resolution goes through the feature-activated rows, so this
/// resolves iff the feature that declares the concept is the one compiling it.
fn blocks_action_def(capability: &'static str) -> ActionDef {
    let object_class = "ogit-blocks/block_palette".to_owned();
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
///
/// Every entry is a real `blockly-abi` public function. `resolve_hotplug`
/// checks coverage in BOTH directions, so an aspirational entry fails the
/// consumer's own activation rather than quietly describing work nobody did.
#[must_use]
pub fn blocks_actions() -> Vec<ActionDef> {
    BLOCKS_ACTION_NAMES
        .iter()
        .map(|&capability| blocks_action_def(capability))
        .collect()
}

/// The executors the authority EXPECTS to register against this table.
pub const BLOCKS_EXPECTED_EXECUTORS: &[&str] = &["blockly-abi"];

/// The distinct subject classids this table binds. A registering consumer must
/// activate exactly this set — the substrate's `0x1701`/`0x1702` are
/// deliberately absent.
pub const BLOCKS_SUBJECT_CLASSIDS: &[u16] = &[BLOCK_PALETTE];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_registry::{HotplugDrift, resolve_hotplug};

    #[test]
    fn the_activated_concept_resolves_only_because_this_feature_is_on() {
        // The whole point of the mechanism: 0x1717 is NOT in class_ids::ALL —
        // it resolves through the feature-activated rows. Assert BOTH halves,
        // or "it resolved" proves nothing about where it resolved from.
        assert!(
            !crate::class_ids::ALL
                .iter()
                .any(|&(_, id)| id == BLOCK_PALETTE),
            "0x1717 must NEVER enter the globally-mirrored codebook"
        );
        assert!(
            ACTIVATED_CONCEPTS
                .iter()
                .any(|&(_, id)| id == BLOCK_PALETTE)
        );

        let (concepts, capabilities) =
            resolve_hotplug("blockly-abi", BLOCKS_SUBJECT_CLASSIDS, BLOCKS_ACTION_NAMES)
                .expect("the blocks domain must activate under its own feature");
        assert_eq!(capabilities.len(), BLOCKS_ACTION_NAMES.len());
        assert_eq!(
            concepts.iter().map(|&(n, _)| n).collect::<Vec<_>>(),
            vec!["block_palette"]
        );
    }

    #[test]
    fn the_palette_never_claims_the_substrates_node_shapes() {
        // The ownership line, asserted rather than trusted. 0x1701/0x1702 are
        // ogar-loco's; a plug of either must NOT resolve through this table.
        assert!(!BLOCKS_SUBJECT_CLASSIDS.contains(&0x1701));
        assert!(!BLOCKS_SUBJECT_CLASSIDS.contains(&0x1702));
        assert!(
            !ACTIVATED_CONCEPTS
                .iter()
                .any(|&(_, id)| id == 0x1701 || id == 0x1702)
        );
        for shape in [0x1701u16, 0x1702] {
            assert!(
                matches!(
                    resolve_hotplug("blockly-abi", &[shape], BLOCKS_ACTION_NAMES),
                    Err(HotplugDrift::UnknownClassid(id)) if id == shape
                ),
                "plugging {shape:#06x} must not resolve — it is the substrate's"
            );
        }
    }

    #[test]
    fn the_port_rejects_a_wrong_consumer_and_coverage_gaps_both_ways() {
        // Can-fire halves, so the activation above is not "yes to everything".
        assert!(matches!(
            resolve_hotplug(
                "some-other-crate",
                BLOCKS_SUBJECT_CLASSIDS,
                BLOCKS_ACTION_NAMES
            ),
            Err(HotplugDrift::UnexpectedConsumer(_))
        ));
        assert!(matches!(
            resolve_hotplug("blockly-abi", BLOCKS_SUBJECT_CLASSIDS, &["lower_script"]),
            Err(HotplugDrift::Uncovered(_))
        ));
        let mut over = BLOCKS_ACTION_NAMES.to_vec();
        over.push("compile_to_wasm");
        assert!(matches!(
            resolve_hotplug("blockly-abi", BLOCKS_SUBJECT_CLASSIDS, &over),
            Err(HotplugDrift::Undeclared(_))
        ));
    }

    #[test]
    fn the_fingerprint_matches_the_table_in_order() {
        let defs = blocks_actions();
        assert_eq!(defs.len(), BLOCKS_ACTION_NAMES.len());
        for (def, name) in defs.iter().zip(BLOCKS_ACTION_NAMES) {
            assert_eq!(&def.predicate, name, "fingerprint drifted from the table");
        }
    }
}
