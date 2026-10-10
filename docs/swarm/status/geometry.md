# Status: GEOMETRY

**Last updated:** 2026-10-10 UTC | **Branch:** lane/geometry/modules (from integration; settling #25, build #32, truck #34, export #35, hull #44-#46, silhouette #52-#53 are merged) | **Contract pinned:** `contract-v0.1` = commit `f8f5e5d` | **Phase:** settling round, continuing into the build (owner instruction in force: do not wait for review)

## Done
- Settling PR: spike S-G (kernel in `crates/w5k_geo/src/{mesh,edge,bvh,cavity}.rs`, tests `tests/spike_g.rs`, report `docs/lanes/geometry/spike-g.md`, image `media/s-g.png`), design note, CCR text (`docs/swarm/requests/geometry-ccr-shapes-and-tracks.md`), theory starter.
- A6 and A7 reproduced as spike tests; BVH agrees with brute force on 2,560 rays.

- Build PR (this branch): mesh kernel (weld, closed check, mirror, slice), exact mass properties, loft with bevel rings, wheels (tyre, lugs, rim, nuts from `shapes/wheel.ron`), flag bake, software rasteriser, `w5k geometry sheet wheel`. Tests: A1, A2, A3, A8 and the closed/mirror/weld checks.

- Truck PR (this branch): `part`, `truck` (data-driven part list `shapes/utility_4x4.ron`), subdivision before baking, panel plates and fittings (C-004), tests A4 (200 random in-range sets), A5 (golden hash `tests/golden/utility_4x4.hash`), A9 (against `box_truck()`), dimension and budget check (placeholder, 19.5k triangles of 40k). Pictures `media/truck-look.png`, `truck-edge.png`.

- Export PR (this branch): `export.rs` (RenderRig in the stand-in layout, normals smoothed within 40 degrees and split beyond, f32 once; binary glTF with `COLOR_0` = edge, cavity, 0), `w5k geometry export truck`, dimension table, tests (rig validates, flags in 0..1, 28.9k triangles of 40k, wheel meshes centred on their hubs, GLB chunks). Subdivision edge 0.3 m and 48-gon wheels at detail 1 (0.2 m gave 46k triangles); golden re-blessed for that.

- Hull PR (this branch), owner feedback "windows, mirrors and bumper float; mud guards are slabs": the hull is ONE lofted shell with the wheel arches cut into the section and a swept lip around each arch; every fitting is anchored to a surface of the shell and overlaps it (new tests: `every_fitting_is_embedded_in_the_hull_or_in_a_fitting_that_is`, `plates_and_glass_hug_the_shell_within_3_cm`, `wheels_clear_the_shell_and_the_arches_open_over_every_tyre`); `loft_beveled` (chamfered ends, vertex rows beside hard edges), `polygon_ring`, `sweep_arc`, `triangulate_ring` (non-convex caps); trapezoid side panes that follow the A-pillar; grille slats, bezels, sill steps, roof hatch, antenna, pintle. 34.1k triangles of 40k. Pictures `media/truck-*-look.png`; `--view` and `--size` options on `sheet`. Golden re-blessed (new hull).

- Silhouette PR (this branch), owner feedback "improve the silhouette; the windows need to conform better; the front side window and the front top must line up": glass as panel + raised mitered frame on the leaning (tumblehome) walls; the front window's slanted edge is the windscreen ramp offset into the cab (test: both 57.8 degrees); steeper windscreen; approach and departure ramps (`yo`), sloped hood nose, tucked sill, sloped cover; lips end at the local underside; M998-class dimensions from the dossier and A10 for the HMMWV (`utility_4x4_dimensions_match_the_m998_dossier_within_3_percent`); `sweep_loop` and `chamfer_polygon` kernel functions; perspective-correct depth in the rasteriser (a picture-tool bug that made the shell speckle through glass). About 36k triangles of 40k. Pictures `media/truck-*-look.png`.

- **Skins for the game garage (ARCH asked 2026-10-10, ahead of the weapons PRs).** `scout_4x4` (#79): `TruckKind`, `skin::Skin` (stand-in dimensions or `from_def` of FORGE's VehicleDef), the scout template `shapes/scout_4x4.ron`; the joint layout is the utility truck's by construction and a test compares them; dimensions equal FORGE's (test reads `content/vehicles/game/scout_4x4.ron`); 28.9k of 40k triangles; golden `tests/golden/scout_4x4.hash` (new). `hauler_4x4` (this branch, stacked on #79): `shapes/hauler_4x4.ron`, tall cab-over and flat bed, 38.3k of 40k triangles, golden `tests/golden/hauler_4x4.hash` (new), a silhouette test (the three skins differ by 48 to 75% of their silhouettes at one scale); API for VIEWER: `Skin::for_id("hauler_4x4")` or `Skin::from_def(&def)`, then `.parts(detail)`, then `render_rig`.

## In progress
- **Owner goal (2026-10-10): a vehicle is a separate hull, separate propulsion, separate weapon mount and separate weapons** (the first truck was parametric but one monolith: audit in design note section 9). The modules series, one PR each, stacked:
  1. #56 `module.rs`: sockets, modules, `Assembly::attach` (kind and size gating, mirroring for left sockets, caller-labelled names, refusals that change nothing); 14 tests, each new behaviour checked by deliberately breaking the code.
  2. #68 the truck is a hull module (`utility_hull`, publishes `Station` sockets, any number of axles) plus wheel modules (`gear.rs`) joined by the recipe `utility_truck`; golden hash unchanged and the exported rig JSON and GLB byte-identical to the monolith's; pictures `media/modules-propulsion-*.png` (hull alone, 4x4, 6x6).
  3. this branch (stacked on #68): the ring mount (`mount.rs`, `hardware.rs`, `shapes/ring_mount.ron`) on a `Ring` socket the hull publishes from its RON on the cab roof; truck 37.3k of 40k triangles; pictures `media/modules-mount-*.png`.
  4. next: weapon families (machine gun, autocannon) on the mount's trunnion, with swap pictures; 5. export for N stations and turret / gun / recoil nodes, exploded views, interface request to FORGE; 6. hull regions in metres.
  `PROVISIONAL(status:geometry)`: the socket vocabulary (`Station`, `Ring`, `Trunnion`) and frame convention.
- After the modules: the M113 (sloped hull, road wheels, sprocket, idler, track run and link: A10 A11), after FORGE/ARCH answer on the CCRs.

## Blocked
- Nothing blocking. Later: FORGE agreement on the part API and datum (design note sections 4 and 6); VALIDATION's M998 dossier (A10; placeholder dimensions until then, `PROVISIONAL(C-002)`); `contract-v0.2` for `ShapeDef` and `track_runs`.

## Next
1. Modules PRs 2 to 5 (above), each with a picture of a swap.
2. `lines` plan (sections, sheer, half-breadth), then the M113 (sloped hull, road wheels, sprocket, idler, track run) built as a tracked-gear module on the same sockets.

## Cards needed / PROVISIONAL decisions in force
- `PROVISIONAL(status:geometry)`: the edge definition until LOOK agrees. `PROVISIONAL(D3)`: hull datum = hull box centre (FORGE). `PROVISIONAL(status:geometry)`: socket vocabulary and frame convention (design note section 9). No new card.

## Evidence
- `cargo test --release -p w5k_geo`: 5 spike tests pass; A7 worst error 0.0033 at 1024 rays; bake 1.6 s / 3.1 s / 11.6 s at 128 / 256 / 1024 rays for 15k vertices (the 1024 figure breaks the 10 s line, so the default is 256).
- Modules PR 1: `cargo test -p w5k_geo --test modules` 12 pass; four mutations (no half turn, never mirror, no size check, reversed spin) each fail the intended tests; fmt, clippy, constants lint, line budget clean; about 210 lines of code, no golden touched.
- fmt, clippy `-D warnings`, constants lint, line budget (223 of 8000), deps lint: clean. Not run: Windows, goldens (none touched), impact matrix (no contract touched).
- Wheel sheet: `docs/lanes/geometry/media/wheel-look.png` (`w5k geometry sheet wheel --out DIR`, 0.5 s). A8 passes at the reference detail (128 segments, sag 0.12 mm).
- Kill criterion "wear over 25% more triangles": hood alone +36%, nominal breach; the whole-vehicle test is at build step 6.
- Hull rebuild findings: edge wear smeared as clouds across large faces until every hard edge had a vertex row `band` beside it, including the perpendicular ends and the crease stations (the spike's rule applied along the length); a 0.2 m cavity subdivision gave 46k triangles, 0.4 m gives 34k; a fan cap from a ring vertex was degenerate on collinear points, a centroid fan or ear clipping is not.
- Found while building the truck: the loft's end caps were fans from a ring vertex, degenerate on the collinear loop points (a sliver filter then opened holes); caps are now centroid fans. Flags need vertices where they vary: parts are subdivided to 0.2 m before baking (`flags.ron`), which is what keeps cavity from smearing across large faces.
- Tripwire note (RULES section 8): the hull PR is a rewrite of `truck.rs` (about 470 changed non-test lines, over 400). The kernel (#44) and the tool options (#45) were split out first; the rewrite replaces one module and has no working intermediate state, so it ships as one PR, stacked on those two.
