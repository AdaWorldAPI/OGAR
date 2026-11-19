//! What the dt64 converter does with `ActionDef`s lifted from Knowledge Items
//! (`ogar-from-ki`). `ogar-from-ki` depends on `ogar-vocab` only, so this test
//! lives here, with `ogar-from-ki` as a dev-dependency: no cycle.
//!
//! Run with `--features dt64`.
#![cfg(feature = "dt64")]

use lance_graph_contract::class_view::{ClassId, ClassView};
use lance_graph_contract::ontology::{DisplayTemplate, FieldRef};
use lance_graph_contract::set_rules::Truth;
use lance_graph_dt64::MethodId;
use ogar_action_handler::dt64::{ConvertRefusal, FieldRole, method_of};
use ogar_from_ki::{LiftOptions, lift_source};
use ogar_vocab::class_ids;

const HTTP: &str = include_str!("../../ogar-from-ki/tests/fixtures/http_request.ki");
const SSH_LINE: &str =
    include_str!("../../ogar-from-ki/tests/fixtures/ssh_execute_command_line.ki");

struct Fields {
    class: ClassId,
    fields: Vec<FieldRef>,
}

impl Fields {
    fn textline(labels: &[&str]) -> Fields {
        Fields {
            class: class_ids::TEXTLINE,
            fields: labels
                .iter()
                .map(|l| FieldRef::new((*l).to_string(), (*l).to_string()))
                .collect(),
        }
    }
}

impl ClassView for Fields {
    fn fields(&self, class: ClassId) -> &[FieldRef] {
        if class == self.class {
            &self.fields
        } else {
            &[]
        }
    }
    fn template(&self, _class: ClassId) -> DisplayTemplate {
        DisplayTemplate::Detail
    }
    fn dolce_category_id(&self, _class: ClassId) -> u8 {
        0
    }
}

fn opts(id: &str) -> LiftOptions {
    LiftOptions {
        item_id: Some(id.to_string()),
    }
}

#[test]
fn http_def_as_lifted_is_refused_because_it_names_no_class() {
    // `on ogit/_id` names no class: `object_class` is empty, and the converter
    // refuses an empty concept as unminted. Nothing is registered under a
    // class that was never named.
    let l = lift_source(HTTP, &opts("http-request")).unwrap();
    let view = Fields::textline(&["ProcessIssue"]);
    assert_eq!(
        method_of(&l.defs[0], MethodId(0), &view),
        Err(ConvertRefusal::UnmintedConcept {
            concept: String::new()
        })
    );
}

#[test]
fn http_def_over_a_class_the_caller_chose_is_refused_by_the_guard_value() {
    // The CALLER maps the item onto a minted class (the lift does not).
    let l = lift_source(HTTP, &opts("http-request")).unwrap();
    let mut d = l.defs[0].clone();
    d.object_class = "ogit/textline".into();

    // Class WITH the matching field: the guard field resolves, the value does
    // not: `TestHTTPRequest` is a string, the converter states decimal codes.
    let with = Fields::textline(&["ProcessIssue"]);
    assert_eq!(
        method_of(&d, MethodId(0), &with),
        Err(ConvertRefusal::UnportedGuard {
            kausal: "StateGuard: value `TestHTTPRequest` is not a decimal u32 code".into()
        })
    );

    // Class WITHOUT the field: the guard field is unknown.
    let without = Fields::textline(&["state", "text"]);
    assert_eq!(
        method_of(&d, MethodId(0), &without),
        Err(ConvertRefusal::UnknownField {
            role: FieldRole::GuardField,
            name: "ProcessIssue".into()
        })
    );

    // Call inputs are never what refuses a def: `url`, `timeout`, `params` and
    // the results `code`, `body` are not fields and are not asked to be.
    d.kausal = None;
    d.reads = vec![];
    assert!(method_of(&d, MethodId(0), &without).is_ok());
}

#[test]
fn a_numeric_equality_registers_as_a_three_valued_guard() {
    // SYNTHETIC, in the documented subset: one equality over a field with a
    // decimal code, on a minted class.
    let src = "on\n  ogit/_type == \"textline\"\nwhen\n  state == \"1\"\ndo\n  action(\"Run\", width: \"5\")\n";
    let l = lift_source(src, &opts("num")).unwrap();
    let d = &l.defs[0];
    assert_eq!(d.object_class, "textline");
    let view = Fields::textline(&["state", "text", "width"]);
    let m = method_of(d, MethodId(0), &view).expect("registers");
    assert_eq!(m.name, "Run");
    assert!(m.when.is_some(), "the equality is the WHEN");
    assert_eq!(m.truth, Truth::ThreeValued, "unknown never fires");
    assert!(
        m.effects.is_empty(),
        "the call is external, not an F000 word"
    );

    // The same item over a class with no `state` field.
    let bare = Fields::textline(&["text"]);
    assert_eq!(
        method_of(d, MethodId(0), &bare),
        Err(ConvertRefusal::UnknownField {
            role: FieldRole::GuardField,
            name: "state".into()
        })
    );
}

#[test]
fn a_guard_over_a_name_that_is_also_a_call_argument_is_refused_by_name() {
    // SYNTHETIC: the tested name `width` is also an argument of the call. The
    // class has no such field, so it is a call parameter in a guard.
    let src = "on\n  ogit/_type == \"textline\"\nwhen\n  width == \"1\"\ndo\n  action(\"Run\", width: \"5\")\n";
    let l = lift_source(src, &opts("cp")).unwrap();
    let view = Fields::textline(&["state"]);
    assert_eq!(
        method_of(&l.defs[0], MethodId(0), &view),
        Err(ConvertRefusal::CallParameterInGuard {
            role: FieldRole::GuardField,
            name: "width".into()
        })
    );
}

#[test]
fn a_bare_name_def_converts_with_no_guard_and_the_name_as_a_read() {
    // The bare name is unresolved in the lift: no `kausal`, so no WHEN. The
    // converter therefore states NO guard for it (it is not an equality, and
    // not a presence test). `Synthetic` stays a read, which needs a field.
    let src = format!("on\n  ogit/_type == \"textline\"\nwhen\n  Synthetic\ndo\n{SSH_LINE}");
    let l = lift_source(&src, &opts("ssh")).unwrap();
    let d = &l.defs[0];
    assert!(d.kausal.is_none());
    assert_eq!(d.reads, vec!["Synthetic"]);

    let without = Fields::textline(&["text"]);
    assert_eq!(
        method_of(d, MethodId(0), &without),
        Err(ConvertRefusal::UnknownField {
            role: FieldRole::Reads,
            name: "Synthetic".into()
        })
    );
    let with = Fields::textline(&["Synthetic"]);
    let m = method_of(d, MethodId(0), &with).expect("registers");
    assert!(m.when.is_none(), "no guard is stated for a bare name");
    assert_eq!(m.truth, Truth::TwoValued);
}
