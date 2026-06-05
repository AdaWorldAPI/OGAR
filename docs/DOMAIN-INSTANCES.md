# OGAR Domain Instances — universality demonstrated, not asserted

> **Purpose.** Catalogue the concrete domains OGAR has been instantiated
> against, mapping each to the substrate capabilities it exercises. The
> "be everything later" claim (`SUBSTRATE-ENDGAME.md`) and the Foundry-
> parity argument (`§5.2` there) rest on *real* domain coverage, not
> aspiration — and two of these are **production-grade shipping
> projects**, not calibration toys.
>
> **The claim in one line:** five domains — two calibration
> (`chess`, `OpenProject`), one migration target (`Elixir-HIRO`), and
> **two production instances** (`Odoo/ERP` via Woa-rs, `HIPAA/healthcare`
> via MedCare-rs) — together exercise the full substrate capability
> surface. Universality is *demonstrated across real domains*, not
> claimed.
>
> **Privacy note.** Woa-rs and MedCare-rs are **private** AdaWorldAPI
> repos; this catalogue (in the public OGAR repo) names them + their
> domain at the architecture altitude. No private internals / PHI /
> credentials are reproduced. If the public/private boundary matters,
> these references can be genericised to "a production ERP instance" /
> "a production healthcare instance" on request.
>
> Status: **CARVED v0** (2026-06-05).

## 1. The instances

| Domain | Kind | Repo / spec | Status |
|---|---|---|---|
| **Chess** | calibration (closed-formal) | `docs/CHESS-TRANSCODING.md` + `AdaWorldAPI/shakmaty` | spec'd; merged |
| **OpenProject** | calibration (open-messy Rails) | `docs/OPENPROJECT-TRANSCODING.md` + `opf/openproject` | spec'd; merged |
| **Elixir / HIRO** | migration target (OLD stack) | `docs/ELIXIR-HIRO-PREFETCH.md` + `crates/ogar-from-elixir` | scaffold; merged |
| **Odoo / ERP** | **production instance** | `docs/ODOO-TRANSCODING.md` + **`AdaWorldAPI/Woa-rs`** (private) | shipping |
| **HIPAA / healthcare** | **production instance** | **`AdaWorldAPI/MedCare-rs`** (private) | shipping |

The first three are how the substrate is *calibrated* (chess proves the
Semantik/Syntax/Pragmatik trichotomy separates cleanly; OpenProject
proves production-Rails-AR survives; Elixir-HIRO is the migration spine).
The last two are how the substrate is *already used in anger*.

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

### 2.4 Odoo / ERP (Woa-rs) — production instance
**OGAR for Odoo, made real** — the `ogit-erp::` prefix instantiated.
Woa-rs is a production Odoo transcode (SeaORM persistence, analytic
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

Woa-rs proves the Odoo transcoding spec isn't paper: a production ERP
runs on the OGAR-shaped IR.

### 2.5 HIPAA / healthcare (MedCare-rs) — production instance
**OGAR for healthcare with HIPAA compliance.** The instance that
exercises the substrate's **Security Mesh** (the parity matrix's
"row-level permissions" row) end-to-end, and the canonical
demonstration of The Firewall (`THE-FIREWALL.md §7.2`):
- **row-level access control (inner / hot)** — palette256
  `_effectiveReaders` bitmap + Hamming-popcount bit-intersection per PHI
  access. No serialization (the firewall's inner rule); fast enough to
  gate every field read.
- **immutable audit trail (outer / firewall)** — audit-as-Lance-version
  append, serialized + signed, once per access crossing. The HIPAA legal
  requirement met by the audit-log ↔ Lance-version consolidation
  (ADR-013's pattern generalized to compliance).
- **`ExternalMembrane` + `LazyLock`** — the outer-boundary pattern
  (`crates/medcare-rbac`, `crates/medcare-analytics`) the firewall
  principle generalizes.

MedCare-rs proves the firewall split is a *requirement*, not a nicety:
HIPAA needs fast inner auth AND durable outer audit, and a real system
ships exactly that separation.

## 3. Capability coverage matrix

Which domain proves which substrate capability (Foundry-parity columns
from `SUBSTRATE-ENDGAME.md §5.2`):

| Capability | Chess | OpenProject | Elixir/HIRO | Odoo/Woa-rs | HIPAA/MedCare-rs |
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
five together cover the full surface. HIPAA/MedCare-rs is the *only* one
that hard-proves row-level perms + signed audit (the Security Mesh);
Odoo/Woa-rs is the *only* one that hard-proves money/decimal fidelity +
production `@api.depends`. The calibration trio (chess/OP/HIRO) proves
the lifecycle + migration core. **That's why all five matter** — drop
any and a capability loses its production witness.

## 4. Why the two production instances change the Foundry argument

`SUBSTRATE-ENDGAME.md §5.3` argues substrate-b is "deeper than Foundry
going OSS." The two production instances sharpen it from architecture to
evidence:

- **Foundry's pitch is "one platform, many verticals."** Substrate-b's
  answer: the verticals *already exist as independent OGAR instances* —
  an ERP (Woa-rs) and a HIPAA healthcare system (MedCare-rs) — built on
  the same `Class`/`ActionDef`/`Identity` core + the same firewall, with
  no shared application code, only the shared substrate. That's the
  "be everything" claim with two production receipts.
- **Foundry's row-level security is a platform feature you adopt.**
  Substrate-b's is a *substrate primitive* (palette256 + Hamming on the
  inner hot path) that a HIPAA system already depends on — proven under
  a real compliance regime, not a sales demo.
- **Different storage per deployment** (the §5.3.3 pluggability point):
  Woa-rs uses SeaORM; the substrate-b reference uses Lance; MedCare-rs
  uses its own membrane backend. Same contract (`ExternalMembrane` /
  `KnowableFromStore`), different backends — exactly the firewall's
  outer-boundary pluggability, demonstrated across instances.

## 5. Cross-references

- `docs/THE-FIREWALL.md` §7 — the precedent + the HIPAA firewall worked example.
- `docs/SUBSTRATE-ENDGAME.md` §5.2 (Foundry parity), §5.3 (the three differentiators), Room 3 (OP-as-operator-pane).
- `docs/ODOO-TRANSCODING.md` — the Odoo spec; Woa-rs is its production instance.
- `docs/CHESS-TRANSCODING.md`, `docs/OPENPROJECT-TRANSCODING.md`, `docs/ELIXIR-HIRO-PREFETCH.md` — the calibration set.
- `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` — ADR-013 (paper-trail/audit consolidation), ADR-022 (the firewall).
- Private instances: `AdaWorldAPI/Woa-rs` (Odoo/ERP), `AdaWorldAPI/MedCare-rs` (HIPAA/healthcare).

## 6. Doc lifecycle

- **Author:** OGAR session, 2026-06-05.
- **Update cadence:** when a new domain is instantiated against OGAR, add
  a row to §1 + a §2 subsection + a §3 matrix column. The capability
  matrix is the "what does this domain newly prove" check.
- **Privacy:** Woa-rs / MedCare-rs references are architecture-altitude
  only; genericise on request.
