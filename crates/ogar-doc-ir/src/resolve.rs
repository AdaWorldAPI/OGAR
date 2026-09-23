//! `resolve` — the **ObjectSlot resolver**: a composed document
//! ([`DocCompose`]) becomes a renderer-neutral [`ResolvedDoc`] by resolving
//! every slot through the contract's rail walk
//! (`lance_graph_contract::selection::walk_rails`).
//!
//! # What this closes
//!
//! The composition brick ([`crate::compose`]) stores *addresses and projection
//! choices*; this module is the missing verb: **resolve**. Each
//! [`ObjectSlot`] resolves to its live object (or its snapshot fallback),
//! projects it through its named view (`ClassView × WideFieldMask` — the
//! contract's `ViewRegistry` masks), and emits plain `(label, value)` rows
//! that ANY renderer consumes — the askama HTML surface and the Typst page
//! drink from the same [`ResolvedDoc`]. The ActionText discipline holds end
//! to end: the document stored a typed reference; the object's CURRENT state
//! is read at resolve time; a missing object renders its explicit fallback.
//!
//! # Dependency inversion (house shape)
//!
//! The resolver owns none of the domain: the consumer implements
//! [`DocObjectSource`] (its object graph via the contract's `RailGraph`, its
//! value store, its view bindings), exactly as `PlannerContract` /
//! `MailboxSoaView` invert. An observation document (a tesseract-rs `doc.v1`)
//! participates by being a NODE of the consumer's graph whose presence/values
//! come from [`crate::project`] (`field_mask` / `masked_values`) — the retina
//! and the live objects flow through the SAME walk.
//!
//! # Nesting = masks × the walk (operator-ruled)
//!
//! There is no query document anywhere: a slot names a view (a persisted mask
//! constant); nested projection is the rail walk — at each hop the target's
//! own classid resolves its own view via [`DocObjectSource::view_for_class`].
//! The slot's wire masks (`field_mask`/`wide_mask_words`) are the persisted
//! copy of the named view's recipe; the registry is authoritative at resolve
//! time (persisted-query semantics). Slot-local narrowing is future `DocOp`
//! territory, deliberately not invented here.

use lance_graph_contract::class_view::{ClassId, ClassView};
use lance_graph_contract::selection::{RailGraph, ViewId, ViewRegistry, walk_rails};

use crate::compose::{
    ComposeError, DocCompose, DocNode, NodeId, ObjectRef, ObjectSlot, ResolutionMode,
    format_ogar_uri,
};

/// One projected field of a resolved slot — renderer-neutral `(label, value)`
/// plus its mask `position` (the layout address) and rail-walk `depth`
/// (root = 0; a nested `assignee { name }` row carries depth 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedField {
    /// The field's mask position — its layout address.
    pub position: u8,
    /// Nesting depth (root object = 0; each rail hop adds 1).
    pub depth: usize,
    /// Display label, late-resolved from the `ClassView` field basis.
    pub label: String,
    /// Formatted value text, read from the consumer's store at resolve time.
    pub value: String,
}

/// One axis of a resolved grid: per member tuple, its CANONICAL coordinate
/// (fixed-width ordinals, kept so a rendered cell can be traced back to the
/// aggregate coordinate it came from) and its display labels (resolved at
/// this boundary, never earlier).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GridAxis {
    /// Canonical coordinate of each member tuple, in presentation order.
    pub keys: Vec<Vec<u32>>,
    /// Display labels of each member tuple (one label per axis dimension).
    pub labels: Vec<Vec<String>>,
}

/// A two-axis projection of a resolved object — the renderer-neutral table.
///
/// This is the one shape the flat `(label, value)` rows cannot carry: an
/// object whose projection is addressed by TWO coordinates (a pivot, a
/// cross-tab, a matrix-shaped measurement). It is still a projection of ONE
/// addressed object through ONE named view — orientation is the view's
/// choice, so two slots naming the same target through two views present
/// the same cells rotated, and nothing about the object is copied into the
/// composed document.
///
/// `cells` is row-major: `cells[(r * columns + c) * measures + m]`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolvedGrid {
    /// Row axis.
    pub rows: GridAxis,
    /// Column axis.
    pub columns: GridAxis,
    /// Measure (value) labels — one or more values per cell.
    pub measures: Vec<String>,
    /// Formatted cell values, row-major, `measures.len()` per cell. An empty
    /// string is an empty cell (the source's NULL), never a zero.
    pub cells: Vec<String>,
}

impl ResolvedGrid {
    /// Whether the shape is internally consistent (every axis carries one
    /// label tuple per key, and there is exactly one value per
    /// row × column × measure).
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        self.rows.keys.len() == self.rows.labels.len()
            && self.columns.keys.len() == self.columns.labels.len()
            && self.cells.len()
                == self.rows.keys.len() * self.columns.keys.len() * self.measures.len()
    }

    /// The value at row `r`, column `c`, measure `m`.
    #[must_use]
    pub fn cell(&self, r: usize, c: usize, m: usize) -> Option<&str> {
        let (nc, nm) = (self.columns.keys.len(), self.measures.len());
        if r >= self.rows.keys.len() || c >= nc || m >= nm {
            return None;
        }
        self.cells.get((r * nc + c) * nm + m).map(String::as_str)
    }
}

/// How one slot resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotOutcome {
    /// The target resolved; `fields` are the projected rows in walk order.
    Resolved {
        /// The root object's class (its own classid chose the projection).
        class: ClassId,
        /// The projected rows (view mask ∩ presence, nested via rails).
        fields: Vec<ResolvedField>,
    },
    /// The target resolved to a two-axis projection (see [`ResolvedGrid`]):
    /// its source answered [`DocObjectSource::grid_of`] for the slot's view.
    Grid {
        /// The root object's class.
        class: ClassId,
        /// The projected table.
        grid: ResolvedGrid,
    },
    /// The target was unresolvable and the slot carried a snapshot fallback —
    /// the explicit ActionText missing-object path, surfaced as the
    /// content-address (hex sha256) for the renderer to show.
    Fallback {
        /// Lowercase hex of the snapshot's sha256.
        content_sha256_hex: String,
    },
    /// The target was unresolvable and NO fallback exists (or the named view
    /// is unknown). Renderers must show an explicit marker — never silence.
    Unresolvable,
}

/// One resolved slot: the address it came from, the named projection it asked
/// for, and the outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSlot {
    /// The slot's `ogar://` address (with its resolution mode) — provenance.
    pub uri: String,
    /// The named view (`"user.card"`, …) the slot asked for.
    pub class_view: String,
    /// What resolution produced.
    pub outcome: SlotOutcome,
}

/// One block of the resolved document, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedBlock {
    /// A section heading's text.
    Heading(String),
    /// A run of plain text.
    Text(String),
    /// A resolved projection portal.
    Slot(ResolvedSlot),
}

/// The renderer-neutral resolved document: what every renderer consumes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolvedDoc {
    /// Blocks in document order.
    pub blocks: Vec<ResolvedBlock>,
}

/// The consumer-owned resolution surface — dependency inversion: the resolver
/// owns the *procedure*, the consumer owns the graph, the values, and the
/// view bindings.
pub trait DocObjectSource {
    /// The consumer's object graph (the contract's `RailGraph`).
    type Graph: RailGraph;

    /// The graph itself.
    fn graph(&self) -> &Self::Graph;

    /// Resolve a slot's target address (honouring its [`ResolutionMode`] —
    /// `Live` reads head; `Revision`/`Snapshot` read pinned state) to a graph
    /// key, or `None` when the object is gone/unknown (→ fallback path).
    fn lookup(
        &self,
        target: &ObjectRef,
        mode: &ResolutionMode,
    ) -> Option<<Self::Graph as RailGraph>::Key>;

    /// The formatted value of `key`'s field at `position`, or `None` for a
    /// position with no scalar display value (e.g. a rail position — its
    /// content is the nested rows, not a cell of its own).
    fn value_of(&self, key: <Self::Graph as RailGraph>::Key, position: u8) -> Option<String>;

    /// Resolve a slot's named view (`"user.card"`) to a registry id.
    fn view_by_name(&self, name: &str) -> Option<ViewId>;

    /// The view a NESTED hop's class projects through (the per-class binding
    /// the walk uses below the root). `None` prunes that subtree.
    fn view_for_class(&self, class: ClassId) -> Option<ViewId>;

    /// The two-axis projection of `key` through `view`, when that view is
    /// grid-shaped for this source — `None` (the default) means "project
    /// through the rail walk as usual", so every existing source is
    /// unaffected. A source whose objects are addressed by two coordinates
    /// (an aggregate result, a matrix-shaped measurement) answers here; the
    /// orientation is the VIEW's, so rotating a table is naming another
    /// view, never re-deriving the object.
    fn grid_of(&self, key: <Self::Graph as RailGraph>::Key, view: ViewId) -> Option<ResolvedGrid> {
        let _ = (key, view);
        None
    }
}

/// Resolve a composed document into renderer-neutral blocks.
///
/// Validates the composition first (same gate as [`DocCompose::validate`]);
/// then walks the node tree in document order, resolving every
/// [`DocNode::ObjectSlot`] via [`resolve_slot`].
///
/// # Errors
///
/// Propagates [`ComposeError`] from validation — a malformed composition is
/// refused loudly, never partially rendered.
pub fn resolve_doc<S, V>(
    doc: &DocCompose,
    source: &S,
    class_view: &V,
    registry: &ViewRegistry,
    max_depth: usize,
) -> Result<ResolvedDoc, ComposeError>
where
    S: DocObjectSource,
    V: ClassView,
{
    doc.validate()?;
    let mut out = ResolvedDoc::default();
    walk_doc_node(
        doc, doc.root, source, class_view, registry, max_depth, &mut out,
    );
    Ok(out)
}

/// Resolve one slot: target → (walk → rows) | fallback | unresolvable.
pub fn resolve_slot<S, V>(
    slot: &ObjectSlot,
    source: &S,
    class_view: &V,
    registry: &ViewRegistry,
    max_depth: usize,
) -> ResolvedSlot
where
    S: DocObjectSource,
    V: ClassView,
{
    let uri = format_ogar_uri(&slot.target, &slot.resolution);

    // Root-view/class agreement (codex P2 on #222): the slot's named view must
    // project the ROOT's own class. A stale or cross-class view name would
    // otherwise yield a silently-EMPTY `Resolved` (the walker prunes the
    // foreign mask fail-closed), which reads as "object rendered with no
    // fields" instead of "slot did not resolve". Fail closed into the
    // fallback/unresolvable arm so the outcome says what actually happened.
    let resolved = match (
        source.lookup(&slot.target, &slot.resolution),
        source.view_by_name(&slot.class_view),
    ) {
        (Some(root), Some(root_view)) => {
            let root_class = source.graph().class_of(root);
            registry
                .get(root_view)
                .is_some_and(|v| v.class == root_class)
                .then_some((root, root_view, root_class))
        }
        _ => None,
    };

    let outcome = match resolved {
        // A grid-shaped view: the source projects the table itself. A
        // malformed grid fails closed (never a half-rendered table).
        Some((root, root_view, root_class)) if let Some(grid) = source.grid_of(root, root_view) => {
            if grid.is_well_formed() {
                SlotOutcome::Grid {
                    class: root_class,
                    grid,
                }
            } else {
                match &slot.fallback {
                    Some(snap) => SlotOutcome::Fallback {
                        content_sha256_hex: hex32(&snap.content_sha256),
                    },
                    None => SlotOutcome::Unresolvable,
                }
            }
        }
        Some((root, root_view, root_class)) => {
            let graph = source.graph();
            let mut fields = Vec::new();
            walk_rails(
                graph,
                class_view,
                registry,
                // The slot's named view governs the ROOT; nested hops resolve
                // per-class via the consumer's binding.
                |class| {
                    if class == root_class {
                        Some(root_view)
                    } else {
                        source.view_for_class(class)
                    }
                },
                root,
                max_depth,
                &mut |visit| {
                    // A position with no scalar value (a rail) contributes its
                    // nested rows instead of a cell of its own.
                    if let Some(value) = source.value_of(visit.key, visit.position) {
                        let label = class_view
                            .field_label(visit.class, visit.position)
                            .unwrap_or("")
                            .to_string();
                        fields.push(ResolvedField {
                            position: visit.position,
                            depth: visit.depth,
                            label,
                            value,
                        });
                    }
                },
            );
            SlotOutcome::Resolved {
                class: root_class,
                fields,
            }
        }
        _ => match &slot.fallback {
            Some(snap) => SlotOutcome::Fallback {
                content_sha256_hex: hex32(&snap.content_sha256),
            },
            None => SlotOutcome::Unresolvable,
        },
    };

    ResolvedSlot {
        uri,
        class_view: slot.class_view.clone(),
        outcome,
    }
}

fn walk_doc_node<S, V>(
    doc: &DocCompose,
    id: NodeId,
    source: &S,
    class_view: &V,
    registry: &ViewRegistry,
    max_depth: usize,
    out: &mut ResolvedDoc,
) where
    S: DocObjectSource,
    V: ClassView,
{
    let Some(node) = doc.nodes.get(id.0 as usize) else {
        return; // validate() already refused out-of-range ids
    };
    match node {
        DocNode::Document { children } => {
            for &c in children {
                walk_doc_node(doc, c, source, class_view, registry, max_depth, out);
            }
        }
        DocNode::Section { heading, children } => {
            if let Some(h) = heading
                && let Some(DocNode::Text { text }) = doc.nodes.get(h.0 as usize)
            {
                out.blocks.push(ResolvedBlock::Heading(text.clone()));
            }
            for &c in children {
                walk_doc_node(doc, c, source, class_view, registry, max_depth, out);
            }
        }
        DocNode::Paragraph { children } => {
            for &c in children {
                walk_doc_node(doc, c, source, class_view, registry, max_depth, out);
            }
        }
        DocNode::Text { text } => out.blocks.push(ResolvedBlock::Text(text.clone())),
        DocNode::ObjectSlot { slot } => out.blocks.push(ResolvedBlock::Slot(resolve_slot(
            slot, source, class_view, registry, max_depth,
        ))),
    }
}

/// Lowercase hex of a 32-byte digest (zero-dep; the fallback surface).
fn hex32(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes {
        use core::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::SnapshotRef;
    use lance_graph_contract::class_view::WideFieldMask;
    use lance_graph_contract::ontology::{DisplayTemplate, FieldRef};
    use lance_graph_contract::selection::NamedView;

    const USER: ClassId = 1;
    const WP: ClassId = 2;

    struct TestView;
    impl ClassView for TestView {
        fn fields(&self, class: ClassId) -> &[FieldRef] {
            use std::sync::OnceLock;
            static USER_F: OnceLock<Vec<FieldRef>> = OnceLock::new();
            static WP_F: OnceLock<Vec<FieldRef>> = OnceLock::new();
            match class {
                USER => USER_F.get_or_init(|| {
                    vec![
                        FieldRef::new("u:name", "name"),
                        FieldRef::new("u:email", "email"),
                    ]
                }),
                _ => WP_F.get_or_init(|| {
                    vec![
                        FieldRef::new("wp:subject", "subject"),
                        FieldRef::new("wp:assignee", "assignee"),
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

    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    enum K {
        Alice,
        Wp1,
    }

    struct G;
    impl RailGraph for G {
        type Key = K;
        fn class_of(&self, key: K) -> ClassId {
            match key {
                K::Alice => USER,
                K::Wp1 => WP,
            }
        }
        fn present_mask(&self, _key: K) -> WideFieldMask {
            WideFieldMask::from(0b11)
        }
        fn rail_target(&self, key: K, position: u8) -> Option<K> {
            match (key, position) {
                (K::Wp1, 1) => Some(K::Alice),
                _ => None,
            }
        }
    }

    struct Src {
        registry_names: Vec<(&'static str, ViewId)>,
        graph: G,
    }
    impl DocObjectSource for Src {
        type Graph = G;
        fn graph(&self) -> &G {
            &self.graph
        }
        fn lookup(&self, target: &ObjectRef, _mode: &ResolutionMode) -> Option<K> {
            match target.id.as_str() {
                "alice" => Some(K::Alice),
                "wp1" => Some(K::Wp1),
                _ => None,
            }
        }
        fn value_of(&self, key: K, position: u8) -> Option<String> {
            match (key, position) {
                (K::Alice, 0) => Some("Alice".into()),
                (K::Alice, 1) => Some("alice@example.test".into()),
                (K::Wp1, 0) => Some("Install fire door".into()),
                (K::Wp1, 1) => None, // rail position: no scalar cell
                _ => None,
            }
        }
        fn view_by_name(&self, name: &str) -> Option<ViewId> {
            self.registry_names
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, id)| *id)
        }
        fn view_for_class(&self, class: ClassId) -> Option<ViewId> {
            match class {
                USER => self.view_by_name("user.inline"),
                _ => None,
            }
        }
    }

    /// A source that answers `grid_of` for ONE view on ONE object — the
    /// minimal two-axis projection source (the "M" object below).
    struct GridSrc {
        inner: Src,
        grid_view: ViewId,
        malformed: bool,
    }
    impl DocObjectSource for GridSrc {
        type Graph = G;
        fn graph(&self) -> &G {
            &self.inner.graph
        }
        fn lookup(&self, target: &ObjectRef, mode: &ResolutionMode) -> Option<K> {
            self.inner.lookup(target, mode)
        }
        fn value_of(&self, key: K, position: u8) -> Option<String> {
            self.inner.value_of(key, position)
        }
        fn view_by_name(&self, name: &str) -> Option<ViewId> {
            if name == "wp.grid" {
                Some(self.grid_view)
            } else {
                self.inner.view_by_name(name)
            }
        }
        fn view_for_class(&self, class: ClassId) -> Option<ViewId> {
            self.inner.view_for_class(class)
        }
        fn grid_of(&self, key: K, view: ViewId) -> Option<ResolvedGrid> {
            (key == K::Wp1 && view == self.grid_view).then(|| ResolvedGrid {
                rows: GridAxis {
                    keys: vec![vec![0], vec![1]],
                    labels: vec![vec!["r0".into()], vec!["r1".into()]],
                },
                columns: GridAxis {
                    keys: vec![vec![0], vec![1], vec![2]],
                    labels: vec![vec!["c0".into()], vec!["c1".into()], vec!["c2".into()]],
                },
                measures: vec!["m".into()],
                cells: if self.malformed {
                    vec!["1".into()]
                } else {
                    (0..6).map(|i| i.to_string()).collect()
                },
            })
        }
    }

    fn grid_setup(malformed: bool) -> (GridSrc, ViewRegistry) {
        let (inner, mut registry) = setup();
        let grid_view = registry.register(NamedView::new(
            WP,
            WideFieldMask::from(0),
            DisplayTemplate::Detail,
        ));
        (
            GridSrc {
                inner,
                grid_view,
                malformed,
            },
            registry,
        )
    }

    #[test]
    fn a_grid_view_resolves_to_a_grid_and_other_views_still_walk_rails() {
        let (src, registry) = grid_setup(false);
        let r = resolve_slot(&slot("wp1", "wp.grid", None), &src, &TestView, &registry, 4);
        let SlotOutcome::Grid { class, grid } = &r.outcome else {
            panic!("expected Grid, got {:?}", r.outcome);
        };
        assert_eq!(*class, WP);
        assert_eq!(grid.cell(1, 2, 0), Some("5"));
        assert_eq!(grid.cell(2, 0, 0), None);
        // Can-stay-silent twin: the SAME object through a non-grid view is
        // the ordinary rail walk — the default `grid_of` changes nothing.
        let r = resolve_slot(
            &slot("wp1", "work_package.summary", None),
            &src,
            &TestView,
            &registry,
            4,
        );
        assert!(matches!(r.outcome, SlotOutcome::Resolved { .. }));
    }

    #[test]
    fn a_malformed_grid_fails_closed() {
        let (src, registry) = grid_setup(true);
        let r = resolve_slot(&slot("wp1", "wp.grid", None), &src, &TestView, &registry, 4);
        assert_eq!(r.outcome, SlotOutcome::Unresolvable);
    }

    #[test]
    fn a_grid_view_still_obeys_root_class_agreement() {
        // The grid view projects WP; aimed at a USER object it must not
        // resolve (the class gate runs before `grid_of` is consulted).
        let (src, registry) = grid_setup(false);
        let mut s = slot("alice", "wp.grid", None);
        s.target.class = "user".into();
        let r = resolve_slot(&s, &src, &TestView, &registry, 4);
        assert_eq!(r.outcome, SlotOutcome::Unresolvable);
    }

    fn slot(target_id: &str, view: &str, fallback: Option<SnapshotRef>) -> ObjectSlot {
        ObjectSlot {
            target: ObjectRef {
                app: "openproject".into(),
                class: "work-package".into(),
                id: target_id.into(),
            },
            class_view: view.into(),
            field_mask: 0,
            wide_mask_words: vec![],
            resolution: ResolutionMode::Live,
            fallback,
        }
    }

    fn setup() -> (Src, ViewRegistry) {
        let mut registry = ViewRegistry::new();
        let inline = registry.register(NamedView::new(
            USER,
            WideFieldMask::from(0b01), // name only
            DisplayTemplate::Card,
        ));
        let summary = registry.register(NamedView::new(
            WP,
            WideFieldMask::from(0b11), // subject + assignee rail
            DisplayTemplate::Detail,
        ));
        let src = Src {
            registry_names: vec![("user.inline", inline), ("work_package.summary", summary)],
            graph: G,
        };
        (src, registry)
    }

    #[test]
    fn slot_resolves_with_nested_rail_hop() {
        let (src, registry) = setup();
        let s = slot("wp1", "work_package.summary", None);
        let r = resolve_slot(&s, &src, &TestView, &registry, 4);
        let SlotOutcome::Resolved { class, fields } = &r.outcome else {
            panic!("expected Resolved, got {:?}", r.outcome);
        };
        assert_eq!(*class, WP);
        // subject at depth 0, nested alice name at depth 1; the rail position
        // itself contributes no cell; email is masked out by user.inline.
        assert_eq!(fields.len(), 2);
        assert_eq!((fields[0].label.as_str(), fields[0].depth), ("subject", 0));
        assert_eq!(fields[0].value, "Install fire door");
        assert_eq!((fields[1].label.as_str(), fields[1].depth), ("name", 1));
        assert_eq!(fields[1].value, "Alice");
    }

    #[test]
    fn missing_target_uses_snapshot_fallback() {
        let (src, registry) = setup();
        let s = slot(
            "ghost",
            "work_package.summary",
            Some(SnapshotRef {
                content_sha256: [0xAB; 32],
            }),
        );
        let r = resolve_slot(&s, &src, &TestView, &registry, 4);
        assert_eq!(
            r.outcome,
            SlotOutcome::Fallback {
                content_sha256_hex: "ab".repeat(32)
            }
        );
    }

    #[test]
    fn missing_target_without_fallback_is_unresolvable() {
        let (src, registry) = setup();
        let s = slot("ghost", "work_package.summary", None);
        let r = resolve_slot(&s, &src, &TestView, &registry, 4);
        assert_eq!(r.outcome, SlotOutcome::Unresolvable);
    }

    #[test]
    fn unknown_view_is_unresolvable_not_a_panic() {
        let (src, registry) = setup();
        let s = slot("wp1", "no.such.view", None);
        let r = resolve_slot(&s, &src, &TestView, &registry, 4);
        assert_eq!(r.outcome, SlotOutcome::Unresolvable);
    }

    /// Falsifier (codex P2 on #222): a slot whose named view projects a
    /// DIFFERENT class than its target must fail closed — Unresolvable (or the
    /// fallback), never a silently-EMPTY `Resolved`.
    #[test]
    fn cross_class_root_view_is_unresolvable_not_empty_resolved() {
        let (src, registry) = setup();
        // Target wp1 (class WP) with the USER-class "user.inline" view.
        let s = slot("wp1", "user.inline", None);
        let r = resolve_slot(&s, &src, &TestView, &registry, 4);
        assert_eq!(r.outcome, SlotOutcome::Unresolvable);

        // With a fallback attached, the same mismatch takes the fallback arm.
        let s = slot(
            "wp1",
            "user.inline",
            Some(SnapshotRef {
                content_sha256: [0xCD; 32],
            }),
        );
        let r = resolve_slot(&s, &src, &TestView, &registry, 4);
        assert_eq!(
            r.outcome,
            SlotOutcome::Fallback {
                content_sha256_hex: "cd".repeat(32)
            }
        );
    }
}
