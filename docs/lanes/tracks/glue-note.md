# TRACKS: the tracked path through the glue (design note for ARCH; contract 0.3.0 = 0982843)

No code in `w5k_vehicle` is mine. This says what the glue calls, in what order, in which frame, and what comes back. The code it calls is merged or in review (`TrackedRunningGear`, `Belly`, `soil::*`).

## Per substep, per track (left, then right: ordered, no maps)
1. **Wheel penetrations (CHASSIS).** For each road wheel of `TrackDef::stations` that is a `RoadWheel`: the penetration of the belt bottom into the *undeformed* ground, `d_w = ground_height(wheel x, z) - (wheel bottom - thickness_m)`, and its rate. The ground is not deformed by TRACKS: soil sinkage is inside the sample, so `WorldQuery` stays a pure height-and-material query. Negative `d_w` (clear of the ground) is allowed; samples clamp at 0.
2. **Track-frame velocity.** The hull velocity at the track's contact centre, expressed in the track frame (x forward along the belt, y left) and the hull yaw rate about the up axis: `GearInput::{vel_long, vel_lat, yaw_rate}`. Contact centre = mean of the road-wheel positions (`GearConfig::from_rig` already centres the wheels on it); the glue maps it to the hull with the track node's pose.
3. **Ground per sample.** `ground: &[&Material]`, one per sample, from `WorldQuery` at the sample's world position (`GearConfig::samples` cell centres along the ground run: `TrackedRunningGear::sample_x_m()`), so a pit edge under a long track is seen.
4. **Sprocket speed.** `sprocket_omega_rad_s` from the DRIVE port's shaft state for `TrackDef::sprocket` (positive rolling forward, as in `UNITS-AND-FRAMES.md`). The belt speed is `omega * pitch radius`.
5. `step(&GearInput) -> GearTotals`.

## What the glue does with the result
- **Forces on the hull:** `outputs()[k]` is a `ContactOutput` in the track frame (x forward, y left, z up) per sample, at `sample_x_m()[k]` along the track: apply `(fx, fy, fz)` at that point (rotate by the track node's pose). `fx` already includes the compaction resistance; `GearTotals::{shear_thrust_n, compaction_n}` are the ledger rows ("track shear", "soil compaction"); `shaft_reaction_nm` is not a hull force.
- **Vertical load back to the suspension:** `wheel_force_n()[w]` (same order as the penetrations you passed) is the soil's reaction on road wheel `w`, partition-of-unity consistent with the sample forces: feed it to the wheel's `SuspensionElement` as the contact force, instead of a tyre force. Sum of wheel forces = sum of sample `fz`.
- **Shaft reaction to the drivetrain:** `GearTotals::shaft_reaction_nm` is the torque the sprocket must overcome (positive resists forward rolling): hand it to DRIVE as the load on the output of that track. Power balance: `reaction * omega = shear thrust * belt speed + internal running resistance`.
- **Belly:** once per hull, `Belly::step(belt_penetration_m, vel_long, vel_lat, ground, dt)` with the belt penetration of the wheels (the mean of the road wheels' `d_w`) and the hull velocity at the hull centre; `BellyGeom::from_rig` reads the rig's `ProxyRole::Belly` box (FORGE/GEOMETRY must author one; the reference tank has none, so tracked benches use `content/physics/tracks/ladder_tank.ron`). Apply the output at the hull centre. It is zero until the sinkage passes the clearance.
- **Tyres on soil (wheeled trucks):** CHASSIS fills `ContactOutput::sinkage_m` and a compaction drag from `soil::rigid_wheel_sinkage_m` and `rigid_wheel_compaction_resistance_n(soil, b, z0)` (tyre width and free diameter, the wheel's load): free functions, `UNVALIDATED` for a pneumatic tyre.

## Substeps and stability
One call is one substep; the gear is explicit over it, like the other ground loads (contract 0.3 `ArticulationLoads` note). The stiffest thing is the wheel contact (`WheelContact::vertical_stiffness_n_m`, 4 MN/m for the reference tank, a 44 t rig: bake the substep count from it as for a tyre) and the shear spring (about 5 Hz, damped inside the sample). Cost is about 0.2 us per sample, so 5 us per substep for two tracks of 12 samples.

## Things the glue must not do
- Add its own ground friction or rolling resistance for a tracked rig (`resist_c0/c1` are already in the sprocket reaction; the soil law supplies compaction). Do not read `Material::rolling_coeff` for a rig with tracks on soft ground.
- Pass a time step of zero except to read the static load (the benches do).
- Reset the gear on a teleport without `TrackedRunningGear::reset()` (shear state is history).

## Contract 0.3 fields
`track_runs` and `ArticulationLoads` need nothing from me. **`grouser_height_m` and `belt_stiffness_n_m` are in the contract but not consumed yet**: `GearConfig` ignores them (sag still comes from tension and belt weight; grousers still go through `shoe_mu_scale_soft`). I consume them in a later PR once the carrier's numbers exist; until then a rig that sets them gets the 0.2 behaviour, which is what "0 = none" promised only when they are 0.

## Open questions for ARCH
1. Contact centre and road-wheel order: I assume the glue passes wheels in ascending forward position (as `GearConfig::from_rig` sorts them); confirm or I add a permutation.
2. The glue owns the hull-level `Belly` call; if you would rather it sit inside a per-vehicle "tracked running gear" object with both tracks and the belly, I can provide that wrapper (about 40 lines) so the glue makes one call per substep.
