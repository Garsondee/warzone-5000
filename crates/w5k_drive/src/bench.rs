//! The lumped vehicle bench: a point mass on N driven shafts, to close the loop around the powertrain without CHASSIS.
//!
//! `(m + n J / r^2) dv/dt = sum(T_i) / r - rolling - drag - grade`, with rigid wheels (`omega_i = v / r`). Each shaft is handed the
//! vehicle's mass reflected onto it (`J + m r^2 / n`), which is what a stiff tyre makes the wheel feel. All physical numbers are fields,
//! supplied by the caller (tests, `w5k drive bench`) from RON or a spec sheet.

use w5k_contract::ports::DrivePort;
use w5k_contract::{DriveInputs, ShaftState};

#[derive(Clone, Debug)]
pub struct LumpedVehicle {
    pub mass_kg: f64,
    pub wheel_radius_m: f64,
    /// Inertia of one wheel assembly, kg m^2.
    pub wheel_inertia_kg_m2: f64,
    pub rolling_coeff: f64,
    /// `0.5 rho Cd A`, N per (m/s)^2.
    pub drag_n_s2_m2: f64,
    pub gravity_m_s2: f64,
    pub grade_rad: f64,
    /// Speed below which rolling resistance ramps to zero (so a stationary vehicle is not pushed by it), m/s.
    pub rolling_ramp_m_s: f64,
    pub speed_m_s: f64,
    pub distance_m: f64,
    torque: Vec<f64>,
}

impl LumpedVehicle {
    pub fn new(mass_kg: f64, wheel_radius_m: f64, wheel_inertia_kg_m2: f64, outputs: usize) -> LumpedVehicle {
        LumpedVehicle {
            mass_kg,
            wheel_radius_m,
            wheel_inertia_kg_m2,
            rolling_coeff: 0.0,
            drag_n_s2_m2: 0.0,
            gravity_m_s2: 0.0,
            grade_rad: 0.0,
            rolling_ramp_m_s: 0.05, // const-ok: numerical ramp for the stationary rolling-resistance sign, not a physical property
            speed_m_s: 0.0,
            distance_m: 0.0,
            torque: vec![0.0; outputs],
        }
    }

    /// Resisting force at the current speed (rolling, drag, grade), N, positive opposes forward motion.
    pub fn resistance_n(&self) -> f64 {
        let v = self.speed_m_s;
        let ramp = (v / self.rolling_ramp_m_s).clamp(-1.0, 1.0);
        let rolling =
            self.rolling_coeff * self.mass_kg * self.gravity_m_s2 * w5k_math::scalar::cos(self.grade_rad) * ramp;
        let grade = self.mass_kg * self.gravity_m_s2 * w5k_math::scalar::sin(self.grade_rad);
        rolling + grade + self.drag_n_s2_m2 * v * v.abs()
    }

    pub fn step(&mut self, dt: f64, port: &mut dyn DrivePort, inputs: &DriveInputs) {
        let n = self.torque.len();
        let (r, nf) = (self.wheel_radius_m, n as f64);
        let share = self.mass_kg * r * r / nf;
        let force = self.resistance_n();
        let slope_f = 2.0 * self.drag_n_s2_m2 * self.speed_m_s.abs()
            + self.rolling_coeff * self.mass_kg * self.gravity_m_s2 / self.rolling_ramp_m_s;
        let shaft = ShaftState {
            omega_rad_s: self.speed_m_s / r,
            inertia_kg_m2: self.wheel_inertia_kg_m2 + share,
            vehicle_speed_m_s: self.speed_m_s,
            load_torque_nm: force * r / nf,
            load_stiffness_nm_s_rad: slope_f * r * r / nf,
        };
        let shafts = vec![shaft; n];
        port.step(dt, inputs, &shafts, &mut self.torque);
        let drive: f64 = self.torque.iter().sum::<f64>() / r;
        let m_eff = self.mass_kg + nf * self.wheel_inertia_kg_m2 / (r * r);
        let dv = dt * (drive - force) / m_eff;
        // a vehicle held by its brakes does not creep backwards through a sign flip of the resistance ramp
        self.speed_m_s += dv;
        self.distance_m += self.speed_m_s * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::powertrain::{Powertrain, Tunings};
    use w5k_contract::testing::box_truck;

    fn truck() -> (Powertrain, LumpedVehicle) {
        let rig = box_truck().0;
        let p = Powertrain::new(&rig.drivetrain, &Tunings::shipped()).unwrap();
        let mut v = LumpedVehicle::new(2500.0, 0.38, 2.0, p.output_count());
        v.rolling_coeff = 0.015;
        v.drag_n_s2_m2 = 0.5 * 1.2 * 0.8 * 3.0;
        v.gravity_m_s2 = 9.81;
        (p, v)
    }

    #[test]
    fn the_truck_launches_shifts_up_and_brakes_to_a_stop() {
        let (mut p, mut v) = truck();
        let dt = 1.0 / 240.0;
        let go = DriveInputs { throttle: 1.0, ..Default::default() };
        let mut gears = Vec::new();
        for k in 0..(40 * 240) {
            v.step(dt, &mut p, &go);
            let g = p.telemetry().gear;
            if gears.last() != Some(&g) {
                gears.push(g);
            }
            assert!(v.speed_m_s.is_finite(), "NaN at step {k}");
        }
        eprintln!("v = {:.1} m/s after 40 s, gears {gears:?}, rpm {:.0}", v.speed_m_s, p.telemetry().engine_rpm);
        assert!(v.speed_m_s > 15.0, "only {} m/s", v.speed_m_s);
        assert!(gears.len() >= 3 && gears.windows(2).all(|w| w[1] >= w[0]), "gears {gears:?}");
        let stop = DriveInputs { brake: 1.0, ..Default::default() };
        let mut lowest = v.speed_m_s;
        for _ in 0..(20 * 240) {
            v.step(dt, &mut p, &stop);
            lowest = lowest.min(v.speed_m_s);
        }
        assert!(v.speed_m_s.abs() < 0.05 && lowest > -0.05, "stopped at {}, lowest {lowest}", v.speed_m_s);
    }

    #[test]
    fn two_runs_give_the_same_state_hash() {
        let hash = || {
            let (mut p, mut v) = truck();
            for _ in 0..2400 {
                v.step(1.0 / 240.0, &mut p, &DriveInputs { throttle: 0.7, ..Default::default() });
            }
            let mut h = w5k_math::StateHasher::new();
            p.hash_state(&mut h);
            h.write_f64(v.speed_m_s);
            h.finish()
        };
        assert_eq!(hash(), hash());
    }
}
