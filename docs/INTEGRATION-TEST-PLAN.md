# INTEGRATION & TEST PLAN — the probe-first execution companion

> **Status: EXECUTION PLAN v1** (2026-06-10). Companion to
> `DISCOVERY-MAP.md` (what was found) and `INTEGRATION-MAP.md` (how it
> composes). **This doc orders the work: no integration brick lands
> before its probe is green.** It exists because the session's own
> audit found six turns of synthesis and zero probes run — the
> workspace rule applies verbatim: *if the probe is NOT RUN, the next
> deliverable is the probe, not more synthesis.*
>
> Grading as everywhere: `[G]`/`[H]`/`[S]`; every probe row names what
> a PASS grounds and — equally load-bearing — **what a FAIL kills**.
> A probe without a kill-condition is theater.

---

## 0. The three shaky joints (self-audit, 2026-06-10) — first-class falsification targets

The canon floor (operator-pinned GUID, key-of-key-value, 3×4) is
ground truth by definition. The conjecture tower above it has three
joints that were never checked; each gets a named probe and a kill:

| # | Joint | The unexamined step | Resolving probe | Kill condition |
|---|---|---|---|---|
| J1 | **TILE256 ↔ PREFIXBOOK circularity** | the address picks the codebook that picks the address — does the recursion bottom out? | **PROBE-BOOT-1** (+ CODEBOOK-44): mint a class codebook from a corpus at BUILD time, assign addresses, re-derive — show fixpoint or document one-shot non-recursive build | no stable bootstrap → D-TILE256 demoted `[S]`; tier = spatial Morton only; CAM-PQ distance stays value-side |
| J2 | **"lossless for synthesis" scope drift** | dither-grade reconstruction ≠ content reconstruction; prose drifts toward implying the latter | **PROBE-PERT-RHO** with the *escalation rate reported* | ρ < anchor on representative tiles → D-PHASE stays dither/anti-moiré only; all codec-savings claims struck |
| J3 | **Parseval ≠ gaussian-preservation** | WH orthogonality conserves L2 *of the transform*; says nothing about the **quantized** magnitude envelope | **PROBE-WHP-1a** (transform, should pass — math) split from **WHP-1b** (L2 drift through `RollingFloor` quantization — the real question) | WHP-1b outside the Pflug band → strike "top gaussian preserved"; keep WH as sign-algebra only |

---

## 1. Wave 0 — ground in 48 hours (runs against SHIPPED code; zero new impl)

Every row here is executable today; total new code ≈ 170 LOC of tests.

| Probe | Where (proposal) | LOC | Pass | Fail kills | Grounds |
|---|---|---|---|---|---|
| **PROBE-PHASE-1** | `lance-graph/crates/helix/tests/phase_determinism.rs` | ~40 | `CurveRuler` walk bit-identical across runs/platforms; full 17-permutation verified | the deterministic-phase generator claim → D-PHASE loses its `[G]`-mechanism leg | the generator leg of D-PHASE |
| **PROBE-WHP-1a** | `helix` or `jc` example (scalar) | ~30 | `Σ\|cell\|² = Σ\|M_L\|²` exact for ±1 fields (orthogonality) | the WH framing itself (unlikely — math) | D-WHP transform leg |
| **PROBE-WHP-1b** | same file | ~30 | L2 drift through `RollingFloor` ≤ Pflug band | **"top gaussian preserved" wording (J3)** | the only part of D-WHP that was actually in doubt |
| **PROBE-PHI-1** | `jc` (companion to its `weyl` module) | ~40 | φ-stride probe sets beat uniform-random on star-discrepancy at equal n | the φ leg of the quorum (quorum survives on uniform probes) | quorum placement |
| **F2 (read+run)** | `lance-graph/crates/quasicryth-research` — `cargo test` | 0 | `paper_theorems.rs` green + hierarchy read confirms/refutes generalized-Morton addressability of a hat-equivalent tiling | nothing (red = D-MONOTILE stays `[H]` with a concrete failure shape — still progress) | possibly the biggest single promotion available (D-MONOTILE) |
| **F13 (verify)** | run the existing `simd_dispatch` W1c parity suite | 0 | 1-ULP across AVX-512/NEON/scalar (contract says mandatory — may already be green) | the correctness floor under every other probe | the floor |
| **F8** | helix fidelity probe (owed since #459: "CONJECTURE — NOT RUN") | ~60 | naive-u8 floor ≥ 0.9980 Pearson vs ground truth | helix's clean-room promotion | closes lance-graph's own oldest owed probe |

**Wave-0 exit:** ≥5 of 7 green → proceed. J3 resolved either way.
Any red is a *result* — recorded in DISCOVERY-MAP, grade moved, prose
corrected (the D-EXCITON pattern: revert with reasons, keep the
`[G]`-real legs).

## 2. Wave 1 — first implementation bricks (each gated by its probe)

| Brick | Repo | LOC est | Gate | Depends on |
|---|---|---|---|---|
| **C: `impl SoaEnvelope for MailboxSoA<N>`** (the keystone; zero production impls today) | lance-graph shader-driver | ~150 | **F6**: `verify_layout()` green; `as_le_bytes().as_ptr() == backing` (zero-copy proven) | — (leaf) |
| **O1: `Class` traversal API** (`member_of`/`members`/`group_members`/`includes`/`associations_of`) — methods on the carrier per The Click | OGAR `ogar-vocab` | ~50 + tests | unit tests incl. `through`/`inverse_of` resolution | — (leaf) |
| **PrefixShapeTable** (layout-only sibling of `MultiLaneColumn`) | ndarray `simd_soa` layer | ~150 | **PROBE-ROUTE-1**: batch ≡ scalar on 10⁶ keys; ≥4× at N=1024 | — (leaf; consumer registration comes later) |
| **WHP-3 + WHP-4** against shipped `vsa_bind`/`vsa_bundle` | lance-graph contract tests | ~130 | WHP-3: unbind margin holds to N ≤ √d/4, fails cleanly past; WHP-4: raw-XOR-on-magnitudes FAILS Chapman-Kolmogorov (a deliberately-failing guard) | — |
| **HILBERT-L4 fix** (PP-13 P0-4: re-derive `NEXT_STATE` from Hamilton 2006; exhaustive 4096-cell round-trip) | ndarray `linalg/hilbert.rs` | ~80 | `decode(encode(p,4),4)==p` exhaustive (≈16 ms) | — ; **blocks every L4 cascade claim until green** |

**Wave-1 exit:** C green unlocks the critical path; the two-algebra
guard (WHP-4) is permanently in CI.

## 3. Wave 2 — the conjecture tower gets its numbers

| Brick | Gate(s) | Depends | Kill |
|---|---|---|---|
| **4⁴ hierarchical codebook builder** (4-level 4-ary k-means, 256 leaves) | **CODEBOOK-44**: fidelity within Pflug band of flat-256; **PROBE-BOOT-1**: bootstrap fixpoint (J1) | corpus (synthetic v0 OK) | J1 kill above — D-TILE256 → `[S]` |
| **Perturbation encoder v0** (magnitude-plane + CurveRuler phase; coarse-granularity M) | **PERT-RHO** (J2; escalation rate REPORTED) + **PYR-1** (escalation tier byte-exact) + **WHP-2** (cross-backend roundtrip) | PHASE-1 green | J2 kill above |
| **Spectral anti-moiré** (jc P3 + `hpc::fft`: golden vs Base17, pre/post quantization) | **F3** — promotes D-MOIRE/D-MANTISSA or falsifies; demonstrates D-QUANTGATE's contrast | runtime session owns jc — coordinate | D-MOIRE `[H]`→dead if FFT peak appears |
| **ρ anchors re-measured** | **F10/F11/F12** (depth law; 0.9973/0.965 anchors; θ window) | runtime session | anchors drift → every τ recalibrates |

## 4. Wave 3 — identity-arc integration (the lance-graph N-phases, now gated)

Critical path `(C → D) ∥ ((R1 ∥ O7 ∥ B) → R2) → R3`, all gates already
defined in INTEGRATION-MAP §5–6:

| Phase | Gate | Note |
|---|---|---|
| D: cognitive-write `TripletProjection` + `roundtrip_eq` | **F5** (corrupt-pack must FAIL; NARS (f,c) within 1/1023) | needs C ✅(W1) |
| B: registry mint `(entity_type ↔ NiblePath)` + live `StructuralSignature` | **F4** bijection round-trip at build time | ontology side; also gates F14 |
| O7: `String → Identity` lift in ogar-vocab | unit: parent resolution via registry | the IR's typed-edge prerequisite for R2 |
| **Phase-B addendum (new):** NodeGuid group-3/4 audit | groups 3–4 yield all 8 nibbles to HIP/TWIG (canon-pass consequence) | one layout change + field-isolation matrix per I-LEGACY-API |
| R1/R2/R3: state_machine verify → ActionDef lowering → **F1** (chain + DIAMOND fixture, C3-over-`LastOrderedSet`) | F1 | **externally owned** — the only critical-path leg owned by no present session; coordination risk stays flagged |
| E/F/G: project_graph → MetadataStore migration → EntityKey wiring | per-phase DoD (INTEGRATION-MAP) | after D; F needs B |

## 5. Wave 4 — consumers & the long tail

O2 parse-walk completion (EnumDecl lift · DEFINE EVENT→ActionDef ·
non-owning post-pass — O1 unblocks the post-pass) → O3 `ogar-python`
(Odoo 17) → quorum certificate type (the #411 `contract::quorum`
scaffold) + **QUORUM-1** → O5 content tiers + **F7** → F9 kv-lance pins
→ X-track debt (X1 rename, X3 ResonanceDto fold, X4 HHTL
route-by-prefix, X5/X6) → F14/X7 wide-model render after B.

## 6. The standing rules this plan operates under

1. **Probe before synthesis** (workspace P0, now enforced by wave
   structure: a wave does not open until the prior wave's exit
   condition is met or its reds are *recorded as results*).
2. **Every red is a ledger event** — grade moves in DISCOVERY-MAP the
   same day (the D-EXCITON revert is the template: keep `[G]`-real
   legs, strike the dead analogy, name the survivor).
3. **Thresholds are measured, never optimism** (PP-13 P0-2/P0-3:
   placeholder thresholds must not gate; relative tolerances for
   spectral claims).
4. **Two-algebra guard is permanent CI** (WHP-4 ships as a
   deliberately-failing-if-violated test).
5. **Standing watches stay armed**: 3×4-vs-4×3 flip condition;
   "correct the operator at any time" mandate (CLAUDE.md).

## 7. Ownership & coordination

| Who | Owns |
|---|---|
| this session (OGAR + docs) | Wave 0 helix/jc probes, O1, O2-prep, ledger folds on every result |
| lance-graph session(s) | C, D, B, WHP-3/4, F5/F6, X-track |
| ndarray session(s) | PrefixShapeTable + ROUTE-1, HILBERT-L4 fix, perturbation encoder, CODEBOOK-44 |
| runtime/jc session | F3, F10–F12, R1/R2 (state_machine + binding) |
| operator | merge order #47→#48; PRs for the two crystallization branches; the J1/J2/J3 verdict calls if a kill fires |

---

*Cross-refs: `DISCOVERY-MAP.md` §2.10 (the canon pins this plan
tests) · `INTEGRATION-MAP.md` §5–6 (the DAG + gates F1–F14) ·
ndarray `guid-prefix-shape-routing.md` §3–5 · lance-graph
`guid-canon-and-prefix-routing.md` §2–7 · PP-13 verdict (the theater
casebook every threshold rule traces to).*
