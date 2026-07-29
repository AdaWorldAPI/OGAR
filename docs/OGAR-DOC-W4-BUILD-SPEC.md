# ogar-doc W4 (doc-layer wave) — Persistence + Reconstruction — BUILD SPEC v2

> **Status:** BUILD SPEC v2 (2026-07-16) — the persistence half of `D-OGAR-DOC-LAYER`
> (charter `docs/OGAR-DOC-LAYER-PROPOSAL.md`, merged #191), reconciled with:
> (a) the as-built `D-DOC-IR-SECOND-RETINA` W1/W2/W3 (`ogar-doc-ir` #197 ·
> `ogar-from-docv1` #202 · `spider_doc_ir` merged), and — new in v2 —
> (b) the **A2UI screen-addressing wave** (#204 #205 #206 #207:
> `docs/A2UI-SCREEN-ADDRESSING-PROPOSAL.md`, `ogar-a2ui-frame` W1 CODED,
> `ogar-render-askama::field_view` W2/W3 CODED, ledger `D-A2UI-SCREEN-ADDRESSING`)
> and the a2ui-rs consumer as-built (#3 server RBAC projection · #4 wasm
> FieldviewClient · #9/#11 paint skins + wgpu). v1 of this spec predated a merged consumer of its
> own template brick; v2 corrects that. Detailed enough that the 5+3 council
> VERIFIES rather than redesigns; deviations require a numbered factual error,
> not taste.
>
> **Operator gate (unchanged):** the two canon mints (`0x080A`/`0x080B`) land WITH
> the council-verified build, **never ahead of it** (the `D-OGAR-DOC-LAYER` ledger
> ruling, `DISCOVERY-MAP.md:1300-1301`; mints council-gated per charter §D3;
> D-DOC-IR-SECOND-RETINA "operator-gated"). This doc mints NOTHING — it is the
> blueprint the mint approval unlocks.
>
> **One sentence:** W1/W2/W3 gave us the source-agnostic perceptual IR
> ([`ogar_doc_ir::DocIr`]) from two retinas; **W4 PERSISTS a `DocIr` as a
> GUID-keyed SoA subtree + a raw KV reference, and RE-PROJECTS it — as a PDF
> artifact (executor leg) or as a living addressed screen (a2ui session tier) —
> from ONE `ClassView × WideFieldMask` template.** "One template projection;
> re-issue a document as PDF or serve it as a living screen — same mechanism,
> different renderer" (`A2UI-SCREEN-ADDRESSING-PROPOSAL.md:37`).

## W4-0. Naming — two wave namespaces (collision note, new in v2)

The A2UI arc defines its **own** W0–W5 (`A2UI-SCREEN-ADDRESSING-PROPOSAL.md`
C5:176-179): its **W4 = P-REHOST** (re-host one harvested MedCare screen) and
W5 = the rdp-2graph session. With a2ui W1 + W2/W3 already CODED, a2ui-W4 is
imminent. **An unqualified "W4" in board/PR/branch text is therefore ambiguous.**
This spec's wave is the DOC-LAYER wave: write it **doc-W4** in ledger entries, PR
titles, and branch names. (This file's §-anchors keep the short form for internal
cross-refs only.)

## W4-1. The reconciliation — W4 consumes `DocIr`; the screen ruling folds in

The #191 charter said "persist a `doc.v1`." The as-built refines the input by one
hop: both retinas normalize to the **neutral** `ogar_doc_ir::DocIr` (pixel retina
`ogar-from-docv1::from_doc_v1`; DOM retina `spider_doc_ir`). W4's persistence
input is a **`DocIr`**, already source-agnostic — a scan and the HTML of the same
document converge on ONE subtree because `ogar_doc_ir::converges_on_facts`
guarantees equal typed-field facts (P-XRETINA).

The IR was **built for this** — its doc-comments already pin the persistence
mapping onto the 12-byte facet register:

- `BBoxRail` = two `Rail`s = 4 bytes = **two of the six `u8:u8` pairs** of the
  facet register (`ogar-doc-ir/src/lib.rs:127`).
- `TableCell{row,col}` = **one `u8:u8` pair** on the register (`lib.rs:156`).
- `DocIr` = "the `document 0x080B` subtree *before* persistence" (`lib.rs:207`).

New in v2 — the screen convergence is now **ruled and partially shipped**
(`A2UI-SCREEN-ADDRESSING-PROPOSAL.md:24-28`; `DISCOVERY-MAP.md:1390`):

> "A screen and a document are the SAME positional projection — the doc-layer
> amendment **A3** already ruled it for documents (template = ClassView ×
> WideFieldMask; Kopfzeile/main/Fußzeile = positions in the ordered set, same
> brick as the Klickwege menu-quad). A live desktop is that projection with a
> heartbeat." *(On the "Klickwege" phrasing see the A3 note in §W4-5.)*

Consequences W4 inherits verbatim: desktop→window→region→widget is nested
ClassView projection **exactly as a doc-IR `Region` carries `children`**; the
`X:Y` u8:u8 rail on the 256×256 tile is the layout address for both; the 16-byte
key prerenders layout with zero value decode (P0). The operator's wider direction
(a2ui-rs `plans/projectional-knowledge-editor-v1.md:17-18`): *"Document, spreadsheet,
desktop, and CAD are positional projections of the same graph."* W4 is the wave
that puts the document INTO that graph.

## W4-2. The subtree — raw as KV reference, awareness as SoA facets

Per the charter §D2 and P0 "THE GUID IS THE KEY OF KEY-VALUE": a `DocIr` does not
fit one 496-byte value, so a document is a **GUID-keyed subtree**, one node per
structural unit, each a content-blind 4+12 facet the ClassView projects:

```
document           classid = DOCUMENT 0x080B          ← subtree root
  value: raw-ref = { content_sha256[32]→KV key digest, kv_key, mime, page_count,
                     field_count, mean_conf }   (NEVER the raw bytes)
  ├─ page[n]       classid = PAGE_LAYOUT 0x0807  (page number = one u8:u8 typed pair;
  │                                                native w/h = out-of-line provenance)
  │   ├─ region[k] classid = PAGE_LAYOUT 0x0807  (RegionKind in register byte; bbox = 2 rails;
  │   │   │                                        reading_order = the temporal-stream key)
  │   │   ├─ cell   (Table region only) — a FACET under the region, addressed by
  │   │   │          (row:col) u8:u8 + bbox rails; NOT a new classid
  │   │   └─ figure classid = PAGE_IMAGE 0x0808  (raster region; bbox rails; raw tile → KV)
  └─ field[m]      classid = TYPED_FIELD 0x080A  (key/value out-of-line by identity;
                                                   bbox rails; confidence byte)
```

- **Raw as reference (KV):** original bytes live in the consumer's blob store
  (MedCare's encrypted `file_filelist`, a Lance blob column, S3). The root value
  carries only the reference (sha256 + storage key + mime + counts). Iron rule:
  **awareness never re-embeds raw bytes** — the same doctrine as a2ui's
  "don't push pixels" (pixels are the anti-thesis; the fidelity-PDF leg binds the
  original raster from KV at the renderer, never on an addressed surface).
- **Awareness as SoA (graph):** region kind, bbox rails (`X:Y`), cell `(row:col)`,
  field confidence all read from the 12-byte register the ClassView projects
  (`E-V3-FACET-4-PLUS-12`). *Carving note (v2, never-widen canon):* the page
  `number` reads as one `u8:u8` typed-content pair — never a widened u16
  register read; native `width`/`height` are NOT register fields at all — the
  rails are already tile-normalized, so native units stay **out-of-line as
  provenance**, matching the IR's own "provenance for un-quantizing" framing
  (`ogar-doc-ir/src/lib.rs:198`). Text/`key`/`value` strings are out-of-line
  **value-slab** stores keyed by classid+identity (`I-VSA-IDENTITIES`: facet =
  identity + typed dims, never the content register). *Consumption note (v2, from
  the as-built): the a2ui wire today emits only facet positions `< 12`; value-slab
  fields are deliberately skipped, fail-SAFE empty delta
  (`a2ui-rs render_stream.rs:29-46`). See §W4-8 dependency ledger.*
- **The decomposition is wire-proven (v2):** `NodeDelta`'s carving rule — values
  "resolved by the classid's ClassView on the client, never described on the wire
  (content-blind, like the 4+12 facet register)"
  (`ogar-a2ui-frame/src/lib.rs:115-121`) — is the SAME contract as this subtree's
  facet decomposition; the persisted subtree and the wire delta are two
  projections of one ClassView carving. The frame ceilings (u16 mask-word count
  per `NodeDelta`; `FieldView.position: u8` ⇒ ≤256 positions per rendered node
  surface) **reinforce** per-structural-unit decomposition: one frame per node,
  never one giant document frame — the subtree IS the answer.
- **Nesting maps 1:1 onto the shipped mechanism (v2):** document root = an L1
  screen class whose slot positions link to child node keys (`a2ui-wasm`
  `NestedSurface`, lib.rs:158-192; `link_child` lib.rs:358 / `resolve_nested`
  lib.rs:377); page → region → cell = further slot links; composition is
  codebook-layer resolution; **the wire stays a flat NodeDelta stream** (T1/T3).
- **Minimal mints:** only `document 0x080B` + `typed_field 0x080A` are new.
  Page/region reuse `PAGE_LAYOUT 0x0807`; figures reuse `PAGE_IMAGE 0x0808`;
  cells are facets, not a new class ("container kinds, not content" discipline).

## W4-3. The two mints (canon-high — council-gated)

Confirmed free on `main` (highest OCR mint = `OCR_RENDERER 0x0809`,
`ogar-vocab/src/lib.rs` `class_ids`), and the a2ui wave **takes no adjacent
slot** — its ruling C6: widget skins are templates, never concepts; a genuine UI
concept would go through the codebook council-gated, so a2ui claims nothing near
`0x080A`/`0x080B`:

| concept | classid | role | value register |
|---|---|---|---|
| `typed_field` | `0x080A` (v2-reserved slot, charter §D3) | one harvested field as a graph node | `confidence` byte + bbox rails; `key`/`value` out-of-line by identity (value-slab) |
| `document` | `0x080B` | the subtree root | raw-ref (sha256 digest + kv_key + mime + counts) |

Both canon-high (concept in hi u16, APP_PREFIX in lo u16 per
D-CLASSID-CANON-HIGH-FLIP; the client reads it back with
`concept_of_key = (u32 LE at key[0..4]) >> 16` — implemented independently
server- and wasm-side in a2ui-rs). Append-only: no existing concept edited. Exact
hex is the canon's to ratify; this honors the v2 `typed_field → 0x080A`
reservation and takes the next slot `0x080B` for `document`.

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
- `reconstruct_document` — the artifact re-emit: walk the awareness subtree, bind
  a `template`, render a **byte artifact** (PDF; a Word leg later). **v2 ruling —
  `produces = pdf_bytes` is deliberately scoped to the artifact legs:** the
  living-screen path is NOT an ActionDef output. Serving a screen is the **a2ui
  session tier** projecting the SAME subtree over the SAME template through
  `project_node` → NodeDelta frames (`a2ui-server render_stream.rs:20-23`: "one
  template projection; serve it as a living screen or re-issue it as a
  document"). The executor produces artifacts; the session tier produces frames;
  both read one subtree, one template. No signature generalization needed.
- **Ordinal stability (new invariant, v2):** on the a2ui wire an action travels
  by `action_ordinal: u32` — the index into the class's ActionDef set
  (`ogar-a2ui-frame/src/lib.rs:137-145`, trap T2). The `ocr_actions` table
  position of `persist_document` / `read_document` / `reconstruct_document` for
  `0x080B` is therefore a **wire contract: append-only, never reordered** — the
  same discipline as the classid ledger itself.
- **The mutation headline rides T2 machinery available today (v2):** "mutate a
  `typed_field` node then re-fire ⇒ the document re-issues with updated
  knowledge" (the operator's headline) is delivered as **ActionDef invocation**
  (`ActionInvoke` up on the typed_field/document node; masked re-projection
  down) — one round-trip over the shipped frames. A *direct* field-edit up-frame
  (`SetField`) is an **open a2ui council point**
  (a2ui-rs `projectional-knowledge-editor-v1.md:179`), explicitly NOT a W4
  dependency: W4's mutation path is by action address, not by field write.
- `template` optional: a data-only re-issue uses the default template; a
  fidelity re-issue may bind the original raster from the KV raw-ref
  (searchable-PDF path). RBAC scoping of `template` — §W4-5 C1.4 block.

## W4-5. The renderer — one projection, four legs, C1.4 mask discipline

Per `D-DOC-IR-SECOND-RETINA` operator rulings A1–A5, updated to the as-built
(the template ruling is **A3**):

- **A1 — one `ogar-doc` crate.** Split axis is STATE (recognized → persisted →
  reconstructed), not direction. The executor is one crate binding the 3
  ActionDefs via the HOT_PLUG fuse, exactly as `tesseract-ogar` binds the OCR
  rows. **Deps (charter A5, restored in v2):** `ogar-vocab` + `ogar-doc-ir`
  (`OGAR-DOC-LAYER-PROPOSAL.md:259`), + `lance-graph-contract` for the HotPlug
  socket types (the `tesseract-ogar` precedent), + `ogar-render-askama` as the
  askama leg's compile-time callee (deviation D-1, under A2); NOT
  `tesseract-ogar` (recognition is a separate, already-fused executor). The
  *codebook membrane* is the CONSUMER-side zero-`ogar-vocab` mirror
  (`OGAR-AS-IR.md`) — it belongs in §W4-6, never in the executor's dep list
  (v1 had it on the wrong side of the BBB; the executor resolves against
  `ogar-vocab` directly, exactly as §W4-7 already requires).
- **A2 — `DocRenderer` trait, the one-leg rule — now FOUR ruled legs**
  (`DISCOVERY-MAP.md:1390`: "`DocRenderer` gains its fourth adapter — a2ui, the
  live interactive surface"), with honest as-built status per leg:

  | leg | output | as-built status |
  |---|---|---|
  | `tesseract-ocr-pdf::render_searchable_pdf` | PDF (raster + text layer) | **shipped, proven** |
  | `ogar-render-askama` (fieldview / detail / list / form) | addressed **HTML** (`String`) | **shipped** (`render_field_view`, `field_view.rs:109`) — but "Askama IS the output; nothing transcodes it further" (`lib.rs:15-17`): the **HTML→PDF step exists nowhere and is named NEW WORK** for the data-only-PDF leg |
  | Spire.Doc-style (Word) | DOCX | future |
  | **a2ui — the live addressed surface** | NodeDelta frame stream | screen half **shipped** (`ogar-a2ui-frame` + `a2ui-server::project_node` + `a2ui-wasm::FieldviewClient` + `a2ui-paint::Skin::Flow`); served by the **session tier**, not the executor (§W4-4) |

  **The trait is NEW work in `ogar-doc` (v2 correction):** `ogar-render-askama`'s
  `ArtifactEmitter`/`for_kind` is compile-time enum dispatch with every leg
  compiled in (`artifact_kinds/mod.rs:54-63`; README: "Not a runtime template
  engine. Templates are compile-time-bound via `#[derive(Template)]`") — it is a
  *callee*, not a liftable runtime-binding pattern. The runtime-bound
  `DocRenderer` boundary lives in `ogar-doc`; its askama leg CALLS
  `ogar-render-askama`'s public fns (`render_field_view`, `from_value_rows` /
  `from_render_rows` over `lance_graph_contract::class_view::{ValueRow,
  RenderRow}`), keeping that crate dependency-free of doc-ir. And per its
  one-template-per-kind doctrine (the 800 → 7-70 collapse, `lib.rs:25-32`):
  **per-classid askama templates are forbidden** — class variation flows through
  data (mask/rows), never through template choice.

  **Impl count (v2 clarifier):** the Rust trait has **three** impls — the
  byte-artifact legs (tesseract PDF, askama+HTML→PDF, Spire.Doc). The a2ui
  "leg" is the fourth member of the projection FAMILY (the DISCOVERY-MAP
  adapter language), realized as the session tier's `project_node` over the
  same subtree × template — **never a trait impl in `ogar-doc`** (it produces
  frames, not bytes; §W4-4).

  **Deviation D-1 (numbered, per this spec's own bar):** charter A5 annotates
  all three renderer adapters "runtime-bound (A2)"
  (`OGAR-DOC-LAYER-PROPOSAL.md:262`). v2 upholds runtime binding for the
  tesseract/Spire.Doc legs (A2's actual rationale) and **revises the askama
  leg to a compile-time callee** — factual basis: `ogar-render-askama` is "Not
  a runtime template engine" (README:35-36); templates compile in via
  `#[derive(Template)]`; `ArtifactEmitter`/`for_kind` is in-crate enum dispatch
  (`artifact_kinds/mod.rs:54-63`). A4 (the second-retina rule) is satisfied by
  the merged W1–W3 and needs no W4 action; A5's crate topology is otherwise
  restored verbatim (see A1).
- **A3 — document template = `ClassView × WideFieldMask`** (numbered A3 per
  `A2UI-SCREEN-ADDRESSING-PROPOSAL.md:24` and `DISCOVERY-MAP.md:1408`). The same
  brick as the **screen-addressing projection** — as-built in
  `a2ui-server::project_surface`/`project_node` and
  `ogar-render-askama::render_class_with_methods_wide` (`rust_class.rs:174`, no
  64-field ceiling) — and the same mask discipline as the Klickwege-arc codegen.
  (v2 phrase fix: "Klickwege" names the interaction-telemetry edge
  (`KlickwegEdge`), not a template type — the shared thing is the projection
  brick.) The template selects which subtree facets bind into which rendered
  slots; evolution is a new mask, never a new language. `WideFieldMask`'s home
  is **`lance_graph_contract::class_view`** (per `field_view.rs:53`); mask bit
  position = layout address = `data-field-pos`; ActionDef ordinal =
  `data-action-ordinal`.

**C1.4 mask discipline — MANDATORY wherever a W4 projection leaves the server**
(operator correction 2026-07-14 + codex P2 on #204, merged #205; inherited
verbatim, new in v2):

1. **RBAC by projection, fail-closed:** what leaves is `template ∩ role`
   (`WideFieldMask::intersect`); an unauthorized field is **absent from the
   output**, not hidden downstream. A role-blind re-issued PDF that embeds every
   `typed_field` regardless of caller role is exactly the pixel-stream hole this
   arc closed (`a2ui-rs project.rs:75`: "A role with no grant NEVER falls back to
   the surface"). **This applies to `reconstruct_document` itself**: a
   role-scoped re-issue binds `template ∩ role` before any leg renders; the
   default template (= `full_for(field_count)`) is sanctioned ONLY as the render
   convenience for system/operator-scope re-issue, **never** as a fallback for a
   missing role mask (the sentinel ban, C1.4(c)). Missing role mask = refusal.
2. **Retype-in-place, no dual vocabulary:** `ClassRbac::field_mask` is retyped to
   `WideFieldMask` directly — NO parallel `field_mask_wide` method (T1 applied to
   the seam itself). W4 code uses the one wide type everywhere.
3. **Zero-extension fail-closed:** where a legacy narrow `FieldMask` meets a wide
   surface, it extends past bit 63 as ZEROS, never as full.
4. **The permit-all identity** (`WideFieldMask::ALL` vs default
   `full_for(field_count)`) **is the a2ui wave's ONE W1 decision** — W4
   **consumes** the decided identity, it does not re-decide it.
5. **RBAC happens before rendering/framing, exactly once** — legs and frames are
   dumb; `ogar-render-askama` deliberately holds zero RBAC logic
   (`field_view.rs:136-139`: "nothing about presence or address is recomputed
   here"), and so must every `DocRenderer` leg.

**classid width note (v2):** subtree nodes carry the 32-bit facet classid
(canon-high); the fieldview's `data-class-id` renders the **u16 concept half**
(`0x%04X`) — a W4 render call passes `(facet_classid >> 16) as u16`, exactly the
`concept_of_key` read.

## W4-6. Consumers inherit it — no per-consumer wiring

MedCare-rs (immediate), woa-rs, smb-office-rs, odoo-rs: on a DMS upload the
executor fires `recognize_document` (existing) → `persist_document` (raw-ref =
the consumer's encrypted blob key; awareness → the subtree). A search / re-issue
fires `read_document` / `reconstruct_document`. **Zero tesseract-rs / doc-model
types in any consumer** — it pulls the `document` classid via
`ogar_codebook::canonical_concept_id` (the BBB membrane) and invokes. This is why
the layer is OGAR-level, not per-consumer (OGAR-CONSUMER-BEST-PRACTICES §2) —
and `ogar-a2ui-frame` itself has zero hot-path deps, so the live-surface path
drags nothing into consumer binaries either. New in v2:

- **Capability shape:** an rdp-2graph session grant over the inclusive classid
  range `0x0800..=0x08FF` + a role `WideFieldMask` authorizes the whole
  OCR/document concept family with the existing range gate
  (`a2ui-server session.rs:98`) — no per-concept wiring.
- **The composed demo (sequencing opportunity):** a2ui-W4 = P-REHOST re-hosts one
  harvested MedCare screen through `ClassView × WideFieldMask`. A re-hosted
  screen whose DATA is a **persisted document subtree** (doc-W4) is the natural
  composed demo — "the legacy app's screen, re-rendered from the graph" showing a
  *persisted invoice* — so landing doc-W4 persistence before/with P-REHOST
  multiplies both waves.
- **Default-template precedent:** `a2ui-paint::Skin::Flow` is the shipped
  document/prose projection over the same resolved surface — the concrete,
  already-tested layout for W4-4's "data-only re-issue uses the default
  template" path.

## W4-7. OGAR-AS-IR §3 + SURREAL-AST-TRAP-PREFLIGHT

- **IR-shape tests (§3):** (2) effect-annotations first-class — `reads`/`writes`
  on the 3 ActionDefs. (3) typed signatures not field-bags —
  `OcrActionParam`-shaped. (5) semantic-preservation — additive; no existing
  row/concept/IR type edited. (6) IR canonical — the vocab table + the two mints
  are the canonical IR the `ogar-doc` executor resolves against. (1)/(4) N/A
  (declared capabilities, not a lowering pass).
- **SURREAL-AST-TRAP:** not a producer→IR / codegen / `.surql` session. Behavior
  lands as `ActionDef` facts (sanctioned home); the subtree is graph DATA (SoA
  facets), not a `DEFINE EVENT … THEN` lifecycle. On the surface, behavior
  travels by ordinal address only (T2 — "`DEFINE EVENT` in DDL and
  `onClick: <lambda>` in a component tree are the same hijack"; no handler is
  representable in the shipped types). **On the sanctioned path by construction.**
- **Consumer-side value-construction precedent (v2):** a2ui-rs `lowering.rs`
  (#209) proves the exact shape W4's executor seam wants — pure
  `&owned-scalars → already-minted OGAR value` construction, zero IR minting,
  zero SPO stamping, unit-testable with nothing running (the
  door-knocking-compiler test).

## W4-8. Non-goals + dependency ledger (v2, sharpened)

**Non-goals (charter §D8, scoped in v2):**
- No storage backend chosen (KV blob is the consumer's).
- No new template DSL (the brick is `ClassView × WideFieldMask`, as-built).
- **The EXECUTOR carries no streaming payload** (v2 scoping: the a2ui-server
  tier legitimately emits NodeDelta frame streams — that is its job, outside
  `ogar-doc`; v1's blanket "no streaming payload" read as contradicting the
  shipped wire).
- No `.surql` / DDL anywhere. No recognition changes (tesseract-rs untouched; W4
  is pure downstream of the merged `DocIr`).
- **No new `FrameKind`** — document nodes ride the existing
  `NodeDelta`/`ActionInvoke` closed vocabulary as ordinary GUID-keyed nodes.
- **No `ArtifactKind::A2uiPayload`** needed by W4 (T6 stays deferred upstream);
  no per-classid askama templates (one-template-per-kind doctrine).
- **No `SetField` write-frame** — W4's mutation path is ActionDef invocation
  (§W4-4); the direct-field-edit frame is a future a2ui council point.

**Dependency ledger — what "view the persisted document as a living screen"
still waits on (honest boundary, new in v2):**
1. **The value-slab render path** (a2ui side): today only facet positions `< 12`
   travel on the NodeDelta wire; `typed_field` **text** (out-of-line by identity)
   is skipped fail-SAFE (`render_stream.rs:29-46` — "a future value-slab render
   path WILL emit them"). The subtree's *structure* (region kinds, bbox rails,
   cell row:col, confidence) is fully addressable today; the field TEXT on-screen
   waits for the value-slab path. Not W4 work; named so the council doesn't
   assume the leg is whole.
2. **The permit-all identity decision** (a2ui W1 — §W4-5 discipline item 4).
3. **The HTML→PDF step** for the data-only-PDF leg (§W4-5 table) — new work,
   named, unowned by any shipped crate.

## W4-9. Falsifiers + the gate

- **P-XRETINA (already on `main`, #199):** `ogar_doc_ir::converges_on_facts(a,b)`
  — same document via both retinas ⇒ same typed-field facts ⇒ ONE subtree.
  `persist_document` MUST be idempotent under this equality (re-persisting the
  DOM retina's `DocIr` of a document already persisted from the pixel retina is a
  no-op/version bump, not a second subtree). This is W4's pre-persist gate.
- **P-TEMPLATE-MASK (new in v2 — clone the shipped falsifier shape):** the
  document-template falsifier clones `E-ONE-MASK-TWO-ENGINES`
  (`ogar-render-askama/tests/mask_dual_target.rs` + its jinja twin over one
  fixture): one `(subtree-fields, template-mask, role-mask)` fixture must yield
  the identical present-field set through (a) the `reconstruct_document` PDF leg
  and (b) the a2ui `project_node` screen leg — proving "one template projection,
  two renderers" and the fail-closed intersection in the same test. **Home (v2,
  dependency-direction honest):** OGAR cannot dep a2ui-rs, so the falsifier is
  SPLIT over one shared JSON fixture — the OGAR half asserts the present-field
  set at the `FieldView`/row level (pre-render, no a2ui dep; gates the doc-W4 PR
  in-repo), and the a2ui-rs half re-reads the SAME fixture through
  `project_node` (the `mask_dual_target` + jinja-twin pattern applied
  cross-repo).
- **Council + mint gate:** this spec is 5+3-council-ready. On a green council +
  the operator's mint approval, `0x080A`/`0x080B` land WITH the executor build —
  packaged as one PR (this spec's v2 packaging proposal; the
  with-the-build-never-ahead ruling itself is the `D-OGAR-DOC-LAYER` ledger,
  `DISCOVERY-MAP.md:1300-1301`, mints council-gated per charter §D3). The
  council output (ratified v3) + the DISCOVERY-MAP
  `D-OGAR-DOC-LAYER` status bump are the board-hygiene deliverables that land in
  the same commit — with every ledger reference written **doc-W4** (§W4-0).
