//! ERP capability surface — the **erp-rs authoritative action table**.
//!
//! This module declares the ERP capabilities the erp-rs consumer (crate
//! `erp-ledger`, in AdaWorldAPI/erp-rs) executes, one [`ActionDef`] per
//! capability. Every capability has the same subject concept,
//! `erp_polyfill` (`0x020C`, [`crate::class_ids::ERP_POLYFILL`]). It
//! registers in [`crate::capability_registry::domain_tables`] through the
//! generic [`crate::capability_registry::entries_from_actions`] derive, the
//! same path the other domain tables use.
//!
//! # Hand-authored for one consumer
//!
//! The table is hand-authored for one consumer, erp-rs (crate
//! `erp-ledger`). The behaviour behind `create_entry`, `post_entry` and
//! `read_entry` is checked in erp-rs against observations recorded on
//! Odoo 19.0 (commit `9cd6f090`); it is not derived here. The evidence
//! stays in the consumer repo and is never committed here.
//!
//! `record_time_entry` and `emit_row` belong to the time-entry slice. The
//! entry itself is the shared concept `billable_work_entry` (`0x0103`), which
//! the consumer plugs for its reading only; the capabilities stay on the
//! seat, so `0x0103` keeps answering `NoCapabilitiesFor` for everyone else.
//!
//! # Why the subject is the seat, not `commercial_document`
//!
//! The subject is `erp_polyfill`, the ERP consumer's behaviour seat, and not
//! `commercial_document` (`0x0202`): lance-graph-ogar routes
//! `commercial_document` as a reading-only canon storage domain for other
//! consumers, so it must keep answering `NoCapabilitiesFor`. No capability in
//! this table is declared on any concept other than `erp_polyfill`.
//!
//! # Name-only ActionDefs (empty effect facts)
//!
//! `reads`/`writes` are left empty, as in
//! [`crate::healthcare_actions`]: the hot-plug join keys on capability name
//! and subject classid alone, and no effect facts are invented here.

use crate::{ActionDef, ActionSubject, KausalSpec};

/// Every ERP capability name, in table order — the `const`-evaluable
/// fingerprint of [`erp_actions`] (same role as
/// [`crate::healthcare_actions::HEALTHCARE_ACTION_NAMES`]).
pub const ERP_ACTION_NAMES: &[&str] = &[
    // erp_polyfill (0x020C)
    "create_entry",
    "post_entry",
    "read_entry",
    "set_const",
    "mark_set",
    "record_time_entry",
    "emit_row",
];

const _: () = assert!(ERP_ACTION_NAMES.len() == 7);

/// Build one [`ActionDef`] for an ERP capability. `subject_concept` MUST be
/// a name minted in [`crate::class_ids::ALL`] under the `0x02XX` (Commerce)
/// domain — enforced by this module's tests, same fuse shape as
/// `healthcare_actions::healthcare_action_def`.
fn erp_action_def(capability: &'static str, subject_concept: &'static str) -> ActionDef {
    let object_class = format!("ogit-erp/{subject_concept}");
    let identity = format!("{object_class}::action_def::{capability}");
    ActionDef {
        identity,
        predicate: capability.to_owned(),
        object_class,
        default_subject: ActionSubject::User,
        // Invoked by the erp-rs host as an external cause with no
        // OGAR-side precondition to guard on, matching
        // `KausalSpec::External`'s doc (same as healthcare).
        kausal: Some(KausalSpec::External),
        ..ActionDef::default()
    }
}

/// The erp-rs ERP capability surface — the **authoritative OGAR action
/// table** for the ERP consumer. One [`ActionDef`] per capability, in
/// [`ERP_ACTION_NAMES`] order, all on the `erp_polyfill` seat (`0x020C`).
///
/// | capability | meaning |
/// |---|---|
/// | `create_entry` | creates a journal entry in draft |
/// | `post_entry` | posts a draft entry (guards: in scope, state = draft, has lines, balanced) |
/// | `read_entry` | reads an entry's state |
/// | `set_const` | host-effect word: target field := constant |
/// | `mark_set` | host-effect word: target field := 1 |
/// | `record_time_entry` | records one time entry (guards: in scope, fields present, quantity in range, employee known) |
/// | `emit_row` | host-effect word: hand the entry's row to a named sink |
#[must_use]
pub fn erp_actions() -> Vec<ActionDef> {
    const SUBJECT_OF: &[(&str, &str)] = &[
        ("create_entry", "erp_polyfill"),
        ("post_entry", "erp_polyfill"),
        ("read_entry", "erp_polyfill"),
        ("set_const", "erp_polyfill"),
        ("mark_set", "erp_polyfill"),
        ("record_time_entry", "erp_polyfill"),
        ("emit_row", "erp_polyfill"),
    ];
    SUBJECT_OF
        .iter()
        .map(|&(capability, subject)| erp_action_def(capability, subject))
        .collect()
}

/// The executors the authority EXPECTS to register against this table:
/// `erp-ledger`, the consumer crate in AdaWorldAPI/erp-rs.
pub const ERP_EXPECTED_EXECUTORS: &[&str] = &["erp-ledger"];

/// The distinct subject classids this table binds (canon-high concept ids):
/// the `erp_polyfill` seat only. A registering consumer must activate
/// exactly this set.
pub const ERP_SUBJECT_CLASSIDS: &[u16] = &[crate::class_ids::ERP_POLYFILL];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{canonical_concept_domain, class_ids};
    use std::collections::BTreeSet;

    fn subject_concept_of(def: &ActionDef) -> &str {
        def.object_class
            .strip_prefix("ogit-erp/")
            .expect("every ERP ActionDef.object_class starts with `ogit-erp/`")
    }

    #[test]
    fn table_length_matches_const_name_fingerprint() {
        let actions = erp_actions();
        assert_eq!(actions.len(), ERP_ACTION_NAMES.len());
        for (def, name) in actions.iter().zip(ERP_ACTION_NAMES) {
            assert_eq!(&def.predicate, name);
        }
    }

    #[test]
    fn capability_names_are_unique() {
        let actions = erp_actions();
        let names: BTreeSet<&str> = actions.iter().map(|d| d.predicate.as_str()).collect();
        assert_eq!(names.len(), actions.len());
    }

    /// Every name in [`ERP_ACTION_NAMES`] is the predicate of exactly one
    /// def, and every def's subject resolves to `0x020C` in the Commerce
    /// domain.
    #[test]
    fn each_name_is_one_def_and_every_subject_is_the_seat() {
        let actions = erp_actions();
        for name in ERP_ACTION_NAMES {
            assert_eq!(
                actions.iter().filter(|d| d.predicate == *name).count(),
                1,
                "`{name}` must be the predicate of exactly one def"
            );
        }
        for def in &actions {
            let concept = subject_concept_of(def);
            assert_eq!(concept, "erp_polyfill", "{}: wrong subject", def.predicate);
            let id = crate::canonical_concept_id(concept)
                .unwrap_or_else(|| panic!("{}: `{concept}` is not minted", def.predicate));
            assert_eq!(id, 0x020C);
            assert_eq!(id, class_ids::ERP_POLYFILL);
            assert_eq!(canonical_concept_domain(id), crate::ConceptDomain::Commerce);
        }
        assert_eq!(ERP_SUBJECT_CLASSIDS, &[0x020C]);
    }

    #[test]
    fn every_action_declares_external_kausal_and_user_subject() {
        for def in erp_actions() {
            assert_eq!(def.kausal, Some(KausalSpec::External));
            assert_eq!(def.default_subject, ActionSubject::User);
        }
    }

    /// Name-only by design (module doc): no invented effect facts.
    #[test]
    fn actions_are_name_only() {
        for def in erp_actions() {
            assert!(def.reads.is_empty(), "{}: unexpected reads", def.predicate);
            assert!(
                def.writes.is_empty(),
                "{}: unexpected writes",
                def.predicate
            );
        }
    }
}
