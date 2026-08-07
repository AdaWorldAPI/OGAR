//! `field_view` — the **addressed desktop surface** (charter C1.2 of
//! `AdaWorldAPI/OGAR docs/A2UI-SCREEN-ADDRESSING-PROPOSAL.md`).
//!
//! This is the render half of *"don't push pixels — address the screen."*
//! Where [`rust_class`](crate::rust_class) emits a masked class as Rust
//! **source** (build-time codegen) and [`html_detail_view`](crate::artifact_kinds::html_detail_view)
//! emits a record's detail page from `RenderColumn`/`CellSource`, this module
//! renders a **`WideFieldMask`-projected [`ClassView`] instance** as a live,
//! addressed screen — the operator's *"ERB pattern fieldview ClassView
//! rendering from memory without serialization."*
//!
//! # What "addressed" means (the fieldview contract)
//!
//! - **Every field carries its POSITION** (`data-field-pos`) — the field's
//!   [`WideFieldMask`](lance_graph_contract::class_view::WideFieldMask) bit
//!   index, which IS its layout address (the ordered-set position the nested
//!   ClassView projects; the same `X:Y` rail documents and Klickwege surfaces
//!   use). The client repaints exactly the positions a `NodeDelta` mask
//!   carries — nothing else redraws.
//! - **Every action carries its ORDINAL only** (`data-action-ordinal`,
//!   charter trap T2) — the index into the class's `ActionDef` set. A click
//!   emits `ActionInvoke { key, action_ordinal, args }`; the surface carries
//!   the *address* of behavior, never behavior. `onClick: <lambda>` in a
//!   component tree is the `DEFINE EVENT` hijack, and it is rejected by
//!   construction here — there is nowhere to put a handler.
//! - **The key is the node address** (`data-key`) — the canonical 16-byte
//!   GUID as hex. The client prerenders layout from the key alone (OGAR P0
//!   "the key prerenders nodes"); the fields fill in the addressed slots.
//!
//! # Where the values come from (ClassView, not the wire)
//!
//! [`FieldView`]s are built from a [`ClassView`] projection — see
//! [`from_value_rows`], which maps
//! [`ClassView::facet_rows`](lance_graph_contract::class_view::ClassView::facet_rows)
//! output straight onto the surface. The labels are the meta-DTO's *late*
//! resolution (above the SoA); the value bytes are the row's V3 facet. The
//! render borrows both — nothing serializes in order to draw (charter T3).
//!
//! # XSS
//!
//! The template compiles with `escape = "html"`, and **every** interpolated
//! field (title, labels, values, action captions) is data-derived, so all of
//! it is HTML-escaped. There is no `|safe` escape hatch on this surface — a
//! fieldview never carries pre-rendered HTML — which closes the class of
//! injection the list/detail kits had to fix under codex P1 on #83/#84.

use askama::Template;

use lance_graph_contract::class_view::{RenderRow, ValueRow};

/// One projected field on the addressed surface.
///
/// `position` is the field's [`WideFieldMask`](lance_graph_contract::class_view::WideFieldMask)
/// bit index — its **layout address** on the screen. `value` is the
/// already-formatted display text (the consumer's ClassView formats the raw
/// facet byte / value-slab field into text); the template escapes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldView {
    /// The field's mask position — its layout address (the ordered-set slot).
    pub position: u8,
    /// The display label, late-resolved from the ontology cache (above the SoA).
    pub label: String,
    /// The field's predicate IRI (the stable key behind the label).
    pub predicate: String,
    /// The formatted value text — escaped by the template.
    pub value: String,
}

/// One action the surface exposes — carried by **address only** (charter T2).
///
/// `ordinal` is the index into the class's `ActionDef` set; a click emits
/// `ActionInvoke { key, action_ordinal: ordinal, args }`. There is no handler
/// field — behavior lives on the Core node, never on the surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRef {
    /// Index into the class's `ActionDef` set — the invocation address.
    pub ordinal: u32,
    /// The human caption for the control (escaped by the template).
    pub label: String,
}

// ── askama binding struct ────────────────────────────────────────────

#[derive(Template)]
#[template(path = "dispatch/field_view.askama", escape = "html")]
struct FieldViewCtx<'a> {
    class_id_hex: String,
    canonical_concept: &'a str,
    key_hex: &'a str,
    title: &'a str,
    fields: &'a [FieldView],
    actions: &'a [ActionRef],
}

/// Render a `WideFieldMask`-projected [`ClassView`](lance_graph_contract::class_view::ClassView)
/// instance as an **addressed desktop surface** (the fieldview).
///
/// - `class_id` / `canonical_concept` identify the class (the client resolves
///   the template codebook from `class_id` — the font of the desktop).
/// - `key_hex` is the node's canonical 16-byte GUID as hex — the node address.
/// - `fields` are the projected, present fields (build them with
///   [`from_value_rows`] / [`from_render_rows`] from a ClassView projection).
/// - `actions` are the `ActionDef` addresses the surface exposes (T2).
///
/// # Errors
///
/// Propagates [`askama::Error`] if template rendering fails (it never should
/// for well-formed input — the template has no fallible expressions).
pub fn render_field_view(
    class_id: u16,
    canonical_concept: &str,
    key_hex: &str,
    title: &str,
    fields: &[FieldView],
    actions: &[ActionRef],
) -> Result<String, askama::Error> {
    FieldViewCtx {
        class_id_hex: format!("0x{class_id:04X}"),
        canonical_concept,
        key_hex,
        title,
        fields,
        actions,
    }
    .render()
}

/// Build the surface's [`FieldView`]s from a ClassView's
/// [`facet_rows`](lance_graph_contract::class_view::ClassView::facet_rows)
/// output — the value-projected rows (each present facet-backed field paired
/// with its byte).
///
/// The raw facet byte is formatted with `fmt` (the consumer supplies the
/// class's display formatting — e.g. an enum label, a scaled number); the
/// default caller can pass `|b| b.to_string()`. This is the direct
/// ClassView-facet → addressed-surface bridge: the mask already gated which
/// rows exist (C2 presence, above the SoA), and each row already knows its
/// `position` (layout address) — nothing about presence or address is
/// recomputed here.
pub fn from_value_rows(rows: &[ValueRow<'_>], mut fmt: impl FnMut(u8) -> String) -> Vec<FieldView> {
    rows.iter()
        .map(|r| FieldView {
            position: r.position,
            label: r.label.to_string(),
            predicate: r.predicate.to_string(),
            value: fmt(r.value),
        })
        .collect()
}

/// Build the surface's [`FieldView`]s from a ClassView's
/// [`render_rows`](lance_graph_contract::class_view::ClassView::render_rows)
/// output — the label-only rows (no facet byte), paired positionally with the
/// consumer's already-formatted value strings.
///
/// `render_rows` yields `(label, predicate)` for each present field but not
/// the field's mask position (it filters the projection down to present rows),
/// so the caller supplies `positions` (the present bit indices, ascending) and
/// `values` (one formatted value per present row) alongside. All three slices
/// are index-aligned; the shortest wins (a length mismatch truncates rather
/// than panics — the caller is expected to pass matched slices).
pub fn from_render_rows(
    rows: &[RenderRow<'_>],
    positions: &[u8],
    values: &[String],
) -> Vec<FieldView> {
    rows.iter()
        .zip(positions.iter())
        .zip(values.iter())
        .map(|((r, &position), value)| FieldView {
            position,
            label: r.label.to_string(),
            predicate: r.predicate.to_string(),
            value: value.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fields() -> Vec<FieldView> {
        vec![
            FieldView {
                position: 0,
                label: "Total".to_string(),
                predicate: "amount_total".to_string(),
                value: "1200".to_string(),
            },
            FieldView {
                // A position past 63 — the wide-surface case (charter C1.4):
                // the fieldview addresses fields beyond the 64-field ceiling.
                position: 133,
                label: "Partner".to_string(),
                predicate: "partner_id".to_string(),
                value: "ACME GmbH".to_string(),
            },
        ]
    }

    #[test]
    fn renders_addressed_fields_and_action_ordinals() {
        let actions = vec![ActionRef {
            ordinal: 7,
            label: "Post".to_string(),
        }];
        let src = render_field_view(
            0x0102,
            "commercial_document",
            "0801000301020304",
            "Invoice #42",
            &sample_fields(),
            &actions,
        )
        .unwrap();

        // The surface is addressed by class + concept + key.
        assert!(src.contains("data-class-id=\"0x0102\""), "{src}");
        assert!(src.contains("data-concept=\"commercial_document\""), "{src}");
        assert!(src.contains("data-key=\"0801000301020304\""), "{src}");
        // Each field carries its POSITION (layout address) — including the
        // wide position past 63.
        assert!(src.contains("data-field-pos=\"0\""), "{src}");
        assert!(
            src.contains("data-field-pos=\"133\""),
            "wide position must address past the 64 ceiling:\n{src}"
        );
        assert!(src.contains("Total"), "{src}");
        assert!(src.contains("ACME GmbH"), "{src}");
        // The action carries its ORDINAL address (T2) and the node key — never
        // an inline handler.
        assert!(
            src.contains("data-action-ordinal=\"7\""),
            "action must carry its ordinal address:\n{src}"
        );
        assert!(!src.contains("onclick"), "no inline handler (T2):\n{src}");
    }

    #[test]
    fn escapes_data_derived_strings_xss_regression() {
        // Every fieldview string is data-derived and MUST be escaped — the
        // fieldview has no `|safe` hatch (unlike the list/detail kits, which
        // needed codex-P1 fixes on #83/#84). A poisoned label/value/title/
        // action-caption must not inject raw HTML.
        let fields = vec![FieldView {
            position: 0,
            label: "<script>alert('label')</script>".to_string(),
            predicate: "x".to_string(),
            value: "<img src=x onerror=alert('value')>".to_string(),
        }];
        let actions = vec![ActionRef {
            ordinal: 0,
            label: "<script>alert('action')</script>".to_string(),
        }];
        let src = render_field_view(
            0x0102,
            "<concept-xss>",
            "<key-xss>",
            "<title-xss>x</title-xss>",
            &fields,
            &actions,
        )
        .unwrap();

        assert!(!src.contains("<script>alert('label')"), "label raw:\n{src}");
        assert!(!src.contains("<img src=x onerror"), "value raw:\n{src}");
        assert!(!src.contains("<script>alert('action')"), "action raw:\n{src}");
        assert!(!src.contains("<title-xss>"), "title raw:\n{src}");
        assert!(!src.contains("<concept-xss>"), "concept raw:\n{src}");
        // The escaped form is present (quote encoding varies by askama
        // version, so assert only the angle-bracket escaping).
        assert!(src.contains("&lt;script&gt;"), "{src}");
    }

    #[test]
    fn empty_actions_omit_the_nav() {
        let src =
            render_field_view(1, "c", "00", "T", &sample_fields(), &[]).unwrap();
        assert!(
            !src.contains("fieldview-actions"),
            "no action nav when there are no actions:\n{src}"
        );
    }

    #[test]
    fn from_value_rows_maps_classview_facet_projection() {
        // ValueRow is what ClassView::facet_rows yields — position + label +
        // predicate + the raw facet byte. from_value_rows maps it straight
        // onto the addressed surface, formatting the byte.
        let rows = vec![
            ValueRow {
                label: "Status",
                predicate: "state",
                position: 3,
                value: 2,
            },
            ValueRow {
                label: "Priority",
                predicate: "priority",
                position: 5,
                value: 9,
            },
        ];
        let fields = from_value_rows(&rows, |b| format!("code:{b}"));
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].position, 3);
        assert_eq!(fields[0].label, "Status");
        assert_eq!(fields[0].value, "code:2");
        assert_eq!(fields[1].position, 5);
        assert_eq!(fields[1].value, "code:9");

        // And it renders through the surface with those addresses.
        let src = render_field_view(0x0102, "c", "00", "T", &fields, &[]).unwrap();
        assert!(src.contains("data-field-pos=\"3\""), "{src}");
        assert!(src.contains("data-field-pos=\"5\""), "{src}");
        assert!(src.contains("code:2"), "{src}");
    }

    #[test]
    fn from_render_rows_pairs_positions_and_values() {
        let rows = vec![
            RenderRow {
                label: "Total",
                predicate: "amount_total",
            },
            RenderRow {
                label: "Partner",
                predicate: "partner_id",
            },
        ];
        let positions = [0u8, 133u8];
        let values = ["1200".to_string(), "ACME".to_string()];
        let fields = from_render_rows(&rows, &positions, &values);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[1].position, 133);
        assert_eq!(fields[1].label, "Partner");
        assert_eq!(fields[1].value, "ACME");
    }
}
