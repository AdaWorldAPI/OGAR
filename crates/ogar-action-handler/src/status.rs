//! The `action_status` codes HIRO's public documentation lists, and the one
//! graph_conn adds.
//!
//! This is a table to read, not a behaviour. Which code the handler sends is
//! decided where it always was ([`crate::submit_state::EXEC_TIMEOUT_STATUS`],
//! [`crate::crash::CRASH_STATUS`], [`crate::wire::TOO_BIG_STATUS`]); the tests of
//! `tests/hiro_public_docs.rs` pin that those three equal the constants here.
//!
//! The source of the table is the public HIRO 7.0 page "Action API in Hiro 7"
//! (section "action_status codes in use"). The short meanings in the doc comments
//! are the page's own words. Where the page and graph_conn's handler disagree on a
//! code, the doc comment of that code says so; the code graph_conn sends is kept.
//!
//! `action_status` is "not representation of exit code", and an HTTP 500 from a
//! remote call still gives `action_status` 0 (same page). The page recommends
//! (it does not require) that a result carries `action_status` (a non-negative
//! number) and `action_error` (nil or a string).

/// 0: "action executed".
pub const EXECUTED: u32 = 0;

/// 11: "Action API didn't ack request".
pub const API_DID_NOT_ACK_REQUEST: u32 = 11;

/// 12: "Action API didn't respond".
pub const API_DID_NOT_RESPOND: u32 = 12;

/// 13: "ActionHandler didn't respond" (the page continues "Last status in Action
/// API was ..." with the last status).
///
/// **graph_conn differs.** The docs describe 13 as the Action API reporting that
/// the handler never answered. graph_conn's handler sends 13 itself, for its own
/// execution deadline (`"Execution timed out after <N>ms"`,
/// [`crate::submit_state::EXEC_TIMEOUT_STATUS`]). The session keeps sending 13 for
/// that; the docs' reading is an open question.
pub const HANDLER_DID_NOT_RESPOND: u32 = 13;

/// 14: "ActionHandler responded with execution timeout".
///
/// **graph_conn differs.** The docs give 14 as the handler's execution timeout.
/// graph_conn's handler never sends 14; its execution timeout is 13 (see
/// [`HANDLER_DID_NOT_RESPOND`]). The session sends no 14 either.
pub const HANDLER_RESPONDED_WITH_EXECUTION_TIMEOUT: u32 = 14;

/// 51: "AH configuration problem (proxy misconfigured?)".
pub const HANDLER_CONFIGURATION_PROBLEM: u32 = 51;

/// 52: "Request doesn't match AH config (wrong cert requested?)".
pub const REQUEST_DOES_NOT_MATCH_HANDLER_CONFIG: u32 = 52;

/// 53: "Error preparing request in AH (ah can't build request with provided args)".
pub const ERROR_PREPARING_REQUEST: u32 = 53;

/// 54: "Execution failed/crashed or there was an error with processing response".
/// This is the code graph_conn's handler sends for a crash
/// ([`crate::crash::CRASH_STATUS`]).
pub const EXECUTION_FAILED: u32 = 54;

/// 666: "Unexpected error".
pub const UNEXPECTED_ERROR: u32 = 666;

/// 59: a result replaced because its `sendActionResult` frame was over the
/// frame limit (`Response was too big (X.XX MB, limit Y.YY MB)`).
///
/// **Only graph_conn has this code.** The public docs' list of `action_status`
/// codes does not contain 59 and describes no frame limit
/// ([`crate::wire::TOO_BIG_STATUS`]).
pub const GRAPH_CONN_ONLY_RESPONSE_TOO_BIG: u32 = 59;

/// The codes the public docs list, in the docs' order, each with the docs' short
/// meaning (the page's words; 13 and 14 are the docs' reading, see their
/// constants).
pub const DOCUMENTED: [(u32, &str); 10] = [
    (EXECUTED, "action executed"),
    (API_DID_NOT_ACK_REQUEST, "Action API didn't ack request"),
    (API_DID_NOT_RESPOND, "Action API didn't respond"),
    (HANDLER_DID_NOT_RESPOND, "ActionHandler didn't respond"),
    (
        HANDLER_RESPONDED_WITH_EXECUTION_TIMEOUT,
        "ActionHandler responded with execution timeout",
    ),
    (
        HANDLER_CONFIGURATION_PROBLEM,
        "AH configuration problem (proxy misconfigured?)",
    ),
    (
        REQUEST_DOES_NOT_MATCH_HANDLER_CONFIG,
        "Request doesn't match AH config (wrong cert requested?)",
    ),
    (
        ERROR_PREPARING_REQUEST,
        "Error preparing request in AH (ah can't build request with provided args)",
    ),
    (
        EXECUTION_FAILED,
        "Execution failed/crashed or there was an error with processing response",
    ),
    (UNEXPECTED_ERROR, "Unexpected error"),
];

/// The codes only graph_conn has, with the text it sends.
pub const GRAPH_CONN_ONLY: [(u32, &str); 1] = [(
    GRAPH_CONN_ONLY_RESPONSE_TOO_BIG,
    "Response was too big (X.XX MB, limit Y.YY MB)",
)];

/// The docs' short meaning of `code`, or `None` when the docs do not list it
/// (59 included).
#[must_use]
pub fn documented_meaning(code: u32) -> Option<&'static str> {
    DOCUMENTED
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, meaning)| *meaning)
}
