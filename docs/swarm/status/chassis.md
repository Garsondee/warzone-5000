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
- Owner direction via ARCH (13:40Z): three trucks + proving ground. (1) this PR: `modes` (hop, hop-on-stop, heave; substeps = max(declared, rule)); Mule needs 5 (declares 4). Next: (2) ledger, (3) skidpad + side-slope benches.
- **Parameters I would not trust yet:** tyre curve is linear to the friction-circle cap (no slide drop: `mu_slide`, `kappa_peak`, `alpha_peak_rad` unused) so locked
  braking and the cornering limit read high; no tyre load sensitivity (heavy outer tyres grip too well: the Hauler's rollover margin reads low); single-ray contact (step and hole edges too harsh);
  `relaxation_length_m`, `slip_damping_time_s` are estimates; unsprung inertia uses the previous substep's hull acceleration (translation only); brakes come from the stand-in powertrain.
- **6x6:** expressible today if every axle is independent (any number of stations, Ackermann about the unsteered axles, one drive output per wheel); a walking beam or
  inboard leaf (`LinkageDef`) is refused (`required_features`) until CHASSIS implements linkages (a weighted shared spring on the travel coordinates, about one PR).

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
