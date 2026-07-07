# MedCare ⇄ OGAR C# Parity — Future Integration Plan

> **Status:** PROPOSAL (2026-07-07). Not shipped; gated on operator
> green-light per OGAR probe-first discipline (`docs/INTEGRATION-TEST-PLAN.md`
> — no integration brick lands before its probe is green).
>
> **READ WITH:** `.claude/knowledge/hotplug-consumer-migration.md`
> (§ Future synergies item 1 — the ActionDef-from-ruff path this plan
> instantiates for C#), `docs/OGAR-TRANSPILE-SUBSTRATE.md` (pull-in /
> pull-back / 85-15 split), `docs/ARAGO-ACTIONHANDLER-PARITY.md` (the
> parity-doc pattern this mirrors), `MedCare-rs/CLAUDE.md` (MySQL as the
> permanent parity oracle; MedCareV2 as the C# verification twin).

---

## 0. One-paragraph thesis

MedCare's healthcare classids (`Patient 0x0901`, `Diagnosis`, … — the 7
`HealthcarePort` Health entities) are **minted concepts with no action
table**, so `resolve_hotplug` returns `NoCapabilitiesFor` for every one
of them (OGAR `capability_registry.rs` encodes this as an explicit test).
That absence is a **deficiency, not a design limit**: the OCR table is
hand-authored *only because tesseract-rs has no ActiveRecord source*
(`ocr_actions.rs` § "Why a hand-authored table, not a `lift_*`
extraction"). **MedCare has a C# source** — so its healthcare action
table should be **harvested** (`ruff_csharp_spo` → `ModelGraph` →
`ogar-from-ruff::lift_actions`), not fabricated. And because MedCare is
C#, the SAME capability surface materialized two ways — `medcare-rs`
(Rust, via `ogar-vocab`) and MedCare/MedCareV2 (C#, via
`ogar-adapter-csharp`'s emitted module, #177) — becomes a **cross-language
parity witness**: two independent renderings of one authority, diffed.

---

## 1. What shipped, what's green, what's missing

### Shipped (#177, merge `e8626b9`)
- `ogar-adapter-python` / `ogar-adapter-csharp` — emit a self-contained
  foreign-consumer module (`CLASS_IDS`, `domain_tables()`,
  `resolve_hotplug` mirror with the same 5 drift arms + check order, the
  V3 4+12 `Facet` reader) with **zero `ogar-vocab` link, zero runtime
  serialization**. This is the "pull-back → codegen emit" leg for
  non-Rust consumers.
- Verification: both crates share ONE `ground_truth::assert_dump_matches`
  comparator (Python side owns it; C# `[dev-dependencies]`-path-deps it),
  so the two loops can never silently disagree on "correct." Each also
  has a hand-built 16-byte facet byte-parity decode.

### Green here (2026-07-07)
- **Python parity: 2/2 pass for real** (emit → `python3 -m py_compile`
  → import → `dump()` diffed vs live `ogar_vocab`, + facet decode).
- **C# parity: now skips gracefully** when `dotnet` is absent (this
  session added a `dotnet_available()` / `python3_available()` probe:
  skip-with-notice off-CI, hard-fail under `CI` so runner coverage is
  never silently lost). Unverified in a bare sandbox; runs for real on a
  `net8.0`-provisioned CI image.

### Missing (this plan's work-list)
1. **No healthcare action table** in `ogar-vocab` — the OCR table is the
   only `domain_tables()` entry. (deficiency to fix)
2. **No C# frontend for `lift_actions`.** `ogar-from-ruff::lift_actions`
   (`lib.rs:574`) is language-agnostic (operates on a `Model`), but the
   only shipped `ModelGraph` frontend is `sqlalchemy.rs` (Python). A
   `ruff_csharp_spo → ModelGraph` bridge (a `csharp.rs` sibling of
   `sqlalchemy.rs`) does not yet exist. This is the C# analogue of the
   doc's "missing C++ arm for `lift_actions`."
3. **No cross-language parity harness** wiring MedCare's C# side to the
   emitted `ogar-adapter-csharp` module.

> **Distinction to keep straight:** `ogar-from-ruff::emit_csharp(&CompiledClass)`
> (`emit.rs:373`) emits a **per-class wrapper record**; `ogar-adapter-csharp::emit_csharp(namespace)`
> (#177) emits the **whole capability surface** module. Two different
> emit layers; this plan uses the latter for the parity witness and the
> former (optionally) for per-class MedCare wrappers.

---

## 2. The two loops this unlocks

### Loop A — HARVEST (fixes the deficiency; table auto-derived)

```
MedCare C# source (db_*.cs, service methods)
  → ruff_csharp_spo        harvest AST → (subject, predicate, object) facts
  → [NEW] ogar-from-ruff::csharp.rs   facts → ModelGraph
  → ogar-from-ruff::lift_actions(&Model) -> Vec<ActionDef>   (lib.rs:574)
  → [NEW] healthcare_actions.rs        Vec<ActionDef> → OcrActionSpec-shaped table
  → ogar_vocab::capability_registry::domain_tables()  += one entry
  → medcare-rs HOT_PLUG const + activation test resolves GREEN
```

The healthcare action table is then **harvested, not hand-authored** —
a MedCare method promoted/renamed in C# reshapes the table on
re-harvest, exactly the property the doc's Future-synergy-1 promises.
`lift_actions` already carries read/write/call effect facts +
kausal-depends (`lib.rs` tests `lift_actions_carries_read_write_call_effect_facts`,
`lift_actions_depends_field_yields_kausal_depends`), so the DO-arm facts
survive the lift.

### Loop B — PARITY (the brilliant part; MedCare IS C#)

```
                    ogar_vocab (Rust authority)
                   /                            \
   medcare-rs (Rust)                    ogar-adapter-csharp::emit_csharp
   via ogar-vocab path dep               → generated C# capability module
        |                                        |
   resolve_hotplug (Rust)                consumed by MedCareV2 (C#)
        |                                        |
        \______________ diff via ground_truth __/
             (same dump() format, two languages)
```

The capability surface is rendered by **two independent codepaths in two
languages** and diffed by the one `ground_truth` comparator. This is a
*diverse-redundancy* witness in the exact spirit of MedCare's
architecture: MySQL is already the permanent parity oracle, and MedCareV2
already exists as the C# verification twin (`MedCare-rs/CLAUDE.md`).
Extending that witness to the OGAR capability surface costs one emitted
module + one comparison — no new architecture.

---

## 3. Why MedCare-C# makes this uniquely worth doing

- **The C# adapter has a real consumer for the first time.** #177's
  `ogar-adapter-csharp` currently proves itself only against a synthetic
  `net8.0` scaffold. MedCare/MedCareV2 is a *shipping* C# codebase — the
  emitted module becomes a real dependency, not a test fixture.
- **Diverse redundancy, not echo.** A Rust-vs-Rust parity test shares a
  compiler and a codebase; a Rust-vs-C# parity test shares *only the
  authority's data*. A divergence can only mean the emitter or the
  authority drifted — the strongest signal shape available.
- **It closes the transpiler's bidirectional claim** (`docs/OGAR-TRANSPILE-SUBSTRATE.md`):
  pull-in (C# → OGAR via ruff harvest, Loop A) and pull-back (OGAR → C#
  via adapter emit, Loop B) both exercised end-to-end on ONE domain.

---

## 4. Phased plan (probe-first; each phase gated on its probe green)

Ordering follows the operator's steer: **finish the consumer parity
first, then the harvest.** Loop B before Loop A — the parity harness is
the cleaner, lower-risk brick and it de-risks Loop A by pinning "correct"
before the table is auto-derived.

| Phase | Deliverable | Probe (KILL condition) | Gate |
|---|---|---|---|
| **P0** ✅ | Parity tests skip-when-toolchain-absent (this session) | `cargo test -p ogar-adapter-{csharp,python}` green in bare sandbox AND on CI | done |
| **P1** | CI provisions `net8.0`; C# parity runs for real | C# `emitted_library_builds_and_dump_matches_ground_truth` green on CI (KILL: emitted C# ≠ ground truth) | operator |
| **P2** | MedCareV2 consumes the emitted `ogar-adapter-csharp` module; a MedCare-side test dumps its capability surface | MedCareV2 `Dump()` == `ground_truth` for the OCR domain (KILL: divergence) | operator |
| **P3** | `ruff_csharp_spo → ModelGraph` frontend (`ogar-from-ruff/src/csharp.rs`, sibling of `sqlalchemy.rs`) | harvest MedCare's C# → non-empty `ModelGraph`; round-trips a known class (KILL: facts drop) | operator |
| **P4** | `lift_actions` over the MedCare `ModelGraph` → `healthcare_actions.rs` table + `domain_tables()` entry + `HEALTHCARE_{SUBJECT_CLASSIDS,EXPECTED_EXECUTORS}` | `resolve_hotplug("medcare-…", HEALTHCARE_IDS, covered)` GREEN (KILL: `NoCapabilitiesFor`/`Uncovered`) | operator |
| **P5** | `medcare-rs` `HOT_PLUG` const + activation test; executor arms | medcare activation test green against the sibling OGAR (KILL: any drift arm) | operator |
| **P6** | Loop B extended to the **healthcare** domain: MedCareV2 (C#) vs medcare-rs (Rust) parity over the harvested table | both dumps == ground truth (KILL: cross-language divergence) | operator |

**P1–P2 are "finish the consumer parity."** P3–P6 are "harvest the
deficiency away." The plan is written so P1/P2 ship value even if P3+
is deferred indefinitely.

---

## 5. Prerequisites & open questions (operator input)

1. **What are MedCare's healthcare capabilities / the executor?** The
   OCR table names 8 concrete ops + `tesseract-ogar` as executor.
   MedCare's analogue is presumably its cohort-similarity / lab-trends /
   suggestion routes (`MedCare-rs/CLAUDE.md` item 4 — axum handlers into
   `lance_graph::*`). P4 needs the executor crate name
   (`medcare-…-ogar`?) and the capability set — **or** P3's harvest
   defines them mechanically from the C# method surface (preferred:
   that's the whole point of "harvest, not fabricate"). Decide whether
   the capability set is harvest-derived (P3 drives P4) or operator-named.
2. **Which C# tree is canonical for the harvest** — MedCare (the C#
   original) or MedCareV2 (the verification twin)? P3 must point
   `ruff_csharp_spo` at one.
3. **PII guard.** German PII labels must never be emitted (OGAR
   non-negotiable; medcare leaf-rename at the adapter is the guarantee).
   The C# harvest (P3) and any emitted C# (P2/P6) run through the
   word-boundary abort-guard before commit.
4. **Approval gate.** Per `MedCare-rs/CLAUDE.md` item 5, a new symbol
   medcare needs upstream is filed + surfaced, never silently
   reimplemented. `healthcare_actions.rs` (P4) is exactly such an
   upstream OGAR addition — it lands in OGAR, medcare consumes it.

---

## 6. Non-negotiables this plan must respect

- **NO-PIN.** Every cross-repo dep is a path dep on the sibling; no
  `git+branch` (writes a rev pin). medcare's `ogar-vocab` was unified to
  a path dep 2026-07-07 for exactly this.
- **classid is address, magic is at the resolution target.** The
  healthcare table binds capabilities to the **Core node** a classid
  resolves to; neither classid half carries behavior
  (`docs/OGAR-CONSUMER-BEST-PRACTICES.md`).
- **No serialization in the hot path** (ADR-022/023). Both adapters emit
  compile-time artifacts; nothing serializes to cross a boundary.
- **Append-only canon.** This doc is a new PROPOSAL; regrade in place,
  never delete. A `DISCOVERY-MAP.md` D-entry should be filed if/when P4
  lands the first harvested (non-hand-authored) `domain_tables()` entry —
  that's a genuine substrate milestone (first auto-derived capability
  table).

---

## 7. Relationship to the OCR precedent

The OCR table is the **hand-authored control case**; the healthcare
table is the **harvested experimental case**. If P4's harvested table
resolves through the identical `resolve_hotplug` / `domain_tables()`
machinery with no special-casing, that is the proof that the authority
surface is genuinely source-agnostic — the codegen (adapter emit) and
the harvest (`lift_actions`) are two halves of one system, exactly as
`.claude/knowledge/hotplug-consumer-migration.md` § Future synergies
claims. The OCR table stays as the hand-authored baseline the harvested
table is diffed against for shape conformance.
