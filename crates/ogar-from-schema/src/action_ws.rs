//! HIRO **Action API** (`action-ws`) protocol core — the testable runtime
//! binding for an OGAR-native ActionHandler (parity brick **B2**).
//!
//! This is the *protocol core* of the action-ws adapter — the typed messages
//! plus the binding `submitAction → ActionInvocation → sendActionResult` — with
//! **no live WebSocket and no command execution**. It is the deterministic,
//! unit-tested heart that an outer transport (the live `tokio-tungstenite` loop)
//! and the executor (parity brick **B1**, the `ExecTarget` runner) wrap.
//!
//! Source: the HIRO 7 Action API tutorial (`tutorial-action-handler-action-api`),
//! transcribed verbatim in `docs/ARAGO-ACTIONHANDLER-PARITY.md` §2. The lifecycle
//!
//! ```text
//!   engine ──submitAction──► handler ──acknowledged{200}──► engine
//!                            handler  (execute)
//!                            handler ──sendActionResult──►   engine ──acknowledged──►
//! ```
//!
//! maps field-for-field onto OGAR's [`ActionInvocation`] Rubicon lifecycle
//! (`Pending → Committed`): `submitAction` builds a `Pending` invocation
//! ([`submit_to_invocation`]); the engine's final ack is the Lance commit; a
//! `Committed` invocation yields the `sendActionResult` ([`invocation_to_result`]).
//! Parameter binding ([`bind_parameters`]) validates the engine's `parameters`
//! against the capability's [`ActionParam`] signature — the same check arago's
//! Python handler performs before executing.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use ogar_vocab::{
    ActionDef, ActionInvocation, ActionState, ActionSubject, LokalSpec, ModalSpec, TemporalSpec,
};

use crate::do_arm::ActionParam;

/// The `action-ws` WebSocket connect path (HIRO Action API 1.0). The full URL is
/// `wss://<host>/api/action-ws/1.0/connect`.
pub const ACTION_WS_PATH: &str = "/api/action-ws/1.0/connect";

/// The WebSocket subprotocol header value carrying the auth token — HIRO passes
/// the token as `sec-websocket-protocol: token-$TOKEN`.
#[must_use]
pub fn auth_subprotocol(token: &str) -> String {
    format!("token-{token}")
}

/// Spec bounds on a `submitAction` / `sendActionResult` correlation `id`
/// (12–256 chars). [`validate_id`] enforces it.
pub const ID_MIN_LEN: usize = 12;
/// Upper bound on the correlation `id` length (spec).
pub const ID_MAX_LEN: usize = 256;

/// A `submitAction` message (engine → handler). The engine asks the handler to
/// run `capability` on a target with the supplied `parameters`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SubmitAction {
    /// Correlation id — `"$appId:$requestId"`. Carried through to the result.
    pub id: String,
    /// The capability requested (e.g. `"ExecuteCommand"`) — must match an
    /// [`ActionDef::predicate`].
    pub capability: String,
    /// The handler id this action is routed to.
    pub handler: String,
    /// The instance scope (tenant).
    pub scope: Option<String>,
    /// The action inputs (`{host, command, user, …}`), as `(key, value)` pairs.
    pub parameters: Vec<(String, String)>,
    /// Per-action SLA in milliseconds.
    pub timeout_millis: Option<i64>,
}

/// An `acknowledged` message (either direction): receipt confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Acknowledged {
    /// The id of the message being acknowledged.
    pub id: String,
    /// Status code (`200` on success).
    pub code: u16,
    /// Human-readable note.
    pub message: String,
}

/// A `sendActionResult` message (handler → engine): the outcome payload.
///
/// Per the `action-ws` spec the `result` is a **single string** (max
/// `1048576` chars) — the capability's `resultParameters` JSON-encoded into one
/// field (build it with [`json_object`]). The engine replies `acknowledged` /
/// `negativeAcknowledged`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SendActionResult {
    /// The same correlation id as the originating [`SubmitAction`].
    pub id: String,
    /// The result value — a JSON object string of the bound `resultParameters`
    /// (spec: `string`, max 1 MiB).
    pub result: String,
}

/// Max length of the [`SendActionResult::result`] string (spec: `1048576`).
pub const MAX_RESULT_LEN: usize = 1_048_576;

/// A `negativeAcknowledged` message (engine ↔ handler): receipt *rejection*
/// (e.g. `code = 400`). The negative twin of [`Acknowledged`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NegativeAcknowledged {
    /// The id of the message being rejected.
    pub id: String,
    /// Error code (e.g. `400`).
    pub code: u16,
    /// Error description.
    pub message: String,
}

/// A `configChanged` notification (engine → handler): the handler's
/// capabilities / applicabilities changed; the handler must re-fetch them from
/// the REST Action API (`GET /capabilities`, `GET /applicabilities`). Carries
/// no payload beyond `type`; the handler replies `acknowledged`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConfigChanged;

/// An asynchronous `error` message (engine → handler) — not tied to a specific
/// request id.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InboundError {
    /// Error code.
    pub code: u16,
    /// Error details.
    pub message: String,
}

/// Errors in the protocol binding (the pure core — no I/O errors here).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActionWsError {
    /// `submitAction.capability` does not match the [`ActionDef`] it was routed to.
    CapabilityMismatch {
        /// The def's predicate.
        expected: String,
        /// The submitAction's capability.
        got: String,
    },
    /// A mandatory parameter of the capability signature was not supplied and
    /// has no default.
    MissingMandatoryParam(String),
    /// A result was requested from an invocation that has not reached
    /// [`ActionState::Committed`] (the Rubicon crossing).
    NotCommitted(ActionState),
    /// A correlation `id` outside the spec bounds (12–256 chars); carries the
    /// offending length.
    InvalidId(usize),
    /// The encoded `result` exceeds [`MAX_RESULT_LEN`] (spec: 1 MiB); carries the
    /// offending length.
    ResultTooLarge(usize),
}

impl core::fmt::Display for ActionWsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CapabilityMismatch { expected, got } => {
                write!(
                    f,
                    "capability mismatch: def expects `{expected}`, got `{got}`"
                )
            }
            Self::MissingMandatoryParam(p) => write!(f, "missing mandatory parameter `{p}`"),
            Self::NotCommitted(s) => write!(f, "invocation not committed (state = {s:?})"),
            Self::InvalidId(n) => write!(f, "correlation id length {n} out of bounds (12..=256)"),
            Self::ResultTooLarge(n) => write!(f, "result length {n} exceeds 1 MiB"),
        }
    }
}

impl std::error::Error for ActionWsError {}

/// The handler's immediate receipt acknowledgement (code 200), echoing the
/// action's `id`. Sent before execution; the engine re-sends `submitAction`
/// until this arrives (at-least-once → idempotency).
#[must_use]
pub fn acknowledge(msg: &SubmitAction) -> Acknowledged {
    Acknowledged {
        id: msg.id.clone(),
        code: 200,
        message: "Received the action".to_owned(),
    }
}

/// Reject a message by id (the `negativeAcknowledged` twin of [`acknowledge`]).
#[must_use]
pub fn negative_acknowledge(
    id: &str,
    code: u16,
    message: impl Into<String>,
) -> NegativeAcknowledged {
    NegativeAcknowledged {
        id: id.to_owned(),
        code,
        message: message.into(),
    }
}

/// Validate a correlation `id` against the spec bounds (12–256 chars).
///
/// # Errors
///
/// [`ActionWsError::InvalidId`] when the length is out of range.
pub fn validate_id(id: &str) -> Result<(), ActionWsError> {
    if (ID_MIN_LEN..=ID_MAX_LEN).contains(&id.len()) {
        Ok(())
    } else {
        Err(ActionWsError::InvalidId(id.len()))
    }
}

/// Encode `(key, value)` pairs as a JSON object string — the wire form of the
/// [`SendActionResult::result`] field (the bound `resultParameters`). A minimal,
/// correctly-escaping encoder; the live transport may use `serde_json` instead.
#[must_use]
pub fn json_object(pairs: &[(String, String)]) -> String {
    let mut s = String::from("{");
    for (i, (k, v)) in pairs.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        json_string(k, &mut s);
        s.push(':');
        json_string(v, &mut s);
    }
    s.push('}');
    s
}

/// Append `raw` as a JSON string literal (RFC 8259 escaping) to `out`.
fn json_string(raw: &str, out: &mut String) {
    out.push('"');
    for c in raw.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Bind the engine-supplied `parameters` to the capability's [`ActionParam`]
/// signature: every mandatory param must be supplied (or have a default);
/// optional params fall back to their default when present, and are dropped
/// when absent. Returns the bound `(name, value)` set in signature order — the
/// same validation arago's handler runs before executing the `Command`.
///
/// # Errors
///
/// [`ActionWsError::MissingMandatoryParam`] if a mandatory param is neither
/// supplied nor defaulted.
pub fn bind_parameters(
    supplied: &[(String, String)],
    signature: &[ActionParam],
) -> Result<Vec<(String, String)>, ActionWsError> {
    let mut bound = Vec::with_capacity(signature.len());
    for p in signature {
        if let Some((_, v)) = supplied.iter().find(|(k, _)| k == &p.name) {
            bound.push((p.name.clone(), v.clone()));
        } else if let Some(default) = &p.default {
            bound.push((p.name.clone(), default.clone()));
        } else if p.mandatory {
            return Err(ActionWsError::MissingMandatoryParam(p.name.clone()));
        }
        // optional + absent + no default → omitted
    }
    Ok(bound)
}

/// The target node an action acts on — arago routes by the `host` parameter
/// (the MARS node); fall back to the handler id when absent.
fn target_node(msg: &SubmitAction) -> String {
    msg.parameters
        .iter()
        .find(|(k, _)| k == "host" || k == "node")
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| msg.handler.clone())
}

/// Build a **`Pending`** [`ActionInvocation`] from a `submitAction`, realizing
/// `def` (whose [`predicate`](ActionDef::predicate) must equal the action's
/// `capability`). This is the `submitAction → ActionInvocation` half of the
/// lifecycle; the invocation then passes through the RBAC/guard/MUL gate
/// (`commit_via` in `lance-graph-contract`) before reaching `Committed`.
///
/// Field mapping (`docs/ARAGO-ACTIONHANDLER-PARITY.md` §2):
/// `capability`→`def.predicate`, `id`→`idempotency_key`, `handler`→`lokal.actor`,
/// `scope`→`lokal.tenant`, target node→`object_instance`. Automation defaults:
/// `subject = System`, `temporal = Deferred`, `modal = Idempotent` (at-least-once).
///
/// # Errors
///
/// [`ActionWsError::CapabilityMismatch`] if `msg.capability != def.predicate`.
pub fn submit_to_invocation(
    msg: &SubmitAction,
    def: &ActionDef,
) -> Result<ActionInvocation, ActionWsError> {
    if msg.capability != def.predicate {
        return Err(ActionWsError::CapabilityMismatch {
            expected: def.predicate.clone(),
            got: msg.capability.clone(),
        });
    }
    let object_instance = target_node(msg);
    let identity = format!("{}::invocation::{}", def.object_class, msg.id);
    let mut inv = ActionInvocation::new(identity, def.identity.clone(), object_instance);
    inv.subject = ActionSubject::System;
    inv.temporal = TemporalSpec::Deferred;
    inv.modal = ModalSpec::Idempotent;
    inv.state = ActionState::Pending;
    inv.idempotency_key = Some(msg.id.clone());
    // LokalSpec is #[non_exhaustive] — build via Default + field set, not a literal.
    let mut lokal = LokalSpec::default();
    lokal.actor = Some(msg.handler.clone());
    lokal.tenant = msg.scope.clone();
    inv.lokal = lokal;
    Ok(inv)
}

/// Build the `sendActionResult` from a **`Committed`** invocation plus the
/// executor's result payload (the bound `resultParameters`). Only a committed
/// invocation (the Rubicon crossing) yields a result — a `Pending` / `Failed` /
/// `Cancelled` invocation has nothing to report on the success path.
///
/// # Errors
///
/// [`ActionWsError::NotCommitted`] if the invocation has not reached
/// [`ActionState::Committed`].
pub fn invocation_to_result(
    inv: &ActionInvocation,
    result_params: &[(String, String)],
) -> Result<SendActionResult, ActionWsError> {
    if inv.state != ActionState::Committed {
        return Err(ActionWsError::NotCommitted(inv.state));
    }
    // The spec's `result` is a single string (max 1 MiB) — JSON-encode the bound
    // resultParameters into it.
    let result = json_object(result_params);
    if result.len() > MAX_RESULT_LEN {
        return Err(ActionWsError::ResultTooLarge(result.len()));
    }
    Ok(SendActionResult {
        id: inv.idempotency_key.clone().unwrap_or_default(),
        result,
    })
}

// ─────────────────────────────────────────────────────────────────────
// The handler reactive core — turn one inbound `submitAction` into the
// ordered outbound messages, with execution behind a trait (the B1 seam).
// Socket-free and pure given the executor; the live transport (B2-transport)
// just ships these messages, and the RBAC/guard gate (`commit_via`,
// lance-graph-contract) wraps the executor downstream.
// ─────────────────────────────────────────────────────────────────────

/// The executor seam (parity brick **B1**): run a bound capability and return
/// its `resultParameters`. Implemented per `ExecTarget` (SSH / REST / native) by
/// rs-graph-llm's `graph-flow-action`; modelled here as a trait so the dispatch
/// core is testable without real I/O. The impl is also where the
/// RBAC/guard gate (`commit_via`) runs — it owns the `lance-graph` dependency
/// OGAR's producer crate deliberately does not.
pub trait CapabilityExecutor {
    /// Execute `capability` with the `bound` parameters.
    ///
    /// `Ok(result_params)` → the success `resultParameters`; `Err(message)` → a
    /// failure the handler reports back in the result.
    fn execute(
        &self,
        capability: &str,
        bound: &[(String, String)],
    ) -> Result<Vec<(String, String)>, String>;
}

/// The immediate receipt response to a `submitAction`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Receipt {
    /// Accepted (code 200) — execution follows, then a [`SendActionResult`].
    Acknowledged(Acknowledged),
    /// Rejected before execution (invalid id / unknown capability) — no result.
    NegativeAcknowledged(NegativeAcknowledged),
}

/// The ordered outbound reaction to one inbound `submitAction`: the receipt,
/// then (when accepted) the eventual `sendActionResult`. The live transport
/// emits `receipt` first, then `result` when present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerReaction {
    /// The receipt — `Acknowledged` on accept, `NegativeAcknowledged` on reject.
    pub receipt: Receipt,
    /// The result message — present iff the action was accepted (it ran, or it
    /// failed *after* acceptance and reports the failure in the result).
    pub result: Option<SendActionResult>,
}

/// OGAR convention: a post-acceptance failure (bad params, executor error) is
/// reported in the `sendActionResult` as a one-field `{"error": "<message>"}`
/// object (the action was already acknowledged, so a result is owed). The exact
/// HIRO failure-reporting field is confirmed against a live engine in
/// B2-transport; until then this is the documented OGAR shape.
fn error_result(id: &str, message: &str) -> SendActionResult {
    SendActionResult {
        id: id.to_owned(),
        result: json_object(&[("error".to_owned(), message.to_owned())]),
    }
}

/// Drive one inbound `submitAction` through the handler's reactive flow:
/// validate → ack-or-nack → bind → execute (via `executor`) → result.
///
/// - invalid `id` / capability ≠ `def.predicate` → reject **before** ack
///   (`NegativeAcknowledged`, no result).
/// - accepted → `Acknowledged`, then bind the inputs against `signature` and run
///   `executor`; the outcome (success `resultParameters`, or an `{"error":…}`
///   on bind/exec failure) rides in the `sendActionResult`.
///
/// Pure given `executor`; the same flow arago's Python daemon runs, minus the
/// socket (B2-transport) and the real command (the `executor` impl, B1).
#[must_use]
pub fn handle_submit(
    msg: &SubmitAction,
    def: &ActionDef,
    signature: &[ActionParam],
    executor: &dyn CapabilityExecutor,
) -> HandlerReaction {
    // Reject malformed / mis-routed actions BEFORE acknowledging.
    if let Err(e) = validate_id(&msg.id) {
        return HandlerReaction {
            receipt: Receipt::NegativeAcknowledged(negative_acknowledge(
                &msg.id,
                400,
                e.to_string(),
            )),
            result: None,
        };
    }
    if msg.capability != def.predicate {
        return HandlerReaction {
            receipt: Receipt::NegativeAcknowledged(negative_acknowledge(
                &msg.id,
                400,
                format!("unknown capability `{}`", msg.capability),
            )),
            result: None,
        };
    }

    // Accept: acknowledge receipt, then bind + execute.
    let ack = acknowledge(msg);
    let result = match bind_parameters(&msg.parameters, signature) {
        Err(e) => error_result(&msg.id, &e.to_string()),
        Ok(bound) => match executor.execute(&msg.capability, &bound) {
            Ok(params) => SendActionResult {
                id: msg.id.clone(),
                result: json_object(&params),
            },
            Err(e) => error_result(&msg.id, &e),
        },
    };
    HandlerReaction {
        receipt: Receipt::Acknowledged(ack),
        result: Some(result),
    }
}

// ───────────────────────────────────────────────────────────── tests ──
//
// The pure protocol core: the full submitAction → bind → invocation(Pending)
// → (Committed) → sendActionResult flow, deterministic and socket-free.

#[cfg(test)]
mod tests {
    use super::*;

    /// An ExecuteCommand-shaped capability signature (the arago SSH handler):
    /// mandatory `command`, optional `timeout` defaulting to `60000`.
    fn execute_command_signature() -> Vec<ActionParam> {
        vec![
            ActionParam {
                name: "command".to_owned(),
                mandatory: true,
                default: None,
            },
            ActionParam {
                name: "timeout".to_owned(),
                mandatory: false,
                default: Some("60000".to_owned()),
            },
        ]
    }

    fn execute_command_def() -> ActionDef {
        ActionDef::new(
            "ogit-automation/action_capability::action_def::ExecuteCommand",
            "ExecuteCommand",
            "ogit-automation/mars_machine",
        )
    }

    fn submit() -> SubmitAction {
        SubmitAction {
            id: "app1:req42".to_owned(),
            capability: "ExecuteCommand".to_owned(),
            handler: "handler-7".to_owned(),
            scope: Some("tenant-A".to_owned()),
            parameters: vec![
                ("host".to_owned(), "node-9".to_owned()),
                ("command".to_owned(), "uptime".to_owned()),
            ],
            timeout_millis: Some(60_000),
        }
    }

    #[test]
    fn acknowledge_echoes_id_with_200() {
        let ack = acknowledge(&submit());
        assert_eq!(ack.id, "app1:req42");
        assert_eq!(ack.code, 200);
    }

    #[test]
    fn bind_parameters_fills_default_and_keeps_supplied() {
        let bound =
            bind_parameters(&submit().parameters, &execute_command_signature()).expect("binds");
        // `command` supplied, `timeout` defaulted; signature order preserved.
        assert_eq!(
            bound,
            vec![
                ("command".to_owned(), "uptime".to_owned()),
                ("timeout".to_owned(), "60000".to_owned()),
            ]
        );
    }

    #[test]
    fn bind_parameters_rejects_missing_mandatory() {
        let supplied = vec![("timeout".to_owned(), "5".to_owned())];
        let err = bind_parameters(&supplied, &execute_command_signature()).unwrap_err();
        assert_eq!(
            err,
            ActionWsError::MissingMandatoryParam("command".to_owned())
        );
    }

    #[test]
    fn submit_builds_pending_invocation_with_provenance() {
        let inv = submit_to_invocation(&submit(), &execute_command_def()).expect("builds");
        assert_eq!(inv.state, ActionState::Pending);
        assert_eq!(inv.object_instance, "node-9"); // routed by the `host` param
        assert_eq!(inv.idempotency_key.as_deref(), Some("app1:req42"));
        assert_eq!(inv.action_def, execute_command_def().identity);
        assert_eq!(inv.lokal.actor.as_deref(), Some("handler-7"));
        assert_eq!(inv.lokal.tenant.as_deref(), Some("tenant-A"));
        assert!(matches!(inv.modal, ModalSpec::Idempotent));
    }

    #[test]
    fn submit_rejects_capability_mismatch() {
        let mut bad = submit();
        bad.capability = "RunScript".to_owned();
        let err = submit_to_invocation(&bad, &execute_command_def()).unwrap_err();
        assert_eq!(
            err,
            ActionWsError::CapabilityMismatch {
                expected: "ExecuteCommand".to_owned(),
                got: "RunScript".to_owned(),
            }
        );
    }

    #[test]
    fn committed_invocation_yields_result_pending_does_not() {
        let mut inv = submit_to_invocation(&submit(), &execute_command_def()).expect("builds");

        // Pending → no result on the success path.
        let pending = invocation_to_result(&inv, &[]);
        assert_eq!(
            pending.unwrap_err(),
            ActionWsError::NotCommitted(ActionState::Pending)
        );

        // The Rubicon crossing (the gate would set this) → result emitted as a
        // JSON object string (the spec's single `result` field).
        inv.state = ActionState::Committed;
        let result =
            invocation_to_result(&inv, &[("output".to_owned(), "12:00 up 3 days".to_owned())])
                .expect("committed → result");
        assert_eq!(result.id, "app1:req42"); // correlation id round-trips
        assert_eq!(result.result, r#"{"output":"12:00 up 3 days"}"#);
    }

    #[test]
    fn negative_acknowledge_carries_code_and_message() {
        let nack = negative_acknowledge("app1:req42", 400, "bad capability");
        assert_eq!(nack.id, "app1:req42");
        assert_eq!(nack.code, 400);
        assert_eq!(nack.message, "bad capability");
    }

    #[test]
    fn validate_id_enforces_spec_bounds() {
        assert!(validate_id("123456789012").is_ok()); // 12 chars (min)
        assert_eq!(
            validate_id("short").unwrap_err(),
            ActionWsError::InvalidId(5)
        );
        let too_long = "x".repeat(257);
        assert_eq!(
            validate_id(&too_long).unwrap_err(),
            ActionWsError::InvalidId(257)
        );
    }

    #[test]
    fn json_object_escapes_correctly() {
        // Empty, simple, and escape-needing values.
        assert_eq!(json_object(&[]), "{}");
        assert_eq!(
            json_object(&[("k".to_owned(), "v".to_owned())]),
            r#"{"k":"v"}"#
        );
        assert_eq!(
            json_object(&[("out".to_owned(), "a\"b\\c\nd".to_owned())]),
            r#"{"out":"a\"b\\c\nd"}"#
        );
    }

    #[test]
    fn auth_subprotocol_prefixes_the_token() {
        assert_eq!(auth_subprotocol("abc123"), "token-abc123");
        assert_eq!(ACTION_WS_PATH, "/api/action-ws/1.0/connect");
    }

    /// The whole loop, end-to-end (socket-free): submit → ack → bind → invoke
    /// → commit → result, with the `id` correlating throughout.
    #[test]
    fn full_action_ws_roundtrip() {
        let msg = submit();
        let def = execute_command_def();

        let ack = acknowledge(&msg);
        assert_eq!(ack.code, 200);

        let _bound = bind_parameters(&msg.parameters, &execute_command_signature()).expect("bind");

        let mut inv = submit_to_invocation(&msg, &def).expect("invoke");
        assert_eq!(inv.state, ActionState::Pending);

        // (the executor + commit_via gate run here; we simulate the crossing)
        inv.state = ActionState::Committed;

        let result =
            invocation_to_result(&inv, &[("exitcode".to_owned(), "0".to_owned())]).expect("result");
        assert_eq!(result.id, msg.id);
        assert_eq!(result.result, r#"{"exitcode":"0"}"#);
    }

    // ── the handler reactive core (handle_submit + the B1 executor seam) ──

    /// A mock executor: returns a fixed success, or a fixed error.
    struct MockExecutor(Result<Vec<(String, String)>, String>);
    impl CapabilityExecutor for MockExecutor {
        fn execute(
            &self,
            _capability: &str,
            _bound: &[(String, String)],
        ) -> Result<Vec<(String, String)>, String> {
            self.0.clone()
        }
    }

    /// A spec-valid submit (id ≥ 12 chars) for the dispatch tests.
    fn valid_submit() -> SubmitAction {
        let mut s = submit();
        s.id = "app1:req-000042".to_owned(); // 15 chars
        s
    }

    #[test]
    fn handle_submit_accepts_runs_and_returns_result() {
        let exec = MockExecutor(Ok(vec![("output".to_owned(), "ok".to_owned())]));
        let r = handle_submit(
            &valid_submit(),
            &execute_command_def(),
            &execute_command_signature(),
            &exec,
        );
        match r.receipt {
            Receipt::Acknowledged(a) => assert_eq!(a.code, 200),
            other => panic!("expected ack, got {other:?}"),
        }
        let res = r.result.expect("result present");
        assert_eq!(res.id, "app1:req-000042");
        assert_eq!(res.result, r#"{"output":"ok"}"#);
    }

    #[test]
    fn handle_submit_rejects_unknown_capability_before_ack() {
        let exec = MockExecutor(Ok(vec![]));
        let mut bad = valid_submit();
        bad.capability = "RunScript".to_owned();
        let r = handle_submit(
            &bad,
            &execute_command_def(),
            &execute_command_signature(),
            &exec,
        );
        assert!(matches!(r.receipt, Receipt::NegativeAcknowledged(_)));
        assert!(r.result.is_none(), "rejected actions carry no result");
    }

    #[test]
    fn handle_submit_rejects_invalid_id() {
        let exec = MockExecutor(Ok(vec![]));
        let short = submit(); // id "app1:req42" is 10 chars (< 12)
        let r = handle_submit(
            &short,
            &execute_command_def(),
            &execute_command_signature(),
            &exec,
        );
        match r.receipt {
            Receipt::NegativeAcknowledged(n) => assert_eq!(n.code, 400),
            other => panic!("expected nack, got {other:?}"),
        }
    }

    #[test]
    fn handle_submit_reports_bind_failure_after_ack() {
        let exec = MockExecutor(Ok(vec![]));
        let mut no_command = valid_submit();
        no_command.parameters = vec![("host".to_owned(), "node-9".to_owned())]; // no `command`
        let r = handle_submit(
            &no_command,
            &execute_command_def(),
            &execute_command_signature(),
            &exec,
        );
        // Accepted (acked), but the result carries the bind error.
        assert!(matches!(r.receipt, Receipt::Acknowledged(_)));
        let res = r.result.expect("result present");
        assert!(
            res.result.contains("error"),
            "bind failure reported in result: {}",
            res.result
        );
    }

    #[test]
    fn handle_submit_reports_executor_failure_in_result() {
        let exec = MockExecutor(Err("ssh: connection refused".to_owned()));
        let r = handle_submit(
            &valid_submit(),
            &execute_command_def(),
            &execute_command_signature(),
            &exec,
        );
        assert!(matches!(r.receipt, Receipt::Acknowledged(_)));
        let res = r.result.expect("result present");
        assert_eq!(res.result, r#"{"error":"ssh: connection refused"}"#);
    }
}
