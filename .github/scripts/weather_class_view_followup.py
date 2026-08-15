from pathlib import Path


def once(text: str, old: str, new: str, label: str) -> str:
    n = text.count(old)
    if n != 1:
        raise SystemExit(f"{label}: expected exactly one anchor, found {n}")
    return text.replace(old, new, 1)


p = Path("crates/ogar-class-view/src/lib.rs")
s = p.read_text()

s = once(
    s,
    "    visit,\n    vital_sign,\n};\n",
    "    visit,\n    vital_sign,\n    weather_cell,\n    weather_static_cell,\n};\n",
    "weather builder imports",
)

s = once(
    s,
    "        (\"unit_of_measure\", unit_of_measure()),\n"
    "        // ── 0x08XX — OCR (container kinds; content stays in content stores) ──\n",
    "        (\"unit_of_measure\", unit_of_measure()),\n"
    "        // ── 0x04XX — Weather / Atmosphere ──\n"
    "        // These canonical views intentionally carry no W1 payload fields:\n"
    "        // field/level/unit slots are selected by WeatherNext's ClassView\n"
    "        // manifest, not promoted into the shared OGAR schema.\n"
    "        (\"weather_cell\", weather_cell()),\n"
    "        (\"weather_static_cell\", weather_static_cell()),\n"
    "        // ── 0x08XX — OCR (container kinds; content stays in content stores) ──\n",
    "all_canonical_classes weather rows",
)

p.write_text(s)
