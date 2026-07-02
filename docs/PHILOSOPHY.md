# OGAR — Philosophy

> **English** · [Deutsch](PHILOSOPHIE.md) · [↑ README (spec)](../README.md)

This is the *why* before the *what*. For the spec, see the
[README](../README.md). For the operational discipline, see the
[seven design guards](../README.md#design-guards) in `docs/`.

---

### Ask a fencer with thirty years what fencing is, and she picks up a ladle and scoops water.

One motion. Unhurried, exact, nothing spilled. The whole art is in it —
the economy, the line, the weight known before the lift. It looks like
nothing. That *is* the thirty years.

OGAR is a ladle.

```rust
let cid = HealthcarePort::class_id("Patient");   // Some(0x0901)
```

One call. You pull the classid, and everything the concept needs is
already there — its shape, its grant, its lifecycle, its place in a
2.8 × 10¹⁴-cell address space shared by every app that ever named a
patient. You don't build it. You don't mint it. You **scoop**.

The rest of this document is the thirty years.

> Order flipped 2026-07-02 — canon HIGH / custom LOW: the canonical
> concept sits in the high u16, the per-app render prefix sits in the
> low u16. Pre-flip material and already-baked data use the legacy
> order; see `docs/DISCOVERY-MAP.md` D-CLASSID-CANON-HIGH-FLIP.

---

## The three epiphanies

Everything else is detail. If you only remember three things, remember
these — they are the muscle memory the whole stack is built to make
automatic.

### 1 · The classid is pure address. The magic is what it resolves to.

A classid is 32 bits of *address* — nothing more. It carries no
behaviour. Knowing it tells you *which* node, *whose* skin, *what*
shape — never *how it acts*.

```
classid : u32  =  0xDDCC ‖ 0xAAAA              both halves are ADDRESS
                    │         │
                    │         └─ lo u16 — WHOSE RENDER   (per-app skin)
                    └─────────── hi u16 — WHICH CONCEPT  (shared identity)

         ──────►  resolves to  ──────►
            ├─ ClassView                the SKIN    (render — per-app)
            ├─ Class                    the SHAPE   (structural — shared)
            └─ ActionDef + KausalSpec   the MAGIC   (behaviour — lifecycle,
                                                     callbacks, validations;
                                                     always in the Core,
                                                     never in the address)
```

The rails `class Patient < ApplicationRecord; validates …; before_save
…; end` blob — the *class magic* — is **not** in the id. It is what the
id keys into. The id addresses; the Core behaves.

### 2 · One concept, many renders.

The **high** 16 bits name the concept — shared by every app, with one
RBAC grant and one ontology. The **low** 16 bits name the render lens —
each app's own template, its own skin. Same concept, different dress:

```
0x0102_0001  ─ OpenProject's WorkPackage  ┐  same hi 0x0102 = project_work_item
0x0102_0007  ─ Redmine's Issue            ┘  → ONE grant, ONE ontology, TWO templates
```

Five apps can render *billable time* five ways and still resolve to the
**same** `0x0103` — so "planner hours align with billing" is a codebook
lookup, not a translation layer.

### 3 · The Core behaves; the producer feeds it; the adapter only echoes.

Behaviour flows **producer → OGAR `Class` + `ActionDef` → adapter** —
never producer → DDL. SurrealQL, PostgreSQL, OpenAPI, TypeScript are
**adapters**: lossy echoes of the Core, emitted *from* it, never the
spine. Trying to encode lifecycle in `DEFINE EVENT … WHEN … THEN …` is
putting the value into the key — the trap the
[design guards](../README.md#design-guards) exist to stop.

---

## The muscle memory, in one breath

> **Pull the classid. Never re-mint the Core.**

A consumer (medcare-rs, woa-rs, smb-office-rs, odoo-rs, …) is
*enrichment over a classid* — nothing more. Four moves, and only one of
them is ever yours to write:

| Move | The gesture | Yours? |
|---|---|---|
| **Pull** | `Port::class_id("Patient")` → `0x0901` | no — a pure function |
| **Render** | `(concept << 16) \| APP_PREFIX` → `0x0901_0005` | no — a typed helper |
| **Authorize** | `authorize(actor, concept, op)` | no — the shared grant lattice |
| **Enrich** | your domain logic keyed on the classid | **yes** — this is the part that's legitimately yours |

You never copy the codebook. You never construct a `*Bridge`. You never
mint a parallel registry. The Core owns the shape and the behaviour; you
hold the classid and add your domain. That is the scoop.

---

## Where to next

- [**README**](../README.md) — the spec (reverse-pyramid, pattern-dense)
- [**OGAR-CONSUMER-BEST-PRACTICES.md**](OGAR-CONSUMER-BEST-PRACTICES.md) — the muscle memory drilled with worked examples for every consumer
- [**SURREAL-AST-TRAP-PREFLIGHT.md**](SURREAL-AST-TRAP-PREFLIGHT.md) — the producer-side guard against epiphany 3's negative-beauty trap
- [**APP-CLASS-CODEBOOK-LAYOUT.md**](APP-CLASS-CODEBOOK-LAYOUT.md) — epiphany 2 in formal spec form
- [**THE-FIREWALL.md**](THE-FIREWALL.md) — the structural reason adapters can't be the spine

The README is the spec. The seven design guards in `docs/` are the
operational discipline. This document is just the why.
