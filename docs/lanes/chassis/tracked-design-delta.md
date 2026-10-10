# CHASSIS design delta: the tracked hull against TRACKS' glue note

*Short, as ARCH asked (2026-10-10 18:16Z). Supersedes section 1's `RunningGear` seam in `tracked-and-sinkage-note.md`: there is no `ports.rs` trait and no CCR.
CHASSIS calls `w5k_terramech::vehicle::TrackedVehicleGear` directly (the LAYERS edge chassis -> terramech is documented). Module: `w5k_chassis::tracked`
(`TrackedChassis`); `wheeled.rs` is untouched.*

## State
- **Hull:** the same `Hull` (angular-momentum state, semi-implicit Euler), the same substep floor from the stiffest mode (`modes.rs` learns the road wheel on its
  torsion arm against `TrackDef::wheel_contact.vertical_stiffness_n_m`).
- **Road wheels** (`WheelKind::RoadWheel`): one travel coordinate each, `c` = the wheel centre's vertical rise (the contract's convention), with the arm geometry
  below; its own unsprung mass; a `Suspension` (the `Torsion` spring returns the vertical force at the wheel, already tested).
- **Sprockets** (one per track, `TrackDef::sprocket`): spin only, `J_eff dw/dt = T_shaft - shaft_reaction`, with the belt-linked inertia reflected onto it:
  `J_eff = J_sprocket + sum over the track's other stations J_i (r_s / r_i)^2 + m_belt r_s^2`, `m_belt = mass_per_m_kg * belt_length_m`. Road wheels, idler and
  rollers have no spin state of their own; their displayed spin is `omega_s r_s / r_i`.
- **Idler and return rollers:** rigid, mass in the hull (the contract says so).

## Torsion-arm geometry (pivot `P`, arm length `L = |rest - P|` in the hull's y-z plane, rest angle `phi0` below horizontal)
`sin(phi) = sin(phi0) - c / L`; centre `= P + L (cos(phi) h, -sin(phi))` with `h` the unit fore-aft direction from the pivot to the wheel (trailing or leading).
`d centre / dc = (tan(phi) h, 1)`: the wheel rises by `dc` and swings fore-aft by `tan(phi) dc`. The travel equation uses the generalized mass
`m_u / cos^2(phi)` and the generalized force `Q = F . (tan(phi) h, 1)` (exactly the work the force does per unit of `c`).

## Per substep, in this order (the glue note's steps, then ours)
1. Road-wheel centres from the hull pose and each `c`; belt penetration per road wheel `d_w = ground_height - (centre_y - r - thickness)` and its rate, wheels in
   ascending forward position, per track (left, then right).
2. Track-frame velocity at each track's contact centre (`vel_long`, `vel_lat`, yaw rate about up); the ground material at every `sample_x_m()` position.
3. DRIVE: `ShaftState` per sprocket output (omega, `J_eff`, vehicle speed, last `shaft_reaction_nm` as the load), `DrivePort::step` returns `T_shaft`.
4. `TrackedVehicleGear::step(&[TrackStepInput; 2], belly, dt)` (belly: the mean `d_w`, hull-centre velocity, the ground under the hull centre).
5. Forces: every sample's `(fx, fy, fz)` at its point on the hull (rotated by the track frame), the `r x F` torque included; `wheel_force_n()[w]` drives road
   wheel `w`'s travel (`Q` above) and the strut pushes the hull; the belly output at the hull centre; gravity; aero.
6. Integrate: hull, each `c` (droop and bump limits as for a wheel), each sprocket's spin.
7. Ledger: hull rows for every sample force (`TrackShear` = the sample's `fx`, `TrackNormal` = `fz`, lateral as `TrackShear`), `SoilCompaction` from
   `GearTotals::compaction_n` split out of `fx`, `BellyDrag`, strut terms on hull and road wheel, gravity, aero; road-wheel bodies as in `wheeled.rs`
   (along-strut parts plus the frame force), so `ledger_net_force_equals_mass_times_acceleration` holds for every body.

## Not doing (glue note)
No ground friction or rolling resistance of our own for a tracked rig; never `dt = 0` except to read the static load; `TrackedRunningGear::reset()` on a teleport.

## PRs
1. This note, the module skeleton, `torsion_arm_wheel_centre_follows_the_circle` and `reflected_inertia_spins_up_at_t_over_j_eff`.
2. The integrated tank: `tank_rests_at_its_design_ride_height_within_5mm` (ladder tank / `box_tank()`), the tracked ledger balance, then FORGE's T2 rig.
