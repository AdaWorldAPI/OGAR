# Grounding: the DocIr Composition Layer (+ the STL/geometry chunk)

> **Companion to `docs/DOCIR-COMPOSITION-LAYER.md`** (the operator ruling,
> #217, ADOPTED). This doc does not restate the doctrine — it **grounds it in
> shipped code and the append-only ledger**, and answers the "produce a repo
> map / gap analysis / crate boundary / smallest slice / exact files /
> terminology" ask that came with the 2026-07-20 feedback.
>
> **Two parts, two grades.**
> **Part A** grounds the DocIr composition layer — `[G]`, because it is the
> continuation of an operator-ruled arc, and every "new" name turns out to be
> an **expansion of a brick the canon already shipped or ruled**, not fresh
> architecture.
> **Part B** grounds the STL / mesh / surfel / `ogar-bim` chunk — `[S]`,
> **council-pending, not adopted**: it is largely a **rediscovery** of the
> "address is geometry" arc (`D-FMA-SKELETON` / `D-GUID-TIER` / `D-BOUNDS` /
> `helix` / `jc` + the lance-graph `3DGS-*` plans). Part B is a **reuse-map
> that defers the build** — recorded so a future session does not rebuild what
> exists; it mints nothing, adds no crate, writes no code.
>
> Verbatim-preservation rule (as in the doctrine): the operator's `[G]`
> formulations are canon text; this grounding regrades or appends, never
> rewrites.

---

## Part A — DocIr composition is the expansion of the doc-layer arc `[G]`

### A0. The arc it continues (not a greenfield proposal)

The composition layer is the fourth beat of one arc, each beat the **same
positional-projection brick at a new altitude**:

```
D-OGAR-DOC-LAYER (#191)  → persist/reconstruct: document node = {sha256, kv_key,
                            mime, counts}; awareness = GUID-keyed SoA subtree;
                            reconstruct_document ActionDef; DocRenderer trait
D-DOC-IR-SECOND-RETINA    → doc.v1 promoted to the perceptual IR (ogar-doc-ir);
      (#197 / #199)         N retinas → one closed doc IR; A3: "document template
                            = ClassView × WideFieldMask, same brick as Klickwege
                            — no new DSL"; "a screen and a document are the SAME
                            positional projection; DocRenderer gains adapters"
D-A2UI-SCREEN-ADDRESSING  → the live surface is DocRenderer's fourth adapter;
      (#204–#207)           RBAC by projection = WideFieldMask ∩ role-mask;
                            ogar-render-askama::field_view is the render half
D-DOCIR-COMPOSITION (#217)→ THIS: the document becomes a lens over the object
                            graph; ObjectSlots are typed projection portals
```

> **Status 2026-08-25:** the first beat's mints + ActionDefs (`typed_field`
> `0x080A`, `document` `0x080B` confirmed, `persist_document`/
> `read_document`/`reconstruct_document`) are 5+3 council-ratified and
> shipped — see `D-OGAR-DOC-LAYER`'s Status paragraph in `DISCOVERY-MAP.md`.
> The design/arc grade here (`[G]`, "continuation of an operator-ruled arc")
> was always about the SHAPE, independent of whether the first beat's mints
> had landed; both are now true together.

Reading the composition layer against this arc dissolves the "is this a new
subsystem?" question: it is the same brick, made **addressable to a foreign
node**.

### A1. The expansion map (the "new" names are widenings, not collisions)

Every element the doctrine frames as new is an expansion of a shipped/ruled
brick. The corrected framing (a naming *collision* would force an arbitrary
rename; an *expansion* keeps the concept and widens it — the general name goes
to the general concept):

| Doctrine element | The canon it expands | Grounding | Verdict |
|---|---|---|---|
| **`ObjectSlot`** (typed projection portal) | `D-DOC-IR-SECOND-RETINA` ruling **A3**: *"document template = ClassView × WideFieldMask, same brick as Klickwege — no new DSL."* | `ObjectSlot{target, class_view, field_mask, wide_field_mask, resolution}` **is** that A3 brick + an `ObjectRef` + a `ResolutionMode`. The portal is the doc-template projection pointed at another node. | **Expansion** (A3 brick made addressable) |
| **`FieldView`** (renderer-neutral enum) | `E-ONE-MASK-THREE-PORTS` / `E-OGAR-CONVERGENCE-SHAPE`: render is *"classview fieldmask → **fieldview fold** → askama/jinja"*; the fold is *"one fieldview partial per column type × skin, ~22 of them"* resolved longest-prefix (app-specific → concept-canonical). | The shipped `ogar-render-askama::field_view::FieldView` (`field_view.rs:58` — `struct { position, label, predicate, value }`) is the **text-leaf reading** of that fold. The doctrine's `enum FieldView {Text, Badge, Table, ObjectSlot, Geometry, …}` is **the fold formalized as its variant set**. The general name (`FieldView`) belongs to the general concept (the enum); today's struct is specifically an addressed field *row* and becomes the enum's `Text` payload (rename to e.g. `FieldRow`). | **Expansion, not collision** — the struct is the narrow reading of the widened concept |
| **`ProjectionRenderer`** (trait) | `A3`: *"a screen and a document are the SAME positional projection; **`DocRenderer` gains its fourth adapter**."* The trait already exists as `DocRenderer` (one-leg rule; runtime-bound tesseract / Spire.Doc / askama adapters). | The doctrine's `ProjectionRenderer` **is** `DocRenderer`. Askama/Typst/Blitz/Vello are adapters on the existing trait; do not mint a second trait — widen `DocRenderer`'s adapter set (and rename `ProjectionRenderer → DocRenderer` in the composition types). | **Expansion** (existing trait, more adapters) |
| **Named multi-view per class** (`WorkPackage.inline` vs `.summary`) | `E-ONE-MASK-THREE-PORTS`: a classview bitmask is **per-mode** ("one mask, three projections … per mode"). | Today `OgarClassView.by_id: HashMap<ClassId, ObjectView>` (`ogar-class-view/src/lib.rs:311`) keys by bare `ClassId` — one view per class. The expansion is to key by **`(ClassId, mode)`**; a "named view" is a mode, a mode is a mask. This is the one structural registry change the slice forces. | **Expansion** of the registry key |
| **RBAC by projection** | `E-ONE-MASK-THREE-PORTS` + the `D-A2UI` #205 correction: `effective_mask = classview_mask ∧ role_mask`; `field_mask` retyped to `WideFieldMask`; fail-closed; sentinel ban. | **Ruled, but only the transport half is coded.** The wire carries wide masks (positions ≥ 64 decode on the `ogar-a2ui-frame` wire) and `a2ui-server` projects *before* framing (`render_stream.rs`: RBAC before frame). But the **enforcement** — an `impl ClassRbac` / `effective_mask` that computes `classview_mask ∧ role_mask` — does **not** exist under OGAR `crates/` (`ClassRbac` is spec-only, `docs/CLASSID-RBAC-KEYSTONE-SPEC.md`; enforcement is **probe-gated on `PROBE-OGAR-RBAC-AUTHORIZE`**, `ISSUES.md` ISS-RBAC-AUTHORIZE-BY-CLASSID). | **Transport CODED; enforcement spec + probe-gated — NOT shipped** |
| **`DocNode`** (composition-only) | `D-DOC-IR-SECOND-RETINA`: *"one closed doc IR"* — the observation IR (`DocIr{version, source, geometry, content_sha256, mime, pages, fields}`, `ogar-doc-ir/src/lib.rs:207`) is the perceptual retina and is **untouched**. | `DocNode` is a **new composition layer** whose leaves reference observations through `ObjectSlot`s (an imported scan appears in a report as a portal, not as pasted regions). No domain variants (`Wall`/`WorkPackage`) enter `DocNode` — those are OGAR nodes the slots point at. | **New module**, observation IR preserved |
| **`ResolutionMode{Live, Revision, Snapshot}`** + `ogar://` | `@sha256:` = the content-address discipline the doc KV already enforces (`DocumentId = sha256(raw)`, `D-OGAR-DOC-LAYER`); `@revision:` = Lance versioning (ADR-008/013 shape). | Clean, unclaimed names (confirmed absent from `crates/`). The beyond-Rails move; the storage semantics are reuse. | **New names, reused semantics** |

### A2. Crate boundary — bound by the A1 ruling, not open

The doctrine's §3 leaves crate placement "decided at implementation time
(`ogar-doc-ir` module vs sibling `ogar-doc-compose`)." The ledger already
narrowed it: `D-DOC-IR-SECOND-RETINA` operator ruling **A1** — *"one `ogar-doc`
crate; split axis is STATE not direction."*

**Recommendation:** the composition graph is a **module in the doc-ir family**,
not a new `ogar-doc-compose` crate. The observation IR (`ogar-doc-ir`, serde-only,
zero canon dep) stays as-is; `DocNode`/`ObjectSlot`/`DocOp` land as a
composition module referencing observations by slot. This honors A1 and keeps
the "one closed doc IR" invariant — the composition layer is *state above* the
observation IR, not a *direction split* into a second crate.

### A3. The genuine gaps (what the slice actually has to build)

Filtered through A1: what is truly new vs. what is a widening.

| Gap | Kind | Note |
|---|---|---|
| `ObjectSlot` / `ResolutionMode` / `ogar://` parse | **new** | clean names; `ObjectSlot` = A3 brick + `ObjectRef` + `ResolutionMode` |
| composition `DocNode` | **new module** | in the doc-ir family (A1), leaves reference observations by slot |
| `DocOp` (editor authority; Trix pattern — model is authoritative, not the DOM) | **new** | CRDT-compatible later; single-writer + content-addressed saves suffice for v1 |
| `FieldView` (enum) | **widen** | formalize the E-ONE-MASK-THREE-PORTS fold; the shipped struct becomes the `Text` leaf (rename struct → `FieldRow`) |
| `DocRenderer` named multi-view | **widen** | registry key `ClassId → (ClassId, mode)` |
| `ogar-render-typst` | **new adapter** | greenfield (no Typst anywhere in the workspace); the doctrine's §9 kill-criteria apply (demote to export-only if Typst can't be generated from render nodes without arbitrary-syntax leakage) |

### A4. First vertical slice — grounded landing zone in `openproject-nexgen-rs`

The doctrine's §7 slice (`WorkPackage.description` → 3 attachables × 2 modes →
render twice HTML + Typst) lands here, exact files:

- **The field:** domain `WorkPackage.description: Formattable`
  (`crates/op-work-packages/src/work_package.rs:74-76`) — **hand-written, safe
  to touch.** **Never** the generated `op-generated` `description: String`
  (`crates/op-generated/src/generated/work_package.rs:22` — `// @generated …
  do not edit by hand`, regenerated by `op-codegen-pipeline`'s `emit_generated`).
- **Adapt-don't-rewrite is already the idiom:** `impl From<&WorkPackage> for
  CanonicalWorkPackage` (`work_package.rs:253-277`) already projects the rich
  domain type down to the generated `String`. The `DocNode` tree lives in the
  hand-written layer; that same `From` projects it to/from the generated
  `String` (markdown or DocIr-JSON) at the crate boundary — exactly the
  doctrine's "adapt generated DTOs, don't hand-rewrite them."
- **The render seam:** `CellData::RichText` at `crates/op-server/src/board.rs:735`
  and `:1446` — today `format!("<p>{}</p>", esc(...))` (a bare escaper; **no
  markdown renderer is wired anywhere** — `Formattable::markdown` is a `TODO`
  placeholder at `op-core/src/types.rs:36-44`). Replacing that one expression
  with a `DocRenderer` pass over the resolved slots is the minimal-diff
  insertion point; `render_wp_detail` (`board.rs:701-832`) is already a pure,
  DB-decoupled function that resolves `status_name`/`author_name`/… before the
  renderer — slot resolution extends that same pre-render step.
- **The attachables:** `User` (`crates/op-models/src/user/model.rs`), `Attachment`
  (`crates/op-attachments/src/model.rs`). Both already have generated +
  hand-written shapes. Crucially, the in-text machinery a slot would replace is
  **all-TODO**: no `@mention` parser exists, and the WP-link macro is a
  generated stub with every method `// TODO`
  (`op-generated/.../work_package_exports_macros_work_packages_link_handler.rs`).
  → **clean, unblocked landing zone; nothing to migrate away from.**
- **Classid discipline (fix as a forcing function):** `op-server` currently
  bypasses the tested `op_canon::app::class_id_of` and calls
  `ogar_vocab::canonical_concept_id` inline (`board.rs:25` + 7 call sites).
  Slot-target classid resolution should route through `op_canon::app`
  (the OGAR-consumer-best-practices pattern), fixing that inconsistency.
- **RBAC is the slice's own responsibility — do NOT assume it is shipped.**
  Because `ClassRbac`/`effective_mask` enforcement is spec + probe-gated (§A1),
  the slice's slot resolution MUST apply the fail-closed intersection
  `resolved_mask = requested_wide_mask ∧ role_wide_mask` **server-side, before
  rendering**, and treat a missing/narrow role mask as a **refusal**, never a
  fall-through to the requested mask (the `full_for` sentinel is a render
  convenience, never an RBAC fallback — the #205 rule). A slot that supplies an
  unfiltered mask must not cause unauthorized fields to render. Until the OGAR
  `ClassRbac` enforcement lands, the slice carries the intersection itself
  (mirroring `a2ui-server`'s `project_surface` fail-closed gate); it does not
  wait for, nor assume, the upstream primitive.

**Slice = ** 5 `DocNode` kinds + 3 attachables × 2 modes each + render HTML
(askama, exists) + Typst PDF (greenfield adapter). The Typst leg is the only
part with no reuse floor.

### A5. `ogar-bim` recommendation

No BIM / mesh / `Wall` / CAD concept is minted anywhere in the codebase today
(confirmed across `crates/`), so the names are clean. Per doctrine §6,
`ogar-bim` is **semantic objects + ClassViews** (a `Wall` is an OGAR node with
`document-inline` / `property-table` / `viewport-selection` views), **not a
document format** — addressable from documents via `ObjectSlot` like any other
node. Recommendation: a new **producer + vocab family** (mint concepts in
`ogar-vocab` under a new domain byte, consumed via the classid membrane like
every other consumer) — but **deferred**, because BIM geometry is exactly the
Part B chunk that is unruled and probe-gated. The slice's "one `BimObject`"
step should be **one minimal semantic node with a `document-inline` view**, not
an IFC ontology.

### A6. The OGAR-AS-IR gate (composition types are an IR-surface change)

`DocNode` / `ObjectSlot` / `FieldView` / `DocOp` must pass the six
`docs/OGAR-AS-IR.md` tests before landing: (1) SSA / dataflow-explicit; (2)
effect annotations first-class; (3) typed signature not field-bag; (4) named
lowering passes (each renderer is a pass `IR → target`, adapters lower not
transform); (5) optimization passes declare a semantic-preservation guarantee;
(6) the IR is the canonical artifact (no renderer dialect creeps into
`DocNode`). A type failing a test is *rerouted* to the layer it belongs to, not
rejected. `SURREAL-AST-TRAP-PREFLIGHT` applies to any `ogar://` → storage
lowering.

---

## Part B — `[S]` The STL / geometry chunk rediscovers the "address is geometry" arc

> **Status: `[S]` PROPOSAL-GROUNDING. Council-pending; probe-gated; NOT
> adopted.** No mint, no crate, no code. This is a **reuse-map that defers the
> build** — recorded so a future session grounds on the existing arc instead of
> rebuilding it. Adoption requires the 5+3 council **and** a green probe.

### B0. The gestalt it rediscovers

DISCOVERY-MAP §1, the spine's one sentence:

> *"the substrate is an immaterialized Morton cascade with templated payloads —
> **address is geometry (helix + HHTL)**, error is closed-form (jc),
> materialization is lazy and columnar; the only forbidden cost is
> non-amortized per-query."*

The STL → mesh → surfel stack proposed in the feedback maps almost one-to-one
onto this already-`[G]`/CODED arc.

### B1. Reuse map (proposed type → existing anchor)

| Proposed type | Existing anchor | Grounding |
|---|---|---|
| `HhtlNode` / spatial-LOD octree | **`HhtlMode::Located`** — `D-GUID-TIER` **CODED** | `ogar-fma-skeleton/src/guid.rs:328` (`located_3d(x,y,z)` / `position_3d` / `morton3`) is a shipped Cesium 3D-octree CRS *on the GUID key*. The dual `HhtlMode::{Located, Cascade}` is the exact "don't overload HHTL silently" discipline — HHTL stays the key-path canon; a spatial index is the **`Located` reading**, not a new hierarchy. |
| `HelixAddress` | **`helix`** crate (lance-graph) | `crates/helix` is a shipped golden-spiral hemisphere Place/Residue codec (`ResidueEdge`, 3-byte endpoint pair, O(1) 256×256 metric-safe LUT). `hhtl ++ helix` is the sanctioned **L7 "absolute location, two hemispheres"** facet layout (`le-contract.md:62`). **Surfel normals ride `helix::ResidueEdge`; do not mint `HelixAddress`.** |
| `Surfel` / splat math | **`jc::ewa_sandwich_3d`** (certified 3×3 SPD push-forward) + `ndarray::hpc::splat3d` (renderer) + **`D-FMA-SKELETON`** | The splat math is shipped and certified (ρ ≥ 0.999); the FMA arc is the splat-native mesh precedent. A `Surfel` is a splat carrier over this math, not new math. |
| `Aabb` / bounds | **`D-BOUNDS`** | Tile bounding volume is **derived from the address** (address arithmetic), *not stored*, by design. No `Aabb` struct is the intended state. |
| LOD / geometric-error | **`D-CESIUM-PROBE`** + `jc` closed-form + lance-graph `3DGS-*.md` (20-file plan family) | Cesium SSE refine/collapse is the trial-and-error probe `ADR-025` **removes** (closed-form level). LOD is address arithmetic, not a stored scene graph. |
| `BufferRef` / content-addressed mesh payload | `canonical_node.rs:706-723` | The anatomy mesh (**4M vertices / 6M triangles**) is *named as the worked example* of an out-of-line Lance table referenced by key/classid. That IS the `BufferRef` pattern — unnamed, already canon. |
| `PartGraph` | `ogar-fma-skeleton` `same_family` partonomy (`guid.rs`) | "relations ARE the addressing: local relation ⇔ shared family prefix; prefix = partonomy = spatial containment" (`D-BOTHCASC`). An STL `PartGraph` should extend this, not duplicate it. |
| `FeatureIr` | — | Genuinely new, but **name-overload risk**: "Feature" is already used for `Fragment → Feature/VectorCentroid` in the 3DGS datalake plan, and `Field*` is the classview vocabulary. Pick a name that doesn't overload `Field`/`Feature`. |
| `SourceMesh` / `MeshIr` | — (3DGS plans are design-only) | The one genuinely-new tissue (see B3). |

### B2. Naming discipline (same principle as Part A: widen, don't fork)

- **Do NOT mint a new "HHTL"** for a spatial octree. HHTL is the key-path /
  routing-cascade canon (`hhtl.rs`, `canonical_node.rs`, `CLAUDE.md` P0). A
  spatial index is the **`HhtlMode::Located` reading of the same key**, with the
  explicit-mode guard already shipped in `ogar-fma-skeleton`.
- **Do NOT mint `HelixAddress`.** Ride `helix::ResidueEdge` (and the L7
  `hhtl++helix` layout).
- **Bounds are derived** (`D-BOUNDS`), not a stored `Aabb`.
- A `Surfel`'s math is `jc` / `splat3d`, not new.

### B3. The one genuinely-new sliver

Everything downstream of ingestion is reuse. The new tissue is narrow:

> **STL ingest → mesh normalize → feature/part segmentation**, as a **new
> producer** that feeds the existing address + splat substrate — structurally
> the mirror of `ogar-from-ruff` (which feeds the class substrate). `SourceMesh`
> (immutable evidence) + `MeshIr` (normalized topology) + the segmentation into
> a `PartGraph` (extending `same_family`) is the producer; addressing, LOD,
> surfel encoding, and storage are all reuse.

### B4. Probe-first gate (why this is `[S]`, not `[H]`)

`D-FMA-SKELETON` already grades **splat-fit convergence a CONJECTURE**
(`address structure CODED; splat-fit convergence CONJECTURE`). The proposed
producer inherits that gate: it is `[S]` until a probe runs — e.g. **one STL →
`PartGraph` → surfel → re-render round-trip**, fidelity-checked against the
source mesh, with selection round-tripping from a rendered pixel back to the
source triangle and the OGAR node. Per workspace probe-first discipline, the
next deliverable for this chunk is **that probe, not more synthesis**. It
belongs in the SPLAT-NATIVE / FMA arc (`docs/SPLAT-NATIVE-CUSTOMER.md`,
`docs/FMA-SKELETON-CONVERGENCE-ANCHOR.md`) + the lance-graph `3DGS-*.md` plans.

### B5. Explicitly not adopted

No mint, no crate, no code, no ADR. This section is a reuse-map so the arc is
not rebuilt; adoption needs the 5+3 council **and** a green probe. The
`ogar-bim` semantic-object family (Part A §A5) is the *document-facing* half and
can proceed independently of this geometry chunk — a `Wall` node with a
`document-inline` ClassView needs no mesh pipeline; the mesh pipeline is what B0–B4
defer.

---

## Appendix — one-line index of what this doc grounds

- **Part A** — the composition layer is `D-OGAR-DOC-LAYER → D-DOC-IR-SECOND-RETINA
  → D-A2UI → D-DOCIR-COMPOSITION`, and its "new" names are widenings:
  `ObjectSlot` = the A3 Klickwege brick made addressable; `FieldView` (enum) =
  the `E-ONE-MASK-THREE-PORTS` fold formalized (the shipped struct is its `Text`
  leaf); `ProjectionRenderer` = `DocRenderer`; named multi-view = mask-per-mode;
  RBAC = the ruled `classview_mask ∧ role_mask` (transport CODED; the
  `ClassRbac`/`effective_mask` enforcement is spec + probe-gated, NOT shipped).
  Crate placement is bound by
  the A1 "one crate" ruling. Slice landing zone is grounded to exact files in
  `openproject-nexgen-rs`. (`D-OGAR-DOC-LAYER`'s mints/ActionDefs: council-
  ratified + shipped 2026-08-25, see `DISCOVERY-MAP.md`.)
- **Part B** — the STL/geometry chunk is a `[S]` rediscovery of the "address is
  geometry" arc (`HhtlMode::Located` / `helix` / `jc` / `D-FMA-SKELETON` /
  `D-BOUNDS` / `3DGS-*` plans); reuse, don't fork; the one new sliver is an
  STL-ingest producer, probe-gated; not adopted.
