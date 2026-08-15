from pathlib import Path


def once(text: str, old: str, new: str, label: str) -> str:
    n = text.count(old)
    if n != 1:
        raise SystemExit(f"{label}: expected exactly one anchor, found {n}")
    return text.replace(old, new, 1)


libp = Path("crates/ogar-vocab/src/lib.rs")
s = libp.read_text()
s = once(
    s,
    "///   0x04XX  unassigned\n",
    "///   0x04XX  Weather / Atmosphere (forecast + atmospheric reference cells)\n",
    "domain map doc",
)
s = once(
    s,
    "    // ── 0x07XX — OSINT domain: ZERO vocabulary rows BY DESIGN (operator\n",
    "    // ── 0x04XX — Weather / Atmosphere domain ──\n"
    "    // Canonical atmospheric/grid concepts consumed by WeatherNext and the\n"
    "    // lance-graph weather SoA bake. These are shared meanings; renderer /\n"
    "    // ClassView selection remains in the low u16 of the full classid.\n"
    "    (\"weather_cell\", 0x0401),\n"
    "    (\"weather_static_cell\", 0x0402),\n"
    "    // ── 0x07XX — OSINT domain: ZERO vocabulary rows BY DESIGN (operator\n",
    "CODEBOOK weather rows",
)
s = once(
    s,
    "    Ontology,\n    /// `0x07XX` — OSINT (open-source intelligence).\n",
    "    Ontology,\n"
    "    /// `0x04XX` — Weather / Atmosphere. Shared atmospheric and forecast-grid\n"
    "    /// concepts; public environmental reference data, not an OSM extension.\n"
    "    Weather,\n"
    "    /// `0x07XX` — OSINT (open-source intelligence).\n",
    "ConceptDomain Weather variant",
)
s = once(
    s,
    "        0x03 => ConceptDomain::Ontology,\n        0x07 => ConceptDomain::Osint,\n",
    "        0x03 => ConceptDomain::Ontology,\n"
    "        0x04 => ConceptDomain::Weather,\n"
    "        0x07 => ConceptDomain::Osint,\n",
    "Weather domain routing",
)

# Scope the constants insertion to class_ids so the identically-worded CODEBOOK
# OCR marker cannot be selected accidentally.
class_ids_at = s.index("pub mod class_ids {")
head, class_ids_tail = s[:class_ids_at], s[class_ids_at:]
class_ids_tail = once(
    class_ids_tail,
    "    // ── 0x08XX — OCR domain (document extraction; the Tesseract-rs arc) ──\n",
    "    // ── 0x04XX — Weather / Atmosphere domain ──\n\n"
    "    /// `weather_cell` (`0x0401`) — one dynamic atmospheric/forecast grid\n"
    "    /// cell. Time is a dataset/version axis, not a new concept id.\n"
    "    pub const WEATHER_CELL: u16 = 0x0401;\n"
    "    /// `weather_static_cell` (`0x0402`) — static support cell for terrain /\n"
    "    /// geography-derived weather context, distinct from dynamic fields.\n"
    "    pub const WEATHER_STATIC_CELL: u16 = 0x0402;\n\n"
    "    // ── 0x08XX — OCR domain (document extraction; the Tesseract-rs arc) ──\n",
    "class_ids constants",
)
s = head + class_ids_tail
s = once(
    s,
    "        (\"unit_of_measure\", UNIT_OF_MEASURE),\n        // 0x07XX — OSINT: ZERO vocabulary rows BY DESIGN",
    "        (\"unit_of_measure\", UNIT_OF_MEASURE),\n"
    "        // 0x04XX — Weather / Atmosphere\n"
    "        (\"weather_cell\", WEATHER_CELL),\n"
    "        (\"weather_static_cell\", WEATHER_STATIC_CELL),\n"
    "        // 0x07XX — OSINT: ZERO vocabulary rows BY DESIGN",
    "class_ids::ALL weather rows",
)
s = once(
    s,
    "                91,\n                \"class_ids::ALL count changed",
    "                93,\n                \"class_ids::ALL count changed",
    "class_ids count fuse",
)

geo_builder_anchor = (
    "// ─────────────────────────────────────────────────────────────────────\n"
    "// 0x0FXX — Geo domain builders (OpenStreetMap geodata reference ontology).\n"
)
weather_builders = '''// ─────────────────────────────────────────────────────────────────────
// 0x04XX — Weather / Atmosphere canonical builders.
// Field/level/unit slot semantics live in the consumer ClassView manifest;
// the canonical class carries identity, never the weather payload layout.
// ─────────────────────────────────────────────────────────────────────

/// `weather_cell` (`0x0401`) — one dynamic atmospheric / forecast-grid cell.
#[must_use]
pub fn weather_cell() -> Class {
    let mut c = Class::new("WeatherCell");
    c.language = Language::Unknown;
    c.canonical_concept = Some("weather_cell".to_string());
    c
}

/// `weather_static_cell` (`0x0402`) — static geography-derived support cell.
#[must_use]
pub fn weather_static_cell() -> Class {
    let mut c = Class::new("WeatherStaticCell");
    c.language = Language::Unknown;
    c.canonical_concept = Some("weather_static_cell".to_string());
    c
}

'''
s = once(s, geo_builder_anchor, weather_builders + geo_builder_anchor, "weather builders")
s += '''
#[cfg(test)]
mod weather_classid_mint_tests {
    use super::*;

    #[test]
    fn weather_domain_and_codebook_are_append_only_and_not_geo() {
        assert_eq!(canonical_concept_domain(class_ids::WEATHER_CELL), ConceptDomain::Weather);
        assert_eq!(canonical_concept_domain(class_ids::WEATHER_STATIC_CELL), ConceptDomain::Weather);
        assert_ne!(canonical_concept_domain(class_ids::WEATHER_CELL), ConceptDomain::Geo);
        assert_eq!(canonical_concept_id("weather_cell"), Some(0x0401));
        assert_eq!(canonical_concept_id("weather_static_cell"), Some(0x0402));
        assert_eq!(weather_cell().canonical_id(), Some(class_ids::WEATHER_CELL));
        assert_eq!(weather_static_cell().canonical_id(), Some(class_ids::WEATHER_STATIC_CELL));
    }
}
'''
libp.write_text(s)

pp = Path("crates/ogar-vocab/src/ports.rs")
p = pp.read_text()
anchor = "#[cfg(test)]\nmod tests {"
weather_port = '''// ── WeatherNext / weathernext-rs port ───────────────────────────────

/// WeatherNext's `PortSpec` over the shared Weather / Atmosphere (`0x04XX`)
/// canonical concepts. `0x0009` is the reserved WeatherNext ClassView skin;
/// the canonical meaning remains in the high u16.
pub struct WeatherNextPort;

impl PortSpec for WeatherNextPort {
    const NAMESPACE: &'static str = "WeatherNext";
    const BRIDGE_ID: &'static str = "weathernext";
    const APP_PREFIX: u16 = 0x0009;
    fn aliases() -> &'static [(&'static str, u16)] {
        WEATHERNEXT_ALIASES
    }
}

/// WeatherNext public names mapped onto shared canonical weather concepts.
pub const WEATHERNEXT_ALIASES: &[(&str, u16)] = &[
    ("WeatherCell", class_ids::WEATHER_CELL),
    ("WeatherStaticCell", class_ids::WEATHER_STATIC_CELL),
];

'''
p = once(p, anchor, weather_port + anchor, "WeatherNext PortSpec")
p = once(
    p,
    "    use super::*;\n",
    "    use super::*;\n"
    "    use crate::app::{app_of, concept_of, render_classid};\n\n"
    "    #[test]\n"
    "    fn weathernext_classview_composes_canon_high_custom_low() {\n"
    "        assert_eq!(WeatherNextPort::APP_PREFIX, 0x0009);\n"
    "        assert_eq!(WeatherNextPort::classview(), 0x0009);\n"
    "        assert_eq!(WeatherNextPort::class_id(\"WeatherCell\"), Some(0x0401));\n"
    "        assert_eq!(WeatherNextPort::class_id(\"WeatherStaticCell\"), Some(0x0402));\n"
    "        let dynamic = render_classid(WeatherNextPort::APP_PREFIX, class_ids::WEATHER_CELL);\n"
    "        let statics = render_classid(WeatherNextPort::APP_PREFIX, class_ids::WEATHER_STATIC_CELL);\n"
    "        assert_eq!(dynamic, 0x0401_0009);\n"
    "        assert_eq!(statics, 0x0402_0009);\n"
    "        assert_eq!(concept_of(dynamic), 0x0401);\n"
    "        assert_eq!(app_of(dynamic), 0x0009);\n"
    "    }\n",
    "WeatherNext tests",
)
pp.write_text(p)

dp = Path("docs/APP-CLASS-CODEBOOK-LAYOUT.md")
d = dp.read_text()
d = once(
    d,
    "| `0x0008` | OpenStreetMap (openstreetmap-website-rs) | `0x0F` geo | **no** — maps entirely onto core |\n",
    "| `0x0008` | OpenStreetMap (openstreetmap-website-rs) | `0x0F` geo | **no** — maps entirely onto core |\n"
    "| `0x0009` | WeatherNext / weathernext-rs | `0x04` weather / atmosphere | **no** — maps onto core |\n",
    "ClassView allocation table",
)
dp.write_text(d)
