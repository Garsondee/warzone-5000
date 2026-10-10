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

The picker shows a picture of each vehicle (rendered by the page from the rig the server sends). The HUD is a huge speed bar (green to amber, `TOP SPEED!` when the kid cap holds the throttle back), a banner when the server reports `message_event` ("Back on the road!"), and nothing else: no plots, no numbers beyond the speed.
Test: start the server, then `node tools/viewer/live-smoke.mjs http://127.0.0.1:8787/ out/live` (headless Chromium: picker pictures, picking, keyboard, on-screen buttons, gamepad, camera key, reset and banner, backing up, sound, big targets, no console errors).
