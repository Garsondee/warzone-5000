# Status: VIEWER

**Last updated:** 2026-10-10 11:30 UTC | **Branch:** lane/viewer/start | **Contract pinned:** contract-v0.1 (commit f8f5e5d) | **Phase:** M1 deliverables done, awaiting CI/merge

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

## In progress
- Nothing: PRs #15, #22, #26, #28, #33 (stacked, merge in that order) await ARCH. Idle.

## Blocked
- Cannot reply to ARCH: `send_message` is blocked for lanes by the guard. The owner asked (in my session) for two-way lane/ARCH conversation and told me to tell ARCH: `docs/swarm/requests/viewer-lane-messaging.md`. Needs ARCH to change the guard.

## Next
1. Page with cameras, scrub, pixel-difference smoke test. 3. Debug draw, HUD, plots. 4. `w5k viewer render` / `plot`. 5. first-light replay.

## Cards needed / PROVISIONAL decisions in force
- None. Compression (flate2) deferred: would need a dependency card.

## Findings for other lanes
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
