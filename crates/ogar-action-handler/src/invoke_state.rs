//! The HIRO **invoker** (the party that submits an action and waits for its
//! result) as a pure, sans-IO state machine.
//!
//! The reference is graph_conn's `lib/action_api/invoker.ex` (`I` below),
//! `request.ex` (`R`) and `invoker/request_registry.ex` (`G`). The oracle
//! (`HIRO_ORACLE.md`, sections 4 to 7) states the assertions; the source wins
//! where the two differ (see the notes at the end of this page).
//!
//! [`InvokeState::step`] takes the current time and one [`Input`] and returns the
//! [`Output`]s the caller must carry out: a frame to send, a completion for a
//! caller, or a marker for something HIRO only logs. It reads no clock, opens no
//! socket, starts no thread and logs nothing. The caller owns the socket and the
//! timer, and calls [`InvokeState::step`] again at [`InvokeState::next_wakeup`].
//! [`crate::invoke_session`] adapts text frames to [`Input`] and [`Output`].
//!
//! One [`InvokeState`] is one invoker handling many concurrent requests. Each
//! request is a *call* with a caller-chosen [`CallId`], which plays the part of
//! the calling process in Elixir: outputs for it carry that handle.
//!
//! # The life of a call
//!
//! | step | rule | source |
//! |---|---|---|
//! | validate | the `timeout` is checked before anything is sent | `I:409-428`, `:521-563` |
//! | send | the `submitAction` goes out; the ack wait starts | `I:608-612`, `:648-669` |
//! | ack wait | [`DEFAULT_ACK_TIMEOUT_MS`] (3000 ms), or the `ack_timeout` option | `I:109`, `:442`, `:666` |
//! | resend | on expiry the same request is sent again, [`SEND_ATTEMPTS`] sends in all | `I:110`, `:582-590`, `:665-668` |
//! | give up | after the third expiry: [`InvokeError::AckTimeout`] of `ack_timeout * 3` | `I:587` |
//! | response wait | after an ack: the request timeout plus [`RESPONSE_GRACE_MS`] | `I:737-738` |
//! | last call | the first response timeout sends the request once more, with a full ack cycle | `I:704-714` |
//! | give up | the second response timeout: [`InvokeError::ExecTimeout`] | `I:704-708` |
//!
//! A refusal (an in-band 429 or 503 with the request's id), a NACK, a send
//! failure or a result ends the call at once. HIRO never resends into a refusal
//! (`I:246-268`) and a NACK is terminal whatever its code (`I:659-664`).
//!
//! # Selective receive, and the one window that matters
//!
//! In Elixir the registry sends the caller messages (`ack`, `nack`, refusals,
//! the response); the caller process takes the ones its current wait matches and
//! leaves the rest in its mailbox, and drops the stale ones before each send
//! (`_flush_stale`, `I:486-510`). The machine keeps exactly what can matter:
//!
//! * a **response** that arrives early (before the ack, or while a send is being
//!   retried) is kept and answers the call at its response wait (`I:473`);
//! * a **refusal** that arrives after an ack wait has expired and before the next
//!   send is kept. The next send's flush answers the call with it instead of
//!   sending again, but only from the second send on; before the first send of a
//!   round (the first send, and the first send of a last call) a stale refusal is
//!   dropped (`I:512-517`, assertion 41);
//! * everything else (acks, NACKs, refusals during a response wait) would be
//!   flushed or never matched, so it is dropped on arrival. This keeps a call's
//!   state constant in size.
//!
//! Inside [`InvokeState::step`] the order is: **timers that are due at `now`
//! fire first** (an expired ack wait leaves the call between sends, the window),
//! then the input is applied, then the pending sends happen. So an input stamped
//! at or after a wait's deadline arrives *late* for that wait; an input stamped
//! before it is on time. Time that jumps past several deadlines fires each wait
//! once, in deadline order; the next deadline counts from `now`, as in
//! [`crate::submit_state`].
//!
//! # Request ids are not byte-equal to HIRO's
//!
//! HIRO's id is `MD5(:erlang.term_to_binary({ticket, handler, capability,
//! params}))` in base 32 (`R:51-58`). That encoding is Erlang's and is not
//! reproduced here. [`request_id`] is FNV-1a 64 (offset basis
//! `0xcbf29ce484222325`, prime `0x100000001b3`) over a canonical,
//! length-prefixed encoding in which the object keys of `params` are sorted, so
//! the id does not depend on key order. It is 16 lowercase hex digits.
//! That is acceptable because the id is opaque to the handler (the handler's
//! dedupe key is the wire `id`, never recomputed, `handler.ex:167-168`) and no
//! peer parses it. What the oracle asks of the id is kept: deterministic, and
//! different when any of the four inputs differ (assertion 37). A 64-bit hash
//! can collide; the chance for two distinct requests is about 2^-64.
//!
//! # Choices where the source is silent or cannot be copied
//!
//! * An integer timeout whose value in ms does not fit 64 bits, or a string of
//!   digits that does not fit an `i64`, is refused as an invalid timeout. Elixir
//!   has big integers; the wait could not be armed anyway.
//! * An explicit `null` timeout is "the caller gave one" (`I:539-543` matches the
//!   key), so it overrides a capability default and is then no timeout.
//! * With no timeout at all the `parameters` still carry `"timeout": null`,
//!   because `Map.put(params, "timeout", nil)` runs on that path (`I:421`).
//! * A call handle that is already live is refused with a
//!   [`Log::DuplicateCallHandle`] marker; nothing else happens for it.
//! * When several calls share an id, an input reaches them in the order they
//!   were registered. Elixir's `Registry.lookup` order is unspecified.
//! * Capability defaults are given by [`InvokeState::set_capability_defaults`];
//!   the machine does not gate on a "ready" status (HIRO answers `%{}` for
//!   defaults while not ready, which an empty default set also gives).
//! * `Log` markers never carry parameters (assertion 42): ids, codes and counts
//!   only.
//!
//! The oracle's line numbers for `graph_rest_calls.ex` are 15 to 20 lines off
//! the checked-out source (`:115-130` is `:115-125`); `invoker.ex` matches.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::{Map, Value};

use crate::submit_state::Millis;
use crate::wire::RateLimited;

/// Wait for an `acknowledged` after each send (`I:109`).
pub const DEFAULT_ACK_TIMEOUT_MS: Millis = 3_000;
/// Sends before the ack-timeout error (`I:110`).
pub const SEND_ATTEMPTS: u32 = 3;
/// Added to the request timeout for the response wait (`I:737-738`).
pub const RESPONSE_GRACE_MS: Millis = 1_000;
/// Request timeout when neither the caller, the capability nor the options give
/// one (`R:30`, `R:42`).
pub const DEFAULT_REQUEST_TIMEOUT_MS: Millis = 60_000;
/// `code` of the ack the invoker sends for a received result (`I:217`).
pub const RESULT_ACK_CODE: u16 = 200;

/// The handle of one call: chosen by the caller, unique among live calls.
pub type CallId = u64;

/// The caller's `timeout` parameter, as loosely typed as Elixir lets it be.
///
/// Only a whole number of seconds, at least one, as an integer or a string of
/// one, is valid (`I:519-537`). The other variants exist so the refused inputs
/// are representable.
#[derive(Debug, Clone, PartialEq)]
pub enum TimeoutArg {
    /// No `timeout` key at all.
    Absent,
    /// The key is present with `null`.
    Null,
    /// An integer (seconds).
    Int(i64),
    /// A number written with a decimal point or too large for `i64`.
    Float(f64),
    /// A string.
    Str(String),
    /// A boolean.
    Bool(bool),
    /// An array.
    List(Vec<Value>),
    /// An object.
    Object(Map<String, Value>),
}

impl TimeoutArg {
    /// Read a JSON value the way the caller would have supplied it.
    #[must_use]
    pub fn from_value(v: &Value) -> Self {
        match v {
            Value::Null => Self::Null,
            Value::Bool(b) => Self::Bool(*b),
            Value::Number(n) => match n.as_i64() {
                Some(i) => Self::Int(i),
                None => Self::Float(n.as_f64().unwrap_or(f64::NAN)),
            },
            Value::String(s) => Self::Str(s.clone()),
            Value::Array(a) => Self::List(a.clone()),
            Value::Object(o) => Self::Object(o.clone()),
        }
    }

    /// The JSON value this stands for; `None` for [`TimeoutArg::Absent`].
    #[must_use]
    pub fn to_value(&self) -> Option<Value> {
        Some(match self {
            Self::Absent => return None,
            Self::Null => Value::Null,
            Self::Int(i) => Value::from(*i),
            Self::Float(f) => serde_json::Number::from_f64(*f).map_or(Value::Null, Value::Number),
            Self::Str(s) => Value::String(s.clone()),
            Self::Bool(b) => Value::Bool(*b),
            Self::List(a) => Value::Array(a.clone()),
            Self::Object(o) => Value::Object(o.clone()),
        })
    }

    /// The timeout in milliseconds: `Ok(None)` when there is none, `Err(())`
    /// when the value is refused (`I:521-537`). Seconds times 1000.
    ///
    /// # Errors
    /// `()` for anything but absent, `null`, an integer of at least 1 or a
    /// string of one.
    #[allow(clippy::result_unit_err)]
    pub fn millis(&self) -> Result<Option<Millis>, ()> {
        match self {
            Self::Absent | Self::Null => Ok(None),
            Self::Int(s) => seconds_to_millis(*s).map(Some).ok_or(()),
            Self::Str(s) => parse_integer(s)
                .and_then(seconds_to_millis)
                .map(Some)
                .ok_or(()),
            _ => Err(()),
        }
    }
}

fn seconds_to_millis(seconds: i64) -> Option<Millis> {
    u64::try_from(seconds)
        .ok()
        .filter(|s| *s >= 1)?
        .checked_mul(1_000)
}

/// Elixir's `Integer.parse/1` accepting only a whole string: an optional sign
/// and ASCII digits, nothing else (`I:527-534`). No whitespace is skipped.
fn parse_integer(s: &str) -> Option<i64> {
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.strip_prefix('+').unwrap_or(s).parse().ok()
}

/// What the caller asks for: the arguments of `Invoker.execute/5`.
#[derive(Debug, Clone, PartialEq)]
pub struct CallRequest {
    /// The caller's handle for this call.
    pub call: CallId,
    /// The ticket id.
    pub ticket: String,
    /// The action handler id the request is routed to.
    pub handler: String,
    /// The capability name.
    pub capability: String,
    /// The caller's parameters, without `timeout` (see [`CallRequest::timeout`]).
    pub params: Map<String, Value>,
    /// The caller's `timeout` parameter, in SECONDS. [`TimeoutArg::Absent`]
    /// selects the capability default.
    pub timeout: TimeoutArg,
    /// The `ack_timeout` option; `None` is [`DEFAULT_ACK_TIMEOUT_MS`] (`I:442`).
    pub ack_timeout_ms: Option<Millis>,
    /// The `timeout` option in ms, used when no `timeout` parameter is in effect
    /// (`I:451`); then [`DEFAULT_REQUEST_TIMEOUT_MS`] (`R:42`).
    pub default_timeout_ms: Option<Millis>,
}

impl CallRequest {
    /// A call with no parameters, no timeout and no options.
    #[must_use]
    pub fn new(
        call: CallId,
        ticket: impl Into<String>,
        handler: impl Into<String>,
        capability: impl Into<String>,
    ) -> Self {
        Self {
            call,
            ticket: ticket.into(),
            handler: handler.into(),
            capability: capability.into(),
            params: Map::new(),
            timeout: TimeoutArg::Absent,
            ack_timeout_ms: None,
            default_timeout_ms: None,
        }
    }

    /// A call from the caller's whole parameter map: a `timeout` key is taken
    /// out of it into [`CallRequest::timeout`].
    #[must_use]
    pub fn from_params(
        call: CallId,
        ticket: impl Into<String>,
        handler: impl Into<String>,
        capability: impl Into<String>,
        mut params: Map<String, Value>,
    ) -> Self {
        let timeout = params
            .remove("timeout")
            .map_or(TimeoutArg::Absent, |v| TimeoutArg::from_value(&v));
        Self {
            params,
            timeout,
            ..Self::new(call, ticket, handler, capability)
        }
    }

    /// Set one parameter.
    #[must_use]
    pub fn param(mut self, key: &str, value: Value) -> Self {
        self.params.insert(key.to_owned(), value);
        self
    }

    /// Set the caller's `timeout` parameter (seconds).
    #[must_use]
    pub fn with_timeout(mut self, timeout: TimeoutArg) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the `ack_timeout` option.
    #[must_use]
    pub fn with_ack_timeout(mut self, ms: Millis) -> Self {
        self.ack_timeout_ms = Some(ms);
        self
    }

    /// Set the `timeout` option (ms).
    #[must_use]
    pub fn with_default_timeout(mut self, ms: Millis) -> Self {
        self.default_timeout_ms = Some(ms);
        self
    }
}

/// Why a send could not be made (`I:621-634`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendFailure {
    /// The send-time limiter refused: `{:rate_limited, ms}` (`I:621-625`).
    RateLimited(u64),
    /// Any other transport reason, as text (`I:631-634`).
    Other(String),
}

/// What the caller feeds the machine.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Input {
    /// A caller starts a request.
    Call(CallRequest),
    /// `acknowledged` for `id` (`I:198-199`).
    Ack {
        /// The request id.
        id: String,
    },
    /// `negativeAcknowledged` for `id` (`I:201-212`). HIRO passes `msg["code"]`
    /// and `msg["message"]` on as they are, `nil` when the frame had none.
    Nack {
        /// The request id.
        id: String,
        /// The code, when the frame carried one.
        code: Option<u16>,
        /// The message, when the frame carried one.
        message: Option<String>,
    },
    /// `sendActionResult` (`I:214-237`). `result` is the wire string, decoded
    /// once; `None` is a missing or `null` result.
    Result {
        /// The request id.
        id: String,
        /// The result text.
        result: Option<String>,
    },
    /// The typeless in-band 429 (`I:246-258`, `:272-273`).
    RateLimited(RateLimited),
    /// The typeless in-band 503 (`I:260-268`, `:275-278`).
    Unavailable {
        /// The refused request's id; `None` when absent, null or not a string.
        id: Option<String>,
        /// The frame's own `error` object.
        error: Map<String, Value>,
    },
    /// `configChanged` (`I:239-240`).
    ConfigChanged,
    /// A frame the invoker has no clause for (`I:280-287`); carries its kind.
    Unexpected {
        /// The frame's `type`, or a short name for a typeless one.
        kind: String,
    },
    /// The send of `call` could not be made.
    SendFailed {
        /// The call whose [`Output::Send`] failed.
        call: CallId,
        /// Why.
        failure: SendFailure,
    },
    /// Time passed; nothing else happened.
    Tick,
}

/// The `submitAction` the invoker sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitFrame {
    /// The request id.
    pub id: String,
    /// The handler id.
    pub handler: String,
    /// The capability.
    pub capability: String,
    /// The parameters as typed JSON values; `timeout` is in ms or `null`.
    pub parameters: Map<String, Value>,
    /// The wire `timeout`, in milliseconds.
    pub timeout_ms: Millis,
}

/// A frame the machine asks the caller to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// `submitAction` (`R:23-30`).
    Submit(SubmitFrame),
    /// `{"type":"acknowledged","id":ID,"code":200}` for a received result
    /// (`I:217`).
    AckResult {
        /// The id of the result being acknowledged.
        id: String,
    },
}

/// Why a call ended without a result. These are HIRO's error terms (oracle
/// section 5b).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvokeError {
    /// `{:not_sent, reason}` (`I:545-563`, `:631-634`).
    NotSent(NotSent),
    /// `{:rate_limited, ms}`, in `0..=86_400_000` (`I:621-625`, `:681-684`).
    RateLimited(u64),
    /// `{:ack_timeout, ack_timeout * 3}` (`I:587`).
    AckTimeout(u64),
    /// `{:exec_timeout, request_timeout_ms}` (`I:707`).
    ExecTimeout(u64),
    /// `{:handler_returned_timeout, request_timeout_ms}`: the result was
    /// `{"error":"exec_timeout"}` (`I:724-725`).
    HandlerReturnedTimeout(u64),
    /// `{:action_api_returned_timeout, last_status}`: the result was
    /// `{"error":"request_timed_out","last_status":S}` (`I:727-728`).
    ActionApiReturnedTimeout(Value),
    /// `{:nack, %{code, message}}` (`I:659-664`); either is `nil` in HIRO when
    /// the frame had none (`I:210`).
    Nack {
        /// The NACK code, when the frame carried one.
        code: Option<u16>,
        /// The NACK message, when the frame carried one.
        message: Option<String>,
    },
    /// A 503's `error` object, as the frame carried it (`I:260-268`, `:676-679`).
    Refused(Map<String, Value>),
    /// The `error` value of a result object (`I:730-731`).
    Handler(Value),
}

/// The `{:not_sent, reason}` reasons the machine produces.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NotSent {
    /// `{:invalid_timeout, value}`: the value as given (`I:562`).
    InvalidTimeout(Value),
    /// Any transport reason, as text (`I:631-634`).
    Transport(String),
}

impl fmt::Display for InvokeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSent(NotSent::InvalidTimeout(v)) => {
                write!(f, "not sent: invalid timeout {v}")
            }
            Self::NotSent(NotSent::Transport(r)) => write!(f, "not sent: {r}"),
            Self::RateLimited(ms) => write!(f, "rate limited, retry in {ms}ms"),
            Self::AckTimeout(ms) => write!(f, "no acknowledgement within {ms}ms"),
            Self::ExecTimeout(ms) => write!(f, "no response within {ms}ms"),
            Self::HandlerReturnedTimeout(ms) => write!(f, "handler timed out after {ms}ms"),
            Self::ActionApiReturnedTimeout(s) => write!(f, "action api timed out, last status {s}"),
            Self::Nack { code, message } => {
                write!(f, "nacked")?;
                if let Some(c) = code {
                    write!(f, " {c}")?;
                }
                if let Some(m) = message {
                    write!(f, ": {m}")?;
                }
                Ok(())
            }
            Self::Refused(e) => write!(f, "refused: {}", Value::Object(e.clone())),
            Self::Handler(e) => write!(f, "handler error: {e}"),
        }
    }
}

impl std::error::Error for InvokeError {}

/// How a call ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    /// The caller's handle.
    pub call: CallId,
    /// The request id (HIRO returns it with every error).
    pub request_id: String,
    /// The decoded result, or why there is none.
    pub result: Result<Value, InvokeError>,
}

/// Whose `timeout` was refused (`I:539-543`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutSource {
    /// The caller gave it.
    Caller,
    /// It came from the capability defaults.
    CapabilityDefaults,
}

/// Something HIRO only logs. No marker carries parameters or a result.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Log {
    /// A request was refused for its timeout and not sent (warning, `I:558-560`).
    InvalidTimeoutRefused {
        /// The id computed before the timeout was converted (`I:545-553`).
        request_id: String,
        /// Whose value it was.
        source: TimeoutSource,
    },
    /// An ack wait ran out (warning, `I:667`).
    AckTimeout {
        /// The request id.
        request_id: String,
        /// The wait that ran out.
        ack_timeout_ms: Millis,
    },
    /// A response wait ran out and the request is sent once more (warning,
    /// `I:711`).
    SendingLastCall {
        /// The request id.
        request_id: String,
    },
    /// The last call's response wait ran out too (error, `I:705`).
    ResponseTimeout {
        /// The request id.
        request_id: String,
    },
    /// A NACK with a code other than 404 (error, `I:662-664`; a 404 is not
    /// logged, `I:659-660`). A NACK with no code is not 404, so it is logged.
    MessageNacked {
        /// The request id.
        request_id: String,
        /// The code, when the frame carried one.
        code: Option<u16>,
    },
    /// A refusal found no waiting caller (warning, `I:290-302`; assertion 40).
    RefusalNoCaller {
        /// The refused request's id.
        request_id: String,
        /// `429` or `503`.
        code: u16,
    },
    /// An in-band 429 with no usable id (warning, `I:272-273`; assertion 26).
    RateLimitedNoId {
        /// The sanitised advertised wait.
        retry_after_ms: u64,
    },
    /// An in-band 503 with no usable id (warning, `I:275-278`; assertion 26).
    UpstreamUnavailableNoId,
    /// A frame with no clause (error, `I:280-287`); carries its kind.
    UnexpectedFrame {
        /// The frame's kind.
        kind: String,
    },
    /// A [`CallRequest::call`] handle that is already live was refused.
    DuplicateCallHandle {
        /// The handle.
        call: CallId,
    },
}

/// What the machine asks the caller to do.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Output {
    /// Send this frame on the socket; report a failure with
    /// [`Input::SendFailed`].
    Send {
        /// The call it belongs to; `None` for a result ack, which has no caller.
        call: Option<CallId>,
        /// The frame.
        frame: Frame,
    },
    /// A call ended.
    Complete(Completion),
    /// HIRO logs this and does nothing else.
    Log(Log),
    /// `configChanged` arrived. HIRO calls `on_config_changed/0`, which only
    /// warns unless overridden (`I:239-240`, `:896-902`).
    ConfigChanged,
}

#[derive(Debug, Clone)]
enum Refusal {
    RateLimited(u64),
    Unavailable(Map<String, Value>),
}

impl Refusal {
    fn into_error(self) -> InvokeError {
        match self {
            Self::RateLimited(ms) => InvokeError::RateLimited(ms),
            Self::Unavailable(e) => InvokeError::Refused(e),
        }
    }

    fn code(&self) -> u16 {
        match self {
            Self::RateLimited(_) => 429,
            Self::Unavailable(_) => 503,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    /// Between sends: the next send is due in this step. `attempt` counts from 1;
    /// 4 is "give up".
    Retry {
        attempt: u32,
        last_call: bool,
    },
    AwaitAck {
        attempt: u32,
        last_call: bool,
        deadline: Millis,
    },
    AwaitResponse {
        last_call: bool,
        deadline: Millis,
    },
}

#[derive(Debug, Clone)]
struct Call {
    id: String,
    handler: String,
    capability: String,
    parameters: Map<String, Value>,
    timeout_ms: Millis,
    ack_timeout_ms: Millis,
    phase: Phase,
    /// The first response that arrived before it was wanted.
    response: Option<Value>,
    /// The first refusal that arrived between two sends.
    refusal: Option<Refusal>,
}

/// One invoker: many concurrent calls, each answered by the frames that carry
/// its request id.
#[derive(Debug, Default)]
pub struct InvokeState {
    clock: Millis,
    calls: BTreeMap<CallId, Call>,
    by_id: BTreeMap<String, Vec<CallId>>,
    defaults: BTreeMap<String, Map<String, Value>>,
}

impl InvokeState {
    /// An invoker with no calls and no capability defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the parameter defaults of one capability: field to default value, as
    /// HIRO's `capability_defaults/1` returns them (`I:343-360`). An empty map
    /// is the same as unknown.
    pub fn set_capability_defaults(
        &mut self,
        capability: impl Into<String>,
        defaults: Map<String, Value>,
    ) {
        self.defaults.insert(capability.into(), defaults);
    }

    /// Take the parameter defaults from a capabilities body (the JSON of
    /// `GET /capabilities`), as the invoker reads them after a fetch
    /// (assertion 32): see [`crate::discovery::capability_defaults`]. Replaces
    /// the defaults of every capability.
    pub fn load_capabilities(&mut self, capabilities: &Value) {
        self.defaults = crate::discovery::all_capability_defaults(capabilities);
    }

    /// Replace the defaults of every capability.
    pub fn replace_capability_defaults(&mut self, all: BTreeMap<String, Map<String, Value>>) {
        self.defaults = all;
    }

    /// The defaults of one capability.
    #[must_use]
    pub fn capability_defaults(&self, capability: &str) -> Option<&Map<String, Value>> {
        self.defaults.get(capability)
    }

    /// Calls in flight. A finished call leaves no entry.
    #[must_use]
    pub fn tracked(&self) -> usize {
        self.calls.len()
    }

    /// Distinct request ids in the registry. Always at most [`Self::tracked`].
    #[must_use]
    pub fn tracked_ids(&self) -> usize {
        self.by_id.len()
    }

    /// The earliest time at which [`InvokeState::step`] with [`Input::Tick`]
    /// would do something.
    #[must_use]
    pub fn next_wakeup(&self) -> Option<Millis> {
        self.calls
            .values()
            .filter_map(|c| match c.phase {
                Phase::AwaitAck { deadline, .. } | Phase::AwaitResponse { deadline, .. } => {
                    Some(deadline)
                }
                Phase::Retry { .. } => None,
            })
            .min()
    }

    /// Apply one input at `now` (milliseconds, non-decreasing; an earlier value
    /// is treated as the latest one seen) and return what the caller must do,
    /// in order.
    pub fn step(&mut self, now: Millis, input: Input) -> Vec<Output> {
        let now = now.max(self.clock);
        self.clock = now;
        let mut out = Vec::new();
        self.expire(now, &mut out);
        self.apply(now, input, &mut out);
        loop {
            self.settle(now, &mut out);
            if !self.expire(now, &mut out) {
                break;
            }
        }
        out
    }

    // ---------------------------------------------------------- timers --

    /// Fire every wait due at `now`, in deadline order. True if any fired.
    fn expire(&mut self, now: Millis, out: &mut Vec<Output>) -> bool {
        let mut due: Vec<(Millis, CallId)> = self
            .calls
            .iter()
            .filter_map(|(id, c)| match c.phase {
                Phase::AwaitAck { deadline, .. } | Phase::AwaitResponse { deadline, .. }
                    if deadline <= now =>
                {
                    Some((deadline, *id))
                }
                _ => None,
            })
            .collect();
        due.sort_unstable();
        let any = !due.is_empty();
        for (_, cid) in due {
            let Some(call) = self.calls.get_mut(&cid) else {
                continue;
            };
            match call.phase {
                Phase::AwaitAck {
                    attempt, last_call, ..
                } => {
                    out.push(Output::Log(Log::AckTimeout {
                        request_id: call.id.clone(),
                        ack_timeout_ms: call.ack_timeout_ms,
                    }));
                    call.phase = Phase::Retry {
                        attempt: attempt + 1,
                        last_call,
                    };
                }
                Phase::AwaitResponse {
                    last_call: true, ..
                } => {
                    out.push(Output::Log(Log::ResponseTimeout {
                        request_id: call.id.clone(),
                    }));
                    let timeout = call.timeout_ms;
                    self.finish(cid, Err(InvokeError::ExecTimeout(timeout)), out);
                }
                Phase::AwaitResponse {
                    last_call: false, ..
                } => {
                    out.push(Output::Log(Log::SendingLastCall {
                        request_id: call.id.clone(),
                    }));
                    call.phase = Phase::Retry {
                        attempt: 1,
                        last_call: true,
                    };
                }
                Phase::Retry { .. } => {}
            }
        }
        any
    }

    /// Make every pending send, in call order: flush stale answers, then send
    /// (`_execute/4`, `I:582-606`).
    fn settle(&mut self, now: Millis, out: &mut Vec<Output>) {
        let pending: Vec<CallId> = self
            .calls
            .iter()
            .filter(|(_, c)| matches!(c.phase, Phase::Retry { .. }))
            .map(|(id, _)| *id)
            .collect();
        for cid in pending {
            let Some(call) = self.calls.get_mut(&cid) else {
                continue;
            };
            let Phase::Retry { attempt, last_call } = call.phase else {
                continue;
            };
            // Before a first send of a round a stale refusal is dropped; later
            // sends, and the give-up, answer with it (`I:512-517`).
            let stale = call.refusal.take();
            if attempt > 1
                && let Some(refusal) = stale
            {
                self.finish(cid, Err(refusal.into_error()), out);
                continue;
            }
            if attempt > SEND_ATTEMPTS {
                let wait = call.ack_timeout_ms.saturating_mul(u64::from(SEND_ATTEMPTS));
                self.finish(cid, Err(InvokeError::AckTimeout(wait)), out);
                continue;
            }
            out.push(Output::Send {
                call: Some(cid),
                frame: Frame::Submit(SubmitFrame {
                    id: call.id.clone(),
                    handler: call.handler.clone(),
                    capability: call.capability.clone(),
                    parameters: call.parameters.clone(),
                    timeout_ms: call.timeout_ms,
                }),
            });
            call.phase = Phase::AwaitAck {
                attempt,
                last_call,
                deadline: now.saturating_add(call.ack_timeout_ms),
            };
        }
    }

    // ---------------------------------------------------------- inputs --

    fn apply(&mut self, now: Millis, input: Input, out: &mut Vec<Output>) {
        match input {
            Input::Call(req) => self.start(req, out),
            Input::Ack { id } => {
                for cid in self.registered(&id) {
                    self.on_ack(now, cid, out);
                }
            }
            Input::Nack { id, code, message } => {
                for cid in self.registered(&id) {
                    if matches!(
                        self.calls.get(&cid).map(|c| c.phase),
                        Some(Phase::AwaitAck { .. })
                    ) {
                        // `%{code: 404}` is not logged; any other code is.
                        if code != Some(404) {
                            out.push(Output::Log(Log::MessageNacked {
                                request_id: id.clone(),
                                code,
                            }));
                        }
                        self.finish(
                            cid,
                            Err(InvokeError::Nack {
                                code,
                                message: message.clone(),
                            }),
                            out,
                        );
                    }
                }
            }
            Input::Result { id, result } => {
                // Acked first, whether or not anybody waits for it (`I:217-236`).
                out.push(Output::Send {
                    call: None,
                    frame: Frame::AckResult { id: id.clone() },
                });
                let decoded = result.as_deref().map_or(Value::Null, decode_result);
                for cid in self.registered(&id) {
                    self.on_response(cid, decoded.clone(), out);
                }
            }
            Input::RateLimited(r) => {
                let ms = r.retry_after_ms();
                match r.id {
                    None => out.push(Output::Log(Log::RateLimitedNoId { retry_after_ms: ms })),
                    Some(id) => self.refuse(&id, &Refusal::RateLimited(ms), out),
                }
            }
            Input::Unavailable { id, error } => match id {
                None => out.push(Output::Log(Log::UpstreamUnavailableNoId)),
                Some(id) => self.refuse(&id, &Refusal::Unavailable(error), out),
            },
            Input::ConfigChanged => out.push(Output::ConfigChanged),
            Input::Unexpected { kind } => {
                out.push(Output::Log(Log::UnexpectedFrame { kind }));
            }
            Input::SendFailed { call, failure } => {
                if matches!(
                    self.calls.get(&call).map(|c| c.phase),
                    Some(Phase::AwaitAck { .. })
                ) {
                    let err = match failure {
                        SendFailure::RateLimited(ms) => InvokeError::RateLimited(ms),
                        SendFailure::Other(reason) => {
                            InvokeError::NotSent(NotSent::Transport(reason))
                        }
                    };
                    self.finish(call, Err(err), out);
                }
            }
            Input::Tick => {}
        }
    }

    fn registered(&self, id: &str) -> Vec<CallId> {
        self.by_id.get(id).cloned().unwrap_or_default()
    }

    fn on_ack(&mut self, now: Millis, cid: CallId, out: &mut Vec<Output>) {
        let Some(call) = self.calls.get_mut(&cid) else {
            return;
        };
        if let Phase::AwaitAck { last_call, .. } = call.phase {
            call.phase = Phase::AwaitResponse {
                last_call,
                deadline: now
                    .saturating_add(call.timeout_ms)
                    .saturating_add(RESPONSE_GRACE_MS),
            };
            // A response that came first answers at once (`I:473`).
            if let Some(r) = call.response.take() {
                let timeout = call.timeout_ms;
                self.finish(cid, classify_response(r, timeout), out);
            }
        }
        // Any other phase: a stale ack, flushed in Elixir.
    }

    fn on_response(&mut self, cid: CallId, value: Value, out: &mut Vec<Output>) {
        let Some(call) = self.calls.get_mut(&cid) else {
            return;
        };
        if matches!(call.phase, Phase::AwaitResponse { .. }) {
            let timeout = call.timeout_ms;
            self.finish(cid, classify_response(value, timeout), out);
        } else if call.response.is_none() {
            call.response = Some(value);
        }
    }

    fn refuse(&mut self, id: &str, refusal: &Refusal, out: &mut Vec<Output>) {
        let callers = self.registered(id);
        if callers.is_empty() {
            out.push(Output::Log(Log::RefusalNoCaller {
                request_id: id.to_owned(),
                code: refusal.code(),
            }));
            return;
        }
        for cid in callers {
            let Some(call) = self.calls.get_mut(&cid) else {
                continue;
            };
            match call.phase {
                Phase::AwaitAck { .. } => {
                    self.finish(cid, Err(refusal.clone().into_error()), out);
                }
                Phase::Retry { .. } => {
                    if call.refusal.is_none() {
                        call.refusal = Some(refusal.clone());
                    }
                }
                // Never matched by a response wait, flushed before the next send.
                Phase::AwaitResponse { .. } => {}
            }
        }
    }

    fn start(&mut self, req: CallRequest, out: &mut Vec<Output>) {
        if self.calls.contains_key(&req.call) {
            out.push(Output::Log(Log::DuplicateCallHandle { call: req.call }));
            return;
        }
        let prepared = self.prepare(&req);
        match prepared {
            Prepared::Refused {
                request_id,
                source,
                value,
            } => {
                out.push(Output::Log(Log::InvalidTimeoutRefused {
                    request_id: request_id.clone(),
                    source,
                }));
                out.push(Output::Complete(Completion {
                    call: req.call,
                    request_id,
                    result: Err(InvokeError::NotSent(NotSent::InvalidTimeout(value))),
                }));
            }
            Prepared::Ready {
                request_id,
                parameters,
                timeout_ms,
            } => {
                self.by_id
                    .entry(request_id.clone())
                    .or_default()
                    .push(req.call);
                self.calls.insert(
                    req.call,
                    Call {
                        id: request_id,
                        handler: req.handler,
                        capability: req.capability,
                        parameters,
                        timeout_ms,
                        ack_timeout_ms: req.ack_timeout_ms.unwrap_or(DEFAULT_ACK_TIMEOUT_MS),
                        phase: Phase::Retry {
                            attempt: 1,
                            last_call: false,
                        },
                        response: None,
                        refusal: None,
                    },
                );
            }
        }
    }

    /// `execute/5` up to the send (`I:409-428`, `:565-569`).
    fn prepare(&self, req: &CallRequest) -> Prepared {
        let mut params = self
            .defaults
            .get(&req.capability)
            .cloned()
            .unwrap_or_default();
        // The caller's parameters win over the defaults (`I:565-569`).
        for (k, v) in &req.params {
            params.insert(k.clone(), v.clone());
        }
        params.remove("timeout");
        let (arg, source) = if req.timeout == TimeoutArg::Absent {
            match self
                .defaults
                .get(&req.capability)
                .and_then(|d| d.get("timeout"))
            {
                Some(v) => (TimeoutArg::from_value(v), TimeoutSource::CapabilityDefaults),
                None => (TimeoutArg::Absent, TimeoutSource::CapabilityDefaults),
            }
        } else {
            (req.timeout.clone(), TimeoutSource::Caller)
        };
        let raw = arg.to_value();
        if let Some(v) = &raw {
            params.insert("timeout".to_owned(), v.clone());
        }
        match arg.millis() {
            Err(()) => Prepared::Refused {
                // The id is taken before the timeout is converted (`I:545-553`).
                request_id: request_id(&req.ticket, &req.handler, &req.capability, &params),
                source,
                value: raw.unwrap_or(Value::Null),
            },
            Ok(ms) => {
                // `Map.put(params, "timeout", timeout)` runs with nil too (`I:421`).
                params.insert("timeout".to_owned(), ms.map_or(Value::Null, Value::from));
                Prepared::Ready {
                    request_id: request_id(&req.ticket, &req.handler, &req.capability, &params),
                    parameters: params,
                    timeout_ms: ms
                        .or(req.default_timeout_ms)
                        .unwrap_or(DEFAULT_REQUEST_TIMEOUT_MS),
                }
            }
        }
    }

    /// End a call: no entry is left behind.
    fn finish(&mut self, cid: CallId, result: Result<Value, InvokeError>, out: &mut Vec<Output>) {
        let Some(call) = self.calls.remove(&cid) else {
            return;
        };
        if let Some(ids) = self.by_id.get_mut(&call.id) {
            ids.retain(|c| *c != cid);
            if ids.is_empty() {
                self.by_id.remove(&call.id);
            }
        }
        out.push(Output::Complete(Completion {
            call: cid,
            request_id: call.id,
            result,
        }));
    }
}

enum Prepared {
    Refused {
        request_id: String,
        source: TimeoutSource,
        value: Value,
    },
    Ready {
        request_id: String,
        parameters: Map<String, Value>,
        timeout_ms: Millis,
    },
}

/// Decode a result string once (`I:742-754`): JSON if it parses, the string as
/// it is if not.
#[must_use]
pub fn decode_result(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_owned()))
}

/// Turn a decoded result into the call's outcome (`I:720-735`; assertion 36).
///
/// # Errors
/// An object with an `error` key is an error: `"exec_timeout"` is
/// [`InvokeError::HandlerReturnedTimeout`], `"request_timed_out"` with a
/// `last_status` is [`InvokeError::ActionApiReturnedTimeout`], any other value
/// is [`InvokeError::Handler`].
pub fn classify_response(value: Value, request_timeout_ms: Millis) -> Result<Value, InvokeError> {
    let Value::Object(obj) = &value else {
        return Ok(value);
    };
    match obj.get("error") {
        None => Ok(value),
        Some(Value::String(e)) if e == "exec_timeout" => {
            Err(InvokeError::HandlerReturnedTimeout(request_timeout_ms))
        }
        Some(Value::String(e)) if e == "request_timed_out" && obj.contains_key("last_status") => {
            Err(InvokeError::ActionApiReturnedTimeout(
                obj["last_status"].clone(),
            ))
        }
        Some(e) => Err(InvokeError::Handler(e.clone())),
    }
}

// ------------------------------------------------------------ request id --

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

struct Fnv(u64);

impl Fnv {
    fn byte(&mut self, b: u8) {
        self.0 ^= u64::from(b);
        self.0 = self.0.wrapping_mul(FNV_PRIME);
    }

    fn bytes(&mut self, bs: &[u8]) {
        for b in bs {
            self.byte(*b);
        }
    }

    fn len(&mut self, n: usize) {
        self.bytes(&(n as u64).to_le_bytes());
    }

    fn string(&mut self, s: &str) {
        self.byte(b's');
        self.len(s.len());
        self.bytes(s.as_bytes());
    }

    fn value(&mut self, v: &Value) {
        match v {
            Value::Null => self.byte(b'n'),
            Value::Bool(b) => self.byte(if *b { b't' } else { b'f' }),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    self.byte(b'i');
                    self.bytes(&i.to_le_bytes());
                } else if let Some(u) = n.as_u64() {
                    self.byte(b'u');
                    self.bytes(&u.to_le_bytes());
                } else {
                    self.byte(b'd');
                    self.bytes(&n.as_f64().unwrap_or(f64::NAN).to_bits().to_le_bytes());
                }
            }
            Value::String(s) => self.string(s),
            Value::Array(a) => {
                self.byte(b'[');
                self.len(a.len());
                for x in a {
                    self.value(x);
                }
            }
            Value::Object(o) => self.object(o),
        }
    }

    fn object(&mut self, o: &Map<String, Value>) {
        self.byte(b'{');
        self.len(o.len());
        let mut keys: Vec<&String> = o.keys().collect();
        keys.sort_unstable();
        for k in keys {
            self.string(k);
            self.value(&o[k]);
        }
    }
}

/// The request id: a deterministic function of the ticket, the handler, the
/// capability and the parameters (assertion 37). See the module documentation
/// for how it differs from HIRO's.
///
/// `params` is the parameter map as it is sent, `timeout` (in ms) included.
#[must_use]
pub fn request_id(
    ticket: &str,
    handler: &str,
    capability: &str,
    params: &Map<String, Value>,
) -> String {
    let mut h = Fnv(FNV_OFFSET);
    h.string(ticket);
    h.string(handler);
    h.string(capability);
    h.object(params);
    format!("{:016x}", h.0)
}
