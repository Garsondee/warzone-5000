# Status: CHASSIS

*Update with every PR. Keep it under 40 lines. ARCH reads this file at every check-in instead of your whole session.*

**Last updated:** 2026-10-10 UTC | **Branch:** lane/chassis/load-sensitivity | **Contract pinned:** 0.3.0 (`0982843`) | **Phase:** slice 2, stage A

## Done
- S1 + design note (#7); steps 1-4 (#13, #30); wheeled assembly + Ackermann (#42); Mule strip run (#43); modes + ledger (#59);
  tilt-table and skidpad benches + patch-moment fix (#69); Scout whoops check: a real resonance, not a bug (#76). Contract 0.2.0.

## In progress
- Slice 2 stage A (ARCH 17:13Z: owner approved; C-016 still OPEN in QUEUE.md): tyre load sensitivity PR #118 (CCR-chassis-4 text in the design
- **Critical path (ARCH 18:16Z): tracked hull** on the chassis integrator via TRACKS' `TrackedVehicleGear` (glue-note supersedes my seam). PR 1:
  design delta + `tracked.rs` skeleton (torsion arm, reflected sprocket inertia, tests). PR 2: integrated tank rest + ledger balance.
  note, shared PROVISIONAL Params) + skidpad bench fix (a gear-shift lurch read as slide-out: the Scout's -34% impact row; now +0.4%).
  Hauler skidpad is at rollover onset (lightest wheel 0.6 kN of ~15 kN static), not power: `min_wheel_load_n` added. Tracked/sinkage note #123
  (ARCH: terramech soil dep + RunningGear seam approved). Sinking tyre (rigid-wheel Bekker, UNVALIDATED): #126. Tyre reads contract 0.3 fields
  (stand-in dropped; FORGE asked to fill them: `requests/chassis-forge-tyre-load-sensitivity.md`). Next: tracked hull.
- **Not trusted yet:** tyre curve linear to the cap (no slide drop; locked braking reads high); roll centre at wheel-centre height (sliding struts);
  single-ray contact; `relaxation_length_m`, `slip_damping_time_s` are estimates.
- **6x6:** expressible today with independent axles; walking beams / inboard leaves (`LinkageDef`) are refused until linkages land (about one PR).

## Blocked
- Nothing.

## Next
- suspension, tyre (rolling-resistance speed scale as a Param, not a const-ok literal), hull, wheeled assembly on the bump strip, ledger, glue API.

## Cards needed / PROVISIONAL decisions in force
- None.

## Evidence
- `python3 -I spikes/chassis/s1/s1.py` output in `spikes/chassis/s1/output.txt`. No Rust changed.

## Owner instructions received
- 2026-10-10 (owner, in chat): "go ahead with the hull".
- ARCH (2026-10-10): CCR-1..3 landed in contract-v0.2 (applied); rolling-resistance fade speed is a `Param` (done).
## Handoff note (fill in when you stop)
- (fill in at M1)
