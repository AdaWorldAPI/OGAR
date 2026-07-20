//! `ogar-doc-ir` — the **source-agnostic perceptual IR** for OGAR's document
//! layer (W1 of `docs/DOC-IR-SPIDER-CONVERGENCE-PLAN.md`).
//!
//! # The one idea
//!
//! A document's *awareness* — its regions, tables, and typed fields laid out
//! in space and reading order — is not an OCR output format. It is a
//! **perceptual IR** that more than one retina produces:
//!
//! - **tesseract-rs** (pixels) → `doc.v1` from a scan;
//! - **spider-rs** (DOM) → `doc.v1` from a web page (HTML5 `<header>/<main>/
//!   <footer>/<table>` self-label the regions OCR must infer).
//!
//! This mirrors the code side (N language frontends → one closed ndjson → one
//! `ModelGraph`): N retinas → one closed doc IR → one `document 0x080B`
//! awareness subtree. Everything upstream (`ogar-doc` persistence, DeepNSM,
//! the discovery arm, MedCare-rs as an abstract document store) consumes ONE
//! shape regardless of retina.
//!
//! # The load-bearing invariants (what keeps it source-agnostic)
//!
//! 1. **Closed region-kind vocabulary + version marker.** [`from_json`]
//!    hard-fails on any kind outside [`RegionKind`] or any version other than
//!    [`DOC_IR_VERSION`] — the exact discipline of
//!    `ruff_spo_triplet::from_ndjson`'s closed predicate gate. A producer that
//!    emits an off-vocab kind is caught at the seam, never silently dropped
//!    downstream. Adding a kind is a `doc.v2` bump, not a silent reshape.
//! 2. **`u8:u8` spatial rails, never widened.** [`Rail`] is two separate
//!    bytes (a `const` size assert enforces it) — the `X:Y` reading of the
//!    OGAR 4+12 facet register (P0 GUID canon: "`u8:u8` stays two separate
//!    bytes, never widened to u16"). A page/viewport is a 256×256 tile; the
//!    rail IS the "2D focus of attention" address.
//! 3. **Per-source confidence.** OCR recognition confidence and crawl trust
//!    are different quantities and never share a float — the [`Provenance`]
//!    lane keeps them distinct (I-LEGACY-API-FEATURE-GATED, prospectively).
//! 4. **Content identity.** [`DocIr::content_sha256`] is the sha256 of the
//!    ORIGINAL bytes — the convergence key. The same document via both
//!    retinas resolves to ONE subtree (the P-XRETINA identity assertion).
//!
//! # This crate mints nothing
//!
//! serde-only. The `document 0x080B` / `typed_field 0x080A` classid mints and
//! the persistence ActionDefs live in `ogar-vocab` / `ogar-doc` (W4), gated on
//! the doc-layer 5+3 council. This crate is pure perceptual structure with no
//! canon dependency, so it can land ahead of that ratification.
//!
//! # Observation IR vs. the composition layer (DocIr composition grounding)
//!
//! [`DocIr`] here is the **observation IR** — the perceptual retina (what a
//! scan / DOM faithfully *saw*). The 2026-07-20 ruling
//! (`docs/DOCIR-COMPOSITION-LAYER.md`, #217; grounded in
//! `docs/DOCIR-COMPOSITION-GROUNDING.md`, #218) promotes *DocIr* to also name a
//! **composition layer** over the OGAR object graph — a `DocNode` graph of
//! `Text` / `Section` / `ObjectSlot` (typed projection portals into OGAR
//! nodes). Per operator ruling **A1** (*"one `ogar-doc` crate; split axis is
//! STATE not direction"*), that composition `DocNode` is a **module in this
//! crate family, never a sibling `ogar-doc-compose`**. It **references this
//! observation IR through an `ObjectSlot`** (an imported scan appears in a
//! composed document as a portal, not as pasted regions) — this observation IR
//! is **untouched, referenced, never retyped in place**. An `ObjectSlot` is the
//! `D-DOC-IR-SECOND-RETINA` **A3 brick** (`ClassView × WideFieldMask`, "same
//! brick as Klickwege — no new DSL") plus an `ObjectRef` + a `ResolutionMode`.
//!
//! # Refinements this crate makes to the plan's P6 sketch (building validated the spec)
//!
//! - **Enums are EXHAUSTIVE, not `#[non_exhaustive]`.** A closed vocabulary
//!   that a `ClassView` renderer must fully handle wants a *compile error*
//!   when a new kind is unhandled — `#[non_exhaustive]` would defeat that by
//!   forcing a wildcard arm. Evolution goes through [`DOC_IR_VERSION`]
//!   (`doc.v2`), which is the intended, louder major-version mechanism. The
//!   closed-vocab guarantee thus extends to the render side.
//! - **`DocIr::version` is an owned `String`** (deserialization can't target
//!   `&'static str`); [`from_json`] checks it equals [`DOC_IR_VERSION`].

use serde::{Deserialize, Serialize};

pub mod compose;
#[cfg(feature = "classview")]
pub mod project;

/// Perceptual-IR version marker. [`from_json`] refuses any other value.
/// A `doc.v2` is a NEW marker + (possibly) new [`RegionKind`]s, never a
/// silent reshape of `doc.v1`.
pub const DOC_IR_VERSION: &str = "doc.v1";

/// Which retina produced this document. Confidence semantics differ per
/// source (OCR recognition uncertainty vs. crawl/boilerplate trust), so a
/// consumer must read a `confidence` byte through the lens of its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// A pixel retina (tesseract-rs): confidence = recognition certainty.
    Ocr,
    /// A DOM retina (spider-rs): confidence = source/boilerplate trust.
    Dom,
}

/// How a region's rails were derived — kept explicit so a consumer never
/// mistakes document-order placement for measured layout geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Geometry {
    /// Measured layout: OCR pixel bbox, or DOM `getBoundingClientRect`.
    Rendered,
    /// Reading-order placement: `lol_html` stream index / OCR line order,
    /// quantized top-to-bottom onto the unit square.
    DomOrder,
}

/// The **closed** region-kind vocabulary. [`from_json`] hard-fails on any
/// value outside this set — the single rule that keeps the IR source-agnostic
/// instead of drifting into a tesseract-shaped or DOM-shaped grab-bag.
/// Exhaustive by design (see the crate doc): a `ClassView` renderer gets a
/// compile error until it handles every kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionKind {
    /// Page-furniture top (OCR) / HTML `<header>`.
    Header,
    /// Page-furniture bottom (OCR) / HTML `<footer>`.
    Footer,
    /// Body content (OCR) / HTML `<main>` or `<article>`.
    Main,
    /// HTML `<nav>` (OCR: rarely emitted).
    Nav,
    /// A table region (OCR) / HTML `<table>` — carries [`TableCell`]s.
    Table,
    /// A halftone/figure region (OCR) / HTML `<figure>` or `<img>`.
    Figure,
    /// A text block that is none of the above landmarks.
    Text,
}

/// One spatial coordinate on the unit-square tile — **one byte per axis**.
/// A page/viewport is a 256×256 tile (P0 "256×256 centroid tile"); this is
/// the "2D focus of attention" address. NEVER a `u16` (canon).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rail {
    /// Horizontal byte-index into the 256-wide tile.
    pub x: u8,
    /// Vertical byte-index into the 256-tall tile.
    pub y: u8,
}

// Canon guard: the rail is exactly two bytes, never widened (P0 GUID canon).
const _: () = assert!(core::mem::size_of::<Rail>() == 2);

/// A region's box: two rails (top-left, bottom-right) = 4 bytes = two of the
/// six `u8:u8` pairs of the 12-byte facet register when persisted (W4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BBoxRail {
    /// Top-left corner.
    pub tl: Rail,
    /// Bottom-right corner.
    pub br: Rail,
}

// Canon guard: a bbox is exactly four bytes (two rails), never widened.
const _: () = assert!(core::mem::size_of::<BBoxRail>() == 4);

/// One harvested typed field (an invoice `netto`/`ust`/`IBAN`, a schema.org
/// property, …). The value stays a string here; typed validation
/// (`arithmetic_ok`, `iban_mod97_ok`) is the executor/consumer's job — the IR
/// carries **facts, not policy**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedField {
    /// Field name (e.g. `"netto"`, `"iban"`).
    pub key: String,
    /// Raw string value; interpretation is the consumer's.
    pub value: String,
    /// Where on the page the field was read.
    pub bbox: BBoxRail,
    /// Per-source confidence (read through [`DocIr::source`]).
    pub confidence: u8,
}

/// One cell of a [`RegionKind::Table`] region, addressed by `(row, col)` —
/// itself a `u8:u8` pair on the facet register when persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCell {
    /// Zero-based row index.
    pub row: u8,
    /// Zero-based column index.
    pub col: u8,
    /// Cell text.
    pub text: String,
    /// Where on the page the cell was read.
    pub bbox: BBoxRail,
}

/// A region node — the recursive awareness unit. Text-bearing kinds carry
/// [`Self::text`]; [`RegionKind::Table`] carries [`Self::cells`]; container
/// regions (a `Main` holding `Table`s) carry [`Self::children`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region {
    /// The closed-vocabulary kind.
    pub kind: RegionKind,
    /// The region's box on the unit-square tile.
    pub bbox: BBoxRail,
    /// Stream position within the page — the reading-order the temporal
    /// stream (and DeepNSM) consumes.
    pub reading_order: u16,
    /// Text for text-bearing kinds; `None` for pure containers / figures.
    #[serde(default)]
    pub text: Option<String>,
    /// Cells for [`RegionKind::Table`]; empty otherwise.
    #[serde(default)]
    pub cells: Vec<TableCell>,
    /// Nested regions in reading order.
    #[serde(default)]
    pub children: Vec<Region>,
}

/// One page of a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocPage {
    /// Zero-based page number.
    pub number: u16,
    /// Page width in the producer's native units (pixels / CSS px); the rails
    /// are already normalized, this is provenance for un-quantizing.
    pub width: u32,
    /// Page height in native units.
    pub height: u32,
    /// Top-level regions in reading order.
    pub regions: Vec<Region>,
}

/// The document root — the `document 0x080B` subtree *before* persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocIr {
    /// Must equal [`DOC_IR_VERSION`]; [`from_json`] enforces it.
    pub version: String,
    /// Which retina produced this.
    pub source: Provenance,
    /// How the rails were derived.
    pub geometry: Geometry,
    /// sha256 of the ORIGINAL bytes (scan or HTML) — the convergence key.
    /// Same document via both retinas ⇒ same id ⇒ one subtree.
    pub content_sha256: [u8; 32],
    /// The original bytes' MIME type.
    pub mime: String,
    /// Pages in order.
    pub pages: Vec<DocPage>,
    /// Document-level typed fields (an invoice's fields need not sit on one
    /// page).
    #[serde(default)]
    pub fields: Vec<TypedField>,
}

/// Why [`from_json`] refused a document.
#[derive(Debug)]
pub enum DocIrError {
    /// Malformed JSON, or an off-vocabulary [`RegionKind`] / [`Provenance`] /
    /// [`Geometry`] (serde rejects unknown enum values — the closed-vocab
    /// gate).
    Json(serde_json::Error),
    /// The `version` field was not [`DOC_IR_VERSION`].
    WrongVersion {
        /// The version the document carried.
        found: String,
        /// The version this crate accepts.
        expected: &'static str,
    },
}

impl core::fmt::Display for DocIrError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Json(e) => write!(f, "doc-ir JSON rejected (closed vocabulary): {e}"),
            Self::WrongVersion { found, expected } => write!(
                f,
                "doc-ir version `{found}` unsupported (this crate reads `{expected}`; \
                 a new version is a deliberate migration, not a silent reshape)"
            ),
        }
    }
}

impl std::error::Error for DocIrError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(e) => Some(e),
            Self::WrongVersion { .. } => None,
        }
    }
}

/// The **load gate** — closed-vocabulary + version enforcement at the
/// boundary. Fails loud on unknown region kind / provenance / geometry (serde
/// rejects the unknown enum value) or a wrong `version`. A producer that
/// drifts off the vocabulary is caught here, not silently downstream.
///
/// # Errors
///
/// [`DocIrError::Json`] on malformed JSON or an off-vocab enum value;
/// [`DocIrError::WrongVersion`] when `version != `[`DOC_IR_VERSION`].
pub fn from_json(s: &str) -> Result<DocIr, DocIrError> {
    let ir: DocIr = serde_json::from_str(s).map_err(DocIrError::Json)?;
    if ir.version != DOC_IR_VERSION {
        return Err(DocIrError::WrongVersion {
            found: ir.version,
            expected: DOC_IR_VERSION,
        });
    }
    Ok(ir)
}

/// Serialize a [`DocIr`] to JSON (the symmetric egress of [`from_json`]).
///
/// # Errors
///
/// Propagates any `serde_json` serialization error (in practice only an
/// allocator failure for this flat shape).
pub fn to_json(ir: &DocIr) -> Result<String, serde_json::Error> {
    serde_json::to_string(ir)
}

// ─────────────────────────────────────────────────────────────────────────
// Cross-retina convergence (the P-XRETINA probe surface)
// ─────────────────────────────────────────────────────────────────────────

/// The **retina-invariant identity** of a document: the set of its typed-field
/// facts `(key, value)`.
///
/// # Why NOT `content_sha256`
///
/// The plan's first sketch named [`DocIr::content_sha256`] the cross-retina
/// convergence key. Building the probe surfaced that this is **wrong for the
/// cross-retina case**: `content_sha256` hashes the ORIGINAL bytes, and the
/// same invoice as a *scan* (pixels) and as *HTML* (markup) has different
/// bytes → different hashes. So `content_sha256` is a **per-acquisition dedup
/// key** (the same file uploaded twice ⇒ one subtree), NOT a semantic
/// identity across retinas.
///
/// The retina-invariant thing is the **facts**: netto / ust / IBAN read the
/// same whether a pixel retina or a DOM retina produced them. This function is
/// that identity — the basis of [`converges_on_facts`], the P-XRETINA
/// assertion.
#[must_use]
pub fn field_facts(ir: &DocIr) -> std::collections::BTreeSet<(&str, &str)> {
    ir.fields
        .iter()
        .map(|f| (f.key.as_str(), f.value.as_str()))
        .collect()
}

/// The **P-XRETINA convergence assertion**: two documents produced by
/// different retinas from the same source converge iff their typed-field facts
/// match — regardless of [`Provenance`], [`Geometry`], region ordering, or
/// [`DocIr::content_sha256`] (which differs by acquisition, see [`field_facts`]).
///
/// Deliberately compares FACTS, not the whole tree: two retinas legitimately
/// differ on geometry (rendered vs DomOrder rails) and region granularity; the
/// convergence claim is about the *harvested knowledge*, which must agree.
#[must_use]
pub fn converges_on_facts(a: &DocIr, b: &DocIr) -> bool {
    field_facts(a) == field_facts(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rail(x: u8, y: u8) -> Rail {
        Rail { x, y }
    }

    fn bbox(x0: u8, y0: u8, x1: u8, y1: u8) -> BBoxRail {
        BBoxRail {
            tl: rail(x0, y0),
            br: rail(x1, y1),
        }
    }

    /// A minimal but non-trivial document: a header, a main with one table,
    /// a footer, and one document-level typed field.
    fn sample() -> DocIr {
        DocIr {
            version: DOC_IR_VERSION.to_string(),
            source: Provenance::Dom,
            geometry: Geometry::DomOrder,
            content_sha256: [7u8; 32],
            mime: "text/html".to_string(),
            pages: vec![DocPage {
                number: 0,
                width: 1000,
                height: 1400,
                regions: vec![
                    Region {
                        kind: RegionKind::Header,
                        bbox: bbox(0, 0, 255, 20),
                        reading_order: 0,
                        text: Some("Acme GmbH".to_string()),
                        cells: vec![],
                        children: vec![],
                    },
                    Region {
                        kind: RegionKind::Main,
                        bbox: bbox(0, 20, 255, 230),
                        reading_order: 1,
                        text: None,
                        cells: vec![],
                        children: vec![Region {
                            kind: RegionKind::Table,
                            bbox: bbox(10, 40, 245, 120),
                            reading_order: 2,
                            text: None,
                            cells: vec![TableCell {
                                row: 0,
                                col: 0,
                                text: "Pos".to_string(),
                                bbox: bbox(10, 40, 60, 55),
                            }],
                            children: vec![],
                        }],
                    },
                    Region {
                        kind: RegionKind::Footer,
                        bbox: bbox(0, 240, 255, 255),
                        reading_order: 3,
                        text: Some("Seite 1".to_string()),
                        cells: vec![],
                        children: vec![],
                    },
                ],
            }],
            fields: vec![TypedField {
                key: "iban".to_string(),
                value: "DE00 0000 0000 0000 0000 00".to_string(),
                bbox: bbox(10, 200, 200, 215),
                confidence: 200,
            }],
        }
    }

    #[test]
    fn round_trips_through_json() {
        let ir = sample();
        let s = to_json(&ir).expect("serialize");
        let back = from_json(&s).expect("load-gate accepts a valid doc.v1");
        assert_eq!(ir, back, "doc IR must survive a JSON round-trip unchanged");
    }

    #[test]
    fn load_gate_rejects_unknown_region_kind() {
        // `banner` is not in the closed RegionKind vocabulary.
        let s = r#"{
            "version":"doc.v1","source":"dom","geometry":"dom_order",
            "content_sha256":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
            "mime":"text/html",
            "pages":[{"number":0,"width":1,"height":1,"regions":[
                {"kind":"banner","bbox":{"tl":{"x":0,"y":0},"br":{"x":1,"y":1}},"reading_order":0}
            ]}]
        }"#;
        let err = from_json(s).expect_err("off-vocab kind must be refused at the gate");
        assert!(
            matches!(err, DocIrError::Json(_)),
            "unknown kind → closed-vocab Json rejection"
        );
    }

    #[test]
    fn load_gate_rejects_wrong_version() {
        let mut ir = sample();
        ir.version = "doc.v2".to_string();
        let s = to_json(&ir).unwrap();
        let err = from_json(&s).expect_err("a future version must not silently load as v1");
        match err {
            DocIrError::WrongVersion { found, expected } => {
                assert_eq!(found, "doc.v2");
                assert_eq!(expected, DOC_IR_VERSION);
            }
            other => panic!("expected WrongVersion, got {other:?}"),
        }
    }

    #[test]
    fn load_gate_rejects_unknown_provenance() {
        // `audio` is not a sanctioned retina in doc.v1.
        let mut s = to_json(&sample()).unwrap();
        s = s.replace("\"source\":\"dom\"", "\"source\":\"audio\"");
        let err = from_json(&s).expect_err("off-vocab provenance must be refused");
        assert!(matches!(err, DocIrError::Json(_)));
    }

    #[test]
    fn reading_order_is_preserved_depth_first() {
        // The sample's reading_order values are 0..=3 in a pre-order walk;
        // confirm the tree carries them intact (the temporal-stream contract).
        let ir = from_json(&to_json(&sample()).unwrap()).unwrap();
        let page = &ir.pages[0];
        assert_eq!(page.regions[0].reading_order, 0); // header
        assert_eq!(page.regions[1].reading_order, 1); // main
        assert_eq!(page.regions[1].children[0].reading_order, 2); // table under main
        assert_eq!(page.regions[2].reading_order, 3); // footer
    }

    #[test]
    fn rail_is_two_bytes_and_never_widened() {
        // Belt-and-braces alongside the module-level const asserts: the rail
        // is the u8:u8 facet pair, never a u16.
        assert_eq!(core::mem::size_of::<Rail>(), 2);
        assert_eq!(core::mem::size_of::<BBoxRail>(), 4);
    }

    /// Build one invoice as two retinas would see it — deliberately differing
    /// on Provenance, Geometry, region shape, AND raw-bytes hash, but agreeing
    /// on the harvested facts.
    fn two_retinas() -> (DocIr, DocIr) {
        let facts = || {
            vec![
                TypedField {
                    key: "netto".to_string(),
                    value: "100.00".to_string(),
                    bbox: bbox(0, 0, 10, 10),
                    confidence: 180, // OCR recognition confidence
                },
                TypedField {
                    key: "iban".to_string(),
                    value: "DE00 0000 0000 0000 0000 00".to_string(),
                    bbox: bbox(0, 10, 10, 20),
                    confidence: 180,
                },
            ]
        };
        // Pixel retina: rendered geometry, one big Main region, scan bytes.
        let ocr = DocIr {
            version: DOC_IR_VERSION.to_string(),
            source: Provenance::Ocr,
            geometry: Geometry::Rendered,
            content_sha256: [0xAA; 32], // sha256 of the SCAN
            mime: "image/png".to_string(),
            pages: vec![DocPage {
                number: 0,
                width: 2480,
                height: 3508,
                regions: vec![Region {
                    kind: RegionKind::Main,
                    bbox: bbox(0, 0, 255, 255),
                    reading_order: 0,
                    text: Some("Rechnung".to_string()),
                    cells: vec![],
                    children: vec![],
                }],
            }],
            fields: {
                let mut f = facts();
                // DOM-declared trust differs from OCR — the fields' confidence
                // bytes need not match; only key/value (the facts) do.
                for tf in &mut f {
                    tf.confidence = 255;
                }
                f
            },
        };
        // DOM retina: DomOrder geometry, header/main/footer, HTML bytes.
        let dom = DocIr {
            version: DOC_IR_VERSION.to_string(),
            source: Provenance::Dom,
            geometry: Geometry::DomOrder,
            content_sha256: [0xBB; 32], // sha256 of the HTML — DIFFERENT bytes
            mime: "text/html".to_string(),
            pages: vec![DocPage {
                number: 0,
                width: 0,
                height: 0,
                regions: vec![
                    Region {
                        kind: RegionKind::Header,
                        bbox: bbox(0, 0, 255, 8),
                        reading_order: 0,
                        text: Some("Acme".to_string()),
                        cells: vec![],
                        children: vec![],
                    },
                    Region {
                        kind: RegionKind::Main,
                        bbox: bbox(0, 8, 255, 250),
                        reading_order: 1,
                        text: Some("Rechnung".to_string()),
                        cells: vec![],
                        children: vec![],
                    },
                ],
            }],
            fields: facts(),
        };
        (ocr, dom)
    }

    /// P-XRETINA: two retinas of the same invoice CONVERGE on facts even
    /// though their raw-bytes hashes, geometry, provenance, and region shape
    /// all differ. This is the plan's killer probe, made executable.
    #[test]
    fn cross_retina_converges_on_facts_not_bytes() {
        let (ocr, dom) = two_retinas();
        assert!(
            converges_on_facts(&ocr, &dom),
            "same invoice via two retinas must agree on the harvested facts"
        );
        // THE FINDING: content_sha256 is NOT the cross-retina key — the raw
        // bytes (scan vs HTML) differ, so the byte-hash cannot be the
        // convergence identity. It is a per-acquisition dedup key only.
        assert_ne!(
            ocr.content_sha256, dom.content_sha256,
            "raw-bytes hash differs across retinas — it is per-acquisition dedup, \
             not the semantic convergence key"
        );
        // And the facts identity is stable regardless of the confidence lane.
        assert_eq!(field_facts(&ocr), field_facts(&dom));
    }

    /// The converse: a genuine fact disagreement must NOT converge (the probe
    /// can actually fail — it is a falsifier, not a tautology).
    #[test]
    fn cross_retina_diverges_when_a_fact_disagrees() {
        let (ocr, mut dom) = two_retinas();
        dom.fields[0].value = "999.99".to_string(); // netto misread
        assert!(
            !converges_on_facts(&ocr, &dom),
            "a disagreeing fact must break convergence — P-XRETINA can fail"
        );
    }
}
