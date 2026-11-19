//! The text-frame adapter over [`InvokeState`]: one invoker's `action-ws`
//! session, sans-IO.
//!
//! [`InvokeSession`] joins [`wire`] and [`invoke_state`](crate::invoke_state), as
//! [`crate::session::Session`] does for the handler. A text frame goes in; each
//! [`Output`] comes out as an [`Effect`]: an encoded text frame to send, a
//! completion, a log marker. There is no socket, no thread, no async runtime and
//! no clock read. The caller passes `now` (milliseconds, non-decreasing) with
//! every call and calls [`InvokeSession::on_tick`] at
//! [`InvokeSession::next_wakeup`].
//!
//! # What each inbound frame does
//!
//! The reference is graph_conn `lib/action_api/invoker.ex:198-287`.
//!
//! | inbound frame | input |
//! |---|---|
//! | `acknowledged` | [`Input::Ack`] |
//! | `negativeAcknowledged` | [`Input::Nack`] |
//! | `sendActionResult` | [`Input::Result`]; a missing or `null` `result` is `None` (HIRO hands over `nil`, `invoker_result_test.exs:42-48`) |
//! | `configChanged` | [`Input::ConfigChanged`] |
//! | in-band 429 | [`Input::RateLimited`] |
//! | in-band 503 | [`Input::Unavailable`], with the frame's own `error` object |
//! | `hello` | none (time advances); the connection layer reads it, not the invoker |
//! | `error`, `submitAction`, `clientHello` | [`Input::Unexpected`] |
//!
//! A frame that does not decode is a [`WireError`] and does nothing else, as for
//! the handler session. A `sendActionResult` whose `result` is missing is the one
//! decode error this adapter repairs, because HIRO accepts it.
//!
//! ```
//! use ogar_action_handler::invoke_session::{Effect, InvokeSession};
//! use ogar_action_handler::invoke_state::{CallRequest, InvokeError};
//!
//! let mut s = InvokeSession::new();
//! let fx = s.call(0, CallRequest::new(1, "ticket", "handler", "ExecuteCommand"));
//! let Effect::Send { text, .. } = &fx[0] else { panic!() };
//! let id = serde_json::from_str::<serde_json::Value>(text).unwrap()["id"]
//!     .as_str()
//!     .unwrap()
//!     .to_owned();
//! let ack = format!(r#"{{"type":"acknowledged","id":"{id}"}}"#);
//! assert!(s.on_text(10, &ack).unwrap().is_empty());
//! let result = format!(r#"{{"type":"sendActionResult","id":"{id}","result":"{{\"ok\":true}}"}}"#);
//! let fx = s.on_text(20, &result).unwrap();
//! assert!(matches!(fx[0], Effect::Send { .. })); // the result is acked first
//! let Effect::Complete(done) = &fx[1] else { panic!() };
//! assert_eq!(done.result, Ok(serde_json::json!({"ok": true})));
//! # let _ = InvokeError::AckTimeout(0);
//! ```

use serde_json::{Map, Value, json};

use crate::invoke_state::{
    CallId, CallRequest, Completion, Frame, Input, InvokeState, Log, Output, RESULT_ACK_CODE,
    SendFailure, SubmitFrame,
};
use crate::submit_state::Millis;
use crate::wire::{self, AckFrame, ActionWsFrame, WireError};

/// What the caller must do after a call to the [`InvokeSession`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Effect {
    /// Send this JSON text frame; report a failure with
    /// [`InvokeSession::on_send_failed`] when `call` is `Some`.
    Send {
        /// The call the frame belongs to; `None` for a result ack.
        call: Option<CallId>,
        /// The frame.
        text: String,
    },
    /// A call ended.
    Complete(Completion),
    /// HIRO logs this and does nothing else.
    Log(Log),
    /// `configChanged` arrived.
    ConfigChanged,
}

/// One invoker session: an [`InvokeState`] behind a text-frame interface.
#[derive(Debug, Default)]
pub struct InvokeSession {
    state: InvokeState,
}

impl InvokeSession {
    /// A session with no calls.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A caller starts a request.
    pub fn call(&mut self, now: Millis, request: CallRequest) -> Vec<Effect> {
        self.step(now, Input::Call(request))
    }

    /// Apply one inbound text frame at `now`.
    ///
    /// # Errors
    /// The [`WireError`] of a frame that does not decode. Nothing else happens
    /// for it, and the state machine is not stepped.
    pub fn on_text(&mut self, now: Millis, text: &str) -> Result<Vec<Effect>, WireError> {
        let input = match wire::decode(text) {
            Ok(frame) => input_of(frame, text),
            Err(WireError::MissingField {
                frame: "sendActionResult",
                field: "result",
            }) => Input::Result {
                id: peek_id(text).ok_or(WireError::MissingField {
                    frame: "sendActionResult",
                    field: "id",
                })?,
                result: None,
            },
            Err(e) => return Err(e),
        };
        Ok(self.step(now, input))
    }

    /// The send of `call` failed.
    pub fn on_send_failed(
        &mut self,
        now: Millis,
        call: CallId,
        failure: SendFailure,
    ) -> Vec<Effect> {
        self.step(now, Input::SendFailed { call, failure })
    }

    /// Time passed: fires the waits that are due.
    pub fn on_tick(&mut self, now: Millis) -> Vec<Effect> {
        self.step(now, Input::Tick)
    }

    /// The earliest time at which [`InvokeSession::on_tick`] would do something.
    #[must_use]
    pub fn next_wakeup(&self) -> Option<Millis> {
        self.state.next_wakeup()
    }

    /// The state machine, for its counts.
    #[must_use]
    pub fn state(&self) -> &InvokeState {
        &self.state
    }

    /// The state machine, to set capability defaults.
    pub fn state_mut(&mut self) -> &mut InvokeState {
        &mut self.state
    }

    fn step(&mut self, now: Millis, input: Input) -> Vec<Effect> {
        self.state
            .step(now, input)
            .into_iter()
            .map(effect_of)
            .collect()
    }
}

fn input_of(frame: ActionWsFrame, text: &str) -> Input {
    match frame {
        ActionWsFrame::Acknowledged(AckFrame { id, .. }) => Input::Ack { id },
        ActionWsFrame::NegativeAcknowledged(n) => Input::Nack {
            id: n.id,
            code: n.code,
            message: n.message,
        },
        ActionWsFrame::SendActionResult(r) => Input::Result {
            id: r.id,
            result: Some(r.result),
        },
        ActionWsFrame::ConfigChanged => Input::ConfigChanged,
        ActionWsFrame::RateLimited(r) => Input::RateLimited(r),
        ActionWsFrame::UpstreamUnavailable(u) => {
            // The frame's own error object, whatever fields it carries (`I:267`).
            let error = raw_error(text).unwrap_or_else(|| {
                let mut m = Map::new();
                m.insert("code".to_owned(), Value::from(503));
                if let Some(msg) = &u.message {
                    m.insert("message".to_owned(), Value::from(msg.as_str()));
                }
                if let Some(up) = &u.upstream {
                    m.insert("upstream".to_owned(), Value::from(up.as_str()));
                }
                m
            });
            Input::Unavailable { id: u.id, error }
        }
        ActionWsFrame::Hello(_) => Input::Tick,
        ActionWsFrame::Error(_) => Input::Unexpected {
            kind: "error".to_owned(),
        },
        ActionWsFrame::SubmitAction(_) => Input::Unexpected {
            kind: "submitAction".to_owned(),
        },
        ActionWsFrame::ClientHello(_) => Input::Unexpected {
            kind: "clientHello".to_owned(),
        },
    }
}

fn raw_error(text: &str) -> Option<Map<String, Value>> {
    match serde_json::from_str::<Value>(text).ok()? {
        Value::Object(mut o) => match o.remove("error")? {
            Value::Object(e) => Some(e),
            _ => None,
        },
        _ => None,
    }
}

fn peek_id(text: &str) -> Option<String> {
    match serde_json::from_str::<Value>(text).ok()? {
        Value::Object(o) => o.get("id")?.as_str().map(str::to_owned),
        _ => None,
    }
}

fn effect_of(out: Output) -> Effect {
    match out {
        Output::Send { call, frame } => Effect::Send {
            call,
            text: encode_frame(&frame),
        },
        Output::Complete(c) => Effect::Complete(c),
        Output::Log(l) => Effect::Log(l),
        Output::ConfigChanged => Effect::ConfigChanged,
    }
}

/// The JSON text of a frame the invoker sends.
///
/// `submitAction` is written with typed parameters, as HIRO's struct encoder
/// does (`parameters.timeout` is a number): the handler-side [`wire::encode`]
/// writes every parameter as a string and is not used for it.
#[must_use]
pub fn encode_frame(frame: &Frame) -> String {
    match frame {
        Frame::Submit(SubmitFrame {
            id,
            handler,
            capability,
            parameters,
            timeout_ms,
        }) => json!({
            "type": "submitAction",
            "id": id,
            "handler": handler,
            "capability": capability,
            "parameters": Value::Object(parameters.clone()),
            "timeout": timeout_ms,
        })
        .to_string(),
        Frame::AckResult { id } => wire::encode(&ActionWsFrame::Acknowledged(AckFrame {
            id: id.clone(),
            code: Some(RESULT_ACK_CODE),
            message: None,
        })),
    }
}
