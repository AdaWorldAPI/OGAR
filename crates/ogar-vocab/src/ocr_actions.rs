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
];

const _: () = assert!(OCR_ACTION_NAMES.len() == 8);

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
    ]
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
}
