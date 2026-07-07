//! Parity-checker skeleton — the legacy-parity revival drift-fuse seed.
//!
//! # The legacy-parity revival mode
//!
//! Operator Ruling (b, refined) revives the existing WoA/SQLAlchemy ORM
//! path as a **parity oracle**, not a schema of record: the sink-in write
//! path (this crate's facet table + per-class DDL) is the new
//! System-of-Record, but the legacy ORM's read shape is kept around to
//! catch drift between what the lift *thinks* the schema is and what the
//! legacy ORM actually persists.
//!
//! [`check_parity`] compares the sink-in projected [`Class`] shape against
//! a legacy [`Class`] shape (a description of the ORM's actual columns)
//! field-by-field and reports every divergence as a [`ParityDrift`]. An
//! empty [`ParityReport`] is the `COUNT_FUSE`-style green state; a
//! non-empty report is the fuse blowing.
//!
//! This module is **pure, no I/O** — it compares two already-constructed
//! `Class` values. Associations are out of scope for this skeleton (scalar
//! attributes only); a follow-up can extend the comparison once the
//! skeleton is load-bearing.
//!
//! # The runtime this seeds (documented, NOT built this session)
//!
//! The full legacy-parity-revival runtime is a **transactional-outbox
//! dual-write**, not a naive parallel write (Ruling b explicitly rejects
//! naive parallel-write — a crash between two independent writes must
//! never cause silent drift):
//!
//! 1. A write transaction inserts into PostgreSQL (the System-of-Record)
//!    **and** inserts an outbox row in the *same* transaction. The outbox
//!    row is the single committed intent — either both land, or neither
//!    does.
//! 2. An async worker drains the outbox and:
//!    - projects the row into the lance-graph hot-path (the zero-copy read
//!      side), and
//!    - runs [`check_parity`] against the legacy ORM's read of the same
//!      logical record, screaming (alert / refuse-to-advance) on any
//!      [`ParityDrift`].
//!
//! Further follow-ups (also documented, not built): CDC as an eventual
//! replacement for the outbox-worker dual-write; a full PostgreSQL-ORM
//! adapter (this crate only emits DDL, it does not read/write rows).

use ogar_vocab::Class;

/// One divergence between the sink-in shape and the legacy ORM shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParityDrift {
    /// The legacy ORM has this field; the sink-in lift dropped it.
    MissingInSinkIn {
        /// The attribute name present only in the legacy shape.
        field: String,
    },
    /// The sink-in lift has this field; the legacy ORM shape lacks it.
    MissingInLegacy {
        /// The attribute name present only in the sink-in shape.
        field: String,
    },
    /// Both shapes have the field, but its `type_name` disagrees.
    TypeMismatch {
        /// The shared attribute name.
        field: String,
        /// The sink-in side's `type_name`.
        sink_in: Option<String>,
        /// The legacy side's `type_name`.
        legacy: Option<String>,
    },
    /// Both shapes have the field, but `options.required` disagrees.
    NullabilityMismatch {
        /// The shared attribute name.
        field: String,
        /// The sink-in side's `options.required`.
        sink_in: Option<bool>,
        /// The legacy side's `options.required`.
        legacy: Option<bool>,
    },
}

/// Drift-fuse report. Empty `drifts` == parity holds (the `COUNT_FUSE`
/// green state). A non-empty report is the fuse blowing — the caller (the
/// dual-write outbox worker, in the full design) refuses to commit /
/// alerts rather than silently tolerating drift.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ParityReport {
    /// Every divergence found, in legacy-declaration order (missing-in-
    /// sink-in / type / nullability first), then sink-in-only fields
    /// (missing-in-legacy) last. Iteration order, not sorted by name.
    pub drifts: Vec<ParityDrift>,
}

impl ParityReport {
    /// `true` when no divergence was found — the fuse has not blown.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.drifts.is_empty()
    }
}

/// Compare two `Class` shapes (sink-in vs legacy) field-by-field. Pure, no
/// I/O. Compares the attribute name sets, then per shared name the
/// `type_name` and `options.required`. Associations can be added in a
/// follow-up; this skeleton covers scalar columns — enough for the
/// drift-fuse seed.
#[must_use]
pub fn check_parity(sink_in: &Class, legacy: &Class) -> ParityReport {
    let mut drifts = Vec::new();

    for legacy_attr in &legacy.attributes {
        let Some(sink_in_attr) = sink_in
            .attributes
            .iter()
            .find(|a| a.name == legacy_attr.name)
        else {
            drifts.push(ParityDrift::MissingInSinkIn {
                field: legacy_attr.name.clone(),
            });
            continue;
        };

        if sink_in_attr.type_name != legacy_attr.type_name {
            drifts.push(ParityDrift::TypeMismatch {
                field: legacy_attr.name.clone(),
                sink_in: sink_in_attr.type_name.clone(),
                legacy: legacy_attr.type_name.clone(),
            });
        }

        if sink_in_attr.options.required != legacy_attr.options.required {
            drifts.push(ParityDrift::NullabilityMismatch {
                field: legacy_attr.name.clone(),
                sink_in: sink_in_attr.options.required,
                legacy: legacy_attr.options.required,
            });
        }
    }

    for sink_in_attr in &sink_in.attributes {
        if !legacy
            .attributes
            .iter()
            .any(|a| a.name == sink_in_attr.name)
        {
            drifts.push(ParityDrift::MissingInLegacy {
                field: sink_in_attr.name.clone(),
            });
        }
    }

    ParityReport { drifts }
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ogar_vocab::Attribute;

    fn attr(name: &str, type_name: &str, required: Option<bool>) -> Attribute {
        let mut a = Attribute::new(name);
        a.type_name = Some(type_name.into());
        a.options.required = required;
        a
    }

    #[test]
    fn identical_shapes_have_clean_parity() {
        let mut c = Class::new("timesheet_activities");
        c.attributes
            .push(attr("beschreibung", "string", Some(true)));
        c.attributes.push(attr("created_at", "datetime", None));

        let report = check_parity(&c, &c.clone());
        assert!(report.is_clean(), "expected clean, got: {report:?}");
    }

    #[test]
    fn dropped_column_blows_the_fuse() {
        let mut legacy = Class::new("t");
        legacy.attributes.push(attr("foo", "string", None));
        let sink_in = Class::new("t");

        let report = check_parity(&sink_in, &legacy);
        assert_eq!(
            report.drifts,
            vec![ParityDrift::MissingInSinkIn {
                field: "foo".into()
            }]
        );
    }

    #[test]
    fn type_mismatch_blows_the_fuse() {
        let mut sink_in = Class::new("t");
        sink_in.attributes.push(attr("foo", "string", None));
        let mut legacy = Class::new("t");
        legacy.attributes.push(attr("foo", "text", None));

        let report = check_parity(&sink_in, &legacy);
        assert_eq!(
            report.drifts,
            vec![ParityDrift::TypeMismatch {
                field: "foo".into(),
                sink_in: Some("string".into()),
                legacy: Some("text".into()),
            }]
        );
    }

    #[test]
    fn nullability_mismatch_blows_the_fuse() {
        let mut sink_in = Class::new("t");
        sink_in.attributes.push(attr("foo", "string", Some(true)));
        let mut legacy = Class::new("t");
        legacy.attributes.push(attr("foo", "string", Some(false)));

        let report = check_parity(&sink_in, &legacy);
        assert_eq!(
            report.drifts,
            vec![ParityDrift::NullabilityMismatch {
                field: "foo".into(),
                sink_in: Some(true),
                legacy: Some(false),
            }]
        );
    }

    /// Ties SPEC-7's parity oracle to a real WoA class: `TimesheetActivity`
    /// (`WoA/models.py:1746`, read-only source, transcribed faithfully).
    ///
    /// NOTE (Sonnet grind, SPEC-7): as in `lib.rs`'s
    /// `woa_timesheet_activity_ddl_roundtrips_field_set`, SPEC-5's real
    /// `CompiledClass` lift output was not available on this branch at
    /// build time (parallel, unmerged sibling worktree). Both "sink_in"
    /// and "legacy" sides here are hand-transcribed directly from
    /// `WoA/models.py:1746` (read in full) as a local stand-in — this
    /// specific test therefore does not yet exercise the real
    /// lift-vs-ORM disagreement surface the full design targets; it only
    /// proves the oracle agrees with itself on a real WoA shape. Re-wire
    /// to the actual SPEC-5 fixture once merged (tracked as a Ledger
    /// follow-up in the sign-off report, not invented here).
    #[test]
    fn woa_timesheet_activity_sink_in_matches_hand_legacy() {
        let mut sink_in = Class::new("TimesheetActivity");
        sink_in
            .attributes
            .push(attr("beschreibung", "string", Some(true)));
        sink_in
            .attributes
            .push(attr("created_at", "datetime", None));

        let mut legacy = Class::new("TimesheetActivity");
        legacy
            .attributes
            .push(attr("beschreibung", "string", Some(true)));
        legacy.attributes.push(attr("created_at", "datetime", None));

        let report = check_parity(&sink_in, &legacy);
        assert!(report.is_clean(), "expected clean, got: {report:?}");
    }
}
