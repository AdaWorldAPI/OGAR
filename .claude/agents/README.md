# `.claude/agents` — the 5+3 fan-out / adversarial-review pattern

A reusable harness for **hardening a claim before it is written into the substrate
canon** (ADRs, `DISCOVERY-MAP.md`, the synergy catalog). It exists because the
expensive failure mode here is not "missing an idea" — it is **letting a
hypothesis masquerade as grounded** until it dilutes the shape of the
architecture downstream.

## The pattern

```
                      ┌─ arxiv-grounder ──────────┐
                      ├─ runtime-archaeologist ────┤
   claim / change ───▶├─ theorem-checker ──────────┤──▶  orchestrator
                      ├─ cross-domain-synthesizer ─┤      synthesises a
                      └─ doctrine-keeper ──────────┘      graded draft
                                                                │
                                                                ▼
                      ┌─ overclaim-auditor ────────┐
                      ├─ dilution-collapse-sentinel ┤──▶  3 BRUTAL reviews
                      └─ firewall-warden ──────────┘      (block / pass)
                                                                │
                                                                ▼
                                              orchestrator applies, grades,
                                              commits — or sends back
```

**5 research agents** fan out (parallel) — one lens each, no redundancy:

| agent | lens | promotes a claim to… |
|---|---|---|
| `arxiv-grounder` | primary literature | `[G]` if a paper *proves* it; `[H]` if it merely suggests |
| `runtime-archaeologist` | shipped code / receipts | `[G]` only if it can point at `file:line` |
| `theorem-checker` | the math | `[G]` if a theorem closes it; checks every asserted number |
| `cross-domain-synthesizer` | analogy honesty | MECHANISM-SHARED `[H]+` vs MERE-RHYME `[S]` |
| `doctrine-keeper` | canon consistency | flags every mirror/cross-ref a change must update |

**3 brutally-honest review agents** then attack the *draft* (parallel). They do
not soften, do not rubber-stamp, do not say "looks good" without naming why each
claim earns its grade:

| agent | catches |
|---|---|
| `overclaim-auditor` | grade inflation; absolute words (`cannot`/`guarantee`/`proven`) on `[H]`/`[S]` |
| `dilution-collapse-sentinel` | conflating motifs that must stay separate (dilution); deleting a valid leg (collapse) |
| `firewall-warden` | PII label leak, model-id in artifacts, hot-path serialization, prohibited shell tools |

## Grading discipline (the whole point)

- **`[G]` grounded** — closed by a theorem, shipped code, or a measurement. Pinnable.
- **`[H]` hypothesis** — plausible and bounded, but unproven. **Must carry a named test.**
- **`[S]` speculative** — analogy/rhyme only. Catalog it; **do not build on it.**

A claim may carry **different grades on different legs** (e.g. a bundling that is
`[G]`-real but whose *irrational* reading is `[S]`). Keep the legs separate — that
separation is what stops dilution.

## How to invoke

These are `subagent_type` definitions. Freshly-added files are picked up on the
next session; within a running session the orchestrator runs the same charters
via `general-purpose` agents. Fan out the 5, synthesise a graded draft, then fan
out the 3 reviewers on that draft. The orchestrator is the **only** writer — the
sub-agents are read-only by construction (the review trio carries no edit tools).

## Non-negotiables (every agent enforces, the warden blocks on)

- The Firewall (ADR-022): **no serialization in the hot path**; the IR is wire-truth (ADR-023).
- **PII**: never emit German PII labels (`Geburt*`, `Krankenkasse`, `Versicherten*`,
  `Diagnose`, `Vorname`, `Nachname`, `Geschlecht`, `Krankenversicherung`) — medcare-rs
  labels must not leak into any artifact.
- Never write the model identifier into a committed artifact (chat only).
- Prohibited shell: `grep`/`sed`/`tail`/`head`/`awk`/`echo` via Bash — use the
  `Grep`/`Read`/`Glob` tools.
