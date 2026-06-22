# OGIT domain lift catalogue

> **Coverage register for the 72 NTO domains** mirrored at
> `vocab/imports/ogit/NTO/`. One row per domain. Update on every lift
> promotion. The point: future sessions never re-fetch what's already
> here.
>
> Status: **CATALOGUE v0** (2026-06-22).

## Coverage legend

| Status | Meaning |
|---|---|
| **Imported** | TTL files mirrored in `vocab/imports/ogit/NTO/<Domain>/` (every row here) |
| **Lift-tested** | `ogar-from-schema::ttl` round-trip verified on this domain's entities/attributes |
| **Cross-walked** | `Class.name` mapped to an OGAR canonical concept (`class_ids` in `ogar-vocab`) |
| **Production** | A consumer deployment exercises the lifted form (see `DOMAIN-INSTANCES.md`) |

A domain advances Imported → Lift-tested → Cross-walked → Production
left-to-right. All 72 are Imported today (just landed). MARS is the
first Lift-tested. OpenProject/Odoo/Healthcare are Production via the
existing canonical concept work but use the source-AST lift, not the
schema lift — they enter Lift-tested when their TTLs are added to the
round-trip stress test.

## How to add a new domain to the lift

1. **Verify import** — `ls vocab/imports/ogit/NTO/<Domain>/`. If
   missing (only happens for SHA bumps), `cp -r /home/user/OGIT/NTO/<Domain>/.
   vocab/imports/ogit/NTO/<Domain>/` and bump
   `vocab/imports/ogit/PROVENANCE.md`.
2. **Round-trip the domain** — add a test that walks
   `vocab/imports/ogit/NTO/<Domain>/` and asserts every TTL passes
   `parse(emit(parse(src))) == parse(src)`. Mirror the
   `all_mars_ttl_files_roundtrip` pattern in
   `crates/ogar-from-schema/src/ttl_emit.rs`.
3. **Cross-walk** — if the domain's entities have OGAR canonical
   concepts (e.g. `Auth/User` → `class_ids::PROJECT_ACTOR`), add the
   mapping table to the domain's section below.
4. **Promote** — update this row's status. Mention it in the next PR
   description so reviewers know the lift surface grew.

## Per-domain inventory

| Domain | Entities | Attributes | Verbs | Status | Notes |
|---|--:|--:|--:|---|---|
| `Accounting` | 9 | 20 | 7 | Imported | Covered conceptually via `0x02XX` commerce/ERP via Odoo lift |
| `Advertising` | 16 | 0 | 0 | Imported | |
| `Audit` | 3 | 0 | 0 | Imported | Audit-as-Lance-version (ADR-013) covers the semantics |
| `Auth` | 13 | 24 | 6 | Imported | Cross-walk to `0x0BXX` auth domain (Zitadel/Zanzibar) queued |
| `Automation` | 22 | 105 | 0 | Imported | OLD `marsNodeType` superseded by `NTO/MARS/` |
| `Botany` | 2 | 0 | 0 | Imported | |
| `ClassificationStandard` | 2 | 5 | 2 | Imported | |
| `Compliance` | 1 | 4 | 4 | Imported | |
| `Cost` | 5 | 0 | 0 | Imported | |
| `Credit` | 12 | 0 | 9 | Imported | |
| `CustomerSupport` | 7 | 31 | 2 | Imported | |
| `Data` | 1 | 1 | 0 | Imported | |
| `DataProcessing` | 2 | 6 | 0 | Imported | |
| `Datacenter` | 15 | 6 | 0 | Imported | Includes `Virtual/` (Cluster, ResourcePool — MARS Machine targets) |
| `Documents` | 3 | 6 | 0 | Imported | |
| `EmailCorrespondance` | 0 | 2 | 0 | Imported | Attributes-only |
| `Examples` | 6 | 6 | 0 | Imported | `Crow/` calibration examples |
| `Factory` | 9 | 1 | 1 | Imported | |
| `FinancialAccounting` | 6 | 7 | 0 | Imported | `AccountsPayable/` subdir |
| `FinancialMarket` | 20 | 24 | 7 | Imported | |
| `Forms` | 3 | 0 | 0 | Imported | |
| `Forum` | 22 | 1 | 4 | Imported | Covered by `class_ids::PROJECT_FORUM` etc. |
| `GeoProfile` | 1 | 11 | 0 | Imported | `Codes/` subdir — country/region codes |
| `HR` | 10 | 0 | 4 | Imported | `Recruiting/` subdir |
| `Health` | 7 | 22 | 0 | Imported | `Diagnostics/` subdir; HIPAA domain covered by `0x09XX` |
| `Healthcare` | 7 | 0 | 0 | Imported | `entities/` + `enumerations/` — namespace for `0x09XX` |
| `Knowledge` | 4 | 1 | 0 | Imported | |
| `Legal` | 2 | 3 | 1 | Imported | |
| `Location` | 4 | 7 | 0 | Imported | |
| **`MARS`** | **4** | **25** | **0** | **Lift-tested** | **XSD oracle in `_oracle/`; 15/15 tests green; round-trip enforced; see `MARS-TRANSCODING.md`** |
| `ML` | 4 | 18 | 1 | Imported | |
| `MRO` | 11 | 0 | 11 | Imported | `Aviation/` subdir |
| `MRP` | 10 | 17 | 5 | Imported | |
| `MaterialManagement` | 1 | 0 | 0 | Imported | |
| `Medical` | 0 | 0 | 0 | Imported | `namespaces/`, `sql_mirror/` — special form (non-TTL) |
| `Meteorology` | 8 | 2 | 0 | Imported | |
| `Mobile` | 7 | 40 | 0 | Imported | |
| `Network` | 27 | 0 | 0 | Imported | NetworkInterface = MARS Machine `contains` target |
| `OSLC-arch` | 2 | 0 | 0 | Imported | OSLC architecture mgmt |
| `OSLC-asset` | 2 | 0 | 9 | Imported | OSLC asset mgmt |
| `OSLC-automation` | 5 | 1 | 11 | Imported | OSLC automation domain |
| `OSLC-change` | 1 | 12 | 10 | Imported | OSLC change mgmt |
| `OSLC-core` | 19 | 19 | 35 | Imported | OSLC core vocabulary |
| `OSLC-crtv` | 12 | 17 | 9 | Imported | OSLC creative mgmt |
| `OSLC-ems` | 50 | 12 | 52 | Imported | OSLC EMS — **largest domain by entities (50) and verbs (52)** |
| `OSLC-perfmon` | 50 | 3 | 1 | Imported | OSLC perfmon — 50 entities |
| `OSLC-qm` | 5 | 0 | 15 | Imported | OSLC QM |
| `OSLC-reqman` | 2 | 0 | 7 | Imported | OSLC requirements mgmt |
| `PLM` | 2 | 0 | 0 | Imported | |
| `PTF` | 3 | 23 | 0 | Imported | |
| `Politics` | 6 | 1 | 1 | Imported | |
| `Price` | 2 | 7 | 0 | Imported | |
| `Procurement` | 5 | 4 | 0 | Imported | |
| `Project` | 2 | 0 | 0 | Imported | Covered by `0x01XX` project-mgmt (OpenProject/Redmine) |
| `Publications` | 6 | 5 | 0 | Imported | |
| `RDDL` | 2 | 1 | 2 | Imported | |
| `RL` | 1 | 7 | 0 | Imported | |
| `RPA` | 6 | 1 | 1 | Imported | |
| `Religion` | 1 | 0 | 0 | Imported | |
| `SaaS` | 10 | 12 | 0 | Imported | |
| `SalesDistribution` | 12 | 11 | 0 | Imported | |
| `Schedule` | 5 | 7 | 0 | Imported | |
| `Security` | 2 | 0 | 0 | Imported | |
| `ServiceManagement` | 17 | 42 | 0 | Imported | MARS Machine `generates` Log/Timeseries lands here |
| `SharePoint` | 0 | 2 | 0 | Imported | Attributes-only |
| `Software` | 5 | 0 | 0 | Imported | Distinct from `NTO/MARS/Software/` — this is a software-engineering vocabulary |
| `Statistics` | 1 | 0 | 0 | Imported | |
| `Survey` | 3 | 0 | 0 | Imported | |
| `Transport` | 5 | 14 | 8 | Imported | |
| `UserMeta` | 4 | 0 | 4 | Imported | |
| `Version` | 0 | 3 | 0 | Imported | Used by MARS Machine for OS version |
| `WorkOrder` | 15 | 0 | 12 | Imported | Covered conceptually by WoA (`0x0003`) |
| **TOTALS** | **549** | **599** | **241** | — | + 42 other (Medical sql_mirror, etc.) |

## Adjacent imports (not NTO)

| Path | Files | Purpose |
|---|--:|---|
| `vocab/imports/ogit/SGO/` | 508 TTLs | Upper ontology — `core/`, `ogit/`, `sgo/`. **`SGO/sgo/verbs/` is the 176-verb canonical AST predicate vocabulary** lifted by `ogar-from-schema::sgo` |
| `vocab/imports/ogit/SDF/` | 7 JSON | Standard Data Format config samples (MARS/Automation) — instance configs, not schema |
| `vocab/imports/ogit/ogit.ttl` | 1 TTL | Root ontology declaring `ogit:Entity`, `ogit:Verb`, `ogit:Attribute` |

## Provenance

All imports at OGIT SHA `d0f489fff94640fef1e6abe7eacba90a1a144579`
(2026-05-30). See `vocab/imports/ogit/PROVENANCE.md` for the re-vendor
recipe and `vocab/imports/ogit/NTO/MARS/PROVENANCE.md` for the
MARS-specific XSD-oracle provenance.
