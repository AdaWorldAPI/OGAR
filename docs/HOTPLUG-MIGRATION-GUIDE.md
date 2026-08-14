# HOT-PLUG MIGRATION GUIDE — from compile-time fuse to plug-and-play

> Audience: anyone maintaining a cross-repo invariant between OGAR's codebook
> and a consumer-side mirror, table, or registry. Companion to
> `CONSUMER-MIGRATION-HOWTO.md` (which covers *classid-first* migration); this
> one covers *how the two sides stay honest about each other*.
>
> Operator ruling, 2026-08-14: the compile-time fuse is **deprecated**; migrate
> to the `ogar-vocab` + `hotplug.rs` pattern — *"akin to USB plug and play"*.

## The one-paragraph version

A hand-maintained mirror plus a global equality assert is **the opposite of
plug-and-play**: a device that works only if you also patch the host's driver
table by hand, in another repo, in another PR. The assert can only ever
*detect* the omission — never prevent or resolve it. Plug-and-play inverts the
direction: **the device announces, the host enumerates and binds.** A consumer
declares the classids it actually plugs; the authority resolves exactly those
and returns a named drift for the ids that consumer *uses*. A concept nobody
plugs cannot break anybody's build.

## Why this was written — the incident

On 2026-08-14 `osm_street_node` (`0x0F0B`) was minted in OGAR (#268). Its row
in the consumer-side wire mirror lived in a second, unopened PR. The two halves
had been written a minute apart; only one got a PR.

`lance-graph-ogar` carried a `COUNT_FUSE`:

```rust
pub const COUNT_FUSE: () = assert!(
    mirror::CODEBOOK.len() == ogar_vocab::class_ids::ALL.len(),
    "ogar_codebook mirror drifted from ogar_vocab::class_ids::ALL",
);
```

The moment #268 merged, that const-eval panicked (`E0080`) in **every build that
compiled the crate** — and killed a production deploy at compile, for a concept
that deploy never used. The redeploy reproduced it, because Docker had cached
the pre-fix clone of the sibling repo.

Three properties of the fuse made this inevitable, and they generalise to any
guard of this shape:

1. **It asserted a GLOBAL invariant for a LOCAL need.** No consumer needs the
   whole codebook to agree; each needs the handful of ids it uses.
2. **It fired in the wrong place.** The crate is workspace-*excluded*, so the
   fuse never gated the producer's own CI — only every consumer's build. A
   producer-side bookkeeping lapse became a downstream outage.
3. **It could only detect.** Nothing about a length assert helps anyone
   *resolve* the mismatch, and it fails in a repo that cannot fix it.

## The three roles

| Role | Home | Owns |
|---|---|---|
| **Socket** | `lance_graph_contract::hotplug` (zero-dep) | `HotPlug`, `Activation`, `ActivationDrift`, the `CapabilityAuthority` trait |
| **Authority** | OGAR (`ogar_vocab::capability_registry::resolve_hotplug`) | resolving classids → concepts + capabilities, and verifying the registration |
| **Consumer** | its own crate | ONE `HotPlug` const, and a call to `activate` in its binary/tests |

The socket must stay **dependency-free** — a path dep there breaks every CI
cargo invocation at workspace-load time (learned 2026-07-07). That constraint
is why a mirror exists at all, and why it cannot simply be deleted.

## Migrating a consumer — the whole surface

**1. Declare what you plug.** One const, naming the classids and the capability
names your executor covers:

```rust
use lance_graph_contract::hotplug::HotPlug;

const PLUG: HotPlug = HotPlug {
    consumer: "tesseract-ogar",            // crate name by convention
    classids: &[0x0805, 0x0808, 0x0809],   // ONLY what you actually use
    covered: &["recognize_line", "recognize_page", /* … */],
};
```

**2. Activate, in your own binary or tests.**

```rust
use lance_graph_contract::hotplug::CapabilityAuthority;

let activation = lance_graph_ogar::OgarAuthority.activate(&PLUG)?;
```

**3. Read the drift arms as instructions, not as noise.** Each one names a
different mistake:

| Arm | Means | Fix |
|---|---|---|
| `UnknownClassid(id)` | you plugged an id OGAR has not minted | mint it, or stop plugging it |
| `NoCapabilitiesFor(id)` | the id is minted but declares no action | premature plug, or the table was forgotten |
| `UnexpectedConsumer(c)` | you are not an expected executor for a table you resolved | add yourself upstream, or you are plugging someone else's ids |
| `Uncovered(cap)` | the authority declares a capability you did not implement | implement it, or narrow `classids` |
| `Undeclared(cap)` | you claim a capability the authority does not declare | remove it, or declare it upstream |
| `MirrorDrift { .. }` | the authority and the zero-dep wire mirror disagree **about an id you plugged** | update the mirror; this is the fuse's job, now scoped |

`covered` must match the requested classids' action set **exactly** —
`activate` rejects both `Uncovered` and `Undeclared`, so the plug cannot drift
silently in either direction.

**4. Delete your fuse.** Before you do, confirm the two conditions that made it
safe to delete in `lance-graph-ogar`:

- A **runtime full-bijection check already exists** and strictly *contains*
  what the fuse asserted. `assert_codebook_parity` verifies forward, reverse
  and domain agreement; a length equality is a subset of that. If you have no
  such check, write it before removing the fuse — otherwise you are deleting
  detection, not relocating it.
- The fuse's only unique property is **firing during `cargo build`**. Establish
  where that actually fires. If (as here) the crate is excluded from the
  producer's workspace, the answer is "in consumers only", which is the case
  for removal rather than against it.

## Do not re-derive these

- **Prefer the authority over the mirror.** If you can call `activate`, do —
  the mirror is the BBB-safe fallback for consumers that cannot depend on OGAR
  at all. A stale mirror is then a test failure at the authority rather than a
  silent mis-resolution in a consumer.
- **The mirror is still hand-maintained.** Removing it entirely would require
  the zero-dep contract to depend on `ogar-vocab`, which its own docs forbid.
  That is an open design question, not an oversight.
- **A length check cannot see a wrong id.** If a mirror row is present but
  points at the wrong classid, counts still match and a `COUNT_FUSE` is blind.
  `MirrorDrift` carries `mirror_id: Option<u16>` precisely so the two cases —
  *missing* and *wrong* — are distinguishable in the message.
- **Test the checker against a deliberately-wrong table.** With the real mirror
  there is, by construction, no disagreeing concept, so a test using it can
  only assert the happy path and would still pass with the checker deleted.
  `mirror_disagreement(concepts, lookup)` takes the lookup as a parameter for
  exactly this reason; `verify_against_mirror` binds the real one.

## Verifying a migration

Reproduce the incident's shape rather than arguing from the diff. Remove one
row from the mirror and check both halves:

```
cargo build   -> must SUCCEED   (it used to E0080 here)
parity test   -> must FAIL      naming the concept
activate()    -> must FAIL with MirrorDrift, IF that concept is plugged
              -> must SUCCEED,  if it is not — unplugged concepts are inert
```

That last line is the property the fuse could never provide, and it is the one
that would have prevented the outage.

## References

- Socket: `lance-graph/crates/lance-graph-contract/src/hotplug.rs`
- Authority: `ogar-vocab::capability_registry::resolve_hotplug`,
  `lance-graph-ogar::OgarAuthority`
- The migration: lance-graph **#954** (fuse retired; `MirrorDrift` added after
  a codex P2 established that hot-plug alone could not see mirror drift)
- The hotfix that preceded it: lance-graph **#953** (mirror row added by hand)
- The cache half of the same outage: q2 **#130** (sibling clones were served
  from a stale Docker layer, so the first fix appeared not to work)
- Companion: `CONSUMER-MIGRATION-HOWTO.md` (classid-first migration)
