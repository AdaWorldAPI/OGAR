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

### W1 — the POC cast (`blockly-rs`) — **NEXT**

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

**Also in W1:** app-prefix mint for `blockly-rs` in `ogar-vocab::ports` (small,
same review path as #234).

### W2 — Klickwege wiring into a2ui-rs

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

**W2 gates:**
1. One block placed in a2ui-rs produces exactly one `KlickwegEdge` that
   `lower_action_fire` turns into a valid `ActionInvocation` — no new predicate.
2. `resolve_nested` walks `canvas → script → block` on real rows.
3. A `Skin::Grid` renders placed tiles from `position` addresses, and hit-test →
   ordinal → `ActionInvoke` fires by address (never an inline handler).

### W3 — the second skin (PowerAutomate-shaped)

Operator-set: the same ABI, a low-code editor surface, **also** Mario-editor
ergonomics over `ClassView : WideFieldMask` projections. Two skins, one ABI —
which is the T1 discipline (*a new widget is a template, never a variant*)
applied at editor scale.

Gate: the identical rows render under both skins with no ABI change and no
second vocabulary.

### W4 — execution (`scratch-rs`, GPL leaf)

`.sb3` import via `rash_loader_sb3`; SoA calls → `rash_vm::ScratchBlock` →
Cranelift. Audit finding: **nothing in rash's IR forbids this** — `Ptr` is a
`usize` index into a `MEMORY` slab, not a raw pointer. The blockers were ours
(operands, nesting) and W0 resolved both.

Reciprocity: contribute the rash TODO gaps we need anyway (`Wait`, `Wait Until`,
`Stop all`, `Ceiling`, `ASin/ACos/ATan`, `Ln/Log`, Lists, Broadcasts) upstream,
where GPL is already the license.

### W5 — grammar projection

`A = B + C` rendered from and parsed back into the call stream. Storage
unchanged (the W0 ruling). This is where lance-graph's grammar machinery
becomes relevant, not before.

---

## Open decisions

| # | question | recommendation | blocks |
|---|---|---|---|
| **D1** | POC drives JS Blockly via a thin WASM shim in `blockly-rs`, or over the a2ui-server wire? | **shim** — reaches the W1 falsifier fastest; the wire is W2's job | W1 |
| **D2** | Does "place tile at slot N" travel as `ActionInvoke{ordinal: PLACE, args: [N, fn]}`, or does it need a third `FrameKind`? | **`ActionInvoke` args** — `args` is explicitly ClassView/ActionDef-carved, so this is an address-carried write, which is what T2 mandates. A third kind widens a deliberately closed vocabulary | W2 |
| **D3** | Palette drift detection against upstream Apache-2.0 sources | **hand-curation + drift test** (~half a day). ruff has NO JS/TS parser; a `ruff_ts_spo` is multi-week for a 228-definition harvest. `ruff_spo_address`'s mint half is confirmed language-agnostic if that changes | any |
| **D4** | Constant pool home for wide literals (`WAIT:1.5`, strings) | Inventory SoA is the natural host — it is already the label codebook | W1 (only when a literal exceeds a byte) |
| **D5** | `scratch-rs` licensing: whole-repo GPL-3.0, or GPL leaf crate + permissive siblings? | **leaf crate** — maximum optionality at zero cost | W4 |

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
