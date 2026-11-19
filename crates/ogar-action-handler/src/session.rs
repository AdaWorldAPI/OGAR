//! The text-frame adapter over [`HandlerState`]: one `action-ws` session, sans-IO.
//!
//! [`Session`] joins [`wire`] and [`submit_state`]. A text frame goes in; each
//! [`Output`] of the state machine comes out as an [`Effect`]: an encoded text
//! frame to send, or a request to run a capability. There is no socket, no
//! thread, no async runtime and no clock read here. The caller passes `now`
//! (milliseconds, non-decreasing) with every call, runs the [`Effect::Run`]
//! requests itself, reports the end with [`Session::on_finished`], and calls
//! [`Session::on_tick`] at [`Session::next_wakeup`].
//!
//! # What each inbound frame does
//!
//! The reference is graph_conn `lib/action_api/handler.ex`, which handles three
//! frames and answers every other frame with a warning only (`handler.ex:152-159`).
//!
//! | inbound frame | effect |
//! |---|---|
//! | `submitAction` | [`Input::Submit`]: ack, then run once per request id |
//! | `acknowledged` | [`Input::Acked`]: stops the resend of that id's result |
//! | `negativeAcknowledged` | [`Input::Nacked`]: stops the resend as well |
//! | `hello` | remembers its `max_frame_bytes`; a result frame over it is sent as the status 59 error (`handler.ex:255-264`) |
//! | any other decoded frame | ignored (time still advances) |
//!
//! The frame limit is the one the latest `hello` advertised. A `hello` that
//! advertised none, and a session that has seen no `hello`, use
//! [`wire::DEFAULT_MAX_FRAME_BYTES`], 1,000,000 bytes, as graph_conn does
//! (`connection_manager.ex:201-221`). A caller that wants no fitting at all can
//! call `session.set_max_frame_bytes(Some(u64::MAX))`.
//!
//! # A failed execution
//!
//! [`run_capability`] and [`Session::on_text_sync`] turn an executor that returns
//! `Err(message)` and one that panics into the `action_status` 54 result of
//! graph_conn's `crash.ex`; see [`crate::crash`] for the exact text and the
//! mapping. A caller that runs capabilities itself and reports with
//! [`Session::on_finished`] builds the same result with [`ExecResult::crashed`].
//!
//! `submitAction.timeout` is milliseconds and becomes the execution deadline;
//! absent selects [`Config::default_timeout_ms`]. A negative `timeout` cannot be a
//! duration; it is read as a deadline of 0 ms (the execution has already
//! timed out). graph_conn does not say what it does with one, so this is a choice.
//!
//! # Frames that do not decode
//!
//! [`Session::on_text`] returns the [`WireError`] and does nothing else: no
//! execution, no frame out, and the state machine is not stepped, so the clock
//! does not advance. The Elixir handler sends no reply to a frame it does not
//! understand (`handler.ex:152-159`); the Python client replies
//! `negativeAcknowledged` 400 with the id (`actionwebsocket.py:658-661`). The
//! reference is Elixir, so no NACK is invented here. A caller that wants the
//! Python behaviour can build one from the error and the id it parsed itself.
//!
//! ```
//! use ogar_action_handler::session::{Effect, Session};
//! use ogar_action_handler::submit_state::{Config, ExecResult};
//!
//! let mut s = Session::new(Config::default()).unwrap();
//! let frame = r#"{"type":"submitAction","id":"id-1","handler":"h","capability":"ExecuteCommand","parameters":{},"timeout":30000}"#;
//! let fx = s.on_text(0, frame).unwrap();
//! assert!(matches!(fx[0], Effect::Send(_)));
//! assert!(matches!(fx[1], Effect::Run { .. }));
//! // The same frame again: acked, not run again.
//! let fx = s.on_text(5, frame).unwrap();
//! assert_eq!(fx.len(), 1);
//! let fx = s.on_finished(9, "id-1", ExecResult::Success("{}".into()));
//! assert!(matches!(fx[0], Effect::Send(_)));
//! ```

use std::panic::{AssertUnwindSafe, catch_unwind};

use ogar_from_schema::action_ws::{
    CapabilityExecutor, NegativeAcknowledged, SendActionResult, SubmitAction, bind_parameters,
    json_object,
};
use ogar_from_schema::do_arm::ActionParam;
use ogar_vocab::ActionDef;

use crate::crash::Crash;
use crate::submit_state::{
    Config, ConfigError, Episode, ExecResult, HandlerState, Input, Millis, Output, Redacted, Submit,
};
use crate::wire::{self, AckFrame, ActionWsFrame, WireError};

/// What the caller must do after a call to the [`Session`].
///
/// `Debug` prints the length of a frame, not its text, and the names of the
/// parameters, not their values: a frame carries results, and a request carries
/// parameters (oracle 42). The frame text itself, as [`Effect::Send`] holds it, is
/// not altered.
#[derive(Clone, PartialEq, Eq)]
pub enum Effect {
    /// Send this JSON text frame on the socket.
    Send(String),
    /// Run the capability, then report the end with [`Session::on_finished`].
    Run {
        /// The request id, as the peer sent it.
        id: String,
        /// The capability to run.
        capability: String,
        /// The submit's parameters, in the order [`wire::decode`] produced them.
        parameters: Vec<(String, String)>,
    },
}

impl core::fmt::Debug for Effect {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Send(text) => f.debug_tuple("Send").field(&Redacted(text.len())).finish(),
            Self::Run {
                id,
                capability,
                parameters,
            } => f
                .debug_struct("Run")
                .field("id", id)
                .field("capability", capability)
                .field("parameters", &crate::submit_state::ParamNames(parameters))
                .finish(),
        }
    }
}

/// One `action-ws` session: a [`HandlerState`] behind a text-frame interface.
#[derive(Debug, Default)]
pub struct Session {
    state: HandlerState,
}

impl Session {
    /// A session with the given timing.
    ///
    /// # Errors
    /// [`ConfigError::NonPositiveResendInterval`] if the resend interval is 0.
    pub fn new(config: Config) -> Result<Self, ConfigError> {
        Ok(Self {
            state: HandlerState::with_config(config)?,
        })
    }

    /// Apply one inbound text frame at `now`.
    ///
    /// # Errors
    /// The [`WireError`] of a frame that does not decode. Nothing else happens
    /// for it; see the module documentation.
    pub fn on_text(&mut self, now: Millis, text: &str) -> Result<Vec<Effect>, WireError> {
        let input = match wire::decode(text)? {
            ActionWsFrame::SubmitAction(s) => Input::Submit(submit_of(s)),
            ActionWsFrame::Acknowledged(AckFrame { id, .. }) => Input::Acked { id },
            ActionWsFrame::NegativeAcknowledged(n) => Input::Nacked { id: n.id },
            ActionWsFrame::Hello(h) => {
                self.state.set_max_frame_bytes(h.max_frame_bytes);
                Input::Tick
            }
            _ => Input::Tick,
        };
        Ok(self.step(now, input))
    }

    /// The execution started for `id` ended. Ignored if `id` is not executing.
    pub fn on_finished(&mut self, now: Millis, id: &str, result: ExecResult) -> Vec<Effect> {
        self.step(
            now,
            Input::Finished {
                id: id.to_owned(),
                result,
            },
        )
    }

    /// Time passed: fires the deadlines and resends that are due.
    pub fn on_tick(&mut self, now: Millis) -> Vec<Effect> {
        self.step(now, Input::Tick)
    }

    /// The earliest time at which [`Session::on_tick`] would do something.
    #[must_use]
    pub fn next_wakeup(&self) -> Option<Millis> {
        self.state.next_wakeup()
    }

    /// Sets the frame limit by hand, as a `hello` does: `None` is the default of
    /// [`wire::DEFAULT_MAX_FRAME_BYTES`]. See [`HandlerState::set_max_frame_bytes`].
    pub fn set_max_frame_bytes(&mut self, max_frame_bytes: Option<u64>) {
        self.state.set_max_frame_bytes(max_frame_bytes);
    }

    /// The state machine, for its episodes and counts.
    #[must_use]
    pub fn state(&self) -> &HandlerState {
        &self.state
    }

    /// Remove and return the finished episodes, oldest first.
    pub fn drain_finished(&mut self) -> Vec<Episode> {
        self.state.drain_finished()
    }

    /// Apply one inbound text frame and run every [`Effect::Run`] it causes
    /// through `executor` at once, at the same `now`. Returns the frames to send,
    /// in order. This is the drop-in for callers of
    /// [`ogar_from_schema::action_ws::handle_submit`] that want a repeated request
    /// id to run once.
    ///
    /// The execution path is that of `handle_submit`: the parameters are bound
    /// against `signature`, the executor runs, and a bind error is the one-field
    /// `{"error":"<message>"}` result. An executor that returns `Err` or panics is
    /// the `action_status` 54 result of [`run_capability`]; the panic is caught and
    /// does not unwind through the session. Two further differences, because the
    /// state machine owns the request lifecycle: a capability other than
    /// `def.predicate` is the result `{"error":{"code":404,"message":"Unhandled
    /// capability \"X\""}}` (`handler.ex:369-371`) and not a NACK, and the id is
    /// not length-checked, since graph_conn treats it as opaque.
    ///
    /// # Errors
    /// The [`WireError`] of a frame that does not decode.
    pub fn on_text_sync(
        &mut self,
        now: Millis,
        text: &str,
        def: &ActionDef,
        signature: &[ActionParam],
        executor: &dyn CapabilityExecutor,
    ) -> Result<Vec<String>, WireError> {
        let mut queue: std::collections::VecDeque<Effect> = self.on_text(now, text)?.into();
        let mut frames = Vec::new();
        while let Some(effect) = queue.pop_front() {
            match effect {
                Effect::Send(f) => frames.push(f),
                Effect::Run {
                    id,
                    capability,
                    parameters,
                } => {
                    let result = run_capability(&capability, &parameters, def, signature, executor);
                    queue.extend(self.on_finished(now, &id, result));
                }
            }
        }
        Ok(frames)
    }

    fn step(&mut self, now: Millis, input: Input) -> Vec<Effect> {
        self.state
            .step(now, input)
            .into_iter()
            .map(effect_of)
            .collect()
    }
}

/// Run `capability` the way `handle_submit` does, and report it as an
/// [`ExecResult`] for the state machine.
///
/// * A capability other than `def.predicate`: the 404 result of `handler.ex:369-371`.
/// * Parameters that do not bind: the one-field `{"error":"<message>"}` result of
///   `handle_submit`. This is not an execution failure; nothing ran.
/// * An executor that returns `Err(message)`: the `action_status` 54 result
///   `Execution failed: ** (RuntimeError) <message>` ([`Crash::RuntimeError`]).
///   This differs from `handle_submit`, which reports it as `{"error":"<message>"}`.
/// * An executor that **panics**: the `action_status` 54 result
///   `Execution failed: ** (exit)` ([`Crash::panicked`]). The panic is caught here
///   and does not unwind through the session. See [`crate::crash`] for why the
///   message is dropped.
///
/// The panic is caught with [`std::panic::catch_unwind`] around
/// [`AssertUnwindSafe`]. A `&dyn CapabilityExecutor` is not `UnwindSafe`, since an
/// executor may hold interior mutability. Asserting it is sound here because the
/// closure holds only shared borrows of the executor and of the inputs, and
/// nothing of the session: after a panic the closure is dropped and the only
/// thing used again is the executor, which the caller owns. If an executor is left
/// inconsistent by its own panic, that is the executor's to guard (a poisoned lock,
/// say), as it would be for any caller of it. A build with `panic = "abort"` cannot
/// catch a panic. This crate installs no panic hook, so the default hook still
/// writes the panic message to stderr: a host that must keep it out of its logs sets
/// its own hook.
#[must_use]
pub fn run_capability(
    capability: &str,
    parameters: &[(String, String)],
    def: &ActionDef,
    signature: &[ActionParam],
    executor: &dyn CapabilityExecutor,
) -> ExecResult {
    let error_field = |message: &str| ExecResult::Error {
        code: None,
        payload: json_object(&[("error".to_owned(), message.to_owned())]),
    };
    if capability != def.predicate {
        let message = format!("Unhandled capability \"{capability}\"");
        let quoted = serde_json::Value::String(message).to_string();
        return ExecResult::Error {
            code: Some(404),
            payload: format!(r#"{{"error":{{"code":404,"message":{quoted}}}}}"#),
        };
    }
    match bind_parameters(parameters, signature) {
        Err(e) => error_field(&e.to_string()),
        Ok(bound) => {
            match catch_unwind(AssertUnwindSafe(|| executor.execute(capability, &bound))) {
                Ok(Ok(params)) => ExecResult::Success(json_object(&params)),
                Ok(Err(message)) => ExecResult::crashed(&Crash::RuntimeError(message)),
                Err(_panic_payload) => ExecResult::crashed(&Crash::panicked()),
            }
        }
    }
}

fn submit_of(s: SubmitAction) -> Submit {
    // A negative duration is a deadline that has already passed.
    let timeout_ms = s.timeout_millis.map(|t| u64::try_from(t).unwrap_or(0));
    Submit::new(s.id, s.handler, s.capability, s.parameters, timeout_ms)
}

fn effect_of(out: Output) -> Effect {
    match out {
        Output::SendAck { id, code, message } => {
            Effect::Send(wire::encode(&ActionWsFrame::from(AckFrame {
                id,
                code: Some(code),
                message: Some(message),
            })))
        }
        Output::SendNack { id, code, message } => Effect::Send(wire::encode(
            &NegativeAcknowledged { id, code, message }.into(),
        )),
        Output::StartExecution {
            id,
            capability,
            parameters,
        } => Effect::Run {
            id,
            capability,
            parameters,
        },
        Output::SendResult { id, payload, .. } => Effect::Send(wire::encode(
            &SendActionResult {
                id,
                result: payload,
            }
            .into(),
        )),
    }
}
