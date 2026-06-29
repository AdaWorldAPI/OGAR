# ClassView field-view via Askama + a field bitmask

> **Consumer-side rendering pattern.** Render an OGAR `ClassView` as an HTML
> field view the way Redmine renders a model's fields with ERB — but **compiled,
> type-safe, JSON-free, and driven by a field bitmask instead of per-field
> conditionals**. This is the *render-side twin* of §1.5 ("the spine is the
> COMPILED ClassView"): the read-mask that selects which facets to decode and the
> render-mask that selects which fields to show are the **same bits**.

## The problem

A `ClassView` has a field set. Different contexts want different *subsets* of it:
a `V1` vs `V2` facet layout, an RBAC-restricted view, a compact card vs a full
record, a tenant projection. Redmine solves this with ERB: a generic field
partial loops a **column list** (`available_columns` filtered to the selected
`column_names`) and renders each. The visibility lives in the *data*, not the
template — there is no `<% if show_dose %>` per field.

The naive port to a *compile-time* engine (Askama) reaches for per-field
conditionals — `{% if self.shows(Dose) %}…{% endif %}` — and that is **wrong**:
it is `if`-noise, it does not scale to wide classes, and it is not how Redmine
does it. A compiled template looks like it forbids dynamic field sets. It does
not. The fix is to move selection out of the template and **into an iterated,
mask-filtered list** — one loop, zero conditionals.

## The shape (no `if` noise)

Selection is data. The template is a dumb iterator over the **already-filtered**
field list; a `u64` mask decides membership in Rust.

```rust
// FieldDesc[] and the bit indices are GENERATED from the ClassView schema
// (derive macro or build.rs) — never hand-written next to hand-numbered bits.
struct FieldDesc { idx: u8, label: &'static str, kind: FieldKind }

struct ClassFieldView<'a> {
    fields: &'a [FieldDesc],   // the maximal, ordered field set for this class
    mask:   u64,               // which fields are SELECTED (1 << idx)
    rec:    &'a Record,        // the SoA-backed values
}

impl ClassFieldView<'_> {
    fn selected(&self)   -> impl Iterator<Item = &FieldDesc> {
        self.fields.iter().filter(move |f| self.mask & (1 << f.idx) != 0)
    }
    fn unselected(&self) -> impl Iterator<Item = &FieldDesc> {
        self.fields.iter().filter(move |f| self.mask & (1 << f.idx) == 0)
    }
    fn value(&self, f: &FieldDesc) -> Cell<'_> { self.rec.cell(f.idx) }
}
```

```jinja
{# the ERB-shaped field partial — ONE loop, zero ifs #}
{% for f in self.selected() %}
  <tr><th>{{ f.label }}</th><td>{{ self.value(f)|fmt }}</td></tr>
{% endfor %}
```

The bitmask **is** the selected / unselected partition:

- **bit set** → the field is in the loop → rendered in the view.
- **bit clear** → the field is out → available but hidden. `unselected()` is
  exactly Redmine's "available columns" palette for a column chooser.

Versions, roles, and projections are simply **different masks over the same
template**. There is no template per version — one compiled artifact, any subset.

## Why this is right (not just convenient)

1. **§1.5 alignment — render-mask = read-mask.** "One compiled reader subsumes
   V1/V2/V3 — no hardcoded facet versions." The render mask is the *same selector*
   the ClassView reader uses to choose which facets to decode. V1 is one bit
   pattern, V2 another, a role view another; all over one template.
2. **Mirror of columnar projection.** Arrow / lance-graph prune *columns* with a
   selection mask; this prunes *view fields* with a selection mask. SoA + a
   selection mask = a dynamic projection — same shape at the data layer and the
   view layer. The "schema glove" over the SoA columns *is* the mask.
3. **JSON-free.** `Record` struct → Askama → HTML, entirely in Rust. The field
   views never touch a serializer or a JS frontend. htmx-friendly without React.
4. **No `if` noise.** The template never branches per field; it iterates a
   pre-filtered set. Wide classes stay readable; the structure is compile-checked.
5. **Auto-escaping by default** → XSS-safe, which matters on a clinical/PII
   surface (and the no-German-PII rule). Baked into the binary → no runtime parse,
   no template-file I/O, no path traversal.

## The one discipline (iron rule)

**`field ↔ idx ↔ bit` must come from ONE generated source.** Generate the
`FieldDesc[]` table *and* the bit indices from the ClassView definition (derive
macro or `build.rs`) so the mask indexes the **same ordered field set** the loop
renders. Never hand-number bits next to a hand-written list — that is precisely
where bit 17 silently starts meaning a different field across a version bump.
This is `I-LEGACY-API-FEATURE-GATED`: a bitmask over a layout is exactly where
bits alias. The read-mask and the render-mask **share** the generated constants.

Corollary discipline: **mask logic lives in Rust, the template is dumb.** Compute
*which* bits are set (role × version × projection) in Rust; hand the template a
struct that only answers `selected()` / `unselected()` / `value(f)`. Conditionals
gate presence (and here there are none); filters (`|fmt`, `|escape`) shape the
shown value. The ClassView is the data, the mask is the selector, the template is
the skin.

## The residual (what the mask cannot do)

The mask selects **presence over the known maximal field set** — facet, version,
role, projection. That is the entire common case. The one thing it *cannot* do is
render a field that was **never compiled** (a genuinely novel field from a
runtime-imported ontology). That residual has two blessed routes, in order:

1. a **generic `Vec<(name, value)>` view** + one Askama template that loops the
   tuples — covers the whole dynamic tail with one compiled template; or
2. **`build.rs` codegen** that emits a `FieldDesc[]` (and template, if bespoke)
   per class from the ontology manifest — compile-time codegen, which the stack
   explicitly blesses ("build-time serde codegen *is* compile types").

Reach for a *runtime* template engine (MiniJinja/Tera) only for genuinely
user-authored, per-tenant templates — never as the default.

## Summary

One generic Askama field partial + a generated `FieldDesc[]` table + a `u64`
mask = the whole dynamic ClassView field view: Redmine-shaped, JSON-free, no
conditionals in the template, type-checked structure, dynamic across
versions/roles/projections. **The mask carves; the loop renders.** Askama's
compile-time nature is not a cage — the mask is the runtime knob, and it is the
same selector the data layer already uses to prune columns.

## Cross-references

- §1.5 "the spine is the COMPILED ClassView" — the read-side twin (the render
  mask = the facet read selector).
- `docs/OGAR-AS-IR.md` — ClassView as a compiler IR surface.
- `docs/OGAR-CONSUMER-BEST-PRACTICES.md` — the consumer muscle-memory guide this
  pattern slots into.
- `I-LEGACY-API-FEATURE-GATED` — why the mask bits must be generated, never
  hand-numbered alongside a layout.
