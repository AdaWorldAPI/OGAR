//! `ogar-from-ki`: a reader for HIRO Knowledge Items, for the documented
//! subset only.
//!
//! HIRO's public documentation shows a Knowledge Item as text in clauses
//! (`ki`, `on`, `when`, `do`) and gives three short examples. It gives no
//! grammar and no semantics beyond them. This crate reads exactly what those
//! examples show, refuses everything else by name, and keeps every open
//! question open. See the README.
//!
//! One reader, two projections:
//!
//! 1. [`parse`] gives a typed tree ([`KnowledgeItem`]) with line and column
//!    spans, or a named [`ParseError`].
//! 2. [`lift`] gives `ogar_vocab::ActionDef`s (one per `action(...)` call)
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

pub use analysis::{
    Contact, Interference, KiEdge, KiEdgeKind, SharedName, can_share_vertex, edges,
    interfering_pairs,
};
pub use ast::{KnowledgeItem, NameKind, Pos, Span, outline};
pub use error::{ErrorKind, ParseError};
pub use lift::{
    Branch, CallCondition, ConditionSite, KiArg, KiCall, KiCondition, KiNote, KiResultBinding,
    LiftError, LiftOptions, Lifted, LiftedItem, NoteKind, OnTarget, SourceError, lift, lift_source,
};
pub use parser::parse;
