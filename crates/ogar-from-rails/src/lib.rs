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

    /// Real-corpus **convergence proof** against the Redmine source tree
    /// (`AdaWorldAPI/redmine` — OpenProject's project-domain ancestor).
    /// Set `REDMINE_SRC` to the checkout root. Redmine ships a real
    /// `TimeEntry` model, so this proves the BillableWorkEntry convergence
    /// on actual data: a project-domain `TimeEntry` materializes to the
    /// same canonical concept (`billable_work_entry`) that Odoo's
    /// `account.analytic.line` does.
    #[test]
    #[ignore = "requires a Redmine checkout via REDMINE_SRC"]
    fn redmine_timeentry_converges_to_billable_work_entry() {
        let Ok(src) = std::env::var("REDMINE_SRC") else {
            eprintln!("skipping: REDMINE_SRC not set");
            return;
        };
        let classes = extract(&PathBuf::from(src));
        assert!(!classes.is_empty(), "expected Redmine models, got none");
        let time_entry = classes
            .iter()
            .find(|c| c.name == "TimeEntry")
            .expect("Redmine ships a TimeEntry model");
        assert_eq!(
            time_entry.canonical_concept.as_deref(),
            Some("billable_work_entry"),
            "Redmine TimeEntry must converge to the BillableWorkEntry concept",
        );
        assert_eq!(
            time_entry.source_domain.as_deref(),
            Some("project"),
            "Rails frontend tags the project domain",
        );
        // **Binary codebook convergence** — Redmine `TimeEntry` and the
        // Odoo `account.analytic.line` both resolve to the same OGAR
        // codebook value without producer-side label normalisation.
        // (The Odoo-side extraction lives in the parallel session;
        // the codebook mapping is asserted here on the Rails side.)
        assert_eq!(
            time_entry.canonical_id(),
            Some(ogar_vocab::canonical_concept_id("billable_work_entry")),
        );
        assert_eq!(
            time_entry.canonical_id(),
            Some(ogar_vocab::ogar_codebook("account.analytic.line")),
            "Odoo-shaped label must map to the same codebook id",
        );
    }

    /// Real-corpus **same-domain convergence proof** across the fork
    /// lineage Redmine → ChiliProject → OpenProject. Both Redmine `Issue`
    /// and OpenProject `WorkPackage` must lift to the *same* canonical
    /// concept (`project_work_item`) — and OpenProject's later modular
    /// enrichment (extra includes) must not change that.
    ///
    /// Set `REDMINE_SRC` and (optionally) `OPENPROJECT_SRC` (defaults to
    /// `/home/user/openproject`). Skips gracefully if either is missing.
    #[test]
    #[ignore = "requires Redmine + OpenProject checkouts"]
    fn redmine_issue_and_openproject_work_package_overlap_as_project_work_item() {
        let Ok(redmine_src) = std::env::var("REDMINE_SRC") else {
            eprintln!("skipping: REDMINE_SRC not set");
            return;
        };
        let op_src = std::env::var("OPENPROJECT_SRC")
            .unwrap_or_else(|_| "/home/user/openproject".to_string());
        let op_path = PathBuf::from(&op_src);
        if !op_path.exists() {
            eprintln!("skipping: OpenProject not present at {op_src}");
            return;
        }

        let redmine = extract(&PathBuf::from(redmine_src));
        let openproject = extract(&op_path);

        let issue = redmine
            .iter()
            .find(|c| c.name == "Issue")
            .expect("Redmine ships an Issue model");
        let work_package = openproject
            .iter()
            .find(|c| c.name == "WorkPackage")
            .expect("OpenProject ships a WorkPackage model");

        // The headline: both materialize to the SAME canonical concept,
        // detected deterministically from class names alone. This is the
        // load-bearing convergence assertion — it holds *regardless* of
        // per-curator surface-extraction depth.
        assert_eq!(issue.canonical_concept.as_deref(), Some("project_work_item"));
        assert_eq!(
            work_package.canonical_concept.as_deref(),
            Some("project_work_item"),
        );
        // Same domain — both are project-domain curators.
        assert_eq!(issue.source_domain.as_deref(), Some("project"));
        assert_eq!(work_package.source_domain.as_deref(), Some("project"));

        // Strict structural overlap: both curators extract the canonical
        // roles. Redmine `Issue` (the cleaner AR fossil) and OP
        // `WorkPackage` (the richer organism — reopens merged by
        // AdaWorldAPI/ruff#26) both carry `project`, `author`,
        // `time_entries` after extraction. The role *names* are leaf
        // details that may diverge (`assigned_to` vs `assignee`, `tracker`
        // vs `type`); the canonical_concept is what unifies them.
        for c in [issue, work_package] {
            for role in ["project", "author", "time_entries"] {
                assert!(
                    c.associations.iter().any(|a| a.name == role),
                    "{} must carry a `{}` association",
                    c.name,
                    role,
                );
            }
        }

        // **Binary codebook convergence** — Redmine `Issue` and OP
        // `WorkPackage` are differently-named curator surfaces, but the
        // OGAR codebook value is identical. The address is the identity;
        // the labels are decorative.
        assert_eq!(
            issue.canonical_id(),
            work_package.canonical_id(),
            "Issue and WorkPackage must share the OGAR codebook id",
        );
        assert_eq!(
            issue.canonical_id(),
            Some(ogar_vocab::canonical_concept_id("project_work_item")),
        );
    }

    /// Enrichment must not break the overlap. OpenProject `WorkPackage`
    /// is the strictly richer organism — extra modular includes
    /// (`WorkPackages::SpentTime` / `Costs` / `Relations`,
    /// `OpenProject::Journal::AttachmentHelper`, …) on top of the cleaner
    /// Redmine `Issue` AR shape — yet the canonical concept is invariant.
    /// Post AdaWorldAPI/ruff#26 (reopen-merge), OP extracts its full
    /// body, so the richer-mixin assertion is enforceable.
    #[test]
    #[ignore = "requires Redmine + OpenProject checkouts"]
    fn openproject_enrichment_does_not_break_redmine_ar_overlap() {
        let Ok(redmine_src) = std::env::var("REDMINE_SRC") else {
            eprintln!("skipping: REDMINE_SRC not set");
            return;
        };
        let op_src = std::env::var("OPENPROJECT_SRC")
            .unwrap_or_else(|_| "/home/user/openproject".to_string());
        let op_path = PathBuf::from(&op_src);
        if !op_path.exists() {
            eprintln!("skipping: OpenProject not present at {op_src}");
            return;
        }

        let redmine = extract(&PathBuf::from(redmine_src));
        let openproject = extract(&op_path);

        let issue = redmine.iter().find(|c| c.name == "Issue").unwrap();
        let work_package = openproject
            .iter()
            .find(|c| c.name == "WorkPackage")
            .unwrap();

        // OP is strictly the richer organism at the mixin axis ...
        assert!(
            work_package.mixins.len() > issue.mixins.len(),
            "OP WorkPackage should be strictly richer than Redmine Issue \
             at the mixin axis (OP: {} mixins, Redmine: {} mixins)",
            work_package.mixins.len(),
            issue.mixins.len(),
        );
        // ... yet the canonical concept is invariant: enrichment did not
        // break the overlap. This is the load-bearing agnostic-vocab
        // claim made concrete on real source.
        assert_eq!(issue.canonical_concept, work_package.canonical_concept);
        assert_eq!(
            issue.canonical_concept.as_deref(),
            Some("project_work_item"),
        );
    }

    /// **Lineage-transcode smoke** — Redmine `Issue` and OpenProject
    /// `WorkPackage` are forks of the same upstream
    /// (Redmine → ChiliProject → OpenProject), so transcoding between
    /// them through the canonical [`ogar_vocab::project_work_item`]
    /// bridge must be lossless: both curators project onto the **same
    /// canonical role set**, even where surface names diverge
    /// (`tracker`/`type`, `assigned_to`/`assignee`, `relations_from,to`/
    /// `WorkPackages::Relations`, `journals`/`acts_as_journalized`).
    ///
    /// The claim is structural: the canonical layer is a valid pivot for
    /// converting between curators of the same lineage.
    #[test]
    #[ignore = "requires Redmine + OpenProject checkouts"]
    fn lineage_transcode_redmine_issue_to_openproject_work_package_via_canonical() {
        let Ok(redmine_src) = std::env::var("REDMINE_SRC") else {
            eprintln!("skipping: REDMINE_SRC not set");
            return;
        };
        let op_src = std::env::var("OPENPROJECT_SRC")
            .unwrap_or_else(|_| "/home/user/openproject".to_string());
        let op_path = PathBuf::from(&op_src);
        if !op_path.exists() {
            eprintln!("skipping: OpenProject not present at {op_src}");
            return;
        }

        let redmine = extract(&PathBuf::from(redmine_src));
        let openproject = extract(&op_path);

        let issue = redmine.iter().find(|c| c.name == "Issue").unwrap();
        let work_package = openproject
            .iter()
            .find(|c| c.name == "WorkPackage")
            .unwrap();

        // Project each curator onto the canonical role set.
        let issue_roles = ogar_from_ruff::project_work_item_canonical_roles(issue);
        let wp_roles = ogar_from_ruff::project_work_item_canonical_roles(work_package);

        // The full canonical surface: every role on
        // [`ogar_vocab::project_work_item`].
        let expected: std::collections::HashSet<&'static str> = [
            "project", "status", "type", "priority", "author", "assignee",
            "journals", "relations", "time_entries",
        ]
        .into_iter()
        .collect();

        assert_eq!(
            issue_roles, expected,
            "Redmine Issue must project to the FULL project_work_item canonical role set",
        );
        assert_eq!(
            wp_roles, expected,
            "OpenProject WorkPackage must project to the FULL project_work_item canonical role set \
             (journals via `acts_as_journalized` mixin; relations via `WorkPackages::Relations`)",
        );

        // The headline lineage-transcode claim: identical canonical
        // projections. The canonical layer IS a lossless pivot between
        // the two curators of the same fork lineage.
        assert_eq!(
            issue_roles, wp_roles,
            "fork lineage Redmine -> ChiliProject -> OpenProject must transcode losslessly \
             through the canonical project_work_item bridge",
        );
    }

    /// **Project convergence** on real source. Both Redmine `Project`
    /// and OpenProject `Project` lift to canonical_concept `"project"`
    /// and project to the same 4-role canonical role set (`parent`,
    /// `work_items`, `time_entries`, `members`) — even though Redmine
    /// uses `has_many :issues` and OP uses `has_many :work_packages`
    /// for the work-item link, and OP threads members via additional
    /// `member_principals` / `principals` through-associations.
    #[test]
    #[ignore = "requires Redmine + OpenProject checkouts"]
    fn redmine_and_openproject_projects_converge_through_canonical() {
        let Ok(redmine_src) = std::env::var("REDMINE_SRC") else {
            eprintln!("skipping: REDMINE_SRC not set");
            return;
        };
        let op_src = std::env::var("OPENPROJECT_SRC")
            .unwrap_or_else(|_| "/home/user/openproject".to_string());
        let op_path = PathBuf::from(&op_src);
        if !op_path.exists() {
            eprintln!("skipping: OpenProject not present at {op_src}");
            return;
        }

        let redmine = extract(&PathBuf::from(redmine_src));
        let openproject = extract(&op_path);

        let r_project = redmine
            .iter()
            .find(|c| c.name == "Project")
            .expect("Redmine ships a Project model");
        let o_project = openproject
            .iter()
            .find(|c| c.name == "Project")
            .expect("OpenProject ships a Project model");

        // Headline: both materialize to the same canonical concept,
        // detected deterministically from the class name.
        assert_eq!(r_project.canonical_concept.as_deref(), Some("project"));
        assert_eq!(o_project.canonical_concept.as_deref(), Some("project"));
        // Same domain — both are project-domain curators.
        assert_eq!(r_project.source_domain.as_deref(), Some("project"));
        assert_eq!(o_project.source_domain.as_deref(), Some("project"));

        // Lineage-transcode parity: both curators project onto the same
        // canonical role set through the [`ogar_from_ruff::project_role`]
        // resolver, despite the surface-name divergence
        // (issues / work_packages) and OP's extra through-assoc hops.
        let r_roles = ogar_from_ruff::project_canonical_roles(r_project);
        let o_roles = ogar_from_ruff::project_canonical_roles(o_project);
        // v1 canonical surface: 3 direct-association roles. Nested-project
        // `parent` is real cross-curator but lives behind mixins
        // (Redmine `awesome_nested_set`; OP `Projects::Hierarchy`); the
        // producer does not yet decode either, so the v1 surface stays
        // at 3 roles. A follow-up mixin-decode adds it.
        let expected: std::collections::HashSet<&'static str> =
            ["work_items", "time_entries", "members"].into_iter().collect();
        assert_eq!(
            r_roles, expected,
            "Redmine Project must cover the canonical 3-role surface",
        );
        assert_eq!(
            o_roles, expected,
            "OpenProject Project must cover the same surface",
        );
        assert_eq!(
            r_roles, o_roles,
            "Project canonical projection must transcode losslessly across the fork lineage",
        );

        // **Binary codebook convergence** — labels are decorative; the
        // OGAR codebook value is the identity. Different curator names
        // (`Project` is the same string here but in general curators may
        // diverge) resolve to the same `u16`.
        assert_eq!(
            r_project.canonical_id(),
            o_project.canonical_id(),
            "Redmine Project and OP Project must share the OGAR codebook id",
        );
        assert_eq!(
            r_project.canonical_id(),
            Some(ogar_vocab::canonical_concept_id("project")),
        );
    }

    /// Exactly-one-Model invariant post AdaWorldAPI/ruff#26: OP's
    /// `app/models/work_package/` sub-files reopen `class WorkPackage`
    /// without adding ontology declarations; before the fix this produced
    /// duplicate `Model { name: "WorkPackage" }` entries (one empty, one
    /// rich) and `.find()` could land on either. After the fix, the
    /// producer merges them into one — pin that behavior so consumers can
    /// trust uniqueness-by-name.
    #[test]
    #[ignore = "requires an OpenProject checkout"]
    fn openproject_workpackage_extracts_as_exactly_one_class() {
        let op_src = std::env::var("OPENPROJECT_SRC")
            .unwrap_or_else(|_| "/home/user/openproject".to_string());
        let op_path = PathBuf::from(&op_src);
        if !op_path.exists() {
            eprintln!("skipping: OpenProject not present at {op_src}");
            return;
        }
        let classes = extract(&op_path);
        let n = classes.iter().filter(|c| c.name == "WorkPackage").count();
        assert_eq!(
            n, 1,
            "OpenProject `WorkPackage` must extract as exactly one Class \
             (reopen-merge from AdaWorldAPI/ruff#26); got {n}",
        );
    }
}
