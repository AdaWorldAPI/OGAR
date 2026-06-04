# SurrealQL AST as the universal adapter — feasibility, beauty, and brutal honesty

> **Purpose.** Record the deliberate architectural decision on whether to use
> `surrealdb-ast` as the *universal* adapter / IR — for both DDL sources
> (SurrealQL itself) and non-DDL sources (Ruby AR, Python Odoo, Elixir Ecto,
> chess via shakmaty, …). The question is genuinely beautiful in one half and
> genuinely impossible in the other, and the reasoning matters more than the
> conclusion — future sessions will re-ask it, and without this doc the
> answer has to be re-derived from first principles.
>
> **Brutally honest TL;DR** (§0 below): the structural arm WOULD work and
> we're already on that path; the behavioral arm CAN'T work because SurrealQL
> DDL has no vocabulary for the lifecycle FSM / SPO+TeKaMoLo / Rubicon binding,
> and the encoding tricks that "look like" they could (comments, sentinels,
> hijacking `DEFINE EVENT … WHEN … THEN …`) are negative-beauty workarounds
> that hide the abstraction mismatch instead of dissolving it. **Hybrid wins
> outright; pure-surreal-AST loses an irreplaceable arm.**
>
> Grounded in: `OGAR-AST-CONTRACT.md` (`Class`/`ActionDef`/`ActionInvocation`),
> `ADAPTERS-AND-ACTORS.md` (SPO+TeKaMoLo / Action vocabulary),
> `OPENPROJECT-TRANSCODING.md §10` (two-arm pattern + nexgen convergence),
> `LANCE-GRAPH-INTEGRATION.md` (substrate alignment),
> `AdaWorldAPI/surrealdb/.claude/op-codegen-bridge/README.md` (Sprint C16b
> `TableDefinition::new_for_ddl` + chainable `with_*` builders),
> `AdaWorldAPI/openproject-nexgen-rs` (`op-surreal-ast`, `op-codegen-projection`,
> C16a/C16c sprints).
>
> Status: **CARVED v0** (2026-06-04). Decision pinned; revisit only if the
> SurrealQL AST grows behavioral vocabulary upstream (unlikely — different
> project, different abstraction level).

## 0. The question, the answer, the recommendation

**The question.** Could OGAR (and every cross-language producer in its
ecosystem — `ogar-from-ruby`, `ogar-from-elixir`, future `ogar-from-shakmaty`,
TTL hydrators for Wikidata-med, etc.) target `surrealdb-ast::Library` /
`DefineTable` / `DefineField` *directly* as its IR, instead of `ogar-vocab::Class`
+ `ActionDef`? One fewer type to maintain; parsers and emitters already
mature upstream; SurrealQL is graph-flavored (`record<X>` carries
association semantics natively); the roundtrip `parse(emit(parse(x))) == parse(x)`
is bijective by construction.

**The split answer.**

| Arm | Use `surrealdb-ast` as IR? | Why |
|---|---|---|
| **Structural** (schema: `Class` + `Association` + `Attribute` + `EnumDecl`) | **Yes, partially — via projection, not substitution.** Bridge OGAR's wide `Class` to `surrealdb-core::catalog::TableDefinition` via `From<Class>`; let `TableDefinition` be the canonical *projection* target. Keep `Class` as the wide source-side IR so producer metadata (Rails `dependent:`, `inverse_of:`, `polymorphic:`; Ecto `embedded_schema`; Odoo `selection_add`; etc.) doesn't vanish. | SurrealQL DDL is the right schema-projection vocabulary for graph-flavored databases; it covers the *what is the shape* question well, but it doesn't cover *what does the producer know about how this shape came to be*. |
| **Behavioral** (lifecycle: `ActionDef` + `ActionInvocation` + `KausalSpec` + `EnterEffect` + `GuardFailurePolicy` + `state_timeout_millis`) | **No. Period.** SurrealQL DDL has no equivalent vocabulary; `DEFINE EVENT` / `DEFINE FUNCTION` are row-trigger / stored-procedure shapes, not lifecycle-FSM shapes. Trying to encode SPO+TeKaMoLo + the `Pending → Committed/Failed/Cancelled` lifecycle in DDL is the abstraction-mismatch hijack version of beauty — it works in a narrow sense and rots immediately. | Different layers of the stack. SurrealQL is a database query language; OGAR is a substrate IR for cross-stack actor modeling. Rubicon's binding (the other session's verified Phase-1 `RubiconMachine` over `State=ActionState`, `Event=ActionDef`, `Context=ActionInvocation`) *requires* OGAR's behavioral vocab and cannot be reconstructed from `DEFINE EVENT`. |

**The recommendation.** The hybrid is what's already happening with
`ogar-adapter-surrealql` (#18, in flight): `Class` stays as the wide IR;
`From<Class> for catalog::TableDefinition` is the durable bridge interface;
behavioral types (`ActionDef`, `ActionInvocation`, all four §6 Rubicon-statem
carriers from PR #10) remain OGAR-owned and irreplaceable. Name it precisely
and lock it: **structural canonicalization, not unification.**

## 1. The beauty of design — what unification would buy

Sketching this honestly first so it's visible what would be sacrificed in the
counterfactual where we DON'T unify (which is the answer).

If `surrealdb-ast` were the universal IR, the win column would be:

- **One fewer type-translation hop.** Today: `source AST → producer (e.g. ogar-from-ruby) → ogar-vocab::Class → surrealdb-core::catalog::TableDefinition → ToSql → SurrealQL DDL string`. With surreal-AST-as-IR: `source AST → producer → surrealdb-ast → DDL string`. One fewer struct family; one fewer round of `From<X> for Y`.
- **Parsers and emitters are mature.** `surrealdb-parser` exists, `ToSql` is wired, `surrealdb-ast::Library` has the `library!` macro tree with `define_table_stmt: Vec<DefineTable>` / `define_field_stmt: Vec<DefineField>` / etc. — battle-tested across SurrealDB's own consumers.
- **Roundtrip is bijective by construction.** `parse(emit(parse(x))) == parse(x)` is provable for any well-formed SurrealQL DDL. OGAR's hand-written formatter (in `ogar-adapter-surrealql`'s `emit_surrealql_ddl`) can't claim that today; it's aligned with the catalog builders' output but not formally bijective.
- **Convergence with nexgen is automatic.** `op-surreal-ast` (C16a) is *already* a mirror of `surrealdb-core::catalog`. If everyone targets the AST / catalog directly, the mirror dissolves; C16c's planned `From<op_surreal_ast::*> for catalog::*` is a no-op.
- **Zero "OGAR-side type maintenance".** New domain (chess, foundry, wikidata-med, AGI substrate) attaches by producing surreal-AST; nothing in OGAR needs to know about it.
- **Graph semantics carry natively.** `record<X>` is exactly `Association{BelongsTo, class_name:X}`; `TYPE option<…>` is optionality; `TYPE string + ASSERT $value IN [...]` is enums; `record<X|Y|Z>` is polymorphism; junction tables are graph edges. Near-1:1.

That's a real-list-of-wins. And it's what makes the question worth taking seriously rather than dismissing.

## 2. Structural arm — yes, but as projection not substitution

The structural mapping is genuinely tight:

| OGAR `Class` field | SurrealQL DDL counterpart | Roundtrip fidelity |
|---|---|---|
| `name` | `DEFINE TABLE <name>` | full |
| `Attribute { name, type_name }` | `DEFINE FIELD <name> ON <table> TYPE <type>` | full (modulo type-name normalization Ruby/Python/Elixir → SurrealQL: `string`/`int`/`bool`/`decimal`/`uuid`/`datetime`/…) |
| `Association { kind: BelongsTo, name, class_name: Some(X) }` | `DEFINE FIELD <name> ON <table> TYPE record<X>` | full |
| `Association { kind: BelongsTo, name, optional: Some(true) }` | `DEFINE FIELD <name> ON <table> TYPE option<record<X>>` | full |
| `Association { kind: HasMany / HasOne / HasAndBelongsToMany }` | (no field on this table — inverse side, FK on target) | full via projection from the owning side |
| `EnumDecl { column, source: EnumSource::Static(items) }` | `DEFINE FIELD <column> ON <table> TYPE string ASSERT $value IN [<keys>]` | full |
| `description` | `DEFINE TABLE … COMMENT '…'` | full |
| `parent` (single-table inheritance / subClassOf) | (no native — encode via convention; SurrealQL doesn't have an STI primitive) | **partial** — convention needed |

So far, so beautiful. Now the brutally honest part — **what OGAR's `Class` carries that doesn't survive the DDL projection**:

| Source-side metadata on `ogar-vocab::Association` | What it tells consumers | Fits in DDL? |
|---|---|---|
| `dependent: Option<String>` (Rails `:destroy`/`:delete_all`/`:nullify`/…) | cascade behaviour on parent deletion | **No** — `DEFINE FIELD` has no equivalent; needs `DEFINE EVENT WHEN DELETED THEN …` (different model, lossy on roundtrip) |
| `inverse_of: Option<String>` | reciprocal relation name on target | **No** — DDL is per-table; inverse is bookkeeping for the source's ORM |
| `polymorphic: Option<bool>` + `as_target: Option<String>` | runtime-discriminated target type via `<name>_type` column | **Partial** — `record<X|Y|Z>` covers some cases; the `<name>_type` companion column is lost |
| `through: Option<String>` + `source: Option<String>` | indirection via intermediate association | **No** — DDL only sees the materialized join; the "go through this association" instruction is gone |
| `before_add` / `after_add` / `before_remove` / `after_remove` | collection-membership callbacks | **No** — Rails-AR specific; not even close to DDL |
| `class_name` (when target name ≠ relation name) | explicit target type override | partial via `record<X>` if known at AST time |

And on `Class` itself:
- `mixins: Vec<Identity>` (concerns / `include` / `use Module`)
- `store_accessors: Vec<String>` (Rails store-backed accessors)
- `decorators: Vec<String>` (`acts_as_*`, `has_paper_trail`, `acts_as_list`)
- `callbacks: Vec<Callback>` (`before_save`/`after_destroy`/…)
- `computed_fields: Vec<ComputedField>` (Odoo `@api.depends` derivations)
- `validations: Vec<Validation>` (changeset rules, ActiveModel validations)
- `scopes: Vec<Scope>` (query rewrites)
- `methods: Vec<MethodDecl>` (CRUD overrides, business methods)

None of those have a SurrealQL DDL home. Some are *behavior* (covered in §3),
but several are *structural-but-source-side*: a roundtrip through DDL erases
them. If the IR is surreal-AST, every producer has to either (a) drop the
metadata, or (b) carry a side-car struct that defeats the unification.

**Verdict for structural arm:** the projection is real and we should
canonicalize it (`From<Class> for catalog::TableDefinition`), but the IR
stays `Class` because the projection is lossy on source-side metadata that
downstream consumers (codegen for PG DDL, OpenAPI, TS interfaces, GraphQL,
the runtime ORM behaviours) need.

## 3. Behavioral arm — no, and here's the brutal honesty

The behavioral arm is where the question goes from "elegant trade-off" to
"abstraction-level mismatch." Let's enumerate what would need to survive:

### What OGAR's behavioral arm encodes

| OGAR type | What it captures | DDL equivalent |
|---|---|---|
| `ActionDef { predicate, object_class, default_subject: ActionSubject, default_temporal: TemporalSpec, default_modal: ModalSpec, kausal: KausalSpec, … }` | An action declaration with full SPO+TeKaMoLo annotation | **none** — DDL has no SPO grammar |
| `ActionInvocation { state: ActionState, idempotency_key, trace_id, parent_invocation, emitted_at_millis, failure_reason, lokal, … }` | A dynamic fire of an action, with lifecycle state + provenance | **none** — DDL describes schema, not invocations |
| `ActionState { Pending, Committed, Failed, Cancelled }` | The Rubicon lifecycle — Pending → Committed is the load-bearing crossing | **none** — `DEFINE EVENT` has no lifecycle state model |
| `KausalSpec { StateGuard{guard_field, guard_values}, LifecycleTrigger{event}, Depends{paths}, ContextDepends{keys}, External }` | The five guard kinds for "when is this action allowed to fire" | **none** — `DEFINE EVENT WHEN <expr>` is a single SQL predicate; no Depends/ContextDepends typing, no LifecycleTrigger taxonomy |
| `EnterEffect { field, to_value }` (typed since #13) | The state mutation that fires on entering Committed (the Rubicon crossing); lowers to `StateMachine::on_enter` → `CommitHook::on_commit` → `LanceMembrane::commit_event` | **none** — closest is `DEFINE EVENT WHEN $event THEN UPDATE …`, but it's row-trigger semantics, not lifecycle-crossing semantics, and it doesn't compose with `CommitHook` |
| `GuardFailurePolicy { Postponable, Reject }` (since #10) | When a `StateGuard` fails: replay (`Postpone`) vs hard-fail (`Reject`) | **none** — DDL events have no postpone semantics; failures abort the trigger transaction |
| `state_timeout_millis: Option<i64>` (since #10) | Per-state SLA; lowers to `state_machine::timeout()` (gen-stamped, auto-cancels at crossing) | **none** — DDL has no per-state timer concept |
| `TemporalSpec { Immediate, Deferred, Scheduled, OnCommit, StateTimeout }` | When does the action run (sync vs deferred vs scheduled vs post-commit) | **partial** — `DEFINE EVENT` is essentially `Immediate` only; no scheduling, no post-commit-only, no per-state timeout |
| `ModalSpec { Sync, Async, Idempotent, Atomic, Requires }` | How the action runs | **partial** — DDL trigger semantics are roughly Atomic + Sync; no Async, no Idempotent typing, no Requires |
| `ActionSubject { User, System, Cron, Trigger, Cascade }` | Who/what initiated | **none** — DDL events don't carry SPO subject typing |
| `LokalSpec { actor, tenant, company }` | Where it executes | **partial** — `DEFINE EVENT … FOR …` has scope, but not actor / tenant / company typing |

### Why the "encode it in DDL anyway" tricks don't work

You CAN smuggle some of this into SurrealQL by:

- **Hijacking `DEFINE EVENT WHEN … THEN …`** with structured comments / sentinel patterns to encode TeKaMoLo: e.g. `DEFINE EVENT confirm ON sale_order WHEN $value.state == 'draft' THEN BEGIN /* OGAR: subject=User predicate=confirm temporal=Immediate modal=Atomic kausal=StateGuard{state,[draft]} */ UPDATE … END;`
- **Inventing convention-driven table layouts**: an `_ogar_actions` table with rows per `ActionDef`, an `_ogar_invocations` table for `ActionInvocation`, etc. Use `DEFINE TABLE` for the metadata.
- **Encoding the lifecycle as a state column** on every row and writing `DEFINE EVENT` triggers for each `Pending → Committed` transition.

Each of these is a **language hijack** — DDL parses it because the parser
doesn't enforce semantics, but no DDL-consuming tool understands the
encoding. You've added an OGAR-specific layer *inside* what's nominally
surreal-AST, which means:

1. **The IR is no longer surreal-AST** — it's "surreal-AST plus an OGAR
   convention layer." The unification disappears the moment you write the
   first sentinel-comment parser.
2. **Roundtrip is no longer bijective** — `parse(emit(parse(x)))` produces
   the same DDL text, but the OGAR semantics encoded in the comments are
   opaque to `parse`; you can never reconstruct `ActionDef` from the
   DDL-side without your sentinel parser. The bijectivity is in surface
   syntax, not in semantics. The conventional bijectivity promise becomes
   a lie.
3. **Other DDL-consuming tools (the SurrealDB engine itself, op-codegen-projection, future tools) treat the OGAR encoding as opaque text** — they lose the semantics on every roundtrip, can introduce surprising behavior, and cement OGAR's encoding as a hidden language.
4. **The abstraction mismatch becomes load-bearing**: every future change to SurrealQL syntax risks breaking OGAR's sentinel encoding, and every change to OGAR's behavioral vocab requires re-grammaring the sentinels. The cost grows with both surfaces.

In short: the encoding tricks **look like beauty but feel like sand-castles**.
They survive demo, not maintenance.

### Why the abstraction levels can't fold

The deeper reason is structural:

- **SurrealQL is a database query / DDL language.** Its job is to describe the shape of data, to query it, to define triggers that fire on data changes. The vocabulary is well-fit to the data layer.
- **OGAR is a substrate IR for cross-stack actor modeling.** Its job is to describe what classes exist across heterogeneous source languages, what their lifecycle FSMs look like, what guards their transitions, what effects their commits have, what gets replayed (`Postpone`) versus rejected (`Reject`) versus deferred (`StateTimeout`).

These sit at different layers of the same stack. Trying to fold OGAR's
behavioral arm into SurrealQL is like trying to encode Erlang's OTP
behaviors as SQL DDL, or like trying to encode Kubernetes controllers as
Helm chart templates: structurally possible in some narrow sense, but the
target language doesn't have the vocabulary for the source's semantics, so
every encoding is a workaround.

**The crystallizing test:** Rubicon's `RubiconMachine` (the other session's
verified Phase-1 crate) lowers from OGAR's behavioral types
(`State=ActionState`, `Event=ActionDef`, `Context=ActionInvocation`). If
the IR were surreal-AST, Rubicon would need to invent a parallel behavioral
IR somewhere else, because surreal-AST simply doesn't have the types.
We've now spent multiple PRs (`OGAR-AST-CONTRACT.md` #9, `EnterEffect` #13,
`§3 reconciliation` #11) deliberately landing OGAR's behavioral types as
the canonical lifecycle-IR. Folding them away would re-litigate that.

## 4. Adapter classes — structural differences side-by-side

For completeness, here's the precise type-level diff between the two
candidate IRs in the structural arm (where the comparison even makes
sense):

### OGAR `ogar-vocab::Class` (the wide source-side IR)

```
Class {
    identity: String,                      // OGIT-prefixed Identity
    name: String,
    parent: Option<Identity>,              // subClassOf / STI parent
    language: Language,                    // Ruby | Python | SurrealQl | Sql | TypeScript | Elixir | Unknown
    declared_in_module: Option<String>,
    source_version: Option<String>,
    description: Option<String>,
    record_order: Option<String>,          // Odoo _order
    rec_name: Option<String>,              // Odoo _rec_name
    abstract_model: bool,                  // Odoo abstract, Rails abstract_class
    transient: bool,                       // Odoo Transient, Rails (no equiv)
    auto_create_table: Option<bool>,       // Odoo _auto
    log_access: Option<bool>,              // Odoo _log_access
    inheritance_column_disabled: bool,     // Rails inheritance_column = nil
    mixins: Vec<Identity>,                 // include / use Module / use M.Behaviour
    store_accessors: Vec<String>,          // Rails store accessors / Ecto embeds
    associations: Vec<Association>,        // 11 BelongsTo / HasMany / HasAndBelongsToMany sub-fields
    enums: Vec<EnumDecl>,                  // EnumSource::{Static|Computed|Add}
    scopes: Vec<Scope>,
    callbacks: Vec<Callback>,
    computed_fields: Vec<ComputedField>,
    methods: Vec<MethodDecl>,
    validations: Vec<Validation>,
    attributes: Vec<Attribute>,
}
```

### `surrealdb-core::catalog::TableDefinition` (the canonical schema projection)

```
TableDefinition {
    namespace_id: NamespaceId,             // in-DB ID; new_for_ddl supplies dummy
    database_id: DatabaseId,               // ditto
    table_id: TableId,                     // ditto
    name: TableName,
    schemafull: bool,                      // via with_schemafull
    drop: bool,                            // via with_drop
    comment: Option<String>,               // via with_comment
    table_type: TableType,                 // via with_table_type (Normal | Relation | Any)
    view: Option<ViewDefinition>,          // via with_view
    permissions: Permissions,              // via with_permissions (pub(crate) — auth sprint)
    changefeed: Option<…>,                 // via with_changefeed
    cache_*: Uuid,                         // in-DB cache timestamps; new_for_ddl supplies Uuid::now_v7()
}
// Fields land via DEFINE FIELD (FieldDefinition::new_for_ddl(...).with_*(...))
// Indexes via DEFINE INDEX (IndexDefinition::new_for_ddl(...))
```

### Where they diverge

| Concept | `Class` | `TableDefinition` | Comment |
|---|---|---|---|
| **Identity** | OGIT-prefixed `String` (`ogit-op::WorkPackage`) | `TableName` (just `"work_package"`) | OGIT prefix is OGAR-side bookkeeping; doesn't fit in `TableName`. Lost on projection unless carried as `comment` or via convention. |
| **Language provenance** | `language: Language` (`Ruby`/`Elixir`/…) | none | Schema-side projection erases source-language identity. |
| **Source location** | `declared_in_module`, `source_version` | none | Important for debugging / re-extraction; no DDL home. |
| **STI / subClassOf** | `parent: Option<Identity>` | none (no STI primitive in SurrealQL) | Encode via convention column or lose. |
| **Mixins / concerns** | `mixins: Vec<Identity>` | none | Concerns are how Ruby/Elixir compose behavior; lose entirely. |
| **Decorators** | `decorators: Vec<String>` (`acts_as_*`, `has_paper_trail`) | none | These drive ORM-level behavior; lose. |
| **Validations** | `Validation` per attribute | partial via `DEFINE FIELD … ASSERT …` | Some validations encode (regex, range); compound ones (`validate :method`, conditional, cross-field) don't. |
| **Computed fields** | `ComputedField` (with `@api.depends` paths) | partial via `DEFINE FIELD … VALUE …` | SurrealQL `VALUE` is an expression; OGAR's structured `depends_on` paths can encode but lose the typed dependency graph (the SPO arm carries that explicitly). |
| **Callbacks** | `Callback` (before/after save/create/destroy) | partial via `DEFINE EVENT WHEN $event THEN …` | Each `Callback` becomes a `DEFINE EVENT`; loses the `Action` SPO+TeKaMoLo annotation. |
| **Scopes** | `Scope` (query rewrites) | none | Scopes are ORM-side query construction; not DDL. |
| **Methods** | `MethodDecl` (business methods, CRUD overrides) | partial via `DEFINE FUNCTION` | Some methods can land as functions; most are class-instance methods that don't translate. |

The list isn't decoration — every line is a real producer-extracted thing
that some downstream tool needs. Lose it and the codegen ecosystem narrows.

## 5. The hybrid that works — recommendation, pinned

```
Source AST (Ruby AR / Python Odoo / Elixir Ecto / SurrealQL DDL / TTL / …)
              │
              ▼
      ┌───────────────┐
      │  Producer     │     (ogar-from-ruby / ogar-from-elixir / parse_surrealql_ddl / hydrators)
      │               │
      └───────┬───────┘
              │ fills
              ▼
  ┌──────────────────────────┐        ┌──────────────────────────┐
  │  STRUCTURAL              │        │  BEHAVIORAL              │
  │  ogar-vocab::Class       │        │  ogar-vocab::ActionDef   │
  │  (wide source-side IR;   │        │  ogar-vocab::ActionInv.  │
  │   keeps all metadata)    │        │  (lifecycle FSM; SPO+    │
  │                          │        │   TeKaMoLo; KausalSpec;  │
  │                          │        │   EnterEffect; …)        │
  └──────────┬───────────────┘        └──────────┬───────────────┘
             │ From<Class>                       │ unchanged
             ▼                                   ▼
  ┌──────────────────────────┐        ┌──────────────────────────┐
  │  surrealdb-core::catalog │        │  ractor_actors::          │
  │  ::TableDefinition       │        │  state_machine            │
  │  (canonical schema proj.)│        │  (RubiconMachine)         │
  └──────────┬───────────────┘        └──────────┬───────────────┘
             │ ToSql                             │ on_event / on_commit
             ▼                                   ▼
  SurrealQL DDL string               Lance commit via LanceMembrane::commit_event
  (Sprint C16b builders)             (lance-graph PR #467 — gate 1 closed)
```

### What this nails down

1. **`Class` is the wide source-side IR.** Producers fill it. It keeps
   everything the source AST knows.
2. **`From<Class> for catalog::TableDefinition` is the durable schema
   projection.** Single function. Once it exists, `op-codegen-projection`
   (nexgen, C15) and `ogar-adapter-surrealql` (OGAR, #18) both call it.
   Schema-side ecosystem converges automatically. `op-surreal-ast` (C16a
   mirror) becomes the *fast in-repo path*; the general path is this `From`.
3. **`ActionDef` / `ActionInvocation` stay OGAR-owned, irreplaceable.** No
   surreal-AST counterpart exists; none is being built; trying to encode
   them in DDL is the negative-beauty workaround that doesn't survive
   maintenance.
4. **`Identity` (NiblePath) is the shared routing key.** Both arms reference
   the same string. `Class.identity` and `ActionDef.object_class` resolve to
   the same `TableDefinition::name` via the projection.
5. **Roundtrip:**
   - **emit:** `Class → catalog::TableDefinition → DDL string` (bijective on the schema-side subset).
   - **unmap:** `DDL string → surreal-AST → catalog::TableDefinition → Class with metadata reconstructed from the producer (or empty/default if recovering from DDL alone)`. Roundtrip is one-way-lossy on source-side metadata; that's the honest version, not a bug.
6. **The §10.3 meet-point holds:** `ogar-adapter-surrealql` sources `knowable_from` at DDL registration; `lance-graph-planner::temporal::classify` (#468) consumes it. The frame ownership doesn't change in the hybrid.

### Decision pinned (do not re-litigate without new evidence)

> **Decision (2026-06-04, this doc):** OGAR will NOT unify on
> `surrealdb-ast` as the universal IR. The structural arm canonicalizes
> via `From<Class> for catalog::TableDefinition` (Sprint C16b alignment);
> the behavioral arm stays OGAR-owned (`ActionDef` / `ActionInvocation` /
> the four §6 Rubicon-statem carriers). Re-open only if SurrealQL grows
> behavioral vocabulary upstream (unlikely; different project, different
> abstraction level) OR if a concrete migration debt forces the
> conversation.

## 6. Migration scaffold — a third option during transition, not at steady state

§3 concluded "no" to using SurrealQL DDL as the universal IR because at
*steady state* the behavioral arm's lifecycle FSM has no DDL home and the
encoding tricks that look like beauty rot immediately. That verdict stands.

But §3's analysis assumed a *steady-state target architecture* — every
actor is a native Rust ractor handler over OGAR's IR, every commit goes
through `LanceMembrane::commit_event`. **During migration** from existing
Rails-AR / Elixir-OTP stacks to substrate-b, a different question
emerges: can the substrate's actor scheduler (the Kanban dispatcher per
`SOA-IMPLEMENTATION.md §5.2`) tolerate *non-native* executable forms as
work-items, so production behavior runs on the substrate before every
actor has been translated through `ogar-from-{elixir,ruby}` + Rubicon
codegen?

The answer is yes. The Kanban contract is shaped narrowly enough — a
work-item is "given `(state, event, ctx)`, produce `Transition`" — that
multiple executable forms can satisfy it:

| Work-item form | Source | Migration role |
|---|---|---|
| Native Rust ractor handler | Rubicon-from-OGAR codegen | **the steady-state target** (every actor graduates here) |
| BEAM-compiled Elixir function | HIRO/Bardioc actors via Erlang Port / NIF | scaffold for Elixir stacks (high binary cost, full runtime fidelity) |
| Tiny Elixir-AST interpreter | The "limited commandlets" subset (gen_statem returns, GenServer handle_*, Ecto changesets) evaluated in-process | scaffold for Elixir stacks without BEAM dependency (engineering cost: a real interpreter) |
| HTTP RPC to Rails sidecar | OpenProject's Rails app on Puma; dispatch = `POST /api/v3/work_packages/:id/<action>` | scaffold for Rails stacks (lowest friction, production-ready Rails app deploys unchanged) |
| Embedded CRuby via FFI | In-process Ruby dispatch with `ActiveRecord::Reflection` introspection | scaffold for Rails stacks in-process (CRuby GIL serializes per-actor — matches Kanban's per-actor mailbox model) |
| Rails-AR-reflection dump-as-producer-input | Static-time `rails runner` dump of every `Model.reflect_on_all_*` / `_validators` / `_save_callbacks` fed to `ogar-from-ruby` | **the cheapest beauty win** — no embedding, no sidecar, no runtime; just better producer extraction (Rails AR's runtime reflection is wider than what static Ruby-AST extraction sees: macro expansions, `acts_as_*` metadata, derived columns all materialize) |

Each form's commitment cost differs (HTTP sidecar lowest, BEAM/CRuby
embed highest, reflection-dump effectively zero). The §14 oracle gates
per-actor graduation from migration-form to native-Rust steady state.
The Kanban dispatcher is the **stable contract**; the work-item form
migrates underneath it without disrupting the rest of the substrate
(Lance commits, deinterlace, Rubicon's downstream semantics).

This is **architecturally different from §3's unification question** — it
doesn't replace OGAR's IR with anything; OGAR's `Class`/`ActionDef`
remain the target form every actor graduates to. It's an *additional
layer*: work-item polyglotism *during* the migration window, dissolving
as actors graduate.

The full architecture of the migration scaffold + its end-state
(OpenProject as the substrate's own operator pane + visualization
tier-stack + SDK shape comparison vs Palantir Foundry) is in
**`docs/SUBSTRATE-ENDGAME.md`** — separate doc, different concern from
this doc's structural-vs-behavioral analysis, not folded in here to keep
this record focused on its original topic.

**Decision pinned:** §3's verdict on OGAR's behavioral IR being
irreplaceable for the *steady-state* substrate stands. The
kanban-as-polyglot-dispatcher pattern is the orthogonal migration
architecture. Both decisions live; they don't conflict; the migration
scaffold dissolves once graduation completes.

## 7. What changes in the existing crates

Nothing immediate — this doc *records* the decision; the trajectory was
already correct. The pieces this doc explicitly endorses:

- `ogar-adapter-surrealql` (#18, in flight): `emit_surrealql_ddl(&[Class]) -> String` is the OGAR-side render of the hybrid; its body can later swap to `From<Class> for catalog::TableDefinition` + `ToSql` once OGAR's `rust-version` bumps and the `surrealdb-parser` feature unblocks.
- `ogar-vocab::Class` + `ActionDef` + `ActionInvocation`: stay as canonical OGAR types. No deprecation.
- nexgen's `op-surreal-ast` (C16a) + `op-codegen-projection` (C15): coexist as fast OP-specific in-repo paths; converge with OGAR via C16c's `From<op_surreal_ast::*> for catalog::*`. No collision; same destination.

## 8. Cross-references

- `OGAR-AST-CONTRACT.md` §2 — the SurrealQL DDL bridge (this doc's structural arm).
- `OGAR-AST-CONTRACT.md` §3 — the lowering contract (this doc's behavioral arm is irreplaceable for Rubicon).
- `ADAPTERS-AND-ACTORS.md` §3 — SPO+TeKaMoLo / Action vocabulary (the behavioral arm's grammar).
- `OPENPROJECT-TRANSCODING.md` §10.1 — two-arm pattern (companion framing); §10.2 nexgen convergence; §10.3 `knowable_from` meet-point.
- `LANCE-GRAPH-INTEGRATION.md` — the substrate alignment story (OGAR as a `SchemaSource` producer into upstream `OntologyRegistry`).
- `ELIXIR-HIRO-PREFETCH.md` §2.2 — `gen_statem` as the load-bearing case for the behavioral arm (would be impossible to encode in DDL).
- `CHESS-TRANSCODING.md` §0 — the trichotomy calibration (Semantik / Syntax / Pragmatik) for which OGAR is the carrier; SurrealQL is one syntactic projection.
- Sprint C16b op-codegen-bridge README (`AdaWorldAPI/surrealdb/.claude/op-codegen-bridge/README.md`) — the builders this doc endorses.
- nexgen `op-surreal-ast` / `op-codegen-projection` (`AdaWorldAPI/openproject-nexgen-rs`) — the OP-specific path that coexists.
- `lance-graph` PR #468 — `temporal::classify` consumes `knowable_from` per the §10.3 pin; cited here to anchor the meet-point reference.
- `docs/SUBSTRATE-ENDGAME.md` — the migration scaffold (§6 here points there) + OP-as-operator-pane + visualization tier-stack + SDK endgame with Foundry comparison.
- `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` — ADR-style capture of every architectural decision from this session, including the structural-vs-behavioral decision recorded in this doc.
