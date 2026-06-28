# ADR: V3 as the spine + the polyglot transpiler (Rust / Python / C#)

**Status:** Proposed (RFC). Design contract; not yet implemented or
compile-verified.
**Date:** 2026-06-28
**Context:** completes the "spine vs adapter" question left open by
`SURREAL-AST-AS-ADAPTER.md` + `SURREAL-AST-TRAP-PREFLIGHT.md`, and names the
transpiler superpower (re-emit the OGAR AST to any language via adapter).

---

## Decision

1. **V3 (the content-addressed rail record) is the spine.** SurrealQL /
   ClickHouse / PostgreSQL / TTL DDL are demoted to **peer adapters** that
   lower *from* V3 + `ClassView`. SurrealQL stops being a spine candidate.
2. **The V3 record is dual-mode and tenant-structured** (below).
3. **Codegen is an adapter family**: just as DDL adapters project the schema,
   `LangBackend` adapters re-emit *source code* (Rust / Python / C#) from the
   same IR. The IR is the interlingua; codegen is the transpiler.

---

## 1 · The V3 record (the spine primitive)

### 1.1 Dual-mode facet — `12 B = 96 bits = 6×16 = 4×24`, classid tags which
```
FacetCascade { facet_classid: u32, payload: [u8; 12] }   // 16 B, content address

   classid tag = Cascade  →  [FacetTier; 6]   // 6 × (part_of:8, is_a:8)   — POSITION (hierarchy)
   classid tag = Triplet  →  [SpoTriple; 4]   // 4 × (subject:8, pred:8, object:8) — LOCAL EDGES (graph)
```
- Cascade = depth-with-implied-predicates (mereology:taxonomy); subsumption is
  a bit-op. Triplet = breadth-with-explicit-predicates; **an SPO triple is a
  triplet-mode facet**, which unifies the SPO corpus with the facet primitive
  (today they are unjoined substrates).
- The tag rides in the classid (zero extra bytes; precedent: `TailVariant`).

### 1.2 The 512-byte record — canon is `key(16) + value(496)`
```
NodeRow 512 B  =  key(16) | value(496)            // OGAR canon (CLAUDE.md:51-52)
             ≡  32 × 16-byte slots ≡ 32 tenants × [GUID; N]   ("tenant" = a GUID member column)
   slot 0        Self GUID — the 16-byte key; never compressed, addressable with zero value decode
   slots 1..31   31 value tenants → GUID references / facets
```
**No separate `EdgeBlock`.** The `12+4` EdgeBlock is **superseded** canon
(`NODEGUID-CANON-AUDIT.md` F-5; operator 2026-06-23: "don't use 12-4, that's
the old taxonomy before family nodes"). **Relations ARE the addressing** — a
shared family prefix is a local edge; a GUID reference to another node is a
cross edge. So "edges" are simply GUID-reference tenants, not a dedicated block.

**Type info lives on `Class`, never on `ClassView`** (`CLASSVIEW-MATERIALIZATION-
PLAN.md` §3 + anti-pattern #2: `Class` carries types, `ClassView` is label-only;
"the right tool for codegen is `Class` directly"). So the tenant typing is an
**expansion of the existing `lance_graph_contract::Class`** — not a new type,
not on `ClassView`:
```rust
// lance-graph-contract — expand the existing Class; keep ClassView label-only
impl Class {
    fn tenant_schema(&self) -> [TenantRole; 31];   // static per classid; SIMD-scannable columns
}
enum TenantRole { Structural, Edge, Do, Think, Adapter }   // + nested: bool
```
The `Do` (ActionDef / do-arm) · `Think` (cognitive plane) · `Adapter`
(projection) tenants are the three arms reached *through* the classid; nesting =
a content-addressed FK column → a columnar composition DAG.

> Cross-repo divergence to reconcile (not here): lance-graph's
> `canonical_node.rs` still ships the superseded `key(16) | edges(16) |
> value(480)` with a `12+4` EdgeBlock. Per `NODEGUID-CANON-AUDIT.md` F-5 this is
> "a genuine canon-vs-operator divergence to resolve at the lance-graph level."
> This ADR follows the OGAR canon (`16 + 496`); the lance-graph-contract
> expansion reconciles `canonical_node.rs` against the family-node supersession
> in the same change.

### 1.3 Capacity is the SoC lint, not a limit
`>64 fields` · `>256/tier` · `>6 deep` · `>4 edges` · `>31 value tenants` → the
class lacks separation of concerns. **The encoding makes good SoC the only
representable shape**: overflow in any dimension is the signal; "reference
another class" (grow a limb) is always the fix. This is **already canon** —
`CLASSVIEW-MATERIALIZATION-PLAN.md` §5: "No promoted class may have more than 64
slots… if a future class crosses, **paginate via class hierarchy**" (enforced by
`field_basis_fits_in_one_u64_mask`). "Paginate via class hierarchy" *is* "grow a
limb via another class." Detector and refactor are the same mechanism. The law
is also written as a falsifier in `ruff_spo_address/examples/medcare_probe.rs`
§[G]; promote it to a registered `ruff` diagnostic (`OGAR-SOC`).

---

## 2 · The transpiler (the superpower)

The IR (`ruff_spo_triplet::ModelGraph`) is bidirectional *by intent* — `expand`
is general, but **`reassemble` today recovers only the C++ projection** (a
general reassembler is a prerequisite; see the ruff RFC) — and
`ruff_cpp_codegen` already proves `ModelGraph → Rust source`. Generalize that
one backend into an adapter family:

```
SOURCE (py/cpp/cs) ─ruff_*_spo─▶ ModelGraph ─mint─▶ Facet (content address, dedup across langs)
                                     │
TARGET (py/rust/cs) ◀─LangBackend─── ModelGraph
```
- `LangBackend { fn render(&self, &ModelGraph) -> String }` — one adapter per
  target, peers of the DDL adapters.
- Rust ◀ `ruff_cpp_codegen` (exists) · Python ◀ extend `ruff_python_codegen`
  (the formatter's generator) · C# ◀ new `ruff_csharp_codegen`.
- Content-addressing gives **cross-language dedup**: the same construct in
  Python/C++/C# mints the same `Facet` (CI convergence test).

### Honest boundary — structure transpiles, behaviour does not
`OGAR-AS-IR.md`: "the behavioural arm cannot survive lowering and stays in the
IR." The existing backend renders `MethodSig` *signatures*, not method bodies.
So the deliverable is a **schema / interface / DTO / ORM-model transpiler**
(API contracts, type defs, model shells) — enormous on its own. Full behaviour
transpilation (method bodies → executable logic) is a later arm via
`ActionDef` / `KausalSpec`, explicitly out of this ADR.

---

## 3 · Consequences

- **Positive:** one content-addressed spine; SurrealQL/DDL become honest peer
  adapters (closes the trap); the SPO corpus and the facet primitive unify;
  capacity-as-lint is enforced structurally; codegen-via-adapter gives polyglot
  re-export; cross-app/cross-language dedup for free.
- **Costs / risks:** (1) the rail address is **lossy** — it is a CAM *key*, not
  the content; the lossless shape lives in `ClassView` + the value tenants.
  (2) **Minting governance** — a content address is only stable if the
  rank-minter is frozen; the cross-language convergence test must be
  CI-enforced *before* scaling. (3) **"Everything in OGAR" = OGAR is the fleet
  bottleneck** — the zero-dep contract crate must be the only stable surface;
  the `#[deprecated]` `*Bridge` churn already shows the strain.
- **Scale honesty:** the substrate is ~11 K nodes / ~24 K triples today, not
  the aspirational 2 M; the `ruff_spo_triplet` per-language pipeline is the
  lever that scales it.

## 4 · Status of the pieces (verified `main`)
Real: `ModelGraph` interlingua (`expand` general; `reassemble` C++-projection-
only today), `ruff_cpp_spo` (`extract_dir`/`extract_tree`; `extract()` is a
`todo!()`) / `ruff_ruby_spo` frontends, `ruff_csharp_spo` loader, the 16-byte
mint (`ruff_spo_address`), one backend (`ruff_cpp_codegen` → Rust),
`bridge_codebook_convergence` (identity).
To build: Python→ModelGraph normalization, C# harvester generalization, the
`LangBackend` trait + Python/C# backends, the dual-mode `FacetMode`, the
`tenant_schema`, the round-trip + convergence CI, the `OGAR-SOC` lint.

## 5 · Companion
Implementation plan: `ruff` PR "OGAR Polyglot AST Integration (RFC)".
