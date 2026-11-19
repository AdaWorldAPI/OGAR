//! The hand-written parser for the language the HIRO User Guides 7.0
//! document. See the crate README for what is read and what is refused.

use crate::ast::{
    ActionCall, ArgValue, ArithOp, Binding, Block, BlockKind, CallArg, CallStmt, CapabilityForm,
    CmpOp, Comparison, Cond, Direction, DoClause, ExactNeSpelling, Expr, GraphScope,
    GraphScopeKind, Header, Ident, IfStmt, Key, KnowledgeItem, Leaf, LogLevel, Name, NamedArg,
    Number, OnClause, OnItem, Pos, RegexLit, RelatedBlock, Span, Stmt, StrLit, Tier, VarRef,
    WhenClause,
};
use crate::error::{ErrorKind, ParseError};
use crate::lexer::{Tok, Token, lex};

type Res<T> = Result<T, ParseError>;

/// The deepest nesting the reader descends into: `not`, parentheses, list
/// literals and `begin ... end` blocks together. The parser is recursive, so
/// without a bound a short text (10,000 times `not `) would overflow the stack
/// and abort the process. 64 is far beyond anything in the guide (its deepest
/// item nests twice) and keeps the recursion shallow enough for a 2 MB thread
/// stack in an unoptimised build, where each level costs several frames.
/// Nesting of `if` is bounded separately (the guide shows none inside an
/// `if`), nested calls are refused, and keys do not recurse.
pub const MAX_NESTING: usize = 64;

const CLAUSES: [&str; 4] = ["ki", "on", "when", "do"];
const RESERVED: [&str; 14] = [
    "ki", "on", "when", "do", "if", "then", "else", "end", "not", "and", "or", "as", "with",
    "begin",
];

/// Parse one Knowledge Item.
///
/// # Errors
///
/// A [`ParseError`] naming the first construct outside the documented
/// language, with its line and column and what was expected. Never panics, on
/// any input.
pub fn parse(src: &str) -> Res<KnowledgeItem> {
    let mut p = Parser {
        toks: lex(src),
        i: 0,
        depth: 0,
    };
    let mut item = p.item()?;
    item.source = src.to_string();
    Ok(item)
}

struct Parser {
    toks: Vec<Token>,
    i: usize,
    /// Current nesting of `not`, parentheses, list literals and `begin` blocks.
    depth: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CondCtx {
    /// A line of `on`, `when` or a block.
    Match,
    /// The condition of an `if`.
    If,
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
        Tok::Str(s, _) => format!("string \"{s}\""),
        Tok::Number(n) => format!("number `{n}`"),
        Tok::Regex(p, f) => format!("regex /{p}/{f}"),
        Tok::Op(o) => format!("operator `{o}`"),
        Tok::Colon => "`:`".to_string(),
        Tok::Comma => "`,`".to_string(),
        Tok::LParen => "`(`".to_string(),
        Tok::RParen => "`)`".to_string(),
        Tok::LBracket => "`[`".to_string(),
        Tok::RBracket => "`]`".to_string(),
        Tok::Dot => "`.`".to_string(),
        Tok::At => "`@`".to_string(),
        Tok::Newline => "end of line".to_string(),
        Tok::Bad(_) => "invalid input".to_string(),
        Tok::Eof => "end of input".to_string(),
    }
}

fn cmp_op(op: &str) -> Option<CmpOp> {
    Some(match op {
        "==" => CmpOp::Eq,
        "!=" => CmpOp::Ne,
        "===" => CmpOp::ExactEq,
        "!===" => CmpOp::ExactNe(ExactNeSpelling::Table),
        "!==" => CmpOp::ExactNe(ExactNeSpelling::Example),
        ">=" => CmpOp::Ge,
        "<=" => CmpOp::Le,
        ">" => CmpOp::Gt,
        "<" => CmpOp::Lt,
        "=~" => CmpOp::Match,
        "!~" => CmpOp::NotMatch,
        _ => return None,
    })
}

fn arith_op(op: &str) -> Option<ArithOp> {
    Some(match op {
        "+" => ArithOp::Add,
        "-" => ArithOp::Sub,
        "*" => ArithOp::Mul,
        "/" => ArithOp::Div,
        "%" => ArithOp::Mod,
        _ => return None,
    })
}

fn plain_chars(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
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

    /// The token kind at offset `n`, with a lexical fault treated as "other".
    fn kind_at(&self, n: usize) -> &Tok {
        &self.toks[(self.i + n).min(self.toks.len() - 1)].tok
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

    fn not_read<T>(&self, t: &Token, construct: &str, expected: &str) -> Res<T> {
        self.fail(
            t,
            ErrorKind::DocumentedNotRead {
                construct: construct.to_string(),
            },
            expected,
        )
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
        let mut h = Header {
            name: None,
            description: None,
            version: None,
            tier_text: None,
            tier: None,
            id: None,
            span: kw.span,
        };
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
            if !matches!(self.kind_at(1), Tok::Colon) {
                break;
            }
            let slot = match key.as_str() {
                "name" => &mut h.name,
                "description" => &mut h.description,
                "version" => &mut h.version,
                "tier" => &mut h.tier_text,
                "id" => &mut h.id,
                _ => {
                    return self.fail(
                        &t,
                        ErrorKind::UnknownHeaderKey { key },
                        "`name:`, `description:`, `version:`, `tier:` or `id:`",
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
            let Tok::Str(s, q) = &v.tok else {
                return self.unexpected(&v, "a string literal");
            };
            *slot = Some(StrLit {
                value: s.clone(),
                span: v.span,
                quote: *q,
            });
            if key == "tier" {
                h.tier = Some(match s.as_str() {
                    "default" => Tier::Default,
                    "policy" => Tier::Policy,
                    "fallback" => Tier::Fallback,
                    other => {
                        return self.fail(
                            &v,
                            ErrorKind::UnknownTier {
                                value: other.to_string(),
                            },
                            "\"default\", \"policy\" or \"fallback\"",
                        );
                    }
                });
            }
            end = v.span.end;
            self.bump();
            self.expect_eol()?;
        }
        h.span = Span {
            start: kw.span.start,
            end,
        };
        Ok(h)
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

    /// `with <Name>` or `count <Name>` at the current token.
    fn block_start(&self) -> Option<BlockKind> {
        let Tok::Name(w) = self.kind_at(0) else {
            return None;
        };
        let kind = match w.as_str() {
            "with" => BlockKind::With,
            "count" => BlockKind::Count,
            _ => return None,
        };
        // `count` is also a plain variable name, so it starts a block only as
        // `count <type>@<direction>`; `count and Ready` is a condition. `with`
        // is reserved, so it always starts a block when a name follows.
        (matches!(self.kind_at(1), Tok::Name(n) if !RESERVED.contains(&n.as_str()))
            && (kind == BlockKind::With || matches!(self.kind_at(2), Tok::At)))
        .then_some(kind)
    }

    fn on_clause(&mut self) -> Res<OnClause> {
        let kw = self.expect_clause("on")?;
        self.body_not_empty("on")?;
        let mut items: Vec<OnItem> = Vec::new();
        loop {
            self.skip_nl();
            let t = self.peek()?.clone();
            if matches!(t.tok, Tok::Eof) {
                break;
            }
            if self.line_start_check(&t)? {
                break;
            }
            if let Some(kind) = self.block_start() {
                items.push(OnItem::Block(self.related_block(kind)?));
            } else {
                items.push(OnItem::Cond(self.or_expr(CondCtx::Match)?));
                self.expect_eol()?;
            }
        }
        let end = items.last().map_or(kw.span.end, |i| i.span().end);
        Ok(OnClause {
            items,
            span: Span {
                start: kw.span.start,
                end,
            },
        })
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
            if let Some(kind) = self.block_start() {
                return self.fail(
                    &t,
                    ErrorKind::BlockOutsideOn { word: kind.word() },
                    "a condition (`with` and `count` blocks are shown in the `on` section)",
                );
            }
            conds.push(self.or_expr(CondCtx::Match)?);
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

    // ── related node matching ────────────────────────────────────────

    /// `with|count <vertexType>@<dir>[:<edgeType>] [as Alias]` lines `end`.
    fn related_block(&mut self, kind: BlockKind) -> Res<RelatedBlock> {
        let kw = self.bump();
        let word = kind.word();
        let vt = self.peek()?.clone();
        let Tok::Name(vt_text) = &vt.tok else {
            return self.unexpected(&vt, "a vertex type");
        };
        if RESERVED.contains(&vt_text.as_str()) || vt_text.contains("::") {
            return self.fail(
                &vt,
                ErrorKind::ReservedWord {
                    word: vt_text.clone(),
                },
                "a vertex type",
            );
        }
        self.bump();
        let vertex_type = Name {
            text: vt_text.clone(),
            span: vt.span,
        };
        let at = self.peek()?.clone();
        if !matches!(at.tok, Tok::At) {
            return self.unexpected(&at, "`@` and an edge direction (`in`, `out`, `connected`)");
        }
        self.bump();
        let d = self.peek()?.clone();
        let Tok::Name(dir_text) = &d.tok else {
            return self.unexpected(&d, "an edge direction (`in`, `out`, `connected`)");
        };
        let direction = match dir_text.as_str() {
            "in" => Direction::In,
            "out" => Direction::Out,
            "connected" => Direction::Connected,
            "parent" => Direction::ExampleOnlyParent,
            "child" => Direction::ExampleOnlyChild,
            other => {
                return self.fail(
                    &d,
                    ErrorKind::UnknownDirection {
                        found: other.to_string(),
                    },
                    "`in`, `out` or `connected`",
                );
            }
        };
        self.bump();
        let mut edge_type = None;
        if matches!(self.peek()?.tok, Tok::Colon) {
            self.bump();
            let e = self.peek()?.clone();
            let Tok::Name(et) = &e.tok else {
                return self.unexpected(&e, "an edge type after `:`");
            };
            if RESERVED.contains(&et.as_str()) || et.contains("::") {
                return self.fail(
                    &e,
                    ErrorKind::ReservedWord { word: et.clone() },
                    "an edge type",
                );
            }
            self.bump();
            edge_type = Some(Name {
                text: et.clone(),
                span: e.span,
            });
        }
        let mut alias = None;
        if word_is(self.peek()?, "as") {
            self.bump();
            alias = Some(self.plain_ident()?);
        }
        if kind == BlockKind::Count && alias.is_none() {
            return self.fail(
                &kw,
                ErrorKind::CountWithoutAlias,
                "`as <Name>` after the `count` header",
            );
        }
        self.expect_eol()?;
        let mut conds: Vec<Cond> = Vec::new();
        let end_tok;
        loop {
            self.skip_nl();
            let t = self.peek()?.clone();
            match &t.tok {
                Tok::Eof => {
                    return self.fail(
                        &kw,
                        ErrorKind::UnterminatedBlock { word },
                        &format!("`end` to close the `{word}` block"),
                    );
                }
                Tok::Name(w) if w == "end" => {
                    end_tok = self.bump();
                    break;
                }
                Tok::Name(w) if t.first_on_line && CLAUSES.contains(&w.as_str()) => {
                    return self.fail(
                        &kw,
                        ErrorKind::UnterminatedBlock { word },
                        &format!("`end` to close the `{word}` block"),
                    );
                }
                _ => {}
            }
            self.line_start_check(&t)?;
            if let Some(inner) = self.block_start() {
                return self.fail(
                    &t,
                    ErrorKind::NestedBlock {
                        outer: word,
                        inner: inner.word(),
                    },
                    "a condition (the guide shows no nested `with` or `count` block)",
                );
            }
            conds.push(self.or_expr(CondCtx::Match)?);
            self.expect_eol()?;
        }
        if conds.is_empty() {
            return self.fail(
                &end_tok,
                ErrorKind::EmptyClause { clause: word },
                "at least one condition before `end`",
            );
        }
        self.expect_eol()?;
        Ok(RelatedBlock {
            kind,
            vertex_type,
            direction,
            direction_span: d.span,
            edge_type,
            alias,
            conds,
            span: Span::join(kw.span, end_tok.span),
        })
    }

    // ── names ────────────────────────────────────────────────────────

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
            let rest_ok = !rest.is_empty()
                && !rest.contains("::")
                && rest
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '/');
            if !plain_chars(prefix) || !rest_ok {
                return self.fail(
                    &t,
                    ErrorKind::UnknownNamespace { name: text.clone() },
                    "a plain name, an attribute path, or `SCOPE::name` with SCOPE one of NODE, ISSUE, LOCAL or a block alias",
                );
            }
        }
        self.bump();
        Ok(Name {
            text: text.clone(),
            span: t.span,
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

    /// `Name`, `Name["key"]`, `Name[1]`, `Name[Other]`, each with an optional
    /// `@any` or `@self` after it.
    fn var_ref(&mut self) -> Res<VarRef> {
        let name = self.name()?;
        let mut end = name.span.end;
        let mut key = None;
        if matches!(self.peek()?.tok, Tok::LBracket) {
            self.bump();
            let k = self.peek()?.clone();
            key = Some(match &k.tok {
                Tok::Str(s, q) => {
                    self.bump();
                    Key::Str(StrLit {
                        value: s.clone(),
                        span: k.span,
                        quote: *q,
                    })
                }
                Tok::Number(n) => {
                    self.bump();
                    Key::Number(Number {
                        text: n.clone(),
                        span: k.span,
                    })
                }
                Tok::Name(_) => Key::Var(self.name()?),
                Tok::RBracket => {
                    return self.not_read(
                        &k,
                        "empty key `[]` (copy a list with its keys)",
                        "a key: a string, a number or a variable",
                    );
                }
                _ => return self.unexpected(&k, "a key: a string, a number or a variable"),
            });
            let c = self.peek()?.clone();
            if !matches!(c.tok, Tok::RBracket) {
                return self.unexpected(&c, "`]`");
            }
            end = c.span.end;
            self.bump();
        }
        let mut graph = None;
        if matches!(self.peek()?.tok, Tok::At) {
            let at = self.bump();
            let g = self.peek()?.clone();
            let Tok::Name(gname) = &g.tok else {
                return self.unexpected(&g, "`any` or `self` after `@`");
            };
            let kind = match gname.as_str() {
                "any" => GraphScopeKind::Any,
                "self" => GraphScopeKind::SelfOnly,
                "in" => GraphScopeKind::InAsWrittenInExample,
                other => {
                    return self.fail(
                        &g,
                        ErrorKind::UnknownGraphScope {
                            found: other.to_string(),
                        },
                        "`@any` or `@self`",
                    );
                }
            };
            self.bump();
            end = g.span.end;
            graph = Some(GraphScope {
                kind,
                span: Span::join(at.span, g.span),
            });
        }
        let span = Span {
            start: name.span.start,
            end,
        };
        Ok(VarRef {
            name,
            key,
            graph,
            span,
        })
    }

    // ── expressions ──────────────────────────────────────────────────

    fn operand(&mut self, in_arg: bool) -> Res<Expr> {
        let t = self.peek()?.clone();
        match &t.tok {
            Tok::Str(s, q) => {
                self.bump();
                Ok(Expr::Str(StrLit {
                    value: s.clone(),
                    span: t.span,
                    quote: *q,
                }))
            }
            Tok::Number(n) => {
                self.bump();
                Ok(Expr::Number(Number {
                    text: n.clone(),
                    span: t.span,
                }))
            }
            Tok::Regex(pattern, flags) => {
                self.bump();
                Ok(Expr::Regex(RegexLit {
                    pattern: pattern.clone(),
                    flags: flags.clone(),
                    span: t.span,
                }))
            }
            Tok::LBracket => {
                self.bump();
                let mut items = Vec::new();
                self.enter(&t)?;
                loop {
                    items.push(self.operand(in_arg)?);
                    let n = self.peek()?.clone();
                    match &n.tok {
                        Tok::Comma => {
                            self.bump();
                        }
                        Tok::RBracket => {
                            self.bump();
                            self.depth -= 1;
                            let span = Span::join(t.span, n.span);
                            return Ok(Expr::List { items, span });
                        }
                        _ => return self.unexpected(&n, "`,` or `]`"),
                    }
                }
            }
            Tok::Name(n) => {
                if matches!(self.kind_at(1), Tok::LParen) {
                    let kind = if in_arg {
                        ErrorKind::NestedCall { name: n.clone() }
                    } else {
                        ErrorKind::CallInExpression { name: n.clone() }
                    };
                    return self.fail(&t, kind, "a value: a string, a number or a variable");
                }
                Ok(Expr::Var(self.var_ref()?))
            }
            _ => self.unexpected(&t, "a value: a string, a number or a variable"),
        }
    }

    /// An operand, or one `operand op operand`. A second operator is refused.
    fn expr(&mut self, in_arg: bool) -> Res<Expr> {
        let left = self.operand(in_arg)?;
        let t = self.peek()?.clone();
        let Tok::Op(o) = &t.tok else {
            return Ok(left);
        };
        let Some(op) = arith_op(o) else {
            return Ok(left);
        };
        self.bump();
        let right = self.operand(in_arg)?;
        let n = self.peek()?.clone();
        if let Tok::Op(o2) = &n.tok
            && arith_op(o2).is_some()
        {
            return self.fail(
                &n,
                ErrorKind::ChainedArithmetic,
                "one operator per expression (the guide shows `a + b`, not chains)",
            );
        }
        let span = Span::join(left.span(), right.span());
        Ok(Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
            span,
        })
    }

    // ── conditions ───────────────────────────────────────────────────

    /// `a or b`, `a and b`, with `not` over the next expression. `and` and
    /// `or` on one level are not mixed: the guide gives no order between them.
    fn or_expr(&mut self, ctx: CondCtx) -> Res<Cond> {
        let first = self.not_expr(ctx)?;
        let mut parts = vec![first];
        let mut seen: Option<String> = None;
        loop {
            let t = self.peek()?.clone();
            let Tok::Name(w) = &t.tok else { break };
            if w != "and" && w != "or" {
                break;
            }
            if let Some(s) = &seen
                && s != w
            {
                return self.fail(
                    &t,
                    ErrorKind::AmbiguousConnectives {
                        first: s.clone(),
                        second: w.clone(),
                    },
                    "parentheses around one of them (the guide states `not` over `or` and no order between `and` and `or`)",
                );
            }
            seen = Some(w.clone());
            self.bump();
            parts.push(self.not_expr(ctx)?);
        }
        let Some(w) = seen else {
            return Ok(parts.remove(0));
        };
        let span = Span::join(
            parts[0].span(),
            parts.last().map_or(parts[0].span(), Cond::span),
        );
        Ok(if w == "and" {
            Cond::And { parts, span }
        } else {
            Cond::Or { parts, span }
        })
    }

    fn enter(&mut self, t: &Token) -> Res<()> {
        if self.depth >= MAX_NESTING {
            return self.fail(
                t,
                ErrorKind::NestingTooDeep { limit: MAX_NESTING },
                "a shallower item",
            );
        }
        self.depth += 1;
        Ok(())
    }

    fn not_expr(&mut self, ctx: CondCtx) -> Res<Cond> {
        let t = self.peek()?.clone();
        let nests = word(&t) == Some("not") || matches!(t.tok, Tok::LParen);
        if nests {
            self.enter(&t)?;
        }
        let r = self.not_expr_inner(ctx, t);
        if nests {
            self.depth -= 1;
        }
        r
    }

    fn not_expr_inner(&mut self, ctx: CondCtx, t: Token) -> Res<Cond> {
        if word(&t) == Some("not") {
            self.bump();
            let inner = self.not_expr(ctx)?;
            let span = Span::join(t.span, inner.span());
            return Ok(Cond::Not {
                inner: Box::new(inner),
                span,
            });
        }
        if matches!(t.tok, Tok::LParen) {
            self.bump();
            let inner = self.or_expr(ctx)?;
            let c = self.peek()?.clone();
            if !matches!(c.tok, Tok::RParen) {
                return self.unexpected(&c, "`)`");
            }
            self.bump();
            return Ok(Cond::Group {
                inner: Box::new(inner),
                span: Span::join(t.span, c.span),
            });
        }
        Ok(Cond::Leaf(Box::new(self.leaf(ctx)?)))
    }

    fn leaf(&mut self, ctx: CondCtx) -> Res<Leaf> {
        let first = self.peek()?.clone();
        let mut optional = false;
        if matches!(&first.tok, Tok::Op(o) if o == "?") {
            if ctx == CondCtx::If {
                return self.fail(
                    &first,
                    ErrorKind::MatchFormInIf { form: "?" },
                    "a condition",
                );
            }
            self.bump();
            optional = true;
        }
        let mut list = false;
        let lt = self.peek()?.clone();
        if word(&lt) == Some("list")
            && matches!(self.kind_at(1), Tok::Name(n) if !matches!(n.as_str(), "as" | "and" | "or" | "has" | "in" | "then"))
        {
            if ctx == CondCtx::If {
                return self.fail(
                    &lt,
                    ErrorKind::MatchFormInIf { form: "list" },
                    "a condition",
                );
            }
            self.bump();
            list = true;
        }
        let t = self.peek()?.clone();
        match &t.tok {
            Tok::Number(n) => {
                return self.fail(
                    &t,
                    ErrorKind::NumberInCondition { text: n.clone() },
                    "a variable on the left (a name, alone or compared with an operator)",
                );
            }
            Tok::Str(..) => {
                return self.not_read(
                    &t,
                    "a constructed string on the left side of a match",
                    "a variable on the left",
                );
            }
            Tok::Name(n) => {
                let call = matches!(self.kind_at(1), Tok::LParen)
                    || (matches!(self.kind_at(1), Tok::Op(o) if o == "?")
                        && matches!(self.kind_at(2), Tok::LParen));
                if call {
                    return self.fail(
                        &t,
                        ErrorKind::CallInExpression { name: n.clone() },
                        "a variable on the left (function calls in a condition are documented but not read)",
                    );
                }
                if n == "alias" && word_is_at(self.kind_at(1), "as") {
                    return self.not_read(
                        &t,
                        "the multi variable `alias ... as Name ... end` block",
                        "a variable on the left",
                    );
                }
            }
            _ => return self.unexpected(&t, "a condition: a variable, `not` or `(`"),
        }
        let subject = self.var_ref()?;
        let mut end = subject.span.end;

        let mut test = None;
        let n = self.peek()?.clone();
        match &n.tok {
            Tok::Op(o) if o != ":=" => {
                let Some(op) = cmp_op(o) else {
                    return self.fail(
                        &n,
                        ErrorKind::UnsupportedOperator { op: o.clone() },
                        "a comparison operator from the matching page: `==`, `!=`, `===`, `!===`, `!==`, `>=`, `<=`, `>`, `<`, `=~`, `!~`",
                    );
                };
                self.bump();
                let r = self.peek()?.clone();
                let right = match &r.tok {
                    Tok::Regex(pattern, flags) if matches!(op, CmpOp::Match | CmpOp::NotMatch) => {
                        self.bump();
                        Expr::Regex(RegexLit {
                            pattern: pattern.clone(),
                            flags: flags.clone(),
                            span: r.span,
                        })
                    }
                    _ => self.expr(false)?,
                };
                end = right.span().end;
                test = Some(Comparison {
                    op,
                    op_span: n.span,
                    right,
                });
            }
            Tok::Name(w) if w == "has" => {
                return self.not_read(
                    &n,
                    "`has` (does a list contain values)",
                    "a comparison operator or the end of the line",
                );
            }
            Tok::Name(w) if w == "in" => {
                return self.not_read(
                    &n,
                    "`in [...]` (is a value in a list)",
                    "a comparison operator or the end of the line",
                );
            }
            _ => {}
        }

        let mut alias = None;
        let a = self.peek()?.clone();
        if word(&a) == Some("as") {
            if ctx == CondCtx::If {
                return self.fail(&a, ErrorKind::MatchFormInIf { form: "as" }, "`then`");
            }
            self.bump();
            let id = self.plain_ident()?;
            end = id.span.end;
            alias = Some(id);
        }

        let mut default = None;
        let d = self.peek()?.clone();
        if matches!(&d.tok, Tok::Op(o) if o == ":=") {
            if !optional {
                return self.unexpected(&d, "end of line (`:=` follows an optional `?` match)");
            }
            self.bump();
            let e = self.expr(false)?;
            end = e.span().end;
            default = Some(e);
        }
        if optional && default.is_none() && !list {
            return self.fail(
                &first,
                ErrorKind::OptionalWithoutDefault,
                "`:= <default>` after an optional match (only `?list` has none)",
            );
        }
        Ok(Leaf {
            optional,
            list,
            subject,
            test,
            alias,
            default,
            span: Span {
                start: first.span.start,
                end,
            },
        })
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
                        "a statement (an `if` inside an `if` is not shown in the guide)",
                    );
                }
                return self.if_stmt();
            }
            "then" | "else" | "end" => {
                return self.fail(
                    &t,
                    ErrorKind::StrayBlockWord { word: w.clone() },
                    "a statement (no block is open here)",
                );
            }
            _ => {}
        }
        let next = self.peek_at(1)?.clone();
        match &next.tok {
            Tok::LParen => self.call_stmt(Vec::new(), t.span.start, in_branch),
            Tok::Dot if w == "log" => self.call_stmt(Vec::new(), t.span.start, in_branch),
            Tok::Dot => self.unexpected(&next, "`(` (a `.` only follows `log`)"),
            Tok::Colon => self.tagged_stmt(in_branch),
            Tok::Op(o) if o == "=" => self.assign_stmt(),
            Tok::LBracket | Tok::At => self.assign_stmt(),
            Tok::Op(o) => self.fail(
                &next,
                ErrorKind::UnsupportedOperator { op: o.clone() },
                "`=` (assignment), `:` (result tag) or `(` (call)",
            ),
            _ => self.unexpected(&next, "`=` (assignment), `:` (result tag) or `(` (call)"),
        }
    }

    fn assign_stmt(&mut self) -> Res<Stmt> {
        let target = self.var_ref()?;
        let eq = self.peek()?.clone();
        match &eq.tok {
            Tok::Op(o) if o == "=" => {
                self.bump();
            }
            Tok::Op(o) => {
                return self.fail(&eq, ErrorKind::UnsupportedOperator { op: o.clone() }, "`=`");
            }
            _ => return self.unexpected(&eq, "`=`"),
        }
        let value = self.expr(false)?;
        let span = Span {
            start: target.span.start,
            end: value.span().end,
        };
        Ok(Stmt::Assign {
            target,
            value,
            span,
        })
    }

    /// `tag: Var, tag: Var = function(...)`.
    fn tagged_stmt(&mut self, in_branch: bool) -> Res<Stmt> {
        let start = self.peek()?.span.start;
        let mut bindings: Vec<Binding> = Vec::new();
        loop {
            let field = self.plain_ident()?;
            let c = self.peek()?.clone();
            if !matches!(c.tok, Tok::Colon) {
                return self.unexpected(&c, "`:` after a result tag");
            }
            self.bump();
            let target = self.name()?;
            bindings.push(Binding { field, target });
            let n = self.peek()?.clone();
            match &n.tok {
                Tok::Comma => {
                    self.bump();
                    self.skip_nl();
                }
                Tok::Op(o) if o == "=" => {
                    self.bump();
                    break;
                }
                _ => return self.unexpected(&n, "`,` or `=` after a result tag"),
            }
        }
        let t = self.peek()?.clone();
        let Tok::Name(fname) = &t.tok else {
            return self.unexpected(&t, "a function call after the result tags");
        };
        let call = matches!(self.kind_at(1), Tok::LParen)
            || (fname == "log" && matches!(self.kind_at(1), Tok::Dot));
        if !call {
            return self.fail(
                &t,
                ErrorKind::CallWithoutArguments {
                    function: fname.clone(),
                },
                "an argument list: `function(...)`",
            );
        }
        self.call_stmt(bindings, start, in_branch)
    }

    /// At `name` `(` or `log` `.` `level` `(`.
    fn call_stmt(&mut self, bindings: Vec<Binding>, start: Pos, in_branch: bool) -> Res<Stmt> {
        let head = self.bump();
        let fname = word(&head).unwrap_or_default().to_string();
        let mut level = LogLevel::Default;
        if fname == "log" && matches!(self.peek()?.tok, Tok::Dot) {
            self.bump();
            let l = self.peek()?.clone();
            let Tok::Name(ln) = &l.tok else {
                return self.unexpected(&l, "a log level: `debug`, `info`, `warn` or `error`");
            };
            level = match ln.as_str() {
                "debug" => LogLevel::Debug,
                "info" => LogLevel::Info,
                "warn" => LogLevel::Warn,
                "error" => LogLevel::Error,
                other => {
                    return self.fail(
                        &l,
                        ErrorKind::UnknownLogLevel {
                            level: other.to_string(),
                        },
                        "`debug`, `info`, `warn` or `error`",
                    );
                }
            };
            self.bump();
        }
        let lp = self.peek()?.clone();
        if !matches!(lp.tok, Tok::LParen) {
            return self.unexpected(&lp, "`(`");
        }
        self.bump();
        if !bindings.is_empty() && matches!(fname.as_str(), "log" | "delete" | "set" | "add") {
            return self.fail(
                &head,
                ErrorKind::BindingsOnCall { function: fname },
                "tags only in front of a function that returns values",
            );
        }
        match fname.as_str() {
            "log" => self.log_rest(level, &head),
            "delete" => self.delete_rest(&head),
            "set" | "add" => self.set_add_rest(&head, fname == "set"),
            _ => {
                let (args, rp) = self.call_args(in_branch)?;
                let span = Span {
                    start,
                    end: rp.span.end,
                };
                if fname == "action" {
                    return self.action_from(bindings, &head, args, span);
                }
                Ok(Stmt::Call(CallStmt {
                    bindings,
                    function: Ident {
                        text: fname,
                        span: head.span,
                    },
                    args,
                    span,
                }))
            }
        }
    }

    fn close_paren(&mut self, what: &str) -> Res<Token> {
        self.skip_nl();
        let c = self.peek()?.clone();
        if !matches!(c.tok, Tok::RParen) {
            return self.unexpected(&c, what);
        }
        Ok(self.bump())
    }

    fn log_rest(&mut self, level: LogLevel, head: &Token) -> Res<Stmt> {
        self.skip_nl();
        let message = self.expr(true)?;
        let c = self.close_paren("`)` (`log` takes exactly one argument)")?;
        Ok(Stmt::Log {
            level,
            message,
            span: Span::join(head.span, c.span),
        })
    }

    fn delete_rest(&mut self, head: &Token) -> Res<Stmt> {
        self.skip_nl();
        let v = self.peek()?.clone();
        if let Tok::Name(n) = &v.tok
            && matches!(self.kind_at(1), Tok::LParen)
        {
            return self.fail(
                &v,
                ErrorKind::NestedCall { name: n.clone() },
                "a variable (calls do not nest)",
            );
        }
        let target = self.var_ref()?;
        let mut created_on = None;
        self.skip_nl();
        if matches!(self.peek()?.tok, Tok::Comma) {
            self.bump();
            self.skip_nl();
            let k = self.peek()?.clone();
            if word(&k) != Some("created_on") || !matches!(self.kind_at(1), Tok::Colon) {
                return self.unexpected(&k, "`created_on: <node id>`");
            }
            self.bump();
            self.bump();
            created_on = Some(self.expr(true)?);
        }
        let c = self.close_paren("`)` (`delete` takes one variable and `created_on:`)")?;
        Ok(Stmt::Delete {
            target,
            created_on,
            span: Span::join(head.span, c.span),
        })
    }

    fn set_add_rest(&mut self, head: &Token, is_set: bool) -> Res<Stmt> {
        self.skip_nl();
        let target = self.var_ref()?;
        let eq = self.peek()?.clone();
        if !matches!(&eq.tok, Tok::Op(o) if o == "=") {
            return self.unexpected(&eq, "`=` (`set` and `add` take `Var = value`)");
        }
        self.bump();
        let value = self.expr(true)?;
        let c = self.close_paren("`)`")?;
        let span = Span::join(head.span, c.span);
        Ok(if is_set {
            Stmt::Set {
                target,
                value,
                span,
            }
        } else {
            Stmt::Add {
                target,
                value,
                span,
            }
        })
    }

    /// After `(`: positional and keyword arguments up to `)`.
    fn call_args(&mut self, in_branch: bool) -> Res<(Vec<CallArg>, Token)> {
        let mut args: Vec<CallArg> = Vec::new();
        let mut after_comma = false;
        loop {
            self.skip_nl();
            let t = self.peek()?.clone();
            if matches!(t.tok, Tok::RParen) {
                if after_comma {
                    return self.unexpected(&t, "an argument after `,` (no trailing comma)");
                }
                let rp = self.bump();
                return Ok((args, rp));
            }
            let keyword = matches!(t.tok, Tok::Name(_)) && matches!(self.kind_at(1), Tok::Colon);
            if keyword {
                let name = self.plain_ident()?;
                self.bump(); // `:`
                if args
                    .iter()
                    .any(|a| a.name.as_ref().is_some_and(|n| n.text == name.text))
                {
                    return self.fail(
                        &t,
                        ErrorKind::DuplicateArgument { name: name.text },
                        "each argument once",
                    );
                }
                let value = self.arg_value(in_branch)?;
                args.push(CallArg {
                    name: Some(name),
                    value,
                });
            } else {
                if args.iter().any(|a| a.name.is_some()) {
                    return self.fail(
                        &t,
                        ErrorKind::PositionalAfterKeyword,
                        "`name: value` (positional arguments come first)",
                    );
                }
                let value = self.arg_value(in_branch)?;
                args.push(CallArg { name: None, value });
            }
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
    }

    fn arg_value(&mut self, in_branch: bool) -> Res<ArgValue> {
        let t = self.peek()?.clone();
        let nests = word(&t) == Some("begin");
        if nests {
            self.enter(&t)?;
        }
        let r = self.arg_value_inner(in_branch, t);
        if nests {
            self.depth -= 1;
        }
        r
    }

    fn arg_value_inner(&mut self, in_branch: bool, t: Token) -> Res<ArgValue> {
        if word(&t) == Some("begin") {
            self.bump();
            let mut stmts: Vec<Stmt> = Vec::new();
            loop {
                self.skip_nl();
                let n = self.peek()?.clone();
                match &n.tok {
                    Tok::Eof => {
                        return self.fail(
                            &t,
                            ErrorKind::UnterminatedBlock { word: "begin" },
                            "`end` to close the `begin` block",
                        );
                    }
                    Tok::Name(w) if w == "end" => {
                        let e = self.bump();
                        return Ok(ArgValue::Block(Block {
                            stmts,
                            span: Span::join(t.span, e.span),
                        }));
                    }
                    Tok::Name(w) if n.first_on_line && CLAUSES.contains(&w.as_str()) => {
                        return self.fail(
                            &t,
                            ErrorKind::UnterminatedBlock { word: "begin" },
                            "`end` to close the `begin` block",
                        );
                    }
                    _ => {}
                }
                stmts.push(self.stmt(in_branch)?);
                // A statement ends at a newline, or right before `end`
                // (the guide writes `each: begin SUM = SUM + value end`).
                let e = self.peek()?.clone();
                match &e.tok {
                    Tok::Newline => {
                        self.bump();
                    }
                    Tok::Name(w) if w == "end" => {}
                    Tok::Eof => {}
                    _ => return self.unexpected(&e, "end of line or `end`"),
                }
            }
        }
        Ok(ArgValue::Expr(self.expr(true)?))
    }

    /// Turn a parsed `action(...)` call into an [`ActionCall`].
    fn action_from(
        &self,
        bindings: Vec<Binding>,
        head: &Token,
        args: Vec<CallArg>,
        span: Span,
    ) -> Res<Stmt> {
        let mut positional: Option<StrLit> = None;
        let mut named: Vec<NamedArg> = Vec::new();
        for (idx, a) in args.into_iter().enumerate() {
            match a.name {
                None => {
                    let at = a.value.span();
                    if idx > 0 {
                        return Err(ParseError {
                            kind: ErrorKind::PositionalAfterKeyword,
                            pos: at.start,
                            expected: "`name: value` (only the first argument may be positional)"
                                .to_string(),
                        });
                    }
                    match a.value {
                        ArgValue::Expr(Expr::Str(s)) => positional = Some(s),
                        other => {
                            return Err(ParseError {
                                kind: ErrorKind::UnexpectedToken {
                                    found: "a value that is not a string".to_string(),
                                },
                                pos: other.span().start,
                                expected: "a string: the capability".to_string(),
                            });
                        }
                    }
                }
                Some(name) => named.push(NamedArg {
                    name,
                    value: a.value,
                }),
            }
        }
        let cap_idx = named.iter().position(|a| a.name.text == "capability");
        let (capability, form) = match (positional, cap_idx) {
            (Some(_), Some(_)) => {
                return self.fail(head, ErrorKind::DuplicateCapability, "one capability");
            }
            (Some(c), None) => (c, CapabilityForm::Positional),
            (None, Some(i)) => {
                let a = named.remove(i);
                match a.value {
                    ArgValue::Expr(Expr::Str(s)) => (s, CapabilityForm::Named),
                    other => {
                        return Err(ParseError {
                            kind: ErrorKind::UnexpectedToken {
                                found: "a value that is not a string".to_string(),
                            },
                            pos: other.span().start,
                            expected: "a string: the capability".to_string(),
                        });
                    }
                }
            }
            (None, None) => {
                return self.fail(
                    head,
                    ErrorKind::MissingCapability,
                    "a capability: a first string or `capability: \"...\"`",
                );
            }
        };
        if capability.value.is_empty() {
            return self.fail(
                head,
                ErrorKind::EmptyCapability,
                "a non-empty capability name",
            );
        }
        Ok(Stmt::Action(ActionCall {
            bindings,
            capability,
            capability_form: form,
            args: named,
            span,
        }))
    }

    fn if_stmt(&mut self) -> Res<Stmt> {
        let kw = self.bump();
        let cond = self.or_expr(CondCtx::If)?;
        let t = self.peek()?.clone();
        if word(&t) != Some("then") {
            return self.unexpected(&t, "`then`");
        }
        self.bump();
        let (then_branch, stop) = self.block(&kw)?;
        let mut else_branch = None;
        let mut end_tok = self.toks[self.i - 1].clone();
        if stop == "else" {
            let (eb, stop2) = self.block(&kw)?;
            end_tok = self.toks[self.i - 1].clone();
            if stop2 != "end" {
                return self.fail(&end_tok, ErrorKind::StrayBlockWord { word: stop2 }, "`end`");
            }
            else_branch = Some(eb);
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

fn word_is(t: &Token, w: &str) -> bool {
    matches!(&t.tok, Tok::Name(n) if n == w)
}

fn word_is_at(t: &Tok, w: &str) -> bool {
    matches!(t, Tok::Name(n) if n == w)
}
