# Status: WORLD

**Last updated:** 2026-10-10 UTC | **Branch:** lane/world/course | **Contract pinned:** contract-v0.1 (commit f8f5e5d) | **Phase:** building

## Done
- PR 17 settling (merged): spike S-W, design note, CCR text. PR 21 data-driven bump strip (merged).

## In progress
- Branch `lane/world/course`, PR open: `GridWorld` (size-generic spike world, same golden hash), `CourseDef` RON + generator (hills with an exact max-grade clamp, one named hill, road by slope-cost A* then graded and stamped), `w5k world preview` (top-down PNG: `docs/lanes/world/media/slice/topdown.png`), `content/world/courses/slice.ron` (400 m square).
- Tests: course_ron_round_trips_and_every_param_checks, course_generator_is_deterministic_for_a_seed, terrain_never_exceeds_the_stated_maximum_grade, roads_never_exceed_the_stated_maximum_grade, road_cells_are_road_material_and_start_and_finish_are_where_the_waypoints_say.
- Known blemish: the Manhattan-metric grade clamp leaves faint axis-aligned streaks on steep flanks (cosmetic; a smoother clamp is a later polish).

## Blocked
- Nothing.

## Next
1. Mud by drainage area, trees (Poisson disc), a barricade across the road; then `WorldQuery` props show in the preview.
2. `MaterialTable` with cited soil numbers (Wong tables; anything unciteable stays UNVALIDATED and goes to VALIDATION).
3. Export for the viewers, the theory note, the village.

## Cards needed / PROVISIONAL decisions in force
- None. C-004 default (parametric world) applies.

## Evidence
- Tests: see spike table. Image: `docs/lanes/world/media/spike-w-map.png`.

## Owner instructions received
- 2026-10-10: run without checking in (STATE.md); continue into build steps without waiting for review.
