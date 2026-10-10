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
| The canned replays and rigs | `w5k viewer dump-canned truck\|tank --out dir` |

The replay may be JSON (the debug form) or binary (`W5KR`). The rig is the canned one named in the header (`box_truck`, `box_tank`) or `--rig file.json` (a serialised `RenderRig`; FORGE's compiled rigs come this way).
Clips and pages are CI artifacts: do not commit videos; a PNG under `docs/lanes/<lane>/media/` is fine.

Tests: `cargo test -p w5k_replay` (codec), `node tools/viewer/test-decoder.mjs <dir>` (JS decoder against the Rust one),
`node tools/viewer/smoke.mjs page.html part1,part2` (triangle count, console errors, each joint alone moves the picture; SKIP means the rig has nothing to see there).
Controls: drag to orbit, wheel to zoom, space to pause, click the scope to seek.
