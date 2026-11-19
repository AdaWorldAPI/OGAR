//! Tokens. The lexer stops at the first lexical fault and leaves a `Bad` token
//! that carries the error, so the parser reports an earlier grammar fault
//! first and this one only when the parser reaches it.

use crate::ast::{Pos, Span};
use crate::error::{ErrorKind, ParseError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tok {
    /// A name or keyword (`ogit/_id`, `LOCAL::X`, `if`, `on`).
    Name(String),
    Str(String),
    Number(String),
    EqEq,
    Eq,
    Colon,
    Comma,
    LParen,
    RParen,
    Newline,
    /// A run of operator characters other than `=` and `==`.
    Operator(String),
    Bad(ParseError),
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub tok: Tok,
    pub span: Span,
    /// First token on its line.
    pub first_on_line: bool,
}

const OP_CHARS: &str = "!<>=~&|+*%^?@-";

fn is_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_cont(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

struct Lexer {
    chars: Vec<char>,
    i: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    fn pos(&self) -> Pos {
        Pos {
            line: self.line,
            col: self.col,
        }
    }
    fn at(&self, n: usize) -> Option<char> {
        self.chars.get(self.i + n).copied()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.i).copied()?;
        self.i += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }
}

fn bad(kind: ErrorKind, pos: Pos, expected: &str) -> Tok {
    Tok::Bad(ParseError {
        kind,
        pos,
        expected: expected.to_string(),
    })
}

pub(crate) fn lex(src: &str) -> Vec<Token> {
    let mut lx = Lexer {
        chars: src.chars().collect(),
        i: 0,
        line: 1,
        col: 1,
    };
    let mut out: Vec<Token> = Vec::new();
    let mut first = true;
    while let Some(c) = lx.at(0) {
        let start = lx.pos();
        let tok = match c {
            '\n' => {
                lx.bump();
                let t = Token {
                    tok: Tok::Newline,
                    span: Span {
                        start,
                        end: lx.pos(),
                    },
                    first_on_line: first,
                };
                out.push(t);
                first = true;
                continue;
            }
            ' ' | '\t' | '\r' => {
                lx.bump();
                continue;
            }
            '"' => lex_string(&mut lx, start),
            '-' if lx.at(1).is_some_and(|d| d.is_ascii_digit()) => lex_number(&mut lx, start),
            c if c.is_ascii_digit() => lex_number(&mut lx, start),
            c if is_start(c) => lex_name(&mut lx),
            ':' => {
                lx.bump();
                Tok::Colon
            }
            ',' => {
                lx.bump();
                Tok::Comma
            }
            '(' => {
                lx.bump();
                Tok::LParen
            }
            ')' => {
                lx.bump();
                Tok::RParen
            }
            c if OP_CHARS.contains(c) => {
                let mut run = String::new();
                while let Some(d) = lx.at(0) {
                    if OP_CHARS.contains(d) {
                        run.push(d);
                        lx.bump();
                    } else {
                        break;
                    }
                }
                match run.as_str() {
                    "=" => Tok::Eq,
                    "==" => Tok::EqEq,
                    _ => Tok::Operator(run),
                }
            }
            other => bad(
                ErrorKind::UnexpectedChar { ch: other },
                start,
                "a name, string, number, `:`, `,`, `(`, `)` or `==`",
            ),
        };
        let is_bad = matches!(tok, Tok::Bad(_));
        out.push(Token {
            tok,
            span: Span {
                start,
                end: lx.pos(),
            },
            first_on_line: first,
        });
        first = false;
        if is_bad {
            break;
        }
    }
    let end = lx.pos();
    out.push(Token {
        tok: Tok::Eof,
        span: Span { start: end, end },
        first_on_line: first,
    });
    out
}

fn lex_string(lx: &mut Lexer, start: Pos) -> Tok {
    lx.bump(); // opening quote
    let mut value = String::new();
    loop {
        match lx.at(0) {
            None | Some('\n') => {
                return bad(
                    ErrorKind::UnterminatedString,
                    start,
                    "a closing `\"` on the same line",
                );
            }
            Some('"') => {
                lx.bump();
                return Tok::Str(value);
            }
            Some('\\') => {
                return bad(
                    ErrorKind::EscapeInString,
                    lx.pos(),
                    "a string without backslashes (escapes are not documented)",
                );
            }
            Some(c) => {
                value.push(c);
                lx.bump();
            }
        }
    }
}

fn lex_number(lx: &mut Lexer, start: Pos) -> Tok {
    let mut text = String::new();
    if lx.at(0) == Some('-') {
        text.push('-');
        lx.bump();
    }
    while let Some(d) = lx.at(0).filter(char::is_ascii_digit) {
        text.push(d);
        lx.bump();
    }
    if lx.at(0) == Some('.') && lx.at(1).is_some_and(|d| d.is_ascii_digit()) {
        text.push('.');
        lx.bump();
        while let Some(d) = lx.at(0).filter(char::is_ascii_digit) {
            text.push(d);
            lx.bump();
        }
    }
    if lx.at(0).is_some_and(is_cont) {
        while let Some(d) = lx.at(0).filter(|d| is_cont(*d)) {
            text.push(d);
            lx.bump();
        }
        return bad(
            ErrorKind::MalformedNumber { text },
            start,
            "a number such as `0` or `2.5`",
        );
    }
    Tok::Number(text)
}

fn lex_name(lx: &mut Lexer) -> Tok {
    let mut text = String::new();
    while let Some(c) = lx.at(0).filter(|c| is_cont(*c)) {
        text.push(c);
        lx.bump();
    }
    loop {
        match (lx.at(0), lx.at(1), lx.at(2)) {
            (Some('/'), Some(n), _) if is_cont(n) => {
                text.push('/');
                lx.bump();
            }
            (Some(':'), Some(':'), Some(n)) if is_start(n) => {
                text.push_str("::");
                lx.bump();
                lx.bump();
            }
            _ => break,
        }
        while let Some(c) = lx.at(0).filter(|c| is_cont(*c)) {
            text.push(c);
            lx.bump();
        }
    }
    Tok::Name(text)
}
