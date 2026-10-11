# Design Impact Matrix v3 (step bench of #182; ARCH's unresolved-cell rule adopted)

Same runner, levers (`levers::apply_both`) and table as v2. Changes: the step-climb bench now holds 2 m/s with the PI driver, bisects to 0.5 mm, takes the median of three run-ups and echoes `step_run_up_spread_m`, `step_resolution_m` and `approach_speed_m_s`; and the matrix adopts ARCH's rule from `docs/swarm/requests/arch-validation-step-noise.md` as written: **a cell is unresolved (shown, not counted) when |change in metres| is at most twice the larger of the two designs' `step_run_up_spread_m`, and never below `step_resolution_m`** (`impact::is_unresolved`; 29 of the 198 cells, listed in `impact-v3.json` with `unresolved: true`). The hand-kept `IMPACT_TRANSIENT` list (Hauler B6) is kept for now; the same rule replaces it once gradeability echoes a spread. Data: `docs/lanes/validation/impact-v3.json`.

## Correction to my v2 note
v2 read the Hauler's step-climb cells as one bisection quantum (7.8 mm). That was wrong as the main cause: ARCH refined the bisection to 0.25 mm and the Hauler engine-power cell stayed at -6.9%. The main cause is the run-up phase (1 to 6% between run-up lengths, one run-up in thirteen 30% off), because a single ray under each tyre meets the step as an impulse (`docs/lanes/arch/proving.md`, "Step climb"). The resolution was a contributor, not the cause; the sweep of engine power is jagged for the run-up reason.

## Headline: the no-change threshold is 0.5% (committed in v0); the count at 1% is beside it; neither is chosen to reach a bar
| way of counting | right / scored at 0.5% | at 1% |
|---|---|---|
| every cell counted, transient cells left out (as v2) | 53 of 65 (82%) | 54 of 65 (83%) |
| **noise-aware** (transient and unresolved cells left out) | 42 of 51 (82%) | 43 of 51 (84%) |
| without B11, the step-climb cells (transient cells left out) | 41 of 50 (82%) | 42 of 50 (84%) |
| no cell left out at all | 55 of 69 (80%) | 56 of 69 (81%) |
These agree with ARCH's own run (53 of 65, 54 of 65, 41 of 50). **Do not read 82% as "the bar is met" on the step row.** The step row's big changes are regime switches, not smooth responses: the Mule's step height moves +22% to +45% for torque levers (centre differential +22%, engine power +28%, first gear +39%, final drive +45%), +37% for centre-of-mass height, and -35% for a 10% change of tyre friction or of ride frequency: the climb switches between bouncing over the edge and driving over it. The Scout shows the same (+28% to +39% for engine power, gears and mass). Their sign agreement with the table is not evidence of anything. The rest of the matrix is steady: without B11 the count is 41 of 50 in both v2 and v3.

## Dead levers and orphans
- Dead: Mule `ground_clearance` (it moves nothing on a runnable benchmark: its only change is -0.3% on the step, unresolved). In v2 the Scout's ground clearance and centre differential were dead; in v3 they respond only inside the step bench's noise, which the rule marks unresolved, so they are not dead by the counting rule but have no resolved effect.
- Orphan benchmarks: none. 23 of the 53 table rows are tested; 10 of the 16 benchmarks have no runner (B2, B3, B5, B8, B9, B10, B13 to B16) and 24 rows concern levers with no runner (unchanged: see the full result).

## The 12 wrong signs that remain (transient Hauler B6 cells aside), each with a reason
- Hauler final drive on B1 (+2.6%, table `-`): shift schedule; a numerically higher final drive reaches the upshift earlier (DRIVE).
- Hauler ground clearance on B7 (0.0%, table `-`): clearance and centre-of-mass height are separate fields (FORGE).
- Mule ride frequency on B12 (0.0%, table `+`) and Scout (+0.2%): barely moves a grip-plateau or slide-out limit (CHASSIS).
- Mule and Scout centre-of-mass height and track on B7 (-0.7% to +0.9%, table `~0`): they slide before they tip; sub-1% couplings (all four are right at 1%).
- Scout tyre friction on B4 (-0.2%, table `-`): brake-limited; friction is not the limit (DRIVE content).
- Hauler final drive, Mule and Scout ground clearance on B11 (-3.3%, -0.3%, -1.7%; table `+` or `~0` for the gear and `+` for clearance): **unresolved** (inside the step bench's noise), shown and not counted in the noise-aware headline.

# Full result

### hauler_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -4.2% ok | -1.4% transient | +0.0%  | +0.0% unresolved | +0.0%  |
| centre_diff_limited_slip | -0.0%  | +0.0%  | +1.4% transient | +0.0%  | +0.0% unresolved | +6.2% unlisted |
| com_height | +0.4%  | +2.0% ok | +1.4% transient | -7.5% ok | -2.6% unresolved | -8.5% ok |
| engine_power | -3.7% ok | +1.7% unlisted | -11.3% transient (table: +) | +1.4% unlisted | +4.4% unresolved | +1.4% unlisted |
| final_drive | +2.6% WRONG (table: -) | +0.4%  | -2.8% transient | +1.3% unlisted | -3.3% unresolved (table: + or ~0) | +1.3% unlisted |
| first_gear | +0.5% unlisted | +0.5%  | -2.8% transient (table: +) | +1.3% unlisted | +0.7% unresolved | +1.5% unlisted |
| ground_clearance | -0.4%  | -0.0%  | -8.5% transient | +0.0% WRONG (table: -) | +1.1% unresolved | +0.0%  |
| mass | +2.4% ok | +1.9% ok | -1.4% transient | -1.8% unlisted | -2.6% unresolved | -2.0% unlisted |
| ride_frequency | -0.5% unlisted | -0.1%  | +0.0% transient | +0.8% unlisted | -5.1% unresolved | +0.8% ok |
| track_gauge | +0.0%  | +0.0%  | +0.0% transient | +8.2% ok | +0.0% unresolved | +8.6% ok |
| tyre_mu | -0.6% unlisted | -2.8% ok | -1.4% transient | +0.0%  | +2.2% unresolved | +0.8% ok |

### mule_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -5.7% ok | +2.7% unlisted | +0.0%  | +0.0% unresolved | +0.0%  |
| centre_diff_limited_slip | -0.1%  | +0.0%  | +12.7% ok | +0.0%  | +22.3% ok | -0.2%  |
| com_height | -0.2%  | +1.4% ok | -1.8% unlisted | -0.7% WRONG (table: ~0) | +37.3% unlisted | -1.6% ok |
| engine_power | -7.0% ok | +2.3% unlisted | +0.9% ok | +0.0%  | +28.1% unresolved | +0.2%  |
| final_drive | -1.1% ok | +2.9% unlisted | +1.8% unlisted | +0.0%  | +45.5% unresolved | +0.1%  |
| first_gear | +1.4% unlisted | +0.4%  | +1.8% ok | +0.0%  | +39.4% unresolved | +0.1%  |
| ground_clearance | -0.3%  | -0.0%  | +0.0%  | +0.0% ok | -0.3% unresolved (table: +) | -0.0%  |
| mass | +3.5% ok | +1.9% ok | -1.8% ok | -0.1%  | -1.5% unresolved | -0.2%  |
| ride_frequency | -0.1%  | -0.0%  | +0.0%  | +0.1%  | -34.7% unlisted | +0.0% WRONG (table: +) |
| track_gauge | +0.0%  | +0.0%  | +0.0%  | +0.7% WRONG (table: ~0) | +0.0% unresolved | +1.2% ok |
| tyre_mu | +0.0%  | -2.0% ok | +5.5% unlisted | +6.6% unlisted | -35.1% unlisted | +8.0% ok |

### scout_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -5.6% ok | +1.9% unlisted | +0.0%  | +0.0% unresolved | +0.0%  |
| centre_diff_limited_slip | -0.4%  | +0.0%  | +0.0% ok | +0.0%  | +0.7% unresolved | -0.1%  |
| com_height | +0.0%  | +1.1% ok | +0.0%  | -0.7% WRONG (table: ~0) | +0.0% unresolved | -2.9% ok |
| engine_power | -8.1% ok | +1.9% unlisted | +7.4% ok | +0.0%  | +34.4% unresolved | -1.4% unlisted |
| final_drive | -2.2% ok | +0.8% unlisted | +5.6% unlisted | +0.0%  | +39.1% unresolved | +2.8% unlisted |
| first_gear | -0.4%  | +0.6% unlisted | +5.6% ok | +0.0%  | +39.1% unresolved | -1.9% unlisted |
| ground_clearance | -0.4%  | +0.0%  | +0.0%  | +0.0% ok | -1.7% unresolved (table: +) | +0.0%  |
| mass | +4.3% ok | +4.0% ok | -5.6% ok | -0.0%  | +28.4% unresolved | +0.2%  |
| ride_frequency | -0.5% unlisted | -0.1%  | +0.0%  | +0.4%  | +32.8% unresolved | +0.2% WRONG (table: +) |
| track_gauge | +0.0%  | +0.0%  | +0.0%  | +0.9% WRONG (table: ~0) | +0.0% unresolved | +1.7% ok |
| tyre_mu | +0.0%  | -0.2% WRONG (table: -) | +0.0%  | +6.8% unlisted | +1.7% unresolved | +5.8% ok |

**Right signs, three ways (the no-change threshold is 0.5%, the one committed in v0; the count at 1% is beside it, and neither is chosen to reach a bar):**

- every cell counted, transient cells left out: 53 of 65 (82%) at 0.5%; 54 of 65 (83%) at 1%
- noise-aware (transient and unresolved cells left out): 42 of 51 (82%) at 0.5%; 43 of 51 (84%) at 1%
- without B11 (the step-climb cells), transient cells left out: 41 of 50 (82%) at 0.5%; 42 of 50 (84%) at 1%
- no cell left out at all: 55 of 69 (80%) at 0.5%; 56 of 69 (81%) at 1%

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

Dead levers (moved no runnable benchmark): [("mule_4x4", "ground_clearance")]

Orphan benchmarks (no lever moved them): []
