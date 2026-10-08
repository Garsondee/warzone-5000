# Technical Architecture

*Status: approved approach, 2026-10-07. Decisions and their reasons are recorded in
[../planning/decisions.md](../planning/decisions.md); the build order is in [../planning/roadmap.md](../planning/roadmap.md).*

## 1. The core idea

**Godot 4.6 renders and presents. A deterministic Rust core decides every gameplay outcome and generates all content.**

```
            content/*.ron (parts, materials, tech)          player and AI commands
                           |                                         |
                           v                                         v
   +---------------- Rust ---------------------------------------------------------+
   |  w5k_math   fixed-point numbers, vectors, RNG, state hashing                   |
   |  w5k_forge  parts -> meshes + derived physical stats; vehicle assembly         |
   |  w5k_sim    deterministic simulation (tick = 50 ms): movement, legs, ropes,    |
   |             vision, combat, economy, research. Commands in, state + events out |
   +---------------------------|---------------------------------------------------+
                               | w5k_godot (GDExtension bridge)
                               v
   +---------------- Godot 4.6 ----------------------------------------------------+
   |  rendering (vertex-colour shader, fog of war), camera, UI (design screen,      |
   |  research draft), audio, interpolation between ticks, IK leg *visuals*,         |
   |  cosmetic physics (debris, ragdoll look), network transport                     |
   +--------------------------------------------------------------------------------+
```

**The rule:** gameplay truth lives only in the Rust simulation. Godot may embellish (detailed IK, ragdolls, particles) but its
results never flow back into the simulation.

> **Theory note: why separate simulation from presentation?** A battle is stored and shared as its *inputs* (rosters, map,
> seed), and each machine that shows it runs the full simulation itself. For that to work, every machine must compute exactly
> the same result from the same inputs, down to the last bit. Rendering, by contrast, can differ freely: one player can have a
> better graphics card or look at a different part of the map. Keeping the two in separate layers makes it impossible for a
> visual detail (camera position, frame rate, a physics-engine ragdoll) to change the game.

## 2. Why these tools

| Need | Choice | Why |
|---|---|---|
| Bit-identical gameplay on every machine | Rust with fixed-point maths | Full control of arithmetic; no hidden floating-point differences; fast; runs and tests in cloud sessions |
| Rendering, UI, input, audio, networking | Godot 4.6 | Open source, plain-text project files, runs headless in cloud sessions, imports Rust code as a GDExtension |
| Procedural content the AI can author and check | Rust part generator + headless renderer | Parts are text; renders are PNGs Claude can inspect; same code feeds the game |
| Owner can inspect models | GLB export | Opens in Blender and any glTF viewer |

Rejected: Unity (cannot run in cloud sessions, Personal licence needs a GUI sign-in), GDScript or C# for the simulation (too
slow or float-based; .NET is unavailable in cloud sessions), the engine's own physics for gameplay (not deterministic across
machines).

## 3. Crates and folders

| Path | Role | Status |
|---|---|---|
| `crates/w5k_math` | `Fx` (Q32.32 fixed point), `FxVec3`, deterministic sqrt/trig, PCG32 RNG, FNV-1a state hasher | M0 |
| `crates/w5k_forge` | Part schema (RON), convex-primitive mesher, per-vertex attributes, voxeliser, physical stats, armour tables, sockets, vehicle assembly, software preview renderer, GLB export | M1 |
| `crates/w5k_tools` | Command-line tools: render part previews and stats cards; later replays, seed sweeps, balance runs | M1 |
| `crates/w5k_sim` | Deterministic simulation | M3+ |
| `crates/w5k_godot` | GDExtension bridge | M2 |
| `game/` | Godot project | M2 |
| `content/` | Parts, materials, vehicle designs (later technology) as RON text | M1 |

## 4. Content pipeline (Part Forge)

1. A part is written as a **tree of primitives** (box, wedge, cylinder, frustum..., each optionally chamfered and tapered)
   placed with transforms, mirrored, arrayed or repeated radially. See [03-part-forge.md](03-part-forge.md).
2. Each primitive is built as a **convex polyhedron** (the convex hull of generated points), so faceted, flat-shaded
   geometry falls out naturally.
3. The part is **voxelised** to compute what it physically is: mass, centre of mass, inertia, armour shell versus internal
   volume, directional armour thickness, silhouette.
4. Vertex attributes (colour slot, baked ambient occlusion, edge flag) drive the procedural look in the shader.
5. Part statistics are **baked at content-build time** and shipped as data, so every client uses identical numbers. Assembly of
   parts into a vehicle is computed in the simulation in fixed point.

## 5. Simulation outline (M3 onwards)

- Fixed tick of 50 ms (20 Hz); walkers and ropes sub-step at 60 Hz.
- Entities stored as struct-of-arrays with stable ids; no unordered iteration.
- Movement in layers: strategic paths per movement class, local avoidance, then a body model per locomotion trait.
- Walkers: step scheduling against the support polygon, planar IK for legs, rope chains (XPBD), deterministic toppling.
- Vision: integer line-of-sight pair checks over a height grid, eye and target heights, optional virtual planet curvature,
  shared team detection, spotting for indirect fire.
- Research: seeded per-match card pool, drafts weighted by focus, objectives granting focus and blueprints, coverage validator.

Each of these gets its own design document when its milestone starts.
