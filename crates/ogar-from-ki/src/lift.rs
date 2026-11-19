//! The lift: a parsed Knowledge Item to `ogar_vocab::ActionDef`s, beside the
//! item's own facts and a list of typed notes for everything the lift could
//! not state.
//!
//! What is stated and what is left open is the whole point of this module:
//! the guide gives no formal grammar and says little about what a form means
//! at run time, so a form whose meaning is not documented is carried as a
//! named, unresolved fact, never turned into something it might mean.

use ogar_vocab::{ActionDef, CallParam, KausalSpec};

use crate::ast::{
    ActionCall, ArgValue, CapabilityForm, Comparison, Cond, Expr, Key, KnowledgeItem, NameKind,
    OnItem, Pos, Stmt, Tier, VarRef, expr_text, kind_of,
};

/// The attribute that names an `on` clause's vertex in the documented
/// `on ogit/_id` form.
pub const ID_ATTRIBUTE: &str = "ogit/_id";
/// The attribute that names the class in `on ogit/_type == "..."`.
pub const TYPE_ATTRIBUTE: &str = "ogit/_type";

/// Caller input to the lift.
#[derive(Debug, Clone, Default)]
pub struct LiftOptions {
    /// The identity of an item that has no header name. Used ONLY when the
    /// item has no `name:`; with a name, the name wins.
    pub item_id: Option<String>,
}

/// Why an item could not be lifted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiftError {
    /// The item has no `name:` and the caller gave no id: a stable identity
    /// cannot be derived, and none is invented.
    NoIdentity,
}

impl std::fmt::Display for LiftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiftError::NoIdentity => {
                write!(f, "the item has no `name:` header and no item id was given")
            }
        }
    }
}

impl std::error::Error for LiftError {}

/// What an `on` clause names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnTarget {
    /// `on ogit/_id`: any vertex. No class is named, and none is invented.
    AnyVertex,
    /// `on ogit/_type == "X"`: class `X`.
    Class(String),
    /// Any other attribute form. What class it names, if any, is not
    /// documented.
    Unresolved {
        /// The attribute or name written.
        attribute: String,
        /// The compared string, when the form was a comparison.
        value: Option<String>,
    },
}

/// A condition of a `when` clause or an `if`, as the lift carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KiCondition {
    /// `name == "value"`.
    Equals {
        /// The attribute or variable.
        name: String,
        /// The string it is compared with.
        value: String,
        /// Where it was written.
        pos: Pos,
    },
    /// A bare name alone. UNRESOLVED: whether it tests presence or a truthy
    /// value is not documented. It is never read as either, and never as an
    /// equality.
    BareName {
        /// The name.
        name: String,
        /// Where it was written.
        pos: Pos,
    },
    /// Any other condition: another operator, `not`, `and`, `or`,
    /// parentheses, a key, `list`, `?`, an alias, a graph limiter, a number or
    /// a variable on the right. The tree has all of it
    /// ([`crate::KnowledgeItem`]); the lift states none of it and carries only
    /// the variables the condition names.
    Other {
        /// The variables the condition names, in order of first appearance
        /// (aliases are not variables and are not listed).
        names: Vec<String>,
        /// Where it was written.
        pos: Pos,
    },
}

impl KiCondition {
    /// The name the condition tests: for [`KiCondition::Other`], the first
    /// variable it names (empty if it names none).
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            KiCondition::Equals { name, .. } | KiCondition::BareName { name, .. } => name,
            KiCondition::Other { names, .. } => names.first().map_or("", String::as_str),
        }
    }

    /// Every variable the condition names.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        match self {
            KiCondition::Equals { name, .. } | KiCondition::BareName { name, .. } => {
                vec![name.as_str()]
            }
            KiCondition::Other { names, .. } => names.iter().map(String::as_str).collect(),
        }
    }
}

/// Which branch of an `if` a call sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    /// After `then`.
    Then,
    /// After `else`.
    Else,
}

/// The `if` a call is inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallCondition {
    /// Where the `if` starts.
    pub if_pos: Pos,
    /// The branch.
    pub branch: Branch,
}

/// What kind of value a [`KiArg`] carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KiArgKind {
    /// A string literal (the text between the quotes).
    Str,
    /// A number literal, as written.
    Number,
    /// A variable, as written.
    Name,
    /// A list, an arithmetic expression or a regex, rendered in the tree's
    /// outline form.
    Other,
    /// A `begin ... end` block: the value is the word `begin`.
    Block,
}

/// A named argument as written at a call site: a BINDING (a value), not a
/// declaration. The matching declaration is the [`CallParam`] on the def.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KiArg {
    /// The argument name.
    pub name: String,
    /// The value: a string's text, or a number or variable as written.
    pub value: String,
    /// What the value is.
    pub kind: KiArgKind,
}

/// `field: LOCAL::NAME` in front of a call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KiResultBinding {
    /// The result field of the capability.
    pub field: String,
    /// The local it is bound to.
    pub local: String,
}

/// One `action(...)` call of an item, with everything the `ActionDef` has no
/// field for. `Lifted::defs[i]` is the declaration of `LiftedItem::calls[i]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KiCall {
    /// 1-based position among the item's `action` calls, in text order
    /// (calls inside `if` branches count where they are written).
    pub ordinal: usize,
    /// The `ActionDef::identity` of the declaration.
    pub identity: String,
    /// The capability name.
    pub capability: String,
    /// How the capability was written.
    pub capability_form: CapabilityForm,
    /// Where the call starts.
    pub pos: Pos,
    /// The named arguments with their literal values.
    pub args: Vec<KiArg>,
    /// The result bindings.
    pub results: Vec<KiResultBinding>,
    /// The `if` branch the call sits in, if any.
    pub condition: Option<CallCondition>,
}

/// What an other call is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtherCallKind {
    /// `execute(...)`. The guide calls it a simpler front end to `action`
    /// capabilities, chosen by `host` and `scriptmode`. Reported as its own
    /// kind; not lifted to a def.
    Execute,
    /// Any other function (`javascript`, `query`, `dateprint`, ...). Only the
    /// name is recorded.
    Generic,
}

/// A call that is not `action(...)`: recorded, not lifted, no meaning given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KiOtherCall {
    /// The function name.
    pub function: String,
    /// `execute` or another function.
    pub kind: OtherCallKind,
    /// Where the call starts.
    pub pos: Pos,
    /// The result tags in front of the call.
    pub results: Vec<KiResultBinding>,
}

/// The facts of one item, independent of whether it calls anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiftedItem {
    /// The item's identity: the header name, else the caller's id.
    pub id: String,
    /// The header `name:`.
    pub name: Option<String>,
    /// The header `description:`.
    pub description: Option<String>,
    /// The header `version:`.
    pub version: Option<String>,
    /// The header `tier:`, typed.
    pub tier: Option<Tier>,
    /// What the `on` clause names.
    pub on: OnTarget,
    /// Where the `on` clause starts.
    pub on_pos: Pos,
    /// The `when` conditions, in order.
    pub when: Vec<KiCondition>,
    /// The conditions of the `if`s in `do`, in text order.
    pub if_conditions: Vec<KiCondition>,
    /// Names tested by `when` and `if` conditions, not item-local, in order of
    /// first appearance. These are the `reads` of every def of the item.
    pub reads: Vec<String>,
    /// Names assigned or deleted in `do`, not item-local. These are the
    /// `writes` of every def of the item.
    pub writes: Vec<String>,
    /// `LOCAL::` names tested by an `if`.
    pub locals_read: Vec<String>,
    /// `LOCAL::` names assigned or bound in `do`.
    pub locals_written: Vec<String>,
    /// The `action` calls, in text order.
    pub calls: Vec<KiCall>,
    /// Every other call (`execute`, `javascript`, `query`, ...), in text
    /// order. Not lifted.
    pub other_calls: Vec<KiOtherCall>,
    /// The item's text.
    pub source: String,
}

/// Which clause a condition was written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionSite {
    /// A `when` line.
    When,
    /// An `if` in `do`.
    If,
}

/// Something the lift could not state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteKind {
    /// `on ogit/_id`: the item applies on any vertex; `object_class` is empty
    /// because no class is named.
    OnAnyVertex,
    /// An `on` form that is not `ogit/_id` and not `ogit/_type == "X"`; what
    /// it names is not documented, so `object_class` is empty.
    OnNotAClass {
        /// The attribute or name written.
        attribute: String,
    },
    /// A bare name as a condition. Meaning not documented.
    UnresolvedBareName {
        /// The name.
        name: String,
        /// Where it was written.
        site: ConditionSite,
    },
    /// More than one `when` condition. The matching page says lines are
    /// joined by an implicit `and`; the lift states a `kausal` only for one
    /// equality, so it states none here. (The variant keeps its original
    /// name.)
    WhenConjunctionUndocumented {
        /// The number of conditions.
        count: usize,
    },
    /// A `when` condition over a `LOCAL::` name: a local exists only inside
    /// `do`, so the lift does not state it as a precondition.
    LocalInWhen {
        /// The name.
        name: String,
    },
    /// A def has no `kausal` although the item has a `when`: the guard is in
    /// `LiftedItem::when`, not on the def. A def with `kausal: None` does NOT
    /// mean "fires unconditionally" here.
    PreconditionNotStated,
    /// The call sits in an `if` branch: it runs only when the `if` condition
    /// holds, which the def does not state.
    ConditionalCall {
        /// The capability.
        capability: String,
        /// The branch.
        branch: Branch,
        /// Where the `if` starts.
        if_pos: Pos,
    },
    /// The call passes named arguments; whether each is mandatory is a
    /// property of the capability, not of the call. The `CallParam`s are all
    /// `mandatory: false`, which claims nothing.
    ParamMandatoryUnknown {
        /// The capability.
        capability: String,
    },
    /// The item has no `action(...)` call: it lifts to no def.
    NoActionCall,
    /// A condition more complex than a name or `name == "string"`. The lift
    /// states none of it; the variables it names are in `reads`.
    ConditionBeyondLift {
        /// Where it was written.
        site: ConditionSite,
    },
    /// An `execute(...)` call. The guide calls it a front end to `action`
    /// capabilities chosen by `host` and `scriptmode`; the lift does not
    /// choose one and states no def for it.
    ExecuteNotLifted,
}

/// A note, with where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KiNote {
    /// What is open.
    pub kind: NoteKind,
    /// The position in the item, when there is one.
    pub pos: Option<Pos>,
    /// The 1-based ordinal of the call it concerns, when it concerns one.
    pub call: Option<usize>,
}

/// The result of the lift.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lifted {
    /// The item's facts.
    pub item: LiftedItem,
    /// One `ActionDef` per `action(...)` call, in text order.
    pub defs: Vec<ActionDef>,
    /// Everything the lift left open.
    pub notes: Vec<KiNote>,
}

#[derive(Default)]
struct Acc {
    reads: Vec<String>,
    writes: Vec<String>,
    locals_read: Vec<String>,
    locals_written: Vec<String>,
    if_conditions: Vec<KiCondition>,
    calls: Vec<KiCall>,
    other_calls: Vec<KiOtherCall>,
    notes: Vec<KiNote>,
}

fn push_unique(v: &mut Vec<String>, s: &str) {
    if !v.iter().any(|x| x == s) {
        v.push(s.to_string());
    }
}

fn push_expr_names(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Var(v) => push_var_names(v, out),
        Expr::List { items, .. } => {
            for i in items {
                push_expr_names(i, out);
            }
        }
        Expr::Binary { left, right, .. } => {
            push_expr_names(left, out);
            push_expr_names(right, out);
        }
        Expr::Str(_) | Expr::Number(_) | Expr::Regex(_) => {}
    }
}

fn push_var_names(v: &VarRef, out: &mut Vec<String>) {
    push_unique(out, &v.name.text);
    if let Some(Key::Var(k)) = &v.key {
        push_unique(out, &k.text);
    }
}

fn cond_names(c: &Cond) -> Vec<String> {
    let mut out = Vec::new();
    for l in c.leaves() {
        push_var_names(&l.subject, &mut out);
        if let Some(t) = &l.test {
            push_expr_names(&t.right, &mut out);
        }
        if let Some(d) = &l.default {
            push_expr_names(d, &mut out);
        }
    }
    out
}

fn condition_of(c: &Cond) -> KiCondition {
    if let Cond::Leaf(l) = c
        && l.is_plain()
    {
        let pos = l.span.start;
        let name = l.subject.name.text.clone();
        match &l.test {
            None => return KiCondition::BareName { name, pos },
            Some(Comparison {
                right: Expr::Str(s),
                ..
            }) => {
                return KiCondition::Equals {
                    name,
                    value: s.value.clone(),
                    pos,
                };
            }
            Some(_) => {}
        }
    }
    KiCondition::Other {
        names: cond_names(c),
        pos: c.span().start,
    }
}

fn on_target(item: &KnowledgeItem) -> OnTarget {
    let first = item.on.items.first();
    let conds: Vec<&Cond> = item.on.conds().collect();
    for c in &conds {
        if let Cond::Leaf(l) = c
            && l.is_plain()
            && l.subject.name.text == TYPE_ATTRIBUTE
            && let Some(Comparison {
                right: Expr::Str(s),
                ..
            }) = &l.test
            && !s.value.is_empty()
        {
            return OnTarget::Class(s.value.clone());
        }
    }
    let lone = item.on.items.len() == 1;
    match first {
        Some(OnItem::Cond(Cond::Leaf(l))) if l.is_plain() => match &l.test {
            None if lone && l.subject.name.text == ID_ATTRIBUTE => OnTarget::AnyVertex,
            None => OnTarget::Unresolved {
                attribute: l.subject.name.text.clone(),
                value: None,
            },
            Some(Comparison {
                right: Expr::Str(s),
                ..
            }) => OnTarget::Unresolved {
                attribute: l.subject.name.text.clone(),
                value: Some(s.value.clone()),
            },
            Some(_) => OnTarget::Unresolved {
                attribute: l.subject.name.text.clone(),
                value: None,
            },
        },
        Some(OnItem::Cond(c)) => OnTarget::Unresolved {
            attribute: cond_names(c).into_iter().next().unwrap_or_default(),
            value: None,
        },
        Some(OnItem::Block(b)) => OnTarget::Unresolved {
            attribute: format!("{} {}", b.kind.word(), b.vertex_type.text),
            value: None,
        },
        None => OnTarget::Unresolved {
            attribute: String::new(),
            value: None,
        },
    }
}

fn read_name(acc: &mut Acc, name: &str) {
    if kind_of(name) == NameKind::Local {
        push_unique(&mut acc.locals_read, name);
    } else {
        push_unique(&mut acc.reads, name);
    }
}

fn read_condition(acc: &mut Acc, c: &KiCondition) {
    for n in c.names() {
        read_name(acc, n);
    }
}

fn note_beyond(acc: &mut Acc, c: &KiCondition, site: ConditionSite) {
    if let KiCondition::Other { pos, .. } = c {
        acc.notes.push(KiNote {
            kind: NoteKind::ConditionBeyondLift { site },
            pos: Some(*pos),
            call: None,
        });
    }
}

fn write_name(acc: &mut Acc, name: &str) {
    if kind_of(name) == NameKind::Local {
        push_unique(&mut acc.locals_written, name);
    } else {
        push_unique(&mut acc.writes, name);
    }
}

fn note_bare(acc: &mut Acc, c: &KiCondition, site: ConditionSite) {
    if let KiCondition::BareName { name, pos } = c {
        acc.notes.push(KiNote {
            kind: NoteKind::UnresolvedBareName {
                name: name.clone(),
                site,
            },
            pos: Some(*pos),
            call: None,
        });
    }
}

fn arg_of(name: &str, v: &ArgValue) -> KiArg {
    let (value, kind) = match v {
        ArgValue::Expr(Expr::Str(s)) => (s.value.clone(), KiArgKind::Str),
        ArgValue::Expr(Expr::Number(n)) => (n.text.clone(), KiArgKind::Number),
        ArgValue::Expr(Expr::Var(var)) if var.key.is_none() && var.graph.is_none() => {
            (var.name.text.clone(), KiArgKind::Name)
        }
        ArgValue::Expr(e) => (expr_text(e), KiArgKind::Other),
        ArgValue::Block(_) => ("begin".to_string(), KiArgKind::Block),
    };
    KiArg {
        name: name.to_string(),
        value,
        kind,
    }
}

fn walk_value(v: &ArgValue, ctx: Option<CallCondition>, id: &str, acc: &mut Acc) {
    if let ArgValue::Block(b) = v {
        walk(&b.stmts, ctx, id, acc);
    }
}

fn walk(stmts: &[Stmt], ctx: Option<CallCondition>, id: &str, acc: &mut Acc) {
    for s in stmts {
        match s {
            Stmt::Assign { target, .. }
            | Stmt::Set { target, .. }
            | Stmt::Add { target, .. }
            | Stmt::Delete { target, .. } => write_name(acc, &target.name.text),
            Stmt::Log { .. } => {}
            Stmt::Action(a) => call(a, ctx, id, acc),
            Stmt::Call(c) => {
                for b in &c.bindings {
                    write_name(acc, &b.target.text);
                }
                let kind = if c.function.text == "execute" {
                    acc.notes.push(KiNote {
                        kind: NoteKind::ExecuteNotLifted,
                        pos: Some(c.span.start),
                        call: None,
                    });
                    OtherCallKind::Execute
                } else {
                    OtherCallKind::Generic
                };
                acc.other_calls.push(KiOtherCall {
                    function: c.function.text.clone(),
                    kind,
                    pos: c.span.start,
                    results: c
                        .bindings
                        .iter()
                        .map(|b| KiResultBinding {
                            field: b.field.text.clone(),
                            local: b.target.text.clone(),
                        })
                        .collect(),
                });
                for a in &c.args {
                    walk_value(&a.value, ctx, id, acc);
                }
            }
            Stmt::If(i) => {
                let c = condition_of(&i.cond);
                read_condition(acc, &c);
                note_bare(acc, &c, ConditionSite::If);
                note_beyond(acc, &c, ConditionSite::If);
                acc.if_conditions.push(c);
                let at = i.span.start;
                walk(
                    &i.then_branch,
                    Some(CallCondition {
                        if_pos: at,
                        branch: Branch::Then,
                    }),
                    id,
                    acc,
                );
                if let Some(e) = &i.else_branch {
                    walk(
                        e,
                        Some(CallCondition {
                            if_pos: at,
                            branch: Branch::Else,
                        }),
                        id,
                        acc,
                    );
                }
            }
        }
    }
}

fn call(a: &ActionCall, ctx: Option<CallCondition>, id: &str, acc: &mut Acc) {
    let ordinal = acc.calls.len() + 1;
    let capability = a.capability.value.clone();
    for b in &a.bindings {
        write_name(acc, &b.target.text);
    }
    if let Some(c) = ctx {
        acc.notes.push(KiNote {
            kind: NoteKind::ConditionalCall {
                capability: capability.clone(),
                branch: c.branch,
                if_pos: c.if_pos,
            },
            pos: Some(a.span.start),
            call: Some(ordinal),
        });
    }
    if !a.args.is_empty() {
        acc.notes.push(KiNote {
            kind: NoteKind::ParamMandatoryUnknown {
                capability: capability.clone(),
            },
            pos: Some(a.span.start),
            call: Some(ordinal),
        });
    }
    acc.calls.push(KiCall {
        ordinal,
        identity: format!("ki/{id}::action_def::{ordinal}::{capability}"),
        capability,
        capability_form: a.capability_form,
        pos: a.span.start,
        args: a
            .args
            .iter()
            .map(|g| arg_of(&g.name.text, &g.value))
            .collect(),
        results: a
            .bindings
            .iter()
            .map(|b| KiResultBinding {
                field: b.field.text.clone(),
                local: b.target.text.clone(),
            })
            .collect(),
        condition: ctx,
    });
    for g in &a.args {
        walk_value(&g.value, ctx, id, acc);
    }
}

/// Lift one parsed item.
///
/// Identity of the item: its header `name:` verbatim; with no name, the
/// caller's [`LiftOptions::item_id`]; with neither, [`LiftError::NoIdentity`].
/// Identity of a def: `ki/{item id}::action_def::{ordinal}::{capability}`,
/// where `ordinal` is the 1-based position of the call among the item's
/// `action(...)` calls in text order. Renaming the item or reordering its
/// calls changes the identities; editing arguments or conditions does not.
///
/// Each def carries:
/// - `predicate`: the capability; `calls`: `[capability]`;
/// - `object_class`: the class of `on ogit/_type == "X"` (`X`), else empty (see
///   [`OnTarget`] and the notes);
/// - `params`: the call's named arguments as [`CallParam`]s (`mandatory:
///   false`: unknown, not claimed); the literal values are on the [`KiCall`];
/// - `results`: the result fields bound in front of the call;
/// - `reads`, `writes`: the item's, repeated on each def (not item-local
///   `LOCAL::` names, which are on [`LiftedItem`]);
/// - `kausal`: a `StateGuard` ONLY for an item whose `when` is exactly one
///   equality over a non-local name; otherwise `None`, with notes;
/// - `body_source`: the item's text.
///
/// # Errors
///
/// [`LiftError::NoIdentity`].
pub fn lift(item: &KnowledgeItem, opts: &LiftOptions) -> Result<Lifted, LiftError> {
    let name = item
        .header
        .as_ref()
        .and_then(|h| h.name.as_ref())
        .map(|n| n.value.clone());
    let description = item
        .header
        .as_ref()
        .and_then(|h| h.description.as_ref())
        .map(|n| n.value.clone());
    let version = item
        .header
        .as_ref()
        .and_then(|h| h.version.as_ref())
        .map(|n| n.value.clone());
    let tier = item.header.as_ref().and_then(|h| h.tier);
    let id = name
        .clone()
        .or_else(|| opts.item_id.clone())
        .ok_or(LiftError::NoIdentity)?;

    let mut acc = Acc::default();
    let on = on_target(item);
    let on_pos = item.on.span.start;
    match &on {
        OnTarget::AnyVertex => acc.notes.push(KiNote {
            kind: NoteKind::OnAnyVertex,
            pos: Some(on_pos),
            call: None,
        }),
        OnTarget::Unresolved { attribute, .. } => acc.notes.push(KiNote {
            kind: NoteKind::OnNotAClass {
                attribute: attribute.clone(),
            },
            pos: Some(on_pos),
            call: None,
        }),
        OnTarget::Class(_) => {}
    }

    let when: Vec<KiCondition> = item.when.conds.iter().map(condition_of).collect();
    for c in &when {
        read_condition(&mut acc, c);
        note_bare(&mut acc, c, ConditionSite::When);
        note_beyond(&mut acc, c, ConditionSite::When);
        if let KiCondition::Equals { name, pos, .. } = c
            && kind_of(name) == NameKind::Local
        {
            acc.notes.push(KiNote {
                kind: NoteKind::LocalInWhen { name: name.clone() },
                pos: Some(*pos),
                call: None,
            });
        }
    }
    if when.len() > 1 {
        acc.notes.push(KiNote {
            kind: NoteKind::WhenConjunctionUndocumented { count: when.len() },
            pos: Some(item.when.span.start),
            call: None,
        });
    }

    walk(&item.do_.stmts, None, &id, &mut acc);

    // The one truthful precondition: exactly one equality over a stored name.
    let kausal = match when.as_slice() {
        [KiCondition::Equals { name, value, .. }] if !name.contains("::") => {
            Some(KausalSpec::state_guard(name.clone(), vec![value.clone()]))
        }
        _ => None,
    };
    if acc.calls.is_empty() {
        acc.notes.push(KiNote {
            kind: NoteKind::NoActionCall,
            pos: Some(item.do_.span.start),
            call: None,
        });
    } else if kausal.is_none() {
        acc.notes.push(KiNote {
            kind: NoteKind::PreconditionNotStated,
            pos: Some(item.when.span.start),
            call: None,
        });
    }

    let object_class = match &on {
        OnTarget::Class(c) => c.clone(),
        _ => String::new(),
    };
    let defs: Vec<ActionDef> = acc
        .calls
        .iter()
        .map(|c| {
            let mut d = ActionDef::new(
                c.identity.clone(),
                c.capability.clone(),
                object_class.clone(),
            );
            d.kausal = kausal.clone();
            d.body_source = Some(item.source.clone());
            d.reads = acc.reads.clone();
            d.writes = acc.writes.clone();
            d.calls = vec![c.capability.clone()];
            d.params = c
                .args
                .iter()
                .map(|g| CallParam::owned(g.name.clone(), false))
                .collect();
            d.results = c.results.iter().map(|r| r.field.clone()).collect();
            d
        })
        .collect();

    let lifted_item = LiftedItem {
        id,
        name,
        description,
        version,
        tier,
        on,
        on_pos,
        when,
        if_conditions: acc.if_conditions,
        reads: acc.reads,
        writes: acc.writes,
        locals_read: acc.locals_read,
        locals_written: acc.locals_written,
        calls: acc.calls,
        other_calls: acc.other_calls,
        source: item.source.clone(),
    };
    // Order the notes by position so a reader sees them in text order.
    let mut notes = acc.notes;
    notes.sort_by_key(|n| n.pos);
    Ok(Lifted {
        item: lifted_item,
        defs,
        notes,
    })
}

/// Parse and lift in one call.
///
/// # Errors
///
/// A parse refusal or a lift error, as a string-free enum.
pub fn lift_source(src: &str, opts: &LiftOptions) -> Result<Lifted, SourceError> {
    let item = crate::parse(src).map_err(SourceError::Parse)?;
    lift(&item, opts).map_err(SourceError::Lift)
}

/// Either half of [`lift_source`] failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// The text is outside the documented subset.
    Parse(crate::ParseError),
    /// The item has no identity.
    Lift(LiftError),
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceError::Parse(e) => e.fmt(f),
            SourceError::Lift(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for SourceError {}
