//! The second projection: rows and static interference pairs.
//! The items below are SYNTHETIC, hand-written inside the documented subset.

mod common;

use common::HTTP;
use ogar_from_ki::{
    Contact, KiEdge, KiEdgeKind, LiftOptions, LiftedItem, edges, interfering_pairs, lift_source,
};

fn lifted(name: &str, on: &str, when: &str, do_: &str) -> LiftedItem {
    let src = format!("ki\n  name: \"{name}\"\non\n  {on}\nwhen\n  {when}\ndo\n  {do_}\n");
    lift_source(&src, &LiftOptions::default())
        .expect("lifts")
        .item
}

const CAL: &str = "ogit/_type == \"ogit/Schedule/Calendar\"";
const TASK: &str = "ogit/_type == \"ogit/Task\"";

#[test]
fn rows_of_the_http_item() {
    let l = lift_source(
        HTTP,
        &LiftOptions {
            item_id: Some("h".into()),
        },
    )
    .unwrap();
    let e = |from: &str, kind, to: &str| KiEdge {
        from: from.into(),
        kind,
        to: to.into(),
    };
    assert_eq!(
        edges(&l.item),
        vec![
            e("h", KiEdgeKind::AppliesOn, "ogit/_id"),
            e("h", KiEdgeKind::Reads, "ProcessIssue"),
            e("h", KiEdgeKind::Writes, "ProcessIssue"),
            e("h", KiEdgeKind::Calls, "HTTPRequest"),
            e(
                "ki/h::action_def::1::HTTPRequest.code",
                KiEdgeKind::BindsResult,
                "LOCAL::STATUS"
            ),
            e(
                "ki/h::action_def::1::HTTPRequest.body",
                KiEdgeKind::BindsResult,
                "LOCAL::RESULT"
            ),
        ]
    );
}

#[test]
fn one_deletes_what_the_other_tests_is_reported() {
    let a = lifted("deleter", CAL, "Done == \"1\"", "delete(ProcessIssue)");
    let b = lifted("tester", CAL, "ProcessIssue == \"t\"", "log(\"x\")");
    let pairs = interfering_pairs(&[a.clone(), b.clone()]);
    assert_eq!(pairs.len(), 1);
    assert_eq!(
        (pairs[0].first.as_str(), pairs[0].second.as_str()),
        ("deleter", "tester")
    );
    assert_eq!(pairs[0].names.len(), 1);
    assert_eq!(pairs[0].names[0].name, "ProcessIssue");
    assert_eq!(pairs[0].names[0].contact, Contact::WriteRead);
    // The other order reads as read then write.
    let pairs = interfering_pairs(&[b, a]);
    assert_eq!(pairs[0].names[0].contact, Contact::ReadWrite);
}

#[test]
fn write_write_is_reported() {
    let a = lifted("a", CAL, "X == \"1\"", "Counter = 1");
    let b = lifted("b", CAL, "Y == \"1\"", "Counter = 2");
    let pairs = interfering_pairs(&[a, b]);
    assert_eq!(pairs[0].names[0].contact, Contact::WriteWrite);
}

#[test]
fn different_classes_are_not_reported() {
    let a = lifted("deleter", CAL, "Done == \"1\"", "delete(ProcessIssue)");
    let b = lifted("tester", TASK, "ProcessIssue == \"t\"", "log(\"x\")");
    assert!(interfering_pairs(&[a, b]).is_empty());
}

#[test]
fn either_on_any_vertex_is_reported() {
    let a = lifted(
        "deleter",
        "ogit/_id",
        "Done == \"1\"",
        "delete(ProcessIssue)",
    );
    let b = lifted("tester", TASK, "ProcessIssue == \"t\"", "log(\"x\")");
    assert_eq!(interfering_pairs(&[a.clone(), b.clone()]).len(), 1);
    assert_eq!(interfering_pairs(&[b, a]).len(), 1);
}

#[test]
fn disjoint_names_and_read_read_are_not_reported() {
    let a = lifted("a", CAL, "Shared == \"1\"", "Mine = 1");
    let b = lifted("b", CAL, "Shared == \"1\"", "Yours = 1");
    assert!(
        interfering_pairs(&[a, b]).is_empty(),
        "two readers do not interfere"
    );
}
