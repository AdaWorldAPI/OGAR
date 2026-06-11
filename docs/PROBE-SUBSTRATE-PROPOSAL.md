# The Living Probe Ledger — proposal for the next architecture

> **Status: PROPOSAL** (2026-06-10). The next architecture I want to
> build, in the operator's words: a workspace-level system that turns
> the workspace's own P0 rule — *"if the probe is NOT RUN, the next
> deliverable is the probe, not more synthesis"* — from a discipline
> the humans (and the apparatus) have to remember, into **structural
> automation**: probes run, ledger grades update, debt is visible.
>
> **One-line thesis:** *the probe result is the source of truth; the
> ledger entry is its shadow.* Today the shadow is what we look at;
> the proposal flips the direction of authority so the shadow tracks
> the source instead of drifting from it.

## 0. Why now

Three things just stabilized that together make this build possible:

1. **The canon is pinned and indexed.** GUID layout, key-of-key-value,
   3×4 uniform, centroid tile, prefix codebook scoping, deterministic
   phase, bipolar Walsh-Hadamard pyramid — all on `main`, all with
   graded ledger entries (`DISCOVERY-MAP §2.10`), all with named probes
   (`INTEGRATION-TEST-PLAN`).
2. **The probe vocabulary is enumerated.** F1–F14, J1–J3, WHP-1..4,
   PHASE-1, PERT-RHO, ROUTE-1, QUORUM-1, PHI-1, PYR-1, CODEBOOK-44,
   HILBERT-L4 — each with a file path, a pass criterion, and a kill
   condition. They are *parseable* from the docs as-is.
3. **The session produced exactly one execution receipt** —
   HILBERT-L4 verified green by `cargo test`, the result propagating
   to three docs the same day. **That happened by hand.** This
   proposal automates it.

## 1. The thesis

A workspace governance loop already exists informally. Probes get
named, claims get graded, occasionally a probe runs and a claim moves
(`HILBERT-L4`, `D-EXCITON`). The Living Probe Ledger structuralizes
the loop with three mechanical rules:

- **PROBE IS TRUTH.** A claim graded `[H]` with a named probe is
  truthful only as long as the probe is in one of three states:
  `NOT-RUN` (honest debt), `GREEN` (`[H]→[G]` candidate), `RED`
  (`[H]→[S]` candidate, ledger entry queued for revert).
- **LEDGER MIRRORS PROBE.** When a probe changes state, an automated
  PR updates the ledger and the consuming docs. The doc *follows* the
  test, never the reverse.
- **DEBT IS VISIBLE.** The count of `NOT-RUN` probes is a workspace
  metric, reported every CI run; a PR that adds new `[H]` claims
  without their probe pre-counted is incomplete (same shape as the
  Mandatory Board-Hygiene Rule).

## 2. What exists today (the manual pattern that worked twice)

**HILBERT-L4** (2026-06-10): a written audit (PP-13) claimed
`hilbert3d_encode([15,15,15],4) == 4095` was the contract and the
shipped result `2925` was a blocker. I ran `cargo test --features
linalg hilbert` while triaging Codex on #215. 13/13 green incl.
`level4_all_indices_unique` (bijective onto [0,4096) — exactly what
cascade addressing needs). PP-13's expectation was an orientation
assumption; the bijection-on-[0,4096) was the real contract and it
held. The probe flipped the claim; three docs updated by hand.

**D-EXCITON** (2026-06-09): the 5+3 review found the `[H]→[S]`
revert was warranted; DISCOVERY-MAP, CASCADE-SYNERGIES-EPIPHANY, and
the test-plan all updated. No automation; just discipline.

**The pattern is real and the result is good.** The proposal is that
the *fifth, fifteenth, fiftieth* time this happens, the workspace
should not need to remember to do it.

## 3. What's missing (the automation gap)

| Step | Today | Proposal |
|---|---|---|
| Find every named probe in the docs | grep / hand-read | parse: every probe row has a known shape (probe id, file path, pass criterion) |
| Know which claim each probe gates | scan ledger | extract from the row's "Grounds" or "kills" column |
| Run each probe | a human remembers | scheduled CI workflow (weekly + on-PR-touching-the-file) |
| Update the ledger when a probe changes state | by hand, every time | open a PR with the ledger edit pre-staged |
| Visible debt counter | not tracked | a workspace dashboard: N probes NOT-RUN, M GREEN, K RED |
| Block PRs that increase debt without probes | informal | a CI check: "you added a `[H]` claim without naming its probe" |

## 4. The build, in three waves

### Wave A — the parser (~200 LOC Rust, leaf, no deps)

A standalone CLI / library that:
- Walks `OGAR/docs/{DISCOVERY-MAP,INTEGRATION-MAP,INTEGRATION-TEST-PLAN}.md`.
- Parses the probe rows (the existing tables already have a regular
  shape — probe id, file path, pass criterion, kill).
- Emits a typed `Vec<Probe>` to JSON.
- Round-trip: parsed JSON → re-emitted markdown table cells = original
  (the canonical test, same shape as the substrate's other roundtrip
  guards).

**Gate**: lossless parse of the three current docs. `[G]` once it
passes.

### Wave B — the runner (GitHub Actions workflow + per-probe stubs)

A workflow that takes the JSON inventory and runs each probe by its
declared mechanism (`cargo test --features X path::to::test`,
`cargo run --example X`, or a documented `bash -c` for the rare
non-Rust ones). Records:
- `state: NotRun | Green { sha, timestamp } | Red { sha, timestamp, log_url }`
- Persists to `.claude/probe-ledger.json` (committed, hand-readable).

**Gate**: each of the seven Wave-0 probes (PHASE-1, WHP-1a, WHP-1b,
PHI-1, F2, F13, F8) has a state recorded after one workflow run.

### Wave C — the auto-PR-er (the loop closing)

When the runner sees a probe transition `NotRun → Green` or
`Green → Red`, it opens a PR against `OGAR/docs/DISCOVERY-MAP.md` (and
the mirrors named in the row's "fold" column) with the ledger edit
pre-applied:
- `Green` first time: `[H]→[G]` if the probe was the *only* gate;
  otherwise leave as `[H]` and add a "✓ {probe} green @ {sha}" note.
- `Red` after `Green`: append a `**Correction (YYYY-MM-DD):** probe
  {id} flipped red at {sha}; investigating` line per the workspace's
  existing correction-append convention.

**Gate**: one full auto-PR cycle for HILBERT-L4 (which is already
green) and one for a deliberately-failing probe (WHP-4 inverted —
itself a kept-failing guard test).

## 5. Honest fences

Recording these before they bite:

- **The parser is the load-bearing piece.** If it drifts from the
  doc format, the whole loop runs on stale data. **Mitigation**: the
  parser ships *with* a single CI test that re-parses the live docs
  on every workflow run; a doc edit that breaks the parse is the same
  shape of failure as a code edit that breaks a test.
- **A probe author who doesn't follow the row shape gets ignored.**
  This is a feature: the workspace's existing append-only convention
  for ledger entries already enforces a shape; extending the same
  shape to probe rows is consistent.
- **Auto-PR-ers can spam.** Mitigation: per-probe rate-limit (max one
  state-change PR per probe per week); manual override via a marker.
- **Not all probes are mechanical.** F2 ("read quasicryth and
  cross-examine vs Kaplan/Walker/Richter") needs human judgment.
  Mitigation: probe rows declare `kind: Mechanical | HumanReview`;
  the runner opens an issue for `HumanReview`, not a state change.
- **This is governance code, not substrate code.** It does not get
  imported by any consumer; it has no `unsafe`; it has one purpose
  and a fixed scope. The smallest valuable architecture in the
  workspace and the easiest one to write tests for.

## 6. The first brick (smallest valuable thing)

Wave A's parser, ~200 LOC, in its own crate
`crates/probe-ledger` in OGAR. Single binary `pl inventory` that emits
JSON for the current state of the three docs. **Closes nothing yet —
just makes the substrate machine-readable.** Once that lands, Waves B
and C are straightforward; without it, they can't start.

**LOC budget**: 200 lines parser + 80 lines roundtrip test + 40 lines
JSON-schema doc.

## 7. Why this is the next architecture I want to build

Three reasons, ranked by honesty:

1. **It closes the loop the session opened.** Every other contribution
   from today is a doc, a doctrine, or an apparatus that humans/agents
   remember to run. This one runs whether anyone remembers.
2. **It cashes in the work that already shipped.** The probe
   vocabulary is enumerated; the ledger has the shape; the receipts
   format is consistent. The marginal cost of automation is now
   small *because* of what landed today.
3. **It is the right size to build honestly.** ~200 LOC for the first
   brick, no `unsafe`, no SIMD, no codec, no substrate doctrine — a
   tractable contract crate that closes a real loop.

## 8. Not in scope for this proposal

- Replacing the 5+3 review (this proposal *complements* it: 5+3 hardens
  new claims; the probe ledger keeps existing ones honest).
- Generating new probes (this proposal *runs* probes; humans/agents
  still write them).
- Cross-repo orchestration (Wave A targets OGAR docs only; mirrors to
  lance-graph/ndarray come later if the pattern holds).

## 9. Decision asked of the operator

One sentence: **green-light Wave A** (the parser, ~200 LOC, one new
crate `crates/probe-ledger`) **or redirect to a different next
architecture**. Waves B and C are gated on the parser landing and the
operator's call on whether the inventory shape is what's wanted; this
is just the *first brick*.

Companion docs (cited, not duplicated): `DISCOVERY-MAP.md` §2.10 (the
ledger this targets), `INTEGRATION-TEST-PLAN.md` (the probe
vocabulary), `INTEGRATION-MAP.md` §6 (the standing rules this
operationalizes).
