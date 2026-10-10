# Status: DRIVE

*Update with every PR. Keep it under 40 lines. ARCH reads this file at every check-in instead of your whole session.*

**Last updated:** 2026-10-10 | **Branch:** lane/drive/build | **Contract pinned:** contract-v0.1 (f8f5e5d) | **Phase:** building

## Done
- Spike S-D (`spikes/drive/spike_d.py`, `docs/lanes/drive/spike-d.md`): implicit stick/slip clutch stable 60 Hz to 1 kHz; explicit regularised sign never stable; converter fine with linearised pump torque; no DRIVE substepping needed.
- Settling PR #9 merged. Owner instruction (STATE.md, 2026-10-10 02:30 UTC): run without waiting for review; ARCH confirmed.
- Design note `docs/lanes/drive/design-note.md` with the CCRs expected (all additive, none blocking).

## In progress
- Merged: #12 engine, #31 couplings, #37 gearbox/brakes/driveline/Powertrain/bench. In a new PR: shift map moved onto gearbox input speed (fixes the 1<->2 hunting ARCH saw at 40 km/h), economy scale 0.45, kick-down must land clear of the upshift point; fuel map wired (`fuel_rate_kg_s`, `fuel_used_kg`). Earlier note on steps 5-6 (PR #37): driveline tree (open diffs, final drives, modes), `Powertrain: DrivePort`, lumped-vehicle bench (the Mule-class box truck launches, shifts 1-2-3, brakes to a stop). Next: locked/LSD diffs and steering units, fuel map, the named bench tests (0-32 km/h, top speed, hill start), `w5k drive bench`, the the DrivePort implementation (ARCH wants it today so CHASSIS can swap in).

## Blocked
- Tripwire noted: PR #37 is over 400 non-test lines (steps 3-6 accumulated while earlier PRs awaited merge, and ARCH asked for DrivePort today); commits are separable, history not rewritten. Future steps go in smaller PRs.

## Next
- PR (locked and limited-slip differentials, requested by ARCH for the off-road sections): `driveline.rs` now links the two children of a Locked/LimitedSlip `Diff` (and the `locked_groups` of a drive mode) as implicit constraints/torque biases; bias ramps in over `lsd_full_bias_speed_rad_s` (Param, `driveline_tuning.ron`). Also fixed the reflected load slope. Tests: locked forces equal speeds and creates no torque; LSD never exceeds its bias, moves not creates torque, equals open at equal speeds; a mode can lock a group. Theory note `docs/theory/drive.md` written. After that, per ARCH: engine braking and brake fade on a long steep descent (switchback road), then steering units and the named bench tests.
- PR (hauler stuck in 1st, from ARCH course-compare): root cause was not the schedule values but that the hauler (and scout) have MANUAL boxes and `GearRequest::Auto` held the gear for them (no driver model). A manual box under Auto is now shifted by the driver model (same schedule, clutch opened for `shift_time_s`); the upshift point is also capped at 0.92 of each engine's redline (Param) so a schedule can never be unreachable. Garage test in `crates/w5k_tools/src/cmd/drive.rs` (DRIVE may not depend on FORGE): every garage vehicle accelerates flat out to its top gear with a monotone gear history, within its limiter, and settles at a top speed.
- PR (hunting round 2, from the w5k scenario mule-course): the replay showed shifts forced by pedal blips (kick-down on a one-frame full throttle) and by a pedal swing at one road speed turning an upshift into a downshift. Fix, all Params in `shift_tuning.ron`: shift map reads a low-pass-filtered pedal (0.8 s), the downshift curve follows the pedal only by 0.3 (so it lies under the upshift curve at every pedal pair), dwell 2.5 s, margin 250 rpm. Scenario now makes 11 gear changes in 68 s (was ~30), holds up to 10.5 s; remaining shifts are real (kick-down to 1st on 20 km/h corner exits, 3rd at 40 km/h). Tests: `no_road_speed_can_trigger_an_upshift_at_one_pedal_and_a_downshift_at_another`, `a_short_pedal_blip_does_not_kick_down` (both fail with the old Params), `a_cruise_with_a_wobbling_pedal_holds_one_gear_for_at_least_ten_seconds`.
- For ARCH (PR #49): the Mule-class gear set settles in 4th (about 1000 rpm) at a 40 km/h cruise, not 3rd; say if you want 3rd and I will raise the light-throttle downshift scale. `Gearbox::update` now takes output shaft speed (rad/s), not engine rpm; Powertrain already passes it.
- After review: engine tests and `w5k_drive` engine/coupling code (build order 1-2); lumped bench to test the lock with a speed-dependent load.

## Cards needed / PROVISIONAL decisions in force
- None. Fuel map shape (quadratic bowl around the BSFC best point) is an ESTIMATE, not a card.

## Evidence
- Cruise test `a_steady_40_kmh_cruise_holds_a_high_gear_without_hunting` (Mule-class gears: 1-2-3-4 on the way up, then no shifts) and `fuel_burned_equals_bsfc_times_work_on_the_map` pass.
- Driveline tests (3) and bench smoke tests (launch/shift/stop, deterministic hash) pass. Known gaps: locked/LSD diffs and steering units are refused with a reason.
- Brake tests (4: stop without reversing, energy balance, fade, parking hold on a grade) pass.
- Gearbox tests (5: ratio arithmetic, shift hysteresis, interruption time, reverse, no hunting) pass; shift tuning in `content/physics/drive/shift_tuning.ron`.
- Coupling tests (5, incl. the speed-dependent-load lock at 60-480 Hz vs a 4 kHz reference, and lock-up removing slip) pass; contract-v0.2 merged (pin 80adac2). Found and fixed: lock-up must share load with the converter while it ramps in (a hard switch hunted).
- Engine tests (7, named per brief) pass: `cargo test -p w5k_drive`. New: `serde` dep (workspace) and `ron` dev-dep in w5k_drive; `content/physics/drive/engine_tuning.ron` (idle gains, limiter fade, all Params).
- `python3 -I spikes/drive/spike_d.py` output in `docs/lanes/drive/spike-d-output.txt`. No image yet (no plotter); spike plots to follow with the first bench.

## Owner instructions received
- 2026-10-10: via ARCH, run on without checking in (STATE.md); slice target: M998-class truck powertrain through DrivePort.

## Handoff note (fill in when you stop)
- Changed: ... | Unfinished: ... | Surprised me: ... | I would do next: ...
