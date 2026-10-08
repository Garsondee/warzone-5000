# Part Forge

*Status: built (M1). Code: `crates/w5k_forge`. CLI: `crates/w5k_tools` (`w5k render | check | family`). Tests:
`crates/w5k_forge/tests/`. Parametric families: [04-parametric-components.md](04-parametric-components.md).*

The forge turns a part description into everything the game needs: a render mesh, physical properties, an armour table,
a preview sheet and a GLB file. Vehicles are parts attached to each other's sockets.

```
RON part / family generator --> shape tree --> convex pieces --+--> render mesh (flat faces, slot, edge, AO)
                                                              +--> voxel grid --> mass, centre of mass, inertia, internal volume
                                                              +--> exact ray casts --> armour and silhouette tables
```

## Authoring parts (`content/parts/*.ron`)
Conventions: metres, kilograms; +Y up, **forward -Z**, right +X; rotations are Euler degrees applied X, then Y, then Z.
Start each file with `#![enable(implicit_some)]` so optional values can be written plainly (`shell: 0.05`).

| Primitive | Key fields | Notes |
|---|---|---|
| `Box` | `size`, `taper` (top x, z scale), `shift` (top x, z offset) | tapered boxes make sloped plates |
| `Wedge` | `size`, `slope` (fraction of length), `nose` (front height fraction) | front is -Z |
| `Cylinder` | `radius`, `length`, `axis`, `segments`, `taper` (far end scale) | prisms and frustums; edges between segments stay sharp |
| `Sphere` | `radius`, `segments`, `scale` | faceted ellipsoids |
| `Beam` | `from`, `to`, `size` (width, height), `end`, `up` | legs, struts, arms, rams |
| `Hull` | `points` | convex hull of explicit points |

Every primitive takes `at`, `rot` (except `Beam`), `mat`, `slot`, `shell` and `chamfer`.
- **`slot`** names a colour role: Primary, Secondary, Trim, Dark, Metal, Rubber, Glow, Glass. The palette and team decide
  the actual colour.
- **`shell`** makes the piece hollow with walls of that thickness. Leave it out for solid pieces.
- **`chamfer`** bevels every edge sharper than 40 degrees.

Operators: `Group` (`at`, `rot`, `scale`), `Mirror` (`axis`, keeps the original), `Array` (`count`, `step`), `Radial`
(`count`, `axis`, `phase`).

Part fields:
- `id`, `name`, `category`, `size`, `tags`, `palette`;
- `voxels` (minimum resolution);
- `vital` (whether hollow insides are vital space; the default is true for hulls and turrets);
- `sockets`;
- `function`.

**Sockets** have a `name`, a `kind`, a `size`, a position `at`, an outward `normal` and a `forward` reference. Every
attachable part has a socket named `mount`. Attaching aligns the child's mount normal against the parent socket's normal
and the forwards together. `mirror` reflects the child left and right first; `spin` rotates it about the socket normal.

**Function** fields (all optional): power (`power_kw`, `draw_kw`); running gear (`locomotion`, `load_kg` rated load,
`rolling` C_rr, `traction`, `max_kmh`, `contact_m2`, `contact_len_m`, `step_m`, `ride_height_m` which raises the hull,
`rail_bound`); lift (`rotor_radius_m`, `cushion_area_m2` / `cushion_perimeter_m` / `cushion_gap_m`, `grav_kw_per_t`);
fit checks (`ring_m` for turrets, `mast_m` for mast-mounted parts); and the weapon, sensor and repair summaries
(`weapon`: kind, energy, rate, penetration, range, recoil, burst power, guided, salvo; `sensor`: kind, range, height;
`repair_kg_s`, `repair_reach_m`).

**Materials** (`content/materials.ron`) have three properties:
- `density`;
- `hardness`: protection per metre relative to armour steel;
- `armour`: false for guns, engines and electronics, which do not protect what is behind them.

Homogenised materials stand in for mixtures, so chunky shapes weigh what the real thing would: `running_gear` for
track pods, `fittings` for stowage and fenders, `machinery` for engine bays.

## How things are measured
- **Pieces are convex sets of half-spaces** (planes). Containment is a few dot products. Transforms, mirroring and
  chamfers are plane operations, and faces fall out of vertex enumeration.
- **Classification** (the same rule for voxels and rays):
  - solids are always material;
  - otherwise any vital shell's interior wins, so overlapping hollow pieces form one hollow volume with no hidden
    bulkheads;
  - otherwise a shell's wall is material;
  - the hollow of a non-vital shell is air.
- **Mass, centre of mass and inertia** come from the voxel grid. Resolution is at least 64 cells along the longest axis
  and is raised until the thinnest shell spans two cells (up to 256). Each piece's cells are rescaled to the piece's
  **exact** polyhedral volume, so thin barrels and plates weigh what they should; the grid only decides how overlapping
  pieces share space.
- **Armour tables:**
  - 8 elevations by 32 azimuths, with an 80 by 80 grid of parallel rays per direction;
  - each ray is clipped exactly against every protective piece;
  - the result is the steel-equivalent thickness before the first vital point;
  - mean and 10th percentile ("weak") per direction, plus hittable and vital area.
  Sloped armour emerges: a plate of thickness t at angle a is crossed over a path t / cos a.
- **Render mesh:** flat-shaded polygons; faces hidden inside another piece are dropped. Each vertex carries its colour
  slot, an edge flag (chamfer faces, for worn highlights) and ambient occlusion (24 rays through the voxel grid).

## Vehicles (`content/vehicles/*.ron`)
A design names a hull and a tree of attachments. Mass, centre of mass and inertia are **composed from the parts' own
properties**: masses add, the parallel-axis theorem moves each inertia to the common centre. So a vehicle always weighs
exactly the sum of its parts. The vehicle's own grid gives bounds, free internal volume (internal parts take up their
hull's space) and AO.

The vehicle sheet reports:
- totals (power, draw, load), frontal, side and top area, and the median steel-equivalent armour from four directions;
- **lift power** for rotors (actuator-disc theory), air cushions (leakage flow) and anti-gravity (kW per tonne), and the
  ground pressure where something touches the ground;
- top speed, from the power balance P = C_rr m g v + 1/2 rho C_d A v^3 after drive efficiency and after the lift has
  taken its share, capped by the running gear's `max_kmh`, and which of the two set it; turn rate by kind of gear;
- weapons (firepower, alpha strike, best penetration, longest range, recoil), sight (the horizon from the highest
  sensor, limited by its range) and repair capacity;
- problems: no locomotion, no engine, overloaded, power deficit, cannot hover/float/rise, engine too big for its bay,
  turret ring or mast head too big for its mount, bad sockets.

## Outputs and tools
Mixing and sampling tools are listed in [07 Possibility space](07-possibility-space.md): `sweeps`, `view`, `gallery`, `roll`,
`atlas`, `ladder`, `space`, `fit`.
- `w5k render content --out DIR [--only ids]` writes, per part and vehicle:
  - a contact sheet: 8 views, stats and two armour polar plots;
  - a `.glb`: `COLOR_0` with baked edge wear and AO, the raw `_W5K` attributes, and socket nodes;
  - a `.stats.ron`.

  It also writes `overview.png`.
- `w5k check content` builds everything and reports problems (exit code 1 on any).
- `w5k family content --out DIR` renders parametric family sweeps and slider-coupling demos.

## Known limits
- Shapes are unions of convex pieces; concave detail is kitbashed.
- The armour tables average over the whole silhouette. Per-part hit-location tables come with the battle simulation.
- Stats are baked in floating point; battle inputs carry baked stats from one authoritative baker (see
  [05-game-loop.md](05-game-loop.md), technical consequences).
- Speed comes from a flat-ground power balance. Slopes, soft ground and gait limits arrive with the simulation.
