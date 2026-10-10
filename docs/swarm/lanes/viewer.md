# Lane VIEWER: anyone can watch any replay, with the forces drawn on

## Mission
When you have succeeded, any replay the simulation writes can be watched in a browser page (and recorded as an MP4) by the owner in a cloud-only session: articulated vehicles with wheels spinning, steering, suspension travelling, turret and gun moving,
force vectors and contact patches drawn on top, and scope plots of speed, rpm, gear and travel underneath. You also own the replay v2 format that is the interface between the simulation and every viewer (this page, the Godot player, the validation harness).
Think of the replay as a G-buffer for time: written once, read by many passes.

## You own  (the CI lane guard enforces it)
`crates/w5k_replay/**`, `tools/viewer/**`, `crates/w5k_tools/src/cmd/viewer.rs`; always `docs/swarm/status/viewer.md`, `docs/swarm/requests/viewer-*.md`, `docs/theory/viewer.md`, `docs/lanes/viewer/**`, `spikes/viewer/**`.

## You read, never edit
**Contract 0.1.1 note.** `Frame` now carries contact material, `limiting`, weapon state and `projectiles`, and `VehicleHeader` a `livery`; revolute joint coordinates are continuous (interpolate linearly); the contact frame is right-handed (y = left). Reserve an extension block and a version for projectiles and weapon state in your encoding; the aim-joint quantum must be finer than the gun-laying error you plot (16 bits over 2 pi, 0.1 mrad, at least).

`crates/w5k_contract` (`Frame`, `ReplayHeader`, `VehicleFrame`, `RenderRig`, `JointBinding`, `ForceTerm`), `docs/architecture/CONTRACTS.md` and `UNITS-AND-FRAMES.md`; the prototype viewer for ideas only (`reference/prototype-v0/tools/trial/{template.html,build.py,capture.js,frames.js}`: QUARRY, re-write, do not copy blindly).

## Stand-ins you start on
`truck_over_bumps()` and `tank_slew_and_pitch()` (canned replays with the matching `RenderRig`s from `box_truck()` and `box_tank()`). The JSON replay writer in `w5k_replay` (Launch Kit version) is yours to replace.

## Settling round (first hours; then stop for review)
1. **Spike S-V, can a cloud session see it?** (`docs/lanes/viewer/spike-v.md`): build a self-contained HTML page (three.js from an npm-installed copy pinned in `tools/viewer/package-lock.json`, inlined; cdnjs is blocked in cloud sessions) that plays `tank_slew_and_pitch()`; capture 10 s at 30 fps with headless Chromium (`/opt/pw-browsers`, do not run `playwright install`) and encode with `ffmpeg`. Report time per frame, file size, and whether WebGL works (software GL is fine).
2. **Design note:** the binary replay v2 layout (header JSON + frame blocks): quantisation of position (mm), rotation (smallest-three), joints (per-kind scale), rpm, gear, contacts, ledger summary; target at most 100 bytes per vehicle-frame at 30 Hz, and how 20 vehicles for 5 minutes stays reasonable (delta coding, optional compression); how a page receives a replay (embedded base64 vs fetched); the viewer's module structure.
3. **CCRs** you expect: anything missing from `Frame` that a viewer needs (camera targets, events, per-contact data).

## Build order  (named tests)
1. `w5k_replay` binary encode and decode, JSON kept as the debugging form: `position_round_trips_within_1mm`, `rotation_round_trips_within_0p01_degrees`, `joint_values_round_trip_within_their_quantum`, `replay_costs_under_100_bytes_per_vehicle_frame`, `decoded_replay_equals_the_original_within_quantisation` for both canned replays.
2. The page: load a `RenderRig` + replay; build the node tree; apply each `JointBinding` (axis, kind, index) every frame; orbit and chase cameras; scrub and speed control. Smoke test in headless Chromium: no console errors; triangle count equals `RenderRig::triangle_count()`; frame-to-frame pixel difference proves the wheels, turret, gun and recoil move (a check you script, not an eyeball).
3. Debug draw: contact patches and normal-force bars, centre-of-mass marker, suspension travel bars, force vectors per `ForceTerm` from the ledger summary, friction-circle glyph per tyre; a HUD (speed, rpm, gear, limiting factor); toggles for each.
4. Scope plots in the page (time series with a cursor synced to playback).
5. `w5k viewer render <replay> --out clip.mp4` (headless capture + ffmpeg) and `w5k viewer plot <csv> --out chart.png` (a small PNG chart tool other lanes use for their evidence; quarry `reference/prototype-v0/crates/w5k_tools/src/plot.rs` and the rasteriser for the idea, re-write for the new tree).
6. Real data: play the first-light replay from `w5k scenario first-light`.

## Acceptance for M1
The tests above pass on Linux and Windows; the page and the MP4 work for both canned replays and the first-light replay; the replay size budget holds; a screenshot for the owner of the truck over the hump with force vectors and the tank with the turret slewed and the gun fired.

## Theory to explain in `docs/theory/viewer.md`
The replay as a G-buffer for time; a vehicle as a skeleton (the joint chains are forward kinematics, exactly like a rig: travel > steer > spin); interpolation and why viewers can run at any frame rate; quantisation error budgets; why the wagon-wheel effect (a wheel looking like it spins backwards) is aliasing, not a bug.

## Non-goals
Materials, camo and weathering (LOOK); meshes (GEOMETRY); the Godot player (GODOT); any physics; editing replays.

## Needs from others / gives to others
Needs: LOOK's material-slot parameters to apply camo (a hook by M1, the full look at M2); GEOMETRY's real meshes (box rigs until then); ARCH's first-light replays. Gives: the replay reader to GODOT and VALIDATION; the PNG plotter and the MP4 recorder to every lane as their evidence tools.

## Tripwires specific to this lane
A viewer that computes physics (the page must be a pure function of the replay); a network fetch at run time; a dependency not in `package-lock.json`; a video committed to git (clips are CI artifacts).

## Owner's direction 2026-10-10 (card C-020, `docs/architecture/DESIGN-MODEL.md`)
- The Workshop shows **choices as sliders and consequences as read-only numbers**: no Mass, no centre-of-mass height, no top speed as a slider (tyre pressure replaces Mass now). Add a Consequences panel (mass budget bar, balance, ground pressure, power-to-weight) and a Flags panel fed by FORGE's design audit; later a hit map.

## Done
M1 acceptance passes, the plotter and recorder are documented in `docs/lanes/viewer/README.md`, the status file has the handoff note, and you have idled.
