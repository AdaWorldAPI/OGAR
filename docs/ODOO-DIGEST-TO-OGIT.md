# Odoo digest → OGIT TTL templates → agnostic relive

> **For colleagues building anything that needs Odoo as a typed ontology
> outside the Odoo Python runtime.** This is the architectural shape:
> digest Odoo source once into OGIT-shaped TTL templates (stored
> in-tree at `vocab/exports/ogit/NTO/<Domain>/`), then relive any model
> or workflow action agnostically via `ogar-render-askama`.
>
> Status: **FRAMING v0** (2026-06-22). Companion to
> `docs/ODOO-TRANSCODING.md` (the producer spec) and
> `docs/VERB-AS-CLASS-TEMPLATE.md` (the askama-template framing this
> reuses).

The shape in one sentence: **the `ruff_python_spo + ogar-from-ruff`
pipeline digests Odoo source once into the OGAR IR; `ttl_emit` writes
the IR back as OGIT-shaped TTL templates stored at
`vocab/exports/ogit/NTO/<Domain>/`; consumers re-instantiate any model
or workflow action by rendering against the TTL via
`ogar-render-askama`, never touching Odoo Python.**

> **Producer naming correction (2026-06-22):** earlier drafts of this
> doc called the producer "`ogar-from-python`" — that name is wrong.
> The correct pipeline is the existing **`ruff_python_spo`** AST
> frontend (sibling of `ruff_ruby_spo` / `ruff_elixir_spo` in the
> `ruff/` workspace) producing `ruff_spo_triplet::Model`, then the
> existing **`ogar-from-ruff`** crate mechanically projecting that IR
> into `ogar_vocab::Class`. A `ruff_python_spo` frontend is queued
> (the projector `ogar-from-ruff` already exists and works for Ruby).
> Building a new `ogar-from-python` from scratch would duplicate the
> projection that's already shipping.
>
> **Storage location correction (2026-06-22):** earlier drafts said
> digests land in `vocab/imports/ogit/NTO/<Domain>/`. They actually
> STAGE in **`vocab/exports/ogit/NTO/<Domain>/`** — produced-but-not-
> yet-promoted content — and only land in `imports/` after being
> promoted to the AdaWorldAPI/OGIT fork and re-vendored. See §2 for the
> staging-tier model and `vocab/exports/PROVENANCE.md` for the full
> rationale. (The same draft also wrongly claimed 11 existing
> Accounting files were "at re-vendor risk"; they are committed to the
> OGIT fork and correctly mirrored — see §2.)

---

## §1. The pipeline

```
   Odoo Python source
   (addons/<module>/models/*.py)
            │
            │  ruff_python_spo  (sibling of ruff_ruby_spo / ruff_elixir_spo;
            │                    queued — Python AST → ruff_spo_triplet::Model)
            │  — keeps structural arm: _name, _inherit, fields.*, selections
            │  — keeps behavioural-arm SIGNATURES (decorators, action def names)
            │  — drops bodies (computed methods, action implementations)
            ▼
   ruff_spo_triplet::Model
            │
            │  ogar-from-ruff  (existing — mechanical projection)
            ▼
   OGAR Class IR (in memory)
            │
            │  ttl_emit::emit_entity  (semantic bijection)
            │  — entity-as-class for models
            │  — verb-as-class for workflow action signatures
            ▼
   OGIT-shaped TTL templates  (STAGED — not yet promoted)
   (vocab/exports/ogit/NTO/<Domain>/<DigestedClass>.ttl)
   — dcterms:creator = bus-compiler  (digester provenance)
   — review here (round-trip + bijection tests, drift check)
            │
            │  PROMOTE: commit to the AdaWorldAPI/OGIT fork, then
            │           re-vendor (cp -r /OGIT/NTO/. vocab/imports/ogit/NTO/)
            ▼
   vocab/imports/ogit/NTO/<Domain>/  (faithful mirror of the enriched fork)
            │
            │  ogar-render-askama  (entity render → views; verb render → actions)
            ▼
   Materialized output (any consumer, any medium)
   — HTML view (per-app skin)
   — JSON / OpenAPI surface
   — SurrealQL DDL / Postgres CREATE TABLE
   — SPO triple emit + ACL gate + audit record (action render)
```

The Python runtime is **only** touched at digest time. Consumers
(`woa-rs`, `smb-office-rs`, `medcare-rs`, `q2`, any future renderer)
never depend on Odoo Python, only on TTL + the askama renderer — and
they read `imports/`, never the `exports/` staging tier.

## §2. The staging-tier model — `exports/` → OGIT fork → `imports/`

(Operator-decided 2026-06-22.) `exports/` is a **staging area** for
produced-but-not-yet-promoted content; the AdaWorldAPI/OGIT fork is
the **enriched canonical store**; `imports/` faithfully mirrors the
fork; consumers read only `imports/`. See `vocab/exports/PROVENANCE.md`
for the full model.

```
producer ──► vocab/exports/ogit/NTO/<Domain>/   (review/iterate, CI runs here)
                  │  promote (commit to OGIT fork on a branch, PR there)
                  ▼
             AdaWorldAPI/OGIT fork  (enriched: upstream + OGAR-promoted)
                  │  re-vendor
                  ▼
             vocab/imports/ogit/   (SHA-pinned mirror — consumers read this)
```

Why a staging tier and not "commit straight to the fork": a digest run
produces N TTLs at once; staging lets the round-trip + bijection tests
run, the drift-vs-prior-digest diff fire, and a human review the lift —
all inside OGAR + CI — **before** anything touches the shared fork. The
promote step is the single auditable gate between OGAR-produced and
OGIT-canonical.

**The 11 Accounting files are NOT a migration target.** An earlier
draft (commit `7d68042`) claimed 11 OGAR-produced TTLs in
`vocab/imports/ogit/NTO/Accounting/` were "at re-vendor risk" and
"belong in `exports/`." **That was wrong.** Those files are committed
to the **AdaWorldAPI/OGIT fork** (commit `c5dc1b8` "shrink 3-hop Odoo
lookups…", on `master`, pushed) — i.e. they are a *completed*
promotion. `imports/` correctly mirrors them; re-vendor preserves
them. They are the worked example of the staging-tier model run to
completion, not stranded content. `exports/` is for content that has
**not yet** made that trip.
list.

## §3. The four shapes the digester produces

| Shape | Source pattern | TTL form | Render path |
|---|---|---|---|
| **Entity-as-class** | `class Foo(models.Model): _name = '...'; <fields>` | `rdfs:Class` + `mandatory-attributes` enumerating field names | view render (`ogar-render-askama::views`) |
| **Datatype attribute** | `field_name = fields.Char/Integer/Selection(...)` (with selection) | `owl:DatatypeProperty` + `ogit:validation-type "fixed"` + `validation-parameter "a,b,c"` | binding validation |
| **Association** | `partner_id = fields.Many2one('res.partner', ...)` | `owl:ObjectProperty` (lifted as SGO verb) OR an entry in the parent class's `ogit:allowed (...)` block | edge in lifted SPO graph |
| **Verb-as-class** | `def action_confirm(self)` workflow method | `rdfs:Class` + slot list (subject = SaleOrder, object = confirmation event) + policy attrs (`requires-perm`, `emits-audit`) | action render (`ogar-render-askama::actions`, queued) |

The last shape is what makes "relive agnostically" possible for
workflow actions: the Python method body stays in Odoo (behavioural
arm), but the **action contract** (what it requires, what it emits,
what it depends on) becomes a typed template any consumer can render
against.

## §4. The mapping table — v0 from the existing codebook

The six commerce/ERP concepts already minted in `ogar-vocab::class_ids`
give the digester its first six targets. Concepts beyond these need
the 5+3 codebook pass (per `docs/APP-CLASS-CODEBOOK-LAYOUT.md §4`)
before mint; the digester can lift them in the meantime with
`Class.name` carrying the Odoo model name and no concept_id assigned.

| Odoo model | Source addon path | OGIT NTO target | OGAR concept | Shape | Concept-mint status |
|---|---|---|---|---|---|
| `res.partner` | `addons/base/models/res_partner.py` | `Accounting/` | `BILLING_PARTY` `0x0204` | entity-as-class | minted |
| `account.move` | `addons/account/models/account_move.py` | `Accounting/` | `COMMERCIAL_DOCUMENT` `0x0202` | entity-as-class | minted |
| `account.move.line` | `addons/account/models/account_move_line.py` | `Accounting/` | `COMMERCIAL_LINE_ITEM` `0x0201` | entity-as-class | minted |
| `account.tax` | `addons/account/models/account_tax.py` | `Accounting/` | `TAX_POLICY` `0x0203` | entity-as-class | minted |
| `account.payment` | `addons/account/models/account_payment.py` | `Accounting/` | `PAYMENT_RECORD` `0x0205` | entity-as-class | minted |
| `res.currency` | `addons/base/models/res_currency.py` | `Accounting/` | `CURRENCY_POLICY` `0x0206` | entity-as-class | minted |
| `sale.order` | `addons/sale/models/sale_order.py` | `SalesDistribution/` | TBD `0x02??` | entity-as-class | needs mint |
| `sale.order.line` | `addons/sale/models/sale_order_line.py` | `SalesDistribution/` | TBD | entity-as-class | needs mint |
| `stock.picking` | `addons/stock/models/stock_picking.py` | `Transport/` | TBD | entity-as-class | needs mint |
| `hr.employee` | `addons/hr/models/hr_employee.py` | `HR/` | TBD | entity-as-class | needs mint |
| `crm.lead` | `addons/crm/models/crm_lead.py` | (new NTO?) | TBD | entity-as-class | needs mint + domain decision |
| `product.product` | `addons/product/models/product_product.py` | (new NTO?) | TBD | entity-as-class | needs mint + domain decision |
| `account.move::action_post` | workflow method on `account.move` | `Accounting/verbs/Post.ttl` | TBD | **verb-as-class** | needs mint |
| `sale.order::action_confirm` | workflow method on `sale.order` | `SalesDistribution/verbs/Confirm.ttl` | TBD | **verb-as-class** | needs mint |
| `stock.picking::action_assign` | workflow method on `stock.picking` | `Transport/verbs/Assign.ttl` | TBD | **verb-as-class** | needs mint |

The table grows additively. Each new Odoo addon module digested by
the v0 producer adds rows; concept-mint passes work in parallel.

## §5. The drift detector — Foundry's "ontology change management" for free

```
Day 1 — digest Odoo at SHA-A
    ruff_python_spo addons/account → ruff_spo_triplet::Model
    ogar-from-ruff               → Class IR
    ttl_emit::emit_entity        → vocab/exports/ogit/NTO/Accounting/*.ttl
    git commit (the TTL set is the frozen contract)

Day N — Odoo upstream releases SHA-B
    same pipeline → /tmp/odoo-shaB-digest/
    diff -r vocab/exports/ogit/NTO/Accounting/ /tmp/odoo-shaB-digest/

    Any output line is a structural change Odoo just made:
    - added field            → diff shows a new ogit:optional-attributes entry
    - renamed column         → diff shows the rename
    - extended selection     → diff shows the new validation-parameter values
    - changed _inherit       → diff shows the rdfs:subClassOf rewire
    - dropped a model        → diff shows file deletion
```

Same diff fires in CI on the same PR if a contributor edits an Odoo
model without re-running the digest. **The TTL templates are the
contract; the digest re-run is the audit; the diff is the gate.**
Foundry sells this as "ontology change management" for a recurring
license fee.

## §6. What blocks doing it today

| Piece | Status |
|---|---|
| Read-only upstream mirror (`vocab/imports/ogit/`) | exists; 72 NTO + SGO + ogit.ttl + SDF imported, MARS oracle proven |
| OGAR-produced export tree (`vocab/exports/ogit/`) | **skeleton exists** (this commit); content populates as digests run |
| TTL emitter for the structural arm | exists (`ttl_emit::emit_entity`); semantic bijection proven on 29 MARS + 176 SGO TTLs |
| Verb-as-class template surface | exists (WorkOrder convention; `docs/VERB-AS-CLASS-TEMPLATE.md`) |
| Author-provenance discriminator | exists (`dcterms:creator` scan in `OGIT-DOMAIN-LIFT-CATALOGUE.md`); now a secondary check behind the directory split |
| `ogar-from-ruff` (mechanical projector from `ruff_spo_triplet::Model` → `Class`) | exists for Ruby AR; same projector handles Python and Elixir once their `ruff_*_spo` frontends ship |
| `ruff_python_spo` (Python AST frontend, sibling of `ruff_ruby_spo`) | **does not exist** — needs `libcst` or `rustpython-parser`; ~1500 LOC for the structural-arm filter |
| `ruff_rust_spo` (Rust AST frontend, for digesting medcare-rs / woa-rs / etc.) | **does not exist** — needs `syn` walker; symmetric with the other ruff frontends |
| `ogar-render-askama::actions` (verb-as-class render path) | **does not exist** — ~200 LOC mirroring the existing `views/` path |
| Concept mints for non-Accounting Odoo models | needs the 5+3 codebook pass per `APP-CLASS-CODEBOOK-LAYOUT.md` |
| The 11 Accounting files in `imports/` | **not a migration target** — already promoted to the OGIT fork (`c5dc1b8`); correctly mirrored. See §2. |

`ruff_python_spo` and `ogar-render-askama::actions` are independent
and can ship in parallel PRs. Concept mints are the slow path
(codebook discipline) and don't block the digest — a digested model
without a minted concept_id just gets `Class.name = "sale.order"` and
gets the id assigned later.

## §7. The Foundry-parity collapse

This section is the punchline. Foundry's platform pitch decomposes
into four layers; each layer maps to a free, open-source piece in
this architecture:

| Foundry layer | Vendor cost | Our equivalent | Marginal cost |
|---|---|---|---|
| Ingest (vendor pipelines) | $ | `ruff_python_spo + ogar-from-ruff` digest (one-shot per Odoo upgrade) | engineer-hours per frontend ~1500 LOC (projector exists) |
| Storage (vendor platform) | $$ | `vocab/exports/ogit/NTO/<Domain>/` TTL templates (mirrors upstream OGIT layout) | zero (skeleton shipped) |
| Render (vendor UI) | $$ | `ogar-render-askama::{views, actions}` | engineer-hours per render path ~200 LOC each |
| Access control / audit (vendor IAM) | $$$ | verb-as-class `requires-perm` slot + `emits-audit` + Lance-version-as-audit (ADR-013) | zero (the substrate already does it) |
| Ontology change management (vendor feature) | $$$ | `diff -r` of digest output (§5) | zero |

The substrate eats every layer the platform sells, **using artifacts
that already exist in this repo**, in less than 2000 lines of new
code. The architecture has been latent the whole time — your
"digest → relive agnostically" framing is what makes it visible as
one shape.

## §8. Cross-references

- `docs/ODOO-TRANSCODING.md` — the existing producer spec (sections 1-18:
  module discovery, field type mapping, attribute kwargs, association
  kwargs, enum sources, class-level metadata, state machines, `_inherit`
  resolution, decorator mapping, CRUD overrides, registered prefixes,
  conformance corpus). The "how" of the digest.
- `docs/VERB-AS-CLASS-TEMPLATE.md` — the askama-template framing the
  digest reuses for workflow actions.
- `docs/HIRO-IN-CLASSES.md` — the bardioc-efficiency story (the same
  digest pattern applied to MARS; this doc generalises it to Odoo).
- `docs/FOUNDRY-ODOO-MARS-LENS.md` — the three-postures cross-reading
  (MARS frozen schema, Odoo extensible source, Foundry vendor platform).
- `docs/OGIT-DOMAIN-LIFT-CATALOGUE.md` — coverage register; new Odoo
  digests advance the rows from Imported → Lift-tested → Cross-walked.
- `docs/APP-CLASS-CODEBOOK-LAYOUT.md` — the 5+3 mint protocol for
  new concept_ids (the §6 slow path).
- `crates/ogar-from-schema/` — the schema-arm producer (TTL today;
  XSD queued) that the Odoo digester pairs with on the structural
  boundary.
- `crates/ogar-render-askama/` — the askama renderer (views today;
  actions queued).
