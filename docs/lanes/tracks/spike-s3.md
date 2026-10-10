# Spike S3: skid-steer contact feel (TRACKS)

Code: `crates/w5k_terramech` (`sample.rs`, `gear.rs`, `plan.rs`), tests `crates/w5k_terramech/tests/s3_skid_steer.rs`. Rig: a 40 t tank in plan view, 0.6 m tracks, 4.8 m contact length, gauge 2.8 m, 12 samples per track, five road wheels, on `dirt` (`mu_peak` 0.65, firm). **Kill criterion: pivot moment within 25% of `mu W L / 4` without tuning, and no oscillation.**

## Verdict: passes, no tuning of the model against its answer
| Check | Oracle | Result |
|---|---|---|
| Pivot turn moment (`pivot_turn_moment_equals_mu_w_l_over_4_on_firm_ground`) | `mu W L / 4` = 305.97 kN m | 321.2 kN m, **+5.0%** (limit 25%) |
| Stopped on a 30% grade (`stopped_track_on_a_grade_does_not_creep`) | holds (`tan(theta)` 0.3 < `mu` 0.65), no jitter | max speed 2e-16 m/s after 5 s, 0 sign changes |
| Power to pivot vs drive straight (`power_to_pivot_exceeds_power_to_drive_straight`) | pivot costs far more | straight 18.7 kW, pivot 197.5 kW (10.6x) |
| Turn radius, short footprint (`skid_steer_turn_radius_follows_the_track_speeds`) | `R = (B/2)(vo+vi)/(vo-vi)` = 7.00 m | 7.06 m (+0.9%) |
| Turn radius against footprint length (`a_longer_footprint_turns_wider_for_the_same_track_speeds`) | longer scrubs more, so turns wider | L = 0.8, 2.4, 4.8 m give R = 7.06, 7.47, 11.86 m |

## What the model is
Each track is a row of massless samples along the ground run. Per sample: a vertical load from the soil or ground (see S4), a belt speed from the sprocket, and a **shear displacement vector** `j` (in the track plane). `j` grows with the slip velocity `u = v_hull - v_belt` and is carried along the footprint by the belt (the shoe point that entered at the front of the patch arrives at sample *k* after `d_k / v_belt`; the Janosi-Hanamoto law then gives the stress `tau = tau_max (1 - e^(-|j|/K)) * j/|j|`, pointing against the slip). Summing force and moment over both tracks gives the wrench on the hull.

## Findings worth keeping
1. **The pivot moment is Coulomb, not viscous.** Each sample scrubs sideways with `u_y = omega x`; `j` far exceeds `K` almost everywhere, so the stress saturates at `mu p` and the moment is `integral mu p |x| dx = mu W L / 4` whatever the yaw rate. The first 5% of error comes from the few samples near the centre that are not yet saturated.
2. **The kinematic radius is a short-footprint result.** Because the pivot scrub moment is there at any radius (it does not shrink with `omega`), the tracks must run slipped to produce the differential thrust that overcomes it. A 4.8 m footprint on a 2.8 m gauge (L/B 1.7) turns at R = 11.9 m where the belts alone say 7.0 m (effective gauge 1.7x; the sideslip is 0.28 m/s). This is what tanks do (steering is a *resistance* problem), so slice 2's acceptance wording "the track-speed turn radius equals the kinematic value" holds for the *actual* ground speeds of the two tracks and for short footprints, and the code reports the steering ratio instead of hiding it. UNVALIDATED against a published steering factor; VALIDATION to check Wong ch. 5 (card C-017).
3. **A shear spring needs damping.** Left alone, a parked hull on a grade rings on its own shear stiffness (natural frequency `sqrt(mu g / K)`, about 5 Hz). The sample adds a small viscous shear stress sized from that stiffness (damping ratio 1 at creep speed, fading above 1 cm/s so it never shapes the thrust-slip curve). With it the grade test settles in a single step with no sign change. A first value of 5 cm/s biased the drawbar curve by +7.8% at 5% slip; 1 cm/s brings it under 1.4% (see S4). That is the only constant changed after seeing a result, and it is a numerical regularisation, logged in `content/physics/tracks/tuning.ron`.
4. **Cost.** 218 ns per soft-ground sample step, so about 5 us per substep for a rig with two tracks of 12 samples. No lookup table needed.

## Limits
Plan view only: heave and pitch are frozen at the static equilibrium (the glue integrates them in the real rig). The shoe friction on firm ground uses the surface's `mu_peak`; no peak-to-slide drop yet.
