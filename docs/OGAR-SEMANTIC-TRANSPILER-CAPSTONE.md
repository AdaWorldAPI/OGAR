# OGAR as a semantic transpiler that sinks into the v3 substrate

> **Status:** CAPSTONE / landing-zone (a *destination*, ahead of the code by
> design). Hardened by the 5+3 pass — factual mechanics corrected, the vision
> kept whole. Each claim is tagged **CODED-today** (shipped, file:line) or
> **LANDING-ZONE** (the target we transpile *toward*). A landing-zone claim is
> not weaker for being unbuilt; it is the direction.
>
> **The one-line payoff:** an Odoo ERP *at the cost of an import* — and the same
> substrate that holds the ERP also holds anatomy, ontologies, and genomics under
> one address space. This is whole-app transpilation into a compiled, living
> substrate, not data-integration-into-an-ontology. It is past Palantir Foundry,
> not chasing it.

## 1. The frame — and why it beats Foundry

A transpiler is a source-to-source compiler. OGAR transpiles **source → substrate**:
every front-end lowers into the OGAR AST and **sinks** into the v3 node store
(§1.5's own word: the ClassView "recombines the carvings while *sinking into
OGAR* and getting compiled into the binary"). The transpiler does not emit rows
into a database — it emits **typed, classid-addressed, compiled** nodes that
route, group, and skeleton-render from the key alone (`D-KEYKV` **[G]**, "the key
prerenders with zero value decode").

**Two kinds of front-end — name them distinctly up front:**

- **Dissolving front-ends** (GFA/VCF, raw RDF, DICOM pixels). No DO arm; meaning
  *is* the columns. "GFA = SoA wearing a schema glove" — a thin parser; syntax
  dissolves into SoA columns + a classid schema.
- **Re-hosting front-ends** (Odoo, Redmine, OpenProject — whole applications).
  The source is *already semantic syntax*. This is **semantic-syntax → semantic-
  syntax**: the **selection structure is re-hosted, not dissolved** (Redmine's
  ERB field-view → the Askama bitmask field-view; the host language changes, the
  field-selection structure survives). *(Edge: OWL-with-inference straddles the
  two — its subClassOf/domain-range semantics are a behavioural arm; classify it
  case-by-case.)*

**Why this is past Foundry.** Palantir Foundry integrates *data* into an Ontology
you build on top of a data lake: you re-model the source and **rebuild its
workflows** in Foundry's own action layer; it is a proprietary, batch-pipeline
metadata surface. OGAR imports the **whole app** — the THINK arm *and* the DO arm
— into **one classid-addressed, compiled, living substrate**. You do not rebuild
the ERP; you **transpile** it. That is the "at the cost of an import" claim
Foundry structurally cannot make, and the substrate is **domain-universal** (the
same address space holds FMA anatomy, SNOMED, RxNorm, genomics — not just
enterprise tables).

## 2. The three arms — what is preserved, what is *carried*, what re-hosts

Per `OGAR-AST-CONTRACT.md`:

- **THINK arm — `Class` → ClassView → fieldview (a three-level chain, not one
  layer):** `Class` is the source-of-truth IR node (data + declared hooks);
  **ClassView** is a per-app render skin (hi-u16); **fieldview** is a presence
  mask over ClassView. Structure-preserving: Odoo view → ClassView; Redmine ERB
  partial → Askama bitmask partial; OpenProject WP schema → `Class` facets.
  **CODED-today** (`lance-graph-contract/src/class_view.rs`: `ClassView`,
  `render_rows`, `project`; impl `ogar-class-view`).
- **DO arm — `ActionDef` / `ActionInvocation` → a generated ractor `StateMachine`
  (with `KausalSpec` state-guards).** This is the catch that makes "ERP at the
  cost of an import" real: Odoo's action methods are **carried into actual state
  machines**, not stubbed as stateless adapters. (The "classid-keyed adapter" of
  the Core-First doctrine is the *data-shaped leaf-method* lowering only — its
  scope boundary; behaviour-bearing actions are the `StateMachine` path.) Foundry
  rebuilds these workflows by hand; OGAR transpiles them. **LANDING-ZONE** (the
  `Class`→`StateMachine` codegen is specced; gate **F1** — delegation ≡ Odoo
  `_inherit` — proves behavioural parity and **has not run yet**; the God-object→
  three-way-split "refactor dividend" is **CONJECTURE** until F1 is green).
- **DO-arm guard — `KausalSpec`:** the state-guard *field on* `ActionDef` that
  gates a transition (not a separate co-equal "arm").

The split is itself the separation-of-concerns the source monolith never had —
`Class` (data) + ClassView/fieldview (view) + `StateMachine` (behaviour) out of
one fat Odoo/Rails model. We *argue* this is a net architectural win; F1 is the
falsifier.

## 3. The universal selector — one representation, the jobs it really does

The presence `FieldMask` over the **N3 append-only field basis** is a genuine,
shipped, single-bit-space selector. Be exact about which jobs it does:

| job | mask / mechanism | owner of the bits | status |
|---|---|---|---|
| **read** (facet decode) | N3 presence `FieldMask` | compiled reader | **CODED** (`class_view.rs`) |
| **render** (field view) | same N3 `FieldMask`, bit-gated loop | compiled template | **CODED** (`render_rows`) |
| **auth field-projection** | same N3 `FieldMask`, OR-folded | role × lo-u16 grant lattice | **CODED** (`lance-graph-rbac::authorize_scoped`, `permission.rs::projection`) |
| **query** (column pruning) | a **second** `FieldMask` on the **physical value-tenant** basis | the value-slab layout | **CODED but a DIFFERENT basis** (`canonical_node.rs::ValueSchema::field_mask`) |
| **version** (V1/V2/V3) | a `classid → ReadMode/ValueSchema` **radix lookup** (not a mask op) | `ENVELOPE_LAYOUT_VERSION` registry | **CODED as a lookup** |
| **auth verdict + row-scope** | boolean gate + **AND**-folded scope | RBAC keystone **I-K8** (four orthogonal axes) | **CODED, not the mask** |

**The honest collapse is THREE jobs over the N3 basis — read + render + auth-
field-projection — shipped and tested** (`distinct_roles_carry_disjoint_projections_of_one_class`).
That is the real universal selector, and it is real today. `query` rides a
*second* mask on the physical basis; `version` is a radix lookup; auth's
verdict/scope are ordered folds with a different algebra (AND-scope vs OR-
projection — one mask cannot be both). Collapse the three; keep the others
named-distinct.

**Two disciplines (both load-bearing, neither a blocker):**

1. **Field count is a representation parameter, not a ceiling.** A presence mask
   is `[u64; N]` — `[u64; 4]` = 256 fields, wider as needed; Odoo's 109-field
   `account.move` is a `[u64; 2]`, not a wall. Widen the basis; the algebra is
   unchanged. *(`I-LEGACY-API-FEATURE-GATED` still applies: `field ↔ idx ↔ bit`
   must be generated from one source, never hand-numbered, or a version bump
   re-aliases bits.)*
2. **The two bases must be type-separated (a live fix).** The N3 *logical* mask
   and the value-tenant *physical* mask are both `FieldMask(u64)` today — nothing
   stops a caller intersecting them and getting a silently-wrong result. Make
   them un-intersectable: `FieldMask<Logical>` vs `FieldMask<Physical>`. This is a
   real defect to close, surfaced by the hardening pass.

## 4. The address is the zoom

**CODED-today.** The classid/HHTL cascade is the multi-scale axis: a longer
Morton-nibble prefix is simultaneously a finer tile AND a deeper partonomy node
(`FMA-SKELETON` §2.1: `address_prefix_is_partonomy_containment`,
`morton_address_encodes_laterality`, **[G]**). Planet → organ → cell →
chromosome → gene is **prefix routing** over the **3×4 path** (`D-3X4`, tier =
`nibble >> 2`). The renderer for a node is **resolved *from* the classid** (per
the Non-negotiable "classid is pure address; the magic is what it resolves to") —
the structural signature lives at the resolution target, not in the address bits.

**LANDING-ZONE.** Renderer-dispatch-*by-signature* across domains (one tube
renderer for VG graphs *and* arterial trees) is a target, not built:
`StructuralSignature` today is a group-by family key, not a renderer selector,
and no tube/mesh/molecular renderer exists yet. A coordinate-free source (a VG
sequence graph has no rest-pose centroid) needs an explicit address-assignment
step before it lands here — that is a future landing zone, named, not a finished
claim.

## 5. Living substrate; the Firewall-correct egress

- **Living, not a dump.** ractor gives a **runtime ownership guarantee**: each
  actor owns its SoA partition and its mailbox serializes access to that partition
  at runtime, so the fleet runs **n actors × up-to-64k partitions concurrently** —
  parallel across actors, race-free because each partition has exactly one owner.
  This is a *runtime* actor-ownership guarantee (the mailbox is the runtime
  serializer), **not** a compile-time borrow proof (`MailboxSoA`, `kanban_actor`).
  The graph is live (kanban updates streaming in), not a batch snapshot.
- **JSON-free, Firewall-correct.** **LANDING-ZONE / [H]** (`D-KV-RENDER`, pending
  the no-serde probe). The inner path moves **keys** (a tree of keys), content
  materializes **only at the egress membrane, exactly once**. The Rust→browser
  transport (Arrow IPC / binary columns / rendered HTML) is the **OUTER firewall
  crossing** — legitimate boundary tax paid once per crossing (`THE-FIREWALL.md`
  §5; ADR-022/023 are **not** violated — there is no Arrow encode on the *inner*
  hot path). The JS engine (JSC/V8) is in-process via op/FFI, so the runtime↔Rust
  handoff is memory, not a wire. No JSON reconstitution on the core path.

## 6. The dots, connected

Parse once per source → sink into classid-keyed SoA → and the jobs that *share
the N3 basis* (read, render, auth-field-projection) come from the same mask, while
query/version/scope stay distinct selectors. Genomics (VG/GFA) is the next
*dissolving* front-end; Odoo/Redmine/OpenProject are the *re-hosting* front-ends
where structure is preserved and behaviour is carried into state machines. The
Askama bitmask field-view is the render-side face of the N3 selector the
substrate already uses to read and project. **An ERP at the cost of an import,
in the same substrate as the anatomy and the genome** — that is the destination,
and the spine of it (sink, N3 selector ×3, address-as-zoom) is shipped today; the
rest is a named set of landing zones, not hand-waving.

## 7. Honest edges (sharp, not subjunctive)

1. **Two `FieldMask` bases must be type-separated** (logical N3 vs physical
   value-tenant) — a live aliasing defect; `FieldMask<Logical/Physical>`.
2. **Field count widens, it does not block** — `[u64; N]`; the only discipline is
   single-source bit generation (`I-LEGACY-API-FEATURE-GATED`).
3. **DO-arm parity is unproven** — gate **F1** (delegation ≡ `_inherit`) must run
   before the God-object→adapter "dividend" is more than CONJECTURE.
4. **Cross-domain renderer-by-signature is a landing zone** — needs a coded
   signature→renderer map and an address-assignment for coordinate-free sources.
5. **Upper ontology: adopt Biolink**, do not invent the category tree.
6. **First brick (coded surface, not genomics):** render an existing OpenProject
   `WorkPackage` / medcare `Patient` row through `ClassView::render_rows(class,
   FieldMask)`, proving one N3 mask drives projection *and* render. ~60 LOC; it
   promotes the three-job selector claim from CODED-in-parts to end-to-end.

## Cross-references

- `lance-graph-contract/src/class_view.rs` (`ClassView`/`FieldMask`/`render_rows`),
  `canonical_node.rs` (the physical value-tenant basis), `lance-graph-rbac`
  (`authorize_scoped`, `permission.rs::projection`) — the real receipts for §3.
- `CLASSVIEW-FIELDVIEW-ASKAMA-BITMASK.md` (this PR) + `integration/CLASSVIEW-MATERIALIZATION-PLAN.md` — the render-side face.
- §1.5 compiled-ClassView spine; `OGAR-AST-CONTRACT.md` (arms); `OGAR-AS-IR.md`.
- `CLASSID-RBAC-KEYSTONE-SPEC.md` **I-K8** (four orthogonal auth axes).
- `THE-FIREWALL.md` §5 + `D-KV-RENDER` (egress is an outer crossing); ADR-022/023/024.
- `D-KEYKV`, `D-3X4`, `D-BOTHCASC`, `FMA-SKELETON` §2.1 (address = zoom, coded);
  `I-LEGACY-API-FEATURE-GATED`; `core-first-transcode-doctrine` (leaf-adapter scope).
