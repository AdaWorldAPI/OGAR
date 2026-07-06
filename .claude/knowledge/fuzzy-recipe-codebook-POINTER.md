# POINTER — the fuzzy recipe codebook lives in ruff

> The **method** for cooking a `(verb, criteria)` recipe codebook from
> imperative method-body facts — and correlating fuzzy bodies to the
> declarative recipes OGAR's DO arm (`ActionDef`) lowers — is calcified in the
> producer repo:
>
> **`ruff/.claude/knowledge/fuzzy-recipe-codebook.md`** (+ the `fuzzy-proposer`
> agent in `ruff/.claude/agents/`).

## Why it lives there, not here

The fingerprint is the ruff `Function` body-fact arm (`writes_field` /
`reads_field` / `raises` / `calls` / `writes_if_blank`) — a producer-frontend
concern. OGAR CONSUMES the result: the recipe centroids decide, per method,
whether a body lowers to a declarative `ActionDef` recipe (the 85%:
normalize/default/compute/cascade/guard) or stays a hand-ported imperative core
(the 15%: compensate/write-raise). This is the measured foundation of the 85/15
transpile split (`OGAR-TRANSPILE-SUBSTRATE.md`).

## OGAR-side hooks the doc references

- **F17 body-triage** (`docs/INTEGRATION-MAP.md` + `E-BODY-TRIAGE-ODOO-CONTROL-1`):
  the recipe codebook IS the refinement that rolled the F17 coarse triage
  (93.5%) to the recipe-recoverable band (93.8–98.4%), Redmine leg.
- **SoC families** (reserved, mint-on-emit, `E-RECIPE-FAMILIES-MINT-ON-EMIT`):
  the codebook's §8b SoC proposer feeds them — god-object bucket overflow →
  Concern (`0x06`); duplicate-routes dedup → Scope (`0x05`).
- **J1 `writes_if_blank`** (shipped in ruff): splits the `SelfMap` recipe into
  schema-default vs `normalizes` — the emit-target disambiguation OGAR needs.
- **Config-as-data** (§8c): a detected `config.json`/schema/resolver map is
  ingested as codebook priors, never transcribed — the same rule as the
  `.claude/harvest/` resolver config.
