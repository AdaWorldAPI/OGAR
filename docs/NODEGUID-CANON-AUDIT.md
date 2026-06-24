# NodeGuid ↔ FMA-skeleton `Guid` — Canon Audit

> **Status:** AUDIT (2026-06-23). Per the canon rule — *"Wrappers (lance-graph
> `NodeGuid`, #480) are audited against THIS canon group-by-group, never the
> reverse"* (OGAR `CLAUDE.md` P0) — this reconciles the `ogar-fma-skeleton`
> [`Guid`] against lance-graph `lance-graph-contract::canonical_node::NodeGuid`
> and the locked CANON (lance-graph `CLAUDE.md` § "Minimal SoA node", 2026-06-13).
> **No lance-graph code is changed here** — divergences are findings + recommended
> reconciliations for the operator.

## 1. Byte layout, group by group

| Group | Canon `NodeGuid` (16 B, **LE**) | FMA `Guid` (16 B, tier `[container:member]`) | Verdict |
|---|---|---|---|
| classid | `0..4` — **u32**, 4 bytes | tier 0 `0..2` — **2 bytes** (concept only) | **DIVERGE (width)** |
| HEEL | `4..6` — u16 | tier 1 `2..4` | align (offset, width) |
| HIP | `6..8` — u16 | tier 2 `4..6` | align |
| TWIG | `8..10` — u16 | tier 3 `6..8` | align |
| family | `10..13` — **u24**, 3 bytes (v1) / `12..14` u16 (v2) | tier 4 hi `8..9` — **1 byte** (`familyNode`) | **DIVERGE (width)** |
| identity | `13..16` — **u24**, 3 bytes (v1) / `14..16` u16 (v2) | tier 4 lo `9..10` — **1 byte** (`identity`) | **DIVERGE (width)** |
| (v2 LEAF) | `10..12` u16 — the **4th HHTL tier** (`guid-v2-tail`) | tier 4 `8..10` — the LEAF | **ALIGN (v2)** |
| (reserved) | — | `10..16` — 6 bytes reserved | FMA-only headroom |
| byte order | **little-endian throughout** | **container-first per tier (big-endian)** | **DIVERGE (endianness)** |
| edge block | separate `edges(16)` = **12 + 4** | **none** — relations via family addressing | **DIVERGE (superseded)** |

The HHTL middle (HEEL/HIP/TWIG, 3×u16) is **byte-for-byte aligned**. The
divergences are at the two ends (classid, family/identity), the endianness, and
the edge block.

## 2. Findings

### F-1 — `[G]` **classid collision: lance-graph `CLASSID_FMA = 0x0901` aliases OGAR `patient`.**

`canonical_node.rs` pins `NodeGuid::CLASSID_FMA = 0x0000_0901` (comment:
*"anatomy concept 0x01 in the Health domain 0x09"*, realigned 2026-06-20
ISS-CLASSID-OGAR-DRIFT). **But OGAR's codebook has `patient = 0x0901`**
(`ogar_vocab::class_ids::PATIENT`). So lance-graph's FMA classid **collides with
OGAR's `patient`** — a concrete cross-repo drift that predates this work.

This session's OGAR mint **resolves it**: anatomy got its own domain
**`0x0A` Anatomy** (`anatomical_structure 0x0A01` / `skeleton 0x0A02` /
`bone 0x0A03` / `joint 0x0A04`), deliberately *not* Health `0x09` (reference ≠
PHI; keeps medcare's fail-closed Health-RBAC set at 7). **Recommended
reconciliation:** retarget `NodeGuid::CLASSID_FMA` from `0x0901` to the Anatomy
domain — `0x0A01` for the FMA root (`anatomical_structure`), or `0x0A03` for the
bone anchor specifically. The OGAR codebook is canon; the wrapper realigns.

### F-2 — `[G]` **classid width: FMA `Guid` carries 2 bytes, canon carries 4.**

The canon classid is `u32 = [app:u16][concept:u16]` (`app_of` / `concept_of`,
`ogar-vocab/src/app.rs`). The FMA `Guid` tier 0 holds only the **concept** half
(`0x0A03`), i.e. the `app = 0x0000` (core/default-render) projection. For a
non-zero app render prefix it must widen to 4 bytes (tiers 0–1), shifting HEEL to
tier 2. **Reconciliation:** treat the FMA `Guid` as the *core-render* (app=0)
special case; a `Guid::with_app(prefix)` that occupies tiers 0–1 aligns it to the
full 4-byte canon classid. Bones are app-agnostic reference, so app=0 is correct
for them today.

### F-3 — `[H]` **family/identity width: `256:256` (1 B each) vs `u24:u24` (3 B each).**

The FMA leaf is the **brutal `[256:256]`** the operator landed this session
(256 families × 256 instances per spatial cell). The canon tail is
`family(u24) + identity(u24)` = 16.7M each. For the skeleton this is ample
(≤206 bones, a handful per cell). **This is the open economy decision** (the
`64K⁵ = 256¹⁰` discussion): the FMA `Guid` is the *byte-direct* projection;
high-cardinality domains (per-instance patient rows, per-Gaussian splat ids) need
the canon's 24-bit tail. **Update (2026-06-23):** the canon already has a closer
shape — `NodeGuid::new_v2` (feature `guid-v2-tail`) repartitions the tail to
`leaf(u16) · family(u16) · identity(u16)`, where **`leaf` (bytes 10..12) is an
explicit 4th HHTL tier** — i.e. the canon's *own* v2 carries the LEAF tier the
FMA `Guid` uses. The remaining gap is only the per-field *width* (u16 vs the FMA
byte), not the *shape*. **Reconciliation:** make the family:identity split a
**per-classid property** (canon already scopes codebooks by prefix) — bones run
`8:8`, a patient class runs `4:20` — same key width, class-local carve. Until
ratified, the FMA `Guid` is a constrained projection of the canon, not a
replacement.

### F-4 — `[G]` **endianness: FMA tiers are container-first (big-endian); canon is LE throughout.**

`Guid::classid()` reads `(bytes[0] << 8) | bytes[1]`, so `0x0A03` stores as
`[0x0A, 0x03]`. The canon stores `classid.to_le_bytes()` → `[0x03, 0x0A, 0x00,
0x00]`. The canon's LE is load-bearing: *"the trailing-6-byte local key is a
single masked load."* **The FMA `Guid` is NOT wire-compatible with `NodeGuid`
byte-for-byte.** **Reconciliation:** the FMA `Guid` is an *addressing model*
(its tiers are the semantic groups); a `From<Guid> for NodeGuid` conversion at
the lance-graph membrane must (a) LE-encode each group and (b) widen
classid/family/identity to the canon field sizes. The tier model stays
human-legible (container:member, MSB-first per the `[256:256]` notation); the
membrane pays the endianness adaptation — exactly the "wrappers adapt to the
canon, never the reverse" rule, with the byte order owned at the boundary.

### F-5 — `[G]` **edge block: canon keeps the separate `12+4` `EdgeBlock`; this session superseded it.**

The locked CANON (2026-06-13) ships `edges(16) = 12 in-family + 4 out-of-family`
as a **separate block** after the key. The operator's 2026-06-23 word
(this session): *"don't use 12-4, that's the old taxonomy before family nodes"* —
relations are the addressing (shared family prefix = local; a reference to
another node's `Guid` = cross). **The FMA crate carries no `EdgeBlock`.**
**This is a genuine canon-vs-operator divergence to resolve at the lance-graph
level**, not something to change unilaterally here. Recorded so the next
lance-graph session reconciles `canonical_node.rs`'s `EdgeBlock` against the
family-node supersession.

### F-6 — `[G]` **Independent convergence: q2 #50 builds the same tier model from the heart side.**

q2 PR #50 (`osint-bake/fma`, merged 2026-06-24, a sibling session) hydrates an
FMA heart fixture into `NodeGuid::new_v2` with each tier an **`[mixin:instance]`
8:8** pair — structurally identical to the FMA `Guid`'s `[container:member]`
`[256:256]`. It is the **Cascade/ontology** reading (HHTL = partonomy
organ→chamber→wall), the soft-tissue counterpart to the skeleton's **Located**
reading — exactly the `HhtlMode` split this crate encodes. Two independent
sessions (bones / heart) landed the same LEAF-tier `[kind:instance]` model on
`new_v2`. Consequence: q2 #50 also inherits **F-1** — it keys FMA nodes under
`CLASSID_FMA = 0x0901`, so the `0x0901 → 0x0A01` retarget fixes both at the root
(q2 uses the constant, so it auto-inherits the fix). The vocabulary divergence
(`mixin:instance` vs `container:member` vs `familyNode:identity`) is worth
unifying across the three sites (lance-graph `new_v2`, q2 #50, OGAR `Guid`).

## 3. What aligns (no action)

- **16-byte key width** — identical.
- **HHTL = 3 × u16 tiers** at the same byte offsets (`4..10`).
- **Zero-fallback ladder / RESERVE-DON'T-RECLAIM** — the FMA `Guid` honours it
  (zeroed reserved tail; classid/family fixed offsets).
- **classid = `0xDDCC` domain-prefixed concept** routing on `>> 8` — the FMA
  `Guid` concept tier is exactly an OGAR codebook id.
- **`identity` attached to `family`** — same semantics (family routes, identity
  discriminates within).

## 4. Reconciliation checklist (for the operator / a lance-graph session)

- [ ] **F-1 (highest):** retarget `NodeGuid::CLASSID_FMA` `0x0901 → 0x0A01`
      (Anatomy domain), clearing the `patient` collision. Update the
      ISS-CLASSID-OGAR-DRIFT note.
- [ ] **F-2/F-4:** add a `From<Guid> for NodeGuid` membrane that LE-encodes +
      widens (app/family/identity) — the wrapper adapts at the boundary.
- [ ] **F-3:** ratify the per-classid `family:identity` split (the `[256:256]`
      economy direction) or keep the FMA `Guid` as a documented constrained
      projection.
- [ ] **F-5:** reconcile `canonical_node.rs`'s `12+4 EdgeBlock` against the
      family-node supersession (operator decision).

## 5. Cross-references

- lance-graph `crates/lance-graph-contract/src/canonical_node.rs` — `NodeGuid`.
- lance-graph `CLAUDE.md` § "CANON — Minimal SoA node" (2026-06-13).
- OGAR `crates/ogar-fma-skeleton/src/guid.rs` — the `[container:member]` tier `Guid`.
- OGAR `crates/ogar-vocab/src/lib.rs` — `ConceptDomain::Anatomy` (`0x0A`).
- OGAR `docs/FMA-SKELETON-CONVERGENCE-ANCHOR.md` — the address model.
