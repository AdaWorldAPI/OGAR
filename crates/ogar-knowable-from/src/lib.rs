//! `ogar-knowable-from` — the OGAR-side producer seam for the §10.3
//! `knowable_from` meet-point.
//!
//! # The seam
//!
//! Per `docs/OPENPROJECT-TRANSCODING.md §10.3` (the authoritative pin) +
//! `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` ADR-010: the SurrealQL
//! frame's `knowable_from` is **sourced by an OGAR producer** (this
//! crate) and **consumed by `lance-graph-planner::temporal::classify`**
//! (runtime session, shipped in lance-graph PR #468).
//!
//! Single ownership: **nowhere else** in the substrate owns either side
//! of this seam.
//!
//! # Trait-mediated
//!
//! This crate stays **Lance-free**: it defines the [`KnowableFromWriter`]
//! trait + the [`register_class_knowable_from`] generic. The runtime
//! side (typically `lance-graph-callcenter`'s `LanceMembrane`, which is
//! the sole-writer surface per ADR-008) provides
//! `impl KnowableFromWriter for LanceMembrane` in a small follow-up.
//!
//! ```text
//!  OGAR producer side                  Runtime / membrane side
//!  ──────────────────                  ──────────────────────
//!     ogar-knowable-from   ──trait──►  lance-graph-callcenter::LanceMembrane
//!     (this crate)                     impl KnowableFromWriter
//!         │                                     │
//!         │ register_class_knowable_from        │ commit_event(row) -> u64
//!         │   builds registration record        │   appends Lance row
//!         │   delegates to writer               │   returns new version
//!         ▼                                     ▼
//!     Class → write_class_registration → u64 (the `knowable_from` stamp)
//! ```
//!
//! Future: `register_class_knowable_from` can be extended to render the
//! schema DDL via `ogar-adapter-surrealql::emit_surrealql_ddl(&[class])`
//! and pass it as `schema_ddl` so the registry is self-describing. The
//! `schema_ddl: Option<&str>` parameter is reserved for that.
//!
//! # Why a separate crate (not in `ogar-adapter-surrealql`)
//!
//! Architectural symmetry with `CommitHook` (the runtime side's
//! Lance-write seam, also trait-mediated to keep the membrane opaque to
//! consumers). Also: keeps `ogar-adapter-surrealql` lightweight — that
//! crate's deps stay on `ogar-vocab` + the optional `surrealdb-parser`
//! feature; `ogar-knowable-from` is even lighter (only `ogar-vocab` +
//! optional `serde`).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use ogar_vocab::Class;

/// The §10.3 producer-side seam — write a class registration into the
/// schema-registry Lance dataset, returning the new monotonic Lance
/// version. The returned `u64` becomes the `knowable_from` value
/// `lance-graph-planner::temporal::classify(_, knowable_from, _)`
/// consumes.
///
/// The runtime backend (typically `lance-graph-callcenter::LanceMembrane`)
/// implements this trait. OGAR stays Lance-free.
pub trait KnowableFromWriter {
    /// Persist the class registration. Returns the resulting Lance
    /// version (the `knowable_from` stamp).
    ///
    /// `class_identity` is the OGIT-prefixed canonical identity for the
    /// class (e.g. `"ogit-op::WorkPackage"`).
    ///
    /// `schema_ddl` is an optional SurrealQL DDL rendering of the class
    /// (typically via `ogar-adapter-surrealql::emit_surrealql_ddl`), so
    /// the registry is self-describing. `None` for the v1 minimum-shape
    /// path; consumers that want the schema in the registry row can
    /// pass it in.
    fn write_class_registration(
        &self,
        class_identity: &str,
        schema_ddl: Option<&str>,
    ) -> Result<u64, WriteError>;
}

/// Register a class with the substrate's schema registry and return its
/// `knowable_from` stamp.
///
/// Constructs the registration data from the [`Class`] + delegates the
/// actual write to a [`KnowableFromWriter`] implementation. Validates
/// the class has a non-empty name (an empty-name class is malformed at
/// the IR level and shouldn't reach the registry).
pub fn register_class_knowable_from<W: KnowableFromWriter>(
    class: &Class,
    writer: &W,
) -> Result<u64, RegisterError> {
    if class.name.is_empty() {
        return Err(RegisterError::MalformedClass(
            "Class.name is empty; refusing to register".into(),
        ));
    }
    let identity = class_identity_string(class);
    // v1 minimum-shape: pass None for schema_ddl. Future PRs can render
    // via ogar-adapter-surrealql::emit_surrealql_ddl(&[class.clone()]).
    writer
        .write_class_registration(&identity, None)
        .map_err(RegisterError::Write)
}

/// Compute the OGAR-canonical identity string for a [`Class`].
///
/// For v1, returns `class.name` (the unqualified class name). Future
/// versions extend this to include prefix segments (per
/// `docs/IDENTITY-MAPPING.md`'s canonical Identity grammar); the
/// minimum-shape v1 keeps the registry working with whatever name the
/// producer populated.
fn class_identity_string(class: &Class) -> String {
    // TODO(identity): when ogar-vocab grows a typed Identity carrier
    // for Class (today it has `identity: String` + `name: String`),
    // prefer `class.identity` when populated, fall back to `class.name`.
    // For v1 minimum, just `name`.
    class.name.clone()
}

/// Errors from [`register_class_knowable_from`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegisterError {
    /// The class is missing required IR fields (e.g. empty name).
    MalformedClass(String),
    /// The underlying writer failed.
    Write(WriteError),
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegisterError::MalformedClass(msg) => {
                write!(f, "malformed class for registration: {msg}")
            }
            RegisterError::Write(e) => write!(f, "registration write failed: {e}"),
        }
    }
}

impl std::error::Error for RegisterError {}

/// Errors from [`KnowableFromWriter::write_class_registration`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WriteError {
    /// The Lance dataset / backend write failed.
    Backend(String),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::Backend(msg) => write!(f, "backend write error: {msg}"),
        }
    }
}

impl std::error::Error for WriteError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Mock writer for tests — records every call + returns sequential
    /// versions starting from `start_version`.
    struct MockKnowableFromWriter {
        calls: RefCell<Vec<(String, Option<String>)>>,
        next_version: RefCell<u64>,
    }

    impl MockKnowableFromWriter {
        fn new(start_version: u64) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                next_version: RefCell::new(start_version),
            }
        }
    }

    impl KnowableFromWriter for MockKnowableFromWriter {
        fn write_class_registration(
            &self,
            class_identity: &str,
            schema_ddl: Option<&str>,
        ) -> Result<u64, WriteError> {
            self.calls.borrow_mut().push((
                class_identity.to_string(),
                schema_ddl.map(String::from),
            ));
            let v = *self.next_version.borrow();
            *self.next_version.borrow_mut() = v + 1;
            Ok(v)
        }
    }

    #[test]
    fn register_simple_class_returns_writer_version() {
        let c = Class::new("Account");
        let writer = MockKnowableFromWriter::new(42);
        let v = register_class_knowable_from(&c, &writer).expect("register OK");
        assert_eq!(v, 42);
        let calls = writer.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "Account");
        assert!(calls[0].1.is_none(), "v1 minimum-shape passes None schema");
    }

    #[test]
    fn register_empty_class_name_rejects_without_writing() {
        let c = Class::new("");
        let writer = MockKnowableFromWriter::new(0);
        match register_class_knowable_from(&c, &writer) {
            Err(RegisterError::MalformedClass(msg)) => {
                assert!(msg.contains("empty"), "expected empty-name message, got: {msg}");
            }
            other => panic!("expected MalformedClass, got: {other:?}"),
        }
        // Confirm the writer was NOT invoked (validation rejected before write).
        assert_eq!(writer.calls.borrow().len(), 0);
    }

    #[test]
    fn register_propagates_writer_errors() {
        struct FailingWriter;
        impl KnowableFromWriter for FailingWriter {
            fn write_class_registration(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<u64, WriteError> {
                Err(WriteError::Backend("disk full".into()))
            }
        }
        let c = Class::new("X");
        match register_class_knowable_from(&c, &FailingWriter) {
            Err(RegisterError::Write(WriteError::Backend(msg))) => assert_eq!(msg, "disk full"),
            other => panic!("expected propagated WriteError, got: {other:?}"),
        }
    }

    #[test]
    fn register_multiple_classes_gets_monotonic_versions() {
        let writer = MockKnowableFromWriter::new(100);
        let v1 = register_class_knowable_from(&Class::new("A"), &writer).unwrap();
        let v2 = register_class_knowable_from(&Class::new("B"), &writer).unwrap();
        let v3 = register_class_knowable_from(&Class::new("C"), &writer).unwrap();
        assert_eq!(v1, 100);
        assert_eq!(v2, 101);
        assert_eq!(v3, 102);
        let calls = writer.calls.borrow();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, "A");
        assert_eq!(calls[1].0, "B");
        assert_eq!(calls[2].0, "C");
    }

    #[test]
    fn errors_display_meaningfully() {
        let m = RegisterError::MalformedClass("name is empty".into());
        assert!(format!("{m}").contains("malformed class"));
        let w = RegisterError::Write(WriteError::Backend("nope".into()));
        assert!(format!("{w}").contains("registration write failed"));
        assert!(format!("{w}").contains("nope"));
    }
}
