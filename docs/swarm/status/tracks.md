# Status: TRACKS

**Last updated:** 2026-10-10 UTC | **Branch:** lane/tracks/settling | **Contract pinned:** contract-v0.2 (0.2.0) | **Phase:** settling (PR 1 of 4), then building

## Done
- Spikes S3 and S4 written up and passing their kill criteria without tuning: `docs/lanes/tracks/spike-s3.md` (pivot +5.0% of `mu W L / 4`), `spike-s4.md` (drawbar thrust within 1.4% of the closed form, three soils with n != 1, 218 ns per sample).
- PR 1 (settling): soil laws (`soil.rs`), tuning and reference soils as RON (`content/physics/tracks/`), `w5k tracks bench plate`, design note with CCRs (`docs/lanes/tracks/design-note.md`), theory note (`docs/theory/tracks.md`).

## In progress
- PR 2 `lane/tracks/contact`: `TrackSample: ContactElement` + `TrackedRunningGear`. PR 3: plan-view skid model + the S3/S4 tests + `w5k tracks bench thrust|pivot`. PR 4: belly drag, the ladder bench, track-vs-tyre float test. (Split because the kernel is over 400 non-test lines; the working code exists and is being cut into reviewable pieces.)

## Blocked
- Nothing. Published-number checks (Wong's examples) wait on card C-017.

## Next
- Open PRs 2 to 4 in order; `sinkage_m` for CHASSIS's tyre through `soil::rigid_wheel_*` (interface request not needed: free functions in my crate).

## Cards needed / PROVISIONAL decisions in force
- C-017 (sources): every soil number and the Wong oracles are UNVALIDATED. Default (c): carry on.
- PROVISIONAL(shoe_mu_scale_soft): grousers scale soil strength; a rubber pad on soil uses the soil's own strength.

## Evidence
- `cargo test -p w5k_terramech`: `plate_sinkage_matches_bekker_closed_form`, `shear_curve_slope_at_zero_displacement_is_tau_max_over_k`, `compaction_resistance_for_n_equal_one_is_half_b_p_z`, `pivot_turn_moment_equals_mu_w_l_over_4_on_firm_ground`, `stopped_track_on_a_grade_does_not_creep`, `drawbar_thrust_matches_the_closed_form_shear_integral_for_three_soils`.
- Image: `docs/lanes/tracks/media/plate_sinkage.png`.

## Owner instructions received
- None direct. Slice 2 stage A and B assigned by ARCH (`docs/swarm/SLICE-2.md`).

## Handoff note (fill in when you stop)
- Changed: ... | Unfinished: ... | Surprised me: a long footprint turns 1.7x wider than the belts say | I would do next: ...
