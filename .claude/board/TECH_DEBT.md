# TECH_DEBT.md — known technical debt for OGAR

> **APPEND-ONLY.** Newest at top. Each entry: an id, a `**Status:**` line
> (OPEN / MITIGATED / RESOLVED — the only mutable line), what the debt is,
> why it was accepted rather than fixed, and what would retire it.
> Corrections append as new dated lines citing the original. This file is
> for debt that does not need a decision (that's `ISSUES.md`) and is not a
> dated finding (that's `EPIPHANIES.md`) — it is a standing, known gap
> someone should be able to find without re-deriving it from source.

## Entries (newest first)

## TD-0X03-CROSS-REPO-OCCUPANCY — the `0x03` Ontology domain has no cross-repo occupancy registry
**Status:** OPEN
**Filed:** 2026-08-17

### What the debt is

`crates/ogar-obo/src/registry.rs`'s own module doc for `META_STUDY_SPINE`
documents, in its own words, three real minting attempts before landing:

1. Minted at `0x0306..=0x030D` — collided with `ogar_ro::RELATION_BODY_CONCEPT_ID`
   (`0x0306`), a live, already-consumed constant in a sibling crate this
   crate cannot see (the dependency runs the other way: `ogar-ro` depends
   on `ogar-obo`, not vice versa).
2. Moved to `0x0310..=0x0317` — collided with **four** separate allocations
   held by a private downstream consumer, on an odd-stride run
   (`0x0307, 09, 0B, 0D, 0F, 11, 13, 15, 17, 19, 1B, 1D, …`) that no test in
   this repo can see and that keeps growing into odd slots.
3. Landed at `0x0340..=0x0347`, clear of both the core band and the known
   run — but the module doc is explicit that this is a **mitigation, not a
   guard**: *"there is no vantage point in this crate from which the block
   is verifiably clean."*

The guard that exists (`spine_does_not_collide_with_the_core_or_the_reserved_band`,
same file) only proves `META_STUDY_SPINE` is internally self-consistent and
clear of `OBO_CORE` — it cannot and does not prove clearance against
allocations held by consumer repos, because nothing in this crate (or
anywhere in OGAR) can see them.

### Why it was accepted rather than fixed

The actual fix — a registry that both a producer (this crate) and a
consumer (the private downstream repo, or any future one) can both read
before minting — does not exist yet. Building one is a real design task
(does it live in a shared crate? a manifest file consumers register into?
a runtime probe?), not a follow-up patch. Picking a "clean-looking" block
by eye is exactly the failure mode that produced collisions #1 and #2, so
a third eyeballed pick was rejected in favor of documenting the gap
honestly and shipping with the widest spacing available.

### What would retire it

A cross-repo `0x03XX` occupancy registry — some mechanism visible to both
OGAR and every downstream consumer that mints inside the Ontology domain,
checked (ideally at build/CI time, not just by convention) before a new
block is claimed. Until it exists, any future mint inside `0x03XX` should
re-read `registry.rs`'s module doc first and treat "looks clear" as
insufficient evidence.

### Refs
`crates/ogar-obo/src/registry.rs` (`META_STUDY_SPINE` module doc,
`spine_does_not_collide_with_the_core_or_the_reserved_band` test); the
collision-history commits (`git log --oneline -- crates/ogar-obo/src/registry.rs`).
