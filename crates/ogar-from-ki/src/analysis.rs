//! The second projection: plain analysis rows from lifted items.
//!
//! STATIC AND NAME-LEVEL. A row says that an item, as written, names a class,
//! reads or writes a name, calls a capability or binds a result. Nothing here
//! evaluates a condition, resolves a bare name, or knows which vertex an item
//! meets at run time. In particular, "two items can interfere" means: one
//! writes (or deletes) a name the other reads or writes, and their `on`
//! clauses can name the same vertex. It does not mean they ever do, and it
//! says nothing about order, ticket identity or values.

use crate::lift::{LiftedItem, OnTarget};

/// The `to` of an `AppliesOn` row for an item that applies on any vertex
/// (`on ogit/_id`).
pub const ANY_VERTEX: &str = "ogit/_id";
/// The `to` of an `AppliesOn` row for an `on` form that names no class the
/// docs define.
pub const UNRESOLVED_ON: &str = "?";

/// The kind of a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KiEdgeKind {
    /// item to the class (or [`ANY_VERTEX`], or [`UNRESOLVED_ON`]) it applies on.
    AppliesOn,
    /// item to a name its conditions test.
    Reads,
    /// item to a name its `do` assigns or deletes.
    Writes,
    /// item to a capability it calls.
    Calls,
    /// `<call identity>.<field>` to the `LOCAL::` name the field is bound to.
    BindsResult,
}

/// One row.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KiEdge {
    /// The source of the row (see [`KiEdgeKind`]).
    pub from: String,
    /// The kind.
    pub kind: KiEdgeKind,
    /// The target of the row.
    pub to: String,
}

fn edge(from: &str, kind: KiEdgeKind, to: &str) -> KiEdge {
    KiEdge {
        from: from.to_string(),
        kind,
        to: to.to_string(),
    }
}

/// The rows of one item, in a fixed order: `AppliesOn`, `Reads`, `Writes`,
/// then per call `Calls` and its `BindsResult` rows. Rows are not repeated.
#[must_use]
pub fn edges(item: &LiftedItem) -> Vec<KiEdge> {
    let mut out: Vec<KiEdge> = Vec::new();
    let on = match &item.on {
        OnTarget::AnyVertex => ANY_VERTEX,
        OnTarget::Class(c) => c.as_str(),
        OnTarget::Unresolved { .. } => UNRESOLVED_ON,
    };
    out.push(edge(&item.id, KiEdgeKind::AppliesOn, on));
    for r in &item.reads {
        out.push(edge(&item.id, KiEdgeKind::Reads, r));
    }
    for w in &item.writes {
        out.push(edge(&item.id, KiEdgeKind::Writes, w));
    }
    for c in &item.calls {
        let e = edge(&item.id, KiEdgeKind::Calls, &c.capability);
        if !out.contains(&e) {
            out.push(e);
        }
        for b in &c.results {
            out.push(edge(
                &format!("{}.{}", c.identity, b.field),
                KiEdgeKind::BindsResult,
                &b.local,
            ));
        }
    }
    out
}

/// How two items touch one name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contact {
    /// The first writes it, the second reads it.
    WriteRead,
    /// The first reads it, the second writes it.
    ReadWrite,
    /// Both write it.
    WriteWrite,
}

/// One shared name of an interfering pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedName {
    /// The name.
    pub name: String,
    /// How the two items touch it.
    pub contact: Contact,
}

/// Two items that can interfere, statically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interference {
    /// Id of the earlier item in the input.
    pub first: String,
    /// Id of the later item in the input.
    pub second: String,
    /// The names that make it so.
    pub names: Vec<SharedName>,
}

/// Whether two `on` clauses can name the same vertex: the same class, or
/// either one is `ogit/_id` (any vertex). An `on` form the docs do not define
/// ([`OnTarget::Unresolved`]) is treated like any vertex: the analysis does
/// not claim two items are apart when it cannot tell.
#[must_use]
pub fn can_share_vertex(a: &OnTarget, b: &OnTarget) -> bool {
    match (a, b) {
        (OnTarget::Class(x), OnTarget::Class(y)) => x == y,
        _ => true,
    }
}

/// The pairs of `items` that can interfere, in input order (`i < j`).
#[must_use]
pub fn interfering_pairs(items: &[LiftedItem]) -> Vec<Interference> {
    let mut out = Vec::new();
    for (i, a) in items.iter().enumerate() {
        for b in &items[i + 1..] {
            if !can_share_vertex(&a.on, &b.on) {
                continue;
            }
            let mut names: Vec<SharedName> = Vec::new();
            for w in &a.writes {
                let contact = if b.writes.contains(w) {
                    Contact::WriteWrite
                } else if b.reads.contains(w) {
                    Contact::WriteRead
                } else {
                    continue;
                };
                names.push(SharedName {
                    name: w.clone(),
                    contact,
                });
            }
            for w in &b.writes {
                if a.reads.contains(w) && !a.writes.contains(w) {
                    names.push(SharedName {
                        name: w.clone(),
                        contact: Contact::ReadWrite,
                    });
                }
            }
            if !names.is_empty() {
                out.push(Interference {
                    first: a.id.clone(),
                    second: b.id.clone(),
                    names,
                });
            }
        }
    }
    out
}
