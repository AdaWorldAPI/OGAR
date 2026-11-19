//! The closed loop: one invoker session, one mock gateway and one handler
//! session in a single process, with a virtual clock and an executor that is a
//! test double.
//!
//! Everything between the machines is text. The invoker session writes frames
//! with `invoke_session` and reads them with `wire::decode`; the handler session
//! reads and writes with `wire`; the mock gateway reads JSON and writes JSON. No
//! typed value is passed from one machine to another.
//!
//! Time is the harness's `now`, in milliseconds. [`ClosedLoop::run_until`] moves it
//! to the earliest wakeup of the invoker, the handler, the gateway and the
//! executor, one at a time, ticks the machines that are due, and moves the frames
//! they produce until nothing is left to move at that instant. No clock is read,
//! nothing sleeps.
//!
//! The harness records every frame it moves, with its time and direction (the
//! [`Traced`] list). A frame a gateway fault drops is in the trace of the side
//! that SENT it and in the fault log, never in the trace of the side it did not
//! reach.

#![allow(dead_code)]

use std::cell::Cell;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Once;

use ogar_action_handler::invoke_session::{Effect as IEffect, InvokeSession};
use ogar_action_handler::invoke_state::{CallRequest, Completion, Log, TimeoutArg};
use ogar_action_handler::mock_gateway::{Direction, MockConfig, MockError, MockGateway, Out, Side};
use ogar_action_handler::session::{Effect as HEffect, Session, run_capability};
use ogar_action_handler::submit_state::{Config, Episode, ExecResult, Millis, RESULT_TTL_MS};
use ogar_from_schema::action_ws::CapabilityExecutor;
use ogar_from_schema::do_arm::ActionParam;
use ogar_vocab::ActionDef;
use serde_json::{Value, json};

/// The handler id every request is addressed to (`mock.ex:122-124`).
pub const HANDLER_ID: &str = "action_handler";
/// The capability the executor serves and the mock knows.
pub const CAPABILITY: &str = "ExecuteCommand";

/// The message the test capability panics with.
const PANIC_MESSAGE: &str = "the capability panicked";

/// A panicking executor writes its message through the default panic hook. Hide
/// that one message, and only that one, so a failed assertion still prints.
fn quiet_panics() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let payload = info.payload();
            let ours = payload
                .downcast_ref::<&str>()
                .is_some_and(|m| *m == PANIC_MESSAGE)
                || payload
                    .downcast_ref::<String>()
                    .is_some_and(|m| m == PANIC_MESSAGE);
            if !ours {
                default(info);
            }
        }));
    });
}

/// What the capability does when it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Behavior {
    /// Returns `{"output": <text>}`.
    Succeed(String),
    /// Returns `Err(message)`: status 54.
    Fail(String),
    /// Panics: status 54, banner only.
    Panic,
}

/// The capability, as a [`CapabilityExecutor`]. It does not know the request id,
/// which is what the real trait gives an executor too.
pub struct Capability {
    pub behavior: Behavior,
    /// How many times `execute` was called.
    pub calls: Cell<u32>,
}

impl CapabilityExecutor for Capability {
    fn execute(
        &self,
        _capability: &str,
        _bound: &[(String, String)],
    ) -> Result<Vec<(String, String)>, String> {
        self.calls.set(self.calls.get() + 1);
        match &self.behavior {
            Behavior::Succeed(text) => Ok(vec![("output".to_owned(), text.clone())]),
            Behavior::Fail(message) => Err(message.clone()),
            Behavior::Panic => panic!("{}", PANIC_MESSAGE),
        }
    }
}

/// How long a run takes on the virtual clock, from the request id and which run
/// of that id it is (1 for the first).
pub type DurationFn = Box<dyn FnMut(&str, u32) -> Millis>;

/// The executor test double: counts the runs per request id, says what a run
/// returns and how long it takes.
pub struct Executor {
    pub capability: Capability,
    pub duration: DurationFn,
    /// The runs started per request id.
    pub runs: BTreeMap<String, u32>,
}

impl Executor {
    fn new() -> Self {
        Self {
            capability: Capability {
                behavior: Behavior::Succeed("done".to_owned()),
                calls: Cell::new(0),
            },
            duration: Box::new(|_, _| 0),
            runs: BTreeMap::new(),
        }
    }

    /// Every run takes `ms`.
    pub fn take(&mut self, ms: Millis) {
        self.duration = Box::new(move |_, _| ms);
    }

    /// The runs started for `id`.
    pub fn runs_of(&self, id: &str) -> u32 {
        self.runs.get(id).copied().unwrap_or(0)
    }

    /// The runs started in all.
    pub fn total_runs(&self) -> u32 {
        self.runs.values().sum()
    }
}

/// One frame the harness moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Traced {
    pub at: Millis,
    pub dir: Direction,
    pub text: String,
}

#[derive(Debug)]
enum Step {
    InvokerSent(String),
    HandlerSent(String),
    ToInvoker(String),
    ToHandler(String),
}

struct Finishing {
    due: Millis,
    seq: u64,
    id: String,
    result: ExecResult,
}

/// The `type` of a frame, or `error<code>` for a typeless refusal.
pub fn kind(text: &str) -> String {
    let v: Value = serde_json::from_str(text).expect("a frame is JSON");
    if let Some(t) = v.get("type").and_then(Value::as_str) {
        return t.to_owned();
    }
    match v.pointer("/error/code").and_then(Value::as_i64) {
        Some(code) => format!("error{code}"),
        None => "untyped".to_owned(),
    }
}

pub fn parse(text: &str) -> Value {
    serde_json::from_str(text).expect("JSON")
}

/// The closed loop. The fields are public so a scenario can read the machines.
pub struct ClosedLoop {
    pub now: Millis,
    pub invoker: InvokeSession,
    pub gateway: MockGateway,
    pub handler: Session,
    pub executor: Executor,
    pub trace: Vec<Traced>,
    pub completions: Vec<(Millis, Completion)>,
    pub invoker_logs: Vec<(Millis, Log)>,
    /// Frames a machine refused to decode.
    pub wire_errors: Vec<String>,
    /// Frames the gateway refused.
    pub gateway_errors: Vec<MockError>,
    /// How many times the handler was restarted.
    pub restarts: u32,
    def: ActionDef,
    signature: Vec<ActionParam>,
    work: VecDeque<Step>,
    finishing: Vec<Finishing>,
    seq: u64,
}

impl ClosedLoop {
    /// A loop whose gateway knows `ExecuteCommand` (mandatory `command`, no
    /// defaults), with the machines not yet connected.
    pub fn new(config: MockConfig) -> Self {
        quiet_panics();
        let mut gateway = MockGateway::new(config);
        gateway
            .put_capabilities(r#"{"ExecuteCommand": {"command": null}}"#)
            .expect("capabilities");
        let def = ActionDef::new(
            "ogit-automation/action_capability::action_def::ExecuteCommand",
            CAPABILITY,
            "ogit-automation/mars_machine",
        );
        let signature = vec![ActionParam {
            name: "command".to_owned(),
            mandatory: true,
            default: None,
        }];
        Self {
            now: 0,
            invoker: InvokeSession::new(),
            gateway,
            handler: Session::new(Config::default()).expect("config"),
            executor: Executor::new(),
            trace: Vec::new(),
            completions: Vec::new(),
            invoker_logs: Vec::new(),
            wire_errors: Vec::new(),
            gateway_errors: Vec::new(),
            restarts: 0,
            def,
            signature,
            work: VecDeque::new(),
            finishing: Vec::new(),
            seq: 0,
        }
    }

    /// The loop with the default gateway, connected.
    pub fn connected() -> Self {
        let mut lp = Self::new(MockConfig::default());
        lp.connect();
        lp
    }

    /// Both peers connect: each is sent the gateway's `hello`.
    pub fn connect(&mut self) {
        for side in [Side::Invoker, Side::Handler] {
            let text = self.gateway.hello_text(side);
            self.work.push_back(match side {
                Side::Invoker => Step::ToInvoker(text),
                Side::Handler => Step::ToHandler(text),
            });
        }
        self.pump();
    }

    // ------------------------------------------------------------- driving --

    /// A caller starts a request now.
    pub fn call(&mut self, request: CallRequest) {
        let fx = self.invoker.call(self.now, request);
        self.on_invoker_effects(fx);
        self.pump();
    }

    /// The usual request: capability `ExecuteCommand`, parameter `command`, and
    /// a request timeout of `seconds`.
    pub fn request(call: u64, command: &str, seconds: i64) -> CallRequest {
        CallRequest::new(call, "ticket-1", HANDLER_ID, CAPABILITY)
            .param("command", json!(command))
            .with_timeout(TimeoutArg::Int(seconds))
    }

    /// Hand the invoker or the handler a frame of the test's own making, now,
    /// bypassing the gateway's fault rules.
    pub fn inject(&mut self, to: Side, text: &str) {
        let outs = self.gateway.inject(self.now, to, text);
        self.enqueue(outs);
        self.pump();
    }

    /// Deliver frames the gateway handed back from an injection.
    pub fn deliver(&mut self, outs: Vec<Out>) {
        self.enqueue(outs);
        self.pump();
    }

    /// The handler process restarts: a fresh session (its cache is gone), the
    /// executions in flight are lost, and the connection is sent a new `hello`.
    pub fn restart_handler(&mut self) {
        self.restarts += 1;
        self.handler = Session::new(Config::default()).expect("config");
        self.finishing.clear();
        let outs = self.gateway.restart_handler(self.now);
        self.enqueue(outs);
        self.pump();
    }

    /// The earliest time at which anything would happen.
    pub fn next_event(&self) -> Option<Millis> {
        [
            self.invoker.next_wakeup(),
            self.handler.next_wakeup(),
            self.gateway.next_wakeup(),
            self.finishing.iter().map(|f| f.due).min(),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// Run the virtual clock to `t`, firing every event on the way.
    pub fn run_until(&mut self, t: Millis) {
        let mut guard = 0_u32;
        while let Some(next) = self.next_event() {
            if next > t {
                break;
            }
            guard += 1;
            assert!(guard < 200_000, "the loop is not making progress at {next}");
            self.now = self.now.max(next);
            self.fire();
        }
        self.now = self.now.max(t);
        self.pump();
    }

    /// Run `ms` more.
    pub fn run_for(&mut self, ms: Millis) {
        self.run_until(self.now + ms);
    }

    /// Run on until the handler's dedupe window has closed for everything it
    /// saw, so that nothing is tracked but what HIRO's cache keeps for 60
    /// minutes.
    pub fn settle(&mut self) {
        self.run_for(RESULT_TTL_MS + 1_000);
    }

    /// Fire what is due at `now`, in a fixed order: the gateway releases its
    /// queued frames (and executions that have finished are reported as the
    /// frames move), the handler ticks, the invoker ticks.
    fn fire(&mut self) {
        let outs = self.gateway.on_tick(self.now);
        self.enqueue(outs);
        self.pump();
        let fx = self.handler.on_tick(self.now);
        self.on_handler_effects(fx);
        self.pump();
        let fx = self.invoker.on_tick(self.now);
        self.on_invoker_effects(fx);
        self.pump();
    }

    // -------------------------------------------------------------- moving --

    fn enqueue(&mut self, outs: Vec<Out>) {
        for Out { to, text } in outs {
            self.work.push_back(match to {
                Side::Invoker => Step::ToInvoker(text),
                Side::Handler => Step::ToHandler(text),
            });
        }
    }

    fn record(&mut self, dir: Direction, text: &str) {
        self.trace.push(Traced {
            at: self.now,
            dir,
            text: text.to_owned(),
        });
    }

    fn on_invoker_effects(&mut self, fx: Vec<IEffect>) {
        for e in fx {
            match e {
                IEffect::Send { text, .. } => self.work.push_back(Step::InvokerSent(text)),
                IEffect::Complete(c) => self.completions.push((self.now, c)),
                IEffect::Log(l) => self.invoker_logs.push((self.now, l)),
                _ => {}
            }
        }
    }

    fn on_handler_effects(&mut self, fx: Vec<HEffect>) {
        for e in fx {
            match e {
                HEffect::Send(text) => self.work.push_back(Step::HandlerSent(text)),
                HEffect::Run {
                    id,
                    capability,
                    parameters,
                } => self.start_run(&id, &capability, &parameters),
            }
        }
    }

    /// A run starts: counted per request id, its result decided now by the real
    /// `run_capability`, its end scheduled on the virtual clock.
    fn start_run(&mut self, id: &str, capability: &str, parameters: &[(String, String)]) {
        let n = self.executor.runs.entry(id.to_owned()).or_insert(0);
        *n += 1;
        let nth = *n;
        let result = run_capability(
            capability,
            parameters,
            &self.def,
            &self.signature,
            &self.executor.capability,
        );
        let took = (self.executor.duration)(id, nth);
        self.seq += 1;
        self.finishing.push(Finishing {
            due: self.now.saturating_add(took),
            seq: self.seq,
            id: id.to_owned(),
            result,
        });
    }

    /// Move frames until none is left at this instant.
    pub fn pump(&mut self) {
        while let Some(step) = self.work.pop_front() {
            match step {
                Step::InvokerSent(text) => {
                    self.record(Direction::InvokerToGateway, &text);
                    match self.gateway.on_invoker_text(self.now, &text) {
                        Ok(outs) => self.enqueue(outs),
                        Err(e) => self.gateway_errors.push(e),
                    }
                }
                Step::HandlerSent(text) => {
                    self.record(Direction::HandlerToGateway, &text);
                    match self.gateway.on_handler_text(self.now, &text) {
                        Ok(outs) => self.enqueue(outs),
                        Err(e) => self.gateway_errors.push(e),
                    }
                }
                Step::ToInvoker(text) => {
                    self.record(Direction::GatewayToInvoker, &text);
                    match self.invoker.on_text(self.now, &text) {
                        Ok(fx) => self.on_invoker_effects(fx),
                        Err(e) => self.wire_errors.push(format!("invoker: {e}")),
                    }
                }
                Step::ToHandler(text) => {
                    self.record(Direction::GatewayToHandler, &text);
                    match self.handler.on_text(self.now, &text) {
                        Ok(fx) => self.on_handler_effects(fx),
                        Err(e) => self.wire_errors.push(format!("handler: {e}")),
                    }
                }
            }
        }
        // An execution that ends at this instant (a zero-length one, or one
        // whose end is due) is reported, and its frames moved.
        let mut due: Vec<Finishing> = Vec::new();
        let mut keep = Vec::new();
        for f in self.finishing.drain(..) {
            if f.due <= self.now {
                due.push(f);
            } else {
                keep.push(f);
            }
        }
        self.finishing = keep;
        if due.is_empty() {
            return;
        }
        due.sort_by_key(|f| (f.due, f.seq));
        for f in due {
            let fx = self.handler.on_finished(self.now, &f.id, f.result);
            self.on_handler_effects(fx);
        }
        self.pump();
    }

    // ---------------------------------------------------------- inspection --

    /// `(time, kind)` of the frames in one direction, in order.
    pub fn kinds(&self, dir: Direction) -> Vec<(Millis, String)> {
        self.trace
            .iter()
            .filter(|t| t.dir == dir)
            .map(|t| (t.at, kind(&t.text)))
            .collect()
    }

    /// How many frames of `kind` went in `dir`.
    pub fn count(&self, dir: Direction, frame_kind: &str) -> usize {
        self.kinds(dir)
            .iter()
            .filter(|(_, k)| k == frame_kind)
            .count()
    }

    /// The times at which frames of `kind` went in `dir`.
    pub fn times(&self, dir: Direction, frame_kind: &str) -> Vec<Millis> {
        self.kinds(dir)
            .into_iter()
            .filter(|(_, k)| k == frame_kind)
            .map(|(t, _)| t)
            .collect()
    }

    /// The parsed frames of `kind` in `dir`.
    pub fn frames(&self, dir: Direction, frame_kind: &str) -> Vec<Value> {
        self.trace
            .iter()
            .filter(|t| t.dir == dir && kind(&t.text) == frame_kind)
            .map(|t| parse(&t.text))
            .collect()
    }

    /// The request id of the first submit the invoker sent.
    pub fn first_request_id(&self) -> String {
        self.frames(Direction::InvokerToGateway, "submitAction")[0]["id"]
            .as_str()
            .expect("id")
            .to_owned()
    }

    /// What `call` finally received. Panics unless it completed exactly once.
    pub fn outcome(&self, call: u64) -> &Completion {
        let all: Vec<&Completion> = self
            .completions
            .iter()
            .filter(|(_, c)| c.call == call)
            .map(|(_, c)| c)
            .collect();
        assert_eq!(all.len(), 1, "call {call} completed {} times", all.len());
        all[0]
    }

    /// When `call` completed.
    pub fn completed_at(&self, call: u64) -> Millis {
        self.completions
            .iter()
            .find(|(_, c)| c.call == call)
            .map(|(t, _)| *t)
            .expect("completed")
    }

    /// The handler's episode for `id`, open or finished.
    pub fn episode(&self, id: &str) -> Episode {
        self.handler
            .state()
            .episode(id)
            .cloned()
            .or_else(|| {
                self.handler
                    .state()
                    .finished_episodes()
                    .filter(|e| e.intervention_id == id)
                    .last()
                    .cloned()
            })
            .unwrap_or_else(|| panic!("no episode for {id}"))
    }

    /// All three machines hold nothing beyond HIRO's cache: no call, no
    /// execution, no frame in flight; the handler tracks nothing once its
    /// 60 minute window has passed.
    pub fn assert_clean(&self) {
        assert_eq!(self.invoker.state().tracked(), 0, "invoker calls");
        assert_eq!(self.invoker.state().tracked_ids(), 0, "invoker ids");
        assert_eq!(self.handler.state().tracked(), 0, "handler ids");
        assert_eq!(self.gateway.queued(), 0, "gateway queue");
        assert!(self.finishing.is_empty(), "executions in flight");
        assert!(self.work.is_empty(), "frames in flight");
        assert!(self.wire_errors.is_empty(), "{:?}", self.wire_errors);
        assert!(self.gateway_errors.is_empty(), "{:?}", self.gateway_errors);
    }
}
