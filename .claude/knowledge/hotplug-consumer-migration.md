# Hot-Plug Consumer Migration — the generic plug-and-play pattern

> READ BY: any session migrating a consumer (woa-rs, medcare-rs,
> smb-office-rs, odoo-rs, tesseract-rs, osm-website-rs, …) onto the
> OGAR-authoritative capability surface; any session adding a new
> authoritative domain table (thinking-styles best practices, …);
> integration-lead / baton-handoff-auditor before any cross-repo
> capability wiring.
>
> Shipped 2026-07-07 (operator-ruled same day). Reference PRs:
> OGAR #174 (ocr_actions) + #175 (resolve_hotplug), lance-graph #658
> (hotplug socket + OgarAuthority + CI fuse gate), tesseract-rs #13/#14
> (executor + HOT_PLUG migration — the template consumer).

## The model — three roles, three homes, one binary

Everything links into ONE binary (lance-graph + OGAR + consumer +
ndarray). Nothing serializes; every check is a compile-time const or a
test in the consumer's own suite. "Wenn's knallt, dann einmal — nicht
200 Pins monitoren."

| Role | Home | Surface |
|---|---|---|
| **SOCKET** (agnostic, zero-dep) | `lance_graph_contract::hotplug` | `HotPlug { consumer, classids, covered }`, `Activation`, `ActivationDrift`, trait `CapabilityAuthority` |
| **AUTHORITY** (declares + resolves) | OGAR `ogar_vocab` | domain action tables (e.g. `ocr_actions`) + `capability_registry::{domain_tables, resolve_hotplug, HotplugDrift}` |
| **BRIDGE** (socket ⇄ authority) | `lance-graph-ogar` (workspace-EXCLUDED) | `OgarAuthority: CapabilityAuthority`; also owns COUNT_FUSE + per-entry mirror parity + the roundtrip green light |
| **CONSUMER** | your repo | ONE `HOT_PLUG` const + ONE activation test + your executor |

The classid is the join key on BOTH sides: the consumer says "these
canon-high ids are hot", the authority returns BOTH the vocab rows and
every capability whose subject is one of those ids.

## Migration recipe (per consumer, ~1 hour)

1. **Authority side (OGAR PR #1):** declare your domain's action table
   in `ogar-vocab` next to `ocr_actions` — one `ActionDef` per
   capability on YOUR already-minted canon-high concepts (mint via the
   normal codebook process if missing; hi u16 = concept, lo u16 = APP
   render prefix, NEVER a shape ordinal). Export `<DOMAIN>_ACTION_NAMES`
   (const fingerprint), `<DOMAIN>_SUBJECT_CLASSIDS`,
   `<DOMAIN>_EXPECTED_EXECUTORS = ["<your-crate>"]`. Register ONE entry
   in `capability_registry::domain_tables()` mapping
   `(capability, subject classid)`. That's the whole authority change.
2. **Consumer side:** add path deps (NO PINS — local siblings):
   `ogar-vocab = { path = "<rel>/OGAR/crates/ogar-vocab" }` and
   `lance-graph-contract = { path = "<rel>/lance-graph/crates/lance-graph-contract" }`.
3. Declare the plug:
   ```rust
   pub const HOT_PLUG: lance_graph_contract::hotplug::HotPlug =
       lance_graph_contract::hotplug::HotPlug {
           consumer: "<your-crate>",
           classids: ogar_vocab::<domain>_actions::<DOMAIN>_SUBJECT_CLASSIDS,
           covered: COVERED_CAPABILITIES,   // your executor's arms
       };
   ```
4. Close the loop with ONE test:
   ```rust
   let (concepts, capabilities) = ogar_vocab::capability_registry::resolve_hotplug(
       HOT_PLUG.consumer, HOT_PLUG.classids, HOT_PLUG.covered,
   ).expect("hot-plug drifted from the authoritative OGAR tables");
   ```
   (Or through the socket: `OgarAuthority.activate(&HOT_PLUG)` via
   `dyn CapabilityAuthority` when you want authority-agnostic code.)
5. **CI:** your workflows need the sibling checkouts (OGAR +
   lance-graph), same as tesseract-rs `rust.yml` / lance-graph
   `rust-test.yml`. Sibling layout: check the repo out into a subdir
   (`path: <repo>`) so `path: OGAR` lands as a true sibling.

## The drift arms (each one named bang, test-time, in YOUR binary)

| Arm | Meaning |
|---|---|
| `UnknownClassid(id)` | plugged id not minted in the codebook |
| `NoCapabilitiesFor(id)` | id resolves to no capability — the table/ontology was forgotten |
| `UnexpectedConsumer` | you are not in the table's expected-executor list |
| `Uncovered(cap)` | authority declares it, your executor has no arm |
| `Undeclared(cap)` | your executor claims it, authority doesn't declare it |

Plus, CI-side (lance-graph `rust-test.yml`): COUNT_FUSE + per-entry
mirror parity + the roundtrip green light run against the real OGAR
sibling on every push.

## What NOT to do (each burned once, 2026-07-07)

- **No bespoke per-consumer plug crate/mechanism.** The registration
  surface is the `HOT_PLUG` const + one test. (tesseract-ogar's first
  `CapabilityRegistration` draft was replaced for exactly this.)
- **No path/optional deps on `lance-graph-contract` toward OGAR.** The
  contract is a workspace MEMBER; any path dep there (even optional) is
  resolved at workspace-load time and kills every CI cargo invocation.
  The contract stays zero-dep; armed things live in excluded crates.
- **No shape ordinals in the classid low u16.** Low half = APP render
  prefix (`PortSpec::APP_PREFIX`). Distinct shapes are distinct hi-u16
  concepts or ClassView payload readings.
- **No ontology payload in lance-graph.** lance-graph carries the wire
  mirror + the green light, nothing else. Authority checks live in OGAR.
- **No git deps on OGAR.** git+branch always writes a rev pin into
  Cargo.lock. Path deps to the sibling, always.

## Future synergies (evaluated 2026-07-07 — next arcs, not shipped)

1. **ActionDef plug-and-play from ruff (HIGH, near-term).**
   `ogar-from-ruff::lift_actions` already lifts ActionDefs from
   Rails/Python model graphs — the OCR table is hand-declared only
   because tesseract has no AR source. The generic path: ruff harvest
   (`ruff_*_spo`) → fuzzy-recipe codebook (`(verb, criteria)` recipes,
   ruff `.claude/knowledge/fuzzy-recipe-codebook.md`) → `lift_actions` →
   the domain table auto-derives instead of being hand-written; a
   consumer migration then starts with a harvest, not an authorship
   session. Missing piece: a C++ arm for `lift_actions` (ruff_cpp_spo
   has walk_tu/walk_free_functions/walk_enums; an action-lift over the
   method-body facts is the natural fourth arm) and a
   `derive_domain_table!`-style bridge from lifted ActionDefs into
   `domain_tables()`.
2. **Param-enum fidelity via `walk_enums` (MEDIUM).** Capability params
   that are C/C++ enums (PSM modes, connectivity 4/8, …) can carry
   their variants from the ruff enum harvest instead of free-form
   names — the same shapes-from-ruff discipline the dawg/dict arc used.
3. **Ontology plug-and-play (MEDIUM, design needed).** Today
   `resolve_hotplug` returns vocab rows + capability names. The same
   join can return ONTOLOGY fragments per classid: OGAR's
   `vocab/exports → OGIT promote → vocab/imports` pipeline already
   stages TTL per domain, and `ogar-adapter-ttl` renders it. A
   `resolve_hotplug_with_ontology` returning the OGIT NTO fragment
   references for the plugged classids would let a consumer pull
   concepts + actions + ontology in one activation — still zero
   serialization in-binary; the TTL stays an OGAR-internal artifact
   (imports/ tier), never hauled through lance-graph. Gate: needs the
   imports/ tier populated for the domain (ontologies, not harvests —
   harvests stay in the consumer repo).
4. **Thinking-styles best practices (HIGH, planned all along).** One
   `domain_tables()` entry + a `<STYLES>_EXPECTED_EXECUTORS` list arms
   the thinking-style table for whichever engine executes styles —
   same recipe as step 1 above, zero new machinery.
