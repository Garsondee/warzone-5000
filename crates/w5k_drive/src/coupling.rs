//! The coupling between engine and gearbox: direct, friction clutch, or torque converter with an optional lock-up.
//!
//! Principle (spike S-D, `docs/lanes/drive/spike-d.md`): a friction clutch is a *constraint*, not a spring. Each step we ask "what torque would make
//! engine and downstream shaft turn at the same speed at the end of this step?" and clamp the answer to the clutch's capacity. If the answer fits, the
//! clutch sticks (exactly, however big the step); if not, it slips at capacity. The converter is smooth (its torque falls as slip falls) so a
//! linearised implicit pump torque is enough.

use serde::{Deserialize, Serialize};
use w5k_contract::rig::CouplingDef;
use w5k_contract::Param;
use w5k_math::scalar::clamp;
use w5k_math::StateHasher;

use crate::engine::Engine;

/// rad/s per rpm.
const RPM_TO_RAD_S: f64 = core::f64::consts::PI / 30.0; // const-ok: unit conversion, mathematical

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CouplingTuning {
    pub clutch_engage_band_rpm: Param,
    pub converter_coupling_speed_ratio: Param,
    pub lockup_hysteresis_speed_ratio: Param,
    pub lockup_engage_time_s: Param,
    pub lockup_capacity_over_peak_torque: Param,
}

impl CouplingTuning {
    pub fn check(&self) -> Result<(), String> {
        self.clutch_engage_band_rpm.check("clutch_engage_band_rpm")?;
        self.converter_coupling_speed_ratio.check("converter_coupling_speed_ratio")?;
        self.lockup_hysteresis_speed_ratio.check("lockup_hysteresis_speed_ratio")?;
        self.lockup_engage_time_s.check("lockup_engage_time_s")?;
        self.lockup_capacity_over_peak_torque.check("lockup_capacity_over_peak_torque")
    }
}

/// The shaft the coupling drives, as the powertrain sees it (reflected to the coupling's output side).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Downstream {
    pub omega_rad_s: f64,
    pub inertia_kg_m2: f64,
    /// Net torque on that shaft from everything but the coupling (road load, brakes), positive accelerates it, N m.
    pub ext_torque_nm: f64,
    /// How much that torque falls per rad/s of shaft speed (a drag load: positive), so a lock is solved implicitly.
    pub ext_slope_nm_s_rad: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CouplingOut {
    /// Torque delivered to the downstream shaft, N m.
    pub torque_nm: f64,
    /// Engine speed minus downstream speed, rad/s.
    pub slip_rad_s: f64,
    /// Heat dissipated by friction this step, J (clutch and lock-up only).
    pub heat_j: f64,
}

#[derive(Clone, Debug)]
enum Kind {
    Direct,
    Clutch { max_torque_nm: f64, engage_w: f64 },
    Converter { stall_ratio: f64, k_rad: f64, lockup_sr: Option<f64> },
}

#[derive(Clone, Debug)]
pub struct Coupling {
    kind: Kind,
    band_w: f64,
    coupling_sr: f64,
    hysteresis: f64,
    lockup_time_s: f64,
    lockup_capacity_nm: f64,
    locked: bool,
    lock_fraction: f64,
}

/// Torque that locks engine and downstream together at the end of the step, clamped to `+-cap`. `slope` is the engine's drag slope.
fn stick_slip(dt: f64, we: f64, te: f64, je: f64, slope: f64, ds: &Downstream, cap: f64) -> f64 {
    let je_eff = je + dt * slope;
    let jd = ds.inertia_kg_m2 + dt * ds.ext_slope_nm_s_rad;
    let jeff = je_eff * jd / (je_eff + jd);
    let t_lock = jeff * ((we - ds.omega_rad_s) / dt + te / je_eff - ds.ext_torque_nm / jd);
    clamp(t_lock, -cap, cap)
}

impl Coupling {
    pub fn new(def: &CouplingDef, peak_engine_torque_nm: f64, tuning: &CouplingTuning) -> Result<Coupling, String> {
        tuning.check()?;
        let kind = match *def {
            CouplingDef::Direct => Kind::Direct,
            CouplingDef::Clutch { max_torque_nm, engage_rpm } => {
                if !(max_torque_nm > 0.0 && engage_rpm >= 0.0) {
                    return Err("clutch needs max_torque_nm > 0 and engage_rpm >= 0".into());
                }
                Kind::Clutch { max_torque_nm, engage_w: engage_rpm * RPM_TO_RAD_S }
            }
            CouplingDef::TorqueConverter { stall_ratio, k_factor_rpm_per_sqrt_nm, lockup_speed_ratio } => {
                if !(stall_ratio >= 1.0 && k_factor_rpm_per_sqrt_nm > 0.0) {
                    return Err("converter needs stall_ratio >= 1 and k_factor > 0".into());
                }
                Kind::Converter {
                    stall_ratio,
                    k_rad: k_factor_rpm_per_sqrt_nm * RPM_TO_RAD_S,
                    lockup_sr: lockup_speed_ratio,
                }
            }
        };
        Ok(Coupling {
            kind,
            band_w: tuning.clutch_engage_band_rpm.v * RPM_TO_RAD_S,
            coupling_sr: tuning.converter_coupling_speed_ratio.v,
            hysteresis: tuning.lockup_hysteresis_speed_ratio.v,
            lockup_time_s: tuning.lockup_engage_time_s.v,
            lockup_capacity_nm: tuning.lockup_capacity_over_peak_torque.v * peak_engine_torque_nm,
            locked: false,
            lock_fraction: 0.0,
        })
    }

    /// Converter torque ratio at speed ratio `sr`: `stall_ratio` at 0, falling linearly to 1 at the coupling point, 1 beyond.
    pub fn torque_ratio(&self, sr: f64) -> f64 {
        match self.kind {
            Kind::Converter { stall_ratio, .. } => {
                stall_ratio - (stall_ratio - 1.0) * clamp(sr / self.coupling_sr, 0.0, 1.0)
            }
            _ => 1.0,
        }
    }

    /// Pump torque the converter asks of the engine at pump speed `w`, `(w / K)^2`, N m.
    pub fn pump_torque_nm(&self, w: f64) -> f64 {
        match self.kind {
            Kind::Converter { k_rad, .. } => (w / k_rad) * (w / k_rad),
            _ => 0.0,
        }
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Advance engine and coupling one step. `pedal` is the manual clutch pedal (`None` = automatic); `capacity_scale` (0..1) opens a
    /// clutch for a gear shift. Moves the engine's flywheel; the downstream shaft is integrated by the caller (the chassis).
    pub fn step(
        &mut self,
        dt: f64,
        engine: &mut Engine,
        throttle: f64,
        pedal: Option<f64>,
        capacity_scale: f64,
        ds: &Downstream,
    ) -> CouplingOut {
        let prep = engine.prepare(dt, throttle);
        let (we, je) = (engine.omega_rad_s(), engine.inertia_kg_m2());
        let scale = clamp(capacity_scale, 0.0, 1.0);
        let (torque, heat) = match self.kind {
            Kind::Direct => {
                let t = stick_slip(dt, we, prep.torque_nm, je, prep.drag_slope_nm_s_rad, ds, f64::INFINITY) * scale;
                (t, 0.0)
            }
            Kind::Clutch { max_torque_nm, engage_w } => {
                let engaged = match pedal {
                    Some(p) => 1.0 - clamp(p, 0.0, 1.0),
                    None => clamp((we - engage_w) / self.band_w, 0.0, 1.0),
                };
                self.clutch_torque(dt, &prep, we, je, ds, max_torque_nm * engaged * scale)
            }
            Kind::Converter { lockup_sr, k_rad, .. } => {
                let sr = if we > 0.0 { clamp(ds.omega_rad_s / we, 0.0, 1.0) } else { 0.0 };
                if let Some(lock_at) = lockup_sr {
                    if !self.locked && sr >= lock_at {
                        self.locked = true;
                    } else if self.locked && sr < lock_at - self.hysteresis {
                        self.locked = false;
                    }
                }
                self.lock_fraction = clamp(
                    self.lock_fraction + if self.locked { dt / self.lockup_time_s } else { -dt / self.lockup_time_s },
                    0.0,
                    1.0,
                );
                if self.lock_fraction > 0.0 {
                    // the lock-up clutch ramps in alongside the converter, which carries the rest: no torque hole while it applies
                    let conv = (1.0 - self.lock_fraction) * scale;
                    let tp = conv * (we / k_rad) * (we / k_rad);
                    let slope = 2.0 * tp / we.max(f64::EPSILON); // const-ok: guard against dividing by a stopped pump
                    let tt = tp * self.torque_ratio(sr);
                    let ds_conv = Downstream { ext_torque_nm: ds.ext_torque_nm + tt, ..*ds };
                    let cap = self.lockup_capacity_nm * self.lock_fraction * scale;
                    let tc =
                        stick_slip(dt, we, prep.torque_nm - tp, je, prep.drag_slope_nm_s_rad + slope, &ds_conv, cap);
                    engine.advance(dt, &prep, tp + tc, slope);
                    return CouplingOut {
                        torque_nm: tt + tc,
                        slip_rad_s: we - ds.omega_rad_s,
                        heat_j: (tc * (we - ds.omega_rad_s)).abs() * dt,
                    };
                } else {
                    // pump load linearised about the current speed: T_p(w + dw) = T_p + (2 T_p / w) dw, solved with the engine step
                    let tp = (we / k_rad) * (we / k_rad) * scale; // a shift unloads the pump too: the engine flares, as a real automatic does
                    let slope = 2.0 * tp / we.max(f64::EPSILON); // const-ok: guard against dividing by a stopped pump
                    let before = engine.omega_rad_s();
                    engine.advance(dt, &prep, tp, slope);
                    let tp_eff = tp + slope * (engine.omega_rad_s() - before);
                    let out = tp_eff * self.torque_ratio(sr);
                    return CouplingOut { torque_nm: out, slip_rad_s: we - ds.omega_rad_s, heat_j: 0.0 };
                }
            }
        };
        // the friction paths: the engine feels the transmitted torque as its load
        engine.advance(dt, &prep, torque, 0.0);
        CouplingOut { torque_nm: torque, slip_rad_s: we - ds.omega_rad_s, heat_j: heat * dt }
    }

    fn clutch_torque(
        &self,
        dt: f64,
        prep: &crate::engine::EnginePrep,
        we: f64,
        je: f64,
        ds: &Downstream,
        cap: f64,
    ) -> (f64, f64) {
        let t = stick_slip(dt, we, prep.torque_nm, je, prep.drag_slope_nm_s_rad, ds, cap);
        (t, (t * (we - ds.omega_rad_s)).abs())
    }

    pub fn hash_state(&self, h: &mut StateHasher) {
        h.write_u8(u8::from(self.locked));
        h.write_f64(self.lock_fraction);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{coupling_tuning, diesel_engine, flat_engine};

    fn converter(lockup: Option<f64>) -> Coupling {
        let def = CouplingDef::TorqueConverter {
            stall_ratio: 2.1,
            k_factor_rpm_per_sqrt_nm: 100.0,
            lockup_speed_ratio: lockup,
        };
        Coupling::new(&def, 400.0, &coupling_tuning()).unwrap()
    }

    #[test]
    fn converter_multiplies_torque_at_stall_by_the_stall_ratio() {
        let c = converter(None);
        assert!((c.torque_ratio(0.0) - 2.1).abs() < 1e-12);
        // closed loop: turbine held at rest, full throttle; at steady state the engine torque (here ~ T_full) arrives times the stall ratio
        let (mut e, mut c) = (diesel_engine(), converter(None));
        let ds = Downstream { omega_rad_s: 0.0, inertia_kg_m2: 100.0, ..Default::default() };
        let mut out = CouplingOut::default();
        for _ in 0..(4 * 240) {
            out = c.step(1.0 / 240.0, &mut e, 1.0, None, 1.0, &ds);
        }
        let expect = 2.1 * e.torque_nm();
        assert!((out.torque_nm - expect).abs() < 0.02 * expect, "turbine torque {} vs {}", out.torque_nm, expect);
    }

    #[test]
    fn converter_torque_ratio_falls_to_one_at_the_coupling_point() {
        let c = converter(None);
        assert!((c.torque_ratio(0.85) - 1.0).abs() < 1e-12);
        assert!((c.torque_ratio(0.95) - 1.0).abs() < 1e-12);
        assert!((c.torque_ratio(0.425) - 1.55).abs() < 1e-12); // halfway: midway between 2.1 and 1
        let mut last = c.torque_ratio(0.0);
        for k in 1..=17 {
            let r = c.torque_ratio(f64::from(k) * 0.05);
            assert!(r <= last + 1e-12);
            last = r;
        }
    }

    #[test]
    fn clutch_never_transmits_more_than_its_capacity() {
        let def = CouplingDef::Clutch { max_torque_nm: 250.0, engage_rpm: 1200.0 };
        for pedal in [None, Some(0.0), Some(0.3), Some(1.0)] {
            for wd in [0.0, 50.0, 150.0, 400.0] {
                for thr in [0.0, 0.5, 1.0] {
                    let mut e = diesel_engine();
                    let mut c = Coupling::new(&def, 400.0, &coupling_tuning()).unwrap();
                    let ds = Downstream {
                        omega_rad_s: wd,
                        inertia_kg_m2: 1.2,
                        ext_torque_nm: -40.0,
                        ext_slope_nm_s_rad: 0.0,
                    };
                    for _ in 0..50 {
                        let out = c.step(1.0 / 120.0, &mut e, thr, pedal, 1.0, &ds);
                        let cap = 250.0 * pedal.map_or(1.0, |p| 1.0 - p);
                        assert!(out.torque_nm.abs() <= cap + 1e-9, "{} > {cap}", out.torque_nm);
                    }
                }
            }
        }
    }

    /// Closed loop: clutch locked to a vehicle whose road load grows with the square of speed.
    fn run_locked(dt: f64) -> (f64, usize) {
        let def = CouplingDef::Clutch { max_torque_nm: 800.0, engage_rpm: 600.0 };
        let mut c = Coupling::new(&def, 400.0, &coupling_tuning()).unwrap();
        let mut e = flat_engine();
        let (jd, drag) = (1.2, 0.00875);
        let mut wd = 0.0;
        let (mut flips, mut prev) = (0, 1.0);
        for k in 0..((12.0 / dt) as usize) {
            let ds = Downstream {
                omega_rad_s: wd,
                inertia_kg_m2: jd,
                ext_torque_nm: -drag * wd * wd,
                ext_slope_nm_s_rad: 2.0 * drag * wd,
            };
            let out = c.step(dt, &mut e, 1.0, None, 1.0, &ds);
            wd += dt * (out.torque_nm + ds.ext_torque_nm) / jd;
            if k as f64 * dt > 8.0 && out.torque_nm * prev < 0.0 {
                flips += 1;
            }
            prev = out.torque_nm;
        }
        (wd, flips)
    }

    #[test]
    fn a_locked_clutch_holds_a_speed_dependent_load_without_chatter_at_any_step() {
        let (reference, _) = run_locked(1.0 / 4000.0);
        for hz in [60.0, 120.0, 240.0, 480.0] {
            let (w, flips) = run_locked(1.0 / hz);
            assert_eq!(flips, 0, "torque chattered at {hz} Hz");
            assert!((w - reference).abs() < 0.01 * reference, "{hz} Hz: {w} vs {reference}");
        }
    }

    #[test]
    fn lockup_removes_slip() {
        let (mut e, mut c) = (flat_engine(), converter(Some(0.8)));
        let (jd, drag, dt) = (1.2, 0.00875, 1.0 / 240.0);
        let mut wd = 0.0;
        let mut max_slip: f64 = 0.0;
        for _ in 0..(40 * 240) {
            let ds = Downstream {
                omega_rad_s: wd,
                inertia_kg_m2: jd,
                ext_torque_nm: -drag * wd * wd,
                ext_slope_nm_s_rad: 2.0 * drag * wd,
            };
            let out = c.step(dt, &mut e, 1.0, None, 1.0, &ds);
            wd += dt * (out.torque_nm + ds.ext_torque_nm) / jd;
            max_slip = max_slip.max(out.slip_rad_s);
        }
        let slip = e.omega_rad_s() - wd;
        assert!(c.is_locked(), "never locked (speed ratio {})", wd / e.omega_rad_s());
        assert!(max_slip > 5.0 && slip.abs() < 0.5, "slip went {max_slip} -> {slip}");
    }
}
