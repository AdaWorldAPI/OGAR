//! Per-kind emitter dispatch. Mirror of `woa-rs::codegen::handler_kinds`.
//!
//! Each [`ArtifactKind`] owns its own emitter. The dispatcher is a tiny
//! `match` over the enum, returning a boxed trait object so the call site
//! is one line:
//!
//! ```ignore
//! let emitter = artifact_kinds::for_kind(spec.kind);
//! let source = emitter.emit(&spec)?;
//! ```
//!
//! Real emitters (so far):
//! - [`RustStruct`](rust_struct::RustStructEmitter) — T1, codegen flavour,
//!   from PR #78.
//! - [`HtmlListView`](html_list_view::HtmlListViewEmitter) — T2, render
//!   flavour. Mirrors Redmine's `_list.html.erb` shape on our substrate
//!   (see `docs/integration/REDMINE-QUERY-HARVEST.md`).
//!
//! Remaining kinds use [`Stub`] — placeholder code that compiles and
//! emits a marker comment so callers can exercise the full pipeline
//! (lookup + dispatch + return) before T3–T5 land.

pub(crate) mod cells;

use crate::spec::{ArtifactKind, ArtifactSpec};

pub mod html_list_view;
pub mod rust_struct;
pub mod stub;

pub use html_list_view::{
    render_list, AttachmentEntryOwned, CellData, CellSource, GroupHeader, HtmlListViewEmitter,
    RelationEntryOwned, RowSource, UserEntryOwned,
};

/// Contract every kind's emitter implements.
pub trait ArtifactEmitter {
    /// Render `spec.class` as the target artifact for this emitter's
    /// [`ArtifactKind`]. Returns the emitted source as a `String`;
    /// downstream tooling writes it to disk.
    fn emit(&self, spec: &ArtifactSpec<'_>) -> Result<String, askama::Error>;
}

/// Dispatch to the concrete emitter for `kind`. Always returns Some
/// emitter — unimplemented kinds fall through to [`Stub`].
pub fn for_kind(kind: ArtifactKind) -> Box<dyn ArtifactEmitter> {
    match kind {
        ArtifactKind::RustStruct => Box::new(rust_struct::RustStructEmitter),
        ArtifactKind::HtmlListView => Box::new(html_list_view::HtmlListViewEmitter),
        other => Box::new(stub::Stub { kind: other }),
    }
}
