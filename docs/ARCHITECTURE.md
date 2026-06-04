# OGAR Architecture

## The trichotomy that grounds the design

| Layer       | What it is                                  | Where it lives in OGAR                                              |
|-------------|---------------------------------------------|---------------------------------------------------------------------|
| Semantics   | Sign ↔ object — the nodes                   | `ogar-vocab` (Class, Association, Field …) + lance-graph triples    |
| Syntax      | Sign ↔ sign — the figure rules              | `ogar-ontology` prefix routing + `lance-graph-planner` query rules  |
| Pragmatics  | Sign ↔ interpreter — the wave               | `lance-graph-callcenter` actors over `ogar/Class` instances         |

Business data classes (semantics) and actors (pragmatics) are two views
on the same append-only log. The class IS the actor spec; the actor IS
the class running. The append-only log is the one substrate.

## The four lance-graph layers

```
┌─────────────────────────────────────────────────────────────────────┐
│  lance-graph-callcenter   — actor runtime (BEAM-equivalent)         │
│      dispatch messages to ogar/Class actors via ontology lookup     │
├─────────────────────────────────────────────────────────────────────┤
│  lance-graph-planner      — ontology-aware query planning           │
│      rewrites traversals using OGAR class hierarchy + figure rules  │
├─────────────────────────────────────────────────────────────────────┤
│  lance-graph-ontology     — ontology registry + cache               │
│      OGAR + ogit-op + ogit-erp + ogar-extensions all registered     │
│      fast resolution from identity string → class metadata          │
├─────────────────────────────────────────────────────────────────────┤
│  lance-graph-contract     — substrate primitives                    │
│      NiblePath (prefix-radix), identity, append-only Lance versions │
└─────────────────────────────────────────────────────────────────────┘
              ▲
              │
        Arrow/Lance columnar (SoA — Structure of Arrays)
              │
        append-only log on disk
```

## The OGAR ↔ OGIT ↔ HIRO ↔ BEAM analogy

| Aspect              | OGIT-world                       | OGAR-world                                          |
|---------------------|----------------------------------|-----------------------------------------------------|
| Substrate           | Graph storage (Graphit)          | lance-graph-contract (append-only Lance, NiblePath) |
| Ontology            | OGIT vocabulary                  | OGAR vocabulary + ogit-erp + ogit-op + ...          |
| Runtime             | HIRO automation engine + BEAM    | lance-graph-callcenter (actor-per-class)            |
| Identity            | OGIT URIs                        | `ogar/Class` / `ogit-<app>/Class` / dotted names    |
| Process model       | OTP processes                    | Actor per ogar/Class                                |
| Supervision         | OTP supervision tree             | `subClassOf` hierarchy                              |
| Hot-code reload     | BEAM module reload               | Ontology update (append new triples)                |
| Message routing     | BEAM `!`/`receive`               | callcenter dispatch via ontology lookup             |
| Replication         | OTP `gen_server` + Erlang distr. | Raft over Lance versions (append-only commit)       |

Distributed cognition replicates the **generator** (frozen semantics +
syntax), not the **wave** (pragmatics). Each peer re-runs the actors
locally from the replicated frozen layers. That is why "a thought is a
Raft commit": both halves are append-only writes to the same log.

## Universal AST ↔ OGAR ↔ DLL AST

```
                 ┌── Ruby AR AST ────────────┐
                 │   lib-ruby-parser         │
                 │   (ruff_ruby_spo)         │
                 └────────────┬──────────────┘
                              │
                 ┌── Python Odoo / Django AST ┐
                 │   libcst / ast            │
                 │   (ogar-python, planned)  │
                 └────────────┬──────────────┘
                              │
                 ┌── SQL DDL AST ────────────┐
                 │   sqlparser-rs            │
                 │   (ogar-sql-ddl, planned) │
                 └────────────┬──────────────┘
                              │
                              ▼
                       ┌─────────────┐
                       │  OGAR IR    │   ←──→  ogar-extensions/* (Odoo
                       │  (canonical)│            ComputedField,
                       └──────┬──────┘            Delegation, Workflow)
                              │
                              ▼
                  ┌───────────────────────┐
                  │  lance-graph-ontology │  validates IR against schema
                  └───────────┬───────────┘
                              ▼
                  ┌───────────────────────┐
                  │  lance-graph-contract │  store as Arrow SoA columnar
                  └───────────┬───────────┘
                              │
            ┌─────────────────┼──────────────────────┬─────────────────┐
            ▼                 ▼                      ▼                 ▼
       SurrealQL DDL    PostgreSQL DDL          OpenAPI/JSON-LD    TypeScript
       (ogar-to-       (ogar-to-postgres,       (ogar-to-openapi,  interfaces +
        surrealql,      planned)                 planned)          zod schemas
        planned)
```

SurrealQL DDL AST is **both producer and consumer** — the IR is a
two-way meeting point. SurrealDB used as a frontend DSL over lance-graph
gets the dev-facing query layer without storage duplication.

## Why one append-only substrate

The four layers above all read and write the **same** log. There is no
separate "events store" plus "current-state cache" plus "schema
registry" plus "audit log" — there is one Lance dataset with append-only
versions. Each layer is a different access path over the same physical
data:

- The contract layer reads bytes.
- The ontology layer reads the latest `ogar:Class` and `ogar:Association`
  triples to know what shapes exist.
- The planner reads the same triples plus figure-rule annotations to
  optimise.
- The callcenter reads the same triples plus actor registrations to
  dispatch.

Schema, instances, and execution history are uniformly addressed,
uniformly replicated, uniformly auditable.

## Compression to the floor

OGAR identity is prefix-radix (`ogit-op/WorkPackage` shares the
`ogit-op/` segment with every other OpenProject class), so the on-disk
representation compresses to the floor a Lance segment can reach.
Combined with `NiblePath`-style radix indexing, the same single-node
deployment that holds a planet-scale ontology (Wikidata fits) holds
every modelled business object plus its versioning history with room to
spare. Cluster-by-choice, not by capacity.

## What this enables

- **One source of truth per class** — no migration ↔ model drift, no API
  ↔ DB skew, no TS ↔ Ruby skew. Class definition compiles to N
  read-only projections (PG DDL, OpenAPI, TS, SurrealQL).
- **Cross-system queries** — `SaleOrder.workPackage` joins Odoo and
  OpenProject in one lance-graph traversal because both extend OGAR
  under the same prefix-radix.
- **Actor isolation per class** — `Project.find(id)` routes through the
  callcenter to the registered `ogit-op/Project` actor without the
  developer changing syntax.
- **Hot ontology updates** — append a new `ogar:Class` triple, the
  callcenter routes new messages to the new actor version; old
  in-flight messages drain to the old actor; both are recoverable from
  the version log.
