//! `ogar-render-askama` — build-time codegen harness over the calcified
//! canonical layer.
//!
//! Structurally a mirror of [`AdaWorldAPI/woa-rs`](https://github.com/AdaWorldAPI/woa-rs)
//! `crates/codegen` (RFC-v02-006): one [`ArtifactKind`] enum dispatched
//! through a per-kind [`ArtifactEmitter`] trait, with one askama template
//! per kind. The canonical input here is [`ogar_vocab::Class`] instead of
//! WoA's `RouteSpec`, but the kit shape is the same.
//!
//! # The 800 → 7-70 collapse
//!
//! The number of templates is bounded by **artifact kind**, never by
//! `(class × target)`. Adding a new canonical concept (e.g. promoting
//! `project_costs`) is one ogar-vocab class fn + zero new templates — the
//! existing kit renders it through. Adding a new target (gremlin, proto,
//! …) is one new [`ArtifactKind`] variant + one askama template; every
//! promoted concept emits through it automatically.
//!
//! # Layering (where this lives in the OGAR stack)
//!
//! ```text
//!   ogar-vocab            (codebook + Class fns)
//!         │
//!         │  pure construction at build time
//!         ▼
//!   ogar-render-askama    (THIS CRATE — askama-bound emitters per kind)
//!         │
//!         │  .rs / .ts / .surql / .json source text
//!         ▼
//!   downstream consumers  (op-codegen-projection, rm-codegen, medcare, …)
//! ```
//!
//! `ClassView` (the **run-time** projection layer in `lance-graph-contract`)
//! is a sibling concern: it materialises a SoA row's render rows at query
//! time. Both pipelines are jinja-templated; both share the N3 field order
//! convention; they consume different shapes. This crate handles the
//! build-time path (typed source emission); `ogar-class-view` handles the
//! run-time path (label-resolved row projection).
//!
//! # Proof-of-shape phase
//!
//! [`ArtifactKind::RustStruct`] has a real askama template + concrete
//! emitter. The other four kinds use [`artifact_kinds::stub::Stub`] —
//! placeholder code that compiles and emits a marker comment so callers
//! can exercise the full pipeline against every promoted concept while
//! T2–T5 templates land in follow-on PRs.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod artifact_kinds;
pub mod spec;

pub use artifact_kinds::{for_kind, ArtifactEmitter};
pub use spec::{ArtifactKind, ArtifactSpec};

use ogar_vocab::Class;

/// Render one artifact in one call. Convenience over
/// [`artifact_kinds::for_kind`] + the emitter trait.
pub fn render(class: &Class, kind: ArtifactKind) -> Result<String, askama::Error> {
    let spec = ArtifactSpec::new(class, kind);
    for_kind(kind).emit(&spec)
}

/// Render every promoted concept for one [`ArtifactKind`], returning
/// `(canonical_concept, source)` pairs. Useful for batch codegen of a
/// full target (e.g. emit every concept as a Rust struct).
///
/// Walks the same 32-concept set [`crate::artifact_kinds`]'s tests use.
/// Concepts without a `canonical_concept` field are skipped.
pub fn render_all(
    classes: &[Class],
    kind: ArtifactKind,
) -> Result<Vec<(String, String)>, askama::Error> {
    let emitter = for_kind(kind);
    let mut out = Vec::with_capacity(classes.len());
    for class in classes {
        let Some(concept) = class.canonical_concept.clone() else {
            continue;
        };
        let spec = ArtifactSpec::new(class, kind);
        let source = emitter.emit(&spec)?;
        out.push((concept, source));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ogar_vocab::{
        billable_work_entry, project, project_actor, project_role, project_work_item,
    };

    #[test]
    fn artifact_kind_all_const_enumerates_every_variant() {
        // Pin: `ArtifactKind::ALL` enumerates every variant. New variants
        // must be appended here (the enum + the const slice both change).
        let all = ArtifactKind::ALL;
        assert!(
            all.contains(&ArtifactKind::RustStruct)
                && all.contains(&ArtifactKind::TsInterface)
                && all.contains(&ArtifactKind::SurrealqlTable)
                && all.contains(&ArtifactKind::OpenapiSchema)
                && all.contains(&ArtifactKind::NodeGuidRoutingArm),
            "ArtifactKind::ALL missing a variant"
        );
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn rust_struct_emits_pub_const_class_id_for_promoted_concept() {
        // Proof of shape: render project_work_item — verify the emitted
        // source declares the right struct name + CLASS_ID + canonical
        // concept.
        let class = project_work_item();
        let src = render(&class, ArtifactKind::RustStruct).unwrap();
        assert!(src.contains("pub struct ProjectWorkItem"), "{src}");
        assert!(
            src.contains("pub const CLASS_ID: u16 = 0x0102;"),
            "expected CLASS_ID = 0x0102 in:\n{src}"
        );
        assert!(
            src.contains("pub const CANONICAL_CONCEPT: &str = \"project_work_item\";"),
            "{src}"
        );
        // Doc comment should reference the class fn name.
        assert!(src.contains("project_work_item()"), "{src}");
    }

    #[test]
    fn rust_struct_emits_family_edges() {
        // billable_work_entry has its 12 family edges; the emitted struct
        // must surface them as fields (Vec<u64> for has_many,
        // Option<u64> for belongs_to/has_one).
        let class = billable_work_entry();
        let src = render(&class, ArtifactKind::RustStruct).unwrap();
        for edge in &class.associations {
            assert!(
                src.contains(&format!("pub {}:", edge.name)),
                "billable_work_entry rust_struct missing family edge `{}`:\n{src}",
                edge.name
            );
        }
    }

    #[test]
    fn rust_struct_emits_typed_attribute_for_each_class_attribute() {
        // project_role has typed attributes (name, position, permissions).
        // Every one must appear in the emitted struct.
        let class = project_role();
        let src = render(&class, ArtifactKind::RustStruct).unwrap();
        for attr in &class.attributes {
            assert!(
                src.contains(&format!("pub {}:", attr.name)),
                "project_role rust_struct missing attribute `{}`:\n{src}",
                attr.name
            );
        }
    }

    #[test]
    fn stub_emits_marker_for_unimplemented_kinds() {
        // The remaining stub kinds compile + emit a marker comment naming
        // the kind + class. T1 (RustStruct) + T2 (TsInterface) now have
        // real emitters; T3 / T4 / T5 are still stubbed (per Northstar §3).
        let class = project();
        for kind in [
            ArtifactKind::SurrealqlTable,
            ArtifactKind::OpenapiSchema,
            ArtifactKind::NodeGuidRoutingArm,
        ] {
            let src = render(&class, kind).unwrap();
            assert!(
                src.contains(kind.name()),
                "stub for {:?} should mention its name:\n{src}",
                kind
            );
            assert!(
                src.contains("Project"),
                "stub should mention the class name:\n{src}"
            );
        }
    }

    #[test]
    fn ts_interface_emits_class_id_and_canonical_concept() {
        // T2 proof of shape: render project_work_item — verify the emitted
        // .ts source declares the right interface name + CLASS_ID const
        // (`as const` for literal-type) + canonical concept const.
        let class = project_work_item();
        let src = render(&class, ArtifactKind::TsInterface).unwrap();
        assert!(
            src.contains("export interface ProjectWorkItem"),
            "ts_interface should declare `export interface ProjectWorkItem`:\n{src}"
        );
        assert!(
            src.contains("export const CLASS_ID = 0x0102 as const;"),
            "expected CLASS_ID = 0x0102 (as const) in:\n{src}"
        );
        assert!(
            src.contains("export const CANONICAL_CONCEPT = \"project_work_item\" as const;"),
            "{src}"
        );
    }

    #[test]
    fn ts_interface_maps_rails_types_to_ts() {
        // Coarse Rails→TS type mapping: integer → number, string → string,
        // boolean → boolean. Pinned against project_role (carries `name`,
        // `position` int, `permissions` text).
        let class = project_role();
        let src = render(&class, ArtifactKind::TsInterface).unwrap();
        // `name: string` (Rails "string")
        assert!(src.contains("name: string;"), "expected `name: string;` in:\n{src}");
        // `position: number` (Rails "integer")
        assert!(
            src.contains("position: number;"),
            "expected `position: number;` in:\n{src}"
        );
        // `permissions: string` (Rails "text")
        assert!(
            src.contains("permissions: string;"),
            "expected `permissions: string;` in:\n{src}"
        );
    }

    #[test]
    fn ts_interface_emits_family_edges_as_arrays_and_nullable_ids() {
        // belongs_to / has_one → `number | null` (nullable FK id)
        // has_many / habtm     → `ReadonlyArray<number>`
        let class = billable_work_entry();
        let src = render(&class, ArtifactKind::TsInterface).unwrap();
        // Every family edge name should appear as a TS property.
        for edge in &class.associations {
            assert!(
                src.contains(&format!("{}: ", edge.name)),
                "ts_interface missing family edge `{}`:\n{src}",
                edge.name
            );
        }
        // At least one of each shape:
        assert!(
            src.contains("number | null"),
            "expected at least one belongs_to/has_one as `number | null`:\n{src}"
        );
        // (billable_work_entry's edges are all belongs_to today, so
        // `ReadonlyArray<number>` may not appear — assert via project_actor
        // which has the `groups` / `users` has_many shape.)
        let actor_src = render(&project_actor(), ArtifactKind::TsInterface).unwrap();
        assert!(
            actor_src.contains("ReadonlyArray<number>"),
            "expected at least one has_many edge as `ReadonlyArray<number>`:\n{actor_src}"
        );
    }

    #[test]
    fn ts_interface_handles_keyword_property_names_safely() {
        // project_actor declares an attribute named `type` (the same hazard
        // that bit Rust on PR #78). TypeScript accepts `type` as a property
        // name in interface bodies, but quoting is the conservative move
        // for any JS reserved word. Per `escape_ts_property`, `type` is in
        // the conservative quote list -> emitted as `"type": string;`.
        let class = project_actor();
        let src = render(&class, ArtifactKind::TsInterface).unwrap();
        assert!(
            src.contains("\"type\": string;"),
            "expected `\"type\": string;` (quoted JS-reserved name) in:\n{src}"
        );
        assert!(
            !src.contains("\n    type: "),
            "unquoted `type:` is a hazard; emitter must quote it:\n{src}"
        );
    }

    #[test]
    fn ts_interface_quotes_dotted_property_names() {
        // Odoo-style identifiers (e.g. `account.move.line`) aren't valid
        // bare TS property names — must be quoted. This is the same logic
        // catching `escape_ts_property`'s non-identifier branch.
        // Use a fabricated class with a dotted attribute name.
        use ogar_vocab::{Attribute, Class as VocabClass, Language};
        let mut c = VocabClass::new("Synth");
        c.canonical_concept = Some("synth".to_string());
        c.language = Language::Unknown;
        let mut a = Attribute::new("account.move.line");
        a.type_name = Some("string".to_string());
        c.attributes = vec![a];
        let src = render(&c, ArtifactKind::TsInterface).unwrap();
        assert!(
            src.contains("\"account.move.line\": string;"),
            "dotted property name must be quoted:\n{src}"
        );
    }

    #[test]
    fn rust_struct_escapes_keyword_attribute_names() {
        // Codex P1 on #78: `project_actor()` declares an attribute named
        // `type` (Rails STI convention). The naive template would emit
        // `pub type: String,` which is illegal Rust. The emitter must
        // raw-escape Rust reserved words so the output compiles.
        let class = project_actor();
        assert!(
            class.attributes.iter().any(|a| a.name == "type"),
            "regression precondition: project_actor must ship a `type` attribute"
        );
        let src = render(&class, ArtifactKind::RustStruct).unwrap();
        // The illegal form must NOT appear ...
        assert!(
            !src.contains("pub type:"),
            "rust_struct must not emit `pub type:` (illegal); got:\n{src}"
        );
        // ... and the raw-escaped form MUST appear.
        assert!(
            src.contains("pub r#type:"),
            "expected raw-escaped `pub r#type:` for the `type` attribute:\n{src}"
        );
    }

    #[test]
    fn render_all_walks_a_slice_of_classes() {
        let classes = vec![project(), project_work_item(), project_role()];
        let out = render_all(&classes, ArtifactKind::RustStruct).unwrap();
        assert_eq!(out.len(), 3);
        let concepts: Vec<&str> = out.iter().map(|(c, _)| c.as_str()).collect();
        assert!(concepts.contains(&"project"));
        assert!(concepts.contains(&"project_work_item"));
        assert!(concepts.contains(&"project_role"));
        for (_, src) in &out {
            assert!(src.contains("pub const CLASS_ID:"), "{src}");
        }
    }
}
