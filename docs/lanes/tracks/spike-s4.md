# Spike S4: soil fidelity versus cost (TRACKS)

Code: `crates/w5k_terramech/src/soil.rs`, tests `crates/w5k_terramech/tests/s4_soil.rs`. Soils: `content/physics/tracks/reference_soils.ron`, three with `n != 1`:
sandy (LETE class, n 0.79), clayey (n 0.5), snow (n 1.6). **UNVALIDATED: the numbers are the Bekker-Wong table classes as remembered; lane sessions cannot reach Wong, *Theory of Ground Vehicles* (ch. 2 tables, ch. 5 tracked vehicles), so no page or example number can be cited.** The oracles below are closed forms the code did not produce; VALIDATION owes the published-number check (card C-017, `wong_drawbar_pull_example_matches_the_published_value` stays a to-do tagged UNVALIDATED rather than a test with an invented number).

## Verdict: full Bekker-Wong at every sample is cheap enough; no table
| Check | Oracle | Result |
|---|---|---|
| Plate sinkage, three soils, three pressures (`gear_sinkage_matches_the_bekker_plate_curve_for_three_soils`) | `z = (p / (kc/b + kphi))^(1/n)` | within 1e-4 relative |
| Drawbar thrust of a rigid footprint, 3 soils x slip 5, 10, 20, 40% (`drawbar_thrust_matches_the_closed_form_shear_integral_for_three_soils`) | `H = (A c + W tan(phi)) [1 - K/(iL) (1 - e^(-iL/K))]` | within **0.4%** at every point (worst +0.33%; the 12 points range +0.07% to +0.33%), 24 samples |
| Compaction resistance (`compaction_resistance_equals_the_numerical_integral_of_pressure_over_sinkage`) | numerical `b integral p dz` | within 0.1%; `n = 1` gives `b p z / 2` |
| Cost (`contact_cost_per_sample_is_a_few_microseconds`) | | 210 ns per soft sample step (release-like test profile) |

## Why the sinkage solve is not the textbook closed form
The contract hands a contact only a *geometric penetration* (the belt bottom against the undeformed ground). The wheel and belt are a spring in **series** with the soil, so the sample solves `k_s (delta - z) = A k z^n` for the sinkage `z` (a bracketed Newton, about six iterations; bisection when Newton leaves the bracket). `z` is the soil law, `delta - z` the deflection of the wheel contact. With a stiff wheel the answer is exactly the plate curve, which is what the first row tests.

## Wheels sink too
`soil::rigid_wheel_sinkage_m` and `rigid_wheel_compaction_resistance_n` implement Wong's rigid-wheel formulas (`z0 = [3W / (b (3 - n) k sqrt(D))]^(2/(2n+1))`, small sinkage form), taking a `SoilParams` and a width, with no track in sight: CHASSIS's tyre can call them today (width and diameter from its `TyreDef`, load from its own vertical force) to fill `sinkage_m` and a compaction drag (`UNVALIDATED`: a tyre is not a rigid wheel; Wong's pneumatic-tyre form is a later refinement).
