//! Python compile+run parity loop.
//!
//! Emits the generated module to a temp dir under `/tmp`, verifies it
//! compiles (`python3 -m py_compile`), then imports it via `python3 -c`
//! and compares its `dump()` output against the live `ogar_vocab` ground
//! truth ([`ogar_adapter_python::ground_truth`]). A second test proves
//! the facet decoder specifically: Rust builds a known 16-byte facet by
//! hand (per the documented byte layout, independent of the `Facet`
//! type), Python decodes those exact bytes, and the fields must match.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MODULE_NAME: &str = "ogar_capability_surface";

/// A unique temp dir under `/tmp` (or `$TMPDIR`) for one test run.
fn unique_tmp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let dir = std::env::temp_dir().join(format!(
        "ogar-adapter-python-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp dir under /tmp");
    dir
}

fn python3() -> Command {
    Command::new("python3")
}

fn write_module(dir: &Path) -> PathBuf {
    let module_path = dir.join(format!("{MODULE_NAME}.py"));
    fs::write(&module_path, ogar_adapter_python::emit_python(MODULE_NAME))
        .expect("write emitted module");
    module_path
}

#[test]
fn emitted_module_py_compiles_and_dump_matches_ground_truth() {
    let dir = unique_tmp_dir("compile-dump");
    let module_path = write_module(&dir);

    // `python3 -m py_compile <path>` — exit 0 required.
    let compile = python3()
        .args(["-m", "py_compile"])
        .arg(&module_path)
        .output()
        .expect("spawn python3 -m py_compile");
    assert!(
        compile.status.success(),
        "python3 -m py_compile failed (exit {:?}):\nstdout: {}\nstderr: {}",
        compile.status.code(),
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr),
    );

    // `python3 -c` importing the module and dumping (a) all (name,id)
    // pairs sorted, (b) capability names + subject ids, (c) the green
    // resolve_hotplug result, (d) each of the five drift arms fired —
    // all encoded in dump()'s single canonical text format.
    let run = python3()
        .env("PYTHONPATH", &dir)
        .arg("-c")
        .arg(format!(
            "import {MODULE_NAME} as m; print(m.dump(), end='')"
        ))
        .output()
        .expect("spawn python3 -c ... dump()");
    assert!(
        run.status.success(),
        "python3 -c dump() failed (exit {:?}):\nstdout: {}\nstderr: {}",
        run.status.code(),
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    );
    let dump = String::from_utf8_lossy(&run.stdout).into_owned();

    // Compare every dump line against the live ogar_vocab ground truth
    // (string compare, both sides already produced in sorted/canonical
    // order by construction).
    ogar_adapter_python::ground_truth::assert_dump_matches("python", &dump);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn facet_bytes_built_in_rust_decode_correctly_in_python() {
    let dir = unique_tmp_dir("facet");
    write_module(&dir);

    // Rust builds a known 16-byte facet BY HAND, straight from the
    // documented layout (facet.rs / ruff_spo_address::Facet doc):
    //   byte[0..4)    = classid (u32 LE)
    //   byte[4 + 2t]  = is_a_chain[t]   (lo)
    //   byte[5 + 2t]  = part_of_chain[t] (hi)
    // This is independent of ogar-adapter-python's own `Facet` type —
    // it proves Python decodes bytes Rust produced, not just bytes
    // Python produced itself.
    let classid =
        ogar_vocab::app::render_classid(0x0002, ogar_vocab::class_ids::COMMERCIAL_DOCUMENT);
    let is_a: [u8; 6] = [0x21, 0x22, 0x23, 0x24, 0x25, 0x26];
    let part_of: [u8; 6] = [0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6];
    let mut bytes = [0u8; 16];
    bytes[0..4].copy_from_slice(&classid.to_le_bytes());
    for t in 0..6 {
        bytes[4 + 2 * t] = is_a[t];
        bytes[5 + 2 * t] = part_of[t];
    }
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();

    let script = format!(
        "import {MODULE_NAME} as m\n\
         f = m.Facet.from_bytes(bytes.fromhex({hex:?}))\n\
         print(f'{{f.classid:08X}}')\n\
         print(','.join(f'{{b:02X}}' for b in f.is_a_chain))\n\
         print(','.join(f'{{b:02X}}' for b in f.part_of_chain))\n"
    );
    let run = python3()
        .env("PYTHONPATH", &dir)
        .arg("-c")
        .arg(&script)
        .output()
        .expect("spawn python3 -c ... facet decode");
    assert!(
        run.status.success(),
        "python3 facet decode failed (exit {:?}):\nstdout: {}\nstderr: {}",
        run.status.code(),
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    );
    let got = String::from_utf8_lossy(&run.stdout).into_owned();
    let expected = format!(
        "{classid:08X}\n{}\n{}",
        is_a.iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(","),
        part_of
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(","),
    );
    assert_eq!(
        got.trim_end(),
        expected.trim_end(),
        "python-decoded facet fields do not match the Rust-built bytes"
    );

    let _ = fs::remove_dir_all(&dir);
}
