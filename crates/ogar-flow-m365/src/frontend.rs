//! Lowering an admitted PowerShell-shaped pipeline into `ogar-loco` calls.
//!
//! This is a declarative input format, not PowerShell. The admitted subset:
//!
//! ```text
//! pipeline   := stage ( "|" stage )*
//! stage      := label argument*
//! argument   := "-" Name value? | value
//! value      := Word | "double-quoted" | 'single-quoted' | value "," value
//! ```
//!
//! with one shape per operation:
//!
//! | operation | admitted form |
//! |---|---|
//! | `query_user` | `Get-MgUser -UserId <ref>` or `Get-MgUser <ref>` |
//! | `read_license_evidence`, `read_membership` | no arguments |
//! | `filter` | `Where-Object <Field> -eq\|-ne\|-contains <value>` |
//! | `project` | `Select-Object [-Property] <Field>, <Field>, …` |
//! | `export_report` | `Export-Report [-Format JSON]` |
//!
//! Anything else is refused: script blocks, variables, subexpressions,
//! statement separators, redirection, escapes, unknown parameters, and every
//! label [`resolve`](crate::adjacency::resolve) refuses.

use lance_graph_contract::class_view::FieldMask;
use ogar_loco::{Call, FnIndex, FunctionBody, Program};

use crate::adjacency::{self, FlowShape, OperationId, ResolveError};
use crate::vocab::{self, Comparison, ExportFormat, FLOW_SHAPE, ReportField};

/// Characters that only appear in PowerShell constructs this frontend does
/// not admit (script blocks, variables, subexpressions, separators,
/// redirection, escapes, splatting, hashtables, call operators).
const REFUSED_CHARS: &[char] = &['{', '}', '$', ';', '&', '(', ')', '`', '>', '<', '[', ']'];

/// Characters refused only where a token starts (`@params`, `@(…)`, `@{…}`);
/// inside a bareword such as `john.doe@example.test` they are literal.
const REFUSED_AT_START: &[char] = &['@'];

/// One resolved stage, recorded for the report's command trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEntry {
    /// The label exactly as written.
    pub label: String,
    /// The canonical operation it resolved to.
    pub operation: &'static str,
    /// The call byte it lowered to.
    pub fn_index: FnIndex,
}

/// A lowered pipeline: one `ogar-loco` program plus the strings its calls
/// index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledFlow {
    /// The program. One body: the entry.
    pub program: Program,
    /// The argument table `get_user` and `where` index by value byte.
    pub args: Vec<String>,
    /// One entry per stage, in order.
    pub trace: Vec<TraceEntry>,
    /// The user reference the pipeline asked for.
    pub requested: String,
}

/// Why a pipeline did not compile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// The input uses PowerShell syntax this frontend does not admit.
    UnsupportedSyntax {
        /// The offending character.
        found: char,
    },
    /// An unterminated quoted string.
    UnterminatedString,
    /// A stage with no command.
    EmptyStage,
    /// The label did not resolve to an executable operation.
    Resolve(ResolveError),
    /// A parameter the operation does not take.
    UnknownParameter {
        /// The stage's label.
        label: String,
        /// The parameter as written.
        parameter: String,
    },
    /// The stage's arguments do not match its admitted form.
    BadArguments {
        /// The stage's label.
        label: String,
        /// What was expected.
        expected: &'static str,
    },
    /// A property name that is not a [`ReportField`].
    UnknownField(String),
    /// `-contains` on a scalar field, or `-eq`/`-ne` on a list field.
    ComparisonMismatch {
        /// The field.
        field: ReportField,
    },
    /// An export format this host does not produce.
    UnknownFormat(String),
    /// A stage cannot consume what the previous stage produces.
    ShapeMismatch {
        /// The stage's label.
        label: String,
        /// What the stage consumes.
        expected: FlowShape,
        /// What the previous stage produced.
        found: FlowShape,
    },
    /// The pipeline does not end in a report.
    NoExport,
    /// More strings than one value byte can index.
    TooManyArguments,
    /// More stages than one function body holds.
    TooManyStages,
}

impl core::fmt::Display for CompileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl core::error::Error for CompileError {}

impl From<ResolveError> for CompileError {
    fn from(e: ResolveError) -> Self {
        Self::Resolve(e)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    /// `-Name`.
    Param(String),
    /// A bare word or quoted string.
    Value(String),
    /// `,`.
    Comma,
}

/// Append to the current stage. `stages` always holds at least one.
fn push(stages: &mut [Vec<Token>], tok: Token) {
    if let Some(s) = stages.last_mut() {
        s.push(tok);
    }
}

/// Split the pipeline into stages and tokens, refusing unadmitted syntax.
fn tokenize(source: &str) -> Result<Vec<Vec<Token>>, CompileError> {
    let mut stages = vec![Vec::new()];
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {}
            '|' => stages.push(Vec::new()),
            ',' => push(&mut stages, Token::Comma),
            '"' | '\'' => {
                let mut s = String::new();
                loop {
                    match chars.next() {
                        None => return Err(CompileError::UnterminatedString),
                        Some(q) if q == c => break,
                        Some(x) if REFUSED_CHARS.contains(&x) => {
                            return Err(CompileError::UnsupportedSyntax { found: x });
                        }
                        Some(x) => s.push(x),
                    }
                }
                push(&mut stages, Token::Value(s));
            }
            c if REFUSED_CHARS.contains(&c) || REFUSED_AT_START.contains(&c) => {
                return Err(CompileError::UnsupportedSyntax { found: c });
            }
            c => {
                let mut word = String::from(c);
                while let Some(&n) = chars.peek() {
                    if n.is_whitespace() || n == '|' || n == ',' || n == '"' || n == '\'' {
                        break;
                    }
                    if REFUSED_CHARS.contains(&n) {
                        return Err(CompileError::UnsupportedSyntax { found: n });
                    }
                    word.push(n);
                    chars.next();
                }
                let tok = match word.strip_prefix('-') {
                    Some(name) if !name.is_empty() => Token::Param(name.to_owned()),
                    _ => Token::Value(word),
                };
                push(&mut stages, tok);
            }
        }
    }
    Ok(stages)
}

/// Values separated by commas, with no parameters among them.
fn value_list(tokens: &[Token]) -> Option<Vec<&str>> {
    let mut out = Vec::new();
    let mut expect_value = true;
    for t in tokens {
        match (t, expect_value) {
            (Token::Value(v), true) => {
                out.push(v.as_str());
                expect_value = false;
            }
            (Token::Comma, false) => expect_value = true,
            _ => return None,
        }
    }
    (!out.is_empty() && !expect_value).then_some(out)
}

struct Lowerer {
    args: Vec<String>,
}

impl Lowerer {
    fn intern(&mut self, s: &str) -> Result<u8, CompileError> {
        if let Some(i) = self.args.iter().position(|a| a == s) {
            return Ok(i as u8);
        }
        let i = u8::try_from(self.args.len()).map_err(|_| CompileError::TooManyArguments)?;
        self.args.push(s.to_owned());
        Ok(i)
    }

    fn lower(
        &mut self,
        label: &str,
        op: OperationId,
        args: &[Token],
    ) -> Result<Call, CompileError> {
        let bad = |expected| CompileError::BadArguments {
            label: label.to_owned(),
            expected,
        };
        let unknown = |p: &str| CompileError::UnknownParameter {
            label: label.to_owned(),
            parameter: p.to_owned(),
        };
        let spec = adjacency::operation(op);
        let f = spec
            .fn_index
            .expect("resolve only returns operations with a call byte");
        match f {
            vocab::GET_USER => {
                if let Some(Token::Param(p)) = args
                    .iter()
                    .find(|t| matches!(t, Token::Param(p) if !p.eq_ignore_ascii_case("UserId")))
                {
                    return Err(unknown(p));
                }
                let user = match args {
                    [Token::Param(p), Token::Value(v)] if p.eq_ignore_ascii_case("UserId") => v,
                    [Token::Value(v)] => v,
                    _ => return Err(bad("-UserId <user>")),
                };
                Ok(Call::with_value(f, self.intern(user)?))
            }
            vocab::GET_LICENSE_EVIDENCE | vocab::GET_MEMBERSHIP => match args {
                [] => Ok(Call::new(f)),
                [Token::Param(p), ..] => Err(unknown(p)),
                _ => Err(bad("no arguments")),
            },
            vocab::WHERE => {
                let [Token::Value(field), Token::Param(cmp), Token::Value(value)] = args else {
                    if let Some(Token::Param(p)) = args.first() {
                        return Err(unknown(p));
                    }
                    return Err(bad("<Field> -eq|-ne|-contains <value>"));
                };
                let field = ReportField::from_name(field)
                    .ok_or_else(|| CompileError::UnknownField(field.clone()))?;
                let cmp = match cmp.to_ascii_lowercase().as_str() {
                    "eq" => Comparison::Eq,
                    "ne" => Comparison::Ne,
                    "contains" => Comparison::Contains,
                    _ => return Err(unknown(cmp)),
                };
                if field.is_list() != (cmp == Comparison::Contains) {
                    return Err(CompileError::ComparisonMismatch { field });
                }
                Ok(Call::with_values(
                    f,
                    [field.position(), cmp.byte(), self.intern(value)?],
                ))
            }
            vocab::SELECT => {
                let rest = match args {
                    [Token::Param(p), rest @ ..] if p.eq_ignore_ascii_case("Property") => rest,
                    [Token::Param(p), ..] => return Err(unknown(p)),
                    rest => rest,
                };
                let names = value_list(rest).ok_or_else(|| bad("<Field>, <Field>, …"))?;
                let mut mask = FieldMask::EMPTY;
                for n in names {
                    let field = ReportField::from_name(n)
                        .ok_or_else(|| CompileError::UnknownField(n.to_owned()))?;
                    mask = mask.with(field.position());
                }
                let packed =
                    vocab::pack_mask(mask).ok_or_else(|| bad("fields below position 24"))?;
                Ok(Call::with_values(f, packed))
            }
            vocab::EXPORT => {
                let format = match args {
                    [] => ExportFormat::Json,
                    [Token::Param(p), Token::Value(v)] if p.eq_ignore_ascii_case("Format") => {
                        if v.eq_ignore_ascii_case("JSON") {
                            ExportFormat::Json
                        } else {
                            return Err(CompileError::UnknownFormat(v.clone()));
                        }
                    }
                    [Token::Param(p), ..] => return Err(unknown(p)),
                    _ => return Err(bad("[-Format JSON]")),
                };
                Ok(Call::with_value(f, format as u8))
            }
            _ => unreachable!("every executable operation is lowered above"),
        }
    }
}

/// Compile a pipeline into a [`CompiledFlow`].
///
/// # Errors
///
/// The first [`CompileError`] found. Nothing partial is returned.
pub fn compile(source: &str) -> Result<CompiledFlow, CompileError> {
    let stages = tokenize(source)?;
    let mut lowerer = Lowerer { args: Vec::new() };
    let mut calls = Vec::with_capacity(stages.len());
    let mut trace = Vec::with_capacity(stages.len());
    let mut requested = None;
    let mut flowing = FlowShape::Nothing;
    for stage in &stages {
        let (Token::Value(label), args) = stage.split_first().ok_or(CompileError::EmptyStage)?
        else {
            return Err(CompileError::EmptyStage);
        };
        let (op_id, op) = adjacency::resolve(label)?;
        if op.input != flowing {
            return Err(CompileError::ShapeMismatch {
                label: label.clone(),
                expected: op.input,
                found: flowing,
            });
        }
        let call = lowerer.lower(label, op_id, args)?;
        if call.function == vocab::GET_USER {
            requested = Some(lowerer.args[usize::from(call.values[0])].clone());
        }
        calls.push(call);
        trace.push(TraceEntry {
            label: label.clone(),
            operation: op.name,
            fn_index: call.function,
        });
        flowing = op.output;
    }
    if flowing != FlowShape::Report {
        return Err(CompileError::NoExport);
    }
    let body =
        FunctionBody::from_calls(FLOW_SHAPE, &calls).map_err(|_| CompileError::TooManyStages)?;
    Ok(CompiledFlow {
        program: Program {
            functions: vec![body],
        },
        args: lowerer.args,
        trace,
        requested: requested.expect("a pipeline ending in a report starts with its one source"),
    })
}
