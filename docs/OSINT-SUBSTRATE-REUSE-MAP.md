# OSINT reasoning substrate — reuse/wiring map

> Durable capture of the 2026-07-01 crate tour. What's already built in
> `lance-graph` for OSINT reasoning, what to reuse vs rewire, and the small
> OSINT-specific deltas. Everything below is shipped unless marked.

## The point: entropy → converged awareness per SoA

Massive codebase → massive entropy: the same awareness facet is re-implemented
across crates (4× `CausalEdge64`, 4× `ThinkingStyle`, N× fingerprint/qualia). The
**V3 `2+14` tenant node is the convergence vessel** — every facet becomes **one
typed tenant column on one 512-B SoA row**, and each crate stops owning its own
state-type and instead reads/writes the shared tenant:

```
key (2)   classid (identity/route)  |  EdgeBlock (16 in/out-family adjacency)
value (14+, ValueTenant):
  Meta(0)              MetaWord: thinking + awareness + NARS⟨f,c⟩ + free-energy   ← the vertical mantissa
  Qualia(1)            16×i4 chroma (felt sense)
  MaterializedEdges(2) 4× CausalEdge64 (causal steps)                             ← causal-edge converges here
  Fingerprint(3)       identity print          HelixResidue(4) · TurbovecResidue(5)  codec residues
  Energy(6)            accumulator             Plasticity(7)  Hebbian counter
  EntityType(8)        classid discriminator   Kanban(9)      Rubicon phase cursor
```

So "converge the awareness massively per SoA" = **the dedup work IS the convergence**:
each duplicate type deleted is one crate re-pointed at its tenant. The `CausalEdge64`
dedup below is the first, smallest instance of the whole program.

**Write-sandbox invariant (load-bearing — this is what makes convergence sound).**
The SoA is sandboxed from external writers: the **only writer is the SoA's own owner**
(`MailboxSoaOwner` / ractor single-`&mut self`, compile-time-proven exclusive —
E-CE64-MB-4). Every other crate **reads** (readonly `&view`) or **proposes**; the owner
**disposes**. No cross-writer race on the shared tenants is possible because aliasing is
a compile error. External "writes" are cycle-aware *proposals* the owner mediates
(`WriteOutcome::{Accepted, Stale, Future}`) — never a direct `value[..]` mutation. Convergence
works *because* the sole writer is the SoA itself.

## One view

```
LEARN (offline, LLM ok)  spider → Reader-LM → osint_bridge → rig/rs-graph-llm → cognitive-compiler
REFLEX (hot, no LLM)                                    elixir-template ◄─ compiled ┘
                                                          └ template-runtime runs OGAR actions
DISTANCE (one format)    causal_distance = Σ Pearl-planes · 256×256 palette   ── shared by:
                         arm-discovery oracle · deepnsm(→6×8:8) · V3 GUID tiers · SpoHead
EDGE                     causal_edge::CausalEdge64  (SPO palette + NARS⟨f,c⟩ + Pearl mask + mantissa)
RUNTIME (central)        cognitive-shader-driver: shader cycle step[5] emits CausalEdge64 →
                         MailboxSoA EdgeColumn, via the owner (ractor compile-time fence)
CARRIER                  MailboxSoA: Morton palette tiles (address) + MetaWord awareness
                         (vertical mantissa, rolls cycle→cycle until F descends) + cycle stamps
REASON                   causal_distance → 8 Pearl projections → thinking/style weights →
                         nars_dispatch → ir/logical_op ;  syllogize() chains ;
                         Pearl PO/SPO = osint_person Intervention/Counterfactual
KANBAN                   KanbanColumn Rubicon DAG ; tenant Kanban(9) phase ; owner-advanced
STORE                    ValueTenant: MaterializedEdges(2)=full CausalEdge64 · Kanban(9)=phase
```

## Reuse verdicts

| Crate | Verdict | Note |
|---|---|---|
| `causal-edge` | REUSE | `CausalEdge64` = SPO edge; **`syllogize()`** = is_a/part_of chainer; Pearl `CausalMask` = intervention/counterfactual. SKIP `CausalNetwork` (re-owns CSR, breaks borrow rule). |
| `lance-graph-planner` `cache/nars_engine` | REUSE | `causal_distance` (Pearl-masked Σ of 3×256² palette), `nars_infer`, 8 Pearl projections; styles weight them |
| `lance-graph-planner` `thinking/` + `ir/` | REUSE | `style.rs` = weight vectors; `nars_dispatch`; `logical_op` plan |
| `cognitive-shader-driver` | REUSE | central causal-edge runtime; MailboxSoA owner; uses canonical `CausalEdge64` |
| `symbiont` | REUSE | SoA kanban loop; each `NodeRow` = one board |
| `elixir-template` + `template-runtime` + `template-equivalence` + `cognitive-compiler` + `cognitive-stack` | REUSE | deterministic reflex (LLM at learn-time only). `source_ranking_v1` + `OsintGuardrail` already OSINT-native |
| `lance-graph-arm-discovery` | REUSE | Aerial+ transcode; mines NARS-truth SPO rules from tabular OSINT dims (deterministic). **Gate: D-ARM-7 Jirak floor before live SpoStore** |
| `spider` | REUSE | external OSINT crawler = the learn-time front-end, behind `OsintGuardrail` |
| `thinking-engine` | MIXED | REUSE `think.rs`/`dto`/`awareness_dto`/`bridge_gate`/`f32_engine`; REWIRE `osint_bridge`/`cognitive_trace`; **dedup its local `CausalEdge64`** (below) |
| `deepnsm` | REWIRE | adopt 2×48-bit / 6×(8:8) palette256² tenant = the V3 format (one distance shape) |

## OSINT-specific deltas (the only new work)

1. **Done** — mint `osint_system`(0x0700) + `osint_person`(0x0701) in OGAR (tests green).
2. **Dedup** — thinking-engine's local `CausalEdge64` (in `layered.rs`/`domino.rs`) is an
   8-channel cascade accumulator shadowing the canonical name; it already `to_spo()`-collapses
   to `causal_edge::CausalEdge64` at L3. Rename it → `CascadeChannels8` so the canonical is the
   sole `CausalEdge64`. Crate-internal (4 files, not re-exported). *(needs a compile-checkable pass)*
3. **Emit path** — cascade `to_spo()` → write to `ValueTenant::MaterializedEdges(2)` **via the
   `MailboxSoaOwner`** (never raw `value[..]`); phase in `Kanban(9)`, owner-advanced.
4. **Retarget `osint_bridge`** — its output is crawler text; point it at the OSINT ClassView so
   entities land as classid-keyed nodes; feed `cognitive_trace` SPO → AriGraph.
5. **Register OSINT `OgarAction` bodies** in the `template-runtime` `ActionRegistry` + an OSINT
   Rubicon/source-ranking template.
6. **deepnsm** — re-express its 4096² u8 distance as the 6×(8:8) palette256² code.
7. **AriGraph reference tenant + cold KV blob table (hot/cold split).** Add an AriGraph
   value tenant holding a small fixed-size **`path:documentid` reference** (not text). A
   **second KV table** (cold; `surreal_container`/kv-lance) maps `documentid → {wiki
   profile, links, blobs}`. The 6×(8:8) tiers stay the hot structured OSINT format; **raw
   text/links never touch the hot path** — the SIMD sweep / `causal_distance` / kanban /
   `syllogize` read only structured tenants; the profile resolves cold, off-path, for
   render/LLM/human. One-way: hot→cold read-only reference, cold never writes hot (the
   ladybug HOT/COLD invariant). Node stays 512 B; Lance compresses the cold value while the
   key stays addressable. → rich wiki node-profiles at zero hot-path cost.

## Why it's GoBD-clean
No LLM on the hot path → deterministic → replayable/auditable. The LLM (rig/spider) teaches at
learn-time; `cognitive-compiler` compiles to an `ElixirTemplate`; `template-runtime` runs OGAR
actions on the OSINT ClassView. Discovery (`arm-discovery`) and distance (`causal_distance`) are
integer/palette, float-free.

---

# Research roadmap — convergence baby steps (V3 format)

V3 is integer/palette + float-free, so every probe is an **exact** assertion (no
tolerance except at the f32 NARS edge). Each step is one small `#[test]` against
shipped code. Order = dependency order; stop at the first red — a red step means
the "one format" claim breaks there.

**Keystone first:**

- **P1 · distance identity (THE convergence test).** For N random index pairs
  `(a,b)`, assert the 256×256 palette distance agrees across the three sources:
  `SpoDistances.{s,p,o}_dist(a,b)` (nars_engine) == `arm-discovery::CodebookDistance(a,b)`
  == `deepnsm` distance. If these diverge, deepnsm/arm-discovery/causal_distance are
  NOT one format — everything downstream is suspect. Fix deepnsm→6×(8:8) here.

**Then, small steps up the stack:**

- **P0 · tenant round-trip.** Pack 12 AIRO dims → 6×(8:8) GUID1 tiers → read back
  via `ValueTenant` offsets = identity. (Bake exists; assert the carve.)
- **P2 · edge round-trip.** OSINT AIRO dims → `CausalEdge64::pack_v2` → `to_spo`/
  `from_spo` round-trips (already tested in `layered.rs`); `causal_distance` of the
  packed edge = expected Pearl-masked sum.
- **P3 · Pearl ladder.** `causal_distance(mask=PO)` drops the `s_dist` term (Intervention
  excludes the Subject confounder); `mask=SPO` keeps all three (Counterfactual). Direct
  from the code — one assert each, on an `osint_person` pair.
- **P4 · discovery determinism.** `AerialProposer` over a tiny synthetic OSINT table
  (3–4 dual-use rows) mines a known rule (`militaryUse=X ⟹ impact=Y`) → `CandidateRule`
  with exact support/confidence ppm → `arm_to_truth_u8` → `CausalEdge64`. Same data+θ ⇒
  identical rules (no seed). *(live SpoStore promotion still gated on D-ARM-7.)*
- **P5 · syllogize chain.** Two SPO edges sharing a term → `syllogize()` → derived edge
  with `Figure::Chain` + expected NARS truth (`is_a(A,B)∧is_a(B,C) ⊢ is_a(A,C)`).
- **P6 · awareness rollover.** Write an OSINT node's `MetaWord` awareness; `tick()` a
  cycle with no consumption → awareness persists (`last_active_cycle` unchanged); consume →
  stamped, skipped next cycle. (MailboxSoA test.)
- **P7 · owner fence.** `write_row` with a stale cycle → `WriteOutcome::Stale` (no mutation);
  `current_cycle` → `Accepted`. Proves the ractor single-owner cycle-aware write on an OSINT row.
- **P8 · reflex determinism.** An OSINT `ElixirTemplate` through `template-runtime` with stub
  OGAR actions: same input ⇒ same output; `template-equivalence` (rank_tolerance, no_new_claims)
  green. Closes the GoBD replay loop.
- **P9 · hot/cold isolation.** Attach a large wiki text profile to a node's AriGraph tenant
  (`documentid` ref). Assert: (a) the node stays 512 B; (b) `causal_distance` / the SIMD sweep
  is byte-identical with and without the profile (raw text never touches the hot path); (c)
  resolving the reference fetches the profile from the cold KV. The HOT/COLD invariant, tested.

**Milestone (convergence proven):** P1 green + P2–P3 green = the V3 palette format is one
metric across discovery/distance/edge. Then P4–P5 = deterministic OSINT reasoning; P6–P8 =
the awareness/owner/reflex loop. Only after P1–P8 green do we wire the external `spider`→OSINT
learn path (the operator's "only if it works without" gate).

## Probe status ledger (append-only; regrade in place)

| Probe | Status | Receipt |
|---|---|---|
| **P0** · edge-type dedup (pre-req) | **GREEN** 2026-07-01 | lance-graph `thinking-engine`: local `CausalEdge64` → `CascadeChannels8`; canonical `causal_edge::CausalEdge64` is now the sole `CausalEdge64` in the crate. `cargo check -p thinking-engine` green. Commit `7e31cd7`; EPIPHANIES `E-CE64-NAME-COLLISION-DEDUP`. |
| **P1** · distance identity (keystone) | **GREEN** 2026-07-01 | `crates/lance-graph-osint/tests/p1_distance_identity.rs` (2 passed): deepnsm `subspace_distance_table` (f32 source) → quantizer → u16 palette → `SpoDistances::s_dist` (planner) ≡ `MatrixDistance::distance` (arm-discovery), byte-exact over 4096 pairs; `causal_distance(0b111)` == plane sum. Commit `3c79f29`; EPIPHANIES `E-P1-DISTANCE-IDENTITY-GREEN`. **deepnsm already ships 6×256 CAM-PQ — no re-bake needed.** |
| **P2** · edge round-trip | **GREEN** 2026-07-01 | `crates/lance-graph-osint/tests/p2_p3_edge_pearl.rs` (2 of 3 tests): `CausalEdge64::pack_v2` round-trips `s_idx/p_idx/o_idx/causal_mask/frequency/confidence`; `causal_distance` of two edges' heads == per-plane palette sum. Commit `23aff55`; EPIPHANIES `E-P2-P3-EDGE-PEARL-GREEN`. |
| **P3** · Pearl ladder | **GREEN** 2026-07-01 | same test (3rd): `causal_edge::CausalMask` ≡ `SpoDistances::causal_distance` mask byte (S=0b100,P=0b010,O=0b001); each mask keeps exactly its planes; `PO < SPO` when Subject term > 0 (do-calculus confounder projection). Commit `23aff55`. |
| **P4** · discovery determinism | **GREEN** 2026-07-01 | `crates/lance-graph-osint/tests/p4_discovery_edge.rs` (3 passed): OSINT `militaryUse⟹impact` fixture, `AerialProposer` mines the known rule with exact `support_ppm=500_000`/`confidence_ppm=1_000_000`; two mines byte-identical (no seed); `arm_to_truth_u8` → `TruthU8{255,254}` → `CausalEdge64::pack_v2` round-trips. Commit `c2c0dd8`; EPIPHANIES `E-P4-DISCOVERY-EDGE-GREEN`. Live-SpoStore promotion still gated on D-ARM-7. |
| **P5** · syllogize chain | **GREEN** 2026-07-01 | `crates/lance-graph-osint/tests/p5_syllogize.rs` (4 passed): `is_a(A,B).syllogize(is_a(B,C))` → `Figure::Chain`, conclusion `is_a(A,C)`, deduction truth exact (f=255, c=163), mantissa +1, mask SPO&SPO=SPO; no-shared-term and identical-(S,O) ⇒ `None`; deterministic. Commit `a3820df`; EPIPHANIES `E-P5-SYLLOGIZE-GREEN`. |
| P6–P8 · MailboxSoA runtime loop | queued (needs wiring) | awareness rollover / owner fence / reflex determinism — exercise the `MailboxSoA` + `template-runtime` runtime; `lance-graph-osint` does not yet dep those. |
| P9 · hot/cold isolation | blocked | needs the AriGraph cold-KV reference tenant; `arigraph` feature still commented out in `lance-graph-osint`. |

**Convergence milestone PROVEN across FIVE vertices:** P1 (distance identity) +
P2 (edge round-trip) + P3 (Pearl ladder) + P4 (discovery determinism) + P5
(syllogize reasoning) all green ⇒ the V3 palette is ONE integer-exact metric
across distance sources, the edge carrier, the causal-mask semantics, ARM
discovery, and multi-hop NAL reasoning — on shipped code, no new production
deps (only an `arm-discovery` dev-dep + 4 test files). The "static" convergence
(how a fact is *encoded*, *compared*, *discovered*, *chained*) is proven. The
remaining probes (P6–P8 awareness/owner/reflex loop, P9 hot/cold) are the
*runtime* convergence (how facts *roll over between cycles* under single-owner
writes) and require `MailboxSoA`/`template-runtime`/AriGraph wiring into
`lance-graph-osint` that does not yet exist — the next wiring milestone, gated on
the operator's "only if it works without" external-OSINT sequencing.

**Rung-ladder framing (operator, 2026-07-01).** The probes are not a flat menu —
they are a climb of the 0–9 `RungLevel` ladder
(`lance_graph_contract::cognitive_shader::RungLevel`), which gates higher
reasoning on grounded lower rungs (`ShaderDispatch.rung` *elevates on sustained
BLOCK*, bottom-up): observation (0–1) ← P1; hypothesis (2–5) ← P3-`PO` + P4;
counterfactual (6, *on top*) ← P3-`SPO` + P5; the D-ARM-7 Jirak floor is the
stack guard (no counterfactual promotion of a discovery without observation
evidence). Consequence: **P6 (awareness rollover) IS the rung-elevation
mechanism** — unresolved surprise carried in the MailboxSoA `MetaWord` awareness
bits across cycles is what pushes `rung` up toward Counterfactual and rests it
back down on FLOW. P6 must assert the *elevation order*, not just bit
persistence. Full invariant: lance-graph EPIPHANIES
`E-RUNG-LADDER-IS-A-DEPENDENCY-STACK`.

## Formal pillars (`jc`) — the math the baby steps rest on

The `jc` crate PROVES the substrate's statistical/geometric foundations, one
`prove() -> PillarResult` per pillar. Baby steps test the *wiring*; the pillars
certify the *math* above them:

- **Pillar 5 (`jirak.rs`)** — Berry-Esseen under weak dependence = the **D-ARM-7
  Jirak noise floor** (`I-NOISE-FLOOR-JIRAK`) gating `arm-discovery` → SpoStore.
  P4's promotion gate *is* this pillar; wire P4's threshold to `jirak::prove()`.
- **Pillar 5b (`pearl.rs`)** — Pearl 2³ mask-classification accuracy = the
  `causal_distance` PO/SPO masks (P3).
- **Pillars 6/9 (`ewa_sandwich[_3d]`)** — Σ push-forward as EWA-sandwich along
  `CausalEdge64` edge paths (SPD cone) = the multi-hop `syllogize` chain (P5).
- **Pillars 7/8 (`koestenberger`/`dueker_zoubouloglou`)** — Hadamard / separable
  Hilbert fingerprint geometry (the ℓ²-fingerprint convergence).

Run `jc`'s `prove()`s as the formal gate above the probes: a baby step may pass on
one dataset; the matching pillar certifies it holds under weak dependence at scale.
