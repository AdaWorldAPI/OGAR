//! The crate against the gateway's own published specification.
//!
//! The reference files are copied unchanged into `tests/fixtures/hiro_public_specs/`
//! (see `SOURCES.md` there):
//!
//! * `action-ws.yaml` (Swagger 2.0, the action service "aapi" as deployed),
//!   cited below as `YAML:<line>`;
//! * `action.yaml` (REST: capabilities and applicabilities);
//! * `api-version.json` (the catalog).
//!
//! Source ranking. The published specification and catalog are the reference for
//! the documented contract. graph_conn (the Elixir client) is the witness of what
//! a client does; the other test files pin it. The YAML describes the service as
//! deployed and lists its own departures from an older specification that was not
//! read. The captured files have known defects (HIRO_PUBLIC_API_SPECS.md, section
//! 5); the tests below name the ones they run into.
//!
//! No YAML parser is a dependency of this workspace, so the schema facts are
//! transcribed into [`SCHEMAS`], one row per message definition, each with its
//! YAML line range. `schema_table_matches_the_yaml` reads the fixture and checks
//! every row against it (the range, the `required:` line and the property names),
//! so a drift in the table or in the file fails a test.
//!
//! Close codes (YAML:40-45: 1000 missing pongs, 1001 quorum lost, 1009 frame over
//! `hello.max_frame_bytes`). This crate has no notion of a close code: closing a
//! socket belongs to the transport crate. The nearest thing here is the mock
//! gateway's `MockError::FrameTooBig`, pinned below without a code. No code was
//! added.
//!
//! `setSettings` (YAML:243-258, 426-435) is deprecated and client to aapi only.
//! The crate never sends it (`clientHello.settings` replaces it) and a handler
//! session that is sent one reports `UnknownType`, which is pinned below.

use std::fs;
use std::path::PathBuf;

use ogar_action_handler::discovery::{Registration, parse_versions};
use ogar_action_handler::invoke_session::{Effect as InvokeEffect, InvokeSession};
use ogar_action_handler::invoke_state::{CallRequest, InvokeError, TimeoutArg};
use ogar_action_handler::mock_gateway::{MockConfig, MockError, MockGateway, Side};
use ogar_action_handler::session::{Effect, Session};
use ogar_action_handler::submit_state::{Config, ExecResult};
use ogar_action_handler::wire::{
    self, ActionWsFrame, ClientHello, DEFAULT_MAX_FRAME_BYTES, Hello, WireError,
};
use ogar_from_schema::action_ws::{
    ACTION_WS_DOCS_PATH, ACTION_WS_SUBPROTOCOL, NegativeAcknowledged, auth_subprotocol,
    docs_subprotocols,
};
use serde_json::{Value, json};

// ------------------------------------------------------------ the table --

/// The JSON types the schema uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ty {
    Str,
    Int,
    Obj,
}

impl Ty {
    fn holds(self, v: &Value) -> bool {
        match self {
            Self::Str => v.is_string(),
            Self::Int => v.is_i64() || v.is_u64(),
            Self::Obj => v.is_object(),
        }
    }
}

/// One message definition of `action-ws.yaml`.
struct Schema {
    name: &'static str,
    /// First and last line of the definition (1-based, inclusive).
    lines: (usize, usize),
    /// The line holding `required: [...]`.
    required_line: usize,
    /// The required properties, in the order the YAML lists them.
    required: &'static [(&'static str, Ty)],
    /// Every property the definition lists (required first).
    properties: &'static [&'static str],
}

const SCHEMAS: &[Schema] = &[
    Schema {
        name: "Hello",
        lines: (295, 318),
        required_line: 297,
        required: &[
            ("type", Ty::Str),
            ("host", Ty::Str),
            ("server_version", Ty::Str),
            ("client_id", Ty::Str),
            ("max_frame_bytes", Ty::Int),
        ],
        properties: &[
            "type",
            "host",
            "server_version",
            "client_id",
            "max_frame_bytes",
        ],
    },
    Schema {
        name: "SubmitActionInbound",
        lines: (321, 338),
        required_line: 323,
        required: &[
            ("type", Ty::Str),
            ("id", Ty::Str),
            ("handler", Ty::Str),
            ("capability", Ty::Str),
            ("parameters", Ty::Obj),
            ("timeout", Ty::Int),
        ],
        properties: &[
            "type",
            "id",
            "handler",
            "capability",
            "parameters",
            "timeout",
        ],
    },
    Schema {
        name: "SubmitActionOutbound",
        lines: (341, 356),
        required_line: 343,
        required: &[
            ("type", Ty::Str),
            ("id", Ty::Str),
            ("capability", Ty::Str),
            ("parameters", Ty::Obj),
            ("timeout", Ty::Int),
        ],
        properties: &["type", "id", "capability", "parameters", "timeout"],
    },
    Schema {
        name: "SendActionResult",
        lines: (358, 369),
        required_line: 360,
        required: &[("type", Ty::Str), ("id", Ty::Str), ("result", Ty::Str)],
        properties: &["type", "id", "result"],
    },
    Schema {
        name: "Acknowledged",
        lines: (373, 382),
        required_line: 375,
        required: &[("type", Ty::Str), ("id", Ty::Str)],
        properties: &["type", "id"],
    },
    Schema {
        name: "NegativeAcknowledged",
        lines: (384, 401),
        required_line: 386,
        required: &[
            ("type", Ty::Str),
            ("id", Ty::Str),
            ("code", Ty::Int),
            ("message", Ty::Str),
        ],
        properties: &["type", "id", "code", "message"],
    },
    Schema {
        name: "ClientHello",
        lines: (403, 424),
        required_line: 405,
        required: &[("type", Ty::Str)],
        properties: &["type", "client", "settings"],
    },
    Schema {
        name: "SetSettings",
        lines: (426, 435),
        required_line: 428,
        required: &[("type", Ty::Str), ("settings", Ty::Obj)],
        properties: &["type", "settings"],
    },
];

fn schema(name: &str) -> &'static Schema {
    SCHEMAS
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no schema row for {name}"))
}

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hiro_public_specs")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Asserts `frame` is a valid instance of `schema` as far as the schema goes:
/// every required property is there with its JSON type, `type` is the message's
/// own name (YAML:377-380 types it as an unrestricted string, so the check is
/// ours), and any property the schema does not list is exactly one of `extras`.
fn assert_conforms(schema: &Schema, wire_type: &str, frame: &Value, extras: &[&str]) {
    let obj = frame
        .as_object()
        .unwrap_or_else(|| panic!("{}: not an object: {frame}", schema.name));
    for (field, ty) in schema.required {
        let v = obj
            .get(*field)
            .unwrap_or_else(|| panic!("{}: required `{field}` missing in {frame}", schema.name));
        assert!(
            ty.holds(v),
            "{}: `{field}` is not {ty:?} in {frame}",
            schema.name
        );
    }
    assert_eq!(obj["type"], wire_type, "{}: wrong type", schema.name);
    let mut unlisted: Vec<&str> = obj
        .keys()
        .map(String::as_str)
        .filter(|k| !schema.properties.contains(k))
        .collect();
    unlisted.sort_unstable();
    let mut expected = extras.to_vec();
    expected.sort_unstable();
    assert_eq!(
        unlisted, expected,
        "{}: properties the schema does not list",
        schema.name
    );
}

fn parse(text: &str) -> Value {
    serde_json::from_str(text).expect("the crate emits JSON")
}

fn sends(fx: &[Effect]) -> Vec<Value> {
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
    Session::new(Config::default()).expect("default config")
}

fn invoke_sends(fx: &[InvokeEffect]) -> Vec<Value> {
    fx.iter()
        .filter_map(|e| match e {
            InvokeEffect::Send { text, .. } => Some(parse(text)),
            _ => None,
        })
        .collect()
}

/// The schema's own examples (YAML:261-294): the submit the gateway forwards,
/// with exactly the required fields of `SubmitActionOutbound`.
fn outbound_submit_minimal() -> String {
    json!({
        "type": "submitAction",
        "id": "id1",
        "capability": "ExecuteCommand",
        "parameters": {},
        "timeout": 30000
    })
    .to_string()
}

/// The 1-based last line of the definition that starts at line `start`: the
/// last line before the next blank line, comment or line at indentation of two
/// spaces or less, or the end of the file.
fn definition_end(lines: &[&str], start: usize) -> usize {
    let mut end = start;
    for (i, l) in lines.iter().enumerate().skip(start) {
        let indent = l.len() - l.trim_start().len();
        if l.trim().is_empty() || indent <= 2 {
            break;
        }
        end = i + 1;
    }
    end
}

fn ty_of(name: &str) -> Ty {
    match name {
        "string" => Ty::Str,
        "integer" => Ty::Int,
        "object" => Ty::Obj,
        other => panic!("a type the table does not know: {other}"),
    }
}

/// The type of property `field` of the definition spanning `start..=end`: its
/// inline `type:`, or the `type:` of the local definition its `$ref` names.
fn property_type(lines: &[&str], start: usize, end: usize, field: &str) -> Ty {
    let key = format!("      {field}:");
    let at = lines[start - 1..end]
        .iter()
        .position(|l| *l == key)
        .unwrap_or_else(|| panic!("property {field} not found"))
        + start;
    // Its block: the following lines indented deeper than six spaces.
    let block: Vec<&str> = lines[at..end]
        .iter()
        .copied()
        .take_while(|l| l.starts_with("        "))
        .collect();
    if let Some(t) = block.iter().find_map(|l| l.strip_prefix("        type: ")) {
        return ty_of(t.trim());
    }
    let target = block
        .iter()
        .find_map(|l| l.strip_prefix("        $ref: \"#/definitions/"))
        .and_then(|r| r.strip_suffix('"'))
        .unwrap_or_else(|| panic!("property {field} has neither type nor $ref"));
    let def = lines
        .iter()
        .position(|l| *l == format!("  {target}:"))
        .unwrap_or_else(|| panic!("definition {target} not found"));
    let t = lines[def + 1..]
        .iter()
        .take_while(|l| l.starts_with("   "))
        .find_map(|l| l.strip_prefix("    type: "))
        .unwrap_or_else(|| panic!("definition {target} has no type"));
    ty_of(t.trim())
}

// -------------------------------------------- the table against the YAML --

#[test]
fn schema_table_matches_the_yaml() {
    let yaml = fixture("action-ws.yaml");
    let lines: Vec<&str> = yaml.lines().collect();
    let at = |n: usize| lines[n - 1];
    for s in SCHEMAS {
        assert_eq!(at(s.lines.0), format!("  {}:", s.name), "{} starts", s.name);
        // The real end of the definition, from the file: the last content line
        // before the next line at the definitions' indentation (a sibling key or
        // a comment) or a blank line, or the end of the file.
        let end = definition_end(&lines, s.lines.0);
        assert_eq!(s.lines.1, end, "{}: recorded range end", s.name);
        let names: Vec<&str> = s.required.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            at(s.required_line).trim(),
            format!("required: [{}]", names.join(", ")),
            "{}: required line",
            s.name
        );
        // Top-level properties sit at six spaces; nested ones (ClientHello's
        // `client.app`) sit deeper and are not counted.
        let mut found: Vec<&str> = lines[s.lines.0 - 1..end]
            .iter()
            .filter(|l| l.starts_with("      ") && !l.starts_with("       "))
            .filter_map(|l| l.trim_start().strip_suffix(':'))
            .collect();
        let mut listed = s.properties.to_vec();
        found.sort_unstable();
        listed.sort_unstable();
        assert_eq!(found, listed, "{}: property names", s.name);
        // The type of every required property, from the file.
        for (field, ty) in s.required {
            assert_eq!(
                property_type(&lines, s.lines.0, end, field),
                *ty,
                "{}: type of `{field}`",
                s.name
            );
        }
    }
    // The header says the definitions are these and no others of the eight.
    for name in [
        "Hello",
        "SubmitActionOutbound",
        "SubmitActionInbound",
        "SendActionResult",
        "Acknowledged",
        "NegativeAcknowledged",
        "ClientHello",
        "SetSettings",
    ] {
        assert!(SCHEMAS.iter().any(|s| s.name == name), "{name}");
    }
}

// ------------------------------------------------ what we send: handler --

/// `Acknowledged` (YAML:373-382, required `type`, `id`). The handler's ack of a
/// submit is `{type, id, code: 200, message: ""}` (graph_conn `handler.ex`).
/// `code` and `message` are NOT in the published schema: aapi dropped them
/// (YAML:15-16, 370-372) because no consumer reads them off a positive ack. We
/// still send both, as graph_conn does. This test names the deviation so it is
/// visible and intentional.
#[test]
fn handler_ack_has_the_required_fields_and_two_extras() {
    let mut s = session();
    let fx = s.on_text(0, &outbound_submit_minimal()).expect("decodes");
    let frames = sends(&fx);
    assert_eq!(frames.len(), 1);
    assert_conforms(
        schema("Acknowledged"),
        "acknowledged",
        &frames[0],
        &["code", "message"],
    );
    assert_eq!(frames[0]["id"], "id1");
    assert_eq!(frames[0]["code"], 200);
    assert_eq!(frames[0]["message"], "");
}

/// `SendActionResult` (YAML:358-369, required `type`, `id`, `result`).
#[test]
fn handler_send_action_result_has_the_required_fields() {
    let mut s = session();
    s.on_text(0, &outbound_submit_minimal()).expect("decodes");
    let fx = s.on_finished(5, "id1", ExecResult::Success(r#"{"ok":true}"#.to_owned()));
    let frames = sends(&fx);
    assert_eq!(frames.len(), 1);
    assert_conforms(
        schema("SendActionResult"),
        "sendActionResult",
        &frames[0],
        &[],
    );
    assert_eq!(frames[0]["id"], "id1");
}

/// `result` is `type: string` (YAML:277-281). A result that is a JSON object is
/// sent as the string holding its JSON text, never as an object.
#[test]
fn send_action_result_result_is_a_json_string_never_an_object() {
    for payload in [r#"{"ok":true}"#, "ok", "", r#"[1,2]"#] {
        let mut s = session();
        s.on_text(0, &outbound_submit_minimal()).expect("decodes");
        let fx = s.on_finished(5, "id1", ExecResult::Success(payload.to_owned()));
        let frame = &sends(&fx)[0];
        assert!(
            frame["result"].is_string(),
            "result for {payload:?}: {frame}"
        );
        assert_eq!(frame["result"], payload);
    }
    // Also an error result and the frame-too-big replacement.
    let mut s = session();
    s.set_max_frame_bytes(Some(200));
    s.on_text(0, &outbound_submit_minimal()).expect("decodes");
    let big = "x".repeat(500);
    let fx = s.on_finished(5, "id1", ExecResult::Success(big));
    let frame = &sends(&fx)[0];
    assert!(frame["result"].is_string());
    let inner: Value = parse(frame["result"].as_str().unwrap());
    assert_eq!(inner["error"]["action_status"], 59);
}

/// `NegativeAcknowledged` (YAML:384-401, required `type`, `id`, `code`,
/// `message`). The state machine never emits a NACK (graph_conn's handler sends
/// none for a submit), but the session adapter encodes one on request, so the
/// encoder is pinned here. It writes all four required fields and nothing else.
#[test]
fn negative_acknowledged_the_crate_encodes_has_the_required_fields() {
    let text = wire::encode(
        &NegativeAcknowledged {
            id: "id1".to_owned(),
            code: 400,
            message: "bad request".to_owned(),
        }
        .into(),
    );
    assert_conforms(
        schema("NegativeAcknowledged"),
        "negativeAcknowledged",
        &parse(&text),
        &[],
    );
}

// ---------------------------------------------- what we send: clientHello --

/// `ClientHello` (YAML:403-424, required `type` only; no `id`).
#[test]
fn client_hello_has_only_what_the_schema_lists() {
    let full = wire::encode(&ActionWsFrame::ClientHello(ClientHello {
        app: Some("hiro-actionhandler".to_owned()),
        version: Some("2.5.3".to_owned()),
        redelivery: Some(false),
    }));
    let v = parse(&full);
    assert_conforms(schema("ClientHello"), "clientHello", &v, &[]);
    assert_eq!(
        v["client"],
        json!({"app": "hiro-actionhandler", "version": "2.5.3"})
    );
    assert!(
        v.get("id").is_none(),
        "a clientHello carries no id (YAML:223-225)"
    );
}

/// `settings.redelivery` is a boolean (YAML:286-294): carried when configured,
/// and `settings` is omitted when not.
#[test]
fn client_hello_carries_redelivery_when_set_and_omits_settings_when_not() {
    for flag in [false, true] {
        let v = parse(&wire::encode(&ActionWsFrame::ClientHello(ClientHello {
            redelivery: Some(flag),
            ..ClientHello::default()
        })));
        assert_eq!(v["settings"], json!({"redelivery": flag}));
        assert!(v["settings"]["redelivery"].is_boolean());
    }
    let bare = parse(&wire::encode(&ActionWsFrame::ClientHello(
        ClientHello::default(),
    )));
    assert_eq!(bare, json!({"type": "clientHello"}));
    assert!(bare.get("settings").is_none());
}

/// A clientHello the schema allows round-trips through the decoder, including the
/// 64 byte limits: the schema says an over-long `app` or `version` is ignored by
/// aapi (YAML:409-418). The crate does not truncate or refuse; that is the
/// service's rule and the crate is more lenient on its own side. Not pinned
/// further.
#[test]
fn client_hello_round_trips() {
    let hello = ClientHello {
        app: Some("hiro-actionhandler".to_owned()),
        version: Some("2.5.3".to_owned()),
        redelivery: Some(true),
    };
    let text = wire::encode(&ActionWsFrame::ClientHello(hello.clone()));
    assert_eq!(wire::decode(&text), Ok(ActionWsFrame::ClientHello(hello)));
}

/// `setSettings` is deprecated and client to aapi only (YAML:236-258). The crate
/// does not send it. A handler session that is sent one is told it is an unknown
/// type and does nothing.
#[test]
fn set_settings_is_not_sent_and_not_a_frame_the_crate_knows() {
    let frame = json!({"type": "setSettings", "settings": {"redelivery": false}}).to_string();
    assert_eq!(
        wire::decode(&frame),
        Err(WireError::UnknownType("setSettings".to_owned()))
    );
    let mut s = session();
    assert!(s.on_text(0, &frame).is_err());
}

// ------------------------------------------------- what we send: invoker --

fn invoke_call() -> (InvokeSession, Value) {
    let mut s = InvokeSession::new();
    let mut req = CallRequest::new(1, "ticket-1", "handlerId", "ExecuteCommand");
    req.timeout = TimeoutArg::Int(30);
    let fx = s.call(0, req);
    let frames = invoke_sends(&fx);
    assert_eq!(frames.len(), 1);
    (s, frames[0].clone())
}

/// `SubmitActionInbound` (YAML:321-338): the invoker's submit to aapi keeps
/// `handler`. No extra fields.
#[test]
fn invoker_submit_action_has_the_inbound_required_fields() {
    let (_, frame) = invoke_call();
    assert_conforms(schema("SubmitActionInbound"), "submitAction", &frame, &[]);
    assert_eq!(frame["handler"], "handlerId");
    assert_eq!(frame["capability"], "ExecuteCommand");
}

/// `timeout` is milliseconds (YAML:282-285). The invoker takes seconds from its
/// caller and writes `30` as the integer `30000`.
#[test]
fn invoker_timeout_is_written_in_milliseconds_as_an_integer() {
    let (_, frame) = invoke_call();
    assert_eq!(frame["timeout"], json!(30_000));
    assert!(frame["timeout"].is_i64() || frame["timeout"].is_u64());
}

/// `Acknowledged` again, from the invoker: its ack of a received result is
/// `{type, id, code: 200}` (graph_conn `invoker.ex:217`). `code` is not in the
/// published schema; named here as the intentional extra. There is no `message`.
#[test]
fn invoker_result_ack_has_the_required_fields_and_one_extra() {
    let (mut s, submit) = invoke_call();
    let id = submit["id"].as_str().unwrap().to_owned();
    let result = json!({"type": "sendActionResult", "id": id, "result": "{\"ok\":true}"});
    let fx = s.on_text(10, &result.to_string()).expect("decodes");
    let frames = invoke_sends(&fx);
    assert_eq!(frames.len(), 1, "the result is acked: {fx:?}");
    assert_conforms(
        schema("Acknowledged"),
        "acknowledged",
        &frames[0],
        &["code"],
    );
    assert_eq!(frames[0]["id"].as_str(), Some(id.as_str()));
    assert_eq!(frames[0]["code"], 200);
}

// ----------------------------------------------------- what we accept --

/// `SubmitActionOutbound` (YAML:341-356): exactly the required fields, with the
/// schema's example values and NO `handler`. The submit is claimed, acked and run
/// once; the same frame again is acked and not run again.
#[test]
fn outbound_submit_without_handler_is_acked_and_run_once() {
    let frame = outbound_submit_minimal();
    let mut s = session();
    let fx = s.on_text(0, &frame).expect("decodes");
    assert_eq!(runs(&fx), 1);
    assert_eq!(sends(&fx).len(), 1, "one ack");
    match &fx[1] {
        Effect::Run {
            id,
            capability,
            parameters,
        } => {
            assert_eq!(id, "id1");
            assert_eq!(capability, "ExecuteCommand");
            assert!(parameters.is_empty());
        }
        other => panic!("expected a run, got {other:?}"),
    }
    let again = s.on_text(1, &frame).expect("decodes");
    assert_eq!(runs(&again), 0, "a repeat is not run again");
    assert_eq!(sends(&again).len(), 1, "but it is acked");
    match wire::decode(&frame) {
        Ok(ActionWsFrame::SubmitAction(m)) => assert_eq!(m.handler, ""),
        other => panic!("{other:?}"),
    }
}

/// More lenient than the schema: `timeout` is required (YAML:343) but a submit
/// without one is accepted and runs under the session's default deadline. graph_conn
/// takes it that way.
#[test]
fn leniency_submit_without_timeout_is_accepted_with_the_default_deadline() {
    let frame = json!({"type": "submitAction", "id": "id1", "capability": "ExecuteCommand", "parameters": {}})
        .to_string();
    let mut s = session();
    let fx = s.on_text(0, &frame).expect("decodes");
    assert_eq!(runs(&fx), 1);
    assert_eq!(s.next_wakeup(), Some(60_000));
}

/// More lenient than the schema: properties the schema does not list (the
/// inbound `handler` on an outbound frame, `host` and the like on a hello) are
/// read or ignored, not refused. aapi's own inbound frame carries `handler`
/// (YAML:323); a handler session accepts that too.
#[test]
fn leniency_extra_properties_are_ignored_and_handler_is_read_when_present() {
    let frame = json!({
        "type": "submitAction", "id": "id1", "handler": "handlerId",
        "capability": "ExecuteCommand", "parameters": {}, "timeout": 30000,
        "somethingNew": true
    })
    .to_string();
    match wire::decode(&frame) {
        Ok(ActionWsFrame::SubmitAction(m)) => assert_eq!(m.handler, "handlerId"),
        other => panic!("{other:?}"),
    }
    let mut s = session();
    let fx = s.on_text(0, &frame).expect("decodes");
    assert_eq!(runs(&fx), 1);
}

/// `timeout` is milliseconds (YAML:282-285): 30000 is a 30,000 ms deadline. The
/// execution is still open at 29,999 ms and timed out at 30,000 ms (the
/// `action_status` 13 result of graph_conn).
#[test]
fn submit_timeout_30000_is_a_30000_ms_deadline() {
    let mut s = session();
    s.on_text(0, &outbound_submit_minimal()).expect("decodes");
    assert_eq!(s.next_wakeup(), Some(30_000));
    assert!(sends(&s.on_tick(29_999)).is_empty());
    let fx = s.on_tick(30_000);
    let frames = sends(&fx);
    assert_eq!(frames.len(), 1, "the timeout result is sent: {fx:?}");
    assert_conforms(
        schema("SendActionResult"),
        "sendActionResult",
        &frames[0],
        &[],
    );
    let inner = parse(frames[0]["result"].as_str().expect("a string"));
    assert_eq!(inner["error"]["action_status"], 13);
}

/// `Hello` (YAML:295-318) with all five fields and the schema's examples. The
/// frame decodes, and the session's frame limit becomes `max_frame_bytes`
/// (YAML:313-319: "largest inbound frame this aapi accepts").
#[test]
fn hello_with_all_five_fields_sets_the_frame_limit() {
    let frame = json!({
        "type": "hello",
        "host": "action-api-0",
        "server_version": "1.0.18",
        "client_id": "ah_app_id_...",
        "max_frame_bytes": 4_194_304
    })
    .to_string();
    assert_eq!(
        wire::decode(&frame),
        Ok(ActionWsFrame::Hello(Hello {
            max_frame_bytes: Some(4_194_304)
        }))
    );
    let mut s = session();
    assert_eq!(s.state().max_frame_bytes(), DEFAULT_MAX_FRAME_BYTES);
    s.on_text(0, &frame).expect("decodes");
    assert_eq!(s.state().max_frame_bytes(), 4_194_304);
}

/// `max_frame_bytes` is required by the schema (YAML:297) but absent before aapi
/// 1.0.20, whose limit is 1,000,000 (YAML:316-319; HIRO_PUBLIC_API_SPECS.md
/// section 5). A `hello` without it decodes, and the limit is 1,000,000 again
/// even after an earlier hello advertised another.
#[test]
fn hello_without_max_frame_bytes_means_one_million() {
    let older = json!({
        "type": "hello", "host": "action-api-0", "server_version": "1.0.18",
        "client_id": "ah_app_id_..."
    })
    .to_string();
    assert_eq!(
        wire::decode(&older),
        Ok(ActionWsFrame::Hello(Hello {
            max_frame_bytes: None
        }))
    );
    assert_eq!(DEFAULT_MAX_FRAME_BYTES, 1_000_000);
    let mut s = session();
    s.on_text(0, r#"{"type":"hello","max_frame_bytes":4194304}"#)
        .expect("decodes");
    assert_eq!(s.state().max_frame_bytes(), 4_194_304);
    s.on_text(1, &older).expect("decodes");
    assert_eq!(s.state().max_frame_bytes(), 1_000_000);
}

/// `Acknowledged` with only `type` and `id` (YAML:373-382) stops the resend of a
/// result, as the handler's ack of that result. Without the ack the result is
/// sent again after the resend interval; with it, it is not.
#[test]
fn acknowledged_with_only_type_and_id_settles_the_result() {
    let finished = || {
        let mut s = session();
        s.on_text(0, &outbound_submit_minimal()).expect("decodes");
        s.on_finished(5, "id1", ExecResult::Success("{}".to_owned()));
        s
    };
    let mut unacked = finished();
    assert_eq!(
        sends(&unacked.on_tick(3_005)).len(),
        1,
        "resent without an ack"
    );
    let mut s = finished();
    let ack = json!({"type": "acknowledged", "id": "id1"}).to_string();
    assert!(s.on_text(10, &ack).expect("decodes").is_empty());
    assert!(
        sends(&s.on_tick(3_005)).is_empty(),
        "no resend after the ack"
    );
}

/// The same frame for the invoker: its call goes on to wait for the result.
#[test]
fn acknowledged_with_only_type_and_id_is_taken_by_the_invoker() {
    let (mut s, submit) = invoke_call();
    let id = submit["id"].as_str().unwrap();
    let ack = json!({"type": "acknowledged", "id": id}).to_string();
    let fx = s.on_text(10, &ack).expect("decodes");
    assert!(fx.is_empty());
    // After the ack the wait is the request timeout plus the grace, not the ack wait.
    assert_eq!(s.next_wakeup(), Some(10 + 30_000 + 1_000));
}

/// `NegativeAcknowledged` with `code` and `message` (YAML:384-401), the schema's
/// examples. A handler takes it as the end of the resend, as graph_conn does.
#[test]
fn negative_acknowledged_settles_a_handler_result() {
    let mut s = session();
    s.on_text(0, &outbound_submit_minimal()).expect("decodes");
    s.on_finished(5, "id1", ExecResult::Success("{}".to_owned()));
    let nack =
        json!({"type": "negativeAcknowledged", "id": "id1", "code": 400, "message": "bad request"})
            .to_string();
    assert!(s.on_text(10, &nack).expect("decodes").is_empty());
    assert!(
        sends(&s.on_tick(3_005)).is_empty(),
        "no resend after the nack"
    );
}

/// The same frame for the invoker ends the call with `Nack`, carrying `code` and
/// `message`.
#[test]
fn negative_acknowledged_ends_an_invoker_call_with_its_code_and_message() {
    let (mut s, submit) = invoke_call();
    let id = submit["id"].as_str().unwrap();
    let nack =
        json!({"type": "negativeAcknowledged", "id": id, "code": 400, "message": "bad request"})
            .to_string();
    let fx = s.on_text(10, &nack).expect("decodes");
    let done = fx
        .iter()
        .find_map(|e| match e {
            InvokeEffect::Complete(c) => Some(c),
            _ => None,
        })
        .expect("the call completes");
    assert_eq!(
        done.result,
        Err(InvokeError::Nack {
            code: Some(400),
            message: Some("bad request".to_owned())
        })
    );
}

/// `SendActionResult` (YAML:358-369) with the schema's example `result` "ok",
/// which is a string and not JSON text. After the ack, the invoker acks the
/// result and completes the call with that string as the value.
#[test]
fn result_frame_with_the_schema_example_is_accepted_by_the_invoker() {
    let (mut s, submit) = invoke_call();
    let id = submit["id"].as_str().unwrap();
    s.on_text(5, &json!({"type": "acknowledged", "id": id}).to_string())
        .expect("decodes");
    let frame = json!({"type": "sendActionResult", "id": id, "result": "ok"}).to_string();
    let fx = s.on_text(10, &frame).expect("decodes");
    assert_eq!(invoke_sends(&fx).len(), 1, "acked: {fx:?}");
    let done = fx
        .iter()
        .find_map(|e| match e {
            InvokeEffect::Complete(c) => Some(c),
            _ => None,
        })
        .expect("completed");
    assert_eq!(done.result, Ok(json!("ok")));
}

// ----------------------------------------------- post-ack re-delivery --

/// Re-delivery after an ack. The YAML names only the opt-out (YAML:286-294,
/// `settings.redelivery`: "false opts the connection out of aapi's post-ack
/// re-delivery") and does not say how a handler answers a duplicate. This is
/// our behaviour, as graph_conn does, and is NOT required by the specification.
/// The handler has acked, run and sent its result, and the peer has acked the
/// result. A re-delivered submit is answered from the stored result and the
/// capability does not run again.
#[test]
fn redelivery_after_ack_and_result_is_answered_from_the_stored_result() {
    let mut s = session();
    let frame = outbound_submit_minimal();
    let first = s.on_text(0, &frame).expect("decodes");
    assert_eq!(runs(&first), 1);
    let done = s.on_finished(5, "id1", ExecResult::Success(r#"{"n":1}"#.to_owned()));
    let result_frame = sends(&done).remove(0);
    s.on_text(8, &json!({"type": "acknowledged", "id": "id1"}).to_string())
        .expect("decodes");

    let again = s.on_text(20, &frame).expect("decodes");
    assert_eq!(runs(&again), 0, "not run again: {again:?}");
    let frames = sends(&again);
    assert_eq!(frames.len(), 2, "an ack and the stored result: {again:?}");
    assert_eq!(frames[0]["type"], "acknowledged");
    assert_eq!(frames[1], result_frame, "the same result, from the store");
}

// ------------------------------------------------------------ the mock --

/// The mock gateway's `hello` carries `type` and, when configured,
/// `max_frame_bytes` as an integer. It does NOT carry `host`, `server_version` or
/// `client_id`, which the published schema requires (YAML:297). The mock models
/// graph_conn's mock, whose `hello` is that older shape, and the crate's [`Hello`]
/// keeps only the field a client reads. Nothing in the crate sends a `hello` to a
/// real peer: the gateway does. So this is a known gap of the test double, pinned
/// so it is not mistaken for conformance.
#[test]
fn mock_hello_is_the_older_two_field_shape() {
    let mock = MockGateway::new(MockConfig {
        handler_max_frame_bytes: Some(4_194_304),
        ..MockConfig::default()
    });
    let v = parse(&mock.hello_text(Side::Handler));
    assert_eq!(v, json!({"type": "hello", "max_frame_bytes": 4_194_304}));
    let missing: Vec<&str> = schema("Hello")
        .required
        .iter()
        .map(|(n, _)| *n)
        .filter(|n| v.get(*n).is_none())
        .collect();
    assert_eq!(missing, ["host", "server_version", "client_id"]);
    // With no limit configured it is the shape of a service before 1.0.20 as far
    // as `max_frame_bytes` goes.
    let none = MockGateway::new(MockConfig::default());
    assert_eq!(
        parse(&none.hello_text(Side::Invoker)),
        json!({"type": "hello"})
    );
}

/// Close code 1009 (YAML:45): the nearest notion in this crate is the mock's
/// `MockError::FrameTooBig`, which carries sizes and no close code.
#[test]
fn oversize_frame_is_the_mocks_frame_too_big_without_a_close_code() {
    let mut mock = MockGateway::new(MockConfig {
        handler_max_frame_bytes: Some(100),
        ..MockConfig::default()
    });
    let big = json!({"type": "sendActionResult", "id": "x", "result": "y".repeat(300)}).to_string();
    match mock.on_handler_text(0, &big) {
        Err(MockError::FrameTooBig { bytes, limit, .. }) => {
            assert!(bytes > limit);
            assert_eq!(limit, 100);
        }
        other => panic!("{other:?}"),
    }
}

// -------------------------------------------------- api-version.json --

/// The catalog (`api-version.json`): `action-ws` has `endpoint`
/// `/api/action-ws/1.0/` and `protocols` `action-1.0.0`. The crate's constants
/// equal them, and the crate's version parser reads the catalog as fetched.
/// YAML:67 spells the connect path `/[connect]`, a placeholder (section 5 of
/// HIRO_PUBLIC_API_SPECS.md); the catalog's endpoint is the path to use.
#[test]
fn constants_equal_the_catalog() {
    let catalog: Value = serde_json::from_str(&fixture("api-version.json")).expect("JSON");
    let entry = &catalog["action-ws"];
    assert_eq!(entry["endpoint"], ACTION_WS_DOCS_PATH);
    assert_eq!(entry["protocols"], ACTION_WS_SUBPROTOCOL);
    assert_eq!(ACTION_WS_DOCS_PATH, "/api/action-ws/1.0/");
    assert_eq!(ACTION_WS_SUBPROTOCOL, "action-1.0.0");
    assert_eq!(entry["specs"], "action-ws.yaml");
    // The offer: the protocol and the token, as the portal shows both
    // (HIRO_PUBLIC_API_SPECS.md section 5; YAML:75 names only the token).
    assert_eq!(
        docs_subprotocols("T"),
        [ACTION_WS_SUBPROTOCOL.to_owned(), auth_subprotocol("T")]
    );
    assert_eq!(auth_subprotocol("T"), "token-T");

    let versions = parse_versions(&fixture("api-version.json")).expect("the catalog parses");
    let ws = versions.action_ws().expect("action-ws entry");
    assert_eq!(ws.path, ACTION_WS_DOCS_PATH);
    assert_eq!(ws.protocol, ACTION_WS_SUBPROTOCOL);
    assert_eq!(ws.subprotocol, "1.0");
    let rest = versions.action().expect("action entry");
    assert_eq!(rest.path, "/api/action/1.0/");
    assert_eq!(rest.protocol, "");
}

// --------------------------------------------------------- action.yaml --

/// `action.yaml` types the capabilities response as `MapOfCapabilities` and the
/// applicabilities response as `MapOfApplicabilities`. Both are maps keyed by
/// capability name and by handler id. The YAML types only the literal keys
/// `capabilityName1` (line 113) and `handlerId1` (line 126), a defect of
/// the capture (HIRO_PUBLIC_API_SPECS.md section 5). The tests use the portal's
/// shape with real names.
#[test]
fn the_yaml_types_only_the_placeholder_keys() {
    let yaml = fixture("action.yaml");
    let lines: Vec<&str> = yaml.lines().collect();
    let def = |name: &str| -> String {
        let start = lines
            .iter()
            .position(|l| *l == format!("  {name}:"))
            .unwrap_or_else(|| panic!("{name} not found"));
        lines[start..]
            .iter()
            .enumerate()
            .take_while(|(i, l)| *i == 0 || l.starts_with("   "))
            .map(|(_, l)| *l)
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert!(def("MapOfCapabilities").contains("      capabilityName1:"));
    assert!(def("MapOfApplicabilities").contains("      handlerId1:"));
    // `Applicability` (line 119) also uses the key `capabilityName1`, for its
    // own entry type; pinned apart so it is not mistaken for the map's key.
    assert!(def("Applicability").contains("      capabilityName1:"));
    // And the Capability definition (lines 95-97) requires these two.
    assert!(yaml.contains("required: [description, mandatoryParameters]"));
}

#[test]
fn capabilities_keyed_by_real_names_are_accepted() {
    let body = json!({
        "ExecuteCommand": {
            "description": "Run a command",
            "mandatoryParameters": {"command": {"description": "the command"}},
            "optionalParameters": {"timeout": {"default": 60000}}
        },
        "RunScript": {"description": "Run a script", "mandatoryParameters": {}}
    })
    .to_string();
    let caps = ogar_action_handler::parse_capabilities(&body).expect("parses");
    let mut names: Vec<&str> = caps.iter().map(|c| c.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["ExecuteCommand", "RunScript"]);
    let exec = caps.iter().find(|c| c.name == "ExecuteCommand").unwrap();
    assert_eq!(exec.description.as_deref(), Some("Run a command"));
    assert!(
        exec.params
            .iter()
            .any(|p| p.name == "command" && p.mandatory)
    );
    // The invoker keeps the same body as its registration.
    let reg = Registration {
        capabilities: serde_json::from_str(&body).unwrap(),
        applicabilities: json!({}),
    };
    assert_eq!(reg.concrete_capabilities().expect("parses").len(), 2);
}

/// More lenient than the schema: `Capability.required` is `description` and
/// `mandatoryParameters` (action.yaml line 97); the crate accepts a capability
/// with neither, as graph_conn's mock serves them.
#[test]
fn leniency_a_capability_without_description_or_mandatory_parameters_is_accepted() {
    let body = r#"{"ExecuteCommand":{"optionalParameters":{"timeout":{"default":60000}}}}"#;
    let caps = ogar_action_handler::parse_capabilities(body).expect("parses");
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].description, None);
}

#[test]
fn applicabilities_keyed_by_real_handler_ids_are_accepted() {
    let body = json!({
        "ah_handler_one": {"ExecuteCommand": {"on ogit/_id": {"LocalNodeID": "${ogit/_id}"}}},
        "ah_handler_two": {}
    })
    .to_string();
    let rows = ogar_action_handler::applicability::parse_applies_on(&body).expect("parses");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].handler, "ah_handler_one");
    let reg = Registration {
        capabilities: json!({}),
        applicabilities: serde_json::from_str(&body).unwrap(),
    };
    let mut ids = reg.handler_ids();
    ids.sort_unstable();
    assert_eq!(ids, ["ah_handler_one", "ah_handler_two"]);
    assert!(reg.typed_applicabilities().is_ok());
}
