#!/usr/bin/env python3
"""SPEC-5 Part D — usable-dataclass gate.

Imports the `emit_python`-generated module (path given as argv[1]) and
instantiates its `TimesheetActivity` dataclass with dummy field values,
asserting the CLASSID const and field names — proving it's a usable data
model, not just syntactically valid text (py_compile only proves the
latter; this proves the former).

Exit 0 = usable dataclass. Any assertion failure / exception = non-zero
exit, surfaced by the calling Rust test.
"""
import dataclasses
import importlib.util
import sys


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: instantiate_check.py <emitted_module.py>", file=sys.stderr)
        return 2
    module_path = sys.argv[1]

    spec = importlib.util.spec_from_file_location("woa_timesheet_activity", module_path)
    assert spec is not None and spec.loader is not None, "module spec must load"
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    cls = module.TimesheetActivity

    # The rail classid travels as a ClassVar — the WoaPort convergence pin
    # (BILLABLE_WORK_ENTRY 0x0103 | WoA app 0x0003), per the mission report's
    # named SPEC-5 classid deviation (see woa_parity_probe.rs's module doc).
    assert cls.CLASSID == 0x0103_0003, f"CLASSID mismatch: 0x{cls.CLASSID:08X}"

    # Instantiate with dummy values for every declared field (proves it's a
    # real, usable @dataclass — not just syntactically valid text).
    instance = cls(
        id=1,
        beschreibung="dummy timesheet activity",
        created_at=None,
        timesheet=None,
    )

    assert instance.id == 1
    assert instance.beschreibung == "dummy timesheet activity"
    assert instance.created_at is None
    assert instance.timesheet is None

    # Field-name inventory (dataclass field order == emit order: scalars
    # then associations) — proves the shape, not just that *a* instance works.
    field_names = [f.name for f in dataclasses.fields(cls)]
    assert field_names == ["id", "beschreibung", "created_at", "timesheet"], field_names
    assert "timesheet_id" not in field_names, "FK column must stay deduped"

    print(f"OK: {cls.__name__} CLASSID=0x{cls.CLASSID:08X} fields={field_names}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
