# Status: DRIVE

*Update with every PR. Keep it under 40 lines. ARCH reads this file at every check-in instead of your whole session.*

**Last updated:** 2026-10-10 | **Branch:** lane/drive/build | **Contract pinned:** contract-v0.1 (f8f5e5d) | **Phase:** building

## Done
- Spike S-D (`spikes/drive/spike_d.py`, `docs/lanes/drive/spike-d.md`): implicit stick/slip clutch stable 60 Hz to 1 kHz; explicit regularised sign never stable; converter fine with linearised pump torque; no DRIVE substepping needed.
- Settling PR #9 merged. Owner instruction (STATE.md, 2026-10-10 02:30 UTC): run without waiting for review; ARCH confirmed.
- Design note `docs/lanes/drive/design-note.md` with the CCRs expected (all additive, none blocking).

## In progress
- Build steps 5-6 in PR #37: driveline tree (open diffs, final drives, modes), `Powertrain: DrivePort`, lumped-vehicle bench (the Mule-class box truck launches, shifts 1-2-3, brakes to a stop). Next: locked/LSD diffs and steering units, fuel map, the named bench tests (0-32 km/h, top speed, hill start), `w5k drive bench`, the the DrivePort implementation (ARCH wants it today so CHASSIS can swap in).

## Blocked
- Tripwire noted: PR #37 is over 400 non-test lines (steps 3-6 accumulated while earlier PRs awaited merge, and ARCH asked for DrivePort today); commits are separable, history not rewritten. Future steps go in smaller PRs.

## Next
- After review: engine tests and `w5k_drive` engine/coupling code (build order 1-2); lumped bench to test the lock with a speed-dependent load.

## Cards needed / PROVISIONAL decisions in force
- None. Fuel map shape (quadratic bowl around the BSFC best point) is an ESTIMATE, not a card.

## Evidence
- Driveline tests (3) and bench smoke tests (launch/shift/stop, deterministic hash) pass. Known gaps: fuel_rate_kg_s and fuel_used_kg are 0 until the fuel map lands; locked/LSD diffs and steering units are refused with a reason.
- Brake tests (4: stop without reversing, energy balance, fade, parking hold on a grade) pass.
- Gearbox tests (5: ratio arithmetic, shift hysteresis, interruption time, reverse, no hunting) pass; shift tuning in `content/physics/drive/shift_tuning.ron`.
- Coupling tests (5, incl. the speed-dependent-load lock at 60-480 Hz vs a 4 kHz reference, and lock-up removing slip) pass; contract-v0.2 merged (pin 80adac2). Found and fixed: lock-up must share load with the converter while it ramps in (a hard switch hunted).
- Engine tests (7, named per brief) pass: `cargo test -p w5k_drive`. New: `serde` dep (workspace) and `ron` dev-dep in w5k_drive; `content/physics/drive/engine_tuning.ron` (idle gains, limiter fade, all Params).
- `python3 -I spikes/drive/spike_d.py` output in `docs/lanes/drive/spike-d-output.txt`. No image yet (no plotter); spike plots to follow with the first bench.

## Owner instructions received
- 2026-10-10: via ARCH, run on without checking in (STATE.md); slice target: M998-class truck powertrain through DrivePort.

## Handoff note (fill in when you stop)
- Changed: ... | Unfinished: ... | Surprised me: ... | I would do next: ...
