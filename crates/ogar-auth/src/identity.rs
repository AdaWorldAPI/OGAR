//! `identity` — the key an external login resolves by: `(issuer, subject)`.
//!
//! An OIDC `sub` is only unique **within its issuer** (OpenID Connect Core
//! §2: the `iss` + `sub` pair is the stable identifier). Two issuers may hand
//! out the same `sub` to two different people. Resolving by `sub` alone — or by
//! a provider *label* plus `sub` — therefore conflates them. This module
//! provides the pair as a value so the store can key on both.
//!
//! # Byte-exact, no normalisation
//!
//! [`Issuer`] is compared **byte for byte**. There is no trailing-slash trim,
//! no case fold, no percent-decoding. `https://idp.example` and
//! `https://idp.example/` are two different issuers. This matches how OIDC
//! discovery requires the `iss` claim to equal the configured issuer string
//! exactly; any "helpful" normalisation here would be a place two distinct
//! issuers could be merged (or one split) without anyone configuring it.
//!
//! Validation ([`Issuer::new`]) only rejects strings that cannot be an issuer
//! identifier; it never rewrites the input.
//!
//! # What this module does NOT do
//!
//! It does not verify tokens, fetch JWKS, or decide which issuers are trusted
//! (see [`crate::federation::TrustedIssuers`]). Constructing an
//! [`ExternalIdentity`] proves nothing: it is a *name*, not a credential.

use core::fmt;

/// Why a string was refused as an [`Issuer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidIssuer {
    /// The string was empty.
    Empty,
    /// The string does not start with `https://`.
    NotHttps,
    /// Nothing follows the `https://` prefix.
    MissingHost,
    /// The string contains whitespace or a control character.
    ForbiddenCharacter,
    /// The string contains `?` or `#` (an issuer has no query or fragment).
    QueryOrFragment,
}

impl fmt::Display for InvalidIssuer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let m = match self {
            Self::Empty => "issuer is empty",
            Self::NotHttps => "issuer must start with https://",
            Self::MissingHost => "issuer has no host after https://",
            Self::ForbiddenCharacter => "issuer contains whitespace or a control character",
            Self::QueryOrFragment => "issuer must not contain a query or fragment",
        };
        f.write_str(m)
    }
}

impl std::error::Error for InvalidIssuer {}

/// An OIDC issuer identifier, held byte-exact.
///
/// The field is private so every value has passed [`Issuer::new`]. Equality,
/// ordering and hashing are those of the underlying bytes: **no trailing-slash
/// trim, no case folding.**
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Issuer(String);

impl Issuer {
    /// Validate and wrap an issuer string without altering it.
    ///
    /// # Errors
    ///
    /// [`InvalidIssuer`] when the string is empty, has whitespace or control
    /// characters, does not start with `https://`, has nothing after that
    /// prefix, or contains `?` or `#`.
    pub fn new(s: &str) -> Result<Self, InvalidIssuer> {
        if s.is_empty() {
            return Err(InvalidIssuer::Empty);
        }
        if s.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(InvalidIssuer::ForbiddenCharacter);
        }
        let Some(rest) = s.strip_prefix("https://") else {
            return Err(InvalidIssuer::NotHttps);
        };
        if rest.is_empty() {
            return Err(InvalidIssuer::MissingHost);
        }
        if s.contains('?') || s.contains('#') {
            return Err(InvalidIssuer::QueryOrFragment);
        }
        Ok(Self(s.to_owned()))
    }

    /// The issuer exactly as supplied.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An issuer-scoped subject (`sub`) — opaque, never parsed.
///
/// Only meaningful together with its [`Issuer`]; see [`ExternalIdentity`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExternalSubject(String);

/// A subject string was empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptySubject;

impl fmt::Display for EmptySubject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("external subject is empty")
    }
}

impl std::error::Error for EmptySubject {}

impl ExternalSubject {
    /// Wrap a subject. Only emptiness is rejected; the bytes are kept as-is.
    ///
    /// # Errors
    ///
    /// [`EmptySubject`] for an empty string.
    pub fn new(s: &str) -> Result<Self, EmptySubject> {
        if s.is_empty() {
            Err(EmptySubject)
        } else {
            Ok(Self(s.to_owned()))
        }
    }

    /// The subject exactly as supplied.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The stable external identity: `(issuer, subject)`.
///
/// This — not a provider label, not an email — is what a federated login is
/// resolved by. Ordering and hashing are over `(issuer, subject)`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExternalIdentity {
    issuer: Issuer,
    subject: ExternalSubject,
}

impl ExternalIdentity {
    /// Pair an issuer with a subject.
    #[must_use]
    pub fn new(issuer: Issuer, subject: ExternalSubject) -> Self {
        Self { issuer, subject }
    }

    /// The issuer half.
    #[must_use]
    pub fn issuer(&self) -> &Issuer {
        &self.issuer
    }

    /// The subject half.
    #[must_use]
    pub fn subject(&self) -> &ExternalSubject {
        &self.subject
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iss(s: &str) -> Issuer {
        Issuer::new(s).expect("valid issuer")
    }

    #[test]
    fn trailing_slash_issuer_is_a_different_issuer() {
        assert_ne!(iss("https://idp.example"), iss("https://idp.example/"));
        assert_eq!(iss("https://idp.example/").as_str(), "https://idp.example/");
    }

    #[test]
    fn case_is_not_folded() {
        assert_ne!(iss("https://IdP.example"), iss("https://idp.example"));
    }

    #[test]
    fn byte_identical_issuers_and_identities_match() {
        let a = ExternalIdentity::new(
            iss("https://idp.example/realm"),
            ExternalSubject::new("u-1").unwrap(),
        );
        let b = ExternalIdentity::new(
            iss("https://idp.example/realm"),
            ExternalSubject::new("u-1").unwrap(),
        );
        assert_eq!(a, b);
        let c = ExternalIdentity::new(
            iss("https://idp.example/realm/"),
            ExternalSubject::new("u-1").unwrap(),
        );
        assert_ne!(a, c, "same sub under a different issuer string");
    }

    #[test]
    fn invalid_issuers_are_rejected() {
        assert_eq!(Issuer::new(""), Err(InvalidIssuer::Empty));
        assert_eq!(
            Issuer::new("http://idp.example"),
            Err(InvalidIssuer::NotHttps)
        );
        assert_eq!(Issuer::new("idp.example"), Err(InvalidIssuer::NotHttps));
        assert_eq!(Issuer::new("https://"), Err(InvalidIssuer::MissingHost));
        assert_eq!(
            Issuer::new("https://idp.example/a b"),
            Err(InvalidIssuer::ForbiddenCharacter)
        );
        assert_eq!(
            Issuer::new("https://idp.example/\n"),
            Err(InvalidIssuer::ForbiddenCharacter)
        );
        assert_eq!(
            Issuer::new("https://idp.example?x=1"),
            Err(InvalidIssuer::QueryOrFragment)
        );
        assert_eq!(
            Issuer::new("https://idp.example#frag"),
            Err(InvalidIssuer::QueryOrFragment)
        );
        // stay-silent: a path-bearing https issuer is fine.
        assert!(Issuer::new("https://idp.example/tenants/a").is_ok());
    }

    #[test]
    fn empty_subject_is_rejected_anything_else_kept() {
        assert_eq!(ExternalSubject::new(""), Err(EmptySubject));
        assert_eq!(ExternalSubject::new(" x ").unwrap().as_str(), " x ");
    }
}
