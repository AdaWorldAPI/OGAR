# PROVENANCE — `vocab/exports/`

> **Staging tier for OGAR-produced TTL, in OGIT-compatible shape.**
> Content here is AUTHORED in OGAR (digested from source by a producer
> — `ruff_*_spo + ogar-from-ruff`, `ogar-from-schema` — or
> hand-authored) and is **NOT YET PROMOTED** to the AdaWorldAPI/OGIT
> fork. Once reviewed, content is committed to the OGIT fork and
> re-vendored into `vocab/imports/` like any other upstream content.
>
> Status: **STAGING TIER v1** (2026-06-22). Empty until a producer
> stages content; transient by design.

## The model (operator-decided 2026-06-22)

```
   OGAR producer  ──►  vocab/exports/ogit/NTO/<Domain>/   (review / iterate)
   (ruff_*_spo +            │
    ogar-from-ruff,         │  promote (commit to the OGIT fork on a branch, PR there)
    ogar-from-schema)       ▼
                       AdaWorldAPI/OGIT fork  (the enriched canonical OGIT store —
                            │                  upstream arago/almato + OGAR-promoted)
                            │  re-vendor (cp -r /OGIT/NTO/. vocab/imports/ogit/NTO/)
                            ▼
                       vocab/imports/ogit/   (faithful SHA-pinned mirror of the fork)
                            │
                            ▼
                       consumers read ONLY imports/
```

- **`exports/` is the staging area** — produced-but-not-yet-promoted
  content. Review it here, then promote to the fork.
- **The AdaWorldAPI/OGIT fork is the enriched canonical store** — it
  already mixes upstream arago/almato content with OGAR-promoted
  additions (e.g. commit `c5dc1b8` "shrink 3-hop Odoo lookups —
  promoted attrs + shortcut verbs + FiscalJurisdiction codebook"
  added 11 Accounting TTLs to the fork deliberately).
- **`imports/` faithfully mirrors the enriched fork** — including any
  OGAR-promoted content. Re-vendor is **safe**: it copies *from* the
  fork, which has everything.
- **Consumers read only `imports/`.** `exports/` never ships to a
  consumer; it is the pre-promotion workbench.

## Why a staging tier (not "just commit to the fork directly")

1. **Review surface.** A digest run produces N TTLs at once. Staging
   them in `exports/` lets the round-trip + bijection tests run, the
   diff-vs-prior-digest drift check fire, and a human review the lift,
   all inside the OGAR repo and CI — before anything touches the fork.

2. **License + governance boundary.** Until promoted, OGAR-produced
   content carries OGAR's license, not the fork's MIT (Almato AI GmbH).
   Promotion is the deliberate act that relicenses / blesses it into
   the shared OGIT store.

3. **The producer↔consumer split stays clean.** Producers write
   `exports/`; consumers read `imports/`. Nothing reads the half-baked
   tree. The promote step is the single, auditable gate between them.

## Layout

```
vocab/exports/
└── ogit/                          ← OGIT-shape (matches the fork's layout)
    ├── NTO/
    │   ├── <Domain>/              ← mirrors the OGIT NTO layout
    │   │   ├── entities/          ← entity TTLs (a rdfs:Class)
    │   │   ├── attributes/        ← datatype-property TTLs (a owl:DatatypeProperty)
    │   │   ├── verbs/             ← verb TTLs (a owl:ObjectProperty
    │   │   │                       OR a rdfs:Class for askama-template verbs)
    │   │   └── PROVENANCE.md      ← per-domain: which producer ran, which
    │   │                            source input + revision, promotion status
    │   ├── ...
    └── PROVENANCE.md (this file)
```

The layout mirrors `imports/ogit/` 1:1 so a promote step is a plain
`cp`/`git mv` into the fork at the same relative path.

## What lives here today

Empty. Producers haven't staged anything yet; content populates as
digests run:

| Source | Producer | Stages at |
|---|---|---|
| Odoo `addons/account/*` | `ruff_python_spo + ogar-from-ruff` (frontend queued) | `exports/ogit/NTO/Accounting/` |
| Odoo `addons/sale/*` | same | `exports/ogit/NTO/SalesDistribution/` |
| Odoo `addons/stock/*` | same | `exports/ogit/NTO/Transport/` |
| Odoo workflow `def action_*` | same (verb-as-class shape) | `exports/ogit/NTO/<Domain>/verbs/` |
| Medcare-rs domain types | `ruff_rust_spo + ogar-from-ruff` (frontend queued) | `exports/ogit/NTO/Healthcare/` |
| Medcare-rs MongoDB schemas | `ogar-from-schema` (XSD/JSON-Schema) | `exports/ogit/NTO/Healthcare/` |
| Hand-authored OGAR Class views | direct authoring | `exports/ogit/NTO/<Domain>/` |

## NOT a migration target — the 11 Accounting files are already promoted

> **Correction (2026-06-22).** An earlier draft of this file claimed
> the 11 OGAR-produced TTLs in `vocab/imports/ogit/NTO/Accounting/`
> were "at re-vendor risk" and "belong in `exports/`". **That was
> wrong.** Those 11 files are committed to the **AdaWorldAPI/OGIT
> fork** (commit `c5dc1b8`, on `master`, pushed). `imports/`
> faithfully mirrors the fork, so re-vendor **preserves** them — there
> is no data-loss risk, and they are NOT a migration candidate.
>
> They are the worked example of a *completed* promotion: produced by
> a prior session, promoted to the fork, now correctly mirrored in
> `imports/`. Under the staging-tier model they belong exactly where
> they are. `exports/` is for content that has **not yet** made that
> trip.

## License + promotion

OGAR repository license (see top-level `LICENSE`) applies to content
**in this tree**. Promotion to the OGIT fork is the deliberate act
that moves a file into the fork's MIT-licensed store; it requires
committing to the fork (a separate repo, a separate PR) and is the
single auditable gate between OGAR-produced and OGIT-canonical.
