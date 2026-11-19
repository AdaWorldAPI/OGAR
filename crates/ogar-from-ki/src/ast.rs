//! The typed syntax tree of a Knowledge Item, as the HIRO User Guides 7.0
//! document it.
//!
//! Every node carries a [`Span`]: the line and column of where it starts and
//! where it ends (end exclusive). Lines and columns are 1-based; a column
//! counts characters, a tab is one column.
//!
//! The tree records what was written. It does not evaluate anything: no
//! condition is resolved, no scope is looked up, no call is run.

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

/// The quote character a string was written with. The guide uses both: `"`
/// in conditions and messages, `'` around script and query text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quote {
    /// `"..."`.
    Double,
    /// `'...'`.
    Single,
}

/// A string literal: the characters between the quotes, verbatim (a `${...}`
/// interpolation is kept as written; a backslash is kept as written).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrLit {
    /// The text between the quotes.
    pub value: String,
    /// Span including the quotes.
    pub span: Span,
    /// The quote character used.
    pub quote: Quote,
}

/// A number literal, kept as written (`0`, `-3`, `2.5`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Number {
    /// The literal as written.
    pub text: String,
    /// Span of the literal.
    pub span: Span,
}

/// A regex literal `/pattern/flags`, kept as written. Named groups are not
/// interpreted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexLit {
    /// The text between the slashes.
    pub pattern: String,
    /// The flag letters after the closing slash (`i`).
    pub flags: String,
    /// Span including the slashes and flags.
    pub span: Span,
}

/// A name: an attribute (`ogit/Automation/issueType`), a variable
/// (`ProcessIssue`), or a qualified name (`LOCAL::STATUS`, `NODE::Foo`,
/// `Parent::Foo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The name as written, with its qualifier if any.
    pub text: String,
    /// Span of the name.
    pub span: Span,
}

/// What a name looks like. SYNTACTIC ONLY.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NameKind {
    /// Starts with `LOCAL::`.
    Local,
    /// Contains a `/` (`ogit/_id`, `ogit/Automation/issueType`).
    Attribute,
    /// Anything else (`ProcessIssue`).
    Variable,
}

/// The qualifier written in front of `::`. The scope page documents `NODE`,
/// `ISSUE` and `LOCAL`; the same page and the matching page also write
/// `<alias>::Name` for the scope of a `with` block (`Parent::Foo`,
/// `test::A`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Qualifier {
    /// `NODE::`: the MARS node data.
    Node,
    /// `ISSUE::`: the issue data.
    Issue,
    /// `LOCAL::`: the current execution context.
    Local,
    /// `<alias>::`: the scope of a `with` or `count` block of that alias.
    Alias(String),
}

/// How a scope qualifier was spelled. The scope page documents `NODE::` and
/// prints `Node::ogit/status` once; both are recorded, neither is preferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeSpelling {
    /// `NODE::`, `ISSUE::`, `LOCAL::` or an alias.
    Documented,
    /// `Node::`, as in the scope page's "Node::ogit/status = "visited"".
    NodeTitleCase,
}

impl Name {
    /// The syntactic kind of the name.
    #[must_use]
    pub fn kind(&self) -> NameKind {
        kind_of(&self.text)
    }

    /// The qualifier written in front of `::`, if any.
    #[must_use]
    pub fn qualifier(&self) -> Option<Qualifier> {
        qualifier_of(&self.text).map(|(q, _)| q)
    }

    /// How the qualifier was spelled, if there is one.
    #[must_use]
    pub fn scope_spelling(&self) -> Option<ScopeSpelling> {
        qualifier_of(&self.text).map(|(_, s)| s)
    }

    /// The name without its qualifier.
    #[must_use]
    pub fn base(&self) -> &str {
        self.text.split_once("::").map_or(&self.text, |(_, b)| b)
    }
}

fn qualifier_of(text: &str) -> Option<(Qualifier, ScopeSpelling)> {
    let (prefix, _) = text.split_once("::")?;
    Some(match prefix {
        "NODE" => (Qualifier::Node, ScopeSpelling::Documented),
        "Node" => (Qualifier::Node, ScopeSpelling::NodeTitleCase),
        "ISSUE" => (Qualifier::Issue, ScopeSpelling::Documented),
        "LOCAL" => (Qualifier::Local, ScopeSpelling::Documented),
        other => (
            Qualifier::Alias(other.to_string()),
            ScopeSpelling::Documented,
        ),
    })
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

/// An identifier used as an argument name, result tag or alias (plain: no
/// `/`, no `::`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    /// The identifier.
    pub text: String,
    /// Its span.
    pub span: Span,
}

// ── references and expressions ───────────────────────────────────────

/// The graph scope limiter after a name: `Foo@any`, `Foo@self`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphScopeKind {
    /// `@any`: values created anywhere in the graph (matching page).
    Any,
    /// `@self`: only values created on the local node (matching page).
    SelfOnly,
    /// `@in`, written in the guide's full example
    /// (`MonitoringStatus@in == "true"`); the structure page writes `@any` for
    /// the same line, and `in` is documented only as an edge direction. Kept
    /// as its own variant, not read as `@any`.
    InAsWrittenInExample,
}

/// A graph scope limiter with its position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphScope {
    /// Which limiter.
    pub kind: GraphScopeKind,
    /// Span of `@name`.
    pub span: Span,
}

/// A key in square brackets: `VarB["key"]`, `DiskPos[1]`, `VarC[VarA]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// A string key.
    Str(StrLit),
    /// A number key.
    Number(Number),
    /// A key given by another variable.
    Var(Name),
}

/// A variable as written: name, optional key, optional graph scope limiter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarRef {
    /// The name, with its qualifier if any.
    pub name: Name,
    /// `["key"]`, `[1]` or `[Var]`.
    pub key: Option<Key>,
    /// `@any`, `@self`.
    pub graph: Option<GraphScope>,
    /// From the name to the last character of the suffixes.
    pub span: Span,
}

/// An arithmetic operator. The varstrings page shows `+ - / * %`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArithOp {
    /// `+`.
    Add,
    /// `-`.
    Sub,
    /// `*`.
    Mul,
    /// `/`.
    Div,
    /// `%`.
    Mod,
}

impl ArithOp {
    /// The operator as written.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            ArithOp::Add => "+",
            ArithOp::Sub => "-",
            ArithOp::Mul => "*",
            ArithOp::Div => "/",
            ArithOp::Mod => "%",
        }
    }
}

/// A value expression: the right side of a comparison or an assignment, an
/// argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// A string.
    Str(StrLit),
    /// A number.
    Number(Number),
    /// A variable.
    Var(VarRef),
    /// `["A", "B"]`.
    List {
        /// The elements.
        items: Vec<Expr>,
        /// Span from `[` to `]`.
        span: Span,
    },
    /// A regex literal (only after `=~` or `!~`).
    Regex(RegexLit),
    /// `left op right`: one operator, no chains.
    Binary {
        /// The operator.
        op: ArithOp,
        /// Left operand.
        left: Box<Expr>,
        /// Right operand.
        right: Box<Expr>,
        /// Whole expression.
        span: Span,
    },
}

impl Expr {
    /// The span of the expression.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Expr::Str(s) => s.span,
            Expr::Number(n) => n.span,
            Expr::Var(v) => v.span,
            Expr::Regex(r) => r.span,
            Expr::List { span, .. } | Expr::Binary { span, .. } => *span,
        }
    }
}

// ── conditions ───────────────────────────────────────────────────────

/// Which spelling of the exact inequality was written. The guide writes
/// `!===` in the operator table and `Bar !== Foo` in an example.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExactNeSpelling {
    /// `!===`, the matching page's operator table.
    Table,
    /// `!==`, the matching page's example and its prose ("use === / !==").
    Example,
}

/// A comparison operator of the matching page's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CmpOp {
    /// `==`.
    Eq,
    /// `!=`.
    Ne,
    /// `===`.
    ExactEq,
    /// `!===` or `!==`, as written.
    ExactNe(ExactNeSpelling),
    /// `>=`.
    Ge,
    /// `<=`.
    Le,
    /// `>`.
    Gt,
    /// `<`.
    Lt,
    /// `=~`.
    Match,
    /// `!~`.
    NotMatch,
}

impl CmpOp {
    /// The operator as written.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            CmpOp::Eq => "==",
            CmpOp::Ne => "!=",
            CmpOp::ExactEq => "===",
            CmpOp::ExactNe(ExactNeSpelling::Table) => "!===",
            CmpOp::ExactNe(ExactNeSpelling::Example) => "!==",
            CmpOp::Ge => ">=",
            CmpOp::Le => "<=",
            CmpOp::Gt => ">",
            CmpOp::Lt => "<",
            CmpOp::Match => "=~",
            CmpOp::NotMatch => "!~",
        }
    }
}

/// A comparison: operator and right side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    /// The operator.
    pub op: CmpOp,
    /// Where the operator was written.
    pub op_span: Span,
    /// The right side.
    pub right: Expr,
}

/// One condition leaf: `Foo`, `list Foo`, `Foo == "x"`, `Foo as Alias`,
/// `? Foo < 5 := 0`, `?list Foo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leaf {
    /// Written with a leading `?` (an optional match).
    pub optional: bool,
    /// Written with `list` (matches the whole list once).
    pub list: bool,
    /// The variable on the left.
    pub subject: VarRef,
    /// The comparison, if there is one. None means: the variable exists.
    pub test: Option<Comparison>,
    /// `as Alias`.
    pub alias: Option<Ident>,
    /// `:= default` of an optional match.
    pub default: Option<Expr>,
    /// Whole leaf.
    pub span: Span,
}

impl Leaf {
    /// True for the plain shapes `Name` and `Name == "string"`: no key, no
    /// graph limiter, no `list`, no `?`, no alias, no default.
    #[must_use]
    pub fn is_plain(&self) -> bool {
        !self.optional
            && !self.list
            && self.subject.key.is_none()
            && self.subject.graph.is_none()
            && self.alias.is_none()
            && self.default.is_none()
            && match &self.test {
                None => true,
                Some(c) => c.op == CmpOp::Eq && matches!(c.right, Expr::Str(_)),
            }
    }
}

/// A condition tree. `not` binds the next expression only and binds tighter
/// than `or` (matching page); lines are joined by an implicit `and`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cond {
    /// A leaf.
    Leaf(Box<Leaf>),
    /// `not <cond>`.
    Not {
        /// What is negated.
        inner: Box<Cond>,
        /// From `not` to the end of the operand.
        span: Span,
    },
    /// `a and b [and c ...]` on one line.
    And {
        /// The operands, in order.
        parts: Vec<Cond>,
        /// Whole expression.
        span: Span,
    },
    /// `a or b [or c ...]`.
    Or {
        /// The operands, in order.
        parts: Vec<Cond>,
        /// Whole expression.
        span: Span,
    },
    /// `( cond )`, kept so the written shape survives.
    Group {
        /// Inside the parentheses.
        inner: Box<Cond>,
        /// From `(` to `)`.
        span: Span,
    },
}

impl Cond {
    /// The span of the condition.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Cond::Leaf(l) => l.span,
            Cond::Not { span, .. }
            | Cond::And { span, .. }
            | Cond::Or { span, .. }
            | Cond::Group { span, .. } => *span,
        }
    }

    /// The leaves of the tree, in text order.
    #[must_use]
    pub fn leaves(&self) -> Vec<&Leaf> {
        let mut out = Vec::new();
        self.collect_leaves(&mut out);
        out
    }

    fn collect_leaves<'a>(&'a self, out: &mut Vec<&'a Leaf>) {
        match self {
            Cond::Leaf(l) => out.push(l),
            Cond::Not { inner, .. } | Cond::Group { inner, .. } => inner.collect_leaves(out),
            Cond::And { parts, .. } | Cond::Or { parts, .. } => {
                for p in parts {
                    p.collect_leaves(out);
                }
            }
        }
    }
}

// ── related node matching ────────────────────────────────────────────

/// The edge direction of a `with` or `count` block.
///
/// The structure page lists `in`, `out`, `connected`. The matching and scope
/// pages also write `@parent` in examples. `child` appears with it in the
/// scope page's `with test@child:owns`. Both are kept as their own variants:
/// used in the guide, not in its documented list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// `in`: incoming edge.
    In,
    /// `out`: outgoing edge.
    Out,
    /// `connected`: any direction; the edge only has to exist.
    Connected,
    /// `parent`: used in the guide's examples, not in its documented list.
    ExampleOnlyParent,
    /// `child`: used in the guide's examples, not in its documented list.
    ExampleOnlyChild,
}

impl Direction {
    /// True for the three directions the structure page lists.
    #[must_use]
    pub fn is_documented(self) -> bool {
        matches!(self, Direction::In | Direction::Out | Direction::Connected)
    }

    /// The direction as written.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Direction::In => "in",
            Direction::Out => "out",
            Direction::Connected => "connected",
            Direction::ExampleOnlyParent => "parent",
            Direction::ExampleOnlyChild => "child",
        }
    }
}

/// `with` (a second vertex over an edge) or `count` (the number of such
/// vertices).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockKind {
    /// `with <vertexType>@<dir>[:<edgeType>] [as Alias] ... end`.
    With,
    /// `count <vertexType>@<dir>[:<edgeType>] as Name ... end`.
    Count,
}

impl BlockKind {
    /// The keyword.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            BlockKind::With => "with",
            BlockKind::Count => "count",
        }
    }
}

/// A `with` or `count` block inside `on`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedBlock {
    /// `with` or `count`.
    pub kind: BlockKind,
    /// The vertex type of the related vertex.
    pub vertex_type: Name,
    /// The edge direction.
    pub direction: Direction,
    /// Where the direction was written.
    pub direction_span: Span,
    /// The edge type after `:`, if given. Omitted means any edge type.
    pub edge_type: Option<Name>,
    /// `as <Alias>`. Always present for `count`.
    pub alias: Option<Ident>,
    /// The conditions on the related vertex, one per line.
    pub conds: Vec<Cond>,
    /// From the keyword to `end`.
    pub span: Span,
}

// ── statements ───────────────────────────────────────────────────────

/// `tag: Var` in front of a call: the call's returned value `tag` is stored
/// in `Var`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The returned tag (`exit`, `stdout`, `result`).
    pub field: Ident,
    /// The variable it is stored in (`LOCAL::STATUS`, `date`).
    pub target: Name,
}

/// `begin ... end`: a block of statements passed as an argument (`each:`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// The statements.
    pub stmts: Vec<Stmt>,
    /// From `begin` to `end`.
    pub span: Span,
}

/// The value of one call argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgValue {
    /// An expression.
    Expr(Expr),
    /// A `begin ... end` block.
    Block(Block),
}

impl ArgValue {
    /// The span of the value.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            ArgValue::Expr(e) => e.span(),
            ArgValue::Block(b) => b.span,
        }
    }
}

/// One argument of a call: positional (`name` is None) or keyword.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallArg {
    /// The keyword, for `name: value`.
    pub name: Option<Ident>,
    /// The value.
    pub value: ArgValue,
}

/// A call to any function: `execute("x", timeout: 60)`. Only the function
/// name and the arguments are recorded; no meaning is given to them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallStmt {
    /// Result tags in front, in the order written (possibly empty).
    pub bindings: Vec<Binding>,
    /// The function name.
    pub function: Ident,
    /// The arguments, in the order written.
    pub args: Vec<CallArg>,
    /// From the first tag (or the function name) to `)`.
    pub span: Span,
}

/// `name: value` inside `action(...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedArg {
    /// The argument name.
    pub name: Ident,
    /// The value.
    pub value: ArgValue,
}

/// How the capability was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityForm {
    /// The first positional string: `action("ExecuteCommand", ...)`.
    Positional,
    /// A named argument: `action(capability: "HTTPRequest", ...)`.
    Named,
}

/// One `action(...)` call, with the result tags in front of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionCall {
    /// Result tags, in the order written (possibly empty).
    pub bindings: Vec<Binding>,
    /// The capability name.
    pub capability: StrLit,
    /// How the capability was written.
    pub capability_form: CapabilityForm,
    /// The other named arguments, in the order written. The `capability:`
    /// argument is not in this list.
    pub args: Vec<NamedArg>,
    /// From the first tag (or `action`) to the closing parenthesis.
    pub span: Span,
}

/// The log level of `log`, `log.debug`, `log.info`, `log.warn`, `log.error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    /// Plain `log(...)`: the guide's level is "info (default)".
    Default,
    /// `log.debug`.
    Debug,
    /// `log.info`.
    Info,
    /// `log.warn`.
    Warn,
    /// `log.error`.
    Error,
}

impl LogLevel {
    /// The call as written, without the argument list.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            LogLevel::Default => "log",
            LogLevel::Debug => "log.debug",
            LogLevel::Info => "log.info",
            LogLevel::Warn => "log.warn",
            LogLevel::Error => "log.error",
        }
    }
}

/// `if cond then ... [else ...] end`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfStmt {
    /// The condition.
    pub cond: Cond,
    /// Statements of the `then` branch.
    pub then_branch: Vec<Stmt>,
    /// Statements of the `else` branch, if there is one (the `if` page calls
    /// it optional).
    pub else_branch: Option<Vec<Stmt>>,
    /// From `if` to `end`.
    pub span: Span,
}

/// A statement of the `do` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    /// `Var = <expression>`.
    Assign {
        /// Target, with qualifier, key and limiter as written.
        target: VarRef,
        /// Value.
        value: Expr,
        /// Whole statement.
        span: Span,
    },
    /// `set(Var = value)`.
    Set {
        /// Target.
        target: VarRef,
        /// Value.
        value: Expr,
        /// Whole statement.
        span: Span,
    },
    /// `add(Var = value)`.
    Add {
        /// Target.
        target: VarRef,
        /// Value.
        value: Expr,
        /// Whole statement.
        span: Span,
    },
    /// `action(...)`, with optional result tags.
    Action(ActionCall),
    /// `delete(Var)`, with the optional `created_on:` argument.
    Delete {
        /// What is deleted.
        target: VarRef,
        /// `created_on: <expr>`, if given.
        created_on: Option<Expr>,
        /// Whole statement.
        span: Span,
    },
    /// `log(...)` or `log.<level>(...)`.
    Log {
        /// The level.
        level: LogLevel,
        /// The message: a string with `${...}` kept verbatim, or a variable.
        message: Expr,
        /// Whole statement.
        span: Span,
    },
    /// Any other function: `execute`, `javascript`, `query`, `dateprint`...
    Call(CallStmt),
    /// `if ... then ... else ... end`.
    If(IfStmt),
}

impl Stmt {
    /// The span of the statement.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Stmt::Assign { span, .. }
            | Stmt::Set { span, .. }
            | Stmt::Add { span, .. }
            | Stmt::Delete { span, .. }
            | Stmt::Log { span, .. } => *span,
            Stmt::Action(a) => a.span,
            Stmt::Call(c) => c.span,
            Stmt::If(i) => i.span,
        }
    }
}

// ── sections ─────────────────────────────────────────────────────────

/// The execution tier of a KI (structure page, "KI Tiers").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    /// `"default"`: the normal priority; also what an absent tier means.
    Default,
    /// `"policy"`: runs before KIs of the other tiers. The ordering applies
    /// only among items on the same vertex: "KI tiers are only applied on a
    /// vertex local scope" (structure page), so a matching `fallback` item
    /// on the current vertex runs before the issue is routed to a `policy`
    /// item on a connected remote vertex.
    Policy,
    /// `"fallback"`: runs only when no default or policy KI matches. Like
    /// `policy`, this holds only among items on the same vertex (structure
    /// page, "KI tiers are only applied on a vertex local scope").
    Fallback,
}

impl Tier {
    /// The value as written.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Tier::Default => "default",
            Tier::Policy => "policy",
            Tier::Fallback => "fallback",
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
    /// `version: "..."`.
    pub version: Option<StrLit>,
    /// `tier: "..."`, as the string written.
    pub tier_text: Option<StrLit>,
    /// The tier, typed.
    pub tier: Option<Tier>,
    /// `id: "..."`. The RL user guide's example writes `id:` where the
    /// structure page requires `name:`. Recorded as its own key.
    pub id: Option<StrLit>,
    /// From `ki` to the last header line.
    pub span: Span,
}

/// One line of the `on` section: a condition or a related-node block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnItem {
    /// A condition on the node.
    Cond(Cond),
    /// A `with` or `count` block.
    Block(RelatedBlock),
}

impl OnItem {
    /// The span of the item.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            OnItem::Cond(c) => c.span(),
            OnItem::Block(b) => b.span,
        }
    }
}

/// The `on` section: conditions on the node ("according to the MARS-Model
/// Node data"). Lines are joined by an implicit `and`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnClause {
    /// The conditions and blocks, in order.
    pub items: Vec<OnItem>,
    /// From `on` to the end of the last item.
    pub span: Span,
}

impl OnClause {
    /// The top-level conditions (not the ones inside blocks).
    pub fn conds(&self) -> impl Iterator<Item = &Cond> {
        self.items.iter().filter_map(|i| match i {
            OnItem::Cond(c) => Some(c),
            OnItem::Block(_) => None,
        })
    }

    /// The `with` and `count` blocks.
    pub fn blocks(&self) -> impl Iterator<Item = &RelatedBlock> {
        self.items.iter().filter_map(|i| match i {
            OnItem::Block(b) => Some(b),
            OnItem::Cond(_) => None,
        })
    }
}

/// The `when` section: conditions on the issue. Lines are joined by an
/// implicit `and`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhenClause {
    /// The conditions, in order.
    pub conds: Vec<Cond>,
    /// From `when` to the end of the last condition.
    pub span: Span,
}

/// The `do` section.
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
    /// The `on` section.
    pub on: OnClause,
    /// The `when` section.
    pub when: WhenClause,
    /// The `do` section.
    pub do_: DoClause,
    /// The whole item.
    pub span: Span,
    /// The item's text, as parsed.
    pub source: String,
}

// ── outline ──────────────────────────────────────────────────────────

fn p(pos: Pos) -> String {
    format!("{}:{}", pos.line, pos.col)
}

fn key_text(k: &Key) -> String {
    match k {
        Key::Str(s) => format!("[{:?}]", s.value),
        Key::Number(n) => format!("[{}]", n.text),
        Key::Var(v) => format!("[{}]", v.text),
    }
}

fn graph_text(g: GraphScopeKind) -> &'static str {
    match g {
        GraphScopeKind::Any => "@any",
        GraphScopeKind::SelfOnly => "@self",
        GraphScopeKind::InAsWrittenInExample => "@in",
    }
}

fn var_text(v: &VarRef) -> String {
    let mut s = v.name.text.clone();
    if let Some(k) = &v.key {
        s.push_str(&key_text(k));
    }
    if let Some(g) = &v.graph {
        s.push_str(graph_text(g.kind));
    }
    s
}

pub(crate) fn expr_text(e: &Expr) -> String {
    match e {
        Expr::Str(s) => format!("str {:?}", s.value),
        Expr::Number(n) => format!("num {}", n.text),
        Expr::Var(v) => format!("var {}", var_text(v)),
        Expr::List { items, .. } => {
            let inner: Vec<String> = items.iter().map(expr_text).collect();
            format!("[{}]", inner.join(", "))
        }
        Expr::Regex(r) => format!("re /{}/{}", r.pattern, r.flags),
        Expr::Binary {
            op, left, right, ..
        } => format!("({} {} {})", op.text(), expr_text(left), expr_text(right)),
    }
}

fn leaf_outline(l: &Leaf) -> String {
    if l.is_plain() {
        return match &l.test {
            None => format!("(bare@{} {})", p(l.span.start), l.subject.name.text),
            Some(Comparison {
                right: Expr::Str(s),
                ..
            }) => format!(
                "(eq@{} {} {:?})",
                p(l.span.start),
                l.subject.name.text,
                s.value
            ),
            Some(_) => unreachable!("is_plain admits only a string equality"),
        };
    }
    let mut out = format!("(leaf@{}", p(l.span.start));
    if l.optional {
        out.push_str(" ?");
    }
    if l.list {
        out.push_str(" list");
    }
    let _ = write!(out, " {}", var_text(&l.subject));
    if let Some(c) = &l.test {
        let _ = write!(out, " {} {}", c.op.text(), expr_text(&c.right));
    }
    if let Some(a) = &l.alias {
        let _ = write!(out, " as {}", a.text);
    }
    if let Some(d) = &l.default {
        let _ = write!(out, " := {}", expr_text(d));
    }
    out.push(')');
    out
}

fn cond_outline(c: &Cond) -> String {
    match c {
        Cond::Leaf(l) => leaf_outline(l),
        Cond::Not { inner, span } => format!("(not@{} {})", p(span.start), cond_outline(inner)),
        Cond::Group { inner, span } => {
            format!("(group@{} {})", p(span.start), cond_outline(inner))
        }
        Cond::And { parts, span } | Cond::Or { parts, span } => {
            let word = if matches!(c, Cond::And { .. }) {
                "and"
            } else {
                "or"
            };
            let inner: Vec<String> = parts.iter().map(cond_outline).collect();
            format!("({word}@{} {})", p(span.start), inner.join(" "))
        }
    }
}

fn arg_text(a: &ArgValue) -> String {
    match a {
        ArgValue::Expr(Expr::Str(s)) => format!("{:?}", s.value),
        ArgValue::Expr(e) => expr_text(e),
        ArgValue::Block(b) => format!("begin<{} stmts>", b.stmts.len()),
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
            let _ = writeln!(
                out,
                "{pad}(assign@{} {} {})",
                p(span.start),
                var_text(target),
                expr_text(value)
            );
        }
        Stmt::Set {
            target,
            value,
            span,
        } => {
            let _ = writeln!(
                out,
                "{pad}(set@{} {} {})",
                p(span.start),
                var_text(target),
                expr_text(value)
            );
        }
        Stmt::Add {
            target,
            value,
            span,
        } => {
            let _ = writeln!(
                out,
                "{pad}(add@{} {} {})",
                p(span.start),
                var_text(target),
                expr_text(value)
            );
        }
        Stmt::Delete {
            target,
            created_on,
            span,
        } => {
            let _ = write!(out, "{pad}(delete@{} {}", p(span.start), var_text(target));
            if let Some(c) = created_on {
                let _ = write!(out, " created_on={}", expr_text(c));
            }
            let _ = writeln!(out, ")");
        }
        Stmt::Log {
            level,
            message,
            span,
        } => {
            let shown = match message {
                Expr::Str(s) => format!("{:?}", s.value),
                other => expr_text(other),
            };
            let word = level.text();
            let _ = writeln!(out, "{pad}({}@{} {shown})", word, p(span.start));
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
                let _ = write!(out, " [{} -> {}]", b.field.text, b.target.text);
            }
            for g in &a.args {
                let _ = write!(out, " {}={}", g.name.text, arg_text(&g.value));
            }
            let _ = writeln!(out, ")");
        }
        Stmt::Call(c) => {
            let _ = write!(out, "{pad}(call@{} {}", p(c.span.start), c.function.text);
            for b in &c.bindings {
                let _ = write!(out, " [{} -> {}]", b.field.text, b.target.text);
            }
            for a in &c.args {
                match &a.name {
                    Some(n) => {
                        let _ = write!(out, " {}={}", n.text, arg_text(&a.value));
                    }
                    None => {
                        let _ = write!(out, " {}", arg_text(&a.value));
                    }
                }
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
            if let Some(e) = &i.else_branch {
                let _ = writeln!(out, "{pad}  else");
                for t in e {
                    stmt_outline(t, out, depth + 2);
                }
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
        for (key, v) in [
            ("id", &h.id),
            ("name", &h.name),
            ("version", &h.version),
            ("description", &h.description),
            ("tier", &h.tier_text),
        ] {
            if let Some(v) = v {
                let _ = writeln!(out, "  {key}@{} {:?}", p(v.span.start), v.value);
            }
        }
    }
    let _ = writeln!(out, "on@{}", p(item.on.span.start));
    for i in &item.on.items {
        match i {
            OnItem::Cond(c) => {
                let _ = writeln!(out, "  {}", cond_outline(c));
            }
            OnItem::Block(b) => {
                let _ = write!(
                    out,
                    "  ({}@{} {}@{}",
                    b.kind.word(),
                    p(b.span.start),
                    b.vertex_type.text,
                    b.direction.text()
                );
                if let Some(e) = &b.edge_type {
                    let _ = write!(out, ":{}", e.text);
                }
                if let Some(a) = &b.alias {
                    let _ = write!(out, " as {}", a.text);
                }
                let _ = writeln!(out);
                for c in &b.conds {
                    let _ = writeln!(out, "    {}", cond_outline(c));
                }
                let _ = writeln!(out, "  )");
            }
        }
    }
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
