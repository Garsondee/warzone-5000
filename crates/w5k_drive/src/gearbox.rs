//! The gearbox: ratios as a lever, an automatic shift map with hysteresis, and the torque interruption of a shift.
//!
//! Principle: a gear pair multiplies torque by the ratio and divides speed by it (power in = power out, less the mesh losses). The shift
//! map is a Schmitt trigger: the upshift and downshift points are different, and a shift is refused if it would land next to the opposite
//! point, so the box cannot hunt between two gears.

use serde::{Deserialize, Serialize};
use w5k_contract::command::GearRequest;
use w5k_contract::rig::GearboxDef;
use w5k_contract::Param;
use w5k_math::scalar::lerp;
use w5k_math::StateHasher;

use crate::coupling::Downstream;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShiftTuning {
    pub light_throttle_shift_scale: Param,
    pub kickdown_throttle: Param,
    pub hysteresis_margin_rpm: Param,
    pub min_gear_dwell_s: Param,
    pub reverse_engage_max_speed_m_s: Param,
}

impl ShiftTuning {
    pub fn check(&self) -> Result<(), String> {
        self.light_throttle_shift_scale.check("light_throttle_shift_scale")?;
        self.kickdown_throttle.check("kickdown_throttle")?;
        self.hysteresis_margin_rpm.check("hysteresis_margin_rpm")?;
        self.min_gear_dwell_s.check("min_gear_dwell_s")?;
        self.reverse_engage_max_speed_m_s.check("reverse_engage_max_speed_m_s")
    }
}

/// Road speed for an engine speed through a total reduction `ratio` (engine / wheel) and a rolling radius: `v = w r / ratio`.
pub fn road_speed_m_s(engine_omega_rad_s: f64, total_ratio: f64, wheel_radius_m: f64) -> f64 {
    engine_omega_rad_s * wheel_radius_m / total_ratio
}

/// What the gearbox tells the rest of the powertrain this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShiftOut {
    /// 0 neutral, positive forward, negative reverse.
    pub gear: i8,
    pub shifting: bool,
    /// Multiplies the coupling's capacity: 0 while the torque is interrupted, else 1.
    pub capacity_scale: f64,
}

#[derive(Clone, Debug)]
pub struct Gearbox {
    forward: Vec<f64>,
    reverse: Vec<f64>,
    efficiency: f64,
    inertia: f64,
    automatic: bool,
    up_rpm: f64,
    down_rpm: f64,
    shift_time_s: f64,
    light_scale: f64,
    kickdown: f64,
    margin_rpm: f64,
    dwell_min_s: f64,
    reverse_max_speed: f64,
    gear: i8,
    pending: Option<i8>,
    shift_left_s: f64,
    dwell_s: f64,
    driving: bool,
}

impl Gearbox {
    pub fn new(def: &GearboxDef, tuning: &ShiftTuning) -> Result<Gearbox, String> {
        tuning.check()?;
        if def.forward_ratios.is_empty()
            || def.forward_ratios.iter().chain(&def.reverse_ratios).any(|r| r.is_nan() || *r <= 0.0)
        {
            return Err("gearbox needs at least one forward ratio and all ratios > 0".into());
        }
        if def.forward_ratios.windows(2).any(|p| p[1] >= p[0]) {
            return Err("forward ratios must fall from first gear to top".into());
        }
        if !(def.efficiency > 0.0 && def.efficiency <= 1.0 && def.inertia_kg_m2 > 0.0) {
            return Err("gearbox efficiency must be in (0, 1] and inertia > 0".into());
        }
        if def.forward_ratios.len() > 100 {
            return Err("gearbox: more than 100 forward gears".into()); // const-ok: gear index must fit i8 with reverse
        }
        let s = &def.shift;
        Ok(Gearbox {
            forward: def.forward_ratios.clone(),
            reverse: def.reverse_ratios.clone(),
            efficiency: def.efficiency,
            inertia: def.inertia_kg_m2,
            automatic: s.automatic,
            up_rpm: s.upshift_rpm,
            down_rpm: s.downshift_rpm,
            shift_time_s: s.shift_time_s.max(0.0),
            light_scale: tuning.light_throttle_shift_scale.v,
            kickdown: tuning.kickdown_throttle.v,
            margin_rpm: tuning.hysteresis_margin_rpm.v,
            dwell_min_s: tuning.min_gear_dwell_s.v,
            reverse_max_speed: tuning.reverse_engage_max_speed_m_s.v,
            gear: if def.forward_ratios.is_empty() { 0 } else { 1 },
            pending: None,
            shift_left_s: 0.0,
            dwell_s: tuning.min_gear_dwell_s.v,
            driving: true,
        })
    }

    pub fn gear(&self) -> i8 {
        self.gear
    }

    fn ratio_of(&self, gear: i8) -> f64 {
        match gear {
            0 => 1.0,
            g if g > 0 => self.forward[usize::from(g.unsigned_abs() - 1)],
            g => self
                .reverse
                .get(usize::from(g.unsigned_abs() - 1))
                .copied()
                .unwrap_or_else(|| self.reverse.last().copied().unwrap_or(1.0)),
        }
    }

    /// Signed reduction from engine side to output: negative in reverse (the output turns the other way). 1 in neutral (unused).
    pub fn signed_ratio(&self) -> f64 {
        let r = self.ratio_of(self.gear);
        if self.gear < 0 {
            -r
        } else {
            r
        }
    }

    /// Decide gear changes and report the torque interruption. `engine_rpm` and `throttle` feed the automatic map.
    pub fn update(
        &mut self,
        dt: f64,
        request: GearRequest,
        throttle: f64,
        engine_rpm: f64,
        road_speed_m_s: f64,
    ) -> ShiftOut {
        self.dwell_s += dt;
        if self.shift_left_s > 0.0 {
            self.shift_left_s -= dt;
            if self.shift_left_s <= 0.0 {
                self.shift_left_s = 0.0;
                if let Some(g) = self.pending.take() {
                    self.gear = g;
                    self.dwell_s = 0.0;
                }
            }
        } else if let Some(target) = self.choose(request, throttle, engine_rpm, road_speed_m_s) {
            if target != self.gear {
                self.pending = Some(target);
                self.shift_left_s = self.shift_time_s;
                if self.shift_left_s <= 0.0 {
                    self.gear = target;
                    self.pending = None;
                    self.dwell_s = 0.0;
                }
            }
        }
        let shifting = self.shift_left_s > 0.0 || self.gear == 0;
        ShiftOut {
            gear: self.gear,
            shifting: self.shift_left_s > 0.0,
            capacity_scale: if shifting { 0.0 } else { 1.0 },
        }
    }

    fn choose(&self, request: GearRequest, throttle: f64, rpm: f64, speed: f64) -> Option<i8> {
        let top = i8::try_from(self.forward.len()).unwrap_or(i8::MAX);
        let slow = speed.abs() < self.reverse_max_speed;
        match request {
            GearRequest::Neutral => Some(0),
            GearRequest::Reverse => (slow && !self.reverse.is_empty()).then_some(-1),
            GearRequest::Hold => None,
            GearRequest::Up => (self.gear >= 0 && self.gear < top).then(|| self.gear + 1),
            GearRequest::Down => (self.gear > 1).then(|| self.gear - 1),
            GearRequest::Gear(n) => {
                (n >= 1 && (self.gear > 0 || slow)).then(|| i8::try_from(n).unwrap_or(top).min(top))
            }
            GearRequest::Auto => {
                if self.gear <= 0 {
                    return slow.then_some(1);
                }
                if !self.automatic || self.dwell_s < self.dwell_min_s {
                    return None;
                }
                let scale = lerp(self.light_scale, 1.0, throttle.clamp(0.0, 1.0));
                let (up, down) = (self.up_rpm * scale, self.down_rpm * scale);
                let (here, g) = (self.ratio_of(self.gear), self.gear);
                if g < top && rpm > up && rpm * self.ratio_of(g + 1) / here > down + self.margin_rpm {
                    Some(g + 1)
                } else if g > 1
                    && self.ratio_of(g - 1) / here * rpm < (if rpm < down { up } else { self.up_rpm }) - self.margin_rpm
                    && (rpm < down || throttle > self.kickdown)
                {
                    // a downshift: below the downshift point, or a kick-down on the pedal; either way it must not land near the upshift point
                    Some(g - 1)
                } else {
                    None
                }
            }
        }
    }

    /// The driven side as the coupling sees it (everything reflected through the ratio to the gearbox input).
    pub fn reflect(&self, out: &Downstream) -> Downstream {
        let r = self.signed_ratio();
        let k = if self.driving { 1.0 / self.efficiency } else { self.efficiency };
        Downstream {
            omega_rad_s: r * out.omega_rad_s,
            inertia_kg_m2: out.inertia_kg_m2 / (r * r) + self.inertia,
            ext_torque_nm: out.ext_torque_nm / r * k,
            ext_slope_nm_s_rad: out.ext_slope_nm_s_rad / (r * r),
        }
    }

    /// Torque at the output for a torque `t_in` delivered to the gearbox input: times the ratio, less the mesh loss in the direction of flow.
    pub fn output_torque(&mut self, t_in: f64) -> f64 {
        self.driving = t_in >= 0.0;
        let k = if self.driving { self.efficiency } else { 1.0 / self.efficiency };
        t_in * self.signed_ratio() * k
    }

    pub fn hash_state(&self, h: &mut StateHasher) {
        h.write_u8(self.gear.to_le_bytes()[0]);
        h.write_f64(self.shift_left_s);
        h.write_f64(self.dwell_s);
        h.write_u8(u8::from(self.driving));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::shift_tuning;
    use w5k_contract::rig::ShiftDef;

    fn def() -> GearboxDef {
        GearboxDef {
            forward_ratios: vec![2.48, 1.48, 1.0, 0.73],
            reverse_ratios: vec![2.0],
            efficiency: 0.95,
            inertia_kg_m2: 0.05,
            shift: ShiftDef { automatic: true, upshift_rpm: 3200.0, downshift_rpm: 1500.0, shift_time_s: 0.3 },
        }
    }

    fn gb() -> Gearbox {
        Gearbox::new(&def(), &shift_tuning()).unwrap()
    }

    #[test]
    fn speed_in_each_gear_at_redline_matches_the_ratio_arithmetic() {
        let (w, radius, final_drive) = (4000.0 * core::f64::consts::PI / 30.0, 0.4, 4.1);
        let mut g = gb();
        for (i, r) in [2.48, 1.48, 1.0, 0.73].into_iter().enumerate() {
            g.gear = i8::try_from(i + 1).unwrap();
            let v = road_speed_m_s(w, g.signed_ratio() * final_drive, radius);
            assert!((v - w * radius / (r * final_drive)).abs() < 1e-12, "gear {}", i + 1);
        }
        // and it is a lever: output speed is input speed over the ratio, output torque is input torque times it (less 5% loss)
        let mut g = gb();
        let out = Downstream { omega_rad_s: 100.0, inertia_kg_m2: 10.0, ..Default::default() };
        assert!((g.reflect(&out).omega_rad_s - 248.0).abs() < 1e-9);
        assert!((g.output_torque(100.0) - 100.0 * 2.48 * 0.95).abs() < 1e-9);
    }

    #[test]
    fn automatic_upshifts_at_the_stated_rpm_and_downshifts_with_hysteresis() {
        let dt = 1.0 / 120.0;
        let (mut g, ratios) = (gb(), [2.48, 1.48, 1.0, 0.73]);
        let mut shifts: Vec<(f64, i8)> = Vec::new();
        let mut last = g.gear();
        // the road (output) speed in rpm rises to 1300 over 10 s and falls back; the engine follows through the current ratio.
        // 80% pedal is below kick-down, so the shift points scale to 0.7 + 0.3 * 0.8 = 0.94 of their full-throttle values.
        for k in 0..2400 {
            let t = f64::from(k) * dt;
            let out_rpm = if t < 10.0 { 130.0 * t } else { 1300.0 - 130.0 * (t - 10.0) };
            let rpm = out_rpm * ratios[usize::from(g.gear().unsigned_abs()) - 1];
            let o = g.update(dt, GearRequest::Auto, 0.8, rpm, 5.0);
            if o.gear != last {
                shifts.push((out_rpm, o.gear));
                last = o.gear;
            }
        }
        assert_eq!(shifts.len(), 2, "{shifts:?}");
        // 1 -> 2 when the engine passes 3200 * 0.94 = 3008 rpm (taking effect one 0.3 s interruption later)
        assert_eq!(shifts[0].1, 2);
        let up_engine_rpm = shifts[0].0 * 2.48;
        assert!(
            up_engine_rpm > 3008.0 && up_engine_rpm < 3008.0 + 130.0 * 2.48 * 0.3 + 5.0,
            "upshift at {up_engine_rpm} rpm"
        );
        // 2 -> 1 when the engine falls below 1500 * 0.94 = 1410 rpm: at a lower road speed than the upshift (the hysteresis)
        assert_eq!(shifts[1].1, 1);
        let down_engine_rpm = shifts[1].0 * 1.48;
        assert!(
            down_engine_rpm < 1410.0 && down_engine_rpm > 1410.0 - 130.0 * 1.48 * 0.3 - 5.0,
            "downshift at {down_engine_rpm} rpm"
        );
        assert!(shifts[1].0 < shifts[0].0, "the downshift speed must be below the upshift speed");
    }

    #[test]
    fn shift_interrupts_torque_for_the_stated_time() {
        let dt = 1.0 / 1000.0;
        let mut g = gb();
        let mut off = 0.0;
        for _ in 0..200 {
            g.update(dt, GearRequest::Auto, 1.0, 1000.0, 5.0);
        }
        for _ in 0..900 {
            let o = g.update(dt, GearRequest::Auto, 1.0, 3500.0, 5.0);
            if o.capacity_scale == 0.0 {
                off += dt;
            }
        }
        assert!((off - 0.3).abs() < 2.0 * dt, "interrupted for {off} s");
        assert_eq!(g.gear(), 2);
    }

    #[test]
    fn reverse_gears_work() {
        let mut g = gb();
        // refused while rolling, accepted when nearly stopped
        g.update(0.5, GearRequest::Reverse, 0.0, 800.0, 8.0);
        for _ in 0..100 {
            g.update(0.01, GearRequest::Reverse, 0.0, 800.0, 8.0);
        }
        assert_eq!(g.gear(), 1);
        for _ in 0..100 {
            g.update(0.01, GearRequest::Reverse, 0.0, 800.0, 0.2);
        }
        assert_eq!(g.gear(), -1);
        assert!(g.signed_ratio() < 0.0 && (g.signed_ratio() + 2.0).abs() < 1e-12);
        let drive = g.output_torque(100.0);
        assert!(drive < 0.0 && (drive + 100.0 * 2.0 * 0.95).abs() < 1e-9, "reverse drives backwards");
        let out = Downstream { omega_rad_s: -10.0, inertia_kg_m2: 4.0, ..Default::default() };
        assert!((g.reflect(&out).omega_rad_s - 20.0).abs() < 1e-9, "backwards output turns the input forwards");
    }

    #[test]
    fn the_automatic_does_not_hunt_when_cruising_between_two_gears() {
        let mut g = gb();
        g.gear = 2;
        let mut shifts = 0;
        let mut last = 2;
        for _ in 0..6000 {
            let o = g.update(0.01, GearRequest::Auto, 0.5, 2300.0, 10.0);
            if o.gear != last {
                shifts += 1;
                last = o.gear;
            }
        }
        assert_eq!(shifts, 0);
    }
}
