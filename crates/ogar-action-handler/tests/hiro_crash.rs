//! A failed execution is `action_status` 54: oracle 6, 7, 8 and 43.
//!
//! The strings are graph_conn's. `G` is
//! `/home/user/bardioc/extraction/corpus/graph_conn`:
//!
//! * `G/test/action_api/crash_test.exs` pins the banners (`crash_test.exs:7-37`).
//! * `G/test/action_api/handler_crash_test.exs` pins the result around them
//!   (`:11-19` for raise, exit and throw; `:55-60` for the cache).
//! * `G/lib/action_api/handler.ex:309`, `:327`, `:356-357` build
//!   `"Execution failed: #{banner}"`, and `:335-339` the
//!   `{"error":{"req_id","action_status","action_error"}}` result.

use std::cell::Cell;
use std::panic::panic_any;
use std::sync::Once;

use ogar_action_handler::crash::{CRASH_STATUS, Crash, FAILURE_PREFIX, Reason};
use ogar_action_handler::session::{Session, run_capability};
use ogar_action_handler::submit_state::{Config, ExecResult, Outcome};
use ogar_from_schema::action_ws::CapabilityExecutor;
use ogar_from_schema::do_arm::ActionParam;
use ogar_vocab::ActionDef;
use serde_json::{Value, json};

/// A panicking executor writes its message through the default panic hook. The
/// tests panic on purpose, so keep the output readable.
fn quiet_panics() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}

fn parse(text: &str) -> Value {
    serde_json::from_str(text).expect("JSON")
}

fn def_and_signature() -> (ActionDef, Vec<ActionParam>) {
    let def = ActionDef::new(
        "ogit-automation/action_capability::action_def::ExecuteCommand",
        "ExecuteCommand",
        "ogit-automation/mars_machine",
    );
    let sig = vec![ActionParam {
        name: "command".to_owned(),
        mandatory: true,
        default: None,
    }];
    (def, sig)
}

fn submit(id: &str) -> String {
    json!({
        "type": "submitAction", "id": id, "handler": "h",
        "capability": "ExecuteCommand", "parameters": {"command": "ls"},
    })
    .to_string()
}

/// The result string of the `sendActionResult` frame among `frames`.
fn result_of(frames: &[String]) -> Value {
    let frame = parse(frames.last().expect("a frame"));
    assert_eq!(frame["type"], "sendActionResult");
    parse(frame["result"].as_str().expect("result is a string"))
}

// ───────────────────────────────────────────────────────── the banners ──

/// `crash_test.exs:7-10`.
#[test]
fn banner_keeps_the_message_a_capability_raised_itself() {
    assert_eq!(
        "** (RuntimeError) capability blew up",
        Crash::RuntimeError("capability blew up".into()).banner()
    );
}

/// Oracle 7. `crash_test.exs:12-16`: a `KeyError` whose term holds a password is
/// named by its module alone.
#[test]
fn banner_names_any_other_exception_by_its_module_alone() {
    assert_eq!(
        "** (KeyError)",
        Crash::Exception("KeyError".into()).banner()
    );
}

/// `crash_test.exs:18-20`: a raw Erlang error is named by the exception it
/// normalises to.
#[test]
fn banner_names_a_normalised_erlang_error_by_its_exception() {
    assert_eq!(
        "** (FunctionClauseError)",
        Crash::Exception("FunctionClauseError".into()).banner()
    );
}

/// Oracle 8. `crash_test.exs:22-26`: an exit or throw reason that is a string or
/// an atom is kept.
#[test]
fn banner_keeps_an_exit_or_throw_reason_that_is_a_string_or_an_atom() {
    assert_eq!(
        r#"** (exit) "capability exited""#,
        Crash::Exit(Reason::Text("capability exited".into())).banner()
    );
    assert_eq!(
        "** (exit) :killed",
        Crash::Exit(Reason::Atom("killed".into())).banner()
    );
    assert_eq!(
        r#"** (throw) "capability threw""#,
        Crash::Throw(Reason::Text("capability threw".into())).banner()
    );
}

/// Oracle 8. `crash_test.exs:28-31`: any other reason is left out.
#[test]
fn banner_leaves_any_other_exit_or_throw_reason_out() {
    assert_eq!("** (exit)", Crash::Exit(Reason::Other).banner());
    assert_eq!("** (throw)", Crash::Throw(Reason::Other).banner());
}

/// `crash_test.exs:33-37`: a crashed task's exception is named the way a raised
/// one is, which is the `Exception` clause.
#[test]
fn banner_of_a_crashed_tasks_exception_is_the_exceptions() {
    assert_eq!(
        "** (KeyError)",
        Crash::Exception("KeyError".into()).banner()
    );
}

/// `inspect/1` of atoms and strings, as far as the banner uses it. Elixir:
/// `inspect(nil)` is `nil`, `inspect(:killed)` is `:killed`, a quote in a string is
/// escaped.
#[test]
fn banner_inspects_atoms_and_strings_like_elixir() {
    assert_eq!(
        "** (exit) nil",
        Crash::Exit(Reason::Atom("nil".into())).banner()
    );
    assert_eq!(
        r#"** (exit) :"not plain""#,
        Crash::Exit(Reason::Atom("not plain".into())).banner()
    );
    assert_eq!(
        r#"** (throw) "a \"b\"\n""#,
        Crash::Throw(Reason::Text("a \"b\"\n".into())).banner()
    );
}

/// The `action_error` is `"Execution failed: "` and the banner
/// (`handler.ex:309`, `:327`, `:357`); the status is 54.
#[test]
fn failure_message_and_result_of_a_crash() {
    assert_eq!("Execution failed: ", FAILURE_PREFIX);
    assert_eq!(54, CRASH_STATUS);
    let crash = Crash::RuntimeError("capability blew up".into());
    assert_eq!(
        "Execution failed: ** (RuntimeError) capability blew up",
        crash.failure_message()
    );
    assert_eq!(
        ExecResult::crashed(&crash),
        ExecResult::Failure {
            code: 54,
            message: "Execution failed: ** (RuntimeError) capability blew up".into(),
        }
    );
}

// ───────────────────────────────────── a Rust executor that fails or panics ──

struct Failing(&'static str);

impl CapabilityExecutor for Failing {
    fn execute(
        &self,
        _capability: &str,
        _bound: &[(String, String)],
    ) -> Result<Vec<(String, String)>, String> {
        Err(self.0.to_owned())
    }
}

/// Counts its runs in a `Cell`, which is not `UnwindSafe`: this is the case
/// `AssertUnwindSafe` in `run_capability` is for. It panics with a message that
/// quotes the secret it was given, as `unwrap` on an `Err` would.
struct Panicking {
    runs: Cell<u32>,
}

impl CapabilityExecutor for Panicking {
    fn execute(
        &self,
        _capability: &str,
        bound: &[(String, String)],
    ) -> Result<Vec<(String, String)>, String> {
        self.runs.set(self.runs.get() + 1);
        panic!("failed on {:?}", bound);
    }
}

/// Oracle 6, with the choice documented in `crash.rs`: an executor `Err(message)`
/// is a `RuntimeError` with its message, status 54.
#[test]
fn an_executor_error_is_status_54_with_the_runtime_error_banner() {
    let (def, sig) = def_and_signature();
    let got = run_capability(
        "ExecuteCommand",
        &[("command".into(), "ls".into())],
        &def,
        &sig,
        &Failing("capability blew up"),
    );
    assert_eq!(
        got,
        ExecResult::Failure {
            code: 54,
            message: "Execution failed: ** (RuntimeError) capability blew up".into(),
        }
    );
}

/// Oracle 6, 8, and the drop-in: a panicking executor does not unwind through
/// `on_text_sync`; the peer gets status 54 with the banner-only text and none of
/// the values the panic quoted.
#[test]
fn a_panicking_executor_is_caught_and_sent_as_status_54_with_no_values() {
    quiet_panics();
    let (def, sig) = def_and_signature();
    let exec = Panicking { runs: Cell::new(0) };
    let mut s = Session::new(Config::default()).unwrap();
    let frames = s
        .on_text_sync(0, &submit("req-panic"), &def, &sig, &exec)
        .expect("the session returns; the panic did not unwind through it");
    assert_eq!(frames.len(), 2);
    assert_eq!(
        parse(&frames[0]),
        json!({"type": "acknowledged", "id": "req-panic", "code": 200, "message": ""})
    );
    assert_eq!(
        result_of(&frames),
        json!({"error": {
            "req_id": "req-panic",
            "action_status": 54,
            "action_error": "Execution failed: ** (exit)",
        }})
    );
    assert!(!frames[1].contains("\"ls\""), "no parameter value is sent");
    assert_eq!(exec.runs.get(), 1);
}

/// A panic with a payload that is not a string is caught as well.
#[test]
fn a_panic_with_any_payload_is_caught() {
    quiet_panics();
    struct Odd;
    impl CapabilityExecutor for Odd {
        fn execute(
            &self,
            _: &str,
            _: &[(String, String)],
        ) -> Result<Vec<(String, String)>, String> {
            panic_any(42_u8)
        }
    }
    let (def, sig) = def_and_signature();
    let got = run_capability(
        "ExecuteCommand",
        &[("command".into(), "ls".into())],
        &def,
        &sig,
        &Odd,
    );
    assert_eq!(
        got,
        ExecResult::Failure {
            code: 54,
            message: "Execution failed: ** (exit)".into(),
        }
    );
}

/// Oracle 43 (`handler_crash_test.exs:55-60`): the crashed result is cached like
/// any result. A repeat is acked and answered with the same result, and the
/// executor does not run a second time, for an error and for a panic.
#[test]
fn a_crashed_result_is_cached_and_a_repeat_does_not_run_again() {
    quiet_panics();
    let (def, sig) = def_and_signature();

    let exec = Panicking { runs: Cell::new(0) };
    let mut s = Session::new(Config::default()).unwrap();
    let first = s
        .on_text_sync(0, &submit("req-43"), &def, &sig, &exec)
        .unwrap();
    let again = s
        .on_text_sync(100, &submit("req-43"), &def, &sig, &exec)
        .unwrap();
    assert_eq!(exec.runs.get(), 1, "the repeat is answered from the cache");
    assert_eq!(again.len(), 2);
    assert_eq!(parse(&again[1]), parse(&first[1]));
    assert_eq!(result_of(&again)["error"]["action_status"], 54);

    let mut s = Session::new(Config::default()).unwrap();
    let err = Failing("capability blew up");
    let first = s
        .on_text_sync(0, &submit("req-43b"), &def, &sig, &err)
        .unwrap();
    let again = s
        .on_text_sync(100, &submit("req-43b"), &def, &sig, &err)
        .unwrap();
    assert_eq!(parse(&again[1]), parse(&first[1]));
    assert_eq!(
        s.state().episode("req-43b").and_then(|e| e.outcome),
        Some(Outcome::Failed(Some(54)))
    );
}

/// The crashed result is resent like any unacked result.
#[test]
fn a_crashed_result_is_resent_until_acked() {
    let (def, sig) = def_and_signature();
    let mut s = Session::new(Config::default()).unwrap();
    s.on_text_sync(0, &submit("req-r"), &def, &sig, &Failing("boom"))
        .unwrap();
    assert_eq!(s.next_wakeup(), Some(3_000));
    let fx = s.on_tick(3_000);
    assert_eq!(fx.len(), 1);
}

/// Parameters that do not bind are not an execution failure: nothing ran, so the
/// one-field result of `handle_submit` stays.
#[test]
fn a_bind_error_is_not_a_crash() {
    let (def, sig) = def_and_signature();
    let got = run_capability("ExecuteCommand", &[], &def, &sig, &Failing("never"));
    let ExecResult::Error {
        code: None,
        payload,
    } = got
    else {
        panic!("expected the bind error result");
    };
    assert_eq!(
        parse(&payload),
        json!({"error": "missing mandatory parameter `command`"})
    );
}
