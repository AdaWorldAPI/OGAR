//! `project` — the **ClassView × WideFieldMask projection** of the observation IR.
//!
//! # The arrow points OUT
//!
//! This module is how a [`DocIr`](crate::DocIr) becomes *addressable* by a
//! class's field basis. It depends on ONLY the zero-dep masking contract
//! (`lance_graph_contract::class_view`: [`ClassView`], [`WideFieldMask`]) and
//! hands out two things:
//!
//! - a **presence mask** ([`field_mask`]) — bit `i` set iff the document carries
//!   the `i`-th field of the class;
//! - the **values addressed through that mask** ([`masked_values`]) — each
//!   present field paired with the typed string read from the in-memory `DocIr`.
//!
//! A *consumer* (e.g. `lance-graph-arm-discovery`) turns those into its own
//! `Dataset` / `FeatureSpec`: **presence is the mask bit** (a binary feature, no
//! `∅` sentinel invented anywhere), and **categories are the addressed typed
//! values** (the consumer interns them). The neutral IR NEVER names a consumer —
//! it does not depend on the arm crate, does not build a `Dataset`, and does not
//! serialize. This is `ClassView` addressing as **masking over existing in-memory
//! data**, not a wire format (the Firewall, ADR-022/023).
//!
//! # Why a mask and not a re-typed struct
//!
//! The `DocIr` is untouched: `field_mask` reads `ir.fields` and
//! `view.fields(class)` and produces a [`WideFieldMask`] whose bit `i` is the
//! stable `ClassView` field position `i` (N3 append-only). Nothing is copied,
//! parsed, or reshaped — the projection is a *view*, addressed by the class.

use lance_graph_contract::class_view::{ClassId, ClassView, WideFieldMask};

use crate::DocIr;

/// A class field the document carries, with its typed value read **through the
/// mask** — no copy, no serde.
///
/// `position` is the stable [`WideFieldMask`] bit (= the field's index in
/// [`ClassView::fields`]); `value` borrows the `DocIr`'s
/// [`TypedField`](crate::TypedField) string in place. A consumer interns
/// `value` into its own category codebook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskedField<'a> {
    /// The class field position — the `WideFieldMask` bit this field occupies.
    pub position: u8,
    /// The typed value, borrowed from the in-memory `DocIr` (never serialized).
    pub value: &'a str,
}

/// The position of a `DocIr` field `key` within a class's ordered field set, or
/// `None` if the class does not declare it.
///
/// Matches the `FieldRef` by `label` first, then `predicate_iri` — the two names
/// a `ClassView` field carries. A key the class does not know is **not present**
/// (its bit is never set) — the exact discipline of the doc-IR load gate: an
/// off-vocabulary field is dropped at the addressing boundary, never folded onto
/// a valid bit. Positions `>= 256` are unaddressable in a `u8` mask and return
/// `None`.
#[must_use]
pub fn position_of<V: ClassView>(view: &V, class: ClassId, key: &str) -> Option<u8> {
    view.fields(class)
        .iter()
        .position(|f| f.label == key || f.predicate_iri == key)
        .and_then(|p| u8::try_from(p).ok())
}

/// The **presence mask** — bit `i` set iff the document carries the `i`-th field
/// of `class`.
///
/// This IS the `ClassView × WideFieldMask` masking over the in-memory `DocIr`:
/// it reads `ir.fields` and `view.fields(class)`, and nothing else. A consumer
/// reads bit `i` as the binary presence feature for field `i` — **presence is
/// the mask bit**, never a category value invented at a boundary. Zero
/// serialization.
#[must_use]
pub fn field_mask<V: ClassView>(ir: &DocIr, view: &V, class: ClassId) -> WideFieldMask {
    let mut mask = WideFieldMask::EMPTY;
    for tf in &ir.fields {
        if let Some(pos) = position_of(view, class, &tf.key) {
            mask = mask.with(pos);
        }
    }
    mask
}

/// The **values addressed through the mask** — one [`MaskedField`] per class
/// field the document carries, in class (bit) order.
///
/// Presence is the mask; the value is the typed string read in place. A consumer
/// interns these into its own arm `Dataset` categories; this crate neither
/// interns nor serializes. When a document carries two typed fields that resolve
/// to the same class position, the **first wins** (document order) — the mask is
/// presence, not a multiset — so the returned rows are exactly the set bits of
/// [`field_mask`], addressed.
#[must_use]
pub fn masked_values<'a, V: ClassView>(
    ir: &'a DocIr,
    view: &V,
    class: ClassId,
) -> Vec<MaskedField<'a>> {
    let n = view.field_count(class);
    let mut slots: Vec<Option<MaskedField<'a>>> = vec![None; n];
    for tf in &ir.fields {
        if let Some(pos) = position_of(view, class, &tf.key) {
            let idx = pos as usize;
            if idx < n && slots[idx].is_none() {
                slots[idx] = Some(MaskedField {
                    position: pos,
                    value: tf.value.as_str(),
                });
            }
        }
    }
    slots.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BBoxRail, DOC_IR_VERSION, DocIr, Geometry, Provenance, Rail, TypedField};
    use lance_graph_contract::ontology::{DisplayTemplate, FieldRef};

    /// A minimal in-crate `ClassView` — a fixed ordered field set per class.
    /// Stands in for the W4 OGIT-backed `OgarClassView`; the projection is
    /// generic over `ClassView`, so a test impl proves the masking without a
    /// canon dependency.
    struct TestView {
        fields: Vec<FieldRef>,
    }

    impl TestView {
        /// An "invoice" class: netto, ust, iban — the field basis (bit order).
        fn invoice() -> Self {
            Self {
                fields: vec![
                    FieldRef::new("inv:netto", "netto"),
                    FieldRef::new("inv:ust", "ust"),
                    FieldRef::new("inv:iban", "iban"),
                ],
            }
        }
    }

    impl ClassView for TestView {
        fn fields(&self, _class: ClassId) -> &[FieldRef] {
            &self.fields
        }
        fn template(&self, _class: ClassId) -> DisplayTemplate {
            DisplayTemplate::Card
        }
        fn dolce_category_id(&self, _class: ClassId) -> u8 {
            0
        }
    }

    fn bbox() -> BBoxRail {
        BBoxRail {
            tl: Rail { x: 0, y: 0 },
            br: Rail { x: 1, y: 1 },
        }
    }

    fn field(key: &str, value: &str) -> TypedField {
        TypedField {
            key: key.to_string(),
            value: value.to_string(),
            bbox: bbox(),
            confidence: 200,
        }
    }

    /// A doc carrying netto + iban but NOT ust, plus an off-vocabulary field.
    fn doc(fields: Vec<TypedField>) -> DocIr {
        DocIr {
            version: DOC_IR_VERSION.to_string(),
            source: Provenance::Ocr,
            geometry: Geometry::Rendered,
            content_sha256: [0u8; 32],
            mime: "image/png".to_string(),
            pages: vec![],
            fields,
        }
    }

    #[test]
    fn mask_sets_exactly_the_carried_field_bits() {
        let view = TestView::invoice();
        let ir = doc(vec![field("netto", "100.00"), field("iban", "DE00")]);
        let mask = field_mask(&ir, &view, 0);
        assert!(mask.has(0), "netto (position 0) present");
        assert!(
            !mask.has(1),
            "ust (position 1) absent — bit clear, not a category"
        );
        assert!(mask.has(2), "iban (position 2) present");
        assert_eq!(mask.count(), 2);
    }

    #[test]
    fn off_vocabulary_field_is_dropped_at_the_boundary() {
        let view = TestView::invoice();
        // `skonto` is not a class field — it must not set any bit.
        let ir = doc(vec![field("netto", "100.00"), field("skonto", "2%")]);
        let mask = field_mask(&ir, &view, 0);
        assert_eq!(mask.count(), 1, "only netto addressed; skonto dropped");
        assert!(mask.has(0));
    }

    #[test]
    fn matches_by_predicate_iri_too() {
        let view = TestView::invoice();
        let ir = doc(vec![field("inv:ust", "19")]); // key = predicate_iri, not label
        let mask = field_mask(&ir, &view, 0);
        assert!(mask.has(1), "ust addressed via predicate_iri");
    }

    #[test]
    fn masked_values_are_the_typed_values_in_bit_order() {
        let view = TestView::invoice();
        let ir = doc(vec![field("iban", "DE00"), field("netto", "100.00")]);
        let rows = masked_values(&ir, &view, 0);
        // Bit order (0=netto, 2=iban), regardless of document order.
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            MaskedField {
                position: 0,
                value: "100.00"
            }
        );
        assert_eq!(
            rows[1],
            MaskedField {
                position: 2,
                value: "DE00"
            }
        );
    }

    #[test]
    fn masked_values_agree_with_the_mask() {
        let view = TestView::invoice();
        let ir = doc(vec![field("netto", "100.00"), field("iban", "DE00")]);
        let mask = field_mask(&ir, &view, 0);
        let rows = masked_values(&ir, &view, 0);
        assert_eq!(rows.len(), mask.count() as usize);
        for r in &rows {
            assert!(mask.has(r.position), "every addressed value is a set bit");
        }
    }

    #[test]
    fn duplicate_field_first_wins() {
        let view = TestView::invoice();
        let ir = doc(vec![field("netto", "100.00"), field("netto", "999.99")]);
        let rows = masked_values(&ir, &view, 0);
        assert_eq!(rows.len(), 1, "mask is presence, not a multiset");
        assert_eq!(rows[0].value, "100.00", "first document occurrence wins");
    }
}
