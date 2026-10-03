//! Pure rules: `G(n+1) = R(G(n), evidence)`.
//!
//! A rule reads one version and returns the changes it proposes. It cannot
//! write: it receives `&GraphState`, returns `Vec<Change>`, and has no I/O
//! handle. The store turns the proposal into a new version.

use crate::graph::{Attribute, Change, GraphState};
use crate::population::Population;
use ogar_dir_core::Guid128;

/// Rule identity recorded in every version a rule produces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId {
    /// Stable name, e.g. `"ExchangeAccess"`.
    pub name: &'static str,
    /// Rule version; a behaviour change is a new version, never an edit.
    pub version: u16,
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/v{}", self.name, self.version)
    }
}

/// Opaque reference to the input that justified running a rule (a request
/// id, ticket, HR record…). Stored in the version's provenance.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceRef(pub String);

/// A pure graph transformation.
pub trait Rule {
    /// Identity recorded in provenance.
    fn id(&self) -> RuleId;
    /// Proposed changes against `g`. Must be deterministic in `(g, evidence)`.
    fn propose(&self, g: &GraphState, evidence: &[EvidenceRef]) -> Vec<Change>;
}

/// Grant `group` to an explicit population (e.g. the users named in a
/// request). Members already in the group are skipped, so the proposal is
/// exactly the net change.
pub struct GrantGroup {
    /// Rule identity (lets one implementation serve several named rules).
    pub rule: RuleId,
    /// The group to grant.
    pub group: Guid128,
    /// Who should receive it.
    pub to: Vec<Guid128>,
}

impl Rule for GrantGroup {
    fn id(&self) -> RuleId {
        self.rule
    }
    fn propose(&self, g: &GraphState, _: &[EvidenceRef]) -> Vec<Change> {
        let wanted = Population::of(g, &self.to);
        let missing = wanted.minus(&Population::members_of(g, self.group));
        missing
            .guids()
            .into_iter()
            .map(|user| Change::AddMembership {
                user,
                group: self.group,
            })
            .collect()
    }
}

/// Population rule: every active member of `source` is also a member of
/// `target`. Evaluated as `active ∩ members(source) − members(target)`.
pub struct ImplyGroup {
    /// Rule identity.
    pub rule: RuleId,
    /// Membership that implies…
    pub source: Guid128,
    /// …membership here.
    pub target: Guid128,
}

impl Rule for ImplyGroup {
    fn id(&self) -> RuleId {
        self.rule
    }
    fn propose(&self, g: &GraphState, _: &[EvidenceRef]) -> Vec<Change> {
        Population::active_users(g)
            .and(&Population::members_of(g, self.source))
            .minus(&Population::members_of(g, self.target))
            .guids()
            .into_iter()
            .map(|user| Change::AddMembership {
                user,
                group: self.target,
            })
            .collect()
    }
}

/// Set one user's primary SMTP address (compare-and-set against the value
/// in the version the rule reads).
pub struct SetPrimarySmtp {
    /// Rule identity.
    pub rule: RuleId,
    /// User.
    pub user: Guid128,
    /// New address.
    pub to: String,
}

impl Rule for SetPrimarySmtp {
    fn id(&self) -> RuleId {
        self.rule
    }
    fn propose(&self, g: &GraphState, _: &[EvidenceRef]) -> Vec<Change> {
        let from = g.node(&self.user).and_then(|n| n.primary_smtp.clone());
        if from.as_deref() == Some(self.to.as_str()) {
            return Vec::new();
        }
        vec![Change::SetAttribute {
            node: self.user,
            attribute: Attribute::PrimarySmtp,
            from,
            to: Some(self.to.clone()),
        }]
    }
}
