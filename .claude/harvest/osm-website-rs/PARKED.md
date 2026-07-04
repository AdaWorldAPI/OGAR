# PARKED — belongs in `AdaWorldAPI/openstreetmap-website-rs`

This tree is the ruff → OGAR transcode of `openstreetmap/openstreetmap-website`
(Rails). Its real home is the empty repo **`AdaWorldAPI/openstreetmap-website-rs`**.

It is parked here **only** because that repo is not in this session's write
allowlist: `git clone`/`fetch` of it succeed, but `git push` returns `403`
(the GUI "add repository to current session" did not update the proxy
allowlist — a backend bug). Pushing to an in-scope repo (OGAR) is the durable
save until the target repo becomes writable.

## Relocation (when `openstreetmap-website-rs` is in scope)

```sh
git clone <proxy>/AdaWorldAPI/openstreetmap-website-rs /tmp/osm-rs
cp -r harvest/osm-website-rs/. /tmp/osm-rs/     # (excluding this PARKED.md)
cd /tmp/osm-rs && git add -A && git commit && git push
```

## Provenance

| Input | Pin |
|---|---|
| OSM source (`openstreetmap/openstreetmap-website`) | `173885c17d91c4a2ceb70f7a4e911f2b250628ef` |
| ruff (`AdaWorldAPI/ruff`) | `61ce2b490fc3c432d36c44eceed08125f838b405` |
| OGAR | `4037e88` |

Regenerate the IR with the harvest driver committed alongside:
`cargo run -p ogar-from-rails --example harvest_osm -- <osm-website-root> --ir`.
