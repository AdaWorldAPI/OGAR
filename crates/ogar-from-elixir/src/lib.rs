//! `ogar-from-elixir` — **SCAFFOLD** Elixir/HIRO frontend for OGAR-wide IR.
//!
//! This crate exists to be *finished*, not to work yet. It pins the target
//! OGAR shape ([`Class`] / [`ActionDef`]) via passing tests and marks every
//! place a real Elixir parser must plug in with a `todo!()` and a
//! doc-comment naming the exact Elixir construct to read.
//!
//! # The two-arm pattern (per `docs/OPENPROJECT-TRANSCODING.md §10.1`)
//!
//! This is the **wide / OGAR** arm for Elixir — lifecycle contract
//! (`Class` + `ActionDef` + `ActionInvocation`). A future `ruff_elixir_spo`
//! (mirroring `AdaWorldAPI/ruff/crates/ruff_ruby_spo`) is the narrow SPO
//! arm — data-dependency edges via `ruff_spo_triplet`. **Both arms feed
//! off the same parsed Elixir AST** — one parser, two emitters.
//!
//! # How to finish it
//!
//! See `docs/ELIXIR-HIRO-PREFETCH.md §2` for the full mapping. In short:
//!
//! 1. Add an Elixir parser dep. Recommended: `tree-sitter` +
//!    `tree-sitter-elixir`.
//! 2. Replace the `todo!()` in [`parse_modules`] to produce a
//!    `Vec<ElixirModule>`.
//! 3. Replace the `todo!()`s in the per-construct extractors
//!    ([`extract_ecto_schemas`], [`extract_gen_server_actions`],
//!    [`extract_gen_statem_actions`], [`extract_phoenix_actions`],
//!    [`extract_oban_actions`]) to read the Elixir constructs documented
//!    on each.
//! 4. Run the locked-shape tests after each step — they assert the OGAR
//!    output for hand-built fixtures, so they tell you when your
//!    extraction produces the right shape.
//! 5. Point [`extract`] / [`extract_action_defs`] at a real Elixir source
//!    tree (e.g., HIRO's `lib/` or Bardioc's `apps/*/lib/`).
//!
//! The downstream consumers (`ogar-emitter`, `ogar-proposal`, and through
//! the §6 `lance-bind` boundary `lance-graph-ontology::OntologyRegistry`)
//! need ZERO changes — they already consume the OGAR vocab shape this
//! crate targets.
//!
//! # The four §6 Rubicon statem terms are first-class
//!
//! `gen_statem` is the dominant lifecycle construct in HIRO/Bardioc and
//! the load-bearing case for OGAR's Rubicon binding. The four §6 terms
//! lower 1:1 from `gen_statem` returns:
//!
//! | `gen_statem` return / callback        | OGAR `ActionDef` field             |
//! | ---                                   | ---                                |
//! | `state_enter` callback                | `on_enter: Some(EnterEffect)`      |
//! | `[:postpone]` action                  | `guard_failure_policy: Postponable`|
//! | `[{:state_timeout, ms, _}]` action    | `state_timeout_millis: Some(ms)`   |
//! | `{:next_state, NewState, _}` return   | `on_enter: EnterEffect::transition("state", NewState)` |

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::path::Path;

use ogar_vocab::{
    ActionDef, ActionSubject, Association, AssociationKind, Attribute, Class, EnterEffect,
    GuardFailurePolicy, KausalSpec, Language, ModalSpec, TemporalSpec,
};

/// The namespace prefix for OGIT-Elixir mappings.
///
/// OLD HIRO entities map under `ogit:` / `ogit-<app>:` per
/// `ELIXIR-HIRO-PREFETCH.md §4`. Phoenix context module names
/// (e.g. `MyApp.Accounts`) become Identity prefix segments below this.
pub const NAMESPACE_OGIT: &str = "ogit";

/// A minimally-parsed Elixir module — what a parser frontend should
/// produce before the IR mapping. Kept tiny on purpose; extend as the
/// real extractor needs more raw material.
#[derive(Debug, Clone, Default)]
pub struct ElixirModule {
    /// Module name as written (`MyApp.Accounts.Account`).
    pub name: String,
    /// Raw source of the module body — the extractors scan this.
    pub body_source: String,
    /// `use M [, opts]` and `@behaviour M` directives, normalised to the
    /// module name (`"Ecto.Schema"`, `"GenServer"`, `":gen_statem"`,
    /// `"Phoenix.Controller"`, `"Oban.Worker"`). Drives which extractor(s)
    /// apply.
    pub use_directives: Vec<String>,
}

/// Top-level entry: walk an Elixir source tree and produce OGAR
/// `Class`es (the structural arm — per `ELIXIR-HIRO-PREFETCH.md §2.1`).
///
/// # Panics
///
/// Currently `todo!()` — wire [`parse_modules`] first.
#[must_use]
pub fn extract(source_tree: &Path) -> Vec<Class> {
    let modules = parse_modules(source_tree);
    let mut classes = Vec::new();
    for module in &modules {
        if module.use_directives.iter().any(|u| u == "Ecto.Schema") {
            classes.extend(extract_ecto_schemas(module));
        }
    }
    classes
}

/// Top-level entry for the behavior arm: extract `ActionDef`s from
/// GenServers, gen_statems, Phoenix handlers, Oban workers (per
/// `ELIXIR-HIRO-PREFETCH.md §2.2`).
///
/// # Panics
///
/// Currently `todo!()` — wire [`parse_modules`] first.
#[must_use]
pub fn extract_action_defs(source_tree: &Path) -> Vec<ActionDef> {
    let modules = parse_modules(source_tree);
    let mut defs = Vec::new();
    for module in &modules {
        for u in &module.use_directives {
            match u.as_str() {
                "GenServer" => defs.extend(extract_gen_server_actions(module)),
                "Phoenix.Controller" | "Phoenix.Channel" | "Phoenix.LiveView" => {
                    defs.extend(extract_phoenix_actions(module));
                }
                "Oban.Worker" => defs.extend(extract_oban_actions(module)),
                _ => {}
            }
        }
        // gen_statem is conventionally `@behaviour :gen_statem`, not `use`.
        if module
            .use_directives
            .iter()
            .any(|u| u == ":gen_statem" || u == "GenStateMachine")
        {
            defs.extend(extract_gen_statem_actions(module));
        }
    }
    defs
}

/// Parse every `*.ex` / `*.exs` under `source_tree` into [`ElixirModule`].
///
/// # What to wire
///
/// Use `tree-sitter-elixir` to:
/// - find each `defmodule X do … end` (sets `ElixirModule.name`),
/// - capture its body source range (`ElixirModule.body_source`),
/// - collect every `use M [, opts]` and `@behaviour M` into
///   `use_directives` (normalised to the module name).
fn parse_modules(_source_tree: &Path) -> Vec<ElixirModule> {
    todo!("wire tree-sitter-elixir; produce Vec<ElixirModule> per ELIXIR-HIRO-PREFETCH §2")
}

// ─────────────────────────────────────────────────────────────────────
// Structural arm — `Class` extraction (per ELIXIR-HIRO-PREFETCH §2.1)
// ─────────────────────────────────────────────────────────────────────

/// Extract one `Class` per `schema "table_name" do … end` block inside
/// an `Ecto.Schema` module.
///
/// # What to wire
///
/// Walk the module's body AST per `ELIXIR-HIRO-PREFETCH.md §2.1`:
/// - find `schema "table_name" do … end` (sets `Class.name` from the
///   module's final segment; the table name goes elsewhere via
///   producer-specific decoration if needed),
/// - inside it, find `field :name, :type [, opts]` calls →
///   `Attribute { name, type_name: Some(type) }`,
/// - find `belongs_to :rel, M` →
///   `Association { kind: BelongsTo, name, class_name: Some(M) }`,
/// - `has_one` / `has_many` / `many_to_many` likewise (kind varies),
/// - `embeds_one` / `embeds_many` → nested association + embedded attribute,
/// - `@primary_key` / `@foreign_key_type` → `AttributeOptions`,
/// - `validate_required` / `validate_format` (in changeset functions) →
///   `Validation { target, rule_source }`.
///
/// The Phoenix-context module name (e.g. `MyApp.Accounts`) becomes the
/// Identity prefix segment (HHTL namespace) — the producer-level
/// concern.
fn extract_ecto_schemas(_module: &ElixirModule) -> Vec<Class> {
    todo!("walk schema/field/belongs_to/has_*/embeds_*/@primary_key per ELIXIR-HIRO-PREFETCH §2.1")
}

// ─────────────────────────────────────────────────────────────────────
// Behavioral arm — `ActionDef` extraction (per ELIXIR-HIRO-PREFETCH §2.2)
// ─────────────────────────────────────────────────────────────────────

/// Extract `ActionDef`s from `GenServer.handle_call/3` /
/// `handle_cast/2` / `handle_info/2` callbacks.
///
/// # What to wire
///
/// Per `ELIXIR-HIRO-PREFETCH.md §2.2`:
/// - `def handle_call(msg, _from, state)` →
///   `ActionDef { default_modal: Sync, ... }`
/// - `def handle_cast(msg, state)` →
///   `ActionDef { default_modal: Async, ... }`
/// - `def handle_info(msg, state)` →
///   `ActionDef { default_subject: Trigger, ... }`
fn extract_gen_server_actions(_module: &ElixirModule) -> Vec<ActionDef> {
    todo!("extract handle_call/cast/info per ELIXIR-HIRO-PREFETCH §2.2 GenServer.* rows")
}

/// Extract `ActionDef`s from `gen_statem` state-callback functions,
/// lowering the four §6 Rubicon statem terms onto
/// `ActionDef.{on_enter, guard_failure_policy, state_timeout_millis}`.
///
/// # What to wire
///
/// `gen_statem` callbacks in `state_functions` callback mode look like:
///
/// ```text
/// def follower({:call, from}, :vote_request, data) do
///   case ... do
///     {:next_state, :candidate, new_data,
///       [{:state_timeout, election_timeout(), :become_candidate},
///        {:reply, from, :ok}]}
///     {:keep_state_and_data, [:postpone]}
///     ...
///   end
/// end
/// ```
///
/// Map per `ELIXIR-HIRO-PREFETCH.md §2.2`:
/// - The function head's state name (`follower`) is the
///   `KausalSpec::StateGuard { guard_field: "state", guard_values: [state] }`
///   — the precondition for this branch.
/// - The event term (the second arg) is the `predicate`.
/// - `{:next_state, T, _}` returns →
///   `on_enter: Some(EnterEffect::transition("state", T))`.
/// - `[{:state_timeout, ms, _}]` action →
///   `state_timeout_millis: Some(ms as i64)`.
/// - `[:postpone]` action →
///   `guard_failure_policy: Some(GuardFailurePolicy::Postponable)`.
/// - `state_enter(_old, new, data)` callback (callback-mode-dependent) →
///   `on_enter: Some(EnterEffect::transition("state", new))` plus any
///   side-effect captured as the EnterEffect's value (future tightening).
fn extract_gen_statem_actions(_module: &ElixirModule) -> Vec<ActionDef> {
    todo!(
        "extract state callbacks + the four Rubicon returns per \
         ELIXIR-HIRO-PREFETCH §2.2 gen_statem.* rows"
    )
}

/// Extract `ActionDef`s from Phoenix controller actions, channel
/// `handle_in`/`handle_out`, and LiveView `handle_event`.
///
/// # What to wire
///
/// Per `ELIXIR-HIRO-PREFETCH.md §2.2`:
/// - Phoenix controller `def create(conn, params)` →
///   `ActionDef { default_subject: User, default_modal: Sync, ... }`
/// - Phoenix channel `def handle_in(event, payload, socket)` /
///   `handle_out(...)` → `ActionDef` on the wire surface (directly
///   relevant to §14 wire-roundtrip).
/// - LiveView `def handle_event(event, params, socket)` →
///   `ActionDef { default_subject: User }`
fn extract_phoenix_actions(_module: &ElixirModule) -> Vec<ActionDef> {
    todo!(
        "extract controller / channel / liveview handlers per ELIXIR-HIRO-PREFETCH §2.2 Phoenix.* rows"
    )
}

/// Extract `ActionDef`s from `Oban.Worker.perform/1` definitions and
/// `Oban.insert*` callsites.
///
/// # What to wire
///
/// Per `ELIXIR-HIRO-PREFETCH.md §2.2`:
/// - `defmodule MyWorker do use Oban.Worker ...; def perform(job) ...; end` →
///   `ActionDef { default_subject: System, default_temporal: Deferred }`
/// - `Oban.insert(...)` / `Oban.insert_after(at, ...)` callsites →
///   `default_temporal: Scheduled` (the `at` is the schedule).
/// - `Oban.insert_all(...)` callsites → cascade `ActionInvocation`s on
///   enqueue.
fn extract_oban_actions(_module: &ElixirModule) -> Vec<ActionDef> {
    todo!(
        "extract Oban.Worker perform/1 + insert*/insert_after callsites per \
         ELIXIR-HIRO-PREFETCH §2.2 Oban.* rows"
    )
}

// ─────────────────────────────────────────────────────────────────────
// Locked-shape tests — assert the target OGAR shape for hand-built
// fixtures. When the extractors are wired, they must produce these
// shapes.
// ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    /// Locked-shape test for an Ecto.Schema:
    ///
    /// ```elixir
    /// defmodule MyApp.Accounts.Account do
    ///   use Ecto.Schema
    ///   schema "accounts" do
    ///     field :email, :string
    ///     belongs_to :owner, MyApp.Users.User
    ///   end
    /// end
    /// ```
    ///
    /// When [`extract_ecto_schemas`] is wired, the produced `Class`
    /// must match this shape (modulo identity prefix, set by the
    /// producer-level Phoenix-context normalization).
    #[test]
    fn ecto_schema_class_shape_is_locked() {
        let mut expected = Class::new("Account");
        expected.language = Language::Elixir;
        expected.declared_in_module = Some("MyApp.Accounts.Account".into());
        let mut email = Attribute::new("email");
        email.type_name = Some("string".into());
        expected.attributes.push(email);
        let mut owner = Association::new(AssociationKind::BelongsTo, "owner");
        owner.class_name = Some("User".into());
        expected.associations.push(owner);

        assert_eq!(expected.language, Language::Elixir);
        assert_eq!(
            expected.declared_in_module.as_deref(),
            Some("MyApp.Accounts.Account")
        );
        assert_eq!(expected.attributes.len(), 1);
        assert_eq!(expected.attributes[0].name, "email");
        assert_eq!(expected.attributes[0].type_name.as_deref(), Some("string"));
        assert_eq!(expected.associations.len(), 1);
        assert!(matches!(
            expected.associations[0].kind,
            AssociationKind::BelongsTo
        ));
        assert_eq!(expected.associations[0].class_name.as_deref(), Some("User"));
    }

    /// Locked-shape test for a `gen_statem` state callback (Raft
    /// consensus flavor — `rafted_value` is the load-bearing
    /// `gen_statem` case per `ELIXIR-HIRO-PREFETCH.md §2.2` rafted_value row):
    ///
    /// ```elixir
    /// def follower({:call, from}, :vote_request, data) do
    ///   {:next_state, :candidate, data,
    ///     [{:state_timeout, 150, :become_candidate},
    ///      {:reply, from, :ok}]}
    /// end
    /// ```
    ///
    /// When [`extract_gen_statem_actions`] is wired, the produced
    /// `ActionDef` must carry all four §6 Rubicon terms.
    #[test]
    fn gen_statem_action_def_shape_is_locked() {
        let mut def = ActionDef::new(
            "ogit-hiro/raft.consensus::action_def::vote_request",
            "vote_request",
            "ogit-hiro/raft.consensus",
        );
        def.kausal = Some(KausalSpec::StateGuard {
            guard_field: "state".into(),
            guard_values: vec!["follower".into()],
        });
        def.default_modal = ModalSpec::Sync;
        // randomized election-timeout window upper-bound
        def.state_timeout_millis = Some(150);
        // gen_statem :postpone action
        def.guard_failure_policy = Some(GuardFailurePolicy::Postponable);
        // {:next_state, :candidate, _}
        def.on_enter = Some(EnterEffect::transition("state", "candidate"));

        assert_eq!(def.predicate, "vote_request");
        assert_eq!(def.object_class, "ogit-hiro/raft.consensus");
        match &def.kausal {
            Some(KausalSpec::StateGuard {
                guard_field,
                guard_values,
            }) => {
                assert_eq!(guard_field, "state");
                assert_eq!(guard_values, &vec!["follower".to_string()]);
            }
            other => panic!("expected StateGuard, got {other:?}"),
        }
        assert!(matches!(def.default_modal, ModalSpec::Sync));
        assert_eq!(def.state_timeout_millis, Some(150));
        assert!(matches!(
            def.guard_failure_policy,
            Some(GuardFailurePolicy::Postponable)
        ));
        let on_enter = def.on_enter.as_ref().expect("on_enter set");
        assert_eq!(on_enter.field, "state");
        assert_eq!(on_enter.to_value, "candidate");
    }

    /// Locked-shape: a Phoenix controller action is User-subject,
    /// Sync-modal, no state-machine guard (controller actions are
    /// stateless wire entry points).
    #[test]
    fn phoenix_controller_action_def_shape_is_locked() {
        let mut def = ActionDef::new(
            "ogit-op/account_controller::action_def::create",
            "create",
            "ogit-op/account",
        );
        def.default_subject = ActionSubject::User;
        def.default_modal = ModalSpec::Sync;

        assert!(matches!(def.default_subject, ActionSubject::User));
        assert!(matches!(def.default_modal, ModalSpec::Sync));
        assert!(def.kausal.is_none());
    }

    /// Locked-shape: an Oban worker is System-subject, Deferred-temporal.
    #[test]
    fn oban_worker_action_def_shape_is_locked() {
        let mut def = ActionDef::new(
            "ogit-hiro/notification_worker::action_def::perform",
            "perform",
            "ogit-hiro/notification",
        );
        def.default_subject = ActionSubject::System;
        def.default_temporal = TemporalSpec::Deferred;

        assert!(matches!(def.default_subject, ActionSubject::System));
        assert!(matches!(def.default_temporal, TemporalSpec::Deferred));
    }

    /// Top-level dispatch dispatches by `use` directive — `parse_modules`
    /// is not yet wired, but we verify the dispatch logic compiles and
    /// the use-directive matching is what each top-level entry consults.
    #[test]
    fn dispatch_keys_are_the_expected_use_directives() {
        // This test documents the recognized `use` / `@behaviour`
        // directives so a parser wiring change can't silently drop
        // dispatch keys.
        let recognized_use = [
            "Ecto.Schema",
            "GenServer",
            "Phoenix.Controller",
            "Phoenix.Channel",
            "Phoenix.LiveView",
            "Oban.Worker",
        ];
        let recognized_behaviour = [":gen_statem", "GenStateMachine"];
        // Compile-time presence is the assertion; the list is the spec.
        assert_eq!(recognized_use.len(), 6);
        assert_eq!(recognized_behaviour.len(), 2);
    }
}
