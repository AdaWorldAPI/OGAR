# CONSUMER MIGRATION HOW-TO — onto OGAR classid-first

> Audience: every downstream consumer of the ontology/bridge surface —
> **woa-rs, smb-office-rs, odoo-rs, openproject-nexgen-rs, q2, neo4j-rs,
> gotham**, and any future one. One page, same shape for all.

## The principle (read once)

A consumer is **enrichment over a `classid`**, nothing more. It does NOT
own an ontology, a registry, or a bridge. It:

1. **Pulls a `classid`** for its surface entity (static OGAR codebook
   lookup), then
2. **Enriches** by that classid (its RLS / masks / projections / domain
   behaviour), and
3. **Authorizes** by that classid (`lance_graph_rbac::authorize(actor,
   classid, op)` — the keystone, see `CLASSID-RBAC-KEYSTONE-SPEC.md`).

The class carries its own schema *and* its own access policy (AR/Rails
virtue). The consumer never re-declares either. **Cost of migrating =
one OGAR PortSpec (the adapter) + a thin enrichment module. Zero spine
edits, zero per-consumer bridge.**

## Prerequisite — does your domain have an OGAR PortSpec?

A `PortSpec` is the agnostic adapter: **surface name → canonical
`classid`** (`ogar_vocab::ports`). It is *data* (an alias table), lives
in OGAR, names no consumer code.

| Consumer | PortSpec | Status |
|---|---|---|
| openproject-nexgen-rs | `OpenProjectPort` | ✅ exists |
| (redmine) | `RedminePort` | ✅ exists |
| woa-rs | `WoaPort` (`WorkOrder`) | ✅ exists (OGAR #93) |
| smb-office-rs | `SmbPort` (`SMB`) | ✅ exists (OGAR #93) |
| odoo-rs | `OdooPort` (`Odoo`) | ✅ exists (OGAR #94) |
| **q2** | — | ❌ **author first** (map q2's graph entities → classids) |
| **neo4j-rs** | — | ❌ **author first** (legacy; map its node/rel types → classids) |
| **gotham** | — | ❌ **author first** (map its domain → classids) |

**If your row is ❌:** the FIRST migration PR is in **OGAR** — add a
`YourPort: PortSpec` to `ogar_vocab::ports` mapping your public names
(EN + native synonyms collapse to one canonical id) onto the existing
codebook `class_ids`. Mint **no** new `class_id` unless your concept is
genuinely new to the codebook (most map onto the `0x01XX`/`0x02XX`
commerce/work blocks or a domain block like Health `0x09XX`). Pattern:
copy `OdooPort` (OGAR #94) or `WoaPort` (#93). Tests: assert
`YourPort::class_id("Name") == Some(0xNNNN)` and the cross-fork
convergence pins.

## The migration, step by step (identical for every consumer)

1. **Confirm/author your PortSpec** in OGAR (above). This is the only
   per-domain artifact, and it's data.
2. **Pull the classid, statically.** Replace any
   `use lance_graph_ontology::bridges::XBridge` /
   `use lance_graph_ogar::bridges::XBridge` with the static port lookup:
   ```rust
   let cid = ogar_vocab::ports::YourPort::class_id(entity_name); // Option<u16>
   ```
   No registry, no hydration, no construction. (Template:
   `MedCare-rs/crates/medcare-analytics/src/rls_policies.rs:170`.)
3. **Delete the per-consumer bridge.** Remove `XBridge` (= the
   `UnifiedBridge<XPort>` alias), any hand-rolled `XRegistry`/hydration,
   and the crate/module that existed only to house them. The consumer
   holds **no** ontology.
4. **Enrich by classid.** Your domain logic (row-level security, column
   masks, projections, view shaping) keys on `cid`. This is the part that
   is legitimately yours.
5. **Authorize by classid.** Once the keystone lands, gate access with
   `lance_graph_rbac::authorize(actor, cid, op)`. The actor is its
   membership set (I-K6); roles/grants/inheritance are resolved upstream.
   Until the keystone lands, keep your existing auth (or none) — do NOT
   re-introduce a bridge as a stopgap.
6. **Verify (the litmus):**
   - `grep` your repo: no `XBridge` / `UnifiedBridge<…>` symbol survives.
   - the classid pull is a pure function call (no `Registry`, no
     `hydrate`, no `OntologyRegistry` field).
   - your diff touches only OGAR (a port, if new) + your own crate. The
     agnostic spine (`lance-graph-ogar`, `lance-graph-rbac`) is **byte-for-
     byte unchanged**. If you edited the spine, you did it wrong.

## Per-consumer notes

- **woa-rs** — `WoaPort` exists. Repoint `src/registry.rs`,
  `src/unified_bridge.rs`, `src/lib.rs`, `tests/…` off
  `lance_graph_ontology::bridges::WoaBridge` to the classid pull. Iron
  Rule 1 (CLAUDE.md) allow-list: the dep you keep is `ogar-vocab` (for the
  port) + `lance-graph-rbac` (for authorize) — NOT a brain crate. File the
  RFC for the allow-list delta. Spec: `.claude/board/OGAR-MIGRATION-GAP-2026-06-21.md`.
- **smb-office-rs** — `SmbPort` exists. `crates/smb-bridge/src/unified_bridge_wiring.rs`
  drops `OgitBridge`; pull `SmbPort::class_id`. Tracked:
  `.claude/board/TECH_DEBT.md TD-OGAR-CONSUMER-MIGRATION-1`.
- **odoo-rs** — `OdooPort` exists (OGAR #94). The bigger move: odoo-rs
  forks `op-surreal-ast` / `ogar-adapter-surrealql` in its bespoke
  `od-ontology::{surreal_ast,triple,emit}` and **never touches
  `ogar-vocab`**. Converge it: lower onto `ogar_vocab::Class`, emit via
  `ogar-adapter-surrealql`. Delete the fork. (This is the severity
  case — it re-derives the AR layer OGAR exists to own.)
- **openproject-nexgen-rs** — `OpenProjectPort` exists. Pull
  `OpenProjectPort::class_id`; drop any `OpenProjectBridge` usage.
- **q2 / neo4j-rs / gotham** — **no port yet.** Step 1 (author the OGAR
  PortSpec) is the gate; until their domain entities are mapped onto
  canonical `class_id`s in OGAR, they cannot pull classids. Define the
  port first (its own OGAR PR), then the generic steps apply unchanged.
  neo4j-rs note: it is legacy (superseded by lance-graph as L3) — confirm
  it is still a live consumer before authoring its port.

## Why this is the whole story

Adding or migrating a consumer is **one OGAR port + one enrichment
module**. The spine never learns the consumer's name; every other
consumer is untouched; the class carries its shape and its policy. That
is the agnostic, AR-shaped target — not a bridge pretending to be one.
