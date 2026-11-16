//! An in-memory [`CredentialProvider`]: synthetic data only, never production.
//!
//! It exists for tests and offline fixtures. It records every path it is asked
//! for, so a test can prove which paths a broker did and did not touch.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::{CredentialMaterial, CredentialProvider, ProviderError, Redacted, SecretPath};

/// A provider that holds secrets in a map. Synthetic data only; never production.
#[derive(Debug, Default)]
pub struct InMemoryProvider {
    secrets: BTreeMap<(String, u32), String>,
    down: bool,
    sealed: bool,
    calls: RefCell<Vec<String>>,
}

impl InMemoryProvider {
    /// An empty provider.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Store `secret` at `path`, `version`.
    #[must_use]
    pub fn with_secret(mut self, path: &str, version: u32, secret: &str) -> Self {
        self.secrets
            .insert((path.to_string(), version), secret.to_string());
        self
    }

    /// Make every fetch fail as unreachable.
    #[must_use]
    pub fn with_down(mut self, down: bool) -> Self {
        self.down = down;
        self
    }

    /// Make every fetch fail as sealed.
    #[must_use]
    pub fn with_sealed(mut self, sealed: bool) -> Self {
        self.sealed = sealed;
        self
    }

    /// Every path that was fetched, in order (even when the fetch failed).
    #[must_use]
    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl CredentialProvider for InMemoryProvider {
    fn fetch(
        &self,
        path: &SecretPath,
        version: Option<u32>,
    ) -> Result<CredentialMaterial, ProviderError> {
        let p = path.as_str().to_string();
        self.calls.borrow_mut().push(p.clone());
        if self.down {
            return Err(ProviderError::Unreachable);
        }
        if self.sealed {
            return Err(ProviderError::Sealed);
        }
        let mut versions = self
            .secrets
            .range((p.clone(), 0)..=(p.clone(), u32::MAX))
            .peekable();
        if versions.peek().is_none() {
            return Err(ProviderError::NotFound);
        }
        let found = match version {
            Some(v) => self.secrets.get(&(p, v)).map(|s| (v, s)),
            None => versions.last().map(|((_, v), s)| (*v, s)),
        };
        match found {
            Some((version, secret)) => Ok(CredentialMaterial {
                version,
                secret: Redacted::new(secret.clone()),
            }),
            None => Err(ProviderError::VersionNotFound),
        }
    }
}
