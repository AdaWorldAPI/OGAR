# D-TLAC falsification, parity, safety and migration plan

**Date:** 2026-10-12. **Status:** RESEARCH PLAN, NO TESTS EXECUTED HERE.
**Parent proposal:** [Typed logic algebra compiler RFC](ALGEBRAIC-LOGIC-COMPILER-RFC.md).
**Scope:** OGAR method-as-object / semantic binder ↔ lance-graph R2IL-mask-risc ↔ rs-graph-llm/Rig ↔ OGAR hard DO gate ↔ IAM and HubSPO. No production actions. No source-bit reinterpretation.

## 0. A conjecture becomes evidence only by surviving an oracle

### Falsification discipline

Each experiment must specify:
- **Claim:** one *particular* semantic/physical equivalence, not universal expressivity.
- **Oracle:** independent plain-loop, formal state transition, pinned ONA/Hyperon/PLN runtime or existing source-verified method.
- **Input surface:** class/field mapping, sort, population identity, snapshot generation, schema reading, null/evidence policy, auth scope.
- **Counterexample arm:** a known wrong implementation must fail (e.g. dropped Not, Sum where idempotent Max is required, duplicate evidential base, reused stale handle).
- **Measurements:** allocation bytes, decoded bytes, fold/scan counts, CPU/time when relevant, IO/ops, error identity, cardinality, replay results. Do not call absence of extra allocations "zero cost".
- **Kill condition:** stop this lowering when semantics differ, not merely improve performance until discrepancy is hidden.
- **Permission:** probes may not mint opcodes, persist CE64 meanings, change default ClassView interpretation, modify production IAM/HubSPO or execute remote commands.

Required labels: [G] observed in current code; [P] proposed test not executed; [H] semantic hypothesis; [X] disproven. A passing fixture is evidence for the tested subset and versions only.

### Independent oracles and controls

| Subject | Semantic oracle | Material reference / control |
|---|---|---|
| Boolean mask & relation guards | plain scalar iteration plus SQL-style truth-table oracle | same rows/versions/keys, deterministic mask-risc byte program |
| Typed method binding | OGAR `Class/MethodDecl/ActionDef` roundtrip and symbol table | reject unbound class, wrong reading, duplicate method name+version |
| NARS | pinned literal Narsese implementation(s), ONA C tests and scalar truth formulas; use separate roots | per-case truth values, substitutions, evidential stamps, temporal implications |
| PLN | pinned modern PLN MeTTa rules/fixture interpreter | don't silently use NARS formula as PLN oracle |
| Hyperon/MeTTa | pinned Hyperon interpreter with native fixture and output multiplicity | explicit multiset of substitutions and error/unreduced cases |
| Phase transition | explicitly defined Heckhausen/Gollwitzer phase semantics; OGAR/kanban contract transition table for inner loop | never compare postaction phase to precommit Evaluation as though equal |
| Erlang `gen_statem` | bounded scripted OTP reference on isolated fixture (if available) | postpone/FIFO, entry, timeout and retry traces |
| IAM | dir-sim snapshot validator and policy oracle | no directory API mutation, source precedence/uniqueness and known tenant |
| HubSPO | H-ADR-7 persisted workflow and field/owner mask oracle | compare prior/next Lance version and mock executor receipt |

Every pin needs repo commit SHA, toolchain, feature flags and fixture hash in the experimental report. Upstream license/runtime is an integration concern, **not** a reason to silently substitute a homemade oracle.

## 1. Type and truth semantics to prove before compiling

Proposed method sort skeleton (illustrative, not implemented):

```text
RowMask<ClassId, Tenant, RowUniverse, LanceGeneration>
Truth<LogicFamily, EvidenceBase>
Epistemic<ClassViewReading>
BindingsMultiset<VariableTypes, TermShape>
Transition<FromState, Event, ToState, Version>
ActionProposal<ActionDefId, ObjectGuid, Preconditions, Witness>
AuthorizedAction<CommitReceipt>
```

**T1: Population and version:** AND/OR inputs must have the same universe, tenant and snapshot version, unless a typed join is explicitly performed. Stale plan handles fail closed, even when the raw number of rows coincides.

**T2: Three-valued and open-world logic:** represent TRUE and FALSE as disjoint masks, UNKNOWN as neither (or two-bit known+value), and define truth tables explicitly. Kleene AND:

```text
T(A∧B) = T(A) ∩ T(B)
F(A∧B) = F(A) ∪ F(B)
U(A∧B) = universe \ (T ∪ F)
NOT: T(¬A)=F(A); F(¬A)=T(A); U(¬A)=U(A)
SQL WHERE keeps T only; missing/unknown ≠ false
```

This shape is a candidate implementation over masks, not a proof that other epistemic logics use Kleene tables. Test \`UNKNOWN ∧ FALSE = FALSE\`, \`UNKNOWN ∧ TRUE = UNKNOWN\`, \`NOT UNKNOWN = UNKNOWN\`, and empty/all-null columns. Encode source absence separately from a false belief.

**T3: Belnap/positive-negative evidence (optional):** if four-valued `{neither,true,false,both}` is requested, the two evidence-support masks are a typed product; their knowledge/truth order and conflict resolution cannot be inferred from a single TERNLOG immediate. Require a full independent 16-case binary operation table per operator before any rewrite.

**T4: Quantifiers and multiplicity:** `Any/All` operate over a declared universe; universal over the empty set must use the declared mathematical/SQL convention. Distinctness, correlated joins, order and repeated bindings require separate typed witnesses. \`Count(mask)\` is not necessarily \`Count(bindings)\`.

**T5: Algebra law metadata:** the compiler must reject invalid claims of associativity, commutativity and idempotence. A law certificate includes sort, version, overflow regime, null policy, evidence policy, and a reduced/optimized-vs-unoptimized oracle. Especially **Sum is not idempotent**, and floating regrouping is not byte-exact by default.

## 2. Probe grid: semantic parity

The following tests are **proposed**. Names specify their first intended case and a disable/kill arm. All entries remain unchecked until a run records full receipt.

| ID | Construct and fixture | Oracle | Kill / disable guard |
|---|---|---|---|
| **TLAC-P01** | Three domain expressions with form \`A & !B & C\`: IAM active/unlicensed/evidenced, HubSPO active/unblocked/complete, Rubicon eligible/not-vetoed/gate-open; sparse/dense/empty/all masks, 1/63/64/65/256/257 rows | plain loop per domain + normalized mask-risc op graph | Make \`Not\` a no-op or swap field binder: at least one fixture **must** go red; plan graph shape may match while lane identity must remain distinct |
| **TLAC-P02** | Typed mismatches: foreign tenant, class, row universe, source generation, semantic ClassView reading, wrong-lane width | type-check refusal | If mismatched inputs produce a mask, fail; *equal row count* is not compatibility |
| **TLAC-P03** | Null/Unknown 3VL truth-table exhaustive 3×3 pairs and nullable field comparisons | Kleene/SQL scalar | Treat missing as false or \`NOT NULL\` as \`TRUE\`: red; if not, test is vacuous |
| **TLAC-P04** | `Any/All/Count/Keep` on zero rows and 1/64/65 selected, tail bits set | scalar quantifier oracle | Remove tail mask clearing / change empty ALL behavior: red |
| **TLAC-P05** | FK semijoin (gated `Gather` / `EqU32Via`) with repeated, missing and out-of-range IDs, forbidden field scope, 1%/50%/100% survivor masks | scalar FK resolution and owner/field policy | Force ungated gather or return false on authorized rows; semantics red for invalid FK; benchmark must also report read bytes |
| **TLAC-P06** | GroupCount/Sum/PowerSums/CrossPowerSums and \`BlendI32\` on mixed signs, bounds and nullable lanes; mutate group order | exact mathematical reference under current documented widths; scalar baseline | overflow silently wraps/saturates, combines finalized means or changes unsupported sort to scalar: red |
| **TLAC-P07** | NARS truth deduction/induction/abduction/revision with pinned f/c, disjoint stamps and delayed duplicate deliveries | scalar NARS/ONA fixtures | Remove stamp overlap check: doubled contribution changes result and **must** be rejected |
| **TLAC-P08** | CE64 statistical f/c and 59..63 epistemic reading, valid/invalid class reading, alternatives for bits 40..63 | explicit ClassView interpretation + bit-exact raw register | Remove reading gate, convert bytes as implicit semantic model, alter storage bits: red |
| **TLAC-P09** | Two mutually exclusive hypotheses over same node/row and sparse delta ordering; merge-on-read vs in-place revision | exact scalar rational / weighted accumulator + D-HL oracle | Under overlapping evidence, extra confidence or c=u8 stall must be visible; test must include a case where candidate differs from reference |
| **TLAC-P10** | Modern PLN rule and ONA NAL counterpart on same premises; vary formula and attention separately | pinned upstream independent engines | Any **unlabeled** difference in truth/confidence or ordering is a refusal, not harmonized "parity" |
| **TLAC-P11** | Hyperon \`(pair (bin) (bin))\` producing four solutions, repeated vars, nested outputs, empty and unreduced/Error | pinned MeTTa interpreter multiset | Two source rows collapsed into only two results instead of four: fail; only equal \`COUNT\` not sufficient |
| **TLAC-P12** | 4 action phases and two crossings: predecision goal-intention, preaction plan, OGAR guard/commit, postaction observed outcome | table state machine + event trace | Map existing inner \`Evaluation\` onto postaction without observed receipt: red |
| **TLAC-P13** | Erlang-style postpone, FIFO resume, timeout cancellation, entry effect and crash/retry against persisted virtual-state record | scripted OTP or formal reference traces | Reordering postponed events, running twice or missing timeout: fail; do not need BEAM in production |
| **TLAC-P14** | IAM identity conflict (duplicate UPN/SMTP normalized), HR source precedence, two tenants, 2nd source with stale membership, missing license | dir-sim and hard-policy oracle | NARS confidence overrides precedence/RBAC or produces a live write: fail |
| **TLAC-P15** | HubSPO team scope: direct membership, nested subteam, ADMINS nested but not subteam, sibling/parent groups, field mask; 10M logical population | `hubspo-iam::scope` and independent scalar owner oracle | Parent/sibling leaks; masked field reaches Rig output; actor/supervisor dependency appears: fail |
| **TLAC-P16** | Repeated action delivery, crash after external provider success before local receipt, crash before send, clock skew | idempotent mock API + outbox/reconcile protocol | Provider effect count ≠ 1 for logically same invocation; local version alone insufficient |
| **TLAC-P17** | Handle/session resumed at identical generation then republished source, old plan and old result | current `graph-flow-lance` stale-source tests | If stale handle executes/materializes on new generation, fail |
| **TLAC-P18** | Unauthorized field, wrong scope, adversarial model-generated tools, fabricated \`AuthorizedAction\` | OGAR `run_gated` mock executor call count + canary | Any denied/missing-field observation or external call before gate: fail |
| **TLAC-P19** | Goal/procedure learning: ONA suggests action, OGAR only builds proposal, blocked gate, observed expectation failure | ONA selected operation vs independent expected action and mock receipt | ONA callback reaches API before gate, or a failed postcondition treated as success: fail |
| **TLAC-P20** | Rewrites: unoptimized vs certified fused TERNLOG, mask survivor gating, effectful order preserved | bit-exact mask + deterministic action trace | Fusion changes truth rows, action order, source version or provenance: fail |

### Anti-vacuity requirements

- At least one non-empty, non-full mask per target scenario; at least one *differing* result under wrong source/tenant/field/null/unknown.
- Stamps must overlap and not overlap; if only disjoint stamps exist then removing overlap check cannot prove anything.
- One paired numeric dataset must cause intermediate u8 revision vs exact accumulation to differ by **at least 2 confidence/frequency codes** under the stated precision.
- Include cases with 2+ bindings for one atom and with four outcomes from a two-binary-slot product; otherwise Hyperon equivalence is untested.
- Independently vary inference rules vs schedulers; a model answering from cached prior evidence is an invalid run.
- An external mock must increment a counter **after** committing its own effect but **before** failure injection, to expose the crash-after-send window.

## 3. Algebraic equivalence is not semantic equivalence

Comparison levels (**do not skip**):

1. **L0 shape**: same typed method signature, universe, version, authorization and ClassView reading.
2. **L1 pure algebra**: same normalized primitive call DAG under independent bound ids, not the same hard-coded lanes across domains.
3. **L2 value parity**: exact masks/scalars/typed result sets under independent oracle. For approximate numeric laws declare tolerance *in advance*.
4. **L3 provenance parity**: evidence independence/duplicate handling; no statistical laundering.
5. **L4 transition parity**: correct order, postponement, timeout, source version, goal/action/postaction distinction.
6. **L5 external effect parity**: authorization and exactly-once *logical* effect via idempotency/reconciliation. Not inferable from L1–L4.
7. **L6 runtime viability**: peak memory, row materializations, read bytes, time, parallel load, model tokens, number of folds.

A library may pass L0–L2 and still **not** implement NARS/PLN, temporal planning or safe IAM. Report levels independently.

### Minimal canonicalization test

```text
Input 1: IAM.active AND NOT IAM.licensed AND IAM.independently_grounded
Input 2: CRM.active AND NOT CRM.blocked AND CRM.complete
Input 3: Rubicon.eligible AND NOT Rubicon.veto AND Rubicon.gate_open

Normalization -> AND(AND(a, NOT(b)), c) or one equivalent 3-input truth table
Physical operations -> 3 predicates + bitwise combination + Keep
Bound sources: ALWAYS different classid, fields, tenant, generation and auth.
Test equality of normalized *op shapes*, not equality of bound Program bytes.
```

Negative case: under a NULL-aware source, `NOT missing` remains UNKNOWN. Without known/validity masks, a seemingly equivalent Boolean plan must be rejected or explicitly lowered with the extra validity mask; otherwise the compiler changes semantics.

## 4. NARS/CE64 and causal epistemology matrix

| Concern | Separate representation | Required test |
|---|---|---|
| Observed frequency | NARS f/statistical frequency | scalar truth oracle; never call this causal probability |
| Evidence confidence | NARS c/statistical weight, `Σw` accumulator | duplicate/overlap stamps and round-once bounds |
| Epistemic status | declared CE64 reading in bits 59..63 | direct vs indirect, intermediate known/unknown, class version |
| Correlation vs causation | explicit CARE/Pearl/counterfactual interpretation | ablation/intervention fixture; observational association alone not promoted |
| Hypothesis concurrency | Alpha sparse delta overlays, versions, 64 layers **design** | layer isolation and merge-on-read vs 10 existing rung lanes; no hidden persisted-bit decision |
| Change of belief | explicit revise/suspend/refuse verdict | no reweight without new independent root |
| Attention policy | FIFO/confidence, Shannon, EIG, Moore/Staunen | equal rule set/evidence budget/CPU & decode bytes |
| Goal/action | ONA goal/procedure suggestion | action proposal, gate separation and observed outcome |

Crucial numerical bound: group moments with a weighted f are safe only under the chosen integer width, bounded rows and signedness. If a missing primitive is required to write computed per-row truth values, first prove a *typed lane-producing* insufficiency against the already available `BlendI32` and the R2IL procedure path; add the smallest generic primitive only after that proof.

## 5. Rubicon and external DO acceptance contract

The proposed outer state table is **not** the existing Kanban enum:

| Outer phase | Input | Output | Mandatory constraint |
|---|---|---|---|
| Deliberation | candidate goals/hypotheses | chosen goal intention, alternatives rejected | evidence/provenance preserved |
| Preaction | goal + contextual if/then | plan, applicable `ActionDef`, deadlines/witness | fixed authority and environment preconditions |
| Action | approved invocation | executor receipt + next observed version | `commit_via`; idempotent external effect |
| Postaction | expected vs observed | satisfaction/feedback, revise/continue/escalate | do not treat successful API call as proven goal outcome |

The inner cycle remains `Planning→CognitiveWork→Evaluation→{Commit,Plan,Prune}`; its \`Evaluation\` happens before accepting/persisting candidate cognitive result, whereas outer postaction evaluates a realized action. Provide two separate typed state components, or a demonstrated lossless projection before merging their types.

**Fault injection timeline:**

```text
T0 source v read / private logical invocation id built
T1 goal proposed and plan generation pinned
T2 action guard/RBAC/current-generation check
T3 outbox prepared (idempotency key and requested effect)
T4 provider mutates (inject crash BEFORE receipt is saved)
T5 replay with same key checks provider receipt before resend
T6 observe source version v+1; postcondition may still fail
T7 append evidential witness without reusing an old stamp as new support
```

At T4→T5, neither an isolated Lance append nor a claimed "exactly once" local queue suffices. A real integration must have idempotent provider behavior or an explicit reconciliation protocol.

### IAM simulation fixture

- Users with valid and conflicting UPN/mail/proxy-address; normalize for comparisons without changing canonical GUID.
- HR source A vs source B ordered by an explicitly declared authority/date rule, plus recently changed account.
- Missing Entra license, delayed provisioning, wrong group membership, and a false-positive NARS proposal.
- Approver lacking required role, absent tenant grant, stale directory generation, and a second concurrent reconciliation.
- Expected: deterministic plan shows diffs, does no production write, refuses duplicates; NARS can rank investigations but never override authority/uniqueness/grants.

### HubSPO scope and CRM fixture

- Owned record vs team-owned vs global; direct team group vs nested subteam vs non-team AD group; sibling/parent excluded.
- Company/deal stage guard and required properties, plus NULL fields, workflow resume_at and published version offset.
- Field-level sensitive property withheld from export before a model sees it.
- Expected: Quack mask equals independent scalar authorized owner set. Workflow state and due run in Lance rows; no ractor dependencies, actor mailboxes or new timers.

## 6. Performance and memory: report facts, don't predict speed

Report by row count `n ∈ {0,1,63,64,65,256,257,4096,1_000_000,10_000_000}` (if feasible) and mask selectivity `p ∈ {0,0.001,0.01,0.5,1}`. For each oracle-matching program measure:

- cold/warm wall time (median and dispersion), CPU time, number of physical fold programs/terminals, decoded bytes, bytes read from Lance and scratch peak;
- allocated bytes per invocation and peak heap, particularly **no row-sized intermediate** in the method stack;
- output count, truth values and error/refusal case, exact stale handle behavior;
- idempotent replay counts and external API call count (mock);
- when Rig participates, materialized export bytes/tokens and whether denied fields appear.

Compare independent arms:

- scalar reference vs mask-risc compiled method;
- specialized hand-written current guard vs generic typed compiler;
- fused ternlog vs ordinary mask ops vs survivor-gated condition;
- current R2IL procedure vs proposed method-linked procedure;
- persistent row workflow vs materialized actor reference *only in isolated benchmark*, not as HubSPO dependency;
- NARS identical inference engine with FIFO/confidence/Shannon/EIG/Moore scheduling; then identical scheduler with NARS vs PLN where semantics allow.

**KILL on overhead without expressivity:** if generic typed dispatch adds meaningful hot path allocations, row copies, extra folds, or serializes/allocates row-sized values, do not promote; fix the binder/offline compilation or stop. Never claim a wall-time target that was not benchmarked.

**KILL on useless optimization:** identical semantic outcomes with increased code/bytes/complexity and no measured improvement in CPU, IO, memory, or maintainability => keep the simple existing procedure.

## 7. Implementation waves / ownership / no-go checkpoints

| Wave | Code owner | Deliverable (only after preceding gate) | Stop test |
|---|---|---|---|
| **W0** | OGAR docs | this RFC, this falsification plan, discovery-map pointer | docs don't alter runtime or ABI |
| **W1** | lance-graph test consumer + OGAR declarations | pure canonical 3-guard declaration and binder, no new opcode | P01–P06, P20 and disabled guards |
| **W2** | OGAR `ogar-loco` / vocabulary inventory | function-as-object identity + body-ref resolution under canonical classid | missing/stale/duplicate function refuses; W1 parity unchanged |
| **W3** | lance-graph CE64/NAL + OGAR reading | candidate stamped revision, exact truth and epistemic mask | P07–P11, no persisted bit meanings change |
| **W4** | rs-graph-llm | typed outer phase context, inner Kanban preserved; graph-flow-lance stable plan/session handle | P12, P13, P17 |
| **W5** | OGAR + IAM / HubSPO + rs-graph-llm | mock-only stable DO plan → gate → outbox receipt → observation → revision | P14–P19; forbidden dependency guards stay green |
| **W6** | only measured deficient primitive owner | smallest new typed lane operator/sort or refused subset | fail-before/green-after with oracle, ABI migration and version gate |

**Shared rule:** no producer-to-DDL behavioral lowering; no AI callback executing commands; no actor-per-object dependency in HubSPO; no cross-repo change without its own review/contract parity. Existing classids and method ordinals are retrieved, not re-minted. Newly proposed IDs require a separate minting decision.

### Open exact decisions

- Which class reading/version defines CE64 and which bit fields are currently blocked under D-HL?
- What data model represents evidential source identity and exclusion of stamp overlap, even when all row keys match?
- Can `MethodDecl` remain source AST while Inventory carries linked typed executable bodies, avoiding a new canonical field?
- What method-result sorts need new type-level representation, versus new machine operations?
- Which outer phase and Intent/ImplementationIntention type attaches to `graph-flow`? How are action goal targets and postaction observations versioned?
- What is the exact stable invocation identity and provider receipt contract on durable restart?
- How are law certificates verified/invalidated if the ClassView/field schema evolves?
- Can the zero-row and typed unknown cases be represented without new bool semantics in the global ISA?

## 8. PR review checklist

- [x] **Design only**: no files outside documentation are required by this plan.
- [x] Current code and explicit "Not built" gaps linked without treating plans as implementations.
- [x] No new opcode/semantic bits or automatic action authorization proposed.
- [x] Semantic mismatch, provenance, null/unknown, temporal and effects explicitly separated.
- [x] P01–P20 with independent oracles, error expectations and negative/disable controls specified.
- [ ] No tests claimed executed; executable probes not yet implemented.
- [ ] No performance parity claimed; benchmarks deferred to wave-specific PRs.
- [ ] Upstream ONA/PLN/Hyperon runtime versions and licenses pinned at test implementation.
- [ ] Security review of plan and external API receipt semantics before any real IAM/CRM mutation.
- [ ] Explicit operator decision on optional new types/bytes/generic primitives.

**Conclusion:** a single *physical* masking and fold executor can host many *typed* logic families if the compiler preserves the family-specific laws and refuses all unsupported semantics. The test above determines the subset, not the architectural slogan.
