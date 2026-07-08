//! HIRO **Action API** REST registration lift — parity brick **B2-lift**.
//!
//! Registration is REST, not a WebSocket handshake (`docs/ARAGO-ACTIONHANDLER-PARITY.md`
//! §2a): a deployed handler's capabilities and applicabilities live in the graph
//! and are read via the REST Action API (`/api/action/1.0/`, Bearer token) —
//! `GET /capabilities` → a `MapOfCapabilities` (each
//! `{description, mandatoryParameters, optionalParameters}`), `GET /applicabilities`
//! → a `MapOfApplicabilities` (keyed by handler id, each carrying the node-match
//! `ModelFilter`s). The `action-ws` socket has **no** registration message — a
//! `configChanged` just tells the handler to re-`GET` them.
//!
//! This module is the **instance lift**: it turns that deployed REST view into the
//! concrete OGAR signatures the schema half cannot supply. The OGIT ontology
//! declares only *that* the param slots exist (the `mandatoryParameters` /
//! `optionalParameters` / `resultParameters` attributes — see
//! [`crate::do_arm::CapabilitySlot`]); the concrete `(name, mandatory, default)`
//! tuples come from a *deployed* handler config — exactly the gap
//! [`crate::do_arm::ActionParam`]'s doc-comment names. B2-lift fills it:
//!
//! - `GET /capabilities` → [`ConcreteCapability`] (a [`do_arm::ActionParam`] list
//!   per capability — the signature [`crate::action_ws::bind_parameters`] validates
//!   the engine's `parameters` against).
//! - `GET /applicabilities` → the node-match guards: arago `ModelFilter{Var,Mode,
//!   Value}` lifts field-for-field to [`KausalSpec::StateGuard`] (`Var` →
//!   `guard_field`, `Value` → `guard_values`).
//!
//! **No parser here.** Mirroring the crate's parser-free default path, this module
//! defines the typed REST DTOs (`Deserialize` behind the `serde` feature) plus the
//! *pure* lift mapping. The actual JSON read (`serde_json::from_str`) is the
//! runtime crate's job (`ogar-action-handler`), which owns the I/O — the same
//! producer-defines-types / runtime-does-I/O split the rest of the crate keeps.

use std::collections::BTreeMap;

#[cfg(feature = "serde")]
use serde::Deserialize;

use ogar_vocab::KausalSpec;

use crate::do_arm::ActionParam;

/// One parameter as the REST Action API reports it: a `description` and an
/// optional `default`. Whether it is *mandatory* is carried by which map it
/// appears in (`mandatoryParameters` vs `optionalParameters`), per the spec —
/// so this DTO does not repeat it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
pub struct RegisteredParam {
    /// Human-readable description (arago `Parameter.Description`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub description: Option<String>,
    /// Default value when the engine does not supply one (arago `Parameter.Default`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub default: Option<String>,
}

/// One capability as `GET /capabilities` reports it. The parameter maps are
/// keyed by parameter name (`MapOfParameters`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RegisteredCapability {
    /// Human-readable description (arago `Capability.Description`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub description: Option<String>,
    /// Required-input signature, keyed by parameter name.
    #[cfg_attr(feature = "serde", serde(default))]
    pub mandatory_parameters: BTreeMap<String, RegisteredParam>,
    /// Optional-input signature, keyed by parameter name.
    #[cfg_attr(feature = "serde", serde(default))]
    pub optional_parameters: BTreeMap<String, RegisteredParam>,
}

/// The `GET /capabilities` response — a `MapOfCapabilities` keyed by capability
/// name (the arago `Capability.Name`, which matches an `ActionDef::predicate`).
pub type RegisteredCapabilities = BTreeMap<String, RegisteredCapability>;

/// A single node-match filter as arago reports it: `ModelFilter{Var, Mode,
/// Value}` — "the node attribute `Var` `Mode`-matches `Value`". The documented
/// field-for-field source of a [`KausalSpec::StateGuard`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ModelFilter {
    /// The node attribute being matched (arago `ModelFilter.Var`) → `guard_field`.
    pub var: String,
    /// The match mode (`equals` / `matches` / …). `StateGuard` is set-membership,
    /// so the mode is preserved here for fidelity but does not change the lifted
    /// guard's shape (an `equals` over one value).
    #[cfg_attr(feature = "serde", serde(default))]
    pub mode: Option<String>,
    /// The value to match (arago `ModelFilter.Value`) → the single `guard_values`.
    pub value: String,
}

/// A concrete capability signature, lifted from the deployed REST registration:
/// the capability name plus its full [`ActionParam`] list (mandatory params
/// first, then optional — each with its concrete `(name, mandatory, default)`).
///
/// This is the half the schema lift cannot produce. Feed `params` to
/// [`crate::action_ws::bind_parameters`] to validate an engine `submitAction`'s
/// `parameters` the way arago's Python handler does before executing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcreteCapability {
    /// The capability name (`ActionDef::predicate`).
    pub name: String,
    /// Human-readable description, if the registration carried one.
    pub description: Option<String>,
    /// The concrete parameter signature — mandatory params first, then optional.
    pub params: Vec<ActionParam>,
    /// Per-parameter descriptions (arago `Parameter.Description`), keyed by
    /// parameter name. Kept OFF [`ActionParam`] deliberately: the binding
    /// signature (`bind_parameters`) needs `(name, mandatory, default)` only;
    /// descriptions are render/doc metadata, so they ride on the lift artifact
    /// instead of widening the IR type (ground-truth gap found 2026-07-08
    /// re-checking `AdaWorldAPI/ActionHandlers` — the DTO carried it, the
    /// lift dropped it).
    pub param_descriptions: BTreeMap<String, String>,
}

/// Lift one deployed capability into its concrete [`ConcreteCapability`] —
/// mandatory params (mandatory = `true`) first, then optional (mandatory =
/// `false`), each carrying its `default`. Deterministic order (the source maps
/// are [`BTreeMap`]s, sorted by parameter name).
#[must_use]
pub fn lift_capability(name: &str, cap: &RegisteredCapability) -> ConcreteCapability {
    let mandatory = cap.mandatory_parameters.iter().map(|(n, p)| ActionParam {
        name: n.clone(),
        mandatory: true,
        default: p.default.clone(),
    });
    let optional = cap.optional_parameters.iter().map(|(n, p)| ActionParam {
        name: n.clone(),
        mandatory: false,
        default: p.default.clone(),
    });
    let param_descriptions = cap
        .mandatory_parameters
        .iter()
        .chain(cap.optional_parameters.iter())
        .filter_map(|(n, p)| p.description.clone().map(|d| (n.clone(), d)))
        .collect();
    ConcreteCapability {
        name: name.to_owned(),
        description: cap.description.clone(),
        params: mandatory.chain(optional).collect(),
        param_descriptions,
    }
}

/// Lift a whole `GET /capabilities` response into the concrete capability
/// signatures, one [`ConcreteCapability`] per capability (sorted by name).
#[must_use]
pub fn lift_registration(caps: &RegisteredCapabilities) -> Vec<ConcreteCapability> {
    caps.iter().map(|(n, c)| lift_capability(n, c)).collect()
}

/// Lift one arago `ModelFilter{Var,Mode,Value}` into its OGAR
/// [`KausalSpec::StateGuard`] — `Var` → `guard_field`, `Value` → the single
/// `guard_values` entry. The documented field-for-field applicability mapping
/// (the same one `do_arm::kausal_from_entity` reaches at the schema level, here
/// carrying the deployed *value*).
#[must_use]
pub fn model_filter_to_guard(filter: &ModelFilter) -> KausalSpec {
    KausalSpec::StateGuard {
        guard_field: filter.var.clone(),
        guard_values: vec![filter.value.clone()],
    }
}

/// Lift a deployed applicability's `ModelFilter`s into the OGAR guard set — one
/// [`KausalSpec::StateGuard`] per filter (an applicability applies where **all**
/// of its filters match; each guard is one such condition).
#[must_use]
pub fn lift_applicability(filters: &[ModelFilter]) -> Vec<KausalSpec> {
    filters.iter().map(model_filter_to_guard).collect()
}

/// One applicability as `GET /applicabilities` reports it — its node-match
/// `ModelFilter`s.
///
/// The harvested spec (`docs/ARAGO-ACTIONHANDLER-PARITY.md` §2a) pins the
/// *outer* shape (a `MapOfApplicabilities` keyed by handler id) and the
/// `ModelFilter{Var,Mode,Value}` element, but not the exact name of the field
/// that carries the filter list. This DTO accepts the plausible names
/// (`modelFilters`, `model`, `filters`) via serde aliases and defaults to an
/// empty list, so a real response binds without a code change once the field
/// name is confirmed. The element mapping itself (`ModelFilter → StateGuard`) is
/// the documented, `[G]` part.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RegisteredApplicability {
    /// The node-match filters (an applicability applies where they all match).
    #[cfg_attr(feature = "serde", serde(default, alias = "model", alias = "filters"))]
    pub model_filters: Vec<ModelFilter>,
    /// Applicability precedence (arago `Applicability.Priority` — e.g. `50` in
    /// the canonical `ssh_based_actions` stanza). Higher wins when several
    /// applicabilities match the same node. `None` when the deployment does not
    /// rank. Ground-truth gap closed 2026-07-08 (`AdaWorldAPI/ActionHandlers`);
    /// the parity doc had flagged it as the one config field without an OGAR
    /// home (§1 footnote ²).
    #[cfg_attr(feature = "serde", serde(default, alias = "Priority"))]
    pub priority: Option<i64>,
    /// Applicability-scoped parameter defaults (arago
    /// `Applicability.Parameter{Name,Value}`) — values injected into every
    /// capability this applicability provides. The canonical example is the
    /// SSH handler's `SshOpts` (`-o StrictHostKeyChecking=no -o BatchMode=yes …`),
    /// declared once at the applicability and expanded in each capability's
    /// `Command` template. Ground-truth gap closed 2026-07-08.
    #[cfg_attr(
        feature = "serde",
        serde(default, alias = "parameter", alias = "Parameter")
    )]
    pub parameters: Vec<ApplicabilityParam>,
}

/// One applicability-scoped parameter default as arago declares it:
/// `Parameter{Name, Value}` on the `Applicability` stanza (NOT the capability
/// signature — capability params carry `Mandatory`/`Default`; applicability
/// params are always concrete `(name, value)` injections).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ApplicabilityParam {
    /// Parameter name (arago `Parameter.Name`).
    #[cfg_attr(feature = "serde", serde(alias = "Name"))]
    pub name: String,
    /// The injected value (arago `Parameter.Value`).
    #[cfg_attr(feature = "serde", serde(alias = "Value"))]
    pub value: String,
}

/// The `GET /applicabilities` response — a `MapOfApplicabilities` keyed by
/// handler id.
pub type RegisteredApplicabilities = BTreeMap<String, RegisteredApplicability>;

/// Lift a whole `GET /applicabilities` response into the per-handler guard sets:
/// handler id → its [`KausalSpec::StateGuard`] list (each handler applies where
/// **all** of its guards match). Deterministic order (the source is a
/// [`BTreeMap`], sorted by handler id).
#[must_use]
pub fn lift_applicabilities(apps: &RegisteredApplicabilities) -> BTreeMap<String, Vec<KausalSpec>> {
    apps.iter()
        .map(|(handler, app)| (handler.clone(), lift_applicability(&app.model_filters)))
        .collect()
}

/// A fully-lifted applicability — the guards PLUS the two config fields the
/// guard-only lift ([`lift_applicability`]) does not carry: the precedence
/// (`Priority`) and the applicability-scoped parameter defaults
/// (`Parameter{Name,Value}`, e.g. the SSH handler's `SshOpts`).
///
/// The `params` are already in [`ActionParam`] form (`mandatory = false`,
/// `default = Some(value)`), so a dispatcher concatenates them onto a
/// capability's own signature and [`crate::action_ws::bind_parameters`] fills
/// them exactly the way arago expands applicability parameters into the
/// capability `Command` template.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConcreteApplicability {
    /// The node-match guard set (same as [`lift_applicability`]).
    pub guards: Vec<KausalSpec>,
    /// Applicability precedence (arago `Priority`); `None` = unranked.
    pub priority: Option<i64>,
    /// Applicability-scoped defaults, lifted to bindable [`ActionParam`]s.
    pub params: Vec<ActionParam>,
}

/// Lift one deployed applicability in full — guards + priority + scoped
/// parameter defaults. The additive sibling of [`lift_applicability`] (which
/// stays as the guard-only view the hard gate consumes).
#[must_use]
pub fn lift_applicability_full(app: &RegisteredApplicability) -> ConcreteApplicability {
    ConcreteApplicability {
        guards: lift_applicability(&app.model_filters),
        priority: app.priority,
        params: app
            .parameters
            .iter()
            .map(|p| ActionParam {
                name: p.name.clone(),
                mandatory: false,
                default: Some(p.value.clone()),
            })
            .collect(),
    }
}

/// Lift a whole `GET /applicabilities` response in full: handler id →
/// [`ConcreteApplicability`]. Deterministic order (the source is a
/// [`BTreeMap`], sorted by handler id).
#[must_use]
pub fn lift_applicabilities_full(
    apps: &RegisteredApplicabilities,
) -> BTreeMap<String, ConcreteApplicability> {
    apps.iter()
        .map(|(handler, app)| (handler.clone(), lift_applicability_full(app)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ssh_execute_command() -> RegisteredCapability {
        let mut mandatory = BTreeMap::new();
        mandatory.insert(
            "command".to_owned(),
            RegisteredParam {
                description: Some("the shell command to run".to_owned()),
                default: None,
            },
        );
        mandatory.insert(
            "host".to_owned(),
            RegisteredParam {
                description: Some("target host".to_owned()),
                default: None,
            },
        );
        let mut optional = BTreeMap::new();
        optional.insert(
            "user".to_owned(),
            RegisteredParam {
                description: Some("ssh user".to_owned()),
                default: Some("root".to_owned()),
            },
        );
        RegisteredCapability {
            description: Some("run a command over SSH".to_owned()),
            mandatory_parameters: mandatory,
            optional_parameters: optional,
        }
    }

    #[test]
    fn lifts_concrete_params_mandatory_first_then_optional() {
        let cap = lift_capability("ExecuteCommand", &ssh_execute_command());
        assert_eq!(cap.name, "ExecuteCommand");
        assert_eq!(cap.description.as_deref(), Some("run a command over SSH"));
        // BTreeMap order: mandatory {command, host} then optional {user}.
        assert_eq!(
            cap.params,
            vec![
                ActionParam {
                    name: "command".to_owned(),
                    mandatory: true,
                    default: None
                },
                ActionParam {
                    name: "host".to_owned(),
                    mandatory: true,
                    default: None
                },
                ActionParam {
                    name: "user".to_owned(),
                    mandatory: false,
                    default: Some("root".to_owned())
                },
            ]
        );
    }

    #[test]
    fn lifts_a_full_capabilities_map_sorted_by_name() {
        let mut caps: RegisteredCapabilities = BTreeMap::new();
        caps.insert("ExecuteCommand".to_owned(), ssh_execute_command());
        caps.insert("CheckStatus".to_owned(), RegisteredCapability::default());
        let lifted = lift_registration(&caps);
        // Sorted by capability name: CheckStatus before ExecuteCommand.
        assert_eq!(lifted.len(), 2);
        assert_eq!(lifted[0].name, "CheckStatus");
        assert!(lifted[0].params.is_empty());
        assert_eq!(lifted[1].name, "ExecuteCommand");
        assert_eq!(lifted[1].params.len(), 3);
    }

    #[test]
    fn the_lifted_signature_drives_bind_parameters() {
        // The B2-lift signature is exactly what the dispatch's bind step consumes:
        // a concrete ActionParam[] feeds action_ws::bind_parameters.
        let cap = lift_capability("ExecuteCommand", &ssh_execute_command());
        let supplied = vec![
            ("command".to_owned(), "uptime".to_owned()),
            ("host".to_owned(), "node-1".to_owned()),
        ];
        let bound = crate::action_ws::bind_parameters(&supplied, &cap.params)
            .expect("mandatory present; optional default filled");
        // `user` was omitted → its default "root" is filled in by bind.
        assert!(bound.iter().any(|(k, v)| k == "user" && v == "root"));
        assert!(bound.iter().any(|(k, v)| k == "command" && v == "uptime"));
    }

    #[test]
    fn model_filter_lifts_to_state_guard_field_for_field() {
        let filter = ModelFilter {
            var: "ogit/Network/Machine/class".to_owned(),
            mode: Some("equals".to_owned()),
            value: "Linux".to_owned(),
        };
        assert_eq!(
            model_filter_to_guard(&filter),
            KausalSpec::StateGuard {
                guard_field: "ogit/Network/Machine/class".to_owned(),
                guard_values: vec!["Linux".to_owned()],
            }
        );
    }

    #[test]
    fn applicability_lifts_each_filter_to_a_guard() {
        let filters = vec![
            ModelFilter {
                var: "os".to_owned(),
                mode: None,
                value: "Linux".to_owned(),
            },
            ModelFilter {
                var: "env".to_owned(),
                mode: Some("equals".to_owned()),
                value: "prod".to_owned(),
            },
        ];
        let guards = lift_applicability(&filters);
        assert_eq!(guards.len(), 2);
        assert_eq!(
            guards[1],
            KausalSpec::StateGuard {
                guard_field: "env".to_owned(),
                guard_values: vec!["prod".to_owned()],
            }
        );
    }

    /// The canonical `ssh_based_actions` applicability stanza from
    /// `AdaWorldAPI/ActionHandlers` (`aae.yaml`, verbatim values): Priority 50,
    /// `ModelFilter{NodeType, string, Machine}`, and the applicability-scoped
    /// `SshOpts` default. The full lift must carry all three.
    #[test]
    fn ssh_stanza_lifts_priority_guards_and_scoped_params() {
        let app = RegisteredApplicability {
            model_filters: vec![ModelFilter {
                var: "NodeType".to_owned(),
                mode: Some("string".to_owned()),
                value: "Machine".to_owned(),
            }],
            priority: Some(50),
            parameters: vec![ApplicabilityParam {
                name: "SshOpts".to_owned(),
                value: "-o StrictHostKeyChecking=no -o BatchMode=yes -o LogLevel=quiet".to_owned(),
            }],
        };
        let full = lift_applicability_full(&app);
        assert_eq!(full.priority, Some(50));
        assert_eq!(
            full.guards,
            vec![KausalSpec::StateGuard {
                guard_field: "NodeType".to_owned(),
                guard_values: vec!["Machine".to_owned()],
            }]
        );
        assert_eq!(
            full.params,
            vec![ActionParam {
                name: "SshOpts".to_owned(),
                mandatory: false,
                default: Some(
                    "-o StrictHostKeyChecking=no -o BatchMode=yes -o LogLevel=quiet".to_owned()
                ),
            }]
        );
    }

    /// Applicability-scoped params concatenate onto the capability signature and
    /// bind exactly like arago expands them into the `Command` template: the
    /// engine supplies nothing for `SshOpts`, the applicability default fills it.
    #[test]
    fn applicability_params_drive_bind_parameters_like_arago_injection() {
        let cap = lift_capability("ExecuteCommand", &ssh_execute_command());
        let app = RegisteredApplicability {
            parameters: vec![ApplicabilityParam {
                name: "SshOpts".to_owned(),
                value: "-o BatchMode=yes".to_owned(),
            }],
            ..Default::default()
        };
        let full = lift_applicability_full(&app);
        let mut signature = cap.params.clone();
        signature.extend(full.params);
        let supplied = vec![
            ("command".to_owned(), "uptime".to_owned()),
            ("host".to_owned(), "node-1".to_owned()),
        ];
        let bound = crate::action_ws::bind_parameters(&supplied, &signature)
            .expect("mandatory present; applicability default filled");
        assert!(
            bound
                .iter()
                .any(|(k, v)| k == "SshOpts" && v == "-o BatchMode=yes")
        );
    }

    /// The lift preserves per-parameter descriptions (the DTO carried them;
    /// the lift used to drop them — ground-truth gap closed 2026-07-08).
    #[test]
    fn lift_preserves_per_param_descriptions() {
        let cap = lift_capability("ExecuteCommand", &ssh_execute_command());
        assert_eq!(
            cap.param_descriptions.get("command").map(String::as_str),
            Some("the shell command to run")
        );
        assert_eq!(
            cap.param_descriptions.get("user").map(String::as_str),
            Some("ssh user")
        );
    }

    #[test]
    fn applicabilities_map_lifts_per_handler_guard_sets() {
        let mut apps: RegisteredApplicabilities = BTreeMap::new();
        apps.insert(
            "ssh-handler".to_owned(),
            RegisteredApplicability {
                model_filters: vec![ModelFilter {
                    var: "ogit/_type".to_owned(),
                    mode: Some("equals".to_owned()),
                    value: "ogit/Network/Machine".to_owned(),
                }],
                ..Default::default()
            },
        );
        apps.insert(
            "noop-handler".to_owned(),
            RegisteredApplicability::default(),
        );
        let lifted = lift_applicabilities(&apps);
        // Keyed by handler id (BTreeMap → sorted): noop-handler, ssh-handler.
        assert_eq!(lifted.len(), 2);
        assert!(lifted["noop-handler"].is_empty());
        assert_eq!(
            lifted["ssh-handler"],
            vec![KausalSpec::StateGuard {
                guard_field: "ogit/_type".to_owned(),
                guard_values: vec!["ogit/Network/Machine".to_owned()],
            }]
        );
    }
}
