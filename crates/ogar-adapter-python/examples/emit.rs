//! Write the generated Python module to a file.
//!
//! Usage:
//! ```text
//! cargo run -p ogar-adapter-python --example emit -- <output_path> [module_name]
//! ```
//! `module_name` defaults to `ogar_capability_surface` (documentation-only —
//! see [`ogar_adapter_python::emit_python`]).

use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(output_path) = args.next() else {
        eprintln!("usage: emit <output_path> [module_name]");
        return ExitCode::from(2);
    };
    let module_name = args
        .next()
        .unwrap_or_else(|| "ogar_capability_surface".to_string());

    let content = ogar_adapter_python::emit_python(&module_name);
    if let Err(e) = fs::write(&output_path, content) {
        eprintln!("failed to write {output_path}: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("wrote {output_path} (module {module_name})");
    ExitCode::SUCCESS
}
