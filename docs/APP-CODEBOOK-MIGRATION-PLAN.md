# APP-CODEBOOK MIGRATION PLAN — Odoo · WoA · SMB · q2(Gotham/aiwar/neo4j)

> Companion to `APP-CLASS-CODEBOOK-LAYOUT.md` (the layout) and
> `CONSUMER-MIGRATION-HOWTO.md` (the generic steps). This doc applies
> the **APP‖class** model (`classid = APP(hi u16) ‖ class(lo u16)`) to
> each remaining app and orders the work.
>
> Status: **PLAN**. Append-only. The minting steps are gated on the
> 5+3 codebook pass; nothing here mints a classid yet.

---

## The one decision every app makes first

For each surface entity, the app answers **one** question:

> *Would a second, unrelated consumer reuse this concept verbatim?*

- **Yes →** map onto a **CORE** classid (`hi = 0x0000`) via the app's
  OGAR `PortSpec`. No private mint. This is the default and the
  overwhelming majority.
- **No →** mint an **APP-private** classid (`hi = 0xAAAA`) in the app's
  reserved codebook namespace. Escape hatch only.

The migration is then identical to `CONSUMER-MIGRATION-HOWTO.md`: pull
classid → enrich by classid → authorize by classid → delete any bridge.
The APP‖class model changes only *where new ids come from* when an app
genuinely needs a private one.

**Reserved prefixes** (from `APP-CLASS-CODEBOOK-LAYOUT.md` §2):
`0x0002` Odoo · `0x0003` WoA · `0x0004` SMB · `0x0006` q2. `0x0000` is
core (auth lives here at domain `0x0B`).

---

## Wave order (cheapest, most-grounded first)

| Wave | App | Port status | Private codebook needed? | Gating |
|---|---|---|---|---|
| **W1** | WoA / woa-rs | `WoaPort` ✅ (OGAR #93) | no (maps to commerce `0x02`) | none — pure repoint |
| **W2** | SMB / smb-office-rs | `SmbPort` ✅ (OGAR #93) | no (maps to commerce `0x02`) | none — pure repoint |
| **W3** | Odoo / odoo-rs | `OdooPort` ✅ (OGAR #94) | no, but **delete the fork** | converge `od-ontology` |
| **W4** | q2 (Gotham/aiwar/neo4j) | ❌ no port | **likely yes** (`0x0006`) | author port FIRST |

W1–W2 are mechanical (the ports exist, concepts are canonical commerce).
W3 is the severity case (odoo-rs re-derives the AR layer). W4 is
greenfield (no port, domain not yet in the codebook).

---

## W1 — WoA / woa-rs  (`0x0003`, consumes commerce `0x02`)

**Port:** `WoaPort` exists (OGAR #93), maps `WorkOrder` and friends onto
the commerce block. **Private codebook: no** — work orders, line items,
invoices, dunning stages are canonical commerce concepts (`0x0000_02CC`).

Steps:
1. Repoint `src/registry.rs`, `src/unified_bridge.rs`, `src/lib.rs`,
   `tests/…` off `lance_graph_ontology::bridges::WoaBridge` to the
   static pull: `WoaPort::class_id(name) -> Option<u16>`, widened to
   `0x0000_0000 | id` at the u32 boundary.
2. Delete `WoaBridge` (= `UnifiedBridge<WoaPort>`) + any hand-rolled
   registry/hydration.
3. Enrich by classid (tenant scoping, Mahnwesen stage rules) — this is
   woa-rs's legitimate domain logic.
4. Authorize by classid once the keystone lands (the `perm_buchhaltung`
   / `@login_required` checks become `authorize(actor, cid, op)`).
5. **Iron Rule 1 (woa-rs CLAUDE.md):** the deps you keep are
   `ogar-vocab` (port) + `lance-graph-rbac` (authorize) — both BBB-tier,
   not brain crates. **File the allow-list RFC** (`rfcs/`) for that
   delta before merging; never add a `lance-graph-*` non-allow-listed
   crate.
6. **Private mint only if** WoA has a genuinely non-canonical object
   (e.g. a Stefan-specific KeePass-vault row with no commerce analogue)
   → `0x0003_FFCC`. Default: none.

Spec cross-ref: `woa-rs/.claude/board/` OGAR-migration note;
behaviour-parity stays the witness (Python writer ↔ Rust writer).

---

## W2 — SMB / smb-office-rs  (`0x0004`, consumes commerce `0x02`)

**Port:** `SmbPort` exists (OGAR #93). **Private codebook: no** — SKR04
accounts, customers, suppliers, work orders, FiBu reconciliation map
onto canonical commerce (`0x0000_02CC`).

Steps:
1. `crates/smb-bridge/src/unified_bridge_wiring.rs` drops `OgitBridge`;
   pull `SmbPort::class_id`.
2. Delete the bridge wiring; the consumer holds no ontology.
3. Enrich by classid (German BSON field-name mapping stays at the
   adapter — `mongo-schema-warden` invariant; PII never leaves).
4. Authorize by classid once the keystone lands.
5. **Iron rule 3 (smb-office-rs CLAUDE.md):** `lance-graph` is
   additive-only — this migration edits **only** smb-office-rs + (if a
   port gap) OGAR. The spine is untouched.
6. **Private mint only if** an SMB object has no commerce analogue →
   `0x0004_FFCC`. Default: none.

Tracked: `smb-office-rs/.claude/board/TECH_DEBT.md`
`TD-OGAR-CONSUMER-MIGRATION-1`.

---

## W3 — Odoo / odoo-rs  (`0x0002`, consumes commerce `0x02`)  — SEVERITY CASE

**Port:** `OdooPort` exists (OGAR #94). **Private codebook: no.** The
problem is not the ids — it's that odoo-rs **forks the AR layer OGAR
exists to own**: its bespoke `od-ontology::{surreal_ast,triple,emit}`
re-derives `op-surreal-ast` / `ogar-adapter-surrealql` and **never
touches `ogar-vocab`**.

Steps (this is convergence, not just a repoint):
1. Lower `od-ontology` onto `ogar_vocab::Class` — its model objects
   (sale.order, account.move, res.partner, product.template, …) map onto
   canonical commerce classids via `OdooPort`.
2. Emit SurrealQL via **`ogar-adapter-surrealql`** (the canonical
   emitter), not the `od-ontology::emit` fork.
3. **Delete the fork** (`surreal_ast` + `triple` + `emit`). This is the
   deliverable — the fork is the debt.
4. Odoo's `ir.model.access` (class-grant) and `ir.rule` (row-scope)
   become the two RBAC axes the keystone already models
   (`CLASSID-RBAC-KEYSTONE-SPEC.md` §4): class-grant on the role class,
   row-scope compiled to a bitmap (NOT runtime domain-eval — that would
   violate the Firewall).
5. **Private mint only if** an Odoo module ships an object with no
   canonical commerce analogue → `0x0002_FFCC`. Most map onto core.

Cross-ref: `ODOO-TRANSCODING.md`, `SURREAL-AST-AS-ADAPTER.md`.

---

## W4 — q2 (Gotham / aiwar / neo4j)  (`0x0006`)  — AUTHOR PORT FIRST

**Port:** ❌ none. This is the gate: until q2's domain entities are
mapped onto canonical `class_id`s in OGAR, q2 cannot pull classids.
q2 is also the app **most likely to need a private codebook** — its
graph/intel entities (Gotham nodes, aiwar war-game objects, neo4j
node/relationship types) are not all canonical, and `0x07 osint` in
core is thin.

Sub-steps, in order:
1. **Triage which sub-surfaces are live.** q2 spans Gotham, aiwar, and a
   neo4j compat layer. neo4j-rs is **legacy** (superseded by lance-graph
   as L3) — confirm it is still a consumer before authoring its port. If
   dead, skip it.
2. **Author `Q2Port: PortSpec`** in `ogar-vocab::ports` (its own OGAR
   PR). Map q2's public entity names → classids. For entities that ARE
   canonical (a generic `document`, `person`, `organization`,
   `location`), map onto core (`0x0000_07CC` osint or the relevant
   domain). For entities that are q2-specific (a Gotham investigation
   case, an aiwar scenario branch, a neo4j-native node label with no
   canonical analogue), mint **app-private** under `0x0006`:
   ```
   0x0006_01CC   q2/Gotham object classes
   0x0006_02CC   q2/aiwar scenario + branch classes
   0x0006_03CC   q2/neo4j legacy node/rel adapter classes (if still live)
   ```
   This is the operator's "codebook per project" win in its purest form:
   q2 gets a full 65 536-class private space, its own centroid-codebook
   hierarchy, **without** overflowing the shared core radix trie.
3. **Then the generic steps apply** (`CONSUMER-MIGRATION-HOWTO.md`):
   pull classid → enrich → authorize → no bridge.
4. **scenario/time-travel note:** aiwar's "what-if" branching maps onto
   the existing scenario inventory (Lance version time-travel /
   `World::fork` / Pearl Rung-3), **not** a new `scenario_id` column —
   spawn `scenario-world` before proposing anything there.

Cross-ref: q2 has no OGAR transcoding doc yet — **authoring
`Q2-TRANSCODING.md` is part of W4** (mirror `ODOO-TRANSCODING.md`).

---

## Rendering convergence (all waves) — key-value, zero serde in the hot path

Every app's render path migrates to the same shape
(`APP-CLASS-CODEBOOK-LAYOUT.md` §3.5–3.6):

- **Template dispatch becomes classid-driven.** Replace ad-hoc
  `match entity_kind { … }` template selection with the object's full
  classid → `ClassView::resolve(classid)` → Askama template. The **high
  u16** is the app's render prefix; the **low u16** is the shared concept
  (RBAC + ontology). woa-rs and smb-office-rs already use Askama
  (compile-time-checked) — only the *selection* changes, not the engine.
- **Fields become keys, not blobs.** Strings, text, media, online
  sources resolve by key-value lookup against typed content stores
  (string dictionary / text column / media bytes / URI registry) — never
  serialized into the row and parsed back. If a render step needs
  `serde::Deserialize`, the field was inlined as a blob: that is the
  Firewall violation; make it a key.
- **DoD render litmus:** grep the app's render path for
  `serde_json::from_*` / `Deserialize` on the hot path — there should be
  none. Content is addressed, not deserialized.

This is the operator's stated goal: strings / text / media / online
sources rendered via key-value, so no serialization exists in the hot
path. The per-app classid (high u16) is what makes per-app rendering
scale without forking concept ids or overflowing the shared codebook.

## Convergence with the RBAC keystone (all waves)

Every wave's step 4 ("authorize by classid") lands the same way once
`lance-graph-rbac` ships (`CLASSID-RBAC-KEYSTONE-SPEC.md`):

```
authorize(actor, classid, op) -> Allow | Deny
  where actor = its membership set (I-K6),
        grants resolve up the ROLE lattice (ReBAC, not the class lattice),
        scope (row-level) compiles to a bitmap, never runtime-walked.
```

The auth providers (Zitadel / Zanzibar / Ory-Keto) are **core**
preminted class profiles (`0x0000_0BCC`), so token→actor resolution is
shared by every app — no per-app auth wiring. An app hands a classid;
the grant lattice is upstream. Until the keystone lands, each app keeps
its existing auth (do NOT reintroduce a bridge as a stopgap).

---

## Definition of done (per app)

1. The app's OGAR `PortSpec` exists and resolves every surface entity
   (to a core id, or to a reserved app-private id).
2. No `XBridge` / `UnifiedBridge<…>` symbol survives in the app repo.
3. The classid pull is a pure static function call (no `Registry`, no
   `hydrate`).
4. The diff touches **only** OGAR (port + any private class block) + the
   app's own crate. `lance-graph-ogar` / `lance-graph-rbac` are
   byte-for-byte unchanged. (If you edited the spine, you did it wrong.)
5. Private mints, if any, are justified by the "would a second consumer
   reuse this?" test in the PR description, and reserved under the app's
   `0xAAAA` prefix — never flat in core, never in another app's prefix.
