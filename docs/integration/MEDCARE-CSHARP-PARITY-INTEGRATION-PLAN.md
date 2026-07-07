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

### Progress this session (2026-07-07)
- **Generic derive seam shipped** — `ogar_vocab::capability_registry::entries_from_actions(&[ActionDef])`
  turns ANY frontend's lifted `ActionDef`s into the `(capability,
  subject classid)` join rows. The hand-authored OCR table now routes
  through it (behavior-unchanged; 4 new tests: prefix-agnostic,
  effect-fact-independent, unminted-row-survives-for-the-fuse,
  OCR-eight-rows). **This is the "config becomes data" core:** a new
  consumer registers a domain by supplying harvested `ActionDef`s, never
  by copying table logic. `healthcare_actions.rs` (P3) becomes a thin
  data source over this seam, not a bespoke twin of `ocr_actions.rs`.
- **The C++ DO-arm shipped upstream (ruff #57)** —
  `ruff_cpp_spo::method_body_arm` (`clang_walker.rs:873`) now walks C++
  method bodies into `writes`/`reads`/`raises`/`calls`, provenance-mapped
  to `Function`, with tests. That **closes the doc's "missing C++ arm"**
  and is the exact template the C# Roslyn arm must mirror (§ P6).

### Missing (this plan's work-list)
1. **No healthcare action table** in `ogar-vocab` — the OCR table is the
   only `domain_tables()` entry. (deficiency to fix; now a thin
   `entries_from_actions` data source, not bespoke code.)
2. **The C# pull-in chain EXISTS but harvests THINK-arm only.**
   (Corrected 2026-07-07 after reading the code — an earlier draft of
   this plan wrongly said "no C# frontend exists".) Every link ships:
   `ruff_csharp_spo::load` (loader/validator, `NAMESPACE="medcare"`,
   "MedCare first") → `ruff_spo_triplet::reassemble(&[Triple]) ->
   ModelGraph` (`reassemble.rs:89`) → `ogar-from-ruff::lift_actions`
   (`lib.rs:574`). The C# parse is an **out-of-process Roslyn harvester**
   (`ruff_csharp_spo/harvester/Program.cs` — Roslyn isn't Rust-callable),
   emitting ndjson. **The gap is narrower than a missing frontend:** the
   harvester emits only the THINK-arm predicates (`rdf:type`,
   `inherits_from`, `has_field`, `field_type`, `has_function`,
   `is_static` — per `ruff_csharp_spo`'s own test fixture). It does NOT
   yet emit the DO-arm method-body effect facts (`reads_field`,
   `writes_field`, `raises`, `calls`) — even though the closed
   `ruff_spo_triplet::Predicate` vocab already defines them
   (`triple.rs:100-456`) and `lift_actions` already consumes them. So
   `lift_actions` over a C# model works TODAY but yields **name-only
   ActionDefs** (empty effect facts, no `kausal`). The C# analogue of
   the doc's "missing C++ arm" is thus a *harvester method-body walk*,
   NOT a Rust frontend crate.

   **Consequence for the hot-plug table:** name-only is ENOUGH for it.
   `domain_tables()` needs `(capability_name, subject_classid)` rows —
   capability = the `has_function` object, subject = the concept's
   classid — both already in the THINK-arm harvest. The DO-arm walk
   enriches ActionDefs (projection / kausal / RBAC) but is NOT required
   for `resolve_hotplug`. So the medcare hot-plug is materially closer
   than "harvest a whole new arm"; see the revised P3/P4 split.
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
  → Roslyn harvester (harvester/Program.cs, .NET)   AST → SPO triples (ndjson)   [SHIPS]
  → ruff_csharp_spo::load                            validate vs closed vocab    [SHIPS]
  → ruff_spo_triplet::reassemble(&[Triple])          triples → ModelGraph        [SHIPS, reassemble.rs:89]
  → ogar-from-ruff::lift_actions(&Model)             ModelGraph → Vec<ActionDef> [SHIPS, lib.rs:574]
  → [NEW] healthcare_actions.rs                      ActionDefs → OcrActionSpec-shaped table
  → ogar_vocab::capability_registry::domain_tables() += one entry
  → medcare-rs HOT_PLUG const + activation test resolves GREEN
```

Only ONE new file on the OGAR side (`healthcare_actions.rs`) — the entire
harvest chain above it already ships (§1 item 2). The table is then
**harvested, not hand-authored**: a MedCare method promoted/renamed in C#
reshapes the table on re-harvest, exactly the property the doc's
Future-synergy-1 promises. `lift_actions` already copies the effect
facts + kausal-depends (`lib.rs` tests
`lift_actions_carries_read_write_call_effect_facts`,
`lift_actions_depends_field_yields_kausal_depends`) — so once the Roslyn
harvester's DO-arm walk lands (P6), those facts flow through
untouched. **Until then the lift still runs** and yields name-only
ActionDefs (empty effect facts) — enough for the hot-plug capability
rows, which key on the method name + subject classid, not the body.

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
| **P3** | Run the ALREADY-SHIPPING C# chain over MedCare (Roslyn THINK-arm harvest → `ruff_csharp_spo::load` → `reassemble` → `lift_actions`, yielding name-only ActionDefs) → shape into `healthcare_actions.rs` (capabilities = the `has_function` method names) + a `domain_tables()` entry + `HEALTHCARE_{SUBJECT_CLASSIDS,EXPECTED_EXECUTORS}` | `resolve_hotplug("medcare-…", HEALTHCARE_IDS, covered)` GREEN (KILL: `NoCapabilitiesFor`/`Uncovered`) | operator |
| **P4** | `medcare-rs` `HOT_PLUG` const + activation test; executor arms | medcare activation test green against the sibling OGAR (KILL: any drift arm) | operator |
| **P5** | Loop B extended to the **healthcare** domain: MedCareV2 (C#) vs medcare-rs (Rust) parity over the harvested table | both dumps == ground truth (KILL: cross-language divergence) | operator |
| **P6** *(enrichment, orthogonal)* | Roslyn harvester **DO-arm method-body walk**: emit `reads_field`/`writes_field`/`raises`/`calls` — **mirror `ruff_cpp_spo::method_body_arm` (clang_walker.rs:873), shipped in ruff #57** — gated on `.claude/knowledge/fuzzy-recipe-codebook.md` → the healthcare ActionDefs gain effect facts + `kausal`. (C++ arm already done; C# is the sole remaining harvester arm. **Blocked on ruff write access** — see the session note.) | a known MedCare method's effect facts land in its ActionDef (KILL: facts drop / vocab break) | operator |

**Key correction (2026-07-07):** the whole C# pull-in chain already ships
(§1 item 2). The minimal hot-plug table (P3) needs **no new harvest
code** — the existing THINK-arm harvest already carries `has_function`
(capability names) + the class (subject classid), which is all
`resolve_hotplug` requires. **P1–P3 are the critical path** to a green
medcare healthcare hot-plug (P1–P2 "finish the consumer parity"; P3 the
name-only table). **P6 (the DO-arm walk) is ENRICHMENT** — it upgrades
name-only ActionDefs into projection/kausal/RBAC-bearing ones — and is
orthogonal: the hot-plug is green without it. The plan is written so
P1/P2 ship value even if P3+ is deferred, and P3 lands the table even if
P6 never does.

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
