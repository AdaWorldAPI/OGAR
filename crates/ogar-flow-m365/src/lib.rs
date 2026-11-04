//! `ogar-flow-m365` — PowerShell-shaped command labels over the `ogar-loco`
//! call ABI.
//!
//! ```text
//!   "Get-MgUser -UserId … | Get-IamLicenseEvidence | Select-Object … | Export-Report"
//!        │  frontend::compile        (strings stop here)
//!        ▼
//!   label ─► adjacency::Operation ─► vocab call byte ─► ogar_loco::Program (Quads body)
//!                                                            │
//!   VerifiedLogin ─► ReportAuthority ─► ReportScope           │  ogar_loco::Interpreter::run
//!                                           │                 ▼
//!                                           └────► FlowDialect ─► DirectoryObservations
//!                                                            │
//!                                                            ▼
//!                                                  Report ─► to_json (export boundary)
//! ```
//!
//! - [`adjacency`] — the label → typed operation table. Labels are metadata;
//!   the operation (effect, capability, shapes, backend, call byte) is
//!   authoritative. Aliases are several labels naming one operation.
//! - [`vocab`] — the flow [`Vocabulary`](ogar_loco::Vocabulary): six domain
//!   call bytes at `0x90..=0x95`, validated by `ogar-loco`'s conformance
//!   check.
//! - [`frontend`] — lowers the admitted read-only pipeline subset into one
//!   `ogar-loco` function body and refuses everything else.
//! - [`dialect`] — the [`Dialect`](ogar_loco::Dialect) the `ogar-loco`
//!   interpreter runs the body under.
//! - [`host`] — the two host seams: observed directory facts, and the report
//!   authority.
//!
//! `ogar-loco` is used unchanged: no new opcode, no change to its call ABI,
//! its interpreter or its conformance rules.
//!
//! The production authority (OGAR RBAC) and observation adapters (`ogar-az`
//! over Microsoft Graph) are not wired here; see the crate README.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod adjacency;
pub mod dialect;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod frontend;
pub mod host;
pub mod report;
pub mod vocab;

#[cfg(test)]
mod tests;

pub use adjacency::{
    Capability, CapabilitySet, Effect, LABELS, OPERATIONS, Operation, ResolveError, resolve,
};
pub use dialect::{FlowDialect, FlowError};
pub use frontend::{CompileError, CompiledFlow, TraceEntry, compile};
pub use host::{
    DirectoryObservations, FieldValue, ReportAuthority, ReportScope, SourceStamp, UserObservation,
};
pub use report::{Report, ReportRow};
pub use vocab::{FlowVocabulary, ReportField};

use ogar_auth::federation::VerifiedLogin;
use ogar_loco::{Interpreter, RunError};

/// Run a compiled flow for a verified login.
///
/// The actor is the login's canonical OGAR user. Its scope comes from
/// `authority`, before anything runs; an actor with no scope gets
/// [`FlowError::NoScope`] and the host is never queried.
///
/// # Errors
///
/// `RunError::Dialect` carrying a [`FlowError`] for a refused call or a
/// missing scope, or any other [`RunError`] the `ogar-loco` interpreter
/// reports. A failed run returns no report.
pub fn run_flow<H: DirectoryObservations, A: ReportAuthority>(
    login: &VerifiedLogin,
    flow: &CompiledFlow,
    host: &H,
    authority: &A,
) -> Result<Report, RunError<FlowError>> {
    let scope = authority
        .scope_for(login.authenticated_user())
        .ok_or(RunError::Dialect(FlowError::NoScope))?;
    let vocab = vocab::checked();
    if !flow.program.references_are_resolvable(&vocab) {
        return Err(RunError::Dialect(FlowError::NoReport));
    }
    let dialect = FlowDialect::new(host, &scope, flow);
    let mut interp = Interpreter::new(&vocab, &flow.program, dialect);
    interp.run()?;
    if !interp.stack().is_empty() {
        return Err(RunError::Dialect(FlowError::NoReport));
    }
    interp
        .dialect()
        .report()
        .cloned()
        .ok_or(RunError::Dialect(FlowError::NoReport))
}
