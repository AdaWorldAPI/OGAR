//! The closed loop: the OGAR invoker, the mock gateway and the OGAR handler in one
//! process, joined by text frames and a virtual clock.
//!
//! The mock is graph_conn's test mock (`lib/graph_conn/test/mock_socket.ex`), not
//! a gateway; what these scenarios prove is that the two OGAR machines interoperate
//! through a double with the mock's behaviour, and nothing about the real gateway
//! (oracle section 9). `S` is `mock_socket.ex`, `H` `lib/action_api/handler.ex`, `I`
//! `lib/action_api/invoker.ex`, `R` `responder.ex`, `M` `lib/graph_conn/mock.ex`.
//!
//! Each scenario asserts what the caller finally receives, how many times the
//! capability ran, the frame trace where order matters, that all three machines
//! end with nothing tracked beyond HIRO's 60 minute cache (`H:72-77`), and the
//! handler's episode for the request.
//!
//! Timing the scenarios rely on, all from the machines' own constants: the mock
//! acks and forwards 1 ms after a known submit (`S:264-266`, the bottom of its
//! 1 to 5 ms); the invoker waits 3000 ms for an ack and sends 3 times
//! (`I:109-110`), then waits the request timeout plus 1000 ms for the response
//! (`I:737-738`); the handler resends an unacked result every 3000 ms
//! (`H:365-366`).

#[path = "closed_loop/harness.rs"]
mod harness;

use harness::{Behavior, CAPABILITY, ClosedLoop, HANDLER_ID, kind, parse};
use ogar_action_handler::discovery::{
    Draw, FetchInput, FetchKind, FetchOutcome, FetchOutput, FetchState,
};
use ogar_action_handler::invoke_state::{CallRequest, InvokeError, Log};
use ogar_action_handler::mock_gateway::{
    Direction, FORBIDDEN_CAPABILITY, IdField, MockConfig, MockError, Side,
};
use ogar_action_handler::submit_state::{AttemptKind, Outcome, RESULT_TTL_MS};
use serde_json::{Value, json};

use Direction::{GatewayToHandler, GatewayToInvoker, HandlerToGateway, InvokerToGateway};

fn done() -> Value {
    json!({"output": "done"})
}

fn handler_error(c: &harness::ClosedLoop, call: u64) -> Value {
    match &c.outcome(call).result {
        Err(InvokeError::Handler(v)) => v.clone(),
        other => panic!("call {call}: {other:?}"),
    }
}

/// The frames of the whole trace as `(time, direction, kind)`.
fn trace(lp: &ClosedLoop) -> Vec<(u64, Direction, String)> {
    lp.trace
        .iter()
        .map(|t| (t.at, t.dir, kind(&t.text)))
        .collect()
}

fn t(at: u64, dir: Direction, k: &str) -> (u64, Direction, String) {
    (at, dir, k.to_owned())
}

/// What every scenario ends with: the loop runs on through HIRO's 60 minute cache
/// window and then all three machines hold nothing.
fn end_clean(lp: &mut ClosedLoop) {
    lp.settle();
    lp.assert_clean();
}

// ──────────────────────────────────────────────────────── the round trip ──

/// The whole exchange, frame by frame (`S:257-298`, `S:220-246`, `H:168-172`,
/// `I:214-237`). The mock acks the invoker and forwards the request 1 ms after
/// the submit; the handler acks and runs it; the result goes to the mock, which
/// acks the handler itself and forwards the result; the invoker acks it and the
/// call completes. Nothing is resent.
#[test]
fn a_plain_round_trip_runs_once_and_delivers_once() {
    let mut lp = ClosedLoop::connected();
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.run_for(100);
    let id = lp.first_request_id();

    // (a) what the caller received
    assert_eq!(lp.outcome(1).result, Ok(done()));
    assert_eq!(lp.outcome(1).request_id, id);
    assert_eq!(lp.completed_at(1), 1);
    // (b) how often the capability ran
    assert_eq!(lp.executor.runs_of(&id), 1);
    assert_eq!(lp.executor.capability.calls.get(), 1);
    // (c) the frames, in order
    assert_eq!(
        trace(&lp),
        vec![
            t(0, GatewayToInvoker, "hello"),
            t(0, GatewayToHandler, "hello"),
            t(0, InvokerToGateway, "submitAction"),
            t(1, GatewayToInvoker, "acknowledged"),
            t(1, GatewayToHandler, "submitAction"),
            t(1, HandlerToGateway, "acknowledged"),
            t(1, HandlerToGateway, "sendActionResult"),
            t(1, GatewayToHandler, "acknowledged"),
            t(1, GatewayToInvoker, "sendActionResult"),
            t(1, InvokerToGateway, "acknowledged"),
        ]
    );
    // The mock's ack to the invoker is `type` and `id` only (`S:268-273`); the
    // invoker's ack of the result is `{type, id, code: 200}` (`I:217`).
    let ack_to_invoker = &lp.frames(GatewayToInvoker, "acknowledged")[0];
    assert_eq!(ack_to_invoker, &json!({"type": "acknowledged", "id": id}));
    let result_ack = &lp.frames(InvokerToGateway, "acknowledged")[0];
    assert_eq!(
        result_ack,
        &json!({"type": "acknowledged", "id": id, "code": 200})
    );
    // The handler's ack of the submit (`H:170`) is recorded by the mock and
    // relayed to nobody (`S:171-172`).
    assert_eq!(
        lp.frames(HandlerToGateway, "acknowledged")[0],
        json!({"type": "acknowledged", "id": id, "code": 200, "message": ""})
    );
    // (e) the handler's episode
    let ep = lp.episode(&id);
    assert_eq!(ep.request_deliveries, 1);
    assert_eq!(ep.result_deliveries, 1);
    assert_eq!(ep.outcome, Some(Outcome::Succeeded));
    assert!(ep.acknowledged && !ep.nacked);
    // The mock saw the handler's ack and the invoker's result ack, relayed neither.
    assert_eq!(lp.gateway.acknowledged().len(), 2);
    assert_eq!(lp.gateway.registrations_of(&id), 1);
    // (d) the handler keeps the finished id for 60 minutes, and no longer
    lp.run_until(1 + RESULT_TTL_MS - 1);
    assert_eq!(lp.handler.state().tracked(), 1, "kept for the cache window");
    end_clean(&mut lp);
    assert_eq!(lp.handler.state().finished_episodes().count(), 1);
}

// ───────────────────────────────────────────────────── lost and resent ──

/// The gateway's ack of the submit is lost. The mock still forwarded the request,
/// so the handler ran it and the result reached the invoker, which kept it:
/// a response that arrives before the ack is held (`I:473`). The ack wait
/// expires at 3000 ms and the invoker sends again; the mock registers it a second
/// time and acks; the handler answers the repeat from its cache, and the caller
/// completes at the ack, once. The capability ran once.
///
/// The result frame comes to the invoker twice for the second delivery, because the
/// mock's registry is a `:duplicate` one and the invoker is registered twice
/// (`S:261-262`); the invoker acks every copy (`I:217`) and completes once.
#[test]
fn the_gateways_ack_lost_the_invoker_resends_and_the_capability_still_runs_once() {
    let mut lp = ClosedLoop::connected();
    lp.gateway
        .faults_mut()
        .drop_next_of(GatewayToInvoker, "acknowledged", 1);
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.run_for(10_000);
    let id = lp.first_request_id();

    assert_eq!(lp.outcome(1).result, Ok(done()));
    assert_eq!(lp.completed_at(1), 3_001, "completed by the second ack");
    assert_eq!(lp.executor.runs_of(&id), 1);
    assert_eq!(lp.executor.capability.calls.get(), 1);
    // The invoker sent at 0 and, its ack wait having run out, at 3000.
    assert_eq!(lp.times(InvokerToGateway, "submitAction"), vec![0, 3_000]);
    // The first ack was dropped, the second arrived.
    assert_eq!(lp.times(GatewayToInvoker, "acknowledged"), vec![3_001]);
    assert_eq!(lp.gateway.faults().log().len(), 1);
    // The handler saw the request twice, acked both, sent the result twice: the
    // first at once, the second from its cache.
    assert_eq!(lp.times(GatewayToHandler, "submitAction"), vec![1, 3_001]);
    assert_eq!(lp.times(HandlerToGateway, "acknowledged"), vec![1, 3_001]);
    assert_eq!(
        lp.times(HandlerToGateway, "sendActionResult"),
        vec![1, 3_001]
    );
    // One result copy for the first registration, two for the second delivery
    // (two registrations), each acked by the invoker, the caller completed once.
    assert_eq!(lp.gateway.registrations_of(&id), 2);
    assert_eq!(lp.count(GatewayToInvoker, "sendActionResult"), 3);
    assert_eq!(lp.count(InvokerToGateway, "acknowledged"), 3);
    let ep = lp.episode(&id);
    assert_eq!((ep.request_deliveries, ep.result_deliveries), (2, 2));
    assert_eq!(ep.outcome, Some(Outcome::Succeeded));
    assert!(ep.acknowledged);
    end_clean(&mut lp);
}

/// The handler's own ack of the submit is swallowed by the mock (`S:171-172`): the
/// invoker's ack is the mock's, not the handler's. Losing the handler's ack
/// changes nothing at all. This pins the mirrored behaviour; it is the reason the
/// scenario above drops the gateway's ack, not the handler's.
#[test]
fn the_handlers_ack_of_the_submit_never_reaches_the_invoker_so_losing_it_changes_nothing() {
    let mut lp = ClosedLoop::connected();
    lp.gateway
        .faults_mut()
        .drop_next_of(HandlerToGateway, "acknowledged", 1);
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.run_for(10_000);
    let id = lp.first_request_id();
    assert_eq!(lp.outcome(1).result, Ok(done()));
    assert_eq!(lp.completed_at(1), 1);
    assert_eq!(lp.count(InvokerToGateway, "submitAction"), 1, "no resend");
    assert_eq!(lp.executor.runs_of(&id), 1);
    // The mock recorded the invoker's result ack only: the handler's ack was lost.
    assert_eq!(lp.gateway.acknowledged().len(), 1);
    end_clean(&mut lp);
}

/// The handler's result is lost once on its way to the gateway. Nothing acks it,
/// so the handler sends it again after 3000 ms (`R:143-163`, `H:365-366`), and
/// that one is delivered once to the caller.
#[test]
fn the_handlers_result_lost_once_is_resent_after_3000ms_and_delivered_once() {
    let mut lp = ClosedLoop::connected();
    lp.executor.take(100);
    lp.gateway
        .faults_mut()
        .drop_next_of(HandlerToGateway, "sendActionResult", 1);
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.run_for(10_000);
    let id = lp.first_request_id();

    assert_eq!(lp.outcome(1).result, Ok(done()));
    assert_eq!(lp.executor.runs_of(&id), 1);
    // Sent at 101 (lost), resent at 101 + 3000, which the mock forwarded.
    assert_eq!(
        lp.times(HandlerToGateway, "sendActionResult"),
        vec![101, 3_101]
    );
    assert_eq!(lp.times(GatewayToInvoker, "sendActionResult"), vec![3_101]);
    assert_eq!(lp.completed_at(1), 3_101);
    assert_eq!(lp.count(InvokerToGateway, "acknowledged"), 1);
    let ep = lp.episode(&id);
    assert_eq!(ep.result_deliveries, 2);
    assert_eq!(
        ep.attempts
            .iter()
            .filter(|a| a.kind == AttemptKind::ResultDelivery)
            .map(|a| a.at)
            .collect::<Vec<_>>(),
        vec![101, 3_101]
    );
    assert!(ep.acknowledged);
    end_clean(&mut lp);
}

/// The invoker's ack of the result is lost. The mock records an invoker's ack and
/// relays it to nobody (`S:171-172`); it acked the handler's result itself
/// (`S:224-236`). So the handler is not made to resend, and the caller, already
/// completed, is not completed again.
#[test]
fn the_invokers_ack_of_the_result_lost_makes_the_handler_do_nothing() {
    let mut lp = ClosedLoop::connected();
    lp.gateway
        .faults_mut()
        .drop_next_of(InvokerToGateway, "acknowledged", 1);
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.run_for(20_000);
    let id = lp.first_request_id();

    assert_eq!(lp.outcome(1).result, Ok(done()));
    assert_eq!(lp.executor.runs_of(&id), 1);
    assert_eq!(
        lp.count(HandlerToGateway, "sendActionResult"),
        1,
        "no resend"
    );
    // The ack was sent and lost; the mock holds only the handler's ack.
    assert_eq!(lp.count(InvokerToGateway, "acknowledged"), 1);
    assert_eq!(lp.gateway.acknowledged().len(), 1);
    assert!(lp.episode(&id).acknowledged, "acked by the mock's own ack");
    end_clean(&mut lp);
}

/// The mock's ack of the result, which is what stops the handler's resend, is
/// lost. The handler sends the result again after 3000 ms; the mock acks it this
/// time and forwards it; the invoker acks it again and the caller, completed by
/// the first copy, is not completed twice.
#[test]
fn the_result_ack_lost_on_the_way_to_the_handler_resends_the_result_and_the_caller_completes_once()
{
    let mut lp = ClosedLoop::connected();
    lp.gateway
        .faults_mut()
        .drop_next_of(GatewayToHandler, "acknowledged", 1);
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.run_for(20_000);
    let id = lp.first_request_id();

    assert_eq!(lp.outcome(1).result, Ok(done()), "completed exactly once");
    assert_eq!(lp.completed_at(1), 1);
    assert_eq!(lp.executor.runs_of(&id), 1);
    assert_eq!(
        lp.times(HandlerToGateway, "sendActionResult"),
        vec![1, 3_001]
    );
    // The invoker was sent the result twice and acked it twice.
    assert_eq!(
        lp.times(GatewayToInvoker, "sendActionResult"),
        vec![1, 3_001]
    );
    assert_eq!(lp.times(InvokerToGateway, "acknowledged"), vec![1, 3_001]);
    let ep = lp.episode(&id);
    assert_eq!(ep.result_deliveries, 2);
    assert!(ep.acknowledged);
    end_clean(&mut lp);
}

// ───────────────────────────────────────────────────────── duplicates ──

/// A submit delivered twice while the first is executing: both are acked, one
/// run, the result sent once (`H:201-203`, `:224-229`, oracle 2 and 16). Later a
/// third delivery after completion is answered from the cache with no second run
/// (`H:186-188`, oracle 18).
#[test]
fn a_duplicated_submit_while_executing_and_after_completion_runs_once() {
    let mut lp = ClosedLoop::connected();
    lp.executor.take(500);
    lp.gateway
        .faults_mut()
        .duplicate_next_of(InvokerToGateway, "submitAction", 1);
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.run_for(2_000);
    let id = lp.first_request_id();

    assert_eq!(lp.outcome(1).result, Ok(done()));
    assert_eq!(lp.completed_at(1), 501);
    assert_eq!(lp.executor.runs_of(&id), 1);
    assert_eq!(lp.times(GatewayToHandler, "submitAction"), vec![1, 1]);
    assert_eq!(lp.times(HandlerToGateway, "acknowledged"), vec![1, 1]);
    assert_eq!(lp.times(HandlerToGateway, "sendActionResult"), vec![501]);
    // Two registrations: the invoker is sent the one result twice and acks both.
    assert_eq!(lp.gateway.registrations_of(&id), 2);
    assert_eq!(lp.count(InvokerToGateway, "acknowledged"), 2);
    let ep = lp.episode(&id);
    assert_eq!((ep.request_deliveries, ep.result_deliveries), (2, 1));

    // After completion: the very request the handler saw, delivered again.
    let again = lp.frames(GatewayToHandler, "submitAction")[0].to_string();
    lp.run_until(10_000);
    lp.inject(Side::Handler, &again);
    lp.run_for(1_000);
    assert_eq!(lp.executor.runs_of(&id), 1, "the cache answers it");
    assert_eq!(
        lp.times(HandlerToGateway, "acknowledged"),
        vec![1, 1, 10_000]
    );
    assert_eq!(
        lp.times(HandlerToGateway, "sendActionResult"),
        vec![501, 10_000]
    );
    // The mock forwards that result to the invoker, which acks it and has no call.
    assert_eq!(lp.completions.len(), 1, "the caller is not completed twice");
    let ep = lp.episode(&id);
    assert_eq!((ep.request_deliveries, ep.result_deliveries), (3, 2));
    assert!(ep.acknowledged);
    end_clean(&mut lp);
}

// ───────────────────────────────────────────────────────────── refusals ──

/// An unknown capability: the mock NACKs 404 at once (`S:277-288`, oracle 44), the
/// call ends there (`I:659-660`, oracle 33), and the handler never sees the
/// request: no frame, no run, no episode.
#[test]
fn an_unknown_capability_is_nacked_404_and_never_reaches_the_handler() {
    let mut lp = ClosedLoop::connected();
    lp.call(CallRequest::new(
        1,
        "ticket-1",
        HANDLER_ID,
        "invalid_capability",
    ));
    lp.run_for(60_000);

    assert_eq!(
        lp.outcome(1).result,
        Err(InvokeError::Nack {
            code: Some(404),
            message: Some("capability invalid_capability not found".to_owned()),
        })
    );
    assert_eq!(lp.completed_at(1), 0, "at once, no ack wait");
    assert_eq!(lp.count(InvokerToGateway, "submitAction"), 1, "no resend");
    assert_eq!(
        lp.frames(GatewayToInvoker, "negativeAcknowledged")[0],
        json!({
            "type": "negativeAcknowledged",
            "id": lp.first_request_id(),
            "code": 404,
            "message": "capability invalid_capability not found",
        })
    );
    // The handler was sent its hello and nothing else.
    assert_eq!(lp.kinds(GatewayToHandler), vec![(0, "hello".to_owned())]);
    assert!(lp.kinds(HandlerToGateway).is_empty());
    assert_eq!(lp.executor.total_runs(), 0);
    assert_eq!(lp.handler.state().tracked(), 0);
    assert_eq!(lp.handler.state().finished_episodes().count(), 0);
    assert!(
        lp.invoker_logs.is_empty(),
        "a 404 is not logged (`I:659-660`)"
    );
    assert_eq!(lp.gateway.registrations_of(&lp.first_request_id()), 0);
    end_clean(&mut lp);
}

/// The mock-only forbidden capability (`S:177-193`): a NACK 403, terminal, logged
/// by the invoker as a NACK with a code other than 404 (`I:662-664`).
#[test]
fn the_mocks_forbidden_capability_is_nacked_403_and_logged() {
    let mut lp = ClosedLoop::connected();
    lp.call(CallRequest::new(1, "t", HANDLER_ID, FORBIDDEN_CAPABILITY));
    lp.run_for(1_000);
    assert_eq!(
        lp.outcome(1).result,
        Err(InvokeError::Nack {
            code: Some(403),
            message: Some("Forbidden".to_owned()),
        })
    );
    assert!(matches!(
        lp.invoker_logs.as_slice(),
        [(
            0,
            Log::MessageNacked {
                code: Some(403),
                ..
            }
        )]
    ));
    assert_eq!(lp.executor.total_runs(), 0);
    end_clean(&mut lp);
}

// ───────────────────────────────────────────────────────────── timeouts ──

/// An execution longer than the request timeout. The request carries
/// `timeout` 2 s, the wire says 2000 ms, and the handler's deadline fires at
/// 1 + 2000: the result is the status 13 error (`H:311-313`, oracle 5), sent while
/// the invoker still waits (its wait is 2000 + 1000 from the ack). The invoker
/// completes with the handler's error object; the capability's late end is ignored.
#[test]
fn an_execution_longer_than_the_request_timeout_is_the_status_13_result() {
    let mut lp = ClosedLoop::connected();
    lp.executor.take(10_000);
    lp.call(ClosedLoop::request(1, "sleep", 2));
    lp.run_for(20_000);
    let id = lp.first_request_id();

    assert_eq!(
        lp.frames(InvokerToGateway, "submitAction")[0]["timeout"],
        json!(2_000)
    );
    assert_eq!(
        handler_error(&lp, 1),
        json!({
            "req_id": id,
            "action_status": 13,
            "action_error": "Execution timed out after 2000ms",
        })
    );
    assert_eq!(lp.completed_at(1), 2_001);
    assert_eq!(lp.executor.runs_of(&id), 1);
    assert_eq!(lp.count(HandlerToGateway, "sendActionResult"), 1);
    let ep = lp.episode(&id);
    assert_eq!(ep.outcome, Some(Outcome::TimedOut));
    assert_eq!(ep.finished_at, Some(2_001));
    assert!(ep.acknowledged);
    end_clean(&mut lp);
}

/// Nothing answers the submit: the invoker sends three times, 3000 ms apart,
/// and gives up with `{:ack_timeout, 9000}` (`I:109-110`, `:582-590`, oracle 20
/// and 21). The handler is never reached.
#[test]
fn when_nothing_answers_the_invoker_sends_three_times_and_times_out_on_the_ack() {
    let mut lp = ClosedLoop::connected();
    lp.gateway.ignore_next_request(10);
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.run_for(30_000);

    assert_eq!(
        lp.times(InvokerToGateway, "submitAction"),
        vec![0, 3_000, 6_000]
    );
    assert_eq!(lp.outcome(1).result, Err(InvokeError::AckTimeout(9_000)));
    assert_eq!(lp.completed_at(1), 9_000);
    assert_eq!(
        lp.invoker_logs
            .iter()
            .filter(|(_, l)| matches!(l, Log::AckTimeout { .. }))
            .count(),
        3
    );
    assert!(lp.kinds(GatewayToInvoker).iter().all(|(_, k)| k == "hello"));
    assert_eq!(lp.kinds(GatewayToHandler), vec![(0, "hello".to_owned())]);
    assert_eq!(lp.executor.total_runs(), 0);
    end_clean(&mut lp);
}

/// The request is acked and then no result ever comes (the mock acks and does not
/// forward, `S:291-293`). The response wait is the request timeout + 1000 ms
/// (`I:737-738`, oracle 22), after which the invoker sends once more, a last call
/// with a full ack cycle (`I:704-714`, oracle 23), and a second wait of the same
/// length ends in `{:exec_timeout, 1000}`.
#[test]
fn when_acked_and_never_answered_the_response_wait_is_timeout_plus_1000_then_one_last_call() {
    let mut lp = ClosedLoop::connected();
    lp.gateway.drop_next_submit(2);
    lp.call(ClosedLoop::request(1, "ls", 1));
    lp.run_for(20_000);

    // Submit at 0; acked at 1, so the wait is 1 + 1000 + 1000. The last call goes
    // out at 2001, is acked at 2002, and its wait ends at 2002 + 2000.
    assert_eq!(lp.times(InvokerToGateway, "submitAction"), vec![0, 2_001]);
    assert_eq!(lp.times(GatewayToInvoker, "acknowledged"), vec![1, 2_002]);
    assert_eq!(lp.outcome(1).result, Err(InvokeError::ExecTimeout(1_000)));
    assert_eq!(lp.completed_at(1), 4_002);
    assert!(
        lp.invoker_logs
            .iter()
            .any(|(at, l)| *at == 2_001 && matches!(l, Log::SendingLastCall { .. }))
    );
    assert!(
        lp.invoker_logs
            .iter()
            .any(|(at, l)| *at == 4_002 && matches!(l, Log::ResponseTimeout { .. }))
    );
    assert_eq!(lp.executor.total_runs(), 0, "never forwarded");
    end_clean(&mut lp);
}

// ──────────────────────────────────────────────────── a failing capability ──

/// A capability that returns `Err` is the status 54 result with the
/// `RuntimeError` banner (`H:343-358`, oracle 6); one that panics is status 54
/// with the banner only (oracle 8). The caller gets the handler's error object.
/// A repeat of the same request, after the first completed, is answered from the
/// cache with the same crash result and no second run (oracle 43).
#[test]
fn a_failing_and_a_panicking_capability_are_status_54_and_a_repeat_gets_the_cached_result() {
    for (behavior, banner) in [
        (
            Behavior::Fail("boom".to_owned()),
            "Execution failed: ** (RuntimeError) boom",
        ),
        (Behavior::Panic, "Execution failed: ** (exit)"),
    ] {
        let mut lp = ClosedLoop::connected();
        lp.executor.capability.behavior = behavior.clone();
        lp.call(ClosedLoop::request(1, "ls", 60));
        lp.run_for(1_000);
        let id = lp.first_request_id();
        let expected = json!({"req_id": id, "action_status": 54, "action_error": banner});
        assert_eq!(handler_error(&lp, 1), expected, "{behavior:?}");
        assert_eq!(lp.executor.runs_of(&id), 1);
        let ep = lp.episode(&id);
        assert_eq!(ep.outcome, Some(Outcome::Failed(Some(54))));

        // The repeat: the same call again, a new caller handle, the same id.
        lp.call(ClosedLoop::request(2, "ls", 60));
        lp.run_for(1_000);
        assert_eq!(handler_error(&lp, 2), expected, "the cached crash");
        assert_eq!(lp.outcome(2).request_id, id);
        assert_eq!(
            lp.executor.runs_of(&id),
            1,
            "no second run for {behavior:?}"
        );
        assert_eq!(lp.executor.capability.calls.get(), 1);
        let ep = lp.episode(&id);
        assert_eq!((ep.request_deliveries, ep.result_deliveries), (2, 2));
        end_clean(&mut lp);
    }
}

// ─────────────────────────────────────────────────────────── frame limit ──

/// The gateway announces `max_frame_bytes` in its `hello`; a result whose frame
/// would exceed it is replaced by the status 59 error before it is sent
/// (`H:255-264`, `result_frame.ex:27-38`, oracle 10). The mock closes a socket
/// whose frame is over the limit it advertised (`S:112-120`), so its
/// `gateway_errors` stay empty only because the handler fitted the frame.
#[test]
fn a_result_over_the_announced_frame_limit_reaches_the_caller_as_status_59() {
    let mut lp = ClosedLoop::new(MockConfig {
        handler_max_frame_bytes: Some(400),
        ..MockConfig::default()
    });
    lp.connect();
    lp.executor.capability.behavior = Behavior::Succeed("x".repeat(2_000));
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.run_for(1_000);
    let id = lp.first_request_id();

    let err = handler_error(&lp, 1);
    assert_eq!(err["req_id"], json!(id));
    assert_eq!(err["action_status"], json!(59));
    assert!(
        err["action_error"]
            .as_str()
            .expect("text")
            .starts_with("Response was too big ("),
        "{err}"
    );
    assert_eq!(
        parse(&lp.frames(GatewayToHandler, "hello")[0].to_string())["max_frame_bytes"],
        json!(400)
    );
    let sent = lp
        .trace
        .iter()
        .find(|t| t.dir == HandlerToGateway && kind(&t.text) == "sendActionResult");
    assert!(sent.expect("sent").text.len() <= 400);
    assert_eq!(lp.executor.runs_of(&id), 1);
    assert_eq!(lp.episode(&id).outcome, Some(Outcome::Failed(Some(59))));
    assert!(lp.gateway_errors.is_empty(), "the mock refused no frame");
    end_clean(&mut lp);
}

/// The other side of the same rule: a frame over the limit the gateway
/// advertised is refused by the mock (the real socket would close).
#[test]
fn the_mock_refuses_a_frame_over_the_limit_it_advertised() {
    let mut lp = ClosedLoop::new(MockConfig {
        handler_max_frame_bytes: Some(100),
        ..MockConfig::default()
    });
    let big = json!({"type": "sendActionResult", "id": "x", "result": "y".repeat(500)}).to_string();
    let err = lp.gateway.on_handler_text(0, &big).unwrap_err();
    assert!(matches!(
        err,
        MockError::FrameTooBig {
            side: Side::Handler,
            limit: 100,
            ..
        }
    ));
}

// ───────────────────────────────────────────────────── in-band 429 and 503 ──

/// An armed in-band 429 (`S:202-213`, `M:303-318`) in place of the ack: the
/// call completes at once with `{:rate_limited, 1500}` (`I:246-258`, oracle 24)
/// and is not resent. The mock forwards nothing, so the handler never saw the
/// request: no run, no episode, nothing tracked.
#[test]
fn an_inband_429_with_an_id_completes_the_call_at_once_and_the_handler_never_sees_it() {
    let mut lp = ClosedLoop::connected();
    lp.gateway.deny_next_request(1, 1_500);
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.run_for(60_000);
    let id = lp.first_request_id();

    assert_eq!(lp.outcome(1).result, Err(InvokeError::RateLimited(1_500)));
    assert_eq!(lp.completed_at(1), 0);
    assert_eq!(lp.count(InvokerToGateway, "submitAction"), 1, "not resent");
    let refusal = &lp.frames(GatewayToInvoker, "error429")[0];
    assert_eq!(
        refusal,
        &json!({"error": {"code": 429, "message": "Rate limit exceeded", "retryAfterMs": 1500}, "id": id})
    );
    assert_eq!(lp.kinds(GatewayToHandler), vec![(0, "hello".to_owned())]);
    assert_eq!(lp.executor.total_runs(), 0);
    assert_eq!(lp.handler.state().tracked(), 0);
    end_clean(&mut lp);
}

/// The same for the in-band 503 (`S:316-323`): the caller gets the frame's own
/// error object at once (`I:260-268`, oracle 25), no resend, the handler never
/// involved.
#[test]
fn an_inband_503_with_an_id_completes_the_call_at_once_with_the_error_object() {
    let mut lp = ClosedLoop::connected();
    lp.gateway.fail_next_request_unavailable(1);
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.run_for(60_000);

    let Err(InvokeError::Refused(map)) = &lp.outcome(1).result else {
        panic!("{:?}", lp.outcome(1));
    };
    assert_eq!(
        Value::Object(map.clone()),
        json!({"code": 503, "message": "Upstream unavailable", "upstream": "action"})
    );
    assert_eq!(lp.completed_at(1), 0);
    assert_eq!(lp.count(InvokerToGateway, "submitAction"), 1);
    assert_eq!(lp.kinds(GatewayToHandler), vec![(0, "hello".to_owned())]);
    assert_eq!(lp.executor.total_runs(), 0);
    end_clean(&mut lp);
}

/// A 429 or 503 injected while the request is already on its way (a refusal the
/// mock did not itself arm): the caller completes at once with it; but the mock
/// had forwarded the request, so the handler side runs it, once, and its
/// result is acked by the mock and by the invoker and delivered to nobody. That is
/// the fate of the handler side: it cannot know the caller gave up.
#[test]
fn an_injected_refusal_ends_the_call_but_the_forwarded_request_still_runs_once() {
    for refusal in ["429", "503"] {
        let mut lp = ClosedLoop::connected();
        lp.call(ClosedLoop::request(1, "ls", 60));
        let id = lp.first_request_id();
        let outs = match refusal {
            "429" => lp
                .gateway
                .inject_rate_limited(0, &IdField::Present(id.clone()), 2_500),
            _ => lp
                .gateway
                .inject_unavailable(0, &IdField::Present(id.clone())),
        };
        lp.deliver(outs);
        lp.run_for(10_000);

        match refusal {
            "429" => assert_eq!(lp.outcome(1).result, Err(InvokeError::RateLimited(2_500))),
            _ => assert!(matches!(lp.outcome(1).result, Err(InvokeError::Refused(_)))),
        }
        assert_eq!(lp.completed_at(1), 0);
        // The handler side: ran once, result acked and not resent.
        assert_eq!(lp.executor.runs_of(&id), 1, "{refusal}");
        assert_eq!(lp.count(HandlerToGateway, "sendActionResult"), 1);
        let ep = lp.episode(&id);
        assert_eq!(ep.outcome, Some(Outcome::Succeeded));
        assert!(ep.acknowledged);
        // The invoker acked the orphaned result and kept no call.
        assert_eq!(lp.count(InvokerToGateway, "acknowledged"), 1);
        end_clean(&mut lp);
    }
}

/// A 429 or a 503 with no usable id belongs to no caller (`I:272-278`, oracle
/// 26): nobody is answered, the invoker logs, and the real request goes on to
/// complete normally.
#[test]
fn an_inband_429_or_503_without_an_id_answers_nobody() {
    let mut lp = ClosedLoop::connected();
    lp.call(ClosedLoop::request(1, "ls", 60));
    let id = lp.first_request_id();
    for outs in [
        lp.gateway.inject_rate_limited(0, &IdField::Absent, 1_986),
        lp.gateway.inject_rate_limited(0, &IdField::Null, 1_986),
        lp.gateway.inject_unavailable(0, &IdField::Absent),
        lp.gateway.inject_unavailable(0, &IdField::Null),
    ] {
        lp.deliver(outs);
    }
    assert!(lp.completions.is_empty(), "nobody was answered");
    let logs: Vec<&Log> = lp.invoker_logs.iter().map(|(_, l)| l).collect();
    assert_eq!(
        logs,
        vec![
            &Log::RateLimitedNoId {
                retry_after_ms: 1_986
            },
            &Log::RateLimitedNoId {
                retry_after_ms: 1_986
            },
            &Log::UpstreamUnavailableNoId,
            &Log::UpstreamUnavailableNoId,
        ]
    );
    lp.run_for(10_000);
    assert_eq!(lp.outcome(1).result, Ok(done()));
    assert_eq!(lp.executor.runs_of(&id), 1);
    end_clean(&mut lp);
}

// ───────────────────────────────────────────────────────── handler restart ──

/// A handler that restarts has an empty cache. HIRO's dedupe cache lives in the
/// handler process (`H:72-77`; the oracle's section 3 marks survival across a
/// restart UNVERIFIED, and no code persists it), so the same request delivered
/// to the fresh handler runs again. This is parity, not a defect: the count is 2.
#[test]
fn a_handler_restart_loses_the_cache_so_the_same_request_runs_again() {
    let mut lp = ClosedLoop::connected();
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.run_for(1_000);
    let id = lp.first_request_id();
    assert_eq!(lp.executor.runs_of(&id), 1);
    assert_eq!(lp.handler.state().tracked(), 1);

    lp.restart_handler();
    assert_eq!(lp.handler.state().tracked(), 0, "the cache is gone");
    lp.call(ClosedLoop::request(2, "ls", 60));
    lp.run_for(1_000);

    assert_eq!(lp.outcome(2).result, Ok(done()));
    assert_eq!(lp.outcome(2).request_id, id);
    assert_eq!(
        lp.executor.runs_of(&id),
        2,
        "parity: the cache does not survive a restart"
    );
    // The fresh handler's episode starts over.
    assert_eq!(lp.episode(&id).request_deliveries, 1);
    // The handler was sent a new hello when it reconnected.
    assert_eq!(lp.count(GatewayToHandler, "hello"), 2);
    end_clean(&mut lp);
}

/// The restart in the middle of an execution, between a first delivery and its
/// redelivery. The gateway's ack is lost, the handler is restarted 1500 ms in
/// (the run is lost with the old process), the invoker's resend at 3000 ms
/// reaches the fresh handler, which runs it again: count 2, one result, the caller
/// completes once.
#[test]
fn a_handler_restart_between_the_first_delivery_and_a_redelivery_runs_it_again() {
    let mut lp = ClosedLoop::connected();
    lp.executor.take(2_000);
    lp.gateway
        .faults_mut()
        .drop_next_of(GatewayToInvoker, "acknowledged", 1);
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.run_until(1_500);
    let id = lp.first_request_id();
    assert_eq!(lp.executor.runs_of(&id), 1, "the first delivery ran");
    lp.restart_handler();
    lp.run_until(20_000);

    assert_eq!(lp.outcome(1).result, Ok(done()));
    assert_eq!(
        lp.executor.runs_of(&id),
        2,
        "parity: the fresh handler has no memory of the first"
    );
    // The redelivery at 3000 + 1 starts a run that ends 2000 later.
    assert_eq!(lp.completed_at(1), 5_001);
    assert_eq!(lp.times(HandlerToGateway, "sendActionResult"), vec![5_001]);
    let ep = lp.episode(&id);
    assert_eq!((ep.request_deliveries, ep.result_deliveries), (1, 1));
    end_clean(&mut lp);
}

// ─────────────────────────────────────────────────── concurrent callers ──

/// Two callers with identical requests share an id (`request.ex:48-49`,
/// oracle 37 and 38). Each sends its own submit; the handler runs the capability
/// once and the result completes both callers.
#[test]
fn two_concurrent_identical_calls_complete_from_one_execution() {
    let mut lp = ClosedLoop::connected();
    lp.executor.take(300);
    lp.call(ClosedLoop::request(1, "ls", 60));
    lp.call(ClosedLoop::request(2, "ls", 60));
    lp.run_for(5_000);
    let id = lp.first_request_id();

    assert_eq!(lp.outcome(1).result, Ok(done()));
    assert_eq!(lp.outcome(2).result, Ok(done()));
    assert_eq!(lp.outcome(1).request_id, lp.outcome(2).request_id);
    assert_eq!(lp.completed_at(1), 301);
    assert_eq!(lp.completed_at(2), 301);
    assert_eq!(lp.executor.runs_of(&id), 1);
    assert_eq!(lp.count(InvokerToGateway, "submitAction"), 2);
    assert_eq!(lp.times(HandlerToGateway, "sendActionResult"), vec![301]);
    let ep = lp.episode(&id);
    assert_eq!((ep.request_deliveries, ep.result_deliveries), (2, 1));
    end_clean(&mut lp);
}

// ───────────────────────────────────────────────────── the registration path ──

/// The invoker's fetch machine reads the mock's two REST bodies, becomes ready,
/// and a capability default reaches the wire. The default `timeout` of the mock's
/// own example, `{"ExecuteCommand": {"timeout": 60000, "command": null}}`
/// (`mock_test.exs:73-93`), is read as SECONDS (`invoker.ex:527-531`, `:565-569`,
/// oracle 30 and 32), so the wire carries 60_000_000 ms, in the frame and in
/// `parameters.timeout`; the handler's deadline is that far off.
#[test]
fn the_fetch_reads_the_mocks_registration_and_a_capability_default_reaches_the_wire() {
    let mut lp = ClosedLoop::connected();
    lp.gateway
        .put_capabilities(r#"{"ExecuteCommand": {"timeout": 60000, "command": null}}"#)
        .expect("capabilities");
    lp.gateway
        .put_applicabilities(
            "action_handler",
            r#"[
              {"name": "LocalHandler", "capability": "ExecuteLocalCommand", "implementation": "local",
               "applicability": {"on ogit/_id": {"LocalNodeID": "${ogit/_id}"}}, "exec": "${command}"},
              {"name": "LocalHandler", "capability": "RunLocalScript", "implementation": "local",
               "applicability": ["on ogit/_id"], "tempfiles": {"tempfile": "${command}"},
               "exec": "sh -- ${tempfile}"}
            ]"#,
        )
        .expect("applicabilities");

    // The fetch machine asks for both bodies, the gateway answers.
    let mut fetch = FetchState::new("/api/0.9/action/", Draw::Seeded(1));
    let mut out = fetch.step(0, FetchInput::Start);
    let mut ready = None;
    while let Some(o) = out.pop() {
        match o {
            FetchOutput::Fetch { kind: k, path } => {
                let body = match k {
                    FetchKind::Capabilities => {
                        assert_eq!(path, "/api/0.9/action/capabilities");
                        lp.gateway.capabilities_body()
                    }
                    FetchKind::Applicabilities => {
                        assert_eq!(path, "/api/0.9/action/applicabilities");
                        lp.gateway.applicabilities_body()
                    }
                };
                let outcome = FetchOutcome::Response {
                    status: 200,
                    headers: vec![],
                    body,
                };
                out = fetch.step(0, FetchInput::Fetched { kind: k, outcome });
            }
            FetchOutput::Ready(reg) => ready = Some(reg),
            other => panic!("{other:?}"),
        }
    }
    assert!(fetch.is_ready());
    let reg = ready.expect("ready");

    // What the registration holds.
    assert_eq!(reg.handler_ids(), vec!["action_handler".to_owned()]);
    let caps = reg.concrete_capabilities().expect("capabilities parse");
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].name, "ExecuteCommand");
    assert!(
        caps[0]
            .params
            .iter()
            .any(|p| p.name == "command" && p.mandatory)
    );
    assert!(
        caps[0]
            .params
            .iter()
            .any(|p| p.name == "timeout" && p.default.as_deref() == Some("60000"))
    );
    match reg.typed_applicabilities().expect("applicabilities parse") {
        ogar_action_handler::Applicabilities::AppliesOn(rows) => {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].capability, "ExecuteLocalCommand");
            assert_eq!(rows[0].parameters[0].1, "${ogit/_id}", "stored verbatim");
        }
        other => panic!("{other:?}"),
    }

    // The default reaches the wire, as seconds.
    lp.invoker.state_mut().load_capabilities(&reg.capabilities);
    let req = CallRequest::new(1, "ticket-1", HANDLER_ID, CAPABILITY).param("command", json!("ls"));
    lp.call(req);
    lp.run_for(100);
    let submit = &lp.frames(InvokerToGateway, "submitAction")[0];
    assert_eq!(submit["timeout"], json!(60_000_000));
    assert_eq!(submit["parameters"]["timeout"], json!(60_000_000));
    assert_eq!(submit["parameters"]["command"], json!("ls"));
    assert_eq!(lp.outcome(1).result, Ok(done()));
    // The mock was shown the typed parameters, as the invoker wrote them.
    assert_eq!(lp.gateway.submitted()[0].1["timeout"], json!(60_000_000));
    // The handler took that as its deadline: its window is the 60 minutes after the
    // run, the deadline having been cleared by the result.
    assert_eq!(
        lp.episode(&lp.first_request_id()).outcome,
        Some(Outcome::Succeeded)
    );
    end_clean(&mut lp);
}

/// `put_applicabilities` for a handler the mock holds nothing for raises in the
/// source (`mock.ex:133`, a `Map.merge(nil, ...)`); here it is an error.
#[test]
fn the_mock_refuses_applicabilities_for_an_unknown_handler() {
    let mut lp = ClosedLoop::connected();
    let err = lp
        .gateway
        .put_applicabilities("other", "[]")
        .expect_err("refused");
    assert_eq!(err, MockError::UnknownHandler("other".to_owned()));
}

// ─────────────────────────────────────────────────── a seeded random run ──

/// A tiny deterministic generator (xorshift64*), so a failing seed replays.
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

fn fnv(s: &str) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Many requests with random drops, duplicates and delays on every direction,
/// and, for some seeds, one handler restart. The faults are bounded so that
/// everything gets through: at most one outstanding fault at a time and 6500 ms
/// between one and the next, which is more than the invoker's ack wait times two,
/// so no request meets two. Every call completes exactly once, with the result;
/// the capability ran at most once per request id, and at most twice when a
/// restart was injected; and all three machines end clean.
#[test]
fn a_seeded_random_run_completes_every_call_once_and_runs_each_request_at_most_once() {
    const SPACING_MS: u64 = 6_500;
    let mut resends = 0_u64;
    for (seed, with_restart) in [
        (1_u64, false),
        (2, false),
        (3, false),
        (7, false),
        (42, true),
        (2026, true),
        (9001, true),
        (123_456_789, true),
    ] {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut lp = ClosedLoop::connected();
        lp.executor.duration = Box::new(|id, _| fnv(id) % 1_500);

        // 24 calls at multiples of 500 ms in the first 30 s; every sixth repeats
        // an earlier command, so some ids are shared.
        const CALLS: u64 = 24;
        let mut starts: Vec<(u64, u64, String)> = Vec::new();
        for i in 0..CALLS {
            let at = rng.below(60) * 500;
            let cmd = if i % 6 == 5 {
                format!("cmd-{}", rng.below(i))
            } else {
                format!("cmd-{i}")
            };
            starts.push((at, i + 1, cmd));
        }
        starts.sort();

        let mut restarted = false;
        let mut last_fault_at: Option<u64> = None;
        let mut budget = 6_u32;
        let mut step = 0_u64;
        while step <= 40_000 {
            lp.run_until(step);
            for (_, call, cmd) in starts.iter().filter(|(at, _, _)| *at == step) {
                lp.call(ClosedLoop::request(*call, cmd, 4));
            }
            let quiet = lp.gateway.faults().pending() == 0
                && last_fault_at.is_none_or(|a| step >= a + SPACING_MS);
            if quiet {
                if with_restart && !restarted && step >= 10_000 && rng.below(4) == 0 {
                    lp.restart_handler();
                    restarted = true;
                    last_fault_at = Some(step);
                } else if budget > 0 && rng.below(3) == 0 {
                    let dir = [
                        InvokerToGateway,
                        GatewayToInvoker,
                        HandlerToGateway,
                        GatewayToHandler,
                    ][rng.below(4) as usize];
                    match rng.below(4) {
                        0 | 1 => lp.gateway.faults_mut().drop_next(dir, 1),
                        2 => lp.gateway.faults_mut().duplicate_next(dir, 1),
                        _ => {
                            let ms = 1 + rng.below(2_500);
                            lp.gateway.faults_mut().delay_next(dir, ms, 1);
                        }
                    }
                    budget -= 1;
                    last_fault_at = Some(step);
                }
            }
            if let Some(ev) = lp.gateway.faults().log().last() {
                last_fault_at = Some(last_fault_at.map_or(ev.at, |a| a.max(ev.at)));
            }
            step += 500;
        }
        lp.run_until(90_000);

        for call in 1..=CALLS {
            assert_eq!(
                lp.outcome(call).result,
                Ok(done()),
                "seed {seed}: call {call}"
            );
        }
        assert_eq!(lp.completions.len() as u64, CALLS, "seed {seed}");
        let ceiling = if with_restart { 1 + lp.restarts } else { 1 };
        for (id, runs) in &lp.executor.runs {
            assert!(
                *runs <= ceiling,
                "seed {seed}: request {id} ran {runs} times (restarts: {})",
                lp.restarts
            );
        }
        assert_eq!(
            lp.executor.capability.calls.get(),
            lp.executor.total_runs(),
            "seed {seed}"
        );
        // The run was not a quiet one.
        assert!(lp.gateway.faults().log().len() >= 2, "seed {seed}");
        resends += lp.count(InvokerToGateway, "submitAction") as u64 - CALLS;
        end_clean(&mut lp);
    }
    assert!(resends > 0, "no seed made the invoker send again");
}
