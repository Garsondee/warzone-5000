# Status: WORLD

**Last updated:** 2026-10-10 UTC | **Branch:** lane/world/mobility | **Contract pinned:** contract-v0.1 (commit f8f5e5d) | **Phase:** building

## Done
- PR 17 settling (merged): spike S-W, design note, CCR text. PR 21 data-driven bump strip (merged).

## In progress
- Branch `lane/world/course`, PR open: `GridWorld` (size-generic spike world, same golden hash), `CourseDef` RON + generator (hills with an exact max-grade clamp, one named hill, road by slope-cost A* then graded and stamped), `w5k world preview` (top-down PNG: `docs/lanes/world/media/slice/topdown.png`), `content/world/courses/slice.ron` (400 m square).
- Tests: course_ron_round_trips_and_every_param_checks, course_generator_is_deterministic_for_a_seed, terrain_never_exceeds_the_stated_maximum_grade, roads_never_exceed_the_stated_maximum_grade, road_cells_are_road_material_and_start_and_finish_are_where_the_waypoints_say.
- Known blemish: the Manhattan-metric grade clamp leaves faint axis-aligned streaks on steep flanks (cosmetic; a smoother clamp is a later polish).

- Branch `lane/world/props` (stacked on PR 40): mud by drainage area (D8 flow accumulation, spread), Poisson-disc tree stands, barricade across the road with a gap, props in the preview; slice course hash is a committed constant. Tests: mud_appears_only_where_the_drainage_rule_puts_it, no_tree_overlaps_a_road_or_a_building, trees_keep_their_minimum_spacing, every_prop_lies_inside_the_bounds, barricade_stands_across_the_road_and_leaves_the_stated_gap.
- PROVISIONAL(C-004): barricade placed at a fixed road fraction (chokepoint detection later).

- `w5k world export` + `docs/swarm/requests/world-viewer-terrain.md` (format `w5k-terrain-1`, no contract change: `WorldHeader.terrain` already exists). ARCH asks: (1) terrain in the replay header, then (2) land #41 and a mud/barricade section.

- Road mud crossing (25 m at 35% of the road, `road.mud_crossings` in the course RON) so the truck crosses soft ground on the road; test `road_is_mud_exactly_where_the_crossing_says`. The slice golden hash changed deliberately (course content added), constant updated in the same PR.

- Owner request 2026-10-10 (cliffs with switchbacks, rock fields, a river with amphibious / wooden bridge / road bridge crossings, route choices an AI picks between): plan in `docs/lanes/world/routes-design.md`; CCR text `docs/swarm/requests/world-ccr-water-bridges.md` (water surface query, bridge load limit). Step 1 (rock fields, gravel) in branch `lane/world/rocks`. PROVISIONAL(route-cliff) face grade 1.0; PROVISIONAL(route-bridge) wooden bridge rated not collapsing; PROVISIONAL(route-ai) the AI never sees the survey.

- ARCH order (13:40Z): (1) per-cell grade limits + cliff + switchback + steep direct line [this PR, `lane/world/cliff`, course `content/world/courses/ridge.ron`: scarp 20 m at face grade 1.0, switchback road with 7 m hairpins at the stated 8%, a gravel chute at grade 0.45, the long way round the eased end]; (2) washboard + whoops; (3) river + ford + bridges; (4) soft-ground patch (Bekker-style, UNVALIDATED); (5) `w5k world stats` (slope histogram, road grades, ISO 8608 PSD classes, material numbers beside Wong ranges, JSON + PNG). Each as its own small PR; PR numbers are visible on GitHub (lane sessions cannot message ARCH).

- (2) washboard + whoops [PR open, `lane/world/rough`, stacked on #62]: whoops (>= 4 m wavelength) baked into the heightfield, washboard (0.2 to 2 m) as a per-query formula (`corrugation.rs`: phase, weight and section per node, bilinear; normal is the exact gradient). Slice course now has both (golden hash updated deliberately). `raycast` ignores ripples (centimetres); not yet benchmarked with a washboard layer present.

- (3a) river + ford + water surface [`lane/world/river`, stacked on #65]: meandering channel in a levelled valley, trapezoid bed with a stated bank grade, flat water surface falling downstream, fords, `riverbed` material, `GridWorld::water_surface_m` (WORLD-only until CCR W-6), `terrain.json` gains `water_m`. Course `content/world/courses/river.ron`. (3b, next) bridges (wooden, road) and multi-road support (the long road bridge as a detour, a track to the wooden bridge). Amphibious crossing = swim the channel anywhere; the survey (later) will list depth and exit bank grade.

- (3b) bridges + multi-road [`lane/world/bridges`, stacked on the river PR]: `plan_road` extracted (per-road width, grade, surface, mud crossings, rough sections; slice/ridge/river golden hashes unchanged by the refactor), `extra_roads`, `BridgeDef` (Wooden / Road, width, load rating as data, rails as `Wall` props), decks laid after the clamp, `Course.bridges`/`extra_roads`, `terrain.json` gains `bridges` and `extra_roads_m`. Course `content/world/courses/crossing.ron`: road bridge (west, long), wooden bridge on a track (mid, short, light-rated), ford (east), swim anywhere else. Next: (4) soft-ground patch, (5) `w5k world stats`, then the route survey.

- (4) soft ground [`lane/world/soft`, stacked on #71]: `sand` material with Bekker-Wong soil parameters (dry-sand class, every number UNVALIDATED with bands; VALIDATION to verify the table and page), `SoftPatchDef` (a disc laid in any soft material, refused if the material has no soil), a sand patch on the slice course, and the brief's material tests (`material_ron_round_trips_and_every_param_checks`, `soil_params_are_inside_their_published_bands`).

- (5) `w5k world stats` [`lane/world/stats`, stacked on #72]: slope shares (>5, >10, >20, >30 deg), road grade distributions, roughness PSD (Welch, Hann, detrended) with the ISO 8608 class A to H and waviness exponent for each road and five cross-country lines, material numbers beside `content/world/published_ranges.ron` (all UNVERIFIED), `stats.json` + one-page `stats.png`. Reports for the four courses in `docs/lanes/world/media/stats-*/`. Theory note `docs/theory/world.md` written. Findings for VALIDATION: mud `soil.n` 0.8 is OUT of the (unverified) clay range 0.1 to 0.7; the slice course is hilly (about half of the ground over 5 deg, 14.5% over 10) and its main road reads class F at n0 because the 12 m whoops sit at the ISO reference frequency.

- Owner's visual target (4 reference images, 2026-10-10): `docs/lanes/world/visual-target.md` (observations, what it implies per lane, plan V0 to V8: perspective preview, decimated flat-shaded terrain mesh, rule-based material painting, procedural rocks, road dressing, decor, compound generator, lakes, diorama preset). Next: V0.

- SLICE-2 order from ARCH (owner approved): (1) ISO 8608 spectrum fix [this PR `lane/world/spectrum`: fit band 0.06-0.2 cycles/m, ground detail layer with a stated G(n0), roads carry their stated condition, washboard phase ring fix]; (2) mud pit with cited Bekker-Wong parameters; (3) proving-ground fixtures (ramp, side slope, step, trench); (4) `w5k world mobility` (go / slow / no-go map per vehicle); then visual-target V0 onward. NOTE for CHASSIS: `Material.roughness_rms_m` is read by WORLD as the resolved-band (2.5-20 m) roughness of a surface and baked into heights wherever a course has ground detail; a vehicle model must not add that same roughness again.

- (4) `w5k world mobility` [`lane/world/mobility`]: `mobility.rs` (`MobilitySpec` = name, mass, width and the contract's `CapabilityTable`; `classify` gives go / slow / no-go per node; `route` is a directional-edge Dijkstra from the start to the end of the main road reporting length, time, steepest climb and tilt, and what it used), command `w5k world mobility <course.ron> --out DIR [--vehicle spec.ron]` writing `mobility.json` + `mobility.png` (a map per vehicle with the route), three PLACEHOLDER/UNVALIDATED vehicles `content/world/mobility/{scout,mule,hauler}.ron` (the mule takes mass, width, 60% grade, 40% side slope and 0.762 m fording from `content/dossier/m998.ron`, all UNVERIFIED; the per-surface scaling is a guess). Output for `crossing` and `ridge` in `docs/lanes/world/media/mobility-*/`. Tests are physics sentences (grade limit, side slope zig-zag, gap detour, fording, bog, rock step, bridge width and load). Needs from ARCH: measured `CapabilityTable`s to replace the placeholders. Tripwire note: `w5k_tools` is at 99% of its line budget (5980/6000); the next command needs ARCH to raise it or move commands out. Not done: material ids above 6 (sandy_loam, from the mud pit PR #124) are not in the specs yet; they fall back to the vehicle's worst row until the specs list them.

- Mobility now uses VALIDATION's measured capability export [`lane/world/capability`]: `w5k world mobility` reads `docs/lanes/validation/capability/capability.json` (embedded; `--capability FILE` overrides) and replaces each stand-in's max grade, side slope and step by the measured values, except those the proving ground's own scorer marked red (the hauler's step stays the placeholder). Measured grade is for asphalt; WORLD scales it per surface by the peak-friction ratio (traction limit `tan(slope) <= mu`), PROVISIONAL(world-surface-grade), crude on soft ground until the proving ground runs on other surfaces. Still placeholders: fording, bog soils (`soil_go`), per-surface top speed, mass and width. Owner/ARCH question: is friction ratio an acceptable stand-in for per-surface grade until measured? Media regenerated. #124 and #129 had integration merged in and pushed (slice hash is the only golden change; stats treats mud-pit dips as road features like whoops).

## Blocked
- Nothing.

## Next
1. (done in props PR) mud, trees, barricade.
2. `MaterialTable` with cited soil numbers (Wong tables; anything unciteable stays UNVALIDATED and goes to VALIDATION).
3. Export for the viewers, the theory note, the village.

## Cards needed / PROVISIONAL decisions in force
- None. C-004 default (parametric world) applies.

## Evidence
- Tests: see spike table. Image: `docs/lanes/world/media/spike-w-map.png`.

## Owner instructions received
- 2026-10-10: run without checking in (STATE.md); continue into build steps without waiting for review.
