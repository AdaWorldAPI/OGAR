# Handover: lance-graph orchestrator → ogar-loco session (2026-08-05)

> APPEND-ONLY. Consumer-side wishlist for the loco ABI, written after a
> source-verified review of merged #241 (R4 data-first vocabularies, R5
> statement boundaries) from the seat that will consume it: lance-graph's
> rig/rs-graph-llm oracle loop and the compiled-template stack. Every item
> respects #241's recorded-not-minted list — nothing here asks for gated
> mints, and all ids stay the operator's.

## What I did (context for this wishlist)

- Verified #241 claim-by-claim against merged source (`0d39025`). All
  load-bearing claims code-true: unforgeable shared half in
  `VocabularyTable::compose` (test `the_composed_tables_shared_half_is_
  unforgeable`), table-backed `CheckedVocabulary`, the four-way refusal in
  `statement_bounds`, `DOMAIN_FLOOR = 0x90` with the compile-time pin.
- Sketched the consumer loop this wishlist serves: LLM (via lance-graph's
  shipped rig CompletionModel adapter) emits N candidate shortcode bodies →
  `validate` + `statement_bounds` refuse the garbage → the surviving bag is
  executed and scored by shipped falsifiers (the Strict/Aware projection
  machinery). The refuse-don't-guess design of #241 is the LLM-output
  firewall; `table()`'s doc already names oracle schemas as a consumer.

## FINDING (one, small, letter-vs-spirit)

**F-1 — validate() samples the domain hooks twice.** `check(&v)` runs, THEN
`compose(&v)` samples the hooks again. For a hooks-unstable vocabulary the
stored table is a second, unvalidated sampling: `check` can pass on answers A
while the table freezes answers B — including a `body_refs` beyond 3, which
re-opens the traversal edge the wrapper's proof claims closed
(`program.rs`'s `debug_assert`s already half-know this: they name "a
hypothetical non-deterministic vocabulary" and degrade to a safe index panic
in release). Shared half unaffected (stamped from core); stable vocabularies
unaffected. Severity: pathological-input-only, fail-loud not corrupting.

## The wishlist (consumer-priority order)

**W-1 — Compose-then-check hardening (closes F-1).** Compose the table
first, then validate: keep the method-drift half of `check` exactly as is
(drift detection inherently needs the method reads), and add a shape check
over the STORED table's own `body_refs` column so the artifact everything
reads is the proven object. A few lines + one can-fire test with an
unstable-hooks vocabulary. Makes "what was validated is literally what gets
read" true to zero resamplings, before the second vocabulary lands on top.

**W-2 — The name column (the prompt-legend enabler).** The table carries
semantics but no names: `FnIndex` constants are named in code only. An LLM
legend / oracle schema / renderer needs `name(f) -> Option<&'static str>` —
shared-core names in the core, a `domain_name` hook for siblings, composed
into the table (or beside it) under the same unforgeable-shared-half rule.
Without it every consumer hand-rolls a name map that can drift; with it the
legend is a serialization of the validated table itself, at the membrane
where serialization is legal.

**W-3 — The NARS-34 vocabulary mint (ids operator-gated, as always).** The
34 rung-3 tactic recipes as the first reasoning vocabulary above the floor.
Two constraints discovered from the consumer side:
- The slot arithmetic forces recipes-not-atoms anyway: the domain range
  holds 112 slots; the 144 rung-2 atoms don't fit one sibling, and their
  unification is gated. 34 fits with room.
- `pushes_result` must be declared per verb at mint time, and the mint is
  where the `ResultBehavior`/`statement_terminal` question comes due: a
  declared-pushing verb whose result is legally discarded produces SILENT
  MASK-GRANULARITY COARSENING, not refusal — depth never returns to zero
  mid-body, and the trailing-value rule merges intended statements into one
  mask unit. Execution stays correct; per-statement masking silently
  degrades. The can-fire test for that trace should ride in WITH the mint.

**W-4 — Funnel telemetry as data (the oracle contract's safe half).** The
generate-and-filter loop needs refusal STATISTICS: per candidate, which gate
killed it (`ConformanceError` / `StatementError` variants are already typed
— a small summary struct over a batch suffices). Distinction worth pinning
in the doc when this lands: **validity feedback** (your candidate refused at
underflow, call 7) is safe to loop back to the generator raw; **fitness
feedback** (your candidate scored κ=X) falls under lance-graph's
observer-effect payload law (distribution shape × rank, never the raw
scalar) and stays out of the loop until that instrument exists. #241 parked
the oracle contract; this is its smallest safe slice.

**W-5 — The >64-statement split contract (doc-only).** `statement_bounds`
already yields the count; what a lowerer must DO at statement 65 is
undocumented ("a split signal, not a mask-widening use case" — but split
how: second function? second template?). One paragraph in the module doc
prevents N divergent conventions.

## Explicitly NOT asked

No 144-atom unification (gated). No structured PA control mints (ids the
operator's). No replay/session representation. No cross-repo dependency in
either direction — the consumer wants the TABLE AS DATA (W-2 makes it
self-describing); lance-graph never becomes an ogar-loco dependency and
ogar-loco never imports an LLM adapter.

## Open questions for the loco session

- OQ-1: Should the name column live IN `FnSpec` (one more field, table stays
  the single artifact) or beside it (keeps `FnSpec` `Copy`-tiny)? Consumer
  has no preference beyond "same unforgeable-shared-half rule".
- OQ-2: `DOMAIN_FLOOR = 0x90` makes the shared range exactly 144 slots — the
  same number as the rung-2 verb atoms. Almost certainly coincidence (the
  core covers ~50 today), flagged only so the option is visibly open if the
  gated unification question ever returns. Numerology until ruled otherwise.
