//! `arm_ledger` — the explicit, append-only ledger of behaviour arms.
//!
//! A behaviour (one [`crate::ActionDef`]) registers in the DT64 `BindStore` as
//! one method at a behaviour classid. Its LEFT 16 bits are the class it applies
//! on, as the registering consumer names it; its RIGHT 16 bits are the
//! behaviour's **arm**, in [`ARM_FIRST`]`..=`[`ARM_LAST`] (`0xF001..=0xFFFF`;
//! `0xF000` is the class's polyfill lens and is never assigned). The arm is the
//! same whichever consumer registers the behaviour and never changes once
//! assigned.
//!
//! # Assignment is a ledger line
//!
//! An arm is assigned by one ledger entry, `behaviour identity -> arm`
//! ([`ActionDef::identity`](crate::ActionDef)). There is no computation: an
//! earlier version derived the arm from a hash of the identity, which can
//! collide and offered no override. An identity with no ledger entry has no
//! arm; the converter refuses it by name and never falls back to a number it
//! made up.
//!
//! # Two ledgers, two ranges
//!
//! | ledger | who writes it | arms |
//! |---|---|---|
//! | [`CANONICAL`] | OGAR, for every def of [`crate::capability_registry::domain_tables`] | [`OGAR_ARM_FIRST`]`..=`[`OGAR_ARM_LAST`] = `0xF001..=0xF7FF` |
//! | a consumer's [`ConsumerArms`] | the consumer, for behaviours OGAR does not know | [`CONSUMER_ARM_FIRST`]`..=`[`CONSUMER_ARM_LAST`] = `0xF800..=0xFFFF` |
//!
//! The upper half of the arm space is reserved for consumers and the lower half
//! for OGAR, so a number a consumer takes today can never be one OGAR assigns
//! tomorrow: OGAR's future arms are drawn from `0xF001..=0xF7FF` only. The rule
//! is enforced by [`ConsumerArms::validate`], which refuses by name
//! ([`LedgerRefusal`]):
//!
//! - an arm outside `0xF001..=0xFFFF` ([`LedgerRefusal::ArmOutOfRange`]);
//! - a canonical identity given another arm than its canonical one
//!   ([`LedgerRefusal::RebindsCanonicalIdentity`]); restating the canonical arm
//!   is accepted and changes nothing;
//! - an arm that the canonical ledger already assigns to another identity
//!   ([`LedgerRefusal::ReusesCanonicalArm`]);
//! - an arm in OGAR's range that is not assigned yet
//!   ([`LedgerRefusal::ReservedForOgar`]);
//! - the same identity twice, or two identities on one arm, in one consumer
//!   ledger ([`LedgerRefusal::DuplicateIdentity`], [`LedgerRefusal::DuplicateArm`]).
//!
//! Two different consumers each own their range numbers; OGAR does not
//! arbitrate between them. If two consumers give two behaviours one arm on one
//! shared class, the DT64 store refuses the second as `BehaviourImmutable` at
//! registration (the two bindings differ at least in `name`); the consumers
//! agree on the numbers out of band.
//!
//! # Append-only
//!
//! [`CANONICAL`] is only ever appended to: a line is never edited, removed,
//! renumbered or reordered, and a retired def keeps its line. The next arm is
//! the last one plus one, until `0xF7FF`. `tests/` pins the whole ledger
//! literally, so any other change fails the build's tests by name, and a def
//! added to a domain table without a ledger line fails
//! `every_domain_table_def_has_a_ledger_entry`.
//!
//! # The canonical order
//!
//! The first 50 lines are the 50 defs of the five authoritative tables, in the
//! order [`crate::capability_registry::domain_tables`] lists the tables and
//! each table lists its defs: ocr (14), geo (6), healthcare (20), document (3),
//! erp (7), numbered `0xF001` upward with no gap. That order is only how the
//! numbers were first handed out; it does not constrain later lines.

use std::collections::{BTreeMap, BTreeSet};

/// The first assignable arm. `0xF000` is the class's polyfill lens.
pub const ARM_FIRST: u16 = 0xF001;
/// The last arm.
pub const ARM_LAST: u16 = 0xFFFF;
/// The first arm OGAR assigns.
pub const OGAR_ARM_FIRST: u16 = 0xF001;
/// The last arm OGAR assigns. Everything above is reserved for consumers.
pub const OGAR_ARM_LAST: u16 = 0xF7FF;
/// The first arm a consumer may assign.
pub const CONSUMER_ARM_FIRST: u16 = 0xF800;
/// The last arm a consumer may assign.
pub const CONSUMER_ARM_LAST: u16 = 0xFFFF;

// The two ranges are adjacent, disjoint and together exactly the arm space.
const _: () = assert!(OGAR_ARM_FIRST == ARM_FIRST);
const _: () = assert!(OGAR_ARM_LAST + 1 == CONSUMER_ARM_FIRST);
const _: () = assert!(CONSUMER_ARM_LAST == ARM_LAST);

/// OGAR's canonical ledger: `(behaviour identity, arm)`, append-only (see the
/// module doc). Covers every def of [`crate::capability_registry::domain_tables`],
/// including the ones whose conversion is refused for another reason.
pub const CANONICAL: &[(&str, u16)] = &[
    ("ogit-ocr/textline::action_def::recognize_line", 0xF001),
    ("ogit-ocr/page_image::action_def::recognize_page", 0xF002),
    (
        "ogit-ocr/page_image::action_def::extract_text_layer",
        0xF003,
    ),
    (
        "ogit-ocr/page_image::action_def::extract_page_image",
        0xF004,
    ),
    ("ogit-ocr/ocr_renderer::action_def::render_text", 0xF005),
    ("ogit-ocr/ocr_renderer::action_def::render_tsv", 0xF006),
    ("ogit-ocr/ocr_renderer::action_def::render_hocr", 0xF007),
    (
        "ogit-ocr/ocr_renderer::action_def::render_searchable_pdf",
        0xF008,
    ),
    (
        "ogit-ocr/page_image::action_def::recognize_page_words",
        0xF009,
    ),
    (
        "ogit-ocr/page_image::action_def::recognize_document",
        0xF00A,
    ),
    ("ogit-ocr/page_layout::action_def::harvest_fields", 0xF00B),
    ("ogit-ocr/page_image::action_def::segment_page", 0xF00C),
    (
        "ogit-ocr/page_image::action_def::detect_halftone_regions",
        0xF00D,
    ),
    (
        "ogit-ocr/page_layout::action_def::detect_page_furniture",
        0xF00E,
    ),
    ("ogit-geo/osm_node::action_def::locate_point", 0xF00F),
    ("ogit-geo/osm_node::action_def::locate_tile", 0xF010),
    ("ogit-geo/osm_node::action_def::locate_fragment", 0xF011),
    ("ogit-geo/osm_node::action_def::project_fields", 0xF012),
    ("ogit-geo/osm_way::action_def::street_edges", 0xF013),
    ("ogit-geo/osm_way::action_def::polyline_length", 0xF014),
    ("ogit-health/patient::action_def::register_patient", 0xF015),
    (
        "ogit-health/patient::action_def::get_patient_record",
        0xF016,
    ),
    ("ogit-health/patient::action_def::list_patients", 0xF017),
    (
        "ogit-health/patient::action_def::update_patient_access",
        0xF018,
    ),
    ("ogit-health/diagnosis::action_def::add_diagnosis", 0xF019),
    ("ogit-health/diagnosis::action_def::get_diagnosis", 0xF01A),
    ("ogit-health/diagnosis::action_def::list_diagnoses", 0xF01B),
    (
        "ogit-health/diagnosis::action_def::delete_diagnosis",
        0xF01C,
    ),
    ("ogit-health/lab_value::action_def::add_lab_result", 0xF01D),
    ("ogit-health/lab_value::action_def::get_lab_result", 0xF01E),
    (
        "ogit-health/lab_value::action_def::list_lab_results",
        0xF01F,
    ),
    ("ogit-health/medication::action_def::add_medication", 0xF020),
    ("ogit-health/medication::action_def::get_medication", 0xF021),
    (
        "ogit-health/medication::action_def::list_medications",
        0xF022,
    ),
    ("ogit-health/treatment::action_def::add_treatment", 0xF023),
    ("ogit-health/treatment::action_def::get_treatment", 0xF024),
    ("ogit-health/treatment::action_def::list_treatments", 0xF025),
    ("ogit-health/vital_sign::action_def::add_vital_sign", 0xF026),
    (
        "ogit-health/vital_sign::action_def::list_vital_signs",
        0xF027,
    ),
    (
        "ogit-health/vital_sign::action_def::delete_vital_signs",
        0xF028,
    ),
    ("ogit-ocr/document::action_def::persist_document", 0xF029),
    ("ogit-ocr/document::action_def::read_document", 0xF02A),
    (
        "ogit-ocr/document::action_def::reconstruct_document",
        0xF02B,
    ),
    ("ogit-erp/erp_polyfill::action_def::create_entry", 0xF02C),
    ("ogit-erp/erp_polyfill::action_def::post_entry", 0xF02D),
    ("ogit-erp/erp_polyfill::action_def::read_entry", 0xF02E),
    ("ogit-erp/erp_polyfill::action_def::set_const", 0xF02F),
    ("ogit-erp/erp_polyfill::action_def::mark_set", 0xF030),
    (
        "ogit-erp/erp_polyfill::action_def::record_time_entry",
        0xF031,
    ),
    ("ogit-erp/erp_polyfill::action_def::emit_row", 0xF032),
];

/// The canonical arm of the behaviour with this `identity`, if OGAR has
/// assigned one.
#[must_use]
pub fn canonical_arm(identity: &str) -> Option<u16> {
    CANONICAL
        .iter()
        .find(|(id, _)| *id == identity)
        .map(|&(_, arm)| arm)
}

/// Why a consumer's ledger entries were refused. Every case is named; none is
/// resolved by picking a winner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerRefusal {
    /// An arm outside `0xF001..=0xFFFF` (`0xF000` is the polyfill lens; below
    /// it is not a behaviour arm).
    ArmOutOfRange {
        /// The identity the entry is for.
        identity: String,
        /// The arm it asked for.
        arm: u16,
    },
    /// A canonical identity given an arm other than its canonical one.
    RebindsCanonicalIdentity {
        /// The canonical identity.
        identity: String,
        /// Its canonical arm.
        canonical: u16,
        /// The arm the consumer asked for.
        asked: u16,
    },
    /// An arm the canonical ledger assigns to another identity.
    ReusesCanonicalArm {
        /// The consumer's identity.
        identity: String,
        /// The arm it asked for.
        arm: u16,
        /// The canonical identity that holds the arm.
        canonical_identity: String,
    },
    /// An arm in OGAR's range (`0xF001..=0xF7FF`) that a consumer may not take,
    /// even where OGAR has not assigned it yet.
    ReservedForOgar {
        /// The consumer's identity.
        identity: String,
        /// The arm it asked for.
        arm: u16,
    },
    /// The same identity twice in one consumer ledger.
    DuplicateIdentity {
        /// The identity.
        identity: String,
    },
    /// Two identities on one arm in one consumer ledger.
    DuplicateArm {
        /// The arm.
        arm: u16,
        /// The first identity.
        first: String,
        /// The second identity.
        second: String,
    },
}

/// A consumer's own ledger entries, for behaviours OGAR does not know. Passed
/// beside the consumer's class mapping; [`ConsumerArms::default`] is empty, so
/// a consumer with no behaviour of its own uses the canonical ledger alone.
/// Entries are checked by [`ConsumerArms::validate`], not on insertion, so a
/// bad entry is reported by name where the ledger is used.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConsumerArms {
    entries: Vec<(String, u16)>,
}

impl ConsumerArms {
    /// An empty ledger: the canonical ledger alone.
    #[must_use]
    pub fn new() -> ConsumerArms {
        ConsumerArms::default()
    }

    /// Add one entry: the behaviour `identity` takes `arm`.
    #[must_use]
    pub fn with(mut self, identity: &str, arm: u16) -> ConsumerArms {
        self.entries.push((identity.to_string(), arm));
        self
    }

    /// The entries as added, in order.
    #[must_use]
    pub fn entries(&self) -> &[(String, u16)] {
        &self.entries
    }

    /// Check every entry against the rules in the module doc.
    ///
    /// # Errors
    ///
    /// The first [`LedgerRefusal`], in entry order.
    pub fn validate(&self) -> Result<(), LedgerRefusal> {
        let mut by_arm: BTreeMap<u16, &str> = BTreeMap::new();
        let mut identities: BTreeSet<&str> = BTreeSet::new();
        for (identity, arm) in &self.entries {
            let (identity, arm) = (identity.as_str(), *arm);
            if !(ARM_FIRST..=ARM_LAST).contains(&arm) {
                return Err(LedgerRefusal::ArmOutOfRange {
                    identity: identity.to_string(),
                    arm,
                });
            }
            if !identities.insert(identity) {
                return Err(LedgerRefusal::DuplicateIdentity {
                    identity: identity.to_string(),
                });
            }
            if let Some(canonical) = canonical_arm(identity) {
                if canonical != arm {
                    return Err(LedgerRefusal::RebindsCanonicalIdentity {
                        identity: identity.to_string(),
                        canonical,
                        asked: arm,
                    });
                }
                // Restating the canonical arm: accepted, nothing to add.
                continue;
            }
            if let Some((held_by, _)) = CANONICAL.iter().find(|(_, a)| *a == arm) {
                return Err(LedgerRefusal::ReusesCanonicalArm {
                    identity: identity.to_string(),
                    arm,
                    canonical_identity: (*held_by).to_string(),
                });
            }
            if !(CONSUMER_ARM_FIRST..=CONSUMER_ARM_LAST).contains(&arm) {
                return Err(LedgerRefusal::ReservedForOgar {
                    identity: identity.to_string(),
                    arm,
                });
            }
            if let Some(first) = by_arm.insert(arm, identity) {
                return Err(LedgerRefusal::DuplicateArm {
                    arm,
                    first: first.to_string(),
                    second: identity.to_string(),
                });
            }
        }
        Ok(())
    }

    /// The arm of `identity`: its canonical arm, else this consumer's entry,
    /// else `None`. There is no other source: an identity with no entry has no
    /// arm. Meaningful for a ledger that passed [`ConsumerArms::validate`].
    #[must_use]
    pub fn arm_of(&self, identity: &str) -> Option<u16> {
        canonical_arm(identity).or_else(|| {
            self.entries
                .iter()
                .find(|(id, _)| id == identity)
                .map(|&(_, arm)| arm)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_registry::domain_tables;

    /// The canonical ledger, pinned literally: identity and arm, in order.
    /// Append a line here together with the line in [`CANONICAL`]; never edit,
    /// remove, renumber or reorder one.
    const GOLDEN: [(&str, u16); 50] = [
        ("ogit-ocr/textline::action_def::recognize_line", 0xF001),
        ("ogit-ocr/page_image::action_def::recognize_page", 0xF002),
        (
            "ogit-ocr/page_image::action_def::extract_text_layer",
            0xF003,
        ),
        (
            "ogit-ocr/page_image::action_def::extract_page_image",
            0xF004,
        ),
        ("ogit-ocr/ocr_renderer::action_def::render_text", 0xF005),
        ("ogit-ocr/ocr_renderer::action_def::render_tsv", 0xF006),
        ("ogit-ocr/ocr_renderer::action_def::render_hocr", 0xF007),
        (
            "ogit-ocr/ocr_renderer::action_def::render_searchable_pdf",
            0xF008,
        ),
        (
            "ogit-ocr/page_image::action_def::recognize_page_words",
            0xF009,
        ),
        (
            "ogit-ocr/page_image::action_def::recognize_document",
            0xF00A,
        ),
        ("ogit-ocr/page_layout::action_def::harvest_fields", 0xF00B),
        ("ogit-ocr/page_image::action_def::segment_page", 0xF00C),
        (
            "ogit-ocr/page_image::action_def::detect_halftone_regions",
            0xF00D,
        ),
        (
            "ogit-ocr/page_layout::action_def::detect_page_furniture",
            0xF00E,
        ),
        ("ogit-geo/osm_node::action_def::locate_point", 0xF00F),
        ("ogit-geo/osm_node::action_def::locate_tile", 0xF010),
        ("ogit-geo/osm_node::action_def::locate_fragment", 0xF011),
        ("ogit-geo/osm_node::action_def::project_fields", 0xF012),
        ("ogit-geo/osm_way::action_def::street_edges", 0xF013),
        ("ogit-geo/osm_way::action_def::polyline_length", 0xF014),
        ("ogit-health/patient::action_def::register_patient", 0xF015),
        (
            "ogit-health/patient::action_def::get_patient_record",
            0xF016,
        ),
        ("ogit-health/patient::action_def::list_patients", 0xF017),
        (
            "ogit-health/patient::action_def::update_patient_access",
            0xF018,
        ),
        ("ogit-health/diagnosis::action_def::add_diagnosis", 0xF019),
        ("ogit-health/diagnosis::action_def::get_diagnosis", 0xF01A),
        ("ogit-health/diagnosis::action_def::list_diagnoses", 0xF01B),
        (
            "ogit-health/diagnosis::action_def::delete_diagnosis",
            0xF01C,
        ),
        ("ogit-health/lab_value::action_def::add_lab_result", 0xF01D),
        ("ogit-health/lab_value::action_def::get_lab_result", 0xF01E),
        (
            "ogit-health/lab_value::action_def::list_lab_results",
            0xF01F,
        ),
        ("ogit-health/medication::action_def::add_medication", 0xF020),
        ("ogit-health/medication::action_def::get_medication", 0xF021),
        (
            "ogit-health/medication::action_def::list_medications",
            0xF022,
        ),
        ("ogit-health/treatment::action_def::add_treatment", 0xF023),
        ("ogit-health/treatment::action_def::get_treatment", 0xF024),
        ("ogit-health/treatment::action_def::list_treatments", 0xF025),
        ("ogit-health/vital_sign::action_def::add_vital_sign", 0xF026),
        (
            "ogit-health/vital_sign::action_def::list_vital_signs",
            0xF027,
        ),
        (
            "ogit-health/vital_sign::action_def::delete_vital_signs",
            0xF028,
        ),
        ("ogit-ocr/document::action_def::persist_document", 0xF029),
        ("ogit-ocr/document::action_def::read_document", 0xF02A),
        (
            "ogit-ocr/document::action_def::reconstruct_document",
            0xF02B,
        ),
        ("ogit-erp/erp_polyfill::action_def::create_entry", 0xF02C),
        ("ogit-erp/erp_polyfill::action_def::post_entry", 0xF02D),
        ("ogit-erp/erp_polyfill::action_def::read_entry", 0xF02E),
        ("ogit-erp/erp_polyfill::action_def::set_const", 0xF02F),
        ("ogit-erp/erp_polyfill::action_def::mark_set", 0xF030),
        (
            "ogit-erp/erp_polyfill::action_def::record_time_entry",
            0xF031,
        ),
        ("ogit-erp/erp_polyfill::action_def::emit_row", 0xF032),
    ];

    #[test]
    fn the_canonical_ledger_is_pinned_literally() {
        assert_eq!(
            CANONICAL.len(),
            GOLDEN.len(),
            "the ledger is append-only: extend GOLDEN with the new line"
        );
        for (i, (got, want)) in CANONICAL.iter().zip(GOLDEN).enumerate() {
            assert_eq!(*got, want, "ledger line {i} changed");
        }
        assert_eq!(CANONICAL, &GOLDEN[..]);
    }

    #[test]
    fn the_ledger_has_no_duplicate_identity_or_arm_and_every_arm_is_in_ogars_range() {
        let ids: BTreeSet<&str> = CANONICAL.iter().map(|(i, _)| *i).collect();
        let arms: BTreeSet<u16> = CANONICAL.iter().map(|(_, a)| *a).collect();
        assert_eq!(ids.len(), CANONICAL.len(), "duplicate identity");
        assert_eq!(arms.len(), CANONICAL.len(), "duplicate arm");
        for (identity, arm) in CANONICAL {
            assert!((ARM_FIRST..=ARM_LAST).contains(arm), "{identity}: {arm:#x}");
            assert!(
                (OGAR_ARM_FIRST..=OGAR_ARM_LAST).contains(arm),
                "{identity}: {arm:#x} is outside OGAR's range"
            );
        }
    }

    #[test]
    fn the_first_fifty_numbers_are_dense_from_f001() {
        for (i, (_, arm)) in CANONICAL.iter().take(50).enumerate() {
            assert_eq!(usize::from(*arm), 0xF001 + i);
        }
    }

    #[test]
    fn every_domain_table_def_has_a_ledger_entry() {
        let mut n = 0;
        for table in domain_tables() {
            for def in (table.actions)() {
                assert!(
                    canonical_arm(&def.identity).is_some(),
                    "{}: def `{}` has no line in arm_ledger::CANONICAL",
                    table.domain,
                    def.identity
                );
                n += 1;
            }
        }
        assert_eq!(n, 50);
    }

    #[test]
    fn a_valid_consumer_ledger_is_accepted_and_resolves() {
        let known = CANONICAL[0];
        let c = ConsumerArms::new()
            .with("zammad/crm_ticket::action_def::close", 0xF800)
            .with("zammad/crm_ticket::action_def::reopen", 0xFFFF)
            // Restating a canonical arm is accepted.
            .with(known.0, known.1);
        assert_eq!(c.validate(), Ok(()));
        assert_eq!(
            c.arm_of("zammad/crm_ticket::action_def::close"),
            Some(0xF800)
        );
        assert_eq!(c.arm_of(known.0), Some(known.1));
        assert_eq!(c.arm_of("nobody::action_def::x"), None);
        assert_eq!(ConsumerArms::new().validate(), Ok(()));
    }

    #[test]
    fn a_consumer_ledger_is_refused_by_name() {
        let (canon_id, canon_arm) = CANONICAL[3];
        let v = |c: ConsumerArms| c.validate();
        assert_eq!(
            v(ConsumerArms::new().with("x", 0xF000)),
            Err(LedgerRefusal::ArmOutOfRange {
                identity: "x".into(),
                arm: 0xF000
            })
        );
        assert_eq!(
            v(ConsumerArms::new().with("x", 0)),
            Err(LedgerRefusal::ArmOutOfRange {
                identity: "x".into(),
                arm: 0
            })
        );
        // A canonical identity cannot move.
        assert_eq!(
            v(ConsumerArms::new().with(canon_id, 0xF900)),
            Err(LedgerRefusal::RebindsCanonicalIdentity {
                identity: canon_id.into(),
                canonical: canon_arm,
                asked: 0xF900
            })
        );
        // A canonical arm cannot be reused by another identity.
        assert_eq!(
            v(ConsumerArms::new().with("x", canon_arm)),
            Err(LedgerRefusal::ReusesCanonicalArm {
                identity: "x".into(),
                arm: canon_arm,
                canonical_identity: canon_id.into()
            })
        );
        // Nor may a consumer take a number OGAR has not assigned yet.
        assert_eq!(
            v(ConsumerArms::new().with("x", 0xF7FF)),
            Err(LedgerRefusal::ReservedForOgar {
                identity: "x".into(),
                arm: 0xF7FF
            })
        );
        assert_eq!(
            v(ConsumerArms::new().with("x", 0xF800).with("x", 0xF801)),
            Err(LedgerRefusal::DuplicateIdentity {
                identity: "x".into()
            })
        );
        assert_eq!(
            v(ConsumerArms::new().with("x", 0xF800).with("y", 0xF800)),
            Err(LedgerRefusal::DuplicateArm {
                arm: 0xF800,
                first: "x".into(),
                second: "y".into()
            })
        );
    }
}
