//! `ogar-cpic` — the **CPIC pharmacogenomics reference surface**: the public
//! gene → phenotype → drug-recommendation guideline table that q2 `/cpic`
//! chains, and the NARS `(frequency, confidence)` premise it deduces over.
//!
//! # What this is
//!
//! A clinical reasoning layer knows a therapy holds a *join key* into
//! pharmacogenomics — "azathioprine is dosed by TPMT/NUDT15 phenotype". The
//! CPIC guideline itself (which phenotype, what to do about it, how strong the
//! evidence) is **public reference** owned upstream (commitment #3: outside
//! ontology is owned upstream), not something the consumer restates. This
//! crate is the single public place that guideline table lives.
//!
//! A consumer pulls it — `ogar_cpic::resolve("TPMT/NUDT15", "azathioprine")` —
//! and gets the phenotype axis, the actionable recommendation, and a NARS
//! `(f, c)` deduction premise. It never hand-maintains a parallel CPIC table.
//!
//! # This is reference, NOT a genotype
//!
//! Everything here is the **published CPIC guideline** — gene-name, phenotype
//! *axis* (the set of metabolizer statuses that exist), the population-level
//! recommendation. It carries **no patient genotype and no PHI**: a named
//! patient's `TPMT *3A/*3C` call is Health data that stays inside the clinical
//! repo, exactly as the firewall split keeps FMA structure (public) apart from
//! a fracture diagnosis (PHI). This surface tells you *what CPIC says about the
//! gene*; it never tells you *what a person carries*.
//!
//! # Classid routing — the reserved Genetics domain (`0x0EXX`)
//!
//! `ogar_vocab` reserves `ConceptDomain::Genetics` (`0x0EXX`) for CPIC
//! pharmacogenomics, **consumed by q2** — but ships ZERO concept rows there,
//! guarded by an explicit *"do not mint without an operator ruling"* note (the
//! same posture as the OSINT domain). This crate honors that gate: it routes at
//! the **domain** level ([`GENETICS_DOMAIN`] / [`GENETICS_DOMAIN_HI`]) and does
//! NOT mint a concept id into the shared codebook. The committed anchor for the
//! first Genetics concept is the V3 marker form `0x0E01_1000` (CPIC under q2,
//! appid `0x01`; `docs/DISCOVERY-MAP.md` D-CLASSID-CANON-HIGH-FLIP) — minting it
//! is a deliberate, operator-gated follow-up, surfaced not silently taken.
//! Until then a consumer routes CPIC nodes on the reserved domain tag and keys
//! the concrete guideline by its (gene, drug) pair.
//!
//! # Curated subset of the full CPIC corpus
//!
//! This table is the small, **actionable** subset a consumer resolves directly.
//! The **full** CPIC corpus (all guidelines, pairs, alleles, diplotype→phenotype
//! maps) is the graph a reasoner walks, loaded separately as `graph:cpic`. The
//! contract: the curated row answers "what does CPIC say about *this* pair?" in
//! one lookup; the graph answers the open-ended "is there a guideline for X, and
//! what does it chain to?" A pair absent here *may* still have a CPIC guideline
//! in `graph:cpic` (this table carries only the small actionable subset) — but
//! many gene-drug pairs have no CPIC guideline at all and resolve in neither.
//! [`resolve`] returning `None` therefore means "no actionable guideline in this
//! curated table," not "no guideline exists anywhere"; consult `graph:cpic` for
//! the authoritative answer.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use ogar_vocab::ConceptDomain;

/// The reserved Genetics [`ConceptDomain`] every CPIC node routes on. The
/// domain slot is live in `ogar_vocab` even with zero concept rows, so a
/// consumer can branch on it today.
pub const GENETICS_DOMAIN: ConceptDomain = ConceptDomain::Genetics;

/// The high byte of the Genetics domain (`0x0E`) — the `id >> 8` a consumer
/// matches when routing a CPIC node from a bare classid. The committed first
/// concept anchor is `0x0E01` (see the module docs); this crate does not mint
/// it.
pub const GENETICS_DOMAIN_HI: u8 = 0x0E;

/// CPIC recommendation strength for a gene-drug pair — the published guideline
/// classification, which sets how hard the NARS premise leans.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum Strength {
    /// A strong recommendation — the evidence is high and the action clear
    /// (e.g. `HLA-B*58:01` positive ⇒ do not use allopurinol).
    Strong,
    /// A moderate recommendation.
    Moderate,
    /// An optional recommendation — actionable but weaker evidence.
    Optional,
}

impl Strength {
    /// A NARS `(frequency, confidence)` deduction premise for a guideline at
    /// this strength — the `(f, c)` q2 `/cpic` chains gene→phenotype→rec over.
    /// Frequency is how reliably the phenotype implies the action; confidence
    /// tracks the CPIC evidence grade.
    #[must_use]
    pub const fn nars_truth(self) -> (f64, f64) {
        match self {
            Strength::Strong => (0.95, 0.92),
            Strength::Moderate => (0.85, 0.78),
            Strength::Optional => (0.70, 0.60),
        }
    }
}

/// A resolved public CPIC guideline — one gene(-pair) × drug row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CpicGuideline {
    /// The CPIC gene or gene-pair label (e.g. `"TPMT/NUDT15"`, `"HLA-B*58:01"`)
    /// — the exact join key a consumer's therapy annotation carries.
    pub gene: &'static str,
    /// The affected drug (the therapy the guideline doses).
    pub drug: &'static str,
    /// The phenotype axis CPIC stratifies on — the *set of statuses that
    /// exist* (e.g. `"metabolizer status (normal / intermediate / poor)"`).
    /// Public reference: the axis, never a patient's position on it.
    pub phenotype_axis: &'static str,
    /// The actionable population-level recommendation for the affected
    /// phenotype(s).
    pub recommendation: &'static str,
    /// The CPIC recommendation strength.
    pub strength: Strength,
}

impl CpicGuideline {
    /// The NARS `(frequency, confidence)` premise for this guideline.
    #[must_use]
    pub const fn nars_truth(&self) -> (f64, f64) {
        self.strength.nars_truth()
    }
}

/// The curated public CPIC guideline table — actionable gene-drug pairs. This
/// is published CPIC reference (Level A / actionable guidelines), not an
/// exhaustive mirror; extended as consumers pull more pairs.
const GUIDELINES: &[CpicGuideline] = &[
    CpicGuideline {
        gene: "TPMT/NUDT15",
        drug: "azathioprine",
        phenotype_axis: "thiopurine metabolizer status (normal / intermediate / poor)",
        recommendation: "poor metabolizer ⇒ substantially reduce dose or use an alternative agent \
             (myelosuppression risk); intermediate ⇒ reduce starting dose",
        strength: Strength::Strong,
    },
    CpicGuideline {
        gene: "HLA-B*58:01",
        drug: "allopurinol",
        phenotype_axis: "HLA-B*58:01 allele carrier status (positive / negative)",
        recommendation: "positive ⇒ do NOT prescribe allopurinol (SCAR / SJS-TEN risk); \
             use an alternative urate-lowering therapy (e.g. febuxostat with caution)",
        strength: Strength::Strong,
    },
    CpicGuideline {
        gene: "CYP2C19",
        drug: "clopidogrel",
        phenotype_axis: "metabolizer status (ultrarapid / normal / intermediate / poor)",
        recommendation: "intermediate or poor metabolizer ⇒ use an alternative antiplatelet \
             (prasugrel / ticagrelor) — reduced active-metabolite formation",
        strength: Strength::Strong,
    },
    CpicGuideline {
        gene: "CYP2D6",
        drug: "codeine",
        phenotype_axis: "metabolizer status (ultrarapid / normal / intermediate / poor)",
        recommendation: "ultrarapid ⇒ avoid (morphine toxicity); poor ⇒ avoid (no analgesia); \
             use a non-CYP2D6 opioid alternative",
        strength: Strength::Strong,
    },
    CpicGuideline {
        gene: "SLCO1B1",
        drug: "simvastatin",
        phenotype_axis: "SLCO1B1 function (normal / decreased / poor)",
        recommendation: "decreased or poor function ⇒ lower dose or an alternative statin \
             (myopathy risk)",
        strength: Strength::Strong,
    },
    CpicGuideline {
        gene: "DPYD",
        drug: "fluorouracil",
        phenotype_axis: "DPD activity (normal / intermediate / poor)",
        recommendation: "intermediate ⇒ reduce starting dose; poor ⇒ avoid \
             (severe / fatal fluoropyrimidine toxicity)",
        strength: Strength::Strong,
    },
    CpicGuideline {
        gene: "CYP2C9/VKORC1",
        drug: "warfarin",
        phenotype_axis: "combined CYP2C9 + VKORC1 dosing genotype",
        recommendation: "use the pharmacogenetic dosing algorithm — variant carriers need a \
             lower maintenance dose",
        strength: Strength::Moderate,
    },
];

/// Resolve a CPIC guideline by its gene(-pair) label and drug. Case-insensitive
/// on ASCII; whitespace-trimmed. `None` = no actionable CPIC guideline for that
/// pair (which a consumer treats exactly as its `cpic_gene = None` loose end).
///
/// ```
/// let g = ogar_cpic::resolve("HLA-B*58:01", "allopurinol").unwrap();
/// assert_eq!(g.strength, ogar_cpic::Strength::Strong);
/// let (f, c) = g.nars_truth();
/// assert!(f > 0.9 && c > 0.9);
/// ```
#[must_use]
pub fn resolve(gene: &str, drug: &str) -> Option<&'static CpicGuideline> {
    let (gene, drug) = (gene.trim(), drug.trim());
    GUIDELINES
        .iter()
        .find(|g| g.gene.eq_ignore_ascii_case(gene) && g.drug.eq_ignore_ascii_case(drug))
}

/// Resolve by gene(-pair) label alone — the first guideline naming that gene.
#[must_use]
pub fn resolve_gene(gene: &str) -> Option<&'static CpicGuideline> {
    let gene = gene.trim();
    GUIDELINES
        .iter()
        .find(|g| g.gene.eq_ignore_ascii_case(gene))
}

/// Every published guideline — for a consumer indexing the whole table once.
#[must_use]
pub fn guidelines() -> &'static [CpicGuideline] {
    GUIDELINES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genetics_domain_routing_is_live_without_a_mint() {
        // The domain tag resolves even though the codebook ships zero Genetics
        // concept rows (the reserved-domain posture).
        assert_eq!(GENETICS_DOMAIN, ConceptDomain::Genetics);
        assert_eq!(GENETICS_DOMAIN_HI, 0x0E);
        // The committed first-concept anchor 0x0E01 routes to Genetics, but we
        // do NOT mint it here — resolving the domain from the anchor is enough.
        assert_eq!(
            ogar_vocab::canonical_concept_domain(0x0E01),
            ConceptDomain::Genetics,
        );
        // And it is genuinely unminted (gate honored).
        assert_eq!(ogar_vocab::canonical_concept_name(0x0E01), None);
    }

    #[test]
    fn medcare_actionable_pairs_resolve() {
        // The two actionable pgx pairs medcare-cohorts carries as join keys.
        let aza = resolve("TPMT/NUDT15", "azathioprine").unwrap();
        assert_eq!(aza.strength, Strength::Strong);
        assert!(aza.recommendation.contains("poor metabolizer"));

        let allo = resolve("HLA-B*58:01", "allopurinol").unwrap();
        assert_eq!(allo.strength, Strength::Strong);
        assert!(allo.recommendation.contains("do NOT"));
    }

    #[test]
    fn resolve_is_case_and_whitespace_insensitive() {
        assert!(resolve("  cyp2c19 ", "CLOPIDOGREL").is_some());
        assert!(resolve_gene("dpyd").is_some());
        assert!(resolve("TPMT/NUDT15", "aspirin").is_none());
        assert!(resolve("ABCB1", "digoxin").is_none());
    }

    #[test]
    fn nars_truth_leans_with_strength() {
        let strong = Strength::Strong.nars_truth();
        let moderate = Strength::Moderate.nars_truth();
        let optional = Strength::Optional.nars_truth();
        // Full monotone chain Strong > Moderate > Optional on BOTH axes — pins
        // Moderate, which a Strong-vs-Optional-only check would leave unguarded.
        assert!(
            strong.0 > moderate.0 && moderate.0 > optional.0,
            "frequency is monotone in strength",
        );
        assert!(
            strong.1 > moderate.1 && moderate.1 > optional.1,
            "confidence is monotone in strength",
        );
        // Bounded probabilities.
        for s in [Strength::Strong, Strength::Moderate, Strength::Optional] {
            let (f, c) = s.nars_truth();
            assert!((0.0..=1.0).contains(&f) && (0.0..=1.0).contains(&c));
        }
    }

    #[test]
    fn gene_labels_carry_no_patient_genotype() {
        // Reference-not-genotype invariant: a guideline names the phenotype
        // *axis* (a status/function/activity/carrier space), never a specific
        // patient star-allele diplotype call (a "*3A/*3C" would live in the
        // clinical repo, never here). Every axis uses axis vocabulary.
        const AXIS_WORDS: [&str; 5] = ["status", "function", "activity", "genotype", "carrier"];
        for g in guidelines() {
            let axis = g.phenotype_axis.to_ascii_lowercase();
            assert!(
                AXIS_WORDS.iter().any(|w| axis.contains(w)),
                "{} phenotype_axis reads like a genotype call, not an axis: {}",
                g.gene,
                g.phenotype_axis,
            );
            // A concrete diplotype call is `*<allele>/*<allele>`; never present.
            assert!(
                !axis.contains("*/") && !axis.contains("/*"),
                "{} phenotype_axis contains a diplotype call",
                g.gene,
            );
        }
    }

    #[test]
    fn gene_drug_pairs_are_unique() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for g in guidelines() {
            assert!(
                seen.insert((g.gene, g.drug)),
                "duplicate CPIC pair {} / {}",
                g.gene,
                g.drug,
            );
        }
    }
}
