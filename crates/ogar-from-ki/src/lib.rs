//! `ogar-from-ki`: a reader for HIRO Knowledge Items, for the language the
//! HIRO User Guides 7.0 ("KI Syntax") document.
//!
//! This is a reader. Nothing is evaluated, no condition is lowered to a guard
//! or a query, and the `on` (node) and `when` (issue) scopes are recorded as
//! data, not enforced. The guide has no formal grammar; a form that is not on
//! one of its pages is refused by name, with line and column. Where the guide
//! contradicts itself, both documented forms are read and the one written is
//! recorded. See the README.
//!
//! One reader, three projections:
//!
//! 1. [`parse`] gives a typed tree ([`KnowledgeItem`]) with line and column
//!    spans, or a named [`ParseError`]. [`KnowledgeItem::condition_refs`] lists
//!    the variables of the `on` and `when` conditions with their scope data.
//! 2. [`lift()`] gives `ogar_vocab::ActionDef`s (one per `action(...)` call)
//!    plus the item's facts and typed [`KiNote`]s for what could not be
//!    stated. This is the execution view: the defs register as methods
//!    through the existing converter.
//! 3. [`edges`] and [`interfering_pairs`] derive plain analysis rows from the
//!    same lifted items: static, name-level, not an evaluation.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod analysis;
pub mod ast;
pub mod error;
mod lexer;
pub mod lift;
mod parser;
pub mod scope;

pub use analysis::{
    Contact, Interference, KiEdge, KiEdgeKind, SharedName, can_share_vertex, edges,
    interfering_pairs,
};
pub use ast::{KnowledgeItem, NameKind, Pos, Span, outline};
pub use error::{ErrorKind, ParseError};
pub use lift::{
    Branch, CallCondition, ConditionSite, KiArg, KiArgKind, KiCall, KiCondition, KiNote,
    KiOtherCall, KiResultBinding, LiftError, LiftOptions, Lifted, LiftedItem, NoteKind, OnTarget,
    OtherCallKind, SourceError, lift, lift_source,
};
pub use parser::parse;
pub use scope::{ConditionRef, Section};
