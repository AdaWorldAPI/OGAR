//! Layout + density accounting for a block function node.
//!
//! Prints the byte budget and the amortized cost per call at several lane
//! shapes and occupancies, so the density claims in `docs/DISCOVERY-MAP.md`
//! `D-BLOCKS-PALETTE` can be re-measured rather than trusted.
//!
//! ```sh
//! cargo run -p ogar-blockly --example density
//! ```

use ogar_blockly::{
    CLASSID_BYTES, CONTENT_SLOTS, FunctionBody, LaneShape, PAYLOAD_BYTES_PER_SLOT, SLOT_STRIDE,
    VALUE_SLAB_LEN,
};

/// Bytes of a whole node: key(16) + reserved(16) + value(480).
const NODE_BYTES: usize = 512;

fn main() {
    println!("── node layout ──");
    println!("  node                {NODE_BYTES} B  = 32 × {SLOT_STRIDE} B slots");
    println!("  key                  16 B  (slot 0)");
    println!("  reserved             16 B  (slot 1 — zeroed; retired edge-block NOT revived)");
    println!("  value slab          {VALUE_SLAB_LEN} B  (slots 2..31 = {CONTENT_SLOTS} lanes)");
    println!(
        "    classid overhead  {} B  ({CONTENT_SLOTS} × {CLASSID_BYTES}, interleaved)",
        CONTENT_SLOTS * CLASSID_BYTES
    );
    println!(
        "    call bytes        {} B  ({CONTENT_SLOTS} × {PAYLOAD_BYTES_PER_SLOT})",
        CONTENT_SLOTS * PAYLOAD_BYTES_PER_SLOT
    );

    println!("\n── lane shapes: same 360 bytes, three carvings ──");
    for shape in LaneShape::ALL {
        println!(
            "  {shape:?}: {} B/call ({} immediate{}) → {} calls/lane, {} calls/node",
            shape.bytes_per_call(),
            shape.values_per_call(),
            if shape.values_per_call() == 1 {
                ""
            } else {
                "s"
            },
            shape.calls_per_lane(),
            shape.calls_per_function()
        );
    }

    println!("\n── in-memory vs wire ──");
    println!(
        "  FunctionBody        {} B  ([u8; 360] + u16 len + LaneShape)",
        size_of::<FunctionBody>()
    );
    println!(
        "  wire payload        360 B  (len & shape NOT written: padding is length, classid is shape)"
    );

    println!("\n── amortized cost per call (whole {NODE_BYTES} B node) ──");
    for shape in LaneShape::ALL {
        let cap = shape.calls_per_function();
        for div in [1usize, 2, 4] {
            let calls = cap / div;
            let per_call = NODE_BYTES as f64 / calls as f64;
            let occupancy = 100.0 / div as f64;
            println!("  {shape:?} {calls:3} calls ({occupancy:5.1}% full)  {per_call:6.3} B/call");
        }
    }

    println!("\n── calls are NOT contiguous in the slab ──");
    for shape in [LaneShape::Pairs, LaneShape::Quads] {
        let cpl = shape.calls_per_lane();
        for i in [0usize, cpl - 1, cpl, shape.calls_per_function() - 1] {
            println!(
                "  {shape:?} call {i:3} → slab offset {:3}  (lane {:2}, byte {:2} of its payload)",
                FunctionBody::call_slab_offset(shape, i),
                i / cpl,
                (i % cpl) * shape.bytes_per_call()
            );
        }
    }
}
