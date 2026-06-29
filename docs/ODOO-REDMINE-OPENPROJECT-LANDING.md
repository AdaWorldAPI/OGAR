# Odoo + Redmine/OpenProject — ERP/planning landing plan (OGAR)

> **Companion to `docs/OGAR-TRANSPILE-SUBSTRATE.md`** (the per-class transpile
> machine — lift + mint + emit — landed in OGAR #132). This is the durable
> ERP/planning *landing plan*: how Odoo (ERP) + Redmine/OpenProject (planning)
> land in OGAR in the *best native-yet-agnostic-yet-truthful* shape — "Odoo"/
> "Redmine" are **labels**; the **ontology** is the canonical ERP/planning
> form, and lance-graph is the Foundry it executes on. (Relocated from
> `.claude/handovers/` per `.claude/AGENTS.md` §Forbidden — durable docs live
> in `docs/`.)

---

## 0. The frame (do not re-derive)

- **OGAR = the transpiler** (transpile-time): `ogar-from-*` parse a source AST →
  the SPO/`ModelGraph` IR → classids, `ClassView`s, facets. It *produces shapes*.
- **lance-graph = the Palantir-Foundry-shaped execution** (runtime): where those
  shapes run / store / think. Consumers (medcare-rs, woa, smb, and the
  ERP/planning apps) **execute on lance-graph**.
- **Connecting tissue** (BBB-allowed in any consumer binary): `lance-graph-contract`
  / `-ontology` / `-ogar` / `-callcenter`. The BBB bars only the **brain/engine**
  crates (planner, cognitive engine).
- **"Odoo" / "Redmine" are render labels** (the hi-u16 `AppPrefix`): Odoo=`0x0002`,
  OpenProject=`0x0001`, Redmine=`0x0007`. The **lo-u16 canonical concept** is the
  shared, agnostic identity. Same concept across apps ⇒ same lo-u16.

## 1. Mission: native + agnostic + truthful

- **Native:** faithful to the source's real model (Odoo's ORM fields/associations/
  methods; Rails/AR `belongs_to`/`has_many`/`validates`/`acts_as_*`).
- **Agnostic:** the landed shape is the **universal ERP/planning ontology**, not an
  Odoo/Rails-specific struct. The label drops; the concept stays.
- **Truthful (lossless):** nothing dropped. Each member lands in an ontology target
  (below); the 16-byte facet is just the content-addressed index over it.
- **Adjacent (drift-free):** ontologically-related concepts share a `part_of:is_a`
  prefix; identity is by **reference** (one ClassView / one adapter, never copies);
  the canonical codebook is single-source ("pull, never re-mint").

## 2. The import contract (per member — the truthful shape)

| source member | lands as | drift-proofing |
|---|---|---|
| `has_field` | a `ClassView` slot | reference |
| `field_type` | the **type's** `ClassView` (recursive — types are classes) | reference, not copy |
| `has_function` / Odoo method / AR callback | **ActionDef/KausalSpec ontology + a consumer wrapper over reusable OGAR adapters** (logic *extracted as ontology*, not reimplemented; for ERP this is the posting/validation behaviour) | shared adapter |
| object type | the **shared codebook concept id** | canonical id |
| ERP ontology glue (the ICD-analogue) | **FIBO** (finance grounding) + **DOLCE** (upper) + **SKR03/04** + l10n_de regulation IRIs (HGB/UStG/GoBD) | hierarchy / pivot |

> Over-cap classes (`>= 256` members — Odoo has god-models) **branch** via the SoC
> lint (`ruff_spo_address::soc` / `soc_branches`): data → ClassView(s) (paginate via
> class hierarchy if `> 64` distinct types), behaviour → ActionDefs/adapters. Branch,
> never widen.

## 3. What already exists (grounded — build on it, don't reinvent)

**odoo-rs** (private):
- `od-ontology` — SPO triple corpus (`triple.rs`, ~13 predicates × 4 NARS bands);
  `schema_to_classes` / `emit_via_ogar` (`ogar_bridge.rs`) lower Odoo → `ogar_vocab::Class`
  (pulls classids via `OdooPort`, a pure static lookup — no bridge object/registry).
- `od-posting` — **GoBD double-entry** posting host (gapless Belegnummer +
  inalterability hash chain). This is the ERP *behaviour* arm.
- `alignment::ODOO_SEED` (`pivot_uri` + DOLCE + `label_uri`, keyed by odoo_class) +
  `data/ontologies/odoo/alignment/odoo-to-fibo.ttl` — the FIBO pivot already exists
  on the Odoo side.

**lance-graph** (`crates/lance-graph-ontology/src/odoo_blueprint/`):
- ~404 curated + huge auto-extracted `OdooEntity` consts (`account` 685 KB, `stock`
  383 KB, `l10n_de_chart` 864 KB / SKR03-04 **2466 rows** + **37 UStVA Kennzahlen**),
  with fields, methods, state machines, German `regulation_iri` anchors.
- `class_signature.rs` `StructuralSignature` / `shape_families` (FNV-1a histogram) —
  **the route/form duplication clustering** (collapse identical-shape classes to one
  shared ClassView).
- `hydrators/{odoo,dolce_odoo,fibo}.rs` — Odoo hydrated as `OGIT::ODOO_V1` inheriting
  `FIBOFND_V1` (FIBO Foundations).

**OGAR** (the transpiler): `ogar-from-ruff` (Python/Odoo SPO frontend),
`ogar-from-rails` (Rails/AR), `docs/{ODOO-TRANSCODING,ODOO-DIGEST-TO-OGIT,FOUNDRY-ODOO-MARS-LENS}.md`.

**ruff** (the SPO frontends + mint):
- `ruff_spo_triplet` — closed **60-predicate** vocab, the neutral `ModelGraph` IR,
  `expand` ⇄ `reassemble` (NOTE: `reassemble` is **C++-projection-only today** — a
  general reassembler is the first build item if you round-trip Python/Ruby).
- `ruff_spo_address` — the 16-byte facet mint (`mint_with_classid(triples, classid_of)`),
  `soc` + `soc_branches` (the SoC lint, ruff PR #33).
- `ruff_ruby_spo` — Ruby/Rails frontend (lib-ruby-parser, `app/models`, **27
  OpenProject AR predicates**); `NAMESPACE="openproject"`.
- `ruff_python_dto_check` — Python harvester (→ JSON bundles; needs a `bundle →
  ModelGraph` adapter to enter the same IR door).

**lance-graph-contract** (the tissue):
- `ogar_codebook.rs` — `CODEBOOK` (`0xDDCC` concept ids), `canonical_concept_id`,
  `AppPrefix`, `render_classid`. **The cross-app join key.**
- `class_view.rs` — `ClassView` (trait), `FieldMask`, `RenderRow`, `FieldRef`.
- `facet.rs` + `facet_schema.rs` (lance-graph PR #619, **merged**) — the 16-byte
  facet with classid-selected format (`6×2` cascade | `4×3` SPO | `2×48`).

**Convergence proof (the agnostic identity, already wired):**
`crates/lance-graph-ogar/tests/bridge_codebook_convergence.rs` — OpenProject
`WorkPackage` & Redmine `Issue` → `project_work_item 0x0102`; `TimeEntry` /
`Stundenzettel` / Odoo `account.analytic.line` → `billable_work_entry 0x0103`;
Odoo `account.move`+`sale.order` → `commercial_document 0x0202`; `product.template`
→ `product 0x0207`; `account.account` → `accounting_account 0x0208`.

## 4. Redmine / OpenProject (the parallel landing — same methodology)

- Frontend: `ogar-from-rails` / `ruff_ruby_spo` over `app/models` (the AR shape:
  `belongs_to`/`has_many`/`validates`/`acts_as_list`/callbacks → the 27 AR predicates).
- Identity: `WorkPackage`/`Issue` → `0x0102`; `TimeEntry` → `0x0103` (shared with Odoo
  — that 0x0103 convergence is the planner↔ERP billable-hours pin).
- Render: the **"17-year Redmine QueryColumn evolution"** lands as the `ClassView` +
  `FieldMask` line-view (`ogar-render-askama::list_view`, `render_rows`) — the same
  field-view primitive medcare's views use.

## 5. The cross-domain map (the Foundry payoff — LATER, easy once truthful)

ERP ↔ planning ↔ health align at the **ontology plane**, by:
- **shared lo-u16 codebook concept** (the de-facto `owl:equivalentClass`, expressed
  as a u16) — *the only mechanism wired today*;
- **FIBO/DOLCE pivots** (Odoo already has `odoo-to-fibo.ttl`; author the symmetric
  twins so domains inherit a common FIBO/DOLCE category via `ContextBundle.inherits_from`).

## 6. Gaps the session must close (from the ontology-preservation verification)

1. **Carry the codebook concept through the mint** — always stamp `facet_classid`
   via the OGAR resolver; **never ship `classid=0`** (no join key, no ontology).
2. **Promote groundings to typed data** — give each `CODEBOOK` row an optional
   `{ogit_uri, dolce_category, fibo_equivalent_class}` (lift today's doc-comment
   annotations), and thread it through the mint (don't drop it at the `_ =>` arm).
3. **FK facet → registry** — put the concept u16 on `MappingRow` (O(1) facet →
   `OntologyRegistry`; today it's indirect `ogit_uri` string-match).
4. **Fix the OWL hydrator** — store subject→object edges so `rdfs:subClassOf` is
   reconstructable (today it flattens s/p/o into one `iri_to_id`; TTL export hardcodes
   a flat `rdfs:subClassOf ogit:Entity`).
5. **Behaviour arm** — land Odoo methods / AR callbacks as `ActionDef` ontology +
   reusable adapter, **not** reimplemented (od-posting's GoBD logic is the test case).
6. **Land `odoo_blueprint` extracted entities as OGAR `Class`** via
   `od-ontology::schema_to_classes` (the future `od-ontology-bridge` binary).

## 7. Hard constraints

- Lossless + truthful **now**; cross-domain tissue **later** (don't let mapping drive
  the import shape).
- OGAR = transpiler; lance-graph = Foundry execution. Tissue crates in the binary;
  brain/engine crates out (BBB).
- Canonical codebook is single-source: **pull, never re-mint** (drift-guarded by
  `codebook_ids_match_ogar_vocab`).
- SoC lint fires at `>= 256` members → branch.
- No German PII; **no model/session identifier in any committed artifact**; audit
  never egresses.

## 8. First moves (ordered)

1. Land **one Odoo model end-to-end** (e.g. `account.move` → `0x0202`): SPO →
   `ModelGraph` → `Class` (via `schema_to_classes`) → mint with classid → facet,
   with its FIBO/DOLCE grounding **retained** (Gap #2). Round-trip it.
2. Do the **OpenProject twin** (`WorkPackage` → `0x0102`) — prove the agnostic
   convergence (same concept, two app labels) on a real model, not just the test.
3. Promote the grounding tuple on `CODEBOOK` (Gap #2) + the `MappingRow` FK (Gap #3).
4. Then scale `odoo_blueprint` (the 404 entities) through the same path.

## 9. References

- Merged: lance-graph #619 (`facet_schema`), ruff #32 (polyglot AST RFC), OGAR #130
  (V3-transpiler ADR), lance-graph #620 (node-block layout — resolved: flexible
  per-classid, first GUID already encodes location+relations).
- In-flight: ruff #33 (`soc` + `soc_branches`, SoC lint).
- Verification: ontology-preservation (workflow over ruff/lance-graph/odoo-rs);
  the cross-domain mapping is codebook-keyed today, FIBO/DOLCE pivots aspirational.
- medcare-rs board `EPIPHANIES` E-7…E-19 (the substrate clarifications; E-18→E-19
  correct the OGAR-transpiler / lance-graph-Foundry polarity).
