# Lane WORLD: an obstacle course worth driving: hills, roads, mud, trees, barricades, buildings

## Mission
When you have succeeded, there is a 2 km square course, built reproducibly from a seed, with rolling hills, roads that respect a grade limit, mud where water would collect, stands of trees, barricades across the roads and a village of buildings, and every vehicle model can ask it for height, normal, surface material and
props in a few hundred nanoseconds. You own the terrain data model, the material and soil table (with cited numbers), the props, the procedural generator, the `WorldQuery` implementation, and the mesh export that the viewers draw. The owner will see top-down and perspective renders of the course and a fly-through.

## You own  (the CI lane guard enforces it)
`crates/w5k_world/**`, `content/world/**`, `crates/w5k_tools/src/cmd/world.rs`; always `docs/swarm/status/world.md`, `docs/swarm/requests/world-*.md`, `docs/theory/world.md`, `docs/lanes/world/**`, `spikes/world/**`.

## You read, never edit
`crates/w5k_contract` (`WorldQuery`, `Material`, `MaterialDef`, `MaterialTable`, `SoilParams`, `PropRef`, `PropShape`, `PropKind`, `RayHit`), `docs/architecture/*`, `docs/brief/NOT-MODELLED.md`.

## Stand-ins you start on
`FlatPlane` and `BumpStrip` show what a `WorldQuery` is used for; their numbers are stand-ins. You need nothing from other lanes to start; others need you: replace `BumpStrip` with a real strip early.

## Settling round (first hours; then stop for review)
1. **Spike S-W, query cost and determinism** (`docs/lanes/world/spike-w.md`): a 2001 x 2001 heightfield (1 m cells), bilinear height and a consistent normal, O(1) material lookup, a uniform-grid spatial hash for props; measure ns per `height_m` and `raycast`; check the generator is bit-identical across runs and platforms (hash the output).
2. **Design note:** the heightfield representation (f32 or f64 storage, f64 maths), the bilinear patch and the normal (analytic from the patch or central differences, consistent with the height), the material splat map, micro-roughness as a pure function of position (hash noise, never an RNG stream), props and their colliders, the baked binary format and the RON `CourseDef` (generator parameters plus hand-placed overrides: start, finish, waypoints, targets as `PropKind::Target`), the raycast (grid DDA plus prop shapes), and what the viewers need for the terrain mesh.
3. **CCRs** you expect: `Material` fields (wetness, vegetation drag), prop shapes (a capsule for tree trunks, a convex hull), `bounds`.

## Build order  (named tests)
1. Heightfield and queries: `flat_heightfield_returns_the_constant_height`, `bilinear_height_is_continuous_across_cell_boundaries`, `normal_matches_the_finite_difference_gradient`, `planar_ramp_has_exactly_its_slope`, `raycast_hits_the_analytic_plane_at_the_expected_distance`, `raycast_through_a_box_prop_hits_the_face`, `query_cost_is_under_100_ns_per_height_lookup` (a release-profile bench you report).
2. Materials: a real `MaterialTable` from RON with `Param`s and **cited soil numbers** (the textbook tables of Wong, *Theory of Ground Vehicles*, and the military cone-index literature; cite page or table, or tag UNVALIDATED and ask VALIDATION to source): `material_ron_round_trips_and_every_param_checks`, `soil_params_are_inside_their_published_bands`.
3. The generator (deterministic from a seed): hills from domain-warped fBm with a maximum-grade clamp; roads as splines laid by a slope-cost A* between waypoints, graded and rasterised into the splat map; mud where a simple drainage-area rule says water collects; tree stands by Poisson-disc sampling with density by material and slope; barricades at road chokepoints; a village of box buildings along a road. `generator_is_deterministic_for_a_seed` (hash of the baked output), `roads_never_exceed_the_stated_maximum_grade`, `mud_appears_only_where_the_drainage_rule_puts_it`, `no_tree_overlaps_a_road_or_a_building`, `every_prop_lies_inside_the_bounds`.
4. Export: terrain mesh with per-vertex material ids and a props list for the viewers (`w5k world export <seed> --out DIR`); `w5k world preview <seed> --out DIR` writes top-down and perspective PNGs (via VIEWER's plotter or a small rasteriser of your own for the top-down map).
5. The real **bump strip**: replace the stand-in with a data-driven strip (same features, same names) so CHASSIS and VALIDATION can use it on day one of M1.

## Acceptance for M1
The tests above pass on Linux and Windows; the data-driven bump strip is available to CHASSIS; one 2 km course exists as a committed `CourseDef` (the baked binary is a CI artifact, not a git object if it is large). The owner will see: a top-down map and a perspective render of the course, a grade histogram of the roads, and the mud map.

## Theory to explain in `docs/theory/world.md`
A heightfield as a displacement map (graphics analogy: it is exactly that, sampled with bilinear filtering); fBm and domain warping for plausible hills (the same noise as procedural terrain in any DCC tool); why a road is a path on a slope-cost graph; why mud sits where water collects (drainage area); why micro-roughness must be a pure function of position (so the same bump is there every time a wheel visits).

## Non-goals
Vehicle physics and soil laws (TRACKS, CHASSIS); persistent deformation and ruts; water simulation; rendering and materials look (LOOK); an in-Godot editor (later); vegetation beyond collision stands.

## Needs from others / gives to others
Needs: soil sources from VALIDATION; mesh export conventions from VIEWER and GODOT. Gives: `WorldQuery` to CHASSIS, TRACKS, COMBAT and AI; the bump strip and the course to ARCH's spine and VALIDATION's scenarios; the terrain mesh to the viewers.

## Tripwires specific to this lane
An RNG stream consumed in query order (queries must be pure functions); a soil number without a source; a query allocating memory; a generator result depending on iteration order of a hash map (use ordered containers); a binary larger than 20 MB in git.

## Done
M1 acceptance passes in CI, the theory note is written, the status file has the handoff note, you have idled; richer course features for M2 come as follow-up tasks.
