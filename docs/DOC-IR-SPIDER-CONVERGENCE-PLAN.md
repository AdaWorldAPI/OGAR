# DOC-IR × spider-rs Convergence — Integration Plan v1 (2026-07-13)

> **Status:** PLAN — companion to `OGAR-DOC-LAYER-PROPOSAL.md` (see its
> 2026-07-13 AMENDMENT A1–A5, which this plan sequences). The doc-layer
> 5+3 council gates W1+; W0 is verification-only and runs now.
>
> **Ask (operator, 2026-07-13):** combine recognition and
> processing/representation of knowledge **whether it comes from
> tesseract-rs or spider-rs** (`AdaWorldAPI/spider`, fork of
> `spider-rs/spider`) — "using the JSON-like structuring as a reusable
> input awareness (2D-'spatial' focus of attention)."
>
> **One sentence:** the `doc.v1` structure is promoted from "OCR output"
> to the substrate's **perceptual IR** (`ogar-doc-ir`), with tesseract-rs
> (pixels) and spider-rs (DOM) as its two sanctioned producers — the
> code-side pattern (N language frontends → one closed ndjson → one
> ModelGraph) replayed on perception, so everything upstream (ogar-doc
> persistence, DeepNSM, lance-graph discovery, MedCare-rs as abstract
> document store) consumes ONE shape regardless of retina.

## P0. The convergence thesis (why one IR, stated once)

| code side (shipped) | perception side (this plan) |
|---|---|
| Roslyn / ruff_python / ruff_ruby frontends | tesseract-rs (scan) / spider-rs (web) retinas |
| closed 62-predicate ndjson, `from_ndjson` hard-fail gate | closed region-kind doc IR, `from_json` hard-fail gate |
| `ruff_spo_triplet::ModelGraph` | `ogar-doc-ir` region tree |
| `CompiledClass` in the graph | `document 0x080B` awareness subtree |

spider-rs is an unusually good second retina because **HTML5 already
declares its regions**: `<header>/<main>/<footer>/<table>` are the literal
Kopfzeile/Fußzeile/main positions the amendment's A3 template projects —
the web *self-labels* what OCR must *infer*. Same regions, two
acquisition modes, one IR.

## P1. Access preflight — VERIFIED against source (2026-07-13, this session)

`AdaWorldAPI/spider` is not readable via GitHub-MCP (repo-access gate) but
clones fine over the local git proxy (same transport as MedCare-rs). Cloned
to `/home/user/spider`; all four PF facts verified against the Rust source
(the fork's own `.claude`/`CLAUDE.md` are stale upstream boilerplate and were
NOT used — the seam is grounded in code):

- **PF-1 ✓** Fork of `spider-rs/spider`, default branch `main` @ v2.52.9,
  exactly ONE AdaWorldAPI commit over upstream: `8df3a1b3` *"Add pluggable
  in-process HttpFetchEngine seam (#403)"* — the fork already thinks in
  swappable seams (same shape as `DocRenderer` / OGAR producer seams),
  lowering the risk of wiring a doc-IR producer. Otherwise virgin.
- **PF-2 ✓** Page output surface: `spider::page::Page` exposes `get_html()`
  (String) + `get_url()` + `get_bytes()` + status/headers + optional
  `screenshot_bytes`. That is the W3 producer input.
- **PF-3 ✓ (upgraded from "unknown" to "feasible-later")** Per-element
  geometry is NOT exposed as page output, BUT the CDP primitive is used
  throughout the `chrome` feature path — `bounding_box()` on element handles
  (`features/solvers.rs`) and `getBoundingClientRect()` in injected JS
  (`features/webdriver.rs`, `chrome_common.rs`). So rendered-coordinate rails
  are a KNOWN-FEASIBLE later increment (a CDP/JS pass over the region
  landmarks reusing that in-repo primitive), not speculative. W3 v1 ships
  DOM-order pseudo-geometry (below); rendered rails are the increment.
- **PF-4 ✓** Harvest crate `spider_doc_ir` lands as a workspace sibling next
  to **`spider_agent_html`** — which is NOT an empty field: it already ships
  `clean_html_with_profile` / `smart_clean_html` / `CleaningIntent` /
  `HtmlCleaningProfile{Raw,Default,Aggressive,Slim,Minimal,Auto}` with
  `from_content_analysis` — a content-relevance layer the region harvest
  builds ON, not around.

## P2. Wave plan

### W0 — council gate (no code)

The doc-layer 5+3 council verifies the AMENDED proposal: mints
`typed_field 0x080A` + `document 0x080B`; one-crate `ogar-doc`;
`ogar-doc-ir` as neutral tissue; the A2 one-leg runtime-binding rule; the
A3 ClassView × WideFieldMask template. **The council verifies the
source-agnostic boundary — a spec that only a tesseract producer could
satisfy is a numbered factual error.**

### W1 — `ogar-doc-ir` (the IR crate; gates everything downstream)

- Types: region tree (page / region / table+cells / figure / header /
  footer), typed fields, quality.
- **Closed region-kind vocabulary**; `from_json` HARD-FAILS on unknown
  kinds; explicit `doc.v1` version marker (v2 = a new marker, never a
  silent reshape).
- **Spatial rails:** every bbox quantized to u8 × u8 unit square (the
  `X:Y` rail; the page IS a 256×256 tile on the existing centroid/Morton
  machinery). Raw coordinates = optional provenance, never the address.
- **Provenance lane:** source enum (`ocr` / `dom`) + per-source confidence
  semantics (recognition confidence ≠ crawl trust; no shared float).
- **Identity:** content sha256 as the subtree dedup key.
- Deps: serde only. Tests: vocab closure, gate hard-fail on unknown kind,
  rail quantization round-trip, version-marker refusal.

### W2 — tesseract-rs producer migration (byte-parity gated)

`structured::{DocPage, render_json_with_regions, …}` emits `ogar-doc-ir`
types; the **existing doc.v1 JSON output is the committed golden** and the
migration must reproduce it byte-for-byte (same discipline as the sono
parity witness: the shipped artifact gates its own refactor). tesseract-rs
gains a dep on `ogar-doc-ir`; `ogar-doc-ir` never depends back.

### W3 — spider-fork producer (`spider_doc_ir` harvest crate, in the fork)

**Grounded in the verified surface (PF-2/3/4).** `spider_agent_html` parses
with **`lol_html`** (Cloudflare's streaming CSS-selector rewriter — no
random-access DOM tree), and that is a *gift* for region harvesting, not a
constraint:

- **HTML5 landmarks are CSS selectors → lol_html element handlers.**
  `header`→Header, `main`/`article`→Main, `footer`→Footer, `nav`→furniture,
  `table`→Table (with nested `tr`/`td`/`th` handlers → cell grids),
  `figure`/`img`→Figure. The harvest is a rewriter with one handler per
  landmark selector, emitting `ogar-doc-ir` region nodes as the stream
  passes.
- **Reading order is free.** lol_html is forward-only streaming = document
  order = the reading-order the IR needs (and the temporal stream DeepNSM
  consumes). No sort, no tree walk.
- **DOM-order pseudo-geometry is free.** The streaming index → u8 rail
  (top-to-bottom) IS the v1 spatial rail — lol_html gives no layout, but
  document order is exactly the honest v1 the amendment specified. Rendered
  rects (PF-3, the `bounding_box()` path) are the later increment; the
  provenance lane marks which geometry mode produced the rails.
- **Furniture/boilerplate discrimination is half-done.** Reuse
  `HtmlCleaningProfile::from_content_analysis` / `CleaningIntent` to mark
  which regions are content vs chrome — the "detect page furniture" concern
  (header/footer/nav) the OCR side infers, the web side already scores.
- **Typed fields:** microdata / schema.org / `<meta>` → typed-field nodes
  (the DOM analogue of OCR's `harvest_profile`).

Crate: `spider_doc_ir`, sibling to `spider_agent_html`, deps = `lol_html`
(already in the workspace) + `ogar-doc-ir` (git dep floating on main,
D-NEVER-PIN-BUMP). Fixture-driven tests on committed HTML files; no live
crawling in CI (P4). Mirrors `ruff_*_spo`-in-ruff.

### W4 — `ogar-doc` (persistence + template + renderer trait)

Per the amended proposal: `persist_document` / `read_document` /
`reconstruct_document` facts; `DocTemplate` = ClassView × WideFieldMask
(A3); `trait DocRenderer` with tesseract / Spire.Doc / askama adapters
bound at runtime (A2). First consumer: MedCare-rs `routes/dms.rs` upload
→ `recognize_document` → `persist_document` (raw-ref = encrypted
`file_filelist` key) — sonography, histology, invoices as the abstract
document store.

### W5 — upstream foresight seams (small, deliberate, no redesign)

- **DeepNSM** (stays 0-dep): entry point accepts
  `(text_span, region_kind, spatial_rail)` tuples in reading order — the
  tuple IS the contract; no doc-schema dep enters the crate. Gives the
  FSM region priors (a table cell is not a sentence), the temporal stream
  its reading order (per the temporal-stream ruling), and spatial
  adjacency (caption-near-figure).
- **lance-graph discovery arm:** `document 0x080B` registered as a
  discoverable concept; sha256 as the convergence key. Typed fields are
  already SPO facts + rails — "all invoices from X with IBAN Y" is a
  graph query + rail scan, no new machinery.

## P3. Probes (falsifiable; graded [H] until run)

- **P-XRETINA (the killer witness):** the same invoice through both
  retinas (scan → tesseract; HTML → spider) must converge to the same
  **typed-field facts**. Diverse redundancy applied to perception — the
  MySQL-oracle pattern. GREEN promotes the whole IR from [S] to [G]; RED
  means the IR is tesseract-shaped-with-a-second-door and W1 gets corrected
  before more producers land.
  **CORRECTION (2026-07-13, building the probe): the convergence key is the
  FACTS, NOT `content_sha256`.** The v1 sketch said "same sha-keyed subtree
  identity" — that is wrong for the cross-retina case. `content_sha256`
  hashes the ORIGINAL bytes, and the same invoice as a scan (pixels) and as
  HTML (markup) has different bytes → different hashes. So `content_sha256`
  is a **per-acquisition dedup key** (same file twice ⇒ one subtree), NOT a
  semantic identity across retinas. The retina-invariant identity is the
  harvested facts (netto/ust/IBAN read the same either way). Implemented +
  tested in `ogar-doc-ir`: `field_facts()` / `converges_on_facts()`, with a
  cross-retina test that converges on facts while the byte-hashes differ, and
  a converse test that fails on a disagreeing fact (the probe can actually
  falsify). Open question for W4: whether a `semantic_id` (a hash OF the
  facts) should be minted so the discovery arm can dedup across retinas —
  deferred to the doc-layer council with the persistence mints.
- **P-DOM-GROUNDTRUTH:** spider + headless render yields (pixels,
  DOM-labeled regions) pairs at zero annotation cost — a calibration
  corpus for tesseract's region classifier. The web trains the eye that
  reads paper.
- **P-2D-MARKOV:** does spatial-neighborhood attention (Morton rail
  adjacency) beat pure reading-order for field harvest? Slots into the
  existing temporal-stream probe queue; not a W-gate.

## P4. Non-goals (v1 of this plan)

- No live crawling in any CI path (fixtures only).
- No spider-side scheduling/politeness/auth policy — the fork's own
  concern; this plan touches only the page → doc-IR seam.
- No multi-language OCR slot, no streaming payloads, no storage-backend
  choice (all inherited deferrals from the doc-layer proposal).
- No DeepNSM redesign — W5's tuple signature is the entire ask.

## P5. Sequencing + gates

`W0 (council) → W1 (IR) → {W2, W3} in parallel → P-XRETINA → W4 → W5`.
W2/W3 are independent once W1 lands; **P-XRETINA runs the moment both
producers exist and BEFORE W4 persists anything** — persisting a
tesseract-shaped subtree and discovering the divergence later would bake
the bias into stored data. Merge-order discipline per house rule:
OGAR-side crates first, fork-side producers second, in lockstep.

## P6. The `ogar-doc-ir` type surface (the concrete contract W0 verifies)

Per the operator protocol (spec detailed enough that the council VERIFIES,
not redesigns), the W1 crate's surface in concrete form. Types only; no
minting, no canon change — the council ratifies THIS shape, then W1 codes it.
serde-only crate. **Canon-critical:** the spatial rail is `u8:u8` — **two
separate bytes, never widened to u16** (P0 GUID canon) — and IS the `X:Y`
reading of the 4+12 facet payload when a region persists as a node (W4).

```rust
/// Perceptual-IR version marker. `from_json` refuses any other value
/// (I-LEGACY-API-FEATURE-GATED: a v2 reshape is a NEW marker, never silent).
pub const DOC_IR_VERSION: &str = "doc.v1";

/// Which retina produced this document. Confidence semantics differ per
/// source, so they never share a float (OCR recognition ≠ crawl trust).
#[non_exhaustive]
pub enum Provenance { Ocr, Dom }

/// One spatial coordinate on the unit-square tile — ONE byte per axis.
/// A page/viewport is a 256×256 tile (P0 "256×256 centroid tile"); this
/// is the "2D focus of attention" address. NEVER a u16.
pub struct Rail { pub x: u8, pub y: u8 }

/// A region's box: two rails (top-left, bottom-right) = 4 bytes = two of
/// the six `u8:u8` pairs of the 12-byte facet register.
pub struct BBoxRail { pub tl: Rail, pub br: Rail }

/// How the rails were derived — kept in the provenance lane so a consumer
/// never mistakes document-order placement for measured geometry.
#[non_exhaustive]
pub enum Geometry {
    /// Measured layout (OCR pixel bbox / DOM `getBoundingClientRect`).
    Rendered,
    /// Reading-order placement (lol_html stream index / OCR line order).
    DomOrder,
}

/// The CLOSED region-kind vocabulary. `from_json` HARD-FAILS on any kind
/// outside this set (mirrors `ruff_spo_triplet::from_ndjson`'s closed
/// predicate gate) — the single rule that keeps the IR source-agnostic
/// instead of drifting into a tesseract-shaped or DOM-shaped grab-bag.
#[non_exhaustive]
pub enum RegionKind {
    Header,   // OCR page-furniture top / HTML <header>
    Footer,   // OCR page-furniture bottom / HTML <footer>
    Main,     // body text / HTML <main>|<article>
    Nav,      // HTML <nav> (OCR: rarely emitted)
    Table,    // OCR table region / HTML <table>
    Figure,   // OCR halftone region / HTML <figure>|<img>
    Text,     // a text block that is none of the above landmarks
}

/// One harvested typed field (invoice netto/ust/IBAN, a schema.org prop…).
/// Value stays a string here; typed validation (arithmetic_ok, iban_mod97)
/// is the executor/consumer's — the IR carries facts, not policy.
pub struct TypedField {
    pub key: String,
    pub value: String,
    pub bbox: BBoxRail,
    pub confidence: u8,          // per-source semantics (see Provenance)
}

/// One cell of a Table region, addressed by (row, col) — itself a `u8:u8`
/// pair on the facet register when persisted.
pub struct TableCell { pub row: u8, pub col: u8, pub text: String, pub bbox: BBoxRail }

/// A region node — the recursive awareness unit.
pub struct Region {
    pub kind: RegionKind,
    pub bbox: BBoxRail,
    pub reading_order: u16,      // stream position within the page
    pub text: Option<String>,    // for text-bearing kinds
    pub cells: Vec<TableCell>,   // for Table
    pub children: Vec<Region>,   // nested regions (a Main containing Tables…)
}

/// One page of a document.
pub struct DocPage { pub number: u16, pub width: u32, pub height: u32, pub regions: Vec<Region> }

/// The document root — the `document 0x080B` subtree before persistence.
pub struct DocIr {
    pub version: &'static str,   // == DOC_IR_VERSION
    pub source: Provenance,
    pub geometry: Geometry,
    /// Content identity: sha256 of the ORIGINAL bytes (scan or HTML) —
    /// the convergence key. Same document via both retinas ⇒ same id ⇒
    /// one subtree (P-XRETINA's identity assertion).
    pub content_sha256: [u8; 32],
    pub mime: String,
    pub pages: Vec<DocPage>,
    pub fields: Vec<TypedField>,
}

/// The load gate — closed-vocabulary + version enforcement at the boundary.
/// Fails loud on unknown region kind or wrong version (the source-agnostic
/// guard; a producer that emits an off-vocab kind is caught at the seam,
/// not silently dropped downstream).
pub fn from_json(s: &str) -> Result<DocIr, DocIrError>;
```

**What the council checks against this surface:** (1) every field a
*tesseract* producer needs and every field a *spider* producer needs are
both expressible with no source-specific escape hatch (source-agnostic
test); (2) `Rail`/`TableCell` stay `u8:u8`, never widened (canon);
(3) the closed `RegionKind` gate + `DOC_IR_VERSION` are the only load-time
authority; (4) `content_sha256` is the identity, not a producer-assigned id;
(5) `confidence` never conflates the two `Provenance` semantics. A numbered
factual error here is a council finding; taste is not.
