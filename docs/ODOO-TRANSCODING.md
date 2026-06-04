# Odoo Transcoding — Carved Semantics

> **Purpose.** Companion to `docs/IDENTITY-MAPPING.md`. Carves the
> Odoo-specific design decisions for `ogar-python` (the planned Odoo
> producer) and `ogar-ext-odoo` (the extension vocabulary) so they
> ship coherently from day one. Same rigor as IDENTITY-MAPPING:
> every Odoo concept maps to **exactly one** OGAR construct, every
> gap is named.
>
> Status: **CARVED v0** (2026-06-04). Grounded in agent research on
> Odoo 17.0 source (`addons/account`, `sale`, `stock`, `mail`).

## 1. Scope (v1)

Per BO3 (YAGNI review), the minimum viable Odoo transcoder for v1
covers:

- **Target Odoo version**: 17.0 only. v13–v16 + v18-dev defer.
- **Source surface**: `addons/<module>/models/*.py` +
  `wizard/*.py` (only if `models.Model`, not `TransientModel`) +
  `report/*.py` (`_auto = False` SQL-backed models).
- **Excluded**: `controllers/`, `tests/`, `static/`, `views/*.xml`,
  `data/*.xml`, `security/*.csv`. No XML, no wizards-as-dialogs,
  no QWeb reports.
- **Addon sources**: core `odoo/addons/` (Community). Enterprise
  + OCA addons defer until core round-trips cleanly.
- **Multi-version support**: not in v1. The `Class.source_version`
  field (added in this PR) is reserved but unused.

## 2. Module discovery algorithm

Per RO1 (Odoo source structure). The transcoder walks addons by:

```
1. Configure: list of `--addons-path` roots (default: ['odoo/addons']).
2. For each root: walk top-level subdirectories containing
   `__manifest__.py` (or legacy `__openerp__.py`).
3. For each addon: parse `__manifest__.py` to extract:
     - name (= directory name, normalized)
     - depends (transitive — recurse through ALL listed dependencies)
     - license (distinguish Community / OEEL-1 / AGPL-3)
4. Build a topological sort of addons by `depends`.
5. For each addon in topological order:
     a. Parse `__init__.py` at addon root — extract imported modules.
     b. Recurse: for each imported subdirectory (`models`, `wizard`,
        `report`), parse its `__init__.py` for explicit imports.
     c. AST-walk each imported `.py` file (NOT glob — avoids
        deprecated/test fixtures).
6. Within each file: find every `class X(BaseClass):` where
   BaseClass MRO includes `models.Model`, `models.TransientModel`,
   or `models.AbstractModel`.
```

**Carve-out**: discovery is `__init__.py`-driven, not glob-driven.
Glob produces false positives (test fixtures, deprecated files,
half-merged additions).

## 3. Field type mapping

Per RO2 (`odoo/fields.py` 5249 lines surveyed).

| Odoo `fields.X`        | OGAR construct                            | Crate         | Notes                                              |
|------------------------|-------------------------------------------|---------------|----------------------------------------------------|
| `Boolean`              | `Attribute(type_name="boolean")`          | ogar-vocab    |                                                    |
| `Integer`              | `Attribute(type_name="integer")`          | ogar-vocab    |                                                    |
| `Float`                | `Attribute(type_name="float")` + digits   | ogar-vocab    | `digits=(precision, scale)`                        |
| `Monetary`             | `Attribute(type_name="monetary")` + currency_field | ogar-vocab | Carries `currency_field` FK reference          |
| `Char`                 | `Attribute(type_name="char")` + size      | ogar-vocab    |                                                    |
| `Text`                 | `Attribute(type_name="text")`             | ogar-vocab    |                                                    |
| `Html`                 | `Attribute(type_name="html")` + sanitize  | ogar-vocab    | MIME=text/html                                      |
| `Date`                 | `Attribute(type_name="date")`             | ogar-vocab    |                                                    |
| `Datetime`             | `Attribute(type_name="datetime")`         | ogar-vocab    | UTC naive                                           |
| `Binary`               | `Attribute(type_name="binary")` + attachment | ogar-vocab | `attachment=True` default                          |
| `Image`                | `Attribute(type_name="image")` + dims     | ogar-vocab    | max_width/max_height                                |
| `Selection`            | `EnumDecl` (Static or Computed source)    | ogar-vocab    | `selection=` or `selection=lambda`                  |
| `Json`                 | `Attribute(type_name="json")`             | ogar-vocab    | jsonb                                               |
| `Many2one`             | `Association(BelongsTo)`                  | ogar-vocab    | `ondelete=`, `auto_join=`, `domain=`                |
| `One2many`             | `Association(OwnsMany)`                   | ogar-vocab    | `inverse_name=` is the FK on target                 |
| `Many2many`            | `Association(HasAndBelongsToMany)`        | ogar-vocab    | `relation=` join table, `column1/column2`           |
| **`Reference`**        | `ogar_ext_odoo::PolymorphicReference`     | ogar-ext-odoo | `"model,id"` string, polymorphic                    |
| **`Many2oneReference`**| `ogar_ext_odoo::SplitPolymorphicRef`      | ogar-ext-odoo | id + sibling Char `model_field`                     |
| **`Properties`**       | `ogar_ext_odoo::Properties`               | ogar-ext-odoo | Dynamic schemaless                                  |
| **`PropertiesDefinition`** | `ogar_ext_odoo::PropertiesDefinition` | ogar-ext-odoo | Container-side schema descriptor                    |

**Carve-out**: `Monetary`, `Html`, `Image` live in **base vocab**
(`Attribute.type_name`) because they're representable as
typed-scalar-with-metadata; the metadata goes into structured
options. Polymorphic refs and Properties (dynamic-schema fields)
live in **ext-odoo** because they don't fit the Attribute shape.

## 4. Attribute kwargs (the gap BO1 #1)

`Attribute` in ogar-vocab now carries an `options` struct covering
every cross-cutting Odoo kwarg:

```rust
pub struct AttributeOptions {
    /// `default=value` — literal or callable name as source text.
    pub default_source: Option<String>,
    /// `required=True`.
    pub required: Option<bool>,
    /// `readonly=True`.
    pub readonly: Option<bool>,
    /// `index=True` — DB index on the column.
    pub indexed: Option<bool>,
    /// `store=True` (relevant for computed fields).
    pub stored: Option<bool>,
    /// `translate=True` — i18n column (jsonb in 17.0).
    pub translate: Option<bool>,
    /// `tracking=True` / `tracking=10` — Odoo audit log priority.
    pub tracking: Option<TrackingPriority>,
    /// `groups='group.xml.id,...'` — visibility ACL.
    pub groups: Vec<String>,
    /// `company_dependent=True` — value varies by `res.company`.
    pub company_dependent: Option<bool>,
    /// `prefetch=False` / int — batch-prefetch hint.
    pub prefetch: Option<PrefetchPolicy>,
    /// `copy=False` — excluded from `model.copy()`.
    pub copy_on_duplicate: Option<bool>,
    /// `help='...'` — UI tooltip text.
    pub help_text: Option<String>,
    /// `string='Label'` — UI label override.
    pub label: Option<String>,
    /// `digits=(precision, scale)` — Float/Monetary precision.
    pub digits: Option<(u8, u8)>,
    /// `size=N` — Char/Binary size limit.
    pub size: Option<usize>,
    /// `currency_field='currency_id'` — Monetary linkage.
    pub currency_field: Option<String>,
}
```

**Carve-out**: every Odoo kwarg the transcoder encounters MUST land
in this struct. If a new kwarg surfaces, add it as a new optional
field (forward-compat via `#[non_exhaustive]`); don't smuggle it
through `help_text` or another bucket.

Rails / Django / Ecto producers MAY populate the subset they
support (`required`, `default_source`, `label`); they leave the
Odoo-specific ones `None`.

## 5. Association kwargs (the gap BO1 #2)

`Association` extends with:

```rust
pub struct Association {
    // ... existing fields ...

    /// `ondelete='cascade'/'restrict'/'set null'/'set default'` —
    /// DB-level FK action (distinct from Rails `dependent:` which
    /// is app-level).
    pub ondelete: Option<String>,
    /// `auto_join=True` — auto SQL-join, not lazy load.
    pub auto_join: Option<bool>,
    /// `context={'default_partner_id': partner_id}` — UI default
    /// context for navigation.
    pub context_source: Option<String>,
    /// `check_company=True` — multi-company tenancy check.
    pub check_company: Option<bool>,
    /// `delegate=True` — old-style delegation (rare, prefer _inherits).
    pub delegate: Option<bool>,
}
```

**Carve-out**: `ondelete` (DB-level) and `dependent` (Rails
app-level) are STORED SEPARATELY. Conflating them in PG DDL
emission produces wrong cascade semantics.

## 6. EnumDecl with computed and additive sources (the gap BO1 #3)

`EnumDecl.values` becomes an enum to distinguish static, computed,
and additive cases:

```rust
pub enum EnumSource {
    /// `selection=[('draft', 'Draft'), ('done', 'Done')]`.
    Static(Vec<(String, String)>),
    /// `selection=lambda self: self.env['res.country']...`.
    /// The lambda body is captured verbatim.
    Computed(String),
    /// `selection_add=[('paid', 'Paid')]` — extends a parent
    /// `_inherit` model's selection without redeclaring.
    /// `parent_selection` names the parent class.
    Add { items: Vec<(String, String)>, parent_selection: String },
}

pub struct EnumDecl {
    pub column: String,
    pub source: EnumSource,
    pub scopes_disabled: Option<bool>,
}
```

**Carve-out**: this is structural, not optional. Real Odoo addons
(notably `account.move`) extend the `state` selection in dependent
modules; collapsing to `Static` produces wrong enums in OCA
addons.

## 7. Class-level metadata (the gap BO1 #4)

`Class` extends with Odoo's underscore-prefixed class attributes:

```rust
pub struct Class {
    // ... existing fields ...

    /// `_description = 'Sale Order'` — human-readable name.
    pub description: Option<String>,
    /// `_order = 'date desc, id'` — default record ordering.
    pub record_order: Option<String>,
    /// `_rec_name = 'name'` — UI display field.
    pub rec_name: Option<String>,
    /// `_check_company_auto = True` — auto multi-company check.
    pub check_company_auto: Option<bool>,
    /// `_log_access = False` — skip create_uid/write_uid columns.
    pub log_access: Option<bool>,
    /// `_auto = False` — no auto table create (SQL view models).
    pub auto_create_table: Option<bool>,
    /// `_abstract = True` — base class, no table.
    pub abstract_model: bool,
    /// `_transient = True` — wizard/scratchpad model.
    pub transient: bool,
    /// `_register = False` — skip from registry.
    pub register: Option<bool>,
    /// Module name from `__manifest__.py` ('sale', 'account').
    /// Captured per BO2 #3 — the missing addon-scope provenance.
    pub declared_in_module: Option<String>,
    /// Source language version hint (Odoo 17.0, Rails 7.2, …).
    /// Reserved for multi-version support; v1 leaves it `None`.
    pub source_version: Option<String>,
}
```

**Carve-out**: `declared_in_module` is REQUIRED for Odoo classes
(every Odoo class has an owning addon). It's OPTIONAL for Rails
classes (Rails apps don't have a module concept beyond engines /
gems). The producer fills it; the emitter emits `ogar:declaredIn`
triple per class. This makes "show me all classes in the sale
addon" answerable.

## 8. MethodDecl + ComputedField (the gap BO1 #5)

CRUD method overrides and computed fields get first-class struct
representation:

```rust
pub enum MethodKind {
    /// `def create(self, vals_list)` — total override of CRUD.
    /// Distinguished from `Callback` (which is hook-style around).
    CrudOverride,
    /// `@api.model def my_helper(self, ...)` — classmethod-like.
    ApiModel,
    /// `@api.model_create_multi def create(self, vals_list)` —
    /// Odoo's bulk-create override.
    ApiModelCreateMulti,
    /// Plain instance method.
    Instance,
}

pub struct MethodDecl {
    pub name: String,
    pub kind: MethodKind,
    pub body_source: String,
    /// Decorator name(s) as written: ["api.depends", "api.constrains"].
    pub decorators: Vec<String>,
    /// Recordset semantics — does this method bind to a record,
    /// a recordset, or class-level?
    pub semantics: RecordSemantics,
}

pub enum RecordSemantics {
    /// Single record context.
    Record,
    /// Recordset (default for most Odoo methods).
    Recordset,
    /// Class-level (`@api.model` or no `self`).
    ClassLevel,
}

pub struct ComputedField {
    /// The field being computed.
    pub field: String,
    /// Compute method name: `compute='_compute_total'` → `"_compute_total"`.
    pub compute_method: String,
    /// Dependency paths from `@api.depends('partner_id', 'order_line.price_total')`.
    pub depends: Vec<String>,
    /// `@api.depends_context('uid')` — env-context dependencies.
    pub depends_context: Vec<String>,
    /// `store=True/False`.
    pub stored: bool,
    /// `inverse='_inverse_total'` — write-direction helper.
    pub inverse_method: Option<String>,
    /// `search='_search_total'` — search helper.
    pub search_method: Option<String>,
}
```

**Carve-out**: `ComputedField` lives in **base vocab**, not
ext-odoo. Computed fields exist in Django (`@cached_property` +
manual recompute) and Rails (instance methods); having the struct
in base lets cross-language consumers query "all computed fields"
uniformly.

**Carve-out**: `MethodDecl` is required for Odoo because CRUD
overrides are common; Rails producers may emit empty
`Class.methods` (Rails callbacks are already in `Class.callbacks`).

## 9. State machines (per RO4)

`states={}` dict pattern is GONE in Odoo 17.0. The transcoder
captures state machines as a composite:

```rust
// In ogar-ext-odoo:
pub struct StateMachine {
    /// The state field name (usually "state").
    pub state_field: String,
    /// All valid states with labels.
    pub states: Vec<(String, String)>,
    /// Default state ("draft" typically).
    pub default_state: Option<String>,
    /// Computed-state flag (e.g. `stock.picking` derives state).
    pub computed_state: bool,
    /// Tracking flag for audit log.
    pub tracking: bool,
}

pub struct Transition {
    /// Method name: `action_confirm`, `_action_post`, `button_draft`.
    pub method: String,
    /// Visibility convention: Public (`action_*`), Protected
    /// (`_action_*`), Button (`button_*`).
    pub visibility: TransitionVisibility,
    /// Allowed source states (inferred from method-body checks).
    /// May be empty if guards live entirely in the method body.
    pub from_states: Vec<String>,
    /// Target state(s).
    pub to_state: Option<String>,
}

pub struct StateGuard {
    /// Predicate source text from the method body
    /// (`self.state in {'draft', 'sent'}`).
    pub predicate_source: String,
    /// Exception raised on guard failure: "UserError", "ValidationError".
    pub raises: Option<String>,
}

pub struct ScheduledTransition {
    /// XML ID of the ir.cron record.
    pub cron_xml_id: String,
    /// Method name that the cron calls.
    pub calls_method: String,
}
```

**Carve-out**: states are CONVENTION-extracted from Odoo source,
not declarative. The transcoder uses heuristics
(`action_*`/`_action_*`/`button_*` naming, `tracking=True` on the
`state` field, `self.state = '...'` writes inside methods). Low
confidence cases are tagged `confidence: Heuristic` so consumers
can choose how to handle.

## 10. `_inherit` resolution algorithm (per RO5)

Six-pass static algorithm:

```
PASS 1 (parse)
  Walk every addon's models/*.py via __init__.py imports.
  Collect every ClassDef whose MRO touches models.Model/
  TransientModel/AbstractModel. Extract _name, _inherit (list-
  normalized), _inherits (dict), plus addon manifest depends.

PASS 2 (classify)
  For each ClassDef tag as:
    NEW    — has _name, no _inherit
    EXTEND — _name matches _inherit item (or _inherit only with
             implicit _name)
    MIXIN  — _name differs from all _inherit items

PASS 3 (model_table)
  Build one Model entry per distinct _name. Append every
  NEW/EXTEND ClassDef as a `definition` tagged with (module, file,
  lineno). MIXIN classes are separate Model entries that other
  Models include via Role::Include.

PASS 4 (topological_merge)
  Order each Model's definitions by manifest depends DAG (Kahn
  sort). Fold fields + methods in that order; later wins (mirrors
  Odoo's `reversed(__base_classes)`).

PASS 5 (MRO_assembly)
  For each Model, resolve _inherit items to other Model entries.
  Emit InheritanceEdge (Role::Include) per parent. Emit
  DelegationEdge (Role::Delegate, ogar-ext-odoo) per _inherits
  entry. NEVER mix the two.

PASS 6 (validate)
  Assert every _inherit target exists in the table. Assert every
  _inherits FK field is declared. Fail loudly on missing deps.
```

**Carve-out**: this algorithm is **producer responsibility**
(`ogar-python`). The OGAR IR (`Class`) only sees the
post-merge, post-MRO result. The intermediate `definitions`
list is for producer-internal traceability; it does NOT
become triples.

## 11. `Role::Extends` (per BO2 #3)

EXTEND-pattern classes (same `_name`, additional fields in a
downstream addon) are NOT mixins. They're model-extensions.

New Role variant:

```rust
pub enum Role {
    // ... existing variants ...
    /// EXTEND pattern: same `_name`, additional fields/methods
    /// declared in a downstream addon. Distinct from `Include`
    /// (which composes a different class as mixin).
    Extends,
}
```

Path syntax (pathlike):
```
ogit-erp::sale.order
ogit-erp::sale.order::extends::sale_stock           # the sale_stock module's extension of sale.order
ogit-erp::sale.order::extends::sale_stock::field::picking_ids
```

The `extends` segment carries the **module name** that owns the
extension. This lets queries like "what does sale_stock add to
sale.order" return only the relevant subset of triples.

**Carve-out**: `Role::Extends` triples are emitted by the producer
in parallel with the merged-class triples. Both views (per-addon
delta + merged class) coexist in lance-graph.

## 12. Decorator mapping (per RO3)

| Decorator                  | Role mapping                                                                   |
|----------------------------|--------------------------------------------------------------------------------|
| `@api.depends(...)`        | `DependsSpec` (ext) linked to a `ComputedField` (base) by method name          |
| `@api.depends_context(...)`| `DependsSpec` (ext, with `context=True`)                                       |
| `@api.constrains(...)`     | `Validation` with multi-target list                                            |
| `@api.onchange(...)`       | `Callback` with `event="onchange"` (form-only UI hook)                         |
| `@api.model`               | Role qualifier on host `MethodDecl` — `kind=ApiModel`                          |
| `@api.model_create_multi`  | `MethodDecl{ kind: ApiModelCreateMulti, name: "create" }` + implicit `Callback`|
| `@api.returns(...)`        | Role qualifier on host method — recorded in `MethodDecl.decorators`            |
| `@api.ondelete(...)`       | `Callback` with `event="before_destroy"` (declarative form)                    |
| `@api.autovacuum`          | `ScheduledJob` (ext)                                                           |
| `@api.private`             | `AccessPolicy` (ext, `rpc_exposed=false`)                                      |
| `@api.readonly` (18.0+)    | `AccessPolicy` (ext, `cursor=readonly`)                                        |

New roles introduced (in `IDENTITY-MAPPING.md` Role enum):
- `DependsSpec` (ext) — captures `@api.depends`/`@api.depends_context` info
- `ScheduledJob` (ext) — captures `@api.autovacuum` and ir.cron callers
- `AccessPolicy` (ext) — captures `@api.private`/`@api.readonly`

## 13. CRUD overrides — the two-stage detection (per RO3)

```
STAGE 1 (AST candidate)
  If a class inherits (directly or via _inherit) from
  models.Model/TransientModel/AbstractModel, AND it defines
  `def create|write|unlink|copy(self, ...)`, emit a
  MethodDecl{ kind: CrudOverride, name: "create", ... }.
  Confidence: high if @api.model_create_multi is present; medium
  otherwise.

STAGE 2 (MRO confirmation, optional)
  If the producer can resolve the MRO (it has access to BaseModel's
  AST), confirm the method exists upstream. Mark unresolved as
  confidence: low. v1 producer may skip this and ship with
  medium-confidence emissions.
```

## 14. Registered-prefix table (per BO2 #1)

Prevent cross-language identity collisions by REGISTERING each
prefix to a source language at the ontology layer.

```rust
// In ogar-ontology:
pub struct PrefixRegistration {
    pub prefix: &'static str,
    pub source_language: ogar_vocab::Language,
    pub description: &'static str,
}

pub static REGISTRY: &[PrefixRegistration] = &[
    PrefixRegistration {
        prefix: "ogit-op",
        source_language: Language::Ruby,
        description: "OpenProject Rails application classes",
    },
    PrefixRegistration {
        prefix: "ogit-erp",
        source_language: Language::Python,
        description: "Odoo / ERP business semantics",
    },
    // ...
];

pub fn validate_prefix_for_lang(prefix: &str, lang: Language) -> Result<(), PrefixError> {
    match REGISTRY.iter().find(|r| r.prefix == prefix) {
        Some(r) if r.source_language == lang => Ok(()),
        Some(r) => Err(PrefixError::LanguageMismatch { prefix, expected: r.source_language, got: lang }),
        None => Err(PrefixError::UnregisteredPrefix(prefix.into())),
    }
}
```

**Carve-out**: producers MUST call `validate_prefix_for_lang`
before emitting triples. Unregistered prefixes are errors; mixed-
language prefixes are errors. This makes cross-language collisions
**impossible by construction**.

## 15. Conformance corpus (per BO2 #2)

`crates/ogar-conformance/` (Sprint 2.5) defines a frozen
fixture set of (source-snippet, expected-OGAR-IR, expected-triples)
triples per Role variant. Every producer (`ogar-from-ruff`,
`ogar-python`, future `ogar-sql-ddl`) runs this suite as a
`cargo test` gate.

```
crates/ogar-conformance/
  fixtures/
    member_of/
      ruby-belongs-to.rb         + expected.json
      odoo-many2one.py           + expected.json
      django-foreignkey.py       + expected.json
      ecto-belongs-to.ex         + expected.json
    owns_many/
      ruby-has-many.rb           + expected.json
      odoo-one2many.py           + expected.json
      ...
    ...
  src/
    lib.rs  — provides assert_conforms!(producer, fixture_dir)
```

The corpus is THE drift detector. Producers diverging from the
fixture fail their own test suite, not someone else's.

## 16. Out of scope (deliberate)

- XML data files (`data/*.xml`, `demo/*.xml`).
- View XML, action records, menu items.
- QWeb reports.
- Security CSVs (`ir.model.access.csv`, record rules).
- Wizards (`TransientModel`) for UI dialogs.
- Multi-version source compatibility (v13–v18 mix).
- OCA + Enterprise addons.
- Runtime introspection from a live Odoo Postgres (the "Path B"
  from R5 — defer).
- Custom fields added through Odoo UI (`ir.model.fields` runtime
  rows) — defer to `ogar-from-odoo-runtime` (no sprint yet).

## 17. Cross-references

- `docs/IDENTITY-MAPPING.md` — the base carve-out (Role enum,
  Identity struct, syntax variants); this document extends it.
- `.claude/PLAN.md` Sprint 4 (`ogar-python` producer), Sprint 5
  (`ogar-ext-odoo`), Sprint 2.5 (conformance corpus).
- `crates/ogar-vocab/src/lib.rs` — the base IR types referenced above.
- `crates/ogar-ext-odoo/` — Odoo-specific types (Properties,
  PolymorphicReference, StateMachine, Transition, StateGuard,
  ScheduledTransition, DependsSpec, ScheduledJob, AccessPolicy).
- `crates/ogar-conformance/` — the per-Role fixture corpus.
- Brutal-review provenance:
  - RO1 source structure (`addons/account`, `sale`, `stock`, `mail` 17.0)
  - RO2 fields.py survey
  - RO3 api.py decorator inventory
  - RO4 state machine convention survey
  - RO5 `_inherit` runtime resolution + pylint-odoo borrowed algorithm
  - BO1 vocabulary gaps (top 5 fixed here)
  - BO2 architectural decisions (registered-prefix + conformance + Extends — all locked here)
  - BO3 YAGNI scope (v1 = Odoo 17.0 core only)

## 18. Carve-outs summary (the non-negotiable Odoo list)

1. **Discovery is `__init__.py`-driven**, not glob-driven.
2. **`Monetary`, `Html`, `Image` live in base vocab**; polymorphic refs and Properties live in ext-odoo.
3. **`AttributeOptions` is structured** — every Odoo kwarg has a field. No kwarg-dump bucket.
4. **`ondelete` ≠ `dependent`** — DB-level vs app-level FK action; stored separately.
5. **`EnumSource` has three variants** — Static / Computed / Add; collapsing them breaks OCA addons.
6. **Class-level metadata** (`_description`/`_order`/`_rec_name`/`_abstract`/`_transient`) gets first-class fields on `Class`.
7. **`MethodDecl` is required for CRUD overrides** — `Callback` does not cover them.
8. **`ComputedField` lives in base vocab**, not ext.
9. **`declared_in_module` is REQUIRED for Odoo classes** — addon scope is queryable only if this triple exists.
10. **State machines use a composite shape** (StateField + Transition + StateGuard + ScheduledTransition).
11. **`Role::Extends` is distinct from `Role::Include`** — model extension is not mixin composition.
12. **`Registered-prefix table` makes cross-language collisions impossible by construction**.
13. **Conformance corpus is the drift detector** — every producer runs it as a test gate.

These thirteen are the **Odoo drift-prevention contract**.
