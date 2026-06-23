//! `projection.rs` — the **modality projection contract**.
//!
//! The seam where "address the heart muscle with ArcGIS, measure the sinus with
//! ViT" lands: ViT / X-ray / ultrasound × Doppler each implement one trait —
//! **register** the acquisition to the bone frame (bones are the rigid
//! fiducials), then **project** measurements addressed by the canonical
//! [`Guid`] onto the anatomy. The address is the join key that lets
//! heterogeneous sensors write to the same node (no information destroyed;
//! longitudinal tracking by stable id).
//!
//! This module is a **contract** (trait + types) plus one worked example
//! ([`XrayBoneFiducial`]) proving the shape compiles. Real ViT / ultrasound
//! engines live downstream (ndarray / the splat layer); they implement
//! [`ModalityProjection`] against this surface.

use crate::{Guid, RigidTransform, Skeleton};

/// An imaging / sensing modality projecting onto the FMA-addressed anatomy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Modality {
    /// Projectional radiography — a Radon line-integral; *shows* bone, the
    /// natural skeletal fiducial.
    Xray,
    /// B-mode ultrasound — PSF-convolved reflectivity along the beam;
    /// registers off bone surfaces (specular returns).
    Ultrasound,
    /// Doppler — a velocity field; view-dependent by physics (the splat SH term).
    Doppler,
    /// Vision Transformer — learned 2D features lifted into 3D, anchored at
    /// bone landmarks.
    Vit,
}

/// The physical quantity a [`ProjectionSample`] carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SampleKind {
    /// X-ray attenuation along the ray.
    Attenuation,
    /// Ultrasound echo amplitude (structure).
    Structure,
    /// Doppler flow velocity.
    Flow,
    /// A learned ViT feature score.
    Feature,
}

/// One measurement, **addressed by the canonical [`Guid`]** of the anatomy it
/// lands on. The address is what lets ArcGIS spatial queries and ViT feature
/// measurements meet on the same node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectionSample {
    /// The anatomy node this sample lands on.
    pub at: Guid,
    /// What was measured.
    pub kind: SampleKind,
    /// The measured value (modality-specific units).
    pub value: f32,
}

/// The contract every modality implements to project onto FMA-addressed
/// anatomy. Two steps that mirror the physics: **register** (align the modality
/// frame to the body via the rigid bone fiducials), then **project** (emit
/// Guid-addressed samples).
pub trait ModalityProjection {
    /// Which modality this is.
    fn modality(&self) -> Modality;

    /// Register the acquisition to the skeleton, returning the rigid pose that
    /// aligns the modality frame to the body frame. Bones are the fiducials —
    /// the clamped convergence anchors — which is why they must be stable.
    fn register(&self, skeleton: &Skeleton) -> RigidTransform;

    /// Project the registered acquisition into [`Guid`]-addressed samples. Each
    /// sample writes to the anatomy node at its address (the value side; the
    /// key is never decoded to address it).
    fn project(&self, pose: &RigidTransform) -> Vec<ProjectionSample>;
}

/// Worked example: an X-ray that registers to the skeleton by its bone
/// fiducials and emits an attenuation sample at every bone's [`Guid`].
///
/// A **scaffold** demonstrating the seam — it returns the identity pose (X-ray
/// *shows* bone, so the skeleton already lives in its frame) and a constant
/// attenuation per bone. A real engine replaces both with measured values.
pub struct XrayBoneFiducial {
    /// Per-bone attenuation to emit (illustrative).
    pub attenuation: f32,
}

impl ModalityProjection for XrayBoneFiducial {
    fn modality(&self) -> Modality {
        Modality::Xray
    }

    fn register(&self, _skeleton: &Skeleton) -> RigidTransform {
        // X-ray shows bone: the skeleton's frame IS the radiograph's frame.
        RigidTransform::at([0.0, 0.0, 0.0])
    }

    fn project(&self, _pose: &RigidTransform) -> Vec<ProjectionSample> {
        Skeleton::resolve()
            .clamped_anchors()
            .map(|bone| ProjectionSample {
                at: bone.guid(),
                kind: SampleKind::Attenuation,
                value: self.attenuation,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xray_projects_attenuation_addressed_by_bone_guid() {
        let xray = XrayBoneFiducial { attenuation: 0.8 };
        assert_eq!(xray.modality(), Modality::Xray);

        let skeleton = Skeleton::resolve();
        let pose = xray.register(&skeleton);
        assert!(pose.is_valid());

        let samples = xray.project(&pose);
        assert!(!samples.is_empty());
        // Every sample is addressed by a bone's classid-routed Guid.
        for s in &samples {
            assert_eq!(s.kind, SampleKind::Attenuation);
            assert_eq!(s.at.classid(), 0x0A03, "addressed by the bone concept");
        }
        // One sample per clamped anchor — the fiducial frame.
        assert_eq!(samples.len(), skeleton.clamped_anchors().count());
    }
}
