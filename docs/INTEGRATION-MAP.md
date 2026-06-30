# INTEGRATION MAP — how the substrate composes across repos

> **Status: LIVING MAP v1.1** (2026‑06‑09; v1.1 = hardened by the 5+3
> savant pass — capacity table L0b, delegation lineage corrected, gates
> F10–F14, Track O7 promoted — **plus the G-pass logic audit** (B joined
> the critical path, diamond ≠ prefix-tree, shape_hash risk-model fixed,
> F2 bridge condition, F14 run-gate) — **plus the canon-pass (operator,
> 2026‑06‑10): the pinned canonical is HEX-counted — it IS the GUID,
> whose `8-4-4-4-12` dash-groups carry the semantics
> `classid-HEEL-HIP-TWIG-[basin6+id6]` (32 hex = 128 bit); hierarchy is
> the overflow; wrappers are audited against the canon group-by-group
> and never the reverse**). Companion to `DISCOVERY-MAP.md` —
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
              methods, callbacks, validations, computed_fields,
              parent — SHIPPED: Option<String> (ogar-vocab lib.rs:95) · CONTRACT-SPEC:
              Option<Identity> ("subClassOf == supervision edge"); the String→Identity
              lift is Track O7 — ★CRITICAL PATH, gates R2/R4 }
DO arm        ActionDef (separate SPO node, object_class → Class) + ActionInvocation (state: ActionState)
MEMBRANE      KausalSpec { StateGuard | LifecycleTrigger | Depends{paths} }  ← the only place domain
              workflow survives IR flattening (OGAR-AST-CONTRACT §3)
IDENTITY      THE CANONICAL GUID (operator-pinned; counted in HEX DIGITS, 32 hex = 128 bit):
                xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
                classid    HEEL  HIP   TWIG  family-basin-leaf(6)+identity(6)
                8 hex      4     4     4     12 hex
              The UUID's own dash-groups ARE the semantic delimiters — every printed GUID
              is self-describing at sight. The cascade tiers are literal hex groups; the
              path nibbles ARE the tree (FAN_OUT=16, 4-bit nibble = 1 hex digit).
              Shipped-code agreement: group1 (8 hex = 32b) = SchemaPtr.packed u32 ·
              group2 = PREFIX_NIBBLES=4 · the 6+6 tail = EdgeRef-24b / LOCAL_BITS=24.
              NodeGuid (#480) = the lance-graph carving of this GUID — Phase B audits it
              GROUP BY GROUP against the canon (see L0 apex row for the one collision).
                              ╚═════════════════════════════════════════════════════════════╝
ADAPTERS      emit_surrealql_ddl [G] · ogar-adapter-ttl [G] · ogar-adapter-clickhouse-ddl [G]
              · ogar-knowable-from (vart-backend) [G] · parse_surrealql_ddl walk [H scaffold]
CONTRACT      lance-graph-contract: TripletProjection · SoaEnvelope · ClassView/FieldMask · kanban
              (KanbanColumn/KanbanMove/ExecTarget) · NodeGuid · CollapseGateEmission (Batons)
RUNTIME       ractor: generic state_machine (OGAR-agnostic) + OGAR/Rubicon binding [R1 unverified]
              · MailboxSoA columns (edges/meta/qualia/entity_type) · supervision routing by NiblePath
              prefix (is_ancestor_of = one bit-shift) · commit_event sole-writer membrane
STORAGE       Lance (DatasetVersion = self-contained snapshot, the "frame" of other docs;
              last_active_cycle = per-row RECENCY stamp, not a delta) · SurrealDB
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
| **THE CANONICAL ADDRESS — counted in HEX, it IS the GUID**: `classid(8 hex)‑HEEL(4)‑HIP(4)‑TWIG(4)‑[family‑basin‑leaf(6)+identity(6)]` = the UUID text format's own `8‑4‑4‑4‑12` dash-groups carrying the semantics. 32 hex = 128 bit. Path nibbles = the 16-ary tree (1 hex = 1 level); 12 path nibbles across HEEL/HIP/TWIG + leaf addressing inside the basin group ≈ hhtl.rs MAX_DEPTH=16. Shipped agreement: group1 = `SchemaPtr.packed` u32 (exact) · group2 = `PREFIX_NIBBLES=4` (exact) · tail 6+6 = `EdgeRef` 24 b / `LOCAL_BITS=24` (exact). **RFC question — RESOLVED: 3×4 UNIFORM, RFC-WAIVED (operator, 2026‑06‑10):** HEEL/HIP/TWIG keep all 4 nibbles each — uniform Morton stride, tier-of-level = `level >> 2` (shift, never branch); RFC 9562 is a WRAPPER format and wrappers adapt to the canon, never the reverse — RFC-needing boundaries adapt at their membrane; native/foreign discrimination via `classid`, not a format constant (§9.10 carries the full episode). **And the GUID is the KEY of key-value** (CLAUDE.md P0): node = 4096 bits = key(128) + value(3968) — the value is everything the key isn't; the key prerenders nodes with zero value decode; Lance may compress the value bits arbitrarily — **compression never costs addressability** | substrate canon (operator-pinned, 2026‑06‑10) | **CANONICAL** — everything below implements or WRAPS it; wrappers are audited against IT, never the reverse |
| `Identity` (NiblePath segments, class-side; 27‑bit segs, dict-encoded) | OGAR `ogar-ontology` (#31), `OGAR-AST-CONTRACT.md §1` | **[G] CODED** — the "27‑bit segments" doc note reconciles TO the canonical (§9.9) |
| `NiblePath{path:u64,depth:u8}`, `FAN_OUT=16`, `MAX_DEPTH=16`, `parent()`, `child()`, `is_ancestor_of` (prefix shift), LCA | lance-graph `contract/src/hhtl.rs` | **[G] CODED** `[per rt]` |
| `NodeGuid([u8;16])` UUIDv8 — octets: ns(1)·entity_type(2)·kind(1)·niblepath_prefix(2)·ver/depth(1)·shape_hash(22b)·local(24b)·layout_version(1)·spare(2) | lance-graph `contract/src/identity.rs:66-121` (#480, Phase A, 599 tests) | **[G] CODED — a CARVING of the canonical GUID**, group-by-group: groups 1–2 and the 24-bit local MATCH the canon exactly; groups 3–4 (canon: HIP/TWIG path nibbles) currently hold ver/depth + shape_hash + variant instead — **the Phase B audit question is whether shape_hash/layout_version yield those groups back to path nibbles or the canon accepts the RFC-carved variant** |
| The five register-reads of one GUID: **resolve** (entity_type→ClassView) / **route** (niblepath prefix `is_ancestor_of`) / **witness** (frozen bytes + merkle) / **ground‑truth** (shape_hash drift) / **dispatch‑to‑store** (`as_bytes()`→EntityKey) | `identity.rs:31-37` | **[G] CODED** |
| Bijection law: `entity_type:u16` canonical/exact; `NiblePath` the derived view; GUID prefix = 4‑nibble routing cache; **registry mints `(entity_type, NiblePath)` unique pairs** | `identity-architecture-exists-vs-needs-v1.md` (ratified 2026‑06‑09) | **[G] law / [H] mint** — mint + build-time round-trip = lance-graph **Phase B** |
| `SchemaPtr.packed:u32=[ns:8\|entity_type:16\|kind:8]`, `ClassId=u16` ("never a content hash"), `EntityTypeId=u16`, `EdgeRef{family:u8,local:u16}`, `StructuralSignature` | namespace.rs:119 · class_view.rs:53 · ontology.rs:81 · episodic_edges.rs:34 · odoo_blueprint | **[G] CODED** (StructuralSignature: [G] type / [H] live-wire → Phase B) |
| `class_id` **aliases** the `entity_type` slot on the SoA row — no new column | soa_view.rs:47 `[per xs]` | **[G] CODED** |
| Cold-path identity TODAY: `node_id:u32` + String labels (MetadataStore), `u64` content `dn_hash` (SpoStore), `CogRecord` id-less | metadata.rs:60,86 · spo/store.rs:38 · cogrecord.rs:56 | **[G] CODED but the gap NodeGuid fills** → Phase F migration |

### L0b — bounded-by-design vocabularies + wrapper-field policies (canon-pass reframe)

> **Canon-pass correction (operator, 2026‑06‑10):** bounded widths are the
> DESIGN — the escape for scale is **the next cascade level**, never wider
> integers. The rows below are **lance-graph WRAPPER field policies**
> (they constrain #480's GUID carving, not the substrate canon) — except
> the FieldMask row, which is an empirical finding with a canonical
> answer. Volume rows are estimates with stated assumptions.

| Field | Width | Ceiling | First-consumer impact | Escape |
|---|---|---|---|---|
| `entity_type` | u16 | 65,536 classes | Odoo ~3.1k ✓; SNOMED ~350k / planet-scale ✗ | TD‑WIKI‑SCALE family; widen at the registry mint when a >64k ontology lands |
| `shape_hash` | 22 b | **G-pass risk-model correction:** comparisons are SAME-CLASS, temporal only (stored vs current hash of one class) — cross-class hashes are never compared (`entity_type` discriminates). Relevant miss = h(old)=h(new) per schema change ≈ 2⁻²² (~2.4e‑7), **negligible** | the birthday-over-all-classes figure (50% @ ~2,411) answered the wrong population | keep 22 b; PIN the use-invariant "never compare cross-class" — if that is ever violated, the birthday math applies and 22 b is too small (§9.6) |
| `local` | 24 b | 16,777,216 per minting scope — **the scope itself is UNSPECIFIED** (global per (ns, entity_type)? per tenant? per prefix?) | IF global per (ns, entity_type): `account.move.line` exceeds in ~10 tenant-years (assumes ~1.7M lines/tenant-yr vs 2²⁴ ≈ 16.8M — an estimate, not a theorem) | §9.7 = first PIN the minting scope, THEN pick the escape (prefix-shard / widen / per-tenant ns) |
| `FieldMask` | u64 | **64 fields**; positions ≥64 silently IGNORED (#441 N3, class_view.rs:76) | **BLOCKER: `account.move` carries 109 field declarations (counted first-hand, account_move.py Odoo 17 — auditor-measured) → THINK render silently drops 45+** | **canonical answer: page via the hierarchy** (a wide class = a basin of field-pages — the cascade IS the overflow mechanism); multi-word mask is merely a wrapper option — **Track X7 + gate F14** |
| `niblepath_prefix` | 16 b / 4 nibbles | depth >4 → prefix-only routing | none — falls back to `entity_type` resolve (documented in identity.rs) | ✓ |

### L1 — The OGAR IR (the two arms + the membrane)

| Piece | Shape | Status |
|---|---|---|
| **THINK arm** `Class` | contract §1: `{identity, name, parent: Option<Identity> ("subClassOf == supervision edge"), language, mixins, store_accessors, associations, enums, scopes, callbacks, computed_fields, methods, validations, attributes}`. **Shipped `ogar-vocab::Class.parent` is `Option<String>` (lib.rs:95)** — the typed edge is SPEC, not shipped (the contract's "mirrors the RecordBatch schemas 1:1" claim holds *except* this parent type — that exception IS the O7 divergence) | **[G] CODED (vocab) / [H] typed-edge** — the supervision-edge binding is real only after Track O7 |
| **DO arm — static** `ActionDef` | separate SPO node: `{identity, predicate, object_class → Class, default_subject/temporal/modal, kausal: KausalSpec, method_body, results_in: StateTransition, on_enter: EnterEffect, guard_failure_policy, state_timeout_millis}` | **[G] CODED** (§1; statem terms landed OGAR PR #10) |
| **DO arm — dynamic** `ActionInvocation` | `{identity, realizes → ActionDef, state: ActionState (Pending→Committed/Failed/Cancelled), subject, object_instance, lokal, idempotency_key, trace_id, parent_invocation, emitted_at_millis, failure_reason}` | **[G] CODED** (§1) |
| **The membrane** `KausalSpec` | `StateGuard{field,value} \| LifecycleTrigger \| DependsPath \| None` — *"the only place a domain workflow survives the IR flattening"*; lifecycle (machine) ≠ workflow (data) | **[G] CODED + doctrine** (§3: *"Lifecycle formalized; workflow as data"*) |
| **The four AR primitives** `AssociationKind` | `BelongsTo / HasOne / HasMany / HasAndBelongsToMany` — cross‑ORM: Rails verbatim; Odoo `Many2one/One2many/Many2many` (has_one = One2many constrained to 1) | **[G] CODED** `ogar-vocab/src/lib.rs:655-664` |
| `Association` full option set | `class_name, foreign_key, polymorphic, through, source, as_target, dependent` (app-level) **separate from** `ondelete` (DB-level), `optional, inverse_of, before/after_add/remove, scope_source, auto_join, context_source, check_company, delegate` | **[G] CODED** `lib.rs:672-732` |
| **Traversal vocabulary** `::includes / ::memberof() / ::members() / ::groups::members()` | methods on `Class` (`member_of(name)`, `members(name)`, `group_members(name)`, `includes()`, `associations_of(kind)`, `members_through(name)`) — per The Click: methods on the carrier, never free functions | **ABSENT** — the data is there, the API isn't. **Track O1**, ~50 LOC |
| **Two `Class` shapes** | producer-side `ogar-vocab::Class.parent: Option<String>` (**verified first-hand, lib.rs:95**) vs canonical contract §1 `Option<Identity>` — the String→Identity lift | **[G] divergence** → **Track O7, ★PROMOTED to critical path**: until it lands, the IR carries no typed edge — R2/R4 would lean on mint-side name resolution; land O7 or name that coupling (pairs with the registry mint + D-ODOO-BP-1c/d/e) |

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
| `SoaEnvelope` + `ColumnDescriptor` (byte-geometry ONLY; "width only — no domain meaning"; `name_id` is an ordinal, NOT a string) | **[G] trait / [H] zero PRODUCTION impls** (only `#[cfg(test)] TestEnvelope`, soa_envelope.rs:266) → **Phase C is the keystone gate** |
| `ClassView` (resolve-late from OGIT cache) + `FieldMask(u64)` presence + `FieldMask::inherit(delta)` = the HHTL `subClassOf` walk as bitwise parent-OR-delta | **[G] CODED** (#441) |
| Rubicon kanban: `KanbanColumn`(6: Planning/CognitiveWork/Evaluation/Commit/Plan/Prune), `KanbanMove ≤16B`, `ExecTarget`(Native/Jit/SurrealQl/Elixir), `MailboxSoaOwner::try_advance_phase()` (checked DAG), Libet −550 ms anchor | **[G] CODED** (#437) |
| 34 `Tactic` kernels over `ThoughtCtx` (the executable bodies DO dispatches into) | **[G] CODED** (#411) |
| Batons: `CollapseGateEmission` `(u16, CausalEdge64)`; `wire_cost_bytes()=13+10·n` — header 13 B = `source_mailbox u32 + chain_position u32 + merge_mode u8 + 4 B reserved` (**verified** collapse_gate.rs:114-124); 10 B/baton = u16+u64 | **[G] CODED + arithmetic verified** — the Firewall's hot-path carrier |

### L5 — Runtime (ractor + mailbox + membrane)

| Piece | Status |
|---|---|
| Generic `state_machine` crate (ractor_actors) — OGAR-agnostic; `Context` opaque; `on_enter` → `CommitHook` | **CLAIMED** (contract §0; runtime-session-owned) → **Track R1: verify first-hand** |
| OGAR/Rubicon binding (fills Context/Event/State; the callcenter codegen lands here; "the class IS the actor spec — the actor is *generated from* the `Class`") | **CLAIMED** → **R2** |
| Routing-on-miss up the parent edge by NiblePath prefix (`route` reading of NodeGuid; one bit-shift) — **prototype-chain DELEGATION semantics (Self/Smalltalk lineage), NOT OTP message routing**: OTP supervisors restart on failure, they do not dispatch | **[G] mechanism** (identity.rs + hhtl.rs) / **[H] wiring** → R2/R4 must wire delegation explicitly; restart coverage ≠ dispatch coverage |
| `MailboxSoA` columns `edges[CausalEdge64;N]` / `meta[MetaWord;N]` / `qualia` / `entity_type` — edges and meta SEPARATE (**that fact is [G]**; D‑META64's bit-budget reconciliation to `MetaWord` is **still [H]/REVISE** — this row does not close it) | **[G] CODED** `[per rt]` (#477) |
| `commit_event` sole-writer + `ExternalMembrane::project` + `CommitFilter`/`MembraneGate`; today emits scalar `CognitiveEventRow` | **[G] CODED** (lance_membrane.rs:315) — node/edge `project_graph` **ABSENT → Phase E** |
| HEEL/HIP/TWIG/LEAF cascade legend in `high_heel.rs` | **CLAIMED only** — "no code routes by prefix" (q4‑hhtl‑audit) → **Track X4** (also resolves D‑BGZ17's §4.1 unwired gap) |

### L6 — Storage

| Piece | Status |
|---|---|
| Lance: `DatasetVersion(v)→(v+1)` = **self-contained immutable snapshot** (the "frame" of other docs; no delta chain — any version readable alone); `last_active_cycle[u32;N]` = **per-row RECENCY stamp** (WHEN last changed, not WHAT — consumers watermark-filter `WHERE cycle > watermark`, never diff-reconstruct; Phase E depends on this reading) (**D‑DELTA's mechanism stays [G]** — only the "changed-cell delta" *label* is corrected; fold owed, H2) | **[G] CODED** `[per rt]` |
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
| S1 | **class identity ↔ instance identity** | OGAR `Identity` (NiblePath) | lance-graph `NodeGuid` | registry mint `(entity_type ↔ NiblePath)` bijection | **[G] type / [H] mint** (Phase B) — **canon-pass correction**: the canonical nibble layout is PINNED (HEEL/HIP/TWIG are address nibbles; lance hhtl.rs 4-bit nibbles AGREE with it); what reconciles is the OGAR contract §1 "27-bit segments" doc note — fix the note TO the canon, not vice versa (§9.9). Phase B additionally audits NodeGuid's embedding of the canonical u32 |
| S2 | **IR → SurrealQL DDL** | `emit_surrealql_ddl` | SurrealDB `DEFINE TABLE/FIELD` | DDL string; future body = `TableDefinition::new_for_ddl` | **[G] wired** |
| S3 | **SurrealQL DDL → IR** | surrealdb-parser AST | `walk_query → Vec<Class>` | `Parser::enter_parse::<Query>` (depth 1000) | **[H] partial walk** (O2) |
| S4 | **knowable_from** (the four-clock pin) | `ogar-adapter-surrealql` stamps at DDL registration | `lance-graph-planner::temporal::classify` deinterlaces (lance version / schema / awareness / thinking) | `LanceVersion` via `KnowableFromWriter` | **[G] pin (ADR‑010) / [H] Lance writer impl**; consumer `temporal.rs` landed #479 |
| S5 | **AR source → SPO → DDL** | `ruff_openproject` | `op-codegen-projection` | `lance_graph_contract::codegen_spine::TripletProjection` | **[G] CODED first-hand** — `impl TripletProjection for OpSurrealProjection`, op-codegen-projection/src/lib.rs:213 |
| S6 | **ActionDef → runtime** | OGAR DO arm | `ractor_actors::state_machine` via the OGAR/Rubicon binding (`CommitHook`; Context opaque) — delegation-on-miss is OGAR's own dispatch design, **not** an OTP behavior; R1 verifies the binding wires it explicitly | the §0 two-layer contract | **CLAIMED → R1/R2** |
| S7 | **mailbox bytes ↔ cold bytes** | `MailboxSoA<N>` | Lance columnar | `SoaEnvelope` (`as_le_bytes().as_ptr()==backing`; `verify_layout()`) | **[H] ZERO impls → Phase C (keystone)** |
| S8 | **cycle → graph** | committed cycle | queryable `NodeGuid` nodes + edge ids (**`EdgeGuid` itself ABSENT** — only `NodeGuid` shipped; EdgeGuid is Phase‑E design surface, not code) | `project_graph` through `commit_event`+gate | **ABSENT → Phase E** |
| S9 | **GUID → consumer store** | any | smb/MedCare/Lance | `EntityKey(guid.as_bytes())` | **[G] transport / [H] 16-byte wiring** (Phase G) |
| S10 | **content display** | SoA refs | rendered strings | `ClassView::render_rows` + tier dispatch (§4) | **[G] render / [H] tier byte** (O5) |
| S11 | **kind-generic codegen** | non-Odoo targets | `RouteBucketTyped<Kind>` | blanket impl preserves `RouteBucket` | **ABSENT on main @ `62bca5e`** — the +228 working-tree WIP did not land (superseded or dropped); re-confirm with its session before planning on it |
| S12 | **PII boundary** | OGAR/OGIT labels | MedCare | label-free leaf-rename at adapter (D‑PII) | **[G] CODED** |

---

## 3. DO vs THINK — the resolved architecture (the session's central correction)

**THINK (Semantik / structural).** Resolution = a *graph walk*:
aggregate `{attributes, associations, computed_fields, enums, scopes}`
over `parent` + `mixins` edges; child shadows parent by identity slot
(Odoo `_inherit` semantics); presence at the row = `FieldMask`, and
`FieldMask::inherit(delta)` IS the subClassOf walk done as bitwise
parent-OR-delta — **structural presence only** (monotone union; a child
cannot structurally remove a parent field, and view-layer hiding like
Odoo `invisible=` is a display-side concern NOT representable in the
mask). Pre-computable per class at OGIT-classification time. Capacity
caveat: the mask is u64 — see **L0b / gate F14** for the >64-field
blocker on wide Odoo models.
Labels resolve LATE from the OGIT cache (`ClassView`); **zero labels in
the SoA bytes**.

**DO (Pragmatik / behavioral).** NOT inherited as aggregated
`action_defs` — that model is **dead** (corrected from source).
`ActionDef`s are independent SPO nodes keyed `object_class → Class`.
Resolution = *runtime routing*: lookup `WHERE object_class = me`; on
miss, **walk the `parent` edge upward** — mechanically a NiblePath-prefix
`is_ancestor_of` bit-shift (NodeGuid's `route` reading). **Lineage
correction (5+3 pass): this is prototype-chain DELEGATION (Self,
Lieberman 1986; JS `__proto__`; Smalltalk's `doesNotUnderstand:` as the
class-chain cousin) carried
ON the supervision topology — it is NOT an OTP behavior.** OTP
supervisors restart on failure; they do not route unhandled messages.
One tree serves two orthogonal semantics (fault containment AND
delegation); R2/R4 must wire delegation explicitly — restart coverage
does not grant dispatch coverage. **Two G-pass invariants:** (1) prefix
ancestry is a TREE relation — the `is_ancestor_of` bit-shift covers only
the single-`parent` spine; **diamonds (multi-`_inherit`) have no prefix
encoding** and require an ORDERED `mixins` traversal — a second
mechanism, currently ABSENT, and precisely what F1(b) gates (the
bit-shift alone cannot pass it). (2) A delegated fire makes
`ActionDef.object_class` an ANCESTOR of the instance's class —
invocation validity must check `object_class is_ancestor_of
instance.class`, not equality, or every inherited fire is rejected.
Effects land as `CausalEdge64`
batons; lifecycle = `ActionState` on `ActionInvocation`; domain workflow
= guarded `on_enter` effect at the `Pending→Committed` crossing.

**The membrane = `KausalSpec`.** `Depends{paths}` references THINK
attribute-paths from inside DO triggers — the §0 endgame's "AST-named-
fact extraction" as a concrete typed predicate. One type ties the arms.

**The instance pin = `NodeGuid`.** resolve / route / witness /
ground-truth / dispatch-to-store — five register reads of one frozen
key; write-once; drift repair = new immutable Lance version.

**Open falsification (the architecture's make-or-break):** does
delegation routing reproduce Odoo `_inherit` resolution? `[H]` — gate F1.
Two grounded sharpenings (5+3 pass): **(1) Odoo is NOT naive C3 over the
source hierarchy** — `_build_model` assembles bases in `LastOrderedSet`
declaration/install order, then Python's C3 runs over THAT tuple; F1 must
replicate the assembly, not assume source-order C3. **(2) The fixture
needs a diamond**: D(B,C), B(A), C(A), method on A and C only — C3 order
`[D,B,C,A]` picks **C**; naive parent-first `[D,B,A,C]` picks **A**. A
single-chain `mail.thread` test cannot falsify. Named failure mode: if
the orders diverge, the fix is `Class.mixins` **ordering** carrying the
linearization — not the delegation walk itself.

---

## 4. Content — strings become as cheap as CAM-PQ (the tier ROUTING table)

**Framing correction (5+3 pass): T0–T5 are routing buckets by value
POPULATION, not fidelity tiers of one object.** A T4 dict entry is not a
lossy T5; there is no early-exit certificate. The codec cascade
(Full→…→Scent) downsamples the SAME object; this table PARTITIONS the
value space — related in spirit, different in mechanism. The SoA row
never carries a string; it carries a **ref-cell**
`{class_id:u16, field_id:u16, tier:u8, value_ref:u64}` — **13 B packed /
16 B `repr(C)`** (state the layout attribute explicitly; "12 B" was an
arithmetic error caught by the pass). Resolution happens only at the
display edge (`render_rows`, O(window)).

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
                      ┌── Q1 quasicryth vs D-MONOTILE (leaf; the crate self-identifies as the
                      │     Quasicryth transcode, arXiv 2603.14999 — substitution hierarchy +
                      │     deep-position, purpose-built for F2) ──→ feeds O6
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

OGAR       O1 Class traversal API (leaf, ~50 LOC) ──→ O2's non-owning post-pass
              (G-pass: the real dependent — producers BUILD, consumers NAVIGATE;
              O3/O4 benefit from O1 but are not gated by it)
           O2 parse walk completion (EnumDecl lift · DEFINE EVENT→ActionDef · non-owning post-pass)
           O3 ogar-python (Odoo) · O4 ruby producer decision (build vs reuse nexgen ruff)
           O5 content_tier + ContentResolver (needs T-backends: all exist)
           O6 ADR-026 draft (ready bucket; enriched by Q1's verdict)
           O7 String→Identity lift — ★PROMOTED TO CRITICAL PATH: shipped parent is
              Option<String> (lib.rs:95); until O7, the IR carries no typed edge —
              routing would lean on mint-side name resolution (Phase B); land O7
              or name that coupling explicitly

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
           X7 FieldMask widening — multi-word / paged presence (the >64-field
              BLOCKER, L0b; gates F14 and real Odoo THINK rendering)

HYGIENE    H1 SYN §3 co-revert on OGAR #47 (D-EXCITON mirror — outstanding)
           H2 DISCOVERY-MAP folds: D-IDENTITY-PIN [G] · §4.1 wording → delegation-on-
              supervision-topology (5+3 lineage fix) · D-META64 split note (separate
              columns [G]; bit-budget reconciliation still [H]) · D-DELTA rewording
              (recency stamp, not delta — enumerate ALL its sites in the map, 4+) ·
              D-MONOTILE promotion-condition tightening (Walker-addressable leg
              required; a 5+3 pass alone does not promote — mirror of F2) ·
              birth D-DELEG-INHERIT (né D-OTP-INHERIT) · birth D-CANON-GUID (the
              operator-pinned canonical: HEX-counted, the GUID's own 8-4-4-4-12
              dash-groups = classid-HEEL-HIP-TWIG-[basin6+id6]; 32 hex = 128 bit;
              hierarchy = the overflow) · this map's cross-link
           H3 lance-graph board prepends #477-480 (CLAUDE.md mandate; formally incomplete until done)
           H4 merge order: #47 before #48 (SYN links)
```

**Critical path to the falsification that matters (G-pass corrected):**
`(C → D)` ∥ `((R1 ∥ O7 ∥ B) → R2)` → **R3**. Two fixes over v1.1: **B
(the registry mint) joins the path** — O7 types OGAR's edge, but B is
what makes the lance-side GUID prefix agree (S1); without B, R2's
routing and the instance GUIDs can disagree silently. And **R1 is a
parallel read, not downstream of O7**. Coordination risk: R1/R2 are the
only critical-path items owned by no present session (§7). Everything
else is parallel or feeds ADR-026. (Upstream nit, keeper-found: the
identity plan's prose path `A→(B,C)→D` conflicts with its own Phase-D
dep row `A, C` — this map follows the dep rows; B gates F, S1, and now
R2 — not D.)
**Leaf bricks available today:** Q1, O1, C, X1, X7-spec, H1–H4.

---

## 6. Falsification gates — every load-bearing `[H]` with its named test

| # | Gate | Test | Promotes / falsifies |
|---|---|---|---|
| F1 | **Delegation ≡ Odoo `_inherit`** | fixture MUST include **(a)** the single chain (`mail.thread`→`account.move`: `message_post` escalates and fires on the parent; `action_post` fires locally, `on_enter` writes `state→posted`, `results_in` lands as a baton) **AND (b) a diamond** D(B,C),B(A),C(A) with the method on A and C — C3-over-`LastOrderedSet` picks C, naive parent-first picks A; replicate Odoo's declaration-order base assembly, not source-order C3 | D‑DELEG‑INHERIT (né D‑OTP‑INHERIT — lineage corrected) `[H]→[G]`, or names the mixins-ordering fix |
| F2 | **Monotile addressability** | read `quasicryth-research/{tiling,hierarchy}.rs` + run `tests/paper_theorems.rs` (crate self-identifies, lib.rs:1-42, as the Quasicryth transcode, arXiv 2603.14999 — substitution hierarchy + deep-position, purpose-described for exactly this); cross-examine vs Kaplan/Walker/Richter | promotes `[H]→[G]` **only if the substitution hierarchy is shown generalized-Morton/Hilbert-addressable (the Walker leg) AND the crate's tiling is the hat / hat-equivalent class (or the addressing provably generalizes)** — otherwise the pass is vacuous for D‑MONOTILE (G-pass bridge condition); a 5+3 pass alone does NOT promote; failure = a concrete shape |
| F3 | **Spectral anti-moiré** | jc P3 (φ-Weyl) + `hpc::fft`: golden-tile spectrum pre/post quantization vs Base17 | D‑MOIRE/D‑MANTISSA `[H]→[G]` or falsified; D‑QUANTGATE contrast demo |
| F4 | **Eineindeutigkeit** | registry mint uniqueness + build-time `(entity_type ↔ NiblePath)` round-trip (Phase B); GUID prefix-consistency already green (Phase A) | S1 `[H] mint → [G]` |
| F5 | **Round-trip integrity** | `roundtrip_eq` over the identity graph; corrupt-pack must FAIL; NARS `(f,c)` within 1/1023 (Phase D DoD) | S8 path trustworthy |
| F6 | **Zero-copy geometry** | `SoaEnvelope::verify_layout()` green + `as_le_bytes().as_ptr()==backing` on `MailboxSoA` (Phase C DoD) | S7 `[H]→[G]` |
| F7 | **Content cascade round-trip** | build row → ref in SoA → `render_rows` → decode across T0–T5 → byte-equal | O5 ships or tier table revises |
| F8 | **Helix fidelity** | naive-u8 floor ≥0.9980 Pearson vs ground truth (CONJECTURE — NOT RUN) | helix graduates clean-room |
| F9 | **kv-lance resolution** | fork PR bumps the three `=6.0.0` pins → workspace `=7.0.0` resolves | unblocks the Surreal storage leg (and de-risks N8) |
| F10 | **Probe-free depth law** | jc P5 (Jirak) + `hpc::cascade`/`reductions`: `r* = ⌈log₄(C/τ)⌉`, inclusive ≤τ acceptance | D‑RSTAR / D‑PROBEFREE `[H]→[G]` (ADR‑025 demonstrated) |
| F11 | **Palette/CAM fidelity floor** | jc P10 (Pflug) + `hpc::quantized`/`fingerprint`: re-measure the ρ anchors | ρ ≥ 0.99 against 0.9973 (HIP) / 0.965 (TWIG) — D‑PAL256/D‑CAM/D‑RHO stop being *cited*, become *re-measured* |
| F12 | **θ conditioning window** | jc P5b (Pearl 2³) + `hpc::quantized` θ-sweep | θ ∈ [1.45, 1.6] with ρ envelope [0.93..0.9973] — D‑THETA/D‑RHOENV `[H]→[G]` |
| F13 | **Backend parity** | `hpc::simd_dispatch` W1c: AVX‑512 vs NEON vs scalar | identical within **1 ULP** — the correctness floor under every gate above |
| F14 | **Wide-model render** | render `account.move` through `ClassView` with full presence (**>64 is the load-bearing bound; 109 declarations counted** in account_move.py, Odoo 17) | **gated: cannot RUN until Phase B** wires the field-enum into `RegistryClassView`; once runnable, fails by construction (FieldMask u64, L0b) until presence exceeds u64 — **Track X7 is the named path** |
| F15 | **AR-recipe collapse** (`PROBE-OGAR-AR-RECIPE-COLLAPSE`) | measure how much of a consumer's lifted behavioural arm folds to the shared ActiveRecord-lifecycle recipe + per-class override bitmask vs genuine per-class leftover. **Odoo upper-bound RUN** (`odoo-rs tests/recipe_redundancy_probe.rs`, default build, slice_2): guard arm 47→1 full-collapse, compute arm 101 distinct of 141 resolved, **45.7% collapse / 54.3% leftover**; clean run is Rails/OpenProject where `ruff_ruby_spo` captures `callbacks` as first-class `Model` data | D‑RECIPE‑BITMASK `[H]→[G]` when the Rails-AR clean run lands near the ~7% leftover target; the Odoo bound already refutes the strong "Odoo→7%" reading |
| F16 | **Constructor-chaining collapse** (`PROBE-OGAR-CHAINING-COLLAPSE`) | the inheritance axis: naive flatten (own + inherited copies) vs chained (stored once at the base `LazyLock<ClassView>` constant) over a consumer's `inherits_from` DAG. **Odoo RUN** (`odoo-rs tests/recipe_chaining_collapse.rs`, full manifest, 388 classes / 166 edges / 3328 methods): naive 4215 vs chained 3328 → **21.0% collapse / 22.7% behavioural**; top base `mail_activity_mixin` (324 copies). Chain-order correctness = F1; acyclic DAG | D‑RECIPE‑BITMASK‑CHAIN `[H]→[G]` (measured); confirms chaining resolves F15's "out-of-slice" upper bound — the real leftover sits below it; orthogonal to F15, stacks. Rails leg (concerns captured) should exceed Odoo's collapse |

(F3 already carries the D‑QUANTGATE pre/post-quantization contrast.
F10–F13 restore the DISCOVERY-MAP §4.2 jc×hpc floor that v1.0 omitted —
the doctrine-keeper's reverse-miss finding: v1.0 gated the identity arc
but left the codec/no-collapse chain ungated.)

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

**Not read first-hand yet (honest fence):** `quasicryth-research` beyond
its crate-doc (lib.rs:1-42 now read; the F2 deep-read of
tiling/hierarchy/tests is still owed); `ractor_actors::state_machine`
internals; #480's `mul.rs` / `recipes.rs` / `savants.rs` bodies (diffstat
only); nexgen's `INTEGRATION_PLAN.md` (other session read it); AriGraph
eq.1's exact formula (PDF fetch 403 — carried `[per lance-graph TD]`).
Nothing in this map *depends* on their contents; F2/R1 read them before
any regrade.

---

## 8. Intersecting debt (blocks or is blocked by this map)

| Debt | Intersects | Why it matters here |
|---|---|---|
| TD‑SURREALDB‑KVLANCE‑LANCE7 | S2/S4/N8 | kv-lance can't resolve against the workspace until pins move |
| TD‑UNBUNDLE‑FROM‑1 | runtime gestalt | silent ~1 bit/epoch corruption under the same SoA this map lands on |
| TD‑ARIGRAPH‑EPISODIC‑FIDELITY‑1 | §0 endgame | episodic retrieval currently the RAG baseline, not eq.1 structural — the THINK memory the DO axis consults (eq.1 cited per that TD entry; not re-verified first-hand, PDF 403) |
| TD‑RESONANCEDTO‑DUP‑1 | X3 | stranded DTO vs column doctrine |
| TD‑HELIX‑OVERLAP‑1 | Q3/F8 | fidelity probe owed before promotion |
| TD‑WIKI‑SCALE | S1/identity | `StructuralSignature` u32 birthday (~77k) + NiblePath depth-16 ceiling — both bite at load scale; widen/escape paths named (home: lance-graph `TECH_DEBT.md`; joined by this map's L0b ceilings) |
| TYPE_DUPLICATION §13 (CausalEdge64 ×2) | X1/X2 | same name, two semantics — rename + byte contract |

---

## 9. Blocked & open decisions (the complete list)

1. **N8 fork coords** — BLOCKED(C), human gate (lance-graph P0 "STOP and ask").
2. **ADR-026 go** — ready bucket complete; recommend after F2 (Q1) so the monotile verdict pins with it.
3. **O4** — build `ogar-from-ruby` vs reuse nexgen's ruff path (recommend: reuse; it's CODED and on the same spine).
4. **S11 status** — `RouteBucketTyped` did NOT land on main (`62bca5e`); confirm with its session whether superseded or dropped before any consumer planning.
5. **#47 → #48 merge order** — operator action; H1 must land on #47 first.
6. *(wrapper-field policy, not canon)* **shape_hash use-invariant** — PIN "same-class temporal comparison only, never cross-class"; then the 2⁻²² per-change residual is acceptable as-is (L0b, G-pass risk-model correction).
7. *(wrapper-field policy, not canon)* **`local` minting scope** — PIN the scope (currently unspecified); the canonical overflow is hierarchical (next cascade level), wrapper-side widening is a lance-graph implementation choice (L0b).
8. **X7 / F14 resolution shape** — canonical answer = page via the hierarchy (wide class = basin of field-pages); multi-word mask = wrapper option. Pick which lands (L0b BLOCKER, 109 fields measured).
9. **Reconcile the doc note to the canon** — OGAR-AST-CONTRACT §1's "27-bit segments" wording vs the pinned canonical GUID (lance hhtl nibbles already agree: 1 hex = 1 level). Fix the NOTE; the canon stands. Phase B audits NodeGuid against the canon **group-by-group** (groups 1–2 + the 24-bit local already match exactly; groups 3–4 are the open carving question — the RFC version/variant nibble collision, L0 apex row). Supersedes both the earlier "two-structures" framing AND the first canon-pass "u32" misread — the canonical counts HEX, and it IS the GUID.
10. **RFC 9562 nibble decision — PINNED: v8-native** (autoresolved under the standing full-authority mandate after a 3× repeated ask; one operator word reverses it). Accept the version hex + variant bits as the substrate's *signature*: UUIDv8 is the RFC's bring-your-own-layout version, so the canon and the spec are the same philosophy; the version hex `8` doubles as a free native/foreign key discriminator at every membrane (S9); cost = HEEL 4 + HIP 3 + TWIG 3 = 10 native path levels (16¹⁰ ≈ 10¹², still overkill — deep chains were always the overflow story's job: registry resolve + try_child ref-escape, which is also why Wikidata-HHTL is NOT a different scheme, just the canon's heaviest consumer). Entropy argument: one skip-rule beats per-tool interop friction paid forever; 6 constant bits are zero-entropy structure. If ratified: pin in CLAUDE.md P0 + L0 apex; Phase B's remaining audit question reduces to whether #480's shape_hash/layout_version yield groups 3–4's free nibbles back to HIP/TWIG path. **REVERSED (operator word, 2026‑06‑10): final = 3×4 UNIFORM, RFC-WAIVED.** The 4/3/3 carving broke the uniform Morton stride — tier-of-level must stay `level >> 2` (shift, not branch) in a substrate whose cascade is shift/mask by doctrine; the interop benefit was mostly hypothetical (Lance/Surreal/`EntityKey` are byte-agnostic; few tools validate version nibbles); and the pin violated this map's own apex rule — **RFC 9562 is a WRAPPER concern, and wrappers adapt to the canon, never the reverse**. RFC-needing boundaries adapt at their membrane; discrimination via `classid`. (Answer to the operator's question: the 4/3/3 was never 4096-codebook-motivated — it fell out of RFC mark positions — but the reversal restores the Morton-tile alignment regardless.) Phase B audit consequence: #480's ver/variant/shape_hash carving of groups 3–4 must yield ALL eight nibbles back to HIP/TWIG path.
11. **STANDING WATCH — 3×4 vs 4×3** (operator mandate: "correct me at any time"): 3×4 stands on the 2026‑06‑10 ledger — shift vs divide, u16-aligned vs byte-straddling tiers, dashes=tiers, byte-per-axis Morton, 3 hops, XOR locality; 4×3's lone synergy (tier index = 4096 slot) is recoverable as a 3-nibble-prefix sub-table inside 3×4. Flip condition: a measured radix/de-interleave workload where 4-tier granularity wins despite alignment costs. CLAUDE.md carries the full ledger.

---

## 10. Maintenance

Same five rules as `DISCOVERY-MAP.md §6` (append-only; terse; grade
honestly — `[S]` stays `[S]` until a measurement; this map points, never
re-derives; the map mirrors the substrate). Plus one of its own:
**every seam row must name its contract TYPE** — a seam described only
in prose is a seam that will fork.
