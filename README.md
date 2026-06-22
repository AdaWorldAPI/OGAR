<img src="docs/assets/ogar-logo.svg" alt="OGAR" height="64"/>

# OGAR — Open Graph of Active Record

> **English** · [Deutsch](README.de.md) · [Philosophy](docs/PHILOSOPHY.md)

The [Active Record pattern](https://martinfowler.com/eaaCatalog/activeRecord.html)
(Martin Fowler, 2003) as a canonical 32-bit `classid`. One codebook;
every consumer pulls, never re-mints.

```rust
let cid: u16  = HealthcarePort::class_id("Patient");        // 0x0901
let render    = HealthcarePort::APP_PREFIX | (cid as u32);  // 0x0005_0901
authorize(actor, cid, Op::Read);                            // shared grant on lo u16
//                                                          // enrich locally — yours
```

```
classid : u32  =  0xAAAA ‖ 0xDDCC                  both halves are ADDRESS
                    │         │
                    │         └─ lo u16 — WHICH CONCEPT  (shared identity)
                    └─────────── hi u16 — WHOSE RENDER   (per-app skin)

         ──────►  resolves to  ──────►
            ├─ ClassView                the SKIN    (per-app)
            ├─ Class                    the SHAPE   (canonical)
            └─ ActionDef + KausalSpec   the MAGIC   (behaviour — in the Core,
                                                     never in the address)
```

## Active Record cross-walk

| OGAR                          | Rails ActiveRecord                       | Odoo `models.Model`                                  |
|-------------------------------|------------------------------------------|------------------------------------------------------|
| `ogar/Class`                  | `class WorkPackage < ApplicationRecord`  | `class sale_order(models.Model): _name='sale.order'` |
| `ogar/Association(BelongsTo)` | `belongs_to :project`                    | `fields.Many2one('res.partner')`                     |
| `ogar/Association(HasMany)`   | `has_many :line_items`                   | `fields.One2many('sale.order.line', 'order_id')`     |
| `ogar/Association(HabTm)`     | `has_and_belongs_to_many :tags`          | `fields.Many2many('res.partner.category')`           |
| `ogar/Mixin`                  | `include Mentionable`                    | `_inherit = 'mail.thread'`                           |
| `ogar/Enum`                   | `enum status: { open: 0, closed: 1 }`    | `fields.Selection([('draft','Draft'), ...])`         |
| `ogar/Field`                  | `t.string :subject` (migration)          | `subject = fields.Char(required=True)`               |
| `ogar/Validation` ¹           | `validates :subject, presence: true`     | `@api.constrains('subject')`                         |
| `ogar/Callback` ¹             | `before_save :touch_parent`              | `@api.depends('field_x')` / `@api.onchange`          |
| `ogar/Scope`                  | `scope :open, -> { where(state:'open')}` | search-domain `[('state','=','open')]`               |

¹ Validation/Callback are **behaviour** — `ActionDef` + `KausalSpec`,
reached *through* the classid, never inside it. Rails, Odoo, Sequel,
Django, Prisma — same vocabulary, one IR.

## Render lens — one concept, many apps

| classid       | concept (lo u16)            | render (hi u16)          | app                  |
|---------------|-----------------------------|--------------------------|----------------------|
| `0x0001_0102` | `0x0102` project_work_item  | `0x0001` OpenProject     | openproject-nexgen-rs |
| `0x0007_0102` | `0x0102` project_work_item  | `0x0007` Redmine         | (consumer TBD)        |
| `0x0005_0901` | `0x0901` patient            | `0x0005` Medcare         | medcare-rs            |
| `0x0003_0103` | `0x0103` billable_work_entry | `0x0003` WoA            | woa-rs                |
| `0x0004_0103` | `0x0103` billable_work_entry | `0x0004` SMB            | smb-office-rs         |
| `0x0001_0103` | `0x0103` billable_work_entry | `0x0001` OpenProject    | openproject-nexgen-rs |
| `0x0007_0103` | `0x0103` billable_work_entry | `0x0007` Redmine        | (consumer TBD)        |
| `0x0002_0103` | `0x0103` billable_work_entry | `0x0002` Odoo           | odoo-rs               |

Same `0x0103` → same RBAC grant, same ontology, same cross-fork
identity. Five apps render *billable time* five ways; planner hours
align with billing by codebook lookup, not translation.

## Consumer pattern — four moves, only one is yours

| Move      | Call                                       | Yours?    |
|-----------|--------------------------------------------|-----------|
| pull      | `Port::class_id(name) -> Option<u16>`      | no — pure function |
| render    | `Port::APP_PREFIX \| (cid as u32)`         | no — typed helper |
| authorize | `authorize(actor, cid, op)` ²              | no — shared grant lattice |
| enrich    | your domain logic keyed on `cid`           | **yes**   |

² `lance-graph-rbac::authorize` is `[H]`, gated on `PROBE-OGAR-RBAC-AUTHORIZE`.
Until it ships, keep existing auth — do NOT re-introduce a `*Bridge` as a stopgap.

## Design guards

| Guard | Read before |
|---|---|
| [OGAR as IR](docs/OGAR-AS-IR.md) | designing any IR addition (a new `Class` field, `ActionDef` variant, `KausalSpec` slot, lowering pass) |
| [Consumer best practices](docs/OGAR-CONSUMER-BEST-PRACTICES.md) | any consumer call site (classid · `APP_PREFIX` · `ClassView` · `*Bridge`) |
| [SurrealQL-AST trap pre-flight](docs/SURREAL-AST-TRAP-PREFLIGHT.md) | producer→IR · transcode · codegen · `.surql` authoring |
| [SurrealQL AST as adapter](docs/SURREAL-AST-AS-ADAPTER.md) | deciding spine vs. adapter |
| [The Firewall](docs/THE-FIREWALL.md) | any hot-path serialization (ADR-022/023) |
| [APP‖class codebook layout](docs/APP-CLASS-CODEBOOK-LAYOUT.md) | minting a classid or app prefix |
| [classid-RBAC keystone](docs/CLASSID-RBAC-KEYSTONE-SPEC.md) | wiring authorization |
| [Consumer migration how-to](docs/CONSUMER-MIGRATION-HOWTO.md) | moving a consumer off a `*Bridge` |

The trap pre-flight (producer-side) and the best-practices guide
(consumer-side) are the two arms of one boundary: the pre-flight stops
a *producer* substituting DDL for the Core; the best-practices guide
stops a *consumer* re-implementing the Core locally.

## Architecture

```
   ─── source ASTs (producers) ──────────────────────────────────
   Ruby AR              Python Odoo            SQL DDL          ...
   lib-ruby-parser      libcst / ast           sqlparser-rs
       │                    │                    │
       └──────────┬─────────┴──────────┬─────────┘
                  ▼                    ▼
              OGAR IR (canonical)  ⟷  ogar-extensions/*
                  │   Class · ActionDef · Identity · KausalSpec
                  │
                  │  validated by  lance-graph-ontology  (+ cache)
                  │  planned by    lance-graph-planner
                  │  stored as     SoA columnar (Arrow IPC, append-only Lance)
                  ▼
              lance-graph triples
                  │
                  ├─ projections (consumers — same IR, other targets):  ← ADAPTERS
                  │     • SurrealQL DDL AST   (DEFINE TABLE / FIELD)    structural arm only;
                  │     • PostgreSQL DDL       (CREATE TABLE / migration) behavioural arm
                  │     • OpenAPI / JSON-Schema (API contracts)           lives in the Core
                  │     • TypeScript types     (frontend interfaces)      (behaviour arm: Core only)
                  │     • Prisma / Drizzle     (other ORMs)
                  │
                  └─ runtime (actor layer — pragmatics):
                      lance-graph-callcenter (BEAM-equivalent over OGAR)
```

### Full key (16-byte zoom)

```
key  =  classid(4)  │  HEEL · HIP · TWIG (6)  │  family(3)  │  identity(3)
        ▲              ▲                          ▲             ▲
        class addr     HHTL cascade tiers         basin         instance
        (this doc)     (256×256 centroid tiles,   grouping      within basin
                       O(1) distance lookup)
```

## Repository layout

```
OGAR/
├── crates/
│   ├── ogar-vocab/     — Rust types: canonical IR + codebook (class_ids, ports, APP_PREFIX)
│   └── ogar-ontology/  — prefix conventions, NiblePath-compatible identity
├── vocab/
│   ├── ogar.ttl        — Turtle/RDF canonical form
│   ├── ogar.json-ld    — JSON-LD canonical form  (planned)
│   └── ogar.surql      — SurrealQL DDL projection (structural arm only)
└── docs/               — the seven guards above + the discovery / integration / ADR ledger
```

## Producers / consumers

**Producers** (source AST → OGAR IR): `ruff_ruby_spo` (Ruby AR, shipping
in [`openproject-nexgen-rs`](https://github.com/AdaWorldAPI/openproject-nexgen-rs))
· `ogar-python` (Django + Odoo, planned) · `ogar-sql-ddl` (planned) ·
`ogar-typescript` (planned).

**Consumers** (OGAR IR → target): `op-codegen-pipeline` (shipping) ·
`ogar-to-postgres` · `ogar-to-surrealql` · `ogar-to-openapi` ·
`ogar-to-typescript` (all planned). All follow the consumer pattern:
**pull · render · authorize · enrich**.

## Runtime — lance-graph-callcenter

OGIT ↔ HIRO ↔ OTP/BEAM is *ontology ↔ runtime ↔ actor substrate*. The
OGAR analogue is the four-crate `lance-graph` stack:

| Layer                   | OGIT-world              | OGAR-world                                                          |
|-------------------------|-------------------------|--------------------------------------------------------------------|
| Substrate primitives    | (raw graph)             | **lance-graph-contract** — NiblePath, identity, versions, codebook |
| Ontology layer + cache  | OGIT vocab + extensions | **lance-graph-ontology** — OGAR registered; fast type-resolution   |
| Query / plan            | HIRO query planner      | **lance-graph-planner** — ontology-aware traversal optimisation    |
| Actor runtime           | HIRO automation + BEAM  | **lance-graph-callcenter** — dispatch to `ogar/Class` actors       |

`subClassOf` is the supervision tree; ontology updates are hot-code
reload. The class is the actor spec; the actor is the class running.

## Status

**v0 — shipping codebook.** [`crates/ogar-vocab`](crates/ogar-vocab) is
the language-neutral lift of the C17a–c stable shape from
`ruff_ruby_spo`. `class_ids`, `ports::*Port`, `APP_PREFIX` are live; the
TTL/SurrealQL projections in [`vocab/`](vocab) are generated artifacts
(stable URI prefix `ogar/`). Vocabulary repo first, code crate second —
like FOAF or SKOS.

## License

MIT — see [`LICENSE`](LICENSE).
