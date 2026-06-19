# ClassView materialization plan — wiring the canonical → askama seam

> Status: the bridge crate (`ogar-class-view`) lands in this PR. Templates +
> consumer wiring are the **+5+3** follow-ons enumerated below.

## What just landed

The seam from the **calcified inner shape** (`ogar_vocab::Class` + codebook id)
onto the **classview-parameterized outer materialization** surface (askama, in
`lance-graph-contract::class_view`'s framing).

```text
ogar_vocab::Class            ──┐                          ┌──► Rust struct
  (codebook id, attrs,         │                          │
   family edges)                │   ogar-class-view       │──► TS interface
                                 │     (this PR)           │
ogar_vocab::class_ids::PROJECT_WORK_ITEM   │──► askama ─►  │──► SurrealQL TABLE
                                 │                         │
lance_graph_contract::            │                         │──► OpenAPI schema
  ClassView trait                ──┘                         │
                                                            └──► (target N+1)
```

The XSD analogy from `lance-graph-contract::class_view`:

| Layer | Today |
|---|---|
| SoA row | the XML document — agnostic bytes |
| ObjectView (field set) | the XSD schema — bit basis |
| `OgarClassView` (this crate) | the parser+schema — projects row → typed view |
| `FieldMask` | optional-elements presence |
| askama template (deferred) | the XSLT — renders the projection |

## The bridge contract

`ogar-class-view` exposes one impl: `OgarClassView`. Construct once at
startup; the registry walks every promoted canonical concept (`project()`,
`project_work_item()`, `billable_work_entry()`, … the 32 entries in the
codebook) and lifts each into an `ObjectView`:

- **`FieldRef` order = N3 stable, append-only**: attributes first (declaration
  order), then family-edge associations (declaration order). A test pins it.
- **`ClassId` = `ogar_vocab::canonical_concept_id`** (same `u16`).
- **`render_rows(class_id, mask)`** is the askama input: list of populated
  `(label, predicate_iri)` rows in field order, off-bits skipped.
- Unknown `ClassId` → empty field set (no panic).

## +5 templates (the askama kit)

One template per **artifact kind**, parameterized via classview-supplied
variables. The kit is the 7-70 number; the first 5 cover the targets
already in flight on the ecosystem.

| PR | Template | Target | Consumers |
|---|---|---|---|
| **T1** | `rust_struct.askama` | Rust `struct` (newtype, Serialize/Deserialize, `pub const CLASS_ID: u16`) | `op-models`, `rm-models` (future) |
| **T2** | `ts_interface.askama` | TS `interface` + `class_ids.ts` | OpenProject frontend, future Redmine TS port |
| **T3** | `surrealql_table.askama` | SurrealQL `DEFINE TABLE` + field defs | `op-surreal-ast`, `ogar-adapter-surrealql` |
| **T4** | `openapi_schema.askama` | OpenAPI 3.1 `components.schemas.{X}` | API spec generation; SDK clients |
| **T5** | `node_guid_routing.askama` | Rust `match` arm dispatch keyed on `ClassId` for `NodeGuid`-shaped graph entries | `lance-graph-planner`, kanban router |

Each PR is small (one template + a tiny render harness + a `cargo check` on
the emitted artifact). Shared invariant: every template uses the same
`(OgarClassView, ClassView_bits, ClassId, FieldMask)` context. **No template
hardcodes a class name.**

## +3 calibration + consumer wiring

| PR | What | Repo |
|---|---|---|
| **C1** | `op-codegen-projection` / `op-codegen-pipeline` adopt `OgarClassView` + askama for the templated paths currently using `format!`. Calibration test: emitted Rust still passes `cargo check`; emitted TS still passes `tsc --noEmit`. | `openproject-nexgen-rs` |
| **C2** | `redmine-canon` exposes `OgarClassView` re-exported from `ogar-class-view` (no logic), so `rm-*` domain crates (future) consume one canonical bridge. Symmetric move with `op-canon` already re-exporting `class_ids`. | `redmine-rs` |
| **C3** | `op-surreal-ast::from_class_view` — adapter from `OgarClassView` to the existing typed `Schema` AST (which is byte-identical-pinned). Lets the SurrealQL emission path consume the canonical shape *without* breaking the byte-identical-output pin: ClassView feeds the typed AST, the AST emits SurrealQL the same way it always has. | `openproject-nexgen-rs` |

## Conformance gates (the calibration loop)

Each of T1-T5 + C1-C3 lands with:

1. **Round-trip test on the canonical layer**: emit → re-parse → assert
   structural equality where the target supports it (Rust via `syn`; TS via
   `swc`/`@babel/parser`; SurrealQL via `op-surreal-ast`).
2. **Compilation test**: emitted artifact compiles in its target toolchain
   (`cargo check`, `tsc --noEmit`, SurrealQL parse).
3. **ClassView drift guard**: every `FieldRef` referenced in a template is
   present in the registry for the rendered class (test renders against
   `OgarClassView`, asserts no missing slots).

The cascade — codebook calcified → `OgarClassView` typed → askama bound to
typed context → emitted artifact compiled — means a misstep anywhere fails
*before* runtime. That is the "Apple/iPhone" inner+outer integration:
declarative seams instead of negotiated ones.

## Out of scope for this plan

- ClassView **bitmask semantics per target** (which bits flip a slot
  visible/hidden for Rust vs TS vs SurrealQL). Belongs in T1-T5 as each
  template asserts what it needs; the contract emerges from the templates,
  not pre-designed.
- DOLCE upper-category dispatch (`dolce_category_id`): the trait method is
  implemented to return `0` (unclassified) today. Live DOLCE comes from
  `lance-graph-ontology` when the consumer wires it.
- Persistence / serde on `ObjectView`. The registry is built in-process from
  the canonical class fns; nothing reads files in this crate.

## Risks + mitigations

- **`FieldMask` is `u64` — 64 fields max.** Today's widest canonical class
  (`billable_work_entry`, 12 family edges + a handful of attributes) is well
  under. Test `field_basis_fits_in_one_u64_mask` fires if any future class
  exceeds. Mitigation if it ever does: paginate via class hierarchy (the
  contract's documented escape — see `lance-graph-contract` L0b).
- **`build.rs` of `lance-graph-contract` reads `modules/*/manifest.yaml`
  from its checkout root.** As a git dep, that's the lance-graph checkout
  cache — the manifests are checked out alongside the crate, so this works.
  Sanity-checked locally on `cargo check`.
- **Template count creep.** The kit is bounded by *artifact kind*, not by
  (class × target). New concepts cost zero new templates (they flow through
  the existing kit via `OgarClassView`). New targets cost one template +
  one classview bit position.

## Pin

The bridge is the **only** new artifact required to begin templating.
Everything else (T1-T5, C1-C3) is downstream and can land sprint-by-sprint
without re-touching this crate.
