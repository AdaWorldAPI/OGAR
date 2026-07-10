# OCR Action Surface v2 — PROPOSAL (council pending)

> **Status:** PROPOSAL — 5+3 council scheduled (5 verification savants →
> consolidate → 3 brutal reviewers → fix → consolidate). Per operator
> protocol the spec below is deliberately detailed enough that the council
> VERIFIES rather than redesigns; deviations require evidence of factual
> error in a numbered claim, not taste.
>
> **Ask (operator, 2026-07-10):** "add an API for OGAR … so that ogar can
> allow OCR via ogar-vocab in the other consumers … needs actiondef."
>
> **One sentence:** extend the existing hand-authored OCR ActionDef table
> (`ogar_vocab::ocr_actions`, 8 capabilities) with the six capabilities the
> tesseract-rs arc shipped since the table was authored — word-level page
> recognition, the doc.v1 structured document, the typed invoice-field
> harvest, page segmentation (XY-cut / deimposition), halftone (figure)
> detection, and page-furniture detection — so consumers (woa-rs,
> medcare-rs, smb-office-rs, …) invoke them through ogar-vocab like every
> other Core capability, with the registration fuse forcing the executor
> to stay in lockstep.

## C1. Current state (verified in-repo, 2026-07-10)

- `ogar_vocab::ocr_actions` (crates/ogar-vocab/src/ocr_actions.rs, 445
  lines) declares EIGHT capabilities as real `ActionDef`s:
  `recognize_line, recognize_page, extract_text_layer, extract_page_image,
  render_text, render_tsv, render_hocr, render_searchable_pdf`.
- Pattern (all preserved by v2): hand-authored table (sanctioned for
  tesseract-rs — no upstream AST to lift from, per the module doc);
  `object_class = "ogit-ocr/<concept>"` with `<concept>` minted in
  `class_ids::ALL` under `0x08XX`; `kausal = Some(KausalSpec::External)`;
  `default_subject = ActionSubject::System`; `reads`/`writes` carry
  name-level effect facts; `OcrActionSpec { def, params, produces }`
  carries the arago-parity typed signature (`OcrActionParam
  { name, mandatory }`); `OCR_ACTION_NAMES` is the const fingerprint;
  `OCR_SUBJECT_CLASSIDS` lists the bound concepts;
  `verify_ocr_registration` is the drift fuse
  (`capability_registry::verify_registration`).
- Minted `0x08XX` concepts: `unicharset 0x0801, recoder 0x0802, charset
  0x0803, network_layer 0x0804, textline 0x0805, blob 0x0806, page_layout
  0x0807, page_image 0x0808, ocr_renderer 0x0809`.
- Executor expectation: `OCR_EXPECTED_EXECUTORS = ["tesseract-ogar"]`
  (crate exists in tesseract-rs: crates/tesseract-ogar).
- The library capabilities shipped in tesseract-rs since the table was
  authored (all merged to master; PRs #29-#32 + branch e517a60):
  `LstmRecognizer::recognize_page_makerow_words` (word/box page API),
  `structured::{DocPage, render_json, render_json_with_regions,
  build_regions, harden_numeric_tokens, harvest_fields,
  german_invoice_fields}` (doc.v1 + typed fields), `xy_cut::xy_cut`
  (recursive segmentation / deimposition), `pageseg::
  generate_halftone_mask` (leptonica-parity figure detector),
  `page_furniture::detect_page_furniture` (header/footer/page number).

## C2. Proposed v2 table — exact rows

Six NEW capabilities appended to the existing eight (table order below =
`OCR_ACTION_NAMES` order; existing rows unchanged and not repeated):

| # | capability | subject concept | mandatory params | optional params | produces |
|---|---|---|---|---|---|
| 9 | `recognize_page_words` | `page_image` (`0x0808`) | `grey_page, width, height` | `with_dict` | `line_words` |
| 10 | `recognize_document` | `page_image` (`0x0808`) | `grey_page, width, height` | `with_dict, harvest_profile, classify_regions` | `doc_json, fields` |
| 11 | `harvest_fields` | `page_layout` (`0x0807`) | `line_words, page_w, page_h, harvest_profile` | — | `fields` |
| 12 | `segment_page` | `page_image` (`0x0808`) | `grey_page, width, height` | `min_gap_frac, min_region_px, max_depth` | `regions_rects` |
| 13 | `detect_halftone_regions` | `page_image` (`0x0808`) | `binary_page, width, height` | — | `figure_rects, mask_w, mask_h, found` |
| 14 | `detect_page_furniture` | `page_layout` (`0x0807`) | `line_words, page_w, page_h` | — | `header_lines, footer_lines, page_number` |

Numbered claims the council verifies (not redesigns):

- **V2-1 (granularity):** `recognize_document` is the ONE-SHOT consumers
  need (image in → doc.v1 JSON + typed fields out — the exact composition
  the tesseract-ocr-web JSON arm runs: words → DocPage → harden →
  harvest(profile) → furniture+xy_cut+halftone → build_regions →
  render_json_with_regions). The fine-grained rows (9, 11, 12, 13, 14)
  exist for consumers that already hold intermediate artifacts. Both
  granularities mirror the existing table's own split (recognize_page vs
  render_*).
- **V2-2 (zero new mints):** every subject is an ALREADY-MINTED `0x08XX`
  concept. `page_layout` (`0x0807`) is the natural subject for
  layout-DOM-level actions (11, 14); `page_image` (`0x0808`) for
  pixel-level actions (9, 10, 12, 13). Deliberate deferral: a
  `typed_field` concept mint (would-be `0x080A`) is NOT needed until a
  consumer persists harvested fields as graph nodes — the trigger
  condition is recorded here so the deferral is a decision, not an
  omission.
- **V2-3 (harvest_profile vocabulary):** `harvest_profile` is a string
  slot with ONE defined value in v2 — `"german_invoice"` (maps to
  `tesseract_ocr::german_invoice_fields()`); absent/empty = no harvest
  (empty `fields`). Unknown profiles are an executor-side invocation
  FAILURE (fail-closed), not a silent no-harvest: a typo must not
  silently drop invoice validation. Future profiles extend the
  vocabulary; the ActionDef facts do not change.
- **V2-4 (fields are FACTS, executor stays consumer-private):** per the
  AR-OGAR mailbox plan §4 rule, the v2 rows carry facts only (predicate,
  subject, reads/writes, External kausal). No exec binding, no transport,
  no encoding enters ogar-vocab. How `grey_page` bytes travel (same
  process slice, mailbox payload, REST) is the consumer/executor seam's
  business, exactly as for the existing eight rows.
- **V2-5 (naming):** `line_words` (not `textlines`) names the word/box
  unit (tesseract-rs `LineWords`: words with per-char boxes + confidences
  per line) to distinguish it from row 2's flat `textlines` text output.
  `regions_rects` are reading-ordered `(l,t,r,b)` rects (xy_cut leaves);
  `figure_rects` are halftone component bboxes in page space (mask may be
  smaller than the page — `mask_w`/`mask_h` carried so the consumer can
  interpret; the found=0 arm yields empty rects and found=false).
- **V2-6 (fuses extend, not fork):** `OCR_ACTION_NAMES` grows to 14 (the
  `const _` length assert updates 8 → 14); `OCR_SUBJECT_CLASSIDS` is
  UNCHANGED (`{TEXTLINE, PAGE_IMAGE, OCR_RENDERER}` ∪ v2 subjects adds
  `PAGE_LAYOUT` — so it DOES change: `+ class_ids::PAGE_LAYOUT`);
  `verify_ocr_registration` therefore forces every registered executor to
  either handle all 14 or fail registration — the fuse IS the pairing
  mechanism that keeps tesseract-ogar in lockstep, by design.
  (Council: note the deliberate self-correction in this claim — the
  subjects set changes by exactly {PAGE_LAYOUT}; verify.)
- **V2-7 (docs pairing):** `docs/ARAGO-ACTIONHANDLER-PARITY.md` (the
  8-capability parity table the module doc cites) gains the six v2 rows
  marked "tesseract-rs-native (no arago counterpart)" — the honest
  labeling that these six have NO arago twin; parity claims stay scoped
  to the original eight.

## C3. Executor pairing (tesseract-rs side, separate PR)

- `tesseract-ogar` extends its executor match to the six new predicates,
  each a thin call into the already-public tesseract-ocr functions listed
  in C1 (no new logic in the executor — composition only, mirroring the
  web crate's JSON arm for `recognize_document`).
- Merge order: OGAR PR first (table + fuse), tesseract-rs PR second
  (executor catches up; its registration test goes green again). The
  interim red on the tesseract-rs registration test is the fuse working
  as designed — same pattern as the lance-graph #556 / tesseract-rs #3
  merge-order note in tesseract-rs CLAUDE.md.
- Consumer sketch (woa-rs, IN THE SPEC ONLY — not shipped in v2):
  resolve classid via `PAGE_IMAGE`, fire `recognize_document` with
  `harvest_profile = "german_invoice"`, read back `doc_json` + `fields`
  (netto/ust/brutto with `arithmetic_ok`, IBAN with `iban_mod97_ok`) —
  the Rechnungs-Erfassung path with zero tesseract-rs types in woa-rs.

## C4. OGAR-AS-IR §3 — the six IR-shape tests, answered

1. *Facts or policy?* Facts only (V2-4). 2. *Effect annotations
   name-level?* Yes — reads/writes mirror params/produces exactly as the
   existing eight. 3. *Behavior on the Core node, not the address?*
   Yes — capabilities attach to `0x08XX` concepts via ActionDef; classids
   stay pure address. 4. *Adapter-neutral?* Yes — no transport/encoding
   in the table; executors register. 5. *Additive, version-gated?*
   Additive rows + fuse-count bump; no existing row edited. 6. *Lift or
   hand-authored, and why?* Hand-authored, sanctioned: tesseract-rs has
   no upstream AST (the table's own module doc is the precedent).

## C5. SURREAL-AST-TRAP-PREFLIGHT — answered for this change

Not a producer→IR / codegen / .surql session (no DDL, no lift). The five
questions collapse to: behavior lands as `ActionDef` facts in the vocab
(the sanctioned home), invocation stays `ActionInvocation` at runtime,
no lifecycle enters any adapter. Verdict: the change is ON the sanctioned
path by construction.

## C6. Test plan (OGAR side)

- Existing module tests extend mechanically (length 14, uniqueness,
  minted-subject fuse now also passing PAGE_LAYOUT, External kausal,
  identity well-formedness — all loops, no per-row edits needed).
- NEW: `recognize_document_reads_cover_composed_inputs` — row 10's reads
  ⊇ row 9's mandatory reads (the one-shot cannot need less than its
  first stage); `harvest_profile_documented_value_is_stable` — the
  `"german_invoice"` string is pinned (a rename is a breaking change).
- Registry: a `verify_ocr_registration` negative test — a registration
  carrying only the original 8 must FAIL with the missing-capability
  drift (the fuse proof).

## C7. Non-goals (v2)

- No `typed_field` mint (trigger recorded in V2-2).
- No streaming/chunked payload story (executor seam's business).
- No REST/HTTP surface anywhere in OGAR (lab-vs-canonical rule).
- No deu.lstm / multi-language slot — `language` is NOT added in v2
  because the executor cannot honor it yet (eng-only model shipped);
  adding a dead param would be a lie in the facts. Recorded as the v3
  trigger: the slot lands WITH the multi-model executor.
