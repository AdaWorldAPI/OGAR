# a2ui-rs — Screen Addressing over ClassView — PROPOSAL (council pending)

> **Status:** PROPOSAL — spec detailed enough that a council VERIFIES rather
> than redesigns; deviations require a numbered factual error, not taste.
>
> **Ask (operator, 2026-07-14):** *"prepare for some sort of Citrix via same
> desktop addressing via classview and widefieldmask, via askama/jinja and
> using nested classview for address and ERB pattern fieldview classview for
> rendering from memory without serialization … Argon2 etc for the rdp-2graph
> … not pushing pixels but instead addressing the screen … using
> AdaWorldAPI/A2UI and transcode it to AdaWorldAPI/a2ui-rs as an ogar target
> for desktop addressing/rendering via classview using the a2ui pattern but
> reimagined with our optimizations."*
>
> **One sentence:** the remote desktop stops pushing pixels and starts
> **addressing the screen** — down the wire go `(GUID key 16 B, WideFieldMask
> delta, changed LE values)`, up the wire go `ActionInvocation`s; the client
> holds the ClassView/template codebook and renders locally from borrowed
> memory (askama, zero serialization) — **the ClassView registry is the font
> of the desktop**: you don't stream glyph rasters, you reference codepoints.

## C0. The convergence this closes

A screen and a document are the SAME positional projection — the doc-layer
amendment A3 already ruled it for documents (template = ClassView ×
WideFieldMask; Kopfzeile/main/Fußzeile = positions in the ordered set, same
brick as the Klickwege menu-quad). A live desktop is that projection with a
heartbeat. The `DocRenderer` adapter family gains its fourth member:

| adapter | output |
|---|---|
| tesseract PDF | raster + text layer |
| Spire.Doc | Word |
| askama | data-only PDF/HTML |
| **a2ui (this proposal)** | **live interactive surface** |

One template projection; re-issue a document as PDF or serve it as a living
screen — same mechanism, different renderer.

## C1. Nested ClassView = the screen address (numbered claims)

1. **Hierarchy:** desktop → window → region → widget is nested ClassView
   projection — a widget is a field position in its parent's ordered set,
   exactly as a doc-IR `Region` carries `children`. The `X:Y` u8:u8 rail on
   the 256×256 tile is the layout address (the same rail documents and
   Klickwege surfaces use). No new addressing scheme.
2. **Wire-truth (the Firewall, ADR-022/023):** the hot path carries LE bytes
   — `key(16 B) + mask delta + changed values` — never JSON, never protobuf.
   Protobuf/JSON exist only at a browser membrane if a non-wasm client needs
   them. The render reads the SoA row **in place**: askama borrows the
   struct; nothing serializes in order to draw. That is the operator's "ERB
   pattern fieldview ClassView rendering from memory without serialization."
3. **The thin-client question, answered:** the client must hold the
   ClassView/template codebook (amortized once, D-AMORT — like a font). With
   templates resident, the *same Rust* (ClassView resolve + askama render)
   compiles to **wasm** — so the browser IS the thin client; a bespoke native
   client is a later option, not a prerequisite. Hardware acceleration =
   wasm + canvas/webgpu on the client's own silicon — which is exactly what
   "don't push pixels" buys: the server never rasterizes at all.
4. **RBAC by projection (the security headline):** what leaves the server is
   `WideFieldMask ∩ role-mask` (`FieldMask::intersect`, the existing RBAC
   brick from the SoC/dedup pipeline). A field the role may not see is not
   *hidden on the client* — it is **absent from the wire**. Pixel streams
   cannot make that guarantee; address streams get it for free.
   **CORRECTION (codex P2 on #204, verified in-repo):** as merged, this
   claim silently breaks for surfaces with >64 fields — the RBAC seam's
   `ClassRbac::field_mask` returns a NARROW `FieldMask` (u64;
   `CLASSID-RBAC-KEYSTONE-SPEC.md` §4), while the wide renderer exists
   precisely because surfaces exceed `FieldMask::MAX_FIELDS`
   (`ogar-render-askama::render_class_with_methods_wide`, whose
   `WideFieldMask::full_for` "emit everything" sentinel is exactly the
   unsafe fallback). Three requirements make the claim hold, all MANDATORY
   for W1+:
   (a) **Retype in place — `WideFieldMask` already exists (operator
   correction, 2026-07-14):** NO parallel `field_mask_wide` method.
   `ClassRbac::field_mask` changes its return type to [`WideFieldMask`]
   directly — verified cheap: the entire production surface is the trait
   DEFAULT (`FieldMask::FULL`, rbac.rs:176) + one test override
   (rbac.rs:455); no production impl overrides it, so the retype touches
   two bodies and the compiler surfaces everything else. `WideFieldMask`
   is already the universal mask (`Small` repr ≤64, self-promoting past
   64) — a second method name for the same projection would be the
   two-vocabularies anti-pattern (T1) applied to the seam itself.
   (b) **The permit-all identity is the ONE W1 decision:** `WideFieldMask`
   has `EMPTY` + `full_for(count)` but no count-free permit-all, and the
   axis-4 default is deliberately "FULL unless configured" (RBAC opt-in).
   Either `WideFieldMask` gains `ALL` (the intersection identity) or the
   default derives `full_for(field_count)` from the ClassView. W1 decides;
   the opt-in default semantics survive either way.
   (c) **Zero-extension rule (fail-closed) for legacy narrow interop:**
   wherever an existing narrow `FieldMask` (SoC/dedup bricks) meets a wide
   surface, the narrow mask extends past bit 63 as ZEROS, never as full.
   And the sentinel ban: `full_for` as a RENDER convenience is fine; as an
   RBAC fallback it is forbidden — a missing role mask is a refusal, not a
   full grant.
5. **rdp-2graph security = reuse, no new crypto:** session KDF via
   `ogar-auth` (Argon2 — the MedCare Tier-1 password/totp home), transport
   via `ogar-encryption` (the single generic encryption surface). The
   session is a capability over classid ranges + role mask.
6. **Events upstream are live Klickwege:** a click IS a `navigates_to` /
   `ActionInvocation` edge — the SAME closed vocabulary the C# harvest
   emits statically becomes the runtime event stream. Interaction telemetry
   lands as graph edges with zero new vocabulary.

## C2. The A2UI transcode — what carries over, what is replaced

The `AdaWorldAPI/A2UI` fork (cloned + verified 2026-07-14): the upstream
pattern is *"agents speak UI"* — declarative intent, client renders natively;
`renderers/{react,angular,flutter,lit,web_core,markdown}` are the client
precedents; `blueprints/`, `agent_sdks/`, `eval/` the ecosystem.

**Prior art IN the fork, regraded:** `proto/a2ui/hamming.proto` (commit
`28ed91b`, "gRPC bitpacked Hamming protocol … thin client transport") is an
earlier substrate-flavored pass — 1,250-byte fingerprint frames, XOR deltas,
BindSpace/ladybug-compatible. That is the **pre-V3 carrier era** (the
singleton-BindSpace/fingerprint-as-carrier model the V3 rulings retired).
**Regrade, don't delete:** the SERVICE SHAPE survives — `RenderStream`
(server→client surface), `ActionStream` (client→server actions),
`SyncCodebook`/`PushCodebookDelta` (template/codebook amortization),
session init with codebook version — while the PAYLOAD is replaced by the
V3 facet: 16 B key + WideFieldMask delta + LE values instead of 1,250 B
fingerprint frames. (~78× smaller per addressed node, and the key
prerenders layout with zero value decode — the P0 canon doing the work.)

| A2UI pattern (kept) | reimagined payload (V3) |
|---|---|
| declarative component tree | nested ClassView projection (classid-addressed) |
| stream whole tree / JSON patches | WideFieldMask deltas + changed LE values |
| inline actions/callbacks | `ActionDef` refs, invoked as `ActionInvocation` (C3 trap) |
| per-framework renderers | ONE wasm fieldview client (askama) + membrane adapters |
| codebook sync (hamming.proto) | ClassView/template registry sync (same RPC shape) |

## C3. Traps (named before they bite)

- **T1 — component-vocabulary fork:** A2UI's Card/List/TextField vocabulary
  must NOT become a second closed vocabulary beside the doc-IR RegionKinds.
  Widget *skins* are ClassView/template side (render, lo-u16 world);
  structural kinds stay ONE closed vocabulary. A "new widget" is a template,
  never an enum variant.
- **T2 — behavior in the tree (the SURREAL-AST trap, UI edition):** A2UI
  callbacks/handlers must never ride inline in the surface. Behavior is
  `ActionDef` on the Core node; the surface carries only the *address* of an
  action. `DEFINE EVENT` in DDL and `onClick: <lambda>` in a component tree
  are the same hijack.
- **T3 — a serialization layer creeping back:** the moment the "protocol"
  grows a JSON/proto struct in the server→wasm hot path, the Firewall is
  breached. LE bytes end-to-end; wasm ingests them zero-copy.

## C4. The killer probe — re-host a MedCare screen (the transpile endgame)

Everything needed already exists in the harvest: 106 screens with
screen→concept bindings (Klickwege golden v2), 162 `CompiledClass` + 2,748
`ActionDef`s (the corpus), ClassView + `ogar-render-askama`.
**P-REHOST:** pick ONE harvested MedCare form; project its `CompiledClass`
through ClassView × WideFieldMask into an askama template; serve it as an
addressed surface; fire one of its harvested `ActionDef`s round-trip.
GREEN = the legacy app's screen, re-rendered from the graph, **no WinForms**
— harvest the app → re-render the app. That is what "Citrix via desktop
addressing" concretely means, and it is the transpile substrate's endgame
demo.

## C5. Waves

- **W0 — repo + regrade (operator gates):** mint `AdaWorldAPI/a2ui-rs`;
  regrade `hamming.proto` in the A2UI fork (keep shape, mark payload
  pre-V3). Council verifies THIS spec.
- **W1 — surface contract (canon-free, in OGAR):** the addressed-surface
  types — nested ClassView address, WideFieldMask delta frame, ActionInvocation
  ref — serde-only, mirroring `ogar-doc-ir` discipline (closed vocab, load
  gate, version marker).
- **W2 — a2ui-rs transcode:** the service shape (RenderStream/ActionStream/
  codebook sync) reimplemented over the W1 frames; membrane adapters
  (JSON/proto) only at the edge.
- **W3 — wasm fieldview client:** ClassView resolve + askama render compiled
  to wasm; LE ingest zero-copy; canvas/webgpu paint.
- **W4 — P-REHOST** (C4) — the gate before any scale-out.
- **W5 — rdp-2graph session:** ogar-auth Argon2 KDF + ogar-encryption
  transport + role-mask projection (C1.4) + upstream Klickwege events.

## C6. Non-goals (v1)

- No pixel-codec fallback (that's what we're replacing).
- No new template DSL (askama IS the fieldview; jinja parity later if a
  non-Rust server ever needs it).
- No new crypto (ogar-auth/ogar-encryption only).
- No canon mints — widget skins are templates, not concepts; if a genuine
  UI *concept* ever needs a classid, it goes through the codebook like any
  other mint, council-gated.
- No gRPC/protobuf in the hot path (membrane only).

## C7. DISCOVERY-MAP entry (mandatory)

Append `D-A2UI-SCREEN-ADDRESSING`: screens are addressed, not rasterized —
nested ClassView × WideFieldMask down, ActionInvocation up, client holds the
template codebook (the font of the desktop), RBAC by mask-intersection
projection, hamming.proto regraded pre-V3 (shape kept, payload replaced).
Grade `[S]` until the council + P-REHOST promote it.
