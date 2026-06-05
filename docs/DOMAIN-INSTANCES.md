# OGAR Domain Instances — universality demonstrated, not asserted

> **Purpose.** Catalogue the concrete domains OGAR has been instantiated
> against, mapping each to the substrate capabilities it exercises. The
> "be everything later" claim (`SUBSTRATE-ENDGAME.md`) and the Foundry-
> parity argument (`§5.2` there) rest on *real* domain coverage, not
> aspiration — and two of these are **production-grade shipping
> deployments**, not calibration toys.
>
> **The claim in one line:** five domains — two calibration
> (`chess`, `OpenProject`), one migration target (`Elixir-HIRO`), and
> **two production instances** (a production `Odoo/ERP` deployment, a
> production `HIPAA/healthcare` deployment) — together exercise the full
> substrate capability surface. Universality is *demonstrated across real
> domains*, not claimed.
>
> **Generic labels by design — "inherit schema via contract".** Domain
> instances are named by their *domain* (ERP, healthcare/HIPAA), never by
> concrete project. That's not just hygiene — it *is* the architecture: a
> deployment **inherits the schema shape from the contract** (`Class` /
> `ActionDef` / `KnowableFromStore` / `ExternalMembrane`) and **rebinds
> the concrete labels** via the `Adapter` pattern (`ADAPTERS-AND-ACTORS.md`
> §2 — HHTL leaf renames like `move → transport`). The label (the project
> name, the prefix string, the field captions) is a **consumer property**,
> not an architectural constant — the consumer changes it at will. Two
> deployments in the same domain share the contract-inherited shape and
> differ *only* in their rebound labels. So this catalogue names
> *domains*; the concrete instance is whatever a consumer labels it. The
> genericisation here is the worked example: you can't tell which project
> it is, because the architecture doesn't depend on which project it is.
>
> **This is also a confidentiality property, not just hygiene.** Because
> the contract carries the schema *shape* and never the labels, a
> deployment's **PII field captions** (e.g. a healthcare deployment's
> German field labels) are consumer-bound via the `Adapter` and **never
> enter OGAR's contract surface**. The substrate holds "there is a
> protected field here, with these access controls," not "the field is
> called `<PII caption>`." For PII / GDPR / HIPAA that's a *guarantee by
> construction*: the labels can't leak through OGAR because OGAR never
> holds them. (The firewall's outer boundary — `THE-FIREWALL.md` — is
> where a consumer's labelled schema is read; it stays consumer-side.)
>
> Status: **CARVED v0** (2026-06-05).

## 1. The instances

| Domain | Kind | Spec / instance | Status |
|---|---|---|---|
| **Chess** | calibration (closed-formal) | `docs/CHESS-TRANSCODING.md` + `AdaWorldAPI/shakmaty` | spec'd; merged |
| **OpenProject** | calibration (open-messy Rails) | `docs/OPENPROJECT-TRANSCODING.md` + `opf/openproject` | spec'd; merged |
| **Elixir / HIRO** | migration target (OLD stack) | `docs/ELIXIR-HIRO-PREFETCH.md` + `crates/ogar-from-elixir` | scaffold; merged |
| **Odoo / ERP** | **production instance** | `docs/ODOO-TRANSCODING.md` + a production ERP deployment | shipping |
| **HIPAA / healthcare** | **production instance** | a production healthcare (HIPAA) deployment | shipping |

The first three are how the substrate is *calibrated* (chess proves the
Semantik/Syntax/Pragmatik trichotomy separates cleanly; OpenProject
proves production-Rails-AR survives; Elixir-HIRO is the migration spine).
The last two are how the substrate is *already used in anger* — named by
domain, per §0.

## 2. Per-instance — what each exercises

### 2.1 Chess (`shakmaty`) — closed-formal calibration
The cleanest separation of Semantik / Syntax / Pragmatik (per
`CHESS-TRANSCODING.md §0`): finite vocabulary (12 pieces × 64 squares),
published bijective notation (FEN/SAN/UCI/PGN), and a free §14 oracle
(`shakmaty::Position::play`). Exercises: lifecycle FSM (`ActionState`),
`Postpone` (premove), `StateTimeout` (clock), `on_enter` (move
application). The calibration target — pass here and the substrate's
core is sound.

### 2.2 OpenProject — open-messy production-Rails calibration
Concerns, `acts_as_*`, STI, polymorphic associations, data-driven FSMs
(`Workflow` table), `has_paper_trail` (→ Lance-version consolidation).
Exercises: the full structural arm + the database-hydrator pattern
(ADR-014) + paper-trail-as-audit. The destination is OP-as-operator-pane
(`SUBSTRATE-ENDGAME.md` Room 3).

### 2.3 Elixir / HIRO — the OLD-stack migration target
`gen_statem` lifecycles, GenServer/Phoenix/Oban actions, Ecto schemas.
The load-bearing `gen_statem`→Rubicon case (`ELIXIR-HIRO-PREFETCH.md
§2.2`). Exercises: the migration scaffold (`SUBSTRATE-ENDGAME.md`
Room 2) + the wire-roundtrip §14 oracle. The reason `ogar-from-elixir`
exists.

### 2.4 Odoo / ERP — production instance
**OGAR for Odoo, made real** — the `ogit-erp::` prefix instantiated. A
production ERP deployment transcodes Odoo (SeaORM persistence, analytic
accounting, HR, the ERP money/decimal model). Exercises the substrate's:
- **structural arm** — Odoo models → `Class` (the `ODOO-TRANSCODING.md`
  mapping in production).
- **behavioral arm** — Odoo `@api.depends` computed fields →
  `KausalSpec::Depends` (the data-causal guard); workflow transitions →
  `ActionState` lifecycle.
- **enum/selection handling** — Odoo `selection` + `selection_add` →
  `EnumSource::{Static, Add}` (the inheritance-aware enum the SurrealQL
  emitter handles).
- **money/decimal precision** — the ERP correctness constraint
  (decimals, not floats) — a real-world data-fidelity requirement.

A production ERP deployment proves the Odoo transcoding spec isn't
paper: real ERP runs on the OGAR-shaped IR. (Which project, and what
it's labelled, is a consumer detail — per §0.)

### 2.5 HIPAA / healthcare — production instance
**OGAR for healthcare with HIPAA compliance.** A production healthcare
deployment exercises the substrate's **Security Mesh** (the parity
matrix's "row-level permissions" row) end-to-end, and is the canonical
demonstration of The Firewall (`THE-FIREWALL.md §7.2`):
- **row-level access control (inner / hot)** — palette256
  `_effectiveReaders` bitmap + Hamming-popcount bit-intersection per PHI
  access. No serialization (the firewall's inner rule); fast enough to
  gate every field read.
- **immutable audit trail (outer / firewall)** — audit-as-Lance-version
  append, serialized + signed, once per access crossing. The HIPAA legal
  requirement met by the audit-log ↔ Lance-version consolidation
  (ADR-013's pattern generalized to compliance).
- **`ExternalMembrane` + `LazyLock`** — the outer-boundary pattern the
  firewall principle generalizes.

A production HIPAA deployment proves the firewall split is a
*requirement*, not a nicety: a real HIPAA-compliant system needs fast
inner auth AND durable outer audit, and ships exactly that separation.

## 3. Capability coverage matrix

Which domain proves which substrate capability (Foundry-parity columns
from `SUBSTRATE-ENDGAME.md §5.2`):

| Capability | Chess | OpenProject | Elixir/HIRO | Odoo/ERP | HIPAA |
|---|:--:|:--:|:--:|:--:|:--:|
| Ontology (Class/Association) | ✓ | ✓ | ✓ | ✓ | ✓ |
| Action types / lifecycle FSM | ✓ | ✓ | ✓ (gen_statem) | ✓ (workflows) | ✓ |
| `Postpone` / `StateTimeout` | ✓ (premove/clock) | partial | ✓ | — | — |
| `Depends` (data-causal) | — | ✓ (reactive) | ✓ | ✓ (`@api.depends`) | — |
| Time-versioned / time-travel | ✓ | ✓ (paper-trail) | ✓ | ✓ | ✓ (audit) |
| **Row-level permissions** | — | partial (RBAC) | — | partial | **✓ (HIPAA, palette256)** |
| **Immutable audit** | — | ✓ (journals) | — | ✓ | **✓ (HIPAA, signed)** |
| Multi-language frontends | (Rust) | Ruby | Elixir | Python (Odoo)+SeaORM | Rust |
| Money/decimal fidelity | — | — | — | **✓ (ERP)** | — |
| Migration scaffold | — | ✓ (target) | ✓ (spine) | (already Rust) | (already Rust) |

**Coverage observation:** no single domain exercises everything, but the
five together cover the full surface. The HIPAA instance is the *only*
one that hard-proves row-level perms + signed audit (the Security Mesh);
the Odoo/ERP instance is the *only* one that hard-proves money/decimal
fidelity + production `@api.depends`. The calibration trio
(chess/OP/HIRO) proves the lifecycle + migration core. **That's why all
five matter** — drop any and a capability loses its production witness.

## 4. Why the two production instances change the Foundry argument

`SUBSTRATE-ENDGAME.md §5.3` argues substrate-b is "deeper than Foundry
going OSS." The two production instances sharpen it from architecture to
evidence:

- **Foundry's pitch is "one platform, many verticals."** Substrate-b's
  answer: the verticals *already exist as independent OGAR instances* —
  an ERP deployment and a HIPAA healthcare deployment — built on the
  same `Class`/`ActionDef`/`Identity` core + the same firewall, with no
  shared application code, only the shared substrate. That's the "be
  everything" claim with two production receipts.
- **Foundry's row-level security is a platform feature you adopt.**
  Substrate-b's is a *substrate primitive* (palette256 + Hamming on the
  inner hot path) that a HIPAA system already depends on — proven under
  a real compliance regime, not a sales demo.
- **Different storage per deployment** (the §5.3.3 pluggability point):
  the ERP deployment uses SeaORM; the substrate-b reference uses Lance;
  the healthcare deployment uses its own membrane backend. Same contract
  (`ExternalMembrane` / `KnowableFromStore`), different backends —
  exactly the firewall's outer-boundary pluggability, demonstrated
  across instances. And the *labels* differ across all of them while the
  *contract-inherited schema* is shared — the "inherit schema via
  contract" pattern (§0) at deployment scale.

## 5. Cross-references

- `docs/THE-FIREWALL.md` §7 — the precedent + the HIPAA firewall worked example.
- `docs/SUBSTRATE-ENDGAME.md` §5.2 (Foundry parity), §5.3 (the three differentiators), Room 3 (OP-as-operator-pane).
- `docs/ODOO-TRANSCODING.md` — the Odoo spec; its production instance is an ERP deployment (§2.4).
- `docs/ADAPTERS-AND-ACTORS.md` §2 — the `Adapter` HHTL leaf-rename pattern (the consumer-rebindable label mechanism behind §0's "inherit schema via contract").
- `docs/CHESS-TRANSCODING.md`, `docs/OPENPROJECT-TRANSCODING.md`, `docs/ELIXIR-HIRO-PREFETCH.md` — the calibration set.
- `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` — ADR-013 (paper-trail/audit consolidation), ADR-022 (the firewall).

## 6. Doc lifecycle

- **Author:** OGAR session, 2026-06-05.
- **Update cadence:** when a new domain is instantiated against OGAR, add
  a row to §1 + a §2 subsection + a §3 matrix column. The capability
  matrix is the "what does this domain newly prove" check.
- **Labels:** domain instances are named by *domain*, not project (per
  §0 — the concrete label is consumer-rebindable via the `Adapter`
  contract; the architecture doesn't depend on it).
