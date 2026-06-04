# OGAR — Open Graph of Active Record

The canonical graph-ontology and Rust toolkit for the [Active Record
pattern](https://martinfowler.com/eaaCatalog/activeRecord.html) — a
language-independent vocabulary that expresses any AR-shaped data model
(Rails, Odoo, Sequel, Django, Prisma, …) as a stable set of triples.

OGAR sits next to [OGIT](https://www.arago.co/ogit/) (Ontology for Graph
IT) in the same prefix-radix namespace: where OGIT covers IT-operations
semantics, OGAR covers the schema-as-class-declaration pattern that
underpins ERPs, project trackers, and any record-oriented business
application.

## Status

**v0 — vocabulary stub.** The Rust types in [`crates/ogar-vocab`](crates/ogar-vocab)
are the language-neutral lift of the C17a–c stable shape currently
shipping inside [`AdaWorldAPI/openproject-nexgen-rs`](https://github.com/AdaWorldAPI/openproject-nexgen-rs)'s
`ruff_ruby_spo`. The TTL / SurrealQL projections in [`vocab/`](vocab) are
generated from those types and intended as the cite-able canonical
artifacts (stable URI prefix `ogar/`).

This is a **vocabulary repo first, code crate second** — like FOAF or
SKOS, the ontology files are the public contract; the crate is a
producer/consumer convenience.

## Why OGAR

Active Record isn't a Ruby thing — it's a pattern Martin Fowler named in
2003, and every modern record-class ORM is a re-implementation of it.
Compare:

| OGAR vocabulary               | Rails ActiveRecord                       | Odoo `models.Model`                                  |
|-------------------------------|------------------------------------------|------------------------------------------------------|
| `ogar/Class`                  | `class WorkPackage < ApplicationRecord`  | `class sale_order(models.Model): _name='sale.order'` |
| `ogar/Association(BelongsTo)` | `belongs_to :project`                    | `fields.Many2one('res.partner')`                     |
| `ogar/Association(HasMany)`   | `has_many :line_items`                   | `fields.One2many('sale.order.line', 'order_id')`     |
| `ogar/Association(HabTm)`     | `has_and_belongs_to_many :tags`          | `fields.Many2many('res.partner.category')`           |
| `ogar/Mixin`                  | `include Mentionable`                    | `_inherit = 'mail.thread'`                           |
| `ogar/Enum`                   | `enum status: { open: 0, closed: 1 }`    | `fields.Selection([('draft','Draft'), ...])`         |
| `ogar/Validation`             | `validates :subject, presence: true`     | `@api.constrains('subject')`                         |
| `ogar/Callback`               | `before_save :touch_parent`              | `@api.depends('field_x')` / `@api.onchange`          |
| `ogar/Scope`                  | `scope :open, -> { where(state: 'open')}`| search-domain `[('state','=','open')]`               |
| `ogar/Field`                  | `t.string :subject` (in migration)       | `subject = fields.Char(required=True)`               |

The same vocabulary handles all of them. Application-specific overlays
(`ogit-op/*` for OpenProject, `ogit-erp/*` for Odoo business semantics,
`ogit-gitlab/*` for GitLab) extend OGAR with domain identifiers under
the same prefix-radix routing.

## Architecture layer-stack

```
                          lance-graph  (append-only Arrow/Lance storage)
                                  ▲
              ┌───────────────────┼───────────────────┐
              │                   │                   │
        ogit/IT/*           ogar/  +  ogit-op/*    ogit-erp/*  (Odoo)
        (HIRO baseline)    (Rails apps)            (ERP business)
              ▲                   ▲                   ▲
              │                   │                   │
         ITOM tools         OGAR crate            ERP extractors
                          (this repo)
                                  ▲
                        ┌─────────┴──────────┐
                        │                    │
                  ruff_ruby_spo       ogar-python (planned, for
                  (Ruby AR AST)        Django + Odoo models)
```

### Universal AST ↔ OGAR ↔ DLL AST

```
   ─── source ASTs (producers) ──────────────────────────────────
   Ruby AR              Python Odoo            SQL DDL          ...
   lib-ruby-parser      libcst/ast             sqlparser-rs
       │                    │                    │
       └──────────┬─────────┴──────────┬─────────┘
                  ▼                    ▼
              OGAR IR (canonical)  ⟷  ogar-extensions/*
                  │
                  │  validated by  lance-graph-ontology  (+ cache)
                  │  planned by    lance-graph-planner
                  │  stored as     SoA columnar (Arrow IPC, append-only Lance)
                  ▼
              lance-graph triples
                  │
                  ├─ projections (consumers — same IR, other targets):
                  │     • SurrealQL DDL AST   (DEFINE TABLE / FIELD)
                  │     • PostgreSQL DDL       (CREATE TABLE / migration)
                  │     • OpenAPI / JSON-Schema (API contracts)
                  │     • TypeScript types     (frontend interfaces)
                  │     • Prisma / Drizzle     (other ORMs)
                  │
                  └─ runtime (actor layer — pragmatics):
                      lance-graph-callcenter (BEAM-equivalent over OGAR)
```

The SurrealQL DLL AST is **both producer and consumer**: source-of-truth
in SurrealQL → OGAR → Rails/Odoo/PG, or source-of-truth in Rails AR →
OGAR → SurrealQL/PG. The IR is the meeting point.

## Repository layout

```
OGAR/
├── crates/
│   ├── ogar-vocab/       — Rust types: the canonical IR
│   └── ogar-ontology/    — prefix conventions, NiblePath-compatible identity
├── vocab/
│   ├── ogar.ttl          — Turtle/RDF canonical form
│   ├── ogar.json-ld      — JSON-LD canonical form  (planned)
│   └── ogar.surql        — SurrealQL DDL projection
└── docs/
    └── ARCHITECTURE.md   — the layer-stack writeup
```

## Producers (current and planned)

- **`ruff_ruby_spo`** (lives in `AdaWorldAPI/openproject-nexgen-rs`,
  C17a–c stable): parses Ruby ActiveRecord models with
  `lib-ruby-parser`, emits OGAR-shaped IR. Closes 17/21 coverage-probe
  gaps over real OpenProject source.
- **`ogar-python`** (planned): Django + Odoo `models.Model` extractor
  via `libcst` / Python `ast`.
- **`ogar-sql-ddl`** (planned): SQL DDL via `sqlparser-rs`.
- **`ogar-typescript`** (planned): Prisma + TypeORM + Drizzle schema
  extractor.

## Consumers (current and planned)

- **`op-codegen-pipeline`** (lives in `AdaWorldAPI/openproject-nexgen-rs`):
  consumes OGAR IR, emits `ogit-op/*` triples into lance-graph.
- **`ogar-to-postgres`** (planned): OGAR → PostgreSQL DDL / Rails
  migrations.
- **`ogar-to-surrealql`** (planned): OGAR → SurrealQL `DEFINE TABLE` /
  `DEFINE FIELD`.
- **`ogar-to-openapi`** (planned): OGAR → OpenAPI / JSON:API contracts.
- **`ogar-to-typescript`** (planned): OGAR → `interface` declarations
  with zod schemas.

## The runtime — lance-graph-callcenter

OGIT ↔ HIRO ↔ OTP/BEAM is *ontology ↔ runtime ↔ actor substrate*. The
OGAR analogue is the four-crate `lance-graph` stack:

| Layer                   | OGIT-world              | OGAR-world (planned)                                                                  |
|-------------------------|-------------------------|---------------------------------------------------------------------------------------|
| Substrate primitives    | (raw graph)             | **lance-graph-contract** — NiblePath, identity, versions, append-only                 |
| Ontology layer + cache  | OGIT vocab + extensions | **lance-graph-ontology** — OGAR + extensions registered; fast type-resolution         |
| Query / plan            | HIRO query planner      | **lance-graph-planner** — ontology-aware traversal optimisation                       |
| Actor runtime           | HIRO automation + BEAM  | **lance-graph-callcenter** — dispatch messages to `ogar/Class` actors                 |

Each `ogar/Class` (e.g. `ogit-op/WorkPackage`) registers actors that
handle messages addressed to instances. `subClassOf` is the supervision
tree; ontology updates are hot-code reload.

**Same form, two modes**: business-data-classes (semantics — what is
true) and actors (pragmatics — what is done) are two views on the same
append-only log. The class is the actor spec; the actor is the class
running.

## License

MIT — see [`LICENSE`](LICENSE).
