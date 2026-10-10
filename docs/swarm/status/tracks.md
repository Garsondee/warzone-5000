# Status: TRACKS

**Last updated:** 2026-10-10 UTC | **Branch:** lane/tracks/bog | **Contract pinned:** contract-v0.3 = 0982843 (0.3.0) | **Phase:** stage A and B kernel built (4 stacked PRs); integration waits on others

## Done
- Spikes S3 and S4 pass their kill criteria without tuning: pivot +4.6% of `mu W L / 4`; drawbar thrust within 0.4% of Wong's closed form on three soils with n != 1; 210 ns per sample (`docs/lanes/tracks/spike-s3.md`, `spike-s4.md`).
- PR 116 settling: soil laws, reference soils and tuning as RON, design note with CCRs, theory note. PR 120: `TrackSample` and `TrackedRunningGear`. PR 125: plan-view skid model, S3/S4 tests, thrust and pivot benches. PR 4 (`lane/tracks/bog`): belly drag with a smooth onset, the ladder bench and charts, track-versus-tyre float test, `BellyGeom::from_rig`. PRs 120, 125 and 4 are stacked on 116.
- Rigid-wheel Bekker (`soil::rigid_wheel_*`) is a free function: CHASSIS's tyre can fill `sinkage_m` today (11 t on clay: tracks 0.9 mm, four tyres 112 mm).

## In progress
- Nothing coded. Waiting for review and for the glue (below).

## Blocked
- The tracked vehicle itself needs CHASSIS's tracked hull on the integrator and ARCH's glue calling `TrackedRunningGear` (interface: design note, "The object the glue calls"), WORLD's cited soil table and DRIVE's steering unit. Published-number checks (Wong's examples) wait on card C-017.

## Next
- Adapt to review changes. Once the glue exists: the M113 and Sherman crossing the mixed course, mobility limits against the dossiers, the ladder on WORLD's mud.

## Cards needed / PROVISIONAL decisions in force
- C-017 (sources): every soil number and Wong oracle is UNVALIDATED. Default (c): carry on.
- TRACKS-D1 (bog criterion), PROVISIONAL: bogged when the net drawbar pull at 50% slip is below 10% of the weight (`bog_pull_fraction`); Wong's zero-pull definition (0) is never reached by the reference tank in snow below 150 t. The owner can pick another fraction and the ladder redraws.
- PROVISIONAL(shoe_mu_scale_soft): grousers scale soil strength; a rubber pad on soil uses the soil's own strength.

## Evidence
- `cargo test -p w5k_terramech` (30 tests), e.g. `pivot_turn_moment_equals_mu_w_l_over_4_on_firm_ground`, `drawbar_thrust_matches_the_closed_form_shear_integral_for_three_soils`, `belly_drag_ramps_smoothly_with_sinkage_over_clearance`, `heavier_tank_sinks_deeper_and_bogs_beyond_a_ground_pressure_threshold`, `track_floats_where_a_tyre_of_the_same_weight_sinks`.
- Images in `docs/lanes/tracks/media/`: plate_sinkage, thrust_slip, pivot_response, ladder_sinkage, ladder_pull.

## Owner instructions received
- None direct. Stage A and B of slice 2 assigned by ARCH (`docs/swarm/SLICE-2.md`).

## Handoff note (fill in when you stop)
- Changed: the kernel only; no contract or other lane touched. | Unfinished: integration into a vehicle, mobility limits against dossiers (M2), published-number validation. | Surprised me: a long footprint turns 1.7x wider than the belts say; Bekker's `kc/b` makes a narrow plate sink slightly LESS at the same pressure (the brief's "width effect in kc/b" has the sign backwards: width floats through pressure). | I would do next: the tracked glue test on the real rig, then the grouser model once `grouser_height_m` exists.
