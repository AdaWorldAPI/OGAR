//! Per-[`ColumnKind`](crate::ColumnKind) cell renderers — one askama
//! sub-template per variant. Called from
//! [`html_list_view`](super::html_list_view) at row-build time to
//! pre-render each cell's body, so the spine template just emits
//! `{{ cell.body_html|safe }}` with no runtime polymorphism.
//!
//! Per Northstar §1.6 (mass-mail templates): each cell template is the
//! smallest bag of variables it needs (the `*Cell` binding-structs
//! below). Different cell kinds don't share a binding-struct.
//!
//! The catalog mirrors Redmine `queries_helper::column_value`'s 12-arm
//! case (see `docs/integration/REDMINE-QUERY-HARVEST.md` §1.4).

use askama::Template;

// ── Plain ────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "dispatch/cell/plain.askama", escape = "html")]
pub(crate) struct PlainCell<'a> {
    pub value: &'a str,
}

// ── IdLink ───────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "dispatch/cell/id_link.askama", escape = "html")]
pub(crate) struct IdLinkCell<'a> {
    pub id: u64,
    pub href: &'a str,
}

// ── PrimaryLink ──────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "dispatch/cell/primary_link.askama", escape = "html")]
pub(crate) struct PrimaryLinkCell<'a> {
    pub label: &'a str,
    pub href: &'a str,
}

// ── RecordRef ────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "dispatch/cell/record_ref.askama", escape = "html")]
pub(crate) struct RecordRefCell<'a> {
    pub label: &'a str,
    pub href: &'a str,
    /// Canonical concept name of the target (e.g. `"project"`); surfaced
    /// in the `title` attribute for accessibility.
    pub target_concept: &'a str,
}

// ── RichText ─────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "dispatch/cell/rich_text.askama", escape = "none")]
pub(crate) struct RichTextCell<'a> {
    /// Already-rendered prose HTML (markdown / textile expanded
    /// upstream; this template just wraps it in `.wiki`).
    pub body: &'a str,
}

// ── ProgressBar ──────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "dispatch/cell/progress_bar.askama", escape = "html")]
pub(crate) struct ProgressBarCell {
    pub pct: u8,
}

// ── RelationList ─────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "dispatch/cell/relation_list.askama", escape = "html")]
pub(crate) struct RelationListCell<'a> {
    pub relations: Vec<RelationEntry<'a>>,
}

pub(crate) struct RelationEntry<'a> {
    pub id: u64,
    pub kind: &'a str,
    pub href: &'a str,
}

// ── Hours ────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "dispatch/cell/hours.askama", escape = "html")]
pub(crate) struct HoursCell<'a> {
    pub hours: &'a str,
    /// Optional link to the underlying time entries (Redmine's
    /// `:spent_hours` links to the report); empty if unlinked.
    pub href: &'a str,
}

// ── AttachmentList ───────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "dispatch/cell/attachment_list.askama", escape = "html")]
pub(crate) struct AttachmentListCell<'a> {
    pub attachments: Vec<AttachmentEntry<'a>>,
}

pub(crate) struct AttachmentEntry<'a> {
    pub filename: &'a str,
    pub href: &'a str,
}

// ── UserList ─────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "dispatch/cell/user_list.askama", escape = "html")]
pub(crate) struct UserListCell<'a> {
    pub users: Vec<UserEntry<'a>>,
}

pub(crate) struct UserEntry<'a> {
    pub name: &'a str,
    pub href: &'a str,
}
