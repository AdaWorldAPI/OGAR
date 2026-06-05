# RDF / OWL ↔ OGAR Alignment — Morris triad as the architectural axiom; OGAR as semantic-aware ingestion substrate

> **What this is.** The OGAR-side pin for *how OGAR composes with the existing
> W3C / OMG / Foundry ontology stack* — and the architectural map for the
> next phase: OGAR as an *ontology-aware ingestion substrate* (parse OWL/RDF
> /TTL via DDL adapter; auto-detect syntax / semantics / pragmatics layer;
> propose pattern-AST decomposition with confidence scoring; cache in VART
> radix-trie; testdrive in a Claude Code session across multi-domain overlays).
>
> **What this is not.** A re-derivation of the L1-L5 ontology architecture.
> That work has already been done in Woa-rs's reference docs
> (`erp_foundry_hhtl_ontology_distillation.md`, `four_way_alignment_seam.md`,
> `LANCE-GRAPH-HYDRATORS-AVAILABLE.md`) and in lance-graph PR #407 (11
> hydrators across DOLCE / OWL-Time / PROV-O / QUDT / SKOS / schema.org /
> FIBO-FND / ZUGFeRD / SKR03 / SKR04 + Tier-C `XsdHydrator` /
> `SchematronHydrator` / `SkrHydrator`). This doc declares OGAR's position
> in that stack and pins the OGAR-shaped next steps.

---

## 0. TL;DR

1. **The Morris triad (syntax / semantics / pragmatics) cuts ACROSS the
   existing L1-L5 ontology stack.** OGAR HHTL sits at the consumer end:
   inner = compile-time HHTL (semantics resolved), outer = adapter
   (pragmatics rebound per deployment), boundary = TTL serialization
   (syntax). The triad framing names what's already implicit in the L1-L5
   layering.

2. **OGAR is the runtime application-schema producer for L5** — the
   AR-pattern lift (Ruby AR / Odoo / Django / Ecto / SurrealQL DDL) into a
   unified `Class`/`Attribute`/`Association`/`EnumDecl`/`ActionDef` IR.
   `vocab/ogar.ttl` (168 OWL/RDFS declarations) + `ogar-emitter` (129 RDF
   predicates) make OGAR a *first-class OWL participant*, not a translator.

3. **The brutal upgrade** (§4): OGAR grows an **OWL / RDF / TTL DDL adapter**
   (`ogar-adapter-ttl`, companion to `ogar-adapter-surrealql`) + per-element
   **syntax / semantics / pragmatics auto-detection** + **pattern-AST
   decomposition with confidence scoring** + **VART radix-trie cache** +
   **Claude Code session as the validation harness**. The point: ingest
   FIBO / FMA / GoBD / PROV-O / schema.org / any OWL TTL — get back a
   SPO + TeKaMoLo + Morris-aware representation, NOT "just Neo4j flat
   triples".

4. **The architectural litmus is FMA bones-rendering** (§6) — ~75K static
   anatomy classes, ~2.1M relationships, perfect fit for compile-time HHTL
   (anatomy doesn't change; generate const tables once). If OGAR HHTL can
   reason about the human skeletal hierarchy in sub-millisecond, the
   architecture works. Bones → muscles → vasculature → innervation, each
   layer rebound at the per-deployment Adapter for 3D mesh / locale label
   / rendering material.

5. **Adjacent multi-hop alignment ontologies** (§8) — `odoo-to-fibo.ttl`,
   `GoBD ↔ PROV-O`, `FIBO-FND ↔ ogit:smb:Organization`, etc. — are
   **low-effort inputs** because the cross-vocab semantic work is already
   done. OGAR's contribution is consuming them at L5 and lifting them
   into the AR-pattern IR.

6. **Statistical confidence calibration** (§4.10) — a one-time
   calibration against a 4096-dim Deep-NSM encoder (Wierzbicka's
   semantic primes as the substrate) upgrades the confidence surface
   from hand-coded heuristics to statistically-grounded scores. Same
   encoder maps text *and* OWL into the same 4096-dim space; calibrated
   heads emit per-axis confidence (SPO / TeKaMoLo / Morris-layer /
   DOLCE-slot / Action-likelihood). Trained once against curated text +
   TTL corpora; every subsequent import is a const-table inference
   pass inside the firewall's inner-loop budget.

---

## 1. The existing harvest (verified, not re-derived)

| Side | Artifact | Status |
|---|---|---|
| Upstream hydrator inventory | `lance-graph` PR #407 — 11 hydrators wired (`hydrate_dolce`, `hydrate_owltime`, `hydrate_provo`, `hydrate_qudt`, `hydrate_skos`, `hydrate_schemaorg`, `hydrate_fibo_fnd`, `hydrate_zugferd`, `hydrate_zugferd_rules`, `hydrate_skr03`, `hydrate_skr04`) + Tier-C `XsdHydrator` / `SchematronHydrator` / `SkrHydrator` | **merged upstream** |
| ERP-side L1-L5 architecture | `Woa-rs/.claude/reference/erp_foundry_hhtl_ontology_distillation.md` (686 lines) — DOLCE+DUL / OWL-Time / PROV-O / QUDT / SKOS / FIBO / XBRL-GL / IFRS / UBL / ISO-20022 / SKR03/04 / HGB / UStG / **GoBD** / ZUGFeRD / Datev | distilled |
| Four-way alignment seam | `Woa-rs/.claude/reference/four_way_alignment_seam.md` (441 lines) — odoo ↔ OWL/DOLCE/FIBO ↔ OGIT ↔ lance-graph; CAM codebook family allocation; DOLCE classifier extension; Pillar-14 partial-order axioms | distilled |
| Pillar-14 runtime guarantee | `ndarray` PR #188 + `OntologySchema::is_ancestor` PR #189 | merged / in-review |
| Production code (Woa-rs) | `src/gobd.rs`; `crates/skr_data/src/{skr03,skr04}.rs` + DATEV chart-of-accounts CSV data; `WoaBridge` over OGIT `WorkOrder` namespace | shipping |
| Production code (MedCare-rs) | `OntologyRegistry` integration via `medcare_bridge::MedcareRegistry::hydrate(…)`; `MedcareOntology::from_registry(&OntologyRegistry)` building DE + EN `Arc<OntologyDto>`; `column_mask_bridge.rs` row-level perms | shipping |
| MedCare side FMA / SNOMED | `.claude/patterns.md`, `.claude/board/LATEST_STATE.md`, sprint-1 handover | **reference / plan; not yet hydrated** |
| OGAR canonical TTL | `vocab/ogar.ttl` — 444 lines, 168 OWL/RDFS declarations | shipping |
| OGAR runtime triples emit | `crates/ogar-emitter/src/lib.rs` — 129 RDF predicates emitted | shipping |
| OGAR ↔ registry seam | `crates/ogar-proposal/src/lib.rs` — `class_to_drafts(class, "ogit-op")`; owned-mirror `String` types; `lance-bind` feature with `intern` (Box-leak'd dedup'd `&'static str`) planned | designed, pending Sprint 5b |

**Do not re-derive.** This doc cites; it does not replicate.

---

## 2. The Morris triad cuts ACROSS L1-L5

Charles Morris (1938) — *syntax* (signs to signs), *semantics* (signs to
designata), *pragmatics* (signs to interpreters) — maps directly across
the layered ontology stack:

| Morris axis | L1 (Upper) | L2 (Cross-domain) | L3 (Industry) | L4 (Jurisdictional) | L5 (Operational / AR) |
|---|---|---|---|---|---|
| **Syntax** | DUL.owl serialization | OWL-Time / PROV-O / QUDT / SKOS TTL | FIBO / XBRL-GL / IFRS / UBL TTL | SKR-CSV / ZUGFeRD-XSD / Datev-XML | `vocab/ogar.ttl` ; `ogar-emitter` triples ; SurrealQL DDL |
| **Semantics** | DOLCE `Endurant`/`Perdurant`/`Quality`/`Abstract` + DnS roles | `time:Instant`/`Interval` ; `prov:Activity` ; `qudt:Unit` ; `skos:broader` | `fibo:LegalEntity` ; `fibo:MonetaryAmount` ; `xbrl-gl:entryHeader` | `skr03:1623` accounts ; `gobd:Nachvollziehbarkeit` ; `zugferd:Invoice` | OGAR `Class.inheritance` (= `rdfs:subClassOf`) ; `AssociationKind` ; `EnumDecl` ; `KausalSpec` |
| **Pragmatics** | DOLCE DnS role-binding (low-8 of CAM slot) | SKOS-prefLabel per locale ; PROV-O attribution context | FIBO module selection (FND vs BE vs Loans) | DE-jurisdictional overlay ; per-tenant chart of accounts ; locale labels | **HHTL leaf-rename at the Adapter** ; TeKaMoLo annotations ; `OntologyDto::project(…, locale)` |

**The framing claim.** The Morris triad is not a fourth axis bolted onto
the L1-L5 stack — it is the **invariant under transformation** that the
L1-L5 layering preserves. Every L is RDF/OWL-shaped (syntax); every L
adds semantic refinements (sub-classing + axioms); every L admits
pragmatic rebind at the boundary. OGAR HHTL is the consumer-end terminal:
inner = semantics fully resolved, outer = pragmatics fully rebound,
boundary = syntax serialized.

This is why `OntologyDto::project(registry, namespace, locale)` IS the
canonical Morris-pragmatic-projection in code: the registry is locale-
free (semantic); the DTO is locale-bound (pragmatic); the projection IS
the SKOS-prefLabel rebind made first-class.

---

## 3. OGAR's position in the stack

OGAR sits at the **bottom of L5** — the *runtime AR-pattern extractor +
unified application-schema IR*. It is the layer that:

- **Lifts source-language AR shapes** (Rails `belongs_to`, Odoo
  `fields.Many2one`, Django `ForeignKey`, Ecto `belongs_to`, SurrealQL
  `DEFINE FIELD … TYPE record<…>`) into a unified
  `Class`/`Association`/`Attribute`/`EnumDecl` IR.
- **Carries the lifecycle behavior** (`ActionDef` + `KausalSpec` —
  StateGuard / LifecycleTrigger / Depends / ContextDepends / External)
  that OWL doesn't model natively.
- **Emits canonical TTL** (`vocab/ogar.ttl` + `ogar-emitter`) — OGAR
  classes / fields / associations are first-class OWL `owl:Class` /
  `rdf:Property` declarations with `rdfs:subClassOf ogit:ObjectType`
  anchoring into the OGIT baseline.
- **Carries pragmatic context as first-class** — TeKaMoLo annotations
  (`ogar:actionTemporal`, `ogar:actionKausal`, `ogar:actionModal`,
  `ogar:actionLokal`) preserve the per-action context that flat triple
  stores discard.

**OGAR's three layers, named:**

```text
  ┌────────────────────────────────────────────────────────────────┐
  │  Inner = compile-time HHTL                                     │
  │    typestate, const tables, generated code                     │
  │    Identity resolution: NiblePath → typestate                  │
  │    Label resolution: HHTL leaf-rename via Adapter at build.rs  │
  │    No serde, no trait-object dispatch (per The Firewall)       │
  └─────────────────────────┬──────────────────────────────────────┘
                            │  (HHTL codegen from VART cache)
                            ▼
  ┌────────────────────────────────────────────────────────────────┐
  │  Middle = OGAR IR + VART radix-trie cache                      │
  │    Class / Attribute / Association / EnumDecl / ActionDef      │
  │    NiblePath identity (ogit-erp/sale.order/42)                 │
  │    VART-backed (timed adaptive radix trie, version-stamped)    │
  │    Ingested OWL/TTL lives here pending HHTL compilation        │
  └─────────────────────────┬──────────────────────────────────────┘
                            │  (parse / emit via DDL adapters)
                            ▼
  ┌────────────────────────────────────────────────────────────────┐
  │  Outer = DDL adapters + Membrane contracts                     │
  │    ogar-adapter-surrealql (DDL syntax — shipping)              │
  │    ogar-adapter-ttl (OWL/RDF/TTL syntax — proposed §4.1)       │
  │    ExternalMembrane / KnowableFromStore (contract surface)     │
  │    Per-deployment pragmatic rebind                             │
  └────────────────────────────────────────────────────────────────┘
```

---

## 4. The brutal upgrade — OGAR as semantic-aware ingestion substrate

The vision: any OWL / RDF / TTL dataset that lands in the workspace
gets *understood* before it is *stored*. Not "just Neo4j" — a flat
triple store discards the Morris-triad context, the SPO + TeKaMoLo
annotation surface, and the pattern-shape information that makes the
data actionable.

Nine sub-components, each scoped:

### 4.1 The OWL / RDF / TTL DDL adapter — `crates/ogar-adapter-ttl`

**Companion to `ogar-adapter-surrealql`.** Same parse / emit shape, same
"drive a real parser, validate, then walk the AST" pattern (closes the
gap toward ADR-017 for the OWL side just as the surrealql adapter
closes it for SQL).

Proposed surface:

```rust
// crates/ogar-adapter-ttl/src/lib.rs

/// Parse a TTL string into a Vec<Class>. Wraps a real TTL parser
/// (candidate: oxttl from the oxigraph project — pure Rust, OWL DL
/// compatible, used by lance-graph's ogit_to_owl_glue::Mapper).
pub fn parse_ttl(ttl: &str) -> Result<Vec<Class>, TtlParseError>;

/// Emit canonical TTL from a Class slice. Round-trips through parse_ttl
/// for any class the IR can represent.
pub fn emit_ttl(classes: &[Class]) -> String;

/// Parse a TTL file containing alignment axioms (owl:equivalentClass,
/// owl:equivalentProperty, skos:exactMatch) and return them as
/// AlignmentEdges. Consumed by §4.4 pattern recognition.
pub fn parse_alignment(ttl: &str) -> Result<Vec<AlignmentEdge>, TtlParseError>;
```

Scope: parse / emit / round-trip for OGAR's expressible subset of OWL
DL (the bits `vocab/ogar.ttl` already declares). Out of scope: SROIQ-
complete OWL 2 DL (no `owl:disjointUnion`, no nested boolean class
expressions beyond what `EnumDecl::Add` covers).

**Sprint sizing**: similar to `ogar-adapter-surrealql` — 1 sprint for
scaffold + parse / emit / round-trip; smoke tests against the existing
`vocab/ogar.ttl` as the round-trip fixture.

### 4.2 Syntax / semantics / pragmatics auto-detection

Per imported element, classify into the Morris layer:

- **Syntax-layer**: serialization format (TTL / RDF-XML / JSON-LD / N-
  Triples / N-Quads). Detected by parser dispatch.
- **Semantics-layer**: OWL / RDFS axioms found. `rdfs:subClassOf`
  produces a `Class.inheritance` edge; `rdfs:domain` + `rdfs:range`
  produce typed `Attribute` / `Association`; `owl:oneOf` produces
  `EnumDecl::Static`; `owl:unionOf` over a parent's `oneOf` produces
  `EnumDecl::Add { parent_selection }`; cardinality restrictions
  produce `AssociationKind` refinement.
- **Pragmatics-layer**: SKOS labels (`skos:prefLabel`, `skos:altLabel`,
  with locale tags `@de`/`@en`); PROV-O attribution (`prov:wasAttributedTo`);
  TeKaMoLo annotations if already present; deployment-local rendering
  conventions.

Each ingested triple gets a `MorrisLayer` tag plus the OGAR construct
it lowers to.

### 4.3 SPO + TeKaMoLo + Morris-triad representation — *not* "just Neo4j"

A flat triple store stores `(s, p, o)`. OGAR stores:

```text
SPO triple:      (subject, predicate, object)
TeKaMoLo annot.: (temporal, kausal, modal, lokal) per action
Morris layer:    {Syntax | Semantics | Pragmatics} per element
Provenance:      prov:wasGeneratedBy + prov:wasDerivedFrom (audit trail)
Confidence:      f32 — heuristic-derived vs canonical (§4.6)
DOLCE slot:      16-bit (high-8 DOLCE category + low-8 DnS role)
Identity:        NiblePath prefix-radix (ogit-erp/sale.order, etc.)
```

This is what `four_way_alignment_seam.md` already names as the
"packed schema 3-5 byte content-addressable pointer with CAM codebook
family-prefix slot". OGAR's representation lifts that surface into the
IR — every imported element retains all three Morris layers + the
DOLCE category + the provenance chain.

### 4.4 Pattern-AST recognition with confidence scoring — `crates/ogar-pattern`

When a new dataset is imported, recognize its *shape* against known
patterns. The Woa-rs four-way alignment seam already names some:

- **FMA-Pattern-D** (referenced from `pr-bO-1-dolce-hydrator.md`):
  hierarchical anatomy with `is_a` + `part_of` + `regional_part_of`
  + `bounded_by`. Anchors at FMA upper classes.
- **FIBO-FND-shape**: contracts + parties + monetary amounts + accounts.
  Anchors at `fibo-fnd:LegalEntity`, `fibo-fnd:MonetaryAmount`.
- **Schema.org-shape**: organization + person + address + product.
  Anchors at `schema:Organization`, `schema:Person`.
- **SKR03/04-shape**: hierarchical chart of accounts via `skos:broader` /
  `skos:narrower`. Detected by the SKR account-code pattern.
- **PROV-O-audit-shape**: every entity has `prov:wasGeneratedBy` + `prov:wasDerivedFrom`.
  Auto-detected as a GoBD compliance candidate.

Each pattern carries:

```rust
pub struct OntologyPattern {
    pub name: &'static str,              // "FMA-Pattern-D"
    pub anchor_classes: &'static [&'static str], // ["fma:AnatomicalEntity", ...]
    pub characteristic_properties: &'static [&'static str],
    pub recognize: fn(&[Triple]) -> RecognitionScore,
}

pub struct RecognitionScore {
    pub confidence: f32,        // 0.0..1.0
    pub matched_classes: u32,
    pub matched_properties: u32,
    pub total_triples: u32,
}
```

Confidence scoring: triple-count ratios + anchor-class hits + property-
co-occurrence frequencies. Calibrated against known-good datasets.

**Sprint sizing**: 2 sprints — one for the pattern library (10-15
patterns), one for the scoring + calibration harness.

### 4.5 Actionable-item extraction — `crates/ogar-actionable`

OWL doesn't model state machines or workflows natively. OGAR does
(`ActionDef` + `KausalSpec`). When a dataset *implies* lifecycle —
FIBO `Contract` with states `Issued → Active → Matured → Settled`;
FHIR `Encounter` with states `planned → arrived → in-progress → finished`;
Odoo `account.move` with states `draft → posted → cancelled` — OGAR
can recognize the lifecycle and emit `ActionDef` candidates.

Recognition signals:

- An `EnumDecl` with state-shaped variant names (`draft`, `posted`,
  `cancelled`, `done`, etc.).
- Properties named `*State`, `*Status`, `current_state`, etc.
- Properties pointing at controlled vocabularies of states.
- PROV-O `prov:Activity` chains.

Output: a `Vec<ActionDef>` per `Class`, each annotated with:
- Confidence score
- Source signal (which heuristic fired)
- Suggested `KausalSpec` (StateGuard inferred from state-transition
  constraints).

Human review surface: the Claude Code session loads the proposals,
reviews each, accepts / rejects / amends — closing the loop on a
human-in-the-loop import.

**Sprint sizing**: 2 sprints — one for the recognition heuristics,
one for the human-review session protocol.

### 4.6 Confidence scoring — orthogonal axis

Every imported element gets a confidence score:

- **1.0** — canonical (from an upstream-shipped hydrator, e.g.
  `hydrate_fibo_fnd` outputs).
- **0.8-0.95** — heuristic-derived but pattern-matched to a known
  shape (§4.4) with high anchor-class hit rate.
- **0.5-0.8** — heuristic-derived with partial pattern match.
- **<0.5** — best-effort guess; human review required before HHTL
  compilation.

The confidence threshold to enter the HHTL compilation surface is
**configurable per deployment**. Production may require ≥0.9; dev /
exploration may accept ≥0.5.

The `ProposalDraft.confidence` field in `ogar-proposal` (already in
place — `pub confidence: f32`) is exactly this carrier.

### 4.7 The VART radix-trie cache — the OGAR-side ontology store

VART (`AdaWorldAPI/vart` 0.9.3, via surrealkv 0.21.2) is the right
backend per multiple converging signals:

- **Append-only, version-stamped** — every import creates a new
  version; the `knowable_from` u64 in `KnowableFromStore` IS the VART
  version. Already pinned in `ogar-knowable-from` PR #25 as the
  reference backend.
- **Prefix-radix native** — matches `NiblePath` identity routing
  byte-for-byte. `ogit-erp/sale.order/42` descends the same trie as
  `ogit-op/WorkPackage/17`; cross-vocab queries traverse one index.
- **Immutable audit by construction** — fits the GoBD compliance
  requirement (§7) without additional engineering.
- **20-minute addon** — the operator's framing in the architectural
  conversation: "Vart radix trie based append in OGIT or OGAR, thats
  20 minutes addon."

Architecture:

```text
  OWL TTL import
       │
       │  ogar-adapter-ttl::parse_ttl
       ▼
  Vec<Class> + alignment edges
       │
       │  ogar-pattern::recognize + ogar-actionable::extract
       ▼
  ProposalDraft { confidence, kind, marking, … }
       │
       │  VART::insert(NiblePath, proposal, version)
       ▼
  VART radix trie — keyed by NiblePath prefix
       │                  ▲
       │  (lookup)        │  (test-drive in Claude Code session)
       ▼                  │
  HHTL build.rs ──────────┘
       │
       │  jinja / xml / const-codegen
       ▼
  Compile-time typestate (the inner HHTL surface)
```

**Sprint sizing**: 1 sprint — extend `ogar-knowable-from` with a
feature-gated `vart-backend` (reads the actual VART API first to
ground the binding).

### 4.8 Claude Code session as the validation harness

The Claude Code session IS the test-drive surface. Flow:

1. User: *"Load FIBO-FND TTL and propose OGAR Class drafts."*
2. Session: invokes `ogar-adapter-ttl::parse_ttl` → `ogar-pattern::
   recognize` → `ogar-actionable::extract` → reports proposals with
   confidence scores.
3. User: reviews. Accepts the FIBO Foundations classes; rejects a
   speculative state machine on `fibo:FinancialInstrument` (too
   speculative); amends the inferred `KausalSpec` on `fibo:Contract`.
4. Session: writes the accepted proposals into VART (`ogar-knowable-
   from::register` returns the new version).
5. Session: runs round-trip emit (`emit_ttl`) and diffs against the
   original — confirms zero structural drift.
6. Session: runs cross-vocab consistency check — does `fibo:LegalEntity`
   alignment with `ogit:smb:Organization` (per `four_way_alignment_seam.md`)
   compose correctly?
7. User: commits the version. HHTL codegen reads from VART at build
   time and generates the typestate.

This loop is the *validation gate* — nothing enters the HHTL compile-
time surface without passing through it. The session's role is the
"investigation agent" referenced in the lance-graph GLUE_LAYER spec.

### 4.9 Multi-domain overlay

A single OGAR substrate supports multiple deployments overlaying
different domain stacks:

- **Healthcare (MedCare-rs)**: FMA + SNOMED CT + LOINC + ICD + FHIR R4
  at L3; HIPAA + GDPR at L4; FHIR-resource-name labels at L5; German
  rendering labels at the i18n adapter.
- **ERP (Woa-rs)**: FIBO + XBRL-GL + IFRS + UBL + ISO-20022 at L3;
  GoBD + SKR03/04 + ZUGFeRD + HGB + UStG + Datev at L4; Odoo model
  names at L5; German tax form labels at the i18n adapter.
- **IT-ops (OGIT-canonical)**: OGIT baseline at L1-L3; per-tenant
  configuration at L4; application class names at L5.
- **Calibration set**: chess (shakmaty), OpenProject, HIRO/Elixir —
  per `docs/DOMAIN-INSTANCES.md`.

Each overlay shares L1-L3 (DOLCE / OWL-Time / PROV-O / QUDT / SKOS /
schema.org / FIBO-FND) and diverges at L4-L5. The VART cache supports
multi-tenant prefixing (`tenant/<id>/ogit-erp/…`) so overlays don't
collide.

### 4.10 Statistical confidence calibration — 4096-dim Deep NSM → SPO + TeKaMoLo + Morris

The hand-coded confidence heuristics in §4.6 are the *bootstrap*
floor — usable from day one, but not great at the long tail of
ambiguous inputs. A **one-time calibration** against a 4096-dim deep
semantic model upgrades the confidence surface from hand-coded to
*statistically calibrated*, and the calibration cost is amortised
over every subsequent import.

**The calibration target.** A 4096-dimensional Deep-NSM
(Natural-Semantic-Metalanguage-derived) feature vector per
sentence-or-clause unit, anchored on Wierzbicka's universal semantic
primes (~65 cross-linguistically stable atomic meanings —
`SOMEONE`/`SOMETHING`/`DO`/`HAPPEN`/`SAY`/`KNOW`/`THINK`/`FEEL`/
`WANT`/`GOOD`/`BAD`/`BIG`/`SMALL`/`BEFORE`/`AFTER`/`NEAR`/`FAR`/
`PART`/`KIND`/…). NSM primes provide a sparse, language-independent
semantic basis; 4096 dims accommodates the prime × composition-
context combinatorics with substantial headroom.

**Two corpora to calibrate against:**

1. **Text → SPO + TeKaMoLo** — natural-language sentences from
   business / clinical / legal corpora, paired with their hand-
   curated SPO + TeKaMoLo decomposition. Trains the projection
   *text features → OGAR representation*.
2. **OWL TTL → SPO + TeKaMoLo + Morris-layer** — existing-hydrator
   TTL outputs (FIBO-FND, schema.org, DOLCE, PROV-O), paired with
   their already-canonical Morris-layer + DOLCE classification.
   Trains the projection *OWL features → OGAR representation*.

The calibrated model then **scores any new input** (text *or* OWL
TTL) against the same OGAR target — emitting a confidence per:

- Each SPO triple (Δ from calibration manifold)
- Each TeKaMoLo annotation (which axis fired, with what weight)
- Each Morris-layer assignment (syntax/semantics/pragmatics
  probability)
- The DOLCE 16-bit slot assignment (upper + DnS role)
- **Intra-sentence semantical resolution** — for each prime-or-
  composition unit, the confidence in its anchor onto an OGAR
  `Class` / `Attribute` / `Association`.

**Architecture:**

```text
  Text-Aerius+ (text source / external interpretation pipeline)
       │                                  OWL TTL input
       │                                       │
       ▼                                       ▼
  ┌─────────────────────────────────────────────────────┐
  │  4096-dim Deep NSM encoder (calibrated, frozen)     │
  │    sentence → 4096-dim feature vector               │
  │    TTL element → 4096-dim feature vector            │
  │    (same encoder, two input modalities)             │
  └─────────────────────┬───────────────────────────────┘
                        │
                        ▼
  ┌─────────────────────────────────────────────────────┐
  │  Calibration heads (one per axis)                   │
  │    head_spo:        4096 → SPO confidence           │
  │    head_tekamolo:   4096 → TeKaMoLo confidence (×4) │
  │    head_morris:     4096 → Morris-layer prob (×3)   │
  │    head_dolce:      4096 → DOLCE-slot prob (16-bit) │
  │    head_action:     4096 → ActionDef likelihood     │
  └─────────────────────┬───────────────────────────────┘
                        │
                        ▼
  ┌─────────────────────────────────────────────────────┐
  │  ProposalDraft.confidence (per axis + composite)    │
  │  feeds §4.6 threshold gating into HHTL compilation  │
  └─────────────────────────────────────────────────────┘
```

**Why this pays off:**

- **Cross-modal consistency.** The same encoder maps text *and* OWL
  to the same 4096-dim space; calibrated heads emit scores in the
  same units. *"This FIBO TTL fragment and this contract sentence
  refer to the same semantic prime composition"* becomes a
  computable claim.
- **Statistical floor on the long tail.** Hand-coded heuristics
  (§4.4 pattern recognition) cover the canonical shapes; the
  calibrated model covers the *novel* combinations the patterns
  don't predict.
- **Confidence becomes auditable.** Every score has a calibration
  receipt — *"this Class was admitted at confidence 0.83 because the
  encoder placed it within ε of the FIBO-FND manifold and the DOLCE
  head emitted Endurant with p=0.91."* Reproducible; explainable to
  the human in the Claude Code validation loop (§4.8).
- **Pays the calibration cost ONCE.** The encoder + heads are
  trained once against the curated text + TTL corpora; the trained
  weights ship as a static asset. Every subsequent import is an
  inference pass — sub-millisecond per element on modern hardware,
  fits inside the firewall's inner-loop budget (no serialization,
  no trait dispatch) as a const-table lookup once compiled.
- **Composes with pattern recognition.** §4.4 hand-coded patterns
  remain high-precision anchors (anchor confidence = 1.0 by
  construction). The calibrated model fills the gaps with
  statistically-grounded confidence, not best-guess heuristics.

**Sprint sizing**: 2-3 sprints — one for encoder training (against
the curated corpora), one for the calibration heads + threshold
calibration, one for the inference integration into `ogar-pattern` +
`ogar-actionable` + the Claude Code validation harness.

**Out of scope for this doc**: the encoder's architecture details
(transformer / MLP / VSA — to be decided in a dedicated calibration
spec). What this doc pins is the *role* of the calibration in the
overall ingestion substrate and the *interface* it presents to the
rest of OGAR (`ProposalDraft.confidence` + per-axis confidence
fields, slotted in alongside the existing `confidence: f32`).

---

## 5. Direct OGAR ↔ OWL correspondences (the structural map)

| OGAR construct | OWL / RDFS analog | Notes |
|---|---|---|
| `Class.identity` (NiblePath) | `rdf:about` URI | Identity is the contract; labels are pragmatic |
| `Class.inheritance` | `rdfs:subClassOf` | Direct; Pillar-14 partial-order axiom (`ndarray` #188) guarantees no cycles |
| `Attribute` | `owl:DatatypeProperty` | OGAR `type_name` → `rdfs:range` of `xsd:*` type |
| `Association::BelongsTo` (owning, 1:1) | `owl:ObjectProperty` + `owl:FunctionalProperty` | Single-target on the owning side |
| `Association::HasOne` (non-owning, 1:1) | inverse of `BelongsTo` | `owl:inverseOf` |
| `Association::HasMany` (1:N) | `owl:ObjectProperty` non-functional | Multi-target |
| `Association::HasAndBelongsToMany` (M:N) | `owl:ObjectProperty` non-functional both sides | M:N via join |
| `EnumDecl::Static(items)` | `owl:oneOf` over IRIs | Direct |
| `EnumDecl::Add { items, parent_selection }` | `owl:unionOf` of parent's `oneOf` and the added items | The Odoo `selection_add` lift |
| `EnumDecl::Computed(body)` | Runtime-computed; **outside OWL DL** | Cannot enumerate at DDL time |
| `KausalSpec::StateGuard` | SHACL `sh:NodeShape` + `sh:property` over the state field | OGAR extension past OWL DL |
| `KausalSpec::LifecycleTrigger` | None — pure OGAR extension | State-machine semantics |
| `KausalSpec::Depends` | SHACL property chains; SPARQL property paths | Partial OWL coverage |
| TeKaMoLo (`actionTemporal`, `actionKausal`, `actionModal`, `actionLokal`) | DOLCE DnS role binding + Fillmore case grammar | The 16-bit DOLCE slot's low-8 (DnS role) IS the TeKaMoLo carrier |
| `Adapter::HHTL leaf-rename` | `skos:prefLabel@locale` rebind | Pragmatic-layer; per-deployment |
| `OntologyDto::project(…, locale)` | SKOS-prefLabel projection | The canonical Morris-pragmatic projection in code |

---

## 6. FMA bones-rendering as the architectural litmus

The Foundational Model of Anatomy gives OGAR HHTL its hardest design
test:

- **~75,000 classes, ~120,000 terms, ~2.1M relationships.** Pure
  structural anatomy. Static (anatomy doesn't change daily; new
  hominin discoveries arrive in years, not minutes).
- **The static property is the architectural fit.** Compile-time HHTL
  is the ONLY viable strategy at this scale; runtime trait-object
  dispatch over 75K classes is dead on arrival. Generate const tables
  once at build time, never invalidate.
- **`is_a` hierarchy** → OGAR `Class.inheritance` → `rdfs:subClassOf`.
  Direct lower.
- **`part_of`, `regional_part_of`, `constitutional_part_of`** — FMA's
  spatial mereology. Design question raised:
  - Option A: extend `AssociationKind` with `PartOf` / `RegionalPartOf`
    variants. Preserves the FMA structure verbatim; clear semantics.
  - Option B: model them as `Association` with a `kind_extension:
    Option<String>` field. Keeps the IR minimal; pushes mereology to
    the consumer.
  - **Recommendation: Option A**. Anatomy ontologies are first-class
    OGAR consumers; FMA-Pattern-D recognition (§4.4) is one of the
    cited patterns. The IR should carry the mereology explicitly.
- **`attaches_to`, `innervates`, `vascularized_by`, `bounded_by`** —
  functional / topological associations. Generic `Association` works.
- **3D mesh data — NOT in FMA**. Lives in a per-deployment Adapter
  (OpenAnatomy / Z-Anatomy / commercial). This IS the pragmatic-layer
  rebind: FMA IR is shape; mesh assets are per-deployment rendering.

**The bones-rendering pipeline**:

```text
  FMA OWL (~75K classes)
       │
       │  ogar-adapter-ttl::parse_ttl
       ▼
  Vec<Class> + part_of / is_a edges
       │
       │  ogar-pattern: FMA-Pattern-D match (high confidence)
       ▼
  VART radix-trie cache (keyed by fma/<bone>/<region>/<part>)
       │
       │  HHTL build.rs reads VART
       ▼
  Compile-time typestate (Femur is_a LongBone is_a Bone is_a AnatomicalEntity)
       │
       │  Per-deployment Adapter: 3D mesh + locale labels
       ▼
  Renderable substrate (skeleton → muscles → vasculature → organs)
```

**Litmus pass criterion**: traverse "what nerves innervate the left
biceps brachii" via the HHTL typestate in sub-millisecond without
heap allocation. If yes, the architecture works.

---

## 7. GoBD ↔ PROV-O ↔ OGAR's `commit_event` ledger

`erp_foundry_hhtl_ontology_distillation.md` pins the foundation:

> **PROV-O — Critical for GoBD compliance.** Every entity in the
> ontology needs `prov:wasGeneratedBy`, `prov:wasDerivedFrom`,
> `prov:wasAttributedTo` to satisfy the German bookkeeping audit
> trail requirement (*Nachvollziehbarkeit und Nachprüfbarkeit*).

OGAR's substrate-level answer (per `THE-FIREWALL.md` + ADR-008
`commit_event` + the Lance version log):

| GoBD requirement | OGAR substrate mechanism |
|---|---|
| *Nachvollziehbarkeit* (traceability) | `commit_event` row carries `actor`, `action`, `target_class`, `target_row_id`, `version` — a one-to-one analog of `prov:Activity` with `prov:used` (inputs) + `prov:generated` (outputs) + `prov:wasAssociatedWith` (actor) |
| *Nachprüfbarkeit* (verifiability) | Lance append-only versioned dataset; every query at `v_ref` returns the world-as-of-that-version |
| Immutability | Lance does not support row-level update; only append + tombstone. The audit log is append-only by storage design, not by policy |
| Retention | Lance versions are retained until explicit `vacuum`; retention policy is per-deployment configurable at the firewall outer-boundary |
| Cross-reference | Every `commit_event` row has a `correlation_id` linking to the originating user request, satisfying the cross-reference requirement |

**The framing claim**: GoBD compliance composes naturally with The
Firewall's outer audit substrate. *No additional GoBD-specific code*
is needed in OGAR — the `commit_event` ledger IS the GoBD compliance
surface, via the PROV-O mapping. A deployment can ship GoBD-compliant
by configuring the retention policy + enabling the `prov-o-emit`
feature on the membrane.

---

## 8. Multi-hop alignment ontologies as low-effort inputs

Several existing alignment ontologies carry **already-curated
multi-hop semantics**. These are *low-effort inputs* to the brutal
upgrade because the cross-vocab semantic work is already done — OGAR
just needs to consume the alignment axioms.

| Alignment | Hops | Effort |
|---|---|---|
| `odoo:res.partner.Company` → `fibo-fnd:LegalEntity` → `ogit:smb:Organization` | 2-hop | Low — `four_way_alignment_seam.md §1` already specifies the alignment file format |
| `gobd:Buchung` → `prov:Activity` → `xbrl-gl:entryHeader` → `fibo:Transaction` | 3-hop | Low — PROV-O / XBRL-GL / FIBO are all upstream-hydrated |
| `fhir:Patient` → `dolce:Endurant` → `dul:Person` | 2-hop | Low — DOLCE / DUL already hydrated at L1 |
| `skr03:1200` → `xbrl-gl:Account` → `fibo:Account` | 2-hop | Low — SKR03/04 already hydrated; XBRL-GL bO-9 queued |
| `fma:Femur` → `dolce:PhysicalObject` → `dul:Object` | 2-hop | Low — DOLCE classifier extends naturally to FMA classes |

Consumption: `ogar-adapter-ttl::parse_alignment` (§4.1) returns
`AlignmentEdge`s; pattern recognition (§4.4) uses them as
high-confidence anchors; the VART cache stores them with the same
NiblePath identity routing as the underlying classes.

**The leverage**: every alignment ontology that lands in OGAR adds
N-hop transitive semantics to the substrate without per-hop
engineering. The graph traversal happens through the equivalence-pair
table in the packed schema (§5 of `GLUE_LAYER_OGIT_TO_OWL_SPEC.md`).

---

## 9. Closing the loop — `ogar-proposal::lance-bind` + reverse direction

Two directions:

**OGAR → registry (existing seam, Sprint 5b)**:

```rust
// crates/ogar-proposal — existing
let drafts = class_to_drafts(&class, "ogit-op");
let proposals: Vec<MappingProposal> = drafts.into_iter()
    .map(intern_into_static_str)  // Box::leak'd dedup
    .map(MappingProposal::from)
    .collect();
ontology_registry.append_proposals(proposals);
```

**Registry → OGAR (new direction; needed for the OWL ingestion flow §4)**:

```rust
// proposed crate / trait: ImportSchemaSource
pub trait ImportSchemaSource {
    /// Read classes from an upstream OWL/TTL source via the
    /// OntologyRegistry's hydrator API. Used by the OWL ingestion flow
    /// to pull existing-hydrator output into OGAR's IR.
    fn import_classes(
        &self,
        registry: &OntologyRegistry,
        namespace: &str,
        locale: Option<&str>,
    ) -> Result<Vec<Class>, ImportError>;
}
```

This is the *reverse* of `SchemaSource` — instead of OGAR producing
proposals for the registry, the registry produces classes for OGAR.
Lets the OWL ingestion (§4.1) compose with existing hydrators
(`hydrate_fibo_fnd`, `hydrate_skr03`, etc.) without re-ingesting the
same TTL.

---

## 10. Sequencing — phase 1 (now) → phase 5

| Phase | Deliverable | Sized | Status |
|---|---|---|---|
| **1** | This doc (`docs/RDF-OWL-ALIGNMENT.md`) | 1 PR | **this PR** |
| **2** | `ogar-adapter-ttl` crate scaffold — parse / emit / round-trip against `vocab/ogar.ttl` | 1 sprint | Next OGAR PR after #25 P2 fix + surrealql AST walk |
| **3** | `ogar-knowable-from::vart-backend` feature — wires VART as the cache | 1 sprint | Concurrent with phase 2 |
| **4** | `ogar-pattern` crate — recognition library + confidence scoring + 10-15 patterns (FMA-D, FIBO-FND, schema.org, SKR, PROV-O-audit, etc.) | 2 sprints | After phase 2 |
| **5** | `ogar-actionable` crate — lifecycle extraction + `ActionDef` / `KausalSpec` proposal | 2 sprints | After phase 4 |
| **6** | Claude Code validation harness — session protocol for §4.8 | 1 sprint (mostly docs + tool wiring) | After phase 5 |
| **7** | `ImportSchemaSource` trait + reverse direction (§9) | 1 sprint | After phase 5, in coordination with lance-graph session |
| **8** | FMA hydrator on the registry side (`hydrate_fma`) — feeds the bones-rendering litmus | 2 sprints (coordination with lance-graph + medcare-rs) | After phase 7 |
| **9** | 4096-dim Deep-NSM encoder training + calibration heads (per §4.10) — corpus curation, training, threshold calibration, inference integration into `ogar-pattern` + `ogar-actionable` | 2-3 sprints | Slottable in parallel with phases 4-5 once the curated text + TTL corpora are assembled |

Each phase ships as a PR; each PR carries a green CI floor (per #29);
each phase's deliverable is verified against the cited precedents
(Woa-rs reference docs + lance-graph #407 + Pillar-14 axioms) before
merge.

---

## 11. Cross-references

### OGAR (this repo, public)

- `docs/THE-FIREWALL.md` — the inner/outer boundary; HHTL inner /
  contract outer (PR #26).
- `docs/HEALTHCARE-TRANSCODING.md` — OGAR for HIPAA / healthcare;
  §1 FHIR resources → `Class`; §3 Security Mesh (PR #28).
- `docs/DOMAIN-INSTANCES.md` — 5-domain catalogue; §0
  inherit-schema-via-contract (PR #27).
- `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` — ADR-022 The Firewall;
  reception receipts (PR #26, ADR-022 in #29).
- `docs/LANCE-GRAPH-INTEGRATION.md` — OGAR as `SchemaSource` into
  `OntologyRegistry`; §6 hydrator integration.
- `docs/SUBSTRATE-ENDGAME.md` — five-rooms architecture.
- `docs/ADAPTERS-AND-ACTORS.md` — TeKaMoLo annotation surface;
  `ogar:Action` OWL class.
- `vocab/ogar.ttl` — 168 OWL/RDFS declarations; canonical OGAR
  vocabulary.
- `crates/ogar-emitter/src/lib.rs` — 129 RDF predicates emitted.
- `crates/ogar-proposal/src/lib.rs` — owned mirror types; planned
  `lance-bind` feature; `class_to_drafts` mapping.
- `crates/ogar-knowable-from/src/lib.rs` — `KnowableFromStore` trait;
  VART named as reference backend (PR #25).
- `crates/ogar-ontology/src/lib.rs` — prefix conventions + identity
  routing (`class_identity`, `field_identity`, `association_identity`).

### Cross-session precedents (cited, not duplicated here)

- `Woa-rs/.claude/reference/erp_foundry_hhtl_ontology_distillation.md`
  — L1-L5 layered architecture; PROV-O as GoBD compliance substrate.
- `Woa-rs/.claude/reference/four_way_alignment_seam.md` — odoo ↔ OWL
  /DOLCE/FIBO ↔ OGIT ↔ lance-graph chain; CAM codebook family
  allocation; DOLCE classifier extension; Pillar-14 partial-order.
- `Woa-rs/docs/LANCE-GRAPH-HYDRATORS-AVAILABLE.md` — lance-graph
  PR #407 inventory.
- `lance-graph/.grok/GLUE_LAYER_OGIT_TO_OWL_SPEC.md` —
  `ogit_to_owl_glue::Mapper` API contract.
- `lance-graph/.claude/knowledge/ogit-owl-dolce-ontology-compartments.md`
  — 8-bit CAM codebook slot layout; DOLCE 16-bit slot.
- `lance-graph` PR #407 — the 11 wired hydrators + Tier-C `XsdHydrator`
  / `SchematronHydrator` / `SkrHydrator`.
- `ndarray` PR #188 + `OntologySchema::is_ancestor` PR #189 —
  Pillar-14 partial-order axioms (the runtime guarantee for the
  subClassOf chain across all hops).
- `bardioc` PR #17 — Rubicon Phases 1-5 (the implementation of THE-
  FIREWALL's inner state-machine surface).
- `bardioc` PR #18 + `lance-graph` PR #470 — BindSpace dissolution
  architectural delta; runtime-side cross-session triangulation.

### Upstream W3C / OMG / OASIS / domain references

- DOLCE+DUL: `http://www.ontologydesignpatterns.org/ont/dul/DUL.owl`
- W3C Time: `https://www.w3.org/TR/owl-time/rdf/time.ttl`
- W3C PROV-O: `https://www.w3.org/TR/prov-o/prov-o.ttl`
- QUDT: `http://qudt.org/2.1/schema/qudt.ttl`
- SKOS: `https://www.w3.org/2009/08/skos-reference/skos.rdf`
- schema.org: `https://schema.org/version/latest/schemaorg-current-https.ttl`
- FIBO: `https://spec.edmcouncil.org/fibo/ontology/master/latest/AboutFIBOProd.ttl`
- FMA: `http://sig.biostr.washington.edu/projects/fm/` — request through
  Bioportal: `https://bioportal.bioontology.org/ontologies/FMA`
- HL7 FHIR R4: `https://hl7.org/fhir/R4/`
- ZUGFeRD / EN 16931: UN/CEFACT CII; published by DIN /
  Bundesministerium der Finanzen.
- DATEV SKR03/04: `https://www.datev.de/web/de/datev-shop/zentrale-kontenrahmen/`
- GoBD: BMF, *Grundsätze zur ordnungsmäßigen Führung und Aufbewahrung
  von Büchern, Aufzeichnungen und Unterlagen in elektronischer Form
  sowie zum Datenzugriff.*
