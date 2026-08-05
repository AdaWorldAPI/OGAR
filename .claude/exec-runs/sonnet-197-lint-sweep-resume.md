# Sonnet 1.97.1 lint/test sweep — OGAR (RESUME run)

Branch: `claude/loco-funnel-probe-s0` (unchanged, no commits/pushes made — per instructions).

**RUN HALTED EARLY — disk-safety hard rule.** Free space on `/home/user` dropped
from 7.0 GB (start) to 3.9 GB over the course of this run, crossing the 4 GB
stop threshold. Per instruction, halting immediately after the crate in
progress (`ogar-action-handler`) finished cleanly, and reporting instead of
continuing.

`du -sh` at halt: `OGAR/target` = 1.5G, `ndarray/target` = 1.3G,
`lance-graph/target` = 7.7G (external, same driver noted in the previous
run's halt — a sibling repo's build growing on this shared host, not residue
from this sweep).

## Part 1 — re-verify of prior halt point

| Crate | Clean? | Fix applied | Tests | fmt |
|---|---|---|---|---|
| ogar-adapter-clickhouse-ddl | **yes, confirmed clean** | (fix from prior run — `map_or(false,f)` → `is_some_and(f)` at src/lib.rs:230 — verified still in place and clippy-clean) | 7 unit + 0 doctest — all pass | no changes |

## Part 2 — new crates swept this run (5 of 22, before disk halt)

| Crate | Clean before? | Clippy fix | Tests | fmt |
|---|---|---|---|---|
| ogar-adapter-postgres-ddl | yes | — | 10 unit + 0 doctest — all pass | no changes |
| ogar-knowable-from | yes | — | 10 unit + 0 doctest — all pass | no changes |
| ogar-from-elixir | **no** | see below | 5 unit + 0 doctest — all pass | 1 cosmetic whitespace change (rustfmt) |
| ogar-from-ruff | **no** | see below | 3 unit + 3 integration + 0 doctest — all pass | 1 cosmetic reflow change (rustfmt, unrelated whitespace in `examples/compile_corpus.rs`) |
| ogar-from-rails | yes | — | 3 unit + 18 ignored (require external Redmine/OpenProject checkouts, unrelated to this sweep) + 0 doctest — all pass | no changes |
| ogar-from-schema | yes | — | 2 unit + 1 integration (membrane_probe) + 0 doctest — all pass | no changes |
| ogar-action-handler | yes | — | 12 unit + 1 integration (lifted_action_dispatch) + 1 doctest — all pass | no changes |

### ogar-from-elixir detail

`crates/ogar-from-elixir/src/lib.rs:58-61` — `unused_imports`: 9 of the 11
names imported at module scope from `ogar_vocab` (`ActionSubject`,
`Association`, `AssociationKind`, `Attribute`, `EnterEffect`,
`GuardFailurePolicy`, `KausalSpec`, `Language`, `ModalSpec`, `TemporalSpec`)
are used only inside `#[cfg(test)] mod tests` (via `use super::*;`), not by
the crate's runtime code (which uses only `ActionDef` and `Class`).

Fix (matches the "unused import that only tests use belongs in the test
module" prior from the lance-graph sweep): trimmed the top-level `use` to
`use ogar_vocab::{ActionDef, Class};` and added the 9 test-only names as a
new `use ogar_vocab::{...};` inside `mod tests`. No behaviour change — purely
a scope move; `mod tests` already re-imports everything via `use super::*;`
so no name resolution changed.

### ogar-from-ruff detail

`crates/ogar-from-ruff/examples/compile_corpus.rs:30` — `unused_imports`:
`Write` in `use std::io::{BufRead, BufReader, Write};`. Grepped the whole
example file — `Write` appears only in a doc-comment (`//! 6. Write the
output...`), never as a trait-method call site (the actual JSON-write path
under `#[cfg(feature = "serde")]` uses `std::fs::write`-shaped helpers, not
the `io::Write` trait directly, or that arm wasn't compiled without the
`serde` feature enabled for this default `--all-targets` run). Fix: dropped
`Write` from the import list. No behaviour change.

## Crates NOT reached (17 of 22) — new resume point

```
crates/ogar-class-view
crates/ogar-render-askama
crates/ogar-fma-skeleton
crates/ogar-fma
crates/ogar-obo
crates/ogar-cpic
crates/ogar-adapter-python
crates/ogar-adapter-csharp
crates/ogar-auth
crates/ogar-encryption
crates/ogar-doc-ir
crates/ogar-a2ui-frame
crates/ogar-from-docv1
crates/ogar-render-typst
crates/ogar-blockly
crates/ogar-loco
```

(`ogar-action-handler` — last crate attempted — completed fully: clippy
clean, tests pass, fmt run with no changes, all before the disk threshold
was crossed. It does NOT need to be re-verified next run.)

None of the 17 above were built or linted this run — no clippy/test/fmt data
exists for them yet under 1.97.1.

## Nothing skipped for API/behaviour reasons

Both fixes applied this run (`ogar-from-elixir`, `ogar-from-ruff`) are pure
import-scope/unused-import mechanical cleanups with zero API or behaviour
change. No fix was skipped for "would change public API" reasons in the
crates reached this run.

## Disk trend observed (for orchestrator diagnosis)

```
7.0G (start) -> 6.8G -> 6.4G -> 6.3G -> 6.2G -> 5.9G -> 5.8G -> 5.2G
  -> 4.9G -> 4.8G -> 4.4G -> 4.2G -> 4.1G -> 4.0G -> 3.9G (halt)
```

Same external driver as the prior halt: `lance-graph/target` at 7.7G,
apparently still an actively-growing build from a concurrent session on this
shared host. OGAR's own `target/` (1.5G) and ndarray's (1.3G) are not the
cause. Recommend either waiting for the sibling build to finish/shrink, or
running the remaining 17 crates on a host/session with more headroom.
