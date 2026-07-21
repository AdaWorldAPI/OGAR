//! `ogar-render-typst` — the **paged projection**: emit Typst SOURCE from the
//! same [`FieldView`] rows the askama HTML surface renders.
//!
//! # One surface, two renderers
//!
//! The askama crate renders a projected `ClassView × WideFieldMask` instance as
//! a *living screen* (`render_field_view`, `@live`). This crate renders the
//! SAME rows as a *page* — the record projection: signed reports, invoices,
//! planning baselines, the `@revision:N`-pinned documents of the composition
//! layer's `ResolutionMode`. Nothing here queries a domain store; the rows
//! arrive already resolved (the renderer-independence hinge).
//!
//! # Source emission only — deliberately
//!
//! This crate emits `.typ` TEXT and stops. Emitting markup source is render
//! egress exactly like askama emitting HTML text; embedding the typst
//! *compiler* (fonts, layout, a heavy dep tree) is a separate, feature-gated
//! decision for a consumer that wants in-binary PDF. Keeping the emitter
//! text-only keeps this crate as light as the surface it mirrors.

use ogar_render_askama::FieldView;

/// Escape a text run for safe interpolation into Typst markup.
///
/// Typst's active characters in markup context are escaped with a backslash —
/// the set below covers the markup-significant ASCII characters (Typst accepts
/// a backslash escape before any of them). Newlines are preserved.
///
/// Additionally, Typst recognizes `=` (heading), `-` (list item), and `+`
/// (enum item) as BLOCK markers at the start of a line: user data beginning a
/// line with one of them would otherwise become a heading/list in the emitted
/// page (codex P2 on #222). They are escaped in line-start position only —
/// mid-line they are inert and stay readable. The escape is conservative at
/// the start of the string too: `\=` renders identically to `=` when the
/// context turns out to be mid-line.
#[must_use]
pub fn escape_typst(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut at_line_start = true;
    for c in s.chars() {
        match c {
            // Markup-active characters in Typst content mode.
            '\\' | '#' | '*' | '_' | '`' | '$' | '<' | '>' | '@' | '[' | ']' | '/' | '~' | '\''
            | '"' => {
                out.push('\\');
                out.push(c);
                at_line_start = false;
            }
            // Line-start block markers: heading / list / enum.
            '=' | '-' | '+' if at_line_start => {
                out.push('\\');
                out.push(c);
                at_line_start = false;
            }
            '\n' => {
                out.push(c);
                at_line_start = true;
            }
            _ => {
                out.push(c);
                at_line_start = false;
            }
        }
    }
    out
}

/// Emit one resolved field-view as a Typst fragment: a bold caption line and a
/// two-column table of `(label, value)` rows — the paged sibling of the askama
/// `<dl class="fieldview-fields">` surface. `class_view` is stamped as a small
/// caption suffix so the page carries its projection provenance (which named
/// view produced it), mirroring the HTML surface's `data-*` addressing.
#[must_use]
pub fn emit_field_view(class_view: &str, title: &str, fields: &[FieldView]) -> String {
    let mut out = String::new();
    out.push_str("#block[\n");
    out.push_str(&format!(
        "*{}* #text(size: 0.8em)[({})]\n",
        escape_typst(title),
        escape_typst(class_view)
    ));
    out.push_str("#table(\n  columns: 2,\n");
    for f in fields {
        out.push_str(&format!(
            "  [{}], [{}],\n",
            escape_typst(&f.label),
            escape_typst(&f.value)
        ));
    }
    out.push_str(")\n]\n");
    out
}

/// Emit an explicit unresolved-slot marker — the paged form of the ActionText
/// missing-object fallback: the snapshot's content address is shown, never
/// silently dropped.
#[must_use]
pub fn emit_fallback(class_view: &str, content_sha256_hex: &str) -> String {
    format!(
        "#block[\n_unresolved {}_ — snapshot #raw(\"sha256:{}\")\n]\n",
        escape_typst(class_view),
        escape_typst(content_sha256_hex)
    )
}

/// Emit a section heading.
#[must_use]
pub fn emit_heading(text: &str) -> String {
    format!("= {}\n", escape_typst(text))
}

/// Emit a plain text run as a paragraph line.
#[must_use]
pub fn emit_text(text: &str) -> String {
    format!("{}\n", escape_typst(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fv(position: u8, label: &str, value: &str) -> FieldView {
        FieldView {
            position,
            label: label.into(),
            predicate: String::new(),
            value: value.into(),
        }
    }

    #[test]
    fn field_view_fragment_carries_labels_values_and_provenance() {
        let t = emit_field_view(
            "user.card",
            "Alice",
            &[fv(0, "name", "Alice"), fv(2, "role", "engineer")],
        );
        assert!(t.contains("*Alice*"));
        assert!(t.contains("(user.card)"));
        assert!(t.contains("[name], [Alice],"));
        assert!(t.contains("[role], [engineer],"));
    }

    #[test]
    fn markup_active_characters_are_escaped() {
        let t = emit_field_view("v", "T", &[fv(0, "amount", "#100 *net* [EUR] @q3")]);
        assert!(t.contains(r"\#100 \*net\* \[EUR\] \@q3"), "got: {t}");
        // and the structural table brackets are still intact
        assert!(t.contains("#table("));
    }

    #[test]
    fn fallback_marker_is_explicit() {
        let t = emit_fallback("work_package.inline", &"ab".repeat(32));
        assert!(t.contains("unresolved"));
        assert!(t.contains("sha256:abab"));
    }

    #[test]
    fn heading_and_text_escape() {
        assert_eq!(emit_heading("A & B"), "= A & B\n");
        assert!(emit_text("cost: $5").contains(r"\$5"));
    }

    /// Falsifier (codex P2 on #222): user data starting a line with a Typst
    /// block marker must not become a heading/list/enum in the emitted page.
    #[test]
    fn line_start_block_markers_are_escaped() {
        // Start of string.
        assert_eq!(emit_text("= fake heading"), "\\= fake heading\n");
        // After an embedded newline — each line start is guarded.
        assert_eq!(emit_text("- item\n+ item"), "\\- item\n\\+ item\n");
        // Mid-line the same characters are inert and stay unescaped.
        assert_eq!(emit_text("a - b = c + d"), "a - b = c + d\n");
        // A multi-line value inside a field view cannot smuggle a heading in.
        let t = emit_field_view("v", "T", &[fv(0, "note", "x\n= injected")]);
        assert!(t.contains("x\n\\= injected"), "got: {t}");
    }
}
