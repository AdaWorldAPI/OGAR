# Unified Verb Facade — one small set of verbs over the ClassView

> **Status:** council-hardened v1 (5+3 ran 2026-07-01, 8/8 → REVISE, land the
> safe subset). Renamed from `UNIFIED-EMITTER-API-v1.md`.
> **Under:** #546, #137 ("the spine is the compiled ClassView, not SurrealQL"),
> `SURREAL-AST-AS-ADAPTER.md`.

## The idea, plainly

Give consumers **six verbs** — `define · query · relate · act · subscribe ·
authorize` — as `step_type` names on the **existing** `OrchestrationBridge` /
`UnifiedStep`. Nothing new underneath. These are just the natural things you do
to a ClassView node.

SurrealDB happens to have the same-looking verbs (`DEFINE`, `SELECT`, `RELATE`,
`LIVE`, …). That's a **familiar shape we borrow, not the engine.** The engine is
the compiled ClassView. Think "same silhouette, different body."

## The one honest rule the council found

Only **`define` really matches** SurrealDB (schema in → schema out, no runtime).
The other five just **borrow the name** — underneath they're OGAR's own things
(EdgeBlock, ActionDef, the scheduler, RBAC). So:

- Direction is always **OGAR → SurrealQL (out)**, never SurrealQL → behavior (in).
- **`authorize` is two verbs, never one**: `authorize.filter` (who can *see*) and
  `authorize.admit` (who can *do*). Merging them is a bug.
- Ship only what's built: `define`, `query`, `relate`, `subscribe`, and basic
  `act`. Leave `authorize.*` and lifecycle-`act` as **reserved names** until the
  Core pieces (`Authorization`, `StateMachineDecl`) land.

## What's actually built vs. borrowed

| Verb | Built? |
|---|---|
| `define` | **yes** — `OgarClassView` (ClassView × FieldMask), tested |
| `query` | mostly — `Backend::MailboxSoa` scans work; no Cypher entry point yet |
| `relate` | partly — `EdgeBlock` struct decodes; write/traverse not done |
| `act` | data yes (`ActionDef`), runtime later (via `CommitHook`, not "CommitGate" — that name doesn't exist) |
| `subscribe` | trait yes (`VersionScheduler`), live binding later |
| `authorize` | RBAC exists (`ClassRbac`/`OgarRbac`), but the two-plane split is not built |

## First step (safe now)

A `step_type` name list + a `UnifiedStep` builder. **No new code that runs**, no
SurrealQL, no OSINT state machine yet. It's a **proposal, not proven**, until the
parity tests (F1/F3 below) pass.

Tests to pass later: **F1** a `query` step returns the same nodes as the raw scan;
**F2** filter-rules never hit the admit-gate and vice-versa; **F3** with SurrealQL
off, behaviour is identical; **F4** no new type beyond a builder; **F5** every verb
names a real symbol in code (fixed the two fake ones this pass).

## Council (5+3), one line each

- **convergence** — only `define` truly converges; the rest just share a name.
- **core-first** — the ClassView is the authority, not SurrealDB; drop "two spines".
- **integration-lead** — 4 verbs are safe today; reserve `authorize`/lifecycle-`act`.
- **cross-domain** — an API over a running DB ≠ an API over emitted code; home is the bridge.
- **runtime-archaeologist** — `CommitGate` and `Authorization{enforcement_phase}` don't exist yet; point at the real types.
- **overclaim** — don't call unbuilt things "shipped"; one backend is live, not two.
- **doctrine** — split `authorize`; `act` is egress-only; rename off "SurrealQL-shaped"; add ADR-026.
- **dilution/collapse** — keep it "silhouette, not spine"; split `authorize` in the table itself.

**Verdict: revise, land the safe subset, keep the big claim as a proposal until tested.**
