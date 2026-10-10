# Lane CHASSIS: a wheeled vehicle that rides, grips and handles, every force explainable

## Mission
When you have succeeded, the 4x4 truck of milestone M1 drives the bump strip with a real hull, four independent suspensions and tyres: it squats when it accelerates, dives when it brakes, rolls in a corner,
bottoms out only when it should, and every one of those motions can be traced to a force in the ledger. The owner will see a truck bouncing over a speed hump with its wheels, body and force vectors moving believably,
and plots of the same motion against analytic answers. You build the *chassis* in the engineering sense: 6-DoF hull, stations (wheel, suspension, steering), tyre contact, aero. DRIVE gives you torque; WORLD gives you ground.

## You own  (the CI lane guard enforces it)
`crates/w5k_chassis/**`, `content/physics/chassis/**`, `crates/w5k_tools/src/cmd/chassis.rs`; always `docs/swarm/status/chassis.md`, `docs/swarm/requests/chassis-*.md`, `docs/theory/chassis.md`, `docs/lanes/chassis/**`, `spikes/chassis/**`.

## You read, never edit
**Contract 0.1.1 note.** Read the section for your lane in `docs/architecture/redteam/` (what the red-teams found and the numbers behind it) and `docs/architecture/CONTRACT-0.1.1-PLAN.md` (the conventions). `PhysRig::required_features()` lists the optional rig features a rig uses: refuse a rig that needs one you do not implement yet, never ignore it.

`crates/w5k_contract` (pin the commit named under *Contract pin* in `docs/swarm/STATE.md`: `f8f5e5d`, contract 0.1.1, until `contract-v0.2`), `crates/w5k_math`, `docs/architecture/{CONTRACTS,UNITS-AND-FRAMES,DETERMINISM}.md`, `docs/validation/METHOD.md`, `docs/theory/the-physics-of-a-time-trial.md` (a model of the explanatory style we want), `docs/brief/*`.

## Stand-ins you start on
`FlatPlane`, `BumpStrip` (worlds); `ConstantTorquePowertrain` (a `DrivePort`); `box_truck()` (a valid `PhysRig`); `ScriptedCommands`; `ForceLedger::on()`. You need a **real** `WorldQuery` from WORLD, a **real** `DrivePort` from DRIVE and the HMMWV rig from FORGE by M1; until then these are enough.

## Settling round (first hours; then stop for review)
1. **Spike S1, stiffness vs step size** (`spikes/chassis/s1`, finding note `docs/lanes/chassis/spike-s1.md`): a quarter-car (sprung mass 400 kg, unsprung 50 kg, spring 30 kN/m, damper 2 kN s/m, tyre 250 kN/m) driven by a step and a sine, integrated with semi-implicit Euler at 60 to 1000 Hz. Report: stability limit, natural-frequency and damping error against closed form, energy drift, and the number of substeps per 60 Hz tick a rig needs as a function of its stiffest mode (propose the rule `substeps = ceil(20 * f_max_hz / 60)` or better). Also: a stopped truck on a 10% grade with relaxation-length friction must not creep or jitter.
2. **Design note** (one page, `docs/lanes/chassis/design-note.md`). Answer: the state you integrate and the integrator; how the contact patch is found (ray, cylinder or enveloping) and how a station's travel couples to the hull; how unsprung mass is integrated; the tyre model (brush or Pacejka-lite), its parameters, combined slip (friction ellipse) and relaxation; steering geometry (Ackermann, rack ratio); how the designer sliders (ride frequency, damping ratio) become rates (`k = m_corner (2 pi f)^2 / motion_ratio^2`, `c = 2 zeta sqrt(k m)`); what API the glue `w5k_vehicle` calls (propose it); what you need in the contract.
3. **CCRs** you expect: tyre and suspension parameter additions to `rig.rs`, anything the reference substep order in `CONTRACTS.md` gets wrong.

## Build order  (each step ends with its named tests)
1. `quarter_car` bench: `quarter_car_natural_frequency_within_1_percent`, `quarter_car_damping_ratio_within_5_percent` (log decrement), `energy_drift_below_0p1_percent_per_minute_frictionless`, `static_deflection_equals_mg_over_k`.
2. `suspension`: implement `SuspensionElement` for every `SpringKind` plus damper (asymmetric, digressive) and bump stop. `hydropneumatic_spring_follows_polytropic_law`, `torsion_bar_wheel_rate_matches_arm_geometry`, `bump_stop_engages_at_the_stated_travel`, `damper_force_is_asymmetric_and_knees_at_the_stated_speed`.
3. `tyre`: implement `ContactElement` (vertical spring-damper, longitudinal and lateral force from slip with relaxation length, friction ellipse, rolling resistance). `tyre_longitudinal_slope_at_zero_slip_equals_slip_stiffness_times_load`, `tyre_force_peaks_at_mu_times_load`, `combined_slip_never_exceeds_the_friction_circle`, `stopped_tyre_on_a_grade_does_not_creep`, `rolling_resistance_equals_crr_times_load`.
4. `hull`: the 6-DoF rigid body with wrench accumulation, world-frame inertia and the gyroscopic term; semi-implicit Euler with `Quat::integrate_world`. `free_fall_matches_g_t_squared_over_2`, `torque_free_spin_conserves_angular_momentum`, `spring_supported_hull_conserves_energy_to_0p1_percent`.
5. `wheeled` assembly: four stations on `BumpStrip` with `ConstantTorquePowertrain` (and later DRIVE's real one). `truck_rests_at_the_design_ride_height_within_5mm`, `axle_loads_match_com_position` (`F_front = m g b / L`), `braking_load_transfer_matches_m_a_h_over_L`, `steady_turn_lateral_load_transfer_matches_m_ay_h_over_track`, `braking_distance_matches_v2_over_2mu_g`, `low_speed_turn_radius_matches_ackermann_L_over_tan_delta`, `step_steer_yaw_rate_settles_without_oscillation_below_the_critical_speed`, `truck_crosses_the_speed_hump_at_20kmh_without_bottoming_out`.
6. The ledger: every term recorded; `ledger_net_force_equals_mass_times_acceleration` (to 1e-9, per body).
7. Wheel and hub dynamics: **you integrate each wheel's spin** (`J d(omega)/dt = T_shaft - shaft_reaction`, with `T_shaft` the net torque DRIVE's `DrivePort` returns and the reaction from your tyre model), each station's travel and steer, and you build the `ShaftState`s that DRIVE reads (inertia comes from the rig's `WheelDef::inertia_kg_m2`).
8. The glue API (agreed in the design note) and `w5k chassis bench <name> --out DIR` (CSV traces and a small PNG per bench via VIEWER's plotter once it lands; CSV until then).

## Acceptance for M1  (CI must pass these on Linux and Windows)
All the tests above; at most 20 us per vehicle-tick for the chassis alone on the 4x4 at its substep count (the milestone budget is 30 us end to end: DRIVE 8, glue 2; measure with a release-profile bench and report it); no NaN across the fuzz of 200 random sliders in range (ride frequency 0.8-3 Hz, damping ratio 0.1-0.8, wheelbase 2-4 m, mass 1-5 t) or a rejection with a stated reason; state hash identical on Linux and Windows.
The owner will see: the truck over the hump (replay through the reference viewer), force vectors at the contact patches, traces of travel, pitch and load transfer against the closed form.

## Theory to explain in `docs/theory/chassis.md`
Spring-mass-damper, ride frequency and damping ratio (graphics analogy: a critically damped spring is the "smooth damp" every engine uses for cameras); why a stiff spring needs a small step (about 20 steps per period) and why semi-implicit Euler conserves energy; the tyre as a slip-force curve and the friction circle; load transfer (why a braking truck dives); Ackermann geometry; why penalty contacts and relaxation length instead of a stick-slip switch.

## Non-goals
Tracked contact and soil (TRACKS), the powertrain (DRIVE), terrain (WORLD), anti-lock braking, tyre temperature or wear, wheel-hop beyond what unsprung mass gives, flexible bodies, any rendering. Do not invent a constraint solver.

## Needs from others / gives to others
Needs: DRIVE's `DrivePort`; WORLD's `WorldQuery`; FORGE's HMMWV rig; VIEWER's plotter; VALIDATION's M998 figures (suspension frequency, braking, wheelbase). Gives: the chassis API to ARCH's glue (wheel spin, travel and steer state, shaft states for DRIVE); `SuspensionElement` implementations that TRACKS reuses for road wheels.

## Tripwires specific to this lane
A rig needing more than 8 substeps (stop, write it up: this is spike S1's kill criterion); any constant tuned to one vehicle; any banned maths call; modelling something listed in `NOT-MODELLED.md`; a result you cannot explain from the ledger.

## Owner's direction 2026-10-10 (card C-020, `docs/architecture/DESIGN-MODEL.md`)
- Expose the checks FORGE's audit needs from your side: tyre load against rating per wheel at rest and in a turn, static suspension margin, static stability factor, and the skidpad limiter (grip, wheel lift, traction).

## Done
M1 acceptance passes in CI, the theory note is written, the status file has the handoff note, and you have idled. Anything further (the tracked version, ABS, the substep rule revisited for a 70 t rig) is a follow-up task from ARCH.
