//! A **test double** of the HIRO action gateway, mirroring graph_conn's mock.
//! It is not a gateway, and nothing here says what the real one does.
//!
//! The only gateway in the HIRO client sources is graph_conn's test mock
//! (`lib/graph_conn/test/mock_socket.ex`, `S` below, and `lib/graph_conn/mock.ex`,
//! `M`). This module reproduces what that mock does between an invoker connection
//! and a handler connection, so the OGAR invoker ([`crate::invoke_session`]) and
//! the OGAR handler ([`crate::session`]) can be joined in one process and tested
//! as a pair. It is sans-IO like them: no socket, no thread, no clock read, no
//! sleep. Time is a [`Millis`] argument; what the mock would do after a delay is
//! queued and released by [`MockGateway::on_tick`] at [`MockGateway::next_wakeup`].
//!
//! The mock matches JSON maps, it does not use the typed codec: it reads `type`,
//! `id` and `capability` and forwards the rest untouched (`S:161-169`). So does
//! this module (`serde_json::Value`), and a request reaches the handler side with
//! its parameters as the invoker typed them.
//!
//! # What the mock does, per frame (the mirrored behaviour)
//!
//! | frame from | what | source |
//! |---|---|---|
//! | invoker | `submitAction` whose `capability` is not a key of the capability map: `negativeAcknowledged` `404`, message `"capability X not found"`, to the invoker at once; nothing is forwarded (oracle 44) | `S:277-288` |
//! | invoker | `submitAction` with a known capability: registers the invoker under the request `id`, waits [`MockConfig::ack_delay_ms`], sends `acknowledged` (`type` and `id` only, no `code`) to the invoker, then the request, as received, to the handler (oracle 45) | `S:257-298` |
//! | invoker | `submitAction` while a refusal is armed: the refusal is sent instead of the ack, before the capability is looked at, and nothing is forwarded (see [`MockGateway::deny_next_request`]) | `S:202-217`, `M:303-350` |
//! | invoker | `submitAction` while a forward drop is armed: acked, never forwarded | `S:215`, `:291-293`, `M:344-350` |
//! | handler | `sendActionResult`: the mock itself sends `acknowledged` (`type` and `id` only) to the handler at once, and the result frame, re-encoded, to the invoker once for every registration of that `id` | `S:220-246` |
//! | either | `acknowledged`: recorded and nothing else. The handler's ack of a submit is **not** relayed to the invoker (the mock sent its own), and the invoker's ack of a result is **not** relayed to the handler (the mock acked the result itself) | `S:171-172` |
//! | either | `submitAction` with capability `"nack"`: `negativeAcknowledged` `403` `"Forbidden"` back to the sender's side; the mock-only trigger [`FORBIDDEN_CAPABILITY`] | `S:177-193` |
//! | either | `clientHello`: recorded | `S:174-175` |
//! | either | anything else (a `negativeAcknowledged`, a `sendActionResult` from the invoker, a `submitAction` from the handler, a frame with no `type`): a typed `error` frame, code 400, to the sender alone | `S:248-255` |
//!
//! Three things in that table are easy to get wrong and are on purpose:
//!
//! * **The handler never reaches the invoker with an ack.** The invoker's ack
//!   of its submit is the mock's, sent after [`MockConfig::ack_delay_ms`]. Losing
//!   the handler's ack changes nothing.
//! * **The mock acks the handler's result by itself.** The invoker's ack of a
//!   result stops nothing on the handler side; only the mock's ack does.
//! * **Registrations are not de-duplicated.** The mock's registry is a
//!   `:duplicate` registry (`lib/graph_conn/test/mock_server.ex:78`), and every
//!   known-capability submit registers the invoker socket under the `id` again
//!   (`S:261-262`). An invoker that resends a submit is therefore sent the
//!   handler's result twice. The OGAR invoker acks each and completes once.
//!
//! ## Where the source is silent, and what was chosen
//!
//! * The mock sleeps a random 1 to 5 ms before the ack (`S:264-266`). The delay
//!   is a [`MockConfig::ack_delay_ms`], [`DEFAULT_ACK_DELAY_MS`] = 1 (the bottom
//!   of the range), so a test is deterministic. It delays the forward too, as in
//!   the source (both come after the sleep).
//! * The error text of a typed `error` frame is `"invalid action message "` and
//!   Elixir's `inspect(msg)` of the decoded map (`S:251`). `inspect` is not
//!   reproduced; the compact JSON of the frame stands in for it.
//! * A frame that is not JSON crashes the mock's socket process (`Jason.decode!`,
//!   `S:163`); here it is [`MockError::NotJson`] and nothing else happens. A frame
//!   longer than the limit the `hello` advertised closes the socket (cowboy's
//!   `max_frame_size`, `S:112-120`); here it is [`MockError::FrameTooBig`].
//! * The mock keeps one capability map and one applicability map and serves
//!   them as the two REST bodies (`M:100-141`); there is no HTTP here, the
//!   bodies are returned as text.
//!
//! # Fault injection (not part of the mirrored behaviour)
//!
//! Everything under [`Faults`], [`MockGateway::inject`] and
//! [`MockGateway::restart_handler`] is for tests and is not something the mock
//! does. A [`FaultRule`] drops, duplicates or delays the next frames in one
//! [`Direction`]; it applies where a frame crosses the gateway's edge: a frame
//! from a peer when it arrives, a frame to a peer when it is emitted (the ack,
//! the forward and the refusals included). The armed refusals
//! ([`MockGateway::deny_next_request`] and its siblings) are not faults in this
//! sense: they mirror `M`'s own arming functions.
//!
//! ```
//! use ogar_action_handler::mock_gateway::{MockConfig, MockGateway, Side};
//!
//! let mut gw = MockGateway::new(MockConfig::default());
//! gw.put_capabilities(r#"{"ExecuteCommand": {"timeout": 60000, "command": null}}"#).unwrap();
//! let submit = r#"{"type":"submitAction","id":"a","handler":"h","capability":"Nope","parameters":{}}"#;
//! let out = gw.on_invoker_text(0, submit).unwrap();
//! assert_eq!(out.len(), 1);
//! assert_eq!(out[0].to, Side::Invoker);
//! assert!(out[0].text.contains("capability Nope not found"));
//! ```

use std::fmt;

use serde_json::{Map, Value, json};

use crate::applicability::{ApplicabilityError, parse_applies_on_definitions};
use crate::submit_state::Millis;
use crate::wire::{self, ActionWsFrame, Hello};

/// The capability name that makes the mock answer a `403` NACK whoever sends it
/// (`S:177-193`; mock-only).
pub const FORBIDDEN_CAPABILITY: &str = "nack";
/// `code` of the NACK for an unknown capability (`S:283`).
pub const NOT_FOUND_CODE: u16 = 404;
/// `code` of the mock's NACK for [`FORBIDDEN_CAPABILITY`] (`S:183`).
pub const FORBIDDEN_CODE: u16 = 403;
/// `code` of the typed `error` frame for a frame the mock has no clause for (`S:251`).
pub const INVALID_MESSAGE_CODE: u16 = 400;
/// The handler id the mock's default applicabilities hold (`M:122-124`).
pub const DEFAULT_HANDLER_ID: &str = "action_handler";
/// The wait before the ack, ms: the bottom of the mock's random 1 to 5 ms
/// (`S:264-266`). A choice, see the module documentation.
pub const DEFAULT_ACK_DELAY_MS: Millis = 1;
/// The `message` of the in-band 429 (`S:306`).
pub const RATE_LIMIT_MESSAGE: &str = "Rate limit exceeded";
/// The `message` of the in-band 503 (`S:318`).
pub const UNAVAILABLE_MESSAGE: &str = "Upstream unavailable";
/// The `upstream` of the in-band 503 (`S:318`).
pub const UNAVAILABLE_UPSTREAM: &str = "action";
/// The frame limit cowboy enforces when no `max_frame_bytes` was advertised
/// (`S:117`).
pub const DEFAULT_SOCKET_FRAME_LIMIT: u64 = 1_000_000;

/// One of the two connections the mock serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// The invoker's socket (registry key `action_invoker`).
    Invoker,
    /// The handler's socket (registry key `action_handler`).
    Handler,
}

/// A way a frame can travel through the gateway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// From the invoker to the gateway.
    InvokerToGateway,
    /// From the gateway to the invoker.
    GatewayToInvoker,
    /// From the handler to the gateway.
    HandlerToGateway,
    /// From the gateway to the handler.
    GatewayToHandler,
}

impl Direction {
    /// The direction a frame sent by `side` takes.
    #[must_use]
    pub fn from_side(side: Side) -> Self {
        match side {
            Side::Invoker => Self::InvokerToGateway,
            Side::Handler => Self::HandlerToGateway,
        }
    }

    /// The direction a frame sent to `side` takes.
    #[must_use]
    pub fn to_side(side: Side) -> Self {
        match side {
            Side::Invoker => Self::GatewayToInvoker,
            Side::Handler => Self::GatewayToHandler,
        }
    }
}

/// A frame the gateway hands to one of its peers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Out {
    /// The peer it is for.
    pub to: Side,
    /// The JSON text frame.
    pub text: String,
}

/// Why the mock did nothing with a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MockError {
    /// The text is not JSON; the real mock's socket process would crash.
    NotJson(String),
    /// The frame is longer than the limit this side's `hello` advertised; the
    /// real socket would be closed.
    FrameTooBig {
        /// The sender.
        side: Side,
        /// The frame's length in bytes.
        bytes: u64,
        /// The limit.
        limit: u64,
    },
    /// A capabilities input is not `{name: {param: default-or-null}}`.
    BadCapabilities(String),
    /// An applicabilities input is not a list of definitions the converter reads.
    BadApplicabilities(ApplicabilityError),
    /// `put_applicabilities` for a handler id the mock does not hold (the
    /// source raises, `M:133`).
    UnknownHandler(String),
}

impl fmt::Display for MockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotJson(e) => write!(f, "frame is not JSON: {e}"),
            Self::FrameTooBig { side, bytes, limit } => {
                write!(
                    f,
                    "{side:?} frame of {bytes} bytes is over the limit {limit}"
                )
            }
            Self::BadCapabilities(e) => write!(f, "capabilities input: {e}"),
            Self::BadApplicabilities(e) => write!(f, "applicabilities input: {e}"),
            Self::UnknownHandler(h) => write!(f, "no applicabilities held for handler `{h}`"),
        }
    }
}

impl std::error::Error for MockError {}

/// The settings of a mock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MockConfig {
    /// The wait between a known-capability submit and its ack and forward.
    pub ack_delay_ms: Millis,
    /// The `max_frame_bytes` of the invoker's `hello`; `None` advertises none
    /// (`M:384-403`).
    pub invoker_max_frame_bytes: Option<u64>,
    /// The `max_frame_bytes` of the handler's `hello`.
    pub handler_max_frame_bytes: Option<u64>,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            ack_delay_ms: DEFAULT_ACK_DELAY_MS,
            invoker_max_frame_bytes: None,
            handler_max_frame_bytes: None,
        }
    }
}

/// What an armed refusal answers a submit with (`M:303-350`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denial {
    /// An in-band 429 with `retryAfterMs`.
    RateLimited {
        /// `error.retryAfterMs`.
        retry_after_ms: u64,
    },
    /// An in-band 503.
    UpstreamUnavailable,
    /// No answer at all: no ack, no error, no forward.
    Ignore,
}

/// How a [`FaultRule`] treats a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// The frame is lost.
    Drop,
    /// The frame is delivered twice, one after the other.
    Duplicate,
    /// The frame is delivered this many ms late.
    Delay(Millis),
}

/// A rule for the next frames in one direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultRule {
    /// Which way.
    pub direction: Direction,
    /// Only frames whose `type` is this; `None` is every frame. A typeless frame
    /// (the in-band 429 and 503) has the type `""`.
    pub frame_type: Option<String>,
    /// What to do.
    pub fault: Fault,
    /// How many more frames the rule applies to.
    pub remaining: u32,
}

/// A fault that was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultEvent {
    /// When.
    pub at: Millis,
    /// Which way the frame was going.
    pub direction: Direction,
    /// What happened to it.
    pub fault: Fault,
    /// The `type` of the frame (`""` if typeless).
    pub frame_type: String,
    /// The frame's text.
    pub text: String,
}

/// The fault rules of a [`MockGateway`]. Rules are tried in the order they were
/// added; a frame takes the first that matches and has uses left.
#[derive(Debug, Clone, Default)]
pub struct Faults {
    rules: Vec<FaultRule>,
    log: Vec<FaultEvent>,
}

impl Faults {
    /// Add a rule.
    pub fn add(&mut self, rule: FaultRule) {
        self.rules.push(rule);
    }

    fn rule(&mut self, direction: Direction, frame_type: Option<&str>, fault: Fault, n: u32) {
        self.add(FaultRule {
            direction,
            frame_type: frame_type.map(str::to_owned),
            fault,
            remaining: n,
        });
    }

    /// Drop the next `n` frames in `direction`.
    pub fn drop_next(&mut self, direction: Direction, n: u32) {
        self.rule(direction, None, Fault::Drop, n);
    }

    /// Drop the next `n` frames of type `frame_type` in `direction`.
    pub fn drop_next_of(&mut self, direction: Direction, frame_type: &str, n: u32) {
        self.rule(direction, Some(frame_type), Fault::Drop, n);
    }

    /// Deliver the next `n` frames in `direction` twice.
    pub fn duplicate_next(&mut self, direction: Direction, n: u32) {
        self.rule(direction, None, Fault::Duplicate, n);
    }

    /// Deliver the next `n` frames of type `frame_type` in `direction` twice.
    pub fn duplicate_next_of(&mut self, direction: Direction, frame_type: &str, n: u32) {
        self.rule(direction, Some(frame_type), Fault::Duplicate, n);
    }

    /// Deliver the next `n` frames in `direction` `ms` late.
    pub fn delay_next(&mut self, direction: Direction, ms: Millis, n: u32) {
        self.rule(direction, None, Fault::Delay(ms), n);
    }

    /// Deliver the next `n` frames of type `frame_type` in `direction` `ms` late.
    pub fn delay_next_of(&mut self, direction: Direction, frame_type: &str, ms: Millis, n: u32) {
        self.rule(direction, Some(frame_type), Fault::Delay(ms), n);
    }

    /// How many frames the rules still apply to, in all.
    #[must_use]
    pub fn pending(&self) -> u32 {
        self.rules.iter().map(|r| r.remaining).sum()
    }

    /// Forget every rule that has not been used up.
    pub fn clear(&mut self) {
        self.rules.clear();
    }

    /// The faults applied so far, oldest first.
    #[must_use]
    pub fn log(&self) -> &[FaultEvent] {
        &self.log
    }

    fn take(&mut self, at: Millis, direction: Direction, text: &str) -> Option<Fault> {
        let frame_type = frame_type_of(text);
        let rule = self.rules.iter_mut().find(|r| {
            r.remaining > 0
                && r.direction == direction
                && r.frame_type.as_deref().is_none_or(|t| t == frame_type)
        })?;
        rule.remaining -= 1;
        let fault = rule.fault;
        self.log.push(FaultEvent {
            at,
            direction,
            fault,
            frame_type,
            text: text.to_owned(),
        });
        Some(fault)
    }
}

/// How an injected frame names the request it is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdField {
    /// `"id": "<id>"`.
    Present(String),
    /// No `id` key.
    Absent,
    /// `"id": null`.
    Null,
}

/// The `type` of a frame as text; `""` when there is none or it is not JSON.
#[must_use]
pub fn frame_type_of(text: &str) -> String {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| v.get("type").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
enum Item {
    /// A frame from a peer, held by a delay fault, to be processed.
    Inbound { side: Side, text: String },
    /// A frame to a peer that has still to pass the outbound faults.
    Emit { to: Side, text: String },
    /// A frame to a peer that has passed them and was delayed by one.
    Release { to: Side, text: String },
}

#[derive(Debug, Clone)]
struct Scheduled {
    due: Millis,
    seq: u64,
    item: Item,
}

/// The mock gateway: one invoker connection and one handler connection.
#[derive(Debug, Clone)]
pub struct MockGateway {
    config: MockConfig,
    clock: Millis,
    seq: u64,
    capabilities: Map<String, Value>,
    applicabilities: Map<String, Value>,
    /// The ids the invoker socket is registered under: one entry per known
    /// submit, repeats included (`S:261-262`, a `:duplicate` registry).
    registrations: Vec<String>,
    submitted: Vec<(String, Value)>,
    acknowledged: Vec<String>,
    client_hellos: Vec<(Side, Value)>,
    denial: Option<(Denial, u32)>,
    drop_forward: u32,
    queue: Vec<Scheduled>,
    faults: Faults,
}

impl Default for MockGateway {
    fn default() -> Self {
        Self::new(MockConfig::default())
    }
}

impl MockGateway {
    /// A mock with no capabilities and the default applicabilities
    /// `{"action_handler": {}}` (`M:122-124`).
    #[must_use]
    pub fn new(config: MockConfig) -> Self {
        let mut applicabilities = Map::new();
        applicabilities.insert(DEFAULT_HANDLER_ID.to_owned(), Value::Object(Map::new()));
        Self {
            config,
            clock: 0,
            seq: 0,
            capabilities: Map::new(),
            applicabilities,
            registrations: Vec::new(),
            submitted: Vec::new(),
            acknowledged: Vec::new(),
            client_hellos: Vec::new(),
            denial: None,
            drop_forward: 0,
            queue: Vec::new(),
            faults: Faults::default(),
        }
    }

    /// The settings.
    #[must_use]
    pub fn config(&self) -> &MockConfig {
        &self.config
    }

    // ------------------------------------------------------- registration --

    /// Merge capabilities given in the mock's simple input form,
    /// `{"<Name>": {"<param>": <default or null>}}`, into the held map: a `null`
    /// default is a mandatory parameter `{}`, any other value an optional one
    /// `{"default": V}`; a name already held is replaced (`M:106-118`, `:592-611`,
    /// verbatim in `mock_test.exs:73-93`).
    ///
    /// # Errors
    /// [`MockError::BadCapabilities`] when the input is not that shape.
    pub fn put_capabilities(&mut self, json: &str) -> Result<(), MockError> {
        let bad = |s: &str| MockError::BadCapabilities(s.to_owned());
        let Value::Object(input) =
            serde_json::from_str(json).map_err(|e| MockError::BadCapabilities(e.to_string()))?
        else {
            return Err(bad("not an object"));
        };
        let mut converted = Map::new();
        for (name, params) in input {
            let Value::Object(params) = params else {
                return Err(bad("a capability's value is not an object"));
            };
            let mut mandatory = Map::new();
            let mut optional = Map::new();
            for (param, default) in params {
                if default.is_null() {
                    mandatory.insert(param, Value::Object(Map::new()));
                } else {
                    optional.insert(param, json!({ "default": default }));
                }
            }
            converted.insert(
                name,
                json!({
                    "mandatoryParameters": Value::Object(mandatory),
                    "optionalParameters": Value::Object(optional),
                }),
            );
        }
        self.capabilities.extend(converted);
        Ok(())
    }

    /// Merge applicabilities given in the mock's definition-list input form (the
    /// one of `mock_test.exs:95-121`) under `handler_id`, as converted
    /// `{"<capability>": {"on <expr>": {"<param>": "<template>"}}}`
    /// (`M:129-141`, `:614-637`), by the existing
    /// [`parse_applies_on_definitions`]. A capability already held under that
    /// handler is replaced whole. As in the source, the held map then holds that
    /// handler alone: any other handler's applicabilities are dropped
    /// (`M:135-138`). Templates are stored verbatim, never expanded.
    ///
    /// # Errors
    /// [`MockError::UnknownHandler`] for a handler the mock holds nothing for
    /// (the source raises); [`MockError::BadApplicabilities`] for a malformed
    /// definition list. The converter here is stricter than the mock's about
    /// template values (a string is required).
    pub fn put_applicabilities(&mut self, handler_id: &str, json: &str) -> Result<(), MockError> {
        let Some(Value::Object(held)) = self.applicabilities.get(handler_id) else {
            return Err(MockError::UnknownHandler(handler_id.to_owned()));
        };
        let mut merged = held.clone();
        let rows = parse_applies_on_definitions(handler_id, json)
            .map_err(MockError::BadApplicabilities)?;
        let mut by_capability: Map<String, Value> = Map::new();
        for row in rows {
            let entry = by_capability
                .entry(row.capability.clone())
                .or_insert_with(|| Value::Object(Map::new()));
            let params: Map<String, Value> = row
                .parameters
                .iter()
                .map(|(k, v)| (k.clone(), Value::from(v.as_str())))
                .collect();
            if let Value::Object(ons) = entry {
                ons.insert(row.wire_key(), Value::Object(params));
            }
        }
        merged.extend(by_capability);
        let mut only = Map::new();
        only.insert(handler_id.to_owned(), Value::Object(merged));
        self.applicabilities = only;
        Ok(())
    }

    /// The body of `GET /api/0.9/action/capabilities` (`M:102-104`).
    #[must_use]
    pub fn capabilities_body(&self) -> String {
        Value::Object(self.capabilities.clone()).to_string()
    }

    /// The body of `GET /api/0.9/action/applicabilities` (`M:122-124`).
    #[must_use]
    pub fn applicabilities_body(&self) -> String {
        Value::Object(self.applicabilities.clone()).to_string()
    }

    // --------------------------------------------------------------- hello --

    /// The `hello` the mock sends a peer when it connects (`S:139-147`): `max_frame_bytes`
    /// only when one is configured for that side.
    #[must_use]
    pub fn hello_text(&self, side: Side) -> String {
        let max_frame_bytes = match side {
            Side::Invoker => self.config.invoker_max_frame_bytes,
            Side::Handler => self.config.handler_max_frame_bytes,
        };
        wire::encode(&ActionWsFrame::Hello(Hello { max_frame_bytes }))
    }

    fn frame_limit(&self, side: Side) -> u64 {
        match side {
            Side::Invoker => self.config.invoker_max_frame_bytes,
            Side::Handler => self.config.handler_max_frame_bytes,
        }
        .unwrap_or(DEFAULT_SOCKET_FRAME_LIMIT)
    }

    // ---------------------------------------------------- armed refusals --

    /// Answer the next `times` submits from the invoker with an in-band 429
    /// advertising `retry_after_ms`, in place of the ack, with nothing forwarded
    /// (`M:303-318`, `S:302-313`). Arming one of the three refusals replaces what
    /// another armed, as in the source (`M:309-310`).
    pub fn deny_next_request(&mut self, times: u32, retry_after_ms: u64) {
        self.denial = Some((Denial::RateLimited { retry_after_ms }, times));
    }

    /// Answer the next `times` submits with an in-band 503 carrying the request's
    /// id (`M:320-331`, `S:316-323`).
    pub fn fail_next_request_unavailable(&mut self, times: u32) {
        self.denial = Some((Denial::UpstreamUnavailable, times));
    }

    /// Leave the next `times` submits unanswered: no ack, no error, no forward
    /// (`M:333-342`).
    pub fn ignore_next_request(&mut self, times: u32) {
        self.denial = Some((Denial::Ignore, times));
    }

    /// Ack the next `times` known-capability submits and forward none of them
    /// (`M:344-350`). A submit with an unknown capability uses one up as well
    /// (`S:215-216`).
    pub fn drop_next_submit(&mut self, times: u32) {
        self.drop_forward = times;
    }

    fn take_denial(&mut self) -> Option<Denial> {
        match &mut self.denial {
            Some((d, n)) if *n > 0 => {
                *n -= 1;
                Some(*d)
            }
            _ => None,
        }
    }

    fn take_drop_forward(&mut self) -> bool {
        if self.drop_forward > 0 {
            self.drop_forward -= 1;
            true
        } else {
            false
        }
    }

    // ---------------------------------------------------------- fault API --

    /// The fault rules, to add to and to read.
    pub fn faults_mut(&mut self) -> &mut Faults {
        &mut self.faults
    }

    /// The fault rules.
    #[must_use]
    pub fn faults(&self) -> &Faults {
        &self.faults
    }

    /// Hand `to` a frame of the test's own making, now, bypassing the fault
    /// rules. Returns it for the caller to deliver. Not mirrored behaviour.
    #[must_use]
    pub fn inject(&mut self, now: Millis, to: Side, text: impl Into<String>) -> Vec<Out> {
        self.clock = self.clock.max(now);
        vec![Out {
            to,
            text: text.into(),
        }]
    }

    /// An in-band 429 to the invoker, shaped as the mock's (`S:302-313`), with the
    /// `id` as given. Not mirrored behaviour: the mock only sends one for an
    /// armed refusal.
    #[must_use]
    pub fn inject_rate_limited(
        &mut self,
        now: Millis,
        id: &IdField,
        retry_after_ms: u64,
    ) -> Vec<Out> {
        let error = json!({
            "code": 429,
            "message": RATE_LIMIT_MESSAGE,
            "retryAfterMs": retry_after_ms,
        });
        self.inject(now, Side::Invoker, typeless(error, id))
    }

    /// An in-band 503 to the invoker, shaped as the mock's (`S:316-323`), with
    /// the `id` as given.
    #[must_use]
    pub fn inject_unavailable(&mut self, now: Millis, id: &IdField) -> Vec<Out> {
        let error = json!({
            "code": 503,
            "message": UNAVAILABLE_MESSAGE,
            "upstream": UNAVAILABLE_UPSTREAM,
        });
        self.inject(now, Side::Invoker, typeless(error, id))
    }

    /// The handler connection restarted: the frames queued for it, and the ones
    /// it had sent that were still held by a delay, belong to the dead
    /// connection and are lost; the new connection is sent its `hello`. The
    /// handler session itself is the caller's to replace. Not mirrored behaviour.
    #[must_use]
    pub fn restart_handler(&mut self, now: Millis) -> Vec<Out> {
        self.clock = self.clock.max(now);
        self.queue.retain(|s| {
            !matches!(
                &s.item,
                Item::Emit {
                    to: Side::Handler,
                    ..
                } | Item::Release {
                    to: Side::Handler,
                    ..
                } | Item::Inbound {
                    side: Side::Handler,
                    ..
                }
            )
        });
        vec![Out {
            to: Side::Handler,
            text: self.hello_text(Side::Handler),
        }]
    }

    // ------------------------------------------------------------- frames --

    /// A frame from the invoker arrives at `now`. Returns the frames to deliver
    /// now; later ones come from [`MockGateway::on_tick`].
    ///
    /// # Errors
    /// A [`MockError`] for a frame that is not JSON or is over the frame limit.
    pub fn on_invoker_text(&mut self, now: Millis, text: &str) -> Result<Vec<Out>, MockError> {
        self.receive(now, Side::Invoker, text)
    }

    /// A frame from the handler arrives at `now`.
    ///
    /// # Errors
    /// As [`MockGateway::on_invoker_text`].
    pub fn on_handler_text(&mut self, now: Millis, text: &str) -> Result<Vec<Out>, MockError> {
        self.receive(now, Side::Handler, text)
    }

    /// Time passed: release what is due, in the order it was queued.
    pub fn on_tick(&mut self, now: Millis) -> Vec<Out> {
        self.clock = self.clock.max(now);
        let mut out = Vec::new();
        self.release_due(&mut out);
        out
    }

    /// The earliest time at which [`MockGateway::on_tick`] would release
    /// something.
    #[must_use]
    pub fn next_wakeup(&self) -> Option<Millis> {
        self.queue.iter().map(|s| s.due).min()
    }

    /// How many frames are queued.
    #[must_use]
    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    /// Every submit the invoker side made, in order: the request id (as JSON
    /// text) and its `parameters` (`M:526-529`).
    #[must_use]
    pub fn submitted(&self) -> &[(String, Value)] {
        &self.submitted
    }

    /// The ids of every `acknowledged` frame either peer sent, in order
    /// (`M:412-417`). The mock records them and relays none.
    #[must_use]
    pub fn acknowledged(&self) -> &[String] {
        &self.acknowledged
    }

    /// The `clientHello` frames received, with their sender (`M:405-410`).
    #[must_use]
    pub fn client_hellos(&self) -> &[(Side, Value)] {
        &self.client_hellos
    }

    /// How many times the invoker is registered under the request `id`.
    #[must_use]
    pub fn registrations_of(&self, id: &str) -> usize {
        let key = Value::from(id).to_string();
        self.registrations.iter().filter(|r| **r == key).count()
    }

    // ----------------------------------------------------------- internals --

    fn receive(&mut self, now: Millis, side: Side, text: &str) -> Result<Vec<Out>, MockError> {
        self.clock = self.clock.max(now);
        let bytes = text.len() as u64;
        let limit = self.frame_limit(side);
        if bytes > limit {
            return Err(MockError::FrameTooBig { side, bytes, limit });
        }
        let value: Value =
            serde_json::from_str(text).map_err(|e| MockError::NotJson(e.to_string()))?;
        let mut out = Vec::new();
        match self.faults.take(now, Direction::from_side(side), text) {
            None => self.process(now, side, &value, &mut out),
            Some(Fault::Drop) => {}
            Some(Fault::Duplicate) => {
                self.process(now, side, &value, &mut out);
                self.process(now, side, &value, &mut out);
            }
            Some(Fault::Delay(ms)) => self.defer(
                now.saturating_add(ms),
                Item::Inbound {
                    side,
                    text: text.to_owned(),
                },
            ),
        }
        // A zero-delay ack is already in the queue and due.
        self.release_due(&mut out);
        Ok(out)
    }

    fn defer(&mut self, due: Millis, item: Item) {
        self.seq += 1;
        self.queue.push(Scheduled {
            due,
            seq: self.seq,
            item,
        });
    }

    fn release_due(&mut self, out: &mut Vec<Out>) {
        loop {
            let now = self.clock;
            let mut due: Vec<(Millis, u64)> = self
                .queue
                .iter()
                .filter(|s| s.due <= now)
                .map(|s| (s.due, s.seq))
                .collect();
            if due.is_empty() {
                return;
            }
            due.sort_unstable();
            let (_, seq) = due[0];
            let Some(pos) = self.queue.iter().position(|s| s.seq == seq) else {
                return;
            };
            let s = self.queue.remove(pos);
            match s.item {
                Item::Inbound { side, text } => {
                    // It was parsed on arrival.
                    if let Ok(value) = serde_json::from_str::<Value>(&text) {
                        self.process(now, side, &value, out);
                    }
                }
                Item::Emit { to, text } => self.emit(now, to, text, out),
                Item::Release { to, text } => out.push(Out { to, text }),
            }
        }
    }

    /// A frame to a peer, through the outbound faults.
    fn emit(&mut self, now: Millis, to: Side, text: String, out: &mut Vec<Out>) {
        match self.faults.take(now, Direction::to_side(to), &text) {
            None => out.push(Out { to, text }),
            Some(Fault::Drop) => {}
            Some(Fault::Duplicate) => {
                out.push(Out {
                    to,
                    text: text.clone(),
                });
                out.push(Out { to, text });
            }
            Some(Fault::Delay(ms)) => {
                self.defer(now.saturating_add(ms), Item::Release { to, text });
            }
        }
    }

    /// `_respond/2` of the mock socket (`S:171-255`).
    fn process(&mut self, now: Millis, side: Side, frame: &Value, out: &mut Vec<Out>) {
        let obj = frame.as_object();
        let get = |k: &str| obj.and_then(|o| o.get(k));
        let ty = get("type").and_then(Value::as_str);
        match ty {
            Some("acknowledged") if get("id").is_some() => {
                self.acknowledged.push(id_key(get("id")));
            }
            Some("clientHello") => self.client_hellos.push((side, frame.clone())),
            Some("submitAction")
                if get("id").is_some()
                    && get("capability").and_then(Value::as_str) == Some(FORBIDDEN_CAPABILITY) =>
            {
                let nack = json!({
                    "type": "negativeAcknowledged",
                    "id": get("id"),
                    "code": FORBIDDEN_CODE,
                    "message": "Forbidden",
                });
                self.emit(now, side, nack.to_string(), out);
            }
            Some("submitAction")
                if side == Side::Invoker && get("id").is_some() && get("capability").is_some() =>
            {
                self.on_submit(now, frame, out);
            }
            Some("sendActionResult") if side == Side::Handler && get("id").is_some() => {
                let id = get("id").cloned().unwrap_or(Value::Null);
                let ack = json!({"type": "acknowledged", "id": id});
                self.emit(now, Side::Handler, ack.to_string(), out);
                let key = id_key(Some(&id));
                let copies = self.registrations.iter().filter(|r| **r == key).count();
                let text = frame.to_string();
                for _ in 0..copies {
                    self.emit(now, Side::Invoker, text.clone(), out);
                }
            }
            _ => {
                let error = json!({
                    "type": "error",
                    "code": INVALID_MESSAGE_CODE,
                    "message": format!("invalid action message {frame}"),
                });
                self.emit(now, side, error.to_string(), out);
            }
        }
    }

    /// The invoker clause of `submitAction` (`S:195-218`, `:257-298`).
    fn on_submit(&mut self, now: Millis, request: &Value, out: &mut Vec<Out>) {
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let capability = request.get("capability").cloned().unwrap_or(Value::Null);
        let parameters = request
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::new()));
        self.submitted.push((id.to_string(), parameters));
        match self.take_denial() {
            Some(Denial::UpstreamUnavailable) => {
                let error = json!({
                    "code": 503,
                    "message": UNAVAILABLE_MESSAGE,
                    "upstream": UNAVAILABLE_UPSTREAM,
                });
                let text = typeless(error, &id_field(&id));
                self.emit(now, Side::Invoker, text, out);
            }
            Some(Denial::Ignore) => {}
            Some(Denial::RateLimited { retry_after_ms }) => {
                let error = json!({
                    "code": 429,
                    "message": RATE_LIMIT_MESSAGE,
                    "retryAfterMs": retry_after_ms,
                });
                let text = typeless(error, &id_field(&id));
                self.emit(now, Side::Invoker, text, out);
            }
            None => {
                let drop = self.take_drop_forward();
                let known = capability
                    .as_str()
                    .is_some_and(|c| self.capabilities.contains_key(c));
                if known {
                    self.registrations.push(id.to_string());
                    let ack = json!({"type": "acknowledged", "id": id}).to_string();
                    let forward = request.to_string();
                    self.after_delay(
                        now,
                        Item::Emit {
                            to: Side::Invoker,
                            text: ack,
                        },
                        out,
                    );
                    if !drop {
                        self.after_delay(
                            now,
                            Item::Emit {
                                to: Side::Handler,
                                text: forward,
                            },
                            out,
                        );
                    }
                } else {
                    let name = capability
                        .as_str()
                        .map_or_else(|| capability.to_string(), str::to_owned);
                    let nack = json!({
                        "type": "negativeAcknowledged",
                        "id": id,
                        "code": NOT_FOUND_CODE,
                        "message": format!("capability {name} not found"),
                    });
                    self.emit(now, Side::Invoker, nack.to_string(), out);
                }
            }
        }
    }

    /// What follows the mock's sleep (`S:264-266`): at once when the delay is 0.
    fn after_delay(&mut self, now: Millis, item: Item, out: &mut Vec<Out>) {
        if self.config.ack_delay_ms == 0 {
            if let Item::Emit { to, text } = item {
                self.emit(now, to, text, out);
            }
        } else {
            self.defer(now.saturating_add(self.config.ack_delay_ms), item);
        }
    }
}

fn id_key(id: Option<&Value>) -> String {
    id.map_or_else(|| Value::Null.to_string(), Value::to_string)
}

fn id_field(id: &Value) -> IdField {
    match id {
        Value::Null => IdField::Null,
        Value::String(s) => IdField::Present(s.clone()),
        other => IdField::Present(other.to_string()),
    }
}

/// A typeless frame: `{"error": ERROR}` plus the `id` as given.
fn typeless(error: Value, id: &IdField) -> String {
    let mut m = Map::new();
    m.insert("error".to_owned(), error);
    match id {
        IdField::Present(s) => {
            m.insert("id".to_owned(), Value::from(s.as_str()));
        }
        IdField::Null => {
            m.insert("id".to_owned(), Value::Null);
        }
        IdField::Absent => {}
    }
    Value::Object(m).to_string()
}
