# ARCH proving-ground runner gaps (`w5k scenario proving`)

Found while building `crates/w5k_tools/src/cmd/arch_proving.rs` against `docs/validation/proving-ground.md` and `w5k_validate::proving::TESTS`.
A test the current APIs cannot run is reported as `not implemented` and writes **no file** (so the dashboard shows `not measured`), never a fake number.
Format follows `docs/swarm/requests/README.md`; ARCH is the author.

## 1. Ride tests (`ride_iso8608_c`, `ride_iso8608_d`, `ride_washboard`): no runner in this series
From: arch   To: validation   Needed by: phase 1 delivery step 3   Status: OPEN
What I need: a decision on who builds them. They are not in the ARCH proving brief (braking, acceleration, rollover, step climb, skidpad, gradeability).
Why: they need a seeded ISO 8608 road (a `WorldQuery` that WORLD does not yet offer: `BumpStrip` has a washboard, but no random profile) and echoed quarter-car inputs (`spring_rate_n_m`, `damping_n_s_m`, `tyre_rate_n_m` per axle, an average the scorer must define).
Meanwhile: `w5k scenario proving --test ride_washboard` says `not implemented`; `--test all` skips them.

## 2. Brake and traction findings (not gaps, for the owner of each model)
The first braking runs from 50 km/h on dry asphalt, wheels braked, engine disconnected, no ABS: Mule 12.1 m (0.83 g, +11 % over `v^2/(2 mu g)`), Scout 14.1 m (0.73 g, +30 %), Hauler 19.2 m (0.54 g, +76 %). The Hauler and the Scout decelerate well below `mu g`, which points at brake torque (DRIVE content) or brake-force distribution, not at the tyre; VALIDATION's oracle will say so in the light. Nothing is tuned by this runner.

## 3. CHASSIS: `bench::tilt_table` cannot report sliding
From: arch   To: chassis   Needed by: nothing blocking   Status: OPEN
What I need: `TiltResult` with a `slide_angle_rad` (the angle at which every loaded patch reaches its friction limit), or a public way to run the same loop with a callback.
Why: VALIDATION's side-slope scorer wants a `mode` label (`roll` or `slide`, the smaller of `atan(t/2h)` and `atan(mu)`). The bench stops at wheel lift only, and its own tests use a 2.5 grip plane so that it never slides.
Meanwhile: the runner rotates gravity itself (`WheeledChassis::gravity_m_s2` is public) and a test checks that its lift angle agrees with the bench's to 0.02 rad.

## 4. WORLD: no vertical step in the stand-in worlds
From: arch   To: world   Status: OPEN (observation)
`w5k_contract::testing::BumpStrip`'s `Plateau` rises as a smoothstep over `ramp_m`; the step test sets 2 cm. The ground normal there is a finite difference over 10 cm, so the face reads as a steep ramp rather than a wall. A WORLD `DataStrip` with a true vertical face (and a contact that samples the face, not only the height under the wheel centre) would make the Mule's 0.34 m (about 0.8 wheel radii, above the quasi-static traction limit of 0.23) explainable from the force ledger; today it is explained by momentum (0.125 m at 0.5 m/s) by experiment only.
