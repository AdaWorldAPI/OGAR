# Handover — OGAR V3 as polyglot transpiler; the odoo-rs / OP / ruff convergence arc

> **Date:** 2026-07-09. **Scope:** the whole transpiler arc (ruff harvest →
> OGAR IR → materializations), not one repo. **Author's honesty contract:**
> this handover separates **[VERIFIED]** (I built/read it myself this session)
> from **[REPORTED]** (a parallel session claimed it; I could not confirm —
> odoo-rs was unreachable from my session: git 403 / pygithub 403 / MCP 404).
> Do not promote a [REPORTED] line to fact without checking the cited PR.

---

## 0. The locked frame (operator, verbatim) — the spine everything hangs on

OGAR V3 is a **polyglot transpiler**; each consumer repo is a **lens**, not a
"port". odoo-rs is the **Odoo lens**.

```
IN (harvest, per-language frontends)      IR (the substrate)            OUT (materializations)
────────────────────────────────────     ───────────────────────       ───────────────────────────────
Python/Odoo   ruff_python_spo            CompiledClass                  emit_rust    (Rust SDK)
Ruby/Rails    ruff_ruby_spo       ──►    { class:  ogar_vocab::Class    emit_python  (Python SDK — the
C#            ruff_csharp_spo            , facet:  16B classid+12                     mirror-back container)
C++           ruff_cpp_spo               , actions: ActionDef+Kausal }   emit_csharp  (C# SDK)
                                         one per class, addressable      ─ storage skins ─
+ views (ERB/Jinja/Odoo-XML/askama       by classid = domain:appid:      PG facet table (p0..p11 = 3×SPOG ORM)
  → one WideFieldMask brick)             classview                       lance-graph V3 (same 16B, native)
+ nav (navigates_to, 4 frontends                                         ─ view/nav skins ─
  → one connectivity brick)                                              4 renderers, 1 mask; 1 nav graph
```

**Roles, locked:**
- **odoo (Python upstream)** — read-only *harvest source* + *parity oracle*. Never a deploy target from this work.
- **odoo-rs** — the *substrate lens*: compile-in, hydrate the storage matrix, serve the mirror-backs (`emit_python` SDK is the "Python container", **not** upstream Odoo).
- **OGAR** — the *engine* (IR + vocab + emitters + adapters).

Every asset built this arc — kausal parity, field masks, Klickweg nav, the
ndjson corpora — is **transpiler infrastructure**, not app code.

---

## 1. Cross-repo state

Local branch is `claude/odoo-rs-transcode-lf8ya5` in every repo.

| Repo | My local branch tip | Notes |
|---|---|---|
| OGAR | `9e49ba4` **[VERIFIED]** | rebased clean onto `origin/main` (76dcb27); PR diff now additive-only (see §3). |
| ruff | `684ba99` **[VERIFIED]** | schema strata + lineage union; main at `72b7759` (#43 merged). |
| openproject-nexgen-rs | `5092688` **[VERIFIED]** | §5 steps 1–5 done; rebased linear on main. |
| lance-graph | `df36747` **[VERIFIED local]** | at my clone the contract nav API (`screens_reachable_from`/`nav_is_fully_connected`) was **not** found; parallel track reports it landed at `5284755`/#669 **[REPORTED]** — my clone is likely behind. Re-fetch before relying. |
| odoo-rs | `d724313` **[VERIFIED local]** | **unreachable via GitHub this session.** Parallel track reports PRs #30/#31/#32 + handover `2e7c55c` + fix `96b9d79` — **none present in my clone**, all **[REPORTED]**. |
| odoo | — | Parallel track reports PRs #2/#3 (Railway parity-oracle container) merged into `19.0` **[REPORTED]**. Default branch is **`19.0`, not `main`** — a `fetch origin main` silently no-ops. |

---

## 2. What I verified myself this session **[VERIFIED]**

- **odoo-rs builds green both ways:**
  - `cargo build --release --features cli` → `od-codegen` compiles serde-only, offline, 41s; binary runs (`--help` ok). This is the ground truth for any **job-container `ENTRYPOINT`**.
  - `cargo build --release --features cli,ogar-emit` → **EXIT 0**, 25s. Fetched + compiled OGAR (`7d0dca2`) and the **ruff AST frontend** (`ruff_python_spo` / `ruff_spo_triplet` / `ruff_python_parser` @ `4860e79`). The "use ruff for AST" convergence path is **complete and compiles** at the pinned revs.
- **od-codegen is a batch CLI, not a server** — zero web-server deps; the "Railway `0.0.0.0:$PORT`" ask does **not** fit the repo *as-is* (nothing listens). The retarget below resolves this correctly.
- **No runtime DB in odoo-rs today** — zero `DATABASE_URL`/`env::var` DB reads; `lance_graph` appears only as doc-comment provenance. Input = static committed ndjson (`data/*.spo.ndjson`); output = SurrealQL DDL text. The **PG-vs-lance-graph storage choice does not exist in this repo yet** — it is the deployment work below, not current behavior.

---

## 3. The messy part, recorded so it isn't repeated

Earlier today I opened **OGAR #156** and **OP #71 without rebasing first**. Both
branches had forked before their mains advanced, so the PR diffs read as massive
deletions (OP: **10,634** — the entire `vendor/AdaWorldAPI-ruff` tree; OGAR:
main's OSM/render body shown as reverts). The operator closed both as
unacceptable. **Fix applied:** both branches rebased so `origin/main` is a clean
ancestor; PR diffs collapse to additive-only (OGAR: 6 files, +607/−46 — the −46
are our own replaced lines; OP: the 10,634 are the intended un-vendor deletion,
now clearly isolated). **Lesson, now doctrine:** *rebase before every PR; a diff
that shows deletions of another track's work is a hard stop, never a footnote.*

Also standing: the operator **no-rev-pins ruling** — AdaWorldAPI-internal git
deps float on the convergence branch; drift protection is the **fuse tests**, not
pins (a floating dep that fails within a build is the fuse working). Flip to
`branch = "main"` when the convergence branches merge.

---

## 4. The retargeted deployment ask — od-hydrate substrate host

The original "Railway `$PORT` Dockerfile for odoo-rs" was correctly rejected (no
server). The **retarget** (operator-framed) is a genuine feature that *does*
legitimately bind `$PORT`:

- **`od-hydrate` substrate host** — a real axum server (binds `0.0.0.0:$PORT`), so the Railway web pattern fits honestly.
- **First-start hydration variable** — check-if-hydrated on the facet table; idempotent (probe before mint).
- **Storage choice at onboarding** — **PG facet table** (`p0..p11` = 3×SPOG ORM shape) **vs lance-graph V3** (same 16B, native). This is the DB-hydration decision, living where it belongs (the runtime host), not in the transpiler.
- **`/sdk/python` (+ /rust, /csharp)** — serve the `emit_*` mirror-back of whatever is hydrated.
- **Surfaces already exist upstream** — `emit_facet_table_ddl`, `emit_python` + prelude, `compile_source` in-process. It is **assembly, not invention**.

Ownership per operator ("take it"): **this handover is mine**; the container
build is being carried by the track with odoo-rs access. If that inverts, the
build needs: odoo-rs GitHub access unblocked for the session + the other track
paused (do not two-write the same crate).

---

## 5. Open tail (non-blocking)

- **od-hydrate container** — the §4 substrate host (axum + tokio-postgres, Dockerfile + railway.toml, the variable contract).
- **Q9** — the OGAR carrier asks (council tail).
- **`odoo_actions` authority table** — deferred, gated on W2′ storage.
- **lance-graph contract nav API** — confirm `5284755`/#669 landed the `screens_reachable_from`/`nav_is_fully_connected` brick before any consumer delegates to it (my clone couldn't see it).
- **medcare lens** — ruff now has the MySQL-DDL stratum (`ruff_csharp_spo::schema`, `684ba99`); the medcare session lifts `Struktur.sql → ModelGraph → compile` through the same door, with corpus-specific acceptance tests kept private (no corpus names/stats in the public fork).

---

## 6. Provenance note

Written by the session that held the **handover**, not the code. Cross-repo PR
claims (#30/#31/#32 odoo-rs, #2/#3 odoo, #669 lance-graph) are **[REPORTED]** by
a parallel track and were **not** independently verifiable from here. Everything
under **[VERIFIED]** was built or read first-hand. When in doubt, trust the cited
commit/PR over this prose.
