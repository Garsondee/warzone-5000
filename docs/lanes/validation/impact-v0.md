# Design Impact Matrix v0: first runs (scout, mule, hauler)

`w5k validation impact --out DIR` perturbs eleven levers on each vehicle through FORGE's lever API (`w5k_forge::levers::apply_both`: ten at +10%, plus the centre differential swapped from open to limited slip, bias 2, PROVISIONAL), runs the proving ground on every variant (about 35 s in total), compares the sign of each change with `docs/validation/IMPACT-MATRIX.md` and writes `impact.json` and `impact.md`. Nothing is tuned: this is a measurement.

**Coverage today:** 6 of the 16 benchmarks (B1 is timed 0-48 km/h, not 0-32; B4, B6, B7, B11, B12). Ride (B9, B10), fuel (B3), soft ground (B8), pivot (B13) and the turret benchmarks have no runner, and the size check (within a factor of 2) is not implemented: only two table rows carry a number.
**Threshold:** a change under 0.5% counts as "no change" (`~0`).

## Result: 41 of 57 scored signs agree with the table (72%), below the 80% target

Cells are the change in the benchmark for the lever's perturbation; `ok` agrees with the table, `WRONG` does not (the table's sign follows), `unlisted` moved but the table has no row for it (reported, not scored). No dead lever and no orphan benchmark.

**The score depends on the "no change" threshold:** 72% at 0.5%, 79% at 1%, 67% at 2%. Six of the sixteen wrong entries are couplings below 1% (a sliding vehicle's side-slope angle moving by 0.6 to 0.9% with centre-of-mass height or track). I kept 0.5% because moving the threshold after seeing the results would be tuning the judge.

### hauler_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -4.2% ok | +7.8% unlisted | +0.0%  | +0.0%  | +0.0%  |
| centre_diff_limited_slip | -0.5% unlisted | -0.0%  | +0.0% ok | +0.0%  | +7.7% unlisted | +4.1% unlisted |
| com_height | -0.1%  | +2.0% ok | +1.6% unlisted | -7.5% ok | +0.0%  | -8.9% WRONG (table: ~0) |
| engine_power | -6.2% ok | +0.0%  | +0.0% WRONG (table: +) | +0.0%  | +0.0%  | -0.3%  |
| final_drive | +1.1% WRONG (table: -) | -0.2%  | -3.1% unlisted | +0.0%  | +0.0%  | -0.1%  |
| first_gear | -0.6% unlisted | -0.0%  | +0.0% WRONG (table: +) | +0.0%  | +7.7% unlisted | +0.0%  |
| ground_clearance | -0.1%  | -0.0%  | +0.0%  | +0.0% WRONG (table: -) | +7.7% ok | +0.1%  |
| mass | +6.2% ok | +4.0% ok | -4.7% ok | -0.6% unlisted | +7.7% unlisted | -1.3% unlisted |
| ride_frequency | -0.3%  | -0.0%  | +1.6% unlisted | +0.8% unlisted | +7.7% unlisted | +0.6% WRONG (table: ~0) |
| track_gauge | +0.0%  | +0.0%  | +0.0%  | +8.2% ok | +0.0%  | +7.6% WRONG (table: ~0) |
| tyre_mu | +0.2%  | -2.8% ok | +0.0%  | +0.0%  | +0.0%  | +1.1% WRONG (table: ~0) |

### mule_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -5.7% ok | +2.7% unlisted | +0.0%  | +0.0%  | +0.0%  |
| centre_diff_limited_slip | +0.1%  | -0.0%  | +12.7% ok | +0.0%  | -36.4% unlisted | -0.2%  |
| com_height | +0.0%  | +1.4% ok | -1.8% unlisted | -0.6% WRONG (table: ~0) | +0.0%  | -1.6% ok |
| engine_power | -9.4% ok | -0.1%  | +0.9% ok | +0.0%  | +6.8% unlisted | +0.0%  |
| final_drive | -1.7% ok | +0.4%  | +1.8% unlisted | +0.0%  | +9.1% unlisted | +0.0%  |
| first_gear | +0.6% unlisted | +0.0%  | +1.8% ok | +0.0%  | +4.5% unlisted | +0.0%  |
| ground_clearance | +0.0%  | +0.0%  | +0.0%  | +0.0% ok | -2.3% WRONG (table: +) | -0.0%  |
| mass | +7.6% ok | +4.8% ok | -2.7% ok | -0.0%  | +0.0%  | -0.0%  |
| ride_frequency | -0.1%  | -0.1%  | +0.0%  | +0.1%  | -47.7% unlisted | +0.0% WRONG (table: +) |
| track_gauge | +0.0%  | +0.0%  | +0.0%  | +0.7% WRONG (table: ~0) | +0.0%  | +1.2% ok |
| tyre_mu | +0.0%  | -2.0% ok | +5.5% unlisted | +6.6% unlisted | +0.0%  | +8.0% ok |

### scout_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -5.7% ok | +1.9% unlisted | +0.0%  | +0.0%  | +0.0%  |
| centre_diff_limited_slip | +0.4%  | -0.0%  | +0.0% ok | +0.0%  | +7.1% unlisted | -0.1%  |
| com_height | -0.5%  | +1.1% ok | +0.0%  | -0.7% WRONG (table: ~0) | +7.1% unlisted | -2.1% ok |
| engine_power | -9.5% ok | +0.1%  | +9.3% ok | +0.0%  | +21.4% unlisted | +3.3% unlisted |
| final_drive | -2.6% ok | +0.2%  | +7.4% unlisted | +0.0%  | +42.9% unlisted | +2.4% unlisted |
| first_gear | -0.6% unlisted | -0.0%  | +7.4% ok | +0.0%  | +7.1% unlisted | -0.0%  |
| ground_clearance | -0.2%  | +0.1%  | +0.0%  | +0.0% ok | +14.3% ok | -0.0%  |
| mass | +6.8% ok | +6.9% ok | -9.3% ok | -0.0%  | +7.1% unlisted | -0.0%  |
| ride_frequency | +0.2%  | -0.3%  | +0.0%  | +0.4%  | +7.1% unlisted | +0.5% WRONG (table: +) |
| track_gauge | +0.0%  | +0.0%  | +0.0%  | +0.9% WRONG (table: ~0) | +0.0%  | +1.6% ok |
| tyre_mu | +0.0%  | -0.5% WRONG (table: -) | +0.0%  | +6.7% unlisted | +7.1% unlisted | +7.3% ok |

**Right signs: 41 of 57 scored (72%).**

Dead levers (moved no runnable benchmark): []

Orphan benchmarks (no lever moved them): []

## What the wrong signs say (each is for the lane that owns the model; none is "fixed" here)
History: the first run (own RON edits) scored 65%; regime rules (side slope, skidpad: see "Regimes" in `IMPACT-MATRIX.md`) 69%; after contract 0.3 and FORGE's lever API (brake torque authored, mass holds brake torque) 72%.
1. **Fixed since the first run:** mass now lengthens a brake-limited stop on all three vehicles (+4% to +7%); the brake lever is an authored axle torque. The ground-clearance lever now helps the step climb on the Scout (+14%) and Hauler (+7.7%).
2. **The Mule's gradeability responds to the centre differential (+12.7% open to limited slip), as DRIVE found; the Scout and the Hauler do not.** More engine torque or a lower first gear still does nothing for the Hauler's grade (0.0%; table: `+`): DRIVE reports a launch-transient limit there.
3. **The Hauler's skidpad is labelled power-limited but moves with track (+7.6%) and centre-of-mass height (-8.9%).** The label or the limit is wrong (CHASSIS and ARCH).
4. **Ground clearance on the Mule lowers the step climb (-2.3%; table: `+`), and on the Hauler it does not move the side-slope limit** because `ground_clearance_m` and `com_height_m` are separate fields (FORGE).
5. **Ride frequency barely moves the skidpad limit (0.0 to +0.5%; table: `+`):** the earlier -34% on the Scout is gone; the sign is weak (CHASSIS).
6. **A numerically higher final drive slows the Hauler's launch by 1.1% (table: `-`)**; the Scout's tyre-friction lever moves its braking by only -0.5% (brake-limited, so the tyre is not the limit).
7. **Large unlisted effects on the step climb** (ride frequency -48% on the Mule; final drive +43% on the Scout): the step test is decided by run-up momentum (ARCH gap 4), not by the quasi-static limit the table assumes. The table needs momentum rows, or the test a fixed approach speed.
