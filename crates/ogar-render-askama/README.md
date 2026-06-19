# `ogar-render-askama`

Build-time askama codegen harness over the calcified canonical layer.
One [`ArtifactKind`] enum, one askama template per kind, one
[`ArtifactEmitter`] trait, one [`for_kind`] dispatcher. Consumes
[`ogar_vocab::Class`] as the typed input.

## Structural antecedent

This crate mirrors [`AdaWorldAPI/woa-rs`](https://github.com/AdaWorldAPI/woa-rs)
`crates/codegen` (RFC-v02-006: "route codegen for the WoA → woa-rs port").
WoA's structure:

| WoA-rs concept | OGAR analog |
|---|---|
| `RouteSpec` (JSON-loaded) | `ogar_vocab::Class` (in-process from class fn) |
| `enum HandlerKind` (13 variants) | `enum ArtifactKind` (5 today, append-only) |
| `trait HandlerKindEmitter` | `trait ArtifactEmitter` |
| `for_kind(s) -> Box<dyn …>` | identical |
| `templates/_dispatch/list_view.html` | `templates/dispatch/rust_struct.askama` |
| Phase-0: one real emitter (`list_for_tenant`) + stubs | Phase-0: one real emitter (`RustStruct`) + stubs |

The bounded-template-count proof from WoA carries over: adding a new
canonical concept (one OGAR class fn) costs **zero new templates** — it
flows through the existing kit via the codebook + class data.

## What this is NOT

- **Not** the run-time projection layer. That is
  [`lance-graph-contract::class_view::ClassView`](https://github.com/AdaWorldAPI/lance-graph/blob/main/crates/lance-graph-contract/src/class_view.rs)
  — `ClassId + FieldMask → Vec<RenderRow>`, late-resolved labels, presence
  bits. OGAR-side `ClassView` impl lives in `ogar-class-view` (sister
  crate; PR #77). Both pipelines are askama-templated; both share the N3
  field order convention. They consume different shapes.
- **Not** a runtime template engine. Templates are compile-time-bound
  via `#[derive(Template)]`; the binary ships with them inlined.

## Phase-0 state

- `ArtifactKind::RustStruct` — **real** emitter + template. Emits a Rust
  struct + `pub const CLASS_ID: u16` constant + family-edge fields from
  the canonical class.
- `ArtifactKind::TsInterface` / `SurrealqlTable` / `OpenapiSchema` /
  `NodeGuidRoutingArm` — **stubs** (compilable; emit a marker comment).
  Concrete templates land in follow-on PRs T2–T5 per the integration plan.

## Roadmap (informational — not in this PR)

- **T2–T5**: each remaining `ArtifactKind` gets a concrete askama
  template + emitter. Same `(class, kind)` context shape across all.
- **T6 — A2UI payload** (deferred):
  [`AdaWorldAPI/A2UI`](https://github.com/AdaWorldAPI/A2UI) v0.8's
  "declarative JSON UI intent payload" is the output side of the same
  northstar. Adding it = one more `ArtifactKind::A2uiPayload` variant +
  one askama template; the A2UI renderers (Flutter / Angular / Lit) then
  consume the canonical layer with no Ruby/Rails coupling.
- **Prior art (no active deps)**: `DUSK_Solution` (multi-renderer
  scenes + theme/mood; .NET), `MUIBridge` (bridge pattern, .NET).
  Both encode the same insight; kept as design lineage.

## Quick check

```bash
cargo test -p ogar-render-askama   # 6/6 unit tests
```

## Layering in the OGAR stack

```text
ogar-vocab           (codebook + Class fns)            ← source of truth
      │
      │  pure construction at build time
      ▼
ogar-render-askama   (this crate — askama emitters per kind)
      │
      ▼
.rs / .ts / .surql / .json source text                 ← downstream consumers
```
