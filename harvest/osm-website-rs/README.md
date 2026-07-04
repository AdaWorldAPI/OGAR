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
- ⏳ **Render** — `Vec<Class>` → Rust structs (+ classid mint, ClassView) via
  `ogar-render-askama`. `crates/osm-domain` is the landing crate.

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
