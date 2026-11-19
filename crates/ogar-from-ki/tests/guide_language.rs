//! The language the HIRO User Guides 7.0 document (KI Syntax): conditions,
//! `with` and `count` blocks, the header, scope rows and `do` statements.
//! The snippets are labelled with the guide page they come from, or
//! SYNTHETIC when they are written here to cover a documented form.

mod common;

use common::{called, shape, stmt_kinds};
use ogar_from_ki::ast::{
    ArgValue, BlockKind, CmpOp, Cond, Direction, ExactNeSpelling, Expr, GraphScopeKind, Key,
    KnowledgeItem, LogLevel, Qualifier, ScopeSpelling, Stmt, Tier,
};
use ogar_from_ki::scope::{RefOperator, RefRole, Scope, default_lookup_order};
use ogar_from_ki::{
    ConditionRef, ErrorKind, LiftOptions, NoteKind, OnTarget, ParseError, Section, lift_source,
    parse,
};

fn indent(body: &str) -> String {
    body.lines().map(|l| format!("  {l}\n")).collect::<String>()
}

fn on_item(on: &str) -> Result<KnowledgeItem, ParseError> {
    parse(&format!("on\n{}when\n  X\ndo\n  log(\"x\")\n", indent(on)))
}

fn when_item(when: &str) -> Result<KnowledgeItem, ParseError> {
    parse(&format!(
        "on\n  ogit/_id\nwhen\n{}do\n  log(\"x\")\n",
        indent(when)
    ))
}

fn do_item(do_: &str) -> Result<KnowledgeItem, ParseError> {
    parse(&format!("on\n  ogit/_id\nwhen\n  X\ndo\n{}", indent(do_)))
}

fn when_conds(when: &str) -> Vec<Cond> {
    when_item(when).expect("parses").when.conds
}

fn one_when(when: &str) -> Cond {
    let mut c = when_conds(when);
    assert_eq!(c.len(), 1);
    c.remove(0)
}

fn leaf_of(c: &Cond) -> &ogar_from_ki::ast::Leaf {
    let Cond::Leaf(l) = c else {
        panic!("a leaf, got {c:?}");
    };
    l
}

// ── precedence (matching page, "Not Exists") ─────────────────────────

/// "not Foo or Foo == "hello"" means "Foo does not exist or it does exist and
/// there is a value 'hello' contained within it": `(not Foo) or (Foo ==
/// "hello")`.
#[test]
fn not_binds_tighter_than_or_first_example() {
    let c = one_when("not Foo or Foo == \"hello\"");
    assert_eq!(shape(&c), "or(not(exists),==)");
    let Cond::Or { parts, .. } = &c else {
        panic!("an or");
    };
    assert!(matches!(&parts[0], Cond::Not { inner, .. } if matches!(**inner, Cond::Leaf(_))));
    let l = leaf_of(&parts[1]);
    assert_eq!(l.test.as_ref().unwrap().op, CmpOp::Eq);
}

/// "not Foo or not Foo == "hello"": `(not Foo) or (not (Foo == "hello"))`. The
/// second `not` covers the whole comparison, not just the name.
#[test]
fn not_binds_tighter_than_or_second_example() {
    let c = one_when("not Foo or not Foo == \"hello\"");
    assert_eq!(shape(&c), "or(not(exists),not(==))");
    let Cond::Or { parts, .. } = &c else {
        panic!("an or");
    };
    let Cond::Not { inner, .. } = &parts[1] else {
        panic!("a not");
    };
    assert!(
        leaf_of(inner).test.is_some(),
        "not covers `Foo == \"hello\"`"
    );
}

#[test]
fn not_over_a_comparison_and_over_a_group() {
    // matching page: `not Foo == "hello" # => Match if variable Foo does not
    // contain a value "hello"`
    assert_eq!(shape(&one_when("not Foo == \"hello\"")), "not(==)");
    // SYNTHETIC
    assert_eq!(
        shape(&one_when("not (A == \"1\" or B == \"2\")")),
        "not((or(==,==)))"
    );
    assert_eq!(shape(&one_when("not not A")), "not(not(exists))");
}

/// The page states "not" over "or" and gives no order between "and" and
/// "or". Mixing them without parentheses is refused, not guessed.
#[test]
fn and_mixed_with_or_without_parentheses_is_refused_by_name() {
    let e = when_item("A and B or C").expect_err("refused");
    assert_eq!(
        e.kind,
        ErrorKind::AmbiguousConnectives {
            first: "and".into(),
            second: "or".into()
        }
    );
    assert_eq!((e.pos.line, e.pos.col), (4, 11));
    // With parentheses either reading is explicit.
    assert_eq!(
        shape(&one_when("(A and B) or C")),
        "or((and(exists,exists)),exists)"
    );
    assert_eq!(
        shape(&one_when("A and (B or C)")),
        "and(exists,(or(exists,exists)))"
    );
    // Chains of one connective are one node.
    assert_eq!(shape(&one_when("A or B or C")), "or(exists,exists,exists)");
}

/// matching page, "And": "All conditions are connected by a logical and."
/// One condition per line; the tree keeps them as separate entries.
#[test]
fn lines_are_implicitly_anded_and_kept_separate() {
    let c = when_conds("Foo == 1\nBar == 10");
    assert_eq!(c.len(), 2);
    // matching page, "Or": `Foo == 1 or Foo == 10`
    assert_eq!(shape(&one_when("Foo == 1 or Foo == 10")), "or(==,==)");
}

// ── operators (matching page, table 1) ───────────────────────────────

#[test]
fn every_listed_comparison_operator_is_read() {
    let table: [(&str, CmpOp); 10] = [
        ("==", CmpOp::Eq),
        ("!=", CmpOp::Ne),
        ("===", CmpOp::ExactEq),
        ("!===", CmpOp::ExactNe(ExactNeSpelling::Table)),
        ("!==", CmpOp::ExactNe(ExactNeSpelling::Example)),
        (">=", CmpOp::Ge),
        ("<=", CmpOp::Le),
        (">", CmpOp::Gt),
        ("<", CmpOp::Lt),
        ("=~", CmpOp::Match),
    ];
    for (text, op) in table {
        let c = one_when(&format!("Bar {text} Foo"));
        let t = leaf_of(&c).test.clone().expect("a comparison");
        assert_eq!(t.op, op, "{text}");
        assert_eq!(t.op.text(), text);
        assert!(matches!(t.right, Expr::Var(_)));
    }
    let c = one_when("Foo !~ \"[Ff][oO]*\"");
    assert_eq!(leaf_of(&c).test.as_ref().unwrap().op, CmpOp::NotMatch);
}

/// The guide writes the exact inequality `!===` in its table and `Bar !==
/// Foo` in its example. Both are read; the tree says which one was written.
#[test]
fn exact_inequality_keeps_the_spelling_that_was_written() {
    let table = one_when("Bar !=== Foo");
    let example = one_when("Bar !== Foo");
    let spelling = |c: &Cond| match leaf_of(c).test.as_ref().unwrap().op {
        CmpOp::ExactNe(s) => s,
        other => panic!("exact inequality, got {other:?}"),
    };
    assert_eq!(spelling(&table), ExactNeSpelling::Table);
    assert_eq!(spelling(&example), ExactNeSpelling::Example);
    assert_ne!(spelling(&table), spelling(&example));
    // They are not the same node, so a later stage cannot merge them
    // unknowingly.
    assert_ne!(
        leaf_of(&table).test.as_ref().unwrap().op,
        leaf_of(&example).test.as_ref().unwrap().op
    );
    // The same in `on`.
    let it = on_item("Bar !== Foo").unwrap();
    let c = it.on.conds().next().unwrap();
    assert_eq!(leaf_of(c).test.as_ref().unwrap().op.text(), "!==");
}

#[test]
fn right_sides_may_be_strings_numbers_variables_and_arithmetic() {
    // matching page: `BarN == FooN +1`, `BarT == "Hello, ${FooT}"`
    let c = one_when("BarN == FooN +1");
    let Expr::Binary { op, .. } = &leaf_of(&c).test.as_ref().unwrap().right else {
        panic!("arithmetic");
    };
    assert_eq!(op.text(), "+");
    let c = one_when("BarT == \"Hello, ${FooT}\"");
    assert!(common::is_str(
        &leaf_of(&c).test.as_ref().unwrap().right,
        "Hello, ${FooT}"
    ));
    // varstrings page, "Mathematical operations": `Bar == Foo + 1`
    assert_eq!(shape(&one_when("Bar == Foo + 1")), "==");
    // types page / matching page: a number on the right.
    assert_eq!(shape(&one_when("Foo > 0")), ">");
}

#[test]
fn two_arithmetic_operators_in_one_expression_are_refused() {
    let e = when_item("Bar == Foo + 1 + 2").expect_err("refused");
    assert_eq!(e.kind, ErrorKind::ChainedArithmetic);
}

/// Operators the table does not list are refused by name.
#[test]
fn operators_the_guide_does_not_list_are_refused() {
    for (text, line) in [("<>", 4), ("=>", 4), ("=", 4), ("^=", 4), ("&&", 4)] {
        let e = when_item(&format!("Foo {text} 1")).expect_err(text);
        assert_eq!(
            e.kind,
            ErrorKind::UnsupportedOperator { op: text.into() },
            "{text}"
        );
        assert_eq!(e.pos.line, line);
    }
    // `||` and `|` are only documented inside `has`: refused too.
    assert!(matches!(
        when_item("Foo | Bar").expect_err("refused").kind,
        ErrorKind::UnsupportedOperator { .. }
    ));
}

#[test]
fn regex_literals_keep_pattern_and_flags_and_nothing_is_interpreted() {
    // matching page, "Regex": `Foo =~ /fo*/i`
    let c = one_when("Foo =~ /fo*/i");
    let Expr::Regex(r) = &leaf_of(&c).test.as_ref().unwrap().right else {
        panic!("a regex");
    };
    assert_eq!((r.pattern.as_str(), r.flags.as_str()), ("fo*", "i"));
    // matching page, "Named captures": the group is kept as text.
    let c = one_when("Hostname =~ /^(?<Site>[a-z]+)-(?<Idx>[0-9]+)$/");
    let Expr::Regex(r) = &leaf_of(&c).test.as_ref().unwrap().right else {
        panic!("a regex");
    };
    assert_eq!(r.pattern, "^(?<Site>[a-z]+)-(?<Idx>[0-9]+)$");
    assert_eq!(r.flags, "");
    // `Foo !~ /foo*/i`
    let c = one_when("Foo !~ /foo*/i");
    assert_eq!(leaf_of(&c).test.as_ref().unwrap().op, CmpOp::NotMatch);
    // An escaped slash stays inside the pattern.
    let c = one_when("Foo =~ /a\\/b/");
    let Expr::Regex(r) = &leaf_of(&c).test.as_ref().unwrap().right else {
        panic!("a regex");
    };
    assert_eq!(r.pattern, "a\\/b");
    // A string pattern is a string.
    let c = one_when("Foo =~ \"[Ff][oO]*\"");
    assert!(common::is_str(
        &leaf_of(&c).test.as_ref().unwrap().right,
        "[Ff][oO]*"
    ));
}

#[test]
fn unterminated_regex_is_refused_by_name() {
    let e = when_item("Foo =~ /abc").expect_err("refused");
    assert_eq!(e.kind, ErrorKind::UnterminatedRegex);
    assert_eq!((e.pos.line, e.pos.col), (4, 10));
}

// ── keys, list, alias, scope prefixes, limiters, optional ────────────

#[test]
fn key_access_by_string_number_and_variable() {
    let c = one_when("VarB[\"key\"]");
    assert!(matches!(&leaf_of(&c).subject.key, Some(Key::Str(s)) if s.value == "key"));
    let c = one_when("DiskPos[1] as DP1");
    let l = leaf_of(&c);
    assert!(matches!(&l.subject.key, Some(Key::Number(n)) if n.text == "1"));
    assert_eq!(l.alias.as_ref().unwrap().text, "DP1");
    let c = one_when("VarC[VarA] == NodeType");
    assert!(matches!(&leaf_of(&c).subject.key, Some(Key::Var(v)) if v.text == "VarA"));
    // matching page, "Match value in key": `Foo["key"] == Bar`
    let c = one_when("Foo[\"key\"] == Bar");
    assert_eq!(shape(&c), "==[key]");
    // An empty key is documented ("copy a list with its keys") but not read.
    let e = when_item("Foo[]").expect_err("refused");
    assert!(matches!(e.kind, ErrorKind::DocumentedNotRead { .. }));
}

#[test]
fn list_and_alias_forms() {
    let c = one_when("list VarD");
    assert!(leaf_of(&c).list);
    let c = one_when("VarF as Test");
    assert_eq!(leaf_of(&c).alias.as_ref().unwrap().text, "Test");
    // matching page, "Variable Alias": `Foo == "first" as FooAlias`
    let c = one_when("Foo == \"first\" as FooAlias");
    let l = leaf_of(&c);
    assert!(l.test.is_some());
    assert_eq!(l.alias.as_ref().unwrap().text, "FooAlias");
    // matching page, "And": `A == 3 as A1`
    assert_eq!(shape(&one_when("A == 3 as A1")), "== as");
    // `list Foo as AllFoos`
    assert_eq!(shape(&one_when("list Foo as AllFoos")), "list exists as");
    // A variable named `list` is a variable when nothing follows it.
    assert!(!leaf_of(&one_when("list")).list);
}

#[test]
fn documented_forms_that_are_not_read_are_refused_by_name() {
    // matching page: `list A has "A"`, `Name in [...]`, `count(A) > 3`,
    // `starts_with?(A, "foo")`, `slice(A, 0, 3) =~ ...`
    for (src, want_not_read) in [
        ("list A has \"A\"", true),
        ("Name in [\"Waldo\", \"Wenda\"]", true),
        ("count(A) > 3", false),
        ("starts_with?(A, \"foo\")", false),
        ("slice(A, 0, 3) =~ /^a$/", false),
    ] {
        let e = when_item(src).expect_err(src);
        if want_not_read {
            assert!(
                matches!(e.kind, ErrorKind::DocumentedNotRead { .. }),
                "{src}: {e}"
            );
        } else {
            assert!(
                matches!(e.kind, ErrorKind::CallInExpression { .. }),
                "{src}: {e}"
            );
        }
        assert_eq!(e.pos.line, 4);
    }
}

#[test]
fn scope_prefixes_on_condition_names() {
    // scope page: NODE::, ISSUE::, LOCAL::; matching page: `NODE::Hostname =~
    // ...` inside a `when`.
    let c = one_when("NODE::Hostname =~ /^(?<Site>[a-z]+)-/");
    assert_eq!(leaf_of(&c).subject.name.qualifier(), Some(Qualifier::Node));
    assert_eq!(leaf_of(&c).subject.name.base(), "Hostname");
    let c = one_when("ISSUE::SendMessage");
    assert_eq!(leaf_of(&c).subject.name.qualifier(), Some(Qualifier::Issue));
    let c = one_when("LOCAL::Var3 == \"x\"");
    assert_eq!(leaf_of(&c).subject.name.qualifier(), Some(Qualifier::Local));
    // An alias qualifier (`Parent::Foo`, scope page: `test::A`).
    let c = one_when("Parent::Foo");
    assert_eq!(
        leaf_of(&c).subject.name.qualifier(),
        Some(Qualifier::Alias("Parent".into()))
    );
    // A qualified attribute path.
    let c = one_when("NODE::ogit/status");
    assert_eq!(leaf_of(&c).subject.name.base(), "ogit/status");
    // A malformed qualified name is refused.
    let e = when_item("A::b::c").expect_err("refused");
    assert!(matches!(e.kind, ErrorKind::UnknownNamespace { .. }));
}

#[test]
fn graph_scope_limiters_any_and_self() {
    // matching page: `not Foo@any`, `Foo@any`, `Foo@self`
    let c = one_when("Foo@any");
    assert_eq!(leaf_of(&c).subject.graph.unwrap().kind, GraphScopeKind::Any);
    let c = one_when("Foo@self");
    assert_eq!(
        leaf_of(&c).subject.graph.unwrap().kind,
        GraphScopeKind::SelfOnly
    );
    let c = one_when("not Foo@any");
    assert_eq!(shape(&c), "not(exists@any)");
    // ki-structure page: `MonitoringStatus@any == "true"`; guide's full
    // example: `MonitoringStatus@in == "true"` (kept apart, not read as @any).
    assert_eq!(
        shape(&one_when("MonitoringStatus@any == \"true\"")),
        "==@any"
    );
    assert_eq!(shape(&one_when("MonitoringStatus@in == \"true\"")), "==@in");
    // A key and a limiter together: `delete(FooVar["key"]@any)` shape.
    assert_eq!(shape(&one_when("Foo[\"key\"]@any")), "exists[key]@any");
    // An unknown limiter is refused.
    let e = when_item("Foo@everywhere").expect_err("refused");
    assert_eq!(
        e.kind,
        ErrorKind::UnknownGraphScope {
            found: "everywhere".into()
        }
    );
}

#[test]
fn optional_matches() {
    // matching page, "Optional Match"
    let c = one_when("? Count := 0");
    let l = leaf_of(&c);
    assert!(l.optional && l.default.is_some() && l.test.is_none());
    let c = one_when("? Count < 5 := 0");
    let l = leaf_of(&c);
    assert!(l.optional && l.default.is_some());
    assert_eq!(l.test.as_ref().unwrap().op, CmpOp::Lt);
    assert_eq!(shape(&c), "?< :=");
    // `?list Values`: no default needed.
    let c = one_when("?list Values");
    let l = leaf_of(&c);
    assert!(l.optional && l.list && l.default.is_none());
    // `? ogit/Automation/maxIdleTime@any := 6`
    assert_eq!(
        shape(&one_when("? ogit/Automation/maxIdleTime@any := 6")),
        "?exists@any :="
    );
    // An optional match with no default (and no `list`) is refused.
    let e = when_item("? Count").expect_err("refused");
    assert_eq!(e.kind, ErrorKind::OptionalWithoutDefault);
    assert_eq!((e.pos.line, e.pos.col), (4, 3));
}

#[test]
fn every_condition_node_records_its_position() {
    let it = when_item("not Foo or Foo == \"hello\"").unwrap();
    let c = &it.when.conds[0];
    assert_eq!(c.span().start.line, 4);
    assert_eq!(c.span().start.col, 3);
    let Cond::Or { parts, .. } = c else {
        panic!("an or");
    };
    assert_eq!(parts[0].span().start.col, 3);
    assert_eq!(parts[1].span().start.col, 14);
    assert_eq!(parts[1].span().end.col, 14 + "Foo == \"hello\"".len());
}

#[test]
fn comments_are_dropped_and_do_not_end_a_condition() {
    // ki-structure page: `NodeType    # exists`
    let it = on_item("NodeType    # exists\nnot NodeFoo # not exists").unwrap();
    assert_eq!(it.on.conds().count(), 2);
    // A `#` inside a string is part of the string.
    let c = one_when("Foo == \"a # b\"");
    assert!(common::is_str(
        &leaf_of(&c).test.as_ref().unwrap().right,
        "a # b"
    ));
}

// ── `with` and `count` blocks (ki-structure page, matching page) ─────

fn only_block(it: &KnowledgeItem) -> &ogar_from_ki::ast::RelatedBlock {
    let mut b = it.on.blocks();
    let first = b.next().expect("a block");
    assert!(b.next().is_none());
    first
}

#[test]
fn each_documented_direction() {
    for (word, dir) in [
        ("in", Direction::In),
        ("out", Direction::Out),
        ("connected", Direction::Connected),
    ] {
        let it = on_item(&format!("with Foo@{word}\n  Bar\nend")).unwrap();
        let b = only_block(&it);
        assert_eq!(b.direction, dir);
        assert!(b.direction.is_documented());
        assert_eq!(b.direction.text(), word);
        assert_eq!(b.kind, BlockKind::With);
    }
}

/// matching page: `with ogit/Automation/MARSNode@parent:ogit/relates`; scope
/// page: `with test@child:owns as test`. Neither direction is in the
/// structure page's list; both are recorded as their own variants.
#[test]
fn parent_and_child_are_recorded_as_used_in_the_guide_but_not_documented() {
    let it = on_item("ogit/_xid == \"Node2\"\nwith ogit/Automation/MARSNode@parent:ogit/relates\n  ogit/_xid == \"Node1\"\nend").unwrap();
    let b = only_block(&it);
    assert_eq!(b.direction, Direction::ExampleOnlyParent);
    assert!(!b.direction.is_documented());
    assert_eq!(b.edge_type.as_ref().unwrap().text, "ogit/relates");
    // scope page: with test@child:owns as test / A / B == A / end
    let it = on_item("A\nwith test@child:owns as test\n  A\n  B == A\nend\nB == A").unwrap();
    let b = only_block(&it);
    assert_eq!(b.direction, Direction::ExampleOnlyChild);
    assert_eq!(b.direction, Direction::ExampleOnlyChild);
    assert_ne!(b.direction, Direction::In);
    assert_eq!(b.vertex_type.text, "test");
    assert_eq!(b.edge_type.as_ref().unwrap().text, "owns");
    assert_eq!(b.alias.as_ref().unwrap().text, "test");
    assert_eq!(b.conds.len(), 2);
    // The block sits between two top-level conditions.
    assert_eq!(it.on.conds().count(), 2);
    assert_eq!(it.on.items.len(), 3);
}

#[test]
fn unknown_direction_is_refused_by_name() {
    let e = on_item("with Foo@sideways\n  Bar\nend").expect_err("refused");
    assert_eq!(
        e.kind,
        ErrorKind::UnknownDirection {
            found: "sideways".into()
        }
    );
    assert_eq!((e.pos.line, e.pos.col), (2, 12));
    // Capital letters are not the documented spelling.
    let e = on_item("with Foo@In\n  Bar\nend").expect_err("refused");
    assert!(matches!(e.kind, ErrorKind::UnknownDirection { .. }));
    // A missing direction.
    let e = on_item("with Foo\n  Bar\nend").expect_err("refused");
    assert!(matches!(e.kind, ErrorKind::UnexpectedToken { .. }));
}

#[test]
fn edge_type_is_optional_and_may_be_a_path() {
    // matching page: `with FooType@in` (no edge type = any edge type)
    let it = on_item("Foo\nwith FooType@in\n  Foo == \"has childs\"\nend").unwrap();
    let b = only_block(&it);
    assert!(b.edge_type.is_none());
    assert!(b.alias.is_none());
    assert_eq!(b.vertex_type.text, "FooType");
    // ki-structure page, "Related Node Matching Syntax Example"
    let it = on_item(
        "ogit/_type == \"ogit/Person\"\nwith ogit/Software/Application@out:ogit/uses\n  ogit/name == \"LunchApp\"\n  ogit/url\nend",
    )
    .unwrap();
    let b = only_block(&it);
    assert_eq!(b.vertex_type.text, "ogit/Software/Application");
    assert_eq!(b.direction, Direction::Out);
    assert_eq!(b.edge_type.as_ref().unwrap().text, "ogit/uses");
    let inner: Vec<String> = b.conds.iter().map(shape).collect();
    assert_eq!(inner, vec!["==".to_string(), "exists".to_string()]);
    // ... and the lift still finds the class from the top-level condition.
    let l = lift_source(
        &format!(
            "on\n{}when\n  X\ndo\n  log(\"x\")\n",
            indent("ogit/_type == \"ogit/Person\"\nwith ogit/Software/Application@out:ogit/uses\n  ogit/url\nend")
        ),
        &LiftOptions {
            item_id: Some("p".into()),
        },
    )
    .unwrap();
    assert_eq!(l.item.on, OnTarget::Class("ogit/Person".into()));
}

#[test]
fn with_alias_is_optional_and_connected_has_no_edge_requirement() {
    let it = on_item("with FooType@in as Parent\n  Foo\nend").unwrap();
    let b = only_block(&it);
    assert_eq!(b.alias.as_ref().unwrap().text, "Parent");
    let it = on_item("with FooType@connected:ogit/x\n  Foo\nend").unwrap();
    assert_eq!(only_block(&it).direction, Direction::Connected);
}

#[test]
fn count_blocks() {
    // matching page, "Related count"
    let it =
        on_item("Foo\ncount FooType@in as ParentCount\n  Bar == \"test\"\nend\nBar > ParentCount")
            .unwrap();
    let b = only_block(&it);
    assert_eq!(b.kind, BlockKind::Count);
    assert_eq!(b.alias.as_ref().unwrap().text, "ParentCount");
    assert_eq!(b.conds.len(), 1);
    assert_eq!(it.on.conds().count(), 2);
    // matching page: the second example
    let it = on_item("count ogit/ConfigurationItem@out:ogit/_created as CreatedConfigurationCount\n  ogit/_id\nend").unwrap();
    let b = only_block(&it);
    assert_eq!(b.edge_type.as_ref().unwrap().text, "ogit/_created");
    // values page: `count ogit/Automation/MARSNode@in:mars as Count`
    let it = on_item("count ogit/Automation/MARSNode@in:mars as Count\n  Foo\nend").unwrap();
    assert_eq!(only_block(&it).edge_type.as_ref().unwrap().text, "mars");
    // `count` without a name to hold the number is refused.
    let e = on_item("count FooType@in\n  Foo\nend").expect_err("refused");
    assert_eq!(e.kind, ErrorKind::CountWithoutAlias);
    // `count` stays a plain variable elsewhere (matching page: `? count := 0`).
    assert!(leaf_of(&one_when("? count := 0")).optional);
}

#[test]
fn unterminated_with_is_refused_by_name_at_the_with() {
    let e = on_item("with Foo@in\n  Bar").expect_err("refused");
    assert_eq!(e.kind, ErrorKind::UnterminatedBlock { word: "with" });
    assert_eq!((e.pos.line, e.pos.col), (2, 3));
    // A clause keyword ends the search for `end`.
    let e =
        parse("on\n  with Foo@in\n    Bar\nwhen\n  X\ndo\n  log(\"x\")\n").expect_err("refused");
    assert_eq!(e.kind, ErrorKind::UnterminatedBlock { word: "with" });
    assert_eq!(e.pos.line, 2);
    // Same for `count`.
    let e = on_item("count Foo@in as N\n  Bar").expect_err("refused");
    assert_eq!(e.kind, ErrorKind::UnterminatedBlock { word: "count" });
}

/// The guide shows no nesting of `with` blocks (the digest: "Nested `with`
/// blocks: NOT FOUND"). Refused by name.
#[test]
fn nested_with_is_refused_by_name() {
    let e = on_item("with A@in\n  with B@out\n    C\n  end\nend").expect_err("refused");
    assert_eq!(
        e.kind,
        ErrorKind::NestedBlock {
            outer: "with",
            inner: "with"
        }
    );
    assert_eq!((e.pos.line, e.pos.col), (3, 5));
    let e = on_item("with A@in\n  count B@out as N\n    C\n  end\nend").expect_err("refused");
    assert_eq!(
        e.kind,
        ErrorKind::NestedBlock {
            outer: "with",
            inner: "count"
        }
    );
}

#[test]
fn empty_with_and_with_in_when_are_refused() {
    let e = on_item("with A@in\nend").expect_err("refused");
    assert_eq!(e.kind, ErrorKind::EmptyClause { clause: "with" });
    let e = when_item("with A@in\n  B\nend").expect_err("refused");
    assert_eq!(e.kind, ErrorKind::BlockOutsideOn { word: "with" });
    assert_eq!((e.pos.line, e.pos.col), (4, 3));
    // `with` stays a reserved word: it cannot be a variable.
    assert!(matches!(
        when_item("with").expect_err("refused").kind,
        ErrorKind::ReservedWord { .. } | ErrorKind::UnexpectedToken { .. }
    ));
}

#[test]
fn the_multi_variable_alias_block_is_documented_but_not_read() {
    // matching page, "Multi variable Alias"
    let e = on_item("alias as FooBar\n  Foo\n  Bar\nend").expect_err("refused");
    assert!(matches!(e.kind, ErrorKind::DocumentedNotRead { .. }), "{e}");
}

// ── header: version and tier ─────────────────────────────────────────

fn header_src(extra: &str) -> String {
    format!("ki\n  name: \"K\"\n{extra}on\n  ogit/_id\nwhen\n  X\ndo\n  log(\"x\")\n")
}

#[test]
fn version_and_each_tier() {
    for (text, tier) in [
        ("default", Tier::Default),
        ("policy", Tier::Policy),
        ("fallback", Tier::Fallback),
    ] {
        let it = parse(&header_src(&format!(
            "  version: \"1@Author\"\n  tier: \"{text}\"\n"
        )))
        .unwrap();
        let h = it.header.unwrap();
        assert_eq!(h.tier, Some(tier));
        assert_eq!(tier.text(), text);
        assert_eq!(h.version.unwrap().value, "1@Author");
    }
    // No tier written: none recorded (the page says it defaults to "default";
    // the tree does not write that in).
    let it = parse(&header_src("")).unwrap();
    assert_eq!(it.header.unwrap().tier, None);
}

#[test]
fn unknown_tier_is_refused_by_name_at_the_value() {
    let e = parse(&header_src("  tier: \"urgent\"\n")).expect_err("refused");
    assert_eq!(
        e.kind,
        ErrorKind::UnknownTier {
            value: "urgent".into()
        }
    );
    assert_eq!((e.pos.line, e.pos.col), (3, 9));
    // The structure page prose says "Default policy" in capitals; only the
    // lowercase values of the attribute list are tiers.
    let e = parse(&header_src("  tier: \"Default\"\n")).expect_err("refused");
    assert!(matches!(e.kind, ErrorKind::UnknownTier { .. }));
}

#[test]
fn header_keys_are_each_once_and_known() {
    let e = parse(&header_src("  version: \"1\"\n  version: \"2\"\n")).expect_err("refused");
    assert_eq!(
        e.kind,
        ErrorKind::DuplicateHeaderKey {
            key: "version".into()
        }
    );
    let e = parse(&header_src("  author: \"x\"\n")).expect_err("refused");
    assert_eq!(
        e.kind,
        ErrorKind::UnknownHeaderKey {
            key: "author".into()
        }
    );
    // The lift carries the typed tier and the version.
    let l = lift_source(
        &header_src("  version: \"3@Me\"\n  tier: \"policy\"\n"),
        &LiftOptions::default(),
    )
    .unwrap();
    assert_eq!(l.item.tier, Some(Tier::Policy));
    assert_eq!(l.item.version.as_deref(), Some("3@Me"));
}

#[test]
fn unknown_section_is_refused_by_name() {
    let e = parse("on\n  ogit/_id\nwhere\n  X\ndo\n  log(\"x\")\n").expect_err("refused");
    assert_eq!(
        e.kind,
        ErrorKind::UnknownClauseKeyword {
            word: "where".into()
        }
    );
    assert_eq!(e.pos.line, 3);
    let e = parse("ki\n  name: \"K\"\nthen\n  X\n").expect_err("refused");
    assert!(matches!(e.kind, ErrorKind::UnknownClauseKeyword { .. }));
}

// ── scope rows (scope page) ──────────────────────────────────────────

fn rows(src: &str) -> Vec<ConditionRef> {
    parse(src).expect("parses").condition_refs()
}

/// scope page: `on` is the NODE scope, `when` the ISSUE scope; an unscoped
/// name is looked up LOCAL, then ISSUE, then NODE.
#[test]
fn rows_name_the_section_the_prefix_and_the_documented_lookup_order() {
    let r =
        rows("on\n  Var1\nwhen\n  Var2\n  ISSUE::Var3 == \"x\"\n  LOCAL::Var4\ndo\n  log(\"x\")\n");
    assert_eq!(r.len(), 4);
    // on: Var1
    assert_eq!(r[0].section, Section::On);
    assert_eq!(r[0].section_scope, Scope::Node);
    assert_eq!(r[0].written, None);
    assert_eq!(r[0].name, "Var1");
    assert_eq!(r[0].operator, RefOperator::Exists);
    assert_eq!(r[0].role, RefRole::Subject);
    assert_eq!(r[0].lookup, default_lookup_order());
    assert_eq!(
        r[0].lookup,
        vec![Qualifier::Local, Qualifier::Issue, Qualifier::Node]
    );
    assert_eq!((r[0].pos.line, r[0].pos.col), (2, 3));
    assert!(!r[0].outside_permitted_scopes);
    // when: Var2 (unscoped), ISSUE::Var3, LOCAL::Var4
    assert_eq!(r[1].section, Section::When);
    assert_eq!(r[1].section_scope, Scope::Issue);
    assert_eq!(r[1].written, None);
    assert_eq!(r[1].lookup, default_lookup_order());
    assert_eq!(r[2].written, Some(Qualifier::Issue));
    assert_eq!(r[2].name, "Var3");
    assert_eq!(r[2].name_as_written, "ISSUE::Var3");
    assert_eq!(r[2].lookup, vec![Qualifier::Issue]);
    assert_eq!(r[2].operator, RefOperator::Compare(CmpOp::Eq));
    assert!(!r[2].outside_permitted_scopes);
    assert_eq!(r[3].written, Some(Qualifier::Local));
    assert_eq!(r[3].lookup, vec![Qualifier::Local]);
}

/// The scope page lists the permitted scopes (`on`: NODE and LOCAL, `when`:
/// ISSUE and LOCAL), and the matching page writes `NODE::Hostname` in a
/// `when`. The row says so; the item is not refused and nothing is enforced.
#[test]
fn a_prefix_outside_the_permitted_list_is_flagged_not_refused() {
    let r = rows("on\n  A\nwhen\n  NODE::Hostname =~ /^(?<Site>[a-z]+)-/\ndo\n  log(\"x\")\n");
    assert_eq!(r[1].section, Section::When);
    assert_eq!(r[1].written, Some(Qualifier::Node));
    assert!(r[1].outside_permitted_scopes);
    assert_eq!(r[1].lookup, vec![Qualifier::Node]);
    let r = rows("on\n  ISSUE::A\nwhen\n  B\ndo\n  log(\"x\")\n");
    assert!(r[0].outside_permitted_scopes);
    // The ISSUE name in `on` and the NODE name in `when` are the flagged
    // cases; the permitted combinations are not.
    let r = rows("on\n  NODE::A\n  LOCAL::C\nwhen\n  ISSUE::B\ndo\n  log(\"x\")\n");
    assert!(r.iter().all(|x| !x.outside_permitted_scopes));
}

#[test]
fn a_title_case_node_prefix_keeps_its_spelling() {
    // scope page: `Node::ogit/status = "visited"` (documented prefix: NODE::)
    let r = rows("on\n  Node::ogit/status\nwhen\n  X\ndo\n  log(\"x\")\n");
    assert_eq!(r[0].written, Some(Qualifier::Node));
    assert_eq!(r[0].scope_spelling, Some(ScopeSpelling::NodeTitleCase));
    let r = rows("on\n  NODE::ogit/status\nwhen\n  X\ndo\n  log(\"x\")\n");
    assert_eq!(r[0].scope_spelling, Some(ScopeSpelling::Documented));
    // No prefix, no spelling.
    let r = rows("on\n  ogit/status\nwhen\n  X\ndo\n  log(\"x\")\n");
    assert_eq!(r[0].scope_spelling, None);
}

#[test]
fn rows_cover_operands_keys_negation_and_blocks() {
    let r = rows(
        "on\n  Foo\n  not Bar\n  with FooType@in as Parent\n    Parent::Foo == Baz\n  end\nwhen\n  VarC[VarA] == NodeType\n  list L\ndo\n  log(\"x\")\n",
    );
    let names: Vec<(&str, RefRole)> = r.iter().map(|x| (x.name.as_str(), x.role)).collect();
    assert_eq!(
        names,
        vec![
            ("Foo", RefRole::Subject),
            ("Bar", RefRole::Subject),
            ("Foo", RefRole::Subject),
            ("Baz", RefRole::RightOperand),
            ("VarC", RefRole::Subject),
            ("VarA", RefRole::Key),
            ("NodeType", RefRole::RightOperand),
            ("L", RefRole::Subject),
        ]
    );
    assert!(!r[0].negated);
    assert!(r[1].negated, "under `not`");
    // The block rows carry the block they sit in.
    assert!(r[0].block.is_none());
    let b = r[2].block.as_ref().expect("in a with block");
    assert_eq!(b.kind, BlockKind::With);
    assert_eq!(b.vertex_type, "FooType");
    assert_eq!(b.alias.as_deref(), Some("Parent"));
    assert_eq!(r[2].written, Some(Qualifier::Alias("Parent".into())));
    assert_eq!(r[2].lookup, vec![Qualifier::Alias("Parent".into())]);
    // `Baz` is on the right of `==`: the operator is the comparison.
    assert_eq!(r[3].operator, RefOperator::Compare(CmpOp::Eq));
    // `list L`.
    assert!(r[7].list);
    assert_eq!(r[7].section, Section::When);
}

#[test]
fn rows_record_limiters_and_optional() {
    let r = rows("on\n  A\nwhen\n  ? Foo@any := 6\n  not Bar@self\ndo\n  log(\"x\")\n");
    assert_eq!(r[1].graph, Some(GraphScopeKind::Any));
    assert!(r[1].optional);
    assert_eq!(r[2].graph, Some(GraphScopeKind::SelfOnly));
    assert!(r[2].negated);
}

// ── do statements ────────────────────────────────────────────────────

fn only_stmt(do_: &str) -> Stmt {
    let mut it = do_item(do_).unwrap_or_else(|e| panic!("{do_}: {e}"));
    assert_eq!(it.do_.stmts.len(), 1, "{do_}");
    it.do_.stmts.remove(0)
}

#[test]
fn assignments_with_scope_prefixes_keys_and_lists() {
    // values page: `VarA = "foobar"`, `VarA = ["A", "B", "C"]`, scope page:
    // `LOCAL::Message = "..."`, `Node::ogit/status = "visited"`
    let Stmt::Assign { target, value, .. } = only_stmt("LOCAL::Message = \"${A} on ${B}\"") else {
        panic!("an assignment");
    };
    assert_eq!(target.name.qualifier(), Some(Qualifier::Local));
    assert!(common::is_str(&value, "${A} on ${B}"));
    let Stmt::Assign { value, .. } = only_stmt("VarA = [\"A\", \"B\", \"C\"]") else {
        panic!("an assignment");
    };
    let Expr::List { items, .. } = value else {
        panic!("a list");
    };
    assert_eq!(items.len(), 3);
    let Stmt::Assign { target, .. } = only_stmt("Node::ogit/status = \"visited\"") else {
        panic!("an assignment");
    };
    assert_eq!(
        target.name.scope_spelling(),
        Some(ScopeSpelling::NodeTitleCase)
    );
    // values page: `VarA@any = "new value"`, `VarA["key"] = "new value"`
    let Stmt::Assign { target, .. } = only_stmt("VarA@any = \"new value\"") else {
        panic!("an assignment");
    };
    assert_eq!(target.graph.unwrap().kind, GraphScopeKind::Any);
    let Stmt::Assign { target, .. } = only_stmt("VarA[\"key\"] = \"new value\"") else {
        panic!("an assignment");
    };
    assert!(target.key.is_some());
    // values page: `VarB = VarA`
    assert!(matches!(
        only_stmt("VarB = VarA"),
        Stmt::Assign {
            value: Expr::Var(_),
            ..
        }
    ));
    // varstrings page: `LOCAL::result = number1 / number2`
    let Stmt::Assign { value, .. } = only_stmt("LOCAL::result = number1 / number2") else {
        panic!("an assignment");
    };
    assert!(matches!(value, Expr::Binary { .. }));
    // `VarB[] = VarA` (copy with keys) is documented but not read.
    let e = do_item("VarB[] = VarA").expect_err("refused");
    assert!(matches!(e.kind, ErrorKind::DocumentedNotRead { .. }));
}

#[test]
fn set_add_and_delete() {
    // values page
    assert!(matches!(only_stmt("set(FooVar=\"Bar\")"), Stmt::Set { .. }));
    let Stmt::Set { target, .. } = only_stmt("set(FooVar@any = \"Bar\")") else {
        panic!("a set");
    };
    assert_eq!(target.graph.unwrap().kind, GraphScopeKind::Any);
    let Stmt::Add { target, value, .. } = only_stmt("add(VarA[\"foo\"] = \"bar\")") else {
        panic!("an add");
    };
    assert!(matches!(target.key, Some(Key::Str(_))));
    assert!(common::is_str(&value, "bar"));
    assert!(matches!(
        only_stmt("add(VarA = \"foobar\")"),
        Stmt::Add { .. }
    ));
    // `add(MachineList = ogit/name)`: the query page's gremlin example.
    assert!(matches!(
        only_stmt("add(MachineList = ogit/name)"),
        Stmt::Add {
            value: Expr::Var(_),
            ..
        }
    ));
    let Stmt::Delete {
        target, created_on, ..
    } = only_stmt("delete(FooVar@any, created_on: SomeOtherNodeID)")
    else {
        panic!("a delete");
    };
    assert_eq!(target.name.text, "FooVar");
    assert!(created_on.is_some());
    let Stmt::Delete { target, .. } = only_stmt("delete(FooVar[\"key\"]@any)") else {
        panic!("a delete");
    };
    assert!(target.key.is_some() && target.graph.is_some());
    // `set` without an assignment, and an unknown keyword argument, are refused.
    assert!(matches!(
        do_item("set(FooVar)").expect_err("refused").kind,
        ErrorKind::UnexpectedToken { .. }
    ));
    assert!(matches!(
        do_item("delete(FooVar, since: X)")
            .expect_err("refused")
            .kind,
        ErrorKind::UnexpectedToken { .. }
    ));
}

#[test]
fn log_levels() {
    // log page, table 1
    for (call, level) in [
        ("log(\"m\")", LogLevel::Default),
        ("log.debug(\"m\")", LogLevel::Debug),
        ("log.info(\"m\")", LogLevel::Info),
        ("log.warn(\"m\")", LogLevel::Warn),
        ("log.error(\"m\")", LogLevel::Error),
    ] {
        let Stmt::Log { level: got, .. } = only_stmt(call) else {
            panic!("a log");
        };
        assert_eq!(got, level, "{call}");
    }
    // log page: `log(totalSum)`: a variable as the message.
    assert!(matches!(
        only_stmt("log(totalSum)"),
        Stmt::Log {
            message: Expr::Var(_),
            ..
        }
    ));
    let e = do_item("log.fatal(\"m\")").expect_err("refused");
    assert_eq!(
        e.kind,
        ErrorKind::UnknownLogLevel {
            level: "fatal".into()
        }
    );
    // log page, last example: `log\tinfo("default")` (a tab where a dot
    // belongs) is a defect of the page and is refused, not read as `log.info`.
    let e = do_item("log\tinfo(\"default\")").expect_err("refused");
    assert!(matches!(e.kind, ErrorKind::UnexpectedToken { .. }), "{e}");
    // A `.` after another name is refused.
    assert!(do_item("send.mail(\"x\")").is_err());
}

#[test]
fn tagged_results_and_generic_calls() {
    // functions page: `result: LOCAL::RESULT = regex("...", expression: "...")`
    let Stmt::Call(c) = only_stmt(
        "result: LOCAL::RESULT = regex(\"Here is a hidden value 47.11 inside\", expression: \".*value ([0-9.]+) .*\")",
    ) else {
        panic!("a call");
    };
    assert_eq!(c.function.text, "regex");
    assert_eq!(c.bindings.len(), 1);
    assert_eq!(c.args.len(), 2);
    assert!(c.args[0].name.is_none());
    assert_eq!(c.args[1].name.as_ref().unwrap().text, "expression");
    // keyword arguments in any order
    let Stmt::Call(c) = only_stmt(
        "result: LOCAL::RESULT = regex(expression: \".*value ([0-9.]+) .*\", string:\"Here\")",
    ) else {
        panic!("a call");
    };
    let kw: Vec<&str> = c
        .args
        .iter()
        .filter_map(|a| a.name.as_ref())
        .map(|n| n.text.as_str())
        .collect();
    assert_eq!(kw, vec!["expression", "string"]);
    // a call with no tags and no arguments of any kind recorded
    let Stmt::Call(c) = only_stmt("sleep(5)") else {
        panic!("a call");
    };
    assert_eq!(c.function.text, "sleep");
    assert!(c.bindings.is_empty());
    assert!(matches!(&c.args[0].value, ArgValue::Expr(Expr::Number(n)) if n.text == "5"));
    // any name is kept as a generic call; no function is given a meaning here
    for f in [
        "eject_issue",
        "create_vertex",
        "read_timeseries",
        "anything_at_all",
    ] {
        let Stmt::Call(c) = only_stmt(&format!("{f}(X)")) else {
            panic!("a call");
        };
        assert_eq!(c.function.text, f);
    }
    // tags in front of a call that returns nothing are refused
    let e = do_item("exit: LOCAL::X = log(\"m\")").expect_err("refused");
    assert_eq!(
        e.kind,
        ErrorKind::BindingsOnCall {
            function: "log".into()
        }
    );
    // a positional argument after a keyword is refused
    let e = do_item("execute(timeout: 60, \"cmd\")").expect_err("refused");
    assert_eq!(e.kind, ErrorKind::PositionalAfterKeyword);
    // a call inside an argument or as a value is refused
    assert!(matches!(
        do_item("X = foo(1)").expect_err("refused").kind,
        ErrorKind::CallInExpression { .. }
    ));
    assert!(matches!(
        do_item("execute(foo(1))").expect_err("refused").kind,
        ErrorKind::NestedCall { .. }
    ));
}

/// query page: `each: begin ... end`; read_timeseries page: the same on one
/// line, `each: begin SUM = SUM + value end`.
#[test]
fn begin_blocks_as_arguments() {
    let Stmt::Call(c) =
        only_stmt("exit: ExitCode = read_timeseries(TS_ID, each: begin SUM = SUM + value end)")
    else {
        panic!("a call");
    };
    let ArgValue::Block(b) = &c.args[1].value else {
        panic!("a block");
    };
    assert_eq!(stmt_kinds(&b.stmts), vec!["assign".to_string()]);
    let it = do_item("output: O = query(\"vertices\", \"q\",\n  each: begin\n    add(L = ogit/name)\n    log(L)\n  end\n)").unwrap();
    assert_eq!(called(&it.do_.stmts), vec!["query", "add", "log"]);
    // an unterminated `begin` is refused at the `begin`
    let e = do_item("output: O = query(\"vertices\", \"q\", each: begin\n  log(L)\n")
        .expect_err("refused");
    assert_eq!(e.kind, ErrorKind::UnterminatedBlock { word: "begin" });
    // `begin` is not a statement of its own
    assert!(do_item("begin").is_err());
}

#[test]
fn if_conditions_use_the_same_tree() {
    // if page: `if LOCAL::CODE == "0" and LOCAL::Status == "SUCCESS" then`
    let Stmt::If(i) = only_stmt(
        "if LOCAL::CODE == \"0\" and LOCAL::Status == \"SUCCESS\" then\n  delete(GetPersonData)\nend",
    ) else {
        panic!("an if");
    };
    assert_eq!(shape(&i.cond), "and(==,==)");
    assert!(i.else_branch.is_none());
    // `or`, `not`, numbers, comparison operators
    let Stmt::If(i) = only_stmt("if not A or B > 3 then\n  log(\"a\")\nelse\n  log(\"b\")\nend")
    else {
        panic!("an if");
    };
    assert_eq!(shape(&i.cond), "or(not(exists),>)");
    assert!(i.else_branch.is_some());
    // forms that belong to a match line are refused inside an `if`
    let e = do_item("if list A then\n  log(\"a\")\nend").expect_err("refused");
    assert_eq!(e.kind, ErrorKind::MatchFormInIf { form: "list" });
    let e = do_item("if A as B then\n  log(\"a\")\nend").expect_err("refused");
    assert_eq!(e.kind, ErrorKind::MatchFormInIf { form: "as" });
    // The guide shows no `if` inside an `if`.
    let e = do_item("if A then\n  if B then\n    log(\"a\")\n  end\nend").expect_err("refused");
    assert_eq!(e.kind, ErrorKind::NestedIf);
}

#[test]
fn lift_carries_tags_other_calls_and_writes_of_the_new_statements() {
    let src = "on\n  ogit/_id\nwhen\n  X\ndo\n  VarA[\"k\"] = \"v\"\n  set(S = \"1\")\n  add(Ad = \"2\")\n  delete(De)\n  result: date = dateprint(format: \"%d\")\n  stdout: Out = execute(\"ls\")\n";
    let l = lift_source(
        src,
        &LiftOptions {
            item_id: Some("w".into()),
        },
    )
    .unwrap();
    assert_eq!(l.item.writes, vec!["VarA", "S", "Ad", "De", "date", "Out"]);
    let fns: Vec<&str> = l
        .item
        .other_calls
        .iter()
        .map(|c| c.function.as_str())
        .collect();
    assert_eq!(fns, vec!["dateprint", "execute"]);
    assert_eq!(
        l.item.other_calls[0].kind,
        ogar_from_ki::OtherCallKind::Generic
    );
    assert_eq!(
        l.item.other_calls[1].kind,
        ogar_from_ki::OtherCallKind::Execute
    );
    assert!(l.defs.is_empty(), "only action(...) lifts to a def");
    assert!(l.notes.iter().any(|n| n.kind == NoteKind::ExecuteNotLifted));
}

// ── never panics on the guide's text either ──────────────────────────

#[test]
fn every_truncation_of_the_guide_fixtures_is_ok_or_a_named_error() {
    for name in [
        "full_example",
        "querying_graph",
        "checking_spelling",
        "rl_hn3_r1",
    ] {
        let text = common::guide(name);
        let chars: Vec<char> = text.chars().collect();
        for n in 0..=chars.len() {
            let cut: String = chars[..n].iter().collect();
            match parse(&cut) {
                Ok(it) => {
                    let _ = ogar_from_ki::outline(&it);
                    let _ = it.condition_refs();
                }
                Err(e) => {
                    assert!(e.pos.line >= 1 && e.pos.col >= 1, "{name} {n}: {e:?}");
                    assert!(!e.expected.is_empty());
                }
            }
        }
    }
}

// ── more of the guide's own text ─────────────────────────────────────

/// `functions/if` page, "Usage": a call whose arguments run over several
/// lines, then an `if` over its exit tag.
#[test]
fn if_page_usage_example_parses() {
    let it = do_item(
        "stdout: ExecuteOutput,\nexit: Error = execute(\n\"/some/command/path -with arguments\",\nhost: \"_AAE_\"\n)\n\nif Error then\n    log.error(\"command exited with non-zero exit code\")\nelse\n    log(\"execute returned ${ExecuteOutput}\")\nend",
    )
    .expect("parses");
    assert_eq!(called(&it.do_.stmts), vec!["execute", "log.error", "log"]);
}

/// `functions/query` page, "Example Gremlin query": there is no comma between
/// `root: known_vertex_id` and `fields:`. The page is defective; the call is
/// refused at the `fields` token (line 9 here, after the five header lines
/// and the two-space indent), not repaired.
#[test]
fn query_page_gremlin_example_with_a_missing_comma_is_refused() {
    let call = "exit: LOCAL::EXIT, output: LOCAL::IDs = query('gremlin',\n    'out(\"ogit/dependsOn\")',\n    root: known_vertex_id\n    fields:  \"ogit/_xid, ogit/_id, ogit/name\",\n    each: begin\n        add(MachineList = ogit/name)\n    end\n)";
    let e = do_item(call).expect_err("refused");
    assert!(matches!(e.kind, ErrorKind::UnexpectedToken { .. }), "{e}");
    assert_eq!((e.pos.line, e.pos.col), (9, 7));
    // With the comma the same call parses.
    let fixed = call.replace("known_vertex_id\n", "known_vertex_id,\n");
    let it = do_item(&fixed).expect("parses");
    assert_eq!(called(&it.do_.stmts), vec!["query", "add"]);
}

/// `definitions/values` page: the copy examples and the `count` on a list in
/// `on`.
#[test]
fn values_page_examples_parse() {
    parse("on\n  list VarA\nwhen\n  ProcessIssue\ndo\n  VarB = VarA # will copy list in VarB\n")
        .expect("parses");
    parse("on\n  list VarA\n  count(VarA) > 3\nwhen\n  X\ndo\n  log(\"x\")\n")
        .expect_err("`count(` is documented but not read");
}

// ── nesting bound ────────────────────────────────────────────────────

fn too_deep(e: &ParseError) -> bool {
    e.kind
        == ErrorKind::NestingTooDeep {
            limit: ogar_from_ki::MAX_NESTING,
        }
}

#[test]
fn ten_thousand_nots_are_refused_not_a_stack_overflow() {
    let e = when_item(&format!("{}A", "not ".repeat(10_000))).expect_err("refused");
    assert!(too_deep(&e), "{e}");
    assert_eq!(e.pos.line, 4);
    // the refusal is at the first `not` past the limit
    assert_eq!(e.pos.col, 3 + 4 * ogar_from_ki::MAX_NESTING);
}

#[test]
fn deep_parentheses_and_lists_are_refused() {
    let n = 5_000;
    let e = when_item(&format!("{}A{}", "(".repeat(n), ")".repeat(n))).expect_err("refused");
    assert!(too_deep(&e), "{e}");
    let e = do_item(&format!("X = {}1{}", "[".repeat(n), "]".repeat(n))).expect_err("refused");
    assert!(too_deep(&e), "{e}");
}

#[test]
fn deep_begin_chain_is_refused() {
    let n = 2_000;
    let src = format!(
        "{}log(\"x\"){}",
        "f(each: begin ".repeat(n),
        " end)".repeat(n)
    );
    let e = do_item(&src).expect_err("refused");
    assert!(too_deep(&e), "{e}");
}

#[test]
fn nested_if_and_nested_calls_are_refused_by_their_own_names() {
    let mut s = String::new();
    for _ in 0..1_000 {
        s.push_str("if A then\n");
    }
    s.push_str("log(\"x\")\n");
    for _ in 0..1_000 {
        s.push_str("end\n");
    }
    assert_eq!(do_item(&s).expect_err("refused").kind, ErrorKind::NestedIf);
    let call = format!("{}1{}", "f(".repeat(5_000), ")".repeat(5_000));
    assert!(matches!(
        do_item(&call).expect_err("refused").kind,
        ErrorKind::NestedCall { .. }
    ));
}

#[test]
fn exactly_the_limit_still_parses_and_the_walkers_cope() {
    let n = ogar_from_ki::MAX_NESTING;
    let it = when_item(&format!("{}A", "not ".repeat(n))).expect("parses at the limit");
    let _ = ogar_from_ki::outline(&it);
    let _ = it.condition_refs();
    assert_eq!(
        it.condition_refs()[1].negated,
        n % 2 == 1,
        "negation parity over {n} nots"
    );
    let _ = it.clone();
    let it = when_item(&format!("{}A{}", "(".repeat(n), ")".repeat(n))).expect("parses");
    let _ = it.condition_refs();
    let src = format!(
        "{}log(\"x\"){}",
        "f(each: begin ".repeat(n),
        " end)".repeat(n)
    );
    let it = do_item(&src).expect("parses at the limit");
    let l = lift_source(
        &format!("on\n  ogit/_id\nwhen\n  X\ndo\n  {src}\n"),
        &LiftOptions {
            item_id: Some("deep".into()),
        },
    )
    .unwrap();
    assert_eq!(l.item.other_calls.len(), n);
    drop(it);
    // one more is refused
    assert!(when_item(&format!("{}A", "not ".repeat(n + 1))).is_err());
}

// ── newlines inside the parentheses of specialised calls ─────────────

/// The guide pages show calls over several lines (`if` page's `execute(`,
/// `action` and `query` pages). No page limits where a newline may appear
/// inside the parentheses, so every call, specialised or generic, accepts
/// one after `(`, around `,` and before `)`.
#[test]
fn newlines_are_free_inside_every_call_form() {
    for (src, kind) in [
        ("set(\n  Foo = 1\n)", "set"),
        ("add(\n  Foo[\"k\"] = \"v\"\n)", "add"),
        ("delete(\n  Foo\n)", "delete"),
        ("delete(\n  Foo,\n  created_on: Node1\n)", "delete"),
        ("delete(Foo\n)", "delete"),
        ("log(\n  \"m\"\n)", "log"),
        ("log.warn(\n  \"m\"\n)", "log.warn"),
        ("log(\"m\"\n)", "log"),
        ("sleep(\n  5\n)", "call:sleep"),
        ("action(\n  \"C\",\n  a: \"1\"\n)", "action"),
        ("x: Y = execute(\n  \"c\"\n)", "call:execute"),
    ] {
        let it = do_item(src).unwrap_or_else(|e| panic!("{src:?}: {e}"));
        assert_eq!(stmt_kinds(&it.do_.stmts), vec![kind.to_string()], "{src:?}");
    }
}
