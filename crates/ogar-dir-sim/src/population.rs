//! A set of nodes of one version, as a dense bitset over that version's
//! node order — the shape rules select with, so they never have to be
//! written as `for user in users { run_workflow(user) }`.
//!
//! Identity stays [`Guid128`]; the bit index is an execution ordinal valid
//! only for the [`GraphState`] it was taken from (the version's sorted node
//! order). [`Population::guids`] resolves it back.
//!
//! The contract crate has no row-population mask today (`FieldMask` /
//! `WideFieldMask` are column masks, `RowFocusMask` holds address subtrees).
//! This is deliberately the same shape as lance-graph-java's `Mask`
//! (`Box<[u64]>` over rows, `and` / `minus`), so it can be replaced by a
//! shared row mask without changing any rule.

use crate::graph::{GraphState, Node, NodeKind};
use ogar_dir_core::Guid128;

/// Node set over one version's ordinals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Population {
    words: Vec<u64>,
    order: Vec<Guid128>,
}

impl Population {
    fn empty_for(g: &GraphState) -> Self {
        let order: Vec<Guid128> = g.nodes().map(|(id, _)| *id).collect();
        Self {
            words: vec![0; order.len().div_ceil(64)],
            order,
        }
    }
    fn set(&mut self, i: usize) {
        self.words[i / 64] |= 1 << (i % 64);
    }

    /// All nodes matching `pred`.
    pub fn select(g: &GraphState, pred: impl Fn(&Guid128, &Node) -> bool) -> Self {
        let mut p = Self::empty_for(g);
        for (i, (id, n)) in g.nodes().enumerate() {
            if pred(id, n) {
                p.set(i);
            }
        }
        p
    }

    /// Active users.
    pub fn active_users(g: &GraphState) -> Self {
        Self::select(g, |_, n| n.kind == NodeKind::User && n.active)
    }

    /// Members of `group`.
    pub fn members_of(g: &GraphState, group: Guid128) -> Self {
        Self::select(g, |id, _| g.is_member(*id, group))
    }

    /// Exactly the given ids (unknown ids are ignored).
    pub fn of(g: &GraphState, ids: &[Guid128]) -> Self {
        Self::select(g, |id, _| ids.contains(id))
    }

    fn zip(&self, o: &Self, f: impl Fn(u64, u64) -> u64) -> Self {
        assert_eq!(self.order, o.order, "populations from different versions");
        Self {
            words: self
                .words
                .iter()
                .zip(&o.words)
                .map(|(a, b)| f(*a, *b))
                .collect(),
            order: self.order.clone(),
        }
    }
    /// Intersection.
    pub fn and(&self, o: &Self) -> Self {
        self.zip(o, |a, b| a & b)
    }
    /// Set difference.
    pub fn minus(&self, o: &Self) -> Self {
        self.zip(o, |a, b| a & !b)
    }
    /// Cardinality.
    pub fn count(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }
    /// Resolve to identities, in version order.
    pub fn guids(&self) -> Vec<Guid128> {
        self.order
            .iter()
            .enumerate()
            .filter(|(i, _)| self.words[i / 64] >> (i % 64) & 1 == 1)
            .map(|(_, g)| *g)
            .collect()
    }
}
