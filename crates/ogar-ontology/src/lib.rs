//! `ogar-ontology` — prefix conventions and identity routing for OGAR.
//!
//! OGAR sits alongside OGIT in the same prefix-radix namespace. This
//! crate owns the prefix registry and the identity-string format that
//! `lance-graph-contract`'s `NiblePath` consumes for routing.
//!
//! # Prefix conventions
//!
//! ```text
//!   ogar/                    — the core OGAR vocabulary (Class, Association, …)
//!   ogar-extensions/<lang>/  — language-specific extensions
//!                              (e.g. ogar-extensions/odoo/ComputedField)
//!   ogit/                    — the OGIT baseline (IT-ops semantics)
//!   ogit-erp/                — ERP business semantics shared across Odoo / SAP / …
//!   ogit-op/                 — OpenProject business semantics
//!   ogit-<app>/              — application-specific extensions (GitLab, Redmine, …)
//! ```
//!
//! All prefixes share the same `/`-segmented radix routing in
//! `NiblePath` — class identities like `ogit-op/WorkPackage` descend the
//! same prefix tree as `ogit/IT/Application`, so cross-system queries
//! traverse one index, not many.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

/// Reserved top-level prefixes registered by OGAR.
///
/// Adding a new prefix is an ontology-level decision — extensions
/// should pick a deeper segment under an existing prefix
/// (`ogar-extensions/<lang>/...` or `ogit-<app>/...`) rather than
/// claiming a new top-level.
pub const RESERVED_PREFIXES: &[&str] = &["ogar", "ogar-extensions", "ogit", "ogit-erp", "ogit-op"];

/// The `ogar/` prefix for core vocabulary terms.
pub const OGAR_PREFIX: &str = "ogar";

/// The `ogar-extensions/` prefix for language-specific extension
/// vocabulary (Odoo `ComputedField`, `Delegation`, …).
pub const OGAR_EXTENSIONS_PREFIX: &str = "ogar-extensions";

/// Build the canonical OGAR identity for a class given a
/// per-application prefix and a class name. The result is suitable as
/// both the subject of triples and as the input to `NiblePath`.
///
/// # Examples
///
/// ```
/// use ogar_ontology::class_identity;
/// assert_eq!(class_identity("ogit-op", "WorkPackage"), "ogit-op/WorkPackage");
/// assert_eq!(class_identity("ogit-erp", "SaleOrder"), "ogit-erp/SaleOrder");
/// // Dotted names (Odoo) are preserved verbatim under the prefix.
/// assert_eq!(class_identity("ogit-erp", "sale.order"), "ogit-erp/sale.order");
/// ```
#[must_use]
pub fn class_identity(prefix: &str, class_name: &str) -> String {
    format!("{prefix}/{class_name}")
}

/// Build the canonical OGAR identity for a class field.
///
/// # Examples
///
/// ```
/// use ogar_ontology::field_identity;
/// assert_eq!(
///     field_identity("ogit-op", "WorkPackage", "subject"),
///     "ogit-op/WorkPackage.subject"
/// );
/// ```
#[must_use]
pub fn field_identity(prefix: &str, class_name: &str, field_name: &str) -> String {
    format!("{prefix}/{class_name}.{field_name}")
}

/// Build the canonical OGAR identity for an association edge.
///
/// # Examples
///
/// ```
/// use ogar_ontology::association_identity;
/// assert_eq!(
///     association_identity("ogit-op", "WorkPackage", "project"),
///     "ogit-op/WorkPackage->project"
/// );
/// ```
#[must_use]
pub fn association_identity(prefix: &str, class_name: &str, relation_name: &str) -> String {
    format!("{prefix}/{class_name}->{relation_name}")
}

/// Build a **versioned** class identity for ontology evolution and hot
/// reload addressing. The `@v<n>` suffix distinguishes vocabulary
/// versions of the same class so the runtime callcenter can route
/// in-flight messages to the old actor version while new messages go
/// to the new one.
///
/// Use the unversioned [`class_identity`] for the latest-version
/// addressing pattern; use this when versioned addressing is required
/// (callcenter dispatch, schema evolution audit, multi-version
/// queries).
///
/// # Examples
///
/// ```
/// use ogar_ontology::class_identity_versioned;
/// assert_eq!(
///     class_identity_versioned("ogit-op", "WorkPackage", 3),
///     "ogit-op/WorkPackage@v3"
/// );
/// ```
#[must_use]
pub fn class_identity_versioned(prefix: &str, class_name: &str, version: u64) -> String {
    format!("{prefix}/{class_name}@v{version}")
}

/// Build a **tenant-scoped** prefix for multi-tenancy isolation.
/// Routes the same canonical OGAR identity to a per-tenant slice
/// without leaking across tenants under the prefix-radix index.
///
/// The tenant segment leads, separated from the prefix by `.`, so
/// `acme.ogit-op/WorkPackage` and `globex.ogit-op/WorkPackage`
/// share the `ogit-op/WorkPackage` suffix for vocabulary lookup but
/// route to distinct dataset partitions under the radix.
///
/// # Examples
///
/// ```
/// use ogar_ontology::{tenant_prefix, class_identity};
/// let acme = tenant_prefix("acme", "ogit-op");
/// assert_eq!(acme, "acme.ogit-op");
/// assert_eq!(class_identity(&acme, "WorkPackage"), "acme.ogit-op/WorkPackage");
/// ```
#[must_use]
pub fn tenant_prefix(tenant: &str, prefix: &str) -> String {
    format!("{tenant}.{prefix}")
}

/// Returns `true` if `prefix` is a reserved top-level OGAR prefix.
#[must_use]
pub fn is_reserved_prefix(prefix: &str) -> bool {
    RESERVED_PREFIXES.contains(&prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_helpers() {
        assert_eq!(
            class_identity("ogit-op", "WorkPackage"),
            "ogit-op/WorkPackage"
        );
        assert_eq!(
            field_identity("ogit-op", "WorkPackage", "subject"),
            "ogit-op/WorkPackage.subject"
        );
        assert_eq!(
            association_identity("ogit-op", "WorkPackage", "project"),
            "ogit-op/WorkPackage->project"
        );
    }

    #[test]
    fn reserved_prefixes() {
        assert!(is_reserved_prefix("ogar"));
        assert!(is_reserved_prefix("ogit-op"));
        assert!(!is_reserved_prefix("random"));
    }

    #[test]
    fn dotted_names_preserved() {
        // Odoo class names like `sale.order` stay verbatim.
        assert_eq!(
            class_identity("ogit-erp", "sale.order"),
            "ogit-erp/sale.order"
        );
    }

    #[test]
    fn versioned_identity_format() {
        assert_eq!(
            class_identity_versioned("ogit-op", "WorkPackage", 1),
            "ogit-op/WorkPackage@v1"
        );
        assert_eq!(
            class_identity_versioned("ogit-op", "WorkPackage", 42),
            "ogit-op/WorkPackage@v42"
        );
    }

    #[test]
    fn tenant_prefix_isolation() {
        let acme = tenant_prefix("acme", "ogit-op");
        let globex = tenant_prefix("globex", "ogit-op");
        assert_ne!(
            class_identity(&acme, "WorkPackage"),
            class_identity(&globex, "WorkPackage")
        );
        // Same canonical suffix, distinct tenant prefixes.
        assert!(class_identity(&acme, "WorkPackage").ends_with("/WorkPackage"));
        assert!(class_identity(&globex, "WorkPackage").ends_with("/WorkPackage"));
    }
}
