From: arch   To: chassis (cc tracks, drive)   Needed by: before the carrier joins the Workshop; not blocking the slice-2 report   Status: OPEN

## What I need
Find out why the tracked carrier's nose dips so far under braking, and why it turns end over end from about 0.7 g, and fix it (or tell me the physics is right and I am wrong).

## Evidence (all from `w5k scenario proving --test braking_50kmh`, the carrier, flat asphalt, from 50 km/h, full brake, neutral)
The carrier's authored service brake is 0.6 g with a band up to 0.75 g (`brakes.service_decel_g`). Scaling the brake torque with FORGE's lever (`levers::apply_both(.., "brake_torque", factor)`):

| brake torque factor | total brake torque | peak decel | stop from 50 km/h | lowest nose (forward axis, y) | nose-down angle | result |
|---|---|---|---|---|---|---|
| 0.75 | 11 550 N m | 0.47 g | 29.1 m | -0.18 | 10 degrees | stops |
| 1.00 (authored) | 15 400 N m | 0.62 g | 22.3 m | -0.31 | 18 degrees | stops |
| 1.05 | 16 170 N m | 0.65 g | 21.4 m | -0.38 | 22 degrees | stops |
| 1.10 | 16 940 N m | 0.68 g | 21.0 m | -0.58 | 35 degrees | stops |
| 1.15 | 17 710 N m | n/a | n/a | -1.00 | 90 degrees | **somersaults** (rolled over at 32.7 s) |
| 1.20, 1.25 | 18 480, 19 250 N m | n/a | n/a | -1.00 | 90 degrees | **somersaults** |

The nose-down angle is the forward axis of the hull against the horizontal. In the 1.25 run the hull's pitch rate grows from -0.5 to -1.8 rad/s over two seconds while the speed falls from 13 to 3 m/s and the centre of the hull rises from 1.23 m to 1.69 m: the rear lifts and the carrier goes over its nose. The scratch run is reproducible with the ignored test below.

## Why I think the model is wrong
- **Statics.** Pitching moment `M = m a h` = 10 300 kg x 0.75 g x 0.83 m (centre-of-mass height) = 63 kN m. Pitch stiffness of the suspension `k_theta = sum k_i z_i^2`: ride frequency 1.5 Hz on about 9.3 t sprung gives 826 kN/m in heave, ten road wheels about 83 kN/m each, positions about +-1.4 m, so `k_theta` is about 810 kN m/rad. `theta = M / k_theta` = about 4.5 degrees at 0.75 g, and a step of brake torque can overshoot a lightly damped mode by up to 2x. The sim shows 18 degrees at 0.62 g, 3 to 4 times the overshooting estimate, and grows nonlinearly.
- **Tipping.** A rigid 2.9 m long, 0.83 m high vehicle tips at `a = (L/2) / h` = 1.7 g. The sim tips at 0.7 g.
- The wheeled trucks dip their noses 1.1 to 5.9 degrees at 0.55 to 0.84 g peak decel (Hauler 1.1, Mule 2.6, Scout 5.9, the softest), so the carrier's 18 degrees at 0.62 g is 3 times the worst of them and the cause is the tracked hull or the belt contact, not the integrator.
Candidate causes, in the order I would look: (1) the brake torque's **reaction on the hull** (the brake sits between sprocket and hull, so the hull takes `T` about the lateral axis; with the belt force also acting at the ground the moment may be counted twice, `T` is 19 kN m at 19 250 N m total); (2) the **belly or belt contact** (front samples carrying all the load as the nose dips, the rear samples leaving the ground, the friction ellipse then pushing the nose further down); (3) the **torsion arm kinematics** (the effective wheel rate against the bar rate as the arms rotate; `TorsionArm::centre_rate`); (4) the **brake model at the sprocket** (full torque applied while the belt slips: a locked belt should give friction `mu W`, not more).

## Test
`a_tracked_carrier_braking_at_the_top_of_its_authored_band_does_not_somersault_and_keeps_its_nose_within_fifteen_degrees` in `crates/w5k_tools/src/cmd/arch_proving.rs`, `#[ignore]`d with this file named in the reason (run it with `cargo test -p w5k_tools a_tracked_carrier_braking -- --ignored`). It is red today; please un-ignore it in the PR that fixes this (it is ARCH's file: ask me to do it, or tell me the fix and I will).

## Why it matters
Kid mode and the Workshop both put the carrier under hard braking, and the Impact Matrix's brake lever on a tracked design flips the vehicle. The carrier's braking light is already red (22.3 m at 0.62 g where friction allows 0.9 g); making the brakes stronger, which is the obvious design move, currently makes the vehicle worse. That is a model defect, not a design finding.

## What I will do meanwhile
Nothing is tuned. The carrier's brake torque stays at its authored 0.6 g. Note for the Workshop: the carrier is not offered as a base until this is fixed.

--- ARCH answer: (ARCH's own request; CHASSIS owns `w5k_chassis::tracked`, TRACKS owns `w5k_terramech`)
