//! Agnostic xref → baked-address crosswalk — the data-as-config adapter.
//!
//! One mechanism, every consumer: a **reverse index** over the bake's preserved
//! xrefs. Any external code (ICD · FMA · MeSH · SNOMED · OMIM · Orphanet · …)
//! resolves to its **baked OBO address** (`classid` + 24-bit identity) — no
//! name-match, no per-consumer hand-rolled table. medcare-rs resolves ICD→MONDO
//! and FMA/DICOM→Uberon through it; odoo-rs can resolve its own account codes
//! through the *same* adapter the day its vocabulary is baked. The corpus is
//! OBO-public; the mechanism is agnostic (OGAR connective tissue, not a
//! consumer mirror).
//!
//! **Not** a hand-rolled literal table: the index is built from the parsed
//! [`Bake`](crate::Bake) (`generate from the source, never retype it`).

use crate::{Bake, XrefSource};
use std::collections::HashMap;

/// A baked OBO address — the canon-high classid + the node's 24-bit identity.
/// This is what a resolved external code points AT, replacing a string label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address {
    /// canon-high classid (`concept<<16 | app`); app = `0x0000` reference skin
    pub classid: u32,
    /// the node's 24-bit identity (the CURIE numeric id)
    pub identity: u32,
}

/// A stable tag for an xref source, so the reverse index key is `(tag, code)`
/// without carrying an owned `XrefSource` per entry.
fn tag(s: &XrefSource) -> u8 {
    match s {
        XrefSource::Mesh => 1,
        XrefSource::Umls => 2,
        XrefSource::Omim => 3,
        XrefSource::Orphanet => 4,
        XrefSource::Snomed => 5,
        XrefSource::Icd => 6,
        XrefSource::Other(_) => 7,
    }
}

/// The reverse xref index: `(source-tag, code) -> baked address`. Built once
/// from a [`Bake`]; resolves in O(1). For `Other` sources the code is prefixed
/// with the raw source string (`"FMA:7148"`) so distinct `Other` systems don't
/// collide.
#[derive(Debug, Clone, Default)]
pub struct Crosswalk {
    idx: HashMap<(u8, String), Address>,
}

impl Crosswalk {
    /// Build the reverse index from the parsed bake's preserved xrefs — the
    /// data-as-config core. Every `(node, xref)` becomes `xref -> node address`.
    #[must_use]
    pub fn from_bake(bake: &Bake) -> Crosswalk {
        let mut idx = HashMap::with_capacity(bake.xrefs.len());
        for (id, x) in &bake.xrefs {
            let addr = Address {
                classid: id.namespace().render_classid(0x0000),
                identity: id.num,
            };
            let key = match &x.source {
                XrefSource::Other(src) => (7u8, format!("{src}:{}", x.id)),
                other => (tag(other), x.id.clone()),
            };
            // first writer wins → stable, deterministic (bake.xrefs is sorted by node)
            idx.entry(key).or_insert(addr);
        }
        Crosswalk { idx }
    }

    /// Resolve an external code to its baked OBO address, if the bake carries the
    /// xref. Agnostic over the source — the same call medcare and odoo use.
    #[must_use]
    pub fn resolve(&self, source: &XrefSource, code: &str) -> Option<Address> {
        let key = match source {
            XrefSource::Other(src) => (7u8, format!("{src}:{code}")),
            other => (tag(other), code.to_string()),
        };
        self.idx.get(&key).copied()
    }

    /// Resolve an FMA id (e.g. `"7148"`) to its Uberon address — the glass-patient
    /// helix-zone lookup. `FMA` rides the `Other("FMA")` source.
    #[must_use]
    pub fn resolve_fma(&self, fma_id: &str) -> Option<Address> {
        self.resolve(&XrefSource::Other("FMA".into()), fma_id)
    }

    /// Resolve an ICD-10 code to its disease (MONDO) address, with the **2-hop
    /// rollup** the measured MONDO coverage requires: MONDO xrefs the WHO axis
    /// densely but carries **no ICD-10-GM**, so a German billing code like
    /// `"E11.9"` that has no direct xref rolls up its ICD hierarchy
    /// (`E11.9 → E11 → E1`) until a baked MONDO address is found. Pure string
    /// rollup on the code — no new table.
    #[must_use]
    pub fn resolve_icd10(&self, code: &str) -> Option<Address> {
        let mut c = code.trim().to_string();
        loop {
            if let Some(a) = self.resolve(&XrefSource::Icd, &c) {
                return Some(a);
            }
            // roll up: drop a trailing digit past the 3-char WHO stem, else the
            // sub-category dot, else give up.
            if let Some(pos) = c.rfind(|ch: char| ch.is_ascii_digit())
                && (c.len() > 3 || (c.contains('.') && pos > c.find('.').unwrap()))
            {
                c.truncate(pos);
                if c.ends_with('.') {
                    c.pop();
                }
                if c.is_empty() {
                    return None;
                }
                continue;
            }
            return None;
        }
    }

    /// Number of indexed xrefs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.idx.len()
    }
    /// Whether the index is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.idx.is_empty()
    }
}

/// The Uberon anatomy classid (`0x0303`), for a consumer that resolves an FMA
/// zone and wants to confirm the returned address is anatomy.
pub const ANATOMY_CLASSID: u32 = 0x0303_0000;
/// The MONDO disease classid (`0x0301`).
pub const DISEASE_CLASSID: u32 = 0x0301_0000;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Namespace, OboNode, Predicate, TermId, Xref, bake};
    use std::collections::HashMap;

    fn node_with_xrefs(xr: Vec<Xref>) -> OboNode {
        OboNode {
            xref: xr,
            ..Default::default()
        }
    }

    #[test]
    fn reverse_index_resolves_any_source_to_a_baked_address() {
        let mut nodes: HashMap<TermId, OboNode> = HashMap::new();
        // MONDO:5148 (T2DM) xrefs ICD-10 E11 + MeSH D003924
        nodes.insert(
            TermId {
                ns: Namespace::Mondo as u8,
                num: 5148,
            },
            node_with_xrefs(vec![
                Xref {
                    source: XrefSource::Icd,
                    id: "E11".into(),
                },
                Xref {
                    source: XrefSource::Mesh,
                    id: "D003924".into(),
                },
            ]),
        );
        // UBERON:945 (stomach) xrefs FMA:7148
        nodes.insert(
            TermId {
                ns: Namespace::Uberon as u8,
                num: 945,
            },
            node_with_xrefs(vec![Xref {
                source: XrefSource::Other("FMA".into()),
                id: "7148".into(),
            }]),
        );
        let baked = bake(&nodes, 0x0000);
        let cw = Crosswalk::from_bake(&baked);

        // ICD → MONDO baked address (the reasoning-chain enabler)
        let d = cw.resolve(&XrefSource::Icd, "E11").unwrap();
        assert_eq!(d.classid, Namespace::Mondo.render_classid(0));
        assert_eq!(d.identity, 5148);
        // MeSH → same disease (a different bearing, one adapter)
        assert_eq!(cw.resolve(&XrefSource::Mesh, "D003924"), Some(d));
        // FMA → Uberon (the glass-patient zone)
        let a = cw.resolve_fma("7148").unwrap();
        assert_eq!(a.classid, Namespace::Uberon.render_classid(0));
        assert_eq!(a.identity, 945);
        // an unbaked code resolves to nothing (honest boundary)
        assert!(cw.resolve(&XrefSource::Snomed, "44054006").is_none());
    }

    #[test]
    fn icd_gm_rolls_up_to_the_baked_who_stem() {
        // MONDO xrefs only the WHO stem "E11"; the German billing "E11.9" must
        // roll up to it (measured: MONDO carries no ICD-10-GM).
        let mut nodes: HashMap<TermId, OboNode> = HashMap::new();
        nodes.insert(
            TermId {
                ns: Namespace::Mondo as u8,
                num: 5148,
            },
            node_with_xrefs(vec![Xref {
                source: XrefSource::Icd,
                id: "E11".into(),
            }]),
        );
        let cw = Crosswalk::from_bake(&bake(&nodes, 0x0000));
        // direct GM code fails, rollup succeeds — no separate GM table
        assert!(cw.resolve(&XrefSource::Icd, "E11.9").is_none());
        assert_eq!(cw.resolve_icd10("E11.9").unwrap().identity, 5148);
        // a code with no baked stem at all still returns None (can-stay-silent)
        assert!(cw.resolve_icd10("Z99.9").is_none());
    }

    #[test]
    fn predicate_import_is_used() {
        // keep the Predicate import meaningful (edges exist alongside xrefs)
        assert_ne!(Predicate::IsA, Predicate::PartOf);
    }
}
