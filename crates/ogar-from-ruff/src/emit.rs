//! Pull-back emit — the OGAR transpile substrate's *output* leg.
//!
//! [`mint`](crate::mint) is the pull-in half (source → `CompiledClass`); this
//! is the pull-back half: render a [`CompiledClass`] back into a target
//! language's source. Three emitters share the `&CompiledClass -> String`
//! seam — [`emit_rust`] (the reference), [`emit_csharp`], and [`emit_python`]
//! — the **codegen mode** of the per-language SDKs (substrate doc §1.6, "Three
//! SDKs, one compiled spine"). They mirror the role `ogar-adapter-surrealql`
//! plays for SurrealQL DDL.
//!
//! **The wrapper-contract pivot.** The emitted class does not inline native
//! types — it uses the consumer's thin wrapper-contract types: `OgScalar` for
//! a column, `ToOne<T>` / `ToMany<T>` for a relation. So "the language just
//! needs to put a wrapper contract akin to lance-graph" is literal: a consumer
//! provides those three aliases (its wrapper contract) and the emitted,
//! rail-shaped class compiles. The `classid` travels with the class as a
//! `const`/`ClassVar`.
//!
//! **All three emitters use the *same* type names** (`OgScalar` / `ToOne` /
//! `ToMany`); only the generic-bracket syntax differs (`<T>` for Rust and C#,
//! `[T]` for Python). That shared vocabulary is exactly what makes an SDK a
//! **mechanical transliteration** of the same compiled spine rather than a
//! re-implementation — the layer-1 / layer-2 story of substrate doc §1.6.
//!
//! Scalar attributes currently emit `OgScalar` uniformly: the Odoo field type
//! (`Char` / `Monetary` / …) is not yet carried on the SPO `Field`, so there
//! is nothing to specialise on. When the `field_type` capture lands (the ruff
//! follow-up), `OgScalar` refines to the mapped concrete type with no change
//! to this seam.

use ogar_vocab::{Association, AssociationKind};

use crate::mint::CompiledClass;

/// Shared relation classifier for every emitter: `(comodel PascalCase, is_many)`.
/// `HasMany` / `HasAndBelongsToMany` → many (`ToMany`); everything else
/// (including a future [`AssociationKind`]) → one (`ToOne`). Only the bracket
/// syntax differs per language (`<T>` Rust/C#, `[T]` Python) — the type *names*
/// are identical, which is what lets an SDK be a mechanical transliteration
/// (substrate doc §1.6).
fn assoc_target(assoc: &Association) -> (String, bool) {
    let target = pascal_case(assoc.class_name.as_deref().unwrap_or(&assoc.name));
    let is_many = matches!(
        assoc.kind,
        AssociationKind::HasMany | AssociationKind::HasAndBelongsToMany
    );
    (target, is_many)
}

/// Emit a [`CompiledClass`] as Rust source: a struct whose fields use the
/// consumer's wrapper-contract types (`OgScalar` / `ToOne` / `ToMany`),
/// prefixed by its rail `classid` const and a facet/concept doc line.
/// Computed fields are emitted as trailing doc lines (the compute behaviour
/// is the "impossible" 15% — it lands as an adapter, not inline codegen).
#[must_use]
pub fn emit_rust(cc: &CompiledClass) -> String {
    let ty = pascal_case(&cc.class.name);
    let mut out = String::new();

    out.push_str(&format!(
        "/// Rail class `{}` — classid `0x{:08X}` (concept `0x{:04X}`).\n",
        cc.class.name,
        cc.facet.facet_classid(),
        cc.facet.facet_classid() as u16,
    ));
    out.push_str(&format!(
        "pub const {}_CLASSID: u32 = 0x{:08X};\n\n",
        screaming_snake(&cc.class.name),
        cc.facet.facet_classid(),
    ));

    out.push_str(&format!("pub struct {ty} {{\n"));
    for attr in &cc.class.attributes {
        out.push_str(&format!("    pub {}: OgScalar,\n", attr.name));
    }
    for assoc in &cc.class.associations {
        let (target, is_many) = assoc_target(assoc);
        let field_ty = if is_many {
            format!("ToMany<{target}>")
        } else {
            format!("ToOne<{target}>")
        };
        out.push_str(&format!("    pub {}: {field_ty},\n", assoc.name));
    }
    out.push_str("}\n");

    for c in &cc.class.computed_fields {
        out.push_str(&format!(
            "// computed: {} <- {}({})\n",
            c.field,
            c.compute_method,
            c.depends.join(", "),
        ));
    }

    out
}

/// Emit a [`CompiledClass`] as **C#** source: a `sealed record` whose members
/// use the C# SDK's wrapper-contract types (`OgScalar` / `ToOne<T>` /
/// `ToMany<T>`), with the rail `classid` as a `public const uint ClassId`. The
/// `<T>` generic syntax is shared with Rust; only Python differs (`[T]`).
/// Computed fields are trailing comments — the compute behaviour is the
/// "impossible" 15 % and lands as an adapter, not inline codegen. This is the
/// codegen mode of the C# SDK (substrate doc §1.6); a host compiles the emitted
/// record into an assembly — the strongest "compiled, not parsed" form.
///
/// Field/member identifiers are emitted **verbatim** (Odoo `snake_case`),
/// matching [`emit_rust`]'s wire fidelity; idiomatic PascalCase member casing
/// is a future refinement on this same seam (cf. the `OgScalar` `field_type`
/// note above).
#[must_use]
pub fn emit_csharp(cc: &CompiledClass) -> String {
    let ty = pascal_case(&cc.class.name);
    let mut out = String::new();

    out.push_str(&format!(
        "/// <summary>Rail class <c>{}</c> — classid 0x{:08X} (concept 0x{:04X}).</summary>\n",
        cc.class.name,
        cc.facet.facet_classid(),
        cc.facet.facet_classid() as u16,
    ));
    out.push_str(&format!("public sealed record {ty}\n{{\n"));
    out.push_str(&format!(
        "    public const uint ClassId = 0x{:08X};\n",
        cc.facet.facet_classid(),
    ));
    for attr in &cc.class.attributes {
        out.push_str(&format!(
            "    public OgScalar {} {{ get; init; }}\n",
            attr.name
        ));
    }
    for assoc in &cc.class.associations {
        let (target, is_many) = assoc_target(assoc);
        let field_ty = if is_many {
            format!("ToMany<{target}>")
        } else {
            format!("ToOne<{target}>")
        };
        out.push_str(&format!(
            "    public {field_ty} {} {{ get; init; }}\n",
            assoc.name
        ));
    }
    for c in &cc.class.computed_fields {
        out.push_str(&format!(
            "    // computed: {} <- {}({})\n",
            c.field,
            c.compute_method,
            c.depends.join(", "),
        ));
    }
    out.push_str("}\n");

    out
}

/// Emit a [`CompiledClass`] as **Python** source: a `@dataclass` whose
/// annotations use the Python SDK's wrapper-contract types (`OgScalar` /
/// `ToOne[T]` / `ToMany[T]`), with the rail `classid` as a `ClassVar[int]`.
/// Python uses `[T]` subscripts (not `<T>`), and comodels are forward-ref
/// strings since they may be defined later in the module. Computed fields are
/// trailing comments (the 15 % adapter). This is the codegen mode of the Python
/// SDK (substrate doc §1.6); CPython compiles the emitted module to bytecode on
/// import — the "cost of an import" made literal.
#[must_use]
pub fn emit_python(cc: &CompiledClass) -> String {
    let ty = pascal_case(&cc.class.name);
    let mut out = String::new();

    out.push_str("@dataclass\n");
    out.push_str(&format!("class {ty}:\n"));
    out.push_str(&format!(
        "    \"\"\"Rail class `{}` — classid 0x{:08X} (concept 0x{:04X}).\"\"\"\n",
        cc.class.name,
        cc.facet.facet_classid(),
        cc.facet.facet_classid() as u16,
    ));
    out.push_str(&format!(
        "    CLASSID: ClassVar[int] = 0x{:08X}\n",
        cc.facet.facet_classid(),
    ));
    for attr in &cc.class.attributes {
        out.push_str(&format!("    {}: OgScalar\n", attr.name));
    }
    for assoc in &cc.class.associations {
        let (target, is_many) = assoc_target(assoc);
        let field_ty = if is_many {
            format!("ToMany[\"{target}\"]")
        } else {
            format!("ToOne[\"{target}\"]")
        };
        out.push_str(&format!("    {}: {field_ty}\n", assoc.name));
    }
    for c in &cc.class.computed_fields {
        out.push_str(&format!(
            "    # computed: {} <- {}({})\n",
            c.field,
            c.compute_method,
            c.depends.join(", "),
        ));
    }

    out
}

/// `account.move` / `account_move` → `AccountMove`. Treats both `.` and `_`
/// as word separators (Odoo dotted comodels and underscore-normalised model
/// names both arrive here).
fn pascal_case(name: &str) -> String {
    name.split(['.', '_'])
        .filter(|seg| !seg.is_empty())
        .map(|seg| {
            let mut chars = seg.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect()
}

/// `account_move` → `ACCOUNT_MOVE` (for the `*_CLASSID` const name).
fn screaming_snake(name: &str) -> String {
    name.split(['.', '_'])
        .filter(|seg| !seg.is_empty())
        .map(str::to_uppercase)
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mint::compile_graph_python;
    use ogar_vocab::ports::OdooPort;
    use ruff_spo_triplet::{Field, Function, Model, ModelGraph};

    fn account_move_graph() -> ModelGraph {
        let mut m = Model::new("account_move");
        m.fields.push(Field {
            name: "name".to_string(),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "partner_id".to_string(),
            target: Some("res.partner".to_string()),
            relation_kind: Some("many2one".to_string()),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "line_ids".to_string(),
            target: Some("account.move.line".to_string()),
            inverse_name: Some("move_id".to_string()),
            relation_kind: Some("one2many".to_string()),
            ..Default::default()
        });
        m.fields.push(Field {
            name: "amount_total".to_string(),
            emitted_by: Some("_compute_amount".to_string()),
            depends_on: vec!["line_ids.balance".to_string()],
            ..Default::default()
        });
        m.functions.push(Function {
            name: "_compute_amount".to_string(),
            reads: Vec::new(),
            raises: Vec::new(),
            traverses: Vec::new(),
        });
        let mut g = ModelGraph::new("odoo");
        g.models.push(m);
        g
    }

    #[test]
    fn emits_rail_struct_with_wrapper_contract_types() {
        let cc = &compile_graph_python::<OdooPort>(&account_move_graph())[0];
        let rust = emit_rust(cc);

        // The rail address travels as a const.
        assert!(rust.contains("pub const ACCOUNT_MOVE_CLASSID: u32 = 0x00020202;"));
        // The struct is PascalCase.
        assert!(rust.contains("pub struct AccountMove {"));
        // Scalar -> the wrapper's OgScalar.
        assert!(rust.contains("pub name: OgScalar,"));
        // Many2one -> ToOne<comodel>; One2many -> ToMany<comodel>.
        assert!(
            rust.contains("pub partner_id: ToOne<ResPartner>,"),
            "got:\n{rust}"
        );
        assert!(
            rust.contains("pub line_ids: ToMany<AccountMoveLine>,"),
            "got:\n{rust}"
        );
        // Computed behaviour is a doc line (the 15% lands as an adapter).
        assert!(rust.contains("// computed: amount_total <- _compute_amount(line_ids.balance)"));
    }

    #[test]
    fn emits_csharp_record_with_wrapper_contract_types() {
        let cc = &compile_graph_python::<OdooPort>(&account_move_graph())[0];
        let cs = emit_csharp(cc);

        // The rail address travels as a const inside the record.
        assert!(
            cs.contains("public const uint ClassId = 0x00020202;"),
            "got:\n{cs}"
        );
        // The type is a PascalCase sealed record.
        assert!(
            cs.contains("public sealed record AccountMove"),
            "got:\n{cs}"
        );
        // Scalar -> the wrapper's OgScalar (init-only property).
        assert!(
            cs.contains("public OgScalar name { get; init; }"),
            "got:\n{cs}"
        );
        // Many2one -> ToOne<comodel>; One2many -> ToMany<comodel> (shared <T> syntax).
        assert!(
            cs.contains("public ToOne<ResPartner> partner_id { get; init; }"),
            "got:\n{cs}"
        );
        assert!(
            cs.contains("public ToMany<AccountMoveLine> line_ids { get; init; }"),
            "got:\n{cs}"
        );
        // Computed behaviour is a comment (the 15% lands as an adapter).
        assert!(
            cs.contains("// computed: amount_total <- _compute_amount(line_ids.balance)"),
            "got:\n{cs}"
        );
    }

    #[test]
    fn emits_python_dataclass_with_wrapper_contract_types() {
        let cc = &compile_graph_python::<OdooPort>(&account_move_graph())[0];
        let py = emit_python(cc);

        // The rail address travels as a ClassVar.
        assert!(
            py.contains("CLASSID: ClassVar[int] = 0x00020202"),
            "got:\n{py}"
        );
        // A PascalCase @dataclass.
        assert!(py.contains("@dataclass"), "got:\n{py}");
        assert!(py.contains("class AccountMove:"), "got:\n{py}");
        // Scalar -> OgScalar annotation.
        assert!(py.contains("    name: OgScalar"), "got:\n{py}");
        // Relations use [T] subscripts with forward-ref comodels (not <T>).
        assert!(
            py.contains("    partner_id: ToOne[\"ResPartner\"]"),
            "got:\n{py}"
        );
        assert!(
            py.contains("    line_ids: ToMany[\"AccountMoveLine\"]"),
            "got:\n{py}"
        );
        // Computed behaviour is a comment (the 15% adapter).
        assert!(
            py.contains("# computed: amount_total <- _compute_amount(line_ids.balance)"),
            "got:\n{py}"
        );
    }

    #[test]
    fn all_three_emitters_share_the_same_type_vocabulary() {
        // §1.6: the SDK is a transliteration — same type NAMES, only bracket
        // syntax differs. Assert the shared vocabulary across all three.
        let cc = &compile_graph_python::<OdooPort>(&account_move_graph())[0];
        for src in [emit_rust(cc), emit_csharp(cc), emit_python(cc)] {
            assert!(src.contains("OgScalar"), "OgScalar in every emitter");
            assert!(src.contains("ToOne"), "ToOne in every emitter");
            assert!(src.contains("ToMany"), "ToMany in every emitter");
            // The same rail classid concept in every emitter.
            assert!(
                src.contains("0x00020202"),
                "classid travels in every emitter"
            );
        }
    }

    #[test]
    fn pascal_case_handles_dotted_and_underscored() {
        assert_eq!(pascal_case("account.move.line"), "AccountMoveLine");
        assert_eq!(pascal_case("account_move"), "AccountMove");
        assert_eq!(pascal_case("res.partner"), "ResPartner");
        assert_eq!(screaming_snake("account_move"), "ACCOUNT_MOVE");
    }
}
