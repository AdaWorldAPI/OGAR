# OGAR Vision

> The Open Graph of Active Record. One vocabulary, one substrate, every
> record-shaped business application.

## The one-sentence form

**OGAR is the ActiveRecord pattern lifted from any source language into
a stable graph ontology, stored on an append-only Lance log under the
same prefix-radix routing as OGIT, runnable as actors over the same
log.**

## The trichotomy that grounds the design

Charles Morris's sign trichotomy maps onto the four-layer lance-graph
stack with no slack:

| Layer       | Sign relation         | Where it lives in OGAR                                              |
|-------------|----------------------|---------------------------------------------------------------------|
| Semantics   | sign ↔ object         | `ogar-vocab` types (Class, Association, Field …) + lance-graph triples |
| Syntax      | sign ↔ sign           | `ogar-ontology` prefix routing + `lance-graph-planner` figure rules |
| Pragmatics  | sign ↔ interpreter    | `lance-graph-callcenter` actors over `ogar/Class` instances         |

The class IS the actor spec. The actor IS the class running. Business-data
(semantics — what is true) and actors (pragmatics — what is done) are two
views on the same append-only log.

## The substrate is one append-only log

Identity (HHTL / NiblePath prefix-radix), the arc (write-versions), replication
(Raft over Lance append), metacognition (the version log), and the stop
(compression to the floor) are facets of a **single append-only log**, at
scales from one record (~6 bytes) to one planet (Wikidata on one node).

Replication ships the **generator** (frozen semantics + syntax + version log
dump), not the live meaning. Each peer re-runs pragmatics locally from the
replicated frozen layers. **Distributed cognition is free** because pragmatics
isn't replicated — it's re-run.

## The four lance-graph layers

```
┌─────────────────────────────────────────────────────────────────────┐
│  lance-graph-callcenter   — actor runtime  (BEAM-equivalent)        │
│      dispatch messages to ogar/Class actors via ontology lookup     │
├─────────────────────────────────────────────────────────────────────┤
│  lance-graph-planner      — ontology-aware query planning           │
│      rewrites traversals using OGAR class hierarchy + figure rules  │
├─────────────────────────────────────────────────────────────────────┤
│  lance-graph-ontology     — ontology registry + cache               │
│      OGAR + ogit-op + ogit-erp + ogar-extensions all registered     │
├─────────────────────────────────────────────────────────────────────┤
│  lance-graph-contract     — substrate primitives                    │
│      NiblePath (prefix-radix), identity, append-only Lance versions │
└─────────────────────────────────────────────────────────────────────┘
        Arrow/Lance columnar (SoA — Structure of Arrays)
        append-only log on disk
```

## The OGIT ↔ HIRO ↔ BEAM analogue

| Aspect          | OGIT-world             | OGAR-world                                          |
|-----------------|------------------------|-----------------------------------------------------|
| Substrate       | Graphit                | lance-graph-contract                                |
| Ontology        | OGIT vocab             | OGAR + ogit-erp + ogit-op + ogar-extensions/*       |
| Runtime         | HIRO + OTP/BEAM        | lance-graph-callcenter (actor-per-class)            |
| Process model   | OTP processes          | Actor per `ogar/Class`                              |
| Supervision     | OTP supervision tree   | `subClassOf` hierarchy                              |
| Hot reload      | BEAM module reload     | Ontology update (append new triples)                |
| Replication     | OTP + Erlang distr.    | Raft over Lance versions                            |

## Universal AST ↔ OGAR ↔ DLL AST

```
 ┌── Ruby AR AST ────┐  ┌── Python Odoo/Django ──┐  ┌── SQL DDL ─────┐
 │  lib-ruby-parser  │  │  libcst / ast           │  │  sqlparser-rs  │
 │  (ruff_ruby_spo)  │  │  (ogar-python, planned) │  │  (planned)     │
 └─────────┬─────────┘  └────────────┬────────────┘  └────────┬───────┘
           └────────────────┬────────┴───────────────────────┘
                            ▼
                    ┌─────────────┐
                    │  OGAR IR    │ ⟷ ogar-extensions/* (Odoo
                    │ (canonical) │     ComputedField, Delegation,
                    └──────┬──────┘     Workflow)
                           ▼
              ┌────────────────────────┐
              │  lance-graph-ontology  │
              │  lance-graph-contract  │
              └────────────┬───────────┘
                           ▼
         ┌─────────────────┼─────────────────────┐
         ▼                 ▼                     ▼
   SurrealQL DDL    PostgreSQL DDL         TypeScript / OpenAPI
   (bidirectional)  (Rails migrations)     (frontend / API contract)
```

**SurrealQL DDL AST is both producer and consumer** — IR is a two-way
meeting point. SurrealDB as frontend DSL over lance-graph gives the
dev-facing query layer with no storage duplication.

## Why this matters

- **One source of truth per class.** No migration ↔ model drift, no API
  ↔ DB skew, no TS ↔ Ruby skew. Class definition compiles to N
  read-only projections.
- **Cross-system queries.** `SaleOrder.workPackage` joins Odoo and
  OpenProject in one traversal because both extend OGAR under shared
  prefix-radix.
- **Actor isolation per class.** `Project.find(id)` routes through the
  callcenter to the registered `ogit-op/Project` actor — developer
  syntax unchanged.
- **Hot ontology updates.** Append new `ogar:Class` triple → callcenter
  routes new messages to new actor version; old in-flight messages
  drain to old actor; both recoverable from the log.

## Through-line

**Append-only is the one substrate. OGAR is the one grammar. The
callcenter is the one runtime.** Cross-language (Ruby + Python + SQL +
TS), cross-storage-format (lance-arrow primary, SurrealQL/PG as
projections), cross-system (OpenProject + Odoo + every other Rails or
Django app) — one queryable graph surface from 6 bytes (one record) to
one planet (Wikidata on one node).

## Cross-references

- `docs/ARCHITECTURE.md` — the longer architectural writeup
- `.claude/PLAN.md` — meticulous sprint-by-sprint roadmap
- `.claude/AGENTS.md` — extension rules + producer/consumer contract
- `.claude/board/EPIPHANIES.md` — append-only findings log
- `vocab/ogar.ttl` — the canonical Turtle/RDF vocabulary
- `vocab/ogar.surql` — the SurrealQL DDL projection
- `crates/ogar-vocab/` — the canonical Rust IR types
- `crates/ogar-ontology/` — prefix conventions + identity helpers
