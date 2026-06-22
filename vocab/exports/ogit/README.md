# `vocab/exports/ogit/` — OGAR-produced TTL in OGIT-compatible shape

Mirrors the upstream OGIT layout (`NTO/<Domain>/{entities,attributes,verbs}/`,
`SGO/sgo/verbs/`) so consumers see one shape across `imports/` and
`exports/`. The choice between "upstream-mirrored" and "OGAR-produced"
is the path prefix (`vocab/imports/ogit/` vs `vocab/exports/ogit/`),
nothing else.

See `vocab/exports/PROVENANCE.md` for the split rationale (re-vendor
safety, license/governance, upstream-contribution path) and the
migration note on the 11 stranded Accounting files currently sitting
in `imports/`.
