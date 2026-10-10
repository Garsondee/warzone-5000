# Visual target: what the four reference images ask of the world

Owner's aspirational images (2026-10-10), kept in `media/reference/target-1..4.webp`: (1) a green hillside with a rock outcrop, (2) a red-rock valley with a pool and a dirt track, (3) a desert road beside a fractured cliff, (4) a small military compound on a road. "The level of visuals we are hoping for eventually." **Everything must stay procedural, parametric and simple in geometry.** This note is what I see, what it implies, and the order to get there. It extends `routes-design.md`; none of it changes the physics (the ground a vehicle drives on is still the heightfield and the colliders).

## 1. What I see
**Form and geometry**
- **Low-poly, flat-shaded everything.** Terrain, rocks and cliffs are visible facets: large planar triangles on smooth ground, small ones where the form is complex. No smooth normals, no detail textures doing the work. (Images 1 to 3.)
- **Rock is faceted, strata-like and angular**: tall fractured outcrops with vertical creases, caps that overhang slightly, shards and slabs; boulders are rounded polyhedra (about 20 to 60 faces); **scree** (piles of small rocks) collects at the foot of every cliff and in gullies. (1, 2, 3.)
- **Cliffs read as planes with ledges**: stepped, with a flat grassy or mossy cap on top and shelves on the way up. (1.)
- **Roads are laid objects**: a clear asphalt surface with a **double yellow centre line and white edge lines** (3) or **dashed white lines** (4), a gravel or sand **shoulder**, kerbs and a footpath in the compound (4), slight wear and patches (3).
- **A compound**: flat-roofed boxes with roof clutter (air-conditioning units), sheds, a guard booth and a barrier gate, a **shipping container** in a saturated accent colour, a **chain-link fence** (thin, see-through), utility and light poles, traffic cones, parked pads of dirt; a vehicle on the road gives scale. (4.)
- **Vegetation as small clustered shapes**: trees as faceted ball-clusters on a thin trunk (4), shrubs as dark clumps (1, 4), **tufts of tall dry grass** (2, 3, 4), orange dry scrub in the desert (3).
- **Water as a flat grey-blue pool** with a sand/pebble shore (2).

**Colour and material**
- **One material per idea, strongly coloured**: green grass, **red sandstone vs grey granite** (the rock *type* is a palette choice), sand, asphalt. Variation is slow noise in value and hue, not texture detail. (1, 2, 3.)
- **Materials follow the terrain by rule**: grass on gentle slopes, bare rock where steep, moss on ledges, scree at cliff feet, sand near water and in dry valleys, dirt where traffic goes. (1, 2.)

**Presentation**
- **Diorama framing**: a cut-out slab of ground floating on a plain neutral gradient background, the camera about 30 to 35 degrees above the horizon, 3/4 view; the slab's edge shows a clean skirt (thin in 3, with a soil strip in 4).
- **One strong low sun** with long soft **cast shadows** (rock throws a shadow over the ground in 1 and 3), warm light, cool fill; a little ambient occlusion in creases. No post effects beyond that.

## 2. What it means for us
| Look | Needs | Lane |
|---|---|---|
| Facets, few triangles, large flats | **Decimated terrain mesh with flat normals** (error-driven, so flats are big triangles and rock is dense); per-face material colour | WORLD (export) + VIEWER (draw) |
| Faceted rock, outcrops, scree | **Procedural rock generator**: noise-displaced low-poly polyhedron, strata planes for cliffs, scatter rules for scree | WORLD |
| Materials follow terrain | **Rule-based painting** from slope, height above cliff foot, curvature, distance to water and road | WORLD |
| Lines on roads, shoulders, kerbs | **Road decals as polylines** exported with the road and a gravel shoulder in the splat map | WORLD (data) + VIEWER/LOOK (draw) |
| Grass tufts, shrubs, tree canopies | **Render-only "decor" instances** with parameters (never queried by physics) | WORLD (data) + VIEWER/LOOK |
| A compound | **Compound generator**: lot, fence ring, gate on the road side, buildings by setback rules, containers, poles, cones | WORLD |
| Pool and shore | **Lakes in basins**, sand shoreline | WORLD |
| Diorama, sun, shadows, AO | A preset: slab skirt, background gradient, sun with a shadow map | VIEWER + LOOK |
| Seeing our own progress | **A perspective software render** with flat shading and a z-buffer in `w5k world preview` | WORLD |

Geometry stays simple on purpose: a 2 km course at 1 m cells is 8 M triangles raw; error-driven decimation (the RTIN idea: split a right triangle only where the surface deviates more than a tolerance) takes a rolling field to about 5 to 20 thousand, which is *why* flat shading looks intentional rather than cheap. The collision world is untouched: decor and facets are presentation.

## 3. Plan (each one small PR, cut from the latest integration)
**V0 Perspective preview** (DONE, `w5k world view`, `crates/w5k_world/src/render.rs`: z-buffered flat-shaded rasteriser, fog, sky, a shadow ray per ground quad against the heightfield; props are low-poly solids; pictures in `docs/lanes/world/media/view/`; trees and rocks do not cast shadows yet) (do first: it is our eyes). Flat-shaded triangle rasteriser with z-buffer, sun direction, hard shadows by shadow-ray against the heightfield, the diorama camera. Outputs `perspective.png` next to `topdown.png`. Test: a known tilted plane gets the Lambert shade the formula gives; a pillar casts a shadow of the right length.
**V1 Decimated terrain mesh** (DONE, `crates/w5k_world/src/mesh.rs`, `w5k world export|view --mesh TOL`, `docs/swarm/requests/world-viewer-terrain.md`: exact-error RTIN, watertight, material boundaries kept sharp; slice 31 thousand triangles at 25 cm; images `media/view/*-mesh.png`; the interface request to VIEWER is the `mesh` section of that file; the slab skirt is V8) (RTIN or quadtree), flat per-face normals, per-face material from the splat; `terrain.json` gains `mesh` (vertices, indices, face materials) with a stated triangle budget and the maximum height error. Interface request to VIEWER to draw it with flat shading and a slab skirt. Test: max error <= tolerance everywhere; triangle count falls as tolerance rises; every query point is within tolerance of the mesh.
**V2 Rule-based material painting**: slope > threshold becomes rock of the course's rock type, ledges (flat cells on a cliff) become moss, scree where slope relaxes below a cliff, sand near water and in dry valleys, dirt strips; parameters in the course RON; rock type is a palette entry (grey granite, red sandstone, sand-coloured). Test: no grass on cells steeper than the rock threshold; scree only within a stated distance below a cliff face.
**V3 Procedural rocks**: a generator for boulder meshes (noise-displaced icosphere decimated to N faces, flat shaded) with a size/shape/rounding parameter set and a collider (sphere now, convex hull after CCR W-3), outcrop and scree scatter rules, cliff strata (layered planes by quantised height plus ledge cap). Test: facet count within budget; mesh volume vs collider volume sane; scatter never overlaps a road.
**V4 Road dressing**: centre line (single, double, none), edge lines, dashes, shoulder width and surface, kerbs, wear patches; exported as polylines with style; `w5k world preview` draws them. Test: lines are inside the road, spacing of dashes is as stated, shoulder keeps its stated width.
**V5 Decor**: tufts, shrubs, tree canopies as render-only instances (position, scale, kind) in `terrain.json`, density by material and slope with the same Poisson sampling; never in `WorldQuery`. Test: none on road or in water; counts follow the stated density; deterministic.
**V6 Compound generator**: a lot beside a road with a gate, fence ring (thin box colliders that break), buildings (box plus roof clutter props, setbacks and spacing by rule), sheds, container, poles at spacing, cones at the gate; parameters: size, building count, fence style. Test: fence closed except at the gate; buildings inside the lot, clear of each other; every building reachable from the gate (a path exists).
**V7 Lakes and shore**: fill basins up to a stated level (drainage pits from the mud rule), sand shoreline; reuses the river's water surface storage. Test: flat surface, shore band width as stated.
**V8 Diorama preset** (VIEWER/LOOK): slab skirt with a soil strip, background gradient, sun with shadow map, AO; WORLD supplies the slab outline and a camera preset (`diorama` in the course RON).

Order by value over cost: V0, V1, V2, V3, V4, V5, V6, V7. Each lands with a before/after image from V0's renderer, so the owner sees the gap to these four pictures closing.

## 4. Decisions and defaults (PROVISIONAL)
- **PROVISIONAL(look-flat)**: flat shading, not smooth normals, is the house style for terrain and rock.
- **PROVISIONAL(look-decor)**: decor is render-only and does not appear in `WorldQuery` (CCR needed only if a gameplay lane wants cover from bushes).
- **PROVISIONAL(look-palette)**: three starting palettes: temperate (green and grey), red desert (red sandstone, sand, dry scrub), compound (muted greens, tan, one accent orange).

## 5. Requests this implies for other lanes (to be filed when each step starts)
VIEWER: draw `terrain.json` `mesh` flat shaded with a slab skirt (V1, V8); polylines for road lines (V4); instanced decor (V5). LOOK: palette and shader look, shadow and AO (V8). ARCH: CCR W-3 (convex-hull prop shape) for the rocks (V3).
