# Chess Transcoding — OGAR as the substrate for a closed, formalized domain

> **Purpose.** Expand OGAR against chess via the
> [shakmaty](https://github.com/AdaWorldAPI/shakmaty) crate, as the
> **calibration target** for the substrate. There is no cleaner separation of
> Semantik / Syntax / Pragmatik to calibrate against — chess gives each layer
> in formally closed form (§0), and the contract's lifecycle primitives
> (`Pending → Committed/Failed/Cancelled`, `Postpone`, `StateTimeout`,
> `on_commit`) all light up under it. Plus shakmaty's
> `play(Move) -> Result<_, PlayError>` is a free **§14 oracle** — same
> `(FEN_before, UCI_move) → FEN_after` on shakmaty and on the OGAR-generated
> actor.
>
> **Position vs Odoo.** Same kind of work as `ODOO-TRANSCODING.md` (a domain →
> OGAR mapping), narrower in scope: chess is closed and formally specified,
> so the mapping is provably complete *and* the §14 oracle is exact. That's
> why it's the calibration domain — not because it's frivolous, but because
> it's the cleanest possible test of universality.
>
> **Grounded in:** `OGAR-AST-CONTRACT.md` (the lowering: `State=ActionState`,
> `Event=ActionDef`, `Context=ActionInvocation`), `ARCHITECTURE.md` + `ADAPTERS-AND-ACTORS.md`
> (the Semantik/Syntax/Pragmatik trichotomy, SPO + TeKaMoLo), `IDENTITY-MAPPING.md`
> (prefix), `ELIXIR-HIRO-PREFETCH.md` (the prefetch-the-types-now pattern,
> applied here too).
>
> Status: **CARVED v0** (2026-06-04).

## 0. The cleanest calibration target — trichotomy separated by construction

OGAR is grounded in the Semantik/Syntax/Pragmatik trichotomy
(`ARCHITECTURE.md`; `ADAPTERS-AND-ACTORS.md §3.2`). In open domains (ERP,
HIRO/Bardioc, Wikidata) the three layers are real but **entangled** —
Odoo's `def action_confirm(self):` mixes vocabulary (`SaleOrder`), syntax
(Python decorators, method-name conventions) and pragmatics (transaction
boundary, who-can-call, when-it-runs) in one source line, and a producer has
to *disentangle* them on every parse. Chess is the rare domain where the
trichotomy is **separated by construction** — the field has spent 500 years
giving each layer its own formal notation:

| OGAR layer | Sign relation | Chess (formally specified) | Carrier |
|---|---|---|---|
| **Semantik** | sign ↔ object | `{Role × Color}` (12 pieces) over `Square` (64), `Position` (Board + side-to-move + castling rights + ep square + clocks), `Outcome`, `Game` | `Class` / `Attribute` / `EnumDecl` (§2) |
| **Syntax** | sign ↔ sign | **FEN** (position notation), **SAN** (move notation: `Nf3`), **UCI** (`e2e4`), **PGN** (game notation) — each a published grammar with proven parsers in `shakmaty::{fen, san, uci}` | the `Adapter` HHTL — bidirectional `map` / `unmap` per notation (§7) |
| **Pragmatik** | sign ↔ interpreter | **whose-turn** (`Color`), **clock** (per-side ms remaining), **legality** (`pos.legal_moves()`), **modality** (Atomic: a move is all-or-nothing; Sync: turn-based), **lokal** (which player, which game instance) | `ActionInvocation` SPO + TeKaMoLo + lifecycle (§3, §4) |

Three properties make this the calibration target:

1. **Layers are formally specified and finite.** A producer cannot
   accidentally fold pragmatics into syntax (the way Odoo decorators sometimes
   do) — the parsers themselves enforce the boundary.
2. **Each layer has an *independent* oracle.** Semantik: an exhaustive enum
   of pieces/squares/outcomes — any deviation is detectable. Syntax: FEN/SAN/
   UCI/PGN are bijective with the position/move space — `parse(emit(x)) == x`
   is provable, not asserted. Pragmatik: shakmaty's `play()` is the §14
   oracle (§6).
3. **The OGAR contract's primitives map 1:1 to the trichotomy.** `Class` /
   `EnumDecl` carry semantik; `Adapter` carries syntax; `ActionInvocation` +
   TeKaMoLo + the `state_machine` lifecycle carry pragmatik. If chess lowers
   cleanly, the carrier mapping is sound — not just for chess, for any domain
   that can be projected onto the same three layers.

This is why chess is the calibration domain. Pass on chess and the substrate
is *demonstrably* universal — fail and the failure pinpoints exactly which of
the three carriers leaks.

## 1. The core carries chess via `ogit-chess::` — zero core changes

Chess attaches under a new `Identity` prefix; nothing in `ogar-vocab`,
`ractor_actors::state_machine`, or the binding contract changes. The recipe:

```
ogit-chess::Game            → Class  (the game-as-process; one actor per game)
ogit-chess::Position        → Class  (the piece-arrangement-at-a-ply; nested in Game)
ogit-chess::Piece           → Class  (Role × Color)
ogit-chess::Game::action::* → ActionDef × {move, castle, en_passant, promote, resign, offer_draw}
ogit-chess::Game::invocation::<ulid>
                            → ActionInvocation  (one per ply, plus offers/resigns)
```

**Adding chess = a producer (`ogar-from-shakmaty`) + a short TTL ontology.**
Not touching: the state-machine crate, the codegen, the membrane — that's the
universality guarantee from `OGAR-AST-CONTRACT.md §4`. The one *known
extension point* the producer exercises is the `Language` enum in `ogar-vocab`
(open by design — `Elixir` was added the same way via PR #10). The chess
producer ships with `Language::Unknown` on day one and earns a typed
`Language::Rust` variant via a one-line PR when ready — `Language` extensions
are the precedent path for any new source-AST tag and don't violate "core
unchanged" the way an IR-struct change would.

## 2. Structural arm — shakmaty → `Class`

The producer walks `shakmaty::Chess` + the `Position` trait + the `Move` enum
and emits the following classes:

| shakmaty type | OGAR mapping |
|---|---|
| `Chess` (impl `Position`) | `Class { identity: "ogit-chess::Game", language: Unknown /* or Language::Rust once added — see §1 */, attributes: [turn, halfmove_clock, fullmoves, castling_rights, ep_square, outcome], associations: [position: Position, white: Player, black: Player] }` |
| `Board` + `Setup` (piece arrangement) | `Class { identity: "ogit-chess::Position", attributes: [board_fen, side_to_move, …] }` (nested; one per ply) |
| `Role` (Pawn..King) | `EnumDecl { name: "Role", values: [Pawn, Knight, Bishop, Rook, Queen, King] }` |
| `Color` (White/Black) | `EnumDecl { name: "Color", values: [White, Black] }` |
| `Square` (A1..H8) | `EnumDecl { name: "Square", values: [A1, …, H8] }` (64 variants; or scalar) |
| `Outcome` (`Unknown`, `Decisive{winner}`, `Draw`) | `EnumDecl { name: "Outcome", values: [Ongoing, WhiteWins, BlackWins, Draw] }` |
| `CastlingSide` (Kingside/Queenside) | `EnumDecl` on the castle ActionDef |
| `PlayError` | not a `Class` — it's the `KausalSpec` rejection → `Pending → Failed` |

The `language` field uses `Language::Unknown` until a typed `Language::Rust`
variant is added to `ogar-vocab` (one-line PR, precedent: `Language::Elixir`
in PR #10). `Language` is the established extension point for new source-AST
tags; the chess producer doesn't require it to ship but earns the typed tag
when convenient.

## 3. Behavioral arm — shakmaty → `ActionDef`

One `ActionDef` per `Move` variant, plus the meta-actions (resign, draw offer,
clock-tick). All carry the four PR-#10 statem fields on `ActionDef`.

| Move / event | `ActionDef` projection |
|---|---|
| `Move::Normal { role, from, to, capture, promotion }` | `predicate="move"`, `kausal=StateGuard{field:"turn", values:[<color-to-move>]}`, `default_modal=Atomic`, `on_enter=Some("apply Move::Normal")`, `state_timeout_millis=Some(<clock-ms-for-side>)`, `guard_failure_policy=Reject` |
| `Move::Castle { king, rook }` | `predicate="castle"`, same kausal/modal; `on_enter` applies both king-move + rook-move atomically (no Cascade — it's *one* `on_enter` since the move is one transition) |
| `Move::EnPassant { from, to }` | `predicate="en_passant"`, capture is implicit in `on_enter` |
| `Move::Normal { promotion: Some(r), .. }` | same `predicate="move"`, but `on_enter` includes the role swap; the prompt-for-promotion is a *separate* user-input action when the UI is involved |
| `Move::Put { role, to }` (variants only) | `predicate="drop"`, gated by variant |
| resign | `predicate="resign"`, `kausal=None`, on-commit sets `Game.outcome` and transitions to terminal |
| offer_draw | `predicate="offer_draw"`, fires a *paired* Pending invocation on the other side; that's a Cascade |
| clock_tick (internal) | not user-action; the `state_timeout` carrier — `on_timeout(Pending)` → `Goto(Failed)` (flag-fall = loss on time) |
| **premove** | same as the corresponding move, but `guard_failure_policy=Postponable`: if illegal *now* (opponent hasn't moved yet), the invocation stays `Pending` and gets replayed FIFO when state changes |

## 4. Lifecycle binding — `1. e4` end-to-end

A single ply as it flows through the merged contract:

```text
ActionInvocation { identity: "ogit-chess::Game::001::ply::01",
                   realizes:  "ogit-chess::Game::action::move",
                   state:     Pending,
                   subject:   ActionSubject::User,         // White
                   object_instance: "ogit-chess::Game::001",
                   lokal:     LokalSpec { actor: "player::white::magnus" },
                   idempotency_key: Some("uuid-…"),        // §14 OLD↔NEW correlation handle
                   emitted_at_millis: Some(1733345678901), // Decision-#4 HLC slot
                   ... }
              │
              │  StateMachine::on_event(Pending, ActionDef{move}, ctx) →
              │
              │   guard:  pos.legal_moves().contains(&move)            // KausalSpec::StateGuard
              │   ├── true  → Transition::Goto(Committed)
              │   └── false → Transition::Goto(Failed)                  // PlayError → Pending→Failed
              ▼
StateMachine::is_commit(Committed) == true
              │
              ▼
CommitHook::on_commit(Pending, Committed, ctx) -> Result<(), ActorProcessingErr>:
   1. pos = pos.play(move)?;                                            // shakmaty applies the move
   2. row = CognitiveEventRow {
          subject: object_instance, predicate: "move",
          object: encode_move(&move),
          metadata: { fen_after, side_to_move=Black, halfmove_clock, … }
      };
   3. self.membrane.commit_event(row);                                  // LanceMembrane sole-writer
                                                                         // (gate-1 sibling; returns the new Lance version)
   Ok(())

Next ply: a fresh ActionInvocation with subject=Black, state=Pending.
The Pending → Committed → Pending → … sequence is the game.
```

The `Game.outcome` transitions to terminal when `pos.outcome() != Outcome::Unknown` —
no new `Pending` invocations spawn after that. The final ply's `Committed` is
the game's last Lance version.

## 5. Edge cases — each one exercises a distinct statem primitive

| Chess situation | Lowers to | Primitive exercised |
|---|---|---|
| **Illegal move** | `on_event` returns `Goto(Failed)` | `KausalSpec::StateGuard` rejection |
| **Premove** (queued while opponent is on clock) | `ActionDef.guard_failure_policy = Postponable`; `on_event` returns `Postpone`; replayed FIFO after opponent's Committed | **`Transition::Postpone`** + the FIFO replay invariant (the load-bearing `postponed_event_is_replayed_after_transition` test in the scaffold) |
| **Chess clock per side** | `ActionDef.state_timeout_millis = Some(remaining_ms)`; `on_timeout(Pending)` → `Goto(Failed)` | **`ogar:StateTimeout`** + the gen-stamped timer (auto-cancels at the crossing — Pending→Committed cancels the SLA for that side) |
| **Castling (compound move)** | `on_commit` applies both king + rook atomically | `ModalSpec::Atomic` — *one* `on_enter`, no Cascade |
| **En passant** | `on_commit` applies move + removes the captured pawn from `(to.file, from.rank)` | one `on_commit`, structural payload |
| **Pawn promotion** | `Move::Normal { promotion: Some(role) }`; `on_commit` swaps the role | `ActionDef.on_enter` carries the promotion role |
| **Check** | `pos.is_check()` after `on_commit`; not a separate ActionInvocation | derived predicate; *could* emit a Cascade `ogit-chess::Game::action::notify_check` (`subject=Cascade`) for the UI/observer side |
| **Checkmate / stalemate** | `pos.outcome()` becomes terminal; final ply's Committed writes `Game.outcome` | terminal state — no more Pending invocations; lifecycle ends |
| **Draw offer** | one `Pending` on offerer side + Cascade Pending on opponent; opponent's accept/decline transitions both | Cascade + paired lifecycle |
| **Threefold repetition / fifty-move** | claim is a separate `ActionDef`; engine sets `Outcome::Draw` on Committed | rule expressed as guard on the claim action |
| **Resign** | `predicate="resign"`, `on_commit` sets `Outcome::Decisive{winner: !color}` | terminal Cascade-free path |

Every primitive in the merged contract has a chess situation that exercises it
naturally. That's the substrate test.

## 6. §14 ground-truth — shakmaty (+ optional Stockfish)

The contract's wire-roundtrip framing (per `ELIXIR-HIRO-PREFETCH.md §2.4`)
applies directly:

```
record (shakmaty):  pos = Chess::default();
                    for each ply:
                        legal = pos.legal_moves();
                        choose move ∈ legal;
                        pos' = pos.play(move)?;
                        emit (FEN(pos), UCI(move), FEN(pos'))   // the truth tape
                    ...

replay (OGAR):      for each (FEN_before, UCI_move, FEN_after) in tape:
                        ctx = ActionInvocation { object_instance: …,
                                                 payload: UCI_move, … };
                        codegen_actor.fire(ActionDef{move});
                        on_event → Goto(Committed)
                        on_commit → membrane.commit_event(...)
                        FEN_ogar = read_back_position(version+1);
                        ASSERT FEN_ogar == FEN_after
                          (provenance-normalized: identity ULIDs, trace_id,
                           emitted_at_millis stripped; idempotency_key links rows)
```

This is **the §14 oracle for the substrate** — pass = the OGAR pipeline is
behaviour-equivalent to the reference implementation for every legal chess
move. Failures fall into the four §14 buckets (PASS / DIVERGENT-RECONCILABLE
/ DIVERGENT-FAULTY / INDETERMINATE) exactly as for any production workload.

**Optional second tier — Stockfish:** orthogonal correctness check. For any
position the OGAR pipeline produces, Stockfish should agree on legality
(`uci position fen <…> moves <…>` succeeds) and on the eval *sign* (who's
winning shouldn't flip between shakmaty's view and an independently
recomputed one). This catches subtle representation bugs that produce a
syntactically-valid-but-semantically-wrong position.

## 7. Producer shape — `ogar-from-shakmaty`

Mirrors the existing producers (`ogar-from-ruff` etc.). One crate, two
emit-modes (structural + behavioural per `ADAPTERS-AND-ACTORS.md §1`):

```rust
// crates/ogar-from-shakmaty/ (proposed; Sprint N)
pub fn emit_classes() -> Vec<Class>;      // §2 mapping — static, derived from shakmaty's pub API
pub fn emit_action_defs() -> Vec<ActionDef>;  // §3 mapping — one per Move variant + meta actions

// optional: PGN → ActionInvocation stream (the truth-tape side, for §14 replay)
pub fn pgn_to_invocations(pgn: &str) -> Result<Vec<ActionInvocation>>;
```

The producer is the only new code chess requires. Everything downstream
(codegen, callcenter, membrane) consumes it via the contract.

## 8. What this proves about universality

| Claim from the contract | Chess evidence |
|---|---|
| **"Adding a domain = adding a producer/adapter; never touching the codegen or core types."** (`§4`) | Chess attaches via `ogit-chess::` + an `ogar-from-shakmaty` producer. The core types, the state-machine crate, the codegen, and the membrane are all untouched. |
| **`State = ActionState` lifecycle is universal** (`§3, §5 carve-out 3`) | Every chess ply uses the same `Pending → Committed/Failed/Cancelled` lifecycle. The domain workflow (whose turn / `Outcome`) rides as guarded data, not as machine state — confirming the resolved binding. |
| **The §6 statem terms are sufficient** | `Postpone` (premove), `StateTimeout` (clock), `on_enter` (apply move), `guardFailurePolicy=Reject` (illegal moves) are all naturally exercised — no missing primitive. |
| **§14 wire-roundtrip is reachable end-to-end** | shakmaty itself is the producer of the truth tape; the OGAR pipeline replays it; FEN-equality (provenance-normalized) is the verdict. No external dependency to validate the substrate. |
| **"flexible enough to be everything later — bardioc, foundry, wikidata-med, who knows maybe AGI"** | If a closed formal domain like chess lowers cleanly onto the contract with zero core changes, an open domain (Wikidata-med via TTL hydrator) adds the same way: TTL or producer in, `Class` / `ActionDef` out. |
| **Semantik / Syntax / Pragmatik trichotomy is the substrate's organizing axiom** (§0; `ARCHITECTURE.md`) | Each chess layer maps to its OGAR carrier with an *independent* oracle: `Class`/`EnumDecl` ← Semantik (12 pieces × 64 squares — exhaustive enum); `Adapter` ← Syntax (FEN/SAN/UCI/PGN — published grammars, `parse(emit(x))==x` provable); `ActionInvocation` + lifecycle ← Pragmatik (`legal_moves()` + `play()` is the §14 oracle). The trichotomy survives the lowering — the universality claim has a receipt, not just a sketch. |

## 9. Cross-references

- `OGAR-AST-CONTRACT.md` — the typed surface chess lowers onto (`State=ActionState`, `Event=ActionDef`, `Context=ActionInvocation`).
- `ADAPTERS-AND-ACTORS.md` §3 — Action / SPO+TeKaMoLo / the actor-as-resolved-sentence.
- `ELIXIR-HIRO-PREFETCH.md` — the "type home now, wire later" pattern; same shape applied here.
- `ODOO-TRANSCODING.md` — the original domain transcoding precedent (Odoo ERP).
- `vocab/ogar.ttl` — the three statem terms (`onEnter`, `guardFailurePolicy`, `StateTimeout`) carried on `ActionDef`.
- Upstream: [shakmaty](https://github.com/AdaWorldAPI/shakmaty) (the crate this transcoding targets); upstream of that, [niklasf/shakmaty](https://github.com/niklasf/shakmaty).
- Runtime: `ractor_actors::state_machine` (the lifecycle shim chess lowers onto, signatures per `feat/state-machine-actor` @ `38a71a4`).
- §14 oracle: shakmaty's `Position::play(Move) -> Result<_, PlayError>` is the truth tape. Optional second tier: [stockfish](https://github.com/official-stockfish/stockfish) for eval-sign sanity.
