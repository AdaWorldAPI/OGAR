//! The invoker's text-frame adapter (`ogar_action_handler::invoke_session`)
//! against the HIRO parity oracle: the frames the invoker reads and writes.
//! `I` is graph_conn `lib/action_api/invoker.ex`, `T` is `test/action_api`.

use ogar_action_handler::invoke_session::{Effect, InvokeSession, encode_frame};
use ogar_action_handler::invoke_state::{
    CallRequest, Completion, Frame, InvokeError, Log, SendFailure, TimeoutArg,
};
use ogar_action_handler::wire::{self, ActionWsFrame, MAX_RETRY_AFTER_MS, WireError};
use serde_json::{Value, json};

fn req(call: u64) -> CallRequest {
    CallRequest::new(call, "ticket-1", "handler-1", "ExecuteCommand").param("command", json!("ls"))
}

/// The submit frame of the first effect, parsed.
fn submitted(fx: &[Effect]) -> Value {
    let Effect::Send { text, .. } = &fx[0] else {
        panic!("{fx:?}")
    };
    serde_json::from_str(text).expect("the invoker sends JSON")
}

fn started(s: &mut InvokeSession, now: u64, r: CallRequest) -> String {
    let fx = s.call(now, r);
    submitted(&fx)["id"].as_str().expect("id").to_owned()
}

fn completion(fx: &[Effect]) -> &Completion {
    let all: Vec<&Completion> = fx
        .iter()
        .filter_map(|e| match e {
            Effect::Complete(c) => Some(c),
            _ => None,
        })
        .collect();
    assert_eq!(all.len(), 1, "{fx:?}");
    all[0]
}

fn frame(v: &Value) -> String {
    v.to_string()
}

/// The wire shape of the submit (`R:23-30`, the Java javadoc
/// `ActionHandlerSubmit.java:14-22`): `type`, `id`, `handler`, `capability`,
/// `parameters`, `timeout` in ms. The parameters keep their JSON types, and
/// `parameters.timeout` is the number of milliseconds (`I:421`;
/// `T/invoker_test.exs:99-104` reads `"timeout" => 123_000` back).
#[test]
fn the_submit_frame_has_the_wire_shape_with_typed_parameters() {
    let mut s = InvokeSession::new();
    let fx = s.call(
        0,
        req(1)
            .param("retries", json!(3))
            .with_timeout(TimeoutArg::Int(123)),
    );
    let v = submitted(&fx);
    let id = v["id"].as_str().expect("id");
    assert_eq!(
        v,
        json!({
            "type": "submitAction",
            "id": id,
            "handler": "handler-1",
            "capability": "ExecuteCommand",
            "parameters": {"command": "ls", "retries": 3, "timeout": 123_000},
            "timeout": 123_000,
        })
    );
    // The handler-side decoder reads it (parameter values as text there).
    let ActionWsFrame::SubmitAction(sub) = wire::decode(&frame(&v)).expect("decodes") else {
        panic!()
    };
    assert_eq!(sub.id, id);
    assert_eq!(sub.timeout_millis, Some(123_000));
    assert_eq!(sub.capability, "ExecuteCommand");
}

/// Oracle 34 (`I:217`): the result ack is
/// `{"type":"acknowledged","id":ID,"code":200}` and nothing else, sent before
/// the completion.
#[test]
fn a34_the_result_ack_frame_and_its_order() {
    let mut s = InvokeSession::new();
    let id = started(&mut s, 0, req(1));
    s.on_text(1, &frame(&json!({"type": "acknowledged", "id": id})))
        .expect("ack");
    let fx = s
        .on_text(
            2,
            &frame(&json!({"type": "sendActionResult", "id": id, "result": "{\"ok\":true}"})),
        )
        .expect("result");
    assert_eq!(fx.len(), 2);
    let Effect::Send { call, text } = &fx[0] else {
        panic!("{fx:?}")
    };
    assert_eq!(*call, None);
    assert_eq!(
        serde_json::from_str::<Value>(text).expect("json"),
        json!({"type": "acknowledged", "id": id, "code": 200})
    );
    assert_eq!(completion(&fx[1..]).result, Ok(json!({"ok": true})));
}

/// Oracle 35 (`T/invoker_result_test.exs:24-48`): a result sent as a JSON object,
/// a bare number or a plain string, and a missing or `null` one (`nil`), all
/// reach the caller as HIRO hands them over.
#[test]
fn a35_result_shapes_on_the_wire() {
    let cases: Vec<(Value, Value)> = vec![
        (json!({"result": "{\"ok\":true}"}), json!({"ok": true})),
        (json!({"result": {"ok": true}}), json!({"ok": true})),
        (json!({"result": "ok"}), json!("ok")),
        (json!({"result": 42}), json!(42)),
        (json!({"result": true}), json!(true)),
        (json!({}), Value::Null),
        (json!({"result": null}), Value::Null),
    ];
    for (extra, want) in cases {
        let mut s = InvokeSession::new();
        let id = started(&mut s, 0, req(1));
        s.on_text(1, &frame(&json!({"type": "acknowledged", "id": id})))
            .expect("ack");
        let mut f = json!({"type": "sendActionResult", "id": id});
        f.as_object_mut()
            .expect("object")
            .extend(extra.as_object().expect("object").clone());
        let fx = s.on_text(2, &frame(&f)).expect("accepted");
        assert!(matches!(fx[0], Effect::Send { .. }), "acked first: {f}");
        assert_eq!(completion(&fx).result, Ok(want), "{f}");
    }
}

/// A `sendActionResult` with no id is a decode error, not a repaired one.
#[test]
fn a_result_with_no_id_is_a_wire_error() {
    let mut s = InvokeSession::new();
    let e = s
        .on_text(0, r#"{"type":"sendActionResult","result":"x"}"#)
        .expect_err("no id");
    assert_eq!(
        e,
        WireError::MissingField {
            frame: "sendActionResult",
            field: "id"
        }
    );
}

/// Oracle 24 (`P/tests/unit/test_actionwebsocket_rate_limit.py:25-27`,
/// `{"error": {"code": 429, "retryAfterMs": 1500}, "id": "a1"}`): the typeless
/// 429 completes the call at once; no resend.
#[test]
fn a24_the_typeless_429_frame_completes_the_call() {
    let mut s = InvokeSession::new();
    let id = started(&mut s, 0, req(1));
    let fx = s
        .on_text(
            5,
            &frame(&json!({"error": {"code": 429, "retryAfterMs": 1500}, "id": id})),
        )
        .expect("decodes");
    assert_eq!(fx.len(), 1);
    assert_eq!(completion(&fx).result, Err(InvokeError::RateLimited(1_500)));
    assert!(s.on_tick(1_000_000).is_empty());
    assert_eq!(s.state().tracked(), 0);
}

/// Oracle 25 (`mock_socket.ex:316-323`): the typeless 503 completes the call with
/// the frame's own error object, whatever fields it carries.
#[test]
fn a25_the_typeless_503_frame_completes_the_call_with_its_error_object() {
    let mut s = InvokeSession::new();
    let id = started(&mut s, 0, req(1));
    let error = json!({
        "code": 503,
        "message": "Upstream unavailable",
        "upstream": "action",
        "region": "eu-1"
    });
    let fx = s
        .on_text(5, &frame(&json!({"error": error, "id": id})))
        .expect("decodes");
    let Err(InvokeError::Refused(map)) = &completion(&fx).result else {
        panic!("{fx:?}")
    };
    assert_eq!(Value::Object(map.clone()), error, "no field is lost");
}

/// Oracle 26 (`I:246-251`, `:272-278`; `T/invoker_rate_limited_test.exs:153-179`,
/// `T/invoker_upstream_unavailable_test.exs:40-58`): an absent, `null` or
/// non-string id is no id. Nobody is answered.
#[test]
fn a26_frames_without_a_usable_id_are_markers_only() {
    let mut s = InvokeSession::new();
    started(&mut s, 0, req(1));
    for f in [
        json!({"error": {"code": 429, "message": "Rate limit exceeded", "retryAfterMs": 1986}}),
        json!({"error": {"code": 429, "retryAfterMs": 1986}, "id": null}),
        json!({"error": {"code": 429, "retryAfterMs": 1986}, "id": 7}),
    ] {
        let fx = s.on_text(1, &frame(&f)).expect("decodes");
        assert_eq!(
            fx,
            vec![Effect::Log(Log::RateLimitedNoId {
                retry_after_ms: 1_986
            })],
            "{f}"
        );
    }
    for f in [
        json!({"error": {"code": 503, "message": "Upstream unavailable"}}),
        json!({"error": {"code": 503, "message": "Upstream unavailable"}, "id": null}),
    ] {
        let fx = s.on_text(1, &frame(&f)).expect("decodes");
        assert_eq!(fx, vec![Effect::Log(Log::UpstreamUnavailableNoId)], "{f}");
    }
    assert_eq!(s.state().tracked(), 1);
}

/// Oracle 27 (`T/invoker_rate_limited_test.exs:181-206`): `retryAfterMs` of nil,
/// "1986", -1 or 999999999999, as frames. The caller gets an integer in
/// `0..=86_400_000`; a float of at least 1 is truncated (`retry_after.ex:82-91`).
#[test]
fn a27_the_advertised_wait_in_a_frame_is_sanitised() {
    let cases: Vec<(Option<Value>, u64)> = vec![
        (Some(json!(null)), 0),
        (None, 0),
        (Some(json!("1986")), 0),
        (Some(json!(-1)), 0),
        (Some(json!(0)), 0),
        (Some(json!(999_999_999_999_i64)), 86_400_000),
        (Some(json!(1986)), 1_986),
        (Some(json!(1986.0)), 1_986),
        (Some(json!(1986.9)), 1_986),
        (Some(json!(0.5)), 0),
        (Some(json!(true)), 0),
    ];
    assert_eq!(MAX_RETRY_AFTER_MS, 86_400_000);
    for (advertised, want) in cases {
        let mut s = InvokeSession::new();
        let id = started(&mut s, 0, req(1));
        let mut error = json!({"code": 429});
        if let Some(v) = &advertised {
            error["retryAfterMs"] = v.clone();
        }
        let fx = s
            .on_text(1, &frame(&json!({"error": error, "id": id})))
            .expect("decodes");
        assert_eq!(
            completion(&fx).result,
            Err(InvokeError::RateLimited(want)),
            "advertised {advertised:?}"
        );
    }
}

/// `I:239-240`, `:280-287`; `ws_connection.ex:258-264`: `configChanged` is handed
/// on, `hello` is the connection layer's (time advances, nothing else), a typed
/// `error` and any other frame with no clause is a marker with its kind.
#[test]
fn other_frames() {
    let mut s = InvokeSession::new();
    assert_eq!(
        s.on_text(0, r#"{"type":"configChanged"}"#).expect("ok"),
        vec![Effect::ConfigChanged]
    );
    assert!(
        s.on_text(0, r#"{"type":"hello","max_frame_bytes":1000000}"#)
            .expect("ok")
            .is_empty()
    );
    assert_eq!(
        s.on_text(
            0,
            r#"{"type":"error","code":400,"message":"invalid action message"}"#
        )
        .expect("ok"),
        vec![Effect::Log(Log::UnexpectedFrame {
            kind: "error".into()
        })]
    );
    assert_eq!(
        s.on_text(
            0,
            r#"{"type":"submitAction","id":"x","capability":"C","handler":"h","parameters":{}}"#
        )
        .expect("ok"),
        vec![Effect::Log(Log::UnexpectedFrame {
            kind: "submitAction".into()
        })]
    );
}

/// A frame that does not decode is a wire error and does nothing else: no
/// effect, and the clock does not advance.
#[test]
fn a_frame_that_does_not_decode_is_an_error_and_nothing_else() {
    let mut s = InvokeSession::new();
    started(&mut s, 100, req(1));
    for bad in ["not json", "[]", r#"{"type":"nope"}"#, r#"{"foo":1}"#] {
        assert!(s.on_text(5_000_000, bad).is_err(), "{bad}");
    }
    assert_eq!(s.next_wakeup(), Some(3_100));
    assert_eq!(s.state().tracked(), 1);
}

/// `I:621-634` through the adapter, and `ack_timeout` through to the frame loop:
/// a failed send ends the call, an unanswered call is resent as the same text.
#[test]
fn a_resend_is_the_same_frame_and_a_failed_send_is_reported() {
    let mut s = InvokeSession::new();
    let fx = s.call(0, req(1).with_ack_timeout(10));
    let first = fx[0].clone();
    let fx = s.on_tick(10);
    let Some(second) = fx.iter().find(|e| matches!(e, Effect::Send { .. })) else {
        panic!("{fx:?}")
    };
    assert_eq!(&first, second);
    let fx = s.on_send_failed(11, 1, SendFailure::Other("not_connected".into()));
    assert!(matches!(
        completion(&fx).result,
        Err(InvokeError::NotSent(_))
    ));
}

/// `encode_frame` of a result ack is exactly what `wire::decode` reads back.
#[test]
fn the_result_ack_decodes_as_an_acknowledged_frame() {
    let text = encode_frame(&Frame::AckResult { id: "r-1".into() });
    let ActionWsFrame::Acknowledged(a) = wire::decode(&text).expect("decodes") else {
        panic!()
    };
    assert_eq!((a.id.as_str(), a.code, a.message), ("r-1", Some(200), None));
}
