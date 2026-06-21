# Redmine Integration Plan — width × depth, parallel-scopable

> **Reality check, calcified.** The Northstar plan (`CLASSVIEW-MATERIALIZATION-PLAN.md`)
> built the substrate. The Redmine harvest (`REDMINE-QUERY-HARVEST.md`)
> proved 17 years of Redmine GUI evolution maps onto our askama kit
> 1:1. This doc is the **wiring plan** — how to deliver an actually-running
> Redmine-shaped web app on top of that substrate, scoped so multiple
> contributors / sessions can work in parallel without stepping on each
> other.

## 0. TL;DR — the picture

```text
┌─────────────────────────────────────── DONE (the foundation) ───────────────────────────────────────┐
│                                                                                                       │
│   ogar-vocab          ogar-render-askama       op-codegen-projection       UnifiedBridge<P>          │
│   (codebook +         (HtmlListView /          (Class → SurrealQL DDL      (registry + g_lock,       │
│    Class fns +        HtmlDetailView /         via askama;                  codebook-aware           │
│    ports::PortSpec)   HtmlForm + cells +       byte-identical pinned)       entity() override)        │
│                       formatters)                                                                     │
│                                                                                                       │
└───────────────────────────────────────────────┬───────────────────────────────────────────────────────┘
                                                │
                                                ▼  this PR's scope
┌────────────────────────────────────── TODO (the wiring plan) ────────────────────────────────────────┐
│                                                                                                       │
│   rm-server     →  rm-handlers  →  ogar_render_askama::render_*  →  HTML to browser                  │
│                ↑                  ↑                                                                   │
│                │                  └─ Redmine 17-yr UX inherited                                       │
│                ▼                     for free (Query/QueryColumn                                      │
│            rm-store                  → RenderColumn/ColumnKind)                                       │
│            (SurrealDB                                                                                 │
│             via from_class)         rm-auth (session + RBAC)                                          │
│                                                                                                       │
└───────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

**The bet:** per-resource handler code is **mechanical** because the
render kit takes `(canonical Class, Vec<RowSource>) → HTML` and Redmine's
17 years of UX learnings are already inside the kit as data
(`ColumnKind`, default formatters, sort/group/total flags). Adding a new
resource page = a `for col in default_columns(class)` loop, not a new
template.

## 1. The four axes — orthogonal dimensions of progress

Every workstream below positions itself on these four axes. Mixing them
is how parallel scoping works; conflating them is how scope creeps.

| Axis | What it varies | Where progress shows up |
|---|---|---|
| **Width** | how many canonical concepts have functional pages | URL count: `/issues`, `/projects`, `/time_entries`, …  |
| **Depth** | how complete one concept's experience is | per-resource feature: list → +filter → +sort → +CSV → +saved-query → +bulk-edit |
| **GUI** | the visual layer (CSS, JS, theming, layout chrome) | what the page looks like, with NO behaviour change |
| **Templates** | the askama kit itself (new `ColumnKind`s, new formatters, new `InputKind`s) | OGAR-side; cross-port (every consumer benefits) |
| **Wiring** | the glue: routes, store, middleware, auth, sessions | `rm-server` scaffolding, then bound to handlers |

**Width × Depth = the resource × feature matrix.** Filling a row = one
resource gets every feature. Filling a column = every resource gets one
feature. The plan below makes both shapes parallel-friendly.

## 2. Phase 0 — Foundation (sequential gate, ~1 week, 1 stream)

The foundation is the only **sequential** part. Everything else fans out
parallel after this lands.

### W0.1 — `rm-server` scaffold
- New crate in `redmine-rs/crates/rm-server`.
- Deps: `axum`, `askama`, `askama_axum`, `tokio`, `tower-http` (cors,
  trace, compression), `tower-cookies` (sessions).
- One main.rs: `serve()` fn that wires the router, middleware stack
  (trace → compression → cookies → CSRF → auth), and static-file fallback.
- Hello-world handler at `/` that renders an empty `HtmlListView` for
  `class_ids::PROJECT` — proves the askama_axum bridge works.
- **DoD:** `cargo run -p rm-server`, visit `localhost:3000/`, see a page.

### W0.2 — `rm-store` substrate
- New crate `redmine-rs/crates/rm-store`.
- Connect to SurrealDB (in-memory for dev, file-backed for prod).
- Apply the schema emitted by
  `op_codegen_projection::render_classes_schema(&ogar_canonical_classes())`
  at startup. (The schema generator is already a sibling crate; we just
  drive it.)
- `CRUD<T>` trait: `find_one`, `find_all`, `find_by_filter`, `insert`,
  `update`, `delete`. Generic over a `T: PortSpec`-derived row type.
- Seed fixture: at minimum 3 Projects, 10 Issues, 5 TimeEntries —
  enough for the first list page to render with realistic content.
- **DoD:** `rm_store::Store::open()` returns a connected instance with
  schema applied; integration test inserts an Issue, reads it back.

### W0.3 — `rm-auth` core
- New crate `redmine-rs/crates/rm-auth`.
- Session middleware over signed cookies (HS256 or similar).
- `Login` form + handler (the only handler in this crate).
- `current_user(req)` extractor.
- **Defer:** RBAC enforcement; just expose the User, downstream
  handlers gate later (see D3).
- **DoD:** `POST /login` with seed-user credentials sets session cookie;
  `GET /me` returns the username.

**Dependency:** W0.1 (server) before W0.2 (store can register routes for
seed-import endpoint), before W0.3 (auth needs the server's
middleware-builder API). Single contributor, sequential, ~1 week total.

## 3. Phase 1 — Width pass (parallel after Phase 0, N streams)

Every canonical concept gets a **list page** and a **detail page**.
Forms come in Phase 2 (depth pass). This phase is breadth-first: ship
26 concepts at depth-1 before any single concept hits depth-5.

### The width track — one stream per resource group

| Track | Resources | Owner-shaped | Effort/track |
|---|---|---|---|
| **W1 — Work item core** | Issue (`project_work_item`), Journal | hot-path UX | 5 days |
| **W2 — Project core** | Project, Version, EnabledModule | container | 4 days |
| **W3 — Time tracking** | TimeEntry, Activity (via PortSpec extension) | reports | 4 days |
| **W4 — Actors & access** | User, Member (Membership), Role, MemberRole, Watcher | admin | 4 days |
| **W5 — Taxonomy** | Tracker, IssueStatus, IssuePriority, CustomField, CustomValue | admin | 4 days |
| **W6 — Comms** | Comment, News, Message, Board (Forum), WikiPage, Attachment | content | 5 days |
| **W7 — SCM-light** | Repository, Changeset (read-only; no Git driver yet) | dev-facing | 3 days |
| **W8 — Saved views & rels** | Query, IssueRelation | power-user | 3 days |

**Shape of one width track** (this is what makes them parallelizable):

```rust
// rm-handlers/src/issue.rs — pattern repeats per resource
pub async fn list(
    State(store): State<Arc<Store>>,
    Query(q): Query<ListQuery>,
) -> Result<askama_axum::Response, AppError> {
    let class = ogar_vocab::project_work_item();
    let cols = default_columns_for(&class);          // ← from ColumnKind defaults
    let rows = store.find_issues(&q).await?;
    let row_sources = rows.iter().map(|r| build_row(r, &cols)).collect::<Vec<_>>();
    ogar_render_askama::render_list(
        "Issues", 0x0102, "project_work_item",
        &cols, &[], &row_sources,
    )
}

pub async fn detail(
    State(store): State<Arc<Store>>,
    Path(id): Path<u64>,
) -> Result<askama_axum::Response, AppError> {
    let issue = store.find_issue(id).await?.ok_or(NotFound)?;
    let class = ogar_vocab::project_work_item();
    let cols = detail_columns_for(&class);
    let cells = build_cells(&issue, &cols);
    ogar_render_askama::render_detail(
        0x0102, "project_work_item", id,
        &headline_html(&issue), &subtitle(&issue),
        &cols, &cells,
    )
}
```

Every resource follows this shape. Eight contributors can land eight
tracks in parallel. **No track touches another track's file** — they
each own `rm-handlers/src/<resource>.rs` plus a route registration.

### Width-pass DoD per track
- `GET /<resource_plural>` returns a list page that renders.
- `GET /<resource_plural>/:id` returns a detail page.
- Seed-data rows are visible.
- Empty state renders correctly when no rows.
- Co-located handler test that hits both URLs and asserts the HTML
  contains the resource name and the canonical class_id.

## 4. Phase 2 — Depth pass (parallel cross-cutting tracks)

Phase 2 adds features that **every resource needs** but each is a
single cross-cutting concern. These are tracks (D1..D5) that, after
landing, retrofit every Phase-1 resource page automatically.

### D1 — Forms (create + edit)
- New handlers per resource: `new`, `create`, `edit`, `update`,
  `destroy`. RESTful URL convention.
- Each calls `ogar_render_askama::render_form` with the right
  `FormSource`.
- CSRF token from the session middleware.
- Server-side validation: pull from `Class::attributes[i].options.required`
  + simple per-kind validation (date parsing, etc).
- **Width compatibility:** every Phase-1 resource gets forms in one
  pass. D1 lands one set of handlers in `rm-handlers/src/forms.rs`
  generic over `T: PortSpec`-derived row type.

### D2 — Filters, sort, group, total (Redmine Query)
- Adopt Redmine's `query.inline_columns` / `query.block_columns` shape
  directly — the substrate already has it (`RenderColumn::sortable()`,
  `RenderColumn::groupable()`, `RenderColumn::totalable()`).
- Parse query-string `?filter[status]=open&sort=priority:desc&group=tracker`.
- Render filter chrome (a `<details>` panel with input rows) via a
  new askama template `dispatch/filter_panel.askama`.
- Saved Queries become first-class: the `Query` resource from W8
  already exists; persistence is a SurrealDB row.
- **Width compatibility:** generic over `PortSpec`; every resource
  inherits.

### D3 — RBAC enforcement
- Use the `Membership` / `Role` / `MemberRole` canonical concepts.
- Permission table per (Role, action_set): mirror Redmine's
  `Redmine::AccessControl.permission` (well-documented spec).
- Middleware extractor: `RequirePermission<{ project_id, perm_name }>`
  rejects with 403 if the current user's role on the project lacks the
  permission.
- **Width compatibility:** add one extractor per route; mechanical.

### D4 — Workflows (state transitions)
- `IssueStatus` carries `is_closed: bool`. Redmine's workflows are a
  per-(Tracker, Role) → allowed-transition table.
- Store the transition table as SurrealDB rows (canonical concept
  `project_workflow` — promotion candidate, not in codebook yet; OGAR
  side-PR to add `0x011B`).
- Handler enforces: `update` rejects status changes outside the allowed
  transition set.
- **Width compatibility:** retrofits any state-machine-shaped resource
  (Issue today, future Versions if they grow status).

### D5 — Custom fields (dynamic schema)
- `CustomField` (0x0110) + `CustomValue` (0x0119) canonical concepts.
- Dynamic per-resource attribute set: load all `CustomField`s for the
  current Tracker at request time, append to `cols: Vec<RenderColumn>`
  before render. The askama template already takes a `&[RenderColumn]`
  slice — no template change needed.
- Heaviest of the depth tracks; budget 1 week.
- **Width compatibility:** every resource that supports custom fields
  inherits, mechanical wiring per resource.

### D6 — Full-text search (deferred, parallel)
- SurrealDB has FULLTEXT indexes; emit them via a new
  `op_surreal_ast::IndexKind::Fulltext` variant + `from_class` opt-in.
- One `/search?q=…` route; renders an aggregate `HtmlListView` over
  hits with a `_type` column showing the resource kind.
- **Defer:** until D1+D2 land; the substrate isn't blocking.

### Depth-pass DoD per track
- The cross-cutting feature works on at least 2 Phase-1 resources.
- An integration test exercises the feature on Issue (the busiest
  resource).
- The feature wired through the `PortSpec` substrate — no
  resource-specific code in the depth track itself.

## 5. Phase-parallel — Template + GUI tracks (parallel with everything)

These tracks are **OGAR-side** (templates) or **redmine-rs-side** (CSS,
JS, layouts) and can run concurrent with any phase.

### T1 — Column-kind extensions
- New `ColumnKind` variants Redmine uses that aren't in the 10 we
  shipped: `RelativeTime` ("3 hours ago"), `UserAvatar`,
  `BadgeStatus` (coloured status pill), `MultiTag`, `BoolCheck`,
  `Sparkline` (for done_ratio over time).
- Each lands as `ogar-render-askama/src/artifact_kinds/cells/<kind>.askama`
  + a `CellData` enum variant + a default-kind-for resolver entry.
- **OGAR PR per kind.** Highly parallel — one contributor per kind.

### T2 — InputKind extensions
- Mirror T1 on the form side: `InputKind::FileUpload`,
  `InputKind::RichTextarea` (TinyMCE / Trix substitute), 
  `InputKind::DateRange`, `InputKind::AutocompleteUser`.
- OGAR-side; each is one askama sub-template + a binding-struct variant.

### G1 — Layout chrome
- Master template (`base.askama`): top nav, sidebar with project
  selector, footer.
- Page slot: `{{ content|safe }}` for the per-resource render output.
- **No behavioural code.** Pure CSS + HTML structure.

### G2 — CSS port
- Port Redmine's stylesheet (`public/stylesheets/application.css`,
  ~7000 LOC organized by area). Modern revision: Tailwind utility layer
  underneath, Redmine class names preserved so old plugin themes still
  resolve.
- **No coupling to handlers** — CSS port can land before, during, or
  after Phase 1 with zero conflict.

### G3 — Client-side widgets (deferred-ish)
- Hotwire-style enhancement: Turbo (or htmx) for partial swaps,
  Stimulus controllers for the date-range / autocomplete / collapse
  widgets. Defer until G1+G2 land; the page is server-rendered HTML
  by default and gracefully degrades without JS.

## 6. The dependency graph

```text
                          ┌──────────────┐
                          │  Phase 0     │
                          │  (W0.1-3)    │  ← sequential, 1 week
                          │  Foundation  │
                          └───┬───────┬──┘
                              │       │
       ┌──────┬──────┬────────┴───────┴───────┬──────┬──────┐
       ▼      ▼      ▼                        ▼      ▼      ▼
     ┌────┐ ┌────┐ ┌────┐                  ┌────┐ ┌────┐ ┌────┐
     │ W1 │ │ W2 │ │ W3 │   …  Width    …  │ W6 │ │ W7 │ │ W8 │  ← parallel, 3-5 days each
     │Iss │ │Proj│ │Time│                  │Cmnt│ │ SCM│ │Qry │
     └─┬──┘ └─┬──┘ └─┬──┘                  └─┬──┘ └─┬──┘ └─┬──┘
       │      │      │                       │      │      │
       └──────┴──────┴───────────┬───────────┴──────┴──────┘
                                 │
                                 ▼
                          ┌──────────────┐
                          │  Phase 2     │
                          │  (D1-D6)     │  ← parallel cross-cutting
                          │  Depth pass  │
                          └──────────────┘

      ─── parallel-anytime, OGAR-side ───            ─── parallel-anytime, redmine-rs-side ───
      ┌────┐ ┌────┐                                  ┌────┐ ┌────┐ ┌────┐
      │ T1 │ │ T2 │      column / input kinds        │ G1 │ │ G2 │ │ G3 │   chrome / CSS / JS
      └────┘ └────┘                                  └────┘ └────┘ └────┘
```

**Hard dependencies** (must-land-before):
- W0 before any W*.
- D1 (forms) before D5 (custom fields render in forms).
- D2 (filters) before D6 (search uses the filter parser).
- W4 (User/Membership/Role) before D3 (RBAC needs the substrate).

**Soft dependencies** (one helps the other but they can swap order):
- T1 before W* — but every W* can ship using the existing 10
  `ColumnKind`s and pick up the richer ones in a refresh.
- G2 (CSS) is independent of everything else.

## 7. Effort estimate

| Phase | Streams | Per-stream effort | Wall-clock with 1 contributor | Wall-clock with 4 contributors |
|---|---|---|---|---|
| Phase 0 | 1 (sequential) | 1 week | **1 week** | **1 week** |
| Phase 1 | 8 width tracks | 4 days each | ~6 weeks serial | **~1.5 weeks parallel** |
| Phase 2 | 6 depth tracks | 3-7 days each | ~5 weeks serial | **~2 weeks parallel** |
| T-tracks | 6 template/GUI | 2-4 days each | ~3 weeks serial | **~1 week parallel** |
| **Total** | | | **~15 weeks** | **~5-6 weeks** |

**MVP cut-line** (Issue browse + create + login): Phase 0 + W1 + D1 +
minimum G1 = ~2 weeks with 1 contributor, ~1 week with 2.

**Feature parity cut-line** (everything Redmine does, minus SCM
driver): Phase 0 + W1-W8 + D1-D5 + T1-T2 + G1-G2 = ~5-6 weeks with 4
contributors.

## 8. Parallel-safety — file ownership

The streams are designed so **no two parallel streams touch the same
file**. Per-stream file ownership:

| Stream | Owns |
|---|---|
| W0.1 | `rm-server/src/main.rs`, `rm-server/Cargo.toml` |
| W0.2 | `rm-store/src/**`, `rm-store/Cargo.toml` |
| W0.3 | `rm-auth/src/**`, `rm-auth/Cargo.toml` |
| W1 | `rm-handlers/src/issue.rs` + `journal.rs` + 1 route block in `rm-server/src/routes.rs` |
| W2 | `rm-handlers/src/project.rs` + `version.rs` + `enabled_module.rs` + 1 route block |
| W3..W8 | Same shape — own one file per resource + one route block |
| D1 | `rm-handlers/src/forms.rs`, generic |
| D2 | `rm-handlers/src/filter.rs`, generic |
| D3 | `rm-auth/src/rbac.rs` |
| D4 | `rm-handlers/src/workflow.rs` |
| D5 | `rm-handlers/src/custom_fields.rs` |
| T1 | `ogar-render-askama/src/artifact_kinds/cells/<new_kind>.askama` + `cells.rs` add variant |
| T2 | `ogar-render-askama/src/artifact_kinds/inputs/<new_kind>.askama` + `inputs.rs` add variant |
| G1 | `rm-server/templates/base.askama` + layout templates |
| G2 | `rm-server/assets/css/**` |
| G3 | `rm-server/assets/js/**` + Stimulus controllers |

**`rm-server/src/routes.rs`** is the one shared file. Convention: each
W* track adds its route block at the bottom in alphabetical order; merge
conflicts there are trivial.

## 9. Calibration gates per stream

Every stream lands with the three gates Northstar §3 calibrated:

1. **Round-trip parse** — emitted HTML parses as valid HTML5 (use
   `html5ever` in tests).
2. **Target-toolchain compile** — `cargo check` green; askama templates
   compile.
3. **ClassView drift guard** — every `RenderColumn::new(name, …)` for a
   class references a `Class::attributes` entry. A test enumerates
   the columns and asserts each name is in the class's attributes.

Plus stream-specific gates from the DoD lists above.

## 10. Open questions — to decide before / during

1. **Datastore choice** — SurrealDB everywhere (consistent with the
   schema emission story), OR support reading existing Redmine
   Postgres/MySQL via a `rm-store-pg` sibling? Recommendation:
   SurrealDB-only for MVP; import tool comes later as a separate PR.
2. **i18n** — Redmine ships 50+ locales. Use `rust-i18n` (already in
   OpenProject's deps) or `fluent`? Defer choice; design handlers to
   take labels from a `Localizer` trait so the impl is swappable.
3. **Plugin API surface** — Redmine has a plugin system
   (`Redmine::Plugin.register`). Out of scope for MVP. Long term, the
   PortSpec / canonical-concept pattern is the modern equivalent: a
   plugin defines new concepts in OGAR + handlers in `rm-handlers`.
4. **REST API parity** — JSON variants of every handler. Add a
   `serde_json::Serialize` impl on `RowSource` and one extractor that
   switches on `Accept: application/json`. Defer to a depth track
   after Phase 1 settles.

## 11. The single-paragraph "ship it" summary

The substrate is done. The askama kit already inherits Redmine's 17
years of UX organic discovery (column dispatch, formatter library,
default index hints — all sitting in `ColumnKind` and the per-kind
sub-templates). What's left is **mechanical wiring**: an axum server,
a SurrealDB store, per-resource handlers that compose
`(class, query) → render_list / render_detail / render_form`. Eight
contributors can land MVP-shaped Redmine in ~2 weeks of parallel work,
feature-complete in ~6 weeks. The substrate is the hard part, and the
substrate is the part already in main.
