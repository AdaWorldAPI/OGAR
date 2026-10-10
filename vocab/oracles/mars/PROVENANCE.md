# PROVENANCE — `vocab/oracles/mars`

> The MARS XSD oracle. These files are OGAR's, not OGIT's: they sat in the
> old vendored mirror at `vocab/imports/ogit/NTO/MARS/_oracle/` until
> 2026-10-10, when OGAR stopped vendoring OGIT. The MARS TTLs they are
> checked against are read from a checkout of AdaWorldAPI/OGIT at its
> moving `master` (`crates/ogar-from-schema/src/ogit_checkout.rs`).

## Files

The MARS XSD oracle from `arago/MARS-Schema @ master`, sizes measured
2026-10-10:

| File | Source | Size |
|---|---|---|
| `MARSSchema2015.xsd` | `arago/MARS-Schema/schemas/MARSSchema2015.xsd` | 283 695 bytes |
| `extract_classes.py` | `arago/MARS-Schema/tools/extract_classes.py` (Python 2, **as-is**) | 14 504 bytes |
| `extract_classes_py3.py` | `2to3-3.11 -w -n extract_classes.py` (mechanical conversion only, zero hand-edits) | 14 519 bytes |
| `classifications.adoc` | `python3 extract_classes_py3.py -s MARSSchema2015.xsd -F asciidoc` | 628 lines |
| `classifications.html` | `python3 extract_classes_py3.py -s MARSSchema2015.xsd -F html` | 604 lines |

## Regenerate

```bash
cd vocab/oracles/mars
python3 extract_classes_py3.py -s MARSSchema2015.xsd -F asciidoc > classifications.adoc
python3 extract_classes_py3.py -s MARSSchema2015.xsd -F html > classifications.html
```

The output is byte-equal to what is committed apart from the
`:revdate:` line, which carries the run date. The committed files carry
`22-Jun-2026`, and `xsd::tests::asciidoc_matches_python_oracle` passes
that same date to the Rust transcode.

## What the oracle checks

`extract_classes.py` walks the XSD and enumerates every
`(class, subclass)` pair for Application/Software and every `class` for
Resource/Machine. The OGIT MARS TTLs (`NTO/MARS/Application/attributes/class.ttl`
etc.) carry the same classification values in `ogit:validation-parameter`.
`ttl::tests::application_class_values_appear_in_xsd_oracle` and, with
the `xsd` feature, `xsd::tests::xsd_classes_match_ttl_enum` hold the two
sides to agreement. Extracted taxonomy (MARS Schema 5.3.8):

| Section | Classes | (class, subclass) pairs |
|---|--:|--:|
| Application | 7 | 50 |
| Resource | 19 | — (2-col, no subclass) |
| Software | 40 | 336 |
| Machine | 11 | — (2-col, no subclass) |
