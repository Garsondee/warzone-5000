# Design Impact Matrix v2 (after FORGE's component mass budget, #176)

Same runner, same levers (`levers::apply_both`), same table as `impact-v1-corrected.md`; only the vehicles changed (hull mass is now structure plus parts; `engine_peak_power` carries mass, centre of mass, axle loads and spring rates; the `mass` lever probes structure mass only; hull inertia comes from the parts; rig hashes moved on purpose). Data: `docs/lanes/validation/impact-v2.json`.

## Headline (both thresholds, always)
**49 of 65 scored signs right (75%) at the 0.5% no-change threshold; 50 of 65 (77%) at 1%.** Was 56 of 69 (81%) and 58 of 69 (84%) in the corrected v1. **The Hauler's gradeability (B6) is now tagged `transient`** (DRIVE: a launch-transient limit that jumps by 10% or more with small inertia changes): its 11 cells are shown and left out of the count (4 of them would have scored: 2 right, 2 wrong). Counting them: 51 of 69 (74%) at 0.5%, 52 of 69 (75%) at 1%. Tested today: still 23 of 53 table rows; the untested list (10 of 16 benchmarks without a runner, 24 rows for levers without a runner) is unchanged and is in the full result below.

## What changed against v1, and why
- **The score fell, and the cause is the Hauler's step climb, which is mostly resolution noise.** The step test bisects in quanta of about 7.8 mm (0.102, 0.109, 0.117 m on the Hauler: 6.7% per quantum), and the Hauler's base step moved from 0.102 to 0.117 m with the mass budget, so five Hauler B11 cells now read -6.7% or -13.3% (one or two quanta down) where they read 0 or +7.7% before. Four of the five are lenient rows that fail only on a loss. That is one quantum of the bisection, not evidence of a torque effect. **Ask for ARCH:** echo the step bisection tolerance in the result (`step_resolution_m`) so the matrix can treat a change of one quantum as unresolved.
- **Engine power now carries mass, so it moves braking, side slope and skidpad (new, physically right).** Hauler +10% engine power adds 253 kg (+4%) and lowers the centre of mass slightly (1.3 to 1.2 m at the precision I printed): braking distance +1.7% (the Mule +2.3%, the Scout +1.9%), side-slope angle +1.4%, skidpad +1.4%. The table has no row for these (unlisted): a bigger engine lengthening a brake-limited stop is the mass coupling the table's mass row already expects.
- **Baselines moved:** Hauler grade 0.375 to 0.416, Hauler skidpad 0.59 to 0.571 g, Mule step 0.344 to 0.305 m, Scout skidpad 0.738 to 0.755 g, Scout step 0.109 to 0.117 m.
- **The Mule's ground clearance now agrees with the table** (+2.6% on the step; was -2.3%); the Scout's no longer does (0.0%; was +14%), which makes it a dead lever on that vehicle. The Scout's limited-slip centre differential is also dead (its grade is engine-limited).
- No orphan benchmark. Dead levers: Scout `centre_diff_limited_slip`, Scout `ground_clearance`.

## VIEWER's question: do the Hauler's non-monotonic rows come from the new mass coupling?
A sweep of Hauler engine power (0.8 to 1.3 of baseline, `--factor` and `--only`):
| power | mass kg | 0-48 s | braking m | grade | step m | skidpad g | side slope rad |
|---|---|---|---|---|---|---|---|
| 0.8 | 5734 | 12.51 | 18.17 | 0.445 | 0.102 | 0.553 | 0.569 |
| 0.9 | 5987 | 11.90 | 18.52 | 0.434 | 0.109 | 0.562 | 0.577 |
| 1.0 | 6240 | 11.36 | 18.86 | 0.416 | 0.117 | 0.571 | 0.585 |
| 1.1 | 6493 | 10.94 | 19.18 | 0.369 | 0.102 | 0.578 | 0.593 |
| 1.2 | 6746 | 10.60 | 19.50 | 0.381 | 0.109 | 0.586 | 0.601 |
| 1.3 | 6999 | 10.31 | 19.82 | 0.357 | 0.109 | 0.593 | 0.608 |
**Braking, skidpad, side slope and 0-48 are monotone** with power, and the braking rise is the mass coupling (brake torque is held, mass grows 4% per 10% power). **Gradeability falls with power (0.445 to 0.357), which is the mass, and it is not smooth (0.369 at 1.1, 0.381 at 1.2): the launch transient. The step height is not monotone (0.117 at 1.0, 0.102 at 1.1) and moves in 7.8 mm quanta: resolution, not physics.** So the jumps VIEWER saw are the transient and the quantisation, not an error in the new coupling. The skidpad rises with power, which is what a power-limited run does.

## Still-wrong signs (not counting the transient Hauler B6 cells), one line each
- Hauler B11 for centre differential, engine power, final drive, first gear, ground clearance (-6.7% to -13.3%; table `+` or `~0`): one or two bisection quanta (above).
- Hauler final drive on B1 (+2.6%, table `-`): shift schedule (DRIVE).
- Hauler ground clearance on B7 (0.0%, table `-`): clearance and centre-of-mass height are separate fields (FORGE).
- Mule centre differential on B11 (-17.9%): a limited slip makes the step worse; momentum-dominated (ARCH gap 4), possibly a launch effect (DRIVE).
- Scout ground clearance on B11 (0.0%, table `+`): dead on this vehicle now.
- Mule and Scout ride frequency on B12 (0.0%, +0.2%, table `+`): barely moves a grip-plateau or slide-out limit (CHASSIS).
- Mule and Scout centre-of-mass height and track on B7 (-0.7% to +0.9%, table `~0`): they slide before they tip; sub-1% couplings (all four are right at 1%).
- Scout tyre friction on B4 (-0.2%, table `-`): brake-limited, friction is not the limit (DRIVE content).

# Full result

### hauler_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -4.2% ok | -1.4% transient | +0.0%  | +0.0%  | +0.0%  |
| centre_diff_limited_slip | -0.0%  | +0.0%  | +1.4% transient | +0.0%  | -6.7% WRONG (table: + or ~0) | +6.2% unlisted |
| com_height | +0.4%  | +2.0% ok | +1.4% transient | -7.5% ok | -6.7% unlisted | -8.5% ok |
| engine_power | -3.7% ok | +1.7% unlisted | -11.3% transient (table: +) | +1.4% unlisted | -13.3% WRONG (table: + or ~0) | +1.4% unlisted |
| final_drive | +2.6% WRONG (table: -) | +0.4%  | -2.8% transient | +1.3% unlisted | -6.7% WRONG (table: + or ~0) | +1.3% unlisted |
| first_gear | +0.5% unlisted | +0.5%  | -2.8% transient (table: +) | +1.3% unlisted | -13.3% WRONG (table: + or ~0) | +1.5% unlisted |
| ground_clearance | -0.4%  | -0.0%  | -8.5% transient | +0.0% WRONG (table: -) | -13.3% WRONG (table: +) | +0.0%  |
| mass | +2.4% ok | +1.9% ok | -1.4% transient | -1.8% unlisted | +0.0%  | -2.0% unlisted |
| ride_frequency | -0.5% unlisted | -0.1%  | +0.0% transient | +0.8% unlisted | +6.7% unlisted | +0.8% ok |
| track_gauge | +0.0%  | +0.0%  | +0.0% transient | +8.2% ok | +0.0%  | +8.6% ok |
| tyre_mu | -0.6% unlisted | -2.8% ok | -1.4% transient | +0.0%  | -13.3% unlisted | +0.8% ok |

### mule_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -5.7% ok | +2.7% unlisted | +0.0%  | +0.0%  | +0.0%  |
| centre_diff_limited_slip | -0.1%  | +0.0%  | +12.7% ok | +0.0%  | -17.9% WRONG (table: + or ~0) | -0.2%  |
| com_height | -0.2%  | +1.4% ok | -1.8% unlisted | -0.7% WRONG (table: ~0) | +12.8% unlisted | -1.6% ok |
| engine_power | -7.0% ok | +2.3% unlisted | +0.9% ok | +0.0%  | +28.2% ok | +0.2%  |
| final_drive | -1.1% ok | +2.9% unlisted | +1.8% unlisted | +0.0%  | +23.1% ok | +0.1%  |
| first_gear | +1.4% unlisted | +0.4%  | +1.8% ok | +0.0%  | +20.5% ok | +0.1%  |
| ground_clearance | -0.3%  | -0.0%  | +0.0%  | +0.0% ok | +2.6% ok | -0.0%  |
| mass | +3.5% ok | +1.9% ok | -1.8% ok | -0.1%  | +12.8% unlisted | -0.2%  |
| ride_frequency | -0.1%  | -0.0%  | +0.0%  | +0.1%  | -43.6% unlisted | +0.0% WRONG (table: +) |
| track_gauge | +0.0%  | +0.0%  | +0.0%  | +0.7% WRONG (table: ~0) | +0.0%  | +1.2% ok |
| tyre_mu | +0.0%  | -2.0% ok | +5.5% unlisted | +6.6% unlisted | +10.3% unlisted | +8.0% ok |

### scout_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -5.6% ok | +1.9% unlisted | +0.0%  | +0.0%  | +0.0%  |
| centre_diff_limited_slip | -0.4%  | +0.0%  | +0.0% ok | +0.0%  | +0.0% ok | -0.1%  |
| com_height | +0.0%  | +1.1% ok | +0.0%  | -0.7% WRONG (table: ~0) | -6.7% unlisted | -2.9% ok |
| engine_power | -8.1% ok | +1.9% unlisted | +7.4% ok | +0.0%  | +0.0% ok | -1.4% unlisted |
| final_drive | -2.2% ok | +0.8% unlisted | +5.6% unlisted | +0.0%  | +6.7% ok | +2.8% unlisted |
| first_gear | -0.4%  | +0.6% unlisted | +5.6% ok | +0.0%  | +6.7% ok | -1.9% unlisted |
| ground_clearance | -0.4%  | +0.0%  | +0.0%  | +0.0% ok | +0.0% WRONG (table: +) | +0.0%  |
| mass | +4.3% ok | +4.0% ok | -5.6% ok | -0.0%  | +0.0%  | +0.2%  |
| ride_frequency | -0.5% unlisted | -0.1%  | +0.0%  | +0.4%  | +0.0%  | +0.2% WRONG (table: +) |
| track_gauge | +0.0%  | +0.0%  | +0.0%  | +0.9% WRONG (table: ~0) | +0.0%  | +1.7% ok |
| tyre_mu | +0.0%  | -0.2% WRONG (table: -) | +0.0%  | +6.8% unlisted | +0.0%  | +5.8% ok |

**Right signs: 49 of 65 scored (75%) at the 0.5% no-change threshold; 50 of 65 (77%) at 1%.** Both are reported; the threshold is never chosen to reach a bar.

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

Dead levers (moved no runnable benchmark): [("scout_4x4", "centre_diff_limited_slip"), ("scout_4x4", "ground_clearance")]

Orphan benchmarks (no lever moved them): []
