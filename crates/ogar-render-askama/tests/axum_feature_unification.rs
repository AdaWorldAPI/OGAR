//! **The `askama_axum` dependency is load-bearing and must not be tidied away.**
//!
//! This crate never calls `askama_axum`. A dependency-hygiene pass (`cargo
//! machete`, a "remove unused deps" cleanup, a reviewer asking "why is this
//! here?") will therefore read it as dead weight and delete it — and the
//! deletion is invisible, because every test in this repository still passes.
//!
//! It breaks CONSUMERS instead, and only those that combine two innocuous
//! choices:
//!
//! ```text
//! consumer  ──> askama_axum 0.4        (its own axum handlers)
//!           └─> ogar-render-askama     (this crate)
//!
//! askama_axum 0.4 enables askama/with-axum
//!           ↓  (cargo unifies features across the WHOLE graph)
//! askama's #[derive(Template)] now emits `impl IntoResponse` naming
//! `::askama_axum` — in every crate that derives Template, including this one
//!           ↓
//! error[E0433]: cannot find `askama_axum` in the crate root
//!   --> crates/ogar-render-askama/src/artifact_kinds/cells.rs:21
//! ```
//!
//! Measured 2026-08-20 wiring this crate into `AdaWorldAPI/MedCare-rs`'s
//! `medcare-server`, which has used `askama_axum = "0.4"` for its own handlers
//! all along. The consumer cannot fix it: the failure is inside THIS crate's
//! derives, so the only consumer-side escapes are forking or pinning, and the
//! workspace doctrine forbids both.
//!
//! # What this test can and cannot prove
//!
//! It CANNOT reproduce the failure: the trigger is a feature enabled by a
//! sibling package, and a test in this crate cannot enable it on the consumer's
//! behalf. Honest limitation, stated rather than dressed up.
//!
//! What it DOES do is make the dependency reachable from source, so removing it
//! fails to compile here instead of failing in a downstream repository weeks
//! later. That is the whole job.

/// Referenced so the dependency is used from source. Deleting the
/// `askama_axum` dependency breaks this line — which is the point.
#[test]
fn the_askama_axum_dependency_is_referenced_so_it_cannot_be_tidied_away() {
    use askama_axum as _;
    // The version that carries the `with-axum` feature this crate's derives
    // start depending on the moment any consumer enables it.
    assert!(!env!("CARGO_PKG_NAME").is_empty());
}
