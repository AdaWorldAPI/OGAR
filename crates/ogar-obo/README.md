# ogar-obo — the OBO-core reference bake

Parses the OBO Foundry biological-ontology core into a **canonical 512-byte
SoA `NodeRow` buffer** the lance-graph loader reads **zero-copy**, plus the
OWL-EL completion subset the OBO EL profile actually exercises.

| namespace | role | concept id | domain |
|---|---|---|---|
| MONDO | disease | `0x0B01` | OBO clinical-reference |
| HPO | phenotype / symptom | `0x0B02` | OBO clinical-reference |
| Uberon | anatomy spine | `0x0A02` | Anatomy (public, sibling of FMA `0x0A01`) |
| PATO | quality | `0x0B03` | OBO clinical-reference |
| RO | relations | `0x0B04` | OBO clinical-reference |

Pure **public CC-BY / CC0** reference. **Zero PHI, zero consumer-private
codebook.** Wired exactly as `ogar-fma` / `ogar-cpic`.

## The loader connection is a byte-layout contract

`ogar-obo` emits the exact little-endian 512-byte geometry of
`lance_graph_contract::canonical_node::NodeRow` — `key(16) | edges(16) |
value(480)`. lance-graph's `node_rows_from_le_bytes` reads those bytes back as
`&[NodeRow]` with no deserialize. The CURIE numeric id (`MONDO:0007739` →
`7739`) is the node's 24-bit **identity**; the namespace is the **classid**
(canon-high `concept<<16 | app`). Compatibility is proven by the round-trip
test (`as_le_bytes` ↔ `rows_from_le_bytes`, 512×N, 64-align gate) — no
cross-repo compile coupling.

## What survives the parse (never truncated)

- **is_a / part_of** → the backbone rails + the EL subsumption/mereology spine.
- **has_phenotype / disease_has_location** → MONDO ↔ HP/Uberon convergence.
- **HP logical defs** (`hp-base.owl`) → HP `anatomy:quality` grounding
  (`has_anatomy` → Uberon, `has_quality` → PATO).
- **xrefs** (MeSH · UMLS · OMIM · Orphanet · SNOMED · ICD) → the projection-join
  / multilateration bearings **and** the guideline-spider path: a MeSH bearing
  resolves a disease/phenotype to its clinical guideline (online→local). These
  are load-bearing; the bake keeps every one.

## Build & run

```
cargo test -p ogar-obo                              # 8 tests incl. the loader round-trip
cargo run  -p ogar-obo --example bake_obo -- <dir>  # <dir> holds the 6 pinned sources
```

## Release layout

- **code + `manifest.json`** (source pins: PURL · version · sha256 · SPDX) → git.
- **`obo-core.soa`** (the 512×N derived artifact) → **release asset**, gitignored.
- OBO mandates open licenses, so the frozen source snapshots are re-hostable
  alongside the derived bake for full replicability (re-fetch → re-bake → sha).

## EL saturation (ELK subset, `reason.rs`)

`is_a` transitivity · transitive-role `part_of` · existential-through-spine
(R∃): an HP grounded to a Uberon site is grounded to that site's ancestors too
— the deductive form of "inherit the grounding up". Full OWL-EL classification
(disjointness / unsatisfiability) is the follow-up pass.
