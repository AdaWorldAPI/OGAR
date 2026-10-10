//! `ogit_tables [<ogit-root>]`: loads the OGIT tables and prints their sizes
//! and how long the load took. The root defaults to `OGIT_FORK_PATH`, then to
//! `../OGIT`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use ogar_action_handler::ogit::{OgitTables, Table};

fn main() -> ExitCode {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("OGIT_FORK_PATH").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("../OGIT"));
    let start = Instant::now();
    let tables = match OgitTables::load(&root) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", root.display());
            return ExitCode::FAILURE;
        }
    };
    let elapsed = start.elapsed();
    for table in [
        Table::Type,
        Table::Attribute,
        Table::TypeAttr,
        Table::Verb,
        Table::Allowed,
    ] {
        println!("{table:?}\t{} rows", tables.rows(table));
    }
    println!("domains\t{}", tables.domains().len());
    println!("rejected files\t{}", tables.rejected().len());
    println!("unresolved references\t{}", tables.unresolved().len());
    println!("load\t{:.1} ms", elapsed.as_secs_f64() * 1000.0);
    ExitCode::SUCCESS
}
