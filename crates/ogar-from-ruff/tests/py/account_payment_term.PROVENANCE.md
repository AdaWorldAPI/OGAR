# Provenance — `account_payment_term.py`

- **Source repo:** `AdaWorldAPI/odoo` (LGPLv3, mirrored via the local read-only
  clone at `/home/user/odoo`)
- **Commit:** `2c78d5f1ff1d4e32b0c480c3b2714f31fb71bef7`
- **Original path:** `addons/account/models/account_payment_term.py`
- **Copied:** 2026-07-06
- **Copy method:** byte-for-byte (`diff` verified identical against source)

## Purpose

Kausal-parity **consume** witness (W1 of the odoo→odoo-rs transpile arc,
`D-KAUSAL-CONSUME-PIN-ODOO`). This is real, unmodified Odoo 19 source —
it replaces the deprecated odoo-rs corpus witness as the fixture that
proves `ruff_python_spo::extract_from_source` → `ogar-from-ruff::lift_actions`
reproduces the file's `compute=` / `@api.depends(...)` reactive wiring
(`ActionDef.kausal = Some(KausalSpec::Depends { paths })`) verbatim, dotted
paths and all, off REAL production source rather than a synthetic or
hand-authored stand-in.

The file declares two Odoo models (`AccountPaymentTerm` /
`account.payment.term` and `AccountPaymentTermLine` /
`account.payment.term.line`), giving the probe both a multi-model graph
and, within `AccountPaymentTerm._compute_example_preview`, a single compute
method that emits two distinct fields (`example_preview` +
`example_preview_discount`) from the same `@api.depends(...)` set — an
edge case the synthetic WoA/`account_move` fixtures used elsewhere in this
crate do not exercise.

See `crates/ogar-from-ruff/tests/odoo_kausal_parity_probe.rs` for the pinned
assertions and `docs/DISCOVERY-MAP.md`'s `D-KAUSAL-CONSUME-PIN-ODOO` entry
for the full writeup (including scope: constrains/onchange arms and
computed.stored parity remain out of scope, pending ruff #49).
