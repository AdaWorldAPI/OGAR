# INTEGRATION MAP — how the substrate composes across repos

> **Status: LIVING MAP** (2026‑06‑09). Companion to `DISCOVERY-MAP.md` —
> the discovery map indexes *what was found*; **this maps *how it
> composes*: layers, seams, phases, gates.** Implementation follows this
> map; the map precedes the code (the "document before it dilutes"
> mandate).
>
> **How to read.** Same grading as the discovery map: `[G]` grounded
> (theorem / shipped code / measurement), `[H]` hypothesis **with a named
> test**, `[S]` speculative (catalog only). Code-existence uses the
> runtime‑archaeologist scale: **CODED** (file:line), **CLAIMED** (prose
> or module-doc only), **ABSENT** (a valid finding). Provenance marks:
> `[per rt]` = runtime-session-owned receipt; `[per xs]` = verified by
> the parallel session's first-hand read (CCA2A feedback), not re-read
> here.
>
> **Cross‑PR caveat** (same as the map's `SYN` legend): rows citing
> lance‑graph plans/boards reference `/home/user/lance-graph` state at
> main `62bca5e` (post **#480**); OGAR rows reference this branch.

---

## 0. The one picture

```
SOURCES      Odoo 17 (Python)        OpenProject (Ruby/Rails)      SurrealQL DDL        TTL/RDF    ClickHouse DDL   Elixir/HIRO
               │ ogar-python [ABSENT]   │ ruff_openproject [G per xs]  │ parse walk [H]      │ [G]       │ [G]            │ scaffold [G]
               ▼ (Track O3)             ▼ SPO → op-codegen-projection  ▼ (Track O2)         ▼           ▼                ▼
PRODUCERS ───────────────────────────────────────────────────────────────────────────────────────────────────────────────
                              ╔════════════════════ OGAR IR (the spine) ════════════════════╗
THINK arm     Class { attributes, associations (4× AR kinds), enums, scopes, mixins,
              parent: Option<Identity>  ←  "subClassOf == SUPERVISION EDGE" }
DO arm        ActionDef (separate SPO node, object_class → Class) + ActionInvocation (state: ActionState)
MEMBRANE      KausalSpec { StateGuard | LifecycleTrigger | Depends{paths} }  ← the only place domain
              workflow survives IR flattening (OGAR-AST-CONTRACT §3)
IDENTITY      Identity (NiblePath, class-side)  ←─ registry mint (entity_type ↔ NiblePath, bijective) ─→
              NodeGuid (128-bit UUIDv8, instance-side, lance-graph-contract::identity)  [G] #480
                              ╚═════════════════════════════════════════════════════════════╝
ADAPTERS      emit_surrealql_ddl [G] · ogar-adapter-ttl [G] · ogar-adapter-clickhouse-ddl [G]
              · ogar-knowable-from (vart-backend) [G] · parse_surrealql_ddl walk [H scaffold]
CONTRACT      lance-graph-contract: TripletProjection · SoaEnvelope · ClassView/FieldMask · kanban
              (KanbanColumn/KanbanMove/ExecTarget) · NodeGuid · CollapseGateEmission (Batons)
RUNTIME       ractor: generic state_machine (OGAR-agnostic) + OGAR/Rubicon binding [R1 unverified]
              · MailboxSoA columns (edges/meta/qualia/entity_type) · supervision routing by NiblePath
              prefix (is_ancestor_of = one bit-shift) · commit_event sole-writer membrane
STORAGE       Lance (DatasetVersion = frame; per-row cycle stamp = delta) · SurrealDB
              (TableDefinition::new_for_ddl → ToSql; kv-lance) · consumer stores via EntityKey(&[u8])
CONSUMERS     openproject-nexgen-rs · woa-rs (Odoo) · MedCare-rs (HIPAA, label-free) · smb-office-rs · q2
```

The Firewall (ADR‑022/023) holds at every layer crossing: **no
serialization in the hot path** — inter‑mailbox state is Batons
(`(u16 target, CausalEdge64)`); the IR is wire‑truth; strings never
enter the SoA (§4).

---

## 1. Layer inventory — what exists, with receipts

### L0 — Identity & addressing

| Piece | Where | Status |
|---|---|---|
| `Identity` (NiblePath segments, class-side; 27‑bit segs, dict-encoded) | OGAR `ogar-ontology` (#31), `OGAR-AST-CONTRACT.md §1` | **[G] CODED** |
| `NiblePath{path:u64,depth:u8}`, `FAN_OUT=16`, `MAX_DEPTH=16`, `parent()`, `child()`, `is_ancestor_of` (prefix shift), LCA | lance-graph `contract/src/hhtl.rs` | **[G] CODED** `[per rt]` |
| `NodeGuid([u8;16])` UUIDv8 — octets: ns(1)·entity_type(2)·kind(1)·niblepath_prefix(2)·ver/depth(1)·shape_hash(22b)·local(24b)·layout_version(1)·spare(2) | lance-graph `contract/src/identity.rs:66-121` (#480, Phase A, 599 tests) | **[G] CODED** |
| The five register-reads of one GUID: **resolve** (entity_type→ClassView) / **route** (niblepath prefix `is_ancestor_of`) / **witness** (frozen bytes + merkle) / **ground‑truth** (shape_hash drift) / **dispatch‑to‑store** (`as_bytes()`→EntityKey) | `identity.rs:31-37` | **[G] CODED** |
| Bijection law: `entity_type:u16` canonical/exact; `NiblePath` the derived view; GUID prefix = 4‑nibble routing cache; **registry mints `(entity_type, NiblePath)` unique pairs** | `identity-architecture-exists-vs-needs-v1.md` (ratified 2026‑06‑09) | **[G] law / [H] mint** — mint + build-time round-trip = lance-graph **Phase B** |
| `SchemaPtr.packed:u32=[ns:8\|entity_type:16\|kind:8]`, `ClassId=u16` ("never a content hash"), `EntityTypeId=u16`, `EdgeRef{family:u8,local:u16}`, `StructuralSignature` | namespace.rs:119 · class_view.rs:53 · ontology.rs:81 · episodic_edges.rs:34 · odoo_blueprint | **[G] CODED** (StructuralSignature: [G] type / [H] live-wire → Phase B) |
| `class_id` **aliases** the `entity_type` slot on the SoA row — no new column | soa_view.rs:47 `[per xs]` | **[G] CODED** |
| Cold-path identity TODAY: `node_id:u32` + String labels (MetadataStore), `u64` content `dn_hash` (SpoStore), `CogRecord` id-less | metadata.rs:60,86 · spo/store.rs:38 · cogrecord.rs:56 | **[G] CODED but the gap NodeGuid fills** → Phase F migration |

### L1 — The OGAR IR (the two arms + the membrane)

| Piece | Shape | Status |
|---|---|---|
| **THINK arm** `Class` | `{identity, name, parent: Option<Identity> ("subClassOf == supervision edge"), language, mixins: Vec<Identity>, store_accessors, associations, enums, scopes, callbacks, computed_fields, methods, validations, attributes}` | **[G] CODED** (`OGAR-AST-CONTRACT.md §1`; mirrors `ogar-vocab-soa` RecordBatch 1:1) |
| **DO arm — static** `ActionDef` | separate SPO node: `{identity, predicate, object_class → Class, default_subject/temporal/modal, kausal: KausalSpec, method_body, results_in: StateTransition, on_enter: EnterEffect, guard_failure_policy, state_timeout_millis}` | **[G] CODED** (§1; statem terms landed OGAR PR #10) |
| **DO arm — dynamic** `ActionInvocation` | `{identity, realizes → ActionDef, state: ActionState (Pending→Committed/Failed/Cancelled), subject, object_instance, lokal, idempotency_key, trace_id, parent_invocation, emitted_at_millis, failure_reason}` | **[G] CODED** (§1) |
| **The membrane** `KausalSpec` | `StateGuard{field,value} \| LifecycleTrigger \| DependsPath \| None` — *"the only place a domain workflow survives the IR flattening"*; lifecycle (machine) ≠ workflow (data) | **[G] CODED + doctrine** (§3: *"Lifecycle formalized; workflow as data"*) |
| **The four AR primitives** `AssociationKind` | `BelongsTo / HasOne / HasMany / HasAndBelongsToMany` — cross‑ORM: Rails verbatim; Odoo `Many2one/One2many/Many2many` (has_one = One2many constrained to 1) | **[G] CODED** `ogar-vocab/src/lib.rs:655-664` |
| `Association` full option set | `class_name, foreign_key, polymorphic, through, source, as_target, dependent` (app-level) **separate from** `ondelete` (DB-level), `optional, inverse_of, before/after_add/remove, scope_source, auto_join, context_source, check_company, delegate` | **[G] CODED** `lib.rs:672-732` |
| **Traversal vocabulary** `::includes / ::memberof() / ::members() / ::groups::members()` | methods on `Class` (`member_of(name)`, `members(name)`, `group_members(name)`, `includes()`, `associations_of(kind)`, `members_through(name)`) — per The Click: methods on the carrier, never free functions | **ABSENT** — the data is there, the API isn't. **Track O1**, ~50 LOC |
| **Two `Class` shapes** | producer-side `ogar-vocab::Class` (`parent: Option<String>`) vs canonical contract §1 (`parent: Option<Identity>`) — the String→Identity lift | **[G] divergence named** `[per xs]` → **Track O7** (pairs with registry mint + lance-graph D-ODOO-BP-1c/d/e) |

### L2 — Producers (source → IR)

| Producer | Source | Status |
|---|---|---|
| `ogar-python` | Odoo 17.0 core (`@api.depends/@api.onchange/@api.constrains` → `ActionDef` per `ADAPTERS-AND-ACTORS §3.4.1`; `_inherits` → mixins; mapping locked `ODOO-TRANSCODING.md §3,§5`) | **ABSENT** (queued) → **Track O3** |
| `ruff_openproject` / `ruff_ruby_spo` | Rails AR source → SPO triples; vendored in openproject‑nexgen‑rs; test extracts `[TimeEntry, WorkPackage]` from a Rails fixture | **[G] CODED** `[per xs]` |
| `parse_surrealql_ddl` walk | SurrealQL DDL → `Vec<Class>`: DEFINE TABLE→Class; `record<X>`→BelongsTo; `option<record<X>>`→BelongsTo+optional; `option<prim>`→Attribute(required=false). NOT yet: `ASSERT IN`→EnumDecl, DEFINE EVENT→ActionDef, non-owning-side post-pass | **[H] scaffold** — feature `surrealdb-parser` wired (OGAR #23 rust 1.95), walk partial (`ogar-adapter-surrealql/src/lib.rs:143-165, 219-299`) → **Track O2** |
| `ogar-from-elixir` (HIRO `gen_statem` → Rubicon) | Elixir | **[G] scaffold** |
| `ogar-from-ruby` (generic Rails) | — | **ABSENT**; decision: build vs reuse the nexgen ruff path → **Track O4** |

### L3 — Adapters (IR → target forms)

| Adapter | Direction | Status |
|---|---|---|
| `emit_surrealql_ddl(&[Class]) → String` | IR → SurrealQL DDL | **[G] CODED** (hand formatter; signature durable — body swaps to `TableDefinition::new_for_ddl().with_*() + ToSql::to_sql()` when convergence lands, `lib.rs:68-77`) |
| `ogar-adapter-ttl` (Turtle round-trip), `ogar-adapter-clickhouse-ddl` (dotted-name round-trip) | bidirectional | **[G] CODED** (#37, #38/#40) |
| `ogar-knowable-from` — `KnowableFromWriter` trait + `register_class_knowable_from`; `surrealql-hint` (self-describing via emit); `vart-backend` (`Tree<VariableSizeKey,u64>`, NULL-terminated keys) | IR → registry | **[G] CODED** (#25/#33/#43); Lance writer impl = `lance-bind` boundary **[H]** |
| `TripleEmitter` (129 RDF predicates, SPO + TeKaMoLo) | IR → triples | **[G] CODED** |

### L4 — The contract spine (lance-graph-contract, zero-dep)

| Piece | Status |
|---|---|
| `TripletProjection` + `roundtrip_eq → RoundTripFailure` (codegen_spine.rs:107) | **[G] trait** — impls: `op-codegen-projection` **[G per xs]**; odoo blueprint **CLAIMED**; cognitive-write **ABSENT → Phase D** |
| `SoaEnvelope` + `ColumnDescriptor` (byte-geometry ONLY; "width only — no domain meaning"; `name_id` is an ordinal, NOT a string) | **[G] trait / [H] ZERO impls** → **Phase C is the keystone gate** |
| `ClassView` (resolve-late from OGIT cache) + `FieldMask(u64)` presence + `FieldMask::inherit(delta)` = the HHTL `subClassOf` walk as bitwise parent-OR-delta | **[G] CODED** (#441) |
| Rubicon kanban: `KanbanColumn`(6: Planning/CognitiveWork/Evaluation/Commit/Plan/Prune), `KanbanMove ≤16B`, `ExecTarget`(Native/Jit/SurrealQl/Elixir), `MailboxSoaOwner::try_advance_phase()` (checked DAG), Libet −550 ms anchor | **[G] CODED** (#437) |
| 34 `Tactic` kernels over `ThoughtCtx` (the executable bodies DO dispatches into) | **[G] CODED** (#411) |
| Batons: `CollapseGateEmission` `(u16, CausalEdge64)`; `wire_cost_bytes()=13+10·n` | **[G] CODED** — the Firewall's hot-path carrier |

### L5 — Runtime (ractor + mailbox + membrane)

| Piece | Status |
|---|---|
| Generic `state_machine` crate (ractor_actors) — OGAR-agnostic; `Context` opaque; `on_enter` → `CommitHook` | **CLAIMED** (contract §0; runtime-session-owned) → **Track R1: verify first-hand** |
| OGAR/Rubicon binding (fills Context/Event/State; the callcenter codegen lands here; "the class IS the actor spec — the actor is *generated from* the `Class`") | **CLAIMED** → **R2** |
| Supervision routing by NiblePath prefix (`route` reading of NodeGuid; one bit-shift) | **[G] mechanism** (identity.rs + hhtl.rs) / **[H] wiring** → R2/R4 |
| `MailboxSoA` columns `edges[CausalEdge64;N]` / `meta[MetaWord;N]` / `qualia` / `entity_type` — edges and meta SEPARATE (D‑META64 revision holds) | **[G] CODED** `[per rt]` (#477) |
| `commit_event` sole-writer + `ExternalMembrane::project` + `CommitFilter`/`MembraneGate`; today emits scalar `CognitiveEventRow` | **[G] CODED** (lance_membrane.rs:315) — node/edge `project_graph` **ABSENT → Phase E** |
| HEEL/HIP/TWIG/LEAF cascade legend in `high_heel.rs` | **CLAIMED only** — "no code routes by prefix" (q4‑hhtl‑audit) → **Track X4** (also resolves D‑BGZ17's §4.1 unwired gap) |

### L6 — Storage

| Piece | Status |
|---|---|
| Lance: `DatasetVersion(v)→(v+1)` = frame; `last_active_cycle[u32;N]` per-row stamp = changed-cell delta (D‑DELTA `[G]`) | **[G] CODED** `[per rt]` |
| SurrealDB fork (local `/home/user/surrealdb`): `surrealdb/{ast,parser,core}`; `TableType::{Normal,Relation,Any}` (table_type.rs:8); `TableDefinition::new_for_ddl` (catalog/table.rs:161) | **[G] CODED** upstream |
| `kv-lance` pins `lance =6.0.0` vs workspace `=7.0.0` | **[G] debt** TD‑SURREALDB‑KVLANCE‑LANCE7 — blocks the kv-lance storage engine resolving; companion fork PR owed |
| Consumer stores via `EntityKey<'a>(&'a [u8])` — length-agnostic; smb `key_to_filter` branches on length (12→ObjectId, else Binary); a 16-byte GUID is "just another length" | **[G] CODED** (repository.rs:12; smb mongo.rs:79/lance.rs:92; MedCare dms.rs:14) → **Phase G** wiring |

### L7 — Consumers

| Consumer | Integration shape | Status |
|---|---|---|
| **openproject-nexgen-rs** (local; 298 tests) | vendors ruff extractors + lance-graph-contract; `op-codegen-projection impl TripletProjection`; owns `op-surreal-ast` (mirror of surrealdb-core::catalog). Convergence: *"op-codegen-projection is a special case of the general OGAR `Class` → `TableDefinition::new_for_ddl()`"* (OPENPROJECT‑TRANSCODING §10.2) | **[G] sibling on the same spine** `[per xs]` — not yet an `ogar-vocab` dependent |
| **woa-rs / Odoo** | consumes via the blueprint (`OdooEntity` typed, 12 TIER-1 addons extracted #426) + the queued `ogar-python` | **[G] blueprint / ABSENT producer** |
| **MedCare-rs** | label-free contract IS the PII guarantee (HHTL leaf-rename at adapter, D‑PII); `column_mask_bridge`; `EntityKey` import | **[G] CODED** |
| **q2 cockpit** | `graph_render` contract surface | **[G] types** (stubs-dedup debt TD‑Q2‑STUBS‑DEDUP‑1) |

---

## 2. The seams — every cross-boundary contract

| # | Seam | Producer side | Consumer side | Contract type | Status |
|---|---|---|---|---|---|
| S1 | **class identity ↔ instance identity** | OGAR `Identity` (NiblePath) | lance-graph `NodeGuid` | registry mint `(entity_type ↔ NiblePath)` bijection | **[G] type / [H] mint** (Phase B) |
| S2 | **IR → SurrealQL DDL** | `emit_surrealql_ddl` | SurrealDB `DEFINE TABLE/FIELD` | DDL string; future body = `TableDefinition::new_for_ddl` | **[G] wired** |
| S3 | **SurrealQL DDL → IR** | surrealdb-parser AST | `walk_query → Vec<Class>` | `Parser::enter_parse::<Query>` (depth 1000) | **[H] partial walk** (O2) |
| S4 | **knowable_from** (the four-clock pin) | `ogar-adapter-surrealql` stamps at DDL registration | `lance-graph-planner::temporal::classify` deinterlaces (lance version / schema / awareness / thinking) | `LanceVersion` via `KnowableFromWriter` | **[G] pin (ADR‑010) / [H] Lance writer impl**; consumer `temporal.rs` landed #479 |
| S5 | **AR source → SPO → DDL** | `ruff_openproject` | `op-codegen-projection` | `lance_graph_contract::codegen_spine::TripletProjection` | **[G] CODED** `[per xs]` |
| S6 | **ActionDef → runtime** | OGAR DO arm | `ractor_actors::state_machine` via the OGAR/Rubicon binding (`CommitHook`; Context opaque) | the §0 two-layer contract | **CLAIMED → R1/R2** |
| S7 | **mailbox bytes ↔ cold bytes** | `MailboxSoA<N>` | Lance columnar | `SoaEnvelope` (`as_le_bytes().as_ptr()==backing`; `verify_layout()`) | **[H] ZERO impls → Phase C (keystone)** |
| S8 | **cycle → graph** | committed cycle | queryable `NodeGuid` nodes + `EdgeGuid` edges | `project_graph` through `commit_event`+gate | **ABSENT → Phase E** |
| S9 | **GUID → consumer store** | any | smb/MedCare/Lance | `EntityKey(guid.as_bytes())` | **[G] transport / [H] 16-byte wiring** (Phase G) |
| S10 | **content display** | SoA refs | rendered strings | `ClassView::render_rows` + tier dispatch (§4) | **[G] render / [H] tier byte** (O5) |
| S11 | **kind-generic codegen** | non-Odoo targets | `RouteBucketTyped<Kind>` (sidecar WIP, +228 uncommitted, other session's) | blanket impl preserves `RouteBucket` | **[H] WIP — needs first consumer** |
| S12 | **PII boundary** | OGAR/OGIT labels | MedCare | label-free leaf-rename at adapter (D‑PII) | **[G] CODED** |

---

## 3. DO vs THINK — the resolved architecture (the session's central correction)

**THINK (Semantik / structural).** Resolution = a *graph walk*:
aggregate `{attributes, associations, computed_fields, enums, scopes}`
over `parent` + `mixins` edges; child shadows parent by identity slot
(Odoo `_inherit` semantics); presence at the row = `FieldMask`, and
`FieldMask::inherit(delta)` IS the subClassOf walk done as bitwise
parent-OR-delta. Pre-computable per class at OGIT-classification time.
Labels resolve LATE from the OGIT cache (`ClassView`); **zero labels in
the SoA bytes**.

**DO (Pragmatik / behavioral).** NOT inherited as aggregated
`action_defs` — that model is **dead** (corrected from source).
`ActionDef`s are independent SPO nodes keyed `object_class → Class`.
Resolution = *runtime routing*: lookup `WHERE object_class = me`; on
miss, the **supervisor routes the invocation up the `parent` edge**
("subClassOf == supervision edge") — OTP's "route up" doctrine as
substrate topology, mechanically a NiblePath-prefix `is_ancestor_of`
bit-shift (NodeGuid's `route` reading). Effects land as `CausalEdge64`
batons; lifecycle = `ActionState` on `ActionInvocation`; domain workflow
= guarded `on_enter` effect at the `Pending→Committed` crossing.

**The membrane = `KausalSpec`.** `Depends{paths}` references THINK
attribute-paths from inside DO triggers — the §0 endgame's "AST-named-
fact extraction" as a concrete typed predicate. One type ties the arms.

**The instance pin = `NodeGuid`.** resolve / route / witness /
ground-truth / dispatch-to-store — five register reads of one frozen
key; write-once; drift repair = new immutable Lance version.

**Open falsification (the architecture's make-or-break):** does
supervisor routing reproduce Odoo `_inherit` MRO (C3 linearization)
across mixin diamonds? `[H]` — gate F1 below. Named failure mode: if C3
order matters where parent-first routing differs, the fix is
`Class.mixins` **ordering** doing C3, not the supervisor.

---

## 4. Content — strings become as cheap as CAM-PQ (the tier cascade)

The SoA row never carries a string; it carries a **ref + tier byte**
(12 B fixed): `{class_id:u16, field_id:u16, tier:u8, value_ref:u64}`.
Resolution happens only at the display edge (`render_rows`, O(window)).

| Tier | Stores | Ref | Backend | Status |
|---|---|---|---|---|
| T0 | schema labels (class/field/relation/enum) | 4 B | `OgitFamilyTable` (sparse `HashMap<u16,FamilyEntry>`) | **[G]** #364 |
| T1 | common English tokens | 12 b/word | deepnsm 4096-COCA (98.4% coverage) | **[G]** |
| T2 | top-256 values per `(class, field)` | 1 B | `bgz17::palette` per-pair codebooks | **[G] infra / [H] per-class palettes undeclared** |
| T3 | semantic text (meaning-bearing) | 48 b | CAM-PQ 6×256 (`contract::cam::CodecParams`) | **[G] codec / [H] text-encoder pre-pass** |
| T4 | unique-but-recurrent strings (names) | 4 B | Arrow `DictionaryArray` in a Lance pool dataset | **[H]** — upstream-free, contract surface unwritten |
| T5 | long-tail blobs | 8 B (content hash) | Lance blob, content-addressed | **[H]** contract unwritten |

**The binding** (Track O5): `Attribute.content_tier: Tier` on the THINK
arm + a `contract::content_store::{ContentRef, Tier, ContentResolver}`
slice (~300 LOC) + one round-trip test across all six tiers (gate F7).
Iron rule throughout: **bundle identities, never content**
(I‑VSA‑IDENTITIES); the GUID/refs point, the stores hold.

---

## 5. The phased dependency DAG (one merged sequence)

```
                      ┌── Q1 quasicryth vs D-MONOTILE (leaf, read-only) ──→ feeds O6
THEORY                ├── Q2 spectral anti-moiré (jc P3 + hpc::fft)  [per rt]
                      └── Q3 helix fidelity ≥0.9980 (TD-HELIX-OVERLAP-1)  [per rt]

IDENTITY   A NodeGuid ✅(#480) ──→ B SchemaSig→ClassView live ─┐
(lance-     │                                                   ├─→ D cognitive-write TripletProjection
 graph      └────────── C impl SoaEnvelope for MailboxSoA ──────┘    + roundtrip_eq (the account.move
 N-arc)                 (KEYSTONE — leaf, unblocks D)                fixture = gate F1+F5)
                                                                       │
                        G GUID-as-EntityKey (parallel off A)           ▼
                                                            E project_graph through commit_event
                                                                       │
                                                                       ▼
                                                            F MetadataStore string→identity
                        H SurrealQL read glove ─ BLOCKED(C) fork coords (only blocked phase)

OGAR       O1 Class traversal API (leaf, ~50 LOC) ──→ O3 ogar-python (Odoo)
                                              └────→ O4 ruby producer decision
           O2 parse walk completion (EnumDecl lift · DEFINE EVENT→ActionDef · non-owning post-pass)
           O5 content_tier + ContentResolver (needs T-backends: all exist)
           O6 ADR-026 draft (ready bucket; enriched by Q1's verdict)
           O7 String→Identity lift (pairs with Phase B mint)

DO-AXIS    R1 verify state_machine + Rubicon binding (other session has the ball)
           R2 ActionDef → state_machine lowering codegen (needs R1)
           R3 account.move MRO falsification (needs C, D, R2)  ← THE make-or-break
           R4 supervision routing wired by prefix (with R2)

RECONV     X1 rename thinking_engine::CausalEdge64 → CascadeEvent64 (feature-gated)
(lance-    X2 LE byte contract on canonical CausalEdge64 + SoaEnvelope tie-in
 graph     X3 D-MBX-2 ResonanceDto fold into MailboxSoA columns
 debt)     X4 HEEL/HIP/TWIG/LEAF route-by-prefix (closes q4 audit + D-BGZ17 §4.1 gap)
           X5 TD-UNBUNDLE-FROM-1 raw-sum+count fix
           X6 TD-ARIGRAPH-EPISODIC-FIDELITY-1 (Option B = W-slot convergence, D-CSV-6/7)

HYGIENE    H1 SYN §3 co-revert on OGAR #47 (D-EXCITON mirror — outstanding)
           H2 DISCOVERY-MAP folds: D-IDENTITY-PIN [G] · §4.1 supervisor-edge promotion ·
              D-META64 rename cross-note · this map's cross-link
           H3 lance-graph board prepends #477-480 (CLAUDE.md mandate; formally incomplete until done)
           H4 merge order: #47 before #48 (SYN links)
```

**Critical path to the falsification that matters:**
`C → D → (R1 → R2) → R3`. Everything else is parallel or feeds ADR-026.
**Leaf bricks available today:** Q1, O1, C, X1, H1–H4.

---

## 6. Falsification gates — every load-bearing `[H]` with its named test

| # | Gate | Test | Promotes / falsifies |
|---|---|---|---|
| F1 | **MRO/supervision equivalence** | `account.move` fixture: parent=`mail.thread`; send `message_post` (must escalate up the edge and fire there) + `action_post` (must fire locally; `on_enter` writes `state→posted`; `results_in` lands as a baton) | D‑OTP‑INHERIT `[H]→[G]` or names the C3-ordering fix |
| F2 | **Monotile addressability** | read `quasicryth-research/{tiling,hierarchy}.rs` + run `tests/paper_theorems.rs`; 5+3 pass vs the Kaplan/Walker/Richter ladder | D‑MONOTILE cascade-addressability `[H]→[G]` or concrete failure shape |
| F3 | **Spectral anti-moiré** | jc P3 (φ-Weyl) + `hpc::fft`: golden-tile spectrum pre/post quantization vs Base17 | D‑MOIRE/D‑MANTISSA `[H]→[G]` or falsified; D‑QUANTGATE contrast demo |
| F4 | **Eineindeutigkeit** | registry mint uniqueness + build-time `(entity_type ↔ NiblePath)` round-trip (Phase B); GUID prefix-consistency already green (Phase A) | S1 `[H] mint → [G]` |
| F5 | **Round-trip integrity** | `roundtrip_eq` over the identity graph; corrupt-pack must FAIL; NARS `(f,c)` within 1/1023 (Phase D DoD) | S8 path trustworthy |
| F6 | **Zero-copy geometry** | `SoaEnvelope::verify_layout()` green + `as_le_bytes().as_ptr()==backing` on `MailboxSoA` (Phase C DoD) | S7 `[H]→[G]` |
| F7 | **Content cascade round-trip** | build row → ref in SoA → `render_rows` → decode across T0–T5 → byte-equal | O5 ships or tier table revises |
| F8 | **Helix fidelity** | naive-u8 floor ≥0.9980 Pearson vs ground truth (CONJECTURE — NOT RUN) | helix graduates clean-room |
| F9 | **kv-lance resolution** | fork PR bumps the three `=6.0.0` pins → workspace `=7.0.0` resolves | unblocks the Surreal storage leg (and de-risks N8) |

**Discipline** (the lance-graph probe rule, adopted): *if the relevant
probe is NOT RUN, the next deliverable is the probe, not more synthesis.*

---

## 7. Ownership & work-shape

| Track | Owner | Shape |
|---|---|---|
| Q1, O1–O7, H1–H2, H4 | **this session (OGAR)** | docs + small additive contract slices; 5+3-hardened regrades |
| A–H (identity arc), X1–X6, H3 | **lance-graph sessions** | phased landable PRs per the N-plan; board hygiene per CLAUDE.md |
| R1–R2 (state_machine + binding) | **runtime / other session** (has the ball) | first-hand verify, then the lowering codegen |
| F9 (kv-lance pins) | **surrealdb fork session** | one Cargo.toml PR |
| N8 / fork-coords, ADR-026 go, O4 build-vs-reuse | **operator decisions** | the only human gates |

**Not read first-hand yet (honest fence):** `quasicryth-research/*`
contents; `ractor_actors::state_machine` internals; #480's `mul.rs` /
`recipes.rs` / `savants.rs` bodies (diffstat only); nexgen's
`INTEGRATION_PLAN.md` (other session read it). Nothing in this map
*depends* on their contents; F2/R1 read them before any regrade.

---

## 8. Intersecting debt (blocks or is blocked by this map)

| Debt | Intersects | Why it matters here |
|---|---|---|
| TD‑SURREALDB‑KVLANCE‑LANCE7 | S2/S4/N8 | kv-lance can't resolve against the workspace until pins move |
| TD‑UNBUNDLE‑FROM‑1 | runtime gestalt | silent ~1 bit/epoch corruption under the same SoA this map lands on |
| TD‑ARIGRAPH‑EPISODIC‑FIDELITY‑1 | §0 endgame | episodic retrieval currently the RAG baseline, not eq.1 structural — the THINK memory the DO axis consults |
| TD‑RESONANCEDTO‑DUP‑1 | X3 | stranded DTO vs column doctrine |
| TD‑HELIX‑OVERLAP‑1 | Q3/F8 | fidelity probe owed before promotion |
| TD‑WIKI‑SCALE | S1/identity | `StructuralSignature` u32 birthday (~77k) + NiblePath depth-16 ceiling — both bite at load scale; widen/escape paths named |
| TYPE_DUPLICATION §13 (CausalEdge64 ×2) | X1/X2 | same name, two semantics — rename + byte contract |

---

## 9. Blocked & open decisions (the complete list)

1. **N8 fork coords** — BLOCKED(C), human gate (lance-graph P0 "STOP and ask").
2. **ADR-026 go** — ready bucket complete; recommend after F2 (Q1) so the monotile verdict pins with it.
3. **O4** — build `ogar-from-ruby` vs reuse nexgen's ruff path (recommend: reuse; it's CODED and on the same spine).
4. **S11 first consumer** — `RouteBucketTyped` needs one genuine non-Odoo impl or it's YAGNI (candidate: Wikidata-HHTL or the OP target).
5. **#47 → #48 merge order** — operator action; H1 must land on #47 first.

---

## 10. Maintenance

Same five rules as `DISCOVERY-MAP.md §6` (append-only; terse; grade
honestly — `[S]` stays `[S]` until a measurement; this map points, never
re-derives; the map mirrors the substrate). Plus one of its own:
**every seam row must name its contract TYPE** — a seam described only
in prose is a seam that will fork.
