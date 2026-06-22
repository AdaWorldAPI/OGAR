# MARS calibration — run the oracle

> **The recipe to re-prove the MARS bijection from scratch.** Anyone
> with this repo + a Python 3 interpreter should be able to reproduce
> every claim in `docs/MARS-TRANSCODING.md`.

## The three levels of bijection

| Level | What it proves | Command |
|---|---|---|
| **1. Byte equality vs upstream** | The `vocab/imports/ogit/NTO/MARS/` mirror is bit-identical to `AdaWorldAPI/OGIT/NTO/MARS/` at the SHA in `PROVENANCE.md` | `diff -qr vocab/imports/ogit/NTO/MARS/ /path/to/OGIT/NTO/MARS/ \| grep -v '^Only in vocab.*: \(PROVENANCE\|_oracle\)$'` |
| **2. XSD-oracle agreement** | The TTL fixed-enum values equal the XSD-extracted classification set (chess-grade, structural-arm only) | `cargo test -p ogar-from-schema ttl::tests::application_class_values_appear_in_xsd_oracle` |
| **3. Semantic round-trip** | Every MARS TTL and every SGO verb survives `parse → emit → re-parse` with equal lifted form | `cargo test -p ogar-from-schema -- ttl_emit::tests sgo::tests::all_sgo_verbs_roundtrip` |

All three pass at the pinned SHA. CI runs Level 2 + Level 3; Level 1
requires a local OGIT checkout and is documented for manual runs.

## Regenerate the XSD oracle

```bash
cd vocab/imports/ogit/NTO/MARS/_oracle
python3 extract_classes_py3.py -s MARSSchema2015.xsd -F asciidoc > classifications.adoc
python3 extract_classes_py3.py -s MARSSchema2015.xsd -F html > classifications.html
```

Both outputs should be byte-equal to what's committed (the script is
deterministic). The `extract_classes.py` file is the literal Python 2
script from `arago/MARS-Schema/tools/`; `extract_classes_py3.py` is its
mechanical `2to3-3.11 -w -n` conversion — see `PROVENANCE.md` for the
conversion provenance.

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

- `vocab/imports/ogit/NTO/MARS/PROVENANCE.md` — the SHA + the
  re-vendor recipe
- `vocab/imports/ogit/NTO/MARS/_oracle/MARSSchema2015.xsd` — the XSD
  oracle (frozen since 2015)
- `vocab/imports/ogit/NTO/MARS/_oracle/extract_classes.py` — the
  upstream Py2 script, vendored as-is
- `vocab/imports/ogit/NTO/MARS/_oracle/extract_classes_py3.py` — the
  Py3 conversion (`2to3-3.11 -w -n`, zero hand-edits)
- `crates/ogar-from-schema/src/ttl.rs` — the parser + agreement test
- `crates/ogar-from-schema/src/ttl_emit.rs` — the reverse emitter +
  round-trip test
- `crates/ogar-from-schema/src/sgo.rs` — the SGO verb parser + 176-verb
  round-trip test
- `docs/MARS-TRANSCODING.md` — the calibration spec
- `docs/HIRO-IN-CLASSES.md` — the bardioc-efficiency story
- `docs/CHESS-TRANSCODING.md` — the calibration template MARS follows
