# WASM Consumer Guide — the browser is a consumer like any other

> **Audience:** any session compiling a consumer to `wasm32-unknown-unknown`
> and feeding it an ABI byte stream — a2ui-rs' field renderer today, any
> future browser-side surface.
>
> **Why this doc:** the browser looks like a new tier and is not. It is a
> consumer, bound by the same invariant as every other one — *the classid is
> pure address; the magic is what it resolves to.* What genuinely changes at
> the wasm boundary is narrow: how bytes get in, what survives the link, and
> where arithmetic gets its vector registers. This doc is those three things
> and nothing else, so a browser session does not re-derive the canon and
> does not re-implement it either.
>
> Status: **GUIDE v1** (2026-08-14). Append-only.
>
> Companions: `OGAR-CONSUMER-BEST-PRACTICES.md` (the muscle-memory guide —
> read it FIRST; everything there applies unchanged in the browser),
> `A2UI-SCREEN-ADDRESSING-PROPOSAL.md` (the addressed-surface proposal).
> Sibling repos: `AdaWorldAPI/ndarray`
> `.claude/knowledge/wasm-simd-consumer-guide.md` (the SIMD polyfill half),
> `AdaWorldAPI/a2ui-rs` `docs/WASM-INTEGRATION.md` (the end-to-end recipe).

---

## §0. What does NOT change

Everything in `OGAR-CONSUMER-BEST-PRACTICES.md`. A wasm consumer pulls a
classid the same way, composes a render address the same way, authorizes
the same way, and is subject to the same anti-pattern catalogue.

The consumer trap has a browser-shaped variant worth naming, because the
wasm boundary invites it: **the FFI edge is not a licence to keep a local
copy of the canon.** A codebook mirrored into JS "so the client doesn't
have to ask" is the same violation as a `*_CODEBOOK` copy in a server-side
consumer, with a worse blast radius — it now drifts per browser tab.

The consumer resolves; it does not re-implement.

---

## §1. Why the browser can render before it understands anything

This is the P0 pin doing real work rather than sitting in a preamble:

> **The key prerenders nodes — in any way — with zero value decode.**

For a browser consumer that is not an optimisation, it is the entire
architecture:

- `classid` → the class template. Layout, grouping and skeleton render are
  decided from the key alone.
- HEEL / HIP / TWIG → the cascade position, which is *already* a spatial
  arrangement. A field renderer does not need a layout oracle to know where
  things roughly belong.
- family + identity → neighbourhood and instance.

So a client can lay out and draw a graph field having decoded **no values
at all**. The value half can arrive later, arrive partially, or never
arrive, and the surface stays coherent. That is what makes a browser tier
viable over a network that a server-side consumer never has to think about.

The corollary is a discipline, not a freedom: **a wasm consumer colours by
the byte and never interprets it.** `domain` and `vocab` in a wire lane are
opaque palette indices. What a domain *means* belongs to the consumer's own
codebook, resolved through the class — never guessed in the renderer from
the byte's numeric value.

---

## §2. Getting bytes in — one copy, at the boundary, never per frame

The wire crosses into wasm as a `Uint8Array`. Two rules keep the Firewall
(ADR-022/023, *no serialization in the hot path*) intact on this edge:

1. **No JSON, ever, on this path.** The ABI *is* the wire format. A browser
   consumer that receives JSON and builds objects has re-introduced exactly
   the serialization the Firewall exists to remove, and has additionally
   allocated one object per node — which is what makes a large field
   impossible rather than merely slow.

2. **Own the bytes once, borrow forever after.** Take owned bytes at the
   mount boundary and let every read borrow into that one buffer. A
   borrowed slice across the FFI ties the client's lifetime to a buffer JS
   is free to release; a per-frame copy re-pays the cost the zero-copy view
   exists to avoid. One copy when the stream arrives, none thereafter.

Everything after that is offsets into a single allocation: no per-node
object, therefore nothing to leak and nothing for a garbage collector to
walk.

---

## §3. The link is a filter — this is the trap that is specific to wasm

A browser build is a `cdylib`, and **only exported items survive the
link.** A consumer crate can compile, link, emit a `.wasm`, and contain
none of the subsystem it was built for, because nothing reachable from an
export calls it.

Measured 2026-08-14 in `a2ui-rs`, release, browser feature enabled:

| | before exports | after exports |
|---|---|---|
| module size | 1 269 153 B | 4 880 319 B |
| SIMD instructions | 2 | 13 375 |
| layout/client symbols | **0** | 121 |

The client struct had no export attribute, so the linker removed the whole
subsystem. Nothing warned. `crate-type = ["cdylib", "rlib"]` is necessary
and **not sufficient**: the crate type produces a module, the exports
decide what is in it.

The check is one command and belongs in any browser consumer's definition
of done:

```bash
wasm-objdump -x module.wasm | grep -ci '<your crate>.*<your module>'
```

A **shrinking** module after adding a subsystem is the same tell.

---

## §4. Arithmetic — the browser is where "the compiler will vectorise it"
stops being true

Consumer-side math routes through `ndarray::simd`, workspace-wide. On x86
and ARM that rule is often invisible in effect, because an autovectoriser
has registers to aim at and a well-shaped loop frequently becomes SIMD by
accident.

**wasm32 has no vector registers without `+simd128`.** There is nothing for
the autovectoriser to find, so a scalar loop stays scalar — silently,
correctly, and slowly. The browser is therefore the target that turns the
polyfill rule from hygiene into a requirement.

The recipe, the feature gates, the wasm32 backend's type coverage, and the
cross-backend semantic divergences a parity test must tolerate all live in
`AdaWorldAPI/ndarray` `.claude/knowledge/wasm-simd-consumer-guide.md`. Two
points belong here because they are canon-shaped rather than build-shaped:

- **Vectorise the phase that is shaped for it, and say why the others are
  not.** An elementwise sweep over parallel lanes is a lane workload; a walk
  over a grid or a scatter along an edge list is not, and forcing lanes onto
  it costs more in shuffles than the arithmetic saves. A blanket
  "SIMD-accelerated" claim over a mixed pipeline is a dilution.

- **A measurement is not a benchmark.** An instruction count proves the
  vector path *exists*. Whether it is faster on a real device is a separate
  claim requiring a separate measurement, and the two must not be conflated
  in a doc, a PR body, or a ledger entry.

---

## §5. Verify with a contrast, or do not claim it

A SIMD count on its own proves nothing: any large module carries vectorised
dependency code, and a raw opcode-byte scan is dominated by data. Build both
ways; the **zero** is what gives the other number meaning.

Two traps, both hit in practice on the day this doc was written:

- **Do not isolate a function with a text window around its name.** A regex
  match on a symbol name is not a function boundary, and an object archive
  is full of the polyfill's own vectorised functions — the count can be
  almost entirely somebody else's code. Ask the symbol table
  (`llvm-nm` + `--disassemble-symbols`) instead. The text-window form was
  used once and returned 801 where the symbol-scoped answer is 800: right by
  luck, which is not a measurement.

- **Follow contradictions before explanations.** The bad receipt above was
  not caught by re-reading it. It was caught because 2 instructions in the
  shipped module could not be reconciled with 801 in the archive — and
  chasing that gap found §3's linker bug, which was the real defect. A
  sloppy number costs more than its own inaccuracy: it costs the anomaly it
  would otherwise have exposed.

---

## §6. Definition of done for a browser consumer

1. Reads the ABI wire directly; no JSON on the hot path (§2).
2. Owns the byte buffer once at mount; borrows thereafter (§2).
3. Its subsystem is verifiably **present in the module** (§3).
4. Its per-frame arithmetic goes through the polyfill, built with the
   target feature, verified against a zero-contrast (§4, §5).
5. Resolves classids through the canon; keeps no local copy of it (§0).
6. Names what it did **not** measure (§4).
