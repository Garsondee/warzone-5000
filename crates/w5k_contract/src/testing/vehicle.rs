//! `RigidBoxVehicle`: a kinematic stand-in for a real vehicle. It accelerates, brakes and steers like a plausible vehicle, rides
//! the terrain with lagged pitch/roll/height, moves its suspension, spins its wheels, swings a turret and kicks a gun back, all
//! through the real `VehicleModel` interface and with the real joint layout of its rig. It has **no physics**: no forces, no
//! contact model, no powertrain. Its job is to give the viewers, the AI and the integration spine something that moves.

use w5k_math::{scalar, Quat, StateHasher, Vec3};

use crate::command::{Command, GearRequest};
use crate::frame::{ContactFrame, VehicleFrame};
use crate::ledger::{ForceLedger, ForceTerm};
use crate::rig::{JointRole, PhysRig, WheelKind};
use crate::vehicle::{LimitingFactor, StepReport, VehicleModel};
use crate::world::WorldQuery;

#[derive(Clone, Debug)]
pub struct RigidBoxVehicle {
    rig: PhysRig,
    // Derived once from the rig.
    tracked: bool,
    ride_height_m: f64,
    power_w: f64,
    mass_kg: f64,
    wheelbase_m: f64,
    max_steer_rad: f64,
    // State.
    x: f64,
    z: f64,
    yaw: f64,
    speed: f64,
    steer_angle: f64,
    zb: f64,
    pitch: f64,
    roll: f64,
    spin: Vec<f64>,
    travel: Vec<f64>,
    artic: Vec<f64>,
    recoil_vel: f64,
    prev_fire: bool,
    rpm: f64,
    gear: i8,
    started: bool,
}

impl RigidBoxVehicle {
    /// A vehicle at `(x, z)` facing `yaw` (rad, positive = left; yaw 0 faces -Z).
    pub fn new(rig: PhysRig, x: f64, z: f64, yaw: f64) -> RigidBoxVehicle {
        let tracked = !rig.tracks.is_empty();
        let ride_height_m = -rig.stations.iter().map(|s| s.rest_pos_m.y - s.wheel.radius_m).fold(f64::MAX, f64::min);
        let peak_torque_power = rig
            .drivetrain
            .engine
            .torque_curve
            .iter()
            .map(|&(rpm, t)| t * scalar::rpm_to_rad_s(rpm))
            .fold(0.0, f64::max);
        let tyre_z: Vec<f64> =
            rig.stations.iter().filter(|s| s.wheel.kind == WheelKind::Tyre).map(|s| s.rest_pos_m.z).collect();
        let wheelbase_m = if tyre_z.len() >= 2 {
            tyre_z.iter().cloned().fold(f64::MIN, f64::max) - tyre_z.iter().cloned().fold(f64::MAX, f64::min)
        } else {
            3.0
        };
        let max_steer_rad = rig
            .stations
            .iter()
            .filter_map(|s| s.steer.as_ref().map(|st| st.max_angle_rad))
            .fold(0.0, f64::max)
            .max(0.3);
        let n = rig.stations.len();
        let m = rig.articulation.len();
        let mass_kg = rig.total_mass_kg();
        RigidBoxVehicle {
            power_w: 0.85 * peak_torque_power,
            mass_kg,
            tracked,
            ride_height_m,
            wheelbase_m,
            max_steer_rad,
            x,
            z,
            yaw,
            speed: 0.0,
            steer_angle: 0.0,
            zb: 0.0,
            pitch: 0.0,
            roll: 0.0,
            spin: vec![0.0; n],
            travel: vec![0.0; n],
            artic: vec![0.0; m],
            recoil_vel: 0.0,
            prev_fire: false,
            rpm: 800.0,
            gear: 0,
            started: false,
            rig,
        }
    }

    pub fn speed_m_s(&self) -> f64 {
        self.speed
    }

    pub fn position(&self) -> (f64, f64) {
        (self.x, self.z)
    }

    pub fn yaw_rad(&self) -> f64 {
        self.yaw
    }

    fn forward(&self) -> Vec3 {
        Quat::from_yaw(self.yaw).rotate(Vec3::FORWARD)
    }

    /// World (x, z) of a point given in hull coordinates, for the current yaw.
    fn plan_point(&self, p: Vec3) -> (f64, f64) {
        let w = Quat::from_yaw(self.yaw).rotate(p);
        (self.x + w.x, self.z + w.z)
    }

    /// First-order lag toward `target`. A decaying lag walks toward denormal numbers on a long coast; flush it on purpose (see
    /// `scalar::flush_tiny`) so results do not depend on the host's denormal mode.
    fn lag(current: f64, target: f64, dt: f64, tau: f64) -> f64 {
        scalar::flush_tiny(current + (target - current) * (1.0 - scalar::exp(-dt / tau)))
    }
}

impl VehicleModel for RigidBoxVehicle {
    fn rig(&self) -> &PhysRig {
        &self.rig
    }

    fn step(&mut self, dt_s: f64, cmd: &Command, world: &dyn WorldQuery, ledger: &mut ForceLedger) -> StepReport {
        let cmd = cmd.sanitized();
        let g = scalar::G;
        // --- longitudinal: power-limited, grip-limited, drag, brake.
        let reverse = matches!(cmd.gear, GearRequest::Reverse);
        let dir = if reverse { -1.0 } else { 1.0 };
        let v = self.speed;
        let grip_a = 0.4 * g;
        let a_drive = if matches!(cmd.gear, GearRequest::Neutral) {
            0.0
        } else {
            let a_power = self.power_w / (self.mass_kg * v.abs().max(2.0));
            cmd.throttle * a_power.min(grip_a) * dir * if reverse { 0.6 } else { 1.0 }
        };
        let drag =
            (0.5 * crate::AIR_DENSITY_KG_M3 * self.rig.aero.drag_coeff * self.rig.aero.frontal_area_m2 * v * v.abs()
                + 0.02 * self.mass_kg * g * scalar::sign(v))
                / self.mass_kg;
        let brake_a = (cmd.brake * if self.tracked { 0.6 } else { 0.8 } * g) * scalar::sign(v);
        let mut v_new = v + (a_drive - drag - brake_a) * dt_s;
        // Brakes and drag cannot reverse the direction of travel.
        if v != 0.0 && v_new * v < 0.0 && a_drive * v <= 0.0 {
            v_new = 0.0;
        }
        if cmd.parking_brake && v_new.abs() < 0.3 {
            v_new = 0.0;
        }
        self.speed = v_new;
        // --- lateral: bicycle model for wheels, pivot rate for tracks. `steer_angle` follows the yaw convention (positive = left, like the
        // steer joint coordinate), so a command of +1 (full right) asks for a negative angle and a negative yaw rate.
        let target_steer = -cmd.steer * self.max_steer_rad;
        self.steer_angle = Self::lag(self.steer_angle, target_steer, dt_s, 0.15);
        let yaw_rate = if self.tracked {
            -cmd.steer * 0.6 * if self.speed.abs() < 0.5 { 1.0 } else { 0.7 }
        } else {
            self.speed / self.wheelbase_m * scalar::tan(self.steer_angle)
        };
        self.yaw += yaw_rate * dt_s;
        let f = self.forward();
        self.x += f.x * self.speed * dt_s;
        self.z += f.z * self.speed * dt_s;
        // --- ride: sample the terrain under every station, filter pitch, roll and height.
        let n = self.rig.stations.len();
        let mut heights = Vec::with_capacity(n);
        let (mut hf, mut nf, mut hr, mut nr, mut hl, mut nl, mut hrr, mut nrr, mut hm) =
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for s in &self.rig.stations {
            let (px, pz) = self.plan_point(s.rest_pos_m);
            let h = world.height_m(px, pz);
            heights.push(h);
            hm += h;
            if s.rest_pos_m.z < 0.0 {
                hf += h;
                nf += 1.0;
            } else {
                hr += h;
                nr += 1.0;
            }
            if s.rest_pos_m.x < 0.0 {
                hl += h;
                nl += 1.0;
            } else {
                hrr += h;
                nrr += 1.0;
            }
        }
        let nn = n.max(1) as f64;
        let h_mean = hm / nn;
        let zs: Vec<f64> = self.rig.stations.iter().map(|s| s.rest_pos_m.z).collect();
        let xs: Vec<f64> = self.rig.stations.iter().map(|s| s.rest_pos_m.x).collect();
        let span_z =
            (zs.iter().cloned().fold(f64::MIN, f64::max) - zs.iter().cloned().fold(f64::MAX, f64::min)).max(0.5);
        let span_x =
            (xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min)).max(0.5);
        let target_pitch = if nf > 0.0 && nr > 0.0 { scalar::atan2(hf / nf - hr / nr, 0.8 * span_z) } else { 0.0 };
        let target_roll = if nl > 0.0 && nrr > 0.0 { scalar::atan2(hl / nl - hrr / nrr, 0.8 * span_x) } else { 0.0 };
        if !self.started {
            self.zb = h_mean;
            self.pitch = target_pitch;
            self.roll = target_roll;
            self.started = true;
        }
        self.zb = Self::lag(self.zb, h_mean, dt_s, 0.2);
        self.pitch = Self::lag(self.pitch, target_pitch, dt_s, 0.12);
        self.roll = Self::lag(self.roll, target_roll, dt_s, 0.12);
        let (tp, tr) = (scalar::tan(self.pitch), scalar::tan(self.roll));
        for (i, s) in self.rig.stations.iter().enumerate() {
            let plane = self.zb - s.rest_pos_m.x * tr - s.rest_pos_m.z * tp;
            let comp = scalar::clamp(heights[i] - plane, -s.droop_travel_m, s.bump_travel_m);
            self.travel[i] = Self::lag(self.travel[i], comp, dt_s, 0.05);
            self.spin[i] += self.speed / s.wheel.radius_m * dt_s;
        }
        // --- articulation: slew toward the commanded angles at the servo's rate limit; recoil is a damped spring.
        let mut fire_event = false;
        if cmd.fire && !self.prev_fire {
            self.recoil_vel = 3.0;
            fire_event = true;
        }
        self.prev_fire = cmd.fire;
        for (k, j) in self.rig.articulation.iter().enumerate() {
            let rate = j.servo.as_ref().map(|s| s.max_rate).unwrap_or(1.0);
            match j.role {
                JointRole::TurretYaw | JointRole::GunPitch => {
                    let target = if j.role == JointRole::TurretYaw { cmd.turret_yaw_rad } else { cmd.gun_pitch_rad };
                    if let Some(t) = target {
                        let t = match j.limits {
                            Some((lo, hi)) => scalar::clamp(t, lo, hi),
                            None => t,
                        };
                        let err =
                            if j.limits.is_none() { scalar::wrap_pi(t - self.artic[k]) } else { t - self.artic[k] };
                        self.artic[k] += scalar::clamp(err, -rate * dt_s, rate * dt_s);
                    }
                }
                JointRole::Recoil => {
                    let stroke = j.limits.map(|l| l.1).unwrap_or(0.3);
                    let w = 12.0;
                    let zeta = 0.9;
                    let acc = -w * w * self.artic[k] - 2.0 * zeta * w * self.recoil_vel;
                    self.recoil_vel += acc * dt_s;
                    self.artic[k] = scalar::clamp(self.artic[k] + self.recoil_vel * dt_s, 0.0, stroke);
                }
                JointRole::Other => {}
            }
        }
        let _ = fire_event;
        // --- cosmetics: gearbox and rpm.
        let vv = self.speed.abs();
        self.gear = match cmd.gear {
            GearRequest::Neutral => 0,
            GearRequest::Reverse => -1,
            _ => (1.0 + (vv / 7.0).floor()).min(self.rig.drivetrain.gearbox.forward_ratios.len() as f64) as i8,
        };
        let ratio =
            if self.gear > 0 { self.rig.drivetrain.gearbox.forward_ratios[(self.gear - 1) as usize] } else { 3.0 };
        let wheel_r = self.rig.stations.first().map(|s| s.wheel.radius_m).unwrap_or(0.4);
        let e = &self.rig.drivetrain.engine;
        // A made-up overall ratio of 3.7 on top of the gearbox: only the shape of the rpm trace matters for a stand-in.
        self.rpm = scalar::clamp(scalar::rad_s_to_rpm(vv / wheel_r * ratio * 3.7), e.idle_rpm, e.redline_rpm);
        ledger.add(ForceTerm::Gravity, 0, Vec3::new(0.0, -self.mass_kg * g, 0.0), Vec3::ZERO);
        let limiting = if self.speed.abs() < 0.2 && cmd.throttle > 0.5 {
            LimitingFactor::Stuck
        } else if cmd.brake > 0.5 && self.speed.abs() > 0.5 {
            LimitingFactor::Brake
        } else if a_drive.abs() >= grip_a * cmd.throttle * 0.999 && cmd.throttle > 0.0 {
            LimitingFactor::Grip
        } else if cmd.throttle > 0.0 {
            LimitingFactor::Power
        } else {
            LimitingFactor::None
        };
        StepReport { limiting, speed_m_s: self.speed.abs(), substeps: 1 }
    }

    fn frame(&self) -> VehicleFrame {
        let n = self.rig.stations.len();
        let steered: Vec<usize> =
            self.rig.stations.iter().enumerate().filter(|(_, s)| s.steer.is_some()).map(|(i, _)| i).collect();
        let mut joints: Vec<f32> = Vec::with_capacity(self.rig.joint_names().len());
        joints.extend(self.spin.iter().map(|&a| scalar::wrap_pi(a) as f32));
        joints.extend(steered.iter().map(|_| self.steer_angle as f32));
        joints.extend(self.travel.iter().map(|&t| t as f32));
        joints.extend(self.artic.iter().map(|&a| a as f32));
        let pos = Vec3::new(self.x, self.zb + self.ride_height_m, self.z);
        let rot = Quat::from_ypr(self.yaw, self.pitch, self.roll);
        let f = self.forward();
        let weight_share = (self.mass_kg * scalar::G / n.max(1) as f64) as f32;
        VehicleFrame {
            vehicle: 0,
            pos_m: pos,
            rot,
            lin_vel_m_s: f * self.speed,
            ang_vel_rad_s: Vec3::ZERO,
            joints,
            engine_rpm: self.rpm as f32,
            gear: self.gear,
            contacts: (0..n)
                .map(|_| ContactFrame { flags: 1, normal_force_n: weight_share, sinkage_m: 0.0, slip: 0.0 })
                .collect(),
            ledger_n: vec![],
        }
    }

    fn hash_state(&self, h: &mut StateHasher) {
        for v in [
            self.x,
            self.z,
            self.yaw,
            self.speed,
            self.steer_angle,
            self.zb,
            self.pitch,
            self.roll,
            self.recoil_vel,
            self.rpm,
        ] {
            h.write_f64(v);
        }
        for v in self.spin.iter().chain(&self.travel).chain(&self.artic) {
            h.write_f64(*v);
        }
        h.write_u8(self.gear as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::rigs::{box_tank, box_truck};
    use crate::testing::world::{BumpStrip, FlatPlane};

    fn run(v: &mut RigidBoxVehicle, w: &dyn WorldQuery, cmd: Command, seconds: f64) {
        let mut l = ForceLedger::off();
        let dt = 1.0 / 60.0;
        for _ in 0..(seconds / dt) as usize {
            v.step(dt, &cmd, w, &mut l);
        }
    }

    #[test]
    fn the_truck_accelerates_to_a_plausible_speed_and_stops_again() {
        let (rig, _) = box_truck();
        let mut v = RigidBoxVehicle::new(rig, 0.0, 0.0, 0.0);
        let w = FlatPlane::new();
        run(&mut v, &w, Command::throttle(1.0), 10.0);
        let s = v.speed_m_s();
        assert!(s > 12.0 && s < 45.0, "10 s of full throttle gave {s} m/s");
        run(&mut v, &w, Command::braking(1.0), 10.0);
        assert!(v.speed_m_s().abs() < 1e-9, "must stop and stay stopped, got {}", v.speed_m_s());
        assert!(v.position().1 < 0.0, "forward is -Z");
    }

    #[test]
    fn steering_right_turns_the_heading_clockwise_seen_from_above() {
        let (rig, _) = box_truck();
        let mut v = RigidBoxVehicle::new(rig, 0.0, 0.0, 0.0);
        let w = FlatPlane::new();
        run(&mut v, &w, Command::throttle(0.6), 4.0);
        run(&mut v, &w, Command { throttle: 0.4, steer: 1.0, ..Command::NEUTRAL }, 6.0);
        assert!(v.yaw_rad() < -0.3, "a right turn must reduce yaw, got {}", v.yaw_rad());
        assert!(v.position().0 > 1.0, "and the vehicle ends up to the right (+X), got x = {}", v.position().0);
    }

    #[test]
    fn the_frame_carries_exactly_the_joints_the_rig_declares_and_is_finite() {
        for (rig, _) in [box_truck(), box_tank()] {
            let n = rig.joint_names().len();
            let mut v = RigidBoxVehicle::new(rig, 0.0, 0.0, 0.0);
            run(&mut v, &BumpStrip::standard(), Command::throttle(0.5), 5.0);
            let f = v.frame();
            assert_eq!(f.joints.len(), n);
            assert!(f.is_finite());
        }
    }

    #[test]
    fn bumps_move_the_suspension_and_the_body_pitches() {
        let (rig, _) = box_truck();
        let mut v = RigidBoxVehicle::new(rig, 0.0, -20.0, 0.0);
        let w = BumpStrip::standard();
        let mut l = ForceLedger::off();
        let (mut max_travel, mut max_pitch) = (0.0f32, 0.0f64);
        for _ in 0..(8.0 * 60.0) as usize {
            v.step(1.0 / 60.0, &Command::throttle(0.35), &w, &mut l);
            let f = v.frame();
            let n = v.rig().stations.len();
            for t in &f.joints[n + 2..n + 2 + n] {
                max_travel = max_travel.max(t.abs());
            }
            max_pitch = max_pitch.max(f.rot.to_ypr().1.abs());
        }
        assert!(max_travel > 0.01, "wheels must move over a bump: {max_travel}");
        assert!(max_pitch > 0.005, "the body must pitch over a bump: {max_pitch}");
    }

    #[test]
    fn the_tank_turret_slews_and_the_gun_recoils_when_it_fires() {
        let (rig, _) = box_tank();
        let mut v = RigidBoxVehicle::new(rig, 0.0, 0.0, 0.0);
        let w = FlatPlane::new();
        let aim = Command { turret_yaw_rad: Some(1.0), gun_pitch_rad: Some(0.2), ..Command::NEUTRAL };
        run(&mut v, &w, aim, 3.0);
        let n = v.rig().joint_names().len();
        let f = v.frame();
        assert!((f.joints[n - 3] - 1.0).abs() < 1e-3, "turret reached 1 rad");
        assert!((f.joints[n - 2] - 0.2).abs() < 1e-3, "gun reached 0.2 rad");
        let mut l = ForceLedger::off();
        v.step(1.0 / 60.0, &Command { fire: true, ..aim }, &w, &mut l);
        let mut max_recoil = 0.0f32;
        for _ in 0..30 {
            v.step(1.0 / 60.0, &aim, &w, &mut l);
            max_recoil = max_recoil.max(v.frame().joints[n - 1]);
        }
        assert!(max_recoil > 0.05 && max_recoil <= 0.35 + 1e-6, "recoil {max_recoil}");
    }

    #[test]
    fn two_runs_give_identical_state_hashes() {
        let hash = || {
            let (rig, _) = box_truck();
            let mut v = RigidBoxVehicle::new(rig, 0.0, 0.0, 0.0);
            run(&mut v, &BumpStrip::standard(), Command::throttle(0.5), 6.0);
            let mut h = StateHasher::new();
            v.hash_state(&mut h);
            h.finish()
        };
        assert_eq!(hash(), hash());
    }
}
