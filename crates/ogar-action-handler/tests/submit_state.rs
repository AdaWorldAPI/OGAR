//! The stateful submit handler (`ogar_action_handler::submit_state`) against the
//! HIRO parity oracle (`hiro_oracle.md`, section 7). The reference is the Elixir
//! handler: `G` below is `graph_conn/lib/action_api` and `T` is
//! `graph_conn/test/action_api`. Time is injected; nothing here reads a clock.

use std::collections::BTreeMap;

use ogar_action_handler::submit_state::{
    Attempt, AttemptKind, Config, ConfigError, DEFAULT_EXECUTION_TIMEOUT_MS, EXEC_TIMEOUT_STATUS,
    ExecResult, HandlerState, Input, MAX_RECORDED_ATTEMPTS, Millis, Outcome, Output,
    RESEND_INTERVAL_MS, RESULT_TTL_MS, Submit,
};
use ogar_vocab::effect_class::EffectClass;

const TTL: Millis = RESULT_TTL_MS;

fn submit(id: &str, timeout: Option<Millis>) -> Input {
    Input::Submit(Submit::new(
        id,
        "handler-1",
        "ExecuteCommand",
        vec![("command".to_owned(), "ls".to_owned())],
        timeout,
    ))
}

fn finished(id: &str, payload: &str) -> Input {
    Input::Finished {
        id: id.to_owned(),
        result: ExecResult::Success(payload.to_owned()),
    }
}

fn acked(id: &str) -> Input {
    Input::Acked { id: id.to_owned() }
}

fn nacked(id: &str) -> Input {
    Input::Nacked { id: id.to_owned() }
}

/// A handler plus a count of the executions it asked for.
struct Rig {
    st: HandlerState,
    executions: usize,
}

impl Rig {
    fn new() -> Self {
        Self {
            st: HandlerState::new(),
            executions: 0,
        }
    }

    fn step(&mut self, now: Millis, input: Input) -> Vec<Output> {
        let out = self.st.step(now, input);
        self.executions += out
            .iter()
            .filter(|o| matches!(o, Output::StartExecution { .. }))
            .count();
        out
    }
}

fn results(out: &[Output]) -> Vec<(String, u32)> {
    out.iter()
        .filter_map(|o| match o {
            Output::SendResult {
                payload, attempt, ..
            } => Some((payload.clone(), *attempt)),
            _ => None,
        })
        .collect()
}

fn acks(out: &[Output]) -> usize {
    out.iter()
        .filter(|o| matches!(o, Output::SendAck { .. }))
        .count()
}

/// Oracle 1 (`G/handler.ex:168-172`): the first output for a claim is the ack,
/// `code` 200 and `message` `""`, and it comes before the execution starts.
#[test]
fn ack_on_claim_comes_first_with_code_200_and_empty_message() {
    let mut r = Rig::new();
    let out = r.step(0, submit("req-1", None));
    assert_eq!(
        out,
        vec![
            Output::SendAck {
                id: "req-1".to_owned(),
                code: 200,
                message: String::new()
            },
            Output::StartExecution {
                id: "req-1".to_owned(),
                capability: "ExecuteCommand".to_owned(),
                parameters: vec![("command".to_owned(), "ls".to_owned())],
            },
        ]
    );
}

/// Oracle 2 (`G/handler.ex:169-172`, `:193-208`; `T/handler_test.exs:30-45`),
/// the falsifier "a re-sent submitAction runs once", case (a): a repeat while
/// the first is executing is acked, joins the waiters, and starts nothing.
#[test]
fn resent_submit_runs_once_while_in_progress() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    for t in [10, 20, 30] {
        let out = r.step(t, submit("req-1", None));
        assert_eq!(acks(&out), 1, "a duplicate is acked");
        assert_eq!(out.len(), 1, "and nothing else: no execution, no result");
    }
    assert_eq!(r.executions, 1);
    let out = r.step(40, finished("req-1", "{\"ok\":true}"));
    assert_eq!(r.executions, 1);
    assert_eq!(out.len(), 1);
}

/// Oracle 16 and 17 (`G/handler.ex:193-208`, `:224-229`; `T/handler_test.exs:30-45`):
/// with three waiters the duplicates are sent no result while the first runs,
/// and at completion exactly one result goes out, not one per waiter.
#[test]
fn waiters_get_one_result_when_the_execution_completes() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    let mut sent_before = 0;
    for t in [1, 2, 3] {
        sent_before += results(&r.step(t, submit("req-1", None))).len();
    }
    assert_eq!(sent_before, 0);
    let out = r.step(50, finished("req-1", "R"));
    assert_eq!(results(&out), vec![("R".to_owned(), 1)]);
    assert_eq!(r.executions, 1);
}

/// Oracle 2 and 18 (`G/handler.ex:186-188`; `T/handler_test.exs:47-56`), the
/// falsifier case (b): a repeat after completion and within 60 minutes is acked
/// and answered with the stored result at once; the executor is not run again.
#[test]
fn resent_submit_runs_once_after_completion_within_ttl() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    r.step(100, finished("req-1", "STORED"));
    r.step(150, acked("req-1"));
    let out = r.step(TTL / 2, submit("req-1", None));
    assert_eq!(acks(&out), 1);
    assert_eq!(results(&out), vec![("STORED".to_owned(), 2)]);
    assert!(
        matches!(out[0], Output::SendAck { .. }),
        "the ack precedes the cached result"
    );
    assert_eq!(r.executions, 1);
    r.step(TTL / 2 + 10, acked("req-1"));
    // Just inside the window.
    let out = r.step(100 + TTL - 1, submit("req-1", None));
    assert_eq!(results(&out), vec![("STORED".to_owned(), 3)]);
    assert_eq!(r.executions, 1);
}

/// Oracle 19 (`G/handler.ex:72-77`), the falsifier case (c): after 60 minutes
/// from completion the id is new again and executes a second time.
#[test]
fn id_is_new_again_after_the_ttl_so_it_runs_twice() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    r.step(100, finished("req-1", "FIRST"));
    r.step(150, acked("req-1"));
    assert_eq!(r.st.tracked(), 1);
    let out = r.step(100 + TTL, submit("req-1", None));
    assert!(
        out.iter()
            .any(|o| matches!(o, Output::StartExecution { .. }))
    );
    assert!(results(&out).is_empty());
    assert_eq!(r.executions, 2);
}

/// Oracle 19: the 60 minutes count from completion, not from the claim
/// (`G/handler.ex:210-217` puts the result again, which restarts the Cachex
/// expiry), and reads of the cache do not extend it (`:186-188` only reads).
#[test]
fn ttl_counts_from_completion_and_reads_do_not_extend_it() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", Some(1_000_000)));
    r.step(900_000, finished("req-1", "R"));
    r.step(900_001, acked("req-1"));
    // 60 minutes after the claim, but only 45 after completion: still cached.
    r.step(TTL, submit("req-1", None));
    assert_eq!(r.executions, 1);
    // A read at 900_000 + TTL - 1 does not push the end of the window out.
    r.step(900_000 + TTL - 1, submit("req-1", None));
    r.step(900_001, Input::Tick);
    let out = r.step(900_000 + TTL, submit("req-1", None));
    assert!(
        out.iter()
            .any(|o| matches!(o, Output::StartExecution { .. }))
    );
    assert_eq!(r.executions, 2);
}

/// Oracle 19: an id whose result was never acked is still resent after the
/// window closes (the responder has no give-up), but is new again for dedupe;
/// the second claim starts a second execution and retires the first episode.
#[test]
fn unacked_id_is_new_again_after_the_ttl() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    r.step(100, finished("req-1", "OLD"));
    let out = r.step(100 + TTL, submit("req-1", None));
    assert_eq!(r.executions, 2);
    // The one resend due at that instant is the old result, sent before the
    // new claim; the new claim sends no stored result.
    assert_eq!(results(&out), vec![("OLD".to_owned(), 2)]);
    assert_eq!(r.st.drain_finished().len(), 1, "the old episode is retired");
    // The new execution's result replaces the old one for resends.
    let out = r.step(100 + TTL + 10, finished("req-1", "NEW"));
    assert_eq!(results(&out), vec![("NEW".to_owned(), 1)]);
}

/// Oracle 12 (`G/responder.ex:143-163`, `G/handler.ex:365-366`;
/// `T/responder_resend_test.exs:28-37`): the result is sent at completion and
/// then every 3000 ms exactly, with the attempt number rising by one, and no
/// give-up.
#[test]
fn result_is_resent_every_3000ms_with_rising_attempts() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    let out = r.step(1_000, finished("req-1", "R"));
    assert_eq!(results(&out), vec![("R".to_owned(), 1)]);
    assert_eq!(r.st.next_wakeup(), Some(1_000 + RESEND_INTERVAL_MS));
    // Nothing a millisecond early.
    assert!(r.step(1_000 + 2_999, Input::Tick).is_empty());
    for n in 1..=50u32 {
        let at = 1_000 + u64::from(n) * 3_000;
        let out = r.step(at, Input::Tick);
        assert_eq!(results(&out), vec![("R".to_owned(), n + 1)], "step {n}");
        assert!(r.step(at + 2_999, Input::Tick).is_empty());
        assert_eq!(r.st.next_wakeup(), Some(at + 3_000));
    }
}

/// Oracle 12 (`G/responder.ex:150-153`): an overdue resend is sent once and the
/// next is an interval after the time it was handled, not a burst.
#[test]
fn a_late_tick_sends_one_resend_not_a_burst() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    r.step(0, finished("req-1", "R"));
    let out = r.step(10_000, Input::Tick);
    assert_eq!(results(&out), vec![("R".to_owned(), 2)]);
    assert_eq!(r.st.next_wakeup(), Some(13_000));
}

/// Oracle 13 (`G/handler.ex:109-110`, `G/responder.ex:124-139`;
/// `T/responder_resend_test.exs:49-59`): an ack cancels the resend. After it,
/// ticks produce nothing, and the episode records the ack.
#[test]
fn ack_stops_the_resends() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    r.step(0, finished("req-1", "R"));
    assert_eq!(results(&r.step(3_000, Input::Tick)).len(), 1);
    assert!(r.step(3_500, acked("req-1")).is_empty());
    for t in [6_000, 9_000, 60_000, 600_000] {
        assert!(r.step(t, Input::Tick).is_empty(), "no resend at {t}");
    }
    let ep =
        r.st.episode("req-1")
            .expect("still open until the window closes");
    assert!(ep.acknowledged);
    assert!(!ep.nacked);
    assert_eq!(ep.result_deliveries, 2);
}

/// Oracle 14 (`G/handler.ex:112-119`): a NACK of the result also stops the
/// resend, and is not recorded as an ack.
#[test]
fn nack_stops_the_resends() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    r.step(0, finished("req-1", "R"));
    assert!(r.step(1_000, nacked("req-1")).is_empty());
    for t in [3_000, 6_000, 60_000] {
        assert!(r.step(t, Input::Tick).is_empty(), "no resend at {t}");
    }
    let ep = r.st.episode("req-1").expect("open");
    assert!(ep.nacked);
    assert!(!ep.acknowledged);
}

/// Oracle 13 (`G/responder.ex:129-130`): an ack for an id with nothing pending
/// (unknown, still executing, or already acked) does nothing.
#[test]
fn ack_for_nothing_pending_is_a_no_op() {
    let mut r = Rig::new();
    assert!(r.step(0, acked("ghost")).is_empty());
    assert_eq!(r.st.tracked(), 0);
    r.step(0, submit("req-1", None));
    assert!(r.step(5, acked("req-1")).is_empty());
    assert!(r.step(6, nacked("req-1")).is_empty());
    let ep = r.st.episode("req-1").expect("open");
    assert!(!ep.acknowledged && !ep.nacked);
    // And it still runs to a result afterwards.
    assert_eq!(results(&r.step(7, finished("req-1", "R"))).len(), 1);
}

/// Oracle 18 (`G/responder.ex:85`, `:110-111`): a repeat after the result was
/// acked is answered with the stored result, and the resend then runs again
/// until that is acked too.
#[test]
fn cached_answer_restarts_the_resend_until_acked() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    r.step(0, finished("req-1", "R"));
    r.step(10, acked("req-1"));
    assert!(r.step(5_000, Input::Tick).is_empty());
    let out = r.step(6_000, submit("req-1", None));
    assert_eq!(results(&out), vec![("R".to_owned(), 2)]);
    assert_eq!(
        results(&r.step(9_000, Input::Tick)),
        vec![("R".to_owned(), 3)]
    );
    r.step(9_100, acked("req-1"));
    assert!(r.step(20_000, Input::Tick).is_empty());
}

/// Oracle 18 (`G/responder.ex:110-111`): a repeat while the result is still
/// pending keeps the running timer; the immediate send goes out but the resend
/// schedule is not reset.
#[test]
fn a_repeat_while_unacked_keeps_the_resend_timer() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    r.step(0, finished("req-1", "R"));
    let out = r.step(1_000, submit("req-1", None));
    assert_eq!(results(&out), vec![("R".to_owned(), 2)]);
    assert_eq!(r.st.next_wakeup(), Some(3_000));
    assert_eq!(
        results(&r.step(3_000, Input::Tick)),
        vec![("R".to_owned(), 3)]
    );
}

/// Oracle 3 (`G/handler.ex:133`, `:361-362`) and 5 (`:311-313`): with no
/// `timeout` the deadline is 60000 ms from the submit. One millisecond before,
/// nothing; at 60000 the error result is sent: `action_status` 13 and the
/// exact message.
#[test]
fn default_deadline_is_60000ms() {
    assert_eq!(DEFAULT_EXECUTION_TIMEOUT_MS, 60_000);
    let mut r = Rig::new();
    r.step(5, submit("req-1", None));
    assert_eq!(r.st.next_wakeup(), Some(60_005));
    assert!(r.step(60_004, Input::Tick).is_empty());
    let out = r.step(60_005, Input::Tick);
    let sent = results(&out);
    assert_eq!(sent.len(), 1);
    let v: serde_json::Value = serde_json::from_str(&sent[0].0).expect("json");
    assert_eq!(
        v,
        serde_json::json!({"error": {
            "req_id": "req-1",
            "action_status": 13,
            "action_error": "Execution timed out after 60000ms",
        }})
    );
    let ep = r.st.episode("req-1").expect("open");
    assert_eq!(ep.outcome, Some(Outcome::TimedOut));
    assert_eq!(ep.finished_at, Some(60_005));
}

/// Oracle 4 (`G/handler.ex:133`): the wire `timeout` is milliseconds; 123000
/// gives a deadline of 123000 ms, and the message names it. Oracle 5: a finish
/// at or after the deadline is late and is ignored (the task was killed,
/// `handler.ex:301`); the timeout result stands and is what a repeat receives.
#[test]
fn explicit_timeout_is_milliseconds_and_a_late_finish_is_ignored() {
    assert_eq!(EXEC_TIMEOUT_STATUS, 13);
    let mut r = Rig::new();
    r.step(0, submit("req-1", Some(123_000)));
    assert!(r.step(122_999, Input::Tick).is_empty());
    let out = r.step(123_000, finished("req-1", "TOO LATE"));
    let sent = results(&out);
    assert_eq!(sent.len(), 1, "the timeout result, not the late one");
    assert!(
        sent[0]
            .0
            .contains("\"action_error\":\"Execution timed out after 123000ms\"")
    );
    let ep = r.st.episode("req-1").expect("open");
    assert_eq!(ep.outcome, Some(Outcome::TimedOut));
    // A redelivery is answered with the cached timeout result, not a rerun.
    let out = r.step(130_000, submit("req-1", None));
    assert_eq!(results(&out)[0].0, sent[0].0);
    assert_eq!(r.executions, 1);
}

/// Oracle 5: a finish before the deadline wins, and the deadline then does not
/// fire.
#[test]
fn a_finish_before_the_deadline_wins() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", Some(1_000)));
    let out = r.step(999, finished("req-1", "R"));
    assert_eq!(results(&out), vec![("R".to_owned(), 1)]);
    assert_eq!(
        r.st.episode("req-1").and_then(|e| e.outcome),
        Some(Outcome::Succeeded)
    );
    // At and after 1000 the timer is the resend, not a timeout.
    let out = r.step(4_000, Input::Tick);
    assert_eq!(results(&out), vec![("R".to_owned(), 2)]);
}

/// Oracle 5 (`G/handler.ex:133`): `msg["timeout"] || default` treats only `nil`
/// as absent, so `Some(0)` is a deadline of 0 ms and fires in the same step.
#[test]
fn a_zero_timeout_is_a_zero_deadline_not_the_default() {
    let mut r = Rig::new();
    let out = r.step(7, submit("req-1", Some(0)));
    assert_eq!(acks(&out), 1);
    let sent = results(&out);
    assert_eq!(sent.len(), 1);
    assert!(sent[0].0.contains("Execution timed out after 0ms"));
}

/// Oracle 43 (`G/handler.ex:335-339`, `T/handler_crash_test.exs:55-60`) and 6:
/// a failed execution is a result in the `{"error":{req_id,action_status,
/// action_error}}` shape, cached like any result, and a redelivery is answered
/// with it. The episode outcome carries the status.
#[test]
fn a_failure_is_cached_as_its_error_result() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    let out = r.step(
        10,
        Input::Finished {
            id: "req-1".to_owned(),
            result: ExecResult::Failure {
                code: 54,
                message: "Execution failed: ** (RuntimeError) boom".to_owned(),
            },
        },
    );
    let first = results(&out)[0].0.clone();
    let v: serde_json::Value = serde_json::from_str(&first).expect("json");
    assert_eq!(v["error"]["req_id"], "req-1");
    assert_eq!(v["error"]["action_status"], 54);
    assert_eq!(
        v["error"]["action_error"],
        "Execution failed: ** (RuntimeError) boom"
    );
    let out = r.step(20, submit("req-1", None));
    assert_eq!(results(&out)[0].0, first);
    assert_eq!(r.executions, 1);
    assert_eq!(
        r.st.episode("req-1").and_then(|e| e.outcome),
        Some(Outcome::Failed(Some(54)))
    );
}

/// Oracle 9 (`G/handler.ex:278-285`, `:369-371`): an executor `{:error, term}`
/// is a result the caller has encoded; it is sent as is and is not a NACK.
#[test]
fn an_executor_error_is_a_result_sent_as_is() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    let payload = r#"{"error":{"code":404,"message":"Unhandled capability \"X\""}}"#;
    let out = r.step(
        1,
        Input::Finished {
            id: "req-1".to_owned(),
            result: ExecResult::Error {
                code: Some(404),
                payload: payload.to_owned(),
            },
        },
    );
    assert_eq!(
        out,
        vec![Output::SendResult {
            id: "req-1".to_owned(),
            payload: payload.to_owned(),
            attempt: 1
        }]
    );
    assert_eq!(
        r.st.episode("req-1").and_then(|e| e.outcome),
        Some(Outcome::Failed(Some(404)))
    );
}

/// Oracle 37 (`G/request.ex:51-58`; `hiro_oracle.md` section 9 last bullet): ids
/// are opaque. Ids that differ only in case, whitespace, or normalization are
/// different ids; nothing is parsed; a 256 character id and a non-ASCII id are
/// handled like any other.
#[test]
fn request_ids_are_opaque_strings() {
    let ids = [
        "app:req-000001",
        "APP:req-000001",
        "app:req-000001 ",
        " app:req-000001",
        "app:req-1",
        "",
        "caf\u{e9}",
        "cafe\u{301}",
        &"x".repeat(256),
        "a\u{0}b",
    ];
    let mut r = Rig::new();
    for (i, id) in ids.iter().enumerate() {
        let out = r.step(i as Millis, submit(id, None));
        assert!(
            out.iter()
                .any(|o| matches!(o, Output::StartExecution { .. })),
            "{id:?} is a new id"
        );
    }
    assert_eq!(r.executions, ids.len());
    assert_eq!(r.st.tracked(), ids.len());
    // Each result goes back under exactly the id it came in with.
    for id in ids {
        let out = r.step(100, finished(id, "R"));
        assert!(matches!(&out[0], Output::SendResult { id: got, .. } if got == id));
    }
}

/// Oracle 5c (`hiro_oracle.md` section 5c): the handler sends no NACK for a
/// submit. Every failure is a result.
#[test]
fn the_machine_never_sends_a_nack() {
    let mut r = Rig::new();
    let mut all = Vec::new();
    all.extend(r.step(0, submit("a", Some(0))));
    all.extend(r.step(1, submit("a", None)));
    all.extend(r.step(2, nacked("a")));
    all.extend(r.step(
        3,
        Input::Finished {
            id: "a".to_owned(),
            result: ExecResult::Failure {
                code: 54,
                message: "x".to_owned(),
            },
        },
    ));
    all.extend(r.step(TTL * 2, Input::Tick));
    assert!(!all.is_empty());
    assert!(!all.iter().any(|o| matches!(o, Output::SendNack { .. })));
}

/// Oracle 15 (`G/responder.ex:88-94`; `T/responder_resend_test.exs:61-80`): a
/// resend interval of 0 is refused, naming the callback, and a positive one is
/// honoured.
#[test]
fn a_zero_resend_interval_is_refused() {
    let bad = Config {
        resend_interval_ms: 0,
        ..Config::default()
    };
    let err = HandlerState::with_config(bad).expect_err("refused");
    assert_eq!(err, ConfigError::NonPositiveResendInterval(0));
    assert!(err.to_string().contains("resend_response_timeout/0"));
    assert!(err.to_string().contains('0'));
    let ok = Config {
        resend_interval_ms: 500,
        ..Config::default()
    };
    let mut st = HandlerState::with_config(ok).expect("accepted");
    st.step(0, submit("req-1", None));
    st.step(0, finished("req-1", "R"));
    assert_eq!(st.next_wakeup(), Some(500));
}

/// Oracle 19 and the bound: state is bounded by expiry. Many ids, all finished
/// and acked, are all gone once their window has closed; the episodes are moved
/// to the finished list (not counted as handler state) and drain empties it.
#[test]
fn state_is_bounded_by_expiry() {
    let mut r = Rig::new();
    for i in 0..200u64 {
        let id = format!("req-{i}");
        r.step(i, submit(&id, None));
        r.step(i + 1, finished(&id, "R"));
        r.step(i + 2, acked(&id));
    }
    assert_eq!(r.st.tracked(), 200);
    assert_eq!(r.st.finished_episodes().count(), 0);
    // Just before the first window closes, nothing is gone.
    r.step(1 + TTL - 1, Input::Tick);
    assert_eq!(r.st.tracked(), 200);
    // After the last window closes, nothing is tracked and nothing is due.
    r.step(200 + TTL, Input::Tick);
    assert_eq!(r.st.tracked(), 0);
    assert_eq!(r.st.next_wakeup(), None);
    assert_eq!(r.st.finished_episodes().count(), 200);
    assert_eq!(r.st.drain_finished().len(), 200);
    assert_eq!(r.st.finished_episodes().count(), 0);
}

/// Oracle 19: state stays bounded over a long run. A new id every second for
/// ten hours, each finished and acked, holds at most one window's worth of ids
/// (3600) at any step.
#[test]
fn state_does_not_grow_over_a_long_run() {
    let mut r = Rig::new();
    let mut peak = 0;
    for i in 0..36_000u64 {
        let t = i * 1_000;
        let id = format!("req-{i}");
        r.step(t, submit(&id, None));
        r.step(t + 1, finished(&id, "R"));
        r.step(t + 2, acked(&id));
        peak = peak.max(r.st.tracked());
        r.st.drain_finished();
    }
    assert!(peak <= 3_601, "peak {peak}");
    assert!(r.st.tracked() <= 3_601);
}

/// Oracle 12 (no give-up) against the bound: an unacked result stays tracked
/// and is resent, because the responder never gives up; the episode's attempt
/// list is capped, with the overflow counted, and the numbers keep rising.
#[test]
fn an_unacked_result_is_resent_for_ever_with_a_capped_attempt_list() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    r.step(0, finished("req-1", "R"));
    let rounds = (MAX_RECORDED_ATTEMPTS as u64) * 3;
    for n in 1..=rounds {
        let out = r.step(n * 3_000, Input::Tick);
        assert_eq!(results(&out).len(), 1);
    }
    let ep = r.st.episode("req-1").expect("still tracked");
    assert_eq!(ep.attempts.len(), MAX_RECORDED_ATTEMPTS);
    assert_eq!(ep.result_deliveries as u64, rounds + 1);
    assert_eq!(
        ep.attempts_dropped(),
        1 + rounds + 1 - MAX_RECORDED_ATTEMPTS as u64
    );
    let nums: Vec<u32> = ep
        .attempts
        .iter()
        .filter(|a| a.kind == AttemptKind::ResultDelivery)
        .map(|a| a.number)
        .collect();
    assert!(nums.windows(2).all(|w| w[1] == w[0] + 1));
}

/// Episodes: one intervention per request id, whatever the number of retries.
/// A first delivery, two repeats while executing, the result, two resends and a
/// repeat after completion are seven attempts of one episode with the context of
/// the first delivery.
#[test]
fn one_intervention_with_n_attempts() {
    let mut r = Rig::new();
    let mut first = Submit::new("req-1", "handler-7", "ExecuteCommand", vec![], None);
    first.class_id = Some(42);
    first.target = Some("node-9".to_owned());
    first.effect_class = Some(EffectClass::ExternalHost);
    r.step(0, Input::Submit(first));
    // The repeats carry other optional context; the first claim's stands.
    r.step(10, submit("req-1", None));
    r.step(20, submit("req-1", None));
    r.step(30, finished("req-1", "R"));
    r.step(3_030, Input::Tick);
    r.step(6_030, Input::Tick);
    r.step(6_040, acked("req-1"));
    r.step(7_000, submit("req-1", None));

    assert_eq!(r.executions, 1);
    assert_eq!(r.st.open_episodes().count(), 1);
    let ep = r.st.episode("req-1").expect("episode");
    assert_eq!(ep.intervention_id, "req-1");
    assert_eq!(ep.handler_id, "handler-7");
    assert_eq!(ep.capability, "ExecuteCommand");
    assert_eq!(ep.class_id, Some(42));
    assert_eq!(ep.target.as_deref(), Some("node-9"));
    assert_eq!(ep.effect_class, Some(EffectClass::ExternalHost));
    assert_eq!(ep.started_at, 0);
    assert_eq!(ep.finished_at, Some(30));
    assert_eq!(ep.outcome, Some(Outcome::Succeeded));
    assert!(ep.acknowledged);
    let req = |number, at| Attempt {
        number,
        kind: AttemptKind::RequestDelivery,
        at,
    };
    let res = |number, at| Attempt {
        number,
        kind: AttemptKind::ResultDelivery,
        at,
    };
    assert_eq!(
        ep.attempts,
        vec![
            req(1, 0),
            req(2, 10),
            req(3, 20),
            res(1, 30),
            res(2, 3_030),
            res(3, 6_030),
            req(4, 7_000),
            res(4, 7_000),
        ]
    );
    assert_eq!(ep.attempts_dropped(), 0);
}

/// Episodes: a finished episode is drained once its window has closed and its
/// result is settled, and a re-claim after expiry is a second episode, not a
/// continuation. Context left unset stays `None`.
#[test]
fn drained_episodes_are_separate_interventions_per_claim() {
    let mut r = Rig::new();
    r.step(0, submit("req-1", None));
    r.step(5, finished("req-1", "R"));
    r.step(6, nacked("req-1"));
    r.step(5 + TTL, submit("req-1", None));
    let done = r.st.drain_finished();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].intervention_id, "req-1");
    assert!(done[0].nacked && !done[0].acknowledged);
    assert_eq!(done[0].class_id, None);
    assert_eq!(done[0].target, None);
    assert_eq!(done[0].effect_class, None);
    let open = r.st.episode("req-1").expect("second claim");
    assert_eq!(open.started_at, 5 + TTL);
    assert_eq!(open.request_deliveries, 1);
    assert_eq!(open.outcome, None);
}

// ---------------------------------------------------------------------------
// Property-style test: a seeded random interleaving against a reference model.
// ---------------------------------------------------------------------------

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

/// The reference: one record per id, every timer found by scanning all of
/// them. No timer set, no episode bookkeeping; the same rules stated plainly.
#[derive(Default)]
struct Model {
    clock: Millis,
    ids: BTreeMap<String, M>,
    claims: usize,
}

enum M {
    Executing {
        deadline: Millis,
        timeout: Millis,
    },
    Done {
        payload: String,
        completed: Millis,
        next_resend: Option<Millis>,
    },
}

impl Model {
    fn due(m: &M) -> Millis {
        match m {
            M::Executing { deadline, .. } => *deadline,
            M::Done {
                next_resend: Some(t),
                ..
            } => *t,
            M::Done {
                completed,
                next_resend: None,
                ..
            } => completed + TTL,
        }
    }

    fn next_wakeup(&self) -> Option<Millis> {
        self.ids.values().map(Self::due).min()
    }

    fn timers(&mut self, now: Millis, out: &mut Vec<Output>) {
        loop {
            let next = self
                .ids
                .iter()
                .map(|(id, m)| (Self::due(m), id.clone()))
                .min();
            let Some((due, id)) = next else { return };
            if due > now {
                return;
            }
            match self.ids.get_mut(&id).expect("present") {
                M::Executing { timeout, .. } => {
                    let payload = format!(
                        r#"{{"error":{{"req_id":"{id}","action_status":13,"action_error":"Execution timed out after {timeout}ms"}}}}"#
                    );
                    out.push(Output::SendResult {
                        id: id.clone(),
                        payload: payload.clone(),
                        attempt: 1,
                    });
                    *self.ids.get_mut(&id).expect("present") = M::Done {
                        payload,
                        completed: now,
                        next_resend: Some(now + 3_000),
                    };
                }
                M::Done {
                    next_resend: Some(t),
                    ..
                } => {
                    *t = now + 3_000;
                    // The attempt number is filled in by the caller's counter.
                    out.push(Output::SendResult {
                        id: id.clone(),
                        payload: String::new(),
                        attempt: 0,
                    });
                }
                M::Done {
                    next_resend: None, ..
                } => {
                    self.ids.remove(&id);
                }
            }
        }
    }
}

/// A model with per-id result attempt counters, driven like the handler.
struct Reference {
    m: Model,
    attempts: BTreeMap<String, u32>,
}

impl Reference {
    fn fix(&mut self, out: &mut [Output]) {
        for o in out {
            if let Output::SendResult {
                id,
                payload,
                attempt,
            } = o
            {
                let n = self.attempts.entry(id.clone()).or_insert(0);
                *n += 1;
                *attempt = *n;
                if payload.is_empty()
                    && let Some(M::Done { payload: p, .. }) = self.m.ids.get(id.as_str())
                {
                    payload.clone_from(p);
                }
            }
        }
    }

    fn step(&mut self, now: Millis, input: &Input) -> Vec<Output> {
        let now = now.max(self.m.clock);
        self.m.clock = now;
        let mut out = Vec::new();
        self.drive(now, input, &mut out);
        out
    }

    /// Timers, the input, timers again; a run of `fix` after each phase keeps
    /// the attempt counters in the order the outputs are produced.
    fn drive(&mut self, now: Millis, input: &Input, out: &mut Vec<Output>) {
        let mut phase = Vec::new();
        self.m.timers(now, &mut phase);
        self.fix(&mut phase);
        out.extend(phase);

        let mut phase = Vec::new();
        match input {
            Input::Submit(s) => self.submit(now, s, &mut phase),
            Input::Finished { id, result } => {
                if let Some(M::Executing { .. }) = self.m.ids.get(id) {
                    let payload = match result {
                        ExecResult::Success(p) => p.clone(),
                        _ => unreachable!("the driver only sends Success"),
                    };
                    phase.push(Output::SendResult {
                        id: id.clone(),
                        payload: payload.clone(),
                        attempt: 0,
                    });
                    self.m.ids.insert(
                        id.clone(),
                        M::Done {
                            payload,
                            completed: now,
                            next_resend: Some(now + 3_000),
                        },
                    );
                }
            }
            Input::Acked { id } | Input::Nacked { id } => {
                if let Some(M::Done {
                    completed,
                    next_resend,
                    ..
                }) = self.m.ids.get_mut(id)
                    && next_resend.take().is_some()
                    && *completed + TTL <= now
                {
                    self.m.ids.remove(id);
                }
            }
            Input::Tick => {}
        }
        self.fix(&mut phase);
        out.extend(phase);

        let mut phase = Vec::new();
        self.m.timers(now, &mut phase);
        self.fix(&mut phase);
        out.extend(phase);
    }

    fn submit(&mut self, now: Millis, s: &Submit, out: &mut Vec<Output>) {
        let id = &s.id;
        if let Some(M::Done { completed, .. }) = self.m.ids.get(id)
            && *completed + TTL <= now
        {
            self.m.ids.remove(id);
            self.attempts.remove(id);
        }
        out.push(Output::SendAck {
            id: id.clone(),
            code: 200,
            message: String::new(),
        });
        match self.m.ids.get_mut(id) {
            None => {
                self.m.claims += 1;
                let timeout = s.timeout_ms.unwrap_or(60_000);
                out.push(Output::StartExecution {
                    id: id.clone(),
                    capability: s.capability.clone(),
                    parameters: s.parameters.clone(),
                });
                self.attempts.remove(id);
                self.m.ids.insert(
                    id.clone(),
                    M::Executing {
                        deadline: now + timeout,
                        timeout,
                    },
                );
            }
            Some(M::Executing { .. }) => {}
            Some(M::Done {
                payload,
                next_resend,
                ..
            }) => {
                out.push(Output::SendResult {
                    id: id.clone(),
                    payload: payload.clone(),
                    attempt: 0,
                });
                if next_resend.is_none() {
                    *next_resend = Some(now + 3_000);
                }
            }
        }
    }
}

fn random_input(rng: &mut Rng, serial: &mut u64) -> Input {
    let id = format!("id-{}", rng.below(6));
    match rng.below(10) {
        0..=3 => {
            let timeout = match rng.below(6) {
                0 => Some(0),
                1 => Some(50),
                2 => Some(3_000),
                3 => Some(7_500),
                _ => None,
            };
            submit(&id, timeout)
        }
        4..=5 => {
            *serial += 1;
            finished(&id, &format!("result-{serial}"))
        }
        6 => acked(&id),
        7 => nacked(&id),
        _ => Input::Tick,
    }
}

fn random_advance(rng: &mut Rng) -> Millis {
    match rng.below(20) {
        0 => 0,
        1..=6 => 1,
        7..=11 => 999,
        12..=14 => 3_000,
        15 => 3_001,
        16 => 30_000,
        17 => TTL - 1,
        18 => TTL,
        _ => 61_000,
    }
}

/// Oracles 1, 2, 5, 12-14, 16-19 together: 20000 random steps over six ids with
/// the clock jumping across the 3000 ms and 60 minute boundaries, against the
/// reference model. Outputs, the next wakeup, the tracked count and the number of
/// executions agree at every step; the machine never sends a NACK; every episode
/// counts the request deliveries it was given.
#[test]
fn random_interleaving_agrees_with_the_reference_model() {
    for seed in [1u64, 2, 3, 0xDEAD_BEEF, 0x1234_5678_9ABC_DEF1] {
        let mut rng = Rng(seed);
        let mut st = HandlerState::new();
        let mut reference = Reference {
            m: Model::default(),
            attempts: BTreeMap::new(),
        };
        let mut now: Millis = 0;
        let mut serial = 0;
        let mut starts = 0usize;
        let mut submits = 0u64;
        let mut episodes = Vec::new();
        for step in 0..20_000 {
            now += random_advance(&mut rng);
            let input = random_input(&mut rng, &mut serial);
            if matches!(input, Input::Submit(_)) {
                submits += 1;
            }
            let want = reference.step(now, &input);
            let got = st.step(now, input.clone());
            assert_eq!(got, want, "seed {seed:#x} step {step} input {input:?}");
            assert_eq!(
                st.tracked(),
                reference.m.ids.len(),
                "seed {seed:#x} step {step}"
            );
            assert_eq!(
                st.next_wakeup(),
                reference.m.next_wakeup(),
                "seed {seed:#x} step {step}"
            );
            starts += got
                .iter()
                .filter(|o| matches!(o, Output::StartExecution { .. }))
                .count();
            assert!(!got.iter().any(|o| matches!(o, Output::SendNack { .. })));
            if step % 500 == 0 {
                episodes.extend(st.drain_finished());
            }
        }
        assert_eq!(starts, reference.m.claims, "seed {seed:#x}");
        episodes.extend(st.drain_finished());
        let open: Vec<_> = st.open_episodes().cloned().collect();
        assert_eq!(open.len(), st.tracked());
        episodes.extend(open);
        // One episode per claim, and every submit is a request delivery of one.
        assert_eq!(episodes.len(), starts, "seed {seed:#x}");
        let deliveries: u64 = episodes
            .iter()
            .map(|e| u64::from(e.request_deliveries))
            .sum();
        assert_eq!(deliveries, submits, "seed {seed:#x}");
        for e in &episodes {
            let mut last = [0u32; 2];
            for a in &e.attempts {
                let k = usize::from(a.kind == AttemptKind::ResultDelivery);
                assert!(a.number > last[k], "attempt numbers rise within a kind");
                last[k] = a.number;
            }
        }
    }
}
