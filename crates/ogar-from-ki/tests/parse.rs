//! The documented subset parses; everything else is refused by name.

mod common;

use common::{HTTP, SSH_LINE, TUTORIAL, item, ssh_wrapped};
use ogar_from_ki::{ErrorKind, KnowledgeItem, ParseError, outline, parse};

fn refused(src: &str) -> ParseError {
    parse(src).expect_err("must be refused")
}

#[test]
fn fixtures_are_the_public_examples_byte_for_byte() {
    assert!(TUTORIAL.starts_with("ki\n  name: \"I decide that nothing should be done.\"\n"));
    assert!(
        HTTP.contains("\n\tcapability: \"HTTPRequest\",\n"),
        "tabs kept"
    );
    assert_eq!(
        SSH_LINE,
        "  exit: LOCAL::SYSTEMRC = action(\"ExecuteCommand\", command: \"echo Hello World\", host: \"{ip of your target system}\", user: \"{user for your target system}\", timeout: \"20\")\n"
    );
}

#[test]
fn golden_tutorial_tree() {
    let item = parse(TUTORIAL).expect("parses");
    assert_eq!(
        outline(&item),
        "\
ki@1:1
  name@2:9 \"I decide that nothing should be done.\"
  description@3:16 \"I decide that nothing should be done, and finish the task.\"
on@4:1
  (eq@5:3 ogit/_type \"ogit/Schedule/Calendar\")
when@6:1
  (eq@7:3 ogit/Automation/issueType \"RescheduleMeetings\")
  (bare@8:3 ProcessIssue)
do@9:1
  (assign@10:3 NumberOfMeetingsRescheduled num 0)
  (delete@11:3 ProcessIssue)
"
    );
    assert_eq!(item.source, TUTORIAL);
}

#[test]
fn golden_http_tree() {
    let item = parse(HTTP).expect("parses");
    assert_eq!(
        outline(&item),
        "\
on@1:1
  (bare@2:3 ogit/_id)
when@3:1
  (eq@4:3 ProcessIssue \"TestHTTPRequest\")
do@5:1
  (action@6:3 \"HTTPRequest\" named [code -> LOCAL::STATUS] [body -> LOCAL::RESULT] params=\"format=json\" url=\"https://api64.ipify.org/\" timeout=\"10\")
  (if@13:3 (eq@13:6 LOCAL::STATUS \"200\")
    then
      (log@14:5 \"SUCCESS: Your IP adress is ${LOCAL::RESULT}\")
    else
      (log@16:5 \"ERROR ${LOCAL::STATUS}: ${LOCAL::RESULT}\")
  )
  (delete@18:3 ProcessIssue)
"
    );
}

#[test]
fn golden_ssh_tree_in_synthetic_wrapper() {
    let item = parse(&ssh_wrapped()).expect("parses");
    assert_eq!(
        outline(&item),
        "\
on@1:1
  (bare@2:3 ogit/_id)
when@3:1
  (bare@4:3 Synthetic)
do@5:1
  (action@6:3 \"ExecuteCommand\" positional [exit -> LOCAL::SYSTEMRC] command=\"echo Hello World\" host=\"{ip of your target system}\" user=\"{user for your target system}\" timeout=\"20\")
"
    );
}

#[test]
fn spans_have_start_and_end() {
    let item: KnowledgeItem = parse(TUTORIAL).unwrap();
    let name = item.header.as_ref().unwrap().name.as_ref().unwrap();
    assert_eq!((name.span.start.line, name.span.start.col), (2, 9));
    assert_eq!((name.span.end.line, name.span.end.col), (2, 48));
    assert_eq!(item.span.start.line, 1);
    assert_eq!(item.span.end.line, 11);
}

// ── refusals: each names the construct and the line ──────────────────

// CHANGED: the guide's matching page lists `!=` and `>` in its operator table,
// so they are read now (they were refused when only three examples were known).
// What is still refused is an operator the table does not list.
#[test]
fn reads_not_equal_and_greater_than_and_refuses_unlisted_operators() {
    let ok = parse(&item("ProcessIssue != \"x\"", "delete(ProcessIssue)")).expect("parses");
    assert_eq!(ok.when.conds.len(), 1);
    let ok = parse(&item("Count > \"3\"", "delete(ProcessIssue)")).expect("parses");
    assert_eq!(ok.when.conds.len(), 1);
    let e = refused(&item("Count <> \"3\"", "delete(ProcessIssue)"));
    assert_eq!(e.kind, ErrorKind::UnsupportedOperator { op: "<>".into() });
    assert_eq!(e.pos.line, 4);
    assert!(e.to_string().contains("line 4"), "{e}");
}

#[test]
fn refuses_single_equals_as_comparison() {
    let e = refused(&item("Count = \"3\"", "delete(ProcessIssue)"));
    assert_eq!(e.kind, ErrorKind::UnsupportedOperator { op: "=".into() });
}

// CHANGED: a number on the right of a comparison is documented (`StepCounter
// < 1000`, `"ABC2.5" > 0`). A number where a condition starts is still refused.
#[test]
fn number_on_the_right_is_read_and_number_on_the_left_is_refused() {
    parse(&item("Count == 3", "delete(ProcessIssue)")).expect("parses");
    let e = refused(&item("3", "delete(ProcessIssue)"));
    assert_eq!(e.kind, ErrorKind::NumberInCondition { text: "3".into() });
    assert_eq!(e.pos.line, 4);
}

// CHANGED: the guide documents a long list of functions, so a call to any
// function name is recorded as a generic call and given no meaning.
#[test]
fn any_function_is_recorded_as_a_generic_call() {
    let it = parse(&item("ProcessIssue", "send_mail(\"x\")")).expect("parses");
    let ogar_from_ki::ast::Stmt::Call(c) = &it.do_.stmts[0] else {
        panic!("a generic call");
    };
    assert_eq!(c.function.text, "send_mail");
    assert_eq!((c.span.start.line, c.span.start.col), (6, 3));
}

#[test]
fn refuses_unknown_clause_keyword() {
    let src = "on\n  ogit/_id\nunless\n  X\ndo\n  delete(X)\n";
    let e = refused(src);
    assert_eq!(
        e.kind,
        ErrorKind::UnknownClauseKeyword {
            word: "unless".into()
        }
    );
    assert_eq!(e.pos.line, 3);
    // A flush-left unknown word inside the `when` body.
    let src = "on\n  ogit/_id\nwhen\n  X\nfoo\n  Y\ndo\n  delete(X)\n";
    let e = refused(src);
    assert_eq!(
        e.kind,
        ErrorKind::UnknownClauseKeyword { word: "foo".into() }
    );
    assert_eq!(e.pos.line, 5);
}

#[test]
fn refuses_missing_do() {
    let e = refused("on\n  ogit/_id\nwhen\n  X\n");
    assert_eq!(e.kind, ErrorKind::MissingClause { clause: "do" });
    let e = refused("on\n  ogit/_id\n");
    assert_eq!(e.kind, ErrorKind::MissingClause { clause: "when" });
    let e = refused("");
    assert_eq!(e.kind, ErrorKind::MissingClause { clause: "on" });
}

#[test]
fn refuses_empty_do_and_out_of_order_clauses() {
    let e = refused("on\n  ogit/_id\nwhen\n  X\ndo\n");
    assert_eq!(e.kind, ErrorKind::EmptyClause { clause: "do" });
    let e = refused("on\n  ogit/_id\ndo\n  delete(X)\nwhen\n  X\n");
    assert!(matches!(e.kind, ErrorKind::UnexpectedClause { .. }), "{e}");
}

#[test]
fn refuses_unterminated_string() {
    let e = refused(&item("ProcessIssue", "log(\"abc)"));
    assert_eq!(e.kind, ErrorKind::UnterminatedString);
    assert_eq!(e.pos.line, 6);
    assert_eq!(e.pos.col, 7);
    assert!(e.to_string().contains("unterminated string"), "{e}");
}

#[test]
fn refuses_if_without_end() {
    let src = "on\n  ogit/_id\nwhen\n  X\ndo\n  if A == \"1\" then\n    log(\"a\")\n  else\n    log(\"b\")\n";
    let e = refused(src);
    assert_eq!(e.kind, ErrorKind::UnterminatedIf);
    assert_eq!(e.pos.line, 6, "points at the `if`");
    assert!(e.to_string().contains("`if` without `end`"), "{e}");
}

#[test]
fn if_without_else_is_read_and_nested_if_is_refused() {
    // CHANGED: the guide's `if` page calls the `else` block optional.
    let src = "on\n  ogit/_id\nwhen\n  X\ndo\n  if A == \"1\" then\n    log(\"a\")\n  end\n";
    let it = parse(src).expect("parses");
    let ogar_from_ki::ast::Stmt::If(i) = &it.do_.stmts[0] else {
        panic!("an if");
    };
    assert!(i.else_branch.is_none());
    let src = "on\n  ogit/_id\nwhen\n  X\ndo\n  if A == \"1\" then\n    if B == \"2\" then\n      log(\"a\")\n    else\n      log(\"b\")\n    end\n  else\n    log(\"c\")\n  end\n";
    let e = refused(src);
    assert_eq!(e.kind, ErrorKind::NestedIf);
    assert_eq!(e.pos.line, 7);
}

#[test]
fn refuses_nested_action_in_argument() {
    let e = refused(&item(
        "X",
        "action(capability: \"Outer\", cmd: action(capability: \"Inner\"))",
    ));
    assert_eq!(
        e.kind,
        ErrorKind::NestedCall {
            name: "action".into()
        }
    );
    assert_eq!(e.pos.line, 6);
    assert!(e.to_string().contains("nested call `action(`"), "{e}");
    // A call as the first positional argument is refused the same way.
    let e = refused(&item("X", "action(action(\"Inner\"))"));
    assert_eq!(
        e.kind,
        ErrorKind::NestedCall {
            name: "action".into()
        }
    );
}

#[test]
fn refuses_other_undocumented_forms() {
    // CHANGED: `#` starts a comment (structure page, "Comments"), so a
    // trailing comment is read.
    parse(&item("X", "log(\"a\") # note")).expect("parses");
    // an unknown character outside a string
    assert!(matches!(
        refused(&item("X", "log(\"a\") ; note")).kind,
        ErrorKind::UnexpectedChar { ch: ';' }
    ));
    // escaped quote in a string
    assert_eq!(
        refused(&item("X", "log(\"a\\\"b\")")).kind,
        ErrorKind::EscapeInString
    );
    // CHANGED: a number as an argument value is documented (`timeout: 60`).
    parse(&item("X", "action(\"C\", timeout: 10)")).expect("parses");
    // trailing comma, duplicate, missing capability
    assert!(matches!(
        refused(&item("X", "action(\"C\", a: \"1\",)")).kind,
        ErrorKind::UnexpectedToken { .. }
    ));
    assert_eq!(
        refused(&item("X", "action(\"C\", a: \"1\", a: \"2\")")).kind,
        ErrorKind::DuplicateArgument { name: "a".into() }
    );
    assert_eq!(
        refused(&item("X", "action(a: \"1\")")).kind,
        ErrorKind::MissingCapability
    );
    assert_eq!(
        refused(&item("X", "action(\"C\", capability: \"D\")")).kind,
        ErrorKind::DuplicateCapability
    );
    // CHANGED: `and` between conditions is documented (`if` page, structure
    // page), and a result tag may be stored in any variable (`result: date =
    // dateprint(...)` in the guide's date example).
    parse(&item("A == \"1\" and B == \"2\"", "delete(A)")).expect("parses");
    parse(&item("X", "code: Foo = action(\"C\")")).expect("parses");
    // a second item in one text
    assert!(matches!(
        refused(&format!(
            "{}{}",
            item("X", "delete(X)"),
            item("Y", "delete(Y)")
        ))
        .kind,
        ErrorKind::UnexpectedClause { .. }
    ));
    // unknown header key
    assert_eq!(
        refused("ki\n  title: \"x\"\non\n  ogit/_id\nwhen\n  X\ndo\n  delete(X)\n").kind,
        ErrorKind::UnknownHeaderKey {
            key: "title".into()
        }
    );
}

// ── no panic on any input ─────────────────────────────────────────────

fn never_panics(src: &str) {
    match parse(src) {
        Ok(item) => {
            // A tree that parsed renders and lifts without panicking.
            let _ = outline(&item);
            let _ = ogar_from_ki::lift(
                &item,
                &ogar_from_ki::LiftOptions {
                    item_id: Some("p".into()),
                },
            );
        }
        Err(e) => {
            assert!(e.pos.line >= 1 && e.pos.col >= 1, "{e:?}");
            assert!(!e.to_string().is_empty());
            assert!(!e.expected.is_empty());
        }
    }
}

fn texts() -> Vec<String> {
    vec![TUTORIAL.to_string(), HTTP.to_string(), ssh_wrapped()]
}

#[test]
fn every_truncation_of_every_fixture_is_ok_or_named_error() {
    for t in texts() {
        let bytes = t.as_bytes();
        for n in 0..=bytes.len() {
            never_panics(&String::from_utf8_lossy(&bytes[..n]));
        }
    }
}

#[test]
fn seeded_random_mutations_are_ok_or_named_error() {
    // xorshift64: deterministic, no dependency.
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let alphabet: Vec<u8> = b"\"\\ \t\n:,()=!<>/_#${}abzAZ09-.'".to_vec();
    let bases = texts();
    for round in 0..4000 {
        let base = &bases[round % bases.len()];
        let mut b = base.as_bytes().to_vec();
        for _ in 0..(1 + next() % 4) {
            let at = (next() as usize) % b.len();
            match next() % 3 {
                0 => b[at] = alphabet[(next() as usize) % alphabet.len()],
                1 => {
                    b.remove(at);
                }
                _ => b.insert(at, alphabet[(next() as usize) % alphabet.len()]),
            }
            if b.is_empty() {
                b.push(b'\n');
            }
        }
        never_panics(&String::from_utf8_lossy(&b));
    }
}
