# Lance-Graph Integration — The Clean Idea

> **Purpose.** Re-anchors OGAR against what `AdaWorldAPI/lance-graph`
> **already ships**, so OGAR consumes rather than duplicates. Written
> after reading the upstream `lance-graph-ontology`,
> `lance-graph-contract`, and `lance-graph-callcenter` crates.
>
> **This is a strategic correction.** The earlier `SOA-IMPLEMENTATION.md`
> and `ADAPTERS-AND-ACTORS.md` assumed OGAR would build the four-layer
> stack itself. It already exists upstream. OGAR's real shape is
> narrower and cleaner: **a cross-language `SchemaSource` producer**.
>
> Status: **CARVED v0** (2026-06-04).

## 1. What lance-graph already ships (do NOT rebuild)

| Crate | What it is | OGAR was going to… | Verdict |
|---|---|---|---|
| `lance-graph-contract` | Zero-dep canonical types: `Schema`, `LinkSpec`, `SemanticType`, `Marking`, `PropertySpec`, `Cardinality`, `CodecRoute`, `ExternalMembrane` | "build SoA integration" (Sprint 5) | **Consume.** These ARE the IR target. |
| `lance-graph-ontology` | `OntologyRegistry` + `MappingProposal` + `SchemaSource` + TTL hydrators (SKOS, PROV-O, schema.org, FIBO, **Odoo**, ZUGFeRD, SKR03/04) + 47KB Lance dictionary cache + `wikidata_hhtl` | "build ontology cache" (Sprint 6) | **Consume.** OGAR is a `SchemaSource` into this. |
| `lance-graph-ontology::odoo_blueprint` | 15 lanes (l1–l15) of typed `OdooEntity` consts carrying **fields, methods, decorators, state machine, constraints, provenance**. `op_emitter.rs` (OpenProject), `class_signature.rs`, extractor at `tools/odoo-blueprint-extractor/` | "build ODOO-TRANSCODING from scratch" (Sprint 4/5) | **Align.** `OdooEntity` IS OGAR's `Class`, Odoo-specific. OGAR generalizes it cross-language. |
| `lance-graph-callcenter` | `ExternalMembrane` impl: Phoenix/pgwire server, cognitive-event / steering-intent / memory / actor-session ledgers | "build Ractor actor-per-class" (Sprint 7) | **Naming collision.** This is a different callcenter. OGAR's actor runtime must rename. |
| `lance-graph-planner` | Cypher / Gremlin / SPARQL / GQL parsers (`strategy::*`) | (referenced) | **Consume.** Query layer exists. |
| `lance-graph-supervisor`, `lance-graph-rbac`, `lance-graph-catalog`, `lance-graph-consumer-conformance` | supervision, row-level security, catalog, producer conformance | (partly Sprint 2.6) | **Consume.** Conformance crate already exists. |

## 2. OGAR's real shape (the clean idea)

> **OGAR is the language-agnostic Active-Record vocabulary + the
> cross-language producer layer that emits `MappingProposal`s into the
> existing `OntologyRegistry`. It generalizes `odoo_blueprint::OdooEntity`
> from Odoo-only to Ruby / Python / Ecto / SQL, and adds the behavior-
> execution layer (Action invocation) that ontology does not cover.**

```
   ┌─ Ruby AR ──┐  ┌─ Python Odoo ─┐  ┌─ Django ─┐  ┌─ SQL DDL ─┐
   │ ruff_ruby  │  │  ogar-python  │  │  ...     │  │  ...      │
   │   _spo     │  │ (≈ aligns with│  │          │  │           │
   │            │  │ odoo_blueprint│  │          │  │           │
   │            │  │  extractor)   │  │          │  │           │
   └─────┬──────┘  └──────┬────────┘  └────┬─────┘  └─────┬─────┘
         └────────────────┴────────────────┴──────────────┘
                            │
                            ▼
                    ┌───────────────┐
                    │  ogar-vocab   │  Class / Association / Attribute /
                    │  (canonical   │  EnumDecl / MethodDecl / ComputedField /
                    │   AR IR)      │  ActionDef / ActionInvocation
                    └───────┬───────┘
                            │
                            ▼  ogar-to-proposal (NEW — Sprint 5 revised)
                    ┌───────────────────────────────┐
                    │  impl SchemaSource for OGAR    │
                    │  ogar::Class      → Schema     │
                    │  ogar::Association→ LinkSpec   │
                    │  ogar::Attribute  → SemanticType + Marking │
                    │  → MappingProposal             │
                    └───────────────┬───────────────┘
                                    │
                                    ▼
                    ┌───────────────────────────────┐
                    │  lance-graph-ontology          │  ← EXISTS upstream
                    │  OntologyRegistry::append(...)  │
                    │  Lance dictionary cache         │
                    │  TTL hydration / time-travel    │
                    └───────────────────────────────┘
```

## 3. The producer seam (exact types)

`lance-graph-ontology::SchemaSource`:

```rust
pub trait SchemaSource {
    fn proposals(&self, sem: &SemanticTypeMap) -> Result<Vec<MappingProposal>>;
    fn created_by(&self) -> String;
}
```

OGAR implements this. Each OGAR `Class` becomes a `MappingProposal`:

```rust
// lance-graph-ontology::MappingProposal
pub struct MappingProposal {
    pub public_name: String,       // OGAR class name ("WorkPackage")
    pub bridge_id: String,         // "ogit-op" / "ogit-erp" / tenant
    pub ogit_uri: OgitUri,         // canonical OGIT URI
    pub namespace: String,         // "WorkOrder" / "Network" / ...
    pub kind: MappingProposalKind, // Entity{Schema} / Edge{LinkSpec} / Attribute{SemanticType}
    pub marking: Marking,          // PII / Financial / Restricted / None
    pub confidence: f32,           // 1.0 canonical, <1.0 scanner-suggested
    pub source_uri: String,        // audit: the .rb / .py file path
    pub checksum: String,          // SHA256 for idempotent re-hydration
    pub created_by: String,        // "ogar_ruby_producer_v1" etc.
}
```

### 3.1 Structural mapping

| OGAR IR | → lance-graph-contract type | Notes |
|---|---|---|
| `Class` | `Schema { name, properties: Vec<PropertySpec>, view }` via `Schema::builder()` | one `MappingProposalKind::Entity` |
| `Attribute` | `PropertySpec` (predicate + `SemanticType` + `Marking` + `CodecRoute`) | properties of the Entity's Schema |
| `Association` | `LinkSpec { subject_type, predicate, object_type, cardinality, codec_route }` | one `MappingProposalKind::Edge` |
| `AssociationKind::BelongsTo` | `Cardinality::OneToOne` (FK side) | |
| `AssociationKind::HasMany` | `Cardinality::OneToMany` | |
| `AssociationKind::HasAndBelongsToMany` | `Cardinality::ManyToMany` | |
| `EnumDecl` | `SemanticType` annotation on the backing `PropertySpec` | enum-backed column |
| `Attribute.options.required` | `PropertyKind::Required` | drives `Schema::validate()` |
| `Attribute.options.company_dependent` / PII-ish | `Marking::{Financial, PII, Restricted}` | tenancy + governance |

### 3.2 The `&'static str` impedance

`Schema.name` and `PropertySpec.predicate` are `&'static str` — the
contract is **const-leaning** (compile-time schemas, like
`odoo_blueprint`'s `const ENTITIES: &[OdooEntity]`). OGAR producers
generate schemas at **runtime** from parsed source. Resolution
options (decide in Sprint 5):
1. **String interning** — `Box::leak` interned names into `&'static`
   (acceptable: ontology terms are bounded + long-lived).
2. **Runtime Schema variant** — propose upstream a `SchemaOwned` /
   `RuntimeSchema` with `String` fields (coordination issue — but
   Issues are disabled on the fork; raise via `UPSTREAM-DEPS.md`).
3. **MappingProposal-only path** — `MappingProposal` already uses
   `String` for `public_name` / `namespace`; if the registry can
   ingest an owned-schema proposal, no `&'static` needed at the OGAR
   boundary.

**Carve-out**: Sprint 5 resolves this by reading the registry's
actual append path. Until then, OGAR's `Triple` emitter (Sprint 1–3)
stays as the **portable / debug / RDF-export** form; the
`MappingProposal` path is the **canonical lance-graph integration**.

## 4. odoo_blueprint alignment (not duplication)

`lance-graph-ontology::odoo_blueprint::OdooEntity` already carries
exactly OGAR's shape, Odoo-specific:

```
OdooEntity { fields, methods, decorators, state_machine, constraints, provenance }
   ≈
ogar::Class { attributes, methods, callbacks/computed_fields, (Workflow ext), validations, declared_in_module }
```

**The clean division of labor:**

- **odoo_blueprint** owns the Odoo→OGIT→OWL→DOLCE→FIBO normalization
  chain + 15 lanes of curated Odoo entity consts + the extractor at
  `tools/odoo-blueprint-extractor/`.
- **OGAR** owns the **cross-language generalization**: the same
  entity shape from **Ruby AR / Django / Ecto / SQL DDL**, plus the
  **runtime action-execution layer** (ActionInvocation with
  provenance + state) that the blueprint (static const data) doesn't
  model.

**Carve-out**: OGAR's `ogar-python` Odoo producer (Sprint 4) SHOULD
emit `OdooEntity`-compatible output or consume the
`odoo-blueprint-extractor` directly, NOT reimplement Odoo parsing.
The two converge at `OdooEntity` ≅ `ogar::Class`.

## 5. The behavior layer is OGAR's genuine addition

`lance-graph-ontology` is about **static schema** (Entity / Edge /
Attribute). It does NOT model:

- **ActionInvocation** — a fired business action with SPO+TeKaMoLo +
  provenance (trace_id, parent_invocation, idempotency_key) +
  lifecycle state (Pending/Committed/Failed).
- The **runtime dispatch** of actions to per-class actors.

This is OGAR's differentiated contribution. BUT:

- `ActionDef` (the static declaration) overlaps `OdooEntity.methods`
  + `OdooStateMachine` + `OdooDecorator`. Align with those.
- `ActionInvocation` (the runtime firing) is genuinely new — it's
  execution history, closer to `lance-graph-callcenter`'s
  cognitive-event ledger than to the ontology.

**Carve-out**: OGAR's action-runtime (formerly "Sprint 7
lance-graph-callcenter") must RENAME to avoid the collision with the
existing `lance-graph-callcenter` (ExternalMembrane / Phoenix /
pgwire). Candidate: `ogar-runtime` or fold into
`lance-graph-supervisor`. The ActionInvocation ledger may be a
projection over `lance-graph-callcenter`'s `cognitive_event` /
`actor / session` ledgers rather than a new store.

## 6. Revised sprint map

| Old sprint | Was | Now |
|---|---|---|
| 4 (`ogar-vocab-soa`) | "build RecordBatch schemas" | **KEEP but narrow** — Arrow conversion only where the registry doesn't already provide it; prefer feeding `MappingProposal`. |
| 4.5 (`ogar-adapter-surrealql`) | SurrealQL bidirectional | **KEEP** — still OGAR-owned (registry has no SurrealQL hydrator). |
| 5 (`lance-graph-contract` SoA) | "build SoA integration" | **REPLACE** → `ogar-to-proposal`: `impl SchemaSource for OGAR producers`; `From<ogar::Class> for MappingProposal`. |
| 6 (`lance-graph-ontology` cache) | "build cache" | **REPLACE** → register OGAR proposals into the existing `OntologyRegistry`; add `hydrate_ar` / OGAR-TTL to the hydrator set. |
| 7 (`lance-graph-callcenter`) | "build Ractor runtime" | **RENAME + RESCOPE** → `ogar-runtime` (Ractor actor-per-class), and evaluate whether ActionInvocation is a projection over the existing callcenter ledgers. |

## 7. What stays unambiguously OGAR-owned

1. **The AR-pattern vocabulary** (`ogar-vocab`, `vocab/ogar.ttl`) —
   the cross-language canonical form. Nothing upstream generalizes
   AR across Ruby/Python/Ecto/SQL.
2. **The producers** (`ruff_ruby_spo`, `ogar-python`) — Ruby AR +
   Django producers. (Odoo aligns with `odoo-blueprint-extractor`.)
3. **The Action vocabulary** (`ActionDef` + `ActionInvocation` +
   SPO+TeKaMoLo) — the behavior layer.
4. **The cross-vocab bridges** (`vocab/ogar-bridges.ttl`, Sprint 2.5)
   — `skos:exactMatch` between OGAR roles and source-language verbs.
5. **The identity grammar** (`IDENTITY-MAPPING.md`) — the 5 syntax
   variants + parser (Sprint 1c).

## 8. Immediate next actions

1. **Sprint 5 (revised)** — read `OntologyRegistry`'s actual append
   API + the `Schema` runtime-construction path; implement
   `SchemaSource for OgarRubySource` emitting `MappingProposal`s for
   one real OpenProject `WorkPackage` model; verify it lands in the
   registry dictionary.
2. **Resolve the `&'static str` impedance** (§3.2) — pick interning
   vs owned-schema vs proposal-only.
3. **Coordinate odoo_blueprint alignment** — note in
   `UPSTREAM-DEPS.md` that `ogar-python` consumes / aligns with
   `tools/odoo-blueprint-extractor` rather than reimplementing.
4. **Rename the action-runtime** away from `lance-graph-callcenter`.

## 10. The SurrealQL → kanban → lance-graph lane (partly wired upstream)

Per fork-maintainer note: `SurrealQL → kanban → lance-graph` plus a
`lance self-trigger CI after version update` are **mostly wired but
not tested in detail**. What's actually upstream:

### 10.1 `surreal_container` — SurrealDB-on-Lance (storage execution)

`crates/surreal_container` wires an **in-process** `surrealdb::Datastore`
backed by the `kv-lance` storage engine (the `AdaWorldAPI/surrealdb`
fork feature, NOT upstream crates.io surrealdb):

```
SurrealQL query → surrealdb::Datastore → kv-lance backend → Lance dataset (append-only)
```

It is heavily **BLOCKED** (Lance 6 semver unconfirmed, `surrealdb`
fork URL/branch + `kv-lance` feature flag needed, ndarray fork patch)
— this is the "mostly wired, not tested" surface.

**OGAR relationship**: `surreal_container` executes SurrealQL **queries**
against Lance. OGAR's `ogar-adapter-surrealql` (Sprint 4.5) parses
SurrealQL **DDL** (`DEFINE TABLE` / `DEFINE FIELD`) → `ogar::Class`.
**Different altitude — complementary, not overlapping.** OGAR is
schema-translation; `surreal_container` is query-execution. OGAR's
parsed schema can `DEFINE` the tables `surreal_container` then serves.

### 10.2 kanban — genuine OGAR contribution

Code search for `kanban` across lance-graph: **zero matches**. The
Kanban-bounded mailbox (bounded WIP + pull + backpressure, per
`SOA-IMPLEMENTATION.md` §5.2) is **not yet built upstream**. This is
a real OGAR contribution opportunity — the flow-control layer between
SurrealQL ingest and lance-graph append, gating burst writes against
the ~1–4 commits/sec Lance ceiling (per R2 research).

**Carve-out**: OGAR's Kanban mailbox is the natural home for the
"surrealQL > kanban < lance-graph" pacing. It belongs in the
renamed `ogar-runtime` (§5), OR offered upstream as
`ractor-kanban` / `lance-graph`-side bounded-ingest (per
`UPSTREAM-DEPS.md` §3 contribution candidate).

### 10.3 Lance version self-trigger CI

`.github/workflows/release.yml` + `.bumpversion.toml` drive a
`workflow_dispatch` version-bump → tag → release flow (bump-my-version,
preview/stable channels). This is the mechanism OGAR Sprint 6 relies
on for **ontology cache invalidation on version bump** — when the
ontology dataset version increments, the watcher fires. It's wired
but "not tested in detail," so OGAR Sprint 6 must NOT assume it's
bullet-proof: add an explicit integration test that a version bump
propagates a cache-invalidation event end-to-end.

**Carve-out**: OGAR Sprint 6 owns the **test** for the
version→invalidation propagation even though the trigger mechanism is
upstream. The conformance corpus (Sprint 2.6) is the right place for
this assertion.

## 11. SurrealQL/kanban/CI — net effect on OGAR sprints

| OGAR sprint | Adjustment from §10 |
|---|---|
| 4.5 (`ogar-adapter-surrealql`) | Confirmed OGAR-owned + complementary to `surreal_container`. OGAR parses DDL; surreal_container executes queries. Note the seam. |
| 6 (ontology cache) | Lean on the existing version→CI trigger, but OWN the integration test that version bump → cache invalidation actually fires (it's undertested upstream). |
| 7 → `ogar-runtime` | Kanban mailbox is genuinely unbuilt upstream — it's OGAR's to build, and it's the "kanban" in `surrealQL > kanban < lance-graph`. |

## 9. Cross-references

- `docs/UPSTREAM-DEPS.md` — the dependency tiers (updated by this doc)
- `docs/SOA-IMPLEMENTATION.md` — SUPERSEDED in part: the four-layer
  stack exists upstream; OGAR consumes it. Kept for the SoA wire-form
  carve-outs that still hold at the producer boundary.
- `docs/ADAPTERS-AND-ACTORS.md` — the Action vocabulary stands; the
  "build the callcenter" framing is rescoped per §5.
- `docs/ODOO-TRANSCODING.md` — aligns with `odoo_blueprint`, not
  duplicates it (§4).
- `.claude/PLAN.md` — Sprints 5/6/7 revised per §6.
- Upstream: `AdaWorldAPI/lance-graph` crates `lance-graph-contract`,
  `lance-graph-ontology` (+ `odoo_blueprint`), `lance-graph-callcenter`,
  `lance-graph-supervisor`, `lance-graph-consumer-conformance`.
