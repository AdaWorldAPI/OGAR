# Foundry · Odoo · MARS — three angles on one ontology

> **For colleagues thinking about the Palantir-Foundry-Odoo angle.**
> MARS, Odoo, and Foundry each solve "manage business objects" with a
> different posture. Cross-reading them lets OGAR borrow the strong
> property from each without inheriting the failure modes. The Foundry
> lens improves Odoo. The MARS lens improves both. OGAR's IR carries
> all three.
>
> Status: **LENS v0** (2026-06-22). Reads like an inventory of
> trade-offs; the implementation work is in
> `docs/MARS-TRANSCODING.md` + `docs/ODOO-TRANSCODING.md`.

The cross-reading in one sentence: **Foundry sells ontology rigor as a
platform feature; MARS gives it for free via a frozen XSD; Odoo gives
up rigor for extensibility. OGAR holds the IR all three lower to and
keeps the wins from each.**

---

## §1. The three postures in one table

| | **Palantir Foundry** | **Odoo** | **MARS (OGIT)** |
|---|---|---|---|
| Ontology source | platform-curated, vendor-locked | per-model `selection=[...]` lists in Python | **frozen XSD** (`MARSSchema2015.xsd`, unchanged since 2015) |
| Adding a new classification | platform UI / vendor support | `selection_add=[('foo','Foo')]` on an inheriting model | re-issue the XSD; bumps schema version |
| Drift detection | vendor "ontology change management" (paid) | nothing built-in — `selection_add` can silently extend | mechanical (`extract_classes.py`); the schema IS the contract |
| Object composition | platform-defined join graph | `_inherit` / `_inherits` mixin chain | flat 4-tier A→R→S→M dependency |
| Multi-tenant labels (PII / German captions) | platform-managed | per-model Python strings | `dcterms:description` + `xml:lang` (XSD only); TTL doesn't carry labels at all |
| Lift cost into OGAR | vendor API (no direct lift) | `ogar-from-rails`/`ogar-python` source AST | `ogar-from-schema::ttl` — pure, bijective |

Each posture has one strength worth borrowing and one failure worth
guarding against.

---

## §2. What MARS teaches Odoo (and OGAR)

### Strength: **frozen-schema drift detection**

Odoo's `selection_add` is operator gold — it lets a downstream
deployment extend a parent model's classification without forking the
parent. The price: **silent drift**. Two deployments of the same Odoo
module can have different `account.move.state` value sets and never
know it until a federation/migration moment.

MARS gives the recipe for free: **author an XSD (or any schema) as the
frozen contract; run it as the drift detector against every PR**. The
producer side (`extract_classes.py`) already exists; the consumer side
is what the OGAR `ogar-from-schema::xsd` follow-up adds. Per PR:

```bash
# In CI:
ogar-from-rails --output classes.json        # source-AST lift
ogar-emit-xsd --input classes.json > emitted.xsd  # reverse-emit XSD
extract_classes.py -s emitted.xsd -F asciidoc > emitted.adoc
diff committed.adoc emitted.adoc
# any output line = a structural change → require schema commit
```

**Foundry charges money for this.** OGAR gets it from
`extract_classes.py` + 50 lines of producer.

### Lesson for OGAR

Odoo's lifted `Class` should carry a `validation: Strict | Permissive`
slot. **Strict** (the MARS default) refuses unknown enum values.
**Permissive** (the current Odoo default) accepts them but logs to a
drift channel. Same `Class`, two policies, one declared per deployment.

---

## §3. What Odoo teaches MARS (and OGAR)

### Strength: **`_inherit` mixin composition**

MARS's four entities are flat. Application, Resource, Software,
Machine — no shared traits. Every Application carries the same
`automationState` + `serviceStatus` + `tenantId` because those are
**copy-pasted** into each entity's `optional-attributes`. That's
schema duplication; if you want to add `auditChannel` to all four,
you edit four files.

Odoo solved this with `_inherit = 'mail.thread'` etc. — mixins that
contribute fields and behaviour to any model that opts in. The lift
into OGAR is already there: `Class.mixins: Vec<String>`.

### Lesson for MARS

MARS's `Application` should `_inherit` from a generic
`Monitorable`/`Auditable` mixin so the shared attributes
(`automationState`, `serviceStatus`, `tenantId`, the future
`auditChannel`) live in **one** TTL file. The OGIT TTL dialect doesn't
have a mixin verb today; OGAR's lift can add it without breaking the
upstream contract — emit OGIT-flavoured TTL with the mixin fields
expanded inline (preserving bijection) and carry the mixin reference
only in the lifted `Class`.

This is the **structural-arm version of source-AST inheritance**:
mixins as schema sugar, expanded at emit time, tracked as IR.

---

## §4. What both teach Foundry-shaped consumers

### Strength: **vendor-platform consistency**

Foundry's pitch is "one ontology, every app sees it the same way." It
works because the platform owns the schema, the API, the UI. The
vendor lock is the cost of that consistency.

### Lesson for OGAR

OGAR achieves the same consistency **without the vendor lock** by
making the IR the contract. `Class` + `Association` + `ActionDef` are
the same shape every consumer sees; the divergence between Foundry's
"object graph" and Odoo's "model registry" and bardioc's "MARS nodes"
collapses to one IR with three lowering targets:

| Foundry object surface | OGAR `Class.<x>` | Lower to |
|---|---|---|
| object type | `Class` | `surql DEFINE TABLE` / Postgres CREATE TABLE |
| property | `Attribute` | column |
| link | `Association` | foreign-key / `RELATE` |
| action | `ActionDef` | stored procedure / GenServer / Cranelift kernel |
| ontology change management | XSD drift detector (§2) | CI gate |

Foundry's "object explorer" is one `ClassView` (render lens —
`docs/APP-CLASS-CODEBOOK-LAYOUT.md`); Odoo's web UI is another;
bardioc's CLI is a third. **Same hi u16 concept, different lo u16
render prefix** (order flipped 2026-07-02 — canon HIGH / custom LOW) —
already the architecture per `docs/OGAR-CONSUMER-BEST-PRACTICES.md`.

---

## §5. The lens reduces to one rule per direction

| From | To | The rule it teaches |
|---|---|---|
| **MARS → Odoo** | "Author your structural arm as a schema. Run it as the drift detector." |
| **Odoo → MARS** | "Lift mixins. Don't copy-paste attributes across four flat entities." |
| **Foundry → OGAR** | "Ontology consistency is the IR, not the platform. Lock the IR; let the renders bloom." |
| **OGAR → all three** | "Structural arm = schema; behavioural arm = source AST; they're disjoint and they cross-validate at the structural boundary." |

---

## §6. Cross-references

- `docs/HIRO-IN-CLASSES.md` — the bardioc-efficiency story (the funny
  insight that powers the MARS→Odoo direction)
- `docs/MARS-TRANSCODING.md` — the calibration spec
- `docs/ODOO-TRANSCODING.md` — the Odoo lift (companion target for the
  drift detector idea)
- `docs/APP-CLASS-CODEBOOK-LAYOUT.md` — the hi/lo classid split that
  makes "same concept, different render" work without vendor lock
- `docs/OGAR-CONSUMER-BEST-PRACTICES.md` — the consumer-side muscle
  memory (per-app `ClassView` is the Foundry "object explorer"
  equivalent)
- `docs/DOMAIN-INSTANCES.md §2.4` — the production Odoo/ERP instance
- `docs/SUBSTRATE-ENDGAME.md §5` — the Foundry-parity argument
