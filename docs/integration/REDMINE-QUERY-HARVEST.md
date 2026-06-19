# Redmine Query Harvest — what 17 years of evolution found, mapped to ClassView

> **Apple meets Apple.** Redmine spent years iterating to a clean
> column-dispatch render pattern (`Query` + `QueryColumn` + helper trio).
> This doc harvests it as a *spec*, maps each piece onto our
> `ClassView` / `ogar-vocab::Class` / `ogar-render-askama` substrate, and
> derives the concrete shape of T2 (`HtmlListView`) so the askama template
> inherits everything Redmine learned without re-living the years.
>
> Read alongside `CLASSVIEW-MATERIALIZATION-PLAN.md` §3 (T2 row). The plan
> says *what* T2 is; this doc says *what its binding struct + template
> shape must be*, based on what Redmine evolved.

## 0. The headline

Redmine's spine for every list page (issues, projects, users, time
entries, project admin) is a **single ERB partial** —
`app/views/issues/_list.html.erb` — that knows nothing about issues
specifically. It iterates `query.inline_columns` and `query.block_columns`,
calls `column_header(query, column)` for thead and
`column_content(column, item)` for tbody, and that's the entire shape.

Their organic discovery: **make the column set dynamic + meta-tagged, and
the template body collapses to one file.** That's the same insight WoA-rs
codified with `RouteSpec`/`HandlerKind` and our Northstar plan codifies
with `ClassView`/`ArtifactKind`. **Redmine already did the design work for
the HTML render side.** We harvest, we don't reinvent.

## 1. The Redmine pattern — what they actually built

### 1.1 `QueryColumn` — the dynamic-column meta object (`app/models/query.rb`:21–)

```ruby
class QueryColumn
  attr_accessor :name, :totalable, :default_order
  attr_writer   :sortable, :groupable

  def initialize(name, options={})
    self.name           = name
    self.sortable       = options[:sortable]
    self.groupable      = options[:groupable] || false
    self.totalable      = options[:totalable] || false
    self.default_order  = options[:default_order]
    @inline             = options.key?(:inline) ? options[:inline] : true
    @caption_key        = options[:caption] || :"field_#{name}"
    @frozen             = options[:frozen]
  end
  # ...
  def value(object);        object.send name; end
  def value_object(object); object.send name; end
end
```

**Eight properties per column** — the *exact* render-meta a generic list
view needs:

| Property | What it controls |
|---|---|
| `name` | The attribute on the record to read (e.g. `:subject`) |
| `caption` | Label for the column header (i18n-resolved key by default) |
| `sortable` | SQL expression or `Proc` for ORDER BY; `nil` = not sortable |
| `default_order` | `'asc'` / `'desc'` — initial sort direction when the column is the active sort |
| `groupable` | Whether the user can choose this column as the group-by axis |
| `totalable` | Whether a per-group / overall total is meaningful (numeric columns) |
| `inline` | `true` = `<td>` in the row; `false` = `<tr class="block_column">` that spans full width (description, last_notes, …) |
| `frozen` | Always visible regardless of user's column selection (typically the checkbox / id column) |

### 1.2 Specialised subclasses — when the default `value` isn't enough

```ruby
class TimestampQueryColumn < QueryColumn      # groupable by date (timezone-aware)
class WatcherQueryColumn   < QueryColumn      # value gated by ACL check
class QueryAssociationColumn        < QueryColumn        # dotted name "parent.subject"
class QueryCustomFieldColumn        < QueryColumn        # cf_42 → CustomField #42
class QueryAssociationCustomFieldColumn < QueryCustomFieldColumn   # both at once
```

What each adds:
- `TimestampQueryColumn` — `groupable?` returns true *only if* a timezone-cast
  SQL expression exists. Groups rows by `DATE(updated_on AT TIME ZONE …)`.
- `WatcherQueryColumn` — overrides `value_object` to return `nil` when the
  current user lacks `view_*_watchers` permission. **ACL-aware columns.**
- `QueryAssociationColumn` — for dotted names like `parent.subject`. Walks
  the family edge before pulling the attribute.
- `QueryCustomFieldColumn` — for custom-field columns; sortable/totalable
  derive from the `CustomField`'s own metadata, name is `:"cf_#{id}"`.

### 1.3 Per-resource Query subclass — declares available columns

```ruby
# app/models/issue_query.rb
class IssueQuery < Query
  self.available_columns = [
    QueryColumn.new(:id, :sortable => "#{Issue.table_name}.id",
                    :default_order => 'desc', :caption => '#', :frozen => true),
    QueryColumn.new(:project, :sortable => "#{Project.table_name}.name", :groupable => true),
    QueryColumn.new(:tracker, :sortable => "#{Tracker.table_name}.position", :groupable => true),
    # ... ~25 columns total
    QueryColumn.new(:description, :inline => false),
    QueryColumn.new(:last_notes, :caption => :label_last_notes, :inline => false),
  ]
  # ...
end
```

Six Query subclasses today: `IssueQuery`, `ProjectQuery`, `ProjectAdminQuery`,
`TimeEntryQuery`, `UserQuery`, `Query` (abstract). One per resource;
each declares its `available_columns` list at class-load time.

### 1.4 The render helpers — three functions (`app/helpers/queries_helper.rb`)

```ruby
def column_header(query, column, options={})        # the <th> + sort-link
def column_content(column, item)                    # the <td> body (handles Array → join)
def column_value(column, item, value)               # ONE value formatted (case stmt per kind)
```

`column_value` is a `case column.name` formatter dispatch:

```ruby
def column_value(column, item, value)
  case column.name
  when :id                  then link_to value, issue_path(item)
  when :subject             then link_to value, issue_path(item)
  when :parent              then ... link_to_issue(value, :subject => false) ...
  when :description         then content_tag('div', textilizable(...), :class => "wiki")
  when :last_notes          then content_tag('div', textilizable(...), :class => "wiki")
  when :done_ratio          then progress_bar(value)
  when :relations           then ...
  when :hours, :estimated_hours, :total_estimated_hours, :estimated_remaining_hours
                            then ...formatted_hours...
  when :spent_hours, :total_spent_hours
                            then ...formatted_hours, link to time entries...
  when :attachments         then ...attachment-list...
  when :watcher_users       then ...user-list...
  else
    format_object(value)    # fallback formatter — handles dates, decimals, refs, etc.
  end
end
```

This case statement is **the formatter library** Redmine built over 17
years. ~12 explicit kinds + `format_object`-as-default that handles dates,
decimals, ActiveRecord references, etc.

### 1.5 The list partial — the *only* template per resource

`app/views/issues/_list.html.erb` (61 LOC, the spine):

```erb
<table class="list issues">
  <thead>
    <tr>
      <th class="checkbox hide-when-print"><%= check_box_tag 'check_all' ... %></th>
      <% query.inline_columns.each do |column| %>
        <%= column_header(query, column, query_options) %>
      <% end %>
      <th class="buttons hide-when-print"></th>
    </tr>
  </thead>
  <tbody>
    <% grouped_issue_list(issues, query) do |issue, level, group_name, group_count, group_totals| %>
      <% if group_name %>
        <tr class="group open"><td colspan="..."><%= group_name %><%= group_count %><%= group_totals %></td></tr>
      <% end %>
      <tr id="issue-<%= issue.id %>" class="<%= cycle('odd', 'even') %> <%= issue.css_classes %>">
        <td class="checkbox"><%= check_box_tag("ids[]", issue.id, ...) %></td>
        <% query.inline_columns.each do |column| %>
          <%= content_tag('td', column_content(column, issue), :class => column.css_classes) %>
        <% end %>
        <td class="buttons"><%= link_to_context_menu %></td>
      </tr>
      <% query.block_columns.each do |column| %>
        <% if (text = column_content(column, issue)).present? %>
          <tr class="<%= current_cycle %>">
            <td colspan="..." class="<%= column.css_classes %> block_column"><%= text %></td>
          </tr>
        <% end %>
      <% end %>
    <% end %>
  </tbody>
</table>
```

**Substrate-agnostic**: nothing in this template knows it's rendering
issues. Swap `query.inline_columns` for any column set + an `each`-yielding
helper, and the same body renders projects, users, time entries, etc.

## 2. Mapping onto our ClassView — point by point

### 2.1 What stays (we just rename)

| Redmine | Our ClassView |
|---|---|
| `Query` model holding `available_columns` | `ClassView` impl holding `ObjectView::fields` |
| `query.column_names` (user's current selection) | `FieldMask` bits the request carries |
| `query.inline_columns` | `FieldMask & inline_set` (the bits whose column has `inline=true`) |
| `query.block_columns` | `FieldMask & block_set` |
| `QueryColumn.name` | `FieldRef.predicate_iri` |
| `QueryColumn.caption` | `FieldRef.label` (late-resolved from cache) |
| `column.value_object(item)` | `view.render_rows(class_id, mask)` → `RenderRow.value` |

### 2.2 What we extend — `FieldRef` needs the render-meta Redmine pinned

`FieldRef { predicate_iri, label }` (current contract) is enough for the
*projection* layer, but the *list-view template* needs every column to
also carry: `sortable`, `groupable`, `totalable`, `inline`, `frozen`,
`default_order`, `kind` (formatter dispatch).

Per Northstar §1.3, **types live on `Class`, labels live on `ClassView`.**
The render-meta is neither pure type nor pure label — it's *presentation
intent*. Two options:

- **Sidecar struct on the binding-struct side** (preferred). The askama
  binding-struct for T2 carries its own `RenderColumn { field_ref,
  sortable, groupable, totalable, inline, frozen, default_order, kind }`,
  built up at call time from `(Class, ClassView)` + a per-target
  presentation policy. The contract crates stay untouched.
- *Not recommended*: extending `FieldRef` to carry presentation flags.
  Anti-pattern #2 territory — same shape as bolting `type_kind` onto
  `RenderRow`. The contract stays minimal; presentation is the consumer's
  concern.

### 2.3 What collapses — N Query subclasses → 0 (one ClassView, M views)

Redmine declares `available_columns` per Query subclass (6 subclasses).
We don't need subclasses: every promoted concept already declares its
`ObjectView::fields` via `OgarClassView`. The user's per-view column
selection (Redmine's `query.column_names` saved as a `Query` record) maps
onto a per-view `FieldMask` that the consumer holds.

So:
- **Redmine**: 6 Query subclasses × ~25 columns each = ~150 column declarations.
- **Ours**: 32 ClassView entries × N fields = ~one canonical declaration.
  The "saved view" (user's column selection + sort + group) is just a
  `FieldMask + sort_criteria` triple held in storage — not a class.

### 2.4 What we adopt 1:1 — the formatter library

`column_value`'s case statement is the *catalog* of cell formatters
Redmine built. Port to askama as **per-kind sub-templates** the main
list-view template dispatches into:

| Redmine `when …` | Our `RenderColumn.kind` | Askama sub-template |
|---|---|---|
| `:id` | `IdLink` | `dispatch/cell/id_link.askama` |
| `:subject` (+ any `string` with primary_label) | `PrimaryLink` | `dispatch/cell/primary_link.askama` |
| `:parent` (record-ref) | `RecordRef` | `dispatch/cell/record_ref.askama` |
| `:description` / `:last_notes` (textilized) | `RichText` | `dispatch/cell/rich_text.askama` |
| `:done_ratio` (percentage) | `ProgressBar` | `dispatch/cell/progress_bar.askama` |
| `:relations` | `RelationList` | `dispatch/cell/relation_list.askama` |
| `:estimated_hours` / `:spent_hours` etc. | `Hours` | `dispatch/cell/hours.askama` |
| `:attachments` | `AttachmentList` | `dispatch/cell/attachment_list.askama` |
| `:watcher_users` | `UserList` | `dispatch/cell/user_list.askama` |
| `format_object` fallback (date, decimal, ref, …) | `Plain` | `dispatch/cell/plain.askama` |

The `RenderColumn.kind` is derived at binding-struct build time from
`(Class::Attribute.type_name, FieldRef.predicate_iri,
ClassView_bits & primary_label)`. Default is `Plain`. Specific kinds
(`ProgressBar` for `done_ratio`, `Hours` for time-typed slots, `IdLink`
for the codebook id) are routed by a small mapping function. **Adding a
new formatter = one sub-template.**

### 2.5 What we get for free that Redmine had to retrofit

- **Custom fields**: Redmine added `QueryCustomFieldColumn` years in;
  we already have `project_custom_field` + `project_custom_value` as
  promoted concepts. A custom-field column is just a row in the canonical
  graph; the same template renders it.
- **ACL on individual columns** (Redmine's `WatcherQueryColumn`): becomes
  a `view_bits & VISIBILITY_MASK` test before the column is included in
  the row stream. UnifiedBridge's policy layer (`lance-graph-callcenter`)
  feeds the `view_bits`; the template never branches on permissions.
- **Cross-curator** (the whole point): `IssueQuery`'s columns are Issue-
  specific; we render OpenProject `WorkPackage` and Redmine `Issue`
  through the *same* `project_work_item` ClassView. No per-curator
  template.

## 3. The concrete T2 spec — `HtmlListView` binding-struct + template

### 3.1 Binding struct

```rust
#[derive(Template)]
#[template(path = "dispatch/html_list_view.askama", escape = "html")]
struct HtmlListViewCtx<'a> {
    /// Page heading. Caller supplies; usually the saved-view name or the
    /// concept's primary_label pluralised.
    title: &'a str,

    /// The class being listed (carries the canonical concept + class_id).
    class_id_hex: String,           // "0x0102"
    canonical_concept: &'a str,     // "project_work_item"

    /// Inline columns — one `<th>` and one `<td>` per row.
    inline_columns: Vec<RenderColumn<'a>>,
    /// Block columns — `<tr class="block_column"><td colspan>…</td></tr>` per row.
    block_columns: Vec<RenderColumn<'a>>,

    /// The rows, one per record. Each row carries pre-projected cells in
    /// the same order as (inline_columns ++ block_columns).
    rows: Vec<RenderRowGroup<'a>>,

    /// Current sort state (highlights the active column + direction).
    sort: SortCriteria<'a>,
    /// Optional group-by axis (renders `<tr class="group">` separators).
    group_by: Option<&'a str>,
}

struct RenderColumn<'a> {
    name: &'a str,           // predicate_iri (e.g. "subject")
    caption: &'a str,        // FieldRef.label, late-resolved
    sortable: bool,
    groupable: bool,
    totalable: bool,
    inline: bool,            // false → block column
    frozen: bool,            // never hideable
    default_order: SortOrder,
    kind: ColumnKind,        // dispatch tag for the cell sub-template
}

enum ColumnKind {
    Plain, IdLink, PrimaryLink, RecordRef, RichText,
    ProgressBar, RelationList, Hours, AttachmentList, UserList,
    // Append-only; new variants = one new cell/* template.
}

struct RenderRowGroup<'a> {
    /// Optional group separator before this row (Some for first row of a
    /// new group). Carries (label, count, totals_html).
    group_header: Option<GroupHeader<'a>>,
    record_id: u64,
    css_classes: &'a str,    // "hascontextmenu odd" etc.
    cells: Vec<RenderCell<'a>>,  // same order as columns
}

struct RenderCell<'a> {
    css_classes: &'a str,
    /// The pre-formatted body of the cell. For `Plain` kind this is the
    /// late-resolved string from RenderRow; for richer kinds, the sub-
    /// template's already-rendered output.
    body_html: askama::filters::SafeString,
}
```

### 3.2 Template skeleton (the spine — same shape as `_list.html.erb`)

```jinja
{# templates/dispatch/html_list_view.askama #}
{% extends "_base.askama" %}
{% block title %}{{ title }}{% endblock %}
{% block content %}
<h2>{{ title }}</h2>

<form method="get" class="query-form">
  {% include "dispatch/_filter_bar.askama" %}
</form>

<div class="autoscroll">
<table class="list" data-class-id="{{ class_id_hex }}" data-concept="{{ canonical_concept }}">
  <thead>
    <tr>
      <th class="checkbox"><input type="checkbox" class="toggle-selection"></th>
      {% for col in inline_columns %}
        {% include "dispatch/_column_header.askama" %}
      {% endfor %}
      <th class="buttons"></th>
    </tr>
  </thead>
  <tbody>
    {% for row in rows %}
      {% if let Some(g) = row.group_header %}
        <tr class="group open">
          <td colspan="{{ inline_columns.len() + 2 }}">
            <span class="name">{{ g.label }}</span>
            {% if g.count.is_some() %}<span class="badge">{{ g.count.unwrap() }}</span>{% endif %}
            <span class="totals">{{ g.totals_html|safe }}</span>
          </td>
        </tr>
      {% endif %}
      <tr id="record-{{ row.record_id }}" class="{{ row.css_classes }}">
        <td class="checkbox"><input type="checkbox" name="ids[]" value="{{ row.record_id }}"></td>
        {% for cell in row.cells.iter().take(inline_columns.len()) %}
          <td class="{{ cell.css_classes }}">{{ cell.body_html|safe }}</td>
        {% endfor %}
        <td class="buttons"></td>
      </tr>
      {% for cell in row.cells.iter().skip(inline_columns.len()) %}
        {% if !cell.body_html.is_empty() %}
        <tr class="{{ row.css_classes }}">
          <td colspan="{{ inline_columns.len() + 2 }}" class="block_column {{ cell.css_classes }}">
            {{ cell.body_html|safe }}
          </td>
        </tr>
        {% endif %}
      {% endfor %}
    {% endfor %}
  </tbody>
</table>
</div>

{# pagination, action menu, totals — additive includes #}
{% endblock %}
```

### 3.3 Cell sub-templates (one per `ColumnKind`)

Each is mass-mail simple — bag of variables the kind needs. Example:

```jinja
{# templates/dispatch/cell/progress_bar.askama #}
<div class="progress" role="progressbar" aria-valuenow="{{ pct }}" aria-valuemin="0" aria-valuemax="100">
  <div class="progress-bar" style="width: {{ pct }}%">{{ pct }}%</div>
</div>
```

```jinja
{# templates/dispatch/cell/id_link.askama #}
<a href="{{ href }}" class="record-id">#{{ id }}</a>
```

The dispatch from `ColumnKind` → sub-template happens **once in Rust**
(at row-build time the emitter pre-renders each cell into `body_html`),
so the spine template just emits `{{ cell.body_html|safe }}`. **No
runtime polymorphism on the askama side** — each sub-template is a fixed
binding-struct, called when the kind matches.

### 3.4 What this collapses, concretely

| | Redmine | Our T2 |
|---|---|---|
| List partials | 26 (`app/views/<resource>/_list.html.erb` per controller, often near-identical) | 1 (`html_list_view.askama`) |
| Query model subclasses | 6 (`IssueQuery`, `ProjectQuery`, …) | 0 (ClassView is the spine) |
| Column declarations | ~150 (per-subclass `available_columns` literals) | already in the codebook (32 promoted concepts × their fields) |
| Cell formatter case stmt | 1 mega-`case` in `column_value` | ~10 sub-templates, 1 per `ColumnKind` |
| Per-resource helpers | ~50 helper modules | filters / per-kind sub-templates |

## 4. What we still need (concrete TODO surface for T2's PR)

- [ ] `RenderColumn { kind, sortable, groupable, … }` struct in
      `ogar-render-askama` (binding-struct sidecar; doesn't touch
      lance-graph-contract).
- [ ] `kind_from_field(field_ref, type_name) -> ColumnKind` resolver —
      the small mapping that picks `IdLink` for `id`, `ProgressBar` for
      `done_ratio`/percent-typed slots, `RichText` for `text`-typed
      attributes whose name matches `description|notes|comment`, else
      `Plain`. Append-only; defaults are safe.
- [ ] `html_list_view.askama` + the cell sub-templates (~10).
- [ ] `HtmlListViewEmitter` impl — wires `(Class, ClassView, FieldMask,
      sort, group_by, rows)` into `HtmlListViewCtx`.
- [ ] Tests: round-trip render against a fabricated `project_work_item`
      row set; pin a golden snapshot of the emitted HTML.

## 5. Where each piece lives

| Piece | Crate / repo |
|---|---|
| `ClassView` trait + `FieldMask` + `FieldRef` | `lance-graph-contract::class_view` (already shipped) |
| `OgarClassView` impl over 32 concepts | `ogar-class-view` (PR #77, merged) |
| `RenderColumn` / `ColumnKind` / `HtmlListViewCtx` | `ogar-render-askama` (T2 PR) |
| `dispatch/html_list_view.askama` + cell sub-templates | `ogar-render-askama` (T2 PR) |
| Per-resource saved view storage (column selection, sort, group) | downstream consumer (op-codegen-projection / op-server-renderer) |
| Authorization gate on column visibility | `lance-graph-callcenter::UnifiedBridge` (existing pattern) |

## 6. The Apple-meets-Apple summary

| What Redmine evolved | What we inherit |
|---|---|
| Dynamic columns dispatched through ONE template | Same shape; `inline_columns`/`block_columns` are mask slices |
| `QueryColumn` meta (sortable, groupable, totalable, inline, frozen, default_order) | `RenderColumn` mirrors it 1:1; lives in binding-struct, not in contract |
| Specialized column subclasses for timestamps / watchers / associations / CFs | Replaced by `ColumnKind` enum + kind-aware sub-templates (no inheritance) |
| `column_value` 12-arm formatter `case` | One cell sub-template per `ColumnKind` |
| 6 per-resource Query subclasses | 0 — ClassView is the spine; user-saved selections are `FieldMask` + sort triples |
| 50 helpers | Askama filters + binding-struct accessors |
| 17 years of accumulated cell rendering | Lifted as the spec for T2's askama kit |

Redmine showed the shape works. We replace the substrate (untyped jsonb
+ Ruby helpers + per-controller views) with the typed canonical layer +
askama kit. The rendered HTML can be byte-identical where the design
intends; the production cost is bounded by the kit, not by the resource
count.
