# openstreetmap-website-rs

A Rust transcode of [`openstreetmap/openstreetmap-website`](https://github.com/openstreetmap/openstreetmap-website)
(Ruby on Rails) produced through the **ruff → OGAR** transpile pipeline.

The Ruby source is canonical; this repo is the target. The domain model is
harvested mechanically from the Rails `app/models` tree — not hand-ported — so
the Rust structure tracks the Ruby structure by construction.

## Pipeline

```
openstreetmap-website/app/models/*.rb   (Ruby, canonical)
        │  ruff_ruby_spo::extract
        ▼
ruff_spo_triplet::ModelGraph            (language-agnostic IR)
        │  ogar_from_ruff::lift_model_graph
        ▼
Vec<ogar_vocab::Class>                   (OGAR canonical vocab — the transpile)
        │  render (ogar-render-askama → Rust struct + methods)  [next]
        ▼
crates/osm-domain/                       (Rust domain types)
```

The node / way / relation / changeset / tag model is a graph, so the harvested
`Class` set drops directly onto `lance-graph`.

## Status

- ✅ **Harvest** — 50 classes lifted from real OSM source. Full IR in
  [`harvest/osm_ir.txt`](harvest/osm_ir.txt) (associations, mixins, STI parents).
- ✅ **Render** — all 50 `Class`es rendered to Rust in
  [`crates/osm-domain/src/generated/`](crates/osm-domain/src/generated/) via
  `ogar-render-askama` (`render_class_with_methods`): associations become typed
  edge fields (`belongs_to → Option<u64>`, `has_many → Vec<u64>`) + a `new(..)`
  constructor. **Compiles + tests green** (`cargo build` / `cargo test`).
  Regenerate: `cargo run -p ogar-render-askama --example render_osm -- <osm-root> <out>`.
- ⏳ **Next** — classid mint (OSM concepts into the codebook so `CLASS_ID`
  emits), `ActionDef` DO-arm (behaviour methods) once controller/model actions
  are harvested, and lance-graph wiring (node/way/relation as a graph).

## Provenance (regenerate deterministically)

| Input | Pin |
|---|---|
| OSM source (`openstreetmap/openstreetmap-website`) | `173885c17d91c4a2ceb70f7a4e911f2b250628ef` |
| ruff (`AdaWorldAPI/ruff`) | `61ce2b490fc3c432d36c44eceed08125f838b405` |
| OGAR (`AdaWorldAPI/OGAR`) | `4037e88` |

Harvest driver: `ogar-from-rails/examples/harvest_osm.rs` —
`cargo run -p ogar-from-rails --example harvest_osm -- <osm-website-root> --ir`.

## The 50 harvested classes

Core geodata: `Node` `Way` `Relation` `Changeset` + `*Tag` / `RelationMember` /
`WayNode` + the `Old*` versioned mirrors. Plus `User` (the 32-association hub),
`Note` / `Trace` / `DiaryEntry` / `Message` / `Issue` and their comment /
subscription satellites. See `harvest/osm_ir.txt` for the complete graph.
