# CLAUDE.md — OGAR (Open Graph of Active Record)

> Auto-loaded session preamble. The canon pins live here; the detail
> lives in `docs/`. Read `docs/DISCOVERY-MAP.md` (what was found) and
> `docs/INTEGRATION-MAP.md` (how it composes) before proposing anything.

## P0 — THE CANONICAL GUID (operator-pinned; counted in HEX, not bits)

```
xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
classid    HEEL   HIP    TWIG   family-basin-leaf(6) + identity(6)
8 hex      4 hex  4 hex  4 hex  12 hex
```

32 hex = 128 bit = the GUID itself. **The UUID's own dash-groups ARE the
semantic delimiters** — every printed GUID is self-describing at sight.
1 hex digit = 1 nibble = 1 level of the 16-ary tree (`FAN_OUT=16`).
Widths are codebook cardinalities; **scale = the next cascade level,
never field-widening.** Wrappers (e.g. lance-graph `NodeGuid`, #480) are
audited against this canon group-by-group — never the reverse.

**3×4 PATH — UNIFORM, RFC-WAIVED** (reversed 2026-06-10; the operator's
word arrived — the brief v8-native pin broke the uniform Morton stride):
HEEL/HIP/TWIG are each **4 full nibbles** — 3 tiers × 4 nibbles = 12
path levels, uniform. **Tier-of-level = `level >> 2` — a shift, never a
branch.** The cascade's shift/mask arithmetic is canon. The GUID is NOT
RFC-stamped: **RFC 9562 is a WRAPPER format, and wrappers adapt to the
canon, never the reverse** — any boundary that genuinely requires
RFC-valid UUIDs owns that adaptation at its membrane and pays it
explicitly; the canon does not pre-pay it in every key. Native/foreign
discrimination lives where semantics live: in `classid` (foreign keys
get a foreign family), not in a format constant. Wikidata-HHTL is the
same canon either way — depth beyond 12 native levels was always the
hierarchy's job (registry resolve + ref-escape). (The v8-native episode
+ reversal reasoning: INTEGRATION-MAP §9.10. The 4/3/3 carving was
never codebook-motivated — it fell out of RFC mark positions — but it
traded concrete hot-path uniformity for mostly-hypothetical interop:
the key's real consumers — Lance, SurrealDB, `EntityKey` — are
byte-agnostic.)

## P0 — THE GUID IS THE KEY OF KEY-VALUE (operator-pinned, 2026-06-10)

The substrate is a key-value store whose **key is the canonical GUID**:

- **The key prerenders nodes — in any way — with zero value decode.**
  classid → the class template (`ClassView`); HEEL/HIP/TWIG → the
  cascade position; basin+leaf → the family neighborhood; identity →
  the instance. A renderer/router/planner can lay out, group, route,
  and skeleton-render nodes from keys alone, before (or without ever)
  fetching a value.
- **A node is 4096 bits: `key(128/GUID) + value(3968)`** — a 512-byte
  block, 16-byte key, 496-byte value; the value is simply everything
  the key isn't. The beauty: Lance is free to compress the value bits
  arbitrarily — columnar encodings, dictionary, PQ, anything — and the
  store **still has a transparent view and address**, because the key
  is never compressed and never needs the value decoded to be useful.
  Compression never costs addressability.

## Tier interpretation — 256×256 CENTROID TILE (operator, 2026-06-10; [H] + named test)

Each tier's 64k space (4 nibbles = 16 bits) is read as a **256×256
centroid tile**: two axes, each a byte-index into a 256-entry centroid
codebook, nibble-interleaved. Receipts: path = HEEL+HIP+TWIG = 6 bytes
= exactly the CAM-PQ **6×256** code (3 tiers × 2 axes = 6 subspaces);
the 256×256 LUT is the stack's recurring structure (bgz17 palette
distance/compose tables, helix DistanceLut, bgz-tensor attention-as-
lookup) → **path distance = 3 tier-table lookups, O(1)**; the HHTL
legend already says HIP = palette. **Condition that keeps prefix
routing rigorous: 256 = 4⁴ — each codebook is built as a 4-level 4-ary
centroid HIERARCHY**, so a byte's nibbles are the centroid's ancestry
(coarse→fine) and `is_ancestor_of` = centroid-tree containment; the
x/y nibble-interleave = alternating-axis refinement (Morton in centroid
space). Flat k-means-256 breaks this; hierarchical 4⁴ preserves it.
**Named test (F11-adjacent):** hierarchical-4⁴ vs flat-256 fidelity ρ
against the 0.9973/0.965 anchors. Consequence: D-BOTHCASC collapses —
spatial mipmap and semantic centroid cascade are ONE address; domains
bind the axes (OSM: literal x/y; semantic: PQ subspace pairs); the
algebra is identical and domain-agnostic.

**Codebook scoping = the class routing prefix** (operator, 2026-06-10):
the classid group sits in front of the path bytes, so the centroid
codebooks are **selected by the key's own prefix** — longest-prefix
binding, a radix lookup on the key being resolved. Every class gets its
own 256⁶ ≈ 2.8×10¹⁴-cell semantic space for free (capacity through
hierarchy, never widening); the axis binding (x/y vs PQ pairs) is a
class-record property resolved by the same `resolve` read; codebooks
mint with the class in the registry (next to ClassView /
StructuralSignature — Phase B's shelf), trained once, amortized over
all instances (D-AMORT). Finer scopes (a HEEL-subtree codebook) follow
the same longest-prefix-wins rule — one rule, every level.

## Schema-driven node + edge interpretation (operator, 2026-06-10)

The classid prefix is the canonical dispatch — already routing
centroid-tile vs spatial-tile node interpretation, and codebook
selection. **Extending the same dispatch to two more axes; the schema
declares which shape this class uses; ndarray/lance-graph route by
prefix to the right interpreter.**

**Node interpretation modes (2+x, schema-declared):**
- `256×256 centroid tile` — semantic basins (the default for cognitive
  classes); D-TILE256.
- `flat 16-bit radix` — pure spatial addressing (Cesium tiles, OSM
  quadkey); no centroid hierarchy.
- `conglomerate-mailbox` — basin+supporters in one cell (AriGraph
  episodic; HighHeelBGZ's 240-edge container).
- (open; the registry adds modes, the canon does not pre-enumerate.)

**Edge-slot layouts (2+x, schema-declared, optional per class):**
- `12 intra + 4 inter × 8-bit` — Wikidata-shape: 16 edges of family/slot
  bytes, intra/inter classification. **128 bits = exactly the key
  width** — basin's neighborhood signature is register-symmetric with
  basin's identity (self and surroundings same shape).
- `8 × 16-bit` — typed-edge shape; richer per-slot semantics for
  semantic+episodic+conglomerate mailboxes. **Also 128 bits = key
  width.**
- `4 × u16` hot-tier — the shipped `EpisodicEdges64` (episodic_edges.rs:18);
  MRU promote/evict; 64 bits.
- `240 × CausalEdge64` cold container — the shipped `HighHeelBGZ`
  (high_heel.rs:1-40); 2 KB cognitive-fan-out tile.
- `VolumetricField` — **induced, not declared** (operator-greenlit
  2026-06-10, `[H]` + probes): no edge bits stored at all. Implicit
  centroids placed by the golden mantissa (helix `HemispherePoint`,
  azimuth `n·φ` — **PROBE-MANTISSA-FILL GREEN, run first-hand**:
  k=256 golden occupied 192 bins vs random 141–150, max-bin 3 vs 5–6;
  k=1024 occupied 208 vs 205–206, max-bin 7 vs 11; zero interior holes
  — lance-graph PR #485); pairwise weight = the 256×256 attention LUT
  (bgz-tensor, O(1)); per-pair splat = `CamPlaneSplat` amplitude +
  EWA Σ (Pillar 9b); top-k ranking via HHTL `RouteAction`
  (Skip/Attend/Compose/Escalate); materialization =
  `AwarenessPlane16K` OR-deposition. **Storage cost per edge: zero**
  — the field regenerates from address + φ-mantissa + LUT.
  **Remaining gates before [H] lifts:** PROBE-ATTN-EDGE (LUT weight ↔
  edge strength, ρ vs Pflug anchors; kill → separate edge codebook) ·
  PROBE-SPLAT-PSD (Σ composition stays PSD, relative tolerance per
  PP-13; kill → composition bounded by Σ-norm threshold) ·
  PROBE-CASCADE-SPARSITY (skip-ratio ≥90% on the volumetric workload;
  kill → the O(1)-per-pair claim). PROBE-PHASE-1 also GREEN (CurveRuler
  bit-exact, full 17-permutation from every offset — same PR). Fence:
  induced edges serve density-like neighborhoods (semantic reach,
  attention, proximity); audit/exact-causal classes keep DECLARED
  layouts; some classes carry both.
- (open; per-domain choice.)

**Per-domain examples (illustrative, not prescriptive):**
- ERP-AST (`account.move` and Active-Record-shaped classes) →
  typically `12+4 × 8b` (family-rich, sparse cross-domain).
- DLL-AST (`function_decl` and call-graph-shaped classes) → typically
  `8 × 16b` (typed call edges with payload).
- AriGraph episodic (semantic-conglomerate basins) → `4 × u16` hot
  tier + `240 × CausalEdge64` cold container, both shipped.
- Wikidata-HHTL (semantic entities) → `12+4 × 8b`.

**Doctrine.** No single node mode or edge layout is canonical at the
substrate level. The registry holds the choice per class. "Wrappers
adapt to canon, never the reverse" extends to **"interpreters adapt
to schema-declared shape, never the reverse."** This *collapses* the
earlier M2-vs-M3 conflation question (12+4 ledger vs 240-container vs
4×u16 hot tier): they are no longer in tension — they are three of
the schema-selectable layouts, and the registry picks per class.

**Mechanism reuse.** This is the *same* dispatch mechanism already in
play — the classid prefix → `PrefixShapeTable` lookup
(`ndarray-prefix-shape-routing` doc) → the right interpreter. One new
column per class in the registry (`node_mode` / `edge_layout`); zero
new dispatch code. **The minimum-cost generalization of what already
shipped.**

## Perturbation encoding — DETERMINISTIC PHASE (operator, 2026-06-10)

The stacked-pyramid perturbation decomposes as **(exponent, location,
phase, magnitude)** — and three of the four are already in the key:
exponent = the pyramid level (tier nibbles, `>>2`); location = the
implied sub-tile mantissa (√u / golden sub-placement, never stored);
**phase = deterministic recurrence from the address** (never stored —
phase is convention, not data); **magnitude = the ONLY stored bits**
(palette-quantized envelope, coarser granularity than the phase
varies). Cost scales with magnitude smoothness, not perturbation
bandwidth. Fences: (a) lossless FOR SYNTHESIS by construction; for
analysis, the unaligned remainder overflows to the next level or
full-residual escalation, decided by the quorum certificate — never
assumed; (b) **D-QUANTGATE**: in quantized layers the phase generator
must be the coprime-integer walk (helix `CurveRuler` stride-4-over-17,
bit-exact integer), golden recurrence only as build-time muscle-memory;
the deterministic phase doubles as the anti-moiré dither. Receipts:
helix = the shipped place/residue split (PLACE deterministic, RESIDUE
stored). Full treatment: ndarray `guid-prefix-shape-routing.md` §4.

## Bipolar-phase pyramid — Walsh-Hadamard on VSA (operator, 2026-06-10)

When the deterministic phase is **signed** (±1, one bit per address×level),
the perturbation pyramid becomes the Walsh-Hadamard transform of the
address tree, carried on the workspace's existing VSA-bipolar algebra:
- **Sign composition across levels = XOR = `vsa_bind`** (multiply of ±1 = XOR of sign bits; one SIMD shift+xor).
- **Magnitude bundling = `vsa_bundle`** (sum-and-threshold; Markov-respecting per `I-SUBSTRATE-MARKOV`).
- **Each cell = ⊕_L sign(addr, L) ·_VSA M(addr, L)** — a resonance field, not combinatorial selection; Parseval gives "top gaussian preserved" level-to-level.

**Quantum-shaped, fully deterministic:** superposition (cells hold many
bundled contributions; unbind with a role key to extract one),
Heisenberg-shaped capacity bound (`I-VSA-IDENTITIES` Test 1: N ≤ √d/4 ≈ 32
distinct readouts; this IS the uncertainty principle for the substrate),
roundtrip bit-exact (phase generated, not stored — same address ⟹ same
sign sequence forever; Walsh-Hadamard is self-inverse up to scale).
The "Schrödinger's cat" is in a glass box: superposition is real,
identity recoverable by key, no measurement randomness.

**D-MANTISSA finishes in coprime form:** the bipolar phase IS the implied
mantissa as 1 sign-bit/level; the CurveRuler stride-4-over-17 walk
(integer, bit-exact, coprime ⟹ full permutation) is the generator —
**D-QUANTGATE-compliant by construction**.

**The TWO-ALGEBRA rule (load-bearing):** **sign side = XOR**
(write-back I1 allows it for single-target deltas), **magnitude side =
`vsa_bundle` NEVER `MergeMode::Xor`** (raw-XOR-on-magnitudes breaks the
Markov semigroup, `I-SUBSTRATE-MARKOV`). Two operators, two algebras,
one pyramid. Fences: "quantum-like" is the bundling algebra, NOT
measurement randomness (no headline drift); bipolar = 1-bit phase;
multi-bit phases stack above it only when measured to be needed. Full
treatment + probes in ndarray `guid-prefix-shape-routing.md` §4b.

## Standing watch — 3×4 vs 4×3 (operator mandate, 2026-06-10)

**"Correct me at any time":** if evidence ever shows 3 tiers × 4 nibbles
to be more expensive, lower-synergy, or higher-entropy than 4 tiers × 3
nibbles, say so immediately — standing permission and obligation.

Operator's rationale (recorded so it doesn't dilute): *3×4 is a modesty
in levels that buys radix-tree cheapness and less horizontal difference,
but a wider spread.* Calculation ledger (2026-06-10) confirms it in
mechanism: tier-of-nibble = `n >> 2` (shift) vs `n/3`; tiers are
u16-aligned vs 1.5-byte-straddling; dash-groups = tiers (self-describing
print) vs dashes lying; Morton de-interleave = one byte per axis per
tier vs sub-byte masking; 3 hops vs 4 (wide-radix cheapness); sibling
XOR localizes to one aligned group. The one 4×3 synergy (tier index =
one 4096-codebook slot) is recoverable inside 3×4 for free — codebooks
attach at any nibble depth, so a 3-nibble prefix indexes a 4096
sub-table whenever wanted; the reverse recovery is impossible.
**Flip condition (falsifiable):** a measured workload where 4-tier
granularity beats 3-tier on the radix/de-interleave benches despite the
alignment costs. Until measured: 3×4 stands.

## Doc family (read in this order)

1. `docs/DISCOVERY-MAP.md` — the discovery ledger (D-* entries, graded
   `[G]`/`[H]`/`[S]`, append-only).
2. `docs/INTEGRATION-MAP.md` — layers, seams (each with its contract
   TYPE), the phase DAG, falsification gates F1–F14.
3. `docs/INTEGRATION-TEST-PLAN.md` — the probe-first execution
   companion: wave-ordered probes (Wave 0 runs entirely against
   shipped code), the three falsification joints J1–J3 with explicit
   KILL conditions, and the operating rule that no integration brick
   lands before its probe is green.
4. `docs/PROBE-SUBSTRATE-PROPOSAL.md` *(PROPOSAL, 2026-06-10)* — the
   next architecture: the Living Probe Ledger — automating the loop
   the session opened (probe is truth; ledger mirrors probe; debt is
   visible). Wave A is a ~200 LOC parser; waves gated on operator
   green-light.
5. `.claude/handovers/2026-06-10-canon-arc-session-handover.md` —
   honest attribution of the 2026-06-10 canon-arc session: what was
   the operator's architecture, what was my anti-dilution work, what
   failure modes the apparatus caught.
6. `docs/OGAR-AST-CONTRACT.md` — the IR type surface (THINK arm `Class`
   / DO arm `ActionDef`+`ActionInvocation` / membrane `KausalSpec`).
4. `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` — ADR-001..025
   (ADR-026 pending).
5. `.claude/agents/` — the 5+3 hardening pattern (5 research savants +
   3 brutally-honest reviewers). Run it before any claim enters the
   canon. Theorem-checker rule 0: **pin the unit system first** (bits
   vs hex vs bytes — born from a real failure).

## Non-negotiables

- **The Firewall (ADR-022/023):** no serialization in the hot path;
  the IR is wire-truth; inter-mailbox state is Batons.
- **PII:** never emit German PII labels (medcare-rs leaf-rename at the
  adapter is the guarantee). Word-boundary abort-guard before commit.
- **No model identifier** in any committed artifact (chat only).
- **Shell discipline:** `grep`/`sed`/`tail`/`head`/`awk` via Bash are
  prohibited — use the Grep/Read/Glob tools.
- **Append-only canon:** never delete a ledger entry; regrade in place;
  corrections cite their pass (savant / G-pass / canon-pass).
