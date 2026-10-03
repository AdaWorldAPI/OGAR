//! Where a version came from. The version history is the audit trail.
//!
//! The lifecycle stages are not a workflow enum; they fall out of origin and
//! tags (mirroring lance-graph `VersionedGraph::tag_version`):
//!
//! | stage                    | expressed as                                         |
//! |--------------------------|------------------------------------------------------|
//! | OBSERVED                 | [`Origin::Observed`], tag [`TAG_OBSERVED`]            |
//! | SIMULATED                | [`Origin::Simulated`]                                 |
//! | APPROVED / DESIRED       | tag [`TAG_DESIRED`], set only on a valid version      |
//! | OBSERVED AFTER EXECUTION | a newer `Origin::Observed`; converged when its diff to `"desired"` is empty |

use crate::change::Change;

/// Version identifier — the store's monotonic logical clock. Designed to
/// map 1:1 onto a Lance dataset version once persisted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionId(pub u64);

/// Tag set on every new observation.
pub const TAG_OBSERVED: &str = "observed";
/// Tag naming the current desired state.
pub const TAG_DESIRED: &str = "desired";

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

/// Opaque reference to the input that justified a rule run (request id,
/// ticket, HR record…).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceRef(pub String);

/// What produced a version.
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
