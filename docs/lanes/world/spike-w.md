# Spike S-W: query cost and determinism of a 2001 x 2001 heightfield world

**Question.** Can one flat heightfield (1 m cells, 2 km square) answer height, normal, material, props and rays fast enough for every vehicle model, and can its generator be bit-identical across runs and platforms?
**Answer.** Yes. Height is **24 ns**, height + normal + material **61 ns**, and the generator's output hash is a fixed constant checked on Linux and Windows CI. Ray casts cost far more (about 12 us for a 200 m ray that mostly misses), which is fine for sensors and shells but means vehicles must not ray-cast per wheel per substep (they have `height_m` and `normal` for that). Code: `crates/w5k_world/src/spike_w.rs` (throwaway; M1 splits it into modules). Image: `media/spike-w-map.png` (seed 7, hill-shaded; brown = the spike's low-ground "mud" rule, grey = a test road stripe).

**Method.** f32 height storage (16 MB), all arithmetic f64. Bilinear patch per cell; the normal is the analytic gradient of that same patch. Material = a u8 splat map read at the nearest cell. Props (20 000 tree cylinders, 200 oriented boxes) sit in a 16 m uniform grid stored as CSR arrays, so a query allocates nothing. Terrain ray casts walk the cells with a 2D DDA and solve one quadratic per cell (the bilinear surface is quadratic along any straight line); props are tested in the cells the same ray crosses.

| Check (test name) | Result |
|---|---|
| `flat_heightfield_returns_the_constant_height` | exact, normal exactly up, clamps outside the grid |
| `bilinear_height_is_continuous_across_cell_boundaries` | jump < 1e-6 m across 2000 sampled lattice lines |
| `normal_matches_the_finite_difference_gradient` | within 1e-5 of the central difference of `height_m` in cell interiors |
| `planar_ramp_has_exactly_its_slope` | slope 0.25 to 1e-6 |
| `raycast_hits_the_analytic_plane_at_the_expected_distance` | within 1e-4 m of the closed form |
| `raycast_on_rough_terrain_lands_on_the_surface` | hit point is on `height_m` to 1e-6 m |
| `raycast_through_a_box_prop_hits_the_face` | distance 48 m exact, normal -X, `props_in_aabb` finds it once |
| `generator_is_deterministic_for_a_seed` | two builds equal; seed 8 differs; seed 7 equals the committed constant (`StateHasher` over height bit patterns, splat, props) |
| `micro_roughness_is_a_pure_function_of_position` | same inputs, same bits |

**Cost** (release, one thread, this container; run `cargo test -p w5k_world --release -- --ignored --nocapture`): `height_m` 24.0 ns; `sample` (height + normal + material) 61.3 ns; `raycast` 12 us (200 m, one hit in 4096 rays, so close to the worst case: every cell and every prop cell walked); `props_in_aabb` (6 m box) 201 ns. Target was under 100 ns per height lookup: met with a 4x margin. The test is `#[ignore]` because a shared CI runner's clock should not fail a build; the number is reported here and re-measured in the M1 PR.

## Findings
| # | Finding | Evidence | Consequence |
|---|---|---|---|
| F1 | Bilinear is not planar: the physics surface and a two-triangle render mesh disagree by `|d|/4` where `d = h00 - h10 - h01 + h11` (the cell's twist) | max 8 mm on the spike's gentle noise (`spike_terrain_statistics`) | Acceptable for viewers; the exporter reports the max deviation of every course. Keep bilinear for physics (no diagonal bias; a wheel crossing a cell sees no direction-dependent kink) |
| F2 | The normal is only C0: it jumps across cell edges because the gradient of the patch is discontinuous there | by construction | Fine for 1 m cells and tyre patches of 0.1 m; a wheel model that needs a smooth normal should average the 4 corner normals itself, or WORLD adds `normal_smooth` later (not in M1) |
| F3 | Rays are 500x dearer than heights | 12 us vs 24 ns | Vehicles use `height_m`/`normal` per contact; `raycast` is for sensors, shells and line of sight |
| F4 | Max grade of the spike's fBm is 0.36 (20 deg) while 99% of cells are under 0.20 | histogram: `[173978, 200791, 60338, 8813, 855, 114]` per 0.05 bin | The real generator needs the max-grade clamp the brief asks for; the spike has none |
| F5 | Determinism needed no care beyond the rules: value noise from an integer hash (splitmix64), polynomial fade, only `+ - * / floor sqrt`; no `libm` call is needed until roads need `atan2`/`sin` | the hash is a constant | Noise is a pure function of `(seed, lattice point)`; the `Pcg32` stream is used only in the generator (props), never in a query |
| F6 | `props_in_aabb` must not report a prop twice when it spans cells | first draft would | Report a prop from the first query cell it shares (its minimum cell, clamped to the query's) |
| F7 | `Material` has no wetness and no vegetation drag; `PropShape` has no capsule or convex hull; `WorldQuery::bounds` returns a y range that is made up | design note section 9 | CCR text `docs/swarm/requests/world-ccr-materials-props.md` |
