//! `user` — the canonical OGAR user, and the bindings external identity
//! providers attach to it.
//!
//! # Why this lives in `ogar-auth`
//!
//! `ogar-auth` owns *who you are*. `ogar-rbac` owns *what you may do*, and it
//! depends on this module rather than on any parallel identity normalizer, so
//! the crate graph itself states the invariant: **authorization operates on
//! OGAR's canonical user, never on an arbitrary claim bag.**
//!
//! # This realizes an ALREADY-MINTED vocabulary — it invents nothing
//!
//! The semantics are not new. `ogar-vocab` already mints the classes, and this
//! module is the Rust surface for them:
//!
//! | vocabulary                        | classid  | here                     |
//! |-----------------------------------|----------|--------------------------|
//! | `auth_store` (IdP→classid mapping)| `0x0B01` | [`UserStore`]            |
//! | `project_actor`                   | `0x0104` | [`User`] / [`UserId`]    |
//! | `project_role`                    | `0x0117` | [`User::roles`]          |
//! | `project_membership`              | `0x0108` | [`User::memberships`]    |
//! | `auth_zitadel` / `auth_ory_keto` … | `0x0B02`+| [`AuthBinding::provider`]|
//!
//! `auth_store`'s own OGAR definition carries `sub_claim` / `role_claim` /
//! `org_claim` as **attributes**, and each provider profile is an `is-a` child
//! carrying its `claim_grammar` as **data**. Provider ignorance is therefore a
//! property of the vocabulary, not a promise made by this code: a new IdP is a
//! preminted class with a different `claim_grammar` row, never a new match arm.
//!
//! # The convergence invariant this closes
//!
//! The `federation` module has recorded the requirement since the crate was
//! written: *"a federated login and a local login converge on the SAME identity
//! envelope before any authorization decision is made … the IdP is a source of
//! the envelope, never a fork in the authorization logic."* That envelope is
//! [`lance_graph_contract::auth::ActorContext`], and [`AuthenticatedUser::actor_context`]
//! is the single place it is produced.
//!
//! ```text
//!   ("zitadel", external_subject) ─┐
//!   ("entra",   object_id)        ─┼──► UserStore::resolve ──► User(42)
//!   ("local",   "dr-house")       ─┘                            │
//!   kiosk ──────────────────────────────────────────────────────┤
//!                                                               ▼
//!                                                        ActorContext
//!                                                               │
//!                                                               ▼
//!                                                          ogar-rbac
//! ```
//!
//! # What this module deliberately does NOT do
//!
//! - **No token validation, no JWKS, no claim parsing.** Those belong in an
//!   `ogar-adapter-*` sibling. This module starts *after* an adapter (or the
//!   local password/TOTP path, or kiosk) has already proven the binding.
//! - **No secret aggregation.** A [`User`] holds opaque [`KeyRef`]s and never
//!   key material; there is deliberately no "give me this user's keys" method.
//!   See [`UserStore`]'s contract.
//! - **No role derivation from login method.** Roles are a durable property of
//!   the [`User`]; [`AuthContext`] describes *how* they logged in and never
//!   contributes a role.

use lance_graph_contract::auth::ActorContext;
use lance_graph_contract::sla::TenantId;

/// The canonical OGAR user id — `project_actor` (`0x0104`) as a value.
///
/// Opaque and internal: an external subject is *bound* to one of these
/// ([`AuthBinding`]), never equal to one. Two providers naming the same human
/// resolve to the same `UserId`, which is what lets authorization ignore which
/// provider was used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(pub u64);

/// An authentication provider, as an **opaque label** — never an enum.
///
/// Deliberately not `enum { Zitadel, Entra, Keycloak, … }`: an enum would put
/// the provider matrix in the type system, and every consumer that matched on
/// it would become a place a new IdP has to be taught about. The provider is a
/// key into the preminted `auth_store` family (`auth_zitadel` `0x0B02`, …)
/// whose claim grammar is data. Nothing downstream of `ogar-auth` may branch on
/// this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProviderId(pub &'static str);

impl ProviderId {
    /// The built-in local credential path (password + TOTP, this crate).
    pub const LOCAL: Self = Self("local");
    /// The unauthenticated kiosk path — a real, supported mode, not a stub.
    pub const KIOSK: Self = Self("kiosk");

    /// The provider label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// One external identity bound to a canonical [`User`].
///
/// A user may hold several: `("zitadel", "a1b2…")` and `("entra", "0000-…")`
/// can both resolve to the same [`UserId`]. That is the point — it is what
/// makes an IdP swap invisible to authorization.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthBinding {
    /// Which provider asserted the subject.
    pub provider: ProviderId,
    /// The provider's own subject identifier (OIDC `sub`, Entra object id,
    /// local username…). Stored verbatim; never parsed here.
    pub external_subject: String,
}

impl AuthBinding {
    /// Bind `external_subject` as asserted by `provider`.
    #[must_use]
    pub fn new(provider: ProviderId, external_subject: impl Into<String>) -> Self {
        Self {
            provider,
            external_subject: external_subject.into(),
        }
    }
}

/// An opaque handle to key material held by the encryption authority.
///
/// A reference, never a key: `ogar-auth` associates crypto state with a user
/// without becoming the identity model's key drawer. Resolving one is
/// `ogar-encryption`'s job, under its own authorization.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyRef(pub String);

/// How strongly the actor authenticated. Descriptive only — it never
/// contributes a role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthStrength {
    /// No credential presented (kiosk).
    None,
    /// One factor (password, or a provider's own single-factor assertion).
    SingleFactor,
    /// Two or more factors (e.g. password + TOTP).
    MultiFactor,
}

/// Which channel the actor arrived through. Descriptive only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthChannel {
    /// A shared, unauthenticated terminal.
    Kiosk,
    /// Local credentials verified by this crate.
    Local,
    /// A web/federated session established by an adapter.
    Web,
}

/// The facts about *this login* — as opposed to [`User`], which is durable.
///
/// Roles are deliberately absent: they belong to the identity. Keeping the two
/// apart is what stops "logged in via X" from silently becoming a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthContext {
    /// Did a credential actually verify? `false` for kiosk.
    pub authenticated: bool,
    /// The channel used.
    pub channel: AuthChannel,
    /// Which provider asserted the identity.
    pub provider: ProviderId,
    /// How strong the assertion was.
    pub strength: AuthStrength,
}

impl AuthContext {
    /// The kiosk context — unauthenticated by construction.
    #[must_use]
    pub const fn kiosk() -> Self {
        Self {
            authenticated: false,
            channel: AuthChannel::Kiosk,
            provider: ProviderId::KIOSK,
            strength: AuthStrength::None,
        }
    }

    /// A verified local login.
    #[must_use]
    pub const fn local(strength: AuthStrength) -> Self {
        Self {
            authenticated: true,
            channel: AuthChannel::Local,
            provider: ProviderId::LOCAL,
            strength,
        }
    }

    /// A verified federated login, asserted by `provider`.
    #[must_use]
    pub const fn federated(provider: ProviderId, strength: AuthStrength) -> Self {
        Self {
            authenticated: true,
            channel: AuthChannel::Web,
            provider,
            strength,
        }
    }
}

/// The canonical OGAR user — durable identity facts, independent of any login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// Canonical id (`project_actor` `0x0104`).
    pub id: UserId,
    /// The stable subject string handed to authorization as
    /// [`ActorContext::actor_id`].
    pub subject: String,
    /// Tenant the user belongs to.
    pub tenant: TenantId,
    /// Roles held (`project_role` `0x0117`). A durable property of the user —
    /// never derived from how they logged in.
    pub roles: Vec<String>,
    /// Memberships (`project_membership` `0x0108`), as opaque keys.
    pub memberships: Vec<String>,
    /// External identities that resolve to this user.
    pub bindings: Vec<AuthBinding>,
    /// Opaque references to this user's key material. Never the material.
    pub key_refs: Vec<KeyRef>,
}

/// A [`User`] together with the [`AuthContext`] of the current login.
///
/// This is the type `ogar-rbac` consumes. It is the *only* sanctioned input to
/// authorization, which is what makes "RBAC operates on the canonical user"
/// checkable rather than aspirational.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedUser {
    /// The canonical user.
    pub user: User,
    /// How this session was established.
    pub auth: AuthContext,
}

impl AuthenticatedUser {
    /// Produce the canonical identity envelope.
    ///
    /// This is the single convergence point the [`crate::federation`] stub
    /// specified: local, kiosk and federated logins all arrive here and are
    /// indistinguishable to everything downstream.
    #[must_use]
    pub fn actor_context(&self) -> ActorContext {
        ActorContext::new(
            self.user.subject.clone(),
            self.user.tenant,
            self.user.roles.clone(),
        )
    }
}

/// Resolve external identity bindings to canonical users.
///
/// This is the `auth_store` (`0x0B01`) surface: it *maps*, it does not mint
/// credentials and it does not release secrets.
///
/// # The one method that must never exist
///
/// There is deliberately no `all_keys`, `secrets_of`, or equivalent. A user
/// record may *reference* crypto authority ([`User::key_refs`]) without owning
/// the plaintext keys — otherwise an identity lookup silently becomes
/// unrestricted key retrieval, and the compartmentalization the label
/// architecture depends on is defeated at its cheapest point. Implementors
/// must not add one.
pub trait UserStore {
    /// The canonical user an external binding resolves to, if any.
    fn resolve(&self, binding: &AuthBinding) -> Option<&User>;

    /// Look a canonical user up directly.
    fn user(&self, id: UserId) -> Option<&User>;
}

/// An in-memory [`UserStore`] — the local/kiosk path, and the fixture an
/// adapter-free build uses. Not a placeholder for a database so much as the
/// honest shape of a single-tenant local deployment.
#[derive(Debug, Clone, Default)]
pub struct LocalUserStore {
    users: Vec<User>,
}

impl LocalUserStore {
    /// An empty store.
    #[must_use]
    pub const fn new() -> Self {
        Self { users: Vec::new() }
    }

    /// Add a user. Later lookups resolve any of its bindings.
    pub fn insert(&mut self, user: User) {
        self.users.push(user);
    }
}

impl UserStore for LocalUserStore {
    fn resolve(&self, binding: &AuthBinding) -> Option<&User> {
        self.users.iter().find(|u| u.bindings.contains(binding))
    }

    fn user(&self, id: UserId) -> Option<&User> {
        self.users.iter().find(|u| u.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZITADEL: ProviderId = ProviderId("zitadel");
    const ENTRA: ProviderId = ProviderId("entra");

    fn store() -> LocalUserStore {
        let mut s = LocalUserStore::new();
        s.insert(User {
            id: UserId(42),
            subject: "dr-house".to_string(),
            tenant: 7,
            roles: vec!["physician".to_string()],
            memberships: vec!["ward-3".to_string()],
            bindings: vec![
                AuthBinding::new(ZITADEL, "a1b2c3"),
                AuthBinding::new(ENTRA, "0000-1111"),
                AuthBinding::new(ProviderId::LOCAL, "dr-house"),
            ],
            key_refs: vec![KeyRef("kms://user/42".to_string())],
        });
        s
    }

    /// F6 — two different providers, one canonical user. This is the property
    /// that makes an IdP swap invisible to `ogar-rbac`.
    #[test]
    fn two_bindings_resolve_to_one_canonical_user() {
        let s = store();
        let via_zitadel = s
            .resolve(&AuthBinding::new(ZITADEL, "a1b2c3"))
            .expect("zitadel binding resolves");
        let via_entra = s
            .resolve(&AuthBinding::new(ENTRA, "0000-1111"))
            .expect("entra binding resolves");
        assert_eq!(via_zitadel.id, via_entra.id);
        assert_eq!(via_zitadel.id, UserId(42));
        // and the envelope handed to authorization is identical either way
        let a = AuthenticatedUser {
            user: via_zitadel.clone(),
            auth: AuthContext::federated(ZITADEL, AuthStrength::MultiFactor),
        };
        let b = AuthenticatedUser {
            user: via_entra.clone(),
            auth: AuthContext::federated(ENTRA, AuthStrength::SingleFactor),
        };
        assert_eq!(
            a.actor_context(),
            b.actor_context(),
            "provider must not survive into the identity envelope"
        );
    }

    /// An unknown binding resolves to nothing — never to a default user.
    #[test]
    fn unknown_binding_resolves_to_nothing() {
        let s = store();
        assert!(
            s.resolve(&AuthBinding::new(ZITADEL, "not-a-subject"))
                .is_none()
        );
        assert!(
            s.resolve(&AuthBinding::new(ProviderId("okta"), "a1b2c3"))
                .is_none()
        );
    }

    /// Roles come from the identity, not the login. The same user authenticating
    /// through the weakest and strongest paths carries the same roles.
    #[test]
    fn roles_do_not_depend_on_login_method() {
        let s = store();
        let u = s.user(UserId(42)).expect("user exists").clone();
        let kiosk = AuthenticatedUser {
            user: u.clone(),
            auth: AuthContext::kiosk(),
        };
        let mfa = AuthenticatedUser {
            user: u,
            auth: AuthContext::local(AuthStrength::MultiFactor),
        };
        assert_eq!(kiosk.actor_context().roles, mfa.actor_context().roles);
        assert!(!kiosk.auth.authenticated);
        assert!(mfa.auth.authenticated);
    }

    /// F7 — the store maps identities; it does not hand out key material.
    /// `key_refs` are opaque handles, and there is no aggregate accessor.
    #[test]
    fn store_exposes_key_references_not_key_material() {
        let s = store();
        let u = s.user(UserId(42)).expect("user exists");
        assert_eq!(u.key_refs, vec![KeyRef("kms://user/42".to_string())]);
        // The type carries a reference only — resolving it is the encryption
        // authority's job, under its own authorization.
        assert!(u.key_refs[0].0.starts_with("kms://"));
    }
}
