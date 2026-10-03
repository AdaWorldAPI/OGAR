//! The version history. It is the audit trail: there is no separate log.
//!
//! * An **observed** version is a root holding a full snapshot of what was
//!   read from reality.
//! * A **simulated** version holds only its parent and the changes a rule
//!   proposed (structural sharing by delta). Its state is the parent's state
//!   with the delta applied.
//! * Versions are append-only and never mutated. Simulating, validating or
//!   rejecting a version cannot change any other version.
//!
//! The lifecycle stages are not a workflow enum. They fall out of data the
//! store already keeps:
//!
//! | stage                    | expressed as                                      |
//! |--------------------------|---------------------------------------------------|
//! | OBSERVED                 | `Origin::Observed` (+ tag `"observed"`)            |
//! | SIMULATED                | `Origin::Simulated`                                |
//! | APPROVED / DESIRED       | tag `"desired"`, settable only on a valid version  |
//! | OBSERVED AFTER EXECUTION | a newer `Origin::Observed`; converged when its diff to `"desired"` is empty |
//!
//! Tags mirror lance-graph `VersionedGraph::tag_version`; `VersionId` is
//! meant to map 1:1 onto a Lance dataset version once persisted.

use crate::graph::{ApplyError, Change, GraphState};
use crate::rule::{EvidenceRef, Rule, RuleId};
use crate::validate::{Violation, validate};
use ogar_dir_core::Guid128;
use std::collections::BTreeMap;

/// Version identifier (monotonic logical clock of this store).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionId(pub u64);

/// Tag set on every new observation.
pub const TAG_OBSERVED: &str = "observed";
/// Tag naming the current desired state.
pub const TAG_DESIRED: &str = "desired";

/// Where a version came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Read from reality.
    Observed {
        /// Which observer (e.g. `"ogar-ad:ldif"`).
        source: String,
        /// When it was read (unix ms).
        observed_at_ms: i64,
    },
    /// Produced by a pure rule.
    Simulated {
        /// The rule.
        rule: RuleId,
        /// The input it ran on.
        evidence: Vec<EvidenceRef>,
    },
}

/// One version's provenance record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    /// This version.
    pub id: VersionId,
    /// The version it was derived from (`None` for an observation).
    pub parent: Option<VersionId>,
    /// What produced it.
    pub origin: Origin,
    /// The changes it introduced relative to `parent` (empty for an observation).
    pub delta: Vec<Change>,
}

/// Simulation failure. No version is created.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimError {
    /// No such version.
    UnknownVersion(VersionId),
    /// The rule proposed nothing (no new version — nothing to explore).
    EmptyProposal(RuleId),
    /// The proposal does not apply to the parent.
    Apply(ApplyError),
}

/// Refusal to make a version desired.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejection {
    /// The rejected version (it stays in the store as evidence).
    pub version: VersionId,
    /// Why.
    pub violations: Vec<Violation>,
}

/// Append-only version store.
#[derive(Debug, Default)]
pub struct VersionStore {
    versions: Vec<Version>,
    snapshots: BTreeMap<VersionId, GraphState>,
    tags: BTreeMap<String, VersionId>,
    verdicts: BTreeMap<VersionId, Vec<Violation>>,
}

impl VersionStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }

    fn next_id(&self) -> VersionId {
        VersionId(self.versions.len() as u64)
    }

    /// Record an observation as a new root version; tags it `"observed"`.
    pub fn observe(&mut self, source: &str, observed_at_ms: i64, state: GraphState) -> VersionId {
        let id = self.next_id();
        self.versions.push(Version {
            id,
            parent: None,
            origin: Origin::Observed {
                source: source.into(),
                observed_at_ms,
            },
            delta: Vec::new(),
        });
        self.snapshots.insert(id, state);
        self.tags.insert(TAG_OBSERVED.into(), id);
        id
    }

    /// Provenance record of a version.
    pub fn version(&self, v: VersionId) -> Option<&Version> {
        self.versions.get(v.0 as usize)
    }

    /// Path from the observed root to `v`, root first.
    pub fn lineage(&self, v: VersionId) -> Result<Vec<VersionId>, SimError> {
        let mut path = Vec::new();
        let mut cur = Some(v);
        while let Some(c) = cur {
            path.push(c);
            cur = self.version(c).ok_or(SimError::UnknownVersion(c))?.parent;
        }
        path.reverse();
        Ok(path)
    }

    /// Materialize the full state of `v` (root snapshot + deltas).
    pub fn state(&self, v: VersionId) -> Result<GraphState, SimError> {
        let path = self.lineage(v)?;
        let mut s = self
            .snapshots
            .get(&path[0])
            .cloned()
            .ok_or(SimError::UnknownVersion(path[0]))?;
        for id in &path[1..] {
            s = s
                .apply_all(&self.versions[id.0 as usize].delta)
                .map_err(SimError::Apply)?;
        }
        Ok(s)
    }

    /// Run a pure rule against `parent`, recording the result as a new
    /// hypothetical version. Never touches anything outside the store.
    pub fn simulate(
        &mut self,
        parent: VersionId,
        rule: &dyn Rule,
        evidence: &[EvidenceRef],
    ) -> Result<VersionId, SimError> {
        let base = self.state(parent)?;
        let delta = rule.propose(&base, evidence);
        if delta.is_empty() {
            return Err(SimError::EmptyProposal(rule.id()));
        }
        base.apply_all(&delta).map_err(SimError::Apply)?;
        let id = self.next_id();
        self.versions.push(Version {
            id,
            parent: Some(parent),
            origin: Origin::Simulated {
                rule: rule.id(),
                evidence: evidence.to_vec(),
            },
            delta,
        });
        Ok(id)
    }

    /// Validate a version and record the verdict. Empty = valid.
    pub fn validate(&mut self, v: VersionId) -> Result<Vec<Violation>, SimError> {
        let violations = validate(&self.state(v)?);
        self.verdicts.insert(v, violations.clone());
        Ok(violations)
    }

    /// Recorded verdict, if `validate` ran.
    pub fn verdict(&self, v: VersionId) -> Option<&[Violation]> {
        self.verdicts.get(&v).map(Vec::as_slice)
    }

    /// Make `v` the desired state. Validates first; on any violation the
    /// `"desired"` tag is left exactly as it was and the version is kept.
    pub fn promote_desired(&mut self, v: VersionId) -> Result<(), Rejection> {
        let violations = self.validate(v).map_err(|_| Rejection {
            version: v,
            violations: Vec::new(),
        })?;
        if !violations.is_empty() {
            return Err(Rejection {
                version: v,
                violations,
            });
        }
        self.tags.insert(TAG_DESIRED.into(), v);
        Ok(())
    }

    /// Version a tag points to.
    pub fn tag(&self, name: &str) -> Option<VersionId> {
        self.tags.get(name).copied()
    }

    /// Semantic difference `a → b`.
    pub fn diff(&self, a: VersionId, b: VersionId) -> Result<Vec<Change>, SimError> {
        Ok(self.state(a)?.diff(&self.state(b)?))
    }

    /// Why does `v` contain the membership `(user, group)`? Returns the
    /// lineage from the observed root to the version whose delta introduced
    /// it, or `None` if `v` does not contain it. A chain of length 1 means
    /// it was observed, not simulated.
    pub fn explain_membership(
        &self,
        v: VersionId,
        user: Guid128,
        group: Guid128,
    ) -> Option<Vec<&Version>> {
        if !self.state(v).ok()?.is_member(user, group) {
            return None;
        }
        let path = self.lineage(v).ok()?;
        let add = Change::AddMembership { user, group };
        // The LAST delta on the path that added it (a later re-add after a
        // removal is the one that explains the current state).
        let at = path
            .iter()
            .rposition(|id| self.versions[id.0 as usize].delta.contains(&add))
            .unwrap_or(0);
        Some(
            path[..=at]
                .iter()
                .map(|id| &self.versions[id.0 as usize])
                .collect(),
        )
    }
}
