# Directory simulation PoC — `ogar-dir-sim`

Status: **PoC, 2026-10-03.** Builds on PR #313 (`ogar-dir-core` / `ogar-ad` /
`ogar-az`). Pure and in-memory. Nothing here writes to AD, Entra, Exchange,
LDAP or PowerShell. There is no network, process or file I/O in the crate.

```
observed G0 ──rule──► G1 ──rule──► G2 ──validate──► "desired" ──diff(G0,G2)──► ExecutionPlan ──X  (actuator: later)
```

## 1. What already existed, and what was reused

| Existing | Where | Decision |
|---|---|---|
| `ScenarioBranch` / `ScenarioDiff` / `ScenarioWorld` | lance-graph-contract `scenario.rs` | **Not adopted. Its shape is mirrored.** It is the named write-divergent branch over a `forked_from` Lance version, which is the concept used here. Its fields are NARS/archetype-specific (`inference_mode`, `archetype_prior`, `fork_seed`, interventions as opaque `u64`), and `ScenarioDiff` is a set of counts, not semantics. |
| `VersionedGraph` (`tag_version`, `at_version`, `diff`) | lance-graph `graph/versioned.rs` | **Not adopted (conflict V1).** Node ids are `u32`, which cannot hold a 128-bit directory GUID. History is linear, each write overwrites the whole state, and the diff reports additions only (no removals, no attribute changes). Its *tag* model is mirrored: `"observed"` / `"desired"` are tags, and `VersionId` is designed to map 1:1 onto a Lance version once persisted. |
| `LanceVersion = u64`, `TemporalPov` | contract `temporal_pov.rs` | Same scalar shape as `VersionId`. Not imported, so the crate stays free of the git dependency. |
| `CausalEdge64` | `causal-edge` | Does not fit. It is 8 bytes and cannot carry two 128-bit endpoints. |
| `ActionDef` / `ActionInvocation` | `ogar-vocab` | `ActionInvocation` is the **runtime** record: string ids plus a Pending/Committed/Failed lifecycle. A `PlannedOp` is what an actuator would later lower into one. It is not an invocation itself. |
| `ogar-action-handler` executors | OGAR | This is the side that does I/O (shell, SSH). It is exactly what the plan boundary keeps out. |
| `ogar-loco` / `ogar-r2il` | OGAR | These are the program/call ABI. They are a natural future encoding for compiled rules, but they are not a graph-rule engine. Not used. |
| `FieldMask` / `WideFieldMask` / `standing_mask` | contract | These are **column** masks. `dirty ∩ interest` is the right future shape for "which observation fields would invalidate a plan". They are not used for rows. |
| `RowFocusMask` | contract `attention_facet.rs` | It holds address subtrees, not row bitsets. |
| lance-graph-java `Mask` | lgj | It is the row-bitset algebra (`and` / `minus` over `u64` words). `Population` has the identical shape so it can be swapped in later. |
| `Guid128`, `ogar-ad` records | PR #313 | Reused. Identity is `Guid128`, and `observe::from_ad` reads the `DirRecord` slots. |

## 2. Boundary

There is one new crate, `ogar-dir-sim`. It depends on `ogar-dir-core` and
`ogar-ad` (for the observation bridge only). Its modules are `graph`,
`population`, `rule`, `store`, `validate`, `plan` and `observe`.

## 3. Versions and provenance

`VersionStore` is append-only.

- An **observed** version is a root. It holds a full snapshot and its origin is
  `Origin::Observed { source, observed_at_ms }`.
- A **simulated** version holds `parent` plus `delta: Vec<Change>`. Its origin
  is `Origin::Simulated { rule: RuleId, evidence }`. Its state is the parent's
  state with the delta applied. This is structural sharing by delta: no full
  copy is stored.
- `VersionId` is the logical clock. The `lineage(v)`, `version(v)` and
  `explain_membership` functions read provenance directly from the history.
  There is no separate log.

The lifecycle stages are expressed as data, not as a workflow enum:

- **OBSERVED** is `Origin::Observed`, plus the tag `"observed"`.
- **SIMULATED** is `Origin::Simulated`.
- **DESIRED** is the tag `"desired"`. Only `promote_desired` sets it, and only
  after validation passes.
- **OBSERVED AFTER EXECUTION** is a newer observation. The system has converged
  when `diff(observed, desired)` is empty.

## 4. Rule

```rust
trait Rule {
    fn id(&self) -> RuleId;                                     // e.g. ExchangeAccess/v1
    fn propose(&self, g: &GraphState, evidence: &[EvidenceRef]) -> Vec<Change>;
}
```

A rule has no I/O handle and cannot mutate anything. The slice ships three
example rules:

- `GrantGroup`: an explicit population is granted a group.
- `ImplyGroup`: computes `active ∩ members(source) − members(target)`.
- `SetPrimarySmtp`: a compare-and-set of one attribute.

## 5. Invariants

`validate(&GraphState) -> Vec<Violation>`, which is structured and sorted. It
checks three things:

- `DuplicateSmtp { address, owners }`: no two active users may share the
  normalized primary SMTP.
- `DuplicateUpn { upn, owners }`: no two active users may share the
  normalized UPN.
- `DanglingMembership { user, group, missing }`: a membership must point to an
  existing user and an existing group, each of the right kind.

`promote_desired` validates first. If validation fails, the `"desired"` tag is
left untouched and the version stays in the store with its recorded verdict, so
a rejected future remains available as evidence.

## 6. Semantic diff and execution plan

`Change` is both what a rule proposes and what a diff reports:

- `AddMembership`
- `RemoveMembership`
- `SetAttribute { from, to }`

A change carries the value it expects to replace, so applying a stale change is
refused (compare-and-set).

`ExecutionPlan { basis, target, ops: Vec<PlannedOp { op, precondition }> }`:

- `basis` is the observed root.
- `target` must currently carry the `"desired"` tag.
- `op` is one of `AddGroupMember`, `RemoveGroupMember`, `SetAttribute`.
- `precondition` is `NotMember`, `IsMember` or `AttributeEquals(value)`, taken
  from the basis.

This is the optimistic reality check a future actuator runs before each
operation: if the precondition still holds it executes; otherwise it
re-observes and re-plans. The plan contains no shell text, endpoints or
credentials.

## 7. Population compatibility

Rules receive the whole version and select **populations**: a `Population` is a
bitset over the version's sorted node order, with `and`, `minus` and `count`.
For example, `ImplyGroup` is one expression over sets, not a loop of per-user
workflows. The bit index is an execution ordinal valid only within one version.
Identity stays `Guid128`.

The suitable lance-graph primitives for scaling this up are:

- the lgj row `Mask` algebra, for the population expressions;
- `WideFieldMask` with `standing_mask::fires`, for "which observed fields
  invalidate this plan";
- Lance versions, as `VersionId`.

## 8. Conflicts and open points

- **V1 — VersionedGraph.** lance-graph `VersionedGraph` uses `u32` node ids and
  has an additions-only diff. Persisting this store on Lance needs 128-bit node
  keys and a removal- and attribute-aware diff upstream, or a separate
  directory dataset that mirrors this store's semantics.
- **V2 — no row-population mask in the contract.** `Population` is a local copy
  of the lgj `Mask` shape and should be retired when a shared row mask exists.
- **V3 — node creation and deletion.** These are not part of the `Change`
  algebra in this slice. A plan whose target changes the set of nodes is
  refused with `PlanError::NodeSetChanged`.
- **V4 — the "active" signal.** It comes only from AD's `userAccountControl`.
  Entra `accountEnabled` is not yet mapped by `observe`.
