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
    vec![
        DomainTable {
            domain: "ocr",
            expected_executors: crate::ocr_actions::OCR_EXPECTED_EXECUTORS,
            entries: ocr_entries,
        },
        DomainTable {
            domain: "geo",
            expected_executors: crate::geo_actions::GEO_EXPECTED_EXECUTORS,
            entries: geo_entries,
        },
        DomainTable {
            domain: "healthcare",
            expected_executors: crate::healthcare_actions::HEALTHCARE_EXPECTED_EXECUTORS,
            entries: healthcare_entries,
        },
        DomainTable {
            domain: "blocks",
            expected_executors: crate::blocks_actions::BLOCKS_EXPECTED_EXECUTORS,
            entries: blocks_entries,
        },
    ]
}

/// One slag-ledger row: an ActionDef whose subject concept the codebook
/// could not mint. NOT waste — the empirical boundary of the current
/// codebook/convention: each recurring residual names the next config fact
/// (a mint, an alias row, or a convention fix). (MedCare transcode
/// doctrine: "study the slag, teach the furnace".)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmintedRow {
    /// The capability name (ActionDef.predicate).
    pub capability: String,
    /// The full object_class the def carried.
    pub object_class: String,
    /// The concept segment that failed to resolve (last '/'-segment).
    pub concept: String,
}

/// One row of the single-pass action walk shared by
/// [`entries_from_actions`] and [`entries_from_actions_with_residuals`] —
/// private so both public functions project the SAME walk instead of
/// re-deriving it and risking drift between them.
enum ActionRow {
    /// A resolved `(capability, subject classid)` join row.
    Resolved(String, u16),
    /// A row whose subject concept the codebook could not mint.
    Unminted(UnmintedRow),
}

/// Walk `actions` exactly once, in order, resolving each subject classid
/// from [`crate::ActionDef::object_class`] (`<prefix>/<concept>` — the
/// concept is the last `/`-segment, so any prefix works: `ogit-ocr/…`,
/// `medcare:…/…`, …) via [`crate::canonical_concept_id`]. Effect-fact
/// fields on the `ActionDef` (`reads`/`writes`/`calls`/`raises`) are
/// irrelevant to the join — the hot-plug keys on the capability NAME
/// (`predicate`) and the subject classid alone, so a name-only `ActionDef`
/// (e.g. a C#-Roslyn lift before the harvester's method-body walk lands)
/// derives a valid row.
fn derive_action_rows(actions: &[crate::ActionDef]) -> Vec<ActionRow> {
    actions
        .iter()
        .map(|def| {
            let concept = def.object_class.rsplit('/').next().unwrap_or_default();
            match crate::canonical_concept_id(concept) {
                Some(id) => ActionRow::Resolved(def.predicate.clone(), id),
                None => ActionRow::Unminted(UnmintedRow {
                    capability: def.predicate.clone(),
                    object_class: def.object_class.clone(),
                    concept: concept.to_string(),
                }),
            }
        })
        .collect()
}

/// Residual-aware twin of [`entries_from_actions`]: resolved rows carry
/// their nonzero classid; unresolvable concepts land in the slag ledger
/// instead of degrading to the id-0 sentinel. New consumers (the
/// transcode-doctrine Phase 6 exam) use THIS; the id-0 sentinel path
/// below stays for existing fuse consumers (I-LEGACY-API-FEATURE-GATED:
/// same name keeps same semantics; new semantics get a new name).
#[must_use]
pub fn entries_from_actions_with_residuals(
    actions: &[crate::ActionDef],
) -> (Vec<(String, u16)>, Vec<UnmintedRow>) {
    let mut resolved = Vec::new();
    let mut residuals = Vec::new();
    for row in derive_action_rows(actions) {
        match row {
            ActionRow::Resolved(capability, id) => resolved.push((capability, id)),
            ActionRow::Unminted(residual) => residuals.push(residual),
        }
    }
    (resolved, residuals)
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
///
/// **LEGACY id-0-sentinel projection.** This function is now the id-0
/// projection of [`entries_from_actions_with_residuals`]: same single
/// walk ([`derive_action_rows`]), same original order, but an unminted
/// concept folds to `(capability, 0)` here instead of surfacing as an
/// [`UnmintedRow`] in a slag ledger. Existing fuse consumers
/// (`resolve_hotplug` / `verify_registration`) key on the id-0 sentinel
/// and keep calling this function; new consumers wanting the residual
/// ledger (no unwrap-or-default concept-id fallback — doctrine Phase 6)
/// should call `entries_from_actions_with_residuals` directly.
#[must_use]
pub fn entries_from_actions(actions: &[crate::ActionDef]) -> Vec<(String, u16)> {
    derive_action_rows(actions)
        .into_iter()
        .map(|row| match row {
            ActionRow::Resolved(capability, id) => (capability, id),
            ActionRow::Unminted(residual) => (residual.capability, 0),
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

/// Geo domain rows ([`crate::geo_actions`], the osm-soa-bake table), derived
/// through the same generic [`entries_from_actions`] path as OCR.
fn geo_entries() -> Vec<(String, u16)> {
    entries_from_actions(&crate::geo_actions::geo_actions())
}

/// Blocks domain rows ([`crate::blocks_actions`], the blockly-abi table),
/// derived through the same generic [`entries_from_actions`] path as OCR.
fn blocks_entries() -> Vec<(String, u16)> {
    entries_from_actions(&crate::blocks_actions::blocks_actions())
}

/// Healthcare domain rows ([`crate::healthcare_actions`], the medcare-rs
/// table — parity-plan P3), derived through the same generic
/// [`entries_from_actions`] path as OCR.
fn healthcare_entries() -> Vec<(String, u16)> {
    entries_from_actions(&crate::healthcare_actions::healthcare_actions())
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

    // LIVE references (not hand-mirrored) so these can never drift from the
    // authoritative table — the v2 growth (8→14 caps, +PAGE_LAYOUT subject,
    // 2026-07-10) flows in automatically. `resolve_hotplug` does SET
    // comparison, so the slice order is irrelevant.
    const OCR_IDS: &[u16] = crate::ocr_actions::OCR_SUBJECT_CLASSIDS;
    const OCR_COVERED: &[&str] = crate::ocr_actions::OCR_ACTION_NAMES;

    #[test]
    fn ocr_hotplug_resolves_vocab_and_actions() {
        let (concepts, caps) =
            resolve_hotplug("tesseract-ogar", OCR_IDS, OCR_COVERED).expect("green");
        assert_eq!(
            concepts.len(),
            OCR_IDS.len(),
            "one concept per subject classid"
        );
        assert!(concepts.contains(&("textline", 0x0805)));
        assert!(concepts.contains(&("page_layout", 0x0807)));
        assert_eq!(caps.len(), crate::ocr_actions::OCR_ACTION_NAMES.len());
    }

    /// The parity-plan P3 probe: `resolve_hotplug("medcare-rs",
    /// HEALTHCARE_SUBJECT_CLASSIDS, covered)` GREEN (KILL condition was
    /// `NoCapabilitiesFor`/`Uncovered`). The Health classids that banged
    /// `NoCapabilitiesFor` before this table landed now resolve.
    #[test]
    fn healthcare_hotplug_resolves_vocab_and_actions() {
        let (concepts, caps) = resolve_hotplug(
            "medcare-rs",
            crate::healthcare_actions::HEALTHCARE_SUBJECT_CLASSIDS,
            crate::healthcare_actions::HEALTHCARE_ACTION_NAMES,
        )
        .expect("healthcare hot-plug drifted from the authoritative table");
        assert_eq!(concepts.len(), 6, "six subjects (visit excluded)");
        assert!(concepts.contains(&("patient", 0x0901)));
        assert!(concepts.contains(&("vital_sign", 0x0907)));
        assert_eq!(caps.len(), 20);
        // The wrong consumer against the same ids bangs UnexpectedConsumer.
        assert!(matches!(
            resolve_hotplug(
                "tesseract-ogar",
                crate::healthcare_actions::HEALTHCARE_SUBJECT_CLASSIDS,
                crate::healthcare_actions::HEALTHCARE_ACTION_NAMES,
            ),
            Err(HotplugDrift::UnexpectedConsumer(_))
        ));
    }

    #[test]
    fn each_drift_arm_bangs() {
        assert!(matches!(
            resolve_hotplug("tesseract-ogar", &[0xFFFE], &[]),
            Err(HotplugDrift::UnknownClassid(0xFFFE))
        ));
        // visit (0x0906) is minted but deliberately table-less — the one
        // Health entity the healthcare table excluded for zero harvest
        // evidence (`healthcare_actions` module doc). It replaced patient
        // (0x0901) as this arm's probe when the healthcare table landed
        // and patient gained capabilities.
        assert!(matches!(
            resolve_hotplug("tesseract-ogar", &[crate::class_ids::VISIT], &[]),
            Err(HotplugDrift::NoCapabilitiesFor(0x0906))
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

    /// The residual-aware derive: minted rows resolve to nonzero classids
    /// (no id-0 escapes into the resolved vec), in original order;
    /// unminted concepts land in the slag ledger carrying enough to name
    /// the next config fact — never silently dropped, never degraded to
    /// a fallback id.
    #[test]
    fn entries_from_actions_with_residuals_splits_minted_and_slag() {
        use crate::ActionDef;
        let minted_a = ActionDef::new("a", "recognize_line", "ogit-ocr/textline");
        let unminted = ActionDef::new("z", "bar", "any:ns/totally_not_a_concept");
        let minted_b = ActionDef::new("b", "render_hocr", "ogit-ocr/ocr_renderer");
        let (resolved, residuals) =
            entries_from_actions_with_residuals(&[minted_a, unminted, minted_b]);

        assert_eq!(
            resolved,
            vec![
                ("recognize_line".to_string(), crate::class_ids::TEXTLINE),
                ("render_hocr".to_string(), crate::class_ids::OCR_RENDERER),
            ]
        );
        assert!(
            resolved.iter().all(|&(_, id)| id != 0),
            "no id-0 escapes into resolved rows"
        );

        assert_eq!(residuals.len(), 1);
        assert_eq!(residuals[0].capability, "bar");
        assert_eq!(residuals[0].object_class, "any:ns/totally_not_a_concept");
        assert_eq!(residuals[0].concept, "totally_not_a_concept");
    }

    /// Parity pin: `entries_from_actions` is exactly the interleaved
    /// (resolved ++ residual-as-0) projection of the residual-aware
    /// derive, in the ORIGINAL action order — the legacy id-0 sentinel
    /// behavior is unchanged by the residual-aware rewrite.
    #[test]
    fn entries_from_actions_matches_residual_derive_interleaved() {
        use crate::ActionDef;
        let minted_a = ActionDef::new("a", "recognize_line", "ogit-ocr/textline");
        let unminted = ActionDef::new("z", "bar", "any:ns/totally_not_a_concept");
        let minted_b = ActionDef::new("b", "render_hocr", "ogit-ocr/ocr_renderer");
        let actions = [minted_a, unminted, minted_b];

        let expected = vec![
            ("recognize_line".to_string(), crate::class_ids::TEXTLINE),
            ("bar".to_string(), 0),
            ("render_hocr".to_string(), crate::class_ids::OCR_RENDERER),
        ];
        assert_eq!(entries_from_actions(&actions), expected);
    }

    /// The healthcare table's bijectivity pin (module doc,
    /// `codebook_dto_derive_is_bijective`), mirrored through the
    /// residual-aware derive: zero slag rows — every subject concept is
    /// minted, so nothing degrades and nothing lands in `UnmintedRow`.
    #[test]
    fn healthcare_entries_with_residuals_has_zero_slag() {
        let (resolved, residuals) =
            entries_from_actions_with_residuals(&crate::healthcare_actions::healthcare_actions());
        assert!(residuals.is_empty(), "unexpected slag: {residuals:?}");
        assert_eq!(
            resolved.len(),
            crate::healthcare_actions::HEALTHCARE_ACTION_NAMES.len()
        );
    }

    /// The hand-authored OCR table, routed through the generic derive, has
    /// one join row per declared capability (14 after the v2 growth,
    /// 2026-07-10) — the "config becomes data" seam subsumes the bespoke
    /// path with zero behavior change. Count keyed to the live
    /// `OCR_ACTION_NAMES` so it can never re-drift.
    #[test]
    fn ocr_entries_derive_one_row_per_declared_capability() {
        let ocr = ocr_entries();
        assert_eq!(ocr.len(), crate::ocr_actions::OCR_ACTION_NAMES.len());
        assert!(
            ocr.iter()
                .any(|(cap, id)| cap == "recognize_line" && *id == crate::class_ids::TEXTLINE)
        );
        assert!(
            ocr.iter()
                .any(|(cap, id)| cap == "render_hocr" && *id == crate::class_ids::OCR_RENDERER)
        );
        // v2 rows resolve to their page_image / page_layout subjects.
        assert!(
            ocr.iter()
                .any(|(cap, id)| cap == "recognize_document" && *id == crate::class_ids::PAGE_IMAGE)
        );
        assert!(ocr.iter().any(
            |(cap, id)| cap == "detect_page_furniture" && *id == crate::class_ids::PAGE_LAYOUT
        ));
    }
}
