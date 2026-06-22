# Verb-as-class — the ontological askama/jinja

> **Insight (operator, 2026-06-22).** When a verb is encoded as
> `rdfs:Class` instead of `owl:ObjectProperty`, the TTL file becomes
> a **compile-time-validated action template** — the ontological
> counterpart to askama (Rust) and jinja (Python) HTML templating.
> WorkOrder uses this convention (12 verbs declared as classes); the
> existing `ogar-render-askama` crate is the natural integration
> point.
>
> Status: **FRAMING v0** (2026-06-22). Companion to
> `docs/OGIT-DOMAIN-LIFT-CATALOGUE.md` (WorkOrder row) and
> `docs/FOUNDRY-ODOO-MARS-LENS.md` (the Foundry-parity angle).

The convention in one sentence: **a `rdfs:Class` verb declares a typed
slot list; a render takes a context binding and produces a materialised
SPO triple + declared side effects; the engine validates slot↔binding
at the same point askama validates `{{ name }}` against the struct
field.**

## The two encodings compared

| | `owl:ObjectProperty` verb | `rdfs:Class` verb |
|---|---|---|
| Carries | name + description | name + description + **slot list** + **inheritance** + **policy attributes** |
| Subject/object types | implied (by domain/range, often missing in OGIT) | explicit (`ogit:mandatory-attributes` enumerates the slots) |
| Inheritance | not native | `rdfs:subClassOf` gives template inheritance |
| Policy metadata | not native | any attribute the consumer wants (`ogit:requires-perm`, `ogit:emits-audit`) |
| Compile-time slot validation | no — verbs are flat names | yes — `ogar-from-schema` lifts the slot list; the renderer checks bindings |
| Round-trip with `ogar-from-schema` | yes (via `sgo::parse_verb`) | yes (via `ttl::parse_file` as Entity) |
| **Renders as** | a label on an edge | a typed action with side effects |

Both encodings round-trip cleanly today. The choice is about **what the
verb is *for*** — a flat predicate (use `owl:ObjectProperty`, like
SGO's 176) or a typed action template (use `rdfs:Class`, like WorkOrder's 12).

## Term-for-term with askama / jinja

```
askama / jinja                 │  ontology (verb-as-class)
─────────────────────────────  │  ─────────────────────────────────────────
template.html.j2               │  vocab/imports/ogit/NTO/<Domain>/verbs/<Verb>.ttl
                               │      a rdfs:Class
struct Context { name: String  │  ogit:mandatory-attributes (
                                  │      ogit:subject
                                  │      ogit:object
                                  │      …
                                  │  )
context binding                │  per-call value map
{% extends "base.html.j2" %}   │  rdfs:subClassOf ogit:AuditableAction
filters / macros               │  ogit:requires-perm, ogit:emits-audit
compile-time slot check        │  ogar-from-schema validates the slot list
                                  │      against the binding at lift time
render() → HTML string         │  render() → (SPO triple, audit record,
                                  │             ACL decision, side effects)
```

The structural correspondence is exact. Both are
**compile-time-validated declarative templates**; both separate
"template" (TTL file / `.html.j2`) from "context" (binding / struct);
both surface invalid bindings before render. The output medium
differs (HTML string vs. graph delta) but the engine shape is the same.

## A worked example — `WorkOrder/verbs/AccessesPortal.ttl`

```turtle
ogit.WorkOrder:AccessesPortal
    a rdfs:Class;                               # VERB-AS-CLASS
    rdfs:subClassOf ogit:AuditableAction;       # inherits audit slot
    rdfs:label "AccessesPortal";
    ogit:mandatory-attributes (                 # SLOTS — like askama struct fields
        ogit:subject                            #   who
        ogit:object                             #   what portal
        ogit:timestamp                          #   when
    );
    ogit:requires-perm "portal_login";          # template metadata
    ogit:emits-audit "true";                    # render-time side effect
.
```

When `User#42 accesses Portal#1` at time `T`:

1. **Lookup** — the renderer resolves `AccessesPortal` to its lifted
   `Class` (the template).
2. **Bind** — the binding `{ subject: User#42, object: Portal#1,
   timestamp: T }` is type-checked against the slot list. Missing
   `timestamp` ⇒ render-time error (same as askama: missing struct field
   ⇒ compile error).
3. **Render** — emit:
   - SPO triple `(User#42, AccessesPortal#<id>, Portal#1)` with
     `timestamp = T`
   - Audit record (because `emits-audit = true`)
   - ACL gate (must satisfy `requires-perm = "portal_login"`)

The output is a **graph delta + side-effect spec**, not a string.

## Why this is the right integration point for `ogar-render-askama`

The crate already does askama-template rendering for `Class` **views**
(per-app skin per `docs/OGAR-CONSUMER-BEST-PRACTICES.md`). Verb-as-class
is the parallel render path for `Class` **actions**:

```
ogar-render-askama/
├── views/      — Class views (HTML, JSON, OpenAPI)     ← EXISTING
└── actions/    — Class actions (verb-as-class render)  ← NEW (this framing)
```

Same `Class` IR, same askama engine, same compile-time-validated
template/binding pattern. Dispatch is on what shape the `Class`
declares: noun-shaped (entity fields) → view render; verb-shaped
(`mandatory-attributes` = subject/object/timestamp/…) → action render.

## The Foundry-parity sharpening

Foundry's "action types" carry exactly the four properties listed in
the comparison table:

1. Typed parameters (slots)
2. Compile-time validation of bindings
3. Declared side effects (audit, ACL, downstream effects)
4. Inheritance / composition

Foundry sells those as a paid platform feature. Verb-as-class TTL +
`ogar-render-askama` gives the same four properties from
**open-source schemas and Rust templates** — no vendor, no lock.

## What this changes for the next session

Three small implications:

1. **Don't normalise WorkOrder's convention** to `owl:ObjectProperty`.
   The earlier framing (in the commit message of `cce8420`) was wrong;
   re-encoding would strip the template surface.

2. **The `actions/` submodule in `ogar-render-askama` is the natural
   landing** for the verb-as-class renderer. Not implemented today;
   ~200 LOC mirroring the existing `views/` render path.

3. **The verb-as-class convention is a candidate for prototyping
   in WorkOrder first** (since we're upstream — `dcterms:creator` =
   `bus-compiler` / `family-codec-smith`), then pitching to OGIT
   upstream once `ogar-render-askama`'s actions path has proven the
   pattern.

## Cross-references

- `crates/ogar-render-askama/` — the existing askama renderer (views
  today; actions tomorrow)
- `crates/ogar-from-schema/` — the lift that turns TTL into the IR
  the renderer consumes
- `docs/OGIT-DOMAIN-LIFT-CATALOGUE.md` — WorkOrder row with the
  verb-as-class convention note
- `docs/FOUNDRY-ODOO-MARS-LENS.md` — the Foundry-parity argument
  that verb-as-class sharpens
- `docs/OGAR-CONSUMER-BEST-PRACTICES.md` — the per-app `ClassView`
  pattern that views-side rendering already follows
- `vocab/imports/ogit/NTO/WorkOrder/verbs/*.ttl` — the 12 verb-as-class
  templates this framing describes
