# Plan — the ABI-shaped block editor (v1)

> The sequencing plan for the block-programming arc: an ABI-shaped
> Blockly/Scratch editor over the V3 substrate, wired into a2ui-rs **through
> the Klickwege structure**, with a PowerAutomate-shaped low-code editor as the
> second skin over the same ABI.
>
> **Vocabulary + substrate decisions are LOCKED** (merged: #234, #235, #236).
> Ledger: `docs/DISCOVERY-MAP.md` `D-BLOCKS-DOMAIN` + `D-BLOCKS-PALETTE`.
> Sibling charter for the render target: `docs/A2UI-SCREEN-ADDRESSING-PROPOSAL.md`
> (#204/#205) and its consumer plan `a2ui-rs/.claude/plans/a2ui-screen-addressing-v1.md`.

## The thesis (one line)

**Don't store the block tree — address it.** A block program is V3 SoA rows;
the blocks a user drags are a **ClassView projection** of those rows, and so is
the PowerAutomate surface, and so is the text form. One ABI, many skins,
Mario-editor ergonomics over `ClassView : WideFieldMask`.

## What is LOCKED (do not re-derive)

Three merged PRs. Every line below is shipped code with tests, not intention.

| | ruling | where |
|---|---|---|
| **Domain** | `0x17XX` = `ConceptDomain::Blocks`, reserved-empty; the `0x10`–`0x16` gap is deliberate and test-pinned | `ogar-vocab`, #234 |
| **Producer posture** | concept ids authoritative in `ogar-blockly`, zero `0x17XX` rows in the shared CODEBOOK (`ogar-obo` pattern) | #235 |
| **Provenance fence** | entries derive ONLY from Apache-2.0 Blockly + `scratch-blocks`; never AGPL `scratch-vm` | #235 |
| **One content classid** | `BlockConcept::{Content, Inventory}` — two variants total; operations are payload, not classid space | #235 |
| **Everything is a call** | `Call = (function : value)`. No opcode/function distinction: `ADD` is function `0x40`, a user block is another index in the same `<256` codebook | #236 |
| **Arity by classid** | `LaneShape::{Pairs, Triples, Quads}` = `6×(u8:u8)` / `4×(u8:u8:u8)` / `3×(u8:u8:u8:u8)` → 180 / 120 / 90 calls, always 360 bytes | #236 |
| **Nesting by reference** | a function index names another function's node — SB3's own model. No `END`, no jump offset | #236 |
| **Narrowing is loud** | `BodyError::ValueBeyondShape` refuses a call the shape would truncate | #236 |
| **Literal, not grammar** | `A = B + C` is a *projection* that renders from and parses back into the call stream — never the storage format | #236 |
| **Edge block retired** | slot 1 reserved-zeroed; relations ride the payload rails as indexed calls | #236 |

**Measured, re-runnable** (`cargo run -p ogar-blockly --example density`):
Blockly 57 block types / 71 codes, `scratch-blocks` 171 opcodes (59 shared-core
+ 108 device + 4 helpers), deduplicated union **≈190 ≤ 256**. Whole-node
amortized cost 1.42 B/call at 180 calls.

## The node, as it now stands

```
one function  =  one node  =  512 bytes  =  32 × 16-byte slots
  slot 0        key      classid = Content · identity = which function
  slot 1        reserved zeroed (the edge-block design is RETIRED)
  slots 2..31   value    30 lanes × 12 B, carved by the body's LaneShape
                         → 180 / 120 / 90 calls, always 360 bytes
```

`<256` functions per scope; one byte names any of them, resolved through the
**Inventory SoA — the label codebook**. Scopes cascade rather than widen, the
same rule as the GUID canon.

## Where the seam falls (repo boundary)

| lives in | what | state |
|---|---|---|
| **OGAR** | the vocabulary (`ogar-blockly`), the ledger, this plan | **shipped** |
| **`blockly-rs`** | the ABI half: Blockly event ↔ facet cast, the POC | **empty repo** |
| **`scratch-rs`** | the Scratch half: `.sb3` import + the `rash` JIT binding (GPL) | **empty repo** |
| **`a2ui-rs`** | the render target: Klickwege wiring, palette + placement tier | shipped W2/W3/W5 + #209 |
| **`rash`** | Cranelift JIT, GPL-3.0 whole-workspace | upstream, unmodified |

**GPL containment:** `rash_vm` is the JIT crate and the whole rash workspace is
GPL-3.0 with no per-crate override. It links only from `scratch-rs`, behind a
feature. `ogar-blockly` is deliberately unencumbered (the provenance fence) so
a GPL consumer links it freely — the boundary sits in the consumer repo and
never propagates into OGAR, lance-graph, or ndarray.

---

## Waves

### W0 — vocabulary + substrate — **DONE**

#234, #235, #236 merged. Nothing below may re-open these rulings without a new
operator ruling and a ledger correction.

### W1 — the POC cast (`blockly-rs`) — **DONE (one mint outstanding)**

The operator's original brief, unchanged: *reuse the existing; materialize the
ABI-shaped SoA the way a JSON would, but without serialization, by casting into
the variable.*

```
Blockly workspace event   (JS, reused as-is — Apache-2.0, no fork)
  → block record {opcode, fields, inputs, next, parent}
  → CAST into the V3 facet:
        function index  ← opcode, via the ogar-blockly codebook
        value byte(s)   ← fields, per the class's LaneShape
        references      ← next / parent / inputs as indexed calls
  → cast back → block definition → render
```

No serialization anywhere: `from_le_bytes` / `to_le_bytes` **are** the format.
No bytecode, no ISA — the encoding question was settled in W0.

**Gate (the falsifier that decides the thesis):**
> **A drag produces ZERO SoA writes. An operand change produces EXACTLY ONE.**

If that fails, the projection thesis is wrong and everything above W1 is
decoration. It is cheap to run and must run first.

**Gate result: GREEN**, `blockly-rs` `4d39590`. Both halves route through
`Workspace::apply`, so a handler that let a drag reach the record fails
the test — verified by injecting exactly that leak and confirming two
tests fail. Without the apply step "a drag changes nothing" would have
been `f(x) == f(x)`, which is how the first version of this falsifier
was written and why it was rewritten.

**Also landed in W1** (`31e18fb`, `e4b5e7e`): the ValueParam byte
encoding, both D3 drift anchors, the D4 constant pool (opt-in, pending
mint), and the Klickwege address producer — see § Open decisions for
each, including D4's reversal.

**⚠ NOT done in W1: the app-prefix mint.** It is moved to § Remaining —
operator mints, below. Codex flagged the contradiction on #238 (a wave marked
DONE while its own section named unfinished work), and it was right to: the
risk it named — someone reading "done" and hardcoding an unallocated prefix —
was **already instantiated**. `blockly-abi`'s Klickwege tests used `0x1000`,
which `ogar-vocab::ports` RESERVES for the V3-adoption monitor marker, with a
test asserting it "must never be allocatable as a port's `APP_PREFIX`".

Fixed in `blockly-rs` `e53aefe`: the test constant is now an obviously-unreal
`0xFF00` so it cannot be mistaken for the answer. No library code was
affected — `app_prefix` is a **parameter** on every public function in
`klickweg`, and nothing in the crate names a prefix. That parameterisation is
what kept a documentation contradiction from becoming a stored collision.

### W2 — Klickwege wiring into a2ui-rs — **DONE**

**This is the wave the goal names, and the key finding is that most of it
already exists.**

Charter C1.6: *"a click IS a `navigates_to` / `ActionInvocation` edge"* — the
same closed vocabulary the C# harvest emits statically becomes a runtime event
stream, **with zero new vocabulary**. Shipped in `a2ui-server`:

```
ActionInvoke{key, ordinal, args}          ← up the wire, by ADDRESS (T2)
  → DesktopSession::receive_action
  → KlickwegEdge{from_key, class_id, ordinal, predicate, seq}
  → lowering::lower_action_fire  → ogar_vocab::ActionInvocation
     lowering::lower_screen_jump → NavWitness
```

Both lowerings are **pure compile-time value construction** (#209, warden
COMPILE-TIME-CLEAN). 34 tests.

**The consequence for blocks:** *editing a block program IS a Klickweg stream.*
Placing a block, connecting two blocks, clicking a placed block — each is a
click with an ordinal address, and each lowers to an `ActionInvocation` in the
same predicate vocabulary a harvested legacy app emits. Edit telemetry and app
telemetry unify; neither needs a new predicate.

The ruff-side closed vocabulary this rides on (do **not** extend without the
`Predicate` gate — 79 variants, count-locked by test):

- **UI-navigation:** `navigates_to`, `selects_view`, `invokes_action`, `renders_as`
- **Room map:** `surfaces_concept`, `handles_event`, `contains_control`
- **Klickwege rail:** `part_of`, `purpose`, `guarded_by_permission`

**The nesting maps directly.** `ObjectSlot` recursion is *"the A3 Klickwege
brick"*: `desktop → window → region → widget` becomes
**`canvas → script → block → input`**. `a2ui-wasm::resolve_nested` walks that
tree today, unchanged, via `child_links: (parent_key, slot) → child_key`.

**What is genuinely missing** (measured by audit, not assumed):

| need | state | charter risk |
|---|---|---|
| interaction → graph edge | **exists** (`receive_action` → `KlickwegEdge` → lowering) | none |
| nested addressing | **exists** (`resolve_nested`, L1/L2) | none |
| palette of pickable tiles | **absent** — nothing represents "available but unplaced" | none if built as a ClassView template (T1); a hardcoded list in `a2ui-paint` would violate T1 |
| 2-D placement | **absent** — `Skin::{Form, Flow}` are both 1-D list renderers | none; a new `Skin::Grid` is symmetric with how Form/Flow already map a 1-D address to 2-D pixels |
| multi-facet body ingest | **absent** — `a2ui-wasm` implements ONE facet (`FACET_LEN = 12`, the V3 payload); a body is 30 | none; unimplemented capacity, not a defect |
| drag/connect semantics | **absent** | **T2 pressure** — local drag state is fine, but the RESULT must travel as an address-carried write |

**Producer side landed** (`blockly-rs` `e4b5e7e`, `blockly_abi::klickweg`).
Three of `KlickwegEdge`'s five fields are a property of the program and are
produced there: `class_id` (canon-high under the app prefix), `ordinal`, and
`predicate`. `from_key` and `seq` belong to the session, not to the cast, so
they stay a2ui-side. Neither repo imports the other.

**The ordinal is the CALL INDEX** — where `lower_script` put that block's
`Call` — so `raise_calls(body)[ordinal]` IS the clicked block's call, by
construction. That makes an address checkable against the ABI rather than
merely self-consistent, which is what its falsifier does. A block id is editor
state and a position is presentation; neither survives the trip, and Blockly's
tree order is not the program's order. Verified non-vacuous by injection: a
pre-order walk still yields unique, dense, plausible ordinals — and fails two
tests, because they index the real lowered body.

T2 holds structurally rather than by assertion: nothing in that module returns
anything invocable, so an `onClick` lambda cannot be expressed through it even
by accident. The predicate carries a Selector code (`math_arithmetic[ADD]`) and
never a ValueParam code, so two clicks on one action cannot read as two
different actions.

**W2 gates:**
1. One block placed in a2ui-rs produces exactly one `KlickwegEdge` that
   `lower_action_fire` turns into a valid `ActionInvocation` — no new predicate.
   *(Producer half done; the a2ui-side consumption is what remains.)*
2. `resolve_nested` walks `canvas → script → block` on real rows.
3. A `Skin::Grid` renders placed tiles from `position` addresses, and hit-test →
   ordinal → `ActionInvoke` fires by address (never an inline handler).

**Consumer side landed** (a2ui-rs #18). All three named gaps closed, and the
audit above was **corrected on two points** by reading the source:

- Gap 2's "symmetric with Form/Flow" was generous. Form and Flow do not map
  the address at all — they place by ITERATION ORDER and copy `position`
  through only so hit-test can round-trip it. `Grid` is the FIRST skin where
  `position` is genuinely a coordinate, which makes it more novel than
  "symmetric" and is why its falsifier uses a position GAP (0, 1, 3): with
  contiguous positions the two placement rules coincide and the test would
  pass with `place_flow` substituted.
- Gap 3's framing was wrong. `FACET_LEN = 12` is not a facet COUNT — it is the
  byte width of one V3 facet. (The prior withdrawn "truncation defect" died on
  exactly this confusion; it must not be resurrected under a new name.) The
  capacity deferral is real and already documented upstream as fail-safe.

**But the audit MISSED a real defect, and it is the more important find.**
`apply_node_delta`'s loop was `if pos < FACET_LEN` with no `else`: an
out-of-range mask position consumed no value byte and raised no error, so a
field the sender declared vanished while `apply` returned `Ok`. The same loop
was already loud on the UNDER-supply side (`ValueUnderrun`) and silent on the
over-range side — and the wire is wide-native (`a2ui-core` round-trips
positions 40 and 64), so such frames are well-formed and only unrenderable at
the client. Now `PositionOutOfRange`.

**The >255 ceiling does not bind this arc.** `FieldView.position` is `u8`; a
lowered body is at most 180 calls (`Pairs`) or 90 (`Quads`). No upstream OGAR
widening is needed — recorded so it is not re-derived.

The palette is `(parent, slot) → Vec<ClassId>`, the structural twin of
`child_links`; the pair is the whole palette/canvas distinction with no third
concept (**available = offered but not linked; placed = linked**). Captions
resolve through `ClientClass.title`, so it stores addresses and reads names
rather than carrying names of its own — which is why it cannot mint a second
vocabulary. A pick is an ordinary `ActionInvoke` through the unmodified
up-wire path: no new `Frame` variant, no wire change.

### W3 — the second skin (PowerAutomate-shaped) — **DONE**

Operator-set: the same ABI, a low-code editor surface, **also** Mario-editor
ergonomics over `ClassView : WideFieldMask` projections. Two skins, one ABI —
which is the T1 discipline (*a new widget is a template, never a variant*)
applied at editor scale.

Gate: the identical rows render under both skins with no ABI change and no
second vocabulary.

**GREEN, and it needed no new type** (a2ui-rs #18). `Grid { cols: 1 }` IS the
PowerAutomate-shaped vertical step list; `Grid { cols: 4 }` is the block
canvas. Same variant, different column count — the strongest available form of
the T1 discipline, since a second surface did not even require a second skin.

The gate asserts one `NodeDelta` → one resolved surface → four skins with
identical row sets, field ADDRESSES, and action ordinals, and byte-identical
ABI afterwards (rendering is a read). Its anti-vacuity half is what makes it
worth having: the four skins must actually DIFFER in geometry, checked
pairwise — four skins producing identical rects would satisfy everything else
while "one surface, many skins" quietly collapsed to "one skin". Verified by
injection.

### W4 — execution (`scratch-rs`, GPL leaf) — **DONE**

`.sb3` import via `rash_loader_sb3`; SoA calls → `rash_vm::ScratchBlock` →
Cranelift. Audit finding: **nothing in rash's IR forbids this** — `Ptr` is a
`usize` index into a `MEMORY` slab, not a raw pointer. The blockers were ours
(operands, nesting) and W0 resolved both.

Reciprocity: contribute the rash TODO gaps we need anyway (`Wait`, `Wait Until`,
`Stop all`, `Ceiling`, `ASin/ACos/ATan`, `Ln/Log`, Lists, Broadcasts) upstream,
where GPL is already the license.

**SHIPPED** (`scratch-rs`, two crates). **D5's leaf-crate ruling holds and is
now mechanically checkable**: `cargo tree -p scratch-abi | grep -c rash` is 0.
The licence is not a choice — rash's workspace carries a GPLv3 repo-root
LICENSE and **no `license` field in any of its six `Cargo.toml` files**, so no
per-crate override exists to appeal to and every crate linking `rash_vm` is
GPL. Exactly one does.

- `scratch-abi` (MIT) — `nest` / `nest_all` / `flatten`, the exact inverse of
  the cast's post-order emission. Arity is read from blockly-abi's table,
  deliberately not a second copy: two tables that disagreed would produce two
  different trees from one program, silently.
- `scratch-jit` (GPL-3.0-only) — `to_scratch_block`. A **rename, not a
  translation**: rash's `Input` is `Obj(literal) | Block(nested)`, exactly the
  leaf/branch distinction a `CallTree` node makes, so no evaluation order is
  chosen and no operand reordered.

**The refusal is the load-bearing part.** Where the palette has no
`ScratchBlock` counterpart the mapping refuses rather than composing one from
several rash ops — `NEQ` as `not(cmp)` would be the adapter acquiring
semantics of its own, which is the parallel-object-model anti-pattern the
Core-First doctrine names. rash's operator set is Scratch's, so the gaps are
the Blockly operations Scratch lacks (`LN`/`LOG10`/`EXP`, inverse trig,
`NEQ`/`LTE`/`GTE`, `CEIL`, `POW`); `unmapped_functions()` enumerates them and
each is asserted to really refuse, so the list cannot rot into documentation
nothing checks.

The comparison fold carries both halves on purpose: `LT`/`GT`/`EQ` fold into
rash's single `OpCmp`-with-`Ordering`, AND `NEQ`/`LTE`/`GTE` stay refused —
without the second half the first could quietly become "every comparison maps".

**Gate: GREEN** (`scratch-rs` `tests/sb3_gate.rs`). A real `.sb3` — a genuine
zip carrying `project.json` AND a costume asset, since rash resolves
`currentCostume` and reads the asset's bytes — whose green-flag script computes
`(3 + 4) * 2`. The native arm is `ProjectLoader::new(sb3).build()`: rash unzips,
parses with its own serde types, lowers with its own `load_block`, compiles with
Cranelift, and nothing of ours runs. The SoA arm is the same program as a call
stream (`3, 4, ADD, 2, MUL`) through `to_scratch_block` into the same
`Script`/`SpriteBuilder`/`ProjectBuilder` entry points rash's own loader uses.
Both run; the values are compared.

**The arms meet at EXECUTION, not at the IR, and that is the stronger check.**
An IR diff is not available (`load_block` is public but takes a
`CompileContext` with private fields and no public constructor) — but it would
also be weaker: a structural diff is satisfied by two trees that are equal and
both wrong. A run proves the same VALUE comes out of rash's own compiler either
way.

Both injections were run, because "identical output" is the easiest assertion
in the world to satisfy with an inert harness: changing the fixture to
`(3 + 4) * 9` makes the native arm read **63**, proving rash really parses and
runs OUR fixture rather than returning a cached or default value; lowering
`MUL` to `OpAdd` makes it **native 14 vs SoA 9**, proving the SoA arm really
feeds the comparison.

### W5 — grammar projection — **DONE**

`A = B + C` rendered from and parsed back into the call stream. Storage
unchanged (the W0 ruling). This is where lance-graph's grammar machinery
becomes relevant, not before.

**SHIPPED** (`blockly-abi::projection`). The load-bearing claim is proven
rather than argued: `5 + 3` typed as text and `5 + 3` built as blocks produce
**byte-identical** bodies, with a different program asserted not to match so
the equality is not free.

It needs arity for the same reason `scratch-abi` does, and refuses for the
same reason. Statement-level functions (`IF`, `REPEAT`, `PROC_DEF`) are
deliberately outside the table — they nest by reference, which is the
*statement* projection, a separate surface.

**Both halves of the gate, and the asymmetry that makes the second real:**
the call stream round-trips byte for byte in the direction that matters
(`body → text → body`; parsing is many-to-one, so `text → body → text` would
be the weaker claim). Whitespace, redundant parens, and every trace of
geometry explicitly do NOT survive — five spacings of `1 + 2 * 3` must yield
identical bytes, and the render is canonical rather than reproducing what was
typed. If spacing ever survived, the ABI would be carrying a layout decision.
Two-sided: parens that are NOT redundant must survive, or "spacing does not
matter" would be indistinguishable from "parens are ignored".

Precedence and associativity are pinned by stack ORDER, not by round-trip — a
parser that ignored precedence would still round-trip its own output.

---

## Open decisions

| # | question | recommendation | blocks |
|---|---|---|---|
| **D1** | POC drives JS Blockly via a thin WASM shim in `blockly-rs`, or over the a2ui-server wire? | **shim** — reaches the W1 falsifier fastest; the wire is W2's job | W1 |
| **D2** | Does "place tile at slot N" travel as `ActionInvoke{ordinal: PLACE, args: [N, fn]}`, or does it need a third `FrameKind`? | **`ActionInvoke` args** — `args` is explicitly ClassView/ActionDef-carved, so this is an address-carried write, which is what T2 mandates. A third kind widens a deliberately closed vocabulary | W2 |
| **D3** | Palette drift detection against upstream Apache-2.0 sources | **hand-curation + drift test** (~half a day). ruff has NO JS/TS parser; a `ruff_ts_spo` is multi-week for a 228-definition harvest. `ruff_spo_address`'s mint half is confirmed language-agnostic if that changes | any |
| **D4** | Constant pool home for wide literals (`WAIT:1.5`, strings) | ~~Inventory SoA is the natural host — it is already the label codebook~~ **REVERSED, see below** | W1 (only when a literal exceeds a byte) |
| **D5** | `scratch-rs` licensing: whole-repo GPL-3.0, or GPL leaf crate + permissive siblings? | **leaf crate** — maximum optionality at zero cost | W4 |

**D1 · D2 · D5 — all RESOLVED as recommended**, no reversals. D1: the W1 cast
is a library the shim drives, and the falsifier was reached without the wire.
D2: a pick travels as an ordinary `ActionInvoke` at an ordinal — no third
`FrameKind` was needed, and the palette added zero `Frame` variants. D5: the
leaf crate is shipped and its containment is `cargo tree`-checkable.

### D3 — RESOLVED (`blockly-rs` `e4b5e7e`)

Two drift anchors shipped in `blockly-abi`, because the obvious one was
not sufficient:

- `the_palette_byte_values_are_pinned` pins the **bytes**. The
  pre-existing census compared against `FnIndex::LT` — the *symbol* — so
  an upstream renumber would have left every symbolic assertion passing
  while every stored program changed meaning. The pins are also asserted
  mutually distinct, so a collapsed palette cannot be matched by a
  collapsed expectation table.
- `the_value_param_option_sets_are_pinned` pins the 17 argument-dropdown
  option sets in source order, harvested from the Apache-2.0
  definitions.

### D4 — RESOLVED, and the recommendation above is REVERSED

The Inventory recommendation is **withdrawn**. It conflates two
codebooks: Inventory indexes *functions*, which are shared by
definition; constants are *per-function data*, which are owned by
definition. Putting a per-function pool in the one table every function
shares makes a shared-mutable sink with N writers.

**Shipped instead: a sibling pool node** — same 30 content slots, same
16-byte stride, identity inherited from the owning function, and a
**per-facet classid naming the constant's type**, because an `f64` and a
UTF-8 string are different readings of 12 bytes and "your classid
defines the schema, period" forbids a discriminant byte inside the
payload. Index arithmetic `(idx-1)/30` and `(idx-1)%30`; `0` reserved as
the zero-fallback so a zeroed value byte reads as *no constant* rather
than *constant zero*.

Two further options were considered and killed by specific constraints,
recorded so they are not re-proposed:

- **Literal as a run of calls** (no pool) — killed by the W1 falsifier:
  the call count would track the literal's width, so editing `255` to
  `1000000` would shift every later call and rewrite the tail of the
  body. An operand edit must produce ONE write.
- **Steal content slots from the body node** — legal under the substrate
  (per-facet classids make a mixed node schema-honest), but it makes the
  call budget per-function, so "add one string" can make a program that
  fit stop fitting, with the overflow blaming the calls.

Capacity, explicit: 255 usable indices, 30 per node, 9 nodes. The 256th
returns `PoolFull` and the remedy is a **function split**, never a wider
index. `PoolFull` is reachable only under `Quads` (270 addressable value
bytes > 255); `Pairs` (180) and `Triples` (240) cannot exhaust it.

**Still gated on an operator mint.** The pool is opt-in:
`lower_script` still refuses a wide literal, and only
`lower_script_with_pool` interns — under caller-supplied classids, with
deliberately invalid placeholders in the interim, so a placeholder
cannot reach stored data before the concepts exist. Proposed mints
(`ConstantPool` / `ConstF64` / `ConstUtf8Inline` at `0x1703..0x1705`)
are a **proposal**, not an assumption. The explicitly rejected cheap
alternative — one concept with a type-tag byte in the payload — is
named here so it is refused on the record rather than rediscovered.

### ValueParam encoding — RESOLVED (`blockly-rs` `31e18fb`)

A `ValueParam` dropdown code encodes as its **ordinal in the codebook's
own pinned table**, keyed on `(block type, field name)`. Deliberately
not Blockly's live array order: reordering an options array is cosmetic
upstream and would silently reinterpret every stored program here, so
anchoring the ordinal in the codebook converts that hazard into a loud
drift-test failure. Keyed on the field and not the type alone because
`text_getSubstring`'s `WHERE1`/`WHERE2` differ only in their third
entry.

Widths measured against the Apache-2.0 definitions: largest set is 8
(`math_on_list`), so the byte is not a squeeze. `math_on_list[RANDOM]`
is absent from its table by construction — it is one of the three gaps.

## Remaining — operator mints (the only open work)

Every wave's code is shipped and every gate is green. What is left is three
decisions that are the operator's by standing rule (*"codebook ids are
permanent; a mint is an operator decision with a ledger entry, never a
drive-by"*), so none has been taken here.

| # | mint | blocks | notes |
|---|---|---|---|
| **M1** | `blockly-rs` app prefix in `ogar-vocab::ports` | nothing today; W2 address work if a second consumer appears | Allocated so far: `0x0000` core, `0x0001`–`0x0005`, `0x0007`. `0x0006` and `0x0008+` look free. **`0x1000` is RESERVED** (V3-adoption monitor) and must never be chosen. No value is proposed here. |
| **M2** | the three codebook gaps — `math_on_list[RANDOM]`, `lists_reverse`, `lists_getIndex[GET_REMOVE]` | W4 coverage of those blocks | Each resolves to `None` today and the cast refuses. `lists_getIndex[GET_REMOVE]` is compound (read AND delete), so it is a design question, not just an id. |
| **M3** | the constant-pool facet concepts | wide literals in stored data | Proposed `0x1703` `ConstantPool` / `0x1704` `ConstF64` / `0x1705` `ConstUtf8Inline`. The pool is opt-in in code until this lands, with deliberately invalid placeholder classids, so no placeholder can reach stored data. |

None of the three blocks any shipped falsifier: M1 because the prefix is a
parameter, M2 because the gaps refuse rather than guess, M3 because the pool is
opt-in.

## Falsifiers (each must be able to FAIL)

1. **W1** — drag = 0 writes, operand change = 1 write.
2. **W2** — one placement = one `KlickwegEdge` = one valid `ActionInvocation`,
   zero new predicates.
3. **W3** — one row set, two skins, no ABI delta.
4. **W4** — a `.sb3` project produces identical output through rash-native
   loading and through the SoA path.
5. **W5** — text → ABI → blocks preserves addresses and operands; positions and
   whitespace explicitly do **not** round-trip (a test asserting they don't,
   or geometry has leaked into the ABI).

## Standing rules for this arc

- **Reuse before transcode.** JavaScript keeps dragging puzzle pieces; Rust owns
  semantics, versioning, execution. Do not port a block renderer.
- **Codebook ids are permanent.** A mint is an operator decision with a ledger
  entry, never a drive-by.
- **The ledger records retirements in place.** Append-only; a correction names
  what it retires (see `D-BLOCKS-PALETTE` corrections 1 and 2).
- **The charter traps hold** (a2ui-rs T1/T2/T3): no second vocabulary, behavior
  by address only, no serialization in the hot path.
