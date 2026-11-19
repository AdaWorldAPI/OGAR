//! Scope resolution as data, not evaluation.
//!
//! The scope page of the guide says: `on` works on the NODE scope, `when`
//! works on the ISSUE scope, `LOCAL` is the current execution context. A name
//! with no prefix is looked up LOCAL, then ISSUE, then NODE. The page also
//! lists the scopes each section may use (`on`: NODE and LOCAL; `when`: ISSUE
//! and LOCAL), while the matching page writes `NODE::Hostname` inside a
//! `when`.
//!
//! [`KnowledgeItem::condition_refs`] lists, for every variable named in an
//! `on` or `when` condition, which section it sits in, the prefix written,
//! and that documented lookup order. It looks nothing up and checks nothing:
//! a prefix outside the permitted list is flagged
//! ([`ConditionRef::outside_permitted_scopes`]), not refused.

use crate::ast::{
    BlockKind, CmpOp, Cond, Expr, GraphScopeKind, KnowledgeItem, Leaf, OnItem, Pos, Qualifier,
    RelatedBlock, ScopeSpelling, VarRef,
};

/// The section a condition is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    /// The `on` section (conditions on the node).
    On,
    /// The `when` section (conditions on the issue).
    When,
}

/// One of the three documented scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    /// The MARS node data.
    Node,
    /// The issue data.
    Issue,
    /// The current execution context.
    Local,
}

impl Section {
    /// The scope the guide gives the section: `on` is NODE, `when` is ISSUE.
    #[must_use]
    pub fn scope(self) -> Scope {
        match self {
            Section::On => Scope::Node,
            Section::When => Scope::Issue,
        }
    }
}

/// What the name is used for in its condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefOperator {
    /// The variable stands alone: it has to exist.
    Exists,
    /// The variable is an operand of this comparison.
    Compare(CmpOp),
}

/// Where in its condition the name sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefRole {
    /// The variable on the left of the leaf.
    Subject,
    /// A variable on the right of a comparison.
    RightOperand,
    /// A variable used as a key: `VarC[VarA]` (`VarA`).
    Key,
    /// A variable in the `:= default` of an optional match.
    Default,
}

/// The `with` or `count` block a name sits in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRef {
    /// `with` or `count`.
    pub kind: BlockKind,
    /// The block's vertex type.
    pub vertex_type: String,
    /// The block's alias, if it has one.
    pub alias: Option<String>,
    /// Where the block starts.
    pub pos: Pos,
}

/// One variable named in an `on` or `when` condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionRef {
    /// The section it is written in.
    pub section: Section,
    /// The scope the guide gives that section.
    pub section_scope: Scope,
    /// The qualifier written in front of the name, if any.
    pub written: Option<Qualifier>,
    /// How a NODE qualifier was spelled (`NODE::` or `Node::`).
    pub scope_spelling: Option<ScopeSpelling>,
    /// The name without its qualifier.
    pub name: String,
    /// The name as written, with its qualifier.
    pub name_as_written: String,
    /// The comparison it takes part in, or plain existence.
    pub operator: RefOperator,
    /// Where in the condition it sits.
    pub role: RefRole,
    /// Under an odd number of `not`.
    pub negated: bool,
    /// The leaf is an optional match (`?`).
    pub optional: bool,
    /// The leaf is a `list` match.
    pub list: bool,
    /// The `@any`, `@self` limiter after the name, as written.
    pub graph: Option<GraphScopeKind>,
    /// The documented lookup order for this name: the written qualifier
    /// alone, or LOCAL, ISSUE, NODE for a name with none.
    pub lookup: Vec<Qualifier>,
    /// The written qualifier is NODE in a `when` or ISSUE in an `on`: outside
    /// the scope page's permitted list. The matching page writes
    /// `NODE::Hostname` in a `when`; the item is not refused for it.
    pub outside_permitted_scopes: bool,
    /// The block the name sits in, for a condition inside `with` or `count`.
    pub block: Option<BlockRef>,
    /// Where the name starts.
    pub pos: Pos,
}

/// The documented lookup order for an unscoped name.
#[must_use]
pub fn default_lookup_order() -> Vec<Qualifier> {
    vec![Qualifier::Local, Qualifier::Issue, Qualifier::Node]
}

struct Ctx<'a> {
    section: Section,
    block: Option<BlockRef>,
    out: &'a mut Vec<ConditionRef>,
}

impl Ctx<'_> {
    fn row(&mut self, v: &VarRef, role: RefRole, op: RefOperator, leaf: &Leaf, negated: bool) {
        let written = v.name.qualifier();
        let outside = matches!(
            (self.section, &written),
            (Section::When, Some(Qualifier::Node)) | (Section::On, Some(Qualifier::Issue))
        );
        let lookup = match &written {
            Some(q) => vec![q.clone()],
            None => default_lookup_order(),
        };
        self.out.push(ConditionRef {
            section: self.section,
            section_scope: self.section.scope(),
            written,
            scope_spelling: v.name.scope_spelling(),
            name: v.name.base().to_string(),
            name_as_written: v.name.text.clone(),
            operator: op,
            role,
            negated,
            optional: leaf.optional,
            list: leaf.list,
            graph: v.graph.map(|g| g.kind),
            lookup,
            outside_permitted_scopes: outside,
            block: self.block.clone(),
            pos: v.span.start,
        });
    }

    fn expr(&mut self, e: &Expr, role: RefRole, op: RefOperator, leaf: &Leaf, negated: bool) {
        match e {
            Expr::Var(v) => {
                self.var(v, role, op, leaf, negated);
            }
            Expr::List { items, .. } => {
                for i in items {
                    self.expr(i, role, op, leaf, negated);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.expr(left, role, op, leaf, negated);
                self.expr(right, role, op, leaf, negated);
            }
            Expr::Str(_) | Expr::Number(_) | Expr::Regex(_) => {}
        }
    }

    /// The variable itself, then the variable used as its key.
    fn var(&mut self, v: &VarRef, role: RefRole, op: RefOperator, leaf: &Leaf, negated: bool) {
        self.row(v, role, op, leaf, negated);
        if let Some(crate::ast::Key::Var(k)) = &v.key {
            let key_ref = VarRef {
                name: k.clone(),
                key: None,
                graph: None,
                span: k.span,
            };
            self.row(&key_ref, RefRole::Key, op, leaf, negated);
        }
    }

    fn cond(&mut self, c: &Cond, negated: bool) {
        match c {
            Cond::Leaf(l) => {
                let op = match &l.test {
                    None => RefOperator::Exists,
                    Some(t) => RefOperator::Compare(t.op),
                };
                self.var(&l.subject, RefRole::Subject, op, l, negated);
                if let Some(t) = &l.test {
                    self.expr(&t.right, RefRole::RightOperand, op, l, negated);
                }
                if let Some(d) = &l.default {
                    self.expr(d, RefRole::Default, op, l, negated);
                }
            }
            Cond::Not { inner, .. } => self.cond(inner, !negated),
            Cond::Group { inner, .. } => self.cond(inner, negated),
            Cond::And { parts, .. } | Cond::Or { parts, .. } => {
                for p in parts {
                    self.cond(p, negated);
                }
            }
        }
    }
}

fn block_ref(b: &RelatedBlock) -> BlockRef {
    BlockRef {
        kind: b.kind,
        vertex_type: b.vertex_type.text.clone(),
        alias: b.alias.as_ref().map(|a| a.text.clone()),
        pos: b.span.start,
    }
}

impl KnowledgeItem {
    /// Every variable named in an `on` or `when` condition, in text order,
    /// with the section it sits in, the qualifier written, and the documented
    /// lookup order. Nothing is evaluated or looked up.
    #[must_use]
    pub fn condition_refs(&self) -> Vec<ConditionRef> {
        let mut out = Vec::new();
        for item in &self.on.items {
            match item {
                OnItem::Cond(c) => {
                    Ctx {
                        section: Section::On,
                        block: None,
                        out: &mut out,
                    }
                    .cond(c, false);
                }
                OnItem::Block(b) => {
                    let mut cx = Ctx {
                        section: Section::On,
                        block: Some(block_ref(b)),
                        out: &mut out,
                    };
                    for c in &b.conds {
                        cx.cond(c, false);
                    }
                }
            }
        }
        for c in &self.when.conds {
            Ctx {
                section: Section::When,
                block: None,
                out: &mut out,
            }
            .cond(c, false);
        }
        out
    }
}
