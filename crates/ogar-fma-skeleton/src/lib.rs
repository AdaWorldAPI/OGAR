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
//!    bone. Its [`FamilyAddress`] is immutable: refinement may *extend* depth
//!    (finer Morton levels) but never rewrite an assigned coarse nibble
//!    (RESERVE-DON'T-RECLAIM). The [`stability`](crate#tests) snapshot test
//!    pins the address of every bone.
//! 2. **Multi-modal projection.** ViT / X-ray / ultrasound × Doppler are each
//!    a forward operator onto the shared splat volume; all of them register
//!    *through the bone frame*. X-ray *shows* bone (the natural fiducial);
//!    ultrasound returns specularly off bone surfaces; Doppler's view-dependent
//!    velocity is the splat SH term. Request 1 (bones stable) is the
//!    precondition for Request 2 (cross-modal registration).
//!
//! # The address (per [`morton`])
//!
//! Each bone's identity is a **16×8-bit family-node key** — a uniform 4×4
//! Morton-tile pyramid. The address is *derived from the rest-pose centroid*
//! by descending the Morton quadtree of each parent's children, so
//! `parent.address.is_ancestor_of(child.address)` holds **and** spatially-near
//! siblings share a longer Morton prefix. Partonomy, spatial mipmap, and the
//! perturbation pyramid's `(exponent, location)` are one address. The concept
//! routing prefix is `ogar_vocab::class_ids::BONE` (`0x0A03`).
//!
//! # Honesty fence
//!
//! Rest-pose coordinates are a **canonical proportional T-pose v0** in a body
//! frame (origin at the sacral promontory; `+X` anatomical-left, `+Y` superior,
//! `+Z` anterior; orientation identity). They carry laterality and
//! cranio-caudal order faithfully — enough for the Morton structure and the
//! invariant tests — but are NOT clinically-precise and are to be refined
//! against a real FMA-aligned reference mesh in the splat-native arc. What is
//! rigorous here is the *structure*: tree integrity, prefix=spatial=partonomy
//! containment, uniform-stride Morton keys, unit-quaternion frames, the
//! clamped-anchor invariant, and address stability.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod guid;
pub mod morton;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use guid::{EdgeBlock, Guid, HhtlMode, LeafTile, Tier};
pub use morton::FamilyAddress;

/// Re-export of the `bone` concept id from the canonical OGAR codebook
/// (`ogar_vocab::class_ids::BONE` = `0x0A03`). The high nibbles of every
/// bone's [`FamilyAddress`] are this concept's routing prefix.
pub const BONE_CONCEPT: u16 = ogar_vocab::class_ids::BONE;

/// Our stable atlas identity for a node. Distinct from an FMA id: this is the
/// address identity the [`FamilyAddress`] and stability test pin. The FMA id
/// (when confidently known) is carried separately in [`BoneSpec::fma`] as a
/// cross-reference — we do not fabricate FMA ids we are unsure of.
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

/// A resolved node: its [`BoneSpec`] plus the [`FamilyAddress`] derived from
/// the partonomy + Morton placement.
#[derive(Clone, Debug, PartialEq)]
pub struct Bone {
    spec: BoneSpec,
    address: FamilyAddress,
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
    /// The immutable 16×8-bit Morton-tile family-node address — the **routing
    /// prefix** (concept routing + partonomy + spatial Morton). An interior
    /// node's address is intentionally a byte-prefix of its descendants'; the
    /// leaf discriminator is the identity tail (see [`node_key`](Self::node_key)).
    #[must_use]
    pub const fn address(&self) -> FamilyAddress {
        self.address
    }

    /// The full canonical 16-byte node key: the Morton routing prefix with this
    /// node's atlas id written into the **identity tail** (bytes 13..16, the
    /// canon's 24-bit `identity` field, little-endian). This is globally unique
    /// per node even when two nodes share a routing prefix (ancestor ↔
    /// descendant, or a corner-tile group ↔ child). The 24-bit field holds atlas
    /// ids up to `0xFF_FFFF`.
    #[must_use]
    pub fn node_key(&self) -> [u8; morton::KEY_BYTES] {
        let mut k = self.address.bytes();
        let id = self.spec.id;
        k[13] = (id & 0xFF) as u8;
        k[14] = ((id >> 8) & 0xFF) as u8;
        k[15] = ((id >> 16) & 0xFF) as u8;
        k
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
    /// Resolve the curated atlas: build the partonomy tree and derive every
    /// node's [`FamilyAddress`] by descending the Morton quadtree of each
    /// parent's children (so the address is the rest-pose centroid's position,
    /// prefixed by the partonomy, prefixed by the `bone` concept routing).
    #[must_use]
    pub fn resolve() -> Self {
        let specs = atlas();
        let n = specs.len();

        // Index by atlas id.
        let index = |id: AtlasId| specs.iter().position(|s| s.id == id);

        // children[parent_pos] = [child positions], in spec order.
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut roots: Vec<usize> = Vec::new();
        for (pos, s) in specs.iter().enumerate() {
            match s.parent.and_then(index) {
                Some(pp) => children[pp].push(pos),
                None => roots.push(pos),
            }
        }

        // The concept routing prefix shared by every node: the `bone` u16 as
        // four Morton-tile nibbles (hi nibble first).
        let concept = BONE_CONCEPT;
        let concept_prefix = [
            ((concept >> 12) & 0xF) as u8,
            ((concept >> 8) & 0xF) as u8,
            ((concept >> 4) & 0xF) as u8,
            (concept & 0xF) as u8,
        ];

        let mut address: Vec<FamilyAddress> = vec![FamilyAddress::ROOT; n];
        let base = FamilyAddress::from_nibbles(&concept_prefix);

        // BFS from the roots, assigning Morton suffixes per sibling group.
        let mut stack: Vec<(usize, FamilyAddress)> = Vec::new();

        // Multiple roots are themselves a sibling group under the empty
        // concept-prefixed base.
        let root_centroids: Vec<(f32, f32)> = roots
            .iter()
            .map(|&p| specs[p].rest_pose.coronal())
            .collect();
        let root_suffixes = morton::assign_morton_suffixes(&root_centroids);
        for (k, &p) in roots.iter().enumerate() {
            let addr = base.extend(&root_suffixes[k]);
            address[p] = addr;
            stack.push((p, addr));
        }

        while let Some((pos, addr)) = stack.pop() {
            let kids = &children[pos];
            if kids.is_empty() {
                continue;
            }
            let centroids: Vec<(f32, f32)> =
                kids.iter().map(|&c| specs[c].rest_pose.coronal()).collect();
            let suffixes = morton::assign_morton_suffixes(&centroids);
            for (k, &c) in kids.iter().enumerate() {
                let child_addr = addr.extend(&suffixes[k]);
                address[c] = child_addr;
                stack.push((c, child_addr));
            }
        }

        let nodes = specs
            .into_iter()
            .enumerate()
            .map(|(pos, spec)| Bone {
                spec,
                address: address[pos],
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

    /// `true` if `ancestor` is a partonomy/spatial ancestor of `descendant`
    /// (by Morton-address prefix containment).
    #[must_use]
    pub fn is_ancestor(&self, ancestor: AtlasId, descendant: AtlasId) -> bool {
        match (self.get(ancestor), self.get(descendant)) {
            (Some(a), Some(d)) => a.address().is_ancestor_of(&d.address()),
            _ => false,
        }
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
    use std::collections::{HashMap, HashSet};

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

    #[test]
    fn address_prefix_is_partonomy_containment() {
        // The load-bearing canon claim: parent's Morton address is a prefix of
        // each child's. Partonomy ⟺ address-prefix ⟺ spatial containment.
        let s = skel();
        for b in s.nodes() {
            if let Some(p) = b.parent() {
                let parent = s.get(p).unwrap();
                assert!(
                    parent.address().is_ancestor_of(&b.address()),
                    "{} addr {:?} not under parent {} addr {:?}",
                    b.name_en(),
                    b.address(),
                    parent.name_en(),
                    parent.address(),
                );
                assert!(b.address().depth() > parent.address().depth());
            }
        }
    }

    #[test]
    fn sibling_addresses_are_distinct() {
        // The real Morton invariant: within any parent's children, the derived
        // addresses differ. assign_morton_suffixes guarantees this for distinct
        // centroids, so a failure here means two siblings have coincident
        // rest-pose centroids — the one bug class to fix in the atlas data.
        let s = skel();
        let parents: HashSet<AtlasId> = s.nodes().iter().filter_map(Bone::parent).collect();
        for p in parents {
            let mut seen: HashSet<(Vec<u8>, u8)> = HashSet::new();
            for child in s.children_of(p) {
                let a = child.address();
                let nibbles: Vec<u8> = (0..a.depth() as usize).map(|l| a.nibble(l)).collect();
                assert!(
                    seen.insert((nibbles, a.depth())),
                    "siblings under {p} share an address — coincident centroids on {}",
                    child.name_en(),
                );
            }
        }
    }

    #[test]
    fn full_node_keys_are_globally_unique() {
        // The leaf discriminator: the identity tail makes every node's full
        // 16-byte key unique even when routing prefixes nest (ancestor ↔
        // descendant, corner-tile group ↔ child).
        let s = skel();
        let mut seen: HashMap<[u8; morton::KEY_BYTES], AtlasId> = HashMap::new();
        for b in s.nodes() {
            if let Some(prev) = seen.insert(b.node_key(), b.id()) {
                panic!("node-key collision: {} and {}", prev, b.id());
            }
        }
        // And the identity tail never clobbers the (shallow) Morton prefix:
        // routing prefix and identity tail occupy disjoint byte ranges here.
        for b in s.nodes() {
            assert!(
                b.address().depth() as usize <= 26,
                "{} routing prefix would reach the identity tail",
                b.name_en(),
            );
        }
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
    fn morton_address_encodes_laterality() {
        // Projection precondition: left and right twins must differ in their
        // address (the X axis of the Morton tile = anatomical laterality), so an
        // X-ray / ultrasound sweep registering onto the coronal plane can tell
        // sides apart from the address prefix alone.
        let s = skel();
        let left_femur = s.get(1021).unwrap(); // 1000-block, +21
        let right_femur = s.get(2021).unwrap();
        assert!(left_femur.address().bytes() != right_femur.address().bytes());
        // Their nearest common ancestor is the lower-appendicular group, so they
        // share that prefix but diverge below it.
        let cpl = left_femur
            .address()
            .common_prefix_len(&right_femur.address());
        assert!(cpl >= 4, "share at least the bone-concept routing prefix");
        assert!(
            cpl < left_femur.address().depth(),
            "but diverge before the leaf (laterality split)",
        );
    }

    #[test]
    fn address_stability_snapshot() {
        // RESERVE-DON'T-RECLAIM: pin a few well-known bones' addresses. A change
        // that *moves* (rather than extends) an address breaks this — the
        // immutability guarantee the splat-fit relies on. Refinement that only
        // deepens an address (adds finer Morton levels) without rewriting a
        // coarse nibble keeps the OLD address as a prefix and would update the
        // snapshot deliberately.
        let s = skel();
        // The four concept-prefix nibbles are shared by every node.
        for b in s.clamped_anchors() {
            let a = b.address();
            assert_eq!(a.nibble(0), 0x0);
            assert_eq!(a.nibble(1), 0xA);
            assert_eq!(a.nibble(2), 0x0);
            assert_eq!(
                a.nibble(3),
                0x3,
                "every bone carries the `bone` concept routing prefix 0x0A03"
            );
        }
        // Sacrum sits at the column root spatially (near the body origin); it is
        // a descendant of the vertebral-column group.
        assert!(s.is_ancestor(200, sacrum_id(&s)));
    }

    fn sacrum_id(s: &Skeleton) -> AtlasId {
        s.nodes()
            .iter()
            .find(|b| b.name_en() == "sacrum")
            .map(Bone::id)
            .expect("sacrum present")
    }
}
