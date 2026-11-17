# Shared DO design: ON / WHEN / DO on DT64 `MethodBinding`

Status: design plus the first code that carries it
(`ogar_vocab::effect_class`, `ogar_action_handler::dt64::Registered::effect`).
Shared by OGAR, odoo-rs, zammad-rs and HIRO. Nothing here mints a classid, a
polyfill word or a lens.

DT64 (lance-graph `lance-graph-dt64`) binds, per classid, ordered
`MethodBinding`s: identity, ON (the fields whose change triggers it), WHEN (a
`Guard` over the population), reads, and DO (an ordered list of `Effect`s, each
a polyfill word over fields). The Frozen polyfill lens is `0xFFFF::0xF000`
(`POLYFILL_LENS`, 17 words: `IntAdd` ... `KEY_RUNS`, `lance-graph-r2il/src/polyfill.rs`).
Every word is a fold over a selection; none writes a record, orders a lifecycle
step, or commits a transaction. The design below keeps that true.

## 1. The decision

| Behaviour | Form |
|---|---|
| Queries and derived values (Rails scopes, Odoo `_compute_`, HIRO read-only conditions) | DT64 `MethodBinding` with F000 effects (lens `0xFFFF::0xF000`); the result is the target of the last effect |
| Writes, lifecycle steps, transactions, external calls (all 33 registered OGAR capabilities, Odoo button actions, Rails callbacks, HIRO `ActionHandler` calls) | NEVER F000. Classified by one shared `EffectClass` and handed to the gate (`commit_via`) as an `ActionInvocation` |
| Field mapping between apps | lance-graph #2116 VRL recipe aliases, bound through `LaneBinding::borrow_all` (#1988). Read-only; refuses computation |

The rule, from zammad-rs (`crates/rails-f000/src/effects.rs:3-9`):

> An F000 word is a fold over a population. It never authorizes a write, an
> ordered lifecycle step, or a commit-stage effect.

## 2. The zammad-rs precedent

All paths are in zammad-rs, `crates/rails-f000/src/`.

- `effects.rs:16-26`: `EffectClass { FrameworkGenerated, PurePredicateCondition,
  FieldTransform, Validation, LifecycleWrite, TransactionEffect, ExternalHost,
  Unresolved }`. The same eight are in `ogar_vocab::effect_class::EffectClass`.
- `effects.rs:8-9`: "There is deliberately no function here (or anywhere in
  this crate) that turns a callback into a `MethodBinding`." Callbacks produce
  a refusal or a deferral, never an effect.
- `effects.rs:61-88` (`classify_callback`): every class is answered with a
  `Verdict::Refuse(EffectBoundary)` or a `Verdict::Defer`, with these reasons:
  `TransactionEffect` "runs after the transaction, ordered by it";
  `FrameworkGenerated` "ordered by the save pipeline, per record"; `Validation`
  "adds errors to one record in chain order"; `FieldTransform` "writes a field
  of the record being saved; a fold never writes"; `LifecycleWrite` "writes
  during an ordered lifecycle step; a fold never writes"; `ExternalHost` "body
  is host-bound, outside what the lens can address";
  `PurePredicateCondition` deferred as "per-record scalar, not a population
  fold"; `Unresolved` deferred.
- `effects.rs:90-114` (`classify_class`): the phase and the kind of body
  decide the class. `after_commit` / `after_rollback` are `TransactionEffect`
  before anything else; a lowered body that writes is `FieldTransform` in a
  `before_*` / `after_initialize` phase and `LifecycleWrite` otherwise; a lowered
  body that writes nothing is `Validation` in a validate phase and otherwise
  `Unresolved`.
- `effects.rs:116-125` (`classify_condition`): a `builtin:` condition is
  `PurePredicateCondition`; anything else `Unresolved`; both deferred.
- `transpose.rs:70-98`: `Verdict` is `Alias | Compose | Learned | Residual |
  Refuse | Defer`; only `Alias` and `Compose` admit (`admits()`, 84-86), and
  `Transposed::binding` returns `None` for any other (126-143). Scopes are the
  F000 path: `where` / `where.not` over required boolean or integer columns of
  one class, ended by `count` or `exists?`, become `IntEqual` / `IntNot` /
  `IntAnd` words ending in `PopCount` / `ANY` (`transpose.rs:779-797`), with
  the result field the target of the last effect.
- `exec.rs:90-109` (`Engine::register`): registers methods through the DT64
  `BindStore`; the result field of a method is the target of its last effect
  (`exec.rs:97-100`).

OGAR's `EffectClass` therefore adds nothing to the semantics. It states them in
the vocabulary crate (no lance dependency) so that an `ActionDef` can be
classified without zammad-rs, and so that odoo-rs and HIRO use the same answer.

## 3. How each project maps onto the design

### OGAR

- The 33 registered capabilities (geo 6, healthcare 20, erp 7; #385
  `register_domain_table`) are name-only `kausal: External` defs with no
  `reads`, `writes`, `calls`, `raises` or `on_enter`. Each registers with empty
  `effects` and a `Registered { id, class, capability, identity, effect }` row.
  `capability` is the name a `commit_via` bridge dispatches on to a
  `CapabilityExecutor`.
- `effect` is `ogar_vocab::effect_class::classify(def)`. Distribution over
  OGAR's 50 `ActionDef`s: the 33 that register are all `ExternalHost`; the 17
  refused by the converter (ocr 14, document 3) are `LifecycleWrite`, because
  they name `writes` (for those two tables `writes` lists the capability's
  declared output names, so the class overstates a write; it still requires the
  gate). Nothing is F000-admissible and nothing is `Unresolved`.
- Several of the 33 are reads in fact (`get_*`, `list_*`, `locate_*`,
  `polyline_length`). The def does not say so, so none is admitted as an F000
  query: classifying from evidence means a read is not promoted on its name.
- Invariant (checked in `tests/dt64_methods.rs`): a method whose class is not
  F000-admissible has empty `effects`.

### zammad-rs

The reference implementation. Scopes become F000 `MethodBinding`s
(`Transposed::binding`); callbacks are classified (`classify_callback`) and
refused or deferred; the generated Zammad slice runs callbacks on
`rails-polyfill`, not DT64 (`zammad_map`: no DT64 type in `zammad-generated`).
The migration is to import the shared `EffectClass` instead of the local enum
and keep `Verdict` where it is.

### odoo-rs

- Derived values: `_compute_*` methods (`KausalSpec::Depends`) are the F000
  candidates (ON = the `depends` paths; the fold, if it is one, is the
  derived value). Whether a given `_compute_` is a population fold needs its
  lowered body, which odoo-rs does not have (see blockers). Until then a
  `Depends` def with no `writes` is `Unresolved`, and one with `writes` is
  `FieldTransform`: both require the gate.
- Constraints: `@api.constrains` (`KausalSpec::Constrains`) is `Validation`.
- Button actions (`action_*`, `button_*`) and server actions are
  `LifecycleWrite` / `TransactionEffect`, through the gate.
- Field mapping to other apps (Odoo field to the canonical field) is the VRL
  recipe path, not DO.

### HIRO

- `ActionHandler` calls (HIRO capabilities executed through
  `CapabilityExecutor`, the OGAR-native runtime in `ogar-action-handler`) are
  external calls: gate, never F000. The `ActionDef` lift is documented in
  `docs/HIRO-DO-ARM-LIFT.md`.
- Read-only conditions (a HIRO `ModelFilter`, lifted as a `StateGuard`) are
  WHEN, a `Guard` over the population. `dt64::when_and_on` ports a
  `StateGuard` over decimal-literal values as a Kleene `SetExpr`; see the
  validity-plane blocker for what that does and does not give.

## 4. Boundaries

- Field mapping (lance-graph #2116, VRL recipe aliases through
  `LaneBinding::borrow_all`, #1988) is ON + WHEN: it names which field of one
  app is which field of another, reads lanes, and refuses computation.
  Read-only.
- DO adds exactly two things beyond a mapping: computed assignments (an
  ordered list of F000 `Effect`s, each a fold word writing a result lane of the
  method, validated by DT64's dataflow check: a step reads only what the method
  declares or an earlier step wrote, `Refusal::UndeclaredRead`) and calls. Calls
  are never in the binding; they leave only through the gate as an
  `ActionInvocation` (`ActionDef` -> `ActionInvocation`, `ActionState` Pending
  -> Committed / Failed).
- "Computed assignment" means a derived value in a lane of the method's own
  result. It is not a write to the record: persisting the value is a
  `commit_via` step.
- `requires_gate` fails closed: `Unresolved` requires the gate, exactly as a
  write does. Only `PurePredicateCondition` may carry F000, and even it is only
  a necessary condition (zammad-rs defers it as a per-record scalar; admission
  needs a `Verdict::Alias` / `Compose` of the lowered body).
- `FieldTransform` is not admissible. In zammad-rs it is a refusal
  (`effects.rs:78`). A pure `_compute_` fold is a derived value on its own
  lowering path; it is not admitted by classifying its assigning method as a
  field transform. This is a conscious reading of zammad-rs; it is the open
  point to confirm for odoo-rs (section 5).

### The classification rule (`ogar_vocab::effect_class::classify`)

Only what an `ActionDef` carries is read. First match wins:

1. `on_enter` set, or `default_temporal == OnCommit`, or a `LifecycleTrigger`
   of `after_commit` / `after_rollback`: `TransactionEffect`.
2. `kausal` is `Constrains`: `Validation`.
3. `writes` or `calls` non-empty (`calls` is, by its doc, "lifecycle-mutator
   dispatch"): `FieldTransform` when the trigger is record-local (`Depends`,
   `Onchange`, `before_*`, `after_initialize`), else `LifecycleWrite`.
4. A `LifecycleTrigger` of `validate` / `*validation`: `Validation`.
5. `kausal` is `External` with nothing else: `ExternalHost`.
6. Otherwise `Unresolved`.

Never returned: `PurePredicateCondition` (it classifies an `if:` / `unless:`
condition; no `ActionDef` field carries one) and `FrameworkGenerated` (no
marker that the framework generated the callback). `raises` alone and
`ModalSpec::Atomic` are not evidence of a class.

## 5. Blockers

1. Three-valued WHEN: blocked on lance-graph's validity-plane decision
   (P-EXO-10, "decision B"). Lanes are dense `i32` with no validity bits, so
   SQL `=` / `!=` on a NULL-able column cannot be stated (zammad-rs refuses it
   as `NullableComparison`, `transpose.rs:624-626`; `prepare` refuses a
   three-valued binding, `polyfill.rs` module doc). OGAR's `method_of` sets
   `truth: ThreeValued` exactly when a guard is Kleene; no Kleene guard can
   execute until that decision lands.
2. Call-parameter channel: the 17 `ActionDef`s refused by the converter
   (ocr, document) are refused as `UnknownField(Reads, ...)` because a call
   parameter such as `grey_line` is not a field of the class. DT64 has no
   channel for a call parameter, and `ActionDef` has no parameter list
   (`ActionParam` exists only for schema-lifted capabilities). These stay
   refused, by name, until a channel exists.
3. odoo-rs findings (read-only census of odoo-rs at its HEAD `0e4456c`):
   - No `writes` anywhere: no production code assigns `reads`, `writes`,
     `calls`, `body_source` or `on_enter` on an `ActionDef`
     (`ogar_actions.rs:77-80` is the only construction); the measured
     body-level writes are none, so every Odoo def classifies as
     `Unresolved` or by trigger alone.
   - No production DT64 registration: `BindStore`, `MethodBinding`,
     `AlgebraRef`, `declare_algebra` do not occur; the ON x WHEN x DO cube in
     `tests/common/cube.rs` is a lab-local `Vec<Row>` log.
   - `base.automation` is not harvested: no `base.automation` record, no
     `ir.cron`, and the one `ir.actions.server` record
     (`data/nav/account_account_views.xml:193-204`) is read by no code.
     Form-view button names (`button_draft`, `button_cancel`, ...) occur in
     the view XML and in neither corpus, and no code links a button to a
     method.
4. The 33 registered capabilities declare no outputs and their executors
   (`osm-soa-bake`, `medcare-rs`, `erp-ledger`) are not in OGAR or
   rs-graph-llm: the hand-off row is a dispatch key, not a tested path.

## 6. Allocation rule

No allocation on the accepting path. Where a hot path must hold a bounded
number of items it holds them in a fixed array, as
`quack-boundary-lab::admitted` does (#2045). This crate's part is already
that: `classify` is a pure function of `&ActionDef` returning a `Copy` enum,
and `EffectClass::ALL` is a fixed `[EffectClass; 8]`. Registration
(`register_domain_table`) allocates, and is cold by design; the DT64 hot
paths (`prepare_native` / `run_native`) allocate nothing after `prepare`
(zammad-rs `exec.rs:187`, "Allocates nothing; reads only the lanes").

## 7. Execution shape for VRL and DO folds (polyfill only)

Every WHEN recipe and every F000 DO fold runs in one shape, through one path.
What #2116's probe already does is checked against `crates/vrl-recipe-probe/src/lib.rs`
on the lance-graph branch `vrl-recipe`.

| Property | Rule | Status in #2116 |
|---|---|---|
| Zero copy | Lanes are borrowed through one `LaneBinding::borrow_all` check, never copied; a row value is read in place | Done (`lib.rs:294-325`, `Rows::get` `:505`) |
| SIMD stride | All computation is mask-risc on `ndarray::simd`, tile by tile (`TILE_WORDS = 256`, measured; strided kernels exist) | Done, inherited from mask-risc (`exec.rs:101-116`, `:524-543`) |
| Late materialisation | Folds (`Count`, `Sum`) build no row; rows are a positional sink (kept mask, then a `u32` position table) built only on demand | Done (`lib.rs:404-432`) |
| No allocation on the accepting path | Section 6 | One per-terminal lane `Vec` left (`lib.rs:302`); the probe's author has offered a fixed array |
| Polyfill only | A recipe lowers to frozen `F000` words (`IntEqual`, `AND`/`IntAnd` + `NOT`, `COUNT`/`PopCount`, `SUM`) and runs through `lance-graph-r2il` `prepare` / `run` (`prepare_gated` for WHEN). No `dt64::vrl` module, no second kernel; materialising rows is a sink, not a word | Open: the probe builds a mask-risc `Program` directly (`lib.rs:436-473`). Asked on EXO-290 |
| Work stealing | If added, it belongs to the one shared path at the tile level (tiles are independent: `Count` / `Sum` add up, `Keep` writes separate mask words), never to VRL. Threshold measured first; bit-for-bit parity with the serial run; a fixed scratch per worker | Open: `execute_into` is serial. Asked on EXO-290 |

VRL stays read-only for good. DO adds computed values only as F000 folds and
calls only through the gate; no DO effect type is added to VRL.

## 8. Board

- OGAR design issue: EXO-289 (P-EXO-13), related to EXO-245.
- lance-graph asks (P-EXO-10): EXO-290 (VRL through the polyfill, parallel
  tiles), EXO-291 (is `commit_via` + `ActionInvocation` the only carrier for
  calls; call parameters; effect class on the binding), EXO-292 (R7: Quack
  `Draft` ranges and grouped folds).
- Decision B (validity plane) is posted by the #2116 session.
