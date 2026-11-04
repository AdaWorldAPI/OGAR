//! The host seams: where observed facts come from, and who decides what an
//! actor may see.
//!
//! Both are traits so a fixture today and the production adapters tomorrow
//! (`ogar-az` observations, the OGAR RBAC binding) drop in without the
//! dialect changing.

use lance_graph_contract::class_view::FieldMask;
use lance_graph_contract::sla::TenantId;
use ogar_auth::user::AuthenticatedUser;

use crate::adjacency::CapabilitySet;
use crate::vocab::ReportField;

/// One field value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldValue {
    /// A scalar string.
    Text(String),
    /// A boolean.
    Bool(bool),
    /// A list of strings (licenses, memberships).
    List(Vec<String>),
}

/// One observed directory user, carrying only the fields that were asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserObservation {
    /// The row key used to attach license and membership facts. It is never
    /// exported unless [`ReportField::ObjectId`] is permitted and selected.
    pub object_id: String,
    /// The requested fields that were observed.
    pub fields: Vec<(ReportField, FieldValue)>,
}

/// Where the facts in a report came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceStamp {
    /// A stable identifier for the observation source.
    pub source_id: String,
    /// The version of the observation snapshot the facts were read from.
    pub observed_version: u64,
}

/// Directory facts, as observed. Read-only.
///
/// Every query is scoped to the tenant the authority granted, never to a
/// tenant named in the pipeline.
pub trait DirectoryObservations {
    /// The snapshot every answer below is read from.
    fn source(&self) -> SourceStamp;

    /// The user named by `user_ref` (UPN or object id) in `tenant`, carrying
    /// only the fields in `fields`. `None` when no such user is observed in
    /// that tenant.
    fn find_user(
        &self,
        tenant: TenantId,
        user_ref: &str,
        fields: FieldMask,
    ) -> Option<UserObservation>;

    /// Observed license SKUs of `object_id` in `tenant`.
    fn licenses(&self, tenant: TenantId, object_id: &str) -> Vec<String>;

    /// Observed group memberships of `object_id` in `tenant`.
    fn memberships(&self, tenant: TenantId, object_id: &str) -> Vec<String>;
}

/// What an actor may do and see in one flow run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportScope {
    /// The only tenant the run may read.
    pub tenant: TenantId,
    /// The operations the run may perform.
    pub capabilities: CapabilitySet,
    /// The fields the run may read, filter on, and export.
    pub fields: FieldMask,
    /// The version of the policy this scope was derived from.
    pub policy_version: u32,
}

/// Decides an actor's [`ReportScope`].
///
/// The input is the canonical, already-verified OGAR user. Nothing in a
/// pipeline (a `-UserId`, an email address, a command label) reaches this
/// call, so nothing in a pipeline can widen the scope.
pub trait ReportAuthority {
    /// The scope `actor` may run report flows under; `None` = no access.
    fn scope_for(&self, actor: &AuthenticatedUser) -> Option<ReportScope>;
}
