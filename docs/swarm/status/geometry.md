# Status: GEOMETRY

**Last updated:** 2026-10-10 UTC | **Branch:** lane/geometry/settling | **Contract pinned:** `contract-v0.1` = commit `f8f5e5d` | **Phase:** settling round, continuing into the build (owner instruction in force: do not wait for review)

## Done
- Settling PR: spike S-G (kernel in `crates/w5k_geo/src/{mesh,edge,bvh,cavity}.rs`, tests `tests/spike_g.rs`, report `docs/lanes/geometry/spike-g.md`, image `media/s-g.png`), design note, CCR text (`docs/swarm/requests/geometry-ccr-shapes-and-tracks.md`), theory starter.
- A6 and A7 reproduced as spike tests; BVH agrees with brute force on 2,560 rays.

## In progress
- Build step 1 (mesh kernel: weld, closed checks, mass by the divergence theorem, A2 A3 A4) on `lane/geometry/build`.

## Blocked
- Nothing blocking. Later: FORGE agreement on the part API and datum (design note sections 4 and 6); VALIDATION's M998 dossier (A10; placeholder dimensions until then, `PROVISIONAL(C-002)`); `contract-v0.2` for `ShapeDef` and `track_runs`.

## Next
1. Build step 1, then 2 (primitives, operators, `w5k geometry sheet` and `export`: the first picture), 3 (loft), 4 (bake, mostly done), 5 (wheels), 6 (the 4x4 utility truck with node tags, surface detail per card C-004).

## Cards needed / PROVISIONAL decisions in force
- `PROVISIONAL(status:geometry)`: the edge definition until LOOK agrees. `PROVISIONAL(D3)`: hull datum = hull box centre (FORGE). No new card.

## Evidence
- `cargo test --release -p w5k_geo`: 5 spike tests pass; A7 worst error 0.0033 at 1024 rays; bake 1.6 s / 3.1 s / 11.6 s at 128 / 256 / 1024 rays for 15k vertices (the 1024 figure breaks the 10 s line, so the default is 256).
- fmt, clippy `-D warnings`, constants lint, line budget (223 of 8000), deps lint: clean. Not run: Windows, goldens (none touched), impact matrix (no contract touched).
- Kill criterion "wear over 25% more triangles": hood alone +36%, nominal breach; the whole-vehicle test is at build step 6.
