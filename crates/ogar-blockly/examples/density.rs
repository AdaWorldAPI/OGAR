//! Layout + density accounting for a block function node.
//!
//! Prints the byte budget and the amortized cost per operation at several
//! occupancies, so the density claims in `docs/DISCOVERY-MAP.md`
//! `D-BLOCKS-PALETTE` can be re-measured rather than trusted.
//!
//! ```sh
//! cargo run -p ogar-blockly --example density
//! ```

use ogar_blockly::{
    CLASSID_BYTES, CONTENT_SLOTS, FunctionBody, OPS_PER_FUNCTION, PAYLOAD_BYTES_PER_SLOT,
    SLOT_STRIDE, VALUE_SLAB_LEN,
};

/// Bytes of a whole node: key(16) + edges(16) + value(480).
const NODE_BYTES: usize = 512;

fn main() {
    println!("── node layout ──");
    println!("  node                {NODE_BYTES} B  = 32 × {SLOT_STRIDE} B slots");
    println!("  key                  16 B  (slot 0)");
    println!("  edges                16 B  (slot 1)");
    println!("  value slab          {VALUE_SLAB_LEN} B  (slots 2..31 = {CONTENT_SLOTS} facets)");
    println!(
        "    classid overhead  {} B  ({CONTENT_SLOTS} × {CLASSID_BYTES}, interleaved)",
        CONTENT_SLOTS * CLASSID_BYTES
    );
    println!(
        "    operation bytes   {OPS_PER_FUNCTION} B  ({CONTENT_SLOTS} × {PAYLOAD_BYTES_PER_SLOT})"
    );

    println!("\n── in-memory vs wire ──");
    println!(
        "  FunctionBody        {} B  ([u8; {OPS_PER_FUNCTION}] + u16 len)",
        size_of::<FunctionBody>()
    );
    println!(
        "  wire payload        {OPS_PER_FUNCTION} B  (len is NOT written; NOP padding is the signal)"
    );

    println!("\n── amortized cost per operation (whole {NODE_BYTES} B node) ──");
    for ops in [OPS_PER_FUNCTION, 180, 90, 30] {
        let per_op = NODE_BYTES as f64 / ops as f64;
        let occupancy = 100.0 * ops as f64 / OPS_PER_FUNCTION as f64;
        println!("  {ops:3} ops ({occupancy:5.1}% full)  {per_op:6.3} B/op");
    }

    println!("\n── the operations are NOT contiguous in the slab ──");
    for i in [0usize, 11, 12, 23, 24, OPS_PER_FUNCTION - 1] {
        println!(
            "  op {i:3} → slab offset {:3}  (facet {:2}, byte {:2} of its payload lane)",
            FunctionBody::slab_offset(i),
            i / PAYLOAD_BYTES_PER_SLOT,
            i % PAYLOAD_BYTES_PER_SLOT
        );
    }
}
