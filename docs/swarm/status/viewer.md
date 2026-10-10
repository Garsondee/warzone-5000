# Status: VIEWER

**Last updated:** 2026-10-10 10:45 UTC | **Branch:** lane/viewer/settling | **Contract pinned:** contract-v0.1 (commit f8f5e5d) | **Phase:** settling (PR open); building next

## Done
- Spike S-V passes (WebGL 2 under SwiftShader, 46 ms/frame, 10 s clip = 72 KB): `docs/lanes/viewer/spike-v.md`.
- Design note: `docs/lanes/viewer/design-note.md` (binary layout, ~70-100 B/vehicle-frame, module structure, CCR thoughts).
- `tools/viewer`: page core (rig tree, forward kinematics, sampling), `build.mjs`, `capture.mjs`; `w5k viewer dump-canned`.

## In progress
- Next branch `lane/viewer/build`: `w5k_replay` binary encode/decode and its named tests.

## Blocked
- Nothing.

## Next
1. Binary codec + the five named tests. 2. Page with cameras, scrub, pixel-difference smoke test. 3. Debug draw, HUD, plots. 4. `w5k viewer render` / `plot`. 5. first-light replay.

## Cards needed / PROVISIONAL decisions in force
- None. Compression (flate2) deferred: would need a dependency card.

## Evidence
- `docs/lanes/viewer/media/spike-v-tank.png`.

## Owner instructions received
- 2026-10-10: run without checking in (STATE.md); continuing straight into build steps.

## Handoff note (fill in when you stop)
- Changed: ... | Unfinished: ... | Surprised me: ... | I would do next: ...
