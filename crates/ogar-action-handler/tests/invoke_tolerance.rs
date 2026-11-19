//! What the invoker completes with when the handler on the other end is a Python
//! or a Java client rather than graph_conn's Elixir handler (oracle section 8;
//! assertions 49, 57 and 58). The Rust invoker keeps the Elixir behaviour: it
//! decodes the `result` string once (`invoker.ex:742-754`) and completes
//! `{:ok, decoded}` unless the decoded object has an `error` key
//! (`invoker.ex:730-734`). The Python and Java wrappers (`code`, `message`,
//! `data`) have no `error` key, so every one of them, a 500 included, completes
//! as a success carrying the wrapper object. The invoker does not unwrap
//! `data`, does not decode it a second time and does not read `code`.
//!
//! Each frame below is built the way the client builds it:
//!
//! * Python, `hiro-client-python/src/hiro_graph_client/actionwebsocket.py`:
//!   `ActionHandlerResult.stringify_result` (`:172-177`) is
//!   `json.dumps({"type", "id", "result": json.dumps(self.result)})`, and
//!   `send_action_result` (`:769-780`) builds the wrapper
//!   `{"message", "code", "data"}` with `data` the `json.dumps` of a dict
//!   result. Python's `json.dumps` writes `", "` and `": "`.
//! * Java, `hiro-client-java/src/main/java/co/arago/hiro/client/websocket/ActionWebSocket.java`:
//!   `ResultParams.toResultPayload` (`:351-357`) is the Jackson text of
//!   `{code, message, data}` with null fields skipped, `getCode`
//!   (`:324-326`) and `getMessage` (`:333-335`) supply the defaults, and
//!   `ActionHandlerResult` carries it as the `result` string
//!   (`.../impl/ActionHandlerResult.java:11-14`).

use ogar_action_handler::invoke_session::{Effect, InvokeSession};
use ogar_action_handler::invoke_state::{CallRequest, Completion};
use serde_json::{Value, json};

/// Start a call, ack it, deliver `frame` (built with the call's id) and return
/// what the call completed with.
fn completes_with(frame: impl FnOnce(&str) -> String) -> Completion {
    let mut s = InvokeSession::new();
    let fx = s.call(
        0,
        CallRequest::new(1, "ticket", "handler", "ExecuteCommand"),
    );
    let Effect::Send { text, .. } = &fx[0] else {
        panic!("{fx:?}")
    };
    let id = serde_json::from_str::<Value>(text).expect("json")["id"]
        .as_str()
        .expect("id")
        .to_owned();
    s.on_text(1, &json!({"type": "acknowledged", "id": id}).to_string())
        .expect("ack");
    let fx = s.on_text(2, &frame(&id)).expect("the frame decodes");
    assert!(
        matches!(fx[0], Effect::Send { .. }),
        "the result is acked first"
    );
    let Some(Effect::Complete(c)) = fx.into_iter().find(|e| matches!(e, Effect::Complete(_)))
    else {
        panic!("no completion")
    };
    c
}

/// A frame as Python sends it: `inner` is what `json.dumps(result_params)`
/// produced, and the frame is `json.dumps` of the envelope around it
/// (`actionwebsocket.py:172-177`). `serde_json::to_string(inner)` is the outer
/// `json.dumps(self.result)` of a string.
fn python_frame(id: &str, inner: &str) -> String {
    format!(
        r#"{{"type": "sendActionResult", "id": "{id}", "result": {}}}"#,
        serde_json::to_string(inner).expect("a string")
    )
}

/// A frame as Java sends it: Jackson writes `type`, `id`, `result`
/// (`ActionHandlerResult.java`), `result` being the payload text.
fn java_frame(id: &str, payload: &str) -> String {
    format!(
        r#"{{"type":"sendActionResult","id":"{id}","result":{}}}"#,
        serde_json::to_string(payload).expect("a string")
    )
}

/// Oracle section 8 (`actionwebsocket.py:769-780`): a Python handler that returns a dict
/// sends `{"message": "Action successful", "code": 200, "data": "<json of the
/// dict>"}`. The invoker completes `Ok` with that wrapper object. `data` stays
/// the string Python made of it (it is decoded once, not twice).
#[test]
fn python_result_with_data_completes_ok_with_the_wrapper() {
    let c = completes_with(|id| {
        python_frame(
            id,
            r#"{"message": "Action successful", "code": 200, "data": "{\"x\": 1}"}"#,
        )
    });
    assert_eq!(
        c.result,
        Ok(json!({
            "message": "Action successful",
            "code": 200,
            "data": "{\"x\": 1}"
        }))
    );
}

/// Oracle section 8 (`actionwebsocket.py:770-774`): a Python result with nothing in it is
/// `{"message": "Action successful (no data)", "code": 204}`; the invoker
/// completes `Ok` with it.
#[test]
fn python_result_without_data_completes_ok_with_code_204() {
    let c = completes_with(|id| {
        python_frame(
            id,
            r#"{"message": "Action successful (no data)", "code": 204}"#,
        )
    });
    assert_eq!(
        c.result,
        Ok(json!({"message": "Action successful (no data)", "code": 204}))
    );
}

/// Oracle 49 (`actionwebsocket.py:619-623`, `:770-774`): when `on_submit_action`
/// raises, the Python handler sends `send_action_result(result=None, code=500,
/// message=str(err))`, which is `{"message": "<err>", "code": 500}`. The
/// invoker completes `Ok` with that object, NOT an error: the object has no
/// `error` key, and the invoker does not read `code`. A caller that wants to see
/// the 500 reads `code` itself.
#[test]
fn python_500_from_a_handler_exception_completes_ok_not_as_an_error() {
    let c = completes_with(|id| python_frame(id, r#"{"message": "boom", "code": 500}"#));
    assert_eq!(c.result, Ok(json!({"message": "boom", "code": 500})));
}

/// Oracle section 8 (`ActionWebSocket.java:351-357`): a Java handler with data sends
/// `{"code":200,"message":"Action successful","data":"<text>"}`. The invoker
/// completes `Ok` with the wrapper; `data` stays a string.
#[test]
fn java_result_with_data_completes_ok_with_the_wrapper() {
    let c = completes_with(|id| {
        java_frame(
            id,
            r#"{"code":200,"message":"Action successful","data":"{\"x\":1}"}"#,
        )
    });
    assert_eq!(
        c.result,
        Ok(json!({"code": 200, "message": "Action successful", "data": "{\"x\":1}"}))
    );
}

/// Oracle 57 (`ActionWebSocket.java:164-170`): when `onActionSubmit` throws, the
/// Java handler sends `new ResultParams().setCode(500).setMessage(e.getMessage())`
/// as the result: `{"code":500,"message":"<message>"}`. The invoker completes
/// `Ok` with that object, as for Python: no `error` key, so not an error.
#[test]
fn java_500_from_a_listener_exception_completes_ok_not_as_an_error() {
    let c = completes_with(|id| java_frame(id, r#"{"code":500,"message":"listener failed"}"#));
    assert_eq!(
        c.result,
        Ok(json!({"code": 500, "message": "listener failed"}))
    );
}

/// Oracle 58 (`ActionWebSocket.java:324-335`): Java's result defaults. With no
/// code and no data the code is 204 and the message "Action successful (no
/// data)"; with data and no code, 200 and "Action successful"; a null
/// `getMessage()` on the exception (`:168`) takes the message default too, so a
/// 500 from an exception with no message reads "Action successful (no data)".
/// The payloads are the Jackson text of the getters with `data` skipped when
/// null (`:351-357`). The invoker completes `Ok` with each object as it is.
#[test]
fn java_result_defaults_pass_through() {
    let cases = [
        // `newResultParams()`: no code, no message, no data.
        (
            r#"{"code":204,"message":"Action successful (no data)"}"#,
            json!({"code": 204, "message": "Action successful (no data)"}),
        ),
        // `setData("...")` only.
        (
            r#"{"code":200,"message":"Action successful","data":"done"}"#,
            json!({"code": 200, "message": "Action successful", "data": "done"}),
        ),
        // `setCode(500).setMessage(null)`: the exception had no message.
        (
            r#"{"code":500,"message":"Action successful (no data)"}"#,
            json!({"code": 500, "message": "Action successful (no data)"}),
        ),
    ];
    for (payload, want) in cases {
        let c = completes_with(|id| java_frame(id, payload));
        assert_eq!(c.result, Ok(want), "{payload}");
    }
}

/// The contrast the tolerance tests rest on: only an `error` key makes the result
/// an error (`invoker.ex:730-731`), so a client that wants the invoker to see a
/// failure has to send one, as the Elixir handler does (oracle 5c).
#[test]
fn only_an_error_key_makes_a_result_an_error() {
    let c = completes_with(|id| {
        java_frame(id, r#"{"error":{"code":500,"message":"listener failed"}}"#)
    });
    assert!(c.result.is_err());
}
