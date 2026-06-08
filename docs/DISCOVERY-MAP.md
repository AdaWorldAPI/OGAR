# Discovery Map — the shape‑preserving index

> **Purpose.** A single append‑only index of every discovery, idea, and
> decision in the substrate arc — each with its **grade**, **materialization
> status**, **home**, and **dependencies** — so the *shape* (the connections
> between ideas) survives when ideas materialize into individual ADRs and
> crates.
>
> **The anti‑dilution thesis.** Shape dilutes not from too few documents but
> from **fragmentation**: each idea pinned in isolation loses its links, and
> a future reader sees N disconnected ADRs instead of one doctrine. This map
> is the one place the **topology** lives. It is **terse by discipline** —
> pointers + structure + status, never re‑explanation. The ADRs / crates /
> docs are the *content*; this is the *index*. It is **append‑only +
> status‑tracked**, mirroring the substrate's own versioned‑audit shape
> (ADR‑008/013) — fitting, since a map of an append‑only audited substrate
> should itself be append‑only audited.
>
> **How to read an entry:** `[ID] one‑line shape — grade — status — home —
> depends`.
> - **Grade:** `[G]` Grounded (real artifact/spec) · `[H]` Hypothesis
>   (sound, one side unconfirmed) · `[S]` Speculative (catalog only).
> - **Status (materialization pipeline):** `IDEA` → `EPIPHANY` (graded,
>   captured in a synergy/epiphany doc) → `ADR` (pinned contract) → `CODED`
>   (crate + tests + CI).
> - **`[per runtime session]`** = runtime‑owned; OGAR cannot verify.
>
> **Status: LIVING INDEX** (2026‑06‑08). Update entry *status* on
> materialization; never delete (append‑only). One entry per discovery;
> cross‑link, do not duplicate.

---

## 1. The doctrine spine — the load‑bearing floors

The ADR canon is the spine; every discovery in §2 hangs off the floor it
specializes.

| ADR | Floor | Status | Home |
|---|---|---|---|
| **022** | **boundary** — no serialization in the hot path | `ADR` | `THE-FIREWALL.md` |
| **023** | **IR** — `Class` is wire truth; dialects absorbed by adapters | `ADR` | `ARCH-DECISIONS` + `ogar-vocab/` |
| **024** | **codec** — palette256 + HHTL; 1‑byte index, sub‑µs decode, ρ ≥ 0.99 | `ADR` | `ARCH-DECISIONS` |
| **025** | **selection** — probe‑free closed‑form level (`r* = ⌈log₄(C/τ)⌉`) | `ADR` | `ARCH-DECISIONS` |
| **026** | **(pending)** cascade + amortization gate + meta→content gradient + no‑collapse precondition + storage | `EPIPHANY → ADR` | `CASCADE-SYNERGIES-EPIPHANY.md` |

**The gestalt the spine forms (the one sentence the individual ADRs lose):**

> *The substrate is an **immaterialized Morton cascade with templated
> payloads** — address is geometry (helix + HHTL), error is closed‑form
> (jc), the codec is exact in the θ‑window, materialization is lazy and
> columnar; the **only forbidden cost is non‑amortized per‑query.***

ADR‑022 (the boundary) + ADR‑025 (the probe) are both special cases of that
last clause. ADR‑023 (address vs payload) + ADR‑024 (the codec) describe the
two halves of a cell. ADR‑026 names the cascade that ties them.

---

## 2. The discovery ledger

### 2.1 Addressing & cascade

| ID | Shape | Grade | Status | Home | Deps |
|---|---|:--:|:--:|---|---|
| D‑MORTON | NiblePath nibble = one 4×4 Morton tile (2 x‑bits + 2 y‑bits interleaved) | G | EPIPHANY | SYN §0,§7.5 | 023,024 |
| D‑XOR2 | one address‑XOR, two reductions: **popcount = perms** (024) / **CLZ = containment** (025) | G | EPIPHANY | SYN §0 | 024,025 |
| D‑CASCADE | 64→256→1024→4096→16k→64k→256k = immaterialized Morton enumeration; every level = +1 nibble | G | EPIPHANY | SYN §7.5 | D‑MORTON |
| D‑IMMAT | the cascade is a **coordinate transform, not a stored grid** (`(lat,lon)→quadkey` cheap) | G | EPIPHANY | SYN §7.5 | D‑CASCADE |
| D‑NEIGH | neighbor‑XOR walk + parent‑prefix = structured‑sparse stencil (block‑banded, not sparse GEMM) | H | EPIPHANY | SYN §6 | D‑MORTON, `[per rt]` blasgraph |

### 2.2 Selection & bounds

| ID | Shape | Grade | Status | Home | Deps |
|---|---|:--:|:--:|---|---|
| D‑PROBEFREE | hot path takes zero data‑dependent branches; level is closed‑form | G | ADR | ADR‑025 | 022 |
| D‑RSTAR | `r* = ⌈2·log_b(C/τ)⌉`; `b=16` ⟹ `⌈log₄(C/τ)⌉`; inclusive `≤ τ` | G | ADR | ADR‑025 (#45,#46) | D‑CASCADE |
| D‑BOUNDS | tile `bounding_volume` is **derived** (address arithmetic), not stored | H | ADR | ADR‑025 | D‑MORTON, `[per rt]` helix |
| D‑CESIUM‑PROBE | native Cesium SSE refine/collapse = the trial‑and‑error probe 025 removes | G | EPIPHANY | SYN §1 | D‑PROBEFREE |

### 2.3 Codec & codebooks

| ID | Shape | Grade | Status | Home | Deps |
|---|---|:--:|:--:|---|---|
| D‑PAL256 | palette256 = one PQ subspace's 256 centroids (CAM = 6×256, PR #477) | G | ADR | ADR‑024 | — |
| D‑CAM | CAM‑PQ = 6 roles × 256 = 48‑bit fingerprint per word (`cam_codes.bin`) | G | EPIPHANY | SYN §3 | D‑PAL256 |
| D‑CARD | cascade per‑axis cardinality ↔ codebook size: 256↔palette, 4096↔`MAX_VOCAB` | G | EPIPHANY | SYN §3,§7 | D‑CASCADE, D‑CAM |
| D‑META64 | `CausalEdge64` = meta: `2⁶` role‑mask **+** 48‑bit CAM + 16‑bit headroom, one 64‑bit word | H | EPIPHANY | SYN §9.6 | D‑CAM, `[per rt]` CausalEdge64 |
| D‑BITGATE | 48→64 headroom: store the seed **iff** irreducible‑beyond‑the‑address (else compute) | H | EPIPHANY | SYN §9.6 | D‑META64, D‑AMORT |
| D‑RHO | ρ = 0.9973 vs cosine (arm‑discovery aerial codebook) — the empirical anchor | G | ADR | ADR‑024 | D‑PAL256 |

### 2.4 No‑collapse precondition (θ + golden)

| ID | Shape | Grade | Status | Home | Deps |
|---|---|:--:|:--:|---|---|
| D‑THETA | θ ∈ [1.45,1.6] (near‑orthogonal, cosθ∈[−0.03,0.12]) = the conditioning window | H | EPIPHANY | SYN §2; ADR‑025 ref | D‑PAL256, `[per rt]` jc |
| D‑RHOENV | ρ ∈ [0.93 (band edge) .. 0.9973 (θ≈π/2)] = exactness envelope over θ | H | EPIPHANY | SYN §2 | D‑THETA, D‑RHO |
| D‑GOLDEN | golden‑stride helix placement controls θ into the window **by construction** | H | EPIPHANY | SYN §2 | `[per rt]` helix |
| D‑MOIRE | golden irrationality = X‑Trans‑grade anti‑moiré interlace ("x256 that can't collapse") | H | EPIPHANY | SYN §2 | D‑GOLDEN |
| D‑NOCOLLAPSE | unify: near‑orthogonal **codebook** + aperiodic **lattice** = no degenerate beat in value‑ or position‑space | H | EPIPHANY → ADR‑026 §2 | SYN §9.1 | D‑THETA, D‑MOIRE |

### 2.5 Amortization gate

| ID | Shape | Grade | Status | Home | Deps |
|---|---|:--:|:--:|---|---|
| D‑AMORT | admission test = **amortize‑or‑don't‑spend** (not "free"); unifies 022 + 025 | G | EPIPHANY → ADR‑026 §3 | SYN §7 | 022,025 |
| D‑AMORT‑AXES | two reuse axes: **mipmap cascade** (spatial) / **Semantik volumetric centroid** (semantic) | G | EPIPHANY | SYN §7 | D‑AMORT |
| D‑BOTHCASC | store **both** freq + semantic cascades (SoA headroom affords it); "pick one axis" fork dissolved | G | EPIPHANY | SYN §7 | D‑AMORT‑AXES |
| D‑FRACTAL | the gate is fractal: **storage** (§7.5) / **cascade** (§7) / **bit** (§9.6) — one rule, three scales | G | EPIPHANY | SYN §7 | D‑AMORT, D‑BITGATE |
| D‑MAXAMORT | enrich the **coarse/meta** levels lavishly (amortize over everything below); be frugal only at the per‑query leaf | H | EPIPHANY | SYN §9 | D‑AMORT, D‑META64 |

### 2.6 Storage

| ID | Shape | Grade | Status | Home | Deps |
|---|---|:--:|:--:|---|---|
| D‑COLUMNAR | Morton‑keyed columnar rows + prefix pushdown = tile fetch (Delta `ZORDER`/Iceberg/BigQuery — deployed) | G | EPIPHANY | SYN §7.5 | D‑IMMAT |
| D‑LANCE | "parquet‑shaped" = the columnar family; **Lance** is the instance (random tile access > Parquet row‑group scan) | G | EPIPHANY | SYN §7.5 | D‑COLUMNAR |
| D‑PYRAMID | the unifying shape: columnar (parquet) + Morton rows (grid) + level cascade (pyramid) + closed‑form cell (shader) | H | EPIPHANY | SYN §7.5 | D‑COLUMNAR, D‑CASCADE |
| D‑DELTA | delta frames = version‑diff = changed Morton cells = codec P‑frame (I/P map; B does not, append‑only log) | H | EPIPHANY | SYN §7.5,§1 | D‑LANCE |

### 2.7 Cross‑domain synergy catalog

| ID | Shape | Grade | Status | Home | Deps |
|---|---|:--:|:--:|---|---|
| D‑CTU | Morton cascade = x265/x266 CTU quadtree; codec RDO split = the probe | G | EPIPHANY | SYN §1 | D‑CASCADE, D‑CESIUM‑PROBE |
| D‑ATTN | attention (bgz‑tensor WeightPalette 256) ranks tiles → drives `r*`; τ = min(certificate, attention) | H | EPIPHANY | SYN §4 | D‑RSTAR, D‑PAL256 |
| D‑CONVERGE | 6 lineages (codecs/sensors/displays/attention/PQ/Cesium) → quadtree + 256‑palette + irrational | G | EPIPHANY | SYN §0 | — |
| D‑OLED | OLED exciton ↔ substrate = weakest leg; only candidate diffusion‑length ↔ neighborhood; **do not build on** | S | EPIPHANY | SYN §3 | — |

### 2.8 IR & adapters — the coded layer

| ID | Shape | Grade | Status | Home | Deps |
|---|---|:--:|:--:|---|---|
| D‑VOCAB | `Class`/`Attribute`/`Association`/`EnumDecl`/`ActionDef`/`KausalSpec`/`Identity` IR | G | CODED | `ogar-vocab/` | 023 |
| D‑IDENT | `class_identity(prefix,name)` = NiblePath; canonical‑form invariant | G | CODED | `ogar-ontology/` (#31) | D‑VOCAB |
| D‑EMIT | `TripleEmitter` — 129 RDF predicates; SPO + TeKaMoLo | G | CODED | `ogar-emitter/` | D‑VOCAB |
| D‑SURREALQL | SurrealQL DDL adapter, parse+emit, round‑trip, identifier‑quoted | G | CODED | `ogar-adapter-surrealql/` (#32,#36) | D‑VOCAB |
| D‑TTL | Turtle (RDF/OWL) adapter, parse+emit, round‑trip | G | CODED | `ogar-adapter-ttl/` (#37) | D‑EMIT |
| D‑CHDDL | ClickHouse DDL adapter, parse+emit, dotted‑name round‑trip | G | CODED | `ogar-adapter-clickhouse-ddl/` (#38,#40) | D‑VOCAB |
| D‑KNOWABLE | `KnowableFromStore` + `register_class_knowable_from`; `surrealql-hint`; **`vart-backend`** | G | CODED | `ogar-knowable-from/` (#25,#33,#43) | D‑IDENT |
| D‑HINT | `schema_ddl_hint` loop closed — self‑describing registry via emit | G | CODED | (#33) | D‑SURREALQL, D‑KNOWABLE |
| D‑ELIXIR | Elixir/HIRO SchemaSource scaffold (`gen_statem`→Rubicon) | G | CODED (scaffold) | `ogar-from-elixir/` | D‑VOCAB |
| D‑OSM | `ogar-from-osm-pbf` — Node/Way/Relation; quadkey NiblePath from resolved geometry | H | IDEA | (queued) | D‑VOCAB, `[per rt]` D‑OSM‑3 |
| D‑PATTERN | `ogar-pattern` — recognition library + confidence (FMA‑D/FIBO/SKR/PROV‑O) | H | IDEA | (queued) | D‑TTL |
| D‑ACTION | `ogar-actionable` — lifecycle → `ActionDef`/`KausalSpec` | H | IDEA | (queued) | D‑PATTERN |
| D‑NSM | 4096‑dim Deep‑NSM encoder (Wierzbicka primes, `NUM_PRIMES=63`) calibration | H | IDEA | RDF‑OWL §4.10 | D‑CAM, `[per rt]` |

### 2.9 Domain instances (universality witnesses)

| ID | Shape | Grade | Status | Home |
|---|---|:--:|:--:|---|
| D‑DOM | 6 instances: chess / OpenProject / Elixir‑HIRO / Odoo‑ERP / HIPAA / OSM | G | ADR/doc | `DOMAIN-INSTANCES.md` (#27,#41,#42) |
| D‑PII | label‑free contract IS the PII guarantee (HHTL leaf‑rename at Adapter) | G | doc | `HEALTHCARE-TRANSCODING.md §4` |
| D‑LITMUS‑FMA | FMA bones‑rendering = compile‑time HHTL litmus (~75K static classes) | H | doc | `RDF-OWL-ALIGNMENT.md §6` |
| D‑LITMUS‑GEO | OSM = geographic litmus; "Femur is_a LongBone AND Marienplatz is_in Munich" sub‑µs | H | doc | `DOMAIN-INSTANCES.md §2.6` |

---

## 3. The materialization pipeline — what's ready, what's blocked

```
IDEA ──→ EPIPHANY ──→ ADR ──→ CODED
(floated)  (graded)   (pinned) (crate+CI)
```

**Ready to materialize now** (`G` + `EPIPHANY`, no runtime gate) → **draft
ADR‑026**:
- D‑AMORT, D‑AMORT‑AXES, D‑BOTHCASC, D‑FRACTAL (the amortization gate)
- D‑MORTON, D‑XOR2, D‑CASCADE, D‑IMMAT (the addressing)
- D‑COLUMNAR, D‑LANCE (the storage layer — almost entirely `[G]`)
- D‑CTU, D‑CONVERGE (the grounded synergies)

**Blocked on runtime confirmation** (`H` + `[per runtime session]`) → ADR‑026
lands these with `[per rt]` marks, receipts fill later:
- D‑GOLDEN, D‑MOIRE, D‑THETA, D‑RHOENV (helix spacing + θ/ρ envelope — `jc`/`helix`)
- D‑BOUNDS, D‑META64, D‑BITGATE (helix bounds + `CausalEdge64` layout)
- D‑NEIGH (`blasgraph` scope)

**Coded, done** — §2.8 D‑VOCAB…D‑KNOWABLE (+ vart‑backend), the adapters.

**Queued code** (`IDEA`, gated): D‑OSM (runtime D‑OSM‑3), D‑PATTERN,
D‑ACTION, D‑NSM.

**Do‑not‑build** (`S`): D‑OLED — catalog only.

---

## 4. The OGAR / runtime boundary

OGAR owns + verifies the **left**; the **right** is `[per runtime session]`
(cited as authoritative under the same discipline as ADR‑024's ρ anchor).

| OGAR‑owned (verified) | Runtime‑owned `[per rt]` |
|---|---|
| `ogar-vocab` IR; all adapters; `ogar-knowable-from` + vart‑backend | `crates/helix` (placement template, golden stride) |
| Morton/XOR/CLZ address algebra (D‑MORTON, D‑XOR2) | `crates/jc` (`weyl` + `jirak`; θ/ρ envelope) |
| the amortization gate (D‑AMORT, fractal) | `cognitive-shader-driver` (the consumer) |
| the storage framing (D‑COLUMNAR, D‑LANCE) | `blasgraph` scope (D‑NEIGH) |
| ADR‑022/023/024/025 + the ADR‑026 spine | `CausalEdge64` layout (D‑META64); SoA headroom budget |

**The five open ADR‑026 confirmations** (`CASCADE-SYNERGIES-EPIPHANY.md
§10`): helix spacing · θ‑window/ρ envelope · `blasgraph` scope ·
shader tile‑contract · SoA headroom budget. (`CausalEdge64` cardinality
+ the nesting‑axis fork are **closed** — operator‑resolved.)

---

## 5. The shape graph — the topology individual ADRs lose

```
                         ADR-022 boundary
                              │ (special case of)
        ADR-023 IR ───────────┼─────────── ADR-024 codec
        (address)             │             (palette256/CAM)
            │                 │                  │
        D-MORTON          D-AMORT            D-PAL256 ── D-RHO
        D-XOR2 ───┐     (amortize gate)         │         (ρ=0.9973)
            │     │           │ (unifies        │
        D-CASCADE │      022+025)            D-THETA ── D-RHOENV
            │     │           │                  │  (no-collapse)
        D-IMMAT   │      D-FRACTAL            D-GOLDEN ─ D-MOIRE
            │     │     (storage/cascade/bit)     │   (X-Trans/φ)
        D-COLUMNAR│           │              D-NOCOLLAPSE
        D-LANCE   │           │                  │
            └─────┴───── ADR-025 selection ──────┘
                         (probe-free r*)
                              │
                         ADR-026 (pending)
                = cascade + gate + meta→content + no-collapse + storage
                              │
              ┌───────────────┼───────────────┐
          D-CTU            D-ATTN          D-DELTA
        (x265/x266)     (attn-driven LOD) (version=P-frame)
```

The arrows are the links that vanish when you read ADR‑022..025 in
isolation. The map's job is to keep them visible.

---

## 6. Maintenance discipline

1. **Append‑only.** New discovery → new `D‑*` entry. Never delete; on
   materialization, advance the *status* column (IDEA→EPIPHANY→ADR→CODED).
2. **Terse.** One line per entry. The shape, not the explanation. Link to
   the home doc; do not duplicate its content here.
3. **Grade honestly.** `[S]` stays `[S]` until a measurement promotes it.
   `[per runtime session]` until the runtime session confirms.
4. **This is the index.** The doctrine (ADRs), the synergies
   (`CASCADE-SYNERGIES-EPIPHANY.md`), the IR (crates) are the content. The
   map points; it does not re‑derive.
5. **The map mirrors the substrate.** Append‑only + status‑versioned, like
   the audit log it documents (ADR‑008/013). If that feels recursive, it
   is — and that's the tell that the shape is intact.

---

## 7. Cross‑references

- `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` — ADR‑022..025 (+ 026 pending).
- `docs/CASCADE-SYNERGIES-EPIPHANY.md` — the synergy catalog + amortization
  gate + storage synthesis (the source of most §2 `EPIPHANY` entries).
- `docs/RDF-OWL-ALIGNMENT.md` — the brutal‑upgrade sequencing (§10 phases)
  + Deep‑NSM (§4.10).
- `docs/DOMAIN-INSTANCES.md` — the 6 universality witnesses.
- `docs/THE-FIREWALL.md`, `HEALTHCARE-TRANSCODING.md` — ADR‑022 + PII floor.
- lance‑graph PR #470/#473/#474/#475/#476/#477/#478, bardioc #17/#18/#19 —
  the runtime‑side receipts (`[per runtime session]` sources).
