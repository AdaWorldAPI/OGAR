# DocIr as the Composition Layer over the OGAR Object Graph

> **Operator ruling, 2026-07-20** (jan@msgraph.de, verbatim doctrine preserved
> below). Status: **ADOPTED DOCTRINE** — design canon; implementation gated
> per §9. Supersedes the narrower "DocIr = the OCR document model" framing:
> DocIr does not remain merely the document model, it becomes **the
> composition layer over the OGAR object graph**. Ledger:
> `docs/DISCOVERY-MAP.md` `D-DOCIR-COMPOSITION`.
>
> One line: **the document becomes a lens through the object world, rather
> than a box where copies of that world go to slowly decay.**

## §0 The boundary move

```text
OGAR object graph
    │
    ├── OpenProject objects
    ├── BIM objects
    ├── OCR observations
    ├── files / images / drawings
    └── relationships
          │
          ▼
DocIr composition graph
    Text + structure + typed object slots
          │
          ▼
ClassView + FieldMask + WideFieldMask
          │
          ├── Askama / HTML / HTMX
          ├── Typst / PDF
          ├── Blitz native HTML/CSS
          ├── Vello / Parley canvas
          └── BIM / CAD viewport
```

The beautiful convergence is not choosing one editor. It is **making every
renderer drink from the same spring.**

## §1 The ObjectSlot doctrine (ActionText's attachment doctrine, transplanted)

Take almost exactly Rails ActionText's attachment doctrine:

- a document contains a **lightweight typed reference**
- the reference **resolves to a live domain object**
- its **class chooses the rendering partial**
- **missing objects have an explicit fallback**
- the document **never absorbs the attached object's complete representation**

(ActionText uses application-wide GlobalIDs, resolves an embedded attachment
to its underlying model, and renders that model through a type-specific
partial — so it displays updated model content without rewriting the stored
rich text.)

The OGAR equivalent:

```rust
pub struct ObjectSlot {
    pub target: ObjectRef,
    pub class_view: ClassViewId,
    pub field_mask: FieldMask,
    pub wide_field_mask: WideFieldMask,
    pub resolution: ResolutionMode,
    pub fallback: Option<SnapshotRef>,
}

pub enum ResolutionMode {
    Live,
    Revision(RevisionId),
    Snapshot(ContentHash),
}
```

**`ResolutionMode` is where we go beyond Rails.** A project dashboard wants
live content. A signed report, invoice, planning baseline, or construction
record wants a **pinned revision**. A deleted object needs a **snapshot
fallback**. The OGAR equivalent of an SGID:

```text
ogar://openproject/work-package/8f31...@live
ogar://bim/wall/a127...@revision:42
ogar://document/drawing/c920...@sha256:...
```

(The `@sha256:` arm is the same content-address discipline the document KV
already enforces — `DocumentId = sha256(raw)`, Lance-backed, immutable,
GoBD-grade. `@revision:` maps onto Lance versioning / the substrate's
versioned-audit shape, ADR-008/013.)

**The crucial correction of the pre-ruling proposal:** do NOT put attachments
inside DocIr as foreign little islands. Make them **typed projection portals
into OGAR.**

## §2 Sharp responsibilities — ClassView / FieldMask / WideFieldMask / FieldView

**`ClassView`** — a **named semantic projection** of a class. It says what
the object *means in this context*, not which renderer draws it:

```text
WorkPackage
 ├── compact-card
 ├── document-inline
 ├── inspector
 ├── print-summary
 └── bim-overlay
```

**`FieldMask`** — selects immediate fields:

```text
subject
status
assignee
progress
```

**`WideFieldMask`** — selects **relationship traversal and nested views**
(already CODED narrow→wide per the #204/#205 correction arc —
`ClassRbac::field_mask` is `WideFieldMask`; wide positions ≥ 64 decode on the
wire in `ogar-a2ui-frame`):

```text
project {
    name
}

assignee {
    name
    avatar
}

bim_links {
    object @classview("bim.document-inline") {
        name
        type
        geometry_preview
    }
}
```

This is a GraphQL-like selection tree **compiled into the radix-trie/path
machinery** — never requiring GraphQL itself.

**`FieldView`** — turns the resolved projection into **renderer-neutral
presentation intent**:

```rust
pub enum FieldView {
    Text(TextView),
    Badge(BadgeView),
    Table(TableView),
    ObjectSlot(ObjectSlot),
    Geometry(GeometryView),
    Document(DocumentView),
    Collection(CollectionView),
}
```

**The hinge:** Askama, Typst, Blitz, or Vello render `FieldView`. **They do
not query OpenProject or BIM independently.** (This generalizes the shipped
`ogar-render-askama::field_view` brick — a `WideFieldMask`-projected
ClassView instance rendered as an addressed surface — from one renderer to
the renderer-neutral contract.)

## §3 DocIr becomes composition-only

DocIr contains **only document composition**:

```rust
pub enum DocNode {
    Document(Vec<NodeId>),
    Section {
        heading: Option<NodeId>,
        children: Vec<NodeId>,
    },
    Paragraph(Vec<NodeId>),
    Text(TextRun),
    ObjectSlot(ObjectSlot),
    Figure(FigureRef),
    Table(TableModel),
    PageBreak,
}
```

**Do not put `Wall`, `WorkPackage`, `Patient`, or `Invoice` variants into
`DocNode`. Those belong to OGAR.**

- DocIr says: *put this object here, using this view and these fields.*
- OGAR says: *this is the object — its identity, properties, relationships,
  history, and provenance.*

That keeps the skeleton clean even as the creature grows new wings.

**Relation to today's `ogar-doc-ir` (migration note, additive):** the current
`DocIr{pages, regions, content_sha256, …}` is the **observation IR** — what a
retina (OCR pixel / DOM) faithfully saw (D-DOC-IR-SECOND-RETINA). That
pipeline (`tesseract-rs doc.v1 → ogar-from-docv1`) is untouched. The
composition `DocNode` graph is a NEW layer; OCR observations become OGAR
objects that composed documents **reference through ObjectSlots** (an
imported scan appears in a report as a portal, not as pasted regions). Naming
and crate placement (`ogar-doc-ir` module vs sibling `ogar-doc-compose`) is
decided at implementation time under the §9 IR gate — the observation IR is
never retyped in place.

## §4 Editor authority — what to take from Trix

Trix's deepest architectural idea is more valuable than its toolbar: it
treats `contenteditable` as an **input/output device**, applies edits to its
**own internal document model**, then re-renders. Borrow that. **Do not let
browser DOM, Typst markup, or HTML become authoritative.**

```text
keyboard / paste / drag / OCR
              │
              ▼
        EditOperation
              │
              ▼
           DocIr
              │
              ▼
         rerender view
```

```rust
pub enum DocOp {
    InsertText { at: Cursor, text: String },
    SplitBlock { at: Cursor },
    InsertObject { at: Cursor, slot: ObjectSlot },
    ChangeProjection {
        node: NodeId,
        class_view: ClassViewId,
        mask: FieldMask,
    },
    MoveNode { node: NodeId, destination: Cursor },
}
```

That operation layer can later be **CRDT-backed without replacing DocIr**
(single-writer + content-addressed immutable saves suffice for v1; the DocOp
log is the upgrade seam).

## §5 The renderer map — every renderer drinks from the same spring

```rust
trait ProjectionRenderer {
    type Output;

    fn render_document(
        &self,
        document: &ResolvedDocument,
    ) -> Result<Self::Output, RenderError>;
}
```

| Renderer | Role | Doctrine |
|---|---|---|
| **Askama / HTMX** | The first living interactive surface (exists: `ogar-render-askama::field_view`, medcare views) | Renders `FieldView`, never queries domains |
| **Typst** | **Paged projection** — print + archival (PDF, PDF/A, tagged/accessible PDF, SVG; incremental compile) | **Generate Typst from structured render nodes; never allow arbitrary Typst syntax into the canonical document.** Typst owns the paged projection, not the source document. `DocIr → resolve ObjectSlots → FieldView tree → ogar-render-typst → PDF/PDF-A/SVG` |
| **Blitz (Dioxus)** | Future native HTML/CSS body — the same semantic HTML/CSS the Askama partials emit, rendered natively on wgpu | **Behind the `ProjectionRenderer` trait** — explicitly experimental today; must not become a load-bearing ontology beam. The rare migration path where the temporary surface (browser+HTMX) is not disposable scaffolding |
| **Parley / Vello** | The **precision canvas** — canvas selection, drawing overlays, BIM annotations, infinite zoom, rulers/guides, precision text positioning, geometry-linked labels, mixed document/CAD surfaces | Not the first document renderer. `DocIr paragraph → Parley layout`; `BIM geometry/diagrams → Vello scene`; combined → wgpu texture. **Parley inline boxes carry typed object slots** — a WorkPackage badge / BIM symbol / formula / thumbnail occupies an inline box while remaining an OGAR object reference, never becoming text |
| **BIM / CAD viewport** | 3D/plan projection of the same objects | Same `FieldView`/ClassView spring; `bim-overlay` / `viewport-selection` views |

## §6 How `ogar-bim` lands

`ogar-bim` provides **semantic objects and ClassViews** — not a document
format:

```rust
#[ogar_class]
pub struct Wall {
    pub identity: Identity,
    pub name: LocalizedString,
    pub placement: Placement3,
    pub geometry: GeometryRef,
    pub properties: PropertySets,
    pub containment: Vec<ObjectRef>,
    pub documents: Vec<ObjectRef>,
}
```

```text
Wall.document-inline
Wall.property-table
Wall.drawing-symbol
Wall.viewport-selection
Wall.print-specification
Wall.openproject-link
```

An OpenProject document then contains:

```text
Work package: Install fire door

Affected object:
[ ObjectSlot:
    target = ogar://bim/door/3927
    class_view = "door.document-inline"
    field_mask = [name, fire_rating, status]
    wide_field_mask = [
        storey.name,
        space.name,
        documents.latest_drawing
    ]
]
```

The same door appears as: an inline chip in prose · a full property table · a
highlighted object in 3D · a symbol on a floor plan · a row in a
work-package list · a Typst specification block. **One identity, several
faces.** `ogar-bim` does not merely land beside OpenProject — it becomes
directly addressable from project descriptions, reports, drawings,
schedules, OCR imports, and the desktop.

## §7 The first vertical slice (operator-specified)

Do **not** start by building a general-purpose rich-text editor. Start
inside the OpenProject transcode:

1. **Put DocIr behind one rich-text field** — `WorkPackage.description`.
   Support ONLY: `Document / Section / Paragraph / Text / ObjectSlot`.
2. **Three attachable object classes:** `WorkPackage`, `User`, `Attachment`.
   Prove: `WorkPackage.description` contains ObjectSlot→User,
   ObjectSlot→another WorkPackage, ObjectSlot→Attachment.
3. **Two ClassViews each:** `User.inline` / `User.card`;
   `WorkPackage.inline` / `WorkPackage.summary`; `Attachment.inline` /
   `Attachment.preview`.
4. **Render the same document twice:** Askama HTML and Typst PDF — proving
   DocIr and ClassView are genuinely renderer-independent.
5. **Add the first BIM object** — not an entire IFC ontology; ONE
   `BimObject{identity, type, name, property_sets, geometry_ref,
   document_refs}` embedded in the description, rendered through both
   surfaces.

That one slice tests the whole doctrine:

```text
OpenProject model → OGAR identity → DocIr ObjectSlot
    → ClassView + masks → { HTML, PDF }
```

## §8 Grounding in shipped code (what already exists)

| Piece | Status | Where |
|---|---|---|
| `WideFieldMask` (wide RBAC/render masks, positions ≥ 64) | CODED | lance-graph contract + `ogar-a2ui-frame` (D-A2UI arc, #204/#205 correction) |
| Addressed askama fieldview (`data-field-pos`, `data-action-ordinal`, XSS-safe) | CODED | `ogar-render-askama::field_view` |
| LE wire frames (NodeDelta/ActionInvoke; RBAC before framing) | CODED | `ogar-a2ui-frame` |
| Observation IR + OCR retina | CODED | `ogar-doc-ir`, `ogar-from-docv1`, tesseract-rs doc.v1 |
| Generic `document` concept | MINTED `0x080B` | `ogar-vocab` (#216) + lance-graph mirror (#764) |
| Content-addressed immutable doc KV (Lance) | CODED | medcare-rs `document_kv` + `LanceDocumentKv` (#218/#224/#227) |
| ClassView registry (per-concept views) | CODED (single view per class today; **named multi-view per class is the gap**) | `ogar-class-view` |

Named gaps the slice must fill: `ObjectSlot`/`ResolutionMode`/`ogar://`
parsing · composition `DocNode` · `DocOp` · `FieldView` enum +
`ProjectionRenderer` trait · **named** ClassViews (`WorkPackage.inline` vs
`.summary` — today's registry is one view per class) · `ogar-render-typst`.

## §9 Gates & sequencing

- **IR gate:** `docs/OGAR-AS-IR.md` MUST be read before the composition
  types land (this is an IR-surface change — new lowering + new IR layer);
  the six IR-shape tests apply to `DocNode`/`ObjectSlot`/`FieldView`.
- **SurrealQL trap pre-flight** applies to any `ogar://` → storage lowering.
- **Renderer verification feed-in:** the 2026-07-20 editor-survey workflow
  (Typst incremental/PDF-A maturity, Blitz experimental status, Parley
  editing/IME state, in-house a2ui-paint audit) grades the §5 rows before
  Blitz/Parley/Vello work is scheduled; its verdict is recorded alongside
  this doc when it lands.
- **Kill-criteria:** if Typst cannot be generated from render nodes without
  arbitrary-syntax leakage, the Typst arm is demoted to export-only; if
  Parley inline-box editing is not mature, the precision canvas defers —
  neither kills the doctrine (Askama+HTMX alone proves the slice).
- **Verbatim-preservation rule:** the operator's formulations in §0–§7 are
  canon text — later edits regrade or append, never rewrite.
