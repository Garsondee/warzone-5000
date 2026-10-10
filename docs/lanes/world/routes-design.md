# Route choices: cliffs, rock fields, a river and its crossings

Owner request (2026-10-10): courses with parts that offer **several ways to go, each a different test**: near-vertical cliffs with a zig-zag road up the face, stone-riddled fields that are hard to cross, and a river crossed by an amphibious approach, a rickety wooden bridge or a large road bridge. Later an AI reads the terrain and picks the route; we then see how it performs. This note is the plan; each item is its own small PR.

## 1. The idea: a course is a graph of choices, with an answer key
A **junction** splits the course into **branches** that meet again at a **merge**. A branch is a path (a road, a cross-country line) plus the terrain it crosses. What makes a branch a *test* is what it asks of the vehicle:

| Branch type | Asks of the vehicle | Principle |
|---|---|---|
| Direct, over a rock field | ground clearance, wheel/track size, line choice between rocks | obstacle height vs wheel radius (a wheel of radius R climbs a step of height h only if h < R, and needs traction at the contact angle `acos(1 - h/R)`) |
| Switchback road up a cliff | climbing grade, torque, turning radius, braking on descent | grade = rise/run; a hairpin needs the vehicle's turning circle to fit inside the road's bend |
| River, amphibious | buoyancy, swim speed, bank slope it can climb out of | Archimedes: floats if displaced water mass >= vehicle mass |
| River, wooden bridge | narrow, rough, **load rated**: heavy vehicles must not use it | a beam fails at a load; the rating is course data |
| River, road bridge | wide and strong, but a long detour | length vs risk trade |
| Mud shortcut | soil strength vs ground pressure | Bekker pressure-sinkage (TRACKS' lane) |

**The answer key** (what lets us grade an AI): `w5k world survey` writes, per branch, objective facts the generator knows exactly: length, total climb, steepest grade, fraction of mud, rock density along the line, narrowest width, load rating, water depth and a crossing type. A capability table (`CapabilityTable`: it already has `fording_depth_m`) then gives, for each vehicle, which branches are **feasible** and a cheap estimate of time on each. The AI is scored on (a) picking a feasible branch, (b) how close its time is to the best. The survey is data, not simulation; the real test is driving it.

## 2. What a heightfield can and cannot do (graphics analogy: a displacement map)
A displacement map has one height per (x, z): **no overhangs, no tunnels, and nothing under a bridge**. So:
- **Cliffs** are steep scarps, not overhangs. A 1 m grid can store any slope, but the bilinear patch and the contact model are only meaningful to roughly grade 1.5 (56 degrees); beyond that wheels cannot touch it anyway. We model a cliff as a step of height H over a face grade `g_face` (0.8 to 1.5), so its horizontal depth is `H / g_face`. A vehicle cannot climb it (trucks manage about 0.6); the road can.
- **Switchbacks**: a road of grade `g_road` (about 0.08) up a rise R needs length `R / g_road`. A cliff 20 m high at 8% needs 250 m of road; folded into legs of 80 m across the face that is 3 legs and 2 hairpins. The legs must be at least road width plus shoulder apart, so `legs <= depth / (width + shoulder)`; the generator **refuses** a cliff that cannot hold its switchbacks and says why. Hairpin radius must exceed the vehicle's turning radius, so it is a parameter, not an accident of smoothing.
- **Bridges** are baked into the heightfield as a raised deck over the water (nothing drives under a bridge, so one height per (x, z) is enough). Railings and piers are props. The water surface and the river bed are below it.
- **The global grade clamp** (the promise "no terrain steeper than X") becomes a per-cell limit map: cliff cells carry the cliff's limit, everything else the course limit. The test then reads: no cell exceeds *its own* limit.

## 3. Rock fields (first PR)
A rock field is a disc with a density and a size distribution. Rocks are `PropKind::Rock` spheres, half buried (centre at `0.4 r` above the ground) so a wheel meets a rounded surface rather than a floating ball; positions by Poisson-disc sampling so there is a guaranteed minimum gap (the lane the vehicle must find), sizes skewed to many small and few large (`r = lo + (hi - lo) * u^2`). The ground under the field is `gravel` (more rolling resistance, more roughness, a little less grip). The tests: nothing on the road, nothing outside the disc, minimum spacing kept, partly buried, and density in the stated band. Whether a given vehicle *can* pass is the survey's job (the **clear-gap width** of the field, `spacing - 2 r_max`, against the vehicle's width).

## 4. River and crossings (needs a CCR first)
The contract has no water: `WorldQuery` answers height, normal, material, props. A vehicle model cannot ask "how deep is the water here" or "where is the surface". CCR text: `docs/swarm/requests/world-ccr-water-bridges.md`. Meanwhile WORLD builds the river into its own types (`GridWorld::water_surface_m`), because the carving, the banks, the ford and the bridges are all generator work that needs no contract.
- **River**: a meandering centreline (a low-frequency warped line), carved as a channel of depth `d` and width `w`, banks graded to a stated slope, the splat map marks `water` (deep), `riverbed`/`shallows` (a ford: depth below the typical `fording_depth_m`) and `bank`.
- **Ford** (the amphibious route and the wheeled route through shallows): a stretch where the bed rises to depth < 0.8 m, if the course asks for one.
- **Wooden bridge**: deck 2.6 m wide, planks as a rough low-grip material, a **load limit** in `BridgeDef`. Collapse under overload is **not modelled** (no destructible terrain: NOT-MODELLED.md); the survey marks it infeasible for heavier vehicles and a test run that crosses overloaded is flagged by the scenario, not simulated.
- **Road bridge**: 8 m wide, high rating, long approach.

## 5. Order of work (each one PR under 400 lines of code)
1. This note, the CCR text, **rock fields** + gravel + preview. (now)
2. Per-cell grade limits, **cliffs**, and the **switchback** road generator with its fit check.
3. **River** (channel, banks, water material) and the `water_surface_m` query on `GridWorld`.
4. **Bridges** (wooden, road) and a ford.
5. **Routes**: junction/branch data in `CourseDef`, a fork course ("slice 2") that offers all of the above, and `w5k world survey`.
6. Contract-side: water in `WorldQuery` once ARCH accepts the CCR; viewer draws water.

## Questions for the owner (defaults in force; PROVISIONAL tags)
- **Q1 how steep is a cliff?** Default: face grade 1.0 (45 degrees), 15 to 25 m high. Steeper looks better but the road cannot be laid across it and contact is meaningless. (PROVISIONAL(route-cliff))
- **Q2 does a wooden bridge collapse?** Default: no, only rated; the survey says "do not cross". (PROVISIONAL(route-bridge))
- **Q3 what is the AI told?** Default: the AI gets what its sensors give (heights, materials, props, water depth), never the survey. The survey is the grader's answer key. (PROVISIONAL(route-ai))
