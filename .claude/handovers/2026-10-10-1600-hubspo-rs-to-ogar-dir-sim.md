# Handover: HubSPO-rs → OGAR dir-sim (meaning half) — wishlist (2026-10-10)

> APPEND-ONLY. Consumer-side wishlist from HubSPO-rs (the CRM consumer on
> `HubSpoPort` `0x000B`). Read against OGAR `173de21`. The execution half of
> each item is in the sibling lance-graph handover
> `lance-graph/.claude/handovers/2026-10-10-1600-hubspo-rs-to-lance-graph.md`.
> Nothing here asks for a concept mint.

## Why HubSPO-rs needs these

HubSPO-rs answers "who is this person, which team, which mailbox" from
dir-sim, never from a table of its own (HubSPO-rs plan H-ADR-9). Three
HubSpot abilities hit gaps in dir-sim's vocabulary:

| HubSpot ability | needs |
|---|---|
| an assistant sends email as / on behalf of a rep; a rep works a colleague's inbox | who may act for which mailbox (W-1) |
| a team member reads the team's records and inbox | directory groups reaching RBAC roles (W-2) |
| restart without re-observing AD; roll a directory change back | a persisted version (W-3) |

Source read for this list (all `173de21`): `crates/ogar-dir-sim/src/change.rs`
(the `Change` vocabulary: memberships, nodes, attributes, active, location,
recipient — no delegation), `crates/ogar-dir-sim/src/provenance.rs:16`
(`VersionId` "designed to map 1:1 onto a Lance dataset version once
persisted"), `crates/ogar-rbac/src/lib.rs:276` (`IdentityActors`, the only
`ActorSource` in OGAR, built from an authenticated user, not a directory).
A search for `send.?as|on.?behalf|delegat|FullAccess|publicDelegates` over
`ogar-dir-core`, `ogar-dir-sim`, `ogar-ad`, `ogar-az`, `ogar-rbac` hits only
`ogar_az::mailbox::Grant::Delegated` (an OAuth grant type, not a mailbox
permission).

## The wishlist (consumer priority order)

**W-1 — Mailbox delegation as a directory relation.**
- A right type: `MailboxRight { FullAccess, SendAs, SendOnBehalf }`.
- A relation `(delegate: Guid128, mailbox owner: Guid128, right)`,
  observed like a membership and carried by `Observation` the same way.
- `Change::{GrantMailboxRight, RevokeMailboxRight}`, with the matching plan
  `Operation`s, so a delegation change is simulated, diffed and planned like
  any other.
- `Violation`s, at least: the owner is not a mail recipient; the delegate
  does not resolve; a group as owner (a group has no mailbox of its own).
- Observation sources, stated as candidates for you to confirm:
  - `SendOnBehalf`: `publicDelegates` on the owner.
  - `FullAccess`: a mailbox permission held in the mailbox security
    descriptor. `msExchDelegateListLink` lists only the automapped subset,
    so it is a hint, not the grant.
  - `SendAs`: an extended right in the owner object's security descriptor.
    It is not a plain attribute, so the caller supplies it as observed
    relations, the way `observe.rs` already takes memberships.
- Mailboxes in the cloud: the same three rights from the Exchange Online
  side; no cloud read is asked for here.

**W-2 — Directory groups as RBAC actors (meaning).**
- What HubSPO-rs needs: an `ActorSource` whose roles come from group
  membership in a dir-sim version. spear's `DirectoryActors` does this for
  the mailbox-owner role only, inside spear.
- The meaning question for OGAR: is "group G grants role R" a directory
  fact or RBAC data?
  - Our suggestion: RBAC data, `GrantSource`-shaped, keyed by the group's
    `Guid128`. The directory then supplies membership only.
- The executable `ActorSource` over a `View` is lance-graph W-2.

**W-3 — The persisted form of a version.**
- `VersionId` maps 1:1 to a Lance version by design (`provenance.rs:16`);
  the persisted form itself is not defined.
- What HubSPO-rs needs from it:
  - a version that survives a restart;
  - provenance and tags kept with it;
  - restoring a version reverting a directory change.
- Today's LDIF projection (lance-graph #1452) is export and diff only. The
  storage half is lance-graph W-3.

## What HubSPO-rs does meanwhile

- **Shared inboxes:** team membership stands in for the inbox's members.
  That is enough for reading a shared inbox (HubSPO-rs D-MAIL-5).
- **Send-as / on behalf:** not offered until W-1 exists. HubSPO-rs will not
  keep a second copy of AD's delegation lists.

## Correction (2026-10-10, after OGAR #344 V17)

- **W-1's violation list was too broad.** It named "a group as owner (a
  group has no mailbox of its own)" as a violation for every right. That
  holds for **FullAccess only**. Exchange grants SendAs and SendOnBehalf on
  distribution groups, so a group is a valid object for those two rights.
  V17 (`docs/DIRECTORY-SIMULATION-POC.md`) has it right.
- **Deny entries.** Exchange allows deny entries on FullAccess. A model that
  reads grants only must not apply a grant a witness also denies: drop it,
  or hold it as a violation (review on #344).
- **Transitive group trustees** depend on nested-group expansion, which is
  lance-graph handover W-4.
