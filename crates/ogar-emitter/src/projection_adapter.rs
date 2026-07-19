//! Read-arm projection codegen — deterministic `impl From<Row> for Domain`
//! scaffolds emitted from a column→field projection recipe (see
//! [`emit_projection_adapters`]).
//!
//! This is the READ twin of [`crate::emit_do_adapters`]: where that module
//! emits the DO/behavioral arm (method scaffolds from `ActionDef`
//! signatures), this module emits the **Pattern A** projection — the
//! private `XRow` (MySQL columns 1:1) → public domain struct mapping that
//! a consumer today hand-writes as `impl From<XRow> for Domain`. The
//! furnace's "no hand-roll" rule (`CODEGEN-PARITY-SESSION-SPLIT.md`) makes
//! this the generator arm that retires that residue.
//!
//! Corpus-agnostic by construction: only the typed [`ProjectionClass`] /
//! [`FieldProjection`] structs live here — the caller builds them from the
//! consumer's real row/domain shapes (or, later, from a ruff harvest of
//! the source read-mapping). Output is fully deterministic (stable sort on
//! classes; field order preserved from the recipe, which is itself the
//! domain struct's declaration order) so it is diffable as a golden
//! artifact across regenerations.
//!
//! # The projection vocabulary
//!
//! Each domain field is filled by exactly one [`FieldSource`] — the closed
//! set of transforms the Pattern-A projections actually use. A field whose
//! transform is not yet in this set is a Core gap: extend the enum
//! deliberately (Core-First), never hand-patch the generated output.
//!
//! # Scope boundary — this arm is Pattern A only (row→domain `From`)
//!
//! This emitter covers the **Pattern A** shape: a named row struct
//! (`#[derive(sqlx::FromRow)] struct XRow`) with a hand `From<XRow> for
//! Domain`. It does NOT cover **Pattern C** — the tuple-fold read
//! (`sqlx::query_as::<(T1, T2, …)>` mapped by positional destructure, no
//! named row struct). The furnace's next entity (Anamnese, worklist row 2)
//! is Pattern C, and a 2026-07-19 harvest of medcare-rs `queries/anamnese.rs`
//! found it needs a *distinct* arm, not an extension of this one:
//!
//! - **positional-tuple destructure** — map columns to tuple *position*
//!   (`|(a, b, c)| Domain { … }`), not to a named row field.
//! - **SQL-side coercion templating** — several fields are `CAST(col AS
//!   CHAR) AS alias` in the query text (numeric FK → String); that lives in
//!   the emitted SQL, not in a post-fetch Rust transform.
//! - **clamp-and-cast** — `col.unwrap_or(0).clamp(0, u8::MAX as i32) as u8`
//!   (a new `FieldSource` variant when that arm is built).
//!
//! And it names three genuine slag outliers to hand-keep, never codegen:
//! `GynAnamnese` (17 cols → sqlx's 16-field tuple cap forces name-keyed
//! `try_get`), `Sexualanamnese` (code→enum matching), and `Alkohol` /
//! `GynAnamnese` array assembly (`[bool; 7]`). This corrects the worklist's
//! "purest metal / id·patient_id·date·text ×14" label: those are query
//! filters, not domain fields. Build the Pattern-C arm when the loop reaches
//! Anamnese (A records the C# fixture first) — do not speculatively pre-build.

use ogar_vocab::canonical_concept_id;

/// One projection = one `impl From<Row> for Domain`, built from the
/// consumer's row + domain shapes.
#[derive(Debug, Clone)]
pub struct ProjectionClass {
    /// The private row type (MySQL-column mirror), e.g. `"PatientRow"`.
    pub row_type: String,
    /// The public domain type, e.g. `"Patient"`.
    pub domain_type: String,
    /// The canonical concept snake_case name (for classid provenance),
    /// e.g. `"patient"`. Empty when the shape is not a minted concept.
    pub concept: String,
    /// Domain fields in declaration order, each with its source recipe.
    pub fields: Vec<FieldProjection>,
}

/// One domain field ← the recipe that fills it from the row.
#[derive(Debug, Clone)]
pub struct FieldProjection {
    /// The domain struct field name being assigned (e.g. `"vorname"`).
    pub domain_field: String,
    /// How the value is produced from the row binding `r`.
    pub source: FieldSource,
}

/// The closed set of column→field transforms. Derived from the real
/// Pattern-A projections in medcare-rs; grows deliberately (never a
/// catch-all string) when a new consumer needs a transform not here.
#[derive(Debug, Clone)]
pub enum FieldSource {
    /// `field: r.col` — same type, direct move.
    Passthrough {
        /// Source column on the row binding.
        column: String,
    },
    /// `field: r.col.unwrap_or_default()` — `Option<T>` → `T`.
    UnwrapOrDefault {
        /// Source column on the row binding.
        column: String,
    },
    /// `field: r.col.map(|dt| dt.date())` — `Option<NaiveDateTime>` →
    /// `Option<NaiveDate>` (drop the always-midnight time component).
    MapNaiveDate {
        /// Source column on the row binding.
        column: String,
    },
    /// `field: r.col.unwrap_or(0) != 0` — `Option<iN>` flag → `bool`.
    IntFlagToBool {
        /// Source column on the row binding.
        column: String,
    },
    /// `field: <literal>` — no backing column (e.g. `None` for a domain
    /// field absent from the live schema).
    Const {
        /// Verbatim Rust expression to assign (e.g. `"None"`).
        literal: String,
    },
}

impl FieldSource {
    /// The right-hand side expression assigned to the field, given the
    /// row binding name `r`.
    fn expr(&self) -> String {
        match self {
            FieldSource::Passthrough { column } => format!("r.{column}"),
            FieldSource::UnwrapOrDefault { column } => format!("r.{column}.unwrap_or_default()"),
            FieldSource::MapNaiveDate { column } => format!("r.{column}.map(|dt| dt.date())"),
            FieldSource::IntFlagToBool { column } => format!("r.{column}.unwrap_or(0) != 0"),
            FieldSource::Const { literal } => literal.clone(),
        }
    }
}

/// Emit a deterministic Rust module of `impl From<Row> for Domain`
/// projection adapters, one per [`ProjectionClass`]. Each impl is stamped
/// with a `// [concept: … class_id: 0x….]` provenance comment resolved
/// from the OGAR codebook (`canonical_concept_id`); a class whose concept
/// is empty or unminted gets a `// [concept: UNMINTED]` marker so the gap
/// is visible in the generated code, never silent.
///
/// The body reproduces the consumer's hand-written projection
/// byte-for-byte in behavior (the transform vocabulary is the closed
/// [`FieldSource`] set), so the value oracle
/// (`medcare-parity-fixtures`) replays green against the generated output.
/// Fully deterministic (classes sorted by domain type; field order is the
/// recipe order) — diffable as a golden artifact.
#[must_use]
pub fn emit_projection_adapters(classes: &[ProjectionClass]) -> String {
    let mut sorted: Vec<&ProjectionClass> = classes.iter().collect();
    sorted.sort_by(|a, b| a.domain_type.cmp(&b.domain_type));

    let mut out = String::new();
    // Line comments (not `//!` inner-doc) so the emitted file is
    // `include!`-safe: a consumer can `include!` this straight into a
    // module (e.g. next to a private row type) as the live `impl`, which
    // an inner-doc attribute at a non-start position would reject (E0753).
    out.push_str("// Generated projection adapters — DO NOT EDIT.\n");
    out.push_str("// Emitted from the ClassView column->field projection recipe (Pattern A).\n");
    out.push('\n');

    let mut minted = 0usize;
    let mut unminted = 0usize;

    for class in &sorted {
        let class_id = if class.concept.is_empty() {
            None
        } else {
            canonical_concept_id(&class.concept)
        };
        match class_id {
            Some(id) => {
                minted += 1;
                out.push_str(&format!(
                    "// [concept: {}  class_id: 0x{id:04X}]\n",
                    class.concept
                ));
            }
            None => {
                unminted += 1;
                let display = if class.concept.is_empty() {
                    "?"
                } else {
                    class.concept.as_str()
                };
                out.push_str(&format!(
                    "// [concept: UNMINTED — \"{display}\" has no codebook class_id]\n"
                ));
            }
        }

        out.push_str(&format!(
            "impl From<{}> for {} {{\n",
            class.row_type, class.domain_type
        ));
        out.push_str(&format!("    fn from(r: {}) -> Self {{\n", class.row_type));
        out.push_str(&format!("        {} {{\n", class.domain_type));
        for f in &class.fields {
            out.push_str(&format!(
                "            {}: {},\n",
                f.domain_field,
                f.source.expr()
            ));
        }
        out.push_str("        }\n");
        out.push_str("    }\n");
        out.push_str("}\n");
        out.push('\n');
    }

    let total = sorted.len();
    out.push_str(&format!(
        "// summary: total={total} minted={minted} unminted={unminted}\n"
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real medcare-rs `PatientRow` -> `Patient` projection
    /// (`medcare-db/src/mysql.rs`), expressed as a recipe. The golden
    /// output below must reproduce that hand-written `impl From` in
    /// behavior field-for-field — this is the Patient furnace shakedown.
    fn patient_projection() -> ProjectionClass {
        use FieldSource::{Const, IntFlagToBool, MapNaiveDate, Passthrough, UnwrapOrDefault};
        let pass = |field: &str, col: &str| FieldProjection {
            domain_field: field.to_string(),
            source: Passthrough {
                column: col.to_string(),
            },
        };
        ProjectionClass {
            row_type: "PatientRow".to_string(),
            domain_type: "Patient".to_string(),
            concept: "patient".to_string(),
            fields: vec![
                pass("id", "id"),
                pass("pid", "pid"),
                pass("p_right", "p_right"),
                FieldProjection {
                    domain_field: "vorname".to_string(),
                    source: UnwrapOrDefault {
                        column: "p_firstname".to_string(),
                    },
                },
                FieldProjection {
                    domain_field: "nachname".to_string(),
                    source: UnwrapOrDefault {
                        column: "p_lastname".to_string(),
                    },
                },
                FieldProjection {
                    domain_field: "geburtsdatum".to_string(),
                    source: MapNaiveDate {
                        column: "p_birth".to_string(),
                    },
                },
                pass("spez", "p_spez"),
                FieldProjection {
                    domain_field: "login".to_string(),
                    source: Const {
                        literal: "None".to_string(),
                    },
                },
                FieldProjection {
                    domain_field: "sex_female".to_string(),
                    source: IntFlagToBool {
                        column: "p_sexf".to_string(),
                    },
                },
                FieldProjection {
                    domain_field: "sex_male".to_string(),
                    source: IntFlagToBool {
                        column: "p_sexm".to_string(),
                    },
                },
                FieldProjection {
                    domain_field: "sex_diverse".to_string(),
                    source: IntFlagToBool {
                        column: "p_sexd".to_string(),
                    },
                },
                pass("title1", "p_title1"),
                pass("title2", "p_title2"),
                pass("country", "p_country"),
                pass("ethnic", "p_ethnic"),
                pass("strasse", "p_street"),
                pass("plz", "p_zip"),
                pass("ort", "p_place"),
                FieldProjection {
                    domain_field: "telefon".to_string(),
                    source: Const {
                        literal: "None".to_string(),
                    },
                },
                pass("mobil", "p_mobil"),
                pass("email", "p_mail"),
                pass("studies", "p_studies"),
                pass("housedoc", "p_housedoc"),
            ],
        }
    }

    #[test]
    fn patient_projection_golden_is_pinned() {
        let out = emit_projection_adapters(&[patient_projection()]);
        let expected = r#"// Generated projection adapters — DO NOT EDIT.
// Emitted from the ClassView column->field projection recipe (Pattern A).

// [concept: patient  class_id: 0x0901]
impl From<PatientRow> for Patient {
    fn from(r: PatientRow) -> Self {
        Patient {
            id: r.id,
            pid: r.pid,
            p_right: r.p_right,
            vorname: r.p_firstname.unwrap_or_default(),
            nachname: r.p_lastname.unwrap_or_default(),
            geburtsdatum: r.p_birth.map(|dt| dt.date()),
            spez: r.p_spez,
            login: None,
            sex_female: r.p_sexf.unwrap_or(0) != 0,
            sex_male: r.p_sexm.unwrap_or(0) != 0,
            sex_diverse: r.p_sexd.unwrap_or(0) != 0,
            title1: r.p_title1,
            title2: r.p_title2,
            country: r.p_country,
            ethnic: r.p_ethnic,
            strasse: r.p_street,
            plz: r.p_zip,
            ort: r.p_place,
            telefon: None,
            mobil: r.p_mobil,
            email: r.p_mail,
            studies: r.p_studies,
            housedoc: r.p_housedoc,
        }
    }
}

// summary: total=1 minted=1 unminted=0
"#;
        assert_eq!(out, expected);
    }

    #[test]
    fn generated_body_matches_handwritten_projection_lines() {
        // The exact RHS fragments the hand-written medcare `From` uses —
        // the behavior contract the value oracle replays against.
        let out = emit_projection_adapters(&[patient_projection()]);
        for needle in [
            "vorname: r.p_firstname.unwrap_or_default(),",
            "geburtsdatum: r.p_birth.map(|dt| dt.date()),",
            "sex_female: r.p_sexf.unwrap_or(0) != 0,",
            "login: None,",
            "telefon: None,",
            "housedoc: r.p_housedoc,",
        ] {
            assert!(out.contains(needle), "missing projected line: {needle}");
        }
    }

    #[test]
    fn output_is_deterministic_across_class_order() {
        let a = ProjectionClass {
            row_type: "ARow".to_string(),
            domain_type: "Aaa".to_string(),
            concept: String::new(),
            fields: vec![FieldProjection {
                domain_field: "x".to_string(),
                source: FieldSource::Passthrough {
                    column: "x".to_string(),
                },
            }],
        };
        let b = ProjectionClass {
            row_type: "BRow".to_string(),
            domain_type: "Bbb".to_string(),
            concept: String::new(),
            fields: vec![FieldProjection {
                domain_field: "y".to_string(),
                source: FieldSource::Passthrough {
                    column: "y".to_string(),
                },
            }],
        };
        let forward = emit_projection_adapters(&[a.clone(), b.clone()]);
        let reversed = emit_projection_adapters(&[b, a]);
        assert_eq!(forward, reversed);
    }

    #[test]
    fn unminted_concept_is_marked_not_silent() {
        let c = ProjectionClass {
            row_type: "WidgetRow".to_string(),
            domain_type: "Widget".to_string(),
            concept: "not_a_real_concept_xyz".to_string(),
            fields: vec![FieldProjection {
                domain_field: "id".to_string(),
                source: FieldSource::Passthrough {
                    column: "id".to_string(),
                },
            }],
        };
        let out = emit_projection_adapters(&[c]);
        assert!(out.contains("// [concept: UNMINTED"));
        assert!(out.contains("unminted=1"));
    }
}
