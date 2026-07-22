//! Write-arm CRUD codegen — the **write twin** of [`super::projection_adapter`]
//! (`emit_projection_adapters`, the read arm). Where the read arm emits
//! `impl From<Row> for Domain` (column -> field), this arm emits the inverse:
//! the persistence-leaf fns (field -> column) —
//! `INSERT … VALUES … last_insert_id`, `UPDATE … SET … WHERE id = ?`, and the
//! soft-delete tombstone — that a consumer otherwise hand-writes in its DB
//! layer.
//!
//! **This is NOT the signature-only DO arm** ([`super::do_adapter`], whose
//! bodies are `unimplemented!()` because *thinking* bodies live upstream). A
//! CRUD statement is a mechanical persistence leaf, exactly the regular,
//! falsifiable body the read arm already proves is emittable. So this is a
//! *real-body* emitter, a sibling of `emit_projection_adapters`.
//!
//! Corpus-agnostic by construction: this module only consumes typed
//! [`InsertClass`] / [`UpdateClass`] / [`SoftDeleteClass`] recipes the caller
//! builds; no domain data lives here. Output is deterministic (recipe order
//! preserved, no hidden sort) so it is diffable as a golden artifact across
//! regenerations — and a consumer byte-guards its committed generated file
//! against a fresh emit.
//!
//! The generated fns assume the ambient names a DB layer provides at the
//! `include!` site — `MySqlPool`, `DbResult`, `sqlx`, and the recipe's
//! `input_type` — exactly as the read arm's `impl From<Row>` assumes `Row` /
//! `Domain` are in scope. The emitter targets sqlx's `MySqlPool` idiom (the
//! shape the reference consumer, medcare-rs, uses).

/// One column's value in a write statement: either a bound `?` placeholder fed
/// from an `input.<field>` expression, or a literal SQL constant (e.g. the `0`
/// a create writes into a `deleted` flag) that takes no bind.
#[derive(Debug, Clone)]
pub enum WriteValue {
    /// A `?` placeholder; `expr` is the bind expression, e.g. `input.spo2`.
    Bind {
        /// The Rust bind expression spliced into `.bind(<expr>)`.
        expr: String,
    },
    /// A literal in the `VALUES(...)` / `SET` clause, no bind, e.g. `0`.
    Literal {
        /// The literal SQL text.
        sql: String,
    },
}

/// One `(column, value)` pair of a write statement, in wire order. Order is
/// meaningful — the emitted column list, `VALUES`/`SET` list, and bind chain
/// all follow it — so the generated query is byte-stable.
#[derive(Debug, Clone)]
pub struct WriteColumn {
    /// The SQL column name.
    pub column: String,
    /// How this column's value is produced.
    pub value: WriteValue,
}

impl WriteColumn {
    /// A `?`-bound column fed from `input.<field>`.
    #[must_use]
    pub fn bind(column: &str, field: &str) -> Self {
        Self {
            column: column.to_string(),
            value: WriteValue::Bind {
                expr: format!("input.{field}"),
            },
        }
    }

    /// A literal-constant column (no bind), e.g. `WriteColumn::lit("deleted", "0")`.
    #[must_use]
    pub fn lit(column: &str, sql: &str) -> Self {
        Self {
            column: column.to_string(),
            value: WriteValue::Literal {
                sql: sql.to_string(),
            },
        }
    }

    /// A `?`-bound column with an arbitrary bind expression (the escape hatch;
    /// prefer the typed constructors below so recipes stay declarative).
    #[must_use]
    pub fn bind_expr(column: &str, expr: &str) -> Self {
        Self {
            column: column.to_string(),
            value: WriteValue::Bind {
                expr: expr.to_string(),
            },
        }
    }

    /// A `?`-bound column cast to `ty`: `input.<field> as <ty>` (e.g. a
    /// `u16`-in-DTO -> `int`-in-schema cast).
    #[must_use]
    pub fn bind_cast(column: &str, field: &str, ty: &str) -> Self {
        Self::bind_expr(column, &format!("input.{field} as {ty}"))
    }

    /// A `?`-bound `Option` mapped through a cast:
    /// `input.<field>.map(|p| p as <ty>)`.
    #[must_use]
    pub fn bind_opt_cast(column: &str, field: &str, ty: &str) -> Self {
        Self::bind_expr(column, &format!("input.{field}.map(|p| p as {ty})"))
    }

    /// A `?`-bound column cloned for an owned bind: `input.<field>.clone()`
    /// (for `String` / `Vec` where the input is borrowed).
    #[must_use]
    pub fn bind_clone(column: &str, field: &str) -> Self {
        Self::bind_expr(column, &format!("input.{field}.clone()"))
    }

    /// A `?`-bound `bool` rendered as an int flag:
    /// `if input.<field> { 1i8 } else { 0i8 }` — the write inverse of the read
    /// arm's `FieldSource::IntFlagToBool`.
    #[must_use]
    pub fn bind_bool_flag(column: &str, field: &str) -> Self {
        Self::bind_expr(
            column,
            &format!("if input.{field} {{ 1i8 }} else {{ 0i8 }}"),
        )
    }

    fn is_bind(&self) -> bool {
        matches!(self.value, WriteValue::Bind { .. })
    }
}

/// One INSERT (`C`) fn to emit: `INSERT INTO <table> (<cols>) VALUES (<vals>)`
/// returning the AUTO_INCREMENT id.
#[derive(Debug, Clone)]
pub struct InsertClass {
    /// The generated fn name, e.g. `lung_create`.
    pub fn_name: String,
    /// The `///` doc (may be multi-line; `\n`-split into `///` lines).
    pub doc: String,
    /// The target SQL table.
    pub table: String,
    /// The input DTO type, e.g. `LungInput`.
    pub input_type: String,
    /// The columns in wire order (binds and literals interleaved).
    pub columns: Vec<WriteColumn>,
}

impl InsertClass {
    /// The runtime SQL string, whitespace-canonical (the parity contract: two
    /// source formattings producing this same string are the same query).
    #[must_use]
    pub fn sql(&self) -> String {
        let cols = join_columns(&self.columns);
        let vals = self
            .columns
            .iter()
            .map(|c| match &c.value {
                WriteValue::Bind { .. } => "?".to_string(),
                WriteValue::Literal { sql } => sql.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("INSERT INTO {} ({cols}) VALUES ({vals})", self.table)
    }
}

/// One UPDATE (`U`) fn to emit: `UPDATE <table> SET <col = ?…> WHERE id = ?
/// [AND COALESCE(<soft_delete_col>, 0) = 0]`, returning `DbResult<()>`.
#[derive(Debug, Clone)]
pub struct UpdateClass {
    /// The generated fn name, e.g. `lung_update`.
    pub fn_name: String,
    /// The `///` doc (may be multi-line).
    pub doc: String,
    /// The target SQL table.
    pub table: String,
    /// The input DTO type.
    pub input_type: String,
    /// The `SET` assignments in order.
    pub set_columns: Vec<WriteColumn>,
    /// When `Some(col)`, the `WHERE id = ?` also carries
    /// `AND COALESCE(<col>, 0) = 0` (the soft-delete-aware update). `None`
    /// updates regardless of tombstone state.
    pub not_deleted_column: Option<String>,
}

impl UpdateClass {
    /// The runtime SQL string, whitespace-canonical.
    #[must_use]
    pub fn sql(&self) -> String {
        let sets = self
            .set_columns
            .iter()
            .map(|c| match &c.value {
                WriteValue::Bind { .. } => format!("{} = ?", c.column),
                WriteValue::Literal { sql } => format!("{} = {}", c.column, sql),
            })
            .collect::<Vec<_>>()
            .join(", ");
        let filter = match &self.not_deleted_column {
            Some(col) => format!(" AND COALESCE({col}, 0) = 0"),
            None => String::new(),
        };
        format!("UPDATE {} SET {sets} WHERE id = ?{filter}", self.table)
    }
}

/// One soft-delete (`D`) fn to emit: the tombstone update
/// `UPDATE <table> SET <flag_col> = 1, <date_col> = NOW() WHERE id = ?`,
/// returning `DbResult<()>`.
#[derive(Debug, Clone)]
pub struct SoftDeleteClass {
    /// The generated fn name, e.g. `lung_soft_delete`.
    pub fn_name: String,
    /// The `///` doc.
    pub doc: String,
    /// The target SQL table.
    pub table: String,
    /// The tombstone flag column set to `1` (e.g. `pf_delete`).
    pub flag_column: String,
    /// The tombstone timestamp column set to `NOW()` (e.g. `pf_delete_date`).
    pub date_column: String,
}

impl SoftDeleteClass {
    /// The runtime SQL string.
    #[must_use]
    pub fn sql(&self) -> String {
        format!(
            "UPDATE {} SET {} = 1, {} = NOW() WHERE id = ?",
            self.table, self.flag_column, self.date_column
        )
    }
}

fn join_columns(cols: &[WriteColumn]) -> String {
    cols.iter()
        .map(|c| c.column.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Emit a `///` doc block, one line per `\n`-separated segment.
fn emit_doc_lines(doc: &str, out: &mut String) {
    for line in doc.split('\n') {
        out.push_str(&format!("/// {line}\n"));
    }
}

/// Emit a deterministic Rust module of `*_create` INSERT fns from the recipes.
/// Recipe order preserved (no hidden sort) so the output is a diffable golden
/// artifact.
#[must_use]
pub fn emit_insert_adapters(classes: &[InsertClass]) -> String {
    let mut out = String::new();
    out.push_str("// Generated INSERT adapters — DO NOT EDIT.\n");
    out.push_str("// Emitted from the field->column INSERT recipe (ogar-emitter write arm).\n");
    out.push('\n');

    for class in classes {
        emit_insert_one(class, &mut out);
        out.push('\n');
    }

    out.push_str(&format!("// summary: total={}\n", classes.len()));
    out
}

fn emit_insert_one(c: &InsertClass, out: &mut String) {
    out.push_str(&format!("// [table: {}]\n", c.table));
    emit_doc_lines(&c.doc, out);
    out.push_str(&format!(
        "pub async fn {}(pool: &MySqlPool, input: &{}) -> DbResult<i64> {{\n",
        c.fn_name, c.input_type
    ));
    out.push_str("    let res = sqlx::query(\n");
    out.push_str(&format!("        \"{}\",\n", c.sql()));
    out.push_str("    )\n");
    for col in c.columns.iter().filter(|c| c.is_bind()) {
        if let WriteValue::Bind { expr } = &col.value {
            out.push_str(&format!("    .bind({expr})\n"));
        }
    }
    out.push_str("    .execute(pool)\n");
    out.push_str("    .await?;\n");
    out.push_str("    Ok(res.last_insert_id() as i64)\n");
    out.push_str("}\n");
}

/// Emit a deterministic Rust module of `*_update` + `*_soft_delete` fns.
/// Updates first (recipe order), then soft-deletes; same golden-artifact
/// determinism as [`emit_insert_adapters`].
#[must_use]
pub fn emit_update_adapters(updates: &[UpdateClass], soft_deletes: &[SoftDeleteClass]) -> String {
    let mut out = String::new();
    out.push_str("// Generated UPDATE + soft-delete adapters — DO NOT EDIT.\n");
    out.push_str("// Emitted from the field->column write recipe (ogar-emitter write arm).\n");
    out.push('\n');

    for u in updates {
        emit_update_one(u, &mut out);
        out.push('\n');
    }
    for d in soft_deletes {
        emit_soft_delete_one(d, &mut out);
        out.push('\n');
    }

    out.push_str(&format!(
        "// summary: updates={} soft_deletes={}\n",
        updates.len(),
        soft_deletes.len()
    ));
    out
}

fn emit_update_one(c: &UpdateClass, out: &mut String) {
    out.push_str(&format!("// [table: {}]\n", c.table));
    emit_doc_lines(&c.doc, out);
    out.push_str(&format!(
        "pub async fn {}(pool: &MySqlPool, id: i64, input: &{}) -> DbResult<()> {{\n",
        c.fn_name, c.input_type
    ));
    out.push_str("    sqlx::query(\n");
    out.push_str(&format!("        \"{}\",\n", c.sql()));
    out.push_str("    )\n");
    for col in c.set_columns.iter().filter(|c| c.is_bind()) {
        if let WriteValue::Bind { expr } = &col.value {
            out.push_str(&format!("    .bind({expr})\n"));
        }
    }
    out.push_str("    .bind(id)\n");
    out.push_str("    .execute(pool)\n");
    out.push_str("    .await?;\n");
    out.push_str("    Ok(())\n");
    out.push_str("}\n");
}

fn emit_soft_delete_one(c: &SoftDeleteClass, out: &mut String) {
    out.push_str(&format!("// [table: {}]\n", c.table));
    emit_doc_lines(&c.doc, out);
    out.push_str(&format!(
        "pub async fn {}(pool: &MySqlPool, id: i64) -> DbResult<()> {{\n",
        c.fn_name
    ));
    out.push_str("    sqlx::query(\n");
    out.push_str(&format!("        \"{}\",\n", c.sql()));
    out.push_str("    )\n");
    out.push_str("    .bind(id)\n");
    out.push_str("    .execute(pool)\n");
    out.push_str("    .await?;\n");
    out.push_str("    Ok(())\n");
    out.push_str("}\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    // Synthetic, domain-agnostic recipes (the Core carries no consumer data).
    // A `t_event` table: id + patient_id + level(int cast) + note(String) +
    // active(bool flag) + a `deleted` tombstone flag.
    fn event_insert() -> InsertClass {
        InsertClass {
            fn_name: "event_create".to_string(),
            doc: "INSERT a `t_event` row. Returns the AUTO_INCREMENT id.".to_string(),
            table: "t_event".to_string(),
            input_type: "EventInput".to_string(),
            columns: vec![
                WriteColumn::bind("pid", "patient_id"),
                WriteColumn::bind_cast("level", "level", "i32"),
                WriteColumn::bind_clone("note", "note"),
                WriteColumn::bind_bool_flag("active", "is_active"),
                WriteColumn::bind_opt_cast("score", "score", "i32"),
                WriteColumn::lit("deleted", "0"),
            ],
        }
    }

    fn event_update() -> UpdateClass {
        UpdateClass {
            fn_name: "event_update".to_string(),
            doc: "UPDATE a `t_event` row by id.".to_string(),
            table: "t_event".to_string(),
            input_type: "EventInput".to_string(),
            set_columns: vec![
                WriteColumn::bind("pid", "patient_id"),
                WriteColumn::bind_cast("level", "level", "i32"),
            ],
            not_deleted_column: Some("deleted".to_string()),
        }
    }

    fn event_soft_delete() -> SoftDeleteClass {
        SoftDeleteClass {
            fn_name: "event_soft_delete".to_string(),
            doc: "Soft-delete: `deleted = 1, deleted_at = NOW()`. Idempotent.".to_string(),
            table: "t_event".to_string(),
            flag_column: "deleted".to_string(),
            date_column: "deleted_at".to_string(),
        }
    }

    #[test]
    fn insert_sql_and_binds_are_consistent() {
        let c = event_insert();
        assert_eq!(
            c.sql(),
            "INSERT INTO t_event (pid, level, note, active, score, deleted) \
             VALUES (?, ?, ?, ?, ?, 0)"
        );
        let out = emit_insert_adapters(std::slice::from_ref(&c));
        // one `.bind(` per `?` placeholder
        assert_eq!(out.matches(".bind(").count(), c.sql().matches('?').count());
        for needle in [
            "pub async fn event_create(pool: &MySqlPool, input: &EventInput) -> DbResult<i64> {",
            "    .bind(input.patient_id)",
            "    .bind(input.level as i32)",
            "    .bind(input.note.clone())",
            "    .bind(if input.is_active { 1i8 } else { 0i8 })",
            "    .bind(input.score.map(|p| p as i32))",
            "    Ok(res.last_insert_id() as i64)",
        ] {
            assert!(out.contains(needle), "missing: {needle}");
        }
    }

    #[test]
    fn update_carries_id_bind_last_and_soft_delete_filter() {
        let out = emit_update_adapters(&[event_update()], &[]);
        assert!(out.contains(
            "pub async fn event_update(pool: &MySqlPool, id: i64, input: &EventInput) -> DbResult<()> {"
        ));
        assert!(out.contains(
            "        \"UPDATE t_event SET pid = ?, level = ? WHERE id = ? AND COALESCE(deleted, 0) = 0\","
        ));
        // `.bind(id)` is the last bind, after the SET binds.
        let id_at = out.find(".bind(id)").expect("id bind present");
        let level_at = out.find(".bind(input.level as i32)").expect("level bind");
        assert!(level_at < id_at, "id must bind after the SET columns");
        assert!(out.contains("    Ok(())"));
    }

    #[test]
    fn update_without_filter_has_no_coalesce() {
        let u = UpdateClass {
            not_deleted_column: None,
            ..event_update()
        };
        assert!(u.sql().ends_with("WHERE id = ?"));
        assert!(!u.sql().contains("COALESCE"));
    }

    #[test]
    fn soft_delete_is_the_tombstone_shape() {
        let out = emit_update_adapters(&[], &[event_soft_delete()]);
        assert!(out.contains(
            "pub async fn event_soft_delete(pool: &MySqlPool, id: i64) -> DbResult<()> {"
        ));
        assert!(out.contains(
            "        \"UPDATE t_event SET deleted = 1, deleted_at = NOW() WHERE id = ?\","
        ));
    }

    #[test]
    fn multi_line_doc_becomes_multi_line_doc_comment() {
        let u = UpdateClass {
            doc: "line one.\nline two.".to_string(),
            ..event_update()
        };
        let out = emit_update_adapters(&[u], &[]);
        assert!(out.contains("/// line one.\n/// line two.\n"));
    }

    #[test]
    fn emit_is_deterministic_and_order_preserving() {
        let a = emit_update_adapters(&[event_update()], &[event_soft_delete()]);
        let b = emit_update_adapters(&[event_update()], &[event_soft_delete()]);
        assert_eq!(a, b);
        assert!(a.contains("// summary: updates=1 soft_deletes=1"));
        // updates emit before soft-deletes
        let upd = a.find("fn event_update").unwrap();
        let del = a.find("fn event_soft_delete").unwrap();
        assert!(upd < del);
    }
}
