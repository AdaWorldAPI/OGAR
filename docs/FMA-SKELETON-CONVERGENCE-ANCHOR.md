# FMA Skeleton — the Clamped Convergence Anchor

> **Status:** CODED (crate + tests + CI) for the address/atlas structure;
> CONJECTURE for the splat-fit convergence claim (gated on the splat-native arc).
> **Authored:** 2026-06-23 (operator directive session).
> **Home crate:** `crates/ogar-fma-skeleton`.
> **Codebook:** `ogar_vocab::class_ids::{ANATOMICAL_STRUCTURE, SKELETON, BONE, JOINT}`
> (`0x0A01..0x0A04`, the new `ConceptDomain::Anatomy`).
> **Consumes:** `D-MORTON` (DISCOVERY-MAP §2.1 — nibble = one 4×4 Morton tile).
> **Customer:** `docs/SPLAT-NATIVE-CUSTOMER.md` §6 FMA litmus.

---

## 0. The operator directive (2026-06-23)

> *"FMA is a must. It must be meticulously optimized in order to later have
> stability of the human body — bones not being negotiable. The body is
> hardcoded, hand-optimized convergence optimization. Secondly, we want to
> project ViT or X-ray and especially ultrasound × Doppler."*

Two requirements that are **one constraint**:

> Bones must be hardcoded and stable **because** they are the cross-modal
> registration frame that ViT, X-ray, and ultrasound × Doppler all project
> onto. Request 1 (bones stable) is the *precondition* for Request 2
> (cross-modal projection).

The skeleton is the only rigid body in the human. Soft tissue deforms and
breathes; Doppler is velocity, not structure. So the skeleton is **not data
being fit** — it is the **boundary condition of the fit**: a hand-curated,
immutable, rigid frame. Clamping the splat-fit to it makes the optimization
well-posed and *deterministic* — the Class IIa SaMD requirement.

---

## 1. Bones as clamped Dirichlet anchors

In 3D-Gaussian-Splatting-style fitting, free Gaussians drift; the optimization
is ill-posed without anchors. The FMA skeleton supplies them:

- **Bone Gaussians are clamped** — zero free parameters (Dirichlet boundary).
- **Soft-tissue / Doppler splats are deformable children** registered to a bone
  parent (linear-blend-skinning / as-rigid-as-possible against the skeleton).
- The fit converges stably because the rigid frame is non-negotiable.

In code this is an invariant, not a flag: `Bone::is_clamped_anchor()` is `true`
for **every** `NodeKind::Bone`. There is no un-clamped bone. The
`every_bone_is_a_clamped_anchor` test enforces it.

---

## 2. The address — 16×8-bit Morton-tile family nodes (operator correction)

The original plan modelled the cascade path as opaque tree-branch nibbles. That
**throws away the spatial structure** — "the data we save we will lose by not
adhering to the 2bit×2bit 4×4 Morton tile pyramid perturbation shader cascade."
Corrected:

- The key is a **uniform 16-byte (16×8-bit) family-node array** — never a
  heterogeneous `12+4` carve. Uniformity is load-bearing: it is exactly why the
  operator reversed the `4/3/3` v8-native carve (CLAUDE.md §"3×4 PATH" —
  "broke the uniform Morton stride").
- **Each nibble is one 4×4 Morton tile**: 2 bits X interleaved with 2 bits Y.
  16 bytes ⇒ 32 nibble-levels ⇒ a 32-level pyramid of 4×4 spatial refinements,
  uniform stride (`tier = level >> 2`, a shift, never a branch).
- A bone's address is **derived from its rest-pose centroid** by descending the
  Morton quadtree of each parent's children (`morton::assign_morton_suffixes`).

### 2.1 The one-address collapse (D-BOTHCASC, realized)

Because each nibble is the 4×4 Morton tile of a child's position within its
parent's bounding box, **nibble-prefix containment is simultaneously**:

1. **Partonomy containment** — `parent.address.is_ancestor_of(child.address)`
   holds by construction (`address_prefix_is_partonomy_containment` test).
2. **Spatial containment** — spatially-near siblings share a longer Morton
   prefix (`morton_address_encodes_laterality` test: left/right twins diverge on
   the X-tile but share the region prefix).

FMA partonomy, spatial mipmap, and the perturbation pyramid's
`(exponent, location)` are **one address**.

### 2.2 The perturbation-shader split (OGAR vs ndarray)

The stacked perturbation decomposes as `(exponent, location, phase, magnitude)`:

| Component | Where | Carried by |
|---|---|---|
| `exponent` | OGAR address | the pyramid level (`tier = level >> 2`) |
| `location` | OGAR address | the 4×4 Morton tile at that level |
| `phase` | ndarray (splat) | deterministic recurrence from the address (never stored) |
| `magnitude` | ndarray (splat) | palette-quantized residual envelope (the only stored bits) |

OGAR owns `(exponent, location)` — the *address*. ndarray owns
`(phase, magnitude)` — the *shader residual*. This keeps the OGAR job
(addressing) clean and defers the splat residuals to the SIMD layer
(architecture rule: ndarray = hardware, OGAR = address).

### 2.3 Immutability (RESERVE-DON'T-RECLAIM)

The Morton **routing prefix** is the address; the leaf discriminator is the
canon's 24-bit `identity` tail (`Bone::node_key`, bytes 13..16 = atlas id LE).
An interior node's address is intentionally a byte-prefix of its descendants'.
Refinement may **extend** depth (add finer Morton levels) but never rewrite an
assigned coarse nibble. The `address_stability_snapshot` +
`full_node_keys_are_globally_unique` tests pin this.

---

## 3. Multi-modal projection through the bone frame

Each modality is a forward operator onto the shared splat volume; **all register
through the bone frame** — which is *why* bones must be stable:

| Modality | Forward operator | Registers via |
|---|---|---|
| **X-ray** | Radon line-integral (2D projection) | bones directly — X-ray *shows* bone; the natural skeletal fiducial |
| **Ultrasound** | PSF-convolved reflectivity along the beam (anisotropic Σ) | bone surfaces (strong specular returns) |
| **Doppler** | velocity field → **view-dependent appearance** | spherical-harmonics by physics — Doppler *is* view-dependent because flow is; the splat SH term |
| **ViT** | learned 2D features lifted into 3D (feature-splat) | bone landmarks as anchor tokens |

The coronal `(x, y)` plane the Morton tiles encode (`RigidTransform::coronal`)
is exactly the plane an X-ray or anterior ultrasound sweep projects onto — so
laterality is recoverable from the address prefix alone, before any value
decode.

---

## 4. Why Anatomy is its own codebook domain (`0x0A`), not Health

The bones/skeleton concepts are **public anatomical reference structure** (the
femur exists; it is `part_of` the lower limb). A clinical *finding about*
anatomy (a fracture diagnosis on a named patient) is Health PHI; the structure
itself is not. Putting anatomy in `0x09` Health would wrongly drag it into
medcare-rs's fail-closed Health RBAC coverage set (and break its `Health == 7`
invariant). So anatomy gets `ConceptDomain::Anatomy` (`0x0A`) — reference ≠ PHI.
This is the same firewall split the splat-native arc draws: the **atlas** is
public reference (Anatomy); the **patient splat** is PHI (Health / MedCare wire).

---

## 5. Honesty fence

- **CODED [G]:** the address algebra (Morton tile encode/decode, prefix
  containment, uniform 16-byte key), the partonomy tree, the clamped-anchor
  invariant, unit-quaternion frames, address stability, laterality encoding —
  all under `cargo test -p ogar-fma-skeleton` (16 tests).
- **v0 / proportional [H]:** rest-pose coordinates are a canonical proportional
  T-pose in a body frame (origin sacral promontory; `+X` left, `+Y` superior,
  `+Z` anterior; orientation identity). They carry laterality and cranio-caudal
  order faithfully — enough for the Morton structure — but are **not
  clinically precise** and are to be refined against a real FMA-aligned
  reference mesh in the splat-native arc (D-SPLAT-8 hydrator).
- **CONJECTURE [H]:** the splat-fit convergence-stability claim (§1) is
  asserted from the well-posedness argument, not yet measured. It is gated on
  the splat-native arc's registration loop (`SPLAT-NATIVE-CUSTOMER.md` §3
  acceptance gate).
- **Atlas coverage:** the curated atlas is the axial skeleton (skull, full
  vertebral column C1–coccyx, sternum, 12 rib pairs) + the major appendicular
  bones (clavicle, scapula, humerus, radius, ulna, os coxae, femur, patella,
  tibia, fibula, per side) — ~80 nodes, structured to extend to the full ~206
  without schema change. FMA ids are cross-referenced only where confidently
  known; the address identity is the stable atlas id, never a fabricated FMA id.

---

## 5b. The GUID tier model — `[container:member]`, located vs cascade (2026-06-23)

The address is a uniform stack of `[256:256]` tiers, each the **same relation
at every scale**: `[container : member]` (`Galaxy:planet`, `country:city`,
`school:student`, `bodypart:bone`, `cm²:mm²`, `residue:atom`). The high byte is
the coarse container (a 256-codebook), the low byte the member attached within
it. See `src/guid.rs` (`Guid`, `Tier`, `LeafTile`).

```text
[classid] [HEEL] [HIP] [TWIG] [LEAF=familyNode:identity]   tiers, each [256:256]
 0x0A:03   …      …     …      bodypart : bone
```

**The HEEL has two modes** (`HhtlMode`) — the operator's "heel" distinction:

| mode | HEEL holds | property | who |
|---|---|---|---|
| `Located` (Cesium) | the *literal* heel — a real position (`heel:muscle`) | **preserves location**; HHTL = `spatial_tier` Morton | **bones** — they ARE the anchor |
| `Cascade` (ontology) | a classification rung (`Anatomy…PapMuscle`) | **no spatial address** — pure containment; HHTL = `ontology_tier` | **muscle / soft tissue** — classified, then projected onto |

Both share the LEAF `familyNode:identity` and the 16-byte key; the mode is a
*reading* of the HHTL block, not a different structure. The self-speaking
ontology GUID (`ANAT0001-CARD-HERT-LVNT-PAPMUS-7A3F9C1D`) is the Cascade
reading; the bone's spatial key is the Located reading.

**The `12+4` edge block** (`EdgeBlock`): every family node carries **12
in-family** local relations (sibling/parent/child family-node codes) + **4
out-of-family** inherited connector interfaces (e.g. fibrous skeleton, vascular
supply, innervation, ECM scaffold). An instance inherits the family node's edge
block as a template and adds its own residue. This is the canonical node edge
block (12 in-family + 4 out-of-family, one byte per slot).

**The splat projects onto the class.** The 4D ultrasound×Doppler splat is
*mapped onto* the anatomy class addressed by this GUID — semantic + spatial +
dynamic unified on one key, no information destroyed, longitudinal tracking via
the stable identity.

## 6. Cross-references

- `crates/ogar-fma-skeleton/src/morton.rs` — the 4×4 Morton-tile pyramid.
- `crates/ogar-fma-skeleton/src/lib.rs` — the curated atlas + partonomy API.
- `crates/ogar-vocab/src/lib.rs` — `ConceptDomain::Anatomy`, `class_ids::BONE`.
- `docs/SPLAT-NATIVE-CUSTOMER.md` — the §6 FMA litmus + SaMD evidence base.
- `docs/DISCOVERY-MAP.md` — `D-MORTON` (§2.1), `D-FMA-SKELETON`.
- CLAUDE.md §"Tier interpretation", §"Perturbation encoding", §"Bipolar-phase
  pyramid", §"3×4 PATH — UNIFORM".
