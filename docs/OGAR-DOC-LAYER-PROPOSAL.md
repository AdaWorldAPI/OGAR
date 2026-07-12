# ogar-doc — the Document Layer — PROPOSAL (council pending)

> **Status:** PROPOSAL — 5+3 council pending (5 verification savants →
> consolidate → 3 brutal reviewers → fix → consolidate). Spec below is
> detailed enough that the council VERIFIES rather than redesigns; deviations
> require a numbered factual error, not taste.
>
> **Owner:** the tesseract-rs OCR arc (authored the `ocr_actions` v2 table +
> the `tesseract-ogar` executor + the `doc.v1` recognition surface).
>
> **Ask (operator, 2026-07-12):** "take ownership of the ogar-ocr / ogar-doc
> (PDF, spiredoc style reconstruction to be able to update documents with
> updated knowledge, and to store documents as raw references while storing
> the awareness in the SoA graph substrate)."
>
> **One sentence:** `ogar-ocr` (the shipped 14-capability `ocr_actions` table)
> recognizes a document into `doc.v1`; `ogar-doc` (this proposal) PERSISTS it —
> the **raw bytes as a KV reference**, the **awareness (doc.v1 structure) as a
> GUID-keyed subtree in the SoA graph substrate** — and RECONSTRUCTS it — a
> `reconstruct_document` ActionDef re-emits an updated PDF from the graph, so a
> document can be re-issued with **updated knowledge**.

## D1. Current state (verified in-repo, 2026-07-12)

- **`ogar-ocr` = `ogar_vocab::ocr_actions`** — 14 `ActionDef`s (the v2 table,
  merged), executed by `tesseract-ogar` via the `HOT_PLUG` fuse. `recognize_document`
  (row 10) is the one-shot: image → `doc.v1` JSON (regions: text / **table** with
  cell grids / figure / header / footer · typed fields · quality) + fields.
- **The deferred mint, now triggered.** `OCR-ACTIONS-V2-PROPOSAL.md` V2-2 reserved
  a `typed_field` concept (would-be `0x080A`) with the explicit trigger:
  *"NOT needed until a consumer persists harvested fields as graph nodes."* The
  operator's "store the awareness in the SoA graph substrate" IS that trigger.
- **Minted `0x08XX` concepts today:** `unicharset 0x0801 … ocr_renderer 0x0809`
  (nine). `0x080A` is the reserved next slot.

## D2. The model — raw as reference, awareness as SoA subtree

The GUID canon (`CLAUDE.md` P0): a node is `key(128 GUID) + value(≤496 B)`; the
key prerenders the node with zero value decode. A whole `doc.v1` does not fit one
value — so a document is a **GUID-keyed subtree**, not one node:

```
document            (classid = document 0x080A · APP_PREFIX)   ← the root node
  value: raw-ref = { sha256, kv_key, mime, page_count, mean_conf }  (NOT the bytes)
  ├─ page[n]        (page_layout 0x0807)
  │   ├─ region[k]  (page_layout 0x0807 · region kind in the value byte-register)
  │   │   └─ cell / line …                                    ← table grids, text
  │   └─ figure[j]  (page_image 0x0808 · bbox in the register)
  └─ field[m]       (typed_field 0x080A→relabel, or a fresh 0x080B)  ← invoice fields
```

- **Raw as reference (KV):** the original bytes live in the consumer's blob
  store (MedCare's encrypted `file_filelist`, a Lance blob column, S3 — the
  consumer's choice). The `document` node's value carries only the **reference**
  (`sha256` + storage key + mime + counts). Iron rule: **the awareness never
  re-embeds the raw bytes** — the raw is addressable, not duplicated.
- **Awareness as SoA (graph):** the `doc.v1` structure decomposes into the
  subtree above, one GUID-keyed node per page / region / table-cell / field.
  The doc.v1 `table` cell grids and the typed-field harvest become **addressable
  graph nodes** — this is the "awareness in the SoA graph substrate." Each is a
  facet on the 4+12 register (`E-V3-FACET-4-PLUS-12`): the region kind, the bbox
  rails (`X:Y`), the cell (`row:col`) all read from the 12-byte payload the
  `document`/`page_layout` ClassView projects.
- **classid stays pure address.** The `document`/`typed_field` concepts are
  addresses; the persistence + reconstruction magic is `ActionDef` on the Core
  node (OGAR-AS-IR: behavior on the node, not the address).

## D3. The two new concept mints (canon change — council-gated)

| concept | proposed classid | role |
|---|---|---|
| `typed_field` | `0x080A` (the v2-reserved slot) | one harvested field (invoice netto/ust/IBAN …) as a graph node — value carries `value_cents`, `checks`, bbox rail |
| `document` | `0x080B` | the subtree root — value = the raw-ref, NOT the bytes |

Both under the `0x08` OCR family, **canon-high** (concept in hi u16; APP_PREFIX
in lo u16 per D-CLASSID-CANON-HIGH-FLIP). Exact hex is the canon's to pin; this
proposal honors the v2 `typed_field → 0x080A` reservation and takes `0x080B` for
`document`. No existing concept edited (append-only canon).

## D4. The three new ActionDefs (facts only — V2-4 rule)

| capability | subject | mandatory | produces | kausal |
|---|---|---|---|---|
| `persist_document` | `document 0x080B` | `doc_json, raw_sha256, raw_kv_key` | `document_guid` | `External` |
| `read_document` | `document 0x080B` | `document_guid` | `doc_json` (rehydrated from the subtree) | `External` |
| `reconstruct_document` | `document 0x080B` | `document_guid, template` | `pdf_bytes` | `External` |

- `persist_document` — decompose `doc.v1` → the SoA subtree + store the raw-ref.
  Facts only; HOW the nodes are written (mailbox / Lance) is the executor seam's
  business, exactly as the OCR rows.
- `reconstruct_document` — the **Spire.Doc-style** re-emit: walk the awareness
  subtree, bind it into a `template`, render a PDF. Because the awareness is
  graph nodes, **updating a field node then re-firing `reconstruct_document`
  re-issues the document with the updated knowledge** — the operator's headline.
  The renderer is the existing `ogar-render-askama` template surface + the proven
  `tesseract-ocr-pdf::render_searchable_pdf` brick (raster + text layer); a
  data-only re-issue (no original raster) uses the template alone.
- Executor: a NEW `ogar-doc` executor crate (or a `tesseract-ogar` extension) —
  TBD by the council; the hot-plug fuse binds it, same as `tesseract-ogar` binds
  the OCR rows.

## D5. OGAR-AS-IR §3 — the six IR-shape tests

1. *SSA/dataflow-explicit?* N/A — a declared capability + concept table, not a
   lowering. 2. *Effect-annotations first-class?* YES — `reads`/`writes` on the
   three ActionDefs. 3. *Typed signature not field-bag?* YES — `OcrActionParam`
   signatures. 4. *Named lowering passes?* N/A. 5. *Semantic-preservation?* YES —
   additive; no existing row/concept edited. 6. *IR canonical?* YES — the vocab
   table + the concept mints are the canonical IR the `ogar-doc` executor
   resolves against.

## D6. SURREAL-AST-TRAP-PREFLIGHT

Not a producer→IR / codegen / `.surql` session. Behavior lands as `ActionDef`
facts in the vocab (sanctioned home); the document subtree is graph DATA (SoA
nodes), not a `DEFINE EVENT … THEN` lifecycle in DDL. The persistence + the
reconstruction are `ActionInvocation` at runtime, on the Core node. Verdict: ON
the sanctioned path by construction.

## D7. Consumers inherit it — no per-consumer wiring

MedCare-rs (the immediate consumer): on a DMS upload, the executor fires
`recognize_document` → `persist_document` (raw-ref = the encrypted `file_filelist`
key; awareness → the SoA subtree). A search or a re-issue fires `read_document`
/ `reconstruct_document`. **Zero tesseract-rs / doc-model types in MedCare** — it
pulls the `document` classid via `lance_graph_contract::ogar_codebook::
canonical_concept_id` (the BBB membrane, OGAR-CONSUMER-BEST-PRACTICES §2 1b) and
invokes. Every consumer (woa-rs, smb-office-rs, odoo-rs) inherits the same layer.
This is why the capability is OGAR-level, not per-consumer — the dep-graph
landmine of pulling `tesseract-ogar` into each customer binary is avoided: they
pull the classid + invoke through the membrane.

## D8. Non-goals (this proposal)

- No storage backend chosen (KV blob is the consumer's — Lance column / encrypted
  file / S3; `ogar-doc` sees a reference, not a store).
- No template DSL invented — reuse `ogar-render-askama`; a doc-template vocabulary
  is a follow-up if the askama surface proves insufficient.
- No streaming/chunked payload (executor seam).
- No `.surql` / DDL anywhere (D6).

## D9. DISCOVERY-MAP entry (mandatory per CLAUDE.md)

Append `D-OGAR-DOC-LAYER` to `docs/DISCOVERY-MAP.md`: `ogar-doc` persists `doc.v1`
as a raw-ref (KV) + an awareness subtree (SoA graph), reconstructs it via a
`reconstruct_document` ActionDef; triggers the v2-deferred `typed_field` mint +
a `document` mint. Grade `[S]` (spec) until the council + a probe promote it.
