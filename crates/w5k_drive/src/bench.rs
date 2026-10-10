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

    /// A Mule-class truck: its gear set, final drive and shift points, the box truck's driveline otherwise.
    fn mule_like() -> (Powertrain, LumpedVehicle) {
        mule_with(|_| {})
    }

    /// `mule_like` with a change to its drivetrain definition first.
    fn mule_with(edit: impl FnOnce(&mut w5k_contract::rig::DrivetrainDef)) -> (Powertrain, LumpedVehicle) {
        let mut def = box_truck().0.drivetrain;
        def.gearbox.forward_ratios = vec![2.48, 1.48, 1.0, 0.75];
        def.gearbox.shift.upshift_rpm = 3500.0;
        def.gearbox.shift.downshift_rpm = 1400.0;
        def.gearbox.shift.shift_time_s = 0.35;
        def.engine.torque_curve = vec![(750.0, 300.0), (1900.0, 380.0), (3400.0, 330.0), (3900.0, 280.0)];
        def.engine.redline_rpm = 3900.0;
        def.outputs.iter_mut().for_each(|o| o.final_drive_ratio = 5.13);
        // the Mule has no separate axle-diff reduction: the box truck's 3.73 would multiply the final drive
        fn unit_stages(n: &mut w5k_contract::rig::DriveNode) {
            if let w5k_contract::rig::DriveNode::Diff { ratio, children, .. } = n {
                *ratio = 1.0;
                children.iter_mut().for_each(unit_stages);
            }
        }
        unit_stages(&mut def.driveline);
        edit(&mut def);
        let p = Powertrain::new(&def, &Tunings::shipped()).unwrap();
        let mut v = LumpedVehicle::new(2300.0, 0.4, 2.0, p.output_count());
        v.rolling_coeff = 0.015;
        v.drag_n_s2_m2 = 0.5 * 1.2 * 0.9 * 2.8;
        v.gravity_m_s2 = 9.81;
        (p, v)
    }

    #[test]
    fn a_steady_40_kmh_cruise_holds_a_high_gear_without_hunting() {
        let (mut p, mut v) = mule_like();
        let (dt, target) = (1.0 / 240.0, 40.0 / 3.6);
        // a driver: flat out for 12 s, then a PI controller on speed holds 40 km/h for the rest of 100 s
        let (mut integral, mut gears) = (0.0, Vec::new());
        for k in 0..(100 * 240) {
            let t = f64::from(k) * dt;
            let err = target - v.speed_m_s;
            integral = (integral + 0.05 * err * dt).clamp(0.0, 1.0);
            let throttle = if t < 12.0 { 1.0 } else { (0.4 * err + integral).clamp(0.0, 1.0) };
            v.step(dt, &mut p, &DriveInputs { throttle, ..Default::default() });
            let g = p.telemetry().gear;
            if gears.last().map(|&(_, last)| last) != Some(g) {
                gears.push((t, g));
            }
        }
        // before the fix the box hunted between first and second about every 3 s and never reached third
        assert!((v.speed_m_s - target).abs() < 0.3, "cruise speed {} m/s", v.speed_m_s);
        let late: Vec<_> = gears.iter().filter(|(t, _)| *t > 20.0).collect();
        assert!(late.is_empty(), "shifted during the cruise: {gears:?}");
        assert!(gears.last().unwrap().1 >= 3, "cruise should sit in third or above: {gears:?}");
        assert!(gears.len() <= 4, "hunting on the way up: {gears:?}");
    }

    #[test]
    fn the_truck_burns_fuel_in_proportion_to_work_and_idles_cheaply() {
        let (mut p, mut v) = mule_like();
        for _ in 0..(5 * 240) {
            v.step(1.0 / 240.0, &mut p, &DriveInputs::default());
        }
        let idle_rate = p.telemetry().fuel_rate_kg_s;
        assert!(idle_rate > 0.0 && idle_rate < 1.0e-3, "idle burn {idle_rate} kg/s"); // creeping on the converter at idle
        let before = p.telemetry().fuel_used_kg;
        let mut peak: f64 = 0.0;
        for _ in 0..(10 * 240) {
            v.step(1.0 / 240.0, &mut p, &DriveInputs { throttle: 1.0, ..Default::default() });
            peak = peak.max(p.telemetry().fuel_rate_kg_s);
        }
        let used = p.telemetry().fuel_used_kg - before;
        assert!(peak > 5.0 * idle_rate, "peak {peak} vs idle {idle_rate}");
        assert!(used > idle_rate * 10.0 * 2.0, "accelerating burnt {used} kg, idling would burn {}", idle_rate * 10.0);
    }

    /// Gear changes `(time, gear)` of a run in which the driver's pedal wobbles by `wobble` around what a PI controller asks for.
    fn wobbling_cruise(mean_speed: f64, wobble: f64, gain: f64, secs: f64) -> Vec<(f64, i8)> {
        let (mut p, mut v) = mule_like();
        let (dt, mut integral, mut gears) = (1.0 / 240.0, 0.0, Vec::new());
        for k in 0..((secs / dt) as usize) {
            let t = k as f64 * dt;
            let err = mean_speed - v.speed_m_s;
            integral = (integral + 0.05 * err * dt).clamp(0.0, 1.0);
            // a twitchy speed driver: proportional gain high enough that the pedal swings between lift-off and full throttle
            let base = if t < 12.0 { 1.0 } else { (gain * err + integral).clamp(0.0, 1.0) };
            // two incommensurate wobbles, so it never settles into a pattern the gearbox could sync to
            let w = wobble
                * (0.6 * w5k_math::scalar::sin(core::f64::consts::TAU * 0.35 * t)
                    + 0.4 * w5k_math::scalar::sin(core::f64::consts::TAU * 0.83 * t));
            let throttle = if t < 12.0 { 1.0 } else { (base + w).clamp(0.0, 1.0) };
            v.step(dt, &mut p, &DriveInputs { throttle, ..Default::default() });
            let g = p.telemetry().gear;
            if gears.last().map(|&(_, last)| last) != Some(g) {
                gears.push((t, g));
            }
        }
        gears
    }

    #[test]
    fn a_cruise_with_a_wobbling_pedal_holds_one_gear_for_at_least_ten_seconds() {
        for (speed_kmh, wobble, gain) in [(30.0, 0.2, 0.4), (40.0, 0.2, 0.4), (40.0, 0.35, 1.5), (35.0, 0.2, 3.0)] {
            let gears = wobbling_cruise(speed_kmh / 3.6, wobble, gain, 90.0);
            let after: Vec<_> = gears.iter().filter(|(t, _)| *t > 20.0).collect();
            let times: Vec<f64> =
                std::iter::once(20.0).chain(after.iter().map(|(t, _)| *t)).chain(std::iter::once(90.0)).collect();
            let longest = times.windows(2).map(|w| w[1] - w[0]).fold(0.0, f64::max);
            assert!(
                longest >= 10.0,
                "{speed_kmh} km/h, wobble {wobble}, gain {gain}: longest hold {longest:.1} s, gears {gears:?}"
            );
            let top = gears.last().unwrap().1;
            assert!((3..=4).contains(&top), "{speed_kmh} km/h settles in gear {top}: {gears:?}");
        }
    }

    const RPM: f64 = core::f64::consts::PI / 30.0;

    /// Terminal speed down a grade in a held gear on a closed throttle, from the force balance written independently of the simulation:
    /// gravity along the slope = rolling + drag + engine braking at the wheel (`T_drag R / (r eta)`), found by bisection.
    fn engine_braking_terminal_speed(
        grade: f64,
        gear_ratio: f64,
        rig: &w5k_contract::PhysRig,
        veh: &LumpedVehicle,
    ) -> f64 {
        let d = &rig.drivetrain;
        let (r, eta) = (veh.wheel_radius_m, d.gearbox.efficiency * d.outputs[0].efficiency);
        let total = gear_ratio * d.outputs[0].final_drive_ratio;
        let net = |v: f64| {
            let rpm = v / r * total / RPM;
            let eb = (d.engine.drag_const_nm + d.engine.drag_per_rpm_nm * rpm) * total / (r * eta);
            let g = veh.mass_kg * veh.gravity_m_s2;
            g * w5k_math::scalar::sin(grade)
                - veh.rolling_coeff * g * w5k_math::scalar::cos(grade)
                - veh.drag_n_s2_m2 * v * v
                - eb
        };
        let (mut lo, mut hi) = (0.5, 80.0);
        for _ in 0..80 {
            let mid = 0.5 * (lo + hi);
            if net(mid) > 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    }

    #[test]
    fn engine_braking_terminal_speed_down_a_grade_matches_the_force_balance() {
        let grade = 6.0_f64.to_radians();
        let rig = box_truck().0;
        for gear in [2_u8, 3] {
            let (mut p, mut v) = mule_like();
            let ratio = [2.48, 1.48, 1.0, 0.75][usize::from(gear) - 1];
            v.grade_rad = -grade;
            v.speed_m_s = 8.0;
            let go = DriveInputs { gear: w5k_contract::GearRequest::Gear(gear), ..Default::default() };
            for _ in 0..(240 * 150) {
                v.step(1.0 / 240.0, &mut p, &go);
            }
            let mut mule_rig = rig.clone();
            mule_rig.drivetrain.outputs.iter_mut().for_each(|o| o.final_drive_ratio = 5.13);
            mule_rig.drivetrain.driveline = w5k_contract::rig::DriveNode::Output(0); // unit root stage: only the gear and the final drive remain
            let expected = engine_braking_terminal_speed(grade, ratio, &mule_rig, &v);
            assert!(
                (v.speed_m_s - expected).abs() < 0.04 * expected,
                "gear {gear}: simulated {} m/s, force balance {expected} m/s",
                v.speed_m_s
            );
        }
    }

    /// A 200 s descent of a 12 degree grade at a driver-held 10 m/s, with small lightly cooled discs: peak disc temperature, and the brake
    /// pedal the driver needed early (20 s to 30 s) and late (the last 10 s).
    fn descent(gear: w5k_contract::GearRequest) -> (f64, f64, f64) {
        // small, lightly cooled discs, so the fade (and not the particular truck) is what is being tested
        let (mut p, mut v) = mule_with(|d| {
            for b in &mut d.brakes {
                b.thermal_mass_j_k = 2_000.0;
                b.cooling_w_k = 4.0;
                b.cooling_per_ms_w_k = 1.0;
            }
        });
        v.grade_rad = -12.0_f64.to_radians();
        v.speed_m_s = 10.0;
        let (dt, mut peak_temp) = (1.0 / 240.0, 0.0_f64);
        let (mut early, mut late, mut n_early, mut n_late) = (0.0, 0.0, 0.0, 0.0);
        for k in 0..(200 * 240) {
            let t = f64::from(k) * dt;
            let brake = (0.6 * (v.speed_m_s - 10.0) + 0.2).clamp(0.0, 1.0); // a little feed-forward, then trim
            v.step(dt, &mut p, &DriveInputs { brake, gear, ..Default::default() });
            peak_temp = p.telemetry().brake_temps_k.iter().copied().fold(peak_temp, f64::max);
            if (20.0..30.0).contains(&t) {
                early += brake;
                n_early += 1.0;
            } else if t > 190.0 {
                late += brake;
                n_late += 1.0;
            }
        }
        (peak_temp, early / n_early, late / n_late)
    }

    #[test]
    fn engine_braking_keeps_the_brakes_cooler_on_a_long_descent() {
        let (t_neutral, ..) = descent(w5k_contract::GearRequest::Neutral);
        let (t_auto, ..) = descent(w5k_contract::GearRequest::Auto);
        assert!(t_neutral > t_auto + 20.0, "brakes only {t_neutral} K, with engine braking {t_auto} K");
    }

    #[test]
    fn brake_fade_makes_the_driver_press_harder_as_the_discs_heat_up() {
        let (t, early, late) = descent(w5k_contract::GearRequest::Neutral);
        // brakes alone: the discs pass the fade start (600 K), the same pedal brakes less, so the driver's pedal has to go up to hold the speed
        assert!(t > 600.0, "peak disc temperature {t} K");
        assert!(late > 1.2 * early, "pedal {early} early, {late} late");
        // with engine braking the discs stay under the fade start and the pedal barely changes
        let (t_auto, early_auto, late_auto) = descent(w5k_contract::GearRequest::Auto);
        assert!(t_auto < t && late_auto < late, "auto: {t_auto} K, pedal {early_auto} -> {late_auto}");
    }
}
