# ogar-from-ki

A reader for HIRO Knowledge Items, for the **documented subset** only.

One reader, two projections (`HIRO_PARITY_INTEGRATION_PLAN.md` 3a):

1. `parse` reads the text into a typed tree with line and column spans.
2. `lift` turns the tree into `ogar_vocab::ActionDef`s (one per `action(...)`
   call) plus the item's facts and typed notes for everything it could not
   state. This is the execution view: the defs register as methods through the
   existing converter.
3. `edges` and `interfering_pairs` derive plain analysis rows from the same
   lifted items. Static and name-level; not an evaluation.

No parser generator and no third-party dependency (only `ogar-vocab`).

## Sources

The public HIRO 7.0 developer portal shows a Knowledge Item as text in clauses
and gives three short examples; it gives **no grammar**. The three sources are
recorded in `tests/fixtures/SOURCES.md` and kept byte for byte as fixtures:

| fixture | page |
|---|---|
| `tutorial_nothing_to_do.ki` | https://dev-portal.engine.datagroup.de/7.0/tutorials/automation-tasks/ |
| `http_request.ki` | https://dev-portal.engine.datagroup.de/7.0/documentation/integration/Standard_ActionHandlers/AHconfig/ |
| `ssh_execute_command_line.ki` (one `do` statement) | the same AHConfig page |

**No real-world Knowledge Item beyond these three public examples has been
seen.** Everything below that goes past them is an implementation choice and is
marked as such.

## The accepted subset

```
item      = [ header ] on when do                 one item per text
header    = "ki" { "name:" STRING | "description:" STRING }   each at most once
on        = "on" cond                             exactly one condition
when      = "when" cond { cond }                  one condition per line, at least one
do        = "do" stmt { stmt }                    at least one
cond      = NAME [ "==" STRING ]                  a bare name alone, or a comparison
stmt      = NAME "=" ( NUMBER | STRING )
          | [ FIELD ":" LOCAL { "," FIELD ":" LOCAL } "=" ] "action" "(" args ")"
          | "delete" "(" NAME ")"
          | "log" "(" STRING ")"
          | "if" cond "then" stmt+ "else" stmt+ "end"     branches hold no `if`
args      = STRING { "," named }                  first positional is the capability
          | named { "," named }                   with `capability: "..."` among them
named     = IDENT ":" STRING                      each name once
NAME      = ident { "/" ident-chars | "::" ident }      ogit/_id, ProcessIssue, LOCAL::X
LOCAL     = "LOCAL::" ident
STRING    = '"' chars-without-quote-newline-or-backslash '"'     ${...} kept verbatim
NUMBER    = ["-"] digits ["." digits]
```

Layout: statements end at a newline; inside `action(...)` newlines are free,
and after a `,` in a result-binding list too. Whitespace and tabs are
insignificant. A line that starts at column 1 must be a clause keyword
(implementation choice: it is how the three examples are laid out). There is
no comment form.

## What is refused, by name, with line and column and what was expected

Operators other than `==` (`!=`, `<`, `>`, `<=`, `=` in a condition, ...);
`and`/`or`/`not`; numbers in a condition; any function other than `action`,
`log`, `delete`; a call inside an argument or a value; an unknown clause
keyword; a missing, empty, repeated or out-of-order clause; an unknown header
key; a `capability` missing, empty or given twice; a duplicate argument; a
non-string argument value; a trailing comma; a binding target that is not
`LOCAL::`; a namespace other than `LOCAL::`; an unterminated string; a
backslash in a string; an `if` without `end`, without `else`, with an empty
branch, or nested in a branch; stray `then`/`else`/`end`; any character that
starts no token (`#`, `.`, `'`, `{`, ...); a second item in one text.
Several of these may be valid HIRO that the docs do not show; they are
refused because "not shown" is not "allowed".

## What stays unknown (and how the lift carries it)

| unknown | in the lift |
|---|---|
| what a bare name in `when` or `if` means (presence or truthiness) | `KiCondition::BareName`, note `UnresolvedBareName`; never an equality, no `kausal` |
| how several `when` lines combine | no `kausal`; note `WhenConjunctionUndocumented` |
| what `on ogit/_id` means for classes | `OnTarget::AnyVertex`; `object_class` empty; note `OnAnyVertex` |
| what other `on` forms name | `OnTarget::Unresolved`; `object_class` empty; note `OnNotAClass` |
| what `params:` means in the HTTP example | an ordinary named argument |
| whether a call argument is mandatory | `CallParam.mandatory = false`, claiming nothing; note `ParamMandatoryUnknown` |
| the meaning of an `if` around a call | the call is lifted; note `ConditionalCall` |
| what a missing or unknown value does in a condition | nothing is evaluated here |
| the result-binding rule | the fields before the call become `results`, the locals are on `KiCall` |
| ordering, priority, several items applicable at once | not modelled |

A def with `kausal: None` from this crate does NOT mean "fires
unconditionally": the item's `when` is on `LiftedItem::when`, and the note
`PreconditionNotStated` says so.

## Mapping to `ActionDef`

`identity` = `ki/{item id}::action_def::{n}::{capability}` (item id: the header
`name:`, else the caller's id; `n`: 1-based position among the item's `action`
calls in text order). `predicate` = capability. `object_class` = the class of
`on ogit/_type == "X"`, else empty. `params` = named arguments (values kept on
`KiCall`, not on the def). `results` = result fields. `reads` = non-local names
tested by `when` and `if`; `writes` = non-local names assigned or deleted;
`calls` = `[capability]`; `body_source` = the item text. `reads` and `writes`
are the item's, repeated on every def. `LOCAL::` names are item-local, not
stored fields, and are kept on `LiftedItem` instead. `kausal` is a
`StateGuard` only for a `when` of exactly one equality.

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
