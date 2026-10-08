# Contract red-team: turret, gun and combat (a Sherman and a Leopard 2, plus unusual stations)

*Written by a read-only reviewer agent during the Launch Kit (2026-10-08); condensed by ARCH. All masses, strokes, velocities and armour figures were unsourced
placeholders; none belongs in a dossier. The evidence crate (not committed; 61 tests) wrote the vehicles for real against contract 0.1.0, tagged every workaround
`GAP(n)`, and tried to compute the six combat quantities from the rig. How each finding was settled is in `docs/architecture/CONTRACTS.md` (change log, 0.1.1).*

## Verdict
One gun station (turret > cradle > recoil) is clean. A Sherman and a Leopard 2 are **not** expressible without hacks, so the plan's red-team criterion failed until
this batch. Of the six combat questions: reaction wrench of a slewing turret = the data suffices but there is no interface; recoil momentum, line-of-sight armour,
module ownership and ammunition centre of mass = no; the stabiliser = partly (its law was undefined, a 13 to 20 times spread in benchmark B16).

## What expressed cleanly
A branching articulation tree (cupola ring and .50 beside the cradle; the Leopard's gunner sight, commander periscope and loader MG as four branches); ring against
mount (`limits: None` = unlimited ring); a per-joint servo and stabiliser (the sight is stabilised separately from the gun); `MuzzleDef.joint` naming the carrying body (coax
on the cradle, main gun on the recoiling tube); mass, centre of mass and inertia per body (a slewing turret's reaction wrench is exactly derivable); recoil as a prismatic joint
with spring and damper; `CollisionProxy.attached_to` for a tube capsule; the documented `joint_names()` order; `ForceTerm::Servo` and `Recoil`; hydropneumatic struts and torsion bars.

## Findings
**Tier A (change the shape of existing types; cheap before the freeze).**
- **G1 hack: a weapon fixed to the hull cannot carry a muzzle** (`MuzzleDef.joint: usize`); a bow MG needs a phantom zero-range joint (a permanent always-zero channel in
  every frame). Fix: `joint: Option<usize>` (`None` = hull).
- **G2 BLOCKS: `Command` has one yaw, one pitch and one fire bit.** Joints are routed by `JointRole` (the stand-in slaves every `TurretYaw` joint), so a cupola, a bow MG, a
  loader MG and a commander's periscope cannot be commanded independently; fire is edge-triggered (holding fire gives one recoil stroke, which a machine gun cannot share); aim is
  parent-relative only (no world-frame demand, so what a stabiliser holds cannot be commanded). Fix: `aim: [AimDemand; 4]` with `{yaw, pitch, frame: Parent | World}`, `fire: u8`
  level-triggered bits, `JointDef.aim_channel`; `Command` stays `Copy`.
- **G3 BLOCKS: weapon data never reaches the rig** (`GunDef.muzzle_velocity_m_s` and `TurretDef.ring_diameter_m` are orphaned; no projectile mass, recoil factor, cycle, dispersion; the
  `VehicleDef` can author exactly one gun). Fix: `MuzzleDef.weapon: WeaponDef { catalogue_id, trigger, projectile_mass_kg, muzzle_velocity_m_s, recoil_impulse_factor,
  dispersion_mrad, cycle }`; `VehicleDef.mounts: Vec<MountDef>`; `GunDef.recoiling_mass_kg`. The dynamics numbers live in the rig so the glue conserves momentum without
  reading COMBAT's content; ballistics stay in `content/combat/`, named by `catalogue_id`.
- **G4 BLOCKS M3: no articulation seam, and a vehicle cannot report that it fired** (`ports.rs` has three ports; `VehicleModel` cannot emit events, the stand-in computes the fire
  event and discards it). Fix: `trait ArticulationPort { step(dt, &HullMotion, &Command, &mut ArticulationWrench); joint_positions; drain_shots; hash_state }`, and
  `VehicleModel::{drain_events, swap_rig}` as default methods.
- **G5 hack: passive recoil is dressed as a servo** (`box_tank` clamps the slide at 6 m/s although a 120 mm recoils at 10.6 m/s: the clamp destroys 43% of the impulse);
  `RecoilDef` has no preload, gas spring or metered buffer (a linear damper peaks at 4/e = 1.47 times the constant-force ideal); the stroke is stated twice; limit enforcement is
  unspecified (a rigid clamp destroys momentum, a penalty spring conserves it; M3 asks 1e-6). Fix: `JointDrive { Free, Servo, Recoil(RecoilDef { stroke_m, spring: SpringKind, preload_n,
  damper_ns_m, damper_quad_ns2_m2 }) }`, unit suffixes on the servo fields, limits are penalty springs.
- **G6 hack: the stabiliser has no law** (three defensible readings give a 13 to 20 times spread in B16). Fix: pin the residual `|1 - r / (1 + j f / f_b)|` on the parent's absolute
  angular rate along the joint axis; `ServoDef.stabiliser: Option<StabiliserDef { rejection, bandwidth_hz, latency_s }>`.
- **G7 hack: no mass-distribution helpers and no coherence checks** (forward kinematics, composite centre of mass, sprung mass; the belt counted over its contact length makes the
  box tank 1.8 t light; no static-equilibrium check, `box_tank` springs carry 140 kN of a 431 kN weight; its servo limits contradict each other, `max_accel` 1.2 against
  `max_effort / I` = 0.75). Fix: `PhysRig::{articulation_offset, body_pose_in_hull, composite_mass, sprung_mass_kg, rig_hash}`, `TrackDef.belt_length_m`, validation rules.
- **G8 hack: the operator is part of the servo** (a gunner's shoulder rest, a hand crank that gives 1.5 deg/s instead of 24, latency 0.25 s against 0.01 s electric). Fix:
  `ServoDef.actuator: Actuator { Human(CrewRole), Hydraulic, Electric }`, `latency_s`, `fallback_rate`.
- Also now: `#[non_exhaustive]` on `JointRole`, `JointKind`, `ProxyShape`, `Event`, `NodeRole`, `SlotKind`, `ForceTerm`; documentation fixes (the ledger's torque reference point,
  `BodyDef.com_m` frame, `Command` fire level and `None` = hold, the frame of `VehicleFrame` angular velocity).

**Tier B (additive, M3; fix the conventions now).** G9 armour exists nowhere (closed solids, line of sight = chord: 63.5 mm at 47 degrees is 93.1 mm; `ProxyShape::Convex`,
`CombatDef.armour`, `PropRef.ballistic`, `RayHit.exit_distance_m`); G10 damage modules (nothing owns a volume; `ModuleDef`, `degrade()` as a pure rig transform, `swap_rig`, `rig_hash`);
G11 ammunition, fuel, crew and add-on armour are lumps inside immutable bodies (15 rounds = 321 kg = 8 mm of vehicle centre of mass; skirts and ERA, 3.3 t, matter more); G12 sights and
sensors; G13 richer `Event::Hit`, `Frame.projectiles`, `VehicleFrame.weapons`, target health.
**Tier C.** G14 azimuth-dependent gun limits and `JointKind::Fixed`; G15 hull-pitch aiming and deployable spades; G16 recoilless ejecta and backblast; G17 `CapabilityTable` knows one turret.

## The six questions, with numbers
1. Reaction wrench of a slewing turret: Sherman turret subtree yaw inertia 13,868 kg m^2 (servo effort needed 13.9 kN m against a 12.0 limit: at most 0.87 rad/s^2); Leopard 69,860 kg m^2
   (69.9 kN m of 120). With the Leopard's hull pitched 20 degrees the same slew puts 24.2 kN m of roll and pitch torque on the hull (0.15 level): "a turret slewing on a slope matters" is true.
   On soft ground the pivot resistance `mu W L / 4` is 315 kN m on mud and 70 kN m on ice for 59.5 t: the 120 kN m servo is 38% of mud and exceeds ice.
2. Recoil momentum: Sherman 76 mm J = 7.98 kN s (factor 1.44 times `m v`), free-recoil speed 10.6 m/s, hull dv 0.25 m/s; Leopard 120 mm J = 25.4 kN s (1.82); the prototype's single factor 1.3
   must be per gun (1.35 to 1.82). With spring, damper, hard stop and the impulse applied to the sliding body, momentum is conserved to 5.7e-16.
3. Line-of-sight thickness: no armour data anywhere; the proxies as armour give a 7.0 m chord through the hull for a frontal shot.
4. Module damage: engine, gearbox, fuel and ammunition have no position in the rig; the engine bay and the driver's seat both answer "hull".
5. Ammunition mass: firing the Leopard's 15-round bustle rack removes 321 kg and moves the vehicle centre of mass 8.1 mm forward and 4.8 mm down (turret 22.5 mm).
6. A stabiliser can only read the parent's absolute angular rate along the joint axis (with the hull pitched 90 degrees the elevation loop cannot cancel gun roll).

## Documents that were wrong or ambiguous
The articulation is "a tree" but `JointRole`, `Command` and `NodeRole` assume one chain; who implements the servos contradicted itself (CONTRACTS.md, LAYERS.md, `ports.rs`); the ledger
records "torque" without a reference point; `BodyDef.com_m` says "parent frame" but the code means the body's own frame; the stabiliser fields have no law; the servo limits are three
redundant numbers with no rule which binds (use `alpha = min(max_accel, max_effort / I)`); fire edge against level, and `None` = hold for a stabilised joint (relative angle or world direction?);
`VehicleFrame` linear and angular velocity frames; joint-limit enforcement; the recoil sign against `JointDef.axis`; `total_mass_kg` counts the belt over contact length; `HullDef.com_height_m`
sits under a mass that excludes the turret; `CapabilityTable.rig_hash` undefined and `Event::Hit.damage` has no unit; the consumers table says `VehicleDef` is not consumed by COMBAT although
half of `GunDef` is combat data (the cause of the two orphans).

## Recommendation (followed in 0.1.1)
Before the M1 freeze: G1 to G8, `#[non_exhaustive]`, the documentation fixes, and five conventions (a body's mass includes removable items at as-loaded fill; combat data in one
`#[serde(default)] PhysRig.combat` addressed by `Option<usize>` joint indices; damage is a numbers-only rig swap, not a mutable rig; one shape vocabulary for proxies, armour and module
volumes; VIEWER reserves an extension block and a version for projectiles and weapon state). The types of G9 to G13 can wait for M3 (additive). Extend spike S6 with the branching
tree and a hull-fixed MG; settle armour ownership in the settling round.
