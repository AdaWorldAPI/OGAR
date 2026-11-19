//! The stateful HIRO ActionHandler submit path, as a pure (sans-IO) state
//! machine, plus the action episode it records.
//!
//! [`HandlerState::step`] takes the current time and one [`Input`], and returns
//! the [`Output`]s the caller must carry out. It reads no clock, starts no
//! thread, and performs no I/O: the caller owns the socket, the executor and
//! the timer, and calls [`HandlerState::step`] again at
//! [`HandlerState::next_wakeup`]. It takes no wire frames: the
//! [`Session`](crate::session::Session) adapts them to [`Input`] and [`Output`].
//! It uses the wire encoder in one place only, to measure a result frame against
//! the peer's frame limit.
//!
//! The reference is the Elixir handler, `graph_conn`
//! `lib/action_api/handler.ex` and `lib/action_api/responder.ex`. The oracle
//! section 3 describes the states; each rule below names its source line.
//!
//! * **Ack on claim.** Every submit is acknowledged (`code` 200, `message`
//!   `""`) before anything else is done for it (`handler.ex:168-172`).
//! * **One claim per request id.** The id is the dedupe key and is opaque: it is
//!   never parsed or recomputed (`handler.ex:167-168`). A repeat while the first
//!   is executing joins the waiters and starts nothing (`handler.ex:201-203`);
//!   a repeat after completion is answered from the stored result
//!   (`handler.ex:186-188`).
//! * **Result cache, 60 minutes.** The entry is stored when the claim is made
//!   and stored again, which restarts the 60 minutes, when the result is set
//!   (`handler.ex:72-74`, `:210-217`). Reads do not extend it. So the
//!   60 minutes count from completion.
//! * **Resend every 3000 ms until acked or nacked, with no give-up**
//!   (`responder.ex:143-163`, `handler.ex:365-366`). A nack stops the resend
//!   exactly like an ack (`handler.ex:112-119`).
//! * **Execution deadline.** The `timeout` of the submit in milliseconds, else
//!   60000 (`handler.ex:133`, `:361-362`). On expiry the result is the error
//!   `{"error":{"req_id":ID,"action_status":13,"action_error":"Execution timed
//!   out after <N>ms"}}` (`handler.ex:311-313`, `:335-339`).
//! * **A crashed execution is status 54** (`handler.ex:343-358`,
//!   `crash.ex`): [`ExecResult::crashed`], built from a [`Crash`](crate::crash::Crash).
//!   It is cached like any result (oracle 43).
//! * **A result over the frame limit becomes status 59** before it is cached
//!   (`handler.ex:178`, `:255-264`, `result_frame.ex:27-38`). The limit is set
//!   with [`HandlerState::set_max_frame_bytes`]; unset, it is 1,000,000 bytes.
//!   Every result is fitted, a timeout and a failure included, as in Elixir.
//! * **`Debug` redacts.** No type here prints a parameter value or a result text:
//!   names, lengths and counts only (oracle 42).
//! * **The handler sends no NACK for a submit.** Every failure is a result
//!   (oracle section 5c). [`Output::SendNack`] exists for the adapter's own
//!   wire-level refusals; this machine never emits it.
//!
//! # Episodes
//!
//! Each request id has one [`Episode`], the intervention: a retry is another
//! attempt of it, never a new episode. An episode is open while its id is
//! tracked, and moves to the finished list once it is no longer needed for
//! dedupe or resends. [`HandlerState::drain_finished`] hands those records to
//! the caller; nothing is persisted here.
//!
//! ```
//! use ogar_action_handler::submit_state::{
//!     ExecResult, HandlerState, Input, Output, Submit,
//! };
//!
//! let mut st = HandlerState::new();
//! let out = st.step(0, Input::Submit(Submit::new("id-1", "h1", "ExecuteCommand", vec![], None)));
//! assert!(matches!(out[0], Output::SendAck { .. }));
//! assert!(matches!(out[1], Output::StartExecution { .. }));
//! // A re-sent submit does not run the executor again.
//! let out = st.step(10, Input::Submit(Submit::new("id-1", "h1", "ExecuteCommand", vec![], None)));
//! assert_eq!(out.len(), 1);
//! let out = st.step(20, Input::Finished { id: "id-1".into(), result: ExecResult::Success("{}".into()) });
//! assert!(matches!(out[0], Output::SendResult { attempt: 1, .. }));
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use ogar_from_schema::action_ws::SendActionResult;
use ogar_vocab::effect_class::EffectClass;

use crate::crash::{CRASH_STATUS, Crash};
use crate::wire::{self, ActionWsFrame, DEFAULT_MAX_FRAME_BYTES, TOO_BIG_STATUS};

/// Milliseconds on the caller's clock. Only differences matter; the caller must
/// pass a non-decreasing value (an earlier value is treated as the latest one
/// seen).
pub type Millis = u64;

/// Resend interval of an unacked result: `resend_response_timeout/0`
/// (`handler.ex:365-366`).
pub const RESEND_INTERVAL_MS: Millis = 3_000;
/// How long a finished result is kept for dedupe: 60 minutes
/// (`handler.ex:72-74`).
pub const RESULT_TTL_MS: Millis = 60 * 60 * 1_000;
/// Execution deadline when the submit carries no `timeout`:
/// `default_execution_timeout/1` (`handler.ex:361-362`).
pub const DEFAULT_EXECUTION_TIMEOUT_MS: Millis = 60_000;
/// `code` of the receipt ack (`handler.ex:170`).
pub const ACK_CODE: u16 = 200;
/// `action_status` of a result whose execution outlived its deadline
/// (`handler.ex:313`).
pub const EXEC_TIMEOUT_STATUS: u32 = 13;
/// How many attempts one [`Episode`] keeps. A result with no give-up is resent
/// for ever, so the list is bounded; older entries are dropped and counted in
/// [`Episode::attempts_dropped`].
pub const MAX_RECORDED_ATTEMPTS: usize = 256;

/// The three timing constants of the handler. [`Config::default`] is the
/// Elixir default of each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// Milliseconds between resends of an unacked result. Must be positive.
    pub resend_interval_ms: Millis,
    /// Milliseconds a finished result is kept for dedupe, from completion.
    pub result_ttl_ms: Millis,
    /// Execution deadline for a submit that carries no `timeout`.
    pub default_timeout_ms: Millis,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            resend_interval_ms: RESEND_INTERVAL_MS,
            result_ttl_ms: RESULT_TTL_MS,
            default_timeout_ms: DEFAULT_EXECUTION_TIMEOUT_MS,
        }
    }
}

/// A [`Config`] the machine refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfigError {
    /// The resend interval was 0. The Elixir responder raises, because a
    /// fixed interval of 0 would resend in a hot loop for ever
    /// (`responder.ex:88-94`).
    NonPositiveResendInterval(Millis),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPositiveResendInterval(v) => write!(
                f,
                "resend_response_timeout/0 must return a positive integer, got: {v}"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

/// One `submitAction`, in the machine's own terms.
///
/// `Debug` prints the names of the parameters and how many there are, never their
/// values (oracle 42: parameters are never logged; `params_log_test.exs`).
#[derive(Clone, PartialEq, Eq)]
pub struct Submit {
    /// The request id: opaque, the dedupe key.
    pub id: String,
    /// The handler id the action is routed to (the Actor, as a string).
    pub handler: String,
    /// The capability requested.
    pub capability: String,
    /// The action parameters, passed through to [`Output::StartExecution`].
    pub parameters: Vec<(String, String)>,
    /// The execution deadline in milliseconds; `None` selects the default.
    /// `Some(0)` is a deadline of 0 ms, as in Elixir, where only `nil` selects
    /// the default (`handler.ex:133`).
    pub timeout_ms: Option<Millis>,
    /// The class id, if the caller knows it. Recorded in the episode only.
    pub class_id: Option<u32>,
    /// The Target node id as given. Recorded in the episode only.
    pub target: Option<String>,
    /// The effect class, if the caller knows it. Recorded in the episode only.
    pub effect_class: Option<EffectClass>,
}

impl fmt::Debug for Submit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Submit")
            .field("id", &self.id)
            .field("handler", &self.handler)
            .field("capability", &self.capability)
            .field("parameters", &ParamNames(&self.parameters))
            .field("timeout_ms", &self.timeout_ms)
            .field("class_id", &self.class_id)
            .field("target", &self.target)
            .field("effect_class", &self.effect_class)
            .finish()
    }
}

/// `Debug` for a parameter list: the names and the count, not the values.
pub(crate) struct ParamNames<'a>(pub(crate) &'a [(String, String)]);

impl fmt::Debug for ParamNames<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} parameters ", self.0.len())?;
        f.debug_list()
            .entries(self.0.iter().map(|(k, _)| k))
            .finish()
    }
}

/// `Debug` for text that must not be printed: only its length.
pub(crate) struct Redacted(pub(crate) usize);

impl fmt::Debug for Redacted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} bytes redacted>", self.0)
    }
}

impl Submit {
    /// A submit with no class id, target or effect class.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        handler: impl Into<String>,
        capability: impl Into<String>,
        parameters: Vec<(String, String)>,
        timeout_ms: Option<Millis>,
    ) -> Self {
        Self {
            id: id.into(),
            handler: handler.into(),
            capability: capability.into(),
            parameters,
            timeout_ms,
            class_id: None,
            target: None,
            effect_class: None,
        }
    }
}

/// How an execution ended, as the caller reports it.
///
/// `Debug` prints the variant, the codes and the lengths of the texts, never the
/// texts: a result or an error text can carry what the action worked on.
#[derive(Clone, PartialEq, Eq)]
pub enum ExecResult {
    /// The execution returned; `0` is the encoded result string, sent as is.
    Success(String),
    /// The execution crashed or was refused, with an `action_status` and an
    /// `action_error`. The machine renders the Elixir error result
    /// `{"error":{"req_id":ID,"action_status":CODE,"action_error":MESSAGE}}`
    /// (`handler.ex:335-339`), the shape of status 54 (crash) and 59 (too big).
    Failure {
        /// The `action_status`.
        code: u32,
        /// The `action_error` text.
        message: String,
    },
    /// The execution returned `{:error, term}`: the caller has encoded it, for
    /// instance `{"error":{"code":404,"message":"Unhandled capability ..."}}`
    /// (`handler.ex:278-285`, `:369-371`). Sent as is. `code` only labels the
    /// episode outcome.
    Error {
        /// The error code, when the executor gave one.
        code: Option<u32>,
        /// The encoded result string.
        payload: String,
    },
}

impl ExecResult {
    /// The result of a crashed execution: `action_status` 54 and the text
    /// `Execution failed: <banner>` (`handler.ex:309`, `:327`, `:356-357`). See
    /// [`crate::crash`] for the banner and for how a Rust executor's `Err` and
    /// panic map to it.
    #[must_use]
    pub fn crashed(crash: &Crash) -> Self {
        Self::Failure {
            code: CRASH_STATUS,
            message: crash.failure_message(),
        }
    }
}

impl fmt::Debug for ExecResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success(s) => f.debug_tuple("Success").field(&Redacted(s.len())).finish(),
            Self::Failure { code, message } => f
                .debug_struct("Failure")
                .field("code", code)
                .field("message", &Redacted(message.len()))
                .finish(),
            Self::Error { code, payload } => f
                .debug_struct("Error")
                .field("code", code)
                .field("payload", &Redacted(payload.len()))
                .finish(),
        }
    }
}

/// What the caller feeds the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// A `submitAction` arrived (first delivery or a repeat).
    Submit(Submit),
    /// The execution started for `id` ended. Ignored if `id` is not executing
    /// (unknown, finished, or already past its deadline).
    Finished {
        /// The request id.
        id: String,
        /// How it ended.
        result: ExecResult,
    },
    /// The peer acknowledged the result of `id`.
    Acked {
        /// The request id.
        id: String,
    },
    /// The peer negatively acknowledged the result of `id`. Like an ack, it
    /// stops the resend (`handler.ex:112-119`).
    Nacked {
        /// The request id.
        id: String,
    },
    /// Time passed; nothing else happened.
    Tick,
}

/// What the machine asks the caller to do.
///
/// `Debug` prints names and lengths, never parameter or result values.
#[derive(Clone, PartialEq, Eq)]
pub enum Output {
    /// Send `acknowledged` for the submit `id` (`code` 200, `message` `""`).
    SendAck {
        /// The request id.
        id: String,
        /// The status code.
        code: u16,
        /// The note.
        message: String,
    },
    /// Send `negativeAcknowledged`. Never emitted by [`HandlerState`]: the
    /// Elixir handler sends no NACK for a submit (oracle section 5c).
    SendNack {
        /// The request id.
        id: String,
        /// The error code.
        code: u16,
        /// The error text.
        message: String,
    },
    /// Run the capability. At most one per request id at a time, and one per
    /// claim.
    StartExecution {
        /// The request id; report the end with [`Input::Finished`].
        id: String,
        /// The capability to run.
        capability: String,
        /// The parameters of the submit.
        parameters: Vec<(String, String)>,
    },
    /// Send `sendActionResult`.
    SendResult {
        /// The request id.
        id: String,
        /// The encoded result string.
        payload: String,
        /// 1 for the first delivery of this result, then 2, 3, and so on.
        attempt: u32,
    },
}

impl fmt::Debug for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SendAck { id, code, message } => f
                .debug_struct("SendAck")
                .field("id", id)
                .field("code", code)
                .field("message", message)
                .finish(),
            Self::SendNack { id, code, message } => f
                .debug_struct("SendNack")
                .field("id", id)
                .field("code", code)
                .field("message", message)
                .finish(),
            Self::StartExecution {
                id,
                capability,
                parameters,
            } => f
                .debug_struct("StartExecution")
                .field("id", id)
                .field("capability", capability)
                .field("parameters", &ParamNames(parameters))
                .finish(),
            Self::SendResult {
                id,
                payload,
                attempt,
            } => f
                .debug_struct("SendResult")
                .field("id", id)
                .field("payload", &Redacted(payload.len()))
                .field("attempt", attempt)
                .finish(),
        }
    }
}

/// What one [`Attempt`] delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptKind {
    /// A delivery of the request: the first submit or a repeat.
    RequestDelivery,
    /// A delivery of the result: the first send or a resend.
    ResultDelivery,
}

/// One delivery within an [`Episode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attempt {
    /// Counts within its kind, from 1. For a result delivery it equals the
    /// `attempt` of the [`Output::SendResult`].
    pub number: u32,
    /// What was delivered.
    pub kind: AttemptKind,
    /// When the machine saw it.
    pub at: Millis,
}

/// How the execution of an episode ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The execution returned a result.
    Succeeded,
    /// The execution failed; the `action_status` or error code when there is
    /// one.
    Failed(Option<u32>),
    /// The execution outlived its deadline (status 13).
    TimedOut,
}

/// The record of one intervention: everything that happened under one request
/// id from its claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Episode {
    /// The intervention id: the request id.
    pub intervention_id: String,
    /// The deliveries, oldest first, at most [`MAX_RECORDED_ATTEMPTS`].
    pub attempts: Vec<Attempt>,
    /// The handler id (the Actor), as a string.
    pub handler_id: String,
    /// The capability.
    pub capability: String,
    /// The class id, if the caller supplied one.
    pub class_id: Option<u32>,
    /// The Target as given.
    pub target: Option<String>,
    /// The effect class, if the caller supplied one.
    pub effect_class: Option<EffectClass>,
    /// When the claim was made.
    pub started_at: Millis,
    /// When the execution ended (or hit its deadline); `None` while it runs.
    pub finished_at: Option<Millis>,
    /// How the execution ended; `None` while it runs.
    pub outcome: Option<Outcome>,
    /// Whether the peer acknowledged the result at least once.
    pub acknowledged: bool,
    /// Whether the peer negatively acknowledged the result at least once.
    pub nacked: bool,
    /// How many request deliveries there were in all, dropped ones included.
    pub request_deliveries: u32,
    /// How many result deliveries there were in all, dropped ones included.
    pub result_deliveries: u32,
}

impl Episode {
    fn new(s: &Submit, at: Millis) -> Self {
        Self {
            intervention_id: s.id.clone(),
            attempts: Vec::new(),
            handler_id: s.handler.clone(),
            capability: s.capability.clone(),
            class_id: s.class_id,
            target: s.target.clone(),
            effect_class: s.effect_class,
            started_at: at,
            finished_at: None,
            outcome: None,
            acknowledged: false,
            nacked: false,
            request_deliveries: 0,
            result_deliveries: 0,
        }
    }

    /// Record a delivery and return its number within its kind.
    fn record(&mut self, kind: AttemptKind, at: Millis) -> u32 {
        let counter = match kind {
            AttemptKind::RequestDelivery => &mut self.request_deliveries,
            AttemptKind::ResultDelivery => &mut self.result_deliveries,
        };
        *counter += 1;
        let number = *counter;
        if self.attempts.len() >= MAX_RECORDED_ATTEMPTS {
            self.attempts.remove(0);
        }
        self.attempts.push(Attempt { number, kind, at });
        number
    }

    /// How many recorded attempts were dropped to keep the list bounded.
    #[must_use]
    pub fn attempts_dropped(&self) -> u64 {
        u64::from(self.request_deliveries) + u64::from(self.result_deliveries)
            - self.attempts.len() as u64
    }
}

enum Phase {
    Executing {
        deadline_at: Millis,
        timeout_ms: Millis,
        waiters: u32,
    },
    Done {
        payload: String,
        completed_at: Millis,
        /// `Some` while the result is unacked; the time of the next resend.
        next_resend: Option<Millis>,
    },
}

impl fmt::Debug for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Executing {
                deadline_at,
                timeout_ms,
                waiters,
            } => f
                .debug_struct("Executing")
                .field("deadline_at", deadline_at)
                .field("timeout_ms", timeout_ms)
                .field("waiters", waiters)
                .finish(),
            Self::Done {
                payload,
                completed_at,
                next_resend,
            } => f
                .debug_struct("Done")
                .field("payload", &Redacted(payload.len()))
                .field("completed_at", completed_at)
                .field("next_resend", next_resend)
                .finish(),
        }
    }
}

#[derive(Debug)]
struct Entry {
    phase: Phase,
    episode: Episode,
    /// The time this entry is registered under in `timers`.
    timer: Millis,
}

enum Completion {
    Result(ExecResult),
    TimedOut(Millis),
}

/// The stateful submit handler. See the module documentation.
#[derive(Debug)]
pub struct HandlerState {
    config: Config,
    clock: Millis,
    entries: BTreeMap<String, Entry>,
    /// One timer per tracked id: the deadline of an executing entry, the next
    /// resend of an unacked one, else the end of the dedupe window.
    timers: BTreeSet<(Millis, String)>,
    finished: Vec<Episode>,
    /// The frame limit a result is fitted to.
    max_frame_bytes: u64,
}

impl Default for HandlerState {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerState {
    /// A handler with the Elixir defaults: 3000 ms resend, 60 minute cache,
    /// 60000 ms execution deadline.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            clock: 0,
            entries: BTreeMap::new(),
            timers: BTreeSet::new(),
            finished: Vec::new(),
            max_frame_bytes: DEFAULT_MAX_FRAME_BYTES,
        }
    }

    /// A handler with the given timing.
    ///
    /// # Errors
    /// [`ConfigError::NonPositiveResendInterval`] if the resend interval is 0.
    pub fn with_config(config: Config) -> Result<Self, ConfigError> {
        if config.resend_interval_ms == 0 {
            return Err(ConfigError::NonPositiveResendInterval(0));
        }
        Ok(Self {
            config,
            ..Self::new()
        })
    }

    /// Sets the frame limit of the peer, from its `hello`. A result whose
    /// `sendActionResult` frame is larger is replaced by the `action_status` 59
    /// error before it is cached or sent (`handler.ex:178`, `:255-264`, see
    /// [`wire::fit`]). `None` is a `hello` that advertised no limit, which is
    /// [`DEFAULT_MAX_FRAME_BYTES`] (`connection_manager.ex:201-221`). A result
    /// already finished keeps the frame it was fitted to.
    pub fn set_max_frame_bytes(&mut self, max_frame_bytes: Option<u64>) {
        self.max_frame_bytes = max_frame_bytes
            .filter(|n| *n > 0)
            .unwrap_or(DEFAULT_MAX_FRAME_BYTES);
    }

    /// The frame limit results are fitted to.
    #[must_use]
    pub fn max_frame_bytes(&self) -> u64 {
        self.max_frame_bytes
    }

    /// Advance to `now`, apply `input`, and return what to do, in order.
    ///
    /// Timers due at or before `now` are handled first, in order of time then
    /// request id, so a `Finished` that arrives at or after its deadline is
    /// late. A resend that is overdue by several intervals is sent once and the
    /// next one is due an interval after `now`, as the Elixir timer re-arms
    /// when it is handled (`responder.ex:150-153`).
    pub fn step(&mut self, now: Millis, input: Input) -> Vec<Output> {
        let now = now.max(self.clock);
        self.clock = now;
        let mut out = Vec::new();
        self.run_timers(now, &mut out);
        match input {
            Input::Submit(s) => self.on_submit(now, s, &mut out),
            Input::Finished { id, result } => {
                if self.is_executing(&id) {
                    self.finish(now, &id, Completion::Result(result), &mut out);
                }
            }
            Input::Acked { id } => self.on_settled(now, &id, true),
            Input::Nacked { id } => self.on_settled(now, &id, false),
            Input::Tick => {}
        }
        // A deadline of 0 ms is already due.
        self.run_timers(now, &mut out);
        out
    }

    /// The earliest time at which [`HandlerState::step`] would do something
    /// without a new input: a deadline, a resend, or the end of a dedupe
    /// window. `None` when nothing is tracked.
    #[must_use]
    pub fn next_wakeup(&self) -> Option<Millis> {
        self.timers.first().map(|(at, _)| *at)
    }

    /// How many request ids are tracked: executing, cached, or awaiting an ack.
    /// This is the state that must stay bounded; finished episodes waiting to
    /// be drained are not counted.
    #[must_use]
    pub fn tracked(&self) -> usize {
        self.entries.len()
    }

    /// The open episode of `id`, if it is tracked.
    #[must_use]
    pub fn episode(&self, id: &str) -> Option<&Episode> {
        self.entries.get(id).map(|e| &e.episode)
    }

    /// The open episodes, in request id order.
    pub fn open_episodes(&self) -> impl Iterator<Item = &Episode> {
        self.entries.values().map(|e| &e.episode)
    }

    /// The finished episodes not yet drained, oldest first. An episode is
    /// finished once its dedupe window has closed and its result is no longer
    /// resent (acked or nacked), or once its id was claimed again after the
    /// window closed.
    pub fn finished_episodes(&self) -> impl Iterator<Item = &Episode> {
        self.finished.iter()
    }

    /// Remove and return the finished episodes, oldest first, for the caller to
    /// persist.
    pub fn drain_finished(&mut self) -> Vec<Episode> {
        std::mem::take(&mut self.finished)
    }

    // ---- internals ----

    fn is_executing(&self, id: &str) -> bool {
        matches!(
            self.entries.get(id),
            Some(Entry {
                phase: Phase::Executing { .. },
                ..
            })
        )
    }

    fn due_of(&self, phase: &Phase) -> Millis {
        match phase {
            Phase::Executing { deadline_at, .. } => *deadline_at,
            Phase::Done {
                next_resend: Some(at),
                ..
            } => *at,
            Phase::Done {
                completed_at,
                next_resend: None,
                ..
            } => completed_at.saturating_add(self.config.result_ttl_ms),
        }
    }

    /// Whether the dedupe window of a finished entry has closed at `now`.
    fn window_closed(&self, phase: &Phase, now: Millis) -> bool {
        match phase {
            Phase::Executing { .. } => false,
            Phase::Done { completed_at, .. } => {
                completed_at.saturating_add(self.config.result_ttl_ms) <= now
            }
        }
    }

    fn retime(&mut self, id: &str) {
        let Some(entry) = self.entries.get(id) else {
            return;
        };
        let new = self.due_of(&entry.phase);
        let old = entry.timer;
        self.timers.remove(&(old, id.to_owned()));
        self.timers.insert((new, id.to_owned()));
        if let Some(entry) = self.entries.get_mut(id) {
            entry.timer = new;
        }
    }

    fn insert(&mut self, id: &str, phase: Phase, episode: Episode) {
        let timer = self.due_of(&phase);
        self.timers.insert((timer, id.to_owned()));
        self.entries.insert(
            id.to_owned(),
            Entry {
                phase,
                episode,
                timer,
            },
        );
    }

    fn evict(&mut self, id: &str) {
        if let Some(entry) = self.entries.remove(id) {
            self.timers.remove(&(entry.timer, id.to_owned()));
            self.finished.push(entry.episode);
        }
    }

    fn run_timers(&mut self, now: Millis, out: &mut Vec<Output>) {
        while let Some((due, id)) = self.timers.first().cloned() {
            if due > now {
                break;
            }
            self.timers.remove(&(due, id.clone()));
            self.fire(now, &id, out);
        }
    }

    /// Handle the timer of `id`, which has already left `timers`. Every case
    /// ends with the entry gone or re-timed after `now`.
    fn fire(&mut self, now: Millis, id: &str, out: &mut Vec<Output>) {
        let interval = self.config.resend_interval_ms;
        let Some(entry) = self.entries.get_mut(id) else {
            return;
        };
        match &mut entry.phase {
            Phase::Executing { timeout_ms, .. } => {
                let timeout_ms = *timeout_ms;
                self.finish(now, id, Completion::TimedOut(timeout_ms), out);
            }
            Phase::Done {
                payload,
                next_resend: Some(next),
                ..
            } => {
                let attempt = entry.episode.record(AttemptKind::ResultDelivery, now);
                out.push(Output::SendResult {
                    id: id.to_owned(),
                    payload: payload.clone(),
                    attempt,
                });
                *next = now.saturating_add(interval);
                self.retime(id);
            }
            Phase::Done {
                next_resend: None, ..
            } => self.evict(id),
        }
    }

    fn on_submit(&mut self, now: Millis, s: Submit, out: &mut Vec<Output>) {
        let id = s.id.clone();
        // A finished entry whose window closed is only still here because its
        // result is unacked. The id is new again: retire the old episode.
        if self
            .entries
            .get(&id)
            .is_some_and(|e| self.window_closed(&e.phase, now))
        {
            self.evict(&id);
        }
        let interval = self.config.resend_interval_ms;
        let ack = Output::SendAck {
            id: id.clone(),
            code: ACK_CODE,
            message: String::new(),
        };
        match self.entries.get_mut(&id) {
            None => {
                let timeout_ms = s.timeout_ms.unwrap_or(self.config.default_timeout_ms);
                let mut episode = Episode::new(&s, now);
                episode.record(AttemptKind::RequestDelivery, now);
                out.push(ack);
                out.push(Output::StartExecution {
                    id: id.clone(),
                    capability: s.capability,
                    parameters: s.parameters,
                });
                let phase = Phase::Executing {
                    deadline_at: now.saturating_add(timeout_ms),
                    timeout_ms,
                    waiters: 0,
                };
                self.insert(&id, phase, episode);
            }
            Some(entry) => {
                entry.episode.record(AttemptKind::RequestDelivery, now);
                out.push(ack);
                match &mut entry.phase {
                    Phase::Executing { waiters, .. } => *waiters += 1,
                    Phase::Done {
                        payload,
                        next_resend,
                        ..
                    } => {
                        let attempt = entry.episode.record(AttemptKind::ResultDelivery, now);
                        out.push(Output::SendResult {
                            id: id.clone(),
                            payload: payload.clone(),
                            attempt,
                        });
                        // The responder keeps a pending registration and its
                        // timer; it registers again only when none is pending
                        // (`responder.ex:110-111`).
                        if next_resend.is_none() {
                            *next_resend = Some(now.saturating_add(interval));
                        }
                        self.retime(&id);
                    }
                }
            }
        }
    }

    fn on_settled(&mut self, now: Millis, id: &str, acked: bool) {
        let Some(entry) = self.entries.get_mut(id) else {
            return;
        };
        let Phase::Done { next_resend, .. } = &mut entry.phase else {
            return;
        };
        // Nothing pending: the Elixir responder pops nothing and does nothing
        // (`responder.ex:129-130`).
        if next_resend.take().is_none() {
            return;
        }
        if acked {
            entry.episode.acknowledged = true;
        } else {
            entry.episode.nacked = true;
        }
        if self.window_closed(&self.entries[id].phase, now) {
            self.evict(id);
        } else {
            self.retime(id);
        }
    }

    fn finish(&mut self, now: Millis, id: &str, completion: Completion, out: &mut Vec<Output>) {
        let interval = self.config.resend_interval_ms;
        let Some(entry) = self.entries.get_mut(id) else {
            return;
        };
        let (payload, outcome) = match completion {
            Completion::Result(ExecResult::Success(p)) => (p, Outcome::Succeeded),
            Completion::Result(ExecResult::Failure { code, message }) => (
                error_result(id, code, &message),
                Outcome::Failed(Some(code)),
            ),
            Completion::Result(ExecResult::Error { code, payload }) => {
                (payload, Outcome::Failed(code))
            }
            Completion::TimedOut(timeout_ms) => (
                error_result(
                    id,
                    EXEC_TIMEOUT_STATUS,
                    &format!("Execution timed out after {timeout_ms}ms"),
                ),
                Outcome::TimedOut,
            ),
        };
        // Fitted before it is cached, so a repeat answered from the cache, and
        // every resend, carries the frame the peer accepts (`handler.ex:253-264`).
        let (payload, outcome) = fit_payload(id, payload, outcome, self.max_frame_bytes);
        entry.episode.finished_at = Some(now);
        entry.episode.outcome = Some(outcome);
        // The waiters, if any, are not sent a result of their own: only the
        // latest waiter (or the executing task) sends it (`handler.ex:219-229`).
        let attempt = entry.episode.record(AttemptKind::ResultDelivery, now);
        out.push(Output::SendResult {
            id: id.to_owned(),
            payload: payload.clone(),
            attempt,
        });
        entry.phase = Phase::Done {
            payload,
            completed_at: now,
            next_resend: Some(now.saturating_add(interval)),
        };
        self.retime(id);
    }
}

/// The result swapped for the status 59 error when its `sendActionResult` frame
/// is over `max_frame_bytes`; the outcome then is a failure with that status.
fn fit_payload(
    id: &str,
    payload: String,
    outcome: Outcome,
    max_frame_bytes: u64,
) -> (String, Outcome) {
    let frame = ActionWsFrame::from(SendActionResult {
        id: id.to_owned(),
        result: payload,
    });
    match wire::fit_checked(frame, max_frame_bytes) {
        (ActionWsFrame::SendActionResult(fitted), replaced) => {
            let outcome = if replaced {
                Outcome::Failed(Some(TOO_BIG_STATUS))
            } else {
                outcome
            };
            (fitted.result, outcome)
        }
        _ => unreachable!("a sendActionResult frame fits to a sendActionResult frame"),
    }
}

/// `{"error":{"req_id":ID,"action_status":CODE,"action_error":MESSAGE}}`
/// (`handler.ex:335-339`). The key order is fixed here, not left to the JSON
/// library's map type.
pub(crate) fn error_result(req_id: &str, action_status: u32, action_error: &str) -> String {
    let quote = |s: &str| serde_json::Value::String(s.to_owned()).to_string();
    format!(
        r#"{{"error":{{"req_id":{},"action_status":{action_status},"action_error":{}}}}}"#,
        quote(req_id),
        quote(action_error)
    )
}
