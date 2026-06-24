//! OGIT **Automation** domain → OGAR **DO arm** (behavioral arm, schema level).
//!
//! The structural sibling [`crate::into_class`] lifts an OGIT entity into a
//! [`Class`] (the THINK arm). This module lifts the *actionable* Automation
//! entities — HIRO's actuator vocabulary — into [`ActionDef`]s (the DO arm),
//! per `docs/HIRO-DO-ARM-LIFT.md` §4–§5.
//!
//! # The lossless-DO rule (the governing invariant)
//!
//! `docs/HIRO-DO-ARM-LIFT.md` §1: **DO is lossless iff the `ActionDef`
//! *points to* the body (content-addressed) instead of *compressing it into*
//! a flat target (a DDL `DEFINE EVENT … WHEN … THEN …`).** This is
//! `I-VSA-IDENTITIES` applied to the DO arm — an identity that points to
//! content, never a bundle of the content's register.
//!
//! Concretely here: the KnowledgeItem's opaque body lives in the
//! `knowledgeItemFormalRepresentation` mandatory attribute. This lift **records
//! the attribute name** ([`payload_attribute`]) and **never inlines or reparses
//! the bytes** — [`ActionDef::body_source`] stays `None` at the schema level
//! (the schema carries no instance body; the instance lift fills it behind a
//! content hash, never by flattening into DDL).
//!
//! # Scope — schema level only (the producer half of `PROBE-OGAR-DO-ARM-LIFT`)
//!
//! This is the *structural* half of the DO-arm probe: it proves the **shape**
//! lifts (entity → `ActionDef` contract template with the §4 field mapping).
//! The *behavioral* half — an `ActionDef` → adapter → execute → result
//! reproducing a KnowledgeItem's recorded behavior bit-for-bit — needs KI
//! *instances* (with bodies) and stays CONJECTURE until that corpus is run,
//! exactly as `PROBE-OGAR-RBAC-AUTHORIZE` gates the RBAC arm.

use ogar_vocab::{ActionDef, ActionSubject, KausalSpec, ModalSpec, TemporalSpec};

use super::EntityDecl;

/// OGAR application prefix for the OGIT **Automation** (HIRO) domain.
///
/// Mirrors the `ogit-erp` / `ogit-op` convention used by the other producers
/// (`ogar-adapter-ttl`, `ogar-from-elixir`). The OGIT namespace
/// `ogit.Automation:` lowers to this dashed app prefix.
pub const AUTOMATION_PREFIX: &str = "ogit-automation";

/// camelCase / PascalCase → `snake_case`, correct over acronym runs.
///
/// `MARSNodeTemplate` → `mars_node_template`, `environmentFilter` →
/// `environment_filter`, `knowledgeItemFormalRepresentation` →
/// `knowledge_item_formal_representation`. An uppercase letter starts a new
/// word when it follows a lowercase letter, or when it is the tail of an
/// acronym run immediately before a lowercase letter (the `S` → `N` boundary
/// in `MARS|Node`).
#[must_use]
pub fn snake(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::with_capacity(name.len() + 4);
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() {
            let prev_lower = i > 0 && chars[i - 1].is_ascii_lowercase();
            let prev_upper = i > 0 && chars[i - 1].is_ascii_uppercase();
            let next_lower = i + 1 < chars.len() && chars[i + 1].is_ascii_lowercase();
            if i > 0 && (prev_lower || (prev_upper && next_lower)) {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Local name of a CURIE — the part after the final `:`.
/// `ogit.Automation:MARSNodeTemplate` → `MARSNodeTemplate`; `ogit:contains`
/// → `contains`.
fn local(curie: &str) -> &str {
    curie.rsplit(':').next().unwrap_or(curie).trim()
}

/// Map an OGIT CURIE to the OGAR-canonical class identity.
///
/// - `ogit.Automation:MARSNodeTemplate` → `ogit-automation/mars_node_template`
/// - `ogit:Timeseries` → `ogit/timeseries`
/// - `ogit:Task` → `ogit/task`
///
/// The namespace token left of `:` selects the app prefix (`ogit.<Domain>` →
/// `ogit-<domain>`; bare `ogit` → `ogit`); the local name lowers to
/// `snake_case`.
#[must_use]
pub fn canonical_object_class(curie: &str) -> String {
    let (ns, name) = match curie.split_once(':') {
        Some((ns, name)) => (ns.trim(), name.trim()),
        None => ("ogit", curie.trim()),
    };
    let prefix = if let Some(domain) = ns.strip_prefix("ogit.") {
        format!("ogit-{}", domain.to_ascii_lowercase())
    } else {
        // bare `ogit` (core namespace) or any unprefixed token
        ns.to_ascii_lowercase()
    };
    format!("{prefix}/{}", snake(name))
}

/// The opaque-body attribute (the lossless-DO **pointer**), if the entity
/// declares one.
///
/// Recognized by the `…FormalRepresentation` suffix on a **mandatory**
/// attribute — for `KnowledgeItem` that is
/// `knowledgeItemFormalRepresentation` (the XML body the schema references but
/// never parses; `docs/HIRO-DO-ARM-LIFT.md` §3). Returned in `snake_case`.
/// The bytes are **never** read here — only the slot name.
#[must_use]
pub fn payload_attribute(entity: &EntityDecl) -> Option<String> {
    entity
        .mandatory_attributes
        .iter()
        .map(|curie| local(curie))
        .find(|name| name.ends_with("FormalRepresentation"))
        .map(snake)
}

/// Extract the Kausal precondition from an Automation entity.
///
/// Per `docs/HIRO-DO-ARM-LIFT.md` §4:
/// - a `contains Trigger` allowed-edge → [`KausalSpec::LifecycleTrigger`]
///   (the `event` is the contained entity's local name);
/// - else a mandatory `environmentFilter` attribute (the `ActionApplicability`
///   shape) → [`KausalSpec::StateGuard`] on that field;
/// - else `None` (fires unconditionally at the right temporal point).
#[must_use]
pub fn kausal_from_entity(entity: &EntityDecl) -> Option<KausalSpec> {
    // `contains Trigger` → LifecycleTrigger
    if let Some((_, target)) = entity
        .allowed
        .iter()
        .find(|(verb, target)| local(verb) == "contains" && local(target) == "Trigger")
    {
        return Some(KausalSpec::LifecycleTrigger {
            event: local(target).to_owned(),
        });
    }
    // mandatory `environmentFilter` (ActionApplicability) → StateGuard
    if entity
        .mandatory_attributes
        .iter()
        .any(|curie| local(curie) == "environmentFilter")
    {
        return Some(KausalSpec::StateGuard {
            guard_field: snake("environmentFilter"),
            guard_values: Vec::new(),
        });
    }
    None
}

/// The OGIT relation verbs that participate in the DO-arm mapping
/// (`docs/HIRO-DO-ARM-LIFT.md` §4) — recorded as the extraction provenance on
/// [`ActionDef::decorators`].
const ACTUATOR_VERBS: &[&str] = &[
    "relates",   // → object_class (the MARSNodeTemplate the action targets)
    "contains",  // → kausal (the Trigger)
    "uses",      // → params / defaults (the Variables)
    "utilizes",  // → params / defaults (Intent's Variables)
    "worksOn",   // → ActionInvocation binding (the AutomationIssue)
    "solves",    // → intent (the Task)
    "generates", // → intent (the Timeseries)
];

/// Is this Automation entity an **action carrier** (→ a standalone
/// [`ActionDef`])?
///
/// True iff it declares an opaque body ([`payload_attribute`]) **or** a
/// `contains Trigger` lifecycle edge. `KnowledgeItem` qualifies (it has both);
/// `ActionHandler` / `ActionApplicability` / `ActionCapability` / `Trigger` /
/// `Intent` do **not** — they are *parts* of the contract that map to
/// sub-fields (the adapter/membrane, a `StateGuard`, the params), per §4.
#[must_use]
pub fn is_actionable(entity: &EntityDecl) -> bool {
    payload_attribute(entity).is_some()
        || entity
            .allowed
            .iter()
            .any(|(verb, target)| local(verb) == "contains" && local(target) == "Trigger")
}

/// The canonical action verb (the `predicate`) for a known action carrier.
///
/// `KnowledgeItem` → `execute` — sourced from the entity's own
/// `dcterms:description` (*"the KnowledgeItem executes operations"*), not
/// invented. Any other carrier defaults to its own `snake_case` name (the
/// instance lift overrides with the concrete KI name, per §5).
fn predicate_for(entity: &EntityDecl) -> String {
    match entity.name.as_str() {
        "KnowledgeItem" => "execute".to_owned(),
        other => snake(other),
    }
}

/// Lift one actionable Automation entity into its [`ActionDef`] contract
/// **template** (`docs/HIRO-DO-ARM-LIFT.md` §5).
///
/// Returns `None` for non-actionable entities (the contract *parts* — handler,
/// capability, applicability, trigger, intent — which map to sub-fields via
/// [`kausal_from_entity`] / [`payload_attribute`], not to a standalone def).
///
/// Field mapping (§4):
/// - `predicate` ← [`predicate_for`] (the entity's canonical verb)
/// - `object_class` ← the `relates` allowed-edge target ([`canonical_object_class`])
/// - `kausal` ← [`kausal_from_entity`] (`contains Trigger` → `LifecycleTrigger`)
/// - `decorators` ← the [`ACTUATOR_VERBS`] present (extraction provenance)
/// - `default_subject` = `System`, `default_temporal` = `Deferred`,
///   `default_modal` = `Async` — the automation-engine domain defaults
///   (every KI runs in the background on an issue; overridable per invocation)
/// - `body_source` = `None` — the body is **pointed to**
///   ([`payload_attribute`]), never inlined (the lossless-DO rule).
#[must_use]
pub fn into_action_def(entity: &EntityDecl) -> Option<ActionDef> {
    if !is_actionable(entity) {
        return None;
    }

    let entity_id = format!("{AUTOMATION_PREFIX}/{}", snake(&entity.name));
    let predicate = predicate_for(entity);
    let identity = format!("{entity_id}::action_def::{predicate}");

    // object_class ← the `relates` allowed-edge (the targeted MARSNodeTemplate).
    // Falls back to the entity itself when no `relates` edge is present.
    let object_class = entity
        .allowed
        .iter()
        .find(|(verb, _)| local(verb) == "relates")
        .map(|(_, target)| canonical_object_class(target))
        .unwrap_or_else(|| entity_id.clone());

    let mut decorators: Vec<String> = entity
        .allowed
        .iter()
        .map(|(verb, _)| local(verb).to_owned())
        .filter(|v| ACTUATOR_VERBS.contains(&v.as_str()))
        .collect();
    decorators.dedup();

    let mut def = ActionDef::new(identity, predicate, object_class);
    def.default_subject = ActionSubject::System;
    def.default_temporal = TemporalSpec::Deferred;
    def.default_modal = ModalSpec::Async;
    def.kausal = kausal_from_entity(entity);
    def.decorators = decorators;
    // body_source intentionally left None — the lossless-DO rule: the body is
    // POINTED TO via `payload_attribute(entity)`, never inlined here.
    Some(def)
}

/// Lift every actionable entity in a set into its DO-arm [`ActionDef`].
/// The non-actionable parts are silently skipped (they contribute sub-fields,
/// not standalone defs).
#[must_use]
pub fn lift_action_defs(entities: &[EntityDecl]) -> Vec<ActionDef> {
    entities.iter().filter_map(into_action_def).collect()
}

// ─────────────────────────────────────────────────────────────────────
// The arago ActionHandler contract — parity with arago's HIRO ActionHandler
// (`github.com/arago/ActionHandlers`, `arago/python-hiro-*-actionhandler`).
//
// An arago ActionHandler is a daemon that registers a **Configuration** + one
// or more **Capabilities** + **Applicabilities** with the HIRO engine, receives
// actions over the action-ws API, executes them, and returns a result. The OGIT
// `NTO/Automation` ontology IS that contract's schema:
//
//     ActionHandler  ──provides──►  ActionApplicability  ──provides──►  ActionCapability
//        (the daemon)                  (node-match guard)                  (named op + I/O)
//
// arago's deployed-handler config (the SSH/Stonebranch YAML) maps field-for-field:
//   Capability   { Name, Description, Command/Interpreter, Parameter[]{Name,Mandatory,Default} }
//   Applicability{ Priority, ModelFilter{Var,Mode,Value}, Capability, Parameter[] }
//
// The parity points (see `docs/ARAGO-ACTIONHANDLER-PARITY.md`):
//   Applicability.ModelFilter{Var,Mode,Value} → KausalSpec::StateGuard{field,[value]}
//   Capability.Name                           → ActionDef.predicate
//   Capability.Parameter[] / resultParameters → the typed I/O signature (below)
//   Capability.Command/Interpreter            → ActionDef.body_source (pointed-to, lossless-DO)
// ─────────────────────────────────────────────────────────────────────

/// One typed parameter of a capability/applicability — an element of the
/// action's I/O signature. Mirrors arago's deployed-handler `Parameter
/// { Name, Mandatory, Default }` (the SSH / Stonebranch handler config shape).
///
/// At the *schema* level the OGIT ontology declares only that the param *slots*
/// exist (the `mandatoryParameters` / `optionalParameters` / `resultParameters`
/// attributes); the concrete `(name, mandatory, default)` tuples come from a
/// *deployed* handler config (the instance lift — see the parity doc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionParam {
    /// Parameter name (arago `Parameter.Name`).
    pub name: String,
    /// Whether the parameter is required (arago `Parameter.Mandatory`).
    pub mandatory: bool,
    /// Default value when not supplied (arago `Parameter.Default`).
    pub default: Option<String>,
}

/// An **ActionCapability** — a named operation plus its typed I/O signature.
/// Mirrors arago's `Capability { Name, Description, Command/Interpreter,
/// Parameter[], resultParameters }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySlot {
    /// Canonical identity (`ogit-automation/action_capability`).
    pub identity: String,
    /// Declares a `mandatoryParameters` slot (the required-input signature).
    pub declares_mandatory_params: bool,
    /// Declares an `optionalParameters` slot (the optional-input signature).
    pub declares_optional_params: bool,
    /// Declares a `resultParameters` slot — **the output signature**. This is
    /// the half that makes an action *runnable*: a caller knows what the
    /// capability returns (arago `resultParameters` → the action result).
    pub declares_result_params: bool,
}

/// An **ActionApplicability** — the node-match guard selecting *when/where* a
/// handler applies. arago's `ModelFilter { Var, Mode, Value }` is OGAR's
/// [`KausalSpec::StateGuard`] (`Var` → `guard_field`, `Value` → `guard_values`);
/// the schema-level `environmentFilter` attribute is its carrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicabilitySlot {
    /// Canonical identity (`ogit-automation/action_applicability`).
    pub identity: String,
    /// The node-match guard (the `environmentFilter` → `StateGuard`).
    pub kausal: Option<KausalSpec>,
    /// The capabilities this applicability `provides`.
    pub capabilities: Vec<CapabilitySlot>,
}

/// The full **ActionHandler** contract — arago's ActionHandler assembled from
/// the OGIT `provides` graph (`ActionHandler → ActionApplicability →
/// ActionCapability`). The OGAR-native parity surface for an arago HIRO
/// ActionHandler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionHandlerSpec {
    /// Canonical identity (`ogit-automation/action_handler`).
    pub identity: String,
    /// The applicabilities this handler `provides` (each gating its capabilities).
    pub applicabilities: Vec<ApplicabilitySlot>,
}

/// Find an entity by its local name (`ActionApplicability`) in a set.
fn find_by_local<'a>(entities: &'a [EntityDecl], local_name: &str) -> Option<&'a EntityDecl> {
    entities.iter().find(|e| e.name == local_name)
}

/// Does the entity declare the given parameter slot (in either its mandatory or
/// optional attribute list)? — e.g. `"resultParameters"`.
fn declares_param_slot(entity: &EntityDecl, slot: &str) -> bool {
    entity
        .mandatory_attributes
        .iter()
        .chain(entity.optional_attributes.iter())
        .any(|curie| local(curie) == slot)
}

/// The local names an entity `provides` (the `ogit:provides` allowed-edges).
fn provides_targets(entity: &EntityDecl) -> impl Iterator<Item = &str> {
    entity
        .allowed
        .iter()
        .filter(|(verb, _)| local(verb) == "provides")
        .map(|(_, target)| local(target))
}

/// Lift an `ActionCapability` entity into a [`CapabilitySlot`].
fn capability_slot(entity: &EntityDecl) -> CapabilitySlot {
    CapabilitySlot {
        identity: canonical_object_class(&entity.curie),
        declares_mandatory_params: declares_param_slot(entity, "mandatoryParameters"),
        declares_optional_params: declares_param_slot(entity, "optionalParameters"),
        declares_result_params: declares_param_slot(entity, "resultParameters"),
    }
}

/// Lift an `ActionApplicability` entity into an [`ApplicabilitySlot`], resolving
/// the capabilities it `provides` against `entities`.
fn applicability_slot(entity: &EntityDecl, entities: &[EntityDecl]) -> ApplicabilitySlot {
    let capabilities = provides_targets(entity)
        .filter_map(|target| find_by_local(entities, target))
        .map(capability_slot)
        .collect();
    ApplicabilitySlot {
        identity: canonical_object_class(&entity.curie),
        kausal: kausal_from_entity(entity),
        capabilities,
    }
}

/// Assemble the full arago ActionHandler contract from a set of OGIT Automation
/// entities — walk `ActionHandler → provides → ActionApplicability → provides →
/// ActionCapability`. Returns `None` if the set carries no `ActionHandler`.
///
/// This is the OGAR-native, ontology-grounded parity surface for an arago HIRO
/// ActionHandler: the SAME `(handler, applicabilities[guard], capabilities[I/O])`
/// shape arago registers with the HIRO engine, expressed in OGAR types.
#[must_use]
pub fn assemble_action_handler(entities: &[EntityDecl]) -> Option<ActionHandlerSpec> {
    let handler = find_by_local(entities, "ActionHandler")?;
    let applicabilities = provides_targets(handler)
        .filter_map(|target| find_by_local(entities, target))
        .map(|app| applicability_slot(app, entities))
        .collect();
    Some(ActionHandlerSpec {
        identity: canonical_object_class(&handler.curie),
        applicabilities,
    })
}

// ───────────────────────────────────────────────────────────── tests ──
//
// The schema-level half of PROBE-OGAR-DO-ARM-LIFT: prove the §4 field mapping
// lifts from the real vendored Automation TTL (no fixtures — the bytes are the
// frozen OGIT source, same calibration discipline as the MARS structural arm).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TtlDeclaration, ttl::parse_file};

    const KNOWLEDGE_ITEM_TTL: &str =
        include_str!("../../../vocab/imports/ogit/NTO/Automation/entities/KnowledgeItem.ttl");
    const ACTION_HANDLER_TTL: &str =
        include_str!("../../../vocab/imports/ogit/NTO/Automation/entities/ActionHandler.ttl");
    const ACTION_APPLICABILITY_TTL: &str =
        include_str!("../../../vocab/imports/ogit/NTO/Automation/entities/ActionApplicability.ttl");
    const TRIGGER_TTL: &str =
        include_str!("../../../vocab/imports/ogit/NTO/Automation/entities/Trigger.ttl");

    fn entity(src: &str) -> EntityDecl {
        match parse_file(src).expect("parses") {
            TtlDeclaration::Entity(e) => e,
            other => panic!("expected entity, got {other:?}"),
        }
    }

    #[test]
    fn snake_handles_acronym_runs() {
        assert_eq!(snake("MARSNodeTemplate"), "mars_node_template");
        assert_eq!(snake("KnowledgeItem"), "knowledge_item");
        assert_eq!(snake("environmentFilter"), "environment_filter");
        assert_eq!(
            snake("knowledgeItemFormalRepresentation"),
            "knowledge_item_formal_representation"
        );
        assert_eq!(snake("Trigger"), "trigger");
    }

    #[test]
    fn canonical_object_class_maps_curies() {
        assert_eq!(
            canonical_object_class("ogit.Automation:MARSNodeTemplate"),
            "ogit-automation/mars_node_template"
        );
        assert_eq!(canonical_object_class("ogit:Timeseries"), "ogit/timeseries");
        assert_eq!(canonical_object_class("ogit:Task"), "ogit/task");
    }

    /// The canonical KnowledgeItem → ActionDef lift (§5 worked example),
    /// asserted against the real OGIT TTL bytes.
    #[test]
    fn knowledge_item_lifts_to_action_def() {
        let ki = entity(KNOWLEDGE_ITEM_TTL);
        assert!(
            is_actionable(&ki),
            "KnowledgeItem must be an action carrier"
        );
        let def = into_action_def(&ki).expect("KnowledgeItem → ActionDef");

        assert_eq!(def.predicate, "execute");
        assert_eq!(
            def.identity,
            "ogit-automation/knowledge_item::action_def::execute"
        );
        // object_class ← `relates ogit.Automation:MARSNodeTemplate`
        assert_eq!(def.object_class, "ogit-automation/mars_node_template");
        // kausal ← `contains ogit.Automation:Trigger`
        assert_eq!(
            def.kausal,
            Some(KausalSpec::LifecycleTrigger {
                event: "Trigger".to_owned()
            })
        );
        // automation-engine domain defaults
        assert_eq!(def.default_subject, ActionSubject::System);
        assert_eq!(def.default_temporal, TemporalSpec::Deferred);
        assert_eq!(def.default_modal, ModalSpec::Async);
        // extraction provenance: the §4 actuator verbs present on the KI
        for verb in [
            "relates",
            "contains",
            "uses",
            "worksOn",
            "solves",
            "generates",
        ] {
            assert!(
                def.decorators.iter().any(|d| d == verb),
                "decorator `{verb}` missing from {:?}",
                def.decorators
            );
        }
    }

    /// The lossless-DO invariant (§1): the body is **pointed to**, never
    /// inlined. The KI's opaque `knowledgeItemFormalRepresentation` slot is
    /// recorded by name; `body_source` stays `None` at the schema level.
    #[test]
    fn knowledge_item_payload_is_pointed_to_never_inlined() {
        let ki = entity(KNOWLEDGE_ITEM_TTL);
        assert_eq!(
            payload_attribute(&ki).as_deref(),
            Some("knowledge_item_formal_representation"),
            "the opaque body slot must be recorded by name"
        );
        let def = into_action_def(&ki).expect("ActionDef");
        assert!(
            def.body_source.is_none(),
            "lossless-DO violated: body was inlined into body_source ({:?})",
            def.body_source
        );
    }

    /// `ActionApplicability` is a contract *part* — it maps to a `StateGuard`
    /// (its `environmentFilter`), not to a standalone `ActionDef`.
    #[test]
    fn action_applicability_maps_to_state_guard_not_a_def() {
        let app = entity(ACTION_APPLICABILITY_TTL);
        assert!(
            into_action_def(&app).is_none(),
            "applicability is not a def"
        );
        assert_eq!(
            kausal_from_entity(&app),
            Some(KausalSpec::StateGuard {
                guard_field: "environment_filter".to_owned(),
                guard_values: Vec::new(),
            })
        );
    }

    /// `ActionHandler` is the adapter/membrane (§4) — never a standalone def.
    #[test]
    fn action_handler_is_not_a_standalone_def() {
        let handler = entity(ACTION_HANDLER_TTL);
        assert!(!is_actionable(&handler));
        assert!(into_action_def(&handler).is_none());
    }

    /// `Trigger` is the kausal source, not an action carrier itself.
    #[test]
    fn trigger_is_not_a_standalone_def() {
        let trigger = entity(TRIGGER_TTL);
        assert!(!is_actionable(&trigger));
        assert!(into_action_def(&trigger).is_none());
    }

    /// The set lift skips the parts and keeps only the carriers.
    #[test]
    fn lift_action_defs_keeps_only_carriers() {
        let entities = [
            entity(KNOWLEDGE_ITEM_TTL),
            entity(ACTION_HANDLER_TTL),
            entity(ACTION_APPLICABILITY_TTL),
            entity(TRIGGER_TTL),
        ];
        let defs = lift_action_defs(&entities);
        assert_eq!(
            defs.len(),
            1,
            "only KnowledgeItem is a standalone ActionDef"
        );
        assert_eq!(defs[0].predicate, "execute");
    }

    // ── arago ActionHandler contract parity (the `provides` graph) ──

    const ACTION_CAPABILITY_TTL: &str =
        include_str!("../../../vocab/imports/ogit/NTO/Automation/entities/ActionCapability.ttl");

    /// The full arago ActionHandler contract assembles from the vendored OGIT
    /// `provides` graph: ActionHandler → ActionApplicability → ActionCapability.
    #[test]
    fn assembles_the_full_action_handler_contract() {
        let entities = [
            entity(ACTION_HANDLER_TTL),
            entity(ACTION_APPLICABILITY_TTL),
            entity(ACTION_CAPABILITY_TTL),
        ];
        let spec = assemble_action_handler(&entities).expect("ActionHandler present");

        assert_eq!(spec.identity, "ogit-automation/action_handler");
        // ActionHandler provides one ActionApplicability.
        assert_eq!(spec.applicabilities.len(), 1);
        let app = &spec.applicabilities[0];
        assert_eq!(app.identity, "ogit-automation/action_applicability");
        // arago ModelFilter{Var,Mode,Value} ⟷ OGAR StateGuard (environmentFilter).
        assert_eq!(
            app.kausal,
            Some(KausalSpec::StateGuard {
                guard_field: "environment_filter".to_owned(),
                guard_values: Vec::new(),
            })
        );
        // The applicability provides one ActionCapability with its I/O signature.
        assert_eq!(app.capabilities.len(), 1);
        let cap = &app.capabilities[0];
        assert_eq!(cap.identity, "ogit-automation/action_capability");
        assert!(cap.declares_mandatory_params, "capability declares inputs");
        assert!(cap.declares_optional_params);
        // The OUTPUT signature — what makes the action runnable.
        assert!(
            cap.declares_result_params,
            "capability must declare resultParameters (the output signature)"
        );
    }

    /// No ActionHandler in the set ⇒ no contract to assemble.
    #[test]
    fn assemble_returns_none_without_a_handler() {
        let entities = [entity(KNOWLEDGE_ITEM_TTL), entity(TRIGGER_TTL)];
        assert!(assemble_action_handler(&entities).is_none());
    }

    /// An `ActionParam` carries the arago `Parameter{Name,Mandatory,Default}`
    /// I/O-signature tuple (the instance-level shape the executor binds).
    #[test]
    fn action_param_models_the_arago_parameter_tuple() {
        let p = ActionParam {
            name: "command".to_owned(),
            mandatory: true,
            default: None,
        };
        assert_eq!(p.name, "command");
        assert!(p.mandatory);
        assert!(p.default.is_none());
    }
}
