from pathlib import Path


def once(text: str, old: str, new: str, label: str) -> str:
    n = text.count(old)
    if n != 1:
        raise SystemExit(f"{label}: expected one anchor, found {n}")
    return text.replace(old, new, 1)

p = Path('crates/ogar-vocab/src/lib.rs')
s = p.read_text()
s = once(
    s,
    '    /// Any high-byte slot not yet assigned a domain (`0x04XX`–`0x06XX`,\n'
    '    /// `0x10XX`–`0x16XX`, `0x18XX`+).\n',
    '    /// Any high-byte slot not yet assigned a domain (`0x05XX`–`0x06XX`,\n'
    '    /// `0x10XX`–`0x16XX`, `0x18XX`+).\n',
    'Unassigned domain documentation',
)
p.write_text(s)

dp = Path('docs/APP-CLASS-CODEBOOK-LAYOUT.md')
d = dp.read_text()
d = once(
    d,
    '| `0x0000` | **Shared canonical core** | all (`0x01/02/07/08/09` + `0x0A` anatomy + `0x0B` auth + `0x0C` automation) | n/a (this *is* core) |\n',
    '| `0x0000` | **Shared canonical core** | all (`0x01/02/04/07/08/09` + `0x0A` anatomy + `0x0B` auth + `0x0C` automation) | n/a (this *is* core) |\n',
    'Shared core allocation documentation',
)
dp.write_text(d)
