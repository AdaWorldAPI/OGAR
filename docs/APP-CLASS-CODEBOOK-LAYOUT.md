# APP‖CLASS CODEBOOK LAYOUT — the full classid u32

> **What this resolves:** the operator's observation that the GUID's
> `classid` is **8 hex (u32)** but the codebook has only ever used the
> **low 4 hex (u16)**. The high u16 was *reserved zero*, not used for
> SoA versioning (that is `ENVELOPE_LAYOUT_VERSION: u8`, a separate
> byte — `lance-graph-contract/src/soa_envelope.rs:54`). This doc
> claims the low u16 as the **APP / codebook-namespace + render
> prefix** and pins the rule that keeps "classid is shared currency"
> intact.
>
> **Order flipped 2026-07-02 — canon HIGH / custom LOW.** The layout
> below now reads `classid : u32 = [ hi u16 : concept ] [ lo u16 : APP
> prefix ]` — the canonical concept sits in the **high** u16, the
> per-app render prefix sits in the **low** u16. Pre-flip docs and any
> already-baked data use the legacy order (`hi = APP`, `lo = concept`);
> see `docs/DISCOVERY-MAP.md` D-CLASSID-CANON-HIGH-FLIP.
>
> **Read together with** `docs/OGAR-CONSUMER-BEST-PRACTICES.md` — the
> muscle-memory guide with worked examples across every consumer. **The
> classid is pure address (both halves)**; behavior lives at the
> resolution target (ClassView for the skin / `Class`+`ActionDef` for
> the canonical shape and magic). Lo u16 selects **render** magic, NOT
> class magic; class magic is the Core's, never the address's.
>
> **The goal it serves (§3.5–3.7):** every renderable thing — strings,
> text, media, online sources — is rendered by **key-value resolution**
> against typed content stores, so **no serialization exists in the hot
> path** (the Firewall, ADR-022/023). The low u16 selects *which app's*
> Askama template / `ClassView` renders an object; the high u16 is the
> shared concept (RBAC + ontology); each field within is itself a key
> into a content store. Render = address resolution, never parse. The
> **same discipline extends to RAG** (§3.7): retrieval over the graph
> (rs-graph-llm) moves **keys**; content materializes into the LLM only
> at the membrane, exactly once. **Two membranes (UI render + LLM
> prompt), one rule — the hot path stays blob-free end to end.**
>
> Status: **SPEC** (codebook minting is mechanical from here, gated on
> the 5+3 pass + `PROBE-OGAR-RBAC-AUTHORIZE` for the auth arm).
> Append-only. Cross-refs: `CLASSID-RBAC-KEYSTONE-SPEC.md`,
> `CONSUMER-MIGRATION-HOWTO.md`, OGAR `CLAUDE.md` § "Codebook scoping
> = the class routing prefix", lance-graph CANON § "Minimal SoA node".

---

## 0. The layout (counted in hex, per the canon)

```
classid : u32  =  [ hi u16 : in-codebook class ]  [ lo u16 : APP / codebook namespace ]
                    0xDDCC                              0xAAAA
                    ^ domain DD | concept CC             ^ which codebook (0x0000 = shared core)
```

- **`hi u16` (0xDDCC) — in-codebook class id.** Domain byte `DD` +
  concept byte `CC`, exactly as the codebook encodes today. Within the
  core codebook (`lo = 0x0000`) the domain bytes are the canonical map
  (`0x01` project, `0x02` commerce, `0x07` osint, `0x08` ocr, `0x09`
  health, `0x0A` anatomy, `0x0B` auth, `0x0C` automation). Within an
  app-private codebook the app owns its own `DD|CC` layout.

  **The domain byte carries ALTITUDE** (operator, 2026-08-18 — *"Java is an
  entire different layer that's why I chose another higher level"*): a
  slot's magnitude encodes how high the thing sits, so placement is neither
  mnemonic nor next-free. `0x00`–`0x0F` is the canonical business/reference
  ontology; `0x17` is the substrate's own orchestration tier (`ogar-loco`);
  **`0xC0`+ is the C-band — strata ABOVE the Rust substrate**, foreign host
  layers the substrate reaches into rather than owns:

  | slot | domain | why there |
  |---|---|---|
  | `0xC0` | `JavaRuntime` (Panama FFM) | the managed-runtime membrane over the SoA substrate — the **floor** of the band. Valhalla deliberately has NO domain slot — not because it is unintegrated (lance-graph-java's shipping descriptor vocabulary is `value record`-ready by design, one word per type, with measured EA-build numbers) but because it is a representation *property of* the C0 concepts' vocabulary, never an addressable boundary itself — see the `JavaRuntime` doc comment in `ogar-vocab` |
  | `0xC1` | `Analytics` | the analyst estate — addressable tabular units + catalog ontology (`ogar-bricks` and a lakehouse consumer are two app prefixes over ONE vocabulary) |
  | `0xC4` | `BinaryLifting` | bolted onto `0xC0` (Ghidra is itself a JVM application), so a **tenant** of that layer rather than a peer; the slot number is the blast radius |

  All three are **reserved-empty** — zero concept rows, same posture as
  `Osint` / `Genetics` / `Blocks` — so `canonical_concept_domain` returns a
  stable tag before anything mints. `0xC2`–`0xC3` stay `Unassigned` **by
  intent**, pinned by test, exactly as the `0x10`–`0x16` gap below `Blocks`
  is: the slots were chosen deliberately, so a later pass must not tidy a
  C-band domain downward into the hole.

  Structural note (why altitude is worth the space it takes): the domain
  byte is the first two nibbles of the classid, so its **top nibble is a
  16-way altitude selector** — one mask separates "substrate ontology" from
  "host layer" with no lookup and no value decode. That is §3.5's *the key
  prerenders nodes with zero value decode*, applied to layering. A
  first-nibble split is the most expensive split the 16-ary cascade has;
  spending it on altitude is what makes it worth spending.

  Legibility hazard, recorded rather than rediscovered: **`0xC0` is a digit
  swap of `0x0C` automation** (`0xC001_0000` vs `0x0C01_0000`). Raised, not
  decisive, and pinned two-sided in `ogar-vocab`'s domain test so a
  transposition anywhere in the chain fails there first.
- **`lo u16` (0xAAAA) — APP prefix = codebook namespace selector.**
  Which 256⁶ semantic space / which centroid-codebook set the key
  resolves against (the longest-prefix codebook scoping already pinned
  in OGAR `CLAUDE.md`). `0x0000` is the **shared canonical core** — the
  cross-app ontology every consumer reuses. A non-zero value is an
  **app-private codebook**.

**This is additive, not a reclaim.** Every classid shipped to date is
`0xDDCC_0000` — i.e. it was *always* an APP‖class id with `APP = core`.
Nothing re-numbers. The canon's "RESERVE, DON'T RECLAIM" holds exactly:
`lo = 0x0000` is the bootstrap/core prefix; minting a non-zero `lo`
wakes app-private codebook routing with **zero `ENVELOPE_LAYOUT_VERSION`
change**, because the classid keeps its fixed 4-byte offset at key bytes
`0..4`.

---

## 1. The two halves carry two orthogonal things

An object's key holds **one** classid, yet two facts must travel with
it: *what it means* (shared, for RBAC + ontology + cross-app reasoning)
and *how this app renders it* (per-app — which Askama template + field
layout). The two halves of the u32 carry exactly those, orthogonally:

| half | answers | keyed by | shared? |
|---|---|---|---|
| **hi u16 `0xDDCC`** | **WHAT it is** — canonical concept + domain | RBAC grant lattice, ontology enrichment, cross-app identity | **shared** across all apps |
| **lo u16 `0xAAAA`** | **WHOSE rendering** — app `ClassView` / Askama template / SoA layout | object render + skeleton-layout | **per-app** |

So Medcare's patient is **`0x0901_0005`**: the high half `0x0901`
shares the `patient` grant lattice and OGIT ontology with *every*
health app; the low half `0x0005` binds it to **Medcare's** clinical
template. `0x0901_0000` is the canonical/abstract anchor (the master
concept + default `ClassView`). This is the canon's "the key prerenders
nodes with zero value decode" made literal — **both halves come
straight from the key**: high picks the concept/domain, low picks the
template, no value decode (see §3.5).

**Consequence: the low u16 is the NORM for every rendered object, not
an escape hatch.** Every app stamps its own prefix so its surface
objects bind to its own templates, *while still pulling a shared high-u16
concept* so RBAC and ontology stay cross-app. The unit of currency — the
**shared meaning** — is the high half; the low half is the render lens.

> **Rule: high half = pull a CORE concept (shared identity). Low half =
> stamp YOUR app prefix (your render binding).** An object that has a
> canonical analogue uses `core_concept ‖ your_app`. An object with **no**
> canonical analogue uses `app_local_concept ‖ your_app` — the genuine
> escape hatch, where the high half is also app-minted.

| | high half = CORE concept (`0xDDCC` shared) | high half = APP-LOCAL concept (`0xDDCC` app-minted) |
|---|---|---|
| **When** | the object means something canonical (patient, invoice, project) | the object has **no** canonical analogue |
| **classid** | `0x0901 ‖ your_app` e.g. `0x0901_0005` | `0xFF01 ‖ your_app` e.g. `0xFF01_0005` |
| **RBAC/ontology** | shared lattice via high half | app-private lattice |
| **Rendering** | app template via low half | app template via low half |
| **Frequency** | the **norm** | the **exception** |

The "codebook per project" win the operator named is the low half:
each app prefix roots **its own** centroid-codebook hierarchy and its
own ClassView/template set, so per-app rendering scales **without
radix-trie codebook overflow** — the shared core never has to hold one
template-variant per app.

**Promotion path (app-local → core):** if an app-local *concept* (high
half) turns out reusable, promote it into a core domain block via the
5+3 codebook gate and leave the app-local id as a deprecated alias.
ClassViews/templates never promote (they are app-private by nature).
Demotion never happens (RESERVE, DON'T RECLAIM).

---

## 2. APP-prefix allocation table (reserved; non-zero wakes a private codebook)

`lo = 0x0000` is core. Non-zero prefixes are **reserved by app** so two
apps never collide. Reserving a prefix costs nothing (no codebook is
materialised until the app mints its first private class).

| `lo u16` | App / namespace | Core domain(s) it consumes | Private codebook today? |
|---|---|---|---|
| `0x0000` | **Shared canonical core** | all (`0x01/02/04/07/08/09` + `0x0A` anatomy + `0x0B` auth + `0x0C` automation) | n/a (this *is* core) |
| `0x0001` | OpenProject (openproject-nexgen-rs) | `0x01` project-mgmt | **no** — maps onto core |
| `0x0002` | Odoo | `0x02` commerce | **no** — maps onto core (converge `od-ontology`) |
| `0x0003` | WoA / woa-rs | `0x02` commerce (work orders) | **no** — maps onto core |
| `0x0004` | SMB-Office / smb-office-rs | `0x02` commerce | **no** — maps onto core |
| `0x0005` | **Medcare / medcare-rs** | `0x09` health | **escape hatch only** (see §3) |
| `0x0006` | q2 (Gotham / aiwar / neo4j) | `0x07` osint (+ TBD) | **TBD** — port not yet authored |
| `0x0007` | Redmine | `0x01` project-mgmt | **no** — same concepts as OpenProject, own templates |
| `0x0008` | OpenStreetMap (openstreetmap-website-rs) | `0x0F` geo | **no** — maps entirely onto core |
| `0x0009` | WeatherNext / weathernext-rs | `0x04` weather / atmosphere | **no** — maps onto core |
| `0x00A0` | (reserved) future app block | — | — |

> **OpenProject (`0x0001`) and Redmine (`0x0007`) are the showcase:**
> same high-u16 concepts (`WorkPackage`/`Issue` both → `0x0102`
> project_work_item; same RBAC `project_role 0x0117` lattice), different
> low-u16 render prefix (different ClassView/Askama template). Two
> renders, one concept — the cleanest demonstration of §1. See
> `APP-CODEBOOK-MIGRATION-PLAN.md` W0.

Auth is **not** its own APP — auth providers are canonical, cross-app
profiles, so they live in **core** under a new `0x0B` auth domain
(`auth_store = 0x0B01_0000`, `auth_zitadel`/`auth_zanzibar`/
`auth_ory_keto` as provider profiles). See
`CLASSID-RBAC-KEYSTONE-SPEC.md` §7 — those classes mint into core, not
under any app prefix. (This corrects the earlier "flat 0x011B" mint
attempt: auth classes are core-domain `0x0B`, not project-domain
`0x01`.)

---

## 3. MEDCARE — the worked APP‖class layout

Medcare is one **consumer of the canonical Health domain**, not the
owner of clinical ontology. So:

### 3a. Concept high half — PULLED from core (domain `0x09`, shared)

The 7 canonical OGIT Healthcare concepts — already shipped, shared with
any future health consumer. These are the **high-u16 concept anchors**;
their `lo = 0x0000` form is the abstract master + default ClassView:

| Concept | core anchor (u32) | high half (shared) | Source |
|---|---|---|---|
| patient | `0x0901_0000` | `0x0901` | OGIT `Healthcare:Patient` |
| diagnosis | `0x0902_0000` | `0x0902` | OGIT `Healthcare:Diagnosis` |
| lab_value | `0x0903_0000` | `0x0903` | OGIT `Healthcare:LabValue` |
| medication | `0x0904_0000` | `0x0904` | OGIT `Healthcare:Medication` |
| treatment | `0x0905_0000` | `0x0905` | OGIT `Healthcare:Treatment` |
| visit | `0x0906_0000` | `0x0906` | OGIT `Healthcare:Visit` |
| vital_sign | `0x0907_0000` | `0x0907` | OGIT `Healthcare:VitalSign` |

`HealthcarePort` resolves Medcare's surface names (`Patient`,
`Befund`→`diagnosis`, `Laborwert`→`lab_value`, …) onto these **high
halves**. RBAC + ontology key on the high half, so the grant lattice is
shared with every health consumer. **No medcare bridge.**

### 3b. Render low half — STAMPED as `0x0005` (Medcare's ClassView)

Medcare's *rendered* objects carry its own prefix in the low half. Same
shared concept, Medcare's template + SoA layout:

| Medcare object | classid (u32) | high (shared concept) | low (Medcare render) |
|---|---|---|---|
| Medcare patient view | `0x0901_0005` | `0x0901` patient | Medcare `patient.html` Askama template |
| Medcare diagnosis (Befund) | `0x0902_0005` | `0x0902` diagnosis | Medcare `befund.html`, PII leaf-rename at adapter |
| Medcare lab value (Laborwert) | `0x0903_0005` | `0x0903` lab_value | Medcare `laborwert.html` |

- **Authorize** keys on `classid >> 16` → `0x0901` → shared patient
  grant: `authorize(actor, (0x0901_0005 >> 16) as u16, Read)`.
- **Render** keys on the full u32 → `0x0901_0005` → Medcare ClassView →
  Askama template + field order (§3.5).
- The PII leaf-rename (German clinical labels never leave the membrane)
  is the Medcare ClassView's job — bound by the low half, exactly where
  it should be.

### 3c. Genuinely-bespoke Medcare objects (high half ALSO app-minted)

Only entities with **no** canonical analogue — high half is Medcare's to
mint too:

```
0xF0CC_0005   medcare bespoke object classes (no canonical analogue)
0xFFCC_0005   medcare local special-cases (the long tail CLAUDE.md §3 names)
```

| Candidate | Provisional classid | Why fully app-private |
|---|---|---|
| medcare insurance-case (German GKV/PKV billing quirk) | `0xF001_0005` | clinic-billing specific; no canonical analogue yet |
| medcare KIM/TI-message envelope | `0xF002_0005` | German telematics-infra specific |
| medcare migration import row (`/api/admin/migration/sql/import`) | `0xFF01_0005` | the one MySQL-only-by-design route (medcare-rs CLAUDE.md) |

**If a concept has a canonical analogue, use it in the high half** (§3b)
— do not fork a new high-u16 id just to render it. The fully-private form
(§3c) is for concepts that genuinely don't generalize.

### 3d. Medcare's diff, concretely

1. **OGAR** (one PR, gated): reserve `0x0005` for Medcare; confirm
   `HealthcarePort` maps the 7 core concept **high halves** (it does —
   `ports::HealthcarePort`); register Medcare's ClassViews/templates
   under `0x0005`. Mint fully-private (§3c) classes only when a real
   bespoke entity needs one (none required to ship the patient-read
   gate).
2. **medcare-rs** (its own crate): pull the concept high half statically
   (`HealthcarePort::class_id("Patient") == Some(0x0901)`); form its
   render classid `(0x0901 << 16) | 0x0005 = 0x0901_0005`; enrich (RLS /
   masks) and render by that classid; authorize by the **high half**. The
   spine (`lance-graph-ogar`, `lance-graph-rbac`) is byte-for-byte
   unchanged.

The in-flight medcare patient-read gate (PR #169) is the first
consumer: today it keys on `static_role`; once the keystone + this
layout land, it keys on `authorize(actor, (0x0901_0005 >> 16) as u16, Read)`
(= the shared `0x0901` patient grant) and renders via the `0x0005`
Medcare ClassView.

---

## 3.5. Rendering — the low u16 IS the Askama / ClassView binding

This is *why* the low u16 is the norm, not an escape hatch. **Object
rendering is per-app**, and the render binding must come straight from
the key (canon: "the key prerenders nodes with zero value decode"). The
low u16 is that binding:

```
object key ──► classid (u32) ──► ClassView lookup keyed on FULL u32
                  │                    │
                  │                    ├─ Askama template handle  (which .html)
                  │                    ├─ SoA field order / column projection
                  │                    └─ label set (PII leaf-rename at adapter)
                  │
                  └─ high u16 ─► concept/domain ─► RBAC grant + ontology (shared)
```

- **One concept, many renders.** `0x0901_0000` (canonical patient),
  `0x0901_0005` (Medcare's patient form), and a hypothetical
  `0x0901_0007` (another health app's patient card) are the **same
  concept** (high half `0x0901` — same grant, same ontology) rendered
  three ways (three ClassViews / three Askama templates). The low half
  selects the template **without** decoding the row.
- **`ClassView` is already the render manifest.** The canon binds
  `classid → ClassView`; the ClassView carries the structural signature
  (field set, order) the template iterates. Stamping the low u16 means
  "use *this app's* ClassView for this concept" — the Askama template is
  a property of that ClassView, resolved by the same `resolve` read that
  resolves schema and codebook.
- **Why not render off the high half alone?** Because then every app
  would have to share one template per concept, or fork the concept id
  to get a distinct template — the radix-trie overflow the operator
  flagged. Splitting render (low) from meaning (high) lets an unbounded
  number of apps each render `patient` their own way while the `patient`
  grant lattice and ontology stay singular and shared.
- **Skeleton render with zero value decode.** A list/grid/planner view
  can lay out N objects — pick each one's template, group by app, order
  fields — from the keys alone, before fetching any value bytes. That is
  the canon's KEY-IS-KEY-OF-KEY-VALUE promise applied to the UI layer.

**Consumer pattern (woa-rs / smb / medcare with Askama):**

```rust
// at the boundary: object's full classid is in its key
let concept = (classid >> 16) as u16;    // 0x0901 — shared: RBAC + ontology
let render  = ClassView::resolve(classid); // full u32 — this app's template
let html    = render.template().render(&row)?;   // Askama, compile-time checked
authorize(actor, concept, Op::Read)?;     // grant lattice on the shared high half
```

Stack note: woa-rs and smb-office-rs already use **Askama** (compile-time
checked templates — woa-rs CLAUDE.md stack table). The render classid is
the key that selects *which* compiled template; nothing about the
template engine changes — the classid just replaces ad-hoc
`match entity_kind { … }` template dispatch with a key-driven ClassView
lookup.

---

## 3.6. THE GOAL — content is key-value too, so NO serialization in the hot path

The render binding (§3.5) is only half the win. The other half is that
**every renderable field — string, text, media, online source — is
itself a key-value entry**, resolved by address, never serialized into
the row and deserialized to render. This is the Firewall (ADR-022/023:
*no serialization in the hot path*) made structural: there is nothing to
serialize, because content was never inlined — it is always a key that
resolves to bytes already sitting in their backing store.

This is the **registry axiom** (`CLASSID-RBAC-KEYSTONE-SPEC.md` I-K0 —
label = KEY, meaning = VALUE) applied to *content*:

| Field kind | Inline blob (FORBIDDEN — serializes) | Key-value entry (CANON) |
|---|---|---|
| **String** | the bytes in the row, serde-encoded | a dictionary/palette **key** → interned string table (Lance dictionary column); render = O(1) lookup |
| **Text** | the paragraph serialized into the value | a `text` content **classid ‖ identity** → Lance column; render = zero-copy mmap slice |
| **Media** (image/audio/blob) | base64 in JSON | a `media` content **key** → bytes in Lance / object-store URI resolved from the key; render emits a reference, bytes stream zero-copy |
| **Online source** (URL/remote) | the fetched body serialized into the row | a `source` **key** → URI registry entry; render resolves key → canonical URI (+ cache), the remote body is never serialized into the object |

So a rendered object is a **tree of keys**: the object's classid (low =
app template, high = concept) selects the Askama template; each field the
template iterates is itself a key that resolves — by the same key-value
lookup — into a typed content store. The whole render path, top to leaf,
is **address resolution**, not parsing.

```
object classid (u32) ─► ClassView ─► Askama template
   for each field:
     field key ─► content store (string dict / text col / media / source registry)
                    └─ resolve = columnar / dictionary lookup, LE bytes in place
   ── no serde::Deserialize anywhere on this path ──
```

Why this is exactly the canon:

- **"The key prerenders nodes with zero value decode."** The render
  walks keys; value bytes are touched only as the final zero-copy slice
  the template emits — never decoded into an intermediate struct.
- **"Lance is free to compress the value bits arbitrarily... the store
  still has a transparent view and address."** Content stores compress
  (dictionary, PQ, palette) freely; render still addresses by key
  because the key is never compressed.
- **"Every SoA envelope is zero-copy from creation to Lance tombstone;
  Lance's own columnar I/O writes LE bytes from the in-place backing
  store."** Rendering reads those same LE bytes in place. Nothing is
  serialized *to be rendered*.

**Litmus for any render path:** if rendering a field requires a
`serde::Deserialize` / `serde_json::from_*` / a parse step, the field
was inlined as a blob — that is the Firewall violation. The fix is to
make the field a **key** into a content store and resolve it, not parse
it. Strings/text/media/sources are CAM/registry entries, never inline
serialized payloads. (Build-time codegen that *generates* the ClassView
from a manifest is fine — that is "compile types", not hot-path serde;
cf. medcare-rs CLAUDE.md §7.)

---

## 3.7. RAG / LLM is the SAME egress discipline — key pointer in, content at the membrane

UI render (§3.5) and RAG-to-LLM are the **same pattern with two
membranes**. Retrieval-augmented generation over the graph
(rs-graph-llm `graph-flow` driving lance-graph retrieval) keeps the hot
path blob-free by moving **keys**, and materializes content **only at
the LLM membrane**, exactly once:

```
hot path (BLOB-FREE — pointers only):
  query ─► CAM-PQ / palette / Hamming search on fingerprints
        ─► ranked classid ‖ identity KEYS (pointers, not content)
        ─► graph walk / dedup / rank / assemble  ── all on keys ──
        ─► context = a LIST OF KEYS (a pointer set, never a concatenated blob)

membrane (the ONE egress point — content materializes here, once):
  for each key in context:
     key ─► content store (§3.6: string dict / text col / media / source URI)
          ─► content REFERENCE lands in the LLM prompt
  └─ this is the only place a key becomes tokens ─┘
```

- **Retrieval returns pointers.** Search (CAM-PQ compressed NN, palette
  distance, Hamming) operates on fingerprints/keys and yields ranked
  `classid ‖ identity` keys. Nothing is decompressed or deserialized to
  rank — the canon's "compare without decompressing" (`codec.distance`).
- **Context is a key set, not a blob.** The assembled RAG context is a
  list of pointers. Graph traversal, dedup, and ranking all move keys.
  No serde, no concatenated text buffer, in the hot path.
- **Content lands in the LLM only at the membrane.** Each retrieved key
  resolves to its content reference (§3.6) at the boundary to the LLM
  call — the same "boundary parsed once" the Firewall mandates, and the
  same MarkovBarrier the cognition stack already uses (crewai-rust
  blood-brain-barrier: inner cognition on keys/fingerprints, content
  materialized only at the external API edge). The low u16 still picks
  *which app's* view/template a key materializes through, so RAG
  citations render in the asking app's voice.
- **Two membranes, one rule.** UI render and LLM prompt are both egress
  points where a key resolves to content exactly once. Everything
  *behind* either membrane — storage, retrieval, ranking, assembly — is
  pointer movement. **The hot path stays blob-free end to end.**

**Litmus (RAG):** if context assembly builds a `String`/`Vec<u8>` of
materialized content before the LLM call, the blob entered the hot path
too early. Assemble a `Vec<key>`; resolve at the membrane. (The LLM
*does* receive materialized text — that is correct; the point is it
happens once, at the edge, not threaded through retrieval.)

---

## 3.8. Intuition — this is C64 / 6502 addressing

The model is, deliberately, **8-bit-machine assembler**. It is worth
holding the analogy because every piece maps and the mapping is exact
(re-mapped 2026-07-02: concept is now the page, the APP/render prefix is
now the offset):

| 6502 / C64 / VIC-II | this layout |
|---|---|
| 16-bit address as **page : offset** (zero-page indirect) | `classid` as **hi u16 : lo u16** (concept page : codebook/app offset) |
| **Character ROM** ($D000): char code → 8-byte glyph | **string/glyph codebook**: a key → interned bytes (§3.6 string row) |
| **Screen RAM** byte → VIC-II reads glyph, zero decode, every frame | object field key → ClassView resolves content, zero decode, every render |
| **Sprite pointers** ($07F8–$07FF): 1 byte → 64-byte sprite block | **media key** → media bytes / URI (§3.6 media row) |
| **Jump table** indexed by opcode/class | `classid → ClassView` → Askama template handle (§3.5) |
| **PEEK/POKE** — move addresses, content sits at fixed locations | move keys; content lives in its store; resolve by address |
| **No serialization exists** — there is no serde on a 6502 | no serde on the hot path (§3.6); content is addressed, not parsed |

The VIC-II rendering a frame — read a screen byte, index character ROM,
emit the glyph, 50 times a second — *is* the §3.6 doctrine in silicon: a
key→value lookup table render with zero serialization in the hottest
path a machine has. We are rebuilding that discipline on Lance columns
and a 32-bit classid instead of $0400 screen RAM and a 16-bit address.
The "modern app" habit it rejects is the serde blob: a C64 never
deserialized a sprite, and neither should the hot path.

---

## 4. Routing consequence (one function, longest-prefix-wins)

`classid_concept_domain` today reads only the high u16
(`canonical_concept_domain((classid >> 16) as u16)` —
`lance-graph-contract/src/ogar_codebook.rs:81`). Under APP‖class that
becomes a two-step longest-prefix bind, and it stays O(1):

```rust
fn resolve_codebook(classid: u32) -> Codebook {
    match classid as u16 {
        0x0000 => Codebook::Core,          // shared canon (today's behaviour)
        app    => Codebook::App(app),      // app-private namespace
    }
}
// app = classid as u16; concept = (classid >> 16) as u16; domain = (classid >> 24) as u8
fn domain_in(classid: u32) -> u8 { (classid >> 24) as u8 }
```

For `lo = 0x0000` this is **bit-identical to today** — no regression,
no version bump. App-private codebooks add their own `(app → domain map)`
record next to their `ClassView` in the registry (the codebook-mints-
with-the-class shelf the canon already describes). This is the same
"the classid group sits in front of the path bytes; codebooks are
selected by the key's own prefix" rule, now exercised at the u16
granularity instead of only at the byte granularity.

---

## 5. Invariants (proposed; pin on 5+3 pass)

- **I-APP1 — additive:** every existing classid is `0xDDCC_0000`; the
  low u16 was always present and zero. No id re-numbers.
- **I-APP2 — core is shared:** canonical concepts live in `lo = 0x0000`
  and are pulled by every consumer. An app never re-numbers a canonical
  concept into its own prefix.
- **I-APP3 — private is the exception:** an app mints `lo = 0xAAAA`
  classes only for objects that fail the "would a second consumer reuse
  this?" test. Default is map-onto-core.
- **I-APP4 — reserve, don't reclaim:** app prefixes are reserved once
  and never re-assigned; promotion (private→core) leaves the private id
  as a deprecated alias; demotion never happens.
- **I-APP5 — zero version cost:** because classid keeps its fixed 4-byte
  offset, waking a non-zero low u16 changes no `ENVELOPE_LAYOUT_VERSION`
  and breaks no v1 reader of a `0xDDCC_0000` key.
- **I-APP6 — domain map is codebook-local:** `0x09 = health` is true in
  the core codebook; an app-private codebook defines its own domain
  bytes and must ship its `(app → domain)` record with its ClassView.

---

## 6. What this is NOT

- **Not SoA versioning.** `ENVELOPE_LAYOUT_VERSION: u8 = 2` is the SoA
  version, a separate byte. The low u16 of classid has nothing to do
  with it (the question that prompted this doc).
- **Not a per-app bridge.** The app-private codebook is *data* (a class
  block + a PortSpec), authored in OGAR, named by classid. It is not a
  consumer-side bridge crate. `CONSUMER-MIGRATION-HOWTO.md` still holds:
  pull classid, enrich, authorize.
- **Not a license to mint freely.** Map onto core first. The escape
  hatch is for capacity/specificity, not convenience.
