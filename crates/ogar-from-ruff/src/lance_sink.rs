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
//! - `NodeRow` + `NodeRowPacket::as_le_bytes`, the zero-copy Lance byte path
//!   (the embedding idiom first used by the since-deprecated `symbiont` crate).
//!
//! # What this does (and deliberately does NOT)
//!
//! `CompiledClass.facet` is a [`ruff_spo_address::Facet`] whose 16 LE bytes are
//! **byte-identical** to [`lance_graph_contract::facet::FacetCascade`]
//! (`facet_classid(4) | 6×(is_a:lo, part_of:hi)`, the L1 **rails** plane,
//! `CascadeShape::G6D2`). So [`compiled_class_to_facet`] is a reinterpret no-op.
//! [`compiled_class_to_noderow`] embeds one class as ONE 512-byte CANON
//! [`NodeRow`] whose **key IS the minted facet**: the same 16 bytes, carried
//! through lance-graph-contract's byte-identical `FacetCascade → NodeGuid`
//! conversion — render classid in `[0..4)`, the `part_of:is_a` rails in
//! `[4..16)`. `ValueSchema::Bootstrap` (all-zero value slab). The facet is the
//! row's identity, so [`compiled_classes_to_noderows`] refuses a batch in which
//! two classes mint the same facet. [`compiled_classes_to_le_bytes`] packs a
//! slice onto the zero-copy storage boundary.
//!
//! The key used to be built with `NodeGuid::new(classid, 0, 0, 0, 0, identity)`
//! — a V1 `family:identity` tail that dropped the rails. That was deliberate
//! while the rail-chain vs key-tail reconciliation was open; lance-graph has
//! since settled it (a V3 mint never degrades to a V1 tail; `NodeGuid` and
//! `FacetCascade` are the same 16 bytes and convert without reinterpretation).
//!
//! Out of scope (named, not hidden), per the W2 plan:
//! - The actual Lance `Dataset::write` I/O — stops at `as_le_bytes()`, exactly
//!   as `network::to_facet` stops at the facet and symbiont's tombstone write is
//!   BLOCKED. That column write needs the lance engine / ractor runtime.
//! - The 12-byte rail payload → a NodeRow **value tenant** (`[H2]`: needs a new
//!   append-only `ValueTenant`, `v3-envelope-auditor`-gated) — mirror network,
//!   which added no lane. The rails ride the `FacetCascade` surface only.
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

/// Embed one [`CompiledClass`] as ONE 512-byte CANON [`NodeRow`]: the key is
/// the minted facet itself (render classid + `part_of:is_a` rails, byte for
/// byte), `ValueSchema::Bootstrap` (all-zero 480-byte value slab), empty edge
/// block.
///
/// No V1 `family:identity` tail is minted: the key comes from
/// `NodeGuid::from(FacetCascade)`, never `NodeGuid::new`.
///
/// # Example
///
/// ```ignore
/// let row = compiled_class_to_noderow(&cc);
/// assert_eq!(row.key.as_bytes(), &cc.facet.to_bytes());
/// ```
#[must_use]
pub fn compiled_class_to_noderow(cc: &CompiledClass) -> NodeRow {
    NodeRow {
        key: NodeGuid::from(compiled_class_to_facet(cc)),
        edges: EdgeBlock::default(),
        // ValueSchema::Bootstrap == FieldMask::EMPTY == all-zero slab; asserted
        // by the constant below so a reader never mistakes it for a live schema.
        value: [0u8; 480],
    }
}

/// Two classes in one batch minted byte-identical facets, so they would share
/// a row key. Indices are positions in the input slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateFacetKey {
    /// The earlier class with this key.
    pub first: usize,
    /// The later class that collides with it.
    pub second: usize,
}

impl core::fmt::Display for DuplicateFacetKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "classes {} and {} mint the same 16-byte facet, so they would share a row key",
            self.first, self.second
        )
    }
}

impl std::error::Error for DuplicateFacetKey {}

/// Embed a batch of classes, one [`NodeRow`] each, in input order.
///
/// # Errors
///
/// [`DuplicateFacetKey`] if two classes mint the same facet: the facet is the
/// row key, so a duplicate would make two classes indistinguishable.
pub fn compiled_classes_to_noderows(
    ccs: &[CompiledClass],
) -> Result<Vec<NodeRow>, DuplicateFacetKey> {
    let rows: Vec<NodeRow> = ccs.iter().map(compiled_class_to_noderow).collect();
    let mut seen: std::collections::HashMap<[u8; 16], usize> =
        std::collections::HashMap::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        if let Some(&first) = seen.get(row.key.as_bytes()) {
            return Err(DuplicateFacetKey { first, second: i });
        }
        seen.insert(*row.key.as_bytes(), i);
    }
    Ok(rows)
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

    /// A second, unrelated Odoo model, so two DISTINCT classes can be sunk
    /// (one class now yields exactly one key).
    fn res_partner_compiled() -> CompiledClass {
        let mut g = ModelGraph::new("odoo");
        g.models.push(Model::new("res_partner"));
        crate::mint::compile_graph_python::<OdooPort>(&g)
            .into_iter()
            .find(|c| c.class.name == "res_partner")
            .expect("res_partner compiles")
    }

    // ── T-C: the render classid lands in the key ──
    #[test]
    fn classid_lands_in_key() {
        let cc = account_move_compiled();
        let row = compiled_class_to_noderow(&cc);
        assert_eq!(
            row.key.classid(),
            cc.facet.facet_classid(),
            "key[0..4) decodes the render classid"
        );
    }

    // ── T-D: the key IS the minted facet — rails included, no V1 tail ──
    #[test]
    fn key_is_the_minted_facet_rails_included() {
        let cc = account_move_compiled();
        // Anti-vacuity: with all-zero rails a bootstrap V1 key and the facet
        // key would be indistinguishable in [4..16), and this test would pass
        // for the old code too.
        assert_ne!(
            cc.facet.to_bytes()[4..],
            [0u8; 12],
            "fixture must carry non-zero rails"
        );
        let row = compiled_class_to_noderow(&cc);
        assert_eq!(row.key.as_bytes(), &cc.facet.to_bytes(), "byte for byte");
        let back = row.key.facet();
        assert_eq!(
            back.hi_chain(),
            cc.facet.part_of_chain(),
            "part_of in the key"
        );
        assert_eq!(back.lo_chain(), cc.facet.is_a_chain(), "is_a in the key");
    }

    // ── T-E: round-trip through the storage byte boundary ──
    #[test]
    fn le_bytes_round_trip_preserves_keys() {
        let ccs = [account_move_compiled(), res_partner_compiled()];
        let rows = compiled_classes_to_noderows(&ccs)
            .unwrap_or_else(|e| panic!("distinct classes refused: {e}"));

        // The true zero-copy decode reads the packet's OWN bytes (a borrow of the
        // 64-aligned `&[NodeRow]`, per canonical_node's align(64) contract).
        let packet = NodeRowPacket::new(&rows, 0);
        let aligned = packet.as_le_bytes();
        assert_eq!(aligned.len(), 2 * 512, "two 512-byte CANON rows");
        let decoded = node_rows_from_le_bytes(aligned).expect("aligned zero-copy decode");
        assert_eq!(decoded.len(), 2);
        for (cc, row) in ccs.iter().zip(decoded) {
            assert_eq!(row.key.as_bytes(), &cc.facet.to_bytes());
        }

        // The owned-`Vec` helper carries the identical bytes (a copy off the
        // storage boundary — Lance owns re-alignment on read-back).
        assert_eq!(compiled_classes_to_le_bytes(&rows, 0), aligned);
    }

    // ── T-F: field isolation (I-LEGACY-API-FEATURE-GATED matrix) ──
    #[test]
    fn field_isolation_key_value_and_edges_are_independent() {
        let a = compiled_class_to_noderow(&account_move_compiled());
        let b = compiled_class_to_noderow(&res_partner_compiled());
        assert_ne!(a.key, b.key, "distinct classes, distinct keys");
        assert_eq!(
            a.value, [0u8; 480],
            "no value lane written (bootstrap schema)"
        );
        assert_eq!(b.value, [0u8; 480]);
        assert_eq!(a.edges, EdgeBlock::default(), "edge block reserved-zero");
        assert_eq!(b.edges, EdgeBlock::default());
    }

    // ── T-H: the batch refuses a duplicate key, and only a duplicate ──
    #[test]
    fn batch_refuses_duplicate_facets_and_only_those() {
        let am = account_move_compiled();
        let rp = res_partner_compiled();
        // can it fire: the same class twice is the same key
        assert_eq!(
            compiled_classes_to_noderows(&[am.clone(), rp.clone(), am.clone()]).err(),
            Some(DuplicateFacetKey {
                first: 0,
                second: 2
            })
        );
        // can it stay silent: distinct classes pass, in input order
        let rows = compiled_classes_to_noderows(&[rp.clone(), am.clone()])
            .unwrap_or_else(|e| panic!("distinct classes refused: {e}"));
        assert_eq!(rows[0].key.as_bytes(), &rp.facet.to_bytes());
        assert_eq!(rows[1].key.as_bytes(), &am.facet.to_bytes());
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
