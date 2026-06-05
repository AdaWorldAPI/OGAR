# The Firewall — the substrate's absolute inner/outer boundary

> **Purpose.** Name and pin the substrate's most load-bearing invariant:
> **The Firewall** — the absolute boundary between the *inner* hot-path
> architecture (compile-time HHTL, zero serialization) and the *outer*
> boundary (contract-trait pluggable backends, where boundary tax is
> paid). Declared **absolute** by the operator (2026-06-05).
>
> **The one-line rule: no serialization in the hot path.** Serialization
> (serde, JSON, Arrow IPC encode/decode, any to-bytes/from-bytes),
> network round-trips, crypto/signing, and external schema reads are all
> *firewall-crossing* concerns — paid once at entry/exit, **never per
> inner operation**.
>
> **Why this doc exists.** This is the principle that governs every
> "where does X live / can X take a heavy dep / is X allowed on the hot
> path" question. Without it pinned, future work erodes the hot path one
> "small" serialization at a time. It's the umbrella the prior
> invariants (I-2: no tokio on the hot loop; BBB: Arrow-scalar on the
> hot loop) are facets of.
>
> Companion: `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` ADR-022
> (the decision record); `docs/SUBSTRATE-ENDGAME.md` §5 (the SDK seam
> the outer boundary enables); `docs/SURREAL-AST-AS-ADAPTER.md` (the
> structural/behavioral split that sits *inside* the firewall).
>
> Status: **CARVED v0** (2026-06-05). Absolute invariant — changes
> require an explicit superseding ADR, not incremental erosion.

## 0. The principle in one diagram

```
        OUTER  (the world)                 ║ FIREWALL ║        INNER  (the substrate)
   ───────────────────────────            ║          ║   ──────────────────────────────
   Redis / SeaORM / Postgres              ║          ║   compile-time HHTL identity
   external schema sources                ║  serialize  resolution (const / typestate)
   cross-process / cross-server   ──────► ║  decode   ║   Rubicon on_event dispatch
   HTTP / gRPC / wire formats             ║  crypto   ║   intra-process actor messaging
   storage (Lance row append)             ║  network  ║     (Arc<RecordBatch>, zero-copy)
                                          ║          ║   SoA column projection
   "boundary tax paid HERE,               ║   PAID    ║   Arrow-scalar values (BBB)
    once per crossing"                    ║   ONCE    ║   std::sync only (I-2)
                                          ║          ║
   contract traits:                       ║          ║   NO serialization
     ExternalMembrane                     ║          ║   NO serde
     KnowableFromStore                    ║          ║   NO trait-object dispatch for
     (LazyLock fallback)                  ║          ║      known/compile-time-resolved
                                          ║          ║      identities
                                          ║          ║   NO tokio (I-2)
                                          ║          ║   NO allocation for the
                                          ║          ║      resolved hot path
```

Two rules, one absolute:

1. **(Absolute) No serialization in the hot path.** If a code path runs
   per-message / per-actor-dispatch / per-identity-resolution, it does
   **not** serialize, decode, or `serde`. Full stop.
2. **(Corollary) Boundary tax is paid at the firewall, once per
   crossing.** Serialization, crypto, network, external schema reads are
   *legitimate* — at the firewall. They cross once (entry or exit), the
   result is cached (`LazyLock`/in-memory), and the inner path never
   pays again.

## 1. The inner — compile-time HHTL, zero serialization

The inner architecture is **compile-time HHTL**, exactly as OGIT is
compile-time-checked (per `TARGET_STACK_REFERENCE` / the parity
matrix: "OGIT compile-time check — Rust types encode the SNRA
required-attrs; stronger than runtime"). OGAR gets the same property:

- **Build-time codegen → compile-time artifacts.** OGAR's `Class` /
  `ActionDef` IR is lowered at *build time* (jinja / xml / whatever
  templating — the operator explicitly blessed this) into compile-time
  HHTL structures: `const` prefix-radix tables, typestate, generated
  Rust types. The NiblePath identity resolution (`ogit-op::WorkPackage`
  → class metadata) is resolved **at compile time**, not via a runtime
  hashmap or a runtime trait call.
- **Hot path = const-resolution, zero serialization.** On the hot path
  — Rubicon `on_event`, identity routing, intra-process actor dispatch
  — known classes resolve through the compile-time HHTL with no
  decode, no `serde`, no allocation for the resolved path. Values pass
  as Arrow scalars (the BBB invariant) and `Arc<RecordBatch>`
  (zero-copy); concurrency primitives are `std::sync` only (the I-2
  invariant).
- **No trait-object dispatch on the resolved hot path.** Pluggability
  (the contract traits in §2) is an *outer* concern. Once an identity
  is compile-time-resolved, dispatch is direct — no `dyn` indirection
  for the known case.

The litmus for "is this inner?": **does it run per hot operation?** If
yes, it must be compile-time-resolved and serialization-free.

## 2. The outer — the `ExternalMembrane` contract seam

The outer boundary is where the substrate meets the world: external
storage, external schema sources, other processes, other servers. Here,
**pluggability via contract traits is the SDK seam**, and boundary tax
(serialization, network, crypto) is *acceptable* because it's paid once
per crossing.

- **Contract traits are the seam.** `lance-graph-contract::ExternalMembrane`
  (the existing zero-dep contract trait) + this repo's
  `ogar-knowable-from::KnowableFromStore` (the §10.3 producer seam) are
  the firewall's outer interface. External backends — **Redis, SeaORM,
  Postgres, "schema-from-whatever"** — implement these contracts. The
  impls live at `lance-graph-callcenter` + `lance-graph-contract`
  (the firewall layer), never inside `ogar-vocab` / the IR / the hot
  path.
- **SDK pluggability is the point.** Per `SUBSTRATE-ENDGAME.md §5.3`,
  Room 5 is Foundry-OSS-class capability with deployment flexibility.
  A deployment with 20 years of Postgres infra writes
  `impl ExternalMembrane for MyPostgres` (or `impl KnowableFromStore
  for MyPostgresRegistry`); the substrate doesn't know or care. The
  Lance impl is the *reference*; the trait is the *contract*. This is
  precisely why pluggability lives at the firewall and not inside —
  it's a boundary feature, not an inner one.
- **Fallback = lazy read behind `LazyLock`.** When schema can't be
  resolved at compile time (an unknown external source), the fallback
  is: read the schema once at the boundary, cache it behind a
  `std::sync::LazyLock` (read-once, lock-in), and the inner path then
  treats it as resolved. The boundary tax (the read + decode) is paid
  exactly once; the hot path never re-reads.
- **Reference store backend — VART (timed radix trie) or Lance dataset.**
  The outer-boundary schema/registry store has two natural reference
  backends behind the contract trait: **VART** (`AdaWorldAPI/vart` —
  the timed adaptive radix trie SurrealKV is built on, already in the
  surrealdb-fork dep tree) and a **Lance dataset**. VART is the natural
  fit for the registry: it's a prefix-radix trie (NiblePath-native, so
  class identities compress to the floor), its *timed* version stamp
  **is** the `knowable_from` value, and its append-only history is an
  immutable audit trail for free (the §7.2 HIPAA need). Both are
  outer-boundary — the append/serialize is *the firewall crossing*. The
  same VART-append serves OGIT's identity register or OGAR's schema
  registry ("~20-minute outer-boundary addon" per the operator;
  detail in `crates/ogar-knowable-from`).
- **Serialization happens HERE, and only here.** Writing a Lance row
  (`KnowableFromStore::register`, `LanceMembrane::commit_event`),
  encoding a cross-process message, talking to Redis — all serialize.
  That's correct: they're firewall crossings. `serde` derives on OGAR
  types are *feature-gated* precisely so they're available at the
  boundary without leaking into the hot path's dependency surface.

**Proven precedent.** The external-membrane-via-contract pattern is
already shipped in sibling AdaWorldAPI projects: **a production HIPAA instance**
(the `Membrane` pattern in (consumer-side) +
`.claude/patterns.md`; `LazyLock` in (consumer-side)) and
**a production ERP instance** (SeaORM backend in `src/db.rs` + `src/migrations.rs`). The
operator's estimate: "~20 minutes work" to add a given external
membrane — *because it's outer-boundary caking, not inner
architecture*. The pattern is a known quantity; the firewall is what
keeps it from leaking inward.

## 3. The litmus test — "crypto on post stamps"

The operator's metaphor is the test: **you can print crypto on a
postage stamp, but it's not a hot-path concern.** Stamping the envelope
(the boundary crossing — mailing) is where expensive work belongs;
re-stamping on every inner hand-off is forbidden.

| Cost | At the firewall (once per crossing) | On the hot path (per inner op) |
|---|---|---|
| Serialization (serde / IPC encode / decode) | ✅ acceptable | ❌ **absolute no** |
| Crypto / signing / content-addressing | ✅ acceptable (the "post-stamp crypto") | ❌ no |
| Network round-trip (Redis / HTTP / gRPC) | ✅ acceptable | ❌ no |
| External schema read | ✅ acceptable (once, then `LazyLock`) | ❌ no |
| `tokio` async | ✅ Layer-3 cold sinks (I-2) | ❌ no (std::sync only) |
| Runtime trait-object dispatch | ✅ for pluggable backends | ❌ no for compile-time-resolved identities |
| Allocation | ✅ at the boundary | ❌ minimized on the resolved hot path |

The single question that decides any case: **how often is this paid —
once per firewall crossing, or once per inner operation?** Boundary →
fine. Hot path → forbidden.

**Canonical worked example — HIPAA (see §7).** Healthcare privacy is
the textbook firewall case: it demands *both* ultra-fast inner access
control (every PHI field access is authorized → must be a hot-path
bit-op, not a serialized lookup) *and* a durable immutable audit trail
(every access logged → a boundary write). Inner = palette256 +
Hamming-popcount row-level auth (no serialization); outer = audit-as-
Lance-version append (serialized, at the firewall). The audit signature
is the literal "crypto on the post stamp." A production HIPAA instance exercises both sides — detail in §7.2.

## 4. Where every existing piece sits

| Component | Side | Serializes? | Notes |
|---|---|---|---|
| `ogar-vocab` (`Class` / `ActionDef` / `ActionInvocation`) | inner IR | no (the structs) | `serde` derives feature-gated for boundary use only |
| Compile-time HHTL identity resolution | **inner** | **no** | const/typestate; build-time codegen target (to build) |
| Rubicon `on_event` / `evaluate_guard` | **inner hot** | **no** | pure dispatch over compile-time-resolved state |
| `CommitHook::on_commit` | firewall edge | **yes** (writes Lance row) | sync + fallible (I-2); the write IS the firewall crossing |
| `LanceMembrane::commit_event(row) -> u64` | **outer** | yes | action-commit firewall crossing |
| `ogar-knowable-from::KnowableFromStore` | **outer** | yes (registry write) | the §10.3 producer seam; trait = SDK seam |
| `ExternalMembrane` impls (Redis / SeaORM / Postgres) | **outer** | yes | live at callcenter + contract; LazyLock fallback |
| `lance-graph-planner::temporal::classify` | inner (consumes) | no | takes `row_version` + `knowable_from` as resolved values |
| intra-process actor messaging | **inner hot** | **no** | `Arc<RecordBatch>` zero-copy |
| cross-instance actor messaging | firewall crossing | yes | RecordBatch IPC encode = boundary tax (see §5) |
| `ogar-adapter-surrealql::emit_surrealql_ddl` | outer (build/boundary) | yes (produces DDL string) | DDL emission is a boundary artifact |
| `ogar-adapter-surrealql::parse_surrealql_ddl` | outer (boundary) | yes (parses wire DDL) | parse is a firewall crossing |

## 5. Resolving the `SOA-IMPLEMENTATION §5.3` apparent tension

`SOA-IMPLEMENTATION.md §5.3` says "Inter-actor wire form is RecordBatch
IPC — N actions = 1 batch." Read naively against "no serialization in
hot path," that looks like a contradiction. The Firewall resolves it:

- **Intra-process** (the default, the hot path): actors share
  `Arc<RecordBatch>` by reference — **zero-copy, no serialization**.
  The "batch" is a shared in-memory columnar buffer; passing it between
  actors is an `Arc` clone, not an encode.
- **Cross-instance** (a firewall crossing): RecordBatch IPC *encode*
  applies — but that's a boundary crossing (different process / server),
  so the serialization is legitimate boundary tax, paid once.

The "IPC" framing in §5.3 is therefore **cross-instance only**. The
intra-process hot path is zero-copy `Arc` passing. `SOA-IMPLEMENTATION`
should be read with this clarification (and will be cross-referenced to
this doc).

## 6. Carve-outs (absolute — violations are bugs, not trade-offs)

1. **No serialization on the hot path.** No `serde`, no JSON, no Arrow
   IPC encode/decode, no to-bytes/from-bytes per inner operation. The
   firewall is absolute.
2. **Inner identity resolution is compile-time HHTL.** Known classes
   resolve via const/typestate generated at build time, not runtime
   hashmaps or runtime trait calls.
3. **Pluggability lives at the outer boundary, via contract traits.**
   `ExternalMembrane` / `KnowableFromStore` and friends are firewall
   interfaces. No `dyn` dispatch for compile-time-resolved identities
   on the hot path.
4. **Boundary tax is paid once per crossing, then cached.** Unknown
   schema → read once → `LazyLock` → resolved thereafter. Crypto /
   signing → at the boundary. Never re-paid on the hot path.
5. **`serde` derives stay feature-gated.** Available at the boundary;
   never a hot-path dependency.
6. **The Firewall is the umbrella for I-2 + BBB.** I-2 (no tokio on the
   hot loop) and BBB (Arrow-scalar on the hot loop) are facets of the
   same principle: the hot path is native, synchronous, zero-copy,
   serialization-free. New facets (this doc's "no serialization") join
   the same umbrella.

## 7. Precedent + cross-references

### 7.1 OGAR domain instances (named by domain — per the "inherit schema via contract" rule)

These aren't just "they have the membrane pattern" — they are **OGAR
domain instances**: production-grade transcodes the substrate (and this
firewall principle) generalize. Named by *domain*, not project — the
concrete label is consumer-rebindable via the `Adapter` contract (see
`docs/DOMAIN-INSTANCES.md §0`), and a deployment's PII labels never enter
OGAR's surface. Full catalogue there.

- **A production ERP deployment** = **OGAR for Odoo / ERP** (the
  `ogit-erp::` prefix made real): Odoo models → `Class`, `@api.depends`
  → `KausalSpec::Depends`, the ERP money/decimal model. The production
  Odoo instance behind `docs/ODOO-TRANSCODING.md`.
- **A production healthcare deployment** = **OGAR for HIPAA /
  healthcare**: the `ExternalMembrane` + `LazyLock` outer-boundary
  pattern, exercising the substrate's **Security Mesh** (row-level
  permissions + immutable audit). PII field labels stay consumer-side
  (§7.2 + `DOMAIN-INSTANCES.md §0`).

Both demonstrate the external-membrane-via-contract pattern in
production; the firewall principle generalizes what they already do.

### 7.2 a production HIPAA instance / HIPAA — the canonical firewall demonstration

Healthcare privacy demands two things that map *exactly* to the
firewall's two sides, and HIPAA can't compromise either:

| HIPAA requirement | Firewall side | Mechanism | Serialization? |
|---|---|---|---|
| **Minimum-necessary access control** — every PHI field access authorized | **inner / hot** | palette256 `_effectiveReaders` bitmap + Hamming-popcount bit-intersection (the parity matrix's "Security Mesh, shape-exact") | **none** — a bit-op, checked per access, must be serialization-free |
| **Immutable audit trail** — who-accessed-what-when, tamper-evident | **outer / firewall** | audit-as-Lance-version append (the audit-log ↔ Lance-version consolidation) | **yes** — serialized + signed, once per access crossing |

The tension HIPAA creates is the exact tension the firewall resolves:
auth must be *fast* (it gates every PHI field read — if it serialized,
PHI-heavy queries would crawl) **and** audit must be *durable +
tamper-evident* (it's a legal requirement — must be written + signed).
Putting auth on the hot path as a bit-op and audit at the firewall as a
signed Lance append is the only way to have both. The audit record's
signature is the literal "crypto on the post stamp" (§3) — expensive,
acceptable, paid once per access crossing, never on the inner compute.

**A production HIPAA deployment is the proof** that the firewall split
isn't theoretical: a real HIPAA-compliant healthcare system needs
precisely this inner-auth / outer-audit separation, and it ships.

### 7.3 Cross-references

- `docs/DOMAIN-INSTANCES.md` — the full catalog of OGAR domain
  instances (a production ERP instance/Odoo + a production HIPAA instance/HIPAA + the chess/OP/HIRO
  calibration set) mapped to substrate capabilities.
- `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` ADR-022 — the decision record.
- `docs/SUBSTRATE-ENDGAME.md` §5 — the SDK seam the outer boundary enables.
- `docs/SURREAL-AST-AS-ADAPTER.md` — the structural/behavioral split (inside the firewall).
- `docs/SOA-IMPLEMENTATION.md` §5.3 — the RecordBatch-IPC clarification (see §5 here).
- `docs/ODOO-TRANSCODING.md` — the Odoo transcoding spec (its production instance is an ERP deployment).
- `lance-graph-contract::ExternalMembrane` — the existing outer-boundary contract trait.
- `crates/ogar-knowable-from` — the §10.3 `KnowableFromStore` outer-boundary seam.

### OGAR / substrate cross-references
- `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` ADR-022 — the decision record.
- `docs/SUBSTRATE-ENDGAME.md` §5 — the SDK seam the outer boundary enables (Foundry-OSS-class pluggability).
- `docs/SURREAL-AST-AS-ADAPTER.md` — the structural/behavioral split (inside the firewall).
- `docs/SOA-IMPLEMENTATION.md` §5.3 — the RecordBatch-IPC clarification (see §5 here).
- `docs/OGAR-AST-CONTRACT.md` §5 — the anti-mess carve-outs (hot-path std::sync per I-2).
- `lance-graph-contract::ExternalMembrane` — the existing outer-boundary contract trait.
- `crates/ogar-knowable-from` — the §10.3 `KnowableFromStore` outer-boundary seam.

## 8. Doc lifecycle

- **Author:** OGAR session, 2026-06-05.
- **Status:** Absolute invariant; v0.
- **Change policy:** the Firewall is absolute. Weakening it (allowing
  any serialization on the hot path) requires an explicit superseding
  ADR with a named, measured justification — not incremental erosion.
  New *facets* (additional things forbidden on the hot path) can be
  added as amendments.
- **Enforcement aspiration:** a future lint / CI check (e.g. deny
  `serde::Serialize` calls reachable from hot-path entry points) would
  make the firewall mechanically enforced rather than convention. Noted
  as a follow-up; not built.
