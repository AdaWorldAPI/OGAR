//! The typed syntax tree of a Knowledge Item (documented subset only).
//!
//! Every node carries a [`Span`]: the line and column of where it starts and
//! where it ends (end exclusive). Lines and columns are 1-based; a column
//! counts characters, a tab is one column.

use std::fmt::Write as _;

/// A position in the source text: 1-based line, 1-based column (characters).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Pos {
    /// 1-based line.
    pub line: usize,
    /// 1-based column, counted in characters.
    pub col: usize,
}

/// A source range: `start` inclusive, `end` exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    /// First character of the node.
    pub start: Pos,
    /// One past the last character of the node.
    pub end: Pos,
}

impl Span {
    /// The span from the start of `a` to the end of `b`.
    #[must_use]
    pub fn join(a: Span, b: Span) -> Span {
        Span {
            start: a.start,
            end: b.end,
        }
    }
}

/// A string literal: the characters between the quotes, verbatim (a `${...}`
/// interpolation is kept as written; no escape sequences exist).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrLit {
    /// The text between the quotes.
    pub value: String,
    /// Span including the quotes.
    pub span: Span,
}

/// A number literal, kept as written (`0`, `-3`, `2.5`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Number {
    /// The literal as written.
    pub text: String,
    /// Span of the literal.
    pub span: Span,
}

/// A name: an attribute (`ogit/Automation/issueType`), a variable
/// (`ProcessIssue`) or a local (`LOCAL::STATUS`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The name as written.
    pub text: String,
    /// Span of the name.
    pub span: Span,
}

/// What a name looks like. SYNTACTIC ONLY: the docs do not define the three
/// kinds; this is the shape the three public examples show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NameKind {
    /// Starts with `LOCAL::`.
    Local,
    /// Contains a `/` (`ogit/_id`, `ogit/Automation/issueType`).
    Attribute,
    /// Anything else (`ProcessIssue`).
    Variable,
}

impl Name {
    /// The syntactic kind of the name.
    #[must_use]
    pub fn kind(&self) -> NameKind {
        kind_of(&self.text)
    }
}

/// [`Name::kind`] for a bare string.
#[must_use]
pub fn kind_of(text: &str) -> NameKind {
    if text.starts_with("LOCAL::") {
        NameKind::Local
    } else if text.contains('/') {
        NameKind::Attribute
    } else {
        NameKind::Variable
    }
}

/// A condition in `on`, `when` or an `if`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cond {
    /// `name == "literal"`.
    Eq {
        /// Left side.
        left: Name,
        /// Right side, a string literal.
        right: StrLit,
        /// Whole condition.
        span: Span,
    },
    /// A bare name alone. Its meaning is NOT documented (presence test or
    /// truthiness): the tree records it as written and decides nothing.
    Bare {
        /// The name.
        name: Name,
        /// Same as the name's span.
        span: Span,
    },
}

impl Cond {
    /// The span of the condition.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Cond::Eq { span, .. } | Cond::Bare { span, .. } => *span,
        }
    }
}

/// The right side of an assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A number.
    Number(Number),
    /// A string.
    Str(StrLit),
}

/// An identifier used as an argument or result-field name (plain: no `/`, no
/// `::`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    /// The identifier.
    pub text: String,
    /// Its span.
    pub span: Span,
}

/// `field: LOCAL::NAME` in front of `= action(...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The result field of the capability.
    pub field: Ident,
    /// The local variable it is bound to.
    pub local: Name,
}

/// `name: "value"` inside `action(...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedArg {
    /// The argument name.
    pub name: Ident,
    /// The value, a string literal.
    pub value: StrLit,
}

/// How the capability was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityForm {
    /// The first positional string: `action("ExecuteCommand", ...)`.
    Positional,
    /// A named argument: `action(capability: "HTTPRequest", ...)`.
    Named,
}

/// One `action(...)` call, with the result bindings in front of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionCall {
    /// Result bindings, in the order written (possibly empty).
    pub bindings: Vec<Binding>,
    /// The capability name.
    pub capability: StrLit,
    /// How the capability was written.
    pub capability_form: CapabilityForm,
    /// The other named arguments, in the order written. The `capability:`
    /// argument is not in this list.
    pub args: Vec<NamedArg>,
    /// From the first binding (or `action`) to the closing parenthesis.
    pub span: Span,
}

/// `if cond then ... else ... end`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfStmt {
    /// The condition.
    pub cond: Cond,
    /// Statements of the `then` branch.
    pub then_branch: Vec<Stmt>,
    /// Statements of the `else` branch.
    pub else_branch: Vec<Stmt>,
    /// From `if` to `end`.
    pub span: Span,
}

/// A statement of the `do` clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    /// `Name = <number or string>`.
    Assign {
        /// Target.
        target: Name,
        /// Value.
        value: Value,
        /// Whole statement.
        span: Span,
    },
    /// `action(...)`, with optional result bindings.
    Action(ActionCall),
    /// `delete(Name)`.
    Delete {
        /// What is deleted.
        target: Name,
        /// Whole statement.
        span: Span,
    },
    /// `log("...")`.
    Log {
        /// The message, `${...}` kept verbatim.
        message: StrLit,
        /// Whole statement.
        span: Span,
    },
    /// `if ... then ... else ... end`.
    If(IfStmt),
}

impl Stmt {
    /// The span of the statement.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Stmt::Assign { span, .. } | Stmt::Delete { span, .. } | Stmt::Log { span, .. } => *span,
            Stmt::Action(a) => a.span,
            Stmt::If(i) => i.span,
        }
    }
}

/// The optional `ki` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// `name: "..."`.
    pub name: Option<StrLit>,
    /// `description: "..."`.
    pub description: Option<StrLit>,
    /// From `ki` to the last header line.
    pub span: Span,
}

/// The `on` clause: exactly one condition (more are not documented).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnClause {
    /// The condition.
    pub cond: Cond,
    /// From `on` to the end of the condition.
    pub span: Span,
}

/// The `when` clause: one condition per line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhenClause {
    /// The conditions, in order. How they combine is not documented.
    pub conds: Vec<Cond>,
    /// From `when` to the end of the last condition.
    pub span: Span,
}

/// The `do` clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoClause {
    /// The statements, in order.
    pub stmts: Vec<Stmt>,
    /// From `do` to the end of the last statement.
    pub span: Span,
}

/// One Knowledge Item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeItem {
    /// The `ki` header, if written.
    pub header: Option<Header>,
    /// The `on` clause.
    pub on: OnClause,
    /// The `when` clause.
    pub when: WhenClause,
    /// The `do` clause.
    pub do_: DoClause,
    /// The whole item.
    pub span: Span,
    /// The item's text, as parsed.
    pub source: String,
}

fn p(pos: Pos) -> String {
    format!("{}:{}", pos.line, pos.col)
}

fn cond_outline(c: &Cond) -> String {
    match c {
        Cond::Eq { left, right, span } => {
            format!("(eq@{} {} {:?})", p(span.start), left.text, right.value)
        }
        Cond::Bare { name, span } => format!("(bare@{} {})", p(span.start), name.text),
    }
}

fn stmt_outline(s: &Stmt, out: &mut String, depth: usize) {
    let pad = "  ".repeat(depth);
    match s {
        Stmt::Assign {
            target,
            value,
            span,
        } => {
            let v = match value {
                Value::Number(n) => format!("num {}", n.text),
                Value::Str(t) => format!("str {:?}", t.value),
            };
            let _ = writeln!(out, "{pad}(assign@{} {} {v})", p(span.start), target.text);
        }
        Stmt::Delete { target, span } => {
            let _ = writeln!(out, "{pad}(delete@{} {})", p(span.start), target.text);
        }
        Stmt::Log { message, span } => {
            let _ = writeln!(out, "{pad}(log@{} {:?})", p(span.start), message.value);
        }
        Stmt::Action(a) => {
            let form = match a.capability_form {
                CapabilityForm::Positional => "positional",
                CapabilityForm::Named => "named",
            };
            let _ = write!(
                out,
                "{pad}(action@{} {:?} {form}",
                p(a.span.start),
                a.capability.value
            );
            for b in &a.bindings {
                let _ = write!(out, " [{} -> {}]", b.field.text, b.local.text);
            }
            for g in &a.args {
                let _ = write!(out, " {}={:?}", g.name.text, g.value.value);
            }
            let _ = writeln!(out, ")");
        }
        Stmt::If(i) => {
            let _ = writeln!(
                out,
                "{pad}(if@{} {}",
                p(i.span.start),
                cond_outline(&i.cond)
            );
            let _ = writeln!(out, "{pad}  then");
            for t in &i.then_branch {
                stmt_outline(t, out, depth + 2);
            }
            let _ = writeln!(out, "{pad}  else");
            for t in &i.else_branch {
                stmt_outline(t, out, depth + 2);
            }
            let _ = writeln!(out, "{pad})");
        }
    }
}

/// A compact, line-per-node rendering of the tree with the start position of
/// each clause and statement. Used to pin the tree in golden tests.
#[must_use]
pub fn outline(item: &KnowledgeItem) -> String {
    let mut out = String::new();
    if let Some(h) = &item.header {
        let _ = writeln!(out, "ki@{}", p(h.span.start));
        if let Some(n) = &h.name {
            let _ = writeln!(out, "  name@{} {:?}", p(n.span.start), n.value);
        }
        if let Some(d) = &h.description {
            let _ = writeln!(out, "  description@{} {:?}", p(d.span.start), d.value);
        }
    }
    let _ = writeln!(out, "on@{}", p(item.on.span.start));
    let _ = writeln!(out, "  {}", cond_outline(&item.on.cond));
    let _ = writeln!(out, "when@{}", p(item.when.span.start));
    for c in &item.when.conds {
        let _ = writeln!(out, "  {}", cond_outline(c));
    }
    let _ = writeln!(out, "do@{}", p(item.do_.span.start));
    for s in &item.do_.stmts {
        stmt_outline(s, &mut out, 1);
    }
    out
}
