# Contract red-team: tracked vehicles (Sherman and Abrams)

*Written by a read-only reviewer agent during the Launch Kit (2026-10-08); condensed by ARCH. Every number was plausible, not sourced: this is a test of
what the contract can express, not data. The evidence crate (not committed) expressed an M4A3 Sherman and an M1A1 Abrams as `VehicleDef` + `PhysRig` +
`RenderRig` against contract 0.1.0, tagged every hack `GAP:<n>`, and compiled a sketch of each proposed type. How each finding was settled is in
`docs/architecture/CONTRACTS.md` (change log, 0.1.1).*

## What expressed cleanly
Hull, turret, gun, recoil, muzzle and servos (`turret_yaw > gun_pitch > gun_recoil`, inertia by the parallel-axis kernel); torsion arms as a rate-vs-angle
table (Abrams 184 kN/m at rest, 135 to 310 kN/m over 0.38 m); dampers on only some stations; a track wider than its wheel (`TrackDef.width_m` against
`WheelDef.width_m`: Bekker's `b` is unambiguous); sprocket at front or rear through the explicit `sprocket` and `idler` indices; return rollers as rigid
stations; a hydraulic tensioner as a horizontal `bump_dir` with a linear spring; drivetrain arithmetic that reproduces published figures (Sherman 30.1 t,
94 kPa, 42.4 km/h in 5th at 2600 rpm; Abrams 56.8 t, 95 kPa, 67.9 km/h in 4th at 3000 rpm); RON round trips exactly; `RenderRig::validate()` passes with
47 and 55 joint coordinates.

## Gaps, in priority order ([B] blocks a real vehicle, [H] forces a hack, [C] cosmetic)
1. [B] **Bogies** (Sherman VVSS, M35 tandem): two wheels share one spring. Hack: half springs plus a negative-rate same-side anti-roll element; exact only in
   the linear limit (a front wheel over a 0.2 m step sees 56 kN instead of 28 kN). Fix: `PhysRig.linkages: Vec<LinkageDef>` (members with lever weights that
   sum to 1, one suspension, a rock limit); the glue uses the weighted pivot, calls the existing `SuspensionElement::step`, and gives member `i` the share `w_i F`.
2. [B] **The steering unit has no law.** A kinematic unit fixes a speed *ratio* (one radius per gear), a hydrostatic unit fixes a speed *difference* (the
   Abrams turns R 4.5 m in 1st and 21 m in 4th, and works in neutral), a controlled differential steers by brake torque, a Tiger II has two radii per
   gear. Fix: `SteerUnit.law: SteerLaw { diff_ratio_by_gear, detents, diff_speed_rad_s, works_in_neutral, max_steer_torque_nm, steer_brakes }`.
3. [B] **Substeps cannot be baked from the rig**: road-wheel rubber and pad stiffness are not in it, and they decide the answer (13 and 15 substeps with
   invented 4 and 6 MN/m). Fix: `TrackDef.wheel_contact { vertical_stiffness_n_m, vertical_damping_ns_m }`; document that a track-sample contact is massless.
4. [H] **The belt is undescribed.** `total_mass_kg` counts the ground run (32% of the belt, zero kinetic energy) although belt kinetic energy is exactly
   `m v^2` for any closed loop (effective mass +6.6% and +7.4%; reflected inertia at the sprocket 202 and 500 kg m^2, 13 to 18 times the sprocket's own);
   no belt thickness (wheel centre = ground + radius + 70 or 90 mm); the loop order is not "front to back" (the sprocket is first or mid-list, a roller last);
   no running resistance (asphalt 0.015 gives 184 kW at 67 km/h, a tracked 0.05 gives 547 kW). Fix: `TrackDef { belt_length_m, thickness_m, resist_c0,
   resist_c1_s_m, sprocket_teeth }` (0 = unknown), `PhysRig::ground_y_m()`, stations form a closed loop, `total_mass_kg` uses `belt_length_m`.
5. [H] **Belly**: nothing says where it is or how wide (a lowest-face rule picks a skirt once skirts hang 8 cm lower; clearance from the rig is 90 mm, 19%, off).
   Fix: `CollisionProxy.role: ProxyRole { Hull, Belly, Skirt, Other }`, `ForceTerm::BellyDrag`.
6. [H] **`BrakeDef.max_torque_nm` has no reference shaft or role** (the same number gives 0.38 or 1.03 g on the Sherman, 0.45 or 1.96 g on the Abrams).
   Fix: `BrakeDef { location: AtStation | BeforeFinalDrive, steering, reverse_torque_factor }` (a band brake is self-energising, weaker in reverse rotation).
7. [H] **Torsion**: rate-vs-angle is expressible but `compression_m` is undefined (vertical rise or arc length differ by 5% in force); the 116 mm fore-aft
   arc is lost; `JointBinding` cannot turn metres into an arm angle. Fix: define compression as the vertical rise; add `StationDef.arm_pivot_m` and
   `JointBinding { scale, also }`.
8. [H] **`validate()` accepts 22 of 22 broken tracked rigs** (sprocket index 9999, reversed steer children, inertia diag(-1,-1,1), NaN centre of mass,
   duplicate names, `bump_dir` of length 0.6, a torsion arm of length 0). Fix: adopt the stricter validation (21 of 21 variants caught, both tanks pass).
9. [H] **`serde_json` without `float_roundtrip`** moves rig `f64`s by one ulp (4 Sherman values, 1 Abrams), which breaks `rig_hash` and goldens. Fix: enable the
   feature. (RON is exact.)
10. [H] **A free gas turbine** (stall at 0 rpm, spool lag, idle fuel burn): `EngineDef { free_output, response_time_s, idle_fuel_kg_s }`.
11. [H] **A manual gearbox has no clutch pedal**: `DriveInputs.clutch`, `Command.clutch: Option<f64>`.
12. [H] **Dry-friction damping, bump-stop damping and a hard limit** are missing (a viscous knee for 3 kN of friction needs 14 substeps):
    `DamperDef.friction_n`, `BumpStopDef { damping_ns_m, hard_limit }`.
13. [H] **`VehicleDef` cannot carry** bogies, rollers, idler or tensioner, belt thickness, damper stations or a steering law, and has no volute spring kind.
14. [H] **`shoe_mu_scale` is one scalar** (a steel grouser needs 0.50 on asphalt, 1.44 on mud): `shoe_mu_scale_soft: Option<f64>`.
15. [C] Payload centre of mass (23 mm); render of belt, skirts and bogie rocker; effective against physical unsprung mass (1.3 t); mass of rigid stations
    (940 kg: if the glue treats it as hull mass the hull yaw inertia is 4.3% short); dead `tension_n`; a fixed gear stage; `ContactFrame` has no position.

**Null results (fine as they are):** track slack (belt length swings 0.6%, inside the idler stroke), a track wider than its wheel, the idler as a tensioner (works
with caveats), torsion rate-vs-angle, ammunition centre of mass.

## Documents that were wrong or ambiguous
- **The contact frame was left-handed** (x forward, y right, z up: forward x right = down). The sign of the self-aligning moment was ambiguous.
- **Slip at standstill is unbounded** under `(v_wheel - v_ground) / max(|v_ground|, eps)` (2000 at a 2 m/s belt over still ground); the TRACKS brief used
  `1 - v / v_belt`; `ContactOutput.slip_ratio` is shared.
- `validate()` messages overclaimed (`det > 0` called "positive definite"; `length > 0.5` called "unit vector").
- rpm appears inside `PhysRig` (`idle_rpm`, `redline_rpm`, `torque_curve`, ...) against "rpm only at the edges".
- `box_tank` contradicted the rules (preload 32% of sprung weight, its bars would bottom at 0.31 m static against 0.25 m travel, `axle` numbered the idler 0 and
  the sprocket 6).
- Ambiguous: the loop order of a track and whether return rollers are in `stations`; what `compression_m` means for a torsion bar; whether a `Table` spring is a
  total force or an increment over `preload_n` (read as a total, the static load doubles and `validate()` cannot tell); the reference shaft of a brake; whether
  `ratio` of a diff or steering unit is a reduction or a gain and the order of `children`; `WheelDef.radius_m` is overloaded (sprocket pitch against tip radius,
  rolling radius against ground height); what "too many substeps" means; the behaviour of a bump stop at `bump_travel_m` and what its progression multiplies;
  how a track-sample ground reaction reaches the road wheel's unsprung mass (reference substep order, step 3, is a tyre statement); `AntiRollDef` sign and
  same-side semantics; physical against effective unsprung mass.

## Stiffness and substeps for 30 to 70 t tracked vehicles (the evidence)
| | Sherman 30.1 t | Abrams 56.8 t | `box_tank` |
|---|---|---|---|
| Sprung share per road wheel | 1.7 to 2.7 t | 3.4 to 3.8 t | 4.4 t |
| Unsprung per road-wheel station | 190 kg | 240 kg physical, about 150 effective | 320 kg |
| Wheel rate | 280 kN/m | 184 kN/m (135 to 310) | 138 kN/m |
| Heave f / zeta | 1.80 Hz / 0.20 | 1.14 Hz / 0.27 | none |
| Pitch f / zeta | 1.18 Hz / 0.13 | 0.71 Hz / 0.31 | none |
| Static deflection against travel | 0.094 m against 0.13 | 0.205 m against 0.26 | 0.31 m against 0.25 (bottoms) |
| Pad plus rubber stiffness (not in the rig) | up to 4 MN/m | up to 5 to 6 MN/m | none |
| Wheel hop / stop mode | 23.9 / 36.5 Hz | 25.6 / 43.6 Hz | 13 Hz |
| `ceil(20 f / 60)` | 13 | 15 | 5 (6 baked) |
| Belt | 1.0 t per side, 85 kg/m | 2.1 t per side, 146 kg/m | 85 kg/m |
| Belt inertia reflected at the sprocket | 202 kg m^2 | 500 kg m^2 | none |

- The dummy's bump stops are 3 to 7 times too soft: a 0.5 m drop bottoms an Abrams station (0.303 m of 0.26 at 0.9 MN/m; 0.238 at 6); realistic stops are 3 to 6 MN/m,
  with a peak stop force of 220 to 340 kN per wheel (6 to 10 times static).
- **The "20 steps per period of the stiffest mode" rule rejects both tanks** (13 and 15 against the tripwire of 8), and it is 3 to 5 times more conservative than the
  data supports: semi-implicit Euler is stable for `omega dt < 2`; peak force and compression of a single impulse are within 5% of converged at 3 substeps (8 for the
  Sherman at 12 MN/m); so `omega dt <= 1`, that is `ceil(2 pi f / 60)` = 4 to 7 substeps, is what the data supports. Spike S1 (CHASSIS) owns the final criterion.
- Modes that the rule misses and that do need more substeps: dry friction through a viscous knee (14); a track sample that carries belt mass (300 Hz, 100 substeps);
  a wheel directly on two pad samples (188 Hz, 63). The budget holds only if track samples are massless **and** the wheel rubber is in the rig.
- Belt kinetic energy dominates the sprocket shaft's inertia; soil shear stiffness gives modes near 3 Hz; the stiff drive modes are the clutch or converter
  engagement (DRIVE spike S-D), not the track.
