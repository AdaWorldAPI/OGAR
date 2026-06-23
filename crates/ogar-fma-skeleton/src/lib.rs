//! `ogar-fma-skeleton` — the **FMA skeletal spine**: the hand-curated,
//! clamped convergence-anchor atlas.
//!
//! # What this is (operator directive, 2026-06-23)
//!
//! > *"FMA is a must. It must be meticulously optimized in order to later have
//! > stability of the human body — bones not being negotiable. The body is
//! > hardcoded, hand-optimized convergence optimization."*
//!
//! Bones are the only rigid bodies in the human. Soft tissue deforms, breathes,
//! and is noisy; Doppler is velocity, not structure. So the skeleton is not
//! *data being fit* — it is the **boundary condition of the fit**: the rigid,
//! immutable frame onto which the splat-native ultrasound pipeline (and every
//! other modality) registers. Clamping the splat-fit to a hand-curated skeleton
//! makes the convergence well-posed and deterministic — what Class IIa SaMD
//! requires (`OGAR/docs/SPLAT-NATIVE-CUSTOMER.md` §6 litmus).
//!
//! # The two directives, unified
//!
//! 1. **Bones hardcoded & stable.** Each bone is a `kind == Bone` node and is,
//!    by definition, a [`Bone::is_clamped_anchor`] — there is no un-clamped
//!    bone. Its [`Guid`] classid is immutable (RESERVE-DON'T-RECLAIM); a
//!    snapshot test pins it for every bone.
//! 2. **Multi-modal projection.** ViT / X-ray / ultrasound × Doppler are each
//!    a forward operator onto the shared splat volume; all of them register
//!    *through the bone frame*. X-ray *shows* bone (the natural fiducial);
//!    ultrasound returns specularly off bone surfaces; Doppler's view-dependent
//!    velocity is the splat SH term. Request 1 (bones stable) is the
//!    precondition for Request 2 (cross-modal registration).
//!
//! # The address (per [`guid`])
//!
//! Each bone's identity is a 16-byte [`Guid`] — a stack of `[256:256]`
//! `[container:member]` tiers (`src/guid.rs`), **Located-primary** (bones ARE
//! the position anchor):
//!
//! - **classid** tier = `ogar_vocab::class_ids::BONE` (`0x0A03`, Anatomy:bone);
//! - **HEEL/HIP** = the rest-pose **3-axis CRS** (HEEL coronal `x:y`, HIP depth
//!   `z`) — ArcGIS / Cesium-addressable, so the body drops into a GIS stack as
//!   a first-class spatial layer and ViT / ultrasound × Doppler project onto
//!   the same coordinate;
//! - **LEAF** = `familyNode:identity` = `[bodypart : bone]` — the categorical
//!   family (the group) and the instance attached within it.
//!
//! The family node groups (a bone's parent group is its family; `identity == 0`
//! IS the family node, `identity ≥ 1` are its bones), spatial HHTL locates, and
//! the perturbation pyramid's `(exponent, location)` ride the Morton tiles —
//! all on one key. (The lower-level Morton primitives + the variable-depth
//! [`FamilyAddress`] remain in [`morton`] as the value-side splat-refinement
//! tool.)
//!
//! # Honesty fence
//!
//! Rest-pose coordinates are a **canonical proportional T-pose v0** in a body
//! frame (origin at the sacral promontory; `+X` anatomical-left, `+Y` superior,
//! `+Z` anterior; orientation identity). They carry laterality and
//! cranio-caudal order faithfully — enough for the Morton structure and the
//! invariant tests — but are NOT clinically-precise and are to be refined
//! against a real FMA-aligned reference mesh in the splat-native arc. What is
//! rigorous here is the *structure*: tree integrity, the `[container:member]`
//! tier model, HEEL laterality + HIP depth, family-node grouping with
//! attached identity, globally-unique node keys, unit-quaternion frames, the
//! clamped-anchor invariant, and classid stability.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod guid;
pub mod morton;
pub mod projection;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use guid::{Guid, HhtlMode, LeafTile, Tier};
pub use morton::FamilyAddress;
pub use projection::{Modality, ModalityProjection, ProjectionSample, SampleKind};

/// Re-export of the `bone` concept id from the canonical OGAR codebook
/// (`ogar_vocab::class_ids::BONE` = `0x0A03`). It is the classid tier of every
/// bone's [`Guid`] (`0x0A` Anatomy : `0x03` bone).
pub const BONE_CONCEPT: u16 = ogar_vocab::class_ids::BONE;

/// Our stable atlas identity for a node. Distinct from an FMA id: this is the
/// curated node identity. The FMA id (when confidently known) is carried
/// separately in [`BoneSpec::fma`] as a cross-reference — we do not fabricate
/// FMA ids we are unsure of.
pub type AtlasId = u32;

/// The FMA axial / appendicular partition — the top split of the skeleton.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum Region {
    /// Axial skeleton — skull, vertebral column, thoracic cage (the central
    /// spine the body hangs off; the dominant X-ray / ultrasound landmark set).
    Axial,
    /// Upper appendicular — pectoral girdle + arm.
    AppendicularUpper,
    /// Lower appendicular — pelvic girdle + leg.
    AppendicularLower,
}

/// Whether a node is an actual bone (a clamped convergence anchor) or a pure
/// structural grouping (e.g. "vertebral column") that exists only to shape the
/// partonomy / address prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum NodeKind {
    /// An actual bone. **Always a clamped anchor** — bones are non-negotiable.
    Bone,
    /// A structural grouping node (partonomy interior). Not an anchor.
    Group,
}

/// A rigid body-frame transform: translation (metres) + unit-quaternion
/// rotation `(x, y, z, w)`. The rest pose of a bone in the canonical T-pose.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RigidTransform {
    /// Position in the body frame: `+X` anatomical-left, `+Y` superior,
    /// `+Z` anterior. Origin at the sacral promontory.
    pub translation: [f32; 3],
    /// Orientation quaternion `(x, y, z, w)`. v0 rest poses use identity.
    pub rotation: [f32; 4],
}

impl RigidTransform {
    /// Identity orientation at `position`.
    #[must_use]
    pub const fn at(position: [f32; 3]) -> Self {
        Self {
            translation: position,
            rotation: [0.0, 0.0, 0.0, 1.0],
        }
    }

    /// `true` if the rotation is a unit quaternion (within `1e-3`) — the
    /// validity condition for a rigid frame.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let [x, y, z, w] = self.rotation;
        let n2 = x * x + y * y + z * z + w * w;
        (n2 - 1.0).abs() < 1e-3 && self.translation.iter().all(|v| v.is_finite())
    }

    /// The coronal `(x, y)` centroid used for Morton-tile placement
    /// (anatomical-left × superior). This is the plane an X-ray / anterior
    /// ultrasound sweep projects onto.
    #[must_use]
    pub const fn coronal(&self) -> (f32, f32) {
        (self.translation[0], self.translation[1])
    }
}

/// A static atlas row — the hand-curated declaration of one skeletal node.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BoneSpec {
    /// Stable atlas identity (address identity).
    pub id: AtlasId,
    /// FMA cross-reference (`FMA:xxxxx` numeric) where confidently known.
    pub fma: Option<u32>,
    /// Terminologia Anatomica Latin name.
    pub name_la: &'static str,
    /// Common English name.
    pub name_en: &'static str,
    /// Parent node in the partonomy (`None` for the skeletal-system root).
    pub parent: Option<AtlasId>,
    /// Axial / appendicular region.
    pub region: Region,
    /// Bone vs grouping node.
    pub kind: NodeKind,
    /// Rest pose in the canonical body frame.
    pub rest_pose: RigidTransform,
}

/// A resolved node: its [`BoneSpec`] plus the Located-primary [`Guid`] derived
/// from the rest pose + partonomy family.
#[derive(Clone, Debug, PartialEq)]
pub struct Bone {
    spec: BoneSpec,
    guid: Guid,
}

impl Bone {
    /// The stable atlas id.
    #[must_use]
    pub const fn id(&self) -> AtlasId {
        self.spec.id
    }
    /// The FMA cross-reference, when known.
    #[must_use]
    pub const fn fma(&self) -> Option<u32> {
        self.spec.fma
    }
    /// Latin (Terminologia Anatomica) name.
    #[must_use]
    pub const fn name_la(&self) -> &'static str {
        self.spec.name_la
    }
    /// English name.
    #[must_use]
    pub const fn name_en(&self) -> &'static str {
        self.spec.name_en
    }
    /// Parent atlas id.
    #[must_use]
    pub const fn parent(&self) -> Option<AtlasId> {
        self.spec.parent
    }
    /// Region.
    #[must_use]
    pub const fn region(&self) -> Region {
        self.spec.region
    }
    /// Node kind.
    #[must_use]
    pub const fn kind(&self) -> NodeKind {
        self.spec.kind
    }
    /// Rest pose.
    #[must_use]
    pub const fn rest_pose(&self) -> RigidTransform {
        self.spec.rest_pose
    }
    /// The bone's canonical 16-byte [`Guid`] — Located-primary: classid
    /// `0x0A03` (Anatomy:bone) · HEEL/HIP = the rest-pose 3-axis CRS · LEAF =
    /// `familyNode:identity` (`bodypart:bone`).
    #[must_use]
    pub const fn guid(&self) -> Guid {
        self.guid
    }

    /// The HEEL spatial tier (coronal `x:y` plane).
    #[must_use]
    pub const fn heel(&self) -> Tier {
        self.guid.heel()
    }

    /// The LEAF tier as `familyNode:identity`.
    #[must_use]
    pub const fn leaf(&self) -> LeafTile {
        self.guid.leaf()
    }

    /// The family node — the leaf container (the `bodypart` / group). Bones in
    /// the same group share this byte; `identity == 0` IS the group's family
    /// node, `identity ≥ 1` are its bones.
    #[must_use]
    pub const fn family_node(&self) -> u8 {
        self.guid.family_node()
    }

    /// The instance identity within the family (the leaf member).
    #[must_use]
    pub const fn identity(&self) -> u8 {
        self.guid.identity()
    }

    /// The quantised `(x, y, z)` body position decoded from the Located 3D
    /// HEEL/HIP tiers — `+X` anatomical-left, `+Y` superior, `+Z` anterior.
    #[must_use]
    pub fn position(&self) -> (u8, u8, u8) {
        self.guid.position()
    }

    /// The full canonical 16-byte node key (the [`Guid`] bytes).
    #[must_use]
    pub const fn node_key(&self) -> [u8; morton::KEY_BYTES] {
        self.guid.bytes()
    }
    /// `true` iff this node is a clamped convergence anchor — i.e. an actual
    /// bone. **Bones are non-negotiable; there is no un-clamped bone.**
    #[must_use]
    pub const fn is_clamped_anchor(&self) -> bool {
        matches!(self.spec.kind, NodeKind::Bone)
    }
}

/// The resolved skeletal atlas — every node with its derived address.
#[derive(Clone, Debug)]
pub struct Skeleton {
    nodes: Vec<Bone>,
}

impl Skeleton {
    /// Resolve the curated atlas into Located-primary [`Guid`]s: classid
    /// `0x0A03` · HEEL/HIP = the rest-pose 3-axis CRS · LEAF =
    /// `familyNode:identity`. The family node is the node's parent group
    /// (`region nibble · group-index nibble`); the identity is the node's
    /// 1-based rank among its siblings (`0` for the group node itself).
    #[must_use]
    pub fn resolve() -> Self {
        use std::collections::HashMap;
        let specs = atlas();

        let region_code = |r: Region| -> u8 {
            match r {
                Region::Axial => 0,
                Region::AppendicularUpper => 1,
                Region::AppendicularLower => 2,
            }
        };

        // Each group gets a group-index within its region (the family low nibble).
        let mut group_index: HashMap<AtlasId, u8> = HashMap::new();
        let mut region_counter = [0u8; 3];
        for s in &specs {
            if matches!(s.kind, NodeKind::Group) {
                let rc = region_code(s.region) as usize;
                group_index.insert(s.id, region_counter[rc] & 0x0F);
                region_counter[rc] = region_counter[rc].wrapping_add(1);
            }
        }

        // identity = 1-based rank of a bone among its parent's children; a group
        // node takes identity 0 — it IS the family node.
        let mut sibling_counter: HashMap<Option<AtlasId>, u8> = HashMap::new();
        let mut identity_of = vec![0u8; specs.len()];
        for (pos, s) in specs.iter().enumerate() {
            if matches!(s.kind, NodeKind::Bone) {
                let c = sibling_counter.entry(s.parent).or_insert(0);
                *c = c.wrapping_add(1);
                identity_of[pos] = *c;
            }
        }

        // Body bounding box, for the 8-bit spatial quantize (the CRS extent).
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for s in &specs {
            for a in 0..3 {
                min[a] = min[a].min(s.rest_pose.translation[a]);
                max[a] = max[a].max(s.rest_pose.translation[a]);
            }
        }
        let q = |v: f32, lo: f32, hi: f32| -> u8 {
            let span = hi - lo;
            if span <= f32::EPSILON {
                return 0;
            }
            (((v - lo) / span) * 255.0).round().clamp(0.0, 255.0) as u8
        };

        let classid = Tier {
            container: (BONE_CONCEPT >> 8) as u8,
            member: (BONE_CONCEPT & 0xFF) as u8,
        };

        let nodes = specs
            .into_iter()
            .enumerate()
            .map(|(pos, spec)| {
                // Family node = the node's group (parent for bones, self for groups).
                let fam_group = match spec.kind {
                    NodeKind::Group => spec.id,
                    NodeKind::Bone => spec.parent.unwrap_or(spec.id),
                };
                let gi = group_index.get(&fam_group).copied().unwrap_or(0) & 0x0F;
                let family_node = (region_code(spec.region) << 4) | gi;

                let p = spec.rest_pose.translation;
                let g = Guid::located(
                    classid,
                    q(p[0], min[0], max[0]),
                    q(p[1], min[1], max[1]),
                    q(p[2], min[2], max[2]),
                    LeafTile {
                        family_node,
                        identity: identity_of[pos],
                    },
                );

                Bone { spec, guid: g }
            })
            .collect();
        Self { nodes }
    }

    /// All resolved nodes (bones and grouping nodes).
    #[must_use]
    pub fn nodes(&self) -> &[Bone] {
        &self.nodes
    }

    /// Look up a node by atlas id.
    #[must_use]
    pub fn get(&self, id: AtlasId) -> Option<&Bone> {
        self.nodes.iter().find(|b| b.id() == id)
    }

    /// The direct children of `id` in the partonomy.
    pub fn children_of(&self, id: AtlasId) -> impl Iterator<Item = &Bone> {
        self.nodes.iter().filter(move |b| b.parent() == Some(id))
    }

    /// Every clamped convergence anchor — i.e. every actual bone. This is the
    /// non-negotiable rigid frame the splat-fit clamps to.
    pub fn clamped_anchors(&self) -> impl Iterator<Item = &Bone> {
        self.nodes.iter().filter(|b| b.is_clamped_anchor())
    }

    /// `true` if `ancestor` is a partonomy ancestor of `descendant` (walking
    /// the parent links). With Located addressing, partonomy is the parent-link
    /// tree (the categorical family lives in the leaf byte, the spatial HHTL is
    /// the orthogonal location axis), so ancestry is a link walk, not an
    /// address-prefix test.
    #[must_use]
    pub fn is_ancestor(&self, ancestor: AtlasId, descendant: AtlasId) -> bool {
        let mut cur = self.get(descendant).and_then(Bone::parent);
        while let Some(p) = cur {
            if p == ancestor {
                return true;
            }
            cur = self.get(p).and_then(Bone::parent);
        }
        false
    }
}

// ─────────────────────────────────────────────────────────────────────────
// The curated atlas. v0 proportional T-pose (see crate-level honesty fence).
// Atlas-id allocation blocks (stable forever):
//   1        skeletal system (root)
//   10..19   region groups
//   100..199 skull group + cranial bones
//   200..299 vertebral column (200 + ordinal)
//   400..499 thoracic cage (sternum, ribs: 400 + 2*i + side)
//   1000..   appendicular, left  (1000 + offset)
//   2000..   appendicular, right (2000 + offset)
// ─────────────────────────────────────────────────────────────────────────

fn atlas() -> Vec<BoneSpec> {
    let mut v: Vec<BoneSpec> = Vec::new();

    let group = |id, fma, la, en, parent, region| BoneSpec {
        id,
        fma,
        name_la: la,
        name_en: en,
        parent,
        region,
        kind: NodeKind::Group,
        rest_pose: RigidTransform::at([0.0, 0.0, 0.0]),
    };
    let bone = |id, fma, la, en, parent, region, pos: [f32; 3]| BoneSpec {
        id,
        fma,
        name_la: la,
        name_en: en,
        parent,
        region,
        kind: NodeKind::Bone,
        rest_pose: RigidTransform::at(pos),
    };

    use Region::{AppendicularLower, AppendicularUpper, Axial};

    // Roots / groups (poses = region centroids, for sibling Morton placement).
    let mut g = group(
        1,
        Some(23881),
        "systema skeletale",
        "skeletal system",
        None,
        Axial,
    );
    g.rest_pose = RigidTransform::at([0.0, 0.5, 0.0]);
    v.push(g);

    let mut ax = group(
        10,
        Some(23879),
        "skeleton axiale",
        "axial skeleton",
        Some(1),
        Axial,
    );
    ax.rest_pose = RigidTransform::at([0.0, 0.55, 0.0]);
    v.push(ax);
    let mut au = group(
        11,
        None,
        "skeleton appendiculare superius",
        "upper appendicular",
        Some(1),
        AppendicularUpper,
    );
    au.rest_pose = RigidTransform::at([0.30, 0.45, 0.0]);
    v.push(au);
    let mut al = group(
        12,
        None,
        "skeleton appendiculare inferius",
        "lower appendicular",
        Some(1),
        AppendicularLower,
    );
    al.rest_pose = RigidTransform::at([0.0, -0.40, 0.0]);
    v.push(al);

    // Skull group.
    let mut sg = group(
        100,
        Some(46565),
        "cranium",
        "skull (cranium)",
        Some(10),
        Axial,
    );
    sg.rest_pose = RigidTransform::at([0.0, 0.95, 0.0]);
    v.push(sg);
    v.push(bone(
        101,
        Some(46565),
        "neurocranium",
        "neurocranium",
        Some(100),
        Axial,
        [0.0, 1.00, 0.0],
    ));
    v.push(bone(
        102,
        Some(52748),
        "mandibula",
        "mandible",
        Some(100),
        Axial,
        [0.0, 0.88, 0.04],
    ));
    v.push(bone(
        103,
        Some(9613),
        "os hyoideum",
        "hyoid bone",
        Some(100),
        Axial,
        [0.0, 0.82, 0.05],
    ));

    // Vertebral column — the literal spine. C1..C7, T1..T12, L1..L5, sacrum, coccyx.
    let mut vc = group(
        200,
        Some(13478),
        "columna vertebralis",
        "vertebral column",
        Some(10),
        Axial,
    );
    vc.rest_pose = RigidTransform::at([0.0, 0.45, -0.02]);
    v.push(vc);
    // Cranio-caudal stack: y descends from just below skull to the pelvis.
    // (id = 200 + ordinal; ordinal 1..=26 top→bottom.)
    let cervical = [
        (9915u32, "atlas (C1)"),
        (9917, "axis (C2)"),
        (9920, "vertebra C3"),
        (9921, "vertebra C4"),
        (9922, "vertebra C5"),
        (9923, "vertebra C6"),
        (9924, "vertebra C7"),
    ];
    let mut ordinal = 1u32;
    let mut y = 0.86f32;
    for (fma, en) in cervical {
        v.push(bone(
            200 + ordinal,
            Some(fma),
            "vertebra cervicalis",
            en,
            Some(200),
            Axial,
            [0.0, y, -0.01],
        ));
        ordinal += 1;
        y -= 0.022;
    }
    for t in 1..=12u32 {
        let en: &'static str = THORACIC[(t - 1) as usize];
        v.push(bone(
            200 + ordinal,
            None,
            "vertebra thoracica",
            en,
            Some(200),
            Axial,
            [0.0, y, -0.02],
        ));
        ordinal += 1;
        y -= 0.030;
    }
    for l in 1..=5u32 {
        let en: &'static str = LUMBAR[(l - 1) as usize];
        v.push(bone(
            200 + ordinal,
            None,
            "vertebra lumbalis",
            en,
            Some(200),
            Axial,
            [0.0, y, -0.025],
        ));
        ordinal += 1;
        y -= 0.045;
    }
    v.push(bone(
        200 + ordinal,
        Some(16202),
        "os sacrum",
        "sacrum",
        Some(200),
        Axial,
        [0.0, 0.02, -0.02],
    ));
    ordinal += 1;
    v.push(bone(
        200 + ordinal,
        Some(20229),
        "os coccygis",
        "coccyx",
        Some(200),
        Axial,
        [0.0, -0.04, -0.01],
    ));

    // Thoracic cage — sternum + 12 rib pairs.
    let mut tc = group(
        400,
        Some(7480),
        "cavea thoracis",
        "thoracic cage",
        Some(10),
        Axial,
    );
    tc.rest_pose = RigidTransform::at([0.0, 0.52, 0.08]);
    v.push(tc);
    v.push(bone(
        401,
        Some(7485),
        "sternum",
        "sternum",
        Some(400),
        Axial,
        [0.0, 0.55, 0.11],
    ));
    for i in 1..=12u32 {
        // Rib pair i: descends, curves laterally + anteriorly.
        let ry = 0.70 - (i as f32) * 0.028;
        let rx = 0.04 + (i as f32) * 0.009;
        let rz = 0.06;
        v.push(bone(
            400 + 2 * i,
            None,
            "costa",
            RIB_EN[(i - 1) as usize].0,
            Some(400),
            Axial,
            [rx, ry, rz],
        ));
        v.push(bone(
            401 + 2 * i,
            None,
            "costa",
            RIB_EN[(i - 1) as usize].1,
            Some(400),
            Axial,
            [-rx, ry, rz],
        ));
    }

    // Appendicular — pectoral girdle + arm, pelvic girdle + leg, per side.
    for (side_off, parent_upper, parent_lower, sx, name_side) in [
        (1000u32, 11u32, 12u32, 1.0f32, "left"),
        (2000, 11, 12, -1.0, "right"),
    ] {
        // Upper limb.
        v.push(bone(
            side_off + 10,
            Some(13321),
            "clavicula",
            leak("clavicle", name_side),
            Some(parent_upper),
            AppendicularUpper,
            [sx * 0.08, 0.72, 0.04],
        ));
        v.push(bone(
            side_off + 11,
            Some(13394),
            "scapula",
            leak("scapula", name_side),
            Some(parent_upper),
            AppendicularUpper,
            [sx * 0.12, 0.62, -0.06],
        ));
        v.push(bone(
            side_off + 12,
            Some(13303),
            "humerus",
            leak("humerus", name_side),
            Some(parent_upper),
            AppendicularUpper,
            [sx * 0.18, 0.45, 0.0],
        ));
        v.push(bone(
            side_off + 13,
            Some(23463),
            "radius",
            leak("radius", name_side),
            Some(parent_upper),
            AppendicularUpper,
            [sx * 0.22, 0.18, 0.02],
        ));
        v.push(bone(
            side_off + 14,
            Some(23466),
            "ulna",
            leak("ulna", name_side),
            Some(parent_upper),
            AppendicularUpper,
            [sx * 0.20, 0.17, 0.0],
        ));
        // Lower limb.
        v.push(bone(
            side_off + 20,
            Some(16585),
            "os coxae",
            leak("hip bone", name_side),
            Some(parent_lower),
            AppendicularLower,
            [sx * 0.10, 0.0, 0.0],
        ));
        v.push(bone(
            side_off + 21,
            Some(9611),
            "femur",
            leak("femur", name_side),
            Some(parent_lower),
            AppendicularLower,
            [sx * 0.10, -0.25, 0.0],
        ));
        v.push(bone(
            side_off + 22,
            Some(24485),
            "patella",
            leak("patella", name_side),
            Some(parent_lower),
            AppendicularLower,
            [sx * 0.10, -0.46, 0.05],
        ));
        v.push(bone(
            side_off + 23,
            Some(12856),
            "tibia",
            leak("tibia", name_side),
            Some(parent_lower),
            AppendicularLower,
            [sx * 0.09, -0.66, 0.01],
        ));
        v.push(bone(
            side_off + 24,
            Some(24479),
            "fibula",
            leak("fibula", name_side),
            Some(parent_lower),
            AppendicularLower,
            [sx * 0.12, -0.66, 0.0],
        ));
    }

    v
}

/// Build a `&'static str` "left/right <bone>" name. Leaks intentionally — the
/// atlas is built once per process and the resolved [`Skeleton`] is long-lived;
/// the handful of leaked names is bounded and never grows after `resolve`.
fn leak(base: &str, side: &str) -> &'static str {
    Box::leak(format!("{side} {base}").into_boxed_str())
}

const THORACIC: [&str; 12] = [
    "vertebra T1",
    "vertebra T2",
    "vertebra T3",
    "vertebra T4",
    "vertebra T5",
    "vertebra T6",
    "vertebra T7",
    "vertebra T8",
    "vertebra T9",
    "vertebra T10",
    "vertebra T11",
    "vertebra T12",
];
const LUMBAR: [&str; 5] = [
    "vertebra L1",
    "vertebra L2",
    "vertebra L3",
    "vertebra L4",
    "vertebra L5",
];
/// `(left, right)` English names for rib pairs 1..=12.
const RIB_EN: [(&str, &str); 12] = [
    ("left rib 1", "right rib 1"),
    ("left rib 2", "right rib 2"),
    ("left rib 3", "right rib 3"),
    ("left rib 4", "right rib 4"),
    ("left rib 5", "right rib 5"),
    ("left rib 6", "right rib 6"),
    ("left rib 7", "right rib 7"),
    ("left rib 8", "right rib 8"),
    ("left rib 9", "right rib 9"),
    ("left rib 10", "right rib 10"),
    ("left rib 11", "right rib 11"),
    ("left rib 12", "right rib 12"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn skel() -> Skeleton {
        Skeleton::resolve()
    }

    #[test]
    fn bone_concept_resolves_in_codebook() {
        assert_eq!(BONE_CONCEPT, 0x0A03);
        assert_eq!(
            ogar_vocab::canonical_concept_domain(BONE_CONCEPT),
            ogar_vocab::ConceptDomain::Anatomy,
        );
    }

    #[test]
    fn partonomy_is_a_tree() {
        let s = skel();
        let ids: HashSet<AtlasId> = s.nodes().iter().map(Bone::id).collect();
        assert_eq!(ids.len(), s.nodes().len(), "atlas ids are unique");
        let mut roots = 0;
        for b in s.nodes() {
            match b.parent() {
                None => roots += 1,
                Some(p) => assert!(ids.contains(&p), "{} parent {p} missing", b.name_en()),
            }
        }
        assert_eq!(roots, 1, "exactly one skeletal-system root");
    }

    fn by_name<'a>(s: &'a Skeleton, name: &str) -> &'a Bone {
        s.nodes()
            .iter()
            .find(|b| b.name_en() == name)
            .unwrap_or_else(|| panic!("{name} present"))
    }

    #[test]
    fn classid_tier_is_anatomy_bone() {
        // Every node's GUID routes on the `bone` concept (Anatomy:bone, 0x0A03).
        let s = skel();
        for b in s.nodes() {
            assert_eq!(b.guid().classid(), 0x0A03, "{}", b.name_en());
            let t = b.guid().tier(guid::TIER_CLASSID);
            assert_eq!((t.container, t.member), (0x0A, 0x03));
        }
    }

    #[test]
    fn crs_encodes_laterality_and_depth() {
        // Located 3D CRS: the decoded position recovers anatomical axes.
        // Laterality (X): left vs right femur. Depth (Z): sternum (anterior) vs
        // a thoracic vertebra (posterior). Decoding through the 3D-octree HEEL/HIP
        // is the inverse the GIS / projection layer uses.
        let s = skel();
        let left_femur = s.get(1021).unwrap();
        let right_femur = s.get(2021).unwrap();
        let (lx, _, _) = left_femur.position();
        let (rx, _, _) = right_femur.position();
        assert_ne!(lx, rx, "laterality recovered from the 3D CRS");
        assert_ne!(left_femur.heel(), right_femur.heel(), "and visible in HEEL");
        assert!(
            left_femur.guid().same_family(&right_femur.guid()),
            "yet the SAME categorical family (spatial is orthogonal)",
        );
        assert_ne!(left_femur.identity(), right_femur.identity());

        let (_, _, sternum_z) = by_name(&s, "sternum").position();
        let (_, _, vertebra_z) = by_name(&s, "vertebra T6").position();
        assert!(
            sternum_z > vertebra_z,
            "sternum is anterior (+Z) of the vertebra: {sternum_z} vs {vertebra_z}",
        );
    }

    #[test]
    fn cascade_ontology_guid_builds_the_self_speaking_path() {
        // The soft-tissue (Cascade) mode: the papillary muscle's
        // ANAT0001-CARD-HERT-LVNT-PAPMUS-<id>, no spatial address. Shares the
        // tier algebra with the Located bones; only the HHTL reading differs.
        let classid = Tier {
            container: 0x0A,
            member: 0x10,
        }; // Anatomy : muscle (illustrative)
        let g = Guid::ontological(
            classid,
            guid::ontology_tier(0x01, 0x02), // Cardiovascular : Heart
            guid::ontology_tier(0x02, 0x03), // Heart : LeftVentricle
            guid::ontology_tier(0x03, 0x07), // LeftVentricle : PapillaryMuscleFamily
            LeafTile {
                family_node: 0x07,
                identity: 0x1D,
            },
        );
        assert_eq!(g.classid(), 0x0A10);
        assert_eq!(
            g.leaf(),
            LeafTile {
                family_node: 0x07,
                identity: 0x1D
            }
        );
        // A sibling papillary muscle: same family, identity attached.
        let sibling = Guid::ontological(
            classid,
            guid::ontology_tier(0x01, 0x02),
            guid::ontology_tier(0x02, 0x03),
            guid::ontology_tier(0x03, 0x07),
            LeafTile {
                family_node: 0x07,
                identity: 0x1E,
            },
        );
        assert!(g.same_family(&sibling));
        assert_ne!(g.identity(), sibling.identity());
    }

    #[test]
    fn leaf_is_family_identity_and_node_keys_unique() {
        let s = skel();
        let mut keys: HashSet<[u8; morton::KEY_BYTES]> = HashSet::new();
        let mut leaves: HashSet<(u8, u8)> = HashSet::new();
        for b in s.nodes() {
            assert!(
                keys.insert(b.node_key()),
                "node-key collision on {}",
                b.name_en()
            );
            assert!(
                leaves.insert((b.family_node(), b.identity())),
                "leaf family:identity collision on {}",
                b.name_en()
            );
        }
    }

    #[test]
    fn family_node_groups_bones_identity_attached() {
        // Two vertebrae share the vertebral-column family node; their identities
        // differ ("identity attached to family node"). A femur is a different
        // family. The group node itself is identity 0 — it IS the family node.
        let s = skel();
        let t1 = by_name(&s, "vertebra T1");
        let t2 = by_name(&s, "vertebra T2");
        let femur = s.get(1021).unwrap();
        let column = s.get(200).unwrap(); // vertebral column group

        assert!(t1.guid().same_family(&t2.guid()), "same family node");
        assert_ne!(
            t1.identity(),
            t2.identity(),
            "identity discriminates within"
        );
        assert!(!t1.guid().same_family(&femur.guid()));

        assert_eq!(column.identity(), 0, "the group node IS the family node");
        assert_eq!(column.family_node(), t1.family_node(), "its bones share it");
        assert!(t1.identity() >= 1, "bones are members (identity >= 1)");
    }

    #[test]
    fn every_bone_is_a_clamped_anchor() {
        // The operator's "bones not negotiable" as an invariant: no un-clamped bone.
        let s = skel();
        let anchors = s.clamped_anchors().count();
        assert!(
            anchors >= 60,
            "expected a meticulous skeleton, got {anchors} anchors"
        );
        for b in s.nodes() {
            if matches!(b.kind(), NodeKind::Bone) {
                assert!(
                    b.is_clamped_anchor(),
                    "{} is a bone but not clamped",
                    b.name_en()
                );
            }
        }
    }

    #[test]
    fn rest_frames_are_valid_rigid_transforms() {
        for b in skel().nodes() {
            assert!(
                b.rest_pose().is_valid(),
                "{} has a non-unit rotation",
                b.name_en()
            );
        }
    }

    #[test]
    fn partonomy_ancestry_walks_parent_links() {
        let s = skel();
        let sacrum = by_name(&s, "sacrum").id();
        assert!(s.is_ancestor(200, sacrum), "vertebral column ⊃ sacrum");
        assert!(
            s.is_ancestor(1, sacrum),
            "skeletal system ⊃ sacrum (transitive)"
        );
        assert!(
            !s.is_ancestor(200, 1021),
            "column is not an ancestor of the femur"
        );
    }

    #[test]
    fn classid_stability_snapshot() {
        // Pin the classid tier: every bone routes on Anatomy:bone. A change that
        // moves this breaks the immutability the splat-fit relies on.
        let s = skel();
        for b in s.clamped_anchors() {
            assert_eq!(b.guid().classid(), 0x0A03);
        }
    }
}
