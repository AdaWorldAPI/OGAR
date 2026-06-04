# Temporal Time-Travel & the Version-Watcher Boundary

> **Purpose.** Two things landed on OGAR's side from the parallel
> sessions: (1) decision #3 (the Lance-subscription bus) **shipped** as
> `LanceVersionWatcher` + `std::sync::Condvar`, which unblocks AND
> corrects Sprint 7; (2) a temporal-epistemology framework that the
> other session correctly placed at the **planner query layer** (their
> layer, not OGAR's). This doc records the corrected boundary and the
> one OGAR-IR alignment that surfaces — without building either side
> (both sessions are holding until signal).
>
> Status: **CARVED v0** (2026-06-04). No code — boundary + alignment record.

## 1. Decision #3 shipped: `LanceVersionWatcher` (std::sync, not tokio)

The "Lance-subscription bus" OGAR's Sprint 7 was blocked on is shipped:
`lance-graph-callcenter::version_watcher::LanceVersionWatcher`. The
contract:

```text
hot path (NO tokio — I-2 invariant):
  lance-graph-planner consumes via LanceVersionWatcher::subscribe()
    → WatchReceiver
    → wait_changed()   parks on std::sync::Condvar
    → current()        returns Arc<CognitiveEventRow>  (Arrow-scalar; BBB invariant)
```

**I-2 invariant (upstream, load-bearing):** tokio is reserved for
Layer-3 outbound sinks (`PhoenixServer`, `PostgRestHandler`). The hot
loop uses `std::sync::{Arc, RwLock, Mutex, Condvar}` and **never**
`tokio::sync`. (History: earlier iterations used
`tokio::sync::watch::Sender/Receiver`; migrated per the
supabase-subscriber-v1 plan correction, 2026-05-06.)

### 1.1 This corrects OGAR's Sprint 7 design

OGAR's `SOA-IMPLEMENTATION.md` §5.2 sketched `KanbanMailbox<M>` on
`tokio::sync::mpsc::Sender` + `tokio::sync::watch::Sender`. **That
violates I-2** and is superseded. Corrected:

- The hot-path subscriber wraps `LanceVersionWatcher::subscribe()` →
  `WatchReceiver`; the wait is `wait_changed()` parking on
  `std::sync::Condvar`. No tokio.
- `current()` hands back `Arc<CognitiveEventRow>` (Arrow-scalar) — OGAR
  reads it, never copies the row out.
- The Kanban WIP/pull/backpressure **policy** stays; the **mechanism**
  becomes `std::sync` (`Mutex<VecDeque<_>> + Condvar`) on the hot path.
- Ractor/tokio survive only on the **SLA-coordination / cold path**
  (Layer-3-ish), never the hot loop.

### 1.2 SoA bridge ownership (so OGAR doesn't rebuild it)

```
lance-graph-ontology   owns  identity register + classes + codebooks
lance-graph-callcenter owns  LanceMembrane (SOLE writer) + watcher + CognitiveEventRow
ogar-runtime (OGAR)    owns  the std::sync SUBSCRIBER that reacts to ticks
                             (cache-invalidate + WIP pull); NOT a writer
```

**Carve-out:** OGAR's `ogar-runtime` is a **subscriber + reactor**, not
a writer. The sole writer is `LanceMembrane` (callcenter). OGAR never
writes the Lance dataset directly; it emits `MappingProposal`s (Sprint
5) and reacts to version ticks (Sprint 7). This keeps the
determinism-firewall intact: one writer, many subscribers.

## 2. Temporal-epistemology is a PLANNER-layer query annotation (not OGAR)

The parallel session mapped the Python temporal-epistemology framework
(`epistemology.py` / `detector.py` / `awareness.py` / `hydration.py`)
onto Lance versions, and correctly concluded it is **query-level
annotation, not storage**:

| Python framework concept        | Lance / standing-wave equivalent                       |
|---------------------------------|--------------------------------------------------------|
| `KnowledgeItem.created_at`      | Lance version V at which the row landed                 |
| `KnowledgeItem.knowable_from`   | Lance version V at which the row's class was registered |
| `KnowledgeHorizon` (at time T)  | `dataset.checkout_version(V_ref)` — pinned snapshot     |
| `TemporalStatus.CONTEMPORARY`   | `row.lance_version ≤ V_ref`                             |
| `TemporalStatus.ANACHRONISTIC`  | `row.lance_version > V_ref` (visible only if planner allows) |
| `TemporalStatus.SPOILER`        | intentional `V_now` read with `V_ref < V_now`           |
| `EpistemicMode.STRICT/AWARE/RETRO` | planner-level query annotation                       |
| `EpistemicPolicy.for_rung(N)`   | which `ThinkingStyle` opts into which mode              |
| `CausalChain` (depends_on/enables) | standing-wave tier transitions across versions       |

**What it adds (their layer):** a
`QueryReference { ref_version: u64, mode: STRICT|AWARE|RETRO, rung: u8 }`
annotation on `lance-graph-planner` queries. At read time the planner
reads at `V_now` but filters/tags rows whose `lance_version > ref_version`
per the mode. **No new storage, no new contract, no new container.**

**Cross-server hindsight (GPS-relativity / tick-based deinterlace):**
each substrate instance has its own Lance version sequence (its frame
of reference); a hybrid logical clock (HLC) per writer stamps
`(server_id, local_lance_version, hlc_tick)` on the `CognitiveEventRow`;
multi-server reads sort by `hlc_tick` for a deterministic causal-time
ordering; the planner asks "as of HLC tick T_ref" instead of "as of
local version V_ref." Determinism firewall preserved (each server only
reads others' versions, never cross-writes).

**Carve-out:** OGAR does **NOT** build `QueryReference` / `EpistemicMode`
/ the planner filter (that's `lance-graph-planner`) nor the HLC stamp
on `CognitiveEventRow` (that's `lance-graph-callcenter::LanceMembrane`).
Both are other sessions' layers. OGAR consumes the result.

## 3. The one OGAR-IR alignment that surfaces

OGAR's `ActionInvocation` (Sprint 3) already carries the IR-level
provenance the planner/callcenter time-travel reads over:

| OGAR `ActionInvocation` field | Temporal-epistemology / HLC role                       |
|-------------------------------|--------------------------------------------------------|
| `emitted_at_millis: Option<i64>` | the timestamp the planner orders by — see alignment ⚠ |
| `parent_invocation: Option<String>` | `CausalAnnotation` causal parent (`depends_on`)     |
| `trace_id: Option<String>`    | cross-actor correlation (cross-server trace)            |
| `state: ActionState`          | Pending/Committed/Failed lifecycle (admissibility)      |

Plus OGAR's identity `@v<n>` pinning (`class_identity_versioned`,
Sprint 1) is the IR-level expression of `KnowledgeHorizon` /
`checkout_version`.

**⚠ Alignment to surface (NOT building yet):**
`ActionInvocation.emitted_at_millis` is a **plain wall-clock `i64`**.
The cross-server model needs a **hybrid logical clock**
`(server_id, local_lance_version, hlc_tick)` — wall-clock is **not**
causally ordered across servers; HLC is. So if cross-server hindsight
becomes a real workload, OGAR's `emitted_at_millis` should align to (or
coexist with) an HLC tick rather than wall-clock millis. This is a 4th
surfaced coordination item:

> **Decision #4 (surfaced, not blocking):** does
> `ActionInvocation.emitted_at` stay wall-clock `i64`, or become an HLC
> tuple to match the `CognitiveEventRow` HLC the callcenter stamps? Only
> matters once cross-server hindsight is a real workload. Until then,
> wall-clock `i64` is fine (single-server causal order is the Lance
> version sequence itself).

**Carve-out:** OGAR's job is to make `ActionInvocation` provenance
*alignable* — keep `emitted_at` an `Option` so an HLC variant can be
added non-breakingly (the `#[non_exhaustive]` struct already allows it).
OGAR does NOT define the HLC type (that's the callcenter's
`CognitiveEventRow`); it conforms to it when the workload lands.

## 4. Net effect on OGAR sprints

| Sprint | Effect |
|---|---|
| 7 (`ogar-runtime`) | **UNBLOCKED** (decision #3 shipped) + **CORRECTED**: std::sync `Condvar` subscriber over `LanceVersionWatcher`, NOT tokio/Ractor on the hot path. Consume `lance-graph-callcenter::version_watcher` as the reference impl. Still gated only on the cross-repo build (protoc). |
| 3 (`ActionInvocation`) | No change now. `emitted_at_millis` stays wall-clock; keep it `Option` so an HLC variant is a non-breaking add (decision #4). |
| 6 (cache invalidation) | The version-tick → cache-invalidate reaction IS the `LanceVersionWatcher` subscription. OGAR owns the integration test that a tick fires the reaction. |
| — (epistemology query layer) | NOT OGAR. `lance-graph-planner` owns `QueryReference`/`EpistemicMode`; `lance-graph-callcenter` owns the HLC stamp. OGAR consumes. |

## 5. Holding pattern (matching the other session)

Both sessions: **FYI absorbed; not building yet.** The standing-wave
model meets the bar without the epistemic layer; it's additive once the
cross-server scenario shows up. OGAR's posture is identical:
- Sprint 7 is now unblocked-and-corrected, but **building waits for the
  signal** (and the cross-repo protoc build).
- Decision #4 (emitted_at → HLC) is surfaced, not actioned.
- OGAR does not build the planner/callcenter epistemology layers.

## 6. Cross-references

- `docs/SOA-IMPLEMENTATION.md` §5 — the corrected runtime layer (the
  tokio sketch there is superseded by §1.1 here).
- `docs/LANCE-GRAPH-INTEGRATION.md` §10.3 — the "CI = lance-update →
  kanban subscription" metaphor; this doc is its concrete impl.
- `.claude/PLAN.md` Sprint 7 — unblocked + corrected.
- `.claude/board/EPIPHANIES.md` — the decision-#3-shipped +
  decision-#4-surfaced entries.
- Other session: `STANDING_WAVE_ARCHITECTURE.md` §13 (planned) — the
  planner-layer epistemic-policy mapping table + HLC sketch (their
  canonical placement; OGAR references, does not duplicate).
- Upstream: `lance-graph-callcenter::version_watcher::LanceVersionWatcher`,
  `lance-graph-callcenter::LanceMembrane`, `CognitiveEventRow`.
