# WebMercator ↔ HHTL + helix / cesium / splat3d — the map render mapping

> Grounded against verified code (2026-07-04): ndarray `crates/cesium/src/*`,
> `src/hpc/splat3d/helix_orient.rs`, `src/simd_wasm.rs`; OGAR canon
> `CLAUDE.md` (256×256 centroid tile). `[G]` = in code, `[H]` = design.

## The one binding the OGAR canon already states

OGAR `CLAUDE.md` § "Tier interpretation — 256×256 CENTROID TILE":
> *"domains bind the axes (**OSM: literal x/y**; semantic: PQ subspace pairs);
> the algebra is identical and domain-agnostic."*

So for the Geo domain (`0x0F`), an **HHTL tier IS a 256×256 spatial tile of
(x,y)** — the map pyramid and the semantic cascade are the *same address*
(D-BOTHCASC). WebMercator maps onto HHTL with no new machinery.

## 1. WebMercator z/x/y → HHTL (HEEL/HIP/TWIG) `[H]`

The OSM slippy pyramid is a quadtree: zoom `z`, tile `(x,y)`, `2^z × 2^z` tiles
(`cesium/osm_pbf.rs` `xyz_to_tms_y`; `cesium/esri_crs.rs` WebMercator EPSG:3857).
A quadtree is a cascade — exactly HHTL's three 16-bit tiers:

```
lon/lat --Mercator(3857)--> world (x,y) in [0,1)         (cesium/esri_crs.rs)
        --*2^z, floor-->     tile (tx,ty) at zoom z        (cesium/osm_pbf.rs)
        --Morton interleave--> 48-bit path = HEEL|HIP|TWIG  (3 × 256×256 tiles)
```

- Each tier = 4 nibbles = a 256×256 tile; two axes = the byte-interleaved
  (x,y) — Morton in tile space (canon "one byte per axis per tier").
- `tier = level >> 2` (canon shift, not branch). Coarse zooms → HEEL, mid → HIP,
  fine → TWIG. Beyond 12 native levels: registry resolve / ref-escape (canon).
- **Path distance = 3 tier-table lookups, O(1)** — the same 256×256 LUT the
  canon reuses everywhere. Two tiles' proximity is read without decoding to
  lon/lat.

## 2. helix = the globe-position index (not the display) `[G]` code / `[H]` reuse

`splat3d/helix_orient.rs` is a golden-spiral **spherical-Fibonacci** RVQ on S²
(`codebook(half_angle)`: `y = 1−(1−ymin)(n+.5)/K`, `a = n·GOLDEN_ANGLE`). On a
**globe**, positions are points on the WGS84 sphere, so the *same* equal-area
codebook indexes global positions — a metric-safe address that never
materialises lon/lat to compare (`I-VSA-IDENTITIES` discipline). This is the
address layer. WebMercator (a cylindrical projection) is the **display** layer
— cesium's Cartesian side, *not* helix. Keep them distinct: **helix addresses
the sphere; Mercator/cesium materialises the plane.**

## 3. cesium = the WebMercator + tile I/O half `[G]`

Already in `ndarray/crates/cesium` (the parity oracle, slated to move to
`lance-graph/crates/cesium`): `osm_pbf.rs` (OSM PBF reader + slippy XYZ↔TMS),
`esri_crs.rs` (WebMercator 3857 inverse, no PROJ), `implicit_tiling.rs` +
`tileset.rs` + `sse.rs` (OGC 3D Tiles pyramid + screen-space-error LOD),
`to_cam_soa.rs` (→ ndarray CAM SoA). The map PoC **consumes** this — it does not
re-implement `osmpbfreader` / `webmercator_tiles`.

## 4. splat3d = the renderer; ndarray-WASM-SIMD + WebGL = the surface `[G]` compute

Render path (the q2 `/helix` pattern — ndarray WASM-SIMD compute, consumer
WebGL surface):

```
OSM PBF --cesium::osm_pbf--> CAM SoA --cesium::to_cam_soa-->
   HHTL-address tiles (§1)  --ndarray::simd_wasm (SIMD projection/raster)-->
      splat3d EWA forward render  --> WebGL draw (per-vertex normalized gather,
                                       NO per-vertex trig — helix §2)
```

- **Compute:** `ndarray/src/simd_wasm.rs` (WASM-SIMD) accelerates the hot loops —
  Mercator projection, tile rasterisation, HHTL path distance. `splat3d` is the
  CPU-SIMD EWA splat forward renderer (Kerbl 2023).
- **Surface:** WebGL, driven from WASM (the `cockpit/BodyHelix.tsx` shape) —
  pre-materialised direction LUT + per-vertex normalized-index gather; the map
  tile is an index, not decoded trig.
- **No GPU fallback:** normalized indices gather on CPU-SIMD too (helix doc).

## What to build (map_renderer PoC), in order

1. `cesium::osm_pbf` → CAM SoA for a small extent (D-OSM-2 wires the real
   `osmpbf` dep; today a stub).
2. Mercator (`esri_crs`) → HHTL address (§1) — the Morton-interleave into
   HEEL/HIP/TWIG, Geo classid `0x0F` prefix scoping the codebook.
3. `splat3d` + `simd_wasm` render to a WASM canvas; WebGL surface per the q2
   `/helix` path.
4. Grounding: OSM elements already have Geo classids (`osm_node 0x0F01` …) — the
   tile address + classid IS the node key.

## Fences (don't dilute)

- Mercator is **display**, helix is **address** — never conflate (§2). A point
  that has been projected to screen `(px,py)` has left the index domain.
- `cesium` is a **parity oracle**, not a production dep yet — the PoC proves the
  path; hardening moves cesium to `lance-graph/crates/cesium`.
- `[H]` items (§1 Morton→HHTL binding, §2 globe-position reuse of `helix_orient`)
  are design — the falsifier is a round-trip: `hhtl(mercator(lonlat))` neighbors
  match `lonlat` neighbors within a tile (the canon's 256=4⁴ nibble-ancestry
  condition, ISS-Q2-CASCADE3-NIBBLE-ANCESTRY).
