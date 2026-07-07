//! Write the generated C# class library to a file.
//!
//! Usage:
//! ```text
//! cargo run -p ogar-adapter-csharp --example emit -- <output_path> [namespace]
//! ```
//! `namespace` defaults to `Ogar.CapabilitySurface` and becomes the
//! emitted file's real `namespace` declaration (see
//! [`ogar_adapter_csharp::emit_csharp`]).

use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(output_path) = args.next() else {
        eprintln!("usage: emit <output_path> [namespace]");
        return ExitCode::from(2);
    };
    let namespace = args
        .next()
        .unwrap_or_else(|| "Ogar.CapabilitySurface".to_string());

    let content = ogar_adapter_csharp::emit_csharp(&namespace);
    if let Err(e) = fs::write(&output_path, content) {
        eprintln!("failed to write {output_path}: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("wrote {output_path} (namespace {namespace})");
    ExitCode::SUCCESS
}
