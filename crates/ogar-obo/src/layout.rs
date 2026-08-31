//! The **per-class little-endian row contract** of the OBO-core bake — the
//! carve DECLARED once, per concept, instead of re-typed as literals at every
//! reader.
//!
//! # Why this module exists (the 2026-08-13 drift finding)
//!
//! The value-slab carve of this bake was written down as bare literals
//! (`ENTITY_TYPE_SLAB_OFFSET = 96`, `EDGE_LANE_SLAB_OFFSET = 112`) — and the
//! same literals were hand-mirrored by a downstream consumer. Two repos agreed
//! with each other while both diverged from the generic default tenant carve:
//! a reader resolving `EntityType` through the DEFAULT descriptor reads a
//! different offset than the one this bake writes. The literals asserted
//! self-consistency; nothing declared the class's OWN reading.
//!
//! This module is that declaration: **the classid picks the reading** (the
//! ClassView move). Every lens adapter over these rows — the spine lens, the
//! edge lanes, an ELK observer, a CURIE resolver, a downstream rails carve —
//! derives its offsets from [`ClassRowSchema`], and a consumer pins its mirror
//! against this schema in a bridge test instead of re-typing bytes.
//!
//! # Scope, honestly
//!
//! The schema declares what THIS bake writes. It intentionally differs from
//! the generic default value-tenant carve — that is now a **declared per-class
//! carve**, not a silent drift — and the registry-side registration of this
//! schema as the `0x03xx` classes' `ValueSchema` (so a canon reader resolves
//! it through `ClassView` rather than through this crate) is the paired
//! follow-up, adjudicated at the contract's home, not here.

use crate::registry;

/// One declared little-endian field region of a class's 512-byte row, offsets
/// **relative to the 480-byte value slab** (row offset = [`crate::VALUE_OFFSET`]
/// + `slab_off`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowField {
    /// Value-slab-relative byte offset of the field's first byte.
    pub slab_off: usize,
    /// The field's width in bytes.
    pub bytes: usize,
}

impl RowField {
    /// The field's absolute row offset (key + edge block skipped).
    #[must_use]
    pub const fn row_off(&self) -> usize {
        crate::VALUE_OFFSET + self.slab_off
    }

    /// One past the field's last value-slab-relative byte.
    #[must_use]
    pub const fn slab_end(&self) -> usize {
        self.slab_off + self.bytes
    }
}

/// The little-endian row contract a classid family declares — where, inside
/// the canonical `key(16) | edges(16) | value(480)` row, THIS class's value
/// tenants actually sit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassRowSchema {
    /// The `EntityType` tenant (`u16` namespace ordinal, little-endian).
    pub entity_type: RowField,
    /// The edge-lane slab (`lane_count` lanes of [`crate::edges::LANE_BYTES`]
    /// each; each lane `classid(4) + 4×u24` under the G2 grace carving).
    pub edge_lanes: RowField,
    /// Number of edge lanes inside `edge_lanes`.
    pub lane_count: usize,
}

/// The one row schema every concept this crate bakes declares today. A future
/// class that needs a different carve declares its OWN `ClassRowSchema` and is
/// resolved through [`row_schema_of`] — never by widening or moving this one
/// (RESERVE, DON'T RECLAIM).
pub const OBO_CORE_ROW: ClassRowSchema = ClassRowSchema {
    entity_type: RowField {
        slab_off: 96,
        bytes: 2,
    },
    edge_lanes: RowField {
        slab_off: 112,
        bytes: 23 * 16,
    },
    lane_count: 23,
};

// The schema is self-consistent and fills the row exactly: EntityType ends
// before the lanes begin, and the lanes run to the end of the 480-byte slab.
const _: () = assert!(OBO_CORE_ROW.entity_type.slab_end() <= OBO_CORE_ROW.edge_lanes.slab_off);
const _: () = assert!(
    crate::VALUE_OFFSET + OBO_CORE_ROW.edge_lanes.slab_end() == crate::NODE_ROW_STRIDE,
    "the edge lanes must run to the end of the row"
);
const _: () = assert!(OBO_CORE_ROW.edge_lanes.bytes == OBO_CORE_ROW.lane_count * 16);

/// Resolve a concept id (the hi-u16 of a render classid) to its declared row
/// schema — the ClassView move: **the classid picks the reading**, a reader
/// never assumes a carve.
///
/// `None` for anything this crate does not bake — including `0x0306`
/// (`ogar-ro`'s relation-BODY classid, whose 512-byte row is `ogar-loco`
/// call-slab shaped, not this schema) and every concept outside the
/// registries. Refused, not guessed.
///
/// Accepts BOTH address forms of an aliased namespace (the reader rule the S3
/// producer flip set for [`Namespace::from_concept_id`]): a pre-flip artifact
/// keyed `0x0301` carries exactly this carve, so its legacy concept id must
/// resolve to the same schema the domain form does.
#[must_use]
pub fn row_schema_of(concept: u16) -> Option<&'static ClassRowSchema> {
    // Fold a legacy alias onto its canonical (registry-minted) form before the
    // registry probe; a concept without an alias probes as itself.
    let canonical =
        crate::Namespace::from_concept_id(concept).map_or(concept, crate::Namespace::concept_id);
    let known = registry::OBO_CORE
        .specs()
        .iter()
        .chain(registry::META_STUDY_SPINE.specs())
        .any(|s| s.concept_id == canonical);
    known.then_some(&OBO_CORE_ROW)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_baked_concept_resolves_to_its_declared_schema() {
        for s in registry::OBO_CORE
            .specs()
            .iter()
            .chain(registry::META_STUDY_SPINE.specs())
        {
            assert_eq!(
                row_schema_of(s.concept_id),
                Some(&OBO_CORE_ROW),
                "{} must declare its row carve",
                s.prefix
            );
        }
    }

    /// A pre-flip artifact's rows carry the legacy ids and EXACTLY this
    /// carve — the reader rule (`from_concept_id` accepts both) extends to
    /// the schema resolution, or a legacy artifact becomes unreadable the
    /// moment the registry mints the domain form.
    #[test]
    fn legacy_alias_concepts_resolve_to_the_same_schema() {
        for legacy in [0x0301u16, 0x0302, 0x0303] {
            assert_eq!(
                row_schema_of(legacy),
                Some(&OBO_CORE_ROW),
                "{legacy:#06x} (legacy alias) must resolve the declared carve"
            );
        }
    }

    #[test]
    fn foreign_concepts_are_refused_not_guessed() {
        // 0x0306 is ogar-ro's relation-body classid — call-slab shaped, NOT
        // this row schema. Handing it this carve would misread every byte.
        assert_eq!(row_schema_of(0x0306), None);
        // outside the ontology domain entirely
        assert_eq!(row_schema_of(0x0A01), None);
        assert_eq!(row_schema_of(0x0000), None);
    }

    #[test]
    fn the_declared_schema_is_the_carve_the_writers_use() {
        // The literals the bake writes through are DERIVED from this schema —
        // if someone re-introduces an independent literal, this pins the drift.
        assert_eq!(
            crate::ENTITY_TYPE_SLAB_OFFSET,
            OBO_CORE_ROW.entity_type.slab_off
        );
        assert_eq!(
            crate::edges::EDGE_LANE_SLAB_OFFSET,
            OBO_CORE_ROW.edge_lanes.slab_off
        );
        assert_eq!(crate::edges::EDGE_LANE_COUNT, OBO_CORE_ROW.lane_count);
        assert_eq!(
            crate::edges::EDGE_LANE_COUNT * crate::edges::LANE_BYTES,
            OBO_CORE_ROW.edge_lanes.bytes
        );
    }
}
