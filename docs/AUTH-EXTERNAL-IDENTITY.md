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

## 6. The OIDC adapter (`ogar-oidc`, PR-01)

`ogar_oidc::OidcVerifier` implements `AssertionVerifier` for compact JWS ID
tokens. It is provider-agnostic: a provider is a `Client` (issuer, client id,
JWKS), never a match arm.

- **No network I/O.** The caller fetches discovery and JWKS and passes the
  bytes (`Discovery::from_json`, `KeySet::from_jwks_json`); key rotation is
  `Client::set_keys`. The clock and the expected nonce come in with the token
  (`IdToken`), so verification is deterministic.
- **Algorithms:** ES256 and EdDSA. `none`, `HS*` and a header with `crit` are
  refused. RS256 is not implemented: no AdaWorldAPI fork of an RSA crate is
  wired yet, so a provider must sign ID tokens with a P-256 or Ed25519 key.
- **Keys are per issuer.** The unverified `iss` only chooses which issuer's keys
  to try; a key registered for one issuer never verifies a token naming
  another.
- **Claims:** `iss` byte-equal and in `TrustedIssuers`; `sub` non-empty; `aud`
  contains the client id; several audiences require `azp`; `azp` when present
  equals the client id; `exp` and `iat` required, `nbf` optional, with a
  leeway (default 60 s); `nonce` equal when the login sent one.
- **Strength:** `MultiFactor` when `amr` contains `mfa` or covers two
  independent factor categories (knowledge, possession, inherence; RFC 8176),
  otherwise `SingleFactor`. `user` (presence) is not a factor.
- **Email** is returned only when `email_verified` is `true`, and stays a
  display hint (section 3).
- Errors (`OidcError`) carry no token contents.
