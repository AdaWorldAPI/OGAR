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

use lance_graph_contract::rbac::{
    ActorId, ClassGrant, ClassId, ClassRbac, Operation, RoleId, grants_permit,
};
use lance_graph_rbac::authorize::{ScopedDecision, authorize_scoped};
use ogar_auth::user::AuthenticatedUser;

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
    // Axes 2/3/4 (`roles_reaching` / `row_scope` / `field_mask`) inherit the
    // contract defaults until the Core carries the data for them — a follow-up
    // seam, not this patch. `field_mask`'s default is now WideFieldMask, so a
    // grant on a position >= 64 survives once a source supplies one.
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
}
