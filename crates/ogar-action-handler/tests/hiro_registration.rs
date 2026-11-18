//! Capabilities and applicabilities against graph_conn's registration shapes.
//!
//! `G` is the graph_conn Elixir client
//! (`/home/user/bardioc/extraction/corpus/graph_conn`). Registration has no numbered
//! assertion in the parity oracle (`hiro_oracle.md` section 7 covers the action-ws
//! lifecycle); these tests pin oracle section 2 (REGISTRATION). Fixtures are JSON
//! text copied from `G`; where `G` states the shape as an Elixir map, the fixture is
//! the JSON of that map and says so.

use ogar_action_handler::{
    Applicabilities, ApplicabilityError, parse_applicabilities, parse_applicabilities_full,
    parse_applicabilities_typed, parse_applies_on, parse_applies_on_definitions,
    parse_capabilities,
};
use ogar_from_schema::registration::AppliesOn;

fn on(handler: &str, capability: &str, on: &str, parameters: &[(&str, &str)]) -> AppliesOn {
    AppliesOn {
        handler: handler.to_owned(),
        capability: capability.to_owned(),
        on: on.to_owned(),
        parameters: parameters
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
    }
}

// ───────────────────────────────────────────────── capabilities ──

/// Oracle section 2, "Capabilities and applicabilities fetch": the converted shape
/// the mock serves for `{"ExecuteCommand": {"timeout": 60000, "command": null}}`.
/// Source: G `test/graph_conn/mock_test.exs:73-93`, the expected Elixir map as JSON.
/// The default is the JSON number `60000`, which the parser keeps as `"60000"`.
#[test]
fn capabilities_mock_shape_with_a_numeric_default() {
    let body = r#"{"ExecuteCommand":{"mandatoryParameters":{"command":{}},"optionalParameters":{"timeout":{"default":60000}}}}"#;
    let caps = parse_capabilities(body).expect("parses");
    assert_eq!(caps.len(), 1);
    let cap = &caps[0];
    assert_eq!(cap.name, "ExecuteCommand");
    assert_eq!(cap.params.len(), 2);
    assert_eq!(cap.params[0].name, "command");
    assert!(cap.params[0].mandatory);
    assert_eq!(cap.params[0].default, None);
    assert_eq!(cap.params[1].name, "timeout");
    assert!(!cap.params[1].mandatory);
    assert_eq!(cap.params[1].default.as_deref(), Some("60000"));
    assert!(cap.result_params.is_empty(), "no resultParameters sent");
}

/// `resultParameters` is kept, default empty. NOT a HIRO fixture: no HIRO client
/// repository sends `resultParameters` (graph_conn's `Capability` struct,
/// `lib/action_api/capability.ex:11-16`, has no result field). The body is
/// hand-written from the OGIT attribute `ogit.Automation:resultParameters`
/// ("optional result parameters as key/value mapping") read as a `MapOfParameters`
/// like the two input maps. The shape is UNVERIFIED.
#[test]
fn capabilities_keep_result_parameters() {
    let body = r#"{"ExecuteCommand":{
        "mandatoryParameters":{"command":{}},
        "resultParameters":{"output":{"description":"stdout"},"exitcode":{}}
    }}"#;
    let caps = parse_capabilities(body).expect("parses");
    let cap = &caps[0];
    assert_eq!(cap.result_params, vec!["exitcode", "output"]);
    assert_eq!(
        cap.result_param_descriptions
            .get("output")
            .map(String::as_str),
        Some("stdout")
    );
    // The result parameters are not part of the input signature.
    assert_eq!(cap.params.len(), 1);
    assert_eq!(cap.params[0].name, "command");
}

/// A capability body that is not a map, and one with an object default, are
/// refused with the parser's message, not accepted as empty.
#[test]
fn capabilities_malformed_bodies_are_refused() {
    assert!(parse_capabilities("[]").is_err());
    assert!(parse_capabilities("not json").is_err());
    let err = parse_capabilities(r#"{"C":{"optionalParameters":{"p":{"default":{"a":1}}}}}"#)
        .expect_err("an object default is refused");
    assert!(!err.is_empty());
    let err = parse_capabilities(r#"{"C":{"resultParameters":["output"]}}"#)
        .expect_err("a list of result parameters is refused");
    assert!(!err.is_empty());
}

// ──────────────────────────────────────── applicabilities: on shape ──

/// The converted response shape, stored per handler id. Oracle section 2 (the per
/// handler form `%{"<handler_id>" => %{"<capability>" => %{"on <attr>" => %{<param> =>
/// <template>}}}}`). Source: G `test/graph_conn/mock_test.exs:123-135`, the expected
/// Elixir map as JSON, under the handler id `action_handler` (`lib/graph_conn/mock.ex:122-141`).
/// The `${...}` templates stay verbatim.
const CONVERTED: &str = r#"{
  "action_handler": {
    "ExecuteLocalCommand": {
      "on ogit/_id": {"LocalNodeID": "${ogit/_id}"},
      "on something_else": {"LocalNodeID": "something_else"}
    },
    "RunLocalScript": {"on ogit/_id": {}}
  }
}"#;

#[test]
fn applies_on_reads_the_converted_response_shape() {
    let rows = parse_applies_on(CONVERTED).expect("parses");
    assert_eq!(
        rows,
        vec![
            on(
                "action_handler",
                "ExecuteLocalCommand",
                "ogit/_id",
                &[("LocalNodeID", "${ogit/_id}")]
            ),
            on(
                "action_handler",
                "ExecuteLocalCommand",
                "something_else",
                &[("LocalNodeID", "something_else")]
            ),
            on("action_handler", "RunLocalScript", "ogit/_id", &[]),
        ]
    );
    assert_eq!(rows[0].wire_key(), "on ogit/_id");
}

/// Templates are not expanded: oracle section 2 ("`${...}` is NOT expanded anywhere
/// in `G/lib`"). The parameter value keeps its `${` and `}`.
#[test]
fn templates_are_stored_verbatim() {
    let rows = parse_applies_on(CONVERTED).expect("parses");
    assert_eq!(rows[0].parameters[0].1, "${ogit/_id}");
}

/// The default applicabilities body of graph_conn's mock has a handler and nothing
/// under it. Source: G `test/graph_conn/mock_test.exs:49-50` (`%{"action_handler" => %{}}`)
/// as JSON.
#[test]
fn an_empty_handler_has_no_entries() {
    assert_eq!(
        parse_applies_on(r#"{"action_handler":{}}"#).expect("parses"),
        vec![]
    );
    assert_eq!(
        parse_applicabilities_typed(r#"{"action_handler":{}}"#).expect("parses"),
        Applicabilities::AppliesOn(vec![])
    );
}

// ──────────────────────────── applicabilities: the definition list ──

/// The definition list graph_conn's mock converts, with both applicability forms:
/// the object `{"on <expr>": {...}}` and the list `["on <expr>"]`. Source: G
/// `test/graph_conn/mock_test.exs:96-121`, verbatim JSON.
const DEFINITIONS: &str = r#"
    [
      {
        "name": "LocalHandler",
        "capability": "ExecuteLocalCommand",
        "implementation": "local",
        "applicability": {"on ogit/_id": {"LocalNodeID": "${ogit/_id}"}},
        "exec": "${command}"
      },
      {
        "name": "LocalHandler",
        "capability": "ExecuteLocalCommand",
        "implementation": "local",
        "applicability": {"on something_else": {"LocalNodeID": "something_else"}},
        "exec": "${command}"
      },
      {
        "name": "LocalHandler",
        "capability": "RunLocalScript",
        "implementation": "local",
        "applicability": ["on ogit/_id"],
        "tempfiles": {"tempfile": "${command}"},
        "exec": "sh -- ${tempfile}"
      }
    ]
    "#;

/// Oracle section 2: the definitions convert to the same entries the converted
/// response holds (`mock_test.exs:123-135`), the list form giving no parameters.
#[test]
fn definitions_convert_to_the_converted_shape() {
    let rows = parse_applies_on_definitions("action_handler", DEFINITIONS).expect("parses");
    assert_eq!(
        rows,
        vec![
            on(
                "action_handler",
                "ExecuteLocalCommand",
                "ogit/_id",
                &[("LocalNodeID", "${ogit/_id}")]
            ),
            on(
                "action_handler",
                "ExecuteLocalCommand",
                "something_else",
                &[("LocalNodeID", "something_else")]
            ),
            on("action_handler", "RunLocalScript", "ogit/_id", &[]),
        ]
    );
}

/// The list form alone, as in the first mock test. Source: G
/// `test/graph_conn/mock_test.exs:56-66`, verbatim JSON.
#[test]
fn the_list_form_has_no_parameters() {
    let body = r#"
      [
        {
          "name": "LocalHandler",
          "capability": "ExecuteLocalCommand",
          "implementation": "local",
          "applicability": ["on ogit/_id"],
          "exec": "${command}"
        }
      ]
      "#;
    let rows = parse_applies_on_definitions("action_handler", body).expect("parses");
    assert_eq!(
        rows,
        vec![on("action_handler", "ExecuteLocalCommand", "ogit/_id", &[])]
    );
}

/// A later definition of the same capability and `on` replaces the earlier one, as
/// the mock's `Map.put` does (`lib/graph_conn/mock.ex:630-635`).
#[test]
fn a_later_definition_replaces_an_earlier_one() {
    let body = r#"[
      {"capability":"C","applicability":{"on x":{"p":"1"}}},
      {"capability":"C","applicability":{"on x":{"p":"2"}}}
    ]"#;
    let rows = parse_applies_on_definitions("h", body).expect("parses");
    assert_eq!(rows, vec![on("h", "C", "x", &[("p", "2")])]);
}

// ────────────────────────────── applicabilities: either shape ──

/// The legacy `ModelFilter` shape stays readable and is reported as such. UNCONFIRMED
/// shape: not in any HIRO client repository. The body is the one the existing
/// `ogar-action-handler` unit test uses (`src/lib.rs`, `rest_applicabilities_full_lift_...`),
/// not a HIRO fixture.
#[test]
fn the_legacy_model_filter_shape_is_still_accepted_and_labelled() {
    let body = r#"{
        "ssh-handler": {
            "priority": 50,
            "modelFilters": [ { "var": "NodeType", "mode": "string", "value": "Machine" } ]
        }
    }"#;
    match parse_applicabilities_typed(body).expect("parses") {
        Applicabilities::ModelFilter(apps) => {
            assert_eq!(apps["ssh-handler"].priority, Some(50));
            assert_eq!(apps["ssh-handler"].model_filters[0].var, "NodeType");
        }
        other => panic!("expected the legacy shape, got {other:?}"),
    }
    // The legacy readers still work on it.
    assert_eq!(
        parse_applicabilities(body).expect("parses")["ssh-handler"].len(),
        1
    );
    assert_eq!(
        parse_applicabilities_full(body).expect("parses")["ssh-handler"].priority,
        Some(50)
    );
}

/// The `on` shape through the typed entry point is `AppliesOn`.
#[test]
fn the_typed_entry_point_reads_the_on_shape() {
    match parse_applicabilities_typed(CONVERTED).expect("parses") {
        Applicabilities::AppliesOn(rows) => assert_eq!(rows.len(), 3),
        other => panic!("expected the on shape, got {other:?}"),
    }
}

/// The legacy readers must not read an `on`-shaped body as "no guards": they refuse
/// it and name the right reader.
#[test]
fn the_legacy_readers_refuse_the_on_shape_by_name() {
    for result in [
        parse_applicabilities(CONVERTED).map(|_| ()),
        parse_applicabilities_full(CONVERTED).map(|_| ()),
    ] {
        let err = result.expect_err("refused");
        assert!(err.contains("parse_applies_on"), "{err}");
    }
}

/// A capability may carry a name the legacy shape uses as a key. The shape is read
/// from the structure, so such a body is still the `on` shape and both entry points
/// agree on it.
#[test]
fn a_capability_named_like_a_legacy_key_is_still_the_on_shape() {
    for name in [
        "model",
        "filters",
        "priority",
        "Priority",
        "modelFilters",
        "parameters",
        "parameter",
        "Parameter",
    ] {
        let body = format!(r#"{{"h":{{"{name}":{{"on ogit/_id":{{"k":"${{ogit/_id}}"}}}}}}}}"#);
        let direct = parse_applies_on(&body).expect("on shape parses");
        assert_eq!(direct.len(), 1, "{name}");
        assert_eq!(direct[0].capability, name);
        assert_eq!(
            parse_applicabilities_typed(&body),
            Ok(Applicabilities::AppliesOn(direct)),
            "{name}"
        );
    }
}

/// A body mixing both shapes is refused by name.
#[test]
fn mixed_shapes_are_refused() {
    let body = r#"{
        "a": {"priority": 1},
        "b": {"C": {"on x": {}}}
    }"#;
    assert_eq!(
        parse_applicabilities_typed(body),
        Err(ApplicabilityError::MixedShapes)
    );
}

// ───────────────────────────────────────────── malformed bodies ──

#[test]
fn malformed_response_bodies_are_refused_with_a_named_error() {
    assert!(matches!(
        parse_applies_on("not json"),
        Err(ApplicabilityError::NotJson(_))
    ));
    assert_eq!(parse_applies_on("[]"), Err(ApplicabilityError::NotAnObject));
    assert_eq!(
        parse_applies_on(r#"{"h":[]}"#),
        Err(ApplicabilityError::HandlerNotAnObject {
            handler: "h".to_owned()
        })
    );
    assert_eq!(
        parse_applies_on(r#"{"h":{"C":["on x"]}}"#),
        Err(ApplicabilityError::CapabilityNotAnObject {
            handler: "h".to_owned(),
            capability: "C".to_owned()
        })
    );
    assert_eq!(
        parse_applies_on(r#"{"h":{"C":{"at x":{}}}}"#),
        Err(ApplicabilityError::NotAnOnKey {
            handler: "h".to_owned(),
            capability: "C".to_owned(),
            key: "at x".to_owned()
        })
    );
    assert_eq!(
        parse_applies_on(r#"{"h":{"C":{"on ":{}}}}"#),
        Err(ApplicabilityError::EmptyOnExpression {
            handler: "h".to_owned(),
            capability: "C".to_owned()
        })
    );
    assert_eq!(
        parse_applies_on(r#"{"h":{"C":{"on x":["p"]}}}"#),
        Err(ApplicabilityError::ParametersNotAnObject {
            handler: "h".to_owned(),
            capability: "C".to_owned(),
            on: "x".to_owned()
        })
    );
    assert_eq!(
        parse_applies_on(r#"{"h":{"C":{"on x":{"p":5}}}}"#),
        Err(ApplicabilityError::TemplateNotAString {
            handler: "h".to_owned(),
            capability: "C".to_owned(),
            on: "x".to_owned(),
            parameter: "p".to_owned()
        })
    );
}

#[test]
fn malformed_definition_lists_are_refused_with_a_named_error() {
    assert_eq!(
        parse_applies_on_definitions("h", r#"{"capability":"C"}"#),
        Err(ApplicabilityError::DefinitionsNotAnArray)
    );
    assert_eq!(
        parse_applies_on_definitions("h", "[1]"),
        Err(ApplicabilityError::DefinitionNotAnObject { index: 0 })
    );
    assert_eq!(
        parse_applies_on_definitions("h", r#"[{"applicability":["on x"]}]"#),
        Err(ApplicabilityError::DefinitionMissingField {
            index: 0,
            field: "capability"
        })
    );
    assert_eq!(
        parse_applies_on_definitions("h", r#"[{"capability":"C"}]"#),
        Err(ApplicabilityError::DefinitionMissingField {
            index: 0,
            field: "applicability"
        })
    );
    assert_eq!(
        parse_applies_on_definitions("h", r#"[{"capability":3,"applicability":["on x"]}]"#),
        Err(ApplicabilityError::DefinitionCapabilityNotAString { index: 0 })
    );
    // graph_conn's converter accepts one entry only (`mock.ex:626-627`).
    assert_eq!(
        parse_applies_on_definitions(
            "h",
            r#"[{"capability":"C","applicability":["on x","on y"]}]"#
        ),
        Err(ApplicabilityError::DefinitionApplicabilityShape { index: 0 })
    );
    assert_eq!(
        parse_applies_on_definitions(
            "h",
            r#"[{"capability":"C","applicability":{"on x":{},"on y":{}}}]"#
        ),
        Err(ApplicabilityError::DefinitionApplicabilityShape { index: 0 })
    );
    assert_eq!(
        parse_applies_on_definitions("h", r#"[{"capability":"C","applicability":["x"]}]"#),
        Err(ApplicabilityError::NotAnOnKey {
            handler: "h".to_owned(),
            capability: "C".to_owned(),
            key: "x".to_owned()
        })
    );
    // Display names the problem.
    let shown = parse_applies_on("[]").unwrap_err().to_string();
    assert!(shown.contains("not a JSON object"), "{shown}");
}
