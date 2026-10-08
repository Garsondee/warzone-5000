# Lane DRIVE: an engine, a gearbox and brakes whose choices change what the vehicle can do

## Mission
When you have succeeded, picking a different engine curve, gear spread, final drive or brake changes the 0-32 km/h time, the top speed, the hill start, the fuel range and the brake fade by amounts a mechanical engineer would predict, and a shift, a
stall or a faded brake can be explained from the numbers. You build the powertrain and the brakes behind the `DrivePort` contract: engine with a real torque map and inertia, clutch or torque converter, gearbox with shift logic, differentials or a tracked steering
unit, brakes with heat, fuel. The owner will see a dyno curve, a shift diagram, a speed-in-gear table against the M998 and a brake-fade plot.

## You own  (the CI lane guard enforces it)
`crates/w5k_drive/**`, `content/physics/drive/**`, `crates/w5k_tools/src/cmd/drive.rs`; always `docs/swarm/status/drive.md`, `docs/swarm/requests/drive-*.md`, `docs/theory/drive.md`, `docs/lanes/drive/**`, `spikes/drive/**`.

## You read, never edit
`crates/w5k_contract` (`DrivePort`, `DriveInputs`, `ShaftState`, `DriveTelemetry`, `DrivetrainDef` and its parts in `rig.rs`), `docs/architecture/*`, `docs/validation/METHOD.md`, `docs/theory/the-physics-of-a-time-trial.md` (power versus force; the style we want).

## Stand-ins you start on
`ConstantTorquePowertrain` (to see what a port user expects); `box_truck()` and `box_tank()` rigs (their `DrivetrainDef`s are valid examples: converter, four gears, open diffs; steering unit). You write your own **lumped vehicle bench** (a point mass on N shafts: `F = sum(T_i)/r - rolling - drag`, shaft inertias reflected) to close the loop; CHASSIS provides the real wheels later.

## Settling round (first hours; then stop for review)
1. **Spike S-D, stiff couplings** (`docs/lanes/drive/spike-d.md`): engine flywheel plus a clutch or torque converter plus a heavy output inertia at 240 to 480 Hz. Find a formulation that is stable and does not need microsecond steps: a clutch as a torque-limited friction element with a regularised sign (and a stick/slip rule), a converter from a K-factor and a torque-ratio curve (`T_pump = (omega_pump / K)^2`, `T_turbine = TR(speed ratio) T_pump`), a lock-up clutch. Report the substep each needs and the shift transient (torque interruption) it produces.
2. **Design note** (one page): the engine model (`T(rpm, throttle) = throttle T_full(rpm) - (1 - throttle) T_drag(rpm)`, idle controller, rev limiter, governor, inertia); the coupling formulation from the spike; gearbox (ratios, efficiency, automatic shift map with hysteresis, shift time); differentials (an open differential needs no constraint: it splits torque 50/50 and each wheel integrates its own speed; locked; limited slip as a torque bias); transfer case; the tracked steering unit models (clutch-brake, controlled differential, hydrostatic) as torque splits with the steer demand; brakes (friction torque against temperature, heat in = braking power, heat out = cooling growing with speed, fade curve, parking brake, the "never reverse a stopped shaft" rule); fuel (BSFC map to kg/s, range as an integral). State how `DriveInputs` and `ShaftState` suffice, or what the contract lacks.
3. **CCRs** you expect: anything `DrivePort`, `ShaftState` or `DrivetrainDef` lacks (e.g. clutch pedal for manuals, engine start/stop, temperature inputs).

## Build order  (named tests)
1. Engine: `engine_full_load_torque_matches_the_curve`, `engine_power_equals_torque_times_omega`, `engine_idles_at_idle_rpm_with_no_load`, `rev_limiter_holds_the_redline`, `engine_spin_up_time_matches_inertia_over_torque`, `closed_throttle_gives_the_stated_engine_braking`.
2. Coupling: `converter_multiplies_torque_at_stall_by_the_stall_ratio`, `converter_torque_ratio_falls_to_one_at_the_coupling_point`, `clutch_never_transmits_more_than_its_capacity`, `lockup_removes_slip`.
3. Gearbox: `speed_in_each_gear_at_redline_matches_the_ratio_arithmetic` (`v = omega_engine r / (overall ratio)`), `automatic_upshifts_at_the_stated_rpm_and_downshifts_with_hysteresis`, `shift_interrupts_torque_for_the_stated_time`, `reverse_gears_work`.
4. Driveline: `open_differential_splits_torque_equally_and_creates_no_torque`, `locked_differential_forces_equal_speeds`, `limited_slip_bias_is_bounded`, `steering_unit_demand_turns_the_two_outputs_in_opposite_senses`.
5. Brakes: `brake_stops_a_shaft_without_reversing_it`, `brake_temperature_energy_balance_holds` (heat added equals work done minus cooling), `brake_fade_reduces_torque_above_the_fade_start`, `parking_brake_holds_on_a_grade`.
6. Fuel: `fuel_burned_equals_bsfc_times_work_on_the_map`.
7. Lumped-vehicle benches: `zero_to_32kmh_matches_the_hand_calculation` (constant-torque launch closed form), `top_speed_in_top_gear_matches_the_power_balance` (`P = (C_rr m g) v + (1/2 rho Cd A) v^3`), `hill_start_succeeds_exactly_when_first_gear_force_exceeds_m_g_(sin(theta)+C_rr)`, and, once the dossier lands, `speeds_in_gears_at_redline_match_the_m998_dossier` (a validation entry; gear count and ratios as published, otherwise ESTIMATE with a band).
8. `DrivePort` implementation built from any `DrivetrainDef` (the truck and the tank in the dummy rigs both work); `w5k drive bench <engine|shift|brake|launch> --out DIR` writes CSV and, once VIEWER's plotter exists, PNGs.

## Acceptance for M1  (CI on Linux and Windows)
All the tests above; state hash identical on both platforms; at most 8 us per powertrain step (the milestone budget is 30 us per vehicle-tick end to end: CHASSIS 20, DRIVE 8, glue 2); no NaN across a fuzz of 200 random but valid `DrivetrainDef`s (or a rejection with a reason).
The owner will see: a dyno curve (torque and power against rpm), a shift diagram, a table of speed per gear against the M998, a launch trace, a brake-fade plot.

## Theory to explain in `docs/theory/drive.md`
Power is torque times angular speed (the hyperbola of force against speed); a gearbox as a lever (torque times ratio, speed divided by ratio); the torque converter as a fluid coupling that slips and multiplies; why a diesel torque curve is flat and a petrol one peaky; shift hysteresis (graphics analogy: a Schmitt trigger, like debouncing); brake heat as a leaky bucket; why a tracked vehicle needs a steering unit while a car needs a differential.

## Non-goals
Wheel and ground contact (CHASSIS, TRACKS); driveline torsional wobble and gear lash (NOT-MODELLED); electric drives beyond a torque-speed map placeholder; engine thermal limits and cooling (M3); ABS.

## Needs from others / gives to others
Needs: nothing from CHASSIS at build time (wheel and sprocket inertia are in the rig, `WheelDef::inertia_kg_m2`; CHASSIS integrates the wheels and hands you `ShaftState`s); the real `DrivetrainDef`s from FORGE; M998 figures from VALIDATION. Gives: a `DrivePort` implementation to ARCH's glue, speed-per-gear and fuel-range figures to VALIDATION.

## Tripwires specific to this lane
A coupling that needs more than 8 substeps; a torque that reverses a stopped shaft; an efficiency or ratio written as a bare number; tuning anything to a single vehicle; modelling the NOT-MODELLED list.

## Done
M1 acceptance passes in CI, the theory note is written, the status file has the handoff note, you have idled.
