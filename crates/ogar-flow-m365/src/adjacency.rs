//! The **command adjacency table**: friendly labels beside typed operations.
//!
//! A label is metadata. The [`Operation`] it points at is authoritative: its
//! effect, the capability it needs, its input/output shape, the backend that
//! can execute it, and the call byte it lowers to. Two labels that point at
//! one operation are aliases and cannot mean different things, because there
//! is only one row for them to point at.
//!
//! ```text
//!   "Get-MgUser" ─┐
//!                 ├─► LabelId ─► OperationId ─► Operation { fn_index, effect, capability, … }
//!   "Get-AzureADUser" ┘                                      │
//!                                                    ogar-loco Call (fn_index : value…)
//! ```
//!
//! String matching happens here, once, at the compile boundary. Execution
//! sees only [`FnIndex`] bytes.

use ogar_dir_core::{SchemaFamily, SchemaId};
use ogar_loco::FnIndex;

use crate::vocab;

/// Index into [`LABELS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LabelId(pub u16);

/// Index into [`OPERATIONS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(pub u8);

/// What running an operation does to the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Reads observed facts; changes nothing.
    Read,
    /// Produces an output artifact from facts already read.
    Export,
    /// Changes directory state. Never executable in this crate.
    Mutation,
    /// Evaluates caller-supplied code. Never executable in this crate.
    Execution,
}

/// A capability bit the report authority must grant before an operation runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Capability {
    /// Read directory identity facts.
    ReadIdentity = 1 << 0,
    /// Read observed license facts.
    ReadLicenses = 1 << 1,
    /// Read observed group memberships.
    ReadMemberships = 1 << 2,
    /// Produce a report artifact.
    Export = 1 << 3,
}

/// A set of [`Capability`] bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CapabilitySet(pub u8);

impl CapabilitySet {
    /// No capabilities.
    pub const NONE: Self = Self(0);

    /// The set holding exactly `caps`.
    #[must_use]
    pub const fn of(caps: &[Capability]) -> Self {
        let mut bits = 0u8;
        let mut i = 0;
        while i < caps.len() {
            bits |= caps[i] as u8;
            i += 1;
        }
        Self(bits)
    }

    /// Whether `cap` is in the set.
    #[must_use]
    pub const fn has(self, cap: Capability) -> bool {
        self.0 & (cap as u8) != 0
    }
}

/// What flows into and out of an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlowShape {
    /// Nothing: the operation is a source.
    Nothing,
    /// A population of directory rows.
    Population,
    /// A finished report: the operation is a sink.
    Report,
}

/// Which engine can execute an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backend {
    /// Lowered to `ogar-loco` calls and run by its `Interpreter`.
    Loco,
    /// No backend: the label is known, the operation is refused.
    None,
}

/// The semantic target an operation is bound to.
///
/// No variant carries a classid, because no directory-user, license,
/// group-membership or report concept is minted in `ogar-vocab` today, and
/// this crate does not mint concepts. Binding these to classids is the gate
/// for the production RBAC integration (README § Deferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConceptBinding {
    /// A pipeline operator over any population (filter, projection). It has
    /// no concept of its own.
    Generic,
    /// Bound to an existing typed directory schema in `ogar-dir-core`.
    DirSchema(SchemaId),
    /// The operation has no canonical OGAR concept yet. The string names the
    /// missing piece so the gap is visible at the call site.
    Unminted(&'static str),
}

/// One typed operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation {
    /// Canonical operation name (snake_case). Never a PowerShell label.
    pub name: &'static str,
    /// The call byte this operation lowers to; `None` = not executable.
    pub fn_index: Option<FnIndex>,
    /// What running it does.
    pub effect: Effect,
    /// The capability it needs; `None` = gated by field permissions only.
    pub capability: Option<Capability>,
    /// What it consumes.
    pub input: FlowShape,
    /// What it produces.
    pub output: FlowShape,
    /// Who can execute it.
    pub backend: Backend,
    /// Its semantic target.
    pub binding: ConceptBinding,
}

impl Operation {
    /// Whether this operation can run at all: it has a call byte, a backend,
    /// and its effect is read or export.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.fn_index.is_some()
            && matches!(self.backend, Backend::Loco)
            && matches!(self.effect, Effect::Read | Effect::Export)
    }
}

/// The Graph `user` schema `ogar-az` encodes against.
const MS_GRAPH_USER: SchemaId = SchemaId {
    family: SchemaFamily::MsGraph,
    version: 2,
};

/// Every operation, indexed by [`OperationId`].
pub const OPERATIONS: &[Operation] = &[
    // 0
    Operation {
        name: "query_user",
        fn_index: Some(vocab::GET_USER),
        effect: Effect::Read,
        capability: Some(Capability::ReadIdentity),
        input: FlowShape::Nothing,
        output: FlowShape::Population,
        backend: Backend::Loco,
        binding: ConceptBinding::DirSchema(MS_GRAPH_USER),
    },
    // 1
    Operation {
        name: "read_license_evidence",
        fn_index: Some(vocab::GET_LICENSE_EVIDENCE),
        effect: Effect::Read,
        capability: Some(Capability::ReadLicenses),
        input: FlowShape::Population,
        output: FlowShape::Population,
        backend: Backend::Loco,
        binding: ConceptBinding::Unminted("no license-assignment concept in ogar-vocab"),
    },
    // 2
    Operation {
        name: "read_membership",
        fn_index: Some(vocab::GET_MEMBERSHIP),
        effect: Effect::Read,
        capability: Some(Capability::ReadMemberships),
        input: FlowShape::Population,
        output: FlowShape::Population,
        backend: Backend::Loco,
        binding: ConceptBinding::Unminted("no directory group-membership concept in ogar-vocab"),
    },
    // 3
    Operation {
        name: "filter_rows",
        fn_index: Some(vocab::WHERE),
        effect: Effect::Read,
        capability: None,
        input: FlowShape::Population,
        output: FlowShape::Population,
        backend: Backend::Loco,
        binding: ConceptBinding::Generic,
    },
    // 4
    Operation {
        name: "select_fields",
        fn_index: Some(vocab::SELECT),
        effect: Effect::Read,
        capability: None,
        input: FlowShape::Population,
        output: FlowShape::Population,
        backend: Backend::Loco,
        binding: ConceptBinding::Generic,
    },
    // 5
    Operation {
        name: "export_report",
        fn_index: Some(vocab::EXPORT),
        effect: Effect::Export,
        capability: Some(Capability::Export),
        input: FlowShape::Population,
        output: FlowShape::Report,
        backend: Backend::Loco,
        binding: ConceptBinding::Unminted(
            "report execution is lance-graph-report's; no OGAR concept",
        ),
    },
    // 6 — known, refused.
    Operation {
        name: "update_user",
        fn_index: None,
        effect: Effect::Mutation,
        capability: None,
        input: FlowShape::Population,
        output: FlowShape::Nothing,
        backend: Backend::None,
        binding: ConceptBinding::DirSchema(MS_GRAPH_USER),
    },
    // 7
    Operation {
        name: "remove_user",
        fn_index: None,
        effect: Effect::Mutation,
        capability: None,
        input: FlowShape::Population,
        output: FlowShape::Nothing,
        backend: Backend::None,
        binding: ConceptBinding::DirSchema(MS_GRAPH_USER),
    },
    // 8
    Operation {
        name: "add_group_member",
        fn_index: None,
        effect: Effect::Mutation,
        capability: None,
        input: FlowShape::Population,
        output: FlowShape::Nothing,
        backend: Backend::None,
        binding: ConceptBinding::Unminted("no directory group-membership concept in ogar-vocab"),
    },
    // 9
    Operation {
        name: "assign_license",
        fn_index: None,
        effect: Effect::Mutation,
        capability: None,
        input: FlowShape::Population,
        output: FlowShape::Nothing,
        backend: Backend::None,
        binding: ConceptBinding::Unminted("no license-assignment concept in ogar-vocab"),
    },
    // 10
    Operation {
        name: "evaluate_expression",
        fn_index: None,
        effect: Effect::Execution,
        capability: None,
        input: FlowShape::Nothing,
        output: FlowShape::Nothing,
        backend: Backend::None,
        binding: ConceptBinding::Generic,
    },
];

/// One friendly label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Label {
    /// The label as an administrator types it. Matched case-insensitively.
    pub text: &'static str,
    /// The operation it names.
    pub operation: OperationId,
}

/// Every known label. Several labels may name one operation (aliases).
pub const LABELS: &[Label] = &[
    Label {
        text: "Get-MgUser",
        operation: OperationId(0),
    },
    Label {
        text: "Get-AzureADUser",
        operation: OperationId(0),
    },
    Label {
        text: "Get-IamLicenseEvidence",
        operation: OperationId(1),
    },
    Label {
        text: "Get-IamMembership",
        operation: OperationId(2),
    },
    Label {
        text: "Where-Object",
        operation: OperationId(3),
    },
    Label {
        text: "where",
        operation: OperationId(3),
    },
    Label {
        text: "?",
        operation: OperationId(3),
    },
    Label {
        text: "Select-Object",
        operation: OperationId(4),
    },
    Label {
        text: "select",
        operation: OperationId(4),
    },
    Label {
        text: "Export-Report",
        operation: OperationId(5),
    },
    Label {
        text: "Set-MgUser",
        operation: OperationId(6),
    },
    Label {
        text: "Update-MgUser",
        operation: OperationId(6),
    },
    Label {
        text: "Remove-MgUser",
        operation: OperationId(7),
    },
    Label {
        text: "New-MgGroupMember",
        operation: OperationId(8),
    },
    Label {
        text: "Set-MgUserLicense",
        operation: OperationId(9),
    },
    Label {
        text: "Invoke-Expression",
        operation: OperationId(10),
    },
    Label {
        text: "iex",
        operation: OperationId(10),
    },
];

/// Why a label did not resolve to an executable operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// No label with this text exists.
    Unknown(String),
    /// The label is known, but its operation cannot run here.
    Unavailable {
        /// The label as typed.
        label: String,
        /// The operation it names.
        operation: &'static str,
        /// Why it cannot run.
        effect: Effect,
    },
}

impl core::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unknown(l) => write!(f, "unknown command `{l}`"),
            Self::Unavailable {
                label,
                operation,
                effect,
            } => write!(
                f,
                "`{label}` names `{operation}` ({effect:?}), which this flow host does not execute"
            ),
        }
    }
}

impl core::error::Error for ResolveError {}

/// The operation behind `id`.
#[must_use]
pub fn operation(id: OperationId) -> &'static Operation {
    &OPERATIONS[usize::from(id.0)]
}

/// Find a label by text, case-insensitively. Known-but-unavailable labels
/// resolve here too; [`resolve`] is what refuses them.
#[must_use]
pub fn lookup(text: &str) -> Option<(LabelId, &'static Label)> {
    LABELS
        .iter()
        .enumerate()
        .find(|(_, l)| l.text.eq_ignore_ascii_case(text))
        .map(|(i, l)| (LabelId(i as u16), l))
}

/// Resolve a label to an executable operation.
///
/// # Errors
///
/// [`ResolveError::Unknown`] for an unknown label, and
/// [`ResolveError::Unavailable`] for a known label whose operation mutates,
/// executes code, or has no backend.
pub fn resolve(text: &str) -> Result<(OperationId, &'static Operation), ResolveError> {
    let (_, label) = lookup(text).ok_or_else(|| ResolveError::Unknown(text.to_owned()))?;
    let op = operation(label.operation);
    if op.is_available() {
        Ok((label.operation, op))
    } else {
        Err(ResolveError::Unavailable {
            label: text.to_owned(),
            operation: op.name,
            effect: op.effect,
        })
    }
}

/// The operation whose call byte is `f`, if any.
#[must_use]
pub fn operation_for_fn(f: FnIndex) -> Option<&'static Operation> {
    OPERATIONS.iter().find(|op| op.fn_index == Some(f))
}
