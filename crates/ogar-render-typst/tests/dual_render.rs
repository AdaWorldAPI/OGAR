//! **The dual-render proof** — the initial-prompt vertical slice, executable:
//!
//! ONE composed document (`DocCompose`) containing typed projection portals —
//! a `User`, a `WorkPackage` (whose summary view rail-hops to its assignee),
//! an `Attachment`, a tesseract-rs `doc.v1` OBSERVATION (participating through
//! `ogar_doc_ir::project` masking — retina → same walk), and one DELETED
//! target exercising the snapshot fallback — is resolved ONCE
//! (`ogar_doc_ir::resolve`, riding `lance_graph_contract::selection`), then
//! rendered through TWO genuinely different projections from the SAME rows:
//!
//! - the askama HTML surface (`ogar_render_askama::render_field_view`, `@live`)
//! - the Typst page source (`ogar_render_typst`, the `@revision`/archival leg)
//!
//! Renderer independence is the assertion: both outputs carry the same facts,
//! both omit the same masked-out fields, and neither renderer touched a
//! domain store.

use ogar_doc_ir::compose::{
    DOC_COMPOSE_VERSION, DocCompose, DocNode, NodeId, ObjectRef, ObjectSlot, ResolutionMode,
    SnapshotRef,
};
use ogar_doc_ir::project;
use ogar_doc_ir::resolve::{
    DocObjectSource, ResolvedBlock, ResolvedSlot, SlotOutcome, resolve_doc,
};
use ogar_doc_ir::{BBoxRail, DOC_IR_VERSION, DocIr, Geometry, Provenance, Rail, TypedField};
use ogar_render_askama::{FieldView, render_field_view};
use ogar_render_typst as typst;

use lance_graph_contract::class_view::{ClassId, ClassView, WideFieldMask};
use lance_graph_contract::ontology::{DisplayTemplate, FieldRef};
use lance_graph_contract::selection::{NamedView, RailGraph, ViewId, ViewRegistry};

// ── the slice's classes ────────────────────────────────────────────────────

const USER: ClassId = 1;
const WORK_PACKAGE: ClassId = 2;
const ATTACHMENT: ClassId = 3;
const DOC_SCAN: ClassId = 4; // the observation document class

struct SliceView;
impl ClassView for SliceView {
    fn fields(&self, class: ClassId) -> &[FieldRef] {
        use std::sync::OnceLock;
        static USER_F: OnceLock<Vec<FieldRef>> = OnceLock::new();
        static WP_F: OnceLock<Vec<FieldRef>> = OnceLock::new();
        static ATT_F: OnceLock<Vec<FieldRef>> = OnceLock::new();
        static SCAN_F: OnceLock<Vec<FieldRef>> = OnceLock::new();
        match class {
            USER => USER_F.get_or_init(|| {
                vec![
                    FieldRef::new("u:name", "name"),
                    FieldRef::new("u:email", "email"),
                    FieldRef::new("u:role", "role"),
                ]
            }),
            WORK_PACKAGE => WP_F.get_or_init(|| {
                vec![
                    FieldRef::new("wp:subject", "subject"),
                    FieldRef::new("wp:status", "status"),
                    FieldRef::new("wp:assignee", "assignee"),
                    FieldRef::new("wp:progress", "progress"),
                ]
            }),
            ATTACHMENT => ATT_F.get_or_init(|| {
                vec![
                    FieldRef::new("att:filename", "filename"),
                    FieldRef::new("att:size", "size"),
                ]
            }),
            _ => SCAN_F.get_or_init(|| {
                vec![
                    FieldRef::new("inv:netto", "netto"),
                    FieldRef::new("inv:ust", "ust"),
                    FieldRef::new("inv:iban", "iban"),
                ]
            }),
        }
    }
    fn template(&self, _class: ClassId) -> DisplayTemplate {
        DisplayTemplate::Card
    }
    fn dolce_category_id(&self, _class: ClassId) -> u8 {
        0
    }
}

// ── the consumer's object graph (live objects + the observation node) ──────

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Alice,
    Wp1,
    Spec,
    Invoice, // the tesseract-rs doc.v1 observation, as a graph node
}

/// The scanned invoice as the pixel retina reported it — a `doc.v1` with
/// `netto` and `iban` READ but `ust` NOT read (absent = cleared presence bit,
/// never a sentinel).
fn invoice_observation() -> DocIr {
    let bbox = BBoxRail {
        tl: Rail { x: 0, y: 0 },
        br: Rail { x: 40, y: 8 },
    };
    DocIr {
        version: DOC_IR_VERSION.to_string(),
        source: Provenance::Ocr,
        geometry: Geometry::Rendered,
        content_sha256: [0x51; 32],
        mime: "image/png".to_string(),
        pages: vec![],
        fields: vec![
            TypedField {
                key: "netto".into(),
                value: "100.00".into(),
                bbox,
                confidence: 214,
            },
            TypedField {
                key: "iban".into(),
                value: "DE00 0000 0000 0000 0000 00".into(),
                bbox,
                confidence: 201,
            },
        ],
    }
}

struct SliceGraph {
    invoice: DocIr,
}

impl RailGraph for SliceGraph {
    type Key = Key;
    fn class_of(&self, key: Key) -> ClassId {
        match key {
            Key::Alice => USER,
            Key::Wp1 => WORK_PACKAGE,
            Key::Spec => ATTACHMENT,
            Key::Invoice => DOC_SCAN,
        }
    }
    fn present_mask(&self, key: Key) -> WideFieldMask {
        match key {
            Key::Alice => WideFieldMask::from(0b111),
            Key::Wp1 => WideFieldMask::from(0b1111),
            Key::Spec => WideFieldMask::from(0b11),
            // The observation's presence comes from the RETINA, through the
            // existing project masking: netto+iban present, ust ABSENT.
            Key::Invoice => project::field_mask(&self.invoice, &SliceView, DOC_SCAN),
        }
    }
    fn rail_target(&self, key: Key, position: u8) -> Option<Key> {
        match (key, position) {
            (Key::Wp1, 2) => Some(Key::Alice), // assignee rail
            _ => None,
        }
    }
}

struct SliceSource {
    graph: SliceGraph,
    names: Vec<(&'static str, ViewId)>,
}

impl DocObjectSource for SliceSource {
    type Graph = SliceGraph;
    fn graph(&self) -> &SliceGraph {
        &self.graph
    }
    fn lookup(&self, target: &ObjectRef, _mode: &ResolutionMode) -> Option<Key> {
        match (target.class.as_str(), target.id.as_str()) {
            ("user", "alice") => Some(Key::Alice),
            ("work-package", "wp1") => Some(Key::Wp1),
            ("attachment", "spec-pdf") => Some(Key::Spec),
            ("drawing", "invoice-scan") => Some(Key::Invoice),
            _ => None, // deleted / unknown → fallback path
        }
    }
    fn value_of(&self, key: Key, position: u8) -> Option<String> {
        match (key, position) {
            (Key::Alice, 0) => Some("Alice Muster".into()),
            (Key::Alice, 1) => Some("alice@example.test".into()),
            (Key::Alice, 2) => Some("engineer".into()),
            (Key::Wp1, 0) => Some("Install fire door".into()),
            (Key::Wp1, 1) => Some("in progress".into()),
            (Key::Wp1, 2) => None, // the assignee RAIL: nested rows, no cell
            (Key::Wp1, 3) => Some("60%".into()),
            (Key::Spec, 0) => Some("fire-door-spec.pdf".into()),
            (Key::Spec, 1) => Some("412 KiB".into()),
            // The observation's values are the retina's typed fields, addressed
            // through the mask — the same project::masked_values surface.
            (Key::Invoice, p) => project::masked_values(&self.graph.invoice, &SliceView, DOC_SCAN)
                .iter()
                .find(|mf| mf.position == p)
                .map(|mf| mf.value.to_string()),
            _ => None,
        }
    }
    fn view_by_name(&self, name: &str) -> Option<ViewId> {
        self.names.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
    }
    fn view_for_class(&self, class: ClassId) -> Option<ViewId> {
        // Nested hops render compactly: a railed-to User appears inline.
        match class {
            USER => self.view_by_name("user.inline"),
            _ => None,
        }
    }
}

// ── the fragment library (the initial prompt's named views) ────────────────

fn views() -> (ViewRegistry, Vec<(&'static str, ViewId)>) {
    let mut r = ViewRegistry::new();
    let mut names = Vec::new();
    let mut reg = |names: &mut Vec<(&'static str, ViewId)>, n, v| {
        let id = r.register(v);
        names.push((n, id));
        id
    };
    reg(
        &mut names,
        "user.inline",
        NamedView::new(USER, WideFieldMask::from(0b001), DisplayTemplate::Card),
    );
    reg(
        &mut names,
        "user.card",
        NamedView::new(USER, WideFieldMask::from(0b111), DisplayTemplate::Card),
    );
    reg(
        &mut names,
        "work_package.inline",
        NamedView::new(
            WORK_PACKAGE,
            WideFieldMask::from(0b0011),
            DisplayTemplate::Card,
        ),
    );
    reg(
        &mut names,
        "work_package.summary",
        NamedView::new(
            WORK_PACKAGE,
            WideFieldMask::from(0b1111),
            DisplayTemplate::Detail,
        ),
    );
    reg(
        &mut names,
        "attachment.inline",
        NamedView::new(ATTACHMENT, WideFieldMask::from(0b01), DisplayTemplate::Card),
    );
    reg(
        &mut names,
        "attachment.preview",
        NamedView::new(ATTACHMENT, WideFieldMask::from(0b11), DisplayTemplate::Card),
    );
    reg(
        &mut names,
        "document.scan",
        NamedView::new(
            DOC_SCAN,
            WideFieldMask::from(0b111),
            DisplayTemplate::Detail,
        ),
    );
    (r, names)
}

// ── the composed document ──────────────────────────────────────────────────

fn slot(app: &str, class: &str, id: &str, view: &str, mode: ResolutionMode) -> ObjectSlot {
    ObjectSlot {
        target: ObjectRef {
            app: app.into(),
            class: class.into(),
            id: id.into(),
        },
        class_view: view.into(),
        field_mask: 0,
        wide_mask_words: vec![],
        resolution: mode,
        fallback: None,
    }
}

fn compose() -> DocCompose {
    let mut nodes = Vec::new();
    let mut push = |n: DocNode| {
        nodes.push(n);
        NodeId((nodes.len() - 1) as u32)
    };

    let heading = push(DocNode::Text {
        text: "Install fire door — work record".into(),
    });
    let t1 = push(DocNode::Text {
        text: "Work item:".into(),
    });
    let s_wp = push(DocNode::ObjectSlot {
        slot: slot(
            "openproject",
            "work-package",
            "wp1",
            "work_package.summary",
            ResolutionMode::Live,
        ),
    });
    let t2 = push(DocNode::Text {
        text: "Reported by:".into(),
    });
    let s_user = push(DocNode::ObjectSlot {
        slot: slot(
            "openproject",
            "user",
            "alice",
            "user.inline",
            ResolutionMode::Live,
        ),
    });
    let s_att = push(DocNode::ObjectSlot {
        slot: slot(
            "openproject",
            "attachment",
            "spec-pdf",
            "attachment.preview",
            ResolutionMode::Live,
        ),
    });
    let t3 = push(DocNode::Text {
        text: "Scanned supplier invoice (pixel retina):".into(),
    });
    let s_scan = push(DocNode::ObjectSlot {
        slot: slot(
            "document",
            "drawing",
            "invoice-scan",
            "document.scan",
            ResolutionMode::Snapshot([0x51; 32]),
        ),
    });
    let s_ghost = push(DocNode::ObjectSlot {
        slot: ObjectSlot {
            fallback: Some(SnapshotRef {
                content_sha256: [0xAB; 32],
            }),
            ..slot(
                "openproject",
                "work-package",
                "wp-deleted",
                "work_package.inline",
                ResolutionMode::Live,
            )
        },
    });
    let p1 = push(DocNode::Paragraph {
        children: vec![t1, s_wp],
    });
    let p2 = push(DocNode::Paragraph {
        children: vec![t2, s_user, s_att],
    });
    let p3 = push(DocNode::Paragraph {
        children: vec![t3, s_scan, s_ghost],
    });
    let section = push(DocNode::Section {
        heading: Some(heading),
        children: vec![p1, p2, p3],
    });
    let root = push(DocNode::Document {
        children: vec![section],
    });

    DocCompose {
        version: DOC_COMPOSE_VERSION.to_string(),
        nodes,
        root,
    }
}

// ── the two renderers over the SAME resolved rows ──────────────────────────

fn slot_rows(rs: &ResolvedSlot) -> Vec<FieldView> {
    match &rs.outcome {
        SlotOutcome::Resolved { fields, .. } => fields
            .iter()
            .map(|f| FieldView {
                position: f.position,
                label: if f.depth == 0 {
                    f.label.clone()
                } else {
                    // nested rows carry their depth in the label, renderer-neutrally
                    format!("{} ({})", f.label, "nested")
                },
                predicate: String::new(),
                value: f.value.clone(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn render_html(blocks: &[ResolvedBlock]) -> String {
    let mut html = String::new();
    for b in blocks {
        match b {
            ResolvedBlock::Heading(t) => html.push_str(&format!("<h1>{t}</h1>\n")),
            ResolvedBlock::Text(t) => html.push_str(&format!("<p>{t}</p>\n")),
            ResolvedBlock::Slot(rs) => match &rs.outcome {
                SlotOutcome::Resolved { class, .. } => {
                    let rows = slot_rows(rs);
                    let title = rows.first().map(|r| r.value.clone()).unwrap_or_default();
                    html.push_str(
                        &render_field_view(*class, &rs.class_view, &rs.uri, &title, &rows, &[])
                            .expect("askama render"),
                    );
                    html.push('\n');
                }
                SlotOutcome::Fallback { content_sha256_hex } => html.push_str(&format!(
                    "<p class=\"fallback\">unresolved {} — snapshot sha256:{}</p>\n",
                    rs.class_view, content_sha256_hex
                )),
                SlotOutcome::Unresolvable => {
                    html.push_str(&format!("<p class=\"unresolvable\">{}</p>\n", rs.uri));
                }
            },
        }
    }
    html
}

fn render_typst(blocks: &[ResolvedBlock]) -> String {
    let mut out = String::new();
    for b in blocks {
        match b {
            ResolvedBlock::Heading(t) => out.push_str(&typst::emit_heading(t)),
            ResolvedBlock::Text(t) => out.push_str(&typst::emit_text(t)),
            ResolvedBlock::Slot(rs) => match &rs.outcome {
                SlotOutcome::Resolved { .. } => {
                    let rows = slot_rows(rs);
                    let title = rows.first().map(|r| r.value.clone()).unwrap_or_default();
                    out.push_str(&typst::emit_field_view(&rs.class_view, &title, &rows));
                }
                SlotOutcome::Fallback { content_sha256_hex } => {
                    out.push_str(&typst::emit_fallback(&rs.class_view, content_sha256_hex));
                }
                SlotOutcome::Unresolvable => {
                    out.push_str(&typst::emit_text(&format!("unresolvable: {}", rs.uri)));
                }
            },
        }
    }
    out
}

// ── THE PROOF ──────────────────────────────────────────────────────────────

#[test]
fn one_document_two_projections_same_facts() {
    let (registry, names) = views();
    let source = SliceSource {
        graph: SliceGraph {
            invoice: invoice_observation(),
        },
        names,
    };

    let doc = compose();
    let resolved = resolve_doc(&doc, &source, &SliceView, &registry, 4).expect("valid composition");

    let html = render_html(&resolved.blocks);
    let page = render_typst(&resolved.blocks);

    println!("──── askama HTML (@live surface) ────\n{html}");
    println!("──── Typst source (paged projection) ────\n{page}");

    // The same facts appear in BOTH projections.
    for fact in [
        "Install fire door",  // wp subject
        "in progress",        // wp status
        "60%",                // wp progress
        "Alice Muster",       // the RAIL HOP: assignee resolved at depth 1
        "fire-door-spec.pdf", // attachment.preview filename
        "412 KiB",            // attachment.preview size
        "100.00",             // observation netto (retina fact)
    ] {
        assert!(html.contains(fact), "HTML missing fact {fact:?}");
        assert!(page.contains(fact), "Typst missing fact {fact:?}");
    }
    // The observation IBAN appears in both (Typst escapes nothing in digits).
    assert!(html.contains("DE00 0000 0000 0000 0000 00"));
    assert!(page.contains("DE00 0000 0000 0000 0000 00"));

    // The fallback is explicit in BOTH projections (deleted target).
    assert!(html.contains(&"ab".repeat(32)), "HTML fallback sha missing");
    assert!(
        page.contains(&"ab".repeat(32)),
        "Typst fallback sha missing"
    );

    // Masking holds in BOTH projections: user.inline selects ONLY name —
    // Alice's email and role never reach either surface.
    assert!(!html.contains("alice@example.test"));
    assert!(!page.contains("alice@example.test"));
    assert!(!html.contains("engineer"));
    assert!(!page.contains("engineer"));

    // Retina presence holds in BOTH: `ust` was never read by the OCR —
    // its bit is clear, so no `ust` row exists anywhere (C2: absence is a
    // cleared bit, not an empty cell).
    assert!(!html.contains(">ust<"), "HTML shows an ust row");
    assert!(!page.contains("[ust]"), "Typst shows an ust row");

    // Document prose survived in order in both.
    for text in [
        "Install fire door — work record",
        "Reported by:",
        "Scanned supplier invoice",
    ] {
        assert!(html.contains(text) || html.contains(&text.replace("—", "&#x2014;")));
        assert!(page.contains(&typst::emit_text(text).trim_end().to_string()));
    }
}

#[test]
fn nested_rail_row_is_depth_one() {
    let (registry, names) = views();
    let source = SliceSource {
        graph: SliceGraph {
            invoice: invoice_observation(),
        },
        names,
    };
    let doc = compose();
    let resolved = resolve_doc(&doc, &source, &SliceView, &registry, 4).unwrap();

    let wp_slot = resolved
        .blocks
        .iter()
        .find_map(|b| match b {
            ResolvedBlock::Slot(rs) if rs.class_view == "work_package.summary" => Some(rs),
            _ => None,
        })
        .expect("wp slot present");
    let SlotOutcome::Resolved { fields, .. } = &wp_slot.outcome else {
        panic!("wp must resolve");
    };
    let alice = fields
        .iter()
        .find(|f| f.value == "Alice Muster")
        .expect("assignee resolved through the rail");
    assert_eq!(alice.depth, 1, "the rail hop is depth 1");
    assert_eq!(alice.label, "name");
    // and the rail position itself contributed no cell of its own
    assert!(
        fields
            .iter()
            .all(|f| !(f.depth == 0 && f.label == "assignee"))
    );
}
