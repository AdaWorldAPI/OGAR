//! `ogar-knowable-from` — the OGAR-side producer seam for the §10.3
//! `knowable_from` meet-point.
//!
//! # The seam
//!
//! Per `docs/OPENPROJECT-TRANSCODING.md §10.3` (the authoritative pin) +
//! `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md` ADR-010: the SurrealQL
//! frame's `knowable_from` is **sourced by an OGAR producer** (this
//! crate's [`KnowableFromStore`] trait) and **consumed by
//! `lance-graph-planner::temporal::classify`** (runtime session, shipped
//! in lance-graph PR #468).
//!
//! Single ownership: **nowhere else** in the substrate owns either side
//! of this seam.
//!
//! # Trait-mediated, store (read+write) not just writer
//!
//! This crate stays **Lance-free**: it defines the [`KnowableFromStore`]
//! trait + the [`register_class_knowable_from`] generic. The trait
//! covers **both** register (write) and `knowable_from` (read) because
//! callers of `temporal::classify(row_version, knowable_from, v_ref)`
//! need to *look up* the registered `knowable_from` for a class —
//! hiding the lookup behind a separate seam would just defer the
//! question.
//!
//! The runtime side provides the reference backend (a small
//! `ClassRegistryWriter` type or sibling crate in `lance-graph`, owning
//! its **own** Lance dataset — see "Architecture note" below).
//!
//! ```text
//!  OGAR producer side                  Runtime / store side
//!  ──────────────────                  ─────────────────────
//!     ogar-knowable-from   ──trait──►  lance-graph class registry
//!     (this crate)                     impl KnowableFromStore
//!         │                                     │
//!         │ register_class_knowable_from        │ register(id, hint) -> u64
//!         │   builds registration record        │   appends to dedicated
//!         │   delegates to store                │   class-registry Lance dataset
//!         ▼                                     │   ticks its own version counter
//!     Class → store.register → u64 (the `knowable_from` stamp)
//!
//!     consumer (e.g. Rubicon, planner)
//!         │
//!         │ store.knowable_from(class_id) -> Option<u64>
//!         │ temporal::classify(row_version, knowable_from, v_ref)
//!         ▼
//!     Classification {Contemporary, Anachronistic, Spoiler, Unknowable}
//! ```
//!
//! # Architecture note — **NOT** backed by `LanceMembrane::commit_event`
//!
//! Tempting to reuse `LanceMembrane::commit_event` (the action-commit
//! sole-writer per ADR-008) for class registration, but **two structural
//! reasons not to**:
//!
//! 1. **Schema mismatch.** `CognitiveEventRow` was designed for action
//!    commits + cognitive cycles (`gate_commit`, `cycle_fp_hi/lo`,
//!    `external_role`, `faculty_role`, `nars_*`). A class registration
//!    is neither — forcing it through means stamping `gate_commit=true`
//!    on a row that isn't a commit, role fields meaningless.
//! 2. **Independent event streams should have independent versions.**
//!    `LanceMembrane`'s version counter ticks on every action commit;
//!    if schema registrations also tick it, then `knowable_from` for
//!    class X becomes coupled to "whatever action committed next." The
//!    two are genuinely orthogonal — `temporal::classify` takes
//!    `row_version` and `knowable_from` as **separate arguments**
//!    because they advance on different timelines.
//!
//! ADR-008 says "`commit_event` is the action-commit sole-writer." It
//! is **not** "the sole-writer for everything that wants a monotonic
//! Lance version." Schema registration is a different concern with the
//! same *mechanism* (append + version), not the same *purpose*.
//!
//! Reference backend therefore: a **separate Lance dataset** owned by a
//! small `ClassRegistryWriter` type in `lance-graph-callcenter` (or
//! sibling crate `lance-graph-class-registry`), with its own monotonic
//! version counter (`AtomicU64`, same pattern as `LanceMembrane::version`
//! but independent).
//!
//! # Pluggable backends — why the trait matters for the SDK endgame
//!
//! Per `SUBSTRATE-ENDGAME.md §5`, Room 5 is Foundry-OSS-class capability.
//! Foundry-style deployments need backend flexibility — different
//! tenants may want different schema-registry substrates (Lance for the
//! substrate-b reference, Postgres for a "use my existing infrastructure"
//! deployment, etc.). The trait is the SDK seam; the Lance impl is the
//! reference. Direct-Lance ownership in OGAR would foreclose that.
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

/// The §10.3 producer-side store seam — read + write of the
/// `(class_identity → knowable_from)` mapping.
///
/// Both methods so callers of
/// `lance-graph-planner::temporal::classify(row_version, knowable_from, v_ref)`
/// can fetch the `knowable_from` argument from the same trait that the
/// registering producer uses to write it.
///
/// The runtime backend (typically the `lance-graph-callcenter` /
/// `lance-graph-class-registry` reference impl) owns its **own** Lance
/// dataset — separate from `LanceMembrane`'s cognitive-cycle dataset
/// (see crate-level "Architecture note" for why).
pub trait KnowableFromStore: Send + Sync {
    /// Register `class_identity` and return its `knowable_from` —
    /// the new monotonic version of the schema-registry dataset.
    ///
    /// `schema_ddl_hint` is an optional SurrealQL DDL rendering of the
    /// class (typically via `ogar-adapter-surrealql::emit_surrealql_ddl`)
    /// so the registry is self-describing. `None` for the v1
    /// minimum-shape path.
    fn register(
        &self,
        class_identity: &str,
        schema_ddl_hint: Option<&str>,
    ) -> Result<u64, KnowableFromError>;

    /// Look up the `knowable_from` for a registered class. Used by
    /// callers of `temporal::classify` to fetch the parameter.
    ///
    /// Returns `None` if the class has never been registered.
    fn knowable_from(&self, class_identity: &str) -> Option<u64>;
}

/// Register a class with the substrate's schema registry and return its
/// `knowable_from` stamp.
///
/// Constructs the registration data from the [`Class`] + delegates the
/// actual write to a [`KnowableFromStore`] implementation. Validates
/// the class has a non-empty name (an empty-name class is malformed at
/// the IR level and shouldn't reach the registry).
pub fn register_class_knowable_from<S: KnowableFromStore>(
    class: &Class,
    store: &S,
) -> Result<u64, KnowableFromError> {
    if class.name.is_empty() {
        return Err(KnowableFromError::MalformedClass(
            "Class.name is empty; refusing to register".into(),
        ));
    }
    let identity = class_identity_string(class);
    // v1 minimum-shape: pass None for schema_ddl_hint. Future PRs can
    // render via ogar-adapter-surrealql::emit_surrealql_ddl(&[class.clone()]).
    store.register(&identity, None)
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

/// Errors from the [`KnowableFromStore`] operations and the
/// [`register_class_knowable_from`] helper.
///
/// Consolidated single error type (vs split register/write errors) —
/// callers handle one error surface for the whole §10.3 producer-side
/// concern.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum KnowableFromError {
    /// The class is missing required IR fields (e.g. empty name).
    /// Detected by [`register_class_knowable_from`] before any store
    /// call is made.
    MalformedClass(String),
    /// The store backend (typically a Lance dataset write) failed.
    Backend(String),
}

impl std::fmt::Display for KnowableFromError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnowableFromError::MalformedClass(msg) => {
                write!(f, "malformed class for registration: {msg}")
            }
            KnowableFromError::Backend(msg) => {
                write!(f, "knowable-from store backend error: {msg}")
            }
        }
    }
}

impl std::error::Error for KnowableFromError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock store for tests — records every register call, maintains an
    /// in-memory `(class_identity → knowable_from)` map, and returns
    /// sequential versions starting from `start_version`.
    ///
    /// `Mutex` (not `RefCell`) because the trait bounds `Send + Sync`
    /// and we want the Mock to satisfy them too.
    struct MockKnowableFromStore {
        registry: Mutex<HashMap<String, u64>>,
        register_calls: Mutex<Vec<(String, Option<String>)>>,
        next_version: Mutex<u64>,
    }

    impl MockKnowableFromStore {
        fn new(start_version: u64) -> Self {
            Self {
                registry: Mutex::new(HashMap::new()),
                register_calls: Mutex::new(Vec::new()),
                next_version: Mutex::new(start_version),
            }
        }
    }

    impl KnowableFromStore for MockKnowableFromStore {
        fn register(
            &self,
            class_identity: &str,
            schema_ddl_hint: Option<&str>,
        ) -> Result<u64, KnowableFromError> {
            self.register_calls
                .lock()
                .unwrap()
                .push((class_identity.to_string(), schema_ddl_hint.map(String::from)));
            let mut next = self.next_version.lock().unwrap();
            let v = *next;
            *next = v + 1;
            self.registry
                .lock()
                .unwrap()
                .insert(class_identity.to_string(), v);
            Ok(v)
        }

        fn knowable_from(&self, class_identity: &str) -> Option<u64> {
            self.registry.lock().unwrap().get(class_identity).copied()
        }
    }

    #[test]
    fn register_simple_class_returns_store_version() {
        let c = Class::new("Account");
        let store = MockKnowableFromStore::new(42);
        let v = register_class_knowable_from(&c, &store).expect("register OK");
        assert_eq!(v, 42);
        let calls = store.register_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "Account");
        assert!(calls[0].1.is_none(), "v1 minimum-shape passes None for schema_ddl_hint");
    }

    #[test]
    fn knowable_from_lookup_after_register_returns_same_version() {
        // The trait's READ side is the meet-point's other half — callers
        // of temporal::classify fetch knowable_from via the same store.
        let c = Class::new("WorkPackage");
        let store = MockKnowableFromStore::new(7);
        let v = register_class_knowable_from(&c, &store).unwrap();
        assert_eq!(store.knowable_from("WorkPackage"), Some(v));
    }

    #[test]
    fn knowable_from_lookup_unregistered_returns_none() {
        let store = MockKnowableFromStore::new(0);
        assert_eq!(store.knowable_from("NotRegistered"), None);
    }

    #[test]
    fn register_empty_class_name_rejects_without_storing() {
        let c = Class::new("");
        let store = MockKnowableFromStore::new(0);
        match register_class_knowable_from(&c, &store) {
            Err(KnowableFromError::MalformedClass(msg)) => {
                assert!(msg.contains("empty"), "expected empty-name message, got: {msg}");
            }
            other => panic!("expected MalformedClass, got: {other:?}"),
        }
        // Confirm the store was NOT touched (validation rejected before write).
        assert_eq!(store.register_calls.lock().unwrap().len(), 0);
        assert_eq!(store.knowable_from(""), None);
    }

    #[test]
    fn register_propagates_backend_errors() {
        struct FailingStore;
        impl KnowableFromStore for FailingStore {
            fn register(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<u64, KnowableFromError> {
                Err(KnowableFromError::Backend("disk full".into()))
            }
            fn knowable_from(&self, _: &str) -> Option<u64> {
                None
            }
        }
        let c = Class::new("X");
        match register_class_knowable_from(&c, &FailingStore) {
            Err(KnowableFromError::Backend(msg)) => assert_eq!(msg, "disk full"),
            other => panic!("expected propagated Backend error, got: {other:?}"),
        }
    }

    #[test]
    fn register_multiple_classes_gets_monotonic_versions() {
        let store = MockKnowableFromStore::new(100);
        let v1 = register_class_knowable_from(&Class::new("A"), &store).unwrap();
        let v2 = register_class_knowable_from(&Class::new("B"), &store).unwrap();
        let v3 = register_class_knowable_from(&Class::new("C"), &store).unwrap();
        assert_eq!(v1, 100);
        assert_eq!(v2, 101);
        assert_eq!(v3, 102);
        // Lookups return each class's registered version.
        assert_eq!(store.knowable_from("A"), Some(100));
        assert_eq!(store.knowable_from("B"), Some(101));
        assert_eq!(store.knowable_from("C"), Some(102));
        let calls = store.register_calls.lock().unwrap();
        assert_eq!(calls.len(), 3);
    }

    #[test]
    fn errors_display_meaningfully() {
        let m = KnowableFromError::MalformedClass("name is empty".into());
        assert!(format!("{m}").contains("malformed class"));
        let b = KnowableFromError::Backend("nope".into());
        assert!(format!("{b}").contains("backend error"));
        assert!(format!("{b}").contains("nope"));
    }
}
