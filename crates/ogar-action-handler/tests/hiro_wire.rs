//! The HIRO `action-ws` wire codec against HIRO's own JSON.
//!
//! Every fixture names its source. `G` is the graph_conn Elixir client
//! (`/home/user/bardioc/extraction/corpus/graph_conn`), `J` the Java client and `P`
//! the Python client (siblings under `/home/user/bardioc/extraction/corpus/`). Where
//! the source builds the frame as an Elixir map (not JSON text), the fixture here is
//! the JSON of that map and says so. Assertion numbers are those of the parity
//! oracle (`hiro_oracle.md` section 7).
//!
//! Not pinned here (they are handler or invoker behaviour, not wire shape): 2-3,
//! 5-23, 28, 31-32, 36-43, 47-52, 55-58. Assertion 1 is pinned for the wire shape
//! only; `acknowledge()` now words its message `""` as the handler does (the text
//! was "Received the action" until 2026-11-19).

use ogar_action_handler::wire::{
    AckFrame, ActionWsFrame, ClientHello, Hello, MAX_RETRY_AFTER_MS, RateLimited,
    UpstreamUnavailable, WireError, decode, encode,
};
use ogar_from_schema::action_ws::{
    Acknowledged, InboundError, NegativeAcknowledged, SendActionResult, SubmitAction,
};
use serde_json::Value;

fn json(text: &str) -> Value {
    serde_json::from_str(text).expect("fixture is JSON")
}

/// Encode, check it equals `wire` as JSON, decode `wire`, check it equals `frame`.
fn pin(frame: &ActionWsFrame, wire: &str) {
    assert_eq!(json(&encode(frame)), json(wire), "encode of {frame:?}");
    assert_eq!(&decode(wire).expect("decodes"), frame, "decode of {wire}");
}

// ─────────────────────────────────────────────────── submitAction ──

/// Pins oracle 4 (wire timeout is milliseconds; the frame carries `timeout` as an
/// integer of ms) and the envelope of section 1.
/// Source: J `ActionHandlerSubmit.java:14-22` (javadoc), verbatim.
#[test]
fn submit_action_java_javadoc_roundtrips() {
    let wire = r#"{"type": "submitAction", "id": "id1", "handler": "handlerId", "capability": "ExecuteCommand", "parameters": {}, "timeout": 30000}"#;
    let frame = ActionWsFrame::SubmitAction(SubmitAction {
        id: "id1".to_owned(),
        capability: "ExecuteCommand".to_owned(),
        handler: "handlerId".to_owned(),
        scope: None,
        parameters: vec![],
        timeout_millis: Some(30_000),
    });
    pin(&frame, wire);
}

/// Pins oracle 4, 29 and 30 as far as the wire goes: the frame the invoker sends
/// holds milliseconds (`123` seconds became `123000`), and a numeric parameter is
/// read as its JSON text. Source: G `lib/action_api/request.ex:23-30` (the
/// `Jason.Encoder` struct: `type`, `id`, `handler`, `capability`, `parameters`,
/// `timeout`, default timeout 60_000) filled with the values of
/// `test/action_api/invoker_test.exs:99-104` (`command` "ls", `timeout` 123_000).
/// `id` and `handler` are placeholders: the source computes the id and the test
/// takes the handler from config.
#[test]
fn submit_action_elixir_request_numeric_parameter_is_read_as_text() {
    let wire = r#"{"type":"submitAction","id":"MFRGGZDFMZTWQ2LK","handler":"action_handler","capability":"ExecuteCommand","parameters":{"command":"ls","timeout":123000},"timeout":60000}"#;
    let ActionWsFrame::SubmitAction(s) = decode(wire).expect("decodes") else {
        panic!("not a submitAction");
    };
    assert_eq!(s.timeout_millis, Some(60_000));
    assert_eq!(
        s.parameters,
        vec![
            ("command".to_owned(), "ls".to_owned()),
            ("timeout".to_owned(), "123000".to_owned()),
        ]
    );
}

/// Pins oracle 4: the timeout is ms on the wire, so `timeout_millis` is written
/// as `timeout` and never as `timeout_millis`, and parameters are an object.
/// Source for the frame shape: J `ActionHandlerSubmit.java:14-22`.
#[test]
fn submit_action_encodes_timeout_in_ms_and_parameters_as_an_object() {
    let frame = ActionWsFrame::SubmitAction(SubmitAction {
        id: "id1".to_owned(),
        capability: "ExecuteCommand".to_owned(),
        handler: "handlerId".to_owned(),
        scope: Some("tenant-a".to_owned()),
        parameters: vec![
            ("command".to_owned(), "uptime".to_owned()),
            ("host".to_owned(), "node-9".to_owned()),
        ],
        timeout_millis: Some(123_000),
    });
    let v = json(&encode(&frame));
    assert_eq!(v["type"], "submitAction");
    assert_eq!(v["timeout"], 123_000);
    assert!(v.get("timeout_millis").is_none());
    assert!(v.get("scope").is_none(), "scope is not a HIRO wire field");
    assert_eq!(
        v["parameters"],
        json(r#"{"command":"uptime","host":"node-9"}"#)
    );
}

/// Pins the section 1 note that a submit without `timeout` is legal (the handler
/// then applies its own default, oracle 3): it decodes to `None`.
/// Source: G `lib/action_api/handler.ex:121-133` reads `msg["timeout"]` as optional.
#[test]
fn submit_action_without_timeout_decodes_to_none() {
    let wire =
        r#"{"type":"submitAction","id":"id1","capability":"ExecuteCommand","parameters":{}}"#;
    let ActionWsFrame::SubmitAction(s) = decode(wire).expect("decodes") else {
        panic!("not a submitAction");
    };
    assert_eq!(s.timeout_millis, None);
    assert_eq!(s.handler, "", "graph_conn's handler never reads `handler`");
}

/// Parameters that are not an object are refused by name.
/// Source: G `test/graph_conn/mock_socket_test.exs:38-58` and `:102-118`: a
/// `submitAction` frame (`type`, `id`, `handler`, `capability`, `parameters`,
/// `timeout` 5_000) whose parameters are `nil` or `["ls"]`, the Elixir values
/// rendered as JSON `null` and `["ls"]`.
#[test]
fn submit_action_with_non_object_parameters_is_refused() {
    let null = r#"{"type":"submitAction","id":"u1","handler":"no-such-handler","capability":"NoSuchCapability","parameters":null,"timeout":5000}"#;
    assert_eq!(
        decode(null),
        Err(WireError::MissingField {
            frame: "submitAction",
            field: "parameters"
        })
    );
    let list = r#"{"type":"submitAction","id":"u1","handler":"no-such-handler","capability":"NoSuchCapability","parameters":["ls"],"timeout":5000}"#;
    assert_eq!(
        decode(list),
        Err(WireError::WrongType {
            frame: "submitAction",
            field: "parameters",
            expected: "an object"
        })
    );
}

/// A submit lacking `id`, `capability` or `parameters`, or with a non-integer
/// `timeout`, is refused by field name. Source of the required set: G
/// `lib/action_api/handler.ex:121-130` (the clause matches `type`, `id`,
/// `capability`, `parameters`).
#[test]
fn submit_action_missing_or_ill_typed_fields_are_refused_by_name() {
    assert_eq!(
        decode(r#"{"type":"submitAction","capability":"c","parameters":{}}"#),
        Err(WireError::MissingField {
            frame: "submitAction",
            field: "id"
        })
    );
    assert_eq!(
        decode(r#"{"type":"submitAction","id":"i","parameters":{}}"#),
        Err(WireError::MissingField {
            frame: "submitAction",
            field: "capability"
        })
    );
    assert_eq!(
        decode(
            r#"{"type":"submitAction","id":"i","capability":"c","parameters":{},"timeout":"30"}"#
        ),
        Err(WireError::WrongType {
            frame: "submitAction",
            field: "timeout",
            expected: "an integer (milliseconds)"
        })
    );
    assert_eq!(
        decode(r#"{"type":"submitAction","id":7,"capability":"c","parameters":{}}"#),
        Err(WireError::WrongType {
            frame: "submitAction",
            field: "id",
            expected: "a string"
        })
    );
}

// ───────────────────────────────────────────── acknowledged / nack ──

/// Pins oracle 1: the handler's ack is exactly
/// `{"type":"acknowledged","id":"X","code":200,"message":""}`.
/// Source: G `lib/action_api/handler.ex:170` (the Elixir map `%{id: req_id, type:
/// "acknowledged", code: 200, message: ""}` as JSON).
#[test]
fn handler_ack_wire_shape() {
    let frame = ActionWsFrame::from(Acknowledged {
        id: "X".to_owned(),
        code: 200,
        message: String::new(),
    });
    pin(
        &frame,
        r#"{"id":"X","type":"acknowledged","code":200,"message":""}"#,
    );
}

/// Pins oracle 46: the Python handler's submit ack carries code 200 and the text
/// "submitAction acknowledged". Source: P `actionwebsocket.py:605`
/// (`ActionHandlerAck(action_message.id, 200, "submitAction acknowledged")`),
/// serialised by `__str__` at `:45-49` as `json.dumps({"type": ..., **vars(self)})`,
/// so `type`, `id`, `code`, `message` in that order with Python's default spacing.
/// The id `a1` is a placeholder.
#[test]
fn python_submit_ack_wire_shape() {
    let frame = ActionWsFrame::from(Acknowledged {
        id: "a1".to_owned(),
        code: 200,
        message: "submitAction acknowledged".to_owned(),
    });
    pin(
        &frame,
        r#"{"type": "acknowledged", "id": "a1", "code": 200, "message": "submitAction acknowledged"}"#,
    );
}

/// Pins oracle 45 (the gateway's ack carries only `type` and `id`) and oracle 34
/// (the invoker's ack of a result carries `type`, `id`, `code` 200, no message).
/// Sources: G `lib/graph_conn/test/mock_socket.ex:268-273` and
/// `lib/action_api/invoker.ex:217`, Elixir maps as JSON.
#[test]
fn ack_code_and_message_are_optional_on_the_wire() {
    let id_only = ActionWsFrame::from(AckFrame {
        id: "r1".to_owned(),
        code: None,
        message: None,
    });
    pin(&id_only, r#"{"type":"acknowledged","id":"r1"}"#);

    let with_code = ActionWsFrame::from(AckFrame {
        id: "r1".to_owned(),
        code: Some(200),
        message: None,
    });
    pin(
        &with_code,
        r#"{"type":"acknowledged","id":"r1","code":200}"#,
    );
}

/// An ack without `code` has no typed [`Acknowledged`]: no code is invented.
#[test]
fn ack_without_code_does_not_become_a_typed_acknowledged() {
    let ActionWsFrame::Acknowledged(a) = decode(r#"{"type":"acknowledged","id":"r1"}"#).unwrap()
    else {
        panic!("not an ack");
    };
    assert!(a.clone().into_acknowledged().is_none());
    let ActionWsFrame::Acknowledged(b) =
        decode(r#"{"type":"acknowledged","id":"r1","code":200}"#).unwrap()
    else {
        panic!("not an ack");
    };
    assert_eq!(
        b.into_acknowledged(),
        Some(Acknowledged {
            id: "r1".to_owned(),
            code: 200,
            message: String::new()
        })
    );
}

/// Pins oracle 33 and 44: a `negativeAcknowledged` with code 404 and the message
/// `capability X not found` decodes to the typed nack. Source: G
/// `lib/graph_conn/test/mock_socket.ex:278-285` (Elixir map as JSON) with the
/// capability name of `test/action_api/invoker_test.exs:139`.
#[test]
fn nack_404_roundtrips() {
    let frame = ActionWsFrame::from(NegativeAcknowledged {
        id: "u1".to_owned(),
        code: 404,
        message: "capability invalid_capability not found".to_owned(),
    });
    pin(
        &frame,
        r#"{"type":"negativeAcknowledged","id":"u1","code":404,"message":"capability invalid_capability not found"}"#,
    );
}

/// Source: J `ActionHandlerNack.java:11-17` (javadoc) with the quoting typo
/// corrected (`"message: "bad request"` to `"message": "bad request"`); the oracle
/// notes the field set is `type,id,code,message`.
#[test]
fn nack_java_javadoc_field_set_roundtrips() {
    let frame = ActionWsFrame::from(NegativeAcknowledged {
        id: "id1".to_owned(),
        code: 400,
        message: "bad request".to_owned(),
    });
    pin(
        &frame,
        r#"{"type": "negativeAcknowledged", "id": "id1", "code": 400, "message": "bad request"}"#,
    );
}

/// The javadoc as written, with its typo, is not JSON and is refused as such.
/// Source: J `ActionHandlerNack.java:11-17`, verbatim (the `"message: "` quote).
#[test]
fn nack_java_javadoc_verbatim_typo_is_not_json() {
    let verbatim =
        r#"{"type": "negativeAcknowledged", "id": "id1", "code": 400, "message: "bad request"}"#;
    assert!(matches!(decode(verbatim), Err(WireError::NotJson(_))));
}

/// A nack needs `code` and `message` (oracle section 1 lists both as required).
#[test]
fn nack_missing_code_or_message_is_refused_by_name() {
    assert_eq!(
        decode(r#"{"type":"negativeAcknowledged","id":"i","message":"m"}"#),
        Err(WireError::MissingField {
            frame: "negativeAcknowledged",
            field: "code"
        })
    );
    assert_eq!(
        decode(r#"{"type":"negativeAcknowledged","id":"i","code":400}"#),
        Err(WireError::MissingField {
            frame: "negativeAcknowledged",
            field: "message"
        })
    );
    assert_eq!(
        decode(r#"{"type":"negativeAcknowledged","id":"i","code":70000,"message":"m"}"#),
        Err(WireError::WrongType {
            frame: "negativeAcknowledged",
            field: "code",
            expected: "an integer in 0..=65535"
        })
    );
}

// ───────────────────────────────────────────── sendActionResult ──

/// Source: J `ActionHandlerResult.java:11-14` (javadoc), verbatim.
#[test]
fn send_action_result_java_javadoc_roundtrips() {
    let frame = ActionWsFrame::from(SendActionResult {
        id: "id1".to_owned(),
        result: "...".to_owned(),
    });
    pin(
        &frame,
        r#"{"type": "sendActionResult", "id": "id1", "result": "..."}"#,
    );
}

/// Pins oracle 35 on the wire side: the handler's `result` is a string holding the
/// JSON text of the result. Source: G `lib/action_api/handler.ex:267` (the map
/// `%{id, type: "sendActionResult", result: response}`) with the result string of
/// `test/action_api/invoker_result_test.exs:19` (`{"ok":true}`).
#[test]
fn send_action_result_carries_the_result_as_json_text() {
    let frame = ActionWsFrame::from(SendActionResult {
        id: "X".to_owned(),
        result: r#"{"ok":true}"#.to_owned(),
    });
    let wire = r#"{"id":"X","type":"sendActionResult","result":"{\"ok\":true}"}"#;
    pin(&frame, wire);
    assert!(json(&encode(&frame))["result"].is_string());
}

/// Pins oracle 35 on the wire side, the three result forms the invoker accepts.
/// Source: G `test/action_api/invoker_result_test.exs:19-40`: result as a JSON
/// string (`{"ok":true}`), as a JSON object, as a string that is not JSON (`ok`) and
/// as a bare number (`42`). Object and number are kept as their JSON text.
#[test]
fn send_action_result_accepts_the_invoker_result_forms() {
    let cases = [
        (
            r#"{"type":"sendActionResult","id":"X","result":"{\"ok\":true}"}"#,
            r#"{"ok":true}"#,
        ),
        (
            r#"{"type":"sendActionResult","id":"X","result":{"ok":true}}"#,
            r#"{"ok":true}"#,
        ),
        (
            r#"{"type":"sendActionResult","id":"X","result":"ok"}"#,
            "ok",
        ),
        (r#"{"type":"sendActionResult","id":"X","result":42}"#, "42"),
    ];
    for (wire, expected) in cases {
        let ActionWsFrame::SendActionResult(r) = decode(wire).expect("decodes") else {
            panic!("not a result");
        };
        assert_eq!(r.id, "X");
        assert_eq!(r.result, expected, "{wire}");
    }
}

/// Oracle 35, the missing-result case: graph_conn hands the caller `nil`
/// (`invoker_result_test.exs:42-48`); a `String` result cannot say nil, so the
/// codec refuses it by name instead of inventing an empty result.
#[test]
fn send_action_result_without_result_is_refused_by_name() {
    assert_eq!(
        decode(r#"{"type":"sendActionResult","id":"X"}"#),
        Err(WireError::MissingField {
            frame: "sendActionResult",
            field: "result"
        })
    );
    assert_eq!(
        decode(r#"{"type":"sendActionResult","id":"X","result":null}"#),
        Err(WireError::MissingField {
            frame: "sendActionResult",
            field: "result"
        })
    );
}

// ─────────────────────────────────────── configChanged / error ──

/// Source: P `actionwebsocket.py:646-648` and J `ActionWebSocket.java:214-217` (the
/// frame has `type` only); G `lib/action_api/invoker.ex:239-240`.
#[test]
fn config_changed_is_type_only() {
    pin(&ActionWsFrame::ConfigChanged, r#"{"type":"configChanged"}"#);
}

/// Source: G `lib/graph_conn/test/mock_socket.ex:249-255` (Elixir map as JSON); the
/// message text is `"invalid action message #{inspect(msg)}"`, and
/// `test/graph_conn/mock_socket_test.exs:27-32` matches its prefix
/// `invalid action message`, so that prefix is the message used here.
#[test]
fn typed_error_roundtrips() {
    let frame = ActionWsFrame::from(InboundError {
        code: 400,
        message: "invalid action message".to_owned(),
    });
    pin(
        &frame,
        r#"{"type":"error","code":400,"message":"invalid action message"}"#,
    );
}

// ─────────────────────────────────────────── hello / clientHello ──

/// Source: G `test/gun_test.exs:27` (`{"type":"hello"}`, verbatim) and
/// `lib/graph_conn/test/mock_socket.ex:139-147` (the same map with `max_frame_bytes`).
#[test]
fn hello_roundtrips_and_ignores_an_unusable_limit() {
    pin(
        &ActionWsFrame::Hello(Hello::default()),
        r#"{"type":"hello"}"#,
    );
    pin(
        &ActionWsFrame::Hello(Hello {
            max_frame_bytes: Some(2_000_000),
        }),
        r#"{"type":"hello","max_frame_bytes":2000000}"#,
    );
    // `ws_connection.ex:352-357`: only a positive integer is a limit.
    for unusable in [r#"0"#, r#"-5"#, r#""big""#, r#"null"#] {
        let wire = format!(r#"{{"type":"hello","max_frame_bytes":{unusable}}}"#);
        assert_eq!(
            decode(&wire),
            Ok(ActionWsFrame::Hello(Hello::default())),
            "{wire}"
        );
    }
}

/// Source: G `test/graph_conn/client_hello_test.exs:32-36` (Elixir map as JSON:
/// `type`, `client {app, version}`, `settings {redelivery}`) and `:16-19`.
#[test]
fn client_hello_roundtrips() {
    pin(
        &ActionWsFrame::ClientHello(ClientHello {
            app: Some("my-app".to_owned()),
            version: Some("1.2.3".to_owned()),
            redelivery: Some(true),
        }),
        r#"{"type":"clientHello","client":{"app":"my-app","version":"1.2.3"},"settings":{"redelivery":true}}"#,
    );
    pin(
        &ActionWsFrame::ClientHello(ClientHello {
            app: Some("my-app".to_owned()),
            version: Some("1.2.3".to_owned()),
            redelivery: None,
        }),
        r#"{"type":"clientHello","client":{"app":"my-app","version":"1.2.3"}}"#,
    );
    assert_eq!(
        decode(r#"{"type":"clientHello","client":{"app":7}}"#),
        Err(WireError::WrongType {
            frame: "clientHello",
            field: "app",
            expected: "a string"
        })
    );
}

// ─────────────────────────────── typeless in-band 429 and 503 ──

/// Pins oracle 24 (a 429 with an id reaches the codec as a refusal carrying that
/// id and the advertised wait). Source: P `tests/unit/test_actionwebsocket_rate_limit.py:25-27`,
/// the frame `json.dumps({"error": {"code": 429, "retryAfterMs": 1500}, "id": "a1"})`.
#[test]
fn in_band_429_python_fixture() {
    let wire = r#"{"error": {"code": 429, "retryAfterMs": 1500}, "id": "a1"}"#;
    let frame = ActionWsFrame::RateLimited(RateLimited {
        id: Some("a1".to_owned()),
        message: None,
        retry_after_ms: Some(1500),
    });
    pin(&frame, wire);
    assert!(json(&encode(&frame)).get("type").is_none(), "no type");
}

/// Pins oracle 24. Source: G `lib/graph_conn/test/mock_socket.ex:302-313` (Elixir
/// map as JSON, with the wait of `test/action_api/invoker_rate_limited_test.exs:14`,
/// 1_986).
#[test]
fn in_band_429_elixir_mock_fixture() {
    let wire =
        r#"{"error":{"code":429,"message":"Rate limit exceeded","retryAfterMs":1986},"id":"ID"}"#;
    let frame = ActionWsFrame::RateLimited(RateLimited {
        id: Some("ID".to_owned()),
        message: Some("Rate limit exceeded".to_owned()),
        retry_after_ms: Some(1986),
    });
    pin(&frame, wire);
}

/// Pins oracle 26: a 429 with no `id`, or a `null` id, decodes with `id: None`
/// (the invoker only logs it). Source: G
/// `test/action_api/invoker_rate_limited_test.exs:153-179` (the two Elixir maps as JSON).
#[test]
fn in_band_429_without_a_usable_id() {
    for wire in [
        r#"{"error":{"code":429,"message":"Rate limit exceeded","retryAfterMs":1986}}"#,
        r#"{"error":{"code":429,"retryAfterMs":1986},"id":null}"#,
    ] {
        let ActionWsFrame::RateLimited(r) = decode(wire).expect("decodes") else {
            panic!("not a 429");
        };
        assert_eq!(r.id, None, "{wire}");
        assert_eq!(r.retry_after_ms(), 1986);
    }
}

/// Pins oracle 27: whatever `retryAfterMs` carries, the usable wait is an integer
/// in 0..=86_400_000. Source: G `test/action_api/invoker_rate_limited_test.exs:181-206`
/// (advertised nil, "1986", -1, 999_999_999_999) and `lib/graph_conn/retry_after.ex:82-91`
/// (a float of at least 1 is truncated); the `1986.0` case is the one the source
/// comment names.
#[test]
fn in_band_429_retry_after_is_sanitised() {
    let cases = [
        (r#"null"#, 0),
        (r#""1986""#, 0),
        (r#"-1"#, 0),
        (r#"999999999999"#, MAX_RETRY_AFTER_MS),
        (r#"1986.0"#, 1986),
        (r#"0.5"#, 0),
        (r#"1986"#, 1986),
    ];
    for (advertised, expected) in cases {
        let wire = format!(r#"{{"error":{{"code":429,"retryAfterMs":{advertised}}},"id":"X"}}"#);
        let ActionWsFrame::RateLimited(r) = decode(&wire).expect("decodes") else {
            panic!("not a 429");
        };
        assert_eq!(r.retry_after_ms(), expected, "advertised {advertised}");
        assert!(r.retry_after_ms() <= MAX_RETRY_AFTER_MS);
    }
}

/// Pins oracle 25. Source: G `lib/graph_conn/test/mock_socket.ex:316-323` (Elixir
/// map as JSON) and `test/action_api/invoker_upstream_unavailable_test.exs:33`.
#[test]
fn in_band_503_elixir_mock_fixture() {
    let wire =
        r#"{"error":{"code":503,"message":"Upstream unavailable","upstream":"action"},"id":"ID"}"#;
    let frame = ActionWsFrame::UpstreamUnavailable(UpstreamUnavailable {
        id: Some("ID".to_owned()),
        message: Some("Upstream unavailable".to_owned()),
        upstream: Some("action".to_owned()),
    });
    pin(&frame, wire);
    assert!(json(&encode(&frame)).get("type").is_none(), "no type");
}

/// Pins oracle 26 for 503. Source: G
/// `test/action_api/invoker_upstream_unavailable_test.exs:45-48` (the two Elixir
/// maps as JSON).
#[test]
fn in_band_503_without_a_usable_id() {
    for wire in [
        r#"{"error":{"code":503,"message":"Upstream unavailable"}}"#,
        r#"{"error":{"code":503,"message":"Upstream unavailable"},"id":null}"#,
    ] {
        let ActionWsFrame::UpstreamUnavailable(u) = decode(wire).expect("decodes") else {
            panic!("not a 503");
        };
        assert_eq!(u.id, None, "{wire}");
        assert_eq!(u.message.as_deref(), Some("Upstream unavailable"));
    }
}

// ──────────────────────────────────────────── refusals ──

/// Pins oracle 54: an unknown `type` is an explicit error value naming the type,
/// never a panic and never silently dropped. The type name `closeConnection` is
/// one HIRO's source mentions and does not handle (G `lib/action_api/handler.ex:140-150`,
/// commented out, shape `"type" => "closeConnection", "reason" => reason`); `nope`
/// is the oracle's own example for assertion 54.
#[test]
fn unknown_type_is_an_explicit_error() {
    assert_eq!(
        decode(r#"{"type":"closeConnection","reason":"bye"}"#),
        Err(WireError::UnknownType("closeConnection".to_owned()))
    );
    assert_eq!(
        decode(r#"{"type":"nope","id":"X"}"#),
        Err(WireError::UnknownType("nope".to_owned()))
    );
    // The type string is exact: case and spacing are not folded.
    assert_eq!(
        decode(r#"{"type":"SubmitAction","id":"X"}"#),
        Err(WireError::UnknownType("SubmitAction".to_owned()))
    );
}

/// Malformed frames each get their own named error.
#[test]
fn malformed_frames_are_refused_with_a_named_error() {
    assert!(matches!(decode("not json"), Err(WireError::NotJson(_))));
    assert!(matches!(decode(""), Err(WireError::NotJson(_))));
    assert_eq!(decode("[1,2]"), Err(WireError::NotAnObject));
    assert_eq!(decode("42"), Err(WireError::NotAnObject));
    assert_eq!(decode(r#"{"id":"X"}"#), Err(WireError::MissingType));
    assert_eq!(
        decode(r#"{"error":"x","id":"X"}"#),
        Err(WireError::MissingType)
    );
    assert_eq!(decode(r#"{"type":7}"#), Err(WireError::TypeNotAString));
    assert_eq!(decode(r#"{"type":null}"#), Err(WireError::TypeNotAString));
    assert_eq!(
        decode(r#"{"error":{"code":500,"message":"boom"},"id":"X"}"#),
        Err(WireError::UnknownTypelessError(Some(500)))
    );
    assert_eq!(
        decode(r#"{"error":{"message":"boom"}}"#),
        Err(WireError::UnknownTypelessError(None))
    );
    // Display names the problem.
    let shown = decode(r#"{"type":"nope"}"#).unwrap_err().to_string();
    assert!(shown.contains("nope"), "{shown}");
}

/// Every frame type survives encode then decode unchanged. Pins the envelope of
/// section 1 for all ten frames in one place.
#[test]
fn every_frame_roundtrips_through_text() {
    let frames = [
        ActionWsFrame::Hello(Hello {
            max_frame_bytes: Some(1_000_000),
        }),
        ActionWsFrame::ClientHello(ClientHello {
            app: Some("a".to_owned()),
            version: None,
            redelivery: Some(false),
        }),
        ActionWsFrame::SubmitAction(SubmitAction {
            id: "id1".to_owned(),
            capability: "ExecuteCommand".to_owned(),
            handler: "h".to_owned(),
            scope: None,
            parameters: vec![
                ("a".to_owned(), "1".to_owned()),
                ("b".to_owned(), "quote \" and \\ and\nnewline".to_owned()),
            ],
            timeout_millis: Some(5),
        }),
        ActionWsFrame::Acknowledged(AckFrame {
            id: "x".to_owned(),
            code: Some(200),
            message: Some(String::new()),
        }),
        ActionWsFrame::NegativeAcknowledged(NegativeAcknowledged {
            id: "x".to_owned(),
            code: 404,
            message: "m".to_owned(),
        }),
        ActionWsFrame::SendActionResult(SendActionResult {
            id: "x".to_owned(),
            result: r#"{"output":"hi","exitcode":"0"}"#.to_owned(),
        }),
        ActionWsFrame::ConfigChanged,
        ActionWsFrame::Error(InboundError {
            code: 500,
            message: "e".to_owned(),
        }),
        ActionWsFrame::RateLimited(RateLimited {
            id: Some("x".to_owned()),
            message: None,
            retry_after_ms: Some(10),
        }),
        ActionWsFrame::UpstreamUnavailable(UpstreamUnavailable {
            id: None,
            message: None,
            upstream: Some("action".to_owned()),
        }),
    ];
    for f in frames {
        let text = encode(&f);
        assert_eq!(decode(&text).as_ref(), Ok(&f), "{text}");
    }
}
