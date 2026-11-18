//! The text-frame session against literal HIRO `action-ws` JSON.
//!
//! Oracle numbers are those of the parity oracle (`hiro_oracle.md` section 7).
//! Every frame in and out is JSON text in HIRO's wire shape; nothing here builds a
//! typed frame.

use std::cell::Cell;

use ogar_action_handler::session::{Effect, Session};
use ogar_action_handler::submit_state::{Config, ExecResult};
use ogar_action_handler::wire::WireError;
use ogar_from_schema::action_ws::CapabilityExecutor;
use ogar_from_schema::do_arm::ActionParam;
use ogar_vocab::ActionDef;
use serde_json::{Value, json};

fn submit_frame(id: &str, timeout: Option<u64>) -> String {
    let mut v = json!({
        "type": "submitAction",
        "id": id,
        "handler": "handler-7",
        "capability": "ExecuteCommand",
        "parameters": {"command": "uptime"},
    });
    if let Some(t) = timeout {
        v["timeout"] = json!(t);
    }
    v.to_string()
}

fn ack_frame(id: &str) -> String {
    json!({"type": "acknowledged", "id": id, "code": 200}).to_string()
}

fn nack_frame(id: &str) -> String {
    json!({"type": "negativeAcknowledged", "id": id, "code": 400, "message": "bad"}).to_string()
}

fn parse(text: &str) -> Value {
    serde_json::from_str(text).expect("a frame is JSON")
}

fn sent(fx: &[Effect]) -> Vec<Value> {
    fx.iter()
        .filter_map(|e| match e {
            Effect::Send(s) => Some(parse(s)),
            Effect::Run { .. } => None,
        })
        .collect()
}

fn runs(fx: &[Effect]) -> usize {
    fx.iter()
        .filter(|e| matches!(e, Effect::Run { .. }))
        .count()
}

fn session() -> Session {
    Session::new(Config::default()).expect("default config is valid")
}

fn expected_ack(id: &str) -> Value {
    json!({"type": "acknowledged", "id": id, "code": 200, "message": ""})
}

fn expected_result(id: &str, result: &str) -> Value {
    json!({"type": "sendActionResult", "id": id, "result": result})
}

/// Oracle 1, 2, 16: a submit delivered twice while executing is acked twice and
/// runs once; the result is sent once, when it completes.
#[test]
fn repeated_submit_while_executing_runs_once_and_acks_twice() {
    let mut s = session();
    let frame = submit_frame("req-1", Some(30_000));

    let first = s.on_text(0, &frame).unwrap();
    assert_eq!(sent(&first), vec![expected_ack("req-1")]);
    assert_eq!(runs(&first), 1);
    assert_eq!(
        first[1],
        Effect::Run {
            id: "req-1".into(),
            capability: "ExecuteCommand".into(),
            parameters: vec![("command".into(), "uptime".into())],
        }
    );

    let second = s.on_text(5, &frame).unwrap();
    assert_eq!(sent(&second), vec![expected_ack("req-1")]);
    assert_eq!(runs(&second), 0);

    let done = s.on_finished(9, "req-1", ExecResult::Success(r#"{"output":"up"}"#.into()));
    assert_eq!(
        sent(&done),
        vec![expected_result("req-1", r#"{"output":"up"}"#)]
    );
    assert_eq!(runs(&done), 0);
}

/// Oracle 18: a repeat after completion is acked and answered from the cached
/// result, without running again.
#[test]
fn repeated_submit_after_completion_gets_cached_result_and_no_run() {
    let mut s = session();
    let frame = submit_frame("req-2", None);
    s.on_text(0, &frame).unwrap();
    s.on_finished(1, "req-2", ExecResult::Success(r#"{"ok":1}"#.into()));

    let again = s.on_text(100, &frame).unwrap();
    assert_eq!(runs(&again), 0);
    assert_eq!(
        sent(&again),
        vec![
            expected_ack("req-2"),
            expected_result("req-2", r#"{"ok":1}"#)
        ]
    );
}

/// Oracle 12: an unacked result is resent every 3000 ms as an encoded frame.
#[test]
fn unacked_result_is_resent_every_3000ms_as_frames() {
    let mut s = session();
    s.on_text(0, &submit_frame("req-3", None)).unwrap();
    let done = s.on_finished(10, "req-3", ExecResult::Success("{}".into()));
    assert_eq!(sent(&done), vec![expected_result("req-3", "{}")]);
    assert_eq!(s.next_wakeup(), Some(3_010));

    assert!(s.on_tick(3_009).is_empty());
    for at in [3_010, 6_010, 9_010] {
        let fx = s.on_tick(at);
        assert_eq!(sent(&fx), vec![expected_result("req-3", "{}")], "at {at}");
        assert_eq!(runs(&fx), 0);
    }
    assert_eq!(s.next_wakeup(), Some(12_010));
}

/// Oracle 13: an `acknowledged` frame for the result stops the resend.
#[test]
fn acknowledged_frame_stops_the_resend() {
    let mut s = session();
    s.on_text(0, &submit_frame("req-4", None)).unwrap();
    s.on_finished(10, "req-4", ExecResult::Success("{}".into()));
    assert_eq!(sent(&s.on_tick(3_010)).len(), 1);

    assert!(s.on_text(3_500, &ack_frame("req-4")).unwrap().is_empty());
    assert!(s.on_tick(6_010).is_empty());
    assert!(s.on_tick(60_000).is_empty());
}

/// Oracle 14: a `negativeAcknowledged` frame stops the resend too.
#[test]
fn negative_acknowledged_frame_stops_the_resend() {
    let mut s = session();
    s.on_text(0, &submit_frame("req-5", None)).unwrap();
    s.on_finished(10, "req-5", ExecResult::Success("{}".into()));

    assert!(s.on_text(100, &nack_frame("req-5")).unwrap().is_empty());
    assert!(s.on_tick(3_010).is_empty());
    assert!(s.on_tick(60_000).is_empty());
}

/// An ack for another id does not stop this id's resend.
#[test]
fn acknowledged_frame_for_another_id_does_not_stop_the_resend() {
    let mut s = session();
    s.on_text(0, &submit_frame("req-6", None)).unwrap();
    s.on_finished(10, "req-6", ExecResult::Success("{}".into()));
    s.on_text(20, &ack_frame("someone-else")).unwrap();
    assert_eq!(sent(&s.on_tick(3_010)).len(), 1);
}

/// Oracle 4: the `timeout` of the frame, in ms, is the execution deadline; the
/// expiry is the status-13 result of oracle 5.
#[test]
fn frame_timeout_in_ms_is_the_deadline_and_expiry_sends_the_timeout_result() {
    let mut s = session();
    s.on_text(0, &submit_frame("req-7", Some(500))).unwrap();
    assert_eq!(s.next_wakeup(), Some(500));
    assert!(s.on_tick(499).is_empty());

    let fx = s.on_tick(500);
    let frames = sent(&fx);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0]["type"], "sendActionResult");
    assert_eq!(frames[0]["id"], "req-7");
    let result = parse(frames[0]["result"].as_str().unwrap());
    assert_eq!(
        result,
        json!({"error": {
            "req_id": "req-7",
            "action_status": 13,
            "action_error": "Execution timed out after 500ms",
        }})
    );
    // The late finish is ignored.
    assert!(
        s.on_finished(600, "req-7", ExecResult::Success("{}".into()))
            .is_empty()
    );
}

/// Oracle 3: no `timeout` selects the default deadline, 60000 ms.
#[test]
fn absent_timeout_selects_the_default_deadline() {
    let mut s = session();
    s.on_text(0, &submit_frame("req-8", None)).unwrap();
    assert_eq!(s.next_wakeup(), Some(60_000));
    assert!(s.on_tick(59_999).is_empty());
    let fx = s.on_tick(60_000);
    let result = parse(sent(&fx)[0]["result"].as_str().unwrap());
    assert_eq!(
        result["error"]["action_error"],
        "Execution timed out after 60000ms"
    );
}

/// A negative `timeout` is a deadline already passed (a choice; the oracle is
/// silent).
#[test]
fn negative_timeout_is_an_expired_deadline() {
    let mut s = session();
    let text = r#"{"type":"submitAction","id":"req-9","handler":"h","capability":"ExecuteCommand","parameters":{},"timeout":-5}"#;
    let fx = s.on_text(0, text).unwrap();
    let frames = sent(&fx);
    assert_eq!(frames[0], expected_ack("req-9"));
    let result = parse(frames[1]["result"].as_str().unwrap());
    assert_eq!(result["error"]["action_status"], 13);
}

/// Control: two distinct ids both run.
#[test]
fn two_distinct_ids_both_run() {
    let mut s = session();
    let a = s.on_text(0, &submit_frame("req-a", None)).unwrap();
    let b = s.on_text(1, &submit_frame("req-b", None)).unwrap();
    assert_eq!(runs(&a), 1);
    assert_eq!(runs(&b), 1);
    assert_eq!(s.state().tracked(), 2);
}

/// Requirement 3. The oracle (section 3) says the Elixir handler answers any
/// frame it does not handle with a warning only (`handler.ex:152-159`); it sends
/// no NACK. So a frame that does not decode is an error to the caller, with no
/// execution and no frame out, and no panic.
#[test]
fn frames_that_do_not_decode_are_errors_with_no_run_and_no_reply() {
    let mut s = session();
    type Expect = fn(&WireError) -> bool;
    let cases: [(&str, Expect); 7] = [
        ("not json at all", |e| matches!(e, WireError::NotJson(_))),
        ("[1,2]", |e| matches!(e, WireError::NotAnObject)),
        ("", |e| matches!(e, WireError::NotJson(_))),
        (r#"{"id":"x"}"#, |e| matches!(e, WireError::MissingType)),
        (
            r#"{"type":"nope","id":"req-x"}"#,
            |e| matches!(e, WireError::UnknownType(t) if t == "nope"),
        ),
        (
            r#"{"type":"submitAction","id":"req-x","capability":"ExecuteCommand"}"#,
            |e| {
                matches!(
                    e,
                    WireError::MissingField {
                        field: "parameters",
                        ..
                    }
                )
            },
        ),
        (
            r#"{"type":"submitAction","id":7,"capability":"c","parameters":{}}"#,
            |e| matches!(e, WireError::WrongType { field: "id", .. }),
        ),
    ];
    for (text, expect) in cases {
        let err = s.on_text(0, text).expect_err(text);
        assert!(expect(&err), "{text}: {err:?}");
    }
    assert_eq!(s.state().tracked(), 0);
    assert_eq!(s.next_wakeup(), None);
}

/// Frames the handler does not act on decode fine and are ignored.
#[test]
fn frames_the_handler_does_not_act_on_are_ignored() {
    let mut s = session();
    for text in [
        r#"{"type":"hello","max_frame_bytes":1048576}"#,
        r#"{"type":"configChanged"}"#,
        r#"{"type":"error","code":400,"message":"invalid action message"}"#,
        r#"{"type":"sendActionResult","id":"x","result":"{}"}"#,
        r#"{"error":{"code":429,"retryAfterMs":1500},"id":"a1"}"#,
    ] {
        assert!(s.on_text(0, text).unwrap().is_empty(), "{text}");
    }
    assert_eq!(s.state().tracked(), 0);
}

/// Requirement 5: the episode carries the handler id and the capability; the
/// adapter resolves nothing else.
#[test]
fn episode_records_handler_and_capability_and_nothing_resolved() {
    let mut s = session();
    s.on_text(0, &submit_frame("req-10", None)).unwrap();
    let ep = s.state().episode("req-10").expect("tracked");
    assert_eq!(ep.handler_id, "handler-7");
    assert_eq!(ep.capability, "ExecuteCommand");
    assert_eq!(ep.class_id, None);
    assert_eq!(ep.target, None);
    assert_eq!(ep.effect_class, None);
}

struct Counting(Cell<u32>);

impl CapabilityExecutor for Counting {
    fn execute(
        &self,
        _capability: &str,
        bound: &[(String, String)],
    ) -> Result<Vec<(String, String)>, String> {
        self.0.set(self.0.get() + 1);
        Ok(bound.to_vec())
    }
}

fn def_and_signature() -> (ActionDef, Vec<ActionParam>) {
    let def = ActionDef::new(
        "ogit-automation/action_capability::action_def::ExecuteCommand",
        "ExecuteCommand",
        "ogit-automation/mars_machine",
    );
    let sig = vec![ActionParam {
        name: "command".to_owned(),
        mandatory: true,
        default: None,
    }];
    (def, sig)
}

/// The drop-in: a repeated id runs the executor once, through the same bind and
/// execute path as `handle_submit`.
#[test]
fn sync_convenience_runs_a_repeated_id_once() {
    let (def, sig) = def_and_signature();
    let exec = Counting(Cell::new(0));
    let mut s = session();
    let frame = submit_frame("req-11", None);

    let first = s.on_text_sync(0, &frame, &def, &sig, &exec).unwrap();
    let first: Vec<Value> = first.iter().map(|f| parse(f)).collect();
    assert_eq!(
        first,
        vec![
            expected_ack("req-11"),
            expected_result("req-11", r#"{"command":"uptime"}"#)
        ]
    );
    let second = s.on_text_sync(10, &frame, &def, &sig, &exec).unwrap();
    let second: Vec<Value> = second.iter().map(|f| parse(f)).collect();
    assert_eq!(
        second,
        vec![
            expected_ack("req-11"),
            expected_result("req-11", r#"{"command":"uptime"}"#)
        ]
    );
    assert_eq!(exec.0.get(), 1);

    let other = submit_frame("req-12", None);
    s.on_text_sync(20, &other, &def, &sig, &exec).unwrap();
    assert_eq!(exec.0.get(), 2);
}

/// Oracle 9: a capability nobody handles is a result, not a NACK.
#[test]
fn sync_convenience_reports_an_unhandled_capability_as_a_result() {
    let (def, sig) = def_and_signature();
    let exec = Counting(Cell::new(0));
    let mut s = session();
    let text = r#"{"type":"submitAction","id":"req-13","handler":"h","capability":"Nope","parameters":{}}"#;
    let out = s.on_text_sync(0, text, &def, &sig, &exec).unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(parse(&out[0]), expected_ack("req-13"));
    let result = parse(parse(&out[1])["result"].as_str().unwrap());
    assert_eq!(
        result,
        json!({"error": {"code": 404, "message": "Unhandled capability \"Nope\""}})
    );
    assert_eq!(exec.0.get(), 0);
}
