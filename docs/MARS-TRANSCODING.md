# MARS — calibration spec

> **For OGAR designers / reviewers.** MARS is the **third closed-formal
> calibration domain** (after chess and OSM-quadkey). The XSD acts as a
> bijective oracle — same shape `shakmaty::Position::play` plays for
> chess in `CHESS-TRANSCODING.md §0`. Production substrate, frozen
> schema, mechanical round-trip.
>
> Status: **CALIBRATION v0** (2026-06-22). Companion to
> `docs/HIRO-IN-CLASSES.md` (the why) and
> `docs/ELIXIR-HIRO-PREFETCH.md` (the behavioural-arm prefetch).

The MARS calibration is what locks the structural arm of the lift:
a **frozen XSD** (`MARSSchema2015.xsd`, version 5.3.8, unchanged since
2015) AND a **derived TTL** (`AdaWorldAPI/OGIT @ NTO/MARS/`) both
describe the same four-entity taxonomy. OGAR's `ogar-from-schema`
producer must lift each into a `Class`+`Attribute` set; the sets must
agree byte-for-enum.

---

## §0. The pipeline

```
                 MARSSchema2015.xsd  ─────┐
                                          │  extract_classes.py
                                          │  (Python 2, runs unchanged
                                          │   on Py3 via 2to3 — see
                                          │   PROVENANCE.md table)
                                          ▼
                          classifications.adoc / .html
                                          │
                                          │  (cached at
                                          │   vocab/imports/ogit/NTO/MARS/
                                          │   _oracle/)
                                          ▼
                                  XSD-oracle reference set
                                          │
                                          │  unit test
                                          ▼
  vocab/imports/ogit/NTO/MARS/*.ttl  ──►  ogar-from-schema::ttl  ──►  EntityDecl + AttributeDecl
       (literal byte-mirror of                                                 │
        AdaWorldAPI/OGIT)                                                     │  into_class()
                                                                              ▼
                                                              ogar_vocab::Class (structural arm only;
                                                                                  behaviour arm = empty)
                                                                              │
                                                                              │  ogar-from-elixir
                                                                              │  (sibling, lifts gen_statem)
                                                                              ▼
                                                                       Class + ActionDef
                                                                       (full bardioc replacement target)
```

The XSD-oracle reference set is the bijection witness for the structural
arm; `ogar-from-elixir`'s `gen_statem` lift is the bijection witness for
the behavioural arm (`ELIXIR-HIRO-PREFETCH.md §2.2`). Two oracles, one
target shape.

---

## §1. The four entities (lifted 1:1 from OGIT NTO/MARS)

| Entity | Origin TTL | Classifications | Verbs declared in `ogit:allowed` |
|---|---|--:|---|
| `Application` | `entities/Application.ttl` | 7 classes × 50 subclasses | `relates License`, `generates Log`, `generates Timeseries`, `dependsOn Resource` |
| `Resource` | `entities/Resource.ttl` | 19 classes (no subclass) | (chained) |
| `Software` | `entities/Software.ttl` | 40 classes × 336 subclasses | (chained) |
| `Machine` | `entities/Machine.ttl` | 11 classes (no subclass) | `generates Log`, `generates Timeseries`, `contains NetworkInterface`, `has Tag`, `runsOn Cluster`, `runsOn ResourcePool`, `locatedAt Location` |

The dependency backbone — **A→R→S→M**:

```
Application  ──dependsOn──►  Resource  ──dependsOn──►  Software  ──dependsOn──►  Machine
   0x??01                       0x??02                     0x??03                   0x??04
```

(Classid byte allocation is **provisional**; mint requires the 5+3
codebook pass per `CLAUDE.md`. See `docs/APP-CLASS-CODEBOOK-LAYOUT.md`
for the §4 mint protocol. The 0x??XX domain byte is reserved for MARS-
infrastructure; concrete `class_ids::MARS_*` constants land in a
follow-up after the codebook pass.)

---

## §2. The chess-grade bijection oracle

`extract_classes.py` (the upstream `arago/MARS-Schema/tools/` script,
Py2-as-shipped, vendored at `_oracle/extract_classes.py` and Py3-converted at
`_oracle/extract_classes_py3.py`) is a **complete bijective oracle** for the
classification taxonomy:

| Direction | Oracle role |
|---|---|
| XSD → classifications | `extract_classes.py -s MARSSchema2015.xsd -F asciidoc` enumerates every `(class, subclass)` pair |
| TTL → classifications | OGIT `Application/attributes/class.ttl` etc. carry the same set in `ogit:validation-parameter` |
| OGAR → classifications | `ogar-from-schema::ttl::AttributeDecl::fixed_enum_values()` lifts them as `EnumSource::Static` |

The three sets must be byte-equal modulo encoding artefacts (HTML/asciidoc
escape vs raw). The agreement test
(`ttl::tests::application_class_values_appear_in_xsd_oracle`) is the v0
witness; expansion to full set-equality (XSD set == TTL set, no missing,
no extra) is queued.

Counts at MARS Schema 5.3.8:

| Section | Classes | (class, subclass) pairs |
|---|--:|--:|
| Application | 7 | 50 |
| Resource | 19 | — (2-col, no subclass) |
| Software | 40 | 336 |
| Machine | 11 | — (2-col, no subclass) |

---

## §3. The six IR-shape tests (per `OGAR-AS-IR.md §3`)

The MARS lift was designed to pass all six tests in the IR-design
checklist. Quick audit:

| Test | How MARS satisfies it |
|---|---|
| **1. SSA / dataflow-explicit** | Every attribute named in `mandatory-/optional-attributes`; every edge named in `ogit:allowed`. No implicit state. |
| **2. Effect annotations first-class** | Each entity gets a `Language::Unknown` tag (TTL is language-neutral); behaviour effects don't enter the structural arm (the lift is *pure*). |
| **3. Typed signature** | Every attribute carries a stable `Attribute.name`; fixed enums carry `EnumSource::Static(Vec<(value, value)>)` with a closed value set from the schema. |
| **4. Named lowering passes** | `ttl::parse_file` (frontend), `into_class` (lowering), `ttl_emit::emit_entity` (reverse codegen). Each is an explicit named function. |
| **5. Semantic-preservation guarantee** | The round-trip test (`all_mars_ttl_files_roundtrip`) is the explicit guarantee: `parse(emit(parse(src))) == parse(src)` for every TTL file. |
| **6. IR is canonical** | The XSD oracle and the OGIT TTL are *interchangeable sources*; the OGAR `Class` is the canonical artifact both lower to. |

---

## §4. What this means for the next OGIT domain lift

Every NTO domain that joins this lift inherits the same calibration
machinery for free:

1. Drop the upstream TTL into `vocab/imports/ogit/NTO/<Domain>/` (already
   done for all 72 domains).
2. If the domain has an XSD oracle (`arago/<Domain>-Schema`), drop it in
   `_oracle/` and add an agreement test.
3. Run `ogar-from-schema::ttl::parse_file` over each entity/attribute —
   no per-domain producer code needed.
4. Add a `DOMAIN-INSTANCES.md` row.

That's it. The MARS calibration was the load-bearing demo; the next 71
domains are paperwork.

---

## §5. Cross-references

- `vocab/imports/ogit/NTO/MARS/` — the literal 1:1 mirror
- `vocab/imports/ogit/NTO/MARS/PROVENANCE.md` — SHA + license + re-vendor recipe
- `vocab/imports/ogit/NTO/MARS/_oracle/` — the XSD + `extract_classes.py` oracle
- `crates/ogar-from-schema/` — the producer (TTL + reverse-emit + SGO verbs)
- `docs/HIRO-IN-CLASSES.md` — the bardioc-efficiency story
- `docs/FOUNDRY-ODOO-MARS-LENS.md` — the cross-domain lens
- `docs/ELIXIR-HIRO-PREFETCH.md` — the behavioural-arm prefetch (sibling)
- `docs/OGAR-AS-IR.md` — the compiler framing (six IR-shape tests)
- `docs/DOMAIN-INSTANCES.md` — MARS row in the universality matrix
- `docs/OGIT-DOMAIN-LIFT-CATALOGUE.md` — coverage status for all 72 NTO domains
- `docs/CHESS-TRANSCODING.md` — the calibration template MARS follows
