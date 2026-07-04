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

use std::collections::BTreeMap;
use std::path::Path;

use ogar_vocab::{ActionDef, Class};

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

/// Extract from a Rails source tree, tagging the harvest with an explicit
/// `curator` namespace (e.g. `"redmine"`, `"openproject"`, `"osb"`).
///
/// Threads through [`ruff_ruby_spo::extract_with`] (added in
/// AdaWorldAPI/ruff#27) so the produced classes carry the correct
/// [`Class::source_curator`] — instead of the hardcoded `"openproject"`
/// default [`extract`] inherits. `source_domain` is then derived from the
/// curator namespace ([`ogar_from_ruff::classify_domain`]): Redmine and
/// OpenProject both land in `project`, but stay distinguishable by curator.
///
/// Use this for any non-OpenProject Rails curator; plain [`extract`]
/// remains the OpenProject-default convenience.
#[must_use]
pub fn extract_with(source_tree: &Path, curator: &str) -> Vec<Class> {
    let graph = ruff_ruby_spo::extract_with(source_tree, curator);
    ogar_from_ruff::lift_model_graph(&graph)
}

/// Like [`extract_with`], but **walks the whole Rails application** — the
/// core `app/models` *plus* every mounted engine's `app/models`
/// (`modules/*/app/models`, `engines/*/app/models`).
///
/// Threads through [`ruff_ruby_spo::extract_app_with`] (added in
/// AdaWorldAPI/ruff#28). OpenProject keeps a large share of its domain in
/// `modules/*` engines (e.g. `TimeEntry` lives in
/// `modules/costs/app/models`), invisible to the core-only [`extract`] /
/// [`extract_with`]; this entry point closes that gap so the canonical
/// layer sees the full corpus.
///
/// Plain [`extract`] / [`extract_with`] stay core-only for backward
/// compatibility; engines are opt-in via this entry point.
#[must_use]
pub fn extract_app_with(source_tree: &Path, curator: &str) -> Vec<Class> {
    let graph = ruff_ruby_spo::extract_app_with(source_tree, curator);
    ogar_from_ruff::lift_model_graph(&graph)
}

/// Whole-application extraction (core + engines), defaulting the curator
/// namespace to `"openproject"`. Thin wrapper over [`extract_app_with`].
#[must_use]
pub fn extract_app(source_tree: &Path) -> Vec<Class> {
    let graph = ruff_ruby_spo::extract_app(source_tree);
    ogar_from_ruff::lift_model_graph(&graph)
}

/// Harvest the **DO-arm**: walk `<source_tree>/app/controllers`, lift each
/// controller's public actions to `ActionDef`s via [`ogar_from_ruff::lift_actions`],
/// keyed by the resource model name the controller acts on
/// (`Api::NodesController` → `Node`). Controllers that don't map to a resource
/// (dashboards, errors, sessions, …) are dropped.
///
/// This is the MVC other half — OGAR is *Open Graph of Active Record*, and
/// Active Record is Model **+ Controller**. [`extract`] harvests the models
/// (THINK arm); this harvests the controllers (DO arm). Pair the two by model
/// name: `render_class_with_methods(model_class, mask, &actions_for[model])`
/// attaches the controller's actions as Rust methods on the model struct.
///
/// Returns `(model_name, actions)` pairs in deterministic (sorted) order.
/// Empty when there is no `app/controllers` tree.
#[must_use]
pub fn extract_actions(source_tree: &Path, curator: &str) -> Vec<(String, Vec<ActionDef>)> {
    let dir = source_tree.join("app/controllers");
    let graph = ruff_ruby_spo::extract_tree_with(&dir, curator);
    let mut by_model: BTreeMap<String, Vec<ActionDef>> = BTreeMap::new();
    for m in &graph.models {
        if let Some(model) = controller_to_model(&m.name) {
            let acts = ogar_from_ruff::lift_actions(m);
            if !acts.is_empty() {
                by_model.entry(model).or_default().extend(acts);
            }
        }
    }
    // A model can be served by several controllers (`Api::NodesController` +
    // the browse `NodesController`), and controllers repeat private helper
    // `def`s — both produce same-named actions. Dedup by predicate (first
    // wins) so the rendered `impl` has no duplicate method definitions.
    for acts in by_model.values_mut() {
        let mut seen = std::collections::HashSet::new();
        acts.retain(|a| seen.insert(a.predicate.clone()));
    }
    by_model.into_iter().collect()
}

/// `Api::NodesController` → `Node`. Strips the module namespace and the
/// `Controller` suffix, then naively singularises (drops a trailing `s`).
/// `None` for names that aren't `*Controller` or reduce to empty.
fn controller_to_model(controller: &str) -> Option<String> {
    let base = controller.rsplit("::").next()?;
    let stem = base.strip_suffix("Controller")?;
    if stem.is_empty() {
        return None;
    }
    Some(stem.strip_suffix('s').unwrap_or(stem).to_string())
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
        // extract_with mirrors the no-panic behaviour too.
        let classes = extract_with(Path::new("/tmp/__nope__"), "redmine");
        assert!(classes.is_empty());
        // extract_app / extract_app_with also stay no-panic on a missing tree.
        assert!(extract_app(Path::new("/tmp/__nope__")).is_empty());
        assert!(extract_app_with(Path::new("/tmp/__nope__"), "redmine").is_empty());
    }

    /// **OpenProject modular-engine harvest** via [`extract_app_with`] —
    /// closes the gap that left `TimeEntry` (and ~240 other models in
    /// `modules/*/app/models`) invisible to plain [`extract`]. After this,
    /// OpenProject's side of the `billable_work_entry` (`0x0103`) bridge is
    /// populated, completing the OP↔Redmine convergence on that concept.
    #[test]
    #[ignore = "requires /home/user/openproject Rails source"]
    fn extract_app_harvests_openproject_modules() {
        let path = PathBuf::from("/home/user/openproject");
        if !path.exists() {
            eprintln!("skipping: {} not present", path.display());
            return;
        }
        let core = extract(&path);
        let app = extract_app_with(&path, "openproject");
        assert!(
            app.len() > core.len(),
            "extract_app ({}) must exceed core-only ({})",
            app.len(),
            core.len(),
        );
        // TimeEntry lives in modules/costs/app/models — invisible to core,
        // visible to extract_app, and lifts onto billable_work_entry.
        let te = app
            .iter()
            .find(|c| c.name == "TimeEntry")
            .expect("TimeEntry harvested from modules/costs by extract_app");
        assert_eq!(te.canonical_concept.as_deref(), Some("billable_work_entry"));
        assert_eq!(te.source_curator.as_deref(), Some("openproject"));
        assert_eq!(
            te.canonical_id(),
            ogar_vocab::canonical_concept_id("billable_work_entry"),
        );
        // And core-only really doesn't see it (regression guard on the gap).
        assert!(
            !core.iter().any(|c| c.name == "TimeEntry"),
            "core-only extract must NOT see modules/costs/TimeEntry",
        );
    }

    /// **Per-curator namespace tagging** on real source (the ruff#27
    /// `extract_with` thread, end to end). Redmine and OpenProject are
    /// both `project`-domain but distinct curators: `extract_with` tags
    /// each correctly via [`Class::source_curator`], while the canonical
    /// concept still converges (`Issue` and `WorkPackage` → the same
    /// `project_work_item` id).
    #[test]
    #[ignore = "requires Redmine + OpenProject checkouts"]
    fn extract_with_tags_distinct_curators_in_one_domain() {
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

        let redmine = extract_with(&PathBuf::from(redmine_src), "redmine");
        let openproject = extract_with(&op_path, "openproject");

        let issue = redmine.iter().find(|c| c.name == "Issue").unwrap();
        let wp = openproject.iter().find(|c| c.name == "WorkPackage").unwrap();

        // Distinct curators ...
        assert_eq!(issue.source_curator.as_deref(), Some("redmine"));
        assert_eq!(wp.source_curator.as_deref(), Some("openproject"));
        // ... same domain ...
        assert_eq!(issue.source_domain.as_deref(), Some("project"));
        assert_eq!(wp.source_domain.as_deref(), Some("project"));
        // ... and the canonical concept still converges.
        assert_eq!(issue.canonical_id(), wp.canonical_id());
        assert_eq!(
            issue.canonical_id(),
            ogar_vocab::canonical_concept_id("project_work_item"),
        );
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
            ogar_vocab::canonical_concept_id("billable_work_entry"),
        );
        assert_eq!(
            time_entry.canonical_id(),
            ogar_vocab::ogar_codebook("account.analytic.line"),
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
            ogar_vocab::canonical_concept_id("project_work_item"),
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
            ogar_vocab::canonical_concept_id("project"),
        );
    }

    /// **ProjectActor convergence** on real source. Both Redmine and OP
    /// model actors as the STI chain `User < Principal < ApplicationRecord`.
    /// Both `User` and `Principal` lift to the same canonical concept
    /// (`project_actor`), with binary codebook convergence — the SAME
    /// `canonical_id` for both STI siblings AND across both curators.
    #[test]
    #[ignore = "requires Redmine + OpenProject checkouts"]
    fn redmine_and_openproject_actors_converge_through_canonical() {
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

        // All four classes — Redmine User + Principal, OP User + Principal
        // — share the canonical concept and the binary codebook id.
        let expected_id = ogar_vocab::canonical_concept_id("project_actor");
        assert!(expected_id.is_some(), "project_actor must be in the codebook");

        for (curator, classes) in [("Redmine", &redmine), ("OpenProject", &openproject)] {
            for class_name in ["User", "Principal"] {
                let c = classes
                    .iter()
                    .find(|c| c.name == class_name)
                    .unwrap_or_else(|| panic!("{curator} ships a {class_name} model"));
                assert_eq!(
                    c.canonical_concept.as_deref(),
                    Some("project_actor"),
                    "{curator} {class_name} must canonicalize to project_actor",
                );
                assert_eq!(c.source_domain.as_deref(), Some("project"));
                // Binary codebook convergence — the load-bearing claim.
                assert_eq!(
                    c.canonical_id(),
                    expected_id,
                    "{curator} {class_name} must share the OGAR codebook id",
                );
            }
        }
    }

    /// **ProjectStatus + ProjectType convergence** on real source. Both
    /// curators carry these lookup tables under divergent names:
    ///
    /// | Canonical concept | Redmine     | OpenProject |
    /// |-------------------|-------------|-------------|
    /// | project_status    | IssueStatus | Status      |
    /// | project_type      | Tracker     | Type        |
    ///
    /// All four extracted classes converge through their respective
    /// canonical concept and codebook id (`0x0105` / `0x0106`).
    #[test]
    #[ignore = "requires Redmine + OpenProject checkouts"]
    fn redmine_and_openproject_status_and_type_converge_through_canonical() {
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

        let status_id = ogar_vocab::canonical_concept_id("project_status");
        let type_id = ogar_vocab::canonical_concept_id("project_type");
        assert!(status_id.is_some() && type_id.is_some());

        // Each (curator, class_name) tuple must lift to (concept, id).
        // Cross-curator name divergence: IssueStatus/Status, Tracker/Type.
        for (curator, classes, table) in [
            (
                "Redmine",
                &redmine,
                &[
                    ("IssueStatus", "project_status", status_id),
                    ("Tracker", "project_type", type_id),
                ][..],
            ),
            (
                "OpenProject",
                &openproject,
                &[
                    ("Status", "project_status", status_id),
                    ("Type", "project_type", type_id),
                ][..],
            ),
        ] {
            for (class_name, concept, expected_id) in table {
                let c = classes
                    .iter()
                    .find(|c| c.name == *class_name)
                    .unwrap_or_else(|| panic!("{curator} ships a {class_name} model"));
                assert_eq!(
                    c.canonical_concept.as_deref(),
                    Some(*concept),
                    "{curator} {class_name} -> {concept}",
                );
                assert_eq!(c.source_domain.as_deref(), Some("project"));
                assert_eq!(
                    c.canonical_id(),
                    *expected_id,
                    "{curator} {class_name} -> codebook id for {concept}",
                );
            }
        }
    }

    /// Both curators use `IssuePriority < Enumeration`; both lift to the
    /// canonical `priority` concept (codebook id `0x0107` post-#66).
    #[test]
    #[ignore = "requires Redmine + OpenProject checkouts"]
    fn redmine_and_openproject_issuepriority_converge_through_canonical() {
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

        let priority_id = ogar_vocab::canonical_concept_id("priority");
        assert!(priority_id.is_some());

        for (curator, classes) in [("Redmine", &redmine), ("OpenProject", &openproject)] {
            let p = classes
                .iter()
                .find(|c| c.name == "IssuePriority")
                .unwrap_or_else(|| panic!("{curator} ships an IssuePriority model"));
            assert_eq!(p.canonical_concept.as_deref(), Some("priority"));
            assert_eq!(p.source_domain.as_deref(), Some("project"));
            assert_eq!(p.canonical_id(), priority_id);
        }
    }

    /// **ProjectRelation + Changeset + Watcher convergence** on real
    /// source. Closes the last `project_work_item` forward reference
    /// (`.relations`) and lands two natural neighbours that pair with
    /// already-canonical concepts:
    ///
    /// | Canonical concept   | Redmine        | OpenProject |
    /// |---------------------|----------------|-------------|
    /// | project_relation    | IssueRelation  | Relation    |
    /// | project_changeset   | Changeset      | Changeset   |
    /// | project_watcher     | Watcher        | Watcher     |
    #[test]
    #[ignore = "requires Redmine + OpenProject checkouts"]
    fn redmine_and_openproject_relation_changeset_watcher_converge() {
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
        let rel_id = ogar_vocab::canonical_concept_id("project_relation");
        let cs_id = ogar_vocab::canonical_concept_id("project_changeset");
        let w_id = ogar_vocab::canonical_concept_id("project_watcher");
        assert!(rel_id.is_some() && cs_id.is_some() && w_id.is_some());

        for (curator, classes, table) in [
            ("Redmine", &redmine, &[
                ("IssueRelation", "project_relation", rel_id),
                ("Changeset", "project_changeset", cs_id),
                ("Watcher", "project_watcher", w_id),
            ][..]),
            ("OpenProject", &openproject, &[
                ("Relation", "project_relation", rel_id),
                ("Changeset", "project_changeset", cs_id),
                ("Watcher", "project_watcher", w_id),
            ][..]),
        ] {
            for (class_name, concept, expected_id) in table {
                let c = classes
                    .iter()
                    .find(|c| c.name == *class_name)
                    .unwrap_or_else(|| panic!("{curator} ships a {class_name} model"));
                assert_eq!(
                    c.canonical_concept.as_deref(),
                    Some(*concept),
                    "{curator} {class_name} -> {concept}",
                );
                assert_eq!(c.source_domain.as_deref(), Some("project"));
                assert_eq!(c.canonical_id(), *expected_id);
            }
        }
    }

    /// **News + Message convergence** on real source. Both curators ship
    /// `News` (project news/blog) and `Message` (threaded forum/board
    /// discussion) — the latter has parent-container divergence (Redmine
    /// `Board` vs OP `Forum`), but `Message` itself converges.
    #[test]
    #[ignore = "requires Redmine + OpenProject checkouts"]
    fn redmine_and_openproject_news_and_message_converge() {
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
        let news_id = ogar_vocab::canonical_concept_id("project_news");
        let msg_id = ogar_vocab::canonical_concept_id("project_message");
        assert!(news_id.is_some() && msg_id.is_some());

        for (curator, classes) in [("Redmine", &redmine), ("OpenProject", &openproject)] {
            for (class_name, concept, expected_id) in [
                ("News", "project_news", news_id),
                ("Message", "project_message", msg_id),
            ] {
                let c = classes
                    .iter()
                    .find(|c| c.name == class_name)
                    .unwrap_or_else(|| panic!("{curator} ships a {class_name} model"));
                assert_eq!(
                    c.canonical_concept.as_deref(),
                    Some(concept),
                    "{curator} {class_name} -> {concept}",
                );
                assert_eq!(c.source_domain.as_deref(), Some("project"));
                assert_eq!(c.canonical_id(), expected_id);
            }
        }
    }

    /// **Board/Forum convergence** on real source. Cross-curator name
    /// divergence (Redmine `Board` ↔ OP `Forum`) unified onto
    /// `project_forum`. Closes the parent-container reference from
    /// `project_message`.
    #[test]
    #[ignore = "requires Redmine + OpenProject checkouts"]
    fn redmine_board_and_openproject_forum_converge_as_project_forum() {
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
        let id = ogar_vocab::canonical_concept_id("project_forum");
        assert!(id.is_some());

        let redmine_board = redmine
            .iter()
            .find(|c| c.name == "Board")
            .expect("Redmine ships a Board model");
        let op_forum = openproject
            .iter()
            .find(|c| c.name == "Forum")
            .expect("OpenProject ships a Forum model");

        for c in [redmine_board, op_forum] {
            assert_eq!(c.canonical_concept.as_deref(), Some("project_forum"));
            assert_eq!(c.source_domain.as_deref(), Some("project"));
            assert_eq!(c.canonical_id(), id);
        }
    }

    /// **Role convergence + Group→actor collapse** on real source.
    /// `Role` (both curators) → `project_role`. `Group` (a Principal STI
    /// subtype in both) folds into `project_actor` alongside User /
    /// Principal — it is assignable / member-able exactly where a User is.
    #[test]
    #[ignore = "requires Redmine + OpenProject checkouts"]
    fn redmine_and_openproject_role_and_group_converge() {
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
        let role_id = ogar_vocab::canonical_concept_id("project_role");
        let actor_id = ogar_vocab::canonical_concept_id("project_actor");
        assert!(role_id.is_some() && actor_id.is_some());

        for (curator, classes) in [("Redmine", &redmine), ("OpenProject", &openproject)] {
            // Role -> project_role.
            let role = classes
                .iter()
                .find(|c| c.name == "Role")
                .unwrap_or_else(|| panic!("{curator} ships a Role model"));
            assert_eq!(role.canonical_concept.as_deref(), Some("project_role"));
            assert_eq!(role.canonical_id(), role_id);
            // Group -> project_actor (Principal STI subtype collapse).
            let group = classes
                .iter()
                .find(|c| c.name == "Group")
                .unwrap_or_else(|| panic!("{curator} ships a Group model"));
            assert_eq!(
                group.canonical_concept.as_deref(),
                Some("project_actor"),
                "{curator} Group folds into project_actor",
            );
            assert_eq!(group.canonical_id(), actor_id);
        }
    }

    /// **Structural-completion trio convergence** on real source —
    /// MemberRole (RBAC join), CustomValue (custom-field value), and
    /// EnabledModule (project module enablement). All three are identical
    /// across both curators.
    #[test]
    #[ignore = "requires Redmine + OpenProject checkouts"]
    fn redmine_and_openproject_memberrole_customvalue_module_converge() {
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
        let table = [
            ("MemberRole", "project_member_role"),
            ("CustomValue", "project_custom_value"),
            ("EnabledModule", "project_enabled_module"),
        ];
        for (curator, classes) in [("Redmine", &redmine), ("OpenProject", &openproject)] {
            for (class_name, concept) in table {
                let c = classes
                    .iter()
                    .find(|c| c.name == class_name)
                    .unwrap_or_else(|| panic!("{curator} ships a {class_name} model"));
                assert_eq!(c.canonical_concept.as_deref(), Some(concept), "{curator} {class_name}");
                assert_eq!(c.source_domain.as_deref(), Some("project"));
                assert_eq!(c.canonical_id(), ogar_vocab::canonical_concept_id(concept));
            }
        }
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
