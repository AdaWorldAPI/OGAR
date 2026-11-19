//! REST discovery, registration and the capability fetch
//! (`ogar_action_handler::discovery`) against graph_conn. `GR` is
//! `lib/graph_conn/graph_rest_calls.ex`, `RQ` `lib/graph_conn/request.ex`, `I`
//! `lib/action_api/invoker.ex`, `B` `lib/graph_conn/backoff.ex`, `CM`
//! `lib/graph_conn/connection_manager.ex`, `T` `test/action_api` and `M`
//! `test/graph_conn`. Time is injected; nothing here reads a clock.

use ogar_action_handler::discovery::{
    Auth, AuthError, BackoffConfig, Draw, FetchError, FetchFailure, FetchInput, FetchKind,
    FetchLog, FetchOutcome, FetchOutput, FetchState, RETRY_FLOOR_MAX_MS, RETRY_INITIAL_MS,
    RETRY_JITTER_MS, RETRY_MAX_MS, Registration, VersionsError, all_capability_defaults, auth_path,
    auth_response, capability_defaults, next_delay, parse_auth_response, parse_versions,
    prefix_path, retry_after_ms, versions_response,
};
use ogar_action_handler::invoke_state::{
    CallRequest, Frame, Input, InvokeState, Output, SubmitFrame,
};
use serde_json::{Value, json};

fn h(name: &str, value: &str) -> Vec<(String, String)> {
    vec![(name.to_owned(), value.to_owned())]
}

// ------------------------------------------------------------- versions --

/// The two entries the invoker needs, verbatim from the mock gateway
/// (`mock_router.ex:199-216`), among the other fields the answer carries.
fn mock_versions() -> Value {
    json!({
        "action": {
            "docs": "https://docs.hiro.arago.co/",
            "endpoint": "/api/0.9/action/",
            "lifecycle": "experimental",
            "protocols": "",
            "specs": "action",
            "support": "supported",
            "version": "0.9"
        },
        "action-ws": {
            "docs": "https://docs.hiro.arago.co/",
            "endpoint": "/api/0.9/action-ws/",
            "lifecycle": "experimental",
            "protocols": "action-0.9.0",
            "specs": "",
            "support": "supported",
            "version": "0.9"
        },
        "auth": {
            "docs": "https://docs.hiro.arago.co/",
            "endpoint": "/api/6/auth/",
            "lifecycle": "stable",
            "protocols": "",
            "specs": "auth.yaml",
            "support": "supported",
            "version": "6"
        }
    })
}

/// `GR:115-125`, `:41-45` (the doc example): an entry takes `endpoint`,
/// `protocols` and `version` as `path`, `protocol` and `subprotocol`; the action
/// REST entry is `action` and the socket is `action-ws`.
#[test]
fn versions_finds_the_action_and_action_ws_entries() {
    let v = parse_versions(&mock_versions().to_string()).expect("parses");
    assert!(v.skipped.is_empty());
    let a = v.action().expect("action");
    assert_eq!(
        (a.path.as_str(), a.protocol.as_str(), a.subprotocol.as_str()),
        ("/api/0.9/action/", "", "0.9")
    );
    let w = v.action_ws().expect("action-ws");
    assert_eq!(
        (w.path.as_str(), w.protocol.as_str(), w.subprotocol.as_str()),
        ("/api/0.9/action-ws/", "action-0.9.0", "0.9")
    );
    assert_eq!(v.entry("action").expect("known"), a);
}

/// `GR:132-133`, `:134-148`: a malformed entry is skipped and named, not fatal.
/// The others are kept. Missing field, a non-string field, an entry that is not an
/// object, and a name that cannot be an API name are all malformed.
#[test]
fn versions_a_malformed_entry_is_skipped_not_fatal() {
    let mut body = mock_versions();
    let o = body.as_object_mut().expect("object");
    // `mock_router.ex:164-172` deletes `endpoint`.
    o.insert(
        "broken".into(),
        json!({"protocols": "", "version": "1", "docs": "x"}),
    );
    o.insert(
        "numeric-version".into(),
        json!({"endpoint": "/e/", "protocols": "", "version": 6}),
    );
    o.insert("not-an-object".into(), json!("/api/x/"));
    o.insert("null-entry".into(), Value::Null);
    let v = parse_versions(&body.to_string()).expect("the usable ones are kept");
    assert_eq!(
        v.skipped,
        vec!["broken", "not-an-object", "null-entry", "numeric-version"]
    );
    assert_eq!(
        v.apis.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["action", "action-ws", "auth"]
    );
}

/// `GR:14-15`, `:177-180`: the name must match `\A[a-zA-Z0-9][a-zA-Z0-9_.-]{0,63}\z`.
#[test]
fn versions_a_name_that_is_not_an_api_name_is_skipped() {
    let entry = json!({"endpoint": "/e/", "protocols": "", "version": "1"});
    let long_ok = "a".repeat(64);
    let too_long = "a".repeat(65);
    let mut body = serde_json::Map::new();
    body.insert("good".into(), entry.clone());
    body.insert("a_b.c-d".into(), entry.clone());
    body.insert(long_ok.clone(), entry.clone());
    for bad in [
        too_long.as_str(),
        "",
        "-lead",
        ".lead",
        "_lead",
        "has space",
        "sl/ash",
        "caf\u{e9}",
        "new\nline",
    ] {
        body.insert(bad.into(), entry.clone());
    }
    let v = parse_versions(&Value::Object(body).to_string()).expect("parses");
    let mut kept: Vec<&str> = v.apis.keys().map(String::as_str).collect();
    kept.sort_unstable();
    let mut want = vec!["a_b.c-d", "good", long_ok.as_str()];
    want.sort_unstable();
    assert_eq!(kept, want);
    assert_eq!(v.skipped.len(), 9);
}

/// `GR:98-153`: not JSON, not an object, and no usable entry are refused, each by
/// name.
#[test]
fn versions_refusals() {
    assert_eq!(
        parse_versions("<html>proxy</html>"),
        Err(VersionsError::NotJson)
    );
    assert_eq!(parse_versions(""), Err(VersionsError::NotJson));
    assert_eq!(parse_versions("[1]"), Err(VersionsError::NotAnObject));
    assert_eq!(parse_versions("\"x\""), Err(VersionsError::NotAnObject));
    assert_eq!(parse_versions("{}"), Err(VersionsError::NoUsableApi));
    assert_eq!(
        parse_versions(r#"{"a":{"endpoint":"/e/"}}"#),
        Err(VersionsError::NoUsableApi)
    );
}

/// `GR:80-92`: only a 200 is read; 429 is a rate limit with its `retry-after`;
/// any other status is `{:unexpected_status, status}`.
#[test]
fn versions_response_by_status() {
    let body = mock_versions().to_string();
    assert!(versions_response(200, &[], &body).is_ok());
    assert_eq!(
        versions_response(429, &h("Retry-After", "3"), ""),
        Err(VersionsError::RateLimited(3_000))
    );
    assert_eq!(
        versions_response(429, &[], ""),
        Err(VersionsError::RateLimited(0))
    );
    assert_eq!(
        versions_response(503, &[], &body),
        Err(VersionsError::UnexpectedStatus(503))
    );
}

/// `GR:274-280`: an API the answer did not name is `{:unknown_api, known}`. The
/// auth entry of a second answer is merged in and nothing else of it (`GR:68-72`).
#[test]
fn versions_unknown_api_and_auth_merge() {
    let graph = parse_versions(
        &json!({"action": {"endpoint": "/api/0.9/action/", "protocols": "", "version": "0.9"}})
            .to_string(),
    )
    .expect("parses");
    let e = graph.entry("events-ws").expect_err("unknown");
    assert_eq!(e.known, vec!["action"]);
    let auth_answer = parse_versions(&mock_versions().to_string()).expect("parses");
    let merged = graph.with_auth_from(&auth_answer);
    assert_eq!(
        merged.apis.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["action", "auth"],
        "only `auth` is taken from the second answer"
    );
    assert_eq!(merged.entry("auth").expect("auth").path, "/api/6/auth/");
}

/// `RQ:38-39`, `GR:263`, `:205`: a request path is the endpoint then the path,
/// plain concatenation; the auth request is the auth endpoint then `app`.
#[test]
fn paths_are_prefixed_with_the_version_endpoint() {
    assert_eq!(
        prefix_path("/api/0.9/action/", "capabilities"),
        "/api/0.9/action/capabilities"
    );
    assert_eq!(
        prefix_path("/api/0.9/action/", "applicabilities"),
        "/api/0.9/action/applicabilities"
    );
    assert_eq!(auth_path("/api/6/auth/"), "/api/6/auth/app");
    assert_eq!(prefix_path("", "/api/version"), "/api/version");
}

// ----------------------------------------------------------------- auth --

/// `GR:292-299`, `:321-326`: `_TOKEN` and `expires-at` (unix ms); a float
/// `expires-at` is truncated; only the type is checked, so a past one is fine.
#[test]
fn auth_response_reads_token_and_expiry() {
    assert_eq!(
        parse_auth_response(r#"{"_TOKEN":"tok","expires-at":1760000000000}"#),
        Ok(Auth {
            token: "tok".into(),
            expires_at: 1_760_000_000_000
        })
    );
    assert_eq!(
        parse_auth_response(r#"{"_TOKEN":"tok","expires-at":1760000000000.9,"extra":1}"#)
            .expect("float")
            .expires_at,
        1_760_000_000_000
    );
    assert_eq!(
        parse_auth_response(r#"{"_TOKEN":"tok","expires-at":1}"#)
            .expect("past")
            .expires_at,
        1
    );
}

/// `GR:283-329`: each way an auth answer can be wrong is its own error.
#[test]
fn auth_response_refusals() {
    assert_eq!(parse_auth_response("nope"), Err(AuthError::NotJson));
    assert_eq!(parse_auth_response("[]"), Err(AuthError::NotAnObject));
    // A 200 carrying an error object: named by its keys, sorted (`GR:305-309`).
    assert_eq!(
        parse_auth_response(r#"{"error":"x","code":401}"#),
        Err(AuthError::InvalidAuthResponse(vec![
            "code".into(),
            "error".into()
        ]))
    );
    // A token without `expires-at` still names the keys.
    assert_eq!(
        parse_auth_response(r#"{"_TOKEN":"t"}"#),
        Err(AuthError::InvalidAuthResponse(vec!["_TOKEN".into()]))
    );
    assert_eq!(
        parse_auth_response(r#"{"_TOKEN":"t","expires-at":"soon"}"#),
        Err(AuthError::InvalidExpiresAt(json!("soon")))
    );
    assert_eq!(
        parse_auth_response(r#"{"_TOKEN":"t","expires-at":null}"#),
        Err(AuthError::InvalidExpiresAt(Value::Null))
    );
}

/// `GR:208-232`: by status, 401 is wrong credentials and 429 a rate limit.
#[test]
fn auth_response_by_status() {
    assert_eq!(
        auth_response(401, &[], ""),
        Err(AuthError::WrongCredentials)
    );
    assert_eq!(
        auth_response(429, &h("retry-after", " 7 "), ""),
        Err(AuthError::RateLimited(7_000))
    );
    assert_eq!(auth_response(500, &[], ""), Err(AuthError::Rejected(500)));
    assert!(auth_response(200, &[], r#"{"_TOKEN":"t","expires-at":5}"#).is_ok());
}

/// `RA:27-35`, `:93-123`: only delay-seconds, case-insensitive name, trimmed,
/// capped at a day; missing, non-numeric, zero and negative are no wait.
#[test]
fn retry_after_header() {
    assert_eq!(retry_after_ms(&h("Retry-After", "3")), 3_000);
    assert_eq!(retry_after_ms(&h("RETRY-AFTER", "+3")), 3_000);
    assert_eq!(retry_after_ms(&h("retry-after", "999999999")), 86_400_000);
    assert_eq!(
        retry_after_ms(&h("retry-after", &"9".repeat(60))),
        86_400_000
    );
    for none in ["", "abc", "1.5", "0", "-3", "Wed, 21 Oct 2015 07:28:00 GMT"] {
        assert_eq!(retry_after_ms(&h("retry-after", none)), 0, "{none:?}");
    }
    assert_eq!(retry_after_ms(&[]), 0);
}

// ------------------------------------------------------ capability defaults --

/// The capabilities the mock serves for `{"ExecuteCommand": {"timeout": 60000,
/// "command": null}}` (`M/mock_test.exs:73-93`, `mock.ex:592-611`).
fn mock_capabilities() -> Value {
    json!({
        "ExecuteCommand": {
            "mandatoryParameters": {"command": {}},
            "optionalParameters": {"timeout": {"default": 60000}}
        }
    })
}

/// `I:343-360`: the defaults are the `default` of every parameter. A mandatory
/// parameter with no default has none.
#[test]
fn capability_defaults_as_the_invoker_reads_them() {
    let caps = mock_capabilities();
    let d = capability_defaults(&caps, "ExecuteCommand");
    assert_eq!(Value::Object(d), json!({"timeout": 60000}));
    // An unknown capability has none (`I:348-349`, `invoker_test.exs:39-41`).
    assert!(capability_defaults(&caps, "invalid_capability").is_empty());
    assert!(capability_defaults(&Value::Null, "ExecuteCommand").is_empty());
}

/// `I:351-355`: the optional map is merged over the mandatory one, so a field in
/// both takes the optional entry; a `null` default is a default; a default of any
/// JSON type is kept as it is.
#[test]
fn capability_defaults_merge_and_types() {
    let caps = json!({
        "RunScript": {
            "mandatoryParameters": {
                "host": {"default": "localhost"},
                "both": {"default": "from-mandatory"},
                "plain": {"description": "no default"}
            },
            "optionalParameters": {
                "both": {"default": "from-optional"},
                "flag": {"default": false},
                "none": {"default": null},
                "list": {"default": [1, 2]},
                "shape": "not an object"
            }
        },
        "Bare": {}
    });
    assert_eq!(
        Value::Object(capability_defaults(&caps, "RunScript")),
        json!({
            "host": "localhost",
            "both": "from-optional",
            "flag": false,
            "none": null,
            "list": [1, 2]
        })
    );
    // HIRO raises for a capability without the two maps; here it has no defaults.
    assert!(capability_defaults(&caps, "Bare").is_empty());
    let all = all_capability_defaults(&caps);
    assert_eq!(
        all.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["RunScript"]
    );
}

/// The registration hands its bodies to the parsers that already exist and
/// reads the handler ids from the keys (`M/mock_test.exs:49-50`,
/// `T/invoker_test.exs:281-285`).
#[test]
fn registration_reuses_the_existing_parsers() {
    let reg = Registration {
        capabilities: mock_capabilities(),
        applicabilities: json!({
            "action_handler": {"ExecuteLocalCommand": {"on ogit/_id": {"LocalNodeID": "${ogit/_id}"}}}
        }),
    };
    let caps = reg.concrete_capabilities().expect("MapOfCapabilities");
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].name, "ExecuteCommand");
    assert_eq!(caps[0].params.len(), 2);
    assert!(
        reg.typed_applicabilities().is_ok(),
        "the on-shaped body is read by parse_applicabilities_typed"
    );
    assert_eq!(reg.handler_ids(), vec!["action_handler"]);
}

// ---------------------------------------------------------------- backoff --

/// `CM:70-73`: the defaults of the curve.
#[test]
fn backoff_defaults() {
    assert_eq!(
        (
            RETRY_INITIAL_MS,
            RETRY_MAX_MS,
            RETRY_JITTER_MS,
            RETRY_FLOOR_MAX_MS
        ),
        (1_000, 10_000, 1_000, 300_000)
    );
    let c = BackoffConfig::default();
    assert_eq!(
        (c.initial_ms, c.max_ms, c.jitter_ms, c.floor_max_ms),
        (1_000, 10_000, 1_000, 300_000)
    );
}

/// `B:11-12`, `:16-20`, `:40-63`, `CM:465-479`: the curve from the seed of 1000
/// doubles up to the cap of 10000: 2000, 4000, 8000, 10000, 10000. Each wait is
/// that value halved plus a draw in `1..=max(half, 1)`: so the longest waits are
/// 2000, 4000, 8000, 10000, 10000 and the shortest 1001, 2001, 4001, 5001, 5001.
#[test]
fn backoff_sequence_is_pinned() {
    let cfg = BackoffConfig::default();
    for (mut draw, want) in [
        (Draw::High, [2_000, 4_000, 8_000, 10_000, 10_000, 10_000]),
        (Draw::Low, [1_001, 2_001, 4_001, 5_001, 5_001, 5_001]),
    ] {
        let mut current = RETRY_INITIAL_MS;
        let mut curve = Vec::new();
        for w in want {
            let (delay, next) = next_delay(&cfg, current, 0, &mut draw);
            assert_eq!(delay, w);
            curve.push(next);
            current = next;
        }
        assert_eq!(curve, vec![2_000, 4_000, 8_000, 10_000, 10_000, 10_000]);
    }
}

/// Over thousands of seeded draws every wait stays in `(half, 2*half]` of its
/// curve value, and the curve is the same whatever the draws are.
#[test]
fn backoff_jitter_stays_in_its_range() {
    let cfg = BackoffConfig::default();
    let mut draw = Draw::Seeded(0xC0FFEE);
    let tops = [2_000u64, 4_000, 8_000, 10_000, 10_000];
    let mut lo = [u64::MAX; 5];
    let mut hi = [0u64; 5];
    for round in 0..4_000 {
        let mut current = RETRY_INITIAL_MS;
        for (i, top) in tops.into_iter().enumerate() {
            let (delay, next) = next_delay(&cfg, current, 0, &mut draw);
            assert_eq!(next, top, "round {round} step {i}");
            assert!(delay > top / 2 && delay <= top, "{delay} for {top}");
            lo[i] = lo[i].min(delay);
            hi[i] = hi[i].max(delay);
            current = next;
        }
    }
    // The draws do spread over the range rather than sit at one end.
    for (i, top) in tops.into_iter().enumerate() {
        assert!(lo[i] < top / 2 + top / 20, "low end {} for {top}", lo[i]);
        assert!(hi[i] > top - top / 20, "high end {} for {top}", hi[i]);
    }
}

/// `B:22-33`, `:50-60`, `CM:1063-1071`: an advertised wait is a floor. The delay
/// is the larger of the curve and `floor + draw(1..=spread)`, the floor is
/// clamped to 300000, and the carried curve value ignores it.
#[test]
fn backoff_advertised_wait_is_the_floor() {
    let cfg = BackoffConfig::default();
    // 3 s advertised, first retry: the curve is at most 2000, the floor wins.
    assert_eq!(
        next_delay(&cfg, 1_000, 3_000, &mut Draw::Low),
        (3_001, 2_000)
    );
    assert_eq!(
        next_delay(&cfg, 1_000, 3_000, &mut Draw::High),
        (4_000, 2_000)
    );
    // A short advertised wait under the curve: the curve wins.
    assert_eq!(
        next_delay(&cfg, 8_000, 500, &mut Draw::Low),
        (5_001, 10_000)
    );
    // Clamped to the ceiling.
    assert_eq!(
        next_delay(&cfg, 1_000, 400_000, &mut Draw::Low),
        (300_001, 2_000)
    );
    assert_eq!(
        next_delay(&cfg, 1_000, 86_400_000, &mut Draw::High),
        (301_000, 2_000)
    );
    // A ceiling of 0 is "none" (`CM:1067`).
    let open = BackoffConfig {
        floor_max_ms: 0,
        ..cfg
    };
    assert_eq!(
        next_delay(&open, 1_000, 400_000, &mut Draw::Low),
        (400_001, 2_000)
    );
}

/// `CM:466-469`: a cap under the seed is raised to the seed; a `current` of 0 is
/// absorbed (`B:31-32`, `:43-46`).
#[test]
fn backoff_cap_is_at_least_the_seed_and_zero_is_absorbed() {
    let low_cap = BackoffConfig {
        max_ms: 10,
        ..BackoffConfig::default()
    };
    assert_eq!(
        next_delay(&low_cap, 1_000, 0, &mut Draw::High),
        (1_000, 1_000)
    );
    assert_eq!(
        next_delay(&BackoffConfig::default(), 0, 0, &mut Draw::High),
        (2, 2)
    );
}

// ------------------------------------------------------------ fetch machine --

const ENDPOINT: &str = "/api/0.9/action/";

fn ok(body: &Value) -> FetchOutcome {
    FetchOutcome::Response {
        status: 200,
        headers: vec![],
        body: body.to_string(),
    }
}

fn status(code: u16) -> FetchOutcome {
    FetchOutcome::Response {
        status: code,
        headers: vec![],
        body: String::new(),
    }
}

fn fetched(kind: FetchKind, outcome: FetchOutcome) -> FetchInput {
    FetchInput::Fetched { kind, outcome }
}

fn fetches(out: &[FetchOutput]) -> Vec<(FetchKind, &str)> {
    out.iter()
        .filter_map(|o| match o {
            FetchOutput::Fetch { kind, path } => Some((*kind, path.as_str())),
            _ => None,
        })
        .collect()
}

fn failure_of(out: &[FetchOutput]) -> (FetchFailure, u64) {
    out.iter()
        .find_map(|o| match o {
            FetchOutput::Log(FetchLog::FetchFailed {
                failure,
                retry_in_ms,
            }) => Some((failure.clone(), *retry_in_ms)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no failure in {out:?}"))
}

fn applicabilities() -> Value {
    json!({"action_handler": {}})
}

/// `I:160-170`, `:849-856`: the first attempt is at once, and it is the
/// capabilities, at the path with the action endpoint in front (`I:760`).
#[test]
fn fetch_first_attempt_is_at_once() {
    let mut f = FetchState::new(ENDPOINT, Draw::Low);
    let out = f.step(0, FetchInput::Start);
    assert_eq!(
        fetches(&out),
        vec![(FetchKind::Capabilities, "/api/0.9/action/capabilities")]
    );
    assert_eq!(f.next_wakeup(), None, "nothing is waited for");
    assert!(!f.is_ready());
}

/// `I:759-763`: capabilities then applicabilities, then ready with both bodies.
#[test]
fn fetch_goes_capabilities_then_applicabilities_then_ready() {
    let mut f = FetchState::new(ENDPOINT, Draw::Low);
    f.step(0, FetchInput::Start);
    let out = f.step(
        5,
        fetched(FetchKind::Capabilities, ok(&mock_capabilities())),
    );
    assert_eq!(
        fetches(&out),
        vec![(
            FetchKind::Applicabilities,
            "/api/0.9/action/applicabilities"
        )]
    );
    let out = f.step(
        9,
        fetched(FetchKind::Applicabilities, ok(&applicabilities())),
    );
    assert_eq!(
        out,
        vec![FetchOutput::Ready(Registration {
            capabilities: mock_capabilities(),
            applicabilities: applicabilities(),
        })]
    );
    assert!(f.is_ready());
    assert_eq!(f.next_wakeup(), None);
}

/// `I:849-894` with `B`, `CM`: any failure keeps the invoker initialising and the
/// retry is on the curve. Failing every attempt with the longest draws, the
/// waits are 2000, 4000, 8000, 10000, 10000 ms; each retry starts again from the
/// capabilities. With the shortest draws they are 1001, 2001, 4001, 5001, 5001.
#[test]
fn fetch_retry_sequence_is_pinned() {
    for (draw, want) in [
        (Draw::High, [2_000u64, 4_000, 8_000, 10_000, 10_000]),
        (Draw::Low, [1_001, 2_001, 4_001, 5_001, 5_001]),
    ] {
        let mut f = FetchState::new(ENDPOINT, draw);
        let mut now = 100;
        f.step(now, FetchInput::Start);
        for (i, w) in want.into_iter().enumerate() {
            let out = f.step(now, fetched(FetchKind::Capabilities, status(500)));
            let (fail, delay) = failure_of(&out);
            assert_eq!(delay, w, "retry {i}");
            assert_eq!(fail.path, "capabilities");
            assert_eq!(f.next_wakeup(), Some(now + w));
            // A moment early: nothing. At the time: the capabilities again.
            assert!(f.step(now + w - 1, FetchInput::Tick).is_empty());
            now += w;
            let out = f.step(now, FetchInput::Tick);
            assert_eq!(
                fetches(&out),
                vec![(FetchKind::Capabilities, "/api/0.9/action/capabilities")]
            );
        }
    }
}

/// `I:861-866`: a failure of the applicabilities fetch retries from the
/// capabilities, and the curve carries on from the earlier failures; a success
/// resets nothing until the invoker is ready.
#[test]
fn fetch_an_applicabilities_failure_retries_from_the_capabilities() {
    let mut f = FetchState::new(ENDPOINT, Draw::High);
    f.step(0, FetchInput::Start);
    f.step(
        0,
        fetched(FetchKind::Capabilities, ok(&mock_capabilities())),
    );
    let out = f.step(0, fetched(FetchKind::Applicabilities, status(503)));
    let (fail, delay) = failure_of(&out);
    assert_eq!(fail.path, "applicabilities");
    assert_eq!(fail.error, FetchError::Status(503));
    assert_eq!(delay, 2_000);
    let out = f.step(2_000, FetchInput::Tick);
    assert_eq!(fetches(&out)[0].0, FetchKind::Capabilities);
    // The capabilities answer, the applicabilities fail again: the second step
    // of the curve.
    f.step(
        2_000,
        fetched(FetchKind::Capabilities, ok(&mock_capabilities())),
    );
    let out = f.step(2_000, fetched(FetchKind::Applicabilities, status(503)));
    assert_eq!(failure_of(&out).1, 4_000);
    assert_eq!(f.retry_in(), 4_000);
}

/// `I:772-780`, `:889-894`: the error shapes. A non-200 is `{:status, code}`, a
/// transport error carries its reason, a 429 carries its `retry-after`; an
/// answer that is not a JSON object is a failure of its own.
#[test]
fn fetch_failures_carry_the_error_shape() {
    let cases: Vec<(FetchOutcome, FetchError)> = vec![
        (status(500), FetchError::Status(500)),
        (status(401), FetchError::Status(401)),
        (status(404), FetchError::Status(404)),
        (
            FetchOutcome::Transport("timeout".into()),
            FetchError::Transport("timeout".into()),
        ),
        (
            FetchOutcome::Response {
                status: 429,
                headers: h("retry-after", "3"),
                body: String::new(),
            },
            FetchError::RateLimited(3_000),
        ),
        (
            FetchOutcome::Response {
                status: 429,
                headers: vec![],
                body: String::new(),
            },
            FetchError::RateLimited(0),
        ),
        (
            FetchOutcome::Response {
                status: 200,
                headers: vec![],
                body: "<html>proxy</html>".into(),
            },
            FetchError::BadBody,
        ),
        (
            FetchOutcome::Response {
                status: 200,
                headers: vec![],
                body: "[]".into(),
            },
            FetchError::BadBody,
        ),
    ];
    for (outcome, want) in cases {
        let mut f = FetchState::new(ENDPOINT, Draw::Low);
        f.step(0, FetchInput::Start);
        let out = f.step(0, fetched(FetchKind::Capabilities, outcome.clone()));
        let (fail, _) = failure_of(&out);
        assert_eq!(fail.error, want, "{outcome:?}");
        assert!(!f.is_ready());
    }
}

/// `I:889-894`, `T/invoker_fetch_retry_test.exs:54-68`: a 429's `retry-after` is
/// the floor of the next wait: 3 s here, past the longest first retry of 2 s.
/// It is also clamped (`CM:1063-1071`), and it does not feed the curve.
#[test]
fn fetch_a_429_retry_after_is_the_floor() {
    let mut f = FetchState::new(ENDPOINT, Draw::Low);
    f.step(0, FetchInput::Start);
    let limited = FetchOutcome::Response {
        status: 429,
        headers: h("Retry-After", "3"),
        body: String::new(),
    };
    let out = f.step(0, fetched(FetchKind::Capabilities, limited));
    let (fail, delay) = failure_of(&out);
    assert_eq!(fail.error, FetchError::RateLimited(3_000));
    assert_eq!(delay, 3_001, "3000 plus the lowest draw of the spread");
    assert_eq!(f.retry_in(), 2_000, "the curve ignores the floor");

    let mut f = FetchState::new(ENDPOINT, Draw::Low);
    f.step(0, FetchInput::Start);
    let huge = FetchOutcome::Response {
        status: 429,
        headers: h("retry-after", "86400"),
        body: String::new(),
    };
    let (_, delay) = failure_of(&f.step(0, fetched(FetchKind::Capabilities, huge)));
    assert_eq!(delay, 300_001);
}

/// A success after failures: ready, and the curve is back at the seed for a
/// later `Start` (a new fetch process begins from `__initial_retry__`, `I:837`).
#[test]
fn fetch_recovers_after_failures() {
    let mut f = FetchState::new(ENDPOINT, Draw::High);
    f.step(0, FetchInput::Start);
    f.step(0, fetched(FetchKind::Capabilities, status(500)));
    f.step(2_000, FetchInput::Tick);
    f.step(
        2_000,
        fetched(FetchKind::Capabilities, ok(&mock_capabilities())),
    );
    let out = f.step(
        2_001,
        fetched(FetchKind::Applicabilities, ok(&applicabilities())),
    );
    assert!(matches!(out[0], FetchOutput::Ready(_)));
    assert_eq!(f.retry_in(), 1_000);
}

/// `I:790-797`, `T/invoker_fetch_retry_test.exs:135-144`: a second `:ready`
/// while a fetch or a retry is pending neither fetches again nor replaces the
/// pending retry; nor does one after the invoker is ready.
#[test]
fn fetch_a_second_start_does_nothing() {
    let mut f = FetchState::new(ENDPOINT, Draw::High);
    f.step(0, FetchInput::Start);
    assert!(f.step(1, FetchInput::Start).is_empty(), "while fetching");
    f.step(1, fetched(FetchKind::Capabilities, status(500)));
    let due = f.next_wakeup();
    assert!(f.step(2, FetchInput::Start).is_empty(), "while waiting");
    assert_eq!(f.next_wakeup(), due);
    f.step(due.expect("due"), FetchInput::Tick);
    f.step(
        5_000,
        fetched(FetchKind::Capabilities, ok(&mock_capabilities())),
    );
    f.step(
        5_000,
        fetched(FetchKind::Applicabilities, ok(&applicabilities())),
    );
    assert!(f.step(6_000, FetchInput::Start).is_empty(), "when ready");
}

/// `I:172-184`, `T/invoker_fetch_retry_test.exs:146-167`: leaving `:ready`
/// cancels the pending fetch or retry; the next `:ready` fetches again, from the
/// seed. An answer to the cancelled fetch is ignored.
#[test]
fn fetch_cancel_stops_it_and_the_next_start_begins_again() {
    let mut f = FetchState::new(ENDPOINT, Draw::High);
    f.step(0, FetchInput::Start);
    f.step(0, fetched(FetchKind::Capabilities, status(500)));
    assert_eq!(f.retry_in(), 2_000);
    assert!(f.step(10, FetchInput::Cancel).is_empty());
    assert_eq!(f.next_wakeup(), None, "the retry is gone");
    assert!(f.step(100_000, FetchInput::Tick).is_empty());
    // A late answer to the cancelled request is dropped.
    assert!(
        f.step(
            100_001,
            fetched(FetchKind::Capabilities, ok(&mock_capabilities()))
        )
        .is_empty()
    );
    assert!(!f.is_ready());
    let out = f.step(200_000, FetchInput::Start);
    assert_eq!(fetches(&out).len(), 1);
    assert_eq!(f.retry_in(), 1_000, "from the seed");
    // Cancel in flight too.
    f.step(200_001, FetchInput::Cancel);
    assert!(
        f.step(
            200_002,
            fetched(FetchKind::Capabilities, ok(&mock_capabilities()))
        )
        .is_empty()
    );
}

/// An answer for the request that is not the one in flight is ignored.
#[test]
fn fetch_a_stray_answer_is_ignored() {
    let mut f = FetchState::new(ENDPOINT, Draw::Low);
    f.step(0, FetchInput::Start);
    assert!(
        f.step(
            1,
            fetched(FetchKind::Applicabilities, ok(&applicabilities()))
        )
        .is_empty()
    );
    assert!(!f.is_ready());
}

fn ready_machine() -> FetchState {
    let mut f = FetchState::new(ENDPOINT, Draw::Low);
    f.step(0, FetchInput::Start);
    f.step(
        0,
        fetched(FetchKind::Capabilities, ok(&mock_capabilities())),
    );
    f.step(
        0,
        fetched(FetchKind::Applicabilities, ok(&applicabilities())),
    );
    assert!(f.is_ready());
    f
}

/// `I:239-240`, `:366-381`, `T/invoker_test.exs:4-26`: `configChanged` refetches
/// both bodies and the new registration replaces the old.
#[test]
fn fetch_config_changed_refetches_and_replaces() {
    let mut f = ready_machine();
    let out = f.step(100, FetchInput::ConfigChanged);
    assert_eq!(
        fetches(&out),
        vec![(FetchKind::Capabilities, "/api/0.9/action/capabilities")]
    );
    let new_caps = json!({"additional_capability": {}, "ExecuteCommand": {}});
    let out = f.step(101, fetched(FetchKind::Capabilities, ok(&new_caps)));
    assert_eq!(fetches(&out)[0].0, FetchKind::Applicabilities);
    let out = f.step(
        102,
        fetched(FetchKind::Applicabilities, ok(&applicabilities())),
    );
    assert_eq!(
        out,
        vec![FetchOutput::Reconfigured(Registration {
            capabilities: new_caps,
            applicabilities: applicabilities(),
        })]
    );
    assert!(f.is_ready());
}

/// `I:375-380`, `T/invoker_fetch_retry_test.exs:203-214`: when the refetch fails
/// the invoker keeps what it had, says why, and does not retry; a 429 reports the
/// advertised wait.
#[test]
fn fetch_a_failed_refetch_keeps_the_old_and_does_not_retry() {
    let mut f = ready_machine();
    f.step(100, FetchInput::ConfigChanged);
    let limited = FetchOutcome::Response {
        status: 429,
        headers: h("retry-after", "2"),
        body: String::new(),
    };
    let out = f.step(101, fetched(FetchKind::Capabilities, limited));
    assert_eq!(
        out,
        vec![FetchOutput::Log(FetchLog::ReconfigureFailed(
            FetchFailure {
                path: "capabilities",
                error: FetchError::RateLimited(2_000)
            }
        ))]
    );
    assert!(f.is_ready(), "still ready");
    assert_eq!(f.next_wakeup(), None, "no retry is scheduled");
    assert!(f.step(1_000_000, FetchInput::Tick).is_empty());
    // And a later configChanged works again.
    assert_eq!(
        fetches(&f.step(1_000_001, FetchInput::ConfigChanged)).len(),
        1
    );
}

/// A `configChanged` while not ready, or while a refetch runs, does nothing but
/// say so.
#[test]
fn fetch_config_changed_when_it_cannot_refetch_is_ignored() {
    let mut f = FetchState::new(ENDPOINT, Draw::Low);
    let out = f.step(0, FetchInput::ConfigChanged);
    assert_eq!(out, vec![FetchOutput::Log(FetchLog::ConfigChangedIgnored)]);
    let mut f = ready_machine();
    f.step(1, FetchInput::ConfigChanged);
    let out = f.step(2, FetchInput::ConfigChanged);
    assert_eq!(out, vec![FetchOutput::Log(FetchLog::ConfigChangedIgnored)]);
}

// ----------------------------------------------- the fetch feeds the invoker --

/// Assertion 32 wired: the capabilities the fetch machine delivers set the
/// request defaults of the invoker. The mock's default `timeout` of 60000 is
/// read as SECONDS (`I:417-421`, `:388-391`): the wire carries 60_000_000 ms.
/// A caller's own timeout overrides it.
#[test]
fn a32_fetched_capabilities_become_the_request_defaults() {
    let mut f = FetchState::new(ENDPOINT, Draw::Low);
    f.step(0, FetchInput::Start);
    f.step(
        0,
        fetched(FetchKind::Capabilities, ok(&mock_capabilities())),
    );
    let out = f.step(
        0,
        fetched(FetchKind::Applicabilities, ok(&applicabilities())),
    );
    let FetchOutput::Ready(reg) = &out[0] else {
        panic!("{out:?}")
    };

    let mut st = InvokeState::new();
    st.load_capabilities(&reg.capabilities);
    assert_eq!(
        st.capability_defaults("ExecuteCommand")
            .map(|d| Value::Object(d.clone())),
        Some(json!({"timeout": 60000}))
    );
    let submit = |st: &mut InvokeState, call: u64, r: CallRequest| -> SubmitFrame {
        let out = st.step(0, Input::Call(r));
        let Some(Output::Send {
            frame: Frame::Submit(s),
            ..
        }) = out.into_iter().find(|o| matches!(o, Output::Send { .. }))
        else {
            panic!("call {call} sent nothing")
        };
        s
    };
    let handler = reg.handler_ids().remove(0);
    let base = CallRequest::new(1, "ticket", handler.clone(), "ExecuteCommand")
        .param("command", json!("ls"));
    let s = submit(&mut st, 1, base);
    assert_eq!(s.timeout_ms, 60_000_000);
    assert_eq!(s.parameters["timeout"], json!(60_000_000));
    let own = CallRequest::new(2, "ticket", handler, "ExecuteCommand")
        .param("command", json!("ls"))
        .with_timeout(ogar_action_handler::invoke_state::TimeoutArg::Int(5));
    assert_eq!(submit(&mut st, 2, own).timeout_ms, 5_000);

    // A reconfigure replaces the defaults, including dropping the old ones.
    st.load_capabilities(&json!({"ExecuteCommand": {}}));
    assert!(st.capability_defaults("ExecuteCommand").is_none());
}
