//! `ogar-action-handler` — the OGAR-native HIRO ActionHandler **runtime**
//! (parity brick **B1**: the executor that actually runs a capability).
//!
//! `ogar-from-schema::action_ws` is the I/O-free protocol core — the typed
//! `action-ws` messages, the `submitAction → ActionInvocation → sendActionResult`
//! binding, and [`handle_submit`] (the reactive dispatch). This crate supplies
//! the **[`CapabilityExecutor`]** the dispatch calls — the only piece that does
//! real I/O — so the producer crate stays pure.
//!
//! [`NativeCommandExecutor`] is the reference target: it runs an
//! `ExecuteCommand` capability's `command` parameter via a local POSIX shell and
//! returns the captured `output` / `stderr` / `exitcode` as `resultParameters` —
//! exactly the shape arago's SSH ActionHandler returns, minus the SSH hop. SSH /
//! REST / WinRM targets follow the same trait; rs-graph-llm's `graph-flow-action`
//! provides the production executors (and runs the RBAC `commit_via` gate before
//! executing). See OGAR `docs/ARAGO-ACTIONHANDLER-PARITY.md`.
//!
//! ```
//! use ogar_action_handler::NativeCommandExecutor;
//! use ogar_from_schema::action_ws::{handle_submit, Receipt, SubmitAction};
//! use ogar_from_schema::do_arm::ActionParam;
//! use ogar_vocab::ActionDef;
//!
//! let def = ActionDef::new("ogit-automation/.../ExecuteCommand", "ExecuteCommand", "ogit-automation/mars_machine");
//! let sig = [ActionParam { name: "command".into(), mandatory: true, default: None }];
//! let msg = SubmitAction {
//!     id: "demo:req-000001".into(), capability: "ExecuteCommand".into(),
//!     handler: "h1".into(), scope: None,
//!     parameters: vec![("command".into(), "echo hi".into())],
//!     timeout_millis: None,
//! };
//! let reaction = handle_submit(&msg, &def, &sig, &NativeCommandExecutor);
//! assert!(matches!(reaction.receipt, Receipt::Acknowledged(_)));
//! let result = reaction.result.unwrap();
//! assert!(result.result.contains("\"output\":\"hi\""));
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::process::Command;

use ogar_from_schema::action_ws::CapabilityExecutor;

/// Reference executor for the **native** target: runs a capability's command via
/// the local POSIX shell (`sh -c`). The concrete B1 impl that makes "OGAR
/// running it here" real for local execution.
///
/// Supported capability: `ExecuteCommand` (the arago SSH handler's core verb) —
/// requires a bound `command` parameter; returns `output` (trimmed stdout),
/// `stderr` (trimmed), and `exitcode`. Any other capability is an error (the
/// caller routed the wrong executor).
///
/// **Trust model:** the executor runs the bound command verbatim — by design
/// (this is what an ActionHandler *is*; arago's SSH handler does the same). The
/// authorization gate (`commit_via<ClassRbac>`: RBAC ∧ state-guard ∧ MUL) runs
/// **upstream** of the executor, in the production `graph-flow-action` impl; this
/// reference executor assumes its caller already authorized the action.
#[derive(Debug, Clone, Copy, Default)]
pub struct NativeCommandExecutor;

impl NativeCommandExecutor {
    /// The single capability this reference executor implements.
    pub const CAPABILITY: &'static str = "ExecuteCommand";
}

impl CapabilityExecutor for NativeCommandExecutor {
    fn execute(
        &self,
        capability: &str,
        bound: &[(String, String)],
    ) -> Result<Vec<(String, String)>, String> {
        if capability != Self::CAPABILITY {
            return Err(format!(
                "native executor implements only `{}`, not `{capability}`",
                Self::CAPABILITY
            ));
        }
        let command = bound
            .iter()
            .find(|(k, _)| k == "command")
            .map(|(_, v)| v.as_str())
            .ok_or_else(|| "missing `command` parameter".to_owned())?;

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .map_err(|e| format!("failed to spawn `{command}`: {e}"))?;

        let trim = |b: &[u8]| String::from_utf8_lossy(b).trim_end().to_owned();
        Ok(vec![
            ("output".to_owned(), trim(&output.stdout)),
            ("stderr".to_owned(), trim(&output.stderr)),
            (
                "exitcode".to_owned(),
                output.status.code().unwrap_or(-1).to_string(),
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_command_runs_and_captures_output() {
        let bound = vec![("command".to_owned(), "echo hello".to_owned())];
        let result = NativeCommandExecutor
            .execute("ExecuteCommand", &bound)
            .expect("runs");
        // resultParameters: output / stderr / exitcode.
        assert_eq!(result[0], ("output".to_owned(), "hello".to_owned()));
        assert_eq!(result[2], ("exitcode".to_owned(), "0".to_owned()));
    }

    #[test]
    fn nonzero_exit_is_captured_not_errored() {
        let bound = vec![("command".to_owned(), "exit 3".to_owned())];
        let result = NativeCommandExecutor
            .execute("ExecuteCommand", &bound)
            .expect("runs");
        assert_eq!(result[2], ("exitcode".to_owned(), "3".to_owned()));
    }

    #[test]
    fn missing_command_param_is_an_error() {
        let err = NativeCommandExecutor
            .execute("ExecuteCommand", &[])
            .unwrap_err();
        assert!(err.contains("command"), "got: {err}");
    }

    #[test]
    fn unknown_capability_is_rejected() {
        let err = NativeCommandExecutor
            .execute("RunScript", &[("command".to_owned(), "x".to_owned())])
            .unwrap_err();
        assert!(err.contains("ExecuteCommand"), "got: {err}");
    }

    /// End-to-end: the dispatch core + this executor run a real command and
    /// produce the `sendActionResult` — "OGAR running it here," native target.
    #[test]
    fn full_dispatch_runs_a_real_command() {
        use ogar_from_schema::action_ws::{Receipt, SubmitAction, handle_submit};
        use ogar_from_schema::do_arm::ActionParam;
        use ogar_vocab::ActionDef;

        let def = ActionDef::new(
            "ogit-automation/action_capability::action_def::ExecuteCommand",
            "ExecuteCommand",
            "ogit-automation/mars_machine",
        );
        let sig = [ActionParam {
            name: "command".to_owned(),
            mandatory: true,
            default: None,
        }];
        let msg = SubmitAction {
            id: "e2e:req-000001".to_owned(),
            capability: "ExecuteCommand".to_owned(),
            handler: "h1".to_owned(),
            scope: None,
            parameters: vec![("command".to_owned(), "echo running".to_owned())],
            timeout_millis: None,
        };

        let reaction = handle_submit(&msg, &def, &sig, &NativeCommandExecutor);
        assert!(matches!(reaction.receipt, Receipt::Acknowledged(_)));
        let result = reaction.result.expect("result");
        assert_eq!(result.id, "e2e:req-000001");
        assert!(
            result.result.contains(r#""output":"running""#),
            "got: {}",
            result.result
        );
    }
}
