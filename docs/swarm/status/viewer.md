# Status: VIEWER

**Last updated:** 2026-10-10 11:30 UTC | **Branch:** lane/viewer/mule-showpiece | **Contract pinned:** contract-v0.1 (commit f8f5e5d) | **Phase:** M1 deliverables done, awaiting CI/merge

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

## In progress
- Nothing: PRs #15, #22, #26, #28, #33 (stacked, merge in that order) await ARCH. Idle.

## Blocked
- Nothing.

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
