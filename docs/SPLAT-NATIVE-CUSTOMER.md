# Splat-Native Ultrasound — OGAR's §6 FMA-Litmus Customer

> **Status:** PROPOSAL / cross-session architectural delta.
> **Authored:** 2026-06-05 (session `claude/lance-graph-ontology-review-Pyry3`,
> cross-repo).
> **Companion to:** `docs/RDF-OWL-ALIGNMENT.md` (OGAR PR #30) — specifically §6
> "FMA bones-rendering as the architectural litmus" and §10 Phase 8.
>
> **Canonical plan:** `lance-graph/.claude/plans/splat-native-ultrasound-v1.md`.
> This OGAR-side doc names splat-native ultrasound as the **explicit customer**
> of OGAR's §6 litmus + Phase 8 FMA hydration.

---

## 0. TL;DR

OGAR PR #30 §6 names FMA bones-rendering as the architectural litmus:

> "If HHTL can traverse 'what nerves innervate the left biceps brachii' in
> sub-millisecond without heap allocation, the architecture works."

**Splat-native ultrasound is the customer of that litmus.** A CPU-only
Gaussian-splat ultrasound SaMD pipeline (the canonical plan in lance-graph)
needs exactly this query at exactly this latency, on exactly this corpus
(~75K FMA classes, ~2.1M relationships), for the splat-to-splat registration
loop in stage 6 of the architecture (live splat volume ↔ FMA atlas splat
volume, via Σ-sandwich Mahalanobis ICP).

The cross-session decision (2026-06-05): the splat-native arc IS the §6
litmus passing. Not a future customer; **the contemporary customer**. The
arc lands D-SPLAT-1 through D-SPLAT-14 across 4 repos (ndarray + lance-graph
+ MedCare-rs + OGAR); OGAR's share is Phase 8 of #30 (FMA hydrate) +
KnowableFromStore registration for splat ingest classes, both already on the
OGAR roadmap.

**For OGAR the three high-leverage takeaways:**

1. **Phase 8 from PR #30 is no longer abstract.** It has a concrete customer
   waiting downstream. The TTL produced by OGAR Phase 8 is consumed by
   lance-graph D-SPLAT-8 (FMA atlas hydrator) which emits typed `FmaEntity`
   SoA + a pre-computed atlas splat volume (~150M Gaussians full body,
   ~5 GB compressed via the SH-aware palette extension in D-SPLAT-4).

2. **ADR-022 (The Firewall) is the SaMD evidence base.** The splat-native
   arc lands as a SaMD Class IIa device (Forschungstool → klinische Studie →
   Class IIa, per the user-supplied business-facing diagram). The
   audit-controls + access-controls evidence the regulator wants IS the
   ADR-022 firewall discipline + `LanceMembrane::commit_event` audit chain
   + `KnowableFromStore` registry that PRs #25/#26/#28/#31 already shipped.
   No new OGAR architecture for SaMD certification — only documentation
   that names what's already true.

3. **The §6 FMA litmus is reframed.** It was "if this can be done, the
   architecture works." With splat-native arriving, it becomes "if this
   doesn't work, the splat-native SaMD doesn't ship." Same statement,
   higher stakes — the litmus is no longer optional; the FMA atlas
   substrate IS the operational atlas-registration substrate for the
   SaMD device. Phase 8 work transitions from "demo target" to "load
   path."

---

## 1. Why OGAR is involved

The canonical plan's §11.2 work-division rule states:

> **OGAR owns the upstream ontology; lance-graph owns the runtime atlas;
> MedCare-rs owns the PHI wire.** Three firewalls, three owners.

OGAR is the **upstream ontology owner** — prepares the FMA TTL and its
alignment to the OGAR `Class` IR; lance-graph hydrates the prepared TTL
into the runtime `fma_atlas.lance` dataset + the pre-computed atlas splat
volume; MedCare-rs handles patient-identifying ingestion.

This matches the existing OGAR architecture without modification. The
splat-native arc consumes:

- **`Class` IR + `Attribute` + `Association` + `EnumDecl`** — for FMA
  classes (Bone, Muscle, Innervation, Vasculature, etc.)
- **NiblePath identity prefixes** — `ogit-fma/Bone#7474`, etc. The
  prefix-radix shape that PR #31's `register_class_knowable_from`
  canonical-identity keying assumes.
- **`KnowableFromStore::register` with the OGIT-prefixed identity** —
  per PR #31's amended C-2 framing (registry assigns; caller registers
  + DDL hint).
- **ADR-022 firewall discipline** — inner = HHTL leaf-rename at
  compile-time; outer = `ExternalMembrane`/`LazyLock`. Splat-native ingest
  enters via `LanceMembrane::commit_event` (callcenter PR #467, the
  sole-writer membrane).
- **`HEALTHCARE-TRANSCODING.md §3` Security Mesh** — splat-native PHI
  redaction inherits this directly via MedCare's `column_mask_bridge`
  (cited from §3.3 production-instance reference, PR #31).

---

## 2. What this arc adds to OGAR's roadmap

### Confirms (no new work)

- **OGAR PR #30 §10 sequencing.** Phase 8 ("OGAR-side FMA `Class` walk")
  stays as planned, but now with a named customer downstream. No timeline
  acceleration mandate; just clearer prioritization signal.
- **OGAR PR #25/#31 `KnowableFromStore` trait.** The registry-assigns
  pattern + canonical-identity keying from PR #31 are exactly what splat
  ingest needs (D-SPLAT-11 in the canonical plan). No trait extension
  required.
- **ADR-022 (The Firewall) inner/outer boundary.** Splat-native
  consumes the boundary as-is; the SaMD certification documentation
  (D-SPLAT-14) cites it directly. No firewall redesign.

### Surfaces (small, additive)

- **A new `vocab/fma-alignment.ttl`** — OWL alignment file declaring the
  FMA ↔ OGAR `Class` correspondence (per the §5 alignment table pattern
  in PR #30). Size: ~75K classes × a handful of axioms ≈ a few MB of TTL.
  Per the Phase 8 sized "2 sprints" estimate in #30 §10. **Sized within
  the existing Phase 8 budget.**
- **OGIT vocabulary extension for FMA** — `vocab/ogit-fma.ttl` declaring
  FMA-specific OGIT predicates (`isA`, `partOf`, `innervates`, `supplies`,
  etc.) per the §6 worked example. Mirrors the existing `vocab/ogit.ttl`
  pattern.

### Reframes (zero new work; framing only)

- **§6 from "demo target" → "load path".** No change to §6's prose, but the
  sequencing weight shifts: phases that previously came "before §6 demo"
  now come "before §6 splat-native production wire." The FMA bones-rendering
  customer is no longer hypothetical.
- **§10 sequencing.** Phase 8 was sized "2 sprints" in the #30 plan with no
  specific customer. With splat-native arriving as the customer, the Phase
  8 deliverables get a concrete acceptance gate: the atlas splat volume
  must round-trip through the §6 sub-millisecond HHTL traversal at the
  scale lance-graph D-SPLAT-8 expects (~75K classes + ~150M atlas
  Gaussians).

---

## 3. The litmus, restated

OGAR PR #30 §6, condensed:

> ~75K static anatomy classes, ~2.1M relationships, perfect fit for
> compile-time HHTL. The 3D mesh + locale labels live at the per-deployment
> Adapter (pragmatic-layer rebind). If HHTL can traverse "what nerves
> innervate the left biceps brachii" in sub-millisecond without heap
> allocation, the architecture works.

The splat-native arc adds: **and the live splat volume aligns to the atlas
at frame rate.**

The composed litmus, full version (the SaMD acceptance gate):

> Given a live ultrasound splat volume (~10⁶–10⁷ Gaussians per frame, ~30
> frames/s, CPU-only on a HoloLens-class device) and the FMA atlas splat
> volume (~150M Gaussians, ~5 GB on disk, region-paged), the inner loop
> must:
>
> 1. Resolve `atlas_region_id` from the live volume's dominant Gaussians
>    (NiblePath traversal — sub-millisecond per region; the original §6
>    claim).
> 2. Page the matching atlas-region Gaussians into memory (~10⁵ Gaussians
>    per region; <10ms on warm cache).
> 3. Run Σ-sandwich Mahalanobis ICP (D-SPLAT-5 in the canonical plan;
>    <100ms on the 1M × 1M scale).
> 4. Emit pose + per-Gaussian class_id back to the live volume.
> 5. Render the patient-aligned overlay (D-SPLAT-12; <33ms per frame).
>
> Total inner-loop budget: <150ms per frame, CPU-only, no GPU vendor lock.
> Sub-millisecond on step 1 (the §6 claim) is the gating threshold.

If step 1 doesn't hit sub-millisecond, the rest of the loop doesn't matter
— the splat-native SaMD device doesn't ship at clinical AR rates.

This is **the** OGAR architectural acceptance gate for the splat-native
arc.

---

## 4. Cross-references

### OGAR repo (this repo, public)

- `docs/RDF-OWL-ALIGNMENT.md` §§6, 8, 10 — the litmus + multi-hop alignment
  + sequencing.
- `docs/THE-FIREWALL.md` §7 — the HIPAA worked example (splat-native is a
  super-set: HIPAA + SaMD Class IIa).
- `docs/HEALTHCARE-TRANSCODING.md` §3, §4 — Security Mesh + label-free PII
  guarantee; splat-native inherits both via MedCare-rs's
  `column_mask_bridge` (cited §3.3 production-instance reference, PR #31).
- `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` ADR-022 — The Firewall;
  SaMD certification evidence base.
- `crates/ogar-knowable-from/src/lib.rs` — the `KnowableFromStore` trait
  consumed by splat ingest (D-SPLAT-11 in the canonical plan).

### Canonical splat-native arc

- `lance-graph/.claude/plans/splat-native-ultrasound-v1.md` — canonical plan
  (this doc's source of truth).
- `lance-graph/.claude/plans/splat-native-ultrasound-v1.md` §3.8 / §3.9 —
  D-SPLAT-8 (FMA atlas hydrator) + D-SPLAT-9 (FMA `style_recipe`) detailed
  specs.
- `ndarray/.claude/plans/splat-native-ultrasound-simd-substrate-v1.md` —
  ndarray-side companion (D-SPLAT-2 SIMD ops).
- `MedCare-rs/.claude/handovers/2026-06-05-splat-native-medcare-hipaa-wire.md`
  — MedCare-rs-side companion (D-SPLAT-10/11 HIPAA wire).

### Cross-session precedents

- **bardioc PR #17** — Rubicon kanban impl (splat actors D-SPLAT-7 consume).
- **bardioc PR #18** — `BINDSPACE_DISSOLUTION_HANDOVER.md` (runtime-side
  architectural delta; lance-graph PR #470 is the lance-graph-side pointer).
- **bardioc PR #19** — substrate-b-shadow scaffold (the dry-run / §14
  oracle pattern; **directly relevant to D-SPLAT-14 SaMD clinical-study
  validation** — same shadow pattern + offline comparator).
- **lance-graph PR #434** — unified-SoA convergence (`E-SOA-IS-THE-ONLY`);
  splat-native inherits the carrier doctrine.
- **lance-graph PR #467** — `LanceMembrane::commit_event` sole-writer
  membrane (splat ingest audit home).
- **lance-graph PR #470** (open) — BindSpace dissolution; scoped to
  cognitive-shader-driver, doesn't affect splat-native carriers per C-1 in
  PR #162.
- **MedCare-rs PR #162** — HIPAA architectural-delta handover (splat-native
  D-SPLAT-10/11 inherits the firewall + the F-1 production-instance
  reference now cited from this repo's `HEALTHCARE-TRANSCODING.md §3.3`).
- **ndarray PR #189** — `OntologySchema::is_ancestor` (FMA atlas
  NiblePath traversal uses this; sub-millisecond is the §6 acceptance gate).
- **OGAR PR #25 / #31** — `KnowableFromStore` trait; canonical-identity
  keying with the OGIT prefix.
- **OGAR PR #30** — RDF/OWL alignment; §6 litmus; §10 Phase 8 sequencing
  (this doc's anchor).

### Upstream W3C / OMG / domain references

- **FMA (Foundational Model of Anatomy)** — University of Washington;
  ~75K static anatomy classes; OWL/RDF representation; the §6 corpus.
- **PROV-O** — W3C provenance ontology; the `commit_event` audit chain
  inherits PROV-O semantics per ADR-022.
- **DICOM (Digital Imaging and Communications in Medicine)** — NEMA;
  ultrasound metadata interoperability (out of scope for v1; relevant for
  v3 Class IIa interop).
- **FHIR R4** — HL7; structural arm of the medical-imaging consumer
  (per HEALTHCARE-TRANSCODING.md §1).
- **IEC 62366 (usability) + IEC 80001 (risk) + ISO 14971 (risk mgmt)** —
  SaMD certification standards referenced by D-SPLAT-14.
- **IVD-MDR Rule 11** — EU SaMD risk classification (Class IIa for
  diagnostic-software with non-critical decision support).

---

## 5. FINDING (verified against existing OGAR + lance-graph state)

- **F-SPLAT-O-1.** The FMA TTL preparation work (Phase 8 of PR #30) is
  pre-condition for lance-graph D-SPLAT-8 (FMA atlas hydrator) per the
  canonical plan §3.8 + §4 phase P4 sequencing. The TTL → typed Rust SoA
  pipeline mirrors the existing Odoo extraction (PR #426/#432: source-
  extracted typed `OdooEntity` SoA; ~3,328 functions / 53 entities → 66
  entities). The same pattern at FMA scale is ~75K classes / ~2.1M
  relationships — 1-2 orders of magnitude larger but architecturally
  identical.

- **F-SPLAT-O-2.** The `KnowableFromStore` registry-assigns pattern (PR
  #25/#31) is splat-native-ready. Splat ingest calls `register("ogit-
  medcare/ultrasound_ingest", Some(ddl_hint))` and gets back a monotonic
  `knowable_from: u64`. Per C-2 amended (PR #162), the registry assigns,
  not the caller — same as every other ingest pattern. No trait extension
  needed for v1.

- **F-SPLAT-O-3.** The `HEALTHCARE-TRANSCODING.md §3.3` production-instance
  reference (cited via PR #31) names MedCare's `column_mask_bridge` as the
  Security Mesh production form. Splat-native ingest extends
  `SensitivityReason` with `UltrasoundRawPHI` + `UltrasoundAnonymized`
  variants but reuses the existing `RedactionMode::{Hash, Constant, Null,
  Truncate(n)}` set — no new redaction primitive owed.

## CONJECTURE (needs OGAR-session confirmation when Phase 8 lands)

- **C-SPLAT-O-1.** The FMA TTL's NiblePath identity prefix structure
  (`ogit-fma/Bone#7474`, etc.) is sub-millisecond-traversable at ~75K
  classes per the §6 litmus claim. **Open:** confirm via prototype
  traversal on real FMA TTL (not synthetic). If sub-millisecond is not
  met, the splat-native frame rate target degrades; D-SPLAT-12 (renderer)
  drops back to atlas-region-cached (warm-only) mode and accepts a 1-frame
  registration lag.

- **C-SPLAT-O-2.** The `vocab/ogit-fma.ttl` predicate set is sufficient
  for splat-native registration. **Open:** at minimum needs `isA`,
  `partOf`, `innervates`, `supplies`. Additional predicates (`adjacent_to`,
  `crosses`, `originates_from`) may be required for higher-fidelity
  registration; defer to D-SPLAT-9 review (the FMA `style_recipe` DAtom
  catalogue).

- **C-SPLAT-O-3.** The §14 oracle pattern from bardioc PR #19
  (substrate-b-shadow) is the right shape for splat-native clinical-study
  validation. **Open:** confirm by adopting the bardioc shadow pattern as
  the SaMD §14-oracle pattern in D-SPLAT-14 — same EdgeDecoder + DryRunRecorder
  + offline-comparator structure, applied to splat-native vs 2D-B-mode
  parity comparison rather than HIRO-vs-NEW parity.

---

## 6. Action items for OGAR session

When Phase 8 (FMA hydrate) work opens (per PR #30 §10 sequencing):

- [ ] Read the canonical plan:
      `lance-graph/.claude/plans/splat-native-ultrasound-v1.md` §3.8 / §3.9
      (D-SPLAT-8 / D-SPLAT-9 detailed specs).
- [ ] Read the §6 acceptance gate at the splat-native scale (§3 of this
      doc): sub-millisecond HHTL traversal + atlas-region paging + <100ms
      registration + <33ms render = <150ms per frame total.
- [ ] Confirm C-SPLAT-O-1 via prototype FMA TTL traversal (real FMA,
      not synthetic); report sub-millisecond or otherwise back to this
      session for D-SPLAT-12 fallback design.
- [ ] Author `vocab/fma-alignment.ttl` declaring FMA ↔ OGAR `Class`
      correspondence per the §5 alignment-table pattern in PR #30.
- [ ] Author `vocab/ogit-fma.ttl` with the four minimum predicates +
      whatever D-SPLAT-9 review surfaces (C-SPLAT-O-2 resolution).
- [ ] Consider adopting bardioc PR #19's substrate-b-shadow pattern as
      the SaMD §14-oracle template for D-SPLAT-14 (C-SPLAT-O-3).

---

## 7. Status

- This OGAR-side doc: filed in branch `claude/splat-native-customer` on
  this repo.
- Canonical plan in lance-graph at
  `.claude/plans/splat-native-ultrasound-v1.md`.
- Companion docs in `ndarray/.claude/plans/` and
  `MedCare-rs/.claude/handovers/`.
- **No action required of the OGAR session in the immediate term;** this
  doc names the customer for Phase 8 and the SaMD evidence base for
  ADR-022 + PR #25/#31, so future OGAR sessions can sequence Phase 8 with
  the customer in mind.

---

_End of OGAR companion v1._
