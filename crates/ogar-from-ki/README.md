# ogar-from-ki

A reader for HIRO Knowledge Items, for the language the HIRO User Guides 7.0
document.

This is a reader. It turns text into a typed tree and nothing more.

- Nothing is evaluated.
- No condition is lowered to a guard or a query.
- `on` versus `when` scopes are recorded as data and are not enforced by any
  engine here.
- No function is run, and no function other than `action` is given a meaning.

One reader, three projections:

1. `parse` reads the text into a typed tree with line and column spans.
2. `lift` turns the tree into `ogar_vocab::ActionDef`s (one per `action(...)`
   call) plus the item's facts and typed notes for everything it could not
   state. This is the execution view: the defs register as methods through the
   existing converter.
3. `edges` and `interfering_pairs` derive plain analysis rows from the lifted
   items. Static and name-level; not an evaluation.

`KnowledgeItem::condition_refs` is a fourth view: one row per variable named
in an `on` or `when` condition, with its section, the scope prefix written and
the documented lookup order. Data only.

No parser generator and no third-party dependency (only `ogar-vocab`).

## Sources

The language is read from "KI Syntax" in the HIRO User Guides 7.0
(https://user-guides.engine.datagroup.de/7.0/ki-syntax/). The guide has no
formal grammar. It gives informal templates, tables and examples. Where a rule
is not on a page, the reader refuses the form by name.

The first version of this crate was written from three examples on the
developer portal, before the guide was found. Those three stay as fixtures in
`tests/fixtures/` (see `tests/fixtures/SOURCES.md`). The ten complete items of
the guide are fixtures in `tests/fixtures/guide/` (see
`tests/fixtures/guide/SOURCES.md`), copied byte for byte.

## What is read, and where the guide says it

Page names are the last part of the guide URL
(`.../ki-syntax/definitions/<page>/` or `.../ki-syntax/functions/<page>/`).

| construct | page |
|---|---|
| four sections `ki`, `on`, `when`, `do`, in this order, each once | `ki-structure` |
| header keys `name`, `description`, `version`, `tier` | `ki-structure` |
| `tier`: `"default"`, `"policy"`, `"fallback"` (typed; anything else is refused) | `ki-structure`, "KI Tiers" |
| existence (a bare name), `not` | `matching`, "Exists", "Not Exists" |
| `==` `!=` `===` `!===` `!==` `>=` `<=` `>` `<` `=~` `!~` | `matching`, table 1 and the "Equal" to "Not Regex" sections |
| `and`, `or`, parentheses; `not` over the next expression and over `or`; lines joined by an implicit `and` | `matching`, "Not Exists", "Or", "And"; `ki-structure` example |
| regex literals `/pattern/flags` (kept as text, groups not interpreted) | `matching`, "Regex" |
| keys `Var["key"]`, `Var[1]`, `Var[Other]` | `matching`, "Optional Match"; `ki-structure` |
| `list X`, `X as Alias`, `Foo == "x" as Alias` | `matching`, "Alias Match", "Lists vs Values" |
| scope prefixes `NODE::`, `ISSUE::`, `LOCAL::`, and `Alias::name` | `scope` |
| graph limiters `@any`, `@self` | `matching`, "Context aware variable value scoping" |
| optional match `? X := d`, `? X < 5 := 0`, `?list X` | `matching`, "Optional Match" |
| arithmetic on a right side: one `+ - * / %` | `varstrings`, "Mathematical operations" |
| `with <type>@<dir>[:<edge>] [as Alias] ... end` in `on` | `ki-structure`, "Related Node Matching"; `matching`, "Related nodes" |
| `count <type>@<dir>[:<edge>] as Name ... end` in `on` | `matching`, "Related count" |
| directions `in`, `out`, `connected`; edge type optional | `ki-structure` |
| `#` comments | `ki-structure`, "Comments" |
| assignment `Var = value`, with prefix, key, limiter, list literal | `values`, `scope` |
| `set(Var = v)`, `add(Var = v)`, `delete(Var)`, `delete(Var@any, created_on: X)` | `values` |
| `log(...)`, `log.debug`, `log.info`, `log.warn`, `log.error` | `log` |
| `if ... then ... [else ...] end`, with `and`, `or`, `not` in the condition | `if`, `ki-structure` |
| tagged results `tag: Var, tag2: Var2 = function(...)` | `functions` |
| a call to any function name, positional then keyword arguments | `functions` |
| `begin ... end` as an argument value (`each:`) | `query`, `read_timeseries` |
| `action(...)` with the capability positional or `capability:` | `action` |

### The accepted language, as implemented

```
item      = [ header ] on when do
header    = "ki" { KEY ":" STRING }          KEY: name description version tier id, each once
on        = "on" { cond | block }            at least one
when      = "when" { cond }                  at least one
do        = "do" { stmt }                    at least one
block     = ("with" | "count") TYPE "@" DIR [ ":" EDGE ] [ "as" ALIAS ]
              { cond } "end"                 count needs ALIAS; no nesting
cond      = or-expr                          one per line
or-expr   = not-expr { "or" not-expr }  |  not-expr { "and" not-expr }
not-expr  = "not" not-expr | "(" or-expr ")" | leaf
leaf      = [ "?" ] [ "list" ] VAR [ OP right ] [ "as" ALIAS ] [ ":=" expr ]
VAR       = NAME [ "[" key "]" ] [ "@" ("any" | "self") ]
stmt      = VAR "=" expr
          | ("set" | "add") "(" VAR "=" expr ")"
          | "delete" "(" VAR [ "," "created_on" ":" expr ] ")"
          | "log" [ "." LEVEL ] "(" expr ")"
          | [ TAG ":" NAME { "," TAG ":" NAME } "=" ] NAME "(" args ")"
          | "if" or-expr "then" stmt+ [ "else" stmt+ ] "end"     branches hold no `if`
args      = { positional } { KEY ":" (expr | "begin" stmt+ "end") }
expr      = operand [ ARITH operand ]
operand   = STRING | NUMBER | VAR | "[" operand { "," operand } "]"
```

Layout. Conditions and statements end at a newline. Inside a call's
parentheses newlines are free, and after a `,` in a tag list too. A line that
starts at column 1 must be a clause keyword. Strings use `"` or `'`, may span
lines and keep `${...}` and backslashes as written. `and` and `or` do not mix
on one level without parentheses.

## What is refused, by name, with line and column

- An operator the matching page does not list (`=`, `<>`, `=>`, `&&`, `|`).
- `and` mixed with `or` without parentheses (`AmbiguousConnectives`).
- A number where a condition starts; two arithmetic operators in one
  expression; an optional `?` match with no `:= default` (only `?list` may
  omit it).
- An unknown tier, header key, direction, graph limiter or log level.
- A `with` or `count` block inside another block, in `when`, empty, or with no
  closing `end`; a `count` with no `as`.
- A call with no argument list (the `action` page prints one); tags in front of
  `log`, `delete`, `set` or `add`; a positional argument after a keyword
  argument; a call inside an argument or a value; a duplicate argument; an
  `action` with no capability, two capabilities or an empty one.
- An `if` inside an `if` branch (the guide shows none), stray `then`, `else`,
  `end`, an `if` with no `end`, an empty branch.
- A `?`, `list` or `as` in an `if` condition.
- An unknown clause keyword, a missing, empty, repeated or out-of-order clause,
  a second item in one text.
- An unterminated string or regex; a backslash directly before the closing
  quote of a string; a character that starts no token (`{`, `;`, `$`, ...).

### Documented by the guide, not read yet

These are refused with `DocumentedNotRead` or `CallInExpression`, so a later
version can read them without changing what is accepted today.

- `has` (`list A has "A"`), `in [...]`, the `|` inside `has`.
- Function calls on the left of a condition: `count(A) > 3`,
  `starts_with?(A, "foo")`, `slice(A, 0, 3) =~ ...`.
- A constructed string on the left (`"host-${B}" =~ ...`).
- The multi-variable `alias as Name ... end` block.
- The empty key `VarB[] = VarA`.
- VarString internals (`$[...]`, modifiers, JSON paths): strings are kept as
  written.

## The guide's contradictions, and what the reader does

The reader accepts both documented forms and records which one was written.
It never picks one silently.

| contradiction | handling |
|---|---|
| exact inequality is `!===` in the operator table and `!==` in an example (`matching`) | both read; `CmpOp::ExactNe(ExactNeSpelling::Table)` or `::Example` |
| the full example writes `MonitoringStatus@in` in `when`; the structure page writes `@any` for the same line; `in` is documented only as an edge direction | read as `GraphScopeKind::InAsWrittenInExample`, never as `@any` |
| `on` may use NODE and LOCAL, `when` ISSUE and LOCAL (`scope`), yet the matching page writes `NODE::Hostname` in a `when` | the row says `outside_permitted_scopes`; nothing is refused |
| the scope page documents `NODE::` and prints `Node::ogit/status` once | both read; `ScopeSpelling::NodeTitleCase` marks the second |
| `parent` and `child` appear in `with` examples (`matching`, `scope`); the structure page lists `in`, `out`, `connected` | `Direction::ExampleOnlyParent` and `::ExampleOnlyChild`; `is_documented()` is false for them |
| the structure page says `name` is mandatory; the RL guide's example writes `id:` | `id` is its own header key; no header key is enforced as mandatory (one of the three portal examples has no header) |
| `version` is marked mandatory in the `ki` list; the mandatory sentence leaves it out | not enforced |
| the `if` page calls `else` optional | read; `IfStmt::else_branch` is an `Option` (an earlier version refused it) |
| the `if` page uses `and` and `or`; the matching page states `not` over `or` and no order for `and` against `or` | both read; mixing them without parentheses is refused |
| the "Related count" example writes `ogit/_type = ogit/Person` with one `=`; the table lists only `==` | refused as `UnsupportedOperator`; not read as `==` |
| the `action` page prints a call with no argument list and a call with a positional capability although the usage line says `capability:` | the first is refused by name (`CallWithoutArguments`); the positional capability is read, as the argument table's "Position 1" lists it |
| the `log` page prints `log<tab>info(...)` where `log.info` belongs | refused, not read as `log.info` |
| the query page's Gremlin example has no comma after `root: known_vertex_id` | refused at the next token, not repaired |
| the types page says numeric comparison with a string gives false; the matching page gives `"2bc" > "1.0"` true | nothing is compared here |
| the default graph scope: `matching` says own node, `KI_debugging` says `@any` | nothing is looked up here |
| the guide gives no escape rule for strings, yet two examples hold `\.` and `\/` | a backslash is kept as written; only a backslash before the closing quote is refused |
| nested `with` blocks | not found in the guide; refused (`NestedBlock`) |

## Fixtures

All ten complete items of the guide parse. `tests/guide_examples.rs` asserts
each on its content (header, tier, blocks, kinds of conditions per section,
functions called in `do`). The text of the guide's defects that matter to the
reader is pinned in the same file.

Known facts about the ten, from the guide's own notes:

- The retry item never calls an action; it sets a placeholder. It lifts to no
  def.
- The graph query item's comment says `LOCAL::NODEID` holds a list; the query
  page says `each` runs once per result.

## What stays unknown, and how the lift carries it

| unknown | in the lift |
|---|---|
| what a bare name in `when` or `if` means at run time (the matching page says "exists") | `KiCondition::BareName`, note `UnresolvedBareName`; never an equality, no `kausal` |
| any condition beyond a name or `name == "string"` | `KiCondition::Other` with the names it mentions; note `ConditionBeyondLift`; no `kausal` |
| how several `when` lines combine in a guard | the guide says implicit `and`; the lift states no `kausal` for several lines; note `WhenConjunctionUndocumented` (the variant keeps its first name) |
| what `on ogit/_id` means for classes | `OnTarget::AnyVertex`; `object_class` empty; note `OnAnyVertex` |
| what other `on` forms name (`ogit/name == "..."`, a `with` block first) | `OnTarget::Unresolved`; `object_class` empty; note `OnNotAClass`. A class is found only from `ogit/_type == "X"` among the top-level `on` conditions |
| whether a call argument is mandatory | `CallParam.mandatory = false`, claiming nothing; note `ParamMandatoryUnknown` |
| the meaning of an `if` around a call | the call is lifted; note `ConditionalCall` |
| `execute(...)` | the guide calls it a front end to `action` capabilities chosen by `host` and `scriptmode`; reported as `OtherCallKind::Execute` and note `ExecuteNotLifted`; no def |
| any other function | `KiOtherCall` with the name and the result tags; no def, no meaning |
| ordering, priority, tier selection, several items applicable at once | not modelled; `tier` is carried as a typed fact |

A def with `kausal: None` from this crate does NOT mean "fires
unconditionally": the item's `when` is on `LiftedItem::when`, and the note
`PreconditionNotStated` says so.

## Mapping to `ActionDef`

`identity` = `ki/{item id}::action_def::{n}::{capability}` (item id: the header
`name:`, else the caller's id; `n`: 1-based position among the item's `action`
calls in text order, including calls inside `if` branches and `begin` blocks).
`predicate` = capability. `object_class` = the class of `on ogit/_type ==
"X"`, else empty. `params` = named arguments (values kept on `KiCall`, with a
`KiArgKind`, not on the def). `results` = result tags. `reads` = non-local
names tested by `when` and `if`, now also the names inside complex conditions;
`writes` = non-local names assigned, set, added, deleted or bound by a tag;
`calls` = `[capability]`; `body_source` = the item text. `reads` and `writes`
are the item's, repeated on every def. `LOCAL::` names are item-local, not
stored fields, and are kept on `LiftedItem` instead. Names are kept as written,
so `NODE::Foo` is `NODE::Foo`. `kausal` is a `StateGuard` only for a `when` of
exactly one equality of a plain name with a string.

The dt64 converter (`ogar-action-handler`, `tests/ki_lift_dt64.rs`) refuses a
def with no class (`UnmintedConcept`), refuses a guard whose value is not a
decimal code (`UnportedGuard`), and refuses a read or guard over a name the
class lacks (`UnknownField`, `CallParameterInGuard`).

## Analysis rows

`KiEdge { from, kind, to }`, kinds `AppliesOn`, `Reads`, `Writes`, `Calls`,
`BindsResult`. `interfering_pairs` lists two items when one writes (or deletes)
a name the other reads or writes **and** their `on` clauses can name the same
vertex (same class, or either is `ogit/_id`; an undefined `on` form counts as
any vertex). This is static, name-level analysis: it says nothing about
whether the two ever meet on one vertex, nor about order or values.
