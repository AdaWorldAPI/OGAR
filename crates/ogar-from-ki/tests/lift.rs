//! The lift: the three public examples, the notes, the bare-name rule.

mod common;

use common::{HTTP, TUTORIAL, item, ssh_wrapped};
use ogar_from_ki::{
    Branch, ConditionSite, KiCondition, LiftError, LiftOptions, NoteKind, OnTarget, Pos,
    SourceError, lift_source,
};
use ogar_vocab::{CallParam, KausalSpec};

fn opts(id: &str) -> LiftOptions {
    LiftOptions {
        item_id: Some(id.to_string()),
    }
}

fn p(line: usize, col: usize) -> Pos {
    Pos { line, col }
}

fn kinds(l: &ogar_from_ki::Lifted) -> Vec<(NoteKind, Option<Pos>, Option<usize>)> {
    l.notes
        .iter()
        .map(|n| (n.kind.clone(), n.pos, n.call))
        .collect()
}

#[test]
fn tutorial_item_lifts_to_no_def_but_keeps_facts_and_notes() {
    let l = lift_source(TUTORIAL, &LiftOptions::default()).expect("lifts");
    assert!(l.defs.is_empty(), "no action call, no def");
    let it = &l.item;
    assert_eq!(it.id, "I decide that nothing should be done.");
    assert_eq!(
        it.description.as_deref(),
        Some("I decide that nothing should be done, and finish the task.")
    );
    assert_eq!(it.on, OnTarget::Class("ogit/Schedule/Calendar".into()));
    assert_eq!(it.reads, vec!["ogit/Automation/issueType", "ProcessIssue"]);
    assert_eq!(
        it.writes,
        vec!["NumberOfMeetingsRescheduled", "ProcessIssue"]
    );
    assert!(it.calls.is_empty());
    assert_eq!(
        kinds(&l),
        vec![
            (
                NoteKind::WhenConjunctionUndocumented { count: 2 },
                Some(p(6, 1)),
                None
            ),
            (
                NoteKind::UnresolvedBareName {
                    name: "ProcessIssue".into(),
                    site: ConditionSite::When
                },
                Some(p(8, 3)),
                None
            ),
            (NoteKind::NoActionCall, Some(p(9, 1)), None),
        ]
    );
}

#[test]
fn http_item_lifts_to_one_def() {
    let l = lift_source(HTTP, &opts("http-request")).expect("lifts");
    assert_eq!(l.defs.len(), 1);
    let d = &l.defs[0];
    assert_eq!(d.identity, "ki/http-request::action_def::1::HTTPRequest");
    assert_eq!(d.predicate, "HTTPRequest");
    assert_eq!(d.object_class, "", "`on ogit/_id` names no class");
    assert_eq!(d.calls, vec!["HTTPRequest"]);
    assert_eq!(
        d.params,
        vec![
            CallParam::owned("params", false),
            CallParam::owned("url", false),
            CallParam::owned("timeout", false)
        ]
    );
    assert_eq!(d.results, vec!["code", "body"]);
    assert_eq!(d.reads, vec!["ProcessIssue"]);
    assert_eq!(d.writes, vec!["ProcessIssue"], "the delete");
    assert_eq!(
        d.kausal,
        Some(KausalSpec::state_guard(
            "ProcessIssue",
            vec!["TestHTTPRequest".into()]
        ))
    );
    assert_eq!(d.body_source.as_deref(), Some(HTTP));
    assert!(d.raises.is_empty() && d.on_enter.is_none());

    // The literal values are bindings on the KiCall, not in the def.
    let c = &l.item.calls[0];
    assert_eq!(c.args[1].name, "url");
    assert_eq!(c.args[1].value, "https://api64.ipify.org/");
    assert_eq!(c.results[0].field, "code");
    assert_eq!(c.results[0].local, "LOCAL::STATUS");
    assert!(c.condition.is_none());
    assert_eq!(
        l.item.locals_written,
        vec!["LOCAL::STATUS", "LOCAL::RESULT"]
    );
    assert_eq!(l.item.locals_read, vec!["LOCAL::STATUS"]);
    // LOCAL names are not stored fields: they are not in reads or writes.
    assert!(
        !d.reads
            .iter()
            .chain(&d.writes)
            .any(|n| n.starts_with("LOCAL::"))
    );

    assert_eq!(
        kinds(&l),
        vec![
            (NoteKind::OnAnyVertex, Some(p(1, 1)), None),
            (
                NoteKind::ParamMandatoryUnknown {
                    capability: "HTTPRequest".into()
                },
                Some(p(6, 3)),
                Some(1)
            ),
        ]
    );
}

#[test]
fn ssh_line_lifts_with_positional_capability() {
    let l = lift_source(&ssh_wrapped(), &opts("ssh")).expect("lifts");
    let d = &l.defs[0];
    assert_eq!(d.predicate, "ExecuteCommand");
    assert_eq!(d.results, vec!["exit"]);
    let names: Vec<&str> = d.params.iter().map(|p| p.name.as_ref()).collect();
    assert_eq!(names, vec!["command", "host", "user", "timeout"]);
    assert_eq!(l.item.calls[0].results[0].local, "LOCAL::SYSTEMRC");
    assert_eq!(l.item.calls[0].args[0].value, "echo Hello World");
    assert!(d.kausal.is_none(), "a bare name states no precondition");
    assert!(
        kinds(&l)
            .iter()
            .any(|(k, ..)| *k == NoteKind::PreconditionNotStated)
    );
}

#[test]
fn identity_rules() {
    // The name wins over a caller id.
    let l = lift_source(TUTORIAL, &opts("other")).unwrap();
    assert_eq!(l.item.id, "I decide that nothing should be done.");
    // No name and no id: refused, not invented.
    let e = lift_source(HTTP, &LiftOptions::default()).unwrap_err();
    assert_eq!(e, SourceError::Lift(LiftError::NoIdentity));
    // Parse refusals pass through.
    assert!(matches!(
        lift_source("nope", &opts("x")),
        Err(SourceError::Parse(_))
    ));
    // The ordinal counts calls in text order.
    let src = item("X == \"1\"", "action(\"A\")\n  action(\"B\")");
    let l = lift_source(&src, &opts("two")).unwrap();
    let ids: Vec<&str> = l.defs.iter().map(|d| d.identity.as_str()).collect();
    assert_eq!(
        ids,
        vec!["ki/two::action_def::1::A", "ki/two::action_def::2::B"]
    );
}

#[test]
fn bare_name_is_unresolved_and_never_an_equality() {
    // SYNTHETIC
    let src = item("Flag", "action(\"Run\")");
    let l = lift_source(&src, &opts("bare")).unwrap();
    assert_eq!(
        l.item.when,
        vec![KiCondition::BareName {
            name: "Flag".into(),
            pos: p(4, 3)
        }]
    );
    assert!(
        !l.item
            .when
            .iter()
            .any(|c| matches!(c, KiCondition::Equals { .. })),
        "a bare name is never an equality"
    );
    assert_eq!(l.defs[0].kausal, None, "no precondition is stated for it");
    assert_eq!(
        l.defs[0].reads,
        vec!["Flag"],
        "it is still a name the item tests"
    );
    let ks = kinds(&l);
    assert!(ks.contains(&(
        NoteKind::UnresolvedBareName {
            name: "Flag".into(),
            site: ConditionSite::When
        },
        Some(p(4, 3)),
        None
    )));
    assert!(
        ks.iter()
            .any(|(k, ..)| *k == NoteKind::PreconditionNotStated)
    );

    // A bare name in an `if` is reported too.
    let src = item(
        "X == \"1\"",
        "if Flag then\n    log(\"a\")\n  else\n    log(\"b\")\n  end",
    );
    let l = lift_source(&src, &opts("bare-if")).unwrap();
    assert!(kinds(&l).iter().any(|(k, ..)| matches!(
        k,
        NoteKind::UnresolvedBareName {
            site: ConditionSite::If,
            ..
        }
    )));
}

#[test]
fn single_equality_is_a_state_guard_and_two_conditions_are_not() {
    let l = lift_source(&item("Mode == \"fast\"", "action(\"A\")"), &opts("one")).unwrap();
    assert_eq!(
        l.defs[0].kausal,
        Some(KausalSpec::state_guard("Mode", vec!["fast".into()]))
    );
    let src = "on\n  ogit/_id\nwhen\n  Mode == \"fast\"\n  Other == \"x\"\ndo\n  action(\"A\")\n";
    let l = lift_source(src, &opts("two")).unwrap();
    assert_eq!(l.defs[0].kausal, None);
    assert!(
        kinds(&l)
            .iter()
            .any(|(k, ..)| *k == NoteKind::WhenConjunctionUndocumented { count: 2 })
    );
}

#[test]
fn on_forms() {
    let any = lift_source(&item("X", "action(\"A\")"), &opts("a")).unwrap();
    assert_eq!(any.item.on, OnTarget::AnyVertex);
    assert!(
        kinds(&any)
            .iter()
            .any(|(k, ..)| *k == NoteKind::OnAnyVertex)
    );

    let src = "on\n  ogit/_type == \"ogit/Task\"\nwhen\n  X\ndo\n  action(\"A\")\n";
    let c = lift_source(src, &opts("c")).unwrap();
    assert_eq!(c.defs[0].object_class, "ogit/Task");

    // Another attribute: what it names is not documented.
    let src = "on\n  ogit/name == \"ogit/Task\"\nwhen\n  X\ndo\n  action(\"A\")\n";
    let u = lift_source(src, &opts("u")).unwrap();
    assert_eq!(u.defs[0].object_class, "");
    assert!(matches!(u.item.on, OnTarget::Unresolved { .. }));
    assert!(
        kinds(&u)
            .iter()
            .any(|(k, ..)| matches!(k, NoteKind::OnNotAClass { .. }))
    );
}

#[test]
fn calls_in_if_branches_are_flagged_conditional() {
    // SYNTHETIC
    let src = item(
        "X == \"1\"",
        "if Mode == \"a\" then\n    action(\"First\")\n  else\n    action(\"Second\")\n  end",
    );
    let l = lift_source(&src, &opts("cond")).unwrap();
    assert_eq!(l.defs.len(), 2);
    let c1 = l.item.calls[0].condition.expect("conditional");
    assert_eq!(c1.branch, Branch::Then);
    assert_eq!(
        l.item.calls[1].condition.expect("conditional").branch,
        Branch::Else
    );
    assert_eq!(
        l.item.reads,
        vec!["X", "Mode"],
        "the `if` condition is a read"
    );
    let n = kinds(&l)
        .into_iter()
        .filter(|(k, ..)| matches!(k, NoteKind::ConditionalCall { .. }))
        .count();
    assert_eq!(n, 2);
}
