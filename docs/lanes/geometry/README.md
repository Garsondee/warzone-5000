# Lane GEOMETRY: commands and pictures

Run on your PC (`docs/dev-environment.md`): `cargo run --release -p w5k_tools --bin w5k -- geometry <command>`.

| Command | What it does |
|---|---|
| `geometry sheet wheel --out DIR` | contact sheet (three-quarter, side, front, top) of the parametric wheel |
| `geometry sheet <wheel\|truck\|hull\|truck6\|truck-ring>[,more...] --out DIR [--mode look\|shaded\|edge\|cavity] [--detail 0\|1\|2] [--view front34,rear34,side,front,rear,top,low34,close,gun] [--size WxH]` | the same for the utility 4x4 (`hull`: the hull module alone, no wheels; `truck6`: the same hull on three axles; `truck-ring`: the truck with the ring mount on its roof; a comma-separated list stacks the subjects one above the other); `edge` and `cavity` show the baked per-vertex flags in false colour (blue 0, green 0.5, red 1); one `--view` gives a full-size single picture, several make a 2 x 2 sheet |
| `geometry export truck --out DIR [--detail 0\|1\|2]` | `utility_4x4.renderrig.json` (a `RenderRig` in the stand-in node layout of `testing::box_truck()`, with `edge` and `cavity` baked), `utility_4x4.glb` (open it in Blender: `COLOR_0` = edge, cavity, 0) and `utility_4x4.dimensions.md` |

Pictures in `media/`: `truck-look.png` (four-view sheet with the flags applied the way LOOK will), `truck-front34-look.png`, `truck-side-look.png`, `truck-rear34-look.png`, `truck-close-look.png` (the greenhouse; full size), `truck-edge.png` (the `edge` flag), `modules-propulsion-side.png` and `modules-propulsion-front34.png` (hull module alone, 4x4, 6x6: the same hull family with different running gear), `modules-mount-close.png` and `modules-mount-front34.png` (the ring mount on the roof socket, cradle empty; `--view gun` is the close-up of the roof), `wheel-look.png`, `s-g.png` (spike S-G plot), `utility_4x4.dimensions.md` (the dimension table, PLACEHOLDER until the dossier lands).

Everything is data: `crates/w5k_geo/shapes/utility_4x4.ron` lists the parts as proportions of the hull box; `placeholder_utility_4x4.ron` holds the stand-in dimensions (`PROVISIONAL(C-002)`, replaced by FORGE's Params); `flags.ron` holds the bake settings. Change a number, rerun the command.
