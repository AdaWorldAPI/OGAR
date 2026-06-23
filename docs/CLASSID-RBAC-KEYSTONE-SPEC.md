# CLASSID-RBAC KEYSTONE — SPEC v2 (hardened)

> v1 → v2 after the full hardening arc: an 8-agent 5+3 pass, an 8-agent
> "best-of-prior-art" pass, a registry-RBAC prior-art map (Postgres /
> Odoo / Django / Redmine), a NIST RBAC deep-dive, and an OAuth/OIDC +
> Zitadel convergence check. v1 was BLOCK'd by overclaim-auditor on two
> open questions (Q1 deny, Q3 lattice); **v2 closes both** via the
> ReBAC reframe below. Zero remaining BLOCK.
>
> **Unit pin (theorem-checker rule 0):** `ClassId = u16`, hex
> (`0x09XX`). This is the **contract-tier** discriminator
> (`lance_graph_contract::class_view::ClassId`), distinct from the
> on-disk node-key `classid (u32, 8 hex)` in the GUID canon — related by
> zero-extension, NOT the same field. Roles, memberships, member-roles,
> actors are themselves classids (`0x0117 / 0x0108 / 0x0118 / 0x0104`).

---

## I-K0 — THE REGISTRY AXIOM (foundational)

OGAR is a **DTO registry**. The **label** (classid + canonical name) is
the **KEY**; the **meaning** (schema / DTO shape / grant) is the
**VALUE**. "OGAR inherits SCHEMA" = the *value* composes up the class
hierarchy (the key prerenders the node; the value resolves, and may be
CAM-compressed — the GUID-canon key-value model). **Roles and
permissions are registry/CAM entries — label→meaning — never text
blobs.** Replacing `project_role.permissions: text` with first-class
registry permission tuples is the load-bearing Core change (§6).

## 0. The spine — ReBAC relationship-tuples, COMPILE-resolved

Authorization is a graph of **typed relationship tuples**
`classid # relation @ subject` — Google Zanzibar / Ory-Keto / OpenFGA
shape, which is *literally* OGAR's "RBAC is a relation, not a stamp"
(revert `87e8dd2`). This is **ReBAC, not NIST RBAC** — and that label is
deliberate: the NIST deep-dive proved object/class-hierarchy grant
propagation is **not** RBAC (NIST objects are a flat set) but **is** the
recognized ReBAC userset-rewrite mechanism. Calling it ReBAC turns a
flagged "invention" into a citation, and **dissolves the two-lattice
worry**: role-membership, role-implication, and class-parent are just
**different relation types in one tuple graph**, not conflated lattices.

**OGAR's differentiator (the reason this isn't a Keto clone):** Keto /
OpenFGA / SpiceDB **walk the tuple graph at request time** (the Check
API — a runtime traversal). That is the hot-path interpretation the
**Firewall (ADR-022/023) forbids**. OGAR **compiles** tuple resolution
into the `classid` prefix + CAM/palette bitmaps (the `_effectiveReaders`
precompute) — **"Zanzibar resolved at the key, not at the query."**
There is no shipped Rust compiled-Zanzibar; OGAR is it.

## 1. Why (the defect removed)

Today authz is stringly + per-consumer: each consumer hand-rolls a
`UnifiedBridge<XPort>` whose `authorize_*` calls
`Policy::evaluate(actor_role: &str, entity_type: &str, op)`
(`lance-graph-rbac/policy.rs:33`). The role/membership relation exists as
**inert edge-data** in OGAR (`project_role/membership/member_role`) but
nothing traverses it, and the decision keys on `entity_type: &str`, never
on `classid`. v2 makes the relation real, tuple-shaped, classid-keyed.

## 2. Invariants (PROPOSED — none enforced until the Z-items land)

- **I-K1 — classid is the currency.** Decisions key on `classid`; name→classid is an OGAR adapter (`HealthcarePort::class_id`) resolved *before* authorize.
- **I-K2 — class inheritance carries SCHEMA, role hierarchy carries GRANTS (the correction).** Field/DTO shape inherits up the **class** lattice (`class_ancestors` + `FieldMask::inherit`). **Grants do NOT** — they inherit up the **role** lattice (`implied_roles`; Postgres `INHERIT` / Odoo `implied_ids`). No prior-art system inherits grants up the object hierarchy (Postgres table-inherit explicitly does not propagate GRANTs). [resolves Q3]
- **I-K3 — the spine names no consumer.** `lance-graph-rbac` / `lance-graph-ogar` speak only classid + tuples. One shared-spine edit to `callcenter::UnifiedBridge`, then zero *new* per-consumer bridges.
- **I-K4 — contract handshake, no heavy dep.** rbac consumes a zero-dep `lance_graph_contract::ClassRbac` trait; the **impl lives in `ogar-class-view`** (which already deps both `ogar-vocab` and `lance-graph-contract`, and already `impl ClassView`). rbac never deps `ogar-vocab`.
- **I-K5 — positive grants; deny/scope are separate explicit channels.** The grant stage is positive-only tuples (`R⁺`). Deny is explicit exclusion (OpenFGA `but not` / Postgres restrictive default-deny) and instance scope is a row-predicate — **both routed to the scope axis, never folded into the grant set.** [resolves Q1 — `held ∩ reach` stays monotone because signs live elsewhere]
- **I-K6 — actor = its memberships, sourced from the token.** An actor is the set of `project_membership (0x0108)` tuples it holds; in production these arrive as **OIDC token claims** (the `sub` + role/org claims ARE the membership assertions). `ActorId` is opaque and accepts an external IdP subject.
- **I-K7 — Firewall: parse once, decide on the key.** The token (and any tuple source) is a **boundary** artifact, parsed once at the membrane → resolved to compiled classid/CAM. The inner decision is a bit-op, never a runtime interpreter (this is why Odoo `ir.rule` runtime-domain-eval and the Zanzibar Check-API runtime walk are adopted as *semantics*, not *mechanism*).
- **I-K8 — four orthogonal axes, never collapsed.** `(verb × class)` ⟂ `role-hierarchy` ⟂ `row-scope` ⟂ `field-projection`. Odoo is the only prior art that keeps all four in separate stores; OGAR does too.

## 3. The four axes (one tuple-relation type each)

| Axis | Best-of source | OGAR form |
|---|---|---|
| **1. class-grant** (verb × class) | **Odoo `ir.model.access`** | typed registry/CAM tuple `(role_classid, target_classid, op_mask)` — a value-tenant on `project_role 0x0117`, palette-native (#511 `SoaMemberSpec`), replacing `permissions: text`. NOT Django string codename, NOT Postgres `aclitem` (grantor = delegation state). |
| **2. role-hierarchy** (grant inheritance) | Postgres `INHERIT` / Odoo `implied_ids` | `project_role.implied_roles` edge; `roles_reaching` folds **role** ancestors. Caveat: Odoo `implied_ids` breaks the strict partial order (a user keeps an implied role after the implier is removed) — OGAR uses a **true partial-order** closure (NIST RH). |
| **3. row-scope** (g-lock / tenant) | Odoo `ir.rule` *semantics* / Postgres RLS *default-deny* | a separate predicate facet keyed `(role_classid, target_classid) → ScopeSpec`, **compiled** to a palette/Hamming bitmap (NOT a runtime domain). Carries the re-instated Redmine sign-columns (`*_visibility`, `assignable`) + the `org`/tenant claim. |
| **4. field-projection** | OGAR-native (`FieldMask`) | `FieldMask` over the class field-basis, inherited up the **class** lattice via `FieldMask::inherit`; `authorize` carries the granting roles' unioned mask. |

## 4. The contract seam

```rust
// lance-graph-contract (zero deps) — OGAR implements, rbac consumes
pub trait ClassRbac {
    fn roles_reaching(&self, class: ClassId) -> RoleSet;   // role-hierarchy folded; positive R⁺
    fn actor_roles(&self, actor: ActorId) -> RoleSet;      // membership → member_role → role
    fn row_scope(&self, role: ClassId, class: ClassId) -> Option<ScopeSpec>; // compiled predicate
    fn field_mask(&self, role: ClassId, class: ClassId) -> FieldMask;        // axis 4
}
```
Impl: `impl ClassRbac for OgarClassView` (same struct that already impls
`ClassView`). `roles_reaching` keeps `grants(R)` (direct) queryable for
audit/provenance — do not let the closure erase the direct/inherited
distinction.

## 5. authorize — two-stage, positive ∧ scope, projection-carrying

```rust
pub fn authorize(rbac: &impl ClassRbac, actor: ActorId, class: ClassId, op: Operation)
    -> AccessDecision {
    let granting = rbac.actor_roles(actor) ∩ rbac.roles_reaching(class); // positive, monotone
    if granting.is_empty() { return Deny; }
    if !granting.any(|r| grant_permits(r, class, op)) { return Deny; }   // op_mask gate
    // stage 2: compiled row-scope predicate (restrictive default-deny)
    let scope = granting.filter_map(|r| rbac.row_scope(r, class));      // AND globals, OR group-rules
    let mask  = granting.map(|r| rbac.field_mask(r, class)).fold(inherit); // axis 4 union
    Allow { scope_predicate: scope, projection: mask }   // Allow CARRIES scope + mask
}
```
Mirrors Odoo `ir.model.access ∧ ir.rule` / Postgres `aclmask ∧ RLS`,
hardened to restrictive default-deny.

## 6. OGAR Core changes (deliberate, calibrated)

- **Replace `project_role.permissions: text`** (`lib.rs:2812`) with a typed `granted` value-tenant: `Set<(target_classid: u16, op_mask: u8)>` (Odoo `ir.model.access` shape; #511 width-calibration = max grants/role, low tens → one palette column).
- **Add `project_role.implied_roles`** (role-hierarchy edge; the primitive Redmine lacks, Odoo/Postgres prove).
- **Add a `row_scope` facet** keyed `(role, class) → ScopeSpec` reusing the existing `KausalSpec`/`scope_source` machinery; compiled, not interpreted.
- **Re-instate the dropped Redmine sign-columns** (`*_visibility`, `assignable`) onto the scope axis (the harvest flattened them).
- `class_ancestors` (Z1) stays — but **only for SCHEMA / `FieldMask` (axis 1/4 shape), never for grants** (I-K2).
- `actor_roles` (Z2) unions `inherited_from` member-roles, with individual-revocation honored.
- EdgeBlock (membership triangle `0x0108/0118/0104`) untouched — ratified from the Rails/Redmine harvest, not re-imported.

## 7. OAuth / OIDC boundary — the AuthStore class family (preminted profiles)

> **Correction (2026-06-22, supersedes the `0x011B`–`0x011E` ids below):**
> auth classes do NOT belong in the project block (`0x01XX`). Per
> `APP-CLASS-CODEBOOK-LAYOUT.md` §2, auth is a **core domain of its own,
> `0x0B`** (cross-app, provider-agnostic profiles → core, `hi = 0x0000`).
> Mint: `auth_store = 0x0000_0B01`, `auth_zitadel = 0x0000_0B02`,
> `auth_zanzibar = 0x0000_0B03`, `auth_ory_keto = 0x0000_0B04`. The
> `0x011B`–`0x011E` ids in this section are the earlier (project-block)
> draft, retained for provenance only — use the `0x0B` domain.
> Everything else in §7 (the mapping behaviour, I-K7, the Zitadel 1:1)
> stands unchanged. Auth classes still target `actor 0x0104` / `role
> 0x0117` whose low halves are shared core concepts.
>
> **MINTED + CONFIRMED (2026-06-23):** the `0x0B` family is now in code
> — `ogar_vocab::class_ids::{AUTH_STORE 0x0B01, AUTH_ZITADEL 0x0B02,
> AUTH_ZANZIBAR 0x0B03, AUTH_ORY_KETO 0x0B04}`, `ConceptDomain::Auth`,
> `all_promoted_classes()` builders (`auth_store()` + the three
> provider profiles), and `ogar-class-view` registration. These are
> **reservations** (the enforcement `authorize()` stays gated on §10).
> The mint is CONFIRMED by the canonical OGIT shape: arago's
> January-2026 `NTO/Auth/Configuration` entity — keyed by
> `organizationId`/`accountId`/`applicationId`/`scopeId` +
> `configurationData`, "registered in hiro knowledge core" — IS
> `auth_store`, built upstream independently (the convergence is 1:1;
> see `.claude/board/EPIPHANIES.md` 2026-06-23 + `ISSUES.md`
> ISS-RBAC-AUTHORIZE-BY-CLASSID). The vision and the upstream shape
> agree, which is what made this mint a reservation rather than an
> invention.

The IdP→classid mapping is not a service and not a scattered set of hooks — per the registry axiom (I-K0) **the bridge IS a registry class**, preminted in the codebook:

- **`auth_store` (0x011B)** — the base. It *does the mapping*: `sub → actor (0x0104)`, `role-key → role (0x0117)`, `org/tenant → scope` (axis 3). Carries the three claim-name slots as attributes.
- **`auth_zitadel` (0x011C)**, **`auth_zanzibar` (0x011D)**, **`auth_ory_keto` (0x011E)** — preminted **provider profiles**, each is-a `auth_store`, carrying that provider's claim grammar as data: Zitadel (`urn:zitadel:iam:org:project:roles` + org URN), Zanzibar/OpenFGA (the `object#relation@subject` tuple grammar), Ory Keto. Keycloak / Auth0 follow the identical mint.

Selecting a provider = **picking its preminted classid** — zero spine change, one data profile per IdP. The inner `authorize` kernel never touches a token (I-K7): the token is parsed once at the membrane, the chosen `auth_store` profile resolves its claims to classids/tuples, and only resolved keys go inward. **Zitadel maps 1:1** (Project→class scope, Project-Role→role, Authorization/Grant→membership tuple, Organization→scope, User→`sub`).

## 8. Where OGAR genuinely DIFFERS from every twin (honest section)

- **Compile-time codebook ≠ runtime catalog.** Roles/grants are *minted classids* (schema), not inserted rows. The grant *map* can be runtime; the roles are not.
- **CAM / key-addressed ≠ row-addressed.** No per-row anything inside; scope is a compiled bitmap, not a walked predicate.
- **Actor = membership-set, no subject table** (the identity model is the OIDC `sub`, resolved to tuples).
- **It's ReBAC (Zanzibar), not NIST RBAC** — own the label.

## 9. Open questions — RESOLVED

- **Q1 (deny):** RESOLVED — grants are positive `R⁺`; deny = explicit exclusion + scope predicate (I-K5). Intersection stays monotone.
- **Q3 (lattice):** RESOLVED — grants ride the **role** lattice (I-K2); class lattice carries schema only. The object-hierarchy reach is ReBAC, named, not mislabeled RBAC.
- **Q4 (projection collapse):** RESOLVED — `authorize` returns the unioned `FieldMask` (§5); axis 4 stays separate.
- **Q5 (dep tier):** RESOLVED — impl in `ogar-class-view` (already deps both); rbac stays contract-tier (doctrine-keeper).

## 10. The gate before any consumer code — `PROBE-OGAR-RBAC-AUTHORIZE`

`authorize(actor, classid, op)` + the scope predicate must reproduce a
reference system's decision **bit-for-bit** on a fixed corpus (Odoo
`ir.model.access ∧ ir.rule`, or Redmine `User#allowed_to?`, or an
OpenFGA model) before consumer-collapse (step 5) lands. Until green, the
keystone is **CONJECTURE**.

> **STEP-4 STATUS (2026-06-23) — PARTIAL GREEN (positive ∧ op-gate half).**
> The classid-keyed kernel is built and the gate is green against the
> **in-repo reference** — the shipped membrane gate
> `lance_graph_rbac::policy::Policy::evaluate` (the "reconcile the shipped
> MembraneGate path with the keystone" framing of
> `ISS-RBAC-AUTHORIZE-BY-CLASSID`). Impl + probe:
> `lance-graph/crates/lance-graph-rbac/src/authorize.rs` —
> `ClassRbac` (§4) · `authorize()` (§5 positive ∧ op-gate, deny-reasons
> mirrored exactly) · `ClassGrants` (`PermissionSpec` re-keyed by `ClassId`,
> §11). `probe_ogar_rbac_authorize` reproduces `Policy::evaluate`
> **bit-for-bit** over a 15-tuple corpus (all roles/ops/deny-reasons +
> depth boundary + unknown actor); `probe_is_falsifiable_under_wrong_keying`
> proves the gate is not vacuous (a wrong classid flips an Allow).
>
> **What this certifies:** the §5 *positive ∧ op-gate* half + the §11 classid
> re-keying — promoted CONJECTURE→FINDING **for the shipped reference**.
> **What remains CONJECTURE:** the §5 stage-2 *row-scope* predicate and the
> projecting `Allow { scope, mask }` return — the shipped reference is
> positive-only, so the scope-bearing references (Odoo `ir.model.access ∧
> ir.rule`, OpenFGA) are the follow-on probes that exercise stage 2. The
> keystone stays CONJECTURE **as a whole** until a scope-bearing reference is
> green; the positive half is now FINDING. Cross-ref: lance-graph
> `EPIPHANIES.md` E-RBAC-AUTHORIZE-PROBE-GREEN (2026-06-23).

## 11. Build / PR order + cross-refs

Order: **(1)** `lance-graph-contract` `ClassRbac` trait → **(2)** OGAR
`ogar-vocab` Core changes (§6) + `ogar-class-view` impl → **(3)**
`lance-graph-rbac` `authorize` + re-key `PermissionSpec` to `ClassId`
→ **(4)** `PROBE-OGAR-RBAC-AUTHORIZE` green → **(5)** consumer collapse
(medcare #169 gate → `authorize(actor, HealthcarePort::class_id("Patient"))`;
`MedcareBridge`/`MedcareRegistry`/`medcare-bridge` evaporate — but the
**g-lock scope** moves to axis 3, it does not vanish).

Docs to update in lockstep (doctrine-keeper): `DISCOVERY-MAP.md`
(D-entry + `87e8dd2` provenance), `INTEGRATION-MAP.md` (`ClassRbac`
seam + F-gates), `ADAPTERS-AND-ACTORS.md` / `IDENTITY-MAPPING.md`
(I-K6, OIDC `sub`), `HEALTHCARE-TRANSCODING.md`, `ODOO-TRANSCODING.md:556`
(`ir.rule`-out cross-ref to THE-FIREWALL §3), `ARCHITECTURAL-DECISIONS`
(ADR-026), and lance-graph board hygiene (`LATEST_STATE` Contract
Inventory + `PR_ARC_INVENTORY`).

## 12. Hardening record (condensed)

- **5+3 (v1):** runtime-archaeologist PASS (grounding exact); core-first TARGETS-CORE; doctrine-keeper PASS (impl-site = ogar-class-view); theorem-checker/core-gap/dilution/overclaim/arxiv CONCERNS → the fixes folded above.
- **Best-of-prior-art (8 agents):** unanimous — Odoo registry decomposition as skeleton, Rails/Redmine membership triangle (already harvested), Postgres restrictive-default + role-`INHERIT`; **drop the class-grant-walk**; Django (string codename) and Postgres `aclitem` (grantor state) are storage anti-patterns; Frankenstein guard = never fuse role-lattice and class-lattice.
- **NIST deep-dive:** object-hierarchy grant propagation is **ReBAC, not RBAC** — drove the §0 reframe.
- **OAuth/Zitadel convergence:** Zanzibar tuple ≅ `classid::role::membership`; token = membrane carrier; Zitadel native 1:1 (§7).
