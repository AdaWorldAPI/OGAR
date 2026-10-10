# Directory adapters PoC — `ogar-ad` / `ogar-az`

Status: **PoC, 2026-10-03.** Read-only encoding of observed Active Directory
and Entra ID objects into versioned, source-native, fixed-size records. Not an
IAM, not provisioning, not reconciliation, no business rules.

Crates: `ogar-dir-core` (shared ABI, zero deps) · `ogar-ad` (AD DS / LDIF) ·
`ogar-az` (Microsoft Graph). Same footing as `ogar-doc-ir`: neutral tissue,
**no classid mint, no canon dependency.**

## 1. Existing machinery found, and what was reused

| Existing | Where | Used? |
|---|---|---|
| `NodeRow` 512 B = key 16 / edges 16 / value 480 | lance-graph-contract `canonical_node.rs` | **Shape reused**: `DirRecord` keeps the identical split; the directory payload lives in the 480-byte value slab. |
| `NodeGuid` 16 B key (`classid · HEEL · HIP · TWIG · tail`) | same | **Not reused for identity** — see conflict C1. |
| `NiblePath` (16ⁿ nibble router, u64) / HEEL·HIP·TWIG (3×u16) | `hhtl.rs`, `canonical_node.rs` | Not reused — see C2. |
| `ogar-auth` `AuthBinding(provider, subject)` | OGAR `ogar-auth/src/user.rs` | Not touched. It answers "which login maps to which local user"; this PoC answers "what does the directory say". A later bridge may emit `AuthBinding`s from AZ records. |
| `ogar-vocab` Auth domain `0x0B` | OGAR `ogar-vocab` | No mint (C3). |
| `CausalEdge64` | lance-graph-contract | Does not fit: 8 bytes, no 128-bit endpoints. |
| GUID-pair edge record, schema-version machinery, LDAP/Graph code | — | **None exists**; defined minimally here. |

## 2. Crate boundaries

```
ogar-dir-core   Guid128, Dn, OuHhtl + OuDictionary, ValuePool/StrRef,
                SchemaFamily/SchemaId/AttrDef, DirRecord (512 B), DirEdge (64 B)
   ├── ogar-ad  AD SCHEMA_V1, AdKind, LDIF reader, encode(entry)
   └── ogar-az  Graph SCHEMA_V1, $select derivation, ingest_page, sync_edges
```

`ogar-ad` and `ogar-az` do not depend on each other. The AD↔AZ relation is
computed from records (`ogar_az::sync_edges` takes a set of observed AD GUIDs).

## 3. `DirRecord` — exact 512-byte layout

All multi-byte integers little-endian. The record is one `[u8; 512]`
(`#[repr(C, align(64))]`) addressed by constant offsets, so there is no
compiler padding to reason about. Size/alignment/offset sums are `const`
asserts.

```
0x000..0x010  canonical_key    [16]  NodeRow key slot — ZERO in the PoC (dormant default class)
0x010..0x020  canonical_edges  [16]  NodeRow edge-facet slot — ZERO; relations are DirEdge
---- NodeRow value slab (480 B) ----
0x020..0x024  magic            [4]   "OGDR"
0x024..0x026  abi_major        u16   1   (exact match required)
0x026..0x028  abi_minor        u16   0   (additive; readers ignore newer)
0x028..0x02A  schema_family    u16   1 = AD_DS, 2 = MS_GRAPH
0x02A..0x02C  schema_version   u16   encoder's attribute-set version
0x02C..0x02E  object_kind      u16   family-scoped (AD: user/group/computer/contact/OU; AZ: user)
0x02E..0x030  flags            u16   OU_PRESENT | NON_OU_CONTAINER | DN_UNENCODED
0x030..0x040  node_guid        [16]  WHO   objectGUID / Graph id, textual byte order
0x040..0x050  scope_guid       [16]  WHERE AD domain GUID / Entra tenant id
0x050..0x060  ou_hhtl          [16]  WHERE 8 × u16 LE, root first
0x060..0x068  observed_at_ms   i64   WHEN  observation time (provenance hook)
0x068..0x070  presence         u64   bits 0..31 pooled slots, 32..35 numeric slots
0x070..0x080  num              4×u32 numeric/bool slots
0x080..0x180  str_refs         32 × (u32 off, u32 len) into the batch ValuePool
0x180..0x200  reserved         [128] writers zero, readers ignore
```

**GUID byte order:** textual order (the order of the 32 hex digits, RFC 9562
network order). LDAP `objectGUID` and `onPremisesImmutableId` are Microsoft
mixed-endian (`Data1..3` little-endian); `Guid128::from_ms_bytes` converts,
tested against the .NET `ToByteArray()` pair. No `u128`, no truncation API.

**Strings** never live in the record. A `StrRef` points into a per-batch
`ValuePool` (the column payload). Multi-valued slots are `(u32 len, bytes)*`.
Values are raw as observed; nothing is folded or normalised. Derived values
(enabled-from-UAC, canonical SMTP) are functions over records, never stored.

**Stability:** byte offsets, endianness, GUID order and padding are fixed and
asserted. Not yet claimed: a non-Rust reader (offsets are exported in
`record::off` for generating one) and the NodeRow mirror guard (C4).

## 4. OU-HHTL and the OU dictionary

`OuHhtl = [u16; 8]`, root first; `0` = no level; depth = leading non-zero
levels; a gap is rejected on decode; > 8 OUs is an error and the record gets
`DN_UNENCODED` (never a truncated path). Only `OU=` components are levels —
not the leaf, not `DC=`. An OU object's own `OU=` is its leaf.

Segment ids come from an explicit **per-parent dictionary** (`OuDictionary`,
one per AD domain), allocated sequentially from 1 on first sight. No hashing:
collisions are impossible by construction, exhaustion (65535 children of one
OU) is an error, and `explain` reconstructs the original spelling. Matching is
case-insensitive via Unicode lowercase (an approximation of AD collation).

Price of reversibility: ids depend on first-seen order, so the dictionary is
state persisted next to the records (`entries` / `from_entries`, which
rejects duplicate ids or names). Containers that are not OUs (`CN=Users`) do
not enter the HHTL; the record is flagged `NON_OU_CONTAINER` so it is not
confused with "directly under the domain root". **Open question:** whether
such containers should become levels.

For AZ, `onPremisesDistinguishedName` is parsed the same way into the
*on-premises domain's* dictionary. With a shared dictionary the AD record and
the AZ evidence of one OU carry the identical HHTL (tested). For an AZ record
that HHTL is evidence about the on-prem location, not a Graph location.

## 5. Schema / version strategy

Three independent numbers: **ABI version** (how bytes are laid out),
**schema family** (whose semantics), **schema version** (which attribute set
the encoder understood). Each adapter embeds `SCHEMA_V1: &[AttrDef]`
(`name, slot, kind, since`). Evolution is append-only: a new attribute takes
the next free slot with a higher `since`; a slot never changes meaning (retire,
don't reuse). Old records read under a newer schema with the new slots absent;
no ABI change. Graph `$select` is derived from the table (`select_query`).
A `SchemaGuid` was considered and not adopted: OGAR has no such convention and
(family, version) discriminates; reserved bytes allow adding one later.

## 6. Minimal attribute sets

**AD v1:** `distinguishedName, objectSid (bytes), sAMAccountName,
userPrincipalName, objectClass (multi), displayName, givenName, sn, mail,
mailNickname, proxyAddresses (multi), targetAddress, whenCreated, whenChanged`,
numeric `userAccountControl`. Identity `objectGUID`; scope = domain GUID.
Membership attributes are relations (future edges), `msExch*` deferred.

**Graph v1:** `userPrincipalName, displayName, givenName, surname, mail,
mailNickname, proxyAddresses, employeeId, department, companyName,
officeLocation, userType, onPremisesDistinguishedName,
onPremisesSamAccountName, onPremisesDomainName, onPremisesImmutableId,
onPremisesSecurityIdentifier, onPremisesLastSyncDateTime`, bools
`accountEnabled, onPremisesSyncEnabled` (null = absent = unknown, never
false). Identity `id`; scope = tenant id (supplied by the caller — it is not a
user property). Licences, sign-in activity and groups are out of scope.

Unknown attributes are reported (`ignored`) and never stored: they cannot
reach the record or the pool (tested with 200 synthetic unknowns).

## 7. Edges

`DirEdge`, 64 bytes: `kind u16, evidence u16, src_family u16, dst_family u16,
src_guid[16], dst_guid[16]`, rest zero. One kind: `SynchronizesTo` (AD → AZ),
one evidence: `ImmutableIdIsObjectGuid` (the Graph immutable id base64-decodes
to an **observed** AD objectGUID). The two nodes stay distinct; non-GUID
anchors and unobserved targets yield no edge. `MEMBER_OF` / `MANAGER` get
codes when a slice emits them.

## 8. Conflicts with existing OGAR / HHTL contracts

* **C1 — "NodeGuid" means two things.** In OGAR canon the 16-byte `NodeGuid`
  is a *minted address* (`classid` + HHTL tiers + facet) that prerenders a node
  from the key. A directory `objectGUID` is an opaque external identity and
  cannot be placed there without dropping bits. Resolution here: the source id
  is `Guid128` in the value slab; the canonical key slot is reserved zero. A
  future mint of a canonical key for directory nodes (classid + a 12-byte
  facet derived *from* the record) is an operator decision and must not be a
  truncated GUID.
* **C2 — HHTL width.** Canon HHTL is 3 tiers × 16 bits in the key, and the
  contract `NiblePath` is a 16-ary nibble path. The requested OU-HHTL is
  8 × 16-bit levels with per-parent dictionary ids — a different, wider tree
  living in the value slab. It is named `OuHhtl` to keep it from being
  mistaken for the canonical HEEL/HIP/TWIG path.
* **C3 — No classid.** Directory concepts would naturally sit in the Auth
  domain (`0x0B`) of `ogar-vocab`. Minting is a codebook change and was not
  done; `object_kind` is a family-scoped source-native code.
* **C4 — Unguarded mirror.** `DirRecord`'s 16/16/480 split matches `NodeRow`
  by documentation and local asserts only; the crate deliberately does not
  depend on lance-graph-contract. A feature-gated `From<DirRecord> for
  NodeRow` with a size assert would turn it into a real guard.

## 9. Ingest path for a lab tenant (read-only)

```sh
URL=$(cargo run -q -p ogar-az --example az_ingest -- --url)   # schema-derived $select
curl -s -H "Authorization: Bearer $TOKEN" "$URL" > page1.json  # User.Read.All
cargo run -p ogar-az --example az_ingest -- page1.json <tenant-id>
```

The crate performs no HTTP and no writes; follow `next_link` for more pages.
**Not yet run against a real tenant** (no credentials in the build
environment); verified against a synthetic Graph page fixture only. AD side:
`ldapsearch -LLL … objectGUID …` / `ldifde` output through `ogar_ad::ldif::parse`.

## 10. Exchange Online mailboxes through Graph (2026-10-10)

Graph has no mailbox property on `user`; a mailbox takes its own reads,
each with its own permission. `ogar_az::mailbox` names them as `Pull`s,
all `GET` against v1.0, each with the least-privileged application
permission Microsoft documents:

| pull | path | permission |
|---|---|---|
| `MailboxSettings` | `/users/{id}/mailboxSettings` | `MailboxSettings.Read` |
| `ExchangeSettings` | `/users/{id}/settings/exchange` | `User.Read.All` |
| `MailboxFolders` | `/admin/exchange/mailboxes/{mailboxId}/folders` | `MailboxFolder.Read.All` |
| `MailboxItems` | `/admin/exchange/mailboxes/{mailboxId}/folders/{folderId}/items` | `MailboxItem.Read.All` |
| `UserDrive` | `/users/{id}/drive` | `Files.Read.All` (delegated) |

The mailbox reads take an application permission; Microsoft lists reading
a user's drive as delegated only (`Pull::grant`). `user_drive` accepts a
drive only when its `owner.user.id` is the user, and returns its id, which
Spear's drive scope uses to address uploads.

There is no `Mailbox.ReadWrite.All`; the mailbox-content permissions are
`MailboxFolder.*` and `MailboxItem.*`, and the read variants suffice for
everything here. A granted write scope is not used: no pull writes.

`encode_mailbox` turns one user's settings reads into a `DirRecord` of kind
`AzKind::Mailbox` (family `MsGraph`, `MAILBOX_SCHEMA_V1`): WHO = the Graph
user id, which Exchange Online reports as the mailbox's
`ExternalDirectoryObjectId`; WHERE = the tenant; WHAT = `userPurpose` and
`primaryMailboxId` raw, plus `mailboxGuid` in a guid slot.

`primaryMailboxId` is documented only as an opaque identifier. The spelling
observed in tenants is `MBX:{mailbox GUID}@{tenant id}`, and for a primary
mailbox that GUID is its `ExchangeGuid`; Microsoft's own example
(`MBX:e0643f21@a7809c93`) is shortened. `mailbox_guid_of` therefore decodes
only two complete GUIDs whose tenant part equals the record's tenant; any
other spelling is kept raw with no GUID. **Not yet run against a real
tenant**; the reading that the GUID is the `ExchangeGuid` must be confirmed
against `Get-EXOMailbox -Properties ExchangeGuid` before anything compares
it. `examples/mailbox_ingest.rs` prints the pulls and encodes saved bodies.

