//! `ogar-from-docv1` — the **pixel retina** transcode (W2 of the OGAR doc-IR ×
//! spider convergence, `docs/DOC-IR-SPIDER-CONVERGENCE-PLAN.md`).
//!
//! Parses `tesseract-rs`'s `doc.v1` recognition JSON into the source-agnostic
//! [`ogar_doc_ir`] perceptual IR — so a scanned document and a web page
//! (`spider_doc_ir`) produce ONE shape.
//!
//! # Boundary (operator doctrine, tesseract-rs `CLAUDE.md`)
//!
//! *tesseract-rs = faithful recognition → rich `doc.v1`; store / graph / OGAR
//! are NOT its concerns; the JSON is the seed a consumer feeds via OGAR.* So
//! the `doc.v1 → ogar-doc-ir` transcode lives HERE, on the consumer/OGAR side —
//! exactly as `ogar-from-ruff` lifts ruff's output. tesseract-rs is untouched;
//! its `structured.rs` is the schema spec this crate targets.
//!
//! # Source-agnostic, confirmed at the vocabulary
//!
//! tesseract's `structured::RegionKind` (`text/table/figure/header/footer`)
//! already maps 1:1 onto the closed [`ogar_doc_ir::RegionKind`] — the DOM
//! retina's `header/main/footer/nav/table/figure` and this pixel retina's kinds
//! are the same vocabulary, which is the whole point of the convergence.
//!
//! # Geometry
//!
//! doc.v1 carries REAL pixel bboxes, so this producer emits
//! [`Geometry::Rendered`] — each `(x,y)` quantized to the `u8:u8` unit-square
//! rail against the page `width`/`height` (the DOM retina, lacking layout,
//! emits `DomOrder` instead; the provenance lane keeps them distinct).

use ogar_doc_ir::{
    BBoxRail, DOC_IR_VERSION, DocIr, DocPage, Geometry, Provenance, Rail, Region, RegionKind,
    TableCell, TypedField,
};
use serde::Deserialize;

/// The doc.v1 schema marker tesseract-rs emits (`structured::render_doc`).
const DOC_V1_SCHEMA: &str = "tesseract-rs/doc.v1";

/// Why a `doc.v1` document could not be transcoded.
#[derive(Debug)]
pub enum FromDocV1Error {
    /// The JSON did not parse as the doc.v1 shape.
    Json(serde_json::Error),
    /// The `schema` field was not [`DOC_V1_SCHEMA`] — this crate transcodes
    /// tesseract's doc.v1 specifically, not an arbitrary document JSON.
    WrongSchema { found: String },
    /// A region carried a `type` outside the closed vocabulary (a producer
    /// drift — caught here, not silently dropped downstream).
    UnknownRegionKind(String),
}

impl std::fmt::Display for FromDocV1Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(e) => write!(f, "doc.v1 JSON parse error: {e}"),
            Self::WrongSchema { found } => {
                write!(f, "not a {DOC_V1_SCHEMA} document (schema = `{found}`)")
            }
            Self::UnknownRegionKind(k) => {
                write!(
                    f,
                    "region type `{k}` is outside the closed doc-IR vocabulary"
                )
            }
        }
    }
}

impl std::error::Error for FromDocV1Error {}

// ── doc.v1 wire shape (mirrors tesseract-ocr/src/structured.rs::render_doc) ──

#[derive(Deserialize)]
struct V1Doc {
    schema: String,
    #[serde(default)]
    pages: Vec<V1Page>,
}

#[derive(Deserialize)]
struct V1Page {
    width: u32,
    height: u32,
    #[serde(default)]
    regions: Vec<V1Region>,
    #[serde(default)]
    fields: Vec<V1Field>,
}

#[derive(Deserialize)]
struct V1Region {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    lines: Vec<V1Line>,
    bbox: [i32; 4],
    // A `table` region's grid is emitted FLAT on the region object
    // (`structured::emit_table_json` appends `,"rows":…,"cols":…,"cells":[…]`
    // directly), NOT nested under a `"table"` key. Only `cells` is read here.
    #[serde(default)]
    cells: Vec<V1Cell>,
}

#[derive(Deserialize)]
struct V1Line {
    #[serde(default)]
    words: Vec<V1Word>,
}

#[derive(Deserialize)]
struct V1Word {
    text: String,
    #[serde(default)]
    leading_space: bool,
}

#[derive(Deserialize)]
struct V1Cell {
    row: u32,
    col: u32,
    bbox: [i32; 4],
    text: String,
    /// Per-cell OCR confidence, 0..=100. `#[serde(default)]` so a `doc.v1`
    /// emitted BEFORE cells carried confidence still parses — it reads `0.0`,
    /// which is fail-closed: a consumer's review gate rejects rather than
    /// auto-commits. Never read `0.0` as "measured zero"; it is
    /// indistinguishable from "not reported".
    #[serde(default)]
    conf: f32,
}

#[derive(Deserialize)]
struct V1Field {
    key: String,
    value: String,
    bbox: [i32; 4],
    conf: f32,
}

// ── quantization: pixel coordinate → u8 unit-square rail ──

/// Quantize one pixel coordinate onto `0..=255` against its axis `span`
/// (page width or height). `span == 0` (a degenerate page) collapses to 0.
fn q(v: i32, span: u32) -> u8 {
    if span == 0 {
        return 0;
    }
    let v = v.max(0) as u64;
    ((v * 255) / span as u64).min(255) as u8
}

/// A pixel bbox `[x0,y0,x1,y1]` → the `u8:u8` [`BBoxRail`] on the 256×256 tile.
fn rail_bbox(b: [i32; 4], w: u32, h: u32) -> BBoxRail {
    BBoxRail {
        tl: Rail {
            x: q(b[0], w),
            y: q(b[1], h),
        },
        br: Rail {
            x: q(b[2], w),
            y: q(b[3], h),
        },
    }
}

/// Map a doc.v1 region `type` string onto the closed [`RegionKind`]. doc.v1
/// has no `main`/`nav` (DOM-only landmarks); its `text` is the general block.
fn region_kind(kind: &str) -> Result<RegionKind, FromDocV1Error> {
    Ok(match kind {
        "text" => RegionKind::Text,
        "table" => RegionKind::Table,
        "figure" => RegionKind::Figure,
        "header" => RegionKind::Header,
        "footer" => RegionKind::Footer,
        other => return Err(FromDocV1Error::UnknownRegionKind(other.to_string())),
    })
}

/// Flatten a region's lines/words into its text, honoring `leading_space`
/// within a line and separating lines with a single space.
fn region_text(lines: &[V1Line]) -> Option<String> {
    let mut s = String::new();
    for line in lines {
        if !s.is_empty() && !s.ends_with(' ') {
            s.push(' ');
        }
        for w in &line.words {
            if w.leading_space && !s.is_empty() && !s.ends_with(' ') {
                s.push(' ');
            }
            s.push_str(&w.text);
        }
    }
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// Transcode a tesseract-rs `doc.v1` JSON string into an [`ogar_doc_ir::DocIr`].
///
/// `content_sha256` is the hash of the ORIGINAL image bytes (this crate sees
/// only the JSON, so the caller supplies it); per the #199 finding it is a
/// per-acquisition dedup key, NOT the cross-retina identity — the facts are.
/// `mime` is the source image's media type (`"image/png"`, `"application/pdf"`,
/// …), carried through to [`DocIr::mime`].
///
/// # Errors
///
/// [`FromDocV1Error::Json`] on malformed JSON, [`FromDocV1Error::WrongSchema`]
/// if the `schema` marker is not [`DOC_V1_SCHEMA`], and
/// [`FromDocV1Error::UnknownRegionKind`] on a region `type` outside the closed
/// vocabulary (fail-loud, mirroring `ogar_doc_ir::from_json`'s load gate).
pub fn from_doc_v1(
    json: &str,
    content_sha256: [u8; 32],
    mime: &str,
) -> Result<DocIr, FromDocV1Error> {
    let doc: V1Doc = serde_json::from_str(json).map_err(FromDocV1Error::Json)?;
    if doc.schema != DOC_V1_SCHEMA {
        return Err(FromDocV1Error::WrongSchema { found: doc.schema });
    }

    let mut pages = Vec::with_capacity(doc.pages.len());
    let mut fields = Vec::new();

    for (pi, page) in doc.pages.iter().enumerate() {
        let (w, h) = (page.width, page.height);
        let mut regions = Vec::with_capacity(page.regions.len());
        for (ri, r) in page.regions.iter().enumerate() {
            let cells: Vec<TableCell> = r
                .cells
                .iter()
                .map(|c| TableCell {
                    row: c.row.min(255) as u8,
                    col: c.col.min(255) as u8,
                    text: c.text.clone(),
                    bbox: rail_bbox(c.bbox, w, h),
                    // Same 0..=100 -> u8 conversion the TypedField arm below
                    // uses, so the two confidence surfaces stay comparable.
                    confidence: c.conf.round().clamp(0.0, 255.0) as u8,
                })
                .collect();
            regions.push(Region {
                kind: region_kind(&r.kind)?,
                bbox: rail_bbox(r.bbox, w, h),
                // doc.v1 regions are emitted in reading order (xy-cut order).
                reading_order: ri.min(u16::MAX as usize) as u16,
                text: region_text(&r.lines),
                cells,
                children: vec![],
            });
        }
        // doc.v1 fields are per-page; the IR carries them document-level.
        for f in &page.fields {
            fields.push(TypedField {
                key: f.key.clone(),
                value: f.value.clone(),
                bbox: rail_bbox(f.bbox, w, h),
                // OCR recognition confidence is a 0..100 percentage — kept on
                // that scale (the Provenance lane distinguishes it from a
                // DOM-declared field's trust).
                confidence: f.conf.round().clamp(0.0, 255.0) as u8,
            });
        }
        pages.push(DocPage {
            number: pi.min(u16::MAX as usize) as u16,
            width: w,
            height: h,
            regions,
        });
    }

    Ok(DocIr {
        version: DOC_IR_VERSION.to_string(),
        source: Provenance::Ocr,
        geometry: Geometry::Rendered,
        content_sha256,
        mime: mime.to_string(),
        pages,
        fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "schema":"tesseract-rs/doc.v1",
      "pages":[{
        "page":1,"width":1000,"height":2000,
        "quality":{"mean_conf":98.5,"low_confidence":false},
        "regions":[
          {"type":"header","bbox":[0,0,1000,100],"lines":[
            {"bbox":[0,0,500,100],"words":[
              {"text":"Acme","bbox":[0,0,200,100],"conf":99.0,"leading_space":false},
              {"text":"GmbH","bbox":[210,0,500,100],"conf":98.0,"leading_space":true}]}]},
          {"type":"table","bbox":[0,200,1000,600],"lines":[],
            "rows":1,"cols":2,"cells":[
              {"row":0,"col":0,"bbox":[0,200,500,300],"text":"Pos","header":true,"conf":97.0},
              {"row":0,"col":1,"bbox":[500,200,1000,300],"text":"1","header":true}]},
          {"type":"footer","bbox":[0,1900,1000,2000],"lines":[
            {"bbox":[0,1900,300,2000],"words":[
              {"text":"Seite","bbox":[0,1900,150,2000],"conf":97.0,"leading_space":false},
              {"text":"1","bbox":[160,1900,300,2000],"conf":97.0,"leading_space":true}]}]}
        ],
        "fields":[
          {"key":"iban","label":"IBAN:","value":"DE00 0000 0000 0000 0000 00",
           "bbox":[0,700,400,720],"conf":95.0,"checks":["iban_mod97_ok"]}]
      }]
    }"#;

    #[test]
    fn transcodes_doc_v1_to_source_agnostic_ir() {
        let ir = from_doc_v1(SAMPLE, [0xAB; 32], "image/png").expect("valid doc.v1");
        assert_eq!(ir.source, Provenance::Ocr);
        assert_eq!(ir.geometry, Geometry::Rendered);
        assert_eq!(ir.mime, "image/png");

        let page = &ir.pages[0];
        assert_eq!(page.width, 1000);
        assert_eq!(
            page.regions.iter().map(|r| r.kind).collect::<Vec<_>>(),
            vec![RegionKind::Header, RegionKind::Table, RegionKind::Footer]
        );
        // Header text flattened; pixel bbox quantized to the unit tile.
        assert_eq!(page.regions[0].text.as_deref(), Some("Acme GmbH"));
        assert_eq!(page.regions[0].bbox.tl, Rail { x: 0, y: 0 });
        assert_eq!(page.regions[0].bbox.br, Rail { x: 255, y: 12 }); // 100/2000*255≈12
        // Table cells carried across with quantized bboxes.
        assert_eq!(page.regions[1].cells.len(), 2);
        assert_eq!(page.regions[1].cells[0].text, "Pos");

        // Per-cell confidence survives the bridge — BOTH paths, which is why
        // the fixture gives cell 0 a `"conf"` and deliberately withholds one
        // from cell 1.
        //
        // Before this field existed, per-cell OCR confidence died here: the
        // values a consumer actually imports (a lab result, an invoice line
        // amount) are CELLS, not `TypedField`s, so a review gate downstream
        // had nothing to gate on. `V1Cell` is a plain `Deserialize` with no
        // `deny_unknown_fields`, so a producer emitting `conf` was silently
        // discarded — the omission was invisible from either side.
        assert_eq!(
            page.regions[1].cells[0].confidence, 97,
            "cell 0's doc.v1 conf:97.0 must reach TableCell::confidence"
        );
        // Cell 1 carries NO `conf` in the fixture: a legacy `doc.v1` predating
        // per-cell confidence. `#[serde(default)]` gives 0 — fail-closed, so
        // such a payload routes to human review instead of auto-committing at
        // an assumed score. This asserts the legacy path is REACHABLE, not
        // merely that the new one works.
        assert_eq!(
            page.regions[1].cells[1].confidence, 0,
            "a cell with no conf in doc.v1 must default to 0 (fail-closed)"
        );
        // Field harvested to the document-level facts.
        assert_eq!(ir.fields.len(), 1);
        assert_eq!(ir.fields[0].key, "iban");
        assert_eq!(ir.fields[0].confidence, 95);
    }

    #[test]
    fn output_passes_the_shared_load_gate() {
        // The convergence proof: the pixel retina's output survives the SAME
        // `ogar_doc_ir::from_json` closed-vocab + version gate a DOM producer's
        // does — one shape, two retinas.
        let ir = from_doc_v1(SAMPLE, [1; 32], "image/png").unwrap();
        let json = ogar_doc_ir::to_json(&ir).expect("serialize");
        let back = ogar_doc_ir::from_json(&json).expect("load gate accepts it");
        assert_eq!(ir, back);
    }

    #[test]
    fn wrong_schema_is_refused() {
        let s = r#"{"schema":"some-other/v1","pages":[]}"#;
        assert!(matches!(
            from_doc_v1(s, [0; 32], "image/png"),
            Err(FromDocV1Error::WrongSchema { .. })
        ));
    }

    #[test]
    fn unknown_region_kind_fails_loud() {
        let s = r#"{"schema":"tesseract-rs/doc.v1","pages":[{"page":1,"width":10,"height":10,
            "regions":[{"type":"banner","bbox":[0,0,1,1],"lines":[]}],"fields":[]}]}"#;
        assert!(matches!(
            from_doc_v1(s, [0; 32], "image/png"),
            Err(FromDocV1Error::UnknownRegionKind(k)) if k == "banner"
        ));
    }
}
