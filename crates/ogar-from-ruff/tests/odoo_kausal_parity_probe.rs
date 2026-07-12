//! `D-KAUSAL-CONSUME-PIN-ODOO` (W1, odoo -> odoo-rs transpile arc) — real-Odoo-
//! source kausal-parity **consume** probe.
//!
//! Unlike `woa_parity_probe.rs` (a hand-transcribed synthetic `ModelGraph`,
//! standing in for a not-yet-built SQLAlchemy frontend), this probe drives
//! the REAL, already-shipped Odoo frontend end to end:
//!
//! ```text
//! real .py source (verbatim, see tests/py/account_payment_term.PROVENANCE.md)
//!   -> ruff_python_spo::extract_from_source   (AST -> ModelGraph)
//!   -> ogar_from_ruff::mint::compile_graph_python::<OdooPort>
//!        -> lift_model_graph_python            (THINK arm: Class)
//!        -> lift_actions                       (DO arm: ActionDef + KausalSpec)
//! ```
//!
//! The pin: every `compute='_compute_x'` field with a `@api.depends(...)`-
//! decorated computer method must round-trip, verbatim dotted paths and all,
//! into `ActionDef.kausal == Some(KausalSpec::Depends { paths })` on the
//! `ActionDef` whose `predicate` is that computer method's name — Arm A of
//! `D-ATC2-KAUSAL-AUTARK` (SPEC-ATC2-OGAR), now witnessed against real Odoo
//! 19 source instead of a hand-built fixture.
//!
//! # Scope (named, not hidden)
//!
//! - Arm A (`compute=` + `@api.depends` -> `KausalSpec::Depends`) AND Arm B
//!   (`@api.constrains` -> `KausalSpec::Constrains`) are both pinned here —
//!   Arm B opened up once ruff #49 landed `Function::constrains` on main
//!   (`D-ATC2-KAUSAL-RUFF-GATED`). The fixture has no `@api.onchange`, so the
//!   Onchange arm is not exercised by this witness. `computed.stored` (Arm D)
//!   is a `Class`-side field, out of scope for this DO-arm/kausal probe.
//! - `account.payment.term` / `account.payment.term.line` are NOT in
//!   `ogar_vocab::ports::ODOO_ALIASES`, so both mint the bootstrap facet
//!   classid `0` (see `mint.rs`'s
//!   `unmapped_model_mints_the_bootstrap_address_not_a_wrong_classid`) — this
//!   probe is about the DO-arm (kausal), not classid convergence, so the
//!   bootstrap value is asserted as a fact, not treated as a defect.

use ogar_from_ruff::mint::compile_graph_python;
use ogar_vocab::KausalSpec;
use ogar_vocab::ports::OdooPort;
use ruff_python_spo::extract_from_source;

/// Verbatim Odoo 19 source, `addons/account/models/account_payment_term.py`
/// at `AdaWorldAPI/odoo` commit `2c78d5f1ff1d4e32b0c480c3b2714f31fb71bef7`.
/// See `tests/py/account_payment_term.PROVENANCE.md`.
const ACCOUNT_PAYMENT_TERM_PY: &str = include_str!("py/account_payment_term.py");

/// One pinned `@api.depends(...)` compute-method fact: method name (==
/// `ActionDef::predicate`) and its verbatim dotted dependency paths, read
/// directly off the source above (not derived by running the extractor
/// first and copying its output).
struct ExpectedDepends {
    method: &'static str,
    paths: &'static [&'static str],
}

/// `AccountPaymentTerm` (`account.payment.term` -> `account_payment_term`)
/// compute methods carrying `@api.depends(...)`. `_compute_example_preview`
/// is the interesting case: it emits TWO fields (`example_preview` and
/// `example_preview_discount`), both `compute='_compute_example_preview'`,
/// so it must appear exactly once in the compiled `actions` (one `ActionDef`
/// per METHOD, not per field).
const PAYMENT_TERM_DEPENDS: &[ExpectedDepends] = &[
    ExpectedDepends {
        method: "_compute_fiscal_country_codes",
        paths: &["company_id"],
    },
    ExpectedDepends {
        method: "_compute_currency_id",
        paths: &["company_id"],
    },
    ExpectedDepends {
        method: "_compute_discount_computation",
        paths: &["company_id"],
    },
    ExpectedDepends {
        method: "_compute_example_invalid",
        paths: &["line_ids"],
    },
    ExpectedDepends {
        method: "_compute_example_preview",
        paths: &[
            "currency_id",
            "example_amount",
            "example_date",
            "line_ids.value",
            "line_ids.value_amount",
            "line_ids.nb_days",
            "early_discount",
            "discount_percentage",
            "discount_days",
        ],
    },
];

/// `AccountPaymentTermLine` (`account.payment.term.line` ->
/// `account_payment_term_line`) compute methods carrying `@api.depends(...)`.
const PAYMENT_TERM_LINE_DEPENDS: &[ExpectedDepends] = &[
    ExpectedDepends {
        method: "_compute_display_days_next_month",
        paths: &["delay_type"],
    },
    ExpectedDepends {
        method: "_compute_days",
        paths: &["payment_id"],
    },
    ExpectedDepends {
        method: "_compute_value_amount",
        paths: &["payment_id"],
    },
];

/// `AccountPaymentTerm` methods carrying `@api.constrains(...)` — the Arm-B
/// witness that opened up once ruff #49 landed `Function::constrains` on main
/// (`D-ATC2-KAUSAL-RUFF-GATED`). Each becomes
/// `KausalSpec::Constrains { paths }` (validation trigger), Depends still
/// wins when both are present (`kausal.is_none()` gate in `lift_actions`).
const PAYMENT_TERM_CONSTRAINS: &[ExpectedDepends] = &[ExpectedDepends {
    method: "_check_lines",
    paths: &["line_ids", "early_discount"],
}];

/// `AccountPaymentTermLine` methods carrying `@api.constrains(...)`.
const PAYMENT_TERM_LINE_CONSTRAINS: &[ExpectedDepends] = &[
    ExpectedDepends {
        method: "_check_valid_char_value",
        paths: &["days_next_month"],
    },
    ExpectedDepends {
        method: "_check_percent",
        paths: &["value", "value_amount"],
    },
];

/// Plain (non-compute, non-constrains) methods on `AccountPaymentTerm`, read
/// off the same source, that must NOT carry a `kausal` fact — the facts-only
/// guard (`D-ATC2-KAUSAL-AUTARK`: a method's own body `reads` never gets
/// promoted into a reactive `KausalSpec` trigger).
const PAYMENT_TERM_PLAIN_METHODS: &[&str] = &[
    "_get_amount_due_after_discount",
    "_get_amount_by_date",
    "_compute_terms", // named like a compute target but has no `compute=` field pointing at it
    "_unlink_except_referenced_terms",
    "_get_last_discount_date",
    "_get_last_discount_date_formatted",
    "copy_data",
];

fn assert_kausal_depends(
    actions: &[ogar_vocab::ActionDef],
    expected: &ExpectedDepends,
    model: &str,
) {
    let action = actions
        .iter()
        .find(|a| a.predicate == expected.method)
        .unwrap_or_else(|| panic!("{model}: no ActionDef for method {:?}", expected.method));
    let expected_paths: Vec<String> = expected.paths.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        action.kausal,
        Some(KausalSpec::depends(expected_paths)),
        "{model}::{} kausal depends-paths mismatch",
        expected.method
    );
}

fn assert_kausal_constrains(
    actions: &[ogar_vocab::ActionDef],
    expected: &ExpectedDepends,
    model: &str,
) {
    let action = actions
        .iter()
        .find(|a| a.predicate == expected.method)
        .unwrap_or_else(|| panic!("{model}: no ActionDef for method {:?}", expected.method));
    let expected_paths: Vec<String> = expected.paths.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        action.kausal,
        Some(KausalSpec::constrains(expected_paths)),
        "{model}::{} kausal constrains-paths mismatch",
        expected.method
    );
}

#[test]
fn odoo_kausal_parity_pin_account_payment_term() {
    let graph = extract_from_source(ACCOUNT_PAYMENT_TERM_PY);
    assert_eq!(graph.namespace, "odoo");
    // Two models declared in the source: AccountPaymentTerm (_name =
    // 'account.payment.term') and AccountPaymentTermLine (_name =
    // 'account.payment.term.line').
    assert_eq!(graph.models.len(), 2, "two models in the fixture");

    let compiled = compile_graph_python::<OdooPort>(&graph);
    assert_eq!(compiled.len(), 2);

    let payment_term = compiled
        .iter()
        .find(|c| c.class.name == "account_payment_term")
        .expect("account_payment_term compiles");
    let payment_term_line = compiled
        .iter()
        .find(|c| c.class.name == "account_payment_term_line")
        .expect("account_payment_term_line compiles");

    // Neither model is in ODOO_ALIASES -> bootstrap classid 0 (named in the
    // module doc as a fact, not a defect of this probe).
    assert_eq!(payment_term.facet.facet_classid(), 0);
    assert_eq!(payment_term_line.facet.facet_classid(), 0);

    // ── Arm A pin: every @api.depends compute method round-trips verbatim ──
    for expected in PAYMENT_TERM_DEPENDS {
        assert_kausal_depends(&payment_term.actions, expected, "account_payment_term");
    }
    for expected in PAYMENT_TERM_LINE_DEPENDS {
        assert_kausal_depends(
            &payment_term_line.actions,
            expected,
            "account_payment_term_line",
        );
    }

    // ── Arm B pin: every @api.constrains method → KausalSpec::Constrains ──
    // (opened up by ruff #49 / D-ATC2-KAUSAL-RUFF-GATED; Depends still wins
    // when both are present, so these are the pure-constrains methods.)
    for expected in PAYMENT_TERM_CONSTRAINS {
        assert_kausal_constrains(&payment_term.actions, expected, "account_payment_term");
    }
    for expected in PAYMENT_TERM_LINE_CONSTRAINS {
        assert_kausal_constrains(
            &payment_term_line.actions,
            expected,
            "account_payment_term_line",
        );
    }

    // Exactly one kausal-bearing ActionDef per @api.depends compute METHOD
    // (not per field — _compute_example_preview emits two fields, appears once)
    // PLUS one per @api.constrains method.
    let kausal_bearing = payment_term
        .actions
        .iter()
        .filter(|a| a.kausal.is_some())
        .count();
    assert_eq!(
        kausal_bearing,
        PAYMENT_TERM_DEPENDS.len() + PAYMENT_TERM_CONSTRAINS.len(),
        "one kausal-bearing ActionDef per @api.depends + @api.constrains method (account_payment_term)"
    );
    let kausal_bearing_line = payment_term_line
        .actions
        .iter()
        .filter(|a| a.kausal.is_some())
        .count();
    assert_eq!(
        kausal_bearing_line,
        PAYMENT_TERM_LINE_DEPENDS.len() + PAYMENT_TERM_LINE_CONSTRAINS.len(),
        "one kausal-bearing ActionDef per @api.depends + @api.constrains method (account_payment_term_line)"
    );

    // ── facts-only guard: plain methods carry no kausal fact ──
    for name in PAYMENT_TERM_PLAIN_METHODS {
        let action = payment_term
            .actions
            .iter()
            .find(|a| a.predicate == *name)
            .unwrap_or_else(|| panic!("no ActionDef for plain method {name:?}"));
        assert_eq!(
            action.kausal, None,
            "plain method {name:?} must not carry a kausal fact"
        );
    }

    let expected_kausal = PAYMENT_TERM_DEPENDS.len()
        + PAYMENT_TERM_LINE_DEPENDS.len()
        + PAYMENT_TERM_CONSTRAINS.len()
        + PAYMENT_TERM_LINE_CONSTRAINS.len();
    println!(
        "OK: account_payment_term(+line) kausal_actions={}/{} (depends={} constrains={}) plain_checked={} total_actions={}/{}",
        kausal_bearing + kausal_bearing_line,
        expected_kausal,
        PAYMENT_TERM_DEPENDS.len() + PAYMENT_TERM_LINE_DEPENDS.len(),
        PAYMENT_TERM_CONSTRAINS.len() + PAYMENT_TERM_LINE_CONSTRAINS.len(),
        PAYMENT_TERM_PLAIN_METHODS.len(),
        payment_term.actions.len(),
        payment_term_line.actions.len(),
    );
}
