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

// ─── Generic hot-plug resolution (operator, 2026-07-07) ─────────────────────
//
// The migration target for EVERY consumer: the consumer declares WHICH
// classids it hot-plugs (plus the capabilities its executor covers), and the
// authority resolves BOTH the vocab rows and the action surface for exactly
// those ids — one generic path, no per-consumer plug crate, no per-domain
// verify function. Domain tables register in [`domain_tables`]; the planned
// thinking-styles best-practice table appends itself there and is instantly
// hot-pluggable.

/// One authoritative domain action table, registered for generic resolution.
pub struct DomainTable {
    /// Domain label (diagnostics only).
    pub domain: &'static str,
    /// Executors the authority expects for this table.
    pub expected_executors: &'static [&'static str],
    /// `(capability, subject classid)` rows — the join surface.
    pub entries: fn() -> Vec<(String, u16)>,
}

/// Every registered authoritative domain table. Append-only: a new domain
/// (thinking styles, …) adds one entry and is immediately resolvable.
pub fn domain_tables() -> Vec<DomainTable> {
    vec![DomainTable {
        domain: "ocr",
        expected_executors: crate::ocr_actions::OCR_EXPECTED_EXECUTORS,
        entries: ocr_entries,
    }]
}

/// Derive a domain's `(capability, subject classid)` join rows GENERICALLY
/// from any `[ActionDef]` — the output of `ogar-from-ruff::lift_actions`
/// for ANY language frontend (Ruby/Rails, Python/Odoo, C#/Roslyn,
/// C++/libclang). This is the **"config becomes data" seam**: a consumer
/// registers a domain by handing over its harvested `ActionDef`s, never by
/// hand-writing a bespoke `entries` closure. The first, hand-authored
/// table ([`crate::ocr_actions`]) routes through here too — so the
/// hand-authored and harvested paths cannot diverge on what a row *is*,
/// and every future consumer (medcare healthcare, thinking-styles, …)
/// reuses one agnostic derive instead of copying this logic.
///
/// Each action's subject classid is resolved from its
/// [`crate::ActionDef::object_class`] (`<prefix>/<concept>` — the concept
/// is the last `/`-segment, so any prefix works: `ogit-ocr/…`,
/// `medcare:…/…`, …) via [`crate::canonical_concept_id`]. An action whose
/// concept is unminted maps to id `0` (the default-class sentinel) rather
/// than being silently dropped: the row survives so the caller's
/// [`resolve_hotplug`] / [`verify_registration`] fuse catches it as
/// `UnknownClassid` / `Unminted`, keeping drift audible.
///
/// Effect-fact fields on the `ActionDef` (`reads`/`writes`/`calls`/
/// `raises`) are irrelevant to the join — the hot-plug keys on the
/// capability NAME (`predicate`) and the subject classid alone. So a
/// name-only `ActionDef` (e.g. a C#-Roslyn lift before the harvester's
/// method-body walk lands) derives a valid row; the effect facts are
/// enrichment for projection/kausal/RBAC, not join inputs.
#[must_use]
pub fn entries_from_actions(actions: &[crate::ActionDef]) -> Vec<(String, u16)> {
    actions
        .iter()
        .map(|def| {
            let concept = def.object_class.rsplit('/').next().unwrap_or_default();
            let id = crate::canonical_concept_id(concept).unwrap_or_default();
            (def.predicate.clone(), id)
        })
        .collect()
}

/// OCR domain rows, derived through the generic [`entries_from_actions`]
/// path — the hand-authored [`crate::ocr_actions`] table is just one data
/// source feeding the agnostic derive.
fn ocr_entries() -> Vec<(String, u16)> {
    let defs: Vec<crate::ActionDef> = crate::ocr_actions::ocr_actions()
        .into_iter()
        .map(|spec| spec.def)
        .collect();
    entries_from_actions(&defs)
}

/// A green hot-plug resolution: the `(concept, classid)` vocab rows and the
/// sorted capability names for exactly the hot-plugged ids.
pub type HotplugActivation = (Vec<(&'static str, u16)>, Vec<String>);

/// Resolve a consumer hot-plug: verify and return `(vocab rows,
/// capability names)` for exactly the hot-plugged classids.
///
/// Checks, in order: every id minted ([`HotplugDrift::UnknownClassid`]);
/// every id contributes at least one capability
/// ([`HotplugDrift::NoCapabilitiesFor`] — "the table was forgotten");
/// the consumer is expected by every contributing table
/// ([`HotplugDrift::UnexpectedConsumer`]); coverage both directions
/// ([`HotplugDrift::Uncovered`] / [`HotplugDrift::Undeclared`]).
pub fn resolve_hotplug(
    consumer: &str,
    classids: &[u16],
    covered: &[&str],
) -> Result<HotplugActivation, HotplugDrift> {
    let mut concepts = Vec::new();
    for &id in classids {
        match crate::class_ids::ALL.iter().find(|&&(_, cid)| cid == id) {
            Some(&(name, cid)) => concepts.push((name, cid)),
            None => return Err(HotplugDrift::UnknownClassid(id)),
        }
    }
    let mut capabilities: Vec<String> = Vec::new();
    let mut contributed: std::collections::BTreeMap<u16, usize> =
        classids.iter().map(|&id| (id, 0)).collect();
    for table in domain_tables() {
        let mut table_contributes = false;
        for (cap, subject) in (table.entries)() {
            if let Some(n) = contributed.get_mut(&subject) {
                *n += 1;
                table_contributes = true;
                capabilities.push(cap);
            }
        }
        if table_contributes && !table.expected_executors.contains(&consumer) {
            return Err(HotplugDrift::UnexpectedConsumer(consumer.into()));
        }
    }
    if let Some((&id, _)) = contributed.iter().find(|&(_, &n)| n == 0) {
        return Err(HotplugDrift::NoCapabilitiesFor(id));
    }
    capabilities.sort_unstable();
    capabilities.dedup();
    for cap in &capabilities {
        if !covered.contains(&cap.as_str()) {
            return Err(HotplugDrift::Uncovered(cap.clone()));
        }
    }
    for cap in covered {
        if !capabilities.iter().any(|c| c == cap) {
            return Err(HotplugDrift::Undeclared((*cap).into()));
        }
    }
    Ok((concepts, capabilities))
}

/// Why a hot-plug resolution failed — the generic mirror of the
/// per-registration [`RegistrationDrift`], keyed by classid join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotplugDrift {
    /// A hot-plugged classid is not minted in the codebook.
    UnknownClassid(u16),
    /// A hot-plugged classid resolves to no declared capability.
    NoCapabilitiesFor(u16),
    /// The consumer is not expected by a contributing table.
    UnexpectedConsumer(String),
    /// Declared capability without a consumer arm.
    Uncovered(String),
    /// Consumer arm without a declared capability.
    Undeclared(String),
}

#[cfg(test)]
mod hotplug_tests {
    use super::*;

    const OCR_IDS: &[u16] = &[0x0805, 0x0808, 0x0809];
    const OCR_COVERED: &[&str] = &[
        "extract_page_image",
        "extract_text_layer",
        "recognize_line",
        "recognize_page",
        "render_hocr",
        "render_searchable_pdf",
        "render_text",
        "render_tsv",
    ];

    #[test]
    fn ocr_hotplug_resolves_vocab_and_actions() {
        let (concepts, caps) =
            resolve_hotplug("tesseract-ogar", OCR_IDS, OCR_COVERED).expect("green");
        assert_eq!(concepts.len(), 3);
        assert!(concepts.contains(&("textline", 0x0805)));
        assert_eq!(caps.len(), 8);
    }

    #[test]
    fn each_drift_arm_bangs() {
        assert!(matches!(
            resolve_hotplug("tesseract-ogar", &[0xFFFE], &[]),
            Err(HotplugDrift::UnknownClassid(0xFFFE))
        ));
        // patient (0x0901) is minted but has no action table yet.
        assert!(matches!(
            resolve_hotplug("tesseract-ogar", &[0x0901], &[]),
            Err(HotplugDrift::NoCapabilitiesFor(0x0901))
        ));
        assert!(matches!(
            resolve_hotplug("stranger", OCR_IDS, OCR_COVERED),
            Err(HotplugDrift::UnexpectedConsumer(_))
        ));
        let mut missing = OCR_COVERED.to_vec();
        missing.pop();
        assert!(matches!(
            resolve_hotplug("tesseract-ogar", OCR_IDS, &missing),
            Err(HotplugDrift::Uncovered(_))
        ));
        let mut extra = OCR_COVERED.to_vec();
        extra.push("does_not_exist");
        assert!(matches!(
            resolve_hotplug("tesseract-ogar", OCR_IDS, &extra),
            Err(HotplugDrift::Undeclared(_))
        ));
    }

    /// The generic derive is prefix-agnostic: it keys on the last
    /// `/`-segment of `object_class`, so an `ActionDef` lifted from ANY
    /// frontend's namespace (not just `ogit-ocr/…`) derives a valid row.
    /// This is the "config becomes data" guarantee — a harvested consumer
    /// (medcare-via-Roslyn, a future thinking-styles table) reuses this one
    /// derive instead of copying the OCR logic.
    #[test]
    fn entries_from_actions_is_prefix_agnostic() {
        use crate::ActionDef;
        // Non-OCR prefix, concept `textline` (a known mint). The derive
        // must resolve it exactly as the OCR path does — prefix ignored.
        let a = ActionDef::new(
            "some-app:health/textline::action_def::foo",
            "foo",
            "some-app:health/textline",
        );
        assert_eq!(
            entries_from_actions(&[a]),
            vec![("foo".to_string(), crate::class_ids::TEXTLINE)],
        );
    }

    /// A name-only `ActionDef` (empty effect facts — e.g. a C# lift before
    /// the Roslyn harvester's method-body walk lands) still derives a valid
    /// row: the join keys on `predicate` + subject classid, never the body.
    #[test]
    fn entries_from_actions_ignores_effect_facts() {
        use crate::ActionDef;
        let mut a = ActionDef::new("x", "recognize_line", "ogit-ocr/textline");
        assert!(a.reads.is_empty() && a.writes.is_empty());
        a.reads = vec!["grey_line".into()]; // enrichment present or absent…
        let empty = ActionDef::new("y", "recognize_line", "ogit-ocr/textline");
        // …yields the identical join row either way.
        assert_eq!(entries_from_actions(&[a]), entries_from_actions(&[empty]));
    }

    /// An unminted concept survives as id 0 (the default-class sentinel)
    /// rather than being silently dropped — so `resolve_hotplug`'s
    /// `UnknownClassid` / `verify_registration`'s `Unminted` fuse still
    /// fires. The derive never hides a bad row.
    #[test]
    fn entries_from_actions_keeps_unminted_rows_for_the_fuse() {
        use crate::ActionDef;
        let a = ActionDef::new("z", "bar", "any:ns/totally_not_a_concept");
        assert_eq!(entries_from_actions(&[a]), vec![("bar".to_string(), 0)]);
    }

    /// The hand-authored OCR table, routed through the generic derive, is
    /// byte-for-byte what it was before the refactor — the "config becomes
    /// data" seam subsumes the bespoke path with zero behavior change.
    #[test]
    fn ocr_entries_still_derive_the_eight_known_rows() {
        let ocr = ocr_entries();
        assert_eq!(ocr.len(), 8);
        assert!(
            ocr.iter()
                .any(|(cap, id)| cap == "recognize_line" && *id == crate::class_ids::TEXTLINE)
        );
        assert!(
            ocr.iter()
                .any(|(cap, id)| cap == "render_hocr" && *id == crate::class_ids::OCR_RENDERER)
        );
    }
}
