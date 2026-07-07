# Handover — odoo→odoo-rs V3 transpile: W1+W2 shipped (OGAR side), W3 handed to an odoo-rs-scoped session

> **Date:** 2026-07-07
> **Branch (this repo):** `claude/odoo-rs-v3-ogar-transpile-nwriny`
> **Why this lives in OGAR, not odoo-rs:** the prior handover
> (`odoo-rs/docs/HANDOVER-2026-07-06-odoo-transpile-v3.md`) is in the
> **private `odoo-rs` repo, which is NOT in every session's scope**. This
> session had OGAR/ruff/lance-graph access but `odoo-rs` was hard-blocked at
> the session allowlist (add_repo, git-proxy, PAT-URL, GitHub-MCP, and repo
> search all denied it). So the W3 spec is recorded HERE, in an accessible
> repo, anchored to the shipped commits.

## Status of the arc

harvest ✅ → fingerprint quartet ✅ → lift+mint+carry ✅ (#164) → consume ✅
→ SurrealQL deprecated ✅ → AT-CARRY-2 kausal upstream ✅ (#168) →
**W1 kausal-parity consume (OGAR side) ✅** → **W2 V3 SoA sink ✅** →
**end-to-end capstone ✅**. Remaining: **W3 (odoo-rs-side deletions).**

## Shipped THIS session (OGAR `claude/odoo-rs-v3-ogar-transpile-nwriny`)

| Commit | What | Tests |
|---|---|---|
| `8319690` | **W1** — kausal-parity consume pin. Real Odoo 19 source (`account_payment_term.py`, `AdaWorldAPI/odoo` `2c78d5f1`, byte-identical, PROVENANCE.md) drives `ruff_python_spo::extract_from_source → compile_graph_python::<OdooPort> → lift_actions`; pins 8/8 `@api.depends` compute methods → `KausalSpec::Depends` verbatim, 8 plain methods kausal-free. Ledger `D-KAUSAL-CONSUME-PIN-ODOO`. | `tests/odoo_kausal_parity_probe.rs` 1/1 |
| `ae6bb0f` | **W2** — V3 SoA sink for `CompiledClass`. `crates/ogar-from-ruff/src/lance_sink.rs` behind feature `lance-sink` (zero-dep `lance-graph-contract` only, **no `*Bridge`**). `compiled_class_to_facet` (byte-identical reinterpret → `FacetCascade`, L1 rails `G6D2`), `compiled_class_to_noderow` (classid in key, bootstrap tail, `ValueSchema::Bootstrap`), `compiled_classes_to_le_bytes`. Mirrors `contract::network` (shape) + `symbiont::bridge` (idiom). Ledger `D-V3-SINK-COMPILEDCLASS`. | 7/7 incl. field-isolation matrix + `ENVELOPE_LAYOUT_VERSION==2` fuse |
| `dfc644e` | **Capstone** — end-to-end `account_payment_term.py → ruff → OGAR → 2 CANON NodeRow (1024 B)`, no SurrealQL. | `tests/odoo_v3_sink_e2e.rs` 1/1 (feature `lance-sink`) |

Full crate: `cargo test -p ogar-from-ruff --features lance-sink` = **78 lib + all integration green**; `cargo fmt`/`clippy` clean. Ruff **#51 is MERGED to `main`** (verified via GitHub API: `merged_at 2026-07-06T21:38:44Z`, merge commit `9ef26c1`, base `main`) — downstream `ogar-from-ruff` floats on it green (D-NEVER-PIN-BUMP).

## W3 — what a session WITH odoo-rs access must do (ordered)

The W1 witness (`8319690`) is the gate that was blocking these deletions; it is now green, so they are UNBLOCKED.

1. **Delete the odoo-rs local corpus DO-arm.** The deprecated odoo-rs corpus witness is superseded by the OGAR-side real-source witness (`D-KAUSAL-CONSUME-PIN-ODOO`, fixture `account_payment_term.py`). With W1 green, remove the local corpus DO-arm in odoo-rs and repoint any consumer at the OGAR pipeline (`ruff_python_spo → compile_graph_python::<OdooPort> → lift_actions`). Only Arm A (`compute=`/`@api.depends`) is witnessed; **arms B (`@api.constrains`/`@api.onchange`) + D (`computed.stored`) remain out of scope pending ruff #49** — do NOT delete anything those arms feed until #49 lands and a follow-up OGAR pin covers them.
2. **W3 Stage-C fork delete**, modulo **AT-CARRY-3 (`body_source`)** — the named blocker in the prior odoo-rs handover. Confirm `body_source` carry is resolved before the Stage-C fork is deleted.
3. **(Optional, engine-side) finish the W2 sink's out-of-scope tail:** the actual Lance `Dataset::write` of the `NodeRowPacket::as_le_bytes` output + kanban transition need the lance engine / ractor runtime. `lance_sink` stops at `as_le_bytes()` on purpose (mirrors `contract::network` stopping at the facet, and `symbiont::bridge`'s BLOCKED tombstone write). `[H2]` embedding the 12-byte rail payload into a NodeRow value tenant needs a new append-only `ValueTenant` (v3-envelope-auditor-gated) — deferred, mirror network (added no lane). `[H1]` rail-chain↔key-tail reconciliation stays frozen (tail bootstrapped, test-enforced T-D). `[H3]` L1 rails only; L6 "odoo?" quads unruled — do not implement.

## Gotchas (carried forward)

- **D-NEVER-PIN-BUMP:** every cross-repo dep floats on `branch=main`; drift protection is loud breaks + fuse tests, never rev pins.
- **PAT-URL pushes:** OGAR/ruff push via `https://x-access-token:${GH_TOKEN}@github.com/AdaWorldAPI/<repo>` — the session `origin` git-proxy prompts for a password and fails.
- **odoo-rs scope:** if `odoo-rs` is again absent from the session allowlist, W3 CANNOT be done — request the session be scoped to include `AdaWorldAPI/odoo-rs`.
- **Cargo.lock gitignored** in these crates; **no PR created in-session** (push branch, open PR out-of-band).
