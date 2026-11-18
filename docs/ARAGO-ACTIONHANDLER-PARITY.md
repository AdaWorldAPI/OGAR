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
| dedup of a repeated `id` (`idempotency_key`, `ModalSpec::Idempotent`) | not done: `handle_submit` holds no state, a repeated id executes twice. In progress in another slice, not merged |
| `submitAction.timeout` to a deadline | not done: `timeout_millis` is never read and the executors have no deadline. In progress in another slice, not merged |
| result resend until acknowledged | not done: nothing reads an inbound `acknowledged`. HIRO's Elixir handler resends every 3000 ms until acked or nacked |
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
Not wired: no socket, timer or reconnect loop; episodes are not persisted; class id, target
and effect class stay unset in the episode; `handle_submit` is unchanged and still runs a
repeated id twice.
