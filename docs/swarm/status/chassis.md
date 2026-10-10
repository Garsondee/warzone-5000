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
- #59 merged (modes + ledger). This PR: `bench` (tilt table, skidpad) + the steady-turn load-transfer test + a fix: the tyre's lateral
  force now carries its patch-to-hub moment (rolled too little before; the tilt table caught it). Next: ARCH's call (Scout/Hauler when FORGE lands them).
- **Parameters I would not trust yet:** tyre curve linear to the friction-circle cap (no slide drop: `mu_slide`, `kappa_peak`, `alpha_peak_rad` unused),
  so locked braking and the cornering limit read high; no tyre load sensitivity; roll centre fixed at wheel-centre height by the sliding-strut
  kinematics (real trucks: near the ground or at the spring seat); single-ray contact; `relaxation_length_m`, `slip_damping_time_s` are estimates.
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
