# Status: CHASSIS

*Update with every PR. Keep it under 40 lines. ARCH reads this file at every check-in instead of your whole session.*

**Last updated:** 2026-10-10 UTC | **Branch:** lane/chassis/build | **Contract pinned:** 80adac2 (0.2.0) | **Phase:** build (M1)

## Done
- Spike S1 + design note (PR #7): substep rule `ceil(20 f_max/60)`; relaxation-length friction holds a 10% grade.
- Steps 1-4 merged (#13, #30): `quarter_car`, `suspension`, `tyre`, `hull`, each with its oracle tests; `tuning.ron` Params.
- Step 5a (PR A): `wheeled` assembly + `steering` (Ackermann). Tests: ride height, axle loads, braking distance, braking load transfer,
  Ackermann radius, step steer, hump without bottoming. Contract now 0.2.0 (`80adac2`).
- Step 5b (PR B, stacked on A; split to stay under 400 lines): `w5k chassis strip` drives FORGE's Mule on WORLD's DataStrip (stand-in
  powertrain), writes replay + CSV; plots and a clip frame in `docs/lanes/chassis/media/`.

## In progress
- Next: the ledger (step 6), `steady_turn_lateral_load_transfer_matches_m_ay_h_over_track`, DRIVE's real DrivePort when it lands.
- Simplifications to revisit: unsprung inertia uses the previous substep's hull acceleration (translation only); `kappa_peak`/`alpha_peak_rad` unused;
  the viewer draws the strip flat until WORLD exports terrain.

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
