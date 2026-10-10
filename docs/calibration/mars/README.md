# MARS calibration — run the oracle

> **The recipe to re-prove the MARS bijection from scratch.** Anyone
> with this repo + a Python 3 interpreter should be able to reproduce
> every claim in `docs/MARS-TRANSCODING.md`.

## The three levels of bijection

| Level | What it proves | Command |
|---|---|---|
| **1. Same bytes as upstream** | Holds by construction: the tests read `NTO/MARS/` from a checkout of AdaWorldAPI/OGIT at its moving `master`, and OGAR keeps no copy that could drift | none needed |
| **2. XSD-oracle agreement** | The TTL fixed-enum values equal the XSD-extracted classification set (chess-grade, structural-arm only) | `cargo test -p ogar-from-schema ttl::tests::application_class_values_appear_in_xsd_oracle` |
| **3. Semantic round-trip** | Every MARS TTL and every SGO verb survives `parse → emit → re-parse` with equal lifted form | `cargo test -p ogar-from-schema -- ttl_emit::tests sgo::tests::all_sgo_verbs_roundtrip` |

CI runs Levels 2 and 3 against OGIT `master`, checked out beside the
build; locally they read `OGIT_FORK_PATH` or `../OGIT`.

## Regenerate the XSD oracle

```bash
cd vocab/oracles/mars
python3 extract_classes_py3.py -s MARSSchema2015.xsd -F asciidoc > classifications.adoc
python3 extract_classes_py3.py -s MARSSchema2015.xsd -F html > classifications.html
```

Both outputs should be byte-equal to what's committed apart from the
`:revdate:` line, which carries the run date. The `extract_classes.py`
file is the literal Python 2 script from `arago/MARS-Schema/tools/`;
`extract_classes_py3.py` is its mechanical `2to3-3.11 -w -n` conversion —
see `vocab/oracles/mars/PROVENANCE.md` for the conversion provenance.

## Extracted taxonomy at MARS Schema 5.3.8

| Section | Classes | (class, subclass) pairs |
|---|--:|--:|
| Application | 7 | 50 |
| Resource | 19 | — (2-col, no subclass) |
| Software | 40 | 336 |
| Machine | 11 | — (2-col, no subclass) |

The TTL `ogit:validation-parameter` strings in
`Application/attributes/{class,subClass}.ttl` and
`Software/attributes/{class,subClass}.ttl` carry the same value sets;
the test in `crates/ogar-from-schema/src/ttl.rs` asserts membership of
every TTL value in the XSD oracle output.

## Strengthen to full set-equality (queued)

The current test asserts **every TTL value appears in the XSD oracle**
(one direction of bijection). The reverse — **every XSD oracle value
appears in the TTL** — is the natural strengthening. It catches the
case where the schema admits a classification that the TTL
`validation-parameter` list dropped. Estimated ~30 LOC; queued behind
the XSD-as-second-front-end work in `ogar-from-schema::xsd` (the same
producer that would let us reverse-emit XSD from OGAR `Class`es).

## Cross-references

- `vocab/oracles/mars/PROVENANCE.md` — where each oracle file comes
  from, and how to regenerate it
- `vocab/oracles/mars/MARSSchema2015.xsd` — the XSD oracle (frozen
  since 2015)
- `vocab/oracles/mars/extract_classes.py` — the upstream Py2 script,
  kept as-is
- `vocab/oracles/mars/extract_classes_py3.py` — the Py3 conversion
  (`2to3-3.11 -w -n`, zero hand-edits)
- `crates/ogar-from-schema/src/ogit_checkout.rs` — where the tests find
  the OGIT checkout
- `crates/ogar-from-schema/src/ttl.rs` — the parser + agreement test
- `crates/ogar-from-schema/src/ttl_emit.rs` — the reverse emitter +
  round-trip test
- `crates/ogar-from-schema/src/sgo.rs` — the SGO verb parser + a
  round-trip test over every SGO verb
- `docs/MARS-TRANSCODING.md` — the calibration spec
- `docs/HIRO-IN-CLASSES.md` — the bardioc-efficiency story
- `docs/CHESS-TRANSCODING.md` — the calibration template MARS follows
