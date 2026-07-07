//! Healthcare capability surface — the **medcare-rs authoritative action
//! table** (`MEDCARE-CSHARP-PARITY-INTEGRATION-PLAN.md` P3).
//!
//! This module declares the healthcare capabilities the medcare-rs
//! consumer executes, one [`ActionDef`] per capability, each targeting a
//! minted `0x09XX` Health concept ([`crate::class_ids`]) as its
//! `object_class`. It registers in
//! [`crate::capability_registry::domain_tables`] through the generic
//! [`crate::capability_registry::entries_from_actions`] derive — the
//! "config becomes data" seam — so `resolve_hotplug` turns green for the
//! Health classids that previously banged `NoCapabilitiesFor` (the
//! deficiency the parity plan §0 names).
//!
//! # Why hand-authored (harvest-INFORMED, not harvest-DERIVED)
//!
//! The plan's Loop A hoped to auto-derive this table mechanically
//! (Roslyn harvest → `reassemble` → `lift_actions` →
//! `entries_from_actions`). The real corpus falsified that premise
//! (2026-07-07 harvest over the consumer C# tree): the application is a
//! WinForms monolith — **no AR-shaped class per domain concept exists**.
//! The domain concepts (`patient`, `diagnosis`, `lab_value`, …) never
//! appear as classes; the domain surface lives as `verb_concept` methods
//! on one monolithic DAL class plus WinForms UserControls. A class-keyed
//! lift has nothing to key on, so — exactly like [`crate::ocr_actions`]'s
//! "no ActiveRecord source" rationale — the table is declared by hand.
//!
//! The harvest still did its job as **evidence**: each concept's presence
//! and its per-concept capability split (CRUD-shaped: add / get / list /
//! delete arms) follow the measured method inventory in the consumer's DAL
//! (roughly a dozen methods per concept for the busiest, single digits for
//! the rest). The concrete method-name evidence stays in the consumer repo
//! (the harvest artifact is never committed here); a re-harvest that
//! surfaces a new method family is the trigger to extend this table —
//! harvest-informed, with the authorship step explicit instead of
//! pretending to be mechanical.
//!
//! # `visit` (0x0906) is deliberately absent
//!
//! The harvest found **zero** methods touching a visit/encounter concept
//! anywhere in the DAL or UI layer (operator-confirmed exclusion,
//! 2026-07-07). Declaring a capability with no implementation evidence
//! would be fabrication; the concept stays minted but table-less until a
//! consumer actually grows a visit surface. Consequence:
//! [`HEALTHCARE_SUBJECT_CLASSIDS`] has 6 entries, not the port's 7 — and
//! `resolve_hotplug` on `VISIT` still bangs `NoCapabilitiesFor`, which is
//! the honest signal.
//!
//! # Name-only ActionDefs (empty effect facts) — deliberate
//!
//! `reads`/`writes` are left empty: the DO-arm harvest facts on the DAL
//! methods are *internal* effects (SQL-buffer scratch fields), not
//! domain-field effects — copying them here would be noise, and inventing
//! domain-level effect facts would be fabrication. Per
//! [`crate::capability_registry::entries_from_actions`]'s contract
//! (and its `entries_from_actions_ignores_effect_facts` test), the
//! hot-plug join keys on capability name + subject classid alone, so
//! name-only defs are fully valid rows. Effect-fact enrichment is the
//! plan's P6, orthogonal to this table.

use crate::{ActionDef, ActionSubject, KausalSpec};

/// Every healthcare capability name, in table order — the `const`-evaluable
/// fingerprint of [`healthcare_actions`] (same role as
/// [`crate::ocr_actions::OCR_ACTION_NAMES`]). A consumer that wants to
/// assert "I handle every healthcare capability" matches this slice
/// exhaustively without constructing the full table.
pub const HEALTHCARE_ACTION_NAMES: &[&str] = &[
    // patient (0x0901)
    "register_patient",
    "get_patient_record",
    "list_patients",
    "update_patient_access",
    // diagnosis (0x0902)
    "add_diagnosis",
    "get_diagnosis",
    "list_diagnoses",
    "delete_diagnosis",
    // lab_value (0x0903)
    "add_lab_result",
    "get_lab_result",
    "list_lab_results",
    // medication (0x0904)
    "add_medication",
    "get_medication",
    "list_medications",
    // treatment (0x0905)
    "add_treatment",
    "get_treatment",
    "list_treatments",
    // vital_sign (0x0907)
    "add_vital_sign",
    "list_vital_signs",
    "delete_vital_signs",
];

const _: () = assert!(HEALTHCARE_ACTION_NAMES.len() == 20);

/// Build one [`ActionDef`] for a healthcare capability. `subject_concept`
/// MUST be a name minted in [`crate::class_ids::ALL`] under the `0x09XX`
/// (Health) domain — enforced by this module's tests, same fuse shape as
/// `ocr_actions::ocr_action_def`.
fn healthcare_action_def(capability: &'static str, subject_concept: &'static str) -> ActionDef {
    let object_class = format!("ogit-health/{subject_concept}");
    let identity = format!("{object_class}::action_def::{capability}");
    ActionDef {
        identity,
        predicate: capability.to_owned(),
        object_class,
        default_subject: ActionSubject::User,
        // Every capability here is invoked by an authenticated caller
        // through the medcare-rs axum route surface — an external cause
        // (HTTP request) with no OGAR-side precondition to guard on,
        // matching `KausalSpec::External`'s doc.
        kausal: Some(KausalSpec::External),
        ..ActionDef::default()
    }
}

/// The medcare-rs healthcare capability surface — the **authoritative OGAR
/// action table** for the clinical / patient / care domain. One
/// [`ActionDef`] per capability, in [`HEALTHCARE_ACTION_NAMES`] order.
///
/// The per-concept capability split is CRUD-shaped and follows the
/// consumer DAL's measured method inventory (2026-07-07 harvest over the
/// consumer C# tree; the concrete method-name evidence stays in the
/// consumer repo, never committed here).
///
/// | subject concept | capabilities |
/// |---|---|
/// | `patient` (`0x0901`) | `register_patient` / `get_patient_record` / `list_patients` / `update_patient_access` |
/// | `diagnosis` (`0x0902`) | `add_diagnosis` / `get_diagnosis` / `list_diagnoses` / `delete_diagnosis` |
/// | `lab_value` (`0x0903`) | `add_lab_result` / `get_lab_result` / `list_lab_results` |
/// | `medication` (`0x0904`) | `add_medication` / `get_medication` / `list_medications` |
/// | `treatment` (`0x0905`) | `add_treatment` / `get_treatment` / `list_treatments` |
/// | `vital_sign` (`0x0907`) | `add_vital_sign` / `list_vital_signs` / `delete_vital_signs` |
#[must_use]
pub fn healthcare_actions() -> Vec<ActionDef> {
    const SUBJECT_OF: &[(&str, &str)] = &[
        ("register_patient", "patient"),
        ("get_patient_record", "patient"),
        ("list_patients", "patient"),
        ("update_patient_access", "patient"),
        ("add_diagnosis", "diagnosis"),
        ("get_diagnosis", "diagnosis"),
        ("list_diagnoses", "diagnosis"),
        ("delete_diagnosis", "diagnosis"),
        ("add_lab_result", "lab_value"),
        ("get_lab_result", "lab_value"),
        ("list_lab_results", "lab_value"),
        ("add_medication", "medication"),
        ("get_medication", "medication"),
        ("list_medications", "medication"),
        ("add_treatment", "treatment"),
        ("get_treatment", "treatment"),
        ("list_treatments", "treatment"),
        ("add_vital_sign", "vital_sign"),
        ("list_vital_signs", "vital_sign"),
        ("delete_vital_signs", "vital_sign"),
    ];
    SUBJECT_OF
        .iter()
        .map(|&(capability, subject)| healthcare_action_def(capability, subject))
        .collect()
}

/// The executors the authority EXPECTS to register against this table.
/// medcare-rs is the Rust consumer (P4 of the parity plan wires its
/// `HOT_PLUG` const + activation test against exactly this list).
pub const HEALTHCARE_EXPECTED_EXECUTORS: &[&str] = &["medcare-rs"];

/// The distinct subject classids this table binds (canon-high concept
/// ids). Six of the seven `HealthcarePort` entities — `visit` (0x0906) is
/// deliberately absent (zero harvest evidence; see the module doc). A
/// registering consumer must activate exactly this set.
pub const HEALTHCARE_SUBJECT_CLASSIDS: &[u16] = &[
    crate::class_ids::PATIENT,
    crate::class_ids::DIAGNOSIS,
    crate::class_ids::LAB_VALUE,
    crate::class_ids::MEDICATION,
    crate::class_ids::TREATMENT,
    crate::class_ids::VITAL_SIGN,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{canonical_concept_domain, class_ids};
    use std::collections::BTreeSet;

    fn subject_concept_of(def: &ActionDef) -> &str {
        def.object_class
            .strip_prefix("ogit-health/")
            .expect("every healthcare ActionDef.object_class starts with `ogit-health/`")
    }

    #[test]
    fn table_length_matches_const_name_fingerprint() {
        let actions = healthcare_actions();
        assert_eq!(actions.len(), HEALTHCARE_ACTION_NAMES.len());
        for (def, name) in actions.iter().zip(HEALTHCARE_ACTION_NAMES) {
            assert_eq!(&def.predicate, name);
        }
    }

    #[test]
    fn capability_names_are_unique() {
        let actions = healthcare_actions();
        let names: BTreeSet<&str> = actions.iter().map(|d| d.predicate.as_str()).collect();
        assert_eq!(
            names.len(),
            actions.len(),
            "duplicate capability name in HEALTHCARE_ACTION_NAMES / healthcare_actions()"
        );
    }

    /// Every action's subject resolves to a minted `0x09XX` concept in
    /// [`class_ids::ALL`] — the fuse that catches a renamed or unminted
    /// concept before a consumer trusts this table.
    #[test]
    fn subjects_resolve_to_minted_0x09_health_concepts() {
        for def in healthcare_actions() {
            let concept = subject_concept_of(&def);
            let entry = class_ids::ALL
                .iter()
                .find(|(name, _)| *name == concept)
                .unwrap_or_else(|| {
                    panic!(
                        "{}: subject concept `{concept}` is not in class_ids::ALL",
                        def.predicate
                    )
                });
            assert_eq!(
                canonical_concept_domain(entry.1),
                crate::ConceptDomain::Health,
                "{}: subject concept `{concept}` (0x{:04X}) is not in the Health domain",
                def.predicate,
                entry.1
            );
        }
    }

    /// The distinct subject set of the table equals
    /// [`HEALTHCARE_SUBJECT_CLASSIDS`] — no phantom subject const, no
    /// subject the const forgot.
    #[test]
    fn subject_classids_const_matches_table() {
        let mut from_table: Vec<u16> = healthcare_actions()
            .iter()
            .map(|def| {
                crate::canonical_concept_id(subject_concept_of(def))
                    .expect("minted (previous test)")
            })
            .collect();
        from_table.sort_unstable();
        from_table.dedup();
        let mut from_const = HEALTHCARE_SUBJECT_CLASSIDS.to_vec();
        from_const.sort_unstable();
        assert_eq!(from_table, from_const);
    }

    /// `visit` (0x0906) is minted in the codebook but deliberately absent
    /// from this table (zero harvest evidence — module doc). This pin
    /// keeps the exclusion a visible decision instead of silent drift: if
    /// a visit capability ever lands, this test fails and the author
    /// updates both the table and the exclusion note.
    #[test]
    fn visit_is_minted_but_deliberately_tableless() {
        assert!(
            class_ids::ALL
                .iter()
                .any(|&(name, id)| name == "visit" && id == class_ids::VISIT),
            "visit must stay minted in the codebook"
        );
        assert!(
            !HEALTHCARE_SUBJECT_CLASSIDS.contains(&class_ids::VISIT),
            "visit gained a table row — update the module-doc exclusion note"
        );
        for def in healthcare_actions() {
            assert_ne!(
                subject_concept_of(&def),
                "visit",
                "{}: visit capability landed without updating the exclusion note",
                def.predicate
            );
        }
    }

    #[test]
    fn every_action_declares_external_kausal_and_user_subject() {
        for def in healthcare_actions() {
            assert_eq!(def.kausal, Some(KausalSpec::External));
            assert_eq!(def.default_subject, ActionSubject::User);
        }
    }

    /// Name-only by design (module doc): the hot-plug join must not
    /// depend on effect facts, and this table doesn't fabricate any.
    #[test]
    fn actions_are_name_only() {
        for def in healthcare_actions() {
            assert!(def.reads.is_empty(), "{}: unexpected reads", def.predicate);
            assert!(
                def.writes.is_empty(),
                "{}: unexpected writes",
                def.predicate
            );
        }
    }

    #[test]
    fn identity_and_object_class_are_well_formed() {
        for def in healthcare_actions() {
            assert!(def.identity.starts_with("ogit-health/"));
            assert!(def.object_class.starts_with("ogit-health/"));
            assert!(
                def.identity
                    .ends_with(&format!("::action_def::{}", def.predicate))
            );
        }
    }
}
