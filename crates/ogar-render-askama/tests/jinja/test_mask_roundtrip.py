#!/usr/bin/env python3
"""Falsifier #2, jinja side — same (fields, mask) as the askama-side Rust
test (`tests/mask_dual_target.rs`), read from the SAME fixture file, must
render the SAME field-name set as askama produces.

One ClassView x FieldMask projection, two template engines
(askama/Rust, jinja2/Python), identical result (Operator Sec.5/6;
D-FIELDMASK-LOUD-FAIL companion falsifier, E-ONE-MASK-TWO-ENGINES).

Run:
    uv run --with jinja2 python \
        crates/ogar-render-askama/tests/jinja/test_mask_roundtrip.py
"""
import json
import sys
from pathlib import Path

from jinja2 import Environment, FileSystemLoader

HERE = Path(__file__).resolve().parent
FIXTURE = HERE.parent / "fixtures" / "mask_roundtrip.json"


def main() -> int:
    fixture = json.loads(FIXTURE.read_text())
    fields = fixture["fields"]
    mask = fixture["mask"]
    expected_present = fixture["expected_present"]

    # Pin the fixture is internally self-consistent before trusting the
    # engine output against it (same invariant SPEC-2 states: expected_present[k]
    # == fields[i] for every i where (mask >> i) & 1 == 1, in order).
    recomputed = [f for i, f in enumerate(fields) if (mask >> i) & 1]
    assert recomputed == expected_present, (
        f"fixture is not self-consistent: mask {mask} over {fields} gives "
        f"{recomputed}, but expected_present says {expected_present}"
    )

    env = Environment(loader=FileSystemLoader(str(HERE)))
    template = env.get_template("render_mask.py.j2")
    rendered = template.render(fields=fields, mask=mask)

    present = {line.strip() for line in rendered.splitlines() if line.strip()}

    assert present == set(expected_present), (
        f"jinja render mismatch: got {sorted(present)}, "
        f"expected {sorted(expected_present)}"
    )

    print("OK: jinja field set == askama field set == expected_present:", sorted(present))
    return 0


if __name__ == "__main__":
    sys.exit(main())
