//! OCR capability surface — the **tesseract-rs authoritative action
//! table**.
//!
//! This module declares the eight capabilities the tesseract-rs pure-Rust
//! OCR transcode exposes ([`docs/ARAGO-ACTIONHANDLER-PARITY.md`] parity:
//! `recognize_line` / `recognize_page` / `extract_text_layer` /
//! `extract_page_image` / `render_text` / `render_tsv` / `render_hocr` /
//! `render_searchable_pdf`) as real [`ActionDef`] declarations, each
//! targeting a minted `0x08XX` OCR concept ([`class_ids`]) as its
//! `object_class`.
//!
//! # Why a hand-authored table, not a `lift_*` extraction
//!
//! Every other `Vec<ActionDef>` producer in this workspace
//! (`ogar-from-rails::extract_actions`, `ogar-from-ruff::lift_actions`,
//! `ogar-from-schema::do_arm::lift_action_defs`, …) *extracts* actions from
//! a source AST or schema. tesseract-rs has neither — it is a from-scratch
//! Rust transcode of Tesseract, not a lift target — so its action surface
//! is declared directly, the same way a schema-level `ActionDef` would be
//! authored by hand for a capability with no upstream AST. This table IS
//! the authority a consumer (the tesseract-rs `ogar-action-handler`
//! executor, or a future `graph-flow-action` binding) iterates against;
//! there is no separate "real" source to drift from.
//!
//! # Effect facts vs the typed I/O signature
//!
//! Per `ogar-from-schema::do_arm`'s split (`ActionDef::reads`/`writes` are
//! *effect annotations*; the concrete parameter *signature* — with a
//! mandatory/optional flag per slot — is a **deployed capability** fact
//! arago models as `Parameter { Name, Mandatory, Default }`
//! (`ogar_from_schema::do_arm::ActionParam`)) this module mirrors that
//! same split:
//!
//! - [`ActionDef::reads`] carries every input name (name-level effect
//!   fact, mandatory + optional collapsed — matching how `reads` is
//!   populated everywhere else in this crate).
//! - [`ActionDef::writes`] carries every output name the capability
//!   produces.
//! - [`OcrActionSpec::params`] carries the **typed signature**
//!   ([`OcrActionParam`], with the mandatory/optional split arago's
//!   `Parameter.Mandatory` needs) and [`OcrActionSpec::produces`] carries
//!   the output names as a plain slice for the same consumer.
//!
//! [`OcrActionParam`] is a dependency-free mirror of
//! `ogar_from_schema::do_arm::ActionParam`'s `{ name, mandatory }` shape.
//! It cannot literally BE that type: `ogar-from-schema` already depends on
//! `ogar-vocab` (see its `Cargo.toml`), so the reverse dependency would be
//! cyclic. A consumer that needs the arago-parity type performs the
//! trivial field map:
//!
//! ```
//! use ogar_vocab::ocr_actions::ocr_actions;
//!
//! for spec in ocr_actions() {
//!     let _params: Vec<(&str, bool)> =
//!         spec.params.iter().map(|p| (p.name, p.mandatory)).collect();
//! }
//! ```
//!
//! # Why a `fn`, not a `const`
//!
//! `ActionDef`'s `identity` / `predicate` / `object_class` /
//! `reads` / `writes` fields are `String` / `Vec<String>` — not
//! const-constructible in stable Rust. [`ocr_actions`] is therefore a
//! plain constructor function, matching every other `Vec<ActionDef>`
//! producer in this workspace. [`OCR_ACTION_NAMES`] is the `const`-
//! evaluable fingerprint (capability names only, in table order) a
//! consumer can use for a cheap compile-time-checked exhaustiveness fuse
//! without paying for the full table's allocations.

use crate::{ActionDef, ActionSubject, KausalSpec};

/// One parameter of an OCR capability's typed I/O signature — a
/// dependency-free mirror of `ogar_from_schema::do_arm::ActionParam`'s
/// `{ name, mandatory }` shape. See the module doc for why this is a
/// local type rather than a re-export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OcrActionParam {
    /// Parameter name (arago `Parameter.Name`).
    pub name: &'static str,
    /// Whether the parameter is required (arago `Parameter.Mandatory`).
    pub mandatory: bool,
}

impl OcrActionParam {
    /// Build a required parameter slot.
    #[must_use]
    pub const fn required(name: &'static str) -> Self {
        Self {
            name,
            mandatory: true,
        }
    }

    /// Build an optional parameter slot.
    #[must_use]
    pub const fn optional(name: &'static str) -> Self {
        Self {
            name,
            mandatory: false,
        }
    }
}

/// One OCR capability declaration — the [`ActionDef`] shape (identity /
/// predicate / object_class / kausal) plus the concrete typed I/O
/// signature `ActionDef` itself has no field for (see the module doc's
/// "effect facts vs the typed I/O signature" section).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcrActionSpec {
    /// The schema-level action declaration.
    pub def: ActionDef,
    /// The typed input signature (mandatory + optional slots).
    pub params: &'static [OcrActionParam],
    /// The output names this capability produces.
    pub produces: &'static [&'static str],
}

/// Every OCR capability name, in table order — a `const`-evaluable
/// fingerprint of [`ocr_actions`] for a cheap exhaustiveness fuse (see the
/// module doc's "why a `fn`, not a `const`" section). A consumer that
/// wants to assert "I handle every OCR capability" can match this slice
/// exhaustively without constructing the full table.
pub const OCR_ACTION_NAMES: &[&str] = &[
    "recognize_line",
    "recognize_page",
    "extract_text_layer",
    "extract_page_image",
    "render_text",
    "render_tsv",
    "render_hocr",
    "render_searchable_pdf",
    // v2 (2026-07-10) — the structured-document + layout-classifier surface
    // the tesseract-rs arc shipped after the original eight. See
    // docs/OCR-ACTIONS-V2-PROPOSAL.md.
    "recognize_page_words",
    "recognize_document",
    "harvest_fields",
    "segment_page",
    "detect_halftone_regions",
    "detect_page_furniture",
];

const _: () = assert!(OCR_ACTION_NAMES.len() == 14);

const RECOGNIZE_LINE_PARAMS: &[OcrActionParam] = &[
    OcrActionParam::required("grey_line"),
    OcrActionParam::required("width"),
    OcrActionParam::required("height"),
    OcrActionParam::optional("with_dict"),
];
const RECOGNIZE_LINE_PRODUCES: &[&str] = &["text", "unichar_ids"];

const RECOGNIZE_PAGE_PARAMS: &[OcrActionParam] = &[
    OcrActionParam::required("grey_page"),
    OcrActionParam::required("width"),
    OcrActionParam::required("height"),
    OcrActionParam::optional("with_dict"),
];
const RECOGNIZE_PAGE_PRODUCES: &[&str] = &["textlines", "text"];

const EXTRACT_TEXT_LAYER_PARAMS: &[OcrActionParam] = &[OcrActionParam::required("pdf_bytes")];
/// Per-page text-or-none — a page with no extractable text layer yields
/// `None` at that index rather than shortening the list.
const EXTRACT_TEXT_LAYER_PRODUCES: &[&str] = &["page_texts"];

const EXTRACT_PAGE_IMAGE_PARAMS: &[OcrActionParam] = &[
    OcrActionParam::required("pdf_bytes"),
    OcrActionParam::required("page"),
];
const EXTRACT_PAGE_IMAGE_PRODUCES: &[&str] = &["grey_image"];

const RENDER_TEXT_PARAMS: &[OcrActionParam] = &[OcrActionParam::required("lines")];
const RENDER_TEXT_PRODUCES: &[&str] = &["text"];

const RENDER_TSV_PARAMS: &[OcrActionParam] = &[
    OcrActionParam::required("lines"),
    OcrActionParam::required("page_w"),
    OcrActionParam::required("page_h"),
];
const RENDER_TSV_PRODUCES: &[&str] = &["tsv"];

const RENDER_HOCR_PARAMS: &[OcrActionParam] = &[
    OcrActionParam::required("lines"),
    OcrActionParam::required("page_w"),
    OcrActionParam::required("page_h"),
    OcrActionParam::required("image_name"),
];
const RENDER_HOCR_PRODUCES: &[&str] = &["hocr"];

const RENDER_SEARCHABLE_PDF_PARAMS: &[OcrActionParam] = &[
    OcrActionParam::required("pages"),
    OcrActionParam::required("dpi"),
];
const RENDER_SEARCHABLE_PDF_PRODUCES: &[&str] = &["pdf_bytes"];

// ── v2 capability signatures (2026-07-10) ──────────────────────────────────

/// `recognize_page_words` — a full grey page recognized to WORD/box output
/// (`LineWords`: per-line words each carrying char boxes + confidences),
/// the word-level counterpart of `recognize_page`'s flat-text `textlines`.
const RECOGNIZE_PAGE_WORDS_PARAMS: &[OcrActionParam] = &[
    OcrActionParam::required("grey_page"),
    OcrActionParam::required("width"),
    OcrActionParam::required("height"),
    OcrActionParam::optional("with_dict"),
];
const RECOGNIZE_PAGE_WORDS_PRODUCES: &[&str] = &["line_words"];

/// `recognize_document` — the ONE-SHOT: grey page in → `doc.v1` structured
/// JSON (regions/lines/words with typed region classification) + a typed
/// field harvest out. `harvest_profile` selects the field set (v2 vocabulary:
/// `"german_invoice"`; absent = no harvest, empty `fields`; an unknown value
/// is an executor-side FAIL, not a silent no-harvest).
const RECOGNIZE_DOCUMENT_PARAMS: &[OcrActionParam] = &[
    OcrActionParam::required("grey_page"),
    OcrActionParam::required("width"),
    OcrActionParam::required("height"),
    OcrActionParam::optional("with_dict"),
    OcrActionParam::optional("harvest_profile"),
];
const RECOGNIZE_DOCUMENT_PRODUCES: &[&str] = &["doc_json", "fields"];

/// `harvest_fields` — the typed field harvest over an already-recognized
/// page's word output (numeric hardening + label-proximity + IBAN mod-97 +
/// the netto+ust==brutto arithmetic cross-check). `harvest_profile` as above.
const HARVEST_FIELDS_PARAMS: &[OcrActionParam] = &[
    OcrActionParam::required("line_words"),
    OcrActionParam::required("page_w"),
    OcrActionParam::required("page_h"),
    OcrActionParam::required("harvest_profile"),
];
const HARVEST_FIELDS_PRODUCES: &[&str] = &["fields"];

/// `segment_page` — recursive XY-cut layout segmentation (columns /
/// deimposition) → reading-ordered `(l,t,r,b)` region rects.
const SEGMENT_PAGE_PARAMS: &[OcrActionParam] = &[
    OcrActionParam::required("grey_page"),
    OcrActionParam::required("width"),
    OcrActionParam::required("height"),
    OcrActionParam::optional("min_gap_frac"),
    OcrActionParam::optional("min_region_px"),
    OcrActionParam::optional("max_depth"),
];
const SEGMENT_PAGE_PRODUCES: &[&str] = &["regions_rects"];

/// `detect_halftone_regions` — leptonica-parity `pixGenerateHalftoneMask`
/// image-region detector over a BINARIZED page → figure component rects (+
/// the mask dims and the found flag; the mask may be smaller than the page).
const DETECT_HALFTONE_REGIONS_PARAMS: &[OcrActionParam] = &[
    OcrActionParam::required("binary_page"),
    OcrActionParam::required("width"),
    OcrActionParam::required("height"),
];
const DETECT_HALFTONE_REGIONS_PRODUCES: &[&str] = &["figure_rects", "mask_w", "mask_h", "found"];

/// `detect_page_furniture` — header / footer / page-number detection over an
/// already-recognized page's word output.
const DETECT_PAGE_FURNITURE_PARAMS: &[OcrActionParam] = &[
    OcrActionParam::required("line_words"),
    OcrActionParam::required("page_w"),
    OcrActionParam::required("page_h"),
];
const DETECT_PAGE_FURNITURE_PRODUCES: &[&str] = &["header_lines", "footer_lines", "page_number"];

/// Build one [`ActionDef`] for an OCR capability. `subject_concept` MUST
/// be a name minted in [`class_ids::ALL`] under the `0x08XX` (OCR) domain
/// — enforced by this module's tests, not by this constructor (the tables
/// above are the source of truth; the test is the fuse that catches drift
/// if a concept is ever renamed or unminted).
fn ocr_action_def(
    capability: &'static str,
    subject_concept: &'static str,
    params: &'static [OcrActionParam],
    produces: &'static [&'static str],
) -> ActionDef {
    let object_class = format!("ogit-ocr/{subject_concept}");
    let identity = format!("{object_class}::action_def::{capability}");
    ActionDef {
        identity,
        predicate: capability.to_owned(),
        object_class,
        default_subject: ActionSubject::System,
        // Every OCR capability here is invoked directly by a caller (a
        // REST/RPC edge, or a same-process executor call) — there is no
        // OGAR-side precondition to guard on, matching
        // `KausalSpec::External`'s doc: "External cause (RPC call, HTTP
        // request) — no precondition to check inside the system."
        kausal: Some(KausalSpec::External),
        reads: params.iter().map(|p| p.name.to_owned()).collect(),
        writes: produces.iter().map(|s| (*s).to_owned()).collect(),
        ..ActionDef::default()
    }
}

/// The tesseract-rs OCR capability surface — the **authoritative OGAR
/// action table** for optical character recognition / document
/// extraction. One [`OcrActionSpec`] per capability, in [`OCR_ACTION_NAMES`]
/// order.
///
/// | capability | subject concept | mandatory params | optional params | produces |
/// |---|---|---|---|---|
/// | `recognize_line` | `textline` (`0x0805`) | `grey_line, width, height` | `with_dict` | `text, unichar_ids` |
/// | `recognize_page` | `page_image` (`0x0808`) | `grey_page, width, height` | `with_dict` | `textlines, text` |
/// | `extract_text_layer` | `page_image` (`0x0808`) | `pdf_bytes` | — | `page_texts` |
/// | `extract_page_image` | `page_image` (`0x0808`) | `pdf_bytes, page` | — | `grey_image` |
/// | `render_text` | `ocr_renderer` (`0x0809`) | `lines` | — | `text` |
/// | `render_tsv` | `ocr_renderer` (`0x0809`) | `lines, page_w, page_h` | — | `tsv` |
/// | `render_hocr` | `ocr_renderer` (`0x0809`) | `lines, page_w, page_h, image_name` | — | `hocr` |
/// | `render_searchable_pdf` | `ocr_renderer` (`0x0809`) | `pages, dpi` | — | `pdf_bytes` |
/// | `recognize_page_words` | `page_image` (`0x0808`) | `grey_page, width, height` | `with_dict` | `line_words` |
/// | `recognize_document` | `page_image` (`0x0808`) | `grey_page, width, height` | `with_dict, harvest_profile` | `doc_json, fields` |
/// | `harvest_fields` | `page_layout` (`0x0807`) | `line_words, page_w, page_h, harvest_profile` | — | `fields` |
/// | `segment_page` | `page_image` (`0x0808`) | `grey_page, width, height` | `min_gap_frac, min_region_px, max_depth` | `regions_rects` |
/// | `detect_halftone_regions` | `page_image` (`0x0808`) | `binary_page, width, height` | — | `figure_rects, mask_w, mask_h, found` |
/// | `detect_page_furniture` | `page_layout` (`0x0807`) | `line_words, page_w, page_h` | — | `header_lines, footer_lines, page_number` |
#[must_use]
pub fn ocr_actions() -> Vec<OcrActionSpec> {
    vec![
        OcrActionSpec {
            def: ocr_action_def(
                "recognize_line",
                "textline",
                RECOGNIZE_LINE_PARAMS,
                RECOGNIZE_LINE_PRODUCES,
            ),
            params: RECOGNIZE_LINE_PARAMS,
            produces: RECOGNIZE_LINE_PRODUCES,
        },
        OcrActionSpec {
            def: ocr_action_def(
                "recognize_page",
                "page_image",
                RECOGNIZE_PAGE_PARAMS,
                RECOGNIZE_PAGE_PRODUCES,
            ),
            params: RECOGNIZE_PAGE_PARAMS,
            produces: RECOGNIZE_PAGE_PRODUCES,
        },
        OcrActionSpec {
            def: ocr_action_def(
                "extract_text_layer",
                "page_image",
                EXTRACT_TEXT_LAYER_PARAMS,
                EXTRACT_TEXT_LAYER_PRODUCES,
            ),
            params: EXTRACT_TEXT_LAYER_PARAMS,
            produces: EXTRACT_TEXT_LAYER_PRODUCES,
        },
        OcrActionSpec {
            def: ocr_action_def(
                "extract_page_image",
                "page_image",
                EXTRACT_PAGE_IMAGE_PARAMS,
                EXTRACT_PAGE_IMAGE_PRODUCES,
            ),
            params: EXTRACT_PAGE_IMAGE_PARAMS,
            produces: EXTRACT_PAGE_IMAGE_PRODUCES,
        },
        OcrActionSpec {
            def: ocr_action_def(
                "render_text",
                "ocr_renderer",
                RENDER_TEXT_PARAMS,
                RENDER_TEXT_PRODUCES,
            ),
            params: RENDER_TEXT_PARAMS,
            produces: RENDER_TEXT_PRODUCES,
        },
        OcrActionSpec {
            def: ocr_action_def(
                "render_tsv",
                "ocr_renderer",
                RENDER_TSV_PARAMS,
                RENDER_TSV_PRODUCES,
            ),
            params: RENDER_TSV_PARAMS,
            produces: RENDER_TSV_PRODUCES,
        },
        OcrActionSpec {
            def: ocr_action_def(
                "render_hocr",
                "ocr_renderer",
                RENDER_HOCR_PARAMS,
                RENDER_HOCR_PRODUCES,
            ),
            params: RENDER_HOCR_PARAMS,
            produces: RENDER_HOCR_PRODUCES,
        },
        OcrActionSpec {
            def: ocr_action_def(
                "render_searchable_pdf",
                "ocr_renderer",
                RENDER_SEARCHABLE_PDF_PARAMS,
                RENDER_SEARCHABLE_PDF_PRODUCES,
            ),
            params: RENDER_SEARCHABLE_PDF_PARAMS,
            produces: RENDER_SEARCHABLE_PDF_PRODUCES,
        },
        // ── v2 rows (2026-07-10) ──
        OcrActionSpec {
            def: ocr_action_def(
                "recognize_page_words",
                "page_image",
                RECOGNIZE_PAGE_WORDS_PARAMS,
                RECOGNIZE_PAGE_WORDS_PRODUCES,
            ),
            params: RECOGNIZE_PAGE_WORDS_PARAMS,
            produces: RECOGNIZE_PAGE_WORDS_PRODUCES,
        },
        OcrActionSpec {
            def: ocr_action_def(
                "recognize_document",
                "page_image",
                RECOGNIZE_DOCUMENT_PARAMS,
                RECOGNIZE_DOCUMENT_PRODUCES,
            ),
            params: RECOGNIZE_DOCUMENT_PARAMS,
            produces: RECOGNIZE_DOCUMENT_PRODUCES,
        },
        OcrActionSpec {
            def: ocr_action_def(
                "harvest_fields",
                "page_layout",
                HARVEST_FIELDS_PARAMS,
                HARVEST_FIELDS_PRODUCES,
            ),
            params: HARVEST_FIELDS_PARAMS,
            produces: HARVEST_FIELDS_PRODUCES,
        },
        OcrActionSpec {
            def: ocr_action_def(
                "segment_page",
                "page_image",
                SEGMENT_PAGE_PARAMS,
                SEGMENT_PAGE_PRODUCES,
            ),
            params: SEGMENT_PAGE_PARAMS,
            produces: SEGMENT_PAGE_PRODUCES,
        },
        OcrActionSpec {
            def: ocr_action_def(
                "detect_halftone_regions",
                "page_image",
                DETECT_HALFTONE_REGIONS_PARAMS,
                DETECT_HALFTONE_REGIONS_PRODUCES,
            ),
            params: DETECT_HALFTONE_REGIONS_PARAMS,
            produces: DETECT_HALFTONE_REGIONS_PRODUCES,
        },
        OcrActionSpec {
            def: ocr_action_def(
                "detect_page_furniture",
                "page_layout",
                DETECT_PAGE_FURNITURE_PARAMS,
                DETECT_PAGE_FURNITURE_PRODUCES,
            ),
            params: DETECT_PAGE_FURNITURE_PARAMS,
            produces: DETECT_PAGE_FURNITURE_PRODUCES,
        },
    ]
}

/// The executors the authority EXPECTS to register against this table —
/// "die Ontologie wurde nicht vergessen" has a name attached: an empty or
/// stale list here is itself the drift signal a downstream fuse catches.
pub const OCR_EXPECTED_EXECUTORS: &[&str] = &["tesseract-ogar"];

/// The distinct subject classids this table binds (canon-high concept ids).
/// A registering consumer must activate exactly this set — verified via
/// [`crate::capability_registry::resolve_hotplug`] (the live hot-plug fuse;
/// [`crate::capability_registry::verify_registration`] is the equivalent
/// standalone check). `PAGE_LAYOUT` was added with the v2 rows
/// (`harvest_fields` / `detect_page_furniture`, 2026-07-10).
pub const OCR_SUBJECT_CLASSIDS: &[u16] = &[
    crate::class_ids::TEXTLINE,
    crate::class_ids::PAGE_IMAGE,
    crate::class_ids::PAGE_LAYOUT,
    crate::class_ids::OCR_RENDERER,
];

/// Convenience roundtrip for THIS table: verify a consumer registration
/// against the OCR capability names, subjects and expected executors.
pub fn verify_ocr_registration(
    reg: &crate::capability_registry::CapabilityRegistration,
) -> Result<(), crate::capability_registry::RegistrationDrift> {
    crate::capability_registry::verify_registration(
        reg,
        OCR_ACTION_NAMES,
        OCR_SUBJECT_CLASSIDS,
        OCR_EXPECTED_EXECUTORS,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{canonical_concept_domain, class_ids};
    use std::collections::BTreeSet;

    fn subject_concept_of(def: &ActionDef) -> &str {
        // object_class is always "ogit-ocr/<subject_concept>" — see
        // `ocr_action_def`.
        def.object_class
            .strip_prefix("ogit-ocr/")
            .expect("every OCR ActionDef.object_class starts with `ogit-ocr/`")
    }

    #[test]
    fn table_length_matches_const_name_fingerprint() {
        let actions = ocr_actions();
        assert_eq!(actions.len(), OCR_ACTION_NAMES.len());
        for (spec, name) in actions.iter().zip(OCR_ACTION_NAMES) {
            assert_eq!(&spec.def.predicate, name);
        }
    }

    #[test]
    fn capability_names_are_unique() {
        let actions = ocr_actions();
        let names: BTreeSet<&str> = actions.iter().map(|s| s.def.predicate.as_str()).collect();
        assert_eq!(
            names.len(),
            actions.len(),
            "duplicate capability name in OCR_ACTION_NAMES / ocr_actions()"
        );
    }

    #[test]
    fn param_names_are_non_empty() {
        for spec in ocr_actions() {
            for p in spec.params {
                assert!(
                    !p.name.is_empty(),
                    "{}: empty param name",
                    spec.def.predicate
                );
            }
            for out in spec.produces {
                assert!(
                    !out.is_empty(),
                    "{}: empty produces name",
                    spec.def.predicate
                );
            }
        }
    }

    /// Every OCR action's subject resolves to a minted `0x08XX` concept in
    /// [`class_ids::ALL`] — the fuse that catches a renamed or unminted
    /// concept before a consumer trusts this table.
    #[test]
    fn subjects_resolve_to_minted_0x08_concepts() {
        for spec in ocr_actions() {
            let concept = subject_concept_of(&spec.def);
            let entry = class_ids::ALL
                .iter()
                .find(|(name, _)| *name == concept)
                .unwrap_or_else(|| {
                    panic!(
                        "{}: subject concept `{concept}` is not in class_ids::ALL",
                        spec.def.predicate
                    )
                });
            assert_eq!(
                canonical_concept_domain(entry.1),
                crate::ConceptDomain::Ocr,
                "{}: subject concept `{concept}` (0x{:04X}) is not in the OCR domain",
                spec.def.predicate,
                entry.1
            );
        }
    }

    #[test]
    fn every_action_declares_external_kausal() {
        for spec in ocr_actions() {
            assert_eq!(spec.def.kausal, Some(KausalSpec::External));
        }
    }

    #[test]
    fn identity_and_object_class_are_well_formed() {
        for spec in ocr_actions() {
            assert!(spec.def.identity.starts_with("ogit-ocr/"));
            assert!(spec.def.object_class.starts_with("ogit-ocr/"));
            assert!(
                spec.def
                    .identity
                    .ends_with(&format!("::action_def::{}", spec.def.predicate))
            );
        }
    }

    /// v2: `recognize_document` is the one-shot composition of the word-level
    /// recognition — so its mandatory inputs must be a SUPERSET of
    /// `recognize_page_words`'s mandatory inputs (the one-shot cannot need
    /// less than the first stage it composes).
    #[test]
    fn recognize_document_reads_cover_the_word_stage() {
        let actions = ocr_actions();
        let get = |name: &str| {
            actions
                .iter()
                .find(|s| s.def.predicate == name)
                .unwrap_or_else(|| panic!("missing capability {name}"))
        };
        let words: BTreeSet<&str> = get("recognize_page_words")
            .params
            .iter()
            .filter(|p| p.mandatory)
            .map(|p| p.name)
            .collect();
        let doc: BTreeSet<&str> = get("recognize_document")
            .params
            .iter()
            .filter(|p| p.mandatory)
            .map(|p| p.name)
            .collect();
        assert!(
            words.is_subset(&doc),
            "recognize_document mandatory reads {doc:?} must cover recognize_page_words' {words:?}"
        );
    }

    /// v2: the `harvest_profile` vocabulary has exactly one documented value
    /// in v2 (`"german_invoice"`); pin it so a rename is a visible breaking
    /// change. `harvest_fields` requires the profile; `recognize_document`
    /// makes it optional (absent = no harvest).
    #[test]
    fn harvest_profile_slot_is_present_where_documented() {
        let actions = ocr_actions();
        let harvest = actions
            .iter()
            .find(|s| s.def.predicate == "harvest_fields")
            .expect("harvest_fields present");
        assert!(
            harvest
                .params
                .iter()
                .any(|p| p.name == "harvest_profile" && p.mandatory),
            "harvest_fields must require harvest_profile"
        );
        let doc = actions
            .iter()
            .find(|s| s.def.predicate == "recognize_document")
            .expect("recognize_document present");
        assert!(
            doc.params
                .iter()
                .any(|p| p.name == "harvest_profile" && !p.mandatory),
            "recognize_document must offer harvest_profile as optional"
        );
    }

    /// v2: the two `page_layout`-subject rows drove the single net-new entry
    /// in [`OCR_SUBJECT_CLASSIDS`]; assert the set is exactly the four minted
    /// concepts the 14 rows bind, no more, no less.
    #[test]
    fn subject_classids_match_the_actual_row_subjects() {
        let mut from_rows: BTreeSet<u16> = BTreeSet::new();
        for spec in ocr_actions() {
            let concept = subject_concept_of(&spec.def);
            let id = class_ids::ALL
                .iter()
                .find(|(name, _)| *name == concept)
                .expect("subject minted")
                .1;
            from_rows.insert(id);
        }
        let declared: BTreeSet<u16> = OCR_SUBJECT_CLASSIDS.iter().copied().collect();
        assert_eq!(
            from_rows, declared,
            "OCR_SUBJECT_CLASSIDS must equal the exact set of subjects the rows bind"
        );
    }
}
