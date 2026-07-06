//! Falsifier #2, askama (Rust) side — one ClassView x FieldMask projection,
//! read from `tests/fixtures/mask_roundtrip.json`, must emit exactly the
//! `expected_present` field set. `tests/jinja/test_mask_roundtrip.py` reads
//! the SAME fixture and must agree (Operator Sec.5/6; D-FIELDMASK-LOUD-FAIL
//! companion falsifier, E-ONE-MASK-TWO-ENGINES).
//!
//! Class stays <=64 fields on purpose (SPEC-2 non-goal): the >64 case is
//! covered by the pin test + loud-fail guard in `src/rust_class.rs`, not by
//! a round-trip fidelity proof here.

use std::collections::BTreeSet;

use lance_graph_contract::class_view::FieldMask;
use ogar_render_askama::render_class_with_methods;
use ogar_vocab::{Attribute, Class};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct MaskRoundtripFixture {
    class_name: String,
    fields: Vec<String>,
    mask: u64,
    expected_present: Vec<String>,
}

fn load_fixture() -> MaskRoundtripFixture {
    let raw = include_str!("fixtures/mask_roundtrip.json");
    serde_json::from_str(raw).expect("fixture must be valid JSON matching MaskRoundtripFixture")
}

fn build_class(fixture: &MaskRoundtripFixture) -> Class {
    let mut class = Class::new(&fixture.class_name);
    class.canonical_concept = None;
    class.attributes = fixture
        .fields
        .iter()
        .map(|name| {
            let mut a = Attribute::default();
            a.name = name.clone();
            a.type_name = Some("string".to_string());
            a
        })
        .collect();
    class
}

/// Parse `pub <ident>: <type>,` field-declaration lines out of the emitted
/// struct body into the set of present field names.
fn present_field_names(src: &str) -> BTreeSet<String> {
    src.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            // Only bare `pub <ident>: <type>,` field-declaration lines —
            // excludes `pub struct Foo {`, `pub const CLASS_ID: ...;`, and
            // `pub fn new(...) -> Self {` (none of which end in `,`, and
            // the const/fn variants also carry a keyword right after `pub `).
            let rest = trimmed.strip_prefix("pub ")?;
            if !trimmed.ends_with(',') || rest.starts_with("const ") || rest.starts_with("fn ") {
                return None;
            }
            let ident = rest.split(':').next()?;
            Some(ident.trim().to_string())
        })
        .collect()
}

#[test]
fn fixture_is_self_consistent_mask_matches_expected_present() {
    // The invariant SPEC-2 mandates: expected_present[k] == fields[i] for
    // every i where (mask >> i) & 1 == 1, in order. Recompute independently
    // of the JSON's own `expected_present` value so a hand-miscomputed
    // fixture is caught here rather than silently trusted.
    let fixture = load_fixture();
    let recomputed: Vec<String> = fixture
        .fields
        .iter()
        .enumerate()
        .filter(|(i, _)| (fixture.mask >> i) & 1 == 1)
        .map(|(_, f)| f.clone())
        .collect();
    assert_eq!(
        recomputed, fixture.expected_present,
        "fixture expected_present disagrees with mask bit-walk over fields"
    );
    assert!(
        fixture.fields.len() <= FieldMask::MAX_FIELDS as usize,
        "fixture class must stay <=64 fields (SPEC-2 non-goal: no wide class here)"
    );
}

#[test]
fn askama_mask_projection_yields_expected_present_field_set() {
    let fixture = load_fixture();
    let class = build_class(&fixture);
    let mask = FieldMask(fixture.mask);

    let src = render_class_with_methods(&class, mask, &[])
        .expect("<=64-field class under a partial mask must render Ok");

    let present = present_field_names(&src);
    let expected: BTreeSet<String> = fixture.expected_present.iter().cloned().collect();
    assert_eq!(
        present, expected,
        "askama-rendered field set does not match the fixture's expected_present:\n{src}"
    );

    // Direct per-bit pin against the mask contract: presence of fields[i] in
    // the output == ((mask >> i) & 1) == 1, for every field position.
    for (i, name) in fixture.fields.iter().enumerate() {
        let bit_set = (fixture.mask >> i) & 1 == 1;
        let in_output = src.contains(&format!("pub {name}:"));
        assert_eq!(
            bit_set, in_output,
            "field `{name}` (bit {i}) presence mismatch: mask bit={bit_set} output_has_field={in_output}\n{src}"
        );
    }
}
