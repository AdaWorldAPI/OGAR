//! `ogar-osm` — the **OpenStreetMap geodata reference surface**: the render
//! half of the Geo domain (`0x0FXX`).
//!
//! # What this is NOT
//!
//! It is **not a mint**. The ten Geo concepts already exist in
//! [`ogar_vocab::class_ids`] — `osm_node 0x0F01` … `osm_user 0x0F0A`,
//! harvested mechanically from the `openstreetmap-website` Rails `app/models`
//! tree through ruff → OGAR (`.claude/harvest/osm-website-rs`, 50 classes).
//! This crate consumes those constants and adds nothing to the codebook.
//!
//! # What is actually missing, and what this crate supplies
//!
//! A consumer that bakes OSM into V3 SoA rows can already key them: the
//! concept ids are minted and the classid spelling is fixed. What it cannot do
//! is **read a row back as fields**. `ogar-render-askama`'s `FieldView` carries
//! a `position: u8` — the field's mask address — and something has to say what
//! position 3 of a geo row *means*. Nothing did. That table is this crate.
//!
//! Concretely, three things:
//!
//! 1. [`classid`] — the `concept << 16 | classview` spelling, so a consumer
//!    stops hand-assembling `0x0F01_1000` from two halves it re-derives.
//! 2. [`GEO_V3_FACET`] — the 12-byte V3 facet field table: one [`OsmField`]
//!    per mask position, with the label a `FieldView` renders and the
//!    `(rail, axis)` reading the tier carving gives it.
//! 3. [`osm_capabilities`] — the authoritative capability table a geo
//!    executor registers against ([`verify_osm_registration`]), the
//!    plug-and-play roundtrip [`ogar_vocab::capability_registry`] defines.
//!
//! # The facet reading is canon, not a local convention
//!
//! The 12-byte payload is the content-blind register carved `6×(u8:u8)`
//! (`lance-graph` `.claude/v3/soa_layout/le-contract.md` §3). Each `(u8:u8)`
//! rail is one HHTL tier read as a **256×256 centroid tile** — two axes, each
//! a byte index — and the workspace canon binds those axes per domain with
//! OSM named explicitly: *"domains bind the axes (OSM: literal x/y; semantic:
//! PQ subspace pairs)"* (`OGAR/CLAUDE.md`, § Tier interpretation). So for the
//! Geo domain a rail's two bytes are the tile's **x** and **y** — this table
//! reads the canon, it does not invent a layout.
//!
//! `u8:u8` stays two separate bytes and is never widened to `u16`: a mask
//! addresses a byte position, so a rail is two addresses, not one.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use ogar_vocab::app::render_classid;
use ogar_vocab::capability_registry::{
    CapabilityRegistration, RegistrationDrift, verify_registration,
};
use ogar_vocab::class_ids;
use ogar_vocab::ports::{OsmPort, PortSpec};

/// The Geo domain's high byte — `ogar_vocab::ConceptDomain::Geo`.
///
/// Every concept in [`OSM_CONCEPTS`] satisfies `id >> 8 == GEO_DOMAIN`; pinned
/// by [`tests::every_concept_sits_in_the_geo_domain`].
pub const GEO_DOMAIN: u8 = 0x0F;

/// ClassView (low u16) for the **baked V3 substrate** reading — a geo row in a
/// 512-byte `NodeRow` slab, read through the `6×(u8:u8)` facet carving.
///
/// `0x1000` is the V3 marker form the codebook already uses for the parallel
/// Genetics reservation (`0x0E01_1000`) and the FMA anatomy classid
/// (`0x0A01_1000`). Composed with `osm_node` this is `0x0F01_1000`.
pub const CLASSVIEW_V3_SUBSTRATE: u16 = 0x1000;

/// ClassView (low u16) for the **OSM web render** skin — the same concepts,
/// an HTML field view instead of a slab reading.
///
/// **Pulled from [`OsmPort::APP_PREFIX`]**, never restated. An earlier version
/// of this crate declared the literal `0x0008` here while the port declared it
/// too — the same prefix twice, in one crate, added in one PR. That is the
/// second-source-of-truth defect the port exists to prevent.
pub const CLASSVIEW_OSM_WEB: u16 = OsmPort::APP_PREFIX;

/// Compose a full 32-bit classid from a canonical concept and a ClassView.
///
/// The spelling is `concept(hi u16) : classview(lo u16)` — canon HIGH since
/// the 2026-07-02 flip. The hi half names the **shared concept** (RBAC +
/// ontology); the lo half picks the **render skin**. Neither half carries
/// behaviour — that is a property of the node the address resolves to.
///
/// **Delegates to [`render_classid`]** rather than re-deriving the shift. That
/// module's doc states its purpose exactly: consumers "re-export ONE source of
/// the bit math instead of each re-implementing
/// `((concept as u32) << 16) | prefix`" — which is what this function used to
/// contain, verbatim.
#[must_use]
pub const fn classid(concept: u16, classview: u16) -> u32 {
    render_classid(classview, concept)
}

/// Every Geo concept, in codebook order, with its Rails source class(es).
///
/// The `Old*` versioned Rails classes ground to their base concept — the
/// temporal axis is a Lance version, not a new id — which is why `Node` and
/// `OldNode` share `0x0F01`.
pub const OSM_CONCEPTS: &[(&str, u16, &[&str])] = &[
    ("osm_node", class_ids::OSM_NODE, &["Node", "OldNode"]),
    ("osm_way", class_ids::OSM_WAY, &["Way", "OldWay"]),
    (
        "osm_relation",
        class_ids::OSM_RELATION,
        &["Relation", "OldRelation"],
    ),
    ("osm_changeset", class_ids::OSM_CHANGESET, &["Changeset"]),
    (
        "osm_element_tag",
        class_ids::OSM_ELEMENT_TAG,
        &[
            "NodeTag",
            "WayTag",
            "RelationTag",
            "OldNodeTag",
            "OldWayTag",
            "OldRelationTag",
        ],
    ),
    (
        "osm_relation_member",
        class_ids::OSM_RELATION_MEMBER,
        &["RelationMember", "OldRelationMember"],
    ),
    (
        "osm_way_node",
        class_ids::OSM_WAY_NODE,
        &["WayNode", "OldWayNode"],
    ),
    ("osm_note", class_ids::OSM_NOTE, &["Note"]),
    ("osm_gpx_trace", class_ids::OSM_GPX_TRACE, &["Trace"]),
    ("osm_user", class_ids::OSM_USER, &["User"]),
];

// ── The V3 facet field table ────────────────────────────────────────

/// Which axis of a `256×256` centroid tile a facet byte indexes.
///
/// For the Geo domain the binding is **literal x/y** (`OGAR/CLAUDE.md`
/// § Tier interpretation); a semantic domain binds the same rail to a PQ
/// subspace pair instead. The algebra is identical; only the binding differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Axis {
    /// The tile's x index (longitude direction, TMS).
    X,
    /// The tile's y index (latitude direction, TMS — y increases NORTH).
    Y,
    /// A byte that is not a tile axis (the identity tail).
    None,
}

/// One field of a class's ordered field set — a **mask address** plus what a
/// renderer needs to display it.
///
/// `position` is the same `u8` `ogar-render-askama`'s `FieldView` carries, and
/// the same bit index a `WideFieldMask` sets. A projection is a filter over
/// these addresses; it never gathers bytes into a new container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OsmField {
    /// The field's mask position — its layout address in the 12-byte facet.
    pub position: u8,
    /// Display label, for the `FieldView` a template iterates.
    pub label: &'static str,
    /// Stable key behind the label (the `FieldView::predicate` slot).
    pub predicate: &'static str,
    /// Which `(u8:u8)` rail this byte belongs to — `position / 2`.
    pub rail: u8,
    /// Which tile axis the byte indexes.
    pub axis: Axis,
}

/// Bytes in the V3 content-blind facet register (`classid(4) + payload(12)`).
pub const FACET_PAYLOAD_BYTES: u8 = 12;

/// Rails in the `6×(u8:u8)` carving — `FACET_PAYLOAD_BYTES / 2`.
pub const FACET_RAILS: u8 = FACET_PAYLOAD_BYTES / 2;

/// The Geo domain's V3 facet field table — one entry per mask position.
///
/// Rails 0–3 are the four HHTL cascade tiers (heel/hip/twig/leaf), each a
/// `256×256` tile with x and y bound literally. Rails 4–5 are the identity
/// tail: the family basin (zero while the zero-fallback ladder is dormant)
/// and the per-tile collision counter that only breaks a same-key tie.
///
/// Ordered by `position`, and every position `0..FACET_PAYLOAD_BYTES` present
/// exactly once — pinned by [`tests::the_field_table_covers_every_position_once`].
pub const GEO_V3_FACET: &[OsmField] = &[
    OsmField {
        position: 0,
        label: "Heel x",
        predicate: "geo:heel.x",
        rail: 0,
        axis: Axis::X,
    },
    OsmField {
        position: 1,
        label: "Heel y",
        predicate: "geo:heel.y",
        rail: 0,
        axis: Axis::Y,
    },
    OsmField {
        position: 2,
        label: "Hip x",
        predicate: "geo:hip.x",
        rail: 1,
        axis: Axis::X,
    },
    OsmField {
        position: 3,
        label: "Hip y",
        predicate: "geo:hip.y",
        rail: 1,
        axis: Axis::Y,
    },
    OsmField {
        position: 4,
        label: "Twig x",
        predicate: "geo:twig.x",
        rail: 2,
        axis: Axis::X,
    },
    OsmField {
        position: 5,
        label: "Twig y",
        predicate: "geo:twig.y",
        rail: 2,
        axis: Axis::Y,
    },
    OsmField {
        position: 6,
        label: "Leaf x",
        predicate: "geo:leaf.x",
        rail: 3,
        axis: Axis::X,
    },
    OsmField {
        position: 7,
        label: "Leaf y",
        predicate: "geo:leaf.y",
        rail: 3,
        axis: Axis::Y,
    },
    OsmField {
        position: 8,
        label: "Family (low)",
        predicate: "geo:family.lo",
        rail: 4,
        axis: Axis::None,
    },
    OsmField {
        position: 9,
        label: "Family (mid)",
        predicate: "geo:family.mid",
        rail: 4,
        axis: Axis::None,
    },
    OsmField {
        position: 10,
        label: "Family (high)",
        predicate: "geo:family.hi",
        rail: 5,
        axis: Axis::None,
    },
    OsmField {
        position: 11,
        label: "Identity",
        predicate: "geo:identity",
        rail: 5,
        axis: Axis::None,
    },
];

/// Resolve a Geo concept to its V3 facet field surface — **total** over the
/// domain, the way `medcare-cohorts`' `ClassView::for_disease` is total over
/// `Disease`: dispatch is *data*, never a branch buried in a handler.
///
/// # Why all ten concepts resolve to the same table, and why that is a fact
///
/// This is not a stub standing in for per-concept variation that was skipped.
/// The V3 key is **spatial**: the four cascade tiers are the feature's HHTL
/// position, and every OSM element — node, way centroid, relation anchor — is
/// keyed by the identical `mint_for(tiers…)` call. The element's *kind* lives
/// in the `EntityType` value lane, addressed through `ValueTenant`, **not in a
/// key tier** (le-contract §2: the feature's kind belongs in EntityType + the
/// ClassView, never in a key tier). So the facet carving genuinely does not
/// vary by concept, and a table that pretended otherwise would be inventing a
/// distinction the substrate does not make.
///
/// What *does* vary per concept is the **value-slab** surface (positions past
/// [`FACET_PAYLOAD_BYTES`]) — a way carries geometry a node does not. That
/// surface is read through a different projection and is not this table.
///
/// Returns `None` for a concept outside the Geo domain, so a caller cannot
/// silently read a health or project row through a geo field table.
#[must_use]
pub fn facet_for(concept: u16) -> Option<&'static [OsmField]> {
    OSM_CONCEPTS
        .iter()
        .any(|(_, id, _)| *id == concept)
        .then_some(GEO_V3_FACET)
}

/// The field at a mask position, or `None` past the facet register.
///
/// A position `>= FACET_PAYLOAD_BYTES` is a value-slab field read through a
/// different surface — it is **skipped, not folded** onto a facet byte.
#[must_use]
pub fn field_at(position: u8) -> Option<&'static OsmField> {
    GEO_V3_FACET.iter().find(|f| f.position == position)
}

/// The two positions of one `(u8:u8)` rail, coarse-to-fine.
///
/// Returns `None` for `rail >= FACET_RAILS`.
#[must_use]
pub fn rail_positions(rail: u8) -> Option<[u8; 2]> {
    (rail < FACET_RAILS).then(|| [rail * 2, rail * 2 + 1])
}

// ── The value-lane reading ──────────────────────────────────────────

/// Sixteen-byte facet slots in one 512-byte row (`GUIDS_PER_NODE`).
///
/// The row is **32 lanes of `classid(4) + 12`** — uniformly, key included. Slot
/// 0 is the key facet, slot 1 the edge block, and slots 2..32 are the value
/// slab read the same way. That uniformity is what makes the ABI transparent:
/// every slot is self-describing, so a reader resolves any lane from its own
/// classid without decoding the row.
pub const ROW_SLOTS: usize = 32;

/// Slots consumed by the key and the edge block, before the value slab.
pub const RESERVED_SLOTS: usize = 2;

/// The value slot carrying the geo **identity facet**.
///
/// **Proposed, pending a ClassView** — the same honesty `osm-soa-bake`'s
/// `NAME_SLOT` carries. It is addressed as a slot INDEX; a consumer multiplies
/// by 16 rather than hardcoding a byte offset, which is the drift `tenants.md`
/// records three stale-offset incidents for.
pub const GEO_IDENTITY_SLOT: usize = RESERVED_SLOTS;

/// The carving of the identity facet's 12-byte payload: **`LegacyOutlier::WideQuad`
/// (G3, `3 × u32` contiguous)** — sanctioned, and deliberately marked as debt.
///
/// **This is NOT [`CascadeShape::G3D4`].** G3D4 is the *axis-grouped*
/// `3×(u8:u8:u8:u8)` quad with a per-byte rail; G3 is three contiguous `u32`
/// integers with **no rail at all**. Conflating them is exactly the mistake
/// the grace-period amendment exists to prevent, and this doc named the wrong
/// one before the canon was re-read.
///
/// # Why the discouraged carving is nonetheless right here
///
/// The amendment calls a wide carving *"strongly discouraged if
/// god-object-related or lacking proper bucket rollover"*, and an OSM id is
/// squarely the second case: a **monotonic global counter**, opaque, with no
/// axis and no bucketing. It cannot be decomposed onto the byte axis because
/// there is nothing to decompose — successive ids share no meaningful prefix.
///
/// The canon's usual remedy — *"migrate to cosine-replacement palette256
/// (L4)"* — does **not** apply either: palette quantisation is lossy, and the
/// entire reason to carry this field is exact round-trip to upstream OSM. An
/// approximate `osm_id` is worse than none.
///
/// So this is real debt with no current migration, recorded rather than
/// disguised. A consumer writes it through
/// `lance_graph_contract::legacy_outliers::LegacyOutlier::write_wide_quad`
/// and reads it back with `read_wide_quad` — **never hand-rolled byte
/// arithmetic**, which is the whole point of that module existing.
///
/// [`CascadeShape::G3D4`]: https://docs.rs/lance-graph-contract
pub const GEO_IDENTITY_SHAPE: &str = "LegacyOutlier::WideQuad";

/// One `u32` group of the identity facet's G3 (`3 × u32`) payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GeoIdentityGroup {
    /// Group index within the G3 (`3 × u32`) carving (`0..3`).
    pub group: u8,
    /// Display label.
    pub label: &'static str,
    /// Stable key behind the label.
    pub predicate: &'static str,
}

/// The geo ClassView's reading of the identity facet's three G3 `u32` groups.
///
/// `osm_id` occupies groups 0 and 1 (little-endian low, high) and is read as
/// **`i64`, signed** — negative ids are the convention for elements not yet
/// uploaded (JOSM/iD drafts), so reading them as `u64` would silently mangle
/// exactly the rows an editor round-trips.
///
/// Group 2 is **RESERVE-DON'T-RECLAIM**, not padding. OSM elements also carry
/// `version` and `changeset`, but those live in the PBF's optional `Info`
/// block which extracts routinely omit — declaring fields for data nobody
/// writes would describe fiction. A zero group reads as *not asserted*, and a
/// later `version` lands in group 2 with no migration.
pub const GEO_IDENTITY: &[GeoIdentityGroup] = &[
    GeoIdentityGroup {
        group: 0,
        label: "OSM id (low)",
        predicate: "osm:id.lo",
    },
    GeoIdentityGroup {
        group: 1,
        label: "OSM id (high)",
        predicate: "osm:id.hi",
    },
    GeoIdentityGroup {
        group: 2,
        label: "Reserved",
        predicate: "osm:identity.reserved",
    },
];

/// The identity-facet group with this predicate, if the reading declares one.
#[must_use]
pub fn identity_group(predicate: &str) -> Option<&'static GeoIdentityGroup> {
    GEO_IDENTITY.iter().find(|g| g.predicate == predicate)
}

/// The classid stamped on a row's identity facet — **dynamic, per row**.
///
/// A slot's classid is data, not a constant: it is written per row and read
/// back to decide the reading. Here that is load-bearing rather than
/// decorative — the element's KIND is exactly what the identity facet's
/// classid should say, so `osm_node` / `osm_way` / `osm_relation` stamp
/// different classids into the same slot.
///
/// The consequence is a lane removed, not added. A geo row previously needed
/// the `EntityType` tenant to say what it was; with the kind carried by the
/// identity slot's own classid the slot is **readable standalone** — resolve
/// the classid, get the ClassView, read the 12 bytes — and `EntityType`
/// becomes a duplicate of information the slot already carries.
///
/// Returns `None` for a concept outside the Geo domain, so a non-geo classid
/// cannot be stamped into a geo identity slot.
#[must_use]
pub fn geo_identity_classid(concept: u16) -> Option<u32> {
    OSM_CONCEPTS
        .iter()
        .any(|(_, id, _)| *id == concept)
        .then(|| classid(concept, CLASSVIEW_V3_SUBSTRATE))
}

// ── The capability table ────────────────────────────────────────────

/// One geo capability declaration — name, the concept it acts on, and its
/// name-level effect facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OsmCapability {
    /// Capability name — the consumer's dispatch arm.
    pub name: &'static str,
    /// The canonical concept this capability acts on (its `object_class`).
    pub subject: u16,
    /// Input names (effect annotation, mandatory + optional collapsed).
    pub reads: &'static [&'static str],
    /// Output names this capability produces.
    pub writes: &'static [&'static str],
}

/// The authoritative geo capability table.
///
/// Every entry corresponds to a primitive that **exists in the consumer
/// today** — the roundtrip checks coverage in both directions, so declaring an
/// aspirational capability here would fail the consumer's own registration
/// test rather than quietly describing work that was never done.
pub const OSM_CAPABILITIES: &[OsmCapability] = &[
    OsmCapability {
        name: "locate_point",
        subject: class_ids::OSM_NODE,
        reads: &["lon", "lat"],
        writes: &["morton", "heel", "hip", "twig", "leaf"],
    },
    OsmCapability {
        name: "locate_tile",
        subject: class_ids::OSM_NODE,
        reads: &["z", "x", "y"],
        writes: &["row_range"],
    },
    OsmCapability {
        name: "locate_fragment",
        subject: class_ids::OSM_NODE,
        reads: &["morton_lo", "morton_hi"],
        writes: &["fragment_ids"],
    },
    OsmCapability {
        name: "project_fields",
        subject: class_ids::OSM_NODE,
        reads: &["row", "surface_mask", "role_mask"],
        writes: &["positions", "bytes"],
    },
    OsmCapability {
        name: "street_edges",
        subject: class_ids::OSM_WAY,
        reads: &["junction_row", "name"],
        writes: &["edge_mask"],
    },
    OsmCapability {
        name: "polyline_length",
        subject: class_ids::OSM_WAY,
        reads: &["points"],
        writes: &["metres"],
    },
];

/// Every capability name, in table order — the `const`-evaluable fingerprint
/// for a cheap exhaustiveness fuse.
pub const OSM_CAPABILITY_NAMES: &[&str] = &[
    "locate_point",
    "locate_tile",
    "locate_fragment",
    "project_fields",
    "street_edges",
    "polyline_length",
];

/// The subject concepts [`OSM_CAPABILITIES`] acts on, deduplicated.
///
/// A registering consumer must activate exactly this set — no more, no fewer.
pub const OSM_SUBJECT_CLASSIDS: &[u16] = &[class_ids::OSM_NODE, class_ids::OSM_WAY];

/// Consumers expected to execute this table.
pub const OSM_EXPECTED_EXECUTORS: &[&str] = &["osm-soa-bake"];

/// Verify a consumer's self-registration against the geo capability table.
///
/// The plug-and-play roundtrip: the table declares WHAT exists and WHO is
/// expected to execute it; the consumer exports a [`CapabilityRegistration`]
/// and asserts this in its own tests. Drift bangs once — a capability added
/// here without a consumer arm, a consumer arm for a capability this table
/// dropped, or a wrong activated-concept set.
///
/// # Errors
///
/// Returns the [`RegistrationDrift`] the registry detected.
pub fn verify_osm_registration(reg: &CapabilityRegistration) -> Result<(), RegistrationDrift> {
    verify_registration(
        reg,
        OSM_CAPABILITY_NAMES,
        OSM_SUBJECT_CLASSIDS,
        OSM_EXPECTED_EXECUTORS,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_concept_sits_in_the_geo_domain() {
        for (name, id, rails) in OSM_CONCEPTS {
            assert_eq!(
                (id >> 8) as u8,
                GEO_DOMAIN,
                "{name} ({id:#06x}) is outside the Geo domain"
            );
            assert!(!rails.is_empty(), "{name} has no Rails source class");
        }
        // Ten concepts, and the ids are distinct — a copy-paste that reused
        // one constant twice would pass a domain check but fail here.
        assert_eq!(OSM_CONCEPTS.len(), 10);
        let mut ids: Vec<u16> = OSM_CONCEPTS.iter().map(|(_, id, _)| *id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 10, "two concepts share an id");
    }

    #[test]
    fn every_concept_is_minted_upstream_this_crate_mints_nothing() {
        // The point of the crate: it CONSUMES the codebook. If a concept here
        // were invented locally it would be absent from `class_ids::ALL`.
        for (name, id, _) in OSM_CONCEPTS {
            assert!(
                class_ids::ALL.iter().any(|(_, minted)| minted == id),
                "{name} ({id:#06x}) is not minted in ogar-vocab — this crate \
                 must not introduce concepts"
            );
        }
    }

    #[test]
    fn composition_goes_through_ogar_vocab_not_local_bit_math() {
        // The plug-and-play pin: naming the PORT must give the same id as
        // naming its prefix, because both compose through `render_classid`.
        // If this crate ever re-derives the shift, the two drift apart.
        use ogar_vocab::app::{app_of, concept_of, render_classid_for};
        for concept in [class_ids::OSM_NODE, class_ids::OSM_WAY, class_ids::OSM_USER] {
            let via_port = render_classid_for::<OsmPort>(concept);
            assert_eq!(classid(concept, CLASSVIEW_OSM_WEB), via_port);
            // And ogar-vocab's own decomposition recovers both halves, so the
            // routing key (`classid as u16`) is the port's prefix.
            assert_eq!(app_of(via_port), OsmPort::APP_PREFIX);
            assert_eq!(concept_of(via_port), concept);
        }
        // Anti-vacuity: a DIFFERENT ClassView really does produce a different
        // id, so the agreement above is about the prefix and not about
        // `classid` ignoring its second argument.
        assert_ne!(
            classid(class_ids::OSM_NODE, CLASSVIEW_V3_SUBSTRATE),
            render_classid_for::<OsmPort>(class_ids::OSM_NODE)
        );
    }

    #[test]
    fn the_classid_spelling_puts_the_concept_high_and_the_skin_low() {
        let baked = classid(class_ids::OSM_NODE, CLASSVIEW_V3_SUBSTRATE);
        assert_eq!(baked, 0x0F01_1000);
        assert_eq!((baked >> 16) as u16, class_ids::OSM_NODE);
        assert_eq!(baked as u16, CLASSVIEW_V3_SUBSTRATE);

        // Two ClassViews over the SAME concept: the hi half is shared (so RBAC
        // and ontology see one thing), the lo half differs (two renders).
        let web = classid(class_ids::OSM_NODE, CLASSVIEW_OSM_WEB);
        assert_ne!(baked, web);
        assert_eq!(baked >> 16, web >> 16);
    }

    #[test]
    fn the_field_table_covers_every_position_once() {
        assert_eq!(GEO_V3_FACET.len(), FACET_PAYLOAD_BYTES as usize);
        for p in 0..FACET_PAYLOAD_BYTES {
            let hits = GEO_V3_FACET.iter().filter(|f| f.position == p).count();
            assert_eq!(hits, 1, "position {p} appears {hits} times, want exactly 1");
        }
        // Ordered, so a template can iterate the slice directly.
        for (i, f) in GEO_V3_FACET.iter().enumerate() {
            assert_eq!(f.position as usize, i);
        }
    }

    #[test]
    fn a_rail_is_two_byte_addresses_never_one_widened_field() {
        // `u8:u8` stays two separate bytes (le-contract §3). If a rail were
        // widened to a u16 there would be 6 fields, not 12, and a mask could
        // not address one axis without the other.
        assert_eq!(FACET_RAILS, 6);
        for rail in 0..FACET_RAILS {
            let members: Vec<u8> = GEO_V3_FACET
                .iter()
                .filter(|f| f.rail == rail)
                .map(|f| f.position)
                .collect();
            assert_eq!(members.len(), 2, "rail {rail} must carry two positions");
            assert_eq!(rail_positions(rail), Some([members[0], members[1]]));
        }
        assert_eq!(rail_positions(FACET_RAILS), None);
    }

    #[test]
    fn the_four_cascade_rails_bind_x_and_y_and_the_tail_binds_neither() {
        // The Geo binding is literal x/y for the tiers; the identity tail is
        // not a tile axis. Getting this wrong would render "Heel y" for a
        // collision counter.
        for rail in 0..4 {
            let [px, py] = rail_positions(rail).unwrap();
            assert_eq!(field_at(px).unwrap().axis, Axis::X);
            assert_eq!(field_at(py).unwrap().axis, Axis::Y);
        }
        for rail in 4..FACET_RAILS {
            for p in rail_positions(rail).unwrap() {
                assert_eq!(field_at(p).unwrap().axis, Axis::None);
            }
        }
    }

    #[test]
    fn a_position_past_the_facet_register_is_absent_not_folded() {
        assert!(field_at(FACET_PAYLOAD_BYTES).is_none());
        assert!(field_at(63).is_none());
        // Paired can-fire half: the last in-range position DOES resolve, so
        // this is a boundary test and not a function that returns None always.
        assert!(field_at(FACET_PAYLOAD_BYTES - 1).is_some());
    }

    #[test]
    fn facet_resolution_is_total_over_the_geo_domain_and_closed_outside_it() {
        // Total: every minted Geo concept resolves.
        for (name, id, _) in OSM_CONCEPTS {
            assert!(
                facet_for(*id).is_some(),
                "{name} ({id:#06x}) must resolve to a facet surface"
            );
        }
        // Closed: a concept from another domain must NOT read through the geo
        // field table. Without this a health row would render "Heel x".
        assert!(facet_for(class_ids::PROJECT_WORK_ITEM).is_none());
        assert!(facet_for(0x0000).is_none());
        // A Geo-domain id that is not minted is also refused — the guard is
        // membership in the codebook, not merely the high byte.
        assert_eq!(0x0FFF >> 8, u16::from(GEO_DOMAIN));
        assert!(facet_for(0x0FFF).is_none());
    }

    #[test]
    fn all_ten_concepts_share_the_spatial_carving() {
        // Pins the documented FACT (the key is spatial; the kind lives in the
        // EntityType lane) rather than leaving it as prose. If someone gives
        // one concept a different facet table, this fails and forces the doc
        // comment on `facet_for` to be corrected with it.
        let node = facet_for(class_ids::OSM_NODE).unwrap();
        for (name, id, _) in OSM_CONCEPTS {
            assert_eq!(
                facet_for(*id).unwrap(),
                node,
                "{name} diverged from the shared spatial carving"
            );
        }
    }

    #[test]
    fn the_row_is_thirty_two_uniform_facet_slots() {
        // 512-byte row / 16-byte slot. The key and the edge block are slots
        // like any other — that uniformity is what makes the ABI transparent.
        assert_eq!(ROW_SLOTS, 32);
        assert_eq!(ROW_SLOTS * 16, 512);
        assert_eq!(RESERVED_SLOTS, 2);
    }

    #[test]
    fn the_identity_facet_uses_the_g3_wide_quad_waiting_room() {
        assert_eq!(GEO_IDENTITY.len(), 3);
        assert_eq!(GEO_IDENTITY.len() * 4, FACET_PAYLOAD_BYTES as usize);
        for (i, g) in GEO_IDENTITY.iter().enumerate() {
            assert_eq!(g.group as usize, i, "groups must be ordered and complete");
        }
        // An i64 id spans exactly two groups; the third is reserved, not padding.
        assert_eq!(identity_group("osm:id.lo").map(|g| g.group), Some(0));
        assert_eq!(identity_group("osm:id.hi").map(|g| g.group), Some(1));
        assert_eq!(
            identity_group("osm:identity.reserved").map(|g| g.group),
            Some(2)
        );
        assert!(identity_group("osm:version").is_none());
    }

    #[test]
    fn the_identity_slot_types_itself_per_element_kind() {
        // The dynamic-classid case: three kinds stamp three DIFFERENT classids
        // into the same slot, so the slot is readable standalone.
        let node = geo_identity_classid(class_ids::OSM_NODE).unwrap();
        let way = geo_identity_classid(class_ids::OSM_WAY).unwrap();
        let rel = geo_identity_classid(class_ids::OSM_RELATION).unwrap();
        assert_ne!(node, way);
        assert_ne!(way, rel);
        assert_ne!(node, rel);
        // Each still resolves back to its concept, so the kind is recoverable
        // from the slot alone — which is what lets EntityType go.
        assert_eq!((node >> 16) as u16, class_ids::OSM_NODE);
        assert_eq!((way >> 16) as u16, class_ids::OSM_WAY);
        // And they share the ClassView half: one reading, many kinds.
        assert_eq!(node as u16, CLASSVIEW_V3_SUBSTRATE);
        assert_eq!(way as u16, CLASSVIEW_V3_SUBSTRATE);
    }

    #[test]
    fn a_non_geo_concept_cannot_be_stamped_into_a_geo_identity_slot() {
        assert!(geo_identity_classid(class_ids::PROJECT_WORK_ITEM).is_none());
        assert!(geo_identity_classid(0x0FFF).is_none());
        // Can-fire half, so the guard is not "always None".
        assert!(geo_identity_classid(class_ids::OSM_NOTE).is_some());
    }

    #[test]
    fn the_capability_names_fingerprint_matches_the_table() {
        assert_eq!(OSM_CAPABILITIES.len(), OSM_CAPABILITY_NAMES.len());
        for (cap, name) in OSM_CAPABILITIES.iter().zip(OSM_CAPABILITY_NAMES) {
            assert_eq!(&cap.name, name, "fingerprint drifted from the table");
            assert!(!cap.reads.is_empty(), "{name} declares no inputs");
            assert!(!cap.writes.is_empty(), "{name} declares no outputs");
        }
    }

    #[test]
    fn the_declared_subjects_are_exactly_the_subjects_the_table_uses() {
        let mut used: Vec<u16> = OSM_CAPABILITIES.iter().map(|c| c.subject).collect();
        used.sort_unstable();
        used.dedup();
        let mut declared = OSM_SUBJECT_CLASSIDS.to_vec();
        declared.sort_unstable();
        assert_eq!(used, declared);
    }

    fn good_registration() -> CapabilityRegistration {
        CapabilityRegistration {
            consumer: "osm-soa-bake",
            covered: OSM_CAPABILITY_NAMES,
            subject_classids: OSM_SUBJECT_CLASSIDS,
        }
    }

    #[test]
    fn a_complete_registration_roundtrips() {
        assert!(verify_osm_registration(&good_registration()).is_ok());
    }

    #[test]
    fn the_roundtrip_catches_drift_in_every_direction() {
        // Wrong consumer.
        let mut reg = good_registration();
        reg.consumer = "some-other-crate";
        assert!(matches!(
            verify_osm_registration(&reg),
            Err(RegistrationDrift::UnexpectedConsumer(_))
        ));

        // A declared capability with no consumer arm.
        let mut reg = good_registration();
        reg.covered = &["locate_point"];
        assert!(matches!(
            verify_osm_registration(&reg),
            Err(RegistrationDrift::Uncovered(_))
        ));

        // A consumer arm for a capability this table does not declare.
        let mut reg = good_registration();
        reg.covered = &[
            "locate_point",
            "locate_tile",
            "locate_fragment",
            "project_fields",
            "street_edges",
            "polyline_length",
            "teleport",
        ];
        assert!(matches!(
            verify_osm_registration(&reg),
            Err(RegistrationDrift::Undeclared(_))
        ));

        // A wrong activated-concept set.
        let mut reg = good_registration();
        reg.subject_classids = &[class_ids::OSM_NODE];
        assert!(matches!(
            verify_osm_registration(&reg),
            Err(RegistrationDrift::ClassidMismatch { .. })
        ));
    }
}
