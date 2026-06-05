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
//! # Reference backends — VART (radix trie) or Lance dataset
//!
//! The trait admits any backend; two natural reference impls (a
//! deployment picks one — the firewall's outer-boundary pluggability):
//!
//! 1. **VART — timed adaptive radix trie** (`AdaWorldAPI/vart`,
//!    `vart 0.9.3`, the structure SurrealKV is built on; already in the
//!    surrealdb-fork dep tree via `surrealkv`). A near-perfect fit for
//!    the registry specifically:
//!    - **NiblePath-native** — VART *is* a (prefix) radix trie, and
//!      class identities (`ogit-op::WorkPackage`, …) are prefix-radix
//!      keys that share segments → compresses to the floor, exactly the
//!      `ARCHITECTURE.md` "compression to the floor" property.
//!    - **the trie version IS `knowable_from`** — VART is *timed* /
//!      versioned, so the version stamp the registry must return is the
//!      trie's own version. No separate counter.
//!    - **append-only-versioned = immutable audit** — the version
//!      history is a tamper-evident trail for free (the HIPAA need per
//!      `THE-FIREWALL.md §7.2`).
//!    - lightweight (a radix-trie crate, no protoc / heavy graph).
//!      Could land as a feature-gated reference impl *in this crate*
//!      (`--features vart-backend`) keeping the default trait-only; a
//!      ~20-minute outer-boundary addon (per the operator). Deferred to
//!      a focused follow-up that reads `AdaWorldAPI/vart`'s actual API
//!      first (no guessed-API cascade).
//! 2. **Lance dataset** (`ClassRegistryWriter` in
//!    `lance-graph-callcenter`, the runtime session's committed backend)
//!    — its own `AtomicU64` version counter + a dedicated `class_registry/`
//!    Lance dataset; unifies the registry with the substrate's main
//!    storage. Heavier; right when the registry should live alongside
//!    the rest of the Lance data.
//!
//! Both are **outer-boundary** stores (serialize at the write — that's
//! the firewall, per `THE-FIREWALL.md`). The trait is the seam; the
//! backend is the deployment choice. (OGIT can use the same VART-append
//! for its identity register — same pattern, same crate.)
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
/// Constructs the registration from the [`Class`] + the caller-supplied
/// **OGIT-prefixed canonical identity** + delegates the actual write to
/// a [`KnowableFromStore`] implementation.
///
/// # The `class_identity` parameter
///
/// The caller must supply the OGIT-prefixed canonical identity — e.g.
/// `"ogit-erp/sale.order"`, `"ogit-op/WorkPackage"`, `"ogit-healthcare/Patient"`.
/// Construct via [`ogar_ontology::class_identity(prefix, &class.name)`][1]
/// (which formats as `"{prefix}/{class_name}"`) — duplicated here as
/// `format!("{prefix}/{}", class.name)` if you prefer not to take the
/// `ogar-ontology` dep.
///
/// The prefix is REQUIRED because two producers may emit classes with
/// the same unqualified name under different application prefixes
/// (`ogit-op/WorkPackage` vs `ogit-erp/WorkPackage`). Keying the
/// registry by bare `class.name` would conflate them and return the
/// wrong `knowable_from` stamp on join — closed Codex P2 on PR #25.
///
/// # Validation
///
/// - `class.name` must be non-empty (IR-level invariant).
/// - `class_identity` must be non-empty (registry-key invariant).
///
/// Both produce [`KnowableFromError::MalformedClass`] before any store
/// call is made.
///
/// # Canonical-form invariant — caller's responsibility
///
/// This function does **NOT** canonicalize `class_identity` (no
/// case-fold, no slash-trim, no NiblePath normalization). The string
/// is passed through to `store.register(class_identity, …)` byte-
/// identical. The caller MUST use one consistent canonicalization
/// pipeline across all `register` *and* `knowable_from` lookup sites
/// — typically `ogar_ontology::class_identity(prefix, &class.name)` —
/// or distinct-but-equivalent identities (`Ogit-Op/WorkPackage` vs
/// `ogit-op/WorkPackage`) will register under separate keys.
///
/// # Architectural alignment
///
/// The prefix-radix shape of `class_identity` (`ogit-erp/sale.order/…`)
/// matches the **runtime-side trie-append surface** named in
/// `bardioc` PR #18 / `lance-graph` PR #470: *"Rubicon's
/// `LanceMembrane::commit_event` keyed on `inv.object_instance`
/// becomes a trie append; standing-wave reads at `v_ref =
/// traverse_subtree(prefix, v_ref)`."* The OGAR-side registry shape
/// is the producer mirror of that consumer surface — same
/// `NiblePath` identity, same VART-as-reference-backend pattern
/// (see crate-level "Reference backends").
///
/// # `schema_ddl_hint` — the self-describing-registry loop
///
/// With the **`surrealql-hint` feature ON**, this function renders the
/// SurrealQL DDL for the class (via
/// `ogar-adapter-surrealql::emit_surrealql_ddl`) and passes it to
/// `store.register(class_identity, Some(ddl))`. The registry then
/// carries the producer's view of the class shape alongside the
/// `knowable_from` stamp — *"the registry is self-describing"* per
/// the `KnowableFromStore::register` docstring, no longer aspirational.
///
/// With the feature OFF (the default), `None` is passed — the
/// lightweight path. The feature is opt-in because the
/// `ogar-adapter-surrealql` dep pulls the SurrealDB AST surface into
/// the build graph.
///
/// Aligns with ADR-023 (IR-as-wire-truth): the canonical `Class` IR
/// is the wire-truth carrier; the DDL string is its serialization for
/// the registry. See `docs/ARCHITECTURAL-DECISIONS-2026-06-04.md`.
///
/// [1]: https://docs.rs/ogar-ontology
pub fn register_class_knowable_from<S: KnowableFromStore>(
    class: &Class,
    class_identity: &str,
    store: &S,
) -> Result<u64, KnowableFromError> {
    if class.name.is_empty() {
        return Err(KnowableFromError::MalformedClass(
            "Class.name is empty; refusing to register".into(),
        ));
    }
    if class_identity.is_empty() {
        return Err(KnowableFromError::MalformedClass(
            "class_identity is empty; pass an OGIT-prefixed canonical \
             identity such as \"ogit-erp/sale.order\" (see \
             ogar_ontology::class_identity)".into(),
        ));
    }
    // The `schema_ddl_hint` parameter — `None` by default (lightweight
    // path), auto-rendered via `ogar-adapter-surrealql::emit_surrealql_ddl`
    // when the `surrealql-hint` feature is on. This closes the loop the
    // `KnowableFromStore::register` docstring named: the trait carries a
    // `schema_ddl_hint: Option<&str>` slot so the registry is
    // self-describing; PR #32 landed the producer; this is where it
    // gets wired into the call site.
    #[cfg(feature = "surrealql-hint")]
    {
        let ddl = ogar_adapter_surrealql::emit_surrealql_ddl(std::slice::from_ref(class));
        store.register(class_identity, Some(ddl.as_str()))
    }
    #[cfg(not(feature = "surrealql-hint"))]
    {
        let _ = class; // keep parameter live; not used in the default path
        store.register(class_identity, None)
    }
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
        let v = register_class_knowable_from(&c, "ogit-erp/Account", &store)
            .expect("register OK");
        assert_eq!(v, 42);
        let calls = store.register_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "ogit-erp/Account");
        // The schema_ddl_hint axis is feature-gated: dedicated tests
        // for each path are below (`default_path_passes_none_for_…` and
        // `surrealql_hint_feature_renders_ddl_into_registry`). This
        // test stays axis-agnostic on the hint.
    }

    #[test]
    fn knowable_from_lookup_after_register_returns_same_version() {
        // The trait's READ side is the meet-point's other half — callers
        // of temporal::classify fetch knowable_from via the same store.
        let c = Class::new("WorkPackage");
        let store = MockKnowableFromStore::new(7);
        let v = register_class_knowable_from(&c, "ogit-op/WorkPackage", &store).unwrap();
        assert_eq!(store.knowable_from("ogit-op/WorkPackage"), Some(v));
    }

    #[test]
    fn knowable_from_lookup_unregistered_returns_none() {
        let store = MockKnowableFromStore::new(0);
        assert_eq!(store.knowable_from("ogit-erp/NotRegistered"), None);
    }

    #[test]
    fn same_class_name_under_different_prefixes_does_not_collide() {
        // The Codex P2 motivating case: two producers emit a class
        // with the same unqualified name (e.g. `WorkPackage`) under
        // different OGIT prefixes. Keying by bare class.name would
        // conflate them; the new prefix-aware identity keeps them
        // distinct.
        let store = MockKnowableFromStore::new(50);
        let v_op = register_class_knowable_from(
            &Class::new("WorkPackage"),
            "ogit-op/WorkPackage",
            &store,
        ).unwrap();
        let v_erp = register_class_knowable_from(
            &Class::new("WorkPackage"),
            "ogit-erp/WorkPackage",
            &store,
        ).unwrap();
        assert_ne!(v_op, v_erp, "different prefixes must produce distinct versions");
        assert_eq!(store.knowable_from("ogit-op/WorkPackage"), Some(v_op));
        assert_eq!(store.knowable_from("ogit-erp/WorkPackage"), Some(v_erp));
    }

    #[test]
    fn register_empty_class_name_rejects_without_storing() {
        let c = Class::new("");
        let store = MockKnowableFromStore::new(0);
        match register_class_knowable_from(&c, "ogit-erp/X", &store) {
            Err(KnowableFromError::MalformedClass(msg)) => {
                assert!(msg.contains("Class.name"), "expected Class.name message, got: {msg}");
            }
            other => panic!("expected MalformedClass, got: {other:?}"),
        }
        // Confirm the store was NOT touched (validation rejected before write).
        assert_eq!(store.register_calls.lock().unwrap().len(), 0);
    }

    #[test]
    fn register_empty_class_identity_rejects_without_storing() {
        let c = Class::new("Account");
        let store = MockKnowableFromStore::new(0);
        match register_class_knowable_from(&c, "", &store) {
            Err(KnowableFromError::MalformedClass(msg)) => {
                assert!(msg.contains("class_identity"),
                    "expected class_identity message, got: {msg}");
            }
            other => panic!("expected MalformedClass, got: {other:?}"),
        }
        assert_eq!(store.register_calls.lock().unwrap().len(), 0);
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
        match register_class_knowable_from(&c, "ogit-erp/X", &FailingStore) {
            Err(KnowableFromError::Backend(msg)) => assert_eq!(msg, "disk full"),
            other => panic!("expected propagated Backend error, got: {other:?}"),
        }
    }

    #[test]
    fn register_multiple_classes_gets_monotonic_versions() {
        let store = MockKnowableFromStore::new(100);
        let v1 = register_class_knowable_from(&Class::new("A"), "ogit-erp/A", &store).unwrap();
        let v2 = register_class_knowable_from(&Class::new("B"), "ogit-erp/B", &store).unwrap();
        let v3 = register_class_knowable_from(&Class::new("C"), "ogit-erp/C", &store).unwrap();
        assert_eq!(v1, 100);
        assert_eq!(v2, 101);
        assert_eq!(v3, 102);
        // Lookups return each class's registered version.
        assert_eq!(store.knowable_from("ogit-erp/A"), Some(100));
        assert_eq!(store.knowable_from("ogit-erp/B"), Some(101));
        assert_eq!(store.knowable_from("ogit-erp/C"), Some(102));
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

    // ── `schema_ddl_hint` loop closure (ADR-023 / IR-as-wire-truth) ──
    // Feature-off (default): `None` is passed to store.register, the
    // lightweight path. Feature-on (`surrealql-hint`): the function
    // renders the DDL via `ogar-adapter-surrealql::emit_surrealql_ddl`
    // and passes it as `Some(&ddl)`. Two tests, conditionally compiled.
    // ─────────────────────────────────────────────────────────────────

    #[cfg(not(feature = "surrealql-hint"))]
    #[test]
    fn default_path_passes_none_for_schema_ddl_hint() {
        let c = Class::new("Account");
        let store = MockKnowableFromStore::new(0);
        register_class_knowable_from(&c, "ogit-erp/Account", &store).unwrap();
        let calls = store.register_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        // Default path: the registry receives no DDL hint.
        assert!(
            calls[0].1.is_none(),
            "default (feature-off) path must pass None for schema_ddl_hint, got: {:?}",
            calls[0].1
        );
    }

    #[cfg(feature = "surrealql-hint")]
    #[test]
    fn surrealql_hint_feature_renders_ddl_into_registry() {
        // With the `surrealql-hint` feature on, the registry receives
        // the SurrealQL DDL rendering of the class — the "self-
        // describing registry" claim from KnowableFromStore::register's
        // docstring becomes concrete.
        let mut c = Class::new("Account");
        let mut email = ogar_vocab::Attribute::new("email");
        email.type_name = Some("string".into());
        c.attributes.push(email);

        let store = MockKnowableFromStore::new(0);
        register_class_knowable_from(&c, "ogit-erp/Account", &store).unwrap();

        let calls = store.register_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let hint = calls[0].1.as_deref().expect("feature-on path must populate the hint");
        // The DDL must mention the table and the field — the canonical
        // shape `emit_surrealql_ddl` produces.
        assert!(
            hint.contains("DEFINE TABLE Account SCHEMAFULL;"),
            "expected DEFINE TABLE in the hint, got: {hint}"
        );
        assert!(
            hint.contains("DEFINE FIELD email ON Account TYPE string;"),
            "expected DEFINE FIELD in the hint, got: {hint}"
        );
    }
}
