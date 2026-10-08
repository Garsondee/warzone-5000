# Lane TRACKS: tracked running gear that floats where a truck bogs, and turns by skidding

## Mission
When you have succeeded, a tank crosses firm ground and mud and the outcome follows from its design: wider and longer tracks float on soft ground, heavier armour sinks it deeper until the hull drags and it bogs, and it turns by driving its two tracks at different speeds, paying for it in power.
You write the soft-ground and track contact laws (Bekker pressure-sinkage with an exponent, Mohr-Coulomb strength, Janosi-Hanamoto shear, compaction resistance) and the track as a row of contact samples coupled to the sprocket. The owner will see thrust-against-slip curves per soil, a pivot-turn trace, and the
"ladder" chart: the same tank getting heavier and where it starts to bog.

## You own  (the CI lane guard enforces it)
`crates/w5k_terramech/**`, `content/physics/tracks/**`, `crates/w5k_tools/src/cmd/tracks.rs`; always `docs/swarm/status/tracks.md`, `docs/swarm/requests/tracks-*.md`, `docs/theory/tracks.md`, `docs/lanes/tracks/**`, `spikes/tracks/**`.

## You read, never edit
`crates/w5k_contract` (`TrackDef`, `StationDef`, `ContactElement`, `ContactInput/Output`, `SoilParams`, `Material`, `WorldQuery`), `docs/architecture/*`, `docs/theory/the-physics-of-a-time-trial.md` (the Bekker, Mohr-Coulomb and "why tracks float" derivations, valid; they used n = 1, you may use the general exponent), and the prototype soil code for ideas only (`reference/prototype-v0/crates/w5k_sim/src/soil.rs`: QUARRY, re-write).

## Stand-ins you start on
`FlatPlane` and `standard_materials()` (its `mud` has stand-in soil numbers; WORLD and VALIDATION will cite real ones), `box_tank()` (a valid tracked `PhysRig`), `ConstantTorquePowertrain` (two sprocket outputs), your own linear road-wheel suspension in tests until CHASSIS's `SuspensionElement`s land.

## Settling round (first hours; then stop for review)
1. **Spike S3, skid-steer contact feel** (`docs/lanes/tracks/spike-s3.md`): a plan-view model with 8 to 16 samples per track, each with a vertical load, a belt speed from the sprocket, a longitudinal slip `i = 1 - v_x / v_belt` and a lateral slip velocity `v_y`; the shear stress `tau = (c + p tan(phi)) (1 - exp(-j / K))` with the shear displacement `j` accumulating along the footprint and its direction opposing the slip velocity; sum thrust, lateral force and moment. Check the pivot-turn moment against `mu W L / 4` on firm ground and that a stopped tank on a grade neither creeps nor jitters. **Kill criterion:** pivot torque within 25% of `mu W L / 4` and no oscillation, else change the model.
2. **Spike S4, soil fidelity versus cost:** reproduce the textbook plate-sinkage and drawbar-pull examples (Wong, *Theory of Ground Vehicles*; cite page and example number, or tag UNVALIDATED if you cannot source them) for three soils with `n != 1`; time per contact; tabulate if too slow.
3. **Design note:** the sample geometry and how samples follow the ground; how road-wheel loads become sample pressures (a distribution along the footprint, not a uniform guess); sinkage per sample, compaction resistance (the integral of pressure over sinkage), belly drag when sinkage exceeds clearance (a smooth ramp, never a cliff); firm-ground friction with a shear curve for steel and rubber shoes; sprocket coupling and the reaction torque on the shaft; what `ContactElement` needs beyond today's contract (track-sample geometry, belt speed).
4. **CCRs** you expect: `ContactInput` additions, `TrackDef` additions (grouser height, belt stiffness).

## Build order  (named tests)
1. Soil laws: `plate_sinkage_matches_bekker_closed_form` (`z = (p / (kc/b + kphi))^(1/n)`), `ground_pressure_equals_weight_over_contact_area`, `shear_stress_saturates_at_cohesion_plus_p_tan_phi`, `shear_curve_slope_at_zero_displacement_is_tau_max_over_K`, `compaction_resistance_for_n_equal_one_is_half_b_p_z`, `wong_drawbar_pull_example_matches_the_published_value` (UNVALIDATED if unsourced).
2. Track contact (`ContactElement` per sample): `thrust_sums_to_the_expected_value_for_a_uniform_footprint`, `stopped_track_on_a_grade_does_not_creep`, `belly_drag_ramps_smoothly_with_sinkage_over_clearance`.
3. The track as a whole: `skid_steer_turn_radius_follows_the_track_speeds` (kinematic `R = (B/2)(v_o + v_i)/(v_o - v_i)`), `pivot_turn_moment_equals_mu_W_L_over_4_on_firm_ground`, `track_floats_where_a_tyre_of_the_same_pressure_sinks` (the width effect in `kc/b`), `heavier_tank_sinks_deeper_and_bogs_beyond_a_ground_pressure_threshold` (the ladder), `power_to_pivot_exceeds_power_to_drive_straight`.
4. Integration: `TrackSample: ContactElement` plus a `TrackedRunningGear` object that the glue calls (agreed in the design note) with sprocket speed in and shaft reaction out. `w5k tracks bench <thrust|pivot|ladder> --out DIR` writes CSV and, once available, PNGs.

## Acceptance
For M1 you deliver the spikes S3 and S4 written up, the kernel above passing on stand-ins and the CSV benches. For **M2** (the milestone this lane is built for): the tracked reference vehicles (M113, Sherman, an Abrams-class and a Leopard-class tank) cross the mixed course with explainable outcomes; mobility limits (grade, sinkage, pivot) bracketed against the dossiers (published <= simulated <= 1.25 x published); a ground-pressure ladder chart.

## Theory to explain in `docs/theory/tracks.md`
Soil as a spring (Bekker) and a friction wedge (Mohr-Coulomb); why a wide long footprint floats (the `kc/b` term and the compaction integral); shear needing displacement before it delivers strength (Janosi-Hanamoto; graphics analogy: the saturating tone-mapping curve); skid-steering as scrub (why a pivot turn costs the most power); the `mu W L / 4` turning-resistance moment.

## Non-goals
The powertrain (DRIVE), tyres and the hull integrator (CHASSIS), persistent ruts and multi-pass soil memory (NOT-MODELLED until after M3), track-link dynamics, throwing a track (M3 damage).

## Needs from others / gives to others
Needs: CHASSIS's hull and station API and `SuspensionElement`s; WORLD's `MaterialTable` with cited soil numbers; DRIVE's steering unit; M113 and Sherman dossiers from VALIDATION. Gives: `ContactElement` implementations and the tracked running-gear object to ARCH's glue; soil-law test vectors to VALIDATION.

## Tripwires specific to this lane
A soil constant without a source or `UNVALIDATED`; an exponent hard-wired to 1; a contact that needs more than 8 substeps for a 70 t rig; tuning soil numbers to make one tank pass; a banned maths call (use `w5k_math::scalar::pow`).

## Done
The deliverables for the milestone you are working on pass in CI, the theory note is written, the status file has the handoff note, you have idled.
