//! Pull-back emit — the OGAR transpile substrate's *output* leg.
//!
//! [`mint`](crate::mint) is the pull-in half (source → `CompiledClass`); this
//! is the pull-back half: render a [`CompiledClass`] back into a target
//! language's source. The reference emitter here is **Rust**
//! ([`emit_rust`]); it mirrors the role `ogar-adapter-surrealql` plays for
//! SurrealQL DDL. Other targets (`emit_csharp`, `emit_python`, …) follow the
//! same `&CompiledClass -> String` seam.
//!
//! **The wrapper-contract pivot.** The emitted struct does not inline native
//! types — it uses the consumer's thin wrapper-contract types: `OgScalar` for
//! a column, `ToOne<T>` / `ToMany<T>` for a relation. So "the language just
//! needs to put a wrapper contract akin to lance-graph" is literal: a Rust
//! consumer provides those three aliases (its wrapper contract) and the
//! emitted, rail-shaped class compiles. The `classid` is emitted as a `const`
//! — the rail address travels with the class.
//!
//! Scalar attributes currently emit `OgScalar` uniformly: the Odoo field type
//! (`Char` / `Monetary` / …) is not yet carried on the SPO `Field`, so there
//! is nothing to specialise on. When the `field_type` capture lands (the ruff
//! follow-up), `OgScalar` refines to the mapped concrete type with no change
//! to this seam.

use ogar_vocab::AssociationKind;

use crate::mint::CompiledClass;

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
        let target = pascal_case(assoc.class_name.as_deref().unwrap_or(&assoc.name));
        let field_ty = match assoc.kind {
            AssociationKind::BelongsTo | AssociationKind::HasOne => format!("ToOne<{target}>"),
            AssociationKind::HasMany | AssociationKind::HasAndBelongsToMany => {
                format!("ToMany<{target}>")
            }
            // A future AssociationKind defaults to a single typed reference.
            _ => format!("ToOne<{target}>"),
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
    fn pascal_case_handles_dotted_and_underscored() {
        assert_eq!(pascal_case("account.move.line"), "AccountMoveLine");
        assert_eq!(pascal_case("account_move"), "AccountMove");
        assert_eq!(pascal_case("res.partner"), "ResPartner");
        assert_eq!(screaming_snake("account_move"), "ACCOUNT_MOVE");
    }
}
