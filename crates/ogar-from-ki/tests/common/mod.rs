#![allow(dead_code)]

pub const TUTORIAL: &str = include_str!("../fixtures/tutorial_nothing_to_do.ki");
pub const HTTP: &str = include_str!("../fixtures/http_request.ki");
pub const SSH_LINE: &str = include_str!("../fixtures/ssh_execute_command_line.ki");

/// SYNTHETIC wrapper: the third public example is one `do` statement, not a
/// complete item, so it is placed under a header written here.
pub fn ssh_wrapped() -> String {
    format!("on\n  ogit/_id\nwhen\n  Synthetic\ndo\n{SSH_LINE}")
}

/// SYNTHETIC: a complete item around the given `when` lines and `do` lines.
pub fn item(when: &str, do_: &str) -> String {
    format!("on\n  ogit/_id\nwhen\n  {when}\ndo\n  {do_}\n")
}

// ── the guide fixtures (HIRO User Guides 7.0, KI Syntax) ────────────────

use ogar_from_ki::ast::{ArgValue, Cond, Expr, GraphScopeKind, Stmt};

pub const GUIDE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/guide/");

/// The text of one guide fixture, read from disk.
pub fn guide(name: &str) -> String {
    std::fs::read_to_string(format!("{GUIDE_DIR}{name}.ki")).expect("fixture exists")
}

/// A one-token-per-node description of a condition's shape, e.g. `exists`,
/// `==[key]`, `?< :=`, `not(exists)`, `and((>),(<))`.
pub fn shape(c: &Cond) -> String {
    match c {
        Cond::Leaf(l) => {
            let mut s = String::new();
            if l.optional {
                s.push('?');
            }
            if l.list {
                s.push_str("list ");
            }
            s.push_str(match &l.test {
                None => "exists",
                Some(t) => t.op.text(),
            });
            if l.subject.key.is_some() {
                s.push_str("[key]");
            }
            if let Some(g) = &l.subject.graph {
                s.push_str(match g.kind {
                    GraphScopeKind::Any => "@any",
                    GraphScopeKind::SelfOnly => "@self",
                    GraphScopeKind::InAsWrittenInExample => "@in",
                });
            }
            if l.alias.is_some() {
                s.push_str(" as");
            }
            if l.default.is_some() {
                s.push_str(" :=");
            }
            s
        }
        Cond::Not { inner, .. } => format!("not({})", shape(inner)),
        Cond::Group { inner, .. } => format!("({})", shape(inner)),
        Cond::And { parts, .. } => {
            let p: Vec<String> = parts.iter().map(shape).collect();
            format!("and({})", p.join(","))
        }
        Cond::Or { parts, .. } => {
            let p: Vec<String> = parts.iter().map(shape).collect();
            format!("or({})", p.join(","))
        }
    }
}

/// The kind of each statement of a `do` list, not descending.
pub fn stmt_kinds(stmts: &[Stmt]) -> Vec<String> {
    stmts
        .iter()
        .map(|s| match s {
            Stmt::Assign { .. } => "assign".to_string(),
            Stmt::Set { .. } => "set".to_string(),
            Stmt::Add { .. } => "add".to_string(),
            Stmt::Delete { .. } => "delete".to_string(),
            Stmt::Log { level, .. } => level.text().to_string(),
            Stmt::Action(_) => "action".to_string(),
            Stmt::Call(c) => format!("call:{}", c.function.text),
            Stmt::If(_) => "if".to_string(),
        })
        .collect()
}

fn block_stmts<'a>(v: &'a ArgValue, out: &mut Vec<&'a Stmt>) {
    if let ArgValue::Block(b) = v {
        out.extend(b.stmts.iter());
    }
}

/// The function names called in a `do` list, in text order, descending into
/// `if` branches and `begin` blocks. Assignments are not calls.
pub fn called(stmts: &[Stmt]) -> Vec<String> {
    let mut out = Vec::new();
    for s in stmts {
        match s {
            Stmt::Assign { .. } => {}
            Stmt::Set { .. } => out.push("set".to_string()),
            Stmt::Add { .. } => out.push("add".to_string()),
            Stmt::Delete { .. } => out.push("delete".to_string()),
            Stmt::Log { level, .. } => out.push(level.text().to_string()),
            Stmt::Action(a) => {
                out.push("action".to_string());
                let mut inner = Vec::new();
                for g in &a.args {
                    block_stmts(&g.value, &mut inner);
                }
                let owned: Vec<Stmt> = inner.into_iter().cloned().collect();
                out.extend(called(&owned));
            }
            Stmt::Call(c) => {
                out.push(c.function.text.clone());
                let mut inner = Vec::new();
                for g in &c.args {
                    block_stmts(&g.value, &mut inner);
                }
                let owned: Vec<Stmt> = inner.into_iter().cloned().collect();
                out.extend(called(&owned));
            }
            Stmt::If(i) => {
                out.extend(called(&i.then_branch));
                if let Some(e) = &i.else_branch {
                    out.extend(called(e));
                }
            }
        }
    }
    out
}

/// Whether an expression is the string `value`.
pub fn is_str(e: &Expr, value: &str) -> bool {
    matches!(e, Expr::Str(s) if s.value == value)
}
