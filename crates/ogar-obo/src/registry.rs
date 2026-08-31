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
        concept_id: 0x9101,
    },
    NsSpec {
        prefix: "HP",
        concept_id: 0x9202,
    },
    NsSpec {
        prefix: "UBERON",
        concept_id: 0x9303,
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
/// bake grounds on. Same `0x03` Ontology domain as [`OBO_CORE`], under the
/// same local authority its concept-id docs describe.
///
/// ## Why this starts at `0x0340` — and why it took three attempts
///
/// Minted at `0x0306..=0x030D`, which collided with
/// `ogar_ro::RELATION_BODY_CONCEPT_ID` (`0x0306`). Moved to
/// `0x0310..=0x0317`, which collided with **four** consumer allocations —
/// OPS `0x0311`, a product lane `0x0313`, the UMLS CUI convergence lane
/// `0x0315`, Orphanet `0x0317`.
///
/// The second attempt failed for a reason worth naming: the `0x03` domain's
/// occupancy was enumerated **in this repo only**. A private consumer holds an
/// odd-stride run — `0x0307, 09, 0B, 0D, 0F, 11, 13, 15, 17, 19, 1B, 1D` —
/// that no test here can see, and it grows into odd slots, so any nearby block
/// collides eventually rather than immediately. Picking a "clean" block by eye
/// is the failure mode; there is no vantage point in this crate from which the
/// block is verifiably clean.
///
/// `0x0340..=0x0347` sits clear of both the core band and that run's growth,
/// but **the spacing is a mitigation, not the guard.** The guard has to live
/// where both sides are visible — i.e. in the consumer. The band test below
/// only proves this table is self-consistent and clear of the core; it
/// cannot and does not prove the consumer is clear, and reading it as if it
/// did is what produced the second collision.
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
        concept_id: 0x0340,
    },
    NsSpec {
        prefix: "COB",
        concept_id: 0x0341,
    },
    NsSpec {
        prefix: "IAO",
        concept_id: 0x0342,
    },
    NsSpec {
        prefix: "OBI",
        concept_id: 0x0343,
    },
    NsSpec {
        prefix: "OBCS",
        concept_id: 0x0344,
    },
    NsSpec {
        prefix: "SEPIO",
        concept_id: 0x0345,
    },
    NsSpec {
        prefix: "ECO",
        concept_id: 0x0346,
    },
    NsSpec {
        prefix: "FBbi",
        concept_id: 0x0347,
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
        // `Namespace::ALL` — not a local literal. This test used to carry its
        // own copy of the five, which made three copies of one list (enum,
        // registry, test). Consuming the public const leaves two, and the
        // agreement between those two is exactly what this test asserts.
        let enum_order = Namespace::ALL;
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
                concept_id: 0x0340,
            },
            NsSpec {
                prefix: "MONDO",
                concept_id: 0x0301,
            },
        ]);
        let bfo = EXTENDED.parse("BFO:0000002").expect("BFO parses here");
        assert_eq!(bfo.ns, 0, "BFO is row 0 of this table");
        assert_eq!(bfo.num, 2);
        assert_eq!(EXTENDED.render_classid(0, 0x0000), Some(0x0340_0000));
        assert!(
            TermId::parse("BFO:0000002").is_none(),
            "the shipped core must NOT resolve BFO — otherwise this proves nothing"
        );

        // Ordinals are table-relative: MONDO is row 1 here, row 0 in the core.
        // A TermId is therefore only meaningful WITH its registry.
        assert_eq!(EXTENDED.parse("MONDO:0007739").map(|t| t.ns), Some(1));
        assert_eq!(TermId::parse("MONDO:0007739").map(|t| t.ns), Some(0));
    }

    /// The guard the 2026-08-08 audit had to be run by hand to find.
    ///
    /// `META_STUDY_SPINE` was minted at `0x0306..=0x030D`, and `0x0306` is
    /// `ogar_ro::RELATION_BODY_CONCEPT_ID` — live, and already consumed
    /// downstream. Two concepts at one address is not a near-miss: a baked BFO
    /// row and an `ogar-ro` relation-body row become the same classid.
    ///
    /// This crate cannot see `ogar-ro` (that crate deps *this* one, so the
    /// dependency only runs one way), so the band is asserted instead of the
    /// constant: everything below `SPINE_BAND_START` belongs to the core
    /// allocations, and the spine must stay above it. `ogar-ro` carries the
    /// mirror guard against its own real constant.
    ///
    /// **What would make it fail:** minting a spine row inside the reserved
    /// band, or giving two tables the same id.
    #[test]
    fn spine_does_not_collide_with_the_core_or_the_reserved_band() {
        /// First id of the spine block. Everything below belongs to the core
        /// band or to consumer allocations this crate cannot see.
        const SPINE_BAND_START: u16 = 0x0340;

        for s in META_STUDY_SPINE.specs() {
            assert!(
                s.concept_id >= SPINE_BAND_START,
                "{} at {:#06X} sits below the spine block — that range holds the \
                 core namespaces, ogar-ro (0x0306), and consumer allocations \
                 this crate cannot see",
                s.prefix,
                s.concept_id
            );
        }
        // Since the S3 writer flip the migrated core ids live in the 0x9X
        // domain tree, so "below the spine band" is no longer the core's
        // shape. The invariant that MATTERS is unchanged: no core id may sit
        // inside the spine block itself (0x0340..=0x0347 + headroom to the
        // end of the 0x03 page), and the un-migrated pair (PATO/RO) still
        // sits below it.
        for c in OBO_CORE.specs() {
            let in_legacy_page = c.concept_id >> 8 == 0x03;
            assert!(
                !in_legacy_page || c.concept_id < SPINE_BAND_START,
                "{} at {:#06X} collides with the reserved spine band",
                c.prefix,
                c.concept_id
            );
        }

        // No id twice, across BOTH tables — the general form of the bug.
        let mut ids: Vec<(u16, &str)> = OBO_CORE
            .specs()
            .iter()
            .chain(META_STUDY_SPINE.specs())
            .map(|s| (s.concept_id, s.prefix))
            .collect();
        ids.sort_unstable();
        for w in ids.windows(2) {
            assert_ne!(
                w[0].0, w[1].0,
                "{} and {} both claim {:#06X}",
                w[0].1, w[1].1, w[0].0
            );
        }

        // Every id stays inside a domain THIS table may address — the spine
        // and the un-migrated pair in the legacy 0x03 Ontology page, the
        // migrated three in the 0x90..0x9D domain-reference tree (S3 writer
        // flip). An id outside both would silently land in a foreign domain,
        // which is exactly what this guard exists to catch.
        for s in OBO_CORE.specs().iter().chain(META_STUDY_SPINE.specs()) {
            let hi = (s.concept_id >> 8) as u8;
            assert!(
                hi == 0x03 || (0x90..=0x9D).contains(&hi),
                "{} at {:#06X} sits in a domain this crate does not own",
                s.prefix,
                s.concept_id
            );
        }
    }

    #[test]
    fn unknown_ordinal_resolves_to_none_rather_than_panicking() {
        assert_eq!(OBO_CORE.spec(9), None);
        assert_eq!(OBO_CORE.concept_id(9), None);
        assert_eq!(OBO_CORE.render_classid(9, 0), None);
    }
}
