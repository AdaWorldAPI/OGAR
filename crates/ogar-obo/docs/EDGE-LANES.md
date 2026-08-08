# Edge lanes — the `col_idx` half of the CSR

> Status: **shipped**, measured on the full OBO core.
> Code: `crates/ogar-obo/src/edges.rs` · producer `bake()` · reader `EdgeLanes`.
> Probes: `examples/probe_edge_capacity.rs`, `examples/verify_edges.rs`.

## The gap this closes

A baked row is `key(16) | edges(16) | value(480)`. The `edges` block is a
per-predicate **degree histogram**: one byte per `Predicate` discriminant,
saturating at 255. It says *how many* targets a node has per predicate.

It does not say *which*. In CSR terms the bake persisted `row_ptr` and dropped
`col_idx`. The consequence is sharp and was worth stating plainly before any
code was written: **a row could describe its own local shape and still not be
traversed.** Every traversal had to fall back to the out-of-line SPO triple
table, which is a different artifact with a different lifetime.

This module puts the targets in the row. The degree histogram thereby stops
being decorative and becomes load-bearing — it is the index that tells a reader
which predicate each successive target belongs to.

## The lane form

A lane is the one form the substrate has: `classid(4) + 12 content bytes`, and
**the classid says how to read its own 12 bytes.** An edge lane reads its
content as the G2 `4×u24` carving — four literal links — and carries **the
target ontology's own row classid**.

```text
lane:  | classid: u32 | u24 | u24 | u24 | u24 |
         ^ target ontology     ^ four CURIE numerics
```

So a link is exactly the `(classid, identity)` pair a *key* carries. The same
binary search that resolves a key resolves a link; nothing new is invented to
point at a node. `edges::subject()` and `edges::Link` are deliberately the same
shape, because a subject and a target are the same kind of thing.

### Why `4×u24` and not the `6×(u8:u8)` rails

The L1 rails are the cheaper form and the eventual destination — but a rail ref
is **basin-local**, capped at 256 targets, and explicitly never a global
pointer. The bake has no basin: HEEL/HIP/TWIG and leaf are **zero on all 68,797
rows** (the cascade is dormant, RESERVE-DON'T-RECLAIM), while identities run
past 8.9 million. With no basin to be local to, a one-byte ref has nothing to
mean.

So the G2 carving is spent exactly where a rail cannot reach, and only there.
When an HHTL basin mint wakes routing, the rails become the cheaper in-basin
form; the offsets are already reserved and the migration costs no re-bake.

This is also why the *identity* does **not** use G2: it rides the V3
`family:identity` rail (`family = num >> 16`, `identity = num & 0xFFFF`), which
carries the same 24 bits as an `X:Y` pair with 8 bits of headroom — the thing a
flat u24 cannot do, because it has no axis. Sort order is unchanged, since
`family` holds the high bits, so ordering by `(classid, family, identity)` is
still ordering by `(classid, num)` and binary-search-by-key survives.

## Layout

The 480-byte value slab is 30 lanes.

| lanes | use |
|---|---|
| 0–5 | reserved for further value tenants (RESERVE, DON'T RECLAIM) |
| 6 | `EntityType` tenant — a `u16` namespace ordinal at slab offset 96 |
| 7–29 | **edge targets** — 23 lanes × 4 links = **92 slots** |

`EDGE_LANE_SLAB_OFFSET = 112`, `EDGE_LANE_COUNT = 23`, `EDGE_SLOT_COUNT = 92`.
A `const _: () = assert!(…)` pins that the edge lanes run to the end of the row
without overlapping a value tenant, so a future tenant carve cannot silently
collide with them.

## The reader's contract

Targets are emitted ascending by `(predicate, target namespace, numeric)`, and
**a lane never mixes target ontologies** — its classid names exactly one, so a
namespace change starts a new lane even mid-predicate. That fragmentation is
deliberate: it is what keeps a lane's classid from lying.

Unused slots in a lane are `0`. That is unambiguous, and it is a *measured*
fact, not an assumption: `probe_edge_capacity` checks assumption **A1** — CURIE
numeric `0` never occurs — across the real sources, and it does not.

A reader walks lanes in order and consumes `degree(p)` links for each predicate
`p` in ascending order, treating a `0` slot as "this lane is spent, advance".
That is `EdgeLanes`, which borrows the row and decodes a `u24` with a shift —
no allocation, no gathered window, nothing owned that the lane already holds.

```rust
let lanes = EdgeLanes::new(&row.0);
lanes.degree(Predicate::IsA);          // row_ptr: one byte, zero value decode
lanes.links_of(Predicate::IsA);        // col_idx: borrowed iterator
```

The predicate is **not stored on the lane**. Recovering it from the histogram is
what makes the two halves one structure rather than two parallel encodings that
can disagree.

## Measurements

All from `probe_edge_capacity` and `verify_edges` on the full OBO core
(68,797 rows), run before the layout constant was chosen.

| quantity | value |
|---|---|
| subjects with ≥1 edge | 68,789 |
| total triples | 186,029 |
| max out-degree (one row) | 62 |
| **max lane demand (one row)** | **16** (against a 23-lane budget) |
| rows exceeding 16 lanes | 0 |
| A1 — CURIE numeric `0` occurrences | 0 |
| A2 — numerics over 24 bits | 0 |
| links packed / dropped / rows overflowed | 186,029 / **0** / **0** |

Lane demand, not out-degree, is the binding constraint — the `(predicate,
namespace)` grouping fragments lanes, so a 62-target row needs 16 lanes rather
than `ceil(62/4) = 16`… which here coincides, but does not in general, and the
probe measures the real thing rather than the convenient proxy.

### No silent truncation

`pack_edges` reports overflow in `PackStats` and writes nothing past the
budget; `bake` accumulates `links_dropped` / `rows_overflowed` so a driver can
**refuse to ship** an artifact that is missing edges. A packer that quietly
truncated would produce a graph that looks complete and is not.

## Verification

Three independent paths agree, which is the point — a single path agreeing with
itself would prove nothing.

1. **Lane decode vs the triple table.** `verify_edges` reads links back out of
   the row bytes and compares them as a multiset against the SPO triples the
   bake built from the parsed source. Different code, same input.

   | ontology | rows | links read | `is_a` degree total |
   |---|---:|---:|---:|
   | RO | 4 | 3 | 2 |
   | UBERON | 14,975 | 71,844 | 27,170 |
   | HP | 19,836 | 50,284 | 31,284 |
   | PATO | 1,887 | 3,389 | 2,909 |
   | MONDO | 32,095 | 60,509 | 46,448 |

   All MATCH. The `is_a` totals also equal the degree bytes already present in
   the previously-baked slabs, so the packing agrees with the artifact it
   extends — that was the falsifier fixed *before* any code was written.

2. **Closure vs saturation.** Reading `is_a` out of the lanes and closing it
   (see `ogar-elk`) yields **738,651** transitive ancestors and **0**
   equivalence cycles. `reason::saturate` computes 738,651 subsumption pairs
   and 0 cycles from the out-of-line triple table. Two reasoners, two
   representations, identical to the unit.

3. **By hand.** RO is four rows and was checked against the source directly:

   ```text
   RO:0002532 --Other--> RO:0002533     (relationship: RO:0002524 RO:0002533)
   RO:0002533 --is_a---> RO:0002532
   RO:0002534 --is_a---> RO:0002532
   RO:0002577 --(none)                  (only parent BFO:0000040 — outside the core)
   ```

   `RO:0002577` carrying *zero* links is the interesting case: its degree byte
   is 0, not a dangling 1, so the histogram and the lanes agree about an edge
   that was correctly never emitted.

## Relation to `ogar-ro`

`ogar_obo::Predicate` and the `ogar-ro` crate are **different things** and
should not be conflated:

- `ogar_obo::Predicate` (discriminants 1–7) is the byte palette **carried on
  baked edges** — it indexes the degree histogram and orders the lanes. It is
  the THINK arm: what the baked graph asserts.
- `ogar-ro` is a callable **vocabulary** (`RELATION_BODY_CONCEPT_ID = 0x0306`)
  that mints RO/BFO predicates as `FnIndex` bytes so a template body can
  *assert* an edge — `Call = (predicate : subject, object)`, a statement, not
  an expression. It is the DO arm, and its operands resolve against a
  basin-local `RELATION_TARGET_CODEBOOK`, not against CURIE numerics.

They meet at the ontology, not in the bytes.

## Open

- **The rails are unused until a basin exists.** The migration path is named
  above and costs no re-bake, but it has no scheduled trigger.
- **`LaneShape` / `CascadeShape` duplication (W-RO-5)** is an unresolved
  cross-repo question recorded in `ogar-ro`'s crate docs, deliberately not
  decided by either side.
- **Degree saturation** at 255 is untested against real data because the
  measured maximum is 62. If a future source pushes a single predicate past
  255, the histogram silently clamps while the lanes would still hold the
  targets — the two halves would disagree. A guard belongs here before any
  source with that shape is baked.
