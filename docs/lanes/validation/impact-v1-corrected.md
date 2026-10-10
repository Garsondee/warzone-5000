# Design Impact Matrix v1, corrected table (ARCH's three requests)

The model did not change between this run and `impact-v1.md`: **the table did.** The count therefore says nothing new about the model, and it is decomposed below so the move from 72% to 81% is not mistaken for progress.

## Headline (both thresholds, always)
**56 of 69 scored signs right (81%) at the 0.5% no-change threshold; 58 of 69 (84%) at 1%.** The threshold was not chosen to reach the bar: 0.5% is the one committed in v0, and 1% is reported beside it.

## Where the 72% to 81% came from
| step | right / scored | note |
|---|---|---|
| v1 table, as merged (#152) | 41 / 57 (72%) | |
| remove the "power-limited skidpad gives `~0`" rule | 45 / 57 (79%) | +4: the Hauler's four skidpad levers. ARCH's physics call: cornering drag couples the chassis levers to a power limit, so that rule was wrong. CHASSIS is investigating why the Hauler is power-limited at 0.59 g when the grip bound is near 0.8 g; until then the Hauler's skidpad is a result to explain, not a pass. |
| add step-climb (B11) rows for engine power, first gear, final drive, centre differential | 56 / 69 (81%) | +12 scored, 11 right, 1 wrong. **These rows are lenient: `+` or `~0`** (traction caps what torque can do), so a result fails only if the step climb gets *worse*. They raise the percentage and test little; weigh them accordingly. |

## Acceptance sentence 3, stated honestly
"80% of expected signs right, every dead lever and orphan effect listed" holds **for the 23 of 53 table rows that can be tested today (43%)**, and only with the lenient rows above. The other 30 rows are untested (below): 10 of the 16 benchmarks have no runner (B2, B3, B5, B8, B9, B10, B13, B14, B15, B16), and 17 rows concern levers no vehicle field or lever API entry exists for yet (tracks, turret, tyre pressure and width, damping, travel, wheelbase, fuel tank, gear count, brake thermal mass, torque-curve shape, drag area). No dead lever and no orphan effect among the tested cells. Nothing is tuned.

## The 13 wrong signs, one line each
- Hauler engine power and first gear on B6 (0.0%, table `+`): launch-transient limit, not torque (DRIVE).
- Hauler final drive on B1 (+1.1%, table `-`): shift schedule (DRIVE).
- Hauler ground clearance on B7 (0.0%, table `-`): clearance and centre-of-mass height are separate fields (FORGE).
- Mule centre differential on B11 (-36%, table `+` or `~0`): a limited slip makes the step worse on the Mule; the step test is momentum-dominated (ARCH gap 4), so this is possibly a launch effect, not a corner effect (DRIVE, ARCH).
- Mule ground clearance on B11 (-2.3%, table `+`): inside the momentum noise.
- Mule and Scout ride frequency on B12 (0.0%, +0.5%, table `+`): barely moves a grip-plateau or slide-out limit (CHASSIS).
- Mule and Scout centre-of-mass height and track on B7 (-0.6% to +0.9%, table `~0`): they slide before they tip; sub-1% couplings (all four are right at the 1% threshold).
- Scout tyre friction on B4 (-0.5%, table `-`): brake-limited, so friction is not the limit (DRIVE content).

# Full result

### hauler_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -4.2% ok | +7.8% unlisted | +0.0%  | +0.0%  | +0.0%  |
| centre_diff_limited_slip | -0.5% unlisted | -0.0%  | +0.0% ok | +0.0%  | +7.7% ok | +4.1% unlisted |
| com_height | -0.1%  | +2.0% ok | +1.6% unlisted | -7.5% ok | +0.0%  | -8.9% ok |
| engine_power | -6.2% ok | +0.0%  | +0.0% WRONG (table: +) | +0.0%  | +0.0% ok | -0.3%  |
| final_drive | +1.1% WRONG (table: -) | -0.2%  | -3.1% unlisted | +0.0%  | +0.0% ok | -0.1%  |
| first_gear | -0.6% unlisted | -0.0%  | +0.0% WRONG (table: +) | +0.0%  | +7.7% ok | +0.0%  |
| ground_clearance | -0.1%  | -0.0%  | +0.0%  | +0.0% WRONG (table: -) | +7.7% ok | +0.1%  |
| mass | +6.2% ok | +4.0% ok | -4.7% ok | -0.6% unlisted | +7.7% unlisted | -1.3% unlisted |
| ride_frequency | -0.3%  | -0.0%  | +1.6% unlisted | +0.8% unlisted | +7.7% unlisted | +0.6% ok |
| track_gauge | +0.0%  | +0.0%  | +0.0%  | +8.2% ok | +0.0%  | +7.6% ok |
| tyre_mu | +0.2%  | -2.8% ok | +0.0%  | +0.0%  | +0.0%  | +1.1% ok |

### mule_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -5.7% ok | +2.7% unlisted | +0.0%  | +0.0%  | +0.0%  |
| centre_diff_limited_slip | +0.1%  | -0.0%  | +12.7% ok | +0.0%  | -36.4% WRONG (table: + or ~0) | -0.2%  |
| com_height | +0.0%  | +1.4% ok | -1.8% unlisted | -0.6% WRONG (table: ~0) | +0.0%  | -1.6% ok |
| engine_power | -9.4% ok | -0.1%  | +0.9% ok | +0.0%  | +6.8% ok | +0.0%  |
| final_drive | -1.7% ok | +0.4%  | +1.8% unlisted | +0.0%  | +9.1% ok | +0.0%  |
| first_gear | +0.6% unlisted | +0.0%  | +1.8% ok | +0.0%  | +4.5% ok | +0.0%  |
| ground_clearance | +0.0%  | +0.0%  | +0.0%  | +0.0% ok | -2.3% WRONG (table: +) | -0.0%  |
| mass | +7.6% ok | +4.8% ok | -2.7% ok | -0.0%  | +0.0%  | -0.0%  |
| ride_frequency | -0.1%  | -0.1%  | +0.0%  | +0.1%  | -47.7% unlisted | +0.0% WRONG (table: +) |
| track_gauge | +0.0%  | +0.0%  | +0.0%  | +0.7% WRONG (table: ~0) | +0.0%  | +1.2% ok |
| tyre_mu | +0.0%  | -2.0% ok | +5.5% unlisted | +6.6% unlisted | +0.0%  | +8.0% ok |

### scout_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -5.7% ok | +1.9% unlisted | +0.0%  | +0.0%  | +0.0%  |
| centre_diff_limited_slip | +0.4%  | -0.0%  | +0.0% ok | +0.0%  | +7.1% ok | -0.1%  |
| com_height | -0.5%  | +1.1% ok | +0.0%  | -0.7% WRONG (table: ~0) | +7.1% unlisted | -2.1% ok |
| engine_power | -9.5% ok | +0.1%  | +9.3% ok | +0.0%  | +21.4% ok | +3.3% unlisted |
| final_drive | -2.6% ok | +0.2%  | +7.4% unlisted | +0.0%  | +42.9% ok | +2.4% unlisted |
| first_gear | -0.6% unlisted | -0.0%  | +7.4% ok | +0.0%  | +7.1% ok | -0.0%  |
| ground_clearance | -0.2%  | +0.1%  | +0.0%  | +0.0% ok | +14.3% ok | -0.0%  |
| mass | +6.8% ok | +6.9% ok | -9.3% ok | -0.0%  | +7.1% unlisted | -0.0%  |
| ride_frequency | +0.2%  | -0.3%  | +0.0%  | +0.4%  | +7.1% unlisted | +0.5% WRONG (table: +) |
| track_gauge | +0.0%  | +0.0%  | +0.0%  | +0.9% WRONG (table: ~0) | +0.0%  | +1.6% ok |
| tyre_mu | +0.0%  | -0.5% WRONG (table: -) | +0.0%  | +6.7% unlisted | +7.1% unlisted | +7.3% ok |

**Right signs: 56 of 69 scored (81%) at the 0.5% no-change threshold; 58 of 69 (84%) at 1%.** Both are reported; the threshold is never chosen to reach a bar.

**Tested today: 23 of 53 table rows.** Benchmarks with no runner: B2 (top speed (km/h)), B3 (fuel range (km)), B5 (repeated-stop distance, 5th stop (m)), B8 (soft-ground mobility), B9 (washboard ride roughness (m/s^2 RMS)), B10 (hump settling time (s)), B13 (pivot-turn power or minimum radius at 20 km/h), B14 (ground pressure (kPa) and sinkage in mud (m)), B15 (turret 360-degree traverse time (s)), B16 (gun-laying error while moving (mrad)).

Untested rows (30):
- Engine peak power x B2: benchmark has no runner yet
- Engine peak power x B3: benchmark has no runner yet
- Torque curve shape (peak moved to lower rpm) x B6: no lever in the runner (no lever API entry or no vehicle field)
- First-gear ratio x B2: benchmark has no runner yet
- Final-drive ratio (numerically higher) x B2: benchmark has no runner yet
- Gear count (closer ratios, same span) x B3: no lever in the runner (no lever API entry or no vehicle field)
- Brake thermal mass x B5: no lever in the runner (no lever API entry or no vehicle field)
- Fuel tank capacity x B3: no lever in the runner (no lever API entry or no vehicle field)
- Vehicle mass x B8: benchmark has no runner yet
- Wheelbase x B13: no lever in the runner (no lever API entry or no vehicle field)
- Frontal area x drag coefficient x B2: no lever in the runner (no lever API entry or no vehicle field)
- Frontal area x drag coefficient x B3: no lever in the runner (no lever API entry or no vehicle field)
- Ride frequency (stiffer springs) x B9: benchmark has no runner yet
- Damping ratio x B10: no lever in the runner (no lever API entry or no vehicle field)
- Damping ratio x B9: no lever in the runner (no lever API entry or no vehicle field)
- Suspension travel x B9: no lever in the runner (no lever API entry or no vehicle field)
- Tyre pressure (lower) x B8: no lever in the runner (no lever API entry or no vehicle field)
- Tyre pressure (lower) x B3: no lever in the runner (no lever API entry or no vehicle field)
- Tyre width x B8: no lever in the runner (no lever API entry or no vehicle field)
- Track width x B14: no lever in the runner (no lever API entry or no vehicle field)
- Track width x B8: no lever in the runner (no lever API entry or no vehicle field)
- Track width x B13: no lever in the runner (no lever API entry or no vehicle field)
- Track contact length x B14: no lever in the runner (no lever API entry or no vehicle field)
- Track contact length x B13: no lever in the runner (no lever API entry or no vehicle field)
- Track contact length x B8: no lever in the runner (no lever API entry or no vehicle field)
- Turret mass x B15: no lever in the runner (no lever API entry or no vehicle field)
- Turret mass x B7: no lever in the runner (no lever API entry or no vehicle field)
- Traverse drive effort x B15: no lever in the runner (no lever API entry or no vehicle field)
- Stabiliser rejection x B16: no lever in the runner (no lever API entry or no vehicle field)
- Gun mass (barrel) x B16: no lever in the runner (no lever API entry or no vehicle field)

Dead levers (moved no runnable benchmark): []

Orphan benchmarks (no lever moved them): []
