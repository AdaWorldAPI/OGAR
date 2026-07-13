//! W2 — sink a [`CompiledClass`] onto the lance-graph **V3 SoA** substrate.
//!
//! The pull-in half of the transpiler mints a [`CompiledClass`] (THINK arm
//! [`Class`](ogar_vocab::Class) + DO arm [`ActionDef`](ogar_vocab::ActionDef) +
//! the 16-byte rail [`Facet`](ruff_spo_address::Facet)); nothing *stored* it.
//! This module is the storage seam: `CompiledClass → V3 byte surface`, mirroring
//! the two already-shipped sinks and inventing **no `*Bridge`**:
//!
//! - `lance_graph_contract::network` (harvest → `FacetCascade`, content-blind,
//!   one concept mint, deferred row embedding) — the *shape* borrowed here.
//! - `symbiont::bridge` (`NodeRow` + `NodeRowPacket::as_le_bytes` zero-copy Lance
//!   byte path) — the *embedding idiom* borrowed here.
//!
//! # What this does (and deliberately does NOT)
//!
//! `CompiledClass.facet` is a [`ruff_spo_address::Facet`] whose 16 LE bytes are
//! **byte-identical** to [`lance_graph_contract::facet::FacetCascade`]
//! (`facet_classid(4) | 6×(is_a:lo, part_of:hi)`, the L1 **rails** plane,
//! `CascadeShape::G6D2`). So [`compiled_class_to_facet`] is a reinterpret no-op.
//! [`compiled_class_to_noderow`] embeds one class as ONE 512-byte CANON
//! [`NodeRow`] — the render classid in the key `[0..4)`, a **bootstrap** tail
//! (family=0, HHTL=0, identity=caller-supplied), `ValueSchema::Bootstrap`
//! (all-zero value slab). [`compiled_classes_to_le_bytes`] packs a slice onto the
//! zero-copy storage boundary.
//!
//! Out of scope (named, not hidden), per the W2 plan:
//! - The actual Lance `Dataset::write` I/O — stops at `as_le_bytes()`, exactly
//!   as `network::to_facet` stops at the facet and symbiont's tombstone write is
//!   BLOCKED. That column write needs the lance engine / ractor runtime.
//! - The 12-byte rail payload → a NodeRow **value tenant** (`[H2]`: needs a new
//!   append-only `ValueTenant`, `v3-envelope-auditor`-gated) — mirror network,
//!   which added no lane. The rails ride the `FacetCascade` surface only.
//! - The key **tail from the rail tiers** (`[H1]`: le-contract §3 open item —
//!   the `6×(8:8)` rail chain vs the key's contiguous `u24++u24` tail are
//!   structurally different bit-groupings, "do not unify silently in code").
//!   The tail is bootstrapped; [`compiled_class_to_noderow`] never derives it
//!   from the rails (test-enforced).
//! - `CompiledClass.actions` (behavior) — out-of-line, not in this byte payload
//!   ("neither half carries behavior").
//! - Ownership: this is an OFFLINE **bake** (source → artifact, single-writer) =
//!   BOOTSTRAP-OK; the envelope owner is 0. NOT an online write — do not route
//!   through any batch writer. If it ever goes online, W5 stamp routing applies.

use lance_graph_contract::canonical_node::{NodeRowPacket, ValueSchema};
use lance_graph_contract::facet::FacetCascade;
use lance_graph_contract::soa_envelope::SoaEnvelope;
use lance_graph_contract::{EdgeBlock, NodeGuid, NodeRow};

use crate::mint::CompiledClass;

/// The compiled class's 16-byte rail facet as a [`FacetCascade`] — a reinterpret
/// no-op (`Facet::to_bytes` → `FacetCascade::from_bytes`, byte-identical layout).
///
/// The result is the L1 **rails** reading (`CascadeShape::G6D2`): `hi_chain` =
/// `part_of`, `lo_chain` = `is_a`. `facet_classid` is taken verbatim from the
/// mint (the sanctioned composer) — never recomposed, never `as u16`/`>> 16`.
#[must_use]
pub fn compiled_class_to_facet(cc: &CompiledClass) -> FacetCascade {
    FacetCascade::from_bytes(&cc.facet.to_bytes())
}

/// Embed one [`CompiledClass`] as ONE 512-byte CANON [`NodeRow`]: the render
/// classid in the key `[0..4)`, a **bootstrap** tail (`family = 0`, HHTL = 0,
/// `identity` = caller-supplied), `ValueSchema::Bootstrap` (all-zero 480-byte
/// value slab), empty edge block. Mirrors `symbiont::bridge::board`.
///
/// The key tail is bootstrapped, NOT derived from the rail tiers (`[H1]`
/// freeze — the rail chain vs the key `u24++u24` tail reconciliation is an open
/// operator/envelope ruling). `identity` must fit in 24 bits (`NodeGuid::new`
/// asserts this).
#[must_use]
pub fn compiled_class_to_noderow(cc: &CompiledClass, identity: u32) -> NodeRow {
    NodeRow {
        // classid verbatim from the mint; family/HHTL bootstrap-zero, identity
        // caller-supplied → the zero-fallback ladder's bootstrap address while
        // the class is unbasined.
        key: NodeGuid::new(cc.facet.facet_classid(), 0, 0, 0, 0, identity),
        edges: EdgeBlock::default(),
        // ValueSchema::Bootstrap == FieldMask::EMPTY == all-zero slab; asserted
        // by the constant below so a reader never mistakes it for a live schema.
        value: [0u8; 480],
    }
}

/// `ValueSchema` this sink stamps into every row it builds — the all-zero
/// bootstrap slab (no tenant lane written; the rails ride the facet surface).
pub const SINK_VALUE_SCHEMA: ValueSchema = ValueSchema::Bootstrap;

/// Pack a slice of CANON [`NodeRow`]s onto the zero-copy Lance storage boundary
/// (`NodeRowPacket::as_le_bytes`, the `SoaEnvelope` byte view). `cycle` stamps
/// the packet's write cycle. This is the last step in scope — the actual
/// `Dataset::write` is a separate, engine-side deliverable.
#[must_use]
pub fn compiled_classes_to_le_bytes(rows: &[NodeRow], cycle: u32) -> Vec<u8> {
    NodeRowPacket::new(rows, cycle).as_le_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lance_graph_contract::canonical_node::node_rows_from_le_bytes;
    use lance_graph_contract::facet::CascadeShape;
    use lance_graph_contract::soa_envelope::ENVELOPE_LAYOUT_VERSION;
    use ogar_vocab::ports::OdooPort;
    use ruff_spo_triplet::{Field, Model, ModelGraph};

    /// A representative `account.move` graph, built directly (the
    /// source→ModelGraph parse is `ruff_python_spo`'s job) — the aliased model
    /// the mint resolves to the worked-example classid `0x0202_0002`.
    fn account_move_graph() -> ModelGraph {
        let mut m = Model::new("account_move");
        m.fields.push(Field {
            name: "partner_id".to_string(),
            target: Some("res.partner".to_string()),
            relation_kind: Some("many2one".to_string()),
            ..Default::default()
        });
        let mut g = ModelGraph::new("odoo");
        g.models.push(m);
        g
    }

    /// The canonical `account.move` fixture, minted through the sanctioned
    /// composer so its classid is real (`0x0202_0002`), never hand-rolled hex.
    fn account_move_compiled() -> CompiledClass {
        let graph = account_move_graph();
        let compiled = crate::mint::compile_graph_python::<OdooPort>(&graph);
        compiled
            .into_iter()
            .find(|c| c.class.name == "account_move")
            .expect("account_move compiles")
    }

    // ── T-A: facet byte-parity (the primary gate) ──
    #[test]
    fn facet_sink_is_byte_identical_reinterpret() {
        let cc = account_move_compiled();
        let fc = compiled_class_to_facet(&cc);
        assert_eq!(
            fc.to_bytes(),
            cc.facet.to_bytes(),
            "FacetCascade round-trips the rail facet's 16 bytes verbatim"
        );
        assert_eq!(
            fc.facet_classid,
            cc.facet.facet_classid(),
            "classid taken verbatim from the mint (0x0202_0002 for account.move)"
        );
        assert_eq!(
            cc.facet.facet_classid(),
            0x0202_0002,
            "the worked-example classid"
        );
    }

    // ── T-B: the facet is L1 rails (G6D2), NOT an L6 quad ──
    #[test]
    fn facet_is_rails_plane_not_quads() {
        let cc = account_move_compiled();
        let fc = compiled_class_to_facet(&cc);
        // part_of / is_a chains survive the reinterpret in the operator's
        // 6×(8:8) rails reading. hi = part_of, lo = is_a (le-contract L1).
        assert_eq!(
            fc.hi_chain(),
            cc.facet.part_of_chain(),
            "part_of == FacetCascade hi"
        );
        assert_eq!(
            fc.lo_chain(),
            cc.facet.is_a_chain(),
            "is_a == FacetCascade lo"
        );
        // Explicitly the rails shape (6 groups × 2 levels), NOT the L6 quad
        // shape (whose odoo-? semantics are unruled — we never emit it here).
        assert_eq!(CascadeShape::G6D2.groups(), 6);
    }

    // ── T-C: the render classid lands in the key ──
    #[test]
    fn classid_lands_in_key() {
        let cc = account_move_compiled();
        let row = compiled_class_to_noderow(&cc, 7);
        assert_eq!(
            row.key.classid(),
            cc.facet.facet_classid(),
            "key[0..4) decodes the render classid"
        );
    }

    // ── T-D: bootstrap tail / [H1] guard — rails did NOT leak into the key tail ──
    #[test]
    fn key_tail_is_bootstrap_not_derived_from_rails() {
        let cc = account_move_compiled();
        let row = compiled_class_to_noderow(&cc, 7);
        assert_eq!(row.key.family(), 0, "family bootstrap-zero (unbasined)");
        assert_eq!(
            row.key.identity(),
            7,
            "identity is the caller value, not a rail byte"
        );
        assert!(
            row.key.is_unbasined(),
            "family==0 → unbasined; the rail tiers must not populate the tail ([H1])"
        );
    }

    // ── T-E: round-trip through the storage byte boundary ──
    #[test]
    fn le_bytes_round_trip_preserves_classid() {
        let cc = account_move_compiled();
        let rows = vec![
            compiled_class_to_noderow(&cc, 1),
            compiled_class_to_noderow(&cc, 2),
        ];

        // The true zero-copy decode reads the packet's OWN bytes (a borrow of the
        // 64-aligned `&[NodeRow]`, per canonical_node's align(64) contract) —
        // mirrors symbiont::bridge's round-trip.
        let packet = NodeRowPacket::new(&rows, 0);
        let aligned = packet.as_le_bytes();
        assert_eq!(aligned.len(), 2 * 512, "two 512-byte CANON rows");
        let decoded = node_rows_from_le_bytes(aligned).expect("aligned zero-copy decode");
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].key.classid(), cc.facet.facet_classid());
        assert_eq!(decoded[1].key.identity(), 2);

        // The owned-`Vec` helper carries the identical bytes (a copy off the
        // storage boundary — Lance owns re-alignment on read-back).
        assert_eq!(compiled_classes_to_le_bytes(&rows, 0), aligned);
    }

    // ── T-F: field isolation (I-LEGACY-API-FEATURE-GATED matrix) ──
    #[test]
    fn field_isolation_classid_and_identity_are_independent() {
        let cc = account_move_compiled();
        // Same class, two identities → classid identical, identity distinct,
        // value slab + edges untouched (all zero) in both.
        let a = compiled_class_to_noderow(&cc, 10);
        let b = compiled_class_to_noderow(&cc, 20);
        assert_eq!(
            a.key.classid(),
            b.key.classid(),
            "identity write left classid unchanged"
        );
        assert_ne!(a.key.identity(), b.key.identity());
        assert_eq!(
            a.value, [0u8; 480],
            "no value lane written (bootstrap schema)"
        );
        assert_eq!(b.value, [0u8; 480]);
        assert_eq!(a.edges, EdgeBlock::default(), "edge block reserved-zero");
    }

    // ── T-G: this sink moves ZERO bytes at rest — no layout-version bump ──
    #[test]
    fn sink_does_not_move_the_layout_version() {
        assert_eq!(
            ENVELOPE_LAYOUT_VERSION, 2,
            "W2 adds no tenant lane; the envelope layout is untouched at v2"
        );
        assert_eq!(SINK_VALUE_SCHEMA, ValueSchema::Bootstrap);
    }
}
