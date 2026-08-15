from pathlib import Path


def once(text: str, old: str, new: str, label: str) -> str:
    n = text.count(old)
    if n != 1:
        raise SystemExit(f"{label}: expected exactly one anchor, found {n}")
    return text.replace(old, new, 1)


libp = Path("crates/ogar-vocab/src/lib.rs")
s = libp.read_text()

# all_promoted_classes() is an ordered structural mirror of class_ids::ALL.
# Weather occupies 0x04XX, immediately after 0x02XX commerce and before OCR.
s = once(
    s,
    "        unicharset(),\n",
    "        weather_cell(),\n        weather_static_cell(),\n        unicharset(),\n",
    "all_promoted_classes weather builders",
)

# The domain routing test intentionally enumerates reserved/unassigned blocks.
# 0x04 has now been authoritatively allocated; only 0x05-0x06 remain unassigned.
s = once(
    s,
    "        // Unassigned blocks (4-6).\n"
    "        assert_eq!(canonical_concept_domain(0x0400), ConceptDomain::Unassigned);\n"
    "        assert_eq!(canonical_concept_domain(0x0600), ConceptDomain::Unassigned);\n",
    "        // Weather / Atmosphere block (0x04), then still-unassigned 0x05-0x06.\n"
    "        assert_eq!(canonical_concept_domain(0x0400), ConceptDomain::Weather);\n"
    "        assert_eq!(canonical_concept_domain(0x0401), ConceptDomain::Weather);\n"
    "        assert_eq!(canonical_concept_domain(0x0500), ConceptDomain::Unassigned);\n"
    "        assert_eq!(canonical_concept_domain(0x0600), ConceptDomain::Unassigned);\n",
    "domain allocation test",
)
libp.write_text(s)

# This is a count fuse for the global codebook, not a palette-only count. The
# two canonical weather rows legitimately move it from 91 to 93; the 0x17XX
# exclusion assertion remains unchanged and continues to prove the actual rule.
cp = Path("crates/ogar-vocab/src/capability_registry.rs")
c = cp.read_text()
c = once(
    c,
    "        assert_eq!(crate::class_ids::ALL.len(), 91);\n",
    "        assert_eq!(crate::class_ids::ALL.len(), 93);\n",
    "global codebook count fuse",
)
cp.write_text(c)
