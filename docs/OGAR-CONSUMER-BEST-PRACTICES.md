# OGAR Consumer Best Practices — the muscle-memory guide

> **Audience:** every consumer-side session — medcare-rs, woa-rs,
> smb-office-rs, odoo-rs, openproject-nexgen-rs, q2, and any future
> consumer.
>
> **Why this doc:** the OGAR canon is right. The trap spellbook
> (`SURREAL-AST-TRAP-PREFLIGHT.md`) is right. But sessions don't get
> patterns from principles — they get them from **repeated worked
> examples**. This guide is the muscle-memory pass: every canonical
> pattern shown with a concrete classid, a real consumer, real code,
> and an anti-example alongside.
>
> Read once before authoring consumer code. Pattern-match against the
> examples; if your code doesn't look like one of them, stop and check.
>
> **Order flipped 2026-07-02 — canon HIGH / custom LOW.** Every worked
> example below now reads `classid = concept(hi u16) ‖ APP(lo u16)` —
> the canonical concept sits in the high u16, the per-app render prefix
> sits in the low u16. Pre-flip material and already-baked data use the
> legacy order; see `docs/DISCOVERY-MAP.md` D-CLASSID-CANON-HIGH-FLIP.
>
> Status: **BEST PRACTICE v1** (2026-06-22). Append-only.
>
> Companions: `APP-CLASS-CODEBOOK-LAYOUT.md` (the layout spec),
> `CONSUMER-MIGRATION-HOWTO.md` (the migration recipe),
> `SURREAL-AST-TRAP-PREFLIGHT.md` (the trap to avoid),
> `CLASSID-RBAC-KEYSTONE-SPEC.md` (the authorize gate — pending probe).
> Parallel-session sibling: `lance-graph/.claude/knowledge/ogar-consumer-preflight.md`.

---

## §0. The one-line invariant — read this before everything else

> **The classid is pure address. The magic is what the address
> resolves to.**

Both halves of the classid are address dimensions, never behavior:

```
classid : u32  =  0xDDCC  ‖  0xAAAA                       (8 nibbles)
                    │           │
                    │           └─ lo u16: WHOSE RENDER  ─┐
                    │              (APP / ClassView /       │
                    │               Askama template)        │  BOTH halves
                    │              per-app; never shared    ├─ are pure
                    │                                       │  ADDRESS.
                    └─ hi u16: WHICH CONCEPT ───────────────┤  Neither
                       (DD = domain,                         │  carries
                        CC = concept index)                  │  behavior.
                       shared across all apps                ─┘

         ──────►  resolves to  ──────►
                     │
                     ├─ ClassView           the SKIN     (render shape, per-app — picked by lo)
                     ├─ Class               the SHAPE    (structural — canonical, picked by hi)
                     └─ ActionDef +         the MAGIC    (behavioral — lifecycle, callbacks,
                        KausalSpec                        validations; ALWAYS in OGAR Core,
                                                         NEVER in DDL or in the address)
```

**Three drilled corollaries:**

1. **The address is dumb.** Knowing the classid tells you the *shape* + *skin* + which Core node to fetch — it tells you **nothing** about behavior. Behavior lives at the resolution target.
2. **Class-magic ≠ render-magic.** The lo u16 chooses **render** magic (which app's template), not **class** magic (which callbacks/lifecycle). Class magic is the Core's; render magic is the app's.
3. **You can't smuggle magic into the address.** Encoding behavior in DDL constructs (`DEFINE EVENT … WHEN … THEN …`) puts magic where only the address lives — the trap `SURREAL-AST-TRAP-PREFLIGHT.md` exists to prevent.

---

## §1. Address anatomy — worked examples that drill the layout

Memorize these. They appear across every consumer doc; recognizing them on sight is the muscle.

```
0x0901_0000 ─── canonical patient (core anchor, no app skin)
   │      │
   │      └── 0x0000 = core (shared, no app prefix)
   └───────── 0x09 = Health domain · 0x01 = patient concept

0x0901_0005 ─── Medcare's patient — same concept, Medcare's render lens
   ↑      ↑
   │      └── 0x0005 = Medcare's APP prefix → Medcare's ClassView + Askama template
   └───────── same 0x0901 = same RBAC grant, same ontology, same OGIT identity

0x0102_0001 ─── OpenProject's WorkPackage
0x0102_0007 ─── Redmine's Issue
   ↑              ↑
   │              └── BOTH hi = 0x0102 = project_work_item
   │                  → ONE RBAC grant lattice (project_role 0x0117)
   │                  → ONE ontology shape
   └───────────────── DIFFERENT lo u16 → DIFFERENT templates, zero concept dup

0x0103_0003 ─── WoA's Stundenzettel (billable_work_entry)
0x0103_0004 ─── SMB's Stundenzettel
0x0103_0001 ─── OpenProject's TimeEntry  (planner side)
0x0103_0007 ─── Redmine's TimeEntry      (planner side)
0x0103_0002 ─── Odoo's HrAttendance / account.move.line(qty=hours)
   ↑              ↑
   │              └── FIVE app-private renders of the same concept
   │                  "Planner times align with billable hours" =
   │                  a codebook lookup, not a translation layer.
   └───────────────── ALL share hi = 0x0103 = BILLABLE_WORK_ENTRY
                      → ONE canonical billable-time concept
                      → the cross-fork convergence pin (OGAR #93/#94/#96)
```

**APP prefix allocation** (committed; OGAR #95 §2):

| APP (lo u16) `0xAAAA` | App | Consumer crate |
|---|---|---|
| `0x0000` | shared core / no app skin | — |
| `0x0001` | OpenProject | openproject-nexgen-rs |
| `0x0002` | Odoo | odoo-rs |
| `0x0003` | WoA | woa-rs |
| `0x0004` | SMB | smb-office-rs |
| `0x0005` | Medcare / Healthcare | medcare-rs |
| `0x0007` | Redmine | (consumer TBD) |

**Domain bytes** (committed; the high byte of the hi u16):

| `0xDD` | Domain |
|---|---|
| `0x01` | project mgmt |
| `0x02` | commerce / ERP |
| `0x07` | OSINT |
| `0x09` | Health |

---

## §2. The four canonical consumer patterns

Every consumer code path is one of these four. Pattern-match before you write.

> **Two canonical paths — spine vs membrane.** Every pattern below has
> two forms based on the BBB-barrier: **spine-internal** crates
> (`lance-graph-*`, `ogar-vocab`, `ogar-ontology`, `ogar-class-view`)
> freely depend on `ogar-vocab` and use the typed `*Port::*` surface;
> **membrane / customer binaries** (`woa-rs`, `smb-office-rs`,
> `medcare-realtime`) are restricted to `lance-graph-contract` only and
> use the wire-compat mirror `lance_graph_contract::ogar_codebook::*`.
> Both return identical classids — the choice is about dep-tree posture,
> not concept identity. The barrier is enforced by each consumer's
> CLAUDE.md allow-list (e.g. woa-rs Iron Rule 1, smb-office-rs Iron
> Rule 3). Wire-compat is pinned by parity tests on the contract side.
>
> | Crate type | Allowed OGAR deps | Canonical lookup path |
> |---|---|---|
> | spine-internal | `ogar-vocab` · `ogar-ontology` · `lance-graph-*` | `ogar_vocab::ports::*Port` |
> | membrane / customer binary (BBB) | `lance-graph-contract` only | `lance_graph_contract::ogar_codebook` |

### Pattern 1 — pull a classid (the codebook lookup)

The canonical concept ID, via pure static function call. **No registry,
no hydration, no bridge.** Two paths — pick by your crate's BBB posture.

#### Pattern 1a — spine-internal (lance-graph-* + OGAR-internal)

```rust
// CANONICAL — direct PortSpec lookup, full typed-port surface
use ogar_vocab::ports::{HealthcarePort, PortSpec};

let cid: Option<u16> = HealthcarePort::class_id("Patient");
// → Some(0x0901)
```

When: your crate is INSIDE the spine — `lance-graph-*`, `ogar-vocab`,
`ogar-ontology`, `ogar-class-view`, or another OGAR-internal crate.

#### Pattern 1b — membrane / customer binary (BBB-barrier)

```rust
// CANONICAL — wire-compat mirror, BBB-safe (zero ogar-vocab dep)
use lance_graph_contract::ogar_codebook::canonical_concept_id;

let cid: Option<u16> = canonical_concept_id("patient");
// → Some(0x0901)
```

When: your crate is BEHIND the BBB-barrier (`woa-rs`, `smb-office-rs`,
`medcare-realtime`, any customer binary). Per the consumer's allow-list,
`lance-graph-ogar` / `ogar-vocab` are **forbidden** deps; you depend on
`lance-graph-contract` only. The contract's `ogar_codebook` mirrors the
canonical codebook wire-compat (zero-dep, parity-tested against OGAR per
the `canonical_concept_name` precedent — OGAR #98 — and the APP-prefix
mirror — lance-graph #592).

Both 1a and 1b return `0x0901`. The choice is which crate's **dep tree**
you're inside, not which classid you pull.

| Consumer | Port | Example call | Returns |
|---|---|---|---|
| medcare-rs | `HealthcarePort` | `::class_id("Patient")` | `Some(0x0901)` |
| medcare-rs | `HealthcarePort` | `::class_id("Befund")` (Dx alias) | `Some(0x0902)` |
| woa-rs | `WoaPort` | `::class_id("Stundenzettel")` | `Some(0x0103)` |
| woa-rs | `WoaPort` | `::class_id("TimeEntry")` (EN alias) | `Some(0x0103)` |
| smb-office-rs | `SmbPort` | `::class_id("Kunde")` | `Some(0x0204)` (BILLING_PARTY) |
| odoo-rs | `OdooPort` | `::class_id("res.partner")` | `Some(0x0204)` |
| odoo-rs | `OdooPort` | `::class_id("hr.attendance")` | `Some(0x0103)` |
| openproject-nexgen-rs | `OpenProjectPort` | `::class_id("WorkPackage")` | `Some(0x0102)` |
| (any Redmine consumer) | `RedminePort` | `::class_id("Issue")` | `Some(0x0102)` |

```rust
// ANTI — go through the deprecated bridge layer
use lance_graph_ogar::bridges::HealthcarePort;        // ← works, but extra hop;
                                                       //   migrate to ogar_vocab::ports

// ANTI — re-mint the canonical id locally
const PATIENT_CLASSID: u16 = 0x0901;                  // ← bypasses PortSpec;
                                                       //   loses the alias-table mapping

// ANTI — construct a UnifiedBridge to ask the same question
let b = MedcareBridge::new(registry)?;                // ← deprecated alias; round-trip via
let ent = b.entity("Patient")?;                       //   bridge + registry just to recover
let cid = ent.schema_ptr.entity_type_id();            //   what PortSpec gives in one call
```

### Pattern 2 — compose a render classid (concept ‖ APP prefix)

Stamp the per-app render prefix on the concept. Same spine-vs-membrane
split as Pattern 1.

#### Pattern 2a — spine-internal (OGAR #97 typed helper)

```rust
// CANONICAL — typed APP_PREFIX from the PortSpec
use ogar_vocab::ports::{HealthcarePort, PortSpec};

let cid: u16 = HealthcarePort::class_id("Patient").unwrap();   // 0x0901
let render_classid: u32 = ((cid as u32) << 16) | HealthcarePort::APP_PREFIX;
//                        0x0901_0000                        | 0x0005
//                      = 0x0901_0005   ← Medcare's patient render address

// → resolves to:
//     ClassView       = Medcare's clinical patient view (Askama template:
//                       patient.html, PII leaf-rename adapter, German labels
//                       stripped at the membrane)
//     Class           = canonical Patient (OGAR Core, shared with every health app)
//     ActionDef/      = canonical Healthcare lifecycle (NOT app-private —
//        KausalSpec     in Core, behavior shared)
```

Worked examples by app:

```rust
(0x0901u32 << 16) | HealthcarePort::APP_PREFIX   →  0x0901_0005  // Medcare patient
(0x0103u32 << 16) | WoaPort::APP_PREFIX          →  0x0103_0003  // WoA Stundenzettel
(0x0204u32 << 16) | SmbPort::APP_PREFIX          →  0x0204_0004  // SMB Kunde
(0x0103u32 << 16) | OdooPort::APP_PREFIX         →  0x0103_0002  // Odoo HrAttendance
(0x0102u32 << 16) | OpenProjectPort::APP_PREFIX  →  0x0102_0001  // OpenProject WorkPackage
(0x0102u32 << 16) | RedminePort::APP_PREFIX      →  0x0102_0007  // Redmine Issue
```

#### Pattern 2b — membrane (BBB-safe, per lance-graph #592)

```rust
// CANONICAL — one-call lookup + stamp via the contract mirror
use lance_graph_contract::{AppPrefix, render_classid_for_concept};

let render: Option<u32> = render_classid_for_concept(
    AppPrefix::Healthcare,
    "patient",
);
// → Some(0x0901_0005)
```

Or split (pull then stamp), for symmetry with Pattern 1b:

```rust
use lance_graph_contract::ogar_codebook::{canonical_concept_id, AppPrefix};

let cid: u16 = canonical_concept_id("patient").unwrap();   // 0x0901
let render: u32 = AppPrefix::Healthcare.render(cid);       // 0x0901_0005
```

`AppPrefix` is the OGAR #95 §2 allocation table mirrored into the
contract as typed data (lance-graph #592 closed `ISS-CONTRACT-APP-PREFIX-MIRROR`,
following the OGAR #98 `canonical_concept_name` mirror precedent).
Parity is pinned: the contract's `app_prefixes_match_ogar_allocation_table`
test fires the moment OGAR re-allocates a prefix. **The membrane never
hand-stamps `0x000N`** — both halves come from one source.

Worked examples (mirror of 2a, via the contract):

```rust
AppPrefix::Healthcare.render(0x0901)   →  0x0901_0005  // Medcare patient
AppPrefix::Woa.render(0x0103)          →  0x0103_0003  // WoA Stundenzettel
AppPrefix::Smb.render(0x0204)          →  0x0204_0004  // SMB Kunde
AppPrefix::Odoo.render(0x0103)         →  0x0103_0002  // Odoo HrAttendance
AppPrefix::OpenProject.render(0x0102)  →  0x0102_0001  // OpenProject WorkPackage
AppPrefix::Redmine.render(0x0102)      →  0x0102_0007  // Redmine Issue
```

```rust
// ANTI — hardcode the APP prefix as a magic constant
const MEDCARE_APP: u32 = 0x0005;                        // ← drifts from PortSpec
let render = ((cid as u32) << 16) | MEDCARE_APP;        //   if APP allocation changes

// ANTI — bit-shift inline
let render = ((cid as u32) << 16) | 0x0005u32;          // ← un-typed; lose source-of-truth

// ANTI — store full u32 render classid where hi u16 would do (RBAC, ontology)
fn authorize(actor: &Actor, render_cid: u32, op: Op) { … }
//                                  ^^^^^^^^^^^^ shared grant lattice keys on HI u16;
//                                               passing the full u32 leaks render lens
//                                               into auth (concept is shared, render is not)
```

### Pattern 3 — authorize by classid (PENDING the keystone)

The keystone `authorize(actor, classid, op)` is **[H]** and gated on
`PROBE-OGAR-RBAC-AUTHORIZE`. Until it ships, **keep your existing auth**
— do NOT re-introduce a bridge as a stopgap.

```rust
// FUTURE — once lance-graph-rbac keystone ships:
use lance_graph_rbac::authorize;

let concept: u16 = HealthcarePort::class_id("Patient").unwrap();   // 0x0901
let decision = authorize(&actor, concept, Op::Read);
//                                ^^^^^^^ KEY ON HI u16: shared grant lattice
//                                        across all health apps
```

```rust
// INTERIM — keep existing static_role / Policy / membrane gate
fn authorize_patient_read(actor_role: &str) -> AccessDecision {
    let role_static = static_role(actor_role);
    // medcare-rbac::Policy check, OR MedCareMembraneGate, OR static role map
    // — whatever your repo already uses
    …
}
```

```rust
// ANTI — re-introduce a UnifiedBridge to "auth gate" while waiting for keystone
let bridge = MedcareBridge::new(registry)?;            // ← deprecated; replaces one
let decision = bridge.authorize_read("Patient", &role); //  trap with another. Wait
                                                        //  for the real keystone.
```

### Pattern 4 — distill/migrate from a legacy bridge

The migration recipe in two shapes: import-only repoint (cheap) vs
structural drop (real spell). Pick by what your code actually uses.

**Sub-pattern 4a — import-only repoint** (lowest blast radius):

```rust
// BEFORE:
use lance_graph_ogar::bridges::HealthcarePort;
                   ^^^^^^^ deprecated re-export path; lights up the beacon

// AFTER (canonical):
use ogar_vocab::ports::HealthcarePort;
//  ^^^^^^^^^^^^^^^^^ direct from the typed PortSpec source
```

Same symbol; cleaner import path; no behavior change. This is the
**one-line spell**.

**Sub-pattern 4b — structural drop** (the full migration):

```rust
// BEFORE — UnifiedBridge<PortBridge> wraps the lookup:
use lance_graph_callcenter::UnifiedBridge;
use lance_graph_ogar::bridges::MedcareBridge;          // deprecated alias
let bridge = UnifiedBridge::<MedcareBridge>::new(…);
let cid = bridge.entity("Patient")?.schema_ptr.entity_type_id();

// AFTER — direct PortSpec call, no bridge:
use ogar_vocab::ports::{HealthcarePort, PortSpec};
let cid = HealthcarePort::class_id("Patient").unwrap();   // same value, no wrapper
```

If your consumer has a `per-consumer-bridge` crate (`medcare-bridge`,
`smb-bridge`, etc.) — per `CONSUMER-MIGRATION-HOWTO.md` it gets
**deleted** at the end of the migration. The consumer holds **no**
ontology.

---

## §3. Anti-pattern catalogue — drill these alongside the right shapes

Each anti-pattern is paired with its right shape above. Memorizing the
shapes negatively is half the muscle memory.

| Anti-pattern | Right shape (§reference) | Why it bites |
|---|---|---|
| Re-mint a canonical classid locally (`const PATIENT = 0x0901`) | §2 Pattern 1 | Bypasses PortSpec — loses alias table, alias-to-id stability is a per-Port guarantee |
| Hardcode `APP_PREFIX` as a literal (`0x0005`) | §2 Pattern 2 | Drifts from the typed source; APP allocation changes break silently |
| Pass full u32 render classid to authorize() | §2 Pattern 3 (note) | Auth keys on HI u16; passing full u32 leaks render lens into grant lookup |
| Use the deprecated `lance_graph_ogar::bridges::*` re-export | §2 Pattern 4a | The beacon will warn (lance-graph #589/#590); canonical path is `ogar_vocab::ports` |
| Re-introduce `UnifiedBridge<PortBridge>` as a stopgap while keystone is pending | §2 Pattern 3 (anti) | Replaces one deprecated wrapper with the same wrapper. Wait for the real authorize |
| Smuggle behavior into DDL via `DEFINE EVENT … WHEN … THEN …` | `SURREAL-AST-TRAP-PREFLIGHT.md` | Behavior belongs in OGAR Core (`ActionDef` + `KausalSpec`), not in the address-side artifact |
| Mint a per-tenant high-u16 to dodge sharing the canonical class | §0 corollary 1 | The point of the hi u16 is sharing; per-tenant variation lives in the LO u16 (render lens) |

---

## §4. Worked migration narrative — Medcare's patient end-to-end

To cement: the same patient concept seen through every pattern.

```rust
// The address — pure routing identity
let concept_cid: u16 = HealthcarePort::class_id("Patient").unwrap();   // 0x0901
let render_cid:  u32 = ((concept_cid as u32) << 16) | HealthcarePort::APP_PREFIX;
//                   = 0x0901_0005

// The address resolves to (no magic on the address itself):
//
//   render_cid (0x0901_0005)
//      │
//      ├──► ClassView::resolve(0x0901_0005)  →  Medcare's patient view
//      │                                          (template: patient.html,
//      │                                           PII leaf-rename at adapter,
//      │                                           German labels stripped at membrane)
//      │
//      ├──► Class::canonical(0x0901)         →  Patient (OGAR Core, shared)
//      │                                          (attributes: name, dob, mrn, …;
//      │                                           associations: diagnoses[], visits[])
//      │
//      └──► ActionDef::for_class(0x0901)     →  [validate_mrn_unique,
//                                                  before_save_audit,
//                                                  after_destroy_archive, …]
//                                                (canonical Healthcare lifecycle —
//                                                 ALL apps share these, lo u16 doesn't matter)

// Pattern 1: pull the classid
let cid = HealthcarePort::class_id("Patient").unwrap();

// Pattern 2: compose the render
let render = ((cid as u32) << 16) | HealthcarePort::APP_PREFIX;

// Pattern 3: authorize (interim — keystone pending)
let decision = static_role_check(actor, "physician");   // existing medcare-rbac path
//             ^^^^^^^^^^^^^^^^^ NOT a UnifiedBridge — that's the anti-pattern

// Pattern 4: render — emit the view through the right template
let html = render_classview(render_cid, &patient_row)?;  // ClassView dispatch on full u32
```

This is the whole shape of a clean consumer call site. Every consumer
that resolves a Patient (or a WorkPackage, or a Stundenzettel) does
the same four-step dance with different ids.

---

## §5. When this doc fires (Knowledge Activation)

**Trigger phrases** — if your prompt contains any of these, this doc
is mandatory pre-read:

`classid` · `class_id(` · `APP_PREFIX` · `PortSpec` · `HealthcarePort` ·
`WoaPort` · `SmbPort` · `OdooPort` · `OpenProjectPort` · `RedminePort` ·
`ClassView` · `MedcareBridge` · `WoaBridge` · `SmbBridge` · `OdooBridge` ·
`OpenProjectBridge` · `RedmineBridge` · `UnifiedBridge` · `ogar_vocab::ports` ·
`lance_graph_ogar::bridges` · `render classid` · `concept classid` ·
`per-consumer bridge` · `consumer migration` ·
**BBB-barrier** · `contract::ogar_codebook` · `canonical_concept_id` ·
`AppPrefix` · `render_classid_for_concept` · `lance_graph_contract::ogar_codebook` ·
`membrane consumer` · `spine vs membrane`

**Agents that load it Tier-1**:
- `core-first-architect` · `adapter-shaper` · `core-gap-auditor`
- The `Plan` subagent on any consumer-side task
- Any per-consumer agent specifically (medcare-bridge specialist,
  woa-rs-rust transcoder, etc.)

## §6. Cross-references

- `docs/APP-CLASS-CODEBOOK-LAYOUT.md` — the formal layout spec (with §3.5–3.7 render/RAG/Firewall logic)
- `docs/CONSUMER-MIGRATION-HOWTO.md` — the per-consumer migration recipe
- `docs/SURREAL-AST-TRAP-PREFLIGHT.md` — the trap to avoid (don't smuggle magic into address-side DDL)
- `docs/CLASSID-RBAC-KEYSTONE-SPEC.md` — the pending `authorize(actor, classid, op)` keystone
- `docs/OGAR-AST-CONTRACT.md` — the canonical Class / ActionDef IR (what the address resolves to)
- `docs/APP-CODEBOOK-MIGRATION-PLAN.md` — the wave-ordered consumer migration plan (W0–W4)
- **Parallel-session sibling**: `lance-graph/.claude/knowledge/ogar-consumer-preflight.md`
  (lance-graph #591) — the spine-side spellbook that surfaced the
  `ISS-CONTRACT-APP-PREFIX-MIRROR` Core-gap and pinned the
  spine-vs-membrane BBB-barrier framing now reflected in §2.
- **Membrane mirror landed**: lance-graph #592 — `lance_graph_contract::ogar_codebook`
  now carries `AppPrefix`, `render_classid_for_concept`,
  `classid_app_prefix`, `classid_concept` (mirror of OGAR #97
  `ogar_vocab::app`, no `ogar-vocab` dep). This is what Pattern 1b/2b
  call into; the parity test
  `app_prefixes_match_ogar_allocation_table` fuses drift against OGAR
  `PortSpec::APP_PREFIX`.
