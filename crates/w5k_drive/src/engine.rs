//! The engine: `T(w, thr) = thr f(w) T_full(w) - (1 - thr) T_drag(w)`, an idle controller, a rev-limiter fade, a throttle lag and a flywheel.
//!
//! Principle: torque is published against rpm, but the solver works in rad/s. Power is torque times angular speed, so the same curve
//! gives both the "pull" (torque) and the "reach" (power) of an engine; the gearbox then trades one for the other.

use serde::{Deserialize, Serialize};
use w5k_contract::rig::EngineDef;
use w5k_contract::Param;
use w5k_math::scalar::{clamp, exp, lerp};
use w5k_math::StateHasher;

/// rad/s per rpm = 2 pi / 60.
const RPM_TO_RAD_S: f64 = core::f64::consts::PI / 30.0; // const-ok: unit conversion, mathematical

/// Controller settings shared by all engines (`content/physics/drive/engine_tuning.ron`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineTuning {
    pub idle_kp_per_rad_s: Param,
    pub idle_ki_per_rad: Param,
    pub rev_fade_fraction: Param,
}

impl EngineTuning {
    pub fn check(&self) -> Result<(), String> {
        self.idle_kp_per_rad_s.check("idle_kp_per_rad_s")?;
        self.idle_ki_per_rad.check("idle_ki_per_rad")?;
        self.rev_fade_fraction.check("rev_fade_fraction")
    }
}

/// What the engine would deliver this step (see [`Engine::prepare`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnginePrep {
    pub torque_nm: f64,
    /// How fast the closed-throttle drag grows with speed, N m per rad/s.
    pub drag_slope_nm_s_rad: f64,
}

#[derive(Clone, Debug)]
pub struct Engine {
    curve: Vec<(f64, f64)>,
    idle_w: f64,
    redline_w: f64,
    inertia: f64,
    drag_const_nm: f64,
    drag_per_rad_s: f64,
    free_output: bool,
    response_time_s: f64,
    kp: f64,
    ki: f64,
    fade_w: f64,
    omega: f64,
    demand: f64,
    idle_i: f64,
    torque_nm: f64,
}

impl Engine {
    /// Build from the rig's engine; refuses an unusable one with a reason.
    pub fn new(def: &EngineDef, tuning: &EngineTuning) -> Result<Engine, String> {
        tuning.check()?;
        if def.torque_curve.len() < 2 {
            return Err("engine torque curve needs at least two points".into());
        }
        for pair in def.torque_curve.windows(2) {
            if pair[1].0 <= pair[0].0 {
                return Err("engine torque curve must ascend in rpm".into());
            }
        }
        if def.torque_curve.iter().any(|&(r, t)| !r.is_finite() || !t.is_finite() || r < 0.0 || t < 0.0) {
            return Err("engine torque curve has a negative or non-finite point".into());
        }
        if !(def.inertia_kg_m2 > 0.0 && def.idle_rpm > 0.0 && def.redline_rpm > def.idle_rpm) {
            return Err("engine needs inertia > 0 and 0 < idle_rpm < redline_rpm".into());
        }
        let curve = def.torque_curve.iter().map(|&(r, t)| (r * RPM_TO_RAD_S, t)).collect();
        let idle_w = def.idle_rpm * RPM_TO_RAD_S;
        let redline_w = def.redline_rpm * RPM_TO_RAD_S;
        Ok(Engine {
            curve,
            idle_w,
            redline_w,
            inertia: def.inertia_kg_m2,
            drag_const_nm: def.drag_const_nm,
            drag_per_rad_s: def.drag_per_rpm_nm / RPM_TO_RAD_S,
            free_output: def.free_output,
            response_time_s: def.response_time_s.max(0.0),
            kp: tuning.idle_kp_per_rad_s.v,
            ki: tuning.idle_ki_per_rad.v,
            fade_w: tuning.rev_fade_fraction.v * redline_w,
            omega: if def.free_output { 0.0 } else { idle_w },
            demand: 0.0,
            idle_i: 0.0,
            torque_nm: 0.0,
        })
    }

    pub fn omega_rad_s(&self) -> f64 {
        self.omega
    }
    pub fn rpm(&self) -> f64 {
        self.omega / RPM_TO_RAD_S
    }
    pub fn inertia_kg_m2(&self) -> f64 {
        self.inertia
    }
    /// Torque the engine produced in the last step, N m (positive drives).
    pub fn torque_nm(&self) -> f64 {
        self.torque_nm
    }
    pub fn set_omega_rad_s(&mut self, w: f64) {
        self.omega = w.max(0.0);
    }

    /// Full-load torque at speed `w` (linear between the published points, held outside them), N m.
    pub fn full_load_nm(&self, w: f64) -> f64 {
        let first = self.curve[0];
        if w <= first.0 {
            return first.1;
        }
        for pair in self.curve.windows(2) {
            if w <= pair[1].0 {
                return lerp(pair[0].1, pair[1].1, (w - pair[0].0) / (pair[1].0 - pair[0].0));
            }
        }
        self.curve[self.curve.len() - 1].1
    }

    /// Closed-throttle drag (engine braking) at speed `w`, N m, positive magnitude.
    pub fn drag_nm(&self, w: f64) -> f64 {
        self.drag_const_nm + self.drag_per_rad_s * w
    }

    /// Rev-limiter factor: 1 below the fade band, falling linearly to 0 at the redline.
    fn limiter(&self, w: f64) -> f64 {
        if self.fade_w <= 0.0 {
            return if w < self.redline_w { 1.0 } else { 0.0 };
        }
        clamp((self.redline_w - w) / self.fade_w, 0.0, 1.0)
    }

    /// Net engine torque at speed `w` for an effective throttle `thr` (0..1), N m.
    pub fn torque_at(&self, w: f64, thr: f64) -> f64 {
        thr * self.limiter(w) * self.full_load_nm(w) - (1.0 - thr) * self.drag_nm(w)
    }

    /// Mechanical power at speed `w` and throttle `thr`, W.
    pub fn power_w(&self, w: f64, thr: f64) -> f64 {
        self.torque_at(w, thr) * w
    }

    /// Advance the flywheel by `dt_s` against an external `load_nm` (positive resists). Returns the engine torque used.
    pub fn step(&mut self, dt_s: f64, throttle: f64, load_nm: f64) -> f64 {
        let prep = self.prepare(dt_s, throttle);
        self.advance(dt_s, &prep, load_nm, 0.0);
        prep.torque_nm
    }

    /// First half of a step: run the idle controller and the throttle lag and say what the engine would deliver at the current speed.
    /// A coupling uses this to choose its torque before the flywheel moves.
    pub fn prepare(&mut self, dt_s: f64, throttle: f64) -> EnginePrep {
        let thr_cmd = clamp(throttle, 0.0, 1.0);
        let thr_idle = if self.free_output {
            0.0
        } else {
            let err = self.idle_w - self.omega;
            let unclamped = self.idle_i + self.ki * err * dt_s;
            self.idle_i = clamp(unclamped, 0.0, 1.0); // anti-windup
            clamp(self.kp * err + self.idle_i, 0.0, 1.0)
        };
        let target = thr_cmd.max(thr_idle);
        self.demand = if self.response_time_s > 0.0 {
            self.demand + (target - self.demand) * (1.0 - exp(-dt_s / self.response_time_s))
        } else {
            target
        };
        let thr = self.demand;
        EnginePrep {
            torque_nm: self.torque_at(self.omega, thr),
            drag_slope_nm_s_rad: (1.0 - thr) * self.drag_per_rad_s,
        }
    }

    /// Second half: move the flywheel against `load_nm` (positive resists), where the load rises by `load_slope` per rad/s of speed
    /// (the coupling's linearisation, solved implicitly together with the drag so a stiff load cannot blow the step up).
    pub fn advance(&mut self, dt_s: f64, prep: &EnginePrep, load_nm: f64, load_slope_nm_s_rad: f64) {
        let stiffness = prep.drag_slope_nm_s_rad + load_slope_nm_s_rad;
        let dw = dt_s * (prep.torque_nm - load_nm) / self.inertia / (1.0 + dt_s * stiffness / self.inertia);
        self.omega = (self.omega + dw).max(0.0);
        self.torque_nm = prep.torque_nm;
    }

    pub fn hash_state(&self, h: &mut StateHasher) {
        h.write_f64(self.omega);
        h.write_f64(self.demand);
        h.write_f64(self.idle_i);
        h.write_f64(self.torque_nm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::rig::EngineKind;

    fn tuning() -> EngineTuning {
        let text = include_str!("../../../content/physics/drive/engine_tuning.ron");
        ron::from_str(text).expect("engine_tuning.ron parses")
    }

    fn def(curve: Vec<(f64, f64)>, drag_const: f64, drag_per_rpm: f64) -> EngineDef {
        EngineDef {
            kind: EngineKind::Diesel,
            torque_curve: curve,
            idle_rpm: 700.0,
            redline_rpm: 4000.0,
            inertia_kg_m2: 0.4,
            drag_const_nm: drag_const,
            drag_per_rpm_nm: drag_per_rpm,
            bsfc_best_g_kwh: 220.0,
            free_output: false,
            response_time_s: 0.0,
            idle_fuel_kg_s: 0.0,
        }
    }

    fn diesel() -> Engine {
        Engine::new(
            &def(vec![(700.0, 300.0), (1800.0, 400.0), (3000.0, 350.0), (4000.0, 250.0)], 12.0, 0.006),
            &tuning(),
        )
        .unwrap()
    }

    #[test]
    fn engine_full_load_torque_matches_the_curve() {
        let e = diesel();
        for (rpm, nm) in [(700.0, 300.0), (1800.0, 400.0), (3000.0, 350.0)] {
            assert!((e.full_load_nm(rpm * RPM_TO_RAD_S) - nm).abs() < 1e-9, "at {rpm} rpm");
        }
        // halfway between 1800 (400) and 3000 (350) is 375
        assert!((e.full_load_nm(2400.0 * RPM_TO_RAD_S) - 375.0).abs() < 1e-9);
    }

    #[test]
    fn engine_power_equals_torque_times_omega() {
        let e = diesel();
        let w = 2000.0 * RPM_TO_RAD_S;
        assert!((e.power_w(w, 1.0) - e.torque_at(w, 1.0) * w).abs() < 1e-9);
        // 2000 rpm: torque 400 - 50*(400-350)/1200 ... interpolated value, power in kW
        let t = 400.0 + (350.0 - 400.0) * (200.0 / 1200.0);
        assert!((e.power_w(w, 1.0) - t * w).abs() < 1e-6);
    }

    #[test]
    fn engine_idles_at_idle_rpm_with_no_load() {
        let mut e = diesel();
        e.set_omega_rad_s(1200.0 * RPM_TO_RAD_S); // start above idle and let it fall
        for _ in 0..(8 * 240) {
            e.step(1.0 / 240.0, 0.0, 0.0);
        }
        assert!((e.rpm() - 700.0).abs() < 7.0, "idles at {} rpm", e.rpm());
    }

    #[test]
    fn rev_limiter_holds_the_redline() {
        let mut e = diesel();
        let mut peak: f64 = 0.0;
        for _ in 0..(10 * 240) {
            e.step(1.0 / 240.0, 1.0, 0.0);
            peak = peak.max(e.rpm());
        }
        assert!(peak <= 4000.0 + 1e-6, "peak {peak} rpm");
        assert!(e.rpm() > 3800.0, "it should sit just under the redline, got {}", e.rpm());
    }

    #[test]
    fn engine_spin_up_time_matches_inertia_over_torque() {
        // a flat 200 N m curve and no drag: t = J (w2 - w1) / T
        let mut e = Engine::new(&def(vec![(700.0, 200.0), (4000.0, 200.0)], 0.0, 0.0), &tuning()).unwrap();
        let (w1, w2) = (1000.0 * RPM_TO_RAD_S, 3000.0 * RPM_TO_RAD_S);
        e.set_omega_rad_s(w1);
        let dt = 1.0 / 240.0;
        let mut t = 0.0;
        while e.omega_rad_s() < w2 {
            e.step(dt, 1.0, 0.0);
            t += dt;
        }
        let expected = 0.4 * (w2 - w1) / 200.0;
        assert!((t - expected).abs() < 0.01 * expected + dt, "took {t} s, expected {expected} s");
    }

    #[test]
    fn closed_throttle_gives_the_stated_engine_braking() {
        let mut e = diesel();
        let w = 3000.0 * RPM_TO_RAD_S;
        e.set_omega_rad_s(w);
        let t = e.step(1.0e-3, 0.0, 0.0);
        let stated = -(12.0 + 0.006 * 3000.0); // drag_const + drag_per_rpm * rpm
        assert!((t - stated).abs() < 1e-9, "torque {t}, stated {stated}");
        // and the shaft decelerates by T/J
        let decel = (w - e.omega_rad_s()) / 1.0e-3;
        assert!((decel - (-stated) / 0.4).abs() < 0.01 * (-stated) / 0.4);
    }

    #[test]
    fn the_shipped_tuning_file_passes_its_checks() {
        tuning().check().unwrap();
    }
}
