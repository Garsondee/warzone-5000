# WORLD design note (settling round)

Lane WORLD builds a 2 km course from a seed and answers `WorldQuery` calls in tens of nanoseconds. Evidence for the choices below is the spike `docs/lanes/world/spike-w.md`. Numbers in this note that are not from the spike are plans, not results.

## 1. Heightfield: f32 storage, f64 maths
A 2001 x 2001 grid at 1 m is 4 M samples; f32 is 16 MB (f64 would be 32 MB and halve the cache hits). f32 resolves 1e-5 m at the course's heights (100 m), far finer than any suspension needs. Conversion f64 to f32 is exactly specified, so the baked values are identical on every platform. Everything computed from them is f64. The grid lives on the contract's frame: origin at the centre, x right, z toward the viewer, +Y up, so a 2 km course spans -1000..1000 on both axes. *Graphics analogy:* this is a displacement map, and `height_m` is the bilinear texture fetch of it.

## 2. The patch and the normal
Each cell is the bilinear patch `h = a + b u + c v + d u v`. The normal is the **analytic gradient of that same patch**, not a central difference of neighbours, so height and normal can never disagree (test: `normal_matches_the_finite_difference_gradient`). It is C0 (kinks at cell edges), see spike F2. A two-triangle viewer mesh differs from the patch by at most `|d|/4` per cell; the exporter reports the maximum for each course.

## 3. Materials: a splat map
One u8 per cell, read at the nearest cell: O(1), no blending (physics wants a decision, not a mix). Up to 255 materials; the table is the contract's `MaterialTable`, baked from RON `MaterialDef`s. Soil numbers carry sources: Wong, *Theory of Ground Vehicles*, soil tables (Bekker-Wong parameters for sand, clay, sandy loam, snow, muskeg), and the WES cone-index literature for the strength classes; anything I cannot cite to a page or table is tagged UNVALIDATED and put to VALIDATION as a request, never filled with a plausible number.

## 4. Micro-roughness
`micro_roughness_m(seed, x, z, rms)` is value noise at 2 cycles/m scaled by the material's `roughness_rms_m`: a pure function of position, so the same bump is there every time a wheel visits (a stream of random numbers would give a different bump each visit and make runs unrepeatable). It is added by the vehicle model as extra height; the heightfield itself stores only the resolved shape.

## 5. Props and colliders
Props are the contract's `PropRef`. Storage: a flat `Vec<PropRef>` in generation order (ids are indices) plus a 16 m uniform grid in CSR form (cell start offsets and an item list), built once. A prop is listed in every cell its box overlaps; `props_in_aabb` reports it from the first query cell it shares, so there are no duplicates and no allocation. Colliders: trees are vertical cylinders (capsule wanted, CCR), buildings and barricades are oriented boxes, rocks spheres (convex hulls later, CCR). `break_impulse_ns` decides whether a tree breaks. Target markers are `PropKind::Target`.

## 6. The generator (build step 3, plan)
All stages are pure functions of `(seed, parameters)` and use ordered containers only:
1. **Hills:** domain-warped fBm (spike code), then a max-grade clamp (iterated relaxation of the steepest cells) so a stated grade is a promise.
2. **Roads:** A* on the grid with step cost `length x (1 + (grade / g_max)^k)`, forbidden above `g_max`; the path is smoothed to a spline, graded (the terrain under the road is flattened toward the spline height across a width plus a shoulder) and rasterised into the splat map.
3. **Mud:** flow accumulation (drainage area) on the final terrain; cells with large upslope area and small slope become mud.
4. **Trees:** Poisson-disc sampling (Bridson) with density by material and slope; never on road or building footprints.
5. **Barricades** at road chokepoints (narrowest corridor between steep ground); **village** of box buildings along a road section.
Randomness comes from `Pcg32::derive(seed, [stage, index])` per stage and item, so adding a tree never changes the hills. Validators and lint (reachability start to finish, max road grade, soil sanity, every prop inside bounds and clear of roads) run on every generated course, and `w5k world diff` prints what a parameter change did. The generator parameters live in a RON `CourseDef` (`content/world/courses/*.ron`) with hand-placed overrides (start, finish, waypoints, targets).

## 7. Baked format and `CourseDef`
`CourseDef` (RON) holds seed, size, generator parameters and overrides; it is the only thing committed. The baked binary is a little-endian file: magic, version, N, cell size, f32 heights, u8 splat, material table (RON-in-JSON blob), props (fixed-size records), and a `StateHasher` hash trailer. About 20 MB, so it is a CI artifact and never a git object (large-file rule). Baking is deterministic, and the course's golden hash is a committed constant.

## 8. Raycast and the viewers
Raycast: terrain via 2D DDA plus one quadratic per cell, props via the prop grid walked by the same DDA, nearest wins (implemented in the spike, tested against the analytic plane and a box face). Viewer export (`w5k world export`): a binary vertex/index buffer per chunk (128 x 128 cells, 1 m or decimated by LOD) with position, normal and per-vertex material id; a props JSON (kind, shape, transform); and the maximum bilinear-vs-triangle deviation. `w5k world preview` writes a top-down map and a perspective render with a small rasteriser of my own, without waiting for VIEWER.

## 9. CCRs expected (text only: `docs/swarm/requests/world-ccr-materials-props.md`)
`Material` += wetness and vegetation drag; `PropShape` += `Capsule` and `ConvexHull`; `WorldQuery::bounds` made honest (y range is the terrain min/max plus tallest prop); optional `WorldQuery::normal_smooth`. None blocks M1: the table and queries work on `contract-v0.1`.

## 10. Risks and what I will watch
Road grading modifies the heightfield, so grade limits must be checked on the final field, not the noise. 1 m cells under a tracked vehicle's 0.1 m sampling may need a finer roughness. Raycast cost (spike F3) is fine for sensors; if COMBAT casts thousands per tick I will add a coarse max-height mipmap to skip empty cells.
