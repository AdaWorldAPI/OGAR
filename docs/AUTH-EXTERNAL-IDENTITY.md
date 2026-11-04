# External identity and the verified-login boundary (`ogar-auth`)

> Scope: `ogar-auth::{identity, federation}` and the additions to
> `ogar-auth::user`. Provider-agnostic. No token validation lives here.

## 1. The issuer rule

An OIDC `sub` is unique only **within its issuer**. A federated login is
therefore identified by the pair `(iss, sub)` — `identity::ExternalIdentity` —
never by `sub` alone, never by a provider label plus `sub`.

- `Issuer` is compared **byte for byte**: no trailing-slash trim, no case fold.
  `https://idp.example` and `https://idp.example/` are different issuers.
- `Issuer::new` rejects, without rewriting: empty, whitespace/control
  characters, anything not starting with `https://`, nothing after the prefix,
  any `?` or `#`.
- `ExternalSubject::new` rejects only the empty string.

## 2. Uniqueness invariant

`LocalUserStore` guarantees that resolution is never ambiguous:

- `try_insert` refuses a duplicate `UserId`, a duplicate subject, an
  `AuthBinding` already held by another user, and a duplicate binding on the
  same user (`StoreConflict`).
- `bind_external(user, identity)` refuses an unknown user and an identity
  already bound (to another user, or to this one). It never overwrites.
- Each `(iss, sub)` maps to at most one user; `resolve_external` is a map lookup
  on the full pair. The `UserStore` default for `resolve_external` is `None`
  (fail closed).

## 3. No email linking

Email is a display hint (`VerifiedClaims::email`). `establish` never uses it,
and never falls back to an `AuthBinding` or a provider label: an identity with
no `bind_external` entry is `LoginError::Unbound`. Linking a new IdP identity to
an existing user is an explicit `bind_external` call by the deployment.

## 4. Verified-login boundary

`federation::establish(verifier, trusted, input, store)` runs, in order:
verifier → `TrustedIssuers::provider_of(issuer)` (else `UntrustedIssuer`) →
`store.resolve_external(identity)` (else `Unbound`) → `VerifiedLogin`.
The `ProviderId` on the resulting `AuthContext` comes from the registry, not the
claims.

`VerifiedLogin` has private fields and no public constructor/`From`/`Default`;
two `compile_fail` doctests pin that. `AuthContext::federated` and
`AuthenticatedUser` stay **descriptive**: public fields, constructing one proves
nothing.

### What visibility cannot guarantee

- An `AssertionVerifier` implementation in any crate can lie; `establish` trusts
  what it returns.
- Code can hand-build an `AuthenticatedUser` and pass it straight to RBAC.

Both are a **deployment trust boundary**: which verifiers are linked in, and
which code may call RBAC with a hand-built user, is not enforceable by types.

## 5. Compatibility

- New types/methods are additive; existing signatures are unchanged.
- **Behaviour change:** `LocalUserStore::insert` now panics on conflicting input
  (duplicate id/subject/binding). Use `try_insert` for untrusted input.
- Existing provider-scoped federated `AuthBinding`s (`provider` label +
  unscoped subject) **cannot be auto-migrated**: the issuer is not recorded in
  them. Re-bind explicitly with `bind_external`.

## 6. PR-01 handoff

The next piece is a reusable OIDC adapter implementing `AssertionVerifier`
(signature, audience, expiry, JWKS) that returns `VerifiedClaims`. It must stay
provider-agnostic: provider differences are data in `TrustedIssuers`, not match
arms.
