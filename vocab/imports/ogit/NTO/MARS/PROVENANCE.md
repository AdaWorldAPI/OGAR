# PROVENANCE — `vocab/imports/ogit/NTO/MARS`

> Literal byte-mirror of `AdaWorldAPI/OGIT @ NTO/MARS/`. **Bijective:**
> every file in this directory (except `_oracle/` and this file) is
> `diff -q`-equal to its origin. Re-vendor by re-running the copy and
> bumping the SHA below; never hand-edit.

## Source

| Field | Value |
|---|---|
| Upstream | `AdaWorldAPI/OGIT` (fork of `arago/OGIT`) |
| Path | `NTO/MARS/` |
| Commit SHA | `d0f489fff94640fef1e6abe7eacba90a1a144579` |
| Commit date | `2026-05-30 08:22:13 +0200` |
| License | MIT (Almato AI GmbH, 2013–2024) — see `OGIT/LICENSE.md` upstream |

## What's mirrored

29 TTL files + 2 markdown files, totalling the entire NTO/MARS subtree:

```
entities/         Application.ttl  Machine.ttl  Resource.ttl  Software.ttl
Application/attributes/   class.ttl  subClass.ttl
Machine/attributes/       class.ttl  cpuArch.ttl  cpuCores.ttl  distroName.ttl  kernel.ttl  ram.ttl
Network/attributes/       bindAddress.ttl  defaultGateway.ttl  fqdn.ttl  interfaceIP.ttl  interfaceMAC.ttl
                          interfaceName.ttl  interfacePrefixLength.ttl  ipVersion.ttl  port.ttl  protocol.ttl
Resource/attributes/      class.ttl
Software/attributes/      class.ttl  installPath.ttl  instanceId.ttl  logPath.ttl  serviceName.ttl  subClass.ttl
README.md                 — upstream README explaining the MARS NTO
AttributeMapping.md       — upstream XSD-to-TTL field-by-field mapping
```

## What's added (`_oracle/`)

The MARS XSD oracle from `arago/MARS-Schema @ master`:

| File | Source | Bytes |
|---|---|---|
| `MARSSchema2015.xsd` | `arago/MARS-Schema/schemas/MARSSchema2015.xsd` | 283 695 |
| `extract_classes.py` | `arago/MARS-Schema/tools/extract_classes.py` (Python 2, **as-is**) | 14 504 |
| `extract_classes_py3.py` | `2to3-3.11 -w -n extract_classes.py` (mechanical conversion only, zero hand-edits) | 14 504 |
| `classifications.adoc` | `python3 extract_classes_py3.py -s MARSSchema2015.xsd -F asciidoc` | 628 lines |
| `classifications.html` | `python3 extract_classes_py3.py -s MARSSchema2015.xsd -F html` | 604 lines |

**The oracle test:** `extract_classes.py` walks the XSD and enumerates every
`(class, subclass)` pair for Application/Software and every `class` for
Resource/Machine. The OGIT MARS TTL files (`Application/attributes/class.ttl`
etc.) carry the **same** classification values in `ogit:validation-parameter`.
The OGAR producer ingests both, and the per-entity sets must agree. That
agreement is the chess-grade bijection oracle — same shape as
`shakmaty::Position::play` for chess (`CHESS-TRANSCODING.md §0`).

Extracted taxonomy (MARS Schema 5.3.8):

| Section | Classes | (class, subclass) pairs |
|---|--:|--:|
| Application | 7 | 50 |
| Resource | 19 | — (2-col, no subclass) |
| Software | 40 | 336 |
| Machine | 11 | — (2-col, no subclass) |

## Re-vendor

```bash
# from OGAR repo root, with /home/user/OGIT checked out at desired SHA:
cp -r /home/user/OGIT/NTO/MARS/. vocab/imports/ogit/NTO/MARS/
# (keeps _oracle/ and PROVENANCE.md intact since OGIT has no _oracle/ folder)
# update the SHA + date in this file
```

## Round-trip bijection (mechanically enforced)

The bijection is exercised by `crates/ogar-from-schema` tests:

| Test | What it proves |
|---|---|
| `ttl::tests::application_class_values_appear_in_xsd_oracle` | TTL fixed-enum values agree with XSD-extracted classifications |
| `ttl_emit::tests::all_mars_ttl_files_roundtrip` | Every MARS TTL parses, emits, and re-parses to an **equal** lifted form (semantic bijection) |
| `sgo::tests::all_sgo_verbs_roundtrip` | Every one of 176 SGO verbs (the AST predicate vocabulary the `ogit:allowed` blocks reference) round-trips |

The round-trip is **semantic**, not byte-equal: whitespace, comments,
and `@prefix` declaration order are not preserved (and should not be —
they're not load-bearing for the structural arm). What the contract
guarantees: `parse(emit(parse(src))) == parse(src)` for every predicate
the OGIT dialect uses.

## Why mirror, don't symlink

- The OGIT clone is not guaranteed to exist on every contributor's machine.
- The producer (`ogar-from-schema`) needs to compile against the TTL set as
  test fixtures, not as an external dependency.
- The byte-equality check (above) is the contract — symlinks would hide drift.
- Re-vendor is cheap (548 KB total) and explicit.
