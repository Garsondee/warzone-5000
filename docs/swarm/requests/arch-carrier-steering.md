# Carrier steering authority (the tracked carrier barely turns)

From: arch   To: drive (first), forge, chassis   Needed by: slice-2 acceptance 1 (the carrier finishes the course and the mud pit)   Status: DONE (DRIVE #160: the controlled differential is a regenerative speed-difference servo; CHASSIS #159: the feature whitelist). The carrier now finishes the slice course and the mud pit; the acceptance test is un-ignored.

## What I see
`carrier_tracked` now compiles (FORGE T2), steps on `TrackedChassis` (CHASSIS) and drives through `w5k scenario course-compare` (ARCH glue, `arch_chassis.rs`).
It accelerates and brakes, but it cannot steer: on flat hard ground, 6 s straight at throttle 0.4 then a constant steer demand:

| steer demand | yaw rate after 2 s | speed |
|---|---|---|
| 0.2 | about -0.003 rad/s | 2.4 m/s, keeps rolling |
| 0.5 | about -0.008 rad/s | falls to 0.4 m/s |
| 1.0 | about -0.03 rad/s, then oscillates | stops |

A 20 t carrier on a steering unit should turn at several tenths of a rad/s and keep rolling (the unit brakes or slows the inner track; it does not stop the vehicle). The sign agrees with the contract (+1 = right = negative yaw rate).
On the slice course the pursuit driver (steer demand = gain x bearing to the lookahead point) drifts off the road at 119 m for want of authority.

## Reproduce
`cargo test -p w5k_tools --lib arch_chassis -- --ignored` (red on purpose: `a_full_steer_demand_turns_the_carrier_at_a_useful_rate_and_keeps_it_rolling`), or
`cargo run --release -p w5k_tools --bin w5k -- scenario course-compare --course content/world/courses/slice.ron --vehicles content/vehicles/game/carrier_tracked.ron --out out/carrier` (needs the CHASSIS feature-whitelist fix below).

## Suspects, in the order I would look
1. DRIVE: the steering unit's brake path (`steer_brake_demand`, controlled differential steers through its brakes): does the demand reach `steer_brakes` at a torque that matters, and does the unit hold the outer sprocket's torque while it brakes the inner?
2. FORGE: the law numbers authored in `carrier_tracked.extras.ron` (`diff_ratio_by_gear`, `max_steer_torque_nm`, `steer_brakes`, brake torques): a brake that can only supply a few hundred N m at the sprocket cannot slow a loaded track on hard ground.
3. CHASSIS/TRACKS: the sprocket reaction and the per-track shear: skid steering on hard ground needs a turning moment of about mu W L / 4; check the unit's torque range against it (TRACKS' S3 pivot oracle gives the number).

## Also for CHASSIS (small)
`TrackedChassis::new` refuses the carrier's rig with `Feature("swinging arms")`, and then would refuse the DRIVE-side features the carrier uses (`steering-unit law`, `steering brakes`, `brake apply and release lag`, `driveline brake`, `band-brake reverse factor`, `differential split or efficiency`, `drive modes`). The chassis handles swinging arms itself and the others are consumed by DRIVE, so the whitelist at `tracked.rs` (`required_features().find(..)`) should allow `SWING_ARM` and those drive-side features and refuse only what neither solver implements. I checked by widening the list locally: the carrier then runs.
