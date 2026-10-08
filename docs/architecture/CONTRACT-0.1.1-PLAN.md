# Contract 0.1.1: the pre-launch hardening batch (decided, partly drafted, NOT yet applied)

**Status (2026-10-08, end of the Launch Kit session).** Three read-only red-teams (`docs/architecture/redteam/{wheeled,tracked,weapons}.md`) tried to write an M998, an M35 6x6, a
Sherman, an Abrams and a Leopard 2 against contract 0.1.0 and found real gaps. ARCH decided every one of them (below). Six files of the batch were drafted and are saved in
`docs/architecture/contract-0.1.1-wip/*.rs.txt` (they are **not** in `crates/` and do not compile together yet, because the rest of the crate and the stand-ins are not migrated).
`crates/w5k_contract` is still exactly 0.1.0 and green; `integration` is at the green 0.1.0 commit. **Do not launch any lane until this batch is applied and tagged `contract-v0.1`.**

## How to apply it (about half a day of ARCH work; each step ends green)
1. Copy the drafts into `crates/w5k_contract/src/` (drop the `.txt`): `rig.rs` (rewritten: linkages, patches, drive modes, steering law, brake extras, free turbine, joint drives, weapons,
   `PhysRig.combat`, `required_features()`, `rig_hash()`, `contact_names()`), `combat.rs` (new; PROVISIONAL types for armour, modules, mass items, sensors), `kinematics.rs` (new; forward
   kinematics, composite mass, subtree inertia), `validate.rs` (new; the consolidated `PhysRig::validate()`), `command.rs` (aim channels, level-triggered fire bits, clutch, drive mode),
   `frame.rs` (contact material, livery, terrain link, limiting factor, weapon frames, projectiles, richer `Event::Hit`). Add `pub mod combat; pub mod kinematics; mod validate;` to `lib.rs`;
   make `serde_json` a regular dependency of the contract crate with `float_roundtrip` (workspace entry: `serde_json = { version = "1", features = ["float_roundtrip"] }`); raise the
   contract line budget in `docs/swarm/budgets.toml` to about 7000.
2. Still to write (not drafted): `ports.rs` (below), `vehicle.rs` (`drain_events`, `swap_rig` default methods; `LimitingFactor` gets serde derives), `ledger.rs` (`ForceTerm::BellyDrag = 20`,
   `#[non_exhaustive]`, torque is **about the body's centre of mass**), `render.rs` (`#[non_exhaustive]` on `NodeRole`/`SlotKind`; `joint_count` = `joint_names().len()`, each index bound at most
   once, `edge`/`cavity` empty or as long as `positions`), `world.rs` (`PropRef.ballistic`, `RayHit.exit_distance_m`), `def.rs` (below), `capability.rs` (`rig_hash` is `PhysRig::rig_hash()`).
3. `ports.rs`: `DriveInputs` += `clutch: Option<f64>`, `drive_mode: u8`; `DriveTelemetry` += `drive_mode`; `ShaftState` += `load_torque_nm`, `load_stiffness_nm_s_rad` (last substep's contact
   reaction, positive resists forward rolling, and its slope against shaft speed); `ContactInput`/`ContactOutput` docs (frame below; `slip_ratio` normalised); new `ArticulationPort`
   (`HullMotion`, `ArticulationWrench`, `Shot`; the shapes are in `redteam/sketches/weapons-proposed.rs.txt`).
4. `def.rs` (all `#[serde(default)]`, additive): `AxleDef` += `tyres_per_side`, `dual_spacing_m`, `hub_reduction`, `portal_drop_m`, `layout: AxleLayout { Independent, Solid { spring_track_m },
   Tandem { group, pivot_frac } }`, `brake_share`, per-axle `tyre`, `unsprung_mass_kg`; `TrackedDef` += return rollers, idler, belt thickness and length, bogies, arm length, damped stations,
   `shoe_mu_scale_soft`, wheel-contact stiffness; `SuspensionKind::Volute`; `SteeringUnitSliders.law`; `BrakesDef` location; `GunDef` += `projectile_mass_kg`, `recoiling_mass_kg`,
   `recoil_impulse_factor`, `reload_s`, `rounds_carried`, `stabiliser_rejection_yaw`; `TurretDef` += ring position; `VehicleDef.mounts: Vec<MountDef>` (coax, cupola, loader, bow MGs).
   Armour sliders are FORGE's and COMBAT's CCR in the settling round (GEOMETRY supplies closed shells; COMBAT owns materials and penetration kernels).
5. Stand-ins (`testing/`): migrate every struct literal; make `box_truck` and `box_tank` pass the new `validate()` (a sensible `ride_height_m`; spring preloads that carry the sprung weight;
   `box_tank` bump stops 3 to 6 MN/m, travel above its static deflection, servo `max_accel_si <= max_effort_si / I`, a recoil joint with a real `RecoilDef`, a weapon on its muzzle);
   `RigidBoxVehicle` writes continuous (unwrapped) spin, fills `ang_vel`, emits `Event::Fired` through `drain_events`, routes `aim` channels and `fire` bits, and its steer sign follows the
   documented rule; the canned replays vary contact `slip` and `material` and fire the gun. Re-bless the first-light golden deliberately (`Golden-Change: contract 0.1.1`).
6. Docs: `CONTRACTS.md` (every convention below, change log 0.1.1), `UNITS-AND-FRAMES.md` (contact frame, spin continuity, `_si` suffix, aim mapping). Then tests, `cargo fmt`, clippy with the
   bans, the constants lint (`// const-ok:` where needed), `python3 -B -I -m unittest discover -s tools/ci/tests`, a PR from `claude/sharp-babbage-d702f7` into `integration` (ARCH merges),
   tag `contract-v0.1`, update the lane briefs (next section), then the rehearsal and rank 1 per `docs/swarm/LAUNCH.md`.

## Decisions (the conventions that every lane relies on)
- **Compression** is measured from the rest position along `bump_dir`: positive toward the hull, negative in droop; for a swinging arm it is the vertical rise of the wheel centre. **Spring forces are
  totals**: a `Table` force is the total and the force at compression 0 equals `preload_n` (validated); extrapolate the end segments linearly, never return a negative force. A torsion bar's arm angle follows
  `sin(phi) = sin(phi0) - c / L`; torque `T0 + k (phi0 - phi)` with `T0 = preload * L * cos(phi0)`; wheel force `T / (L cos(phi))`. A hydropneumatic spring: `V = V0 - A c`, `F = p A`, `p V^gamma` constant.
- **`radius_m` is the free (unloaded) radius** (the pitch radius for a sprocket); `PhysRig.ride_height_m` is the datum's height above the ground, so the static tyre deflection is
  `radius - ride_height - rest_pos.y` (validated 0 to 0.35 radius). **Tyre friction** is `Material::mu_peak * TyreDef::mu_scale` (the old `mu_peak_ref` double-counted); FORGE compiles `mu_scale` as the
  VehicleDef's `mu_peak_ref` over the table's dry-hard reference surface. Rolling resistance is tyre plus surface (unchanged).
- **Ratios are reductions** (input speed / output speed), multiplicative down the tree; `bias` is a torque-bias ratio >= 1 (limited slip only); `Diff.split` and `efficiency` are explicit; a `SteerUnit`'s
  children are `[left, right]`, checked against the sides of the stations they drive. **Brake torque** is at the shaft named by `location` (`AtStation` or `BeforeFinalDrive`) and `site`
  (`Wheel` or `Driveline`); `service` is the pedal, `parking` the parking brake; band brakes have a `reverse_torque_factor`; lag and circuits are fields.
- **Contact frame is right-handed: x forward, y LEFT, z up** (the old text had y right, which is left-handed); a positive self-aligning moment is a positive yaw (turns the nose left); slip angle is positive
  when the velocity points left of the wheel's heading. **`slip_ratio` is normalised**: `(v_wheel - v_ground) / max(|v_wheel|, |v_ground|, eps)`, in -1..1 (it equals the track brief's `1 - v / v_belt` when driving).
- **Spin and every revolute joint coordinate in a `Frame` are continuous (never wrapped)**; viewers interpolate linearly (GODOT's test D6 must change accordingly). `ang_vel_rad_s` and `lin_vel_m_s` are world frame.
- **Track loop order**: `stations` start at the sprocket, run along the top run through the return rollers to the idler, then back along the ground run through the road wheels; return rollers are stations.
  The hull mass includes the top run of the belt and every rigid (spin-only) station (their `unsprung_mass_kg` is 0); `total_mass_kg` counts the ground run; belt kinetic energy is `m v^2` over the
  whole loop (`belt_length_m`), which is what reflected inertia at the sprocket uses. Track samples are **massless**; their only stiffness is `TrackDef.wheel_contact`.
- **Substeps**: baked from the stiffest *resolved* mode including bump stops, wheel hop and the slip oscillator (`omega_n^2 = C_s F_z r^2 / (sigma J)`), never from rig data that is not in the rig.
  Validation ceiling `MAX_SUBSTEPS = 16`; the design tripwire stays 8. The tracked red-team's drop tests support `omega dt <= 1` (`ceil(2 pi f / 60)`, 4 to 7 for both tanks) for impulsive stiff modes
  against `ceil(20 f / 60)` (13 and 15, over the tripwire); **spike S1 (CHASSIS) settles the rule**, with this evidence as input.
- **Shared springs are linkages** (walking beams, bogies, inboard leaf springs: two linkages per solid axle); anti-roll rates are >= 0; multi-contact wheels use `patches_x_m` (one station, many patches).
- **Weapons**: weapon numbers live in the rig (`MuzzleDef.weapon`), ballistics in `content/combat/`; `Command` has 4 aim channels and level-triggered fire bits; joints are driven by `JointDrive { Free,
  Servo, Recoil }`; servo fields carry `_si` (the SI unit of the joint's own coordinate); the stabiliser law is `|1 - r / (1 + j f / f_b)|` on the parent's absolute rate along the axis; limits are penalty
  springs (momentum conserved); a joint's slew acceleration is `min(max_accel, max_effort / I)`. **Armour data**: FORGE compiles sliders onto GEOMETRY's closed shells into `PhysRig.combat.armour`;
  COMBAT owns materials and kernels. **Damage** is a numbers-only rig swap (`degrade(&PhysRig, &Health) -> PhysRig`, `VehicleModel::swap_rig`, `PhysRig::rig_hash`). A body's mass includes its
  removable items at as-loaded fill.
- **Optional features**: a solver that does not implement an optional rig feature must refuse the rig (`PhysRig::required_features()`) instead of ignoring the field.
- **Dependencies**: `png` (image output only) is approved under ADR-0004; `serde_json` gets `float_roundtrip` (RON is exact; JSON moved 17 numbers of one rig by an ulp otherwise).

## Lane-brief edits after the batch (small)
COMBAT (servo and recoil field names, `JointDrive`, `ArticulationPort`, aim channels, armour ownership as above, S6 gets the branching tree and a hull-fixed MG; the weapons red-team's `probes.rs` is
an independent oracle for the reaction wrench); GODOT (D6: spin is continuous; interpolate linearly); CHASSIS, TRACKS, FORGE, DRIVE (read `docs/architecture/redteam/*.md`; TRACKS' design note settles how a
track sample's ground reaction reaches the road wheel's unsprung mass; CHASSIS' S1 settles the substep rule); FORGE (armour, weapons, mounts, the `mu_scale` compile, axle layouts); ARCH (an
incremental stepping API in `w5k_sim` for M2 drive mode).
