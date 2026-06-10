# CLAUDE.md — OGAR (Open Graph of Active Record)

> Auto-loaded session preamble. The canon pins live here; the detail
> lives in `docs/`. Read `docs/DISCOVERY-MAP.md` (what was found) and
> `docs/INTEGRATION-MAP.md` (how it composes) before proposing anything.

## P0 — THE CANONICAL GUID (operator-pinned; counted in HEX, not bits)

```
xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
classid    HEEL   HIP    TWIG   family-basin-leaf(6) + identity(6)
8 hex      4 hex  4 hex  4 hex  12 hex
```

32 hex = 128 bit = the GUID itself. **The UUID's own dash-groups ARE the
semantic delimiters** — every printed GUID is self-describing at sight.
1 hex digit = 1 nibble = 1 level of the 16-ary tree (`FAN_OUT=16`).
Widths are codebook cardinalities; **scale = the next cascade level,
never field-widening.** Wrappers (e.g. lance-graph `NodeGuid`, #480) are
audited against this canon group-by-group — never the reverse.

## P0 — THE GUID IS THE KEY OF KEY-VALUE (operator-pinned, 2026-06-10)

The substrate is a key-value store whose **key is the canonical GUID**:

- **The key prerenders nodes — in any way — with zero value decode.**
  classid → the class template (`ClassView`); HEEL/HIP/TWIG → the
  cascade position; basin+leaf → the family neighborhood; identity →
  the instance. A renderer/router/planner can lay out, group, route,
  and skeleton-render nodes from keys alone, before (or without ever)
  fetching a value.
- **A node is 4096 bits: `key(128/GUID) + value(3968)`** — a 512-byte
  block, 16-byte key, 496-byte value; the value is simply everything
  the key isn't. The beauty: Lance is free to compress the value bits
  arbitrarily — columnar encodings, dictionary, PQ, anything — and the
  store **still has a transparent view and address**, because the key
  is never compressed and never needs the value decoded to be useful.
  Compression never costs addressability.

## Doc family (read in this order)

1. `docs/DISCOVERY-MAP.md` — the discovery ledger (D-* entries, graded
   `[G]`/`[H]`/`[S]`, append-only).
2. `docs/INTEGRATION-MAP.md` — layers, seams (each with its contract
   TYPE), the phase DAG, falsification gates F1–F14.
3. `docs/OGAR-AST-CONTRACT.md` — the IR type surface (THINK arm `Class`
   / DO arm `ActionDef`+`ActionInvocation` / membrane `KausalSpec`).
4. `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` — ADR-001..025
   (ADR-026 pending).
5. `.claude/agents/` — the 5+3 hardening pattern (5 research savants +
   3 brutally-honest reviewers). Run it before any claim enters the
   canon. Theorem-checker rule 0: **pin the unit system first** (bits
   vs hex vs bytes — born from a real failure).

## Non-negotiables

- **The Firewall (ADR-022/023):** no serialization in the hot path;
  the IR is wire-truth; inter-mailbox state is Batons.
- **PII:** never emit German PII labels (medcare-rs leaf-rename at the
  adapter is the guarantee). Word-boundary abort-guard before commit.
- **No model identifier** in any committed artifact (chat only).
- **Shell discipline:** `grep`/`sed`/`tail`/`head`/`awk` via Bash are
  prohibited — use the Grep/Read/Glob tools.
- **Append-only canon:** never delete a ledger entry; regrade in place;
  corrections cite their pass (savant / G-pass / canon-pass).
