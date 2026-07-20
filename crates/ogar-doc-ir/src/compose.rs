//! `compose` — the **composition layer** over the OGAR object graph
//! (W1 brick of `docs/DOCIR-COMPOSITION-LAYER.md`, grounded in
//! `docs/DOCIR-COMPOSITION-GROUNDING.md`; ledger `D-DOCIR-COMPOSITION`).
//!
//! # The one idea
//!
//! A composed document is a **lens through the object world, not a box where
//! copies of that world go to slowly decay**. The composition graph carries
//! ONLY document structure; every domain object enters through an
//! [`ObjectSlot`] — a typed projection portal (the `D-DOC-IR-SECOND-RETINA`
//! **A3 brick**, `ClassView × WideFieldMask`, plus an [`ObjectRef`] and a
//! [`ResolutionMode`]). The slot stores an address and a projection choice,
//! never the object's representation — the ActionText attachment doctrine,
//! transplanted (doctrine §1).
//!
//! # Placement (operator ruling A1)
//!
//! *"One `ogar-doc` crate; split axis is STATE not direction"* — so the
//! composition graph is a MODULE in this crate family, beside the observation
//! IR ([`crate::DocIr`]), never a sibling `ogar-doc-compose` crate. The
//! observation IR is untouched; a composed document references an observation
//! subtree through a slot (an imported scan appears as a portal, not as
//! pasted regions).
//!
//! # The load-bearing invariants (the observation IR's discipline, replayed)
//!
//! 1. **Closed node vocabulary + version marker.** [`from_json`] hard-fails on
//!    any kind outside [`DocNode`]'s five variants or any version other than
//!    [`DOC_COMPOSE_VERSION`]. The five kinds are the operator-specified slice
//!    scope (doctrine §7: `Document / Section / Paragraph / Text /
//!    ObjectSlot`); `Figure` / `Table` / `PageBreak` are a
//!    `doc-compose.v2` bump, never a silent widening.
//! 2. **Canon-free.** serde-only, zero canon dependency — like the observation
//!    IR, this lands ahead of mints. Masks travel in their WIRE form
//!    (`u64` words, word `w` bit `b` ⇒ field position `w·64 + b` — the
//!    `ogar-a2ui-frame` precedent), converting to typed
//!    `FieldMask`/`WideFieldMask` at the contract membrane.
//! 3. **Explicit resolution.** Every slot names how it resolves —
//!    [`ResolutionMode::Live`] (dashboards), [`ResolutionMode::Revision`]
//!    (signed/GoBD artifacts), [`ResolutionMode::Snapshot`] (deleted objects
//!    fall back). The printable form is the `ogar://` address; a URI without a
//!    resolution suffix is a **refusal**, not a default.
//! 4. **RBAC is NOT this module's claim.** A slot's masks are a *request*;
//!    the fail-closed `requested ∧ role` intersection happens server-side at
//!    resolution time (grounding §A4 — the `ClassRbac` enforcement is
//!    probe-gated, so the resolver carries the intersection itself).
//!
//! # Deferred (named, not hidden)
//!
//! The editor-authority operation log (`DocOp`, doctrine §4) is the NEXT
//! brick — this one is render-side only (build → validate → resolve →
//! render). CRDT backing is `DocOp`'s upgrade seam, not this module's.

use serde::{Deserialize, Serialize};

/// Composition-IR version marker. [`from_json`] refuses any other value.
/// A `doc-compose.v2` is a NEW marker + (possibly) new [`DocNode`] kinds,
/// never a silent reshape.
pub const DOC_COMPOSE_VERSION: &str = "doc-compose.v1";

/// Index of a node in [`DocCompose::nodes`] (arena-style; the doctrine's
/// `NodeId`). A newtype so an id never silently mixes with a mask position
/// or an ordinal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub u32);

/// A stable typed reference to an OGAR object — the address half of the
/// `ogar://{app}/{class}/{id}@{resolution}` printable form. Segments are the
/// codebook names (canon-free here); the classid resolves at the membrane
/// (`canonical_concept_id`), never in this module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectRef {
    /// App/prefix segment (e.g. `"openproject"`, `"bim"`).
    pub app: String,
    /// Class segment (e.g. `"work-package"`, `"wall"`).
    pub class: String,
    /// Instance segment (the id as the app prints it).
    pub id: String,
}

/// How a slot resolves its target — the beyond-Rails move (doctrine §1):
/// a dashboard wants live content; a signed report wants a pinned revision;
/// a deleted object needs a snapshot fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "mode", content = "value")]
pub enum ResolutionMode {
    /// Resolve the object as it is now (`@live`).
    Live,
    /// Resolve a pinned revision (`@revision:N` — the Lance-versioning
    /// shape, ADR-008/013).
    Revision(u64),
    /// Resolve a content-addressed snapshot (`@sha256:…` — the doc-KV
    /// `DocumentId = sha256(raw)` discipline).
    Snapshot([u8; 32]),
}

/// Content-addressed fallback for a slot whose live target is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRef {
    /// sha256 of the snapshotted projection.
    pub content_sha256: [u8; 32],
}

/// The typed projection portal — the A3 brick (`ClassView × WideFieldMask`)
/// made addressable. Stores the address and the projection CHOICE; never the
/// object's representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectSlot {
    /// Which object.
    pub target: ObjectRef,
    /// Which NAMED semantic projection (e.g. `"work_package.inline"`,
    /// `"user.card"`) — the `(class, mode)` registry key; renderer-independent
    /// by definition (doctrine §2).
    pub class_view: String,
    /// Immediate-field selection, wire form (bit `i` ⇒ field position `i`).
    pub field_mask: u64,
    /// Traversal/nested selection, wire form (`u64` words, word `w` bit `b`
    /// ⇒ position `w·64 + b` — positions ≥ 64 native, the #205 correction).
    #[serde(default)]
    pub wide_mask_words: Vec<u64>,
    /// How the target resolves.
    pub resolution: ResolutionMode,
    /// Content-addressed fallback when the target is unresolvable.
    #[serde(default)]
    pub fallback: Option<SnapshotRef>,
}

/// The **closed** composition vocabulary — exactly the operator-specified
/// slice scope (doctrine §7). Exhaustive by design: a renderer gets a compile
/// error until it handles every kind. NO domain variants — a `Wall` or
/// `WorkPackage` enters only through [`DocNode::ObjectSlot`] (doctrine §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum DocNode {
    /// The document root — ordered children.
    Document {
        /// Child nodes in document order.
        children: Vec<NodeId>,
    },
    /// A section with an optional heading node.
    Section {
        /// Optional heading (a `Text` node, typically).
        #[serde(default)]
        heading: Option<NodeId>,
        /// Child nodes in document order.
        children: Vec<NodeId>,
    },
    /// A paragraph of inline nodes.
    Paragraph {
        /// Inline children (`Text` / `ObjectSlot`) in order.
        children: Vec<NodeId>,
    },
    /// A run of plain text.
    Text {
        /// The text content.
        text: String,
    },
    /// A typed projection portal into the OGAR object graph.
    ObjectSlot {
        /// The slot.
        slot: ObjectSlot,
    },
}

/// A composed document: an arena of nodes + the root. The arena keeps ids
/// stable under edit (the future `DocOp` brick appends; it never re-indexes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocCompose {
    /// Must equal [`DOC_COMPOSE_VERSION`]; [`from_json`] enforces it.
    pub version: String,
    /// The node arena.
    pub nodes: Vec<DocNode>,
    /// The root node (must be in range; see [`DocCompose::validate`]).
    pub root: NodeId,
}

/// Why the composition layer refused an input.
#[derive(Debug)]
pub enum ComposeError {
    /// Malformed JSON or an off-vocabulary [`DocNode`] / [`ResolutionMode`]
    /// (serde rejects unknown enum values — the closed-vocab gate).
    Json(serde_json::Error),
    /// The `version` field was not [`DOC_COMPOSE_VERSION`].
    WrongVersion {
        /// The version the document carried.
        found: String,
        /// The version this module accepts.
        expected: &'static str,
    },
    /// An `ogar://` address that does not parse; the reason is spelled out.
    BadUri(String),
    /// A structurally invalid graph (out-of-range id, or a cycle reachable
    /// from the root).
    Invalid(String),
}

impl core::fmt::Display for ComposeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Json(e) => write!(f, "doc-compose JSON rejected (closed vocabulary): {e}"),
            Self::WrongVersion { found, expected } => write!(
                f,
                "doc-compose version `{found}` unsupported (this module reads `{expected}`; \
                 a new version is a deliberate migration, not a silent reshape)"
            ),
            Self::BadUri(reason) => write!(f, "ogar:// address rejected: {reason}"),
            Self::Invalid(reason) => write!(f, "doc-compose graph rejected: {reason}"),
        }
    }
}

impl std::error::Error for ComposeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}

/// The **load gate** — closed-vocabulary + version enforcement + structural
/// validation at the boundary (the observation IR's `from_json` discipline).
///
/// # Errors
///
/// [`ComposeError::Json`] on malformed JSON or an off-vocab enum value;
/// [`ComposeError::WrongVersion`] when `version != `[`DOC_COMPOSE_VERSION`];
/// [`ComposeError::Invalid`] when [`DocCompose::validate`] refuses the graph.
pub fn from_json(s: &str) -> Result<DocCompose, ComposeError> {
    let doc: DocCompose = serde_json::from_str(s).map_err(ComposeError::Json)?;
    if doc.version != DOC_COMPOSE_VERSION {
        return Err(ComposeError::WrongVersion {
            found: doc.version,
            expected: DOC_COMPOSE_VERSION,
        });
    }
    doc.validate()?;
    Ok(doc)
}

/// Serialize a [`DocCompose`] to JSON (the symmetric egress of [`from_json`]).
///
/// # Errors
///
/// Propagates any `serde_json` serialization error.
pub fn to_json(doc: &DocCompose) -> Result<String, serde_json::Error> {
    serde_json::to_string(doc)
}

impl DocCompose {
    /// Structural validation: every referenced id is in range, and the graph
    /// reachable from [`Self::root`] is acyclic. Orphan nodes (arena slack
    /// left by future edits) are legal; dangling REFERENCES are not.
    ///
    /// # Errors
    ///
    /// [`ComposeError::Invalid`] naming the offending id.
    pub fn validate(&self) -> Result<(), ComposeError> {
        let len = self.nodes.len() as u32;
        let check = |id: NodeId| -> Result<(), ComposeError> {
            if id.0 < len {
                Ok(())
            } else {
                Err(ComposeError::Invalid(format!(
                    "node id {} out of range (arena has {len} nodes)",
                    id.0
                )))
            }
        };
        check(self.root)?;
        for node in &self.nodes {
            match node {
                DocNode::Document { children } | DocNode::Paragraph { children } => {
                    for &c in children {
                        check(c)?;
                    }
                }
                DocNode::Section { heading, children } => {
                    if let Some(h) = heading {
                        check(*h)?;
                    }
                    for &c in children {
                        check(c)?;
                    }
                }
                DocNode::Text { .. } | DocNode::ObjectSlot { .. } => {}
            }
        }
        // Cycle check on the subgraph reachable from root (iterative DFS with
        // an explicit in-stack set — no recursion, no depth limit surprises).
        let mut state = vec![0_u8; self.nodes.len()]; // 0 unvisited, 1 in-stack, 2 done
        let mut stack: Vec<(u32, usize)> = vec![(self.root.0, 0)];
        state[self.root.0 as usize] = 1;
        while let Some(&mut (id, ref mut next)) = stack.last_mut() {
            let kids: Vec<u32> = match &self.nodes[id as usize] {
                DocNode::Document { children } | DocNode::Paragraph { children } => {
                    children.iter().map(|c| c.0).collect()
                }
                DocNode::Section { heading, children } => heading
                    .iter()
                    .map(|h| h.0)
                    .chain(children.iter().map(|c| c.0))
                    .collect(),
                DocNode::Text { .. } | DocNode::ObjectSlot { .. } => Vec::new(),
            };
            if *next < kids.len() {
                let k = kids[*next];
                *next += 1;
                match state[k as usize] {
                    0 => {
                        state[k as usize] = 1;
                        stack.push((k, 0));
                    }
                    1 => {
                        return Err(ComposeError::Invalid(format!(
                            "cycle through node id {k} (a document is a tree, not a loop)"
                        )));
                    }
                    _ => {}
                }
            } else {
                state[id as usize] = 2;
                stack.pop();
            }
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────
// The `ogar://` printable address (doctrine §1 — the SGID equivalent)
// ─────────────────────────────────────────────────────────────────────────

/// Parse an `ogar://{app}/{class}/{id}@{resolution}` address. Strict: every
/// malformed input is a refusal — there is no default resolution, no lenient
/// segment count, no silent hex truncation.
///
/// # Errors
///
/// [`ComposeError::BadUri`] naming the exact defect.
pub fn parse_ogar_uri(uri: &str) -> Result<(ObjectRef, ResolutionMode), ComposeError> {
    let rest = uri
        .strip_prefix("ogar://")
        .ok_or_else(|| ComposeError::BadUri(format!("missing `ogar://` scheme in `{uri}`")))?;
    let (path, res) = rest
        .rsplit_once('@')
        .ok_or_else(|| ComposeError::BadUri(format!("missing `@resolution` suffix in `{uri}`")))?;
    let segs: Vec<&str> = path.split('/').collect();
    let [app, class, id] = segs.as_slice() else {
        return Err(ComposeError::BadUri(format!(
            "path must be app/class/id (got {} segment(s)) in `{uri}`",
            segs.len()
        )));
    };
    if app.is_empty() || class.is_empty() || id.is_empty() {
        return Err(ComposeError::BadUri(format!(
            "empty path segment in `{uri}`"
        )));
    }
    let resolution = if res == "live" {
        ResolutionMode::Live
    } else if let Some(n) = res.strip_prefix("revision:") {
        ResolutionMode::Revision(
            n.parse::<u64>().map_err(|_| {
                ComposeError::BadUri(format!("revision `{n}` is not a u64 in `{uri}`"))
            })?,
        )
    } else if let Some(hex) = res.strip_prefix("sha256:") {
        ResolutionMode::Snapshot(
            parse_sha256_hex(hex)
                .map_err(|reason| ComposeError::BadUri(format!("{reason} in `{uri}`")))?,
        )
    } else {
        return Err(ComposeError::BadUri(format!(
            "unknown resolution `{res}` (expected `live` | `revision:N` | `sha256:<64 hex>`) in `{uri}`"
        )));
    };
    Ok((
        ObjectRef {
            app: (*app).to_string(),
            class: (*class).to_string(),
            id: (*id).to_string(),
        },
        resolution,
    ))
}

/// Print the `ogar://` address (the symmetric egress of [`parse_ogar_uri`]).
#[must_use]
pub fn format_ogar_uri(target: &ObjectRef, resolution: &ResolutionMode) -> String {
    let res = match resolution {
        ResolutionMode::Live => "live".to_string(),
        ResolutionMode::Revision(n) => format!("revision:{n}"),
        ResolutionMode::Snapshot(h) => format!("sha256:{}", sha256_hex(h)),
    };
    format!(
        "ogar://{}/{}/{}@{}",
        target.app, target.class, target.id, res
    )
}

fn parse_sha256_hex(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 {
        return Err(format!("sha256 must be 64 hex chars (got {})", hex.len()));
    }
    let mut out = [0_u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        let pair = &hex[i * 2..i * 2 + 2];
        *byte = u8::from_str_radix(pair, 16)
            .map_err(|_| format!("non-hex byte `{pair}` at position {}", i * 2))?;
    }
    Ok(out)
}

fn sha256_hex(h: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in h {
        use core::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(app: &str, class: &str, id: &str, view: &str) -> ObjectSlot {
        ObjectSlot {
            target: ObjectRef {
                app: app.to_string(),
                class: class.to_string(),
                id: id.to_string(),
            },
            class_view: view.to_string(),
            field_mask: 0b1011,
            wide_mask_words: vec![0, 1 << 5], // position 69 — wide is native
            resolution: ResolutionMode::Live,
            fallback: None,
        }
    }

    /// The §7 proof shape: a WorkPackage.description composing text + three
    /// attachable classes, each through a slot — never as copied objects.
    fn slice_doc() -> DocCompose {
        DocCompose {
            version: DOC_COMPOSE_VERSION.to_string(),
            nodes: vec![
                // 0: root
                DocNode::Document {
                    children: vec![NodeId(1)],
                },
                // 1: one paragraph
                DocNode::Paragraph {
                    children: vec![NodeId(2), NodeId(3), NodeId(4), NodeId(5)],
                },
                // 2: text run
                DocNode::Text {
                    text: "Install fire door — assigned to ".to_string(),
                },
                // 3..5: the three attachables (doctrine §7)
                DocNode::ObjectSlot {
                    slot: slot("openproject", "user", "42", "user.inline"),
                },
                DocNode::ObjectSlot {
                    slot: slot("openproject", "work-package", "8f31", "work_package.inline"),
                },
                DocNode::ObjectSlot {
                    slot: slot("openproject", "attachment", "7", "attachment.inline"),
                },
            ],
            root: NodeId(0),
        }
    }

    #[test]
    fn slice_doc_round_trips_through_the_load_gate() {
        let doc = slice_doc();
        let json = to_json(&doc).expect("serialize");
        let back = from_json(&json).expect("load gate accepts the slice shape");
        assert_eq!(doc, back);
    }

    #[test]
    fn wrong_version_is_a_refusal() {
        let mut doc = slice_doc();
        doc.version = "doc-compose.v2".to_string();
        let json = to_json(&doc).expect("serialize");
        assert!(matches!(
            from_json(&json),
            Err(ComposeError::WrongVersion { .. })
        ));
    }

    #[test]
    fn unknown_node_kind_is_a_refusal() {
        // A domain variant smuggled into the composition vocabulary — the §3
        // rule: `Wall` belongs to OGAR, never to DocNode.
        let json = format!(
            r#"{{"version":"{DOC_COMPOSE_VERSION}","nodes":[{{"kind":"wall","name":"W1"}}],"root":0}}"#
        );
        assert!(matches!(from_json(&json), Err(ComposeError::Json(_))));
    }

    #[test]
    fn out_of_range_reference_is_a_refusal() {
        let doc = DocCompose {
            version: DOC_COMPOSE_VERSION.to_string(),
            nodes: vec![DocNode::Document {
                children: vec![NodeId(7)],
            }],
            root: NodeId(0),
        };
        assert!(matches!(doc.validate(), Err(ComposeError::Invalid(_))));
    }

    #[test]
    fn cycle_is_a_refusal() {
        let doc = DocCompose {
            version: DOC_COMPOSE_VERSION.to_string(),
            nodes: vec![
                DocNode::Document {
                    children: vec![NodeId(1)],
                },
                DocNode::Paragraph {
                    children: vec![NodeId(0)],
                },
            ],
            root: NodeId(0),
        };
        assert!(matches!(doc.validate(), Err(ComposeError::Invalid(_))));
    }

    #[test]
    fn ogar_uri_round_trips_all_three_resolution_arms() {
        let cases = [
            "ogar://openproject/work-package/8f31@live",
            "ogar://bim/wall/a127@revision:42",
            &format!("ogar://document/drawing/c920@sha256:{}", "ab".repeat(32)),
        ];
        for uri in cases {
            let (target, resolution) = parse_ogar_uri(uri).expect("parse");
            assert_eq!(format_ogar_uri(&target, &resolution), *uri);
        }
    }

    #[test]
    fn ogar_uri_refusals_are_loud() {
        for bad in [
            "http://openproject/work-package/8f31@live", // wrong scheme
            "ogar://openproject/work-package/8f31",      // no resolution — no default
            "ogar://openproject/8f31@live",              // two segments
            "ogar://openproject//8f31@live",             // empty segment
            "ogar://a/b/c@revision:many",                // non-numeric revision
            "ogar://a/b/c@sha256:abcd",                  // short hex
            "ogar://a/b/c@pinned",                       // unknown mode
        ] {
            assert!(
                matches!(parse_ogar_uri(bad), Err(ComposeError::BadUri(_))),
                "should refuse `{bad}`"
            );
        }
    }

    #[test]
    fn snapshot_fallback_survives_serde() {
        let mut doc = slice_doc();
        if let DocNode::ObjectSlot { slot } = &mut doc.nodes[3] {
            slot.resolution = ResolutionMode::Revision(42);
            slot.fallback = Some(SnapshotRef {
                content_sha256: [0xAB; 32],
            });
        }
        let json = to_json(&doc).expect("serialize");
        let back = from_json(&json).expect("load");
        assert_eq!(doc, back);
    }
}
