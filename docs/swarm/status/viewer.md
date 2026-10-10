# Status: VIEWER

**Last updated:** 2026-10-10 11:30 UTC | **Branch:** lane/viewer/debug | **Contract pinned:** contract-v0.1 (commit f8f5e5d) | **Phase:** building

## Done
- Spike S-V passes (WebGL 2 under SwiftShader, 46 ms/frame, 10 s clip = 72 KB): `docs/lanes/viewer/spike-v.md`.
- Design note: `docs/lanes/viewer/design-note.md` (binary layout, ~70-100 B/vehicle-frame, module structure, CCR thoughts).
- `tools/viewer`: page core (rig tree, forward kinematics, sampling), `build.mjs`, `capture.mjs`; `w5k viewer dump-canned`.

- Settling PR #15 (open). Build step 1: `w5k_replay::binary` encode/decode, 7 tests (truck 23 B, tank 36 B per vehicle-frame).

- Step 2 (PR on `lane/viewer/page`): JS decoder (`tools/viewer/src/replay.js`, checked against the Rust codec by `test-decoder.mjs`), the page (orbit/chase, scrub, speed), `smoke.mjs` (triangle count, each joint alone moves pixels, no console errors).

- Step 3 (PR on `lane/viewer/debug`): debug draw (normal-force bars coloured by slip/bottoming, datum marker, velocity arrow, toggles), HUD, force-ledger panel, scope plots with synced cursor.

## In progress
- Step 5: `w5k viewer render` (MP4) and `w5k viewer plot` (PNG); then step 6, the first-light replay.

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
- `smoke.mjs` PASS on both canned rigs: turret_yaw, gun_pitch, gun_recoil, wheels, steer each move pixels alone. Pictures `docs/lanes/viewer/media/page-*.png`.
- `docs/lanes/viewer/media/spike-v-tank.png`.

## Owner instructions received
- 2026-10-10: run without checking in (STATE.md); continuing straight into build steps.

## Handoff note (fill in when you stop)
- Changed: ... | Unfinished: ... | Surprised me: ... | I would do next: ...
