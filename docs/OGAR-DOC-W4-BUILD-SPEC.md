# ogar-doc W4 — Persistence + Reconstruction — BUILD SPEC (council-ready)

> **Status:** BUILD SPEC — the persistence half of `D-OGAR-DOC-LAYER` (charter
> `docs/OGAR-DOC-LAYER-PROPOSAL.md`, merged #191), reconciled with the as-built
> `D-DOC-IR-SECOND-RETINA` W1/W2/W3 (`ogar-doc-ir` #197 · `ogar-from-docv1` #202 ·
> `spider_doc_ir` merged). Detailed enough that the 5+3 council VERIFIES rather
> than redesigns; deviations require a numbered factual error, not taste.
>
> **Operator gate (unchanged):** the two canon mints (`0x080A`/`0x080B`) land
> WITH the council-verified build, **never ahead of it** (charter §D3;
> D-DOC-IR-SECOND-RETINA "operator-gated"). This doc mints NOTHING — it is the
> blueprint the mint approval unlocks.
>
> **One sentence:** W1/W2/W3 gave us the source-agnostic perceptual IR
> ([`ogar_doc_ir::DocIr`]) from two retinas; **W4 PERSISTS a `DocIr` as a
> GUID-keyed SoA subtree + a raw KV reference, and RECONSTRUCTS an updated PDF
> from that subtree** — the operator's Spire.Doc-style "re-issue with updated
> knowledge" headline.

## W4-1. The reconciliation — W4 consumes `DocIr`, not `doc.v1`

The #191 charter said "persist a `doc.v1`." The as-built refines the input by
one hop: both retinas normalize to the **neutral** `ogar_doc_ir::DocIr` (pixel
retina `ogar-from-docv1::from_doc_v1`; DOM retina `spider_doc_ir`). So W4's
persistence input is a **`DocIr`**, already source-agnostic — a scan and the
HTML of the same document converge on ONE subtree because
`ogar_doc_ir::converges_on_facts` guarantees equal typed-field facts (P-XRETINA).

The IR was **built for this** — its doc-comments already pin the persistence
mapping onto the 12-byte facet register:

- `BBoxRail` = two `Rail`s = 4 bytes = **two of the six `u8:u8` pairs** of the
  facet register (`ogar-doc-ir/src/lib.rs:127`).
- `TableCell{row,col}` = **one `u8:u8` pair** on the register (`lib.rs:156`).
- `DocIr` = "the `document 0x080B` subtree *before* persistence" (`lib.rs:207`).

W4 makes those comments executable. No re-derivation of `doc.v1`; W4 is a pure
downstream of the merged IR.

## W4-2. The subtree — raw as KV reference, awareness as SoA facets

Per the charter §D2 and P0 "THE GUID IS THE KEY OF KEY-VALUE": a `DocIr` does
not fit one 496-byte value, so a document is a **GUID-keyed subtree**, one node
per structural unit, each a content-blind 4+12 facet the ClassView projects:

```
document           classid = DOCUMENT 0x080B          ← subtree root
  value: raw-ref = { content_sha256[32]→KV key digest, kv_key, mime, page_count,
                     field_count, mean_conf }   (NEVER the raw bytes)
  ├─ page[n]       classid = PAGE_LAYOUT 0x0807  (page structure; number/w/h in register)
  │   ├─ region[k] classid = PAGE_LAYOUT 0x0807  (RegionKind in register byte; bbox = 2 rails;
  │   │   │                                        reading_order = the temporal-stream key)
  │   │   ├─ cell   (Table region only) — a FACET under the region, addressed by
  │   │   │          (row:col) u8:u8 + bbox rails; NOT a new classid
  │   │   └─ figure classid = PAGE_IMAGE 0x0808  (raster region; bbox rails; raw tile → KV)
  └─ field[m]      classid = TYPED_FIELD 0x080A  (key/value out-of-line by identity;
                                                   bbox rails; confidence byte)
```

- **Raw as reference (KV):** original bytes live in the consumer's blob store
  (MedCare's encrypted `file_filelist`, a Lance blob column, S3). The root
  value carries only the reference (sha256 + storage key + mime + counts). Iron
  rule: **awareness never re-embeds raw bytes**.
- **Awareness as SoA (graph):** the `DocIr` structure decomposes into the
  facets above; region kind, bbox rails (`X:Y`), cell `(row:col)`, field
  confidence all read from the 12-byte register the ClassView projects
  (`E-V3-FACET-4-PLUS-12`). Text/`key`/`value` strings are out-of-line stores
  keyed by classid+identity (`I-VSA-IDENTITIES`: facet = identity + typed dims,
  never the content register).
- **Minimal mints:** only `document 0x080B` + `typed_field 0x080A` are new.
  Page/region reuse `PAGE_LAYOUT 0x0807`; figures reuse `PAGE_IMAGE 0x0808`;
  cells are facets, not a new class ("container kinds, not content" discipline).

## W4-3. The two mints (canon-high — council-gated)

Confirmed free on `main` (highest OCR mint = `OCR_RENDERER 0x0809`,
`ogar-vocab/src/lib.rs:class_ids`):

| concept | classid | role | value register |
|---|---|---|---|
| `typed_field` | `0x080A` (v2-reserved slot, charter §D3) | one harvested field as a graph node | `confidence` byte + bbox rails; `key`/`value` out-of-line by identity |
| `document` | `0x080B` | the subtree root | raw-ref (sha256 digest + kv_key + mime + counts) |

Both canon-high (concept in hi u16, APP_PREFIX in lo u16 per
D-CLASSID-CANON-HIGH-FLIP). Append-only: no existing concept edited. Exact hex
is the canon's to ratify; this honors the v2 `typed_field → 0x080A` reservation
and takes the next slot `0x080B` for `document`.

## W4-4. The three ActionDefs (facts only — the V2-4 rule)

`ogar-vocab/src/ocr_actions.rs`, same table as the shipped 14 caps, External kausal:

| capability | subject | mandatory | optional | produces |
|---|---|---|---|---|
| `persist_document` | `document 0x080B` | `doc_ir, raw_sha256, raw_kv_key` | — | `document_guid` |
| `read_document` | `document 0x080B` | `document_guid` | — | `doc_ir` (rehydrated) |
| `reconstruct_document` | `document 0x080B` | `document_guid` | `template` | `pdf_bytes` |

- `persist_document` — decompose `DocIr` → the SoA subtree (§W4-2) + store the
  raw-ref. **Facts only**: HOW nodes are written (mailbox / Lance) is the
  executor seam's business, exactly as the OCR rows. Idempotent on
  `content_sha256` (re-persisting the same document is a no-op / version bump,
  never a duplicate subtree — the convergence key).
- `read_document` — walk the subtree back into a `DocIr` (the inverse; the
  `to_json` load-gate round-trips it).
- `reconstruct_document` — the Spire.Doc-style re-emit: walk the awareness
  subtree, bind a `template`, render a PDF. Because the awareness is graph
  nodes, **mutate a `typed_field` node then re-fire ⇒ the document re-issues
  with updated knowledge** (the operator's headline). `template` optional: a
  data-only re-issue uses the default template; a fidelity re-issue may bind the
  original raster from the KV raw-ref (searchable-PDF path).

## W4-5. The executor + the reconstruct renderer (the A1–A5 rulings)

Per `D-DOC-IR-SECOND-RETINA` operator rulings A1–A5:

- **A1 — one `ogar-doc` crate.** Split axis is STATE (recognized → persisted →
  reconstructed), not direction. The executor is one crate binding the 3
  ActionDefs via the HOT_PLUG fuse, exactly as `tesseract-ogar` binds the OCR
  rows. Deps: `ogar-doc-ir` (the IR) + the codebook membrane; NOT
  `tesseract-ogar` (recognition is a separate, already-fused executor).
- **A2 — `DocRenderer` trait, the one-leg rule.** Runtime-bound adapters:
  `tesseract-ocr-pdf::render_searchable_pdf` (raster + text layer, the proven
  brick), `askama` (data-only HTML→PDF), a future Spire.Doc-style leg. One trait,
  many legs bound at runtime; no leg is compiled into the contract.
- **A-ruling — document template = `ClassView × WideFieldMask`.** The **same
  brick as Klickwege** — no new template DSL. The template selects which subtree
  facets bind into which rendered slots; evolution is a new mask, not a new
  language.

## W4-6. Consumers inherit it — no per-consumer wiring

MedCare-rs (immediate), woa-rs, smb-office-rs, odoo-rs: on a DMS upload the
executor fires `recognize_document` (existing) → `persist_document` (raw-ref =
the consumer's encrypted blob key; awareness → the subtree). A search / re-issue
fires `read_document` / `reconstruct_document`. **Zero tesseract-rs / doc-model
types in any consumer** — it pulls the `document` classid via
`ogar_codebook::canonical_concept_id` (the BBB membrane) and invokes. This is why
the layer is OGAR-level, not per-consumer: the dep-graph landmine of pulling
`tesseract-ogar` (or `ogar-doc`) into each customer binary is avoided — consumers
pull a classid + invoke through the membrane (OGAR-CONSUMER-BEST-PRACTICES §2).

## W4-7. OGAR-AS-IR §3 + SURREAL-AST-TRAP-PREFLIGHT

- **IR-shape tests (§3):** (2) effect-annotations first-class — `reads`/`writes`
  on the 3 ActionDefs. (3) typed signatures not field-bags —
  `OcrActionParam`-shaped. (5) semantic-preservation — additive; no existing
  row/concept/IR type edited. (6) IR canonical — the vocab table + the two mints
  are the canonical IR the `ogar-doc` executor resolves against. (1)/(4) N/A
  (declared capabilities, not a lowering pass).
- **SURREAL-AST-TRAP:** not a producer→IR / codegen / `.surql` session. Behavior
  lands as `ActionDef` facts (sanctioned home); the subtree is graph DATA (SoA
  facets), not a `DEFINE EVENT … THEN` lifecycle. Persistence + reconstruction are
  `ActionInvocation` at runtime on the Core node. **On the sanctioned path by
  construction.**

## W4-8. Non-goals (unchanged from the charter §D8)

No storage backend chosen (KV blob is the consumer's). No new template DSL
(reuse `ClassView × WideFieldMask`). No streaming payload (executor seam). No
`.surql` / DDL anywhere. No recognition changes (tesseract-rs untouched; W4 is
pure downstream of the merged `DocIr`).

## W4-9. Falsifier + the gate

- **P-XRETINA (already on `main`, #199):** `ogar_doc_ir::converges_on_facts(a,b)`
  — same document via both retinas ⇒ same typed-field facts ⇒ ONE subtree.
  `persist_document` MUST be idempotent under this equality (re-persisting the
  DOM retina's `DocIr` of a document already persisted from the pixel retina is a
  no-op/version bump, not a second subtree). This is W4's pre-persist gate.
- **Council + mint gate:** this spec is 5+3-council-ready. On a green council +
  the operator's mint approval, `0x080A`/`0x080B` land WITH the executor build in
  one PR (charter §D3). The council output (ratified v3) + the DISCOVERY-MAP
  `D-OGAR-DOC-LAYER` status bump are the board-hygiene deliverables that land in
  the same commit.
