# HIRO → OGAR DO-arm lift — lossless actionable semantics from the Automation domain

> **Status:** FINDING (mapping + principle, grounded) + CONJECTURE (the producer).
> **Preflight:** authored under `docs/SURREAL-AST-TRAP-PREFLIGHT.md` — behavior
> flows **producer → OGAR `Class`+`ActionDef` → adapter**, never producer → DDL.
> **Reads:** OGIT `NTO/Automation/entities/*` (KnowledgeItem · ActionHandler ·
> ActionCapability · ActionApplicability · Trigger · Intent · AutomationIssue ·
> History · Variable), `docs/OGAR-AST-CONTRACT.md` (the DO-arm types).

The MARS import lifted the **structural** arm (A→R→S→M `Class`es). The
**Automation** domain — HIRO's actuator vocabulary — is the **behavioral** arm
the structural import left on the table. This doc maps it into OGAR's DO arm and
pins the one rule that keeps the lift **lossless**.

---

## 1. The lossless-DO principle (the governing rule)

A DO compiler is **lossy** when it tries to express behavior in **one flat
target** (a DDL `DEFINE EVENT … WHEN … THEN …`, a single schema row). Behavior
has **three irreducible slices**, and any single format holds at most one or two:

| Slice | Home (lossless) | Carries |
|---|---|---|
| **Identity + structure** — *what* is acted on | `Class` / `ClassView` (THINK arm) | the **address** |
| **Contract + lifecycle** — *how* it's invoked | `ActionDef` IR + the trait (`StateMachine` / `UnifiedStep`) | the typed **control** (predicate · object_class · `KausalSpec` · `results_in` · `on_enter` · `guard_failure_policy` · `state_timeout_millis` · idempotency) |
| **Executable body** — the *actual* command | the **adapter** binding | the **behavior**, verbatim |

> **The rule: DO is lossless iff the `ActionDef` *points to* the body
> (content-addressed) instead of *compressing it into* DDL.** This is
> `I-VSA-IDENTITIES` applied to the DO arm — an identity that points to content,
> never a bundle of the content's register. The `classid` is the join key across
> all three slices; nothing is projected away because each slice has a home that
> holds it exactly.

It is **not** "interfaces *vs* adapters *vs* classes" — it is **all three**. The
lossiness is precisely what happens when one of them is forced to carry all three
(the DDL flatten: keeps "when X then Y", drops the lifecycle / guard-failure
modality / SLA / the `ActionDef`(static) ÷ `ActionInvocation`(dynamic) split —
the "negative-beauty hijack" `SURREAL-AST-AS-ADAPTER.md §0` rejects).

---

## 2. Export shape — what NOT-lossy looks like on the wire

Not a DDL. Export **three artifacts joined by `classid`**:

1. **ActionDef manifest** — a typed SoA record (the contract). **Wire-truth** per
   the Firewall (ADR-022/023): the IR *is* the wire, no re-serialization.
2. **Payload table** — the body as an **opaque, content-addressed blob**
   (`hash → bytes`), resolved by the adapter at invocation. The IR holds the
   *pointer* (classid/hash), never the bytes.
3. **ClassView** — identity (the join key).

The adapter is *where behavior executes* (Core-First: it **assumes** the Core via
`classid`, never carries its own state); the interface (`StateMachine` /
`UnifiedStep`) is *how it's invoked*; the class is *what it's addressed by*.
Round-trip is lossless: the body is preserved verbatim, the contract as typed IR.

---

## 3. HIRO is already this shape (the external proof) — `[G]`

arago's Automation graph is a production, lossless DO encoding. Read directly:

- **`KnowledgeItem`** carries the contract as relations + lifecycle attrs
  (`uses Variable`, `contains Trigger`, `worksOn AutomationIssue`,
  `relates MARSNodeTemplate`, `solves Task`, `generates Timeseries`,
  `deployToEngine` / `deployStatus`) **and the body rides as
  `knowledgeItemFormalRepresentation` — an opaque blob the schema references but
  never parses.** That is the identity-points-to-body split, verbatim.
- **`AutomationIssue generates History`** is literally OGAR-AST-CONTRACT's
  "state history IS the version log."
- **`ActionHandler` connects `Configuration` + `ActionCapability` +
  `ActionApplicability`** — and `Configuration` ⊨ `auth_store` (`0x0B01`, the
  family we minted). The **DO arm and the auth/RBAC arm meet at `ActionHandler`.**

We do not invent the shape; we adopt the one arago validated at scale.

---

## 4. Mapping — OGIT Automation → OGAR DO arm

| OGIT `Automation` entity | OGAR DO-arm target | Evidence (the actionable fields) |
|---|---|---|
| `KnowledgeItem` | **`ActionDef`** | `knowledgeItemFormalRepresentation` = opaque body (→ payload table); `uses Variable` = params/defaults; `relates MARSNodeTemplate` = `object_class`; `solves Task` / `generates Timeseries` = `results_in` |
| `ActionCapability` | `ActionDef` params + **adapter binding** | `mandatoryParameters`; "remote execution via ssh" = the executor the adapter wires |
| `ActionApplicability` | **`KausalSpec::StateGuard`** | `environmentFilter`; "on `ogit/_id`" = the guard predicate (WHEN the action applies) |
| `ActionHandler` | the **adapter** (+ membrane) | connects `Configuration` (= `auth_store` `0x0B01`) + `Capability` + `Applicability` |
| `Trigger` | **`KausalSpec::LifecycleTrigger`** | "Trigger of a KI having a set of Subitems" |
| `Intent` | **`ActionDef.results_in`** / desiredState | "The Intent of a KI processing chain" |
| `AutomationIssue` | **`ActionInvocation`** | situational data (Key/Value tuples); `worksOn MARSNode` = `object_instance`; `generates History` |
| `History` | `ActionInvocation` **state log** | = the Lance version log ("state history IS the version log") |
| `Variable` | `ActionDef` params / `defaults` | `uses Variable` |
| `MARSNode` / `MARSNodeTemplate` | `object_class` / `object_instance` | the THINK arm — **already imported** structurally |
| `Configuration` | `auth_store` (`0x0B01`) | already minted; the membrane the handler authorizes through |

---

## 5. Worked example — `KnowledgeItem` → `ActionDef`

```text
ogit.Automation:KnowledgeItem "restart-stuck-service"
  knowledgeItemFormalRepresentation = <XML body / script>        ─┐  opaque
  uses        ogit.Automation:Variable (serviceName, host)        │  payload
  contains    ogit.Automation:Trigger (onIssueType=ServiceDown)   │  (content-
  relates     ogit.Automation:MARSNodeTemplate (Software/Service) │   addressed,
  worksOn     ogit.Automation:AutomationIssue                     │   never
  generates   ogit:Timeseries (restart latency)                  ─┘   parsed)
        │  lift (shape only — body stays a blob)
        ▼
ActionDef {
  predicate:            "restart-stuck-service",
  object_class:         Identity(Software/Service),        // ← relates MARSNodeTemplate
  kausal:               LifecycleTrigger { on: ServiceDown }, // ← contains Trigger
  defaults:             [serviceName, host],               // ← uses Variable
  results_in:           Some(StateTransition::to(Restarted)), // ← Intent / generates
  guard_failure_policy: Postponable,                       // ← env: transient → retry
  payload_ref:          blake3(formalRepresentation),      // ← POINTER, not the bytes
}
// the <XML body> lives in the payload table @ blake3(...), executed by the adapter.
```

Lossless check: every lifecycle/guard/param field is a typed `ActionDef` field;
the executable body is preserved verbatim behind a content hash. A DDL emit of
this would keep "on ServiceDown, run X" and **lose** `guard_failure_policy`,
`results_in` typing, and the `ActionDef`÷`ActionInvocation` split.

---

## 6. The producer (proposed) — `[H]`, gated on a probe

Parallel to `ogar-from-schema` (structural arm), an Automation-domain DO lift
(extend `ogar-from-schema` with a `do_arm` module, reusing its TTL parser):

```text
Automation/entities/*.ttl  →  ActionDef{ predicate, object_class, kausal,
                                          defaults, results_in, payload_ref }
                              + payload table (formalRepresentation blobs)
```

- **`payload_ref` only** — the lift NEVER inlines or reparses
  `knowledgeItemFormalRepresentation`; it hashes it into the payload table.
- **CONJECTURE until `PROBE-OGAR-DO-ARM-LIFT` is green:** an `ActionDef` →
  adapter → execute → result must reproduce the KI's recorded behavior
  bit-for-bit on a fixed corpus (the same falsification discipline as
  `PROBE-OGAR-RBAC-AUTHORIZE`). Until then the mapping in §4 is a FINDING about
  *shape*, not a certified executable equivalence.

---

## 7. One-line rule

> **DO is lossless iff the `ActionDef` points to the body, never flattens it into
> DDL.** Identity → content; three slices (class · interface · adapter) joined by
> `classid`; export the typed manifest + the opaque payload, never the DDL.
