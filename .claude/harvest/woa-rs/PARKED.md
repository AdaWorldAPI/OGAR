# PARKED — belongs in `AdaWorldAPI/woa-rs`

This directory is the ruff → OGAR transcode of `AdaWorldAPI/WoA`'s
Flask-SQLAlchemy `models.py` (139 `db.Model` classes) into the OGAR V3
sink-in substrate. Its real home is `AdaWorldAPI/woa-rs` (the Rust port
repo this session's WoA-side context — `/home/user/WoA/.claude/CLAUDE.md`
§4 "Zwei-Welten-Pattern" — names as the sibling repo, read-only from the
WoA checkout: `../woa-rs/`).

It is parked here **only** because `woa-rs` is not in this session's write
scope — this OGAR session has no clone/checkout of `AdaWorldAPI/woa-rs` and
no registered write access to it (same shape of gap `openstreetmap-website-rs`
hit in OGAR PR #152: the target repo isn't reachable/writable from this
session, so the artifacts are committed here — an in-scope, writable repo —
as the durable save, rather than lost to an ephemeral scratch directory).

## Relocation (when `woa-rs` is in scope)

```sh
git clone <proxy>/AdaWorldAPI/woa-rs /tmp/woa-rs
cp -r .claude/harvest/woa-rs/. /tmp/woa-rs/<target-subpath>/   # excluding this PARKED.md
cd /tmp/woa-rs && git add -A && git commit && git push
```

The natural landing spot is a `python/` (or `generated/`) subtree mirroring
the `osm-website-rs` precedent's `python/osm/models.py` placement — consult
`woa-rs`'s own `CLAUDE.md`/`board/Stand.md` (per `WoA`'s `CLAUDE.md` §2
"Im Rust-Port arbeitest" row) for its actual layout convention before
copying in, since this session cannot read that repo to confirm.

## What's in this harvest

See `HARVEST.md` (same directory) for the full metrics, provenance, gates,
and — importantly — the honestly-documented gaps (behavior is names-only,
`models_shop.py`'s 12 classes are out of scope, `emit_python`'s import list
is incomplete, and others). Read `HARVEST.md` before relying on `models.py`
here as anything more than a structural (schema-stratum) reality check.

## Provenance

| Input | Pin |
|---|---|
| WoA (`AdaWorldAPI/WoA`, `models.py`, READ-ONLY) | `4427b3d715d841290ea9108e3bf94b22d0cb72c2` |
| ruff (`AdaWorldAPI/ruff`, `ruff_sqlalchemy_spo`) | `/tmp/wt-gr`, branch `claude/spo-python-main`, `66db5c417eddf6017e924706031a23b019c17e81` |
| OGAR (`ogar-from-ruff` / `ogar-vocab` / `ogar-adapter-postgres-ddl`) | `/workspace/ogar`, branch `claude/v3-criticals-woa-parity`, `d5479ce382136b3aedad3c122e13d3aab6f9695e` |

Regenerate with the scratch driver documented in `HARVEST.md`'s
"Reproduction" section (not itself committed anywhere, per the mission's
leitplanken — path-deps + a local-only `[patch]` block).
