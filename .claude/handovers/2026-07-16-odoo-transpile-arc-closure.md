# Handover — odoo transpile arc CLOSED on the OGAR side; live seams named

> **Date:** 2026-07-16 · **Closes** `2026-07-07-odoo-transpile-v3-w1-w2-shipped-w3-handoff.md`
> (every item in that handoff has since merged). Append-only: that file is
> not edited; this one records the closure + where the arc's energy goes next.

## What closed (all merged to mains)

| Item (from the 07-07 handoff) | Landed as |
|---|---|
| W1 kausal-parity consume pin (real-source witness) | OGAR **#192** — `tests/odoo_kausal_parity_probe.rs`, 11/11 kausal (8 `Depends` + 3 `Constrains` post ruff #49), facts-only guard. Ledger `D-KAUSAL-CONSUME-PIN-ODOO` |
| W2 V3 SoA sink for `CompiledClass` | OGAR **#192** — `ogar-from-ruff::lance_sink` (feature `lance-sink`): `compiled_class_to_facet` / `compiled_class_to_noderow` / `compiled_classes_to_le_bytes`; field-isolation matrix; `ENVELOPE_LAYOUT_VERSION == 2` fuse. Ledger `D-V3-SINK-COMPILEDCLASS` |
| e2e capstone (real .py → CANON NodeRow bytes) | OGAR **#192** — `tests/odoo_v3_sink_e2e.rs` |
| odoo-rs corpus DO-arm deletion + Stage-C fork delete | odoo-rs side, earlier arc (kausal witness `c2094b4`; `surreal_ast.rs` deleted) |
| Region grammar (knowledge transfer of ruff #76) | ruff **#79** (`extract_odoo_view_regions`; `RegionFact`/`region_triples` promoted into shared `ruff_spo_triplet`) + odoo-rs **#35** (Edit 3 digest, live-harvest byte-parity fuse) |

Verified this session on current mains (OGAR `df7331e`, ruff `6604f45`):
`cargo test -p ogar-from-ruff --features lance-sink` → 86 lib + 5 integration
green. Branch `claude/odoo-rs-v3-ogar-transpile-nwriny` reset to main
(merged-branch policy).

## The two live seams (where the next session plugs in)

1. **W2's out-of-scope tail lands in odoo-rs `od-server`.** odoo-rs #36
   shipped in-process `/compile` (Odoo `.py` → `CompiledClass` JSON) and
   names the hydration writer as the follow-up. The lance leg IS
   `lance_sink` — `compiled_class_to_noderow` → `NodeRowPacket::as_le_bytes`
   → `Dataset::write` (the one step W2 deliberately stopped short of). The
   postgres leg is `ogar-adapter-postgres-ddl`. Consumer executor wiring:
   `.claude/knowledge/hotplug-consumer-migration.md` (`HotPlug` +
   `resolve_hotplug`).
2. **Region grammar feeds the a2ui screen-addressing arc** (#204 proposal,
   #206 `ogar-a2ui-frame`, #207 fieldview). A screen is a nested ClassView
   projection; odoo-rs's `docked_at`/`tab_order`/`opens_popup` facts +
   `odoo_regions.conf` are the harvested Odoo layout feed for that
   projection (region → position in the parent's ordered set). ⚠ WIDE
   masks only (codex P2 on #204/#205): `>64`-field surfaces break narrow
   `FieldMask` — never route RBAC through `ClassRbac::field_mask` narrow.

Authoritative forward plan: odoo-rs
`docs/HANDOVER-2026-07-16-transpile-plan-v3.md` — landing via **odoo-rs
PR #37**; until that merges, the file lives on the odoo-rs branch
`claude/odoo-rs-v3-ogar-transpile-nwriny` (commit `593b9c2`), not on
odoo-rs `main`.
