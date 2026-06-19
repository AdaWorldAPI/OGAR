//! Per-render spec — the typed input each [`ArtifactKind`] emitter consumes.
//!
//! Mirror of `woa-rs::codegen::spec::RouteSpec` but for the canonical-layer
//! pipeline: the input is an [`ogar_vocab::Class`] (the calcified AR shape),
//! not a JSON-loaded `RouteSpec`. Codegen reads the class fns at build time
//! and dispatches each through an emitter for the chosen
//! [`ArtifactKind`].

use ogar_vocab::Class;

/// The set of target artifacts the render kit can emit per canonical class.
/// New kinds are appended (never reordered) so the dispatcher stays
/// backward-compatible. Mirrors WoA-rs's `HandlerKind` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactKind {
    /// Rust `struct` definition + `pub const CLASS_ID: u16` constant.
    RustStruct,
    /// TypeScript `interface` declaration + matching `class_ids.ts` entry.
    TsInterface,
    /// SurrealQL `DEFINE TABLE` + per-field `DEFINE FIELD` statements.
    SurrealqlTable,
    /// OpenAPI 3.1 `components.schemas.{Class}` JSON object.
    OpenapiSchema,
    /// Rust `match` arm dispatching on `ClassId` — useful for routing on
    /// `NodeGuid::classid` in graph consumers.
    NodeGuidRoutingArm,
}

impl ArtifactKind {
    /// All kinds in declaration order. Stable across additions (the
    /// `ArtifactKind` enum is treated append-only).
    pub const ALL: &'static [Self] = &[
        Self::RustStruct,
        Self::TsInterface,
        Self::SurrealqlTable,
        Self::OpenapiSchema,
        Self::NodeGuidRoutingArm,
    ];

    /// Human-readable short name — used in stub emitters' marker comments
    /// and in `cargo doc` text.
    pub fn name(self) -> &'static str {
        match self {
            Self::RustStruct => "rust_struct",
            Self::TsInterface => "ts_interface",
            Self::SurrealqlTable => "surrealql_table",
            Self::OpenapiSchema => "openapi_schema",
            Self::NodeGuidRoutingArm => "node_guid_routing_arm",
        }
    }
}

/// One render request: emit `class` as `kind`.
///
/// Borrow-based so a caller iterating the full codebook does not allocate
/// 32 Class copies per artifact kind.
#[derive(Debug, Clone, Copy)]
pub struct ArtifactSpec<'a> {
    /// The canonical class to render.
    pub class: &'a Class,
    /// Which target artifact to emit.
    pub kind: ArtifactKind,
}

impl<'a> ArtifactSpec<'a> {
    /// Pair a `class` with a target `kind`. No allocation; the `class`
    /// reference outlives the spec.
    pub fn new(class: &'a Class, kind: ArtifactKind) -> Self {
        Self { class, kind }
    }
}
