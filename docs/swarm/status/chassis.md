# Status: CHASSIS

*Update with every PR. Keep it under 40 lines. ARCH reads this file at every check-in instead of your whole session.*

**Last updated:** 2026-10-10 UTC | **Branch:** lane/chassis/hauler-skid | **Contract pinned:** 0.3.0 (`0982843`) | **Phase:** slice 2, stage A

## Done
- S1, design note, quarter car, suspension, tyre, hull, wheeled assembly, Mule strip, modes, ledger, benches (#7-#76); slice 2 stage A (below).

## In progress
- Merged: load sensitivity (#118), tracked note (#123), sinking tyre (#126), 0.3 tyre fields (#138), tracked hull (#148, #153).
- **Hauler skidpad "power" at 0.59 g (ARCH 21:25Z): not mine, not power: traction through open diffs once the inner wheels unload.** Probe
  (`spikes/chassis/skid_probe`): from about 0.58 g the inner wheels carry 0-1 kN (one lifts), they saturate and spin, and the open centre and axle
  diffs send equal torque to them: total tyre drive force falls to -1..+6 kN at full throttle, speed stalls near 7 m/s. Load transfer (COM height,
  track) sets when that starts, hence the rollover levers. Label: "traction (inner wheel unloaded)". DRIVE: the box hunts 4<->5 at 7 m/s here.
- **Tracked assembly recipe (for ARCH's glue):** `w5k_chassis::tracked::TrackedChassis::new(&rig, &ChassisTuning (content/physics/chassis/
  tuning.ron), w5k_terramech::Tuning::shipped(), BellyGeom::from_rig(&rig, &tracks_tuning) (None = no belly), &world, x, z, yaw) -> Result<_,
  ChassisRefusal>`; per tick `tick(dt, &DriveInputs, &world, &mut dyn DrivePort)` (one shaft per sprocket output; skid steer comes from DRIVE's
  per-sprocket torques). Read: `hull` (pose, velocity), `datum_m()`, `wheels[i].travel_m`, `tracks[k].sprocket_angle_rad`/`_omega_rad_s`,
  `gear` (samples, totals, belly), `ledger` (switch on), `hash_state`. The rig needs: `tracks` whose road wheels are `RoadWheel` stations with
  suspension and unsprung mass, a sprocket station with `drive_output`, `wheel_contact` stiffness; preloads from statics; `ride_height_m` should
  include the belt's static penetration (about 13 mm on box_tank); `required_features` only "track running resistance".
- **Not trusted yet:** tyre curve linear to the cap (no slide drop); roll centre at wheel-centre height; single-ray contact; the sprocket `dR/dw`
  estimate (TRACKS asked: `requests/chassis-tracks-shaft-reaction-rate.md`).

## Blocked
- Nothing.

## Next
- Idle after this, per ARCH. Open on ARCH's call: tank replay frames, FORGE's T2 rig in the tank tests, linkages (6x6 walking beams), tyre slide drop.

## Cards needed / PROVISIONAL decisions in force
- PROVISIONAL: sprocket `dR/dw` estimate (TRACKS to report it); private Bekker pressure in `soil_wheel.rs` (until terramech's soil module).

## Evidence
- `cargo test -p w5k_chassis --release`: every oracle test passes (wheeled, benches, soil, tracked); PR bodies carry the numbers.

## Owner instructions received
- Owner in chat (2026-10-10): "go ahead with the hull"; "try again" (resume after a worker restart).
## Handoff note (fill in when you stop)
- (fill in at M1)
