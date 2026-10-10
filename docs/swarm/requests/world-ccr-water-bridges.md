# CCR (text): water in the world, and bridge load ratings

From lane WORLD. Owner request 2026-10-10 (rivers crossed by swimming, a wooden bridge or a road bridge). Design: `docs/lanes/world/routes-design.md` section 4. ARCH owns `crates/w5k_contract/src/world.rs`; this file is the request, not the edit. **Nothing blocks WORLD's generator work**: it builds on its own types and adapts when this lands. All additions are defaulted, so existing worlds and RON keep loading.

- **W-6 `WorldQuery::water_surface_m(x, z) -> Option<f64>`** (default `None`): the height of the water surface at a point, `Some` only where there is water. Depth is `surface - height_m`. Vehicles need it for fording (`CapabilityTable.fording_depth_m` already exists), buoyancy and swim drag; the AI and the viewer need it to see rivers. Pure and `&self` like the rest.
- **No `Material` change for water**: water is a *volume*, not a surface material (a riverbed is `mud` or `gravel` under water), so only W-6 is needed.
- **W-8 `PropKind` += `Bridge`**: bridge railings, piers and signs are props; the *deck* is in the heightfield. A bridge's load rating is course data, not a query: `PropRef` stays as is. Proposal: `WorldQuery::load_limit_kg(x, z) -> Option<f64>` (default `None` = unlimited) for decks, so a scenario can flag an overloaded crossing. Optional; the course survey carries the same number for the AI's grader.
- **W-9 (already in `world-ccr-materials-props.md`) `PropShape::Capsule/ConvexHull`**: rocks as convex hulls instead of half-buried spheres, later.

Who is affected: CHASSIS and TRACKS (read depth for buoyancy/drag), COMBAT (water blocks nothing), AI (water depth in its sensing), VIEWER (draw the surface). Needed by: the river PR (item 3 of the order of work); until then, `GridWorld::water_surface_m` is a WORLD-only method.
--- ARCH answer:
