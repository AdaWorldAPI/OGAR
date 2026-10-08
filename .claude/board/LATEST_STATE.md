# LATEST_STATE.md — current-state snapshot for OGAR

> **MUTABLE.** This is the one board file that gets overwritten in place
> rather than appended to — it answers "what exists right now", not "what
> happened". Refresh the tables below when a crate is added/removed, a
> classid domain is minted, or queued work lands/changes. It does **not**
> replace `docs/DISCOVERY-MAP.md` (the graded discovery ledger — *what was
> found*) or `docs/INTEGRATION-MAP.md` (the composition map — *how it
> composes*); those stay the deep, append-only record. This file is the
> five-second version: the crate inventory, the classid domain map, and
> what is actively queued. When in doubt about a claim here, the two docs
> above (or the source under `crates/`) are the authority — this is an
> index into them, not a replacement.
>
> Snapshot date: 2026-08-17. Refresh this line whenever the tables below
> are edited.

---

## 1. Crate inventory (workspace members, `Cargo.toml`)

32 crates. One line each, taken verbatim (trimmed) from each crate's own
`Cargo.toml` `description`.

| Crate | Role |
|---|---|
| `ogar-vocab` | Canonical IR types for the AR-shape vocabulary — the codebook, `Class`/`Association`/`Attribute`, `ConceptDomain`. |
| `ogar-ontology` | Prefix conventions + NiblePath-compatible identity routing. |
| `ogar-emitter` | `Triple` type + `OgarEmitter` trait — OGAR IR → graph triples. |
| `ogar-adapter` | Adapter trait + HHTL-style lookup tables for cross-language identity translation. |
| `ogar-proposal` | Owned mirror of lance-graph-ontology `MappingProposal` + Class→proposal producer mapping. |
| `ogar-adapter-surrealql` | **DEPRECATED** (2026-07-22, SoC ruling — `docs/DISCOVERY-MAP.md` D-SURREALQL-DEPRECATED). Historical bidirectional SurrealQL DDL bridge. |
| `ogar-adapter-ttl` | Bidirectional Turtle (RDF/OWL) bridge — `emit_ttl` / `parse_ttl` (oxttl). |
| `ogar-adapter-clickhouse-ddl` | Bidirectional ClickHouse DDL bridge. |
| `ogar-adapter-postgres-ddl` | PostgreSQL DDL adapter — the transactional System-of-Record emitter (writes/GoBD/ACID), plus the V3 facet-table emitter and a legacy-parity drift-fuse. |
| `ogar-knowable-from` | Producer seam for the `knowable_from` meet-point (`temporal::classify`, ADR-010). OGAR stays Lance-free. |
| `ogar-from-elixir` | SCAFFOLD — Elixir (Ecto/GenServer/gen_statem/Phoenix/Oban) frontend. |
| `ogar-from-ruff` | Lift `ruff_spo_triplet::Model` → `ogar_vocab::Class` — the producer-side seam for every `ruff_*_spo` frontend. |
| `ogar-from-rails` | Rails/ActiveRecord frontend, via `ruff_ruby_spo` + `ogar-from-ruff::lift_model_graph`. |
| `ogar-from-schema` | Schema-as-input producer family (OGIT TTL today; XSD/JSON-Schema/OpenAPI/Prisma queued) — the structural arm, paired with source-AST producers for the behavioral arm. |
| `ogar-action-handler` | OGAR-native HIRO `ActionHandler` runtime — `CapabilityExecutor` impls over the action-ws protocol core. |
| `ogar-class-view` | Bridge: `ogar_vocab::Class` → `lance_graph_contract::ClassView` (presence-bitmask + render-row resolver). |
| `ogar-render-askama` | Build-time askama codegen harness — one `ArtifactKind` enum, per-kind templates over `Class`. |
| `ogar-fma-skeleton` | FMA skeletal spine — the clamped convergence-anchor atlas (bones as immutable Morton-tile addresses). |
| `ogar-fma` | FMA anatomy structure resolver — name→structure reference beyond the bone skeleton (`0x0AXX`). |
| `ogar-obo` | OBO-core reference bake — MONDO/HPO/Uberon/PATO/RO → canon 512-byte SoA `NodeRow`; OWL-EL completion subset. Public CC-BY reference, never PHI. |
| `ogar-cpic` | CPIC pharmacogenomics reference (`0x0EXX`, Genetics domain) — public reference, never a patient genotype. |
| `ogar-adapter-python` | Generated Python module: classids, domain action tables, hot-plug resolution, V3 facet decoder — plug-and-play without a Rust/OGAR dependency. |
| `ogar-adapter-csharp` | Same as `ogar-adapter-python`, C# class library target. |
| `ogar-auth` | Reusable auth SDK — TOTP, Argon2id, legacy 3DES-EDE2/PBKDF1-MD5 transition, re-export of ndarray `encryption`. |
| `ogar-encryption` | Single generic classid-agnostic encryption surface (thin re-export of ndarray `encryption`). |
| `ogar-doc-ir` | Source-agnostic perceptual IR for the document layer — closed-vocabulary region tree (pixel retina + DOM retina both produce it). |
| `ogar-a2ui-frame` | Addressed-surface wire frames for a2ui screen addressing — LE-first, zero deps in the hot path. |
| `ogar-from-docv1` | Consumer-side transcode: tesseract-rs `doc.v1` JSON → `ogar-doc-ir`. |
| `ogar-render-typst` | Typst SOURCE emitter over `FieldView` rows — the paged/archival projection. |
| `ogar-loco` | Low-code program surface — vocabulary-agnostic call ABI, 512-byte node, `Vocabulary` trait. |
| `ogar-ro` | Relation Ontology (RO) predicates as a callable `ogar-loco` Vocabulary. Mints nothing in the shared codebook. |
| `ogar-elk` | EL subsumption closure as a POST-BAKE observer — `A ⊑ B` entailment + cycle-soundness check. Zero deps, no serialization. |
| `ogar-osm` | OpenStreetMap geodata reference surface (`0x0FXX`, Geo domain) — render half; concepts already minted in `ogar-vocab`. |

Refresh trigger: a member is added/removed from the workspace `[members]`
list in root `Cargo.toml`, or a crate's role materially changes (e.g. a
DEPRECATED marker lands, as it did for `ogar-adapter-surrealql`).

---

## 2. Classid domain occupancy (the high byte of a canonical `u16` id)

Authoritative source: `crates/ogar-vocab/src/lib.rs` `ConceptDomain` enum +
`canonical_concept_domain()`. This table is a five-second index into that
enum's doc comments — read the enum itself before minting into any domain,
it carries the provenance fences and PHI/public-reference distinctions this
table drops for brevity.

| High byte | Domain | Notes |
|---|---|---|
| `0x00` | Reserved | `0x0000` is `NodeGuid::CLASSID_DEFAULT`. |
| `0x01` | ProjectMgmt | OP ↔ Redmine. |
| `0x02` | Commerce | Billing/ERP, OSB ↔ Odoo. |
| `0x03` | Ontology | OBO biomedical reference (MONDO/HPO/Uberon/PATO/RO). Carries zero shared-vocab rows — concept ids live in `ogar-obo` itself, not the shared codebook. See `TECH_DEBT.md` for the collision history inside this domain. |
| `0x04`–`0x06` | **Unassigned** | `0x05` = Scope-kind, `0x06` = Concern-kind — RESERVED but mint-on-emit only (EPIPHANIES 2026-07-05 E-RECIPE-FAMILIES-MINT-ON-EMIT); no variant, no concept, no codebook row until the emit seam exists. |
| `0x07` | Osint | Open-source intelligence. Zero shared-vocab rows (reserved posture). |
| `0x08` | Ocr | Optical character recognition / document extraction. |
| `0x09` | Health | Clinical/patient/care (PHI). |
| `0x0A` | Anatomy | FMA reference ontology — public structure, distinct from `Health` (a finding *about* the structure is PHI, the structure itself is not). |
| `0x0B` | Auth | IAM, provider-agnostic — AuthStore class family (`auth_store` + per-IdP profiles). See `docs/CLASSID-RBAC-KEYSTONE-SPEC.md` §7. |
| `0x0C` | Automation | HIRO IT-automation — MARS CMDB + Automation actuators (THINK+DO meet here). |
| `0x0D` | HR | Employment/org/contracts, public master-data. |
| `0x0E` | Genetics | CPIC pharmacogenomics, consumed by q2. Zero shared-vocab rows (reserved posture); V3 marker form `0x0E01_1000`. |
| `0x0F` | Geo | OpenStreetMap geodata reference. |
| `0x10`–`0x16` | **Unassigned** | No domain minted. |
| `0x17` | Blocks | Visual block-programming opcode vocabulary. Zero shared-vocab rows (reserved posture, operator ruling 2026-08-04). Provenance-fenced: permissively-licensed sources only. |
| `0x18`+ | **Unassigned** | No domain minted. |

Refresh trigger: a new `ConceptDomain` variant lands, or a reserved-but-
unminted domain (`0x04`–`0x06`, `0x10`–`0x16`, `0x18`+) gets its first
concept.

---

## 3. Actively queued / gated (snapshot, not authoritative — see the cited source for the live state)

- **`PROBE-OGAR-RBAC-AUTHORIZE`** — the `authorize()` enforcement path
  (`ClassRbac` trait + bit-for-bit decision) is gated on this probe running
  green against a reference (Odoo `ir.model.access ∧ ir.rule`, Redmine
  `User#allowed_to?`, or an OpenFGA model). Interim shipped: `MembraneGate` /
  `SmbMembraneGate` pattern. See `ISSUES.md` ISS-RBAC-AUTHORIZE-BY-CLASSID.
- **`ogar-obo` numeric-`0` pad collision (gate A1)** — the `+1` numeric bias
  fix landed in `crates/ogar-obo/src/edges.rs` (2026-08-08), but
  `META_STUDY_SPINE` (`crates/ogar-obo/src/registry.rs`) is still
  example-only (`examples/bake_spine.rs`, `examples/probe_spine.rs`), not
  wired into the production `bake()` path. Re-verify against
  `registry.rs`'s own module doc before treating this as resolved.
- **`docs/V3-TRANSPILER-ADR.md`** — RFC, not yet promoted to adopted (per
  `CLAUDE.md` P0's "Paired follow-ups (mandatory)" list). `ogar-fma-skeleton::Guid`
  classid width reconciliation (2→4, F-2) and `docs/NODEGUID-CANON-AUDIT.md`
  F-3 inversion are the other two items in that same follow-up set.
- **`.claude/PLAN.md`** — the sprint-by-sprint roadmap is v0-era (predates
  the V3 facet flip, the OBO bake, and most of `docs/DISCOVERY-MAP.md`'s
  discovery ledger) and has not been refreshed to reflect current
  architecture; treat it as historical, not a live work queue. Its own
  Sprint 7 entry already documents this pattern (tombstoned in place per
  the append-only-at-sprint-level convention) — the roadmap as a whole is
  due the same treatment but hasn't received it.
- **`ogar-from-schema`** — XSD/JSON-Schema/OpenAPI/Prisma frontends are
  named as queued in the crate's own description; only the OGIT-TTL
  frontend ships today.

Refresh trigger: any of the above resolves, or a new gated/queued item is
identified — add it here (or, if it needs a decision, file it as an
`ISSUES.md` entry and reference the id here instead of duplicating the
narrative).
