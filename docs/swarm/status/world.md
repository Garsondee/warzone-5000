# Status: WORLD

**Last updated:** 2026-10-10 UTC | **Branch:** lane/world/props | **Contract pinned:** contract-v0.1 (commit f8f5e5d) | **Phase:** building

## Done
- PR 17 settling (merged): spike S-W, design note, CCR text. PR 21 data-driven bump strip (merged).

## In progress
- Branch `lane/world/course`, PR open: `GridWorld` (size-generic spike world, same golden hash), `CourseDef` RON + generator (hills with an exact max-grade clamp, one named hill, road by slope-cost A* then graded and stamped), `w5k world preview` (top-down PNG: `docs/lanes/world/media/slice/topdown.png`), `content/world/courses/slice.ron` (400 m square).
- Tests: course_ron_round_trips_and_every_param_checks, course_generator_is_deterministic_for_a_seed, terrain_never_exceeds_the_stated_maximum_grade, roads_never_exceed_the_stated_maximum_grade, road_cells_are_road_material_and_start_and_finish_are_where_the_waypoints_say.
- Known blemish: the Manhattan-metric grade clamp leaves faint axis-aligned streaks on steep flanks (cosmetic; a smoother clamp is a later polish).

- Branch `lane/world/props` (stacked on PR 40): mud by drainage area (D8 flow accumulation, spread), Poisson-disc tree stands, barricade across the road with a gap, props in the preview; slice course hash is a committed constant. Tests: mud_appears_only_where_the_drainage_rule_puts_it, no_tree_overlaps_a_road_or_a_building, trees_keep_their_minimum_spacing, every_prop_lies_inside_the_bounds, barricade_stands_across_the_road_and_leaves_the_stated_gap.
- PROVISIONAL(C-004): barricade placed at a fixed road fraction (chokepoint detection later).

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
