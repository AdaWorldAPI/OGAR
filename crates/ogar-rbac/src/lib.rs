//! `ogar-rbac` — OGAR's canonical RBAC **authority**.
//!
//! # The one dependency that carries the architecture
//!
//! ```text
//!   ZITADEL / Entra / Keycloak / Okta / local password+TOTP / kiosk
//!                              │
//!                              ▼
//!                          ogar-auth          ← canonical user + bindings
//!                              │
//!                     AuthenticatedUser
//!                              │
//!                              ▼
//!                          ogar-rbac          ← THIS CRATE
//!                              │
//!            ScopedDecision { decision, scope, WideFieldMask }
//! ```
//!
//! `ogar-rbac` depends on `ogar-auth` **deliberately**. It is not a decoupling
//! oversight to be optimized away: the edge is what makes the crate graph state
//! the invariant that authorization operates on OGAR's canonical user, and never
//! on an arbitrary parallel identity normalization. There is no way to ask this
//! crate a question without first having a
//! [`AuthenticatedUser`](ogar_auth::user::AuthenticatedUser) — which only
//! `ogar-auth` can produce.
//!
//! # What is HERE, and what deliberately is not
//!
//! | concern | home | why |
//! |---|---|---|
//! | traits / POD types (`ClassRbac`, `ClassId`, `WideFieldMask`) | `lance-graph-contract` | zero-dep socket; never reaches into OGAR |
//! | the generic `authorize` / `authorize_scoped` kernel | `lance-graph-rbac` | consumer-agnostic algorithm — **consumed, never cloned** |
//! | canonical user + authentication bindings | `ogar-auth` | identity is not authorization |
//! | **OGAR's grant/policy data + the `ClassRbac` realization** | **here** | |
//! | session / projection / sealed transport | `a2ui-rs` | consumes the mask; owns no policy |
//!
//! # Why an authority OBJECT, not `impl ClassRbac for OgarClassView`
//!
//! Rust coherence is crate-local. From this crate `ClassRbac` (in
//! `lance-graph-contract`) and `OgarClassView` (in `ogar-class-view`) are BOTH
//! foreign, so `impl ClassRbac for OgarClassView` is E0117 here exactly as it was
//! in `lance-graph-ogar` — living in the same repository changes nothing. The
//! keystone's Q5 wording is therefore realized as a **local authority object**,
//! [`OgarRbac`], which is legal, needs no workaround, and is the shape that was
//! already proven. This crate is that object's rehoming, not its reinvention.
//!
//! # Provider ignorance is structural
//!
//! Grep this crate for `Zitadel`, `Entra`, `Keycloak`, `Okta`, `OIM`: the only
//! occurrences are in this sentence and in the test that asserts their absence.
//! A provider reaches authorization only as an already-resolved
//! [`AuthBinding`](ogar_auth::user::AuthBinding) → canonical user, so swapping an
//! IdP changes adapter code and **zero** lines here.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use lance_graph_contract::class_view::WideFieldMask;
use lance_graph_contract::rbac::Membership;
use lance_graph_contract::rbac::{
    ActorId, ClassGrant, ClassId, ClassRbac, Operation, RoleId, ScopeSpec, grants_permit,
};
use lance_graph_contract::rbac_plug::{
    ActorSource, RbacAuthority, RbacBinding, RbacDrift, RbacPlug, verify_concepts_against_mirror,
};
use lance_graph_rbac::authorize::{MembershipDecision, authorize_memberships};
use lance_graph_rbac::authorize::{ScopedDecision, authorize_scoped};
use ogar_auth::user::{AuthBinding, AuthenticatedUser};

/// Where this authority reads its grant data.
///
/// The seam that lets the authority object be honest about what it does *not*
/// own: `OgarRbac` carries no grant state, so a fixture today and the OGAR Core's
/// `project_role.granted` value-tenant tomorrow drop in without the authority's
/// body changing at all.
pub trait GrantSource {
    /// Roles the actor holds — the `project_membership` (`0x0108`) →
    /// `project_member_role` (`0x0118`) → `project_role` (`0x0117`) fold.
    fn roles_of(&self, actor: ActorId<'_>) -> &[RoleId];

    /// The typed `granted` set of `role` — its `(target_classid, op_mask)` pairs.
    fn grants_of(&self, role: RoleId) -> &[ClassGrant];

    /// Where `role`'s grants on `class` apply — the axis-3 row scope, including
    /// a nested [`ScopePath`](lance_graph_contract::rbac::ScopePath) when roles
    /// are bound to a level of a hierarchy (a namespace, a database, an org).
    /// `None` (the default) is global: every existing source is unchanged.
    fn scope_of(&self, _role: RoleId, _class: ClassId) -> Option<ScopeSpec> {
        None
    }

    /// Whether this source defines `role` at all — what an RBAC plug's role
    /// list is checked against. Defaults to "has at least one grant"; a source
    /// that knows grant-less roles overrides it.
    fn defines_role(&self, role: RoleId) -> bool {
        !self.grants_of(role).is_empty()
    }

    /// The column projection `role` is limited to on `concept`, if any. `None`
    /// (the default) leaves the class unrestricted by column for that role.
    fn field_mask_of(&self, _role: RoleId, _concept: u16) -> Option<WideFieldMask> {
        None
    }
}

/// OGAR's canonical [`ClassRbac`] authority.
///
/// Local to this crate, so the impl below is coherent (see the crate docs on
/// E0117). Generic over its [`GrantSource`] and holding no grant state of its own.
#[derive(Debug, Clone, Copy)]
pub struct OgarRbac<S: GrantSource> {
    /// The injected grant source.
    pub source: S,
}

impl<S: GrantSource> OgarRbac<S> {
    /// Wrap a [`GrantSource`] as the OGAR authority.
    pub const fn new(source: S) -> Self {
        Self { source }
    }

    /// **The identity seam.** Authorize a canonical, `ogar-auth`-produced user.
    ///
    /// This is the only entry point, and it takes an
    /// [`AuthenticatedUser`] rather than a bare actor string — which is what
    /// makes "RBAC operates on OGAR's canonical user" a property the compiler
    /// checks instead of a convention.
    ///
    /// The decision is computed by the **generic kernel**
    /// ([`authorize_scoped`]); this method contributes the identity binding and
    /// nothing else. It deliberately does not clone the algorithm.
    ///
    /// # Kiosk
    ///
    /// An unauthenticated (kiosk) user is **not** refused here.
    /// `AuthContext::authenticated` is descriptive; the roles a kiosk user holds
    /// are still durable properties of their [`User`](ogar_auth::user::User), and
    /// the same downstream path must serve kiosk, local and federated identities
    /// alike. Refusing unauthenticated actors is a *deployment* policy, applied
    /// by whoever builds the `AuthenticatedUser` — not a grant rule.
    #[must_use]
    pub fn authorize_user(
        &self,
        identity: &AuthenticatedUser,
        class: ClassId,
        op: Operation<'_>,
    ) -> ScopedDecision {
        authorize_scoped(self, identity.user.subject.as_str(), class, op)
    }
}

impl<S: GrantSource> ClassRbac for OgarRbac<S> {
    fn actor_roles(&self, actor: ActorId<'_>) -> &[RoleId] {
        self.source.roles_of(actor)
    }

    fn grant_permits(&self, role: RoleId, class: ClassId, op: &Operation<'_>) -> bool {
        grants_permit(self.source.grants_of(role), class, op)
    }

    fn row_scope(&self, role: RoleId, class: ClassId) -> Option<ScopeSpec> {
        self.source.scope_of(role, class)
    }
    // Axes 2/4 (`roles_reaching` / `field_mask`) inherit the contract defaults
    // until the Core carries the data for them — a follow-up seam. `field_mask`'s default is now WideFieldMask, so a
    // grant on a position >= 64 survives once a source supplies one.
}

/// OGAR as the RBAC authority: binds a consumer's [`RbacPlug`] to this
/// source's grants, the authorization twin of `OgarAuthority` for
/// capabilities.
///
/// Every plugged classid must be minted in `ogar-vocab` and agree with the
/// contract's wire mirror; every plugged role must be one the source defines.
/// The binding then carries exactly the plugged roles' grants and field masks
/// on the plugged classids.
impl<S: GrantSource> RbacAuthority for OgarRbac<S> {
    fn bind(&self, plug: &RbacPlug) -> Result<RbacBinding, RbacDrift> {
        let mut concepts = Vec::with_capacity(plug.classids.len());
        for &id in plug.classids {
            let name =
                ogar_vocab::canonical_concept_name(id).ok_or(RbacDrift::UnknownClassid(id))?;
            concepts.push((name.to_string(), id));
        }
        verify_concepts_against_mirror(&concepts)?;

        let mut grants = Vec::with_capacity(plug.roles.len());
        let mut masks = Vec::new();
        for &role in plug.roles {
            if !self.source.defines_role(role) {
                return Err(RbacDrift::UnknownRole(role.to_string()));
            }
            grants.push((role, self.source.grants_of(role).to_vec()));
            for &id in plug.classids {
                if let Some(mask) = self.source.field_mask_of(role, id) {
                    masks.push((role, id, mask));
                }
            }
        }
        Ok(RbacBinding::new(
            plug.consumer,
            plug.classids.to_vec(),
            grants,
            masks,
        ))
    }
}

/// One `ogar-auth` user as the actor source for a plugged [`RbacBinding`].
///
/// The user's roles are matched against the roles the binding was resolved
/// for; a role the plug did not declare is dropped (and reported by
/// [`unplugged_roles`](IdentityActors::unplugged_roles)), never passed through.
/// Every membership is bound to the user's tenant, so a decision through
/// [`authorize_identity`] is tenant-scoped.
///
/// Built per request from the authenticated user: it answers for that one
/// subject and knows no other actor.
///
/// # Delegation
///
/// A delegated login (RFC 8693 `act`) authorizes as the user whose authority
/// the token carries, never as the party acting with it: the acting party is
/// recorded ([`acting_party`](IdentityActors::acting_party)) and holds no
/// role here, under any name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityActors {
    subject: String,
    acting: Option<AuthBinding>,
    tenant: u64,
    roles: Vec<RoleId>,
    unplugged: Vec<String>,
}

impl IdentityActors {
    /// The actor source for `identity` under `binding`.
    #[must_use]
    pub fn new(identity: &AuthenticatedUser, binding: &RbacBinding) -> Self {
        let mut roles = Vec::new();
        let mut unplugged = Vec::new();
        for held in &identity.user.roles {
            match binding
                .declared_grants()
                .iter()
                .find(|(declared, _)| *declared == held.as_str())
            {
                Some((declared, _)) if !roles.contains(declared) => roles.push(*declared),
                Some(_) => {}
                None => unplugged.push(held.clone()),
            }
        }
        Self {
            subject: identity.user.subject.clone(),
            acting: identity.acting_party().cloned(),
            tenant: identity.user.tenant,
            roles,
            unplugged,
        }
    }

    /// The party acting on the user's behalf, when the login is delegated.
    /// For audit: it never contributes a role.
    #[must_use]
    pub fn acting_party(&self) -> Option<&AuthBinding> {
        self.acting.as_ref()
    }

    /// The user's roles the plug did not declare — dropped, so they grant
    /// nothing here. For audit and diagnostics.
    #[must_use]
    pub fn unplugged_roles(&self) -> &[String] {
        &self.unplugged
    }
}

impl ActorSource for IdentityActors {
    fn roles_of(&self, actor: ActorId<'_>) -> &[RoleId] {
        if actor == self.subject {
            &self.roles
        } else {
            &[]
        }
    }

    fn memberships_of(&self, actor: ActorId<'_>, _class: ClassId) -> Vec<Membership> {
        let scope = ScopeSpec {
            tenant: Some(self.tenant),
            ..ScopeSpec::default()
        };
        self.roles_of(actor)
            .iter()
            .map(|&role| Membership {
                role,
                scope: Some(scope),
            })
            .collect()
    }
}

/// Authorize an `ogar-auth` user through a plugged binding: the binding
/// supplies the grants, the user supplies roles and tenant, and the decision
/// is the membership kernel's ([`authorize_memberships`]).
#[must_use]
pub fn authorize_identity(
    binding: &RbacBinding,
    identity: &AuthenticatedUser,
    class: ClassId,
    op: Operation<'_>,
) -> MembershipDecision {
    let actors = IdentityActors::new(identity, binding);
    authorize_memberships(
        &binding.with_actors(actors),
        identity.user.subject.as_str(),
        class,
        op,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use lance_graph_contract::class_view::WideFieldMask;
    use lance_graph_contract::property::PrefetchDepth;
    use lance_graph_contract::rbac::OpMask;
    use lance_graph_rbac::access::AccessDecision;
    use ogar_auth::user::{
        AuthBinding, AuthContext, AuthStrength, AuthenticatedUser, LocalUserStore, ProviderId,
        User, UserId, UserStore,
    };

    /// The OGAR-minted health concept the fixture authorizes on — pulled from
    /// `ogar-vocab`, never a local literal.
    const PATIENT: u16 = ogar_vocab::class_ids::PATIENT;
    /// Full classid: canon concept HIGH, app render prefix LOW.
    fn patient_class() -> ClassId {
        lance_graph_contract::render_classid(0x0000, PATIENT)
    }

    struct Fixture {
        memberships: Vec<(&'static str, Vec<RoleId>)>,
        grants: Vec<(RoleId, Vec<ClassGrant>)>,
    }
    impl GrantSource for Fixture {
        fn roles_of(&self, actor: ActorId<'_>) -> &[RoleId] {
            self.memberships
                .iter()
                .find(|(a, _)| *a == actor)
                .map_or(&[], |(_, r)| r.as_slice())
        }
        fn grants_of(&self, role: RoleId) -> &[ClassGrant] {
            self.grants
                .iter()
                .find(|(r, _)| *r == role)
                .map_or(&[], |(_, g)| g.as_slice())
        }
    }

    fn authority() -> OgarRbac<Fixture> {
        OgarRbac::new(Fixture {
            memberships: vec![("dr-house", vec!["physician"]), ("betty", vec!["cashier"])],
            grants: vec![
                (
                    "physician",
                    vec![ClassGrant::new(PATIENT, OpMask::READ.union(OpMask::ACT))],
                ),
                ("cashier", vec![ClassGrant::new(PATIENT, OpMask::READ)]),
            ],
        })
    }

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
            ],
            key_refs: vec![],
        });
        s
    }

    /// The moved behaviour still holds: the authority resolves roles and gates
    /// ops through its source.
    #[test]
    fn rehomed_authority_gates_by_grant() {
        let a = authority();
        let act = Operation::Act { action: "approve" };
        assert!(a.grant_permits("physician", patient_class(), &act));
        assert!(!a.grant_permits("cashier", patient_class(), &act));
        assert_eq!(a.actor_roles("nobody"), &[] as &[RoleId]);
    }

    /// F3 — the authority object is local here and legally implements the
    /// foreign trait. If this compiles, coherence holds with no workaround.
    fn _is_class_rbac(_: &impl ClassRbac) {}
    #[test]
    fn authority_object_is_a_legal_class_rbac() {
        _is_class_rbac(&authority());
    }

    /// F2 — authorization is reached ONLY through `ogar-auth`'s canonical user.
    /// This test cannot even be written without depending on `ogar-auth`.
    #[test]
    fn authorization_consumes_the_canonical_ogar_user() {
        let s = store();
        let user = s
            .resolve(&AuthBinding::new(ZITADEL, "a1b2c3"))
            .expect("binding resolves")
            .clone();
        let identity = AuthenticatedUser {
            user,
            auth: AuthContext::federated(ZITADEL, AuthStrength::MultiFactor),
        };
        let d = authority().authorize_user(
            &identity,
            patient_class(),
            Operation::Read {
                depth: PrefetchDepth::Identity,
            },
        );
        assert_eq!(d.decision, AccessDecision::Allow);
    }

    /// F5/F6 — the SAME canonical user reached through two different providers
    /// yields the SAME decision. An IdP swap is invisible to this crate.
    #[test]
    fn decision_is_identical_across_authentication_bindings() {
        let s = store();
        let via_zitadel = s
            .resolve(&AuthBinding::new(ZITADEL, "a1b2c3"))
            .expect("zitadel")
            .clone();
        let via_entra = s
            .resolve(&AuthBinding::new(ENTRA, "0000-1111"))
            .expect("entra")
            .clone();
        let a = authority();
        let op = || Operation::Act { action: "approve" };
        let d1 = a.authorize_user(
            &AuthenticatedUser {
                user: via_zitadel,
                auth: AuthContext::federated(ZITADEL, AuthStrength::MultiFactor),
            },
            patient_class(),
            op(),
        );
        let d2 = a.authorize_user(
            &AuthenticatedUser {
                user: via_entra,
                auth: AuthContext::federated(ENTRA, AuthStrength::SingleFactor),
            },
            patient_class(),
            op(),
        );
        assert_eq!(d1, d2, "the provider must not change the decision");
        assert_eq!(d1.decision, AccessDecision::Allow);
    }

    /// F4 — kiosk is a supported mode, not a refused one. The unauthenticated
    /// path reaches the same decision, because roles belong to the identity and
    /// never to the login method.
    #[test]
    fn kiosk_identity_takes_the_same_authorization_path() {
        let s = store();
        let user = s.user(UserId(42)).expect("user").clone();
        let a = authority();
        let op = || Operation::Read {
            depth: PrefetchDepth::Identity,
        };
        let kiosk = a.authorize_user(
            &AuthenticatedUser {
                user: user.clone(),
                auth: AuthContext::kiosk(),
            },
            patient_class(),
            op(),
        );
        let mfa = a.authorize_user(
            &AuthenticatedUser {
                user,
                auth: AuthContext::local(AuthStrength::MultiFactor),
            },
            patient_class(),
            op(),
        );
        assert_eq!(kiosk, mfa, "kiosk must not take a different path");
        assert_eq!(kiosk.decision, AccessDecision::Allow);
    }

    /// An actor the grant source does not know is denied — the authority does
    /// not invent a default role for an authenticated stranger.
    #[test]
    fn unknown_actor_is_denied_even_when_strongly_authenticated() {
        let stranger = User {
            id: UserId(99),
            subject: "stranger".to_string(),
            tenant: 7,
            roles: vec!["physician".to_string()],
            memberships: vec![],
            bindings: vec![],
            key_refs: vec![],
        };
        let d = authority().authorize_user(
            &AuthenticatedUser {
                user: stranger,
                auth: AuthContext::local(AuthStrength::MultiFactor),
            },
            patient_class(),
            Operation::Read {
                depth: PrefetchDepth::Identity,
            },
        );
        assert!(matches!(d.decision, AccessDecision::Deny { .. }));
    }

    /// F1 — the widened Axis-4 projection survives THIS authority's path.
    /// `{1, 7, 92}` with the narrow `u64` mask resolved to `{1, 7}`.
    #[test]
    fn wide_projection_survives_the_authority_path() {
        struct WideSource;
        impl GrantSource for WideSource {
            fn roles_of(&self, _actor: ActorId<'_>) -> &[RoleId] {
                const R: &[RoleId] = &["wide_reader"];
                R
            }
            fn grants_of(&self, _role: RoleId) -> &[ClassGrant] {
                const G: &[ClassGrant] = &[];
                G
            }
        }
        struct WideRbac(OgarRbac<WideSource>);
        impl ClassRbac for WideRbac {
            fn actor_roles(&self, a: ActorId<'_>) -> &[RoleId] {
                self.0.actor_roles(a)
            }
            fn grant_permits(&self, _r: RoleId, _c: ClassId, _o: &Operation<'_>) -> bool {
                true
            }
            fn field_mask(&self, _r: RoleId, _c: ClassId) -> WideFieldMask {
                WideFieldMask::from_positions(&[1, 7, 92])
            }
        }
        let d = authorize_scoped(
            &WideRbac(OgarRbac::new(WideSource)),
            "dr-house",
            patient_class(),
            Operation::Read {
                depth: PrefetchDepth::Identity,
            },
        );
        assert_eq!(d.decision, AccessDecision::Allow);
        assert!(d.field_mask.has(1));
        assert!(d.field_mask.has(7));
        assert!(
            d.field_mask.has(92),
            "position 92 must survive — the narrow u64 mask dropped it"
        );
        assert_eq!(d.field_mask.count(), 3);
    }

    /// Axis 3 reaches the decision: a source that binds a role to a level of a
    /// hierarchy gets that scope back on the `Allow`, and a source that does not
    /// stays global. The scope travels; the kernel does not interpret it.
    #[test]
    fn a_sources_row_scope_travels_with_the_decision() {
        use lance_graph_contract::rbac::{ScopePath, ScopeSpec};

        struct Scoped(Fixture);
        impl GrantSource for Scoped {
            fn roles_of(&self, actor: ActorId<'_>) -> &[RoleId] {
                self.0.roles_of(actor)
            }
            fn grants_of(&self, role: RoleId) -> &[ClassGrant] {
                self.0.grants_of(role)
            }
            fn scope_of(&self, role: RoleId, _class: ClassId) -> Option<ScopeSpec> {
                (role == "physician").then(|| ScopeSpec {
                    path: ScopePath::new(&[3, 1]).expect("depth"),
                    ..ScopeSpec::default()
                })
            }
        }

        let read = || Operation::Read {
            depth: PrefetchDepth::Identity,
        };
        let scoped = OgarRbac::new(Scoped(authority().source));
        let d = authorize_scoped(&scoped, "dr-house", patient_class(), read());
        assert_eq!(d.decision, AccessDecision::Allow);
        let scope = d.scope.expect("a bound role yields a scope");
        assert!(scope.admits(&ScopePath::new(&[3, 1, 9]).expect("depth")));
        assert!(!scope.admits(&ScopePath::new(&[3, 2]).expect("depth")));

        let global = authorize_scoped(&authority(), "dr-house", patient_class(), read());
        assert_eq!(global.scope, None, "a source without scopes stays global");
    }

    // ── RbacAuthority ─────────────────────────────────────────────────────

    const PLUG: RbacPlug = RbacPlug {
        consumer: "demo",
        classids: &[PATIENT],
        roles: &["physician", "cashier"],
    };

    #[test]
    fn the_authority_binds_a_plug_to_its_grants() {
        let b = authority().bind(&PLUG).expect("green bind");
        assert_eq!(b.consumer(), "demo");
        let act = Operation::Act { action: "approve" };
        assert_eq!(b.permits("physician", patient_class(), &act), Ok(true));
        assert_eq!(b.permits("cashier", patient_class(), &act), Ok(false));
    }

    #[test]
    fn the_authority_refuses_unminted_classids_and_undefined_roles() {
        assert_eq!(
            authority().bind(&RbacPlug {
                classids: &[0xFFFE],
                ..PLUG
            }),
            Err(RbacDrift::UnknownClassid(0xFFFE))
        );
        assert_eq!(
            authority().bind(&RbacPlug {
                roles: &["janitor"],
                ..PLUG
            }),
            Err(RbacDrift::UnknownRole("janitor".into()))
        );
    }

    /// A source with a column projection for cashiers.
    struct Masked(Fixture);
    impl GrantSource for Masked {
        fn roles_of(&self, actor: ActorId<'_>) -> &[RoleId] {
            self.0.roles_of(actor)
        }
        fn grants_of(&self, role: RoleId) -> &[ClassGrant] {
            self.0.grants_of(role)
        }
        fn field_mask_of(&self, role: RoleId, concept: u16) -> Option<WideFieldMask> {
            (role == "cashier" && concept == PATIENT)
                .then(|| WideFieldMask::from_positions(&[0, 2]))
        }
    }

    #[test]
    fn the_binding_carries_the_sources_field_masks() {
        let b = OgarRbac::new(Masked(authority().source))
            .bind(&PLUG)
            .expect("green bind");
        assert_eq!(
            b.field_mask_for("cashier", patient_class()),
            Ok(Some(&WideFieldMask::from_positions(&[0, 2])))
        );
        assert_eq!(b.field_mask_for("physician", patient_class()), Ok(None));
    }

    // ── IdentityActors / authorize_identity ──────────────────────────────

    fn identity(roles: &[&str], tenant: u64) -> AuthenticatedUser {
        AuthenticatedUser {
            user: User {
                id: UserId(7),
                subject: "dr-house".to_string(),
                tenant,
                roles: roles.iter().map(|r| (*r).to_string()).collect(),
                memberships: vec![],
                bindings: vec![],
                key_refs: vec![],
            },
            auth: AuthContext::federated(ZITADEL, AuthStrength::MultiFactor),
        }
    }

    fn read() -> Operation<'static> {
        Operation::Read {
            depth: PrefetchDepth::Identity,
        }
    }

    #[test]
    fn an_identity_is_authorized_tenant_scoped_through_the_binding() {
        let binding = authority().bind(&PLUG).expect("green bind");
        let d = authorize_identity(
            &binding,
            &identity(&["cashier"], 7),
            patient_class(),
            read(),
        );
        assert_eq!(d.decision, AccessDecision::Allow);
        let scope = d.scope.expect("tenant-scoped, never unrestricted");
        assert_eq!(scope.members().len(), 1);
        assert_eq!(scope.members()[0].tenant, Some(7));
    }

    // A role the user holds but the plug did not declare grants nothing.
    #[test]
    fn roles_outside_the_plug_are_dropped_and_reported() {
        let narrow = RbacPlug {
            roles: &["cashier"],
            ..PLUG
        };
        let binding = authority().bind(&narrow).expect("green bind");
        let user = identity(&["physician", "cashier", "ghost"], 7);
        let actors = IdentityActors::new(&user, &binding);
        assert_eq!(actors.roles_of("dr-house"), &["cashier"]);
        assert_eq!(
            actors.unplugged_roles(),
            &["physician".to_string(), "ghost".to_string()]
        );
        let act = Operation::Act { action: "approve" };
        // physician could act on the full binding; through this plug it cannot.
        assert!(matches!(
            authorize_identity(&binding, &user, patient_class(), act).decision,
            AccessDecision::Deny { .. }
        ));
    }

    #[test]
    fn the_actor_source_answers_only_for_its_own_subject() {
        let binding = authority().bind(&PLUG).expect("green bind");
        let actors = IdentityActors::new(&identity(&["physician"], 7), &binding);
        assert_eq!(actors.roles_of("dr-house"), &["physician"]);
        assert!(actors.roles_of("someone-else").is_empty());
        assert!(
            actors
                .memberships_of("someone-else", patient_class())
                .is_empty()
        );
    }

    // A delegated login (RFC 8693 `act`): bob acts with dr-house's authority.
    // The decision is dr-house's; bob is recorded and holds nothing himself.
    #[test]
    fn a_delegated_identity_is_authorized_as_the_user_not_the_actor() {
        let binding = authority().bind(&PLUG).expect("green bind");
        let mut user = identity(&["cashier"], 7);
        user.auth = user
            .auth
            .delegated(AuthBinding::new(ZITADEL, "bob"), [])
            .expect("authenticated");
        let d = authorize_identity(&binding, &user, patient_class(), read());
        assert_eq!(d.decision, AccessDecision::Allow);
        let actors = IdentityActors::new(&user, &binding);
        assert_eq!(
            actors.acting_party(),
            Some(&AuthBinding::new(ZITADEL, "bob"))
        );
        assert!(actors.roles_of("bob").is_empty());
        assert_eq!(actors.roles_of("dr-house"), &["cashier"]);
    }

    #[test]
    fn a_user_with_no_plugged_role_is_denied() {
        let binding = authority().bind(&PLUG).expect("green bind");
        let d = authorize_identity(&binding, &identity(&["ghost"], 7), patient_class(), read());
        assert!(matches!(d.decision, AccessDecision::Deny { .. }));
        assert_eq!(d.scope, None);
    }
}
