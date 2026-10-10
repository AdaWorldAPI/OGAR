//! Where OGAR's tests read OGIT: a checkout of `AdaWorldAPI/OGIT` at its
//! moving `master`. OGAR keeps no copy of it. `OGIT_FORK_PATH` names the
//! checkout; without it, `OGIT` next to this repository. lance-graph-ontology
//! reads OGIT the same way.
//!
//! A missing checkout is a setup error, not a skip: a test that reads OGIT
//! must not pass by not running.

use std::path::{Path, PathBuf};

/// The checkout's root, the directory that holds `NTO/` and `SGO/`.
///
/// # Panics
/// If the named checkout, or `../OGIT` when none is named, has no `NTO/`.
pub(crate) fn root() -> PathBuf {
    let (root, source) = match std::env::var_os("OGIT_FORK_PATH") {
        Some(path) => (PathBuf::from(path), "OGIT_FORK_PATH"),
        None => (
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../OGIT"),
            "the sibling checkout",
        ),
    };
    assert!(
        root.join("NTO").is_dir(),
        "no AdaWorldAPI/OGIT checkout at {} ({source}); clone \
         https://github.com/AdaWorldAPI/OGIT next to OGAR or set OGIT_FORK_PATH",
        root.display()
    );
    root
}

/// One file of the checkout, by its path inside OGIT.
///
/// # Panics
/// If the checkout is missing or the file does not read.
pub(crate) fn read(path: &str) -> String {
    let full = root().join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{}: {e}", full.display()))
}
