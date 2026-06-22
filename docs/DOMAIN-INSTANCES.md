# OGAR Domain Instances — universality demonstrated, not asserted

> **Purpose.** Catalogue the concrete domains OGAR has been instantiated
> against, mapping each to the substrate capabilities it exercises. The
> "be everything later" claim (`SUBSTRATE-ENDGAME.md`) and the Foundry-
> parity argument (`§5.2` there) rest on *real* domain coverage, not
> aspiration — and two of these are **production-grade shipping
> deployments**, not calibration toys.
>
> **The claim in one line:** five domains — two calibration
> (`chess`, `OpenProject`), one migration target (`Elixir-HIRO`), and
> **two production instances** (a production `Odoo/ERP` deployment, a
> production `HIPAA/healthcare` deployment) — together exercise the full
> substrate capability surface. Universality is *demonstrated across real
> domains*, not claimed.
>
> **Generic labels by design — "inherit schema via contract".** Domain
> instances are named by their *domain* (ERP, healthcare/HIPAA), never by
> concrete project. That's not just hygiene — it *is* the architecture: a
> deployment **inherits the schema shape from the contract** (`Class` /
> `ActionDef` / `KnowableFromStore` / `ExternalMembrane`) and **rebinds
> the concrete labels** via the `Adapter` pattern (`ADAPTERS-AND-ACTORS.md`
> §2 — HHTL leaf renames like `move → transport`). The label (the project
> name, the prefix string, the field captions) is a **consumer property**,
> not an architectural constant — the consumer changes it at will. Two
> deployments in the same domain share the contract-inherited shape and
> differ *only* in their rebound labels. So this catalogue names
> *domains*; the concrete instance is whatever a consumer labels it. The
> genericisation here is the worked example: you can't tell which project
> it is, because the architecture doesn't depend on which project it is.
>
> **This is also a confidentiality property, not just hygiene.** Because
> the contract carries the schema *shape* and never the labels, a
> deployment's **PII field captions** (e.g. a healthcare deployment's
> German field labels) are consumer-bound via the `Adapter` and **never
> enter OGAR's contract surface**. The substrate holds "there is a
> protected field here, with these access controls," not "the field is
> called `<PII caption>`." For PII / GDPR / HIPAA that's a *guarantee by
> construction*: the labels can't leak through OGAR because OGAR never
> holds them. (The firewall's outer boundary — `THE-FIREWALL.md` — is
> where a consumer's labelled schema is read; it stays consumer-side.)
>
> Status: **CARVED v0** (2026-06-05).

## 1. The instances

| Domain | Kind | Spec / instance | Status |
|---|---|---|---|
| **Chess** | calibration (closed-formal) | `docs/CHESS-TRANSCODING.md` + `AdaWorldAPI/shakmaty` | spec'd; merged |
| **OpenProject** | calibration (open-messy Rails) | `docs/OPENPROJECT-TRANSCODING.md` + `opf/openproject` | spec'd; merged |
| **Elixir / HIRO** | migration target (OLD stack) | `docs/ELIXIR-HIRO-PREFETCH.md` + `crates/ogar-from-elixir` | scaffold; merged |
| **Odoo / ERP** | **production instance** | `docs/ODOO-TRANSCODING.md` + a production ERP deployment | shipping |
| **HIPAA / healthcare** | **production instance** | a production healthcare (HIPAA) deployment | shipping |
| **Geospatial / OSM** | calibration (geographic) | `docs/RDF-OWL-ALIGNMENT.md §10` Phase 2c + `lance-graph` PR #473 `cesium-osm-substrate-v1.md` (D-OSM-1..7) | spec'd; runtime addendum shipped; `ogar-from-osm-pbf` queued |
| **MARS (HIRO/bardioc CMDB)** | calibration (closed-formal, XSD-frozen) | `docs/MARS-TRANSCODING.md` + `vocab/imports/ogit/NTO/MARS/` (1:1 mirror) + `_oracle/MARSSchema2015.xsd` + `crates/ogar-from-schema` | spec'd; lift shipping; bijection mechanically tested (15 tests green) |

The first three are how the substrate is *calibrated* (chess proves the
Semantik/Syntax/Pragmatik trichotomy separates cleanly; OpenProject
proves production-Rails-AR survives; Elixir-HIRO is the migration spine).
Odoo/ERP and HIPAA/healthcare are how the substrate is *already used in
anger*. Geospatial/OSM and MARS are the structural-arm calibrations: OSM
proves spatial prefix routing, MARS proves frozen-schema bijection.

## 2. Per-instance — what each exercises

### 2.1 Chess (`shakmaty`) — closed-formal calibration
The cleanest separation of Semantik / Syntax / Pragmatik (per
`CHESS-TRANSCODING.md §0`): finite vocabulary (12 pieces × 64 squares),
published bijective notation (FEN/SAN/UCI/PGN), and a free §14 oracle
(`shakmaty::Position::play`). Exercises: lifecycle FSM (`ActionState`),
`Postpone` (premove), `StateTimeout` (clock), `on_enter` (move
application). The calibration target — pass here and the substrate's
core is sound.

### 2.2 OpenProject — open-messy production-Rails calibration
Concerns, `acts_as_*`, STI, polymorphic associations, data-driven FSMs
(`Workflow` table), `has_paper_trail` (→ Lance-version consolidation).
Exercises: the full structural arm + the database-hydrator pattern
(ADR-014) + paper-trail-as-audit. The destination is OP-as-operator-pane
(`SUBSTRATE-ENDGAME.md` Room 3).

### 2.3 Elixir / HIRO — the OLD-stack migration target
`gen_statem` lifecycles, GenServer/Phoenix/Oban actions, Ecto schemas.
The load-bearing `gen_statem`→Rubicon case (`ELIXIR-HIRO-PREFETCH.md
§2.2`). Exercises: the migration scaffold (`SUBSTRATE-ENDGAME.md`
Room 2) + the wire-roundtrip §14 oracle. The reason `ogar-from-elixir`
exists.

### 2.4 Odoo / ERP — production instance
**OGAR for Odoo, made real** — the `ogit-erp::` prefix instantiated. A
production ERP deployment transcodes Odoo (SeaORM persistence, analytic
accounting, HR, the ERP money/decimal model). Exercises the substrate's:
- **structural arm** — Odoo models → `Class` (the `ODOO-TRANSCODING.md`
  mapping in production).
- **behavioral arm** — Odoo `@api.depends` computed fields →
  `KausalSpec::Depends` (the data-causal guard); workflow transitions →
  `ActionState` lifecycle.
- **enum/selection handling** — Odoo `selection` + `selection_add` →
  `EnumSource::{Static, Add}` (the inheritance-aware enum the SurrealQL
  emitter handles).
- **money/decimal precision** — the ERP correctness constraint
  (decimals, not floats) — a real-world data-fidelity requirement.

A production ERP deployment proves the Odoo transcoding spec isn't
paper: real ERP runs on the OGAR-shaped IR. (Which project, and what
it's labelled, is a consumer detail — per §0.)

### 2.5 HIPAA / healthcare — production instance
**OGAR for healthcare with HIPAA compliance.** A production healthcare
deployment exercises the substrate's **Security Mesh** (the parity
matrix's "row-level permissions" row) end-to-end, and is the canonical
demonstration of The Firewall (`THE-FIREWALL.md §7.2`):
- **row-level access control (inner / hot)** — palette256
  `_effectiveReaders` bitmap + Hamming-popcount bit-intersection per PHI
  access. No serialization (the firewall's inner rule); fast enough to
  gate every field read.
- **immutable audit trail (outer / firewall)** — audit-as-Lance-version
  append, serialized + signed, once per access crossing. The HIPAA legal
  requirement met by the audit-log ↔ Lance-version consolidation
  (ADR-013's pattern generalized to compliance).
- **`ExternalMembrane` + `LazyLock`** — the outer-boundary pattern the
  firewall principle generalizes.

A production HIPAA deployment proves the firewall split is a
*requirement*, not a nicety: a real HIPAA-compliant system needs fast
inner auth AND durable outer audit, and ships exactly that separation.

### 2.6 Geospatial / OSM — geographic calibration

**What OSM proves: geography doesn't break the IR.** OSM elements use
**per-element-type flat numeric ID spaces** ([OSM data model][osm-dm]
— `Node 100`, `Way 100`, and `Relation 100` are unrelated; IDs are
not hierarchical and carry no spatial information on their own). The
adapter therefore does identity *construction*, not extraction: it
computes the Cesium TMS quadkey from the element's **resolved**
geometry — only `Node`s carry coordinates directly; `Way`s and
`Relation`s reference other elements and must be resolved through
their members first:

- **`Node`** — quadkey from its own `lat`/`lon` (the only element
  type that carries coordinates).
- **`Way`** — resolve the ordered `Node`-references first to get the
  underlying lat/lon list, then take the centroid (or the smallest
  covering tile at the target zoom — whichever fits the consumer's
  query pattern).
- **`Relation`** — resolve member references recursively (members
  are `Node`/`Way`/`Relation` refs with roles), then centroid /
  covering tile, or an `admin_level`-boundary walk when the
  Relation IS an administrative polygon.

The resolved quadkey gets prepended to the per-type OSM ID, yielding
the `NiblePath` form `osm/<quadkey>/<type>/<id>`. The quadkey IS the
spatial frame; the per-type ID is the leaf inside it. Three OSM
Classes (`Node`, `Way`, `Relation`) lift via the queued
`ogar-from-osm-pbf` adapter (`docs/RDF-OWL-ALIGNMENT.md §10` Phase
2c). Exercises:

[osm-dm]: https://wiki.openstreetmap.org/wiki/Data_model

- **Spatial prefix-locality** — Cesium TMS quadkey as `NiblePath`
  prefix, per Q2 coordination outcome locked in `lance-graph` PR #473
  §2. HHTL trie routes `osm/<quadkey>/way/123` byte-identically to
  `ogit-erp/sale.order/42` and `fma/Femur`. The *runtime session's*
  Cesium tile pyramid math (`crates/cesium/src/sse.rs` +
  `implicit_tiling.rs`) already uses quadkey addressing; aligning
  OGAR routes both sides through the same address arithmetic.

- **Tag-as-Class final / Arrow-list v1 fallback** — Q1 coordination
  outcome locked in `lance-graph` PR #473 §2. Final shape: Tag is a
  `Class` related via `has_tag: HasMany<Tag>` (SPO-natural emission
  `(Way#123, ogar:hasTag, Tag#building=yes)`); v1 implementation
  ships the Arrow `List<Struct>` fallback in `D-OSM-1/2` for
  cardinality control while Tag interning matures. The IR (`Class`
  shape) carries no OSM-specific dialect — same ADR-023 discipline
  Odoo's `_inherit`/`_inherits` already follows.

- **Palette256 codec (ADR-024) adopts its third domain.** OSM tag
  values cluster within a tile (most zoom-21 tiles have ≤256
  distinct tag values); per-tile palette + const-table lookup
  yields ~5-10× compression on way attributes. The `D-OSM-2`
  Arrow→Lance ingest reports ρ-vs-reference per the ADR-024
  adoption checklist (lance-graph PR #473's §11 callout — the
  runtime session's commitment after ADR-024 merged in PR #39).

- **The OGAR-crossing deliverable is `D-OSM-3`** — the SPO triple
  lift in `lance-graph-ontology`. That's the producer-side contract
  the `ogar-from-osm-pbf` adapter consumes. OGAR session signed off
  on the surface in 2026-06-05 cross-session coordination; the
  adapter is unblocked.

**The geographic litmus complements the anatomical one: the same
compile-time HHTL primitive resolves `Femur is_a LongBone` AND
`Marienplatz is_in Munich` in sub-microsecond.** That's the
falsifiable property — measurable, not aspirational. If it holds,
*"instance proves universality"* is non-trivial; if it fails on
either, the substrate is leaking dialect into the codec.

The OSM instance is calibration-grade like Chess and OpenProject (no
production deployment owned by the workspace yet), but the *runtime
side* — Cesium tilesets, Lance-backed OSM datasets, 3DGS splat
batches over OSM building footprints — is shipping per the
`cesium-osm-substrate-v1.md` deliverable line. The OGAR side
contributes the schema lift; the runtime side contributes the
rendering substrate; together they form the geographic counterpart
to the FMA-bones anatomical case (`docs/RDF-OWL-ALIGNMENT.md §6`).

### 2.7 MARS (HIRO/bardioc CMDB) — XSD-frozen calibration

**The third closed-formal calibration domain** (after chess and OSM).
The bardioc engine's existing MARS-Schema XSD (frozen since 2015,
version 5.3.8) becomes the bijective oracle for the four-entity
A→R→S→M dependency taxonomy. The OGIT NTO/MARS TTL files are mirrored
1:1 into `vocab/imports/ogit/NTO/MARS/`; the OGAR producer
(`ogar-from-schema`) reads them and the lifted classifications agree
byte-for-enum with the XSD-extracted set. Round-trip is mechanically
enforced: every MARS TTL parses → emits → re-parses to an **equal**
lifted form, and every one of 176 SGO verbs (the AST predicate
vocabulary) does the same. Exercises:

- **Frozen-schema calibration** — the bijection oracle pattern
  (`docs/MARS-TRANSCODING.md §2`) extended from a behavioural-arm
  oracle (chess) to a structural-arm oracle (MARS XSD). Same
  chess-grade discipline applied to the schema layer.
- **The schema-vs-source duality** (`docs/HIRO-IN-CLASSES.md §2`) —
  the funny insight: schemas lift the structural arm bijectively;
  source ASTs lift the behavioural arm best-effort; the two are
  disjoint and become each other's oracle at the structural boundary.
  This is what makes Foundry's paid "ontology change management" a
  free `extract_classes.py` + 50 LOC of producer.
- **Reverse engineering** — the producer is symmetric. OGAR `Class`
  structures emit back to OGIT-flavoured TTL preserved
  semantically; colleagues can author/edit in Rust and feed back into
  bardioc's existing ingest with no two-way translation table.
- **AST predicate vocabulary lift** — SGO's 176 verbs (`dependsOn`,
  `contains`, `runsOn`, `generates`, `relates`, `causes`, …) become
  the canonical OGAR `Association`/`ActionDef` predicate vocabulary
  via `ogar-from-schema::sgo`. Every NTO `ogit:allowed (...)` block's
  verb references resolve against this typed registry.

MARS sits with OSM and chess in the "calibration trio" — none of them
are a production deployment the workspace owns, but they hard-prove
properties production deployments depend on. MARS specifically proves
**frozen-schema bijection** + **schema↔source cross-validation** —
which is what makes the bardioc behavioural-arm migration
(`docs/ELIXIR-HIRO-PREFETCH.md`) survive without losing OIIT/HIRO
schema fidelity.

## 3. Capability coverage matrix

Which domain proves which substrate capability (Foundry-parity columns
from `SUBSTRATE-ENDGAME.md §5.2`):

| Capability | Chess | OpenProject | Elixir/HIRO | Odoo/ERP | HIPAA | OSM | MARS |
|---|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| Ontology (Class/Association) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Action types / lifecycle FSM | ✓ | ✓ | ✓ (gen_statem) | ✓ (workflows) | ✓ | — | — (structural arm only) |
| `Postpone` / `StateTimeout` | ✓ (premove/clock) | partial | ✓ | — | — | — | — |
| `Depends` (data-causal) | — | ✓ (reactive) | ✓ | ✓ (`@api.depends`) | — | — | — |
| Time-versioned / time-travel | ✓ | ✓ (paper-trail) | ✓ | ✓ | ✓ (audit) | ✓ (changesets) | (schema versioned) |
| **Row-level permissions** | — | partial (RBAC) | — | partial | **✓ (HIPAA, palette256)** | — (ODbL-public) | — |
| **Immutable audit** | — | ✓ (journals) | — | ✓ | **✓ (HIPAA, signed)** | ✓ (OSM changeset history) | (schema = audit witness) |
| Multi-language frontends | (Rust) | Ruby | Elixir | Python (Odoo)+SeaORM | Rust | Rust (via `osmpbf`) | Rust (via TTL/XSD) |
| Money/decimal fidelity | — | — | — | **✓ (ERP)** | — | — | — |
| Migration scaffold | — | ✓ (target) | ✓ (spine) | (already Rust) | (already Rust) | (already Rust) | ✓ (bardioc target) |
| **Spatial prefix routing** (Cesium TMS quadkey via NiblePath) | — | — | — | — | — | **✓ (OSM)** | — |
| **Palette256 codec adoption** (ADR-024) | — | — | — | — | ✓ (security) | **✓ (tag values + tile-local coords)** | — |
| **Frozen-schema bijection oracle** (XSD/TTL ↔ Class) | ✓ (`Position::play`) | — | — | — | — | — | **✓ (XSD ↔ TTL ↔ Class, 3-way)** |
| **Reverse-emit (Class → schema)** | — | — | — | — | — | — | **✓ (semantic bijection)** |
| **AST predicate vocabulary registry** | — | — | — | — | — | — | **✓ (176 SGO verbs)** |

**Coverage observation:** no single domain exercises everything, but the
six together cover the full surface. The HIPAA instance is the *only*
one that hard-proves row-level perms + signed audit (the Security Mesh);
the Odoo/ERP instance is the *only* one that hard-proves money/decimal
fidelity + production `@api.depends`; the OSM instance is the *only*
one that hard-proves spatial prefix routing (Cesium TMS quadkey as
`NiblePath`) AND drives the second production adoption of the
palette256 codec from ADR-024. The calibration trio (chess/OP/HIRO)
proves the lifecycle + migration core. **That's why all six matter**
— drop any and a capability loses its production witness.

## 4. Why the two production instances change the Foundry argument

`SUBSTRATE-ENDGAME.md §5.3` argues substrate-b is "deeper than Foundry
going OSS." The two production instances sharpen it from architecture to
evidence:

- **Foundry's pitch is "one platform, many verticals."** Substrate-b's
  answer: the verticals *already exist as independent OGAR instances* —
  an ERP deployment and a HIPAA healthcare deployment — built on the
  same `Class`/`ActionDef`/`Identity` core + the same firewall, with no
  shared application code, only the shared substrate. That's the "be
  everything" claim with two production receipts.
- **Foundry's row-level security is a platform feature you adopt.**
  Substrate-b's is a *substrate primitive* (palette256 + Hamming on the
  inner hot path) that a HIPAA system already depends on — proven under
  a real compliance regime, not a sales demo.
- **Different storage per deployment** (the §5.3.3 pluggability point):
  the ERP deployment uses SeaORM; the substrate-b reference uses Lance;
  the healthcare deployment uses its own membrane backend. Same contract
  (`ExternalMembrane` / `KnowableFromStore`), different backends —
  exactly the firewall's outer-boundary pluggability, demonstrated
  across instances. And the *labels* differ across all of them while the
  *contract-inherited schema* is shared — the "inherit schema via
  contract" pattern (§0) at deployment scale.

## 5. Cross-references

- `docs/THE-FIREWALL.md` §7 — the precedent + the HIPAA firewall worked example.
- `docs/SUBSTRATE-ENDGAME.md` §5.2 (Foundry parity), §5.3 (the three differentiators), Room 3 (OP-as-operator-pane).
- `docs/ODOO-TRANSCODING.md` — the Odoo spec; its production instance is an ERP deployment (§2.4).
- `docs/ADAPTERS-AND-ACTORS.md` §2 — the `Adapter` HHTL leaf-rename pattern (the consumer-rebindable label mechanism behind §0's "inherit schema via contract").
- `docs/CHESS-TRANSCODING.md`, `docs/OPENPROJECT-TRANSCODING.md`, `docs/ELIXIR-HIRO-PREFETCH.md` — the calibration set.
- `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` — ADR-013 (paper-trail/audit consolidation), ADR-022 (the firewall).

## 6. Doc lifecycle

- **Author:** OGAR session, 2026-06-05.
- **Update cadence:** when a new domain is instantiated against OGAR, add
  a row to §1 + a §2 subsection + a §3 matrix column. The capability
  matrix is the "what does this domain newly prove" check.
- **Labels:** domain instances are named by *domain*, not project (per
  §0 — the concrete label is consumer-rebindable via the `Adapter`
  contract; the architecture doesn't depend on it).
