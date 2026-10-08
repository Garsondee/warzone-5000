# Units, frames and signs

The most common bug in a vehicle simulator is a sign. This page is the whole convention; the tests in `w5k_math` and `w5k_contract` pin it.
(Decision: ADR-0006.)

## Units
SI: m, kg, s, N, W, Pa, rad, K. The **unit is the field-name suffix**: `mass_kg`, `length_m`, `speed_m_s`, `accel_m_s2`, `force_n`, `torque_nm`, `power_w`,
`pressure_pa`, `angle_rad`, `omega_rad_s`, `rate_n_m` (N/m), `rate_nm_rad` (N m/rad), `damping_ns_m` (N s/m), `inertia_kg_m2`, `temp_k`.
The suffix `_si` (servo and joint-drive fields) means the SI unit of the joint's own coordinate: rad, rad/s, N m for a revolute joint; m, m/s, N for a prismatic one. rpm also appears in engine data (`torque_curve`, `idle_rpm`, ...) because that is how engines are published; a solver converts it once.
km/h, kW, degrees and rpm exist only where a human reads or writes them: spec sheets, RON authoring (`friction_angle_deg`, engine `rpm`),
displays. Convert at the edge with `w5k_math::scalar` (`kmh_to_ms`, `rpm_to_rad_s`, `deg_to_rad`, ...).

## The frame
Right-handed, **+X right, +Y up, -Z forward** (Godot's convention, so the front end converts nothing). Heights are Y; plan position is (X, Z).
A vehicle's own frame (the **hull frame**) has the same axes, its origin at the rig's design datum.

```
        +Y (up)
         |
         |      forward is -Z (into the screen)
         |    /
         |  /
         +-------- +X (right)
        /
      +Z (backward)
```

## Rotations (the three that matter)
| Motion | Axis | Positive means | Memory aid |
|---|---|---|---|
| Yaw | +Y | nose turns **left** (counter-clockwise from above) | the right-hand rule on an up axis |
| Pitch | +X | nose **up** | |
| Roll | forward axis (-Z) | **right** side goes down | |

`Quat::from_ypr(yaw, pitch, roll)` composes them intrinsically in that order; `to_ypr()` inverts it. Quaternions are stored w-x-y-z and map **body to world**.
Angles from `to_ypr()` are what a driver would read on an inclinometer: heading, slope, lean.

## Driver and joint signs
- `Command::steer`: **+1 = right**, which produces a **negative** yaw rate. (This is the one sign that surprises people: steering right yaws "negative".)
- `Command::throttle`, `brake`: 0..1. `turret_yaw_rad` follows the yaw convention (positive = left); `gun_pitch_rad` positive = up.
- Positive shaft speed and torque drive the vehicle **forward**. A wheel's `spin` joint is positive when the wheel is **rolling forward** (render axis -X).
- Suspension `travel` is positive when **compressed** (wheel moves toward the hull); preload and spring force are positive when they push the wheel and hull apart.
- Recoil is positive **rearward** (+Z in the gun's frame).
- **Contact frame** for tyres and track samples (right-handed, like the yaw convention): x = rolling direction (forward +), **y = lateral, LEFT +**, z = surface normal (up +). (Until 0.1.1 this page said y = right, which is left-handed: forward x right = down.) A positive self-aligning moment about z turns the nose left.
  **Slip ratio** is normalised to -1..1: `(v_wheel - v_ground) / max(|v_wheel|, |v_ground|, eps)`, positive when the wheel is faster than the ground (driving), -1 when locked and sliding; for a driven track sample it equals `1 - v / v_belt`. The slip angle is the angle from the rolling direction to the velocity, positive when the velocity points left of the wheel's heading.
- **Aim**: `Command::aim[c]` addresses aim channel `c`; a yaw joint with `aim_channel == c` follows `yaw_rad` (positive = left), a pitch joint follows `pitch_rad` (positive = up). `AimFrame::World` means azimuth about +Y (left +, 0 along -Z) and elevation above the horizon. `None` holds the previous demand. Trigger bit `i` of `Command::fire` fires the weapon whose `trigger` is `i` (level-triggered).

## Joint coordinate order in a frame
Spin and every other revolute coordinate in a frame are **continuous (never wrapped)**: viewers interpolate them linearly, so a fast wheel is not aliased by the wrap.
`PhysRig::joint_names()` is the contract: (1) every station's `<station>.spin`, (2) every **steered** station's `<station>.steer`, (3) every station's `<station>.travel`, (4) the articulation joints in rig order.
`RenderRig` nodes bind to these by index (`JointBinding::index`) with an explicit axis, so a viewer needs no knowledge of the rig beyond its `RenderRig`.

## A worked example
A truck at the origin facing forward (-Z) with `steer = +1` (full right) at 10 m/s: the steer angle `delta` follows the yaw convention (positive = left), so `delta = -delta_max`, and the low-speed yaw rate is `(v / wheelbase) tan(delta)`, which is negative. After 3 s its heading has rotated
clockwise seen from above, its position has moved toward +X, and `to_ypr()` returns a negative yaw. The test `steering_right_turns_the_heading_clockwise_seen_from_above`
in `w5k_contract::testing::vehicle` asserts exactly this; if a lane's model disagrees, the lane's model is wrong.
