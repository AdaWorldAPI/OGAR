//! End-to-end capstone of the odoo→odoo-rs V3 transpile (W1 + W2 joined):
//! **real Odoo source → V3 SoA CANON node bytes**, no SurrealQL anywhere.
//!
//! ```text
//! account_payment_term.py  (verbatim Odoo 19, tests/py/ + PROVENANCE.md)
//!   -> ruff_python_spo::extract_from_source        (AST -> ModelGraph)
//!   -> ogar_from_ruff::mint::compile_graph_python::<OdooPort>
//!        -> Vec<CompiledClass { class, facet, actions:+kausal }>   (W1 arm)
//!   -> ogar_from_ruff::lance_sink::compiled_class_to_noderow       (W2 arm)
//!   -> NodeRowPacket::as_le_bytes                                  (storage boundary)
//!   -> node_rows_from_le_bytes  (zero-copy decode round-trip)
//! ```
//!
//! This is the pull-in → sink half of `OGAR-TRANSPILE-SUBSTRATE.md` proven on a
//! single real file end to end. It joins `D-KAUSAL-CONSUME-PIN-ODOO` (the DO-arm
//! kausal witness) with `D-V3-SINK-COMPILEDCLASS` (the V3 SoA sink): the SAME
//! `CompiledClass`es that carry the pinned `KausalSpec::Depends` facts are the
//! ones embedded as CANON `NodeRow`s.
//!
//! Gated on the `lance-sink` feature (the sink's only new dependency is the
//! zero-dep `lance-graph-contract`).
#![cfg(feature = "lance-sink")]

use lance_graph_contract::canonical_node::{NodeRowPacket, node_rows_from_le_bytes};
use lance_graph_contract::soa_envelope::SoaEnvelope;
use ogar_from_ruff::lance_sink::{compiled_class_to_facet, compiled_class_to_noderow};
use ogar_from_ruff::mint::compile_graph_python;
use ogar_vocab::ports::OdooPort;
use ruff_python_spo::extract_from_source;

/// Same verbatim fixture the W1 kausal probe uses (Odoo 19 `2c78d5f1`).
const ACCOUNT_PAYMENT_TERM_PY: &str = include_str!("py/account_payment_term.py");

#[test]
fn odoo_source_transpiles_to_v3_soa_noderows() {
    // ── pull-in (W1 arm): real source → CompiledClasses ──
    let graph = extract_from_source(ACCOUNT_PAYMENT_TERM_PY);
    let compiled = compile_graph_python::<OdooPort>(&graph);
    assert_eq!(compiled.len(), 2, "two models transpile end to end");

    // ── sink (W2 arm): each CompiledClass → one CANON NodeRow ──
    let rows: Vec<_> = compiled
        .iter()
        .enumerate()
        .map(|(i, cc)| compiled_class_to_noderow(cc, i as u32))
        .collect();

    // Each row's key carries the class's render classid verbatim; the facet
    // reinterpret preserves the rail (part_of/is_a) chains.
    for (i, (cc, row)) in compiled.iter().zip(&rows).enumerate() {
        assert_eq!(
            row.key.classid(),
            cc.facet.facet_classid(),
            "row {i}: key classid == mint classid"
        );
        assert_eq!(row.key.identity(), i as u32, "row {i}: bootstrap identity");
        assert!(
            row.key.is_unbasined(),
            "row {i}: bootstrap tail (no rail leak)"
        );
        let fc = compiled_class_to_facet(cc);
        assert_eq!(
            fc.lo_chain(),
            cc.facet.is_a_chain(),
            "row {i}: is_a rails survive"
        );
        assert_eq!(
            fc.hi_chain(),
            cc.facet.part_of_chain(),
            "row {i}: part_of rails survive"
        );
    }

    // ── storage boundary: pack → zero-copy decode round-trip ──
    let packet = NodeRowPacket::new(&rows, 0);
    let bytes = packet.as_le_bytes();
    assert_eq!(bytes.len(), rows.len() * 512, "512-byte CANON rows");
    let decoded = node_rows_from_le_bytes(bytes).expect("aligned zero-copy decode");
    assert_eq!(decoded.len(), rows.len());
    for (cc, row) in compiled.iter().zip(decoded) {
        assert_eq!(row.key.classid(), cc.facet.facet_classid());
    }

    println!(
        "OK: account_payment_term.py -> {} CompiledClass -> {} CANON NodeRow ({} bytes), no SurrealQL",
        compiled.len(),
        rows.len(),
        bytes.len(),
    );
}
