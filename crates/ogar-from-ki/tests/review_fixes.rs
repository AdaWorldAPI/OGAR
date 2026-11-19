//! Regression tests for review findings on the reader.

mod common;

use common::shape;
use ogar_from_ki::ast::{ArgValue, Expr, Stmt};
use ogar_from_ki::{LiftOptions, NoteKind, OnTarget, lift_source, parse};

fn opts() -> LiftOptions {
    LiftOptions {
        item_id: Some("r".into()),
    }
}

fn when_src(when: &str) -> String {
    format!("on\n  ogit/_type == \"T\"\nwhen\n  {when}\ndo\n  action(\"Run\")\n")
}

// types page: "an optional leading + or -" on integers and floats.
#[test]
fn leading_plus_on_a_number_is_read() {
    let it = parse(&when_src("Count > +1")).expect("`+1` is a number");
    assert_eq!(shape(&it.when.conds[0]), ">");
    let it = parse(&when_src("Count > +1.5")).expect("parses");
    assert_eq!(shape(&it.when.conds[0]), ">");
    // after a value, `+` is still addition
    let it = parse(&when_src("Bar == Foo +1")).expect("parses");
    let ogar_from_ki::ast::Cond::Leaf(l) = &it.when.conds[0] else {
        panic!()
    };
    assert!(matches!(
        l.test.as_ref().unwrap().right,
        Expr::Binary { .. }
    ));
}

// regex page: `expression: /value ([0-9.]+)/i` as a call argument.
#[test]
fn regex_literal_as_a_call_argument() {
    let src = "on\n  ogit/_id\nwhen\n  X\ndo\n  result: LOCAL::RESULT = regex(\"VaLuE: 42.0\", expression: /value ([0-9.]+)/i)\n  result: LOCAL::R2 = regex(\"VaLuE: 42.0\", expression: /value ([0-9.]+)/i, options: \"\")\n";
    let it = parse(src).expect("the page's own example parses");
    for s in &it.do_.stmts {
        let Stmt::Call(c) = s else { panic!() };
        let ArgValue::Expr(Expr::Regex(r)) = &c.args[1].value else {
            panic!("a regex literal");
        };
        assert_eq!(
            (r.pattern.as_str(), r.flags.as_str()),
            ("value ([0-9.]+)", "i")
        );
    }
    // division after an operand is unchanged
    parse("on\n  ogit/_id\nwhen\n  X\ndo\n  LOCAL::r = a / b\n").expect("parses");
}

fn guard_of(when: &str) -> Option<ogar_vocab::KausalSpec> {
    lift_source(&when_src(when), &opts()).unwrap().defs[0]
        .kausal
        .clone()
}

// Only an unqualified name becomes a literal field guard.
#[test]
fn qualified_or_keyed_names_are_not_guards() {
    assert!(guard_of("Status == \"ready\"").is_some());
    for w in [
        "NODE::Status == \"ready\"",
        "ISSUE::Status == \"ready\"",
        "LOCAL::Status == \"ready\"",
        "Node::Status == \"ready\"",
        "Parent::Status == \"ready\"",
        "Status[\"k\"] == \"ready\"",
        "Status@any == \"ready\"",
    ] {
        assert!(guard_of(w).is_none(), "{w}");
    }
    let l = lift_source(&when_src("NODE::Status == \"ready\""), &opts()).unwrap();
    assert!(
        l.notes
            .iter()
            .any(|n| n.kind == NoteKind::PreconditionNotStated)
    );
}

#[test]
fn any_vertex_is_only_a_lone_ogit_id() {
    let lone = lift_source("on\n  ogit/_id\nwhen\n  X\ndo\n  action(\"A\")\n", &opts()).unwrap();
    assert_eq!(lone.item.on, OnTarget::AnyVertex);
    let src = "on\n  ogit/_id\n  Region == \"EU\"\nwhen\n  X\ndo\n  action(\"A\")\n";
    let l = lift_source(src, &opts()).unwrap();
    assert_ne!(l.item.on, OnTarget::AnyVertex);
    assert!(!l.notes.iter().any(|n| n.kind == NoteKind::OnAnyVertex));
    let src = "on\n  ogit/_id\n  with T@in\n    A\n  end\nwhen\n  X\ndo\n  action(\"A\")\n";
    assert_ne!(
        lift_source(src, &opts()).unwrap().item.on,
        OnTarget::AnyVertex
    );
}

#[test]
fn variables_named_like_block_words_are_conditions() {
    for w in [
        "count and Ready",
        "count or Ready",
        "count == \"1\"",
        "count",
        "list and Ready",
        "list or Ready",
        "list",
    ] {
        let it = parse(&format!("on\n  {w}\nwhen\n  X\ndo\n  log(\"x\")\n"))
            .unwrap_or_else(|e| panic!("on {w}: {e}"));
        assert_eq!(it.on.blocks().count(), 0, "{w}");
        let it = parse(&format!("on\n  ogit/_id\nwhen\n  {w}\ndo\n  log(\"x\")\n"))
            .unwrap_or_else(|e| panic!("when {w}: {e}"));
        assert_eq!(it.when.conds.len(), 1, "{w}");
    }
    // the block header still is one
    let it = parse("on\n  count T@in as N\n    A\n  end\nwhen\n  X\ndo\n  log(\"x\")\n").unwrap();
    assert_eq!(it.on.blocks().count(), 1);
    // `with` is reserved: refused as a name, not read as a block
    assert!(parse("on\n  with and Ready\nwhen\n  X\ndo\n  log(\"x\")\n").is_err());
}

// A regex argument on its own line inside a call reads like the inline form.
#[test]
fn regex_literal_on_its_own_line_in_a_call() {
    let src = "on\n  ogit/_id\nwhen\n  X\ndo\n  result: LOCAL::RESULT = regex(\n    \"VaLuE: 42.0\",\n    expression: /value ([0-9.]+)/i\n  )\n  result: LOCAL::R2 = regex(\n    /value/i\n  )\n";
    parse(src).expect("a regex literal after a line break inside a call");
    // after an operand, `/` on the next line is still division
    let it = parse(&when_src("Bar == Foo / 2")).expect("parses");
    let ogar_from_ki::ast::Cond::Leaf(l) = &it.when.conds[0] else {
        panic!()
    };
    assert!(matches!(
        l.test.as_ref().unwrap().right,
        Expr::Binary { .. }
    ));
}
