# PROVENANCE — `vocab/imports/ogit/`

> Literal byte-mirror of the **full OGIT** at the SHA below — NTO + SGO + SDF
> + root `ogit.ttl`. **3245 TTL files, 3.3 MB of content.** Every file in this tree is
> `diff -q`-equal to its origin. Re-vendor by re-running the copy and bumping
> the SHA below; never hand-edit.

## Source

| Field | Value |
|---|---|
| Upstream | `AdaWorldAPI/OGIT` (fork of `arago/OGIT`) |
| Paths | `NTO/`, `SGO/`, `SDF/`, top-level `ogit.ttl` (every subdirectory mirrored 1:1) |
| Commit SHA | `18f8177ec63dc2c26cb635556fedc7f54186b7b4` |
| Commit date | `2026-07-06 22:53:14 +0200` |
| License | MIT (Almato AI GmbH, 2013–2024) — see `OGIT/LICENSE.md` upstream |

## Layout

```
vocab/imports/ogit/
├── ogit.ttl       — root ontology declaring `ogit:Entity` etc
├── NTO/           — domain ontologies (74 domains, 2735 TTLs)
│   ├── MARS/      — the Application/Resource/Software/Machine taxonomy
│   │              + _oracle/ XSD validator (see MARS/PROVENANCE.md)
│   ├── Accounting/ Auth/ Automation/ … (73 other domains)
├── SGO/           — upper ontology (509 TTLs)
│   ├── core/      — root Entity/Node declarations
│   ├── ogit/      — base attribute extensions
│   └── sgo/       — canonical verb vocabulary (176 verbs)
└── SDF/           — Standard Data Format JSON configs (MARS/Automation)
```

## Why a full mirror — both NTO AND SGO

The operator gave a one-shot green-light to import **all** OGIT to
**avoid later duplication**. With the full tree in-repo:

- The `ogar-from-schema` producer can lift any of 72 NTO domains and
  resolve their `ogit:allowed (...)` verbs against the SGO upper-ontology
  registry with no additional fetch.
- SGO's 176 verbs (`dependsOn`, `contains`, `runsOn`, `generates`,
  `relates`, `causes`, `affects`, …) become the **canonical AST predicate
  vocabulary** for OGAR's `Association` and `ActionDef` surfaces.
- Domain-instance proposals stop re-fetching the same content (the
  "have we lifted Auth yet?" question is answered by `ls vocab/imports/ogit/NTO/`).
- `docs/OGIT-DOMAIN-LIFT-CATALOGUE.md` is the per-domain status table —
  always-current because it reads from this directory.
- The bijection check (sample-200 byte-equality vs origin) is a single
  `diff -qr` and lands in CI under one second.

## What's in here

- **`NTO/` — 74 domain ontologies, 2735 TTLs.** Per-domain counts in
  `docs/OGIT-DOMAIN-LIFT-CATALOGUE.md`. Totals, counting each TTL by the
  nearest `entities` / `attributes` / `verbs` directory on its path:
  **814 entities + 1548 attributes + 335 verbs + 38 other**. (The same rule
  gives 537 + 599 + 232 + 63 at `d0f489f`, so the 549 + 599 + 241 + 42 stated
  before used a different rule.)
- **`SGO/` — 509 upper-ontology TTLs.** Notable: `SGO/sgo/verbs/`
  contains **176 verb declarations** that are the canonical AST
  predicate vocabulary (parsed by `ogar-from-schema::sgo`, with
  round-trip enforced by `sgo::tests::all_sgo_verbs_roundtrip`).
- **`SDF/` — JSON config samples** for MARS/Automation (4 + 3 files;
  not TTL, kept for completeness).
- **`ogit.ttl` — root ontology** declaring `ogit:Entity`, `ogit:Verb`,
  `ogit:Attribute`, etc.

## Re-vendor

```bash
# From OGAR repo root, with /home/user/OGIT checked out at desired SHA:
mkdir -p vocab/imports/ogit/NTO
cp -r /home/user/OGIT/NTO/. vocab/imports/ogit/NTO/
cp -r /home/user/OGIT/SGO/. vocab/imports/ogit/SGO/
cp -r /home/user/OGIT/SDF/. vocab/imports/ogit/SDF/
cp /home/user/OGIT/ogit.ttl vocab/imports/ogit/ogit.ttl
# Update the SHA + date in this file and in MARS/PROVENANCE.md (which
# carries an extra _oracle/ from arago/MARS-Schema).
```

`cp -r` only adds and overwrites; it never removes a file the new commit
deleted, so on its own it breaks the byte-mirror claim above. Apply
`git diff --name-status <old>..<new> -- NTO SGO SDF ogit.ttl` instead: copy
every `A`/`M` path, delete every `D` path. The three upstream `NTO/` docs
(`README.md`, `PURL_ID_Registration.md`, `PURL_ID_Registration.md-e`) live at
this directory's root, not under `NTO/`.

## Re-vendor log

| Date | From | To | Delta in the mirrored set |
|---|---|---|---|
| 2026-10-10 | `d0f489f` | `18f8177e` | 1332 added (1295 `NTO/Utilities`, 24 `NTO/GFS` and others), 9 modified, 25 deleted (`NTO/Medical/sql_mirror`, which upstream moved out as a private schema). Mirror verified: 3294 mirrored files, 0 differ from `18f8177e`, 0 extra, 0 missing. `NTO/MARS` is unchanged. |

## Per-domain provenance

The MARS subdirectory carries its own `MARS/PROVENANCE.md` because it
additionally vendors the XSD oracle (`MARS/_oracle/`) from
`arago/MARS-Schema`. All other domains derive provenance from this
top-level file alone.

## Bijection check

```bash
# In OGAR repo root, with the OGIT clone at /home/user/OGIT at the same SHA:
diff -qr vocab/imports/ogit/<DOMAIN>/ /home/user/OGIT/NTO/<DOMAIN>/ \
    | grep -v '^Only in vocab/imports/ogit/.*: \(PROVENANCE\|_oracle\)$'
# expected output: empty (any line = drift)
```

The MARS-specific bijection (XSD oracle ↔ TTL classifications) is in
`MARS/PROVENANCE.md` and exercised by
`crates/ogar-from-schema/src/ttl.rs::application_class_values_appear_in_xsd_oracle`.
