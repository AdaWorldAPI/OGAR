//! **S3 artifact re-key** — carry an existing OBO-core `.soa` from the legacy
//! `0x03` addresses onto the domain-reference tree, WITHOUT re-baking.
//!
//! ```text
//! cargo run -p ogar-obo --example rekey_domain -- <in.soa> <out.soa>
//! ```
//!
//! # Why a transform and not a re-bake
//!
//! The staging migration's whole premise is that the alias is a computation:
//! only the concept's HIGH byte moves (`disease::mondo 0x0301 -> 0x9101`),
//! everything else — identity rail, edge degrees, entity type, HHTL tiers,
//! row ORDER — is byte-identical. A re-bake from the `.obo` sources would
//! reproduce all of that only if every source and every filter decision of
//! the original bake were reproduced too; the transform instead touches
//! exactly the bytes the flip defines and PROVES the rest unchanged.
//!
//! **Row order is deliberately preserved.** The label slab
//! (`obo_labels.slab`) is positional — same order in, same order out, so the
//! existing slab stays valid against the re-keyed artifact.
//!
//! # What is rewritten
//!
//! - the key classid (bytes `0..4`, u32 LE) of every row;
//! - every edge-lane HEADER classid (`layout::OBO_CORE_ROW.edge_lanes`:
//!   23 lanes x 16 B from row offset 144; each lane `classid(4) + 4xu24`).
//!   A lane names its TARGET ontology, so lane headers carry the same five
//!   classids as the keys and move by the same map.
//!
//! Everything else is copied verbatim, and the tool verifies that claim on
//! its own output (a byte-diff accounting over the non-classid positions)
//! rather than asserting it.

use ogar_obo::{NODE_ROW_STRIDE, Namespace, layout};

/// The flip, derived from the enum — never a table of literals: resolve the
/// namespace from whatever form the artifact carries (the reader accepts
/// both), re-render under the SAME app prefix. PATO/RO map to themselves.
fn map_classid(classid: u32) -> Option<u32> {
    let ns = Namespace::from_concept_id((classid >> 16) as u16)?;
    Some(ns.render_classid((classid & 0xFFFF) as u16))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let inp = args.next().expect("usage: rekey_domain <in.soa> <out.soa>");
    let out = args.next().expect("usage: rekey_domain <in.soa> <out.soa>");

    let data = std::fs::read(&inp).unwrap_or_else(|e| panic!("read {inp}: {e}"));
    assert!(
        data.len().is_multiple_of(NODE_ROW_STRIDE),
        "{inp} is not a NodeRow artifact ({} bytes)",
        data.len()
    );
    let n = data.len() / NODE_ROW_STRIDE;

    let lanes = layout::OBO_CORE_ROW.edge_lanes;
    let lane0 = lanes.row_off();
    let lane_count = layout::OBO_CORE_ROW.lane_count;

    let mut rewritten = data.clone();
    let mut keys_moved = 0usize;
    let mut keys_kept = 0usize;
    let mut lane_headers_moved = 0usize;
    let mut lane_headers_kept = 0usize;
    let mut unknown_keys = 0usize;
    let mut before = std::collections::BTreeMap::<u32, usize>::new();
    let mut after = std::collections::BTreeMap::<u32, usize>::new();

    for i in 0..n {
        let row = i * NODE_ROW_STRIDE;
        let key_cid = u32::from_le_bytes(rewritten[row..row + 4].try_into().unwrap());
        *before.entry(key_cid).or_default() += 1;
        match map_classid(key_cid) {
            Some(new) => {
                if new != key_cid {
                    keys_moved += 1;
                } else {
                    keys_kept += 1;
                }
                rewritten[row..row + 4].copy_from_slice(&new.to_le_bytes());
                *after.entry(new).or_default() += 1;
            }
            None => {
                // Not an OBO classid — refuse to guess; counted, kept.
                unknown_keys += 1;
                *after.entry(key_cid).or_default() += 1;
            }
        }
        for l in 0..lane_count {
            let off = row + lane0 + l * 16;
            let cid = u32::from_le_bytes(rewritten[off..off + 4].try_into().unwrap());
            if cid == 0 {
                continue; // unused lane
            }
            if let Some(new) = map_classid(cid) {
                if new != cid {
                    lane_headers_moved += 1;
                } else {
                    lane_headers_kept += 1;
                }
                rewritten[off..off + 4].copy_from_slice(&new.to_le_bytes());
            }
        }
    }

    assert_eq!(
        unknown_keys, 0,
        "an OBO-core artifact must carry only OBO keys"
    );

    // ── verify on the OUTPUT, not on intent ─────────────────────────────────
    // Every byte outside the rewritten classid positions must be identical.
    // Exempt positions within a row: the key classid (bytes 0..4) and each
    // edge-lane header classid (first 4 bytes of every 16-byte lane record).
    let mut foreign_diffs = 0usize;
    for i in 0..n {
        let row = i * NODE_ROW_STRIDE;
        for b in 0..NODE_ROW_STRIDE {
            let is_key_cid = b < 4;
            let in_lane_header = match b.checked_sub(lane0) {
                Some(rel) => rel < lane_count * 16 && rel % 16 < 4,
                None => false,
            };
            if !is_key_cid && !in_lane_header && data[row + b] != rewritten[row + b] {
                foreign_diffs += 1;
            }
        }
    }
    assert_eq!(
        foreign_diffs, 0,
        "the transform touched bytes outside the classid positions"
    );

    std::fs::write(&out, &rewritten).unwrap_or_else(|e| panic!("write {out}: {e}"));

    println!("rekey_domain: {n} rows, order preserved");
    println!("  keys moved {keys_moved} / kept (PATO/RO) {keys_kept}");
    println!("  lane headers moved {lane_headers_moved} / kept {lane_headers_kept}");
    println!("  classid census before -> after:");
    for (cid, cnt) in &before {
        println!("    {cid:#010x}  {cnt}");
    }
    println!("    --");
    for (cid, cnt) in &after {
        println!("    {cid:#010x}  {cnt}");
    }
}
