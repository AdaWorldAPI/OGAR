# HARVEST — WoA sink-in substrate v1

`WoA` (Flask-SQLAlchemy, `/home/user/WoA/models.py`, READ-ONLY corpus) transcoded
through the ruff → OGAR pipeline into the V3 transpile substrate. Produced for
MISSION "OGAR V3: Criticals testen + Python-Arm real machen (WoA 1:1)" as the
concrete SPEC-5 Part B/C/D reality-check artifact set.

## Pipeline (real code path, nothing hand-written)

```
/home/user/WoA/models.py
  │  ruff_sqlalchemy_spo::extract_file            (frontend, /tmp/wt-gr crates/ruff_sqlalchemy_spo,
  │                                                 branch claude/spo-python-main)
  ▼
ruff_spo_triplet::ModelGraph
  │  ogar-from-ruff::sqlalchemy::lift_model_graph_sqlalchemy
  ▼
Vec<ogar_vocab::Class>            (schema stratum: attributes / associations)
  │  ogar-from-ruff::mint::compile_graph_sqlalchemy::<WoaPort>
  ▼
Vec<CompiledClass>                (Class + 16-byte rail Facet, classid via WoaPort)
  │  ogar-from-ruff::emit::emit_python           -> models.py
  │  (association walk, classid-annotated)       -> woa_graph.spo
  │  ogar-adapter-postgres-ddl::emit_facet_table_ddl + emit_postgres_ddl -> woa_facet.sql
  ▼
this directory
```

Driver: a scratch Cargo binary at `/tmp/harvest-woa` (NOT committed anywhere —
path-deps + a local `[patch]` block only, per the mission's leitplanken). See
"Reproduction" below for the exact commands to regenerate.

## Metrics

| Metric | Value |
|---|---|
| `db.Model` classes harvested | **139** (`ruff_sqlalchemy_spo` corpus-gate test asserts `>= 139`; WoA's own `grep -c 'class.*db.Model'` on `models.py` also gives 139 — exact match, no discrepancy) |
| Attributes (`Attribute` after FK-dedup) | **1961** |
| Associations (`db.relationship`, `BelongsTo`/`HasMany`) | **107** |
| Aliased (resolve through `WOA_ALIASES` convergence pins, concept ≠ 0) | **6** |
| Bootstrap (unmapped model name → classid `0x0000_0003`, concept = 0) | **133** |

### The 6 aliased classes (WOA_ALIASES convergence pins, `ogar-vocab/src/ports.rs`)

| Class | classid | concept | Same concept converges with |
|---|---|---|---|
| `TimesheetActivity` | `0x0103_0003` | `0x0103` (`BILLABLE_WORK_ENTRY`) | `OpenProjectPort::TimeEntry`, `RedminePort::TimeEntry` |
| `Customer` | `0x0204_0003` | `0x0204` | (CRM-concept alias) |
| `WorkOrder` | `0x0202_0003` | `0x0202` | (matches Odoo's `account.move`-adjacent concept family — same concept id Odoo's `sale.order`/`account.move` land in) |
| `Position` | `0x0201_0003` | `0x0201` | |
| `RecurringInvoice` | `0x0202_0003` | `0x0202` | same concept as `WorkOrder` — both are "billing document" shaped |
| `TaxRate` | `0x0203_0003` | `0x0203` | |

The remaining 133 models mint the **bootstrap** classid `0x0000_0003`
(concept `0x0000`, WoA app prefix `0x0003`) — `WOA_ALIASES` is a small,
deliberately curated table (per `ports.rs` docs, a handful of named
convergence pins), not a name-for-name mirror of all 139 WoA models. This is
the honest, measured state (matches `sqlalchemy.rs`'s own doc comment on
`compile_graph_sqlalchemy`, which names this exact caveat for
`TimesheetActivity`) — no attempt was made to mint new aliases (mint
governance: no new Codebook concepts without an Operator ruling).

### Convergence spot-check (mission-mandated)

```
TimesheetActivity -> classid 0x01030003 (concept 0x0103, app 0x0003)
```

Matches the fixture in `ogar-from-ruff/src/mint.rs`'s own doc comment on
`compile_graph_sqlalchemy` and its test
`compile_graph_sqlalchemy_timesheet_activity_converges_via_alias_not_bootstrap`
byte-for-byte. `TimesheetActivity`'s harvested shape also matches WoA source
`models.py:1746-1753` exactly: `id` (required), `beschreibung` (required),
`created_at` (nullable — no `nullable=False` in source), `timesheet_id`
FK-shadowed by the `timesheet: ToOne["TimeSheet"]` association (not
double-projected).

## Gates run

- `python3 -m py_compile models.py` → **PASS** (module parses; see "Known
  gaps" below for what this gate does NOT prove).
- `cargo test -p ruff_sqlalchemy_spo` corpus gate (`WOA_SRC=/home/user/WoA/models.py`)
  already asserts `>= 139` models upstream in the frontend crate itself (not
  re-run here — this harvest's own driver run independently reproduces the
  same 139).

## Reproduction

```sh
# 1. Scratch driver (path deps + local-only [patch], never committed):
mkdir -p /tmp/harvest-woa/src
cat > /tmp/harvest-woa/Cargo.toml <<'TOML'
[package]
name = "harvest-woa"
version = "0.1.0"
edition = "2021"
publish = false
[[bin]]
name = "harvest-woa"
path = "src/main.rs"
[dependencies]
ruff_sqlalchemy_spo = { path = "/tmp/wt-gr/crates/ruff_sqlalchemy_spo" }
ogar-from-ruff = { path = "/workspace/ogar/crates/ogar-from-ruff" }
ogar-vocab = { path = "/workspace/ogar/crates/ogar-vocab" }
ogar-adapter-postgres-ddl = { path = "/workspace/ogar/crates/ogar-adapter-postgres-ddl" }
[patch."https://github.com/AdaWorldAPI/ruff"]
ruff_spo_triplet = { path = "/tmp/wt-gr/crates/ruff_spo_triplet" }
ruff_spo_address = { path = "/tmp/wt-gr/crates/ruff_spo_address" }
TOML
cp /workspace/ogar/rust-toolchain.toml /tmp/harvest-woa/rust-toolchain.toml   # pins 1.95.0
# (src/main.rs: extract_file -> compile_graph_sqlalchemy::<WoaPort> -> emit_python
#  + a classid-keyed association walk for woa_graph.spo + the postgres-ddl adapter
#  calls for woa_facet.sql — see this harvest's provenance pins below for the
#  exact commit this was run against.)

cd /tmp/harvest-woa
cargo run --quiet -- /home/user/WoA/models.py /tmp/harvest-woa/out
python3 -m py_compile /tmp/harvest-woa/out/models.py
```

## Provenance

| Input | Pin |
|---|---|
| WoA (`AdaWorldAPI/WoA`, `models.py`, READ-ONLY) | `4427b3d715d841290ea9108e3bf94b22d0cb72c2` (last commit touching `models.py`) |
| ruff (`AdaWorldAPI/ruff`, `ruff_sqlalchemy_spo`) | `/tmp/wt-gr`, branch `claude/spo-python-main`, HEAD `66db5c417eddf6017e924706031a23b019c17e81` |
| OGAR (`ogar-from-ruff`, `ogar-vocab`, `ogar-adapter-postgres-ddl`) | `/workspace/ogar`, branch `claude/v3-criticals-woa-parity`, HEAD `d5479ce382136b3aedad3c122e13d3aab6f9695e` |

## Known gaps (honest, not papered over)

1. **Behavior = names only.** `Model::functions` (methods) are harvested
   name-only by `ruff_sqlalchemy_spo::functions` (v0, per its module doc) —
   no params, no `reads`/`writes`, no `body_source`. `emit_python` therefore
   emits **zero** methods; the generated dataclasses are schema-only mirrors
   of WoA's `db.Model` classes, not behavior-complete ports. This is the same
   gap the mission's Critical #1 (`behavior`) names for `lift_actions`
   generally (`ogar-from-ruff/src/lib.rs:495`) — this SQLAlchemy harvest
   doesn't lift `ActionDef`/`KausalSpec` at all (v0 doesn't wire it), so the
   gap is total rather than partial here.
2. **`woa/models_shop.py` (12 more `db.Model` classes) is out of scope for
   this v1.** Only the monolithic `/home/user/WoA/models.py` (139 classes)
   was harvested, per the mission's explicit instruction. Follow-up: run
   `ruff_sqlalchemy_spo::extract_file` a second time against
   `woa/models_shop.py` (or `extract_with` over the `woa/` tree) and merge
   the two `ModelGraph`s before minting, so shop-domain FKs into the core 139
   resolve rather than dangling.
3. **`emit_python`'s import list is incomplete — `models.py` does not
   `import` (nor is `ogar_sdk.py` shipped alongside).** The header only
   imports `OgScalar, ToOne, ToMany` from `ogar_sdk` (mirroring the PR #154
   Python-emit precedent's own header), but `emit_python`'s
   `og_scalar_type()` mapping emits bare names — `OgInt`, `OgStr`, `OgBool`,
   `OgDateTime`, `OgFloat`, `OgMoney`, `OgDate`, `OgBytes`, `OgSelection`,
   `OgJson` — that are **never imported**. `python3 -m py_compile` still
   passes because compiling to bytecode does not evaluate the class-body
   annotation expressions at import time in a way that raises — it is a
   syntax-level gate only, not proof the module is importable/runnable.
   Actually running `import models` would raise `NameError` on the first
   annotated attribute. This is `ogar-from-ruff::emit_python`'s existing gap
   (not introduced by this harvest), reported per the mission's instruction
   to name the "emit_python required-Lücke" honestly rather than
   hand-patching the generated output.
4. **`woa_graph.spo` target-classid resolution: measured, not assumed.** An
   association's target (`Association::class_name`, e.g. `TimeSheet` for
   `TimesheetActivity.timesheet`) is looked up by name against the 139
   harvested+minted classes; a miss (renamed target, target lives in
   `models_shop.py`, or a `class_name` string that doesn't match any
   harvested class) is written as `[--------]` rather than silently guessed.
   Measured over the actual 107-edge output: **0 of 107 edges are
   `[--------]`** — every association target in `models.py` resolves within
   the same 139-class corpus (the file is self-contained; nothing points
   into the excluded `models_shop.py`). This is a stronger result than
   expected going in and is called out explicitly so it isn't mistaken for
   an unverified assumption.
5. **`PostgreSQL` DDL is the relational (`emit_postgres_ddl`) shape, not yet
   wired to the facet table.** `woa_facet.sql` emits the two pieces
   side-by-side (facet table via `emit_facet_table_ddl("facet")`, then one
   `CREATE TABLE` per class via `emit_postgres_ddl`) but does **not** attempt
   the dual-write / parity-checker wiring the Operator's ruling (b, refined)
   describes — that is explicitly a "follow-up, NOT built here" per
   `ogar-adapter-postgres-ddl`'s own module doc.
6. **Falsifizierer #1 (WP-Parity)-style field-by-field diff against the WoA
   source was not run for all 139 classes** — only the `TimesheetActivity`
   fixture (§ "Convergence spot-check" above) was checked field-for-field
   against source. A full N/N-columns-typed parity metric (mission's
   D-PARITY-PROBE-style measure) across all 139 classes is a reasonable
   next step but out of scope for this harvest pass.
