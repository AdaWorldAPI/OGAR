//! Named refusals. Every construct outside the documented language is refused
//! with its line and column and what was expected; nothing is skipped or
//! guessed.

use std::fmt;

use crate::ast::Pos;

/// What was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// A character that starts no token (`{`, `}`, `;`, `$`, ...).
    UnexpectedChar {
        /// The character.
        ch: char,
    },
    /// A string literal with no closing quote.
    UnterminatedString,
    /// A regex literal `/.../` with no closing slash on its line.
    UnterminatedRegex,
    /// A backslash directly before the closing quote character of a string:
    /// whether it escapes the quote is not documented.
    EscapeInString,
    /// A number followed directly by letters.
    MalformedNumber {
        /// The text read.
        text: String,
    },
    /// An operator the guide does not list, where a comparison or an
    /// assignment was expected.
    UnsupportedOperator {
        /// The operator as written.
        op: String,
    },
    /// A number where a condition must start (a condition names a variable).
    NumberInCondition {
        /// The literal.
        text: String,
    },
    /// A call inside the arguments of a call.
    NestedCall {
        /// The inner function name.
        name: String,
    },
    /// A call where a condition or an assignment value is expected.
    CallInExpression {
        /// The function name.
        name: String,
    },
    /// A function name with no argument list, as in the guide's `action`
    /// reference page (`exit: LOCAL::RESULT = action` and then a new line).
    CallWithoutArguments {
        /// The function name.
        function: String,
    },
    /// Result tags in front of a call that returns nothing (`log`, `delete`,
    /// `set`, `add`).
    BindingsOnCall {
        /// The function name.
        function: String,
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
    /// A clause or block with no body.
    EmptyClause {
        /// The clause or block word.
        clause: &'static str,
    },
    /// A header key other than `name`, `description`, `version`, `tier` and
    /// `id`.
    UnknownHeaderKey {
        /// The key.
        key: String,
    },
    /// A header key written twice.
    DuplicateHeaderKey {
        /// The key.
        key: String,
    },
    /// A `tier` other than `default`, `policy`, `fallback`.
    UnknownTier {
        /// The value as written.
        value: String,
    },
    /// An `if` with no closing `end`.
    UnterminatedIf,
    /// A `with`, `count` or `begin` block with no closing `end`.
    UnterminatedBlock {
        /// `with`, `count` or `begin`.
        word: &'static str,
    },
    /// A `with` or `count` block inside another: the guide shows no nesting.
    NestedBlock {
        /// The enclosing block word.
        outer: &'static str,
        /// The inner block word.
        inner: &'static str,
    },
    /// A `with` or `count` block in `when`: the guide shows them in `on`.
    BlockOutsideOn {
        /// `with` or `count`.
        word: &'static str,
    },
    /// An edge direction other than `in`, `out`, `connected` (and the
    /// example-only `parent`, `child`).
    UnknownDirection {
        /// The direction as written.
        found: String,
    },
    /// A `count` block without `as <Name>`.
    CountWithoutAlias,
    /// A graph scope limiter other than `@any`, `@self` (and the
    /// example-only `@in`).
    UnknownGraphScope {
        /// The suffix as written.
        found: String,
    },
    /// A log level other than `debug`, `info`, `warn`, `error`.
    UnknownLogLevel {
        /// The level as written.
        level: String,
    },
    /// A branch with no statement.
    EmptyBranch,
    /// An `if` inside an `if` branch: not shown in the guide.
    NestedIf,
    /// `then`, `else` or `end` where no block is open.
    StrayBlockWord {
        /// The word.
        word: String,
    },
    /// `and` and `or` on one level with no parentheses: the guide states
    /// "not" over "or" but no order between "and" and "or".
    AmbiguousConnectives {
        /// The connective seen first.
        first: String,
        /// The other one.
        second: String,
    },
    /// A construct the guide documents that this reader does not read yet.
    DocumentedNotRead {
        /// What it is.
        construct: String,
    },
    /// An optional match (`? X`) with no `:= default` (only `?list` may omit
    /// it).
    OptionalWithoutDefault,
    /// Two arithmetic operators in one expression: the guide shows one.
    ChainedArithmetic,
    /// A form that only a match line has (`?`, `list`, `as`) inside an `if`
    /// condition.
    MatchFormInIf {
        /// The form.
        form: &'static str,
    },
    /// A positional argument after a keyword argument.
    PositionalAfterKeyword,
    /// A namespace written wrongly, or a malformed qualified name.
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
            ErrorKind::UnterminatedRegex => write!(f, "unterminated regex literal"),
            ErrorKind::EscapeInString => {
                write!(f, "escaped quote in string (not documented)")
            }
            ErrorKind::MalformedNumber { text } => write!(f, "malformed number `{text}`"),
            ErrorKind::UnsupportedOperator { op } => write!(f, "unsupported operator `{op}`"),
            ErrorKind::NumberInCondition { text } => {
                write!(f, "number `{text}` where a condition must start")
            }
            ErrorKind::NestedCall { name } => write!(f, "nested call `{name}(`"),
            ErrorKind::CallInExpression { name } => {
                write!(f, "call `{name}(` in a condition or value")
            }
            ErrorKind::CallWithoutArguments { function } => {
                write!(f, "`{function}` with no argument list")
            }
            ErrorKind::BindingsOnCall { function } => {
                write!(
                    f,
                    "result tags in front of `{function}`, which returns none"
                )
            }
            ErrorKind::UnknownClauseKeyword { word } => {
                write!(f, "unknown clause keyword `{word}`")
            }
            ErrorKind::MissingClause { clause } => write!(f, "missing `{clause}` clause"),
            ErrorKind::UnexpectedClause { found, wanted } => {
                write!(f, "unexpected clause `{found}` (`{wanted}` was due)")
            }
            ErrorKind::EmptyClause { clause } => write!(f, "empty `{clause}` body"),
            ErrorKind::UnknownHeaderKey { key } => write!(f, "unknown header key `{key}`"),
            ErrorKind::DuplicateHeaderKey { key } => {
                write!(f, "header key `{key}` written twice")
            }
            ErrorKind::UnknownTier { value } => write!(f, "unknown tier \"{value}\""),
            ErrorKind::UnterminatedIf => write!(f, "`if` without `end`"),
            ErrorKind::UnterminatedBlock { word } => write!(f, "`{word}` without `end`"),
            ErrorKind::NestedBlock { outer, inner } => {
                write!(f, "`{inner}` block inside a `{outer}` block")
            }
            ErrorKind::BlockOutsideOn { word } => {
                write!(f, "`{word}` block outside the `on` section")
            }
            ErrorKind::UnknownDirection { found } => {
                write!(f, "unknown edge direction `{found}`")
            }
            ErrorKind::CountWithoutAlias => write!(f, "`count` block without `as <Name>`"),
            ErrorKind::UnknownGraphScope { found } => {
                write!(f, "unknown graph scope `@{found}`")
            }
            ErrorKind::UnknownLogLevel { level } => write!(f, "unknown log level `{level}`"),
            ErrorKind::EmptyBranch => write!(f, "empty `if` branch"),
            ErrorKind::NestedIf => write!(f, "nested `if`"),
            ErrorKind::StrayBlockWord { word } => write!(f, "stray `{word}`"),
            ErrorKind::AmbiguousConnectives { first, second } => {
                write!(f, "`{first}` and `{second}` mixed without parentheses")
            }
            ErrorKind::DocumentedNotRead { construct } => {
                write!(f, "documented but not read: {construct}")
            }
            ErrorKind::OptionalWithoutDefault => {
                write!(f, "optional match without `:= default`")
            }
            ErrorKind::ChainedArithmetic => write!(f, "more than one arithmetic operator"),
            ErrorKind::MatchFormInIf { form } => {
                write!(f, "match form `{form}` in an `if` condition")
            }
            ErrorKind::PositionalAfterKeyword => {
                write!(f, "positional argument after a keyword argument")
            }
            ErrorKind::UnknownNamespace { name } => {
                write!(f, "unknown namespace or malformed qualified name `{name}`")
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
