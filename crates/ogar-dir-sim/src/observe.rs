//! Observed state from `ogar-ad` records (PR #313) into a [`GraphState`].
//!
//! Read-only and derived: "active" comes from `userAccountControl` bit
//! `0x2` (ACCOUNTDISABLE), the primary SMTP from the `SMTP:` proxy. Group
//! membership is not carried by `ogar-ad` records (it is a relation); the
//! caller adds observed memberships with [`GraphState::put_membership`].

use crate::graph::{GraphState, Node, NodeKind};
use ogar_ad::{AdKind, SCHEMA_V1};
use ogar_dir_core::{DirRecord, ValuePool};

const UAC_ACCOUNTDISABLE: u32 = 0x2;

fn slot(name: &str) -> usize {
    SCHEMA_V1
        .iter()
        .find(|d| d.name == name)
        .map(|d| d.slot as usize)
        .expect("ogar-ad schema v1")
}

fn text<'p>(r: &DirRecord, p: &'p ValuePool, name: &str) -> Option<&'p str> {
    std::str::from_utf8(p.get(r.str_ref(slot(name))?)?).ok()
}

/// Users and groups of `records` (other kinds are skipped).
pub fn from_ad(records: &[DirRecord], pool: &ValuePool) -> GraphState {
    let mut g = GraphState::new();
    for r in records {
        let kind = match r.object_kind() {
            k if k == AdKind::User as u16 => NodeKind::User,
            k if k == AdKind::Group as u16 => NodeKind::Group,
            _ => continue,
        };
        let primary_smtp = r
            .str_ref(slot("proxyAddresses"))
            .and_then(|s| pool.get_multi(s))
            .and_then(|vs| {
                vs.into_iter()
                    .filter_map(|v| std::str::from_utf8(v).ok())
                    .find_map(|v| v.strip_prefix("SMTP:").map(str::to_string))
            });
        g.put_node(
            r.node_guid(),
            Node {
                kind,
                name: text(r, pool, "displayName")
                    .or_else(|| text(r, pool, "sAMAccountName"))
                    .unwrap_or("")
                    .into(),
                active: r.num(0).is_none_or(|uac| uac & UAC_ACCOUNTDISABLE == 0),
                upn: text(r, pool, "userPrincipalName").map(str::to_string),
                primary_smtp,
            },
        );
    }
    g
}
