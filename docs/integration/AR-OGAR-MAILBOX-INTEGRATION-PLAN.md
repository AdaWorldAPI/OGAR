# AR → OGAR → SurrealQL — OP-Arm Canonical-Contract Plan

> **Status**: NEGOTIATED. Doctrine [lance-graph#546] is merged (OGAR IS the
>   AR-shaped THINK/DO compiler — curators feed it, it does not adapt to them).
>   #546 landed **doctrine only — no code**. This plan is the *contract layer*
>   downstream of it: it turns the OpenProject curator-arm feedback into concrete
>   OGAR Core contract requirements. OGAR Core has accepted the split —
>   **items 1–5 are flip blockers (Phase A), items 6–12 are promotion candidates
>   (Phase B)**. The 5+3 council (consolidate + brutal critique) has run; findings
>   are folded in (§11).
> **Subordinate to**: #546 defines the doctrine; this defines the stable OGAR Core
>   contracts the OpenProject arm needs. Not to be smuggled into lance-graph
>   Inc 1 / Inc 2 — it is its own plan.
> **Audience**: OGAR Core + the consumer repos (`openproject-nexgen-rs`,
>   `AdaWorldAPI/surrealdb`).
> **Reads from**: `lance-graph/docs/OGAR_CONSUMER_API.md` (the iron rules) + #546.

## 0. The 30-second picture

```
  app/models/        ruff_ruby_spo::extract        ogar_from_ruff::lift_model_graph
    ─────────  ──────────────────────────────  ──────────────────────────────────
    *.rb files  →  ruff_spo_triplet::ModelGraph  →  Vec<ogar_vocab::Class>
                            │                              │
                  ruff_spo_triplet::expand                 ▼
                            │                  lance-graph-ontology::OntologyRegistry
                            ▼                  (mint_classid + register_class_path
                  Vec<Triple {s,p,o,f,c}>       + register_actions)
                            │                               │
              op-surreal-ast::triples_to_schema    classid → ClassView (#534) ─►
                            │                       ├ RegistryClassView (THINK)
                            ▼                       └ ClassActions (DO — §5.2)
              op-surreal-ast::Schema                       │
                            │                              ▼
              surrealdb-core::op_bridge        UnifiedStep at Executor::SurrealAst
                            │                              │
                            ▼                              ▼
              catalog::TableDefinition         the running surreal substrate
              + DDL render (DEFINE TABLE/FIELD/INDEX)
```

The narrow / SPO arm (top-right) lands DDL. The wide / OGAR arm (top-left) lands
`Class` → registry. Two legs of one harvest (one parser, two emitters), per
`OGAR_CONSUMER_API.md` §0.

## 1. The doctrine, in one sentence

The #546 spine: **Curators teach. OGAR compiles. LanceGraph thinks. SurrealAST +
Kanban orchestrate. Adapters obey.** The contract-layer corollary this plan puts
into practice (the line we want in the doctrine almost verbatim):

> **OGAR Core provides canonical semantic slots. Curator sessions may parse local
> syntax, but may not mint parallel semantic models. A curator succeeds when its
> syntax disappears at the extraction boundary and only OGAR-native relations,
> constraints, inheritance, actions, state machines, audit trails, and
> authorization facts remain.**

OpenProject-nexgen-rs is the **positive mirror** of the doctrine: Rails AR syntax
is metabolized into invariants, not transcribed as residue (the Odoo-flat-triples
failure mode). The litmus test: `ogar-from-rails`, `ogar-from-odoo`,
`ogar-from-foundry` must differ only in *how they scrape and filter*, never in the
*canonical shape they emit*. When that holds, OpenProject is the calibration
corpus — the reference implementation, not the privileged one. **OGAR keeps the
crown.**

## 2. Where we are (delivered, on `main`)

### `AdaWorldAPI/ruff` (producer)
- 57 closed-vocab predicates (`association_kind`, `class_name`, `inherits_from`,
  `validation_kind`, `validation_param`, …).
- `harvest_op` example → `/tmp/op_triples.ndjson` (9327 triples / 694 models).

### `AdaWorldAPI/openproject-nexgen-rs` (SurrealQL arm — narrow)
- `op_surreal_ast::triples_to_schema(triples) → Schema` lowers SPO → SurrealQL DDL.
- 82+ tests: FK direction, polymorphic fallback, typed scalars, `class_name`
  override w/ namespace stripping, STI parent separated from concerns, kind-aware
  ASSERT composition, parametric clauses (**i64-only safety gate**), composite
  UNIQUE, 15 class-level annotation lifts.
- **Litmus proof**: zero Rails tokens survive into the emitted DDL — `belongs_to`,
  `validates`, `acts_as_*` are metabolized into `record<>`, `ASSERT`,
  `UNIQUE INDEX`, table comments.

### `AdaWorldAPI/surrealdb` (bridge)
- `op_bridge::bridge_schema(ast::Schema) → Vec<BridgedTable>`, 22+ tests lowering
  `assert` strings to structural `Expr` (i64 RHS only).

### `AdaWorldAPI/OGAR` (vocab arm — wide)
- `ogar-from-ruff`: pure projection `ruff_spo_triplet::Model → ogar_vocab::Class`,
  12 tests. `ogar-from-rails`: walks Rails source → `Vec<Class>`. Live OP: 694.

### `AdaWorldAPI/lance-graph` (substrate — read-only consumer of mine)
- #534 `classid → ClassView`; #538 DO-arm contract + cycle-aware write gate;
  #544/#545 `Backend::MailboxSoa` + Hamming-plane distance; #546 doctrine (no code).

## 3. The agnostic flip — what makes the Rails shape curator-agnostic

The Rails shape is the closest thing we have to the vision. Making it *agnostic*
is a rename + reslot pass, **not** a redesign. Today `ogar_vocab::Class` carries
curator-named slots; the flip renames them to canonical slots and reslots the
three things Rails conflates (STI / abstract / concern). Grounded against the
**actual current `ogar-vocab/src/lib.rs`**:

| Today (`ogar_vocab::Class`)                              | Canonical (post-flip)            | Item |
|---------------------------------------------------------|----------------------------------|------|
| `associations: Vec<Association>`                        | `relations: Vec<Relation>`       | 6 |
| `validations: Vec<Validation>`                          | `constraints: Vec<Constraint>`   | 7 (`Constraint` semantic ≠ `Index` physical) |
| `callbacks: Vec<Callback>`                              | `lifecycle_hooks: Vec<LifecycleHook>` | 1 (rename) |
| `scopes` + `scope_predeclarations` + `default_scope`   | `query_views: Vec<QueryView>`    | 1; security-bearing scopes → `authorization` |
| `parent` + `inheritance_column_disabled` + `abstract_model` | `inheritance: Inheritance`  | 8 (one typed slot) |
| `mixins: Vec<String>`                                  | `trait_uses: Vec<TraitUse>`      | 8 (kept SEPARATE from inheritance) |
| *(leaks today as a DDL comment)*                        | `authorization: Vec<Authorization>` | 11 (`allowed_to`/`visible`) |
| *(dropped today)*                                       | `state_machines: Vec<StateMachineDecl>` | 9 |
| *(dropped today)*                                       | `audit_trail: Option<AuditTrailDecl>`   | 10 |

Once Core lands the slots, `ogar-from-rails` becomes a pure rename/reslot pass.
**No new producer IR.** That is the whole point — the producer never grows a
parallel object model.

## 4. Division of labor (negotiated)

### 4a. OGAR Core owns — Phase A flip blockers (1–5)
1. Canonical slot rename (deprecated aliases for one cycle, not a hard break).
2. `lance-graph-contract::action`: `predicate: PredicateName(Cow<'static, str>)`.
3. `OntologyRegistry::register_actions(class_id, &[ActionDef]) -> Result<(), RegistryError>`.
4. `OntologyRegistry::mint_classid(&ClassFingerprint) -> ClassId` — content-addressed.
5. Cargo feature isolation: `lance-graph-contract` stays substrate-free.

### 4b. OGAR Core promotes — Phase B canonical wave (6–12)
`Relation`, `Constraint`, `Inheritance`, `StateMachineDecl`, `AuditTrailDecl`,
`Authorization`, `OntologyRegistry::resolve_mro`. Shapes in §6.

### 4c. OP session owns locally — dies at the extraction boundary
`belongs_to`/`has_many` token parsing · `validates_*` class-name dispatch ·
`acts_as_*` family filtering · `class_name:` namespace stripping · block-callback
`target_method: None` normalization · ApplicationRecord-rooted STI detection ·
`include Tableless` → `table_name: None`. **Absent from the canonical facts.** The
OP extractor emits canonical facts, never Rails theology:
```
  OpenProject syntax:   belongs_to :project, class_name: "Project"
  OP adapter emits:     Relation { name: "project", kind: Reference,
                                   direction: SubjectToObject,
                                   target: Class(<project classid>),
                                   polymorphic: false, erasure: … }
  No `belongs_to` survives. 👑🚫
```

## 5. Phase A — the five flip-blockers, in detail

### 5.1 Canonical slot rename — with deprecated aliases
The rename of §3 lands on `ogar_vocab::Class` Core-side with `#[deprecated]` field
aliases held one release cycle so consumers migrate without a hard break.
`ogar-from-rails` switches names the same cycle; the SPO / DDL arm is unaffected
(it reads triples, not `Class`).

### 5.2 `PredicateName(Cow<'static, str>)` — kills `OwnedActionDef`
**Council brutal-2 was right; the prior draft was wrong.** A sibling
`OwnedActionDef { String, … }` + `Box::leak` interner (or `build.rs` const table)
is a rule-3 *parallel object model*. Negotiated fix:

```rust
pub struct PredicateName(pub Cow<'static, str>);   // contract-side newtype

pub struct ActionDef {
    pub predicate: PredicateName,   // const &'static OR owned-harvested
    pub object_class: u32,          // §7 — low-u16 classid widened here
    // exec / guard / required_role / overrides …
}
```

With `Cow<'static, str>` the contract `ActionDef` holds a harvested string
directly — no const table, no leak, no shadow type. The producer fact is the
*existing* `ogar_vocab::ActionDef` (already `String`-based); conversion to the
contract `ActionDef` happens at the registration boundary (§5.3). The newtype (vs
bare `Cow`) is Core's call and prevents string-soup later.

`ogar-from-ruff::lift_actions(model) -> Vec<ogar_vocab::ActionDef>` emits **facts
only**: predicate, object-class name, decorators. It does NOT set `exec` (backend
policy is consumer-private, rule §4) and does NOT construct `ActionInvocation` (a
live-instance carrier — §5.5).

### 5.3 `register_actions` on the registry
Only Core can validate identity:
```rust
impl OntologyRegistry {
    pub fn register_actions(&mut self, class_id: ClassId, actions: &[ActionDef])
        -> Result<(), RegistryError>;
}
```
Rules: `class_id` exists · predicate namespace-locally unique · no collision with
an inherited-MRO action unless the override is explicit · producer did not set
backend execution policy. **OP emits discovered action facts; OGAR decides
canonical identity.**

### 5.4 `mint_classid(&ClassFingerprint)` — content-addressed, no truncation
The single biggest correctness trap. **`u32 → u16` truncation is forbidden.**
```rust
pub fn mint_classid(&mut self, fp: &ClassFingerprint) -> ClassId;   // ClassId = u16
```
`ClassFingerprint` is deterministic from stable semantic identity
(`namespace + canonical_iri + source_domain + inheritance_root`), **not** from
transient extractor order. The registry stores `ClassFingerprint → ClassId` and
validates `NodeGuid.classid() → ClassId` (never a blind cast). This kills the
`classid_minter` closure the prior draft pushed onto the producer (that closure
moved Core identity policy into the adapter).

### 5.5 Cargo feature isolation — `lance-graph-contract` stays boring
**Hard blocker; council brutal-3's top blast-radius forecast.** Because
`surrealdb-core` depends on `lance-graph-contract`, any `mailbox-thoughtspace`
feature on the contract crate unifies (Rust features are additive) straight into
`op-surreal-ast`, dragging the Mailbox SoA runtime into a pure schema-projection
crate and breaking the zero-async-deps invariant. Rule:
```
  No mailbox-thoughtspace feature inside lance-graph-contract.
  No transitive feature that activates mailbox/ractor/thoughtspace via surrealdb-core.
  No default feature that pulls runtime substrate into schema projection.
```
If substrate types are needed, split crates: `lance-graph-contract` (stable, tiny,
no substrate) / `lance-graph-mailbox-contract` / `lance-graph-thoughtspace`.

## 6. Phase B — the promotion wave (6–12), agreed shapes

| # | Canonical | Shape (sketch) | Metabolizes |
|---|-----------|----------------|-------------|
| 6 | `Relation` | `{ name, direction: SubjectToObject\|Reverse, kind: Reference\|Composition\|Aggregation\|Derivation, target: Class(ClassId)\|Interface(Symbol)\|UnknownPolymorphic, polymorphic: bool, erasure: InterfaceTable\|Discriminator{field}\|JoinTable\|ExternalAdapter }` | Rails `belongs_to`/`has_many`, Odoo `many2one`, Foundry links |
| 7 | `Constraint` | `{ kind: Presence\|Length\|Range\|Format\|Inclusion\|Numericality\|Unique{scope}, params: BTreeMap<Symbol,ConstraintValue>, condition: Option<Expr> }` — semantic rule, **not** `Index` (physical) | Rails `validates`, composite uniqueness, Odoo `_sql_constraints` |
| 8 | `Inheritance` | `enum { Concrete{parent: ClassId}, Abstract, RootedAt{root: ClassId} }` + separate `TraitUse{ trait_id }` | STI parent / abstract base / concern — three organs, distinct |
| 9 | `StateMachineDecl` | `{ name, state_field, states: Vec<StateDecl>, transitions: Vec<TransitionDecl> }` — **THINK** (policy over time) | Rails `state_machine`/AASM, Odoo workflow, WoA doc lifecycle |
| 10 | `AuditTrailDecl` | `{ events, payload_shape: Option<ClassId>, retention_policy, regulatory_anchor: Vec<RegulationIri> }` — legal audit split from ordinary hooks | `acts_as_journalized`, Odoo chatter, WoA GoBD |
| 11 | `Authorization` | `{ role_predicates: Vec<PredicateName>, scope_kind, condition: Option<Expr>, enforcement_phase: QueryFilter\|CommandAdmission\|CommitGate\|AuditOnly }` | OP `visible`/`allowed_to` (query-plane) vs DO-arm commit-gate — **must not fuse** |
| 12 | `resolve_mro` | `OntologyRegistry::resolve_mro(class_id) -> Result<Vec<ClassId>, MroError>` — registry-owned (identity, cycle detection, trait order, cross-curator collision) | Rails STI chain + Odoo `_inherit` MRO via one lookup |

## 7. classid width — resolved (council contract-surface finding)

Verified against `lance-graph-contract` source: `class_view.rs:53` declares
`pub type ClassId = u16` (OD-CLASSID-WIDTH ratified); the `NodeGuid` layout
(`canonical_node.rs:23-35`) stores classid in bytes `0..4` as **u32 with the high
u16 canon-reserved zero**; `NiblePath::from_guid_prefix` returns `None` when
`classid >> 16 != 0` (lossy fold refused — no silent truncation);
`register_class_path(entity_type_id: u16, …)` is the narrowing seam; `NodeGuid::new`
already `assert!`s family/identity ≤ 24 bits. Not a blocker, not a casting nuance —
a **design asymmetry the producer must honor**: mint into the low u16 only (§5.4),
widen to u32 at the `ActionDef::object_class` boundary. **No contract violation.**

## 8. Mailbox-SoA blast radius — verified zero

Council grep across `openproject-nexgen-rs`, `surrealdb/core/.../op_bridge.rs`,
and `/tmp/ogar/crates/{ogar-from-ruff,ogar-from-rails,ogar-from-elixir}` for
`MailboxSoa|write_row|BindSpace|WriteOutcome|BackingStoreWrite` — **no matches**.
Every producer-side crate emits static class shape and never instantiates a SoA
row. Forward rules: **do not** wire to `BackingStoreWrite::Singleton(&BindSpace)`
(the cycle-blind arm deleted in W7); **do** treat
`WriteOutcome { Accepted | Stale | Future }` as the load-bearing contract for any
future runtime write path (Stale-as-outcome, never a panic).

## 9. Phased implementation order

```
Phase 0  Contract safety   freeze lance-graph-contract feature policy (§5.5);
                           publish ClassId mint rule (§5.4);
                           add PredicateName(Cow<'static,str>) (§5.2)
Phase A  Flip blockers     1 canonical rename · 2 PredicateName ·
                           3 register_actions · 4 mint_classid ·
                           5 feature isolation guarantee
Phase B  Promotion slots   6 Relation · 7 Constraint · 8 Inheritance ·
                           9 StateMachineDecl · 10 AuditTrailDecl ·
                           11 Authorization · 12 resolve_mro
Phase 5  OP flip           ogar-from-rails emits only canonical slots;
                           op-surreal-ast proves invariant projection
```
Phase 0/A/B are Core-side. Phase 5 is this session — it cannot start before the
rename + promotion slots land (starting earlier = the speculative rework the
council warned against).

## 10. Test surface (corrected)

| Step | DoD |
|------|-----|
| `lift_actions` | `lift_actions_emits_one_def_per_method`, `lift_actions_sets_no_exec_policy`, `lift_actions_predicate_is_snake_case` |
| real-OP smoke | corrected expectation: **~1800–1900** `ActionDef` from 694 classes (council: raw ~2643 before non-action filtering). Prior "≥8000" was wrong. **`Function::visibility` does not exist in the ruff IR** — no visibility-based filter; filter on action-shape, not access modifier. |
| `register_actions` | `register_actions_rejects_unknown_class_id`, `register_actions_rejects_unflagged_mro_collision`, `mint_classid_is_deterministic_over_fingerprint` |
| Phase-5 flip | `rails_tokens_absent_from_canonical_facts` (litmus), `relation_count_matches_association_count`, `inheritance_separates_sti_from_traits` |

## 11. Council findings + upstream doctrine dependencies

### 11.1 Council findings folded in
- **Internal contradictions resolved.** Prior §5-step-5 (`DEFINE FUNCTION`) vs §8
  (body-lift out of scope): the DO→SurrealQL projection emits the function
  *declaration shell* (signature + commit-gate guard); Ruby-body transcription
  stays deferred (D-AR-5.6). Prior §3-rule-1 ("I stop at `Class`") vs the classid
  binding step: classid minting is now Core's (`mint_classid`), so the producer
  genuinely stops at `Class` + facts.
- **`op-surreal-actions` is a new crate**, not an extension of `op-surreal-ast`
  (preserves the zero-async-deps invariant `surrealdb-core` consumes).
- **`ActionInvocation` stays producer-absent** (it carries `NodeGuid
  object_instance` + `cycle` + idempotency/trace — none knowable at harvest).
- **`OwnedActionDef`, `Box::leak`, the `classid_minter` closure, producer-side
  `exec` defaults** — all four killed (§5.2, §5.4).

### 11.2 Upstream doctrine dependencies (OGAR Core cleanup PR — not this session)
Two phrases in the merged #546 doctrine still read with the old drift; this plan's
`exec`/Executor references assume the **post-council** wording. Flagged here for
the OGAR Core cleanup PR (the doctrine is Core's artifact, not the OP arm's to
edit):
1. **"ractor-**owned** LanceGraph SoA"** → "ractor-**proven**" / "mailbox-owned,
   ractor-proven at compile time". ractor is the compile-time ownership proof, not
   runtime mutation authority (§9 of the detailed ownership section already says
   so — the ladder phrase lags it).
2. **The doctrine sample `Executor` enum** still lists `ExternalHttp(Url)` (breaks
   zero-dep), `Dll(CapabilityId)` (phantom), and named `OdooAdapter` /
   `RailsAdapter` (concrete adapters belong to the callcenter registry, not the
   contract). Post-council slim shape this plan targets:
   ```rust
   enum Executor { NativeLance, SurrealAst, HumanKanban, Adapter(AdapterTargetId) }
   type AdapterTargetId = &'static str;   // concrete adapters: callcenter registry
   ```

## 12. Out of scope

- DB schema (`db/schema.rb` / migrations) — D-AR-3.7 sprint.
- Constant resolution (`SCOPE_COLS` / `MAX_NAME_LENGTH`) — cross-file analysis.
- Ruby method body → SurrealQL function body — D-AR-5.6 sprint.
- Odoo (Python) arm — `ogar-from-python` / `ruff_python_spo`, parallel work.
- Adapter-local dialect that must **never** be promoted: CarrierWave
  `mount_uploader`, `paper_trail` gem wiring, `closure_tree` impl details, Rails
  callbacks-as-callbacks, OP-specific `acts_as_*` variants.
