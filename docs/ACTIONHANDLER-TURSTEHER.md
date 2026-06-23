# OGAR der Türsteher mit Köpfchen — ActionHandler as classes, RBAC hardcoded, Rung as Flughöhe

> **Status: DOCTRINE v0 (2026-06-23).** Pairs with `HIRO-IN-CLASSES.md`
> (what bardioc gains), `HIRO-DO-ARM-LIFT.md` (the DO-arm mapping +
> lossless-DO rule), `CLASSID-RBAC-KEYSTONE-SPEC.md` (the four-axis
> ReBAC keystone), and ada-docs `architecture/COLD_PATH_MUL_ACTIONHANDLER.md`
> (the cold-path / MUL / kgV-ActionHandler treatment from the consumer side).
> Most of this doc is `[G]` — it is grounded in shipped code, cited inline.

The operator's pin, verbatim: *"wie wäre es wenn wir den actionhandler mit
Klassen im OGAR mit einbauen — dann hat Hiro/Bardioc das Gefühl, daß sie da
gleich andocken können, und die Flughöhe darüber machen wir mit den Rung-Levels
(1-9); die Flughöhe sind dann im Hot-Path und RBAC ist hardcoded in OGAR — das
macht sich bei Compliance-Leuten, die von Semantik keine Ahnung haben, gut. OGAR
der Türsteher mit Köpfchen."* And the containment pin: *"ob der da oben AGI hat,
braucht sie nicht zu beunruhigen — OGAR kriegt sie alle."*

This doc carves that into the canon in three moves: **(1)** the ActionHandler
lives as OGAR classes; **(2)** RBAC is a compile-time `const` on those classes,
not a runtime policy; **(3)** the cognition above is bounded by the Rung
Flughöhe in the hot path, and bounded by *structure* — not by trust.

---

## 1. ActionHandler IS OGAR classes — so HIRO/Bardioc dock native `[G]`

The DO arm of a `Class` is a `const` table of `ActionDef`s — the action-axis
sibling of the THINK-arm `ClassView`. This is now shipped as the
**`OgarActionProvider`** (`lance-graph/crates/lance-graph-ogar/src/actions.rs`):

```
classid  ──►  ClassActions { classid, actions: &'static [ActionDef] }
              actions_for(classid)           — own surface (zero-fallback &[])
              effective_actions(classid)     — own ∪ parent, overrides applied
```

The worked seed is the auth family (keystone §7's `0x0B` core domain):

| class | classid | DO surface |
|---|---|---|
| `auth_store` | `0x0B01` | `issue_token` · `revoke_token` · `rotate_secret` |
| `auth_zitadel` | `0x0B02` | *(is-a `auth_store`)* + `sync_org_roles`, `issue_token` overridden |

`auth_zitadel` **is-a** `auth_store`, so its *effective* surface is the base's
plus its net-new, with `issue_token` overridden onto the Zitadel
org-role-aware path — the exact `classid → ClassView` inheritance the field-set
already uses (`contract::action::effective_actions`). HIRO's `gen_statem`
lifecycles and Bardioc's handlers dock here by becoming `ActionDef` rows on the
same `Class` they already are (`HIRO-IN-CLASSES.md` §1, §3.1) — they do not
wrap OGAR, they *are* OGAR. **No adapter carries its own action table** (the
core-first invariant): the harvest IS the manifest.

This is the "andocken" the operator wants: a HIRO engineer adds a behaviour by
adding a `const ActionDef`, not by writing a new dispatcher.

---

## 2. RBAC is hardcoded — `required_role` is a `const` on the class `[G]`

Every mutating `ActionDef` carries its `required_role` as a **compile-time
literal**, not a runtime policy row:

```rust
// lance-graph-ogar/src/actions.rs  (the auth_store DO surface)
ActionDef { predicate: "rotate_secret", object_class: 0x0B01,
            required_role: Some("auth_admin"),
            guard: Some(StateGuard { field: "status", value: "active" }),
            exec: ExecTarget::Native, overrides: None }
```

Why a `const`, and why this is the Türsteher:

- **A compliance reviewer reads the grant surface off the source.** No database,
  no admin console, no "effective permissions" query — the class's authorization
  is `grep required_role`. For Compliance-Leute who do not read semantics, this
  is the legible artifact: *who may rotate the secret? `auth_admin`, full stop,
  pinned in the class.*
- **A roleless mutating action is structurally visible.** `required_role: None`
  is an explicit, audited declaration ("unguarded"), never an accidental open
  door that a missing policy row would create. The test asserts the invariant
  directly: *every mutating auth action has `Some(role)`*
  (`auth_store_surface_has_hardcoded_rbac_on_every_mutating_action`).
- **An override cannot silently widen access.** `auth_zitadel::issue_token`
  restates `required_role: Some("auth_user")` — the override is `[G]`-tested to
  keep the grant, not elevate it (`zitadel_inherits_auth_store_and_overrides_issue_token`).

The hardcoded role is the **§3 axis-1 verb gate** of the keystone, frozen at the
class. The richer runtime grant *map* (which actor holds `auth_admin`) is still
data — resolved by an `auth_store` OIDC profile from token claims (keystone §7,
I-K7). The class fixes *what role is required*; the token fixes *who has it*. The
typed `granted` value-tenant that carries the runtime half is now shipped too:
`contract::rbac::{OpMask, ClassGrant, grants_permit}` (keystone §6) — the
first-class replacement for `project_role.permissions: text`.

---

## 3. The cold path the class enforces — { RBAC · Libet · Rubikon@MUL } `[G]`

An `ActionDef`'s `required_role` and `guard` are the **def half** of the
cold-path gate. The whole hard lifecycle is already in
`contract::action::ActionInvocation::commit`, which adjudicates in order:

```
def-match → RBAC (required_role vs ActorContext)
          → state guard (Libet do/don't — fire only if field==value)
          → MUL impact (the Rubicon crossing: Flow→Committed, Hold→Pending, Block→Cancelled)
```

The kgV executor that runs a *committed* action is `graph-flow-action`'s
`ActionHandler` (rs-graph-llm); the outer planning-cycle envelope is
`graph-flow-kanban`'s `KanbanPlanEnvelope` over the 4-phase Rubicon board
(`Planning → CognitiveWork → Evaluation → {Commit|Plan|Prune}`). OGAR supplies
the *authorized DO surface* both sit over. Three layers, one rule
(ada-docs `COLD_PATH_MUL_ACTIONHANDLER.md`):

| layer | what | where |
|---|---|---|
| **cold path** (hard floor) | RBAC · Libet do/don't · Rubikon last phase · SLA | OGAR class `const` + `commit` |
| **MUL** (one soft window) | `GateDecision::{Flow,Hold,Block}` | planner / cognition, *before* the Libet veto |
| **hot path** (execution) | `ActionHandler::handle` | the executor, reached only on `Committed` |

The kgV invariant (`I-ACTIONHANDLER-IS-KGV-NOT-CHOKEPOINT`): thinking stays
**wide and upstream**; it never crawls under the executor waist. The class fixes
the floor; cognition decides above it; neither can lower the floor.

---

## 4. Rung 1-9 is the Flughöhe — cognition altitude in the hot path `[G]`

The operator's "Flughöhe darüber" is the **Rung** (`contract::cognitive_shader::RungLevel`,
`Surface`=0 … `Transcendent`=9) — *how deep the thinking goes*. It is a hot-path
quantity (it rides `ShaderDispatch.rung`), distinct from the **execution cost
ladder** `ElevationLevel` (`Point`=0 … `Async`=5) — *how much machinery the
search spends*. The two are now calibrated:

```rust
// lance-graph-planner/src/elevation/mod.rs
ElevationLevel::from_rung(rung) -> ElevationLevel   // monotone non-decreasing
//  Surface/Shallow→Point · Contextual/Analogical→Scan · Abstract/Structural→Cascade
//  Counterfactual→Batch · Meta→IvfBatch · Recursive/Transcendent→Async
```

Deeper cognition implies a higher cost **floor of ambition** — but only a floor.
The **Csíkszentmihályi Flow channel**, already in code as
`mul::FlowState{Flow,Anxiety,Boredom,Transition}` + `flow_state_from(challenge,
skill)` driving `elevation::homeostasis`, tunes the actual spend around that
floor: `Boredom` (skill ≫ challenge) raises the patience budget → go *deeper*
than the rung floor; `Anxiety` (challenge ≫ skill) cuts it → cap *below*. So the
Rung sets where the search starts; the flow-modulated budget decides where it
ends. The Flughöhe is real, measured, and self-regulating — not a knob.

This is why the Rung lives *above* the cold path: it modulates *how hard the
system thinks before it decides*, never *whether the class lets it act*. A
Transcendent-rung deliberation and a Surface-rung reflex hit the **same** `commit`
gate with the **same** `required_role`. Altitude does not buy authority.

---

## 5. Containment by structure — "OGAR kriegt sie alle" `[H]`

The operator: *"ob der da oben AGI hat, braucht sie nicht zu beunruhigen."* The
claim is that OGAR contains whatever sits above it **without needing to trust
it** — and the mechanism is structural, not behavioural:

- **Capability-bounding, not alignment-bounding.** The cognition above can be
  arbitrarily capable; it still acts only through `ActionDef`s that exist, with
  `required_role`s it cannot edit, through a `commit` gate it cannot bypass. The
  bound is on *what doors exist*, not on *how clever the agent is*. An AGI that
  wants to `rotate_secret` still needs `auth_admin` — the class does not care how
  it reasoned its way to the request.
- **The DO surface is closed at the Core.** There is no "register a new action
  at runtime" path; the manifest is `const`, harvested, reviewed. Cognition
  cannot mint itself a capability. (Contrast: a tool-use agent that can author
  arbitrary shell commands has an *open* DO surface — the failure mode this
  closes.)
- **Move/ownership semantics make the gate unforgeable.** `commit` consumes the
  invocation's state transition; there is no aliased path to `Committed` that
  skips the RBAC/Libet/MUL checks (the Firewall: no serialization in the hot
  path, ADR-022/023). The compiler is the bouncer's muscle.

So the Türsteher has *Köpfchen* precisely because the intelligence is in the
**doorway geometry**, not in vetting each guest's intentions. You do not have to
solve alignment of the thing above to bound what it can do — you only have to fix
the doors. That is a structural containment claim (`[H]`: argued from the shipped
`const`-surface + `commit` gate; the falsifier is any path that reaches an
external mutation without transiting `commit`).

---

## 6. Cross-references

- `HIRO-IN-CLASSES.md` — the four efficiency wins bardioc gains (dispatch,
  validation, dep-graph, similarity) by living as OGAR classes.
- `HIRO-DO-ARM-LIFT.md` — HIRO Automation → DO arm mapping; the lossless-DO rule
  (`ActionDef` points-to-body, never flattens into DDL).
- `CLASSID-RBAC-KEYSTONE-SPEC.md` — §3 four axes, §5 `authorize`, §6 typed
  `granted`, §7 the `0x0B` AuthStore family, §10 `PROBE-OGAR-RBAC-AUTHORIZE`.
- `SURREAL-AST-AS-ADAPTER.md` — why the behavioural arm cannot live in DDL (the
  reason RBAC is a class `const`, not a `DEFINE EVENT … WHEN … THEN`).
- ada-docs `architecture/COLD_PATH_MUL_ACTIONHANDLER.md` — the cold-path / MUL /
  kgV-ActionHandler / containment treatment (consumer-side companion).
- Shipped code: `lance-graph/crates/lance-graph-ogar/src/actions.rs`
  (`OgarActionProvider`), `…/lance-graph-contract/src/{action,rbac,kanban}.rs`,
  `…/lance-graph-planner/src/elevation/mod.rs` (`from_rung`),
  `rs-graph-llm/graph-flow-action` (kgV `ActionHandler`),
  `rs-graph-llm/graph-flow-kanban` (`KanbanPlanEnvelope`).
