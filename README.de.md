<img src="docs/assets/ogar-logo.svg" alt="OGAR" height="64"/>

# OGAR — Open Graph of Active Record

> [English](README.md) · **Deutsch** · [Philosophie](docs/PHILOSOPHIE.md)

Das [Active-Record-Muster](https://martinfowler.com/eaaCatalog/activeRecord.html)
(Martin Fowler, 2003) als kanonische 32-Bit-`classid`. Ein Codebook;
jeder Consumer zieht, prägt niemals neu.

```rust
let cid: u16  = HealthcarePort::class_id("Patient");        // 0x0901
let render    = HealthcarePort::APP_PREFIX | (cid as u32);  // 0x0005_0901
authorize(actor, cid, Op::Read);                            // geteilter Grant auf lo u16
//                                                          // lokal anreichern — deins
```

```
classid : u32  =  0xAAAA ‖ 0xDDCC                  beide Hälften sind ADRESSE
                    │         │
                    │         └─ lo u16 — WELCHES KONZEPT  (geteilte Identität)
                    └─────────── hi u16 — WESSEN RENDER     (App-eigene Haut)

         ──────►  löst auf zu  ──────►
            ├─ ClassView                die HAUT      (pro App)
            ├─ Class                    die FORM      (kanonisch)
            └─ ActionDef + KausalSpec   die MAGIE     (Verhalten — im Core,
                                                       nie in der Adresse)
```

## Active-Record-Querverweis

| OGAR-Vokabular                | Rails ActiveRecord                       | Odoo `models.Model`                                  |
|-------------------------------|------------------------------------------|------------------------------------------------------|
| `ogar/Class`                  | `class WorkPackage < ApplicationRecord`  | `class sale_order(models.Model): _name='sale.order'` |
| `ogar/Association(BelongsTo)` | `belongs_to :project`                    | `fields.Many2one('res.partner')`                     |
| `ogar/Association(HasMany)`   | `has_many :line_items`                   | `fields.One2many('sale.order.line', 'order_id')`     |
| `ogar/Association(HabTm)`     | `has_and_belongs_to_many :tags`          | `fields.Many2many('res.partner.category')`           |
| `ogar/Mixin`                  | `include Mentionable`                    | `_inherit = 'mail.thread'`                           |
| `ogar/Enum`                   | `enum status: { open: 0, closed: 1 }`    | `fields.Selection([('draft','Draft'), ...])`         |
| `ogar/Field`                  | `t.string :subject` (in Migration)       | `subject = fields.Char(required=True)`               |
| `ogar/Validation` ¹           | `validates :subject, presence: true`     | `@api.constrains('subject')`                         |
| `ogar/Callback` ¹             | `before_save :touch_parent`              | `@api.depends('field_x')` / `@api.onchange`          |
| `ogar/Scope`                  | `scope :open, -> { where(state:'open')}` | Such-Domain `[('state','=','open')]`                 |

¹ Validation/Callback sind **Verhalten** — `ActionDef` + `KausalSpec`,
erreicht *über* die classid, nie in ihr. Rails, Odoo, Sequel, Django,
Prisma — gleiches Vokabular, ein IR.

## Render-Linse — ein Konzept, viele Apps

| classid       | Konzept (lo u16)            | Render (hi u16)          | App                  |
|---------------|-----------------------------|--------------------------|----------------------|
| `0x0001_0102` | `0x0102` project_work_item  | `0x0001` OpenProject     | openproject-nexgen-rs |
| `0x0007_0102` | `0x0102` project_work_item  | `0x0007` Redmine         | (Consumer TBD)        |
| `0x0005_0901` | `0x0901` patient            | `0x0005` Medcare         | medcare-rs            |
| `0x0003_0103` | `0x0103` billable_work_entry | `0x0003` WoA            | woa-rs                |
| `0x0004_0103` | `0x0103` billable_work_entry | `0x0004` SMB            | smb-office-rs         |
| `0x0001_0103` | `0x0103` billable_work_entry | `0x0001` OpenProject    | openproject-nexgen-rs |
| `0x0007_0103` | `0x0103` billable_work_entry | `0x0007` Redmine        | (Consumer TBD)        |
| `0x0002_0103` | `0x0103` billable_work_entry | `0x0002` Odoo           | odoo-rs               |

Gleiche `0x0103` → gleicher RBAC-Grant, gleiche Ontologie, gleiche
Cross-Fork-Identität. Fünf Apps rendern *abrechenbare Zeit* auf fünf
Arten; Planer-Stunden stimmen mit der Abrechnung per Codebook-Lookup
überein, nicht per Übersetzung.

## Consumer-Muster — vier Züge, nur einer ist deiner

| Zug       | Aufruf                                     | Deiner?   |
|-----------|--------------------------------------------|-----------|
| pull      | `Port::class_id(name) -> Option<u16>`      | nein — reine Funktion |
| render    | `Port::APP_PREFIX \| (cid as u32)`         | nein — typisierter Helfer |
| authorize | `authorize(actor, cid, op)` ²              | nein — geteiltes Grant-Gitter |
| enrich    | deine Domänen-Logik, an `cid` verschlüsselt | **ja**   |

² `lance-graph-rbac::authorize` ist `[H]`, gegated auf
`PROBE-OGAR-RBAC-AUTHORIZE`. Bis es ausliefert, behalte die bestehende
Auth — führe KEINE `*Bridge` als Notlösung wieder ein.

## Design-Wächter

| Wächter | Lesen vor |
|---|---|
| [OGAR als IR](docs/OGAR-AS-IR.md) | Entwurf jeder IR-Erweiterung (neues `Class`-Feld, `ActionDef`-Variante, `KausalSpec`-Slot, Lowering-Pass) |
| [Consumer Best Practices](docs/OGAR-CONSUMER-BEST-PRACTICES.md) | jeder Consumer-Aufrufstelle (classid · `APP_PREFIX` · `ClassView` · `*Bridge`) |
| [SurrealQL-AST-Fallen-Pre-Flight](docs/SURREAL-AST-TRAP-PREFLIGHT.md) | Producer→IR · Transcode · Codegen · `.surql`-Authoring |
| [SurrealQL-AST als Adapter](docs/SURREAL-AST-AS-ADAPTER.md) | Entscheidung Rückgrat vs. Adapter |
| [Die Firewall](docs/THE-FIREWALL.md) | jeder Hot-Path-Serialisierung (ADR-022/023) |
| [APP‖class-Codebook-Layout](docs/APP-CLASS-CODEBOOK-LAYOUT.md) | Prägen einer classid oder eines App-Präfix |
| [classid-RBAC-Keystone](docs/CLASSID-RBAC-KEYSTONE-SPEC.md) | Verdrahten der Autorisierung |
| [Consumer-Migrations-Anleitung](docs/CONSUMER-MIGRATION-HOWTO.md) | Umzug eines Consumers weg von einer `*Bridge` |

Der Fallen-Pre-Flight (Producer-seitig) und der Best-Practices-Leitfaden
(Consumer-seitig) sind die zwei Arme einer Grenze: der Pre-Flight hält
einen *Producer* davon ab, DDL für den Core einzusetzen; der Best-
Practices-Leitfaden hält einen *Consumer* davon ab, den Core lokal neu
zu implementieren.

## Architektur

```
   ─── Quell-ASTs (Producer) ─────────────────────────────────────
   Ruby AR              Python Odoo            SQL DDL          ...
   lib-ruby-parser      libcst / ast           sqlparser-rs
       │                    │                    │
       └──────────┬─────────┴──────────┬─────────┘
                  ▼                    ▼
              OGAR IR (kanonisch)  ⟷  ogar-extensions/*
                  │   Class · ActionDef · Identity · KausalSpec
                  │
                  │  validiert von  lance-graph-ontology  (+ Cache)
                  │  geplant von    lance-graph-planner
                  │  gespeichert als SoA-spaltenförmig (Arrow IPC, append-only Lance)
                  ▼
              lance-graph-Tripel
                  │
                  ├─ Projektionen (Consumer — gleiches IR, andere Ziele):  ← ADAPTER
                  │     • SurrealQL DDL AST   (DEFINE TABLE / FIELD)        nur struktureller Arm;
                  │     • PostgreSQL DDL       (CREATE TABLE / Migration)   Verhaltens-Arm
                  │     • OpenAPI / JSON-Schema (API-Verträge)              lebt im Core
                  │     • TypeScript-Typen     (Frontend-Interfaces)        (Verhaltens-Arm: nur Core)
                  │     • Prisma / Drizzle     (andere ORMs)
                  │
                  └─ Laufzeit (Aktor-Schicht — Pragmatik):
                      lance-graph-callcenter (BEAM-Äquivalent über OGAR)
```

### Voller Schlüssel (16-Byte-Zoom)

```
key  =  classid(4)  │  HEEL · HIP · TWIG (6)  │  family(3)  │  identity(3)
        ▲              ▲                          ▲             ▲
        Klassen-       HHTL-Kaskaden-Tiers         Basin-        Instanz
        Adresse        (256×256 Zentroid-          Gruppierung   im Basin
        (dieses Doc)   Kacheln, O(1)-Distanz)
```

## Repository-Layout

```
OGAR/
├── crates/
│   ├── ogar-vocab/     — Rust-Typen: kanonisches IR + Codebook (class_ids, ports, APP_PREFIX)
│   └── ogar-ontology/  — Präfix-Konventionen, NiblePath-kompatible Identität
├── vocab/
│   ├── ogar.ttl        — Turtle/RDF-Kanonform
│   ├── ogar.json-ld    — JSON-LD-Kanonform  (geplant)
│   └── ogar.surql      — SurrealQL-DDL-Projektion (nur struktureller Arm)
└── docs/               — die sieben Wächter oben + das Discovery- / Integration- / ADR-Ledger
```

## Producer / Consumer

**Producer** (Quell-AST → OGAR IR): `ruff_ruby_spo` (Ruby AR, liefert in
[`openproject-nexgen-rs`](https://github.com/AdaWorldAPI/openproject-nexgen-rs)
aus) · `ogar-python` (Django + Odoo, geplant) · `ogar-sql-ddl` (geplant)
· `ogar-typescript` (geplant).

**Consumer** (OGAR IR → Ziel): `op-codegen-pipeline` (liefert aus) ·
`ogar-to-postgres` · `ogar-to-surrealql` · `ogar-to-openapi` ·
`ogar-to-typescript` (alle geplant). Alle folgen dem Consumer-Muster:
**pull · render · authorize · enrich**.

## Laufzeit — lance-graph-callcenter

OGIT ↔ HIRO ↔ OTP/BEAM ist *Ontologie ↔ Laufzeit ↔ Aktor-Substrat*. Das
OGAR-Analogon ist der Vier-Crate-`lance-graph`-Stack:

| Schicht                   | OGIT-Welt               | OGAR-Welt                                                          |
|---------------------------|-------------------------|--------------------------------------------------------------------|
| Substrat-Primitive        | (roher Graph)           | **lance-graph-contract** — NiblePath, Identität, Versionen, Codebook |
| Ontologie-Schicht + Cache | OGIT-Vokabular + Ext.   | **lance-graph-ontology** — OGAR registriert; schnelle Typ-Auflösung |
| Query / Plan              | HIRO-Query-Planner      | **lance-graph-planner** — ontologie-bewusste Traversierungs-Optimierung |
| Aktor-Laufzeit            | HIRO-Automation + BEAM  | **lance-graph-callcenter** — Dispatch an `ogar/Class`-Aktoren       |

`subClassOf` ist der Supervisions-Baum; Ontologie-Updates sind
Hot-Code-Reload. Die Klasse ist die Aktor-Spezifikation; der Aktor ist
die laufende Klasse.

## Status

**v0 — auslieferndes Codebook.** [`crates/ogar-vocab`](crates/ogar-vocab)
ist der sprachneutrale Lift der stabilen C17a–c-Form aus
`ruff_ruby_spo`. `class_ids`, `ports::*Port`, `APP_PREFIX` sind live;
die TTL- / SurrealQL-Projektionen in [`vocab/`](vocab) sind generierte
Artefakte (stabiles URI-Präfix `ogar/`). Vokabular-Repo zuerst,
Code-Crate zweitens — wie FOAF oder SKOS.

## Lizenz

MIT — siehe [`LICENSE`](LICENSE).
