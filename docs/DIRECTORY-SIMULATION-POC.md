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
  `ogar-ad` schema v4 adds `physicalDeliveryOfficeName`. The executor
  (lance-graph `validate::address_rules`) computes the three violations.
  **Corrected 2026-10-10:** `mail` is not in that key space. It is a
  property on the user's business card, like the telephone number: shown in
  the address book and used inside messages, kept as written, and it follows
  the user rather than the mailbox or the recipient. It is not identity, not
  an address anything is received at, and not provisioned. A `mail` value
  therefore never makes an object a holder in `AddressConflict` (the admin
  with a password-reset `mail` holds nothing), and `AddressRole::Mail` is
  documented as such. This holds as long as Entra Connect does not use
  `mail` as the source anchor.
- **V9 — `onPremisesExtensionAttributes` is a bag.** Graph returns it as
  one object of fifteen `extensionAttributeN` keys; AD stores fifteen
  separate attributes. Both land in one pooled slot of `AttrKind::Bag`:
  the entry at zero-based offset *i* is `extensionAttribute(i+1)`, an empty
  entry is an absent member, and a bag with no values sets no slot. Member
  names are not stored, so the AD and Graph bags are byte-identical and the
  pair syncs `Same`. Read a member with `ValuePool::bag_member(r, n)`,
  which is 1-based: `n = 1` is `extensionAttribute1`, `n = BAG_LEN` (15)
  is `extensionAttribute15`. Only the canonical member spelling counts
  (`schema::bag_member_of`): `extensionAttribute01` is reported as ignored.
  `objectSid` syncs through `SyncTransform::SecurityIdentifier`, the
  binary-to-`S-1-...` string form (`ogar_dir_core::sid::sid_to_string`).
  `ogar-ad` v5 gathers the bag (and stops reporting its members as
  ignored); `ogar-az` v2 requests it. `onPremisesDistinguishedName` and
  `onPremisesSamAccountName` were already ingested: the DN's OU path
  (leaf CN and DC dropped, root first) is interned into the shared OU
  dictionary, so a synced user's `ou_hhtl` equals its AD object's.
- **V10 — Holding an address is not receiving at it (2026-10-10).** Two
  claim planes, one per attribute kind: the UPN is claimed by every node
  that `is_owner` (present and active, or a recipient type that keeps a
  disabled account a recipient); SMTP proxies and the routing address are
  claimed only by a node that `is_mail_recipient`. Provisioning follows the
  recipient type, never the enabled flag: a shared, room or equipment
  mailbox receives with its account disabled; a node that is not
  mail-enabled holds no SMTP address, whatever `proxyAddresses` still
  carries (leftovers after `Disable-RemoteMailbox` claim nothing; its UPN
  still does). `validate::address_owner` answers who holds an address;
  `address_recipient` who receives at it.
- **V11 — Exchange identity (2026-10-10).** Read by the object's GUID, never
  by an address (`ExchangeIdentity { node, recipient, exchange_guid,
  primary_smtp }`):
  - `ExchangeGuid` is the mailbox's immutable identity. On-premises it is
    `msExchMailboxGuid`, ingested by `ogar-ad` schema v6 into a `Guid`
    slot; a record before v6 has not read it.
  - `PrimarySmtpAddress` identifies the recipient implicitly, as a mutable
    string.
  - `ExternalDirectoryObjectId` is the link mailbox ↔ Entra user ("external"
    = the directory outside Exchange). That Entra user (formerly MsolUser)
    carries `{alias}@{tenant}.onmicrosoft.com`, distinct from the routing
    address `{alias}@{tenant}.mail.onmicrosoft.com`, the external EOP
    target (`exchange::ROUTING`).
- **V12 — The cloud fold and the `ExchangeGuid` comparison (2026-10-10).**
  This closes V7's open half on the lance-graph side. `CloudMailboxes`
  reads `correspond::fold` on GUIDs only (source anchor, backsync, Entra
  id) and keeps the AD objects with exactly one Exchange Online mailbox;
  an ambiguous match is no mailbox. `delivers_to` / `address_recipient_in`
  then decide receipt with the cloud observed: a remote mailbox receives
  only when its cloud mailbox exists. The mailbox identity is compared on
  both sides (`mailbox_guid`):
  - `Enable-RemoteMailbox` leaves `msExchMailboxGuid` **empty**. Entra
    Connect writes it back only with its *Exchange hybrid deployment*
    option checked; otherwise it stays empty until set by hand
    (`Set-RemoteMailbox -ExchangeGuid`). Empty = awaiting backsync:
    delivered, not migratable.
  - Equal = migratable. This is the only migratable state.
  - Different = the on-premises value blocks provisioning the cloud
    mailbox: not delivered, not migratable.
  - Cloud value unread, or read with two values for one
    `ExternalDirectoryObjectId` = no verdict.
  **Open:** the Exchange hybrid option is not observed, so an empty value
  reads as awaiting backsync even where nothing will fill it; that needs
  the option as an input. The cloud `ExchangeGuid` values are supplied by
  the caller (`with_exchange_guids`); there is no Exchange Online encoder
  or ingest yet.
- **V13 — Parked until EOP is modelled.** The edge verdict per accepted
  domain (Authoritative: a non-existent address is rejected outright;
  InternalRelay: relayed) and how leftover proxies on non-mail-enabled
  accounts look at the edge. Without EOP neither changes a decision here.
- **V14 — Owner gate across Entra (open).** Entra enforces UPN and proxy
  uniqueness across all objects, Exchange only across mail-enabled ones;
  whether the dir-sim owner gate should follow the wider Entra scope is
  undecided.
- **V15 — Where mailboxes live (2026-10-10).** `Deployment` names the
  on-premises host (`MailboxHost::ExchangeServer` or `Stalwart`, or none)
  and whether Exchange Online is present; the constants are
  `EXCHANGE_SERVER`, `ONLINE`, `EXCHANGE_HYBRID`, `STALWART` and
  `STALWART_WITH_ONLINE`. What each reads as authoritative:
  - an on-premises host: the AD recipient attributes decide;
  - Exchange Online: the cloud mailbox decides delivery to a cloud-hosted
    recipient; without an on-premises host a user is mail-enabled when it
    has a cloud mailbox, typed by `MailboxPurpose` (Graph's
    `mailboxSettings.userPurpose`), and `msExch*` is not read;
  - the `ExchangeGuid` is compared (V12) only for Exchange Server with
    Exchange Online: Stalwart has no `ExchangeGuid`.
  `Deployment::host_of` says where a recipient's mailbox is: an
  on-premises mailbox on the on-premises host, a remote mailbox that is not
  deprovisioned in Exchange Online, and nothing for anything else or for a
  host the deployment lacks. That is the question a mail server answers
  before it accepts a message as local or relays it.
  These are provided and documented as an API only; the routing is used
  later by HubSPO-rs and by SAP on Quack (`lance-graph-sap`), which bind
  to it in their own repos.
  `MailboxLocation { host, kind, mailbox_guid }` carries what
  `Get-MailboxLocation` reports; `MailboxLocationType` is its seven values.
  Exchange names a location `TenantGUID\MailboxGUID`, the pair Graph
  spells `MBX:{MailboxGUID}@{TenantGUID}` (`DIRECTORY-ADAPTERS-POC.md`
  §10). `MailboxPurpose` maps user / shared / room / equipment onto
  `RemoteKind`; linked and `others` have no remote kind.
  **Open:**
  - lance-graph does not consume the deployment yet; the simulator still
    behaves as `EXCHANGE_HYBRID`.
  - Stalwart's DirSim backend answers `Account` for every recipient that
    receives, including a remote mailbox whose mailbox is in Exchange
    Online. With `STALWART_WITH_ONLINE` it should answer `Account` only
    where `host_of` is `Stalwart`, and route the rest to Exchange Online.
  - OneDrive as a delivery location for ERP and ticket tools (invoices,
    reports), executed by Spear. Unattended delivery addresses the drive by
    the directory user's Entra id (`/users/{id}/drive/items/{parent}:/{file}:/content`),
    app-only with `Files.ReadWrite.All` up to 250 MB; larger files take an
    upload session, app-only with `Sites.ReadWrite.All`. The default
    conflict behaviour is `fail`, so an invoice is never silently replaced.
    `ogar_az::mailbox::user_drive` (`GET /users/{id}/drive`, delegated
    only) is for an interactive caller that wants the drive id. The
    directory's part is the Entra id of the recipient, from the
    correspondence fold.
- **V16 — Active Directory emulation (2026-10-10).** dir-sim serves two
  uses with one surface: a mirror of an on-premises AD for emulation, and an
  AD-shaped view of a tenant read from the cloud, presented as an
  IAM-managed directory. Decisions:
  - **Placement.** A user synchronized from AD (Graph reports
    `onPremisesDistinguishedName`) is mirrored at that DN's OU path,
    interned into the on-premises domain's dictionary, so it sits exactly
    where its AD object sits. A cloud-only user has no on-premises location
    and is placed under one marked container, `Cloud Only (emulated)`, and
    reported as synthetic. A real OU in or under that container is refused,
    and so is a reported DN that did not encode: a synchronized user is
    never demoted to the synthetic container.
  - **Surfaces.** The in-process API, an LDIF export, and a read-only LDAP
    server (bind, search, rootDSE). Every result is filtered by the
    caller's IAM rights, so two actors can see different directories. The
    LDAP server never writes: a change goes through simulate, validate and
    plan, and the plan is applied by the operator's own tooling.
  - **Marking.** Every emitted object says whether it was observed in AD,
    read from the cloud and mirrored, or placed synthetically.

  Phases:
  1. **Cloud ingest (lance-graph `observe::from_graph`).** Entra users from
     `ogar-az` records become an observation in the tenant's scope.
     `accountEnabled` gives the flag (absent is unknown). Graph reports no
     Exchange recipient attributes and no `msExchMailboxGuid`, so neither
     is read here; the cloud mailbox comes from the correspondence fold
     (V12).
  2. **Projection and LDIF.** Any version (observed, simulated, cloud-read)
     projected as AD entries: DN from the OU dictionary, the attributes the
     observation carries, the marking above. LDIF for export and diff.
  3. **Read-only LDAP.** Simple bind against the actor source, search over
     the projection with base, one-level and subtree scope, rootDSE naming
     the emulated domain. Results pass the RBAC plug before they are
     encoded; refused objects are absent, not redacted.

  **Status (2026-10-10):** phase 1 is `observe::from_graph` (lance-graph
  #1450), phase 2 is `ad::project` + `ad::to_ldif` (#1452), phase 3 is
  `ldap::Server` (#1453). Each entry carries `dirSimOrigin` (`observed`,
  `mirrored`, `synthetic`, `simulated`); a node's RDN is `CN=<objectGUID>`
  because no display name is read. The LDAP handler takes bytes and
  returns bytes, so dir-sim stays free of I/O; access is the host's
  `Authority` (bind, entry visibility, attribute readability), and a
  hidden entry answers `noSuchObject` as a search base.
  **Open:** groups and memberships from Graph (phase 1 reads users only);
  display names (the RDN is the GUID until one is read); the host binary
  that runs the LDAP listener and plugs `Authority` into the RBAC plug;
  objects under a non-OU container such as `CN=Users`, which the OU path
  places at the domain root;
  a synthetic DN for the cloud-only user that a real LDAP client can bind
  as; whether the emulated domain name is the on-premises one or the
  tenant's initial domain when there is no hybrid.
- **V17 — Mailbox delegation (design, 2026-10-10).** Exchange grants three
  rights on a recipient to another principal (the trustee). They answer
  different questions and none implies another:
  - **FullAccess**: the trustee opens and reads the mailbox as its owner.
    A mailbox permission (`Add-MailboxPermission`); on-premises in the
    mailbox security descriptor. `msExchDelegateListLink` on the mailbox
    lists only the FullAccess trustees that are automapped, so it is a
    witness of some grants, not of all.
  - **SendAs**: the trustee sends with the recipient's address as From and
    leaves no trace of itself. An Active Directory extended right on the
    recipient object (`Send-As`, rights GUID
    `ab721a54-1e2f-11d0-9819-00aa0040529b`; `Add-RecipientPermission` in
    Exchange Online).
  - **SendOnBehalf**: From is the recipient and Sender is the trustee.
    `publicDelegates` on the recipient (`GrantSendOnBehalfTo`).

  Decisions:
  - **A grant is a relation, not a property.** `(object, trustee, right)`
    is its own record, like a membership, and changes by add and remove;
    no node carries a list of trustees.
  - **The object.** FullAccess needs a mailbox; a grant of it on anything
    else is a violation. SendAs and SendOnBehalf also apply to a
    distribution group, which has no mailbox of its own.
  - **Which trustees each right accepts.** Exchange does not accept the
    same group kinds for every right (Microsoft, "Manage permissions for
    recipients"), so the group's kind (V19) is validated per right:
    - **FullAccess and SendAs**: a user or a security-enabled group. A
      user holds the right when it is granted to the user or to a
      security-enabled group it is in, directly or through nesting (V19:
      the user's groups, filtered to security-enabled). Exchange Online resolves these trustees among recipients,
      so there the group must also be mail-enabled; on-premises a
      security group that is not mail-enabled is a valid trustee. A grant
      of either to a distribution group is a violation and confers
      nothing.
    - **SendOnBehalf**: a user or a mail-enabled group, distribution
      groups included, because `GrantSendOnBehalfTo` names recipients. A
      user holds it when it is in the group, directly or through nesting
      (V19). A grant to a group that is not mail-enabled is a violation.

    Until V19's expansion lands, a grant to a group reaches only the
    group's direct members.
  - **Denies follow ACE precedence.** Exchange allows deny entries on
    FullAccess, and Active Directory allows them on SendAs. Each entry
    keeps whether it is inherited (`IsInherited` in
    `Get-MailboxPermission` / `Get-EXOMailboxPermission`; the ACE's
    inherited flag on-premises), and entries are evaluated in Windows
    canonical order: explicit deny, explicit allow, inherited deny,
    inherited allow. So a deny removes a right unless an entry of a
    stronger level allows it; an explicit allow outranks an inherited
    deny. A group entry is at the level of its own ACE, not lower for
    being reached through a group. A deny whose level was not reported
    (a witness that carries no inheritance) is treated as explicit, so it
    fails closed, and a deny that cancels an allow is reported rather
    than applied silently. Reading allows without denies would fail open.
  - **Each grant carries the witness that asserted it.** LDIF supplies
    `publicDelegates` and `msExchDelegateListLink`; Graph supplies none of
    the three. Exchange supplies all three, each from its own read,
    through the same read-only encoder as V12: FullAccess from
    `Get-MailboxPermission`, SendAs from `Get-RecipientPermission` (on-
    premises `Get-ADPermission` with the `Send-As` extended right), and
    SendOnBehalf from the `GrantSendOnBehalfTo` property of the
    recipient, read with `Get-Mailbox`, `Get-DistributionGroup` or
    `Get-DynamicDistributionGroup`; it is a property, not a cmdlet of its
    own. Witnesses that disagree both stay; an absent witness is not a
    refusal.
  - **Consumers.** Spear's mailbox IAM lets a FullAccess trustee read the
    mailbox's rows. SendAs and SendOnBehalf gate sending, and the consumer
    checks the right before every submission; Stalwart does not know about
    V17. Stalwart's JMAP submission compares the envelope `mailFrom` with
    the identity's address (`crates/jmap/src/submission/set.rs`,
    `forbiddenFrom`), and an identity may only carry one of its own
    account's addresses (`crates/jmap/src/identity/set.rs`). So:
    - **SendOnBehalf** is sent from the trustee's own account and
      identity, with `From` set to the object and `Sender` to the trustee.
    - **SendAs on a mailbox** cannot be sent from the trustee's account,
      which has no identity with the object's address. It is submitted to
      the object's own account through master-user impersonation
      (`object%master`, `Impersonate` permission), after the V17 check.
    - **SendAs on a distribution group** has no such route: the group has
      no account or mailbox to impersonate. Until a group-sending path is
      defined (open), the consumer refuses that submission rather than
      sending it from some other account.

  Not modelled: folder-level permissions (`Add-MailboxFolderPermission`)
  and calendar delegates.
- **V18 — Document access (design, open, 2026-10-10).** Nothing in Spear,
  dir-sim or Stalwart decides who may read or change a document (HubSPO-rs
  `comms-api-surface.md` §2). SharePoint and OneDrive grant a role (read,
  write, owner) to a principal on a site, library, folder or item. An item
  inherits its parent's grants until inheritance is broken. A sharing link
  has a scope: `anonymous` admits whoever holds it, `organization` admits a
  user signed in to the same tenant who holds it, `users` admits only the
  people it was granted to, and `existingAccess` grants nothing (it lets
  people who already have access find the item). Graph reports a drive item's `permissions`
  with `roles`, `grantedToV2`, `grantedToIdentitiesV2` and `link`
  (`link.scope`). It does not return `inheritedFrom` for SharePoint and
  OneDrive for Business, so the permission list cannot say which ancestor
  supplied a grant or where inheritance breaks.

  Two regimes, kept apart:
  - **Files attached to a record** (a ticket's or a deal's attachments in
    HubSPO-rs) take their access from the record they belong to. They are
    not SharePoint items and get no grants of their own.
  - **Files in a library or a OneDrive** follow the SharePoint model below.

  Proposal for the second regime, not yet decided:
  - A grant is a relation `(resource, principal, role)`, as in V17. The
    resource is a document, folder or drive node with a parent.
  - Inheritance comes from the tree, not from `inheritedFrom`. The folder
    tree is read by walking each item's `parentReference`, and where
    inheritance breaks is read from SharePoint's own flag on each
    securable object (`HasUniqueRoleAssignments` in the SharePoint REST
    API), with its role assignments read at that object. An item without
    unique assignments takes its grants from the nearest ancestor that has
    them. Until that flag is read, the grants on an item are its
    effective permissions as Graph returns them, with no inheritance
    inferred.
  - **The observation must be complete.** Graph returns all of an item's
    sharing permissions only to its owner or to an application with
    site-wide permission; a delegated caller who is not an owner gets
    only the permissions that apply to it. Ingestion therefore reads in
    an owner or application context. A permission set read in any other
    context is marked caller-scoped, and never decides access for anyone
    but that caller.
  - A principal's effective role is the highest role granted to it, or to
    a group it belongs to, on the item or on an ancestor up to the nearest
    broken inheritance.
  - A sharing link is a grant that keeps its scope. Holding the link is
    never enough on its own: `anonymous` admits any holder, `organization`
    admits a holder signed in to the tenant (an external or guest identity
    from another tenant is refused), and `users` admits only the identities
    in `grantedToIdentitiesV2`, which are evaluated as ordinary principals.
    An `existingAccess` link grants nothing: its holder is evaluated on
    the grants they already have. A link is never folded into one shared
    link principal.
  - A grant keeps its `expirationDateTime`, and an expired grant is
    refused at authorization time, not only when the next observation
    drops it.
  - Spear evaluates it with an RBAC plug over the document classid, the
    way `RBAC_PLUG` covers mail.

  Open: whether SharePoint groups are dir-sim groups; "limited access",
  SharePoint's implicit role on an ancestor of a shared item; where the
  folder tree lives (Spear's `DriveScope` names a drive and an item, not a
  path).
- **V19 — Group kinds and nested groups (design, 2026-10-10).** A group
  has two independent properties, and dir-sim records both:
  - **Security-enabled**: the group can hold permissions. AD sets bit
    `0x80000000` of `groupType`; Graph reports `securityEnabled`.
  - **Mail-enabled**: the group has addresses and receives mail. Graph
    reports `mailEnabled`; dir-sim reads it as the group having a primary
    SMTP address (lance-graph `View::is_mail_recipient`), the same rule it
    already applied to distribution lists.

  That gives three kinds in practice: a security group (not mail-enabled),
  a mail-enabled security group, and a distribution group (mail-enabled,
  not security-enabled). A Microsoft 365 group (`groupTypes` contains
  `Unified`) is mail-enabled and may be security-enabled; it has no
  on-premises equivalent unless written back. An unread flag stays
  unknown, never false: a group read from a source that did not report
  `groupType` or `securityEnabled` is not treated as a distribution group.

  Decisions:
  - **Nesting is one global pattern, expanded in lance-graph** (W-4 of
    lance-graph's HubSPO-rs handover), once, not by each consumer. It
    follows every nested group, whatever its kind, in both directions:
    the users in a group (`View::members_transitive`) and the groups a
    user is in (`View::groups_transitive`), which are inverses. Cycles are
    walked once. A group's kind is never a rule for the walk.
  - **The properties are filters, applied to the result.** lance-graph's
    `groups_where` is a SQL-ish `WHERE` over the two properties (`Is`,
    `Not`, `And`, `Or`), lowered to one Quack program over the group
    population. A consumer composes the two:
    - *Mail*: a list addressed by its mail-enabled group reaches every
      user in it through nesting, a nested group without an address
      included. Spear's `mailbox_members` uses this.
    - *Permissions*: a user's groups filtered to security-enabled are the
      SIDs the user inherits; a permission granted to such a group reaches
      its members through nesting. V17, V18 and RBAC roles from group
      membership use this.
    - *Both*: a mail-enabled security group carries a permission and
      receives mail.
  - **Unknown is not security-enabled.** A group whose security flag was
    not read is never in the security-enabled filter, so a permission
    granted to it is not inherited and fails closed. A group created in a
    simulated version has no flag either (a `Change` carries none) until
    `NodeState` grows one.
  - **Validation.** A permission grant to a group that is not
    security-enabled is a violation (V17). A security group whose
    security flag was read as cleared by a later observation loses every
    grant made to it, which the diff reports.

  Not modelled: group scope (global, domain local, universal) and the
  nesting rules between scopes; dynamic membership rules
  (`DynamicMembership`), whose members are read as observed.
- **V5 — CI.** CI builds `lance-graph-dir-sim` against the OGAR checkout, so
  it needs this OGAR PR merged first.
