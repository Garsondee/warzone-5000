# Status: GEOMETRY

**Last updated:** 2026-10-10 UTC | **Branch:** lane/geometry/truck (stacked on lane/geometry/build, PR 32, which is stacked on settling, PR 25) | **Contract pinned:** `contract-v0.1` = commit `f8f5e5d` | **Phase:** settling round, continuing into the build (owner instruction in force: do not wait for review)

## Done
- Settling PR: spike S-G (kernel in `crates/w5k_geo/src/{mesh,edge,bvh,cavity}.rs`, tests `tests/spike_g.rs`, report `docs/lanes/geometry/spike-g.md`, image `media/s-g.png`), design note, CCR text (`docs/swarm/requests/geometry-ccr-shapes-and-tracks.md`), theory starter.
- A6 and A7 reproduced as spike tests; BVH agrees with brute force on 2,560 rays.

- Build PR (this branch): mesh kernel (weld, closed check, mirror, slice), exact mass properties, loft with bevel rings, wheels (tyre, lugs, rim, nuts from `shapes/wheel.ron`), flag bake, software rasteriser, `w5k geometry sheet wheel`. Tests: A1, A2, A3, A8 and the closed/mirror/weld checks.

- Truck PR (this branch): `part`, `truck` (data-driven part list `shapes/utility_4x4.ron`), subdivision before baking, panel plates and fittings (C-004), tests A4 (200 random in-range sets), A5 (golden hash `tests/golden/utility_4x4.hash`), A9 (against `box_truck()`), dimension and budget check (placeholder, 19.5k triangles of 40k). Pictures `media/truck-look.png`, `truck-edge.png`.

## In progress
- `w5k geometry export truck` (renderrig JSON + GLB) and the dimension table, then the M113.

## Blocked
- Nothing blocking. Later: FORGE agreement on the part API and datum (design note sections 4 and 6); VALIDATION's M998 dossier (A10; placeholder dimensions until then, `PROVISIONAL(C-002)`); `contract-v0.2` for `ShapeDef` and `track_runs`.

## Next
1. `export` (renderrig JSON + GLB with flags), dimension table, `lines` plan; then the M113 (sloped hull, road wheels, sprocket, idler, track run).

## Cards needed / PROVISIONAL decisions in force
- `PROVISIONAL(status:geometry)`: the edge definition until LOOK agrees. `PROVISIONAL(D3)`: hull datum = hull box centre (FORGE). No new card.

## Evidence
- `cargo test --release -p w5k_geo`: 5 spike tests pass; A7 worst error 0.0033 at 1024 rays; bake 1.6 s / 3.1 s / 11.6 s at 128 / 256 / 1024 rays for 15k vertices (the 1024 figure breaks the 10 s line, so the default is 256).
- fmt, clippy `-D warnings`, constants lint, line budget (223 of 8000), deps lint: clean. Not run: Windows, goldens (none touched), impact matrix (no contract touched).
- Wheel sheet: `docs/lanes/geometry/media/wheel-look.png` (`w5k geometry sheet wheel --out DIR`, 0.5 s). A8 passes at the reference detail (128 segments, sag 0.12 mm).
- Kill criterion "wear over 25% more triangles": hood alone +36%, nominal breach; the whole-vehicle test is at build step 6.
- Found while building the truck: the loft's end caps were fans from a ring vertex, degenerate on the collinear loop points (a sliver filter then opened holes); caps are now centroid fans. Flags need vertices where they vary: parts are subdivided to 0.2 m before baking (`flags.ron`), which is what keeps cavity from smearing across large faces.
