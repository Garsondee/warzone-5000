# Lane GEOMETRY: commands and pictures

Run on your PC (`docs/dev-environment.md`): `cargo run --release -p w5k_tools --bin w5k -- geometry <command>`.

A vehicle is a hull module, running-gear modules, a weapon-mount module and a weapon module joined at sockets (design note section 9). Its
name in the commands below is a **subject**: `hull` (the hull alone), `truck` (the 4x4) or `truck6` (the same hull with a bed 1.2 m longer on three axles: front steer, rear tandem),
then optionally `+ring` (the ring mount on the roof socket), `+mg` or `+ac25` (a 12.7 mm class machine gun or a 25 mm class autocannon on the
mount; a gun brings the ring) and `+x` (the modules pulled apart). `truck6+ac25+x` is a 6x6 with the autocannon, exploded. `wheel` is the wheel.

| Command | What it does |
|---|---|
| `geometry sheet <subject>[,more...] --out DIR [--mode look\|shaded\|edge\|cavity] [--detail 0\|1\|2] [--view front34,rear34,side,front,rear,top,low34,close,gun] [--size WxH] [--grid N] [--back F]` | pictures of the subject(s): one `--view` gives a full-size single picture, several make a 2 x 2 sheet; `scout` is the light recon skin (a bare skin name takes no mount); a comma-separated list of subjects stacks them, or lays them out `--grid N` tiles per row, all at one scale, the largest vehicle's; `--back F` pulls the camera back (use 1.2 for an exploded view); `gun` is the close-up of the roof mount, `close` the greenhouse; `edge` and `cavity` show the baked per-vertex flags in false colour (blue 0, green 0.5, red 1) |
| `geometry export <subject> --out DIR [--detail 0\|1\|2]` | `<subject>.renderrig.json` (a `RenderRig`: nodes per station travel > steer > wheel, per mount turret_yaw > gun_pitch > gun_recoil, joints in the contract's layout, `edge` and `cavity` baked) and `<subject>.glb` (open it in Blender: `COLOR_0` = edge, cavity, 0); it lists the node of every joint coordinate. For a bare skin name (`truck` is `utility_4x4`, `scout` is `scout_4x4`) the files are named after the skin and a dimension table is written |

Pictures in `media/`: `truck-look.png` (four-view sheet with the flags applied the way LOOK will), `truck-front34-look.png`, `truck-side-look.png`, `truck-rear34-look.png`, `truck-close-look.png` (the greenhouse; full size), `truck-edge.png` (the `edge` flag); the modular series: `modules-propulsion-*.png` (hull module alone, 4x4, 6x6 with a longer bed; one scale), `modules-mount-*.png` (the ring mount on the roof socket, cradle empty), `modules-weapons-swap-*.png` (empty mount, machine gun, autocannon on the same truck), `modules-exploded-front34.png` (hull, wheels, mount, gun pulled apart), `modules-swap-grid-front34.png` (4x4 and 6x6, machine gun and autocannon); `skin-scout.png` (the scout, four views) and `skin-scout-vs-utility-side.png` (scout and utility truck side by side at one scale); `wheel-look.png`, `s-g.png` (spike S-G plot), `utility_4x4.dimensions.md` (the dimension table, PLACEHOLDER until the dossier lands).

Everything is data: `crates/w5k_geo/shapes/utility_4x4.ron` lists the hull's parts as proportions of the hull box and the sockets it offers; `ring_mount.ron` and `weapons.ron` hold the mount and the gun presets; `placeholder_utility_4x4.ron` holds the stand-in dimensions (`PROVISIONAL(C-002)`, replaced by FORGE's Params); `flags.ron` holds the bake settings. Change a number, rerun the command.
