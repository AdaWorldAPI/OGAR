# ISSUES.md — open issues / blockers / tracked decisions for OGAR

> **APPEND-ONLY.** Newest at top. Each entry: an id, a `**Status:**`
> line (OPEN / RESOLVED / SUPERSEDED — the only mutable line), the
> question, the evidence, the decision/resolution, and what (if
> anything) remains gated. Corrections append as new dated lines citing
> the original.

## Entries (newest first)

## ISS-RBAC-AUTHORIZE-BY-CLASSID — reconcile the shipped MembraneGate path with the keystone; mint the AuthStore family
**Status:** RESOLVED (decision made autoattended; the `0x0B` mint is shipped this session; the `authorize()` *enforcement* remains gated on `PROBE-OGAR-RBAC-AUTHORIZE`, the keystone's own §10 gate)
**Filed/Resolved:** 2026-06-23

### What this corrects
An earlier framing called classid-keyed authorize a "hard blocker." It
is not. The auth surface is largely SHIPPED, just not unified, and
OGAR's own design for it is already hardened:

- `lance-graph-rbac` exists: `Policy` / `Role` / `Operation::{Read{depth},
  Write{predicate}}` / `AccessDecision::{Allow,Deny,Escalate}` /
  `smb_policy()`.
- smb-office-rs PR #29 shipped `SmbMembraneGate` (~30 LOC, newtype over
  `Arc<lance_graph_rbac::Policy>`, keyed `(role × entity_type)`, impl
  `MembraneGate`). The spec's enforcement, instantiated.
- woa-rs is on the OLD path (`UnifiedBridge<WoaBridge>`); migration =
  mirror smb #29. NOT blocked on a missing primitive.
- medcare is the bigger lift (lacks `medcare-rbac` + `medcare-realtime`,
  ~800 LOC, deferred behind DM-7/DM-8 per `MEDCARE_POLICY_GAP.md`).
- **OGAR's own `docs/CLASSID-RBAC-KEYSTONE-SPEC.md` v2 is hardened
  (zero remaining BLOCK)** and already decided classid-keyed authorize
  is canonical (I-K1), with the AuthStore class family preminted (§7).

### It's not two harnesses — it's interim (shipped) vs canonical (gated), one sequence
1. SHIPPED INTERIM: `Policy` (keyed `entity_type: &str`) + `MembraneGate`
   / `SmbMembraneGate` (smb #29). Proven.
2. lance-graph `super-domain-rbac-tenancy-v1.md` §3.9: 4-stage
   `authorize`; §13.1 composes onto `callcenter::policy`.
3. OGAR CANONICAL: `CLASSID-RBAC-KEYSTONE-SPEC` v2 — `authorize(actor,
   classid, op)` via the `ClassRbac` trait, ReBAC-compiled-at-the-key,
   typed grants replacing `project_role.permissions: text`, consumer
   bridges evaporate after collapse (§11.5). Gated on
   `PROBE-OGAR-RBAC-AUTHORIZE` (§10).

The keystone §1 names the string-`entity_type` `Policy` as the *defect*
it removes — so "MembraneGate canonical" and "keystone canonical" differ
on END-STATE but NOT on sequencing: the keystone is gated on an un-green
probe, so MembraneGate is the right thing to ship NOW.

### The OGIT convergence that makes this easy (operator insight 2026-06-23)
OGAR's keystone vision and the canonical OGIT shape CONVERGE 1:1.
arago's January-2026 `NTO/Auth/Configuration` entity (keyed by
`organizationId`/`accountId`/`applicationId`/`scopeId` +
`configurationData`, "registered in hiro knowledge core") IS the
keystone's `auth_store` — the IdP→classid mapping class — built
upstream independently. Zitadel maps 1:1
(Project→class-scope, Project-Role→role, Grant→membership tuple,
Org→scope, User→`sub`). The decision dissolves: we are matching an
existing shape, not inventing one. (Receipt: `EPIPHANIES.md` 2026-06-23
entries; `OGAR-AS-IR.md` linker phase.)

### Decision (autoattended, RESOLVED)
- **Canonical end-state:** the keystone v2 — classid-keyed `authorize`
  via `ClassRbac`, AuthStore profiles. Already hardened; confirmed by
  the OGIT shape.
- **Interim (ship now, unblocks woa):** woa-rs mirrors smb #29 —
  `WoaMembraneGate` over `Arc<lance_graph_rbac::Policy>`, classid from
  `WoaPort::class_id`. This is the SHIPPED `MembraneGate` pattern, NOT a
  `*Bridge` stopgap — so it does not violate the README guidance.
- **Sequencing:** the probe orders them; they do not compete.

### Shipped this session (the tractable, ungated part)
The `0x0B` Auth domain is **minted** in `ogar-vocab` per keystone §7 —
`auth_store 0x0B01` + `auth_zitadel 0x0B02` / `auth_zanzibar 0x0B03` /
`auth_ory_keto 0x0B04`: CODEBOOK entries, `class_ids` consts, `ALL`,
`ConceptDomain::Auth` (`0x0B` → Auth), `all_promoted_classes()`
builders, `ogar-class-view` `all_canonical_classes()` registration,
tests (`auth_domain_concepts_resolve_and_route`). Reservations only —
"reserving costs nothing." 298/0 workspace tests; fmt-clean; no new
clippy.

### Held (gated, NOT this session — the keystone's own gates)
- The `authorize()` **enforcement** (ClassRbac trait impl + the
  bit-for-bit decision) — gated on `PROBE-OGAR-RBAC-AUTHORIZE` (§10)
  running green against a reference (Odoo `ir.model.access ∧ ir.rule`,
  Redmine `User#allowed_to?`, or an OpenFGA model). Security-review-class.
- The woa-rs `WoaMembraneGate` mirror — a different repo; mirrors smb #29
  when woa work is picked up. Unblocked, not gated.
- `project_role.permissions: text` → typed-grant Core change (§6) —
  lands with the keystone build order §11, after the probe.

### Refs
`docs/CLASSID-RBAC-KEYSTONE-SPEC.md` (§7 AuthStore, §10 probe, §11
order); lance-graph `super-domain-rbac-tenancy-v1.md` §3.9/§13.1;
smb-office-rs PR #29; `MEDCARE_POLICY_GAP.md`; `EPIPHANIES.md`
2026-06-23 (the OGIT convergence + the mint).
