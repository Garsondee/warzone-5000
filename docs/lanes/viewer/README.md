# Viewer, recorder and plotter: how to use them

Setup (the SessionStart hook does this): `npm ci` in `tools/viewer`. Chromium is `/opt/pw-browsers`; Playwright is the container's global install
(`PLAYWRIGHT_DIR`, default `/opt/node22/lib/node_modules`); set `CHROME=/opt/pw-browsers/chromium-1194/chrome-linux/chrome`.

| Want | Command |
|---|---|
| A page you can open (or attach): orbit/chase camera, scrub, speed, HUD, ledger panel, scope plots | `w5k viewer page replay.json --out page.html [--rig rig.json]` |
| An MP4 with the overlays | `w5k viewer render replay.json --out clip.mp4 [--camera chase\|orbit] [--start S] [--seconds N] [--fps 30]` (about 0.13 s per frame in software GL) |
| The Mule on the strip with the detailed camouflaged truck and the strip's ground | `w5k viewer render replay.w5kr --rig rig.json --skin utility_4x4 --strip standard --out clip.mp4` (`--skin` draws another rig's meshes over the physics rig's skeleton: same joint indices, wheels placed where the simulation put them; `--strip standard` samples WORLD's bump strip as the ground) |
| The whole course: WORLD's terrain, road, mud and props | `w5k scenario mule-course --out DIR`, `w5k world export content/world/courses/slice.ron --out DIR2`, then `w5k viewer render DIR/replay.w5kr --rig DIR/rig.json --skin utility_4x4 --terrain DIR2/terrain.json --camera quarter --plots inset --out clip.mp4` (a terrain file named in the replay header is found by itself). Cameras: `quarter` (rear-quarter, follows the direction of travel), `chase`, `orbit`; plots: `inset` (small, default in recordings), `full` (the page), `off` |
| A PNG chart for your evidence | `w5k viewer plot data.csv --out chart.png --title "..." --ylabel "..."` (first column is x, other columns are series; no blanks, no NaN) |
| The design-impact tornado from VALIDATION's file (a panel per benchmark, a bar per vehicle for each lever) | `w5k validation impact --out DIR`, then `w5k viewer tornado DIR/impact.json --out tornado.png [--theme light\|dark] [--cols N] [--rows 8]` (shapes: `docs/lanes/viewer/charts.md`; real sample: `tools/viewer/samples/impact.json`) |
| The sinkage ladder beside the soil theory | `w5k viewer ladder ladder.json --out ladder.png [--theme light\|dark]` |
| A design: FORGE's levers on a base vehicle, compiled, with its skin and the proving scoreboard against the base | `w5k viewer design hauler_4x4 --out DIR --lever wheelbase=1.25,tyre_width=1.2,engine_peak_power=1.2 [--skin preview\|final\|off] [--score off]` (writes `<id>.ron`, `.extras.ron`, `.rig.json`, `.skin`, `design.json`; run from the folder that holds `content/`; a `.skin` file is accepted by `--skin` of `render`) |
| The Workshop's design server: bases, sliders, designs and skins over HTTP (the page is the next PR) | `w5k viewer workshop [--port 8789] [--web DIR]`, from the folder that holds `content/`; `GET /api/bases`, `GET /api/base/<id>` (the six sliders with their base values, and the base design), `POST /api/design {base, levers:{wheelbase:1.2,...}, quality:preview\|final}` (the rig, FORGE's report and the skin's URL; 422 with FORGE's reason when the physics refuses), `GET /api/skin/<id>` (packed skin bytes) |
| The Workshop page: sliders for six design levers, the vehicle rebuilt around each change | `w5k viewer workshop` then open `http://127.0.0.1:8789/` (`?still` keeps the turntable still); rebuild the committed page with `node tools/viewer/build.mjs --workshop`; end-to-end test: `node tools/viewer/workshop-smoke.mjs http://127.0.0.1:8789/` |
| The canned replays and rigs | `w5k viewer dump-canned truck\|tank --out dir` |

The replay may be JSON (the debug form) or binary (`W5KR`). The rig is the canned one named in the header (`box_truck`, `box_tank`) or `--rig file.json` (a serialised `RenderRig`; FORGE's compiled rigs come this way).
Clips and pages are CI artifacts: do not commit videos; a PNG under `docs/lanes/<lane>/media/` is fine.

Tests: `node tools/viewer/test-charts.mjs` (what the tornado and ladder draw, flag and count; no browser), `cargo test -p w5k_replay` (codec), `node tools/viewer/test-decoder.mjs <dir>` (JS decoder against the Rust one),
`node tools/viewer/smoke.mjs page.html part1,part2` (triangle count, console errors, each joint alone moves the picture; SKIP means the rig has nothing to see there).
Controls: drag to orbit, wheel to zoom, space to pause, click the scope to seek.

## The test-drive page (`w5k drive`)
A five-year-old can drive with it. Start the server from the folder that holds `content/`:
`w5k drive --vehicle content/vehicles/game/mule_4x4.ron --course content/world/courses/slice.ron --web tools/viewer/dist --open`
then the page is at http://127.0.0.1:8787/. It is **one self-contained file, `tools/viewer/dist/index.html`, committed to git** (no Node on the player's PC; under 1 MiB, the media-lint limit).
Rebuild it after changing `tools/viewer/src/live*.js`, `live.html`, `world.js`, `viewer.js` or `look.js` with `node tools/viewer/build.mjs --live` (`--check` says whether the committed file is current).
It talks to the server over the protocol in `docs/swarm/requests/arch-drive-protocol.md` (same origin; this is a deliberate, owner-driven exception to "no network at run time", localhost only) and computes no physics.

| Control | Keyboard | Screen | Gamepad |
|---|---|---|---|
| go | up arrow, W | big green GO | right trigger |
| stop | space | big red STOP | left trigger |
| down: brake, held after the stop it backs up | down arrow, S | amber BACK (backs up at once) | A |
| steer | left and right arrows, A and D | blue arrows | left stick |
| back on the road | R | orange circle arrow | Y |
| camera: chase or RTS | C | camera button | X |
| choose a vehicle | G or Escape; 1, 2, 3 in the picker | garage button | |
| sound on or off | M | speaker button | |

The page opens on a **start screen**: nothing is driving, the stream is not open, the camera floats slowly over the start of the road. Three big pictures of the vehicles (rendered by the page from the rig the server sends, each with its skin) and a huge green DRIVE button; tapping a picture only chooses it (the Mule is chosen to begin with; keys 1 2 3 or the left and right arrows choose, Enter or space drives). DRIVE turns the sound on, opens the stream and puts the chosen vehicle on the road start. The garage button brings the screen back during a drive (DRIVE then starts over with the chosen vehicle). The HUD is a huge speed bar (green to amber, ending at 30 km/h under the kid cap and `TOP SPEED!` when the cap holds the throttle back; if the vehicle goes faster than 27 km/h the cap is off, e.g. `--no-speed-limit`, and the bar covers 0 to 100 km/h), a banner when the server reports `message_event` ("Back on the road!"), and nothing else: no plots, no numbers beyond the speed.
Test: start the server, then `node tools/viewer/live-smoke.mjs http://127.0.0.1:8787/ out/live` (headless Chromium: picker pictures, picking, keyboard, on-screen buttons, gamepad, camera key, reset and banner, backing up, sound, big targets, no console errors).

### Skins in the test-drive page
The server's rigs are plain boxes (FORGE); the page draws GEOMETRY's detailed truck on each one's skeleton. `w5k viewer pack-skin <utility_4x4|scout_4x4> --out tools/viewer/dist/skins/<id>.skin` writes the compact skin file (GEOMETRY's skin registry; 587 KB for the 36,000-triangle truck: u16 positions, i8 normals, u8 flags; `crates/w5k_replay/src/skinpack.rs`), committed beside the page; `build.mjs --live` lists the files in `dist/skins/` into the page, so it never asks for a skin that is not there.
`fitSkin` (`tools/viewer/src/skin.js`) matches the skin's joints to the server rig's by index, puts the suspension arms where the simulation put them, scales the body by the ratio of the wheelbases and the wheels by the ratio of the tyre radii, so a tyre touches the ground where the physics says. A vehicle whose `skin` id has its own file in `dist/skins/` uses that (today the scout: `scout_4x4.skin`, fitted at scale 1.0); the others use `utility_4x4` (the mule at scale 1.0, the hauler grown by 1.15).
Tests: `node tools/viewer/test-skin.mjs` (unpack and fit, no browser), `cargo test -p w5k_replay skinpack` (round trip: positions within 0.1 mm, normals within a degree), and `live-smoke.mjs` (the scale of each vehicle's fit).

**Keeping the bodies current.** The skin files are generated; two Rust tests in `w5k_tools` keep them honest (`every_committed_skin_is_what_pack_skin_generates_now`: every skin id of GEOMETRY's registry has a committed `dist/skins/<id>.skin` that is byte for byte what `pack-skin` writes now, and no file is left for a removed id; `the_page_lists_exactly_the_skins_it_ships`: the skin list baked into `dist/index.html` is the files that are there). When GEOMETRY changes a body they fail with the commands to run: `w5k viewer pack-skin <id> --out tools/viewer/dist/skins/<id>.skin` for each id (`utility_4x4`, `scout_4x4`, `hauler_4x4`), then `node tools/viewer/build.mjs --live`, then `node tools/viewer/live-smoke.mjs` against a running `w5k drive`.
