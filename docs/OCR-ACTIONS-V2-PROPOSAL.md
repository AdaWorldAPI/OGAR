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
- **V2-2 (zero new mints, at the time this table shipped):** every subject
  is an ALREADY-MINTED `0x08XX` concept. `page_layout` (`0x0807`) is the
  natural subject for layout-DOM-level actions (11, 14); `page_image`
  (`0x0808`) for pixel-level actions (9, 10, 12, 13). Deliberate deferral,
  as originally written: a `typed_field` concept mint (would-be `0x080A`)
  is NOT needed until a consumer persists harvested fields as graph nodes —
  the trigger condition is recorded here so the deferral is a decision, not
  an omission. **Status 2026-08-25 — the deferred trigger fired:** the W4
  5+3 council minted `typed_field` at exactly `0x080A` (`D-OGAR-DOC-LAYER`,
  `DISCOVERY-MAP.md`) once `paperless-rs` needed to persist harvested
  fields as graph nodes. This 14-row table itself is unchanged — the new
  mint's ActionDefs (`persist_document`/`read_document`/
  `reconstruct_document`) landed in their own `document_actions.rs` table,
  not here (Deviation D-1: `resolve_hotplug` gates per contributing table,
  so growing this table would have entangled a non-OCR consumer into
  `OCR_EXPECTED_EXECUTORS`).
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

## C7. Non-goals (v2, at the time this table shipped)

- No `typed_field` mint (trigger recorded in V2-2). **Status 2026-08-25:**
  the trigger fired — see V2-2's status paragraph and `D-OGAR-DOC-LAYER`
  (`DISCOVERY-MAP.md`). Still true of THIS v2 table specifically: the mint's
  ActionDefs did not land here (Deviation D-1, see V2-2).
- No streaming/chunked payload story (executor seam's business).
- No REST/HTTP surface anywhere in OGAR (lab-vs-canonical rule).
- No deu.lstm / multi-language slot — `language` is NOT added in v2
  because the executor cannot honor it yet (eng-only model shipped);
  adding a dead param would be a lie in the facts. Recorded as the v3
  trigger: the slot lands WITH the multi-model executor.

---

# CONSOLIDATION — Phase 1 (5 verification savants, 2026-07-10)

> 5/5 savants completed (0 errors, ~1M tokens). Lens 1 (vocab) 9/9 CONFIRMED.
> The corrections below are folded in BEFORE the 3 brutal reviewers so they
> attack a clean spec (operator protocol: "consolidate first, otherwise the
> 5+3 becomes mushy"). Each is evidence-backed (file:line verified in-repo,
> the load-bearing fuse correction re-verified directly by the orchestrator).

## Corrections applied (supersede the body above where they conflict)

- **[Lens 4 — LOAD-BEARING] The live fuse is `resolve_hotplug` + `HOT_PLUG`,
  NOT `verify_ocr_registration`.** Verified: `ogar-vocab/src/
  capability_registry.rs:242` `resolve_hotplug`; the green test is
  `hotplug_activation_is_green` (:323) calling
  `resolve_hotplug("tesseract-ogar", OCR_IDS, OCR_COVERED)`; the executor's
  live surface is `tesseract-ogar::{COVERED_CAPABILITIES, HOT_PLUG}` +
  `const _: () = assert!(OCR_ACTION_NAMES.len() == COVERED_CAPABILITIES.len())`
  (tesseract-ogar/src/lib.rs:61,81,94). `verify_ocr_registration` still
  compiles + self-tests but is NOT this executor's live path. C1/V2-6 are
  corrected to name the hot-plug surface.
- **[Lens 4 GAP + Lens 2 CORRECTED] Implementation touches THREE fuse sites,
  all in-PR:** (a) OGAR `OCR_ACTION_NAMES` 8→14 + `const _` assert 8→14;
  (b) OGAR `OCR_SUBJECT_CLASSIDS` += `class_ids::PAGE_LAYOUT` (the only net-new
  subject — PAGE_IMAGE already present); (c) the `#[cfg(test)]` mirrors in
  `capability_registry.rs` (`OCR_IDS` += `0x0807`, `OCR_COVERED` → 14) — the
  spec now mandates converting these test mirrors to LIVE references
  (`ocr_actions::{OCR_SUBJECT_CLASSIDS, OCR_ACTION_NAMES}`) so they can never
  drift again. tesseract-ogar side: `COVERED_CAPABILITIES` 8→14 + the
  `hotplug_activation_is_green` concept count 3→4.
- **[Lens 2 CORRECTED — merge order] The interim break is a HARD COMPILE
  FAILURE of the whole tesseract-rs workspace** (the `const _` assert + the
  unpinned sibling path-dep on ogar-vocab), not "a registration test goes
  red." Consequence for C3: OGAR PR merges FIRST; the tesseract-rs PR
  (executor + `COVERED_CAPABILITIES`=14) must be ready to merge in lockstep,
  because tesseract-rs `main` will fail `cargo build` against OGAR `main`=14
  until it lands. Both PRs authored together this session.
- **[Lens 2 GAP — DROP `classify_regions`] Row 10's optional `classify_regions`
  param is REMOVED.** No code precedent (repo-wide grep = 0); the shipped web
  arm classifies regions unconditionally. Row 10 optional params are now just
  `with_dict, harvest_profile`. A future cheap-path toggle (skip
  classification) is recorded as a v3 trigger, not shipped as a dead flag.
- **[Lens 3 CORRECTED — C4] The real OGAR-AS-IR §3 tests** (OGAR-AS-IR.md:57-62)
  are: (1) SSA/dataflow-explicit, (2) effect-annotations-first-class,
  (3) typed-signature-not-field-bag, (4) named-lowering-passes,
  (5) semantic-preservation-guarantee, (6) IR-is-canonical. Re-answered:
  (1) N/A — this is a declared capability table, not a lowering (no dataflow
  to make explicit); (2) YES — `reads`/`writes` are the first-class effect
  annotations, exactly as the existing 8; (3) YES — `OcrActionSpec.params`
  (`OcrActionParam{name,mandatory}`) IS the typed signature, not a field bag;
  (4) N/A — no new lowering pass; (5) YES — additive, no existing row edited,
  semantics of the 8 preserved; (6) YES — the vocab table is the canonical IR
  the executor resolves against. The change PASSES (the 3 applicable tests),
  and my original C4 six invented questions are withdrawn.
- **[Lens 3 GAP — BBB path] The woa-rs consumer sketch (C3) violates no
  anti-pattern, but must resolve via the membrane:** woa-rs (a BBB / customer
  binary, woa-rs Iron Rule 1) pulls `PAGE_IMAGE` (0x0808) through
  `lance_graph_contract::ogar_codebook::canonical_concept_id`, NOT
  `ogar_vocab::ports` directly (OGAR-CONSUMER-BEST-PRACTICES.md §2 Pattern 1b).
  The invocation stays behavior-free at the address; behavior is at the
  tesseract-ogar executor. Sketch remains SPEC-ONLY.
- **[Lens 5 CORRECTED — V2-7 docs] The six rows land in `ocr_actions.rs`'s own
  module-doc capability table** (mirroring the existing 8-row table there), NOT
  in `docs/ARAGO-ACTIONHANDLER-PARITY.md` — verified (401-line read) to be the
  generic arago-protocol scorecard with ZERO OCR content. V2-7 is corrected.
- **[Lens 5 GAP — ledger] Append a `docs/DISCOVERY-MAP.md` D-entry** for the v2
  table growth (CLAUDE.md marks it mandatory; v1 shipped without one — a
  pre-existing gap this change closes rather than repeats).

## Net implementation delta (what the 3 reviewers should hold the diff to)

OGAR: `ocr_actions.rs` +6 `OcrActionSpec` rows (subjects: rows 9/10/12/13 →
`page_image` 0x0808, rows 11/14 → `page_layout` 0x0807), `OCR_ACTION_NAMES`
8→14, `const _` 8→14, `OCR_SUBJECT_CLASSIDS` += `PAGE_LAYOUT`, module-doc table
+6 rows; `capability_registry.rs` test mirrors → live refs; `DISCOVERY-MAP.md`
+1 D-entry. tesseract-rs: `tesseract-ocr::recognize_document` composition helper
(+ web arm refactored onto it, DRY), `tesseract-ogar` +6 request/response
variants + execute/capability_of/param-map arms + `COVERED_CAPABILITIES` 8→14 +
hotplug concept-count 3→4. Merge OGAR first; tesseract-rs in lockstep.
