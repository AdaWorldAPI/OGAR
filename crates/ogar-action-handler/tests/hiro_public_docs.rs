//! The handler and invoker code against HIRO's PUBLIC documentation (the 7.0
//! developer portal), item by item. Nothing here changes behaviour: each test
//! feeds the docs' own sample to the code and pins what happens.
//!
//! Sources of the literals:
//!
//! * `H` is `/home/user/bardioc/docs/handover/HIRO_PUBLIC_DOCS_HARVEST.md`; its
//!   section letters are used below (B is the action-ws protocol, C the REST
//!   capabilities and applicabilities). Page keys: AA is "Action API in Hiro 7",
//!   CAP the "List of Action Capabilities" page, TUT-AH the action handler
//!   tutorial.
//! * `D` is `.../HIRO_PUBLIC_DOCS_DIFF.md`, for the tutorial sample's extra
//!   fields (section (b), "Other docs-versus-docs contradictions").
//! * `G` is graph_conn, `/home/user/bardioc/extraction/corpus/graph_conn`.
//!
//! The docs' placeholders (`handlerId`, `capabilityName`, `parametername1`, the
//! id sentence) are used as written, because they are what the docs show. Where
//! the docs and graph_conn disagree, graph_conn's behaviour is kept and the test
//! says what that is.

use ogar_action_handler::invoke_session::{Effect as IEffect, InvokeSession};
use ogar_action_handler::invoke_state::{CallRequest, Completion, InvokeError, Log};
use ogar_action_handler::session::{Effect, Session};
use ogar_action_handler::status;
use ogar_action_handler::submit_state::{Config, ExecResult};
use ogar_action_handler::wire::{self, ActionWsFrame, DEFAULT_MAX_FRAME_BYTES, Hello};
use ogar_action_handler::{
    Applicabilities, ApplicabilityError, NativeCommandExecutor, parse_applicabilities_typed,
    parse_applies_on, parse_capabilities,
};
use ogar_from_schema::do_arm::ActionParam;
use ogar_from_schema::registration::{AppliesOn, ConcreteCapability};
use ogar_vocab::ActionDef;
use serde_json::{Value, json};

fn parse(text: &str) -> Value {
    serde_json::from_str(text).expect("JSON")
}

fn session() -> Session {
    Session::new(Config::default()).expect("default config")
}

fn sent(fx: &[Effect]) -> Vec<Value> {
    fx.iter()
        .filter_map(|e| match e {
            Effect::Send(s) => Some(parse(s)),
            Effect::Run { .. } => None,
        })
        .collect()
}

/// The `id` of the docs' submit sample (H section B, `submitAction`, AA). The
/// docs describe it in words; the words are the only literal there is.
const DOCS_ID: &str = "the $requestId in App->Graph, the $appId:$requestId in Graph->AH";

// ───────────────────────────────────────────── item 1: capabilities ──

/// H section C, "GET capabilities" (AA), verbatim.
const CAPABILITIES_AA: &str = r#"
{
  "ExecuteCommand": {
    "description": "this one executes commands",
    "mandatoryParameters": {
      "command": { "description": "command to execute" },
      "host": { "description": "hostname to execute command on" }
    },
    "optionalParameters": {
      "timeout": { "description": "timeout in seconds", "default": "120" }
    }
  }
}
"#;

/// H section C, "The List of Action Capabilities page (CAP)", verbatim. The page
/// shows the capability's JSON object; it is wrapped in a map here, which is the
/// shape of the endpoint's answer.
const CAPABILITIES_CAP: &str = r#"
{
  "ExecuteCommand": {
    "ogit/Automation/mandatoryParameters": { "command": { "description": "command to execute" } },
    "ogit/Automation/optionalParameters": {
      "host": { "description": "hostname to execute command on", "default": "" },
      "user": { "description": "user to run the command as", "default": "" },
      "timeout": { "description": "timeout in seconds", "default": "120" }
    },
    "ogit/Automation/resultParameters": {}
  }
}
"#;

fn param(name: &str, mandatory: bool, default: Option<&str>) -> ActionParam {
    ActionParam {
        name: name.to_owned(),
        mandatory,
        default: default.map(str::to_owned),
    }
}

/// The docs' `GET capabilities` example: `description`, mandatory and optional
/// parameters, and the default as the STRING `"120"`. It parses to the typed
/// capability; the default is kept as text. Unchanged behaviour.
#[test]
fn get_capabilities_example_parses_with_its_string_default() {
    let caps = parse_capabilities(CAPABILITIES_AA).expect("parses");
    assert_eq!(caps.len(), 1);
    let cap = &caps[0];
    assert_eq!(cap.name, "ExecuteCommand");
    assert_eq!(
        cap.description.as_deref(),
        Some("this one executes commands")
    );
    assert_eq!(
        cap.params,
        vec![
            param("command", true, None),
            param("host", true, None),
            param("timeout", false, Some("120")),
        ]
    );
    assert_eq!(
        cap.param_descriptions.get("timeout").map(String::as_str),
        Some("timeout in seconds")
    );
    assert!(cap.result_params.is_empty());
}

/// The capabilities-list page's variant, with the keys
/// `ogit/Automation/mandatoryParameters`, `.../optionalParameters` and
/// `.../resultParameters`. Before this change the prefixed keys were ignored
/// without a word (the parser drops a key it does not know), so every parameter
/// was lost; they are now aliases of the unprefixed keys.
#[test]
fn capabilities_list_page_variant_with_ogit_automation_keys_parses() {
    let caps = parse_capabilities(CAPABILITIES_CAP).expect("parses");
    let cap = &caps[0];
    assert_eq!(cap.name, "ExecuteCommand");
    assert_eq!(cap.description, None, "the CAP JSON has no description key");
    assert_eq!(
        cap.params,
        vec![
            param("command", true, None),
            param("host", false, Some("")),
            param("timeout", false, Some("120")),
            param("user", false, Some("")),
        ]
    );
    assert!(cap.result_params.is_empty());
}

/// Both spellings give the same typed capability: the docs' REST example with
/// its three keys renamed to the prefixed form lifts to a value equal to the
/// unprefixed one, parameters, defaults as text, and result parameters.
#[test]
fn both_key_spellings_lift_to_the_same_typed_capability() {
    let prefixed = CAPABILITIES_AA
        .replace("mandatoryParameters", "ogit/Automation/mandatoryParameters")
        .replace("optionalParameters", "ogit/Automation/optionalParameters");
    assert_ne!(prefixed, CAPABILITIES_AA);
    let a: Vec<ConcreteCapability> = parse_capabilities(CAPABILITIES_AA).expect("plain");
    let b: Vec<ConcreteCapability> = parse_capabilities(&prefixed).expect("prefixed");
    assert_eq!(a, b);
    assert_eq!(b[0].params.len(), 3);
}

/// `resultParameters` through its prefixed key. The names are those the CAP page
/// lists for `HTTPRequest` (H section C, "resultParameters": `body`, `code`,
/// `headers`); the page's body shape for them is not in the harvest, so each is
/// given the empty object the page shows for `ExecuteCommand`'s own (an empty
/// map) in the parameter position. Shape UNVERIFIED.
#[test]
fn prefixed_result_parameters_are_kept() {
    let body = r#"{"HTTPRequest":{"ogit/Automation/resultParameters":{"body":{},"code":{},"headers":{}}}}"#;
    let caps = parse_capabilities(body).expect("parses");
    assert_eq!(caps[0].result_params, vec!["body", "code", "headers"]);
}

/// What an unknown key in a capability does, pinned: it is IGNORED. It is neither
/// kept nor reported, and it is not an error. (The prefixed keys used to be in
/// this class; they are now read.) The key below is deliberately not a HIRO string.
#[test]
fn an_unknown_key_in_a_capability_is_ignored_not_reported() {
    let body =
        r#"{"ExecuteCommand":{"x-not-a-hiro-key":{"p":{}},"mandatoryParameters":{"command":{}}}}"#;
    let caps = parse_capabilities(body).expect("an unknown key is not an error");
    assert_eq!(caps[0].params, vec![param("command", true, None)]);
}

// ─────────────────────────────────────────── item 2: applicabilities ──

/// H section C, "GET applicabilities" (AA), the first capability only,
/// verbatim: the `on` keys with trailing words after the comparison.
const APPLICABILITIES_MAP_FORM: &str = r#"
{
  "HandlerID1": {
    "ExecuteCommand": {
      "on ogit/machineClass == \"linux\" ssh_keyfile ssh_options": {
        "ssh_keyfile": "${ssh_keyfile}",
        "ssh_options": "${ssh_options}"
        },
      "on ogit/machineClass == \"linux\" ssh_keyfile not ssh_options": {
        "ssh_keyfile": "${ssh_keyfile}",
        "ssh_options": "some default options here"
        }
      }
  }
}
"#;

/// The whole body of the same example, with the list form for the parameterless
/// capability (H section C, `"ParameterlessExecute": ["on ogit/machineClass ==
/// \"windows\""]`), verbatim.
const APPLICABILITIES_DOCS_FULL: &str = r#"
{
  "HandlerID1": {
    "ExecuteCommand": {
      "on ogit/machineClass == \"linux\" ssh_keyfile ssh_options": {
        "ssh_keyfile": "${ssh_keyfile}",
        "ssh_options": "${ssh_options}"
        },
      "on ogit/machineClass == \"linux\" ssh_keyfile not ssh_options": {
        "ssh_keyfile": "${ssh_keyfile}",
        "ssh_options": "some default options here"
        }
      },
    "ParameterlessExecute": [
      "on ogit/machineClass == \"windows\""
      ]
  }
}
"#;

fn on_row(on: &str, parameters: &[(&str, &str)]) -> AppliesOn {
    AppliesOn {
        handler: "HandlerID1".to_owned(),
        capability: "ExecuteCommand".to_owned(),
        on: on.to_owned(),
        parameters: parameters
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
    }
}

/// The docs' map form (handler id, capability, `on <expr> <words>`, templates)
/// parses. The `on` expression keeps the words after the comparison
/// (`ssh_keyfile ssh_options`, `ssh_keyfile not ssh_options`) VERBATIM: the docs
/// give them no meaning, so none is read (no `not` is interpreted, no parameter
/// name is split off), and the `${...}` templates are not expanded. Unchanged
/// behaviour.
#[test]
fn applicabilities_example_keeps_the_on_expression_and_templates_verbatim() {
    let mut rows = parse_applies_on(APPLICABILITIES_MAP_FORM).expect("parses");
    // The row order follows the JSON map's key order, which depends on a feature
    // of serde_json that other crates may switch on; compare in a fixed order.
    rows.sort_by(|a, b| a.on.cmp(&b.on));
    assert_eq!(
        rows,
        vec![
            on_row(
                r#"ogit/machineClass == "linux" ssh_keyfile not ssh_options"#,
                &[
                    ("ssh_keyfile", "${ssh_keyfile}"),
                    ("ssh_options", "some default options here")
                ]
            ),
            on_row(
                r#"ogit/machineClass == "linux" ssh_keyfile ssh_options"#,
                &[
                    ("ssh_keyfile", "${ssh_keyfile}"),
                    ("ssh_options", "${ssh_options}")
                ]
            ),
        ]
    );
    assert_eq!(
        rows[1].wire_key(),
        r#"on ogit/machineClass == "linux" ssh_keyfile ssh_options"#
    );
}

/// The typed entry point reads the same body as ON entries.
#[test]
fn the_typed_entry_point_reads_the_docs_map_form() {
    let direct = parse_applies_on(APPLICABILITIES_MAP_FORM).expect("parses");
    assert_eq!(
        parse_applicabilities_typed(APPLICABILITIES_MAP_FORM),
        Ok(Applicabilities::AppliesOn(direct))
    );
}

/// The docs' full example, with the list form for a parameterless capability
/// (`"ParameterlessExecute": ["on ogit/machineClass == \"windows\""]`), parses:
/// each `on <expr>` string of the list is one entry with no parameters. A list
/// element that is not a string, or does not begin with `on `, is refused by name.
#[test]
fn the_docs_list_form_inside_a_response_body_is_one_entry_per_on_string() {
    let rows = parse_applies_on(APPLICABILITIES_DOCS_FULL).expect("the docs' example parses");
    let list: Vec<_> = rows
        .iter()
        .filter(|r| r.capability == "ParameterlessExecute")
        .collect();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].handler, "HandlerID1");
    assert_eq!(list[0].on, r#"ogit/machineClass == "windows""#);
    assert!(list[0].parameters.is_empty());
    // The map-form entries of the same body are still read.
    assert_eq!(
        rows.iter()
            .filter(|r| r.capability == "ExecuteCommand")
            .count(),
        2
    );
    // The typed entry point agrees.
    assert_eq!(
        parse_applicabilities_typed(APPLICABILITIES_DOCS_FULL),
        Ok(Applicabilities::AppliesOn(rows))
    );

    assert_eq!(
        parse_applies_on(r#"{"h":{"C":["at x"]}}"#),
        Err(ApplicabilityError::NotAnOnKey {
            handler: "h".to_owned(),
            capability: "C".to_owned(),
            key: "at x".to_owned(),
        })
    );
    assert_eq!(
        parse_applies_on(r#"{"h":{"C":[{"on x":{}}]}}"#),
        Err(ApplicabilityError::CapabilityNotAnObject {
            handler: "h".to_owned(),
            capability: "C".to_owned(),
        })
    );
}

// ───────────────────────────────────────────────────── item 3: hello ──

/// H section B, `hello` (AA), the field descriptions as the docs write them. No
/// `max_frame_bytes`.
const DOCS_HELLO: &str = r#"{"type": "hello", "host": "action api node that accepted connection", "server_version": "application version of the action api node", "client_id": "id of the client in the graph"}"#;

/// The docs' hello decodes; it advertises no frame limit.
#[test]
fn hello_without_max_frame_bytes_decodes() {
    assert_eq!(
        wire::decode(DOCS_HELLO),
        Ok(ActionWsFrame::Hello(Hello {
            max_frame_bytes: None
        }))
    );
}

/// After the docs' hello the session keeps its default frame limit, 1,000,000
/// bytes (graph_conn `connection_manager.ex:61`, `:201-221`; H section B says the
/// docs give no frame limit, so the default is graph_conn's). The limit is pinned
/// both as a number and by what it does: a result frame over it is replaced by
/// the status 59 error, one under it is sent as it is.
#[test]
fn the_docs_hello_leaves_the_default_frame_limit_of_1000000_bytes() {
    assert_eq!(DEFAULT_MAX_FRAME_BYTES, 1_000_000);
    let mut s = session();
    assert!(s.on_text(0, DOCS_HELLO).expect("decodes").is_empty());
    assert_eq!(s.state().max_frame_bytes(), 1_000_000);

    let submit = json!({
        "type": "submitAction", "id": "id-big", "handler": "handlerId",
        "capability": "capabilityName", "timeout": 300_000, "parameters": {},
    })
    .to_string();
    s.on_text(1, &submit).expect("submit");
    let fx = s.on_finished(2, "id-big", ExecResult::Success("x".repeat(1_000_000)));
    let frames = sent(&fx);
    assert_eq!(frames.len(), 1);
    let result: Value = parse(frames[0]["result"].as_str().expect("a string result"));
    assert_eq!(result["error"]["action_status"], json!(59));

    let fx = {
        let submit = submit.replace("id-big", "id-ok");
        s.on_text(3, &submit).expect("submit");
        s.on_finished(4, "id-ok", ExecResult::Success("x".repeat(100)))
    };
    let frames = sent(&fx);
    assert_eq!(frames[0]["result"].as_str(), Some("x".repeat(100).as_str()));
}

// ───────────────────────────────────────── item 4: submitAction variants ──

/// H section B, `submitAction` (AA): `handler`, `capability`, top-level `timeout`
/// 300000 (milliseconds), `parameters` as an object.
fn docs_submit() -> String {
    json!({
        "type": "submitAction",
        "id": DOCS_ID,
        "handler": "handlerId",
        "capability": "capabilityName",
        "timeout": 300_000,
        "parameters": {"parametername1": "parametervalue1"},
    })
    .to_string()
}

/// The tutorial's sample (H section B, TUT-AH; D section (b)): the same plus a
/// `scope`, timeout 60000000, and `timeout` again inside `parameters`.
fn tutorial_submit() -> String {
    json!({
        "type": "submitAction",
        "id": DOCS_ID,
        "handler": "$id of the handler",
        "scope": "$scope of your instance from which the KI was triggered",
        "capability": "capabilityName",
        "timeout": 60_000_000,
        "parameters": {"parametername1": "parametervalue1", "timeout": 60_000_000},
    })
    .to_string()
}

fn ack_frame(id: &str) -> Value {
    json!({"type": "acknowledged", "id": id, "code": 200, "message": ""})
}

/// The docs' sample decodes, is acked, runs once with its parameters, and its
/// deadline is the top-level `timeout` in milliseconds; the end of the run sends
/// the result.
#[test]
fn the_docs_submit_runs_through_the_handler_session() {
    let mut s = session();
    let fx = s.on_text(0, &docs_submit()).expect("decodes");
    assert_eq!(sent(&fx), vec![ack_frame(DOCS_ID)]);
    assert_eq!(
        fx[1],
        Effect::Run {
            id: DOCS_ID.to_owned(),
            capability: "capabilityName".to_owned(),
            parameters: vec![("parametername1".to_owned(), "parametervalue1".to_owned())],
        }
    );
    assert_eq!(s.next_wakeup(), Some(300_000));
    let fx = s.on_finished(10, DOCS_ID, ExecResult::Success("{}".to_owned()));
    let frames = sent(&fx);
    assert_eq!(frames[0]["type"], "sendActionResult");
    assert_eq!(frames[0]["id"], DOCS_ID);
}

/// The tutorial's sample decodes and runs. `scope` is NOT an error and is NOT
/// kept: the decoder reads the keys it needs and drops the rest, so
/// `SubmitAction::scope` is `None` (and `wire::encode` never writes one).
/// `handler` is kept. The deadline is the top-level 60000000 ms. A `timeout`
/// inside `parameters` is an ordinary parameter: it is passed on as text
/// (`"60000000"`) and plays no part in the deadline.
#[test]
fn the_tutorial_submit_with_scope_runs_and_scope_is_ignored() {
    let Ok(ActionWsFrame::SubmitAction(sub)) = wire::decode(&tutorial_submit()) else {
        panic!("the tutorial submit must decode")
    };
    assert_eq!(sub.handler, "$id of the handler");
    assert_eq!(sub.scope, None, "scope is dropped at decode, not an error");
    assert_eq!(sub.timeout_millis, Some(60_000_000));

    let mut s = session();
    let fx = s.on_text(0, &tutorial_submit()).expect("decodes");
    assert_eq!(sent(&fx), vec![ack_frame(DOCS_ID)]);
    assert_eq!(
        fx[1],
        Effect::Run {
            id: DOCS_ID.to_owned(),
            capability: "capabilityName".to_owned(),
            parameters: vec![
                ("parametername1".to_owned(), "parametervalue1".to_owned()),
                ("timeout".to_owned(), "60000000".to_owned()),
            ],
        }
    );
    assert_eq!(s.next_wakeup(), Some(60_000_000));
}

/// "handler field is not sent to the action handler" (AA, in the sentence under
/// the sample whose own text contains `"handler": "handlerId"`): a submit
/// WITHOUT `handler` also runs. The handler decodes to the empty string, as
/// graph_conn's handler never reads it (`handler.ex:121-130`).
#[test]
fn handler_field_is_not_sent_to_the_action_handler() {
    let mut v = parse(&docs_submit());
    v.as_object_mut().expect("object").remove("handler");
    let Ok(ActionWsFrame::SubmitAction(sub)) = wire::decode(&v.to_string()) else {
        panic!("a submit without `handler` must decode")
    };
    assert_eq!(sub.handler, "");
    let mut s = session();
    let fx = s.on_text(0, &v.to_string()).expect("decodes");
    assert_eq!(sent(&fx), vec![ack_frame(DOCS_ID)]);
    assert!(matches!(fx[1], Effect::Run { .. }));
}

// ───────────────────────────────────────────────── item 5: acknowledged ──

/// A session with the docs' submit finished: the result is sent at 10 and due
/// again at 3010.
fn session_with_pending_result() -> Session {
    let mut s = session();
    s.on_text(0, &docs_submit()).expect("decodes");
    s.on_finished(10, DOCS_ID, ExecResult::Success("{}".to_owned()));
    assert_eq!(s.next_wakeup(), Some(3_010));
    s
}

/// The docs' ack, `type` and `id` only (H section B, `acknowledged`, AA), stops the
/// result resend.
#[test]
fn the_docs_acknowledged_with_type_and_id_only_stops_the_resend() {
    let mut s = session_with_pending_result();
    let frame = json!({"type": "acknowledged", "id": DOCS_ID}).to_string();
    assert!(s.on_text(100, &frame).expect("decodes").is_empty());
    assert!(s.on_tick(3_010).is_empty());
}

/// The tutorial's quoted ack with `code` 200 and `message` "" (H section B,
/// TUT-AH) stops it too.
#[test]
fn the_tutorials_acknowledged_with_code_and_message_stops_the_resend() {
    let mut s = session_with_pending_result();
    let frame =
        json!({"type": "acknowledged", "id": DOCS_ID, "code": 200, "message": ""}).to_string();
    assert!(s.on_text(100, &frame).expect("decodes").is_empty());
    assert!(s.on_tick(3_010).is_empty());
}

// ──────────────────────────── item 6: result as an object, the invoker ──

/// Start a call, ack it, deliver `frame` (built with the call's id) and return
/// what the call completed with.
fn completes_with(frame: impl FnOnce(&str) -> String) -> Completion {
    let mut s = InvokeSession::new();
    let fx = s.call(
        0,
        CallRequest::new(1, "ticket", "handler", "ExecuteCommand"),
    );
    let IEffect::Send { text, .. } = &fx[0] else {
        panic!("{fx:?}")
    };
    let id = parse(text)["id"].as_str().expect("id").to_owned();
    s.on_text(1, &json!({"type": "acknowledged", "id": id}).to_string())
        .expect("ack");
    let fx = s.on_text(2, &frame(&id)).expect("the frame decodes");
    assert!(matches!(fx[0], IEffect::Send { .. }), "acked first");
    let Some(IEffect::Complete(c)) = fx.into_iter().find(|e| matches!(e, IEffect::Complete(_)))
    else {
        panic!("no completion")
    };
    c
}

/// graph_conn's invoker (`invoker.ex:742-754`): a `result` that is a binary is
/// JSON-decoded once, and anything else (`_decode_result(result), do: result`,
/// `:753-754`) is handed over as it is, so a JSON OBJECT result is accepted and
/// IS the value. Then (`:723-735`) only an `error` key makes it an error.
/// `action_status` and `action_error` appear nowhere in graph_conn's invoker
/// (`rg` over `lib/` finds them only in `result_frame.ex` and `handler.ex`, the
/// handler's own error results). OGAR does the same; this is the docs'
/// `sendActionResult.result` object (H section B, `resultfield1`).
#[test]
fn the_docs_result_object_completes_the_call_with_the_object() {
    let c = completes_with(|id| {
        json!({
            "type": "sendActionResult", "id": id,
            "result": {"resultfield1": "resultvalue1", "resultfield2": "resultvalue2"},
        })
        .to_string()
    });
    assert_eq!(
        c.result,
        Ok(json!({"resultfield1": "resultvalue1", "resultfield2": "resultvalue2"}))
    );
}

/// An object carrying `action_status` other than 0 and an `action_error` (AA:
/// "action_status which is non-negative number and action_error as nil or string",
/// 0 when the action was executed; 54's meaning is the docs' own) completes as a
/// VALUE, as graph_conn's invoker does with any object without an `error` key; it
/// is not an `InvokeError`. The caller tells the two apart by reading
/// `action_status` from the value, which arrives intact.
#[test]
fn an_object_with_a_nonzero_action_status_is_distinguishable_to_the_caller() {
    let failed = completes_with(|id| {
        json!({
            "type": "sendActionResult", "id": id,
            "result": {
                "action_status": 54,
                "action_error": "Execution failed/crashed or there was an error with processing response",
            },
        })
        .to_string()
    });
    let done = completes_with(|id| {
        json!({
            "type": "sendActionResult", "id": id,
            "result": {"action_status": 0, "action_error": null},
        })
        .to_string()
    });
    let failed = failed.result.expect("a value, not an InvokeError");
    let done = done.result.expect("a value, not an InvokeError");
    assert_eq!(failed["action_status"], json!(54));
    assert_eq!(
        failed["action_error"],
        json!("Execution failed/crashed or there was an error with processing response")
    );
    assert_eq!(done["action_status"], json!(0));
    assert_eq!(done["action_error"], Value::Null);
    assert_ne!(failed, done);
}

/// The object form and graph_conn's JSON-string form of the same result give
/// the same value (oracle 35).
#[test]
fn the_object_form_and_the_json_string_form_give_the_same_value() {
    let object = json!({"resultfield1": "resultvalue1", "resultfield2": "resultvalue2"});
    let as_object = completes_with(|id| {
        json!({"type": "sendActionResult", "id": id, "result": object}).to_string()
    });
    let as_string = completes_with(|id| {
        json!({"type": "sendActionResult", "id": id, "result": object.to_string()}).to_string()
    });
    assert_eq!(as_object.result, as_string.result);
    assert_eq!(as_object.result, Ok(object));
}

/// An object with an `error` key stays an error in the object form, as in the
/// string form (`invoker.ex:730-731`): the shape graph_conn's own handler sends
/// for a crash (`handler.ex:335-339`, `{"error": {req_id, action_status,
/// action_error}}`), here as an object.
#[test]
fn an_object_with_an_error_key_is_still_an_error() {
    let error = json!({"req_id": "r", "action_status": 54, "action_error": "boom"});
    let c = completes_with(|id| {
        json!({"type": "sendActionResult", "id": id, "result": {"error": error}}).to_string()
    });
    assert_eq!(c.result, Err(InvokeError::Handler(error)));
}

// ─────────────────────────── item 7: negativeAcknowledged 404, both ways ──

/// The invoker side. "code 404 should be returned from the action handler if it
/// doesn't support requested capability" (AA, nack; H section B): a
/// `negativeAcknowledged` 404 reaching the invoker is the terminal NACK it
/// already implements (`invoker.ex:659-664`): the call ends with the code and the
/// message, nothing is sent again, and a 404 is not logged (`:662-664`). The
/// message is the docs' placeholder for the field.
#[test]
fn code_404_should_be_returned_from_the_action_handler_if_it_doesnt_support_requested_capability() {
    let mut s = InvokeSession::new();
    let fx = s.call(
        0,
        CallRequest::new(1, "ticket", "handlerId", "capabilityName"),
    );
    let IEffect::Send { text, .. } = &fx[0] else {
        panic!("{fx:?}")
    };
    let id = parse(text)["id"].as_str().expect("id").to_owned();
    let nack = json!({
        "type": "negativeAcknowledged", "id": id, "code": 404,
        "message": "error description why request was nack'ed",
    })
    .to_string();
    let fx = s.on_text(5, &nack).expect("decodes");
    assert_eq!(fx.len(), 1, "no log for a 404, one completion: {fx:?}");
    let IEffect::Complete(c) = &fx[0] else {
        panic!("{fx:?}")
    };
    assert_eq!(
        c.result,
        Err(InvokeError::Nack {
            code: Some(404),
            message: Some("error description why request was nack'ed".to_owned()),
        })
    );
    assert!(s.on_tick(1_000_000).is_empty(), "no resend");
    assert_eq!(s.state().tracked(), 0);
}

/// A difference the docs' flow raises and graph_conn's source answers: in the
/// docs' chain the handler's NACK would follow the gateway's ack of the request.
/// graph_conn's invoker takes a NACK only while it waits for the ack
/// (`invoker.ex:659-664`); one that arrives later is not matched. OGAR drops it
/// too: the call is not ended by it (it ends by the response wait). Pinned, and an
/// open question (see the parity document).
#[test]
fn a_nack_after_the_ack_does_not_end_the_call_as_in_graph_conn() {
    let mut s = InvokeSession::new();
    let fx = s.call(
        0,
        CallRequest::new(1, "ticket", "handlerId", "capabilityName"),
    );
    let IEffect::Send { text, .. } = &fx[0] else {
        panic!("{fx:?}")
    };
    let id = parse(text)["id"].as_str().expect("id").to_owned();
    s.on_text(1, &json!({"type": "acknowledged", "id": id}).to_string())
        .expect("ack");
    let nack = json!({
        "type": "negativeAcknowledged", "id": id, "code": 404,
        "message": "error description why request was nack'ed",
    })
    .to_string();
    let fx = s.on_text(2, &nack).expect("decodes");
    assert!(
        !fx.iter().any(|e| matches!(e, IEffect::Complete(_))),
        "{fx:?}"
    );
    assert!(
        !fx.iter()
            .any(|e| matches!(e, IEffect::Log(Log::MessageNacked { .. }))),
        "{fx:?}"
    );
    assert_eq!(s.state().tracked(), 1, "the call is still waiting");
}

/// The handler side. OGAR keeps graph_conn's behaviour: a capability it does not
/// serve is acked and then answered with a RESULT carrying the 404 error
/// (`handler.ex:369-371`), not with a `negativeAcknowledged`, which is what the
/// docs ask for. An off-by-default option for the docs' reading is not built (see
/// the parity document for what it takes).
#[test]
fn an_unsupported_capability_on_the_handler_is_a_404_result_not_a_nack() {
    let def = ActionDef::new(
        "ogit-automation/.../ExecuteCommand",
        "ExecuteCommand",
        "ogit-automation/mars_machine",
    );
    let sig = [param("command", true, None)];
    let mut s = session();
    let frames = s
        .on_text_sync(0, &docs_submit(), &def, &sig, &NativeCommandExecutor)
        .expect("decodes");
    let frames: Vec<Value> = frames.iter().map(|f| parse(f)).collect();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0], ack_frame(DOCS_ID));
    assert_eq!(frames[1]["type"], "sendActionResult");
    let result = parse(frames[1]["result"].as_str().expect("a string result"));
    assert_eq!(result["error"]["code"], json!(404));
    assert!(
        !frames.iter().any(|f| f["type"] == "negativeAcknowledged"),
        "no NACK is sent"
    );
}

// ─────────────────────────────────────────────── item 8: status codes ──

/// The table lists exactly the docs' codes, in the docs' order, and 59 is not
/// one of them (H section B, "action_status codes in use": "Codes 59 ... NOT FOUND
/// in the docs").
#[test]
fn the_documented_status_table_is_the_docs_list_and_59_is_not_in_it() {
    let codes: Vec<u32> = status::DOCUMENTED.iter().map(|(c, _)| *c).collect();
    assert_eq!(codes, vec![0, 11, 12, 13, 14, 51, 52, 53, 54, 666]);
    assert_eq!(status::documented_meaning(0), Some("action executed"));
    assert_eq!(
        status::documented_meaning(14),
        Some("ActionHandler responded with execution timeout")
    );
    assert_eq!(status::documented_meaning(59), None);
    assert_eq!(
        status::GRAPH_CONN_ONLY
            .iter()
            .map(|(c, _)| *c)
            .collect::<Vec<_>>(),
        vec![59]
    );
}

/// The codes the session sends are unchanged, and are the table's: 13 for its own
/// execution deadline (the docs read 13 as "ActionHandler didn't respond" and give
/// the handler's execution timeout as 14), 54 for a crash, 59 for a result over
/// the frame limit. The session never sends 14.
#[test]
fn the_codes_the_session_sends_are_the_same_as_before() {
    use ogar_action_handler::{crash, submit_state, wire};
    assert_eq!(submit_state::EXEC_TIMEOUT_STATUS, 13);
    assert_eq!(
        submit_state::EXEC_TIMEOUT_STATUS,
        status::HANDLER_DID_NOT_RESPOND
    );
    assert_eq!(crash::CRASH_STATUS, 54);
    assert_eq!(crash::CRASH_STATUS, status::EXECUTION_FAILED);
    assert_eq!(wire::TOO_BIG_STATUS, 59);
    assert_eq!(
        wire::TOO_BIG_STATUS,
        status::GRAPH_CONN_ONLY_RESPONSE_TOO_BIG
    );

    // An execution past its deadline is reported as 13, not 14.
    let mut s = session();
    let submit = json!({
        "type": "submitAction", "id": "id-late", "handler": "handlerId",
        "capability": "capabilityName", "timeout": 1000, "parameters": {},
    })
    .to_string();
    s.on_text(0, &submit).expect("decodes");
    let fx = s.on_tick(1000);
    let frames = sent(&fx);
    let result = parse(frames[0]["result"].as_str().expect("a string result"));
    assert_eq!(result["error"]["action_status"], json!(13));
    assert_ne!(result["error"]["action_status"], json!(14));
}
