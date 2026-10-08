# Contract red-team: wheeled vehicles (M998 HMMWV and M35 6x6)

*Written by a read-only reviewer agent during the Launch Kit (2026-10-08); condensed by ARCH. All vehicle numbers are plausible, not sourced. The evidence
crate (not committed) wrote both vehicles as `PhysRig` + `RenderRig` + `VehicleDef` against contract 0.1.0 (36 tests), tagged every hack `HACK[Gn]` and compiled a
sketch of each CCR. How each finding was settled is in `docs/architecture/CONTRACTS.md` (change log, 0.1.1).*

**Verdict.** The HMMWV fits (portal axle as `OutputDef.final_drive_ratio`, hull-mounted differential proxies, anti-roll bars, converter and automatic). The M35
does not: 2 findings block it, 7 force hacks, 3 are cosmetic.

## What expressed cleanly
Independent coil suspension (wheel rate, asymmetric digressive damper, travel limits, progressive bump stop, anti-roll bars); steered front axle and the
`hull > travel > steer > wheel` chain; a portal reduction as `final_drive_ratio`; a permanently driven 4x4 as nested limited-slip `Diff`s; a locked three-way
transfer case as `Diff { Locked }` with three children; two tyres on one hub as a nested locked pair; converter plus automatic (HMMWV), clutch plus manual
(M35); two brakes on one station (service disc, parking drum); hull-mounted differential proxies with portal lift; mass, centre of mass and inertia with preloads
that balance the weight (zero residual); RON round trip of `PhysRig` is bit-exact.

## Gaps, in priority order
1. **G1 BLOCKS (M35): no runtime driveline state.** High/low range, front-axle declutch, diff lock and a clutch pedal are not in `Command`, `DriveInputs` or `DriveNode`.
   The hack flattened the range into ten "gears" (steps down to 0.6%) and made a declutched truck a different rig. Fix: `DrivetrainDef.modes: Vec<DriveModeDef { name,
   transfer_ratio_scale, declutched_outputs, locked_groups }>` plus `default_mode`; `Command.drive_mode: Option<u8>` and `Command.clutch: Option<f64>`;
   `DriveInputs.drive_mode`, `DriveInputs.clutch`, `DriveTelemetry.drive_mode`.
2. **G2 BLOCKS (authoring the M35): `VehicleDef` has no axle layout, duals, portal, or per-axle tyre, unsprung mass and brake share**; one ride frequency each for
   "front" and "rear" (which is the middle of three?). Fix: `AxleDef { tyres_per_side, dual_spacing_m, hub_reduction, portal_drop_m, layout: AxleLayout { Independent,
   Solid { spring_track_m }, Tandem { group, pivot_frac } }, brake_share, tyre, unsprung_mass_kg }`.
3. **G3 HACK (8 of the M35's 10 wheels): shared springs.** A walking beam, an inboard leaf spring (roll stiffness 3.4 times too high otherwise) and rigid duals need
   one spring on a weighted sum of wheel travels. Only `AntiRollDef` exists: 29 couplers (25 with negative rates), exact for linear springs only (a progressive leaf is
   1.28 times a station's preload wrong; an uncoupled compile loads a bumped hub 3 times too hard) and booked to the `AntiRoll` ledger term on a truck with no bar.
   Fix: `PhysRig.shared_springs: Vec<SharedSpringDef { name, terms: Vec<(station, weight)>, spring, preload_n, damper }>` (travel `h = sum w_i c_i`, force on
   station `i` is `w_i F(h)`); four definitions replace the 29 couplers and are exact for a table spring. Ports unchanged.
4. **G6 HACK (30 to 40 mm): no ground reference, and `radius_m` is free or loaded, unstated.** The contract's own `box_truck` is 21.8 mm off its tyre model and not in
   static equilibrium (1,960 N m pitch residual); M35 loaded radii differ 3.8% front to rear (= 3.8% slip through a locked transfer case). Fix: `PhysRig.ride_height_m`
   (datum above ground at the design pose) and "`radius_m` is the FREE radius".
5. **G12 cosmetic but cheap: `validate()` accepts 14 of 14 broken rigs** (duplicate names, output and station mismatch, an output missing from the tree, NaN, non-positive-definite
   inertia, `bump_dir` of length 0.6, 1000 substeps). Fix: merge the deeper validation and add `MAX_SUBSTEPS = 8`.
6. **G5 HACK: `BrakeDef` has no apply or release lag, circuit or driveline site.** Air lag alone is +16% stopping distance (the tolerance is 15%); the M35 handbrake is a
   transfer-case drum. Fix: `apply_time_s`, `release_time_s`, `circuit`, `site: BrakeSite { Wheel, Driveline }`.
7. **G4 HACK: one contact patch per station.** M35 duals cost 10 stations, 22 joints, 18 brakes and 4 penalty "rigid hub" couplers. Fix: `WheelDef.patches_x_m: Vec<f64>`
   (6 stations, 14 joints); contact names `<station>` or `<station>.<k>`.
8. **G8 HACK: `ShaftState` has no shaft load torque**; `Diff` has no split or efficiency; `bias` is undefined. M35 low 1st: reflected engine inertia is 9.5 times the wheel side, so a
   locked driveline cannot be solved exactly. Fix: `ShaftState.load_torque_nm` and `load_stiffness_nm_s_rad`; `Diff.split` and `Diff.efficiency`; document `bias` as a torque-bias ratio >= 1.
9. **G7 HACK: proxies cannot ride an unsprung axle** (the M35 diff housing moves +-0.14 m on 0.26 m of clearance). Fix: `CollisionProxy.attached_station: Option<usize>`.
10. **G9 HACK (not needed by these two): more than one steered axle gets one `Command.steer`**; no mode, schedule, rate limit or Ackermann reference. Fix: `SteerDef { group,
    rate_limit_rad_s, speed_gain }`, `Command.steer_mode`.
11. **G10 cosmetic: `JointBinding` is one coordinate at unit gain** (no half-shafts, axle housings, beams). Fix: `JointBinding { scale, terms }` and `PhysRig::joint_layout()`.
12. **G11 cosmetic: no leaf-pack dry friction.** Fix: `SuspensionDef.friction_n`.

**Quick wins (global):** the stand-in wrote `fl.steer = +0.53` while the vehicle yawed right (docs say positive = left; fixed in 0.1.0 before this report reached ARCH);
the documented contact frame (forward, right, up) is left-handed; rig JSON through `serde_json` is not bit-exact (17 numbers one ulp off in the M35) so enable `float_roundtrip`;
`mu_peak_ref` times `Material.mu_peak` may double count.

## Documents that were wrong or ambiguous
C0 the contact frame is left-handed and `mz_nm` has no sign rule; C1 `Diff.bias`, `Diff.ratio` direction and compounding, `OutputDef.final_drive_ratio` and `efficiency`,
`Clutch.engage_rpm`, the converter K-factor convention; C2 `SpringKind::Table` (absolute force including preload, or an increment? signed compression for droop? role of
`preload_n`?); C3 `AntiRollDef` sign and whether negative is legal; C4 free or loaded `radius_m`, `preload_n` is the sprung share (the tyre carries that plus unsprung
weight); C5 `patch_length_m` "at rated load" (no rated load), `mu_peak_ref` times `Material.mu_peak` (product or ratio), rolling coefficient in both `TyreDef` and `Material`;
C6 `Command.steer` to `<station>.steer` mapping, `max_angle_rad` of which wheel, the Ackermann reference axle for three axles; C7 brake torque at the wheel or before a portal,
whether `parking` also answers the service pedal, and where reflected inertia is measured (1.92 squared = 3.7 times apart); C8 `VehicleFrame.contacts` per station, patch or
sample, in what order; C9 `RenderRig.joint_count` (largest bound index plus one, or `joint_names().len()`); C10 `validate()` overclaims; C11 `IntegrationDef.substeps` "stiffest
mode" is undefined with or without bump stops (3 or 5) and silent on the slip mode; C12 the reaction path of a non-axial tyre force; C13 `serde_json` round trip; C14 datum
location; C15 whether `Frame.joints` spin is wrapped or accumulated (the stand-in wraps; 30 Hz frames alias above about 94 rad/s).

## Stiffness and substeps (hull clamped; tyre, spring, bump stop and couplers on the unsprung masses)
| rig | hop | on stops | rule `ceil(20 f / 60)` | declared | `omega dt` | slip oscillator |
|---|---|---|---|---|---|---|
| M998 | 8.2 Hz | 12.1 Hz | 5 (3 without stops) | 5 | 0.25 | 12.6 Hz |
| M35 | 10.8 | 14.8 | 5 (4) | 5 | 0.31 | 11.7 Hz |
| contract `box_truck` | 11.8 | 14.6 | 5 (4) | 4 | 0.38 | 25.1 Hz, which needs 9 |
- Five substeps (300 Hz, 20 to 25 steps per period of the stiffest vertical mode) is well inside the tripwire of 8.
- The rule's answer depends on whether bump stops count (3 against 5). **The stiffest mode may not be vertical:** a free wheel with an instantaneous slip-ratio force has the
  rate `C_s F_z r^2 / (J v)` (explicit-unstable below 3.7 to 4.1 m/s at the declared substeps); with `relaxation_length_m` as a lag it becomes an oscillator with a
  speed-independent `omega_n^2 = C_s F_z r^2 / (sigma J)`: 12.6 and 11.7 Hz (fine) but 25.1 Hz for `box_truck` (wheel inertia 1.4 kg m^2 on a 0.4 m wheel), 9 substeps against 4 declared.
- The dual-hub penalty couplers raise the M35's stiffest mode by 2.2 Hz; `patches_x_m` removes them.
- **The real numerical risk is the port coupling, not the substep count** (G8): one substep value serves chassis and driveline although DRIVE's clutch spike wants 240 to 480 Hz.

## Migration cost
Only Rust struct literals break (every file format stays loadable): `Command::NEUTRAL`, `DriveInputs` and `DriveTelemetry` literals, `AxleDef` in `dummy_vehicle_def`,
`PhysRig`, `WheelDef`, `CollisionProxy`, `Diff`, `ShaftState`, `SteerDef`, `JointBinding` and `SuspensionDef` literals, `RigidBoxVehicle` and `run_stand_in` contact sizing.
