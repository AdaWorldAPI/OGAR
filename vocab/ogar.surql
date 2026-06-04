-- OGAR — Open Graph of Active Record
-- SurrealQL DDL projection of the canonical vocabulary.
--
-- This file defines the SurrealDB tables and field schemas that mirror
-- the `ogar-vocab` Rust types. It is intended as a one-shot apply for
-- a fresh SurrealDB namespace + database, or as a reference for
-- producer code emitting SurrealQL.
--
-- Layer position:  ogar-vocab IR  →  this DDL  →  SurrealDB tables.

DEFINE NAMESPACE IF NOT EXISTS ogar;
USE NS ogar DB ogar;

-- ─────────────────────────────────────────────────────────────────────
-- Source language enum
-- ─────────────────────────────────────────────────────────────────────

DEFINE FIELD language ON TABLE ogar_class TYPE option<string>
    ASSERT $value IS NONE OR $value IN ['Ruby', 'Python', 'Sql', 'TypeScript', 'SurrealQl', 'Elixir', 'Unknown'];

-- ─────────────────────────────────────────────────────────────────────
-- The canonical Class table
-- ─────────────────────────────────────────────────────────────────────

DEFINE TABLE ogar_class SCHEMAFULL;
DEFINE FIELD name         ON TABLE ogar_class TYPE string;
DEFINE FIELD prefix       ON TABLE ogar_class TYPE string;          -- e.g. "ogit-op", "ogit-erp"
DEFINE FIELD parent       ON TABLE ogar_class TYPE option<string>;
DEFINE FIELD mixins       ON TABLE ogar_class TYPE array<string>;
DEFINE FIELD table_name   ON TABLE ogar_class TYPE option<string>;
DEFINE FIELD default_scope ON TABLE ogar_class TYPE option<string>;
DEFINE FIELD ignored_columns ON TABLE ogar_class TYPE array<string>;
DEFINE FIELD scope_predeclarations ON TABLE ogar_class TYPE array<string>;
DEFINE FIELD inheritance_column_disabled ON TABLE ogar_class TYPE bool DEFAULT false;

DEFINE INDEX ogar_class_identity ON TABLE ogar_class COLUMNS prefix, name UNIQUE;

-- ─────────────────────────────────────────────────────────────────────
-- Associations
-- ─────────────────────────────────────────────────────────────────────

DEFINE TABLE ogar_association SCHEMAFULL;
DEFINE FIELD class_ref   ON TABLE ogar_association TYPE record<ogar_class>;
DEFINE FIELD kind        ON TABLE ogar_association TYPE string
    ASSERT $value IN ['BelongsTo', 'HasOne', 'HasMany', 'HasAndBelongsToMany'];
DEFINE FIELD name        ON TABLE ogar_association TYPE string;
DEFINE FIELD class_name  ON TABLE ogar_association TYPE option<string>;
DEFINE FIELD foreign_key ON TABLE ogar_association TYPE option<string>;
DEFINE FIELD polymorphic ON TABLE ogar_association TYPE option<bool>;
DEFINE FIELD through     ON TABLE ogar_association TYPE option<string>;
DEFINE FIELD source      ON TABLE ogar_association TYPE option<string>;
DEFINE FIELD as_target   ON TABLE ogar_association TYPE option<string>;
DEFINE FIELD dependent   ON TABLE ogar_association TYPE option<string>;
DEFINE FIELD optional    ON TABLE ogar_association TYPE option<bool>;
DEFINE FIELD inverse_of  ON TABLE ogar_association TYPE option<string>;
DEFINE FIELD scope_source ON TABLE ogar_association TYPE option<string>;
-- scope_source: Rails has_many :x, -> { where(...) } / Django limit_choices_to /
-- Odoo Many2one(..., domain=[...]) — verbatim lambda body.
DEFINE FIELD before_add    ON TABLE ogar_association TYPE option<string>;
DEFINE FIELD after_add     ON TABLE ogar_association TYPE option<string>;
DEFINE FIELD before_remove ON TABLE ogar_association TYPE option<string>;
DEFINE FIELD after_remove  ON TABLE ogar_association TYPE option<string>;

-- ─────────────────────────────────────────────────────────────────────
-- Enums
-- ─────────────────────────────────────────────────────────────────────

DEFINE TABLE ogar_enum SCHEMAFULL;
DEFINE FIELD class_ref      ON TABLE ogar_enum TYPE record<ogar_class>;
DEFINE FIELD column         ON TABLE ogar_enum TYPE string;
DEFINE FIELD values         ON TABLE ogar_enum TYPE array<object>;  -- {name, value}
DEFINE FIELD scopes_disabled ON TABLE ogar_enum TYPE option<bool>;

-- ─────────────────────────────────────────────────────────────────────
-- Store accessors (JSONB pseudo-fields)
-- ─────────────────────────────────────────────────────────────────────

DEFINE TABLE ogar_store_accessor SCHEMAFULL;
DEFINE FIELD class_ref ON TABLE ogar_store_accessor TYPE record<ogar_class>;
DEFINE FIELD column    ON TABLE ogar_store_accessor TYPE string;
DEFINE FIELD fields    ON TABLE ogar_store_accessor TYPE array<string>;
DEFINE FIELD prefix    ON TABLE ogar_store_accessor TYPE option<bool>;

-- ─────────────────────────────────────────────────────────────────────
-- Attributes (typed-attribute overrides)
-- ─────────────────────────────────────────────────────────────────────

DEFINE TABLE ogar_attribute SCHEMAFULL;
DEFINE FIELD class_ref ON TABLE ogar_attribute TYPE record<ogar_class>;
DEFINE FIELD name      ON TABLE ogar_attribute TYPE string;
DEFINE FIELD type_name ON TABLE ogar_attribute TYPE option<string>;

-- ─────────────────────────────────────────────────────────────────────
-- Scopes
-- ─────────────────────────────────────────────────────────────────────

DEFINE TABLE ogar_scope SCHEMAFULL;
DEFINE FIELD class_ref   ON TABLE ogar_scope TYPE record<ogar_class>;
DEFINE FIELD name        ON TABLE ogar_scope TYPE string;
DEFINE FIELD body_source ON TABLE ogar_scope TYPE string;

-- ─────────────────────────────────────────────────────────────────────
-- Callbacks
-- ─────────────────────────────────────────────────────────────────────

DEFINE TABLE ogar_callback SCHEMAFULL;
DEFINE FIELD class_ref     ON TABLE ogar_callback TYPE record<ogar_class>;
DEFINE FIELD event         ON TABLE ogar_callback TYPE string;
DEFINE FIELD target_method ON TABLE ogar_callback TYPE option<string>;
DEFINE FIELD body_source   ON TABLE ogar_callback TYPE option<string>;

-- ─────────────────────────────────────────────────────────────────────
-- Validations
-- ─────────────────────────────────────────────────────────────────────

DEFINE TABLE ogar_validation SCHEMAFULL;
DEFINE FIELD class_ref   ON TABLE ogar_validation TYPE record<ogar_class>;
DEFINE FIELD target      ON TABLE ogar_validation TYPE string;
DEFINE FIELD rule_source ON TABLE ogar_validation TYPE string;
