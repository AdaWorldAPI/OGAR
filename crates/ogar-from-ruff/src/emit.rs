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
//! Scalar attributes emit a **concrete wrapper type** mapped from the Odoo
//! constructor carried on `Attribute::type_name` (the ruff `field_type`
//! predicate): `char`/`text`/`html` → `OgStr`, `integer` → `OgInt`,
//! `float` → `OgFloat`, `monetary` → `OgMoney` (Decimal-backed), `boolean` →
//! `OgBool`, `date`/`datetime` → `OgDate`/`OgDateTime`, `binary` → `OgBytes`,
//! `selection` → `OgSelection`, `json` → `OgJson`; an untyped/unknown column
//! falls back to the generic `OgScalar`. See [`og_scalar_type`].
//!
//! Field identifiers that collide with a target-language reserved word are
//! escaped (see [`escape_ident`]) so the emitted source compiles — e.g. an
//! Odoo field named `type` / `ref` becomes `r#type` / `r#ref` in Rust and
//! `@type` / `@ref` in C#. Python source field names cannot be keywords (the
//! Odoo source would not parse), so Python emit needs no escaping.

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

/// Map an Odoo field constructor (lowercased, carried on `Attribute::type_name`
/// as the ruff `field_type` predicate) to the consumer wrapper-contract scalar
/// type. Shared by all three emitters — the type NAMES are identical across
/// languages (§1.6). `None` (type not captured) and any unrecognised
/// constructor fall back to the generic `OgScalar`, so the emit is always
/// well-typed. `monetary` → `OgMoney` is Decimal-backed (the ERP money
/// doctrine), never a float.
fn og_scalar_type(type_name: Option<&str>) -> &'static str {
    match type_name {
        Some("char" | "text" | "html") => "OgStr",
        Some("integer") => "OgInt",
        Some("float") => "OgFloat",
        Some("monetary") => "OgMoney",
        Some("boolean") => "OgBool",
        Some("date") => "OgDate",
        Some("datetime") => "OgDateTime",
        Some("binary" | "image") => "OgBytes",
        Some("selection") => "OgSelection",
        Some("json") => "OgJson",
        _ => "OgScalar",
    }
}

/// Target language for [`escape_ident`].
#[derive(Clone, Copy)]
enum Lang {
    Rust,
    CSharp,
    Python,
}

/// Escape an emitted field/member identifier that collides with a
/// target-language reserved word, so the generated source compiles.
///
/// Odoo field names are verbatim `snake_case`; some (`type`, `ref`, `move`, …)
/// are Rust and/or C# keywords, and a raw `pub type: …` / `public … type` would
/// not compile. Rust uses raw identifiers (`r#type`); the four that cannot be
/// raw (`crate`/`self`/`super`/`Self`) get a trailing `_`. C# prefixes `@`
/// (`@ref`). Python source field names cannot be keywords (the Odoo class body
/// would not parse), so Python returns the name unchanged.
fn escape_ident(name: &str, lang: Lang) -> String {
    match lang {
        Lang::Rust if matches!(name, "crate" | "self" | "super" | "Self") => format!("{name}_"),
        Lang::Rust if is_rust_keyword(name) => format!("r#{name}"),
        Lang::CSharp if is_csharp_keyword(name) => format!("@{name}"),
        _ => name.to_string(),
    }
}

/// Rust strict + reserved keywords (2021 edition + reserved-for-future).
fn is_rust_keyword(s: &str) -> bool {
    matches!(
        s,
        "as" | "break" | "const" | "continue" | "crate" | "dyn" | "else" | "enum" | "extern"
            | "false" | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match" | "mod"
            | "move" | "mut" | "pub" | "ref" | "return" | "self" | "Self" | "static" | "struct"
            | "super" | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while"
            | "async" | "await" | "abstract" | "become" | "box" | "do" | "final" | "macro"
            | "override" | "priv" | "typeof" | "unsized" | "virtual" | "yield" | "try" | "gen"
    )
}

/// C# reserved keywords. Contextual keywords (`type`, `record`, `var`, …) are
/// legal identifiers in C#, so they are deliberately omitted.
fn is_csharp_keyword(s: &str) -> bool {
    matches!(
        s,
        "abstract" | "as" | "base" | "bool" | "break" | "byte" | "case" | "catch" | "char"
            | "checked" | "class" | "const" | "continue" | "decimal" | "default" | "delegate"
            | "do" | "double" | "else" | "enum" | "event" | "explicit" | "extern" | "false"
            | "finally" | "fixed" | "float" | "for" | "foreach" | "goto" | "if" | "implicit"
            | "in" | "int" | "interface" | "internal" | "is" | "lock" | "long" | "namespace"
            | "new" | "null" | "object" | "operator" | "out" | "override" | "params" | "private"
            | "protected" | "public" | "readonly" | "ref" | "return" | "sbyte" | "sealed"
            | "short" | "sizeof" | "stackalloc" | "static" | "string" | "struct" | "switch"
            | "this" | "throw" | "true" | "try" | "typeof" | "uint" | "ulong" | "unchecked"
            | "unsafe" | "ushort" | "using" | "virtual" | "void" | "volatile" | "while"
    )
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
        out.push_str(&format!(
            "    pub {}: {},\n",
            escape_ident(&attr.name, Lang::Rust),
            og_scalar_type(attr.type_name.as_deref()),
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
            "    pub {}: {field_ty},\n",
            escape_ident(&assoc.name, Lang::Rust),
        ));
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
/// Field/member identifiers keep their Odoo `snake_case` spelling (matching
/// [`emit_rust`]'s wire fidelity), reserved-word-escaped via [`escape_ident`]
/// (`@ref`); idiomatic PascalCase member casing is a future refinement on this
/// same seam.
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
            "    public {} {} {{ get; init; }}\n",
            og_scalar_type(attr.type_name.as_deref()),
            escape_ident(&attr.name, Lang::CSharp),
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
            escape_ident(&assoc.name, Lang::CSharp),
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
        out.push_str(&format!(
            "    {}: {}\n",
            escape_ident(&attr.name, Lang::Python),
            og_scalar_type(attr.type_name.as_deref()),
        ));
    }
    for assoc in &cc.class.associations {
        let (target, is_many) = assoc_target(assoc);
        let field_ty = if is_many {
            format!("ToMany[\"{target}\"]")
        } else {
            format!("ToOne[\"{target}\"]")
        };
        out.push_str(&format!(
            "    {}: {field_ty}\n",
            escape_ident(&assoc.name, Lang::Python),
        ));
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

/// `account_move` → `ACCOUNT_MOVE`; `WorkPackage` → `WORK_PACKAGE` (for the
/// `*_CLASSID` const name). Splits on `.`/`_` (Odoo's dotted/underscored
/// names) AND on a lower→upper case transition (Rails' bare PascalCase class
/// names carry no separator at all — `screaming_snake` must still find the
/// word boundary, or every Rails-sourced const collapses to one run-on word
/// like `WORKPACKAGE_CLASSID`). Does not split consecutive uppercase runs
/// (acronyms): `HTTPServer` → `HTTPSERVER` — no Rails/Odoo class name in the
/// corpus is acronym-prefixed, so this is a deliberately narrow rule, not a
/// general camelCase tokenizer.
fn screaming_snake(name: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for ch in name.chars() {
        if ch == '.' || ch == '_' {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            prev_lower = false;
            continue;
        }
        if ch.is_uppercase() && prev_lower {
            out.push('_');
        }
        out.extend(ch.to_uppercase());
        prev_lower = ch.is_lowercase();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mint::compile_graph_python;
    use ogar_vocab::ports::OdooPort;
    use ruff_spo_triplet::{
        AssocDecl, AssocKind, AttrDecl, AttrKind, Field, Function, Model, ModelGraph,
    };

    fn account_move_graph() -> ModelGraph {
        let mut m = Model::new("account_move");
        m.fields.push(Field {
            name: "name".to_string(),
            field_type: Some("char".to_string()),
            ..Default::default()
        });
        // A field whose name is a Rust + C# reserved word — exercises escape_ident.
        m.fields.push(Field {
            name: "ref".to_string(),
            field_type: Some("char".to_string()),
            ..Default::default()
        });
        // No field_type → the OgScalar fallback path.
        m.fields.push(Field {
            name: "narration".to_string(),
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
        // Monetary + computed → OgMoney (Decimal-backed) AND a computed doc line.
        m.fields.push(Field {
            name: "amount_total".to_string(),
            field_type: Some("monetary".to_string()),
            emitted_by: Some("_compute_amount".to_string()),
            depends_on: vec!["line_ids.balance".to_string()],
            ..Default::default()
        });
        m.functions.push(Function {
            name: "_compute_amount".to_string(),
            reads: Vec::new(),
            raises: Vec::new(),
            traverses: Vec::new(),
            ..Default::default()
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
        // Typed scalar: char -> OgStr.
        assert!(rust.contains("pub name: OgStr,"), "got:\n{rust}");
        // Reserved-word field name -> raw identifier r#ref (and still typed).
        assert!(rust.contains("pub r#ref: OgStr,"), "got:\n{rust}");
        // Untyped scalar -> the OgScalar fallback.
        assert!(rust.contains("pub narration: OgScalar,"), "got:\n{rust}");
        // Monetary -> OgMoney (Decimal-backed money doctrine).
        assert!(rust.contains("pub amount_total: OgMoney,"), "got:\n{rust}");
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

    // ───── Rails (compile_graph_ruby) — the convergence proof ─────
    //
    // The pull-back codegen leg (emit_rust/csharp/python) was, before this
    // session, only ever exercised on an Odoo-lifted `CompiledClass`. This
    // fixture proves the SAME emitters run unmodified on a Rails-lifted one
    // (compile_graph_ruby, ruff#38 + this crate's `mint::compile_graph_ruby`),
    // closing the "unproven on Rails" gap named in the OP+Redmine convergence
    // handover (openproject-nexgen-rs .claude/handovers/
    // 2026-06-30-1200-op-redmine-ogar-convergence-assessment.md §4 step 2).

    fn work_package_rail_graph() -> ModelGraph {
        let mut m = Model::new("WorkPackage");
        m.attributes.push(AttrDecl {
            kind: AttrKind::Attribute,
            name: "estimated_hours".to_string(),
            options: vec![("type".to_string(), "integer".to_string())],
        });
        // No "type" option → the OgScalar fallback path, same as Odoo's
        // `narration` case.
        m.attributes.push(AttrDecl {
            kind: AttrKind::Attribute,
            name: "subject".to_string(),
            options: vec![],
        });
        m.associations.push(AssocDecl {
            kind: AssocKind::BelongsTo,
            name: "project".to_string(),
            options: vec![("class_name".to_string(), "\"Project\"".to_string())],
        });
        m.associations.push(AssocDecl {
            kind: AssocKind::HasMany,
            name: "time_entries".to_string(),
            options: vec![],
        });
        let mut g = ModelGraph::new("openproject");
        g.models.push(m);
        g
    }

    #[test]
    fn emits_rust_struct_for_rails_lifted_class() {
        use crate::mint::compile_graph_ruby;
        use ogar_vocab::ports::OpenProjectPort;

        let cc = &compile_graph_ruby::<OpenProjectPort>(&work_package_rail_graph())[0];
        let rust = emit_rust(cc);

        assert!(
            rust.contains("pub const WORK_PACKAGE_CLASSID: u32 = 0x00010102;"),
            "got:\n{rust}"
        ); // exercises the screaming_snake PascalCase fix below
        assert!(rust.contains("pub struct WorkPackage {"));
        // Rails `attribute :estimated_hours, :integer` -> OgInt (the same
        // og_scalar_type table Odoo's `integer` constructor maps through —
        // shared vocabulary across producers, per §1.6).
        assert!(rust.contains("pub estimated_hours: OgInt,"), "got:\n{rust}");
        // Untyped attribute -> OgScalar fallback.
        assert!(rust.contains("pub subject: OgScalar,"), "got:\n{rust}");
        // belongs_to with class_name override -> ToOne<Project>, not
        // ToOne<Project> from the (singular) relation name by coincidence —
        // assert the class_name path specifically by using a relation name
        // that would pascal_case differently if class_name were ignored.
        assert!(
            rust.contains("pub project: ToOne<Project>,"),
            "got:\n{rust}"
        );
        // has_many, no class_name -> pascal_case(time_entries) = TimeEntries.
        assert!(
            rust.contains("pub time_entries: ToMany<TimeEntries>,"),
            "got:\n{rust}"
        );
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
        // Typed scalar: char -> OgStr (init-only property).
        assert!(
            cs.contains("public OgStr name { get; init; }"),
            "got:\n{cs}"
        );
        // Reserved-word field name -> @-escaped C# identifier (and still typed).
        assert!(
            cs.contains("public OgStr @ref { get; init; }"),
            "got:\n{cs}"
        );
        // Untyped scalar -> the OgScalar fallback; monetary -> OgMoney.
        assert!(
            cs.contains("public OgScalar narration { get; init; }"),
            "got:\n{cs}"
        );
        assert!(
            cs.contains("public OgMoney amount_total { get; init; }"),
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
        // Typed scalar: char -> OgStr annotation.
        assert!(py.contains("    name: OgStr"), "got:\n{py}");
        // Reserved-word field name needs NO escaping in Python (Odoo source
        // field names cannot be Python keywords); still typed.
        assert!(py.contains("    ref: OgStr"), "got:\n{py}");
        // Untyped scalar -> OgScalar fallback; monetary -> OgMoney.
        assert!(py.contains("    narration: OgScalar"), "got:\n{py}");
        assert!(py.contains("    amount_total: OgMoney"), "got:\n{py}");
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
            // Typed scalar wrappers (mapped from field_type) are shared vocab.
            assert!(src.contains("OgStr"), "OgStr in every emitter");
            assert!(src.contains("OgMoney"), "OgMoney in every emitter");
            // The untyped fallback is shared too.
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

    #[test]
    fn screaming_snake_splits_bare_pascal_case_rails_names() {
        // Rails class names carry no separator at all (no dots, no
        // underscores) — screaming_snake must find the word boundary from
        // case alone, or every Rails const collapses to one run-on word.
        assert_eq!(screaming_snake("WorkPackage"), "WORK_PACKAGE");
        assert_eq!(screaming_snake("TimeEntry"), "TIME_ENTRY");
        // Already-snake input is unaffected (the original behaviour).
        assert_eq!(screaming_snake("account.move.line"), "ACCOUNT_MOVE_LINE");
        // A single PascalCase word with no internal boundary stays whole.
        assert_eq!(screaming_snake("Project"), "PROJECT");
    }

    #[test]
    fn og_scalar_type_maps_odoo_constructors() {
        assert_eq!(og_scalar_type(Some("char")), "OgStr");
        assert_eq!(og_scalar_type(Some("text")), "OgStr");
        assert_eq!(og_scalar_type(Some("html")), "OgStr");
        assert_eq!(og_scalar_type(Some("integer")), "OgInt");
        assert_eq!(og_scalar_type(Some("float")), "OgFloat");
        assert_eq!(og_scalar_type(Some("monetary")), "OgMoney");
        assert_eq!(og_scalar_type(Some("boolean")), "OgBool");
        assert_eq!(og_scalar_type(Some("date")), "OgDate");
        assert_eq!(og_scalar_type(Some("datetime")), "OgDateTime");
        assert_eq!(og_scalar_type(Some("binary")), "OgBytes");
        assert_eq!(og_scalar_type(Some("selection")), "OgSelection");
        assert_eq!(og_scalar_type(Some("json")), "OgJson");
        // Unknown constructor and absent type both fall back to OgScalar.
        assert_eq!(og_scalar_type(Some("reference")), "OgScalar");
        assert_eq!(og_scalar_type(None), "OgScalar");
    }

    #[test]
    fn escape_ident_per_language_reserved_words() {
        // Rust: raw identifiers for keywords; the four non-raw-able get a suffix.
        assert_eq!(escape_ident("ref", Lang::Rust), "r#ref");
        assert_eq!(escape_ident("type", Lang::Rust), "r#type");
        assert_eq!(escape_ident("move", Lang::Rust), "r#move");
        assert_eq!(escape_ident("self", Lang::Rust), "self_");
        assert_eq!(escape_ident("crate", Lang::Rust), "crate_");
        assert_eq!(escape_ident("amount", Lang::Rust), "amount");
        // C#: @-escape reserved words; contextual keywords (type) stay legal.
        assert_eq!(escape_ident("ref", Lang::CSharp), "@ref");
        assert_eq!(escape_ident("lock", Lang::CSharp), "@lock");
        assert_eq!(escape_ident("type", Lang::CSharp), "type");
        assert_eq!(escape_ident("amount", Lang::CSharp), "amount");
        // Python: no escaping (Odoo source field names cannot be keywords).
        assert_eq!(escape_ident("ref", Lang::Python), "ref");
        assert_eq!(escape_ident("type", Lang::Python), "type");
    }
}
