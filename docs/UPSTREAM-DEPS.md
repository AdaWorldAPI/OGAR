# OGAR Upstream Dependencies

> **Purpose.** Single source of truth for which upstream APIs OGAR
> depends on, at which stability tier, and when each Sprint binds to
> them. Maintained as the surface of truth across the AdaWorldAPI
> ecosystem.
>
> Companion to `IDENTITY-MAPPING.md`, `ODOO-TRANSCODING.md`,
> `ADAPTERS-AND-ACTORS.md`, `SOA-IMPLEMENTATION.md`. Where those
> carve internal vocabulary, this doc carves external **gravity
> wells** — APIs we will not break free of without significant
> re-engineering.

## 1. AdaWorldAPI/lance-graph

**Repo**: <https://github.com/AdaWorldAPI/lance-graph>
**OGAR uses**: NiblePath identity dictionary, Lance dataset versioning, append-only triple storage.

| Sprint | API surface OGAR binds to | Stability tier |
|---|---|---|
| 5 (`lance-graph-contract` SoA integration) | `NiblePath` segment encoding (27-bit identity per segment), `LanceDataset::append`, `versions()` watcher | LOCKED — breaking changes coordinated |
| 6 (`lance-graph-ontology` cache) | manifest-watch event API for cache invalidation | LOCKED |
| 7 (`lance-graph-callcenter`) | dataset transactional commit boundary; tag-based version retention | LOCKED |

**Coordination notes**:
- OGAR's `Identity::class_identity_versioned(prefix, class, n)` MUST produce a string that `NiblePath` can split losslessly.
- OGAR plans **per-stream optimistic versioning** (per R5 — EventStoreDB pattern) at Sprint 5; needs `dataset.commit_at_expected_version(n)` semantics on lance-graph side. Either it exists or OGAR layers it.
- OGAR expects `v2 manifest paths` from day-one (per R2 gotcha #1). lance-graph's PR #453 confirmed cluster-asymmetry property — this assumption holds for OGAR's "Wikidata + every modeled class fits one node" claim.

**Currently open in lance-graph that touches OGAR scope**:
- #331 Spine surface for TripletGraph + AriGraph — would be consumed by Sprint 5/6 if it lands.
- #332 Cypher IR → DataFusion plan execution — would integrate with `lance-graph-planner` (Sprint 6).
- #333 unified-bridge crate scaffold — registry+projection orchestrator, **directly adjacent** to OGAR's adapter layer.

## 2. AdaWorldAPI/surrealdb

**Repo**: <https://github.com/AdaWorldAPI/surrealdb>
**OGAR uses**: SurrealQL DDL parser + AST.

| Sprint | API surface OGAR binds to | Stability tier |
|---|---|---|
| 4.5 (`ogar-adapter-surrealql`) | `surrealdb-core::sql::parse`, `Statement::Define`, `DefineTable`, `DefineField`, `DefineIndex`, `DefineEvent` | PINNED per R4 — exact version pin; migrate to `surrealdb-parser` + `surrealdb-ast` when crates.io-published |

**Coordination notes**:
- Per R4 verdict: SurrealDB's internal API "does not adhere to SemVer and its API is free to change and break code even between patch versions." OGAR mitigates by pinning exact version + automating round-trip property tests so breakage surfaces immediately.
- When the workspace split (`surrealdb-parser` / `surrealdb-ast` / `surrealdb-token`) lands on crates.io, OGAR migrates to the leaner parser-only dep and drops the kv/rocksdb/tikv/graphql baggage from `surrealdb-core`'s feature tree.
- OGAR's `OdooAdapter` (Sprint 3, shipped) carries decorator-renames that the SurrealQL adapter will mirror for `DEFINE EVENT` triggers.

## 3. AdaWorldAPI/ractor

**Repo**: <https://github.com/AdaWorldAPI/ractor>
**OGAR uses**: actor framework for `lance-graph-callcenter` runtime (Sprint 7).

| Sprint | API surface OGAR binds to | Stability tier |
|---|---|---|
| 7 (`lance-graph-callcenter`) | `Actor` trait, `ActorRef<M>`, `spawn_linked`, `registry::where_is`, `call_t!` / `cast!` macros, `RpcReplyPort` | TARGET (no consumer yet) |

**Coordination notes** (per R1 research):
- Ractor's default mailbox is **unbounded** `tokio::mpsc`. OGAR's Kanban-bounded mailbox (per SOA-IMPLEMENTATION §5.2) must layer manually with a semaphore. Upstream candidate: add a `BoundedActor` trait or per-mailbox capacity option to ractor itself — OGAR would consume natively.
- Ractor's `ActorRef<M>` is monomorphized over one concrete `M`. OGAR per-class `Msg` enum convention (one enum per `ogar:Class`) is the right pattern; no upstream change needed.
- `subClassOf` → supervision: maps via `spawn_linked`. OGAR codegen emits the link explicitly. No upstream change.
- **Hot reload** is impossible in compiled Rust (R3 finding); OGAR solves via versioned actor identity (`@v3` segment) + supervisor restart on ontology bump. No upstream change.

**Potential upstream contribution candidate**:
- `ractor-kanban` crate or `ractor::mailbox::Bounded<M>` type — bounded mailbox + backpressure signal via `tokio::sync::watch`. Pattern is OGAR-driven but generic enough to be a Ractor ecosystem citizen.

## 4. tokio-rs/tokio + tracing

**OGAR uses**: async runtime + `tracing` + `tracing-opentelemetry` for actor span propagation.

| Sprint | API surface | Stability tier |
|---|---|---|
| 7 | `tokio::sync::watch` (backpressure signal), `tokio::sync::mpsc` (under Ractor), `tokio::task::Builder` (worker spawn) | LOCKED — broad ecosystem stability |
| 7 | `tracing` + `tracing-opentelemetry` with `OpenTelemetrySpanExt` | LOCKED — current stable API |

**Coordination notes** (per R2 research):
- Cross-actor span propagation requires envelope-injection of `opentelemetry::Context` per message. ActionInvocation carries `trace_id` for log correlation (OGAR's idempotent low-overhead path); the full OTel context lives in the message envelope when tracing is enabled. Sprint 7 wires both.

## 5. Apache Arrow + Lance

**OGAR uses**: columnar SoA wire format throughout (per SOA-IMPLEMENTATION).

| Sprint | API surface | Stability tier |
|---|---|---|
| 4 (`ogar-vocab-soa`) | `arrow::array::*Builder`, `arrow::record_batch::RecordBatch`, `arrow::datatypes::Schema` | LOCKED |
| 4 | `arrow::array::DictionaryArray<UInt32, Utf8>` for identity columns | LOCKED |
| 4 | `arrow::array::ListArray` for nested `Vec<Association>` etc. | LOCKED |
| 5 | Lance file format v2 manifest paths | LOCKED |

## 6. Sub-repos NOT in scope (deliberate)

- **surrealdb mainline `surrealdb/surrealdb`**: OGAR uses the AdaWorldAPI fork only.
- **slawlor/ractor mainline**: OGAR uses the AdaWorldAPI fork; upstream contributions filed separately if any.
- **lancedb/lance mainline**: OGAR uses through AdaWorldAPI/lance-graph; direct lance deps only through the contract layer.

## 7. Cross-repo coordination convention

When OGAR's roadmap requires a new upstream API:
1. File an Issue in the relevant fork with title `[OGAR Sprint N] needs <API>`.
2. Link the Issue to the matching `.claude/PLAN.md` Sprint section.
3. Note the deadline (Sprint dependency).
4. When the upstream API lands, update this file with the binding tier.

When the upstream changes break OGAR:
1. Pin the old version; open an Issue describing the regression.
2. Coordinate the migration as a follow-up Sprint (NOT a blocker on
   the current OGAR sprint).
3. Use Sprint 2.6's conformance corpus to detect the regression.

## 8. Cross-references

- `.claude/PLAN.md` — Sprint-by-Sprint roadmap.
- `docs/SOA-IMPLEMENTATION.md` — the four-layer SoA stack (storage / contract / IR / runtime).
- `docs/ADAPTERS-AND-ACTORS.md` — the actor + adapter carve-out.
- `.claude/board/EPIPHANIES.md` — research provenance (R1 Ractor, R2 OpenTelemetry, R3 Odoo `@api.depends`, R4 SurrealQL parser, R5 event sourcing).
