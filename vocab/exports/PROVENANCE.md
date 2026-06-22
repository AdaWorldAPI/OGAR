# PROVENANCE — `vocab/exports/`

> **OGAR-produced TTL templates, in OGIT-compatible shape.** Distinct
> from `vocab/imports/` (read-only mirror of upstream OGIT, MIT
> licensed by arago/almato). Files in this tree are AUTHORED in OGAR
> — either digested from source by a producer (`ruff_*_spo +
> ogar-from-ruff`, future `ruff_rust_spo`, etc.) or hand-authored —
> and the OGAR license applies.
>
> Status: **SKELETON v0** (2026-06-22). Scaffold lands empty; content
> populates as producers digest into it.

## Why the split

`vocab/imports/` and `vocab/exports/` exist for three reasons:

1. **Re-vendor safety.** The `imports/` re-vendor recipe is a
   destructive `cp -r /upstream/. vocab/imports/...`. Putting
   OGAR-produced content in `imports/` would be silently nuked on
   the next re-vendor. The split makes the producer-output tree
   immune.

2. **License + governance.** `imports/` inherits MIT (Almato AI GmbH,
   2013–2024) from OGIT upstream. `exports/` inherits OGAR's own
   license. Authorship discriminator (`dcterms:creator`) becomes a
   secondary check rather than the primary one.

3. **Upstream-contribution path.** Files in `exports/` are candidates
   for PR back to OGIT upstream (or onward distribution to consumers
   expecting OGIT shape). Files in `imports/` are immutable mirrors.
   The directory split makes the contribution flow a one-line check.

## Layout

```
vocab/exports/
└── ogit/                          ← OGIT-shape (consumer-compat)
    ├── NTO/
    │   ├── <Domain>/              ← mirrors upstream OGIT NTO layout
    │   │   ├── entities/          ← entity TTLs (a rdfs:Class)
    │   │   ├── attributes/        ← datatype-property TTLs (a owl:DatatypeProperty)
    │   │   ├── verbs/             ← verb TTLs (a owl:ObjectProperty
    │   │   │                       OR a rdfs:Class for askama-template verbs)
    │   │   └── PROVENANCE.md      ← per-domain source provenance (which
    │   │                            producer ran, which upstream input,
    │   │                            which Odoo/Medcare/etc. revision)
    │   ├── ...
    └── PROVENANCE.md (this file)
```

The layout intentionally mirrors `imports/ogit/` 1:1 so consumers
discovering both trees see one shape; the choice between
"upstream-mirrored" and "OGAR-produced" is the path prefix, nothing
else.

## What lives here today

Empty. Producers haven't run yet; content populates as digests land:

| Source | Producer (planned) | Lands at |
|---|---|---|
| Odoo `addons/account/*` | `ruff_python_spo + ogar-from-ruff` | `exports/ogit/NTO/Accounting/` |
| Odoo `addons/sale/*` | same | `exports/ogit/NTO/SalesDistribution/` |
| Odoo `addons/stock/*` | same | `exports/ogit/NTO/Transport/` |
| Odoo workflow `def action_*` | same (verb-as-class shape) | `exports/ogit/NTO/<Domain>/verbs/` |
| Medcare-rs domain types | `ruff_rust_spo + ogar-from-ruff` (queued) | `exports/ogit/NTO/Healthcare/` |
| Medcare-rs MongoDB schemas | `ogar-from-schema` (XSD/JSON-Schema) | `exports/ogit/NTO/Healthcare/` |
| Hand-authored OGAR Class views | direct authoring | `exports/ogit/NTO/<Domain>/` |

## Migration note — the 11 stranded Accounting files

The current `vocab/imports/ogit/NTO/Accounting/` carries 11 TTLs
authored by a prior session (`dcterms:creator = "Claude
(AdaWorldAPI/lance-graph 3-hop optim)"`) sitting alongside 23 upstream
files by Viktor Voss. **Those 11 belong in `exports/ogit/NTO/Accounting/`**
— at re-vendor risk where they sit today. Migration is a separate
decision and a separate PR; the scaffold here doesn't move them yet.
The list:

```
vocab/imports/ogit/NTO/Accounting/verbs/hasProductCategory.ttl
vocab/imports/ogit/NTO/Accounting/verbs/hasPickingType.ttl
vocab/imports/ogit/NTO/Accounting/verbs/hasFiscalCountry.ttl
vocab/imports/ogit/NTO/Accounting/attributes/productCategoryComplete.ttl
vocab/imports/ogit/NTO/Accounting/attributes/iso3166Alpha2.ttl
+ 6 more (run the dcterms:creator scan in
  docs/OGIT-DOMAIN-LIFT-CATALOGUE.md § Verifying domain authorship
  to list all 11)
```

## License + contribution

OGAR repository license (see top-level `LICENSE`). Files here are
authored by OGAR — re-publishing back to OGIT upstream requires
explicit relicensing or arago/almato acceptance.
