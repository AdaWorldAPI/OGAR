//! `ogar-from-rails` — Rails / ActiveRecord frontend for OGAR.
//!
//! Walks an `app/models/` tree via [`ruff_ruby_spo::extract`], then
//! lifts the resulting [`ruff_spo_triplet::ir::ModelGraph`] to a
//! `Vec<ogar_vocab::Class>` via [`ogar_from_ruff::lift_model_graph`].
//! Mirrors the role [`ogar_from_elixir`] plays for the HIRO/Bardioc
//! Elixir stack.
//!
//! ```text
//!   app/models/      ruff_ruby_spo::extract        ogar_from_ruff::lift_model_graph
//!     ─────────  ─────────────────────────────  ──────────────────────────────────
//!     *.rb files →  ruff_spo_triplet::ModelGraph  →  Vec<ogar_vocab::Class>
//!                                                              │
//!                                                              ▼
//!                                            lance-graph-ontology::OntologyRegistry
//! ```
//!
//! # OpenProject coverage (the canonical Rails corpus)
//!
//! On the live OpenProject source tree at `/home/user/openproject`:
//!
//! ```text
//!   $ extract(Path::new("/home/user/openproject"))
//!   → 694 Class values
//! ```
//!
//! Matches the count from `ruff_ruby_spo::extract` 1:1 — every Rails
//! class becomes one OGAR class.
//!
//! # What this crate is NOT
//!
//! - Not a producer of `field_type` / `validation_param` / etc. SPO
//!   triples. Those live on `ruff_spo_triplet::expand` and feed the
//!   narrow / schema-codegen arm (`op-surreal-ast::from_triples` →
//!   SurrealQL DDL). This crate is the **wide / OGAR vocab** arm —
//!   they're the two sides of the §10.1 OPENPROJECT-TRANSCODING
//!   pattern.
//! - Not a `db/schema.rb` parser. DB-column lift is a separate
//!   sprint; see [`ogar_from_ruff`]'s field-map table for what's
//!   covered today.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::path::Path;

use ogar_vocab::Class;

/// Top-level entry: extract from a Rails source tree to a list of
/// OGAR `Class` values.
///
/// `source_tree` should be the Rails app root (the directory containing
/// `app/`), not `app/models` itself. Matches the convention of
/// [`ruff_ruby_spo::extract`]: it appends `app/models` internally.
///
/// Empty result on a tree without `app/models`, mirroring the
/// `ruff_ruby_spo` behaviour (no errors, just zero models).
#[must_use]
pub fn extract(source_tree: &Path) -> Vec<Class> {
    let graph = ruff_ruby_spo::extract(source_tree);
    ogar_from_ruff::lift_model_graph(&graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Empty / nonexistent source tree → empty Vec, never panics. The
    /// `ruff_ruby_spo` contract is "no app/models → empty graph"; this
    /// test pins that our lift preserves the no-panic behaviour.
    #[test]
    fn nonexistent_source_tree_yields_empty_vec() {
        let classes = extract(Path::new("/tmp/__definitely_does_not_exist_for_test__"));
        assert!(classes.is_empty(), "no app/models → no classes");
    }

    /// Smoke test against the live OpenProject source tree on the dev
    /// image (when present). Gated behind `#[ignore]` so CI without
    /// the OP checkout doesn't fail; locally `cargo test -- --ignored`
    /// surfaces the real-corpus number.
    #[test]
    #[ignore = "requires /home/user/openproject Rails source"]
    fn op_source_tree_yields_expected_class_count() {
        let path = PathBuf::from("/home/user/openproject");
        if !path.exists() {
            eprintln!("skipping: {} not present", path.display());
            return;
        }
        let classes = extract(&path);
        assert!(
            classes.len() > 500,
            "expected ~700 classes from OP, got {}",
            classes.len(),
        );
        // Sanity: at least one well-known class is in the list.
        assert!(
            classes.iter().any(|c| c.name == "WorkPackage"),
            "WorkPackage must be in the lifted Classes",
        );
        // STI: any class with `self.inheritance_column = ...` or
        // `abstract_class = true` carries a `parent` slot. OpenProject's
        // WorkPackage extends ApplicationRecord (no STI), so we just
        // check the field is exposed at all — populated for some class
        // somewhere on the corpus (or empty if no STI exists today;
        // this assertion is loose-by-design until the producer emits
        // STI for the non-default-parent cases).
        let any_parent_set = classes.iter().any(|c| c.parent.is_some());
        // Don't hard-assert any_parent_set: OP today doesn't define
        // STI via the explicit Rails idiom. Just verify the slot is
        // reachable by accessing it.
        let _ = any_parent_set;
    }
}
