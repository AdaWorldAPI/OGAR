//! Named refusals. Every construct outside the documented subset is refused
//! with its line and column and what was expected; nothing is skipped or
//! guessed.

use std::fmt;

use crate::ast::Pos;

/// What was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// A character that starts no documented token (`#`, `.`, `{`, `'`, ...).
    UnexpectedChar {
        /// The character.
        ch: char,
    },
    /// A string literal with no closing quote on its line.
    UnterminatedString,
    /// A backslash inside a string: escape sequences are not documented.
    EscapeInString,
    /// A number followed directly by letters.
    MalformedNumber {
        /// The text read.
        text: String,
    },
    /// An operator other than `==` where a comparison was expected.
    UnsupportedOperator {
        /// The operator as written.
        op: String,
    },
    /// `and`, `or` or `not` after a condition: no combination is documented.
    UnsupportedConnective {
        /// The word.
        word: String,
    },
    /// A number where a condition is expected or compared (numbers in `when`).
    NumberInCondition {
        /// The literal.
        text: String,
    },
    /// A call to a function other than `action`, `log`, `delete`.
    UnknownFunction {
        /// The function name.
        name: String,
    },
    /// A call inside the arguments of a call.
    NestedCall {
        /// The inner function name.
        name: String,
    },
    /// A call where a condition or assignment value is expected.
    CallInExpression {
        /// The function name.
        name: String,
    },
    /// A word at a clause position that is not `ki`, `on`, `when`, `do`.
    UnknownClauseKeyword {
        /// The word.
        word: String,
    },
    /// A clause that must come next is missing (end of text).
    MissingClause {
        /// `on`, `when` or `do`.
        clause: &'static str,
    },
    /// A known clause keyword where another was due (order is `ki`, `on`,
    /// `when`, `do`, each once).
    UnexpectedClause {
        /// The keyword found.
        found: String,
        /// The clause that was due.
        wanted: &'static str,
    },
    /// A clause with no body.
    EmptyClause {
        /// The clause.
        clause: &'static str,
    },
    /// A header key other than `name` and `description`.
    UnknownHeaderKey {
        /// The key.
        key: String,
    },
    /// A header key written twice.
    DuplicateHeaderKey {
        /// The key.
        key: String,
    },
    /// An `if` with no closing `end`.
    UnterminatedIf,
    /// An `if` with no `else` (the documented form always has one).
    MissingElse,
    /// A branch with no statement.
    EmptyBranch,
    /// An `if` inside an `if` branch: not documented.
    NestedIf,
    /// `then`, `else` or `end` where no `if` is open.
    StrayBlockWord {
        /// The word.
        word: String,
    },
    /// A namespace other than `LOCAL::`, or a malformed local name.
    UnknownNamespace {
        /// The name as written.
        name: String,
    },
    /// A keyword used as a name.
    ReservedWord {
        /// The word.
        word: String,
    },
    /// A named argument given twice.
    DuplicateArgument {
        /// The argument name.
        name: String,
    },
    /// `action(...)` with no capability.
    MissingCapability,
    /// A capability given both positionally and as `capability:`.
    DuplicateCapability,
    /// A capability with an empty name.
    EmptyCapability,
    /// Anything else: a token that does not fit the grammar here.
    UnexpectedToken {
        /// What was found.
        found: String,
    },
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::UnexpectedChar { ch } => write!(f, "unexpected character {ch:?}"),
            ErrorKind::UnterminatedString => write!(f, "unterminated string"),
            ErrorKind::EscapeInString => {
                write!(f, "escape sequence in string (not documented)")
            }
            ErrorKind::MalformedNumber { text } => write!(f, "malformed number `{text}`"),
            ErrorKind::UnsupportedOperator { op } => write!(f, "unsupported operator `{op}`"),
            ErrorKind::UnsupportedConnective { word } => {
                write!(f, "unsupported connective `{word}`")
            }
            ErrorKind::NumberInCondition { text } => {
                write!(f, "number `{text}` in a condition")
            }
            ErrorKind::UnknownFunction { name } => write!(f, "unknown function `{name}`"),
            ErrorKind::NestedCall { name } => write!(f, "nested call `{name}(`"),
            ErrorKind::CallInExpression { name } => {
                write!(f, "call `{name}(` in a condition or value")
            }
            ErrorKind::UnknownClauseKeyword { word } => {
                write!(f, "unknown clause keyword `{word}`")
            }
            ErrorKind::MissingClause { clause } => write!(f, "missing `{clause}` clause"),
            ErrorKind::UnexpectedClause { found, wanted } => {
                write!(f, "unexpected clause `{found}` (`{wanted}` was due)")
            }
            ErrorKind::EmptyClause { clause } => write!(f, "empty `{clause}` clause"),
            ErrorKind::UnknownHeaderKey { key } => write!(f, "unknown header key `{key}`"),
            ErrorKind::DuplicateHeaderKey { key } => {
                write!(f, "header key `{key}` written twice")
            }
            ErrorKind::UnterminatedIf => write!(f, "`if` without `end`"),
            ErrorKind::MissingElse => write!(f, "`if` without `else`"),
            ErrorKind::EmptyBranch => write!(f, "empty `if` branch"),
            ErrorKind::NestedIf => write!(f, "nested `if`"),
            ErrorKind::StrayBlockWord { word } => write!(f, "stray `{word}`"),
            ErrorKind::UnknownNamespace { name } => {
                write!(f, "unknown namespace or malformed local name `{name}`")
            }
            ErrorKind::ReservedWord { word } => write!(f, "keyword `{word}` used as a name"),
            ErrorKind::DuplicateArgument { name } => {
                write!(f, "argument `{name}` given twice")
            }
            ErrorKind::MissingCapability => write!(f, "`action(` without a capability"),
            ErrorKind::DuplicateCapability => {
                write!(f, "capability given both positionally and as `capability:`")
            }
            ErrorKind::EmptyCapability => write!(f, "empty capability name"),
            ErrorKind::UnexpectedToken { found } => write!(f, "unexpected {found}"),
        }
    }
}

/// A refusal: what, where, and what would have been accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// What was refused.
    pub kind: ErrorKind,
    /// Where (line and column of the offending token).
    pub pos: Pos,
    /// What the grammar expected at that place.
    pub expected: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}; expected {}",
            self.pos.line, self.pos.col, self.kind, self.expected
        )
    }
}

impl std::error::Error for ParseError {}
