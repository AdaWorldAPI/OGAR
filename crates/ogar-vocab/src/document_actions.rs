//! Document persistence capability surface — the **paperless-rs
//! authoritative action table** for the `document` (`0x080B`) subtree.
//!
//! Declares the three capabilities `OGAR-DOC-W4-BUILD-SPEC.md` §W4-4
//! specifies (`persist_document` / `read_document` / `reconstruct_document`),
//! each targeting the minted `document` concept
//! ([`crate::class_ids::DOCUMENT`]) as its `object_class`. It registers in
//! [`crate::capability_registry::domain_tables`] through the generic
//! [`crate::capability_registry::entries_from_actions`] derive, exactly as
//! [`crate::healthcare_actions`] and [`crate::ocr_actions`] do.
//!
//! # Why a SEPARATE table, not a growth of `ocr_actions.rs`
//!
//! `OGAR-DOC-W4-BUILD-SPEC.md` §W4-4's literal text says these three rows
//! belong in "`ogar-vocab/src/ocr_actions.rs`, same table as the shipped 14
//! caps." A council review of that placement (5+3 council, 2026-08-25)
//! found it would silently couple two unrelated consumers:
//! [`crate::capability_registry::resolve_hotplug`] gates per **contributing
//! table**, not globally
//! (`capability_registry.rs::resolve_hotplug`) — so if these three rows
//! physically lived in `ocr_actions.rs` and `paperless-kv` (a non-OCR
//! consumer) plugged `document`, `ocr_actions::OCR_EXPECTED_EXECUTORS`
//! would have to grow to include `paperless-kv`, permanently entangling
//! `tesseract-ogar`'s and `paperless-kv`'s hot-plug resolutions through one
//! shared list. `healthcare_actions.rs` is this crate's own precedent for
//! exactly this situation — "a *private* consumer... registers here in
//! every build" via a table isolated to its own executor list
//! ([`crate::capability_registry::domain_tables`]'s own doc comment) — so
//! this module follows that shape instead. `tesseract-ogar`'s existing
//! `HOT_PLUG` is unaffected: this table never contributes to a plug that
//! doesn't include `document`.
//!
//! # Facts only — the executor body is NOT here
//!
//! Per §W4-4's own rule: *"Facts only: HOW nodes are written (mailbox /
//! Lance) is the executor seam's business."* These `ActionDef`s carry only
//! static identity/signature/kausal fields — no storage state, no write
//! logic. The consumer (`paperless-kv`) declares a [`crate::hotplug`]-style
//! [`lance_graph_contract` (via the consumer, not this crate)] activation
//! against them; the actual `persist_document` implementation (walking a
//! `DocIr` into GUID-keyed SoA nodes, `DedupIndex`-backed) is explicitly
//! future work, not part of this table.
//!
//! # `typed_field` is minted, not used as a subject here
//!
//! [`crate::class_ids::TYPED_FIELD`] (`0x080A`) mints alongside `document`
//! in this same council landing, but none of these three ActionDefs use it
//! as a subject — `typed_field` is the INTERNAL per-field decomposition a
//! future `persist_document` body would write, not part of this table's
//! public capability surface.

use crate::{ActionDef, ActionSubject, KausalSpec};

/// Every document capability name, in table order — the `const`-evaluable
/// fingerprint of [`document_actions`] (same role as
/// [`crate::ocr_actions::OCR_ACTION_NAMES`] /
/// [`crate::healthcare_actions::HEALTHCARE_ACTION_NAMES`]).
pub const DOCUMENT_ACTION_NAMES: &[&str] =
    &["persist_document", "read_document", "reconstruct_document"];

const _: () = assert!(DOCUMENT_ACTION_NAMES.len() == 3);

/// Build one [`ActionDef`] for a document capability. `reads`/`writes`
/// carry the mandatory + optional field names from
/// `OGAR-DOC-W4-BUILD-SPEC.md` §W4-4's param table (name-level effect
/// facts, matching [`crate::ocr_actions::ocr_action_def`]'s convention —
/// richer than [`crate::healthcare_actions`]'s deliberately-empty rows,
/// since §W4-4 explicitly enumerates the signature).
fn document_action_def(capability: &'static str, reads: &[&str], writes: &[&str]) -> ActionDef {
    let object_class = "ogit-ocr/document".to_string();
    let identity = format!("{object_class}::action_def::{capability}");
    ActionDef {
        identity,
        predicate: capability.to_owned(),
        object_class,
        default_subject: ActionSubject::System,
        // Every capability here is invoked directly by a caller (the
        // paperless-kv ingestion pipeline, an RPC edge) — no OGAR-side
        // precondition to guard on, matching `KausalSpec::External`'s doc
        // and the identical convention `ocr_actions::ocr_action_def` uses.
        kausal: Some(KausalSpec::External),
        reads: reads.iter().map(|s| (*s).to_owned()).collect(),
        writes: writes.iter().map(|s| (*s).to_owned()).collect(),
        ..ActionDef::default()
    }
}

/// The paperless-rs document-persistence capability surface — the
/// **authoritative OGAR action table** for the `document` subtree. One
/// [`ActionDef`] per capability, in [`DOCUMENT_ACTION_NAMES`] order,
/// subject = `document` (`0x080B`) throughout.
///
/// | capability | mandatory | optional | produces |
/// |---|---|---|---|
/// | `persist_document` | `doc_ir, raw_sha256, raw_kv_key` | — | `document_guid` |
/// | `read_document` | `document_guid` | — | `doc_ir` |
/// | `reconstruct_document` | `document_guid` | `template` | `pdf_bytes` |
#[must_use]
pub fn document_actions() -> Vec<ActionDef> {
    vec![
        document_action_def(
            "persist_document",
            &["doc_ir", "raw_sha256", "raw_kv_key"],
            &["document_guid"],
        ),
        document_action_def("read_document", &["document_guid"], &["doc_ir"]),
        document_action_def(
            "reconstruct_document",
            &["document_guid", "template"],
            &["pdf_bytes"],
        ),
    ]
}

/// The executors the authority EXPECTS to register against this table.
/// `paperless-kv` is the Rust consumer (`OGAR-DOC-W4-BUILD-SPEC.md` §W4-5
/// §A1: the executor lives in the assembly repo, not a new `ogar-doc`
/// crate — council-ratified 2026-08-25).
pub const DOCUMENT_EXPECTED_EXECUTORS: &[&str] = &["paperless-kv"];

/// The distinct subject classids this table binds (canon-high concept
/// ids). A registering consumer must activate exactly this set.
pub const DOCUMENT_SUBJECT_CLASSIDS: &[u16] = &[crate::class_ids::DOCUMENT];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{canonical_concept_domain, class_ids};
    use std::collections::BTreeSet;

    #[test]
    fn table_length_matches_const_name_fingerprint() {
        let actions = document_actions();
        assert_eq!(actions.len(), DOCUMENT_ACTION_NAMES.len());
        for (def, name) in actions.iter().zip(DOCUMENT_ACTION_NAMES) {
            assert_eq!(&def.predicate, name);
        }
    }

    #[test]
    fn capability_names_are_unique() {
        let actions = document_actions();
        let names: BTreeSet<&str> = actions.iter().map(|s| s.predicate.as_str()).collect();
        assert_eq!(names.len(), actions.len());
    }

    #[test]
    fn every_action_declares_external_kausal() {
        for def in document_actions() {
            assert_eq!(def.kausal, Some(KausalSpec::External));
        }
    }

    /// Every document action's subject resolves to the minted `document`
    /// (`0x080B`) concept, in the OCR domain (per its `class_ids::ALL`
    /// placement) — the fuse that catches a renamed or unminted concept
    /// before a consumer trusts this table.
    #[test]
    fn subjects_resolve_to_the_minted_document_concept() {
        for def in document_actions() {
            assert_eq!(def.object_class, "ogit-ocr/document");
            let entry = class_ids::ALL
                .iter()
                .find(|(name, _)| *name == "document")
                .expect("document minted");
            assert_eq!(entry.1, class_ids::DOCUMENT);
            assert_eq!(canonical_concept_domain(entry.1), crate::ConceptDomain::Ocr);
        }
    }

    /// `reconstruct_document`'s mandatory reads must be a SUBSET of
    /// `read_document`'s + the produced `document_guid` chain — cannot
    /// need less than resolving a document requires (mirrors
    /// `ocr_actions`'s `recognize_document_reads_cover_the_word_stage`
    /// composition pin).
    #[test]
    fn reconstruct_document_requires_a_document_guid_like_read_document() {
        let actions = document_actions();
        let get = |name: &str| actions.iter().find(|d| d.predicate == name).unwrap();
        assert!(
            get("read_document")
                .reads
                .contains(&"document_guid".to_string())
        );
        assert!(
            get("reconstruct_document")
                .reads
                .contains(&"document_guid".to_string())
        );
    }

    /// This table's own registration roundtrip, mirroring
    /// `ocr_actions::verify_ocr_registration` / the healthcare table's
    /// equivalent — a convenience wrapper a consumer's own test can call.
    #[test]
    fn document_hot_plug_resolves_via_the_generic_registry() {
        let (concepts, caps) = crate::capability_registry::resolve_hotplug(
            "paperless-kv",
            DOCUMENT_SUBJECT_CLASSIDS,
            DOCUMENT_ACTION_NAMES,
        )
        .expect("document hot-plug drifted from the authoritative table");
        assert_eq!(concepts, vec![("document", class_ids::DOCUMENT)]);
        assert_eq!(caps.len(), 3);

        // Falsifiability: a different consumer name is rejected, not
        // silently accepted — the can-it-FAIL half.
        assert!(matches!(
            crate::capability_registry::resolve_hotplug(
                "tesseract-ogar",
                DOCUMENT_SUBJECT_CLASSIDS,
                DOCUMENT_ACTION_NAMES,
            ),
            Err(crate::capability_registry::HotplugDrift::UnexpectedConsumer(_))
        ));
    }
}
