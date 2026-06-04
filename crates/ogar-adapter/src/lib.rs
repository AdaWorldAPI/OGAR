//! `ogar-adapter` — the `Adapter` trait and `BTreeMapAdapter` default
//! implementation for cross-language identity translation.
//!
//! Per `docs/ADAPTERS-AND-ACTORS.md` §2: each Adapter is a static
//! lookup table mapping canonical OGAR identity paths to target-form
//! identity strings. The adapter knows POSITION, not MEANING.
//!
//! Sprint 3 ships:
//! - `Adapter` trait with `map()` direction only (`unmap()` deferred to
//!   Sprint 4.5 when SurrealQL parser needs reverse direction).
//! - `BTreeMapAdapter` — a plain `BTreeMap<String, String>` lookup.
//!   Custom `NibleHHTL` data structure deferred until benchmark
//!   evidence demands it (B3 YAGNI carve-out).
//! - `OdooAdapter` — first concrete adapter with leaves for common
//!   OGAR-to-Odoo identity translations.
//!
//! # Example
//!
//! ```
//! use ogar_adapter::{Adapter, OdooAdapter};
//!
//! let odoo = OdooAdapter::new();
//! let canonical = "ogit-erp/sale.order".to_string();
//! assert!(odoo.map(&canonical).is_some());
//! ```

#![warn(missing_docs)]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The Adapter trait — converts a canonical OGAR identity string into
/// the form expected by a specific target language or system.
///
/// Stateful (carries a `BTreeMap` lookup table per instance) so future
/// adapters can hold configuration (e.g. schema name for a Postgres
/// adapter, currency-field convention for an Odoo adapter). The map
/// itself is the static lookup; the struct owns it.
///
/// Reverse direction (`unmap()`) is intentionally not in the trait
/// for v1 — Sprint 4.5 (SurrealQL bidirectional) will introduce it.
pub trait Adapter {
    /// Map a canonical OGAR identity string to its target form.
    /// Returns `None` when no mapping is registered.
    fn map(&self, canonical: &str) -> Option<String>;

    /// Adapter's name, for logging / debugging.
    fn name(&self) -> &'static str;
}

/// A `BTreeMap`-backed adapter — simple, ordered prefix iteration via
/// `BTreeMap::range`, no custom data structure. Per B3 YAGNI carve-out:
/// use the standard library until a benchmark proves it's the bottleneck.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BTreeMapAdapter {
    /// The adapter's name (for diagnostics).
    name: &'static str,
    /// The lookup table: canonical → target.
    pub leaves: BTreeMap<String, String>,
}

impl BTreeMapAdapter {
    /// Build a new empty adapter with the given name.
    #[must_use]
    pub fn new(name: &'static str) -> Self {
        Self { name, leaves: BTreeMap::new() }
    }

    /// Insert a (canonical → target) leaf.
    pub fn insert(&mut self, canonical: impl Into<String>, target: impl Into<String>) {
        self.leaves.insert(canonical.into(), target.into());
    }

    /// Iterate leaves whose canonical path starts with `prefix`.
    /// Used by emitters that want to bulk-translate all paths under
    /// a class (e.g. all `ogit-erp/sale.order/*` paths).
    pub fn iter_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = (&'a String, &'a String)> + 'a {
        self.leaves
            .iter()
            .filter(move |(k, _)| k.starts_with(prefix))
    }
}

impl Adapter for BTreeMapAdapter {
    fn map(&self, canonical: &str) -> Option<String> {
        self.leaves.get(canonical).cloned()
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

/// The Odoo adapter — maps OGAR canonical identities under
/// `ogit-erp/*` to their Odoo-source equivalents.
///
/// First leaves cover the worked examples in `docs/ADAPTERS-AND-ACTORS.md`
/// §3 (e.g. `move ↔ transport`, `action_confirm ↔ confirm`). Additional
/// leaves are added by Sprint 4 (`ogar-python` producer) when real Odoo
/// source is transcoded.
#[derive(Debug, Clone)]
pub struct OdooAdapter {
    inner: BTreeMapAdapter,
}

impl OdooAdapter {
    /// Build the default OdooAdapter with the initial leaf set.
    /// Producers add more leaves via `register()`.
    #[must_use]
    pub fn new() -> Self {
        let mut inner = BTreeMapAdapter::new("OdooAdapter");
        // Class-level renames (the carved "stupid example" from the
        // ADAPTERS-AND-ACTORS doc + a few common ones).
        inner.insert("ogit-erp/move", "odoo:transport");
        inner.insert("ogit-erp/sale.order", "odoo:sale.order");
        inner.insert("ogit-erp/account.move", "odoo:account.move");
        inner.insert("ogit-erp/stock.move", "odoo:stock.move");
        inner.insert("ogit-erp/res.partner", "odoo:res.partner");
        inner.insert("ogit-erp/res.company", "odoo:res.company");
        inner.insert("ogit-erp/product.template", "odoo:product.template");
        // Common action-predicate renames.
        inner.insert("ogit-erp::action::action_confirm", "odoo:action_confirm");
        inner.insert("ogit-erp::action::action_cancel", "odoo:action_cancel");
        inner.insert("ogit-erp::action::action_post", "odoo:action_post");
        // Decorator → role keyword renames (used by emitter when
        // producing action-vocabulary triples for Odoo source).
        inner.insert("ogar/decorator/api.depends", "odoo:@api.depends");
        inner.insert("ogar/decorator/api.constrains", "odoo:@api.constrains");
        inner.insert("ogar/decorator/api.onchange", "odoo:@api.onchange");
        inner.insert("ogar/decorator/api.model", "odoo:@api.model");
        inner.insert("ogar/decorator/api.model_create_multi", "odoo:@api.model_create_multi");
        Self { inner }
    }

    /// Register an additional (canonical → odoo) leaf. Producers call
    /// this when they encounter new class names not in the default set.
    pub fn register(&mut self, canonical: impl Into<String>, odoo: impl Into<String>) {
        self.inner.insert(canonical, odoo);
    }

    /// Number of registered leaves.
    #[must_use]
    pub fn leaf_count(&self) -> usize {
        self.inner.leaves.len()
    }
}

impl Default for OdooAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for OdooAdapter {
    fn map(&self, canonical: &str) -> Option<String> {
        self.inner.map(canonical)
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btreemap_adapter_basic_map() {
        let mut a = BTreeMapAdapter::new("test");
        a.insert("ogit-erp/foo", "target:foo");
        a.insert("ogit-erp/bar", "target:bar");
        assert_eq!(a.map("ogit-erp/foo"), Some("target:foo".into()));
        assert_eq!(a.map("ogit-erp/bar"), Some("target:bar".into()));
        assert_eq!(a.map("ogit-erp/missing"), None);
    }

    #[test]
    fn iter_prefix_returns_only_matching_paths() {
        let mut a = BTreeMapAdapter::new("test");
        a.insert("ogit-erp/sale.order", "odoo:sale.order");
        a.insert("ogit-erp/sale.order.line", "odoo:sale.order.line");
        a.insert("ogit-erp/account.move", "odoo:account.move");
        let under_sale: Vec<_> = a.iter_prefix("ogit-erp/sale").map(|(k, _)| k.as_str()).collect();
        assert_eq!(under_sale.len(), 2);
        assert!(under_sale.iter().all(|k| k.starts_with("ogit-erp/sale")));
    }

    #[test]
    fn odoo_adapter_default_leaves_resolve() {
        let o = OdooAdapter::new();
        assert!(o.leaf_count() >= 10);
        assert_eq!(o.map("ogit-erp/move").as_deref(), Some("odoo:transport"));
        assert_eq!(o.map("ogit-erp/sale.order").as_deref(), Some("odoo:sale.order"));
        assert_eq!(o.map("ogit-erp/missing"), None);
    }

    #[test]
    fn odoo_adapter_register_adds_leaf() {
        let mut o = OdooAdapter::new();
        let before = o.leaf_count();
        o.register("ogit-erp/custom.thing", "odoo:custom.thing");
        assert_eq!(o.leaf_count(), before + 1);
        assert_eq!(o.map("ogit-erp/custom.thing").as_deref(), Some("odoo:custom.thing"));
    }

    #[test]
    fn adapter_trait_has_name() {
        let o = OdooAdapter::new();
        assert_eq!(o.name(), "OdooAdapter");
    }

    #[test]
    fn action_predicate_renames_are_registered() {
        let o = OdooAdapter::new();
        assert_eq!(
            o.map("ogit-erp::action::action_confirm").as_deref(),
            Some("odoo:action_confirm"),
        );
    }
}
