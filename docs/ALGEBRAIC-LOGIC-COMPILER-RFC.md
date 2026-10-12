# RFC: Typed Logic Compiler over OGAR Classes, Methods, Schemas and the Mask/Fold Algebra

**Date:** 2026-10-12. **Status:** DESIGN PROPOSAL / documentation only. **No new instruction, new CE64 meaning, persisted schema, runtime, permission or external write is authorized.**
**Companion:** [Falsification and migration plan](ALGEBRAIC-LOGIC-COMPILER-FALSIFICATION.md).
**Proposal ID:** D-TLAC-0. **Intent:** make heterogeneous logics typed readings and procedures over one existing physical executor, not multiple logic engines.

## 0. Thesis, invariants and non-goals

**Hypothesis:** A useful, finite subset of NARS/NAL and CE64, Heckhausen–Gollwitzer action-phase logic, OGIT/OGAR automation, IAM reconciliation, CRM workflows, and some PLN/MeTTa rules can lower through the **same physical mask/fold algebra**, provided each is bound to a *separate declared semantic type and proof obligations*. A shared machine instruction is **not** evidence that all those logics mean the same thing.

Architectural separation:

- **Class**: canonical OGAR identity, inheritance, applicable object/edge population, ClassView reading and access surface.
- **Schema**: typed fields, null/unknown policy, constraints, effect and evidence semantics, version and legal transitions.
- **Method**: stable function identity + signature, pure procedure or guarded action, claimed algebraic laws, body reference and provenance contract.
- **Compiler**: resolve symbols, bind types, normalize legal rewrites, lower to R2IL procedures and mask-risc; refuse unsound cases.
- **Physical execution**: borrowed SoA planes, `mask-risc::Program` and `ndarray::simd`, never a bespoke per-logic row loop.
- **DO boundary**: OGAR `ActionDef`/`ActionInvocation`, hard RBAC/state/commit checks, then capability executor. Reasoning can suggest an action; it cannot authorize one.

**Unchanged contractual floors:**

1. The 8-bit ALU/ISA lanes are meaning-free bytes. The V3 `classid:u32 + 12-byte opaque register` may be read differently under a ClassView. CE64, NARS frequency/confidence, epistemic bits, identity, SPO and witness are *readings*, not a hard-coded new layout. No bit/tenant/opcode mint here.
2. `ClassId` is address, never permission. Existing canonical id/codebook is linked, not duplicated. Version and class-scoped reading gates apply before decoding or writing.
3. No serialization or row/array materialization in the hot path; fold algebra reads borrowed lanes, graph-flow holds handles. Explicit, authorized and bounded materialization only at an external boundary.
4. No external effects from `mask-risc`, a NARS goal, an LLM tool response, a workflow branch or an `ActionProposal`. External mutation must traverse the hard OGAR gate.
5. No second reasoning executor unless a **named, executable insufficiency test** proves the existing procedures cannot express an exact required result within the specified memory/performance limits.
6. Do not treat a stated law (commutative, idempotent, monotone) as a free optimizer hint. A checked law certificate is a prerequisite for that rewrite.

This is neither a new universal Boolean logic nor a claim of NARS/PLN/MeTTa equivalence. Population masks express selection; truths, bindings, source independence, ordered events, and external effects require richer typed objects and explicit laws.

## 1. Verified code inventory (as examined on 2026-10-12)

| Contract | Existing surface | Verified limitation / caveat |
|---|---|---|
| Canonical types | [`ogar-vocab`](../crates/ogar-vocab/src/lib.rs): `Class`, `MethodDecl` with `MethodKind` / `RecordSemantics` and opaque `body_source`, plus `ActionDef` / `ActionInvocation` / `KausalSpec` | `MethodDecl` describes harvested methods but is not a verified executable algebra body; `ActionDef.reads/writes` are name-level effect annotations, not computed values |
| Function inventory | [loco orchestration gap](LOCO-ORCHESTRATION-GAP.md): `LocoConcept::Inventory` 0x1702 is minted as a function registry concept | Full functions-as-objects address/body resolver is **not implemented** by that declaration |
| NARS vocab | [`ogar-loco/src/nars.rs`](../crates/ogar-loco/src/nars.rs): **34** recipes = **9 Datapath, 19 Control, 6 Gate**; class-selected vocabulary with `0x90..=0xB1` range | All Control recipes currently have `body_refs = 0`. A vocabulary row is not executable procedure parity |
| Bulk executor | [lance-graph mask-risc](https://github.com/AdaWorldAPI/lance-graph/blob/main/crates/lance-graph-mask-risc/src/ir.rs): `Pred`, `MaskOp::{And,Or,Xor,AndNot,Not,Ternlog,Gather}`, `Terminal::{Any,All,Count,Keep,BlendI32}`, grouped and strided folds | One `Program` has **one terminal**; outputs and ordering laws depend on terminal and input layout |
| Procedure dialect | [lance-graph R2IL](https://github.com/AdaWorldAPI/lance-graph/blob/main/crates/lance-graph-r2il/src/lib.rs): `FoldDialect` lowers fold calls onto mask-risc; `Val` is scalar/slot/address, no row-sized loco value | Unsupported R2IL operations and branches on populations deliberately refuse/poison, not automatically executed |
| CE64/NAL | [lance-graph #1496](https://github.com/AdaWorldAPI/lance-graph/pull/1496) **merged** documentation: D-HL-0..10 and D-NALP-0..11, sparse hypothesis layers and NAL compiler plan | `lance-graph-nal` / `ogar-nal` and 64 hypothesis lanes are **proposals**; no equivalence established by the document |
| PLN/attention | [#1497](https://github.com/AdaWorldAPI/lance-graph/pull/1497) **draft** PLN/Shannon/EIG source and falsifier map | PLN truth transfer and NARS confidence are distinct; scheduler comparisons unmeasured |
| Hyperon/MeTTa | [#1498](https://github.com/AdaWorldAPI/lance-graph/pull/1498) **draft** MeTTa/metagraph parity and source map | Row masks do not preserve a nondeterministic multiset of substitutions by themselves |
| Orchestration | [`graph-flow-kanban`](https://github.com/AdaWorldAPI/rs-graph-llm/blob/main/graph-flow-kanban/src/lib.rs): `Planning → CognitiveWork → Evaluation → {Commit,Plan,Prune}` | An **inner cognitive decision cycle**; not automatically the 4 psychological action phases |
| Action gate | [`graph-flow-action-ogar`](https://github.com/AdaWorldAPI/rs-graph-llm/blob/main/graph-flow-action-ogar/src/lib.rs): `GatedOgarHandler` wraps capability executor, runs only after `commit_via` def/RBAC/guard/MUL checks | No proposal may circumvent `ActionInvocation` or let a model construct an authorized action |
| Workflow ↔ fold | [`graph-flow-lance`](https://github.com/AdaWorldAPI/rs-graph-llm/blob/main/crates/graph-flow-lance/README.md): generation-checked plan/result handles, mask-set fanout-as-fold, explicit materialize | External handle identity, fold-result → stable `ActionInvocation` DO adapter, Quack/Rig retrieval and pause-as-bytes remain marked **Not built** |
| HubSPO-rs | [H-ADR-7](https://github.com/AdaWorldAPI/HubSPO-rs/blob/main/.claude/plans/hubspo-integration-v1.md): workflows and delays as Lance rows, Quack due masks, graph-flow execution | `ractor` and supervisor dependency/actor-per-record workflow are **explicitly forbidden** for this consumer |
| Rig | [AdaWorldAPI/rig](https://github.com/AdaWorldAPI/rig): provider/tool interfaces and separate LanceDB vector adapter | `rig-lancedb` does **not** implement the full lance-graph Quack/CE64 substrate |

Read [OGAR as IR](OGAR-AS-IR.md), [The Firewall](THE-FIREWALL.md), [HIRO DO lift](HIRO-DO-ARM-LIFT.md) and [ActionHandler gate](ACTIONHANDLER-TURSTEHER.md) before changing types or runtime policy.

## 2. Proposed compiler contract and dataflow

```text
OGIT TTL / Ruby / Python / Elixir / CRM / IAM / Narsese / finite MeTTa
            │ producer / parser (semantic source only)
            ▼
OGAR registry: ClassId + Class + MethodDecl + Schema + ClassView
            │ link MethodOrdinal and declared semantic reading
            ▼
Typed LogicMethodSpec (sidecar; not a new core IR field)
            │ effect & type check; legal algebraic rewrites only
            ▼
Pure expression graph → OGAR loco procedures / R2IL FoldDialect
            │
            ▼
mask-risc::Program → ndarray::simd over borrowed Lance planes
            │
            ├── RowMask / FoldResult / Truth / BindingSet / PlanHandle
            │                                  │ bounded RBAC materialize
            │                                  ▼
            │                              Rig / LLM
            │
            ├── Evidence revision / new question / re-plan [THINK]
            │
            └── ProposedAction [NOT executable]
                            │
            OGAR ActionInvocation + ClassRbac + StateGuard + MUL
                            │
                     CapabilityExecutor [DO]
                            │
                  idempotent receipt + observation
                            └── post-action evaluation / new evidence
```

**No new layer is justified solely to give existing operations an attractive name.** Use the existing `Class` and `MethodDecl` as canonical source declarations and resolve a separate typed compile-time method manifest until a specific lossless roundtrip test proves an IR extension is necessary.

Proposed illustrative type signature, **pseudocode, not a shipped API**:

```rust
struct LogicMethodSpec {
    owner: ClassId,
    ordinal: MethodOrdinal,        // stable within owner; resolved by registry
    schema_version: SchemaVersion,
    reading: ClassViewReadingId,   // declared bytes→semantics mapping
    inputs: Vec<Sort>,
    output: Sort,                 // RowMask | Fold | Truth | Bindings | Goal | Plan
    effect: EffectClass,          // Pure | ReadVersion | Propose | ExternalMutate
    body_ref: ContentAddressedRef,
    evidence_policy: EvidencePolicy,
    verified_laws: Vec<LawCertificate>,
}
```

A method has *identity* independent from its source-language string or runtime address. Binding resolves names to classids, field offsets, lane kinds and method ordinals **once**; the hot path gets prebound input/constant/slot addresses and versioned handles.

### 2.1 A typed algebra, not a universal truth table

A plain Boolean row mask is `RowMask<Population,Snapshot,Scope>`. It may be intersected only with a mask for the *same* row universe and source generation. A semijoin may transform mask universes only through a **declared typed foreign key**. Related sorts must remain distinct:

- `RowMask<C,V,S>`: selected row addresses.
- `Truth<NarsFC,Stamp>`: frequency/confidence together with independent source identities; combining values needs NARS law and evidence-overlap check.
- `EpistemicState<Reading>`: direct/indirect, known/unknown etc. from declared CE64/59..63 interpretation, **not** equal to statistical confidence.
- `ThreeValued<Truth,False,Unknown>`: null and open-world unknown must not be silently mapped to a Boolean false.
- `Belnap4` or other multivalued logics: a proposed typed product of masks only if the lattice truth tables and merges are independently validated.
- `BindingMultiset<Var,Term>`: ordered/nested and multiply matching substitutions. Cannot be represented as a single row mask in general.
- `StateTransition<S,Event>`: from-state, guard, ordered event, to-state and terminal/deadline semantics.
- `ActionProposal<BodyRef>`: intent and expected effects; never an authorized mutation.

Distinct semantic readings can reuse the *same* ndarray/mask-risc instructions without assuming equal inference laws or equal results.

### 2.2 Law-aware rewrites with precise refusals

| Law/procedure | Valid physical interpretation | Guard/negative counterexample |
|---|---|---|
| Boolean `AND/OR/NOT/TERNLOG` | exact combinational mask logic on one population and version | null/unknown/SQL three-valued tables and RBAC scope mismatch |
| Existential `Any/All/Count` | mask reduction | choice of empty-set convention is schema-specific; existential bindings not just row count |
| Semijoin `Gather/EqU32Via` | mask across FK edge, with gated fetch | missing FK/out-of-range, circular traversal, cardinality/multiset/ordered output |
| Finite numeric fold | `GroupReduce`, `GroupPowerSums`, `GroupCrossPowerSums` where already supported | overflow bounds, weight conversion, rounding/order and duplicated evidence |
| Conditional typed lane | `BlendI32` for selected lane values | class/property permission and semantic sort of chosen values |
| Structural state guard | mask over typed state + `on_enter` effect proposal | execution is never a masked write; persistence, approval and ordering remain explicit |
| Revision / retraction | accumulate **independent** evidence; then apply checked truth function once | `Σw` is not idempotent under replay; repeated source cannot inflate NARS confidence |
| Schedule / fixpoint | external typed composition of finite folds | termination, monotonicity and iteration order need a certificate; general recursion not silently supported |
| Contextual causality | CE64 Pearl/epistemic facet predicates + independent interventions | association/correlation is not proof of causal effect |

A verified law is keyed by **method id + semantic reading version + input sorts + null policy + evidence policy + numerical bounds**. No certificate means no reorder/fuse. Do not invent shared NARS/PLN truth math because both expose "confidence".

### 2.3 Three equivalent Boolean guards (first proof target)

Illustrative declarations, **not a new DSL implementation**:

```text
class IAMUser {
  schema Directory;
  method needs_license_review() : RowMask<IAMUser, V> =
      active & !licensed & independent_evidence;
}
class Deal {
  schema HubSPO;
  method may_enter_stage() : RowMask<Deal, V> =
      active & !blocked & properties_complete;
}
class CognitiveCycle {
  schema Rubicon;
  method may_advance() : RowMask<CognitiveCycle, V> =
      eligible & !veto & gate_open;
}
```

After binding each field to its own domain lanes, all three have the **same canonical Boolean expression shape**, so their optimized *operator sequence* can be equivalent. They do **not** share physical lane IDs, population identity or semantics. The first falsifier compares canonicalized plan shape and separate plain-loop oracles under adversarial masks, including null/unknown and domain mismatches.

### 2.4 NARS revision as a weighted fold (gated hypothesis)

D-NALP proposes carrying a fixed-point weight `w` and frequency `f` with an independent evidential stamp per delta. For each statement key, the group cross-power fold can calculate `Σw` and `Σ(w·f)` exactly within documented bounds; derive frequency and confidence **once at the end**, not by repeatedly rounding u8 revisions. This is only legal after a provenance check that contributing stamps are independent or have been deduplicated under the agreed NARS rule.

```text
Source observations → immutable indexed deltas → typed stamp-overlap guard
→ statement-key semijoin → GroupCrossPowerSumsI32
→ truth law / scalar oracle → output Truth<F,C,Stamped>
```

No semantic guarantee that the exact current NARS confidence formula, D-HL hypothesis layers or ONA are equivalent until the pinned differential suite passes. A missing per-row truth-producing terminal is **a probe question**, not automatic permission to add an opcode.

## 3. Heckhausen/Gollwitzer Rubicon as an outer action-phase model

The psychological action-phase model distinguishes **predecisional deliberation**, **preactional planning**, **actional execution** and **postactional evaluation**. Gollwitzer's implementation intentions refine the preaction phase: *if context X, then act Y*. An internal goal commitment is **not** a permission to mutate production state.

Separate **three** boundaries:

1. **Goal-intention selection (psychological Rubicon):** choose a goal under competing hypotheses and evidence.
2. **Action planning:** choose an if/then situation/action method, compute preconditions and simulation, set wait/approval policy.
3. **OGAR hard commit:** verify ActionDef match, ClassRbac, StateGuard, MUL gate, freshness/idempotency and any required human authorization **before the real executor**.

The existing `lance_graph_contract::kanban::KanbanColumn` / `graph-flow-kanban` cycle is `Planning→CognitiveWork→Evaluation→{Commit,Plan,Prune}`. Its `Evaluation` is **pre-acceptance cognitive validation**. It is **not** the psychological postaction evaluation of an observed external effect. Preserve the existing enum discriminants and transition semantics; add an *outer* typed phase projection (or graph-flow context) if validated, rather than renumbering the inner state machine.

```text
Outer: deliberate → goal_intention → implementation_intention → execute
                                                        │ OGAR authorization
      revise_goal ← postaction_expected_vs_observed ← external receipt
Inner:            Planning → CognitiveWork → Evaluation → Commit | Plan | Prune
```

Postaction evaluation must distinguish observed consequence, causal attribution and confidence update. A "success" returned by an executor is not the same as observed target state. No Libet timing offset is a universal psychological calibration; current offsets are internal code conventions, not a measured physiological fact for IAM/CRM.

## 4. ONA, PLN and Hyperon remain independent oracles

- [OpenNARS for Applications](https://github.com/opennars/OpenNARS-for-Applications) has `NAR_AddInputBelief`, `NAR_AddInputGoal` and `NAR_AddOperation`; `Decision_Execute` performs selected operations in ONA. **Adapt the selected operation to `ActionProposal`**, never run an ONA callback against IAM/CRM. Compare goals, anticipated consequences, failures and procedure choices on pinned fixtures.
- #1496/D-NALP: literal Narsese oracle vs typed mask-risc lowering, source independence and sparse hypothesis layers.
- #1497: separate *logic* parity from FIFO/confidence/Shannon/EIG/Moore/Staunen *attention* policies, with equal candidate/evidence/compute budgets.
- #1498: MeTTa output can be nondeterministic with repeated substitutions, nested atoms and higher-order rewrites. A Boolean mask of matching source rows is not a proven replacement for a typed multiset or general metagraph recursion scheme. Preserve exact cardinality, ordering policy and error/empty distinctions; otherwise refuse.
- Distinguish observation truth, inference uncertainty, epistemic evidence strength and causal intervention. Correlation or confidence does not create entitlement to execute.

**Research anchors (conceptual, not runtime parity):** Heckhausen & Gollwitzer (1987), *Thought contents and cognitive functioning in motivational versus volitional states of mind*; Gollwitzer (1999), *Implementation Intentions: Strong Effects of Simple Plans*. Executable sources: [ONA](https://github.com/opennars/OpenNARS-for-Applications), [modern PLN](https://github.com/trueagi-io/PLN), [Hyperon](https://github.com/trueagi-io/hyperon-experimental).

## 5. Virtualized actor semantics without ractor dependence

A stable logical actor is *address* `(tenant,ClassId,NodeGuid)` + versioned state + pending events. Its execution is activated by a mask over persisted work, not a permanent actor process. On activation: read generation-checked scope, borrow required data, run pure methods, propose an action, revalidate before DO, write receipt. The durable owner remains the Lance version/write lane; a per-object actor is not necessary.

Erlang/OTP `gen_statem` is a **behavioral oracle** for postposition, entry effects, timeouts and ordered event replay; BEAM/ractor may remain optional isolated control-plane adapters, not required hot-path runtimes. HubSPO-rs H-ADR-7 prohibits ractor/supervisor workflow actors and stores resume/deadline/offset as rows with Quack due masks. Cross-process action delivery must prove stable ordering, backpressure, replay and idempotent **external outcome**, not just exactly-once local fold.

## 6. Rig ↔ graph-flow ↔ lance-graph ↔ OGAR

- **Rig = LangChain-shaped interface:** models/tools/retrieval, not independent canonical truth. Its LanceDB vector adapter is not general Quack/CE64 retrieval.
- **rs-graph-llm graph-flow = LangGraph-shaped orchestration:** stateful task graph, session pause/resume, condition edges, outer action intent. Retain `graph-flow-lance` handle-only Context for plans/results; materialize only a bounded, policy-projected export to a model.
- **lance-graph = execution and versioned knowledge substrate:** R2IL, Quack, mask-risc, folds, sparse CE64 and source generation checks.
- **OGAR = class/method/signature/authority:** capability declaration and actual effect gate. `GatedOgarHandler` cannot be bypassed by any agent.
- **z8run = trigger/external edge orchestration, where present:** already shares the handle registry with graph-flow-lance; do not fork a second one.

Explicit known missing seams in [graph-flow-lance README](https://github.com/AdaWorldAPI/rs-graph-llm/blob/main/crates/graph-flow-lance/README.md): cross-process handle identity, Quack retrieval via Rig, suspension-as-bytes, fold result→stable DO invocation mapping, and z8run's AI nodes consolidation. Arrival time must **not** be action identity: duplicate MQTT/QoS1 or webhook delivery must not repeat a side effect.

## 7. IAM and HubSPO worked semantic examples

### IAM source conflict and license repair (simulation first)

```text
Observed (v): AD status active; Entra license missing; HR sources disagree.
Pure query: tenant_scope ∩ active ∩ ¬licensed ∩ independent_source
Hypotheses: delayed provisioning | missing group | competing source overwrote
Goal: verified correct assignment WITHOUT violating canonical source precedence
Method: inspect_memberships → simulate_assignment → validate uniqueness/capacity
Proposal: ActionDef.assign_license(NodeGuid, desired_license, witness, source_version)
Gate: source authority, unique UPN/SMTP, ActorContext role, approval, idempotency
After DO: read next independent Entra observation; compare desired vs observed.
Update: CE64/NARS belief about cause only after evidence-overlap and counterfactual checks.
```

Canonical GUID joins, source precedence, identity uniqueness and grants are **hard invariants**, not learning weights. If the postaction observation fails, do not blindly repeat the same external mutation.

### HubSPO workflow / deal stage

```text
Query: CRM owned rows (dir-sim team membership) ∩ stage predicate ∩ due workflow
Intent: advance deal only if required fields present and explicit trigger applies
ActionDef: move_stage; StateGuard: allowed previous stage(s)
graph-flow: plan → optional human approval → run_gated
Workflow run state / resume_at / offset: persisted Lance rows, due via Quack mask
Postaction: observe accepted CRM version and side-effect receipts, update plan
```

The CRM owner mask is a compiled owner-set filter, not an OU-tree sibling/parent walk. Field-level authorization precedes wire/materialization. No actor-per-deal, custom timer supervisor or DataFusion fallback is implied.

## 8. Function inventory, effect system and read/write authority

A function can be *represented* as an OGAR-identified object even if execution happens elsewhere. A method body may be an R2IL procedure, a graph-flow plan (control), an opaque external handler (effect) or explicitly unsupported. The registry needs typed address, body hash/version, parameters, result sort, approved input lenses, effect level, provenance requirement and verified optimizer laws.

| Effect class | Result | Permitted executor |
|---|---|---|
| Pure | RowMask, Truth, Fold, CandidateSet | existing R2IL / mask-risc / ndarray only |
| ReadVersion | generation-checked plan/result handle | authorized Lance read; declared scope |
| Propose | inert ActionProposal with witness | deterministic builder / graph-flow, no side effects |
| ExternalMutate | receipt / observed next version | **only** OGAR commit-checked CapabilityExecutor + external idempotency/outbox |

A raw prompt, NARS goal or method return type must **never** be coerced into `ExternalMutate`. Use explicit typed escalation. A stale generation, absent schema reading, unauthorized field, duplicate effect or unresolved bindings set yields a typed refusal.

## 9. Decision queue (no automatic implementation approval)

- **O1** Where is the first lossless, linked Function Inventory body stored? Existing `MethodDecl`/`ActionDef`, sidecar, or new canonical field? Require codegen roundtrip.
- **O2** Stable method identity/version and canonical numeric ordinals; no string lookups/inline handlers on hot path.
- **O3** Sort system for mask universe/version/scope, Null/Unknown, Truth/Stamp, BindingMultiset, ActionProposal.
- **O4** Certification/rewrite regime for laws; numeric bounds, overflow, idempotency, order, independent sources.
- **O5** CE64 reading version and evidence overlap; defer to D-HL and D-NALP operators.
- **O6** Psychological goal commitment vs technical `KanbanColumn::Commit` vs OGAR gate; outer state home.
- **O7** Cross-process plan and action IDs, deterministic replay and external receipts/outbox.
- **O8** How exactly Rig obtains bounded, policy-checked Quack exports without alternate state authority.
- **O9** Executable exact subset for ONA/NAL/PLN/MeTTa; unsupported outputs refused, not approximate without label.
- **O10** Whether actor activation semantics improves a measured workload without contradicting HubSPO H-ADR-7.
- **O11** Minimal primitive/byte only after insufficiency proof; no second evaluator.
- **O12** Provenance through revision, action execution and postaction causal evaluation without double count.

## 10. Wave plan and owners

| Wave | Deliverable | Exit rule |
|---|---|---|
| W0 **this PR** | document typed architecture, truth boundaries and falsifiers | docs only, no ABI/code edits |
| W1 | pure 3-domain Boolean-guard normalization prototype | same normalized ops; separate independent scalar oracles |
| W2 | Function Inventory address + method binding/roundtrip | no duplicate authoritative method table and no hot string |
| W3 | NARS stamped revision / typed epistemic gates | exact oracle + duplicates/overflow/unknown falsifiers |
| W4 | psychological goal/implementation/postaction typed state | distinct pre/post evaluation, event/timeout parity |
| W5 | mock-only IAM/CRM proposal → gate → receipt → observe | hard policy failure closes without external command |
| W6 **conditional** | smallest necessary new typed primitive/sort only | documented failing counterexample from earlier waves |

Owners: **OGAR** semantic IR/inventory; **lance-graph** bind and physical folds/evidence; **ndarray** generic SIMD implementation; **rs-graph-llm** graph and action execution; **Rig** untrusted model IO; **IAM/HubSPO** domain source authority and policy. No wave changes other repos implicitly.

Details and executable negative controls: [falsification plan](ALGEBRAIC-LOGIC-COMPILER-FALSIFICATION.md).

**Architecture in one line:** *Classes choose contextual meaning, schemas constrain it, methods name laws, mask/fold algebra performs eligible pure work, and OGAR alone admits effects.*
