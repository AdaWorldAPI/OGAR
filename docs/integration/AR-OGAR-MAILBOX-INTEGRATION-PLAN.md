# AR → OGAR → SurrealQL — Integration Plan

> **Status**: PROPOSAL — written before context dilutes, post lance-graph #530–545.
> **Audience**: OGAR core + the consumer repos (`openproject-nexgen-rs`, `AdaWorldAPI/surrealdb`).
> **Reads from**: `lance-graph/docs/OGAR_CONSUMER_API.md` (the contract), this doc maps the producer side onto it.

## 0. The 30-second picture

```
  app/models/        ruff_ruby_spo::extract        ogar_from_ruff::lift_model_graph
    ─────────  ──────────────────────────────  ──────────────────────────────────
    *.rb files  →  ruff_spo_triplet::ModelGraph  →  Vec<ogar_vocab::Class>
                            │                              │
                  ruff_spo_triplet::expand                  ▼
                            │                  lance-graph-ontology::OntologyRegistry
                            ▼                  (register_class_path + classid mint)
                  Vec<Triple {s,p,o,f,c}>                   │
                            │                               ▼
              op-surreal-ast::triples_to_schema    classid → ClassView (#534) ─►
                            │                       ├ RegistryClassView (THINK)
                            ▼                       └ ClassActions (DO — §2 below)
              op-surreal-ast::Schema                       │
                            │                              ▼
              surrealdb-core::op_bridge        UnifiedStep at ExecTarget::SurrealQl
                            │                              │
                            ▼                              ▼
              catalog::TableDefinition         the running surreal substrate
              + DDL render (DEFINE TABLE/FIELD/INDEX)
```

The narrow / SPO arm (top-right) lands DDL. The wide / OGAR arm (top-left) lands `Class` → registry. They are TWO LEGS of the same harvest (one parser, two emitters), per `lance-graph/docs/OGAR_CONSUMER_API.md` §0.

## 1. Where we are (delivered, on `main`)

### `AdaWorldAPI/ruff` (producer)
- 57 closed-vocab predicates including `association_kind`, `class_name`, `inherits_from`, `validation_kind`, `validation_param`.
- `harvest_op` example binary: `OP_SRC=… cargo run -p ruff_ruby_spo --example harvest_op` writes `/tmp/op_triples.ndjson` (9327 triples / 694 models on live OpenProject).

### `AdaWorldAPI/openproject-nexgen-rs` (SurrealQL arm — narrow)
- `op_surreal_ast::triples_to_schema(triples) → Schema` lowers the SPO stream to typed SurrealQL DDL.
- 82+ unit tests covering: FK direction (`belongs_to` only emits the column), polymorphic fallback, typed scalars from `field_type`, `class_name` override w/ namespace stripping, STI parent (`inherits:`) separated from concerns, kind-aware ASSERT composition (presence / numericality / acceptance / absence / uniqueness), parametric clauses (`length:maximum=N → string::len <= N` etc., **i64-only safety gate**), composite UNIQUE indices from `uniqueness:scope=[…]`, 15 class-level annotation lifts (scope / delegate / acts_as_* / paper_trail / closure_tree / mount / alias_method / alias_attr / journal_formatter / journal_fields / extend / prepend / using / counter_culture / auto_strip), 3 DSL annotation lifts (dsl / col_override / dyn_method).

### `AdaWorldAPI/surrealdb` (bridge)
- `surrealdb_core::catalog::op_bridge::bridge_schema(ast::Schema) → Vec<BridgedTable>`.
- 22+ tests lowering `op_surreal_ast::FieldDefinition.assert` strings to structural `Expr` — covers presence/absence/acceptance/numericality kind clauses, AND-composed compositions, parametric `$value <op> N` and `string::len($value) <op> N` (i64 RHS only), composite UNIQUE preserved.
- `lance-graph-contract`-aligned dep pin on nexgen-rs main.

### `AdaWorldAPI/OGAR` (vocab arm — wide)
- `ogar-from-ruff` (new): pure projection `ruff_spo_triplet::Model → ogar_vocab::Class`. 12 tests, codex-resolved on plural-scopes, enums, block callbacks.
- `ogar-from-rails` (new): walks Rails source via `ruff_ruby_spo::extract`, lifts to `Vec<Class>`. Live OP smoke: 694 Classes.

### `AdaWorldAPI/lance-graph` (substrate — read-only consumer of mine)
- #534 wired `classid → ClassView` resolution. Keystone.
- #538 added the DO-arm contract (`action::{ActionDef, ActionInvocation, ClassActions}`) + the cycle-aware write gate (`mailbox_soa::write_row` + `WriteOutcome`).
- #544/#545 added `Backend::MailboxSoa` (classid node-match + CLAM/CAKES neighborhood) and the Hamming-plane DistanceMeans.

## 2. The two named gaps (post-#538)

### A. The DO-arm lift — `Class.functions → ClassActions`

`OGAR_CONSUMER_API.md` §2 explicitly names this:

> Generate one `const ACTIONS: &[ActionDef]` per class, register as `ClassActions { classid, actions }`, resolve with `actions_for(registry, classid)`.

My `ogar-from-ruff` currently **drops** `Model::functions` — it's noted in the field-map as "no clean semantic mapping" for the Rails-AR domain. With the DO arm contracted, that's now wrong. The lift is straightforward but constrained by the consumer doc's iron rules (§4):

| Rule                                                                | Implication for the lift                                                                                                                                                                |
|---------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `required_role` and `exec` are consumer-private policy, NOT in harvest | `ogar-from-ruff` produces `ActionDef { required_role: None, exec: SurrealQl, … }` defaults; consumer overrides downstream                                                              |
| `(classid, predicate)` is consumer-local; cross-consumer NOT equal  | snake_case normalization at harvest time (already true on the Rails side)                                                                                                               |
| Egress only through commit gate + `UnifiedStep`                     | consumer-side concern — `ogar-from-ruff` produces the **declaration**, the firing happens in `openproject-nexgen-rs` (or wherever)                                                      |
| `Copy`-not-`Copy` deliberate on `ActionInvocation`                  | producer doesn't construct invocations, just declarations; safe                                                                                                                          |

Output shape:

```rust
pub fn lift_actions(model: &Model) -> Vec<ActionDef> {
    model.functions.iter().map(|f| ActionDef {
        predicate: leak_static_str(&f.name),    // const → &'static str via interner
        object_class: 0,                         // consumer mints classid
        exec: ExecTarget::SurrealQl,             // default; consumer may override
        guard: None,                             // KausalSpec lift is downstream
        required_role: None,                     // consumer RBAC
        overrides: None,                         // future: from MRO chain (mro::resolve_overrides)
    }).collect()
}
```

The `&'static str` requirement is the only structural friction — `ActionDef` is `const`-constructible per `lance-graph-contract::action`. Two options:
1. Interner crate (e.g. `string-cache` or hand-rolled `Box::leak`-on-first-encounter).
2. Generated `const` table on the consumer side, lifted-from a separate `OwnedActionDef` shape in OGAR.

Option 2 is cleaner: keep OGAR vocab `OwnedActionDef` with `String`, let the consumer-side codegen turn that into a `const` table via `quote!` or `build.rs`. Matches the OGAR doctrine ("Core owns the type + lookup; you own the data").

### B. The registration hop — `Vec<Class> → OntologyRegistry`

`OntologyRegistry::class_id_for_guid(&NodeGuid) → Option<ClassId>` (#534) is the consumer-side lookup. The producer-side handoff `Class → ClassId binding + register_class_path` is unbuilt.

Proposed seam: a new crate **`ogar-to-registry`** (or method on `ogar-from-rails`) with the shape:

```rust
pub fn register_all(
    registry: &mut OntologyRegistry,
    classes: &[Class],
    namespace: &str,                // "openproject"
    classid_minter: impl FnMut(&Class) -> ClassId,
) -> Vec<(ClassId, String)>
```

Returns the `(ClassId, class_name)` mapping. Consumer supplies the classid-minting policy (`OD-CLASSID-WIDTH` says classid is u16 — 64K shape families). Wiring of the per-class `NiblePath::from_guid_prefix` precondition belongs to the consumer too — `register_class_path(classid, NiblePath)` makes `class_id_for_guid(guid) == Some(classid)` per #534's docstring.

## 3. Iron rules I will NOT step on

From `OGAR_CONSUMER_API.md` and the named "in progress / gaps":

1. **D-CLS field enumeration** (auto-populates `ClassView` fields from a harvested model) — somebody else owns this. My `ogar-from-rails` produces `Class`, not `ClassView`. The lift `Class → ClassView` is the named gap; I stop at `Class`.
2. **`ClassView::{compute_dag, constraints}` extension** — for computed-field recompute + validation dispatch. My validation lifts feed this from below (the kind/param structure) but the Core extension is not mine.
3. **No parallel object model**. `ogar-from-ruff` produces `Class`, not its own custom IR. The Rails AR shape composes through `ogar-vocab::Class` only.
4. **`required_role`/`exec` are consumer-private**. Lift produces defaults, consumer overrides.
5. **`ActionInvocation` is NEVER constructed in OGAR producer crates** — it's a runtime/dispatch concern. Producer side stops at `ActionDef`.

## 4. Mailbox-SoA-replacing-BindSpace-singleton blast radius

The W3+W4a `mailbox-thoughtspace` feature is default-OFF; production stays singleton-write until W7 deletes `BindSpace`. **Nothing in my pipeline (`op-surreal-ast`, `op_bridge`, `ogar-from-*`) instantiates SoA rows** — we produce static class shape (schema/ontology/DDL), runtime instances are downstream of where my output lands.

Implication for forward work:
- **DO NOT** wire anything to `BackingStoreWrite::Singleton(&BindSpace)` — that's the cycle-blind arm being deleted in W7.
- **DO** treat `WriteOutcome { Accepted | Stale | Future }` as the load-bearing contract for any future runtime-instance write path. Stale-as-outcome (not Result error) is intentional — telemetry-counted, never panics.
- The `current_cycle::wrapping_sub` + half-range gate handles `u32` wrap correctly; downstream consumers can rely on it.

## 5. Concrete next steps (in order)

1. **`ogar-from-ruff::lift_actions`** — `Model::functions → Vec<OwnedActionDef>` on the producer side. Consumer codegen turns into `const &[ActionDef]`. Lands in OGAR.
2. **`ogar-from-rails` exposes `lift_with_actions`** — returns `(Vec<Class>, HashMap<ClassName, Vec<OwnedActionDef>>)` so consumers get both arms from one walk.
3. **`ogar-to-registry`** new crate or method — `Class → OntologyRegistry::register_class_path` + classid binding.
4. **Consumer-side codegen example** — in `openproject-nexgen-rs`, a small build.rs / proc-macro that turns `OwnedActionDef` into `const &[ActionDef]` with the OP RBAC map merged.
5. **`op-surreal-ast` extension** — given `&ClassActions` for a class, project to SurrealQL `DEFINE FUNCTION` per `ActionDef` (only when `exec: ExecTarget::SurrealQl`). Closes the AR → DO → SurrealQL loop.

Steps 1–4 are net-new producer-side lifts. Step 5 is the SurrealQL projection that the consumer doc says `op-surreal-ast` *is* the surface for.

## 6. Test surface

Per-step Definition-of-Done:

| Step | DoD                                                                                               |
|------|---------------------------------------------------------------------------------------------------|
| 1    | `lift_actions_emits_one_def_per_function`, `lift_actions_carries_ruff_method_visibility`, `lift_actions_default_exec_is_surrealql` |
| 2    | `lift_with_actions_returns_consistent_class_count`, real-OP smoke (`expect ≥ 8000 OwnedActionDef from 694 classes`) |
| 3    | `register_all_assigns_unique_classids`, `class_id_for_guid_roundtrips_through_register_all`        |
| 4    | `consumer_codegen_renders_const_table_from_owned_action_defs`                                      |
| 5    | `define_function_emits_for_surrealql_exec_only`, `non_surrealql_exec_falls_back_to_annotation`     |

## 7. Risks / open questions

- **`u16` classid → `u32` NodeGuid.classid()` cast**: `OD-CLASSID-WIDTH` says classid is u16; `NodeGuid::classid()` returns u32 in #534's impl. The consumer doc uses u32 in the example (`object_class: 0x0A1E_0001`). The narrowing/widening seam needs to be explicit at the registration hop.
- **MRO chain → `ActionDef::overrides`**: the other session is wiring `mro::resolve_overrides`. My lift should leave `overrides: None` for now and let a follow-up read the chain after `RegistryClassView` is populated.
- **`AcceptsNestedAttributesFor` for DO arm**: currently skipped (UI form helper, not a relation). If the form helper should become an `ActionDef` (a `update_nested` method), that's a §10.3-class decision.
- **`Function::visibility`** — ruff captures `public`/`private`/`protected`. `ActionDef` has no visibility slot today; private methods may not warrant `ActionDef` entries. Filter at lift?

## 8. Out of scope

- DB schema (`db/schema.rb` / migrations) extraction — D-AR-3.7 sprint. Validates the parametric ASSERTs on real-corpus fields.
- Constant resolution (`SCOPE_COLS` / `MAX_NAME_LENGTH`) — cross-file static analysis.
- Body-level lift (Ruby method body → SurrealQL function body) — D-AR-5.6 sprint.
- Odoo (Python) arm — `ogar-from-python` / `ruff_python_spo` is parallel work.
