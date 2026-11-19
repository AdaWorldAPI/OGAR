//! Parameters are never logged: oracle 42.
//!
//! graph_conn pins it with `G/test/action_api/params_log_test.exs:6-80`: no log
//! line from the invoker, handler or socket carries a parameter value, and
//! `G/lib/graph_conn/ws_connection.ex:136-142` logs a `FrameSummary` instead of the
//! frame. This crate does no logging, so the property that can fail here is the one
//! a caller's `{:?}` would break: the `Debug` output of every type that carries a
//! parameter or a result through the session and the state machine names and counts
//! them and never prints a value.
//!
//! The wire text itself is not redacted: it is what is sent. `Effect::Send` holds
//! it, and a test below checks the text is still whole.

use ogar_action_handler::crash::Crash;
use ogar_action_handler::session::{Effect, Session};
use ogar_action_handler::submit_state::{Config, ExecResult, HandlerState, Input, Output, Submit};
use ogar_action_handler::wire::{self, ActionWsFrame};
use ogar_from_schema::action_ws::{
    Acknowledged, HandlerReaction, Receipt, SendActionResult, SubmitAction,
};

/// Unique to this file, and a plausible value to put in a log line.
const SECRET: &str = "s3cret-9f2c41d7-hunter2";

fn assert_redacted(what: &str, value: &dyn std::fmt::Debug) {
    for text in [format!("{value:?}"), format!("{value:#?}")] {
        assert!(!text.contains(SECRET), "{what} printed a value: {text}");
        assert!(!text.contains("hunter2"), "{what} printed a value: {text}");
    }
}

fn submit_with_secret() -> Submit {
    Submit::new(
        "req-1",
        "handler-7",
        "ExecuteCommand",
        vec![
            ("command".to_owned(), "uptime".to_owned()),
            ("password".to_owned(), SECRET.to_owned()),
        ],
        Some(30_000),
    )
}

fn submit_text() -> String {
    format!(
        r#"{{"type":"submitAction","id":"req-1","handler":"h","capability":"ExecuteCommand","parameters":{{"password":"{SECRET}","command":"echo"}}}}"#
    )
}

/// The names and the count are kept: they are what a debugger needs.
#[test]
fn debug_names_the_parameters_and_counts_them() {
    let text = format!("{:?}", submit_with_secret());
    assert!(text.contains("2 parameters"), "{text}");
    assert!(text.contains("password"), "{text}");
    assert!(text.contains("command"), "{text}");
    assert!(text.contains("req-1"), "{text}");
    assert_redacted("Submit", &submit_with_secret());
}

#[test]
fn submit_and_input_and_output_do_not_print_values() {
    let submit = submit_with_secret();
    assert_redacted("Submit", &submit);
    assert_redacted("Input::Submit", &Input::Submit(submit.clone()));

    let mut st = HandlerState::new();
    let out = st.step(0, Input::Submit(submit));
    assert_eq!(out.len(), 2);
    for o in &out {
        assert_redacted("Output", o);
    }
    assert!(matches!(&out[1], Output::StartExecution { .. }));
    let text = format!("{:?}", out[1]);
    assert!(
        text.contains("StartExecution") && text.contains("password"),
        "{text}"
    );

    let results = [
        ExecResult::Success(format!(r#"{{"output":"{SECRET}"}}"#)),
        ExecResult::Failure {
            code: 54,
            message: format!("Execution failed: ** (RuntimeError) {SECRET}"),
        },
        ExecResult::Error {
            code: Some(404),
            payload: format!(r#"{{"error":"{SECRET}"}}"#),
        },
        ExecResult::crashed(&Crash::RuntimeError(SECRET.to_owned())),
    ];
    for r in results {
        assert_redacted("ExecResult", &r);
        assert_redacted(
            "Input::Finished",
            &Input::Finished {
                id: "req-1".to_owned(),
                result: r.clone(),
            },
        );
    }
}

#[test]
fn output_send_result_and_the_state_do_not_print_the_result() {
    let mut st = HandlerState::new();
    st.step(0, Input::Submit(submit_with_secret()));
    let out = st.step(
        1,
        Input::Finished {
            id: "req-1".to_owned(),
            result: ExecResult::Success(format!(r#"{{"output":"{SECRET}"}}"#)),
        },
    );
    assert!(matches!(&out[0], Output::SendResult { .. }));
    for o in &out {
        assert_redacted("Output::SendResult", o);
    }
    // The state holds the cached result and the executing request.
    assert_redacted("HandlerState (done)", &st);

    let mut st = HandlerState::new();
    st.step(0, Input::Submit(submit_with_secret()));
    assert_redacted("HandlerState (executing)", &st);
}

#[test]
fn episodes_do_not_print_values() {
    let mut st = HandlerState::new();
    st.step(0, Input::Submit(submit_with_secret()));
    st.step(
        1,
        Input::Finished {
            id: "req-1".to_owned(),
            result: ExecResult::Success(format!(r#"{{"output":"{SECRET}"}}"#)),
        },
    );
    st.step(
        2,
        Input::Acked {
            id: "req-1".to_owned(),
        },
    );
    st.step(3_700_000, Input::Tick);
    let finished: Vec<_> = st.drain_finished();
    assert_eq!(finished.len(), 1);
    assert_redacted("Episode (finished)", &finished[0]);

    let mut st = HandlerState::new();
    st.step(0, Input::Submit(submit_with_secret()));
    assert_redacted("Episode (open)", st.episode("req-1").unwrap());
}

#[test]
fn effects_and_the_session_do_not_print_values_but_the_wire_text_is_whole() {
    let mut s = Session::new(Config::default()).unwrap();
    let fx = s.on_text(0, &submit_text()).unwrap();
    assert_eq!(fx.len(), 2);
    for e in &fx {
        assert_redacted("Effect", e);
    }
    let Effect::Run { parameters, .. } = &fx[1] else {
        panic!("a run");
    };
    // The values are still passed to the executor.
    assert!(parameters.iter().any(|(_, v)| v == SECRET));

    let fx = s.on_finished(
        1,
        "req-1",
        ExecResult::Success(format!(r#"{{"output":"{SECRET}"}}"#)),
    );
    let Effect::Send(text) = &fx[0] else {
        panic!("a send");
    };
    // Not redacted on the wire.
    assert!(text.contains(SECRET), "the frame is sent whole");
    assert_redacted("Effect::Send", &fx[0]);
    assert_redacted("Session", &s);
}

#[test]
fn wire_frames_do_not_print_parameters_or_results() {
    let ActionWsFrame::SubmitAction(submit) = wire::decode(&submit_text()).unwrap() else {
        panic!("a submit");
    };
    assert_redacted("SubmitAction", &submit);
    let text = format!("{submit:?}");
    assert!(
        text.contains("password") && text.contains("parameter_count"),
        "{text}"
    );
    assert_redacted(
        "ActionWsFrame::SubmitAction",
        &ActionWsFrame::SubmitAction(submit.clone()),
    );

    let result = SendActionResult {
        id: "req-1".to_owned(),
        result: format!(r#"{{"output":"{SECRET}"}}"#),
    };
    assert_redacted("SendActionResult", &result);
    assert_redacted(
        "ActionWsFrame::SendActionResult",
        &ActionWsFrame::from(result.clone()),
    );
    let decoded = wire::decode(&wire::encode(&result.clone().into())).unwrap();
    assert_redacted("decoded SendActionResult", &decoded);

    // The reaction of the stateless `handle_submit`.
    let reaction = HandlerReaction {
        receipt: Receipt::Acknowledged(Acknowledged {
            id: "req-1".to_owned(),
            code: 200,
            message: String::new(),
        }),
        result: Some(result),
    };
    assert_redacted("HandlerReaction", &reaction);
    let _: SubmitAction = submit;
}

/// A frame that does not decode reports where it was wrong, not what it held.
#[test]
fn a_wire_error_does_not_print_the_frame() {
    for text in [
        format!(r#"{{"type":"submitAction","id":"req-1","password":"{SECRET}""#),
        format!(
            r#"{{"type":"submitAction","id":7,"capability":"c","parameters":{{"p":"{SECRET}"}}}}"#
        ),
        // A result with no `id` is still refused (a result with no `result` is
        // accepted since the codec made it optional).
        format!(r#"{{"type":"sendActionResult","result":"x","password":"{SECRET}"}}"#),
    ] {
        let err = wire::decode(&text).expect_err(&text);
        assert_redacted("WireError", &err);
        assert!(!err.to_string().contains(SECRET), "{err}");
    }
}
