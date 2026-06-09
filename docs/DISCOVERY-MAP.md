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
> - **`SYN §…` = `docs/CASCADE-SYNERGIES-EPIPHANY.md` §…** — the synergy /
>   epiphany source doc that holds most `EPIPHANY`‑status entries. **It is
>   introduced in OGAR PR #47 and co‑merges with this map (PR #48); the
>   `SYN` links resolve once both land.** Until then a reviewer auditing
>   from `main` alone will not find it — that is expected (cross‑PR
>   dependency), not a dangling link. (Flagged by Codex on PR #48.)
>
> **Status: LIVING INDEX** (2026‑06‑08). Update entry *status* on
> materialization; never delete (append‑only). One entry per discovery;
> cross‑link, do not duplicate.

---

## 0. The endgame — what the whole substrate is *for*

> **Operator, 2026‑06‑08:** *"endgame is actually getting DeepNSM + AriGraph
> as a meta vs episodic basins and supporters, meaning vectors, and then
> later extraction of AST‑shaped named facts — **do vs think**."*

Everything in §1 (the doctrine) and §2 (the discoveries) is the **foundation
for a cognitive memory architecture.** The teleology, named — and it
reprioritizes the queue.

**THINK — the DeepNSM + AriGraph memory** (the substrate's *world model*):

| AriGraph (arXiv 2407.04363, Anokhin et al. 2024 — semantic + episodic KG world‑model with associative retrieval, for LLM‑agent planning) | Substrate realization | Grade |
|---|---|---|
| **semantic memory** (the world model) | **META = `MetaWord`** (the §4.1 layer — structural awareness) | mapping H |
| **episodic memory** (event sequence) | **EPISODIC = delta frames** = `DatasetVersion(v)→(v+1)` + per‑row `cycle` stamp (D‑DELTA) | H |
| **associative retrieval of interconnected concepts** | **basins + supporters**: basins = palette256/CAM centroids + Semantik volumetric centroids + Morton‑prefix subtrees (D‑PAL256, D‑CAM, D‑AMORT‑AXES; the intra‑basin locality probe); supporters = the neighbor‑XOR retrieval (D‑NEIGH) | H |
| **the encoded concepts** | **meaning vectors** = DeepNSM 4096‑dim (63 primes) → CAM‑PQ 6×256 (D‑NSM, D‑CAM) — *shipping* in lance‑graph `nsm/` | G (DeepNSM), G (AriGraph public) |

**DO — the extracted AST‑shaped named facts** (the substrate's *actions*):
*"later extraction of AST‑shaped named facts"* = the OGAR IR (`Class` /
`ActionDef` / `Association` — D‑VOCAB), **extracted from the meaning‑vector
memory** by the recognition + lifecycle pipeline (D‑PATTERN, D‑ACTION). The
named fact is the wire (ADR‑023): the point where THINK (vectors) becomes
DO (action).

**The master axis — DO vs THINK:**

| | THINK | DO |
|---|---|---|
| substrate | DeepNSM + AriGraph meaning‑vector memory (meta / episodic / basins / supporters) | `ActionDef` / Rubicon lifecycle / Kanban 6‑phase |
| OGAR IR arm | structural (`Class`/`Association`) | behavioral (`ActionDef`/`ActionInvocation`) |
| trichotomy | **Semantik** (what is known) | **Pragmatik** (what is done) |
| membrane | — | **AST‑named‑fact extraction is the membrane between them** |

**What this reprioritizes.** `D‑NSM`, `D‑PATTERN`, `D‑ACTION` (listed
`IDEA`/queued in §2.8) are **not optional extras — they are the
endgame‑critical path** (the *think → do* extraction). The
addressing / codec / storage floors (§2.1–§2.6, §1) are the **foundation
they stand on**, not the destination. The destination is: *the substrate
thinks in meaning‑vector basins and acts via extracted AST‑named‑facts.*

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
| D‑META64 | meta = `2⁶` role‑mask + 48‑bit CAM + 16‑bit headroom. **⚠ PR #477 revises:** `MailboxSoA` has **separate** `edges [CausalEdge64;N]` *and* `meta [MetaWord;N]` columns → **the meta is `MetaWord`, not `CausalEdge64`** (the edge encoding). Reconcile (§4.1). | H → REVISE | SYN §9.6 + §4.1 | D‑CAM, `[per rt]` |
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
| D‑MANTISSA | a cell address **is a float**: `(exponent = Morton level, mantissa = golden sub‑placement)`; the mantissa is **irrational** (φ recurrence) + **implied** (generated from the address, not stored — the hidden‑bit spirit). X‑Trans "in spirit" = an *implied irrational mantissa*; the **anti‑moiré** function stays `[H]` (§2 caveat). Unifies D‑CASCADE (exponent) + D‑GOLDEN (mantissa) + D‑BITGATE (implied) + D‑NOCOLLAPSE | H | EPIPHANY | SYN §2 | D‑CASCADE, D‑GOLDEN, D‑BITGATE |

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

## 4.1 Runtime synergy receipts — lance‑graph PR #477 / #478

Cross‑reference of the runtime‑shipped contract against today's findings.
Several findings are now **confirmed by runtime code** (promotes their
grade/status); one is **revised**; one gradient is **convergent‑but‑unwired
on both sides** (the next materialization).

### Confirmations — runtime code grounds the OGAR finding  [G]

| Finding | Runtime receipt (PR #477) | Verdict |
|---|---|---|
| **D‑MORTON, D‑CASCADE, D‑XOR2 (containment)** | `lance-graph-contract/src/hhtl.rs`: `NiblePath { path:u64, depth:u8 }`, `FAN_OUT=16`, `MAX_DEPTH=16` (16 nibbles = 64 bits), `parent()=path>>4`, `child(n)=(path<<4)\|n`, `is_ancestor_of` = a single prefix‑shift kernel, `common_ancestor` = LCA. (q4‑hhtl‑audit) | **the runtime SHIPS the nibble address algebra** — these are CODED runtime‑side, not just EPIPHANY |
| **D‑IMMAT, D‑COLUMNAR, ADR‑022** | `soa-three-tier-model.md` invariant: *"zero‑copy from creation to Lance tombstone … no bytes leave the backing store … even then the in‑memory store is unchanged, not serialized and freed."* `MailboxSoA::emit()` + `CollapseGateEmission` **scheduled for removal** | the "immaterialized, never serialized" finding IS the runtime's ONE invariant; the serialization path is being **deleted** |
| **D‑COLUMNAR, D‑LANCE (column projection)** | `soa_envelope.rs`: `SoaEnvelope` trait — `columns()->&[ColumnDescriptor]`, `ColumnDescriptor{name_id,kind,elems_per_row,row_offset}` (`repr(C)`), zero‑copy `row_le`/`column_le` views, `as_le_bytes()`, `verify_layout()` gate | the "Morton‑keyed columnar + projection" framing is the `SoaEnvelope` ABI, line‑for‑line |
| **D‑DELTA (delta frames = version‑diff)** | `cycle()->u32` stamp; *"a Lance version IS a coherent LE envelope"* (soa_envelope L16); `last_active_cycle [u32;N]` = per‑row same‑cycle write guard; `DatasetVersion(v)→(v+1)` | a Lance version = a frame; the per‑row cycle stamp = the changed‑cell delta → **D‑DELTA promotes [H]→[G]** |
| **D‑AMORT (amortize‑or‑don't‑spend)** | PR #478 mechanical test: *never‑mutated → const codebook, **keep**; mutated‑after‑init → SoA snapshot, **nudge**.* Read‑only codebooks (role keys, UDFs, `simd_caps`) stay const | the gate's storage instance, **operationalized** — const codebooks = the amortized build‑once structures; SoA = per‑cycle mutable |

### Revision — runtime evidence corrects a finding  ⚠

- **D‑META64.** `MailboxSoA` columns include **both** `edges [CausalEdge64;N]`
  **and** `meta [MetaWord;N]` as *separate* columns. So **the meta layer is a
  dedicated `MetaWord`, not `CausalEdge64`** (which is the causal‑edge
  encoding). The operator's "CausalEdge64 = meta" is contradicted by the
  shipped column layout. **Reconcile:** the `2⁶`/48/16 bit‑budget reasoning
  (D‑BITGATE) most likely applies to `MetaWord`; `CausalEdge64` is the edge.
  `[per runtime session]` on the `MetaWord` layout + whether `CausalEdge64`
  feeds it.

### Convergent but unwired — both sides describe it; neither built it  [the next step]

- **D‑MAXAMORT / D‑CARD / D‑PAL256** (the meta→content→spatial gradient) ↔
  `high_heel.rs` module‑doc legend **"HHTL cascade mapping: HEEL=scent /
  HIP=palette / TWIG=SpoBase17 / LEAF=full planes."** The q4 audit's verdict:
  *"this is a layout legend in a comment. **No code routes by prefix.**"* So
  the coarse→fine cascade (HEEL→HIP→TWIG→LEAF ≈ meta→**palette**→SPO→full) is
  **convergent intent on both sides and wired on neither.** `HIP = palette`
  is literally D‑PAL256 at a cascade level. **This is the highest‑value
  next materialization:** route `high_heel.rs`'s commented cascade by the
  `hhtl.rs` `NiblePath` prefix — the two files "communicate in module docs,
  not in code" (audit), and today's findings are the spec that joins them.

### Related structure (resonant, different decomposition)

- **Three‑tier model** (Tier 1 `MailboxSoA` data / Tier 2 Kanban‑Rubicon
  lifecycle / Tier 3 ractor supervision) is a *data / lifecycle /
  supervision* split, **orthogonal** to the meta→content→spatial gradient —
  note both, don't conflate. The Kanban 6‑phase (Planning → CognitiveWork →
  Evaluation → Commit → Plan → Prune) = the `ActionState` lifecycle (the
  OGAR Rubicon binding, ADR‑001).
- `qualia [QualiaI4_16D;N]` (4‑bit, 16‑D) = the Quintenzirkel qualia codebook
  (BindSpace dissolution, lance‑graph #470) — a codebook column alongside
  `edges`/`meta`; relates to D‑CARD's codebook cascade.

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
- `docs/CASCADE-SYNERGIES-EPIPHANY.md` **(introduced in OGAR PR #47, co‑merges
  with this map)** — the synergy catalog + amortization gate + storage
  synthesis (the source of most §2 `EPIPHANY` entries; the `SYN §…` links).
- `docs/RDF-OWL-ALIGNMENT.md` — the brutal‑upgrade sequencing (§10 phases)
  + Deep‑NSM (§4.10).
- `docs/DOMAIN-INSTANCES.md` — the 6 universality witnesses.
- `docs/THE-FIREWALL.md`, `HEALTHCARE-TRANSCODING.md` — ADR‑022 + PII floor.
- lance‑graph PR #470/#473/#474/#475/#476/#477/#478, bardioc #17/#18/#19 —
  the runtime‑side receipts (`[per runtime session]` sources).
- **The endgame references (§0):** AriGraph — *Learning Knowledge Graph
  World Models with Episodic Memory for LLM Agents*, Anokhin, Semenov,
  Sorokin, Evseev, Burtsev, Burnaev, arXiv **2407.04363** (2024). DeepNSM
  — lance‑graph `crates/lance-graph/src/nsm/` (`MAX_VOCAB=4096`,
  `NUM_PRIMES=63`, `NUM_ROLES=6`, CAM 6×256; PR #477). The `think`/`do`
  axis ↔ OGAR structural/behavioral IR arms (`ogar-vocab`) + the
  Semantik/Pragmatik trichotomy (`CHESS-TRANSCODING.md §0`).
