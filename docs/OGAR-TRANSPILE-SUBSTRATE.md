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

The two modes are not rivals: (a) is the live "pull a class and render it"
path; (b) is the "materialise the class as source for a build target" path.
Both consume the same `CompiledClass`.

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
- **`ogar-from-ruff::emit`** — the pull-back **codegen** leg, reference
  target Rust: `emit_rust(&CompiledClass) -> String` renders a rail struct
  whose fields use the consumer's wrapper-contract types (`OgScalar` /
  `ToOne<T>` / `ToMany<T>`) + a `*_CLASSID` const (OGAR #132).
  `account.move → struct AccountMove { name: OgScalar, partner_id:
  ToOne<ResPartner>, line_ids: ToMany<AccountMoveLine> }`.

**Next (the transpiler direction):**
1. **Pull-back emit, breadth + depth** — `emit_csharp` / `emit_python`
   targets on the same `&CompiledClass -> String` seam; refine `OgScalar`
   to mapped concrete types once the `field_type` capture lands (ruff
   follow-up); extract the family into dedicated `ogar-emit-<lang>` crates
   mirroring `ogar-adapter-surrealql`. The runtime wrapper-contract mode
   (lance-graph-contract for Rust) is the C#/Python sibling.
2. **Thin the consumer** — `odoo-rs` collapses to a `compile_graph::<OdooPort>`
   caller + the `od-posting` GoBD adapter (the 15%).
3. **Scale** — run the `odoo_blueprint` 404 entities through `compile_graph`;
   over-cap god-models (`≥ 256` members) branch via the SoC lint
   (`ruff_spo_address::soc`), never widen.

---

## 7. For a future session — how to extend (the four moves)

| To add… | Do this | Convergence is… |
|---|---|---|
| a **source language** | a `ruff_<lang>_spo` frontend → `ModelGraph`; reuse `lift` + `mint` | automatic (shared IR) |
| a **target language** | an `ogar-emit-<lang>` adapter (`CompiledClass → String`) **or** a thin runtime wrapper contract (traits mirroring `lance-graph-contract`) | the consumer reimplements nothing |
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
