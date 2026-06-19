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
//! Real emitters (so far): [`RustStruct`](rust_struct::RustStructEmitter)
//! (T1, from PR #78) and [`TsInterface`](ts_interface::TsInterfaceEmitter)
//! (T2, this PR). Remaining kinds use [`Stub`] — placeholder code that
//! compiles and emits a marker comment so callers can exercise the full
//! pipeline (lookup + dispatch + return) without waiting for every
//! template to land. Concrete emitters arrive per-kind in follow-on PRs
//! T3–T5 from the Northstar plan §3.

use crate::spec::{ArtifactKind, ArtifactSpec};

pub mod rust_struct;
pub mod stub;
pub mod ts_interface;

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
        ArtifactKind::TsInterface => Box::new(ts_interface::TsInterfaceEmitter),
        other => Box::new(stub::Stub { kind: other }),
    }
}
