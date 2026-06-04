# OGAR Identity & Role Mapping — Carved Semantics

> **Purpose.** This document is the canonical carve-out for OGAR
> identity strings and the `Role` enum. Every Active Record concept
> across Rails / Odoo / Django / Ecto / SurrealQL maps to **exactly
> one** `Role` variant; every `Role` variant has **exactly one**
> canonical meaning. There is no "we'll decide later" in this file.
> Future sessions read this doc, not their intuition.
>
> Status: **CARVED v0** (2026-06-04). Changes to this file are
> ontology-level decisions; open a discussion before editing.

## 1. The `Identity` struct (canonical IR)

Every OGAR identity string — regardless of which syntax variant it
was written in — parses to this struct. The struct is the single
source of truth; string forms are presentation.

```rust
pub struct Identity {
    /// Application prefix: "ogar", "ogit", "ogit-erp", "ogit-op",
    /// "ogar-extensions/odoo", or a tenant-scoped form like
    /// "acme.ogit-op". Never empty.
    pub prefix: String,

    /// Class name as written by the source ORM. PascalCase for Rails
    /// / Ecto ("WorkPackage"), dotted-snake_case for Odoo
    /// ("sale.order"), snake_case for Django ("WorkPackage" is also
    /// the convention). Preserved verbatim — no normalisation.
    pub class: String,

    /// Optional role + target. `None` for a bare class identity.
    /// `Some` for anything addressing a sub-aspect of the class
    /// (association, field, mixin, scope, callback, validation).
    pub role: Option<Role>,
    pub target: Option<String>,

    /// Ontology version for hot-reload addressing. `None` = latest;
    /// `Some(n)` = pinned to version n. See §6.
    pub version: Option<u64>,
}
```

**Carve-out**: `prefix` may contain `.` (tenant separator) and `-`
(sub-prefix). It MUST NOT contain `/`, `::`, `@`, `>`, `(`, `)`. The
parser rejects any prefix violating this.

**Carve-out**: `class` may contain `.` (Odoo `sale.order`), `_`
(snake_case), and ASCII letters/digits. It MUST NOT contain `/`,
`::`, `@`, `>`, `(`, `)`.

**Carve-out**: `target` follows the same rules as `class`.

## 2. The `Role` enum — exhaustive mapping

```rust
pub enum Role {
    // ─── Associations (ORM relation between two classes) ─────────────
    MemberOf,           // belongs_to / Many2one / ForeignKey / Ecto belongs_to
    OwnsOne,            // has_one / One2one (constrained One2many) / OneToOneField
    OwnsMany,           // has_many / One2many / ManyToManyField-style reverse
    GroupOwnsOne,       // has_one :through  (rare but Rails-legal)
    GroupOwnsMany,      // has_many :through  /  has_and_belongs_to_many  /
                        // many_to_many (Ecto) /  Many2many (Odoo) /
                        // ManyToManyField (Django)

    // ─── Includes & delegation ────────────────────────────────────────
    Include,            // include (Rails) / use (Elixir) / _inherit (Odoo)  /
                        // class M(Mixin, ...) (Django MRO mixin)
    ClassInclude,       // extend (Rails) — adds class methods, NOT instance methods
    Delegate,           // Odoo _inherits (FK + auto-forward — STRUCTURALLY
                        // different from Include).
                        // Rails: per-method `delegate :method, to: :assoc` is
                        // EMITTED PER METHOD; this Role applies only when an
                        // entire class is delegated (Odoo style).

    // ─── Field-like declarations ──────────────────────────────────────
    EnumOf,             // enum :col, {...} / fields.Selection (Odoo, Django)
    Attribute,          // attribute :name, :type (Rails) / Column without
                        // table backing / Odoo computed-target fields
    StoreField,         // store_accessor (Rails JSONB) /
                        // Odoo / Django JSONField-derived pseudo-field

    // ─── Class-level filters (always class-method-level in ORM) ───────
    Scope,              // scope :name, -> { ... } / Ecto query function
                        // (named scope at class level)
    DefaultScope,       // default_scope -> { ... }  — applies to all queries
                        // (Ecto has no direct equivalent — see §5.7)

    // ─── Lifecycle ────────────────────────────────────────────────────
    Callback,           // before_save / after_create / @api.depends /
                        // Ecto changeset functions
                        // `target` = event name; index disambiguates duplicates
    Validation,         // validates :col, ... / @api.constrains /
                        // Ecto validate_required etc.
                        // `target` = column; index disambiguates duplicates

    // ─── Extensions (live in ogar-extensions/<lang>/) ─────────────────
    Workflow,           // Odoo state machine / Rails state_machine gem
    ComputedField,      // Odoo @api.depends-computed field / Rails computed
                        // instance method

    // Note: every Role variant above is `#[non_exhaustive]`-additive.
    // Future Roles (e.g. for GraphQL-native relations) are appended
    // here; pattern matches must have a `_ => Role::Unknown`-style
    // catch-all (and `Unknown` itself is a Role).
    Unknown,
}
```

### 2.1 Full mapping table

| Role            | Rails (`ActiveRecord`)               | Odoo (`models.Model`)                   | Django (`models.Model`)               | Ecto (`schema do ... end`)            | SurrealQL                                  |
|-----------------|--------------------------------------|------------------------------------------|---------------------------------------|----------------------------------------|--------------------------------------------|
| `MemberOf`      | `belongs_to :x`                      | `x = fields.Many2one('model.x')`         | `x = models.ForeignKey('X')`           | `belongs_to :x, X`                     | `DEFINE FIELD x_id ON t TYPE record<x>`    |
| `OwnsOne`       | `has_one :x`                         | `x_ids = fields.One2many(...)` size-1    | `x = models.OneToOneField('X')`        | `has_one :x, X`                        | (none — model with unique FK)              |
| `OwnsMany`      | `has_many :xs`                       | `x_ids = fields.One2many('x', 'parent')` | `xs = models.ManyToManyField('X')` rev | `has_many :xs, X`                      | (relational query)                          |
| `GroupOwnsOne`  | `has_one :x, through: :y`            | (rare — composed One2many)               | (no native — through-model required)   | `has_one :x, through: [:y, :x]`        | (composed relation)                        |
| `GroupOwnsMany` | `has_and_belongs_to_many :xs`        | `xs = fields.Many2many('x', 'join')`     | `xs = models.ManyToManyField('X')`     | `many_to_many :xs, X, join_through: J` | `DEFINE TABLE join SCHEMAFULL ...`         |
| `GroupOwnsMany` | `has_many :xs, through: :y`          | (composed via join model)                | (through-model)                        | `has_many :xs, through: [:y, :xs]`     | (composed)                                  |
| `Include`       | `include Mentionable`                | `_inherit = 'mail.thread'`               | `class C(M, Mixin)`                    | `use MyMod`                            | (none — schema composition)                 |
| `ClassInclude`  | `extend ClassHelpers`                | (rare — class-method monkey-patch)       | (rare — metaclass)                     | (none direct — `import` macros)        | (none)                                      |
| `Delegate`      | (per-method `delegate` only)         | `_inherits = {'tpl': 'tpl_id'}`           | (none direct)                          | (none direct)                          | (none direct)                               |
| `EnumOf`        | `enum status: { ... }`               | `state = fields.Selection([...])`         | `status = models.CharField(choices=)`  | `Ecto.Enum`                            | `ASSERT $value IN [...]`                   |
| `Attribute`     | `attribute :name, :type`             | `name = fields.Char()`                   | `name = models.CharField()`            | `field :name, :string`                 | `DEFINE FIELD name ON t TYPE string`       |
| `StoreField`    | `store_accessor :col, [:a, :b]`      | (JSONField derived)                      | `name = JSONField()` derived           | `field :data, :map`                    | `DEFINE FIELD data.* ON t TYPE any`        |
| `Scope`         | `scope :open, -> { ... }`            | (search domain helper)                   | `class Manager(models.Manager)`        | `def open_query(q), do: from(...)`     | (parametric `SELECT`)                       |
| `DefaultScope`  | `default_scope -> { ... }`           | (`_order = '...'` + domain heuristic)    | (Manager default)                       | (none direct)                          | (none direct)                               |
| `Callback`      | `before_save :method`                | `@api.depends`, `@api.onchange`          | `pre_save`/`post_save` signals          | `before_insert`/changeset functions    | `DEFINE EVENT trigger ON t WHEN ...`       |
| `Validation`    | `validates :col, presence: true`     | `@api.constrains('col')`                 | `field.validators=[...]`               | `validate_required`/`validate_length`  | `DEFINE FIELD col ON t ASSERT $value != NONE` |
| `Workflow`      | (state_machine gem)                  | `_inherit = 'mail.activity.mixin'`+state | (django-fsm)                           | (Gen.Statem outside Ecto)              | (DEFINE EVENT cascade)                      |
| `ComputedField` | (instance method)                    | `total = fields.Float(compute=...)`      | (`property` decorator)                 | (function on schema)                   | (`VALUE` expression on field)               |

### 2.2 Cardinality matrix (Role × side-of-FK)

| Role            | Cardinality | FK lives on                          |
|-----------------|-------------|--------------------------------------|
| `MemberOf`      | N:1         | **self** (`x_id` column)             |
| `OwnsOne`       | 1:1         | target (`self_id` on target)         |
| `OwnsMany`      | 1:N         | target                               |
| `GroupOwnsOne`  | 1:1 via Y   | join model Y                         |
| `GroupOwnsMany` | M:N via Y   | join model Y                         |
| `Include`       | -           | n/a (mixin)                          |
| `ClassInclude`  | -           | n/a (class-method mixin)             |
| `Delegate`      | N:1         | self (`tpl_id`) + auto-forward       |

## 3. Path syntax variants

Five variants, all bidirectional via `Identity::parse` and the
matching `to_*` serializer. Every variant round-trips losslessly.

### 3.1 Compact form

Sparse, readable, current OGAR default.

```
ogit-op/WorkPackage
ogit-op/WorkPackage->project
ogit-op/WorkPackage.subject
ogit-op/WorkPackage::scope::open
ogit-op/WorkPackage::callback::0::after_create
ogit-op/WorkPackage@v3->project
acme.ogit-op/WorkPackage->project
```

Separators by role-kind:
- `/` = prefix → class
- `->` = class → association (any Role::*Member*)
- `.` = class → attribute / field name
- `::` = class → synthetic namespace (scope / callback / etc.)
- `@v<n>` = version pin
- `<tenant>.<prefix>` = tenancy

### 3.2 Pathlike form (the "sexy" one)

Uniform `::` separator + semantic role words in the path.

```
ogit-op::WorkPackage
ogit-op::WorkPackage::memberof::project
ogit-op::WorkPackage::members::line_items
ogit-op::WorkPackage::group::members::tags         # HABTM
ogit-op::WorkPackage::has_one::profile
ogit-op::WorkPackage::group::has_one::profile      # has_one :through
ogit-op::WorkPackage::include::Mentionable
ogit-op::WorkPackage::class::include::Helpers      # extend
ogit-op::WorkPackage::delegate::ProductTemplate    # Odoo _inherits
ogit-op::WorkPackage::Enum::status
ogit-op::WorkPackage::attribute::subject
ogit-op::WorkPackage::store::cause
ogit-op::WorkPackage::scope::open
ogit-op::WorkPackage::default_scope
ogit-op::WorkPackage::callback::0::after_create
ogit-op::WorkPackage::validation::0::subject
ogit-op::WorkPackage::workflow::sale_order_state   # extension
ogit-op::WorkPackage::computed::total              # extension
ogit-op::WorkPackage@v3::members::line_items
acme.ogit-op::WorkPackage::members::line_items
```

**Carve-out**: the role word IS the URI's machine-readable
discriminator. The `ogar:kind` triple is REDUNDANT under pathlike
form and SHOULD NOT be emitted when pathlike is the storage
canonical (saves 1 triple/association). When compact is the
canonical, `ogar:kind` IS emitted (because the URI doesn't carry it).

### 3.3 Elixir module-path form

PascalCase module path + dotted role words. Matches `defmodule
MyApp.Accounts.User` style.

```
OgitOp.WorkPackage
OgitOp.WorkPackage.belongs_to.project
OgitOp.WorkPackage.has_one.profile
OgitOp.WorkPackage.has_many.line_items
OgitOp.WorkPackage.many_to_many.tags                # Ecto term for habtm
OgitOp.WorkPackage.use.Mentionable                  # `use` in Elixir
OgitOp.WorkPackage.enum.status
OgitOp.WorkPackage.attribute.subject
OgitOp.WorkPackage.scope.open
OgitOp.WorkPackage.callback.0.after_create
OgitOp.WorkPackage.v3.has_many.line_items           # version pin
Acme.OgitOp.WorkPackage.has_many.line_items         # tenant
```

**Carve-out**: PascalCase conversion on prefix: `ogit-op` →
`OgitOp`, `ogit-erp` → `OgitErp`, `ogar-extensions` →
`OgarExtensions`. The parser is case-insensitive on the prefix
when round-tripping (canonical form preserves `ogit-op`).

### 3.4 Dotted form (debug / log-friendly)

Pure dots — for human-readable logging where uniform separator
matters but Elixir capitalization is overkill.

```
ogit-op.WorkPackage
ogit-op.WorkPackage.belongs_to.project
ogit-op.WorkPackage.has_many.line_items
```

**Caveat**: Odoo class names contain `.` (`sale.order`). In the
dotted form, the class segment is bracketed: `ogit-erp.[sale.order]`.
Compact + Pathlike forms have no such constraint.

### 3.5 Atom-style (Elixir/Erlang interop)

For when OGAR identities cross the Erlang wire as via-tuples.

```elixir
{:via, OgitOp.Registry, {OgitOp.WorkPackage, id}}
{:via, OgitOp.Registry, {OgitOp.WorkPackage, :has_many, :line_items}}
```

Used exclusively by the future `lance-graph-callcenter` Elixir
companion (Sprint 1e). Not a storage form — never written to
lance-graph triples.

## 4. Triple emission rules

Triples vs URI carry the same information twice if both have the
role. To prevent dilution, the rules are:

### 4.1 When canonical form is COMPACT (current Sprint 1 default)

- Role is NOT in URI → emit `ogar:kind` triple to carry role.
- Identity URI is `{prefix}/{class}` for class, `{prefix}/{class}->{target}` for any *Member* role.
- Emitted triples per association:
  ```
  {assoc}  rdf:type           ogar:Association
  {assoc}  ogar:kind          ogar:OwnsMany     ← REQUIRED under compact
  {assoc}  ogar:relationName  line_items
  ```

### 4.2 When canonical form is PATHLIKE (Sprint 1c+ default)

- Role IS in URI → `ogar:kind` triple OMITTED (URI is self-describing).
- Identity URI is `{prefix}::{class}::{role_word}::{target}`.
- Emitted triples per association:
  ```
  {assoc}  rdf:type           ogar:Association
  {assoc}  ogar:relationName  line_items
  ```

### 4.3 Always-emitted attribute triples (both forms)

Regardless of canonical, the following STAY as triples:
- `ogar:targetClass` — when explicit class_name override differs from relation name
- `ogar:foreignKey`, `ogar:polymorphic`, `ogar:through`, `ogar:sourceAlias`, `ogar:asTarget`, `ogar:dependent`, `ogar:optional`, `ogar:inverseOf`, `ogar:scopeSource`
- `ogar:beforeAdd`, `ogar:afterAdd`, `ogar:beforeRemove`, `ogar:afterRemove`

These are **attributes of the edge**, not identities.

### 4.4 The Identity is the URI; metadata is the triples

**Carve-out**: anything that can vary independently of identity-equality stays as a triple. Examples:
- A `belongs_to :project` and a `belongs_to :project, optional: true` have the **same** Identity. The `optional: true` is a triple, not part of the URI.
- A `has_many :line_items` and `has_many :line_items, -> { where(active: true) }` have the **same** Identity. The scope_source is a triple.

This is the canonical rule: **Identity-equality = same conceptual
entity**. Refactoring an attribute (adding `optional:`) does not
change Identity; refactoring the *kind* (belongs_to → has_one)
**does** change Identity (under pathlike) and triggers a
ontology-version bump.

## 5. Edge cases — explicit decisions

### 5.1 Polymorphic `belongs_to`

```ruby
class Comment < ApplicationRecord
  belongs_to :commentable, polymorphic: true
end
```

- Role: `MemberOf`
- Target: `commentable`
- Triples: `ogar:polymorphic true`, **no** `ogar:targetClass`
- Identity: `ogit-op::Comment::memberof::commentable` (no target class — runtime-resolved)

### 5.2 Through associations

```ruby
has_many :tags, through: :taggings
```

- Role: `GroupOwnsMany`
- Target: `tags`
- Triple: `ogar:through taggings`
- Identity: `ogit-op::WorkPackage::group::members::tags`

The `taggings` intermediate is NOT in the URI — it's an
implementation attribute. Two `has_many :tags, through: :X` and
`has_many :tags, through: :Y` on different classes have **distinct**
identities because their class differs; on the **same** class they
collide and the second `has_many :tags` is an error (Rails enforces
this — relation names unique per class).

### 5.3 HABTM vs has_many :through

Rails distinguishes `has_and_belongs_to_many :tags` (HABTM, implicit
join) from `has_many :tags, through: :taggings` (explicit through).
**OGAR collapses both to `GroupOwnsMany`** because:
- The relation IS M:N either way.
- The distinction is implementation-detail (join model exposed or not).
- Ecto's `many_to_many` makes the same collapse.

The presence/absence of `ogar:through` triple distinguishes:
- `GroupOwnsMany` + `ogar:through Y` = explicit through
- `GroupOwnsMany` + no `ogar:through` = HABTM (Rails) or `many_to_many` without explicit join (Ecto)

### 5.4 Odoo `_inherit` vs `_inherits`

| Odoo construct                                | OGAR Role     |
|-----------------------------------------------|---------------|
| `_inherit = 'mail.thread'`                    | `Include`     |
| `_inherit = ['mail.thread', 'mail.activity']` | two `Include` |
| `_inherits = {'product.template': 'tpl_id'}`  | `Delegate`    |

**Carve-out**: `Delegate` always carries a `ogar:foreignKey` triple
(the field name `tpl_id`). `Include` does not.

### 5.5 Class-level vs instance-level mixins

Rails has:
- `include M` — adds M's methods as instance methods → Role `Include`
- `extend M` — adds M's methods as class methods → Role `ClassInclude`

These are SEMANTICALLY different (the methods live on different
objects). Carving them separately prevents downstream consumers
(SurrealQL emitter, PG emitter) from generating wrong DDL.

Elixir's `use M` and `import M` are not direct equivalents — `use`
triggers a `__using__/1` macro that can do either or both. We
encode `use` as `Include` because that's the dominant use; if a
specific `__using__` only adds class-level (compile-time) helpers
the producer SHOULD emit `ClassInclude` instead. This is a
**producer responsibility**.

### 5.6 `default_scope` vs named scopes

`default_scope` is a class-level filter that applies to ALL
queries. `scope :name` is a NAMED filter callable on demand. They
are SEPARATE roles because:
- `default_scope` has no name (no `target`).
- A class has at most ONE `default_scope` but N named `Scope`s.

OGAR carving:
- `Role::DefaultScope` — `target` is `None`. URI: `ogit-op::WorkPackage::default_scope`. Body in `ogar:scopeBody` triple.
- `Role::Scope` — `target` is the scope name. URI: `ogit-op::WorkPackage::scope::open`. Body in `ogar:scopeBody` triple.

### 5.7 Ecto has no `default_scope` equivalent

Producers reading Ecto schemas MUST NOT emit `DefaultScope`. The
closest Ecto pattern is a wrapping function (`def query, do: from(t
in __MODULE__, where: ...)`) but it's CONVENTION not declaration —
producers should not infer `DefaultScope` from it.

### 5.8 Duplicate callbacks / validations

Rails legally allows multiple `after_create do ... end` blocks
declared in sequence. OGAR carves these as:
- `Role::Callback` with `target = "after_create"` and a positional
  index field (stored separately in the `Identity` struct, NOT in
  `Role`).
- URI: `ogit-op::WorkPackage::callback::0::after_create`,
  `ogit-op::WorkPackage::callback::1::after_create`.

The index segment is REQUIRED for `Callback` and `Validation`
roles. The first declaration is `::0::`, never bare. This is
EXPLICIT to prevent silent collision.

### 5.9 Mixed-kind on same target name

Rails forbids:
```ruby
class X < ApplicationRecord
  belongs_to :project
  has_many  :project   # error
end
```

But across mixins, two `Include`s could both define
`belongs_to :project`. OGAR carves this as **producer error**: the
producer must merge / detect / reject before emitting OGAR IR. The
emitter assumes uniqueness of `(prefix, class, target)` per
association-role.

## 6. Versioning, tenancy, hot reload

### 6.1 Version segments

Identity may carry `@v<n>` for ontology version pinning.

- `ogit-op::WorkPackage` = latest version (most common usage).
- `ogit-op::WorkPackage@v3` = explicitly version 3.
- `ogit-op::WorkPackage@v3::members::line_items` = version-pinned association.

Version applies to the **class** (the ontology declaration), not
to instances. An instance is `ogit-op::WorkPackage::<id>` (instance
IDs are out of scope for this document — see lance-graph dataset
spec).

### 6.2 Tenant segments

Tenancy is a LEADING segment separated by `.` (dot).

- `acme.ogit-op::WorkPackage` = tenant `acme`, prefix `ogit-op`.
- `globex.ogit-op::WorkPackage` = different tenant, same prefix.

**Carve-out**: `<tenant>` MUST be alphanumeric + underscore + hyphen
only. No `.`, `/`, `::`, `@`. The parser uses this to disambiguate
tenant from prefix.

### 6.3 Hot reload semantics

When the ontology bumps from v3 to v4:
- Existing `ogit-op::WorkPackage` triples remain addressable as
  `ogit-op::WorkPackage@v3` (historical).
- New triples land under `ogit-op::WorkPackage@v4`.
- Bare `ogit-op::WorkPackage` resolves to latest (v4).
- Callcenter dispatch routes new messages to v4 actor; in-flight
  messages on v3 actor drain to completion.

This requires the callcenter (Sprint 7) to maintain a versioned
actor registry — see `.claude/PLAN.md` Sprint 7 notes for the
implementation sketch.

## 7. Parser disambiguation algorithm

Pseudocode for `Identity::parse(s) -> Result<Identity>`:

```
1. Locate version pin "@v<n>" anywhere after the class segment.
   Strip it; remember n.

2. Locate tenant prefix if present:
   - Split on first '.'.
   - If left side matches /^[a-zA-Z0-9_-]+$/ AND right side starts
     with a known prefix root ("ogar", "ogit", "ogit-erp", ...),
     the left is tenant. Else no tenant.

3. Locate the prefix-class boundary by trying separators in order:
   a. `::` (pathlike) — if the segment after `::` is PascalCase or
      looks like a class.
   b. `/` (compact) — first `/` separates prefix from class.
   c. `.` (Elixir / dotted) — last `.` before any role word, OR
      first `.` after the prefix root.

4. If no role separator found after the class, return Identity
   with role=None.

5. Else parse role:
   - Compact: '->'  followed by target → MemberOf/OwnsOne/OwnsMany
     (kind is in a triple, not the URI).
   - Compact: '.<field_name>' → Attribute.
   - Compact: '::scope::<name>' → Scope.
   - Compact: '::callback::<i>::<event>' → Callback { index: i },
     target = event.
   - Pathlike: '::<role_word>::<target>' → look up role_word in
     ROLE_KEYWORDS table.
   - Elixir: '.<role_word>.<target>' → same lookup.

6. Return Identity { prefix, class, role, target, version }.
```

### 7.1 ROLE_KEYWORDS table

Each row lists the path-segment keyword(s) recognized for each Role,
across all syntax variants. Parser accepts ANY keyword in the row;
serializer emits the canonical (first) keyword.

| Role            | Canonical keyword | Accepted aliases                            |
|-----------------|-------------------|---------------------------------------------|
| `MemberOf`      | `memberof`        | `belongs_to`, `member_of`, `parent`         |
| `OwnsOne`       | `has_one`         | `owns_one`, `member` (singular)             |
| `OwnsMany`      | `members`         | `has_many`, `owns_many`                     |
| `GroupOwnsOne`  | `group::has_one`  | `has_one_through`, `through_one`            |
| `GroupOwnsMany` | `group::members`  | `has_many_through`, `many_to_many`, `habtm`, `has_and_belongs_to_many` |
| `Include`       | `include`         | `use`, `_inherit`, `concern`                |
| `ClassInclude`  | `class::include`  | `extend`                                    |
| `Delegate`      | `delegate`        | `_inherits`                                 |
| `EnumOf`        | `Enum`            | `enum`, `selection`                         |
| `Attribute`     | `attribute`       | `field`, `attr`                             |
| `StoreField`    | `store`           | `store_accessor`, `json_field`              |
| `Scope`         | `scope`           | (none — Ecto has no direct term)            |
| `DefaultScope`  | `default_scope`   | (none)                                      |
| `Callback`      | `callback`        | `hook`, `event`                             |
| `Validation`    | `validation`      | `validates`, `validate`, `constraint`       |
| `Workflow`      | `workflow`        | `state_machine`, `fsm`                      |
| `ComputedField` | `computed`        | `compute`, `derived`                        |
| `Unknown`       | `unknown`         | (parser error if encountered)               |

## 8. Reserved tokens

The following path segments are reserved and MAY NOT be used as
class names, target names, or tenant segments:

```
class    instance    static    self
member   members     memberof  member_of
has      has_one     has_many  has_and_belongs_to_many
belongs  belongs_to  member_in
group    through     via
include  use         extend    concern   _inherit  _inherits
delegate
scope    default_scope    members_in
callback hook        event
validation           validate  validates  constraint
Enum     enum        selection
attribute            field     attr      column
store    store_accessor       json_field
workflow             fsm       state_machine
computed             compute   derived
unknown
v0 v1 v2 ... v999    (numeric version pins)
```

A class named `Member` would conflict with the role word. The
parser rejects such class names from the source AST as a producer
error. (Reality check: no real-world ORM ships a class literally
called `Member` AS A RESERVED-TOKEN class — they're always
namespaced like `Auth::Member` which parses fine as `Auth/Member`.)

## 9. Cross-references

- `.claude/VISION.md` — the one-page synthesis
- `.claude/PLAN.md` — sprint roadmap (Sprint 1c = parser, 1d = Elixir, 1e = `:via`-tuple)
- `.claude/AGENTS.md` — extension rules
- `crates/ogar-vocab/src/lib.rs` — Rust types (the `Class` / `Association` / etc. structs that hold the data this Identity references)
- `crates/ogar-ontology/src/lib.rs` — current `class_identity` / `field_identity` / `association_identity` helpers (will be subsumed by `Identity` struct in Sprint 1c)
- `crates/ogar-emitter/src/lib.rs` — current triple emission (will be updated in Sprint 1c to take `Identity` instead of separate prefix+class+target args)
- `vocab/ogar.ttl` — the canonical Turtle vocabulary (will gain `ogar:Unknown` as an AssociationKind variant, plus the new role-word predicates that are currently missing — see `.claude/board/EPIPHANIES.md` 2026-06-04 entry on TTL drift)

## 10. Carve-outs summary (the non-negotiable list)

1. **Identity-equality = same conceptual entity.** Attributes vary, identity doesn't.
2. **Role kind is in URI for pathlike, in triple for compact.** Never both.
3. **`Callback` and `Validation` always carry an index.** First is `::0::`.
4. **Tenant uses `.`, prefix-class uses `/` or `::`, version uses `@v<n>`.** No mixing.
5. **Odoo dotted class names (`sale.order`) stay verbatim.** No normalisation.
6. **Reserved tokens cannot be class/target/tenant names.** Producer error if encountered.
7. **HABTM and `has_many :through` collapse to `GroupOwnsMany`** — the through-target lives in a triple, not the URI.
8. **`Include` ≠ `ClassInclude` ≠ `Delegate`.** Three distinct semantics; never collapsed.
9. **`DefaultScope` has no name (no target).** Only one per class.
10. **Polymorphic `belongs_to` has no targetClass triple.** Runtime-resolved.
11. **The parser accepts all variants; the canonical serializer emits one.** Configurable per dataset.
12. **Version segment, when present, applies to the class only.** Never to a sub-aspect of the URI.

These twelve are the **drift-prevention contract**. Future sessions
that violate them are wrong; the correct response is to fix the
session, not relax the contract.
