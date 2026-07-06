# HARVEST — WoA sink-in substrate v2

`WoA` (Flask-SQLAlchemy, `/home/user/WoA/models.py` **+** `/home/user/WoA/woa/models_shop.py`,
READ-ONLY corpus) transcoded through the ruff → OGAR pipeline into the V3 transpile substrate.
Produced for MISSION "OGAR V3: Criticals testen + Python-Arm real machen (WoA 1:1)" as the concrete
SPEC-B1-3 reality-check artifact set (v2 of the SPEC-5 Part B/C/D harvest).

**v1 → v2 change:** the corpus grew from the monolithic `models.py` (139 classes) to
`models.py` (139) **+** `woa/models_shop.py` (12) = **151** classes, merged before minting via a
second `ruff_sqlalchemy_spo::extract_file` call and `ModelGraph.models.extend()` (same `"woa"`
namespace, so shop→core associations resolve against the combined corpus). The emitted `models.py`
was also regenerated through the FIXED `emit_python` (Batch-1 Item 1: `emit_python_prelude()` +
`Optional[...]` nullability) — the v2 module now **imports**, not just `py_compile`s (closes v1 gap
#3, see below).

## Pipeline (real code path, nothing hand-written)

```
/home/user/WoA/models.py  +  /home/user/WoA/woa/models_shop.py
  │  ruff_sqlalchemy_spo::extract_file (x2, same "woa" namespace)   (frontend, /tmp/wt-gr
  │                                                                   crates/ruff_sqlalchemy_spo,
  │                                                                   branch claude/spo-python-main)
  ▼
ruff_spo_triplet::ModelGraph   (merged: primary.models.extend(shop.models))
  │  ogar-from-ruff::sqlalchemy::lift_model_graph_sqlalchemy
  ▼
Vec<ogar_vocab::Class>            (schema stratum: attributes / associations)
  │  ogar-from-ruff::mint::compile_graph_sqlalchemy::<WoaPort>
  ▼
Vec<CompiledClass>                (Class + 16-byte rail Facet, classid via WoaPort)
  │  ogar-from-ruff::emit::emit_python_prelude() + emit::emit_python  -> models.py
  │  (association walk, classid-annotated)                           -> woa_graph.spo
  │  ogar-adapter-postgres-ddl::emit_facet_table_ddl + emit_postgres_ddl -> woa_facet.sql
  ▼
this directory (+ ogar_runtime.py shipped alongside, see "Import gate" below)
```

Driver: a scratch Cargo binary at `/tmp/harvest-woa` (NOT committed anywhere — path-deps + a local
`[patch]` block only, per the mission's leitplanken). See "Reproduction" below for the exact
commands to regenerate.

## Metrics (v2, rerun-measured — not hand-estimated, quoted from the driver's own stderr)

```
harvested 139 models from /home/user/WoA/models.py
harvested 12 shop models from /home/user/WoA/woa/models_shop.py
total 151 models
TimesheetActivity -> classid 0x01030003 (concept 0x0103, app 0x0003)
dangling .spo edges: 0/112
done: 151 classes, 2154 attrs, 112 assocs, 6 aliased, 145 bootstrap, 0 dangling
```

| Metric | v1 (139) | v2 (151) |
|---|---|---|
| `db.Model` classes harvested | 139 | **151** (139 `models.py` + 12 `woa/models_shop.py`) |
| Attributes (`Attribute` after FK-dedup) | 1961 | **2154** |
| Associations (`db.relationship`, `BelongsTo`/`HasMany`) | 107 | **112** |
| Aliased (resolve through `WOA_ALIASES` convergence pins, concept ≠ 0) | 6 | **6** (unchanged — no shop model matches a `WOA_ALIASES` pin) |
| Bootstrap (unmapped model name → classid `0x0000_0003`, concept = 0) | 133 | **145** (133 + all 12 new shop models — verified: every `Shop*` class emits `CLASSID: ClassVar[int] = 0x00000000`) |
| Dangling `.spo` edges | 0/107 | **0/112** (shop associations also resolve within the combined 151-class corpus) |

### The 6 aliased classes (WOA_ALIASES convergence pins, `ogar-vocab/src/ports.rs`) — unchanged in v2

| Class | classid | concept | Same concept converges with |
|---|---|---|---|
| `TimesheetActivity` | `0x0103_0003` | `0x0103` (`BILLABLE_WORK_ENTRY`) | `OpenProjectPort::TimeEntry`, `RedminePort::TimeEntry` |
| `Customer` | `0x0204_0003` | `0x0204` | (CRM-concept alias) |
| `WorkOrder` | `0x0202_0003` | `0x0202` | (matches Odoo's `account.move`-adjacent concept family — same concept id Odoo's `sale.order`/`account.move` land in) |
| `Position` | `0x0201_0003` | `0x0201` | |
| `RecurringInvoice` | `0x0202_0003` | `0x0202` | same concept as `WorkOrder` — both are "billing document" shaped |
| `TaxRate` | `0x0203_0003` | `0x0203` | |

The 12 new `models_shop.py` classes (`ShopProduct`, `ShopCategory`, `ShopOrder`, `ShopOrderItem`,
`ShopCartSession`, `ShopPage`, `ShopShippingMethod`, `ShopPaymentMethod`, `ShopCustomer`,
`ShopPaymentConfig`, `ShopSaasConfig`, `ShopProductReview`) all mint the bootstrap classid
`0x0000_0003` — verified directly against the emitted module (`grep -A2 'class Shop' models.py`
shows `CLASSID: ClassVar[int] = 0x00000000` for all twelve). `ShopCustomer` does **not** collide
with the aliased `Customer` — they remain distinct classes with distinct (bootstrap vs. aliased)
classids, per mint governance (no new codebook aliases/mints without an Operator ruling).

### Convergence spot-check (mission-mandated)

```
TimesheetActivity -> classid 0x01030003 (concept 0x0103, app 0x0003)
```

Unchanged from v1 — `TimesheetActivity` lives in the primary `models.py`, unaffected by the
`models_shop.py` merge. Matches the fixture in `ogar-from-ruff/src/mint.rs`'s own doc comment on
`compile_graph_sqlalchemy` and its test
`compile_graph_sqlalchemy_timesheet_activity_converges_via_alias_not_bootstrap` byte-for-byte.

## Import gate (v2 closes v1 gap #3 — proof, not just `py_compile`)

`py_compile` only proves the module *parses*; it does not evaluate class-body annotation
expressions in a way that would raise `NameError` on an unresolved name. v1's `models.py` failed a
real `import` (`OgInt`/`OgStr`/etc. were referenced but never imported). v2 fixes this at the
source (Batch-1 Item 1's `emit_python_prelude()`), and this harvest now runs the import gate
directly in this directory (with `ogar_runtime.py` shipped alongside, since a real consumer may not
have it on `sys.path` yet):

```sh
$ cd .claude/harvest/woa-rs   # (or woa-sinkin-substrate on the WoA side)
$ python3 -m py_compile models.py
$ python3 -c "import models; print('IMPORT_OK', len(models.__dict__))"
IMPORT_OK 176
```

Both commands exit 0. `ogar_runtime.py` (the reference wrapper-contract shipped from
`ogar-from-ruff/python/ogar_runtime.py`, Item 1) sits next to `models.py` in this directory so the
gate is standalone — no external `sys.path` setup required. A real consumer may instead supply its
own `ogar_runtime` (with real Arrow/Decimal/etc.-backed types) on `sys.path` and drop this reference
copy; either way the generated module's `from ogar_runtime import (...)` contract is now closed.

## Gates run

- `python3 -m py_compile models.py` → **PASS**.
- `python3 -c "import models"` → **PASS** (176 names in `models.__dict__`, i.e. no `NameError` —
  v1 gap #3 is closed; see "Import gate" above for the exact transcript).
- `cargo build --quiet` (the scratch driver, against the CURRENT `ogar-from-ruff` post Item 1/2) →
  **PASS**, no `[patch]`/type-identity failures (see "Build caveat" note in the Reproduction
  section — the same `/tmp/wt-gr` local checkout that satisfied v1 still resolves cleanly here,
  since ogar-from-ruff's `ruff_spo_triplet`/`ruff_spo_address` git deps are patched by *source URL*,
  not by matching the `branch =` string).

## Reproduction

```sh
# Driver Cargo.toml path-deps point at the B1-integration worktree (/tmp/wt-b1int, branch
# claude/b1-integration — the SHA d679f850-approved B1-1+B1-2 stand this v2 was regenerated
# against), not /workspace/ogar (session-specific worktree substitution, documented as an
# authorized mission deviation; NOT committed — this Cargo.toml lives only at /tmp/harvest-woa).
cd /tmp/harvest-woa
cargo run --quiet -- /home/user/WoA/models.py /tmp/harvest-woa/out /home/user/WoA/woa/models_shop.py
python3 -m py_compile /tmp/harvest-woa/out/models.py
cp /tmp/wt-b1int/crates/ogar-from-ruff/python/ogar_runtime.py /tmp/harvest-woa/out/ogar_runtime.py
cd /tmp/harvest-woa/out && python3 -c "import models"
```

## Provenance

| Input | Pin |
|---|---|
| WoA (`AdaWorldAPI/WoA`, `models.py` + `woa/models_shop.py`, READ-ONLY) | `438dd8c429ed5db5188118f50490ad24485c92d3` (last commit touching both files — `backup-update: sync Live-/opt/woa/ Snapshot 2026-06-28 10:58:40`) |
| ruff (`AdaWorldAPI/ruff`, `ruff_sqlalchemy_spo`) | `/tmp/wt-gr`, branch `claude/spo-python-main`, HEAD `66db5c417eddf6017e924706031a23b019c17e81` (unchanged from v1 — `/tmp/wt-gr` still exists and still hosts `ruff_sqlalchemy_spo` + `ruff_spo_{triplet,address}`) |
| OGAR (`ogar-from-ruff`, `ogar-vocab`, `ogar-adapter-postgres-ddl`) | `/tmp/wt-b1int`, branch `claude/b1-integration`, HEAD `d679f8504389bb743b08d34ae7352511d0a34b4b` (B1-1 `emit_python` prelude+`Optional[]` + B1-2 FK-dedup consolidation, approved) |

## Known gaps (honest, not papered over)

1. **Behavior = names only.** Unchanged from v1 — `Model::functions` (methods) are harvested
   name-only by `ruff_sqlalchemy_spo::functions` (v0). `emit_python` emits **zero** methods for
   either the 139 core or the 12 shop classes; this v2 pass does not touch the mission's Critical
   #1 (`behavior`) gap at all.
2. **`woa/models_shop.py` gap — RESOLVED in v2.** v1 gap #2 ("`woa/models_shop.py` out of scope")
   is closed: the driver now runs `ruff_sqlalchemy_spo::extract_file` a second time against
   `woa/models_shop.py` and merges (`ModelGraph.models.extend()`) before minting, in the same
   `"woa"` namespace as the primary corpus. All 12 shop classes are present in the emitted
   `models.py` (verified by name, see the aliased-classes section above) and the corpus total is
   the expected 139 + 12 = 151.
3. **`emit_python`'s import list — RESOLVED in v2.** v1 gap #3 ("`models.py` does not `import`") is
   closed by Batch-1 Item 1 (`emit_python_prelude()` emits `from ogar_runtime import (OgScalar,
   OgStr, OgInt, OgFloat, OgMoney, OgBool, OgDate, OgDateTime, OgBytes, OgSelection, OgJson, ToOne,
   ToMany)` plus `from __future__ import annotations`, `from dataclasses import dataclass`,
   `from typing import ClassVar, Optional`) and this harvest shipping `ogar_runtime.py` (Item 1's
   reference wrapper-contract) alongside `models.py`. Verified by *running* `import models`, not
   just `py_compile` — see "Import gate" above for the exact transcript (`IMPORT_OK 176`).
4. **`woa_graph.spo` target-classid resolution: re-measured for v2, still 0 dangling.** An
   association's target is looked up by name against all 151 harvested+minted classes (up from
   139); a miss is written as `[--------]` rather than silently guessed. Measured over the actual
   112-edge v2 output (up from 107 in v1): **0 of 112 edges are `[--------]`** — every association
   target, including any shop→core references introduced by `models_shop.py`, resolves within the
   combined 151-class corpus. Quoted directly from the driver's own stderr
   (`dangling .spo edges: 0/112`), not hand-counted.
5. **`PostgreSQL` DDL is the relational (`emit_postgres_ddl`) shape, not yet wired to the facet
   table.** Unchanged from v1 — `woa_facet.sql` emits the two pieces side-by-side but does not
   attempt the dual-write / parity-checker wiring from the Operator's ruling (b, refined); that
   remains a documented follow-up, not built here.
6. **Falsifizierer #1 (WP-Parity)-style field-by-field diff against the WoA source was not run for
   all 151 classes.** Unchanged scope from v1 — only the `TimesheetActivity` fixture (§ "Convergence
   spot-check" above) is checked field-for-field against source; a full N/N-columns-typed parity
   metric across all 151 classes remains a reasonable next step, out of scope for this harvest pass.
7. **Build/dependency note (report, not a gap in the artifact):** the driver's Cargo.toml points at
   `/tmp/wt-b1int` (not `/workspace/ogar`) for `ogar-from-ruff`/`ogar-vocab`/
   `ogar-adapter-postgres-ddl` — an authorized session-specific substitution (the approved,
   integrated B1-1+B1-2 stand lives in that worktree, on branch `claude/b1-integration`, not yet
   merged to `/workspace/ogar`'s working branch at harvest time). The `[patch."https://github.com/
   AdaWorldAPI/ruff"]` block still resolves `ruff_spo_triplet`/`ruff_spo_address` to `/tmp/wt-gr`
   for both consumers (`ruff_sqlalchemy_spo` and, transitively, `ogar-from-ruff`, which now declares
   `branch = "main"` for those deps rather than the old convergence branch name) — Cargo's `[patch]`
   keys on the git source URL, not the declared branch string, so the override still applies and
   `cargo build` succeeded with no type-identity failures. This Cargo.toml is not committed anywhere
   (scratch driver only, per mission leitplanken).
