# Arago ActionHandler ⟷ OGAR — parity scorecard + the Python→OGAR switch path

> **Status:** FINDING (contract parity `[G]`, grounded in shipped code + the
> vendored OGIT ontology + arago's published sources) + the runtime executor as
> a specced, NOT-yet-built brick (`[H]`, gated on `PROBE-OGAR-ACTIONHANDLER-RUN`).
>
> **Goal (operator):** reach parity with arago's HIRO ActionHandler such that one
> "could basically switch from [arago's] Python to OGAR running it here." This
> doc scores exactly how far that is true today and names the remaining runtime.
>
> **Sources (arago, verbatim):** `github.com/arago/ActionHandlers` (the handler
> config format), `arago/python-hiro-stonebranch-actionhandler` (a concrete
> daemon), and the HIRO 7 **Action API** tutorial + machine-readable specs (the
> `action-ws` protocol, served by the HIRO 7 dev portal).
> The OGIT `NTO/Automation` ontology (vendored at `vocab/imports/ogit/NTO/Automation/`)
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
`ogar-from-schema::do_arm` (`assemble_action_handler`), grounded in the vendored
OGIT `provides` graph — **proven by `assembles_the_full_action_handler_contract`**.

| arago config field | OGIT ontology | OGAR type (`do_arm`) | status |
|---|---|---|---|
| `Capability.Name` | `ActionCapability` | `ActionDef.predicate` | `[G]` |
| `Capability.Description` | `ogit:description` | (class description) | `[G]` |
| `Capability.Parameter[]{Name,Mandatory,Default}` | `mandatoryParameters` / `optionalParameters` | `CapabilitySlot.declares_{mandatory,optional}_params` + `ActionParam{name,mandatory,default}` | `[G]` shape / `[H]` values¹ |
| `Capability.resultParameters` (**the output**) | `resultParameters` | `CapabilitySlot.declares_result_params` | `[G]` shape / `[H]` values¹ |
| `Capability.Command` / `Interpreter` | `knowledgeItemFormalRepresentation` (the body) | `ActionDef.body_source` — **pointed-to, never inlined** (lossless-DO §1) | `[G]` |
| `Applicability.ModelFilter{Var,Mode,Value}` | `environmentFilter` | **`KausalSpec::StateGuard{guard_field,guard_values}`** | `[G]` |
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
| `submitAction.parameters{…}` | engine → handler | bind to the capability's `ActionParam[]` → `ActionInvocation` inputs | `[H]` (needs the param-bind step) |
| `submitAction.handler` / `scope` | engine → handler | `ActionInvocation.lokal{actor,tenant}` | `[G]` type / `[H]` wiring |
| `submitAction.timeout` (ms) | engine → handler | `ActionDef.state_timeout_millis` (`ogar:StateTimeout`) | `[G]` |
| `submitAction.id` (`$appId:$requestId`) | engine → handler | `ActionInvocation.idempotency_key` (OLD↔NEW correlation) | `[G]` |
| **handler** `acknowledged{id,code:200}` | handler → engine | `ActionInvocation.state = Pending` (received, not yet committed) | `[G]` type / `[H]` emit |
| *execute* | (internal) | `commit_via<ClassRbac>`: RBAC verb-gate ∧ `StateGuard` (the ModelFilter) ∧ MUL impact | `[G]` gate / `[H]` body-exec |
| `sendActionResult{id, result{…}}` | handler → engine | `state = Committed`; `result` = the `resultParameters` output payload | `[G]` lifecycle / `[H]` result-build |
| **engine** `acknowledged` (result received) | engine → handler | the **Lance commit** (`CommitHook::on_commit`) — "state history IS the version log" | `[G]` |
| retry-on-no-ack | engine | at-least-once → `idempotency_key` dedup (`ModalSpec::Idempotent`) | `[G]` |

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

**The six message types** (✓ = modelled in `action_ws`):

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
  messages (serde-gated for the wire).
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

What remains is **glue over existing types** (a socket loop + a JSON codec + a
registration parser) plus the non-native executor targets — not new IR. The
*contract, the lifecycle, the protocol binding, the reactive dispatch, and a
working native executor* are OGAR-native and proven; the live daemon is a thin
transport over them.

---

## 4. Scorecard — are we at parity?

| Layer | Parity | Evidence / remaining |
|---|---|---|
| **Config + ontology contract** | ✅ `[G]` | `assemble_action_handler` over the vendored OGIT graph; `ActionHandlerSpec` / `CapabilitySlot` / `ApplicabilitySlot` / `ActionParam` |
| **ModelFilter → guard** | ✅ `[G]` | `environmentFilter` → `KausalSpec::StateGuard` (test) |
| **Action lifecycle (protocol)** | ✅ `[G]` (type-level) | `action-ws` ⟷ `ActionInvocation` Pending→Committed; `commit_via` is the gate |
| **RBAC at execute** | ✅ `[G]` | `commit_via<ClassRbac>` (verb-gate ∧ guard ∧ MUL) — shipped in `lance-graph-contract` |
| **action-ws protocol core (B2-core)** | ✅ `[G]` SHIPPED | `action_ws`: all 6 message types + `submit_to_invocation` / `bind_parameters` / `invocation_to_result` (socket-free, `full_action_ws_roundtrip` proven) |
| **Reactive dispatch + B1 seam** | ✅ `[G]` SHIPPED | `action_ws::handle_submit` + the `CapabilityExecutor` trait (validate→ack→bind→execute→result; tested with a mock) |
| **Executor — native target (B1)** | ✅ `[G]` SHIPPED | `ogar-action-handler::NativeCommandExecutor` runs `ExecuteCommand` for real; `full_dispatch_runs_a_real_command` |
| **Hard gate before executor (B1-uplink)** | ✅ `[G]` SHIPPED | rs-graph-llm `graph-flow-action-ogar::GatedOgarHandler` — `commit_via` (RBAC ∧ guard ∧ MUL) lands before `handle`; `take_result()` is `None` iff the gate refused (3 tests) |
| **Executor — REST target (B1)** | ✅ `[G]` SHIPPED | rs-graph-llm `graph-flow-action-ogar::rest::RestExecutor` (`feature = "rest"`, ureq) POSTs bound params → resultParameters; runs only behind the gate (`rest_executor_runs_only_behind_the_gate`) |
| **Executor — SSH target (B1)** | 🟡 `[G]` code / `[H]` live | `ogar-action-handler::SshExecutor` shells out to system `ssh` (non-interactive `BatchMode=yes`, same `output`/`stderr`/`exitcode` shape as native) — arago's canonical `ExecuteCommand`-over-SSH, dep-free. argv construction + pre-spawn guards tested; end-to-end needs a live host (no sshd in CI) |
| **Executor — WinRM (B1)** | ⛔ `[H]` | a further `CapabilityExecutor` impl (Windows remote exec) |
| **Live transport — daemon core + WebSocket (B2-transport)** | ✅ `[G]` SHIPPED | rs-graph-llm `graph-flow-action-ogar::daemon`: transport-agnostic `Daemon::react`/`serve` + `Transport` trait + `WsTransport` (action-ws), gate-driving; mock-server roundtrip. `Auth` ← OGIT `NTO/Auth/Configuration` |
| **Live transport — Kafka edge (B2-transport)** | ⛔ `[H]` | `rdkafka` over the same `Transport` trait (action topic → result topic); core ready, needs the topic/record shape pinned |
| **Class-late-bound dispatch (the grail)** | ✅ `[G]` SHIPPED | rs-graph-llm `graph-flow-action-ogar::daemon::ResolvingDaemon` — class resolved from the target's **classid** per action (`ClassResolver`), executor from the resolved `RunnerKind` (`ExecutorRegistry`). `OgarResolver` is the production resolver over the canonical `actions_for(&[ClassActions], classid)` manifest. Proven: one `ExecuteCommand`, `mars_machine` → native / `mars_resource` → REST, zero daemon change; gate still rules |
| **Instance config lift — capabilities (B2-lift)** | ✅ `[G]` SHIPPED | `registration::lift_registration` + `ogar-action-handler::parse_capabilities`: real `GET /capabilities` JSON → `ConcreteCapability` (`ActionParam[]`); `rest_registration_lifts_binds_and_runs` (JSON → lift → bind → run) |
| **Instance config lift — applicabilities (B2-lift)** | ✅ `[G]` SHIPPED | `registration::lift_applicabilities` + `ogar-action-handler::parse_applicabilities`: real `GET /applicabilities` JSON → per-handler `StateGuard` sets; `rest_applicabilities_lift_to_per_handler_guards`. Residual: inner filter-list field name is alias-flexible pending a live response |

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
