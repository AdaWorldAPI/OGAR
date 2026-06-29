# OGAR as a semantic transpiler that sinks into the v3 substrate

> **Status:** PROPOSAL — captured before dilution; pending the 5+3 hardening pass
> before it enters the canon. **Grade the claims [G]/[H]/[S] during review.**
>
> Capstone synthesis. Names the whole arc in one frame: OGAR is a **semantic
> transpiler**; the **v3 substrate** is its **sink**; and the patterns already
> established (the compiled ClassView §1.5, the SoA-schema-glove, the bitmask
> field-view, the classid/HHTL address, the ractor mailbox-owned SoA, the no-wire
> FFI) are the **superpowers** that make the sunk graph a living, multi-scale,
> queryable, renderable knowledge graph — without per-source code.

## 1. The frame

A transpiler is a source-to-source compiler. OGAR transpiles **source format →
substrate**: every front-end (GFA/VCF, OWL/OBO, SNOMED, RxNorm, DICOM, FHIR, and
whole applications — Odoo, Redmine, OpenProject) lowers into the OGAR AST and
**sinks** into the v3 node store. "Sink" is §1.5's own word: the ClassView
"recombines the carvings while *sinking into OGAR* and getting compiled into the
binary." The transpiler does not emit rows into a database — it emits **typed,
classid-addressed, compiled** nodes into a substrate that can route, group, and
render them from the key alone (the key prerenders with zero value decode).

## 2. Two kinds of front-end (syntax is NOT generically lost)

The earlier framing — "the format dissolves, only meaning survives" — is correct
for **one** class of source and wrong for the other. The distinction is load-bearing.

- **Serialization formats** (GFA, VCF, raw RDF triples, DICOM pixels). Syntax is
  incidental; the meaning *is* the columns. "GFA = SoA wearing a schema glove" —
  a ~200-line parser, no DO arm, nothing behavioural to preserve. Here syntax
  genuinely dissolves into SoA columns + a classid schema.
- **Semantic application sources** (Odoo models/views/actions, Redmine ERB field
  views + models, OpenProject work-package schema). The source is *already
  semantic syntax*. This is **semantic-syntax → semantic-syntax**, structure-for-
  structure. **Syntax is preserved — re-hosted, not dissolved.** Redmine's
  ERB-field-view-with-column-masking transcodes into the Askama bitmask
  field-view *structure-for-structure* (see `CLASSVIEW-FIELDVIEW-ASKAMA-BITMASK.md`):
  the field-partial-with-selection survived; it only changed host language.

## 3. The three arms — what is preserved vs what is re-imagined

Per `OGAR-AST-CONTRACT.md` (THINK arm `Class` / DO arm `ActionDef`+`ActionInvocation`
/ membrane `KausalSpec`), an application transcode splits cleanly:

- **THINK arm — `Class` / ClassView / fieldview (data + view): STRUCTURE-PRESERVING.**
  Odoo view → ClassView; Redmine ERB field partial → Askama bitmask partial;
  OpenProject WP schema → Class facets. Semantic syntax survives.
- **DO arm — `ActionDef` / `ActionInvocation` (behaviour): RE-IMAGINED as
  classid-keyed adapters.** This is the *only* part that is not a structure-for-
  structure carry. And — the load-bearing insight — **re-imagining actions as
  adapters is not a tax; it is the separation-of-concerns / God-object fix you
  would want regardless.** Odoo and Rails (Redmine/OpenProject) fuse data + view
  + behaviour into one fat model class — the canonical God object. The transcode
  is *forced* to split it into `Class` (data) + fieldview (view) + adapter
  (behaviour), so you exit with the three-way separation the monolith never had.
  The "cost" is a **refactor dividend** — paying down the source codebase's
  architectural debt as a side effect of the port.
- **membrane — `KausalSpec`.**

This is the Core-First Transcode Doctrine applied to whole apps (the
adapter-shaper / core-gap-auditor ensemble): identity = `classid`, state = SoA
value tenants, composition = `ClassView`, **invocation = adapter** — and the iron
rule that an adapter *never carries its own state*; a Core gap means **extend the
Core, never hack the adapter**. "Don't let the adapter hold state" *is* "kill the
God object, don't smear its responsibilities."

## 4. The superpower — one selector, five jobs

The field bitmask documented in `CLASSVIEW-FIELDVIEW-ASKAMA-BITMASK.md` is not a
UI trick. It is the **universal selector**, and it is the same generated bit set
everywhere:

| job        | the mask selects… |
|------------|-------------------|
| **read**   | which facets to decode (§1.5 compiled reader) |
| **query**  | which columns to project (Arrow / lance-graph column pruning) |
| **render** | which fields the Askama loop emits (no `if`-noise) |
| **version**| V1 vs V2 vs V3 = different masks over one compiled reader/template |
| **auth**   | which fields a role may see (RBAC) |

Decode, query, render, version, authorize — **one set of bits.** That is why a
sunk node is *instantly* usable: the transpiler assigns the classid + the facet
mask, and every downstream operation is already wired because they consume the
same selector over the same SoA columns. You do not build a query layer, a view
layer, a versioning layer, and an auth layer — you **generate the bits once** and
reuse them five ways. (Iron-rule gate: `field ↔ idx ↔ bit` must come from ONE
generated source — `I-LEGACY-API-FEATURE-GATED` — or bit 17 silently re-aliases
across a version bump and corrupts all five jobs at once.)

## 5. The address is the zoom

The HHTL / canonical-GUID cascade is the multi-scale "Google Earth" axis for free:
classid prefix → class + **topology signature** → renderer dispatch; the tiers →
the centroid mipmap → the zoom levels. Planet → organ → cell → chromosome → gene →
atom is **prefix routing**, not an application switch. The renderer it dispatches
to (branching-network / mesh / molecular) is chosen by the structural signature
carried in the classid — VG variation graphs and arterial trees land on the *same*
tube renderer because they share a signature, not a domain. Layout is one more
lance-graph pass that annotates positions; renderers stay dumb.

## 6. Living, not a dump; JSON-free, end to end

- **Living substrate.** The ractor mailbox-owned SoA gives `n × 64k` concurrent,
  compile-time race-free node updates — the sunk graph is a living kanban, not the
  batch-built static snapshot every other biomedical KG ships. The renderer sits
  on a surface being updated under it, safely.
- **JSON-free.** The transpiler writes SoA columns; the JS engine (JSC/V8) is
  in-process via op/FFI so Rust↔JS is a memory handoff; Askama renders records to
  HTML in pure Rust; the only wire is Arrow IPC / binary columns to the browser.
  The format dissolved (or was re-hosted) at the front door and never
  reconstitutes as JSON at the back.

## 7. The dots, connected

**Parse once per source (cheap) → sink into classid-keyed SoA → and query, render,
version, authorize, zoom, and live-update all come free, because it is the same
node, the same mask, the same address everywhere.** The biomedical "holy grail"
graph is not an app per layer — it is **the transpiler + the sink + the universal
selector**. Genomics (VG/GFA) is simply the next serialization front-end to lower
in; Odoo/Redmine/OpenProject are the semantic-application front-ends where syntax
is *preserved* and behaviour is *upgraded* (God object → adapters). The Askama
bitmask field-view is the render-side face of the exact selector the substrate
already uses to read, query, and version. One mechanism, wearing every glove.

## 8. Honest edges (do not let the synthesis become poetry)

1. **"One mask, five jobs" holds only if the bits are generated from one source.**
   Hand-number them and `I-LEGACY-API-FEATURE-GATED` bites — and it bites all five
   jobs at once. This is the single highest-leverage discipline in the capstone.
2. **The cross-ontology upper layer is still real work.** Mapping FMA / SNOMED /
   VG / RxNorm into one category tree — **adopt Biolink Model**, do not invent it.
3. **The renderer has a scale ceiling.** three.js goes geometry-bound in the
   millions of tris (the `/helix` mobile-freeze evidence); the multi-scale viewer
   leans on the LOD/HHTL cascade, not raw draw calls.
4. **Grade required.** Each claim above is [G] coded / [H] bounded-plausible / [S]
   analogy-only until the 5+3 pass grades it. The transpiler frame, the three-arm
   split, and the one-selector identity are the claims most in need of a
   runtime-archaeologist CODED-vs-CLAIMED check.

## Cross-references

- `CLASSVIEW-FIELDVIEW-ASKAMA-BITMASK.md` — the render-side face of the universal
  selector (the pattern this capstone frames).
- §1.5 "the spine is the COMPILED ClassView" — the sink + the read-mask twin.
- `OGAR-AST-CONTRACT.md` — the THINK / DO / membrane arms.
- `OGAR-AS-IR.md` — OGAR as a multi-phase compiler IR.
- `OGAR-CONSUMER-BEST-PRACTICES.md` — the consumer surface these transcodes target.
- `SURREAL-AST-AS-ADAPTER.md` / `core-first-transcode-doctrine` — actions → adapters,
  the God-object/SoC dividend.
- `I-LEGACY-API-FEATURE-GATED` — why the universal-selector bits must be generated.
