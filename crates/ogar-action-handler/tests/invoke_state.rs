//! The invoker state machine (`ogar_action_handler::invoke_state`) against the
//! HIRO parity oracle (`HIRO_ORACLE.md`, sections 4 to 7, assertions 20 to 41).
//! The reference is graph_conn: `I` is `lib/action_api/invoker.ex`, `R` is
//! `lib/action_api/request.ex`, `G` is `lib/action_api/invoker/request_registry.ex`
//! and `T` is `test/action_api`. Time is injected; nothing here reads a clock.
//!
//! Every test name starts with the assertion it pins (`a20_...`).

use std::collections::BTreeMap;

use ogar_action_handler::invoke_state::{
    CallId, CallRequest, Completion, DEFAULT_ACK_TIMEOUT_MS, DEFAULT_REQUEST_TIMEOUT_MS, Frame,
    Input, InvokeError, InvokeState, Log, NotSent, Output, RESPONSE_GRACE_MS, SEND_ATTEMPTS,
    SendFailure, SubmitFrame, TimeoutArg, TimeoutSource, request_id,
};
use ogar_action_handler::submit_state::Millis;
use ogar_action_handler::wire::RateLimited;
use serde_json::{Map, Value, json};

// ------------------------------------------------------------- helpers --

fn req(call: CallId) -> CallRequest {
    CallRequest::new(call, "ticket-1", "handler-1", "ExecuteCommand").param("command", json!("ls"))
}

fn submits(out: &[Output]) -> Vec<&SubmitFrame> {
    out.iter()
        .filter_map(|o| match o {
            Output::Send {
                frame: Frame::Submit(s),
                ..
            } => Some(s),
            _ => None,
        })
        .collect()
}

fn completions(out: &[Output]) -> Vec<&Completion> {
    out.iter()
        .filter_map(|o| match o {
            Output::Complete(c) => Some(c),
            _ => None,
        })
        .collect()
}

fn logs(out: &[Output]) -> Vec<&Log> {
    out.iter()
        .filter_map(|o| match o {
            Output::Log(l) => Some(l),
            _ => None,
        })
        .collect()
}

fn ack(id: &str) -> Input {
    Input::Ack { id: id.to_owned() }
}

fn result(id: &str, text: &str) -> Input {
    Input::Result {
        id: id.to_owned(),
        result: Some(text.to_owned()),
    }
}

fn limited(id: Option<&str>, retry_after_ms: Option<i64>) -> Input {
    Input::RateLimited(RateLimited {
        id: id.map(str::to_owned),
        message: None,
        retry_after_ms,
    })
}

fn unavailable(id: Option<&str>) -> Input {
    let mut error = Map::new();
    error.insert("code".to_owned(), json!(503));
    error.insert("message".to_owned(), json!("Upstream unavailable"));
    error.insert("upstream".to_owned(), json!("action"));
    Input::Unavailable {
        id: id.map(str::to_owned),
        error,
    }
}

/// Start `request` at `now` and return its request id and the outputs.
fn start(st: &mut InvokeState, now: Millis, request: CallRequest) -> (String, Vec<Output>) {
    let out = st.step(now, Input::Call(request));
    let id = submits(&out)
        .first()
        .expect("the first send is made at once")
        .id
        .clone();
    (id, out)
}

fn only_completion(out: &[Output]) -> &Completion {
    let c = completions(out);
    assert_eq!(c.len(), 1, "{out:?}");
    c[0]
}

fn tick(st: &mut InvokeState, now: Millis) -> Vec<Output> {
    st.step(now, Input::Tick)
}

// -------------------------------------------------------- 20, 21: acks --

/// Oracle 20 (`I:109`, `:442`, `:666`): with no `ack_timeout` option each send
/// waits 3000 ms for the ack. One millisecond early nothing happens; at 3000 the
/// wait is logged and the request is sent again.
#[test]
fn a20_default_ack_wait_is_3000ms() {
    assert_eq!(DEFAULT_ACK_TIMEOUT_MS, 3_000);
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 5, req(1));
    assert_eq!(st.next_wakeup(), Some(3_005));
    assert!(tick(&mut st, 3_004).is_empty());
    let out = tick(&mut st, 3_005);
    assert_eq!(
        logs(&out),
        vec![&Log::AckTimeout {
            request_id: id,
            ack_timeout_ms: 3_000
        }]
    );
    assert_eq!(submits(&out).len(), 1, "sent again");
}

/// Oracle 20 (`I:442`): the `ack_timeout` option changes the wait.
#[test]
fn a20_ack_timeout_option_changes_the_wait() {
    let mut st = InvokeState::new();
    start(&mut st, 0, req(1).with_ack_timeout(70));
    assert_eq!(st.next_wakeup(), Some(70));
    assert!(tick(&mut st, 69).is_empty());
    assert_eq!(submits(&tick(&mut st, 70)).len(), 1);
}

/// Oracle 21 (`I:110`, `:582-590`, `:587`): with no ack ever, three sends, then
/// `{:ack_timeout, 9000}` (`ack_timeout * 3`). The sends are one ack wait apart.
#[test]
fn a21_three_sends_then_ack_timeout_of_9000() {
    assert_eq!(SEND_ATTEMPTS, 3);
    let mut st = InvokeState::new();
    let mut sends = 0;
    let (_, out) = start(&mut st, 0, req(1));
    sends += submits(&out).len();
    for t in [3_000, 6_000] {
        let out = tick(&mut st, t);
        sends += submits(&out).len();
        assert!(completions(&out).is_empty());
    }
    let out = tick(&mut st, 9_000);
    sends += submits(&out).len();
    assert_eq!(sends, 3, "three sends, no fourth");
    let c = only_completion(&out);
    assert_eq!(c.call, 1);
    assert_eq!(c.result, Err(InvokeError::AckTimeout(9_000)));
    assert_eq!(st.tracked(), 0);
    assert_eq!(st.next_wakeup(), None);
}

/// Oracle 21 with the option: the error value is the option times three.
#[test]
fn a21_ack_timeout_error_follows_the_option() {
    let mut st = InvokeState::new();
    start(&mut st, 0, req(1).with_ack_timeout(70));
    tick(&mut st, 70);
    tick(&mut st, 140);
    let out = tick(&mut st, 210);
    assert_eq!(
        only_completion(&out).result,
        Err(InvokeError::AckTimeout(210))
    );
}

// ------------------------------------------------ 22, 23: response wait --

/// Oracle 22 (`I:737-738`): after the ack the wait is the request timeout plus
/// 1000 ms, counted from the ack.
#[test]
fn a22_response_wait_is_request_timeout_plus_1000() {
    assert_eq!(RESPONSE_GRACE_MS, 1_000);
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1).with_timeout(TimeoutArg::Int(5)));
    st.step(100, ack(&id));
    assert_eq!(st.next_wakeup(), Some(100 + 5_000 + 1_000));
    assert!(tick(&mut st, 6_099).is_empty());
    let out = tick(&mut st, 6_100);
    assert_eq!(logs(&out), vec![&Log::SendingLastCall { request_id: id }]);
}

/// Oracle 23 (`I:694-714`; `T/invoker_test.exs:260-268` expects
/// `{:exec_timeout, 1_000}` for `timeout: 1`): the first response timeout sends
/// the request once more; a second one is `{:exec_timeout, T}`.
#[test]
fn a23_one_last_call_then_exec_timeout() {
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1).with_timeout(TimeoutArg::Int(1)));
    st.step(10, ack(&id));
    // 10 + 1000 + 1000.
    let out = tick(&mut st, 2_010);
    assert_eq!(submits(&out).len(), 1, "the last call is sent");
    assert!(completions(&out).is_empty());
    st.step(2_020, ack(&id));
    assert!(tick(&mut st, 4_019).is_empty());
    let out = tick(&mut st, 4_020);
    assert_eq!(logs(&out), vec![&Log::ResponseTimeout { request_id: id }]);
    let c = only_completion(&out);
    assert_eq!(c.result, Err(InvokeError::ExecTimeout(1_000)));
    assert_eq!(st.tracked(), 0);
}

/// Oracle 23 (`I:713`): the last call "again allows 3 send attempts": a full ack
/// cycle. With no ack it is sent three times and then the ack-timeout error, not
/// the exec-timeout error.
#[test]
fn a23_the_last_call_has_a_full_ack_cycle() {
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1).with_timeout(TimeoutArg::Int(1)));
    st.step(0, ack(&id));
    let mut sends = 0;
    let out = tick(&mut st, 2_000);
    sends += submits(&out).len();
    for t in [5_000, 8_000] {
        sends += submits(&tick(&mut st, t)).len();
    }
    assert_eq!(sends, 3);
    let out = tick(&mut st, 11_000);
    assert!(submits(&out).is_empty());
    assert_eq!(
        only_completion(&out).result,
        Err(InvokeError::AckTimeout(9_000))
    );
}

// ------------------------------------------------ 24 to 27: in-band 4xx --

/// Oracle 24 (`I:246-258`, `:672-684`; `T/invoker_rate_limited_test.exs:122-137`):
/// an in-band 429 with the request's id completes the call at once with the
/// sanitised wait, and the request is not sent again.
#[test]
fn a24_429_with_id_completes_at_once_and_is_not_resent() {
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1));
    let out = st.step(10, limited(Some(&id), Some(1_986)));
    let c = only_completion(&out);
    assert_eq!(c.request_id, id);
    assert_eq!(c.result, Err(InvokeError::RateLimited(1_986)));
    assert!(submits(&out).is_empty());
    assert_eq!(st.tracked(), 0);
    // Nothing more, whenever the clock goes.
    assert!(tick(&mut st, 100_000).is_empty());
}

/// Oracle 25 (`I:260-268`, `:676-679`; `T/invoker_upstream_unavailable_test.exs:26-38`):
/// an in-band 503 with the id completes the call with the frame's own error map.
#[test]
fn a25_503_with_id_completes_at_once_with_the_error_map() {
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1));
    let out = st.step(10, unavailable(Some(&id)));
    let Err(InvokeError::Refused(map)) = &only_completion(&out).result else {
        panic!("{out:?}")
    };
    assert_eq!(
        Value::Object(map.clone()),
        json!({"code": 503, "message": "Upstream unavailable", "upstream": "action"})
    );
    assert!(submits(&out).is_empty());
    assert_eq!(st.tracked(), 0);
}

/// Oracle 26 (`I:272-278`; `T/invoker_rate_limited_test.exs:153-168`): without a
/// usable id a 429 or a 503 is a log marker only. Nobody is answered and the
/// waiting call is untouched.
#[test]
fn a26_429_and_503_without_an_id_are_log_markers_only() {
    let mut st = InvokeState::new();
    start(&mut st, 0, req(1));
    let out = st.step(1, limited(None, Some(1_986)));
    assert_eq!(
        out,
        vec![Output::Log(Log::RateLimitedNoId {
            retry_after_ms: 1_986
        })]
    );
    let out = st.step(2, unavailable(None));
    assert_eq!(out, vec![Output::Log(Log::UpstreamUnavailableNoId)]);
    assert_eq!(st.tracked(), 1, "the call still waits for its ack");
}

/// Oracle 27 (`I:252-256`; `retry_after.ex:82-91`; `T/invoker_rate_limited_test.exs:181-206`):
/// whatever the frame carries as `retryAfterMs`, the caller gets an integer in
/// `0..=86_400_000`. The input here is the decoded `Option<i64>` of
/// [`RateLimited`] (`wire` reads nil and "1986" as none, a float as truncated);
/// the frame-level cases are in `tests/invoke_session.rs`.
#[test]
fn a27_advertised_wait_is_sanitised_into_0_to_a_day() {
    for (advertised, want) in [
        (None, 0),
        (Some(-1), 0),
        (Some(0), 0),
        (Some(1_986), 1_986),
        (Some(86_400_000), 86_400_000),
        (Some(999_999_999_999), 86_400_000),
        (Some(i64::MAX), 86_400_000),
    ] {
        let mut st = InvokeState::new();
        let (id, _) = start(&mut st, 0, req(1));
        let out = st.step(1, limited(Some(&id), advertised));
        assert_eq!(
            only_completion(&out).result,
            Err(InvokeError::RateLimited(want)),
            "advertised {advertised:?}"
        );
    }
}

// --------------------------------------------------- 28 to 32: timeouts --

fn refused_value(arg: TimeoutArg) -> (Vec<Output>, InvokeState) {
    let mut st = InvokeState::new();
    let out = st.step(0, Input::Call(req(1).with_timeout(arg)));
    (out, st)
}

/// Oracle 28 (`I:521-537`, `:545-563`; `T/invoker_test.exs:116-128`): each of
/// these timeouts is refused before anything is sent, with the value as given.
#[test]
fn a28_invalid_timeouts_are_refused_before_anything_is_sent() {
    let cases: Vec<(TimeoutArg, Value)> = vec![
        (TimeoutArg::Float(1.5), json!(1.5)),
        (TimeoutArg::Str("1.5".into()), json!("1.5")),
        (TimeoutArg::Str("abc".into()), json!("abc")),
        (TimeoutArg::Str(String::new()), json!("")),
        (TimeoutArg::Bool(true), json!(true)),
        (TimeoutArg::List(vec![json!(30)]), json!([30])),
        (TimeoutArg::Int(0), json!(0)),
        (TimeoutArg::Str("0".into()), json!("0")),
        (TimeoutArg::Int(-5), json!(-5)),
        (TimeoutArg::Str("-5".into()), json!("-5")),
        // Beyond the oracle's list, from `_timeout_ms/1`: a whole-valued float
        // is not an integer, whitespace is not skipped, an object is not a number.
        (TimeoutArg::Float(30.0), json!(30.0)),
        (TimeoutArg::Str(" 30".into()), json!(" 30")),
        (TimeoutArg::Str("30 ".into()), json!("30 ")),
        (TimeoutArg::Bool(false), json!(false)),
        (TimeoutArg::Object(Map::new()), json!({})),
    ];
    for (arg, shown) in cases {
        let (out, st) = refused_value(arg.clone());
        assert!(submits(&out).is_empty(), "{arg:?}: nothing is sent");
        let c = only_completion(&out);
        assert_eq!(
            c.result,
            Err(InvokeError::NotSent(NotSent::InvalidTimeout(shown.clone()))),
            "{arg:?}"
        );
        assert_eq!(st.tracked(), 0);
        assert_eq!(st.next_wakeup(), None);
        assert!(matches!(
            logs(&out)[..],
            [Log::InvalidTimeoutRefused {
                source: TimeoutSource::Caller,
                ..
            }]
        ));
        // The id is that of the parameters before the timeout was converted
        // (`I:545-553`).
        let mut params = Map::new();
        params.insert("command".to_owned(), json!("ls"));
        params.insert("timeout".to_owned(), shown);
        assert_eq!(
            c.request_id,
            request_id("ticket-1", "handler-1", "ExecuteCommand", &params)
        );
    }
}

/// Oracle 28: an explicit `null` is a timeout the caller gave and is no timeout,
/// not a refusal; an absent one is not a refusal either.
#[test]
fn a28_null_and_absent_timeouts_are_not_refused() {
    for arg in [TimeoutArg::Null, TimeoutArg::Absent] {
        let mut st = InvokeState::new();
        let out = st.step(0, Input::Call(req(1).with_timeout(arg)));
        assert_eq!(submits(&out).len(), 1);
    }
}

/// Oracle 29 (`I:527-531`; `T/invoker_test.exs:106-114`): `"timeout": "30"` is 30
/// seconds, and the wire carries 30000 both as `timeout` and in the parameters.
#[test]
fn a29_string_seconds_are_accepted() {
    for (text, ms) in [("30", 30_000), ("+30", 30_000), ("030", 30_000)] {
        let mut st = InvokeState::new();
        let out = st.step(
            0,
            Input::Call(req(1).with_timeout(TimeoutArg::Str(text.into()))),
        );
        let s = submits(&out)[0];
        assert_eq!(s.timeout_ms, ms, "{text}");
        assert_eq!(s.parameters["timeout"], json!(ms), "{text}");
    }
}

/// Oracle 30 (`I:417-421`; `T/invoker_test.exs:99-104`): the caller's timeout is
/// in SECONDS; `123` is 123000 on the wire.
#[test]
fn a30_caller_timeout_is_seconds_and_the_wire_carries_ms() {
    let mut st = InvokeState::new();
    let mut params = Map::new();
    params.insert("command".to_owned(), json!("ls"));
    params.insert("timeout".to_owned(), json!(123));
    let out = st.step(
        0,
        Input::Call(CallRequest::from_params(
            1,
            "t",
            "h",
            "ExecuteCommand",
            params,
        )),
    );
    let s = submits(&out)[0];
    assert_eq!(s.timeout_ms, 123_000);
    assert_eq!(s.parameters["timeout"], json!(123_000));
    assert_eq!(s.parameters["command"], json!("ls"));
    // And the response wait is that plus a second.
    st.step(0, ack(&s.id));
    assert_eq!(st.next_wakeup(), Some(124_000));
}

/// Oracle 31 (`R:30`, `:42`; `I:451`): with neither the caller nor the
/// capability giving a timeout the request timeout is 60000 ms. The `timeout`
/// option (ms) is used before that. The parameters then carry `"timeout": null`
/// (`I:421`).
#[test]
fn a31_default_request_timeout_is_60000ms() {
    assert_eq!(DEFAULT_REQUEST_TIMEOUT_MS, 60_000);
    let mut st = InvokeState::new();
    let out = st.step(0, Input::Call(req(1)));
    let s = submits(&out)[0];
    assert_eq!(s.timeout_ms, 60_000);
    assert_eq!(s.parameters["timeout"], Value::Null);
    st.step(0, ack(&s.id));
    assert_eq!(st.next_wakeup(), Some(61_000));

    let mut st = InvokeState::new();
    let out = st.step(0, Input::Call(req(1).with_default_timeout(7_000)));
    assert_eq!(submits(&out)[0].timeout_ms, 7_000);
}

/// Oracle 32 (`I:565-569`, `:415-422`): a capability default `timeout` (seconds)
/// is used when the caller gave none, and the caller's value overrides it. The
/// same holds for any other parameter.
#[test]
fn a32_caller_params_override_capability_defaults() {
    let mut st = InvokeState::new();
    let mut defaults = Map::new();
    defaults.insert("timeout".to_owned(), json!(60));
    defaults.insert("command".to_owned(), json!("default-command"));
    defaults.insert("host".to_owned(), json!("localhost"));
    st.set_capability_defaults("ExecuteCommand", defaults);

    // No caller timeout: the default, in seconds.
    let out = st.step(0, Input::Call(req(1)));
    let s = submits(&out)[0];
    assert_eq!(s.timeout_ms, 60_000);
    assert_eq!(s.parameters["timeout"], json!(60_000));
    assert_eq!(s.parameters["command"], json!("ls"), "caller wins");
    assert_eq!(s.parameters["host"], json!("localhost"), "default fills in");

    // A caller timeout: the caller's.
    let out = st.step(
        0,
        Input::Call(req(2).with_timeout(TimeoutArg::Int(5)).param("n", json!(2))),
    );
    assert_eq!(submits(&out)[0].timeout_ms, 5_000);
}

/// Oracle 28 and 32 (`I:539-543`, `T/invoker_timeout_refusal_test.exs:24-38`): a
/// default that is not a valid timeout is refused too, and the marker says it
/// came from the capability defaults.
#[test]
fn a32_an_invalid_default_timeout_is_refused_and_named_as_such() {
    let mut st = InvokeState::new();
    let mut defaults = Map::new();
    defaults.insert("timeout".to_owned(), json!("soon"));
    st.set_capability_defaults("ExecuteCommand", defaults);
    let out = st.step(0, Input::Call(req(1)));
    assert!(submits(&out).is_empty());
    assert_eq!(
        only_completion(&out).result,
        Err(InvokeError::NotSent(NotSent::InvalidTimeout(json!("soon"))))
    );
    assert!(matches!(
        logs(&out)[..],
        [Log::InvalidTimeoutRefused {
            source: TimeoutSource::CapabilityDefaults,
            ..
        }]
    ));
    // The caller's own value is not affected by a bad default.
    let out = st.step(0, Input::Call(req(2).with_timeout(TimeoutArg::Int(3))));
    assert_eq!(submits(&out)[0].timeout_ms, 3_000);
}

// ------------------------------------------------------------ 33: nacks --

/// Oracle 33 (`I:659-660`; `T/invoker_test.exs:137-141`): a NACK 404 is terminal:
/// the caller gets it at once, with no log and no resend.
#[test]
fn a33_nack_404_is_terminal() {
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1));
    let out = st.step(
        10,
        Input::Nack {
            id: id.clone(),
            code: 404,
            message: "capability invalid_capability not found".into(),
        },
    );
    assert_eq!(
        out,
        vec![Output::Complete(Completion {
            call: 1,
            request_id: id,
            result: Err(InvokeError::Nack {
                code: 404,
                message: "capability invalid_capability not found".into()
            }),
        })]
    );
    assert!(tick(&mut st, 100_000).is_empty(), "no resend");
}

/// Oracle 33 and 5a (`I:662-664`): any other NACK code is terminal as well; it is
/// logged as an error, and the caller gets `{:nack, %{code, message}}`.
#[test]
fn a33_other_nack_codes_are_terminal_and_logged() {
    for code in [400, 403, 500] {
        let mut st = InvokeState::new();
        let (id, _) = start(&mut st, 0, req(1));
        let out = st.step(
            10,
            Input::Nack {
                id: id.clone(),
                code,
                message: "Forbidden".into(),
            },
        );
        assert_eq!(
            logs(&out),
            vec![&Log::MessageNacked {
                request_id: id,
                code
            }]
        );
        assert_eq!(
            only_completion(&out).result,
            Err(InvokeError::Nack {
                code,
                message: "Forbidden".into()
            })
        );
        assert_eq!(st.tracked(), 0);
        assert!(tick(&mut st, 100_000).is_empty());
    }
}

// ----------------------------------------------------------- 34, 35, 36 --

/// Oracle 34 (`I:214-237`; `T/invoker_result_test.exs:18-22`; `T/invoker_duplicate_response_test.exs:23`
/// "acked before it delivers"): the result is acked (`code` 200) BEFORE the
/// completion is delivered. The order of the outputs is the ack, then the
/// completion.
#[test]
fn a34_result_is_acked_before_it_is_delivered() {
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1));
    st.step(1, ack(&id));
    let out = st.step(2, result(&id, r#"{"ok":true}"#));
    assert_eq!(out.len(), 2);
    assert_eq!(
        out[0],
        Output::Send {
            call: None,
            frame: Frame::AckResult { id: id.clone() }
        }
    );
    assert_eq!(
        out[1],
        Output::Complete(Completion {
            call: 1,
            request_id: id,
            result: Ok(json!({"ok": true})),
        })
    );
}

/// Oracle 34 (`I:217-236`; `T/invoker_duplicate_response_test.exs:16-36`): a
/// result nobody waits for (a second copy of an answered one) is still acked,
/// and then nothing else happens.
#[test]
fn a34_a_result_nobody_waits_for_is_acked_and_dropped() {
    let mut st = InvokeState::new();
    let out = st.step(0, result("orphan", r#"{"ok":true}"#));
    assert_eq!(
        out,
        vec![Output::Send {
            call: None,
            frame: Frame::AckResult {
                id: "orphan".into()
            }
        }]
    );
    assert_eq!(st.tracked(), 0);
}

fn completes_with(text: Option<&str>) -> Result<Value, InvokeError> {
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1));
    st.step(1, ack(&id));
    let out = st.step(
        2,
        Input::Result {
            id,
            result: text.map(str::to_owned),
        },
    );
    only_completion(&out).result.clone()
}

/// Oracle 35 (`I:742-754`; `T/invoker_result_test.exs:18-40`): the result string
/// is JSON-decoded once. A non-JSON string is returned as the string; a bare
/// number is a number; a missing result is `nil` (`null`).
#[test]
fn a35_result_string_is_decoded_once() {
    assert_eq!(
        completes_with(Some(r#"{"ok":true}"#)),
        Ok(json!({"ok": true}))
    );
    assert_eq!(completes_with(Some("ok")), Ok(json!("ok")));
    assert_eq!(completes_with(Some("42")), Ok(json!(42)));
    assert_eq!(completes_with(None), Ok(Value::Null));
    // Once, not twice: the decoded string is not decoded again.
    assert_eq!(
        completes_with(Some(r#""{\"a\":1}""#)),
        Ok(json!(r#"{"a":1}"#))
    );
    // Not JSON at all, including the empty string.
    assert_eq!(completes_with(Some("")), Ok(json!("")));
    assert_eq!(completes_with(Some("{not json")), Ok(json!("{not json")));
}

/// Oracle 36 (`I:730-731`; `T/invoker_test.exs:143-152`): a decoded object with
/// an `error` key completes as an error carrying that value.
#[test]
fn a36_error_key_in_the_result_is_an_error() {
    assert_eq!(
        completes_with(Some(r#"{"error":{"code":404,"message":"Error message"}}"#)),
        Err(InvokeError::Handler(
            json!({"code": 404, "message": "Error message"})
        ))
    );
    assert_eq!(
        completes_with(Some(
            r#"{"error":"the command does not point to an existing file"}"#
        )),
        Err(InvokeError::Handler(json!(
            "the command does not point to an existing file"
        )))
    );
    // A null error value is still an error (`%{"error" => error}` matches nil).
    assert_eq!(
        completes_with(Some(r#"{"error":null}"#)),
        Err(InvokeError::Handler(Value::Null))
    );
    // Not an object, or no `error` key: success.
    assert_eq!(completes_with(Some(r#"["error"]"#)), Ok(json!(["error"])));
    assert_eq!(
        completes_with(Some(r#"{"errors":1}"#)),
        Ok(json!({"errors": 1}))
    );
}

/// Oracle 36 and 5b (`I:724-728`): the two timeouts a result can carry.
#[test]
fn a36_timeout_errors_in_the_result() {
    assert_eq!(
        completes_with(Some(r#"{"error":"exec_timeout"}"#)),
        Err(InvokeError::HandlerReturnedTimeout(60_000))
    );
    assert_eq!(
        completes_with(Some(
            r#"{"error":"request_timed_out","last_status":"submitted"}"#
        )),
        Err(InvokeError::ActionApiReturnedTimeout(json!("submitted")))
    );
    // Without `last_status` the second clause does not match; it is a plain error.
    assert_eq!(
        completes_with(Some(r#"{"error":"request_timed_out"}"#)),
        Err(InvokeError::Handler(json!("request_timed_out")))
    );
}

// ---------------------------------------------------------------- 37: id --

fn params(pairs: &[(&str, Value)]) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect()
}

/// Oracle 37 (`R:51-58`; `T/request_test.exs:5-155`): the id is a deterministic
/// function of (ticket, handler, capability, params): the same inputs give the
/// same id, and changing any one changes it. It does not depend on the order the
/// parameters were written in.
#[test]
fn a37_request_id_is_deterministic_in_its_four_inputs() {
    let p = params(&[("other_handler", json!("Echo")), ("command", json!("ls"))]);
    let base = request_id("ticket", "handler", "cap", &p);
    assert_eq!(base, request_id("ticket", "handler", "cap", &p));
    assert_eq!(base.len(), 16);
    assert!(base.bytes().all(|b| b.is_ascii_hexdigit()));

    assert_ne!(base, request_id("ticket2", "handler", "cap", &p));
    assert_ne!(base, request_id("ticket", "handler2", "cap", &p));
    assert_ne!(base, request_id("ticket", "handler", "cap2", &p));
    let p2 = params(&[
        ("other_handler", json!("Echo")),
        ("command", json!("ls -l")),
    ]);
    assert_ne!(base, request_id("ticket", "handler", "cap", &p2));

    // Key order does not matter.
    let reordered = params(&[("command", json!("ls")), ("other_handler", json!("Echo"))]);
    assert_eq!(base, request_id("ticket", "handler", "cap", &reordered));
    // The boundaries between the strings matter.
    assert_ne!(
        request_id("ab", "c", "cap", &p),
        request_id("a", "bc", "cap", &p)
    );
    // Types matter: 1 is not "1" is not 1.0 is not true.
    let ids: Vec<String> = [json!(1), json!("1"), json!(1.0), json!(true), json!(null)]
        .iter()
        .map(|v| request_id("t", "h", "c", &params(&[("x", v.clone())])))
        .collect();
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            assert_ne!(ids[i], ids[j]);
        }
    }
    // Nested values are covered too.
    assert_ne!(
        request_id("t", "h", "c", &params(&[("x", json!({"a": [1, 2]}))])),
        request_id("t", "h", "c", &params(&[("x", json!({"a": [2, 1]}))]))
    );
}

/// Oracle 37, as the machine uses it: two calls with the same four inputs send
/// the same id, and any one changed sends another. The timeout is among the
/// parameters (`R:52`, `I:421`).
#[test]
fn a37_calls_derive_their_id_from_ticket_handler_capability_and_params() {
    let id_of = |r: CallRequest| {
        let mut st = InvokeState::new();
        start(&mut st, 0, r).0
    };
    let base = id_of(req(1));
    assert_eq!(base, id_of(req(2)), "the call handle is not an input");
    assert_eq!(
        base,
        id_of(req(1).with_ack_timeout(10)),
        "options are not inputs"
    );
    let mut other_ticket = req(1);
    other_ticket.ticket = "ticket-2".into();
    assert_ne!(base, id_of(other_ticket));
    let mut other_handler = req(1);
    other_handler.handler = "handler-2".into();
    assert_ne!(base, id_of(other_handler));
    let mut other_cap = req(1);
    other_cap.capability = "RunScript".into();
    assert_ne!(base, id_of(other_cap));
    assert_ne!(base, id_of(req(1).param("command", json!("ls -l"))));
    assert_ne!(base, id_of(req(1).with_timeout(TimeoutArg::Int(5))));
    // "30" and 30 are the same timeout and so the same request.
    assert_eq!(
        id_of(req(1).with_timeout(TimeoutArg::Int(30))),
        id_of(req(1).with_timeout(TimeoutArg::Str("30".into())))
    );
}

// ------------------------------------------------------ 38 to 41: sharing --

/// Oracle 38 (`G:153-165`, `local.ex:24-28`; `I:466-469`): two concurrent
/// identical calls share one id; each sends its own request; when the response
/// arrives both complete, and the result is acked once.
#[test]
fn a38_concurrent_identical_calls_share_an_id_and_both_complete() {
    let mut st = InvokeState::new();
    let (id1, out1) = start(&mut st, 0, req(1));
    let (id2, out2) = start(&mut st, 1, req(2));
    assert_eq!(id1, id2);
    assert_eq!(submits(&out1).len() + submits(&out2).len(), 2);
    assert_eq!((st.tracked(), st.tracked_ids()), (2, 1));

    // One ack reaches both.
    st.step(5, ack(&id1));
    let out = st.step(6, result(&id1, r#"{"command":"ls"}"#));
    assert_eq!(
        out.iter()
            .filter(|o| matches!(o, Output::Send { .. }))
            .count(),
        1,
        "acked once"
    );
    let done = completions(&out);
    assert_eq!(done.len(), 2);
    assert_eq!(
        done.iter().map(|c| c.call).collect::<Vec<_>>(),
        vec![1, 2],
        "in registration order"
    );
    for c in done {
        assert_eq!(c.result, Ok(json!({"command": "ls"})));
    }
    assert_eq!((st.tracked(), st.tracked_ids()), (0, 0));
}

/// Oracle 38: calls with different ids are independent (`T/invoker_test.exs:154-192`):
/// one's response does not answer the other.
#[test]
fn a38_calls_with_different_ids_are_independent() {
    let mut st = InvokeState::new();
    let (id1, _) = start(&mut st, 0, req(1).param("n", json!(1)));
    let (id2, _) = start(&mut st, 0, req(2).param("n", json!(2)));
    assert_ne!(id1, id2);
    st.step(1, ack(&id1));
    st.step(1, ack(&id2));
    let out = st.step(2, result(&id2, r#"{"n":2}"#));
    assert_eq!(completions(&out).len(), 1);
    assert_eq!(completions(&out)[0].call, 2);
    assert_eq!(st.tracked(), 1);
}

/// Oracle 39 (`G:49-57`; `T/invoker/request_registry_test.exs:17-21`): an ack for
/// an id with no registered caller is a no-op: no output at all.
#[test]
fn a39_ack_for_an_unknown_id_is_a_no_op() {
    let mut st = InvokeState::new();
    assert!(st.step(0, ack("nobody")).is_empty());
    let (id, _) = start(&mut st, 0, req(1));
    assert!(st.step(1, ack("someone-else")).is_empty());
    // And a NACK for an unknown id too (`G:126-134`).
    assert!(
        st.step(
            1,
            Input::Nack {
                id: "someone-else".into(),
                code: 404,
                message: String::new()
            }
        )
        .is_empty()
    );
    assert_eq!(st.tracked(), 1);
    assert_eq!(st.next_wakeup(), Some(3_000), "its wait was not touched");
    let _ = id;
}

/// Oracle 40 (`I:290-302`, `G:77-90`; `T/invoker_rate_limited_test.exs:109-120`,
/// `T/invoker_upstream_unavailable_test.exs:60-75`): a refusal with an id but no
/// caller is the log marker "found no waiting caller", with the id.
#[test]
fn a40_a_refusal_with_no_caller_is_a_log_marker() {
    let mut st = InvokeState::new();
    assert_eq!(
        st.step(0, limited(Some("gone"), Some(1_986))),
        vec![Output::Log(Log::RefusalNoCaller {
            request_id: "gone".into(),
            code: 429
        })]
    );
    assert_eq!(
        st.step(0, unavailable(Some("gone"))),
        vec![Output::Log(Log::RefusalNoCaller {
            request_id: "gone".into(),
            code: 503
        })]
    );
}

/// Oracle 41 (`I:496-500`; `T/invoker_late_answer_test.exs:28-37`): a refusal
/// that arrives after the first ack wait gave up is held; the next send's flush
/// returns it instead of sending again. Here it is stamped at the deadline, so it
/// is late for that wait. No second submit goes out.
#[test]
fn a41_a_late_refusal_is_used_by_the_next_send_instead_of_resending() {
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1));
    let out = st.step(3_000, limited(Some(&id), Some(1_986)));
    assert!(submits(&out).is_empty(), "no resend into the limiter");
    assert_eq!(
        only_completion(&out).result,
        Err(InvokeError::RateLimited(1_986))
    );
    assert!(
        logs(&out)
            .iter()
            .any(|l| matches!(l, Log::AckTimeout { .. })),
        "the wait did give up"
    );
    assert_eq!(st.tracked(), 0);

    // A 503 too (`T/invoker_late_answer_test.exs:39-44`).
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1));
    let out = st.step(3_000, unavailable(Some(&id)));
    assert!(submits(&out).is_empty());
    assert!(matches!(
        only_completion(&out).result,
        Err(InvokeError::Refused(_))
    ));
}

/// Oracle 41 (`I:582-590`; `T/invoker_late_answer_test.exs:46-51`): after the
/// last send the late refusal is returned, not an ack-timeout error.
#[test]
fn a41_a_late_refusal_after_the_last_send_beats_the_ack_timeout() {
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1));
    tick(&mut st, 3_000);
    tick(&mut st, 6_000);
    let out = st.step(9_000, limited(Some(&id), Some(1_986)));
    assert!(submits(&out).is_empty());
    assert_eq!(
        only_completion(&out).result,
        Err(InvokeError::RateLimited(1_986))
    );
}

/// Oracle 41, the other side (`I:513-517`): before the FIRST send of a round a
/// stale refusal is dropped, not answered. A refusal that landed during the
/// response wait is dropped when the last call starts, and the last call is sent.
#[test]
fn a41_a_refusal_that_landed_during_the_response_wait_is_dropped_by_the_last_call() {
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1).with_timeout(TimeoutArg::Int(1)));
    st.step(10, ack(&id));
    // Lands while the response is awaited: no caller action (`T/invoker_upstream_unavailable_test.exs:77-107`).
    assert!(st.step(20, unavailable(Some(&id))).is_empty());
    assert!(st.step(21, limited(Some(&id), Some(5))).is_empty());
    let out = tick(&mut st, 2_010);
    assert_eq!(submits(&out).len(), 1, "the last call is sent");
    assert!(completions(&out).is_empty());
    // And the same refusal does not answer the last call's ack wait later.
    assert_eq!(st.tracked(), 1);
}

/// Oracle 41 (`I:473`, `T/invoker_late_answer_test.exs:67-76`): a response that
/// lands in front of a send answers the call at its response wait; so does one
/// that arrives before the ack.
#[test]
fn a41_an_early_response_answers_at_the_response_wait() {
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1));
    // The result arrives before the ack: acked at once, held for the call.
    let out = st.step(1, result(&id, r#"{"early":true}"#));
    assert_eq!(out.len(), 1);
    assert!(matches!(out[0], Output::Send { .. }));
    assert_eq!(st.tracked(), 1);
    let out = st.step(2, ack(&id));
    assert_eq!(only_completion(&out).result, Ok(json!({"early": true})));
    assert_eq!(st.tracked(), 0);
}

/// Oracle 41 and 23: a response that lands while a send is being retried is kept
/// across the retry and answers the call after its next ack.
#[test]
fn a41_a_response_landing_in_the_retry_window_answers_the_resent_call() {
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1));
    // Stamped at the deadline: after the wait gave up, before the next ack.
    st.step(3_000, result(&id, r#"{"late":true}"#));
    let out = st.step(3_010, ack(&id));
    assert_eq!(only_completion(&out).result, Ok(json!({"late": true})));
}

/// A stale ack, which Elixir flushes before a send, does not count for the next
/// send: an ack stamped after the wait gave up is dropped, and the resent call
/// still needs its own ack.
#[test]
fn a_stale_ack_does_not_answer_the_next_send() {
    let mut st = InvokeState::new();
    let (id, _) = start(&mut st, 0, req(1));
    // The wait gives up at 3000, the ack is late, then the resent call waits.
    let out = st.step(3_000, ack(&id));
    assert_eq!(submits(&out).len(), 1, "resent anyway");
    assert_eq!(st.next_wakeup(), Some(6_000), "still in its ack wait");
}

// ------------------------------------------------------- sends, misc --

/// `I:621-634`: a send that cannot be made ends the call: a send-time limiter is
/// `{:rate_limited, ms}`, anything else is `{:not_sent, reason}`, and neither is
/// a NACK or retried.
#[test]
fn a_failed_send_ends_the_call_without_a_retry() {
    let mut st = InvokeState::new();
    st.step(0, Input::Call(req(1)));
    let out = st.step(
        1,
        Input::SendFailed {
            call: 1,
            failure: SendFailure::RateLimited(1_500),
        },
    );
    assert_eq!(
        only_completion(&out).result,
        Err(InvokeError::RateLimited(1_500))
    );
    st.step(2, Input::Call(req(2)));
    let out = st.step(
        3,
        Input::SendFailed {
            call: 2,
            failure: SendFailure::Other("not_connected".into()),
        },
    );
    assert_eq!(
        only_completion(&out).result,
        Err(InvokeError::NotSent(NotSent::Transport(
            "not_connected".into()
        )))
    );
    assert_eq!((st.tracked(), st.tracked_ids()), (0, 0));
    // A failure reported for a call that is gone does nothing.
    assert!(
        st.step(
            4,
            Input::SendFailed {
                call: 2,
                failure: SendFailure::Other("again".into())
            }
        )
        .is_empty()
    );
}

/// `I:239-240`, `:280-287`: `configChanged` is handed on; an unexpected frame is
/// a marker carrying its kind and nothing else.
#[test]
fn config_changed_and_unexpected_frames() {
    let mut st = InvokeState::new();
    assert_eq!(
        st.step(0, Input::ConfigChanged),
        vec![Output::ConfigChanged]
    );
    assert_eq!(
        st.step(
            0,
            Input::Unexpected {
                kind: "error".into()
            }
        ),
        vec![Output::Log(Log::UnexpectedFrame {
            kind: "error".into()
        })]
    );
}

/// A call handle that is live is refused with a marker and does not disturb the
/// call that holds it.
#[test]
fn a_live_call_handle_is_refused() {
    let mut st = InvokeState::new();
    start(&mut st, 0, req(1));
    let out = st.step(1, Input::Call(req(1).param("n", json!(2))));
    assert_eq!(out, vec![Output::Log(Log::DuplicateCallHandle { call: 1 })]);
    assert_eq!(st.tracked(), 1);
}

/// The bound: a finished call leaves no entry, whichever way it finished, and
/// nothing is due afterwards.
#[test]
fn a_finished_request_leaves_no_entry() {
    let mut st = InvokeState::new();
    let mut now = 0;
    for n in 0..300u64 {
        let r = req(n).param("n", json!(n));
        let (id, _) = start(&mut st, now, r);
        match n % 6 {
            0 => {
                st.step(now, ack(&id));
                st.step(now, result(&id, "{}"));
            }
            1 => {
                st.step(
                    now,
                    Input::Nack {
                        id,
                        code: 404,
                        message: String::new(),
                    },
                );
            }
            2 => {
                st.step(now, limited(Some(&id), None));
            }
            3 => {
                for _ in 0..3 {
                    now += 3_000;
                    tick(&mut st, now);
                }
            }
            4 => {
                st.step(now, unavailable(Some(&id)));
            }
            _ => {
                st.step(
                    now,
                    Input::SendFailed {
                        call: n,
                        failure: SendFailure::Other("x".into()),
                    },
                );
            }
        }
        assert_eq!(st.tracked(), 0, "call {n}");
        assert_eq!(st.tracked_ids(), 0, "call {n}");
        assert_eq!(st.next_wakeup(), None, "call {n}");
        now += 1;
    }
}

/// `next_wakeup` is the earliest deadline among the waiting calls.
#[test]
fn next_wakeup_is_the_earliest_deadline() {
    let mut st = InvokeState::new();
    assert_eq!(st.next_wakeup(), None);
    start(&mut st, 0, req(1).param("n", json!(1)));
    start(
        &mut st,
        100,
        req(2).param("n", json!(2)).with_ack_timeout(50),
    );
    assert_eq!(st.next_wakeup(), Some(150));
    tick(&mut st, 150);
    assert_eq!(st.next_wakeup(), Some(200));
}

/// Time that jumps past several deadlines fires each wait once: a late tick
/// sends one resend, not a burst, and the next wait counts from the tick.
#[test]
fn a_late_tick_fires_one_wait_not_a_burst() {
    let mut st = InvokeState::new();
    start(&mut st, 0, req(1));
    let out = tick(&mut st, 10_000);
    assert_eq!(submits(&out).len(), 1);
    assert_eq!(st.next_wakeup(), Some(13_000));
}

// ---------------------------------------------------- random interleaving --

/// xorshift64*: deterministic, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// A reference written the naive way: every call keeps its whole mailbox, and
/// each wait is a literal selective receive over it (scan in arrival order, take
/// the first message the wait matches, leave the rest), with `_flush_stale` as a
/// literal scan. The machine keeps at most one response and one refusal per
/// call; the model keeps everything, so the two agree only if that bound loses
/// nothing. The parameters, the timeout rules and the step order are restated
/// here from the Elixir source, not taken from the crate (only `request_id`,
/// which has its own tests, is shared).
mod model {
    use super::*;

    #[derive(Clone, Debug)]
    pub enum Mail {
        Ack,
        Nack(u16, String),
        Refused(Map<String, Value>),
        Limited(u64),
        Response(Value),
    }

    #[derive(Clone, Copy, Debug)]
    pub enum S {
        Retry(u32, bool),
        Ack(u32, bool, Millis),
        Resp(bool, Millis),
    }

    #[derive(Clone, Debug)]
    pub struct C {
        pub call: CallId,
        pub id: String,
        pub handler: String,
        pub cap: String,
        pub params: Map<String, Value>,
        pub timeout_ms: Millis,
        pub ack_ms: Millis,
        pub s: S,
        pub mail: Vec<Mail>,
    }

    #[derive(Default)]
    pub struct Model {
        pub clock: Millis,
        /// In registration order.
        pub calls: Vec<C>,
        pub defaults: BTreeMap<String, Map<String, Value>>,
    }

    fn secs_to_ms(v: &Value) -> Result<Option<Millis>, ()> {
        match v {
            Value::Null => Ok(None),
            Value::Number(n) => match n.as_i64() {
                Some(i) if i >= 1 => {
                    Ok(i.checked_mul(1000).map(|x| x as u64).map(Some).ok_or(())?)
                }
                _ => Err(()),
            },
            Value::String(s) => match s.parse::<i64>() {
                Ok(i) if i >= 1 => Ok(Some(i as u64 * 1000)),
                _ => Err(()),
            },
            _ => Err(()),
        }
    }

    fn arg_value(a: &TimeoutArg) -> Option<Value> {
        match a {
            TimeoutArg::Absent => None,
            TimeoutArg::Null => Some(Value::Null),
            TimeoutArg::Int(i) => Some(json!(i)),
            TimeoutArg::Float(f) => Some(json!(f)),
            TimeoutArg::Str(s) => Some(json!(s)),
            TimeoutArg::Bool(b) => Some(json!(b)),
            TimeoutArg::List(l) => Some(Value::Array(l.clone())),
            TimeoutArg::Object(o) => Some(Value::Object(o.clone())),
        }
    }

    impl Model {
        fn idx(&self, call: CallId) -> Option<usize> {
            self.calls.iter().position(|c| c.call == call)
        }

        fn registered(&self, id: &str) -> Vec<CallId> {
            self.calls
                .iter()
                .filter(|c| c.id == id)
                .map(|c| c.call)
                .collect()
        }

        pub fn next_wakeup(&self) -> Option<Millis> {
            self.calls
                .iter()
                .filter_map(|c| match c.s {
                    S::Ack(_, _, d) | S::Resp(_, d) => Some(d),
                    S::Retry(..) => None,
                })
                .min()
        }

        pub fn step(&mut self, now: Millis, input: &Input) -> Vec<Output> {
            let now = now.max(self.clock);
            self.clock = now;
            let mut out = Vec::new();
            self.expire(now, &mut out);
            self.apply(now, input, &mut out);
            loop {
                self.settle(now, &mut out);
                if !self.expire(now, &mut out) {
                    break;
                }
            }
            out
        }

        fn complete(&mut self, call: CallId, r: Result<Value, InvokeError>, out: &mut Vec<Output>) {
            let i = self.idx(call).expect("live");
            let c = self.calls.remove(i);
            out.push(Output::Complete(Completion {
                call,
                request_id: c.id,
                result: r,
            }));
        }

        fn classify(&self, v: Value, t: Millis) -> Result<Value, InvokeError> {
            if let Some(e) = v.get("error") {
                if e == "exec_timeout" {
                    return Err(InvokeError::HandlerReturnedTimeout(t));
                }
                if e == "request_timed_out"
                    && let Some(ls) = v.get("last_status")
                {
                    return Err(InvokeError::ActionApiReturnedTimeout(ls.clone()));
                }
                return Err(InvokeError::Handler(e.clone()));
            }
            Ok(v)
        }

        /// The receive of whatever wait the call is in, repeated until it
        /// matches nothing.
        fn run(&mut self, call: CallId, now: Millis, out: &mut Vec<Output>) {
            loop {
                let Some(i) = self.idx(call) else { return };
                let c = &mut self.calls[i];
                match c.s {
                    S::Retry(..) => return,
                    S::Ack(_, last, _) => {
                        let Some(p) = c.mail.iter().position(|m| !matches!(m, Mail::Response(_)))
                        else {
                            return;
                        };
                        match c.mail.remove(p) {
                            Mail::Ack => {
                                c.s = S::Resp(last, now + c.timeout_ms + 1_000);
                            }
                            Mail::Limited(ms) => {
                                return self.complete(call, Err(InvokeError::RateLimited(ms)), out);
                            }
                            Mail::Refused(e) => {
                                return self.complete(call, Err(InvokeError::Refused(e)), out);
                            }
                            Mail::Nack(code, message) => {
                                if code != 404 {
                                    out.push(Output::Log(Log::MessageNacked {
                                        request_id: c.id.clone(),
                                        code,
                                    }));
                                }
                                return self.complete(
                                    call,
                                    Err(InvokeError::Nack { code, message }),
                                    out,
                                );
                            }
                            Mail::Response(_) => unreachable!(),
                        }
                    }
                    S::Resp(..) => {
                        let Some(p) = c.mail.iter().position(|m| matches!(m, Mail::Response(_)))
                        else {
                            return;
                        };
                        let Mail::Response(v) = c.mail.remove(p) else {
                            unreachable!()
                        };
                        let t = c.timeout_ms;
                        let r = self.classify(v, t);
                        return self.complete(call, r, out);
                    }
                }
            }
        }

        fn deliver(&mut self, id: &str, m: &Mail, now: Millis, out: &mut Vec<Output>) {
            for call in self.registered(id) {
                let i = self.idx(call).expect("live");
                self.calls[i].mail.push(m.clone());
                self.run(call, now, out);
            }
        }

        fn expire(&mut self, now: Millis, out: &mut Vec<Output>) -> bool {
            let mut due: Vec<(Millis, CallId)> = self
                .calls
                .iter()
                .filter_map(|c| match c.s {
                    S::Ack(_, _, d) | S::Resp(_, d) if d <= now => Some((d, c.call)),
                    _ => None,
                })
                .collect();
            due.sort();
            let any = !due.is_empty();
            for (_, call) in due {
                let i = self.idx(call).expect("live");
                let c = &mut self.calls[i];
                match c.s {
                    S::Ack(n, last, _) => {
                        out.push(Output::Log(Log::AckTimeout {
                            request_id: c.id.clone(),
                            ack_timeout_ms: c.ack_ms,
                        }));
                        c.s = S::Retry(n + 1, last);
                    }
                    S::Resp(true, _) => {
                        out.push(Output::Log(Log::ResponseTimeout {
                            request_id: c.id.clone(),
                        }));
                        let t = c.timeout_ms;
                        self.complete(call, Err(InvokeError::ExecTimeout(t)), out);
                    }
                    S::Resp(false, _) => {
                        out.push(Output::Log(Log::SendingLastCall {
                            request_id: c.id.clone(),
                        }));
                        c.s = S::Retry(1, true);
                    }
                    S::Retry(..) => unreachable!(),
                }
            }
            any
        }

        fn settle(&mut self, now: Millis, out: &mut Vec<Output>) {
            let pending: Vec<CallId> = self
                .calls
                .iter()
                .filter(|c| matches!(c.s, S::Retry(..)))
                .map(|c| c.call)
                .collect();
            let mut pending = pending;
            pending.sort_unstable();
            for call in pending {
                let i = self.idx(call).expect("live");
                let S::Retry(n, last) = self.calls[i].s else {
                    unreachable!()
                };
                // The literal `_flush_stale`: in order, acks and NACKs are dropped,
                // a refusal is answered (from the second send) or dropped.
                let mut answered = None;
                let mut k = 0;
                while k < self.calls[i].mail.len() {
                    match self.calls[i].mail[k].clone() {
                        Mail::Ack | Mail::Nack(..) => {
                            self.calls[i].mail.remove(k);
                        }
                        Mail::Limited(ms) if n > 1 => {
                            answered = Some(InvokeError::RateLimited(ms));
                            break;
                        }
                        Mail::Refused(e) if n > 1 => {
                            answered = Some(InvokeError::Refused(e));
                            break;
                        }
                        Mail::Limited(_) | Mail::Refused(_) => {
                            self.calls[i].mail.remove(k);
                        }
                        Mail::Response(_) => k += 1,
                    }
                }
                if let Some(e) = answered {
                    self.complete(call, Err(e), out);
                    continue;
                }
                let c = &mut self.calls[i];
                if n > 3 {
                    let w = c.ack_ms * 3;
                    self.complete(call, Err(InvokeError::AckTimeout(w)), out);
                    continue;
                }
                out.push(Output::Send {
                    call: Some(call),
                    frame: Frame::Submit(SubmitFrame {
                        id: c.id.clone(),
                        handler: c.handler.clone(),
                        capability: c.cap.clone(),
                        parameters: c.params.clone(),
                        timeout_ms: c.timeout_ms,
                    }),
                });
                c.s = S::Ack(n, last, now + c.ack_ms);
                self.run(call, now, out);
            }
        }

        fn apply(&mut self, now: Millis, input: &Input, out: &mut Vec<Output>) {
            match input {
                Input::Call(r) => self.call(r, out),
                Input::Ack { id } => self.deliver(id, &Mail::Ack, now, out),
                Input::Nack { id, code, message } => {
                    self.deliver(id, &Mail::Nack(*code, message.clone()), now, out);
                }
                Input::Result { id, result } => {
                    out.push(Output::Send {
                        call: None,
                        frame: Frame::AckResult { id: id.clone() },
                    });
                    let v = match result {
                        None => Value::Null,
                        Some(s) => serde_json::from_str(s).unwrap_or_else(|_| json!(s)),
                    };
                    self.deliver(id, &Mail::Response(v), now, out);
                }
                Input::RateLimited(r) => {
                    let ms = r.retry_after_ms();
                    match &r.id {
                        None => out.push(Output::Log(Log::RateLimitedNoId { retry_after_ms: ms })),
                        Some(id) if self.registered(id).is_empty() => {
                            out.push(Output::Log(Log::RefusalNoCaller {
                                request_id: id.clone(),
                                code: 429,
                            }));
                        }
                        Some(id) => self.deliver(id, &Mail::Limited(ms), now, out),
                    }
                }
                Input::Unavailable { id, error } => match id {
                    None => out.push(Output::Log(Log::UpstreamUnavailableNoId)),
                    Some(id) if self.registered(id).is_empty() => {
                        out.push(Output::Log(Log::RefusalNoCaller {
                            request_id: id.clone(),
                            code: 503,
                        }));
                    }
                    Some(id) => self.deliver(id, &Mail::Refused(error.clone()), now, out),
                },
                Input::ConfigChanged => out.push(Output::ConfigChanged),
                Input::Unexpected { kind } => {
                    out.push(Output::Log(Log::UnexpectedFrame { kind: kind.clone() }));
                }
                Input::SendFailed { call, failure } => {
                    if let Some(i) = self.idx(*call)
                        && matches!(self.calls[i].s, S::Ack(..))
                    {
                        let e = match failure {
                            SendFailure::RateLimited(ms) => InvokeError::RateLimited(*ms),
                            SendFailure::Other(r) => {
                                InvokeError::NotSent(NotSent::Transport(r.clone()))
                            }
                        };
                        self.complete(*call, Err(e), out);
                    }
                }
                Input::Tick => {}
                _ => unreachable!("the driver sends no other input"),
            }
        }

        fn call(&mut self, r: &CallRequest, out: &mut Vec<Output>) {
            if self.idx(r.call).is_some() {
                out.push(Output::Log(Log::DuplicateCallHandle { call: r.call }));
                return;
            }
            let defaults = self
                .defaults
                .get(&r.capability)
                .cloned()
                .unwrap_or_default();
            let mut params = defaults.clone();
            for (k, v) in &r.params {
                params.insert(k.clone(), v.clone());
            }
            let (given, source) = match arg_value(&r.timeout) {
                Some(v) => (Some(v), TimeoutSource::Caller),
                None => (
                    defaults.get("timeout").cloned(),
                    TimeoutSource::CapabilityDefaults,
                ),
            };
            params.remove("timeout");
            if let Some(v) = &given {
                params.insert("timeout".to_owned(), v.clone());
            }
            let shown = given.clone().unwrap_or(Value::Null);
            match given.as_ref().map_or(Ok(None), secs_to_ms) {
                Err(()) => {
                    let id = request_id(&r.ticket, &r.handler, &r.capability, &params);
                    out.push(Output::Log(Log::InvalidTimeoutRefused {
                        request_id: id.clone(),
                        source,
                    }));
                    out.push(Output::Complete(Completion {
                        call: r.call,
                        request_id: id,
                        result: Err(InvokeError::NotSent(NotSent::InvalidTimeout(shown))),
                    }));
                }
                Ok(ms) => {
                    params.insert("timeout".to_owned(), ms.map_or(Value::Null, |m| json!(m)));
                    let id = request_id(&r.ticket, &r.handler, &r.capability, &params);
                    self.calls.push(C {
                        call: r.call,
                        id,
                        handler: r.handler.clone(),
                        cap: r.capability.clone(),
                        params,
                        timeout_ms: ms.or(r.default_timeout_ms).unwrap_or(60_000),
                        ack_ms: r.ack_timeout_ms.unwrap_or(3_000),
                        s: S::Retry(1, false),
                        mail: Vec::new(),
                    });
                }
            }
        }
    }
}

fn random_call(rng: &mut Rng, handles: &mut u64, live: &[CallId]) -> CallRequest {
    // Reuse a live handle now and then, to hit the refusal.
    let call = if !live.is_empty() && rng.below(25) == 0 {
        live[rng.below(live.len() as u64) as usize]
    } else {
        *handles += 1;
        *handles
    };
    let cap = ["ExecuteCommand", "RunScript"][rng.below(2) as usize];
    let mut r = CallRequest::new(call, format!("ticket-{}", rng.below(2)), "handler-1", cap);
    if rng.below(2) == 0 {
        r = r.param("n", json!(rng.below(2)));
    }
    r.timeout = match rng.below(12) {
        0..=2 => TimeoutArg::Absent,
        3 | 4 => TimeoutArg::Int(1),
        5 => TimeoutArg::Int(2),
        6 => TimeoutArg::Str("1".into()),
        7 => TimeoutArg::Null,
        8 => TimeoutArg::Int(0),
        9 => TimeoutArg::Float(1.5),
        10 => TimeoutArg::Bool(true),
        _ => TimeoutArg::Str("abc".into()),
    };
    r.ack_timeout_ms = match rng.below(6) {
        0 => Some(50),
        1 => Some(700),
        2 => Some(0),
        _ => None,
    };
    if rng.below(8) == 0 {
        r.default_timeout_ms = Some(2_500);
    }
    r
}

fn random_input(rng: &mut Rng, handles: &mut u64, known: &[String], live: &[CallId]) -> Input {
    let id = if known.is_empty() || rng.below(12) == 0 {
        "ghost".to_owned()
    } else {
        known[rng.below(known.len() as u64) as usize].clone()
    };
    match rng.below(20) {
        0..=4 => Input::Call(random_call(rng, handles, live)),
        5..=8 => Input::Ack { id },
        9 => Input::Nack {
            id,
            code: [404, 403, 500][rng.below(3) as usize],
            message: "nack".to_owned(),
        },
        10..=12 => Input::Result {
            id,
            result: match rng.below(8) {
                0 => None,
                1 => Some("plain text".to_owned()),
                2 => Some(r#"{"error":{"code":404}}"#.to_owned()),
                3 => Some(r#"{"error":"exec_timeout"}"#.to_owned()),
                4 => Some(r#"{"error":"request_timed_out","last_status":"x"}"#.to_owned()),
                5 => Some("42".to_owned()),
                _ => Some(format!(r#"{{"ok":{}}}"#, rng.below(3))),
            },
        },
        13 | 14 => Input::RateLimited(RateLimited {
            id: if rng.below(5) == 0 { None } else { Some(id) },
            message: None,
            retry_after_ms: [None, Some(-1), Some(1_986), Some(999_999_999_999)]
                [rng.below(4) as usize],
        }),
        15 => {
            let mut error = Map::new();
            error.insert("code".to_owned(), json!(503));
            error.insert("n".to_owned(), json!(rng.below(100)));
            Input::Unavailable {
                id: if rng.below(5) == 0 { None } else { Some(id) },
                error,
            }
        }
        16 => {
            if live.is_empty() {
                Input::Tick
            } else {
                Input::SendFailed {
                    call: live[rng.below(live.len() as u64) as usize],
                    failure: if rng.below(2) == 0 {
                        SendFailure::RateLimited(rng.below(5_000))
                    } else {
                        SendFailure::Other("closed".to_owned())
                    },
                }
            }
        }
        17 => {
            if rng.below(2) == 0 {
                Input::ConfigChanged
            } else {
                Input::Unexpected {
                    kind: "error".to_owned(),
                }
            }
        }
        _ => Input::Tick,
    }
}

fn random_advance(rng: &mut Rng) -> Millis {
    match rng.below(20) {
        0..=3 => 0,
        4..=8 => 1,
        9..=10 => 49,
        11..=12 => 50,
        13 => 700,
        14 => 999,
        15 => 1_000,
        16 => 3_000,
        17 => 3_001,
        18 => 6_000,
        _ => 20_000,
    }
}

/// Oracle 20 to 41 together: 20000 random steps per seed (calls, acks, NACKs,
/// results of every kind, 429s and 503s with and without ids, send failures,
/// handle clashes, `configChanged`) with the clock jumping across the ack,
/// response and last-call boundaries, against the naive reference model. The
/// outputs, `next_wakeup` and the tracked counts agree at every step; the
/// registry never holds an id without a call, nor a call without an id; and
/// after a long enough quiet every call has ended and nothing is tracked.
#[test]
fn random_interleaving_agrees_with_the_reference_model() {
    for seed in [1u64, 2, 3, 0xDEAD_BEEF, 0x1234_5678_9ABC_DEF1, 77] {
        let mut rng = Rng(seed);
        let mut st = InvokeState::new();
        let mut want = model::Model::default();
        let mut defaults = Map::new();
        defaults.insert("timeout".to_owned(), json!(2));
        defaults.insert("host".to_owned(), json!("localhost"));
        st.set_capability_defaults("ExecuteCommand", defaults.clone());
        want.defaults.insert("ExecuteCommand".to_owned(), defaults);

        let mut now: Millis = 0;
        let mut handles = 0u64;
        let mut known: Vec<String> = Vec::new();
        let mut completed = 0usize;
        let mut sent = 0usize;
        for step in 0..20_000 {
            now += random_advance(&mut rng);
            let live: Vec<CallId> = want.calls.iter().map(|c| c.call).collect();
            let input = random_input(&mut rng, &mut handles, &known, &live);
            let expected = want.step(now, &input);
            let got = st.step(now, input.clone());
            assert_eq!(got, expected, "seed {seed:#x} step {step} input {input:?}");
            assert_eq!(st.tracked(), want.calls.len(), "seed {seed:#x} step {step}");
            let distinct: std::collections::BTreeSet<&str> =
                want.calls.iter().map(|c| c.id.as_str()).collect();
            assert_eq!(
                st.tracked_ids(),
                distinct.len(),
                "seed {seed:#x} step {step}"
            );
            assert_eq!(
                st.next_wakeup(),
                want.next_wakeup(),
                "seed {seed:#x} step {step}"
            );
            for o in &got {
                match o {
                    Output::Complete(_) => completed += 1,
                    Output::Send {
                        frame: Frame::Submit(s),
                        ..
                    } => {
                        sent += 1;
                        if !known.contains(&s.id) && known.len() < 12 {
                            known.push(s.id.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        assert!(completed > 500 && sent > 500, "{completed} {sent}");
        // Quiet: every wait expires, every call ends, nothing is left.
        for _ in 0..4 {
            now += 100_000;
            let expected = want.step(now, &Input::Tick);
            assert_eq!(st.step(now, Input::Tick), expected, "seed {seed:#x} drain");
        }
        assert_eq!(st.tracked(), 0, "seed {seed:#x}");
        assert_eq!(st.tracked_ids(), 0, "seed {seed:#x}");
        assert_eq!(st.next_wakeup(), None, "seed {seed:#x}");
    }
}
