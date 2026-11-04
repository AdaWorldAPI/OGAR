//! `federation` — the federated-login boundary: issuer-scoped resolution and
//! the [`VerifiedLogin`] proof type.
//!
//! This module records the one invariant a federated adapter MUST honour so the
//! endgame is legible from the SDK, and so nobody re-derives it:
//!
//! > **A federated login and a local login converge on the SAME identity
//! > envelope before any authorization decision is made.** The IdP adapter's
//! > job ends at producing verified claims; [`establish`] maps them to the
//! > canonical user and the envelope is
//! > [`AuthenticatedUser::actor_context`](crate::user::AuthenticatedUser::actor_context)
//! > (mirrors `lance_graph_contract::auth::ActorContext`). Authorization code
//! > downstream is identical whether the actor authenticated locally or through
//! > an IdP; the IdP is a *source of the envelope*, never a fork in the
//! > authorization logic.
//!
//! # What lives here
//!
//! | Item | Role |
//! |---|---|
//! | [`TrustedIssuers`] | deployment-configured `Issuer → ProviderId` registry |
//! | [`AssertionVerifier`] | the seam a token/introspection adapter implements |
//! | [`VerifiedClaims`] | what a verifier returns — **not proof** |
//! | [`establish`] | verify → trust check → `(iss, sub)` resolution → [`VerifiedLogin`] |
//! | [`VerifiedLogin`] | the proof of a federated login |
//!
//! Token validation, JWKS fetch and claim parsing are still out of scope: they
//! belong to a sibling adapter implementing [`AssertionVerifier`]. This module
//! is provider-agnostic.
//!
//! # Resolution rules
//!
//! - A login resolves by the full `(issuer, subject)` pair through
//!   [`UserStore::resolve_external`]. **Never** by email, by a provider label,
//!   or by an [`AuthBinding`](crate::user::AuthBinding): an unbound identity is
//!   [`LoginError::Unbound`], full stop.
//! - The [`ProviderId`] recorded on the login comes from the
//!   [`TrustedIssuers`] registry the deployment configured, never from the
//!   verifier's claims.
//!
//! # What visibility can and cannot guarantee
//!
//! [`VerifiedLogin`] has private fields and no public constructor, `From` or
//! `Default`, so safe code outside this module cannot fabricate one except
//! through [`establish`]. That is a *type-level* guarantee only. It does **not**
//! stop:
//!
//! - an [`AssertionVerifier`] implementation (in any crate) that lies — it can
//!   return whatever [`VerifiedClaims`] it likes and [`establish`] will trust
//!   it; and
//! - code that hand-builds an [`AuthenticatedUser`] (its fields are public) and
//!   passes it to RBAC directly.
//!
//! Which verifier implementations are linked into a binary, and which code may
//! call RBAC with a hand-built user, is a **deployment trust boundary**, not
//! something this crate can enforce.
//!
//! ```compile_fail
//! // A VerifiedLogin cannot be built with a struct literal from outside.
//! use ogar_auth::federation::VerifiedLogin;
//! fn forge(
//!     user: ogar_auth::user::AuthenticatedUser,
//!     identity: ogar_auth::identity::ExternalIdentity,
//! ) -> VerifiedLogin {
//!     VerifiedLogin { user, identity, verifier: "forged" }
//! }
//! ```
//!
//! ```compile_fail
//! // Nor converted from an AuthenticatedUser.
//! fn forge(some_authenticated_user: ogar_auth::user::AuthenticatedUser) {
//!     let _: ogar_auth::federation::VerifiedLogin = some_authenticated_user.into();
//! }
//! ```

use std::collections::BTreeMap;

use lance_graph_contract::auth::ActorContext;

use crate::identity::{ExternalIdentity, Issuer};
use crate::user::{AuthContext, AuthStrength, AuthenticatedUser, ProviderId, UserStore};

/// An issuer is already registered in [`TrustedIssuers`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuerConflict {
    /// The issuer that was already present.
    pub issuer: Issuer,
}

impl core::fmt::Display for IssuerConflict {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "issuer {:?} is already registered", self.issuer.as_str())
    }
}

impl std::error::Error for IssuerConflict {}

/// The deployment's registry of trusted issuers and the provider label each
/// maps to. An issuer absent from it is not trusted, whatever a verifier says.
#[derive(Debug, Clone, Default)]
pub struct TrustedIssuers {
    map: BTreeMap<Issuer, ProviderId>,
}

impl TrustedIssuers {
    /// An empty registry (trusts nothing).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Trust `issuer`, labelled `provider`. Never overwrites.
    ///
    /// # Errors
    ///
    /// [`IssuerConflict`] if the issuer is already registered.
    pub fn insert(&mut self, issuer: Issuer, provider: ProviderId) -> Result<(), IssuerConflict> {
        if self.map.contains_key(&issuer) {
            return Err(IssuerConflict { issuer });
        }
        self.map.insert(issuer, provider);
        Ok(())
    }

    /// The provider label registered for `issuer`, if it is trusted.
    #[must_use]
    pub fn provider_of(&self, issuer: &Issuer) -> Option<ProviderId> {
        self.map.get(issuer).copied()
    }
}

/// Claims a verifier extracted from an assertion.
///
/// Returned by a verifier; **NOT proof** of anything until [`establish`] has
/// checked the issuer and resolved the identity. `email` is a display hint,
/// never a linking key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedClaims {
    /// The issuer-scoped identity the assertion names.
    pub identity: ExternalIdentity,
    /// How strongly the IdP says the user authenticated.
    pub strength: AuthStrength,
    /// A display hint only. Never used to find or link a user.
    pub email: Option<String>,
}

/// The seam a token / introspection adapter implements.
///
/// This crate defines no implementation. The verifier is trusted to have
/// validated the assertion (signature, audience, expiry…); see the module docs
/// for what that trust means.
pub trait AssertionVerifier {
    /// The assertion the verifier consumes (e.g. a raw ID token string).
    type Input: ?Sized;
    /// The verifier's own failure type.
    type Error;
    /// A stable identifier for this verifier implementation, recorded on the
    /// resulting [`VerifiedLogin`].
    const ID: &'static str;

    /// Validate `input` and extract its claims.
    ///
    /// # Errors
    ///
    /// The verifier's own error when the assertion is invalid.
    fn verify(
        &self,
        trusted: &TrustedIssuers,
        input: &Self::Input,
    ) -> Result<VerifiedClaims, Self::Error>;
}

/// Why [`establish`] refused a login.
#[derive(Debug)]
pub enum LoginError<E> {
    /// The verifier rejected the assertion.
    Verify(E),
    /// The asserted issuer is not in [`TrustedIssuers`].
    UntrustedIssuer,
    /// No user is bound to the asserted `(issuer, subject)`.
    Unbound,
}

impl<E> core::fmt::Display for LoginError<E> {
    /// Deliberately omits the verifier error and any claim contents.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Verify(_) => f.write_str("assertion verification failed"),
            Self::UntrustedIssuer => f.write_str("issuer is not trusted"),
            Self::Unbound => f.write_str("no user is bound to this external identity"),
        }
    }
}

impl<E: core::fmt::Debug> std::error::Error for LoginError<E> {}

/// The proof of a federated login: the verifier accepted the assertion, the
/// issuer is trusted, and the `(issuer, subject)` is bound to a canonical user.
///
/// Fields are private and there is no public constructor, `From` or `Default`;
/// the only producer is [`establish`]. See the module docs for what this does
/// and does not guarantee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedLogin {
    user: AuthenticatedUser,
    identity: ExternalIdentity,
    verifier: &'static str,
}

impl VerifiedLogin {
    /// The authenticated user this login established.
    #[must_use]
    pub fn authenticated_user(&self) -> &AuthenticatedUser {
        &self.user
    }

    /// Consume the proof, yielding the user for authorization.
    #[must_use]
    pub fn into_authenticated_user(self) -> AuthenticatedUser {
        self.user
    }

    /// The `(issuer, subject)` the login resolved by.
    #[must_use]
    pub fn identity(&self) -> &ExternalIdentity {
        &self.identity
    }

    /// The [`AssertionVerifier::ID`] that accepted the assertion.
    #[must_use]
    pub fn verifier_id(&self) -> &'static str {
        self.verifier
    }

    /// The canonical identity envelope (delegates to the user).
    #[must_use]
    pub fn actor_context(&self) -> ActorContext {
        self.user.actor_context()
    }
}

/// Run a federated login: verify, check the issuer, resolve `(iss, sub)`.
///
/// Order is fixed: the verifier runs first; the asserted issuer must then be in
/// `trusted`; the identity must then resolve through
/// [`UserStore::resolve_external`]. There is **no fallback** to email or to an
/// [`AuthBinding`](crate::user::AuthBinding). The provider label on the
/// resulting [`AuthContext`] comes from `trusted`, not from the claims.
///
/// # Errors
///
/// [`LoginError::Verify`] from the verifier; [`LoginError::UntrustedIssuer`] if
/// the issuer is unregistered; [`LoginError::Unbound`] if no user is bound to
/// the identity.
pub fn establish<V: AssertionVerifier, S: UserStore + ?Sized>(
    v: &V,
    trusted: &TrustedIssuers,
    input: &V::Input,
    store: &S,
) -> Result<VerifiedLogin, LoginError<V::Error>> {
    let claims = v.verify(trusted, input).map_err(LoginError::Verify)?;
    let provider = trusted
        .provider_of(claims.identity.issuer())
        .ok_or(LoginError::UntrustedIssuer)?;
    let user = store
        .resolve_external(&claims.identity)
        .ok_or(LoginError::Unbound)?
        .clone();
    Ok(VerifiedLogin {
        user: AuthenticatedUser {
            user,
            auth: AuthContext::federated(provider, claims.strength),
        },
        identity: claims.identity,
        verifier: V::ID,
    })
}
