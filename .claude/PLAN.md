# OGAR Roadmap — meticulous sprint-by-sprint mapping

> Sprints are sized to be a 1-PR slice each. Each sprint has explicit
> **deliverables**, **acceptance**, **dependencies**, **risks**. The
> roadmap is append-only at the *sprint* level; corrections become new
> sprints, not edits to old ones.

## Status legend
- ✅ done
- 🟡 in flight
- ⏳ next
- ⬜ planned

---

## Sprint 0 — v0 vocabulary scaffold ✅

**Deliverables**
- `crates/ogar-vocab` — canonical Rust IR types (Class, Association,
  EnumDecl, StoreAccessor, Attribute, Scope, Callback, Validation, Mixin).
- `crates/ogar-ontology` — prefix conventions + identity helpers.
- `vocab/ogar.ttl` — Turtle/RDF canonical vocabulary.
- `vocab/ogar.surql` — SurrealQL DDL projection.
- `docs/ARCHITECTURE.md` — full layer-stack writeup.

**Acceptance**
- `cargo check --workspace` clean (Rust 1.85 / edition 2024).
- `cargo test --workspace` green.
- Repo pushed to `github.com/AdaWorldAPI/OGAR`.

**Commits**
- `d251fdd` — bootstrap README via Contents API.
- `fbf0cf0` — v0 scaffold (10 files, 1 tree commit).

---

## Sprint 1 — `ogar-emitter` trait + ruff_ruby_spo adapter ⏳

**Goal**: prove the canonical IR by lifting a real producer onto it,
keeping the existing producer code working unchanged.

**Deliverables**
- `crates/ogar-emitter/` — defines `OgarEmitter` trait + `Triple` type:
  ```rust
  pub trait OgarEmitter {
      fn emit_class(class: &ogar_vocab::Class, prefix: &str) -> Vec<Triple>;
      fn emit_association(assoc: &ogar_vocab::Association, owner: &str, prefix: &str) -> Vec<Triple>;
      // ... one method per top-level vocab type
  }
  ```
- `crates/ogar-from-ruff/` — adapter crate that consumes
  `ruff_ruby_spo::RubyClass` and produces `ogar_vocab::Class`. Pure
  `From<&RubyClass> for Class` impl, no new logic.
- Round-trip test: a real-OP `WorkPackage` model → OGAR IR → triples
  matching the existing `op-codegen-pipeline` output (within the
  fields that overlap).

**Acceptance**
- `cargo test -p ogar-from-ruff` passes.
- `cargo test -p ogar-emitter` passes.
- The triples emitted match `op-codegen-pipeline`'s current output
  shape for at least one real model.

**Dependencies**: Sprint 0.

**Risks**
- Field naming drift between `RubyClass` and `ogar::Class` — handled
  by keeping the adapter pure `From` with no semantics change.
- `body_source` opaqueness — fields stay verbatim strings on the OGAR
  side too.

---

## Sprint 2 — `ogar-to-surrealql` consumer ⬜

**Goal**: prove the bidirectional path — OGAR IR → SurrealQL DDL.

**Deliverables**
- `crates/ogar-to-surrealql/` — implements `OgarEmitter` to produce
  SurrealQL `DEFINE TABLE` / `DEFINE FIELD` statements.
- Output validates against the schema in `vocab/ogar.surql` (round-trip
  through SurrealDB's parser).

**Acceptance**
- For a sample `ogar::Class`, the emitted SurrealQL parses cleanly with
  SurrealDB's own parser (vendored dep or shell-out test).
- Bidirectional probe: read a SurrealQL DDL file → parse → emit OGAR IR
  → emit SurrealQL → compare with input (lossless except formatting).

**Dependencies**: Sprint 1.

---

## Sprint 3 — `ogar-to-postgres` consumer ⬜

**Goal**: prove multi-target emission. Rails migration generation
becomes a projection of the canonical IR.

**Deliverables**
- `crates/ogar-to-postgres/` — implements `OgarEmitter` to produce
  PostgreSQL DDL (`CREATE TABLE`, `ALTER TABLE … ADD COLUMN`).
- Rails-flavor variant: emits the `db/migrate/<n>_create_<table>.rb`
  Ruby DSL form rather than raw SQL.

**Acceptance**
- DDL output runs against a real PostgreSQL instance (integration
  test) for at least one real-OP model.
- Migration form is `rails db:migrate`-compatible.

**Dependencies**: Sprint 1.

---

## Sprint 4 — `ogar-python` producer ⬜

**Goal**: prove cross-language. Odoo and Django emit the same IR as
Rails AR.

**Deliverables**
- `crates/ogar-python/` — extracts `class Foo(models.Model)` declarations
  from Python source via `rustpython-parser` (or `libcst` via subprocess).
- Maps Odoo `fields.Many2one` → `Association(BelongsTo)`, `One2many` →
  `HasMany`, `Many2many` → `HasAndBelongsToMany`.
- Maps Django `ForeignKey` / `OneToOneField` / `ManyToManyField`
  identically.

**Acceptance**
- A sample Odoo `sale.order` model and a sample Django model both
  produce well-formed `ogar::Class`.
- Sale-order example output matches Rails-WorkPackage output in
  *structure* (different field names + class name, same triple shape).

**Dependencies**: Sprint 1.

---

## Sprint 5 — `ogar-extensions/odoo` ⬜

**Goal**: capture Odoo-specific shapes that don't fit base OGAR
vocabulary, without polluting the canonical types.

**Deliverables**
- `crates/ogar-ext-odoo/` — defines `ComputedField`, `Delegation`
  (`_inherits`), `Workflow` (state machine).
- Producer plug-in into `ogar-python` so Odoo source emits these
  extensions alongside the canonical `Class`.
- `vocab/ogar-ext-odoo.ttl` — the extension vocabulary in RDF.

**Acceptance**
- Odoo source with computed fields + `_inherits` + workflow round-trips
  with all three extension types captured.

**Dependencies**: Sprint 4.

---

## Sprint 6 — `lance-graph-ontology` cache integration ⬜

**Goal**: wire OGAR triples into the runtime ontology cache so
`lance-graph-planner` and `lance-graph-callcenter` can resolve OGAR
identities at sub-microsecond cost.

**Deliverables**
- Loader: read OGAR triples from lance-graph dataset, build in-memory
  cache keyed by `ogar/Class` identity → class metadata struct.
- Cache invalidation on ontology version bump (Lance `versions()` watch).

**Acceptance**
- Cache lookup for a known class identity is <1 µs after warm-up.
- Cache rebuilds correctly when a new ontology version lands.

**Dependencies**: Sprints 1, 2.

---

## Sprint 7 — `lance-graph-callcenter` actor runtime skeleton ⬜

**Goal**: the smallest possible actor-per-class runtime. Routes one
message type (`Find { id }`) to the right `ogar/Class` actor and
returns the row.

**Deliverables**
- Actor trait + supervisor scaffold (probably built on `tokio` + an
  existing actor crate — Sprint 6 brutal review will resolve which).
- Class registration via `ogar/Class` triples (no manual wiring).
- Message dispatch routing through the ontology cache.

**Acceptance**
- A `Find { id: 42 }` message for `ogit-op/WorkPackage` is routed to
  the registered actor, reads the row, returns it.
- Actor supervisor restarts a failed actor without losing other
  classes' state.

**Dependencies**: Sprint 6.

---

## Sprint 8 — cross-system Odoo ↔ OpenProject query proof ⬜

**Goal**: end-to-end demonstration of the architecture: Odoo
`sale.order.work_package` resolves into the OpenProject `WorkPackage`
via shared OGAR prefix-radix routing.

**Deliverables**
- A single lance-graph dataset holding both Odoo and OP triples.
- A query (in SurrealQL frontend or direct API) that traverses an Odoo
  sale-order to its linked work-package in one hop.

**Acceptance**
- The traversal completes in one index lookup (no ETL, no bridge
  table).

**Dependencies**: Sprints 4, 7.

---

## Cross-sprint principles

1. **Each sprint = one PR.** No sprint lands as multiple PRs; if a
   sprint is too big, split it into sub-sprints first.
2. **Tests gate merge.** No sprint merges with red tests.
3. **No new top-level prefixes.** Extensions go under
   `ogar-extensions/<lang>/` or `ogit-<app>/`.
4. **Vocab files are the contract.** Code crates implement; `vocab/*.ttl`
   defines.
5. **EPIPHANIES log every cross-cutting finding.** See
   `.claude/board/EPIPHANIES.md`.

## Out of scope (deliberate)

- A full BEAM-grade hot-code-reload system. Sprint 7 is *skeleton*.
- A SurrealQL parser. Sprint 2 *emits*; reading SurrealQL is a future
  sprint if needed.
- A migration generator for Rails. Sprint 3 *emits*; running migrations
  is the consumer's job.
- Validation rule grammar lift. C17-equivalent sprint for OGAR
  validation will come after Sprint 5.
