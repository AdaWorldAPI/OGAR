//! The canonical **loose-ends-as-DTO** residual — the general "slag" shape.
//!
//! [`crate::capability_registry::UnmintedRow`] is the *capability-minting*
//! slag row: a concept the codebook could not mint. This is its **general**
//! sibling — any surface (a schema column a codegen recipe never binds, an
//! unhandled read, an unmapped case) that a transcode/codegen pass could not
//! confidently resolve, captured as a typed DTO instead of being silently
//! dropped or degraded to an id-0 sentinel.
//!
//! It is the code home of the MedCare transcode doctrine's
//! `ResidualRepresentation { surface, storage, handler, unresolved_reason }`
//! ("study the slag, teach the furnace"): each recurring residual names the
//! next config fact — a mint, an alias, a recipe, a convention fix. First
//! consumer: `medcare-analytics`'s schema-vs-recipe coverage sweep, which
//! emits one residual per column no read or write recipe covers.
//!
//! Contract-appropriate by construction: a plain typed value both producer and
//! consumer build against (like [`crate::capability_registry::UnmintedRow`]);
//! nothing here serializes to leave a boundary.

use crate::capability_registry::UnmintedRow;

/// A surface a transcode/codegen pass could not confidently resolve, kept as a
/// typed loose end — never a silent drop, never an id-0 degrade.
///
/// The four fields answer WHERE / HOW-STORED / WHAT-HANDLES-IT / WHY-UNRESOLVED,
/// which is enough to name the next config fact that would resolve it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResidualRepresentation {
    /// WHERE the loose end is — an addressable surface (e.g. `table.column`,
    /// a route, a concept path).
    pub surface: String,
    /// HOW it is stored / typed, verbatim from the source (e.g. `float`,
    /// `tinyint(1)`, a Rust type name).
    pub storage: String,
    /// WHAT partially handles it, if anything. `None` for a fully-unhandled
    /// residual; `Some` when a pass covers it partially and the residual
    /// records the gap.
    pub handler: Option<String>,
    /// WHY it is unresolved — human-readable, stable enough to assert on and
    /// to name the next config fact.
    pub unresolved_reason: String,
}

impl ResidualRepresentation {
    /// Construct a residual. `surface` / `storage` / `unresolved_reason` accept
    /// anything `Into<String>`; `handler` is passed as-is (`None` for a fully
    /// unhandled loose end).
    #[must_use]
    pub fn new(
        surface: impl Into<String>,
        storage: impl Into<String>,
        handler: Option<String>,
        unresolved_reason: impl Into<String>,
    ) -> Self {
        Self {
            surface: surface.into(),
            storage: storage.into(),
            handler,
            unresolved_reason: unresolved_reason.into(),
        }
    }
}

impl From<UnmintedRow> for ResidualRepresentation {
    /// An unminted capability row IS a residual in general form: the failing
    /// `concept` is the surface, the `object_class` its storage, the
    /// `capability` the (attempted) handler. This is the bridge that shows the
    /// two slag types are one doctrine family, not two ideas.
    fn from(u: UnmintedRow) -> Self {
        Self {
            surface: u.concept,
            storage: u.object_class,
            handler: Some(u.capability),
            unresolved_reason: "subject concept not minted in the codebook".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_builds_the_four_fields() {
        let r = ResidualRepresentation::new(
            "pf_vital_dynmodule.v1",
            "float",
            None,
            "no write recipe binds this column",
        );
        assert_eq!(r.surface, "pf_vital_dynmodule.v1");
        assert_eq!(r.storage, "float");
        assert_eq!(r.handler, None);
        assert!(r.unresolved_reason.contains("no write recipe"));
    }

    #[test]
    fn unminted_row_bridges_to_a_residual() {
        let u = UnmintedRow {
            capability: "recognize_line".to_string(),
            object_class: "ogit-ocr/textline".to_string(),
            concept: "textline".to_string(),
        };
        let r: ResidualRepresentation = u.into();
        assert_eq!(r.surface, "textline");
        assert_eq!(r.storage, "ogit-ocr/textline");
        assert_eq!(r.handler.as_deref(), Some("recognize_line"));
        assert!(r.unresolved_reason.contains("not minted"));
    }
}
