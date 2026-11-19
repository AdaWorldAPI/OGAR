//! The ten complete Knowledge Items printed in the HIRO User Guides 7.0 (KI
//! Syntax). Each fixture is copied byte for byte (see
//! `tests/fixtures/guide/SOURCES.md`). Each one is asserted on its real
//! content. All ten parse with the reader as it stands; no fixture needs a
//! pinned refusal, and none is skipped. The guide's own defects that affect
//! what the reader may accept are pinned from the guide's pages in the last
//! tests of this file.

mod common;

use common::{called, guide, shape, stmt_kinds};
use ogar_from_ki::ast::{
    ArgValue, BlockKind, Direction, Expr, GraphScopeKind, KnowledgeItem, Stmt, Tier,
};
use ogar_from_ki::{
    ErrorKind, KiArgKind, LiftOptions, NoteKind, OnTarget, Section, lift_source, parse,
};

fn load(name: &str) -> (String, KnowledgeItem) {
    let src = guide(name);
    let item = parse(&src).unwrap_or_else(|e| panic!("{name}: {e}"));
    (src, item)
}

fn on_shapes(it: &KnowledgeItem) -> Vec<String> {
    it.on.conds().map(shape).collect()
}

fn when_shapes(it: &KnowledgeItem) -> Vec<String> {
    it.when.conds.iter().map(shape).collect()
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(ToString::to_string).collect()
}

struct Head<'a> {
    name: Option<&'a str>,
    id: Option<&'a str>,
    version: &'a str,
    description: &'a str,
    tier: Option<Tier>,
}

fn check_header(it: &KnowledgeItem, want: &Head) {
    let h = it.header.as_ref().expect("a ki header");
    assert_eq!(h.name.as_ref().map(|s| s.value.as_str()), want.name);
    assert_eq!(h.id.as_ref().map(|s| s.value.as_str()), want.id);
    assert_eq!(
        h.version.as_ref().map(|s| s.value.as_str()),
        Some(want.version)
    );
    assert_eq!(
        h.description.as_ref().map(|s| s.value.as_str()),
        Some(want.description)
    );
    assert_eq!(h.tier, want.tier);
    assert_eq!(h.tier_text.is_some(), want.tier.is_some());
}

// ── E1 full_example ─────────────────────────────────────────────────

#[test]
fn full_example() {
    let (_, it) = load("full_example");
    check_header(
        &it,
        &Head {
            name: Some("TestKi"),
            id: None,
            version: "0@NoName",
            description: "Just a simple test KI",
            tier: None,
        },
    );
    assert_eq!(
        on_shapes(&it),
        strs(&["exists", "not(exists)", "(==)", "=~", "and((>),(<))"])
    );
    let blocks: Vec<_> = it.on.blocks().collect();
    assert_eq!(blocks.len(), 1);
    let b = blocks[0];
    assert_eq!(b.kind, BlockKind::With);
    assert_eq!(b.vertex_type.text, "Type");
    assert_eq!(b.direction, Direction::In);
    assert!(b.edge_type.is_none());
    assert_eq!(
        b.alias.as_ref().map(|a| a.text.as_str()),
        Some("ParentScope")
    );
    let inner: Vec<String> = b.conds.iter().map(shape).collect();
    assert_eq!(inner, strs(&["or((==),(==))", "exists"]));
    assert_eq!(
        when_shapes(&it),
        strs(&[
            "exists",
            "exists[key]",
            "==[key]",
            "list exists",
            "exists as",
            "==@in"
        ])
    );
    // `MonitoringStatus@in`: the structure page writes `@any` for the same
    // line. Recorded as written, not read as `@any`.
    let ogar_from_ki::ast::Cond::Leaf(last) = it.when.conds.last().unwrap() else {
        panic!("a leaf");
    };
    assert_eq!(
        last.subject.graph.unwrap().kind,
        GraphScopeKind::InAsWrittenInExample
    );
    assert_eq!(
        stmt_kinds(&it.do_.stmts),
        strs(&["log.info", "assign", "add", "delete", "call:execute", "if"])
    );
    assert_eq!(
        called(&it.do_.stmts),
        strs(&["log.info", "add", "delete", "execute", "log.error", "log"])
    );
    // The tagged result: `stdout: ExecuteOutput, exit: LOCAL::Error = execute(...)`.
    let Stmt::Call(c) = &it.do_.stmts[4] else {
        panic!("a call");
    };
    let tags: Vec<(&str, &str)> = c
        .bindings
        .iter()
        .map(|b| (b.field.text.as_str(), b.target.text.as_str()))
        .collect();
    assert_eq!(
        tags,
        vec![("stdout", "ExecuteOutput"), ("exit", "LOCAL::Error")]
    );
    assert_eq!(c.args.len(), 2);
    assert!(c.args[0].name.is_none(), "the command is positional");
    assert_eq!(c.args[1].name.as_ref().unwrap().text, "timeout");
    assert!(matches!(
        c.args[1].value,
        ArgValue::Expr(Expr::Number(ref n)) if n.text == "60"
    ));
}

// ── E2 retry_action ─────────────────────────────────────────────────

#[test]
fn retry_action() {
    let (_, it) = load("retry_action");
    check_header(
        &it,
        &Head {
            name: Some("TestKi"),
            id: None,
            version: "0@NoName",
            description: "This KI will retry execution up to three times upon AH failure",
            tier: None,
        },
    );
    assert_eq!(on_shapes(&it), strs(&["=="]));
    assert_eq!(it.on.blocks().count(), 0);
    assert_eq!(when_shapes(&it), strs(&["exists", "==", "?< :=", "exists"]));
    assert_eq!(stmt_kinds(&it.do_.stmts), strs(&["assign", "if"]));
    assert_eq!(called(&it.do_.stmts), strs(&["delete", "log.error", "log"]));
    // `log(counter)`: the message is a variable, not a string.
    let Stmt::If(i) = &it.do_.stmts[1] else {
        panic!("an if");
    };
    let else_branch = i.else_branch.as_ref().expect("an else");
    assert_eq!(else_branch.len(), 3);
    assert!(matches!(
        &else_branch[2],
        Stmt::Log { message: Expr::Var(v), .. } if v.name.text == "counter"
    ));
    // The guide's digest: the example never calls an action; it sets a
    // placeholder instead. It lifts to no def.
    let l = lift_source(&guide("retry_action"), &LiftOptions::default()).unwrap();
    assert!(l.defs.is_empty());
    assert!(l.notes.iter().any(|n| n.kind == NoteKind::NoActionCall));
}

// ── E3 add_comment_valuemation ──────────────────────────────────────

#[test]
fn add_comment_valuemation() {
    let (src, it) = load("add_comment_valuemation");
    check_header(
        &it,
        &Head {
            name: Some("TestKi"),
            id: None,
            version: "0@NoName",
            description: "Update the ticket with a comment stored directly in the ToDo AddComment",
            tier: None,
        },
    );
    assert_eq!(on_shapes(&it), strs(&["=="]));
    assert_eq!(when_shapes(&it), strs(&["exists", "exists", "exists"]));
    assert_eq!(stmt_kinds(&it.do_.stmts), strs(&["action", "if"]));
    assert_eq!(called(&it.do_.stmts), strs(&["action", "delete", "log"]));
    let Stmt::Action(a) = &it.do_.stmts[0] else {
        panic!("an action");
    };
    assert_eq!(a.capability.value, "UpdateTicket");
    let tags: Vec<&str> = a.bindings.iter().map(|b| b.field.text.as_str()).collect();
    assert_eq!(tags, vec!["body", "code", "headers", "exit"]);
    let args: Vec<&str> = a.args.iter().map(|g| g.name.text.as_str()).collect();
    assert_eq!(
        args,
        vec![
            "mainDescription",
            "ticketNo",
            "descriptionBody",
            "descriptionTitle",
            "tenantId"
        ]
    );
    // It lifts to one def: the keyword capability, the five arguments, the
    // four tags as results.
    let l = lift_source(&src, &LiftOptions::default()).unwrap();
    assert_eq!(l.defs.len(), 1);
    assert_eq!(l.defs[0].predicate, "UpdateTicket");
    assert_eq!(l.defs[0].results, vec!["body", "code", "headers", "exit"]);
    assert_eq!(l.defs[0].params.len(), 5);
    assert_eq!(
        l.item.on,
        OnTarget::Unresolved {
            attribute: "ogit/name".into(),
            value: Some("default_issue_node".into())
        }
    );
}

// ── E4 fallback_unresolved ──────────────────────────────────────────

#[test]
fn fallback_unresolved() {
    let (src, it) = load("fallback_unresolved");
    check_header(
        &it,
        &Head {
            name: Some("TestKi"),
            id: None,
            version: "0@NoName",
            description: "KI to be trigger before the datagroup.de:Default:AutoStopIdleIssues KI, it will forward the ticket to the responsible support Group for manual resolution",
            tier: Some(Tier::Fallback),
        },
    );
    assert_eq!(on_shapes(&it), strs(&["=="]));
    assert_eq!(
        when_shapes(&it),
        strs(&["?exists@any :=", ">", "exists", "==", "exists", "exists"])
    );
    assert_eq!(stmt_kinds(&it.do_.stmts), strs(&["action", "if"]));
    assert_eq!(called(&it.do_.stmts), strs(&["action", "log", "log"]));
    let l = lift_source(&src, &LiftOptions::default()).unwrap();
    assert_eq!(l.item.tier, Some(Tier::Fallback));
    assert_eq!(l.item.version.as_deref(), Some("0@NoName"));
    assert_eq!(l.defs[0].predicate, "ForwardTicket");
}

// ── E5 checking_spelling ────────────────────────────────────────────

#[test]
fn checking_spelling() {
    let (src, it) = load("checking_spelling");
    // The description holds `mark\.` as printed: a backslash that is not in
    // front of the quote is kept verbatim.
    check_header(
        &it,
        &Head {
            name: Some("TestKi"),
            id: None,
            version: "0@NoName",
            description: "Before start any further steps you have to check if the first and last name contains special letter like ä,ö,ü or accent mark\\. If so you have to remove or correct it",
            tier: Some(Tier::Fallback),
        },
    );
    assert_eq!(on_shapes(&it), strs(&["=="]));
    assert_eq!(when_shapes(&it), strs(&["exists", "exists"]));
    assert_eq!(stmt_kinds(&it.do_.stmts), strs(&["call:javascript", "if"]));
    assert_eq!(
        called(&it.do_.stmts),
        strs(&["javascript", "log", "delete", "log"])
    );
    // The single-quoted script text spans lines and is kept as written.
    let Stmt::Call(c) = &it.do_.stmts[0] else {
        panic!("a call");
    };
    assert_eq!(c.function.text, "javascript");
    let tags: Vec<&str> = c.bindings.iter().map(|b| b.field.text.as_str()).collect();
    assert_eq!(tags, vec!["exit", "result"]);
    let ArgValue::Expr(Expr::Str(s)) = &c.args[0].value else {
        panic!("a string");
    };
    assert_eq!(s.quote, ogar_from_ki::ast::Quote::Single);
    assert!(s.value.starts_with("\n    spechrs = {"));
    assert!(s.value.contains("for(chr in spechrs)"));
    assert!(s.value.ends_with("    str\n"));
    // Lifted: no action call, one generic call recorded by name.
    let l = lift_source(&src, &LiftOptions::default()).unwrap();
    assert!(l.defs.is_empty());
    assert_eq!(l.item.other_calls.len(), 1);
    assert_eq!(l.item.other_calls[0].function, "javascript");
}

// ── E6 working_with_dates ───────────────────────────────────────────

#[test]
fn working_with_dates() {
    let (_, it) = load("working_with_dates");
    check_header(
        &it,
        &Head {
            name: Some("TestKi"),
            id: None,
            version: "0@NoName",
            description: "example on how to manipulate dates",
            tier: None,
        },
    );
    assert_eq!(on_shapes(&it), strs(&["=="]));
    assert_eq!(when_shapes(&it), strs(&["exists"]));
    assert_eq!(
        stmt_kinds(&it.do_.stmts),
        strs(&[
            "call:dateprint",
            "log",
            "assign",
            "call:javascript",
            "log",
            "call:dateprint",
            "log"
        ])
    );
    assert_eq!(
        called(&it.do_.stmts),
        strs(&["dateprint", "log", "javascript", "log", "dateprint", "log"])
    );
    // `result: date = dateprint(...)`: a tag stored in an unscoped variable.
    let Stmt::Call(c) = &it.do_.stmts[5] else {
        panic!("a call");
    };
    assert_eq!(c.bindings[0].target.text, "date");
    let kw: Vec<&str> = c
        .args
        .iter()
        .map(|a| a.name.as_ref().unwrap().text.as_str())
        .collect();
    assert_eq!(kw, vec!["timestamp", "format"]);
}

// ── E7 querying_graph ───────────────────────────────────────────────

#[test]
fn querying_graph() {
    let (_, it) = load("querying_graph");
    check_header(
        &it,
        &Head {
            name: Some("TestKi"),
            id: None,
            version: "0@NoName",
            description: "example on how to query the graph",
            tier: None,
        },
    );
    assert_eq!(on_shapes(&it), strs(&["=="]));
    assert_eq!(when_shapes(&it), strs(&["=="]));
    assert_eq!(
        stmt_kinds(&it.do_.stmts),
        strs(&["assign", "call:query", "if"])
    );
    // The `each: begin ... end` block holds an assignment and a log call.
    assert_eq!(
        called(&it.do_.stmts),
        strs(&["query", "log", "delete", "log"])
    );
    let Stmt::Call(c) = &it.do_.stmts[1] else {
        panic!("a call");
    };
    assert_eq!(c.args.len(), 4);
    let ArgValue::Expr(Expr::Str(q)) = &c.args[1].value else {
        panic!("a string");
    };
    // `\/` is kept as written; no escape is interpreted.
    assert_eq!(q.value, "ogit\\/_type:ogit\\/MARS\\/Machine ");
    assert_eq!(c.args[3].name.as_ref().unwrap().text, "each");
    let ArgValue::Block(b) = &c.args[3].value else {
        panic!("a begin block");
    };
    assert_eq!(stmt_kinds(&b.stmts), strs(&["assign", "log"]));
}

// ── E8 / E9 lists ───────────────────────────────────────────────────

#[test]
fn lists_ki1_delete_files_on() {
    let (_, it) = load("lists_ki1_delete_files_on");
    check_header(
        &it,
        &Head {
            name: Some("KI 1"),
            id: None,
            version: "0@NoName",
            description: "handling list",
            tier: None,
        },
    );
    assert_eq!(on_shapes(&it), strs(&["=="]));
    assert_eq!(when_shapes(&it), strs(&["exists"]));
    assert_eq!(stmt_kinds(&it.do_.stmts), strs(&["log", "delete"]));
    assert_eq!(called(&it.do_.stmts), strs(&["log", "delete"]));
}

#[test]
fn lists_ki2_list_delete_files_on() {
    let (_, it) = load("lists_ki2_list_delete_files_on");
    check_header(
        &it,
        &Head {
            name: Some("KI 2"),
            id: None,
            version: "0@NoName",
            description: "handling list",
            tier: None,
        },
    );
    assert_eq!(on_shapes(&it), strs(&["=="]));
    // The only difference from KI 1: `list DeleteFilesOn` in `when`.
    assert_eq!(when_shapes(&it), strs(&["list exists"]));
    assert_eq!(stmt_kinds(&it.do_.stmts), strs(&["log", "delete"]));
    assert_eq!(called(&it.do_.stmts), strs(&["log", "delete"]));
}

// ── E10 rl_hn3_r1 ───────────────────────────────────────────────────

#[test]
fn rl_hn3_r1() {
    let (src, it) = load("rl_hn3_r1");
    // `id:` where the structure page requires `name:`: recorded as its own
    // header key; `name` stays absent.
    check_header(
        &it,
        &Head {
            name: None,
            id: Some("HN3_R1"),
            version: "0@rs",
            description: "Move disc 3 to position 1.",
            tier: None,
        },
    );
    assert_eq!(on_shapes(&it), strs(&["exists"]));
    assert_eq!(
        when_shapes(&it),
        strs(&[
            "==",
            "exists[key] as",
            "exists[key] as",
            "!=[key]",
            "<",
            "exists",
            "exists"
        ])
    );
    assert_eq!(
        stmt_kinds(&it.do_.stmts),
        strs(&["assign", "assign", "assign", "assign"])
    );
    assert!(called(&it.do_.stmts).is_empty());
    // The lift needs an identity; `id:` is not read as the item's identity.
    assert!(lift_source(&src, &LiftOptions::default()).is_err());
    let l = lift_source(
        &src,
        &LiftOptions {
            item_id: Some("hn3".into()),
        },
    )
    .unwrap();
    assert_eq!(l.item.on, OnTarget::AnyVertex);
    assert!(l.defs.is_empty());
}

// ── what the lift keeps from the ten ────────────────────────────────

#[test]
fn full_example_lift_reports_execute_and_the_conditions_it_does_not_state() {
    let l = lift_source(&guide("full_example"), &LiftOptions::default()).unwrap();
    assert!(l.defs.is_empty(), "no action call");
    assert_eq!(l.item.other_calls.len(), 1);
    assert_eq!(l.item.other_calls[0].function, "execute");
    assert_eq!(
        l.item.other_calls[0].kind,
        ogar_from_ki::OtherCallKind::Execute
    );
    assert_eq!(l.item.other_calls[0].results.len(), 2);
    assert!(l.notes.iter().any(|n| n.kind == NoteKind::ExecuteNotLifted));
    // Conditions beyond a name or a string equality are noted, not stated.
    let beyond = l
        .notes
        .iter()
        .filter(|n| matches!(n.kind, NoteKind::ConditionBeyondLift { .. }))
        .count();
    assert_eq!(beyond, 5, "VarB[key], VarC[VarA], list, alias, @in");
    // The variables they name are still reads.
    assert!(l.item.reads.contains(&"VarC".to_string()));
    assert!(l.item.reads.contains(&"NodeType".to_string()));
    // `execute`'s tags and the `if` over `Error`.
    assert!(l.item.locals_written.contains(&"LOCAL::Error".to_string()));
}

#[test]
fn query_each_block_is_walked_for_writes() {
    let l = lift_source(&guide("querying_graph"), &LiftOptions::default()).unwrap();
    assert!(l.item.locals_written.contains(&"LOCAL::NODEID".to_string()));
    assert!(l.item.locals_written.contains(&"LOCAL::OUTPUT".to_string()));
    assert_eq!(l.item.other_calls[0].function, "query");
    assert_eq!(l.item.writes, vec!["GraphQuery"]);
}

#[test]
fn action_arguments_keep_their_kind() {
    let src = "on\n  ogit/_id\nwhen\n  X\ndo\n  action(\"C\", timeout: 60, host: Target, cmd: \"echo\")\n";
    let l = lift_source(
        src,
        &LiftOptions {
            item_id: Some("k".into()),
        },
    )
    .unwrap();
    let a = &l.item.calls[0].args;
    assert_eq!((a[0].value.as_str(), a[0].kind), ("60", KiArgKind::Number));
    assert_eq!(
        (a[1].value.as_str(), a[1].kind),
        ("Target", KiArgKind::Name)
    );
    assert_eq!((a[2].value.as_str(), a[2].kind), ("echo", KiArgKind::Str));
}

// ── the guide's own defects, pinned from its pages ──────────────────

/// The `action` reference page prints two examples. The second is
/// ```text
///  exit: LOCAL::RESULT = action
///  stdout: LOCAL::OUTPUT = action(
///  "ExecuteLocalCommand", command: "df -h | ..."
///  )
/// ```
/// The first call has no argument list. The digest lists it as a defect of
/// the guide. It is refused by name, at the word `action` of line 6.
#[test]
fn action_page_example_with_no_argument_list_is_refused_by_name() {
    let src = "on\n  ogit/_id\nwhen\n  X\ndo\n  exit: LOCAL::RESULT = action\n  stdout: LOCAL::OUTPUT = action(\n  \"ExecuteLocalCommand\", command: \"df -h\"\n  )\n";
    let e = parse(src).expect_err("refused");
    assert_eq!(
        e.kind,
        ErrorKind::CallWithoutArguments {
            function: "action".into()
        }
    );
    assert_eq!((e.pos.line, e.pos.col), (6, 25));
    assert!(
        e.to_string().contains("`action` with no argument list"),
        "{e}"
    );
}

/// The same page's first example is complete and parses: a positional
/// capability and a keyword argument, one tag.
#[test]
fn action_page_first_example_parses() {
    let src = "on\n  ogit/_id\nwhen\n  X\ndo\n exit: LOCAL::RESULT = action(\n \"ExecuteLocalCommand\",\n command: \"df -h > /mnt/disk_usage.log\"\n )\n";
    let it = parse(src).expect("parses");
    let Stmt::Action(a) = &it.do_.stmts[0] else {
        panic!("an action");
    };
    assert_eq!(a.capability.value, "ExecuteLocalCommand");
}

/// The matching page's "Related count" example writes `ogit/_type =
/// ogit/Person`, a single `=` in a condition. The operator table lists only
/// `==`. Refused by name; not read as `==`.
#[test]
fn matching_page_single_equals_in_the_count_example_is_refused() {
    let src = "on\n ogit/_type = ogit/Person\nwhen\n  X\ndo\n  log(\"x\")\n";
    let e = parse(src).expect_err("refused");
    assert_eq!(e.kind, ErrorKind::UnsupportedOperator { op: "=".into() });
    assert_eq!((e.pos.line, e.pos.col), (2, 13));
}

/// Which sections the condition rows of the ten came from: every fixture has
/// rows for `on` and `when` where it has conditions.
#[test]
fn every_fixture_has_condition_rows_in_both_sections() {
    for name in [
        "full_example",
        "retry_action",
        "add_comment_valuemation",
        "fallback_unresolved",
        "checking_spelling",
        "working_with_dates",
        "querying_graph",
        "lists_ki1_delete_files_on",
        "lists_ki2_list_delete_files_on",
        "rl_hn3_r1",
    ] {
        let (_, it) = load(name);
        let rows = it.condition_refs();
        assert!(rows.iter().any(|r| r.section == Section::On), "{name}");
        assert!(rows.iter().any(|r| r.section == Section::When), "{name}");
    }
}
