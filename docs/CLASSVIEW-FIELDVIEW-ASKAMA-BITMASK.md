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

## Wide classes — the quadruplet and bucket chaining

The `u64` mask above is **one bucket** of 64 field positions. A wide class — an
Odoo `account.move` carries ~100+ fields — overflows one bucket, but **not the
pattern**: the mask is a *chain of buckets*, each a `u64`, and the `selected()`
loop is bucket-agnostic (it filters `FieldDesc[]` by `idx`; the bucket is
`idx / 64`, the bit `idx % 64`). A clean `ClassView` chains up to **4 buckets —
the `[u64; 4]` quadruplet = 256 positions** (operator 2026-06-29: expand the
single-`u64` cap 64 → 256):

```
fields  buckets  fits one ClassView?
≤ 64       1      yes (the original u64)
65–128     2      yes  — an Odoo ~109-field model is clean here (2 buckets)
129–192    3      yes
193–256    4      yes  — the full quadruplet
> 256      5+     NO — god object: the SoC signal to SPLIT (see below)
```

This is **clean separation overflow automation**: each run of 64 chains the next
bucket automatically; at most 4 buckets is one ClassView; beyond 256 the class
is a god object and the overflow chains into a *second* ClassView (sub-views),
never a wider single mask. Pinned + tested in `ruff_spo_address::soc`:
`FIELD_MASK_BUCKET_BITS = 64`, `FIELD_MASK_MAX_BUCKETS = 4`,
`FIELD_MASK_CAP = 256`, `field_mask_buckets(n) = n.div_ceil(64)`; the
`Duplication` verdict now collapses to `≤ 256` distinct `field_type`s (a
109-field class is `Duplication`/maskable, not a `Counterexample`). The matching
`lance_graph_contract::class_view::FieldMask` (today `u64` / `MAX_FIELDS = 64`)
is the *eventual* expansion to the same quadruplet, validated by the ruff test.

## Simple rules (operator 2026-06-29)

- **If it's a template, it's probably a ClassView.** A Redmine ERB partial, an
  Odoo view, an Askama field partial — each is a render over a class's field set,
  i.e. a `ClassView` + a mask. Don't reach for a per-template type; reach for a
  mask over the generated `FieldDesc[]`.
- **Deduplicate routes.** N routes that are "the same record, different visible
  fields" (a card, a full view, an RBAC view, a tenant projection) are **one**
  templated ClassView render with N masks — not N handlers. Route proliferation
  is usually an un-applied mask.
- **`< 256` is clean; `≥ 256` is the god-object signal.** A field/sibling set
  under the quadruplet is maskable by one ClassView. At/over 256 the design (not
  the storage) is the problem — split concerns (chain a second ClassView /
  sub-view), the same SoC the `ruff_spo_address::soc` lint flags. Never widen the
  mask past the quadruplet to dodge the split.

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

One generic Askama field partial + a generated `FieldDesc[]` table + a
chained-bucket mask (`u64` → up to the `[u64; 4]` quadruplet, 256 fields) = the
whole dynamic ClassView field view: Redmine-shaped, JSON-free, no conditionals
in the template, type-checked structure, dynamic across versions/roles/
projections, and wide-class-clean by bucket chaining (a god object at `> 256` is
a split signal, not a wider mask). **The mask carves; the loop renders.**
Askama's compile-time nature is not a cage — the mask is the runtime knob, and it
is the same selector the data layer already uses to prune columns.

## Cross-references

- §1.5 "the spine is the COMPILED ClassView" — the read-side twin (the render
  mask = the facet read selector).
- `docs/OGAR-AS-IR.md` — ClassView as a compiler IR surface.
- `docs/OGAR-CONSUMER-BEST-PRACTICES.md` — the consumer muscle-memory guide this
  pattern slots into.
- `I-LEGACY-API-FEATURE-GATED` — why the mask bits must be generated, never
  hand-numbered alongside a layout.
- `ruff_spo_address::soc` — the `FIELD_MASK_CAP = 256` quadruplet,
  `field_mask_buckets()` chaining, and the `≥ 256` god-object SoC lint (where
  the "wide classes / split, don't widen" rule is tested).
- `lance_graph_contract::canonical_node::GUIDS_PER_NODE` (= 32) — the node-level
  twin: clean/SoC over packed, Tetris concerns across the 32 GUID slots; the
  field-level quadruplet here is the same doctrine one level down.
