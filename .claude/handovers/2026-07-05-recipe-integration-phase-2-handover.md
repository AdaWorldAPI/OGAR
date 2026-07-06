# Recipe integration — Phase 2 handover + integration plan (2026-07-05)

> **What this is.** A self-contained handover so a fresh session can execute
> Phase 2 of the recipe-integration arc without re-deriving context. Phase 1
> (the recipe-concept codebook + the lift-time resolver) is **shipped** on
> branch `claude/openproject-nexgen-ogar-review-mkjtpq`. The copy-paste
> **session kickoff prompt** is §9 at the bottom.

---

## 0. Mandatory reads (in this order — do NOT skip)

1. **OGAR `CLAUDE.md`** + `docs/DISCOVERY-MAP.md` + `docs/INTEGRATION-MAP.md` (session preamble).
2. **This arc's canon** — `.claude/board/EPIPHANIES.md`, newest first:
   - `E-RECIPE-FAMILIES-MINT-ON-EMIT` — Scope/Concern reserved, mint-on-emit.
   - `E-RECIPE-CODEBOOK-MINTED-P1` — what Phase 1 shipped.
   - `E-GRAMMAR-IS-THE-RECIPE-SHAPE` — the `<port>::<path>(<shape>)` = SPO-triple unification (the WHY of the whole codebook).
   - `E-F17-PREREQ-VERIFIED` — the gap ledger, verified in code.
   - `E-RECIPE-REUNION-ORDER` — the operator ORDER (the reunion; ORM-for-schema+actions-only; keep AR; Redmine ancestry; ERB→askama).
   - `E-ROUTE-KIND-VERB-STRATA` — **SUPERSEDED**; read only as the cautionary record of a mis-framed council.
3. **The two docs a shallow read skipped this session (READ BEFORE ANY recipe/route/fieldmask work):**
   - `docs/CLASSVIEW-FIELDVIEW-ASKAMA-BITMASK.md` — route dedup IS SoC; `FIELD_MASK_CAP = MAX_SIBLINGS_PER_TIER` (one cap).
   - op-nexgen `.claude/knowledge/RAILS-COVERAGE-KIT.md` §5 — the four recipe families + `RecipeConceptId` + `LabelDto` spec (what Phase 1 implements).
4. **`docs/OGAR-AS-IR.md`** — MANDATORY before adding a field to `Class` / a variant to `ActionDef` (Phase 2 does exactly this).

---

## 1. Where we are — Phase 1 SHIPPED

`crates/ogar-vocab/src/recipe.rs` (new `pub mod recipe`):

- `RecipeConceptId(u16)` — a **typed newtype** (not bare `u16` like `class_ids`): a recipe id `0x0101` and class id `0x0101` (`project`) are numerically equal but different address spaces; the newtype makes mixing them a compile error.
- `RecipeFamily` — `Lifecycle 0x01` / `Guard 0x02` / `Relation 0x03` / `Action 0x04` (+ `0x05`/`0x06` **reserved** for Scope/Concern, unminted — §7).
- 27 promoted concepts in `recipe_ids::*` (RESERVE-DON'T-RECLAIM), `RECIPE_CODEBOOK`, `recipe_concept_id` / `recipe_concept_name` (forward/reverse, exact siblings of `canonical_concept_id`/`_name`), `recipes_in_family`.
- **`recipe_concept_from_surface(surface, lang) -> Option<RecipeConceptId>`** — the lift-time predicate resolver. Ruby + Odoo relation surfaces seeded; the verb-side convergence pin (`belongs_to` ≡ `Many2one` → `REL_MANY_TO_ONE`) is machine-checked.
- 13 unit tests + 3 doctests green (106 lib total, zero regressions), clippy + fmt clean.

---

## 2. The architecture in one screen (so you don't re-derive it)

`E-GRAMMAR-IS-THE-RECIPE-SHAPE`: a recipe **is** a canonicalized SPO triple, and the invocation grammar `<port>::<path>(<shape>)` is that triple.

| grammar | SPO leg | canonicalized by | status |
|---|---|---|---|
| `<path>` = `part_of::is_a` | **subject** — class facet → classid | `ruff_spo_address::mint` (noun codebook, `class_ids`) | shipped |
| `<shape>`'s **verb** | **predicate** — the recipe | `RecipeConceptId` (`ogar_vocab::recipe`) | **codebook shipped; not yet wired** |
| `<shape>` `input[type]` | **object** — typed payload | schema/association stratum | shipped |

The "zoo" = the surface predicate string (`Triple.p` is a `String`). `recipe_concept_from_surface` is the canonicalizer. Phase 2 makes the lift call it.

---

## 3. Gap ledger — verified in code (do NOT restate stale doc claims)

- **(a) writes/calls capture — CLOSED.** `Function::{writes, calls}` shipped in `ruff_spo_triplet` (`ir.rs:264-284`), populated by the Ruby walker, emitted as `writes_field`/`calls` triples.
- **(b) `routes.rb` stratum — OPEN.** HTTP verb / member-collection / return-shape — the Action-kind fact source. Upstream ruff work; not required for Phase 2 (Action surfaces are seeded from `HandlerKind` names today).
- **(c) recipe-concept codebook — HALF-CLOSED.** Codebook + resolver shipped (Phase 1). Phase 2 closes it by wiring.

---

## 4. Phase 2 — the work (the critical path)

**Goal:** the harvested recipe predicates carry their `RecipeConceptId`, so the codebook stops being a parked resolver and the reunion's behaviour arm is canonicalized end-to-end.

**The one design decision (gated by `OGAR-AS-IR.md`): where the id attaches.** Recommended (smallest reviewable, zero-break): an **additive `Option<RecipeConceptId>`** on the lift facets — `None` for unresolved — leaving `Triple.p` as the label skin. Do NOT re-encode the triple predicate itself in this pass.

**Concrete steps:**

1. **`ogar-vocab`** — add `Option<RecipeConceptId>` fields:
   - on the DO-arm `ActionDef` (the Action-kind of a controller method);
   - on the `Class` recipe facets that already exist — `Association`, `Validation`/guard, `Callback` (the Relation / Guard / Lifecycle kinds).
   - All types are `#[non_exhaustive]` already, so this is additive. Default `None`.
2. **`ogar-from-ruff`** (`src/lib.rs` — `lift_actions`, `lift_association`, and the validation/callback lifts) — resolve the surface via `recipe_concept_from_surface(surface, RecipeLang::Ruby)` and populate the `Option`:
   - callbacks: the phase token before the `:` in `"<phase>:<target>"` → `LIFECYCLE_*`.
   - validations: the kind string → `GUARD_*`.
   - associations: the assoc kind → `REL_*`.
   - actions (controller methods from `extract_tree_with`): the `HandlerKind` name → `ACTION_*`.
3. **Tests** (`-p ogar-vocab -p ogar-from-ruff`): a fixture `Model` → lift → assert the ids stamped; **re-assert the convergence pin at the lift level** — OP `belongs_to` and Redmine `belongs_to` both stamp `REL_MANY_TO_ONE`.
4. **Board hygiene:** prepend `E-RECIPE-CODEBOOK-WIRED` (gap (c) → CLOSED), update the gap ledger; mirror a one-line note in op-nexgen's board.

**Scope discipline:** `cargo test -p ogar-vocab -p ogar-from-ruff`; `cargo clippy -p …`; **NEVER `--all` / `--all-targets` from OGAR** (path-deps would drag siblings — the tesseract-rs iron rule applies to any path-dep workspace).

---

## 5. After Phase 2 — the measurement gate (the C5 A/B)

The pre-registered **OP⇄Redmine verb A/B** (`E-RECIPE-REUNION-ORDER`, redmine-op plan C5): classify both ports' route/recipe surfaces and measure the shared-`RecipeConceptId` collapse rate. **Pre-register the KILL threshold (collapse-rate %) BEFORE running** — the noun side (26/26) is *asserted*, not measured, so the verb side may not borrow it as a measured precedent. This is distinct from the noun-side convergence and does not stand in for it.

---

## 6. ⚠ The operator decision that blocks consumer adoption — CONSUMABILITY

All recipe work is on `claude/openproject-nexgen-ogar-review-mkjtpq`, but **op-nexgen (and every consumer) deps OGAR via `branch = "claude/odoo-rs-transcode-lf8ya5"`** (the convergence branch). So `ogar_vocab::recipe` is **NOT reachable by any consumer** until it lands on the convergence branch OR the consumer dep is repointed. That is a branch/merge-strategy decision for the **operator** — do not unilaterally repoint or retarget. Phase 2 itself is fully doable OGAR-side without resolving this; consumer adoption (op-nexgen's `OpHandlerKind` → `RecipeConceptId`, retiring the per-consumer enum) is what's gated.

---

## 7. Scope / Concern — reserved, mint ON EMIT (do NOT pre-mint)

`E-RECIPE-FAMILIES-MINT-ON-EMIT` (operator rule). Bytes reserved, no variant/concept until the emit seam exists:

- **`0x05` Scope** — mint when the ruff **DTO-AST route-dedup** path emits a named filtered view (Rails `scope`/`default_scope`, or a `ClassView` fieldmask standing in for N routes).
- **`0x06` Concern** — mint when the **god-object split** emits it (the `ruff_spo_address::soc` `Conflation` verdict → sub-ClassViews; Rails `concerns`/mixins).

Phase 2's controller-action lift may be where Scope first wants to emit (a scoped index handler). If so, mint it then — not before.

---

## 8. Hard lessons from this session — do NOT repeat

1. **Do NOT run the 5+3 council against a standing operator ruling.** This session convened a council that *rejected* the route-dedup↔SoC unification as "mere rhyme" — but the operator had canonized it a week earlier; the council was mis-framed (never pointed at the rulings). Read the plans; execute canon; only council genuinely-new claims.
2. **Verify gap-ledger claims in CODE before restating them.** A stale "ruff captures reads not writes" claim propagated into a board entry; the code had shipped `writes` months earlier. Grep the symbol, read the source.
3. **Read `CLASSVIEW-FIELDVIEW-ASKAMA-BITMASK.md` + `RAILS-COVERAGE-KIT.md` §5 before ANY recipe/route/fieldmask work.** The whole mis-framing traced to not reading them.
4. **Scope cargo to `-p <crate>`; never `--all` from OGAR.**
5. **Board is append-only:** regrade `**Status:**` in place, prepend corrections; never edit an entry body.

---

## 9. SESSION KICKOFF PROMPT (copy-paste to start the next session)

```
You are continuing the OGAR recipe-integration arc. Phase 1 (the recipe-concept
codebook `ogar_vocab::recipe` + the lift-time resolver `recipe_concept_from_surface`)
is SHIPPED on branch `claude/openproject-nexgen-ogar-review-mkjtpq`. Your task is
Phase 2: wire the resolver into the `ogar-from-ruff` lift so harvested recipe
predicates carry their canonical `RecipeConceptId`.

FIRST, read (mandatory, in order):
- OGAR CLAUDE.md, docs/DISCOVERY-MAP.md, docs/INTEGRATION-MAP.md
- .claude/handovers/2026-07-05-recipe-integration-phase-2-handover.md  (full context + the Phase 2 spec — §4 is your work order)
- .claude/board/EPIPHANIES.md — the top 6 entries (the recipe arc)
- docs/CLASSVIEW-FIELDVIEW-ASKAMA-BITMASK.md
- docs/OGAR-AS-IR.md
- openproject-nexgen-rs/.claude/knowledge/RAILS-COVERAGE-KIT.md §5

Those entries carry OPERATOR RULINGS. They are canon — execute them, do NOT
re-litigate. In particular: do NOT run the 5+3 council to "test" the
route-dedup↔SoC unification or the AR-shape reunion; a council did that this
session and wrongly rejected settled canon (see E-ROUTE-KIND-VERB-STRATA,
SUPERSEDED). Verify any gap-ledger claim in code before restating it.

THEN execute handover §4:
- ogar-vocab: additive `Option<RecipeConceptId>` on `ActionDef` + the `Class`
  recipe facets (Association/Validation/Callback), default None.
- ogar-from-ruff: populate them via `recipe_concept_from_surface(surface,
  RecipeLang::Ruby)` at lift (callback phase→LIFECYCLE_*, validation
  kind→GUARD_*, assoc kind→REL_*, controller HandlerKind→ACTION_*).
- Tests incl. the OP⇄Redmine convergence pin at the lift level.
- Board: prepend E-RECIPE-CODEBOOK-WIRED, regrade gap (c) → CLOSED; mirror in
  op-nexgen's board.

Scope: `cargo test/clippy -p ogar-vocab -p ogar-from-ruff`. NEVER `--all` from
OGAR (path-deps drag siblings). Commit small + push to the same branch. Do NOT
open a PR unless asked.

FLAG, don't resolve: consumers dep OGAR via `claude/odoo-rs-transcode-lf8ya5`,
so this work isn't consumable until the operator decides the branch/merge
strategy (handover §6). Phase 2 is fully doable OGAR-side regardless.
```

---

## Appendix — arc commit trail (branch `claude/openproject-nexgen-ogar-review-mkjtpq`)

OGAR (6): `b792002` route-kind-verb-strata → `8503948` reunion-order correction → `5dd2be1` F17-verified → `e02608b` grammar-is-recipe-shape → `f76698c` recipe codebook Phase 1 → `34fa06c` Scope/Concern reserved.
op-nexgen (5): `22c6826` park dto-check → `f942c77` reunion-order → `379580d` F17-verified → `603610b` grammar mirror → `83426d7` codebook-P1 mirror.
