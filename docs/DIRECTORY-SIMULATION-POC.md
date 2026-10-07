# Directory simulation PoC

Status: **PoC, 2026-10-03.** This builds on PR #313 (`ogar-dir-core`, `ogar-ad`
and `ogar-az`). Nothing here writes to AD, Entra, Exchange, LDAP or PowerShell,
and no network, process or file I/O exists in either crate.

```
observed G0 ──rule──► G1 ──rule──► G2 ──validate──► "desired" ──diff(G0,G2)──► ExecutionPlan ──X
```

## 1. Boundary (OGAR-AS-IR)

| crate | repo | holds |
|---|---|---|
| `ogar-dir-sim` | OGAR | Meaning only: `Change`, provenance (`VersionId`, `Origin`, `RuleId`, `EvidenceRef`, `Version`, tags), `Violation`, and `ExecutionPlan` / `Operation` / `Precondition`. |
| `lance-graph-dir-sim` | lance-graph | Execution: the SoA snapshot, versions as snapshot + overlay, rules, invariants, diff, plan and audit, all lowered through Quack onto mask-risc. |

Execution has to live in lance-graph. Quack and mask-risc path-depend on
`../../../ndarray`, so they cannot be git dependencies of OGAR. All existing
Quack consumers live in lance-graph (`lance-graph-sap`, `lance-graph-report`),
and `lance-graph-report-ogar` is the precedent for an excluded crate that
path-depends on OGAR. The arrow points one way: lance-graph depends on OGAR,
never the reverse.

## 2. Reconnaissance: operation → existing primitive

| operation | existing primitive | representation | rows materialised? | allocates | zero-copy? | used |
|---|---|---|---|---|---|---|
| node lookup | sorted `Guid128` id lane | `&[Guid128]`, ordinal = index | no | no | yes (binary search) | yes |
| membership traversal ("members of g") | Quack `GroupReduce Count` keyed on the user FK | sorted `(user, group)` `u32` lanes plus a live plane | no | a K = \|nodes\| sink | lanes borrowed | yes |
| membership add / remove | delta overlay (added rows, removed-row bitmap) | delta-sized `BTreeMap`, plus a bitmap only once something is removed | no | delta-sized | base borrowed | yes |
| UPN uniqueness | Quack `GroupReduce Count` on the normalized-key id (`GROUP BY … HAVING > 1`) | key-id lane plus the active-user plane | owners only, per colliding key | a K = \|keys\| sink | lanes borrowed | yes |
| SMTP uniqueness | same, with the overlay folded in via `Semijoin` against the active plane | same | owners only | same | yes | yes |
| dangling edge | Quack `negate(Semijoin)` lowered to `MaskOp::Gather` (anti-join) over the kind planes | membership lanes plus kind planes | offending rows only | a bitmap of membership-row size | yes | yes |
| graph diff (same root) | overlay touched-key comparison | delta-sized | no | delta-sized | yes | yes |
| graph diff (different roots, i.e. reconcile) | merge of two effective relations | O(n+m) set | effective pairs | O(n+m) | no (documented) | yes |
| subtree selection | Quack `located ∧ GeI32(depth) ∧ Cmp::MatchFacet16Strided` over the `Dn128` lane, read in place | `[[u8;16]]` + depth lanes, presence plane | no | a node bitmap | yes | yes |
| version snapshot read | `Arc<Snapshot>` plus the folded lineage overlay | shared base | no | delta-sized | yes | yes |

**Considered and not used.**

- **`ScenarioBranch` and lance-graph `VersionedGraph`.** Node ids are `u32`,
  history is linear, each version overwrites the whole state, and the diff
  reports additions only. Its tag model is kept, and `VersionId` is shaped to
  map onto a Lance version.
- **`CausalEdge64`.** It is 8 bytes and cannot hold two 128-bit endpoints.
- **`ogar-loco` / `ogar-r2il`.** They are the program-call ABI, not a rule
  engine.
- **`ActionInvocation`.** It is the runtime record an actuator would write,
  which comes after a plan.

## 3. Versions and structural sharing

An observation becomes an immutable `Snapshot` behind an `Arc`: two
populations (users, groups), each at most **65,536** nodes with its own dense
`u16` ordinal space (`UserOrdinal`, `GroupOrdinal`; no value is reserved), and
a sparse membership relation of `(UserOrdinal, GroupOrdinal)` rows — never a
dense matrix. The row lanes are 32-bit because mask-risc has no 16-bit lane;
the values are `u16`. A membership whose endpoint does not resolve is kept by
identity in a side table, never as a lane value, so no ordinal means
"missing".

A simulated version stores only `parent`, `origin` (rule and evidence) and
`delta`. A read folds the lineage's deltas into an `Overlay`:

- added membership rows,
- a removed-rows bitmap, allocated only after the first removal,
- per-attribute override maps.

Queries run over the base lanes, gated by "still live" planes, and over the
overlay rows; the two results are combined by summing the sinks or OR-ing the
masks. The base is never copied.

**Measured** (`tests/alloc.rs`, counting allocator; 1,000 / 16,384 / 65,536
users, the last being the bound): one mutation allocates the same at every
size — membership add 517 B, remove 205 B, attribute set 277 B, node create
357 B. A node delete allocates 711 B at 1,000 and 2,631 B at 16,384 and at
65,536: its membership guard runs one `Count` program whose scratch is one
tile, and a tile is capped at 16,384 rows. The same-root diff is 458–1,056 B,
flat in the same way.

The lifecycle stages are not a workflow enum:

- **OBSERVED** is `Origin::Observed` plus the tag `"observed"`.
- **SIMULATED** is `Origin::Simulated`.
- **DESIRED** is the tag `"desired"`, which only `promote_desired` sets and only
  after validation passes.
- **OBSERVED AFTER EXECUTION** is a newer observation. The system has converged
  when its diff to `"desired"` (the reconcile path) is empty.

## 4. Rule

```rust
trait Rule { fn id(&self) -> RuleId; fn propose(&self, v: &View<'_>, evidence: &[EvidenceRef]) -> Vec<Change>; }
```

A rule receives a borrowed `View` and has no I/O handle. The trait and the
rules live in lance-graph (`crates/lance-graph-dir-sim/src/rule.rs`), because a
`View` is execution state; OGAR holds only the `RuleId` and `Change` they
speak. There are three example rules:

- **`GrantGroup`** handles a request-sized list of users. Its cost is
  proportional to the request.
- **`ImplyGroup`** is a population rule: active ∧ count(source) > 0 ∧
  count(target) = 0. It reads two folded `GROUP BY` sinks and a resident plane.
- **`SetPrimarySmtp`** is a compare-and-set on one attribute.

The two sinks are combined at the consumer, never by feeding one program's
mask into another program's `Semijoin`. That respects Quack's
no-population-intermediate rule.

## 5. Invariants, diff, plan, rejection, audit

- **Invariants.** `Violation` is structured: identities plus the normalized
  value, sorted. Strings are resolved only for keys that actually collide.
- **Diff.** `Change` is shared by rule proposals and diffs. Attribute changes
  carry `from`, and a stale `from` is refused.
- **Plan.** `ExecutionPlan` holds `AddGroupMember`, `RemoveGroupMember` and
  `SetAttribute`. Each operation carries a precondition read from the observed
  basis, which is the hook for an optimistic reality check before execution.
  The plan has no shell text, endpoints or credentials.
- **Rejected futures.** A rejected version is an ordinary version that keeps
  its recorded verdict. The `"desired"` tag does not move.
- **Audit.** `explain_membership` reads the lineage directly. For example:
  observed G0 → `ExchangeAccess/v1` (evidence `REQ-1`) → G1.

## 6. Identity, ordinals, zero-copy, determinism

- **Identity versus ordinals.** `Guid128` is the only identity in provenance,
  diffs, violations and plans. An ordinal is an index into one snapshot's
  `Guid128`-sorted id lane, valid only in that snapshot.
- **Determinism.** Snapshot ordinals and dictionary ids are assigned in `Guid128`
  order, so the semantic output does not depend on input order (test t17). A
  duplicate node in one observation is refused rather than resolved by
  ingestion order.
- **Strings.** External string → ingress (`VersionStore::intern`, the
  observation) → the store's append-only label/value table → `ValueId` (raw
  value) and `KeyId` (comparison form, `normalize`). `Change`, `NodeState`,
  `Violation`, `Operation` and `Precondition` carry ids; compare-and-set,
  uniqueness and rename ordering compare ids. A string comes back only at
  egress (`value`, `key_label`). The table lives as long as the store, so an
  id means the same value from observation through simulation, the desired
  version, re-observation, reconciliation and the plan. Ordinals are never
  used as value ids.
- **Value identity, audited.** `ogar-dir-core::ValuePool` (`StrRef` offsets)
  is per record batch and not deduplicated, so its offsets are not a stable
  identity. lance-graph's `Dicts` is append-only for the store's lifetime and
  is reused as the cold store; `ValueId` / `KeyId` (in `ogar-dir-sim`) are its
  ids.
- **Remaining materialisations, all at the evidence or reconcile boundary:**
  - offending membership rows and duplicate owners, bounded by the number of
    violations;
  - `K`-slot `GroupReduce` sinks, bounded by the population or dictionary size;
  - the reconcile diff across snapshots, O(n + m).

## 7. Conflicts and open points

- **V1 — Lance persistence.** `VersionedGraph` uses `u32` ids and an
  additions-only diff. Persisting this store needs 128-bit keys and
  removal- and attribute-aware diffs upstream, or a dedicated directory dataset.
- **V2 — node creation and deletion (closed).** `Change::CreateNode` /
  `Change::DeleteNode` carry a `NodeState` (kind, active, UPN and primary
  SMTP as `ValueId`, location as `Option<Dn128>`): the content to create, or the compare-and-set expectation of a
  delete. A delete is refused while the node still has a membership on
  either side, so an edge cannot be stranded; its plan precondition
  (`ObjectRemovable`) re-checks that against reality. `Change` and
  `Operation` variant order is a safe application order: membership
  removals, deletes, attribute sets, creates, membership adds — an address
  is freed before it is claimed, including along a chain of renames
  (`b → c` runs before `a → b`). A cycle of renames (two nodes swapping an
  address) is not orderable without a temporary value and stays open.
  The plan is the desired version's net intent rebased onto the latest
  observation: work reality already shows is dropped, and drift outside the
  intent is neither planned nor reverted.
- **V2b — properties of an existing node (closed, 2026-10-07).** Every
  property of a node present in both versions now changes by compare-and-set:
  `SetAttribute` (UPN, primary SMTP, as `ValueId`), `SetActive` (the
  three-valued flag, all nine `from → to` pairs) and `SetLocation`
  (`Option<Dn128>`, never a DN string). `NodeState::apply` is the one
  reference semantics: a change applies only where the node still holds its
  `from` (else `Stale`), and `SetActive` on a group is refused (groups carry
  no flag). Plans lower them to `SetEnabled { enabled: bool }` and
  `MoveObject { to: Dn128 }` with `EnabledEquals` / `LocationEquals`
  preconditions read from the observation. A change *towards* an unknown flag
  or location is reportable by a diff but refused by the planner
  (`PlanError::NotActuatable`): "unknown" is an observation, not an intention.
  **`kind` is immutable by design**: it is identity (separate user and group
  populations and ordinal spaces, typed membership endpoints), no change can
  express it, and the planner's `Unconvergeable` now covers exactly a kind
  mismatch.
- **V3 — hierarchy coordinate (closed by ruling).** The hot location is
  `Dn128` (`ogar-dir-core::dn128`): 16 levels × `u8`, at most 256 distinct
  child codes per parent, depth as explicit side metadata (code 0 is a real
  level, no byte is stolen). The domain or tenant is not a level: it is the
  `DirectoryScope` of the observation, and a subtree query in another scope is
  refused. Subtree selection is one program: `located ∧ depth ≥ d ∧
  MatchFacet16Strided(prefix, care)`, the 16 bytes read in place. `OuHhtl`
  (`[u16; 8]`) stays the ingress and record-wire format; `Dn128::from_ou_hhtl`
  converts it and **fails closed** at a 257th child (`ChildCodeOverflow`) —
  `from_ad` then refuses the whole observation, never hashing or truncating.
- **V4 — "active" across AD and Entra (closed by ruling, 2026-10-07).**
  `NodeState::active` is three-valued (`Some(true)` / `Some(false)` /
  `None` = unknown). Across sources, **active iff at least one source is
  known and no known source says disabled; an unknown source abstains**:
  `(ad_known ∨ entra_known) ∧ (¬ad_known ∨ ad_enabled) ∧ (¬entra_known ∨
  entra_enabled)`. Absence of evidence never disables, negative evidence
  always does, and unknown/unknown stays unknown. The one definition is
  `ogar_dir_sim::effective_active`, pinned row for row; every executor must
  agree with it. `from_ad` no longer reads a missing `userAccountControl` as
  enabled: it is unknown. **Not yet wired:** an Entra user is still a
  separate node from its AD source (`sync_edges` links them as evidence), and
  no observation path merges the two, so today each node carries its one
  source's flag.
- **V6 — Exchange hybrid recipients (2026-10-07).** What an object is to
  Exchange comes from `msExchRemoteRecipientType` (flags: provision /
  migrated / deprovision mailbox, provision / deprovision archive, room,
  equipment, both = shared), `msExchRecipientDisplayType`,
  `msExchRecipientTypeDetails` and `targetAddress` — not from the account's
  enabled flag: a shared, room or equipment mailbox is a disabled account
  and still a recipient. `ogar-ad` schema v2 ingests the triplet raw (type
  details as its 64-bit decimal text). `ogar_dir_sim::exchange::Recipient`
  decodes it strictly: exactly the 27 remote-recipient-type values (the script's 26 plus 97, provisioned shared) the
  hybrid lifecycle produces (with the display and type codes that go with
  them, and the routing address while the mailbox exists), an on-premises
  mailbox with its remote-archive state, not mail-enabled — anything else
  stays raw (`Other`), never guessed. `RemoteMailboxOp` is the lifecycle:
  `Enable` (user / room / equipment / shared, with routing address), `EnableArchive`,
  `DisableArchive`, `CompleteMove`, `SetType`, `Disable`; each step refuses
  outside the table. `NodeState::recipient` (`None` = not read, distinct
  from not mail-enabled) changes by compare-and-set (`SetRecipient`); a plan
  lowers it to the single lifecycle step joining the two roles
  (`Operation::RemoteMailbox`, precondition `RecipientEquals`), or refuses
  (`NotActuatable`) when no single step does. **Open:** the `smtp:` routing
  proxy `Enable-RemoteMailbox` stamps is not added to the proxy relation by
  the vocabulary; the executor derives ownership of the routing address.
- **V7 — Hybrid identity.** On-premises and cloud name each other by three
  attributes: `mS-DS-ConsistencyGuid` (the source anchor; its base64 is the
  cloud `ImmutableId`), `msDS-ExternalDirectoryObjectId` (Entra Connect
  backsync, `User_<cloud object id>`) and `mailNickname` (the Exchange Online
  `Alias`, ingested raw). The Entra (formerly MSOL) `ObjectId`, Exchange
  Online's `ExternalDirectoryObjectId` and AD's `msDS-ExternalDirectoryObjectId`
  are one id: the cloud spellings are the bare GUID, only AD's adds `User_`.
  **Neither id is stored as a string.** `DirRecord` ABI minor 1 gives the
  first 64 reserved bytes four inline 128-bit id slots (`AttrKind::Guid`);
  `ogar-ad` schema v3 puts both anchors there. In: the anchor's mixed-endian
  bytes become a `Guid128`; `User_` / `Group_` is stripped and must match the
  object kind (else the entry is refused). Out:
  `ogar_ad::external_directory_object_id` adds the label back from the kind,
  `ogar_ad::consistency_guid` returns the AD bytes. The label is a template,
  `LabelPattern("User_{0}")` (`{0}` = the id): strip matches the text around
  `{0}`, render substitutes into it, a new kind is one more template. It has one
  definition, `ogar_dir_core::label`, which `ogar_dir_sim::identity`
  re-exports. **Open:** no executor ingests them
  yet; matching an on-premises node to its cloud object by these ids is the
  lance-graph side.
- **V8 — Addresses are one space; the routing address is the alias.**
  AD holds a remote mailbox's addresses as `proxyAddresses`
  (`SMTP:{primary}`, `smtp:{secondary}`,
  `smtp:{alias}@{tenant}.mail.onmicrosoft.com`) plus `targetAddress`
  (`SMTP:{alias}@{tenant}.mail.onmicrosoft.com`); Exchange Online shows the
  same set as `PrimarySmtpAddress` and `EmailAddresses`. The alias is
  `mailNickname`. Vocabulary: `exchange::ROUTING` is the template
  `{0}@{1}.mail.onmicrosoft.com` (`AddressTemplate`, render / strict match);
  `routing_parts` returns `(alias, tenant)`. Three invariants, as
  `Violation`s: `RoutingMismatch` (the routing address is not the template
  for the node's own alias), `RoutingNotInProxies` (the mailbox does not
  own its routing address as an SMTP proxy), and `AddressConflict` — UPN,
  SMTP, `mail` and routing addresses share one key space, so a UPN or SMTP
  address that is another object's SMTP, `mail` or routing address
  conflicts, and so does a `mail` held by another object (an admin account
  with `mail` set as a password-reset target but no mailbox). Each holder
  carries its `AddressRole`; same-attribute collisions stay
  `DuplicateSmtp` / `DuplicateUpn`. Sync: `ogar_az::SYNCED` maps the AD
  attributes to their cloud names (`physicalDeliveryOfficeName` →
  `officeLocation`, `userAccountControl` → `accountEnabled` via
  `effective_active`, `mS-DS-ConsistencyGuid` → `onPremisesImmutableId`);
  `ogar-ad` schema v4 adds `physicalDeliveryOfficeName`. **Open:** the
  executor (lance-graph) does not yet compute these three violations.
- **V5 — CI.** CI builds `lance-graph-dir-sim` against the OGAR checkout, so
  it needs this OGAR PR merged first.
