//! # `registry` — the namespace table as **data**, not as five `match` arms.
//!
//! The shipped bake reaches its namespaces through the [`crate::Namespace`] enum,
//! which spreads one namespace's facts across four `match` bodies and an
//! index array ([`crate::Namespace::prefix`], [`crate::Namespace::from_prefix`],
//! [`crate::Namespace::concept_id`], [`crate::Namespace::namespace`]). Adding a sixth
//! namespace means editing all five sites with hand-written literals — the
//! shape that makes a bake un-extendable without a code change per ontology.
//!
//! This module carries the same facts as **one table of rows**. A driver that
//! wants to bake a *different* set of namespaces passes a different
//! [`NsRegistry`]; it does not touch this crate's types.
//!
//! ## The ordinal is the sort key — table ORDER is load-bearing
//!
//! A [`TermId`]'s `ns` field is the row's **index in the registry**, and
//! [`crate::bake`] sorts ids by `(ns, num)`. So the table's order determines
//! the row order of the baked artifact, and therefore its digest. Reordering
//! [`OBO_CORE`] silently re-bakes every downstream slab.
//!
//! [`obo_core_matches_the_shipped_enum`](self) pins the mirror: every row's
//! index and concept id must equal the enum's discriminant and
//! [`crate::Namespace::concept_id`]. That test fails on any reorder or renumber —
//! it is the drift guard that makes a second copy of these facts safe to hold.
//!
//! ## What is NOT here
//!
//! [`crate::classify`] maps a `(subject_ns, object_ns)` pair to an RO
//! [`Predicate`](crate::Predicate). That is *semantics between two named
//! ontologies*, not a per-namespace attribute, so it stays on the enum. A
//! namespace outside the core classifies as
//! [`Predicate::Other`](crate::Predicate::Other) — correct for the upper-level
//! spine ontologies, whose edges are `is_a` skeletons with no cross-ontology
//! predicate to name.

use crate::TermId;

/// One namespace row — the config unit.
///
/// `prefix` is the OBO CURIE prefix as it appears left of the colon
/// (`"MONDO"`, `"HP"`, …). `concept_id` is the canonical hi-u16 that
/// [`crate::Namespace::render_classid`] shifts into the classid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NsSpec {
    /// The OBO CURIE prefix, matched case-sensitively (OBO prefixes are
    /// upper-case by convention and the shipped five are matched exactly).
    pub prefix: &'static str,
    /// The canonical hi-u16 concept id (`0xDDCC`, domain `DD` · slot `CC`).
    pub concept_id: u16,
}

/// An ordered namespace table. The row **index** is the [`TermId::ns`]
/// ordinal; see the module docs on why order is load-bearing.
#[derive(Debug, Clone, Copy)]
pub struct NsRegistry {
    specs: &'static [NsSpec],
}

impl NsRegistry {
    /// Build a registry over a static table. `specs.len()` must be `<= 256`
    /// because the ordinal is stored in [`TermId::ns`] (`u8`).
    #[must_use]
    pub const fn new(specs: &'static [NsSpec]) -> Self {
        assert!(specs.len() <= 256, "ordinal must fit TermId::ns (u8)");
        Self { specs }
    }

    /// Number of namespaces in the table.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.specs.len()
    }

    /// Whether the table is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    /// Every row, in ordinal order.
    #[must_use]
    pub const fn specs(&self) -> &'static [NsSpec] {
        self.specs
    }

    /// The ordinal of a CURIE prefix, or `None` if the prefix is not in this
    /// table. Linear scan — the tables are tens of rows, and this runs once
    /// per CURIE during a parse, never in a query path.
    #[must_use]
    pub fn ordinal_of(&self, prefix: &str) -> Option<u8> {
        self.specs
            .iter()
            .position(|s| s.prefix == prefix)
            .map(|i| i as u8)
    }

    /// The row at an ordinal.
    #[must_use]
    pub fn spec(&self, ordinal: u8) -> Option<&'static NsSpec> {
        self.specs.get(ordinal as usize)
    }

    /// The concept id at an ordinal.
    #[must_use]
    pub fn concept_id(&self, ordinal: u8) -> Option<u16> {
        self.spec(ordinal).map(|s| s.concept_id)
    }

    /// The full V3 render classid — canon-high `(concept as u32) << 16 |
    /// app_prefix`, identical to [`crate::Namespace::render_classid`].
    #[must_use]
    pub fn render_classid(&self, ordinal: u8, app_prefix: u16) -> Option<u32> {
        self.concept_id(ordinal)
            .map(|c| (u32::from(c) << 16) | u32::from(app_prefix))
    }

    /// Parse a CURIE against **this** table. Same contract as
    /// [`TermId::parse`] (which is this, against [`OBO_CORE`]): `None` if the
    /// prefix is absent from the table, the numeric is malformed, or the
    /// numeric exceeds 24 bits.
    #[must_use]
    pub fn parse(&self, curie: &str) -> Option<TermId> {
        let (p, n) = curie.split_once(':')?;
        let ns = self.ordinal_of(p)?;
        let num: u32 = n.trim().parse().ok()?;
        if num > 0x00FF_FFFF {
            return None;
        }
        Some(TermId { ns, num })
    }
}

/// The **shipped** five-namespace core, in the enum's discriminant order.
///
/// This table is a second copy of facts the [`crate::Namespace`] enum also holds.
/// It is safe to hold only because
/// [`obo_core_matches_the_shipped_enum`](self) fails the build's test suite
/// the moment the two disagree.
pub const OBO_CORE: NsRegistry = NsRegistry::new(&[
    NsSpec {
        prefix: "MONDO",
        concept_id: 0x0301,
    },
    NsSpec {
        prefix: "HP",
        concept_id: 0x0302,
    },
    NsSpec {
        prefix: "UBERON",
        concept_id: 0x0303,
    },
    NsSpec {
        prefix: "PATO",
        concept_id: 0x0304,
    },
    NsSpec {
        prefix: "RO",
        concept_id: 0x0305,
    },
]);

/// The upper-level **meta-study spine** — the `is_a` skeleton a study-level
/// bake grounds on. Continues the same `0x03` Ontology domain as [`OBO_CORE`],
/// under the same local authority its concept-id docs describe.
///
/// ## Order is a convention, not a topological sort
///
/// The rows run roughly upper→applied, by which direction the cross-namespace
/// edges dominate. That is **not** a dependency order in the strict sense: the
/// import graph measured by `examples/probe_spine.rs` is cyclic — e.g. `OBI`
/// and `IAO` reference each other (394 one way, 10 the other), as do `OBI` and
/// `COB` (236 / 3) and `OBI` and `OBCS` (58 / 70).
///
/// Nothing depends on the order being acyclic: [`crate::bake`] takes the
/// *union* of all namespaces and sorts rows by `(ns, num)`, so a cyclic import
/// graph bakes fine. What the order does fix is the **row order**, and through
/// it the artifact digest — so reordering this table re-bakes everything,
/// exactly as for [`OBO_CORE`].
///
/// # ⚠ Gate: this table is NOT bakeable until the numeric-`0` pad collision is resolved
///
/// [`crate::edges`] packs edge targets into value lanes and uses the numeric
/// `0` as the **pad** meaning *lane spent, advance*. Three of these namespaces
/// contain a real term whose CURIE numeric is `0` — in at least one case the
/// namespace's own root. An edge pointing at such a term is indistinguishable
/// from a pad, so a reader's walk stops early and **silently**.
///
/// The fix is local to [`crate::edges`] — bias the stored numeric by `+1`
/// (`store = num + 1`, `read = stored - 1`), which costs one value off the top
/// of the range (2²⁴−2 addressable, against a measured maximum well below it).
/// It is not applied here because it moves **every** shipped slab digest and
/// forces a re-bake of all five core tables; that is an operator call, not a
/// side effect of adding a table.
///
/// Until then: the rows below are config a driver may *read*, and
/// `examples/probe_spine.rs` measures which namespaces actually trip the
/// collision — but a bake over this table would be unsound for those.
pub const META_STUDY_SPINE: NsRegistry = NsRegistry::new(&[
    NsSpec {
        prefix: "BFO",
        concept_id: 0x0306,
    },
    NsSpec {
        prefix: "COB",
        concept_id: 0x0307,
    },
    NsSpec {
        prefix: "IAO",
        concept_id: 0x0308,
    },
    NsSpec {
        prefix: "OBI",
        concept_id: 0x0309,
    },
    NsSpec {
        prefix: "OBCS",
        concept_id: 0x030A,
    },
    NsSpec {
        prefix: "SEPIO",
        concept_id: 0x030B,
    },
    NsSpec {
        prefix: "ECO",
        concept_id: 0x030C,
    },
    NsSpec {
        prefix: "FBbi",
        concept_id: 0x030D,
    },
]);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Namespace;

    /// The drift guard that makes the duplicate table safe.
    ///
    /// **What would make it fail:** reordering [`OBO_CORE`], renumbering a
    /// concept id, or renaming a prefix. Each of those silently changes the
    /// baked row order or the classid of every row in a namespace — i.e.
    /// moves the shipped digests — and this test is what refuses it.
    #[test]
    fn obo_core_matches_the_shipped_enum() {
        let enum_order = [
            Namespace::Mondo,
            Namespace::Hpo,
            Namespace::Uberon,
            Namespace::Pato,
            Namespace::Ro,
        ];
        assert_eq!(
            OBO_CORE.len(),
            enum_order.len(),
            "registry and enum must carry the same namespace count"
        );
        for (i, ns) in enum_order.iter().enumerate() {
            let ord = i as u8;
            assert_eq!(
                OBO_CORE.ordinal_of(ns.prefix()),
                Some(ord),
                "{} must sit at the enum's discriminant — row order is the sort key",
                ns.prefix()
            );
            assert_eq!(
                OBO_CORE.concept_id(ord),
                Some(ns.concept_id()),
                "{} concept id must mirror the enum",
                ns.prefix()
            );
            assert_eq!(
                OBO_CORE.render_classid(ord, 0x00AB),
                Some(ns.render_classid(0x00AB)),
                "{} classid must mirror the enum under a non-zero app prefix",
                ns.prefix()
            );
        }
    }

    /// The registry parse must agree with the shipped [`TermId::parse`] on
    /// every accept AND every reject — otherwise routing a bake through the
    /// table would change which terms are tiled.
    #[test]
    fn registry_parse_agrees_with_shipped_parse() {
        let accept = [
            "MONDO:0007739",
            "HP:0000118",
            "UBERON:0000955",
            "PATO:0000001",
            "RO:0002200",
            "MONDO:0700092", // above u16 — the rail case
        ];
        for c in accept {
            assert_eq!(
                OBO_CORE.parse(c),
                TermId::parse(c),
                "{c} must parse identically through the table"
            );
            assert!(OBO_CORE.parse(c).is_some(), "{c} must actually parse");
        }
        let reject = [
            "CHEBI:12345",     // prefix outside the table
            "MONDO:99999999",  // > 24 bit
            "MONDO:not-a-num", // malformed numeric
            "nocolon",         // no separator
            "mondo:0007739",   // case differs — OBO prefixes are upper-case
        ];
        for c in reject {
            assert_eq!(
                OBO_CORE.parse(c),
                TermId::parse(c),
                "{c} must reject identically through the table"
            );
            assert!(OBO_CORE.parse(c).is_none(), "{c} must actually reject");
        }
    }

    /// A registry over a *different* table resolves namespaces the core does
    /// not — the whole point. This is the can-it-fire half: if the table were
    /// ignored and the core consulted instead, `BFO:0000002` would not parse
    /// and the classid would not be the extended one.
    #[test]
    fn a_different_table_resolves_namespaces_the_core_cannot() {
        const EXTENDED: NsRegistry = NsRegistry::new(&[
            NsSpec {
                prefix: "BFO",
                concept_id: 0x0310,
            },
            NsSpec {
                prefix: "MONDO",
                concept_id: 0x0301,
            },
        ]);
        let bfo = EXTENDED.parse("BFO:0000002").expect("BFO parses here");
        assert_eq!(bfo.ns, 0, "BFO is row 0 of this table");
        assert_eq!(bfo.num, 2);
        assert_eq!(EXTENDED.render_classid(0, 0x0000), Some(0x0310_0000));
        assert!(
            TermId::parse("BFO:0000002").is_none(),
            "the shipped core must NOT resolve BFO — otherwise this proves nothing"
        );

        // Ordinals are table-relative: MONDO is row 1 here, row 0 in the core.
        // A TermId is therefore only meaningful WITH its registry.
        assert_eq!(EXTENDED.parse("MONDO:0007739").map(|t| t.ns), Some(1));
        assert_eq!(TermId::parse("MONDO:0007739").map(|t| t.ns), Some(0));
    }

    #[test]
    fn unknown_ordinal_resolves_to_none_rather_than_panicking() {
        assert_eq!(OBO_CORE.spec(9), None);
        assert_eq!(OBO_CORE.concept_id(9), None);
        assert_eq!(OBO_CORE.render_classid(9, 0), None);
    }
}
