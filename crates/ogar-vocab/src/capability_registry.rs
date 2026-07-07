//! Generic consumer-registration roundtrip for AUTHORITATIVE capability
//! tables (operator, 2026-07-07).
//!
//! OGAR is the authoritative store: a domain table (e.g.
//! [`crate::ocr_actions`], the planned thinking-styles best-practice table)
//! declares WHAT exists and WHO is expected to execute it. The consumer
//! "trägt sich ein" by exporting a [`CapabilityRegistration`] const and
//! asserting [`verify_registration`] in its own tests — so the whole loop
//! (table declared → consumer registered → classids activated) is checked
//! in the one binary, at test time, with zero serialization and zero
//! ontology payload outside OGAR. lance-graph carries NOTHING of this: its
//! armed contract fuse only guards the wire mirror; the authority checks
//! (unique mint, correct classid activation, executor coverage) live HERE.
//!
//! Drift bangs once: a capability added to the table without a consumer arm,
//! a consumer covering a capability the table dropped, a wrong/missing
//! classid activation, or an unregistered executor name — each is a failed
//! [`verify_registration`] in the consumer's test suite.

/// A consumer's self-registration against one authoritative table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityRegistration {
    /// Consumer crate name — must be one of the table's expected executors.
    pub consumer: &'static str,
    /// Capability names the consumer's executor covers (its dispatch arms).
    pub covered: &'static [&'static str],
    /// The subject classids the consumer activated (canon-high concept ids
    /// from [`crate::class_ids`]) — must equal the table's subject set.
    pub subject_classids: &'static [u16],
}

/// Why a registration failed the roundtrip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistrationDrift {
    /// Consumer name is not in the table's expected-executor list.
    UnexpectedConsumer(String),
    /// Declared capability with no consumer arm.
    Uncovered(String),
    /// Consumer arm for a capability the table does not declare.
    Undeclared(String),
    /// Consumer's activated classid set differs from the table's subjects.
    ClassidMismatch {
        /// Subject ids the table declares.
        declared: Vec<u16>,
        /// Ids the consumer activated.
        activated: Vec<u16>,
    },
    /// A subject classid is not minted in [`crate::class_ids::ALL`].
    Unminted(u16),
}

/// Verify one consumer registration against an authoritative table's
/// capability names, subject classids and expected-executor list.
///
/// Checks, in order: consumer expected; coverage both directions
/// (declared ⊆ covered AND covered ⊆ declared); activated classid set ==
/// declared subject set; every id minted (unique mint is guarded by the
/// codebook's own `no_duplicate_ids` test — a concept is assigned once,
/// here we only require it exists).
pub fn verify_registration(
    reg: &CapabilityRegistration,
    declared_capabilities: &[&str],
    declared_subjects: &[u16],
    expected_executors: &[&str],
) -> Result<(), RegistrationDrift> {
    if !expected_executors.contains(&reg.consumer) {
        return Err(RegistrationDrift::UnexpectedConsumer(reg.consumer.into()));
    }
    for cap in declared_capabilities {
        if !reg.covered.contains(cap) {
            return Err(RegistrationDrift::Uncovered((*cap).into()));
        }
    }
    for cap in reg.covered {
        if !declared_capabilities.contains(cap) {
            return Err(RegistrationDrift::Undeclared((*cap).into()));
        }
    }
    let mut declared: Vec<u16> = declared_subjects.to_vec();
    declared.sort_unstable();
    declared.dedup();
    let mut activated: Vec<u16> = reg.subject_classids.to_vec();
    activated.sort_unstable();
    activated.dedup();
    if declared != activated {
        return Err(RegistrationDrift::ClassidMismatch {
            declared,
            activated,
        });
    }
    for &id in &activated {
        if !crate::class_ids::ALL.iter().any(|&(_, cid)| cid == id) {
            return Err(RegistrationDrift::Unminted(id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED: &[&str] = &["demo-executor"];
    const CAPS: &[&str] = &["a", "b"];

    fn reg(covered: &'static [&'static str], ids: &'static [u16]) -> CapabilityRegistration {
        CapabilityRegistration {
            consumer: "demo-executor",
            covered,
            subject_classids: ids,
        }
    }

    #[test]
    fn green_roundtrip() {
        // 0x0805 textline is a real mint.
        assert_eq!(
            verify_registration(&reg(&["a", "b"], &[0x0805]), CAPS, &[0x0805], EXPECTED),
            Ok(())
        );
    }

    #[test]
    fn each_drift_arm_bangs() {
        let mut r = reg(&["a", "b"], &[0x0805]);
        r.consumer = "stranger";
        assert!(matches!(
            verify_registration(&r, CAPS, &[0x0805], EXPECTED),
            Err(RegistrationDrift::UnexpectedConsumer(_))
        ));
        assert!(matches!(
            verify_registration(&reg(&["a"], &[0x0805]), CAPS, &[0x0805], EXPECTED),
            Err(RegistrationDrift::Uncovered(_))
        ));
        assert!(matches!(
            verify_registration(&reg(&["a", "b", "c"], &[0x0805]), CAPS, &[0x0805], EXPECTED),
            Err(RegistrationDrift::Undeclared(_))
        ));
        assert!(matches!(
            verify_registration(&reg(&["a", "b"], &[0x0806]), CAPS, &[0x0805], EXPECTED),
            Err(RegistrationDrift::ClassidMismatch { .. })
        ));
        assert!(matches!(
            verify_registration(&reg(&["a", "b"], &[0xFFFE]), CAPS, &[0xFFFE], EXPECTED),
            Err(RegistrationDrift::Unminted(0xFFFE))
        ));
    }
}
