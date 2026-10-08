# Contracts

The interfaces every lane builds against. The code is `crates/w5k_contract` (data and traits only, no physics, no rendering); this page says what each
contract *means*, who produces and consumes it, and how it may change. **Version: 0.1.0 (Launch Kit draft).** The settling round (each lane's design note
and CCRs) hardens it into v0.2; it freezes at M1 (first light).

## Changing a contract (CCR)
A contract change request is a PR titled `CCR: <what>` that touches `crates/w5k_contract/**` and this file (and, if it breaks users, makes the mechanical
migration edits in the lanes that use it; ARCH reviews every line). Additive, optional fields are a minor bump; anything that breaks a user is a major bump with a
migration of the goldens. ARCH tags the commit `contract-vX.Y`. **Lanes pin the tag they build against and upgrade deliberately.** Until M1 the contract is a
draft: file a CCR early rather than work around a gap.

## The contracts
| Contract (module) | Meaning | Producer | Consumers | Stand-in (`testing`) |
|---|---|---|---|---|
| `Param` (`param`) | a constant with its pedigree: value, band, provenance `Spec/Measured/Estimate/Tuned`, source | every lane that authors RON | validation, forge | `Param::estimate(..)` |
| `VehicleDef` (`def`) | designer-level description of a vehicle (RON): hull, running gear, suspension sliders, powertrain, brakes, aero, turret | designers, FORGE (reference garage) | FORGE (compile), GEOMETRY (shapes), VALIDATION (dossier link) | `dummy_vehicle_def()` |
| `PhysRig` (`rig`) | solver-level description: hull body, stations, tracks, drivetrain, articulation, aero, proxies, muzzles, substeps | FORGE (compile) | chassis, drive, terramech, vehicle glue, combat, ai | `box_truck()`, `box_tank()` |
| `RenderRig` (`render`) | meshes tagged by articulation node, joint bindings, material slots | FORGE (compile, shapes from GEOMETRY) | viewers, Godot | `box_truck()`, `box_tank()` |
| `Command` (`command`) | what a driver asks: throttle, brake, steer, gear, parking brake, turret yaw and gun pitch targets, fire | human, AI, scenario scripts | any `VehicleModel` | `ScriptedCommands` |
| `Frame`, `ReplayHeader`, `Event` (`frame`) | one snapshot of every vehicle, and the header that names the joints; a viewer is a pure function of these | simulation (via `w5k_replay`) | viewers, Godot, validation | `truck_over_bumps()`, `tank_slew_and_pitch()` |
| `WorldQuery`, `Material`, `MaterialTable` (`world`) | height, normal, material and soil parameters, ray casts, props in a box | WORLD | chassis, terramech, combat, ai | `FlatPlane`, `BumpStrip` |
| `DrivePort`, `SuspensionElement`, `ContactElement` (`ports`) | the seams between the physics lanes | DRIVE / CHASSIS / TRACKS implement them | the vehicle glue | `ConstantTorquePowertrain` |
| `ForceLedger`, `ForceTerm` (`ledger`) | every force term on every body, every tick | physics lanes add rows | vehicle glue, viewers, validation | `ForceLedger::on()` |
| `VehicleModel`, `StepReport`, `LimitingFactor` (`vehicle`) | what the scheduler steps: step, frame, hash | `w5k_vehicle` (real), `RigidBoxVehicle` (stand-in) | sim, validation, ai | `RigidBoxVehicle` |
| `CapabilityTable` (`capability`) | what a vehicle can do (grade, step, trench, soil go/no-go, braking, turning), measured by running the vehicle | scenario runner | ai | `CapabilityTable::default()` |

## Key semantics (read before you code against them)
**PhysRig.** The hull is one 6-DoF rigid body (`hull.mass_kg` is the *sprung* mass; stations carry their own `unsprung_mass_kg`; `total_mass_kg()` sums everything). Positions are in the hull frame at the design
datum. A **station** is a wheel (or road wheel, sprocket, idler) with a suspension travel coordinate along `bump_dir` (positive = compressed), an optional steer angle, a spin angle and an optional driven output.
`rest_pos_m` is the wheel centre at the design ride height; `preload_n` is the spring force there (the static share of weight). **Tracks** loop over stations (belt order front to back) and own the ground contact
(`samples` contact points along the footprint); their road wheels only load the belt. The **drivetrain** is an engine, a coupling, a gearbox, a `DriveNode` tree (diffs, transfer case, steering unit) down to numbered
**outputs**, each bound to a station, plus brakes. The **articulation** list is a tree of revolute or prismatic joints hanging off the hull (turret yaw, gun pitch, recoil), each carrying a body and an optional servo.
`integration.substeps` is set when the rig is baked from its stiffest mode; a rig that needs too many is rejected as a numerically unstable design. `PhysRig::validate()` must pass before a solver sees a rig.

**RenderRig.** A node tree; one joint binding per node, so a wheel is a chain `travel` (prismatic along `bump_dir`) > optional `steer` (revolute about +Y) > `wheel` (revolute about -X, so positive spin rolls forward);
the tank's gun is `turret_yaw` > `gun_pitch` > `gun_recoil`. A viewer applies `Frame.joints[binding.index]` about/along `binding.axis` and draws each node's meshes. No UVs: camo and weathering are triplanar,
driven by position, normal and the per-vertex `edge` and `cavity` flags.

**Frame.** `joints` follows `PhysRig::joint_names()` (spin, steer of steered stations, travel, articulation). Quantisation and rate (about 30 Hz) are VIEWER's business; viewers interpolate.

**Ports.** All three are plain traits so each physics lane can be tested alone.
- `DrivePort::step(dt, inputs, shafts, torque_out)`: one call per substep. `shafts` carries each driven output's speed, reflected inertia and the vehicle speed; `torque_out` receives the net torque (drive minus brake) per output. Contract: never reverse a stopped shaft by braking alone.
- `SuspensionElement::step(compression, rate, dt)` returns the force along the strut and its parts (spring, damper, bump stop) for the ledger.
- `ContactElement::step(input)` takes the patch kinematics in the contact frame (penetration and rate, velocities over the ground, the rolling member's surface speed, the ground material) and returns forces and the self-aligning moment, the shaft reaction torque, sinkage, slip and a saturation flag.

**WorldQuery.** Deterministic and side-effect free (`&self`): vehicles are stepped against an immutable world. `height_m`, `normal`, `material_id_at`, `raycast`, `props_in_aabb`, `bounds`. A `Material` is the runtime flattening of a `MaterialDef` (every number a `Param` in RON);
soft ground carries Bekker-Wong `SoilParams` in SI.

**ForceLedger.** Bodies are numbered hull = 0, then stations, then articulation joints, in rig order. Forces are world-frame, N and N m. Switched off in bulk runs (`ForceLedger::off()`); then `add` costs a branch.

## Reference substep order (the glue `w5k_vehicle` implements; the settling round may refine it)
For each substep of length `dt / substeps`:
1. Wheel and sample positions from the hull pose and each station's travel; probe the world for height, normal and material.
2. Suspension: compression and rate per station, then `SuspensionElement::step`; reaction on the hull and the unsprung mass.
3. Contact: `ContactElement::step` per tyre or track sample; rotate the contact-frame force to the world; apply at the contact point; the vertical force carries the unsprung mass.
4. Powertrain: gather `ShaftState`, `DrivePort::step`, integrate each wheel or sprocket (`J d(omega)/dt = T_drive - shaft_reaction`).
5. Gravity, aero drag, anti-roll; articulation servos and their reaction wrenches on the hull.
6. Sum the wrench on the hull about its centre of mass and integrate with semi-implicit Euler (velocity first; `Quat::integrate_world` for orientation); integrate unsprung travel the same way.
7. Record every term in the ledger; flush decaying state (`scalar::flush_tiny`).

## Change log
- **0.1.0** (Launch Kit): first draft. All of the above, with stand-ins, two dummy rigs, canned replays and the first-light spine.
