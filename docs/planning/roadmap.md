# Roadmap

Milestones are ordered to retire the biggest risks first. Each one ships a short theory note in `docs/notes/` so the owner
can follow the reasoning, and each has a check Claude can run in a cloud session.

| # | Milestone | Risk it retires | Verified by | Status |
|---|---|---|---|---|
| M0 | Cargo workspace; `w5k_math` (Q32.32 fixed point, sqrt/trig, RNG, state hashing) with property tests; design docs | Determinism foundation | `cargo test`; golden hash | **in progress** |
| **M1** | **Part Forge**: part schema, convex-primitive mesher, colour/AO/edge attributes, sockets, voxel stats, armour tables, vehicle assembly and budgets, preview contact sheets, GLB export; about 12 parts and 3 vehicles (drone, tank, spider walker) | Procedural look; AI authoring of parts | exact volume/mass/centre of mass for reference shapes; armour rises with slope; triangle budgets; golden images reviewed by Claude; **owner art gate** (renders and GLBs in Blender) | **in progress** |
| M2 | Godot bridge: gdext crate, vertex-colour shader, viewer scene; Windows build of the extension | Engine integration | in-engine screenshots in the cloud (Xvfb + software GL) or owner screenshots; owner runs it locally | planned |
| M3 | Simulation skeleton: 2,000 units, commands, replays, per-tick hashes | Determinism and performance at scale | identical hashes across runs and builds; per-tick time budget | planned |
| M4 | Walker and rope spike | Hardest physics | foot-slip, support and no-intersection invariants; "rope on legs 1-4 makes it fall toward side X", identical over 100 runs; filmstrips | planned |
| M5 | Terrain, movement classes, pathfinding, vision with curvature | Scale; information layer | synthetic terrains with known answers; brute-force references | planned |
| M6 | Lockstep prototype: two headless clients with desync detection | Networking | 30-minute sync run; an injected desync is caught | planned |
| M7 | Vehicles, combat and a vertical slice (designer, factory, economy, scripted AI) | Is it fun? | analytic speed curves; duel tables; owner playtest | planned |
| M8 | Technology draft, objectives, coverage validator | Randomness without unfairness | invariants hold over 10,000 seeds | planned |
| M9 | AI unit designer (MAP-Elites offline, beam search at runtime), balance runs | AI; balance | win-share reports | planned |
| M10 | Titans, flying battleships, strategic zoom at scale | Performance at the top end | benchmarks on the owner's GPU | planned |

## M1 detail (current)
1. Part schema and loader (RON).
2. Convex primitives: box, wedge, cylinder/prism, frustum, each with chamfer and taper; operators mirror, array, radial.
3. Mesh output: flat-shaded faces, per-vertex colour slot, ambient occlusion, edge flag.
4. Voxeliser: armour shell vs internal volume, mass, centre of mass, inertia; directional armour and silhouette tables.
5. Sockets and vehicle assembly with budgets (mass vs load capacity, power, internal volume).
6. Preview: 8-angle turntable contact sheet plus stats card (PNG); GLB export.
7. Starter content: hulls, turret, cannon, wheel, track unit, leg, sensor mast; assembled tank and walker.
8. Owner art gate: review renders, set the style direction (palette, chamfer sizes, level of detail).
