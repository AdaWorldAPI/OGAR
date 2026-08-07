//! The vocabulary registry — plug-and-play routing from classid to table.
//!
//! # Plugging a USB stick (the operator frame)
//!
//! A vocabulary crate is a *device*; this registry is the *hub*. Plugging in
//! is the USB handshake, made typed:
//!
//! 1. **Enumeration = validation.** [`plug`](VocabularyRegistry::plug) only
//!    accepts a [`CheckedVocabulary`] — a device that has not passed
//!    [`conformance::validate`](crate::vocabulary::conformance::validate)
//!    cannot even reach the port. There is no "register now, validate later"
//!    path, so the hub never routes to an unproven table.
//! 2. **The descriptor = the classid.** The node's classid (canon-high:
//!    concept id in the hi u16, app render prefix in the lo u16 —
//!    `D-CLASSID-CANON-HIGH-FLIP`) is what a consumer reads off a stored
//!    node to pick the vocabulary. Same routing move as
//!    `ogar_vocab::canonical_concept_domain` — hi-byte prefix dispatch —
//!    one level down, resolving to a *semantic table* instead of a domain
//!    tag.
//! 3. **What is stored is data, not a driver.** Registration copies the
//!    composed [`VocabularyTable`] (the R4 data-first artifact) — no trait
//!    objects, no lifetimes back into the vocabulary crate, no generics in
//!    the registry type. Unplugging the crate that registered it could not
//!    invalidate the table: it is 256 [`FnSpec`](crate::FnSpec) rows of
//!    plain `Copy` data.
//!
//! # Why the key stays opaque
//!
//! [`FunctionNode::key`](crate::FunctionNode::key) is deliberately
//! uninterpreted by this crate (see `node.rs`) — minting and key layout are
//! the canon's business. The registry therefore takes the **classid** (or
//! bare concept id), which the caller extracts from wherever its keys carry
//! it. The one canon fact this module does encode is the half-order:
//! concept HIGH, app prefix LOW.
//!
//! # Who plugs in
//!
//! Each vocabulary crate ships a `plug_into(&mut registry)` helper —
//! `blockly-rs` plugs the Blockly palette (`0x1717`), `ogar-ro` plugs the
//! relation-body concept — and a consumer (blockly-rs, lance-graph) builds
//! ONE registry at boot from the crates it deps, then resolves every stored
//! function node through it. Two frontends, one hub, no hardcoded "this
//! node must be Blockly."

use crate::vocabulary::conformance::CheckedVocabulary;
use crate::{Vocabulary, VocabularyTable};

/// The hub: concept id → validated [`VocabularyTable`].
///
/// Deliberately small and boring — a sorted-on-demand `Vec` of pairs, not a
/// hash map, because a workspace plugs in a handful of vocabularies (three
/// today), and a `Vec` keeps this module dependency-free and `const`-friendly.
#[derive(Debug, Clone, Default)]
pub struct VocabularyRegistry {
    entries: Vec<(u16, VocabularyTable)>,
}

/// Why a plug was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryError {
    /// The concept id already routes to a table. Two vocabularies claiming
    /// one concept would make resolution order-dependent — refused loudly,
    /// never last-write-wins.
    ConceptTaken {
        /// The contested concept id.
        concept_id: u16,
    },
}

impl core::fmt::Display for RegistryError {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RegistryError::ConceptTaken { concept_id } => write!(
                fmt,
                "concept {concept_id:#06x} already has a registered vocabulary"
            ),
        }
    }
}

impl core::error::Error for RegistryError {}

impl VocabularyRegistry {
    /// An empty hub.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Plug a validated vocabulary in under a concept id.
    ///
    /// Only a [`CheckedVocabulary`] is accepted — validation IS the
    /// enumeration handshake. The composed table is copied out; the
    /// wrapper (and the crate that built it) owes the registry nothing
    /// afterward.
    ///
    /// # Errors
    ///
    /// [`RegistryError::ConceptTaken`] if the concept id is already routed.
    pub fn plug<V: Vocabulary>(
        &mut self,
        concept_id: u16,
        v: &CheckedVocabulary<V>,
    ) -> Result<(), RegistryError> {
        if self.resolve_concept(concept_id).is_some() {
            return Err(RegistryError::ConceptTaken { concept_id });
        }
        self.entries.push((concept_id, *v.table()));
        Ok(())
    }

    /// The table registered for a bare concept id, if any.
    #[must_use]
    pub fn resolve_concept(&self, concept_id: u16) -> Option<&VocabularyTable> {
        self.entries
            .iter()
            .find(|(id, _)| *id == concept_id)
            .map(|(_, t)| t)
    }

    /// The table a full V3 classid routes to — canon-high: the concept id is
    /// the classid's **hi u16**, the lo u16 is the app render prefix and
    /// does not participate in vocabulary routing (skins differ per app;
    /// semantics do not).
    #[must_use]
    pub fn resolve_classid(&self, classid: u32) -> Option<&VocabularyTable> {
        self.resolve_concept((classid >> 16) as u16)
    }

    /// Every plugged concept id, in plug order — the legend hook (which
    /// devices are on the hub).
    pub fn concepts(&self) -> impl Iterator<Item = u16> + '_ {
        self.entries.iter().map(|(id, _)| *id)
    }

    /// How many vocabularies are plugged in.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the hub is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FnIndex;
    use crate::vocabulary::conformance::validate;

    struct EmptyVocab;
    impl Vocabulary for EmptyVocab {
        fn domain_stack_arity(&self, _f: FnIndex) -> Option<u8> {
            None
        }
        fn domain_body_refs(&self, _f: FnIndex) -> u8 {
            0
        }
    }

    struct NamedVocab;
    impl Vocabulary for NamedVocab {
        fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
            (f.0 == 0x90).then_some(0)
        }
        fn domain_body_refs(&self, _f: FnIndex) -> u8 {
            0
        }
        fn domain_name(&self, f: FnIndex) -> Option<&'static str> {
            (f.0 == 0x90).then_some("named_verb")
        }
    }

    #[test]
    fn a_classid_routes_by_its_hi_u16_concept_and_ignores_the_app_prefix() {
        let mut hub = VocabularyRegistry::new();
        hub.plug(0x1701, &validate(NamedVocab).unwrap()).unwrap();
        // Two DIFFERENT app prefixes, same concept: one vocabulary. The skin
        // differs per app; the semantics must not.
        let a = hub.resolve_classid(0x1701_1000).unwrap();
        let b = hub.resolve_classid(0x1701_BEEF).unwrap();
        assert_eq!(a.name(FnIndex(0x90)), Some("named_verb"));
        assert_eq!(a, b);
        // …and a concept nobody plugged resolves to nothing, not a guess.
        assert_eq!(hub.resolve_classid(0x0306_1000), None);
    }

    #[test]
    fn two_vocabularies_on_one_hub_stay_distinct() {
        let mut hub = VocabularyRegistry::new();
        hub.plug(0x1701, &validate(EmptyVocab).unwrap()).unwrap();
        hub.plug(0x0306, &validate(NamedVocab).unwrap()).unwrap();
        assert_eq!(hub.len(), 2);
        // The empty vocabulary refuses 0x90; the named one covers it —
        // resolution by concept keeps them apart.
        assert_eq!(
            hub.resolve_concept(0x1701)
                .unwrap()
                .stack_arity(FnIndex(0x90)),
            None
        );
        assert_eq!(
            hub.resolve_concept(0x0306)
                .unwrap()
                .stack_arity(FnIndex(0x90)),
            Some(0)
        );
        // Both answer the shared core identically — the floor discipline
        // survives registration.
        assert_eq!(
            hub.resolve_concept(0x1701)
                .unwrap()
                .stack_arity(FnIndex::ADD),
            hub.resolve_concept(0x0306)
                .unwrap()
                .stack_arity(FnIndex::ADD),
        );
    }

    #[test]
    fn a_contested_concept_is_refused_not_overwritten() {
        let mut hub = VocabularyRegistry::new();
        hub.plug(0x1701, &validate(NamedVocab).unwrap()).unwrap();
        assert_eq!(
            hub.plug(0x1701, &validate(EmptyVocab).unwrap()),
            Err(RegistryError::ConceptTaken { concept_id: 0x1701 })
        );
        // The FIRST device keeps the port: still the named table.
        assert_eq!(
            hub.resolve_concept(0x1701).unwrap().name(FnIndex(0x90)),
            Some("named_verb")
        );
    }
}
