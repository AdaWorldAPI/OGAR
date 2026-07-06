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
> - **`5+3 review` = the agent‑hardening apparatus** at `.claude/agents/`
>   (5 research + 3 brutal‑review charters; introduced 2026‑06‑09 with the
>   D‑EXCITON revert). "Reverted/graded by the 5+3 review" = the receipts are
>   those agents' findings; the apparatus is in this repo, the per‑run
>   transcripts are session‑local.
> - **`INTEGRATION-MAP.md`** = the composition companion (2026‑06‑09): this
>   map indexes *what was found*; that one maps *how it composes* — layers,
>   seams (each with its contract TYPE), the merged phase DAG, and the
>   falsification gates F1–F9. Discovery → there; sequencing → here.
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
| **episodic memory** (event sequence) | **EPISODIC = delta frames** = `DatasetVersion(v)→(v+1)` + per‑row `cycle` stamp (D‑DELTA; label fold 2026‑06‑10: version = self‑contained snapshot, cycle = RECENCY stamp — watermark‑filter, never diff‑reconstruct) | H |
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
| D‑CASCADE | 64→256→1024→4096→16k→64k→256k = immaterialized Morton enumeration; every level = +1 nibble. **Algorithmically grounded** by generalized Morton/Hilbert ordering for **non‑power‑of‑2 + 3D** dims (arXiv 2309.15199, Walker — the `6×4×4` example = `6 roles × 4×4 tile`; the octant recursion = the cascade; non‑pow‑2 generalization is what lets **Base17 (D‑BGZ17)** be Morton‑ordered). | G | EPIPHANY | SYN §7.5 + §7 papers | D‑MORTON |
| D‑IMMAT | the cascade is a **coordinate transform, not a stored grid** (`(lat,lon)→quadkey` cheap) | G | EPIPHANY | SYN §7.5 | D‑CASCADE |
| D‑NEIGH | neighbor‑XOR walk + parent‑prefix = structured‑sparse stencil (block‑banded, not sparse GEMM) | H | EPIPHANY | SYN §6 | D‑MORTON, `[per rt]` blasgraph |
| D‑FMA‑SKELETON | FMA skeleton = the **clamped convergence anchor**: ~206 bones as immutable **16×8‑bit Morton‑tile family‑node** addresses derived from rest‑pose centroids ⟹ prefix = partonomy = spatial containment (D‑BOTHCASC realized); bones are non‑negotiable Dirichlet anchors, the cross‑modal frame ViT / X‑ray / ultrasound × Doppler register onto. Address structure CODED; splat‑fit convergence CONJECTURE. | G (structure) / H (convergence) | CODED | `crates/ogar-fma-skeleton` + `docs/FMA-SKELETON-CONVERGENCE-ANCHOR.md` | D‑MORTON; `SPLAT-NATIVE-CUSTOMER.md` §6 |
| D‑GUID‑TIER | The brutal uniform **`[256:256]` `[container:member]`** GUID (scale‑free: galaxy:planet … residue:atom); HhtlMode **Located** (Cesium 3D‑octree CRS) vs **Cascade** (self‑speaking ontology path); leaf = familyNode:identity. 3D‑octree HEEL/HIP (`morton3`), the ModalityProjection contract (ViT/X‑ray/US×Doppler `register`+`project` by Guid). 12+4 EdgeBlock removed (superseded by family nodes). | G | CODED | `crates/ogar-fma-skeleton/{guid,morton,projection}.rs` | D‑FMA‑SKELETON, D‑MORTON |
| D‑NODEGUID‑AUDIT | Group‑by‑group audit of the FMA tier `Guid` vs lance‑graph `NodeGuid` (canon rule: wrappers audited against OGAR). **Caught F‑1: lance‑graph `CLASSID_FMA=0x0901` aliases OGAR `patient`** — this session's `0x0A` Anatomy domain resolves it. Also: classid/family/identity width + endianness + EdgeBlock divergences, each with a reconciliation. | G | CODED | `docs/NODEGUID-CANON-AUDIT.md` | D‑GUID‑TIER; lance‑graph `canonical_node.rs` |
| D‑V1‑TAIL‑RETIRED | **MIGRATION MANDATORY (operator, 2026‑07‑04):** the flat `family:u24 ++ identity:u24` V1 tail is **FORBIDDEN for new units.** V3 is the content‑blind `classid(4) + 12B` facet — 12B an axis‑grouped register (`6×(u8:u8)` / `4×(u8:u8:u8)` / `3×(u8:u8:u8:u8)`), the ClassView holds every reading at once (`part_of:is_a` / `X:Y` / `palette256²` / `segment:taxonomy` / typed) and `u8:u8` is **never widened** (u16 = the deprecated V2 flat read; u24 = V1). A flat u24 has no axis → cannot carry a rail. Append‑only: u24 stays read‑only for pre‑flip GUIDs. Producer already V3: `ruff_spo_address::Facet` via `ogar‑from‑ruff`. Regrades D‑NODEGUID‑AUDIT F‑3; pin superseded in `CLAUDE.md` P0. | G | RULING | `CLAUDE.md` P0 (regraded) + `docs/NODEGUID-CANON-AUDIT.md` F‑3 (inverted) + `docs/V3-TRANSPILER-ADR.md` | D‑NODEGUID‑AUDIT; D‑GUID‑TIER; lance‑graph `E‑V3‑FACET‑4‑PLUS‑12` |

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
| D‑BGZ17 | the **discrete/immutable** counterpart to D‑MANTISSA: not "irrational" but **coprime‑aperiodic** — Base17 (`SpoBase17 = [i16;17]×3`, prime 17 ⊥ base‑16 Morton) ⟹ beat period `LCM(16,17)=272` (longest → lowest‑freq moiré). **Anti‑moiré upgrades `[H]`‑unmeasured → `[H]` with a *provable bound*** (number theory, not spectral) — still "longest‑period beat," not "no beat" (§2 caveat persists). `HighHeelBGZ` (basin + ≤240 edges) **realizes §0 AriGraph basin+supporters**; the "quorum" = the L1‑threshold basin‑merge consensus set. **Resolves the §4.1 unwired gap** (route the HEEL/HIP/TWIG/LEAF container by the `hhtl.rs` NiblePath; Base17=TWIG is the anti‑moiré layer). ρ=0.965 (TWIG) joins ρ=0.9973 (HIP) as measured anchors. For *immutable*, prefer this over D‑MANTISSA (exact/hashable vs float). | H | EPIPHANY | SYN §2 + §4.1 + §0 | D‑MANTISSA, D‑CAM, `[per rt]` `bgz17`/`HighHeelBGZ`/`SpoBase17` |
| D‑QUANTGATE | **the quantization principle** (operator, 2026‑06‑08): **whenever there is quantization, the irrational does not have the Morton tile cascade guarantee.** Continuous irrationals (golden mantissa / D‑MANTISSA) lose their aperiodicity when quantized onto a lattice — the rounded value re‑acquires the period of the grid. So at every quantized layer, the substrate must use the **discrete coprime route** (D‑BGZ17, Base17×Morton) to preserve aperiodicity. The continuous irrational only survives as a *muscle‑memory* mantissa quorum: pre‑computed at codebook build time and **never re‑quantized in the hot path** (consistent with the amortization gate D‑AMORT). This **promotes ADR‑026's no‑collapse precondition (D‑NOCOLLAPSE) into a layered rule**: discrete‑coprime at every quantized boundary; muscle‑memory irrationals only at build‑time / continuous layers. **Reframes the §2 anti‑moiré caveat as architectural, not just unmeasured.** | G (the principle is structural) | EPIPHANY → ADR‑026 §2 | SYN §2 | D‑MANTISSA, D‑BGZ17, D‑AMORT, D‑NOCOLLAPSE |
| D‑EXCITON | OLED exciton ↔ anti‑moiré: **REVERTED `[H]→[S]`** by the 5+3 agent review (2026‑06‑09; apparatus `.claude/agents/`, provenance §7). The `[H]` "OLEDs ship the same coprime aperiodicity as D‑BGZ17" was a **category error + a number mis‑label**: (a) the 1:3 S:T ratio is **exactly rational** — the SU(2) microstate count (1 vs 3), a *definitional identity, not a measurement* (`[G]` definitional), so *not* irrational; (b) **a scalar has no period to be aperiodic about** — aperiodicity is a property of an infinite tiling/sequence, and a bias‑shifted ratio or two energy scalars are points, not lattices; (c) the "~1.5 eV triplet *binding* energy" was the **T1 excited‑state energy mis‑labelled** — literature triplet E_b is sub‑eV (≈0.3–0.6 eV, arXiv 2311.03927), so the ratio is order ~1 **not** 3, and the *specific 1:3‑energy coincidence* that motivated the link does not survive. **Kept (`[G]`‑real legs, held apart from the dead analogy):** rational 1:3 (above); and the *harvest‑the‑dark physics* — phosphorescence (heavy‑metal SOC) + TADF (small ΔE_ST RISC) reach ~100% IQE (Baldo/Adachi; Uoyama 2012). Bias‑dependence (PMC4614446) is a **real but material‑specific** observation (`[G]` in its own literature) that carries **no aperiodicity and no substrate test** → its *link* value is `[S]`. **A `[G]` mechanism does not lend its grade to the `[S]` rhyme.** **Rejected re‑links (logged so they're not re‑proposed):** A = anti‑moiré ladder (no lattice/beat); B = amortization gate (no reuse axis — 1‑use recovery ≠ 1/N reuse). **Surviving reframe → `D‑LOSSCHAN` (§2.7), `[S]` do‑not‑build.** | S | EPIPHANY (reverted) | `.claude/agents/` 5+3 review; SYN §3 = also revert | — (anti‑moiré deps removed) |
| **D‑MONOTILE** | **the *third* anti‑moiré route — strongest, by theorem.** An **aperiodic monotile** (the "hat," Smith‑Myers‑Kaplan‑Goodman‑Strauss 2023; arXiv **2509.12216** Kaplan) is a *single shape that tiles the plane only non‑periodically* → it has **no translational period → no moiré beat, by a tiling theorem**, not by placement (D‑MANTISSA, `[H]` continuous) or by base‑coprimality (D‑BGZ17, `[H]`‑bounded). **The anti‑moiré is `[G]` (proven), and it survives quantization (a fixed discrete shape, immutable)** — so it satisfies D‑QUANTGATE *intrinsically*. **Cost / open question (`[H]`):** the hat is a **polykite on a hexagon+triangle lattice**, not the square Morton quadtree; adopting it changes the cascade geometry. Its **substitution (inflation) hierarchy** *is* a cascade — the open test is whether that hierarchy is **generalized‑Morton/Hilbert‑addressable** (arXiv 2309.15199, Walker — non‑power‑of‑2 + 3D Morton/Hilbert). If yes: anti‑moiré‑by‑theorem **+** cascade addressing. Hexagonal substrate ties to the `m²−mn+n²` (Eisenstein) covering norm of arXiv 2203.09323 (Richter). | anti‑moiré **G**; cascade‑addressability **H** — **promotion condition tightened (2026‑06‑10):** `[H]→[G]` ONLY if the substitution hierarchy is shown generalized‑Morton/Hilbert‑addressable (the Walker leg) AND the tiling is hat‑equivalent (or the addressing provably generalizes) — a 5+3 pass alone does NOT promote (F2 bridge condition) | EPIPHANY | §7 papers; `quasicryth-research` (arXiv 2603.14999) is the purpose‑described crate — F2 reads it | D‑BGZ17, D‑QUANTGATE, D‑CASCADE |

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
| D‑DELTA | delta frames = version‑diff = changed Morton cells = codec P‑frame (I/P map; B does not, append‑only log). **Label fold (2026‑06‑10, 5+3):** a Lance `DatasetVersion` is a **self‑contained snapshot** and `last_active_cycle` is a per‑row **RECENCY stamp** (WHEN, not WHAT) — consumers watermark‑filter, never diff‑reconstruct; the *mechanism* stays `[G]`, only the "changed‑cell delta" label is corrected | H→G (mechanism) | EPIPHANY | SYN §7.5,§1; INTEGRATION‑MAP L6 | D‑LANCE |

### 2.7 Cross‑domain synergy catalog

| ID | Shape | Grade | Status | Home | Deps |
|---|---|:--:|:--:|---|---|
| D‑CTU | Morton cascade = x265/x266 CTU quadtree; codec RDO split = the probe | G | EPIPHANY | SYN §1 | D‑CASCADE, D‑CESIUM‑PROBE |
| D‑ATTN | attention (bgz‑tensor WeightPalette 256) ranks tiles → drives `r*`; τ = min(certificate, attention) | H | EPIPHANY | SYN §4 | D‑RSTAR, D‑PAL256 |
| D‑CONVERGE | 6 lineages (codecs/sensors/displays/attention/PQ/Cesium) → quadtree + 256‑palette + irrational | G | EPIPHANY | SYN §0 | — |
| D‑OLED | OLED exciton ↔ substrate = weakest leg; **do not build on**. (The `D‑EXCITON` `[H]` promotion attempt was **reverted to `[S]`** by the 5+3 review — §2.4 + `.claude/agents/`; this row was right all along.) | S | EPIPHANY | SYN §3 | — |
| D‑LOSSCHAN | the one surviving exciton reframe: **loss‑channel suppression** — radiative yield `η = k_prod/(k_prod+k_loss)` (Stern‑Volmer) ↔ cascade **early‑exit** yield (terminate‑cheap vs escalate). *Conjecture to test*, not a shape‑match yet: do TADF IQE and the bgz17 HEEL/Scent early‑exit rate both track that η‑form? Untested → **do‑not‑build**. | S | EPIPHANY | SYN §3 | D‑EXCITON, D‑AMORT |

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
| D‑HIRO‑DO | OGIT Automation → DO arm: `into_action_def` lifts `KnowledgeItem`→`ActionDef` (object_class←`relates`, kausal←`contains Trigger`, body **pointed‑to** not inlined — lossless‑DO §1); schema half of `PROBE‑OGAR‑DO‑ARM‑LIFT` green; lift→`emit_action_def`→SPO triples proven end‑to‑end (`tests/do_arm_emit.rs`, lossless‑DO holds across emit) | G | CODED (schema half) | `ogar-from-schema/src/do_arm.rs` | D‑VOCAB, D‑TTL, D‑ELIXIR, D‑EMIT |
| D‑MARS‑CLASSID | MARS/Automation classids MINTED: `ConceptDomain::Automation` (0x0C), 9 concepts (`mars_application/resource/software/machine` 0x0C01‑04, `knowledge_item` 05, `mars_node_template` 06, `action_handler` 07, `action_applicability` 08, `automation_trigger` 09) — one domain spanning MARS structural CMDB + Automation DO‑arm (Auth precedent); resolves MARS‑TRANSCODING §1 deferral; passed 5+3 hardening (theorem/doctrine/integration/runtime savants + drift‑guards). Reserves the speculative rest | G | CODED | `ogar-vocab/src/lib.rs`, `ogar-class-view/src/lib.rs` | D‑VOCAB, D‑HIRO‑DO |
| D‑ACTIONHANDLER‑PARITY | arago HIRO ActionHandler ⟷ OGAR: `assemble_action_handler` walks the OGIT `provides` graph (`ActionHandler→ActionApplicability→ActionCapability`) into `ActionHandlerSpec`/`CapabilitySlot`/`ApplicabilitySlot`/`ActionParam`. Config+ontology+`action-ws` protocol all map to OGAR types: arago `ModelFilter{Var,Mode,Value}`→`StateGuard`; `Capability.Name`→`predicate`; `resultParameters`→output sig; `action-ws` `submitAction→ack→sendActionResult` ⟷ `ActionInvocation` `Pending→Committed` (`commit_via` is the gate). **B2 protocol core SHIPPED + spec-faithful** (`action_ws`: all 6 `action-ws.yaml` message types — submitAction/sendActionResult/acknowledged/negativeAcknowledged/configChanged/error — + `submit_to_invocation`/`bind_parameters`/`invocation_to_result` (result=JSON string ≤1 MiB per spec) + connection consts (`ACTION_WS_PATH`, `auth_subprotocol`, `validate_id`); socket-free, `full_action_ws_roundtrip` proven; harvested from the HIRO 7.0 dev-portal specs §2a). **Reactive dispatch + B1 native executor SHIPPED**: `action_ws::handle_submit` (validate→ack/nack→bind→execute→result) over the `CapabilityExecutor` trait (the B1 seam); `ogar-action-handler::NativeCommandExecutor` runs `ExecuteCommand` for real (`full_dispatch_runs_a_real_command` — OGAR runs a command end-to-end). Remaining for a live drop-in: B2-transport (WS loop), B2-lift (REST registration parse), SSH/REST executor targets; gated on `PROBE‑OGAR‑ACTIONHANDLER‑RUN` | G (contract+protocol+native exec) / H (live socket) | CODED | `ogar-from-schema/src/{do_arm,action_ws}.rs`, `ogar-action-handler/`, `docs/ARAGO-ACTIONHANDLER-PARITY.md` | D‑HIRO‑DO, D‑MARS‑CLASSID |
| D‑ACTIONHANDLER‑UPLINK | The hard gate wired to the OGAR executor (cross‑repo seam): rs‑graph‑llm `graph-flow-action-ogar::GatedOgarHandler` wraps an OGAR `CapabilityExecutor` as a `graph-flow-action::ActionHandler`, so `dispatch_via`'s cold floor (`commit_via`: def‑match → RBAC `ClassRbac` → `StateGuard` → MUL) lands **before** the executor's `handle`. Structural proof the contract lands first: `take_result()`/`run_gated` returns `None` whenever the gate refused — unauthorized actor → `Denied` (executor never runs), MUL `Block` → `Escalated` (executor never runs); only the authorized path reaches `NativeCommandExecutor` and runs the real command (3 tests). Dependency hygiene held: `graph-flow-action` stays contract‑only (`I‑ACTIONHANDLER‑IS‑KGV‑NOT‑CHOKEPOINT`); `ogar-from-schema` carries no `lance-graph` dep — the two sides meet only at this crate's API (one `lance-graph-contract`). rs‑graph‑llm pinned to toolchain 1.95.0 to match the AdaWorldAPI stack | G | CODED | `rs-graph-llm/graph-flow-action-ogar/src/lib.rs` | D‑ACTIONHANDLER‑PARITY |
| D‑ACTIONHANDLER‑B2LIFT | REST registration **instance lift** (B2-lift) — turns a deployed handler's `GET /capabilities` JSON (`MapOfCapabilities`) into the concrete signatures the *schema* half can't supply: `registration::{RegisteredCapability,ModelFilter}` (typed DTOs, `Deserialize` behind the `serde` feature) + the pure lift `lift_registration → ConcreteCapability` (concrete `ActionParam[]` with `(name,mandatory,default)`) and `model_filter_to_guard` (arago `ModelFilter{Var,Mode,Value}`→`KausalSpec::StateGuard`, field‑for‑field). Producer stays parser‑free; the runtime `ogar-action-handler::parse_capabilities` does the `serde_json` read (producer‑defines‑types / runtime‑does‑I/O split). Proven end‑to‑end: `rest_registration_lifts_binds_and_runs` (real JSON → lift → `bind_parameters` → `NativeCommandExecutor` runs the command). Remaining: the `GET /applicabilities` `MapOfApplicabilities` envelope read (the `ModelFilter→StateGuard` lift is done). **CORRECTION (canon‑pass 2026‑06‑24): applicabilities envelope now SHIPPED** — `registration::{RegisteredApplicability,lift_applicabilities}` + `ogar-action-handler::parse_applicabilities` lift a real `GET /applicabilities` JSON body into per‑handler `StateGuard` sets (`rest_applicabilities_lift_to_per_handler_guards`); inner filter‑list field name alias‑flexible (`modelFilters`/`model`/`filters`) pending a live response. Supersedes the "Remaining" note | G | CODED | `ogar-from-schema/src/registration.rs`, `ogar-action-handler/src/lib.rs` | D‑ACTIONHANDLER‑PARITY |
| D‑ACTIONHANDLER‑TRANSPORT | The live action daemon (B2-transport), **transport-agnostic** by construction: rs‑graph‑llm `graph-flow-action-ogar::daemon::Daemon::react` turns one inbound `action-ws` JSON frame into the outbound frames it warrants (ack + `sendActionResult`, or nack), running the hard gate (`run_gated`) + executor in between — pure, no I/O. A `Transport` trait (`recv`/`send`) is the swappable edge; `Daemon::serve` is the loop; both the WebSocket and a future Kafka edge share it verbatim (HIRO distributes actions over BOTH wires — the wire differs, the dispatch doesn't). The `WsTransport` action-ws edge (`feature = "ws"`, tokio-tungstenite) presents the `token-$TOKEN` subprotocol and is proven by `ws_roundtrip_against_a_mock_server` (engine submitAction → ack → real command → result over a socket). Connection identity is an `Auth` type shaped after OGIT `NTO/Auth/Configuration` (`auth_store` 0x0B01): the principal the transport authenticates as (`accountId`) IS the actor the gate authorizes. Remaining: the Kafka edge (`rdkafka` over the same trait — core ready, needs topic/record shape) and SSH/REST executors | G (core + ws edge) / H (kafka edge) | CODED | `rs-graph-llm/graph-flow-action-ogar/src/daemon.rs` | D‑ACTIONHANDLER‑UPLINK, D‑ACTIONHANDLER‑B2LIFT |
| D‑ACTIONHANDLER‑REST | REST executor target (B1) — the arago HTTP-callout handler shape: rs‑graph‑llm `graph-flow-action-ogar::rest::RestExecutor` (`feature = "rest"`, pure-Rust `ureq`, sync — fits the sync `CapabilityExecutor` trait) POSTs the bound params as a JSON body to a configured endpoint, returns the response `status`+`body` as `resultParameters`. Any completed HTTP response (incl. 4xx/5xx) is `resultParameters`; only a transport failure is an executor `Err` (mirrors arago reporting the callee's response). `Clone` (ureq Agent is Arc-backed) ⟹ composes into `Daemon`/`run_gated` as a gated route. Proven: `posts_params_and_returns_status_and_body` (mock HTTP) + `rest_executor_runs_only_behind_the_gate` (authorized → REST call fires; unauthorized → `Denied`, endpoint never hit). Completes the executor family with native; SSH/WinRM remain | G | CODED | `rs-graph-llm/graph-flow-action-ogar/src/rest.rs` | D‑ACTIONHANDLER‑UPLINK |
| D‑ACTIONHANDLER‑SSH | SSH executor target (B1) — arago's canonical `ExecuteCommand`-over-SSH: `ogar-action-handler::SshExecutor` shells out to the system `ssh` binary (dep-free, like `NativeCommandExecutor` shells to `sh`), non-interactive by construction (`BatchMode=yes`), same `output`/`stderr`/`exitcode` resultParameters shape — the native executor made remote. `build_args` (pure argv construction with `-i`/`-p` + `--` command terminator) and the pre-spawn guards (missing-command / unknown-capability) are tested; end-to-end remote exec needs a live sshd (absent in CI). The two Command-based dep-free executors (native + SSH) live in OGAR; library-based network executors (REST) in rs-graph-llm | G (code) / H (live exec) | CODED | `ogar-action-handler/src/lib.rs` | D‑ACTIONHANDLER‑REST |
| D‑ACTIONHANDLER‑RESOLVER | The grail — **class-late-bound** action dispatch: rs‑graph‑llm `graph-flow-action-ogar::daemon::ResolvingDaemon` holds NO wired classes and NO wired executor. `ClassResolver` resolves the action class from the **target's classid** per action; `ExecutorRegistry` picks the executor from the resolved `RunnerKind` (`RegistryExecutor` adapts it so `run_gated` drives it — runner chosen POST-commit). The three axes (transport / class / executor) all key off the GUID. The production resolver `OgarResolver` is backed by the canonical `actions_for(&[ClassActions], classid)` DO manifest (no new dep — lance-graph-contract already wired); `target_classid` reads a 32-hex node GUID prefix OR a concept name (`canonical_concept_id`); classid→RunnerKind + capability→signature (B2-lift) complete the resolve. THIS is "the key prerenders the node; classid → ClassView" applied to the action arm — a new capability/class/runner is a registry entry, never a daemon change. Proven: one `ExecuteCommand`, `mars_machine`→native runs the real command / `mars_resource`→REST, zero daemon change, gate still rules; `ogar_resolver_drives_the_grail_via_actions_for` + GUID-prefix test. Shared `dispatch_action` core (static `Daemon` funnels through it too, behavior-preserving) | G | CODED | `rs-graph-llm/graph-flow-action-ogar/src/daemon.rs` | D‑ACTIONHANDLER‑TRANSPORT, D‑ACTIONHANDLER‑B2LIFT |
| D‑OSM | `ogar-from-osm-pbf` — Node/Way/Relation; quadkey NiblePath from resolved geometry | H | IDEA | (queued) | D‑VOCAB, `[per rt]` D‑OSM‑3 |
| D‑PATTERN | `ogar-pattern` — recognition library + confidence (FMA‑D/FIBO/SKR/PROV‑O) | H | IDEA | (queued) | D‑TTL |
| D‑ACTION | `ogar-actionable` — lifecycle → `ActionDef`/`KausalSpec` | H | IDEA | (queued) | D‑PATTERN |
| D‑NSM | 4096‑dim Deep‑NSM encoder (Wierzbicka primes, `NUM_PRIMES=63`) calibration | H | IDEA | RDF‑OWL §4.10 | D‑CAM, `[per rt]` |
| D‑RECIPE‑BITMASK | OGAR = Open Graph **Active Record**: the canonical recipe IS the AR lifecycle protocol; a class stores it once + a per‑class override **bitmask** (set=override, clear=inherited default, fall‑through per the zero‑fallback ladder) + the genuine deltas → thins the "impossible 15%" behavioural leftover toward ~7% for best‑shaped (AR‑canonical) consumers. Guards: zero per‑class payload (recipe IS the spec); computes: shape + `Depends.paths` delta. Guards: bitmask=register not VSA (`I‑VSA‑IDENTITIES` T0); redundant = content‑hash‑equal‑to‑default (lossless‑DO §1); slots RESERVE‑DON'T‑RECLAIM (`I‑LEGACY‑API‑FEATURE‑GATED`). Odoo (Python, UPPER bound) RUN: guard arm 47→1 full‑collapse, compute arm 101 distinct of 141 resolved, **45.7% collapse / 54.3% leftover** — REFUTES "Odoo→7%", CONFIRMS the Rails‑AR scoping | H | EPIPHANY | E‑RECIPE‑BITMASK · F15 · `odoo-rs tests/recipe_redundancy_probe.rs` | D‑VOCAB, D‑HIRO‑DO, D‑ACTION |
| D‑RECIPE‑BITMASK‑CHAIN | the **inheritance axis** of the recipe‑bitmask: a derived ClassView is built by chaining its base `LazyLock<ClassView>` constants + its own delta (`classid → ClassView` compositional; chain = MRO; #533 `resolve_overrides` = order). Fixes two things — "out‑of‑slice" dissolves (the base is a registry constant, not a slice dep) and "redundant = content‑hash‑equal" becomes **referential identity** (the inherited part IS the shared cached constant, pointer‑identical, no copy to drift). Orthogonal to the 3×4 carving (value/registry‑side vs address/centroid‑side) → **3×4 stands, no re‑carve**. Chain‑order falsifier = F1; acyclic (DAG, topo order). MEASURED (full manifest, 388 classes / 166 edges / 3328 methods): naive 4215 vs chained 3328 → **21.0% collapse / 22.7% behavioural**; STACKS with the within‑class 54.3%; lower bound (shallow mixin harvest) | G (measured) / H (LazyLock impl) | EPIPHANY | E‑RECIPE‑BITMASK‑CHAIN · F16 · `odoo-rs tests/recipe_chaining_collapse.rs` | D‑RECIPE‑BITMASK, D‑VOCAB, F1 |
| D‑FUNCTION‑CATALOG | a consumer function = `(verb, criteria)`: the verbs (filter=query engine · project=compute/recompute DAG · map=deterministic lookup · reduce=semirings) are SHIPPED/`[G]`; the "catalog" is of CRITERIA (selection-condition + params, grounded on a domain ontology), EXTRACTED not authored, keyed by canonical concept id (E‑RECIPE‑LABEL‑DTO). DTO-guided landing zone = a criteria DTO selecting+parameterizing a shipped verb; lands on `lance-graph-contract::action` (actions_for/OgarResolver) + `ogar-render-askama` artifact_kinds + `ogar-vocab`. **map ≠ CAM‑PQ** (Kontenerkennung = relational rule + precedent; Test‑0 register, not ANN); **CRUD = generic lifecycle + extracted criteria**, not hand-rolled. Gate: extracted criterion must round‑trip | H | EPIPHANY | E‑FUNCTION‑CATALOG · F17 | D‑VOCAB, D‑ACTIONHANDLER‑RESOLVER, D‑RECIPE‑BITMASK |
| D‑ACCIDENTAL‑IMPERATIVE | the hand-rolled residue = **accidentally-imperative** (AR verbs on AR targets, no declarative home — recoverable; Odoo `@api.depends` vs Rails `before_save{lines.sum}` proves it is source-*expressiveness* not logic complexity) ∪ **essentially-foreign** (the only true escape). The body pass **TRIAGES** to `(target classid, verb-class, order-signature)` — NOT decompile; residue becomes legible; lands at a coarse catalog tier + point-to-body (lossless-DO §1). "random orders" = the **round-trip-order-free** recover/preserve gate. Three tiers: clean `(verb,criteria)` · coarse `(target,verb-class)` · foreign | H | EPIPHANY | E‑ACCIDENTAL‑IMPERATIVE · F17 · ruff writes/calls capture SHIPPED 2026‑06‑30 (AdaWorldAPI/ruff#38: `Function.writes`/`calls` + `writes_field`/`calls` predicates + `ruff_ruby_spo` body walker) → F17 RUNNABLE, body‑triage probe is the next deliverable; ratio still unmeasured | D‑FUNCTION‑CATALOG, D‑HIRO‑DO |

### 2.9 Domain instances (universality witnesses)

| ID | Shape | Grade | Status | Home |
|---|---|:--:|:--:|---|
| D‑DOM | 6 instances: chess / OpenProject / Elixir‑HIRO / Odoo‑ERP / HIPAA / OSM | G | ADR/doc | `DOMAIN-INSTANCES.md` (#27,#41,#42) |
| D‑PII | label‑free contract IS the PII guarantee (HHTL leaf‑rename at Adapter) | G | doc | `HEALTHCARE-TRANSCODING.md §4` |
| D‑LITMUS‑FMA | FMA bones‑rendering = compile‑time HHTL litmus (~75K static classes) | H | doc | `RDF-OWL-ALIGNMENT.md §6` |
| D‑LITMUS‑GEO | OSM = geographic litmus; "Femur is_a LongBone AND Marienplatz is_in Munich" sub‑µs | H | doc | `DOMAIN-INSTANCES.md §2.6` |

### 2.10 The GUID canon pins (operator, 2026‑06‑10) — canon = `CLAUDE.md` P0; this table indexes, never re‑derives

| ID | Shape | Grade | Status | Home | Deps |
|---|---|:--:|:--:|---|---|
| **D‑CANON‑GUID** | the canonical identity is **HEX‑counted — it IS the GUID**: dash‑groups `8‑4‑4‑4‑12` = `classid‑HEEL‑HIP‑TWIG‑[basin·leaf(6)+identity(6)]`; 1 hex = 1 nibble = 1 tree level; self‑describing at sight; wrappers (NodeGuid #480) audited against it group‑by‑group, never the reverse | G (operator‑pinned) | **CANON** | `CLAUDE.md` P0; INTEGRATION‑MAP L0 | — |
| D‑KEYKV | **the GUID is the key of key‑value**: node = `key(128) + value(3968)` = 4096 bits; the key routes/resolves/compares/scopes/names with **zero value decode**; Lance compresses the value freely — compression never costs addressability | G (operator‑pinned) | CANON | `CLAUDE.md` P0 | D‑CANON‑GUID |
| D‑3X4 | **3 tiers × 4 nibbles, uniform**; `tier = nibble >> 2` (shift, never branch/divide); tiers u16‑aligned; dashes = tiers; RFC 9562 = **wrapper concern** (v8‑native was pinned then REVERSED — the 4/3/3 carving broke Morton stride uniformity); **standing watch** vs 4×3 with falsifiable flip condition | G (pinned) + watch | CANON | `CLAUDE.md` P0 + standing‑watch §; INTEGRATION‑MAP §9.10–11 | D‑CANON‑GUID |
| D‑TILE256 | each tier's 64k = a **256×256 centroid tile** (two byte‑axes, nibble‑interleaved); path = 6 bytes = **CAM‑PQ 6×256** → path distance = 3 tier‑LUT lookups O(1); rigor condition: codebooks built as **4⁴ hierarchies** so nibble prefixes = centroid ancestry | H + named test (PROBE‑CODEBOOK‑44 / F11‑adjacent) | EPIPHANY→CANON‑pin | `CLAUDE.md` P0 | D‑CANON‑GUID, D‑CAM, D‑PAL256 |
| D‑PREFIXBOOK | **codebook scoping = the class routing prefix** — longest‑prefix binding on the key's own hierarchy; per‑class 256⁶ semantic spaces for free; axis binding (x/y vs PQ pairs) is a class‑record property; codebooks mint with the class (Phase B shelf, D‑AMORT) | G (mechanism) / H (mint) | CANON‑pin | `CLAUDE.md` P0 | D‑TILE256, D‑AMORT |
| D‑PHASE | **perturbation = deterministic phase**: (exponent, location, phase, magnitude) — three terms derive from the KEY; **only the magnitude envelope is stored**; lossless for synthesis by construction; analysis remainder escalates per the quorum certificate; **D‑QUANTGATE picks the generator** (coprime‑integer `CurveRuler` in quantized layers); doubles as the anti‑moiré dither; cost scales with magnitude smoothness, not bandwidth | H + probes (PHASE‑1, PERT‑RHO, PYR‑1) | CANON‑pin | `CLAUDE.md` P0; ndarray `guid-prefix-shape-routing.md` §4 | D‑MANTISSA, D‑QUANTGATE, D‑IMMAT |
| D‑WHP | **bipolar phase = Walsh‑Hadamard on VSA**: signed (±1) phase makes the cascade the WH transform of the address tree; **sign = XOR (`vsa_bind`), magnitude = `vsa_bundle`** — the TWO‑ALGEBRA rule (raw‑XOR on magnitudes breaks Markov, I‑SUBSTRATE‑MARKOV); superposition + unbind‑by‑role‑key; N ≤ √d/4 IS the substrate's uncertainty principle; roundtrip bit‑exact; "quantum‑like" = the bundling algebra, NOT measurement randomness | H + probes (WHP‑1..4) | CANON‑pin | `CLAUDE.md` P0; ndarray §4b; lance‑graph §7 + E‑WHP‑BIPOLAR‑1 | D‑PHASE, D‑MANTISSA |
| D‑DELEG‑INHERIT (né D‑OTP‑INHERIT) | DO‑axis behavior resolution = **prototype‑chain delegation** (Self, Lieberman 1986) carried ON the supervision topology — **NOT an OTP behavior** (supervisors restart, they don't dispatch); diamonds have NO prefix encoding (tree relation) → ordered `mixins` traversal is the absent second mechanism; Odoo = C3 over `LastOrderedSet` install order, not naive source C3 | H + gate F1 (chain + diamond fixture) | EPIPHANY | INTEGRATION‑MAP §3 + F1 | D‑CANON‑GUID, `[per rt]` state_machine |
| D‑IDENTITY‑PIN | lance‑graph `NodeGuid` (#480) = the **wrapper carving** of the canon — groups 1–2 + 24‑bit `local` already match exactly; Phase B audits groups 3–4 yielding all eight nibbles to HIP/TWIG; `IDENTITY_LAYOUT_VERSION` stamp = the version gate | G (wrapper) / H (group 3–4 audit) | CODED (wrapper) | lance‑graph `identity.rs`; INTEGRATION‑MAP S1 | D‑CANON‑GUID |
| **D‑VALFACET** | **value‑slab homogeneous closure = the contained 16‑byte facet** `facet_classid(4)+helix‑place(6)+CAM‑PQ(6)` — identity⊥search⊥schema; the value‑side restatement of canon (key path = 6‑byte CAM‑PQ **address** D‑TILE256; this facet's CAM‑PQ = the **content/search** code; helix = place/residue D‑PHASE); lance‑graph value‑tenant harvest CONFIRMS — 9/10 slab tenants don't homogenize (KEEP/DEFER, only HelixResidue matches) + the slab↔parallel‑MailboxSoA two‑world seam | G (harvest facts) / H (facet closure, gated F‑1+F‑code) | EPIPHANY | `.claude/board/EPIPHANIES.md` E‑VALUE‑SLAB‑FACET; lance‑graph `soa-value-tenant-migration-v1-harvest.md` | D‑TILE256, D‑PHASE, D‑KEYKV, D‑CANON‑GUID |
| **D‑ENVPARSE** | **classid defines V2/V3 + value‑schema + edge‑codec per file+consumer; ONE reusable envelope parser reads `classid → registry → parse`** (operator `0x1007` — a leading‑`1` generation marker before the domain self‑identifies new‑gen envelopes; versioning in the classid/schema‑pointer, never a GUID‑tail nibble) — no per‑file format byte; the single classid‑late‑bound read path that also reconciles the two‑world seam; the registry entry gains a **`tail_variant` (V2/V3)** axis beside `ReadMode {value_schema, edge_codec}`; same shape as D‑ACTIONHANDLER‑RESOLVER (renderer over the classid keyspace) | G (pieces: classid_read_mode / ClassView / new_v2 / cascade_key‑V3 / node_rows_from_le_bytes) / H (the composed parser, to‑wire) | EPIPHANY→to‑wire | `.claude/board/EPIPHANIES.md` E‑CLASSID‑ENVELOPE‑PARSER; lance‑graph `canonical_node.rs` | D‑VALFACET, D‑IDENTITY‑PIN, D‑KEYKV, D‑TILE256 |

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

**Do‑not‑build** (`S`): D‑OLED / D‑EXCITON / D‑LOSSCHAN — catalog only
(D‑EXCITON `[H]→[S]` reverted by the 5+3 review; D‑LOSSCHAN = its one
untested survivor).

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
| **D‑DELTA (delta frames = version‑diff)** | `cycle()->u32` stamp; *"a Lance version IS a coherent LE envelope"* (soa_envelope L16); `last_active_cycle [u32;N]` = per‑row same‑cycle write guard; `DatasetVersion(v)→(v+1)` | a Lance version = a **self‑contained snapshot** ("frame"); the per‑row cycle stamp = a **RECENCY stamp** (WHEN last changed — watermark‑filter, never diff‑reconstruct; label corrected 2026‑06‑10) → **D‑DELTA promotes [H]→[G]** on the mechanism |
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

## 4.2 Test mapping — the validation suite (jc *proofs* × hpc *empirics*)

> **The anti‑dilution gate at the validation layer:** every `[H]` claim
> here names the test that would confirm or falsify it. Without a named
> test, an epiphany is assertion — *that's* how a map collapses. Two
> runtime‑owned pillar sets bind the claims to runnable validation:
>
> - **`jc` — the proofs** (`lance-graph/crates/jc`, "Jirak‑Cartan:
>   five‑pillar [now 11] proof‑in‑code"). Run: `cargo run -p jc --example
>   prove_it`; each yields a `PillarResult`. Pillars used below: **P3
>   φ‑Weyl** (*optimal collocation without aliasing*), **P5 Jirak 2016**
>   (*bounded noise floor under the dependence model*), **P5b Pearl 2³**
>   (*three‑plane Index vs CAM‑PQ‑bundled regime*), **P7
>   Köstenberger‑Stark** (*Hadamard‑space concentration*), **P9/9b
>   EWA‑sandwich** (*Σ push‑forward; P9b explicitly certifies
>   `ndarray::hpc::splat3d`*), **P10 Pflug‑Pichler** (*CAM‑PQ tree
>   quantization preserves FreeEnergy within Lε*).
> - **`ndarray::hpc` — the empirics** (`ndarray/src/hpc/`, behind the
>   `hpc`/`hpc-extras` feature). Modules used: `fft` (spectral),
>   `lapack` (Cholesky 3×3 SPD), `quantized` (Int8/BF16 GEMM, the
>   palette/CAM regime), `cascade` (the Morton cascade), `plane` (the
>   Base17 / 16Kbit planes), `fingerprint` (CAM), `soa` (the envelope),
>   `blas_level2/3` (neighborhood), `simd_dispatch` (the three‑backend
>   parity).
>
> **Division of labour:** OGAR *maps* claim → test (this table); the
> runtime session *runs* the pillars (jc + ndarray::hpc are `[per rt]`).
> A claim is **validated** when its jc pillar proves the bound **and** its
> hpc primitive measures within it.

| Claim (D‑*) | jc pillar (proof) | `ndarray::hpc` (empiric) | measures | pass criterion |
|---|---|---|---|---|
| D‑MORTON / D‑CASCADE / D‑XOR2 | P1 (substrate‑Markov) | `cascade` + `hhtl.rs::NiblePath` | `parent∘child = id`; `is_ancestor` prefix‑exact | round‑trip exact — **already CODED** (q4 audit) |
| D‑RSTAR / D‑PROBEFREE | **P5 Jirak** | `cascade` + `reductions` | measured error at `r* = ⌈log₄(C/τ)⌉` | error `≤ τ` (inclusive boundary, PR #46) |
| D‑PAL256 / D‑CAM / D‑RHO | **P10 Pflug** | `quantized` + `fingerprint` | ρ(palette256‑reconstructed distance vs cosine) | **ρ ≥ 0.99** (anchor 0.9973) |
| D‑THETA / D‑RHOENV | **P5b Pearl 2³** + **P7** | `quantized` (θ sweep) | ρ(θ) across θ ∈ [1.45, 1.6] | **ρ ≥ 0.93** band‑wide; ≈ 0.9973 at θ ≈ π/2 |
| **D‑MANTISSA / D‑MOIRE / D‑NOCOLLAPSE** (golden anti‑moiré) | **P3 φ‑Weyl** | **`fft`** (spectrum of the golden tile vs the LOD lattice) | star‑discrepancy + FFT peak amplitude at the lattice frequency | discrepancy < Jirak bound **and** no FFT peak above noise at the lattice freq → **this is the "spectral validation" the §2/#47 Codex caveat demanded** |
| **D‑BGZ17** (Base17 coprime) | **P3 φ‑Weyl** (discrete) + **P10** | `plane` (Base17) + `fft` | beat spectrum of the 16×17 lattice; ρ(SpoBase17 vs full planes) | dominant beat at **period 272** (lowest freq); **ρ ≥ 0.965** (the TWIG anchor) |
| **D‑QUANTGATE** (quant kills continuous aperiodicity) | **P3 (continuous) vs P10 (quantized)** — the *contrast* | `fft` pre‑ vs post‑quantization | golden‑tile spectral aperiodicity, continuous vs lattice‑rounded | post‑quant peak **appears** for the continuous golden (it fails) **while** Base17 retains aperiodicity → confirms the **layered rule** |
| D‑SPLAT Σ‑sandwich / D‑NEIGH | **P9 / 9b EWA‑sandwich** (certifies `hpc::splat3d`) | `lapack` (Cholesky 3×3) + `blas_level2/3` | Σ stays SPD through `J·W·Σ·Wᵀ·Jᵀ`; neighborhood push‑forward | Cholesky succeeds (SPD preserved); within the P9b Lipschitz bound |
| D‑COLUMNAR / D‑LANCE / D‑IMMAT | P1 (structural) | `soa` + `SoaEnvelope::verify_layout()` | stride/overlap/version conformance; zero‑copy view exactness | `verify_layout()` Ok; `row_le`/`column_le` byte‑exact |
| **all CODED primitives** (correctness floor) | — | **`simd_dispatch`** (the W1c contract) | AVX‑512 vs NEON vs scalar parity | **identical within 1 ULP** across all three backends |
| D‑META64 (revised) | — | read `MailboxSoA` column layout | `edges[CausalEdge64]` vs `meta[MetaWord]` separateness | **confirmed separate** (§4.1) → reconcile the 48/16 bit‑budget to `MetaWord` |
| D‑EXCITON | — *(external)* | — *(external OLED physics)* | — | **`[S]` (reverted from `[H]`)** — the aperiodic reading is literature‑**un**supported (no source calls the 1:3 aperiodic; it is rational SU(2)) and rested on a T1‑vs‑binding‑energy mis‑label; no substrate test. Catalog only; survivor = `D‑LOSSCHAN`. |

**Audit conclusions (the "double‑check" the operator asked for):**

1. **All pillars cited are real.** `crates/jc/src/lib.rs` (11 pillars +
   `PillarResult`); `ndarray/src/hpc/mod.rs` (`fft`/`lapack`/`quantized`/
   `cascade`/`plane`/`fingerprint`/`soa`/`simd_dispatch`). Not invented.
2. **The jc↔hpc bridge is in code, not asserted.** jc **P9b** doc‑comment:
   *"certifies J·W·Σ·Wᵀ·Jᵀ for `ndarray::hpc::splat3d`."* The proof layer
   already names the empiric layer.
3. **The Codex anti‑moiré caveat is now a *named test*, not an open
   `[H]`.** "Requires spectral validation" = **run jc P3 (φ‑Weyl) +
   `hpc::fft`.** If they pass, D‑MOIRE/D‑MANTISSA promote `[H]→[G]`; if the
   FFT peak appears, they're falsified — either outcome ends the dilution.
4. **D‑QUANTGATE gets a falsifiable contrast test** (P3 continuous vs P10
   quantized; `fft` pre/post). Its `[G]` is the *principle*; this test is
   the *demonstration*.
5. **`[per runtime session]` numbers become reproducible.** ρ = 0.9973
   (HIP) and ρ = 0.965 (TWIG) and θ ∈ [1.45, 1.6] stop being *cited* and
   become *re‑measured* by `hpc::quantized` + P10 — so the map can't drift
   from the runtime's actual numbers.
6. **D‑EXCITON was reverted `[H]→[S]` by the 5+3 agent review (2026‑06‑09).**
   The `[H]` "same coprime aperiodicity as D‑BGZ17" rested on a **category
   error** (a scalar ratio / two energy levels have no period to be aperiodic
   about) **+ a T1‑vs‑binding‑energy mis‑label** (the "1.5 eV" was the T1
   excited‑state energy, not E_b ≈0.3–0.6 eV; corrected, the 1:3‑energy
   coincidence does not survive). The `[S]→[H]` jump had itself violated §6.3
   (promotion needs a *measurement*, not a mechanism story) — so reverting
   **restores** the discipline. Entry **kept** (append‑only); the two
   `[G]`‑real legs (rational 1:3; ~100% IQE harvest physics) preserved,
   reframes A/B logged rejected, the one untested survivor split out as
   `D‑LOSSCHAN` (`[S]`). The apparatus (5 research + 3 brutal‑review agents)
   is committed at `.claude/agents/`.

**No claim *collapses*** (append‑only); but the 5+3 review **reverted** one
overclaim — D‑EXCITON `[H]→[S]` (this §, #6) — and split out its one survivor
(`D‑LOSSCHAN`), joining the prior fixes (D‑MOIRE absolutism — #47; D‑META64 —
§4.1). The rest are either CODED, ADR‑pinned, or `[H]`‑with‑a‑named‑test.

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
- **The test‑mapping pillars (§4.2):** `jc` — lance‑graph
  `crates/jc/src/lib.rs` ("Jirak‑Cartan five‑pillar [11] proof‑in‑code";
  `cargo run -p jc --example prove_it`; pillars P1/P3‑φ‑Weyl/P5‑Jirak/
  P5b‑Pearl/P7/P9‑9b‑EWA‑sandwich/P10‑Pflug/P11). `ndarray::hpc` —
  `ndarray/src/hpc/` (`fft`/`lapack`/`quantized`/`cascade`/`plane`/
  `fingerprint`/`soa`/`blas_level2-3`/`simd_dispatch`; `hpc`/`hpc-extras`
  feature). Both `[per runtime session]`‑owned; this map specifies the
  claim→pillar binding, the runtime session executes.
- **The tiling / addressing / aperiodicity papers (D‑MONOTILE, D‑CASCADE,
  D‑BGZ17 groundings; operator‑supplied 2026‑06‑08):**
  - **arXiv 2509.12216** — Kaplan, *The Path to Aperiodic Monotiles* (the
    "hat"; a single shape that tiles only non‑periodically → anti‑moiré
    **by tiling theorem**; D‑MONOTILE).
  - **arXiv 2309.15199** — Walker, *Generalised 3D Morton and Hilbert
    Orderings* (bit‑interleave Morton; non‑power‑of‑2 + 3D via octant
    recursion; the `6×4×4` example; Hilbert alt for locality — grounds
    D‑MORTON/D‑CASCADE/D‑BGZ17 addressing).
  - **arXiv 2203.09323** — Richter, *Covering Rectangles by Few Monotonous
    Polyominoes* (ribbon‑tile covering; the `⌈(2/3)(m+n−√(m²−mn+n²))⌉`
    cardinality with the **Eisenstein / hexagonal norm** `m²−mn+n²` — ties
    the square cover to the hat's hexagonal substrate).
- **The OLED‑exciton papers (D‑EXCITON revert / D‑LOSSCHAN; 5+3 review
  2026‑06‑09):** the 1:3 singlet:triplet ratio is rational SU(2) spin
  statistics, *not* aperiodic — the `[H]` "coprime aperiodicity" reading was
  reverted to `[S]`.
  - **PMC4614446** — Takahashi et al., *Sci. Rep.* 5:15533 (2015):
    bias‑dependent S:T formation ratio (real but material‑specific).
  - **arXiv 2311.03927** — triplet‑exciton binding energies in organic
    semiconductors are sub‑eV (≈0.3–0.6 eV) — refutes the "~1.5 eV E_b" (that
    figure is the T1 excited‑state energy, not a binding energy).
  - **Baldo/Adachi/Forrest** (~100% internal phosphorescence, *J. Appl.
    Phys.* 90:5048, 2001) + **Uoyama/Adachi** (TADF, *Nature* 492:234, 2012)
    — the harvest‑the‑dark physics (`[G]`‑real, ≠ the substrate link).

---

- **D-APPCLASS (classid = APP(hi u16) ‖ class(lo u16); 2026-06-22; [H]):**
  the GUID's `classid` is u32 (8 hex) but only the low u16 (`0xDDCC`
  domain|concept) was ever used; the high u16 was reserved-zero, NOT SoA
  versioning (`ENVELOPE_LAYOUT_VERSION: u8 = 2`,
  `lance-graph-contract/src/soa_envelope.rs:54`). Claimed: **hi u16 =
  APP / codebook-namespace + render prefix; lo u16 = shared canonical
  concept.** The two halves carry orthogonal facts — lo = WHAT it is
  (RBAC grant + ontology + cross-app identity, shared), hi = WHOSE
  rendering (app `ClassView` / Askama template / SoA layout, per-app).
  Additive (every existing id is `0x0000_DDCC`); zero
  `ENVELOPE_LAYOUT_VERSION` cost (classid keeps fixed key offset 0..4);
  RESERVE-DON'T-RECLAIM holds. Resolves the operator's "codebook per
  project avoids radix-trie codebook limits" — each app prefix roots its
  own centroid-codebook hierarchy + template set. Spec:
  `docs/APP-CLASS-CODEBOOK-LAYOUT.md`. Migration (Odoo/WoA/SMB/q2):
  `docs/APP-CODEBOOK-MIGRATION-PLAN.md`. Medcare worked example:
  patient = `0x0005_0901` (lo `0x0901` shared patient grant+ontology, hi
  `0x0005` Medcare clinical template). Gated on the 5+3 codebook pass;
  nothing minted yet. `[H]` pending the pass + a render-path probe.
- **D-KV-RENDER (rendering + RAG are key-value egress; 2026-06-22; [H]):**
  the operator's stated goal — strings / text / media / online sources
  rendered via key-value so **no serialization exists in the hot path**
  (the Firewall, ADR-022/023). The registry axiom (I-K0: label=KEY,
  meaning=VALUE) applied to *content*: every renderable field is a key
  into a typed content store (string dictionary / text column / media
  bytes / URI registry), resolved by zero-copy columnar/dictionary
  lookup — never inlined as a serde blob. A rendered object is a **tree
  of keys**: classid → Askama template (hi u16), each field → content
  key. **Two membranes, one rule:** UI render and the RAG-to-LLM path
  (rs-graph-llm `graph-flow` over lance-graph retrieval) both move keys
  in the hot path and materialize content **only at the membrane,
  exactly once** — RAG context is a `Vec<key>` (pointer set), content
  lands in the LLM prompt at egress (the MarkovBarrier / "boundary
  parsed once" pattern). Litmus: any `serde::Deserialize` /
  `serde_json::from_*` on a render or retrieval hot path = a blob that
  entered too early; make it a key. Build-time codegen that *generates*
  the ClassView from a manifest is fine ("compile types", not hot-path
  serde). Detail: `docs/APP-CLASS-CODEBOOK-LAYOUT.md` §3.5–3.7. `[H]`
  pending a hot-path no-serde probe across one render + one RAG path.

- **D-OSINT-APPID-NOT-CONCEPT (the OSINT low byte is appid space,
  domain-wise; 2026-07-02; [G], operator ruling):** PR #145's two OSINT
  codebook mints were hallucinations — "OSINT Person was a
  hallucination" (operator, verbatim). The corrected semantics: `0x07`
  is the OSINT **domain**; its low byte is allocated **domain-wise as
  an APPID**, not as concept vocabulary. `0x0700` = the domain itself
  (low byte `00` = domain-wide); `0x0701` = **OSINT-for-q2** (q2 is
  appid `0x01`, the consumer); V3 form `0x1000_0701`. Consequently the
  OSINT domain contributes **zero vocabulary rows** — `osint_system` /
  `osint_person` removed from `CODEBOOK` / `class_ids::ALL` / the
  resolver / `all_promoted_classes` / the Class builders (this pass).
  Class *content* (AIRO/VAIR system card, McClelland/Rubicon person
  lens) is consumer-side — q2 `osint_classview.rs` — never OGAR
  vocabulary. Side effect: codebook count returns 67 → 65, balancing
  the lance-graph mirror COUNT_FUSE with zero mirror changes, and
  dissolving lance-graph `ISS-OSINT-SYSTEM-ROOT-SLOT-VIOLATION`
  (neither of its Options A/B — the ruling is sharper: no concept rows
  at all). Adjacent confirmations in the same ruling: FMA anatomy = own
  domain (`0x0A`), adjacent to but separate from Health (`0x09`),
  consumed by q2; CPIC likewise its own Genetics domain (`0x0E`) under
  q2. Do NOT re-mint OSINT concept rows; the codebook section carries
  the guard note.

---

- **D-CLASSID-CANON-HIGH-FLIP (classid half-order flip — canon concept
  now HIGH, APP/render prefix now LOW; 2026-07-02; [G], operator
  triggered):** the operator's `0x07:01::1000` mnemonic ("domain 0x07,
  appid 0x01=q2, custom marker 0x1000" — human-readable as
  `domain:appid::marker`) exposed that the working composed-classid
  order had the APP/render prefix in the **high** u16 and the canonical
  concept in the **low** u16 — backwards from how the mnemonic reads
  (domain/concept first, appid second). **Ruling:** flip the composed
  order to `classid : u32 = [hi u16: canon concept][lo u16: APP/render
  prefix]` — the mnemonic's read order becomes the storage order.
  `ogar_vocab::app::{render_classid, app_of, concept_of}` flipped in
  lockstep with lance-graph-contract's `CLASSID_ORDER = CanonHigh` (PR
  #628 there): `app_of` now reads `classid as u16`; `concept_of` now
  reads `classid >> 16`. **APP_PREFIX *values* are unchanged** (`0x0000`
  Core, `0x0001` OpenProject, `0x0002` Odoo, `0x0003` WoA, `0x0004` SMB,
  `0x0005` Healthcare, `0x0007` Redmine) — only their **position**
  moves (hi → lo). V3 marker forms move in lockstep: `0x1000_0700` →
  `0x0701_1000`; FMA `0x1000_0A01` → `0x0A01_1000`; CPIC `0x1000_0E00` →
  `0x0E01_1000` (appid normalized `:00`→`:01` in the same pass). Auth
  RBAC literals: `0x0000_0B01`→`0x0B01_0000`,
  `0x0000_0B02`→`0x0B02_0000`, `0x0000_0B03`→`0x0B03_0000`,
  `0x0000_0B04`→`0x0B04_0000`. **Legacy stored forms resolve via a
  read-only registry alias** (mint-forward doctrine, RESERVE-DON'T-
  RECLAIM held) — no data is rewritten, and pre-flip docs are annotated
  in place rather than deleted; retirement of the legacy-alias path is
  gated on a corpus proof, not assumed. **This supersedes the order
  stated in D-APPCLASS** (`classid = APP(hi u16) ‖ class(lo u16)`,
  2026-06-22) **and the `0x1000_0701` literal in
  D-OSINT-APPID-NOT-CONCEPT** (2026-07-02, same-day predecessor) — both
  entries stand as written (append-only; do not edit), this entry is
  the correction of record for the half-order going forward. Doc sweep:
  `APP-CLASS-CODEBOOK-LAYOUT.md`, `APP-CODEBOOK-MIGRATION-PLAN.md`,
  `OGAR-CONSUMER-BEST-PRACTICES.md`, `OGAR-TRANSPILE-SUBSTRATE.md`,
  `OGAR-AS-IR.md`, `SURREAL-AST-TRAP-PREFLIGHT.md`,
  `NODEGUID-CANON-AUDIT.md`, `FOUNDRY-ODOO-MARS-LENS.md`,
  `CLASSID-RBAC-KEYSTONE-SPEC.md`, `ODOO-REDMINE-OPENPROJECT-LANDING.md`,
  `PHILOSOPHY.md`/`PHILOSOPHIE.md`, `README.md`/`README.de.md`,
  `integration/AR-OGAR-MAILBOX-INTEGRATION-PLAN.md` §7, and this
  repo's `CLAUDE.md`.

---

- **D-TRUNCATION-DISALLOWED-SOC-REROUTE (truncation-disallowed /
  overflow-as-SoC-reroute doctrine; 2026-07-02; [G], mirrored from
  lance-graph):** names/entries over the 256-slot cascade-tier
  cardinality are NEVER truncated; overflow is rerouted as a
  separation-of-concerns split, never a silent drop and never a
  field-widen. This operationalizes OGAR's own `256-cap-is-a-lint` law
  stated at the top of this file (`scale = the next cascade level,
  never field-widening`) — a bucket/tier that exceeds its 256-slot
  cardinality is a DESIGN smell, and the overflow itself is the reroute
  signal (split the overflowing class into sub-concerns, or escalate to
  the next family/basin), not a minter limitation to "fix" with a bigger
  packer. Doctrine statement: lance-graph
  `.claude/knowledge/ast-as-partof-isa-address.md` ("Truncation is
  DISALLOWED; bucket overflow is a separation-of-concerns REROUTE
  trigger") + lance-graph `.claude/board/EPIPHANIES.md`
  E-BRICK3-RAN-TRUNCATION-DISALLOWED (2026-07-01, operator doctrine:
  "truncations were disallowed / we introduced bucket overflow with
  separation of concerns as a trigger for rerouting"). **Shipped
  implementation** (the reason this is graded `[G]` rather than `[H]`):
  `ruff` `crates/ruff_spo_address/src/soc.rs` —
  `MAX_SIBLINGS_PER_TIER: usize = u8::MAX as usize` (255, the
  byte-cardinality ceiling shared with the per-tier sibling rank),
  `SocVerdict::{Duplication, Conflation, DuplicationAndConflation,
  Counterexample}`, and `law_holds` as the falsifier (`false` iff some
  over-cap class is neither type-collapsible nor
  data⊥behaviour-mixed). Division of labour: `mint_factored`
  (lance-graph's rank-minter, per the brick-3 falsification of the naive
  fixed-width 6-tier packer) handles **addressing precision**;
  overflow→SoC-reroute (`ruff::soc`) handles **structure** — together:
  zero truncation, zero collision, over-cap classes flagged for split
  rather than silently mangled.

- **D-CLASSID-HI-U16-SPELLING (operator confirmation; 2026-07-02; [G],
  recorded in both ledgers same-arc):** the composed classid u32 reads
  **`domain : appid : classview`** — canon hi u16 = `domain byte ++
  appid byte`, custom lo u16 = the ClassView selector — and **"concept"
  NAMES the whole hi u16** while `domain:appid` is its byte spelling.
  Canonical text: lance-graph `.claude/v3/soa_layout/le-contract.md`
  (§ the 4-byte prefix; worked example `0x07:01` = OSINT:q2) +
  `.claude/v3/knowledge/v3-substrate-primer.md` §5 (`[hi u16 CANON
  concept/domain:appid][lo u16 CUSTOM]`). The 2026-07-02 cross-session
  "conflict" between OGAR's `domain:concept-slot` prose and the
  le-contract's `domain:appid` was a PHANTOM — the same second byte
  under two names. **Homonym warning (the root cause):** the word "app"
  appears in BOTH halves with different meanings — the **appid byte**
  (hi half) is the canonical app-concept slot within a domain (shared
  across vendors: `0x0102` = project work item, converged on by
  OpenProject `0x0102_0001` AND Redmine `0x0102_0007`), while the
  **APP render PREFIX** (lo half, OGAR#95 register: `0x0001` =
  OpenProject) is the per-vendor ClassView skin. q2's appid `0x01`
  (inside `0x07:01` OSINT:q2, hi half) and OpenProject's APP_PREFIX
  `0x0001` (lo half) are positionally distinct registers — no collision,
  no guard needed beyond this line. Cite this entry instead of
  re-deriving; a "two ledgers disagree" claim checks the le-contract /
  primer line FIRST.

- **D-OGAR-ODOO-INHERIT-MIXINS (transpile-chain LEG 2; 2026-07-04;
  [G] — CODED + tested):** the middle leg of the operator's transpile
  chain (`ruff *_spo harvest → ogar-from-ruff lift → CompiledClass →
  ClassView × FieldMask → askama render`). ruff PR #40 shipped the input
  end: a frontend-agnostic `ruff_spo_triplet::Model.inherits: Vec<String>`
  populated by the Odoo frontend from `_inherit` (self-reopen self-edge
  excluded upstream). `ogar-from-ruff` previously consumed only
  `sti.inherits_from → Class.parent` (Rails STI) and **dropped**
  `Model.inherits`, so the Odoo is_a linkage never reached the Core.
  **Resolution (this commit):** bump the ruff pin to merged main
  (`61ce2b49`), then `class.mixins.extend(model.inherits)` in
  `lift_model_with_language`. **The multi-parent "decision required" I
  forwarded was already answered by the vocab:** `Class::mixins` doc
  explicitly names `_inherit = 'mixin.thread'`, and `Class::inheritance`
  doc states "Mixins / concerns are a SEPARATE axis … never folded in
  here." So Odoo `_inherit` → `mixins` (the multi-parent `Vec` shelf),
  NOT `parent`/`inheritance` (STI single-parent spine) — no `parent`
  widening, no information loss, no vocab-axis violation. **Consequence
  for LEG 3 (V3/D-VCW-3 render):** the FieldMask compose step must union
  over `parent` ∪ `mixins` when materialising Odoo inherited fields —
  the is_a spine and the mixin shelf are BOTH inheritance surfaces for
  the render; `render_rows` itself stays concept-local (the union is the
  caller's compose-time `FieldMask::inherit` bitwise-or). 48 tests green
  in `ogar-from-ruff` (2 new: `odoo_inherit_lands_on_mixins_not_parent`,
  `empty_inherits_adds_no_mixins`); workspace `cargo check` clean;
  clippy clean. Supersedes the "widen parent vs primary+relation"
  framing in the 2026-07-04 lance-graph broadcast — the vocab's mixins
  axis is the answer.

- **D-OGAR-RENDER-CLASSVIEW-FIELDMASK-METHODS (transpile-chain LEG 3;
  2026-07-04; [G] — CODED + tested):** the render end of the operator's
  chain (`ruff harvest → ogar-from-ruff lift → CompiledClass →
  ClassView × FieldMask → askama render`). `ogar-render-askama` gains
  `render_class_with_methods(class, mask, actions)` — a compile-time
  (askama = the ERB/XSLT analog) transpiler that emits a Rust struct
  whose FIELDS are the `ClassView × FieldMask` projection (the
  `FieldMask` bitmask indexes the ObjectView N3 order — attributes then
  family edges — the exact basis `OgarClassView::render_rows` uses; bit
  `n` set ⇒ n-th field emits) and whose METHODS are the OGAR `ActionDef`
  DO-arm, assembled as a **struct-of-methods constructor** (`impl { new(..)
  ctor + one fn per ActionDef }`). **Operator rulings baked in
  (2026-07-04):** (1) behaviour is Rust methods, NOT SurrealQL DDL — the
  deprecated SurrealQL-AST adapter (`DEFINE EVENT … WHEN … THEN …`
  carrying lifecycle) is not a target; consistent with
  `SURREAL-AST-AS-ADAPTER.md` §0. (2) `on_enter` (the Rubicon state
  mutation) makes a method take `&mut self`; read actions take `&self`.
  New dep: `lance-graph-contract` (for `FieldMask`, branch=main). 6 new
  tests (mask gates fields; ActionDef→methods; no `DEFINE EVENT`/`DEFINE
  TABLE`; dotted-predicate + PascalCase sanitisers); 50 tests green in
  `ogar-render-askama`; workspace `cargo check` clean; `cargo clippy
  -p ogar-render-askama -- -D warnings` clean. End-to-end verified: a
  masked `account.move` (mask={0,2}) renders `struct AccountMove { name,
  state }` (dropping field 1 `amount_total`), `fn new(name, state)`, and
  `fn action_post(&mut self)` with `CLASS_ID = 0x0202`. Closes the
  transpile chain with LEG 1 (ruff #40) + LEG 2 (D-OGAR-ODOO-INHERIT-MIXINS).
- **D-CHAIN-CONSUMPTION-GROUPING (operator framing 2026-07-03;
  RESOLVED 2026-07-05 — see D-CHAIN-GROUPING-RESOLVED-12SLOT below,
  grade lives there; the SDK gate is lifted):** the operator's
  transpiler-substrate reminder names two consumption geometries for
  the substrate: *"to be consumed via part_of/is_a (rails) or triplets
  (4x (8:8:8), or (3x (8:8:8:8) (odoo ?)"* (verbatim, question mark
  included — the Odoo grouping is explicitly open). Candidate readings,
  recorded so the ruling has something concrete to confirm or correct:
  **(a)** the 12 bytes of facet tier chains (`part_of[6] + is_a[6]`)
  admit two groupings — **4×(8:8:8)** = four 3-byte compressed-classid
  entries (deeper ancestry, compressed refs; the Rails hierarchical
  read), vs **3×(8:8:8:8)** = three full 4-byte `u32` classids (fewer
  levels, full fidelity; the Odoo triplet-stream read). Same 12 bytes,
  two groupings — a byte-level echo of the 3×4-vs-4×3 standing watch
  (`CLAUDE.md`). **(b)** the 3-byte entry spelling could be
  `domain:appid:classview-byte` per D-CLASSID-HI-U16-SPELLING's byte
  registers (drops one lo-half byte per entry). **Consequence either
  way:** Rails consumers navigate hierarchically (`part_of`/`is_a` —
  the AR object-graph read); triplet-flavored consumers stream SPO
  groupings — one chain field, two reads, so the grouping choice is a
  *view*, not a layout fork, IF entry width is settled. Gate: OGAR SDK
  chain-navigation API (see EPIPHANIES E-AR-DIRECT-SDK) blocks on this
  entry's ruling.

- **D-CHAIN-GROUPING-RESOLVED-12SLOT (operator ruling + upstream
  handover; 2026-07-05; [G]):** resolves D-CHAIN-CONSUMPTION-GROUPING
  (above; status regraded in place): the grouping is not a layout fork
  but the **shape-adaptive 12-slot factoring** of the V3 node —
  `6·2 = 4·3 = 3·4 = 12`: AR/Rails → **6×(part_of : is_a)** (mereology ×
  taxonomy), generic → **4× SPO triplets**, Odoo → **3× SPOG
  quadruplets** — one substrate, the factoring chosen per source shape.
  Odoo's SPOG lives in the lance-graph⟷OGAR **V3 substrate, NOT a
  SurrealDB AST** (operator verbatim 2026-07-05: *"Odoo's spog lives now
  in V3 substrate in lance-graph <>OGAR, not surrealdb AST"*);
  consumption is the ERB **fieldview×classview** pattern rendered via
  **askama (Rust) / jinja (Python)**, dispatched on the **classview
  bitmask** (operator verbatim: *"you consume it with fieldview
  /classview ERB pattern> askama (rust)/jinja (python) based on
  classview bitmask"*). Consequence: the E-AR-DIRECT-SDK chain-API gate
  is lifted — the SDK's navigation API implements the three factoring
  VIEWS over one 12-slot field, never three layouts. Source of record:
  openproject-nexgen-rs
  `.claude/handovers/2026-07-05-ogar-v3-consumer-migration-plan.md` §2.

- **D-OGAR-CONVERGENCE-SHAPE (operator ruling; 2026-07-05; [G] for the
  declaration; per-layer grades in the EPIPHANIES twin):** OGAR is THE
  convergence shape across seven layers — schema (the addressed global
  schema), ruff codegen (all code = generated projections), classview
  fieldmask (one mask → askama context / typed constructor / SQL column
  set), OGIT ontology as controller DTO (wire membrane; Auth > RBAC as
  mask algebra), lance-graph V3 substrate + classview rows, reasoning
  vs controller methods as ONE invocation surface
  (`<port>::<path>(<shape>)` — StepDomain already carries the union;
  ERP actions join as one more port), ractor mailbox-kanban as graph
  execution (an action executing IS a kanban transition). Canonical
  text + per-layer grades + the three open seams (view stratum ·
  ActionDef↔UnifiedStep mapping · TTL→DTO lowering) + the three
  falsifiers (WorkPackage parity witnesses · classview-mask round-trip
  · one action end-to-end): `.claude/board/EPIPHANIES.md`
  E-OGAR-CONVERGENCE-SHAPE, with companions E-ONE-MASK-THREE-PORTS and
  E-MIRROR-EXTERNALIZATION (same arc). Execution receipts feeding the
  declaration: ruff `8d6c31b` (schema stratum upstreamed), op-nexgen
  `4102eb0` (harvest config), the un-vendor arc (§5 steps 2-3).

- **D-PARITY-PROBE-WP-1 (falsifier #1 measured; 2026-07-05; [G] —
  re-runnable probe, scratchpad parity-probe/):** the WorkPackage parity
  witness ran against the hand-written op-work-packages model (18
  fields). Verdict: convergence shape NOT falsified; freeze gate NOT
  met pre-fix. Schema stratum + addressing were 100% faithful (18/18
  columns with types + nullability in the IR; classid 0x0102_0001
  decomposing correctly); the entire loss was ONE gate —
  project_odoo_fields Python-only, Rails fields dropped at lift
  (GAP-1), plus two og_scalar_type rows (string/bigint) and the
  unwired not_null→required slot (GAP-3/4) — all closed by the commit
  carrying this entry. Post-fix probe tally recorded in that commit's
  body. Doctrinal reading (operator-ratified shape): the 8 oracle
  fields spelled `<x>_id: Id` are the ORM spelling; the transpiler's
  `<x>: ToOne<X>` is the AR spelling the canon keeps — the freeze-gate
  metric is AR-shaped domain object vs oracle-minus-FK-spelling; the
  `_id` scalars are the STORAGE projection of the classview mask, not
  the domain object. Remaining honest 15%: DoneRatio/Formattable
  newtypes, the two nullability inversions (id, done_ratio),
  lock_version→guard (the ActionDef seam), services. Cross-ref:
  E-OGAR-CONVERGENCE-SHAPE falsifier #1, E-ONE-MASK-THREE-PORTS,
  E-KEEP-AR-REMOVE-ORM.

- **D-ROUTE-KIND-VERB-STRATA (5+3 council verdict; 2026-07-05; [S] for
  the rejected unification, [G] receipts, [H] probe):** the proposed
  "route dedup = the SoC lint's DO-arm" was council-REJECTED as mere
  rhyme: `soc` is a harvested-relation, byte-capped (`u8::MAX` = the
  SoA cascade rank), `law_holds`-falsifiable lint; route kinds are a
  human-curated recipe taxonomy with no cap, no law, and (today) no
  harvested discriminant facts (HTTP verb / writes / return-shape are
  not in ruff's predicate set). The shared "N siblings → K
  representatives + residual" shape is the workspace's universal
  quotient primitive (palette / CAM-PQ / centroid tiles) — true and
  vacuous. What lands instead: (a) the carve — a `HandlerKind` is
  **verb × transport × persistence-shape**; the stripped `is_a` verb is
  the only codebook candidate and resolves through the same
  canonical-verb rail as `ActionDef`, never a parallel vocabulary;
  recipes are adapter-side; (b) one independent `[H]` probe — the
  OP⇄Redmine route-surface kind A/B with a pre-registered KILL
  threshold, DISTINCT from capstone C5's verb A/B; (c) the mint fence —
  no verb-row allocation until that A/B is green. Canonical text +
  grounds + receipts: `.claude/board/EPIPHANIES.md`
  E-ROUTE-KIND-VERB-STRATA. Consumer-side artifact: op-nexgen
  `crates/ruff_python_dto_check/` regraded to a PARKED un-upstreamed
  sqlx-target delta against live ruff's `ruff_python_dto_check` (see
  its README).
  **⊘ CORRECTED IN PLACE (2026-07-05, same day — operator ruling;
  regraded: convergence `[G]`, coverage `[H]`):** the `[S]` rejection
  above is WRONG and SUPERSEDED by `E-RECIPE-REUNION-ORDER`. Route/
  fieldview dedup IS the SoC doctrine by operator canon
  (`CLASSVIEW-FIELDVIEW-ASKAMA-BITMASK`, 2026-06-29:
  `FIELD_MASK_CAP = MAX_SIBLINGS_PER_TIER`, ONE cap — the field-view
  mask cap and the soc sibling cap are the same constant), and
  `HandlerKind` is the canon Action-kind recipe family
  (RAILS-COVERAGE-KIT §5, 2026-06-30 — mint as `RecipeConceptId`,
  converges like class concepts). The reunion (Redmine ⇄ OpenProject at
  the AR shape) is an ORDER, not a conjecture. What survives from the
  rejected pass is only the factual gap ledger (unharvested writes/calls
  per F17; the recipe-concept codebook unminted) — queued work, upstream
  in ruff + OGAR, never op-side. `ruff_python_dto_check` re-framed as the
  ERB-fieldview → askama render recipes + Action-kind corpus that seeds
  the `ogar-render-askama` kit and the recipe codebook, not dead weight.
- **D-156-CORRECTNESS-FIXES (2026-07-06; [G]):** the three PR-#156
  open findings closed with red-before-green regression tests —
  (a) `ogar-from-rails::extract_app*` now routes through
  `ruff_ruby_spo::extract_app_with_schema`, so the physical schema
  stratum reaches `Class` on the shipped mainline API (not only a
  direct `lift_model` call); (b) `is_fk_shadowed_by_association` now
  honours an association's explicit `foreign_key:` (not just the
  `<name>_id` naming convention), killing the `user_id`+`author`
  double projection; (c) `project_rails_fields` skips a physical
  column whose name already exists as a lifted AR-DSL attribute,
  ending the duplicate-struct-field bug (the dup-guard). WS-V gate
  honoured: each fix hunk was stashed individually and its paired
  test confirmed RED, then GREEN — a test that passed both ways would
  not have shipped. Physical-vs-declared type reconciliation on
  collision noted as a follow-up, not taken. Drive-by: the
  `ogar-class-view` "68" doc fuse corrected to 79. Cross-ref
  D-PARITY-PROBE-WOA-1 (the parity this unblocks on the mainline
  path).

- **D-BEHAVIOR-ACTION-EDGES (2026-07-06; [G] facts / [H] lowering;
  commit 789c7ed):** `lift_actions` now carries ruff's
  `Function.reads/writes/calls` onto `ActionDef` as first-class
  effect annotations — the IR surface signed off against OGAR-AS-IR
  §3 (test 2) — making behavior part of the compile-time substrate
  (Operator §3) without fabricating reactive `kausal` from plain
  reads. Effect provenance is kept honest rather than flattened:
  **writes/calls are authoritative** (assignment + call targets the
  extractor sees directly) while **reads are inferred** (name
  references, an over-approximation), annotated as such. Full body
  lowering (`body_source`, params, value-carrying `on_enter`) stays
  blocked on a named `ruff_spo_triplet::Function` extension — the
  render crate emits honest `// TODO: port` stubs until then.
  Cross-ref E-BEHAVIOR-AT-COMPILE-TIME.

- **D-FIELDMASK-LOUD-FAIL (2026-07-06; [G]):** the single-`u64`
  `FieldMask` 64-field ceiling is now loud, not silent — a >64-field
  class under a non-FULL mask returns
  `RenderError::TooManyFieldsForMask` from
  `render_class_with_methods` instead of dropping fields 64+ (this
  loud-fail guard replaces the old silent-drop; the FULL sentinel
  still emits all). A pin test documents the pre-fix silent drop.
  **Falsifier #2 RAN and is GREEN** for the ≤64 case: one
  ClassView×FieldMask projection at **mask=45** yields the identical
  field-name set through askama (Rust) and jinja2 (Python) — the
  dual-target render proof (Operator §5). Caveat pinned: the FULL
  sentinel *aliases* every genuine all-ones mask, so FULL-vs-
  explicit-full is indistinguishable at the u64 tier — documented,
  not papered over (it is why the widening carries a canonical form).
  This is the INTERIM OGAR-side guard; the authorized real fix is
  D-FIELDMASK-WIDENING (Ruling c). Cross-ref D-FIELDMASK-WIDENING,
  E-ONE-MASK-TWO-ENGINES.

- **D-FIELDMASK-WIDENING (2026-07-06; [G] — cross-repo lance-graph
  PR):** the authorized backward-compatible `FieldMask` widening
  (Ruling c) lives as a lance-graph-contract PR: a `WideFieldMask`
  carrying a **canonical form** (trailing all-zero chunks trimmed) so
  a ≤64-position mask and its wide spelling compare equal —
  **repr-independent Eq/Hash**, the non-footgun invariant. Every
  existing `FieldMask(u64)` constructor/semantic stays exact;
  positions 64+ become representable without moving bits 0..63 (N3
  stability). A review-found P0 (**V-L-P0**: the first cut hashed the
  raw repr, so `Small(x)` and `Wide([x])` hashed differently while
  comparing equal — a broken Eq/Hash contract) was caught and FIXED
  **before merge** via the canonical-form normaliser plus
  cross-tier Eq/Hash-agreement tests. `account.move` (109 fields,
  Odoo) is the motivating >64 case; WoA parity uses ≤64-field
  classes. No classid version split needed. Cross-ref
  D-FIELDMASK-LOUD-FAIL.

- **D-MEMBRANE-TTL-DTO (2026-07-06; [G] shape):** the membrane now
  has a named pipeline — `ogar_from_schema::lift_ogit_entity` lowers
  an OGIT entity TTL + its attribute TTLs into the canonical `Class`
  (controller-DTO wire shape), pinned by a wire-name test on the
  `DocumentInfoRecord` fixture (documentNumber / Type / PartId /
  Version). Key correction to the mission's `vocab/ogar.ttl` pointer:
  that file is `owl:Class` meta-vocab that the current `rdfs:Class`
  walker **deliberately does not recognise** — the OGIT NTO corpus is
  the real entity-fixture family, not the meta-vocab. Remaining
  `[H]`: label→wire-name late resolution, owl:Class walker support,
  and a dedicated Facet-bearing DTO-emit pass. Cross-ref
  E-OGAR-CONVERGENCE-SHAPE membrane layer.

- **D-EXEC-ONE-ACTION (2026-07-06; [G] machinery / [H] ERP-wiring):**
  one lifted `ActionDef` now punches end-to-end through the reference
  `NativeCommandExecutor` (`lift_actions → ActionDef →
  NativeCommandExecutor → result`), the executable half of Falsifier
  #3. The remainder — kanban transition + Lance tombstone — needs the
  lance-graph ractor runtime (outside `/workspace/ogar`) and is
  documented as the named gap, staying `[H]`. On the cognition side a
  fuse pins that `ActionDef` lowers onto the `UnifiedStep` shape while
  `StepDomain` carries **no** ERP/controller variant yet: the
  exhaustive `match` **breaks loud at compile time** the moment the
  ERP arm is added (the named ActionDef↔UnifiedStep seam). Cross-ref
  E-OGAR-CONVERGENCE-SHAPE falsifier #3 + cognition layer,
  E-ONE-MASK-THREE-PORTS.

- **D-PARITY-PROBE-WOA-1 (2026-07-06; [G] for the OGAR half):** WoA
  `TimesheetActivity` (`models.py:1746`, transcribed as a synthetic
  `ModelGraph` standing in for the not-yet-built
  `ruff_sqlalchemy_spo` frontend) lifts through the new
  `project_sqlalchemy_fields` / `compile_graph_sqlalchemy`
  (`WoaPort`) path and emits via `emit_python` to a structurally 1:1
  `@dataclass` — **classes 1/1, columns typed 3/3, nullability 3/3,
  associations 1/1** (FK `timesheet_id` deduped to the `timesheet`
  BelongsTo); the emitted Python **py_compiles AND instantiates**
  (the language mirror, DP-(b) Option C). **The spec's "unaliased
  bootstrap" assumption is REFUTED and pinned by test:** the class
  does not sit on classid 0 — it converges to **`0x0103_0003`** via a
  `WOA_ALIASES` pin onto the `BILLABLE_WORK_ENTRY` canonical concept
  (`0x0103` high ‖ WoA app `0x0003` low). Bootstrap 0 is the literal
  *pre-alias* value; the alias table maps it, so the test pins the
  aliased id, not 0 — and no new codebook mint is introduced (the pin
  reuses an existing concept). **Honest drift:** `emit_python` does
  not render `options.required`, so nullability annotations are
  absent from the emitted text — a real emitter gap, logged as a
  follow-up, not hand-edited. The FK-dedup logic from
  D-156-CORRECTNESS-FIXES has a **copy in `sqlalchemy.rs`, synced in
  lockstep** here; a consolidation follow-up (one shared dedup
  helper) is named. **Remaining producer gap (named):** ruff has no
  SQLAlchemy frontend — Part A (`ruff_sqlalchemy_spo`, an ruff-side
  harvest brick) is the follow-up that turns the synthetic graph into
  a real harvest of all ~151 WoA models. Methodology re-derived from
  D-PARITY-PROBE-WP-1 (original probe dir gone, RECON R8). Cross-ref
  E-PYTHON-SUBSTRATE-MIRROR, E-OGAR-CONVERGENCE-SHAPE falsifier #1.

- **D-PY-PERSIST-PG-FACET (2026-07-06; [G]):** the Ruling-(b)
  persistence core landed minimal: `ogar-adapter-postgres-ddl` emits
  PostgreSQL DDL from the ClassView — the V3 **facet table**
  (`classid` + **12 axis-indexed SMALLINT** payload columns, each
  axis independently indexable) alongside a per-class typed+nullable
  `CREATE TABLE`, the transactional System-of-Record shape. Role
  split pinned in the DDL doctrine: **PG = System-of-Record**
  (Writes / ACID / GoBD), **lance-graph = zero-copy read hot-path**
  (never the sole booking store); **moka caches ONLY the PG side**
  (rows are materialised there — a cache in front of lance would copy
  Arrow buffers into owned entries and break zero-copy). A
  `check_parity` drift-fuse **skeleton** (COUNT_FUSE pattern) compares
  sink-in vs legacy-ORM `Class` shapes and screams on divergence,
  seeding the legacy-parity revival mode. Follow-ups named (not
  built): transactional-outbox / dual-write, CDC, PostgreSQL-ORM full
  adapter, lance-graph Python hot-path bindings, moka↔moka-py tier,
  Mongo (only on real blob need). Precedent:
  ogar-adapter-clickhouse-ddl. Cross-ref D-PARITY-PROBE-WOA-1,
  E-PYTHON-SUBSTRATE-MIRROR.

- **D-PARITY-PROBE-WOA-1 — Nachtrag/Regrade 2026-07-06 (Batch-1 Item 1):**
  the `emit_python` Optional-nullability gap listed as remaining DRIFT in the
  original entry is **CLOSED**: `emit_python` now wraps `required==Some(false)`
  attributes in `Optional[…]` (mirror of `emit_rust`'s `Option<…>`) and emits
  its wrapper-contract imports via `emit_python_prelude()` → `from ogar_runtime
  import …` (design option (a); reference
  `crates/ogar-from-ruff/python/ogar_runtime.py` shipped). The `wrap_module`
  probe crutch is deleted; the emitted `@dataclass` module py_compiles AND
  imports standalone. `created_at` now emits `Optional[OgDateTime]`. Remaining
  drift on this fixture = the `WoaPort` classid-alias convergence pin
  (`0x0103_0003`) only.

- **D-PARITY-PROBE-WOA-1 — Nachtrag 2026-07-06 (Batch-1 Item 2):** the lockstep
  FK-dedup copy in `ogar-from-ruff/src/sqlalchemy.rs` (its own
  `is_fk_shadowed_by_association` + `project_sqlalchemy_fields` body, flagged
  "any change MUST land in the other") is **consolidated**: both the Rails
  (Ruby) and SQLAlchemy (Python) producers now route through the single
  `pub(crate) project_total_schema_fields` / `is_fk_shadowed_by_association` in
  `lib.rs`. New test `both_producers_share_explicit_foreign_key_dedup` proves
  identical dedup (explicit `foreign_key`, PR #156 finding (b)) across both
  paths from one implementation. Behavior-preserving; `classify_woa_domain`
  lockstep left as a separate, named follow-up.
