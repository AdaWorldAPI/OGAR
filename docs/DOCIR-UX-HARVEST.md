# DocIr UX Harvest — what a decade of Papermerge's user-facing surface implies for the type shape

> **Source note.** Extracted from `papermerge/papermerge-core`
> (papermerge 3.6.0 — FastAPI + SQLAlchemy 2.0 async + Alembic + Celery/Redis
> + PostgreSQL, confirmed from `pyproject.toml`, not assumed), a mature
> document-management system. Ledger: `docs/DISCOVERY-MAP.md`
> `D-DOCIR-UX-HARVEST`. Status: **PROPOSAL** — Rust sketches below are design
> canon to gate, not a live change to `ogar-doc-ir`; each is a `doc.v2`-scale
> addition per the crate's own "adding a kind is a version bump, never a
> silent reshape" rule.
>
> **Scope, deliberately narrow.** This is the UX harvest only — the
> user-facing capabilities Papermerge's feature surface implies the *IR type
> shape* must be able to represent. It excludes everything infra-shaped that
> the same research surfaced: the search-index-as-idempotent-projection
> pattern, the folder-hierarchy + live-permission-walk design, the dead
> `checksum` columns, the audit-column mixin, the Celery task-queue
> boundary. None of those are `ogar-doc-ir` concerns — folders and
> permissions are archive-layer: facts-only `ActionDef`s in `ogar-vocab`
> (`document_actions.rs`, the just-landed W4 pattern — see
> `docs/DISCOVERY-MAP.md` `D-OGAR-DOC-LAYER`), executed by the consumer
> (`tesseract-paperless`/`paperless-kv`) — never a canon dependency inside
> this crate. Filed here only if a future archive-layer harvest wants them;
> not restated.
>
> One line: **`ogar-doc-ir` currently has no way to say "this page" — only
> "the page at this position" — and that one gap is what blocks every
> page-level capability a real DMS's users expect.**

## §0 The gap, read against the actual current type

`DocPage` (`crates/ogar-doc-ir/src/lib.rs:244-254`) carries a `number: u16`
and nothing else that survives a structural edit:

```rust
pub struct DocPage {
    pub number: u16,   // an ORDINAL, not an identity
    pub width: u32,
    pub height: u32,
    pub regions: Vec<Region>,
}
```

A page is addressed only by its position in `DocIr::pages: Vec<DocPage>`.
Delete page 2 and every later page's *identity* silently shifts — there is
no stable handle a consumer (the reasoning layer, a future UI, an archived
`spo_json` blob keyed by page) could hold across an edit. This is exactly
Papermerge's own starting point before it grew page-level features: a page
that exists only as a document's child, addressed by position.

## §1 Page identity + the copy-forward invariant (the one finding worth stealing whole)

Papermerge's actual mechanism, read from source, not the README:

- `Page` FKs to `document_version_id`, **not** to `Document`
  (`papermerge-core/papermerge/core/features/document/db/orm.py:129-142`).
  A page is not even an ownable resource type — permissions are checked
  against `('node', 'custom_field', 'document_type', 'tag')` only
  (`features/ownership/db/orm.py:49`); pages inherit their document's
  permissions, never carry their own.
- Every structural edit — delete, reorder, rotate, cut, move, extract
  (`features/page_mngm/db/api.py`) — mints a **whole new `DocumentVersion`**
  with fresh page rows pre-allocated 1..N (`features/document/db/api.py:555-592`),
  physically rewrites the underlying PDF, then **copies forward** the
  unaffected pages' already-recognized text and preview artifacts by an
  explicit old-page-id → new-page-id mapping
  (`reuse_ocr_data`, `page_mngm/db/api.py:206-225`) — it never re-runs OCR on
  a page that didn't change.
- A document that loses its last page is hard-deleted
  (`page_mngm/db/api.py:336-339, 416-419, 488-493`) — an empty document
  cannot exist.

**The invariant, not the implementation:** *a structural page operation
produces a new, immutable version of the containing document; every
unaffected page's already-harvested content is copied forward by explicit
id, never rederived.* That is directly portable to a byte-parity recognition
pipeline where re-running OCR is exactly the cost this whole stack exists to
avoid paying twice (the S-2 gate's own reasoning, one layer up).

### Proposed shape

```rust
/// Stable page identity, independent of position in `DocIr::pages`.
/// Never reused across a structural edit — a fresh page (including a page
/// carried forward unchanged into a new version) gets a fresh id; identity
/// survives only via the copy-forward mapping an editor keeps explicitly,
/// mirroring Papermerge's own old-id -> new-id table rather than any
/// implicit "same position = same page" assumption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageId(pub [u8; 16]); // ulid/uuid-shaped; generation is the producer's call

pub struct DocPage {
    pub id: PageId,          // NEW — the stable handle
    pub number: u16,         // unchanged: position within the CURRENT version
    pub width: u32,
    pub height: u32,
    pub regions: Vec<Region>,
}

/// A structural page operation, expressed over ids — never positions.
/// This is a description of an edit, not a mutator: `ogar-doc-ir` stays a
/// pure data crate, so the actual PDF/pixel rewrite is the executor's job —
/// facts declared as an `ogar-vocab::document_actions`-style `ActionDef`,
/// executed by the consumer (`tesseract-paperless`), the same facts/executor
/// split `persist_document` already uses — matching this crate's own "mints
/// nothing, carries facts not policy" charter.
pub enum PageOp {
    Delete { page: PageId },
    Reorder { page: PageId, before: Option<PageId> },
    Rotate { page: PageId, degrees: RotationQuarter }, // 90/180/270, never baked into bbox rails
    Extract { pages: Vec<PageId>, into_new_document: bool },
    Move { pages: Vec<PageId>, destination: DocumentRef, strategy: MoveStrategy },
}

pub enum MoveStrategy {
    /// Insert into the destination's existing page order at a position.
    Mix { at: Option<PageId> },
    /// Discard the destination's current pages entirely.
    Replace,
}

/// The copy-forward contract a `PageOp` executor owes: which recognized
/// content survives the edit, keyed by the OLD id, and where it lands.
pub struct CarriedForward {
    pub from: PageId,
    pub to: PageId,
    // fields (TypedField), region tree, and any archived spo_json for `from`
    // are the executor's to re-key onto `to` — NOT rerun.
}
```

`RotationQuarter` staying a first-class enum (not baked into the region
tree's `bbox` rails, the way Papermerge bakes rotation into the PDF content
stream at write time) is a deliberate divergence: this IR's rails are
recognizer-measured facts, and a stored rotation is presentation state, not
an observation — conflating them would mean re-deriving rotation from
already-rotated geometry on the next read.

## §2 Document type + typed-field schema (the metadata side of the same gap)

`TypedField { key, value, bbox, confidence }` (`lib.rs:173-182`) already
carries harvested facts, but nothing says which fields *should* exist for a
given kind of document, or what type they should parse as. Papermerge's
answer, read from source:

- A `CustomField` is global (unique `name`, a type discriminator) and
  attaches to a `DocumentType` through an ordered many-to-many join carrying
  `position` (`features/document_types/db/orm.py:11-19`,
  `features/custom_fields/db/orm.py:21-36`).
- The **value** is one JSONB column per `(document, field)`, with
  `value_text` / `value_numeric` / `value_date` / `value_datetime` /
  `value_boolean` all declared as Postgres `GENERATED` columns computed
  *from* that JSONB (`features/custom_fields/db/orm.py:58-98`), each
  independently indexed. Migration history shows this converged: an earlier,
  narrower `value_yearmonth` / `value_year` pair
  (`alembic/versions/cea868700f4e...py:22-25`, 2024-11) was tried and later
  dropped for this cleaner five-type scheme.

**The invariant:** one canonical value representation per field, plus a
small closed set of *typed readings* of it — not a `value_text`-only EAV
table that pushes parsing onto every consumer.

### Proposed shape

```rust
/// The closed set of typed readings a harvested field's raw string value
/// may be validated/parsed against — mirrors Papermerge's converged
/// five-type scheme, kept exhaustive for the same reason RegionKind is:
/// a consumer must be forced to handle a new kind at compile time, not fall
/// through a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Text,
    Numeric,
    Date,
    DateTime,
    Boolean,
}

/// One expected field on a named document type — the schema a producer's
/// harvest (or a review UI) can validate a `TypedField.key`/`value` against.
/// This crate still carries facts, not policy: `FieldSchema` describes what
/// is EXPECTED, it does not reject or coerce a `TypedField` that arrives
/// without a matching schema entry.
pub struct FieldSchema {
    pub key: String,
    pub kind: FieldKind,
    pub position: u16, // display/entry order, per Papermerge's ordered join
}

/// A named document category (e.g. "Invoice", "Lab Report") and the field
/// schema a document of that type is expected to carry. Assigned to a
/// document downstream of recognition (by a user, or a future classifier) —
/// NOT produced by a retina, so this does not live on `DocIr` itself; it is
/// the archive layer's concern to attach one to an ingested document and
/// use it to validate/guide review of that document's `fields`.
pub struct DocumentType {
    pub name: String,
    pub schema: Vec<FieldSchema>,
}
```

Kept deliberately separate from `DocIr`: a document's *type* is an archive
concept assigned after ingestion, not a perceptual fact a retina observes —
consistent with this crate's own "mints nothing, no canon dependency"
charter. `FieldKind`/`FieldSchema` are the reusable shape; the just-landed
`document_actions.rs` precedent (its own doc comment: a *separate* table
from `ocr_actions.rs` specifically so one consumer's hot-plug never
entangles another's) suggests `DocumentType` would follow the same split —
mint in `ogar-vocab`, execute in the consumer — but whether it needs its
own `ActionDef` table or is purely a `tesseract-paperless` concept is a
question for that same council gate, not this document's to settle.

## §3 Tags — named, not modeled (deliberately thin)

Papermerge tags are a plain many-to-many label on a node
(`nodes_tags` join, referenced by the search-trigger's
`CONCAT_WS`-built vector — `search_index_functions.sql:131-136`) with no
further structure: no hierarchy, no per-tag schema, just a name and a
color. Worth naming because it is the cheapest possible organizational
primitive a UX offers, and it costs nothing to keep it that thin —
`Vec<String>` (or `Vec<TagId>` once a color/identity is worth interning) at
the archive layer is the whole feature. Not sketched further here: there is
no invariant to steal beyond "don't over-model this."

## §4 What this does NOT decide

- Whether `PageId`/`FieldKind`/`FieldSchema`/`DocumentType` land in
  `ogar-doc-ir` itself, a new module, or an `ogar-vocab`-side `ActionDef`
  table mirroring `document_actions.rs` (there is no separate `ogar-doc`
  crate — the council landed persistence facts inside `ogar-vocab` instead,
  per `docs/DISCOVERY-MAP.md` `D-OGAR-DOC-LAYER`).
- Generation scheme for `PageId` (ulid vs uuid vs a facet-register-derived
  id) — deliberately left as "the producer's call" above.
- Whether `PageOp` execution (the actual PDF/pixel rewrite + copy-forward)
  belongs in `tesseract-paperless`, a new `ogar-vocab::document_actions`-
  style `ActionDef`, or neither yet — this document proposes the SHAPE a
  future executor would consume, not the executor.
- Any UI/API surface for these — Papermerge's REST shape (list-of-ids in,
  list-of-ids out, `Option`-typed "source may vanish" response fields) is
  a reasonable precedent if/when this stack grows an HTTP surface beyond
  Askama-rendered pages, but that is a different document's job.

These are open on purpose — this harvest names the capability and the type
shape it implies; ratifying where it lands is the 5+3-council-style gate
this crate's other extensions (the composition layer, `docs/DOCIR-COMPOSITION-LAYER.md`)
already went through.
