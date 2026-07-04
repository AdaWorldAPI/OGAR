# OSM transpile — status of the three autoattended next-steps (2026-07-04)

## ✅ 1. classid mint — DONE (OGAR side)

Allocated **`ConceptDomain::Geo` (`0x0FXX`)** in `ogar-vocab` and minted the 10
OSM geodata concepts: `osm_node` `0x0F01`, `osm_way` `0x0F02`, `osm_relation`
`0x0F03`, `osm_changeset` `0x0F04`, `osm_element_tag` `0x0F05`,
`osm_relation_member` `0x0F06`, `osm_way_node` `0x0F07`, `osm_note` `0x0F08`,
`osm_gpx_trace` `0x0F09`, `osm_user` `0x0F0A`.

Full codebook contract satisfied (all `ogar-vocab` tests green, 96):
`CODEBOOK` + `class_ids::{consts,ALL}` + `all_promoted_classes()` constructors +
`ConceptDomain::Geo` + `canonical_concept_domain(0x0F)` + `COUNT_FUSE` pin
`68→78`.

The render now emits `CLASS_ID` for the 20 grounded geodata files
(`Node → 0x0F01`, …) via the Rails-name→concept grounding map in
`render_osm.rs`.

### ⏳ Sequenced follow-up (post-merge, NOT doable now)

The **lance-graph mirror** (`lance-graph-contract::ogar_codebook::CODEBOOK`) +
`lance-graph-ogar::parity::COUNT_FUSE` must be bumped `68→78` **after** this
OGAR `ogar-vocab` change merges to OGAR `main` — because `lance-graph-ogar`'s
`COUNT_FUSE` is a compile-time assert `contract::CODEBOOK.len() ==
ogar_vocab::class_ids::ALL.len()`, and lance-graph pulls `ogar_vocab` via **git
main** (still 68). Bumping the mirror first would break that assert against the
unmerged ogar-vocab. This is the E-CODEBOOK-MINT-IS-A-CROSS-REPO-ARC lockstep.

**Operator:** confirm the `0x0F` / Geo domain allocation before merge.

## ✅ 2. lance-graph wiring — DONE (manifest)

`harvest/osm_graph.spo` — the OSM association graph as **154 classid-keyed SPO
edges**. node/way/relation is a graph; each grounded subject carries its
`CLASS_ID` and every association is an edge to a target concept (classid where
the target grounds), e.g.:

```
osm_node[0x0F01] --BelongsTo:changeset--> changeset[0x0F04]
osm_node[0x0F01] --HasMany:element_tags--> NodeTag[0x0F05]
osm_node[0x0F01] --HasMany:containing_relations--> Relation[0x0F03]
osm_changeset[0x0F04] --BelongsTo:user--> user[0x0F0A]
```

Targets shown `[----]` are ungrounded (plural relation names that don't
normalise to a minted concept, or non-geodata app entities) — an honest gap in
the simple grounding, not a lift error. This manifest is the feed a
`lance-graph` graph loader consumes.

## ⛔ 3. DO-arm methods — CAPABILITY GAP (honestly scoped, not faked)

The render *supports* behaviour methods — `render_class_with_methods(class,
mask, actions: &[ActionDef])` lifts each `ActionDef` into a struct method. But
**there are no `ActionDef`s to pass**:

- `ruff_ruby_spo::extract` walks **`app/models` only**; `app/controllers` (where
  Rails request behaviour lives) is never parsed.
- The `Class` IR carries `associations` / `enums` / `mixins` / `attributes` —
  **no methods / callbacks / actions field**.

So the DO-arm is empty by construction today. Closing it needs a **new
capability**: a Ruby controller/action harvester (`ruff_ruby_spo` extended to
`app/controllers`, or a `ruff_ruby_action_spo`) that lifts controller actions →
`ActionDef { predicate, object_class, body_source, on_enter }`. That is a ruff
feature, filed as the honest next brick — not something to synthesise from the
model tree.
