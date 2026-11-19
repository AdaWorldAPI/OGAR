//! Tokens. The lexer stops at the first lexical fault and leaves a `Bad` token
//! that carries the error, so the parser reports an earlier grammar fault
//! first and this one only when the parser reaches it.
//!
//! Comments (`#` to the end of the line, outside strings) are dropped here.
//! Strings may be written with `"` or `'` and may span lines; a backslash is
//! kept as written, except directly before the closing quote character,
//! which is refused. A regex literal `/.../flags` is lexed only right after
//! `=~` or `!~`.

use crate::ast::{Pos, Quote, Span};
use crate::error::{ErrorKind, ParseError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tok {
    /// A name or keyword (`ogit/_id`, `LOCAL::X`, `if`, `on`).
    Name(String),
    Str(String, Quote),
    Number(String),
    /// `/pattern/flags`, only after `=~` or `!~`.
    Regex(String, String),
    /// An operator: `==`, `=`, `<`, `+`, `?`, `:=`, or a run of comparison
    /// characters that is no documented operator (the parser refuses it).
    Op(String),
    Colon,
    Comma,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Dot,
    At,
    Newline,
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

/// Characters of the comparison operators. A maximal run is one token.
const CMP_CHARS: &str = "!<>=~&|^";
/// Arithmetic and optional-match characters. Each is one token.
const SINGLE_OPS: &str = "+-*/%?";

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
    // Whether the previous token on this line ends a value. Decides whether
    // `-1` is a negative number or a subtraction.
    let mut prev_value = false;
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
                prev_value = false;
                continue;
            }
            ' ' | '\t' | '\r' => {
                lx.bump();
                continue;
            }
            '#' => {
                while lx.at(0).is_some_and(|d| d != '\n') {
                    lx.bump();
                }
                continue;
            }
            '"' | '\'' => lex_string(&mut lx, start, c),
            '-' | '+' if !prev_value && lx.at(1).is_some_and(|d| d.is_ascii_digit()) => {
                lex_number(&mut lx, start)
            }
            c if c.is_ascii_digit() => lex_number(&mut lx, start),
            c if is_start(c) => lex_name(&mut lx),
            ':' => {
                lx.bump();
                if lx.at(0) == Some('=') {
                    lx.bump();
                    Tok::Op(":=".to_string())
                } else {
                    Tok::Colon
                }
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
            '[' => {
                lx.bump();
                Tok::LBracket
            }
            ']' => {
                lx.bump();
                Tok::RBracket
            }
            '.' => {
                lx.bump();
                Tok::Dot
            }
            '@' => {
                lx.bump();
                Tok::At
            }
            '/' if matches!(
                out.last().map(|t| &t.tok),
                Some(Tok::Op(o)) if o == "=~" || o == "!~"
            ) || matches!(
                out.last().map(|t| &t.tok),
                Some(Tok::Colon | Tok::Comma | Tok::LParen)
            ) =>
            {
                lex_regex(&mut lx, start)
            }
            c if CMP_CHARS.contains(c) => {
                let mut run = String::new();
                while let Some(d) = lx.at(0) {
                    if CMP_CHARS.contains(d) {
                        run.push(d);
                        lx.bump();
                    } else {
                        break;
                    }
                }
                Tok::Op(run)
            }
            c if SINGLE_OPS.contains(c) => {
                lx.bump();
                Tok::Op(c.to_string())
            }
            other => bad(
                ErrorKind::UnexpectedChar { ch: other },
                start,
                "a name, string, number, operator, `:`, `,`, `(`, `)`, `[`, `]`, `.` or `@`",
            ),
        };
        let is_bad = matches!(tok, Tok::Bad(_));
        prev_value = matches!(
            tok,
            Tok::Name(_)
                | Tok::Str(..)
                | Tok::Number(_)
                | Tok::Regex(..)
                | Tok::RParen
                | Tok::RBracket
        );
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

fn lex_string(lx: &mut Lexer, start: Pos, quote: char) -> Tok {
    lx.bump(); // opening quote
    let mut value = String::new();
    loop {
        match lx.at(0) {
            None => {
                return bad(
                    ErrorKind::UnterminatedString,
                    start,
                    "a closing quote (strings may span lines)",
                );
            }
            Some(c) if c == quote => {
                lx.bump();
                let q = if quote == '"' {
                    Quote::Double
                } else {
                    Quote::Single
                };
                return Tok::Str(value, q);
            }
            Some('\\') if lx.at(1) == Some(quote) => {
                return bad(
                    ErrorKind::EscapeInString,
                    lx.pos(),
                    "a string without an escaped quote (escapes are not documented)",
                );
            }
            Some(c) => {
                value.push(c);
                lx.bump();
            }
        }
    }
}

fn lex_regex(lx: &mut Lexer, start: Pos) -> Tok {
    lx.bump(); // opening slash
    let mut pattern = String::new();
    loop {
        match lx.at(0) {
            None | Some('\n') => {
                return bad(
                    ErrorKind::UnterminatedRegex,
                    start,
                    "a closing `/` on the same line",
                );
            }
            Some('/') => {
                lx.bump();
                break;
            }
            Some('\\') if lx.at(1).is_some_and(|d| d != '\n') => {
                pattern.push('\\');
                lx.bump();
                if let Some(d) = lx.bump() {
                    pattern.push(d);
                }
            }
            Some(c) => {
                pattern.push(c);
                lx.bump();
            }
        }
    }
    let mut flags = String::new();
    while let Some(c) = lx.at(0).filter(char::is_ascii_alphabetic) {
        flags.push(c);
        lx.bump();
    }
    Tok::Regex(pattern, flags)
}

fn lex_number(lx: &mut Lexer, start: Pos) -> Tok {
    let mut text = String::new();
    if let Some(sign @ ('-' | '+')) = lx.at(0) {
        text.push(sign);
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
