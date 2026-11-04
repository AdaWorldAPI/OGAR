//! The flow vocabulary: six domain call bytes, and the codebooks their value
//! bytes resolve against.
//!
//! Every flow body uses [`LaneShape::Quads`] (`function : value ×3`, 90 calls
//! per node), because `Where-Object` needs three immediates
//! (`field : comparison : argument`) and `Select-Object` packs a 24-bit field
//! mask into its three value bytes.
//!
//! | byte | name | pops | pushes | value bytes |
//! |---|---|---|---|---|
//! | `0x90` | `get_user` | 0 | population | `arg` (index into the flow's argument table) |
//! | `0x91` | `get_license_evidence` | 1 | population | — |
//! | `0x92` | `get_membership` | 1 | population | — |
//! | `0x93` | `where` | 1 | population | `field : comparison : arg` |
//! | `0x94` | `select` | 1 | population | 24-bit field mask, little-endian |
//! | `0x95` | `export` | 1 | — | `format` |
//!
//! The comparison byte is a shared-core function index ([`FnIndex::EQ`],
//! [`FnIndex::NEQ`], [`FnIndex::LIST_CONTAINS`]) used as an operator name, so
//! filtering reuses the codebook `ogar-loco` already defines instead of a
//! second operator table.

use lance_graph_contract::class_view::FieldMask;
use ogar_loco::{FnIndex, LaneShape, RegistryError, ValueCodebook, Vocabulary, VocabularyRegistry};

/// Every flow body's lane carving.
pub const FLOW_SHAPE: LaneShape = LaneShape::Quads;

/// `0x1718` — the concept id of a flow body node: the classid a
/// [`VocabularyRegistry`] routes to [`FlowVocabulary`].
///
/// A palette slot in `ogar-loco`'s `0x17XX` domain, seated above
/// `blockly-rs` (`0x1717`) as `ogar-loco` reserves `0x1717`+ for consumer
/// palettes. Like `0x1717`, it lives in this crate, not in `ogar-vocab`'s
/// globally mirrored codebook (a palette is a frontend's private reading).
pub const FLOW_BODY_CONCEPT_ID: u16 = 0x1718;

/// The full render classid of a flow body under `app_prefix`.
#[must_use]
pub const fn flow_body_render_classid(app_prefix: u16) -> u32 {
    ((FLOW_BODY_CONCEPT_ID as u32) << 16) | (app_prefix as u32)
}

/// Register [`FlowVocabulary`] under [`FLOW_BODY_CONCEPT_ID`].
///
/// # Errors
///
/// [`RegistryError::ConceptTaken`] when the registry already has a vocabulary
/// for `0x1718`.
pub fn plug_into(registry: &mut VocabularyRegistry) -> Result<(), RegistryError> {
    registry.plug(FLOW_BODY_CONCEPT_ID, &checked())
}

/// `get_user`: look up one directory user.
pub const GET_USER: FnIndex = FnIndex(0x90);
/// `get_license_evidence`: attach observed license facts.
pub const GET_LICENSE_EVIDENCE: FnIndex = FnIndex(0x91);
/// `get_membership`: attach observed group memberships.
pub const GET_MEMBERSHIP: FnIndex = FnIndex(0x92);
/// `where`: keep the rows matching one comparison.
pub const WHERE: FnIndex = FnIndex(0x93);
/// `select`: project the rows onto a field mask.
pub const SELECT: FnIndex = FnIndex(0x94);
/// `export`: render the population as a report.
pub const EXPORT: FnIndex = FnIndex(0x95);

const DOMAIN: [(FnIndex, &str, u8, bool, Option<ValueCodebook>); 6] = [
    (GET_USER, "get_user", 0, true, Some(ARG_CODEBOOK)),
    (GET_LICENSE_EVIDENCE, "get_license_evidence", 1, true, None),
    (GET_MEMBERSHIP, "get_membership", 1, true, None),
    (WHERE, "where", 1, true, Some(PREDICATE_CODEBOOK)),
    (SELECT, "select", 1, true, Some(FIELD_MASK_CODEBOOK)),
    (EXPORT, "export", 1, false, Some(FORMAT_CODEBOOK)),
];

/// `get_user`'s value byte indexes the compiled flow's argument table.
pub const ARG_CODEBOOK: ValueCodebook = ValueCodebook {
    id: 0,
    name: "flow_args",
};
/// `where`'s value bytes are `field : comparison : arg`.
pub const PREDICATE_CODEBOOK: ValueCodebook = ValueCodebook {
    id: 1,
    name: "field_comparison_arg",
};
/// `select`'s value bytes are a 24-bit [`ReportField`] mask.
pub const FIELD_MASK_CODEBOOK: ValueCodebook = ValueCodebook {
    id: 2,
    name: "report_field_mask",
};
/// `export`'s value byte is an [`ExportFormat`].
pub const FORMAT_CODEBOOK: ValueCodebook = ValueCodebook {
    id: 3,
    name: "export_format",
};

/// The flow vocabulary. Validate it with
/// [`ogar_loco::vocabulary::conformance::validate`] (or call [`checked`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlowVocabulary;

fn domain(f: FnIndex) -> Option<&'static (FnIndex, &'static str, u8, bool, Option<ValueCodebook>)> {
    DOMAIN.iter().find(|d| d.0 == f)
}

impl Vocabulary for FlowVocabulary {
    fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
        domain(f).map(|d| d.2)
    }
    fn domain_body_refs(&self, _f: FnIndex) -> u8 {
        0
    }
    fn domain_pushes_result(&self, f: FnIndex) -> Option<bool> {
        domain(f).map(|d| d.3)
    }
    fn domain_name(&self, f: FnIndex) -> Option<&'static str> {
        domain(f).map(|d| d.1)
    }
    fn domain_value_codebook(&self, f: FnIndex) -> Option<ValueCodebook> {
        domain(f).and_then(|d| d.4)
    }
}

/// The validated vocabulary the interpreter runs under.
///
/// # Panics
///
/// Never in practice: conformance is pinned by this crate's tests.
#[must_use]
pub fn checked() -> ogar_loco::CheckedVocabulary<FlowVocabulary> {
    ogar_loco::vocabulary::conformance::validate(FlowVocabulary)
        .expect("FlowVocabulary conforms; pinned by this crate's tests")
}

/// A field a report row can carry. The discriminant is its position in a
/// [`FieldMask`]; positions stay below 24 so a mask fits `select`'s three
/// value bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ReportField {
    /// Entra object id.
    ObjectId = 0,
    /// `userPrincipalName`.
    UserPrincipalName = 1,
    /// `displayName`.
    DisplayName = 2,
    /// `department`.
    Department = 3,
    /// `mail`.
    Mail = 4,
    /// `accountEnabled`.
    AccountEnabled = 5,
    /// Observed license SKUs (attached by `get_license_evidence`).
    Licenses = 6,
    /// Observed group memberships (attached by `get_membership`).
    Memberships = 7,
    /// Whether a primary mailbox was observed.
    MailboxAvailable = 8,
}

impl ReportField {
    /// Every field, in mask-position order.
    pub const ALL: [ReportField; 9] = [
        Self::ObjectId,
        Self::UserPrincipalName,
        Self::DisplayName,
        Self::Department,
        Self::Mail,
        Self::AccountEnabled,
        Self::Licenses,
        Self::Memberships,
        Self::MailboxAvailable,
    ];

    /// The fields `get_user` can read (everything but the attached facts).
    pub const USER_FIELDS: FieldMask = FieldMask::from_positions(&[0, 1, 2, 3, 4, 5, 8]);

    /// The PowerShell-style property name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ObjectId => "ObjectId",
            Self::UserPrincipalName => "UserPrincipalName",
            Self::DisplayName => "DisplayName",
            Self::Department => "Department",
            Self::Mail => "Mail",
            Self::AccountEnabled => "AccountEnabled",
            Self::Licenses => "Licenses",
            Self::Memberships => "Memberships",
            Self::MailboxAvailable => "MailboxAvailable",
        }
    }

    /// Whether the field holds a list (and so admits `-contains`).
    #[must_use]
    pub const fn is_list(self) -> bool {
        matches!(self, Self::Licenses | Self::Memberships)
    }

    /// The field at mask position `p`.
    #[must_use]
    pub const fn from_position(p: u8) -> Option<Self> {
        if (p as usize) < Self::ALL.len() {
            Some(Self::ALL[p as usize])
        } else {
            None
        }
    }

    /// Resolve a property name, case-insensitively.
    #[must_use]
    pub fn from_name(s: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|f| f.name().eq_ignore_ascii_case(s))
    }

    /// This field's mask position.
    #[must_use]
    pub const fn position(self) -> u8 {
        self as u8
    }
}

/// Pack a field mask into `select`'s three value bytes.
///
/// Returns `None` when the mask uses a position at or above 24.
#[must_use]
pub const fn pack_mask(mask: FieldMask) -> Option<[u8; 3]> {
    if mask.0 >> 24 != 0 {
        return None;
    }
    Some([mask.0 as u8, (mask.0 >> 8) as u8, (mask.0 >> 16) as u8])
}

/// Unpack `select`'s three value bytes into a field mask.
#[must_use]
pub const fn unpack_mask(v: [u8; 3]) -> FieldMask {
    FieldMask(v[0] as u64 | (v[1] as u64) << 8 | (v[2] as u64) << 16)
}

/// A report output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ExportFormat {
    /// Deterministic JSON (`ogar-flow-m365/report.v1`).
    Json = 1,
}

impl ExportFormat {
    /// Decode a format byte.
    #[must_use]
    pub const fn from_byte(b: u8) -> Option<Self> {
        match b {
            1 => Some(Self::Json),
            _ => None,
        }
    }
}

/// A `where` comparison, named by its shared-core function index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Comparison {
    /// `-eq` → [`FnIndex::EQ`].
    Eq,
    /// `-ne` → [`FnIndex::NEQ`].
    Ne,
    /// `-contains` on a list field → [`FnIndex::LIST_CONTAINS`].
    Contains,
}

impl Comparison {
    /// The shared-core byte that names this comparison.
    #[must_use]
    pub const fn byte(self) -> u8 {
        match self {
            Self::Eq => FnIndex::EQ.0,
            Self::Ne => FnIndex::NEQ.0,
            Self::Contains => FnIndex::LIST_CONTAINS.0,
        }
    }

    /// Decode a comparison byte.
    #[must_use]
    pub const fn from_byte(b: u8) -> Option<Self> {
        if b == FnIndex::EQ.0 {
            Some(Self::Eq)
        } else if b == FnIndex::NEQ.0 {
            Some(Self::Ne)
        } else if b == FnIndex::LIST_CONTAINS.0 {
            Some(Self::Contains)
        } else {
            None
        }
    }
}
