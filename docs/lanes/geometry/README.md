# Lane GEOMETRY: commands and pictures

Run on your PC (`docs/dev-environment.md`): `cargo run --release -p w5k_tools --bin w5k -- geometry <command>`.

| Command | What it does |
|---|---|
| `geometry sheet wheel --out DIR` | contact sheet (three-quarter, side, front, top) of the parametric wheel |
| `geometry sheet truck --out DIR [--mode look\|shaded\|edge\|cavity] [--detail 0\|1\|2]` | the same for the utility 4x4; `edge` and `cavity` show the baked per-vertex flags in false colour (blue 0, green 0.5, red 1) |
| `geometry export truck --out DIR [--detail 0\|1\|2]` | `utility_4x4.renderrig.json` (a `RenderRig` in the stand-in node layout of `testing::box_truck()`, with `edge` and `cavity` baked), `utility_4x4.glb` (open it in Blender: `COLOR_0` = edge, cavity, 0) and `utility_4x4.dimensions.md` |

Pictures in `media/`: `truck-look.png` (the truck with the flags applied the way LOOK will), `truck-edge.png` (the `edge` flag), `wheel-look.png`, `s-g.png` (spike S-G plot), `utility_4x4.dimensions.md` (the dimension table, PLACEHOLDER until the dossier lands).

Everything is data: `crates/w5k_geo/shapes/utility_4x4.ron` lists the parts as proportions of the hull box; `placeholder_utility_4x4.ron` holds the stand-in dimensions (`PROVISIONAL(C-002)`, replaced by FORGE's Params); `flags.ron` holds the bake settings. Change a number, rerun the command.
