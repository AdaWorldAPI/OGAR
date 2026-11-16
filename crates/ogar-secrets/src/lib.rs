//! `ogar-secrets` — the tenant credential broker contract.
//!
//! The chain, top to bottom:
//!
//! ```text
//!   ZITADEL
//!     -> ogar-auth    VerifiedLogin        (who is this workload)
//!     -> ogar-rbac    authorize_identity   (ACT on tenant_credential?)
//!     -> CredentialGrant                   (proof of that decision)
//!     -> TenantSecretBroker::release       (builds the path itself)
//!     -> CredentialProvider                (Vault adapter, a later crate)
//! ```
//!
//! # What is not here
//!
//! - No network, no Vault client, no async, no serde, no env access.
//! - No identity, role or tenant type of its own. Tenant is `User.tenant`,
//!   identity is `VerifiedLogin`, roles are the `RbacBinding`.
//!
//! # Not a `KeyRef`
//!
//! `ogar_auth::user::KeyRef` points at a user's cryptographic material. This
//! crate releases a tenant's source credential to a workload. It shares the
//! non-disclosure rule of `KeyRef`, not its type.
//!
//! # Who can get a grant
//!
//! A human session's decision never becomes a grant. A grant needs ACT on the
//! `tenant_credential` concept, and a policy gives that only to workload
//! principals. Keep one workload principal per tenant: `IdentityActors` scopes
//! every membership to the user's own tenant, so that tenant is the one the
//! credential belongs to.
//!
//! # The request names nothing
//!
//! A [`CredentialRequest`] has a kind and a version. It has no tenant, no
//! consumer and no path. The broker takes those from the grant.
//!
//! ```compile_fail
//! use ogar_secrets::{CredentialKind, CredentialRequest};
//! let _ = CredentialRequest { kind: CredentialKind::GraphApp, version: None, tenant: 1002 };
//! ```
//!
//! ```
//! use ogar_secrets::{CredentialKind, CredentialRequest};
//! let r = CredentialRequest { kind: CredentialKind::GraphApp, version: None };
//! assert_eq!(r.kind.source(), "graph");
//! ```
//!
//! A grant cannot be built by hand either:
//!
//! ```compile_fail
//! use ogar_secrets::CredentialGrant;
//! let _ = CredentialGrant {
//!     tenant: 1,
//!     consumer: String::new(),
//!     subject: String::new(),
//!     verifier: "x",
//! };
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod memory;

use std::fmt;

use lance_graph_contract::rbac::{ClassId, Operation};
use lance_graph_contract::rbac_plug::RbacBinding;
use ogar_auth::federation::VerifiedLogin;
use ogar_rbac::authorize_identity;

pub use memory::InMemoryProvider;

/// The action name asked of `ogar-rbac`. An ACT grant permits every action
/// name on a class, so the pin that matters is the class.
pub const RELEASE_ACTION: &str = "release_credential";

/// The full classid of the `tenant_credential` concept (no app prefix).
#[must_use]
pub fn credential_class() -> ClassId {
    lance_graph_contract::render_classid(0x0000, ogar_vocab::class_ids::TENANT_CREDENTIAL)
}

/// The label rule for one path segment: non-empty, at most 128 bytes, ASCII
/// alphanumeric or `-` `_` `.`, and not made only of dots (so never `.`/`..`).
fn is_safe_label(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        && !s.bytes().all(|b| b == b'.')
}

/// Why [`CredentialGrant::authorize`] gave no grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantRefusal {
    /// The binding's plug does not declare the `tenant_credential` concept.
    NotPlugged,
    /// `ogar-rbac` did not answer Allow (Deny or Escalate).
    Denied,
    /// The consumer name fails the label rule.
    InvalidConsumer,
}

/// Proof that `ogar-rbac` admitted one verified workload to release credentials
/// of its own tenant, for one consumer. Private fields, one constructor.
#[derive(Debug, Clone)]
pub struct CredentialGrant {
    tenant: u64,
    consumer: String,
    subject: String,
    verifier: &'static str,
}

impl CredentialGrant {
    /// The only constructor.
    ///
    /// Steps, in order:
    /// 1. `binding.plugged(credential_class())` must succeed, else
    ///    [`GrantRefusal::NotPlugged`]. (A plug that does not declare the
    ///    concept cannot gate it; this is checked on the binding itself.)
    /// 2. `authorize_identity` on `credential_class()` with
    ///    `Operation::Act { action: RELEASE_ACTION }` must Allow, else
    ///    [`GrantRefusal::Denied`].
    /// 3. The tenant is `User.tenant` (the kernel scopes every membership to
    ///    it); the consumer is `binding.consumer()` and must pass the label
    ///    rule, else [`GrantRefusal::InvalidConsumer`].
    ///
    /// There is no extra check on an acting party. `establish` never produces a
    /// delegated login, and `IdentityActors` gives an acting party no role.
    ///
    /// # Errors
    ///
    /// A [`GrantRefusal`] naming the step that refused.
    pub fn authorize(
        binding: &RbacBinding,
        workload: &VerifiedLogin,
    ) -> Result<Self, GrantRefusal> {
        binding
            .plugged(credential_class())
            .map_err(|_| GrantRefusal::NotPlugged)?;
        let user = workload.authenticated_user();
        let d = authorize_identity(
            binding,
            user,
            credential_class(),
            Operation::Act {
                action: RELEASE_ACTION,
            },
        );
        if !d.decision.is_allowed() {
            return Err(GrantRefusal::Denied);
        }
        let consumer = binding.consumer();
        if !is_safe_label(consumer) {
            return Err(GrantRefusal::InvalidConsumer);
        }
        Ok(Self {
            tenant: user.user.tenant,
            consumer: consumer.to_string(),
            subject: user.user.subject.clone(),
            verifier: workload.verifier_id(),
        })
    }

    /// The tenant the credential belongs to.
    #[must_use]
    pub fn tenant(&self) -> u64 {
        self.tenant
    }

    /// The consumer the grant is for.
    #[must_use]
    pub fn consumer(&self) -> &str {
        &self.consumer
    }

    /// The workload's subject.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The id of the verifier that accepted the workload's login.
    #[must_use]
    pub fn verifier_id(&self) -> &'static str {
        self.verifier
    }
}

/// Which source credential is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialKind {
    /// A graph app credential.
    GraphApp,
    /// An ARM workload credential.
    ArmWorkload,
    /// A collector auth credential.
    CollectorAuth,
    /// A certificate.
    Certificate,
}

impl CredentialKind {
    /// The source path segment.
    #[must_use]
    pub const fn source(self) -> &'static str {
        match self {
            Self::GraphApp => "graph",
            Self::ArmWorkload => "arm",
            Self::CollectorAuth => "collector",
            Self::Certificate => "certificate",
        }
    }

    /// The service path segment.
    #[must_use]
    pub const fn service(self) -> &'static str {
        match self {
            Self::GraphApp => "app",
            Self::ArmWorkload => "workload",
            Self::CollectorAuth => "auth",
            Self::Certificate => "cert",
        }
    }
}

/// What a caller may ask for. No tenant, no consumer, no path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CredentialRequest {
    /// Which credential.
    pub kind: CredentialKind,
    /// A specific version, or the latest when `None`.
    pub version: Option<u32>,
}

/// A validated logical path:
/// `kv/data/<environment>/tenants/<tenant>/<consumer>/<source>/<service>`.
/// The provider maps it to its own mount. Built only inside the broker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretPath {
    segments: Vec<String>,
    joined: String,
}

impl SecretPath {
    fn build(
        environment: &str,
        tenant: u64,
        consumer: &str,
        source: &str,
        service: &str,
    ) -> Option<Self> {
        let tenant = tenant.to_string();
        let tail = [environment, &tenant, consumer, source, service];
        if !tail.iter().all(|s| is_safe_label(s)) {
            return None;
        }
        let segments: Vec<String> = [
            "kv",
            "data",
            environment,
            "tenants",
            &tenant,
            consumer,
            source,
            service,
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
        let joined = segments.join("/");
        Some(Self { segments, joined })
    }

    /// The path as one string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.joined
    }

    /// The path segments.
    #[must_use]
    pub fn segments(&self) -> &[String] {
        &self.segments
    }
}

impl fmt::Display for SecretPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.joined)
    }
}

/// Holds a value and never prints it.
///
/// No `Serialize`, no `PartialEq`. `Clone` only when `T` is.
pub struct Redacted<T>(T);

impl<T> Redacted<T> {
    /// Wrap a value.
    #[must_use]
    pub fn new(v: T) -> Self {
        Self(v)
    }

    /// The one accessor. Never call it from logging or audit code.
    #[must_use]
    pub fn expose(&self) -> &T {
        &self.0
    }
}

impl<T: Clone> Clone for Redacted<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

impl<T> fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

/// A released credential.
#[derive(Debug, Clone)]
pub struct CredentialMaterial {
    /// The version that was released.
    pub version: u32,
    /// The secret. Redacted in every format.
    pub secret: Redacted<String>,
}

/// Why the broker refused before asking the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalReason {
    /// The built path failed the label rule.
    InvalidPath,
    /// The environment name failed the label rule.
    InvalidEnvironment,
}

/// Why no credential came back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableReason {
    /// No secret at the path.
    NotFound,
    /// The path exists; the version does not.
    VersionNotFound,
    /// The provider could not be reached.
    ProviderDown,
    /// The provider is sealed.
    Sealed,
    /// The provider refused the broker itself.
    PermissionDenied,
}

/// The result of a release. No variant is an empty success.
#[derive(Debug)]
pub enum BrokerOutcome {
    /// The credential.
    Released(CredentialMaterial),
    /// Refused before the provider was asked.
    Refused(RefusalReason),
    /// The provider did not return a credential.
    Unavailable(UnavailableReason),
}

/// What a provider can fail with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderError {
    /// No secret at the path.
    NotFound,
    /// The path exists; the version does not.
    VersionNotFound,
    /// The provider could not be reached.
    Unreachable,
    /// The provider is sealed.
    Sealed,
    /// The provider refused the broker.
    PermissionDenied,
}

/// The backend seam (a Vault adapter later). It gets a validated path, never a
/// grant.
pub trait CredentialProvider {
    /// Fetch `path` at `version` (latest when `None`).
    ///
    /// # Errors
    ///
    /// A [`ProviderError`] when no credential can be returned.
    fn fetch(
        &self,
        path: &SecretPath,
        version: Option<u32>,
    ) -> Result<CredentialMaterial, ProviderError>;
}

/// The class of an outcome, for audit. Carries no secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeClass {
    /// A credential was released.
    Released,
    /// Refused before the provider.
    Refused(RefusalReason),
    /// The provider returned nothing.
    Unavailable(UnavailableReason),
}

/// One audit line, written for every release outcome. Carries no secret.
#[derive(Debug, Clone)]
pub struct AccessRecord {
    /// The workload's subject.
    pub subject: String,
    /// The tenant the grant names.
    pub tenant: u64,
    /// The consumer the grant names.
    pub consumer: String,
    /// The kind asked for.
    pub kind: CredentialKind,
    /// The version released, or asked for when nothing was released.
    pub version: Option<u32>,
    /// The logical path, when one was built.
    pub path: Option<String>,
    /// What happened.
    pub outcome: OutcomeClass,
}

/// Where audit lines go.
pub trait AuditSink {
    /// Record one line. Takes `&self`; use interior mutability.
    fn record(&self, r: AccessRecord);
}

/// Releases a credential for a grant.
pub trait TenantSecretBroker {
    /// Release the credential `request` names, for the grant's tenant only.
    fn release(&self, grant: &CredentialGrant, request: CredentialRequest) -> BrokerOutcome;
}

/// The reference broker: builds the path from the grant, asks the provider,
/// audits every outcome.
pub struct Broker<P: CredentialProvider, A: AuditSink> {
    environment: String,
    provider: P,
    audit: A,
}

impl<P: CredentialProvider, A: AuditSink> Broker<P, A> {
    /// A broker for one environment.
    ///
    /// # Errors
    ///
    /// [`RefusalReason::InvalidEnvironment`] when the name fails the label rule.
    pub fn new(environment: &str, provider: P, audit: A) -> Result<Self, RefusalReason> {
        if !is_safe_label(environment) {
            return Err(RefusalReason::InvalidEnvironment);
        }
        Ok(Self {
            environment: environment.to_string(),
            provider,
            audit,
        })
    }

    /// The provider (for test inspection).
    #[must_use]
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// The audit sink (for test inspection).
    #[must_use]
    pub fn audit(&self) -> &A {
        &self.audit
    }
}

impl<P: CredentialProvider, A: AuditSink> TenantSecretBroker for Broker<P, A> {
    fn release(&self, grant: &CredentialGrant, request: CredentialRequest) -> BrokerOutcome {
        let mut rec = AccessRecord {
            subject: grant.subject.clone(),
            tenant: grant.tenant,
            consumer: grant.consumer.clone(),
            kind: request.kind,
            version: request.version,
            path: None,
            outcome: OutcomeClass::Refused(RefusalReason::InvalidPath),
        };
        let Some(path) = SecretPath::build(
            &self.environment,
            grant.tenant,
            &grant.consumer,
            request.kind.source(),
            request.kind.service(),
        ) else {
            self.audit.record(rec);
            return BrokerOutcome::Refused(RefusalReason::InvalidPath);
        };
        rec.path = Some(path.as_str().to_string());
        let outcome = match self.provider.fetch(&path, request.version) {
            Ok(material) => {
                rec.version = Some(material.version);
                rec.outcome = OutcomeClass::Released;
                BrokerOutcome::Released(material)
            }
            Err(e) => {
                let why = match e {
                    ProviderError::NotFound => UnavailableReason::NotFound,
                    ProviderError::VersionNotFound => UnavailableReason::VersionNotFound,
                    ProviderError::Unreachable => UnavailableReason::ProviderDown,
                    ProviderError::Sealed => UnavailableReason::Sealed,
                    ProviderError::PermissionDenied => UnavailableReason::PermissionDenied,
                };
                rec.outcome = OutcomeClass::Unavailable(why);
                BrokerOutcome::Unavailable(why)
            }
        };
        self.audit.record(rec);
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_refuse_dots_empty_slashes_and_long() {
        for bad in [".", "..", "...", "", "a/b", "a b"] {
            assert!(!is_safe_label(bad), "{bad:?} must be refused");
        }
        assert!(!is_safe_label(&"a".repeat(129)));
        for ok in ["lab", "1001", "graph", "a.b", "a-b_c", &"a".repeat(128)] {
            assert!(is_safe_label(ok), "{ok:?} must pass");
        }
    }

    #[test]
    fn path_builder_refuses_any_bad_segment() {
        let p = SecretPath::build("lab", 1001, "sentinel-rs", "graph", "app").expect("valid");
        assert_eq!(p.as_str(), "kv/data/lab/tenants/1001/sentinel-rs/graph/app");
        assert_eq!(p.segments().len(), 8);
        assert!(SecretPath::build("..", 1001, "c", "graph", "app").is_none());
        assert!(SecretPath::build("lab", 1001, "..", "graph", "app").is_none());
        assert!(SecretPath::build("lab", 1001, "a/b", "graph", "app").is_none());
        assert!(SecretPath::build("lab", 1001, "c", "", "app").is_none());
    }
}
