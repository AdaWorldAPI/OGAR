# OGAR Northstar — ClassView Materialization Plan

> **Calcified plan.** This document encodes the architectural insights of
> the sprint that landed `ogar-class-view` (PR #77) and
> `ogar-render-askama` (PR #78). It exists so a future session can pick up
> the work without re-deriving conclusions or drifting onto wrong paths
> already explored and rejected here.
>
> **Read the "Invariants" + "Anti-patterns" sections before touching any
> code in this lane.** If a proposed change would violate one of them, the
> change is wrong — the design has already settled the question.

## 0. TL;DR — the picture

```text
        OGAR codebook (32 promoted concepts, 0xDDCC ids)              ◄── calcified inner shape
                │
                │ class fns: project_work_item(), billable_work_entry(), ...
                ▼
        ogar_vocab::Class    (typed: attributes + family edges + canonical_concept)
                │
       ─────────┼─────────────────────────────────────────────────────
       │ BUILD-TIME (codegen)        │ RUN-TIME (SoA projection)      │
       ▼                              ▼                                │
  ogar-render-askama        ogar-class-view (OgarClassView)            │
  consumes &Class           consumes ClassView trait                   │
  emits .rs / .ts / .surql  produces RenderRow stream                  │
  via askama templates      via lance-graph-contract::ClassView        │
       │                              │                                │
       ▼                              ▼                                │
  source files               late-bound display rows                   │
  (calibrated by             (calibrated by                            │
   target compilers)          consumer renderers)                      │
       └──────────────┬───────────────┘                                │
                      ▼                                                │
              same canonical contract                                  │
              (codebook id, N3 field order)                            │
       ─────────────────────────────────────────────────────────────────
```

**Two pipelines, one canonical source, one codebook. They do not compete.
They answer different questions.**

## 1. Invariants — never violate these

These are the load-bearing constraints. If a future session is about to
break one, stop and reread Sections 4 & 5 (anti-patterns + epiphany log).

1. **The codebook is calcified.** Class ids in `ogar_vocab::CODEBOOK` are
   stable forever. Ids only arrive (next free slot in the right domain
   block); they never move, never get re-assigned. `0x01XX` =
   project-mgmt, `0x02XX` = commerce, `0x07XX` = OGIT, `0x08XX` = OCR,
   `0x09XX` = Health (reserved). Width is `u16` per
   `lance-graph-contract` `OD-CLASSID-WIDTH`.
2. **N3 field order, append-only.** `FieldMask` bit `n` is the `n`-th
   `FieldRef` in the class's ordered field set. Order is: typed
   attributes (declaration order in the class fn) **first**, then family-
   edge associations (declaration order) **second**. Once instances
   persist, positions are append-only — never reorder, never drop, only
   append.
3. **`Class` carries types; `ClassView` carries labels.** Type information
   (`Attribute.type_name`, `Association.kind`/target) lives on `Class`.
   `ClassView` is intentionally label-only late-binding (`RenderRow {
   label, predicate }`) because the SoA row carries nothing semantic — the
   OGIT cache resolves labels at projection time. Do **not** add type
   info onto `ClassView` to "unify" with codegen; see Section 4
   anti-pattern #2.
4. **`ClassId` == `canonical_concept_id`.** Same `u16`, ratified by
   `OD-CLASSID-WIDTH`. They are one number, not two; never translate.
5. **N3 + ≤64.** `FieldMask` is `u64`. No promoted class may have more
   than 64 slots. Test `field_basis_fits_in_one_u64_mask` enforces; if a
   future class crosses, paginate via class hierarchy (the documented
   contract escape — `lance-graph-contract` L0b).
6. **Templates are mass-mail simple.** A template is a bag-of-variables
   substituter (jinja syntax via askama). The binding struct just supplies
   the variables that template asks for. Two templates with different
   needs don't share a binding struct — they each declare their own bag.
   See Section 5 epiphany #4.

## 2. The two pipelines, in detail

### 2.1 Build-time codegen — `ogar-render-askama` (PR #78)

**Purpose**: emit *source files* (Rust, TypeScript, SurrealQL DDL,
OpenAPI schemas, …) from canonical class definitions at build time.

**Structurally a mirror of `AdaWorldAPI/woa-rs::crates/codegen`**
(RFC-v02-006: WoA's `RouteSpec` + `HandlerKind` + per-kind askama
template). The same kit shape, with `ogar_vocab::Class` as the typed
input instead of a JSON-loaded `RouteSpec`.

| WoA-rs | OGAR |
|---|---|
| `RouteSpec` | `ogar_vocab::Class` |
| `enum HandlerKind` (13) | `enum ArtifactKind` (5 today, append-only) |
| `trait HandlerKindEmitter` | `trait ArtifactEmitter` |
| `for_kind(s) -> Box<dyn …>` | identical |
| `templates/_dispatch/list_view.html` | `templates/dispatch/rust_struct.askama` |
| Phase-0: 1 real + 12 stubs | Phase-0: 1 real (RustStruct) + 4 stubs |

**Bounded template count.** Adding a new canonical concept costs **zero
templates** — flows through the existing kit via class data. Adding a
new target = one new `ArtifactKind` variant + one askama template; every
promoted concept emits through it automatically. **This is the 800 →
7-70 collapse the design is named after.**

### 2.2 Run-time projection — `ogar-class-view` (PR #77)

**Purpose**: project SoA rows through the
`lance-graph-contract::class_view::ClassView` trait at query/render time.
SoA row carries `(class_id, FieldMask)`; the registry resolves to typed
display rows.

**Contract from `lance-graph-contract` (already shipped):**

- `ClassId = u16`.
- `FieldMask(u64)` — presence bits, N3 append-only.
- `FieldRef { predicate_iri, label }` — late-resolved from cache.
- `ObjectView { display_template, fields, primary_label }`.
- `trait ClassView` — `fields(class)`, `template(class)`, `project(class,
  mask) → ClassProjection`, `render_rows(class, mask) → Vec<RenderRow>`.
- `RenderRow { label, predicate }` — what an askama projection template
  iterates.

**`OgarClassView`** is the `impl ClassView` over the 32 promoted concepts,
keyed by `canonical_concept_id`. Walked at startup; read-only after.

### 2.3 The muscle-memory loop (run-time)

```rust
// SoA row knows its class_id; registry knows its ClassView; the rest is glue.
let view: &OgarClassView = /* held by the consumer */;
let rows: Vec<RenderRow> = view.render_rows(row.class_id, row.mask);
// rows feed an askama projection template → HTML/JSON/whatever the consumer renders.
```

Five lines at the call site. Architecture is **already in place** —
adding a new projection target is "write a template + a small binding
struct that names the variables the template wants" (see Section 5
epiphany #4).

### 2.4 Where they cross — the `UnifiedBridge` layer (lance-graph-callcenter)

```text
public name "WorkPackage"
       │
       ▼
  NamespaceBridge<OpenProject>.entity("WorkPackage")    [lance-graph-ontology]
       │
       ▼
  UnifiedBridge<OpenProjectBridge>.authorize_read(...)  [lance-graph-callcenter]
       │
       ▼  EntityRef { schema_ptr }
       │  schema_ptr.entity_type_id() == OGAR codebook id  (DECISION-3: GLOBAL id)
       ▼
  OgarClassView.render_rows(class_id, mask)             [this lane, PR #77]
       │
       ▼  Vec<RenderRow>
       │
       ▼
  askama projection template                            [follow-on, this plan §3]
       │
       ▼
  rendered output
```

The convergence proven on the redmine ↔ openproject corpora extends here:
a Redmine `Issue` and an OpenProject `WorkPackage`, resolved through their
respective `NamespaceBridge`s, both hit `entity_type_id() == 0x0102`
(`PROJECT_WORK_ITEM`). The *same* `OgarClassView` arm projects both.
**Convergence becomes runtime-operational, not just snapshot-pinned.**

## 3. The +5+5 follow-on PRs (concrete, after #77/#78 merge)

### +5 askama templates — the artifact-kind kit (in `ogar-render-askama`)

| PR | ArtifactKind | Template | Target syntax |
|---|---|---|---|
| T1 ✅ | `RustStruct` | `dispatch/rust_struct.askama` | Rust struct + `pub const CLASS_ID` (PR #78 ships this as proof-of-shape) |
| T2 | `TsInterface` | `dispatch/ts_interface.askama` | TypeScript `interface` + matching `class_ids.ts` entry |
| T3 | `SurrealqlTable` | `dispatch/surrealql_table.askama` | `DEFINE TABLE` + per-field `DEFINE FIELD` |
| T4 | `OpenapiSchema` | `dispatch/openapi_schema.askama` | OpenAPI 3.1 `components.schemas.{X}` JSON |
| T5 | `NodeGuidRoutingArm` | `dispatch/node_guid_routing_arm.askama` | Rust `match` arm dispatching on `ClassId` |

Each PR: 1 template + 1 binding struct + 1 emitter impl + tests
(round-trip + target-toolchain compile + golden-snapshot of the first
emitted concept).

### +5 consumer wirings (across repos)

| PR | What | Repo |
|---|---|---|
| C1 | `op-codegen-projection` adopts `ogar-render-askama` for paths currently using `format!` | `openproject-nexgen-rs` |
| C2 | `redmine-canon` re-exports `OgarClassView` (symmetric with the existing `class_ids` re-export) | `redmine-rs` |
| C3 | `op-surreal-ast::from_class_view` adapter — feeds the canonical shape into the byte-identical-pinned typed AST | `openproject-nexgen-rs` |
| C4 | `OpenProjectBridge: impl NamespaceBridge` — `g_lock` to "OpenProject" namespace, public-name dictionary populated from `op-canon` snapshot. `entity_type_id()` returns OGAR codebook ids. | `openproject-nexgen-rs` (sibling of `MedcareBridge`) |
| C5 | `RedmineBridge: impl NamespaceBridge` — symmetric, "Redmine" namespace, same codebook ids. | `redmine-rs` |

After C4 + C5: a consumer holding `UnifiedBridge<OpenProjectBridge>` AND
`UnifiedBridge<RedmineBridge>` resolves `"WorkPackage"` OR `"Issue"`
through the policy-evaluated, audit-chained path → gets an `EntityRef`
whose `class_id` routes to the *same* `OgarClassView` arm.

### Calibration gates (every PR above lands with these)

1. **Round-trip parse**: emit → re-parse → structural equality. Rust via
   `syn`, TS via swc/babel, SurrealQL via `op-surreal-ast`.
2. **Target-toolchain compile**: `cargo check` / `tsc --noEmit` /
   SurrealQL parse on the emitted artifact.
3. **ClassView drift guard**: every `FieldRef` referenced in a template
   is present in the registry for the rendered class.

The cascade — codebook calcified → `OgarClassView` typed → askama bound
to typed context → emitted artifact compiled → consumer drift-guarded —
means a misstep anywhere fails *before* runtime.

## 4. Anti-patterns — drifts already explored and rejected

A future session that proposes one of these is rediscovering a dead end.
Pull up this section before doing the work.

### Anti-pattern #1: Bifurcate templates by static-vs-dynamic data shape

**Form**: "templates need different kits for build-time vs run-time".

**Why wrong**: askama doesn't care whether the data is build-time or
run-time. It takes a binding struct, substitutes its variables. The
distinction is the *binding struct contents*, not the template engine.
The two pipelines (codegen / projection) consume different shapes
because they answer different *questions*, not because askama imposes a
split.

### Anti-pattern #2: Add `typed_rows`/`type_kind` to `ClassView`

**Form**: "extend `lance-graph-contract::FieldRef` with type info so
codegen templates can iterate through `ClassView` like projection
templates do."

**Why wrong (the bicycle):** `ClassView` is a *label-resolver* designed
for nothing-on-the-row late binding (SoA rows carry only `class_id +
mask`; the cache resolves labels). Codegen has the *whole* `Class` in
memory at build time — types and all. Going through `ClassView` is a
needless indirection that bolts a typed contract onto a contract
designed to be untyped. The right tool for codegen is `Class` directly.
"Driving a bicycle" when you have a car parked next to you.

### Anti-pattern #3: One template = one (class × target) pair

**Form**: "to add a new canonical concept, write a template per target
for it."

**Why wrong**: that's the 800-template explosion the kit was built to
avoid. The 7-70 number is `(artifact kinds) + (artifact kinds × targets
beyond the first)` — never `(classes × targets)`. New concept = zero
templates; new target = one template per artifact kind it cares about.

### Anti-pattern #4: A2UI / Flutter / OutputFormat-X as an architectural concern *now*

**Form**: "we should fold A2UI into the kit / build a Flutter renderer
inline / unify with DUSK or MUIBridge".

**Why wrong**: A2UI is an output target with its own shape; folding it in
now would couple the canonical layer to a specific JSON-payload schema
before the kit's basics ship. Roadmap-only (Section 7) until T1–T5 lands
the foundational shape; **then** A2UI is one more `ArtifactKind` variant.

### Anti-pattern #5: Skip the codebook calcification, ship the templating first

**Form**: "we can build the templating layer now and promote concepts
later."

**Why wrong**: the codebook calcification was the *prerequisite*. With
unstable ids, templates can't bind to constants; consumers can't
dispatch on identity; drift surfaces silently. The fact that the inner
shape is calcified (26/26 cross-fork convergence proven on real corpora)
is what makes the templating layer's claims meaningful.

### Anti-pattern #6: Iteration through `HashMap`/random-order surfaces

**Form**: "`known_class_ids()` / similar surfaces are fine returning
`HashMap`'s iterator."

**Why wrong**: generated artifacts (structs, snapshots, drift guards)
must be byte-stable across process runs. Codex P2 on PR #77 caught
exactly this. Fixed; future surfaces should iterate
`ogar_vocab::class_ids::ALL` or sorted collections.

### Anti-pattern #7: Unescaped Rust keyword field names

**Form**: "just emit `pub {{ name }}: {{ ty }}` from the template".

**Why wrong**: curator-side names follow curator conventions, not Rust
ones. `project_actor()` declares `type` (Rails STI); the unescaped form
emits `pub type:` (illegal). Codex P1 on PR #78 caught this. Fix is
`escape_rust_ident` mapping reserved words to `r#name`. Same hazard on
other targets (TS reserved words, SurrealQL keywords); every emitter
needs the equivalent.

## 5. Epiphany log — high-signal architectural moments

Captured verbatim because future sessions will ask "why".

1. **"Rails words die, the invariant lives"** (snapshot rule).
   `Issue/WorkPackage`, `Board/Forum`, `Tracker/Type`, `IssueStatus/Status`
   diverged across the Redmine → ChiliProject → OpenProject fork. The
   canonical id did not. 26/26 cross-fork convergence proven on real
   corpora pins this in code (`redmine-canon::ForkConvergence`).

2. **"OGAR removes the whole JSON PostgreSQL grind."** Custom-field jsonb,
   serialized `Setting.value`, `Journal` STI per-field diffs, hand-rolled
   API representers — all symptoms of missing typed schema. The
   canonical layer + classview is the typed schema applied at every
   seam. iPhone/Apple effect = declarative seams instead of negotiated
   ones.

3. **"Class inherits the labels — it's the task of the class to carry
   type awareness."** Types live on `Class`. `ClassView` is for dynamic
   SoA where the row has no semantics. Don't bolt types onto ClassView
   (Anti-pattern #2 / the bicycle).

4. **"Templates look as simple as an Outlook mass-mail."** Don't wrap
   ceremony around a substitution language. A template names variables;
   the binding struct supplies them. Two templates with different needs
   each declare their own bag — they don't share a contract. Stop
   reasoning about input contracts; just write the bag the template
   asks for.

5. **"Dynamic-as-classview at compile time doesn't suddenly become
   static."** The classview's dispatch (bitmask, late label resolution)
   is dynamic by nature, regardless of when it's invoked. But the
   pipelines for codegen (consumes `Class` directly) and projection
   (consumes `ClassView`) are legitimately different — different
   *questions*, same canonical source.

6. **"When the SoA knows it's classview, how hard can it be — that's
   just muscle memory."** The five-line glue at the call site (Section
   2.3). The architecture is already in place; what remains is plumbing.

7. **"Askama IS jinja-rs."** askama / minijinja distinction is plumbing,
   not language. Picked askama because `lance-graph-contract::class_view`
   explicitly names it ("askama template = the XSLT" docstring) and
   WoA-rs's codegen pattern is the established structural antecedent.

## 6. Prior art — kept as design lineage

These repos converged on the same northstar from different angles:

- **`AdaWorldAPI/woa-rs::crates/codegen`** (RFC-v02-006). The structural
  antecedent of `ogar-render-askama`. `RouteSpec` + `HandlerKind` +
  per-kind template = same shape, different domain. Live.
- **`AdaWorldAPI/A2UI`** (v0.8 public preview, TS/Flutter/Angular/Lit).
  "Agent emits declarative JSON UI intent, client renders with trusted
  catalog." Same north star, output side. **Roadmap T6**, not now.
- **`AdaWorldAPI/DUSK_Solution`** (.NET 8). Multi-renderer scene system
  with theme/mood as classview-bits-style dispatch. Same insight in a
  different ecosystem. Lineage only, no active deps.
- **`AdaWorldAPI/MUIBridge`** (.NET). Bridge pattern of the same shape
  as `NamespaceBridge`/`UnifiedBridge`. Lineage only.

## 7. Roadmap — T6 and beyond, after the +5+5

| Item | What | When |
|---|---|---|
| T6 | `A2uiPayload` artifact kind — emit any promoted concept as an A2UI v0.8 intent payload. A2UI's "catalog of trusted components" maps 1:1 to the OGAR codebook. Existing A2UI renderers (Flutter/Angular/Lit) consume the canonical layer with zero Ruby/Rails coupling. | After T1–T5 stabilises |
| T7 | Calibration-loop receipt aggregator — collect every PR's three gates into one CI artifact so coverage can be reasoned about as a ratio | After several Cn land |
| T8 | DOLCE upper-category live wiring — `OgarClassView::dolce_category_id` currently returns `0`; live values come from `lance-graph-ontology` when the consumer chooses to wire them | Independent; opportunistic |
| T9 | Promote more OpenProject-only concepts (Costs, Budgets, BIM, Meetings, Storages — the 880 OP-only classes from the corpus diff). Each gets its own codebook block (likely `0x03XX`+) | Demand-driven |
| T10 | Convergence proof for the commerce arm (`OSB ↔ Odoo`) mirroring the project-mgmt arm — same shape, different domain, run by parallel session | Parallel-session work |

## 8. Open PR map (as of writing)

| Repo | PR | What | State |
|---|---|---|---|
| OGAR | **#77** | `ogar-class-view` (run-time bridge) — this plan lives here | CI ✅, codex P2 fixed (stable order) |
| OGAR | **#78** | `ogar-render-askama` (build-time codegen) | CI ✅, codex P1 fixed (Rust keyword escape) |

After these merge, queue T1 (RustStruct emitter already in #78), then
T2–T5 sequentially. C-series PRs (C1–C5) land in parallel with the
T-series since they're cross-repo.

## 9. Pointer to the convergence artifacts

The proof these design choices were worth making lives at:

- `AdaWorldAPI/redmine-rs:crates/redmine-canon/data/redmine.ogar.json` —
  26 canonical concepts, every id in `0x01`, from 111 Redmine classes.
- `AdaWorldAPI/redmine-rs:crates/redmine-canon/data/fork_convergence.json` —
  26/26 shared between Redmine and OpenProject (922 OP classes via
  engine-walking, 111 Redmine), schema `fork-convergence/2`.
- `AdaWorldAPI/openproject-nexgen-rs:crates/op-canon/data/op.ogar.json` —
  OP side, 922 classes harvested, 26 promoted (same set).

These three files are why the design is staked on a calcified inner
shape: it's calcified *because* the convergence is proven, not because
we hope it'll converge.

---

**End of plan.** Read Sections 1, 4, 5 before any future work in this
lane. The northstar is stable; drift comes from forgetting why a path
was chosen.
