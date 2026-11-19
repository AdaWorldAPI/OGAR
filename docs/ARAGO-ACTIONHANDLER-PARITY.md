# Arago ActionHandler ⟷ OGAR — parity scorecard + the Python→OGAR switch path

> **Status:** FINDING (contract parity `[G]`, grounded in shipped code + the
> OGIT ontology + arago's published sources) + the runtime executor as
> a specced, NOT-yet-built brick (`[H]`, gated on `PROBE-OGAR-ACTIONHANDLER-RUN`).
>
> **Correction 2026-11-18:** several `[G]`/SHIPPED claims below overclaim what the
> code does. See §8 (the list, with the evidence). Superseded lines are marked
> inline and kept for history.
>
> **Goal (operator):** reach parity with arago's HIRO ActionHandler such that one
> "could basically switch from [arago's] Python to OGAR running it here." This
> doc scores exactly how far that is true today and names the remaining runtime.
>
> **Sources (arago, verbatim):** `github.com/arago/ActionHandlers` (the handler
> config format), `arago/python-hiro-stonebranch-actionhandler` (a concrete
> daemon), and the HIRO 7 **Action API** tutorial + machine-readable specs (the
> `action-ws` protocol, served by the HIRO 7 dev portal).
> The OGIT `NTO/Automation` ontology (in AdaWorldAPI/OGIT, read from a checkout)
> is the contract's schema.

---

## 0. What an arago ActionHandler *is* (three layers)

```
                        ┌─────────────────── the daemon ───────────────────┐
   HIRO engine  ◄── action-ws (WebSocket) ──►  ActionHandler process
   (issues need               │                  registers: Configuration
    a Capability              │                            + Capabilities
    on a Node)                │                            + Applicabilities
                              ▼
   submitAction ──► [match Capability + Applicability] ──► execute Command ──► sendActionResult
```

1. **Config layer** (declarative — `arago/ActionHandlers` YAML): a handler
   registers **Capabilities** (named operations + typed params + a Command/script)
   and **Applicabilities** (a node-match `ModelFilter` gating which capability
   applies where).
2. **Ontology layer** (OGIT `NTO/Automation`): the same shape as RDF entities —
   `ActionHandler --provides--> ActionApplicability --provides--> ActionCapability`.
3. **Runtime layer** (`action-ws`): the engine `submitAction`s; the handler
   `acknowledged`s, executes, `sendActionResult`s; the engine `acknowledged`s.

OGAR must reach parity on all three for the switch to be real. The scorecards
below take them in turn.

---

## 1. Config + ontology parity — `[G]` SHIPPED

The arago handler config (the SSH/Stonebranch YAML) and the OGIT ontology are
two encodings of one contract. OGAR lifts that contract via
`ogar-from-schema::do_arm` (`assemble_action_handler`), grounded in the
OGIT `provides` graph — **proven by `assembles_the_full_action_handler_contract`**.

| arago config field | OGIT ontology | OGAR type (`do_arm`) | status |
|---|---|---|---|
| `Capability.Name` | `ActionCapability` | `ActionDef.predicate` | `[G]` |
| `Capability.Description` | `ogit:description` | (class description) | `[G]` |
| `Capability.Parameter[]{Name,Mandatory,Default}` | `mandatoryParameters` / `optionalParameters` | `CapabilitySlot.declares_{mandatory,optional}_params` + `ActionParam{name,mandatory,default}` | `[G]` shape / `[H]` values¹ |
| `Capability.resultParameters` (**the output**) | `resultParameters` | `CapabilitySlot.declares_result_params` | `[G]` shape / `[H]` values¹ (2026-11-18: the REST DTO now keeps it, `RegisteredCapability::result_parameters`; before, it was dropped silently) |
| `Capability.Command` / `Interpreter` | `knowledgeItemFormalRepresentation` (the body) | `ActionDef.body_source` — **pointed-to, never inlined** (lossless-DO §1) | ~~`[G]`~~ NOT DONE in code (2026-11-18): `body_source` is always `None` (`do_arm.rs` `into_action_def`); no code stores a command text, the command is whatever the submit carries |
| `Applicability.ModelFilter{Var,Mode,Value}` | `environmentFilter` | **`KausalSpec::StateGuard{guard_field,guard_values}`** | ~~`[G]`~~ UNCONFIRMED (2026-11-18): `ModelFilter` and `Priority` are not found in any HIRO client repository (graph_conn, hiro-client-python, hiro-client-java, python-arago-actors). The legacy guard reading; the applicability is the ON, see §8 |
| `Applicability.Priority` | — | (gap — no priority field) | `[H]`² |
| `ActionHandler` (the registration) | `ActionHandler` `--provides-->` | `ActionHandlerSpec{identity, applicabilities[]}` | `[G]` |

¹ **shape vs values:** the OGIT ontology (and OGAR's lift of it) declares the
param *slots*. The concrete `(name, mandatory, default)` tuples live in a
*deployed* handler's config YAML — an **instance lift** (§4, the one new producer
needed). `ActionParam` is the OGAR type that carries them.

² `Priority` (applicability precedence) has no OGAR home yet — a one-field add
when the executor needs it (the MUL/elevation layer already ranks; priority maps
there).

**Net: the contract SHAPE is at full parity.** The OGAR `ActionHandlerSpec`
carries the same `(handler → applicabilities[guard] → capabilities[I/O signature])`
arago registers with the engine.

---

## 2. Runtime parity — the `action-ws` protocol ⟷ OGAR `ActionInvocation`

The HIRO Action API lifecycle maps **field-for-field** onto OGAR's
`ActionInvocation` Rubicon lifecycle (`lance-graph-contract::action`). This is
the load-bearing claim for "run it here": the wire protocol is unchanged; only
the daemon's *insides* become OGAR.

| `action-ws` message / field | direction | OGAR mapping | status |
|---|---|---|---|
| `submitAction.capability` | engine → handler | resolve the `ActionDef` whose `predicate` == capability | `[G]` |
| `submitAction.parameters{…}` | engine → handler | bind to the capability's `ActionParam[]` → `ActionInvocation` inputs | `[H]` (needs the param-bind step) — 2026-11-18: stale, `bind_parameters` exists and `handle_submit` calls it |
| `submitAction.handler` / `scope` | engine → handler | `ActionInvocation.lokal{actor,tenant}` | `[G]` type / `[H]` wiring — 2026-11-18: only set inside `submit_to_invocation`, which `handle_submit` never calls; `scope` is not a HIRO wire field |
| `submitAction.timeout` (ms) | engine → handler | `ActionDef.state_timeout_millis` (`ogar:StateTimeout`) | ~~`[G]`~~ NOT DONE in code (2026-11-18): `timeout_millis` is carried but never read, and no executor has a deadline. Timeouts: in progress, not merged (another slice) |
| `submitAction.id` (`$appId:$requestId`) | engine → handler | `ActionInvocation.idempotency_key` (OLD↔NEW correlation) | ~~`[G]`~~ type only (2026-11-18): set in `submit_to_invocation`, which `handle_submit` never calls |
| **handler** `acknowledged{id,code:200}` | handler → engine | `ActionInvocation.state = Pending` (received, not yet committed) | `[G]` type / `[H]` emit |
| *execute* | (internal) | `commit_via<ClassRbac>`: RBAC verb-gate ∧ `StateGuard` (the ModelFilter) ∧ MUL impact | `[G]` gate / `[H]` body-exec |
| `sendActionResult{id, result{…}}` | handler → engine | `state = Committed`; `result` = the `resultParameters` output payload | `[G]` lifecycle / `[H]` result-build |
| **engine** `acknowledged` (result received) | engine → handler | the **Lance commit** (`CommitHook::on_commit`) — "state history IS the version log" | `[G]` |
| retry-on-no-ack | engine | at-least-once → `idempotency_key` dedup (`ModalSpec::Idempotent`) | ~~`[G]`~~ NOT DONE in code (2026-11-18): `handle_submit` is stateless, a repeated id executes twice. Dedup: in progress, not merged (another slice) |

**The lifecycle is already OGAR's lifecycle.** `submitAction → acknowledged →
execute → sendActionResult → acknowledged` IS `Pending → (RBAC∧guard∧MUL) →
Committed → Lance-append`. The Rubicon crossing (`Pending → Committed`,
`is_commit`) is exactly the `sendActionResult` moment. Nothing in the protocol
needs a type OGAR lacks — only the *glue* (§3) is unbuilt.

---

## 2a. The harvested authoritative contract (`action-ws.yaml` + `action.yaml`)

Harvested from the HIRO 7 Action API machine-readable specs
(`action-ws.yaml` + `action.yaml`, served by the HIRO 7 dev portal).
This is the **complete** message set + connection + registration model the OGAR
`action_ws` module is built to (corrections folded in — the earlier `result{…}`
object framing from the tutorial is superseded by the spec's `result: string`).

**Connection.** `wss://<host>/api/action-ws/1.0/connect` — token passed as the
WebSocket subprotocol `sec-websocket-protocol: token-$TOKEN`. (`action_ws::{ACTION_WS_PATH,
auth_subprotocol}`.)

**The six message types** (✓ = modelled in `action_ws`; 2026-11-18: the serde derives on those types are NOT HIRO's wire, the codec is `ogar_action_handler::wire`, see §8):

| type | dir | fields | OGAR type |
|---|---|---|---|
| `submitAction` | engine → handler | `id` (12–256), `handler`, `capability`, `parameters` (obj), `timeout` (ms) | `SubmitAction` ✓ |
| `sendActionResult` | handler → engine | `id`, `result` (**string**, ≤ `1048576`) | `SendActionResult` ✓ (`result` = `json_object(resultParams)`) |
| `acknowledged` | both | `id`, `code` (200), `message` | `Acknowledged` ✓ (`acknowledge`) |
| `negativeAcknowledged` | both | `id`, `code` (e.g. 400), `message` | `NegativeAcknowledged` ✓ (`negative_acknowledge`) |
| `configChanged` | engine → handler | `type` only → re-fetch capabilities | `ConfigChanged` ✓ |
| `error` | engine → handler | `code`, `message` (no `id`) | `InboundError` ✓ |

**Corrections the spec forced** (vs the tutorial-only first pass): `result` is a
**single string** (≤ 1 MiB), not an object — `invocation_to_result` JSON-encodes
the bound `resultParameters` into it (`MAX_RESULT_LEN`, `ResultTooLarge`); `id`
is **12–256 chars** (`validate_id`, `InvalidId`); the nack / configChanged /
error message types now exist.

**Registration is REST, not a WS handshake.** A handler's capabilities and
applicabilities live in the graph and are read via the REST **Action API**
(`/api/action/1.0/`, Bearer token): `GET /capabilities` →
`MapOfCapabilities` (each `{description, mandatoryParameters, optionalParameters}`),
`GET /applicabilities` → `MapOfApplicabilities` (keyed by handler id). The
`action-ws` socket has **no registration message** — `configChanged` just tells
the handler to re-`GET` them. This is exactly the `do_arm::assemble_action_handler`
shape (handler → applicabilities → capabilities), so the REST registration view
lifts straight into `ActionHandlerSpec` (the B2-lift brick).

---

## 3. The switch path — replacing the Python daemon with OGAR

To "switch from Python to OGAR running it here," one OGAR-side component
replaces the arago Python `ActionHandlerDaemon`: an **action-ws adapter** that
speaks the same WebSocket protocol but routes through OGAR types.

```
   HIRO engine ──submitAction──► [OGAR action-ws adapter]
                                     │  1. resolve ActionDef by `capability`           (canonical_concept_id / OgarActionProvider)
                                     │  2. build ActionInvocation, bind `parameters`   (ActionParam[])
                                     │  3. acknowledged{200}                            (emit)
                                     │  4. commit_via<ClassRbac>(rbac, actor, guard…)   (RBAC ∧ ModelFilter-StateGuard ∧ MUL)
                                     │  5. execute the capability body (Command)        ◄── the ExecTarget executor
                                     │  6. sendActionResult{ result: resultParameters } (emit)
                                     ▼
                                  Lance commit (CommitHook) ── server acknowledged
```

Steps 1–4 and 6 are OGAR types/logic that **exist today** — **the protocol core
is shipped** in `ogar-from-schema::action_ws` (the testable, socket-free binding):

- `SubmitAction` / `Acknowledged` / `SendActionResult` — the typed `action-ws`
  messages (serde-gated for the wire; 2026-11-18: that derive is not HIRO's JSON shape, the codec is `ogar_action_handler::wire`).
- `acknowledge(submit)` — step 3 (the 200 receipt).
- `bind_parameters(supplied, signature)` — validates the engine's `parameters`
  against the capability's `ActionParam[]` (mandatory present, defaults filled) —
  the same check arago's Python handler runs before executing.
- `submit_to_invocation(submit, def)` — step 1+2: builds the `Pending`
  `ActionInvocation` (capability→predicate match, `id`→`idempotency_key`,
  `host`→`object_instance`, `handler`/`scope`→`lokal`).
- `invocation_to_result(committed_inv, result)` — step 6: only a `Committed`
  invocation (the Rubicon crossing) yields the `sendActionResult`.

The full loop is proven socket-free by `full_action_ws_roundtrip`. The **reactive
dispatch is now shipped too** — `action_ws::handle_submit(msg, def, signature,
executor)` runs the whole handler reaction (validate → ack-or-nack → bind →
execute → `sendActionResult`) behind the **`CapabilityExecutor`** trait (the B1
seam), with the RBAC/guard gate (`commit_via`) owned by the executor impl
downstream. The remaining bricks:

- **B1 — the executor (`CapabilityExecutor` impl).** Step 5: actually run the
  capability's Command/script and capture stdout/exit → `resultParameters`. The
  **native target is SHIPPED**: `ogar-action-handler::NativeCommandExecutor` runs
  `ExecuteCommand` via a local POSIX shell and returns `output`/`stderr`/`exitcode`
  — proven end-to-end by `full_dispatch_runs_a_real_command` ("OGAR running it
  here," native). The **REST target is SHIPPED too**: rs-graph-llm
  `graph-flow-action-ogar::rest::RestExecutor` (`feature = "rest"`, pure-Rust
  `ureq`) POSTs the bound params to an HTTP endpoint and returns the response as
  `resultParameters` — the arago HTTP-callout shape — and runs only behind the
  gate (`rest_executor_runs_only_behind_the_gate`). The **SSH target** is coded
  too: `ogar-action-handler::SshExecutor` shells out to the system `ssh`
  (dep-free, non-interactive `BatchMode=yes`) — arago's canonical
  `ExecuteCommand`-over-SSH, the native executor made remote; its argv
  construction + pre-spawn guards are tested, end-to-end exec needs a live host
  (no sshd in CI). WinRM is the one executor target left. The native + SSH
  (Command-based, dep-free) executors live in OGAR `ogar-action-handler`; the
  network ones (REST, library-based) live in rs-graph-llm.
- **B1-uplink — the hard gate before the executor (SHIPPED).** rs-graph-llm's
  `graph-flow-action-ogar` crate is the seam: `GatedOgarHandler` wraps an OGAR
  `CapabilityExecutor` as a `graph-flow-action::ActionHandler`, so the executor's
  `handle` runs **only after** `dispatch_via`'s cold floor commits
  (`commit_via`: def-match → RBAC `ClassRbac` → state-guard → MUL). The structural
  proof: `take_result()` is `None` whenever the gate refused — `run_gated` with an
  unauthorized actor (`Denied`) or a MUL `Block` (`Escalated`) never reaches the
  OGAR executor. Three tests pin it; `NativeCommandExecutor` runs the real command
  only on the authorized path. OGAR owns the executor; rs-graph-llm owns the gate.
- **B2-transport — the live daemon (SHIPPED, WebSocket edge).** Built in
  rs-graph-llm's `graph-flow-action-ogar::daemon` as a **transport-agnostic** core:
  `Daemon::react` turns one inbound `action-ws` JSON frame into the outbound frames
  it warrants (`acknowledged` + `sendActionResult`, or `negativeAcknowledged`),
  running the hard gate (`run_gated`) + the executor in between — pure, no I/O. A
  `Transport` trait is the swappable edge (`recv`/`send`); `Daemon::serve` is the
  loop. The **`WsTransport`** WebSocket edge (`feature = "ws"`) connects with the
  `token-$TOKEN` subprotocol and is proven by a mock-server roundtrip
  (`ws_roundtrip_against_a_mock_server`: engine `submitAction` → ack → run → result
  over a real socket). The connection identity is an `Auth` type shaped after OGIT
  `NTO/Auth/Configuration` (`auth_store` `0x0B01`) — the same principal the
  transport authenticates as (`accountId`) is the actor the gate authorizes.
  **HIRO also distributes actions over Kafka**; that edge (`rdkafka` over the same
  `Transport` trait) is reserved — the core is ready, it needs the topic/record
  shape pinned.
- **B2-lift — the instance config lift (SHIPPED for capabilities).** Parse a
  deployed handler's REST registration → the concrete signatures the *schema*
  half cannot supply. `GET /capabilities` is **shipped**: `registration::{RegisteredCapability,
  lift_registration}` (the pure lift, in the parser-free producer) +
  `ogar-action-handler::parse_capabilities` (the `serde_json` read, in the runtime)
  turn a real `MapOfCapabilities` JSON body into `ConcreteCapability` —
  `ActionParam[]` with concrete `(name, mandatory, default)` — proven end-to-end by
  `rest_registration_lifts_binds_and_runs` (JSON → lift → `bind_parameters` →
  `NativeCommandExecutor` runs the command). The applicability side is **shipped
  too**: `GET /applicabilities` → `registration::{RegisteredApplicability,
  lift_applicabilities}` + `ogar-action-handler::parse_applicabilities` turn a real
  `MapOfApplicabilities` JSON body into per-handler `StateGuard` sets (handler id →
  `Vec<KausalSpec>`) — the documented field-for-field `ModelFilter{Var,Mode,Value}`
  → `KausalSpec::StateGuard` lift, proven by `rest_applicabilities_lift_to_per_handler_guards`.
  The only residual is cosmetic: the inner filter-list field name is alias-flexible
  (`modelFilters` / `model` / `filters`) pending confirmation against a live
  response — the lift itself is exact.
  *(Superseded 2026-11-18: this is the legacy guard reading of an UNCONFIRMED shape,
  not found in HIRO's client repositories; the readable shape is graph_conn's `on
  <expr>` form, read as `AppliesOn`. See §8.)*

What remains is **glue over existing types** (a socket loop + a JSON codec + a
registration parser) plus the non-native executor targets — not new IR. The
*contract, the lifecycle, the protocol binding, the reactive dispatch, and a
working native executor* are OGAR-native and proven; the live daemon is a thin
transport over them.

---

## 4. Scorecard — are we at parity?

| Layer | Parity | Evidence / remaining |
|---|---|---|
| **Config + ontology contract** | ✅ `[G]` | `assemble_action_handler` over the OGIT graph; `ActionHandlerSpec` / `CapabilitySlot` / `ApplicabilitySlot` / `ActionParam` |
| **ModelFilter → guard** | ~~✅ `[G]`~~ legacy, UNCONFIRMED (2026-11-18) | `environmentFilter` → `KausalSpec::StateGuard` (test). The shape is not found in HIRO's client repositories; the applicability is the ON, not a WHEN guard (§8) |
| **Action lifecycle (protocol)** | ✅ `[G]` (type-level) | `action-ws` ⟷ `ActionInvocation` Pending→Committed; `commit_via` is the gate |
| **RBAC at execute** | ✅ `[G]` | `commit_via<ClassRbac>` (verb-gate ∧ guard ∧ MUL) — shipped in `lance-graph-contract` |
| **action-ws protocol core (B2-core)** | ✅ `[G]` SHIPPED | `action_ws`: all 6 message types + `submit_to_invocation` / `bind_parameters` / `invocation_to_result` (socket-free, `full_action_ws_roundtrip` proven) |
| **Reactive dispatch + B1 seam** | ✅ `[G]` SHIPPED (2026-11-18: stateless, no dedup, no timeout, no resend: §8) | `action_ws::handle_submit` + the `CapabilityExecutor` trait (validate→ack→bind→execute→result; tested with a mock) |
| **Executor — native target (B1)** | ✅ `[G]` SHIPPED | `ogar-action-handler::NativeCommandExecutor` runs `ExecuteCommand` for real; `full_dispatch_runs_a_real_command` |
| **Hard gate before executor (B1-uplink)** | ✅ `[G]` SHIPPED | rs-graph-llm `graph-flow-action-ogar::GatedOgarHandler` — `commit_via` (RBAC ∧ guard ∧ MUL) lands before `handle`; `take_result()` is `None` iff the gate refused (3 tests) |
| **Executor — REST target (B1)** | ✅ `[G]` SHIPPED | rs-graph-llm `graph-flow-action-ogar::rest::RestExecutor` (`feature = "rest"`, ureq) POSTs bound params → resultParameters; runs only behind the gate (`rest_executor_runs_only_behind_the_gate`) |
| **Executor — SSH target (B1)** | 🟡 `[G]` code / `[H]` live | `ogar-action-handler::SshExecutor` shells out to system `ssh` (non-interactive `BatchMode=yes`, same `output`/`stderr`/`exitcode` shape as native) — arago's canonical `ExecuteCommand`-over-SSH, dep-free. argv construction + pre-spawn guards tested; end-to-end needs a live host (no sshd in CI) |
| **Executor — WinRM (B1)** | ⛔ `[H]` | a further `CapabilityExecutor` impl (Windows remote exec) |
| **Live transport — daemon core + WebSocket (B2-transport)** | ✅ `[G]` SHIPPED | rs-graph-llm `graph-flow-action-ogar::daemon`: transport-agnostic `Daemon::react`/`serve` + `Transport` trait + `WsTransport` (action-ws), gate-driving; mock-server roundtrip. `Auth` ← OGIT `NTO/Auth/Configuration` |
| **Live transport — Kafka edge (B2-transport)** | ⛔ `[H]` | `rdkafka` over the same `Transport` trait (action topic → result topic); core ready, needs the topic/record shape pinned |
| **Class-late-bound dispatch (the grail)** | ✅ `[G]` SHIPPED | rs-graph-llm `graph-flow-action-ogar::daemon::ResolvingDaemon` — class resolved from the target's **classid** per action (`ClassResolver`), executor from the resolved `RunnerKind` (`ExecutorRegistry`). `OgarResolver` is the production resolver over the canonical `actions_for(&[ClassActions], classid)` manifest. Proven: one `ExecuteCommand`, `mars_machine` → native / `mars_resource` → REST, zero daemon change; gate still rules |
| **Instance config lift — capabilities (B2-lift)** | ✅ `[G]` SHIPPED | `registration::lift_registration` + `ogar-action-handler::parse_capabilities`: real `GET /capabilities` JSON → `ConcreteCapability` (`ActionParam[]`); `rest_registration_lifts_binds_and_runs` (JSON → lift → bind → run) |
| **Instance config lift — applicabilities (B2-lift)** | 🟡 (2026-11-18, was ✅ `[G]` SHIPPED) | `registration::lift_applicabilities` + `parse_applicabilities`: legacy `ModelFilter` shape → per-handler `StateGuard` sets (UNCONFIRMED shape). `parse_applies_on` reads graph_conn's `on <expr>` shape into `AppliesOn`. The applicability → capability path is not done: nothing evaluates applicabilities when a submit arrives, and resolving `on <expr>` to a class id is not built (intake). |

**Verdict:** OGAR is at **full contract + lifecycle + protocol-binding +
reactive-dispatch parity**, and **a working native executor runs real commands
end-to-end** (`handle_submit` + `NativeCommandExecutor`). Every field of the
config, ontology, and protocol has an OGAR type; the gate (`commit_via`), the
binding, the dispatch, and native execution are real and tested — and the gate is
now **wired to the executor**: rs-graph-llm's `graph-flow-action-ogar` runs OGAR's
`CapabilityExecutor` only after `commit_via` commits, so an unauthorized or
MUL-blocked action never executes (proven structurally — `take_result()` is
`None`). The **whole instance lift is shipped too** — real `GET /capabilities`
and `GET /applicabilities` JSON bodies lift to concrete `ActionParam[]` (runs
end-to-end, `rest_registration_lifts_binds_and_runs`) and per-handler `StateGuard`
sets (`rest_applicabilities_lift_to_per_handler_guards`). And the **live daemon
runs over a real socket** — `graph-flow-action-ogar::daemon` drives the gated
dispatch through a `Transport` trait, with the `action-ws` WebSocket edge proven
by a mock-server roundtrip. Three executor targets run gated — **native** (local
command), **SSH** (remote command, arago's canonical shape — coded, live-host test
pending) and **REST** (HTTP callout). The one thing left for a **live** drop-in
replacement of arago's Python daemon that needs a real input: the **Kafka
transport edge** (HIRO's internal bus — `rdkafka` over the same `Transport` trait,
needs the topic/record shape pinned + a broker to test). WinRM is a further
executor for completeness. Everything is a single edge/runner impl over existing
types — **no missing IR, no missing protocol mapping**. That is the honest state:
OGAR *is* an ActionHandler that reads its own registration, gates every action,
runs commands locally / over SSH / as HTTP callouts, and speaks `action-ws` over a
live socket; a Kafka consumer away from being arago's Python daemon, on a HIRO
deployment that distributes over Kafka.

**The grail — class chosen late from the classid.** Beyond the static daemon, the
`ResolvingDaemon` (`graph-flow-action-ogar::daemon`) holds **no** wired classes and
**no** wired executor: it resolves the action class from the target node's
**classid** per action (`ClassResolver`), and the executor from what that class
resolves to (`RunnerKind` → `ExecutorRegistry`). The production resolver
(`OgarResolver`) is backed by the canonical `actions_for(&[ClassActions], classid)`
DO manifest — OGAR's *"the key prerenders the node; classid → ClassView"* applied
to the action arm. One `ExecuteCommand` dispatches to native (`mars_machine`) or
REST (`mars_resource`) purely by what the classid resolves to, with zero daemon
change — and every action still passes the same hard gate. A new capability /
class / runner is a registry entry, never code (*"scale = the next cascade level,
never field-widening"*).

---

## 5. The probe that promotes B1/B2 from `[H]` to `[G]`

**`PROBE-OGAR-ACTIONHANDLER-RUN`** (the falsifier, mirroring
`PROBE-OGAR-DO-ARM-LIFT` / `PROBE-OGAR-RBAC-AUTHORIZE`): stand up the OGAR
action-ws adapter against a recorded `submitAction` corpus from a real arago
handler (e.g. the SSH `ExecuteCommand`), run it through `commit_via` + the
`ExecTarget` executor, and assert the emitted `sendActionResult.result` matches
the arago handler's recorded result **bit-for-bit**. Green ⇒ the Python daemon
is replaceable; the parity claim is certified, not argued.

---

## 6. Cross-references

- `crates/ogar-from-schema/src/do_arm.rs` — `assemble_action_handler`,
  `ActionHandlerSpec` / `CapabilitySlot` / `ApplicabilitySlot` / `ActionParam`.
- `crates/ogar-from-schema/src/registration.rs` — B2-lift: the REST registration
  DTOs (`RegisteredCapability` / `ModelFilter`) + the pure lift
  (`lift_registration` → `ConcreteCapability`; `model_filter_to_guard`).
- `crates/ogar-action-handler/src/lib.rs` — `parse_capabilities` /
  `parse_applicabilities` (the `serde_json` read of the `GET /capabilities` and
  `GET /applicabilities` bodies; the B2-lift I/O half).
- `docs/HIRO-DO-ARM-LIFT.md` — the lossless-DO rule (the body is pointed-to).
- `docs/ACTIONHANDLER-TURSTEHER.md` — RBAC-as-`const`, the cold-path gate, Rung.
- `lance-graph-contract::action` — `ActionInvocation` / `commit_via<ClassRbac>`.
- `lance-graph-ogar::OgarActionProvider` — the `classid → ClassActions` DO surface.
- `rs-graph-llm/graph-flow-action` — the `ActionHandler` executor trait (B1 home).
- `rs-graph-llm/graph-flow-action-ogar` — the **uplink**: OGAR's
  `CapabilityExecutor` behind the hard gate (`GatedOgarHandler` / `run_gated`);
  `commit_via` lands before any execution.
- `rs-graph-llm/graph-flow-action-ogar/src/daemon.rs` — **B2-transport** + **the
  grail**: the transport-agnostic `Daemon` (`react`/`serve`) + the `Transport`
  trait + `WsTransport` (action-ws WebSocket edge) + the OGIT-`Auth`-derived
  identity; **plus** `ResolvingDaemon` + `ClassResolver` / `ExecutorRegistry` /
  `OgarResolver` (class chosen late from the classid via `actions_for`).
- `rs-graph-llm/graph-flow-action-ogar/src/rest.rs` — the **REST executor**
  (`RestExecutor`, `feature = "rest"`): the arago HTTP-callout target, gated.
- `crates/ogar-action-handler/src/lib.rs` — the **native** (`NativeCommandExecutor`)
  + **SSH** (`SshExecutor`, shells out to `ssh`) executor targets, dep-free.
- arago: `github.com/arago/ActionHandlers`,
  `arago/python-hiro-stonebranch-actionhandler`, HIRO 7 Action API tutorial.
- **HIRO 7 Action API machine-readable specs (the authoritative harvest, §2a):**
  `action-ws.yaml` (the WebSocket message contract), `action.yaml` (the REST
  registration API), `auth.yaml` (the token endpoint) — served by the HIRO 7
  dev portal (`/help/specs/`, indexed under `/7.0/api/`).

---

## 7. Addendum (2026-07-08) — ground-truth re-check against `AdaWorldAPI/ActionHandlers`

A re-check of the §1 config table against the actual vendored handler repo
(`AdaWorldAPI/ActionHandlers`, the arago `ssh_based_actions` `aae.yaml` stanza)
found **two gaps the table did not name**, plus closed the one it did (²):

| arago config field (ground truth) | prior state | closure |
|---|---|---|
| `Applicability.Parameter{Name,Value}` — applicability-scoped defaults (the SSH handler's `SshOpts`, declared once at the applicability and expanded in every capability `Command`) | **unlisted gap** — `RegisteredApplicability` / `ApplicabilitySlot` had no home for it | `registration::ApplicabilityParam` + `RegisteredApplicability::parameters`; lifts to bindable `ActionParam{mandatory:false, default:Some(value)}` via `lift_applicability_full` so `bind_parameters` fills them exactly like arago's template expansion (test: `applicability_params_drive_bind_parameters_like_arago_injection`) |
| `Capability.Parameter.Description` (per-param) | **unlisted gap** — the REST DTO (`RegisteredParam.description`) carried it; `lift_capability` dropped it | `ConcreteCapability::param_descriptions` (kept OFF `ActionParam` deliberately — binding needs `(name,mandatory,default)` only; descriptions are doc/render metadata, so they ride the lift artifact, not the IR type) |
| `Applicability.Priority` (the known §1 footnote ² gap) | `[H]` — "no OGAR home yet" | `RegisteredApplicability::priority: Option<i64>` + `ConcreteApplicability::priority` (the MUL/elevation ranking noted in ² can now consume it) |

All three land in the **instance lift** (`registration.rs` + the
`ogar-action-handler` I/O half — `parse_applicabilities_full`), NOT in the
OGIT-schema lift: the ontology genuinely does not declare them (they are
deploy-config), so `ApplicabilitySlot` (do_arm) is unchanged. Additive only —
`lift_applicability`/`parse_applicabilities` (the guard-only view the hard gate
consumes) are untouched.

Two fidelity notes observed in the same re-check, recorded so they don't
re-surface as "gaps":

- In the **config YAML** the filter nests under `Var:` (`ModelFilter: Var:
  {Name, Mode, Value}`); the flat `ModelFilter{Var,Mode,Value}` this doc (and
  the REST DTO) uses is the spec/REST projection. Same contract, two framings.
- Ground truth uses `Mode: string` — the mode is a match-*type* discriminator
  (`string` vs regex), not only `equals`/`matches` as §the DTO doc-comment
  suggests. `ModelFilter.mode` already preserves it verbatim as data, so
  nothing is lost; the executor consumes it when match-modes matter.

Tests grounding this addendum on the verbatim `ssh_based_actions` values
(Priority 50, `NodeType/string/Machine`, the full `SshOpts` string):
`ssh_stanza_lifts_priority_guards_and_scoped_params`,
`lift_preserves_per_param_descriptions`,
`rest_applicabilities_full_lift_carries_priority_and_scoped_params`.

---

## 8. Correction (2026-11-18) — what the code does, against what this doc claimed

Checked against the code at `1948a7b` and the HIRO client sources (graph_conn, the
Elixir reference handler/invoker; hiro-client-python; hiro-client-java). Earlier text
is kept, with the overclaiming marker struck through where it appears.

**Not done in code (stated as `[G]` above):**

| Claim | State |
|---|---|
| dedup of a repeated `id` (`idempotency_key`, `ModalSpec::Idempotent`) | not done in `handle_submit`, which holds no state: a repeated id executes twice there. Done in `Session` (section 8, "Session", below) |
| `submitAction.timeout` to a deadline | not done in `handle_submit`. Done in `Session`: the frame's `timeout` (ms) is the deadline |
| result resend until acknowledged | not done in `handle_submit`. Done in `Session`: resend every 3000 ms until acked or nacked |
| `body_source` for `Capability.Command` | not done: always `None`; the command is whatever the submit carries |
| applicability → capability path | not done: `RegisteredApplicability` names no capability, nothing evaluates applicabilities on a submit, and resolving `on <expr>` to a class id is not built (it belongs at intake) |
| `handler`/`scope`/`id` mapping onto `ActionInvocation` | only inside `submit_to_invocation`, which `handle_submit` does not call |

**Not found in HIRO's client repositories:** `ModelFilter` (`Var`, `Mode`, `Value`) and
`Priority`, searched in graph_conn, hiro-client-python, hiro-client-java and
python-arago-actors. They come from the arago handler YAML and the earlier harvest
(section 7). They are still accepted, as the legacy guard reading, and labelled
UNCONFIRMED in `registration.rs`. The readable wire shape is graph_conn's: per handler
id, per capability, `{"on <expr>": {param: template}}`, plus the list form
`["on <expr>"]` in a definition (`lib/graph_conn/mock.ex`,
`convert_applicabilities_from_json`; `test/graph_conn/mock_test.exs`). HIRO's library
never expands the `${...}` templates, and neither does OGAR.

**The applicability is the ON.** An applicability names the class a behaviour applies
on, not a WHEN guard. `AppliesOn { handler, capability, on, parameters }` is the
primary reading (`ogar_action_handler::parse_applies_on`,
`parse_applies_on_definitions`, `parse_applicabilities_typed`). The
`ModelFilter → KausalSpec::StateGuard` lift stays, as the legacy guard reading, because
existing tests use it.

**Wire.** The serde derives on the `action_ws` types are not HIRO's JSON (no `type`,
parameters as an array of pairs, `timeout_millis`). The HIRO shapes are encoded and
decoded by `ogar_action_handler::wire` (`type` envelope, parameters as an object,
`timeout` in milliseconds, the typeless in-band 429 and 503), pinned by
`tests/hiro_wire.rs` against fixtures from graph_conn, hiro-client-java and
hiro-client-python. `capabilities` bodies now keep `resultParameters` and accept numeric
defaults (graph_conn registers `"timeout": {"default": 60000}`); tests are in
`tests/hiro_registration.rs`.

**Session (text frame to state machine).** `ogar_action_handler::session::Session` joins
`wire` and `submit_state`: a JSON text frame is decoded, stepped through `HandlerState`, and
comes back as `Effect::Send(text)` or `Effect::Run { id, capability, parameters }`. It is
sans-IO (the caller passes `now`, runs the `Run` effects and reports them with
`on_finished`), and `on_text_sync` is the drop-in that runs `Run` effects through a
`CapabilityExecutor`, so a repeated request id runs once. Wired: ack on claim, one run per
id, 60 minute cached result, 3000 ms resend until `acknowledged` or `negativeAcknowledged`,
the frame `timeout` (ms) as the deadline, the status-13 timeout result. Frames that do not
decode are returned as a `WireError`, with no run and no reply (graph_conn sends none).

**Update (2026-11-19), handler complete.** Also wired, each pinned by a test against the HIRO
source it cites:

| Wired | Where | Tests |
|---|---|---|
| a failed execution is `action_status` 54 with the banner rules of `crash.ex` (an executor `Err(m)` is `** (RuntimeError) m`, a panic is `** (exit)`); a panicking executor is caught in `on_text_sync` / `run_capability`; the crashed result is cached | `crash.rs`, `session::run_capability` | `tests/hiro_crash.rs` |
| an oversized `sendActionResult` becomes status 59 (`Response was too big (X.XX MB, limit Y.YY MB)`), before it is cached; the limit is the latest `hello`'s `max_frame_bytes`, else 1,000,000 bytes; no other frame is ever fitted | `wire::fit`, `HandlerState::set_max_frame_bytes`, `Session` on `hello` | `tests/hiro_frame_fit.rs` |
| `Debug` of `Submit`, `Input`, `Output`, `Effect`, `ExecResult`, the wire frames and the state prints names, counts and lengths, never a parameter or result value (oracle 42) | `submit_state`, `session`, `ogar-from-schema::action_ws` | `tests/hiro_redaction.rs` |
| the frames the Python and Java clients write or document are taken as the Elixir handler takes them (section 10) | `wire`, `session` | `tests/hiro_clients.rs` |
| `acknowledge()` words its message `""`, as the handler does (it read "Received the action") | `ogar-from-schema::action_ws` | `action_ws` unit test, `tests/hiro_session.rs` |

Limits, true after this change:

* No socket, timer or reconnect loop: the caller owns them. The frame limit is per session; a
  new connection is a new `Session` (or `set_max_frame_bytes(None)` until its `hello`).
* Episodes are not persisted; class id, target and effect class stay unset in the episode.
* `handle_submit` is unchanged apart from its ack text: it is stateless, runs a repeated id
  twice, has no deadline, reports an executor `Err` as `{"error":"<message>"}` (not 54) and
  does not fit a result. `Session::on_text_sync` is the stateful drop-in, and the in-repo
  callers (the crate doc example and `full_dispatch_runs_a_real_command`) go through it.
* Bind errors (a missing mandatory parameter) stay the one-field `{"error":"<message>"}`
  result: nothing ran, so it is not an execution failure. Elixir would report a missing
  clause as a `FunctionClauseError` with status 54.
* A panic is caught only with `panic = "unwind"`. The crate installs no panic hook, so the
  default hook still writes the panic message to stderr; a host that keeps messages out of its
  logs sets its own hook.
* `Jason.EncodeError` (status 54 for an unencodable result) has no Rust counterpart: a result
  is a list of string pairs.
* A `negativeAcknowledged` without `code` or `message` is accepted (the codec keeps both
  optional) and stops the resend, as the Elixir handler's does. A NACK with no `id` is still a
  `WireError`, which steps nothing; HIRO finds nothing pending for it either.
* In-band 429 and 503 frames, `configChanged` and the typed `error` are decoded and ignored,
  as the Elixir handler ignores them. Python's resend after `retryAfterMs` is not built.
* Not an invoker: submitting, waiting and the invoker's own inbound path (a result wrapped with
  `code` and `message`, a 500 result) are another change.

## 9. Deferred (2026-11-19) — abort as a recall

**Decision.** Abort is not built now. It is recorded here so the gap is deliberate and the
place for it is known. Revisit when both conditions below hold.

**When it becomes needed.** For tickets (`crm_ticket`, `0x0504`, and any other class whose
actions are queued per target), IF invocations run through virtual actor queues AND
lance-context is used as the feedback loop that decides or re-orders what is queued. A
decision that feedback has since overturned must then be withdrawn before it runs: a
recall. Without queues there is nothing waiting to recall; without feedback there is
nothing that changes its mind.

**What exists today.**

| | HIRO reference handler | OGAR `submit_state` / `session` |
|---|---|---|
| Recall of a submitted action by the sender | none: no abort frame in graph_conn or the client repos | none: `Input` has no abort |
| Stop of a running execution | only the deadline (`Task.shutdown(:brutal_kill)`), reported as a timeout result | only the deadline; a late `Finished` is ignored and the timeout result stands |
| `negativeAcknowledged` | stops the result resend; it is not an abort | same |
| Episode outcome | n/a | `Succeeded`, `Failed`, `TimedOut`; no `Recalled` |

So a recall is an extension beyond HIRO parity, not a parity gap.

**What a recall has to settle when it is built.**

1. *Before the run starts* (still queued): the invocation is dropped and never runs. This
   is the easy case and the one the queue design should make the common one.
2. *While it runs*: the executor must be told to stop, and the effect class decides whether
   that is allowed. A pure predicate or read can be dropped; a `LifecycleWrite` or
   `ExternalHost` action may already have had its effect, so a recall there is at best a
   request and the answer must say what happened.
3. *After it finished*: a recall cannot undo it. Undoing is a compensating action, a new
   invocation with its own id, not a recall.
4. The request id stays claimed for the cache window after a recall, so a late redelivery
   of the recalled submit does not run it after all.
5. The episode records it: an outcome `Recalled`, who recalled (the Actor route), and
   whether the run had started. This is the feedback lance-context needs: a recalled
   intervention is a different signal from a failed or timed-out one.
6. The wire: HIRO has no frame for it. It needs a new frame type or stays inside the queue
   and never reaches the HIRO wire.

**Where it would go.** `submit_state`: one new `Input` (recall of an id), one new `Output`
(stop request to the executor), one new `Outcome`. `session`: only if a wire frame is
chosen. The queue and the lance-context side are outside this crate.

**Amendment (2026-11-19).** There are no mailboxes: the actor layer holds no queue per
target, and time is modelled by versioning through thought git. Where this section says
"queued" or "queue", read: an open thought that has not landed. A recall before the run is
then a thought that does not land. See `SHARED-DO-DESIGN.md` section 10.

## 10. Client variants against the OGAR handler (2026-11-19)

The reference is the Elixir handler (graph_conn `lib/action_api/handler.ex`). OGAR keeps its
behaviour and must take, without failing, what the Python (`P`) and Java (`J`) clients write or
document. Assertion numbers are the oracle's (`HIRO_ORACLE.md` section 7). `P` is
`hiro-client-python/src/hiro_graph_client/actionwebsocket.py`, `J` is
`hiro-client-java/.../websocket/ActionWebSocket.java` and `.../impl/`. Rows 49, 57 and 58 are
about a result's wrapper and a 500 result, which a handler never reads: they belong to the
invoker's inbound path and are not changed here.

| # | Elixir | Python | Java | What OGAR does | Test (`tests/hiro_clients.rs` unless noted) |
|---|---|---|---|---|---|
| 46 | acks a submit with `{id, type, code: 200, message: ""}` (`handler.ex:170`) | `message: "submitAction acknowledged"` (`P:605`) | `{type, id}`; `code` and `message` null (`ActionWebSocket.java:144`; null form UNVERIFIED) | sends the Elixir ack. An inbound ack of any of these shapes stops the resend of that id's result; `code` and `message` are not read | `python_style_ack_with_a_message_stops_the_resend`, `java_style_ack_without_a_code_stops_the_resend`; `hiro_session` `repeated_submit_while_executing_runs_once_and_acks_twice` |
| 47 | a duplicate is acked and joins the waiters (`handler.ex:169-172`, `:201-203`) | acked, then `ActionItemExists` is logged and nothing else (`P:607-611`) | acked, then dropped (`J:146-154`) | acked each time, never run again | `a_duplicate_submit_is_acked_each_time_and_runs_once` |
| 48 | a cached result is sent at once (`handler.ex:186-188`) | a stored result is sent without executing (`P:613-617`) | same (`J:156-162`) | acked, and the cached result sent again, executor not run | `a_duplicate_submit_is_acked_each_time_and_runs_once` |
| 49 | raise, exit or throw is a result with status 54 (`handler.ex:343-358`) | `on_submit_action` raising is a result with `code` 500, `message` `str(E)` (`P:619-623`) | `onActionSubmit` throwing is code 500 (`J:164-170`) | the handler side is 54 with the banner (`tests/hiro_crash.rs`). The 500 wrapper is an invoker-path difference: not handled here | `hiro_crash` tests |
| 50 | an in-band 429 is not handled; warning (`handler.ex:152-159`) | resends the stored result after a jittered `retryAfterMs`, retries kept (`P:643-644`, `667-688`) | not handled; `ActionMessageType.fromString(null)` throws (by reading) | decoded and ignored: no frame out, the 3000 ms resend neither stopped nor moved to `retryAfterMs` | `inband_429_for_a_pending_result_changes_nothing`, `inband_429_without_an_id_is_ignored` |
| 51 | a NACK cancels the resend (`handler.ex:112-119`) | retries while `retry_get` allows (4), resend after 1 s (`P:635-641`) | resends after 1 ms (`J:193-203`) | a NACK stops the resend, with no retry and no reply. A NACK 400 `"sendActionResult rejected"` is the one a Python or Java handler sends | `nack_400_sendactionresult_rejected_stops_the_resend_and_is_not_retried` |
| 52 | an ack cancels the resend (`handler.ex:109-110`) | removes the stored result (`P:630-633`) | removes it (`J:184-191`) | stops the resend; an ack for another id changes nothing | `python_style_ack_with_a_message_stops_the_resend`; `hiro_session` `acknowledged_frame_for_another_id_does_not_stop_the_resend` |
| 53 | a `sendActionResult` received: warning (`handler.ex:152-159`) | NACK 400 `"sendActionResult rejected"` (`P:625-628`) | same (`J:174-181`) | decoded and ignored, no NACK | `a_send_action_result_is_ignored_with_no_nack` |
| 54 | an unknown type: warning, no reply | NACK 400 with the id (`P:320-323`, `658-661`) | `WebSocketException` (`J:222-224`) | `WireError::UnknownType` to the caller, no frame sent; the caller may build a NACK from the id | `an_unknown_type_with_an_id_is_a_wire_error_and_gets_no_reply` |
| 55 | no `timeout`: 60000 ms (`handler.ex:133`, `:361-362`) | no `timeout`: `TypeError` in the parser, no NACK (by reading, `P:131`) | no `timeout`: 300000 ms (`ActionHandlerSubmit.java:34`) | 60000 ms. The Java javadoc frame (`"timeout": 30000`) gives 30000 ms | `submit_without_timeout_is_60000ms_not_javas_300000ms`, `java_javadoc_submit_is_acked_run_and_given_30000ms` |
| 56 | resend every 3000 ms for ever (`responder.ex:143-163`) | 4 retries (`P:358`) | 4 retries (`J:44`) | for ever | `the_result_is_resent_beyond_four_attempts` |
| 57 | exception in the handler: result 54 | exception in `on_message`: NACK 500 (`P:662-665`) | listener exception: result code 500 | handler side as 49; invoker path not handled here | n/a |
| 58 | the result is the JSON of the return value, unwrapped (`handler.ex:275-276`) | `{"message","code","data"}` (`P:769-780`) | code 200 or 204, message `"Action successful"` or `"Action successful (no data)"` (`J:324-335`) | sends the Elixir form, unwrapped. Reading a wrapped result is the invoker path: not handled here | n/a |

Further differences of section 8 of the oracle that reach the handler's inbound path:

| Difference | Elixir | Python / Java | What OGAR does | Test |
|---|---|---|---|---|
| in-band 503 with an id | not handled, warning | Python: unknown type, NACK 400; Java: throws (by reading) | decoded and ignored for an executing id and a pending result; no reply | `inband_503_with_an_id_is_ignored_and_not_nacked` |
| `configChanged`, typed `error` | not handled, warning | Python and Java call a listener or log | decoded and ignored | `config_changed_and_a_typed_error_are_ignored` |
| unknown fields in a frame | the keys it needs are matched, the rest ignored (`handler.ex:121-130`) | `dict.get` / Jackson | ignored, in a submit, an ack and a `hello` | `submit_with_unknown_fields_is_accepted_and_they_are_ignored`, `an_ack_with_unknown_fields_is_an_ack`, `hello_with_unknown_fields_is_accepted` |
| `parameters` values that are not strings | any term | `dict` (`P:101`), `Map<String, Object>` (`J:30`) | a string verbatim, any other value as compact JSON text | `submit_parameters_that_are_not_strings_are_kept_as_compact_json` |
| a NACK with neither `code` nor `message` | accepted: `type` and `id` are matched (`handler.ex:112-119`) | not sent by either client | accepted (`code` and `message` are optional in the codec); it stops the resend like any NACK. It used to be a `WireError` (a known gap, closed 2026-11-19). A NACK with no `id` is still a `WireError`: nothing is pending for it in HIRO either | `a_nack_without_code_or_message_stops_the_resend`, `a_nack_without_an_id_changes_nothing` |

Frame limit and the oracle. The oracle's assertion 10 names `GraphConn.max_frame_bytes` without its
default. The source (`connection_manager.ex:61`, `:201-221`; `README.md:174-182`) returns
1,000,000 when the `hello` advertised none or none was received, so a session fits at 1,000,000
bytes until a `hello` says otherwise; it does not skip fitting.

## 11. The invoker side (2026-11-19) — `invoke_state`, `invoke_session`, `discovery`

The handler side (sections 3 and 8) was the first half. This is the other half: the party that
submits an action and waits for the result, as graph_conn's `Invoker` does. All three modules
are sans-IO: time is an argument and the things to send or fetch are returned as values.

| module | covers |
|---|---|
| `invoke_state` | one invoker, many concurrent calls: ack wait 3000 ms and three sends, response wait = request timeout + 1000 ms with one last call, in-band 429/503 with and without an id, timeout validation before any send, NACK, result acked before delivery and decoded once, request ids shared by identical calls, late refusals (oracle assertions 20-41) |
| `invoke_session` | the text-frame adapter: `acknowledged`, `negativeAcknowledged`, `sendActionResult`, the typeless 429/503, `configChanged`; `submitAction` written with typed parameters |

The three codec gaps this section once listed are closed in `wire`: a `negativeAcknowledged` with no
`code` or `message` (`NackFrame`, both optional; the invoker completes with `{:nack, %{code: nil, message:
nil}}` as HIRO does), a `sendActionResult` with a missing or `null` `result` (`ResultFrame`, `None`; the caller
gets `nil`), and the in-band 503's whole `error` object (`UpstreamUnavailable::error`). `invoke_session` no
longer re-reads the raw text for any of them. A NACK, an ack or a result with no `id` is still a
`WireError` (HIRO acks a result with no id using `null`; this does not send that ack).
| `discovery` | the pure parsers of `GET /api/version` (malformed entries skipped), the auth answer and the path prefix; `capability_defaults`; and `FetchState`, the capabilities-then-applicabilities fetch with HIRO's backoff curve (1000 ms seed, cap 10000 ms, jitter, a 429's retry-after as the floor) and the refetch on `configChanged` |

`tests/invoke_tolerance.rs` pins what the invoker completes with when the handler is a Python or
a Java client: their `{code, message, data}` wrapper, including the 500 they send for a handler
exception, completes as a success, because only an `error` key is an error. The Rust invoker
keeps the Elixir behaviour.

**What it does not cover.** There is no HTTP client and no socket: the caller sends the
frames, makes the GETs, keeps the clock and applies the request timeout. Request ids are
FNV-1a 64 over a canonical encoding, not the MD5 of Erlang's term encoding that HIRO uses;
ids are opaque to the handler, so this does not matter on the wire, but an id is not
comparable to one HIRO computed. A 200 capabilities body that is not a JSON object is a
failed fetch here; HIRO accepts it and fails later. The real gateway's behaviour is
unverified (the oracle's section 9 applies: the reference is the mock). Choices where the
source is silent are listed in the module documentation of each.

## 12. The closed loop (2026-11-19) — `mock_gateway` and `tests/closed_loop.rs`

Sections 8 to 11 check each machine against the Elixir source on its own. This section joins them: the OGAR
invoker (`invoke_session`), a double of graph_conn's mock gateway (`mock_gateway`) and the OGAR handler
(`session`) run in one process, with a virtual clock, and are driven through whole scenarios. Everything between
the machines is a JSON text frame; the invoker and the handler never see each other's typed values.

**What is the double.** `mock_gateway::MockGateway` mirrors `mock_socket.ex` and `mock.ex`, each rule with its
source line in the module documentation. The rules that matter for a pair of peers:

| frame | the mock |
|---|---|
| a `submitAction`, unknown capability | NACK 404 `"capability X not found"` to the invoker, nothing forwarded (oracle 44, `mock_socket.ex:277-288`) |
| a `submitAction`, known capability | after a delay, `acknowledged` (`type` and `id` only) to the invoker, then the request to the handler (oracle 45, `:257-298`). The delay is a setting, 1 ms by default (the source's is a random 1 to 5 ms) |
| a handler's `acknowledged` | recorded, **not relayed** to the invoker (`:171-172`) |
| a handler's `sendActionResult` | the mock **acks it itself**, at once, and forwards it to the invoker once per registration of the id (`:220-246`) |
| an invoker's `acknowledged` of a result | recorded, **not relayed** to the handler (`:171-172`) |
| a handler's `negativeAcknowledged`, or any frame it has no clause for | a typed `error` 400 to the sender (`:248-255`) |

Three consequences the oracle does not state and the scenarios pin. The invoker's ack of a submit is the mock's,
so losing the handler's ack changes nothing. The handler's resend is stopped by the mock's ack, so losing the
invoker's ack of a result changes nothing either; to make the handler resend, the mock's ack to the handler has to
be lost. And the mock's registry is a `:duplicate` registry (`mock_server.ex:78`) that registers the invoker under
the id again for every known submit, so an invoker that resends a submit is sent the handler's result twice; it acks
both and completes once.

Fault injection (`Faults`, `inject_*`, `restart_handler`) is separate from the mirrored behaviour: drop, duplicate or
delay the next frames in a direction, inject an in-band 429 or 503 with, without or with a `null` id, restart the
handler connection. The armed refusals (`deny_next_request`, `fail_next_request_unavailable`,
`ignore_next_request`, `drop_next_submit`) are the mock's own arming functions (`mock.ex:303-350`).

**What the loop proves.** 25 scenarios in `tests/closed_loop.rs`, each with the final answer to the caller, the
number of runs of the capability, the frame trace with times, that all three machines end with nothing tracked once
HIRO's 60 minute cache has passed, and the handler's episode:

* the plain round trip; the gateway's ack lost (resend at 3000 ms, one run); the handler's result lost (resent at
  3000 ms, delivered once); the result ack lost either way; a duplicated submit while executing and after completion;
* unknown capability (NACK 404, the handler never involved); the mock's forbidden capability;
* an execution longer than the request timeout (status 13); nothing answering (three sends, `ack_timeout` 9000);
  acked and never answered (response wait timeout + 1000, one last call, `exec_timeout`);
* a failing and a panicking capability (status 54) and the repeat answered from the cache; a result over the
  announced `max_frame_bytes` (status 59);
* in-band 429 and 503 with an id (the caller completes at once; when the mock armed the refusal the handler never
  sees the request; when the refusal is injected after the request was forwarded the handler runs it once and its
  result goes to nobody) and without an id (nobody is answered);
* a handler restart (the fresh handler runs the same request again: count 2; parity, HIRO's cache does not survive a
  restart); two concurrent identical calls (one run, both complete);
* the registration path: the fetch machine reads the mock's two bodies and a capability default `timeout` of 60000
  reaches the wire as 60 000 000 ms, because the invoker reads it as seconds;
* a seeded random run (8 seeds, 24 calls each, drops, duplicates and delays in every direction, a handler restart in
  some): every call completes once with the result and no request ran more often than its restarts allow.

**What it cannot prove.** The double is graph_conn's mock, not a gateway. Nothing here says how the real gateway
orders frames, whether it relays the handler's ack, or whether it acks a result itself; if it does not, the handler's
resend loop (3000 ms until an ack) is the only thing that gets a lost result through. Time is virtual and frame
delivery is instant and ordered except where a fault says otherwise, so real races between a frame and a timer at
the same millisecond are not explored (HIRO says frame order is not guaranteed, oracle section 6). The loop is one
invoker and one handler; several of either are not modelled. The executor is a test double, so a long-running
capability's cost, a killed task and the handler's own supervisor are not exercised. The equivalence run against a
live gateway stays WP8.

## 13. Against HIRO's public documentation (2026-11-19)

Section 8 to 12 check the code against graph_conn. This section checks it against what HIRO
publishes: the 7.0 developer portal (`dev-portal.engine.datagroup.de/7.0/`). The decision
(`HIRO_PARITY_INTEGRATION_PLAN.md` section 3b) is that **graph_conn's behaviour is kept and the
documentation's differences are recorded as questions**. So the change is tolerance and
documentation: the docs' own samples are fed to the code in `tests/hiro_public_docs.rs`, and
code changed only where a sample was not accepted (item 1 only). The harvest and its diff are
`HIRO_PUBLIC_DOCS_HARVEST.md` and `HIRO_PUBLIC_DOCS_DIFF.md` (bardioc `docs/handover/`).

Public pages named below: **Action API in Hiro 7** (`/documentation/api/action-api/`, "AA"),
**List of Action Capabilities** (`/documentation/integration/Capabilities/capabilities_list/`,
"CAP"), **Action handler tutorial** (`/tutorials/tutorial-action-handler-action-api/`, "TUT-AH"),
**API changelog** (`/documentation/api/apichangelog/`, "CHG"), **Migration 6 to 7**
(`/documentation/migration_topics/migration_6_7/`, "MIG"). Quotes are the short samples of the harvest.

### 13.1 Item by item

| # | Docs say | graph_conn does | OGAR does | Test (`tests/hiro_public_docs.rs`) |
|---|---|---|---|---|
| 1 Capabilities, REST shape | AA: `GET capabilities` is `{"ExecuteCommand": {"description", "mandatoryParameters", "optionalParameters": {"timeout": {"default": "120"}}}}`. CAP writes the keys `ogit/Automation/mandatoryParameters`, `.../optionalParameters`, `.../resultParameters` | the unprefixed keys and a numeric default (`mock_test.exs:73-93`); no `description`, no result field | **code changed** (`registration.rs`). Before: the AA form parsed; the CAP form parsed to a capability with **no parameters**, because the unknown prefixed keys were dropped silently. Now the three prefixed keys are serde aliases of the three fields. Defaults are kept as text (`"120"` stays `"120"`). Unknown keys: the parser **ignores** them, neither keeping nor reporting them (unchanged; pinned). A body with both spellings of one key is a duplicate-field error | `get_capabilities_example_parses_with_its_string_default`, `capabilities_list_page_variant_with_ogit_automation_keys_parses`, `both_key_spellings_lift_to_the_same_typed_capability`, `prefixed_result_parameters_are_kept`, `an_unknown_key_in_a_capability_is_ignored_not_reported` |
| 2 Applicabilities | AA: handler id, capability, `"on ogit/machineClass == \"linux\" ssh_keyfile ssh_options"` (and `... ssh_keyfile not ssh_options`), then templates; a parameterless capability is a list `["on ogit/machineClass == \"windows\""]` | the same map form is the stored form (`mock.ex:129-141`); the list form is the **definition** input, converted to an empty map (`mock.ex:626-627`) | the map form parses; the `on` expression keeps the trailing words **verbatim** (no `not` is read, nothing is split off) and the templates are not expanded. The list form inside a response body is read too (since the follow-up to this pass): each `on <expr>` string is one entry with no parameters; a list element that is not such a string is refused by name | `applicabilities_example_keeps_the_on_expression_and_templates_verbatim`, `the_typed_entry_point_reads_the_docs_map_form`, `the_docs_list_form_inside_a_response_body_is_one_entry_per_on_string` |
| 3 `hello` | AA: `host`, `server_version`, `client_id`; no `max_frame_bytes`; "informational" | `max_frame_bytes` optional; with none, 1,000,000 (`connection_manager.ex:61`, `:201-221`) | already decoded (unknown fields are ignored); the session keeps 1,000,000 bytes, and a result frame over it becomes the status 59 error. Test only | `hello_without_max_frame_bytes_decodes`, `the_docs_hello_leaves_the_default_frame_limit_of_1000000_bytes` |
| 4 `submitAction` | AA: `handler`, `capability`, top-level `timeout` 300000, `parameters` object; the sentence under it says "handler field is not sent to the action handler". TUT-AH: also `scope`, timeout 60000000, `timeout` again in `parameters` | the handler matches `type`, `id`, `capability`, `parameters` and an optional `timeout`; the rest is ignored (`handler.ex:121-133`) | both decode, are acked, run once; the deadline is the top-level `timeout`. `scope` is **ignored**: dropped at decode, `SubmitAction::scope` is `None`, no error. `timeout` inside `parameters` is an ordinary parameter (text `"60000000"`). A submit without `handler` runs, `handler` empty. Test only | `the_docs_submit_runs_through_the_handler_session`, `the_tutorial_submit_with_scope_runs_and_scope_is_ignored`, `handler_field_is_not_sent_to_the_action_handler` |
| 5 `acknowledged` | AA: `type` and `id` only. TUT-AH quotes `code` 200 and `message` "" | an ack is matched on `type` and `id` (`handler.ex:109-110`) | both stop the result resend. Test only (the Java form was already pinned in `hiro_clients.rs`) | `the_docs_acknowledged_with_type_and_id_only_stops_the_resend`, `the_tutorials_acknowledged_with_code_and_message_stops_the_resend` |
| 6 `result` as an object, invoker | AA: `"result": {"resultfield1": ...}`, with `action_status` (0 = executed) and `action_error` recommended. TUT-AH sends a plain string | `_decode_result` JSON-decodes a binary once and returns **anything else as it is** (`invoker.ex:742-754`), so an object is accepted and is the value. Then only an `error` key makes it an error (`:723-735`). `action_status` and `action_error` are **read nowhere** in the invoker (they appear only in `handler.ex:335-339` and `result_frame.ex`, where the handler builds its own error results) | already the same: the object is kept as compact JSON text by the codec and decoded once, so the call completes `Ok(object)`. An object with `action_status` 54 is a value like one with 0; the caller reads `action_status` from it, intact. It is **not** an `InvokeError`, as in graph_conn. An object with an `error` key stays an error. Test only | `the_docs_result_object_completes_the_call_with_the_object`, `an_object_with_a_nonzero_action_status_is_distinguishable_to_the_caller`, `the_object_form_and_the_json_string_form_give_the_same_value`, `an_object_with_an_error_key_is_still_an_error` |
| 7 NACK 404 | AA: "code 404 should be returned from the action handler if it doesn't support requested capability" | the handler answers an unknown capability with a **result** `{"error":{"code":404,...}}` (`handler.ex:369-371`). The invoker ends the call on a NACK whatever the code (`invoker.ex:659-664`), but only while it waits for the ack | invoker: the terminal NACK it already has (code and message to the caller, no resend, a 404 not logged). A NACK that arrives **after** the ack is dropped, as in graph_conn. Handler: unchanged, a 404 result; the off-by-default option is **not built** (13.4). Test only | `code_404_should_be_returned_from_the_action_handler_if_it_doesnt_support_requested_capability`, `a_nack_after_the_ack_does_not_end_the_call_as_in_graph_conn`, `an_unsupported_capability_on_the_handler_is_a_404_result_not_a_nack` |
| 8 Status codes | AA lists 0, 11, 12, 13, 14, 51, 52, 53, 54, 666. 13 is "ActionHandler didn't respond", 14 "ActionHandler responded with execution timeout" | the handler sends 13 for its own execution deadline, 54 for a crash, 59 for an oversized result; never 14 | **code added, no behaviour change:** `ogar_action_handler::status`, a table of the documented codes with the docs' short meanings and the code only graph_conn has (59). The doc comments of 13 and 14 state the disagreement. The session sends the same codes as before | `the_documented_status_table_is_the_docs_list_and_59_is_not_in_it`, `the_codes_the_session_sends_are_the_same_as_before` |
| 9 Connection constants | AA: `wss://core.almato.ai/api/action-ws/1.0/`, subprotocols `['action-1.0.0', 'token-' + token]`. MIG and TUT-AH write `/api/action-ws/1/`; TUT-AH writes `'1.0, token-' + ws_token` in its last sample. CHG: "all api version 0.9 calls" removed in action-ws `[1.0.0]` | the Elixir client reads path and subprotocol from `GET /api/version` (`discovery.rs`); its mock gives `/api/0.9/action-ws/` and `action-0.9.0` | see 13.2. **Constants added** beside the existing ones, nothing switched | `ogar-from-schema` unit test `the_public_docs_connection_values_sit_beside_the_existing_ones` |

### 13.2 Connection constants and every 0.9 value (item 9)

What OGAR has, in `ogar-from-schema/src/action_ws.rs`: `ACTION_WS_PATH = "/api/action-ws/1.0/connect"`
and `auth_subprotocol(token) = "token-<token>"`. So OGAR does **not** hard-code 0.9 in code that runs
against a gateway; it already says 1.0. Two differences from the docs, neither changed:

* the path has a `connect` segment the public docs do not show (`/api/action-ws/1.0/`);
* only the token subprotocol is offered; the docs' first value `action-1.0.0` is not.

Added, additive, not used by any code path: `ACTION_WS_DOCS_PATH` (`/api/action-ws/1.0/`),
`ACTION_WS_SUBPROTOCOL` (`action-1.0.0`) and `docs_subprotocols(token)` (both values, in the docs'
order). The owner decides whether to switch.

Every place a **0.9** value appears (none outside the HIRO mock's fixtures and examples):

| Where | What |
|---|---|
| `crates/ogar-action-handler/src/discovery.rs:76,78,80` | doc comments giving `/api/0.9/action/`, `action-0.9.0`, `0.9` as examples of what `GET /api/version` returns |
| `.../src/discovery.rs:225-226,708,710` | doc comment and doctest of `prefix_path` / `FetchState::new("/api/0.9/action/", ...)`: the prefix is an argument, taken from the server's `/api/version` answer, not a constant |
| `.../src/mock_gateway.rs:556,562` | doc comments naming the mock's `GET /api/0.9/action/capabilities` and `.../applicabilities` (graph_conn's mock; stays as it is) |
| `.../tests/discovery.rs` (18 places), `.../tests/closed_loop.rs` (3) | fixtures copied from graph_conn's mock `/api/version` answer and the fetch paths built from it |

The REST registration docs in `registration.rs` (`/api/action/1.0/`) and section 2a of this
document say 1.0; MIG writes `/api/action/1/`. Which of `1` and `1.0` the live gateway takes is
not in the docs.

### 13.3 What the public docs do not cover

Nothing below is in the pages read; OGAR keeps graph_conn's value or behaviour for each.

* a frame size limit, and an `action_status` for one (59); `hello` carries no `max_frame_bytes`;
* the default timeout of a submit (all fields are said to be mandatory), and one unit for capability
  `timeout` defaults (`"120"` is seconds for `ExecuteCommand`; others are 300000, 10000 or 3000
  without a unit);
* the ack wait, the resend interval of a result, a retry cap, the Action API's give-up rule ("each
  couple of seconds" only), the 3000 ms and 60 minute values;
* what a handler does on a NACK of its result;
* NACK codes other than 404; the fields of the `configChanged` and `error` push messages (named in CHG);
* in-band 429 and 503 frames; typeless `error` frames;
* what the gateway does with the token;
* who expands the `${...}` templates and when; a grammar for the `on` expression, the meaning of
  the words after the comparison, and whether `not` is a presence test;
* whether the live REST endpoint serves the plain or the `ogit/Automation/` capability keys;
* the order of frames, a dedupe lifetime, relaying of acks by the gateway.

### 13.4 Open questions and the items stopped

1. **The list form in a response body (item 2): done.** The docs' applicabilities example has
   `"ParameterlessExecute": ["on ..."]` inside the response; graph_conn has that form only as an
   input definition. It is now read in a response as well: one entry per `on ...` string, no
   parameters. The assertion in `tests/hiro_registration.rs` that refused `{"h":{"C":["on x"]}}`
   was changed deliberately; a list holding anything but `on ...` strings is still refused.
2. **Handler NACK 404 option (item 7, not built).** `unknown_capability: Result404 | Nack404` would
   need a decision before the ack, but the state machine does not know the served capabilities (only
   `Session::on_text_sync`, which takes the `ActionDef`, does). It would need a field on `Session` or
   `Config` (`Config` is a public struct without `#[non_exhaustive]`, so a new field is an API
   break for exhaustive literals), a decode of the submit ahead of `step` in `on_text_sync`, and a
   rule for what a NACKed id leaves in the dedupe cache. More than a few lines, and it touches the
   default path, so it is not built.
3. Is the docs' 13 ("ActionHandler didn't respond") the Action API's own code, as the text reads, while
   graph_conn's handler uses 13 for its execution deadline? Should the handler send 14?
4. In the docs' chain the handler's NACK can follow the gateway's ack of the request; graph_conn's
   invoker only takes a NACK before the ack. Which one does the real gateway send, and when?
5. Is a `sendActionResult.result` an object or a JSON string on the real gateway? Both are read.
6. `/api/action-ws/1.0/` with or without `connect`; `1.0` or `1`; which subprotocol value or values.
7. The `ogit/Automation/` capability keys: does the live endpoint serve them?
8. The docs say `handler` is not sent to the handler yet show it in the sample; the code reads it if
   present and does not need it.

## 14. The gateway's published specification (2026-11-19)

The published specification and the catalog are the reference for the documented contract. graph_conn is the witness of what a client does. The YAML describes the action service (aapi) as deployed. It lists its own departures from an older graph specification, and that older specification was not read.

The files are in `crates/ogar-action-handler/tests/fixtures/hiro_public_specs/`, unchanged. `tests/hiro_published_spec.rs` checks the crate against them. Close codes 1000, 1001 and 1009 belong to the transport, not to this crate.

| Point | YAML lines | Result |
|---|---|---|
| Handler ack has `type`, `id`; sends `code`, `message` too | 373-382 | more lenient (extras pinned) |
| Handler `sendActionResult`, `result` is a string | 277-281, 358-369 | agrees |
| `clientHello` fields, `settings.redelivery` carried or omitted | 286-294, 403-424 | agrees |
| Invoker `submitAction` has the inbound fields, `timeout` in ms | 282-285, 321-338 | agrees |
| Invoker result ack: `type`, `id`; sends `code` too | 373-382 | more lenient (extra pinned) |
| Encoded NACK has `type`, `id`, `code`, `message` | 384-401 | agrees |
| Outbound `submitAction` without `handler` is claimed, acked, run once | 341-356 | agrees |
| Submit without `timeout` is accepted (schema requires it) | 343 | more lenient |
| `hello` with five fields sets the frame limit | 295-318 | agrees |
| `hello` without `max_frame_bytes` means 1,000,000 | 297, 316-319 | agrees |
| `acknowledged` with `type`, `id` only, to handler and invoker | 373-382 | agrees |
| `negativeAcknowledged` with `code`, `message`, to handler and invoker | 384-401 | agrees |
| `timeout: 30000` is a 30,000 ms deadline | 282-285 | agrees |
| Post-ack re-delivery answered from the stored result | 290-293 (opt-out only) | not specified by the YAML; our behaviour, as graph_conn does |
| `setSettings` (deprecated, client to aapi) is not sent | 243-258 | agrees (never sent) |
| Mock `hello` has no `host`, `server_version`, `client_id` | 297 | gap of the test double |
| Endpoint and subprotocol constants equal the catalog | catalog | agrees |
| Capabilities and applicabilities keyed by real names | `action.yaml` 113, 126 | agrees (the YAML types only the placeholder keys) |
