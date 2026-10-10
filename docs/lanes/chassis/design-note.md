# CHASSIS design note (settling round)

Contract pin: commit `f8f5e5d` (0.1.1). Evidence for the numbers: `spike-s1.md`.

## 1. State and integrator
Hull: position, orientation (quaternion), linear velocity, angular velocity in the world frame (13 numbers). Per station: travel `c` and rate `c'` (the unsprung mass is a real
second mass on the travel coordinate), spin `omega`, steer angle. Per contact: the tyre's slip state (two stretches). Semi-implicit Euler: forces from the current state, update velocities,
then positions with the new velocities; orientation by `Quat::integrate_world`; the gyroscopic term `omega x (I omega)` goes into the torque sum. Why: it is symplectic (energy does
not drift: S1 measured under 0.04% per minute), costs one force evaluation per step, and is deterministic. Substeps: `ceil(20 f_max / 60)` where `f_max` is the stiffest mechanical mode
(wheel hop with the bump stop engaged); S1 gives 4 to 5 for light wheeled rigs. The tyre slip state is updated with an exact exponential so it adds no stiffness limit.

## 2. Contact patch and travel coupling
A single ray along the wheel's down direction per patch (`WorldQuery` ray) from the wheel centre, then a correction to an equivalent *enveloping* height: effective ground height = ray hit
height plus a fixed-size-cylinder correction from three rays (front/centre/rear of the footprint, `patch_length_m` apart) for slopes and steps. Plan: M1 uses the one-ray version on the
bump strip (smooth) and the three-ray average if the hump test needs it; revisit with WORLD's real terrain. Penetration is measured along the ground normal; the vertical tyre spring-damper
acts on it. A station's travel is a coordinate of the wheel centre along `bump_dir` (or a circle about `arm_pivot_m`): the hull sees the strut force and the tyre force reaction at
the wheel centre via the arm geometry; the unsprung mass integrates `m_u c'' = F_tyre - F_strut - m_u g`. Wheel position is derived from the hull pose and `c`, so there is no constraint solver.

## 3. Unsprung mass
Integrated on its own travel coordinate with the same step as the hull (semi-implicit). Hard limits use `BumpStopDef.hard_limit`: clamp compression and reflect approach velocity with `restitution`.

## 4. Tyre
Brush/Pacejka-lite: normalised slip `kappa`, slip angle `alpha`; unsaturated force from `slip_stiffness` and `cornering_stiffness_per_rad` times load; saturation by the
**friction circle** scaled by `mu_peak * mu_scale * Fz` (combined: the force vector is scaled onto the circle, direction kept). Relaxation: the slip state follows the geometric slip with
the exact exponential `1 - exp(-|v| dt / sigma)`; at rest a small velocity floor `v_min` makes the stretch hold (S1: no creep). Rolling resistance `crr * Fz` against motion, smoothly zero at standstill (tanh in speed, `// const-ok`).
Self-aligning moment from pneumatic trail = fraction of `patch_length_m`. Parameters are exactly the contract `TyreDef`.

## 5. Steering
Per steered station: command `steer` times the rack ratio gives the outer-wheel angle; Ackermann geometry with `ackermann` blends parallel (0) and exact (1): inner angle
`delta_i = atan(L / (L/tan(delta_o) - track))`, about the reference axle named in the contract. Steer rate limit later (G9).

## 6. Designer sliders to rates
`k_wheel = m_corner (2 pi f)^2 / MR^2`, `c = 2 zeta sqrt(k m_corner)` with `MR` the motion ratio (wheel travel per spring travel); FORGE computes them, CHASSIS only consumes rates.
Checked by `quarter_car_*` tests: the closed form there is the same formula.

## 7. Glue API (proposal for `w5k_vehicle`)
```text
WheeledChassis::new(rig: &PhysRig) -> Result<Self, ChassisRefusal>      // refuses unimplemented required_features(), reports why
fn step(&mut self, dt_s, cmd: &Command, world: &dyn WorldQuery, drive: &mut dyn DrivePort, ledger: &mut ForceLedger)  // one substep
fn shaft_states(&self, out: &mut [ShaftState])                          // read by DRIVE
fn hull_motion(&self) -> HullMotion                                    // for ArticulationPort
fn apply_external(&mut self, f_n: Vec3, torque_nm: Vec3)               // articulation wrench
fn fill_frame(&self, ...)  fn hash_state(&self, h)
```
Substep ordering follows `CONTRACTS.md`; the glue loops over substeps and owns the 60 Hz tick.

## 8. Contract needs (CCRs expected, text only)
- CCR-1 `TyreDef`: add `speed_floor_m_s` (relaxation holds at rest), `aligning_trail_frac`, and a peak-slip location `kappa_peak`/`alpha_peak` so the curve shape is data (all `#[serde(default)]`).
- CCR-2 `CONTRACTS.md` substep rule: define `f_max` as the stiffest mechanical mode including the engaged bump stop; the slip mode is excluded because the relaxation update is exact (S1).
- CCR-3 `IntegrationDef`: allow the rig to carry its computed `f_max_hz` (diagnostic) so the cap is checkable in `validate()`.
- No change to substep order expected; step 4 (spin integration by the chassis) is as written.

## 9. CCR-chassis-4: tyre load sensitivity (slice 2, stage A)
**Problem.** `TyreDef` gives peak friction and slip stiffnesses *per unit load*, constant, so a tyre at twice its load makes twice the force and lateral load
transfer never costs an axle any grip. Real tyres are load sensitive, and that is what makes the anti-roll split front/rear (a player lever) set the handling balance.
**Model.** With `Fz0` the tyre's static load and `s(k) = 1 / (1 + k (Fz / Fz0 - 1))`: peak friction `mu * s(k_mu)`, slip and cornering stiffness `C * s(k_c)`.
`s = 1` at the static load (static results unchanged); above it the force `s * Fz` rises ever more slowly and tends to `Fz0 / k`, so it never falls. `k` in 0..1.
**Contract change (additive, `#[serde(default)]`, 0 = off):** `TyreDef.mu_load_sensitivity: f64`, `TyreDef.stiffness_load_sensitivity: f64`, and optionally
`TyreDef.nominal_load_n: f64` (0 = the solver uses the static load from the rig, preload plus unsprung weight). FORGE compiles them from the tyre archetype.
**Landed in contract 0.3.0 (`0982843`)**; CHASSIS reads the `TyreDef` fields and the shared stand-in is gone. *Before it landed:* one shared value per coefficient in `content/physics/chassis/tuning.ron` (`tyre_mu_load_sensitivity` 0.15, `tyre_stiffness_load_sensitivity` 0.3,
ESTIMATE from Pacejka ch. 4), tagged PROVISIONAL(CCR-chassis-4).
