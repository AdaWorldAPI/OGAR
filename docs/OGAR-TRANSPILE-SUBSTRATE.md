# OGAR as the Per-Class Transpile Substrate

> **Read this to understand the power.** OGAR is not "a codebook" or "a DTO
> store" — it is a **bidirectional transpiler** whose unit of currency is the
> *per-class, rail-shaped, language-agnostic compiled class*. This doc names
> the whole machine: how a class is pulled IN from any source language, minted
> into a rail address, and pulled BACK into any consumer language through a
> thin wrapper contract. Companion to `OGAR-AS-IR.md` (the compiler framing)
> and the `#133` handover (the ERP/planning landing plan).

---

## 0. The power in one paragraph

OGAR compiles business logic from any source language (Python/Odoo,
Ruby/Rails, C#, …) into **per-class compiled classes**, each addressed by a
16-byte **rail facet** whose `classid` is a cross-app join key. ~**85 %** of a
consumer's logic — the mechanical, data-shaped part (fields, relations,
computed values, validations, the schema) — lives in OGAR as these minted
classes. A consumer in **any** language pulls a class back through a thin
**wrapper contract** (`lance-graph-contract` is Rust's) and reimplements
**nothing**. The **"impossible" 15 %** — intrusive, stateful, or
genuinely-language-specific logic — is a small per-language **adapter +
ClassView + ontological grounding**. One canonical class, N languages,
cross-app convergence, **at the cost of an import.** "ERP, OpenProject, …
for everything" falls out of the codebook, not out of per-app reimplementation.

---

## 1. The two legs (the transpiler is bidirectional)

```
                          ┌─────────────────────── OGAR substrate ───────────────────────┐
 PULL-IN                  │                                                                │   PULL-BACK
 (source → OGAR)          │   ModelGraph ──lift──►  ogar_vocab::Class     (the schema)     │   (OGAR → language)
                          │       │        ──mint──►  Facet (16B rail addr) (the address)  │
 Python/Odoo ─┐           │       │                                                        │   ┌─► Rust  : import lance-graph-contract
   ruff_python_spo        │       └───────────────►  CompiledClass { class, facet }  ──────┼───┤   (ClassView + FacetCascade, runtime)
 Ruby/Rails  ─┤  ruff_*   │                                  ▲                             │   ├─► C#    : a thin C# wrapper contract
   ruff_ruby_spo  ──IR──► │                                  │ pulled by classid           │   ├─► Python: a thin Python wrapper contract
 C#/…       ─┘  (shared)  │                                  │                             │   └─► any DDL/codegen: ogar-adapter-*
                          └────────────────────────────────────────────────────────────────┘
```

**Pull-in** — `source → ogar-from-<lang> → ModelGraph → lift + mint → CompiledClass`:

| step | crate / fn | output |
|---|---|---|
| parse | `ruff_python_spo` / `ruff_ruby_spo` (ruff frontends) | `ruff_spo_triplet::ModelGraph` (the shared, language-neutral IR) |
| schema | `ogar-from-ruff::lift_model_graph_python` (+ `..._ruby`) | `Vec<ogar_vocab::Class>` — attributes / associations / computed_fields |
| address | `ogar-from-ruff::mint::mint_graph<P>` | `ruff_spo_address::Mint` — a 16-byte `Facet` per node |
| compile | `ogar-from-ruff::mint::compile_graph_python<P>` | `Vec<CompiledClass { class, facet }>` — **the unit a consumer pulls** |

**Pull-back** — a consumer obtains a `CompiledClass` by either:

- **(a) runtime wrapper contract** — the consumer imports a thin contract and
  resolves the class by `classid` at runtime. `lance-graph-contract` is the
  Rust contract: `ClassView` (the schema/render surface), `FacetCascade` (the
  16-byte address), `ActionDef`/`KausalSpec` (behaviour). No codegen — the
  class is *data* interpreted through the contract's traits. A C#/Python
  consumer ships an analogous thin contract.
- **(b) codegen emit adapter** — OGAR emits the class as target-language
  source/DDL. `ogar-adapter-surrealql` (`Class → SurrealQL DDL`) is the
  reference emitter; per-language emitters (`ogar-emit-rust`, …) follow the
  same `CompiledClass → String` seam.

These two modes are **not co-equal** (a correction to an earlier framing).
Mode (a), the **compiled `ClassView`**, is the **spine / hot path**: it sinks
into OGAR and is **compiled into the binary** — no parse, no serialization in
the hot path (ADR-022/023). Mode (b), SurrealQL emit, is a **storage-membrane
adapter** (DDL for the SurrealDB boundary); **parsing SurrealQL back is slow —
even JIT'd it loses to compiled code**, so it is never the hot path. See
§1.5 — it is the load-bearing idea of the whole substrate.

---

## 1.5 The spine is the COMPILED `ClassView` (not SurrealQL)

> **Operator, 2026-06-29.** *"The [facet] versions are useless because the
> ClassView can do recombinations of all of them while sinking into OGAR and
> getting COMPILED into binary, and NOT parsed from SurrealQL — which even with
> JIT will be slow."*

This is the heart of the power. A `ClassView` is not a parser and not a fixed
record format — it is a **compiled, flexible, composable reader** over the
facet, baked into the binary. Three properties:

### a. One ClassView *rotates* the facet layout — no "versions"

The 16-byte facet's tier payload is not locked to a single carving. A ClassView
can **always rotate** — read the SAME 12 cascade bytes under a different
grouping — to fit the class. The carvings (pinned as
`lance_graph_contract::facet::CascadeShape`, `CascadeShape::ROTATIONS`):

```
6× (1:2)        ALIGNED default — 6 tiers, each a 1:2 hierarchy   (group_of = i >> 1, a shift)
3× (1:2:3:4)    ALIGNED default — 3 tier-pairs, each 1:2:3:4      (group_of = i >> 2, a shift)
4× (1:2:3)      WORST CASE      — straddles tier boundaries        (group_of = i / 3, a DIVIDE)
```

**Only the byte-aligned carvings are defaults.** `6×(1:2)` and `3×(1:2:3:4)`
keep `group_of` a pure shift (the canon's "tier-of-level is a shift, never a
branch"). **`4×(1:2:3)` is the worst case, not a co-equal carving** — it
straddles tier boundaries so `group_of` must DIVIDE (`CascadeShape::is_byte_aligned()`
is `false`, `shift()` is `None`). It is *prevented on the common path* and kept
only as the **rare rotation / escape hatch**: a ClassView may rotate to it
deliberately when a rare class (some Odoo models) needs to relieve
**classid-stacking entropy** — rotate the reading rather than mint another
classid. So there is **no need for hardcoded facet "versions" (V1/V2/V3)** — one
compiled ClassView subsumes the rotation set; the straddle stays legal only as a
deliberate, rare rotation. Hardcoding a format per version is the thing to
*delete*.

> **Carvings address the VIEW, never the functions.** A rotation re-reads the
> data layout; it does NOT reach behaviour. Functions are encoded by the
> **classid acting as an additional switch** — `lance_graph_contract::facet::ClassArm`
> `{ View, Functions }`, the OGAR THINK/DO split (`OGAR-AST-CONTRACT.md`).
> Reaching a function = switch the classid to the `Functions` arm (the
> `ActionDef`/`KausalSpec` on the resolved Core node), *never* slice the
> tier-bytes. A straddling carve to "get to" a function is exactly the worst
> case the `4×(1:2:3)` example warns against.

### b. Sub-range mapping + nested ClassViews stacked into constructors

The carving need not be uniform. With `6×(1:2)` over 12 fields it is sometimes
more efficient to **map a sub-range — e.g. `1..3` of the 12 — as its own
hierarchy**, and to **stack *nested* ClassViews into constructors** rather than
read one flat layout. A ClassView composes sub-ClassViews; the composition is
built by **constructors compiled into the binary** (the `emit_rust` direction),
never re-derived by parsing a DDL string. Nesting = composition of compiled
readers, not a runtime interpreter.

### c. Lazy, reused materialization of the `32×GUID` SoA

The nested ClassView constructors run **before materializing the `32×`(hex)
GUID struct-of-arrays**, and the SoA is materialized **lazily and lazy-lock
reused** (`LazyLock`-style: build once on first touch, share thereafter). The
key prerenders nodes with zero value-decode (canon: "THE GUID IS THE KEY OF
KEY-VALUE"); the compiled ClassView lays them out from keys alone, and the
heavier value-SoA is only built when actually needed, then cached.

**The `32×GUID` is literal capacity, and it sets the layout doctrine: clean /
SoC over packed** (operator, 2026-06-29). A 512-byte node is exactly
`512 / 16 = 32` sixteen-byte GUID-sized slots (`key` + `edges` take 2; the
480-byte value slab is the remaining 30) — pinned as
`lance_graph_contract::canonical_node::GUIDS_PER_NODE = 32` (compile-time
asserted). So in the worst case you **Tetris whatever you need across the
slots** — give each concern its own clean slot — rather than bit-pack two
concerns into one. Packing into a single facet via a straddle (`CascadeShape::G4D3`,
§1.5a) is the dispreferred last resort precisely *because* there are 32 slots:
the capacity is what makes separation-of-concerns the default and the straddle
unnecessary. (It is also the headroom behind §1.5a's "rotate / spread to a fresh
slot instead of minting another classid" for the rare classid-stacking-entropy
case.)

### Why this is the power (and where SurrealQL sits)

- **Compiled beats parsed.** The business logic is a `ClassView` compiled into
  the consumer's binary — branch-predictable, inlinable, zero-parse. A
  SurrealQL DDL round-trip (`ogar-adapter-surrealql`) is a *storage-membrane*
  adapter for the SurrealDB boundary; **even a JIT over SurrealQL loses to
  compiled code**, so it is never on the hot path.
- **Consequence for this repo's roadmap:** the SurrealQL emit/parse work
  (#136 Stage A, and the od-ontology Stage B/C fork-deletion) is *membrane*
  work — correct for the storage boundary, but **not the spine**. The spine
  investment is the compiled, nested, lazy `ClassView` over the GUID SoA. When
  the two compete for attention, the compiled ClassView wins.

> **Status:** the rotation *principle* + the nested-constructor + lazy-SoA
> architecture are operator-specified here as the durable design. The exact
> tier-byte arithmetic is **now pinned + implemented** as
> `lance_graph_contract::facet::CascadeShape` (`G6D2` / `G4D3` / `G3D4`,
> `G·D = CASCADE_UNITS = 12`) over `FacetCascade::tier_bytes()` — `index(g,l) =
> g·D + l`, `group_of`/`level_of` inverses, `cascade_byte`, per-group LCP
> `cascade_group_shared`. `CascadeShape::ALIGNED = [G3D4, G6D2]` are the
> shift-`group_of` **defaults** (`shift()` is `Some`); `CascadeShape::ROTATIONS`
> is the full rotation set a ClassView may rotate through. **`G4D3` is the worst
> case** — `is_byte_aligned()` is `false`, `shift()` is `None`, `group_of`
> divides — excluded from `ALIGNED`, kept in `ROTATIONS` only as the rare
> escape-hatch rotation (classid-stacking-entropy relief). **Functions are NOT a
> carving** — `facet::ClassArm { View, Functions }` is the classid's additional
> THINK/DO switch; carvings address `View` only. Zero-dep, `const fn`,
> probe-verified (lance-graph #621). One algebra for both the facet bytes and a
> 12-field class — the shared substrate the three language SDKs (§1.6) all read.

---

## 1.6 Three SDKs, one compiled spine (Rust · C# · Python)

> **Operator, 2026-06-29.** *"For Rust via lance-graph; for Python and C# we
> need sort of an 'SDK' that does that for the others."*

Rust's wrapper contract is `lance-graph-contract` — the consumer `import`s it
and pulls a `CompiledClass` by `classid`. Python and C# need the **same
capability**, packaged as a thin per-language **SDK**. The crucial constraint
(§1.5): the spine is the **compiled `ClassView`, never a SurrealQL parse**. So
an SDK is *not* a query client — it is a thin reader over the already-compiled
rail, plus a host for that language's 15 % adapter.

### What every SDK is (three thin layers, nothing more)

| layer | what it is | Rust (`lance-graph-contract`) | C# SDK | Python SDK |
|---|---|---|---|---|
| **1. address algebra** | the 16-byte facet + the cascade carving math — *byte-identical across languages* | `FacetCascade` + `CascadeShape` (`const fn`, zero-dep) | `readonly struct FacetCascade` (`[StructLayout(Sequential, Size=16)]`) + `enum CascadeShape` | `Facet` (a 16-byte `bytes` view) + `CascadeShape` (`IntEnum`) |
| **2. ClassView reader** | present a pulled `CompiledClass` as native schema objects (fields / relations / computed), grouped by the chosen carving | `ClassView` traits | `IClassView` / records | `@dataclass` ClassView |
| **3. adapter host** | the per-language hook where the 15 % hand-written logic registers, + the late `classid → ClassView → OGIT` grounding resolve | `ActionDef`/`KausalSpec` impls | interface + DI | ABC + registry |

Layer 1 is the whole reason an SDK can be *thin*: the cascade algebra is **~80
lines of `core`-only `const fn`** (`CascadeShape::{groups,levels,index,
group_of,level_of}` + `FacetCascade::{tier_bytes,cascade_byte,
cascade_group_shared}`). Nothing in it is Rust-specific — it is integer
shifts/divides over a 16-byte array. Porting it to C# or Python is a
**mechanical transliteration**, and the bytes it reads are produced once by the
OGAR mint, so all three SDKs agree by construction (the same cross-crate
round-trip probe that pins `ruff_spo_address::Facet ≡ FacetCascade` extends to
"≡ the C#/Python `FacetCascade`").

### How a class reaches each language (mirrors §1's two pull-back modes)

- **(b) codegen emit — the strongest "compiled" form per host.** OGAR emits
  native source on the **same `&CompiledClass -> String` seam** as `emit_rust`:
  - `emit_csharp` → a C# `record`/class compiled by the host into an assembly
    (truly compiled, like the Rust struct).
  - `emit_python` → a `@dataclass` imported as a module (compiled to bytecode by
    CPython; the "cost of an import" literally).
  There is **no OGAR-runtime parse** in this mode — the class IS native source
  the host toolchain compiles. This is the preferred mode and the direct analog
  of "sinks into OGAR and gets compiled into the binary."
- **(a) runtime reader — the thin SDK over the rail artifact.** When codegen is
  undesirable (dynamic discovery, late binding), the SDK loads the **compiled
  rail artifact** (the facet SoA — the 16-byte-per-class bytes themselves, which
  ARE the format) and reads it zero-parse via layer 1. It never loads SurrealQL;
  the artifact is the facet bytes, not DDL.

Either way the 85 % logic stays in OGAR; the SDK carries layer-1 (tiny,
portable) + layer-3 (the language's own 15 %). That is what makes "one
canonical class, N languages, at the cost of an import" concrete for C# and
Python and not just Rust.

### Why this respects every iron rule

- **Compiled, not parsed** (§1.5): codegen emits host-native source; the runtime
  reader is zero-parse over the facet bytes. SurrealQL is never on an SDK path.
- **Pull, never re-mint** (§7): an SDK *reads* a `CompiledClass`; it never owns a
  codebook copy or constructs a bridge — the classid resolution is the mint's.
- **Resolve, don't store** (§4): grounding is layer-3's late `classid → OGIT`
  resolve, identical in all three languages; no SDK copies FIBO/DOLCE onto rows.
- **One algebra** (§1.5): layer 1 is the *same* `CascadeShape` carving in every
  language — the SDKs cannot drift on layout because they share the byte format.

---

## 2. Why it is a *substrate*, not a dump — the addressing

A minted class is **addressable without decoding its value**. That is the
whole power: a renderer/router/planner lays out, groups, and skeleton-renders
classes from the 16-byte key alone.

### 2.1 The classid (the cross-app join key)

```
render classid (u32) = (APP_PREFIX as u32) << 16 | concept (u16)
                         └── high u16 ──┘         └── low u16 ──┘
                         the app RENDER skin       the SHARED concept
```

- **low u16 = the shared concept** — resolved by `PortSpec::class_id` against
  the OGAR codebook (`ogar_vocab::class_ids` / `ogar_codebook`). This is the
  de-facto `owl:equivalentClass` expressed as a u16.
- **high u16 = the app render skin** — `PortSpec::APP_PREFIX`. Picks the
  per-app `ClassView` / template; **carries no behaviour**.
- composed by the canonical `ogar_vocab::app::render_classid_for::<P>(concept)`.

**Cross-app convergence is the payoff.** The same concept across apps gets the
same low u16, so a consumer joins across apps with a `==`:

| concept | id | Odoo (`0x0002`) | OpenProject (`0x0001`) | Redmine (`0x0007`) | WoA/SMB (`0x0003`/`0x0004`) |
|---|---|---|---|---|---|
| `commercial_document` | `0x0202` | `account.move`, `sale.order` | — | — | `Rechnung`/`Invoice`/`Vorgang` |
| `project_work_item` | `0x0102` | — | `WorkPackage` | `Issue` | — |
| `billable_work_entry` | `0x0103` | `account.analytic.line` | `TimeEntry` | `TimeEntry` | `Stundenzettel` |

`billable_work_entry` is the **named first cross-domain bridge**: a logged
unit of work is one concept whether it arrives from the planning arm
(OpenProject) or the commerce arm (Odoo). Pinned in
`ogar_vocab::ports::tests::billable_work_entry_converges_across_all_five_ports`.

### 2.2 The 16-byte rail facet (`FacetCascade` — the "V3" schema)

```
facet_classid : u32         rows 0      ← the classid above
tiers[0..6]   : FacetTier    rows 1-3   ← 6× (lo:hi) = (is_a : part_of) byte pairs
                                            hi_chain() = part_of cascade (containment)
                                            lo_chain() = is_a    cascade (inheritance)
```

- 16 bytes, content-blind, SIMD-transpose-native. `ruff_spo_address::Facet`
  and `lance_graph_contract::facet::FacetCascade` are **byte-identical** —
  the mint's bytes round-trip losslessly into the Foundry's facet (proven by
  the cross-crate round-trip probe).
- `prefix_distance()` = `8 − shared_prefix_tiles()` → O(1) hierarchy distance,
  no value decode.
- The mint builds two forests from the SPO triples — `part_of` (inverted
  `has_field`/`has_function`) and `is_a` (`inherits_from`, fallback
  `rdf:type`) — and stamps each node's coarse→fine rank chains.

### 2.3 Compression never costs addressability

A node is `key(128/GUID) + value`. Lance may compress the value arbitrarily
(columnar, dictionary, PQ); the key is never compressed and never needs the
value decoded to route. (Canon: "THE GUID IS THE KEY OF KEY-VALUE.")

---

## 3. The 85 / 15 split (the consumer model)

> The operator's framing: *"in case of lance-graph we would still pull odoo-rs
> but 85 % would be in the OGAR transpile substrate, and the transcode then is
> just a generic compiler-store caller with some adapters."*

- **85 % — mechanical, minted into OGAR.** Fields, relations, computed values,
  validations, the schema. The consumer is a **thin compiler-store caller**:
  `compile_graph::<P>(graph)` → pull `CompiledClass` → render via `ClassView`.
- **15 % — "impossible", a per-language adapter.** Intrusive / stateful /
  truly language-specific logic that does not fit a clean mold becomes a
  **custom adapter + ClassView + ontological grounding**. Worked example:
  `odoo-rs`'s `od-posting` — the GoBD double-entry posting host (gapless
  Belegnummer + inalterability hash chain) — stays a hand-written Rust adapter;
  *everything else* of `account.move` is minted.

This is the **Core-First Transcode Doctrine** (`.claude/knowledge` /
`core-first-transcode-doctrine.md`) restated for consumers: mechanical leaf
methods → thin `classid`-keyed adapters that **assume the Core**; intrusive
methods → hand-port; a Core gap → **extend the Core deliberately**, never hack
the adapter.

### What a thinned consumer looks like (the #2 target)

```
odoo-rs  (today)                         odoo-rs  (thinned)
─────────────────                        ──────────────────
od-ontology   bespoke Schema+triples     od-ontology   = a compile_graph::<OdooPort> caller
schema_to_classes → DDL                                  (pulls CompiledClass from the substrate)
od-posting    GoBD logic                 od-posting    = unchanged (the 15% adapter)
alignment     FIBO/DOLCE seed            grounding     = resolved late via classid → ClassView → OGIT
                                         + a thin wrapper contract (lance-graph-contract)
```

The consumer shrinks to **(import the substrate) + (a compile_graph call) +
(the GoBD adapter) + (a wrapper contract)**. That is the "cost of an import."

---

## 4. Grounding: resolve, don't store

FIBO / DOLCE / OGIT grounding is **not** stored on the facet or the codebook
rows. The contract is deliberate (`lance-graph-contract`): *"the meta-DTO
resolves; it does not store."* Grounding is resolved **late** via
`classid → ClassView → OGIT registry`:

- the OGIT hydrator inheritance chain `odoo → fibo-fnd → dolce`
  (`lance-graph-ontology::hydrators`), plus `classify_odoo(model)` → DOLCE
  category (e.g. `account.move` → `Perdurant`);
- the FIBO pivot (`account.move` ⇒ `fibo:Transaction`) lives in
  `od-ontology::alignment::ODOO_SEED` + `odoo-to-fibo.ttl`.

So **"retain grounding" = keep the `classid` correct** (never ship `0` outside
the bootstrap address). A 16-byte facet suffices for a richly-grounded class
because the grounding is one resolve away, not copied onto every row. (This is
why the `#133` handover's "add `{ogit_uri, dolce_category, fibo_equivalent}`
to codebook rows" gap was **declined** — it fights this design.)

---

## 5. Worked example — `account.move`

```
account.move (Odoo Python)
  └─ ruff_python_spo ─► ModelGraph { ns: "odoo", model "account_move" }
       fields: name(Char), partner_id(Many2one res.partner),
               line_ids(One2many account.move.line, inverse move_id),
               amount_total(Monetary, compute=_compute_amount, depends line_ids.balance)
  ├─ lift_model_graph_python ─► Class "account_move"
  │     attributes:      [name]
  │     associations:    [partner_id → BelongsTo res.partner,
  │                       line_ids   → HasMany account.move.line (inverse move_id)]
  │     computed_fields: [amount_total ← _compute_amount, depends [line_ids.balance]]
  └─ mint_graph::<OdooPort> ─► Facet
        facet_classid = 0x0002_0202   (Odoo 0x0002 | commercial_document 0x0202)
        ⇒ CompiledClass { class, facet }

  Cross-checks (all probe-verified):
    • facet.to_bytes() ≡ lance_graph_contract::FacetCascade(0x0002_0202)   (byte-exact)
    • canonical_concept_domain(0x0202) == Commerce                          (routes to the right ClassView)
    • grounding resolvable: 0x0202 → fibo:Transaction / DOLCE Perdurant     (late, via OGIT)
    • the GoBD posting (account.move._post) stays od-posting's Rust adapter  (the 15%)
```

The **relation-aware** shape (`Many2one` vs `Many2many` vs `One2many`) is only
correct because of the `relation_kind` predicate (ruff#35): `target` +
`inverse_name` alone cannot separate a Many2one from a Many2many.

---

## 6. What is built / what is next

**Built (this arc):**
- `ruff_python_spo` — Odoo/Python SPO frontend (ruff #34).
- `relation_kind` predicate — Many2one/One2many/Many2many cardinality (ruff #35).
- `lift_model_graph_python` + the `project_odoo_fields` schema projection (OGAR #131/#132).
- **`ogar-from-ruff::mint`** — per-class minting: `mint_graph<P>`,
  `CompiledClass`, `compile_graph_python<P>` (OGAR #132).
- **`ogar-from-ruff::emit`** — the pull-back **codegen** leg, now **three
  emitters on one `&CompiledClass -> String` seam** — the codegen mode of the
  per-language SDKs (§1.6):
  - `emit_rust` (reference, OGAR #132) → `pub struct AccountMove { name:
    OgScalar, partner_id: ToOne<ResPartner>, line_ids: ToMany<AccountMoveLine> }`
    + `ACCOUNT_MOVE_CLASSID`.
  - `emit_csharp` → `public sealed record AccountMove { public const uint
    ClassId = 0x00020202; public OgScalar name {get;init;} public
    ToOne<ResPartner> partner_id {…} public ToMany<AccountMoveLine> line_ids {…} }`.
  - `emit_python` → `@dataclass class AccountMove: CLASSID: ClassVar[int] =
    0x00020202; name: OgScalar; partner_id: ToOne["ResPartner"]; line_ids:
    ToMany["AccountMoveLine"]`.

  All three use the **same wrapper-contract type names** (`OgScalar` / `ToOne` /
  `ToMany`); only the bracket syntax differs (`<T>` Rust/C#, `[T]` Python) — a
  shared `assoc_target` relation classifier drives all three (the mechanical
  transliteration §1.6 promises). The `classid` travels with the class. The
  compute behaviour stays a trailing comment (the 15% adapter). 6 emit tests
  (incl. `all_three_emitters_share_the_same_type_vocabulary`), clippy clean
  (probe-verified offline). (C#/Python: this PR.)

- **`ogar-adapter-surrealql` `array<record>`** — to-many associations
  (`HasMany`/`HasAndBelongsToMany`) emit as `array<record<comodel>>` (OGAR #136,
  #2 Stage A). **Membrane work** (the SurrealDB storage boundary) — *not* the
  spine (§1.5).

**Next (priority order — spine first, per §1.5):**
1. **The compiled `ClassView` spine (THE priority)** — the flexible,
   *nested*, lazy reader: facet-layout recombination (`6×(1:2)` / `4×(1:2:3)` /
   `3×(1:2:3:4)`), sub-range hierarchy mapping, nested ClassViews stacked into
   compiled constructors, and lazy + lazy-lock-reused materialization of the
   `32×GUID` SoA. Pin the per-carving tier-byte arithmetic against
   `FacetCascade` first (don't guess). This subsumes hardcoded facet
   "versions". The `emit_rust` codegen leg is the start; this is the depth.
2. **Pull-back breadth — the C# / Python SDKs (§1.6).** Codegen mode is
   **shipped** (`emit_csharp` / `emit_python`). Remaining: (a) the **thin
   runtime SDKs** — layer-1 `FacetCascade` + `CascadeShape` transliterated to
   C#/Python (mechanical now that `CascadeShape` is pinned in
   `lance-graph-contract`), layer-2 ClassView reader, layer-3 adapter host; and
   (b) the wrapper-contract *packages* themselves (the `OgScalar`/`ToOne`/`ToMany`
   aliases each language ships, the analog of `lance-graph-contract`). Refine
   `OgScalar` once the `field_type` capture lands (ruff follow-up).
3. **Thin the consumer (membrane)** — `odoo-rs` → `compile_graph::<OdooPort>`
   caller + `od-posting` GoBD adapter; delete the native SurrealQL emit fork
   (W3.3, **CI-gated** — od-ontology pulls surrealdb). Recommended path: keep
   the corpus input, delete only the native emit, route emit through the shared
   `ogar-adapter-surrealql`. This is *membrane* — secondary to (1).
4. **Scale** — `odoo_blueprint`'s 404 entities through `compile_graph`;
   over-cap god-models (`≥ 256` members) branch via the SoC lint
   (`ruff_spo_address::soc`), never widen.

---

## 7. For a future session — how to extend (the four moves)

| To add… | Do this | Convergence is… |
|---|---|---|
| a **source language** | a `ruff_<lang>_spo` frontend → `ModelGraph`; reuse `lift` + `mint` | automatic (shared IR) |
| a **target language** | an SDK (§1.6): `ogar-emit-<lang>` codegen **or** a thin runtime SDK = layer-1 `FacetCascade`+`CascadeShape` transliteration + layer-2 ClassView reader + layer-3 adapter host (mirrors `lance-graph-contract`) | the consumer reimplements nothing |
| a **concept** | a `class_ids` codebook entry + a `PortSpec` alias | automatic across all ports that map it |
| a **port (app)** | one `impl PortSpec for FooPort` block (NAMESPACE, BRIDGE_ID, APP_PREFIX, aliases) | the app's classes get a render skin for free |

**Iron rules that bind this surface** (don't relearn the hard way):
- `classid` is **pure address**; the magic is what it resolves to. Neither u16
  half carries behaviour (`ActionDef`/`KausalSpec` is a property of the Core
  node, never the address). See `OGAR-CONSUMER-BEST-PRACTICES.md`.
- **Pull, never re-mint.** The codebook is single-source
  (`ogar_vocab::class_ids`); a consumer pulls via `*Port::class_id`, never
  copies the table or constructs a `*Bridge`.
- **SurrealQL is an adapter, not a spine.** Behaviour flows
  producer → OGAR `Class` + `ActionDef` → adapter; never producer → DDL.
- **Resolve, don't store** (grounding) — §4.
- **No serialization in the hot path** (the Firewall, ADR-022/023); the IR is
  wire-truth.

---

*Authored alongside the `ogar-from-ruff::mint` per-class minting + the
`ogar-from-ruff::emit` Rust pull-back reference (OGAR #132). Both transpile
legs — pull-in (lift + mint) and pull-back (emit) — are shipped and
probe-verified; breadth (more target languages, type-capture refinement) and
the consumer-thinning leg (§6.2) are the next deliverables.*
