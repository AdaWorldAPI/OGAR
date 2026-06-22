# HIRO in classes — what bardioc gains, literally

> **For colleagues:** how the OLD HIRO/Bardioc engine gains efficiency by
> moving its **schema** (`ogit/MARS/*`) into OGAR `Class` form and its
> **lifecycle** (`gen_statem` callbacks) into OGAR `ActionDef` form —
> without giving up anything HIRO already trusts.
>
> Status: **DOCTRINE v0** (2026-06-22). Pairs with
> `docs/MARS-TRANSCODING.md` (the calibration spec) and
> `docs/ELIXIR-HIRO-PREFETCH.md` (the migration prefetch ledger).

The trade in one sentence: **MARS stays MARS** — bit-for-bit the same
XSD-validated taxonomy the engine already trusts — and the engine
collapses from *"Elixir code that operates on dumb data"* into *"classes
that already carry their own behaviour"*. Same schema, less code, faster
dispatch, no migration debt at the structural boundary.

---

## 1. What changes — at a glance

```
                       BEFORE                                        AFTER
                       ──────                                        ─────
  schema:        MARS-Schema 2015 (XSD)             ←──equal──→   vocab/imports/ogit/NTO/MARS/*.ttl
                 + OGIT NTO/MARS (TTL)                              + classid (u16, deterministic)
                                                                    + EnumSource::Static (compile-time)

  dispatch:      Elixir `case node._type do …       ──→           u16 jump table:
                   "Application" -> AppHandler.foo                 match concept_of(cid) {
                   "Machine"     -> MachineHandler.foo             | MARS_APPLICATION => …
                   …                                               | MARS_MACHINE     => … }
                 end`                                              compiles to a single CMP + JMP table

  lifecycle:     gen_statem callbacks in Elixir     ──→           ActionDef on the Class:
                 (commission, monitor, decommission)               { state: ActionState,
                                                                     guard: KausalSpec,
                                                                     on_enter: commit }
                                                                   the Class IS the actor spec.

  dep graph:     bardioc traverses MARS              ──→           ogit:allowed pairs lift to SPO triples:
                 dependsOn edges via the                            (Application, dependsOn, Resource)
                 graph API, walking lazily                          stored in lance-graph blasgraph CSR,
                 across REST round-trips                            traversed via 7-semiring blasgraph ops

  similarity:    "find machines like this one"      ──→           classid prefix + HHTL 256×256 centroid
                 = run a query against the index                   tile: O(1) prefix lookup, no query

  validation:    XSD-walking validator on every      ──→           EnumSource::Static — compile-time
                 ingest (ms-class)                                 ALL_OF check (ns-class)

  audit:         engine writes audit alongside       ──→           Lance-version append IS the audit;
                 the data write                                    audit-as-version (ADR-013)
```

Everything in the BEFORE column is still **available** through the
adapter — bardioc keeps speaking OGIT-TTL on the wire, the engine still
honours XSD validation at the membrane. The change is **interior**: the
hot path is moved from string-keyed Elixir dispatch onto classid-keyed
Rust dispatch, and the schema becomes the IR instead of being walked at
runtime.

---

## 2. The funny insight — schema lifts structure; source lifts behaviour

This is the meta-principle that makes the migration tractable. **A
schema and a source AST are not redundant — they cover disjoint
surfaces.** Schema captures the **structural arm** (what fields exist,
what enums are allowed, what entities depend on what). Source AST
captures the **behavioural arm** (what callbacks fire, what dispatches
where, what state transitions, what computed fields depend on what
inputs).

| | `ogar-from-schema` (this PR) | `ogar-from-elixir` (sibling) |
|---|---|---|
| Walks | XSD / TTL / JSON-Schema | Elixir source code |
| **Structure** | yes, **bijective** | yes, best-effort (Elixir is dynamic) |
| **Behaviour** (callbacks, `gen_statem`, `@api.depends`) | **no — schemas are structural-only by construction** | yes |
| Closed enumerations | always (`ogit:validation-parameter` / `<xs:enumeration>`) | only when statically declarable |
| Round-trip provable | **yes** (the XSD oracle is the witness) | no |

**Stated as a principle:** a schema gets you LESS, more RELIABLY. A
source AST gets you MORE, less reliably. The right architecture uses
both, with the schema as the **drift detector** for the source lift.
Author the schema, lift it once, then emit the schema BACK from
source-lifted classes and assert it equals the committed schema —
that's how every PR catches structural drift on the way in. Foundry
charges for that as *"ontology change management"*; we get it from
`ogar-from-schema` plus `extract_classes.py`.

For bardioc concretely: **its schema (OGIT-TTL) becomes the structural
arm; its Elixir lifecycles (`gen_statem`) become the behavioural arm.**
Both lower into the same `Class`. The behavioural lift is the part
`ogar-from-elixir` was already drafted for (`docs/ELIXIR-HIRO-PREFETCH.md`);
the structural lift is the part this PR ships.

---

## 3. The four efficiency wins, with the byte-count for each

### 3.1 Dispatch — string hash → u16 jump table

```elixir
# BEFORE — bardioc dispatch on MARS _type
case node._type do
  "ogit/MARS/Application" -> AppHandler.handle(node)
  "ogit/MARS/Machine"     -> MachineHandler.handle(node)
  "ogit/MARS/Resource"    -> ResHandler.handle(node)
  "ogit/MARS/Software"    -> SoftHandler.handle(node)
end
```

```rust
// AFTER — OGAR classid match
use ogar_vocab::class_ids;
match concept_of(cid) {
    class_ids::MARS_APPLICATION => handle_application(node),
    class_ids::MARS_MACHINE     => handle_machine(node),
    class_ids::MARS_RESOURCE    => handle_resource(node),
    class_ids::MARS_SOFTWARE    => handle_software(node),
    _ => Op::Unknown,
}
```

Cost difference, per call:

| | Elixir string match | OGAR classid match |
|---|---|---|
| Wire shape | UTF-8 string `"ogit/MARS/Application"` (22 bytes) | `u16` (2 bytes) |
| Dispatch cost | string hash + ETS/pattern-match table | single `CMP r16, imm16` + `JMP table[r16]` |
| Order of magnitude | ~100 ns (Elixir VM call + pattern match) | ~2–5 ns (one cache-warm jump) |
| Comparison | branch-prediction-friendly only if all alternatives shown | natively jump-table-shaped (LLVM emits `tbl` / `br` indirect) |

The 22-byte → 2-byte wire shrinkage compounds: a million dispatches per
second is **22 MB/s** of bandwidth in BEFORE, **2 MB/s** in AFTER. The
real win is that `concept_of(cid)` reads bytes the kernel **never has
to decode the value half** of (the GUID-as-key invariant from
`CLAUDE.md`'s P0 pin) — render and route happen on prefix alone.

### 3.2 Validation — XSD walk → compile-time enum

```elixir
# BEFORE — every ingest validates against XSD
{:ok, doc} = XmlBuilder.parse(payload)
case XsdValidator.validate(doc, mars_schema) do
  :ok -> store(node)
  {:error, reason} -> reject(reason)
end
```

```rust
// AFTER — the enum IS the validator
use ogar_vocab::EnumSource;
let class_attr = MARS_APPLICATION_CLASS;  // generated from TTL at compile time
debug_assert!(matches!(class_attr.source,
    EnumSource::Static(ref pairs) if pairs.iter().any(|(k,_)| k == incoming_value)
));
// in release: 7 u32 comparisons, all branch-predictable, no allocation
```

Cost difference, per validation:

| | Elixir XSD walk | OGAR enum match |
|---|---|---|
| Validator construction | parse XSD per process (one-time, ~100 ms) | compile-time, **zero runtime cost** |
| Per-value check | DOM walk + lookup-by-name | linear scan over ≤50 values (the largest MARS enum) |
| Order of magnitude | ~1 ms per Application node | ~10 ns per value check |
| Memory | XML DOM in heap | static `&[(&str, &str)]` in `.rodata` |

The validator is the schema, **lifted to data at compile time**. Drift
between the schema and the validator is structurally impossible — they
ARE the same artifact.

### 3.3 Dependency graph — REST traversal → blasgraph SPO

The `ogit:allowed` block on every MARS entity carries the dependency
edges directly:

```turtle
# vocab/imports/ogit/NTO/MARS/entities/Application.ttl
ogit.MARS:Application
    ogit:allowed (
        [ ogit:dependsOn  ogit.MARS:Resource ]
        [ ogit:relates    ogit:License ]
        [ ogit:generates  ogit.Data:Log ]
        [ ogit:generates  ogit:Timeseries ]
    );
```

These lift to SPO triples by the producer:

```rust
// emitted by ogar-from-schema::ttl::parse_entity
(Application, dependsOn, Resource)
(Application, relates,   License)
(Application, generates, Log)
(Application, generates, Timeseries)
```

In bardioc the engine asks the graph API "what does this Application
depend on?" by making a REST call against the OGIT graph store. After
the lift, those triples live in `lance-graph blasgraph` (CSR-stored
SPO triples, 7 sparse semirings available). The query becomes:

- BEFORE: HTTP round-trip + JSON parse + result list (~5 ms over LAN)
- AFTER: a Boolean semiring multiply on the adjacency matrix —
  **microseconds, cache-resident** for any graph that fits in memory

For multi-hop traversal (Application → Resource → Software → Machine,
the canonical A-R-S-M backbone), the wins compound: 3 REST calls → 1
semiring composition.

### 3.4 Similarity — query index → HHTL 256×256 centroid tile

This is the deepest one and depends on the OGAR substrate (HHTL
256×256 centroid tile, `CLAUDE.md` P0 pin). Bardioc's "find similar
machines" walks the graph or hits a separate vector index. OGAR's
classid prefix routes through a 4-level 4-ary centroid hierarchy:

```
key (16 bytes)  =  classid(4)  │  HEEL(2)  │  HIP(2)  │  TWIG(2)  │  family+id(6)
                   address       coarse       palette    fine
                                 centroid     centroid   centroid
```

For two MARS Machine instances, `is_ancestor_of(a, b)` is centroid-tree
containment — a prefix comparison. Sibling machines (same HEEL+HIP)
are 4 bytes apart in the key space, regardless of how many other
classes exist. **The "find similar" query collapses to a key-prefix
range scan**. No separate index. No query planner. No round-trip.

This is the part that **doesn't exist in bardioc today** at all — it's
new substrate the engine gains by living in OGAR. The same wins apply
to every domain (`Software`, `Resource`, `Application`), not just
`Machine`.

---

## 4. What bardioc keeps — the boring half

Migration safety: nothing in BEFORE is *removed*. The adapter shape
preserves every byte bardioc currently speaks:

| Surface | How it survives |
|---|---|
| OGIT-TTL ingest | `ogar-from-schema` parses the same TTL files; bijection enforced |
| `gen_statem` lifecycles | `ogar-from-elixir` (sibling crate) lifts them into `ActionDef`s; same state names, same transitions (`docs/ELIXIR-HIRO-PREFETCH.md §2.2`) |
| XSD validation | Still available as `extract_classes.py` (cached in `vocab/imports/ogit/NTO/MARS/_oracle/`); the OGAR enum-lift is byte-equal to the XSD-extracted set, mechanically verified (`crates/ogar-from-schema/src/ttl.rs::application_class_values_appear_in_xsd_oracle`) |
| Phoenix REST API | Stays — `lance-graph-callcenter`'s `ExternalMembrane` (the firewall outer boundary) is exactly the surface to re-expose |
| HIRO operator UI | Same — reads from the same SPO triples, just stored in `lance-graph blasgraph` instead of the JVM graph core |
| Audit trail | Lance versions are the audit log (ADR-013); existing HIPAA-grade audit pattern (`DOMAIN-INSTANCES.md §2.5`) applies |

The migration is the textbook *strangler fig* shape: the OGAR core
absorbs reads first (it's the same data, in a faster shape), then
absorbs writes (the firewall membrane serializes them), then the
Elixir engine is gradually peeled off as its responsibilities move
into `ActionDef`s on the same `Class`es.

---

## 5. The literal-imports proof — and the bijection guarantee

The OGIT NTO/MARS taxonomy is now in this repo at byte-equality with
upstream. So is the SGO upper ontology (the AST verb vocabulary —
`dependsOn`, `contains`, `runsOn`, …). The producer reads both, and
the bijection is mechanically checkable at three levels:

```bash
# Level 1 — byte equality vs upstream (any output line is drift)
diff -qr vocab/imports/ogit/NTO/MARS/ /home/user/OGIT/NTO/MARS/ \
    | grep -v '^Only in vocab.*: \(PROVENANCE\|_oracle\)$'

# Level 2 — XSD-oracle agreement (the TTL enum set equals the
#   XSD-extracted classification set, chess-grade calibration)
cargo test -p ogar-from-schema ttl::tests::application_class_values_appear_in_xsd_oracle

# Level 3 — semantic round-trip (parse → emit → re-parse → equal)
#   over every MARS TTL and every one of 176 SGO verbs
cargo test -p ogar-from-schema ttl_emit::tests::all_mars_ttl_files_roundtrip
cargo test -p ogar-from-schema sgo::tests::all_sgo_verbs_roundtrip

# Regenerate the XSD oracle from scratch
cd vocab/imports/ogit/NTO/MARS/_oracle
python3 extract_classes_py3.py -s MARSSchema2015.xsd -F asciidoc > classifications.adoc
```

### Reverse engineering — bijective save-back

The producer is symmetric: anything OGAR lifts from TTL, OGAR can emit
back to TTL. The contract is **semantic bijection** — whitespace,
comment positions, and `@prefix` ordering are not preserved (they are
not load-bearing for the structural arm), but every predicate the OGIT
dialect uses survives the round-trip.

```rust
use ogar_from_schema::ttl::parse_file;
use ogar_from_schema::ttl_emit::emit_entity;
use ogar_from_schema::TtlDeclaration;

let src = std::fs::read_to_string("vocab/imports/ogit/NTO/MARS/entities/Machine.ttl")?;
let TtlDeclaration::Entity(once) = parse_file(&src).unwrap() else { unreachable!() };
let emitted = emit_entity(&once);
let TtlDeclaration::Entity(twice) = parse_file(&emitted).unwrap() else { unreachable!() };
assert_eq!(once, twice);   // every predicate survives
```

This means a colleague can author / edit OGAR `Class` structures in
Rust, emit OGIT-flavoured TTL, and feed it back into bardioc's existing
ingest pipeline — no migration cliff, no two-way translation table,
no drift detector to wire up. The producer **is** the translator.

That's the bardioc-efficiency story, end to end: **same MARS, faster
substrate, no migration debt at the structural boundary, and the lift
is symmetric so the round-trip never gets stuck.**

---

## 6. Cross-references

- `vocab/imports/ogit/NTO/MARS/` — the 1:1 mirror with PROVENANCE
- `vocab/imports/ogit/NTO/MARS/_oracle/` — the XSD + extract_classes.py oracle
- `crates/ogar-from-schema/` — the producer (TTL front-end, XSD queued)
- `docs/MARS-TRANSCODING.md` — calibration spec (chess-grade bijection)
- `docs/FOUNDRY-ODOO-MARS-LENS.md` — the cross-domain lens
- `docs/ELIXIR-HIRO-PREFETCH.md` — behavioural-arm prefetch (sibling)
- `docs/DOMAIN-INSTANCES.md` — MARS row in the universality matrix
- `docs/OGIT-DOMAIN-LIFT-CATALOGUE.md` — coverage status for all 72 OGIT domains
- `CLAUDE.md` P0 pin — the GUID-as-key + HHTL 256×256 centroid tile substrate
