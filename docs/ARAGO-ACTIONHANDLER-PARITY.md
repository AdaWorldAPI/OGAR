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
  here," native). SSH / REST / WinRM targets follow the same trait; rs-graph-llm's
  `graph-flow-action` provides the production executors (and runs `commit_via`).
- **B1-uplink — the hard gate before the executor (SHIPPED).** rs-graph-llm's
  `graph-flow-action-ogar` crate is the seam: `GatedOgarHandler` wraps an OGAR
  `CapabilityExecutor` as a `graph-flow-action::ActionHandler`, so the executor's
  `handle` runs **only after** `dispatch_via`'s cold floor commits
  (`commit_via`: def-match → RBAC `ClassRbac` → state-guard → MUL). The structural
  proof: `take_result()` is `None` whenever the gate refused — `run_gated` with an
  unauthorized actor (`Denied`) or a MUL `Block` (`Escalated`) never reaches the
  OGAR executor. Three tests pin it; `NativeCommandExecutor` runs the real command
  only on the authorized path. OGAR owns the executor; rs-graph-llm owns the gate.
- **B2-transport — the live WebSocket loop.** Wrap `handle_submit` in a
  `tokio-tungstenite` client (connect with the `token-$TOKEN` subprotocol, JSON-
  codec the six `action_ws` message types, drive the dispatch, retry-on-no-ack).
  All the message types, connection path, and auth are now pinned (§2a).
- **B2-lift — the instance config lift.** Parse a deployed handler's REST
  registration (`GET /capabilities`, `/applicabilities`) → the concrete
  `ActionDef` + `ActionParam[]`, reusing `assemble_action_handler`'s shape.

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
| **Executor — SSH/REST/WinRM (B1)** | ⛔ `[H]` | further `CapabilityExecutor` impls (rs-graph-llm `graph-flow-action`) |
| **Live WebSocket transport (B2-transport)** | ⛔ `[H]` | wrap `handle_submit` in a `tokio-tungstenite` loop + JSON codec (all shapes pinned, §2a) |
| **Instance config lift (B2-lift)** | ⛔ `[H]` | REST `GET /capabilities`/`/applicabilities` → `ActionDef`/`ActionParam` |

**Verdict:** OGAR is at **full contract + lifecycle + protocol-binding +
reactive-dispatch parity**, and **a working native executor runs real commands
end-to-end** (`handle_submit` + `NativeCommandExecutor`). Every field of the
config, ontology, and protocol has an OGAR type; the gate (`commit_via`), the
binding, the dispatch, and native execution are real and tested — and the gate is
now **wired to the executor**: rs-graph-llm's `graph-flow-action-ogar` runs OGAR's
`CapabilityExecutor` only after `commit_via` commits, so an unauthorized or
MUL-blocked action never executes (proven structurally — `take_result()` is
`None`). What's left for
a **live** drop-in replacement of arago's Python daemon: **B2-transport** (the
WebSocket loop — all shapes/auth pinned), **B2-lift** (the REST registration
parse), and the **non-native executor targets** (SSH/REST). Each is
transport/parser/runner glue over existing types — **no missing IR, no missing
protocol mapping**. That is the honest state: OGAR *is* an ActionHandler that
runs commands here; a thin transport away from connecting to a live HIRO engine.

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
- `docs/HIRO-DO-ARM-LIFT.md` — the lossless-DO rule (the body is pointed-to).
- `docs/ACTIONHANDLER-TURSTEHER.md` — RBAC-as-`const`, the cold-path gate, Rung.
- `lance-graph-contract::action` — `ActionInvocation` / `commit_via<ClassRbac>`.
- `lance-graph-ogar::OgarActionProvider` — the `classid → ClassActions` DO surface.
- `rs-graph-llm/graph-flow-action` — the `ActionHandler` executor trait (B1 home).
- `rs-graph-llm/graph-flow-action-ogar` — the **uplink**: OGAR's
  `CapabilityExecutor` behind the hard gate (`GatedOgarHandler` / `run_gated`);
  `commit_via` lands before any execution.
- arago: `github.com/arago/ActionHandlers`,
  `arago/python-hiro-stonebranch-actionhandler`, HIRO 7 Action API tutorial.
- **HIRO 7 Action API machine-readable specs (the authoritative harvest, §2a):**
  `action-ws.yaml` (the WebSocket message contract), `action.yaml` (the REST
  registration API), `auth.yaml` (the token endpoint) — served by the HIRO 7
  dev portal (`/help/specs/`, indexed under `/7.0/api/`).
