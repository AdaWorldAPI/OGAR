//! An oversized result becomes `action_status` 59: oracle 10 and 11.
//!
//! `G` is `/home/user/bardioc/extraction/corpus/graph_conn`. The strings and
//! numbers are from `G/lib/action_api/result_frame.ex:27-38`, its test
//! `G/test/action_api/result_frame_test.exs:18-62`, and
//! `G/test/action_api/frame_limit_test.exs:10-26`. The default limit of 1,000,000
//! bytes is `G/lib/graph_conn/connection_manager.ex:61`, `:201-221`.
//!
//! The oracle's assertion 10 names `GraphConn.max_frame_bytes(handler, :"action-ws")`
//! without its default; the source returns 1,000,000 when the `hello` advertised
//! none (`connection_manager.ex:201-221`, `README.md:174-182`), so a session with no
//! `hello` fits at 1,000,000 and does not skip fitting.

use ogar_action_handler::session::{Effect, Session};
use ogar_action_handler::submit_state::{Config, ExecResult, Outcome};
use ogar_action_handler::wire::{
    ActionWsFrame, DEFAULT_MAX_FRAME_BYTES, TOO_BIG_STATUS, decode, fit, frame_bytes,
};
use ogar_from_schema::action_ws::{
    Acknowledged, NegativeAcknowledged, SendActionResult, SubmitAction,
};
use serde_json::{Value, json};

fn parse(text: &str) -> Value {
    serde_json::from_str(text).expect("JSON")
}

/// `_result_request/2` of `result_frame_test.exs:7-11`.
fn result_frame(id: &str, result_bytes: usize) -> ActionWsFrame {
    SendActionResult {
        id: id.to_owned(),
        result: json!({"data": "a".repeat(result_bytes)}).to_string(),
    }
    .into()
}

fn result_text(frame: &ActionWsFrame) -> Value {
    let ActionWsFrame::SendActionResult(r) = frame else {
        panic!("not a result frame");
    };
    parse(r.result.as_deref().expect("a result"))
}

// ─────────────────────────────────────────────────────── the pure fit ──

/// `result_frame_test.exs:19-23`.
#[test]
fn fit_leaves_a_result_that_fits_untouched() {
    let frame = result_frame("req-small", 100);
    assert_eq!(frame, fit(frame.clone(), 1_000_000));
}

/// `result_frame_test.exs:25-30`: a frame exactly at the limit is kept.
#[test]
fn fit_leaves_a_result_exactly_at_the_limit_untouched() {
    let frame = result_frame("req-exact", 1_000);
    let limit = frame_bytes(&frame);
    assert_eq!(frame, fit(frame.clone(), limit));
    assert_ne!(frame, fit(frame.clone(), limit - 1));
}

/// Oracle 10. `result_frame_test.exs:32-45`: id, type and the exact text.
#[test]
fn fit_replaces_a_result_over_the_limit_with_the_status_59_error() {
    let frame = result_frame("req-big", 1_500_000);
    let fitted = fit(frame, 1_000_000);
    let ActionWsFrame::SendActionResult(r) = &fitted else {
        panic!("still a result frame");
    };
    assert_eq!(r.id, "req-big");
    assert_eq!(
        parse(r.result.as_deref().expect("a result")),
        json!({"error": {
            "req_id": "req-big",
            "action_status": 59,
            "action_error": "Response was too big (1.50 MB, limit 1.00 MB)",
        }})
    );
    assert_eq!(TOO_BIG_STATUS, 59);
}

/// `result_frame_test.exs:47-56`: the replacement fits a limit of 1000.
#[test]
fn the_replacement_fits_the_limit() {
    let fitted = fit(result_frame("req-big", 5_000), 1_000);
    assert!(frame_bytes(&fitted) <= 1_000);
    assert_eq!(result_text(&fitted)["error"]["action_status"], 59);
}

/// The Elixir code does not fit the replacement again; neither does this one. A
/// limit below the size of the error gets the error, not a loop and not nothing.
#[test]
fn a_limit_below_the_replacements_own_size_still_gets_the_error() {
    let fitted = fit(result_frame("req-tiny", 5_000), 10);
    assert_eq!(result_text(&fitted)["error"]["action_status"], 59);
    assert!(frame_bytes(&fitted) > 10);
}

/// Oracle 11. `result_frame_test.exs:58-62`: any other frame is unchanged,
/// whatever its size, even with a limit of 1.
#[test]
fn fit_leaves_every_other_frame_untouched_whatever_its_size() {
    let frames: Vec<ActionWsFrame> = vec![
        Acknowledged {
            id: "req-ack".into(),
            code: 200,
            message: String::new(),
        }
        .into(),
        NegativeAcknowledged {
            id: "req-nack".into(),
            code: 400,
            message: "x".repeat(10_000),
        }
        .into(),
        SubmitAction {
            id: "req-submit".into(),
            capability: "ExecuteCommand".into(),
            handler: "h".into(),
            scope: None,
            parameters: vec![("payload".into(), "p".repeat(10_000))],
            timeout_millis: None,
        }
        .into(),
        decode(r#"{"type":"hello","max_frame_bytes":5}"#).unwrap(),
        ActionWsFrame::ConfigChanged,
    ];
    for frame in frames {
        assert_eq!(frame, fit(frame.clone(), 1));
    }
}

// ───────────────────────────────────────────────────── in the session ──

fn session() -> Session {
    Session::new(Config::default()).unwrap()
}

fn submit_frame(id: &str, timeout: Option<u64>) -> String {
    let mut v = json!({
        "type": "submitAction", "id": id, "handler": "h",
        "capability": "ExecuteCommand", "parameters": {"command": "ls"},
    });
    if let Some(t) = timeout {
        v["timeout"] = json!(t);
    }
    v.to_string()
}

fn sent(fx: &[Effect]) -> Vec<Value> {
    fx.iter()
        .filter_map(|e| match e {
            Effect::Send(s) => Some(parse(s)),
            Effect::Run { .. } => None,
        })
        .collect()
}

fn big_success(bytes: usize) -> ExecResult {
    ExecResult::Success(json!({"data": "a".repeat(bytes)}).to_string())
}

/// The error of `frame_limit_test.exs:11-17`, read from the frame the session sent.
fn assert_too_big(frame: &Value, id: &str) {
    assert_eq!(frame["type"], "sendActionResult");
    assert_eq!(frame["id"], id);
    let result = parse(frame["result"].as_str().unwrap());
    assert_eq!(result["error"]["req_id"], id);
    assert_eq!(result["error"]["action_status"], 59);
    let text = result["error"]["action_error"].as_str().unwrap();
    assert!(text.starts_with("Response was too big ("), "{text}");
    assert!(text.contains(" MB, limit "), "{text}");
}

/// Oracle 10 through the session: a `hello` announces the limit; the oversized
/// result is sent as the 59 error, the first time and on a resend.
#[test]
fn a_hello_limit_turns_an_oversized_result_into_59_on_send_and_on_resend() {
    let mut s = session();
    assert!(
        s.on_text(0, r#"{"type":"hello","max_frame_bytes":200000}"#)
            .unwrap()
            .is_empty()
    );
    assert_eq!(s.state().max_frame_bytes(), 200_000);
    s.on_text(1, &submit_frame("req-1", None)).unwrap();
    let fx = s.on_finished(2, "req-1", big_success(200_000));
    let frames = sent(&fx);
    assert_eq!(frames.len(), 1);
    assert_too_big(&frames[0], "req-1");
    let first = frames[0].clone();

    // `frame_limit_test.exs:39-47`: never resends an oversized result.
    let resend = sent(&s.on_tick(3_002));
    assert_eq!(resend, vec![first.clone()]);
    assert_eq!(
        s.state().episode("req-1").and_then(|e| e.outcome),
        Some(Outcome::Failed(Some(59)))
    );
}

/// `frame_limit_test.exs:20-26`: the cached result is the fitted one, so a
/// redelivery never carries the big one.
#[test]
fn the_cached_result_is_the_fitted_one() {
    let mut s = session();
    s.on_text(0, r#"{"type":"hello","max_frame_bytes":1000}"#)
        .unwrap();
    s.on_text(1, &submit_frame("req-2", None)).unwrap();
    let first = sent(&s.on_finished(2, "req-2", big_success(5_000)));
    assert_too_big(&first[0], "req-2");

    let again = sent(&s.on_text(10, &submit_frame("req-2", None)).unwrap());
    assert_eq!(again.len(), 2, "ack and the cached result");
    assert_eq!(again[1], first[0]);
}

/// Oracle 10: a result that fits is sent as is, at the limit and below it.
#[test]
fn a_result_within_the_limit_is_sent_as_is() {
    let mut s = session();
    s.on_text(0, r#"{"type":"hello","max_frame_bytes":100000}"#)
        .unwrap();
    s.on_text(1, &submit_frame("req-3", None)).unwrap();
    let payload = json!({"data": "a".repeat(1_000)}).to_string();
    let fx = s.on_finished(2, "req-3", ExecResult::Success(payload.clone()));
    let frames = sent(&fx);
    assert_eq!(frames[0]["result"], payload);
}

/// No `hello`: the default limit of 1,000,000 bytes applies (the source, not the
/// oracle's summary).
#[test]
fn with_no_hello_the_default_limit_applies() {
    assert_eq!(DEFAULT_MAX_FRAME_BYTES, 1_000_000);
    let mut s = session();
    assert_eq!(s.state().max_frame_bytes(), 1_000_000);

    s.on_text(0, &submit_frame("req-4", None)).unwrap();
    let frames = sent(&s.on_finished(1, "req-4", big_success(1_500_000)));
    assert_eq!(
        parse(frames[0]["result"].as_str().unwrap())["error"]["action_error"],
        "Response was too big (1.50 MB, limit 1.00 MB)"
    );

    s.on_text(2, &submit_frame("req-5", None)).unwrap();
    let frames = sent(&s.on_finished(3, "req-5", big_success(900_000)));
    assert_eq!(
        parse(frames[0]["result"].as_str().unwrap())["data"]
            .as_str()
            .unwrap()
            .len(),
        900_000
    );
}

/// A `hello` that advertises no positive integer limit is the default again, and
/// replaces the limit of an earlier `hello` (`connection_manager.ex:360-366`
/// stores what the latest `hello` advertised; `ws_connection.ex:352-357`).
#[test]
fn a_hello_without_a_positive_limit_is_the_default() {
    for text in [
        r#"{"type":"hello"}"#,
        r#"{"type":"hello","max_frame_bytes":0}"#,
        r#"{"type":"hello","max_frame_bytes":-5}"#,
        r#"{"type":"hello","max_frame_bytes":"2000"}"#,
        r#"{"type":"hello","max_frame_bytes":null}"#,
        r#"{"type":"hello","max_frame_bytes":1.5}"#,
    ] {
        let mut s = session();
        s.on_text(0, r#"{"type":"hello","max_frame_bytes":100}"#)
            .unwrap();
        assert_eq!(s.state().max_frame_bytes(), 100);
        s.on_text(1, text).unwrap();
        assert_eq!(s.state().max_frame_bytes(), 1_000_000, "{text}");
    }
}

/// Oracle 11 in the session: only the result is ever fitted. With a limit far
/// below every frame, the ack passes unchanged and the result becomes the 59 error.
#[test]
fn only_the_result_is_fitted_the_ack_passes_whatever_the_limit() {
    let mut s = session();
    s.on_text(0, r#"{"type":"hello","max_frame_bytes":1}"#)
        .unwrap();
    let fx = s.on_text(1, &submit_frame("req-6", None)).unwrap();
    assert_eq!(
        sent(&fx),
        vec![json!({"type": "acknowledged", "id": "req-6", "code": 200, "message": ""})]
    );
    let frames = sent(&s.on_finished(2, "req-6", ExecResult::Success("{}".into())));
    assert_too_big(&frames[0], "req-6");
}

/// Every result is fitted, not only a successful one: a timeout result over the
/// limit is the 59 error as well (`handler.ex:178` fits whatever
/// `_execute_unlinked` returns, the status 13 error included).
#[test]
fn a_timeout_result_is_fitted_too() {
    let mut s = session();
    s.on_text(0, r#"{"type":"hello","max_frame_bytes":100}"#)
        .unwrap();
    s.on_text(1, &submit_frame("req-7", Some(10))).unwrap();
    let frames = sent(&s.on_tick(11));
    assert_too_big(&frames[0], "req-7");
}

/// A limit set later does not rewrite a result already finished.
#[test]
fn a_later_hello_does_not_rewrite_a_finished_result() {
    let mut s = session();
    s.on_text(0, &submit_frame("req-8", None)).unwrap();
    let first = sent(&s.on_finished(1, "req-8", big_success(2_000)));
    s.on_text(2, r#"{"type":"hello","max_frame_bytes":500}"#)
        .unwrap();
    let again = sent(&s.on_tick(3_001));
    assert_eq!(again, first);
}
