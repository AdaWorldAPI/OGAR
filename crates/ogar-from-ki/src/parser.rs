//! The hand-written parser for the documented subset. See the crate README
//! for the grammar as implemented.

use crate::ast::{
    ActionCall, Binding, CapabilityForm, Cond, DoClause, Header, Ident, IfStmt, KnowledgeItem,
    Name, NamedArg, Number, OnClause, Pos, Span, Stmt, StrLit, Value, WhenClause,
};
use crate::error::{ErrorKind, ParseError};
use crate::lexer::{Tok, Token, lex};

type Res<T> = Result<T, ParseError>;

const CLAUSES: [&str; 4] = ["ki", "on", "when", "do"];
const RESERVED: [&str; 8] = ["ki", "on", "when", "do", "if", "then", "else", "end"];

/// Parse one Knowledge Item of the documented subset.
///
/// # Errors
///
/// A [`ParseError`] naming the first construct outside the subset, with its
/// line and column and what was expected. Never panics, on any input.
pub fn parse(src: &str) -> Res<KnowledgeItem> {
    let mut p = Parser {
        toks: lex(src),
        i: 0,
    };
    let mut item = p.item()?;
    item.source = src.to_string();
    Ok(item)
}

struct Parser {
    toks: Vec<Token>,
    i: usize,
}

fn word(t: &Token) -> Option<&str> {
    match &t.tok {
        Tok::Name(w) => Some(w.as_str()),
        _ => None,
    }
}

fn describe(t: &Token) -> String {
    match &t.tok {
        Tok::Name(w) => format!("`{w}`"),
        Tok::Str(s) => format!("string \"{s}\""),
        Tok::Number(n) => format!("number `{n}`"),
        Tok::EqEq => "`==`".to_string(),
        Tok::Eq => "`=`".to_string(),
        Tok::Colon => "`:`".to_string(),
        Tok::Comma => "`,`".to_string(),
        Tok::LParen => "`(`".to_string(),
        Tok::RParen => "`)`".to_string(),
        Tok::Newline => "end of line".to_string(),
        Tok::Operator(o) => format!("operator `{o}`"),
        Tok::Bad(_) => "invalid input".to_string(),
        Tok::Eof => "end of input".to_string(),
    }
}

impl Parser {
    fn peek(&self) -> Res<&Token> {
        self.peek_at(0)
    }

    fn peek_at(&self, n: usize) -> Res<&Token> {
        let t = &self.toks[(self.i + n).min(self.toks.len() - 1)];
        if let Tok::Bad(e) = &t.tok {
            return Err(e.clone());
        }
        Ok(t)
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.i.min(self.toks.len() - 1)].clone();
        if self.i < self.toks.len() - 1 {
            self.i += 1;
        }
        t
    }

    fn skip_nl(&mut self) {
        while matches!(self.toks[self.i].tok, Tok::Newline) {
            self.i += 1;
        }
    }

    fn fail<T>(&self, t: &Token, kind: ErrorKind, expected: &str) -> Res<T> {
        Err(ParseError {
            kind,
            pos: t.span.start,
            expected: expected.to_string(),
        })
    }

    fn unexpected<T>(&self, t: &Token, expected: &str) -> Res<T> {
        let found = describe(t);
        self.fail(t, ErrorKind::UnexpectedToken { found }, expected)
    }

    /// The statement or condition must end here: a newline or the end of the text.
    fn expect_eol(&mut self) -> Res<()> {
        let t = self.peek()?.clone();
        match &t.tok {
            Tok::Newline => {
                self.bump();
                Ok(())
            }
            Tok::Eof => Ok(()),
            Tok::Name(w) if matches!(w.as_str(), "and" | "or" | "not") => self.fail(
                &t,
                ErrorKind::UnsupportedConnective { word: w.clone() },
                "end of line (no way to combine conditions is documented)",
            ),
            _ => self.unexpected(&t, "end of line"),
        }
    }

    // ── structure ────────────────────────────────────────────────────

    fn item(&mut self) -> Res<KnowledgeItem> {
        self.skip_nl();
        let start = self.peek()?.span.start;
        let header = if word(self.peek()?) == Some("ki") {
            Some(self.header()?)
        } else {
            None
        };
        let on = self.on_clause()?;
        let when = self.when_clause()?;
        let do_ = self.do_clause()?;
        let end = do_.span.end;
        Ok(KnowledgeItem {
            header,
            on,
            when,
            do_,
            span: Span { start, end },
            source: String::new(),
        })
    }

    fn header(&mut self) -> Res<Header> {
        let kw = self.bump();
        let mut name: Option<StrLit> = None;
        let mut description: Option<StrLit> = None;
        let mut end = kw.span.end;
        loop {
            self.skip_nl();
            let t = self.peek()?.clone();
            let Some(key) = word(&t).map(str::to_string) else {
                break;
            };
            if CLAUSES.contains(&key.as_str()) {
                break;
            }
            if !matches!(self.peek_at(1)?.tok, Tok::Colon) {
                break;
            }
            let slot = match key.as_str() {
                "name" => &mut name,
                "description" => &mut description,
                _ => {
                    return self.fail(
                        &t,
                        ErrorKind::UnknownHeaderKey { key },
                        "`name:` or `description:`",
                    );
                }
            };
            if slot.is_some() {
                return self.fail(
                    &t,
                    ErrorKind::DuplicateHeaderKey { key },
                    "each header key at most once",
                );
            }
            self.bump();
            self.bump();
            let v = self.peek()?.clone();
            let Tok::Str(s) = &v.tok else {
                return self.unexpected(&v, "a string literal");
            };
            *slot = Some(StrLit {
                value: s.clone(),
                span: v.span,
            });
            end = v.span.end;
            self.bump();
            self.expect_eol()?;
        }
        Ok(Header {
            name,
            description,
            span: Span {
                start: kw.span.start,
                end,
            },
        })
    }

    /// Consume the clause keyword `kw`, or refuse by name.
    fn expect_clause(&mut self, kw: &'static str) -> Res<Token> {
        self.skip_nl();
        let t = self.peek()?.clone();
        match &t.tok {
            Tok::Name(w) if w == kw => Ok(self.bump()),
            Tok::Name(w) if CLAUSES.contains(&w.as_str()) => self.fail(
                &t,
                ErrorKind::UnexpectedClause {
                    found: w.clone(),
                    wanted: kw,
                },
                &format!("`{kw}` (clauses come in the order ki, on, when, do, each once)"),
            ),
            Tok::Name(w) => self.fail(
                &t,
                ErrorKind::UnknownClauseKeyword { word: w.clone() },
                &format!("`{kw}`"),
            ),
            Tok::Eof => self.fail(
                &t,
                ErrorKind::MissingClause { clause: kw },
                &format!("`{kw}`"),
            ),
            _ => self.unexpected(&t, &format!("`{kw}`")),
        }
    }

    /// At a clause body: refuse if it is empty (next is a clause keyword or the end).
    fn body_not_empty(&mut self, clause: &'static str) -> Res<()> {
        self.skip_nl();
        let t = self.peek()?.clone();
        let empty = match &t.tok {
            Tok::Eof => true,
            Tok::Name(w) => CLAUSES.contains(&w.as_str()) && t.first_on_line,
            _ => false,
        };
        if empty {
            return self.fail(
                &t,
                ErrorKind::EmptyClause { clause },
                &format!("a body for `{clause}`"),
            );
        }
        Ok(())
    }

    /// At the start of a body line: a clause keyword, or an unknown word at
    /// column 1, ends/refuses the body. Returns true when the body ends here
    /// at a known clause keyword.
    fn line_start_check(&self, t: &Token) -> Res<bool> {
        if let (Tok::Name(w), true) = (&t.tok, t.first_on_line) {
            if CLAUSES.contains(&w.as_str()) {
                return Ok(true);
            }
            if t.span.start.col == 1 && !RESERVED.contains(&w.as_str()) {
                return self.fail(
                    t,
                    ErrorKind::UnknownClauseKeyword { word: w.clone() },
                    "a clause keyword (`on`, `when`, `do`) at column 1",
                );
            }
        }
        Ok(false)
    }

    fn on_clause(&mut self) -> Res<OnClause> {
        let kw = self.expect_clause("on")?;
        self.body_not_empty("on")?;
        let cond = self.cond()?;
        self.expect_eol()?;
        let span = Span {
            start: kw.span.start,
            end: cond.span().end,
        };
        Ok(OnClause { cond, span })
    }

    fn when_clause(&mut self) -> Res<WhenClause> {
        let kw = self.expect_clause("when")?;
        self.body_not_empty("when")?;
        let mut conds: Vec<Cond> = Vec::new();
        loop {
            self.skip_nl();
            let t = self.peek()?.clone();
            if matches!(t.tok, Tok::Eof) {
                return self.fail(&t, ErrorKind::MissingClause { clause: "do" }, "`do`");
            }
            if self.line_start_check(&t)? {
                break;
            }
            conds.push(self.cond()?);
            self.expect_eol()?;
        }
        let end = conds.last().map_or(kw.span.end, |c| c.span().end);
        Ok(WhenClause {
            conds,
            span: Span {
                start: kw.span.start,
                end,
            },
        })
    }

    fn do_clause(&mut self) -> Res<DoClause> {
        let kw = self.expect_clause("do")?;
        self.body_not_empty("do")?;
        let mut stmts: Vec<Stmt> = Vec::new();
        loop {
            self.skip_nl();
            let t = self.peek()?.clone();
            if matches!(t.tok, Tok::Eof) {
                break;
            }
            if self.line_start_check(&t)? {
                let w = word(&t).unwrap_or_default().to_string();
                return self.fail(
                    &t,
                    ErrorKind::UnexpectedClause {
                        found: w,
                        wanted: "end of the Knowledge Item",
                    },
                    "a statement (one Knowledge Item per text; `do` is the last clause)",
                );
            }
            stmts.push(self.stmt(false)?);
            self.expect_eol()?;
        }
        let end = stmts.last().map_or(kw.span.end, |s| s.span().end);
        Ok(DoClause {
            stmts,
            span: Span {
                start: kw.span.start,
                end,
            },
        })
    }

    // ── names, conditions ────────────────────────────────────────────

    fn name(&mut self) -> Res<Name> {
        let t = self.peek()?.clone();
        let Tok::Name(text) = &t.tok else {
            return self.unexpected(&t, "a name");
        };
        if RESERVED.contains(&text.as_str()) {
            return self.fail(
                &t,
                ErrorKind::ReservedWord { word: text.clone() },
                "a name that is not a keyword",
            );
        }
        if let Some((prefix, rest)) = text.split_once("::") {
            let plain =
                !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if prefix != "LOCAL" || !plain {
                return self.fail(
                    &t,
                    ErrorKind::UnknownNamespace { name: text.clone() },
                    "a plain name, an attribute path, or `LOCAL::NAME`",
                );
            }
        }
        self.bump();
        Ok(Name {
            text: text.clone(),
            span: t.span,
        })
    }

    fn cond(&mut self) -> Res<Cond> {
        let t = self.peek()?.clone();
        match &t.tok {
            Tok::Number(n) => self.fail(
                &t,
                ErrorKind::NumberInCondition { text: n.clone() },
                "a name, alone or compared with `==` to a string",
            ),
            Tok::Name(_) => {
                if matches!(self.peek_at(1)?.tok, Tok::LParen) {
                    let n = word(&t).unwrap_or_default().to_string();
                    return self.fail(
                        &t,
                        ErrorKind::CallInExpression { name: n },
                        "a name, alone or compared with `==` to a string",
                    );
                }
                let left = self.name()?;
                let next = self.peek()?.clone();
                match &next.tok {
                    Tok::EqEq => {
                        self.bump();
                        let r = self.peek()?.clone();
                        match &r.tok {
                            Tok::Str(s) => {
                                self.bump();
                                let span = Span::join(left.span, r.span);
                                Ok(Cond::Eq {
                                    left,
                                    right: StrLit {
                                        value: s.clone(),
                                        span: r.span,
                                    },
                                    span,
                                })
                            }
                            Tok::Number(n) => self.fail(
                                &r,
                                ErrorKind::NumberInCondition { text: n.clone() },
                                "a string literal after `==`",
                            ),
                            Tok::Name(_) if matches!(self.peek_at(1)?.tok, Tok::LParen) => {
                                let n = word(&r).unwrap_or_default().to_string();
                                self.fail(
                                    &r,
                                    ErrorKind::CallInExpression { name: n },
                                    "a string literal after `==`",
                                )
                            }
                            _ => self.unexpected(&r, "a string literal after `==`"),
                        }
                    }
                    Tok::Operator(op) => self.fail(
                        &next,
                        ErrorKind::UnsupportedOperator { op: op.clone() },
                        "`==` (the only documented comparison)",
                    ),
                    Tok::Eq => self.fail(
                        &next,
                        ErrorKind::UnsupportedOperator {
                            op: "=".to_string(),
                        },
                        "`==` (the only documented comparison)",
                    ),
                    _ => {
                        let span = left.span;
                        Ok(Cond::Bare { name: left, span })
                    }
                }
            }
            _ => self.unexpected(
                &t,
                "a condition: a name, alone or compared with `==` to a string",
            ),
        }
    }

    // ── statements ───────────────────────────────────────────────────

    fn stmt(&mut self, in_branch: bool) -> Res<Stmt> {
        let t = self.peek()?.clone();
        let Tok::Name(w) = &t.tok else {
            return self.unexpected(&t, "a statement");
        };
        match w.as_str() {
            "if" => {
                if in_branch {
                    return self.fail(
                        &t,
                        ErrorKind::NestedIf,
                        "a statement (an `if` inside an `if` is not documented)",
                    );
                }
                return self.if_stmt();
            }
            "then" | "else" | "end" => {
                return self.fail(
                    &t,
                    ErrorKind::StrayBlockWord { word: w.clone() },
                    "a statement (no `if` is open here)",
                );
            }
            _ => {}
        }
        match self.peek_at(1)?.tok.clone() {
            Tok::LParen => match w.as_str() {
                "action" => Ok(Stmt::Action(self.action_call(Vec::new(), t.span.start)?)),
                "log" => self.log_stmt(),
                "delete" => self.delete_stmt(),
                _ => self.fail(
                    &t,
                    ErrorKind::UnknownFunction { name: w.clone() },
                    "`action`, `log` or `delete`",
                ),
            },
            Tok::Eq => self.assign_stmt(),
            Tok::Colon => self.binding_stmt(),
            Tok::EqEq | Tok::Operator(_) => {
                let n = self.peek_at(1)?.clone();
                let op = match &n.tok {
                    Tok::Operator(o) => o.clone(),
                    _ => "==".to_string(),
                };
                self.fail(
                    &n,
                    ErrorKind::UnsupportedOperator { op },
                    "`=` (assignment), `:` (result binding) or `(` (call)",
                )
            }
            _ => {
                let n = self.peek_at(1)?.clone();
                self.unexpected(&n, "`=` (assignment), `:` (result binding) or `(` (call)")
            }
        }
    }

    fn assign_stmt(&mut self) -> Res<Stmt> {
        let target = self.name()?;
        self.bump(); // `=`
        let v = self.peek()?.clone();
        let value = match &v.tok {
            Tok::Number(n) => Value::Number(Number {
                text: n.clone(),
                span: v.span,
            }),
            Tok::Str(s) => Value::Str(StrLit {
                value: s.clone(),
                span: v.span,
            }),
            Tok::Name(n) if matches!(self.peek_at(1)?.tok, Tok::LParen) => {
                return self.fail(
                    &v,
                    ErrorKind::CallInExpression { name: n.clone() },
                    "a number or a string (a call result is bound with `field: LOCAL::NAME = action(...)`)",
                );
            }
            _ => return self.unexpected(&v, "a number or a string"),
        };
        self.bump();
        let span = Span {
            start: target.span.start,
            end: v.span.end,
        };
        Ok(Stmt::Assign {
            target,
            value,
            span,
        })
    }

    fn plain_ident(&mut self) -> Res<Ident> {
        let t = self.peek()?.clone();
        let Tok::Name(text) = &t.tok else {
            return self.unexpected(&t, "a name");
        };
        if RESERVED.contains(&text.as_str()) {
            return self.fail(
                &t,
                ErrorKind::ReservedWord { word: text.clone() },
                "a plain name",
            );
        }
        if text.contains('/') || text.contains("::") {
            return self.fail(
                &t,
                ErrorKind::UnknownNamespace { name: text.clone() },
                "a plain name (no `/` and no `::`)",
            );
        }
        self.bump();
        Ok(Ident {
            text: text.clone(),
            span: t.span,
        })
    }

    fn binding_stmt(&mut self) -> Res<Stmt> {
        let start = self.peek()?.span.start;
        let mut bindings: Vec<Binding> = Vec::new();
        loop {
            let field = self.plain_ident()?;
            let c = self.peek()?.clone();
            if !matches!(c.tok, Tok::Colon) {
                return self.unexpected(&c, "`:` after a result field");
            }
            self.bump();
            let at = self.peek()?.clone();
            let local = self.name()?;
            if !local.text.starts_with("LOCAL::") {
                return self.fail(
                    &at,
                    ErrorKind::UnknownNamespace {
                        name: local.text.clone(),
                    },
                    "`LOCAL::NAME` (the only documented binding target)",
                );
            }
            bindings.push(Binding { field, local });
            let n = self.peek()?.clone();
            match &n.tok {
                Tok::Comma => {
                    self.bump();
                    self.skip_nl();
                }
                Tok::Eq => {
                    self.bump();
                    break;
                }
                _ => return self.unexpected(&n, "`,` or `=` after a result binding"),
            }
        }
        let t = self.peek()?.clone();
        let is_call = matches!(self.peek_at(1)?.tok, Tok::LParen);
        match word(&t) {
            Some("action") if is_call => Ok(Stmt::Action(self.action_call(bindings, start)?)),
            Some(other) if is_call => self.fail(
                &t,
                ErrorKind::UnknownFunction {
                    name: other.to_string(),
                },
                "`action(` (only `action` returns results to bind)",
            ),
            _ => self.unexpected(&t, "`action(` after the result bindings"),
        }
    }

    /// At `action` `(`.
    fn action_call(&mut self, bindings: Vec<Binding>, start: Pos) -> Res<ActionCall> {
        let head = self.bump();
        self.bump(); // `(`
        let mut positional: Option<StrLit> = None;
        let mut named: Vec<NamedArg> = Vec::new();
        let mut first = true;
        let mut after_comma = false;
        let end;
        loop {
            self.skip_nl();
            let t = self.peek()?.clone();
            match &t.tok {
                Tok::RParen => {
                    if after_comma {
                        return self.unexpected(&t, "an argument after `,` (no trailing comma)");
                    }
                    if first {
                        return self.fail(
                            &t,
                            ErrorKind::MissingCapability,
                            "a capability: a first string or `capability: \"...\"`",
                        );
                    }
                    end = t.span.end;
                    self.bump();
                    break;
                }
                Tok::Str(s) if first => {
                    positional = Some(StrLit {
                        value: s.clone(),
                        span: t.span,
                    });
                    self.bump();
                }
                Tok::Str(_) => {
                    return self.unexpected(
                        &t,
                        "`name: \"value\"` (only the first argument may be positional)",
                    );
                }
                Tok::Name(n) => {
                    let next = self.peek_at(1)?.clone();
                    match &next.tok {
                        Tok::LParen => {
                            return self.fail(
                                &t,
                                ErrorKind::NestedCall { name: n.clone() },
                                "`name: \"value\"` (calls do not nest)",
                            );
                        }
                        Tok::Colon => {}
                        _ => return self.unexpected(&next, "`:` after the argument name"),
                    }
                    let name = self.plain_ident()?;
                    self.bump(); // `:`
                    let v = self.peek()?.clone();
                    match &v.tok {
                        Tok::Str(s) => {
                            self.bump();
                            if named.iter().any(|a| a.name.text == name.text) {
                                return self.fail(
                                    &t,
                                    ErrorKind::DuplicateArgument { name: name.text },
                                    "each argument once",
                                );
                            }
                            named.push(NamedArg {
                                name,
                                value: StrLit {
                                    value: s.clone(),
                                    span: v.span,
                                },
                            });
                        }
                        Tok::Name(inner) if matches!(self.peek_at(1)?.tok, Tok::LParen) => {
                            return self.fail(
                                &v,
                                ErrorKind::NestedCall {
                                    name: inner.clone(),
                                },
                                "a string literal (calls do not nest)",
                            );
                        }
                        _ => return self.unexpected(&v, "a string literal as the argument value"),
                    }
                }
                _ => return self.unexpected(&t, "an argument or `)`"),
            }
            first = false;
            after_comma = false;
            self.skip_nl();
            let n = self.peek()?.clone();
            match &n.tok {
                Tok::Comma => {
                    self.bump();
                    after_comma = true;
                }
                Tok::RParen => {}
                _ => return self.unexpected(&n, "`,` or `)`"),
            }
        }
        let (capability, form) = match (
            positional,
            named.iter().position(|a| a.name.text == "capability"),
        ) {
            (Some(_), Some(_)) => {
                return self.fail(&head, ErrorKind::DuplicateCapability, "one capability");
            }
            (Some(c), None) => (c, CapabilityForm::Positional),
            (None, Some(i)) => (named.remove(i).value, CapabilityForm::Named),
            (None, None) => {
                return self.fail(
                    &head,
                    ErrorKind::MissingCapability,
                    "a capability: a first string or `capability: \"...\"`",
                );
            }
        };
        if capability.value.is_empty() {
            return self.fail(
                &head,
                ErrorKind::EmptyCapability,
                "a non-empty capability name",
            );
        }
        Ok(ActionCall {
            bindings,
            capability,
            capability_form: form,
            args: named,
            span: Span { start, end },
        })
    }

    fn log_stmt(&mut self) -> Res<Stmt> {
        let head = self.bump();
        self.bump(); // `(`
        let v = self.peek()?.clone();
        let message = match &v.tok {
            Tok::Str(s) => StrLit {
                value: s.clone(),
                span: v.span,
            },
            Tok::Name(n) if matches!(self.peek_at(1)?.tok, Tok::LParen) => {
                return self.fail(
                    &v,
                    ErrorKind::NestedCall { name: n.clone() },
                    "one string literal (calls do not nest)",
                );
            }
            _ => return self.unexpected(&v, "one string literal"),
        };
        self.bump();
        let c = self.peek()?.clone();
        if !matches!(c.tok, Tok::RParen) {
            return self.unexpected(&c, "`)` (`log` takes exactly one string)");
        }
        self.bump();
        Ok(Stmt::Log {
            message,
            span: Span::join(head.span, c.span),
        })
    }

    fn delete_stmt(&mut self) -> Res<Stmt> {
        let head = self.bump();
        self.bump(); // `(`
        let v = self.peek()?.clone();
        if let Tok::Name(n) = &v.tok
            && matches!(self.peek_at(1)?.tok, Tok::LParen)
        {
            return self.fail(
                &v,
                ErrorKind::NestedCall { name: n.clone() },
                "a name (calls do not nest)",
            );
        }
        let target = self.name()?;
        let c = self.peek()?.clone();
        if !matches!(c.tok, Tok::RParen) {
            return self.unexpected(&c, "`)` (`delete` takes exactly one name)");
        }
        self.bump();
        Ok(Stmt::Delete {
            target,
            span: Span::join(head.span, c.span),
        })
    }

    fn if_stmt(&mut self) -> Res<Stmt> {
        let kw = self.bump();
        let cond = self.cond()?;
        let t = self.peek()?.clone();
        if word(&t) != Some("then") {
            return self.unexpected(&t, "`then`");
        }
        self.bump();
        let (then_branch, stop) = self.block(&kw)?;
        if stop != "else" {
            let t = self.toks[self.i - 1].clone();
            return self.fail(
                &t,
                ErrorKind::MissingElse,
                "`else` (the documented form always has one)",
            );
        }
        let (else_branch, stop2) = self.block(&kw)?;
        let end_tok = self.toks[self.i - 1].clone();
        if stop2 != "end" {
            return self.fail(&end_tok, ErrorKind::StrayBlockWord { word: stop2 }, "`end`");
        }
        Ok(Stmt::If(IfStmt {
            cond,
            then_branch,
            else_branch,
            span: Span::join(kw.span, end_tok.span),
        }))
    }

    /// Statements up to `else` or `end` (consumed); returns which one stopped it.
    fn block(&mut self, if_kw: &Token) -> Res<(Vec<Stmt>, String)> {
        let mut stmts: Vec<Stmt> = Vec::new();
        loop {
            self.skip_nl();
            let t = self.peek()?.clone();
            match &t.tok {
                Tok::Eof => {
                    return self.fail(if_kw, ErrorKind::UnterminatedIf, "`end` to close the `if`");
                }
                Tok::Name(w) if w == "else" || w == "end" => {
                    if stmts.is_empty() {
                        return self.fail(
                            &t,
                            ErrorKind::EmptyBranch,
                            "at least one statement in each branch",
                        );
                    }
                    let w = w.clone();
                    // The caller ends the statement after `end` (end of line).
                    self.bump();
                    return Ok((stmts, w));
                }
                Tok::Name(w) if t.first_on_line && CLAUSES.contains(&w.as_str()) => {
                    return self.fail(if_kw, ErrorKind::UnterminatedIf, "`end` to close the `if`");
                }
                _ => {
                    stmts.push(self.stmt(true)?);
                    self.expect_eol()?;
                }
            }
        }
    }
}
