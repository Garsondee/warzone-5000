# Design Impact Matrix v1 (after FORGE's levers, DRIVE launch/hill hold, CHASSIS sinking tyre and tyre load sensitivity)

Run: `w5k validation impact --out DIR` on the integration head (contract 0.3, #147 included); levers through `w5k_forge::levers::apply_both` (brake torque is `brake_axle_torque`; the centre differential goes from open to limited slip, bias 2, PROVISIONAL). Baselines agree with ARCH's proving numbers: braking 14.0 / 12.0 / 18.9 m, grade 31.6 / 64.5 / 37.5 % (Scout / Mule / Hauler). The tornado data for VIEWER is `docs/lanes/validation/impact-v1.json` (`w5k viewer tornado docs/lanes/validation/impact-v1.json --out tornado.png`); it now also carries benchmark and lever names, `perturb_pct`, `deadband_pct` and `acceptance_pct`.

## Sign count: 41 of 57 (72%), unchanged from v0; target 80%
No dead lever, no orphan benchmark. The count depends on the "no change" threshold: 72% at 0.5% (committed), 79% at 1%, 67% at 2%. Nothing is tuned; only 6 of the 16 benchmarks run (B1 is 0-48 km/h).

## The 16 wrong signs, one line each
| vehicle, lever, benchmark | change | cause (for the lane named) |
|---|---|---|
| Hauler engine power, first gear on B6 | 0.0%, 0.0% (table `+`) | Hauler gradeability is a launch-transient limit, not a torque limit (DRIVE) |
| Hauler final drive on B1 | +1.1% (table `-`) | shift schedule: a numerically higher final drive reaches the upshift earlier (DRIVE) |
| Hauler skidpad vs com height, track, ride frequency, tyre mu | -8.9%, +7.6%, +0.6%, +1.1% (table `~0`) | labelled power-limited yet moves with chassis levers: the label or the limit is wrong (CHASSIS, ARCH) |
| Hauler ground clearance on B7 | 0.0% (table `-`) | `ground_clearance_m` and `com_height_m` are separate fields: clearance never raises the centre of mass (FORGE) |
| Mule ground clearance on B11 | -2.3% (table `+`) | step test is momentum-dominated (ARCH gap 4), a 2% effect is inside that noise (WORLD, ARCH) |
| Mule, Scout ride frequency on B12 | 0.0%, +0.5% (table `+`) | skidpad limit is grip-plateau or slide-out, and a stiffer ride barely changes tyre load sharing (CHASSIS) |
| Mule, Scout com height, track on B7 | -0.6%, +0.7%, -0.7%, +0.9% (table `~0`) | they slide before they tip; sub-1% coupling (a threshold artifact: 0.5% was kept) |
| Scout tyre mu on B4 | -0.5% (table `-`) | the Scout is brake-limited (0.73 g against mu 0.85), so friction is not the limit (DRIVE content: brake torque) |

## Still unlisted (moved, no table row)
Step climb moves strongly with levers the table does not tie to it: Mule centre differential -36%, Mule ride frequency -48%, Scout engine power +21%, Scout final drive +43%. It is decided by run-up momentum (ARCH gap 4); the table needs momentum rows, or the test a fixed approach speed.

## Full result
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
