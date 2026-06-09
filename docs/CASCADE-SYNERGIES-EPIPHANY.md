# Cascade Synergies — Epiphany Capture (2026-06-08)

> **Epistemic status: EPIPHANY‑CAPTURE — not pinned architecture.**
> Filed under the standing "document everything before it dilutes"
> mandate. Purpose: record the cross‑domain synergies converging on the
> **Morton‑cascade + palette256 + golden‑helix** substrate so the shader
> shape can be *optimized later against a complete map*, not rediscovered
> piecemeal. Nothing here is a contract; ADR‑022/023/024/025 are the
> pinned floors, and a future **ADR‑026** is expected to formalize the
> subset of this doc that survives verification.
>
> **Grading legend (applied per claim):**
> - **[G] Grounded** — both sides are real verified artifacts / public
>   specs; the synergy is structural, not analogical.
> - **[H] Hypothesis** — the mechanism is sound and one side is real;
>   the other side needs a measurement or a definition to confirm.
> - **[S] Speculative** — suggestive shape‑match, not yet load‑bearing;
>   recorded so it isn't lost, flagged so it isn't trusted.
> - **`[per runtime session]`** — depends on a runtime‑owned internal
>   (`crates/helix`, `crates/jc`, `cognitive-shader-driver`, `blasgraph`)
>   that the OGAR session has not personally verified.

---

## 0. The convergence thesis

Six independent engineering lineages each arrived — separately, for their
own reasons — at the **same three primitives**:

| Primitive | Why each lineage needed it |
|---|---|
| **Quadtree tiling** (recursive 4×4 / Morton subdivision) | rate‑distortion‑optimal block coding; spatial LOD; mip pyramids |
| **256‑entry palettes** (8‑bit codebooks) | indexed color; product‑quantization centroids; attention weight buckets |
| **Aperiodic / irrational placement** (break periodicity) | anti‑moiré without an optical low‑pass filter; anti‑aliasing |

| Lineage | Tiling | Palette | Aperiodic |
|---|:--:|:--:|:--:|
| Video codecs (HEVC/x265, VVC/x266) | CTU quadtree | SCC palette mode | dithering |
| Camera sensors (Fuji X‑Trans) | — | Bayer/X‑Trans CFA | **X‑Trans 6×6 aperiodic** |
| Displays (OLED PenTile) | subpixel grid | RGBG subpixel | subpixel offset |
| Transformer attention (bgz‑tensor) | head tiling | **WeightPalette(256)** | — |
| Vector quantization (CAM‑PQ) | subspace split | **6 × 256 centroids** | — |
| 3D‑tile rendering (Cesium) | **implicit quadtree** | — | — |
| **The substrate** | **Morton cascade** | **palette256 / CAM** | **golden helix** |

**They converged because the math is the same.** The substrate is the
unification: Morton cascade = the tiling, palette256/CAM = the codebook,
golden helix = the irrational placement. This doc maps each lineage's
contribution and the optimization each unlocks.

---

## 1. Morton cascade ↔ x265/x266 CTU quadtree  **[G]**

**Both sides real.** HEVC (x265) codes pictures as **Coding Tree Units**
(CTU, up to 64×64) recursively **quadtree**‑split down to 4×4 transform
blocks. VVC (x266) extends the CTU to 128×128 with a quadtree + multi‑type
(binary/ternary) tree (QTMT). The substrate's cascade (64 → 256 → 1024 →
4096 → … per‑axis, 4×4 Morton leaf) **is** the CTU partition structure.

**The structural identity:**

| Codec concept | Substrate concept |
|---|---|
| CTU (64×64 / 128×128) | the coarse cascade level (64‑ or 128‑per‑axis) |
| quadtree split decision | the LOD level pick (which depth to refine to) |
| rate‑distortion optimization (RDO) of split | **the *probe* version** of the level pick |
| split flag per node | one Morton nibble per hop |
| 4×4 transform block | the Morton leaf nibble |

**The synergy that matters:** a codec decides split depth by **probing**
rate‑distortion at each node (try a split, measure cost, keep or prune).
ADR‑025 decides the same split depth by **closed form** (`r* =
⌈log₄(C/τ)⌉` from the Jirak bound). **The codec's RDO loop is exactly the
trial‑and‑error collapse test ADR‑025 removes.** Same tree, two ways to
pick the depth — probe vs certificate.

**Optimization unlocked (later):**
- *Borrow the codec hardware quadtree.* x265/x266 CTU partitioning is
  hardware‑accelerated on most GPUs/ASICs; the substrate's Morton
  addressing could ride that silicon.
- *Feed closed‑form splits to the codec.* The Jirak `r*` could replace
  (or seed) the RDO split search — a probe‑free encoder front‑end.
- *Palette mode reuse.* HEVC‑SCC and VVC both ship an **indexed‑color
  palette mode** for screen content — the codec's own palette primitive,
  the same indexed‑codebook idea as palette256.

---

## 2. Golden helix ↔ Fuji X‑Trans moiré protection  **[H — the key insight]**

**The insight (operator, 2026‑06‑08):** the golden‑ratio irrationality of
the helix placement isn't *only* for deterministic addressing — it doubles
as a **baked‑in anti‑moiré interlacing protocol**, the same job Fuji's
X‑Trans color‑filter array does.

**Why it's sound:** Fuji X‑Trans uses a **6×6 aperiodic** CFA (vs Bayer's
2×2 periodic) specifically so the sensor pattern has **no regular period to
beat against** image frequencies → moiré without an optical low‑pass
filter. The golden angle (137.5°, φ = the *most irrational* number) is the
classic phyllotaxis anti‑aliasing construction (sunflower seeds, Vogel
spiral): irrational spacing → **no rational period → no aliasing beat**.
The helix golden‑stride placement inherits this for free.

**"x256 that can't collapse" — two senses of collapse, distinguished:**

| Sense | What it is | Golden helix's role |
|---|---|---|
| **Good collapse** (LOD) | intentional coarsening: use a parent tile when SSE permits (ADR‑025) | unaffected — still closed‑form |
| **Bad collapse** (moiré) | degenerate aliasing: periodic sampling beats against periodic content → the 256‑palette tile aliases into a false pattern | **hypothesized to resist** — irrational placement *weakens* the rational period that drives the beat |

So a 256‑cell palette tile on the golden lattice is **hypothesized to
resist moiré‑collapse** (while still admitting intentional LOD‑collapse).
**This is `[H]`, not a guarantee** — see the caveat.

**What's `[H]` here — and the caveat Codex flagged (PR #47):** the
phyllotaxis anti‑moiré *tendency* is established, but it is **not an
absolute "cannot".** Two reasons it must stay a *measured* hypothesis:
1. **X‑Trans is itself a *repeating* 6×6 tile** — a finite aperiodic cell
   tiled periodically, not a globally aperiodic field. Its moiré reduction
   is empirical and partial, not a theorem.
2. **Irrational placement *inside* a repeated tile / LOD lattice can still
   have spectral peaks** from the tiling and pyramid lattices. "No rational
   period" is a property of the *infinite* golden sequence; a bounded
   256‑cell tile has a finite spectrum and can beat against the LOD lattice.

So: **`[H]` pending spectral / aliasing validation.** **ADR‑026 must NOT
skip an anti‑alias pass on the strength of this** until the helix's actual
golden‑stride spacing is measured against the tile + LOD spectra. The
spacing constant is `[per runtime session]`; the *guarantee* is `[H]` and
explicitly **unmeasured**.

**Optimization candidate (gated on measurement):**
- *If* a spectral test confirms it, the optical‑low‑pass‑filter analog
  could be skipped — **only after** that measurement, never before.
- The θ‑window (ADR‑025/026, [1.45,1.6] near‑orthogonal) and the
  irrational placement are the **same conditioning story from two angles**:
  near‑orthogonal *codebook* + aperiodic *lattice* = *hypothesized* to
  suppress the degenerate beat in both value space (palette) and position
  space (tile). Worth unifying as "the no‑collapse precondition" in
  ADR‑026 — **as a measured hypothesis, with the spectral validation as
  its receipt.**

---

## 3. palette256 ↔ Product Quantization ↔ codec palette mode ↔ OLED subpixel

**Mixed grade per leg.**

| Leg | Grade | Evidence |
|---|:--:|---|
| palette256 = one PQ subspace's 256 centroids | **[G]** | `nsm_word.rs`: CAM codebook = **6 subspaces × 256 centroids**; `cam_codes.bin` = N words × 6 bytes (lance‑graph PR #477) |
| palette256 ↔ indexed‑color codec palette | **[G]** | HEVC‑SCC + VVC ship an indexed‑palette mode (the codec's own ≤‑256‑ish codebook for screen content) |
| palette256 ↔ OLED subpixel emission | **[S — weakest leg; candidate for demotion]** | OLED PenTile RGBG is a palette‑on‑a‑lattice for *perceived* resolution; shape‑match only. **Operator (2026‑06‑08): "not sure what we can learn from excitons in OLED" — agreed.** The *only* defensible exciton→substrate map is **exciton diffusion length ↔ neighborhood kernel width** (§6 — how far a cell's influence propagates before it "recombines"), and possibly **density quenching / efficiency droop ↔ a saturation limit on useful cell density** (diminishing returns past a refinement depth). Both are thin analogies, not structural. **Recommendation: keep [S], do not build on it; promote only if a measured exciton parameter maps to a measured substrate parameter.** The subpixel *layout* (not the exciton physics) is the part that's even shape‑relevant, and that's already covered by Morton tiling. |

**The convergence number is 256 = 2⁸ = one byte.** PQ centroids, codec
palette indices, attention weight buckets (§4), and Binary16K lane
structure (256² = 64k) all land on it because one byte is the natural
SIMD‑lane / cache‑line / palette‑index granule. ADR‑024 already pins this
as "the codec"; the new observation is how *many* independent lineages
chose the same byte.

**Optimization unlocked (later):** a single 256‑entry codebook can serve
PQ (semantic), codec palette (compression), and tile centroid (spatial)
*simultaneously* if the codebook is laid out once in Morton/Hilbert order
(see §7 — the nesting precondition). One palette, three consumers.

---

## 4. Attention headers ↔ bgz‑tensor WeightPalette ↔ attention‑driven LOD  **[G→H]**

**Grounded base:** `bgz-tensor` ships `WeightPalette::build(…, 256)` +
`AttentionTable::build` (ADR‑024 reference) — attention weights are
palette‑quantized to 256 on the model hot path.

**The hypothesis to wire:** if attention is already palette256‑coded, and
tiles are Morton‑addressed, then **attention can rank tiles → ranking
drives Morton refinement depth.** The cognitive‑shader‑driver attends to a
region; the attention header is the importance map; importance ranks tiles;
rank sets `r*` (refine the attended tiles deeper, coarsen the ignored
ones). This is **attention‑driven LOD** — the transformer's importance map
*is* the LOD oracle.

**Structural identity:** attention = a learned importance distribution;
LOD = an importance‑driven refinement. ADR‑025's `r* = ⌈log₄(C/τ)⌉` uses a
*Jirak‑certificate* tolerance τ; attention‑driven LOD would use a
*learned‑attention* tolerance. Same `r*` machinery, different source of τ —
certificate for provable bounds, attention for learned saliency. They can
compose: τ = min(certificate, attention) → refine where *either* the bound
or the model demands it.

**Optimization unlocked (later):** "palette ranking attention headers wired
into cognitive‑shader‑driver" (operator's phrasing) = the attention table's
top‑ranked palette entries select which tile centroids materialize first —
a saliency‑ordered lazy materialization. Free at the index (rank is a
sort over 256 bytes); paid only at the materialized leaves.

---

## 5. cognitive‑shader‑driver — the consumer  **`[per runtime session]`**

The `cognitive-shader-driver` (the BindSpace‑dissolution target, bardioc
PR #18 / lance‑graph PR #470) is the hot‑path shader that *consumes* the
Morton‑addressed, palette‑coded, attention‑ranked tiles. It is the literal
"GPU shader" in the "akin to a GPU shader with free upscaling" framing:

| Shader stage | Substrate input |
|---|---|
| vertex / tile fetch | Morton prefix → address‑derived bounds (no fetch‑test) |
| fragment / per‑cell | helix template → centroid + Σ (closed‑form) |
| texture sample | palette256 / CAM code → value (1 Lance read at the leaf) |
| LOD / mip select | `r*` closed‑form (ADR‑025) or attention‑ranked (§4) |

**`[per runtime session]`** on everything inside the driver — OGAR sees the
*contract* (Morton address + palette code + `r*`), not the shader internals.

---

## 6. blasgraph + neighborhood = structured‑sparse BLAS  **[H, `[per runtime session]` on `blasgraph`]**

*(Inferring `blasgraph` = the BLAS / GEMM execution layer over the
lance‑graph structure; correct me if it's a specific crate.)*

The cascade's **neighborhood** operation (neighbor‑XOR walk at a level +
parent‑prefix for context — §1 of the prior turn) is a **structured sparse
matrix**: the Morton‑neighbor adjacency is a banded/block matrix with
constant per‑row fan‑out (4 neighbors + 1 parent). Aggregating over a
neighborhood = a sparse matrix‑vector product over that adjacency =
**a BLAS op** (`blasgraph`). The GPU shapes this enables:

| GPU/BLAS shape | Cascade neighborhood equivalent |
|---|---|
| 2D convolution / stencil | neighbor‑XOR aggregation at a fixed level |
| trilinear interp across mips | cross‑level XOR‑weighted blend (Morton‑Hamming weight) |
| sparse GEMM | neighborhood message‑passing over the Morton adjacency |
| anisotropic filtering | neighbor walk weighted by the helix Σ (the per‑cell ellipsoid) |

**Optimization unlocked (later):** because the adjacency is *structured*
(Morton‑regular, constant fan‑out), the sparse BLAS is a **dense
block‑banded** op — no sparse‑matrix overhead, no gather/scatter; it's a
shifted‑add stencil, the cheapest GPU primitive. The neighborhood compute
is therefore as fast as a blur kernel.

---

## 7. The amortization gate — *not* "free vs paid", and *not* one axis  **[corrected 2026-06-08]**

> The prior framing of this section ("free along **one** Morton‑nested axis;
> the other is a reluctant stored CAM lookup; **pick** frequency *xor*
> semantic") was **over‑cautious and is superseded.** Operator correction:
> *"we have SoA with enough headroom for a reason — we can spend whatever
> we need as long as it amortizes on mipmap cascade or Semantik volumetric
> centroid."* The right gate is **amortization, not free.**

**The mechanical fact stands:** lance‑graph PR #477 ships two orderings of
the same 4096 words — `word_rank_lookup.csv` (frequency, `MAX_VOCAB=4096`)
and `cam_codes.bin` (semantic PQ, 6×256) — which **don't** nest into one
Morton order. The prior turn concluded "so pick one." **That conclusion was
wrong.** The SoA register‑file (PR #477 `SoaEnvelope`) has deliberate
headroom; you **store both cascades**, because each **amortizes** on its own
reuse axis.

**The amortization gate (the real admission test):** a cost `C` is
admissible iff it amortizes over **either** reuse axis:

| Reuse axis | Pay `C` once per… | Reused across… | Amortized cost |
|---|---|---|---|
| **Mipmap cascade** (spatial) | build / level | all levels × all queries at them | `C / (levels × queries) ≈ 0` |
| **Semantik volumetric centroid** (semantic) | semantic centroid | all queries falling in that centroid's volume | `C / (queries‑per‑centroid) ≈ 0` |

A cost that amortizes on **neither** — paid per‑query, no reuse — is the
*only* forbidden thing. That is exactly the "serialization in the hot path"
(ADR‑022) and the "probe" (ADR‑025) the floors already ban. **So the
amortization gate is not a new rule — it is the unifying statement of
ADR‑022 + ADR‑025:** the hot path forbids *non‑amortized per‑query* cost;
everything build‑time, per‑level, or per‑centroid is affordable because the
SoA headroom holds it and it amortizes to ≈ 0.

**Consequences for the cascade:**

- **Store both orderings.** The frequency cascade (common‑words‑first LOD)
  *and* the semantic cascade (palette‑coherent LOD) both live in the SoA —
  each is built once (amortizes on the mipmap axis) and each is reused
  across all queries (amortizes on the centroid axis). `cam_codes.bin` is
  **not** a reluctant compromise; it is the deliberately‑stored second
  cascade.
- **The "nesting axis" decision dissolves.** There is no frequency‑*xor*‑
  semantic fork. ADR‑026 records *both* cascades + the amortization gate as
  the admission test, not a single chosen axis.
- **The budget is the SoA headroom, not zero.** "Spend whatever amortizes"
  is bounded by SoA capacity — generous, but finite. The discipline is:
  *amortize or don't spend*; within amortizable spends, bounded by headroom.

This is strictly stronger than the GPU‑shader analogy (§1–§6): a GPU also
spends freely on anything that amortizes over the framebuffer (build a LUT
once, sample it per‑pixel forever). The substrate's "SoA headroom +
amortization gate" is that same discipline, named.

**The gate is fractal — it applies at three scales:**

| Scale | Spend unit | "Amortize or don't spend" test |
|---|---|---|
| **storage** (§7.5) | a column / Z‑ordered row‑group | reused across all queries that scan the tile |
| **cascade** (§7) | a precomputed level / centroid | reused across all levels (mipmap) / queries (centroid) |
| **bit** (§9.6) | the 48→64 headroom bits in `CausalEdge64` | spend on the seed **iff** it's irreducible‑beyond‑the‑address (else compute, don't store) |

Same gate, three granularities. The "right reasons" the operator names for
the bit budget (§9.6) are the bit‑scale instance of this one rule.

---

## 7.5 The immaterialized cascade as storage — the Morton-keyed columnar grid-pyramid  **[storage synthesis, 2026-06-08]**

**The operator's framing:** the Morton cascade is a **coordinate transform,
not a stored grid** — exactly as `(lat, lon) → quadkey` is a cheap
closed-form bit-interleave with no materialized grid. The cascade computes
cell membership on demand; it is never *built*. That makes it a **reusable
addressing layer** any payload can hang off.

**Address vs payload (ADR-023) at storage scale:**
- **ADDRESS** = the Morton prefix — immaterialized, computed cheap, never
  stored as a grid. Amortizes to ≈ 0 (it's arithmetic, not data) — the §7
  gate's best case.
- **PAYLOAD** = columnar (Lance / Parquet-family / the PR #477 SoA register
  file), with rows in Morton order.

**The columnar-pushdown identity  [G — deployed lakehouse tech]:** ordering
a columnar table's rows by a space-filling curve (Z-order / Hilbert) so
range-predicates skip non-matching pages is **production tech** (Databricks
Delta `ZORDER`, Iceberg/Hudi clustering, BigQuery clustering all ship it).
The substrate is the same move:

| Columnar concept | Cascade concept |
|---|---|
| row keyed by Morton prefix | a cell |
| row-group / fragment over a Morton-prefix range | **a tile** |
| predicate pushdown on a Morton-prefix range | **the tile fetch** (subtree scan = data-skipping) |
| column-chunk | a per-role / per-level SoA column (PR #477 `SoaEnvelope`) |
| page | the leaf nibble |
| column projection | fetch only the roles you need |

So the columnar format's **own machinery** (row-groups, pushdown,
projection, data-skipping) *is* the cascade's storage + tile-fetch — free,
on production-proven tech.

**One sharpening (honest):** classic Parquet row-groups are **scan**-
optimized; the cascade wants **random** tile access (jump to any prefix,
any version). That is precisely **Lance's** advantage over classic Parquet
(fragment/page random-access for ML workloads). So "parquet-shaped" is the
right *family* (columnar + Z-ordered); **Lance is the substrate's actual
instance**, chosen because the shader needs random access, not sequential
scans.

**Four payloads, one immaterialized address:**

| Payload | What it is | Grade |
|---|---|---|
| **delta frames** | a version-diff = the Morton cells whose payload changed since the reference version (Lance versioning) — the codec P-frame (§1). I-frame = a materialized version (vacuum point); P-frame = a version-delta. *(B-frames / bidirectional refs don't map onto an append-only version log — honest limit.)* | [H] |
| **radix trie** | VART lazy-materialized prefix nodes — only touched paths stored; the trie *is* the cascade | [G] |
| **HHTL / OGIT / helix** | one Morton address = compile-time HHTL identity + OGIT class identity + helix placement seed (ADR-023/024/025) | [G] |
| **CAM-PQ** | 6 roles × 256 centroids = the semantic column values (§3, §7) | [G] |

**The unifying shape — "parquet-shaped grid-pyramid shader":**

```
columnar storage (Lance / Parquet-family / SoA)   ← the parquet
   + rows in Morton order (Z-ordered)             ← the grid
   + level cascade (mip pyramid)                  ← the pyramid
   + closed-form per-cell from address (§5)       ← the shader
```

The address is the cheap deterministic transform (free — §7 best case); the
columns are the amortized payload (build-once, query-many — §7 gate). **The
whole substrate storage layer is one Z-ordered columnar grid-pyramid,
queried by Morton-prefix pushdown, executed shader-style** — every layer
production-proven (Lance columnar + lakehouse Z-order + GPU shader), just
composed.

**Honest limit (the immaterialization isn't total):** the *address* is
immaterialized (arithmetic); the *payload* is materialized in the columns
(it has to live somewhere). "Immaterialized cascade" = immaterialized
*addressing* over materialized *columnar payload*. The grid is free; the
data is the SoA-headroom spend (§7). The precise claim: you never store the
grid; you always store the (Morton-ordered, palette-coded, amortized)
columns.

---

## 8. The synergy matrix (everything against everything)

| | Morton cascade | golden helix | palette256/CAM | attention | Cesium | x265/x266 |
|---|---|---|---|---|---|---|
| **HHTL** | address = prefix [G] | placement template [per‑rt] | codebook leaf [G] | rank → depth [H] | tileset id [G] | CTU id [G] |
| **helix** | centroid/Σ from prefix [per‑rt] | — | θ‑window conditioning [H] | — | implicit bounds [H] | — |
| **palette256** | leaf value [G] | anti‑moiré value [H] | — | weight bucket [G] | — | SCC palette [G] |
| **neighborhood** | XOR walk [G] | Σ‑weighted [per‑rt] | — | — | LOD blend [H] | deblock filter [S] |
| **Cesium** | implicit quadtree [G] | — | — | saliency LOD [H] | — | shared tiling [G] |
| **x265/x266** | CTU = cascade [G] | dither analog [S] | palette mode [G] | — | shared tiling [G] | — |

*(Cells: the synergy + its grade. Empty = no direct synergy identified yet.)*

---

## 9. Optimization roadmap — the "later‑optimize" targets this doc unlocks

Ordered by leverage (highest first):

1. **Unify the "no‑collapse precondition"** (ADR‑026 §2+): θ‑window
   (near‑orthogonal codebook) + golden placement (aperiodic lattice) are
   one story — *hypothesized* to suppress the degenerate beat in value‑ and
   position‑space. One precondition, two guards — **both requiring
   measurement** (ρ‑vs‑reference for θ; spectral/aliasing test for the
   golden lattice — §2 caveat). Pin as a *measured* precondition, not a
   guarantee.
2. **Store both cascades under the amortization gate** (ADR‑026 §3,
   §7‑corrected): frequency *and* semantic cascades both live in the SoA;
   each amortizes (mipmap / centroid). No "pick one axis" fork — the gate
   is *amortize‑or‑don't‑spend*, bounded by SoA headroom.
3. **Attention‑driven LOD** (§4): wire the bgz‑tensor WeightPalette rank
   into the `r*` pick — saliency‑ordered lazy materialization.
4. **Borrow codec silicon** (§1): map Morton addressing onto x265/x266 CTU
   hardware quadtree; evaluate HEVC‑SCC/VVC palette mode for the leaf codec.
5. **Structured‑sparse neighborhood BLAS** (§6): implement the neighbor
   walk as a block‑banded stencil, not a sparse GEMM.
6. **`CausalEdge64` = the meta layer** (operator‑resolved 2026‑06‑08):
   the 64 is **both** — `2^NUM_ROLES = 2⁶ = 64` role‑participation masks
   (which of the 6 semantic roles this causal edge binds) carried *within*
   a 64‑bit word. It is the cascade's **meta level**: the coarsest level is
   not spatial content but **structural awareness** ("which roles
   participate"), amortizing maximally — one `CausalEdge64` describes the
   role‑structure for everything below it. So the 64‑level is **structural,
   not decorative**, and it is *meta‑structural*, which is why both readings
   are correct simultaneously.

   **The third "both" — the bit budget** (operator, 2026‑06‑08): the CAM‑PQ
   code is `6 roles × 8 bits = 48 bits`; in a 64‑bit word that leaves
   **16 bits headroom**. Candidate spend: the **irrational placement /
   X‑sensor anti‑moiré seed** (§2) — so one 64‑bit word would carry the
   role‑mask (meta) + the 48‑bit CAM‑PQ (semantic) + a 16‑bit seed
   (placement/anti‑collapse). **But "only for the right reasons"** — and
   this is **the amortization gate at the bit level** (§7, now fractal:
   storage §7.5 / cascade §7 / bit §9.6):

   > Spend the 16 headroom bits on the seed **iff the seed carries
   > information not in the address.** A golden‑ratio seed that is
   > *derivable from the Morton prefix* must be **computed, not stored** —
   > storing it is redundant‑with‑address padding (the wrong reason). A
   > *per‑tile measured/learned* moiré perturbation that is **not**
   > prefix‑derivable is irreducible information → store it (the right
   > reason).

   So the test for the 48→64 headroom is sharp: **irreducible‑beyond‑the‑
   address ⟹ store; address‑derivable ⟹ compute.** This is the same
   "amortize‑or‑don't‑spend" gate, applied to bits instead of columns or
   levels. `[per runtime session]` on the actual 64‑bit layout + whether the
   seed is derivable or measured.

---

## 10. What the runtime session must confirm

| Claim | Owner | Confirms |
|---|---|---|
| helix golden‑stride spacing constant | `crates/helix` | §2 X‑Trans‑grade moiré protection |
| ~~`CausalEdge64` cardinality~~ — **resolved** (operator): `2⁶` role‑mask meta *within* a 64‑bit word; structural + meta (§9.6) | — | §0/§9 — closed |
| θ‑window [1.45,1.6] + ρ 0.93–0.9973 envelope | `crates/jc` | §2/§9 the no‑collapse precondition |
| `blasgraph` actual scope | runtime | §6 neighborhood‑BLAS framing |
| cognitive‑shader‑driver tile contract | bardioc/lance‑graph | §5 consumer interface |
| SoA headroom budget (the amortization‑gate ceiling, §7) | lance‑graph `SoaEnvelope` (PR #477) | §7 how much "spend‑if‑amortizes" the register file affords |

---

## 11. Cross‑references

- `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` — ADR‑022 (boundary),
  ADR‑023 (IR‑as‑wire‑truth), ADR‑024 (palette256 + HHTL codec),
  ADR‑025 (probe‑free hot path). The pinned floors this doc sits on.
- `docs/RDF-OWL-ALIGNMENT.md` §4.10 — the 4096‑dim Deep‑NSM encoder /
  Wierzbicka primes (`NUM_PRIMES = 63`, lance‑graph `nsm/encoder.rs`).
- `lance-graph` PR #477 — `CausalEdge64`, the CAM‑PQ codebook
  (6 × 256), `nsm/nsm_word.rs`, the SoA envelope LE contract.
- `lance-graph` PR #478 — singleton‑to‑snapshot nudge; read‑only
  codebooks (role keys `SUBJECT_KEY…`) stay as const tables.
- `lance-graph` PR #470 + bardioc PR #18 — BindSpace dissolution,
  the cognitive‑shader‑driver target.
- External specs (public): ITU‑T H.265 (HEVC/x265) CTU + SCC palette
  mode; ITU‑T H.266 (VVC/x266) CTU + QTMT; Fuji X‑Trans CFA;
  Vogel/phyllotaxis golden‑angle anti‑aliasing; Product Quantization
  (Jégou et al.).
- Deployed columnar / space‑filling‑curve clustering (the §7.5
  storage anchor): Databricks Delta `ZORDER BY`, Apache Iceberg /
  Hudi clustering, BigQuery clustering — all production data‑skipping
  by Z‑order/Hilbert row ordering. Lance (the substrate's columnar
  instance) — fragment/page random‑access columnar format, chosen over
  classic Parquet row‑groups for random tile access.

---

> **Reminder of status:** this is epiphany‑capture. The grades and the
> `[per runtime session]` marks are the honest boundary between what the
> OGAR session can stand behind and what awaits the runtime session's
> internals. Optimize *from* this map; pin *into* ADR‑026 only the subset
> that survives §10's confirmations.
