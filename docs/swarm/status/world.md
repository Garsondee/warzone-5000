# Status: WORLD

**Last updated:** 2026-10-10 UTC | **Branch:** lane/world/rough | **Contract pinned:** contract-v0.1 (commit f8f5e5d) | **Phase:** building

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
