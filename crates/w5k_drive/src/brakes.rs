//! Brakes with heat: friction torque that fades with temperature, a leaky-bucket thermal model, apply/release lag, and a rule that
//! a brake never reverses a stopped shaft.
//!
//! Principle: a brake turns kinetic energy into heat in a small piece of metal. Heat in is the braking power `T w`; heat out is cooling,
//! proportional to the temperature above ambient and growing with the airflow past the disc. The temperature is the water level in a leaky
//! bucket, and when it passes `fade_start_k` the friction coefficient falls and the same pedal brakes less.

use w5k_contract::rig::BrakeDef;
use w5k_math::scalar::{clamp, exp};
use w5k_math::StateHasher;

/// The shaft a brake acts on, as the brake needs to see it for one step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BrakedShaft {
    pub omega_rad_s: f64,
    pub inertia_kg_m2: f64,
    /// Net torque on the shaft from everything but this brake (drive minus road load), positive accelerates it, N m.
    pub other_torque_nm: f64,
    /// How the other torque falls with shaft speed, N m per rad/s (so a held shaft is solved implicitly).
    pub other_slope_nm_s_rad: f64,
    pub vehicle_speed_m_s: f64,
}

#[derive(Clone, Debug)]
pub struct Brake {
    def: BrakeDef,
    temp_k: f64,
    applied: f64,
    heat_j: f64,
}

impl Brake {
    pub fn new(def: &BrakeDef, ambient_k: f64) -> Result<Brake, String> {
        if !(def.max_torque_nm > 0.0
            && def.thermal_mass_j_k > 0.0
            && def.cooling_w_k >= 0.0
            && def.cooling_per_ms_w_k >= 0.0)
        {
            return Err("brake needs max_torque_nm > 0, thermal_mass_j_k > 0 and non-negative cooling".into());
        }
        if def.fade_end_k < def.fade_start_k || !(0.0..=1.0).contains(&def.fade_floor) {
            return Err("brake fade needs fade_end_k >= fade_start_k and fade_floor in 0..=1".into());
        }
        Ok(Brake { def: def.clone(), temp_k: ambient_k, applied: 0.0, heat_j: 0.0 })
    }

    pub fn temperature_k(&self) -> f64 {
        self.temp_k
    }
    /// Total heat put into the disc so far, J.
    pub fn heat_in_j(&self) -> f64 {
        self.heat_j
    }
    pub fn def(&self) -> &BrakeDef {
        &self.def
    }

    /// Friction multiplier at disc temperature `t_k`: 1 below the fade start, falling linearly to the floor at the fade end.
    pub fn fade_factor(&self, t_k: f64) -> f64 {
        let d = &self.def;
        if t_k <= d.fade_start_k {
            1.0
        } else if t_k >= d.fade_end_k {
            d.fade_floor
        } else {
            1.0 - (1.0 - d.fade_floor) * (t_k - d.fade_start_k) / (d.fade_end_k - d.fade_start_k)
        }
    }

    /// Advance the brake one step with demand `command` (0..1, from the pedal, the parking lever or a steer demand).
    /// Returns the torque on the shaft (opposes motion, or holds a stopped shaft), N m.
    pub fn step(&mut self, dt: f64, command: f64, shaft: &BrakedShaft, ambient_k: f64) -> f64 {
        let d = &self.def;
        let target = clamp(command, 0.0, 1.0);
        let lag = if target > self.applied { d.apply_time_s } else { d.release_time_s };
        self.applied =
            if lag > 0.0 { self.applied + clamp(target - self.applied, -dt / lag, dt / lag) } else { target };
        let reverse = if shaft.omega_rad_s < 0.0 { d.reverse_torque_factor } else { 1.0 };
        let cap = d.max_torque_nm * self.applied * self.fade_factor(self.temp_k) * reverse;
        // the torque that would leave the shaft exactly stopped at the end of the step, clamped to what the friction can give:
        // if it fits the brake sticks (holds), otherwise it slips at capacity; it can never push the shaft backwards
        let j_eff = shaft.inertia_kg_m2 + dt * shaft.other_slope_nm_s_rad;
        let t_zero = -(j_eff * shaft.omega_rad_s / dt + shaft.other_torque_nm);
        let torque = clamp(t_zero, -cap, cap);

        let w_end = shaft.omega_rad_s + dt * (torque + shaft.other_torque_nm) / j_eff;
        let heat = torque.abs() * 0.5 * (shaft.omega_rad_s.abs() + w_end.abs()) * dt;
        self.heat_j += heat;
        // exact update of a leaky bucket over a step of constant heating power
        let h = d.cooling_w_k + d.cooling_per_ms_w_k * shaft.vehicle_speed_m_s.abs();
        let p = heat / dt;
        self.temp_k = if h > 0.0 {
            let t_inf = ambient_k + p / h;
            t_inf + (self.temp_k - t_inf) * exp(-h * dt / d.thermal_mass_j_k)
        } else {
            self.temp_k + p * dt / d.thermal_mass_j_k
        };
        torque
    }

    pub fn hash_state(&self, h: &mut StateHasher) {
        h.write_f64(self.temp_k);
        h.write_f64(self.applied);
        h.write_f64(self.heat_j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def() -> BrakeDef {
        BrakeDef {
            station: 0,
            max_torque_nm: 800.0,
            thermal_mass_j_k: 9000.0,
            cooling_w_k: 20.0,
            cooling_per_ms_w_k: 4.0,
            fade_start_k: 600.0,
            fade_end_k: 900.0,
            fade_floor: 0.4,
            parking: true,
            service: true,
            location: Default::default(),
            site: Default::default(),
            steering: false,
            reverse_torque_factor: 1.0,
            apply_time_s: 0.0,
            release_time_s: 0.0,
            circuit: 0,
        }
    }

    #[test]
    fn brake_stops_a_shaft_without_reversing_it() {
        let mut b = Brake::new(&def(), 288.15).unwrap();
        let (mut w, j, dt) = (40.0_f64, 2.0, 1.0 / 120.0);
        let mut lowest = w;
        for _ in 0..(6 * 120) {
            let t = b.step(dt, 1.0, &BrakedShaft { omega_rad_s: w, inertia_kg_m2: j, ..Default::default() }, 288.15);
            w += dt * t / j;
            lowest = lowest.min(w);
        }
        assert!(lowest >= -1e-12, "shaft went to {lowest}");
        assert!(w.abs() < 1e-9, "it should be held at rest, got {w}");
    }

    #[test]
    fn brake_temperature_energy_balance_holds() {
        let mut d = def();
        d.fade_start_k = 5000.0; // no fade: the torque stays constant so the work is known
        d.fade_end_k = 6000.0;
        let mut b = Brake::new(&d, 288.15).unwrap();
        let (mut w, j, dt, ambient) = (200.0_f64, 400.0, 1.0 / 120.0, 288.15);
        let (mut e_cool, mut speed) = (0.0, 20.0);
        for _ in 0..(10 * 120) {
            let t0 = b.temperature_k();
            let tq = b.step(
                dt,
                0.5,
                &BrakedShaft { omega_rad_s: w, inertia_kg_m2: j, vehicle_speed_m_s: speed, ..Default::default() },
                ambient,
            );
            w += dt * tq / j;
            speed = w * 0.1;
            // heat leaving this step, from the mean temperature over the step (independent of the exponential used inside)
            let (t1, h) = (b.temperature_k(), 20.0 + 4.0 * speed.abs());
            e_cool += h * ((t0 + t1) / 2.0 - ambient) * dt;
        }
        let stored = 9000.0 * (b.temperature_k() - ambient);
        let work = 0.5 * j * (200.0_f64 * 200.0 - w * w); // kinetic energy removed (the brake is the only torque)
        assert!((work - (stored + e_cool)).abs() < 0.01 * work, "work {work} vs stored {stored} + cooled {e_cool}");
        assert!((b.heat_in_j() - work).abs() < 0.01 * work);
    }

    #[test]
    fn brake_fade_reduces_torque_above_the_fade_start() {
        let b = Brake::new(&def(), 288.15).unwrap();
        assert!((b.fade_factor(500.0) - 1.0).abs() < 1e-12);
        assert!((b.fade_factor(600.0) - 1.0).abs() < 1e-12);
        assert!((b.fade_factor(750.0) - 0.7).abs() < 1e-12);
        assert!((b.fade_factor(900.0) - 0.4).abs() < 1e-12);
        assert!((b.fade_factor(2000.0) - 0.4).abs() < 1e-12);
        // and it shows in the delivered torque of a hot disc
        let (mut cold, mut hot) = (Brake::new(&def(), 288.15).unwrap(), Brake::new(&def(), 800.0).unwrap());
        let shaft = BrakedShaft { omega_rad_s: 100.0, inertia_kg_m2: 1000.0, ..Default::default() };
        let (tc, th) = (cold.step(0.01, 1.0, &shaft, 288.15), hot.step(0.01, 1.0, &shaft, 288.15));
        assert!((tc + 800.0).abs() < 1e-9 && th.abs() < tc.abs() && (th + 800.0 * 0.6).abs() < 1e-9, "{tc} {th}");
    }

    #[test]
    fn parking_brake_holds_on_a_grade() {
        // a wheel shaft with a downhill pull of 500 N m (backwards): a 800 N m parking brake holds it, a 300 N m one does not
        let pull = -500.0;
        for (cap, holds) in [(800.0, true), (300.0, false)] {
            let mut d = def();
            d.max_torque_nm = cap;
            let mut b = Brake::new(&d, 288.15).unwrap();
            let (mut w, j, dt) = (0.0_f64, 30.0, 1.0 / 120.0);
            for _ in 0..(5 * 120) {
                let t = b.step(
                    dt,
                    1.0,
                    &BrakedShaft { omega_rad_s: w, inertia_kg_m2: j, other_torque_nm: pull, ..Default::default() },
                    288.15,
                );
                w += dt * (t + pull) / j;
            }
            if holds {
                assert!(w.abs() < 1e-9, "should hold, rolled at {w}");
            } else {
                assert!(w < -1.0, "should creep backwards under the grade, got {w}");
            }
        }
    }
}
