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
| **Bad collapse** (moiré) | degenerate aliasing: periodic sampling beats against periodic content → the 256‑palette tile aliases into a false pattern | **prevented** — irrational placement has no period to beat, so the palette tile *can't* alias‑collapse |

So a 256‑cell palette tile placed on the golden lattice carries an
**anti‑degeneracy guarantee**: it can be intentionally LOD‑collapsed
(good) but cannot moiré‑collapse (bad). The irrationality is the guard.

**What's `[H]` here:** the phyllotaxis anti‑moiré math is established; the
specific claim that the helix's *actual* golden‑stride spacing delivers
X‑Trans‑grade protection for the palette tiles needs the runtime session's
helix geometry to confirm the exact stride. **`[per runtime session]`** on
the spacing constant; the *mechanism* is `[H]`.

**Optimization unlocked (later):**
- Skip the optical‑low‑pass‑filter analog entirely (X‑Trans's whole point):
  no separate anti‑alias pass needed if placement is golden.
- The θ‑window (ADR‑025/026, [1.45,1.6] near‑orthogonal) and the
  irrational placement are the **same conditioning story from two angles**:
  near‑orthogonal *codebook* + aperiodic *lattice* = no degenerate beat in
  either the value space (palette) or the position space (tile). Worth
  unifying as "the no‑collapse precondition" in ADR‑026.

---

## 3. palette256 ↔ Product Quantization ↔ codec palette mode ↔ OLED subpixel

**Mixed grade per leg.**

| Leg | Grade | Evidence |
|---|:--:|---|
| palette256 = one PQ subspace's 256 centroids | **[G]** | `nsm_word.rs`: CAM codebook = **6 subspaces × 256 centroids**; `cam_codes.bin` = N words × 6 bytes (lance‑graph PR #477) |
| palette256 ↔ indexed‑color codec palette | **[G]** | HEVC‑SCC + VVC ship an indexed‑palette mode (the codec's own ≤‑256‑ish codebook for screen content) |
| palette256 ↔ OLED subpixel emission | **[S]** | OLED PenTile RGBG is a palette‑on‑a‑lattice for *perceived* resolution; shape‑match only, no structural identity yet |

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

## 7. The nesting precondition — what's free vs paid (carried from prior turn)

The "self‑fulfilling cascade" is free **only along one Morton‑nested axis.**
lance‑graph PR #477 ships **two different orderings of the same 4096 words**:

- `word_rank_lookup.csv` — **frequency** order (`MAX_VOCAB = 4096`).
- `cam_codes.bin` — **semantic** PQ order (6 × 256).

These don't nest into one Morton order (frequency‑rank ≠ semantic‑centroid).
So:

- **Free (one‑time, build):** lay the codebook out in Morton/Hilbert order
  on the *chosen* axis → prefix‑truncation = coarsening → the vertical
  shader cascade (mip / trilinear / DLSS‑upscale) is free at runtime.
- **Paid (stored):** the *other* axis's relationship stays a CAM lookup
  (`cam_codes.bin` *is* that stored freq↔semantic map). You cannot
  Morton‑nest both on one axis.

**The design decision §3 of ADR‑026 must record:** *which* axis gets the
free cascade — **frequency** (common‑words‑first LOD) or **semantic**
(palette‑coherent LOD). Mutually exclusive on one Morton order.

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
   one story — no degenerate beat in value‑space or position‑space. One
   precondition, two guards.
2. **Pick the nesting axis** (ADR‑026 §3): frequency vs semantic free
   cascade. Blocks all vertical‑shader optimization until chosen.
3. **Attention‑driven LOD** (§4): wire the bgz‑tensor WeightPalette rank
   into the `r*` pick — saliency‑ordered lazy materialization.
4. **Borrow codec silicon** (§1): map Morton addressing onto x265/x266 CTU
   hardware quadtree; evaluate HEVC‑SCC/VVC palette mode for the leaf codec.
5. **Structured‑sparse neighborhood BLAS** (§6): implement the neighbor
   walk as a block‑banded stencil, not a sparse GEMM.
6. **Confirm `CausalEdge64 = 2⁶`** (prior turn): if the 64 is the 6‑role
   mask space, the 64‑level is structural and the codebook cascade is
   complete; if it's a 64‑bit word, the 64‑level is decorative.

---

## 10. What the runtime session must confirm

| Claim | Owner | Confirms |
|---|---|---|
| helix golden‑stride spacing constant | `crates/helix` | §2 X‑Trans‑grade moiré protection |
| `CausalEdge64` cardinality (2⁶ mask vs 64‑bit) | lance‑graph‑contract | §0 64‑level structural vs decorative |
| θ‑window [1.45,1.6] + ρ 0.93–0.9973 envelope | `crates/jc` | §2/§9 the no‑collapse precondition |
| `blasgraph` actual scope | runtime | §6 neighborhood‑BLAS framing |
| cognitive‑shader‑driver tile contract | bardioc/lance‑graph | §5 consumer interface |

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

---

> **Reminder of status:** this is epiphany‑capture. The grades and the
> `[per runtime session]` marks are the honest boundary between what the
> OGAR session can stand behind and what awaits the runtime session's
> internals. Optimize *from* this map; pin *into* ADR‑026 only the subset
> that survives §10's confirmations.
