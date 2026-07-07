//! Cognition probe: a type-level witness that an OGAR `ActionDef` lowers onto
//! `lance-graph-contract`'s `UnifiedStep` orchestration surface —
//! "reasoning vs controller methods as ONE invocation surface"
//! (E-OGAR-CONVERGENCE-SHAPE cognition layer).
//!
//! This is a shape-compatibility witness, not a wiring: the mapping below is
//! total on the fields `UnifiedStep` needs from an action, and it does NOT
//! invent a `StepDomain::Erp` variant (that arm does not exist yet on the
//! pinned `lance-graph-contract` branch=main — the named `[H]` seam).
//!
//! **Confirmed against the pinned source** (`lance-graph`
//! `598f8727a7869d6468c631d2977731b573b98e7b`,
//! `crates/lance-graph-contract/src/orchestration.rs`):
//! - `UnifiedStep` is a plain `pub struct` with all-`pub` fields (`step_id:
//!   String`, `step_type: String`, `status: StepStatus`, `thinking:
//!   Option<ThinkingContext>`, `reasoning: Option<String>`, `confidence:
//!   Option<f64>`, `depends_on: Vec<StepId>`) — it is NOT `#[non_exhaustive]`
//!   and has no constructor, so it is built here via a plain struct literal.
//! - `StepStatus` is a plain `pub enum` (`Pending`/`Running`/`Completed`/
//!   `Failed`/`Skipped`), also not `#[non_exhaustive]`.
//! - `StepDomain`'s variant set matches RECON R5 exactly:
//!   `Crew | Ladybug | N8n | LanceGraph | Ndarray | Smb | Medcare | Kanban` —
//!   no drift, no ERP/controller arm.
//!
//! Because both types are fully public and constructible, the *full*
//! `action_def_maps_onto_unified_step_shape` mapping is exercised directly;
//! the leitplanke's documented-gap downgrade is not needed here.

use lance_graph_contract::orchestration::{StepDomain, StepStatus, UnifiedStep};
use ogar_vocab::ActionDef;

/// Lower an `ActionDef` onto the `UnifiedStep` shape. Total on the fields
/// `UnifiedStep` needs from an action:
///   - `step_id`    <- `a.identity` (the numeric `StepId` is FNV-1a-derived
///                     downstream via `UnifiedStep::id`, not stored here).
///   - `step_type`  <- `a.predicate`.
///   - `status`     <- `StepStatus::Pending` (the real initial variant; a
///                     freshly lifted action has not started executing).
///   - `depends_on` <- `[]` (a.reads could seed this in a follow-up; out of
///                     scope for this probe).
fn action_to_step(a: &ActionDef) -> UnifiedStep {
    UnifiedStep {
        step_id: a.identity.clone(),
        step_type: a.predicate.clone(),
        status: StepStatus::Pending,
        thinking: None,
        reasoning: None,
        confidence: None,
        depends_on: vec![],
    }
}

#[test]
fn action_def_maps_onto_unified_step_shape() {
    let a = ActionDef::new(
        "ogit-erp/invoice::action_def::post",
        "post",
        "ogit-erp/invoice",
    );
    let step = action_to_step(&a);
    assert_eq!(step.step_type, "post");
    // step_id carries the action identity (numeric StepId is derived, not stored).
    assert!(step.step_id.contains("invoice::action_def::post"));
    assert_eq!(step.status, StepStatus::Pending);
    assert!(step.depends_on.is_empty());
}

#[test]
fn step_domain_has_no_erp_variant_yet() {
    // Pins the named [H] seam: the reasoning/orchestration union already carries
    // Crew/Ladybug/N8n/LanceGraph/Ndarray/Smb/Medcare/Kanban — but NOT an ERP /
    // controller-method arm. When the ActionDef<->UnifiedStep contract PR lands,
    // this test flips (a new variant is added) and documents the change.
    // Exhaustive match forces a compile error if a variant is added silently:
    fn assert_known(d: StepDomain) {
        match d {
            StepDomain::Crew
            | StepDomain::Ladybug
            | StepDomain::N8n
            | StepDomain::LanceGraph
            | StepDomain::Ndarray
            | StepDomain::Smb
            | StepDomain::Medcare
            | StepDomain::Kanban => {} // no ERP arm today — when one is added, this match fails to
                                       // compile, forcing an intentional update (the seam-closing PR).
        }
    }
    let _ = assert_known;
}
