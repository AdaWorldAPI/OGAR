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

## Sprint 1 — `ogar-emitter` trait + vocab/ontology hardening 🟡

**Goal**: ship the emitter trait + a default SPO triple implementation,
plus apply the brutal-review correctness fixes to vocab/ontology.

**Deliverables**
- `crates/ogar-emitter/` — `OgarEmitter` trait + `Triple` type +
  `TripleEmitter` default implementation. 13 tests covering
  rdf:type emission, association subgraphs, scope_source capture,
  enum variant name/value separation, duplicate-callback indexing,
  mixin-through-class-identity, scope_predeclaration, collection
  callbacks (before/after_add/remove), Unknown-fallback discipline.
- `crates/ogar-vocab/` — `#[non_exhaustive]` on every public struct
  + enum (Class, Association, EnumDecl, StoreAccessor, Attribute,
  Scope, Callback, Validation, AssociationKind, Language). Added
  `Association.scope_source` for Rails `has_many :x, -> { ... }`,
  Django `limit_choices_to`, Odoo `domain=[...]`. Constructors
  (`Class::new(name)`, `Association::new(kind, name)`, etc.) for
  external crates blocked by `#[non_exhaustive]`.
- `crates/ogar-ontology/` — `class_identity_versioned()` for hot-
  reload addressing, `tenant_prefix()` for multi-tenancy. Identity-
  collision fixes for Enum/Store/Attribute on shared column names
  (distinct `::enum::` / `::store::` / bare-attribute namespaces).
- `vocab/ogar.ttl` — `ogar:scopeSource` predicate added.
- `vocab/ogar.surql` — `scope_source` field added to ogar_association.
- `docs/IDENTITY-MAPPING.md` — carved semantics for the Role enum,
  Identity struct, path syntax variants, edge cases. The
  drift-prevention contract.

**Acceptance**
- `cargo check --workspace` clean.
- `cargo test --workspace` green (30+ tests).
- All brutal-review CB1 correctness fixes landed (subject collisions
  resolved, Unknown fallback explicit, mixins routed through
  class_identity).

**Dependencies**: Sprint 0.

**Deferred to follow-up sprints**
- `crates/ogar-from-ruff/` (Ruby AR adapter) → split to Sprint 1f
  due to cross-repo `ruff_ruby_spo` dependency.
- API refactors from brutal-review CB2 (streaming sink, EmitContext,
  Cow Triple) → deferred to Sprint 1g.
- Perf wins from CB3 (with_capacity, owner_id reuse, #[inline]) →
  partially landed (with_capacity in emit_class); rest deferred to
  Sprint 1g.

---

## Sprint 1c — `Identity` struct + parser + serializers ⏳

**Goal**: implement the canonical `Identity` IR (per
`docs/IDENTITY-MAPPING.md`) so identity strings round-trip
losslessly across compact / pathlike / Elixir / dotted forms. Each
session writes the syntax that feels intuitive; the parser
normalizes.

**Deliverables**
- `crates/ogar-ontology/` — add `Identity` struct + `Role` enum (full
  variants from MAPPING.md §2). Replace the three free-function
  helpers (`class_identity`, `field_identity`, `association_identity`)
  with `Identity::class()`, `Identity::association()`, etc.
- `Identity::parse(&str) -> Result<Identity, ParseError>` accepting
  all five syntax variants from MAPPING.md §3.
- Serializers: `to_canonical()`, `to_compact()`, `to_pathlike()`,
  `to_dotted()`. Each round-trips: `parse(x.to_pathlike()) == x` for
  any valid Identity x.
- ROLE_KEYWORDS table from MAPPING.md §7.1 as a `phf` lookup.
- Property-tests (proptest crate) over the round-trip invariant.

**Acceptance**
- Round-trip property tests pass for ≥1000 random identities.
- Parser accepts every example string in MAPPING.md §3.
- Cross-syntax equivalence:
  `parse("ogit-op/WorkPackage->project") ==
   parse("ogit-op::WorkPackage::memberof::project") ==
   parse("OgitOp.WorkPackage.belongs_to.project")`.

**Dependencies**: Sprint 1, `docs/IDENTITY-MAPPING.md`.

---

## Sprint 1d — Elixir module-path serializer + parser variant ⬜

**Goal**: native Elixir/Ecto-style identity strings. PascalCase
module paths, dotted role words, `use` instead of `include`,
`many_to_many` instead of HABTM/through.

**Deliverables**
- `Identity::to_elixir() -> String` per MAPPING.md §3.3.
- Parser variant accepting `Acme.OgitOp.WorkPackage.has_many.line_items`
  and pinned form `OgitOp.WorkPackage.v3.has_many.line_items`.
- Prefix PascalCase normalization (`ogit-op` ⇆ `OgitOp`,
  `ogar-extensions` ⇆ `OgarExtensions`).
- Bidirectional tests covering the four canonical Ecto association
  macros + `use M` mixin.

**Acceptance**
- Round-trip with Sprint 1c canonical form.
- Examples from MAPPING.md §3.3 all parse and re-emit identically.

**Dependencies**: Sprint 1c.

---

## Sprint 1e — OTP `:via`-tuple emission helper ⬜

**Goal**: produce Erlang-wire-compatible identity tuples for the
eventual Elixir-side companion of `lance-graph-callcenter`. Same
`Identity` struct can be serialized as an Erlang term for use with
`{:via, Registry, ...}` GenServer naming.

**Deliverables**
- `Identity::to_erlang_via(registry: &str) -> String` emitting:
  ```
  {:via, OgitOp.Registry, {OgitOp.WorkPackage, :has_many, :line_items}}
  ```
- Optional: `Identity::to_erlang_term()` for raw term form (for
  log/trace).
- Smoke test against a hand-written Elixir parser fixture (no
  Elixir build dependency — the test verifies the string matches
  Elixir's syntax exactly).

**Acceptance**
- Output strings parse cleanly with Elixir's `Code.string_to_quoted/1`
  (verified via fixture string-match).

**Dependencies**: Sprint 1d.

---

## Sprint 1f — `ogar-from-ruff` adapter (ruff_ruby_spo → ogar::Class) ⬜

**Goal**: lift the openproject-nexgen-rs C17a-c stable shape
(`RubyClass`, `AssociationDecl`, `ScopeDecl`, etc.) into `ogar::Class`
via a pure `From` impl. No new logic — just shape mapping.

**Deliverables**
- `crates/ogar-from-ruff/` with `From<&RubyClass> for ogar_vocab::Class`.
- Git dependency on `openproject-nexgen-rs` (subdir crate via
  `package = "ruff_ruby_spo"` Cargo hint).
- Round-trip test: real OP WorkPackage model → RubyClass → OGAR IR
  → triples (via TripleEmitter) → identity match against the
  existing `op-codegen-pipeline` output.

**Acceptance**
- `cargo test -p ogar-from-ruff` passes.
- Triple set for WorkPackage matches the C17c output for the
  overlapping fields.

**Dependencies**: Sprint 1, Sprint 1c.

---

## Sprint 2 — Odoo carve-out + vocab gaps from brutal-review cycle 2 🟡

**Goal**: lift the five BO1 gaps + the BO2 architectural decisions
identified during the Odoo brutal-review cycle. Ships
`docs/ODOO-TRANSCODING.md` (the carve-out doc) + vocab extensions
covering full Odoo coverage.

**Deliverables**
- `docs/ODOO-TRANSCODING.md` — 18-section carve-out doc with 13
  non-negotiable rules. Discovery algorithm, field-type mapping
  table, decorator mapping, state-machine shape, `_inherit`
  resolution algorithm, registered-prefix table sketch.
- `crates/ogar-vocab` additions:
  - `AttributeOptions` struct (required/default/translate/tracking/
    digits/size/groups/company_dependent/...).
  - `EnumSource` enum (Static / Computed / Add) replacing flat values.
  - Class metadata: description, record_order, rec_name,
    check_company_auto, log_access, auto_create_table,
    abstract_model, transient, declared_in_module, source_version,
    computed_fields, methods.
  - `ComputedField` struct (lifted from ext to base).
  - `MethodDecl` struct + `MethodKind` enum + `RecordSemantics` enum.
  - Association: `ondelete`, `auto_join`, `context_source`,
    `check_company`, `delegate`.
- `vocab/ogar.ttl` — RDF predicates + OWL classes for all new types.
- Cross-references in `docs/IDENTITY-MAPPING.md`.

**Acceptance**
- `cargo check --workspace` clean.
- `cargo test --workspace` green.
- Sprint 1 tests still pass after vocab additions.

**Dependencies**: Sprint 1.

---

## Sprint 2.5 — `vocab/ogar-bridges.ttl` cross-vocabulary mappings ⬜

**Goal**: implement the SKOS-design-paper principle "defer to
existing vocabularies" by curating `owl:equivalentProperty` and
`skos:exactMatch` mappings between OGAR terms and existing W3C /
community vocabularies (PROV, Dublin Core, FOAF, SKOS).

**Deliverables**
- `vocab/ogar-bridges.ttl` with:
  ```turtle
  ogar:declaredIn owl:equivalentProperty prov:wasDerivedFrom .
  ogar:description owl:equivalentProperty dc:description .
  ogar:MemberOf skos:exactMatch ruby:belongs_to , odoo:Many2one ,
                                 ecto:belongs_to , django:ForeignKey .
  ogar:OwnsMany skos:exactMatch ruby:has_many , odoo:One2many ,
                                 ecto:has_many .
  ...
  ```
- A doc section in `docs/IDENTITY-MAPPING.md` referencing the
  bridges file.
- The "transitivity trap" warning from Freytag BA §6.2.3:
  cross-vocab `skos:exactMatch` chains can produce false
  equivalences. Document the limit; recommend `skos:closeMatch`
  where semantics differ subtly.

**Dependencies**: Sprint 2.

---

## Sprint 2.6 — `crates/ogar-conformance` fixture corpus ⬜

**Goal**: build the producer-conformance test gate identified as
BO2 #2. Every producer (`ogar-from-ruff`, `ogar-python`, future
`ogar-sql-ddl`) runs this suite as a `cargo test` gate. Drift
detection by construction.

**Deliverables**
- `crates/ogar-conformance/` with `fixtures/` directory containing
  per-Role subdirectories: `member_of/`, `owns_many/`, etc.
- Each fixture: source snippet + expected `ogar_vocab::Class` IR +
  expected triples via `TripleEmitter`.
- `assert_conforms!(producer, fixture_dir)` macro.
- Initial fixture set: 5 fixtures per role (Ruby AR, Odoo, Django,
  Ecto, SQL DDL) for the 4 core associations + Include + Attribute
  + Enum + Validation + Callback.

**Acceptance**
- Fixtures parse cleanly.
- The macro fails loudly on any mismatch with explanatory diff.

**Dependencies**: Sprint 2.

---

## Sprint 2.7 — Registered-prefix table impl + producer integration ⬜

**Goal**: implement the registered-prefix table sketched in
ODOO-TRANSCODING.md §14 so cross-language identity collisions are
impossible by construction (per BO2 #1).

**Deliverables**
- `ogar-ontology::REGISTRY` static table mapping prefix → source
  language.
- `validate_prefix_for_lang(prefix, lang) -> Result<(), PrefixError>`.
- All producer crates call this before emitting any triples.
- Error path: clear messages distinguishing "unregistered prefix"
  from "language mismatch".

**Dependencies**: Sprint 2.

---

## Sprint 3 — Action vocabulary + adapter trait + SPO + TeKaMoLo ⏳

**Goal**: lift the SECOND ingestion arm — ERP transactions, actions,
business rules, hand-rolled Odoo business logic — into OGAR IR via
the `Action` vocabulary with full SPO+TeKaMoLo annotation. Plus
ship the `Adapter` trait as the data-side counterpart.

**Deliverables**
- `docs/ADAPTERS-AND-ACTORS.md` (already in Sprint 2 PR) — the
  canonical carve-out for this sprint.
- `crates/ogar-vocab/`: add `Action` struct + `ActionSubject` enum +
  `TemporalSpec` / `KausalSpec` / `ModalSpec` / `LokalSpec` types.
  Stay in base vocab — both data + behavior IR live together.
- `crates/ogar-adapter/`: `Adapter` trait + `NibleHHTL` lookup-table
  type + `TargetForm` struct. Static lookup-table semantics; no
  conditional logic.
- `crates/ogar-emitter/`: emit `Action` triples with full SPO+
  TeKaMoLo annotation. Add `emit_action()` next to existing
  `emit_callback()` / `emit_method()`.
- `vocab/ogar.ttl`: add the action vocabulary (`ogar:Action`,
  `ogar:actionSubject` + 6 more, plus enumeration classes
  `ogar:ActionSubject` / `ogar:TemporalSpec` / `ogar:ModalSpec`).

**Acceptance**
- `cargo check --workspace` clean.
- `cargo test --workspace` green (~40 tests after additions).
- For one Odoo example (`def action_confirm` on `sale.order`),
  the producer pipeline emits both:
  - One `MethodDecl{kind: CrudOverride}` (syntactic capture)
  - One `Action` with full SPO+TeKaMoLo (pragmatic capture)
- For one Rails example (`before_save :touch_parent`), same
  duality.

**Dependencies**: Sprint 2.

---

## Sprint 3.5 — `OdooAdapter` HHTL implementation ⬜

**Goal**: first concrete `Adapter` impl. The `OdooAdapter` is a
static HHTL with leaves mapping every canonical OGAR concept to
its Odoo-side form. Per ADAPTERS-AND-ACTORS doc §2.

**Deliverables**
- `crates/ogar-adapter/src/odoo.rs` — `OdooAdapter` impl.
- HHTL leaves for: class-name aliasing, field-name renames,
  decorator → role mappings, action-predicate translations,
  modal/temporal qualifier renames.
- Round-trip tests: any Odoo source identity → canonical → Odoo
  source identity is identity.
- Composition test: Odoo identity → canonical → Rails identity
  yields the right Rails form for at least 5 corresponding
  model pairs.

**Dependencies**: Sprint 3.

---

## Sprint 3.6 — `RailsAdapter` HHTL implementation ⬜

**Goal**: second concrete `Adapter` impl, validating that the
HHTL pattern generalizes.

**Deliverables**
- `crates/ogar-adapter/src/rails.rs` — `RailsAdapter` impl.
- HHTL leaves for the Rails canonical concepts (already
  documented in IDENTITY-MAPPING.md §3 + §7).
- Round-trip + composition tests.

**Dependencies**: Sprint 3.

---

> **STRATEGIC CORRECTION (2026-06-04).** Sprints 5–7 below were
> written assuming OGAR builds the lance-graph stack. It already
> exists upstream (`lance-graph-contract`, `lance-graph-ontology`
> with Odoo support, `lance-graph-callcenter`). Per
> `docs/LANCE-GRAPH-INTEGRATION.md`, OGAR is a **`SchemaSource`
> producer** into the existing `OntologyRegistry`, not a stack
> builder. The revised intent is annotated inline below; the full
> rationale is in the integration doc.

## Sprint 4 — SoA implementation: `ogar-vocab-soa` ⬜
> REVISED: narrow to Arrow conversion only where the registry doesn't
> already provide it; prefer feeding `MappingProposal` directly.

**Goal**: implement Apache Arrow RecordBatch schemas + bidirectional
conversions for the OGAR vocab types. Per `docs/SOA-IMPLEMENTATION.md`.

**Deliverables**
- `crates/ogar-vocab-soa/` — RecordBatch schema constants for
  `Class` and `Action`; ArrayBuilder-based conversions both ways.
- Nested ListArray handling for `associations` / `enums` /
  `scopes` / `callbacks` / `computed_fields` / `methods` /
  `validations`.
- Property tests via proptest: `classes → batch → classes` is
  identity for ≥1000 random classes.

**Dependencies**: Sprint 2.

---

## Sprint 4.5 — `ogar-adapter-surrealql`: bidirectional DDL ⬜

**Goal**: implement the SurrealQL adapter per R4 finding (depend
on `surrealdb-core::sql::parse`). Both directions supported.

**Deliverables**
- `crates/ogar-adapter-surrealql/` with:
  - `parse_surrealql_ddl(input) -> Vec<Class>` using
    surrealdb-core's parser.
  - `emit_surrealql_ddl(classes) -> String` reverse direction.
  - `DEFINE TABLE`, `DEFINE FIELD`, `DEFINE INDEX`, `DEFINE EVENT`
    coverage minimum.
- Property test: `parse(emit(parse(x))) == parse(x)` for arbitrary
  well-formed input.
- Pinned surrealdb-core version; migration path noted to
  `surrealdb-parser` + `surrealdb-ast` when crates.io-published.

**Dependencies**: Sprint 4.

---

## Sprint 5 — `ogar-to-proposal`: SchemaSource impl (REVISED) ⬜
> REVISED: NOT "build SoA integration" — that exists. Implement
> `impl SchemaSource for OgarSource` + `From<ogar::Class> for MappingProposal`
> (Class→Schema, Association→LinkSpec, Attribute→SemanticType+Marking).
> Resolve the `&'static str` impedance (intern vs owned vs proposal-only).
> Land one real OpenProject WorkPackage into the existing OntologyRegistry.

**Goal**: wire the contract layer (NiblePath identity routing,
Lance versioning) over SoA RecordBatches.

**Deliverables**
- Identity column dictionary encoding (segment-level NiblePath
  shared dictionary).
- Lance dataset write path with v2 manifest paths from day one.
- Append batching policy: ≥1 message/sec OR ≥100 messages/batch.
- Cleanup policy: retain "frozen" ontology versions via tags;
  cleanup unreferenced versions >1h old.

**Dependencies**: Sprint 4.

---

## Sprint 6 — register OGAR proposals into existing OntologyRegistry (REVISED) ⬜
> REVISED: NOT "build cache" — the 47KB Lance dictionary cache exists.
> Register OGAR proposals; add an `hydrate_ar` / OGAR-TTL hydrator to the
> existing hydrator set (alongside hydrate_odoo). OWN the integration test
> that a Lance version bump fires cache invalidation (upstream trigger is
> wired but undertested).

**Goal**: per Sprint 6 placeholder above, but explicitly read
RecordBatches from the contract layer. Cache invalidation via
Lance `versions()` watch.

**Deliverables**
- Read path: scan Class RecordBatches, project identity column,
  build ontology cache.
- Watch path: Lance manifest watcher → cache invalidation event.
- <1µs cached lookup target.

**Dependencies**: Sprint 5.

---

## Sprint 7 — `ogar-runtime`: Ractor + Kanban (RENAMED) ⬜
> RENAMED: `lance-graph-callcenter` already exists upstream (ExternalMembrane
> /Phoenix/pgwire) — name collision. OGAR's actor-per-class runtime is
> `ogar-runtime`. The Kanban mailbox is genuinely unbuilt upstream (zero code
> matches) — it IS the "kanban" in surrealQL>kanban<lance-graph. Evaluate
> whether ActionInvocation is a projection over the existing callcenter
> cognitive-event ledger rather than a new store.

**Goal**: BEAM-style actor runtime per R3 verdict (Ractor),
organized around Kanban-bounded mailboxes per `SOA-IMPLEMENTATION.md`
§5.

**Deliverables**
- `crates/lance-graph-callcenter/` with:
  - `ClassActor` trait built on Ractor.
  - `KanbanMailbox<M>` — bounded WIP + pull + backpressure.
  - Class registration from ontology triples (no manual wiring).
  - SPO+TeKaMoLo action dispatch routing through ontology cache.
  - Cascade routing: actor emits `RecordBatch` of downstream
    actions; receiving actor pulls when its WIP allows.
- Default WIP=1024 per mailbox; configurable per-class via
  `ogar:mailboxCapacity` triple.

**Dependencies**: Sprints 5, 6.

---

## Sprint 7.5 — End-to-end SoA performance gate ⬜

**Goal**: prove the full SoA flow performs under target latency.

**Deliverables**
- Benchmark: SurrealQL DDL → ogar-vocab-soa → lance-graph → actor
  dispatch in <10ms p99 (cold cache: <50ms).
- Memory benchmark: 100k classes + 1M actions fit in <500MB
  RecordBatch arena.
- Documented as `crates/benchmarks/`.

**Dependencies**: Sprint 7.

---

## Sprint 1g — API + perf refactor (from brutal-review CB2/CB3) ⬜

**Goal**: apply the deferred API and performance fixes.

**Deliverables**
- `OgarEmitter` becomes a `&mut self` sink trait:
  ```rust
  pub trait OgarEmitter {
      fn emit(&mut self, item: Emit, ctx: &EmitContext);
  }
  ```
- `EmitContext<'a>` carries prefix/tenant/owner.
- `Triple<'a>` uses `Cow<'a, str>` + `&'static str` for predicates.
- `Vec::with_capacity` everywhere; `#[inline]` on hot helpers.
- Sink-API benchmark vs current Vec-API: target ≥30% allocation
  reduction on a 100-class corpus.

**Dependencies**: Sprint 1c.

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
