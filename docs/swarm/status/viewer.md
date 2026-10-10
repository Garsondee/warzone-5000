# Status: VIEWER

**Last updated:** 2026-10-10 22:46 UTC | **Branch:** lane/viewer/workshop-page | **Contract pinned:** contract-v0.3 (commit 0982843) | **Phase:** slice 2 stage C (Workshop) step 1 in progress; then the tracked carrier when FORGE T3 lands

## Done
- Spike S-V passes (WebGL 2 under SwiftShader, 46 ms/frame, 10 s clip = 72 KB): `docs/lanes/viewer/spike-v.md`.
- Design note: `docs/lanes/viewer/design-note.md` (binary layout, ~70-100 B/vehicle-frame, module structure, CCR thoughts).
- `tools/viewer`: page core (rig tree, forward kinematics, sampling), `build.mjs`, `capture.mjs`; `w5k viewer dump-canned`.

- Settling PR #15 (open). Build step 1: `w5k_replay::binary` encode/decode, 7 tests (truck 23 B, tank 36 B per vehicle-frame).

- Step 2 (PR on `lane/viewer/page`): JS decoder (`tools/viewer/src/replay.js`, checked against the Rust codec by `test-decoder.mjs`), the page (orbit/chase, scrub, speed), `smoke.mjs` (triangle count, each joint alone moves pixels, no console errors).

- Step 3 (PR on `lane/viewer/debug`): debug draw (normal-force bars coloured by slip/bottoming, datum marker, velocity arrow, toggles), HUD, force-ledger panel, scope plots with synced cursor.

- Steps 5 and 6 (PR on `lane/viewer/render`): `w5k viewer render|page|plot`; the first-light replay plays (clip 14 s, 0.57 MB; frame `docs/lanes/viewer/media/first-light-chase.png`). README `docs/lanes/viewer/README.md`.

- Real truck (PR on `lane/viewer/real-truck`): GEOMETRY's `utility_4x4` rig plays (built-in rig id for `w5k viewer render|page`), LOOK's camo and weathering through the hook `tools/viewer/src/look.js` (answer: `docs/swarm/requests/viewer-look-hook.md`), page modules generalised in `build.mjs`.

- Showpiece (PR on `lane/viewer/mule-showpiece`): CHASSIS's Mule strip replay on GEOMETRY's camouflaged `utility_4x4` (`--skin`, `w5k_replay::skin::retarget`: the skin's suspension arms take the Mule's rest positions, the body moves by the mean offset (0, -0.16, +0.3) m), WORLD's strip as ground (`--strip standard`), contact bars found by joint index. Clip: chase + orbit, 36 s each, force bars on.

- Course (PR on `lane/viewer/course`): reads WORLD's `w5k-terrain-1` (`--terrain`, or the header's `terrain`): hills, road ribbon, mud, trees and barricades as instanced props; layouts `full|inset|off` for the plots; a follow camera (`quarter`, `chase`) driven by the direction of travel over 0.8 s, so it is smooth over bumps and deterministic when scrubbing. Final clip: the whole 520 m road, quarter then orbit.

- Front camera (PR on `lane/viewer/front-cam`): `--camera front` (front-quarter, 7 m, pitch 0.22), page dropdown entry, HUD steering readout from the replay's steer joints, e.g. `steer L -15.0 R -13.2 deg (+ right)`.

- RTS camera (PR on `lane/viewer/rts`, default for recordings): pitch 55 degrees, heading-up (heading = centroid travel over 2.5 s), follows the centroid of all vehicles, aims 25 m ahead, distance auto-framed for the vehicles plus about 60 m of road. Next: multi-vehicle replays (PR B).

- Fleet (PR B on `lane/viewer/fleet`, stacked on the RTS PR): every vehicle in the header is drawn, one rig and skin each (`--rig a.json,b.json`, `--skin a,b,c`, the last repeats), name label above each, an accent tint on each paint (orange, cyan, magenta, lime: plain accents, LOOK has no palette tokens), leader line + per-vehicle table (name, speed, gear, steer), speed plot with one line per vehicle. `w5k viewer fake-fleet` writes test data (the first vehicle repeated, each 3 s behind) until ARCH's `course-compare` exists. The follow cameras (quarter, chase, front, orbit) still follow vehicle 0; RTS frames all.

- Live test-drive page (PR on `lane/viewer/live`): `tools/viewer/dist/index.html` (committed build; `node tools/viewer/build.mjs --live`), modules `live.js`, `live-input.js`, `live-audio.js`, `live.html`; `live-smoke.mjs` runs 21 checks against a real `w5k drive`. Named `index.html` (not `live.html`) because the server serves that name. Note for the tripwire list: the page fetches from the local server (same origin); that is the owner's goal for this page, not a replay-viewer network fetch.

- Skins for the live page (PR on `lane/viewer/skins`, stacked on the live-page PR): GEOMETRY's truck packed to `tools/viewer/dist/skins/utility_4x4.skin` (`w5k viewer pack-skin`, `w5k_replay::skinpack`), fitted onto scout, mule and hauler by `skin.js`; an RTS-view ring under the truck. The scout wears GEOMETRY's own skin (`scout_4x4.skin`, via `Skin::for_id`); for the hauler, pack GEOMETRY's hauler skin when it exists (`w5k viewer pack-skin hauler_4x4 ...`, then `node tools/viewer/build.mjs --live`). Finding for ARCH: `tools/ci/package_testdrive.py` zips only `index.html`; it must add `tools/viewer/dist/skins/*.skin` as `viewer/skins/` or the packaged game falls back to boxes.

- Start screen (PR on `lane/viewer/start`, stacked on the skins PR): the page opens on three big vehicle pictures and a huge DRIVE button with no stream open and no moving truck (the camera floats over the road start); tap = choose, DRIVE = go, keys 1 2 3 / arrows / Enter; no pedals are sent while it is open.

- Skin guard (PR on `lane/viewer/skin-guard`): tests that fail when a committed `.skin` differs from what `pack-skin` generates now, or when `index.html` lists other skins than `dist/skins` holds; all three skins packed after GEOMETRY #99 (hauler 44,410 triangles, 682 KB); the speed bar widens to 0-100 km/h when the kid cap is off (`--no-speed-limit`).

- Slice 2 stage A (PR on `lane/viewer/charts`): the **tornado** and the **ladder** as PNG charts (`w5k viewer tornado DIR/impact.json --out t.png`, `w5k viewer ladder ladder.json --out l.png`; `tools/viewer/chart.mjs`, drawing in `src/charts.js`; light and dark themes; 23 checks in `node tools/viewer/test-charts.mjs`; Rust tests keep the sample files honest and check VALIDATION's own `impact::evaluate` output for the fields and words the chart reads). **The tornado reads VALIDATION's real `impact.json`** (their runner landed while I built it, so I dropped my stub shape): one panel per benchmark, one bar per vehicle for each lever, their verdicts drawn as they are, failed checks always shown, "no runner yet" benchmarks named. Real picture `docs/lanes/viewer/media/tornado-impact.png` (31 of 54 signs right, 57%, on `integration` after TRACKS and CHASSIS merged; the findings are theirs). The **ladder** still reads a stub shape of mine (`w5k-ladder-1`, `docs/lanes/viewer/charts.md`) with invented numbers (the picture says STUB DATA) until TRACKS's ladder bench exists: request `docs/swarm/requests/viewer-validation-impact-shape.md` (optional names and thresholds in `impact.json`; TRACKS to give me its CSV columns or write the JSON). Palette checked with the dataviz validator, both modes. `chart.mjs` re-saves a picture over 300 KB with a 64-colour palette (media lint).
- Workshop design note (text only, same PR): `docs/lanes/viewer/workshop-design-note.md`: three layers of answer (instant re-fit in the browser, proxy skin about 0.5 s, final skin about 5 s), the endpoints `w5k drive` needs, the budget against "under 10 s", the test that will enforce it, five open questions with defaults.

- Track animation plan (text only, at ARCH's request, `docs/lanes/viewer/track-animation-note.md`): the belt needs no new replay data (`RenderRig.track_runs` plus the sprocket spin already in the frame); GEOMETRY's `Belt::round` is the path in closed form, ported to JS and checked against a Rust-written vector; the decisive risk is aliasing (links 0.152 m apart look frozen at 16 km/h at 30 fps), fixed by temporal supersampling (shutter blur); the sinkage overlay reads `ContactFrame.sinkage_m` now, and pressure needs a footprint area per contact (default: show sinkage and force until it exists). No code until ARCH says the contact data is real.

- **Workshop stage C, PR 1 (this PR, `lane/viewer/workshop-1`): `w5k viewer design`.** Levers in (FORGE `apply_both`), compiled design, skin (`FlagParams::preview()`), proving scoreboard against the base, timings out; `.skin` files accepted by `--skin`; 6 new tests (lever parsing, wheelbase factor reaches the skin's axles exactly, a refused lever says why, the board's change and direction, every board row is a benchmark the impact runner measures, a `.skin` file in place of a skin id). Picture `docs/lanes/viewer/media/workshop-design-hauler.jpg`. Decisions made (in the design note, "Step 1 as built"): own `w5k viewer workshop` server and DRIVE as a child `w5k drive`, so no change to ARCH's files. Next: PR 2 the server and page (sliders, three layers of answer, scoreboard), PR 3 DRIVE and the end-to-end test.

- **Workshop stage C, PR 2 (`lane/viewer/workshop-server`): `w5k viewer workshop`, the design server.** `GET /api/bases`, `/api/base/<id>`, `POST /api/design`, `GET /api/skin/<id>`, static files; designs built once per base, levers and quality; 422 with FORGE's reason for a refused lever; nothing served outside the page folder; 4 new tests (garage and six sliders with their base numbers, a design built once and its skin unpacks, refusals, files). Picture `docs/lanes/viewer/media/workshop-wheelbase.jpg`: the three skins the server returned for wheelbase x0.75, x1, x1.3 (drawn by the page of PR 3). **PR 1 (#167) and PR 2 (#169) merged.** Next: PR 3 the page (sliders, the body on a turntable, quick then final detail, latest answer wins) with `workshop-smoke.mjs`; PR 4 the scoreboard endpoint and DRIVE. FORGE's tracked render rig (#164) has landed, so job 2 (the carrier in the viewer) is unblocked after the Workshop.

- **Workshop stage C, PR 3 (`lane/viewer/workshop-page`): the page.** `workshop.html` and `workshop.js` (sliders with base numbers and percent changes, the body on a turntable, quick then final detail, latest answer wins, a refusal shown in words), `build.mjs --workshop`, the committed `dist/workshop.html` with a test that it calls the server's endpoints, and `workshop-smoke.mjs` (9 checks in headless Chromium against a real server: six sliders, the body grows by exactly the wheelbase change, quick body inside the 10 s budget, two quick moves end on the second, final detail replaces quick, reset, no console errors). Picture `docs/lanes/viewer/media/workshop-page.jpg`. Next: PR 4 the scoreboard endpoint, the scoreboard in the page and DRIVE.

## In progress
- Nothing. Waiting for ARCH's stage B (track animation, sinkage and ground-pressure overlay) and for the real impact and ladder files; the charts then read them instead of the stubs.

## Blocked
- Nothing. (Lane to ARCH messaging was enabled by ARCH; the old request `viewer-lane-messaging.md` is answered.)

## Next
1. Stage B when TRACKS lands track contact: track animation in the replay page (links along the belt, from the contract's `RenderRig.track_runs`, new in 0.3.0 and carried through `skinpack` by ARCH's migration edit), a sinkage and ground-pressure overlay under each vehicle. 2. Swap the stub samples for VALIDATION's and TRACKS's real output the day they land. 3. Stage C (Workshop) only after ARCH launches it; the note lists the requests I will file first.

## Cards needed / PROVISIONAL decisions in force
- None. Compression (flate2) deferred: would need a dependency card.

## Findings for other lanes
- GEOMETRY (and LOOK): a skin's cost is the **flag bake**, not the shape. Timed on a release build, hauler detail 0: `Skin::parts` under 10 ms, `render_rig` (edge and cavity bake) 4.9 s at `cavity_rays: 256`, 1.4 s at 64, 0.4 s at 16; the detail level barely matters (5.2 s at detail 0, 7.2 s at detail 2). Anything that reshapes a body live (the Workshop) needs `cavity_rays` as a per-call argument, so a proxy bake takes 0.4 s and the final one runs after the slider stops. The repository's dev profile is about three times slower than release (15 s per skin).
- VALIDATION: the whole proving battery for the Mule runs in 0.47 s on a release build (`w5k scenario proving --test all`), so a live scoreboard is affordable.
- ARCH/contract: `Frame` has no centre-of-mass offset (the marker shows the hull datum) and the ledger summary is magnitude-only (no direction or application point), so force *arrows* per term are drawn as a magnitude panel, not in 3D; the stand-ins write an empty ledger. CCR candidates: `VehicleHeader::com_m`, ledger vectors.
- GEOMETRY/FORGE: the box rigs' gun barrel is a plain tube, so 5 cm of recoil changes only ~8 pixels; a muzzle brake or ring makes recoil legible. Wheels have one small lug each (the only thing that shows spin).

## Evidence
- `docs/lanes/viewer/media/tank-gun-fired.png`.
- `smoke.mjs` PASS on both canned rigs: turret_yaw, gun_pitch, gun_recoil, wheels, steer each move pixels alone. Pictures `docs/lanes/viewer/media/page-*.png`.
- `docs/lanes/viewer/media/spike-v-tank.png`.

## Owner instructions received
- 2026-10-10: run without checking in (STATE.md); continuing straight into build steps.

## Handoff note (fill in when you stop)
- Changed: replay v2 binary codec + tests; three.js page, debug draw, HUD, ledger panel, scope; smoke/decoder tests; `w5k viewer render|page|plot`; theory note; README.
- Unfinished: 3D per-term force arrows (needs ledger direction + CoM in the contract, CCR candidates); camo hook (needs LOOK's slots); terrain drawing (needs WORLD's export); Windows run of the Node tests is not in CI (the Rust codec tests are).
- Surprised me: stand-in replays cost only 23-36 B/vehicle-frame because they are smooth; expect more with real tyre noise. A plain-tube barrel makes recoil nearly invisible.
- I would do next: point the page at FORGE's real truck rig and re-measure bytes per frame; add smoke.mjs to CI once Chromium is available there.
