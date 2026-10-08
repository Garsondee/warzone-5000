# ADR-0006: Units, frames and sign conventions

**Status:** Accepted (ARCH, 2026-10-08). Replaces the prototype's coherent tonne / kN / kW system, which existed to keep Q32.32 numbers in range.

## Decision
1. **SI everywhere**: metres, kilograms, seconds, newtons, watts, radians, pascals, kelvin. The unit is the **field-name suffix** (`mass_kg`, `torque_nm`, `omega_rad_s`, `rate_n_m` for N/m, `rate_nm_rad` for N m/rad). km/h, kW, degrees and rpm exist only at the edges: display, spec sheets and RON authoring (`friction_angle_deg`, `rpm` fields on engines), converted on load by named helpers (`w5k_math::scalar`).
2. **Frame (Godot's, so nothing converts at the front-end boundary):** right-handed, **+Y up, -Z forward, +X right**. The hull datum is the design reference point of the `PhysRig`; positions in a rig are in the hull frame.
3. **Rotations:** quaternions stored w-x-y-z, mapping body coordinates to world coordinates. **Yaw** is about +Y and positive turns the nose **left**; **pitch** is about +X and positive raises the nose; **roll** is about the forward axis (-Z) and positive lowers the **right** side. `Quat::from_ypr` composes yaw, pitch, roll intrinsically; tests in `w5k_math` pin the signs.
4. **Driver inputs:** `Command::steer` is +1 = **right**, which is a *negative* yaw rate. Throttle and brake are 0..1. Turret yaw targets use the yaw convention (positive = left); gun pitch positive = up.
5. **Shaft and wheel signs:** positive shaft speed and torque drive the vehicle **forward**. Wheel spin as a joint coordinate is positive when **rolling forward** (the render axis is -X). Suspension travel is positive when **compressed** (wheel moves toward the hull). Recoil is positive **rearward**.
6. **Joint coordinate order** in a frame is fixed by `PhysRig::joint_names()`: all station spins, then steered stations' steer angles, then all travels, then the articulation joints in rig order.
7. **Contact frame** (tyres, track samples): x along the rolling direction (forward positive), y lateral (right positive), z along the surface normal (up positive).

## Consequences
`docs/architecture/UNITS-AND-FRAMES.md` is the one-page version with a worked example; reference tests assert the signs ("positive drive torque accelerates forward", "steering right moves the vehicle to +X"). The viewers apply joint coordinates using the axis stored in each `JointBinding`.
