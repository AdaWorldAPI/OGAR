//! The synthetic "John Doe 101" fixture: a deterministic directory, a test
//! authority, and a test assertion verifier.
//!
//! **Nothing here is security.** [`FixtureAuthority`] stands in for the
//! production OGAR RBAC binding and [`FixtureVerifier`] for a real OIDC/SAML
//! assertion verifier. They exist so the flow can be exercised end to end
//! through the real `ogar-loco` interpreter and the real
//! `ogar_auth::federation::establish` boundary. Compiled only for tests and
//! under the `fixture` feature.

use std::collections::BTreeMap;

use lance_graph_contract::class_view::FieldMask;
use lance_graph_contract::sla::TenantId;
use ogar_auth::federation::{
    AssertionVerifier, LoginError, TrustedIssuers, VerifiedClaims, VerifiedLogin, establish,
};
use ogar_auth::identity::{ExternalIdentity, ExternalSubject, Issuer};
use ogar_auth::user::{
    AuthBinding, AuthStrength, AuthenticatedUser, LocalUserStore, ProviderId, User, UserId,
};

use crate::adjacency::{Capability, CapabilitySet};
use crate::host::{
    DirectoryObservations, FieldValue, ReportAuthority, ReportScope, SourceStamp, UserObservation,
};
use crate::vocab::ReportField;

/// The fixture's tenant.
pub const TEST_TENANT: TenantId = 0x7E57;
/// A second tenant, holding a user the first tenant must never see.
pub const OTHER_TENANT: TenantId = 0x0BAD;
/// The fixture's issuer.
pub const ISSUER: &str = "https://login.example.test/fixture";
/// The fixture's identity provider.
pub const PROVIDER: ProviderId = ProviderId("fixture-idp");

/// One synthetic directory user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureUser {
    /// Tenant the user lives in.
    pub tenant: TenantId,
    /// Synthetic object id.
    pub object_id: &'static str,
    /// UPN.
    pub upn: &'static str,
    /// Display name.
    pub display_name: &'static str,
    /// Department.
    pub department: &'static str,
    /// Mail address.
    pub mail: &'static str,
    /// Whether the account is enabled.
    pub enabled: bool,
    /// Observed license SKUs.
    pub licenses: &'static [&'static str],
    /// Observed group memberships.
    pub memberships: &'static [&'static str],
    /// Whether a primary mailbox is observed.
    pub mailbox: bool,
}

/// The fixture directory.
pub const USERS: &[FixtureUser] = &[
    FixtureUser {
        tenant: TEST_TENANT,
        object_id: "00000000-0000-4000-8000-000000000101",
        upn: "john.doe@example.test",
        display_name: "John Doe",
        department: "Operations",
        mail: "john.doe@example.test",
        enabled: true,
        licenses: &["M365 E3", "Teams Phone"],
        memberships: &["Reporting-Readers", "ServiceDesk"],
        mailbox: true,
    },
    FixtureUser {
        tenant: TEST_TENANT,
        object_id: "00000000-0000-4000-8000-000000000102",
        upn: "jane.roe@example.test",
        display_name: "Jane Roe",
        department: "Finance",
        mail: "jane.roe@example.test",
        enabled: true,
        licenses: &["M365 E5"],
        memberships: &["Finance-Approvers"],
        mailbox: true,
    },
    FixtureUser {
        tenant: OTHER_TENANT,
        object_id: "00000000-0000-4000-8000-000000000201",
        upn: "john.doe@other.test",
        display_name: "John Doe (other tenant)",
        department: "Secret",
        mail: "john.doe@other.test",
        enabled: true,
        licenses: &["M365 E5"],
        memberships: &["Board"],
        mailbox: true,
    },
];

/// The fixture's observation snapshot.
#[derive(Debug, Clone, Copy, Default)]
pub struct FixtureDirectory;

impl FixtureDirectory {
    fn user(tenant: TenantId, user_ref: &str) -> Option<&'static FixtureUser> {
        USERS.iter().find(|u| {
            u.tenant == tenant
                && (u.upn.eq_ignore_ascii_case(user_ref)
                    || u.object_id.eq_ignore_ascii_case(user_ref))
        })
    }

    fn by_key(tenant: TenantId, object_id: &str) -> Option<&'static FixtureUser> {
        USERS
            .iter()
            .find(|u| u.tenant == tenant && u.object_id == object_id)
    }

    /// Every field of `u`, unprojected. Used by hosts that deliberately
    /// misbehave in tests.
    #[must_use]
    pub fn all_fields(u: &FixtureUser) -> Vec<(ReportField, FieldValue)> {
        vec![
            (
                ReportField::UserPrincipalName,
                FieldValue::Text(u.upn.into()),
            ),
            (
                ReportField::DisplayName,
                FieldValue::Text(u.display_name.into()),
            ),
            (
                ReportField::Department,
                FieldValue::Text(u.department.into()),
            ),
            (ReportField::Mail, FieldValue::Text(u.mail.into())),
            (ReportField::AccountEnabled, FieldValue::Bool(u.enabled)),
            (ReportField::MailboxAvailable, FieldValue::Bool(u.mailbox)),
        ]
    }
}

impl DirectoryObservations for FixtureDirectory {
    fn source(&self) -> SourceStamp {
        SourceStamp {
            source_id: "fixture:john-doe-101".into(),
            observed_version: 7,
        }
    }

    fn find_user(
        &self,
        tenant: TenantId,
        user_ref: &str,
        fields: FieldMask,
    ) -> Option<UserObservation> {
        let u = Self::user(tenant, user_ref)?;
        Some(UserObservation {
            object_id: u.object_id.into(),
            fields: Self::all_fields(u)
                .into_iter()
                .filter(|(k, _)| fields.has(k.position()))
                .collect(),
        })
    }

    fn licenses(&self, tenant: TenantId, object_id: &str) -> Vec<String> {
        Self::by_key(tenant, object_id)
            .map(|u| u.licenses.iter().map(|s| (*s).to_owned()).collect())
            .unwrap_or_default()
    }

    fn memberships(&self, tenant: TenantId, object_id: &str) -> Vec<String> {
        Self::by_key(tenant, object_id)
            .map(|u| u.memberships.iter().map(|s| (*s).to_owned()).collect())
            .unwrap_or_default()
    }
}

/// A test authority: scopes keyed by canonical `(UserId, tenant)`.
#[derive(Debug, Clone, Default)]
pub struct FixtureAuthority {
    scopes: BTreeMap<(u64, TenantId), ReportScope>,
}

impl FixtureAuthority {
    /// Grant `scope` to the canonical user `id` in `tenant`.
    #[must_use]
    pub fn grant(mut self, id: UserId, tenant: TenantId, scope: ReportScope) -> Self {
        self.scopes.insert((id.0, tenant), scope);
        self
    }

    /// The standard fixture: the report reader (`READER`) may read identity,
    /// licenses and memberships and export, on every field except `Mail`.
    /// The reader's scope is pinned to [`TEST_TENANT`].
    #[must_use]
    pub fn standard() -> Self {
        Self::default().grant(READER, TEST_TENANT, reader_scope())
    }
}

/// The reader's scope in [`FixtureAuthority::standard`].
#[must_use]
pub fn reader_scope() -> ReportScope {
    ReportScope {
        tenant: TEST_TENANT,
        capabilities: CapabilitySet::of(&[
            Capability::ReadIdentity,
            Capability::ReadLicenses,
            Capability::ReadMemberships,
            Capability::Export,
        ]),
        fields: FieldMask::from_positions(&[0, 1, 2, 3, 5, 6, 7, 8]),
        policy_version: 1,
    }
}

impl ReportAuthority for FixtureAuthority {
    fn scope_for(&self, actor: &AuthenticatedUser) -> Option<ReportScope> {
        // Keyed by the canonical user and tenant, never by the subject string.
        self.scopes
            .get(&(actor.user.id.0, actor.user.tenant))
            .cloned()
    }
}

/// Canonical id of the fixture's report reader.
pub const READER: UserId = UserId(1001);
/// Canonical id of a fixture user with no report scope.
pub const OUTSIDER: UserId = UserId(1002);

/// A test assertion verifier: accepts assertions of the form
/// `"fixture-signed:<subject>"` from [`ISSUER`]. It stands in for real token
/// verification so the flow is driven through `establish`.
#[derive(Debug, Clone, Copy, Default)]
pub struct FixtureVerifier;

/// Why a fixture assertion was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadAssertion;

impl AssertionVerifier for FixtureVerifier {
    type Input = str;
    type Error = BadAssertion;
    const ID: &'static str = "fixture";

    fn verify(
        &self,
        _trusted: &TrustedIssuers,
        input: &str,
    ) -> Result<VerifiedClaims, BadAssertion> {
        let subject = input.strip_prefix("fixture-signed:").ok_or(BadAssertion)?;
        Ok(VerifiedClaims {
            identity: ExternalIdentity::new(
                Issuer::new(ISSUER).map_err(|_| BadAssertion)?,
                ExternalSubject::new(subject).map_err(|_| BadAssertion)?,
            ),
            strength: AuthStrength::MultiFactor,
            email: None,
        })
    }
}

/// The fixture's user store and trusted issuers: `READER` (subject `reader`)
/// and `OUTSIDER` (subject `outsider`), both in [`TEST_TENANT`].
#[must_use]
pub fn identity_store() -> (LocalUserStore, TrustedIssuers) {
    let mut store = LocalUserStore::new();
    for (id, subject) in [(READER, "reader"), (OUTSIDER, "outsider")] {
        store.insert(User {
            id,
            subject: subject.into(),
            tenant: TEST_TENANT,
            roles: Vec::new(),
            memberships: Vec::new(),
            bindings: vec![AuthBinding::new(PROVIDER, subject)],
            key_refs: Vec::new(),
        });
        store
            .bind_external(
                id,
                ExternalIdentity::new(
                    Issuer::new(ISSUER).expect("fixture issuer is valid"),
                    ExternalSubject::new(subject).expect("fixture subject is non-empty"),
                ),
            )
            .expect("fixture binding is unique");
    }
    let mut trusted = TrustedIssuers::new();
    trusted
        .insert(
            Issuer::new(ISSUER).expect("fixture issuer is valid"),
            PROVIDER,
        )
        .expect("one fixture issuer");
    (store, trusted)
}

/// Log in through `ogar_auth::federation::establish` with a fixture
/// assertion.
///
/// # Errors
///
/// Whatever `establish` reports: a malformed assertion, an untrusted issuer,
/// or a subject bound to no user.
pub fn login(assertion: &str) -> Result<VerifiedLogin, LoginError<BadAssertion>> {
    let (store, trusted) = identity_store();
    establish(&FixtureVerifier, &trusted, assertion, &store)
}
