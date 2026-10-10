From: WORLD   To: VIEWER (reader), ARCH (writes the header)   Needed by: slice   Status: OPEN
What I need: VIEWER draws `terrain.json` when a replay names it; ARCH's scenario runner sets `WorldHeader.terrain = Some("terrain.json")` and copies the file next to the replay.
Why: ARCH asked that the replay show the real hills and road instead of a flat plane. **No contract change is needed**: `WorldHeader.terrain: Option<String>` already exists (its doc comment names `w5k world export`). This file defines what the string points to.
What I will do meanwhile: `w5k world export <course.ron> --out DIR [--step N]` (this PR) writes `DIR/terrain.json`; default step 2 gives a 2 m mesh, 201 x 201 nodes, about 490 KB for the slice (a CI artifact, not committed).

## Format `w5k-terrain-1` (JSON; plain numbers, SI, frame as the contract: +Y up, -Z forward, +X right)
```
{ "format": "w5k-terrain-1", "course": "slice", "seed": 2026,
  "nx": 201, "nz": 201, "cell_m": 2.0, "origin_m": {"x": -200, "z": -200},
  "heights_m": [ ... nx*nz, row-major: index = j*nx + i, node (i, j) at x = origin.x + i*cell_m, z = origin.z + j*cell_m ... ],
  "material_ids": [ ... nx*nz u8, same indexing ... ], "materials": ["asphalt","dirt","mud"],   // id = position in this list
  "road_m": [[x, y, z], ...],          // the graded centreline, start to finish
  "props": [ {"id", "kind": "Tree|Barricade|Building|...", "shape": {"type": "cylinder|box|sphere", ...},
              "pos_m": [x,y,z], "rot_wxyz": [w,x,y,z]} ] }
```
- Shapes: `cylinder {radius_m, height_m}` stands on `pos_m` (base); `box {half_m: [x,y,z]}` is centred on `pos_m` and rotated by `rot_wxyz`; `sphere {radius_m}` centred.
- Triangulate each cell with the diagonal from (i, j) to (i+1, j+1). The physics surface is the bilinear patch, which differs from the two triangles by at most `|h00 - h10 - h01 + h11| / 4` (8 mm on the spike terrain): invisible, and the reason the viewer must not be used to judge contact.
- Colour by `material_ids` (`asphalt` grey, `dirt` green/brown by height, `mud` dark brown). LOOK may restyle; the ids are the contract.
- `water_m` (optional, additive; same indexing as `heights_m`): the water surface height at nodes whose ground is below it, `null` elsewhere. Draw a translucent plane at that height, clipped to the non-null nodes; colour by depth (`water_m - heights_m`).
- Heights are rounded to 1 mm (display precision); the simulation never reads this file.
--- ARCH answer: 
