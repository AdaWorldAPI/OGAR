# Healthcare Transcoding — OGAR for the HIPAA / healthcare domain (FHIR-grounded, label-free)

> **Purpose.** The healthcare-domain `Class`/`ActionDef`/`Identity`
> mapping — the AST/OGAR for healthcare, completing the pair with
> `ODOO-TRANSCODING.md` (ERP). Grounded in the **public FHIR R4 / HL7
> standard + the HIPAA regulations**, *never* in any private deployment's
> schema. That grounding is deliberate: the healthcare production
> instance is PII-laden, so the spec is written against the public
> standard — and is **label-free by construction**, which is itself the
> point (§4).
>
> **Label-free, on purpose.** This doc maps healthcare *shapes* (a
> date-typed protected field exists, with these access controls), never
> *labels* (what the field is called — FHIR `Patient.birthDate`, or a
> deployment's localized caption in any language). Per
> `DOMAIN-INSTANCES.md §0` + `THE-FIREWALL.md §7`, the OGAR contract
> holds shape; the consumer binds labels via the `Adapter`. PII captions
> never enter OGAR's surface. This spec demonstrates that: you can map
> the entire healthcare domain to OGAR without naming a single PHI value
> or deployment caption.
>
> **Grounding sources (all public):** FHIR R4 resource model
> (hl7.org/fhir), HIPAA Privacy + Security Rules (minimum-necessary
> access, audit controls), the substrate's Security Mesh
> (palette256 + Hamming popcount, per the parity matrix).
>
> Status: **CARVED v0** (2026-06-05).

## 0. The trichotomy for healthcare

| OGAR layer | Healthcare (public FHIR) | OGAR carrier |
|---|---|---|
| **Semantik** | FHIR resource types (Patient, Encounter, Observation, Condition, MedicationRequest, …) + their standard elements | `Class` / `Attribute` / `Association` / `EnumDecl` |
| **Syntax** | FHIR wire formats (JSON / XML), HL7 v2 messages, the resource-element grammar | the `Adapter` HHTL — FHIR-JSON ↔ canonical, HL7v2 ↔ canonical |
| **Pragmatik** | clinical workflow (admit → order → result → discharge), who-may-read (role + relationship + consent), when (encounter context), audit obligation | `ActionInvocation` SPO + TeKaMoLo + lifecycle + the Security Mesh |

The healthcare twist vs. ERP/Rails: **the Pragmatik layer is
compliance-load-bearing** — access control and audit aren't features,
they're legal requirements. That's why healthcare is the canonical
firewall case (§4).

## 1. Structural arm — FHIR resources → `Class`

Each FHIR resource type maps to a `Class` under the `ogit-fhir::` prefix
(a deployment may rebind to its own prefix + localized labels via the
`Adapter` — the label-free property):

| FHIR resource (public type) | OGAR mapping |
|---|---|
| `Patient` | `Class { identity: "ogit-fhir::Patient", attributes: [<shape only — identifiers, demographics as typed fields>], marking: PHI }` |
| `Encounter` | `Class` + `Association { kind: BelongsTo, target: Patient }` |
| `Observation` (labs, vitals) | `Class` + `Association → Patient/Encounter`; value as typed `Attribute` |
| `Condition` (diagnoses) | `Class` + `Association → Patient` |
| `MedicationRequest` | `Class` + `Association → Patient/Practitioner` |
| `Practitioner` / `Organization` | `Class` (non-PHI directory data) |
| `Consent` | `Class` — drives the Security Mesh (who may read what) |
| `AuditEvent` | **not a `Class`** — it IS the Lance version log (§3.2) |

**Shape, not labels.** The `attributes` carry *types + markings*
(`date`, `code`, `string`, + a `PHI` marking), never the concrete
caption. A protected birth-date field is `Attribute { type: date,
marking: PHI }` — the OGAR contract knows "a PHI date field exists
here"; the caption (`birthDate`, or a deployment's localized label)
lives consumer-side via the `Adapter`. Same `Marking` mechanism the
`lance-graph-contract` already exposes.

## 2. Behavioral arm — clinical workflow → `ActionDef`

| Clinical action (public workflow) | `ActionDef` projection |
|---|---|
| Admit patient | `predicate="admit"`, `subject=User(clinician)`, `modal=Atomic`, `on_enter=EnterEffect{status, "in-progress"}` |
| Place order (lab/med) | `predicate="order"`, `kausal=StateGuard{encounter.status, ["in-progress"]}`, `modal=Atomic` |
| Result available | `predicate="result"`, `subject=System/Trigger`, `temporal=OnCommit` |
| Amend record | `predicate="amend"`, `modal=Atomic` — the prior version stays in the Lance log (immutable audit) |
| Discharge | `predicate="discharge"`, `on_enter=EnterEffect{status, "finished"}`, terminal |
| Break-glass access | `predicate="emergency_access"`, `subject=User`, `guard_failure_policy=Reject` unless emergency context — and **always audited** |

The `Encounter.status` lifecycle (planned → in-progress → finished →
cancelled) is the per-class domain workflow; the `ActionState`
(Pending → Committed/Failed/Cancelled) is the universal lifecycle — same
two-level split as everywhere (ADR-001).

## 3. The Security Mesh — HIPAA's two requirements, the firewall's two sides

This is where healthcare *is* the canonical firewall case
(`THE-FIREWALL.md §7.2`):

### 3.1 Minimum-necessary access — inner / hot, no serialization
HIPAA requires every PHI access be authorized (role + treatment
relationship + consent). Mapped to the substrate's **Security Mesh**:
per-row `_effectiveReaders` as a palette256 / Binary16K bitmap; the
auth check is a Hamming-popcount bit-intersection. This is **inner /
hot** — it gates every PHI field read, so it must be a bit-op, not a
serialized permission lookup. `Consent` resources + role + relationship
compose the `_effectiveReaders` bitmap (materialized at the boundary;
checked on the hot path).

### 3.2 Immutable audit trail — outer / firewall, serialized
HIPAA requires a tamper-evident record of every PHI access + change.
Mapped to **audit-as-Lance-version**: every access/mutation is a Lance
version append (the FHIR `AuditEvent` is a *projection* of the version
log, not a separate write — ADR-013 generalized to compliance). This is
**outer / firewall** — serialized + signed, once per access crossing.
The audit signature is the literal "crypto on the post stamp"
(`THE-FIREWALL.md §3`).

**The tension is the firewall:** auth must be fast (every field read) +
audit must be durable + tamper-evident (legal). Inner bit-op auth +
outer signed audit-append is the only way to have both — and a real
HIPAA system ships exactly that.

## 4. The label-free contract IS the PII guarantee

The reason this entire spec can map the healthcare domain without
naming a single PHI value or deployment caption: **OGAR holds schema
shape, never labels.**

- The contract knows: "Patient has a PHI date field, readable by
  `_effectiveReaders`, audited on access."
- The contract does *not* know: what that field is called in the
  deployment's UI (FHIR `birthDate`, or a localized caption in any
  language).
- The label is consumer-bound via the `Adapter` (HHTL leaf rename,
  `ADAPTERS-AND-ACTORS.md §2`) and read at the firewall's outer boundary
  (`ExternalMembrane` / `LazyLock`), staying consumer-side.

**Consequence (compliance-grade):** PII field captions cannot leak
*through* OGAR, because OGAR never holds them. For GDPR/HIPAA that's a
guarantee by construction — the substrate is PII-label-free, and this
doc is the proof (a complete healthcare mapping with zero PII labels).

## 5. §14 oracle for healthcare

The §14 acceptance gate (`OracleSubstrate`) for healthcare:
- **resource round-trip** — a FHIR resource → `Class`/`ActionInvocation`
  → Lance commit → read-back → same resource (provenance-normalized:
  strip server-assigned ids, timestamps).
- **access-control equivalence** — for a given (subject, resource,
  consent) the OGAR `_effectiveReaders` bit-op yields the same
  allow/deny as the reference FHIR access-control evaluation.
- **audit completeness** — every access produces exactly one Lance
  version row; the projected `AuditEvent` set equals the reference.

All against the *public* FHIR reference behavior — never a private
deployment's data.

## 6. Capability coverage (ties to `DOMAIN-INSTANCES.md §3`)

Healthcare is the domain that hard-proves the substrate's
**row-level permissions** + **signed immutable audit** columns (the
Security Mesh) — the only domain in the matrix that does. It also
exercises ontology + lifecycle + time-travel (audit history). It does
*not* exercise `Postpone`/`StateTimeout` (no premove/clock analog) or
money/decimal fidelity (that's the ERP domain's witness).

## 7. Cross-references

- `docs/DOMAIN-INSTANCES.md` — the domain catalogue; §2.5 (healthcare), §0 (the label-free / inherit-schema-via-contract property).
- `docs/THE-FIREWALL.md` §7.2 — healthcare as the canonical firewall demonstration (inner palette256 auth + outer signed audit).
- `docs/ODOO-TRANSCODING.md` — the ERP-domain pair (the other half of "AST/OGAR for both").
- `docs/OGAR-AST-CONTRACT.md` — the typed surface healthcare lowers onto.
- `docs/ADAPTERS-AND-ACTORS.md` §2 — the `Adapter` label-rebinding mechanism (how a deployment binds its own captions).
- `lance-graph-contract` — `Marking` (the PHI marking), `ExternalMembrane` (the outer-boundary consumer-schema read).
- Public grounding: FHIR R4 (hl7.org/fhir), HIPAA Privacy + Security Rules.

## 8. Doc lifecycle

- **Author:** OGAR session, 2026-06-05.
- **Grounding:** public FHIR/HL7 + HIPAA only. **No private-deployment
  content.** Any future enrichment must stay label-free (shape, not
  captions) — the PII guarantee depends on it.
