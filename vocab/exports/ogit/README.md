# `vocab/exports/ogit/` — OGAR's staging tier for OGIT-shaped TTL

Produced-but-not-yet-promoted content, in the upstream OGIT layout
(`NTO/<Domain>/{entities,attributes,verbs}/`, `SGO/sgo/verbs/`). A
producer stages a digest here; once reviewed (round-trip + bijection
tests, drift check, human read), it is **promoted** to the
AdaWorldAPI/OGIT fork and re-vendored into `vocab/imports/`.

**Consumers read `imports/`, never `exports/`.** This tree is the
pre-promotion workbench.

See `vocab/exports/PROVENANCE.md` for the full staging-tier model, why
the fork is the enriched canonical store, and the correction on the 11
Accounting files (already promoted via OGIT fork commit `c5dc1b8` —
NOT a migration target).
