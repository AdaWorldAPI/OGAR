//! What the Python and Java clients send, against the OGAR handler: oracle 46-58
//! and section 8.
//!
//! OGAR keeps the Elixir behaviour (`G/lib/action_api/handler.ex`). It must also
//! take, without failing, the frames the Python and Java clients write, or that
//! their documentation shows. Each test names the client source of its frame and
//! states what the handler does with it. `G` is the Elixir client, `P` the Python
//! and `J` the Java one, siblings under
//! `/home/user/bardioc/extraction/corpus/`; the classes are
//! `P/src/hiro_graph_client/actionwebsocket.py` and
//! `J/src/main/java/co/arago/hiro/client/{model/websocket/action/impl,websocket}/`.
//!
//! Differences on the invoker's inbound path (a result wrapped as
//! `{"message","code","data"}`, oracle 49, 57 and 58) are not here: another change
//! handles the invoker, and the handler never reads a result.

use ogar_action_handler::session::{Effect, Session};
use ogar_action_handler::submit_state::{Config, ExecResult};
use ogar_action_handler::wire::WireError;
use serde_json::{Value, json};

fn parse(text: &str) -> Value {
    serde_json::from_str(text).expect("JSON")
}

fn session() -> Session {
    Session::new(Config::default()).unwrap()
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

fn ack(id: &str) -> Value {
    json!({"type": "acknowledged", "id": id, "code": 200, "message": ""})
}

/// A session with `id` submitted at 0 and its result finished at 10: the result
/// is sent at 10 and is next due at 3010.
fn session_with_pending_result(id: &str) -> Session {
    let mut s = session();
    let frame = json!({
        "type": "submitAction", "id": id, "handler": "h",
        "capability": "ExecuteCommand", "parameters": {},
    })
    .to_string();
    s.on_text(0, &frame).unwrap();
    s.on_finished(10, id, ExecResult::Success("{}".into()));
    assert_eq!(s.next_wakeup(), Some(3_010));
    s
}

fn resend_due(s: &mut Session, at: u64, id: &str) {
    let fx = s.on_tick(at);
    let frames = sent(&fx);
    assert_eq!(frames.len(), 1, "a resend at {at}");
    assert_eq!(frames[0]["type"], "sendActionResult");
    assert_eq!(frames[0]["id"], id);
}

// ─────────────────────────────────────────────── the submit it is sent ──

/// Java's own javadoc of the submit, verbatim (`J/.../impl/ActionHandlerSubmit.java:14-22`).
/// The handler acks it, runs it once, and its deadline is the frame's 30000 ms.
#[test]
fn java_javadoc_submit_is_acked_run_and_given_30000ms() {
    let frame = r#"{"type": "submitAction", "id": "id1", "handler": "handlerId", "capability": "ExecuteCommand", "parameters": {}, "timeout": 30000}"#;
    let mut s = session();
    let fx = s.on_text(0, frame).unwrap();
    assert_eq!(sent(&fx), vec![ack("id1")]);
    assert_eq!(runs(&fx), 1);
    assert_eq!(s.next_wakeup(), Some(30_000));
}

/// Oracle 55, 47 and 48 for the field set: Java reads a submit with no `timeout`
/// as 5 minutes (`ActionHandlerSubmit.java:34`, `:44`) and Python's parser fails on
/// it (`actionwebsocket.py:131`). The OGAR handler is the Elixir one: the deadline
/// is 60000 ms (`handler.ex:133`, `:361-362`), not 300000, and the frame is not
/// refused.
#[test]
fn submit_without_timeout_is_60000ms_not_javas_300000ms() {
    let frame = r#"{"type":"submitAction","id":"id1","handler":"handlerId","capability":"ExecuteCommand","parameters":{}}"#;
    let mut s = session();
    let fx = s.on_text(0, frame).unwrap();
    assert_eq!(runs(&fx), 1);
    assert_eq!(s.next_wakeup(), Some(60_000));
    assert_ne!(s.next_wakeup(), Some(300_000));
}

/// A submit with fields no HIRO client reads. Jackson on the Java side is not told
/// to fail on unknown properties in the classes read (UNVERIFIED), and the Python
/// parser uses `dict.get`, which ignores them; the Elixir handler matches the keys
/// it needs and ignores the rest (`handler.ex:121-130`). OGAR decodes the keys it
/// needs and ignores the rest, in the frame and in its `parameters` neighbours.
#[test]
fn submit_with_unknown_fields_is_accepted_and_they_are_ignored() {
    let frame = json!({
        "type": "submitAction", "id": "id-x", "handler": "h",
        "capability": "ExecuteCommand", "parameters": {"command": "ls"},
        "timeout": 5000,
        "scope": "tenant-A", "ticket": 17, "futureField": {"nested": [1, 2]}, "extra": null,
    })
    .to_string();
    let mut s = session();
    let fx = s.on_text(0, &frame).unwrap();
    assert_eq!(sent(&fx), vec![ack("id-x")]);
    assert_eq!(runs(&fx), 1);
    assert_eq!(
        fx[1],
        Effect::Run {
            id: "id-x".into(),
            capability: "ExecuteCommand".into(),
            parameters: vec![("command".into(), "ls".into())],
        }
    );
}

/// Java reads `parameters` as `Map<String, Object>` (`ActionHandlerSubmit.java:30`)
/// and Python as a `dict` (`actionwebsocket.py:101`): a value need not be a string.
/// OGAR keeps a string verbatim and any other value as its compact JSON text.
#[test]
fn submit_parameters_that_are_not_strings_are_kept_as_compact_json() {
    let frame = r#"{"type":"submitAction","id":"id-p","handler":"h","capability":"c","parameters":{"n":60000,"flag":true,"list":[1, 2],"obj":{"a": 1},"s":"text"},"timeout":1}"#;
    let mut s = session();
    let fx = s.on_text(0, frame).unwrap();
    let Effect::Run { parameters, .. } = &fx[1] else {
        panic!("a run");
    };
    // The order of the pairs is the JSON library's, not part of the contract.
    let mut parameters = parameters.clone();
    parameters.sort();
    assert_eq!(
        parameters,
        vec![
            ("flag".to_owned(), "true".to_owned()),
            ("list".to_owned(), "[1,2]".to_owned()),
            ("n".to_owned(), "60000".to_owned()),
            ("obj".to_owned(), r#"{"a":1}"#.to_owned()),
            ("s".to_owned(), "text".to_owned()),
        ]
    );
}

/// Oracle 47 and 48. Python and Java ack a duplicate submit and then ignore it, or
/// send the stored result (`actionwebsocket.py:607-617`, `ActionWebSocket.java:146-162`).
/// The Elixir handler, and OGAR, ack it every time, run once, and answer from the
/// cache once a result exists.
#[test]
fn a_duplicate_submit_is_acked_each_time_and_runs_once() {
    let frame = r#"{"type":"submitAction","id":"id-d","handler":"h","capability":"c","parameters":{},"timeout":30000}"#;
    let mut s = session();
    let first = s.on_text(0, frame).unwrap();
    let second = s.on_text(5, frame).unwrap();
    assert_eq!(runs(&first), 1);
    assert_eq!(runs(&second), 0);
    assert_eq!(sent(&second), vec![ack("id-d")]);

    s.on_finished(9, "id-d", ExecResult::Success("{}".into()));
    let third = s.on_text(20, frame).unwrap();
    assert_eq!(runs(&third), 0);
    let frames = sent(&third);
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[1]["type"], "sendActionResult");
}

// ───────────────────────────────────────── the acks and nacks it is sent ──

/// Oracle 46 and 52. Python's ack of a submit, built by `str(ActionHandlerAck(...))`
/// (`actionwebsocket.py:605`, `:44-50`): `json.dumps` of `{type, id, code, message}`.
/// Arriving for a result of ours, it is an ack like any other and stops the resend
/// (`handler.ex:109-110`). The text of its `message` is not read.
#[test]
fn python_style_ack_with_a_message_stops_the_resend() {
    let mut s = session_with_pending_result("id-a");
    let frame = r#"{"type": "acknowledged", "id": "id-a", "code": 200, "message": "submitAction acknowledged"}"#;
    assert!(s.on_text(100, frame).unwrap().is_empty());
    assert!(s.on_tick(3_010).is_empty());
}

/// Oracle section 8, `acknowledged` without `code`. Java sets only the `id`
/// (`ActionHandlerAck.java:26-30`, `ActionWebSocket.java:144`); serialised, the
/// absent fields are either missing or `null` (the Jackson setting is UNVERIFIED, so
/// both are tested). The Elixir handler matches `type` and `id` only
/// (`handler.ex:109-110`). OGAR takes both and stops the resend.
#[test]
fn java_style_ack_without_a_code_stops_the_resend() {
    for frame in [
        r#"{"type":"acknowledged","id":"id-j"}"#,
        r#"{"type":"acknowledged","id":"id-j","code":null,"message":null}"#,
        r#"{"type":"acknowledged","id":"id-j","message":"ok"}"#,
    ] {
        let mut s = session_with_pending_result("id-j");
        assert!(s.on_text(100, frame).unwrap().is_empty(), "{frame}");
        assert!(s.on_tick(3_010).is_empty(), "{frame}");
    }
}

/// An `acknowledged` with fields no client defines.
#[test]
fn an_ack_with_unknown_fields_is_an_ack() {
    let mut s = session_with_pending_result("id-u");
    let frame = r#"{"type":"acknowledged","id":"id-u","code":200,"message":"","trace":"abc","extra":{"a":1}}"#;
    assert!(s.on_text(100, frame).unwrap().is_empty());
    assert!(s.on_tick(3_010).is_empty());
}

/// Oracle 51 and 53. What a Python or Java handler sends for a result it did not
/// want: `negativeAcknowledged` 400 `"sendActionResult rejected"`
/// (`actionwebsocket.py:628`, `ActionWebSocket.java:180`). For OGAR it is the NACK
/// of a frame it sent. The Elixir handler treats a NACK as an ack: the resend
/// stops (`handler.ex:112-119`). Python resends up to 4 times and Java resends; OGAR
/// does not, and sends nothing in reply.
#[test]
fn nack_400_sendactionresult_rejected_stops_the_resend_and_is_not_retried() {
    let mut s = session_with_pending_result("id-n");
    let frame = r#"{"type": "negativeAcknowledged", "id": "id-n", "code": 400, "message": "sendActionResult rejected"}"#;
    assert!(s.on_text(100, frame).unwrap().is_empty());
    assert!(s.on_tick(3_010).is_empty());
    assert!(s.on_tick(6_010).is_empty());
    assert!(s.on_tick(60_000).is_empty());
    let ep = s.state().episode("id-n").expect("kept for dedupe");
    assert!(ep.nacked && !ep.acknowledged);
}

/// A NACK that carries neither `code` nor `message` (or only one of them): the
/// Elixir handler matches `type` and `id` only (`handler.ex:112-119`), stops the
/// resend, and so does OGAR. No client sends one (Java's 3-argument constructor and
/// Python's constructor both set `code` and `message`,
/// `ActionHandlerNack.java:170-174`, `actionwebsocket.py:205-219`), but the Elixir
/// handler would take it. This test used to pin the opposite (a `WireError` and the
/// resend continuing, named `a_nack_without_code_or_message_is_a_wire_error_and_the_resend_continues`);
/// it was changed on purpose when the codec made both fields optional.
#[test]
fn a_nack_without_code_or_message_stops_the_resend() {
    for frame in [
        r#"{"type":"negativeAcknowledged","id":"id-m"}"#,
        r#"{"type":"negativeAcknowledged","id":"id-m","code":null,"message":null}"#,
        r#"{"type":"negativeAcknowledged","id":"id-m","code":400}"#,
        r#"{"type":"negativeAcknowledged","id":"id-m","message":"m"}"#,
    ] {
        let mut s = session_with_pending_result("id-m");
        assert!(s.on_text(100, frame).unwrap().is_empty(), "{frame}");
        assert!(s.on_tick(3_010).is_empty(), "{frame}");
        assert!(s.on_tick(60_000).is_empty(), "{frame}");
        let ep = s.state().episode("id-m").expect("kept for dedupe");
        assert!(ep.nacked && !ep.acknowledged, "{frame}");
    }
}

/// A NACK with no `id` (or a `null` one) belongs to no result: the Elixir handler
/// has no clause for the missing key and only warns (`handler.ex:152-159`); with a
/// `null` id the clause matches and finds nothing pending (`responder.ex:124-139`).
/// Either way nothing changes. OGAR returns the codec's error for it and steps no
/// state, so the resend of the pending result continues.
#[test]
fn a_nack_without_an_id_changes_nothing() {
    for frame in [
        r#"{"type":"negativeAcknowledged","code":400,"message":"m"}"#,
        r#"{"type":"negativeAcknowledged","id":null,"code":400,"message":"m"}"#,
        r#"{"type":"negativeAcknowledged"}"#,
    ] {
        let mut s = session_with_pending_result("id-i");
        let err = s.on_text(100, frame).expect_err("refused");
        assert!(
            matches!(
                err,
                WireError::MissingField {
                    frame: "negativeAcknowledged",
                    field: "id"
                }
            ),
            "{frame}"
        );
        resend_due(&mut s, 3_010, "id-i");
    }
}

/// Oracle 56 and 51. Java stops after 4 retries (`ActionWebSocket.java:44`) and
/// Python after 4 (`actionwebsocket.py:358`); the Elixir handler resends every
/// 3000 ms with no give-up (`responder.ex:143-163`), and so does OGAR.
#[test]
fn the_result_is_resent_beyond_four_attempts() {
    let mut s = session_with_pending_result("id-r");
    for i in 1..=12_u64 {
        resend_due(&mut s, 10 + 3_000 * i, "id-r");
    }
}

// ───────────────────────────────────── the frames with no type, and others ──

/// Oracle 50. The in-band 429 for a pending result, the frame of
/// `P/tests/unit/test_actionwebsocket_rate_limit.py:25-27`, which Python answers by
/// resending after `retryAfterMs`. The Elixir handler has no clause for it and
/// logs a warning (`handler.ex:152-159`); OGAR decodes it and does nothing: no
/// frame out, the resend neither stopped nor moved to 1500 ms.
#[test]
fn inband_429_for_a_pending_result_changes_nothing() {
    let mut s = session_with_pending_result("id-429");
    let frame = r#"{"error": {"code": 429, "retryAfterMs": 1500}, "id": "id-429"}"#;
    assert!(s.on_text(100, frame).unwrap().is_empty());
    assert_eq!(s.next_wakeup(), Some(3_010));
    assert!(s.on_tick(1_600).is_empty(), "not resent after retryAfterMs");
    resend_due(&mut s, 3_010, "id-429");
}

/// A 429 with no `id`, or a `null` one (oracle 26, `G/test/graph_conn/mock_socket.ex:302-313`).
#[test]
fn inband_429_without_an_id_is_ignored() {
    let mut s = session_with_pending_result("id-429b");
    for frame in [
        r#"{"error":{"code":429,"retryAfterMs":1986}}"#,
        r#"{"error":{"code":429,"message":"Rate limit exceeded","retryAfterMs":1986},"id":null}"#,
    ] {
        assert!(s.on_text(100, frame).unwrap().is_empty(), "{frame}");
    }
    assert_eq!(s.next_wakeup(), Some(3_010));
}

/// Oracle section 8, the in-band 503. Python (typeless, so unknown type) and Java
/// (a `NullPointerException` by reading) both fail on it; the Elixir handler logs a
/// warning. OGAR decodes it and does nothing, for an id that is executing and for
/// one whose result is pending, and sends no NACK. The frame is the mock's,
/// `G/lib/graph_conn/test/mock_socket.ex:316-323`.
#[test]
fn inband_503_with_an_id_is_ignored_and_not_nacked() {
    let frame = r#"{"error":{"code":503,"message":"Upstream unavailable","upstream":"action"},"id":"id-503"}"#;

    let mut s = session_with_pending_result("id-503");
    assert!(s.on_text(100, frame).unwrap().is_empty());
    resend_due(&mut s, 3_010, "id-503");

    let mut s = session();
    let submit = r#"{"type":"submitAction","id":"id-503","handler":"h","capability":"c","parameters":{},"timeout":500}"#;
    s.on_text(0, submit).unwrap();
    assert!(s.on_text(100, frame).unwrap().is_empty());
    assert_eq!(s.next_wakeup(), Some(500), "the deadline is unchanged");
}

/// Oracle 53. A `sendActionResult` arriving at a handler: Python and Java reply
/// `negativeAcknowledged` 400 (`actionwebsocket.py:625-628`,
/// `ActionWebSocket.java:174-181`). The Elixir handler has no clause and warns
/// (`handler.ex:152-159`); OGAR decodes it and sends nothing.
#[test]
fn a_send_action_result_is_ignored_with_no_nack() {
    let mut s = session();
    let frame = r#"{"type": "sendActionResult", "id": "id-s", "result": "{}"}"#;
    assert!(s.on_text(0, frame).unwrap().is_empty());
    assert_eq!(s.state().tracked(), 0);
}

/// Oracle 54. `{"type":"nope","id":X}`: Python replies NACK 400 with the id
/// (`actionwebsocket.py:320-323`, `:658-661`) and Java throws a
/// `WebSocketException`. The Elixir handler warns and replies nothing; OGAR returns
/// the [`WireError`] to the caller and sends nothing.
#[test]
fn an_unknown_type_with_an_id_is_a_wire_error_and_gets_no_reply() {
    let mut s = session();
    let err = s
        .on_text(0, r#"{"type":"nope","id":"id-q"}"#)
        .expect_err("unknown type");
    assert_eq!(err, WireError::UnknownType("nope".to_owned()));
    assert_eq!(s.state().tracked(), 0);
}

/// Oracle section 8, `configChanged` and the typed `error`: Python and Java call
/// their listener or log; the Elixir handler warns (`handler.ex:152-159`). OGAR
/// decodes both and does nothing.
#[test]
fn config_changed_and_a_typed_error_are_ignored() {
    let mut s = session_with_pending_result("id-c");
    for frame in [
        r#"{"type":"configChanged"}"#,
        r#"{"type":"error","code":400,"message":"invalid action message"}"#,
    ] {
        assert!(s.on_text(100, frame).unwrap().is_empty(), "{frame}");
    }
    resend_due(&mut s, 3_010, "id-c");
}

/// A `hello` and a `clientHello` with fields of their own are ignored for
/// everything but the frame limit.
#[test]
fn hello_with_unknown_fields_is_accepted() {
    let mut s = session();
    let fx = s
        .on_text(
            0,
            r#"{"type":"hello","max_frame_bytes":2000000,"server":"gw","features":["a"]}"#,
        )
        .unwrap();
    assert!(fx.is_empty());
    assert_eq!(s.state().max_frame_bytes(), 2_000_000);
}
