//! Execution probe: a `ruff_spo_triplet::Model` method, lifted to an
//! `ogar_vocab::ActionDef` via `ogar_from_ruff::lift_actions`, dispatched
//! through the reference `NativeCommandExecutor` — "lifted action ->
//! ActionHandler -> result", the executable half of Falsifier #3
//! (E-OGAR-CONVERGENCE-SHAPE).
//!
//! **Falsifier #3 gap (documented, not implemented here):** the full
//! falsifier is `op::work_package::update(shape) -> ActionHandler -> kanban
//! transition -> Lance tombstone`. This probe covers `lifted action ->
//! ActionHandler -> result` only. The **kanban transition** (an action
//! executing IS a mailbox-owned board transition) and the **Lance
//! tombstone** require the lance-graph `ractor` runtime + a mailbox/board —
//! **not present in `/workspace/ogar`** (this repo has no `ractor` /
//! `lance-graph` runtime dependency at all). That remainder is the `[H]`
//! ERP-wiring half of the execution layer; it lands in a lance-graph-side
//! probe, not here.

use ogar_action_handler::NativeCommandExecutor;
use ogar_from_ruff::lift_actions;
use ogar_from_schema::action_ws::CapabilityExecutor;
use ruff_spo_triplet::Model;

/// A single method lifted to an ActionDef, then dispatched through the
/// reference ActionHandler as an `ExecuteCommand` capability. This is the
/// minimal "lifted action -> execution -> result" end-to-end — the tested
/// machine (`NativeCommandExecutor`), driven from the harvest arm.
#[test]
fn lifted_action_dispatches_through_native_executor() {
    // 1. Lift a trivial model with one method.
    let mut m = Model::new("Invoice");
    m.functions.push(ruff_spo_triplet::Function {
        name: "post".to_string(),
        reads: vec![],
        writes: vec!["state".to_string()],
        calls: vec![],
        raises: vec![],
        traverses: vec![],
    });
    let actions = lift_actions(&m);
    let action = actions
        .iter()
        .find(|a| a.predicate == "post")
        .expect("post action lifted");

    // 2. Bridge: the ActionDef's predicate becomes the command payload. (This
    //    is the reference bridge — production routing binds real capability
    //    params via the schema; here we prove the seam mechanically.)
    let bound = vec![(
        "command".to_owned(),
        format!("echo dispatched:{}", action.predicate),
    )];

    // 3. Dispatch through the tested executor.
    let result = NativeCommandExecutor
        .execute(NativeCommandExecutor::CAPABILITY, &bound)
        .expect("dispatch runs");

    // 4. Assert the end-to-end result carries the lifted predicate.
    assert_eq!(
        result[0],
        ("output".to_owned(), "dispatched:post".to_owned())
    );
    assert_eq!(result[2], ("exitcode".to_owned(), "0".to_owned()));
}
