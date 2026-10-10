# OGIT domain lift catalogue

> **Coverage register for the NTO domains** of AdaWorldAPI/OGIT. OGAR keeps
> no copy of OGIT: it reads a checkout at OGIT's moving `master`, named by
> `OGIT_FORK_PATH` or found as `OGIT` next to OGAR. One row per domain.
> Update on every lift promotion.
>
> Status: **CATALOGUE v0** (2026-06-22). Every count in this file was
> measured on the last vendored snapshot, `d0f489f` (2026-05-30), which had
> 72 domains. `master` at `2315167d` (2026-10-10) has 77: `Academy`, `GFS`,
> `Integration`, `PublicAdministration` and `Utilities` have no row yet.

## Coverage legend

| Status | Meaning |
|---|---|
| **Imported** | TTL files present in OGIT's `NTO/<Domain>/`, read from the checkout (every row here) |
| **Lift-tested** | `ogar-from-schema::ttl` round-trip verified on this domain's entities/attributes |
| **Cross-walked** | `Class.name` mapped to an OGAR canonical concept (`class_ids` in `ogar-vocab`) |
| **Production** | A consumer deployment exercises the lifted form (see `DOMAIN-INSTANCES.md`) |

A domain advances Imported → Lift-tested → Cross-walked → Production
left-to-right. All 72 rows are Imported. The following
**10 domains are Lift-tested** (round-trip mechanically enforced by
`ttl_emit::tests::nine_domains_lift_surface_round_trip` +
`all_mars_ttl_files_roundtrip`): MARS, Transport, Accounting,
SalesDistribution, Credit, Cost, ServiceManagement, WorkOrder,
Compliance, Audit. OpenProject/Odoo/Healthcare are Production via the
existing canonical concept work but use the source-AST lift, not the
schema lift — they enter Lift-tested when their TTLs are added to the
round-trip stress test.

## Verifying domain authorship (who can change what)

Provenance is `dcterms:creator` on each TTL. Run:

```bash
python3 - <<'PY'
import os, re
from collections import Counter
creator_re = re.compile(r'dcterms:creator\s+"([^"]+)"')
nto = os.path.join(os.environ.get('OGIT_FORK_PATH', '../OGIT'), 'NTO')
for d in sorted(os.listdir(nto)):
    root = os.path.join(nto, d)
    authors = Counter()
    for r,_,fs in os.walk(root):
        for f in fs:
            if not f.endswith('.ttl'): continue
            with open(os.path.join(r,f)) as fh:
                for m in creator_re.finditer(fh.read()):
                    authors[m.group(1)] += 1
    if not authors: continue
    top = ', '.join(f'{a} ({c})' for a,c in authors.most_common(5))
    print(f'{d:<28} {top}')
PY
```

Internal-agent authors (`bus-compiler`, `family-codec-smith`, `Claude
(...)`, etc.) signal "our extension — we can revise without external
coordination." External authors (`chris.boos@almato.com`, `Viktor Voss`,
`fotto@arago.de`, …) signal "upstream-owned — structural changes need
arago/almato coordination."

## How to add a new domain to the lift

1. **Verify the domain is in OGIT** — `ls ../OGIT/NTO/<Domain>/` (or
   under `OGIT_FORK_PATH`). If it is missing, it lands in AdaWorldAPI/OGIT
   first; OGAR keeps no copy to add it to.
2. **Round-trip the domain** — add a test that walks the checkout's
   `NTO/<Domain>/` and asserts every TTL passes
   `parse(emit(parse(src))) == parse(src)`. Mirror the
   `all_mars_ttl_files_roundtrip` pattern in
   `crates/ogar-from-schema/src/ttl_emit.rs`.
3. **Cross-walk** — if the domain's entities have OGAR canonical
   concepts (e.g. `Auth/User` → `class_ids::PROJECT_ACTOR`), add the
   mapping table to the domain's section below.
4. **Promote** — update this row's status. Mention it in the next PR
   description so reviewers know the lift surface grew.

> **Lift-tested → Cross-walked is demand-driven and ownership-gated, NOT a
> completeness sweep** (decision 2026-06-23, `.claude/board/EPIPHANIES.md`
> E-NINE-DOMAIN-PROMOTION-DEFERRED). Round-trip (Lift-tested) proves the
> *shape lands*; it does NOT imply the *id should mint*. A domain earns a
> `class_ids` codebook id (stable forever, P0 canon) only when **(a)** a
> consumer needs to `authorize()`/route on it AND **(b)** we own it or have
> arago/almato coordination for an upstream-owned domain. The nine
> Lift-tested domains are correctly parked un-Cross-walked: most are
> upstream-owned (coordination-gated), Accounting/Audit are already homed
> (`0x02XX` / ADR-013), and WorkOrder (ours) waits on woa-rs's
> consumer-collapse. See the per-domain gate table in that epiphany.

## Per-domain inventory

| Domain | Entities | Attributes | Verbs | Status | Notes |
|---|--:|--:|--:|---|---|
| `Accounting` | 9 | 20 | 7 | Lift-tested | Mixed-authorship: `Viktor Voss` / `Gibson Xavier` / `Moritz Vannahme` (25 files, original arago/almato) + a prior session's extension (`Claude (AdaWorldAPI/lance-graph 3-hop optim)`, 11 files **promoted to the OGIT fork** via commit `c5dc1b8`). The 11 are a completed promotion (fork → mirrored here), the worked example of the staging-tier model (`vocab/exports/PROVENANCE.md`), NOT stranded content. Covered conceptually via `0x02XX` commerce/ERP via Odoo lift. Structural changes to the upstream files need arago coordination; the 11 OGAR-promoted ones are ours. |
| `Advertising` | 16 | 0 | 0 | Imported | |
| `Audit` | 3 | 0 | 0 | Lift-tested | `Marek Meyer` (sole author) — pure upstream. Audit-as-Lance-version (ADR-013) covers the semantics. |
| `Auth` | 13 | 24 | 6 | Imported | Cross-walk to `0x0BXX` auth domain (Zitadel/Zanzibar) queued |
| `Automation` | 22 | 105 | 0 | Imported | OLD `marsNodeType` superseded by `NTO/MARS/` |
| `Botany` | 2 | 0 | 0 | Imported | |
| `ClassificationStandard` | 2 | 5 | 2 | Imported | |
| `Compliance` | 1 | 4 | 4 | Lift-tested | `chris.boos@almato.com` (sole author) — pure upstream |
| `Cost` | 5 | 0 | 0 | Lift-tested | `Peter Larem` (sole author) — pure upstream |
| `Credit` | 12 | 0 | 9 | Lift-tested | `Ola Irgens Kylling` (sole author, 21 files) — pure upstream; capitalised `Entities/` + `Verbs/` dirs (content-driven parser is dir-case-agnostic) |
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
| **`MARS`** | **4** | **25** | **0** | **Lift-tested** | **XSD oracle in `vocab/oracles/mars/`; 15/15 tests green; round-trip enforced; see `MARS-TRANSCODING.md`** |
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
| `SalesDistribution` | 12 | 11 | 0 | Lift-tested | `Marek Meyer` (sole author, 23 files) — pure upstream |
| `Schedule` | 5 | 7 | 0 | Imported | |
| `Security` | 2 | 0 | 0 | Imported | |
| `ServiceManagement` | 17 | 42 | 0 | Lift-tested | 8 distinct authors led by `Peter Larem` (42 files); pure upstream. MARS Machine `generates` Log/Timeseries lands here. |
| `SharePoint` | 0 | 2 | 0 | Imported | Attributes-only |
| `Software` | 5 | 0 | 0 | Imported | Distinct from `NTO/MARS/Software/` — this is a software-engineering vocabulary |
| `Statistics` | 1 | 0 | 0 | Imported | |
| `Survey` | 3 | 0 | 0 | Imported | |
| `Transport` | 5 | 14 | 8 | Lift-tested | `chris.boos@almato.com` (sole author, 27 files) — pure upstream-arago |
| `UserMeta` | 4 | 0 | 4 | Imported | |
| `Version` | 0 | 3 | 0 | Imported | Used by MARS Machine for OS version |
| **`WorkOrder`** | 27 | 0 | 0 | **Lift-tested** | **Our extension** (`dcterms:creator` = `bus-compiler` + `family-codec-smith` — internal agent authors, zero external). Authored for `woa-rs`. All 27 TTLs declared as `rdfs:Class`, including the 12 in `verbs/`. **The `rdfs:Class`-as-verb convention is deliberate, not a quirk** — it makes each verb a typed template (slots, inheritance, policy metadata) that `ogar-render-askama` can compile-time-validate against a binding, the same way askama validates HTML templates against a Rust struct. See `docs/VERB-AS-CLASS-TEMPLATE.md`. Previous catalogue row split 15 entities + 12 verbs by directory; the content-driven count is 27 first-class typed declarations (entities + verb-as-class templates), which is what `ogar-from-schema` sees and what the action-render path consumes. |
| **TOTALS** | **549** | **599** | **241** | — | + 42 other (Medical sql_mirror, etc.) |

## Adjacent OGIT trees (not NTO)

| Path in OGIT | Files | Purpose |
|---|--:|---|
| `SGO/` | 508 TTLs | Upper ontology — `core/`, `ogit/`, `sgo/`. **`SGO/sgo/verbs/` is the 176-verb canonical AST predicate vocabulary** lifted by `ogar-from-schema::sgo` |
| `SDF/` | 7 JSON | Standard Data Format config samples (MARS/Automation) — instance configs, not schema |
| `ogit.ttl` | 1 TTL | Root ontology declaring `ogit:Entity`, `ogit:Verb`, `ogit:Attribute` |

## Provenance

The counts are the last vendored snapshot, OGIT SHA
`d0f489fff94640fef1e6abe7eacba90a1a144579` (2026-05-30). Since 2026-10-10
OGAR vendors nothing: tests read AdaWorldAPI/OGIT at its moving `master`
through `crates/ogar-from-schema/src/ogit_checkout.rs`, and CI checks it
out the same way. The MARS XSD oracle that sat in the mirror's
`NTO/MARS/_oracle/` was OGAR's own, not OGIT's, and lives at
`vocab/oracles/mars/`.
